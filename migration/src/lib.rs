pub use sea_orm_migration::prelude::*;

mod m20250101_000001_initial_schema;
mod m20250101_000002_seed_film_stocks;
mod m20250201_000003_settings_table;
mod m20250216_000004_add_roll_lens_id;
mod m20250217_000005_add_rolls_lens_id_index;
mod m20250219_000006_lens_mount_table;
mod m20250221_000007_seed_lf_shutter_mounts;
mod m20250221_000008_rename_large_format_mount;
mod m20250221_000009_remove_generic_large_format;
mod m20250221_000010_seed_fixed_lens_mount;
mod m20250221_000011_add_default_lens_id;
mod m20260221_000012_seed_instant_film_stocks;
mod m20260221_000013_seed_user_cameras;
mod m20260222_000014_seed_user_lenses;
mod m20260222_000015_seed_qbm_and_extras;
mod m20260222_000016_seed_user_gear_batch2;
mod m20260222_000017_rename_name_on_lens_to_model;
mod m20260222_000018_trim_brand_whitespace;
mod m20260222_000019_schema_hardening;
mod m20260223_000020_repair_fixed_lens_data;
mod m20260602_000021_add_roll_lifecycle_dates;
mod m20260614_000022_create_roll_events;
mod m20260618_000023_add_shot_time;
mod m20260621_000024_drop_date_fuzzy;
mod m20260701_000025_normalize_aperture_bare;
mod m20260711_000026_add_negatives_pickup;
mod m20260713_000027_sync_catalog_after_import;
mod m20260713_000028_create_chemicals;
mod m20260713_000029_normalize_dev_chemistry;
mod m20260718_000030_activity_lifecycle;

/// Re-exported so `tests/chemicals.rs` exercises the exact normalization data and
/// apply step the m..029 migration uses (no drift between test and migration).
pub use m20260713_000029_normalize_dev_chemistry::{NORMALIZATIONS, apply_normalization};

/// Re-exported so `tests/` and `import.rs` reuse the exact status→date backfill
/// mapping the m..030 activity-lifecycle migration applies (kammerz-b0ix).
pub use m20260718_000030_activity_lifecycle::{BACKFILL_ORDER, BackfilledDates, backfilled_dates};

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250101_000001_initial_schema::Migration),
            Box::new(m20250101_000002_seed_film_stocks::Migration),
            Box::new(m20250201_000003_settings_table::Migration),
            Box::new(m20250216_000004_add_roll_lens_id::Migration),
            Box::new(m20250217_000005_add_rolls_lens_id_index::Migration),
            Box::new(m20250219_000006_lens_mount_table::Migration),
            Box::new(m20250221_000007_seed_lf_shutter_mounts::Migration),
            Box::new(m20250221_000008_rename_large_format_mount::Migration),
            Box::new(m20250221_000009_remove_generic_large_format::Migration),
            Box::new(m20250221_000010_seed_fixed_lens_mount::Migration),
            Box::new(m20250221_000011_add_default_lens_id::Migration),
            Box::new(m20260221_000012_seed_instant_film_stocks::Migration),
            Box::new(m20260221_000013_seed_user_cameras::Migration),
            Box::new(m20260222_000014_seed_user_lenses::Migration),
            Box::new(m20260222_000015_seed_qbm_and_extras::Migration),
            Box::new(m20260222_000016_seed_user_gear_batch2::Migration),
            Box::new(m20260222_000017_rename_name_on_lens_to_model::Migration),
            Box::new(m20260222_000018_trim_brand_whitespace::Migration),
            Box::new(m20260222_000019_schema_hardening::Migration),
            Box::new(m20260223_000020_repair_fixed_lens_data::Migration),
            Box::new(m20260602_000021_add_roll_lifecycle_dates::Migration),
            Box::new(m20260614_000022_create_roll_events::Migration),
            Box::new(m20260618_000023_add_shot_time::Migration),
            Box::new(m20260621_000024_drop_date_fuzzy::Migration),
            Box::new(m20260701_000025_normalize_aperture_bare::Migration),
            Box::new(m20260711_000026_add_negatives_pickup::Migration),
            Box::new(m20260713_000027_sync_catalog_after_import::Migration),
            Box::new(m20260713_000028_create_chemicals::Migration),
            Box::new(m20260713_000029_normalize_dev_chemistry::Migration),
            Box::new(m20260718_000030_activity_lifecycle::Migration),
        ]
    }
}
