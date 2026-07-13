mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, open_app_with_db, post_json, put_json};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::{Value, json};
use tower::ServiceExt;

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

#[tokio::test]
async fn lab_retention_create_update_and_validation() {
    let app = open_app().await;

    // Create with retention.
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/labs",
            &json!({ "name": "Lab A", "negative_retention_days": 45 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let id: i32 = json_body(res).await;

    let res = app
        .clone()
        .oneshot(get(&format!("/api/labs/{id}")))
        .await
        .unwrap();
    let lab: Value = json_body(res).await;
    assert_eq!(lab["negative_retention_days"], 45);

    // Update to a new value.
    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/labs/{id}"),
            &json!({ "negative_retention_days": 14 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let res = app
        .clone()
        .oneshot(get(&format!("/api/labs/{id}")))
        .await
        .unwrap();
    let lab: Value = json_body(res).await;
    assert_eq!(lab["negative_retention_days"], 14);

    // Negative value rejected.
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/labs",
            &json!({ "name": "Lab B", "negative_retention_days": -1 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
