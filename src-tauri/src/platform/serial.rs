use crate::application::{PortCandidate, SerialData};
use base64::Engine;
use programmer_core::{ErrorCode, MarkerDetector, OperationError, Result};
use serialport::{SerialPort, SerialPortInfo, SerialPortType};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const USB_SERIAL_JTAG_PID: u16 = 0x1001;
const MAX_SERIAL_WRITE: usize = 4096;
const MONITOR_STOP_TIMEOUT: Duration = Duration::from_secs(2);
const MONITOR_RESET_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorStartMode {
    Passive,
    ResetToNormalBoot,
}

#[derive(Debug, Clone)]
pub enum MonitorSignal {
    MarkerMatched,
    Disconnected(String),
}

pub struct MonitorHandle {
    shutdown: mpsc::Sender<()>,
    commands: mpsc::Sender<MonitorCommand>,
    connected: Arc<AtomicBool>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}

enum MonitorCommand {
    Write(Vec<u8>),
    Reset(mpsc::SyncSender<std::result::Result<(), String>>),
}

struct MonitorRuntime<'a> {
    shutdown: &'a mpsc::Receiver<()>,
    commands: &'a mpsc::Receiver<MonitorCommand>,
    signals: &'a mpsc::Sender<MonitorSignal>,
    on_data: &'a Arc<dyn Fn(SerialData) + Send + Sync>,
}

impl MonitorHandle {
    pub fn send(&self, bytes: Vec<u8>) -> Result<()> {
        if bytes.len() > MAX_SERIAL_WRITE {
            return Err(OperationError::new(
                ErrorCode::PackageInvalid,
                "UART-команда превышает 4096 байт",
            ));
        }
        if !self.is_connected() {
            return Err(OperationError::new(
                ErrorCode::DeviceDisconnected,
                "UART-монитор не подключён",
            ));
        }
        self.commands
            .send(MonitorCommand::Write(bytes))
            .map_err(|error| {
                OperationError::new(
                    ErrorCode::DeviceDisconnected,
                    "Не удалось отправить UART-команду",
                )
                .with_detail(error.to_string())
            })
    }

    pub fn stop(&self) {
        let _ = self.shutdown.send(());
    }

    pub fn stop_and_wait(&self) -> Result<()> {
        self.stop();
        let deadline = Instant::now() + MONITOR_STOP_TIMEOUT;
        loop {
            let finished = self
                .worker
                .lock()
                .expect("monitor worker lock poisoned")
                .as_ref()
                .is_none_or(thread::JoinHandle::is_finished);
            if finished {
                break;
            }
            if Instant::now() >= deadline {
                return Err(OperationError::new(
                    ErrorCode::OperationInProgress,
                    "UART-монитор не освободил COM-порт за 2 секунды",
                )
                .retryable(true));
            }
            thread::sleep(Duration::from_millis(10));
        }
        if let Some(worker) = self
            .worker
            .lock()
            .expect("monitor worker lock poisoned")
            .take()
        {
            worker.join().map_err(|_| {
                OperationError::new(
                    ErrorCode::InternalError,
                    "Поток UART-монитора аварийно завершился",
                )
            })?;
        }
        Ok(())
    }

    pub fn reset(&self) -> Result<()> {
        if !self.is_connected() {
            return Err(OperationError::new(
                ErrorCode::DeviceDisconnected,
                "UART-монитор не подключён",
            ));
        }
        let (result_tx, result_rx) = mpsc::sync_channel(1);
        self.commands
            .send(MonitorCommand::Reset(result_tx))
            .map_err(|error| {
                OperationError::new(
                    ErrorCode::DeviceDisconnected,
                    "Не удалось запросить перезапуск ESP32",
                )
                .with_detail(error.to_string())
            })?;
        result_rx
            .recv_timeout(MONITOR_RESET_TIMEOUT)
            .map_err(|error| {
                OperationError::new(
                    ErrorCode::DeviceDisconnected,
                    "UART-монитор не подтвердил перезапуск ESP32",
                )
                .with_detail(error.to_string())
            })?
            .map_err(|detail| {
                OperationError::new(
                    ErrorCode::FlashConnectFailed,
                    "Не удалось перезапустить ESP32",
                )
                .with_detail(detail)
            })
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }
}

impl Drop for MonitorHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn list_ports() -> Result<Vec<PortCandidate>> {
    let mut ports: Vec<_> = serialport::available_ports()
        .map_err(|error| {
            OperationError::new(
                ErrorCode::DeviceNotFound,
                "Не удалось получить список COM-портов",
            )
            .with_detail(error.to_string())
        })?
        .into_iter()
        .map(port_candidate)
        .collect();
    ports.sort_by_key(|port| (!port.known_bridge, natural_port_key(&port.port)));
    Ok(ports)
}

pub fn start_monitor(
    port_name: &str,
    baud: u32,
    marker: &str,
    start_mode: MonitorStartMode,
    on_data: Arc<dyn Fn(SerialData) + Send + Sync>,
    on_disconnect: Arc<dyn Fn(String) + Send + Sync>,
) -> Result<(MonitorHandle, mpsc::Receiver<MonitorSignal>)> {
    validate_monitor_baud(baud)?;
    let info = find_port(port_name)?;
    let pid = match &info.port_type {
        SerialPortType::UsbPort(usb) => usb.pid,
        _ => 0,
    };
    let mut port = serialport::new(port_name, baud)
        .timeout(Duration::from_millis(50))
        .open()
        .map_err(|error| {
            OperationError::new(ErrorCode::PortBusy, "Не удалось открыть UART-монитор")
                .with_detail(error.to_string())
        })?;
    if start_mode == MonitorStartMode::Passive {
        release_control_lines(&mut *port).map_err(|error| {
            OperationError::new(
                ErrorCode::FlashConnectFailed,
                "Не удалось подготовить UART без перезапуска платы",
            )
            .with_detail(error.to_string())
        })?;
    }
    let mut detector = if marker.is_empty() {
        None
    } else {
        Some(MarkerDetector::new(marker.as_bytes())?)
    };
    let (shutdown_tx, shutdown_rx) = mpsc::channel();
    let (command_tx, command_rx) = mpsc::channel::<MonitorCommand>();
    let (signal_tx, signal_rx) = mpsc::channel();
    let connected = Arc::new(AtomicBool::new(true));
    let thread_connected = connected.clone();

    let worker = thread::spawn(move || {
        let result = run_monitor(
            &mut *port,
            pid,
            start_mode,
            &mut detector,
            MonitorRuntime {
                shutdown: &shutdown_rx,
                commands: &command_rx,
                signals: &signal_tx,
                on_data: &on_data,
            },
        );
        drop(port);
        thread_connected.store(false, Ordering::Release);
        if let Err(message) = result {
            let _ = signal_tx.send(MonitorSignal::Disconnected(message.clone()));
            on_disconnect(message);
        }
    });

    Ok((
        MonitorHandle {
            shutdown: shutdown_tx,
            commands: command_tx,
            connected,
            worker: Mutex::new(Some(worker)),
        },
        signal_rx,
    ))
}

pub fn validate_monitor_baud(baud: u32) -> Result<()> {
    if matches!(baud, 9_600 | 57_600 | 115_200 | 230_400 | 460_800 | 921_600) {
        return Ok(());
    }
    Err(OperationError::new(
        ErrorCode::PackageInvalid,
        "Неподдерживаемая скорость UART-монитора",
    )
    .with_detail(baud.to_string()))
}

pub(crate) fn reset_port_for_normal_boot(port_name: &str) -> Result<()> {
    let info = find_port(port_name)?;
    let pid = match info.port_type {
        SerialPortType::UsbPort(usb) => usb.pid,
        _ => 0,
    };
    let mut port = serialport::new(port_name, 115_200)
        .timeout(Duration::from_millis(500))
        .open()
        .map_err(|error| {
            OperationError::new(
                ErrorCode::PortBusy,
                "Не удалось повторно открыть COM-порт для выхода из reset",
            )
            .with_detail(error.to_string())
        })?;
    reset_for_normal_boot(&mut *port, pid).map_err(|error| {
        OperationError::new(
            ErrorCode::FlashConnectFailed,
            "Не удалось вернуть ESP32 в нормальный режим",
        )
        .with_detail(error.to_string())
    })
}

fn run_monitor(
    port: &mut dyn SerialPort,
    pid: u16,
    start_mode: MonitorStartMode,
    detector: &mut Option<MarkerDetector>,
    runtime: MonitorRuntime<'_>,
) -> std::result::Result<(), String> {
    if start_mode == MonitorStartMode::ResetToNormalBoot {
        reset_for_normal_boot(port, pid).map_err(|error| error.to_string())?;
    }
    let mut marker_reported = false;
    let mut buffer = [0_u8; 4096];
    loop {
        if runtime.shutdown.try_recv().is_ok() {
            return Ok(());
        }
        while let Ok(command) = runtime.commands.try_recv() {
            match command {
                MonitorCommand::Write(bytes) => {
                    port.write_all(&bytes).map_err(|error| error.to_string())?;
                    port.flush().map_err(|error| error.to_string())?;
                }
                MonitorCommand::Reset(result) => {
                    let outcome =
                        reset_for_normal_boot(port, pid).map_err(|error| error.to_string());
                    let _ = result.send(outcome);
                }
            }
        }
        match port.read(&mut buffer) {
            Ok(0) => {}
            Ok(count) => {
                let bytes = &buffer[..count];
                (runtime.on_data)(SerialData {
                    text: String::from_utf8_lossy(bytes).into_owned(),
                    base64: base64::engine::general_purpose::STANDARD.encode(bytes),
                });
                if !marker_reported
                    && detector
                        .as_mut()
                        .is_some_and(|detector| detector.feed(bytes))
                {
                    marker_reported = true;
                    let _ = runtime.signals.send(MonitorSignal::MarkerMatched);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn release_control_lines(port: &mut dyn SerialPort) -> std::result::Result<(), serialport::Error> {
    port.write_data_terminal_ready(false)?;
    port.write_request_to_send(false)
}

fn reset_for_normal_boot(
    port: &mut dyn SerialPort,
    pid: u16,
) -> std::result::Result<(), serialport::Error> {
    thread::sleep(Duration::from_millis(100));
    if pid == USB_SERIAL_JTAG_PID {
        port.write_data_terminal_ready(false)?;
        thread::sleep(Duration::from_millis(100));
        port.write_request_to_send(true)?;
        port.write_data_terminal_ready(false)?;
        thread::sleep(Duration::from_millis(100));
        port.write_request_to_send(false)?;
    } else {
        port.write_data_terminal_ready(false)?;
        port.write_request_to_send(true)?;
        thread::sleep(Duration::from_millis(100));
        port.write_request_to_send(false)?;
    }
    Ok(())
}

fn find_port(port_name: &str) -> Result<SerialPortInfo> {
    serialport::available_ports()
        .map_err(|error| {
            OperationError::new(
                ErrorCode::DeviceNotFound,
                "Не удалось получить список COM-портов",
            )
            .with_detail(error.to_string())
        })?
        .into_iter()
        .find(|port| port.port_name.eq_ignore_ascii_case(port_name))
        .ok_or_else(|| {
            OperationError::new(ErrorCode::DeviceNotFound, "COM-порт не найден")
                .with_detail(port_name)
        })
}

fn port_candidate(info: SerialPortInfo) -> PortCandidate {
    let (description, vid, pid, serial_number) = match info.port_type {
        SerialPortType::UsbPort(usb) => {
            let description = usb
                .product
                .or(usb.manufacturer)
                .unwrap_or_else(|| "USB Serial".to_string());
            (description, Some(usb.vid), Some(usb.pid), usb.serial_number)
        }
        SerialPortType::BluetoothPort => ("Bluetooth".to_string(), None, None, None),
        SerialPortType::PciPort => ("PCI Serial".to_string(), None, None, None),
        SerialPortType::Unknown => ("Serial port".to_string(), None, None, None),
    };
    let known_bridge = vid.is_some_and(|value| matches!(value, 0x303A | 0x10C4 | 0x1A86 | 0x0403));
    PortCandidate {
        port: info.port_name,
        description,
        vid,
        pid,
        serial_number,
        known_bridge,
    }
}

fn natural_port_key(port: &str) -> (String, u32) {
    let prefix: String = port
        .chars()
        .take_while(|value| !value.is_ascii_digit())
        .collect();
    let number = port
        .chars()
        .skip_while(|value| !value.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(u32::MAX);
    (prefix, number)
}

#[cfg(test)]
mod tests {
    use super::validate_monitor_baud;
    use programmer_core::ErrorCode;

    #[test]
    fn accepts_ui_baud_rates_and_rejects_unbounded_values() {
        for baud in [9_600, 57_600, 115_200, 230_400, 460_800, 921_600] {
            validate_monitor_baud(baud).unwrap();
        }
        let error = validate_monitor_baud(0).unwrap_err();
        assert_eq!(error.code, ErrorCode::PackageInvalid);
    }
}
