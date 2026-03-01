mod common;

use kammerz_lib::entities::lens;
use kammerz_lib::services::camera_service::CameraService;
use kammerz_lib::services::lens_service::LensService;
use kammerz_lib::services::shot_service::ShotService;
use sea_orm::Set;

#[tokio::test]
async fn create_and_read_lens() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let lens = common::create_test_lens(&db, mount.id).await;

    assert_eq!(lens.brand, "TestLens");
    assert_eq!(lens.model, Some("50mm f/1.4".into()));
    assert_eq!(lens.focal_length, Some("50mm".into()));
    assert_eq!(lens.max_aperture, Some("f/1.4".into()));

    let fetched = LensService::get_by_id(&db, lens.id).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().brand, "TestLens");
}

#[tokio::test]
async fn update_lens() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Canon EF").await;
    let lens = common::create_test_lens(&db, mount.id).await;

    let mut model: lens::ActiveModel = lens.into();
    model.model = Set(Some("85mm f/1.8".into()));
    model.focal_length = Set(Some("85mm".into()));
    model.serial_number = Set(Some("ABC123".into()));

    let updated = LensService::update(&db, model).await.unwrap();
    assert_eq!(updated.model, Some("85mm f/1.8".into()));
    assert_eq!(updated.serial_number, Some("ABC123".into()));
}

#[tokio::test]
async fn delete_lens() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let lens = common::create_test_lens(&db, mount.id).await;

    LensService::delete(&db, lens.id).await.unwrap();

    let fetched = LensService::get_by_id(&db, lens.id).await.unwrap();
    assert!(fetched.is_none());
}

#[tokio::test]
async fn delete_lens_cascades_camera_junction() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let cam = common::create_test_camera(&db, mount.id).await;
    let lens = common::create_test_lens(&db, mount.id).await;

    CameraService::link_lens(&db, cam.id, lens.id).await.unwrap();

    LensService::delete(&db, lens.id).await.unwrap();

    let linked = CameraService::get_lenses_for_camera(&db, cam.id).await.unwrap();
    assert!(linked.is_empty(), "Camera-lens junction should be cascade-deleted with lens");
}

#[tokio::test]
async fn delete_lens_cascades_shot_junction() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let lens = common::create_test_lens(&db, mount.id).await;
    let film = common::create_test_film_stock(&db).await;
    let roll = common::create_test_roll(&db, None, Some(film.id)).await;
    let shot = common::create_test_shot(&db, roll.id).await;

    ShotService::set_lenses_for_shot(&db, shot.id, vec![lens.id])
        .await
        .unwrap();

    // Verify link exists
    let lenses = ShotService::get_lenses_for_shot(&db, shot.id).await.unwrap();
    assert_eq!(lenses, vec![lens.id]);

    // Delete lens — should cascade shot_lenses
    LensService::delete(&db, lens.id).await.unwrap();

    let lenses_after = ShotService::get_lenses_for_shot(&db, shot.id).await.unwrap();
    assert!(lenses_after.is_empty(), "Shot-lens junction should be cascade-deleted with lens");
}

#[tokio::test]
async fn distinct_brands_and_systems() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    LensService::create(
        &db,
        lens::ActiveModel {
            brand: Set("Nikon".into()),
            lens_mount_id: Set(mount.id),
            lens_system: Set(Some("AI-S".into())),
            created_at: Set(ts.clone()),
            updated_at: Set(ts.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let brands = LensService::distinct_brands(&db).await.unwrap();
    assert!(brands.contains(&"Nikon".to_string()));

    let systems = LensService::distinct_lens_systems(&db).await.unwrap();
    assert!(systems.contains(&"AI-S".to_string()));
}
