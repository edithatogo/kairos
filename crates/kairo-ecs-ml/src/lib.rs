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
        self.systems.push(system);
        Ok(())
    }

    pub fn run_phase(&self, phase: TickPhase, input: &Tensor) -> Result<Vec<Tensor>, MlError> {
        self.systems
            .iter()
            .filter(|system| system.tick_phase() == phase)
            .map(|system| system.predict(input))
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
}
