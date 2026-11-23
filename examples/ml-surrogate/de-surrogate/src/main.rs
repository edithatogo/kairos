use kairo_ecs_ml::{
    AffineSurrogate, FallbackPolicy, ModelMetadata, NeuralSystem, Tensor, TickPhase,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(all(feature = "onnx", any(target_os = "linux", target_os = "macos")))]
    if let Some(library) = std::env::args_os().nth(1) {
        let library = std::path::PathBuf::from(library);
        if !library.is_absolute() {
            return Err("supply an absolute path to the ONNX Runtime library".into());
        }
        let model = std::env::args_os()
            .nth(2)
            .ok_or("supply an ONNX model path")?;
        let environment = kairo_ecs_ml::onnx::init_from(library)?
            .with_telemetry(false)
            .build()?;
        let session = kairo_ecs_ml::OrtSession::from_file(model, vec![1], vec![1])?
            .with_cpu_runtime(&environment)?;
        let mut state = Tensor::scalar(1.0);
        for _ in 0..1000 {
            state = session.run(&state)?;
        }
        println!(
            "onnx={:.9} analytical={:.9}",
            state.values()[0],
            (-1.0_f64).exp()
        );
        return Ok(());
    }

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
    Ok(())
}
