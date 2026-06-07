mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, post_json};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn create_shot_roll(app: &axum::Router, roll_id: &str) -> i32 {
    create_roll_at_status(app, roll_id, "shot").await
}

/// Build a PUT request with a JSON body (the common helpers only cover GET/POST).
fn put_json(path: &str, value: &Value) -> axum::http::Request<axum::body::Body> {
    axum::http::Request::builder()
        .method("PUT")
        .uri(path)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(serde_json::to_vec(value).unwrap()))
        .unwrap()
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
