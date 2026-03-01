mod common;

use kammerz_lib::entities::roll::{self, Entity as Roll, RollStatus};
use kammerz_lib::services::development_service::DevelopmentService;
use kammerz_lib::services::roll_service::RollService;
use kammerz_lib::services::shot_service::ShotService;
use sea_orm::{EntityTrait, Set};

// ---------------------------------------------------------------------------
// CRUD
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_and_read_roll() {
    let db = common::setup_db().await;
    let film = common::create_test_film_stock(&db).await;
    let roll = common::create_test_roll(&db, None, Some(film.id)).await;

    assert_eq!(roll.status, RollStatus::Loaded);
    assert_eq!(roll.film_stock_id, Some(film.id));
    assert_eq!(roll.frame_count, Some(36));

    let fetched = Roll::find_by_id(roll.id).one(&db).await.unwrap();
    assert!(fetched.is_some());
}

#[tokio::test]
async fn update_roll() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;

    let mut model: roll::ActiveModel = roll.into();
    model.status = Set(RollStatus::Shooting);
    model.notes = Set(Some("Updated notes".into()));

    let updated = RollService::update(&db, model).await.unwrap();
    assert_eq!(updated.status, RollStatus::Shooting);
    assert_eq!(updated.notes, Some("Updated notes".into()));
}

#[tokio::test]
async fn delete_roll() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;

    RollService::delete(&db, roll.id).await.unwrap();

    let fetched = Roll::find_by_id(roll.id).one(&db).await.unwrap();
    assert!(fetched.is_none());
}

#[tokio::test]
async fn duplicate_roll_id_rejected() {
    let db = common::setup_db().await;
    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    RollService::create(
        &db,
        roll::ActiveModel {
            roll_id: Set("UNIQUE-001".into()),
            status: Set(RollStatus::Loaded),
            created_at: Set(ts.clone()),
            updated_at: Set(ts.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let result = RollService::create(
        &db,
        roll::ActiveModel {
            roll_id: Set("UNIQUE-001".into()),
            status: Set(RollStatus::Loaded),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await;

    assert!(result.is_err(), "Duplicate roll_id should be rejected by UNIQUE constraint");
}

// ---------------------------------------------------------------------------
// Joined queries
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_with_details_includes_camera_and_film() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let cam = common::create_test_camera(&db, mount.id).await;
    let film = common::create_test_film_stock(&db).await;
    let roll = common::create_test_roll(&db, Some(cam.id), Some(film.id)).await;

    let details = RollService::get_with_details(&db, roll.id).await.unwrap().unwrap();
    assert_eq!(details.camera_brand, Some("TestBrand".into()));
    assert_eq!(details.camera_model, Some("TestModel".into()));
    assert_eq!(details.film_stock_brand, Some("TestFilm".into()));
    assert_eq!(details.film_stock_name, Some("Test 400".into()));
    assert_eq!(details.film_stock_iso, Some(400));
}

#[tokio::test]
async fn list_with_details_handles_null_fks() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;

    let details = RollService::get_with_details(&db, roll.id).await.unwrap().unwrap();
    assert_eq!(details.camera_brand, None);
    assert_eq!(details.film_stock_brand, None);
}

#[tokio::test]
async fn list_for_camera() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let cam = common::create_test_camera(&db, mount.id).await;

    common::create_test_roll(&db, Some(cam.id), None).await;
    common::create_test_roll(&db, Some(cam.id), None).await;
    common::create_test_roll(&db, None, None).await; // different camera

    let rolls = RollService::list_for_camera(&db, cam.id).await.unwrap();
    assert_eq!(rolls.len(), 2);
}

// ---------------------------------------------------------------------------
// Roll ID suggestion
// ---------------------------------------------------------------------------

#[tokio::test]
async fn suggest_id_format() {
    let db = common::setup_db().await;
    let suggested = RollService::suggest_id(&db).await.unwrap();

    // Format: YYMMDD-N
    assert!(suggested.contains('-'), "Suggested ID should have YYMMDD-N format");
    let parts: Vec<&str> = suggested.split('-').collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].len(), 6, "Date prefix should be 6 chars (YYMMDD)");
    assert_eq!(parts[1], "1", "First suggestion of the day should be -1");
}

// ---------------------------------------------------------------------------
// Delete cascades
// ---------------------------------------------------------------------------

#[tokio::test]
async fn delete_roll_cascades_shots() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    common::create_test_shot(&db, roll.id).await;
    common::create_test_shot_numbered(&db, roll.id, "2").await;

    let count_before = ShotService::count_for_roll(&db, roll.id).await.unwrap();
    assert_eq!(count_before, 2);

    RollService::delete(&db, roll.id).await.unwrap();

    let count_after = ShotService::count_for_roll(&db, roll.id).await.unwrap();
    assert_eq!(count_after, 0, "Shots should be cascade-deleted with roll");
}

#[tokio::test]
async fn delete_roll_cascades_dev_records() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;

    common::create_test_lab_dev(&db, roll.id, None).await;

    RollService::delete(&db, roll.id).await.unwrap();

    let lab_dev = DevelopmentService::get_lab_dev_for_roll(&db, roll.id).await.unwrap();
    assert!(lab_dev.is_none(), "Lab dev should be cascade-deleted with roll");
}

#[tokio::test]
async fn delete_roll_cascades_self_dev_and_stages() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;

    let dev = common::create_test_self_dev(&db, roll.id).await;
    common::set_test_stages(&db, dev.id).await;

    let stages_before = DevelopmentService::list_stages(&db, dev.id).await.unwrap();
    assert_eq!(stages_before.len(), 3);

    RollService::delete(&db, roll.id).await.unwrap();

    let self_dev = DevelopmentService::get_self_dev_for_roll(&db, roll.id).await.unwrap();
    assert!(self_dev.is_none(), "Self dev should be cascade-deleted with roll");
}

// ---------------------------------------------------------------------------
// Status auto-sync — the core business logic
// ---------------------------------------------------------------------------

#[tokio::test]
async fn auto_sync_first_shot_advances_loaded_to_shooting() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    assert_eq!(roll.status, RollStatus::Loaded);

    // Simulate what create_shot command does
    let shot = common::create_test_shot(&db, roll.id).await;
    let _ = shot; // just need the insert for auto_sync_status to have context
    let changed = RollService::auto_sync_status(
        &db,
        roll.id,
        &[RollStatus::Loaded],
        RollStatus::Shooting,
    )
    .await
    .unwrap();

    assert!(changed, "Status should advance");
    let roll_after = Roll::find_by_id(roll.id).one(&db).await.unwrap().unwrap();
    assert_eq!(roll_after.status, RollStatus::Shooting);
}

#[tokio::test]
async fn auto_sync_does_not_advance_beyond_range() {
    let db = common::setup_db().await;
    // Roll is already at "shot" — adding a shot should not change it
    let roll = common::create_test_roll_with_status(&db, None, None, RollStatus::Shot).await;

    let changed = RollService::auto_sync_status(
        &db,
        roll.id,
        &[RollStatus::Loaded],
        RollStatus::Shooting,
    )
    .await
    .unwrap();

    assert!(!changed, "Status should NOT advance when already beyond from_statuses range");
    let roll_after = Roll::find_by_id(roll.id).one(&db).await.unwrap().unwrap();
    assert_eq!(roll_after.status, RollStatus::Shot);
}

#[tokio::test]
async fn auto_sync_all_shots_deleted_reverts_to_loaded() {
    let db = common::setup_db().await;
    let roll =
        common::create_test_roll_with_status(&db, None, None, RollStatus::Shooting).await;

    // Simulate: last shot deleted → check if remaining == 0
    let changed = RollService::auto_sync_status(
        &db,
        roll.id,
        &[RollStatus::Shooting, RollStatus::Shot],
        RollStatus::Loaded,
    )
    .await
    .unwrap();

    assert!(changed);
    let roll_after = Roll::find_by_id(roll.id).one(&db).await.unwrap().unwrap();
    assert_eq!(roll_after.status, RollStatus::Loaded);
}

#[tokio::test]
async fn auto_sync_lab_dev_advances_to_at_lab() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll_with_status(&db, None, None, RollStatus::Shot).await;

    let changed = RollService::auto_sync_status(
        &db,
        roll.id,
        &[RollStatus::Loaded, RollStatus::Shooting, RollStatus::Shot],
        RollStatus::AtLab,
    )
    .await
    .unwrap();

    assert!(changed);
    let roll_after = Roll::find_by_id(roll.id).one(&db).await.unwrap().unwrap();
    assert_eq!(roll_after.status, RollStatus::AtLab);
}

#[tokio::test]
async fn auto_sync_lab_dev_deleted_reverts_at_lab_to_shot() {
    let db = common::setup_db().await;
    let roll =
        common::create_test_roll_with_status(&db, None, None, RollStatus::AtLab).await;

    let changed = RollService::auto_sync_status(
        &db,
        roll.id,
        &[RollStatus::AtLab, RollStatus::LabDone],
        RollStatus::Shot,
    )
    .await
    .unwrap();

    assert!(changed);
    let roll_after = Roll::find_by_id(roll.id).one(&db).await.unwrap().unwrap();
    assert_eq!(roll_after.status, RollStatus::Shot);
}

#[tokio::test]
async fn auto_sync_lab_dev_deleted_does_not_revert_from_scanned() {
    let db = common::setup_db().await;
    let roll =
        common::create_test_roll_with_status(&db, None, None, RollStatus::Scanned).await;

    let changed = RollService::auto_sync_status(
        &db,
        roll.id,
        &[RollStatus::AtLab, RollStatus::LabDone],
        RollStatus::Shot,
    )
    .await
    .unwrap();

    assert!(!changed, "Should NOT revert from scanned (beyond range)");
    let roll_after = Roll::find_by_id(roll.id).one(&db).await.unwrap().unwrap();
    assert_eq!(roll_after.status, RollStatus::Scanned);
}

#[tokio::test]
async fn auto_sync_self_dev_advances_to_developing() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll_with_status(&db, None, None, RollStatus::Shot).await;

    let changed = RollService::auto_sync_status(
        &db,
        roll.id,
        &[RollStatus::Loaded, RollStatus::Shooting, RollStatus::Shot],
        RollStatus::Developing,
    )
    .await
    .unwrap();

    assert!(changed);
    let roll_after = Roll::find_by_id(roll.id).one(&db).await.unwrap().unwrap();
    assert_eq!(roll_after.status, RollStatus::Developing);
}

#[tokio::test]
async fn auto_sync_self_dev_deleted_reverts_developing_to_shot() {
    let db = common::setup_db().await;
    let roll =
        common::create_test_roll_with_status(&db, None, None, RollStatus::Developing).await;

    let changed = RollService::auto_sync_status(
        &db,
        roll.id,
        &[RollStatus::Developing, RollStatus::Developed],
        RollStatus::Shot,
    )
    .await
    .unwrap();

    assert!(changed);
    let roll_after = Roll::find_by_id(roll.id).one(&db).await.unwrap().unwrap();
    assert_eq!(roll_after.status, RollStatus::Shot);
}

#[tokio::test]
async fn auto_sync_self_dev_deleted_does_not_revert_from_archived() {
    let db = common::setup_db().await;
    let roll =
        common::create_test_roll_with_status(&db, None, None, RollStatus::Archived).await;

    let changed = RollService::auto_sync_status(
        &db,
        roll.id,
        &[RollStatus::Developing, RollStatus::Developed],
        RollStatus::Shot,
    )
    .await
    .unwrap();

    assert!(!changed, "Should NOT revert from archived (beyond range)");
}

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

#[tokio::test]
async fn import_roll_creates_roll_and_shots() {
    use kammerz_lib::services::roll_service::ImportShotEntry;

    let db = common::setup_db().await;
    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let roll_model = roll::ActiveModel {
        roll_id: Set("IMPORT-001".into()),
        status: Set(RollStatus::Loaded),
        created_at: Set(ts.clone()),
        updated_at: Set(ts),
        ..Default::default()
    };

    let shots = vec![
        ImportShotEntry {
            frame_number: "1".into(),
            aperture: Some("f/8".into()),
            shutter_speed: Some("1/250".into()),
            date: Some("2025-01-15".into()),
            date_fuzzy: None,
            location: Some("Park".into()),
            notes: None,
            lens_ids: None,
        },
        ImportShotEntry {
            frame_number: "2".into(),
            aperture: Some("f/5.6".into()),
            shutter_speed: None,
            date: None,
            date_fuzzy: Some("afternoon".into()),
            location: None,
            notes: Some("Nice light".into()),
            lens_ids: None,
        },
    ];

    let roll_id = RollService::import_roll(&db, roll_model, shots).await.unwrap();

    // Verify roll
    let roll = Roll::find_by_id(roll_id).one(&db).await.unwrap().unwrap();
    assert_eq!(roll.roll_id, "IMPORT-001");

    // Verify shots
    let shot_list = ShotService::list_for_roll(&db, roll_id).await.unwrap();
    assert_eq!(shot_list.len(), 2);
    assert_eq!(shot_list[0].frame_number, "1");
    assert_eq!(shot_list[1].frame_number, "2");
    assert_eq!(shot_list[0].aperture, Some("f/8".into()));
    assert_eq!(shot_list[1].notes, Some("Nice light".into()));
}
