use super::{CudaDeviceInfo, CudaEntityDelta, CudaParticle, ABM_STEP_PTX};
use cudarc::driver::{
    CudaContext, CudaFunction, CudaSlice, CudaStream, DeviceRepr, LaunchConfig, PushKernelArg,
};
use std::sync::Arc;

pub struct CudaDeviceSession {
    info: CudaDeviceInfo,
    model: ResidentModel<NativeStream>,
}

impl CudaDeviceSession {
    pub fn new(device_ordinal: usize) -> Result<Self, String> {
        let result = std::panic::catch_unwind(|| CudaContext::new(device_ordinal));
        let context = match result {
            Ok(Ok(context)) => context,
            Ok(Err(error)) => {
                return Err(format!(
                    "failed to initialize CUDA context on device {device_ordinal}: {error:?}"
                ))
            }
            Err(_) => {
                return Err(
                    "CUDA driver dynamic library unavailable or panicked during initialization"
                        .to_string(),
                )
            }
        };
        let info = CudaDeviceInfo {
            device_name: context.name().map_err(context_error("device name"))?,
            total_memory_bytes: context
                .total_mem()
                .map_err(context_error("device memory"))?,
            compute_capability: context
                .compute_capability()
                .map_err(context_error("compute capability"))?,
        };
        let module = context
            .load_module(cudarc::nvrtc::Ptx::from_src(ABM_STEP_PTX))
            .map_err(context_error("PTX module"))?;
        let stream = NativeStream {
            stream: context.default_stream(),
            abm: module
                .load_function("abm_step_kernel")
                .map_err(context_error("ABM kernel"))?,
            des: module
                .load_function("des_add_kernel")
                .map_err(context_error("DES kernel"))?,
        };
        Ok(Self {
            info,
            model: ResidentModel::new(stream),
        })
    }

    pub fn device_info(&self) -> CudaDeviceInfo {
        self.info.clone()
    }

    pub fn upload_particles(&mut self, particles: &[CudaParticle]) -> Result<usize, String> {
        self.model.upload_particles(particles)
    }

    pub fn step_abm(&mut self, dt: f32, seed: u64) -> Result<u32, String> {
        self.model.step_abm(dt, seed)
    }

    /// Upload a complete model and retain the old buffers until both copies finish.
    pub fn upload_model(
        &mut self,
        particles: &[CudaParticle],
        entities: &[i32],
    ) -> Result<usize, String> {
        self.model.upload_model(particles, entities)
    }

    pub fn resident_entity_count(&self) -> Option<usize> {
        self.model.resident_entity_count()
    }

    pub fn step_des(&mut self, events: &[CudaEntityDelta]) -> Result<u32, String> {
        self.model.step_des(events)
    }

    pub fn download_entities(&self) -> Result<Vec<i32>, String> {
        self.model.download_entities()
    }

    pub fn download_particles(&self) -> Result<Vec<CudaParticle>, String> {
        self.model.download_particles()
    }
}

const THREADS_PER_BLOCK: u32 = 256;

#[derive(Clone, Copy)]
struct Dispatch(u32);

impl Dispatch {
    fn blocks(self) -> u32 {
        self.0.div_ceil(THREADS_PER_BLOCK)
    }

    fn config(self) -> LaunchConfig {
        LaunchConfig {
            grid_dim: (self.blocks(), 1, 1),
            block_dim: (THREADS_PER_BLOCK, 1, 1),
            shared_mem_bytes: 0,
        }
    }
}

trait ModelStream {
    type Buffer<T>;

    fn copy_to_device<T: DeviceRepr + Copy>(&self, host: &[T]) -> Result<Self::Buffer<T>, String>;
    fn copy_to_host<T: DeviceRepr + Copy>(
        &self,
        device: &Self::Buffer<T>,
    ) -> Result<Vec<T>, String>;
    fn synchronize(&self) -> Result<(), String>;
    fn launch_abm(
        &self,
        particles: &mut Self::Buffer<CudaParticle>,
        dispatch: Dispatch,
        dt: f32,
        seed: u64,
    ) -> Result<(), String>;
    fn launch_des(
        &self,
        values: &mut Self::Buffer<i32>,
        events: &Self::Buffer<CudaEntityDelta>,
        dispatch: Dispatch,
    ) -> Result<(), String>;
}

struct NativeStream {
    stream: Arc<CudaStream>,
    abm: CudaFunction,
    des: CudaFunction,
}

fn driver_error(error: cudarc::driver::DriverError) -> String {
    format!("CUDA driver call failed: {error:?}")
}

fn context_error(operation: &'static str) -> impl FnOnce(cudarc::driver::DriverError) -> String {
    move |error| format!("CUDA {operation} initialization failed: {error:?}")
}

impl ModelStream for NativeStream {
    type Buffer<T> = CudaSlice<T>;

    fn copy_to_device<T: DeviceRepr + Copy>(&self, host: &[T]) -> Result<CudaSlice<T>, String> {
        self.stream.clone_htod(host).map_err(driver_error)
    }

    fn copy_to_host<T: DeviceRepr + Copy>(&self, device: &CudaSlice<T>) -> Result<Vec<T>, String> {
        self.stream.clone_dtoh(device).map_err(driver_error)
    }

    fn synchronize(&self) -> Result<(), String> {
        self.stream.synchronize().map_err(driver_error)
    }

    fn launch_abm(
        &self,
        particles: &mut CudaSlice<CudaParticle>,
        dispatch: Dispatch,
        dt: f32,
        seed: u64,
    ) -> Result<(), String> {
        let mut launch = self.stream.launch_builder(&self.abm);
        launch.arg(particles).arg(&dispatch.0).arg(&dt).arg(&seed);
        // The validated count is the slice length and the model holds it through synchronization.
        unsafe { launch.launch(dispatch.config()) }
            .map(|_| ())
            .map_err(driver_error)
    }

    fn launch_des(
        &self,
        values: &mut CudaSlice<i32>,
        events: &CudaSlice<CudaEntityDelta>,
        dispatch: Dispatch,
    ) -> Result<(), String> {
        let mut launch = self.stream.launch_builder(&self.des);
        launch.arg(values).arg(events).arg(&dispatch.0);
        // The events were bounds-checked before upload. Both slices outlive synchronization.
        unsafe { launch.launch(dispatch.config()) }
            .map(|_| ())
            .map_err(driver_error)
    }
}

enum ParticleState {
    AwaitingUpload,
    Resident(u32),
    Indeterminate,
}

const PARTICLE_RECOVERY: &str = "upload particle state after a failed CUDA submission";

struct ResidentModel<S: ModelStream> {
    stream: S,
    entity_buffer: Option<S::Buffer<i32>>,
    entity_count: Option<u32>,
    particle_buffer: Option<S::Buffer<CudaParticle>>,
    particle_state: ParticleState,
}

impl<S: ModelStream> ResidentModel<S> {
    fn new(stream: S) -> Self {
        Self {
            stream,
            entity_buffer: None,
            entity_count: None,
            particle_buffer: None,
            particle_state: ParticleState::AwaitingUpload,
        }
    }

    fn stage<T: DeviceRepr + Copy>(&self, host: &[T]) -> Result<Option<S::Buffer<T>>, String> {
        (!host.is_empty())
            .then(|| self.stream.copy_to_device(host))
            .transpose()
    }

    fn synchronized<T>(&self, result: Result<T, String>) -> Result<T, String> {
        let fence = self.stream.synchronize();
        let result = result?;
        fence?;
        Ok(result)
    }

    fn upload_particles(&mut self, particles: &[CudaParticle]) -> Result<usize, String> {
        let count = u32::try_from(particles.len()).map_err(|_| "too many CUDA particles")?;
        self.particle_buffer = self.synchronized(self.stage(particles))?;
        self.particle_state = ParticleState::Resident(count);
        Ok(std::mem::size_of_val(particles))
    }

    fn upload_model(
        &mut self,
        particles: &[CudaParticle],
        entities: &[i32],
    ) -> Result<usize, String> {
        let particle_count =
            u32::try_from(particles.len()).map_err(|_| "too many CUDA particles")?;
        let entity_count = u32::try_from(entities.len()).map_err(|_| "too many CUDA entities")?;
        let bytes = std::mem::size_of_val(particles)
            .checked_add(std::mem::size_of_val(entities))
            .ok_or("CUDA upload byte size overflow")?;
        let particles = self.stage(particles);
        let entities = if particles.is_ok() {
            self.stage(entities)
        } else {
            Ok(None)
        };
        // Keep successfully staged allocations alive through the fence on both outcomes.
        let fence = self.stream.synchronize();
        let (particles, entities) = (particles?, entities?);
        fence?;
        self.particle_buffer = particles;
        self.particle_state = ParticleState::Resident(particle_count);
        self.entity_buffer = entities;
        self.entity_count = Some(entity_count);
        Ok(bytes)
    }

    fn resident_entity_count(&self) -> Option<usize> {
        self.entity_count.map(|count| count as usize)
    }

    fn step_abm(&mut self, dt: f32, seed: u64) -> Result<u32, String> {
        let count = match self.particle_state {
            ParticleState::AwaitingUpload | ParticleState::Resident(0) => return Ok(0),
            ParticleState::Resident(count) => count,
            ParticleState::Indeterminate => return Err(PARTICLE_RECOVERY.into()),
        };
        let particles = self
            .particle_buffer
            .as_mut()
            .ok_or("no particles uploaded")?;
        let dispatch = Dispatch(count);
        let submitted = self.stream.launch_abm(particles, dispatch, dt, seed);
        if let Err(error) = self.synchronized(submitted) {
            self.particle_state = ParticleState::Indeterminate;
            self.entity_count = None;
            return Err(error);
        }
        Ok(dispatch.blocks())
    }

    fn step_des(&mut self, events: &[CudaEntityDelta]) -> Result<u32, String> {
        let entity_count = self
            .entity_count
            .ok_or("upload CUDA entity state before dispatch")?;
        if let Some(event) = events.iter().find(|event| event.entity >= entity_count) {
            return Err(format!(
                "CUDA entity index {} is out of range",
                event.entity
            ));
        }
        if events.is_empty() {
            return Ok(0);
        }
        let dispatch =
            Dispatch(u32::try_from(events.len()).map_err(|_| "too many CUDA DES events")?);
        let events = match self.stream.copy_to_device(events) {
            Ok(events) => events,
            Err(error) => return self.synchronized(Err(error)),
        };
        let values = self
            .entity_buffer
            .as_mut()
            .expect("validated resident entity buffer");
        let submitted = self.stream.launch_des(values, &events, dispatch);
        if let Err(error) = self.synchronized(submitted) {
            self.entity_count = None;
            return Err(error);
        }
        Ok(dispatch.blocks())
    }

    fn download_entities(&self) -> Result<Vec<i32>, String> {
        let count = self
            .entity_count
            .ok_or("CUDA entity state is unavailable")?;
        if count == 0 {
            return Ok(Vec::new());
        }
        let buffer = self.entity_buffer.as_ref().expect("resident entity buffer");
        self.synchronized(self.stream.copy_to_host(buffer))
    }

    fn download_particles(&self) -> Result<Vec<CudaParticle>, String> {
        match self.particle_state {
            ParticleState::Resident(0) => return Ok(Vec::new()),
            ParticleState::Resident(_) => {}
            ParticleState::AwaitingUpload => return Err("no particles uploaded".into()),
            ParticleState::Indeterminate => return Err(PARTICLE_RECOVERY.into()),
        }
        let buffer = self
            .particle_buffer
            .as_ref()
            .ok_or("no particles uploaded")?;
        self.synchronized(self.stream.copy_to_host(buffer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum Call {
        Upload(usize),
        Fence,
        Abm(u32),
        Des(u32),
        Read(usize),
    }

    #[derive(Default)]
    struct RecordedStream {
        calls: RefCell<Vec<Call>>,
        fail_at: Cell<Option<usize>>,
    }

    impl RecordedStream {
        fn record(&self, call: Call) -> Result<(), String> {
            let mut calls = self.calls.borrow_mut();
            calls.push(call);
            if self.fail_at.get() == Some(calls.len()) {
                return Err("injected stream failure".into());
            }
            Ok(())
        }

        fn expect_calls(&self, expected: &[Call]) {
            assert_eq!(self.calls.take(), expected);
            self.fail_at.set(None);
        }
    }

    impl ModelStream for RecordedStream {
        type Buffer<T> = Vec<T>;

        fn copy_to_device<T: DeviceRepr + Copy>(&self, host: &[T]) -> Result<Vec<T>, String> {
            self.record(Call::Upload(std::mem::size_of_val(host)))?;
            Ok(host.to_vec())
        }

        fn copy_to_host<T: DeviceRepr + Copy>(&self, device: &Vec<T>) -> Result<Vec<T>, String> {
            self.record(Call::Read(std::mem::size_of_val(device.as_slice())))?;
            Ok(device.clone())
        }

        fn synchronize(&self) -> Result<(), String> {
            self.record(Call::Fence)
        }

        fn launch_abm(
            &self,
            particles: &mut Vec<CudaParticle>,
            dispatch: Dispatch,
            _dt: f32,
            _seed: u64,
        ) -> Result<(), String> {
            assert_eq!(dispatch.0 as usize, particles.len());
            assert_eq!(dispatch.config().grid_dim.0, dispatch.blocks());
            self.record(Call::Abm(dispatch.blocks()))
        }

        fn launch_des(
            &self,
            values: &mut Vec<i32>,
            events: &Vec<CudaEntityDelta>,
            dispatch: Dispatch,
        ) -> Result<(), String> {
            assert_eq!(dispatch.0 as usize, events.len());
            assert!(events
                .iter()
                .all(|event| (event.entity as usize) < values.len()));
            self.record(Call::Des(dispatch.blocks()))
        }
    }

    #[test]
    fn resident_state_fences_transfers_and_dispatches_without_readback() {
        let mut model = ResidentModel::new(RecordedStream::default());
        assert_eq!(model.resident_entity_count(), None);
        assert_eq!(model.step_abm(0.25, 8), Ok(0));
        assert!(model.step_des(&[]).is_err());
        assert!(model.download_particles().is_err());
        assert!(model.download_entities().is_err());
        model.stream.expect_calls(&[]);

        let particles = vec![CudaParticle::default(); 257];
        let entities = [2, -3];
        let events = vec![
            CudaEntityDelta {
                entity: 1,
                delta: 2
            };
            513
        ];
        assert_eq!(model.upload_model(&particles, &entities), Ok(4120));
        assert_eq!(model.resident_entity_count(), Some(2));
        assert_eq!(model.step_abm(0.25, 8), Ok(2));
        assert_eq!(model.step_des(&events), Ok(3));
        model.stream.expect_calls(&[
            Call::Upload(4112),
            Call::Upload(8),
            Call::Fence,
            Call::Abm(2),
            Call::Fence,
            Call::Upload(4104),
            Call::Des(3),
            Call::Fence,
        ]);

        let invalid = [
            events[0],
            CudaEntityDelta {
                entity: 2,
                delta: 1,
            },
        ];
        assert!(model.step_des(&invalid).is_err());
        assert_eq!(model.step_des(&[]), Ok(0));
        assert_eq!(model.download_particles().unwrap(), particles);
        assert_eq!(model.download_entities().unwrap(), entities);
        model
            .stream
            .expect_calls(&[Call::Read(4112), Call::Fence, Call::Read(8), Call::Fence]);

        assert_eq!(model.upload_model(&[], &[]), Ok(0));
        assert_eq!(model.resident_entity_count(), Some(0));
        assert_eq!(model.step_abm(0.25, 8), Ok(0));
        assert!(model.download_particles().unwrap().is_empty());
        assert!(model.download_entities().unwrap().is_empty());
        model.stream.expect_calls(&[Call::Fence]);
    }

    #[test]
    fn failed_replacement_keeps_both_previous_buffers() {
        let mut model = ResidentModel::new(RecordedStream::default());
        let initial = [CudaParticle::default()];
        let replacement = [CudaParticle {
            x: 4.0,
            ..CudaParticle::default()
        }; 2];
        model.upload_model(&initial, &[7]).unwrap();
        model
            .stream
            .expect_calls(&[Call::Upload(16), Call::Upload(4), Call::Fence]);
        for (fail_at, expected) in [
            (1, [Call::Upload(32), Call::Fence].as_slice()),
            (
                2,
                [Call::Upload(32), Call::Upload(8), Call::Fence].as_slice(),
            ),
            (
                3,
                [Call::Upload(32), Call::Upload(8), Call::Fence].as_slice(),
            ),
        ] {
            model.stream.fail_at.set(Some(fail_at));
            assert!(model.upload_model(&replacement, &[9, 10]).is_err());
            model.stream.expect_calls(expected);
            assert_eq!(model.download_particles().unwrap(), initial);
            assert_eq!(model.download_entities().unwrap(), [7]);
            model
                .stream
                .expect_calls(&[Call::Read(16), Call::Fence, Call::Read(4), Call::Fence]);
        }
    }

    #[test]
    fn failed_dispatch_requires_reupload_before_reusing_affected_state() {
        let mut model = ResidentModel::new(RecordedStream::default());
        let particles = [CudaParticle::default()];
        let events = [CudaEntityDelta {
            entity: 0,
            delta: 1,
        }];
        model.upload_model(&particles, &[7]).unwrap();
        model
            .stream
            .expect_calls(&[Call::Upload(16), Call::Upload(4), Call::Fence]);

        model.stream.fail_at.set(Some(1));
        assert!(model.step_des(&events).is_err());
        model.stream.expect_calls(&[Call::Upload(8), Call::Fence]);
        assert_eq!(model.resident_entity_count(), Some(1));

        model.stream.fail_at.set(Some(3));
        assert!(model.step_des(&events).is_err());
        model
            .stream
            .expect_calls(&[Call::Upload(8), Call::Des(1), Call::Fence]);
        assert_eq!(model.resident_entity_count(), None);
        assert!(model.download_entities().is_err());
        assert_eq!(model.download_particles().unwrap(), particles);
        model.stream.expect_calls(&[Call::Read(16), Call::Fence]);

        model.upload_model(&particles, &[7]).unwrap();
        assert_eq!(model.resident_entity_count(), Some(1));
        model
            .stream
            .expect_calls(&[Call::Upload(16), Call::Upload(4), Call::Fence]);
        model.stream.fail_at.set(Some(1));
        assert!(model.step_abm(0.25, 7).is_err());
        assert_eq!(model.resident_entity_count(), None);
        assert!(model.step_des(&events).is_err());
        assert_eq!(model.step_abm(0.25, 7), Err(PARTICLE_RECOVERY.into()));
        assert_eq!(model.download_particles(), Err(PARTICLE_RECOVERY.into()));
        model.stream.expect_calls(&[Call::Abm(1), Call::Fence]);

        model.upload_particles(&particles).unwrap();
        assert_eq!(model.step_abm(0.25, 7), Ok(1));
        assert!(model.download_entities().is_err());
        model
            .stream
            .expect_calls(&[Call::Upload(16), Call::Fence, Call::Abm(1), Call::Fence]);
        model.upload_model(&particles, &[7]).unwrap();
        assert_eq!(model.step_des(&events), Ok(1));
        model.stream.expect_calls(&[
            Call::Upload(16),
            Call::Upload(4),
            Call::Fence,
            Call::Upload(8),
            Call::Des(1),
            Call::Fence,
        ]);
        model.stream.fail_at.set(Some(2));
        assert!(model.download_entities().is_err());
        model.stream.expect_calls(&[Call::Read(4), Call::Fence]);
        assert_eq!(model.download_entities(), Ok(vec![7]));
    }

    #[test]
    #[ignore = "requires a native NVIDIA CUDA device"]
    fn indeterminate_particles_require_reupload() {
        let mut device = CudaDeviceSession::new(0).unwrap();
        let particle = CudaParticle {
            x: 0.0,
            y: 1.0,
            vx: 0.5,
            vy: 0.25,
        };
        device.upload_particles(&[particle]).unwrap();
        device.model.particle_state = ParticleState::Indeterminate;
        assert!(device.step_abm(0.25, 7).is_err());
        assert!(device.download_particles().is_err());
        device.upload_particles(&[particle]).unwrap();
        assert_eq!(device.step_abm(0.25, 7).unwrap(), 1);
        assert_eq!(device.download_particles().unwrap().len(), 1);
    }
}
