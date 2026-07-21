use axum::http::StatusCode;
use serde_json::{Value, json};
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
    // The imported `status: "shot"` backfilled date_finished; the lifecycle is then
    // derived (ADR-0013) — finished shooting, no dev → group_key 1, badge "To develop".
    assert_eq!(detail["roll"]["group_key"], 1);
    assert_eq!(detail["roll"]["badge"], "To develop");
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

// --- Server-side input validation (kammerz-grd) ---
// Complements the client-side guards added in PR #75 with authoritative server
// checks on the import-roll payload.

#[tokio::test]
async fn import_roll_rejects_whitespace_roll_id() {
    let app = open_app().await;
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    let res = app
        .oneshot(post_json(
            "/api/import/roll",
            &json!({
                "roll_id": "   ",
                "camera_id": camera_id,
                "status": "shot",
                "shots": []
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("roll_id")
    );
}

#[tokio::test]
async fn import_roll_rejects_negative_frame_count() {
    let app = open_app().await;
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    let res = app
        .oneshot(post_json(
            "/api/import/roll",
            &json!({
                "roll_id": "IMPORT-NEG",
                "camera_id": camera_id,
                "status": "shot",
                "frame_count": -1,
                "shots": []
            }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("frame_count")
    );
}

#[tokio::test]
async fn import_roll_persists_shot_time() {
    let app = open_app().await;

    let payload = json!({
        "roll_id": "IMPORT-TIME",
        "status": "archived",
        "shots": [
            { "frame_number": "1", "time": "07:27" }
        ]
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/import/roll", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let roll_pk: i32 = json_body(res).await;

    let res = app
        .oneshot(get(&format!("/api/rolls/{roll_pk}/detail")))
        .await
        .unwrap();
    let detail: Value = json_body(res).await;
    let shots = detail["shots"].as_array().expect("shots array");
    assert_eq!(shots[0]["time"], "07:27", "imported shot time persisted");
}

#[tokio::test]
async fn import_roll_with_malformed_shot_time_is_rejected() {
    let app = open_app().await;

    let payload = json!({
        "roll_id": "IMPORT-BADTIME",
        "status": "archived",
        "shots": [
            { "frame_number": "1", "time": "7:27pm" }
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
            .unwrap()
            .contains("shots[0].time"),
        "message names the offending shot field"
    );
}

#[tokio::test]
async fn import_roll_blank_shot_time_persists_as_null() {
    let app = open_app().await;

    // A whitespace-only time passes validation (blank is optional) but must
    // persist as NULL — canonical-HH:MM-or-NULL — not an empty string, matching
    // the create/update paths' trim_opt behaviour.
    let payload = json!({
        "roll_id": "IMPORT-BLANKTIME",
        "status": "archived",
        "shots": [
            { "frame_number": "1", "time": "   " }
        ]
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/import/roll", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let roll_pk: i32 = json_body(res).await;

    let res = app
        .oneshot(get(&format!("/api/rolls/{roll_pk}/detail")))
        .await
        .unwrap();
    let detail: Value = json_body(res).await;
    let shots = detail["shots"].as_array().expect("shots array");
    assert_eq!(
        shots[0]["time"],
        Value::Null,
        "blank import time stored as NULL"
    );
}

// --- Legacy-status → derived-lifecycle round-trips (kammerz-gsj6) ---
// Each imports a roll at a legacy status (with a real date_finished so the
// honest-borrow anchor exists) then reads the composite detail and asserts the
// derived activity fields AND the synthesized dev record match the intended
// lifecycle. Together these assert every "anchor present" row of the fidelity
// table. See src/routes/import.rs::import_lifecycle.

/// Import a roll at `status` (anchor = date_finished 2026-04-10) and return its
/// `/detail`. `date_finished` gives a real recorded date to borrow so terminal /
/// completed statuses reach their intended group_key.
async fn import_status_detail(app: &axum::Router, roll_id: &str, status: &str) -> Value {
    let payload = json!({
        "roll_id": roll_id,
        "status": status,
        "date_loaded": "2026-04-01",
        "date_finished": "2026-04-10",
        "shots": []
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/import/roll", &payload))
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::CREATED,
        "import at status {status} should succeed"
    );
    let roll_pk: i32 = json_body(res).await;
    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_pk}/detail")))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    json_body(res).await
}

#[tokio::test]
async fn import_at_lab_creates_lab_dev_and_derives_developing() {
    let app = open_app().await;
    let detail = import_status_detail(&app, "IMPORT-ATLAB", "at-lab").await;
    assert_eq!(detail["roll"]["group_key"], 1);
    assert_eq!(detail["roll"]["badge"], "Developing");
    // Lab dev record exists (in progress — no date_received), no self dev.
    assert!(
        detail["lab_dev"].is_object(),
        "at-lab creates a lab dev record"
    );
    assert_eq!(detail["lab_dev"]["date_received"], Value::Null);
    assert!(detail["self_dev"].is_null(), "no self dev for a lab status");
}

#[tokio::test]
async fn import_developing_creates_self_dev_and_derives_developing() {
    let app = open_app().await;
    let detail = import_status_detail(&app, "IMPORT-DEVING", "developing").await;
    assert_eq!(detail["roll"]["group_key"], 1);
    assert_eq!(detail["roll"]["badge"], "Developing");
    assert!(
        detail["self_dev"].is_object(),
        "developing creates a self dev record"
    );
    assert_eq!(detail["self_dev"]["date_processed"], Value::Null);
    assert!(detail["lab_dev"].is_null(), "no lab dev for a self status");
}

#[tokio::test]
async fn import_lab_done_borrows_completion_and_derives_to_scan() {
    let app = open_app().await;
    let detail = import_status_detail(&app, "IMPORT-LABDONE", "lab-done").await;
    assert_eq!(detail["roll"]["group_key"], 2);
    assert_eq!(detail["roll"]["badge"], "To scan");
    // date_received borrowed from the anchor (finished-shooting date) so the
    // development activity derives done.
    assert_eq!(detail["lab_dev"]["date_received"], "2026-04-10");
    assert!(detail["self_dev"].is_null());
}

#[tokio::test]
async fn import_developed_borrows_completion_and_derives_to_scan() {
    let app = open_app().await;
    let detail = import_status_detail(&app, "IMPORT-DEVED", "developed").await;
    assert_eq!(detail["roll"]["group_key"], 2);
    assert_eq!(detail["roll"]["badge"], "To scan");
    assert_eq!(detail["self_dev"]["date_processed"], "2026-04-10");
    assert!(detail["lab_dev"].is_null());
}

#[tokio::test]
async fn import_scanned_derives_to_edit_with_no_dev_record() {
    let app = open_app().await;
    let detail = import_status_detail(&app, "IMPORT-SCANNED", "scanned").await;
    assert_eq!(detail["roll"]["group_key"], 3);
    assert_eq!(detail["roll"]["badge"], "To edit");
    // Recordless-tail: development derives implicitly-done from date_scanned, so
    // no dev record is (or can be) synthesized for a terminal status.
    assert!(
        detail["lab_dev"].is_null(),
        "no lab dev for a terminal status"
    );
    assert!(
        detail["self_dev"].is_null(),
        "no self dev for a terminal status"
    );
    assert_eq!(detail["roll"]["date_scanned"], "2026-04-10");
}

#[tokio::test]
async fn import_post_processed_derives_to_archive() {
    let app = open_app().await;
    let detail = import_status_detail(&app, "IMPORT-PP", "post-processed").await;
    assert_eq!(detail["roll"]["group_key"], 4);
    assert_eq!(detail["roll"]["badge"], "To archive");
    assert_eq!(detail["roll"]["date_scanned"], "2026-04-10");
    assert_eq!(detail["roll"]["date_post_processed"], "2026-04-10");
}

#[tokio::test]
async fn import_archived_derives_done() {
    let app = open_app().await;
    let detail = import_status_detail(&app, "IMPORT-ARCHIVED", "archived").await;
    assert_eq!(detail["roll"]["group_key"], 5);
    assert_eq!(detail["roll"]["badge"], "Done");
    assert_eq!(detail["roll"]["done"], true);
    assert_eq!(detail["roll"]["date_archived"], "2026-04-10");
}

#[tokio::test]
async fn import_archived_with_no_dates_degrades_to_develop() {
    // Documented degradation: with NO date to borrow (no loaded/finished/shot
    // dates), an archived import has no honest anchor at all — not even a
    // date_finished — so every date stays unset and the roll derives all the way
    // back to group_key 0 "Loaded". Nothing is fabricated (kammerz-gsj6).
    let app = open_app().await;
    let payload = json!({
        "roll_id": "IMPORT-ARCH-NODATES",
        "status": "archived",
        "shots": []
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/import/roll", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let roll_pk: i32 = json_body(res).await;
    let res = app
        .oneshot(get(&format!("/api/rolls/{roll_pk}/detail")))
        .await
        .unwrap();
    let detail: Value = json_body(res).await;
    assert_eq!(detail["roll"]["group_key"], 0);
    assert_eq!(detail["roll"]["badge"], "Loaded");
    assert_eq!(detail["roll"]["date_archived"], Value::Null);
}

#[tokio::test]
async fn import_roll_with_unknown_status_is_422() {
    // With the RollStatus enum retired, `status` is a plain string consumed only
    // by the date backfill — an unknown value would otherwise silently no-op and
    // import the roll with an unintended derived lifecycle. The handler must
    // restore the 422 the enum used to provide via serde.
    let app = open_app().await;
    let payload = json!({
        "roll_id": "IMPORT-BADSTATUS",
        "status": "developped",
        "shots": []
    });
    let res = app
        .oneshot(post_json("/api/import/roll", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    let msg = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("unknown status") && msg.contains("developped"),
        "message should name the bad status: {msg}"
    );
}

#[tokio::test]
async fn import_archived_anchor_falls_back_to_max_shot_date() {
    // Anchor precedence: with no date_finished or date_loaded, a completed-status
    // import borrows the latest SHOT date as its lower-bound anchor — the tail
    // milestones and the backfilled date_finished all land on it, and the roll
    // still derives fully Done.
    let app = open_app().await;
    let payload = json!({
        "roll_id": "IMPORT-ANCHOR-SHOT",
        "status": "archived",
        "shots": [
            { "frame_number": "1", "date": "2026-03-01" },
            { "frame_number": "2", "date": "2026-03-04" }
        ]
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/import/roll", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let roll_pk: i32 = json_body(res).await;

    let res = app
        .oneshot(get(&format!("/api/rolls/{roll_pk}/detail")))
        .await
        .unwrap();
    let detail: Value = json_body(res).await;
    assert_eq!(detail["roll"]["group_key"], 5);
    assert_eq!(detail["roll"]["badge"], "Done");
    assert_eq!(detail["roll"]["done"], true);
    assert_eq!(detail["roll"]["date_archived"], "2026-03-04");
    assert_eq!(detail["roll"]["date_scanned"], "2026-03-04");
    assert_eq!(detail["roll"]["date_finished"], "2026-03-04");
}
