#![cfg(all(target_os = "macos", feature = "metal-backend"))]

use kairo_ecs_gpu::{
    compute::{AgentParticle, GpuBackendAvailability, GpuCompute, GpuState},
    metal::{MetalBackend, METAL_BACKEND_NAME},
};
use std::time::Instant;

#[test]
fn test_metal_device_hardware_provenance_and_availability() {
    let backend = MetalBackend::initialize_macos_hardware_only().unwrap();
    let caps = backend.capabilities();

    assert_eq!(caps.backend_name, METAL_BACKEND_NAME);
    // When run on macOS with Apple Silicon GPU, backend must be Available and not CpuFallback
    assert_eq!(
        caps.availability,
        GpuBackendAvailability::Available,
        "Native Apple Silicon Metal device must report Available"
    );
    assert!(caps.supports_unified_memory);
    assert!(!caps.supports_zero_copy_borrow);

    let info = backend.adapter_info();
    assert!(
        info.adapter_name.contains("Apple"),
        "Hardware adapter provenance must identify Apple Silicon device: got {}",
        info.adapter_name
    );
}

#[test]
fn test_metal_persistent_particles_across_ticks() {
    let mut backend = MetalBackend::initialize_macos_hardware_only().unwrap();

    let n_particles = 10_000;
    let mut particles = Vec::with_capacity(n_particles);
    for i in 0..n_particles {
        particles.push(AgentParticle {
            x: i as f32,
            y: (i * 2) as f32,
            vx: 1.0,
            vy: -0.5,
        });
    }

    let state = GpuState {
        particles,
        entity_values: vec![1, 2, 3],
    };

    // 1. Initial upload: uploads state once to persistent device memory
    let upload_stats = backend.upload_state(&state).expect("upload must succeed");
    assert_eq!(
        upload_stats.uploaded_bytes,
        n_particles * std::mem::size_of::<AgentParticle>() + 3 * std::mem::size_of::<i32>()
    );

    // 2. Run 10 consecutive ticks without re-uploading from host RAM
    let start_ticks = Instant::now();
    for tick in 0..10 {
        let step_stats = backend
            .run_abm_step(0.016, 12345 + tick)
            .expect("ABM step must dispatch to GPU");
        assert_eq!(step_stats.uploaded_bytes, 16, "tick parameters");
        assert_eq!(step_stats.downloaded_bytes, 0);
        assert!(step_stats.dispatched_workgroups > 0);
    }
    // The readback waits for the submitted work.
    let final_state = backend.download_state().expect("download must succeed");
    let elapsed = start_ticks.elapsed();
    println!("Metal steps and readback for 10 ticks and 10,000 particles: {elapsed:?}");
    assert_eq!(final_state.particles.len(), n_particles);
    assert_ne!(
        final_state.particles[0].x, state.particles[0].x,
        "Particles must have moved"
    );
}

#[test]
fn test_metal_agrees_with_cpu_for_large_workload() {
    let mut backend = MetalBackend::initialize_macos_hardware_only().unwrap();
    let mut cpu = kairo_ecs_gpu::compute::CpuFallbackCompute::new();

    let n_particles = 2_048;
    let mut particles = Vec::with_capacity(n_particles);
    for i in 0..n_particles {
        particles.push(AgentParticle {
            x: (i as f32) * 0.1,
            y: (i as f32) * -0.2,
            vx: ((i % 10) as f32) - 5.0,
            vy: ((i % 7) as f32) - 3.0,
        });
    }

    let state = GpuState {
        particles,
        entity_values: vec![42],
    };

    backend.upload_state(&state).unwrap();
    cpu.upload_state(&state).unwrap();

    let dt = 0.05;
    let seed = 987654321;

    backend.run_abm_step(dt, seed).unwrap();
    cpu.run_abm_step(dt, seed).unwrap();

    let metal_out = backend.download_state().unwrap();
    let cpu_out = cpu.download_state().unwrap();

    for (idx, (m, c)) in metal_out
        .particles
        .iter()
        .zip(cpu_out.particles.iter())
        .enumerate()
    {
        assert!(
            (m.x - c.x).abs() <= 5e-5,
            "Particle {idx} x mismatch: metal {}, cpu {}",
            m.x,
            c.x
        );
        assert!(
            (m.y - c.y).abs() <= 5e-5,
            "Particle {idx} y mismatch: metal {}, cpu {}",
            m.y,
            c.y
        );
    }
}
