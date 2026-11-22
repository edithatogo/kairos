#![cfg(all(target_os = "macos", feature = "wgpu-backend"))]

use kairo_ecs_gpu::{AgentParticle, GpuBackendAvailability, GpuCompute, GpuState, MetalBackend};

#[test]
fn empty_upload_retires_previous_native_particle_buffer() {
    let mut backend = MetalBackend::initialize_macos_hardware_only().unwrap();
    assert_eq!(
        backend.capabilities().availability,
        GpuBackendAvailability::Available
    );
    let previous = GpuState {
        particles: vec![AgentParticle {
            x: 0.0,
            y: 0.0,
            vx: 1.0,
            vy: -1.0,
        }],
        entity_values: vec![40],
    };
    backend.upload_state(&previous).unwrap();
    assert_eq!(
        backend.run_abm_step(0.25, 7).unwrap().dispatched_workgroups,
        1
    );
    let empty = GpuState {
        particles: vec![],
        entity_values: vec![41],
    };
    backend.upload_state(&empty).unwrap();
    assert_eq!(
        backend.run_abm_step(0.25, 8).unwrap().dispatched_workgroups,
        0
    );
    assert_eq!(backend.download_state().unwrap(), empty);
}

fn fractional_offset(seed: u64, index: usize) -> f64 {
    // Independent scalar calculation of the 32-bit kernel hash.
    let word = (seed & 0xffff_ffff) ^ ((index as u64 * 747_796_405) & 0xffff_ffff);
    let shifted = word >> (4 + word / (1 << 28));
    let mixed = ((word ^ shifted) * 277_803_737) & 0xffff_ffff;
    let narrow = (mixed ^ (mixed >> 22)) & 0xffff;
    (narrow as f64 / 65_535.0 - 0.5) * 0.001
}

#[test]
fn resident_steps_match_independent_motion_and_replay() {
    let mut backend = MetalBackend::initialize_macos_hardware_only().unwrap();
    assert_eq!(
        backend.capabilities().availability,
        GpuBackendAvailability::Available
    );
    println!("selected adapter {:?}", backend.adapter_info());
    assert!(matches!(
        backend.download_state(),
        Err(kairo_ecs_gpu::GpuComputeError::StateNotResident)
    ));
    let state = GpuState {
        particles: (0..65)
            .map(|i| AgentParticle {
                x: (i as f32 - 32.0) / 32.0,
                y: (32.0 - i as f32) / 32.0,
                vx: (i % 5) as f32 / 4.0,
                vy: -((i % 9) as f32) / 4.0,
            })
            .collect(),
        entity_values: vec![5, 9],
    };
    let steps = [(0.25, 11), (0.5, 42), (0.125, 87), (0.25, 999)];
    let mut run = || {
        let upload = backend.upload_state(&state).unwrap();
        assert_eq!(
            upload.uploaded_bytes,
            65 * std::mem::size_of::<AgentParticle>()
        );
        for (dt, seed) in steps {
            let dispatch = backend.run_abm_step(dt, seed).unwrap();
            assert_eq!(dispatch.uploaded_bytes, 16);
            assert_eq!(dispatch.downloaded_bytes, 0);
            assert_eq!(dispatch.dispatched_workgroups, 2);
        }
        backend.download_state().unwrap()
    };
    let first = run();
    let replay = run();
    assert_eq!(first.entity_values, state.entity_values);
    assert_eq!(first.particles.len(), state.particles.len());
    for (i, (actual, initial)) in first.particles.iter().zip(&state.particles).enumerate() {
        let (mut x, mut y) = (f64::from(initial.x), f64::from(initial.y));
        for (dt, seed) in steps {
            let delta = fractional_offset(seed, i);
            x += (f64::from(initial.vx) + delta) * f64::from(dt);
            y += (f64::from(initial.vy) - delta) * f64::from(dt);
        }
        assert!((x - f64::from(actual.x)).abs() <= 2e-6, "particle {i} x");
        assert!((y - f64::from(actual.y)).abs() <= 2e-6, "particle {i} y");
        assert_eq!(actual.vx.to_bits(), initial.vx.to_bits());
        assert_eq!(actual.vy.to_bits(), initial.vy.to_bits());
        assert_eq!(actual.x.to_bits(), replay.particles[i].x.to_bits());
        assert_eq!(actual.y.to_bits(), replay.particles[i].y.to_bits());
    }

    std::thread::scope(|scope| {
        let left = scope.spawn(|| backend.download_state().unwrap());
        let right = scope.spawn(|| backend.download_state().unwrap());
        assert_eq!(left.join().unwrap(), first);
        assert_eq!(right.join().unwrap(), first);
    });

    let events = [kairo_ecs_gpu::DesEvent {
        timestamp_ns: 1,
        entity_id: 0,
        delta: 50,
    }];
    assert_eq!(
        backend.run_des_step(&events),
        Err(kairo_ecs_gpu::GpuComputeError::UnsupportedBackend(
            kairo_ecs_gpu::METAL_BACKEND_NAME
        ))
    );
    let mut sibling = backend.clone();
    assert_eq!(
        sibling.capabilities().availability,
        GpuBackendAvailability::CpuFallback
    );
    assert_eq!(sibling.download_state().unwrap(), first);
    let host_step = sibling.run_des_step(&events).unwrap();
    assert_eq!(host_step, kairo_ecs_gpu::GpuStepStats::default());
    assert_eq!(sibling.download_state().unwrap().entity_values, [55, 9]);
    assert_eq!(backend.download_state().unwrap(), first);
}

#[test]
#[ignore = "explicit release-profile measurement on a native Apple GPU"]
fn measure_native_abm_end_to_end() {
    use kairo_ecs_gpu::CpuFallbackCompute;
    use std::time::Instant;

    // Construction and pipeline setup are measured separately.
    let started = Instant::now();
    let mut native = MetalBackend::initialize_macos_hardware_only().unwrap();
    println!(
        "setup_ns={} adapter={}",
        started.elapsed().as_nanos(),
        native.adapter_info().adapter_name
    );
    let mut cpu = CpuFallbackCompute::new();

    fn run(backend: &mut impl GpuCompute, state: &GpuState, steps: u64) -> (GpuState, u128) {
        let started = Instant::now();
        backend.upload_state(state).unwrap();
        for step in 0..steps {
            backend.run_abm_step(0.0625, 20261123 + step).unwrap();
        }
        let result = backend.download_state().unwrap();
        (result, started.elapsed().as_nanos())
    }

    for (count, steps) in [
        (4_096, 1),
        (4_096, 16),
        (65_536, 1),
        (65_536, 16),
        (262_144, 16),
    ] {
        let state = GpuState {
            particles: (0..count)
                .map(|i| AgentParticle {
                    x: (i % 64) as f32 / 64.0,
                    y: -((i % 32) as f32) / 32.0,
                    vx: (i % 7) as f32 / 8.0,
                    vy: -((i % 5) as f32) / 8.0,
                })
                .collect(),
            entity_values: vec![],
        };
        run(&mut cpu, &state, steps);
        run(&mut native, &state, steps);
        for pair in 0..12 {
            let (cpu_result, cpu_ns, gpu_result, gpu_ns) = if pair % 2 == 0 {
                let (cpu_result, cpu_ns) = run(&mut cpu, &state, steps);
                let (gpu_result, gpu_ns) = run(&mut native, &state, steps);
                (cpu_result, cpu_ns, gpu_result, gpu_ns)
            } else {
                let (gpu_result, gpu_ns) = run(&mut native, &state, steps);
                let (cpu_result, cpu_ns) = run(&mut cpu, &state, steps);
                (cpu_result, cpu_ns, gpu_result, gpu_ns)
            };
            assert_eq!(gpu_result.particles.len(), count);
            for (gpu, cpu) in gpu_result.particles.iter().zip(cpu_result.particles) {
                assert!((gpu.x - cpu.x).abs() <= 2e-6);
                assert!((gpu.y - cpu.y).abs() <= 2e-6);
            }
            println!("sample {count} {steps} {pair} {cpu_ns} {gpu_ns}");
        }
    }
}
