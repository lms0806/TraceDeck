use std::fs;

use tauri::State;

use crate::{models::ConnectionProfile, state::AppState};

#[tauri::command]
pub(crate) fn list_profiles(state: State<'_, AppState>) -> Result<Vec<ConnectionProfile>, String> {
    if !state.profile_file.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&state.profile_file).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn save_profile(
    profile: ConnectionProfile,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut profiles = list_profiles(state.clone())?;
    if let Some(index) = profiles.iter().position(|item| item.id == profile.id) {
        profiles[index] = profile;
    } else {
        profiles.push(profile);
    }
    let content = serde_json::to_string_pretty(&profiles).map_err(|error| error.to_string())?;
    fs::write(&state.profile_file, content).map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn delete_profile(profile_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut profiles = list_profiles(state.clone())?;
    profiles.retain(|profile| profile.id != profile_id);
    let content = serde_json::to_string_pretty(&profiles).map_err(|error| error.to_string())?;
    fs::write(&state.profile_file, content).map_err(|error| error.to_string())
}
