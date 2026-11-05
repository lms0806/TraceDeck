use tauri::State;

use crate::{models::ConnectionProfile, state::AppState};

#[tauri::command]
pub(crate) fn list_profiles(state: State<'_, AppState>) -> Result<Vec<ConnectionProfile>, String> {
    state.profile_store().list()
}

#[tauri::command]
pub(crate) fn save_profile(
    profile: ConnectionProfile,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.profile_store().save(profile)
}

#[tauri::command]
pub(crate) fn delete_profile(profile_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.profile_store().delete(&profile_id)
}
