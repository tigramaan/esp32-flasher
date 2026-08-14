#[path = "worker_device.rs"]
mod device;
#[path = "worker_monitor.rs"]
mod monitor;
#[path = "worker_supervisor.rs"]
mod supervisor;

use crate::application::{DetectedDevice, FactoryFlashRequest, OperationProgress, UpdateRequest};
use programmer_core::{ErrorCode, OperationError, OperationStage};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};

pub use monitor::open_monitor_after_flash;
pub use supervisor::execute_supervised;

pub const WORKER_ARGUMENT: &str = "--esp32-flasher-worker-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkerRequest {
    Factory { request: FactoryFlashRequest },
    Update { request: UpdateRequest },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkerEnvelope {
    request: WorkerRequest,
    connect_attempt: usize,
}

impl WorkerEnvelope {
    fn new(request: WorkerRequest, connect_attempt: usize) -> Self {
        Self {
            request,
            connect_attempt,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerMessage {
    Log {
        stage: OperationStage,
        message: String,
    },
    Stage {
        stage: OperationStage,
        message: String,
    },
    Progress {
        progress: OperationProgress,
    },
    MutationStarted,
    Complete {
        result: WorkerResult,
    },
    Failed {
        error: OperationError,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    pub device: DetectedDevice,
    pub monitor_baud: u32,
    pub success_marker: String,
    pub success_timeout_ms: u64,
}

pub fn run_worker_from_stdio() -> i32 {
    let envelope: WorkerEnvelope = match serde_json::from_reader(io::stdin().lock()) {
        Ok(envelope) => envelope,
        Err(error) => {
            emit(&WorkerMessage::Failed {
                error: OperationError::new(
                    ErrorCode::PackageInvalid,
                    "Некорректный запрос flash worker",
                )
                .with_detail(error.to_string()),
            });
            return 2;
        }
    };
    let result = match envelope.request {
        WorkerRequest::Factory { request } => {
            device::run_factory(request, envelope.connect_attempt)
        }
        WorkerRequest::Update { request } => device::run_update(request, envelope.connect_attempt),
    };
    match result {
        Ok(result) => {
            emit(&WorkerMessage::Complete { result });
            0
        }
        Err(error) => {
            emit(&WorkerMessage::Failed { error });
            1
        }
    }
}

fn emit(message: &WorkerMessage) {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    if serde_json::to_writer(&mut lock, message).is_ok() {
        let _ = lock.write_all(b"\n");
        let _ = lock.flush();
    }
}
