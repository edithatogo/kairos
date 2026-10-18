#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct Tensor {
    shape: Vec<usize>,
    values: Vec<f32>,
}

impl Tensor {
    pub fn new(shape: impl Into<Vec<usize>>, values: impl Into<Vec<f32>>) -> Result<Self, MlError> {
        let shape = shape.into();
        let values = values.into();
        validate_shape("tensor shape", &shape)?;
        let expected_len = shape.iter().product::<usize>();

        if expected_len != values.len() {
            return Err(MlError::new(format!(
                "tensor shape {:?} expects {} values, got {}",
                shape,
                expected_len,
                values.len()
            )));
        }
        validate_finite_values("tensor values", &values)?;

        Ok(Self { shape, values })
    }

    pub fn scalar(value: f32) -> Self {
        Self {
            shape: vec![1],
            values: vec![value],
        }
    }

    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    pub fn values(&self) -> &[f32] {
        &self.values
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TickPhase {
    BeforeSystems,
    AfterSystems,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FallbackPolicy {
    HoldLastOutput,
    UseOriginalSystem,
    FailTick,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelMetadata {
    pub name: String,
    pub version: String,
    pub input_shape: Vec<usize>,
    pub output_shape: Vec<usize>,
}

impl ModelMetadata {
    pub fn validate(&self) -> Result<(), MlError> {
        if self.name.trim().is_empty() {
            return Err(MlError::new("model name must not be empty"));
        }
        if self.version.trim().is_empty() {
            return Err(MlError::new("model version must not be empty"));
        }
        validate_shape("model input_shape", &self.input_shape)?;
        validate_shape("model output_shape", &self.output_shape)?;
        Ok(())
    }
}

pub trait NeuralSystem: Send + Sync {
    fn metadata(&self) -> &ModelMetadata;
    fn tick_phase(&self) -> TickPhase;
    fn fallback_policy(&self) -> FallbackPolicy;
    fn predict(&self, input: &Tensor) -> Result<Tensor, MlError>;
}

#[derive(Clone)]
pub struct InferenceTickHook {
    systems: Vec<Arc<dyn NeuralSystem>>,
}

impl InferenceTickHook {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn register(&mut self, system: Arc<dyn NeuralSystem>) {
        self.systems.push(system);
    }

    pub fn try_register(&mut self, system: Arc<dyn NeuralSystem>) -> Result<(), MlError> {
        system.metadata().validate()?;
        if self.systems.iter().any(|existing| {
            existing.metadata().name == system.metadata().name
                && existing.metadata().version == system.metadata().version
        }) {
            return Err(MlError::new(format!(
                "model {}:{} is already registered",
                system.metadata().name,
                system.metadata().version
            )));
        }
        self.systems.push(system);
        Ok(())
    }

    pub fn run_phase(&self, phase: TickPhase, input: &Tensor) -> Result<Vec<Tensor>, MlError> {
        self.systems
            .iter()
            .filter(|system| system.tick_phase() == phase)
            .map(|system| {
                validate_model_input(system.metadata(), input)?;
                let output = system.predict(input)?;
                validate_model_output(system.metadata(), &output)?;
                Ok(output)
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.systems.len()
    }

    pub fn is_empty(&self) -> bool {
        self.systems.is_empty()
    }
}

impl Default for InferenceTickHook {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct OrtSession {
    metadata: ModelMetadata,
    model_bytes: Vec<u8>,
}

impl OrtSession {
    pub fn from_bytes(
        name: impl Into<String>,
        version: impl Into<String>,
        model_bytes: impl Into<Vec<u8>>,
        input_shape: Vec<usize>,
        output_shape: Vec<usize>,
    ) -> Result<Self, MlError> {
        let model_bytes = model_bytes.into();
        if model_bytes.is_empty() {
            return Err(MlError::new("model bytes must not be empty"));
        }

        let metadata = ModelMetadata {
            name: name.into(),
            version: version.into(),
            input_shape,
            output_shape,
        };
        metadata.validate()?;

        Ok(Self {
            metadata,
            model_bytes,
        })
    }

    pub fn from_file(
        path: impl AsRef<Path>,
        input_shape: Vec<usize>,
        output_shape: Vec<usize>,
    ) -> Result<Self, MlError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|error| {
            MlError::new(format!(
                "failed to read ONNX model {}: {}",
                path.display(),
                error
            ))
        })?;
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("onnx-model");

        Self::from_bytes(name, "unversioned", bytes, input_shape, output_shape)
    }

    pub fn run(&self, input: &Tensor) -> Result<Tensor, MlError> {
        self.validate_input(input)?;

        let output_len = self.metadata.output_shape.iter().product::<usize>();
        let mut output = Vec::with_capacity(output_len);
        for index in 0..output_len {
            let value = input
                .values()
                .get(index % input.values().len())
                .copied()
                .unwrap_or(0.0);
            output.push(value);
        }

        Tensor::new(self.metadata.output_shape.clone(), output)
    }

    pub fn validate_input(&self, input: &Tensor) -> Result<(), MlError> {
        if input.shape() != self.metadata.input_shape {
            return Err(MlError::new(format!(
                "input shape {:?} does not match model shape {:?}",
                input.shape(),
                self.metadata.input_shape
            )));
        }

        Ok(())
    }

    pub fn model_size_bytes(&self) -> usize {
        self.model_bytes.len()
    }

    pub fn runtime_status(&self) -> BackendStatus {
        BackendStatus::NotConfigured {
            backend: Backend::OnnxRuntime,
            reason: "ONNX Runtime adapter is not wired in the dependency-free scaffold",
        }
    }
}

#[derive(Clone, Debug)]
pub struct OrtNeuralSystem {
    session: OrtSession,
    phase: TickPhase,
    fallback: FallbackPolicy,
}

impl OrtNeuralSystem {
    pub fn new(session: OrtSession, phase: TickPhase, fallback: FallbackPolicy) -> Self {
        Self {
            session,
            phase,
            fallback,
        }
    }
}

impl NeuralSystem for OrtNeuralSystem {
    fn metadata(&self) -> &ModelMetadata {
        &self.session.metadata
    }

    fn tick_phase(&self) -> TickPhase {
        self.phase
    }

    fn fallback_policy(&self) -> FallbackPolicy {
        self.fallback.clone()
    }

    fn predict(&self, input: &Tensor) -> Result<Tensor, MlError> {
        self.session.run(input)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MlError {
    message: String,
}

impl MlError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for MlError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for MlError {}

fn validate_shape(label: &str, shape: &[usize]) -> Result<(), MlError> {
    if shape.is_empty() {
        return Err(MlError::new(format!("{} must not be empty", label)));
    }
    if shape.contains(&0) {
        return Err(MlError::new(format!(
            "{} dimensions must be non-zero",
            label
        )));
    }
    Ok(())
}

fn validate_finite_values(label: &str, values: &[f32]) -> Result<(), MlError> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(MlError::new(format!("{} must be finite", label)));
    }
    Ok(())
}

fn validate_model_input(metadata: &ModelMetadata, input: &Tensor) -> Result<(), MlError> {
    if input.shape() != metadata.input_shape.as_slice() {
        return Err(MlError::new(format!(
            "input shape {:?} does not match model {}:{} input_shape {:?}",
            input.shape(),
            metadata.name,
            metadata.version,
            metadata.input_shape
        )));
    }
    Ok(())
}

fn validate_model_output(metadata: &ModelMetadata, output: &Tensor) -> Result<(), MlError> {
    if output.shape() != metadata.output_shape.as_slice() {
        return Err(MlError::new(format!(
            "output shape {:?} does not match model {}:{} output_shape {:?}",
            output.shape(),
            metadata.name,
            metadata.version,
            metadata.output_shape
        )));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Backend {
    OnnxRuntime,
    TensorRt,
    Burn,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendStatus {
    Configured {
        backend: Backend,
    },
    NotConfigured {
        backend: Backend,
        reason: &'static str,
    },
}

pub fn ensure_backend_configured(backend: Backend) -> Result<(), MlError> {
    Err(MlError::new(format!(
        "{backend:?} backend is not configured in the dependency-free Track 37 scaffold"
    )))
}

#[cfg(feature = "onnx")]
pub mod onnx {
    pub type Session = crate::OrtSession;
}

#[cfg(feature = "tensorrt")]
pub mod tensorrt {
    use crate::MlError;

    #[derive(Clone, Debug, Default)]
    pub struct TensorRtSession;

    impl TensorRtSession {
        pub fn unavailable() -> Result<Self, MlError> {
            Err(MlError::new(
                "TensorRT backend is feature-gated but requires Track 32 GPU contract",
            ))
        }
    }
}

#[cfg(feature = "burn")]
pub mod burn {
    pub type BurnModelSystem = crate::OrtNeuralSystem;
}

#[cfg(feature = "gymnasium")]
pub mod gymnasium {
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct GymSpace {
        pub shape: Vec<usize>,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn tensor_rejects_shape_mismatch() {
        let tensor = Tensor::new(vec![2, 2], vec![1.0, 2.0]);

        assert!(tensor.is_err());
    }

    #[test]
    fn tensor_rejects_zero_dimension() {
        let tensor = Tensor::new(vec![2, 0], Vec::<f32>::new());

        assert_eq!(
            tensor.expect_err("zero dimension should fail").to_string(),
            "tensor shape dimensions must be non-zero"
        );
    }

    #[test]
    fn tensor_rejects_non_finite_values() {
        let tensor = Tensor::new(vec![1], vec![f32::NAN]);

        assert_eq!(
            tensor.expect_err("NaN should fail").to_string(),
            "tensor values must be finite"
        );
    }

    #[test]
    fn metadata_rejects_blank_name_and_empty_shapes() {
        let metadata = ModelMetadata {
            name: " ".to_string(),
            version: "1".to_string(),
            input_shape: vec![1],
            output_shape: vec![1],
        };

        assert_eq!(
            metadata
                .validate()
                .expect_err("blank name should fail")
                .to_string(),
            "model name must not be empty"
        );
    }

    #[test]
    fn ort_session_runs_shape_checked_inference() {
        let session =
            OrtSession::from_bytes("identity", "0", [1, 2, 3], vec![2], vec![2]).expect("session");
        let input = Tensor::new(vec![2], vec![1.0, 2.0]).expect("input");

        let output = session.run(&input).expect("output");

        assert_eq!(output, input);
        assert_eq!(session.model_size_bytes(), 3);
    }

    #[test]
    fn tick_hook_runs_matching_phase_only() {
        let session =
            OrtSession::from_bytes("identity", "0", [1], vec![1], vec![1]).expect("session");
        let system = OrtNeuralSystem::new(
            session,
            TickPhase::BeforeSystems,
            FallbackPolicy::UseOriginalSystem,
        );
        let mut hook = InferenceTickHook::new();
        hook.register(Arc::new(system));

        let input = Tensor::scalar(7.0);

        assert_eq!(
            hook.run_phase(TickPhase::BeforeSystems, &input)
                .expect("before"),
            vec![input.clone()]
        );
        assert!(hook
            .run_phase(TickPhase::AfterSystems, &input)
            .expect("after")
            .is_empty());
    }

    #[test]
    fn tick_hook_rejects_input_shape_before_predict() {
        struct PanickingSystem(ModelMetadata);

        impl NeuralSystem for PanickingSystem {
            fn metadata(&self) -> &ModelMetadata {
                &self.0
            }

            fn tick_phase(&self) -> TickPhase {
                TickPhase::BeforeSystems
            }

            fn fallback_policy(&self) -> FallbackPolicy {
                FallbackPolicy::FailTick
            }

            fn predict(&self, _input: &Tensor) -> Result<Tensor, MlError> {
                panic!("predict must not run after input shape validation fails")
            }
        }

        let mut hook = InferenceTickHook::new();
        hook.try_register(Arc::new(PanickingSystem(ModelMetadata {
            name: "shape-guard".to_string(),
            version: "1".to_string(),
            input_shape: vec![2],
            output_shape: vec![1],
        })))
        .expect("register");

        let error = hook
            .run_phase(TickPhase::BeforeSystems, &Tensor::scalar(1.0))
            .expect_err("wrong input shape should fail");

        assert_eq!(
            error.to_string(),
            "input shape [1] does not match model shape-guard:1 input_shape [2]"
        );
    }

    #[test]
    fn tick_hook_rejects_bad_output_shape_after_predict() {
        struct BadOutputSystem(ModelMetadata);

        impl NeuralSystem for BadOutputSystem {
            fn metadata(&self) -> &ModelMetadata {
                &self.0
            }

            fn tick_phase(&self) -> TickPhase {
                TickPhase::BeforeSystems
            }

            fn fallback_policy(&self) -> FallbackPolicy {
                FallbackPolicy::FailTick
            }

            fn predict(&self, _input: &Tensor) -> Result<Tensor, MlError> {
                Ok(Tensor::scalar(1.0))
            }
        }

        let mut hook = InferenceTickHook::new();
        hook.try_register(Arc::new(BadOutputSystem(ModelMetadata {
            name: "shape-guard".to_string(),
            version: "1".to_string(),
            input_shape: vec![1],
            output_shape: vec![2],
        })))
        .expect("register");

        let error = hook
            .run_phase(TickPhase::BeforeSystems, &Tensor::scalar(1.0))
            .expect_err("wrong output shape should fail");

        assert_eq!(
            error.to_string(),
            "output shape [1] does not match model shape-guard:1 output_shape [2]"
        );
    }

    #[test]
    fn try_register_validates_model_metadata_contract() {
        struct InvalidSystem(ModelMetadata);

        impl NeuralSystem for InvalidSystem {
            fn metadata(&self) -> &ModelMetadata {
                &self.0
            }

            fn tick_phase(&self) -> TickPhase {
                TickPhase::BeforeSystems
            }

            fn fallback_policy(&self) -> FallbackPolicy {
                FallbackPolicy::FailTick
            }

            fn predict(&self, _input: &Tensor) -> Result<Tensor, MlError> {
                Ok(Tensor::scalar(0.0))
            }
        }

        let mut hook = InferenceTickHook::new();
        let error = hook
            .try_register(Arc::new(InvalidSystem(ModelMetadata {
                name: "invalid".to_string(),
                version: "".to_string(),
                input_shape: vec![1],
                output_shape: vec![1],
            })))
            .expect_err("blank version should fail");

        assert_eq!(error.to_string(), "model version must not be empty");
        assert!(hook.is_empty());
    }

    #[test]
    fn try_register_rejects_duplicate_model_name_and_version() {
        let session =
            OrtSession::from_bytes("identity", "1", [1], vec![1], vec![1]).expect("session");
        let system = OrtNeuralSystem::new(
            session.clone(),
            TickPhase::BeforeSystems,
            FallbackPolicy::FailTick,
        );
        let duplicate =
            OrtNeuralSystem::new(session, TickPhase::AfterSystems, FallbackPolicy::FailTick);
        let mut hook = InferenceTickHook::new();

        hook.try_register(Arc::new(system)).expect("register");
        let error = hook
            .try_register(Arc::new(duplicate))
            .expect_err("duplicate model version should fail");

        assert_eq!(error.to_string(), "model identity:1 is already registered");
        assert_eq!(hook.len(), 1);
    }

    #[test]
    fn backend_status_reports_dependency_free_scaffold() {
        let session =
            OrtSession::from_bytes("identity", "0", [1], vec![1], vec![1]).expect("session");

        assert_eq!(
            session.runtime_status(),
            BackendStatus::NotConfigured {
                backend: Backend::OnnxRuntime,
                reason: "ONNX Runtime adapter is not wired in the dependency-free scaffold"
            }
        );
        assert_eq!(
            ensure_backend_configured(Backend::TensorRt)
                .expect_err("backend should not be configured")
                .to_string(),
            "TensorRt backend is not configured in the dependency-free Track 37 scaffold"
        );
    }
}

pub struct ParallelVecEnv {
    pub num_envs: usize,
    pub obs_dim: usize,
    pub action_dim: usize,
    pub num_threads: usize,
}

impl ParallelVecEnv {
    pub fn new(num_envs: usize, obs_dim: usize, action_dim: usize) -> Self {
        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        Self {
            num_envs,
            obs_dim,
            action_dim,
            num_threads,
        }
    }

    pub fn with_threads(
        num_envs: usize,
        obs_dim: usize,
        action_dim: usize,
        num_threads: usize,
    ) -> Self {
        Self {
            num_envs,
            obs_dim,
            action_dim,
            num_threads: num_threads.max(1),
        }
    }

    pub fn num_threads(&self) -> usize {
        self.num_threads
    }

    pub fn reset(&mut self) -> Vec<f32> {
        vec![0.0; self.num_envs * self.obs_dim]
    }

    pub fn step_batch(
        &mut self,
        actions: &[i64],
        out_obs: &mut [f32],
        out_rewards: &mut [f32],
        out_terminated: &mut [u8],
        out_truncated: &mut [u8],
    ) {
        let num_envs = self.num_envs.min(actions.len());
        if num_envs == 0 {
            return;
        }
        let threads = self.num_threads.max(1).min(num_envs);
        let chunk_size = num_envs.div_ceil(threads);

        let obs_dim = self.obs_dim;

        std::thread::scope(|s| {
            let mut obs_chunks = out_obs.chunks_mut(chunk_size * obs_dim);
            let mut rew_chunks = out_rewards.chunks_mut(chunk_size);
            let mut term_chunks = out_terminated.chunks_mut(chunk_size);
            let mut trunc_chunks = out_truncated.chunks_mut(chunk_size);

            for chunk_idx in 0..threads {
                let start_env = chunk_idx * chunk_size;
                if start_env >= num_envs {
                    break;
                }
                let end_env = (start_env + chunk_size).min(num_envs);

                let sub_obs = obs_chunks.next().unwrap_or(&mut []);
                let sub_rew = rew_chunks.next().unwrap_or(&mut []);
                let sub_term = term_chunks.next().unwrap_or(&mut []);
                let sub_trunc = trunc_chunks.next().unwrap_or(&mut []);
                let actions_slice = &actions[start_env..end_env];

                s.spawn(move || {
                    for (local_idx, &action) in actions_slice.iter().enumerate() {
                        let base = local_idx * obs_dim;
                        for d in 0..obs_dim {
                            if base + d < sub_obs.len() {
                                sub_obs[base + d] = (action as f32) + (d as f32) * 0.25;
                            }
                        }
                        if local_idx < sub_rew.len() {
                            sub_rew[local_idx] = 0.1 + (action as f32) * 0.05;
                        }
                        if local_idx < sub_term.len() {
                            sub_term[local_idx] = 0;
                        }
                        if local_idx < sub_trunc.len() {
                            sub_trunc[local_idx] = 0;
                        }
                    }
                });
            }
        });
    }

    pub fn step_with_ort_session(&mut self, _session: &str) {
        // dummy ort session hook
    }
}

#[test]
fn rayon_vecenv_1024_worlds_throughput() {
    let mut env = ParallelVecEnv::new(1024, 4, 2);
    let _obs = env.reset();
    let actions = vec![0i64; 1024];
    let mut out_obs = vec![0.0f32; 1024 * 4];
    let mut out_rewards = vec![0.0f32; 1024];
    let mut out_term = vec![0u8; 1024];
    let mut out_trunc = vec![0u8; 1024];
    env.step_batch(
        &actions,
        &mut out_obs,
        &mut out_rewards,
        &mut out_term,
        &mut out_trunc,
    );
    assert_eq!(out_rewards.len(), 1024);
}

#[test]
fn multicore_vecenv_thread_parity() {
    let mut env1 = ParallelVecEnv::with_threads(1024, 4, 2, 1);
    let mut env8 = ParallelVecEnv::with_threads(1024, 4, 2, 8);

    let actions: Vec<i64> = (0..1024).map(|i| (i % 7) as i64).collect();

    let mut obs1 = vec![0.0f32; 1024 * 4];
    let mut rew1 = vec![0.0f32; 1024];
    let mut term1 = vec![0u8; 1024];
    let mut trunc1 = vec![0u8; 1024];

    let mut obs8 = vec![0.0f32; 1024 * 4];
    let mut rew8 = vec![0.0f32; 1024];
    let mut term8 = vec![0u8; 1024];
    let mut trunc8 = vec![0u8; 1024];

    env1.step_batch(&actions, &mut obs1, &mut rew1, &mut term1, &mut trunc1);
    env8.step_batch(&actions, &mut obs8, &mut rew8, &mut term8, &mut trunc8);

    assert_eq!(obs1, obs8);
    assert_eq!(rew1, rew8);
    assert_eq!(term1, term8);
    assert_eq!(trunc1, trunc8);
    assert_eq!(env1.num_threads(), 1);
    assert_eq!(env8.num_threads(), 8);
}
