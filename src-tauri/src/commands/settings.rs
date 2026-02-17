use tauri::State;

use crate::services::settings_service::SettingsService;
use crate::AppState;

#[tauri::command]
pub async fn get_setting(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, String> {
    SettingsService::get_setting(&state.db, &key)
        .await
        .map_err(|e| {
            log::error!("Failed to get setting '{key}': {e}");
            format!("Could not get setting: {e}")
        })
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    SettingsService::set_setting(&state.db, key.clone(), value)
        .await
        .map_err(|e| {
            log::error!("Failed to set setting '{key}': {e}");
            format!("Could not save setting: {e}")
        })
}
