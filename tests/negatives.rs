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

// Helper: create a lab-developed roll at `lab-done` (date_received set), return
// (roll_pk, lab_dev_id). Mirrors the create flow the UI uses.
async fn lab_developed_roll(app: &axum::Router) -> (i32, i32) {
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/rolls",
            &json!({ "roll_id": "R-NEG-A", "camera_id": camera_id, "status": "shot", "date_loaded": "2026-06-01" }),
        ))
        .await
        .unwrap();
    let roll_pk: i32 = json_body(res).await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/lab",
            &json!({ "roll_id": roll_pk, "date_received": "2026-07-01" }),
        ))
        .await
        .unwrap();
    let lab_dev_id: i32 = json_body(res).await;
    (roll_pk, lab_dev_id)
}

#[tokio::test]
async fn mark_picked_up_sets_date_logs_event_and_keeps_status() {
    let app = open_app().await;
    let (roll_pk, lab_dev_id) = lab_developed_roll(&app).await;

    let status_before = {
        let res = app
            .clone()
            .oneshot(get(&format!("/api/rolls/{roll_pk}")))
            .await
            .unwrap();
        let r: Value = json_body(res).await;
        r["status"].as_str().unwrap().to_string()
    };

    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/development/lab/{lab_dev_id}"),
            &json!({ "date_negatives_picked_up": "2026-07-10" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_pk}/detail")))
        .await
        .unwrap();
    let detail: Value = json_body(res).await;
    assert_eq!(detail["lab_dev"]["date_negatives_picked_up"], "2026-07-10");
    // Status untouched by a pickup edit.
    assert_eq!(detail["roll"]["status"], status_before);
    // Journal recorded the specialized event.
    let types: Vec<&str> = detail["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["event_type"].as_str().unwrap())
        .collect();
    assert!(types.contains(&"negatives_picked_up"), "events: {types:?}");
}

#[tokio::test]
async fn mark_not_collecting_logs_waived_event() {
    let app = open_app().await;
    let (roll_pk, lab_dev_id) = lab_developed_roll(&app).await;

    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/development/lab/{lab_dev_id}"),
            &json!({ "negatives_not_collecting": true }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_pk}/detail")))
        .await
        .unwrap();
    let detail: Value = json_body(res).await;
    assert_eq!(detail["lab_dev"]["negatives_not_collecting"], true);
    let types: Vec<&str> = detail["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["event_type"].as_str().unwrap())
        .collect();
    assert!(types.contains(&"negatives_waived"), "events: {types:?}");
}

#[tokio::test]
async fn invalid_picked_up_date_is_rejected() {
    let app = open_app().await;
    let (_roll_pk, lab_dev_id) = lab_developed_roll(&app).await;
    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/development/lab/{lab_dev_id}"),
            &json!({ "date_negatives_picked_up": "not-a-date" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn roll_list_computes_negatives_deadline_from_retention() {
    let app = open_app().await;

    // Lab with a 10-day retention.
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/labs",
            &json!({ "name": "Lab R", "negative_retention_days": 10 }),
        ))
        .await
        .unwrap();
    let lab_id: i32 = json_body(res).await;

    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/rolls",
            &json!({ "roll_id": "R-DL", "camera_id": camera_id, "status": "shot" }),
        ))
        .await
        .unwrap();
    let roll_pk: i32 = json_body(res).await;
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/lab",
            &json!({ "roll_id": roll_pk, "lab_id": lab_id, "date_received": "2026-07-01" }),
        ))
        .await
        .unwrap();
    let _lab_dev_id: i32 = json_body(res).await;

    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(roll["lab_name"], "Lab R");
    assert_eq!(roll["negatives_date_received"], "2026-07-01");
    assert_eq!(roll["negatives_deadline"], "2026-07-11"); // +10 days
    assert_eq!(roll["negatives_not_collecting"], false);
}

#[tokio::test]
async fn roll_without_lab_dev_has_null_negatives() {
    let app = open_app().await;
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/rolls",
            &json!({ "roll_id": "R-NONE", "camera_id": camera_id, "status": "loaded" }),
        ))
        .await
        .unwrap();
    let roll_pk: i32 = json_body(res).await;
    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    assert!(roll["negatives_deadline"].is_null());
    assert!(roll["lab_dev_id"].is_null());
}

#[tokio::test]
async fn roll_deadline_uses_default_30_when_lab_retention_null() {
    let app = open_app().await;
    let (roll_pk, _lab_dev_id) = lab_developed_roll(&app).await; // no lab_id → retention NULL → 30
    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(roll["negatives_deadline"], "2026-07-31"); // 2026-07-01 + 30
}

// validate_date_opt accepts bare `YYYY` and `YYYY-MM`, but SQLite's date() can't
// add days to those (it yields a garbage negative-year date for `YYYY` and NULL
// for `YYYY-MM`). The query's `length >= 10` guard must NULL the deadline for a
// partial received date so the roll shows no countdown instead of a bogus one.
#[tokio::test]
async fn partial_date_received_yields_null_deadline() {
    let app = open_app().await;
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    for (roll_id, partial) in [("R-YR", "2026"), ("R-YM", "2026-07")] {
        let res = app
            .clone()
            .oneshot(post_json(
                "/api/rolls",
                &json!({ "roll_id": roll_id, "camera_id": camera_id, "status": "shot" }),
            ))
            .await
            .unwrap();
        let roll_pk: i32 = json_body(res).await;
        let res = app
            .clone()
            .oneshot(post_json(
                "/api/development/lab",
                &json!({ "roll_id": roll_pk, "date_received": partial }),
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let res = app
            .clone()
            .oneshot(get(&format!("/api/rolls/{roll_pk}")))
            .await
            .unwrap();
        let roll: Value = json_body(res).await;
        assert_eq!(roll["negatives_date_received"], partial);
        assert!(
            roll["negatives_deadline"].is_null(),
            "partial received date {partial:?} must yield a null deadline, got {:?}",
            roll["negatives_deadline"]
        );
    }
}
