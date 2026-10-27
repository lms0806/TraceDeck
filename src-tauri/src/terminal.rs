use std::{
    io::{Read, Write},
    path::Path,
    sync::atomic::Ordering,
    thread,
};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use tauri::{Emitter, Manager};

use crate::{
    models::{ConnectionProfile, TerminalChunk},
    ssh_client::{interactive_ssh_arguments, reject_password_auth},
    state::{TerminalProcess, TerminalSessions},
};

pub(crate) fn start_terminal(
    profile: &ConnectionProfile,
    known_hosts_file: &Path,
    app: tauri::AppHandle,
    sessions: &TerminalSessions,
) -> Result<u64, String> {
    reject_password_auth(profile, "대화형 터미널")?;
    let terminal_id = sessions.next_id.fetch_add(1, Ordering::Relaxed);
    let profile = profile.clone();
    let known_hosts_file = known_hosts_file.to_path_buf();
    let app_for_thread = app.clone();
    std::thread::spawn(move || {
        let sessions = app_for_thread.state::<TerminalSessions>();
        if let Err(error) = start_terminal_process(
            &profile,
            &known_hosts_file,
            terminal_id,
            app_for_thread.clone(),
            &sessions,
        ) {
            let _ = app_for_thread.emit(
                "terminal-chunk",
                TerminalChunk {
                    terminal_id,
                    chunk: format!("\r\n터미널 연결에 실패했습니다: {error}\r\n"),
                },
            );
        }
    });
    Ok(terminal_id)
}

fn start_terminal_process(
    profile: &ConnectionProfile,
    known_hosts_file: &Path,
    terminal_id: u64,
    app: tauri::AppHandle,
    sessions: &TerminalSessions,
) -> Result<(), String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("로컬 터미널을 열지 못했습니다: {error}"))?;
    let mut command = CommandBuilder::new("ssh");
    command.args(interactive_ssh_arguments(profile, known_hosts_file)?);
    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("SSH 터미널을 시작하지 못했습니다: {error}"))?;
    drop(pair.slave);
    let master = pair.master;
    let reader = master
        .try_clone_reader()
        .map_err(|error| format!("터미널 출력을 열지 못했습니다: {error}"))?;
    let writer = master
        .take_writer()
        .map_err(|error| format!("터미널 입력을 열지 못했습니다: {error}"))?;
    sessions
        .processes
        .lock()
        .map_err(|_| "터미널 상태를 잠글 수 없습니다.")?
        .insert(
            terminal_id,
            TerminalProcess {
                child,
                _master: master,
                writer,
            },
        );
    forward_output(reader, terminal_id, app);
    Ok(())
}

pub(crate) fn write_terminal(
    terminal_id: u64,
    input: String,
    sessions: &TerminalSessions,
) -> Result<(), String> {
    if input.len() > 16 * 1024 {
        return Err("한 번에 보낼 수 있는 터미널 입력은 16KB까지입니다.".into());
    }
    let mut processes = sessions
        .processes
        .lock()
        .map_err(|_| "터미널 상태를 잠글 수 없습니다.")?;
    let process = processes
        .get_mut(&terminal_id)
        .ok_or("이미 종료된 터미널 세션입니다.")?;
    process
        .writer
        .write_all(input.as_bytes())
        .and_then(|_| process.writer.flush())
        .map_err(|error| format!("터미널 입력을 보내지 못했습니다: {error}"))
}

pub(crate) fn stop_terminal(terminal_id: u64, sessions: &TerminalSessions) -> Result<(), String> {
    if let Some(mut process) = sessions
        .processes
        .lock()
        .map_err(|_| "터미널 상태를 잠글 수 없습니다.")?
        .remove(&terminal_id)
    {
        let _ = process.child.kill();
    }
    Ok(())
}

pub(crate) fn resize_terminal(
    terminal_id: u64,
    columns: u16,
    rows: u16,
    sessions: &TerminalSessions,
) -> Result<(), String> {
    let mut processes = sessions
        .processes
        .lock()
        .map_err(|_| "터미널 상태를 잠글 수 없습니다.")?;
    let process = processes
        .get_mut(&terminal_id)
        .ok_or("이미 종료된 터미널 세션입니다.")?;
    process
        ._master
        .resize(PtySize {
            rows: rows.clamp(1, 200),
            cols: columns.clamp(1, 400),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("터미널 크기를 조정하지 못했습니다: {error}"))
}

fn forward_output<R: Read + Send + 'static>(reader: R, terminal_id: u64, app: tauri::AppHandle) {
    thread::spawn(move || {
        let mut reader = reader;
        let mut buffer = [0_u8; 4096];
        loop {
            let count = match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => count,
            };
            let _ = app.emit(
                "terminal-chunk",
                TerminalChunk {
                    terminal_id,
                    chunk: String::from_utf8_lossy(&buffer[..count]).into_owned(),
                },
            );
        }
    });
}
