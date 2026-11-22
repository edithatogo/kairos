use super::{
    validate_finite_values, validate_model_input, validate_shape, FallbackPolicy, MlError,
    ModelMetadata, NeuralSystem, Tensor, TickPhase,
};

/// Rust-native affine surrogate for a versioned model and a bounded input domain.
///
/// Weights are row-major with one row per flattened output coordinate. Bounds
/// list each flattened input coordinate's inclusive calibration range.
#[derive(Clone, Debug)]
pub struct AffineSurrogate {
    metadata: ModelMetadata,
    weights: Tensor,
    bias: Tensor,
    bounds: Vec<(f32, f32)>,
    phase: TickPhase,
    fallback: FallbackPolicy,
}

impl AffineSurrogate {
    pub fn new(
        metadata: ModelMetadata,
        weights: Tensor,
        bias: Tensor,
        bounds: Vec<(f32, f32)>,
        phase: TickPhase,
        fallback: FallbackPolicy,
    ) -> Result<Self, MlError> {
        metadata.validate()?;
        let inputs = validate_shape("model input_shape", &metadata.input_shape)?;
        let outputs = validate_shape("model output_shape", &metadata.output_shape)?;
        if weights.shape() != [outputs, inputs] || bias.shape() != [outputs] {
            return Err(MlError::new(
                "affine coefficient shapes must match the model input and output sizes",
            ));
        }
        validate_finite_values("affine weights", weights.values())?;
        validate_finite_values("affine bias", bias.values())?;
        if bounds.len() != inputs
            || bounds
                .iter()
                .any(|(low, high)| !low.is_finite() || !high.is_finite() || low > high)
        {
            return Err(MlError::new(
                "affine model needs one finite, ordered bound per input coordinate",
            ));
        }
        Ok(Self {
            metadata,
            weights,
            bias,
            bounds,
            phase,
            fallback,
        })
    }
}

impl NeuralSystem for AffineSurrogate {
    fn metadata(&self) -> &ModelMetadata {
        &self.metadata
    }

    fn tick_phase(&self) -> TickPhase {
        self.phase
    }

    fn fallback_policy(&self) -> FallbackPolicy {
        self.fallback.clone()
    }

    fn predict(&self, input: &Tensor) -> Result<Tensor, MlError> {
        validate_model_input(&self.metadata, input)?;
        validate_finite_values("affine inputs", input.values())?;
        for (i, (value, (low, high))) in input.values().iter().zip(&self.bounds).enumerate() {
            if !(low..=high).contains(&value) {
                return Err(MlError::new(format!(
                    "affine input coordinate {i} is outside its calibration domain"
                )));
            }
        }

        let output: Vec<_> = self
            .weights
            .values()
            .chunks_exact(input.values().len())
            .zip(self.bias.values())
            .map(|(row, &bias)| {
                row.iter()
                    .zip(input.values())
                    .fold(f64::from(bias), |sum, (&w, &value)| {
                        sum + f64::from(w) * f64::from(value)
                    }) as f32
            })
            .collect();
        Tensor::new(self.metadata.output_shape.clone(), output)
    }
}
