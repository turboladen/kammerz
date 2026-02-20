use sea_orm::Set;
use tauri::State;

use crate::entities::lens_mount;
use crate::services::lens_mount_service::LensMountService;
use crate::AppState;

#[tauri::command]
pub async fn list_lens_mounts(
    state: State<'_, AppState>,
) -> Result<Vec<lens_mount::Model>, String> {
    LensMountService::list_all(&state.db).await.map_err(|e| {
        log::error!("Failed to list lens mounts: {e}");
        format!("Could not list lens mounts: {e}")
    })
}

#[tauri::command]
pub async fn create_lens_mount(
    state: State<'_, AppState>,
    name: String,
) -> Result<i32, String> {
    let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let model = lens_mount::ActiveModel {
        name: Set(name),
        created_at: Set(now.clone()),
        updated_at: Set(now),
        ..Default::default()
    };
    let result = LensMountService::create(&state.db, model)
        .await
        .map_err(|e| {
            log::error!("Failed to create lens mount: {e}");
            format!("Could not create lens mount: {e}")
        })?;
    Ok(result.id)
}
