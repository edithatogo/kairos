#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;
use std::sync::Arc;

mod affine;
#[cfg(all(feature = "onnx", any(target_os = "linux", target_os = "macos")))]
mod onnx_runtime;
mod vecenv;
pub use affine::AffineSurrogate;
pub use vecenv::{ParallelVecEnv, QueueEnvironment, QueueObservation, QueueStep};

#[derive(Clone, Debug, PartialEq)]
pub struct Tensor {
    shape: Vec<usize>,
    values: Vec<f32>,
}

impl Tensor {
    pub fn new(shape: impl Into<Vec<usize>>, values: impl Into<Vec<f32>>) -> Result<Self, MlError> {
        let shape = shape.into();
        let values = values.into();
        let expected_len = validate_shape("tensor shape", &shape)?;

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

const ONNX_UNAVAILABLE: &str = "no qualified ONNX Runtime is configured";

/// ONNX model metadata, bytes and an optional native CPU session.
#[derive(Clone, Debug)]
pub struct OrtSession {
    metadata: ModelMetadata,
    model_bytes: Arc<[u8]>,
    #[cfg(all(feature = "onnx", any(target_os = "linux", target_os = "macos")))]
    runtime: Option<onnx_runtime::CpuSession>,
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
            model_bytes: model_bytes.into(),
            #[cfg(all(feature = "onnx", any(target_os = "linux", target_os = "macos")))]
            runtime: None,
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

    /// Bind a trusted ONNX model to the application's initialized environment.
    ///
    /// The session uses the CPU provider and retains the environment.
    /// The model must declare one fixed-shape `f32` input and output matching
    /// its metadata.
    #[cfg(all(feature = "onnx", any(target_os = "linux", target_os = "macos")))]
    pub fn with_cpu_runtime(mut self, environment: &onnx::Environment) -> Result<Self, MlError> {
        self.runtime = Some(onnx_runtime::CpuSession::open(
            environment,
            &self.model_bytes,
            &self.metadata,
        )?);
        Ok(self)
    }

    /// Evaluate the model using its configured backend.
    pub fn run(&self, input: &Tensor) -> Result<Tensor, MlError> {
        self.validate_input(input)?;
        #[cfg(all(feature = "onnx", any(target_os = "linux", target_os = "macos")))]
        if let Some(runtime) = &self.runtime {
            return runtime.run(input, &self.metadata.output_shape);
        }
        Err(MlError::new(ONNX_UNAVAILABLE))
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
        #[cfg(all(feature = "onnx", any(target_os = "linux", target_os = "macos")))]
        if self.runtime.is_some() {
            return BackendStatus::Configured {
                backend: Backend::OnnxRuntime,
            };
        }
        BackendStatus::NotConfigured {
            backend: Backend::OnnxRuntime,
            reason: ONNX_UNAVAILABLE,
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

fn validate_shape(label: &str, shape: &[usize]) -> Result<usize, MlError> {
    if shape.is_empty() {
        return Err(MlError::new(format!("{} must not be empty", label)));
    }
    if shape.contains(&0) {
        return Err(MlError::new(format!(
            "{} dimensions must be non-zero",
            label
        )));
    }
    shape
        .iter()
        .try_fold(1_usize, |size, width| size.checked_mul(*width))
        .ok_or_else(|| MlError::new(format!("{label} element count overflows")))
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

/// Check for a process-wide default backend.
///
/// ONNX environments are supplied per session. Use [`OrtSession::runtime_status`]
/// to check whether a specific model is bound to one.
pub fn ensure_backend_configured(backend: Backend) -> Result<(), MlError> {
    Err(MlError::new(format!(
        "{backend:?} backend is not configured in the dependency-free Track 37 scaffold"
    )))
}

#[cfg(feature = "onnx")]
pub mod onnx {
    pub type Session = crate::OrtSession;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub use ort::{environment::Environment, init_from};
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

    fn affine_fixture(phase: TickPhase) -> AffineSurrogate {
        AffineSurrogate::new(
            ModelMetadata {
                name: "affine".into(),
                version: "1".into(),
                input_shape: vec![1],
                output_shape: vec![1],
            },
            Tensor::new([1, 1], [2.0]).unwrap(),
            Tensor::scalar(1.0),
            vec![(0.0, 10.0)],
            phase,
            FallbackPolicy::UseOriginalSystem,
        )
        .unwrap()
    }

    #[test]
    fn tick_hook_runs_matching_phase_only() {
        let mut hook = InferenceTickHook::new();
        hook.register(Arc::new(affine_fixture(TickPhase::BeforeSystems)));

        let input = Tensor::scalar(7.0);
        assert_eq!(
            hook.run_phase(TickPhase::BeforeSystems, &input).unwrap(),
            vec![Tensor::scalar(15.0)]
        );
        assert!(hook
            .run_phase(TickPhase::AfterSystems, &input)
            .unwrap()
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
        let system = affine_fixture(TickPhase::BeforeSystems);
        let duplicate = affine_fixture(TickPhase::AfterSystems);
        let mut hook = InferenceTickHook::new();

        hook.try_register(Arc::new(system)).expect("register");
        let error = hook
            .try_register(Arc::new(duplicate))
            .expect_err("duplicate model version should fail");

        assert_eq!(error.to_string(), "model affine:1 is already registered");
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
                reason: "no qualified ONNX Runtime is configured"
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
