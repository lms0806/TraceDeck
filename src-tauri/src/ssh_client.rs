use std::{path::Path, process::Command};

use crate::models::ConnectionProfile;

pub(crate) fn validate_remote_path(path: &str) -> Result<(), String> {
    if path.contains(['\r', '\n', '\0']) {
        return Err("유효하지 않은 원격 경로입니다.".into());
    }
    Ok(())
}

pub(crate) fn escape_remote_shell_path(path: &str) -> String {
    path.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
}

pub(crate) fn reject_password_auth(
    profile: &ConnectionProfile,
    action: &str,
) -> Result<(), String> {
    if profile.auth_kind == "password" {
        return Err(format!(
            "비밀번호 방식의 {action}은 아직 지원되지 않습니다."
        ));
    }
    Ok(())
}

pub(crate) fn destination(profile: &ConnectionProfile) -> String {
    format!("{}@{}", profile.username, profile.host)
}

pub(crate) fn append_ssh_auth_args(
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

pub(crate) fn ssh_command(
    profile: &ConnectionProfile,
    known_hosts_file: &Path,
) -> Result<Command, String> {
    let mut command = Command::new("ssh");
    hide_console_window(&mut command);
    append_ssh_auth_args(&mut command, profile, known_hosts_file)?;
    command.arg(destination(profile));
    Ok(command)
}

pub(crate) fn interactive_ssh_arguments(
    profile: &ConnectionProfile,
    known_hosts_file: &Path,
) -> Result<Vec<String>, String> {
    let mut arguments = vec![
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "ConnectTimeout=10".into(),
        "-o".into(),
        "StrictHostKeyChecking=accept-new".into(),
        "-o".into(),
        format!("UserKnownHostsFile={}", known_hosts_file.display()),
        "-p".into(),
        profile.port.to_string(),
        "-tt".into(),
    ];
    if profile.auth_kind == "privateKey" {
        let Some(key_path) = profile.private_key_path.as_deref() else {
            return Err("개인키 파일이 지정되지 않았습니다.".into());
        };
        arguments.push("-i".into());
        arguments.push(key_path.into());
    }
    arguments.push(destination(profile));
    Ok(arguments)
}

pub(crate) fn sftp_command(
    profile: &ConnectionProfile,
    known_hosts_file: &Path,
) -> Result<Command, String> {
    let mut command = Command::new("sftp");
    hide_console_window(&mut command);
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
        .arg(format!("UserKnownHostsFile={}", known_hosts_file.display()))
        .arg("-P")
        .arg(profile.port.to_string());
    if profile.auth_kind == "privateKey" {
        let Some(key_path) = profile.private_key_path.as_deref() else {
            return Err("개인키 파일이 지정되지 않았습니다.".into());
        };
        command.arg("-i").arg(key_path);
    }
    command.arg(destination(profile));
    Ok(command)
}

#[cfg(target_os = "windows")]
fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
fn hide_console_window(_command: &mut Command) {}
