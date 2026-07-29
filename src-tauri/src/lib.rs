mod application;
mod ipc;
mod platform;

use application::AppState;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = Arc::new(AppState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            ipc::system_locale,
            ipc::list_devices,
            ipc::start_serial_monitor,
            ipc::validate_package,
            ipc::data_directory,
            ipc::set_data_directory,
            ipc::get_settings,
            ipc::update_settings,
            ipc::start_factory_session,
            ipc::factory_session,
            ipc::start_update,
            ipc::start_factory_flash,
            ipc::send_serial,
            ipc::reset_monitor,
            ipc::disconnect_monitor,
        ])
        .run(tauri::generate_context!())
        .expect("ESP32 Flasher failed to start");
}
