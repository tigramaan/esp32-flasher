use crate::error::{ErrorCode, OperationError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStage {
    Idle,
    Validating,
    Detecting,
    Connecting,
    Erasing,
    Writing,
    Verifying,
    Resetting,
    Monitoring,
    Passed,
    Failed,
    Disconnected,
}

impl OperationStage {
    pub const fn is_active(self) -> bool {
        matches!(
            self,
            Self::Validating
                | Self::Detecting
                | Self::Connecting
                | Self::Erasing
                | Self::Writing
                | Self::Verifying
                | Self::Resetting
                | Self::Monitoring
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationStateMachine {
    stage: OperationStage,
}

impl Default for OperationStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationStateMachine {
    pub const fn new() -> Self {
        Self {
            stage: OperationStage::Idle,
        }
    }

    pub const fn stage(&self) -> OperationStage {
        self.stage
    }

    pub fn transition(&mut self, next: OperationStage) -> Result<()> {
        if is_allowed(self.stage, next) {
            self.stage = next;
            Ok(())
        } else {
            Err(OperationError::new(
                ErrorCode::InvalidState,
                "Недопустимый переход состояния операции",
            )
            .with_detail(format!("{:?} -> {:?}", self.stage, next)))
        }
    }

    pub fn fail(&mut self) -> Result<()> {
        if self.stage.is_active() {
            self.stage = OperationStage::Failed;
            Ok(())
        } else {
            Err(OperationError::new(
                ErrorCode::InvalidState,
                "Неактивную операцию нельзя завершить ошибкой",
            ))
        }
    }
}

fn is_allowed(current: OperationStage, next: OperationStage) -> bool {
    use OperationStage::*;
    matches!(
        (current, next),
        (Idle, Validating)
            | (Validating, Detecting)
            | (Detecting, Connecting)
            | (Connecting, Erasing)
            | (Connecting, Writing)
            | (Erasing, Writing)
            | (Writing, Verifying)
            | (Verifying, Resetting)
            | (Resetting, Monitoring)
            | (Monitoring, Passed)
            | (Monitoring, Failed)
            | (Passed, Disconnected)
            | (Failed, Disconnected)
            | (Disconnected, Idle)
    )
}
