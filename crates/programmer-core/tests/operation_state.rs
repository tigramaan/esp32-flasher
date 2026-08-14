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

#[test]
fn every_active_stage_can_be_cancelled() {
    for stage in [
        OperationStage::Validating,
        OperationStage::Detecting,
        OperationStage::Connecting,
        OperationStage::Erasing,
        OperationStage::Writing,
        OperationStage::Verifying,
        OperationStage::Resetting,
        OperationStage::Monitoring,
    ] {
        let mut state = OperationStateMachine::new();
        state.transition(OperationStage::Validating).unwrap();
        let path = match stage {
            OperationStage::Validating => vec![],
            OperationStage::Detecting => vec![OperationStage::Detecting],
            OperationStage::Connecting => {
                vec![OperationStage::Detecting, OperationStage::Connecting]
            }
            OperationStage::Erasing => vec![
                OperationStage::Detecting,
                OperationStage::Connecting,
                OperationStage::Erasing,
            ],
            OperationStage::Writing => vec![
                OperationStage::Detecting,
                OperationStage::Connecting,
                OperationStage::Writing,
            ],
            OperationStage::Verifying => vec![
                OperationStage::Detecting,
                OperationStage::Connecting,
                OperationStage::Writing,
                OperationStage::Verifying,
            ],
            OperationStage::Resetting => vec![
                OperationStage::Detecting,
                OperationStage::Connecting,
                OperationStage::Writing,
                OperationStage::Verifying,
                OperationStage::Resetting,
            ],
            OperationStage::Monitoring => vec![
                OperationStage::Detecting,
                OperationStage::Connecting,
                OperationStage::Writing,
                OperationStage::Verifying,
                OperationStage::Resetting,
                OperationStage::Monitoring,
            ],
            _ => unreachable!(),
        };
        for next in path {
            state.transition(next).unwrap();
        }
        state.transition(OperationStage::Cancelled).unwrap();
        assert_eq!(state.stage(), OperationStage::Cancelled);
    }
}
