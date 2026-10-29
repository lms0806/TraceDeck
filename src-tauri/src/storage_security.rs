use std::path::Path;

pub(crate) fn restrict_profile_file(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    restrict_profile_file_on_windows(path)
}

#[cfg(target_os = "windows")]
fn restrict_profile_file_on_windows(path: &Path) -> Result<(), String> {
    use std::process::Command;

    let account = Command::new("whoami")
        .output()
        .map_err(|error| format!("현재 Windows 사용자를 확인하지 못했습니다: {error}"))?;
    if !account.status.success() {
        return Err("현재 Windows 사용자를 확인하지 못했습니다.".into());
    }
    let account = String::from_utf8_lossy(&account.stdout).trim().to_string();
    if account.is_empty() {
        return Err("현재 Windows 사용자를 확인하지 못했습니다.".into());
    }
    let permission = format!("{account}:(F)");
    let result = Command::new("icacls")
        .arg(path)
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(permission)
        .output()
        .map_err(|error| format!("프로필 파일 권한을 제한하지 못했습니다: {error}"))?;
    if result.status.success() {
        Ok(())
    } else {
        Err(format!(
            "프로필 파일 권한을 제한하지 못했습니다: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ))
    }
}

#[cfg(not(target_os = "windows"))]
fn restrict_profile_file_on_windows(_path: &Path) -> Result<(), String> {
    Ok(())
}
