use kairo_ecs_des::FlowError;
use std::error::Error;
#[test]
fn flow_errors_implement_standard_error_without_hidden_sources() {
    let cases = [
        FlowError::InvalidEntity,
        FlowError::InvalidResource,
        FlowError::InvalidRequest,
        FlowError::TerminalRequest,
        FlowError::InvalidLease,
        FlowError::CapacityInUse,
        FlowError::ResourceInUse,
        FlowError::PastCommand,
        FlowError::CounterOverflow,
        FlowError::InvalidState,
        FlowError::InvalidWork,
    ];
    for error in cases {
        let e: &dyn Error = &error;
        assert!(!e.to_string().trim().is_empty());
        assert!(e.source().is_none());
        assert_eq!(error, error.clone());
    }
}
