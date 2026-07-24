mod common;

use axum::http::StatusCode;
use common::{delete, get, json_body, open_app, open_app_with_db, post_json, put_json};
use sea_orm::{ActiveModelTrait, Set};
use serde_json::{Value, json};
use tower::ServiceExt;

/// Seed a lab dev directly in the DB, bypassing the API's lab/self
/// mutual-exclusion guard (kammerz-ysw) — simulates legacy both-dev data.
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

/// Reconstruct the roll's legacy lifecycle label from its DERIVED activity view
/// plus its dev records (ADR-0013 — there is no stored status). The derived view
/// alone cannot tell at-lab from developing, or lab-done from developed: each pair
/// collapses to the same `group_key`/`badge` (development in-progress vs done). So
/// this reads the roll's `/detail` to see which dev RECORD exists and uses that
/// for the lab-vs-self distinction — the discrimination the old stored status
/// carried now lives on the record, which is exactly what these tests must assert.
async fn roll_status(app: &axum::Router, roll_pk: i32) -> String {
    let res = app
        .clone()
        .oneshot(get(&format!("/api/rolls/{roll_pk}/detail")))
        .await
        .unwrap();
    let detail: Value = json_body(res).await;
    let roll = &detail["roll"];
    let group_key = roll["group_key"].as_i64().unwrap();
    let shot_count = roll["shot_count"].as_i64().unwrap_or(0);
    let has_lab = !detail["lab_dev"].is_null();
    let has_self = !detail["self_dev"].is_null();
    match group_key {
        0 => {
            if shot_count > 0 {
                "shooting"
            } else {
                "loaded"
            }
        }
        // Development is the earliest unresolved activity: not-started (no record) =
        // shot; in-progress = at-lab/developing per the record kind.
        1 => {
            if has_lab {
                "at-lab"
            } else if has_self {
                "developing"
            } else {
                "shot"
            }
        }
        // Development done, waiting to scan: lab-done vs developed per record kind.
        // Lab precedence mirrors ActivitySignals::with_dev — a legacy both-dev
        // roll derives from the LAB record, so its legacy label is lab-done.
        2 => {
            if has_lab {
                "lab-done"
            } else {
                "developed"
            }
        }
        3 => "scanned",
        4 => "post-processed",
        _ => "archived",
    }
    .to_string()
}

/// Create a roll whose recorded dates derive to `status` (ADR-0013 — there is no
/// stored status). Sets the lifecycle dates the derivation reads: `date_finished`
/// for shot and beyond, then `date_scanned` / `date_post_processed` /
/// `date_archived` for the tail statuses. Dev-backed statuses (at-lab, developing,
/// …) get the shot-level dates here; the test creates the matching dev record to
/// drive the derivation the rest of the way.
async fn create_roll_at_status(app: &axum::Router, roll_id: &str, status: &str) -> i32 {
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    let mut payload = json!({
        "roll_id": roll_id,
        "camera_id": camera_id,
        "date_loaded": "2026-05-01"
    });
    let order = [
        "loaded",
        "shooting",
        "shot",
        "at-lab",
        "lab-done",
        "developing",
        "developed",
        "scanned",
        "post-processed",
        "archived",
    ];
    let rank = order.iter().position(|s| *s == status).unwrap_or(0);
    let obj = payload.as_object_mut().unwrap();
    if rank >= 2 {
        obj.insert("date_finished".into(), json!("2026-05-02"));
    }
    if rank >= 7 {
        obj.insert("date_scanned".into(), json!("2026-05-13"));
    }
    if rank >= 8 {
        obj.insert("date_post_processed".into(), json!("2026-05-14"));
    }
    if rank >= 9 {
        obj.insert("date_archived".into(), json!("2026-05-20"));
    }
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
    // create + set_stages path.
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

    // Roll derives to developing from the newly-created self dev record.
    assert_eq!(roll_status(&app, roll_pk).await, "developing");

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

    assert_eq!(
        roll_status(&app, roll_pk).await,
        "at-lab",
        "lab dev advances roll to at-lab"
    );
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

    assert_eq!(
        roll_status(&app, roll_pk).await,
        "lab-done",
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

    assert_eq!(
        roll_status(&app, roll_pk).await,
        "developed",
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

    assert_eq!(
        roll_status(&app, roll_pk).await,
        "developing",
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

// kammerz-vlyu.3: updating a lab dev that doesn't exist returns 404 NOT_FOUND,
// not 422. The pre-txn `or_404` fetch classifies the ordinary missing-id case;
// the in-txn TOCTOU path (target deleted mid-update) is now routed through
// friendly_txn_err(Op::Write, _) too — that RecordNotFound→404 mapping is
// unit-tested in routes::mod (txn_write_record_not_found_maps_to_404), since a
// race can't be won from an integration test. This guards the update/delete 404
// symmetry against regression.
#[tokio::test]
async fn update_missing_lab_dev_returns_404() {
    let app = open_app().await;

    let res = app
        .oneshot(put_json(
            "/api/development/lab/999999",
            &json!({ "cost": 12.0 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let body: Value = json_body(res).await;
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    assert_eq!(body["error"]["message"], "Lab development 999999 not found");
}

// kammerz-vlyu.3 (batches kammerz-o0vj): symmetric self-dev case — update of a
// missing self dev is 404.
#[tokio::test]
async fn update_missing_self_dev_returns_404() {
    let app = open_app().await;

    let res = app
        .oneshot(put_json(
            "/api/development/self/999999",
            &json!({ "developer": "ghost" }),
        ))
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

// --- Dev-record creation drives the compat status directly (ADR-0013). These
//     scenarios previously exercised kammerz-e2u "cross-flow adoption" of an
//     orphaned stored status; with the status now derived from the dev record's
//     dates there is nothing to adopt — the derived value simply follows the
//     record. The date-labelled `create_roll_at_status` argument seeds only the
//     shot-level date, so the created dev record determines the result. ---

// A lab dev with a received date derives to lab-done regardless of any prior
// (now non-existent) stored status.
#[tokio::test]
async fn create_lab_dev_with_received_date_adopts_orphan_developing_to_lab_done() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "E2U-DEVING-LAB", "developing").await;

    create_lab_dev(&app, roll_pk, json!({ "date_received": "2026-05-10" })).await;
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "lab-done",
        "a lab dev with a received date adopts a developing orphan onto the lab path at lab-done"
    );
}

// kammerz-e2u: same orphan, no received date — adopts to at-lab.
#[tokio::test]
async fn create_lab_dev_without_received_date_adopts_orphan_developed_to_at_lab() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "E2U-DEVED-LAB", "developed").await;

    create_lab_dev(&app, roll_pk, json!({ "lab_name": "The Darkroom" })).await;
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "at-lab",
        "a lab dev with no received date adopts a developed orphan onto the lab path at at-lab"
    );
}

// kammerz-e2u mirror: roll orphaned at 'at-lab' (lab-path, no lab dev record);
// recording a SELF dev with a processed date adopts the self path → developed.
#[tokio::test]
async fn create_self_dev_with_processed_date_adopts_orphan_at_lab_to_developed() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "E2U-ATLAB-SELF", "at-lab").await;

    create_self_dev(&app, roll_pk, json!({ "date_processed": "2026-05-12" })).await;
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "developed",
        "a self dev with a processed date adopts an at-lab orphan onto the self path at developed"
    );
}

// kammerz-e2u mirror: orphan at 'lab-done', no processed date — adopts to developing.
#[tokio::test]
async fn create_self_dev_without_processed_date_adopts_orphan_lab_done_to_developing() {
    let app = open_app().await;
    let roll_pk = create_roll_at_status(&app, "E2U-LABDONE-SELF", "lab-done").await;

    create_self_dev(&app, roll_pk, json!({ "developer": "HC-110" })).await;
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "developing",
        "a self dev with no processed date adopts a lab-done orphan onto the self path at developing"
    );
}

// kammerz-e2u no-regression (kammerz-ysw invariant): adoption only fires for a
// no-record orphan. When a SIBLING dev record already exists (legacy both-dev
// data, seeded directly to bypass the create-side mutual-exclusion guard), the
// cross-flow status must be left alone — we must not yank a roll off a path its
// surviving sibling record still justifies. Seed a lab dev directly so the roll
// sits at a lab-path status, then create the self dev via the service-level
// adoption path... but the API rejects that create (ysw). So we assert the
// adoption helper respects the sibling by driving it through the same direct
// seed + a roll already on the sibling flow, then confirm a create is rejected
// (status untouched). This pins the "sibling present → no adoption" branch.
#[tokio::test]
async fn create_self_dev_rejected_does_not_adopt_when_lab_dev_seeded() {
    let (app, db) = open_app_with_db().await;
    let roll_pk = create_roll_at_status(&app, "E2U-SIBLING", "at-lab").await;
    // Legacy lab dev seeded directly; roll legitimately sits at at-lab.
    insert_lab_dev_directly(&db, roll_pk).await;
    assert_eq!(roll_status(&app, roll_pk).await, "at-lab");

    // A self dev create is rejected by the mutual-exclusion guard; the roll's
    // lab-path status (justified by the surviving lab dev) is untouched.
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/self",
            &json!({ "roll_id": roll_pk, "developer": "Rodinal" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        roll_status(&app, roll_pk).await,
        "at-lab",
        "a rejected self dev must not adopt a roll that already has a lab dev sibling"
    );
}

// --- Server-side input validation (kammerz-grd) ---

#[tokio::test]
async fn create_lab_dev_rejects_negative_cost() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "LABDEV-NEG-COST").await;
    let res = app
        .oneshot(post_json(
            "/api/development/lab",
            &json!({ "roll_id": roll_pk, "cost": -10.0 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = json_body(res).await;
    assert!(body["error"]["message"].as_str().unwrap().contains("cost"));
}

#[tokio::test]
async fn update_lab_dev_rejects_negative_cost() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "LABDEV-UPD-COST").await;
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

    let res = app
        .oneshot(put_json(
            &format!("/api/development/lab/{dev_id}"),
            &json!({ "cost": -1.0 }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn create_self_dev_rejects_whitespace_stage_name() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "SELFDEV-BLANK-STAGE").await;
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/self",
            &json!({
                "roll_id": roll_pk,
                "stages": [
                    { "stage_name": "   ", "duration_seconds": 60, "sort_order": 0 }
                ]
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
            .contains("stage_name")
    );

    // Validation failed before the transaction — no self dev persisted.
    let res = app
        .oneshot(get(&format!("/api/development/self/for-roll/{roll_pk}")))
        .await
        .unwrap();
    let dev: Value = json_body(res).await;
    assert!(dev.is_null(), "rejected self dev must not persist");
}

#[tokio::test]
async fn create_self_dev_rejects_negative_stage_duration() {
    let app = open_app().await;
    let roll_pk = create_shot_roll(&app, "SELFDEV-NEG-STAGE").await;
    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/self",
            &json!({
                "roll_id": roll_pk,
                "stages": [
                    { "stage_name": "Develop", "duration_seconds": -60, "sort_order": 0 }
                ]
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
            .contains("duration_seconds")
    );

    // Validation failed before the transaction — no self dev persisted.
    let res = app
        .oneshot(get(&format!("/api/development/self/for-roll/{roll_pk}")))
        .await
        .unwrap();
    let dev: Value = json_body(res).await;
    assert!(dev.is_null(), "rejected self dev must not persist");
}
