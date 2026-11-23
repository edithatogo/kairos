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

#[cfg(all(feature = "onnx", any(target_os = "linux", target_os = "macos")))]
mod native_onnx {
    use super::*;
    use kairo_ecs_core::Scheduler;
    use kairo_ecs_ml::{onnx, Backend, BackendStatus, InferenceTickHook, OrtNeuralSystem};
    use kairo_ecs_types::{EventKind, ScheduleRequest, SimTime};
    use std::{path::PathBuf, sync::Arc};

    const MODEL: &[u8] = include_bytes!("data/unit-decay-v1.onnx");

    fn describe(bytes: &[u8], input_shape: Vec<usize>, output_shape: Vec<usize>) -> OrtSession {
        OrtSession::from_bytes(
            "unit-decay",
            "fixed-step-v1",
            bytes,
            input_shape,
            output_shape,
        )
        .unwrap()
    }

    fn alternate_operator(operator: &[u8; 3]) -> Vec<u8> {
        let mut bytes = MODEL.to_vec();
        let offset = bytes.windows(3).position(|bytes| bytes == b"Mul").unwrap();
        bytes[offset..offset + 3].copy_from_slice(operator);
        bytes
    }

    #[test]
    #[ignore = "requires KAIRO_TEST_ONNX_LIBRARY pointing to an installed ONNX Runtime library"]
    fn installed_onnx_replays_a_thousand_ticks_and_rejects_invalid_models() {
        assert!(onnx::init_from(std::env::current_exe().unwrap()).is_err());
        let library = PathBuf::from(std::env::var_os("KAIRO_TEST_ONNX_LIBRARY").unwrap());
        assert!(library.is_absolute());
        let environment = onnx::init_from(&library)
            .unwrap()
            .with_telemetry(false)
            .build()
            .unwrap();
        let open = |bytes: &[u8]| describe(bytes, vec![1], vec![1]).with_cpu_runtime(&environment);

        assert!(open(b"invalid ONNX").is_err());
        assert!(open(&alternate_operator(b"Bad")).is_err());
        assert!(describe(MODEL, vec![2], vec![1])
            .with_cpu_runtime(&environment)
            .is_err());
        assert!(describe(MODEL, vec![1], vec![2])
            .with_cpu_runtime(&environment)
            .is_err());
        for mismatch in [
            include_bytes!("data/float64-port.onnx").as_slice(),
            include_bytes!("data/symbolic-port.onnx").as_slice(),
        ] {
            assert_eq!(
                open(mismatch).unwrap_err().to_string(),
                "ONNX model must have one float32 input with fixed shape [1]"
            );
        }

        let session = open(MODEL).unwrap();
        assert_eq!(session.model_size_bytes(), MODEL.len());
        assert_eq!(
            session.runtime_status(),
            BackendStatus::Configured {
                backend: Backend::OnnxRuntime
            }
        );
        assert!(session.run(&Tensor::scalar(f32::NAN)).is_err());
        let divide = open(&alternate_operator(b"Div")).unwrap();
        assert!(divide
            .run(&Tensor::scalar(f32::MAX))
            .unwrap_err()
            .to_string()
            .contains("finite"));
        drop(divide);
        drop(environment);
        let retained = session.run(&Tensor::scalar(2.0)).unwrap();

        let mut hook = InferenceTickHook::new();
        hook.try_register(Arc::new(OrtNeuralSystem::new(
            session.clone(),
            TickPhase::AfterSystems,
            FallbackPolicy::FailTick,
        )))
        .unwrap();
        let mut scheduler = Scheduler::new();
        for tick in 1..=1000 {
            scheduler.schedule(ScheduleRequest {
                at: SimTime::from_ticks(tick),
                priority: 0,
                entity: None,
                kind: EventKind::custom(1),
            });
        }
        let mut state = Tensor::scalar(1.0);
        while let Some(event) = scheduler.step().into_dispatched() {
            state = hook
                .run_phase(TickPhase::AfterSystems, &state)
                .unwrap()
                .pop()
                .unwrap();
            let exact = (-(event.at.ticks() as f64) / 1000.0).exp();
            assert!((f64::from(state.values()[0]) / exact - 1.0).abs() < 0.0001);
        }
        assert_eq!(scheduler.stats().dispatched_events, 1000);
        eprintln!("ONNX decay tick 1000 result {:.9}", state.values()[0]);

        std::thread::scope(|scope| {
            for val in [1.0f32, 2.0, -3.0, 0.0] {
                let session = session.clone();
                scope.spawn(move || {
                    let exact = f64::from(val) * (-0.001_f64).exp();
                    for _ in 0..32 {
                        let actual = session.run(&Tensor::scalar(val)).unwrap().values()[0];
                        assert!((f64::from(actual) - exact).abs() < 1e-6);
                    }
                });
            }
        });
        drop(hook);
        drop(session);
        assert!((f64::from(retained.values()[0]) - 2.0 * (-0.001_f64).exp()).abs() < 1e-6);
    }
}
