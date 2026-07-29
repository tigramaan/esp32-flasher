#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    esp32_flasher_lib::run()
}
