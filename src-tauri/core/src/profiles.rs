use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub group: String,
    #[serde(default = "default_auth_kind")]
    pub auth_kind: String,
    #[serde(default)]
    pub private_key_path: Option<String>,
}

fn default_auth_kind() -> String {
    "privateKey".to_string()
}

/// Persists connection profiles without depending on a desktop UI runtime.
#[derive(Debug, Clone)]
pub struct ProfileStore {
    profile_file: PathBuf,
}

impl ProfileStore {
    pub fn new(profile_file: PathBuf) -> Self {
        Self { profile_file }
    }

    pub fn profile_file(&self) -> &Path {
        &self.profile_file
    }

    pub fn list(&self) -> Result<Vec<ConnectionProfile>, String> {
        restrict_profile_file(&self.profile_file)?;
        if !self.profile_file.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&self.profile_file).map_err(|error| error.to_string())?;
        serde_json::from_str(&content).map_err(|error| error.to_string())
    }

    pub fn save(&self, profile: ConnectionProfile) -> Result<(), String> {
        let mut profiles = self.list()?;
        upsert_profile(&mut profiles, profile);
        self.write(&profiles)
    }

    pub fn delete(&self, profile_id: &str) -> Result<(), String> {
        let mut profiles = self.list()?;
        profiles.retain(|profile| profile.id != profile_id);
        self.write(&profiles)
    }

    fn write(&self, profiles: &[ConnectionProfile]) -> Result<(), String> {
        let content = serde_json::to_string_pretty(profiles).map_err(|error| error.to_string())?;
        fs::write(&self.profile_file, content).map_err(|error| error.to_string())?;
        restrict_profile_file(&self.profile_file)
    }
}

fn upsert_profile(profiles: &mut Vec<ConnectionProfile>, profile: ConnectionProfile) {
    if let Some(index) = profiles.iter().position(|item| item.id == profile.id) {
        profiles[index] = profile;
    } else {
        profiles.push(profile);
    }
}

fn restrict_profile_file(path: &Path) -> Result<(), String> {
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
    let result = Command::new("icacls")
        .arg(path)
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(format!("{account}:(F)"))
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

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{ConnectionProfile, ProfileStore};

    fn profile(id: &str, name: &str) -> ConnectionProfile {
        ConnectionProfile {
            id: id.into(),
            name: name.into(),
            host: "127.0.0.1".into(),
            port: 22,
            username: "tester".into(),
            group: "Test".into(),
            auth_kind: "privateKey".into(),
            private_key_path: None,
        }
    }

    #[test]
    fn persists_updates_and_deletes_profiles() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let file = std::env::temp_dir().join(format!("trace-deck-profiles-{unique}.json"));
        let store = ProfileStore::new(file.clone());

        store.save(profile("one", "first")).unwrap();
        store.save(profile("one", "updated")).unwrap();
        store.save(profile("two", "second")).unwrap();
        assert_eq!(store.list().unwrap().len(), 2);
        assert_eq!(store.list().unwrap()[0].name, "updated");

        store.delete("one").unwrap();
        assert_eq!(store.list().unwrap().len(), 1);
        fs::remove_file(file).unwrap();
    }

    #[test]
    fn preserves_the_existing_profile_json_shape() {
        let profile: ConnectionProfile = serde_json::from_str(
            r#"{
                "id":"prod",
                "name":"Production",
                "host":"prod.example.test",
                "port":22,
                "username":"deploy",
                "group":"Production"
            }"#,
        )
        .unwrap();
        assert_eq!(profile.auth_kind, "privateKey");
        assert!(profile.private_key_path.is_none());

        let json = serde_json::to_value(profile).unwrap();
        assert_eq!(json["authKind"], "privateKey");
        assert!(json.get("privateKeyPath").is_some());
    }
}
