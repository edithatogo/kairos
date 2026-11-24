// SPDX-License-Identifier: Apache-2.0
//! Run the resident model shaders against a native Linux Vulkan GPU.

#[cfg(all(target_os = "linux", feature = "wgpu-backend"))]
fn main() {
    linux::qualify();
}

#[cfg(not(all(target_os = "linux", feature = "wgpu-backend")))]
fn main() {
    eprintln!("This qualification requires Linux, wgpu-backend and a hardware Vulkan device.");
    std::process::exit(2);
}

#[cfg(all(target_os = "linux", feature = "wgpu-backend"))]
mod linux {
    use std::time::Duration;

    use kairo_ecs_gpu::{AgentParticle, CpuFallbackCompute, DesEvent, GpuCompute, GpuState};
    use wgpu::{util::DeviceExt as _, Buffer, BufferUsages, ComputePipeline, Device};

    const PARTICLES: u32 = 67;
    const EVENTS: u32 = 131;
    const TICKS: u64 = 4;
    const SEED: u64 = 20261125;
    const DT: f32 = 0.125;

    fn pipeline(device: &Device, source: &'static str) -> ComputePipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        })
    }

    fn buffer(device: &Device, contents: &[u8], usage: BufferUsages) -> Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents,
            usage,
        })
    }

    fn dispatch(
        device: &Device,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &ComputePipeline,
        first: &Buffer,
        second: &Buffer,
        count: u32,
    ) {
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: first.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: second.as_entire_binding(),
                },
            ],
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bindings, &[]);
        pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
    }

    pub fn qualify() {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .expect("a Vulkan adapter is required");
        let identity = adapter.get_info();
        assert_eq!(identity.backend, wgpu::Backend::Vulkan);
        assert!(
            matches!(
                identity.device_type,
                wgpu::DeviceType::IntegratedGpu | wgpu::DeviceType::DiscreteGpu
            ),
            "physical Vulkan GPU required, observed {identity:?}"
        );
        println!("adapter={identity:?}");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
        let abm = pipeline(&device, include_str!("../src/shaders/resident_abm.wgsl"));
        let des = pipeline(&device, include_str!("../src/shaders/resident_des.wgsl"));
        let initial = GpuState {
            particles: (0..PARTICLES)
                .map(|index| AgentParticle {
                    x: index as f32 * 0.125,
                    y: -(index as f32) * 0.125,
                    vx: 1.0,
                    vy: -0.25,
                })
                .collect(),
            entity_values: (0..PARTICLES).map(|i| i32::MAX - i as i32).collect(),
        };
        let mut cpu = CpuFallbackCompute::new();
        cpu.upload_state(&initial).unwrap();
        let usage = BufferUsages::STORAGE | BufferUsages::COPY_SRC;
        let particles = buffer(&device, bytemuck::cast_slice(&initial.particles), usage);
        let values = buffer(&device, bytemuck::cast_slice(&initial.entity_values), usage);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        for tick in 0..TICKS {
            let seed = SEED + tick;
            let events: Vec<_> = (0..EVENTS)
                .rev()
                .map(|i| DesEvent {
                    timestamp_ns: tick * u64::from(EVENTS) + u64::from(i),
                    entity_id: u64::from(i * 17 % PARTICLES),
                    delta: ((u64::from(i) + tick) % 13) as i32 - 6,
                })
                .collect();
            cpu.run_abm_step(DT, seed).unwrap();
            cpu.run_des_step(&events).unwrap();

            let params = [DT.to_bits(), seed as u32, (seed >> 32) as u32, PARTICLES];
            let params = buffer(&device, bytemuck::bytes_of(&params), BufferUsages::UNIFORM);
            let packed: Vec<_> = events
                .iter()
                .map(|event| [event.entity_id as u32, event.delta as u32])
                .collect();
            let packed = buffer(
                &device,
                bytemuck::cast_slice(&packed),
                BufferUsages::STORAGE,
            );
            dispatch(&device, &mut encoder, &abm, &particles, &params, PARTICLES);
            dispatch(&device, &mut encoder, &des, &values, &packed, EVENTS);
        }
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: particles.size() + values.size(),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&particles, 0, &staging, 0, particles.size());
        encoder.copy_buffer_to_buffer(&values, 0, &staging, particles.size(), values.size());
        let submission = queue.submit([encoder.finish()]);
        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let timeout = Duration::from_secs(30);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(timeout),
            })
            .unwrap();
        receiver.recv_timeout(timeout).unwrap().unwrap();
        let readback = slice.get_mapped_range().unwrap();
        let (particle_bytes, value_bytes) = readback.split_at(particles.size() as usize);
        let observed_particles: &[AgentParticle] = bytemuck::cast_slice(particle_bytes);
        let observed_values: &[i32] = bytemuck::cast_slice(value_bytes);
        let expected = cpu.download_state().unwrap();
        assert_eq!(observed_values, expected.entity_values);
        assert_eq!(observed_particles.len(), expected.particles.len());
        let mut max_abs_error = 0.0f32;
        for (actual, expected) in observed_particles.iter().zip(&expected.particles) {
            assert_eq!((actual.vx, actual.vy), (expected.vx, expected.vy));
            for error in [(actual.x - expected.x).abs(), (actual.y - expected.y).abs()] {
                assert!(error <= 2e-5, "particle position error {error}");
                max_abs_error = max_abs_error.max(error);
            }
        }
        drop(readback);
        staging.unmap();
        println!(
            "parity=pass seed={SEED} ticks={TICKS} particles={PARTICLES} events_per_tick={EVENTS} max_abs_error={max_abs_error}"
        );
    }
}
