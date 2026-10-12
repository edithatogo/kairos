//! Fresh-process proof that trusted fixture identities bind the actual C2
//! image and that restored calibration owners preserve future stream use.
//!
//! This module is included beneath `process_tests` so it can reuse that module's
//! compiled fixture builders. The child receives only a file path and a case
//! name; it rebuilds every expected binding from those trusted builders.

use super::*;
use crate::seed_map::checkpoint_wire::{encode_seed_map, SeedWireLimits};
use crate::seed_map::CalibrationSeedMap;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

const CHILD_IMAGE: &str = "KAIROS_C2_IDENTITY_RNG_FILE";
const CHILD_CASE: &str = "KAIROS_C2_IDENTITY_RNG_CASE";
const RESULT_MARKER: &str = "C2_IDENTITY_RNG_TRACE:";

fn add_frame(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(&(value.len() as u64).to_le_bytes());
    output.extend_from_slice(value);
}

fn add_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn derived_binding(case: &CaseSpec) -> CheckpointBindingV1 {
    // These descriptors identify the executable fixture implementation. They
    // are compiled caller inputs and are never read from the checkpoint.
    let mut model_code = Vec::new();
    add_frame(&mut model_code, b"c2.identity-rng.model-code.v1");
    add_frame(&mut model_code, b"builder=process_tests::build_runtime/v1");
    add_frame(
        &mut model_code,
        format!("context.codec={CONTEXT_CODEC}@1").as_bytes(),
    );
    add_frame(
        &mut model_code,
        format!("template.codec={TEMPLATE_CODEC}@1").as_bytes(),
    );
    add_frame(
        &mut model_code,
        format!("transit.codec={TRANSIT_CODEC}@1").as_bytes(),
    );
    add_frame(&mut model_code, b"restart.factory=c2.matrix.factory@1");
    add_frame(&mut model_code, b"work.handlers=default@1");
    for (graph_spec, _) in graph_catalog(case) {
        add_frame(&mut model_code, graph_spec.registration.as_bytes());
        add_frame(&mut model_code, graph_spec.tag.as_bytes());
        add_u64(&mut model_code, u64::from(graph_spec.event_kind.code()));
        add_frame(
            &mut model_code,
            format!("{}.plan@1", graph_spec.tag).as_bytes(),
        );
        add_frame(
            &mut model_code,
            format!("{}.accept@1", graph_spec.tag).as_bytes(),
        );
    }

    // Bind the current compiled case, canonical approved graphs, provider
    // strata, and complete service-key registry. None of these values comes
    // from the envelope under test.
    let mut configuration = Vec::new();
    add_frame(&mut configuration, b"c2.identity-rng.configuration.v1");
    add_frame(&mut configuration, case.name.as_bytes());
    configuration.push(u8::from(case.pending_policy));
    add_u64(&mut configuration, ROOT_SEED);
    add_frame(&mut configuration, STUDY_ID.as_bytes());
    for (index, work) in case.works.iter().enumerate() {
        add_u64(&mut configuration, index as u64);
        configuration.push(match work.mode {
            FidelityMode::Macro => 1,
            FidelityMode::Micro => 2,
        });
        add_frame(&mut configuration, work.task.as_bytes());
        add_frame(&mut configuration, work.stratum.as_bytes());
        if let Some(graph) = &work.graph {
            add_frame(&mut configuration, graph.tag.as_bytes());
        } else {
            add_frame(&mut configuration, b"no-transit-graph");
        }
    }
    for (graph_spec, trusted_graph) in graph_catalog(case) {
        add_frame(&mut configuration, graph_spec.tag.as_bytes());
        add_frame(
            &mut configuration,
            trusted_graph.canonical_bytes().as_slice(),
        );
    }
    let mut registry = CalibrationSeedMap::new(1, STUDY_ID, ROOT_SEED).unwrap();
    for work in &case.works {
        expected_service_key(&mut registry, case.name, &work.task);
        add_all_purpose_entries(&mut registry, case.name, &work.task);
    }
    let seed_limits = limits().seed;
    let registry_bytes = encode_seed_map(
        &registry,
        SeedWireLimits {
            max_entries: seed_limits.max_entries,
            max_identifier_bytes: seed_limits.max_identifier_bytes,
            max_wire_bytes: seed_limits.max_wire_bytes,
        },
    )
    .unwrap();
    add_frame(&mut configuration, &registry_bytes);
    let provider_bytes = provider_for_matrix()
        .checkpoint_wire_v1(limits().provider)
        .unwrap();
    add_frame(&mut configuration, &provider_bytes);

    let mut owner_schemas = Vec::new();
    add_frame(
        &mut owner_schemas,
        b"c2.identity-rng.owner-schema-inventory.v1",
    );
    for schema in [
        b"envelope=1".as_slice(),
        b"sections=1",
        b"flow=1",
        b"fidelity=1",
        b"provider=1",
        b"seed-map=1",
        b"seed-stream=1",
        b"bound-bridge=1",
        b"submitted-bridge=1",
        b"transit-context=1",
        b"route-receipt=1",
    ] {
        add_frame(&mut owner_schemas, schema);
    }

    CheckpointBindingV1 {
        model_code: Sha256::digest(model_code).into(),
        configuration: Sha256::digest(configuration).into(),
        owner_schemas: Sha256::digest(owner_schemas).into(),
    }
}

fn capture_with_binding(runtime: &MatrixRuntime, binding: CheckpointBindingV1) -> Vec<u8> {
    let mut trusted = resolver(&runtime.bindings);
    C2PortableCheckpointV1::capture(
        &runtime.flow,
        &runtime.adapter,
        &runtime.provider,
        &runtime.seeds,
        &runtime.bounds,
        &runtime.submitted,
        &runtime.codecs,
        &mut trusted,
        binding,
        limits(),
    )
    .unwrap()
    .as_bytes()
    .to_vec()
}

#[derive(Debug, Eq, PartialEq)]
struct Continuation {
    first_draw: u64,
    sampled_ticks: u128,
    sample_draw_before: u64,
    sample_draw_after: u64,
    next_u32: u32,
    next_u64: u64,
    final_position: u64,
}

fn continuation(
    provider: &IntrinsicWorkProvider,
    bound: &mut BoundIntrinsicWork<Vec<u8>, OwnedContext>,
    expected_key: &CalibrationStreamKey,
) -> Continuation {
    let stream = bound.service_stream_for_checkpoint_test();
    assert_eq!(stream.key(), *expected_key);
    let first_draw = stream.next_u64().unwrap();
    let sample = provider.sample("base", stream, expected_key).unwrap();
    let sample_draw_before = sample.draw_before();
    let sample_draw_after = sample.draw_after();
    let sampled_ticks = sample.duration().ticks();
    let next_u32 = stream.next_u32().unwrap();
    let next_u64 = stream.next_u64().unwrap();
    Continuation {
        first_draw,
        sampled_ticks,
        sample_draw_before,
        sample_draw_after,
        next_u32,
        next_u64,
        final_position: stream.draw_position(),
    }
}

fn artifact_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(".artifacts")
        .join("c2-identity-rng")
}

fn unique_file() -> PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    artifact_dir().join(format!("identity-rng-{}-{now}.c2", std::process::id()))
}

fn child_output(file: &Path) -> std::process::Output {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "c2_portable_checkpoint::tests::process_tests::identity_rng_tests::restore_child_entrypoint",
            "--nocapture",
        ])
        .env(CHILD_IMAGE, file)
        .env(CHILD_CASE, "transit-ready")
        .output()
        .unwrap()
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[(byte >> 4) as usize]));
        output.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    output
}

#[test]
fn derived_trusted_identity_and_rng_continuation_survive_fresh_process_restore() {
    let mut case = spec("transit-ready").unwrap();
    // A fixed service sample leaves the stream unchanged. Advance that real
    // bridge-owned stream before capture so restore must continue from a
    // nonzero cursor, with no prefix replay in the checkpoint path.
    case.works[0].stratum = "service20";
    let binding = derived_binding(&case);
    assert_ne!(binding.model_code, [0x11; 32]);
    assert_ne!(binding.configuration, [0x22; 32]);
    assert_ne!(binding.owner_schemas, [0x33; 32]);
    assert_ne!(binding.model_code, binding.configuration);
    assert_ne!(binding.model_code, binding.owner_schemas);
    assert_ne!(binding.configuration, binding.owner_schemas);

    let mut source = build_runtime(&case);
    {
        let stream = source.bounds[0].service_stream_for_checkpoint_test();
        stream.next_u64().unwrap();
        stream.next_u32().unwrap();
        assert_eq!(stream.draw_position(), 2);
    }
    let image_bytes = capture_with_binding(&source, binding);
    let mut wrong_model = binding;
    wrong_model.model_code[0] ^= 0x01;
    let mut wrong_config = binding;
    wrong_config.configuration[0] ^= 0x01;
    let mut wrong_schemas = binding;
    wrong_schemas.owner_schemas[0] ^= 0x01;
    for wrong in [wrong_model, wrong_config, wrong_schemas] {
        assert!(C2PortableCheckpointV1::from_bytes(image_bytes.clone(), wrong, limits()).is_err());
    }

    // The provider-sample stream key corresponds to the compiled case identity.
    let key = source.bindings[&work_id_for(0)].service_key.clone();
    std::fs::create_dir_all(artifact_dir()).unwrap();
    let file = unique_file();
    C2PortableCheckpointV1::from_bytes(image_bytes, binding, limits())
        .unwrap()
        .save_no_clobber(&file, binding, limits())
        .unwrap();
    let expected = continuation(&source.provider, &mut source.bounds[0], &key);
    let expected_after = capture_with_binding(&source, binding);

    let output = child_output(&file);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    std::fs::write(file.with_extension("child.stdout"), &stdout).unwrap();
    std::fs::write(file.with_extension("child.stderr"), &stderr).unwrap();
    std::fs::write(
        file.with_extension("invocation.txt"),
        format!(
            "child_test=c2_portable_checkpoint::tests::process_tests::identity_rng_tests::restore_child_entrypoint\nchild_exit={:?}\nimage_sha256={}\nexpected_continuation={expected:?}\nexpected_after_sha256={}\nmodel_code={}\nconfiguration={}\nowner_schemas={}\n",
            output.status.code(),
            hex(&Sha256::digest(std::fs::read(&file).unwrap())),
            hex(&Sha256::digest(&expected_after)),
            hex(&binding.model_code),
            hex(&binding.configuration),
            hex(&binding.owner_schemas),
        ),
    )
    .unwrap();
    assert!(
        output.status.success(),
        "child status {:?}; stderr: {stderr}",
        output.status.code()
    );
    let actual = stdout
        .lines()
        .find_map(|line| line.strip_prefix(RESULT_MARKER))
        .expect("child continuation marker");
    let expected_trace = format!("{:?}|{}", expected, hex(&Sha256::digest(&expected_after)));
    assert_eq!(actual, expected_trace);
}

#[test]
fn restore_child_entrypoint() {
    let (Ok(path), Ok(case_name)) = (std::env::var(CHILD_IMAGE), std::env::var(CHILD_CASE)) else {
        return;
    };
    let mut case = spec(&case_name).unwrap();
    case.works[0].stratum = "service20";
    let binding = derived_binding(&case);
    let scratch = FlowRuntime::new();
    let codecs = matrix_codecs(scratch.identity(), &case);
    let bindings = trusted_bindings(&case);
    let image = C2PortableCheckpointV1::read_file(Path::new(&path), binding, limits()).unwrap();
    let restored = {
        let mut trusted = resolver(&bindings);
        image
            .restore::<Vec<u8>, OwnedContext>(&codecs, &mut trusted, binding, limits())
            .unwrap()
    };
    let mut runtime = MatrixRuntime {
        codecs: matrix_codecs(restored.flow.identity(), &case),
        flow: restored.flow,
        adapter: restored.adapter,
        provider: restored.provider,
        seeds: restored.seed_registry,
        bounds: Vec::new(),
        submitted: Vec::new(),
        bindings,
    };
    for (_, record) in restored.records {
        match record {
            RestoredBridgeRecord::Bound(bound) => runtime.bounds.push(*bound),
            RestoredBridgeRecord::Submitted(submitted) => runtime.submitted.push(*submitted),
        }
    }
    let bound_index = runtime
        .bounds
        .iter()
        .position(|bound| bound.work().entity_id() == work_id_for(0))
        .expect("compiled case restores its Bound owner");
    let key = runtime.bindings[&work_id_for(0)].service_key.clone();
    let actual = continuation(&runtime.provider, &mut runtime.bounds[bound_index], &key);
    let after = capture_with_binding(&runtime, binding);
    println!("{RESULT_MARKER}{actual:?}|{}", hex(&Sha256::digest(&after)));
}
