use std::sync::Arc;

use kairo_ecs_ml::{
    ensure_backend_configured, Backend, BackendStatus, FallbackPolicy, InferenceTickHook,
    OrtNeuralSystem, OrtSession, Tensor, TickPhase,
};

#[test]
fn featureless_tick_hook_runs_registered_model() {
    let session = OrtSession::from_bytes("identity", "0", [1], vec![1], vec![1]).expect("session");
    let system = OrtNeuralSystem::new(
        session,
        TickPhase::BeforeSystems,
        FallbackPolicy::UseOriginalSystem,
    );
    let mut hook = InferenceTickHook::new();
    hook.try_register(Arc::new(system)).expect("register");

    let output = hook
        .run_phase(TickPhase::BeforeSystems, &Tensor::scalar(3.0))
        .expect("inference");

    assert_eq!(output[0].values(), &[3.0]);
}

#[test]
fn featureless_session_rejects_input_shape_mismatch_before_inference() {
    let session = OrtSession::from_bytes("identity", "0", [1], vec![2], vec![2]).expect("session");
    let input = Tensor::scalar(3.0);

    let error = session
        .validate_input(&input)
        .expect_err("shape mismatch should fail");

    assert_eq!(
        error.to_string(),
        "input shape [1] does not match model shape [2]"
    );
}

#[test]
fn featureless_session_reports_backend_not_configured() {
    let session = OrtSession::from_bytes("identity", "0", [1], vec![1], vec![1]).expect("session");

    assert_eq!(
        session.runtime_status(),
        BackendStatus::NotConfigured {
            backend: Backend::OnnxRuntime,
            reason: "ONNX Runtime adapter is not wired in the dependency-free scaffold"
        }
    );
    assert!(ensure_backend_configured(Backend::Burn).is_err());
}

#[cfg(feature = "onnx")]
#[test]
fn onnx_feature_exposes_session_alias() {
    let session = kairo_ecs_ml::onnx::Session::from_bytes("identity", "0", [1], vec![1], vec![1])
        .expect("session");

    assert_eq!(session.model_size_bytes(), 1);
}

#[cfg(feature = "tensorrt")]
#[test]
fn tensorrt_feature_is_explicitly_contract_blocked() {
    let session = kairo_ecs_ml::tensorrt::TensorRtSession::unavailable();

    assert!(session.is_err());
}

#[cfg(feature = "burn")]
#[test]
fn burn_feature_exposes_native_system_alias() {
    let session = OrtSession::from_bytes("identity", "0", [1], vec![1], vec![1]).expect("session");
    let _system = kairo_ecs_ml::burn::BurnModelSystem::new(
        session,
        TickPhase::BeforeSystems,
        FallbackPolicy::UseOriginalSystem,
    );
}

#[cfg(feature = "gymnasium")]
#[test]
fn gymnasium_feature_exposes_space_contract() {
    let space = kairo_ecs_ml::gymnasium::GymSpace { shape: vec![4] };

    assert_eq!(space.shape, vec![4]);
}
