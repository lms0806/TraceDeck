mod logs;
mod models;
mod profiles;
mod remote;
mod sftp;
mod ssh_client;
mod state;
mod system;
mod terminal;

use tauri::Manager;
use trace_deck_core::AppPaths;

use state::{AppState, SystemStreams, TailStreams, TerminalSessions};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let paths = AppPaths::from_data_dir(data_dir);
            paths.ensure_exists().map_err(std::io::Error::other)?;
            app.manage(AppState::new(paths));
            app.manage(TailStreams::new());
            app.manage(SystemStreams::new());
            app.manage(TerminalSessions::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            profiles::list_profiles,
            profiles::save_profile,
            profiles::delete_profile,
            remote::check_ssh_connection,
            remote::list_remote_directory,
            remote::delete_remote_entry,
            remote::rename_remote_entry,
            remote::create_remote_directory,
            remote::create_remote_file,
            remote::start_remote_system_status_stream,
            remote::stop_remote_system_status_stream,
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
        .expect("error while running TraceDeck");
}
