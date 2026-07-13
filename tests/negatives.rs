mod common;

use common::open_app_with_db;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

// Proves the three new columns exist and round-trip through the entities.
#[tokio::test]
async fn new_negatives_columns_round_trip() {
    let (_app, db) = open_app_with_db().await;

    let lab = entity::lab::ActiveModel {
        name: Set("The Darkroom".into()),
        negative_retention_days: Set(Some(45)),
        created_at: Set("2026-07-01 00:00:00".into()),
        updated_at: Set("2026-07-01 00:00:00".into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    assert_eq!(lab.negative_retention_days, Some(45));

    // A roll to hang the lab dev off (FK). Insert minimally via entity.
    let roll = entity::roll::ActiveModel {
        roll_id: Set("R-NEG-1".into()),
        status: Set(entity::roll::RollStatus::Shot),
        created_at: Set("2026-07-01 00:00:00".into()),
        updated_at: Set("2026-07-01 00:00:00".into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let dev = entity::development_lab::ActiveModel {
        roll_id: Set(roll.id),
        lab_id: Set(Some(lab.id)),
        date_received: Set(Some("2026-07-01".into())),
        date_negatives_picked_up: Set(Some("2026-07-05".into())),
        negatives_not_collecting: Set(true),
        created_at: Set("2026-07-01 00:00:00".into()),
        updated_at: Set("2026-07-01 00:00:00".into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let fetched = entity::development_lab::Entity::find_by_id(dev.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        fetched.date_negatives_picked_up.as_deref(),
        Some("2026-07-05")
    );
    assert!(fetched.negatives_not_collecting);

    // Default when unset is false. Needs a second roll: development_labs.roll_id
    // has a pre-existing UNIQUE index (one lab-dev record per roll, added in
    // m20260222_000019_schema_hardening), so this can't reuse `roll.id`.
    let roll2 = entity::roll::ActiveModel {
        roll_id: Set("R-NEG-2".into()),
        status: Set(entity::roll::RollStatus::Shot),
        created_at: Set("2026-07-01 00:00:00".into()),
        updated_at: Set("2026-07-01 00:00:00".into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let dev2 = entity::development_lab::ActiveModel {
        roll_id: Set(roll2.id),
        created_at: Set("2026-07-01 00:00:00".into()),
        updated_at: Set("2026-07-01 00:00:00".into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    assert!(!dev2.negatives_not_collecting);
}
