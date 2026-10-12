//! C5 synthetic execution: actual Flow worlds, with study-owned CRN draws.
use crate::seed_map::{CalibrationSeedMap, SeedPurpose};
use crate::shadow::{ObservedEvent, ProbeAdapter, ResourcePolicy, Transition};
use crate::shadow_c2_model::{SyntheticFlowConfig, SyntheticFlowProbeModel};
use crate::shadow_ledger::{InitialResource, LedgerLimits, ObservedLedger};
use crate::shadow_native::NativeFlowAdapter;
use crate::trace_order::{EventKindRank, TraceOrderKeyV1};
use kairo_ecs_des::{
    FlowCheckpointCodecs, FlowCheckpointLimits, FlowRuntime, WorkHandlers, WorkState,
};
use kairo_ecs_types::{SimDuration, SimTime};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn error(e: impl std::fmt::Debug) -> String {
    format!("native: {e:?}")
}

fn next_preview_tick(
    flow: &FlowRuntime,
    codecs: &FlowCheckpointCodecs,
    limits: FlowCheckpointLimits,
) -> Result<Option<u128>, String> {
    let image = flow
        .capture_checkpoint_image(codecs, limits)
        .map_err(error)?;
    Ok(image
        .as_ref()
        .scheduler()
        .entries
        .iter()
        .filter(|entry| entry.live)
        .map(|entry| entry.request.at.ticks())
        .min())
}

fn number(v: &Value, key: &str) -> Result<u64, String> {
    v[key]
        .as_u64()
        .ok_or_else(|| format!("invalid integer {key}"))
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key]
        .as_str()
        .ok_or_else(|| format!("invalid string {key}"))
}
fn digest(v: &Value) -> String {
    Sha256::digest(v.to_string().as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(crate) fn verify_trace(config: &Value) -> Result<Value, String> {
    let p = crate::c5_study::plan(config)?;
    let tasks = p["tasks"].as_array().ok_or("plan tasks")?;
    let mut splits = BTreeMap::new();
    for task in tasks {
        if task["split"] == "confirmation" {
            continue;
        }
        splits
            .entry(string(task, "split")?.to_owned())
            .or_insert_with(|| task["records"].clone());
    }
    Ok(
        json!({"schema_version":"kairoecs.calibration.trace-report.v1","status":"valid","study_hash":p["study_hash"],"dataset_id":config["dataset_id"],"mapping_version":config["mapping_version"],"normalized_partitions":splits,"adapter":"synthetic_flow_probe.v1","scope":"synthetic normalized integer tick records"}),
    )
}

pub(crate) fn admitted(config: &Value, task: &Value) -> Result<(), String> {
    let p = crate::c5_study::plan(config)?;
    let mut expected = p["tasks"].as_array().ok_or("plan tasks")?.clone();
    if task["split"] == "final_test" {
        expected.extend(crate::c5_study::final_tasks(
            config,
            string(task, "candidate_id")?,
        )?);
    }
    if !expected.iter().any(|t| t == task) {
        return Err("task does not match immutable study plan".into());
    }
    Ok(())
}

fn sampled_work(config: &Value, task: &Value, row: &Value) -> Result<(u128, u64), String> {
    let mut seeds =
        CalibrationSeedMap::new(1, string(config, "study_id")?, number(config, "root_seed")?)
            .map_err(error)?;
    let mut stream = seeds
        .stream_for(
            string(task, "seed_schedule_id")?,
            number(task, "replication_id")?,
            string(row, "case_id")?,
            "service",
            SeedPurpose::Service,
        )
        .map_err(error)?;
    let seed = stream.derived_seed();
    let noise = config.get("service_noise_ticks").and_then(Value::as_array);
    let perturbation = if let Some(noise) = noise {
        if noise.is_empty() || noise.len() > 4096 {
            return Err("invalid noise inventory".into());
        }
        // Rejection sampling avoids modulo bias; draw count is bounded independently.
        let n = noise.len() as u64;
        let threshold = n.wrapping_neg() % n;
        let mut selected = None;
        for _ in 0..128 {
            let draw = stream.next_u64().map_err(error)?;
            if draw >= threshold {
                selected = Some((draw % n) as usize);
                break;
            }
        }
        noise[selected.ok_or("noise rejection cap")?]
            .as_i64()
            .ok_or("signed integer noise required")?
    } else {
        0
    };
    let ticks = i128::from(number(&task["params"], "work_ticks")?) + i128::from(perturbation);
    if ticks <= 0 {
        return Err("sampled work duration must be positive".into());
    }
    Ok((ticks as u128, seed))
}

fn snapshot(row: &Value) -> Result<crate::shadow::LedgerSnapshot, String> {
    let event = ObservedEvent {
        order: TraceOrderKeyV1 {
            relative_ticks: number(row, "start_tick")?.into(),
            case_key: string(row, "case_id")?.into(),
            occurrence: 0,
            event_kind_rank: EventKindRank::from_canonical_decimal("1").map_err(error)?,
            source_event_key: "anchor".into(),
            source_order: 0,
        },
        available_at: Some(number(row, "start_tick")?.into()),
        source_defined: true,
        transition: Transition::None,
        payload: vec![],
    };
    let mut ledger = ObservedLedger::new(
        vec![event],
        BTreeMap::from([(
            "resource".into(),
            InitialResource {
                capacity: 1,
                claims: vec![],
            },
        )]),
        vec!["Synthetic independent empty-resource anchor".into()],
        ResourcePolicy::Strict,
        LedgerLimits {
            max_events: 4,
            max_payload_bytes: 4096,
            max_assumptions: 4,
            max_identifier_bytes: 8192,
            max_initial_resources: 4,
            max_initial_claims: 4,
            max_assumption_bytes: 4096,
        },
    )
    .map_err(error)?;
    ledger
        .advance()
        .map_err(error)?
        .cloned()
        .ok_or("missing anchor".into())
}

fn shadow_case(config: &Value, task: &Value, row: &Value) -> Result<Value, String> {
    let (work, seed) = sampled_work(config, task, row)?;
    let route = number(&task["params"], "route_ticks")?;
    let at = u128::from(number(row, "start_tick")?);
    let horizon = at
        .checked_add(u128::from(number(task, "horizon_ticks")?))
        .ok_or("horizon overflow")?;
    let model = SyntheticFlowProbeModel {
        config: SyntheticFlowConfig {
            seed_schedule: string(task, "seed_schedule_id")?.into(),
            replication: number(task, "replication_id")?,
            case_key: string(row, "case_id")?.into(),
            work_duration_ticks: work,
            route_speed_mm_per_second: 1,
            route_length_mm: route,
            max_horizon_ticks: horizon,
            ..Default::default()
        },
    };
    let input = model.probe_input().map_err(error)?;
    let adapter = NativeFlowAdapter {
        model,
        max_image_bytes: 4 * 1024 * 1024,
    };
    let snap = snapshot(row)?;
    let mut world = adapter.start(&snap, &input).map_err(error)?;
    let mut predicted = None;
    let mut count = 0;
    let cap = number(task, "max_events")?;
    while count < cap {
        let Some(next) = adapter.next_tick(&world).map_err(error)? else {
            break;
        };
        if next > horizon {
            break;
        }
        let step = adapter.step(&mut world).map_err(error)?;
        count += step.dispatches;
        if let Some(failure) = step.failure {
            return Err(error(failure));
        }
        if let Some(target) = step.target {
            predicted = Some(target);
            break;
        }
    }
    Ok(
        json!({"case_id":row["case_id"],"predicted_tick":predicted.map(|n|n.to_string()),"observed_tick":row["end_tick"],"observed_censored":row["censored"],"predicted_transit_ticks":adapter.model.arrival_tick(&world).and_then(|n|n.checked_sub(at)).map(|n|n.to_string()),"study_service_seed":seed,"native_events":count,"snapshot_digest":snap.digest.iter().map(|b|format!("{b:02x}")).collect::<String>(),"outcome":if predicted.is_some(){"complete"}else{"censored"},"sampled_work_ticks":work.to_string(),"runtime_seed_contract":"sealed-c3-fixed-duration-v1; study randomness sampled before native admission"}),
    )
}

pub(crate) fn evaluate(config: &Value, task: &Value) -> Result<Value, String> {
    admitted(config, task)?;
    evaluate_admitted(config, task)
}
fn evaluate_admitted(config: &Value, task: &Value) -> Result<Value, String> {
    let rows = task["records"].as_array().ok_or("task records")?;
    let invalid = task["valid_candidate"] == false
        || number(&task["params"], "route_ticks")? == 0
        || number(&task["params"], "work_ticks")? == 0;
    let mut records = Vec::new();
    let mut completion = 0f64;
    let mut transit = 0f64;
    let mut eligible = 0u64;
    let mut transit_count = 0u64;
    let mut censored = 0u64;
    let mut missing = 0u64;
    let mut failed = 0u64;
    let free = if task["split"] == "final_test" && !invalid {
        Some(free_run(config, task))
    } else {
        None
    };
    for row in rows {
        if invalid {
            failed += 1;
            records.push(json!({"case_id":row["case_id"],"outcome":"invalid","reason":"zero parameter duration"}));
            continue;
        }
        let native = if let Some(Err(reason)) = &free {
            Err(reason.clone())
        } else if let Some(Ok(free)) = &free {
            free["records"]
                .as_array()
                .and_then(|r| r.iter().find(|r| r["case_id"] == row["case_id"]))
                .cloned()
                .ok_or_else(|| "free record missing".to_owned())
        } else {
            shadow_case(config, task, row)
        };
        match native {
            Err(reason) => {
                failed += 1;
                records.push(json!({"case_id":row["case_id"],"outcome":"failed","reason":reason}));
            }
            Ok(record) => {
                let pred = record["predicted_tick"]
                    .as_str()
                    .and_then(|n| n.parse::<u128>().ok());
                let transit_pair = row["transit_ticks"].as_u64().zip(
                    record["predicted_transit_ticks"]
                        .as_str()
                        .and_then(|s| s.parse::<u128>().ok()),
                );
                let need_transit = config["objective"]["transit_weight"]
                    .as_f64()
                    .unwrap_or(0.0)
                    > 0.0;
                if row["censored"] == true || pred.is_none() {
                    censored += 1;
                } else if need_transit && transit_pair.is_none() {
                    missing += 1;
                } else if let (Some(predicted), Some(observed)) = (pred, row["end_tick"].as_u64()) {
                    eligible += 1;
                    completion += predicted.abs_diff(observed.into()) as f64;
                    if let Some((observed, predicted)) = transit_pair {
                        transit_count += 1;
                        transit += predicted.abs_diff(observed.into()) as f64;
                    }
                } else {
                    missing += 1;
                }
                records.push(record);
            }
        }
    }
    let outcome = if invalid {
        "invalid"
    } else if failed > 0 {
        "failed"
    } else if censored > 0 {
        "censored"
    } else if missing > 0 || eligible == 0 {
        "missing"
    } else {
        "complete"
    };
    Ok(
        json!({"schema_version":"kairoecs.calibration.result.v1","task_id":task["task_id"],"study_hash":task["study_hash"],"candidate_id":task["candidate_id"],"replication_id":task["replication_id"],"split":task["split"],"seed_schedule_id":task["seed_schedule_id"],"params":task["params"],"outcome":outcome,"replay_role":if free.is_some(){"free"}else{"shadow"},"seed_map_version":1,"root_seed":config["root_seed"],"completion_error":if eligible>0 {Some(completion/eligible as f64)} else {None},"transit_error":if transit_count>0 {Some(transit/transit_count as f64)}else{None},"eligible_count":eligible,"censored_count":censored,"missing_count":missing,"failed_count":failed,"records":records}),
    )
}

// A distinct free-running shared FCFS resource. Arrivals/transit and work are
// inputs; observed completions are consulted only after simulation for reporting.
fn free_run(config: &Value, task: &Value) -> Result<Value, String> {
    let input = task["records"].as_array().ok_or("records")?;
    if input.len() > 4096 {
        return Err("free runtime row inventory cap".into());
    }
    let mut rows = input.clone();
    rows.sort_by_key(|r| {
        (
            r["start_tick"].as_u64().unwrap_or(0),
            r["case_id"].as_str().unwrap_or("").to_owned(),
        )
    });
    let route = number(&task["params"], "route_ticks")?;
    if route == 0 || number(&task["params"], "work_ticks")? == 0 {
        return Err("invalid free candidate".into());
    }
    let mut flow = FlowRuntime::new();
    flow.register_work_handlers("c5.free", WorkHandlers::<u32>::default())
        .map_err(error)?;
    let resource = flow.create_resource(1).map_err(error)?;
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context::<u32>(
            "c5.free",
            1,
            |v, cap| {
                if cap < 4 {
                    return Err(kairo_ecs_des::FlowCheckpointCodecError("cap".into()));
                }
                Ok(v.to_le_bytes().to_vec())
            },
            |b, _| {
                Ok(u32::from_le_bytes(b.try_into().map_err(|_| {
                    kairo_ecs_des::FlowCheckpointCodecError("u32".into())
                })?))
            },
        )
        .map_err(error)?;
    codecs
        .register_work_handlers::<u32>(
            "c5.free",
            kairo_ecs_des::FlowHandlerCodeIds::default(),
            WorkHandlers::default(),
        )
        .map_err(error)?;
    let mut pending = Vec::new();
    let mut horizon = 0;
    for row in rows {
        let (duration, seed) = sampled_work(config, task, &row)?;
        let start = u128::from(number(&row, "start_tick")?);
        horizon = horizon.max(start + u128::from(number(task, "horizon_ticks")?));
        let actor = flow.spawn_actor().map_err(error)?;
        let work = flow
            .create_work(actor, SimDuration::from_ticks(duration), "c5.free", 0u32)
            .map_err(error)?;
        flow.acquire(resource)
            .owner(actor)
            .timed_work(work)
            .at(SimTime::from_ticks(start + u128::from(route)))
            .submit()
            .map_err(error)?;
        pending.push((
            row,
            work,
            seed,
            start + u128::from(number(task, "horizon_ticks")?),
        ));
    }
    let mut completions = vec![None; pending.len()];
    let mut events = 0;
    while events < number(task, "max_events")? {
        let next = next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default())?;
        if next.is_none_or(|t| t > horizon) {
            break;
        }
        let Some(dispatch) = flow.step().map_err(error)? else {
            break;
        };
        events += 1;
        if let Some(e) = dispatch.error {
            return Err(error(e));
        }
        // completion_at is a pending deadline, cleared by native completion.
        // Capture the actual transition tick once, before the next dispatch.
        for (i, (_, work, _, cutoff)) in pending.iter().enumerate() {
            if completions[i].is_none()
                && flow.now().ticks() <= *cutoff
                && flow.work_progress(*work).map_err(error)?.state == WorkState::Completed
            {
                completions[i] = Some(flow.now().ticks());
            }
        }
    }
    let mut result = Vec::new();
    for ((row, _, seed, _), completed) in pending.into_iter().zip(completions) {
        let predicted = completed.map(|tick| tick.to_string());
        result.push(json!({"case_id":row["case_id"],"predicted_tick":predicted,"observed_tick":row["end_tick"],"observed_censored":row["censored"],"study_service_seed":seed,"native_events":events,"event_count_scope":"shared-task","predicted_transit_ticks":route.to_string(),"outcome":if predicted.is_some(){"complete"}else{"censored"}}));
    }
    Ok(
        json!({"mode":"free","resource_capacity":1,"records":result,"native_events":events,"prediction_uses_observed_completion":false}),
    )
}

pub(crate) fn replay(config: &Value, mode: &str) -> Result<Value, String> {
    if mode != "shadow" && mode != "free" {
        return Err("unsupported replay mode".into());
    }
    let p = crate::c5_study::plan(config)?;
    let task = p["tasks"]
        .as_array()
        .ok_or("tasks")?
        .iter()
        .find(|t| t["split"] == "validation" && t["valid_candidate"] != false)
        .ok_or("no valid replay task")?;
    let result = if mode == "shadow" {
        evaluate_admitted(config, task)?
    } else {
        free_run(config, task)?
    };
    Ok(
        json!({"schema_version":"kairoecs.calibration.replay.v1","study_hash":p["study_hash"],"mode":mode,"candidate_id":task["candidate_id"],"task_id":task["task_id"],"adapter":"synthetic_flow_probe.v1","result_hash":digest(&result),"result":result}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use kairo_ecs_des::{
        FlowAllocationMode, FlowCheckpointError, FlowCheckpointImage, FlowCommandV3,
        FlowHandlerCodeIds, FlowQueueOrderKey, FlowQueueOrdering, FlowRequestQueueOrderV4,
        RequestState, WorkState,
    };

    fn gated_preview_runtime() -> (
        FlowRuntime,
        FlowCheckpointCodecs,
        kairo_ecs_des::ResourceId,
        kairo_ecs_des::RequestId,
        kairo_ecs_des::WorkId,
    ) {
        let mut flow = FlowRuntime::new();
        flow.register_work_handlers("c5.free", WorkHandlers::<u32>::default())
            .unwrap();
        let resource = flow.create_resource(1).unwrap();
        let owner = flow.spawn_actor().unwrap();
        let work = flow
            .create_work(owner, SimDuration::from_ticks(5), "c5.free", 0u32)
            .unwrap();
        let request = flow
            .acquire(resource)
            .owner(owner)
            .timed_work(work)
            .at(SimTime::from_ticks(0))
            .submit()
            .unwrap();
        flow.schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(2), 0)
            .unwrap();
        flow.schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(7), 0)
            .unwrap();

        let mut codecs = FlowCheckpointCodecs::new();
        codecs
            .register_context::<u32>(
                "c5.free",
                1,
                |value, cap| {
                    if cap < 4 {
                        return Err(kairo_ecs_des::FlowCheckpointCodecError("cap".into()));
                    }
                    Ok(value.to_le_bytes().to_vec())
                },
                |bytes, _| {
                    Ok(u32::from_le_bytes(bytes.try_into().map_err(|_| {
                        kairo_ecs_des::FlowCheckpointCodecError("u32".into())
                    })?))
                },
            )
            .unwrap();
        codecs
            .register_work_handlers::<u32>(
                "c5.free",
                FlowHandlerCodeIds::default(),
                WorkHandlers::default(),
            )
            .unwrap();
        let dispatch = flow.step().unwrap().unwrap();
        assert_eq!(dispatch.at, SimTime::from_ticks(0));
        assert_eq!(dispatch.error, None);
        assert_eq!(flow.work_progress(work).unwrap().state, WorkState::Active);
        (flow, codecs, resource, request, work)
    }

    fn c5_preview_codecs() -> FlowCheckpointCodecs {
        let mut codecs = FlowCheckpointCodecs::new();
        codecs
            .register_context::<u32>(
                "c5.free",
                1,
                |value, cap| {
                    if cap < 4 {
                        return Err(kairo_ecs_des::FlowCheckpointCodecError("cap".into()));
                    }
                    Ok(value.to_le_bytes().to_vec())
                },
                |bytes, _| {
                    Ok(u32::from_le_bytes(bytes.try_into().map_err(|_| {
                        kairo_ecs_des::FlowCheckpointCodecError("u32".into())
                    })?))
                },
            )
            .unwrap();
        codecs
            .register_work_handlers::<u32>(
                "c5.free",
                FlowHandlerCodeIds::default(),
                WorkHandlers::default(),
            )
            .unwrap();
        codecs
    }

    fn count_c5_preview_completion(
        context: &mut u32,
        _: &kairo_ecs_des::FlowCallbackSnapshot,
        _: &mut kairo_ecs_des::FlowCommandSink,
    ) {
        *context += 1;
    }

    fn c5_preview_callback_codecs() -> FlowCheckpointCodecs {
        let mut codecs = c5_preview_codecs();
        codecs
            .register_work_continuations(
                "c5.free",
                kairo_ecs_des::FlowContinuationCodeIds {
                    on_complete: Some(kairo_ecs_des::FlowCallbackCodeV1 {
                        stable_id: "c5.preview.completion-counter".into(),
                        version: 1,
                    }),
                    ..Default::default()
                },
                kairo_ecs_des::FlowContinuations {
                    on_complete: Some(count_c5_preview_completion),
                    ..Default::default()
                },
            )
            .unwrap();
        codecs
    }

    fn deferred_preview_runtime(
        duration: u128,
        transition_limit: Option<std::num::NonZeroU64>,
    ) -> (
        FlowRuntime,
        FlowCheckpointCodecs,
        kairo_ecs_des::ResourceId,
        kairo_ecs_des::RequestId,
        kairo_ecs_des::WorkId,
    ) {
        let mut flow = FlowRuntime::with_config(kairo_ecs_des::FlowConfig {
            max_same_tick_flow_transitions: transition_limit
                .unwrap_or_else(|| std::num::NonZeroU64::new(100_000).unwrap()),
        });
        flow.register_work_handlers("c5.free", WorkHandlers::<u32>::default())
            .unwrap();
        let resource = flow
            .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
            .unwrap();
        let owner = flow.spawn_actor().unwrap();
        let work = flow
            .create_work(owner, SimDuration::from_ticks(duration), "c5.free", 7u32)
            .unwrap();
        let request = flow
            .acquire(resource)
            .owner(owner)
            .timed_work(work)
            .at(SimTime::from_ticks(0))
            .submit()
            .unwrap();
        let codecs = c5_preview_codecs();
        (flow, codecs, resource, request, work)
    }

    fn deferred_v4_preview_runtime(
        duration: u128,
        transition_limit: Option<std::num::NonZeroU64>,
        key: &[u8],
    ) -> (
        FlowRuntime,
        FlowCheckpointCodecs,
        kairo_ecs_des::ResourceId,
        kairo_ecs_des::RequestId,
        kairo_ecs_des::WorkId,
    ) {
        let mut flow = FlowRuntime::with_config(kairo_ecs_des::FlowConfig {
            max_same_tick_flow_transitions: transition_limit
                .unwrap_or_else(|| std::num::NonZeroU64::new(100_000).unwrap()),
        });
        flow.register_work_handlers("c5.free", WorkHandlers::<u32>::default())
            .unwrap();
        let resource = flow
            .create_resource_with_queue_ordering(
                1,
                FlowAllocationMode::Deferred,
                FlowQueueOrdering::StableKey,
            )
            .unwrap();
        let owner = flow.spawn_actor().unwrap();
        let work = flow
            .create_work(owner, SimDuration::from_ticks(duration), "c5.free", 7u32)
            .unwrap();
        let request = flow
            .acquire(resource)
            .owner(owner)
            .timed_work(work)
            .at(SimTime::from_ticks(0))
            .queue_order_key(FlowQueueOrderKey::new(key).unwrap())
            .submit()
            .unwrap();
        (flow, c5_preview_codecs(), resource, request, work)
    }

    fn c5_v4_key(image: &FlowCheckpointImage, request: kairo_ecs_des::RequestId, key: &[u8]) {
        let FlowCheckpointImage::V4(image) = image else {
            panic!("StableKey native preview must select V4");
        };
        assert!(image
            .queue_order_modes
            .iter()
            .any(|(_, mode)| *mode == FlowQueueOrdering::StableKey));
        assert!(image.request_queue_orders.iter().any(|(id, order)| {
            *id == request
                && matches!(order, FlowRequestQueueOrderV4::StableKey(bytes) if bytes == key)
        }));
    }

    fn stale_v4_preview_runtime() -> (
        FlowRuntime,
        FlowCheckpointCodecs,
        kairo_ecs_des::ResourceId,
        kairo_ecs_des::RequestId,
        kairo_ecs_des::WorkId,
        kairo_ecs_des::RequestId,
        kairo_ecs_des::WorkId,
    ) {
        let mut flow = FlowRuntime::new();
        flow.register_work_handlers("c5.free", WorkHandlers::<u32>::default())
            .unwrap();
        flow.register_work_continuations(
            "c5.free",
            kairo_ecs_des::FlowContinuations {
                on_complete: Some(count_c5_preview_completion),
                ..Default::default()
            },
        )
        .unwrap();
        let resource = flow
            .create_resource_with_queue_ordering(
                1,
                FlowAllocationMode::Deferred,
                FlowQueueOrdering::StableKey,
            )
            .unwrap();
        let low_owner = flow.spawn_actor().unwrap();
        let low_work = flow
            .create_work(low_owner, SimDuration::from_ticks(10), "c5.free", 11u32)
            .unwrap();
        let low = flow
            .acquire(resource)
            .owner(low_owner)
            .timed_work(low_work)
            .at(SimTime::from_ticks(1))
            .priority(10)
            .preemptible(kairo_ecs_des::PreemptionStrategy::Suspend)
            .queue_order_key(FlowQueueOrderKey::new(b"low").unwrap())
            .submit()
            .unwrap();
        let first_flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(1), i32::MAX)
            .unwrap();
        drain_c5_through(&mut flow, first_flush, SimTime::from_ticks(1));
        assert_eq!(flow.request(low).unwrap().state, RequestState::Active);
        let urgent_owner = flow.spawn_actor().unwrap();
        let urgent_work = flow
            .create_work(urgent_owner, SimDuration::from_ticks(2), "c5.free", 0u32)
            .unwrap();
        let urgent = flow
            .acquire(resource)
            .owner(urgent_owner)
            .timed_work(urgent_work)
            .at(SimTime::from_ticks(4))
            .priority(1)
            .can_preempt(true)
            .queue_order_key(FlowQueueOrderKey::new(b"urgent").unwrap())
            .submit()
            .unwrap();
        let preempt_flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(4), i32::MAX)
            .unwrap();
        drain_c5_through(&mut flow, preempt_flush, SimTime::from_ticks(4));
        assert_eq!(flow.request(low).unwrap().state, RequestState::Suspended);
        assert_eq!(flow.request(urgent).unwrap().state, RequestState::Active);
        let resume_flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(7), i32::MAX)
            .unwrap();
        let completion = flow.step().unwrap().expect("urgent completion at six");
        assert_eq!(completion.at, SimTime::from_ticks(6));
        assert!(completion.records.iter().any(|row| row.request == urgent
            && row.transition == kairo_ecs_des::LifecycleTransition::Completed));
        let notification = flow.step().unwrap().expect("completion notification");
        assert_eq!(notification.at, SimTime::from_ticks(6));
        assert_eq!(flow.work_context::<u32>(urgent_work), Ok(&1));
        drain_c5_through(&mut flow, resume_flush, SimTime::from_ticks(7));
        assert_eq!(flow.request(low).unwrap().state, RequestState::Active);
        (
            flow,
            c5_preview_callback_codecs(),
            resource,
            low,
            low_work,
            urgent,
            urgent_work,
        )
    }

    fn drain_c5_through(
        flow: &mut FlowRuntime,
        event: kairo_ecs_types::EventId,
        tick: SimTime,
    ) -> Vec<kairo_ecs_des::FlowDispatch> {
        let mut dispatches = Vec::new();
        for _ in 0..16 {
            let dispatch = flow.step().unwrap().expect("scheduled event closure");
            assert_eq!(dispatch.at, tick);
            let reached = dispatch.event == event;
            dispatches.push(dispatch);
            if reached {
                return dispatches;
            }
        }
        panic!("event closure exceeded bound");
    }

    fn stale_completion_runtime() -> (
        FlowRuntime,
        FlowCheckpointCodecs,
        kairo_ecs_des::ResourceId,
        kairo_ecs_des::RequestId,
        kairo_ecs_des::WorkId,
        kairo_ecs_des::RequestId,
        kairo_ecs_des::WorkId,
    ) {
        // Rebuild this fixture with a submit at tick one so useful completion is
        // tick fourteen, while its superseded tick-eleven event remains live.
        let mut flow = FlowRuntime::new();
        flow.register_work_handlers("c5.free", WorkHandlers::<u32>::default())
            .unwrap();
        flow.register_work_continuations(
            "c5.free",
            kairo_ecs_des::FlowContinuations {
                on_complete: Some(count_c5_preview_completion),
                ..Default::default()
            },
        )
        .unwrap();
        let resource = flow
            .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
            .unwrap();
        let low_owner = flow.spawn_actor().unwrap();
        let low_work = flow
            .create_work(low_owner, SimDuration::from_ticks(10), "c5.free", 0u32)
            .unwrap();
        let low = flow
            .acquire(resource)
            .owner(low_owner)
            .timed_work(low_work)
            .at(SimTime::from_ticks(1))
            .priority(10)
            .preemptible(kairo_ecs_des::PreemptionStrategy::Suspend)
            .submit()
            .unwrap();
        let first_flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(1), i32::MAX)
            .unwrap();
        drain_c5_through(&mut flow, first_flush, SimTime::from_ticks(1));
        assert_eq!(flow.request(low).unwrap().state, RequestState::Active);

        let urgent_owner = flow.spawn_actor().unwrap();
        let urgent_work = flow
            .create_work(urgent_owner, SimDuration::from_ticks(2), "c5.free", 0u32)
            .unwrap();
        let urgent = flow
            .acquire(resource)
            .owner(urgent_owner)
            .timed_work(urgent_work)
            .at(SimTime::from_ticks(4))
            .priority(1)
            .can_preempt(true)
            .submit()
            .unwrap();
        let preempt_flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(4), i32::MAX)
            .unwrap();
        drain_c5_through(&mut flow, preempt_flush, SimTime::from_ticks(4));
        assert_eq!(flow.request(low).unwrap().state, RequestState::Suspended);
        assert_eq!(flow.request(urgent).unwrap().state, RequestState::Active);

        let resume_flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(7), i32::MAX)
            .unwrap();
        // Consume the urgent useful completion at six before the explicit
        // deferred readmission at seven.
        let urgent_done = flow.step().unwrap().expect("urgent completion at six");
        assert_eq!(urgent_done.at, SimTime::from_ticks(6));
        assert!(urgent_done.records.iter().any(|record| {
            record.request == urgent
                && record.transition == kairo_ecs_des::LifecycleTransition::Completed
        }));
        let notification = flow.step().unwrap().expect("urgent completion callback");
        assert_eq!(notification.at, SimTime::from_ticks(6));
        assert_eq!(flow.work_context::<u32>(urgent_work), Ok(&1));
        drain_c5_through(&mut flow, resume_flush, SimTime::from_ticks(7));
        assert_eq!(flow.request(low).unwrap().state, RequestState::Active);
        assert_eq!(flow.now(), SimTime::from_ticks(7));
        (
            flow,
            c5_preview_callback_codecs(),
            resource,
            low,
            low_work,
            urgent,
            urgent_work,
        )
    }

    #[derive(Debug, Eq, PartialEq)]
    struct PreviewState {
        image: FlowCheckpointImage,
        now: SimTime,
        dispatched_events: u64,
        resource: kairo_ecs_des::ResourceSnapshot,
        grant_enabled: bool,
        request: kairo_ecs_des::ResourceRequest,
        work: kairo_ecs_des::WorkProgress,
    }

    fn preview_state(
        flow: &FlowRuntime,
        codecs: &FlowCheckpointCodecs,
        resource: kairo_ecs_des::ResourceId,
        request: kairo_ecs_des::RequestId,
        work: kairo_ecs_des::WorkId,
    ) -> PreviewState {
        PreviewState {
            image: flow
                .capture_checkpoint_image(codecs, FlowCheckpointLimits::default())
                .unwrap(),
            now: flow.now(),
            dispatched_events: flow.budget_snapshot().scheduler.dispatched_events,
            resource: flow.resource(resource).unwrap(),
            grant_enabled: flow.resource_grant_enabled(resource).unwrap(),
            request: flow.request(request).unwrap(),
            work: flow.work_progress(work).unwrap(),
        }
    }

    #[test]
    fn preview_v4_deferred_pending_flush_c5() {
        let (mut flow, codecs, resource, request, work) =
            deferred_v4_preview_runtime(5, None, b"flush");
        let submit = flow.step().unwrap().expect("Submit at zero");
        assert_eq!(submit.at, SimTime::from_ticks(0));
        assert_eq!(submit.error, None);
        assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);
        assert!(flow.request(request).unwrap().lease.is_none());
        let flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(4), i32::MAX)
            .unwrap();
        let before = preview_state(&flow, &codecs, resource, request, work);
        c5_v4_key(&before.image, request, b"flush");
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(4)
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            before
        );
        let granted = flow.step().unwrap().expect("allocation flush at four");
        assert_eq!(granted.event, flush);
        assert_eq!(granted.at, SimTime::from_ticks(4));
        assert_eq!(granted.error, None);
        assert_eq!(flow.request(request).unwrap().state, RequestState::Active);
        let after_grant = preview_state(&flow, &codecs, resource, request, work);
        c5_v4_key(&after_grant.image, request, b"flush");
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(9)
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            after_grant
        );
        let completed = flow.step().unwrap().expect("timed completion at nine");
        assert_eq!(completed.at, SimTime::from_ticks(9));
        assert_eq!(completed.error, None);
        assert_eq!(
            flow.request(request).unwrap().state,
            RequestState::Completed
        );
    }

    #[test]
    fn preview_v4_disabled_pending_submit_c5() {
        let mut flow = FlowRuntime::new();
        flow.register_work_handlers("c5.free", WorkHandlers::<u32>::default())
            .unwrap();
        let resource = flow
            .create_resource_with_queue_ordering(
                1,
                FlowAllocationMode::Deferred,
                FlowQueueOrdering::StableKey,
            )
            .unwrap();
        let mut requests = Vec::new();
        let mut works = Vec::new();
        for key in [b"z".as_slice(), b"a".as_slice()] {
            let owner = flow.spawn_actor().unwrap();
            let work = flow
                .create_work(owner, SimDuration::from_ticks(5), "c5.free", 0u32)
                .unwrap();
            let request = flow
                .acquire(resource)
                .owner(owner)
                .timed_work(work)
                .at(SimTime::from_ticks(0))
                .queue_order_key(FlowQueueOrderKey::new(key).unwrap())
                .submit()
                .unwrap();
            requests.push(request);
            works.push(work);
        }
        let codecs = c5_preview_codecs();
        let disable = flow
            .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(0), i32::MIN)
            .unwrap();
        let flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(0), i32::MAX)
            .unwrap();
        assert_eq!(
            flow.request(requests[0]).unwrap().state,
            RequestState::Pending
        );
        assert_eq!(
            flow.request(requests[1]).unwrap().state,
            RequestState::Pending
        );
        let before = preview_state(&flow, &codecs, resource, requests[0], works[0]);
        c5_v4_key(&before.image, requests[0], b"z");
        c5_v4_key(&before.image, requests[1], b"a");
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(0)
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, requests[0], works[0]),
            before
        );
        let disabled = flow.step().unwrap().expect("disable at zero");
        assert_eq!(disabled.event, disable);
        assert_eq!(disabled.error, None);
        assert!(!flow.resource_grant_enabled(resource).unwrap());
        for _ in 0..2 {
            let submitted = flow.step().unwrap().expect("pending submit at zero");
            assert_eq!(submitted.at, SimTime::from_ticks(0));
            assert_eq!(submitted.error, None);
        }
        assert_eq!(
            flow.request(requests[0]).unwrap().state,
            RequestState::Queued
        );
        assert_eq!(
            flow.request(requests[1]).unwrap().state,
            RequestState::Queued
        );
        let closed = flow.step().unwrap().expect("disabled allocation flush");
        assert_eq!(closed.event, flush);
        assert_eq!(closed.at, SimTime::from_ticks(0));
        assert_eq!(closed.error, None);
        assert!(flow.request(requests[0]).unwrap().lease.is_none());
        assert!(flow.request(requests[1]).unwrap().lease.is_none());
        let after = preview_state(&flow, &codecs, resource, requests[0], works[0]);
        c5_v4_key(&after.image, requests[0], b"z");
        c5_v4_key(&after.image, requests[1], b"a");
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            None
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, requests[0], works[0]),
            after
        );
        let enable = flow
            .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(1), i32::MIN)
            .unwrap();
        let grant_flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(1), i32::MAX)
            .unwrap();
        let enabled = flow.step().unwrap().expect("enable at one");
        assert_eq!(enabled.event, enable);
        assert_eq!(enabled.error, None);
        let grant = flow.step().unwrap().expect("enabled flush at one");
        assert_eq!(grant.event, grant_flush);
        assert_eq!(grant.error, None);
        assert_eq!(
            flow.request(requests[1]).unwrap().state,
            RequestState::Active
        );
        assert_eq!(
            flow.request(requests[0]).unwrap().state,
            RequestState::Queued
        );
        assert!(flow.request(requests[0]).unwrap().lease.is_none());
    }

    #[test]
    fn preview_v4_repeated_halt_c5() {
        let (mut flow, codecs, resource, request, work) =
            deferred_v4_preview_runtime(5, std::num::NonZeroU64::new(1), b"halt");
        let submit = flow.step().unwrap().expect("Submit at zero");
        assert_eq!(submit.error, None);
        let flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(0), i32::MAX)
            .unwrap();
        let before = preview_state(&flow, &codecs, resource, request, work);
        c5_v4_key(&before.image, request, b"halt");
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(0)
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            before
        );
        let expected = kairo_ecs_des::FlowError::SameTickBudgetExceeded {
            at_ticks: 0,
            limit: 1,
        };
        let first = flow.step().unwrap_err();
        assert_eq!(first, expected);
        let halted = preview_state(&flow, &codecs, resource, request, work);
        assert!(
            matches!(&halted.image, FlowCheckpointImage::V4(image) if image.budget_halt.is_some())
        );
        c5_v4_key(&halted.image, request, b"halt");
        assert_eq!(flow.peek_next().expect("live scheduler event").id, flush);
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(0)
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            halted
        );
        let second = flow.step().unwrap_err();
        assert_eq!(second, expected);
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            halted
        );
    }

    #[test]
    fn preview_v4_stale_completion_and_limits_c5() {
        let (mut flow, codecs, resource, low, low_work, urgent, urgent_work) =
            stale_v4_preview_runtime();
        assert_eq!(flow.work_context::<u32>(low_work), Ok(&11));
        assert_eq!(flow.work_context::<u32>(urgent_work), Ok(&1));
        let before = preview_state(&flow, &codecs, resource, low, low_work);
        c5_v4_key(&before.image, low, b"low");
        let stale = match &before.image {
            FlowCheckpointImage::V4(image) => image
                .commands
                .iter()
                .find_map(|(id, command)| {
                    matches!(command, FlowCommandV3::Completion(request, _, _, at)
                    if *request == low && *at == SimTime::from_ticks(11))
                    .then_some(*id)
                })
                .expect("live stale completion command at eleven"),
            _ => panic!("keyed runtime must capture V4"),
        };
        let preview = flow.peek_next().expect("live scheduler event");
        assert_eq!(preview.id, stale);
        assert_eq!(preview.at, SimTime::from_ticks(11));
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(11)
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, low, low_work),
            before
        );
        let no_op = flow.step().unwrap().expect("stale completion dispatch");
        assert_eq!(no_op.event, stale);
        assert_eq!(no_op.at, SimTime::from_ticks(11));
        assert_eq!(no_op.error, None);
        assert!(no_op.records.is_empty());
        assert_eq!(flow.request(urgent).unwrap().state, RequestState::Completed);
        assert_eq!(
            flow.work_progress(urgent_work).unwrap().state,
            WorkState::Completed
        );
        assert_eq!(flow.work_context::<u32>(low_work), Ok(&11));
        assert_eq!(flow.work_context::<u32>(urgent_work), Ok(&1));
        assert_eq!(
            flow.work_progress(low_work).unwrap().useful_elapsed,
            SimDuration::from_ticks(7)
        );
        assert_eq!(
            flow.work_progress(low_work).unwrap().remaining,
            SimDuration::from_ticks(3)
        );
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(14)
        );
        let useful = flow.step().unwrap().expect("useful completion at fourteen");
        assert_eq!(useful.at, SimTime::from_ticks(14));
        assert_eq!(useful.error, None);

        let (flow, codecs, resource, request, work) =
            deferred_v4_preview_runtime(5, None, b"limit");
        let before = preview_state(&flow, &codecs, resource, request, work);
        c5_v4_key(&before.image, request, b"limit");
        let limits = FlowCheckpointLimits {
            max_commands: 0,
            ..FlowCheckpointLimits::default()
        };
        let expected = flow.capture_checkpoint_image(&codecs, limits).unwrap_err();
        assert!(matches!(expected, FlowCheckpointError::LimitExceeded(_)));
        assert_eq!(
            next_preview_tick(&flow, &codecs, limits),
            Err(error(expected))
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            before
        );
    }

    fn study() -> Value {
        serde_json::from_str(include_str!("../../../examples/calibration/c5/study.json")).unwrap()
    }
    fn task(c: &Value) -> Value {
        crate::c5_study::plan(c).unwrap()["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| {
                t["split"] == "validation"
                    && t["params"]["route_ticks"] == 2
                    && t["params"]["work_ticks"] == 3
            })
            .unwrap()
            .clone()
    }
    #[test]
    fn c5_native_actual_flow_recovers_known_route_and_work() {
        let c = study();
        let t = task(&c);
        let result = evaluate(&c, &t).unwrap();
        assert_eq!(result["outcome"], "complete", "{result}");
        assert_eq!(result["completion_error"], 0.0);
        assert_eq!(result["transit_error"], 0.0);
        assert_eq!(result["records"][0]["predicted_tick"], "105");
        assert!(result["records"][0]["native_events"].as_u64().unwrap() > 0);
    }
    #[test]
    fn c5_native_task_forgery_fails_before_execution() {
        let c = study();
        let mut t = task(&c);
        t["params"]["route_ticks"] = json!(99);
        assert!(evaluate(&c, &t).unwrap_err().contains("immutable"));
    }
    #[test]
    fn c5_native_horizon_and_event_cap_retain_failed_outcomes() {
        for (name, value) in [("horizon_ticks", 1), ("max_events", 1)] {
            let mut c = study();
            c["budget"][name] = json!(value);
            let t = task(&c);
            let result = evaluate(&c, &t).unwrap();
            assert_eq!(result["outcome"], "censored");
            assert_eq!(result["eligible_count"], 0);
            assert_eq!(result["censored_count"], 1);
        }
    }
    #[test]
    fn c5_native_observed_completion_never_drives_prediction() {
        let c = study();
        let a = evaluate(&c, &task(&c)).unwrap();
        let mut changed = c.clone();
        changed["trace"][1]["end_tick"] = json!(109);
        let b = evaluate(&changed, &task(&changed)).unwrap();
        assert_eq!(
            a["records"][0]["predicted_tick"],
            b["records"][0]["predicted_tick"]
        );
        assert_eq!(
            a["records"][0]["snapshot_digest"],
            b["records"][0]["snapshot_digest"]
        );
        assert_eq!(b["completion_error"], 4.0);
    }
    #[test]
    fn c5_native_crn_draws_are_paired_and_repeatable() {
        let mut c = study();
        c["service_noise_ticks"] = json!([-1, 0, 1]);
        let p = crate::c5_study::plan(&c).unwrap();
        let tasks = p["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|t| t["split"] == "validation" && t["replication_id"] == 0)
            .collect::<Vec<_>>();
        let results = tasks
            .iter()
            .map(|t| evaluate(&c, t).unwrap())
            .collect::<Vec<_>>();
        for (i, result) in results.iter().enumerate() {
            assert_eq!(result["outcome"], "complete", "{result}");
            assert_eq!(
                result["records"][0]["study_service_seed"],
                results[0]["records"][0]["study_service_seed"]
            );
            assert_eq!(result, &evaluate(&c, tasks[i]).unwrap());
        }
    }
    #[test]
    fn c5_native_free_queue_is_coupled_and_observation_independent() {
        let mut c = study();
        c["parameters"]["route_ticks"]["values"] = json!([2]);
        c["parameters"]["work_ticks"]["values"] = json!([3]);
        c["trace"].as_array_mut().unwrap().push(json!({"case_id":"validation-2","day":"validation-1","start_tick":100,"end_tick":105,"transit_ticks":2}));
        let s = replay(&c, "shadow").unwrap();
        let f = replay(&c, "free").unwrap();
        assert_eq!(s["result"]["records"][0]["predicted_tick"], "105");
        assert_eq!(s["result"]["records"][1]["predicted_tick"], "105");
        assert_eq!(f["result"]["records"][0]["predicted_tick"], "105");
        assert_eq!(f["result"]["records"][1]["predicted_tick"], "108");
        c["trace"][1]["end_tick"] = json!(190);
        let changed = replay(&c, "free").unwrap();
        assert_eq!(changed["result"]["records"][1]["predicted_tick"], "108");
    }

    #[test]
    fn c5_native_preview_preserves_pending_disable_and_completion() {
        let (mut flow, codecs, resource, request, work) = gated_preview_runtime();
        assert!(matches!(
            flow.capture_checkpoint(&codecs, FlowCheckpointLimits::default()),
            Err(FlowCheckpointError::InvalidState(_))
        ));
        let before = preview_state(&flow, &codecs, resource, request, work);
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(2)
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            before
        );
        assert_eq!(before.now, SimTime::from_ticks(0));
        assert_eq!(before.request.state, RequestState::Active);
        assert_eq!(before.work.state, WorkState::Active);
        assert_eq!(before.resource.active.len(), 1);
        assert!(before.grant_enabled);

        let disable = flow.step().unwrap().unwrap();
        assert_eq!(disable.at, SimTime::from_ticks(2));
        assert_eq!(disable.error, None);
        assert!(!flow.resource_grant_enabled(resource).unwrap());
        let before = preview_state(&flow, &codecs, resource, request, work);
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(5)
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            before
        );
        assert_eq!(before.now, SimTime::from_ticks(2));
        assert_eq!(before.request.state, RequestState::Active);
        assert_eq!(before.work.state, WorkState::Active);
        assert_eq!(before.resource.active.len(), 1);
        assert!(!before.grant_enabled);

        let completion = flow.step().unwrap().unwrap();
        assert_eq!(completion.at, SimTime::from_ticks(5));
        assert_eq!(completion.error, None);
        assert_eq!(
            flow.work_progress(work).unwrap().state,
            WorkState::Completed
        );
    }

    #[test]
    fn c5_native_preview_limit_error_propagates_without_v1_fallback() {
        let (flow, codecs, _, _, _) = gated_preview_runtime();
        let limits = FlowCheckpointLimits {
            max_commands: 0,
            ..FlowCheckpointLimits::default()
        };
        let expected = flow.capture_checkpoint_image(&codecs, limits).unwrap_err();
        assert!(matches!(expected, FlowCheckpointError::LimitExceeded(_)));
        assert_eq!(
            next_preview_tick(&flow, &codecs, limits),
            Err(error(expected))
        );
    }

    #[test]
    fn preview_v3_deferred_pending_flush_without_advancement() {
        let (mut flow, codecs, resource, request, work) = deferred_preview_runtime(5, None);
        let submit = flow.step().unwrap().expect("Submit at zero");
        assert_eq!(submit.at, SimTime::from_ticks(0));
        assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);
        assert!(flow.request(request).unwrap().lease.is_none());
        let flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(4), i32::MAX)
            .unwrap();

        let before = preview_state(&flow, &codecs, resource, request, work);
        assert!(matches!(before.image, FlowCheckpointImage::V3(_)));
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(4)
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            before
        );

        let admission = flow.step().unwrap().expect("Flush at four");
        assert_eq!(admission.event, flush);
        assert_eq!(admission.at, SimTime::from_ticks(4));
        assert!(admission.records.iter().any(|record| {
            record.request == request
                && record.transition == kairo_ecs_des::LifecycleTransition::Granted
        }));
        assert_eq!(flow.request(request).unwrap().state, RequestState::Active);
        let before = preview_state(&flow, &codecs, resource, request, work);
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(9)
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            before
        );

        let completion = flow.step().unwrap().expect("useful completion at nine");
        assert_eq!(completion.at, SimTime::from_ticks(9));
        assert!(completion.records.iter().any(|record| {
            record.request == request
                && record.transition == kairo_ecs_des::LifecycleTransition::Completed
        }));
        assert_eq!(
            flow.work_progress(work).unwrap().state,
            WorkState::Completed
        );

        let (mut stale, stale_codecs, stale_resource, low, low_work, _, callback_work) =
            stale_completion_runtime();
        let callback_context_before = (
            *stale.work_context::<u32>(low_work).unwrap(),
            *stale.work_context::<u32>(callback_work).unwrap(),
        );
        assert_eq!(callback_context_before, (0, 1));
        let before = preview_state(&stale, &stale_codecs, stale_resource, low, low_work);
        assert_eq!(
            next_preview_tick(&stale, &stale_codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(11)
        );
        assert_eq!(
            preview_state(&stale, &stale_codecs, stale_resource, low, low_work),
            before
        );
        assert_eq!(
            (
                *stale.work_context::<u32>(low_work).unwrap(),
                *stale.work_context::<u32>(callback_work).unwrap(),
            ),
            callback_context_before
        );
        let stale_image = stale
            .capture_checkpoint_image(&stale_codecs, FlowCheckpointLimits::default())
            .unwrap();
        let FlowCheckpointImage::V3(stale_v3) = stale_image else {
            panic!("Deferred stale completion must capture as V3");
        };
        let stored_progress = stale_v3
            .builtins
            .work_progress
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .find(|(id, _)| *id == low_work.entity_id())
            .unwrap()
            .1
            .clone();
        let stale_dispatch = stale
            .step()
            .unwrap()
            .expect("live stale completion at eleven");
        assert_eq!(stale_dispatch.at, SimTime::from_ticks(11));
        assert!(stale_dispatch.records.is_empty());
        assert_eq!(
            (
                *stale.work_context::<u32>(low_work).unwrap(),
                *stale.work_context::<u32>(callback_work).unwrap(),
            ),
            callback_context_before
        );
        let after_image = stale
            .capture_checkpoint_image(&stale_codecs, FlowCheckpointLimits::default())
            .unwrap();
        let FlowCheckpointImage::V3(after_v3) = after_image else {
            panic!("stale dispatch remains a V3 image");
        };
        assert_eq!(
            after_v3
                .builtins
                .work_progress
                .as_ref()
                .unwrap()
                .rows
                .iter()
                .find(|(id, _)| *id == low_work.entity_id())
                .unwrap()
                .1,
            stored_progress
        );
        assert_eq!(
            stale.work_progress(low_work).unwrap().useful_elapsed,
            SimDuration::from_ticks(7)
        );
        assert_eq!(
            stale.work_progress(low_work).unwrap().remaining,
            SimDuration::from_ticks(3)
        );
        assert_eq!(
            next_preview_tick(&stale, &stale_codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(14)
        );
        let useful_completion = stale
            .step()
            .unwrap()
            .expect("new lease completion at fourteen");
        assert_eq!(useful_completion.at, SimTime::from_ticks(14));
        assert!(useful_completion.records.iter().any(|record| {
            record.request == low
                && record.transition == kairo_ecs_des::LifecycleTransition::Completed
        }));
    }

    #[test]
    fn preview_v3_disabled_queue_has_no_invented_completion() {
        let mut flow = FlowRuntime::new();
        flow.register_work_handlers("c5.free", WorkHandlers::<u32>::default())
            .unwrap();
        let resource = flow
            .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
            .unwrap();
        let disable = flow
            .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(0), i32::MIN)
            .unwrap();
        let owner = flow.spawn_actor().unwrap();
        let work = flow
            .create_work(owner, SimDuration::from_ticks(5), "c5.free", 1u32)
            .unwrap();
        let request = flow
            .acquire(resource)
            .owner(owner)
            .timed_work(work)
            .at(SimTime::from_ticks(0))
            .submit()
            .unwrap();
        let flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(0), i32::MAX)
            .unwrap();
        let codecs = c5_preview_codecs();
        let pending = preview_state(&flow, &codecs, resource, request, work);
        assert_eq!(pending.request.state, RequestState::Pending);
        assert!(pending.request.lease.is_none());
        let pending_image = flow
            .capture_checkpoint_image(&codecs, FlowCheckpointLimits::default())
            .unwrap();
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(0)
        );
        assert_eq!(
            flow.capture_checkpoint_image(&codecs, FlowCheckpointLimits::default())
                .unwrap(),
            pending_image
        );

        let disable_dispatch = flow.step().unwrap().expect("Disable event");
        assert_eq!(disable_dispatch.event, disable);
        assert_eq!(disable_dispatch.at, SimTime::from_ticks(0));
        assert_eq!(flow.resource_grant_enabled(resource), Ok(false));
        let submit_event = flow
            .capture_checkpoint_image(&codecs, FlowCheckpointLimits::default())
            .unwrap()
            .as_ref()
            .commands()
            .find_map(|(event, command)| {
                (command == kairo_ecs_des::FlowCommandV3::Submit(request)).then_some(event)
            })
            .expect("live Submit command");
        let submit_dispatch = flow.step().unwrap().expect("Submit event");
        assert_eq!(submit_dispatch.event, submit_event);
        assert_eq!(submit_dispatch.at, SimTime::from_ticks(0));
        assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);
        assert!(flow.request(request).unwrap().lease.is_none());
        let flush_dispatch = flow.step().unwrap().expect("Flush event");
        assert_eq!(flush_dispatch.event, flush);
        assert_eq!(flush_dispatch.at, SimTime::from_ticks(0));
        assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);
        assert!(flow.request(request).unwrap().lease.is_none());
        assert!(flow.resource(resource).unwrap().active.is_empty());
        let after_closure = preview_state(&flow, &codecs, resource, request, work);
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            None
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            after_closure
        );
    }

    #[test]
    fn preview_v3_halted_live_tick_and_repeat_step_error() {
        let limit = std::num::NonZeroU64::new(1).unwrap();
        let (mut flow, codecs, resource, request, work) = deferred_preview_runtime(4, Some(limit));
        let submit = flow
            .step()
            .unwrap()
            .expect("Submit consumes the tick budget");
        assert_eq!(submit.at, SimTime::from_ticks(0));
        assert_eq!(flow.request(request).unwrap().state, RequestState::Queued);
        let flush = flow
            .schedule_allocation_flush(resource, flow.now(), i32::MAX)
            .unwrap();
        let before_halt = preview_state(&flow, &codecs, resource, request, work);
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(0)
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            before_halt
        );
        let first_error = flow.step().unwrap_err();
        assert!(matches!(
            first_error,
            kairo_ecs_des::FlowError::SameTickBudgetExceeded {
                at_ticks: 0,
                limit: 1
            }
        ));
        let halted_budget = flow.budget_snapshot();
        let halted = preview_state(&flow, &codecs, resource, request, work);
        let FlowCheckpointImage::V3(halted_image) = &halted.image else {
            panic!("halted Deferred image must be V3");
        };
        assert!(halted_image.budget_halt.is_some());
        assert!(halted_image.commands.iter().any(|(event, command)| {
            *event == flush && *command == kairo_ecs_des::FlowCommandV3::AllocationFlush(resource)
        }));
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(0)
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            halted
        );
        assert_eq!(flow.step().unwrap_err(), first_error);
        assert_eq!(flow.budget_snapshot(), halted_budget);
        assert_eq!(
            flow.capture_checkpoint_image(&codecs, FlowCheckpointLimits::default())
                .unwrap(),
            halted.image
        );
        assert_eq!(
            next_preview_tick(&flow, &codecs, FlowCheckpointLimits::default()).unwrap(),
            Some(0)
        );
        assert_eq!(
            preview_state(&flow, &codecs, resource, request, work),
            halted
        );
    }

    #[test]
    fn preview_v3_capture_limit_error_without_legacy_fallback() {
        let (flow, codecs, _, _, _) = deferred_preview_runtime(5, None);
        let valid = flow
            .capture_checkpoint_image(&codecs, FlowCheckpointLimits::default())
            .unwrap();
        assert!(matches!(valid, FlowCheckpointImage::V3(_)));
        let limits = FlowCheckpointLimits {
            max_commands: 0,
            ..FlowCheckpointLimits::default()
        };
        let expected = flow.capture_checkpoint_image(&codecs, limits).unwrap_err();
        assert!(matches!(expected, FlowCheckpointError::LimitExceeded(_)));
        assert_eq!(
            next_preview_tick(&flow, &codecs, limits),
            Err(error(expected))
        );
    }
    #[test]
    fn c5_native_complete_study_and_final_holdout_are_separate() {
        let c = study();
        let p = crate::c5_study::plan(&c).unwrap();
        let mut attempts = p["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| evaluate(&c, t).unwrap())
            .collect::<Vec<_>>();
        let before = crate::c5_study::summarize(&c, &attempts).unwrap();
        assert_eq!(before["complete"], false);
        assert_eq!(before["winner"], "c000004", "{before}");
        for t in before["next_tasks"].as_array().unwrap() {
            attempts.push(evaluate(&c, t).unwrap());
        }
        let after = crate::c5_study::summarize(&c, &attempts).unwrap();
        assert_eq!(after["complete"], true);
        assert_eq!(after["final_test"]["mean_completion_error"], 0.0);
        assert_eq!(after["final_test"]["censored_count"], 0);
        assert_eq!(before["ranking"], after["ranking"]);
        attempts.reverse();
        assert_eq!(crate::c5_study::summarize(&c, &attempts).unwrap(), after);
    }
    #[test]
    fn c5_native_noisy_recovery_and_confounded_ridge_are_distinct() {
        let mut c = study();
        c["service_noise_ticks"] = json!([-1, 0, 1]);
        c["budget"]["replications"] = json!(8);
        let summarize_native = |c: &Value| {
            let p = crate::c5_study::plan(c).unwrap();
            let results = p["tasks"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| {
                    let r = evaluate(c, t).unwrap();
                    assert_eq!(r["outcome"], "complete", "{r}");
                    r
                })
                .collect::<Vec<_>>();
            crate::c5_study::summarize(c, &results).unwrap()
        };
        let noisy = summarize_native(&c);
        let winner = noisy["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["candidate_id"] == noisy["winner"])
            .unwrap();
        assert_eq!(winner["params"]["route_ticks"], 2);
        assert!(winner["params"]["work_ticks"].as_u64().unwrap().abs_diff(3) <= 1);
        assert_eq!(noisy["mc_indeterminate"], true);
        let mut c = study();
        c["objective"]["transit_weight"] = json!(0);
        for row in c["trace"].as_array_mut().unwrap() {
            row["transit_ticks"] = Value::Null;
        }
        let ridge = summarize_native(&c);
        assert_eq!(ridge["ridge"], true);
        assert_eq!(ridge["identifiability"], "non_identifiable_ridge");
        assert_eq!(ridge["ranking"][0]["score"], 0.0);
    }
    #[test]
    fn c5_native_free_runtime_failure_is_a_durable_attempt() {
        let mut c = study();
        c["service_noise_ticks"] = json!([-100]);
        let t = crate::c5_study::final_tasks(&c, "c000004")
            .unwrap()
            .remove(0);
        let r = evaluate(&c, &t).unwrap();
        assert_eq!(r["outcome"], "failed");
        assert_eq!(r["failed_count"], 1);
        assert_eq!(r["eligible_count"], 0);
    }
    #[test]
    fn c5_native_free_cutoff_remains_per_case_when_later_cases_extend_run() {
        let mut c = study();
        c["parameters"]["route_ticks"]["values"] = json!([2]);
        c["parameters"]["work_ticks"]["values"] = json!([3]);
        c["budget"]["horizon_ticks"] = json!(4);
        c["trace"].as_array_mut().unwrap().push(json!({"case_id":"validation-later","day":"validation-1","start_tick":120,"end_tick":125,"transit_ticks":2}));
        let r = replay(&c, "free").unwrap();
        assert_eq!(r["result"]["records"][0]["outcome"], "censored");
        assert_eq!(r["result"]["records"][0]["predicted_tick"], Value::Null);
        assert!(r["result"]["native_events"].as_u64().unwrap() > 1);
    }
}
