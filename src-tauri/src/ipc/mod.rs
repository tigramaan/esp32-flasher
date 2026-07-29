use crate::application::{
    run_factory_flash, run_update, AppState, DataDirectoryStatus, FactoryFlashRequest,
    FactorySessionSummary, MonitorEvent, OperationEvent, OperationResult, PortCandidate,
    PortableSettings, UpdateRequest,
};
use crate::platform::package::load_package;
use crate::platform::serial;
use programmer_core::{ErrorCode, OperationError, PackageKind, PackageSummary, Result};
use std::sync::Arc;
use tauri::ipc::Channel;
use tauri::State;

#[tauri::command]
pub fn list_devices() -> Result<Vec<PortCandidate>> {
    serial::list_ports()
}

#[tauri::command]
pub fn system_locale() -> String {
    system_locale_name()
}

#[cfg(windows)]
fn system_locale_name() -> String {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;

    const LOCALE_NAME_MAX_LENGTH: i32 = 85;
    let mut buffer = [0_u16; LOCALE_NAME_MAX_LENGTH as usize];
    let length = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), LOCALE_NAME_MAX_LENGTH) };
    if length <= 1 {
        return "en".to_string();
    }
    String::from_utf16_lossy(&buffer[..length as usize - 1])
}

#[cfg(not(windows))]
fn system_locale_name() -> String {
    std::env::var("LANG").unwrap_or_else(|_| "en".to_string())
}

#[tauri::command]
pub async fn start_serial_monitor(
    state: State<'_, Arc<AppState>>,
    port: String,
    baud: u32,
    on_event: Channel<MonitorEvent>,
) -> Result<()> {
    let state = state.inner().clone();
    tokio::task::spawn_blocking(move || {
        let data_events = on_event.clone();
        let disconnect_events = on_event;
        let disconnect_port = port.clone();
        state.install_user_monitor(move || {
            let (monitor, _signals) = serial::start_monitor(
                &port,
                baud,
                "",
                serial::MonitorStartMode::Passive,
                Arc::new(move |data| {
                    let _ = data_events.send(MonitorEvent::Data { data });
                }),
                Arc::new(move |message| {
                    let _ = disconnect_events.send(MonitorEvent::Disconnected {
                        port: disconnect_port.clone(),
                        message,
                    });
                }),
            )?;
            Ok(monitor)
        })
    })
    .await
    .map_err(join_error)?
}

#[tauri::command]
pub async fn validate_package(path: String) -> Result<PackageSummary> {
    tokio::task::spawn_blocking(move || {
        let package = load_package(path)?;
        Ok(package.summary())
    })
    .await
    .map_err(join_error)?
}

#[tauri::command]
pub fn data_directory(state: State<'_, Arc<AppState>>) -> DataDirectoryStatus {
    state.data.status()
}

#[tauri::command]
pub fn set_data_directory(
    state: State<'_, Arc<AppState>>,
    path: String,
) -> Result<DataDirectoryStatus> {
    state.data.set_root(path)
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Result<PortableSettings> {
    state.data.load_settings()
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, Arc<AppState>>,
    settings: PortableSettings,
) -> Result<PortableSettings> {
    state.data.save_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn start_factory_session(
    state: State<'_, Arc<AppState>>,
    package_path: String,
) -> Result<FactorySessionSummary> {
    let package = load_package(package_path)?;
    if package.validated.manifest.kind != PackageKind::Factory {
        return Err(OperationError::new(
            ErrorCode::PackageInvalid,
            "Для производственной сессии нужен пакет kind=factory",
        ));
    }
    state.reports.start(&state.data, &package.summary())
}

#[tauri::command]
pub fn factory_session(state: State<'_, Arc<AppState>>) -> Option<FactorySessionSummary> {
    state.reports.summary()
}

#[tauri::command]
pub async fn start_update(
    state: State<'_, Arc<AppState>>,
    request: UpdateRequest,
    on_event: Channel<OperationEvent>,
) -> Result<OperationResult> {
    let state = state.inner().clone();
    tokio::task::spawn_blocking(move || {
        run_update(
            state,
            request,
            Arc::new(move |event| {
                let _ = on_event.send(event);
            }),
        )
    })
    .await
    .map_err(join_error)?
}

#[tauri::command]
pub async fn start_factory_flash(
    state: State<'_, Arc<AppState>>,
    request: FactoryFlashRequest,
    on_event: Channel<OperationEvent>,
) -> Result<OperationResult> {
    let state = state.inner().clone();
    tokio::task::spawn_blocking(move || {
        run_factory_flash(
            state,
            request,
            Arc::new(move |event| {
                let _ = on_event.send(event);
            }),
        )
    })
    .await
    .map_err(join_error)?
}

#[tauri::command]
pub fn send_serial(state: State<'_, Arc<AppState>>, data: String) -> Result<()> {
    state.send_serial(data.into_bytes())
}

#[tauri::command]
pub async fn reset_monitor(state: State<'_, Arc<AppState>>) -> Result<()> {
    let state = state.inner().clone();
    tokio::task::spawn_blocking(move || state.reset_monitor())
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn disconnect_monitor(state: State<'_, Arc<AppState>>) -> Result<()> {
    let state = state.inner().clone();
    tokio::task::spawn_blocking(move || state.disconnect_monitor())
        .await
        .map_err(join_error)?
}

fn join_error(error: tokio::task::JoinError) -> OperationError {
    OperationError::new(
        ErrorCode::InternalError,
        "Фоновая операция аварийно завершилась",
    )
    .with_detail(error.to_string())
}
