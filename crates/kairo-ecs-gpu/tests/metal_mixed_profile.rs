#![cfg(all(target_os = "macos", feature = "wgpu-backend"))]

use std::time::{Duration, Instant};

use kairo_ecs_gpu::{
    AgentParticle, CpuFallbackCompute, DesEvent, GpuCompute, GpuState, MetalBackend,
};

fn run(
    backend: &mut impl GpuCompute,
    initial: &GpuState,
    batches: &[Vec<DesEvent>],
) -> (Duration, GpuState) {
    let started = Instant::now();
    backend.upload_state(initial).unwrap();
    for (tick, events) in batches.iter().enumerate() {
        let seed = 20261123 + tick as u64;
        backend.run_abm_step(0.125, seed).unwrap();
        backend.run_des_step(events).unwrap();
    }
    let state = backend.download_state().unwrap();
    (started.elapsed(), state)
}

#[test]
#[ignore = "manual device timing on an idle Apple Metal host"]
fn measure_mixed_device_end_to_end() {
    let started = Instant::now();
    let mut gpu = MetalBackend::initialize_macos_hardware_only().unwrap();
    println!(
        "adapter {:?} setup_ms {:.6}",
        gpu.adapter_info(),
        started.elapsed().as_secs_f64() * 1000.0
    );
    let mut cpu = CpuFallbackCompute::new();

    println!("entities\tevents_per_batch\tbatches\tpair\tfirst\tcpu_ms\tmetal_ms");
    for (entities, event_count, batch_count) in [(65, 131, 4), (4096, 16384, 16), (65536, 65536, 8)]
    {
        let initial = GpuState {
            particles: (0..entities)
                .map(|index| AgentParticle {
                    x: (index % 43) as f32 / 8.0,
                    y: (index % 53) as f32 / 8.0,
                    vx: (index % 7) as f32 / 4.0,
                    vy: -((index % 9) as f32) / 4.0,
                })
                .collect(),
            entity_values: (0..entities).map(|index| (index % 31) - 15).collect(),
        };
        let batches: Vec<Vec<_>> = (0..batch_count)
            .map(|batch| {
                (0..event_count)
                    .rev()
                    .map(|index| DesEvent {
                        timestamp_ns: (batch * event_count + index) as u64,
                        entity_id: (index * 17 % entities) as u64,
                        delta: ((index + batch) % 13) - 6,
                    })
                    .collect()
            })
            .collect();
        let (_, warmup) = run(&mut gpu, &initial, &batches);
        run(&mut cpu, &initial, &batches);
        for pair in 0..12 {
            let ((cpu_duration, reference), (gpu_duration, actual), first) = if pair % 2 == 0 {
                let reference = run(&mut cpu, &initial, &batches);
                let actual = run(&mut gpu, &initial, &batches);
                (reference, actual, "cpu")
            } else {
                let actual = run(&mut gpu, &initial, &batches);
                let reference = run(&mut cpu, &initial, &batches);
                (reference, actual, "metal")
            };
            assert_eq!(actual.entity_values, reference.entity_values);
            assert_eq!(actual, warmup);
            assert_eq!(actual.particles.len(), reference.particles.len());
            for (particle, reference) in actual.particles.iter().zip(reference.particles) {
                assert!((particle.x - reference.x).abs() < 2e-5);
                assert!((particle.y - reference.y).abs() < 2e-5);
                assert_eq!((particle.vx, particle.vy), (reference.vx, reference.vy));
            }
            println!(
                "{entities}\t{event_count}\t{batch_count}\t{pair}\t{first}\t{:.6}\t{:.6}",
                cpu_duration.as_secs_f64() * 1000.0,
                gpu_duration.as_secs_f64() * 1000.0,
            );
        }
    }
}
