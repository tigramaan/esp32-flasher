use crate::application::{DetectedDevice, OperationProgress};
use crate::platform::serial::reset_port_for_normal_boot;
use espflash::connection::{Connection, Port, ResetAfterOperation, ResetBeforeOperation};
use espflash::flasher::Flasher;
use espflash::image_format::Segment;
use espflash::target::ProgressCallbacks;
use programmer_core::{
    parse_partition_table, ChipFamily, ErrorCode, OperationError, OperationStage, PartitionEntry,
    Result, FLASH_BAUD,
};
use serialport::{SerialPort, SerialPortType};
use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tempfile::NamedTempFile;

const RESET_AFTER_OPERATION: ResetAfterOperation = ResetAfterOperation::HardReset;
const ESPTOOL_RESET_DELAY: Duration = Duration::from_millis(50);
const ESPTOOL_EXTENDED_RESET_DELAY: Duration = Duration::from_millis(550);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectResetProfile {
    EsptoolClassic { release_delay: Duration },
    EspflashFallback,
}

fn connect_reset_profile(attempt: usize) -> ConnectResetProfile {
    match attempt {
        1 => ConnectResetProfile::EsptoolClassic {
            release_delay: ESPTOOL_RESET_DELAY,
        },
        2 => ConnectResetProfile::EsptoolClassic {
            release_delay: ESPTOOL_EXTENDED_RESET_DELAY,
        },
        _ => ConnectResetProfile::EspflashFallback,
    }
}

fn esptool_classic_reset(
    serial: &mut Port,
    release_delay: Duration,
) -> std::result::Result<(), serialport::Error> {
    // Match esptool.py on Windows, including its DTR refresh after every RTS change.
    serial.write_data_terminal_ready(false)?; // IO0 = HIGH
    set_rts_with_dtr_refresh(serial, true, false)?; // EN = LOW
    sleep(Duration::from_millis(100));
    serial.write_data_terminal_ready(true)?; // IO0 = LOW
    set_rts_with_dtr_refresh(serial, false, true)?; // EN = HIGH
    sleep(release_delay);
    serial.write_data_terminal_ready(false)?; // IO0 = HIGH
    Ok(())
}

fn set_rts_with_dtr_refresh(
    serial: &mut Port,
    rts: bool,
    current_dtr: bool,
) -> std::result::Result<(), serialport::Error> {
    serial.write_request_to_send(rts)?;
    serial.write_data_terminal_ready(current_dtr)
}

#[derive(Debug, Clone)]
pub struct OwnedFlashSegment {
    pub address: u32,
    pub data: Vec<u8>,
}

pub struct EspSession {
    flasher: Flasher,
    device: DetectedDevice,
    reset_on_drop: bool,
}

impl EspSession {
    pub fn connect_with_diagnostics(
        port_name: &str,
        connect_attempt: usize,
        diagnostic: Arc<dyn Fn(&str) + Send + Sync>,
    ) -> Result<Self> {
        let port_info = find_usb_port_info(port_name)?;
        diagnostic("Открытие COM-порта на 115200 baud");
        let mut serial = serialport::new(port_name, 115_200)
            .timeout(Duration::from_secs(3))
            .open_native()
            .map_err(|error| {
                OperationError::new(
                    ErrorCode::PortBusy,
                    "Не удалось открыть COM-порт для прошивки",
                )
                .with_detail(error.to_string())
                .retryable(true)
            })?;
        let reset_profile = connect_reset_profile(connect_attempt);
        let before_operation = match reset_profile {
            ConnectResetProfile::EsptoolClassic { release_delay } => {
                diagnostic(&format!(
                    "COM-порт открыт; esptool-compatible reset в ROM bootloader ({} мс)",
                    release_delay.as_millis()
                ));
                esptool_classic_reset(&mut serial, release_delay).map_err(|error| {
                    OperationError::new(
                        ErrorCode::FlashConnectFailed,
                        "Не удалось выполнить reset ESP32 в bootloader",
                    )
                    .with_detail(error.to_string())
                    .retryable(true)
                })?;
                ResetBeforeOperation::NoReset
            }
            ConnectResetProfile::EspflashFallback => {
                diagnostic("COM-порт открыт; резервная последовательность reset в ROM bootloader");
                ResetBeforeOperation::DefaultReset
            }
        };
        let connection = Connection::new(
            serial,
            port_info,
            RESET_AFTER_OPERATION,
            before_operation,
            115_200,
        );
        diagnostic("ROM handshake, определение чипа и загрузка flash stub");
        let mut flasher = match Flasher::connect(
            connection,
            true,
            true,
            false,
            None,
            Some(FLASH_BAUD),
        ) {
            Ok(flasher) => flasher,
            Err(error) => {
                let reset_error = reset_port_for_normal_boot(port_name).err();
                let detail = reset_error.map_or_else(
                    || error.to_string(),
                    |reset| {
                        format!(
                            "{error}; не удалось вернуть линии reset в нормальное состояние: {reset}"
                        )
                    },
                );
                return Err(OperationError::new(
                    ErrorCode::FlashConnectFailed,
                    "ESP32 не ответила в режиме загрузчика",
                )
                .with_detail(detail)
                .retryable(true));
            }
        };
        diagnostic("ROM bootloader отвечает; чтение параметров flash и MAC");
        let info = match flasher.device_info() {
            Ok(info) => info,
            Err(error) => {
                reset_connected_flasher(&mut flasher);
                return Err(OperationError::new(
                    ErrorCode::FlashConnectFailed,
                    "Не удалось прочитать информацию ESP32",
                )
                .with_detail(error.to_string()));
            }
        };
        let chip = match ChipFamily::try_from(info.chip.to_string().as_str()) {
            Ok(chip) => chip,
            Err(error) => {
                reset_connected_flasher(&mut flasher);
                return Err(error);
            }
        };
        let device = DetectedDevice {
            port: port_name.to_string(),
            description: format!("{} • {}", info.chip, info.flash_size),
            chip: chip.to_string(),
            mac: info.mac_address.unwrap_or_else(|| "недоступен".to_string()),
            flash_size_bytes: u64::from(info.flash_size.size()),
        };
        diagnostic("ESP32 определена; соединение готово к операции");
        Ok(Self {
            flasher,
            device,
            reset_on_drop: true,
        })
    }

    pub fn device(&self) -> &DetectedDevice {
        &self.device
    }

    pub fn chip(&self) -> Result<ChipFamily> {
        ChipFamily::try_from(self.device.chip.as_str())
    }

    pub fn close_for_monitor(mut self) -> DetectedDevice {
        self.reset_on_drop = false;
        self.device.clone()
    }

    pub fn read_region(&mut self, address: u32, size: u32) -> Result<Vec<u8>> {
        let temporary = NamedTempFile::new().map_err(io_error)?;
        let path = temporary.into_temp_path();
        let file_path: PathBuf = path.to_path_buf();
        self.flasher
            .read_flash(address, size, 0x1000, 64, file_path.clone())
            .map_err(|error| {
                OperationError::new(
                    ErrorCode::FlashVerifyFailed,
                    "Не удалось прочитать flash ESP32",
                )
                .with_detail(error.to_string())
            })?;
        fs::read(file_path).map_err(io_error)
    }

    pub fn read_partition_table(
        &mut self,
        declared_offset: Option<u32>,
    ) -> Result<(u32, Vec<PartitionEntry>)> {
        discover_partition_table_with(declared_offset, |address| self.read_region(address, 0x1000))
    }

    pub fn erase_all(&mut self) -> Result<()> {
        self.flasher.erase_flash().map_err(|error| {
            OperationError::new(
                ErrorCode::FlashEraseFailed,
                "Полное стирание flash завершилось ошибкой",
            )
            .with_detail(error.to_string())
        })
    }

    pub fn write_segments(
        &mut self,
        owned: &[OwnedFlashSegment],
        on_progress: Arc<dyn Fn(OperationProgress) + Send + Sync>,
    ) -> Result<()> {
        if owned.is_empty() {
            return Err(OperationError::new(
                ErrorCode::PackageInvalid,
                "Нет сегментов для записи",
            ));
        }
        let segment_sizes = owned
            .iter()
            .map(|segment| segment.data.len())
            .collect::<Vec<_>>();
        let segments: Vec<_> = owned
            .iter()
            .map(|segment| Segment {
                addr: segment.address,
                data: Cow::Borrowed(segment.data.as_slice()),
            })
            .collect();
        let mut progress = ProgressBridge::new(segment_sizes, on_progress);
        self.flasher
            .write_bins_to_flash(&segments, &mut progress)
            .map_err(|error| {
                OperationError::new(
                    ErrorCode::FlashWriteFailed,
                    "Запись или проверка flash завершилась ошибкой",
                )
                .with_detail(error.to_string())
                .retryable(true)
            })
    }
}

impl Drop for EspSession {
    fn drop(&mut self) {
        if self.reset_on_drop {
            let chip = self.flasher.chip();
            let _ = self.flasher.connection().reset_after(true, chip);
        }
    }
}

fn reset_connected_flasher(flasher: &mut Flasher) {
    let chip = flasher.chip();
    let _ = flasher.connection().reset_after(true, chip);
}

struct ProgressBridge {
    total: usize,
    completed: usize,
    current_steps: usize,
    current_size: usize,
    segment_sizes: Vec<usize>,
    segment_index: usize,
    segment_count: usize,
    sink: Arc<dyn Fn(OperationProgress) + Send + Sync>,
}

impl ProgressBridge {
    fn new(segment_sizes: Vec<usize>, sink: Arc<dyn Fn(OperationProgress) + Send + Sync>) -> Self {
        let total = segment_sizes.iter().sum();
        let segment_count = segment_sizes.len();
        Self {
            total,
            completed: 0,
            current_steps: 0,
            current_size: 0,
            segment_sizes,
            segment_index: 0,
            segment_count,
            sink,
        }
    }

    fn emit(&self, stage: OperationStage, current: usize, message: String) {
        let absolute = self.completed.saturating_add(current).min(self.total);
        (self.sink)(OperationProgress {
            stage,
            current: absolute as u64,
            total: self.total as u64,
            percentage: if self.total == 0 {
                0.0
            } else {
                absolute as f32 * 100.0 / self.total as f32
            },
            segment_index: self.segment_index + 1,
            segment_count: self.segment_count,
            message,
        });
    }
}

impl ProgressCallbacks for ProgressBridge {
    fn init(&mut self, address: u32, total: usize) {
        self.current_steps = total.max(1);
        self.current_size = self
            .segment_sizes
            .get(self.segment_index)
            .copied()
            .unwrap_or_default();
        self.emit(
            OperationStage::Writing,
            0,
            format!(
                "Запись {} байт по адресу 0x{address:08X}",
                self.current_size
            ),
        );
    }

    fn update(&mut self, current: usize) {
        let current_bytes = current
            .min(self.current_steps)
            .saturating_mul(self.current_size)
            / self.current_steps;
        self.emit(
            OperationStage::Writing,
            current_bytes,
            format!("Сегмент {}/{}", self.segment_index + 1, self.segment_count),
        );
    }

    fn verifying(&mut self) {
        self.emit(
            OperationStage::Verifying,
            self.current_size,
            "Проверка записанного сегмента".to_string(),
        );
    }

    fn finish(&mut self, _skipped: bool) {
        self.completed = self.completed.saturating_add(self.current_size);
        self.emit(
            OperationStage::Writing,
            0,
            format!("Сегмент {} записан", self.segment_index + 1),
        );
        self.segment_index += 1;
    }
}

fn find_usb_port_info(port_name: &str) -> Result<serialport::UsbPortInfo> {
    let ports = serialport::available_ports().map_err(|error| {
        OperationError::new(
            ErrorCode::DeviceNotFound,
            "Не удалось получить список COM-портов",
        )
        .with_detail(error.to_string())
    })?;
    for port in ports {
        if port.port_name.eq_ignore_ascii_case(port_name) {
            if let SerialPortType::UsbPort(info) = port.port_type {
                return Ok(info);
            }
            break;
        }
    }
    Ok(serialport::UsbPortInfo {
        vid: 0,
        pid: 0,
        serial_number: None,
        manufacturer: None,
        product: None,
    })
}

fn io_error(error: impl std::fmt::Display) -> OperationError {
    OperationError::new(ErrorCode::IoError, "Ошибка временного файла")
        .with_detail(error.to_string())
}

fn discover_partition_table_with(
    declared_offset: Option<u32>,
    mut read: impl FnMut(u32) -> Result<Vec<u8>>,
) -> Result<(u32, Vec<PartitionEntry>)> {
    const DEFAULT_OFFSET: u32 = 0x8000;
    const LAST_SCAN_OFFSET: u32 = 0x1F000;
    let offsets: Vec<u32> = if let Some(offset) = declared_offset {
        vec![offset]
    } else {
        (DEFAULT_OFFSET..=LAST_SCAN_OFFSET)
            .step_by(0x1000)
            .collect()
    };
    let mut last_parse_error = None;
    for address in offsets {
        let bytes = read(address)?;
        match parse_partition_table(&bytes) {
            Ok(entries)
                if entries
                    .iter()
                    .any(|entry| entry.is_factory_app() || entry.is_ota_app()) =>
            {
                return Ok((address, entries));
            }
            Ok(_) => {
                last_parse_error = Some(OperationError::new(
                    ErrorCode::PartitionInvalid,
                    "Partition table не содержит application-разделов",
                ));
            }
            Err(error) => last_parse_error = Some(error),
        }
    }
    Err(OperationError::new(
        ErrorCode::PartitionInvalid,
        "Не удалось обнаружить partition table на плате",
    )
    .with_detail(
        last_parse_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "проверяемый диапазон пуст".to_string()),
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        connect_reset_profile, discover_partition_table_with, ConnectResetProfile,
        OperationProgress, ProgressBridge, ProgressCallbacks, ESPTOOL_EXTENDED_RESET_DELAY,
        ESPTOOL_RESET_DELAY, RESET_AFTER_OPERATION,
    };
    use espflash::connection::ResetAfterOperation;
    use programmer_core::{ErrorCode, OperationError};
    use std::sync::{Arc, Mutex};

    fn partition() -> Vec<u8> {
        let mut bytes = vec![0xFF; 0x1000];
        bytes[0..2].copy_from_slice(&0x50AA_u16.to_le_bytes());
        bytes[2] = 0;
        bytes[3] = 0;
        bytes[4..8].copy_from_slice(&0x10000_u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&0x100000_u32.to_le_bytes());
        bytes[12..19].copy_from_slice(b"factory");
        bytes[28..32].copy_from_slice(&0_u32.to_le_bytes());
        bytes
    }

    #[test]
    fn discovers_aligned_custom_partition_table() {
        let mut reads = Vec::new();
        let (address, entries) = discover_partition_table_with(None, |address| {
            reads.push(address);
            Ok(if address == 0xA000 {
                partition()
            } else {
                vec![0xFF; 0x1000]
            })
        })
        .unwrap();
        assert_eq!(address, 0xA000);
        assert_eq!(entries[0].label, "factory");
        assert_eq!(reads, vec![0x8000, 0x9000, 0xA000]);
    }

    #[test]
    fn declared_offset_is_not_replaced_by_heuristic() {
        let error =
            discover_partition_table_with(Some(0xB000), |_| Ok(vec![0xFF; 0x1000])).unwrap_err();
        assert_eq!(error.code, ErrorCode::PartitionInvalid);
    }

    #[test]
    fn read_failure_is_not_hidden() {
        let error = discover_partition_table_with(None, |_| {
            Err(OperationError::new(
                ErrorCode::FlashVerifyFailed,
                "read failed",
            ))
        })
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::FlashVerifyFailed);
    }

    #[test]
    fn maps_compressed_chunk_progress_to_uncompressed_package_bytes() {
        let events = Arc::new(Mutex::new(Vec::<OperationProgress>::new()));
        let captured = events.clone();
        let mut bridge = ProgressBridge::new(
            vec![100, 300],
            Arc::new(move |progress| captured.lock().unwrap().push(progress)),
        );

        ProgressCallbacks::init(&mut bridge, 0x1000, 4);
        ProgressCallbacks::update(&mut bridge, 2);
        ProgressCallbacks::finish(&mut bridge, false);
        ProgressCallbacks::init(&mut bridge, 0x10000, 3);
        ProgressCallbacks::update(&mut bridge, 2);
        ProgressCallbacks::finish(&mut bridge, false);

        let events = events.lock().unwrap();
        assert_eq!(events[1].current, 50);
        assert_eq!(events[1].total, 400);
        assert_eq!(events[1].percentage, 12.5);
        assert_eq!(events[4].current, 300);
        assert_eq!(events.last().unwrap().current, 400);
        assert_eq!(events.last().unwrap().percentage, 100.0);
    }

    #[test]
    fn every_completed_connection_defaults_to_normal_boot_reset() {
        assert_eq!(RESET_AFTER_OPERATION, ResetAfterOperation::HardReset);
    }

    #[test]
    fn connection_retries_use_distinct_reset_profiles() {
        assert_eq!(
            connect_reset_profile(1),
            ConnectResetProfile::EsptoolClassic {
                release_delay: ESPTOOL_RESET_DELAY
            }
        );
        assert_eq!(
            connect_reset_profile(2),
            ConnectResetProfile::EsptoolClassic {
                release_delay: ESPTOOL_EXTENDED_RESET_DELAY
            }
        );
        assert_eq!(
            connect_reset_profile(3),
            ConnectResetProfile::EspflashFallback
        );
    }

    #[test]
    #[ignore = "requires a connected ESP32 and ESP32_FLASHER_HIL_PORT"]
    fn hil_esptool_reset_profiles_connect_and_read_device() {
        let port = std::env::var("ESP32_FLASHER_HIL_PORT")
            .expect("set ESP32_FLASHER_HIL_PORT to the test COM port");
        for attempt in [1, 2] {
            let diagnostics =
                Arc::new(move |message: &str| println!("profile {attempt}: {message}"));
            let session = super::EspSession::connect_with_diagnostics(&port, attempt, diagnostics)
                .expect("esptool-compatible reset must connect to the test board");
            assert_eq!(session.device().port, port);
            assert!(!session.device().chip.is_empty());
        }
    }
}
