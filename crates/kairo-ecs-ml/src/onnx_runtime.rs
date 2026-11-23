use crate::{validate_finite_values, MlError, ModelMetadata, Tensor};
use ort::environment::Environment;
use ort::session::builder::AutoDevicePolicy;
use ort::session::Session;
use ort::value::{Outlet, TensorElementType, TensorRef, ValueType};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub(super) struct CpuSession(Arc<Mutex<Session>>);

impl CpuSession {
    pub(super) fn open(
        environment: &Environment,
        bytes: &[u8],
        metadata: &ModelMetadata,
    ) -> Result<Self, MlError> {
        let session = Session::builder(environment)
            .and_then(|builder| {
                Ok(builder
                    .with_auto_device(AutoDevicePolicy::PreferCPU)?
                    .with_no_environment_execution_providers()?
                    .with_independent_thread_pool()?
                    .with_intra_threads(1)?)
            })
            .and_then(|mut builder| builder.commit_from_memory(bytes))
            .map_err(|error| MlError::new(format!("failed to load ONNX model: {error}")))?;
        validate_port("input", session.inputs(), &metadata.input_shape)?;
        validate_port("output", session.outputs(), &metadata.output_shape)?;
        Ok(Self(Arc::new(Mutex::new(session))))
    }

    pub(super) fn run(&self, input: &Tensor, output_shape: &[usize]) -> Result<Tensor, MlError> {
        validate_finite_values("ONNX input", input.values())?;
        let input = TensorRef::from_array_view((input.shape(), input.values()))
            .map_err(|error| MlError::new(format!("invalid ONNX input tensor: {error}")))?;
        let mut session = self
            .0
            .lock()
            .map_err(|_| MlError::new("ONNX inference lock is poisoned"))?;
        let outputs = session
            .run(ort::inputs![input])
            .map_err(|error| MlError::new(format!("ONNX inference failed: {error}")))?;
        if outputs.len() != 1 {
            return Err(MlError::new("ONNX inference must return one output tensor"));
        }
        let (shape, values) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|error| MlError::new(format!("invalid ONNX output tensor: {error}")))?;
        if !matches_shape(shape, output_shape) {
            return Err(MlError::new(
                "ONNX output shape does not match model metadata",
            ));
        }
        Tensor::new(output_shape, values)
    }
}

fn validate_port(label: &str, ports: &[Outlet], expected: &[usize]) -> Result<(), MlError> {
    if let [port] = ports {
        if let ValueType::Tensor {
            ty: TensorElementType::Float32,
            shape,
            ..
        } = port.dtype()
        {
            if matches_shape(shape, expected) {
                return Ok(());
            }
        }
    }
    Err(MlError::new(format!(
        "ONNX model must have one float32 {label} with fixed shape {expected:?}"
    )))
}

fn matches_shape(actual: &[i64], expected: &[usize]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(&actual, &expected)| usize::try_from(actual) == Ok(expected))
}
