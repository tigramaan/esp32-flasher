use crate::platform::storage::PortableDataStore;
use chrono::Utc;
use programmer_core::{ErrorCode, OperationError, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

const MAX_LOG_FILE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_LOG_DIRECTORY_BYTES: u64 = 100 * 1024 * 1024;

pub struct OperationLogger {
    file: Mutex<File>,
}

impl OperationLogger {
    pub fn create(store: &PortableDataStore, operation_id: &str) -> Result<Self> {
        let directory = store.ensure_subdir("logs")?;
        enforce_quota(&directory)?;
        let path = directory.join(format!(
            "{}-{}.log",
            Utc::now().format("%Y%m%d-%H%M%S"),
            safe_id(operation_id)
        ));
        let file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&path)
            .map_err(io_error)?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }

    pub fn write(&self, level: &str, stage: &str, message: &str) -> Result<()> {
        let mut file = self.file.lock().expect("operation log lock poisoned");
        let current = file.metadata().map_err(io_error)?.len();
        if current >= MAX_LOG_FILE_BYTES {
            return Ok(());
        }
        let sanitized = message.replace(['\r', '\n'], " ");
        writeln!(
            file,
            "{}\t{}\t{}\t{}",
            Utc::now().to_rfc3339(),
            level,
            stage,
            sanitized
        )
        .map_err(io_error)?;
        file.flush().map_err(io_error)
    }
}

fn enforce_quota(directory: &Path) -> Result<()> {
    let mut files = Vec::new();
    let mut total = 0_u64;
    for entry in fs::read_dir(directory).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let metadata = entry.metadata().map_err(io_error)?;
        if metadata.is_file() && entry.path().extension().is_some_and(|value| value == "log") {
            total = total.saturating_add(metadata.len());
            files.push((metadata.modified().ok(), metadata.len(), entry.path()));
        }
    }
    files.sort_by_key(|item| item.0);
    for (_, size, path) in files {
        if total <= MAX_LOG_DIRECTORY_BYTES {
            break;
        }
        fs::remove_file(path).map_err(io_error)?;
        total = total.saturating_sub(size);
    }
    Ok(())
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .take(64)
        .collect()
}

fn io_error(error: impl std::fmt::Display) -> OperationError {
    OperationError::new(ErrorCode::IoError, "Ошибка диагностического лога")
        .with_detail(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::OperationLogger;
    use crate::platform::storage::PortableDataStore;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn writes_sanitized_bounded_operation_log() {
        let temporary = TempDir::new().unwrap();
        let store = PortableDataStore::discover();
        store.set_root(temporary.path()).unwrap();
        let logger = OperationLogger::create(&store, "operation-1").unwrap();
        logger
            .write("INFO", "Validating", "первая\r\nвторая")
            .unwrap();

        let log_path = fs::read_dir(temporary.path().join("logs"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        let contents = fs::read_to_string(log_path).unwrap();
        assert!(contents.contains("INFO\tValidating\tпервая  вторая"));
        assert_eq!(contents.lines().count(), 1);
    }
}
