use std::{io::Write, path::Path, process::Stdio};

use crate::{
    models::{ConnectionProfile, RemoteEntry},
    ssh_client::{
        escape_remote_shell_path, reject_password_auth, sftp_command, ssh_command,
        validate_remote_path,
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

pub(crate) fn delete_entry(
    profile: &ConnectionProfile,
    path: &str,
    is_directory: bool,
    known_hosts_file: &Path,
) -> Result<(), String> {
    reject_password_auth(profile, "파일 삭제")?;
    validate_remote_path(path)?;
    let command = if is_directory { "rmdir" } else { "rm" };
    run_sftp_batch(
        profile,
        known_hosts_file,
        format!("{command} \"{}\"\nquit\n", escape_remote_shell_path(path)),
        "원격 항목을 삭제하지 못했습니다.",
    )
}

pub(crate) fn rename_entry(
    profile: &ConnectionProfile,
    path: &str,
    new_name: &str,
    known_hosts_file: &Path,
) -> Result<(), String> {
    reject_password_auth(profile, "이름 변경")?;
    validate_remote_path(path)?;
    validate_entry_name(new_name)?;
    let destination = renamed_path(path, new_name);
    run_sftp_batch(
        profile,
        known_hosts_file,
        format!(
            "rename \"{}\" \"{}\"\nquit\n",
            escape_remote_shell_path(path),
            escape_remote_shell_path(&destination)
        ),
        "원격 항목의 이름을 변경하지 못했습니다.",
    )
}

pub(crate) fn create_directory(
    profile: &ConnectionProfile,
    parent_path: &str,
    name: &str,
    known_hosts_file: &Path,
) -> Result<(), String> {
    reject_password_auth(profile, "폴더 생성")?;
    validate_remote_path(parent_path)?;
    validate_entry_name(name)?;
    let path = child_path(parent_path, name);
    run_sftp_batch(
        profile,
        known_hosts_file,
        format!("mkdir \"{}\"\nquit\n", escape_remote_shell_path(&path)),
        "원격 폴더를 생성하지 못했습니다.",
    )
}

pub(crate) fn create_empty_file(
    profile: &ConnectionProfile,
    parent_path: &str,
    name: &str,
    known_hosts_file: &Path,
) -> Result<(), String> {
    reject_password_auth(profile, "파일 생성")?;
    validate_remote_path(parent_path)?;
    validate_entry_name(name)?;
    let path = child_path(parent_path, name);
    let mut command = ssh_command(profile, known_hosts_file)?;
    command.arg(format!("touch -- \"{}\"", escape_remote_shell_path(&path)));
    let output = command
        .output()
        .map_err(|error| format!("Windows OpenSSH를 실행하지 못했습니다: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if error.is_empty() {
            "원격 파일을 생성하지 못했습니다.".into()
        } else {
            error
        })
    }
}

fn run_sftp_batch(
    profile: &ConnectionProfile,
    known_hosts_file: &Path,
    batch: String,
    fallback_message: &str,
) -> Result<(), String> {
    let mut command = sftp_command(profile, known_hosts_file)?;
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("Windows SFTP를 실행하지 못했습니다: {error}"))?;
    child
        .stdin
        .take()
        .ok_or("SFTP 입력 스트림을 열지 못했습니다.")?
        .write_all(batch.as_bytes())
        .map_err(|error| error.to_string())?;
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if error.is_empty() {
            fallback_message.into()
        } else {
            error
        })
    }
}

fn validate_entry_name(name: &str) -> Result<(), String> {
    if name.is_empty() || matches!(name, "." | "..") || name.contains(['/', '\\', '\r', '\n', '\0'])
    {
        return Err("파일 이름에는 경로 구분자나 제어 문자를 사용할 수 없습니다.".into());
    }
    Ok(())
}

fn renamed_path(path: &str, new_name: &str) -> String {
    match path.rsplit_once('/') {
        Some(("", _)) => format!("/{new_name}"),
        Some((parent, _)) => format!("{parent}/{new_name}"),
        None => new_name.to_string(),
    }
}

fn child_path(parent_path: &str, name: &str) -> String {
    if parent_path == "/" {
        format!("/{name}")
    } else {
        format!("{}/{}", parent_path.trim_end_matches('/'), name)
    }
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
    use super::{child_path, parse_listing, renamed_path, validate_entry_name};

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

    #[test]
    fn validates_new_names_and_keeps_the_parent_path() {
        assert!(validate_entry_name("renamed.log").is_ok());
        assert!(validate_entry_name("../escape").is_err());
        assert!(validate_entry_name(".").is_err());
        assert_eq!(
            renamed_path("/var/log/app.log", "api.log"),
            "/var/log/api.log"
        );
        assert_eq!(renamed_path("./app.log", "api.log"), "./api.log");
        assert_eq!(child_path("/", "new.log"), "/new.log");
    }

    #[test]
    fn rejects_unsafe_names_and_parses_root_entries_with_spaces() {
        for invalid in ["", "..", "a/b", "a\\b", "a\nb", "a\0b"] {
            assert!(validate_entry_name(invalid).is_err(), "{invalid:?}");
        }

        let listing = "total 4\n\
                       -rw-r--r-- 1 opc staff 128 Oct 25 14:01 access log.log\n\
                       invalid listing";
        let entries = parse_listing(listing, "/");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "access log.log");
        assert_eq!(entries[0].path, "/access log.log");
        assert_eq!(renamed_path("app.log", "api.log"), "api.log");
        assert_eq!(child_path("/var/log///", "new.log"), "/var/log/new.log");
    }
}
