#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tracing_log_app_lib::run();
}
