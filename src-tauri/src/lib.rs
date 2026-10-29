mod logs;
mod models;
mod profiles;
mod remote;
mod sftp;
mod ssh_client;
mod state;
mod storage_security;
mod terminal;

use std::fs;

use tauri::Manager;

use state::{AppState, TailStreams, TerminalSessions};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            fs::create_dir_all(&data_dir)?;
            app.manage(AppState {
                profile_file: data_dir.join("profiles.json"),
                known_hosts_file: data_dir.join("known_hosts"),
            });
            app.manage(TailStreams::new());
            app.manage(TerminalSessions::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            profiles::list_profiles,
            profiles::save_profile,
            profiles::delete_profile,
            remote::check_ssh_connection,
            remote::list_remote_directory,
            remote::find_remote_log_files,
            remote::read_remote_file,
            remote::read_remote_log_files,
            remote::start_tail_stream,
            remote::stop_tail_stream,
            remote::start_terminal,
            remote::write_terminal,
            remote::stop_terminal,
            remote::resize_terminal
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tracing Log");
}
