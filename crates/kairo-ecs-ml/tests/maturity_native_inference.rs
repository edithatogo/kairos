use kairo_ecs_ml::{
    AffineSurrogate, FallbackPolicy, ModelMetadata, NeuralSystem, OrtSession, Tensor, TickPhase,
};

#[test]
fn an_unavailable_onnx_backend_does_not_return_the_input_as_a_prediction() {
    let session =
        OrtSession::from_bytes("unqualified-model", "0", [1, 2, 3], vec![1], vec![1]).unwrap();
    let error = session.run(&Tensor::scalar(3.0)).unwrap_err();
    assert!(error.to_string().contains("no qualified ONNX Runtime"));
}

fn metadata(name: &str, dimension: usize) -> ModelMetadata {
    ModelMetadata {
        name: name.into(),
        version: "frozen-v1".into(),
        input_shape: vec![dimension],
        output_shape: vec![dimension],
    }
}

#[test]
fn affine_forward_pass_matches_an_independent_nonidentity_result() {
    let provenance = metadata("calibrated-affine", 2);
    let model = AffineSurrogate::new(
        provenance.clone(),
        Tensor::new([2, 2], [2.0, -1.0, 0.5, 3.0]).unwrap(),
        Tensor::new([2], [0.25, -1.0]).unwrap(),
        vec![(-4.0, 4.0); 2],
        TickPhase::AfterSystems,
        FallbackPolicy::FailTick,
    )
    .unwrap();
    assert_eq!(model.metadata(), &provenance);
    assert_eq!(model.fallback_policy(), FallbackPolicy::FailTick);

    let input = Tensor::new([2], [1.5, -2.0]).unwrap();
    let result = model.predict(&input).unwrap();
    assert_eq!(result.values(), &[5.25, -6.25]);
    assert!(model.predict(&Tensor::scalar(1.0)).is_err());
    assert!(model
        .predict(&Tensor::new([2], [4.01, 0.0]).unwrap())
        .is_err());
}

#[test]
fn decay_surrogate_has_bounded_held_out_error_and_rejects_extrapolation() {
    let model = AffineSurrogate::new(
        metadata("unit-decay-secant-0-to-0.4", 1),
        Tensor::new([1, 1], [-0.824_199_86]).unwrap(),
        Tensor::scalar(1.0),
        vec![(0.0, 0.4)],
        TickPhase::BeforeSystems,
        FallbackPolicy::UseOriginalSystem,
    )
    .unwrap();

    let max_error = (1..40)
        .map(|sample| {
            let t = sample as f32 / 100.0;
            let exact = (-f64::from(t)).exp();
            let actual = f64::from(model.predict(&Tensor::scalar(t)).unwrap().values()[0]);
            (actual - exact).abs()
        })
        .fold(0.0, f64::max);
    eprintln!("held-out decay surrogate maximum absolute error {max_error:.6}");
    assert!(max_error <= 0.025);
    assert!(model.predict(&Tensor::scalar(0.4001)).is_err());
    assert!(model.predict(&Tensor::scalar(f32::NAN)).is_err());
}

#[test]
fn malformed_coefficients_and_unrepresentable_output_fail_closed() {
    let construct = |weights, bias, bounds| {
        AffineSurrogate::new(
            metadata("coefficients", 1),
            weights,
            bias,
            bounds,
            TickPhase::BeforeSystems,
            FallbackPolicy::FailTick,
        )
    };
    assert!(Tensor::new([usize::MAX, 2], []).is_err());
    assert!(construct(Tensor::scalar(1.0), Tensor::scalar(0.0), vec![(0.0, 1.0)]).is_err());
    let weights = Tensor::new([1, 1], [f32::MAX]).unwrap();
    assert!(construct(
        weights.clone(),
        Tensor::scalar(f32::INFINITY),
        vec![(0.0, 1.0)]
    )
    .is_err());
    assert!(construct(weights.clone(), Tensor::scalar(0.0), vec![(1.0, 0.0)]).is_err());
    let model = construct(weights, Tensor::scalar(0.0), vec![(0.0, 2.0)]).unwrap();
    assert!(model.predict(&Tensor::scalar(2.0)).is_err());
}
