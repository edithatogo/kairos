//! Test suite for real wgpu/WGSL device path, metadata recording, shader contracts,
//! and CPU-owned queue arbitration invariance.
//!
//! Verifies:
//! - Device, driver, and compiler details recording with pinned dependency constants.
//! - `batch_transit.wgsl` contract, workgroup size 256, and layout bounds clamping.
//! - `candidate_scoring.wgsl` contract, workgroup size 256, and metric reduction.
//! - Critical invariant: DES queue arbitration and simulation clock remain 100% CPU-owned.
//! - Graceful fallback and error reporting when device is unavailable without panicking.

#![forbid(unsafe_code)]

use kairo_ecs_gpu::compute::{
    AgentParticle, DesEvent, GpuBackendAvailability, GpuCompute, GpuComputeError, GpuState,
};
use kairo_ecs_gpu::numeric_batch::{
    compute_candidate_evaluations, compute_transit, AgentTransitQuery,
};
use kairo_ecs_gpu::wgpu::{
    BackendApi, CandidateScoringUniformParams, DeviceMetadata, DeviceStatus, DeviceType,
    TransitUniformParams, WgpuDeviceCompute, PINNED_NAGA_VERSION, PINNED_WGPU_VERSION,
};
use kairo_ecs_gpu::{ABM_STEP_WGSL, BATCH_TRANSIT_WGSL, CANDIDATE_SCORING_WGSL, DES_DISPATCH_WGSL};

#[test]
fn test_device_metadata_and_driver_compiler_recording() {
    let metadata = DeviceMetadata::probe_host();

    assert!(
        !metadata.adapter_name.is_empty(),
        "Adapter name must be recorded"
    );
    assert!(
        !metadata.driver_info.is_empty(),
        "Driver information must be recorded"
    );

    #[cfg(target_os = "macos")]
    {
        assert_eq!(
            metadata.backend_api,
            BackendApi::Metal,
            "macOS must report Metal backend API"
        );
        assert_eq!(
            metadata.device_type,
            DeviceType::IntegratedGpu,
            "Apple Silicon is integrated GPU"
        );
        assert_eq!(metadata.vendor_id, 0x106b, "Apple vendor ID is 0x106b");
    }

    #[cfg(target_os = "linux")]
    {
        assert_eq!(
            metadata.backend_api,
            BackendApi::Vulkan,
            "Linux must report Vulkan backend API"
        );
    }

    // Verify pinned dependency and compiler recording
    assert_eq!(PINNED_WGPU_VERSION, "30.0.1");
    assert_eq!(PINNED_NAGA_VERSION, "24.0.0");
    assert!(
        metadata.compiler_version.contains(PINNED_NAGA_VERSION),
        "Compiler version string must contain pinned Naga version"
    );
    assert!(
        metadata.compiler_version.contains(PINNED_WGPU_VERSION),
        "Compiler version string must contain pinned wgpu version"
    );

    // Verify max workgroup dimensions
    assert!(
        metadata.max_workgroup_dimensions[0] >= 256,
        "Max workgroup size x must be at least 256"
    );

    // Verify WgpuDeviceCompute initialized with probe_host
    let device = WgpuDeviceCompute::new_with_host_probe();
    assert!(device.is_device_active());
    assert!(device.device_metadata().is_some());
    assert_eq!(device.diagnostic(), None);

    let caps = device.capabilities();
    assert_eq!(caps.availability, GpuBackendAvailability::RealDevice);
    assert!(caps.supports_zero_copy_borrow);
    assert!(caps.supports_unified_memory);
    assert_eq!(caps.max_workgroup_size, 256);
}

#[test]
fn test_batch_transit_wgsl_shader_contract_and_layout() {
    // 1. Verify WGSL shader contracts
    assert!(
        BATCH_TRANSIT_WGSL.contains("@compute @workgroup_size(256)"),
        "Shader must declare compute workgroup size 256"
    );
    assert!(
        BATCH_TRANSIT_WGSL.contains("struct TransitParams"),
        "Shader must declare TransitParams uniform layout"
    );
    assert!(
        BATCH_TRANSIT_WGSL.contains("struct TransitQuery"),
        "Shader must declare TransitQuery storage layout"
    );
    assert!(
        BATCH_TRANSIT_WGSL.contains("struct TransitResult"),
        "Shader must declare TransitResult storage layout"
    );
    assert!(
        BATCH_TRANSIT_WGSL.contains("agent_count: u32"),
        "Uniform must include agent_count"
    );
    assert!(
        BATCH_TRANSIT_WGSL.contains("dt: f32"),
        "Uniform must include delta time"
    );
    assert!(
        BATCH_TRANSIT_WGSL.contains("min_x: f32") && BATCH_TRANSIT_WGSL.contains("max_x: f32"),
        "Uniform must include layout bounds"
    );
    assert!(
        BATCH_TRANSIT_WGSL.contains("sqrt("),
        "Shader must compute Euclidean distance"
    );

    // Also check other shaders are embedded
    assert!(!ABM_STEP_WGSL.is_empty());
    assert!(!DES_DISPATCH_WGSL.is_empty());

    // 2. Uniform buffer packing layout verification
    let uniform = TransitUniformParams::new(1024, 0.05, (-500.0, -500.0), (500.0, 500.0));
    let uniform_bytes = uniform.to_bytes();
    assert_eq!(
        uniform_bytes.len(),
        32,
        "Uniform buffer size must match WGSL 16-byte alignment"
    );

    // 3. Execute batch transit through WgpuDeviceCompute
    let mut device = WgpuDeviceCompute::new_with_host_probe();

    let queries = vec![
        AgentTransitQuery {
            start_x: 0.0,
            start_y: 0.0,
            dest_x: 3.0,
            dest_y: 4.0,
            speed: 5.0, // distance = 5.0, time = 1.0 s
        },
        AgentTransitQuery {
            start_x: 10.0,
            start_y: 10.0,
            dest_x: 10.0,
            dest_y: 10.0,
            speed: 2.0, // distance = 0.0, arrived
        },
        AgentTransitQuery {
            start_x: 0.0,
            start_y: 0.0,
            dest_x: 100.0,
            dest_y: 0.0,
            speed: 0.0, // speed = 0 -> time infinite
        },
        AgentTransitQuery {
            start_x: -600.0, // Outside bounds (-500), will clamp to -500
            start_y: 0.0,
            dest_x: 500.0,
            dest_y: 0.0,
            speed: 10.0, // clamped distance: 1000.0, time: 100.0
        },
    ];

    let results = device
        .execute_batch_transit(&queries, uniform)
        .expect("Batch transit execution must succeed");

    assert_eq!(results.len(), queries.len());

    // Query 0: 3-4-5 right triangle
    assert!((results[0].distance - 5.0).abs() < 1e-6);
    assert!((results[0].time_to_dest - 1.0).abs() < 1e-6);

    // Query 1: Zero distance
    assert!((results[1].distance - 0.0).abs() < 1e-6);
    assert!((results[1].time_to_dest - 0.0).abs() < 1e-6);

    // Query 2: Zero speed
    assert!((results[2].distance - 100.0).abs() < 1e-6);
    assert!(results[2].time_to_dest > 1e20);

    // Query 3: Clamped bounds distance from -500 to 500 = 1000
    assert!((results[3].distance - 1000.0).abs() < 1e-5);
    assert!((results[3].time_to_dest - 100.0).abs() < 1e-5);

    // Compare with CPU oracle for in-bounds subset
    let cpu_oracle_results =
        compute_transit(&queries[0..3]).expect("CPU oracle compute must succeed");
    for (gpu_res, cpu_res) in results[0..3].iter().zip(cpu_oracle_results.iter()) {
        assert!((gpu_res.distance - cpu_res.distance).abs() < 1e-6);
        if cpu_res.time_to_dest.is_finite() {
            assert!((gpu_res.time_to_dest - cpu_res.time_to_dest).abs() < 1e-6);
        }
    }
}

#[test]
fn test_candidate_scoring_wgsl_shader_contract_and_layout() {
    // 1. Verify WGSL shader contracts
    assert!(
        CANDIDATE_SCORING_WGSL.contains("@compute @workgroup_size(256)"),
        "Shader must declare compute workgroup size 256"
    );
    assert!(
        CANDIDATE_SCORING_WGSL.contains("struct CandidateScoringParams"),
        "Shader must declare uniform parameter layout"
    );
    assert!(
        CANDIDATE_SCORING_WGSL.contains("struct CandidateScoreResult"),
        "Shader must declare CandidateScoreResult storage layout"
    );
    assert!(
        CANDIDATE_SCORING_WGSL.contains("weight_w1: f32"),
        "Uniform must declare weight_w1"
    );
    assert!(
        CANDIDATE_SCORING_WGSL.contains("weight_ks: f32"),
        "Uniform must declare weight_ks"
    );
    assert!(
        CANDIDATE_SCORING_WGSL.contains("sum_abs_diff"),
        "Shader must accumulate L1/W1 difference"
    );

    // 2. Uniform buffer packing layout verification
    let uniform = CandidateScoringUniformParams::new(512, 16, 0.7, 0.3);
    let uniform_bytes = uniform.to_bytes();
    assert_eq!(
        uniform_bytes.len(),
        16,
        "Uniform layout must match WGSL 16-byte alignment"
    );

    // 3. Execute candidate scoring through WgpuDeviceCompute
    let mut device = WgpuDeviceCompute::new_with_host_probe();

    let reference_obs: Vec<f32> = (0..100).map(|i| (i as f32) * 1.5).collect();

    // Candidate 0: Perfect match (zero error)
    let cand_0 = reference_obs.clone();
    // Candidate 1: Slight noise (low error)
    let cand_1: Vec<f32> = reference_obs.iter().map(|&x| x + 0.1).collect();
    // Candidate 2: Moderate noise (medium error)
    let cand_2: Vec<f32> = reference_obs.iter().map(|&x| x + 2.0).collect();
    // Candidate 3: High error
    let cand_3: Vec<f32> = reference_obs.iter().map(|&x| x + 10.0).collect();

    let candidates = vec![cand_0, cand_1, cand_2, cand_3];

    let uniform_params =
        CandidateScoringUniformParams::new(reference_obs.len(), candidates.len(), 0.7, 0.3);

    let evaluated = device
        .execute_candidate_scoring(&candidates, &reference_obs, uniform_params)
        .expect("Candidate scoring must succeed");

    assert_eq!(evaluated.len(), 4);

    // Evaluated candidates must be ranked ascending (best score first)
    assert_eq!(
        evaluated[0].candidate_id, 0,
        "Perfect match candidate 0 must rank first"
    );
    assert_eq!(
        evaluated[1].candidate_id, 1,
        "Low error candidate 1 must rank second"
    );
    assert_eq!(
        evaluated[2].candidate_id, 2,
        "Medium error candidate 2 must rank third"
    );
    assert_eq!(
        evaluated[3].candidate_id, 3,
        "High error candidate 3 must rank fourth"
    );

    assert!(
        evaluated[0].score < evaluated[1].score,
        "Candidate scores must be strictly ordered"
    );
    assert!(evaluated[1].score < evaluated[2].score);
    assert!(evaluated[2].score < evaluated[3].score);

    // Candidate 0 perfect match should have score 0.0
    assert!(evaluated[0].score < 1e-6);

    // Compare with CPU reference rankings
    let cpu_evaluations = compute_candidate_evaluations(&candidates, &reference_obs)
        .expect("CPU candidate evaluations must succeed");
    for i in 0..4 {
        assert_eq!(
            evaluated[i].candidate_id, cpu_evaluations[i].candidate_id,
            "Rankings must match between device execution and CPU reference"
        );
    }
}

#[test]
fn test_cpu_queue_arbitration_invariance() {
    let mut device = WgpuDeviceCompute::new_with_host_probe();

    // 1. Invariant verification methods
    assert!(
        device.is_queue_arbitration_cpu_owned(),
        "Queue arbitration must be 100% CPU-owned"
    );
    device.assert_queue_arbitration_cpu_owned();

    // 2. Discrete state setup
    let state = GpuState {
        particles: vec![AgentParticle {
            x: 10.0,
            y: 20.0,
            vx: 1.0,
            vy: -1.0,
        }],
        entity_values: vec![100, 200, 300],
    };
    device
        .upload_state(&state)
        .expect("State upload must succeed");

    // 3. Out-of-order DES events with timestamps
    let events = vec![
        DesEvent {
            timestamp_ns: 3000,
            entity_id: 1,
            delta: -50,
        },
        DesEvent {
            timestamp_ns: 1000,
            entity_id: 0,
            delta: 25,
        },
        DesEvent {
            timestamp_ns: 2000,
            entity_id: 1,
            delta: 10,
        },
        DesEvent {
            timestamp_ns: 1000,
            entity_id: 2,
            delta: 15,
        },
    ];

    // Execution of DES events is sequenced strictly on CPU
    let stats = device
        .run_des_step(&events)
        .expect("DES step execution must succeed");
    assert!(stats.dispatched_workgroups >= 1);

    let downloaded = device
        .download_state()
        .expect("State download must succeed");

    // Entity 0: 100 + 25 = 125
    assert_eq!(downloaded.entity_values[0], 125);
    // Entity 1: 200 + 10 - 50 = 160
    assert_eq!(downloaded.entity_values[1], 160);
    // Entity 2: 300 + 15 = 315
    assert_eq!(downloaded.entity_values[2], 315);

    // 4. Verify out-of-range entity returns typed error without panic
    let invalid_event = vec![DesEvent {
        timestamp_ns: 4000,
        entity_id: 999, // out of range
        delta: 1,
    }];
    let err = device.run_des_step(&invalid_event);
    assert_eq!(
        err,
        Err(GpuComputeError::EntityOutOfRange {
            entity_id: 999,
            entity_count: 3,
        })
    );
}

#[test]
fn test_unavailable_device_graceful_handling() {
    let diagnostic_msg = "Headless CI environment: Metal/Vulkan device not available";
    let mut device = WgpuDeviceCompute::new_unavailable(diagnostic_msg);

    assert!(
        !device.is_device_active(),
        "Unavailable device must report inactive"
    );
    assert_eq!(device.diagnostic(), Some(diagnostic_msg));
    assert_eq!(device.device_metadata(), None);
    assert_eq!(
        device.status(),
        &DeviceStatus::Unavailable {
            diagnostic: diagnostic_msg.to_string()
        }
    );

    // Capabilities must indicate CpuFallback, never masquerading as real GPU
    let caps = device.capabilities();
    assert_eq!(caps.availability, GpuBackendAvailability::CpuFallback);
    assert_eq!(caps.backend_name, "wgpu-fallback-cpu");
    assert!(!caps.supports_zero_copy_borrow);
    assert!(!caps.supports_unified_memory);

    // Transit execution must gracefully fall back to CPU without panic
    let queries = vec![AgentTransitQuery {
        start_x: 0.0,
        start_y: 0.0,
        dest_x: 6.0,
        dest_y: 8.0,
        speed: 5.0,
    }];
    let params = TransitUniformParams::new(1, 0.1, (-100.0, -100.0), (100.0, 100.0));
    let transit_results = device
        .execute_batch_transit(&queries, params)
        .expect("Fallback transit execution must succeed");
    assert_eq!(transit_results.len(), 1);
    assert!((transit_results[0].distance - 10.0).abs() < 1e-6);
    assert!((transit_results[0].time_to_dest - 2.0).abs() < 1e-6);

    // Candidate scoring execution must gracefully fall back to CPU without panic
    let ref_obs = vec![1.0, 2.0, 3.0];
    let cand_preds = vec![vec![1.1, 2.1, 3.1], vec![1.0, 2.0, 3.0]];
    let cand_params = CandidateScoringUniformParams::new(3, 2, 0.5, 0.5);
    let scored = device
        .execute_candidate_scoring(&cand_preds, &ref_obs, cand_params)
        .expect("Fallback candidate scoring must succeed");
    assert_eq!(scored.len(), 2);
    // Candidate 1 (exact match) ranks first
    assert_eq!(scored[0].candidate_id, 1);
    assert_eq!(scored[1].candidate_id, 0);

    // ABM step and DES step must execute cleanly via fallback
    let state = GpuState {
        particles: vec![AgentParticle {
            x: 0.0,
            y: 0.0,
            vx: 1.0,
            vy: 2.0,
        }],
        entity_values: vec![50],
    };
    device
        .upload_state(&state)
        .expect("Upload to fallback must succeed");

    device
        .run_abm_step(0.1, 42)
        .expect("ABM step on fallback must succeed");

    let des_events = vec![DesEvent {
        timestamp_ns: 10,
        entity_id: 0,
        delta: 25,
    }];
    device
        .run_des_step(&des_events)
        .expect("DES step on fallback must succeed");

    let downloaded = device
        .download_state()
        .expect("Download from fallback must succeed");
    assert_eq!(downloaded.entity_values[0], 75);
}
