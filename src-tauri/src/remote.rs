use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Command, Stdio},
    sync::atomic::Ordering,
    thread,
    time::Duration,
};

use tauri::{Emitter, State};

use crate::{
    models::{ConnectionCheck, ConnectionProfile, LogSource, RemoteEntry, RemoteFile, TailChunk},
    state::{AppState, TailStreams},
};

fn validate_remote_path(path: &str) -> Result<(), String> {
    if path.contains(['\r', '\n', '\0']) {
        return Err("유효하지 않은 원격 경로입니다.".into());
    }
    Ok(())
}

fn escape_remote_shell_path(path: &str) -> String {
    path.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
}

fn parse_sftp_listing(output: &str, requested_path: &str) -> Vec<RemoteEntry> {
    let base = if requested_path == "/" {
        "/".to_string()
    } else {
        requested_path.trim_end_matches('/').to_string()
    };
    output
        .lines()
        .filter_map(|line| {
            let columns: Vec<&str> = line.split_whitespace().collect();
            if columns.len() < 9
                || !(columns[0].starts_with('d')
                    || columns[0].starts_with('-')
                    || columns[0].starts_with('l'))
            {
                return None;
            }
            let name = columns[8..].join(" ");
            if name == "." || name == ".." {
                return None;
            }
            Some(RemoteEntry {
                path: if base == "/" {
                    format!("/{name}")
                } else {
                    format!("{base}/{name}")
                },
                is_dir: columns[0].starts_with('d'),
                details: line.to_string(),
                name,
            })
        })
        .collect()
}

fn validate_log_paths(paths: &[String]) -> Result<(), String> {
    if paths.len() > 10 {
        return Err("한 번에 분석할 수 있는 로그 파일은 최대 10개입니다.".into());
    }
    paths.iter().try_for_each(|path| validate_remote_path(path))
}

#[tauri::command]
pub(crate) fn check_ssh_connection(
    profile: ConnectionProfile,
    state: State<'_, AppState>,
) -> Result<ConnectionCheck, String> {
    if profile.auth_kind == "password" {
        return Ok(ConnectionCheck {
      connected: false,
      message: "비밀번호 인증의 안전한 대화형 입력은 아직 구현되지 않았습니다. 개인키 또는 ssh-agent를 선택하세요.".into(),
    });
    }

    let destination = format!("{}@{}", profile.username, profile.host);
    let mut command = Command::new("ssh");
    command
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg("-o")
        .arg("ServerAliveInterval=5")
        .arg("-o")
        .arg("StrictHostKeyChecking=accept-new")
        .arg("-o")
        .arg(format!(
            "UserKnownHostsFile={}",
            state.known_hosts_file.display()
        ))
        .arg("-p")
        .arg(profile.port.to_string());

    if profile.auth_kind == "privateKey" {
        let Some(key_path) = profile.private_key_path.as_deref() else {
            return Ok(ConnectionCheck {
                connected: false,
                message: "개인키 인증을 선택했지만 키 파일이 지정되지 않았습니다.".into(),
            });
        };
        command.arg("-i").arg(key_path);
    }

    command.arg(destination).arg("exit");
    let output = command
        .output()
        .map_err(|error| format!("Windows OpenSSH를 실행하지 못했습니다: {error}"))?;
    if output.status.success() {
        Ok(ConnectionCheck {
            connected: true,
            message: "SSH 인증을 확인했습니다. 대화형 터미널 스트림 연결은 다음 단계입니다.".into(),
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
pub(crate) fn list_remote_directory(
    profile: ConnectionProfile,
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<RemoteEntry>, String> {
    if profile.auth_kind == "password" {
        return Err("비밀번호 방식의 SFTP 입력은 아직 지원되지 않습니다. 개인키 또는 ssh-agent를 선택하세요.".into());
    }
    validate_remote_path(&path)?;

    let destination = format!("{}@{}", profile.username, profile.host);
    let mut command = Command::new("sftp");
    command
        .arg("-q")
        .arg("-b")
        .arg("-")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg("-o")
        .arg("StrictHostKeyChecking=accept-new")
        .arg("-o")
        .arg(format!(
            "UserKnownHostsFile={}",
            state.known_hosts_file.display()
        ))
        .arg("-P")
        .arg(profile.port.to_string());
    if profile.auth_kind == "privateKey" {
        let Some(key_path) = profile.private_key_path.as_deref() else {
            return Err("개인키 파일이 지정되지 않았습니다.".into());
        };
        command.arg("-i").arg(key_path);
    }
    command
        .arg(destination)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("Windows SFTP를 실행하지 못했습니다: {error}"))?;
    let escaped_path = escape_remote_shell_path(&path);
    let batch = format!("ls -l \"{escaped_path}\"\nquit\n");
    child
        .stdin
        .take()
        .ok_or("SFTP 입력 스트림을 열지 못했습니다.")?
        .write_all(batch.as_bytes())
        .map_err(|error| error.to_string())?;
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if error.is_empty() {
            "원격 디렉터리를 읽지 못했습니다.".into()
        } else {
            error
        });
    }
    Ok(parse_sftp_listing(
        &String::from_utf8_lossy(&output.stdout),
        &path,
    ))
}

#[tauri::command]
pub(crate) fn find_remote_log_files(
    profile: ConnectionProfile,
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<RemoteEntry>, String> {
    if profile.auth_kind == "password" {
        return Err("비밀번호 방식의 로그 탐색은 아직 지원되지 않습니다.".into());
    }
    validate_remote_path(&path)?;
    let escaped_path = escape_remote_shell_path(&path);
    // Production logs are commonly rotated as .log.1, .log-YYYYMMDD, etc.
    // Limit both tree depth and result count to keep the UI responsive.
    let remote_command = format!(
        "find -- \"{escaped_path}\" -maxdepth 3 -type f \\(-iname '*.log' -o -iname '*.log.*' -o -iname '*.log-*' \\) -print | head -n 200"
    );
    let mut command = Command::new("ssh");
    append_ssh_auth_args(&mut command, &profile, &state.known_hosts_file)?;
    command
        .arg(format!("{}@{}", profile.username, profile.host))
        .arg(remote_command);
    let output = command
        .output()
        .map_err(|error| format!("Windows OpenSSH를 실행하지 못했습니다: {error}"))?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if error.is_empty() {
            "원격 로그 파일을 찾지 못했습니다.".into()
        } else {
            error
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|item| !item.is_empty())
        .map(|item| RemoteEntry {
            name: item.rsplit('/').next().unwrap_or(item).to_string(),
            path: item.to_string(),
            is_dir: false,
            details: "로그 파일".into(),
        })
        .collect())
}

fn append_ssh_auth_args(
    command: &mut Command,
    profile: &ConnectionProfile,
    known_hosts_file: &Path,
) -> Result<(), String> {
    command
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("ConnectTimeout=10")
        .arg("-o")
        .arg("StrictHostKeyChecking=accept-new")
        .arg("-o")
        .arg(format!("UserKnownHostsFile={}", known_hosts_file.display()))
        .arg("-p")
        .arg(profile.port.to_string());
    if profile.auth_kind == "privateKey" {
        let Some(key_path) = profile.private_key_path.as_deref() else {
            return Err("개인키 파일이 지정되지 않았습니다.".into());
        };
        command.arg("-i").arg(key_path);
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn read_remote_file(
    profile: ConnectionProfile,
    path: String,
    state: State<'_, AppState>,
) -> Result<RemoteFile, String> {
    if profile.auth_kind == "password" {
        return Err("비밀번호 방식의 파일 열기는 아직 지원되지 않습니다.".into());
    }
    validate_remote_path(&path)?;
    let escaped_path = escape_remote_shell_path(&path);
    let remote_command = format!("tail -n 100 -- \"{escaped_path}\"");
    let mut command = Command::new("ssh");
    append_ssh_auth_args(&mut command, &profile, &state.known_hosts_file)?;
    command
        .arg(format!("{}@{}", profile.username, profile.host))
        .arg(remote_command);
    let output = command
        .output()
        .map_err(|error| format!("Windows OpenSSH를 실행하지 못했습니다: {error}"))?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if error.is_empty() {
            "원격 파일을 열지 못했습니다.".into()
        } else {
            error
        });
    }
    let truncated = false;
    Ok(RemoteFile {
        path,
        content: String::from_utf8_lossy(&output.stdout).to_string(),
        truncated,
    })
}

#[tauri::command]
pub(crate) fn read_remote_log_files(
    profile: ConnectionProfile,
    paths: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<LogSource>, String> {
    if profile.auth_kind == "password" {
        return Err("비밀번호 방식의 로그 분석은 아직 지원되지 않습니다.".into());
    }
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    validate_log_paths(&paths)?;

    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        let escaped_path = escape_remote_shell_path(&path);
        // Keep analysis bounded: enough recent context for patterns without
        // transferring an entire production log into the webview.
        let remote_command = format!("tail -n 5000 -- \"{escaped_path}\"");
        let mut command = Command::new("ssh");
        append_ssh_auth_args(&mut command, &profile, &state.known_hosts_file)?;
        command
            .arg(format!("{}@{}", profile.username, profile.host))
            .arg(remote_command);
        let output = command
            .output()
            .map_err(|error| format!("Windows OpenSSH를 실행하지 못했습니다: {error}"))?;
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if error.is_empty() {
                format!("원격 로그 파일을 읽지 못했습니다: {path}")
            } else {
                error
            });
        }
        files.push(LogSource {
            path,
            content: String::from_utf8_lossy(&output.stdout).to_string(),
        });
    }
    Ok(files)
}

#[tauri::command]
pub(crate) fn start_tail_stream(
    profile: ConnectionProfile,
    path: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    streams: State<'_, TailStreams>,
) -> Result<u64, String> {
    if profile.auth_kind == "password" {
        return Err("비밀번호 방식의 실시간 로그 추적은 아직 지원되지 않습니다.".into());
    }
    validate_remote_path(&path)?;
    let escaped_path = escape_remote_shell_path(&path);
    // The UI loads the initial 100 lines separately, then this persistent stream
    // sends only lines appended after the stream begins.
    let remote_command = format!("tail -n 0 -F -- \"{escaped_path}\"");
    let mut command = Command::new("ssh");
    append_ssh_auth_args(&mut command, &profile, &state.known_hosts_file)?;
    command
        .arg(format!("{}@{}", profile.username, profile.host))
        .arg(remote_command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("실시간 로그 스트림을 시작하지 못했습니다: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("로그 스트림 출력을 열지 못했습니다.")?;
    let stream_id = streams.next_id.fetch_add(1, Ordering::Relaxed);
    streams
        .children
        .lock()
        .map_err(|_| "로그 스트림 상태를 잠글 수 없습니다.")?
        .insert(stream_id, child);
    thread::spawn(move || {
        // Give the webview event listener time to subscribe before delivering lines.
        thread::sleep(Duration::from_millis(200));
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let chunk = format!("{line}\n");
                    let _ = app.emit("tail-chunk", TailChunk { stream_id, chunk });
                }
                Err(_) => break,
            }
        }
    });
    Ok(stream_id)
}

#[tauri::command]
pub(crate) fn stop_tail_stream(
    stream_id: u64,
    streams: State<'_, TailStreams>,
) -> Result<(), String> {
    if let Some(mut child) = streams
        .children
        .lock()
        .map_err(|_| "로그 스트림 상태를 잠글 수 없습니다.")?
        .remove(&stream_id)
    {
        let _ = child.kill();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        escape_remote_shell_path, parse_sftp_listing, validate_log_paths, validate_remote_path,
    };

    #[test]
    fn parses_sftp_listing_and_skips_dot_entries() {
        let listing = "drwxr-xr-x 2 opc staff 4096 Oct 25 14:00 .\n\
                       drwxr-xr-x 2 opc staff 4096 Oct 25 14:00 ..\n\
                       -rw-r--r-- 1 opc staff  128 Oct 25 14:01 app.log\n\
                       drwxr-xr-x 2 opc staff 4096 Oct 25 14:02 archive";

        let entries = parse_sftp_listing(listing, "/var/log/");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "app.log");
        assert_eq!(entries[0].path, "/var/log/app.log");
        assert!(!entries[0].is_dir);
        assert_eq!(entries[1].path, "/var/log/archive");
        assert!(entries[1].is_dir);
    }

    #[test]
    fn rejects_unsafe_remote_paths_and_escapes_shell_metacharacters() {
        assert!(validate_remote_path("/var/log/app.log").is_ok());
        assert!(validate_remote_path("/var/log/evil\ncommand").is_err());
        assert!(validate_remote_path("/var/log/evil\0name").is_err());
        assert_eq!(
            escape_remote_shell_path("/var/log/$app`date`.log"),
            "/var/log/\\$app\\`date\\`.log"
        );
    }

    #[test]
    fn limits_log_analysis_to_ten_safe_paths() {
        let allowed = (0..10)
            .map(|index| format!("/var/log/{index}.log"))
            .collect::<Vec<_>>();
        let too_many = (0..11)
            .map(|index| format!("/var/log/{index}.log"))
            .collect::<Vec<_>>();

        assert!(validate_log_paths(&allowed).is_ok());
        assert!(validate_log_paths(&too_many).is_err());
    }
}
