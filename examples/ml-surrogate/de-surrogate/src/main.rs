use kairo_ecs_ml::{
    AffineSurrogate, FallbackPolicy, ModelMetadata, NeuralSystem, Tensor, TickPhase,
};

fn main() {
    // The secant through exact e^(-t) at t=0 and 0.4. Held-out t=0.17.
    let dt = 0.17_f32;
    let surrogate = AffineSurrogate::new(
        ModelMetadata {
            name: "unit-rate-decay".into(),
            version: "secant-0-to-0.4-v1".into(),
            input_shape: vec![1],
            output_shape: vec![1],
        },
        Tensor::new([1, 1], [-0.824_199_86]).expect("weights"),
        Tensor::scalar(1.0),
        vec![(0.0, 0.4)],
        TickPhase::BeforeSystems,
        FallbackPolicy::UseOriginalSystem,
    )
    .expect("surrogate");
    let prediction = surrogate.predict(&Tensor::scalar(dt)).expect("prediction");
    let baseline = (-dt).exp();
    let absolute_error = (prediction.values()[0] - baseline).abs();

    println!(
        "baseline={baseline:.4} surrogate={:.4} absolute_error={absolute_error:.4}",
        prediction.values()[0]
    );
}
