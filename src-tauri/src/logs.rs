use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::Stdio,
    sync::atomic::Ordering,
    thread,
    time::Duration,
};

use tauri::Emitter;

use crate::{
    models::{ConnectionProfile, LogSource, RemoteEntry, RemoteFile, TailChunk},
    ssh_client::{
        escape_remote_shell_path, reject_password_auth, ssh_command, validate_remote_path,
    },
    state::TailStreams,
};

pub(crate) fn find_log_files(
    profile: &ConnectionProfile,
    path: &str,
    known_hosts_file: &Path,
) -> Result<Vec<RemoteEntry>, String> {
    reject_password_auth(profile, "로그 탐색")?;
    validate_remote_path(path)?;
    let remote_command = format!(
        "find -- \"{}\" -maxdepth 3 -type f \\(-iname '*.log' -o -iname '*.log.*' -o -iname '*.log-*' \\) -print | head -n 200",
        escape_remote_shell_path(path)
    );
    let output = run_ssh(profile, known_hosts_file, remote_command)?;
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

pub(crate) fn read_recent_file(
    profile: &ConnectionProfile,
    path: String,
    known_hosts_file: &Path,
) -> Result<RemoteFile, String> {
    reject_password_auth(profile, "파일 열기")?;
    validate_remote_path(&path)?;
    let output = run_ssh(
        profile,
        known_hosts_file,
        format!("tail -n 100 -- \"{}\"", escape_remote_shell_path(&path)),
    )?;
    Ok(RemoteFile {
        path,
        content: String::from_utf8_lossy(&output.stdout).to_string(),
        truncated: false,
    })
}

pub(crate) fn read_for_analysis(
    profile: &ConnectionProfile,
    paths: Vec<String>,
    known_hosts_file: &Path,
) -> Result<Vec<LogSource>, String> {
    reject_password_auth(profile, "로그 분석")?;
    validate_analysis_paths(&paths)?;
    paths
        .into_iter()
        .map(|path| {
            validate_remote_path(&path)?;
            let output = run_ssh(
                profile,
                known_hosts_file,
                format!("tail -n 5000 -- \"{}\"", escape_remote_shell_path(&path)),
            )?;
            Ok(LogSource {
                path,
                content: String::from_utf8_lossy(&output.stdout).to_string(),
            })
        })
        .collect()
}

fn validate_analysis_paths(paths: &[String]) -> Result<(), String> {
    if paths.len() > 10 {
        return Err("한 번에 분석할 수 있는 로그 파일은 최대 10개입니다.".into());
    }
    paths.iter().try_for_each(|path| validate_remote_path(path))
}

pub(crate) fn start_tail_stream(
    profile: &ConnectionProfile,
    path: &str,
    known_hosts_file: &Path,
    app: tauri::AppHandle,
    streams: &TailStreams,
) -> Result<u64, String> {
    reject_password_auth(profile, "실시간 로그 추적")?;
    validate_remote_path(path)?;
    let mut command = ssh_command(profile, known_hosts_file)?;
    command
        .arg(format!(
            "tail -n 0 -F -- \"{}\"",
            escape_remote_shell_path(path)
        ))
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
        thread::sleep(Duration::from_millis(200));
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) => {
                    let _ = app.emit(
                        "tail-chunk",
                        TailChunk {
                            stream_id,
                            chunk: format!("{line}\n"),
                        },
                    );
                }
                Err(_) => break,
            }
        }
    });
    Ok(stream_id)
}

pub(crate) fn stop_tail_stream(stream_id: u64, streams: &TailStreams) -> Result<(), String> {
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

fn run_ssh(
    profile: &ConnectionProfile,
    known_hosts_file: &Path,
    remote_command: String,
) -> Result<std::process::Output, String> {
    let mut command = ssh_command(profile, known_hosts_file)?;
    command.arg(remote_command);
    let output = command
        .output()
        .map_err(|error| format!("Windows OpenSSH를 실행하지 못했습니다: {error}"))?;
    if output.status.success() {
        Ok(output)
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if error.is_empty() {
            "원격 명령을 실행하지 못했습니다.".into()
        } else {
            error
        })
    }
}

#[cfg(test)]
mod tests {
    use super::validate_analysis_paths;
    use crate::ssh_client::{escape_remote_shell_path, validate_remote_path};

    #[test]
    fn rejects_unsafe_paths_and_escapes_shell_metacharacters() {
        assert!(validate_remote_path("/var/log/app.log").is_ok());
        assert!(validate_remote_path("/var/log/evil\ncommand").is_err());
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

        assert!(validate_analysis_paths(&allowed).is_ok());
        assert!(validate_analysis_paths(&too_many).is_err());
    }
}
