pub mod commands;
pub mod db;
pub mod entities;
pub mod patch;
pub mod services;

use sea_orm::DatabaseConnection;
use tauri::Manager;

pub struct AppState {
    pub db: DatabaseConnection,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            let conn = tauri::async_runtime::block_on(async {
                db::init(app.handle())
                    .await
                    .expect("Failed to initialize database")
            });

            app.manage(AppState { db: conn });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Cameras
            commands::cameras::list_cameras,
            commands::cameras::get_camera,
            commands::cameras::create_camera,
            commands::cameras::update_camera,
            commands::cameras::delete_camera,
            commands::cameras::list_maintenance,
            commands::cameras::create_maintenance,
            commands::cameras::update_maintenance,
            commands::cameras::delete_maintenance,
            commands::cameras::list_distinct_camera_brands,
            commands::cameras::list_distinct_vendors,
            commands::cameras::list_distinct_maint_providers,
            commands::cameras::get_lenses_for_camera,
            commands::cameras::link_lens_to_camera,
            commands::cameras::unlink_lens_from_camera,
            // Lens Mounts
            commands::lens_mounts::list_lens_mounts,
            commands::lens_mounts::create_lens_mount,
            // Lenses
            commands::lenses::list_lenses,
            commands::lenses::get_lens,
            commands::lenses::create_lens,
            commands::lenses::update_lens,
            commands::lenses::delete_lens,
            commands::lenses::list_distinct_lens_brands,
            commands::lenses::get_cameras_for_lens,
            // Film stocks
            commands::film_stocks::list_film_stocks,
            commands::film_stocks::get_film_stock,
            commands::film_stocks::create_film_stock,
            commands::film_stocks::update_film_stock,
            commands::film_stocks::delete_film_stock,
            commands::film_stocks::list_distinct_film_brands,
            // Labs
            commands::labs::list_labs,
            commands::labs::get_lab,
            commands::labs::create_lab,
            commands::labs::update_lab,
            commands::labs::delete_lab,
            // Rolls
            commands::rolls::list_rolls,
            commands::rolls::get_roll,
            commands::rolls::create_roll,
            commands::rolls::update_roll,
            commands::rolls::delete_roll,
            commands::rolls::list_rolls_for_camera,
            commands::rolls::suggest_roll_id,
            // Shots
            commands::shots::list_shots_for_roll,
            commands::shots::get_shot,
            commands::shots::create_shot,
            commands::shots::update_shot,
            commands::shots::delete_shot,
            commands::shots::get_lenses_for_shot,
            commands::shots::suggest_next_frame,
            commands::shots::count_shots_for_roll,
            // Development
            commands::development::get_lab_dev_for_roll,
            commands::development::create_lab_dev,
            commands::development::update_lab_dev,
            commands::development::delete_lab_dev,
            commands::development::get_self_dev_for_roll,
            commands::development::create_self_dev,
            commands::development::update_self_dev,
            commands::development::delete_self_dev,
            commands::development::list_dev_stages,
            // Search
            commands::search::search_catalog,
            // Stats
            commands::stats::get_catalog_stats,
            // Settings
            commands::settings::get_setting,
            commands::settings::set_setting,
            // Import
            commands::import::list_models,
            commands::import::parse_note,
            commands::import::import_parsed_roll,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
