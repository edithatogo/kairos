//! ADR-0009 first-leaf oracle for the C2.1 paired Macro / explicit-zero Micro fixture.
//!
//! Event *k* is the measured `dispatched_events` count after `run_for(128)` at
//! Kairos `7af989c8951c232f7bc4e6a2f543c4ee0061af99`. The constant below is that
//! measurement, checked again on every run. `128` is only the run cap.

use super::*;
use crate::checkpoint_envelope::{self, CheckpointEnvelopeError};
use crate::checkpoint_sections::C2SectionDirectoryError;
use crate::flow_bridge::{
    AcquireIntent, PreparationIdentity, TransitRequest, WorkPreparationInput,
};
use crate::seed_map::{CalibrationSeedMap, CalibrationStreamKey, SeedPurpose};
use crate::work_duration::{IntrinsicDurationDistribution, IntrinsicWorkProvider};
use kairo_ecs_des::fidelity::{FidelityAdapter, FidelityMode, FidelityPolicy};
use kairo_ecs_des::{
    FlowCheckpointCodecError, FlowCheckpointCodecs, FlowCheckpointRebindV1,
    FlowContinuationCodeIds, FlowContinuations, FlowRun, LifecycleTransition, RequestState,
};
use kairo_ecs_types::{EntityId, SimDuration, SimTime};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Both modes of
/// `macro_and_explicit_zero_micro_pair_actual_provider_work_without_transit_events_or_draws`
/// reported this `budget_snapshot().scheduler.dispatched_events` after `run_for(128)`.
/// Measured on Rust 1.99.0 at Kairos `7af989c8951c232f7bc4e6a2f543c4ee0061af99`:
/// Macro and explicit-zero Micro each returned 2. `128` remains only the run cap.
const MEASURED_EVENT_K: u64 = 2;

const WORK_REGISTRATION: &str = "paired-work";
const SERVICE_TASK: &str = "triage:1";
const CHILD_FILE: &str = "KAIROS_ADR0009_FILE";
const CHILD_MODE: &str = "KAIROS_ADR0009_MODE";
const RESULT_MARKER: &str = "ADR0009_ORACLE ";
const KAIROS_PIN: &str = "7af989c8951c232f7bc4e6a2f543c4ee0061af99";

#[derive(Clone, Debug, Eq, PartialEq)]
struct Template {
    marker: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Context {
    marker: u64,
}

fn make_context(template: &Template) -> Context {
    Context {
        marker: template.marker,
    }
}

fn encode_u64(value: &u64, max_bytes: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    if max_bytes < 8 {
        return Err(FlowCheckpointCodecError("context byte cap".to_owned()));
    }
    Ok(value.to_le_bytes().to_vec())
}

fn decode_u64(bytes: &[u8], _: &FlowCheckpointRebindV1) -> Result<u64, FlowCheckpointCodecError> {
    let raw: [u8; 8] = bytes
        .try_into()
        .map_err(|_| FlowCheckpointCodecError("invalid paired marker".to_owned()))?;
    Ok(u64::from_le_bytes(raw))
}

fn encode_template(
    value: &Template,
    max_bytes: usize,
) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    encode_u64(&value.marker, max_bytes)
}

fn decode_template(
    bytes: &[u8],
    view: &FlowCheckpointRebindV1,
) -> Result<Template, FlowCheckpointCodecError> {
    Ok(Template {
        marker: decode_u64(bytes, view)?,
    })
}

fn encode_context(value: &Context, max_bytes: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    encode_u64(&value.marker, max_bytes)
}

fn decode_context(
    bytes: &[u8],
    view: &FlowCheckpointRebindV1,
) -> Result<Context, FlowCheckpointCodecError> {
    Ok(Context {
        marker: decode_u64(bytes, view)?,
    })
}

fn provider() -> IntrinsicWorkProvider {
    IntrinsicWorkProvider::new(
        1,
        vec![(
            "triage".to_owned(),
            IntrinsicDurationDistribution::weighted_ticks(vec![(10, 3), (20, 2), (30, 5)]).unwrap(),
        )],
    )
    .unwrap()
}

struct Scenario {
    flow: FlowRuntime,
    adapter: FidelityAdapter,
    provider: IntrinsicWorkProvider,
    seeds: CalibrationSeedMap,
    submitted: SubmittedIntrinsicWork<Template, Context>,
    service_key: CalibrationStreamKey,
}

fn scenario(mode: FidelityMode) -> Scenario {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let _carrier = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    flow.register_work_continuations(WORK_REGISTRATION, FlowContinuations::<Context>::default())
        .unwrap();

    let mut seeds = CalibrationSeedMap::new(1, "study-α", 1234).unwrap();
    let service_key = seeds
        .key_for("crn-v1", 7, "case-0001", SERVICE_TASK, SeedPurpose::Service)
        .unwrap();
    let expected = seeds
        .key_for("crn-v1", 7, "case-0001", SERVICE_TASK, SeedPurpose::Service)
        .unwrap();
    let service = seeds
        .stream_for("crn-v1", 7, "case-0001", SERVICE_TASK, SeedPurpose::Service)
        .unwrap();
    let policy = FidelityPolicy::new(1, Some(mode)).unwrap();
    let mut adapter = FidelityAdapter::new(policy);
    let work_provider = provider();
    let input = WorkPreparationInput::new(
        PreparationIdentity {
            owner,
            subsystem: "triage".to_owned(),
            registration: WORK_REGISTRATION.to_owned(),
            stratum: "triage".to_owned(),
        },
        service,
        expected,
        Template { marker: 0xC20 },
        make_context,
        AcquireIntent {
            resource,
            owner,
            at: SimTime::from_ticks(0),
            priority_level: 0,
            deadline: None,
            scheduler_priority: 0,
            can_preempt: false,
            preemptible: None,
        },
        TransitRequest::Zero,
    );
    let prepared = input
        .prepare(&flow, &mut adapter, &work_provider)
        .unwrap_or_else(|_| panic!("provider preparation against actual Flow failed"));
    assert_eq!(prepared.sampled_duration(), SimDuration::from_ticks(30));
    assert_eq!(prepared.service_draw_position(), 1);
    let created = prepared
        .create(&mut flow)
        .unwrap_or_else(|_| panic!("actual restartable Flow work creation failed"));
    let bound = created
        .bind(&flow)
        .unwrap_or_else(|_| panic!("actual Flow work binding failed"));
    let submitted = bound
        .submit(&mut flow)
        .unwrap_or_else(|_| panic!("actual timed Flow acquire submission failed"));
    Scenario {
        flow,
        adapter,
        provider: work_provider,
        seeds,
        submitted,
        service_key,
    }
}

fn codecs() -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context::<Context>("paired.context", 1, encode_context, decode_context)
        .unwrap();
    codecs
        .register_work_continuations::<Context>(
            WORK_REGISTRATION,
            FlowContinuationCodeIds::default(),
            FlowContinuations::<Context>::default(),
        )
        .unwrap();
    codecs
        .register_restart_template::<Template, Context>(
            "paired.template",
            1,
            encode_template,
            decode_template,
        )
        .unwrap();
    codecs
        .register_restart_factory::<Template, Context>(
            "paired.template",
            "paired.factory",
            make_context,
        )
        .unwrap();
    codecs
}

fn fixture_binding() -> CheckpointBindingV1 {
    let model_code = Sha256::digest(format!(
        "adr0009-first-leaf\0kairos={KAIROS_PIN}\0rustc=1.99.0\0fixture=paired_flow_c20_v2\0test=macro_and_explicit_zero_micro_pair_actual_provider_work_without_transit_events_or_draws"
    ));
    let configuration = Sha256::digest(
        "CalibrationSeedMap::new(1,\"study-α\",1234)\0crn-v1\0replication=7\0case-0001\0triage:1\0SeedPurpose::Service\0provider=1\0triage=[(10,3),(20,2),(30,5)]",
    );
    let owner_schemas = Sha256::digest(
        "KC2ENV01=1\0KCSBODY1=1\0tags=flow,fidelity,provider,seed_registry,submitted",
    );
    CheckpointBindingV1 {
        model_code: model_code.into(),
        configuration: configuration.into(),
        owner_schemas: owner_schemas.into(),
    }
}

fn resolver(
    key: CalibrationStreamKey,
) -> impl FnMut(
    EntityId,
    &SeedIdentity,
    &FlowRuntime,
) -> Result<TrustedC2WorkBindingV1, C2PortableCheckpointError> {
    move |_work, identity, _flow| {
        if !key.matches_identity(identity) {
            return Err(C2PortableCheckpointError::InvalidState);
        }
        Ok(TrustedC2WorkBindingV1 {
            service_key: key.clone(),
            graph: None,
        })
    }
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u128(out: &mut Vec<u8>, value: u128) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn transition_code(transition: LifecycleTransition) -> u8 {
    match transition {
        LifecycleTransition::Queued => 1,
        LifecycleTransition::Granted => 2,
        LifecycleTransition::Released => 3,
        LifecycleTransition::Cancelled => 4,
        LifecycleTransition::TimedOut => 5,
        LifecycleTransition::Preempted => 6,
        LifecycleTransition::Resumed => 7,
        LifecycleTransition::Restarted => 8,
        LifecycleTransition::Completed => 9,
        LifecycleTransition::Aborted => 10,
    }
}

fn dispatch_bytes(run: &FlowRun) -> Vec<u8> {
    let mut out = Vec::new();
    push_u64(&mut out, run.dispatches.len() as u64);
    for dispatch in &run.dispatches {
        push_u64(&mut out, dispatch.event.index);
        push_u32(&mut out, dispatch.event.generation);
        push_u128(&mut out, dispatch.at.ticks());
        push_u64(&mut out, dispatch.records.len() as u64);
        for record in &dispatch.records {
            out.push(transition_code(record.transition));
            push_u64(&mut out, record.causal_event_id.index);
            push_u32(&mut out, record.causal_event_id.generation);
            push_u64(&mut out, record.request.entity_id().index);
            push_u32(&mut out, record.request.entity_id().generation);
        }
    }
    out
}

fn event_ids(run: &FlowRun) -> Vec<(u64, u32)> {
    run.dispatches
        .iter()
        .map(|dispatch| (dispatch.event.index, dispatch.event.generation))
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Terminal {
    draw: u64,
    dispatched: u64,
    scheduled: u64,
    cancelled: u64,
    pending: u64,
    now: u128,
}

fn terminal_bytes(terminal: &Terminal) -> Vec<u8> {
    let mut out = Vec::new();
    push_u64(&mut out, terminal.draw);
    push_u64(&mut out, terminal.dispatched);
    push_u64(&mut out, terminal.scheduled);
    push_u64(&mut out, terminal.cancelled);
    push_u64(&mut out, terminal.pending);
    push_u128(&mut out, terminal.now);
    out
}

fn canonical_hash(dispatch: &[u8], terminal: &Terminal) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(dispatch);
    hasher.update(terminal_bytes(terminal));
    hasher.finalize().into()
}

fn hex(bytes: &[u8]) -> String {
    const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX_DIGITS[(byte >> 4) as usize]));
        output.push(char::from(HEX_DIGITS[(byte & 0x0f) as usize]));
    }
    output
}

fn decode_hex(value: &str) -> Vec<u8> {
    assert!(value.len().is_multiple_of(2), "odd hex length");
    value
        .as_bytes()
        .chunks(2)
        .map(|pair| {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => panic!("invalid hex digit"),
            };
            (digit(pair[0]) << 4) | digit(pair[1])
        })
        .collect()
}

fn run_to_completion(mode: FidelityMode) -> (Scenario, FlowRun) {
    let mut prepared = scenario(mode);
    let run = prepared.flow.run_for(128).unwrap();
    (prepared, run)
}

fn terminal_of(prepared: &Scenario) -> Terminal {
    let stats = prepared.flow.budget_snapshot().scheduler;
    Terminal {
        draw: prepared.submitted.service_draw_position(),
        dispatched: stats.dispatched_events,
        scheduled: stats.scheduled_events,
        cancelled: stats.cancelled_events,
        pending: stats.pending_events,
        now: stats.now.ticks(),
    }
}

fn capture_completed(prepared: &Scenario) -> C2PortableCheckpointV1 {
    let mut resolve = resolver(prepared.service_key.clone());
    C2PortableCheckpointV1::capture_adr0009_first_leaf(
        &prepared.flow,
        &prepared.adapter,
        &prepared.provider,
        &prepared.seeds,
        &[],
        std::slice::from_ref(&prepared.submitted),
        &codecs(),
        &mut resolve,
        fixture_binding(),
        limits(),
    )
    .unwrap()
}

fn live_fingerprint(
    prepared: &Scenario,
    run: &FlowRun,
) -> (Vec<(u64, u32)>, Terminal, RequestState) {
    let state = prepared
        .flow
        .request(prepared.submitted.request())
        .unwrap()
        .state;
    (event_ids(run), terminal_of(prepared), state)
}

#[test]
fn measured_event_k_matches_both_paired_modes() {
    let (macro_case, _) = run_to_completion(FidelityMode::Macro);
    let (micro_case, _) = run_to_completion(FidelityMode::Micro);
    let macro_k = macro_case
        .flow
        .budget_snapshot()
        .scheduler
        .dispatched_events;
    let micro_k = micro_case
        .flow
        .budget_snapshot()
        .scheduler
        .dispatched_events;
    assert_eq!(
        macro_k, micro_k,
        "Macro and explicit-zero Micro dispatched_events diverged after run_for(128)"
    );
    assert_eq!(
        macro_k, MEASURED_EVENT_K,
        "measured dispatched_events after run_for(128) is {macro_k}; record that value as MEASURED_EVENT_K"
    );
    assert_eq!(macro_case.submitted.service_draw_position(), 1);
    assert_eq!(micro_case.submitted.service_draw_position(), 1);
}

fn artifact_file() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("kairos-adr0009-{}-{nanos}.c2", std::process::id()))
}

fn child_output(file: &Path, mode: &str) -> std::process::Output {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "c2_portable_checkpoint::tests::adr0009_paired_restore::restore_child_entrypoint",
            "--nocapture",
        ])
        .env(CHILD_FILE, file)
        .env(CHILD_MODE, mode)
        .output()
        .unwrap()
}

fn parse_child_line(stdout: &str) -> (Vec<u8>, Terminal) {
    let line = stdout
        .lines()
        .find_map(|line| line.strip_prefix(RESULT_MARKER))
        .expect("child oracle marker");
    let mut dispatch_hex = None;
    let mut draw = None;
    let mut dispatched = None;
    let mut scheduled = None;
    let mut cancelled = None;
    let mut pending = None;
    let mut now = None;
    for field in line.split(' ') {
        let Some((key, value)) = field.split_once('=') else {
            continue;
        };
        match key {
            "dispatch" => dispatch_hex = Some(decode_hex(value)),
            "draw" => draw = Some(value.parse().unwrap()),
            "dispatched" => dispatched = Some(value.parse().unwrap()),
            "scheduled" => scheduled = Some(value.parse().unwrap()),
            "cancelled" => cancelled = Some(value.parse().unwrap()),
            "pending" => pending = Some(value.parse().unwrap()),
            "now" => now = Some(value.parse().unwrap()),
            _ => {}
        }
    }
    (
        dispatch_hex.expect("continuation dispatch bytes"),
        Terminal {
            draw: draw.expect("draw"),
            dispatched: dispatched.expect("dispatched"),
            scheduled: scheduled.expect("scheduled"),
            cancelled: cancelled.expect("cancelled"),
            pending: pending.expect("pending"),
            now: now.expect("now"),
        },
    )
}

fn assert_fresh_process_matches(mode: FidelityMode, mode_name: &str) {
    let (uninterrupted, full_run) = run_to_completion(mode);
    let full_terminal = terminal_of(&uninterrupted);
    assert_eq!(full_terminal.dispatched, MEASURED_EVENT_K);
    assert_eq!(full_terminal.draw, 1);
    let full_ids = event_ids(&full_run);
    let full_dispatch = dispatch_bytes(&full_run);
    let full_hash = canonical_hash(&full_dispatch, &full_terminal);

    let (cut, cut_run) = run_to_completion(mode);
    assert_eq!(event_ids(&cut_run), full_ids);
    let image = capture_completed(&cut);
    let file = artifact_file();
    image
        .save_no_clobber(&file, fixture_binding(), limits())
        .unwrap();
    let before = live_fingerprint(&cut, &cut_run);

    let output = child_output(&file, mode_name);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        output.status.success(),
        "fresh-process restore failed for {mode_name}: {stderr}\n{stdout}"
    );
    let (child_dispatch, child_terminal) = parse_child_line(&stdout);
    let empty_continuation = dispatch_bytes(&FlowRun {
        dispatches: Vec::new(),
        budget_exhausted: false,
    });
    assert_eq!(
        child_dispatch, empty_continuation,
        "event k is the completed paired run, so the fresh process continues with no further dispatches"
    );
    assert_eq!(child_terminal.draw, full_terminal.draw);
    assert_eq!(child_terminal.dispatched, full_terminal.dispatched);
    assert_eq!(child_terminal.scheduled, full_terminal.scheduled);
    assert_eq!(child_terminal.cancelled, full_terminal.cancelled);
    assert_eq!(child_terminal.pending, full_terminal.pending);
    assert_eq!(child_terminal.now, full_terminal.now);
    assert_eq!(canonical_hash(&full_dispatch, &child_terminal), full_hash);
    assert_eq!(full_ids.len() as u64, MEASURED_EVENT_K);
    assert_eq!(live_fingerprint(&cut, &cut_run), before);
    let _ = std::fs::remove_file(&file);
}

#[test]
fn fresh_process_restore_matches_uninterrupted_paired_runs() {
    assert_fresh_process_matches(FidelityMode::Macro, "macro");
    assert_fresh_process_matches(FidelityMode::Micro, "micro");
}

#[test]
fn restore_child_entrypoint() {
    let (Ok(path), Ok(mode_name)) = (std::env::var(CHILD_FILE), std::env::var(CHILD_MODE)) else {
        return;
    };
    let mode = match mode_name.as_str() {
        "macro" => FidelityMode::Macro,
        "micro" => FidelityMode::Micro,
        other => panic!("unknown paired mode {other}"),
    };
    let binding = fixture_binding();
    let image = C2PortableCheckpointV1::read_file(Path::new(&path), binding, limits()).unwrap();
    let scratch = scenario(mode);
    let mut resolve = resolver(scratch.service_key.clone());
    let restored = image
        .restore::<Template, Context>(&codecs(), &mut resolve, binding, limits())
        .unwrap();
    let draw = match restored.records.values().next() {
        Some(RestoredBridgeRecord::Submitted(record)) => record.service_draw_position(),
        Some(RestoredBridgeRecord::Bound(record)) => record.service_draw_position(),
        None => panic!("restored paired fixture has no bridge record"),
    };
    let mut flow = restored.flow;
    let continued = flow.run_for(128).unwrap();
    let stats = flow.budget_snapshot().scheduler;
    let dispatch = dispatch_bytes(&continued);
    println!(
        "{RESULT_MARKER}dispatch={} draw={draw} dispatched={} scheduled={} cancelled={} pending={} now={}",
        hex(&dispatch),
        stats.dispatched_events,
        stats.scheduled_events,
        stats.cancelled_events,
        stats.pending_events,
        stats.now.ticks()
    );
}

fn assert_live_unchanged(
    before: (Vec<(u64, u32)>, Terminal, RequestState),
    prepared: &Scenario,
    run: &FlowRun,
) {
    assert_eq!(live_fingerprint(prepared, run), before);
}

#[test]
fn corrupt_truncated_unknown_and_active_work_reject_before_mutation() {
    let (completed, run) = run_to_completion(FidelityMode::Macro);
    let image = capture_completed(&completed);
    let bytes = image.as_bytes().to_vec();
    let binding = fixture_binding();
    let before = live_fingerprint(&completed, &run);

    let mut corrupt = bytes.clone();
    let last = corrupt.len() - 1;
    corrupt[last] ^= 0x5a;
    assert!(matches!(
        C2PortableCheckpointV1::from_bytes(corrupt, binding, limits()),
        Err(C2PortableCheckpointError::Envelope(
            CheckpointEnvelopeError::InvalidFormat
        ))
    ));
    assert_live_unchanged(before.clone(), &completed, &run);

    assert!(matches!(
        C2PortableCheckpointV1::from_bytes(bytes[..10].to_vec(), binding, limits()),
        Err(C2PortableCheckpointError::Envelope(
            CheckpointEnvelopeError::InvalidFormat
        ))
    ));
    assert!(matches!(
        C2PortableCheckpointV1::from_bytes(bytes[..bytes.len() - 1].to_vec(), binding, limits()),
        Err(C2PortableCheckpointError::Envelope(
            CheckpointEnvelopeError::InvalidFormat
        ))
    ));
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(matches!(
        C2PortableCheckpointV1::from_bytes(trailing, binding, limits()),
        Err(C2PortableCheckpointError::Envelope(
            CheckpointEnvelopeError::InvalidFormat
        ))
    ));
    assert_live_unchanged(before.clone(), &completed, &run);

    let mut unknown_schema = bytes.clone();
    unknown_schema[8..12].copy_from_slice(&2u32.to_le_bytes());
    assert!(matches!(
        C2PortableCheckpointV1::from_bytes(unknown_schema, binding, limits()),
        Err(C2PortableCheckpointError::Envelope(
            CheckpointEnvelopeError::UnsupportedSchema(2)
        ))
    ));

    let body = checkpoint_envelope::decode(&bytes, binding, limits().envelope).unwrap();
    let mut bad_schema_body = body.clone();
    bad_schema_body[8..10].copy_from_slice(&2u16.to_le_bytes());
    let unknown_section =
        checkpoint_envelope::encode(&bad_schema_body, binding, limits().envelope).unwrap();
    let loaded = C2PortableCheckpointV1::from_bytes(unknown_section, binding, limits()).unwrap();
    let mut resolve = resolver(completed.service_key.clone());
    assert!(matches!(
        loaded.restore::<Template, Context>(&codecs(), &mut resolve, binding, limits()),
        Err(C2PortableCheckpointError::Sections(
            C2SectionDirectoryError::UnsupportedSchema(2)
        ))
    ));

    let mut bad_tag = body;
    bad_tag[14] = 9;
    let unknown_tag = checkpoint_envelope::encode(&bad_tag, binding, limits().envelope).unwrap();
    let loaded = C2PortableCheckpointV1::from_bytes(unknown_tag, binding, limits()).unwrap();
    assert!(matches!(
        loaded.restore::<Template, Context>(&codecs(), &mut resolve, binding, limits()),
        Err(C2PortableCheckpointError::Sections(
            C2SectionDirectoryError::InvalidTag
        ))
    ));
    assert_live_unchanged(before, &completed, &run);

    let mut active = scenario(FidelityMode::Macro);
    let mut steps = 0u64;
    loop {
        let _ = active.flow.run_for(1).unwrap();
        steps += 1;
        let state = active
            .flow
            .request(active.submitted.request())
            .unwrap()
            .state;
        if state == RequestState::Active {
            break;
        }
        assert!(
            steps < 32 && state != RequestState::Completed,
            "paired fixture never exposed RequestState::Active (last {state:?} after {steps})"
        );
    }
    let active_draw = active.submitted.service_draw_position();
    let active_stats = active.flow.budget_snapshot().scheduler;
    let mut resolve = resolver(active.service_key.clone());
    assert!(matches!(
        C2PortableCheckpointV1::capture_adr0009_first_leaf(
            &active.flow,
            &active.adapter,
            &active.provider,
            &active.seeds,
            &[],
            std::slice::from_ref(&active.submitted),
            &codecs(),
            &mut resolve,
            binding,
            limits(),
        ),
        Err(C2PortableCheckpointError::InvalidState)
    ));
    assert_eq!(active.submitted.service_draw_position(), active_draw);
    assert_eq!(active.flow.budget_snapshot().scheduler, active_stats);
    assert_eq!(
        active
            .flow
            .request(active.submitted.request())
            .unwrap()
            .state,
        RequestState::Active
    );
}
