mod common;

use kammerz_lib::entities::camera::{self, CameraFormat, CameraType};
use kammerz_lib::entities::camera_maintenance::MaintenanceType;
use kammerz_lib::services::camera_service::CameraService;
use kammerz_lib::services::lens_service::LensService;
use sea_orm::Set;

#[tokio::test]
async fn create_and_read_camera() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let cam = common::create_test_camera(&db, mount.id).await;

    assert_eq!(cam.brand, "TestBrand");
    assert_eq!(cam.model, "TestModel");
    assert_eq!(cam.format, CameraFormat::ThirtyFiveMm);
    assert_eq!(cam.camera_type, Some(CameraType::Slr));

    let fetched = CameraService::get_by_id(&db, cam.id).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().id, cam.id);
}

#[tokio::test]
async fn update_camera() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let cam = common::create_test_camera(&db, mount.id).await;

    let mut model: camera::ActiveModel = cam.into();
    model.brand = Set("UpdatedBrand".into());
    model.serial_number = Set(Some("SN12345".into()));
    model.notes = Set(Some("Great camera".into()));

    let updated = CameraService::update(&db, model).await.unwrap();
    assert_eq!(updated.brand, "UpdatedBrand");
    assert_eq!(updated.serial_number, Some("SN12345".into()));
    assert_eq!(updated.notes, Some("Great camera".into()));
}

#[tokio::test]
async fn delete_camera() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let cam = common::create_test_camera(&db, mount.id).await;

    CameraService::delete(&db, cam.id).await.unwrap();

    let fetched = CameraService::get_by_id(&db, cam.id).await.unwrap();
    assert!(fetched.is_none());
}

#[tokio::test]
async fn list_cameras_sorted_by_brand() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;

    // Seed data from migrations may add cameras — create known ones
    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    for (brand, model) in [("Zebra", "Z1"), ("Alpha", "A1")] {
        CameraService::create(
            &db,
            camera::ActiveModel {
                brand: Set(brand.into()),
                model: Set(model.into()),
                format: Set(CameraFormat::ThirtyFiveMm),
                lens_mount_id: Set(mount.id),
                created_at: Set(ts.clone()),
                updated_at: Set(ts.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    }

    let cameras = CameraService::list_all(&db).await.unwrap();
    // Verify ordering: brands should be alphabetical
    let brands: Vec<&str> = cameras.iter().map(|c| c.brand.as_str()).collect();
    let mut sorted = brands.clone();
    sorted.sort_unstable();
    assert_eq!(brands, sorted, "Cameras should be sorted by brand");
}

#[tokio::test]
async fn link_and_unlink_lens() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let cam = common::create_test_camera(&db, mount.id).await;
    let lens = common::create_test_lens(&db, mount.id).await;

    // Link
    CameraService::link_lens(&db, cam.id, lens.id).await.unwrap();
    let linked = CameraService::get_lenses_for_camera(&db, cam.id).await.unwrap();
    assert_eq!(linked, vec![lens.id]);

    // Reverse lookup
    let cameras = LensService::get_cameras_for_lens(&db, lens.id).await.unwrap();
    assert_eq!(cameras, vec![cam.id]);

    // Idempotent link (should not error)
    CameraService::link_lens(&db, cam.id, lens.id).await.unwrap();
    let linked2 = CameraService::get_lenses_for_camera(&db, cam.id).await.unwrap();
    assert_eq!(linked2.len(), 1, "Duplicate link should be ignored");

    // Unlink
    CameraService::unlink_lens(&db, cam.id, lens.id).await.unwrap();
    let unlinked = CameraService::get_lenses_for_camera(&db, cam.id).await.unwrap();
    assert!(unlinked.is_empty());
}

#[tokio::test]
async fn delete_camera_cascades_junction() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let cam = common::create_test_camera(&db, mount.id).await;
    let lens = common::create_test_lens(&db, mount.id).await;

    CameraService::link_lens(&db, cam.id, lens.id).await.unwrap();
    CameraService::delete(&db, cam.id).await.unwrap();

    // Junction row should be cascade-deleted
    let cameras = LensService::get_cameras_for_lens(&db, lens.id).await.unwrap();
    assert!(cameras.is_empty(), "Junction should be cascade-deleted with camera");

    // Lens itself should still exist
    let lens_still = LensService::get_by_id(&db, lens.id).await.unwrap();
    assert!(lens_still.is_some(), "Lens should survive camera deletion");
}

#[tokio::test]
async fn maintenance_crud() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let cam = common::create_test_camera(&db, mount.id).await;

    // Create
    let maint = common::create_test_maintenance(&db, cam.id).await;
    assert_eq!(maint.camera_id, cam.id);
    assert_eq!(maint.maintenance_type, MaintenanceType::Cla);
    assert_eq!(maint.done_by, Some("Test Shop".into()));
    assert_eq!(maint.cost, Some(200.0));

    // List
    let list = CameraService::list_maintenance(&db, cam.id).await.unwrap();
    assert_eq!(list.len(), 1);

    // Update
    let mut model: kammerz_lib::entities::camera_maintenance::ActiveModel = maint.into();
    model.cost = Set(Some(250.0));
    model.maintenance_type = Set(MaintenanceType::Repair);
    let updated = CameraService::update_maintenance(&db, model).await.unwrap();
    assert_eq!(updated.cost, Some(250.0));
    assert_eq!(updated.maintenance_type, MaintenanceType::Repair);

    // Delete
    CameraService::delete_maintenance(&db, updated.id).await.unwrap();
    let list2 = CameraService::list_maintenance(&db, cam.id).await.unwrap();
    assert!(list2.is_empty());
}

#[tokio::test]
async fn delete_camera_cascades_maintenance() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let cam = common::create_test_camera(&db, mount.id).await;
    common::create_test_maintenance(&db, cam.id).await;

    CameraService::delete(&db, cam.id).await.unwrap();
    let list = CameraService::list_maintenance(&db, cam.id).await.unwrap();
    assert!(list.is_empty(), "Maintenance should be cascade-deleted with camera");
}

#[tokio::test]
async fn distinct_brands_returns_unique() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // Create two cameras with same brand
    for model_name in ["A", "B"] {
        CameraService::create(
            &db,
            camera::ActiveModel {
                brand: Set("SameBrand".into()),
                model: Set(model_name.into()),
                format: Set(CameraFormat::ThirtyFiveMm),
                lens_mount_id: Set(mount.id),
                created_at: Set(ts.clone()),
                updated_at: Set(ts.clone()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    }

    let brands = CameraService::distinct_brands(&db).await.unwrap();
    let same_count = brands.iter().filter(|b| b.as_str() == "SameBrand").count();
    assert_eq!(same_count, 1, "Distinct brands should deduplicate");
}
