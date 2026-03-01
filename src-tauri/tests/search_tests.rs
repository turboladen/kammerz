mod common;

use kammerz_lib::services::search_service::SearchService;

#[tokio::test]
async fn search_finds_cameras_by_brand() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    common::create_test_camera(&db, mount.id).await;

    let results = SearchService::search(&db, "TestBrand").await.unwrap();
    assert!(!results.cameras.is_empty(), "Should find camera by brand");
    assert_eq!(results.cameras[0].brand, "TestBrand");
    assert_eq!(results.cameras[0].match_field, "brand");
}

#[tokio::test]
async fn search_finds_cameras_by_model() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    common::create_test_camera(&db, mount.id).await;

    let results = SearchService::search(&db, "TestModel").await.unwrap();
    assert!(!results.cameras.is_empty());
    assert_eq!(results.cameras[0].match_field, "model");
}

#[tokio::test]
async fn search_finds_film_stocks_by_name() {
    let db = common::setup_db().await;

    // Migrations seed film stocks — search for a seeded one
    let results = SearchService::search(&db, "Tri-X").await.unwrap();
    assert!(!results.film_stocks.is_empty(), "Should find seeded Tri-X film stock");
}

#[tokio::test]
async fn search_finds_rolls_by_roll_id() {
    let db = common::setup_db().await;
    common::create_test_roll(&db, None, None).await;

    let results = SearchService::search(&db, "TEST-").await.unwrap();
    assert!(!results.rolls.is_empty(), "Should find roll by roll_id prefix");
    assert_eq!(results.rolls[0].match_field, "roll ID");
}

#[tokio::test]
async fn search_finds_shots_by_location() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;

    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    use kammerz_lib::entities::shot;
    use kammerz_lib::services::shot_service::ShotService;
    use sea_orm::Set;

    ShotService::create(
        &db,
        shot::ActiveModel {
            roll_id: Set(roll.id),
            frame_number: Set("1".into()),
            location: Set(Some("Golden Gate Bridge".into())),
            created_at: Set(ts.clone()),
            updated_at: Set(ts),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let results = SearchService::search(&db, "Golden Gate").await.unwrap();
    assert!(!results.shots.is_empty(), "Should find shot by location");
    assert_eq!(results.shots[0].match_field, "location");
}

#[tokio::test]
async fn search_finds_labs() {
    let db = common::setup_db().await;
    common::create_test_lab(&db).await;

    let results = SearchService::search(&db, "Test Lab").await.unwrap();
    assert!(!results.labs.is_empty(), "Should find lab by name");
    assert_eq!(results.labs[0].match_field, "name");
}

#[tokio::test]
async fn search_returns_empty_for_no_matches() {
    let db = common::setup_db().await;

    let results = SearchService::search(&db, "xyzzy_nonexistent_12345")
        .await
        .unwrap();

    assert!(results.cameras.is_empty());
    assert!(results.lenses.is_empty());
    assert!(results.film_stocks.is_empty());
    assert!(results.rolls.is_empty());
    assert!(results.shots.is_empty());
    assert!(results.labs.is_empty());
}

#[tokio::test]
async fn search_case_insensitive() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    common::create_test_camera(&db, mount.id).await;

    // SQLite LIKE is case-insensitive for ASCII by default
    let results = SearchService::search(&db, "testbrand").await.unwrap();
    assert!(!results.cameras.is_empty(), "Search should be case-insensitive");
}
