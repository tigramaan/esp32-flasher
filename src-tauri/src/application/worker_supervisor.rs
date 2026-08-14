use super::{WorkerEnvelope, WorkerMessage, WorkerRequest, WorkerResult, WORKER_ARGUMENT};
use crate::application::coordinator::{cancelled_error, OperationLease};
use crate::application::{AppState, EventSink, OperationEvent};
use programmer_core::{ErrorCode, OperationError, OperationStage, Result};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const CONNECT_ATTEMPTS: usize = 3;
const CONNECT_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(20);
const CONNECT_RETRY_DELAY: Duration = Duration::from_secs(1);
const FLASH_STAGE_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const WORKER_POLL_INTERVAL: Duration = Duration::from_millis(50);
const WORKER_EXIT_GRACE: Duration = Duration::from_secs(2);

pub fn execute_supervised(
    state: &Arc<AppState>,
    lease: &OperationLease,
    request: WorkerRequest,
    sink: &EventSink,
) -> Result<WorkerResult> {
    let mut last_error = None;
    for attempt in 1..=CONNECT_ATTEMPTS {
        lease.check_cancelled()?;
        report_attempt_start(sink, attempt);
        match run_attempt(state, lease, &request, attempt, sink) {
            Ok(result) => return Ok(result),
            Err(failure) => {
                if lease.is_cancelled() || failure.error.code == ErrorCode::OperationCancelled {
                    return Err(cancelled_error());
                }
                let retryable = should_retry(failure.error.code, failure.mutation_started, attempt);
                report_attempt_failure(sink, attempt, &failure.error, retryable);
                if !retryable {
                    return Err(failure.error);
                }
                last_error = Some(failure.error);
                wait_before_retry(lease, sink)?;
            }
        }
    }
    Err(last_error.unwrap_or_else(connect_timeout_error))
}

fn report_attempt_start(sink: &EventSink, attempt: usize) {
    sink.state(
        OperationStage::Connecting,
        format!("Подключение к ROM bootloader — попытка {attempt}/{CONNECT_ATTEMPTS}"),
        None,
    );
    sink.log(
        OperationStage::Connecting,
        "INFO",
        format!(
            "Запуск попытки {attempt}/{CONNECT_ATTEMPTS}; таймаут {} секунд",
            CONNECT_ATTEMPT_TIMEOUT.as_secs()
        ),
        None,
    );
}

fn report_attempt_failure(
    sink: &EventSink,
    attempt: usize,
    error: &OperationError,
    retryable: bool,
) {
    sink.log(
        OperationStage::Connecting,
        if retryable { "WARN" } else { "ERROR" },
        format!(
            "Попытка {attempt}/{CONNECT_ATTEMPTS} завершилась: {}{}",
            error.message,
            error
                .detail
                .as_deref()
                .map(|detail| format!(" — {detail}"))
                .unwrap_or_default()
        ),
        Some(error.code),
    );
}

fn wait_before_retry(lease: &OperationLease, sink: &EventSink) -> Result<()> {
    sink.log(
        OperationStage::Connecting,
        "INFO",
        "Повтор подключения через 1 секунду",
        None,
    );
    let retry_at = Instant::now() + CONNECT_RETRY_DELAY;
    while Instant::now() < retry_at {
        lease.check_cancelled()?;
        thread::sleep(WORKER_POLL_INTERVAL);
    }
    Ok(())
}

struct AttemptFailure {
    error: OperationError,
    mutation_started: bool,
}

fn run_attempt(
    state: &Arc<AppState>,
    lease: &OperationLease,
    request: &WorkerRequest,
    connect_attempt: usize,
    sink: &EventSink,
) -> std::result::Result<WorkerResult, AttemptFailure> {
    let (shared, stdout, stdin) = spawn_worker()?;
    state
        .register_worker(lease.operation_id(), shared.clone())
        .map_err(|error| AttemptFailure {
            error,
            mutation_started: false,
        })?;
    if let Err(error) = write_request(stdin, request, connect_attempt) {
        terminate(&shared);
        state.unregister_worker(lease.operation_id());
        return Err(AttemptFailure {
            error,
            mutation_started: false,
        });
    }
    let message_rx = read_messages(stdout);
    let outcome = supervise_messages(lease, sink, &shared, &message_rx);
    wait_or_terminate(&shared);
    state.unregister_worker(lease.operation_id());
    outcome
}

type WorkerProcess = (
    Arc<Mutex<Child>>,
    std::process::ChildStdout,
    std::process::ChildStdin,
);

fn spawn_worker() -> std::result::Result<WorkerProcess, AttemptFailure> {
    let executable = std::env::current_exe().map_err(|error| AttemptFailure {
        error: OperationError::new(ErrorCode::InternalError, "Не найден portable EXE")
            .with_detail(error.to_string()),
        mutation_started: false,
    })?;
    let mut command = Command::new(executable);
    command
        .arg(WORKER_ARGUMENT)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn().map_err(|error| AttemptFailure {
        error: OperationError::new(
            ErrorCode::InternalError,
            "Не удалось запустить изолированный flash worker",
        )
        .with_detail(error.to_string()),
        mutation_started: false,
    })?;
    let stdin = child.stdin.take().ok_or_else(|| AttemptFailure {
        error: worker_pipe_error("stdin worker недоступен"),
        mutation_started: false,
    })?;
    let stdout = child.stdout.take().ok_or_else(|| AttemptFailure {
        error: worker_pipe_error("stdout worker недоступен"),
        mutation_started: false,
    })?;
    Ok((Arc::new(Mutex::new(child)), stdout, stdin))
}

fn write_request(
    mut stdin: std::process::ChildStdin,
    request: &WorkerRequest,
    connect_attempt: usize,
) -> Result<()> {
    let envelope = WorkerEnvelope::new(request.clone(), connect_attempt);
    serde_json::to_writer(&mut stdin, &envelope)
        .map_err(|error| worker_pipe_error(error.to_string()))?;
    stdin
        .flush()
        .map_err(|error| worker_pipe_error(error.to_string()))
}

fn read_messages(
    stdout: std::process::ChildStdout,
) -> mpsc::Receiver<std::result::Result<WorkerMessage, String>> {
    let (message_tx, message_rx) = mpsc::channel();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let message = line.map_err(|error| error.to_string()).and_then(|line| {
                serde_json::from_str::<WorkerMessage>(&line).map_err(|error| error.to_string())
            });
            if message_tx.send(message).is_err() {
                break;
            }
        }
    });
    message_rx
}

fn supervise_messages(
    lease: &OperationLease,
    sink: &EventSink,
    child: &Arc<Mutex<Child>>,
    messages: &mpsc::Receiver<std::result::Result<WorkerMessage, String>>,
) -> std::result::Result<WorkerResult, AttemptFailure> {
    let started = Instant::now();
    let mut mutation_started_at = None;
    loop {
        let mutation_started = mutation_started_at.is_some();
        let error = timeout_or_cancel_error(lease, started, mutation_started_at);
        if let Some(error) = error {
            terminate(child);
            return Err(AttemptFailure {
                error,
                mutation_started,
            });
        }
        match messages.recv_timeout(WORKER_POLL_INTERVAL) {
            Ok(Ok(WorkerMessage::Log { stage, message })) => sink.log(stage, "INFO", message, None),
            Ok(Ok(WorkerMessage::Stage { stage, message })) => sink.state(stage, message, None),
            Ok(Ok(WorkerMessage::Progress { progress })) => sink.send(OperationEvent::Progress {
                operation_id: sink.operation_id().to_string(),
                progress,
            }),
            Ok(Ok(WorkerMessage::MutationStarted)) => {
                mutation_started_at.get_or_insert_with(Instant::now);
            }
            Ok(Ok(WorkerMessage::Complete { result })) => return Ok(result),
            Ok(Ok(WorkerMessage::Failed { error })) => {
                return Err(AttemptFailure {
                    error,
                    mutation_started,
                });
            }
            Ok(Err(detail)) => {
                return Err(AttemptFailure {
                    error: worker_pipe_error(detail),
                    mutation_started,
                });
            }
            Err(mpsc::RecvTimeoutError::Timeout) if process_exited(child) => {
                return Err(AttemptFailure {
                    error: worker_pipe_error("flash worker завершился без результата"),
                    mutation_started,
                });
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(AttemptFailure {
                    error: worker_pipe_error("канал flash worker закрыт"),
                    mutation_started,
                });
            }
        }
    }
}

fn timeout_or_cancel_error(
    lease: &OperationLease,
    started: Instant,
    mutation_started_at: Option<Instant>,
) -> Option<OperationError> {
    if lease.is_cancelled() {
        return Some(cancelled_error());
    }
    if mutation_started_at.is_none() && started.elapsed() >= CONNECT_ATTEMPT_TIMEOUT {
        return Some(connect_timeout_error());
    }
    if mutation_started_at.is_some_and(|started| started.elapsed() >= FLASH_STAGE_TIMEOUT) {
        return Some(
            OperationError::new(
                ErrorCode::OperationTimeout,
                "Прошивка не завершилась за 5 минут",
            )
            .with_detail(
                "Операция остановлена без повторной попытки; содержимое flash может быть неполным",
            ),
        );
    }
    None
}

fn should_retry(error: ErrorCode, mutation_started: bool, attempt: usize) -> bool {
    !mutation_started
        && attempt < CONNECT_ATTEMPTS
        && matches!(
            error,
            ErrorCode::PortBusy | ErrorCode::FlashConnectFailed | ErrorCode::OperationTimeout
        )
}

fn connect_timeout_error() -> OperationError {
    OperationError::new(
        ErrorCode::OperationTimeout,
        "ESP32 не ответила за 20 секунд",
    )
    .with_detail("Проверьте USB-кабель, питание и автоматический вход платы в bootloader")
    .retryable(true)
}

fn worker_pipe_error(detail: impl Into<String>) -> OperationError {
    OperationError::new(
        ErrorCode::InternalError,
        "Изолированный flash worker завершился аварийно",
    )
    .with_detail(detail)
}

fn process_exited(child: &Arc<Mutex<Child>>) -> bool {
    child
        .lock()
        .expect("flash worker lock poisoned")
        .try_wait()
        .ok()
        .flatten()
        .is_some()
}

fn terminate(child: &Arc<Mutex<Child>>) {
    let mut child = child.lock().expect("flash worker lock poisoned");
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
}

fn wait_or_terminate(child: &Arc<Mutex<Child>>) {
    let deadline = Instant::now() + WORKER_EXIT_GRACE;
    while !process_exited(child) {
        if Instant::now() >= deadline {
            terminate(child);
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        should_retry, CONNECT_ATTEMPTS, CONNECT_ATTEMPT_TIMEOUT, CONNECT_RETRY_DELAY,
        FLASH_STAGE_TIMEOUT,
    };
    use programmer_core::ErrorCode;

    #[test]
    fn connect_retry_budget_is_bounded() {
        assert_eq!(CONNECT_ATTEMPTS, 3);
        assert_eq!(CONNECT_ATTEMPT_TIMEOUT.as_secs(), 20);
        assert_eq!(CONNECT_RETRY_DELAY.as_secs(), 1);
        assert_eq!(FLASH_STAGE_TIMEOUT.as_secs(), 300);
    }

    #[test]
    fn retry_stops_after_mutation_or_last_attempt() {
        assert!(should_retry(ErrorCode::FlashConnectFailed, false, 1));
        assert!(should_retry(ErrorCode::OperationTimeout, false, 2));
        assert!(!should_retry(ErrorCode::FlashConnectFailed, false, 3));
        assert!(!should_retry(ErrorCode::OperationTimeout, true, 1));
        assert!(!should_retry(ErrorCode::ChipMismatch, false, 1));
    }
}
