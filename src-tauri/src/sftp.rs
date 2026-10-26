use std::{io::Write, path::Path, process::Stdio};

use crate::{
    models::{ConnectionProfile, RemoteEntry},
    ssh_client::{
        escape_remote_shell_path, reject_password_auth, sftp_command, validate_remote_path,
    },
};

pub(crate) fn list_directory(
    profile: &ConnectionProfile,
    path: &str,
    known_hosts_file: &Path,
) -> Result<Vec<RemoteEntry>, String> {
    reject_password_auth(profile, "SFTP 입력")?;
    validate_remote_path(path)?;

    let mut command = sftp_command(profile, known_hosts_file)?;
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("Windows SFTP를 실행하지 못했습니다: {error}"))?;
    let batch = format!("ls -l \"{}\"\nquit\n", escape_remote_shell_path(path));
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
    Ok(parse_listing(
        &String::from_utf8_lossy(&output.stdout),
        path,
    ))
}

pub(crate) fn parse_listing(output: &str, requested_path: &str) -> Vec<RemoteEntry> {
    let base = if requested_path == "/" {
        "/"
    } else {
        requested_path.trim_end_matches('/')
    };
    output
        .lines()
        .filter_map(|line| {
            let columns: Vec<&str> = line.split_whitespace().collect();
            if columns.len() < 9
                || !matches!(columns[0].as_bytes().first(), Some(b'd' | b'-' | b'l'))
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

#[cfg(test)]
mod tests {
    use super::parse_listing;

    #[test]
    fn parses_sftp_listing_and_skips_dot_entries() {
        let listing = "drwxr-xr-x 2 opc staff 4096 Oct 25 14:00 .\n\
                       drwxr-xr-x 2 opc staff 4096 Oct 25 14:00 ..\n\
                       -rw-r--r-- 1 opc staff  128 Oct 25 14:01 app.log\n\
                       drwxr-xr-x 2 opc staff 4096 Oct 25 14:02 archive";
        let entries = parse_listing(listing, "/var/log/");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "/var/log/app.log");
        assert!(!entries[0].is_dir);
        assert!(entries[1].is_dir);
    }
}
