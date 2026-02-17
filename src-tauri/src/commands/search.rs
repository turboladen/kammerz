use tauri::State;

use crate::services::search_service::{SearchResults, SearchService};
use crate::AppState;

#[tauri::command]
pub async fn search_catalog(
    state: State<'_, AppState>,
    query: String,
) -> Result<SearchResults, String> {
    let query = query.trim();
    if query.len() < 2 {
        // Return empty results for very short queries
        return Ok(SearchResults {
            cameras: vec![],
            lenses: vec![],
            film_stocks: vec![],
            rolls: vec![],
            shots: vec![],
            labs: vec![],
        });
    }

    SearchService::search(&state.db, query)
        .await
        .map_err(|e| {
            log::error!("Search failed for '{query}': {e}");
            format!("Search failed: {e}")
        })
}
