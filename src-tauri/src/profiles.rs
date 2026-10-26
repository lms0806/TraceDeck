use std::{fs, path::Path};

use tauri::State;

use crate::{models::ConnectionProfile, state::AppState};

fn read_profiles(profile_file: &Path) -> Result<Vec<ConnectionProfile>, String> {
    if !profile_file.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(profile_file).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

fn write_profiles(profile_file: &Path, profiles: &[ConnectionProfile]) -> Result<(), String> {
    let content = serde_json::to_string_pretty(profiles).map_err(|error| error.to_string())?;
    fs::write(profile_file, content).map_err(|error| error.to_string())
}

fn upsert_profile(profiles: &mut Vec<ConnectionProfile>, profile: ConnectionProfile) {
    if let Some(index) = profiles.iter().position(|item| item.id == profile.id) {
        profiles[index] = profile;
    } else {
        profiles.push(profile);
    }
}

#[tauri::command]
pub(crate) fn list_profiles(state: State<'_, AppState>) -> Result<Vec<ConnectionProfile>, String> {
    read_profiles(&state.profile_file)
}

#[tauri::command]
pub(crate) fn save_profile(
    profile: ConnectionProfile,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut profiles = list_profiles(state.clone())?;
    upsert_profile(&mut profiles, profile);
    write_profiles(&state.profile_file, &profiles)
}

#[tauri::command]
pub(crate) fn delete_profile(profile_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut profiles = list_profiles(state.clone())?;
    profiles.retain(|profile| profile.id != profile_id);
    write_profiles(&state.profile_file, &profiles)
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{read_profiles, upsert_profile, write_profiles};
    use crate::models::ConnectionProfile;

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
        let profile_file = std::env::temp_dir().join(format!("tracing-log-profiles-{unique}.json"));
        let mut profiles = Vec::new();

        upsert_profile(&mut profiles, profile("one", "first"));
        upsert_profile(&mut profiles, profile("one", "updated"));
        upsert_profile(&mut profiles, profile("two", "second"));
        write_profiles(&profile_file, &profiles).unwrap();

        let mut restored = read_profiles(&profile_file).unwrap();
        assert_eq!(restored.len(), 2);
        assert_eq!(restored[0].name, "updated");
        restored.retain(|item| item.id != "one");
        write_profiles(&profile_file, &restored).unwrap();
        assert_eq!(read_profiles(&profile_file).unwrap().len(), 1);

        fs::remove_file(profile_file).unwrap();
    }
}
