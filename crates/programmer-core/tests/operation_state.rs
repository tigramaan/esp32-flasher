use programmer_core::{ErrorCode, OperationStage, OperationStateMachine};

#[test]
fn accepts_happy_path() {
    let mut state = OperationStateMachine::new();
    for next in [
        OperationStage::Validating,
        OperationStage::Detecting,
        OperationStage::Connecting,
        OperationStage::Writing,
        OperationStage::Verifying,
        OperationStage::Resetting,
        OperationStage::Monitoring,
        OperationStage::Passed,
        OperationStage::Disconnected,
        OperationStage::Idle,
    ] {
        state.transition(next).unwrap();
    }
}

#[test]
fn rejects_skipping_verification() {
    let mut state = OperationStateMachine::new();
    state.transition(OperationStage::Validating).unwrap();
    let error = state.transition(OperationStage::Writing).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidState);
}

#[test]
fn active_operation_can_fail() {
    let mut state = OperationStateMachine::new();
    state.transition(OperationStage::Validating).unwrap();
    state.fail().unwrap();
    assert_eq!(state.stage(), OperationStage::Failed);
}
