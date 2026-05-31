mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app, post_json};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn create_shot_roll(app: &axum::Router, roll_id: &str) -> i32 {
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    let payload = json!({
        "roll_id": roll_id,
        "camera_id": camera_id,
        "status": "shot",
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
