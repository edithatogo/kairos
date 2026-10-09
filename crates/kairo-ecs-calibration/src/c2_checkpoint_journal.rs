//! Private replay-only checkpoint proof for one sealed C2 route scenario.
//! This is not a general Flow serializer or durable continuation API.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const LIMIT: u64 = 1024 * 1024;
const SCHEMA: &str = "kairos-c2-replay-journal-v1";
const RECIPE: &str = "c2.synthetic.annotated-micro.route5.pause1.resume3.v1";
const HANDLERS: &[&str] = &["bridge.transit:c20", "bridge.context:u32"];
const OPERATIONS: &[&str] = &[
    "prepare",
    "create",
    "bind",
    "start@0",
    "pause@1",
    "queue-resume@3",
    "observe-pause@1",
];
const EXPECTED_PREFIX_SHA256: &str =
    "f8ef858f10c65eeea7e4d188423a643f1dec15a211ef9a52b4647d986821dee8";
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum JournalError {
    Io,
    TooLarge,
    Invalid,
    Exists,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn digest(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn canonical(value: &Value) -> Result<Vec<u8>, JournalError> {
    serde_json::to_vec(value).map_err(|_| JournalError::Invalid)
}

fn executable_sha256() -> Result<String, JournalError> {
    let path = std::env::current_exe().map_err(|_| JournalError::Io)?;
    let mut file = File::open(path).map_err(|_| JournalError::Io)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 16 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|_| JournalError::Io)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

fn compiled_config(binary: &str) -> Value {
    json!({"recipe": RECIPE, "source_executable_sha256": binary,
        "seed_map": {"version":1,"root_seed":"19","study":"c2-replay-test","schedule":"paired","replication":"0","case":"case-a","task":"triage","purpose":"Service"},
        "provider": {"version":1,"stratum":"triage","weighted_ticks":[["20","1"],["30","1"]]},
        "route": {"graph_version":1,"nodes":[1,2],"edges":[[1,1,2,5000,["walk"]]],"origin":1,"destination":2,"mode":"walk","speed_mm_per_second":"1000","ticks_per_second":"1","purpose":"patient-transfer","distance_provenance":"ConfiguredGeometry","duration_ticks":"5"},
        "acquire": {"capacity":1,"at":"0","priority":3,"scheduler_priority":7},
        "fidelity": {"policy_version":1,"global":"Micro"},
        "flow_setup": {"owner_actor_order":0,"carrier_actor_order":1,"resource_order":2,"context_template":7,"subsystem":"assessment","context_registration":"bridge.context","transit_registration":"bridge.transit","event_kind":3104},
        "handlers": HANDLERS, "operations": OPERATIONS})
}

fn envelope(binary: &str, frontier: Value, prefix: Value) -> Result<Value, JournalError> {
    let config = compiled_config(binary);
    let mut prefix_config = config.clone();
    prefix_config["source_executable_sha256"] = json!("BOUND_BY_ENVELOPE_EXECUTABLE_SHA256");
    let prefix_bytes =
        canonical(&json!({"config":prefix_config,"operations":OPERATIONS,"prefix":prefix}))?;
    let prefix_hash = digest(&prefix_bytes);
    let body = json!({"schema":SCHEMA,"config":compiled_config(binary),"frontier":frontier,"prefix":prefix,"prefix_sha256":prefix_hash});
    let body_bytes = canonical(&body)?;
    Ok(json!({"body":body,"integrity_sha256":digest(&body_bytes)}))
}

fn validate_bytes(raw: &[u8]) -> Result<Value, JournalError> {
    if raw.len() as u64 > LIMIT {
        return Err(JournalError::TooLarge);
    }
    let parsed: Value = serde_json::from_slice(raw).map_err(|_| JournalError::Invalid)?;
    if canonical(&parsed)? != raw {
        return Err(JournalError::Invalid);
    }
    let binary = executable_sha256()?;
    let expected_config = compiled_config(&binary);
    let root = parsed.as_object().ok_or(JournalError::Invalid)?;
    if root.len() != 2 || !root.contains_key("body") || !root.contains_key("integrity_sha256") {
        return Err(JournalError::Invalid);
    }
    let body = root
        .get("body")
        .and_then(Value::as_object)
        .ok_or(JournalError::Invalid)?;
    if body.len() != 5
        || body.get("schema") != Some(&json!(SCHEMA))
        || body.get("config") != Some(&expected_config)
    {
        return Err(JournalError::Invalid);
    }
    let prefix = body.get("prefix").ok_or(JournalError::Invalid)?;
    let frontier = body.get("frontier").ok_or(JournalError::Invalid)?;
    if frontier != &expected_frontier() {
        return Err(JournalError::Invalid);
    }
    if body.get("prefix_sha256") != Some(&json!(EXPECTED_PREFIX_SHA256)) {
        return Err(JournalError::Invalid);
    }
    let expected = envelope(&binary, frontier.clone(), prefix.clone())?;
    if expected != parsed {
        return Err(JournalError::Invalid);
    }
    let body_bytes = canonical(
        &json!({"schema":SCHEMA,"config":expected_config,"frontier":frontier,"prefix":prefix,"prefix_sha256":EXPECTED_PREFIX_SHA256}),
    )?;
    let claimed = root
        .get("integrity_sha256")
        .and_then(Value::as_str)
        .ok_or(JournalError::Invalid)?;
    if claimed != digest(&body_bytes) {
        return Err(JournalError::Invalid);
    }
    Ok(parsed)
}

fn expected_frontier() -> Value {
    json!({"now":"1","phase":"Paused","useful_elapsed":"1","remaining":"4","pending_progress_due_from_route_plan":"5","resume_at":"3","draw_position":"1"})
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, JournalError> {
    let meta = fs::symlink_metadata(path).map_err(|_| JournalError::Io)?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(JournalError::Invalid);
    }
    if meta.len() > LIMIT {
        return Err(JournalError::TooLarge);
    }
    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|_| JournalError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let opened = file.metadata().map_err(|_| JournalError::Io)?;
        if !opened.is_file() || opened.dev() != meta.dev() || opened.ino() != meta.ino() {
            return Err(JournalError::Invalid);
        }
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| JournalError::Io)?;
    if bytes.len() as u64 > LIMIT {
        return Err(JournalError::TooLarge);
    }
    Ok(bytes)
}

fn publish_no_replace(path: &Path, bytes: &[u8]) -> Result<(), JournalError> {
    let parent = path.parent().ok_or(JournalError::Invalid)?;
    fs::create_dir_all(parent).map_err(|_| JournalError::Io)?;
    let name = path
        .file_name()
        .ok_or(JournalError::Invalid)?
        .to_string_lossy();
    let temp = loop {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), id));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                if file.write_all(bytes).and_then(|_| file.sync_all()).is_err() {
                    let _ = fs::remove_file(&candidate);
                    return Err(JournalError::Io);
                }
                break candidate;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(JournalError::Io),
        }
    };
    let linked = fs::hard_link(&temp, path);
    let _ = fs::remove_file(&temp);
    match linked {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(JournalError::Exists);
        }
        Err(_) => return Err(JournalError::Io),
    }
    File::open(parent)
        .and_then(|f| f.sync_all())
        .map_err(|_| JournalError::Io)
}

#[cfg(test)]
fn captured_artifact_path() -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = root.join(".artifacts/c2-checkpoint-replay");
    for directory in [root.join(".artifacts"), output.clone()] {
        if let Ok(meta) = fs::symlink_metadata(&directory) {
            assert!(
                !meta.file_type().is_symlink() && meta.is_dir(),
                "artifact output path must not contain a symlink"
            );
        } else {
            fs::create_dir(&directory).expect("create declared artifact directory");
        }
    }
    let canonical_root = fs::canonicalize(&root).expect("repo root");
    let canonical_output = fs::canonicalize(&output).expect("artifact output directory");
    assert!(
        canonical_output.starts_with(canonical_root),
        "artifact path must remain under repository"
    );
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    output.join(format!("checkpoint-{}-{nonce}.json", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow_bridge::{
        AcquireIntent, BoundIntrinsicWork, PreparationIdentity, TransitRequest,
        WorkPreparationInput,
    };
    use crate::route_receipt::{DistanceProvenance, RouteMetadata};
    use crate::seed_map::{CalibrationSeedMap, SeedPurpose};
    use crate::work_duration::{
        INTRINSIC_WORK_PROVIDER_VERSION_V1, IntrinsicDurationDistribution, IntrinsicWorkProvider,
    };
    use kairo_ecs_abm::spatial::{
        EdgeId, MovementModeId, MovementProfile, NodeId, TransitEdge, TransitGraphV1,
    };
    use kairo_ecs_abm::{TransitContext, TransitPhase, register_transit_context};
    use kairo_ecs_des::fidelity::{FidelityAdapter, FidelityMode, FidelityPolicy};
    use kairo_ecs_des::{FlowBatchReceipt, FlowDomainControl, FlowRuntime, WorkHandlers};
    use kairo_ecs_types::{EventKind, SimTime};
    use std::sync::Arc;

    fn unique_temp_dir(label: &str) -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("kairos-c2-{label}-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).expect("exclusive temp directory");
        path
    }

    fn make_context(v: &u32) -> u32 {
        *v
    }
    fn build() -> (FlowRuntime, FidelityAdapter, BoundIntrinsicWork<u32, u32>) {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let actor = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();
        register_transit_context(&mut flow, "bridge.transit", EventKind::custom(0xC20)).unwrap();
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let mut seeds = CalibrationSeedMap::new(1, "c2-replay-test", 19).unwrap();
        let key = seeds
            .key_for("paired", 0, "case-a", "triage", SeedPurpose::Service)
            .unwrap();
        let stream = seeds
            .stream_for("paired", 0, "case-a", "triage", SeedPurpose::Service)
            .unwrap();
        let graph = Arc::new(
            TransitGraphV1::new(
                1,
                vec![NodeId::new(1), NodeId::new(2)],
                vec![TransitEdge {
                    id: EdgeId::new(1),
                    from: NodeId::new(1),
                    to: NodeId::new(2),
                    length_mm: 5000,
                    allowed_modes: vec![MovementModeId::new("walk").unwrap()],
                }],
            )
            .unwrap(),
        );
        let input = WorkPreparationInput::new(
            PreparationIdentity {
                owner,
                subsystem: "assessment".into(),
                registration: "bridge.context".into(),
                stratum: "triage".into(),
            },
            stream,
            key,
            7,
            make_context,
            AcquireIntent {
                resource,
                owner,
                at: SimTime::from_ticks(0),
                priority_level: 3,
                deadline: None,
                scheduler_priority: 7,
                can_preempt: false,
                preemptible: None,
            },
            TransitRequest::Route {
                graph,
                origin: NodeId::new(1),
                destination: NodeId::new(2),
                profile: MovementProfile::new("walk", 1000).unwrap(),
                ticks_per_second: 1,
                carrier_actor: actor,
                carrier_registration: "bridge.transit".into(),
                kind: EventKind::custom(0xC20),
            },
        )
        .with_route_metadata(RouteMetadata::v1(
            "patient-transfer",
            DistanceProvenance::ConfiguredGeometry,
        ));
        let provider = IntrinsicWorkProvider::new(
            INTRINSIC_WORK_PROVIDER_VERSION_V1,
            vec![(
                "triage".into(),
                IntrinsicDurationDistribution::weighted_ticks(vec![(20, 1), (30, 1)]).unwrap(),
            )],
        )
        .unwrap();
        let mut adapter =
            FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap());
        let prepared = input
            .prepare(&flow, &mut adapter, &provider)
            .unwrap_or_else(|_| panic!("prepare"));
        let created = prepared
            .create(&mut flow)
            .unwrap_or_else(|_| panic!("create"));
        let bound = created.bind(&flow).unwrap_or_else(|_| panic!("bind"));
        (flow, adapter, bound)
    }

    fn scenario() -> (
        FlowRuntime,
        FidelityAdapter,
        BoundIntrinsicWork<u32, u32>,
        Value,
        Value,
        kairo_ecs_types::EventId,
    ) {
        let (mut flow, adapter, mut bound) = build();
        let start = bound.start_transit(&mut flow).unwrap();
        let pause = bound
            .schedule_transit_control(
                &mut flow,
                FlowDomainControl::Pause,
                SimTime::from_ticks(1),
                7,
            )
            .unwrap();
        let resume = bound
            .schedule_transit_control(
                &mut flow,
                FlowDomainControl::Resume,
                SimTime::from_ticks(3),
                7,
            )
            .unwrap();
        let d0 = flow.step().unwrap().unwrap();
        assert_eq!(d0.event, start);
        let pending_admission = match d0.callback_batches.as_slice() {
            [FlowBatchReceipt::Accepted(admissions)] => {
                admissions
                    .first()
                    .expect("start creates first progress command")
                    .event
            }
            _ => panic!("accepted start must expose progress admission"),
        };
        assert!(bound.observe_transit_dispatch(&flow, &d0).is_ok());
        let stale = bound.pending_event_for_checkpoint().unwrap();
        assert_eq!(stale, pending_admission);
        let d1 = flow.step().unwrap().unwrap();
        assert_eq!(d1.event, pause);
        assert!(bound.observe_transit_dispatch(&flow, &d1).is_ok());
        let carrier = bound.carrier_id_for_checkpoint().unwrap();
        let context = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(context.phase(), TransitPhase::Paused);
        assert!(bound.validate_route_context_for_checkpoint(&flow).is_ok());
        let progress = context.progress_at(flow.now()).unwrap();
        let actual_pending_due = SimTime::from_ticks(context.route_plan().duration().ticks());
        assert_eq!(actual_pending_due, SimTime::from_ticks(5));
        assert!(context.expects_event(stale, actual_pending_due));
        let budget = flow.budget_snapshot().scheduler;
        let work = bound.work();
        let work_spec = flow.work(work).unwrap();
        let work_progress = flow.work_progress(work).unwrap();
        let resource = flow.resource(bound.acquire_intent().resource).unwrap();
        let request = work_spec.request.map(|id| { let saved=flow.request(id).unwrap(); json!({"id":{"index":id.entity_id().index,"generation":id.entity_id().generation},"state":format!("{:?}",saved.state),"submitted_at":saved.submitted_at.ticks().to_string()}) });
        let decision = bound.decision();
        let owner = work_spec.owner;
        let resource_id = bound.acquire_intent().resource.entity_id();
        let front = json!({"now":flow.now().ticks().to_string(),"phase":"Paused","useful_elapsed":progress.useful_elapsed.ticks().to_string(),"remaining":progress.remaining.ticks().to_string(),"pending_progress_due_from_route_plan":actual_pending_due.ticks().to_string(),"resume_at":"3","draw_position":bound.service_draw_position().to_string()});
        let prefix = json!({"start_event":{"index":start.index,"generation":start.generation},"pause_event":{"index":pause.index,"generation":pause.generation},"resume_event":{"index":resume.index,"generation":resume.generation},"old_progress_event":{"index":stale.index,"generation":stale.generation},"start_progress_admission_event":{"index":pending_admission.index,"generation":pending_admission.generation},"scheduler":{"scheduled":budget.scheduled_events,"dispatched":budget.dispatched_events,"cancelled":budget.cancelled_events,"pending":budget.pending_events},"service_sample_ticks":bound.sampled_duration().ticks().to_string(),"service_draw_probe":bound.next_service_draw_probe_for_checkpoint().to_string(),"route_receipt_sha256":bound.route_receipt_sha_for_checkpoint(&flow).unwrap(),"carrier_id":{"index":carrier.entity_id().index,"generation":carrier.entity_id().generation},"decision":{"mode":format!("{:?}",decision.mode),"policy_version":decision.policy_version},"owner":{"index":owner.index,"generation":owner.generation},"work":{"id":{"index":work.entity_id().index,"generation":work.entity_id().generation},"state":format!("{:?}",work_progress.state),"useful_elapsed":work_progress.useful_elapsed.ticks().to_string(),"remaining":work_progress.remaining.ticks().to_string(),"context_type":work_spec.context_type_key},"request":request,"resource":{"id":{"index":resource_id.index,"generation":resource_id.generation},"total":resource.total,"available":resource.available,"queued":resource.queued.len(),"active":resource.active.len()},"frontier":front});
        (flow, adapter, bound, front, prefix, stale)
    }

    fn prefix() -> (Value, Value) {
        let (_, _, _, front, prefix, _) = scenario();
        (front, prefix)
    }

    fn run_suffix() -> Value {
        let (mut flow, _adapter, mut bound, front, prefix_data, stale) = scenario();
        assert_eq!(front, expected_frontier());
        let binary = executable_sha256().unwrap();
        let mut cfg = compiled_config(&binary);
        cfg["source_executable_sha256"] = json!("BOUND_BY_ENVELOPE_EXECUTABLE_SHA256");
        let digest_now = digest(
            &canonical(&json!({"config":cfg,"operations":OPERATIONS,"prefix":prefix_data}))
                .unwrap(),
        );
        // The artifact constructor freezes the canonical scenario digest; artifact data never selects runtime inputs.
        let (expected_front, expected_prefix) = prefix();
        assert_eq!(front, expected_front);
        assert_eq!(prefix_data, expected_prefix);
        assert_eq!(digest_now, EXPECTED_PREFIX_SHA256);
        let mut observations = Vec::new();
        let mut arrived = false;
        for _ in 0..3 {
            if arrived {
                break;
            }
            let dispatch = flow
                .step()
                .unwrap()
                .expect("expected one of three route suffix events");
            let observation = bound.observe_transit_dispatch(&flow, &dispatch).unwrap();
            observations.push(json!({"event":{"index":dispatch.event.index,"generation":dispatch.event.generation},"at":dispatch.at.ticks().to_string(),"observation":format!("{observation:?}")}));
            if dispatch.event == stale {
                assert_eq!(dispatch.at, SimTime::from_ticks(5));
                assert_eq!(
                    observation,
                    crate::flow_bridge::TransitObservation::IgnoredStale
                );
            }
            if observation == crate::flow_bridge::TransitObservation::Resumed {
                assert_eq!(dispatch.at, SimTime::from_ticks(3));
                assert_eq!(dispatch.event.index, prefix_data["resume_event"]["index"]);
                assert_eq!(
                    dispatch.event.generation,
                    prefix_data["resume_event"]["generation"]
                );
            }
            if observation == crate::flow_bridge::TransitObservation::Arrived {
                assert_eq!(dispatch.at, SimTime::from_ticks(7));
            }
            arrived = observation == crate::flow_bridge::TransitObservation::Arrived;
        }
        assert!(
            arrived,
            "route did not arrive within the closed three-event suffix"
        );
        assert_eq!(
            observations
                .iter()
                .filter(|v| v["observation"] == "IgnoredStale")
                .count(),
            1
        );
        assert_eq!(
            observations
                .iter()
                .filter(|v| v["observation"] == "Arrived")
                .count(),
            1
        );
        assert_eq!(
            observations
                .iter()
                .filter(|v| v["observation"] == "Resumed")
                .count(),
            1
        );
        let carrier = bound.carrier_id_for_checkpoint().unwrap();
        let context = flow.work_context::<TransitContext>(carrier).unwrap();
        let p = context.progress_at(flow.now()).unwrap();
        let route_phase = format!("{:?}", context.phase());
        assert_eq!(flow.now(), SimTime::from_ticks(7));
        assert_eq!(p.useful_elapsed.ticks(), 5);
        assert_eq!(p.remaining.ticks(), 0);
        assert!(bound.validate_route_context_for_checkpoint(&flow).is_ok());
        let receipt_sha = bound.route_receipt_sha_for_checkpoint(&flow).unwrap();
        let service_draw_position = bound.service_draw_position();
        let next_service_draw = bound.next_service_draw_probe_for_checkpoint();
        let work = bound.work();
        let submitted = bound.finish_transit(&flow).unwrap();
        let request = submitted.request();
        assert_eq!(submitted.work(), work);
        assert_eq!(flow.work(work).unwrap().request, Some(request));
        let mut service_events = Vec::new();
        for _ in 0..2 {
            if flow.work_progress(work).unwrap().state == kairo_ecs_des::WorkState::Completed {
                break;
            }
            let dispatch = flow
                .step()
                .unwrap()
                .expect("one of two service events before completion");
            let lifecycle=dispatch.records.iter().map(|record| json!({"request":{"index":record.request.entity_id().index,"generation":record.request.entity_id().generation},"resource":{"index":record.resource.entity_id().index,"generation":record.resource.entity_id().generation},"work":record.snapshot.work.map(|w|json!({"index":w.entity_id().index,"generation":w.entity_id().generation})),"at":record.at.ticks().to_string(),"state":format!("{:?}",record.state),"transition":format!("{:?}",record.transition),"progress":record.snapshot.progress.as_ref().map(|p|json!({"state":format!("{:?}",p.state),"useful_elapsed":p.useful_elapsed.ticks().to_string(),"remaining":p.remaining.ticks().to_string()}))})).collect::<Vec<_>>();
            service_events.push(json!({"event":{"index":dispatch.event.index,"generation":dispatch.event.generation},"at":dispatch.at.ticks().to_string(),"lifecycle":lifecycle}));
        }
        assert_eq!(
            flow.work_progress(work).unwrap().state,
            kairo_ecs_des::WorkState::Completed
        );
        assert_eq!(service_events.len(), 2);
        assert_eq!(
            service_events
                .iter()
                .map(|v| v["at"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["7", "27"]
        );
        let completed_records = service_events
            .iter()
            .flat_map(|e| e["lifecycle"].as_array().unwrap())
            .filter(|r| r["transition"] == "Completed")
            .collect::<Vec<_>>();
        assert_eq!(completed_records.len(), 1);
        let completed_record = completed_records[0];
        assert_eq!(
            completed_record["request"]["index"],
            request.entity_id().index
        );
        assert_eq!(
            completed_record["request"]["generation"],
            request.entity_id().generation
        );
        assert_eq!(
            completed_record["resource"]["index"],
            submitted.acquire_intent().resource.entity_id().index
        );
        assert_eq!(
            completed_record["resource"]["generation"],
            submitted.acquire_intent().resource.entity_id().generation
        );
        assert_eq!(completed_record["work"]["index"], work.entity_id().index);
        assert_eq!(
            completed_record["work"]["generation"],
            work.entity_id().generation
        );
        assert_eq!(completed_record["at"], "27");
        assert_eq!(completed_record["state"], "Completed");
        assert_eq!(completed_record["progress"]["state"], "Completed");
        assert_eq!(completed_record["progress"]["useful_elapsed"], "20");
        assert_eq!(completed_record["progress"]["remaining"], "0");
        let final_progress = flow.work_progress(work).unwrap();
        let final_request = flow.request(request).unwrap();
        let resource = flow.resource(submitted.acquire_intent().resource).unwrap();
        assert_eq!(flow.now(), SimTime::from_ticks(27));
        assert_eq!(final_progress.useful_elapsed.ticks(), 20);
        assert_eq!(final_progress.remaining.ticks(), 0);
        assert_eq!(final_request.state, kairo_ecs_des::RequestState::Completed);
        assert_eq!(resource.available, resource.total);
        assert!(resource.queued.is_empty());
        assert!(resource.active.is_empty());
        json!({"route_suffix":observations,"route_arrival_at":"7","route_phase":route_phase,"route_useful_elapsed":p.useful_elapsed.ticks().to_string(),"route_remaining":p.remaining.ticks().to_string(),"stale_event":{"index":stale.index,"generation":stale.generation},"route_receipt_sha256":receipt_sha,"service_draw_position":service_draw_position.to_string(),"next_service_draw":next_service_draw.to_string(),"service_events":service_events,"service_completion_at":flow.now().ticks().to_string(),"service_work_state":format!("{:?}",final_progress.state),"service_useful_elapsed":final_progress.useful_elapsed.ticks().to_string(),"service_remaining":final_progress.remaining.ticks().to_string(),"request_state":format!("{:?}",final_request.state),"request_id":{"index":request.entity_id().index,"generation":request.entity_id().generation},"resource":{"total":resource.total,"available":resource.available,"queued":resource.queued.len(),"active":resource.active.len()},"scheduler":{"scheduled":flow.budget_snapshot().scheduler.scheduled_events,"dispatched":flow.budget_snapshot().scheduler.dispatched_events,"pending":flow.budget_snapshot().scheduler.pending_events}})
    }

    fn write_fixture(path: &Path) -> Result<(), JournalError> {
        let (front, prefix) = prefix();
        let value = envelope(&executable_sha256()?, front, prefix)?;
        publish_no_replace(path, &canonical(&value)?)
    }

    #[test]
    fn checkpoint_rejects_rehashed_mutations_and_never_overwrites() {
        let (front, prefix) = prefix();
        let mut value = envelope(&executable_sha256().unwrap(), front, prefix).unwrap();
        let bytes = canonical(&value).unwrap();
        assert_eq!(validate_bytes(&bytes), Ok(value.clone()));
        for mutate in [
            "config",
            "frontier",
            "prefix",
            "graph",
            "profile",
            "purpose",
            "source",
            "schema",
            "integrity",
        ] {
            let mut bad = value.clone();
            let body = bad["body"].as_object_mut().unwrap();
            match mutate {
                "config" => body.get_mut("config").unwrap()["route"]["destination"] = json!(99),
                "frontier" => body.get_mut("frontier").unwrap()["now"] = json!("01"),
                "prefix" => {
                    body.get_mut("prefix").unwrap()["old_progress_event"]["index"] = json!(999)
                }
                "graph" => body.get_mut("config").unwrap()["route"]["edges"][0][3] = json!(6000),
                "profile" => body.get_mut("config").unwrap()["route"]["mode"] = json!("roll"),
                "purpose" => body.get_mut("config").unwrap()["route"]["purpose"] = json!("other"),
                "source" => {
                    body.get_mut("config").unwrap()["source_executable_sha256"] = json!("00")
                }
                "schema" => body
                    .get_mut("schema")
                    .unwrap()
                    .clone_from(&json!("unknown")),
                _ => body
                    .get_mut("prefix_sha256")
                    .unwrap()
                    .clone_from(&json!("00")),
            }
            let body_value = body.clone();
            bad["integrity_sha256"] =
                json!(digest(&canonical(&Value::Object(body_value)).unwrap()));
            assert_eq!(
                validate_bytes(&canonical(&bad).unwrap()),
                Err(JournalError::Invalid)
            );
        }
        let dir = unique_temp_dir("journal");
        let target = dir.join("checkpoint.json");
        let original = b"preexisting sentinel bytes";
        fs::write(&target, original).unwrap();
        assert_eq!(
            publish_no_replace(&target, &bytes),
            Err(JournalError::Exists)
        );
        assert_eq!(fs::read(&target).unwrap(), original);
        fs::remove_dir_all(dir).unwrap();
        value.as_object_mut().unwrap();
    }

    #[test]
    fn artifact_size_and_canonical_json_are_bounded() {
        assert_eq!(
            validate_bytes(b"{\"a\":1,\"a\":1}"),
            Err(JournalError::Invalid)
        );
        assert_eq!(
            validate_bytes(b"{ \"schema\": 1 }"),
            Err(JournalError::Invalid)
        );
        assert_eq!(
            validate_bytes(&vec![b' '; (LIMIT + 1) as usize]),
            Err(JournalError::TooLarge)
        );
        let dir = unique_temp_dir("input-link");
        let oversized = dir.join("oversized.json");
        let oversized_file = fs::File::create(&oversized).unwrap();
        oversized_file.set_len(LIMIT + 1).unwrap();
        assert_eq!(read_bounded(&oversized), Err(JournalError::TooLarge));
        let target = dir.join("target.json");
        fs::write(&target, b"{}").unwrap();
        #[cfg(unix)]
        {
            let link = dir.join("link.json");
            std::os::unix::fs::symlink(&target, &link).unwrap();
            assert_eq!(read_bounded(&link), Err(JournalError::Invalid));
        }
        assert_eq!(read_bounded(&dir), Err(JournalError::Invalid));
        fs::remove_dir_all(dir).unwrap();
        let (front, prefix) = prefix();
        let mut value = envelope(&executable_sha256().unwrap(), front, prefix).unwrap();
        let raw = canonical(&value).unwrap();
        assert_eq!(validate_bytes(&raw), Ok(value.clone()));
        let mut corrupt = value.clone();
        corrupt["integrity_sha256"] = json!("00");
        assert_eq!(
            validate_bytes(&canonical(&corrupt).unwrap()),
            Err(JournalError::Invalid)
        );
        assert_eq!(
            validate_bytes(&raw[..raw.len() - 1]),
            Err(JournalError::Invalid)
        );
        for field in ["schema", "config", "frontier", "prefix", "prefix_sha256"] {
            let mut bad = value.clone();
            bad["body"].as_object_mut().unwrap().remove(field);
            assert_eq!(
                validate_bytes(&canonical(&bad).unwrap()),
                Err(JournalError::Invalid)
            );
        }
        for key in ["unexpected", "handlers", "source_executable_sha256"] {
            let mut bad = value.clone();
            if key == "unexpected" {
                bad["body"]["extra"] = json!(1);
            } else if key == "handlers" {
                bad["body"]["config"]["handlers"] = json!(["unknown"]);
            } else {
                bad["body"]["config"]["source_executable_sha256"] = json!("00");
            }
            let body = bad["body"].as_object().unwrap().clone();
            bad["integrity_sha256"] = json!(digest(&canonical(&Value::Object(body)).unwrap()));
            assert_eq!(
                validate_bytes(&canonical(&bad).unwrap()),
                Err(JournalError::Invalid)
            );
        }
        value.as_object_mut().unwrap();
    }

    #[test]
    fn fresh_process_replay_matches_uninterrupted_suffix() {
        let artifact = captured_artifact_path();
        write_fixture(&artifact).unwrap();
        let expected = run_suffix();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "c2_checkpoint_journal::tests::restore_child_entrypoint",
                "--nocapture",
            ])
            .env("KAIROS_C2_REPLAY_CHILD_ARTIFACT", &artifact)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        let marker = "C2_REPLAY_TRACE:";
        let actual = stdout
            .lines()
            .find_map(|line| line.strip_prefix(marker))
            .expect("child trace");
        assert_eq!(
            actual,
            String::from_utf8(canonical(&expected).unwrap()).unwrap()
        );
    }

    #[test]
    fn restore_child_entrypoint() {
        let Ok(path) = std::env::var("KAIROS_C2_REPLAY_CHILD_ARTIFACT") else {
            return;
        };
        let value = validate_bytes(&read_bounded(Path::new(&path)).unwrap()).unwrap();
        assert_eq!(value["body"]["schema"], SCHEMA);
        assert_eq!(value["body"]["frontier"], expected_frontier());
        let trace = run_suffix();
        println!(
            "C2_REPLAY_TRACE:{}",
            String::from_utf8(canonical(&trace).unwrap()).unwrap()
        );
    }
}
