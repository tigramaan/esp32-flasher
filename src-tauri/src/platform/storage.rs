use crate::application::{DataDirectoryStatus, PortableSettings};
use programmer_core::{ErrorCode, OperationError, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use uuid::Uuid;

pub struct PortableDataStore {
    root: RwLock<Option<PathBuf>>,
}

impl PortableDataStore {
    pub fn discover() -> Self {
        let explicit = parse_data_dir_argument();
        let candidate = explicit.or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(|parent| parent.join("data")))
        });
        let root = candidate.and_then(|path| validate_writable(&path).ok().map(|_| path));
        Self {
            root: RwLock::new(root),
        }
    }

    pub fn status(&self) -> DataDirectoryStatus {
        let root = self.root.read().expect("data root lock poisoned");
        DataDirectoryStatus {
            path: root.as_ref().map(|path| path.display().to_string()),
            writable: root.is_some(),
        }
    }

    pub fn set_root(&self, path: impl AsRef<Path>) -> Result<DataDirectoryStatus> {
        let path = absolute(path.as_ref())?;
        validate_writable(&path)?;
        *self.root.write().expect("data root lock poisoned") = Some(path);
        Ok(self.status())
    }

    pub fn root(&self) -> Result<PathBuf> {
        self.root
            .read()
            .expect("data root lock poisoned")
            .clone()
            .ok_or_else(|| {
                OperationError::new(
                    ErrorCode::DataDirectoryUnwritable,
                    "Выберите доступную для записи рабочую папку",
                )
            })
    }

    pub fn ensure_subdir(&self, name: &str) -> Result<PathBuf> {
        if !matches!(name, "logs" | "reports") {
            return Err(OperationError::new(
                ErrorCode::InternalError,
                "Недопустимый portable data subdirectory",
            ));
        }
        let path = self.root()?.join(name);
        fs::create_dir_all(&path).map_err(io_error)?;
        Ok(path)
    }

    pub fn load_settings(&self) -> Result<PortableSettings> {
        let path = self.root()?.join("settings.json");
        if !path.exists() {
            return Ok(PortableSettings::default());
        }
        let bytes = fs::read(&path).map_err(io_error)?;
        let settings: PortableSettings = serde_json::from_slice(&bytes).map_err(|error| {
            OperationError::new(ErrorCode::PackageInvalid, "settings.json повреждён")
                .with_detail(error.to_string())
        })?;
        if settings.schema_version != 1 {
            return Err(OperationError::new(
                ErrorCode::PackageUnsupported,
                "Версия settings.json не поддерживается",
            ));
        }
        if settings.factory_success_marker.len() > 256 {
            return Err(OperationError::new(
                ErrorCode::PackageInvalid,
                "Производственный UART-маркер превышает 256 байт",
            ));
        }
        validate_monitor_baud(settings.monitor_baud)?;
        Ok(settings)
    }

    pub fn save_settings(&self, settings: &PortableSettings) -> Result<()> {
        if settings.schema_version != 1 {
            return Err(OperationError::new(
                ErrorCode::PackageUnsupported,
                "Версия settings.json не поддерживается",
            ));
        }
        if settings.factory_success_marker.len() > 256 {
            return Err(OperationError::new(
                ErrorCode::PackageInvalid,
                "Производственный UART-маркер превышает 256 байт",
            ));
        }
        validate_monitor_baud(settings.monitor_baud)?;
        let root = self.root()?;
        let destination = root.join("settings.json");
        let temporary = root.join(format!(".settings-{}.tmp", Uuid::new_v4()));
        let bytes = serde_json::to_vec_pretty(settings).map_err(|error| {
            OperationError::new(ErrorCode::InternalError, "Ошибка сериализации settings")
                .with_detail(error.to_string())
        })?;
        let mut file = fs::File::create(&temporary).map_err(io_error)?;
        file.write_all(&bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        drop(file);
        fs::rename(&temporary, &destination).map_err(io_error)
    }
}

fn parse_data_dir_argument() -> Option<PathBuf> {
    let mut args = std::env::args_os();
    while let Some(argument) = args.next() {
        if argument == "--data-dir" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

fn validate_writable(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(unwritable)?;
    let probe = path.join(format!(".programmer-write-{}.tmp", Uuid::new_v4()));
    let mut file = fs::File::create(&probe).map_err(unwritable)?;
    file.write_all(b"ok").map_err(unwritable)?;
    file.sync_all().map_err(unwritable)?;
    fs::remove_file(&probe).map_err(unwritable)?;
    Ok(())
}

fn absolute(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir().map_err(io_error)?.join(path))
    }
}

fn unwritable(error: impl std::fmt::Display) -> OperationError {
    OperationError::new(
        ErrorCode::DataDirectoryUnwritable,
        "Рабочая папка недоступна для записи",
    )
    .with_detail(error.to_string())
}

fn io_error(error: impl std::fmt::Display) -> OperationError {
    OperationError::new(ErrorCode::IoError, "Ошибка portable-хранилища")
        .with_detail(error.to_string())
}

fn validate_monitor_baud(baud: u32) -> Result<()> {
    if matches!(baud, 9_600 | 57_600 | 115_200 | 230_400 | 460_800 | 921_600) {
        return Ok(());
    }
    Err(OperationError::new(
        ErrorCode::PackageInvalid,
        "Неподдерживаемая скорость UART-монитора",
    )
    .with_detail(baud.to_string()))
}

#[cfg(test)]
mod tests {
    use super::PortableDataStore;
    use crate::application::{AppMode, PortableSettings};
    use tempfile::TempDir;

    #[test]
    fn replaces_settings_without_leaving_temporary_files() {
        let temporary = TempDir::new().unwrap();
        let store = PortableDataStore::discover();
        store.set_root(temporary.path()).unwrap();
        let first = PortableSettings::default();
        store.save_settings(&first).unwrap();

        let second = PortableSettings {
            schema_version: 1,
            mode: AppMode::Factory,
            last_update_package: Some("update".into()),
            last_factory_package: Some("factory".into()),
            factory_success_marker: "READY".into(),
            monitor_baud: 230_400,
        };
        store.save_settings(&second).unwrap();

        let loaded = store.load_settings().unwrap();
        assert!(matches!(loaded.mode, AppMode::Factory));
        assert_eq!(loaded.last_factory_package.as_deref(), Some("factory"));
        assert_eq!(loaded.factory_success_marker, "READY");
        assert_eq!(loaded.monitor_baud, 230_400);
        let leftovers = std::fs::read_dir(temporary.path())
            .unwrap()
            .filter_map(std::result::Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".settings-")
            })
            .count();
        assert_eq!(leftovers, 0);
    }

    #[test]
    fn rejects_oversized_factory_marker_before_writing_settings() {
        let temporary = TempDir::new().unwrap();
        let store = PortableDataStore::discover();
        store.set_root(temporary.path()).unwrap();
        let settings = PortableSettings {
            factory_success_marker: "Я".repeat(129),
            ..PortableSettings::default()
        };

        let error = store.save_settings(&settings).unwrap_err();

        assert_eq!(error.code, programmer_core::ErrorCode::PackageInvalid);
        assert!(!temporary.path().join("settings.json").exists());
    }

    #[test]
    fn loads_legacy_settings_with_default_monitor_baud() {
        let temporary = TempDir::new().unwrap();
        let store = PortableDataStore::discover();
        store.set_root(temporary.path()).unwrap();
        std::fs::write(
            temporary.path().join("settings.json"),
            br#"{
                "schema_version": 1,
                "mode": "update",
                "last_update_package": null,
                "last_factory_package": null,
                "factory_success_marker": ""
            }"#,
        )
        .unwrap();

        let settings = store.load_settings().unwrap();

        assert_eq!(settings.monitor_baud, 115_200);
    }
}
