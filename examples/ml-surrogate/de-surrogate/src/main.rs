use kairo_ecs_ml::{FallbackPolicy, NeuralSystem, OrtNeuralSystem, OrtSession, Tensor, TickPhase};

fn original_decay(value: f32, rate: f32, dt: f32) -> f32 {
    value + (-rate * value) * dt
}

fn main() {
    let input = Tensor::new(vec![2], vec![10.0, 0.1]).expect("input tensor");
    let session =
        OrtSession::from_bytes("decay-surrogate", "scaffold", [1, 2, 3], vec![2], vec![1])
            .expect("session");
    let surrogate = OrtNeuralSystem::new(
        session,
        TickPhase::BeforeSystems,
        FallbackPolicy::UseOriginalSystem,
    );

    let prediction = surrogate.predict(&input).expect("surrogate output");
    let baseline = original_decay(10.0, 0.1, 1.0);
    let absolute_error = (prediction.values()[0] - baseline).abs();

    println!(
        "baseline={baseline:.3} surrogate={:.3} absolute_error={absolute_error:.3}",
        prediction.values()[0]
    );
}
