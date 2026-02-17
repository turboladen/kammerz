use tauri::State;

use crate::services::stats_service::{CatalogStats, StatsService};
use crate::AppState;

#[tauri::command]
pub async fn get_catalog_stats(
    state: State<'_, AppState>,
) -> Result<CatalogStats, String> {
    StatsService::get_stats(&state.db)
        .await
        .map_err(|e| {
            log::error!("Failed to get catalog stats: {e}");
            format!("Could not load statistics: {e}")
        })
}
