#![allow(dead_code)]

use std::sync::atomic::{AtomicU32, Ordering};

use kammerz_lib::entities::camera::{self, CameraFormat, CameraType};
use kammerz_lib::entities::camera_maintenance::{self, MaintenanceType};
use kammerz_lib::entities::development_lab;
use kammerz_lib::entities::development_self;
use kammerz_lib::entities::film_stock::{self, FilmFormat, FilmStockType};
use kammerz_lib::entities::lab;
use kammerz_lib::entities::lens;
use kammerz_lib::entities::lens_mount;
use kammerz_lib::entities::roll::{self, RollStatus};
use kammerz_lib::entities::shot;

/// Monotonic counter to guarantee unique roll IDs within a test.
static ROLL_COUNTER: AtomicU32 = AtomicU32::new(0);
use kammerz_lib::services::camera_service::CameraService;
use kammerz_lib::services::development_service::{DevelopmentService, StageInput};
use kammerz_lib::services::film_stock_service::FilmStockService;
use kammerz_lib::services::lab_service::LabService;
use kammerz_lib::services::lens_service::LensService;
use kammerz_lib::services::roll_service::RollService;
use kammerz_lib::services::shot_service::ShotService;
use migration::MigratorTrait;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Set};

/// Create an in-memory SQLite database with all migrations applied.
/// Each test gets a completely isolated database.
pub async fn setup_db() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite");

    // Match production pragma setup from db.rs
    db.execute_unprepared("PRAGMA journal_mode=WAL")
        .await
        .expect("Failed to set journal_mode");
    db.execute_unprepared("PRAGMA busy_timeout=5000")
        .await
        .expect("Failed to set busy_timeout");

    // FK off before migrations (same as production — prevents cascade on table rebuilds)
    db.execute_unprepared("PRAGMA foreign_keys=OFF")
        .await
        .expect("Failed to disable foreign keys");

    migration::Migrator::up(&db, None)
        .await
        .expect("Failed to run migrations");

    // FK on for runtime queries
    db.execute_unprepared("PRAGMA foreign_keys=ON")
        .await
        .expect("Failed to enable foreign keys");

    db
}

// ---------------------------------------------------------------------------
// Factory helpers — create minimal valid entities for tests
// ---------------------------------------------------------------------------

fn now() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}

/// Look up a seeded lens mount by name (migrations seed many mounts).
pub async fn find_mount_by_name(db: &DatabaseConnection, name: &str) -> lens_mount::Model {
    use kammerz_lib::entities::lens_mount::Entity as LensMount;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    LensMount::find()
        .filter(lens_mount::Column::Name.eq(name))
        .one(db)
        .await
        .expect("DB error looking up mount")
        .unwrap_or_else(|| panic!("Seeded mount '{name}' not found"))
}

pub async fn create_test_camera(db: &DatabaseConnection, mount_id: i32) -> camera::Model {
    let ts = now();
    CameraService::create(
        db,
        camera::ActiveModel {
            brand: Set("TestBrand".into()),
            model: Set("TestModel".into()),
            format: Set(CameraFormat::ThirtyFiveMm),
            lens_mount_id: Set(mount_id),
            camera_type: Set(Some(CameraType::Slr)),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await
    .expect("Failed to create test camera")
}

pub async fn create_test_lens(db: &DatabaseConnection, mount_id: i32) -> lens::Model {
    let ts = now();
    LensService::create(
        db,
        lens::ActiveModel {
            brand: Set("TestLens".into()),
            lens_mount_id: Set(mount_id),
            model: Set(Some("50mm f/1.4".into())),
            focal_length: Set(Some("50mm".into())),
            max_aperture: Set(Some("f/1.4".into())),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await
    .expect("Failed to create test lens")
}

pub async fn create_test_film_stock(db: &DatabaseConnection) -> film_stock::Model {
    let ts = now();
    FilmStockService::create(
        db,
        film_stock::ActiveModel {
            brand: Set("TestFilm".into()),
            name: Set("Test 400".into()),
            format: Set(FilmFormat::F135),
            stock_type: Set(FilmStockType::BwNegative),
            iso: Set(Some(400)),
            exposure_count: Set(Some(36)),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await
    .expect("Failed to create test film stock")
}

pub async fn create_test_lab(db: &DatabaseConnection) -> lab::Model {
    let ts = now();
    LabService::create(
        db,
        lab::ActiveModel {
            name: Set("Test Lab".into()),
            location: Set(Some("Test City".into())),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await
    .expect("Failed to create test lab")
}

pub async fn create_test_roll(
    db: &DatabaseConnection,
    camera_id: Option<i32>,
    film_stock_id: Option<i32>,
) -> roll::Model {
    create_test_roll_with_status(db, camera_id, film_stock_id, RollStatus::Loaded).await
}

pub async fn create_test_roll_with_status(
    db: &DatabaseConnection,
    camera_id: Option<i32>,
    film_stock_id: Option<i32>,
    status: RollStatus,
) -> roll::Model {
    let ts = now();
    let seq = ROLL_COUNTER.fetch_add(1, Ordering::Relaxed);
    let roll_id = format!("TEST-{seq}");
    RollService::create(
        db,
        roll::ActiveModel {
            roll_id: Set(roll_id),
            camera_id: Set(camera_id),
            film_stock_id: Set(film_stock_id),
            status: Set(status),
            frame_count: Set(Some(36)),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await
    .expect("Failed to create test roll")
}

pub async fn create_test_shot(db: &DatabaseConnection, roll_id: i32) -> shot::Model {
    let ts = now();
    ShotService::create(
        db,
        shot::ActiveModel {
            roll_id: Set(roll_id),
            frame_number: Set("1".into()),
            aperture: Set(Some("f/5.6".into())),
            shutter_speed: Set(Some("1/125".into())),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await
    .expect("Failed to create test shot")
}

pub async fn create_test_shot_numbered(
    db: &DatabaseConnection,
    roll_id: i32,
    frame: &str,
) -> shot::Model {
    let ts = now();
    ShotService::create(
        db,
        shot::ActiveModel {
            roll_id: Set(roll_id),
            frame_number: Set(frame.into()),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await
    .expect("Failed to create test shot")
}

pub async fn create_test_lab_dev(
    db: &DatabaseConnection,
    roll_id: i32,
    lab_id: Option<i32>,
) -> development_lab::Model {
    let ts = now();
    DevelopmentService::create_lab_dev(
        db,
        development_lab::ActiveModel {
            roll_id: Set(roll_id),
            lab_id: Set(lab_id),
            cost: Set(Some(15.0)),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await
    .expect("Failed to create test lab dev")
}

pub async fn create_test_self_dev(
    db: &DatabaseConnection,
    roll_id: i32,
) -> development_self::Model {
    let ts = now();
    DevelopmentService::create_self_dev(
        db,
        development_self::ActiveModel {
            roll_id: Set(roll_id),
            developer: Set(Some("D-76".into())),
            developer_dilution: Set(Some("1:1".into())),
            fixer: Set(Some("Kodak Fixer".into())),
            temperature: Set(Some("20C".into())),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await
    .expect("Failed to create test self dev")
}

pub async fn create_test_maintenance(
    db: &DatabaseConnection,
    camera_id: i32,
) -> camera_maintenance::Model {
    let ts = now();
    CameraService::create_maintenance(
        db,
        camera_maintenance::ActiveModel {
            camera_id: Set(camera_id),
            maintenance_type: Set(MaintenanceType::Cla),
            done_by: Set(Some("Test Shop".into())),
            cost: Set(Some(200.0)),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await
    .expect("Failed to create test maintenance")
}

pub async fn set_test_stages(db: &DatabaseConnection, dev_id: i32) {
    DevelopmentService::set_stages(
        db,
        dev_id,
        vec![
            StageInput {
                stage_name: "Dev".into(),
                duration_seconds: Some(600),
                notes: None,
                sort_order: 1,
            },
            StageInput {
                stage_name: "Stop".into(),
                duration_seconds: Some(60),
                notes: None,
                sort_order: 2,
            },
            StageInput {
                stage_name: "Fix".into(),
                duration_seconds: Some(300),
                notes: Some("Agitate first 30s".into()),
                sort_order: 3,
            },
        ],
    )
    .await
    .expect("Failed to set test stages");
}
