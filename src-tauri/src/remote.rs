use tauri::State;

use crate::{
    logs,
    models::{ConnectionCheck, ConnectionProfile, LogSource, RemoteEntry, RemoteFile},
    sftp,
    ssh_client::{reject_password_auth, ssh_command},
    state::{AppState, TailStreams, TerminalSessions},
};

#[tauri::command]
pub(crate) fn check_ssh_connection(
    profile: ConnectionProfile,
    state: State<'_, AppState>,
) -> Result<ConnectionCheck, String> {
    if let Err(message) = reject_password_auth(&profile, "SSH 인증") {
        return Ok(ConnectionCheck {
            connected: false,
            message,
        });
    }
    let mut command = ssh_command(&profile, &state.known_hosts_file)?;
    command.arg("exit");
    let output = command
        .output()
        .map_err(|error| format!("Windows OpenSSH를 실행하지 못했습니다: {error}"))?;
    if output.status.success() {
        Ok(ConnectionCheck {
            connected: true,
            message: "SSH 인증을 확인했습니다.".into(),
        })
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Ok(ConnectionCheck {
            connected: false,
            message: if error.is_empty() {
                "SSH 연결에 실패했습니다.".into()
            } else {
                error
            },
        })
    }
}

#[tauri::command]
pub(crate) fn start_terminal(
    profile: ConnectionProfile,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    sessions: State<'_, TerminalSessions>,
) -> Result<u64, String> {
    crate::terminal::start_terminal(&profile, &state.known_hosts_file, app, &sessions)
}

#[tauri::command]
pub(crate) fn write_terminal(
    terminal_id: u64,
    input: String,
    sessions: State<'_, TerminalSessions>,
) -> Result<(), String> {
    crate::terminal::write_terminal(terminal_id, input, &sessions)
}

#[tauri::command]
pub(crate) fn stop_terminal(
    terminal_id: u64,
    sessions: State<'_, TerminalSessions>,
) -> Result<(), String> {
    crate::terminal::stop_terminal(terminal_id, &sessions)
}

#[tauri::command]
pub(crate) fn resize_terminal(
    terminal_id: u64,
    columns: u16,
    rows: u16,
    sessions: State<'_, TerminalSessions>,
) -> Result<(), String> {
    crate::terminal::resize_terminal(terminal_id, columns, rows, &sessions)
}

#[tauri::command]
pub(crate) fn list_remote_directory(
    profile: ConnectionProfile,
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<RemoteEntry>, String> {
    sftp::list_directory(&profile, &path, &state.known_hosts_file)
}

#[tauri::command]
pub(crate) fn delete_remote_entry(
    profile: ConnectionProfile,
    path: String,
    is_directory: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    sftp::delete_entry(&profile, &path, is_directory, &state.known_hosts_file)
}

#[tauri::command]
pub(crate) fn rename_remote_entry(
    profile: ConnectionProfile,
    path: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    sftp::rename_entry(&profile, &path, &new_name, &state.known_hosts_file)
}

#[tauri::command]
pub(crate) fn find_remote_log_files(
    profile: ConnectionProfile,
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<RemoteEntry>, String> {
    logs::find_log_files(&profile, &path, &state.known_hosts_file)
}

#[tauri::command]
pub(crate) fn read_remote_file(
    profile: ConnectionProfile,
    path: String,
    state: State<'_, AppState>,
) -> Result<RemoteFile, String> {
    logs::read_recent_file(&profile, path, &state.known_hosts_file)
}

#[tauri::command]
pub(crate) fn read_remote_log_files(
    profile: ConnectionProfile,
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<LogSource>, String> {
    logs::read_for_analysis(&profile, paths, &state.known_hosts_file)
}

#[tauri::command]
pub(crate) fn start_tail_stream(
    profile: ConnectionProfile,
    path: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    streams: State<'_, TailStreams>,
) -> Result<u64, String> {
    logs::start_tail_stream(&profile, &path, &state.known_hosts_file, app, &streams)
}

#[tauri::command]
pub(crate) fn stop_tail_stream(
    stream_id: u64,
    streams: State<'_, TailStreams>,
) -> Result<(), String> {
    logs::stop_tail_stream(stream_id, &streams)
}
