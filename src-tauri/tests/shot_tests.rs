mod common;

use kammerz_lib::entities::shot;
use kammerz_lib::services::shot_service::ShotService;
use sea_orm::Set;

#[tokio::test]
async fn create_and_read_shot() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    let shot = common::create_test_shot(&db, roll.id).await;

    assert_eq!(shot.roll_id, roll.id);
    assert_eq!(shot.frame_number, "1");
    assert_eq!(shot.aperture, Some("f/5.6".into()));
    assert_eq!(shot.shutter_speed, Some("1/125".into()));

    let fetched = ShotService::get_by_id(&db, shot.id).await.unwrap();
    assert!(fetched.is_some());
}

#[tokio::test]
async fn update_shot() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    let shot = common::create_test_shot(&db, roll.id).await;

    let mut model: shot::ActiveModel = shot.into();
    model.aperture = Set(Some("f/2.8".into()));
    model.location = Set(Some("Downtown".into()));

    let updated = ShotService::update(&db, model).await.unwrap();
    assert_eq!(updated.aperture, Some("f/2.8".into()));
    assert_eq!(updated.location, Some("Downtown".into()));
}

#[tokio::test]
async fn delete_shot() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;
    let shot = common::create_test_shot(&db, roll.id).await;

    ShotService::delete(&db, shot.id).await.unwrap();

    let fetched = ShotService::get_by_id(&db, shot.id).await.unwrap();
    assert!(fetched.is_none());
}

#[tokio::test]
async fn list_for_roll_sorted_by_frame_number() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;

    // Create out of order
    common::create_test_shot_numbered(&db, roll.id, "3").await;
    common::create_test_shot_numbered(&db, roll.id, "1").await;
    common::create_test_shot_numbered(&db, roll.id, "10").await;
    common::create_test_shot_numbered(&db, roll.id, "2").await;

    let shots = ShotService::list_for_roll(&db, roll.id).await.unwrap();
    let frames: Vec<&str> = shots.iter().map(|s| s.frame_number.as_str()).collect();
    // ORDER BY CAST(frame_number AS INTEGER), frame_number
    assert_eq!(frames, vec!["1", "2", "3", "10"]);
}

#[tokio::test]
async fn count_for_roll() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;

    assert_eq!(ShotService::count_for_roll(&db, roll.id).await.unwrap(), 0);

    common::create_test_shot_numbered(&db, roll.id, "1").await;
    common::create_test_shot_numbered(&db, roll.id, "2").await;
    common::create_test_shot_numbered(&db, roll.id, "3").await;

    assert_eq!(ShotService::count_for_roll(&db, roll.id).await.unwrap(), 3);
}

// ---------------------------------------------------------------------------
// Shot-Lens junction
// ---------------------------------------------------------------------------

#[tokio::test]
async fn set_and_get_lenses_for_shot() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let lens1 = common::create_test_lens(&db, mount.id).await;
    let lens2 = common::create_test_lens(&db, mount.id).await;
    let roll = common::create_test_roll(&db, None, None).await;
    let shot = common::create_test_shot(&db, roll.id).await;

    // Set two lenses
    ShotService::set_lenses_for_shot(&db, shot.id, vec![lens1.id, lens2.id])
        .await
        .unwrap();

    let lenses = ShotService::get_lenses_for_shot(&db, shot.id).await.unwrap();
    assert_eq!(lenses.len(), 2);
    assert!(lenses.contains(&lens1.id));
    assert!(lenses.contains(&lens2.id));
}

#[tokio::test]
async fn set_lenses_replaces_existing() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let lens1 = common::create_test_lens(&db, mount.id).await;
    let lens2 = common::create_test_lens(&db, mount.id).await;
    let roll = common::create_test_roll(&db, None, None).await;
    let shot = common::create_test_shot(&db, roll.id).await;

    // First set
    ShotService::set_lenses_for_shot(&db, shot.id, vec![lens1.id])
        .await
        .unwrap();

    // Replace with different lens
    ShotService::set_lenses_for_shot(&db, shot.id, vec![lens2.id])
        .await
        .unwrap();

    let lenses = ShotService::get_lenses_for_shot(&db, shot.id).await.unwrap();
    assert_eq!(lenses, vec![lens2.id], "set_lenses should replace, not append");
}

#[tokio::test]
async fn set_empty_lenses_clears_all() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let lens = common::create_test_lens(&db, mount.id).await;
    let roll = common::create_test_roll(&db, None, None).await;
    let shot = common::create_test_shot(&db, roll.id).await;

    ShotService::set_lenses_for_shot(&db, shot.id, vec![lens.id])
        .await
        .unwrap();
    ShotService::set_lenses_for_shot(&db, shot.id, vec![])
        .await
        .unwrap();

    let lenses = ShotService::get_lenses_for_shot(&db, shot.id).await.unwrap();
    assert!(lenses.is_empty());
}

#[tokio::test]
async fn batch_get_lenses_for_roll_shots() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let lens1 = common::create_test_lens(&db, mount.id).await;
    let lens2 = common::create_test_lens(&db, mount.id).await;
    let roll = common::create_test_roll(&db, None, None).await;

    let shot1 = common::create_test_shot_numbered(&db, roll.id, "1").await;
    let shot2 = common::create_test_shot_numbered(&db, roll.id, "2").await;

    ShotService::set_lenses_for_shot(&db, shot1.id, vec![lens1.id])
        .await
        .unwrap();
    ShotService::set_lenses_for_shot(&db, shot2.id, vec![lens1.id, lens2.id])
        .await
        .unwrap();

    let pairs = ShotService::get_lenses_for_roll_shots(&db, roll.id)
        .await
        .unwrap();

    // Should have 3 pairs total: (shot1, lens1), (shot2, lens1), (shot2, lens2)
    assert_eq!(pairs.len(), 3);
    assert!(pairs.contains(&(shot1.id, lens1.id)));
    assert!(pairs.contains(&(shot2.id, lens1.id)));
    assert!(pairs.contains(&(shot2.id, lens2.id)));
}

#[tokio::test]
async fn delete_shot_cascades_lens_junction() {
    let db = common::setup_db().await;
    let mount = common::find_mount_by_name(&db, "Nikon F").await;
    let lens = common::create_test_lens(&db, mount.id).await;
    let roll = common::create_test_roll(&db, None, None).await;
    let shot = common::create_test_shot(&db, roll.id).await;

    ShotService::set_lenses_for_shot(&db, shot.id, vec![lens.id])
        .await
        .unwrap();

    ShotService::delete(&db, shot.id).await.unwrap();

    // The batch query should return nothing for this roll's shots
    let pairs = ShotService::get_lenses_for_roll_shots(&db, roll.id)
        .await
        .unwrap();
    assert!(pairs.is_empty());
}

// ---------------------------------------------------------------------------
// Frame number suggestion
// ---------------------------------------------------------------------------

#[tokio::test]
async fn suggest_next_frame_empty_roll() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;

    let next = ShotService::suggest_next_frame(&db, roll.id).await.unwrap();
    assert_eq!(next, "1");
}

#[tokio::test]
async fn suggest_next_frame_increments() {
    let db = common::setup_db().await;
    let roll = common::create_test_roll(&db, None, None).await;

    common::create_test_shot_numbered(&db, roll.id, "1").await;
    common::create_test_shot_numbered(&db, roll.id, "5").await;
    common::create_test_shot_numbered(&db, roll.id, "3").await;

    let next = ShotService::suggest_next_frame(&db, roll.id).await.unwrap();
    assert_eq!(next, "6", "Should suggest max(frame) + 1");
}
