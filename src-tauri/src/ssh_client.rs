use std::{path::Path, process::Command};

use crate::models::ConnectionProfile;

const STRICT_HOST_KEY_CHECKING: &str = "StrictHostKeyChecking=yes";

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
        .arg(STRICT_HOST_KEY_CHECKING)
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
        STRICT_HOST_KEY_CHECKING.into(),
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
        .arg(STRICT_HOST_KEY_CHECKING)
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        destination, escape_remote_shell_path, interactive_ssh_arguments, reject_password_auth,
        sftp_command, ssh_command, validate_remote_path,
    };
    use crate::models::ConnectionProfile;

    fn profile(auth_kind: &str, private_key_path: Option<&str>) -> ConnectionProfile {
        ConnectionProfile {
            id: "test".into(),
            name: "Test".into(),
            host: "example.test".into(),
            port: 2200,
            username: "deploy".into(),
            group: "Test".into(),
            auth_kind: auth_kind.into(),
            private_key_path: private_key_path.map(str::to_string),
        }
    }

    #[test]
    fn validates_and_escapes_remote_paths() {
        assert!(validate_remote_path("/var/log/app.log").is_ok());
        for invalid in ["line\nbreak", "line\rbreak", "nul\0byte"] {
            assert!(validate_remote_path(invalid).is_err(), "{invalid:?}");
        }
        assert_eq!(
            escape_remote_shell_path(r#"/var/log/a\\b\"$app`date`.log"#),
            r#"/var/log/a\\\\b\\\"\$app\`date\`.log"#
        );
    }

    #[test]
    fn builds_private_key_and_agent_ssh_arguments() {
        let known_hosts = Path::new("known_hosts");
        let key_profile = profile("privateKey", Some("C:/keys/deploy"));
        let arguments = interactive_ssh_arguments(&key_profile, known_hosts).unwrap();

        assert_eq!(destination(&key_profile), "deploy@example.test");
        assert!(
            arguments
                .iter()
                .any(|argument| argument == "StrictHostKeyChecking=yes")
        );
        assert!(
            !arguments
                .iter()
                .any(|argument| argument == "StrictHostKeyChecking=accept-new")
        );
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["-i", "C:/keys/deploy"])
        );
        assert_eq!(arguments.last(), Some(&"deploy@example.test".to_string()));

        let agent_arguments =
            interactive_ssh_arguments(&profile("agent", None), known_hosts).unwrap();
        assert!(!agent_arguments.iter().any(|argument| argument == "-i"));
        assert!(ssh_command(&profile("agent", None), known_hosts).is_ok());
        assert!(sftp_command(&profile("agent", None), known_hosts).is_ok());
    }

    #[test]
    fn rejects_unsupported_or_incomplete_authentication() {
        let password = profile("password", None);
        assert!(
            reject_password_auth(&password, "SSH 인증")
                .unwrap_err()
                .contains("비밀번호")
        );
        assert!(
            interactive_ssh_arguments(&profile("privateKey", None), Path::new("known_hosts"))
                .is_err()
        );
        assert!(reject_password_auth(&profile("agent", None), "SSH 인증").is_ok());
    }
}
