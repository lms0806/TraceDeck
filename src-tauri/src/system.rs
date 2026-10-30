use std::{path::Path, process::Output};

use crate::{
    models::{ConnectionProfile, RemoteSystemStatus},
    ssh_client::{reject_password_auth, ssh_command},
};

pub(crate) fn read_remote_system_status(
    profile: &ConnectionProfile,
    known_hosts_file: &Path,
) -> Result<RemoteSystemStatus, String> {
    reject_password_auth(profile, "서버 상태 확인")?;
    let mut command = ssh_command(profile, known_hosts_file)?;
    command.arg(
        "LC_ALL=C head -n 1 /proc/stat; sleep 0.4; LC_ALL=C head -n 1 /proc/stat; LC_ALL=C df -Pk /",
    );
    let output = command
        .output()
        .map_err(|error| format!("Windows OpenSSH를 실행하지 못했습니다: {error}"))?;
    parse_system_status(output)
}

fn parse_system_status(output: Output) -> Result<RemoteSystemStatus, String> {
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if error.is_empty() {
            "원격 서버 상태를 읽지 못했습니다.".into()
        } else {
            error
        });
    }
    parse_system_status_text(&String::from_utf8_lossy(&output.stdout))
}

fn parse_system_status_text(output: &str) -> Result<RemoteSystemStatus, String> {
    let mut lines = output.lines();
    let before = parse_cpu_line(lines.next().ok_or("CPU 정보를 받지 못했습니다.")?)?;
    let after = parse_cpu_line(lines.next().ok_or("CPU 정보를 받지 못했습니다.")?)?;
    let disk_line = lines
        .find(|line| !line.starts_with("Filesystem") && !line.trim().is_empty())
        .ok_or("디스크 정보를 받지 못했습니다.")?;
    let (disk_used_kb, disk_total_kb, disk_percent) = parse_disk_line(disk_line)?;

    let total_delta = after.0.saturating_sub(before.0);
    let idle_delta = after.1.saturating_sub(before.1);
    let cpu_percent = if total_delta == 0 {
        0.0
    } else {
        100.0 * (total_delta.saturating_sub(idle_delta)) as f64 / total_delta as f64
    };
    Ok(RemoteSystemStatus {
        cpu_percent,
        disk_used_kb,
        disk_total_kb,
        disk_percent,
    })
}

fn parse_cpu_line(line: &str) -> Result<(u64, u64), String> {
    let values = line
        .split_whitespace()
        .skip(1)
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "CPU 정보를 해석하지 못했습니다.".to_string())?;
    if values.len() < 4 {
        return Err("CPU 정보를 해석하지 못했습니다.".into());
    }
    let total = values.iter().sum();
    let idle = values[3] + values.get(4).copied().unwrap_or(0);
    Ok((total, idle))
}

fn parse_disk_line(line: &str) -> Result<(u64, u64, f64), String> {
    let fields = line.split_whitespace().collect::<Vec<_>>();
    if fields.len() < 5 {
        return Err("디스크 정보를 해석하지 못했습니다.".into());
    }
    let disk_total_kb = fields[1]
        .parse()
        .map_err(|_| "디스크 정보를 해석하지 못했습니다.".to_string())?;
    let disk_used_kb = fields[2]
        .parse()
        .map_err(|_| "디스크 정보를 해석하지 못했습니다.".to_string())?;
    let disk_percent = fields[4]
        .trim_end_matches('%')
        .parse()
        .map_err(|_| "디스크 정보를 해석하지 못했습니다.".to_string())?;
    Ok((disk_used_kb, disk_total_kb, disk_percent))
}

#[cfg(test)]
mod tests {
    use super::parse_system_status_text;

    #[test]
    fn parses_cpu_and_root_disk_status() {
        let result = parse_system_status_text(
            "cpu  100 20 30 800 10 0 0 0 0 0\ncpu  130 20 40 850 10 0 0 0 0 0\nFilesystem 1024-blocks Used Available Capacity Mounted on\n/dev/sda1 100000 25000 75000 25% /\n",
        )
        .expect("status parses");

        assert_eq!(result.disk_used_kb, 25_000);
        assert_eq!(result.disk_total_kb, 100_000);
        assert_eq!(result.disk_percent, 25.0);
        assert!((result.cpu_percent - 44.444_444).abs() < 0.000_001);
    }
}
