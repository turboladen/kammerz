mod common;

use axum::http::StatusCode;
use common::{delete, get, json_body, open_app, open_app_with_db, post_json, put_json};
use sea_orm::{ActiveModelTrait, Set};
use serde_json::{json, Value};
use tower::ServiceExt;

/// Seed a self dev directly in the DB, bypassing the API's lab/self
/// mutual-exclusion guard (kammerz-ysw) — simulates legacy both-dev data.
async fn insert_self_dev_directly(db: &sea_orm::DatabaseConnection, roll_pk: i32) {
    entity::development_self::ActiveModel {
        roll_id: Set(roll_pk),
        developer: Set(Some("Rodinal".into())),
        created_at: Set("2026-05-01T00:00:00Z".into()),
        updated_at: Set("2026-05-01T00:00:00Z".into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// Lab-side twin of [`insert_self_dev_directly`].
async fn insert_lab_dev_directly(db: &sea_orm::DatabaseConnection, roll_pk: i32) {
    entity::development_lab::ActiveModel {
        roll_id: Set(roll_pk),
        created_at: Set("2026-05-01T00:00:00Z".into()),
        updated_at: Set("2026-05-01T00:00:00Z".into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn create_shot_roll(app: &axum::Router, roll_id: &str) -> i32 {
    create_roll_at_status(app, roll_id, "shot").await
}

/// Fetch a roll's current status string.
async fn roll_status(app: &axum::Router, roll_pk: i32) -> String {
    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    roll["status"].as_str().unwrap().to_string()
}

/// Create a roll directly at a given status. Used to simulate imported rolls
/// orphaned past 'shot' with no backing dev record.
async fn create_roll_at_status(app: &axum::Router, roll_id: &str, status: &str) -> i32 {
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    let payload = json!({
        "roll_id": roll_id,
        "camera_id": camera_id,
        "status": status,
        "date_loaded": "2026-05-01"
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/rolls", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    json_body(res).await
}

#[tokio::test]
async fn create_self_dev_with_stages_and_lists() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "DEV-SELF").await;

    // POST a self-dev with a couple of stages — exercises the transactional
    // create + set_stages + auto_sync_status path.
    let payload = json!({
        "roll_id": roll_pk,
        "developer": "Rodinal",
        "developer_dilution": "1:50",
        "temperature": "20C",
        "stages": [
            { "stage_name": "Develop", "duration_seconds": 660, "notes": null, "sort_order": 0 },
            { "stage_name": "Fix", "duration_seconds": 300, "notes": "fresh fixer", "sort_order": 1 }
        ]
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/development/self", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let dev_id: i32 = json_body(res).await;

    // Stages persisted, ordered by sort_order.
    let res = app
        .clone()
        .oneshot(get(&format!("/api/development/self/{dev_id}/stages")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let stages: Vec<Value> = json_body(res).await;
    assert_eq!(stages.len(), 2);
    assert_eq!(stages[0]["stage_name"], "Develop");
    assert_eq!(stages[0]["duration_seconds"], 660);
    assert_eq!(stages[1]["stage_name"], "Fix");

    // Roll status auto-advanced → developing inside the transaction.
    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(roll["status"], "developing");

    // list_all_self_developments includes our record with its merged stages.
    let res = app
        .clone()
        .oneshot(get("/api/development/self"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let all: Vec<Value> = json_body(res).await;
    let ours = all
        .iter()
        .find(|d| d["dev_id"].as_i64() == Some(dev_id as i64))
        .expect("our self-dev appears in list_all");
    assert_eq!(ours["stages"].as_array().unwrap().len(), 2);

    // for-roll lookup returns the same record.
    let res = app
        .oneshot(get(&format!("/api/development/self/for-roll/{roll_pk}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let sd: Value = json_body(res).await;
    assert_eq!(sd["id"].as_i64().unwrap() as i32, dev_id);
}

// kammerz-rv7: the /developments page must include lab developments (it was
// self-only, leaving lab-path users a permanently empty page). GET
// /api/development/lab lists every lab dev with its joined roll context, lab
// name, drop-off/received dates, and cost.
#[tokio::test]
async fn list_all_lab_developments_includes_roll_and_lab_context() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "LAB-LIST").await;

    // A named lab so the join surfaces a non-null lab_name.
    let res = app
        .clone()
        .oneshot(post_json("/api/labs", &json!({ "name": "The Darkroom" })))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let lab_id: i32 = json_body(res).await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/lab",
            &json!({
                "roll_id": roll_pk,
                "lab_id": lab_id,
                "date_dropped_off": "2026-05-01",
                "date_received": "2026-05-10",
                "cost": 18.5
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let dev_id: i32 = json_body(res).await;

    let res = app.oneshot(get("/api/development/lab")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let all: Vec<Value> = json_body(res).await;
    let ours = all
        .iter()
        .find(|d| d["dev_id"].as_i64() == Some(dev_id as i64))
        .expect("our lab dev appears in list_all");

    assert_eq!(ours["roll_pk"].as_i64().unwrap() as i32, roll_pk);
    assert_eq!(ours["roll_id"], "LAB-LIST");
    assert_eq!(ours["lab_name"], "The Darkroom");
    assert_eq!(ours["date_dropped_off"], "2026-05-01");
    assert_eq!(ours["date_received"], "2026-05-10");
    assert_eq!(ours["cost"].as_f64().unwrap(), 18.5);
}

// kammerz-rv7: with no lab devs at all the endpoint returns an empty array (200),
// not an error — so the page renders its empty state instead of failing.
#[tokio::test]
async fn list_all_lab_developments_empty_returns_ok() {
    let app = open_app().await;
    let res = app.oneshot(get("/api/development/lab")).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let all: Vec<Value> = json_body(res).await;
    assert!(all.is_empty(), "no lab devs seeded → empty list");
}

#[tokio::test]
async fn create_lab_dev_advances_status() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "DEV-LAB").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/lab",
            &json!({ "roll_id": roll_pk, "cost": 12.5 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = app
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(roll["status"], "at-lab", "lab dev advances roll to at-lab");
}

// kammerz-afc: an imported roll orphaned at at-lab (no lab dev record). Clicking
// 'Lab Done' opens the lab dialog; entering date_received + Save must land the
// roll at lab-done in ONE action — the create is data-driven (a received date
// means the lab is done), not stranded at at-lab requiring a second click.
#[tokio::test]
async fn create_lab_dev_with_received_date_advances_orphan_at_lab_to_lab_done() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "DEV-ORPHAN-LAB", "at-lab").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/lab",
            &json!({ "roll_id": roll_pk, "date_received": "2026-05-10" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = app
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(
        roll["status"], "lab-done",
        "lab dev with received date lands an orphaned at-lab roll at lab-done in one action"
    );
}

// Symmetric self-dev case: orphan at 'developing' (no self dev). Recording a
// self dev with date_processed (= developed) advances to 'developed' in one action.
#[tokio::test]
async fn create_self_dev_with_processed_date_advances_orphan_developing_to_developed() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "DEV-ORPHAN-SELF", "developing").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/self",
            &json!({ "roll_id": roll_pk, "date_processed": "2026-05-12" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = app
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(
        roll["status"], "developed",
        "self dev with processed date lands an orphaned developing roll at developed in one action"
    );
}

// Regression guard: a self dev with NO processed date on a fresh 'shot' roll
// advances only to 'developing' (the normal shot→developing transition is
// unchanged — the date-driven jump is opt-in via the date field).
#[tokio::test]
async fn create_self_dev_without_processed_date_stops_at_developing() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "DEV-SELF-NODATE").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/self",
            &json!({ "roll_id": roll_pk, "developer": "HC-110" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    let res = app
        .oneshot(get(&format!("/api/rolls/{roll_pk}")))
        .await
        .unwrap();
    let roll: Value = json_body(res).await;
    assert_eq!(
        roll["status"], "developing",
        "self dev without a processed date advances only to developing"
    );
}

// kammerz-42u: editing an existing lab dev to ADD a received date must advance
// at-lab → lab-done in one save (the Edit dialog path, not the chevron).
#[tokio::test]
async fn update_lab_dev_adds_received_date_advances_to_lab_done() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "DEV-LAB-UPD-ADD").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/lab",
            &json!({ "roll_id": roll_pk, "cost": 12.5 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let dev_id: i32 = json_body(res).await;
    assert_eq!(roll_status(&app, roll_pk).await, "at-lab");

    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/development/lab/{dev_id}"),
            &json!({ "date_received": "2026-05-10" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "lab-done",
        "adding a received date via PUT advances at-lab → lab-done"
    );
}

// kammerz-42u: clearing the received date on an existing lab dev reverts
// lab-done → at-lab (symmetric revert). Send an explicit null to clear.
#[tokio::test]
async fn update_lab_dev_clears_received_date_reverts_to_at_lab() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "DEV-LAB-UPD-CLR", "at-lab").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/lab",
            &json!({ "roll_id": roll_pk, "date_received": "2026-05-10" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let dev_id: i32 = json_body(res).await;
    assert_eq!(roll_status(&app, roll_pk).await, "lab-done");

    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/development/lab/{dev_id}"),
            &json!({ "date_received": null }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "at-lab",
        "clearing the received date via PUT reverts lab-done → at-lab"
    );
}

// kammerz-42u: editing an existing self dev to ADD a processed date advances
// developing → developed in one save.
#[tokio::test]
async fn update_self_dev_adds_processed_date_advances_to_developed() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "DEV-SELF-UPD-ADD").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/self",
            &json!({ "roll_id": roll_pk, "developer": "HC-110" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let dev_id: i32 = json_body(res).await;
    assert_eq!(roll_status(&app, roll_pk).await, "developing");

    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/development/self/{dev_id}"),
            &json!({ "date_processed": "2026-05-12" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "developed",
        "adding a processed date via PUT advances developing → developed"
    );
}

// kammerz-42u: clearing the processed date on an existing self dev reverts
// developed → developing (symmetric revert).
#[tokio::test]
async fn update_self_dev_clears_processed_date_reverts_to_developing() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "DEV-SELF-UPD-CLR", "developing").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/self",
            &json!({ "roll_id": roll_pk, "date_processed": "2026-05-12" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let dev_id: i32 = json_body(res).await;
    assert_eq!(roll_status(&app, roll_pk).await, "developed");

    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/development/self/{dev_id}"),
            &json!({ "date_processed": null }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "developing",
        "clearing the processed date via PUT reverts developed → developing"
    );
}

// kammerz-42u no-regression guard: a roll already past the completed status
// (scanned) is NOT pulled back when a dev date is cleared — the revert is scoped
// to the one adjacent rung (lab-done → at-lab), never scanned → at-lab.
#[tokio::test]
async fn update_lab_dev_clears_received_date_leaves_scanned_untouched() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "DEV-LAB-SCANNED", "scanned").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/lab",
            &json!({ "roll_id": roll_pk, "date_received": "2026-05-10" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let dev_id: i32 = json_body(res).await;
    // create is advance-only, so the scanned roll is unchanged by the POST.
    assert_eq!(roll_status(&app, roll_pk).await, "scanned");

    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/development/lab/{dev_id}"),
            &json!({ "date_received": null }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "scanned",
        "clearing a received date must not pull a scanned roll back to at-lab"
    );
}

// kammerz-ysw: lab and self dev are mutually exclusive — the backend must
// reject a self dev when a lab dev already exists (the UI hides the "+ Self"
// button, but a stale tab on another device or a raw API call bypasses that).
// Without the guard the roll's status strands on the first path and deleting
// either record can never auto-revert it.
#[tokio::test]
async fn create_self_dev_rejected_when_lab_dev_exists() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "DEV-EXCL-LAB-FIRST").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/lab",
            &json!({ "roll_id": roll_pk }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    assert_eq!(roll_status(&app, roll_pk).await, "at-lab");

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/self",
            &json!({ "roll_id": roll_pk, "developer": "HC-110" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    assert_eq!(
        body["error"]["message"],
        "This roll already has a lab development record — delete it first."
    );

    // Nothing was inserted and the roll stays on the lab flow.
    let res = app
        .clone()
        .oneshot(get(&format!("/api/development/self/for-roll/{roll_pk}")))
        .await
        .unwrap();
    let sd: Value = json_body(res).await;
    assert!(sd.is_null(), "rejected self dev must not be persisted");
    assert_eq!(roll_status(&app, roll_pk).await, "at-lab");
}

// kammerz-ysw: mirror case — a lab dev is rejected when a self dev exists.
#[tokio::test]
async fn create_lab_dev_rejected_when_self_dev_exists() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "DEV-EXCL-SELF-FIRST").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/self",
            &json!({ "roll_id": roll_pk, "developer": "Rodinal" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    assert_eq!(roll_status(&app, roll_pk).await, "developing");

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/lab",
            &json!({ "roll_id": roll_pk, "cost": 12.5 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    assert_eq!(
        body["error"]["message"],
        "This roll already has a self development record — delete it first."
    );

    // Nothing was inserted and the roll stays on the self flow.
    let res = app
        .clone()
        .oneshot(get(&format!("/api/development/lab/for-roll/{roll_pk}")))
        .await
        .unwrap();
    let ld: Value = json_body(res).await;
    assert!(ld.is_null(), "rejected lab dev must not be persisted");
    assert_eq!(roll_status(&app, roll_pk).await, "developing");
}

// kammerz-rwa: deleting a lab dev that doesn't exist (e.g. a stale-id
// double-delete from the frontend) must return 404 NOT_FOUND, not 422. The
// lookup runs inside the txn closure; or_404_db + friendly_txn_err classify the
// resulting DbErr::RecordNotFound as a 404 (matching non-transactional handlers).
#[tokio::test]
async fn delete_missing_lab_dev_returns_404() {
    let app = open_app().await;

    let res = app
        .oneshot(delete("/api/development/lab/999999"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    assert_eq!(body["error"]["message"], "Lab development 999999 not found");
}

// kammerz-rwa: symmetric self-dev case — delete of a missing self dev is 404.
#[tokio::test]
async fn delete_missing_self_dev_returns_404() {
    let app = open_app().await;

    let res = app
        .oneshot(delete("/api/development/self/999999"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    assert_eq!(
        body["error"]["message"],
        "Self development 999999 not found"
    );
}

// --- kammerz-8rh: delete-side status reverts + sibling/no-regression branches ---

/// POST a lab dev for `roll_id` and return its id. `body` lets callers add
/// date_received etc. on top of the roll linkage.
async fn create_lab_dev(app: &axum::Router, roll_id: i32, extra: Value) -> i32 {
    let mut payload = json!({ "roll_id": roll_id });
    for (k, v) in extra.as_object().unwrap() {
        payload[k] = v.clone();
    }
    let res = app
        .clone()
        .oneshot(post_json("/api/development/lab", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    json_body(res).await
}

/// POST a self dev for `roll_id` and return its id.
async fn create_self_dev(app: &axum::Router, roll_id: i32, extra: Value) -> i32 {
    let mut payload = json!({ "roll_id": roll_id });
    for (k, v) in extra.as_object().unwrap() {
        payload[k] = v.clone();
    }
    let res = app
        .clone()
        .oneshot(post_json("/api/development/self", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    json_body(res).await
}

// kammerz-8rh: deleting the lab dev reverts at-lab → shot (the data that drove
// the status forward is gone, so the status follows it back).
#[tokio::test]
async fn delete_lab_dev_reverts_at_lab_to_shot() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "DEL-LAB-ATLAB").await;
    let dev_id = create_lab_dev(&app, roll_pk, json!({})).await;
    assert_eq!(roll_status(&app, roll_pk).await, "at-lab");

    let res = app
        .clone()
        .oneshot(delete(&format!("/api/development/lab/{dev_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "shot",
        "deleting the lab dev reverts at-lab → shot"
    );
}

// kammerz-8rh: the revert covers both rungs of the lab range — lab-done → shot.
#[tokio::test]
async fn delete_lab_dev_reverts_lab_done_to_shot() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "DEL-LAB-DONE").await;
    let dev_id = create_lab_dev(&app, roll_pk, json!({ "date_received": "2026-05-10" })).await;
    assert_eq!(roll_status(&app, roll_pk).await, "lab-done");

    let res = app
        .clone()
        .oneshot(delete(&format!("/api/development/lab/{dev_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "shot",
        "deleting the lab dev reverts lab-done → shot"
    );
}

// kammerz-8rh: deleting the self dev reverts developing → shot.
#[tokio::test]
async fn delete_self_dev_reverts_developing_to_shot() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "DEL-SELF-DEVING").await;
    let dev_id = create_self_dev(&app, roll_pk, json!({ "developer": "HC-110" })).await;
    assert_eq!(roll_status(&app, roll_pk).await, "developing");

    let res = app
        .clone()
        .oneshot(delete(&format!("/api/development/self/{dev_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "shot",
        "deleting the self dev reverts developing → shot"
    );
}

// kammerz-8rh: the revert covers both rungs of the self range — developed → shot.
#[tokio::test]
async fn delete_self_dev_reverts_developed_to_shot() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "DEL-SELF-DEVED").await;
    let dev_id = create_self_dev(&app, roll_pk, json!({ "date_processed": "2026-05-12" })).await;
    assert_eq!(roll_status(&app, roll_pk).await, "developed");

    let res = app
        .clone()
        .oneshot(delete(&format!("/api/development/self/{dev_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "shot",
        "deleting the self dev reverts developed → shot"
    );
}

// kammerz-8rh sibling-skip: when BOTH dev records exist, deleting the lab dev
// must NOT revert the status because the surviving self dev still justifies a
// post-shot status. The API rejects creating the sibling since kammerz-ysw, so
// seed it directly in the DB — simulating a roll corrupted before that guard
// existed; the delete handlers keep the sibling check as defense-in-depth.
#[tokio::test]
async fn delete_lab_dev_skips_revert_when_self_dev_exists() {
    let (app, db) = open_app_with_db().await;
    let roll_pk = create_shot_roll(&app, "DEL-LAB-SIBLING").await;
    let lab_id = create_lab_dev(&app, roll_pk, json!({})).await;
    assert_eq!(roll_status(&app, roll_pk).await, "at-lab");
    insert_self_dev_directly(&db, roll_pk).await;
    assert_eq!(roll_status(&app, roll_pk).await, "at-lab");

    let res = app
        .clone()
        .oneshot(delete(&format!("/api/development/lab/{lab_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "at-lab",
        "lab dev delete skips the revert while a sibling self dev exists"
    );
}

// kammerz-8rh sibling-skip, mirrored: deleting the self dev while a lab dev
// survives must not revert. As above, the lab sibling is seeded directly in
// the DB because the API has rejected sibling creates since kammerz-ysw.
#[tokio::test]
async fn delete_self_dev_skips_revert_when_lab_dev_exists() {
    let (app, db) = open_app_with_db().await;
    let roll_pk = create_shot_roll(&app, "DEL-SELF-SIBLING").await;
    let self_id = create_self_dev(&app, roll_pk, json!({ "developer": "HC-110" })).await;
    assert_eq!(roll_status(&app, roll_pk).await, "developing");
    insert_lab_dev_directly(&db, roll_pk).await;
    assert_eq!(roll_status(&app, roll_pk).await, "developing");

    let res = app
        .clone()
        .oneshot(delete(&format!("/api/development/self/{self_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "developing",
        "self dev delete skips the revert while a sibling lab dev exists"
    );
}

// kammerz-8rh no-regression: status beyond the dev record's range is untouched —
// deleting a lab dev from a scanned roll must not pull it back to shot.
#[tokio::test]
async fn delete_lab_dev_leaves_scanned_untouched() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "DEL-LAB-SCANNED", "scanned").await;
    let dev_id = create_lab_dev(&app, roll_pk, json!({})).await;
    assert_eq!(roll_status(&app, roll_pk).await, "scanned");

    let res = app
        .clone()
        .oneshot(delete(&format!("/api/development/lab/{dev_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "scanned",
        "deleting a lab dev at scanned leaves the status untouched"
    );
}

// kammerz-8rh no-regression, mirrored for the self path.
#[tokio::test]
async fn delete_self_dev_leaves_scanned_untouched() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "DEL-SELF-SCANNED", "scanned").await;
    let dev_id = create_self_dev(&app, roll_pk, json!({ "developer": "HC-110" })).await;
    assert_eq!(roll_status(&app, roll_pk).await, "scanned");

    let res = app
        .clone()
        .oneshot(delete(&format!("/api/development/self/{dev_id}")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "scanned",
        "deleting a self dev at scanned leaves the status untouched"
    );
}

// kammerz-8rh: the CREATE path is forward-only — POSTing a lab dev (no received
// date) onto a roll already at 'scanned' must not drag it back to at-lab. (The
// update path's equivalent is covered at
// update_lab_dev_clears_received_date_leaves_scanned_untouched.)
#[tokio::test]
async fn create_lab_dev_on_scanned_roll_stays_scanned() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "CREATE-LAB-SCANNED", "scanned").await;

    create_lab_dev(&app, roll_pk, json!({ "lab_name": "The Darkroom" })).await;
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "scanned",
        "creating a lab dev on a scanned roll is a forward-only no-op"
    );
}

// kammerz-8rh: mirrored for the self path — create at 'scanned' stays 'scanned'.
#[tokio::test]
async fn create_self_dev_on_scanned_roll_stays_scanned() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "CREATE-SELF-SCANNED", "scanned").await;

    create_self_dev(&app, roll_pk, json!({ "developer": "Rodinal" })).await;
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "scanned",
        "creating a self dev on a scanned roll is a forward-only no-op"
    );
}
