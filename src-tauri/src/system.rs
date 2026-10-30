use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::Stdio,
    sync::atomic::Ordering,
    thread,
};

use tauri::Emitter;

use crate::{
    models::{ConnectionProfile, RemoteSystemStatus, SystemStatusChunk},
    ssh_client::{reject_password_auth, ssh_command},
    state::SystemStreams,
};

const STATUS_COMMAND: &str = r#"prev_total=0
prev_idle=0
while :; do
  set -- $(head -n 1 /proc/stat)
  shift
  total=0
  idle=0
  index=0
  for value in "$@"; do
    total=$((total + value))
    [ "$index" -eq 3 ] && idle=$value
    [ "$index" -eq 4 ] && idle=$((idle + value))
    index=$((index + 1))
  done
  set -- $(LC_ALL=C df -Pk / | tail -n 1)
  disk_total=$2
  disk_used=$3
  disk_percent=$5
  if [ "$prev_total" -eq 0 ]; then
    cpu=0
  else
    total_delta=$((total - prev_total))
    idle_delta=$((idle - prev_idle))
    if [ "$total_delta" -eq 0 ]; then cpu=0; else cpu=$((100 * (total_delta - idle_delta) / total_delta)); fi
  fi
  printf 'STATUS %s %s %s %s\n' "$cpu" "$disk_used" "$disk_total" "$disk_percent"
  prev_total=$total
  prev_idle=$idle
  sleep 2
done"#;

pub(crate) fn start_remote_system_stream(
    profile: &ConnectionProfile,
    known_hosts_file: &Path,
    app: tauri::AppHandle,
    streams: &SystemStreams,
) -> Result<u64, String> {
    reject_password_auth(profile, "서버 상태 확인")?;
    let mut command = ssh_command(profile, known_hosts_file)?;
    command
        .arg(STATUS_COMMAND)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|error| format!("서버 상태 스트림을 시작하지 못했습니다: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("서버 상태 스트림 출력을 열지 못했습니다.")?;
    let stream_id = streams.next_id.fetch_add(1, Ordering::Relaxed);
    streams
        .children
        .lock()
        .map_err(|_| "서버 상태 스트림을 잠글 수 없습니다.")?
        .insert(stream_id, child);
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            let Ok(status) = parse_status_line(&line) else {
                continue;
            };
            let _ = app.emit(
                "system-status-chunk",
                SystemStatusChunk { stream_id, status },
            );
        }
    });
    Ok(stream_id)
}

pub(crate) fn stop_remote_system_stream(
    stream_id: u64,
    streams: &SystemStreams,
) -> Result<(), String> {
    if let Some(mut child) = streams
        .children
        .lock()
        .map_err(|_| "서버 상태 스트림을 잠글 수 없습니다.")?
        .remove(&stream_id)
    {
        let _ = child.kill();
    }
    Ok(())
}

fn parse_status_line(line: &str) -> Result<RemoteSystemStatus, String> {
    let fields = line.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 5 || fields[0] != "STATUS" {
        return Err("서버 상태를 해석하지 못했습니다.".into());
    }
    Ok(RemoteSystemStatus {
        cpu_percent: fields[1]
            .parse()
            .map_err(|_| "서버 상태를 해석하지 못했습니다.".to_string())?,
        disk_used_kb: fields[2]
            .parse()
            .map_err(|_| "서버 상태를 해석하지 못했습니다.".to_string())?,
        disk_total_kb: fields[3]
            .parse()
            .map_err(|_| "서버 상태를 해석하지 못했습니다.".to_string())?,
        disk_percent: fields[4]
            .trim_end_matches('%')
            .parse()
            .map_err(|_| "서버 상태를 해석하지 못했습니다.".to_string())?,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_status_line;

    #[test]
    fn parses_streamed_cpu_and_root_disk_status() {
        let result = parse_status_line("STATUS 44 25000 100000 25").expect("status parses");

        assert_eq!(result.cpu_percent, 44.0);
        assert_eq!(result.disk_used_kb, 25_000);
        assert_eq!(result.disk_total_kb, 100_000);
        assert_eq!(result.disk_percent, 25.0);
    }
}
