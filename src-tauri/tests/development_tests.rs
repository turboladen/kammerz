mod common;

use kammerz_lib::entities::development_lab;
use kammerz_lib::entities::development_self;
use kammerz_lib::services::development_service::{DevelopmentService, StageInput};
use sea_orm::Set;

// ---------------------------------------------------------------------------
// Lab Development
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_and_read_lab_dev() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    let lab = common::create_test_lab(&db).await;
    let dev = common::create_test_lab_dev(&db, roll.id, Some(lab.id)).await;

    assert_eq!(dev.roll_id, roll.id);
    assert_eq!(dev.lab_id, Some(lab.id));
    assert_eq!(dev.cost, Some(15.0));

    let fetched = DevelopmentService::get_lab_dev_for_roll(&db, roll.id)
        .await
        .unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().id, dev.id);
}

#[tokio::test]
async fn update_lab_dev() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    let dev = common::create_test_lab_dev(&db, roll.id, None).await;

    let mut model: development_lab::ActiveModel = dev.into();
    model.cost = Set(Some(25.0));
    model.date_dropped_off = Set(Some("2025-01-15".into()));
    model.notes = Set(Some("Rush order".into()));

    let updated = DevelopmentService::update_lab_dev(&db, model).await.unwrap();
    assert_eq!(updated.cost, Some(25.0));
    assert_eq!(updated.date_dropped_off, Some("2025-01-15".into()));
    assert_eq!(updated.notes, Some("Rush order".into()));
}

#[tokio::test]
async fn delete_lab_dev() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    let dev = common::create_test_lab_dev(&db, roll.id, None).await;

    DevelopmentService::delete_lab_dev(&db, dev.id).await.unwrap();

    let fetched = DevelopmentService::get_lab_dev_for_roll(&db, roll.id)
        .await
        .unwrap();
    assert!(fetched.is_none());
}

#[tokio::test]
async fn one_lab_dev_per_roll_constraint() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;

    common::create_test_lab_dev(&db, roll.id, None).await;

    // Second lab dev for same roll should fail (UNIQUE constraint on roll_id)
    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let result = DevelopmentService::create_lab_dev(
        &db,
        development_lab::ActiveModel {
            roll_id: Set(roll.id),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await;

    assert!(result.is_err(), "Only one lab dev per roll allowed");
}

// ---------------------------------------------------------------------------
// Self Development
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_and_read_self_dev() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    let dev = common::create_test_self_dev(&db, roll.id).await;

    assert_eq!(dev.roll_id, roll.id);
    assert_eq!(dev.developer, Some("D-76".into()));
    assert_eq!(dev.developer_dilution, Some("1:1".into()));
    assert_eq!(dev.fixer, Some("Kodak Fixer".into()));
    assert_eq!(dev.temperature, Some("20C".into()));

    let fetched = DevelopmentService::get_self_dev_for_roll(&db, roll.id)
        .await
        .unwrap();
    assert!(fetched.is_some());
}

#[tokio::test]
async fn update_self_dev() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    let dev = common::create_test_self_dev(&db, roll.id).await;

    let mut model: development_self::ActiveModel = dev.into();
    model.developer = Set(Some("HC-110".into()));
    model.developer_dilution = Set(Some("Dilution B".into()));
    model.stop_bath = Set(Some("Water".into()));

    let updated = DevelopmentService::update_self_dev(&db, model).await.unwrap();
    assert_eq!(updated.developer, Some("HC-110".into()));
    assert_eq!(updated.developer_dilution, Some("Dilution B".into()));
    assert_eq!(updated.stop_bath, Some("Water".into()));
}

#[tokio::test]
async fn delete_self_dev() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    let dev = common::create_test_self_dev(&db, roll.id).await;

    DevelopmentService::delete_self_dev(&db, dev.id).await.unwrap();

    let fetched = DevelopmentService::get_self_dev_for_roll(&db, roll.id)
        .await
        .unwrap();
    assert!(fetched.is_none());
}

#[tokio::test]
async fn one_self_dev_per_roll_constraint() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;

    common::create_test_self_dev(&db, roll.id).await;

    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let result = DevelopmentService::create_self_dev(
        &db,
        development_self::ActiveModel {
            roll_id: Set(roll.id),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await;

    assert!(result.is_err(), "Only one self dev per roll allowed");
}

// ---------------------------------------------------------------------------
// Dev Stages
// ---------------------------------------------------------------------------

#[tokio::test]
async fn set_and_list_stages() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    let dev = common::create_test_self_dev(&db, roll.id).await;

    common::set_test_stages(&db, dev.id).await;

    let stages = DevelopmentService::list_stages(&db, dev.id).await.unwrap();
    assert_eq!(stages.len(), 3);
    assert_eq!(stages[0].stage_name, "Dev");
    assert_eq!(stages[0].duration_seconds, Some(600));
    assert_eq!(stages[0].sort_order, 1);
    assert_eq!(stages[1].stage_name, "Stop");
    assert_eq!(stages[1].sort_order, 2);
    assert_eq!(stages[2].stage_name, "Fix");
    assert_eq!(stages[2].notes, Some("Agitate first 30s".into()));
    assert_eq!(stages[2].sort_order, 3);
}

#[tokio::test]
async fn set_stages_replaces_existing() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    let dev = common::create_test_self_dev(&db, roll.id).await;

    // Set initial stages
    common::set_test_stages(&db, dev.id).await;

    // Replace with fewer stages
    DevelopmentService::set_stages(
        &db,
        dev.id,
        vec![StageInput {
            stage_name: "Only Stage".into(),
            duration_seconds: Some(900),
            notes: None,
            sort_order: 1,
        }],
    )
    .await
    .unwrap();

    let stages = DevelopmentService::list_stages(&db, dev.id).await.unwrap();
    assert_eq!(stages.len(), 1, "set_stages should replace all existing stages");
    assert_eq!(stages[0].stage_name, "Only Stage");
}

#[tokio::test]
async fn set_empty_stages_clears_all() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    let dev = common::create_test_self_dev(&db, roll.id).await;

    common::set_test_stages(&db, dev.id).await;

    DevelopmentService::set_stages(&db, dev.id, vec![]).await.unwrap();

    let stages = DevelopmentService::list_stages(&db, dev.id).await.unwrap();
    assert!(stages.is_empty());
}

#[tokio::test]
async fn delete_self_dev_cascades_stages() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    let dev = common::create_test_self_dev(&db, roll.id).await;
    common::set_test_stages(&db, dev.id).await;

    DevelopmentService::delete_self_dev(&db, dev.id).await.unwrap();

    // Stages should be gone (cascade delete via FK)
    let stages = DevelopmentService::list_stages(&db, dev.id).await.unwrap();
    assert!(stages.is_empty(), "Stages should be cascade-deleted with self dev");
}

// ---------------------------------------------------------------------------
// Batch queries
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_stages_for_dev_ids() {
    let db = common::setup_db().await;
    let roll1 = common::create_test_roll(&db, None, None).await;
    let roll2 = common::create_test_roll(&db, None, None).await;
    let dev1 = common::create_test_self_dev(&db, roll1.id).await;
    let dev2 = common::create_test_self_dev(&db, roll2.id).await;

    common::set_test_stages(&db, dev1.id).await;
    DevelopmentService::set_stages(
        &db,
        dev2.id,
        vec![StageInput {
            stage_name: "Single".into(),
            duration_seconds: Some(120),
            notes: None,
            sort_order: 1,
        }],
    )
    .await
    .unwrap();

    let all_stages = DevelopmentService::list_stages_for_dev_ids(&db, vec![dev1.id, dev2.id])
        .await
        .unwrap();

    assert_eq!(all_stages.len(), 4); // 3 from dev1 + 1 from dev2
}

#[tokio::test]
async fn list_stages_for_empty_ids() {
    let db = common::setup_db().await;

    let stages = DevelopmentService::list_stages_for_dev_ids(&db, vec![])
        .await
        .unwrap();
    assert!(stages.is_empty(), "Empty IDs should return empty vec (short-circuit)");
}

// ---------------------------------------------------------------------------
// List all self-developments (joined query)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_all_self_devs() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let cam = common::create_test_camera(&db, mount.id).await;
    let film = common::create_test_film_stock(&db).await;
    let roll = common::create_test_roll(&db, Some(cam.id), Some(film.id)).await;
    common::create_test_self_dev(&db, roll.id).await;

    let devs = DevelopmentService::list_all_self_devs(&db).await.unwrap();

    assert!(!devs.is_empty());
    let our_dev = devs.iter().find(|d| d.roll_pk == roll.id).unwrap();
    assert_eq!(our_dev.camera_brand, Some("TestBrand".into()));
    assert_eq!(our_dev.film_stock_brand, Some("TestFilm".into()));
    assert_eq!(our_dev.developer, Some("D-76".into()));
}
