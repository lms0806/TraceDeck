use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};
use tauri::{Emitter, Manager, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionProfile {
    id: String,
    name: String,
    host: String,
    port: u16,
    username: String,
    group: String,
    #[serde(default = "default_auth_kind")]
    auth_kind: String,
    #[serde(default)]
    private_key_path: Option<String>,
}

fn default_auth_kind() -> String {
    "privateKey".to_string()
}

struct AppState {
    profile_file: PathBuf,
    known_hosts_file: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionCheck {
    connected: bool,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteEntry {
    name: String,
    path: String,
    is_dir: bool,
    details: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteFile {
    path: String,
    content: String,
    truncated: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TailChunk {
    stream_id: u64,
    chunk: String,
}

struct TailStreams {
    children: Mutex<HashMap<u64, Child>>,
    next_id: AtomicU64,
}

#[tauri::command]
fn list_profiles(state: State<'_, AppState>) -> Result<Vec<ConnectionProfile>, String> {
    if !state.profile_file.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&state.profile_file).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_profile(profile: ConnectionProfile, state: State<'_, AppState>) -> Result<(), String> {
    let mut profiles = list_profiles(state.clone())?;
    if let Some(index) = profiles.iter().position(|item| item.id == profile.id) {
        profiles[index] = profile;
    } else {
        profiles.push(profile);
    }
    let content = serde_json::to_string_pretty(&profiles).map_err(|error| error.to_string())?;
    fs::write(&state.profile_file, content).map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_profile(profile_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut profiles = list_profiles(state.clone())?;
    profiles.retain(|profile| profile.id != profile_id);
    let content = serde_json::to_string_pretty(&profiles).map_err(|error| error.to_string())?;
    fs::write(&state.profile_file, content).map_err(|error| error.to_string())
}

#[tauri::command]
fn check_ssh_connection(
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
fn list_remote_directory(
    profile: ConnectionProfile,
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<RemoteEntry>, String> {
    if profile.auth_kind == "password" {
        return Err("비밀번호 방식의 SFTP 입력은 아직 지원되지 않습니다. 개인키 또는 ssh-agent를 선택하세요.".into());
    }
    if path.contains(['\r', '\n']) {
        return Err("유효하지 않은 원격 경로입니다.".into());
    }

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
    let escaped_path = path.replace('\\', "\\\\").replace('"', "\\\"");
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
    let base = if path == "/" {
        "/".to_string()
    } else {
        path.trim_end_matches('/').to_string()
    };
    let entries = String::from_utf8_lossy(&output.stdout)
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
        .collect();
    Ok(entries)
}

fn append_ssh_auth_args(
    command: &mut Command,
    profile: &ConnectionProfile,
    known_hosts_file: &PathBuf,
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
fn read_remote_file(
    profile: ConnectionProfile,
    path: String,
    state: State<'_, AppState>,
) -> Result<RemoteFile, String> {
    if profile.auth_kind == "password" {
        return Err("비밀번호 방식의 파일 열기는 아직 지원되지 않습니다.".into());
    }
    if path.contains(['\r', '\n', '\0']) {
        return Err("유효하지 않은 원격 경로입니다.".into());
    }
    let escaped_path = path
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`");
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
fn start_tail_stream(
    profile: ConnectionProfile,
    path: String,
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    streams: State<'_, TailStreams>,
) -> Result<u64, String> {
    if profile.auth_kind == "password" {
        return Err("비밀번호 방식의 실시간 로그 추적은 아직 지원되지 않습니다.".into());
    }
    if path.contains(['\r', '\n', '\0']) {
        return Err("유효하지 않은 원격 경로입니다.".into());
    }
    let escaped_path = path
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`");
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
fn stop_tail_stream(stream_id: u64, streams: State<'_, TailStreams>) -> Result<(), String> {
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
            app.manage(TailStreams {
                children: Mutex::new(HashMap::new()),
                next_id: AtomicU64::new(1),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_profiles,
            save_profile,
            delete_profile,
            check_ssh_connection,
            list_remote_directory,
            read_remote_file,
            start_tail_stream,
            stop_tail_stream
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tracing Log");
}
