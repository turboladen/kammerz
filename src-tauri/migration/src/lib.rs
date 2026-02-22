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
        ]
    }
}
