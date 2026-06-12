use axum::http::StatusCode;
use serde_json::{json, Value};
use tower::ServiceExt;

mod common;
use common::{get, json_body, open_app, post_json};

#[tokio::test]
async fn list_models_without_key_is_422() {
    // open_app() configures no password and no anthropic_api_key, and the fresh
    // in-memory DB has no `claude_api_key` settings row — so key resolution fails
    // before any network call to Anthropic.
    let app = open_app().await;
    let res = app.oneshot(get("/api/import/models")).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let v: serde_json::Value = json_body(res).await;
    let msg = v["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("No Anthropic API key"),
        "expected the no-API-key message, got: {msg}"
    );
}

// --- POST /api/import/roll (transactional roll + shots + lens links, kammerz-6l5) ---
// Unlike /models and /parse, the import-roll endpoint never touches the
// Anthropic API, so it is fully testable without a key.

#[tokio::test]
async fn import_roll_persists_roll_shots_and_lens_links() {
    let app = open_app().await;

    // Borrow seeded gear for the FKs.
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    let res = app.clone().oneshot(get("/api/lenses")).await.unwrap();
    let lenses: Vec<Value> = json_body(res).await;
    let lens_id = lenses[0]["id"].as_i64().unwrap() as i32;

    let payload = json!({
        "roll_id": "IMPORT-001",
        "camera_id": camera_id,
        "status": "shot",
        "frame_count": 36,
        "date_loaded": "2026-04-01",
        "date_finished": "2026-04-10",
        "notes": "field notes import",
        "shots": [
            {
                "frame_number": "1",
                "aperture": "f/8",
                "shutter_speed": "1/250",
                "date": "2026-04-02",
                "location": "Harbor",
                "lens_ids": [lens_id]
            },
            {
                "frame_number": "2",
                "notes": "no lens recorded"
            }
        ]
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/import/roll", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let roll_pk: i32 = json_body(res).await;

    // The composite detail shows everything the transaction wrote.
    let res = app
        .oneshot(get(&format!("/api/rolls/{roll_pk}/detail")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let detail: Value = json_body(res).await;

    assert_eq!(detail["roll"]["roll_id"], "IMPORT-001");
    assert_eq!(detail["roll"]["status"], "shot");
    assert_eq!(detail["roll"]["frame_count"].as_i64(), Some(36));
    assert_eq!(detail["roll"]["date_loaded"], "2026-04-01");

    let shots = detail["shots"].as_array().expect("shots array");
    assert_eq!(shots.len(), 2, "both imported shots persisted");
    let shot1 = shots
        .iter()
        .find(|s| s["frame_number"] == "1")
        .expect("frame 1 present");
    assert_eq!(shot1["aperture"], "f/8");
    assert_eq!(shot1["shutter_speed"], "1/250");
    assert_eq!(shot1["location"], "Harbor");
    let shot1_id = shot1["id"].as_i64().unwrap() as i32;

    // The shot↔lens junction row from `lens_ids` persisted.
    let pairs = detail["shot_lens_pairs"].as_array().expect("pairs array");
    let pairs: Vec<(i32, i32)> = pairs
        .iter()
        .map(|p| (p[0].as_i64().unwrap() as i32, p[1].as_i64().unwrap() as i32))
        .collect();
    assert_eq!(
        pairs,
        vec![(shot1_id, lens_id)],
        "exactly the one imported lens link exists"
    );
}

#[tokio::test]
async fn import_roll_with_malformed_shot_date_is_rejected_atomically() {
    let app = open_app().await;

    let payload = json!({
        "roll_id": "IMPORT-BAD",
        "status": "shot",
        "shots": [
            { "frame_number": "1", "date": "2026-13-45" }
        ]
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/import/roll", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("shots[0].date"),
        "message names the offending shot field"
    );

    // Nothing persisted — validation rejects before the transaction.
    let res = app.oneshot(get("/api/rolls")).await.unwrap();
    let rolls: Vec<Value> = json_body(res).await;
    assert!(
        !rolls.iter().any(|r| r["roll_id"] == "IMPORT-BAD"),
        "rejected import must not persist the roll"
    );
}

#[tokio::test]
async fn import_roll_with_duplicate_frames_is_rejected_with_targeted_message() {
    let app = open_app().await;

    // Duplicate frame numbers are pre-validated before the transaction opens, so
    // the user gets a message naming the offending shot index and value instead
    // of the generic UNIQUE-constraint error mapped through friendly_err.
    let payload = json!({
        "roll_id": "IMPORT-DUP",
        "status": "shot",
        "shots": [
            { "frame_number": "1" },
            { "frame_number": "1" }
        ]
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/import/roll", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    let msg = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("shots[1]") && msg.contains("duplicate frame number") && msg.contains("\"1\""),
        "message names the offending shot index and quoted value, got: {msg}"
    );

    let res = app.oneshot(get("/api/rolls")).await.unwrap();
    let rolls: Vec<Value> = json_body(res).await;
    assert!(
        !rolls.iter().any(|r| r["roll_id"] == "IMPORT-DUP"),
        "a rejected import must not persist the roll"
    );
}

#[tokio::test]
async fn import_roll_duplicate_frames_collide_after_trimming() {
    let app = open_app().await;

    // Frame numbers are trimmed before persistence, so " 1 " and "1" would both
    // land as "1" and violate UNIQUE(roll_id, frame_number). Pre-validation must
    // compare the trimmed values, matching what the DB would see.
    let payload = json!({
        "roll_id": "IMPORT-DUP-TRIM",
        "status": "shot",
        "shots": [
            { "frame_number": "1" },
            { "frame_number": " 1 " }
        ]
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/import/roll", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("duplicate frame number"),
        "trimmed-equal frame numbers must be flagged as duplicates"
    );

    let res = app.oneshot(get("/api/rolls")).await.unwrap();
    let rolls: Vec<Value> = json_body(res).await;
    assert!(
        !rolls.iter().any(|r| r["roll_id"] == "IMPORT-DUP-TRIM"),
        "a rejected import must not persist the roll"
    );
}

#[tokio::test]
async fn import_roll_with_empty_frame_number_is_rejected() {
    let app = open_app().await;

    // An empty/whitespace frame_number is meaningless for the per-frame log and
    // must be rejected with a message naming the offending shot index.
    let payload = json!({
        "roll_id": "IMPORT-EMPTY",
        "status": "shot",
        "shots": [
            { "frame_number": "1" },
            { "frame_number": "   " }
        ]
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/import/roll", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    let msg = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("shots[1]") && msg.contains("frame number is required"),
        "message names the offending shot index, got: {msg}"
    );

    let res = app.oneshot(get("/api/rolls")).await.unwrap();
    let rolls: Vec<Value> = json_body(res).await;
    assert!(
        !rolls.iter().any(|r| r["roll_id"] == "IMPORT-EMPTY"),
        "a rejected import must not persist the roll"
    );
}
