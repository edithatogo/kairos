//! Fresh-process caller-owned checkpoint fixtures with several concrete codec types.

use super::*;
use kairo_ecs_des::{
    FlowCallbackCodeV1, FlowCheckpointCodecError, FlowCheckpointCodecs, FlowCommandSink,
    FlowHandlerCodeIds, FlowRuntime, FlowWorldView, WorkHandlers,
};
use kairo_ecs_types::{EventKind, SimDuration, SimTime};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const CHILD_FILE: &str = "KAIROS_C2_GENERIC_CONTEXT_FILE";
const TRACE_MARKER: &str = "C2_GENERIC_CONTEXT_TRACE:";
const MAIN_REGISTRATION: &str = "c2.generic.main";
const AUX_REGISTRATION: &str = "c2.generic.aux";
const MAIN_TEMPLATE: &str = "c2.generic.main-template";
const AUX_TEMPLATE: &str = "c2.generic.aux-template";
const MUTATION_KIND: EventKind = EventKind::custom(0xC2F1);

struct OwnedContext {
    bytes: Box<[u8]>,
    revision: u32,
}

struct AuxiliaryContext {
    bytes: Box<[u8]>,
    factory: u8,
}

struct AuxiliaryTemplate(Vec<u8>);

#[allow(clippy::ptr_arg)] // Restart factory callbacks retain the registered template type.
fn context_from_template(template: &Vec<u8>) -> OwnedContext {
    OwnedContext {
        bytes: template.clone().into_boxed_slice(),
        revision: 0,
    }
}

fn auxiliary_from_first(template: &AuxiliaryTemplate) -> AuxiliaryContext {
    AuxiliaryContext {
        bytes: template.0.clone().into_boxed_slice(),
        factory: 1,
    }
}

fn auxiliary_from_second(template: &AuxiliaryTemplate) -> AuxiliaryContext {
    let mut bytes = template.0.clone();
    bytes.reverse();
    AuxiliaryContext {
        bytes: bytes.into_boxed_slice(),
        factory: 2,
    }
}

fn mutate_owned_context(
    context: &mut OwnedContext,
    _: &kairo_ecs_des::FlowCallbackSnapshot,
    _: FlowWorldView<'_>,
    _: &mut FlowCommandSink,
) {
    context.bytes = b"mutated-after-create".to_vec().into_boxed_slice();
    context.revision = 73;
}

fn encode_bytes(bytes: &[u8], cap: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    if bytes.len() > cap {
        return Err(FlowCheckpointCodecError(
            "fixture exceeds byte cap".to_owned(),
        ));
    }
    Ok(bytes.to_vec())
}

fn decode_bytes(bytes: &[u8]) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    Ok(bytes.to_vec())
}

fn encode_owned(context: &OwnedContext, cap: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    let needed = context
        .bytes
        .len()
        .checked_add(4)
        .ok_or_else(|| FlowCheckpointCodecError("fixture length overflow".to_owned()))?;
    if needed > cap {
        return Err(FlowCheckpointCodecError(
            "fixture exceeds byte cap".to_owned(),
        ));
    }
    let mut bytes = Vec::with_capacity(needed);
    bytes.extend_from_slice(&context.revision.to_le_bytes());
    bytes.extend_from_slice(&context.bytes);
    Ok(bytes)
}

fn decode_owned(bytes: &[u8]) -> Result<OwnedContext, FlowCheckpointCodecError> {
    let (revision, payload) = bytes
        .split_at_checked(4)
        .ok_or_else(|| FlowCheckpointCodecError("truncated owned context".to_owned()))?;
    Ok(OwnedContext {
        bytes: payload.to_vec().into_boxed_slice(),
        revision: u32::from_le_bytes(revision.try_into().unwrap()),
    })
}

fn encode_auxiliary(
    context: &AuxiliaryContext,
    cap: usize,
) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    let needed = context
        .bytes
        .len()
        .checked_add(1)
        .ok_or_else(|| FlowCheckpointCodecError("fixture length overflow".to_owned()))?;
    if needed > cap {
        return Err(FlowCheckpointCodecError(
            "fixture exceeds byte cap".to_owned(),
        ));
    }
    let mut bytes = Vec::with_capacity(needed);
    bytes.push(context.factory);
    bytes.extend_from_slice(&context.bytes);
    Ok(bytes)
}

fn decode_auxiliary(bytes: &[u8]) -> Result<AuxiliaryContext, FlowCheckpointCodecError> {
    let (factory, payload) = bytes
        .split_first()
        .ok_or_else(|| FlowCheckpointCodecError("truncated auxiliary context".to_owned()))?;
    Ok(AuxiliaryContext {
        bytes: payload.to_vec().into_boxed_slice(),
        factory: *factory,
    })
}

fn encode_template(
    value: &AuxiliaryTemplate,
    cap: usize,
) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    encode_bytes(&value.0, cap)
}

fn decode_template(bytes: &[u8]) -> Result<AuxiliaryTemplate, FlowCheckpointCodecError> {
    Ok(AuxiliaryTemplate(decode_bytes(bytes)?))
}

fn codecs(decoded: Arc<AtomicUsize>) -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    let decoded_main = Arc::clone(&decoded);
    codecs
        .register_context_with_owner::<OwnedContext>(
            MAIN_REGISTRATION,
            1,
            encode_owned,
            move |bytes, _, _| {
                decoded_main.fetch_add(1, Ordering::SeqCst);
                decode_owned(bytes)
            },
        )
        .unwrap();
    let decoded_aux = Arc::clone(&decoded);
    codecs
        .register_context_with_owner::<AuxiliaryContext>(
            AUX_REGISTRATION,
            1,
            encode_auxiliary,
            move |bytes, _, _| {
                decoded_aux.fetch_add(1, Ordering::SeqCst);
                decode_auxiliary(bytes)
            },
        )
        .unwrap();
    codecs
        .register_work_handlers::<OwnedContext>(
            MAIN_REGISTRATION,
            FlowHandlerCodeIds::default(),
            WorkHandlers::default(),
        )
        .unwrap();
    codecs
        .register_work_handlers::<AuxiliaryContext>(
            AUX_REGISTRATION,
            FlowHandlerCodeIds::default(),
            WorkHandlers::default(),
        )
        .unwrap();
    codecs
        .register_domain_view_hook(
            MAIN_REGISTRATION,
            MUTATION_KIND,
            mutate_owned_context,
            FlowCallbackCodeV1 {
                stable_id: "c2.generic.mutate-owned-context".to_owned(),
                version: 1,
            },
        )
        .unwrap();
    codecs
        .register_restart_template_with_owner::<Vec<u8>, OwnedContext>(
            MAIN_TEMPLATE,
            1,
            |template, cap| encode_bytes(template, cap),
            |bytes, _, _| decode_bytes(bytes),
        )
        .unwrap();
    codecs
        .register_restart_factory::<Vec<u8>, OwnedContext>(
            MAIN_TEMPLATE,
            "c2.generic.main-factory",
            context_from_template,
        )
        .unwrap();
    codecs
        .register_restart_template_with_owner::<AuxiliaryTemplate, AuxiliaryContext>(
            AUX_TEMPLATE,
            1,
            encode_template,
            |bytes, _, _| decode_template(bytes),
        )
        .unwrap();
    codecs
        .register_restart_factory::<AuxiliaryTemplate, AuxiliaryContext>(
            AUX_TEMPLATE,
            "c2.generic.aux-first-factory",
            auxiliary_from_first,
        )
        .unwrap();
    codecs
        .register_restart_factory::<AuxiliaryTemplate, AuxiliaryContext>(
            AUX_TEMPLATE,
            "c2.generic.aux-second-factory",
            auxiliary_from_second,
        )
        .unwrap();
    codecs
}

fn configure_runtime(flow: &mut FlowRuntime) {
    flow.register_work_handlers(MAIN_REGISTRATION, WorkHandlers::<OwnedContext>::default())
        .unwrap();
    flow.register_work_handlers(
        AUX_REGISTRATION,
        WorkHandlers::<AuxiliaryContext>::default(),
    )
    .unwrap();
    flow.register_domain_view_hook(MAIN_REGISTRATION, MUTATION_KIND, mutate_owned_context)
        .unwrap();
}

fn expected_key() -> (CalibrationSeedMap, CalibrationStreamKey) {
    let mut registry = CalibrationSeedMap::new(1, "generic-context-tests", 2026).unwrap();
    let key = registry
        .key_for(
            "paired",
            0,
            "mutated-context",
            "main-work",
            SeedPurpose::Service,
        )
        .unwrap();
    (registry, key)
}

fn resolver(
    key: CalibrationStreamKey,
) -> impl FnMut(
    EntityId,
    &SeedIdentity,
    &FlowRuntime,
) -> Result<TrustedC2WorkBindingV1, C2PortableCheckpointError> {
    move |_, identity, _| {
        if !key.matches_identity(identity) {
            return Err(C2PortableCheckpointError::InvalidState);
        }
        Ok(TrustedC2WorkBindingV1 {
            service_key: key.clone(),
            graph: None,
        })
    }
}

struct Fixture {
    flow: FlowRuntime,
    adapter: FidelityAdapter,
    provider: IntrinsicWorkProvider,
    seeds: CalibrationSeedMap,
    key: CalibrationStreamKey,
    bound: BoundIntrinsicWork<Vec<u8>, OwnedContext>,
}

fn fixture() -> Fixture {
    let mut flow = FlowRuntime::new();
    configure_runtime(&mut flow);
    let owner = flow.spawn_actor().unwrap();
    let (mut seeds, key) = expected_key();
    let stream = seeds
        .stream_for(
            "paired",
            0,
            "mutated-context",
            "main-work",
            SeedPurpose::Service,
        )
        .unwrap();
    let mut adapter =
        FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap());
    let provider = provider();
    let preparation = WorkPreparationInput::new(
        PreparationIdentity {
            owner,
            subsystem: "macro".to_owned(),
            registration: MAIN_REGISTRATION.to_owned(),
            stratum: "base".to_owned(),
        },
        stream,
        key.clone(),
        b"original-template".to_vec(),
        context_from_template,
        AcquireIntent {
            resource: flow.create_resource(1).unwrap(),
            owner,
            at: SimTime::from_ticks(0),
            priority_level: 1,
            deadline: None,
            scheduler_priority: 0,
            can_preempt: false,
            preemptible: None,
        },
        TransitRequest::Zero,
    );
    let created = preparation
        .prepare(&flow, &mut adapter, &provider)
        .ok()
        .unwrap()
        .create(&mut flow)
        .ok()
        .unwrap();
    let bound = created.bind(&flow).ok().unwrap();
    flow.schedule_domain(bound.work(), MUTATION_KIND, SimTime::from_ticks(1), 0)
        .unwrap();
    flow.step().unwrap().unwrap();
    let mutated = flow.work_context::<OwnedContext>(bound.work()).unwrap();
    assert_eq!(&*mutated.bytes, b"mutated-after-create");
    assert_eq!(mutated.revision, 73);

    // Keep two other concrete context/template registrations in the same Flow
    // image. Distinct factory functions exercise the exact factory-key mapping.
    let auxiliary_owner = flow.spawn_actor().unwrap();
    flow.create_restartable_work(
        auxiliary_owner,
        SimDuration::from_ticks(5),
        AUX_REGISTRATION,
        AuxiliaryTemplate(b"aux-one".to_vec()),
        auxiliary_from_first,
    )
    .unwrap();
    flow.create_restartable_work(
        auxiliary_owner,
        SimDuration::from_ticks(7),
        AUX_REGISTRATION,
        AuxiliaryTemplate(b"aux-two".to_vec()),
        auxiliary_from_second,
    )
    .unwrap();
    Fixture {
        flow,
        adapter,
        provider,
        seeds,
        key,
        bound,
    }
}

fn capture(fixture: &Fixture, codecs: &FlowCheckpointCodecs) -> C2PortableCheckpointV1 {
    let mut trusted = resolver(fixture.key.clone());
    C2PortableCheckpointV1::capture(
        &fixture.flow,
        &fixture.adapter,
        &fixture.provider,
        &fixture.seeds,
        std::slice::from_ref(&fixture.bound),
        &[],
        codecs,
        &mut trusted,
        binding(),
        limits(),
    )
    .unwrap()
}

#[test]
fn actual_composite_rebinds_mutated_nonclone_context_and_multiple_codec_types_in_child() {
    let fixture = fixture();
    let decode_calls = Arc::new(AtomicUsize::new(0));
    let source_codecs = codecs(Arc::clone(&decode_calls));
    let image = capture(&fixture, &source_codecs);
    let bytes = image.as_bytes().to_vec();
    let original_identity = fixture.flow.identity();

    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(".artifacts")
        .join("c2-generic-context-tests");
    std::fs::create_dir_all(&directory).unwrap();
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = directory.join(format!("generic-context-{}-{nonce}.c2", std::process::id()));
    image.save_no_clobber(&path, binding(), limits()).unwrap();

    // Envelope identity/cap rejection is before any caller decoder.
    let mut wrong = binding();
    wrong.model_code[0] ^= 1;
    assert!(C2PortableCheckpointV1::from_bytes(bytes.clone(), wrong, limits()).is_err());
    let mut capped = limits();
    capped.envelope.max_file_bytes = bytes.len().saturating_sub(1);
    assert!(C2PortableCheckpointV1::from_bytes(bytes.clone(), binding(), capped).is_err());
    assert_eq!(decode_calls.load(Ordering::SeqCst), 0);

    // A resolver failure drops only the detached staged restore; source remains
    // untouched and is still valid for its original control execution.
    let mut invalid_trusted = |_work, _identity: &SeedIdentity, _flow: &FlowRuntime| {
        Err(C2PortableCheckpointError::InvalidState)
    };
    let rejected = C2PortableCheckpointV1::from_bytes(bytes.clone(), binding(), limits())
        .unwrap()
        .restore::<Vec<u8>, OwnedContext>(
            &codecs(Arc::clone(&decode_calls)),
            &mut invalid_trusted,
            binding(),
            limits(),
        );
    assert!(rejected.is_err());
    assert!(decode_calls.load(Ordering::SeqCst) >= 3);
    assert_eq!(fixture.flow.identity(), original_identity);
    let source_context = fixture
        .flow
        .work_context::<OwnedContext>(fixture.bound.work())
        .unwrap();
    assert_eq!(&*source_context.bytes, b"mutated-after-create");
    assert_eq!(source_context.revision, 73);

    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "c2_portable_checkpoint::tests::generic_context_tests::restore_child_entrypoint",
            "--nocapture",
        ])
        .env(CHILD_FILE, &path)
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    std::fs::write(path.with_extension("child.stdout"), &stdout).unwrap();
    std::fs::write(path.with_extension("child.stderr"), &stderr).unwrap();
    std::fs::write(
        path.with_extension("invocation.txt"),
        format!(
            "child_test=c2_portable_checkpoint::tests::generic_context_tests::restore_child_entrypoint\nchild_exit={:?}\nimage_sha256={}\nexpected_trace=mutated-after-create:73:exact-image\n",
            output.status.code(),
            <sha2::Sha256 as sha2::Digest>::digest(&bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        ),
    )
    .unwrap();
    assert!(output.status.success(), "child failed: {stderr}");
    let trace = stdout
        .lines()
        .find_map(|line| line.strip_prefix(TRACE_MARKER))
        .expect("child emitted restored context trace");
    assert_eq!(trace, "mutated-after-create:73:exact-image");
}

#[test]
fn restore_child_entrypoint() {
    let Ok(path) = std::env::var(CHILD_FILE) else {
        return;
    };
    let image = C2PortableCheckpointV1::read_file(std::path::Path::new(&path), binding(), limits())
        .unwrap();
    let expected_image = image.as_bytes().to_vec();
    let (_, key) = expected_key();
    let restored = {
        let mut trusted = resolver(key.clone());
        image
            .restore::<Vec<u8>, OwnedContext>(
                &codecs(Arc::new(AtomicUsize::new(0))),
                &mut trusted,
                binding(),
                limits(),
            )
            .unwrap()
    };
    assert_ne!(restored.flow.identity(), FlowRuntime::new().identity());
    assert!(contains_registered_key(&restored.seed_registry, &key).unwrap());
    let (work, bound) = restored
        .records
        .iter()
        .find_map(|(work, record)| match record {
            RestoredBridgeRecord::Bound(bound) => Some((*work, bound)),
            RestoredBridgeRecord::Submitted(_) => None,
        })
        .unwrap();
    let context = restored
        .flow
        .work_context::<OwnedContext>(bound.work())
        .unwrap();
    assert_eq!(&*context.bytes, b"mutated-after-create");
    assert_eq!(context.revision, 73);
    let RestoredC2 {
        flow,
        adapter,
        provider,
        seed_registry,
        records,
    } = restored;
    let (_, record) = records.into_iter().next().unwrap();
    let mut fixture = Fixture {
        flow,
        adapter,
        provider,
        seeds: seed_registry,
        key,
        bound: match record {
            RestoredBridgeRecord::Bound(bound) => *bound,
            RestoredBridgeRecord::Submitted(_) => panic!("fixture expected Bound record"),
        },
    };
    assert_eq!(fixture.bound.work().entity_id(), work);
    let recaptured = capture(&fixture, &codecs(Arc::new(AtomicUsize::new(0))));
    assert_eq!(recaptured.as_bytes(), expected_image);
    // Make the owned restored source remain usable after verification.
    fixture
        .flow
        .schedule_domain(
            fixture.bound.work(),
            MUTATION_KIND,
            SimTime::from_ticks(2),
            0,
        )
        .unwrap();
    let _ = fixture.flow.step().unwrap();
    println!("{TRACE_MARKER}mutated-after-create:73:exact-image");
}
