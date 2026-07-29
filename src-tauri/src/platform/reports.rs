use crate::application::{DetectedDevice, FactorySessionSummary};
use crate::platform::storage::PortableDataStore;
use chrono::{DateTime, Utc};
use programmer_core::{ErrorCode, OperationError, PackageSummary, Result};
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

pub struct FactoryReporter {
    session: Mutex<Option<Session>>,
}

struct Session {
    summary: FactorySessionSummary,
    writer: csv::Writer<File>,
}

#[derive(Debug, Serialize)]
struct CsvRow<'a> {
    timestamp: String,
    session_id: &'a str,
    package_id: &'a str,
    firmware_version: &'a str,
    port: &'a str,
    mac: &'a str,
    chip: &'a str,
    duration_ms: u64,
    full_erase: bool,
    result: &'a str,
    error_code: &'a str,
}

impl FactoryReporter {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
        }
    }

    pub fn start(
        &self,
        store: &PortableDataStore,
        package: &PackageSummary,
    ) -> Result<FactorySessionSummary> {
        let directory = store.ensure_subdir("reports")?;
        let started: DateTime<Utc> = Utc::now();
        let session_id = Uuid::new_v4().to_string();
        let path = unique_report_path(&directory, started);
        let mut file = File::options()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(io_error)?;
        file.write_all(&[0xEF, 0xBB, 0xBF]).map_err(io_error)?;
        let writer = csv::WriterBuilder::new()
            .delimiter(b';')
            .has_headers(true)
            .from_writer(file);
        let summary = FactorySessionSummary {
            session_id,
            started_at: started.to_rfc3339(),
            package_id: package.package_id.clone(),
            firmware_version: package.firmware_version.clone(),
            total: 0,
            passed: 0,
            failed: 0,
            report_path: path.display().to_string(),
        };
        *self.session.lock().expect("factory report lock poisoned") = Some(Session {
            summary: summary.clone(),
            writer,
        });
        Ok(summary)
    }

    pub fn summary(&self) -> Option<FactorySessionSummary> {
        self.session
            .lock()
            .expect("factory report lock poisoned")
            .as_ref()
            .map(|session| session.summary.clone())
    }

    pub fn ensure_matching_session(
        &self,
        store: &PortableDataStore,
        package: &PackageSummary,
    ) -> Result<FactorySessionSummary> {
        if let Some(summary) = self.summary() {
            if summary.package_id == package.package_id
                && summary.firmware_version == package.firmware_version
            {
                return Ok(summary);
            }
        }
        self.start(store, package)
    }

    pub fn record(
        &self,
        package: &PackageSummary,
        device: &DetectedDevice,
        duration_ms: u64,
        full_erase: bool,
        error: Option<&OperationError>,
    ) -> Result<FactorySessionSummary> {
        let mut guard = self.session.lock().expect("factory report lock poisoned");
        let session = guard.as_mut().ok_or_else(|| {
            OperationError::new(
                ErrorCode::InvalidState,
                "Производственная сессия не запущена",
            )
        })?;
        if session.summary.package_id != package.package_id
            || session.summary.firmware_version != package.firmware_version
        {
            return Err(OperationError::new(
                ErrorCode::InvalidState,
                "Пакет не совпадает с активной производственной сессией",
            ));
        }
        let error_code = error.map(|value| value.code.as_str()).unwrap_or("");
        session
            .writer
            .serialize(CsvRow {
                timestamp: Utc::now().to_rfc3339(),
                session_id: &session.summary.session_id,
                package_id: &package.package_id,
                firmware_version: &package.firmware_version,
                port: &device.port,
                mac: &device.mac,
                chip: &device.chip,
                duration_ms,
                full_erase,
                result: if error.is_none() { "OK" } else { "ERROR" },
                error_code,
            })
            .map_err(csv_error)?;
        session.writer.flush().map_err(io_error)?;
        session.summary.total += 1;
        if error.is_none() {
            session.summary.passed += 1;
        } else {
            session.summary.failed += 1;
        }
        Ok(session.summary.clone())
    }
}

impl Default for FactoryReporter {
    fn default() -> Self {
        Self::new()
    }
}

fn unique_report_path(directory: &Path, started: DateTime<Utc>) -> PathBuf {
    let stem = format!("factory-{}", started.format("%Y%m%d-%H%M%S"));
    let primary = directory.join(format!("{stem}.csv"));
    if !primary.exists() {
        primary
    } else {
        directory.join(format!("{stem}-{}.csv", Uuid::new_v4()))
    }
}

fn io_error(error: impl std::fmt::Display) -> OperationError {
    OperationError::new(ErrorCode::IoError, "Ошибка CSV-отчёта").with_detail(error.to_string())
}

fn csv_error(error: csv::Error) -> OperationError {
    io_error(error)
}

#[cfg(test)]
mod tests {
    use super::FactoryReporter;
    use crate::application::{AppMode, DetectedDevice, PortableSettings};
    use crate::platform::storage::PortableDataStore;
    use programmer_core::{FirmwareSource, PackageKind, PackageSummary};
    use std::fs;
    use tempfile::TempDir;

    fn package() -> PackageSummary {
        PackageSummary {
            package_id: "nova".into(),
            display_name: "NOVA".into(),
            firmware_version: "1.0.0".into(),
            kind: PackageKind::Factory,
            target_chips: vec!["esp32".into()],
            segment_count: 3,
            total_bytes: 42,
            monitor_baud: 115_200,
            success_timeout_ms: 15_000,
            success_marker_configured: false,
            source: FirmwareSource::Platformio,
            requires_device_layout: false,
            segments: Vec::new(),
        }
    }

    #[test]
    fn writes_one_row_and_updates_counters() {
        let temp = TempDir::new().unwrap();
        let store = PortableDataStore::discover();
        store.set_root(temp.path()).unwrap();
        store
            .save_settings(&PortableSettings {
                schema_version: 1,
                mode: AppMode::Factory,
                last_update_package: None,
                last_factory_package: None,
                factory_success_marker: String::new(),
                monitor_baud: 115_200,
            })
            .unwrap();
        let reporter = FactoryReporter::new();
        reporter.start(&store, &package()).unwrap();
        let summary = reporter
            .record(
                &package(),
                &DetectedDevice {
                    port: "COM5".into(),
                    description: "USB".into(),
                    chip: "esp32".into(),
                    mac: "AA:BB:CC:DD:EE:FF".into(),
                    flash_size_bytes: 4 * 1024 * 1024,
                },
                1000,
                false,
                None,
            )
            .unwrap();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.passed, 1);
        let bytes = fs::read(summary.report_path).unwrap();
        assert!(bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
        assert!(String::from_utf8_lossy(&bytes).contains("AA:BB:CC:DD:EE:FF"));
    }
}
