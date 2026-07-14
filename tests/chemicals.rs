//! Guards the chemistry reference (kammerz-9fx): the m..028 seed, the
//! `GET /api/development/chemicals` grouping, the self-dev auto-upsert on both
//! the create and update paths, and the m..029 normalization data.
//!
//! The normalize migration itself can't be observed through migration-at-init (a
//! fresh DB has no `development_selves` rows), so its data is checked two ways:
//! a static consistency test over the exposed `NORMALIZATIONS`, and a runtime
//! test that applies the SAME `apply_normalization` step the migration uses to a
//! crafted row and asserts idempotency.

mod common;

use axum::http::StatusCode;
use common::{get, json_body, open_app_with_db, post_json, put_json};
use migration::{NORMALIZATIONS, apply_normalization};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use serde_json::{Value, json};
use tower::ServiceExt;

use entity::chemical::{self, ChemicalType};

// ── helpers ──────────────────────────────────────────────────────────

/// Create a roll at a given status via the API and return its pk.
async fn create_roll(app: &axum::Router, roll_id: &str, status: &str) -> i32 {
    let res = app.clone().oneshot(get("/api/cameras")).await.unwrap();
    let cams: Vec<Value> = json_body(res).await;
    let camera_id = cams[0]["id"].as_i64().unwrap() as i32;

    let payload = json!({
        "roll_id": roll_id,
        "camera_id": camera_id,
        "status": status,
        "date_loaded": "2026-05-01",
    });
    let res = app
        .clone()
        .oneshot(post_json("/api/rolls", &payload))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    json_body(res).await
}

async fn chemical_count(
    db: &sea_orm::DatabaseConnection,
    name: &str,
    chemical_type: ChemicalType,
) -> u64 {
    chemical::Entity::find()
        .filter(chemical::Column::Name.eq(name))
        .filter(chemical::Column::Type.eq(chemical_type))
        .count(db)
        .await
        .unwrap()
}

// ── m..028 seed ──────────────────────────────────────────────────────

#[tokio::test]
async fn seed_chemicals_present() {
    let (_app, db) = open_app_with_db().await;

    // (name, type, expected default_dilution)
    let expected = [
        ("Kodak D-76", ChemicalType::Developer, Some("stock")),
        ("Kodak XTOL", ChemicalType::Developer, Some("1+1")),
        ("Adox Rodinal", ChemicalType::Developer, Some("1+25")),
        ("Kodak Fixer", ChemicalType::Fixer, Some("1+4")),
        (
            "Photographers Formulary Hyper Flow",
            ChemicalType::WettingAgent,
            None,
        ),
        (
            "Kodak Hypo Clearing Agent",
            ChemicalType::ClearingAgent,
            None,
        ),
    ];
    for (name, chemical_type, dilution) in expected {
        let row = chemical::Entity::find()
            .filter(chemical::Column::Name.eq(name))
            .filter(chemical::Column::Type.eq(chemical_type))
            .one(&db)
            .await
            .unwrap()
            .unwrap_or_else(|| panic!("seed chemical missing: {name}"));
        assert_eq!(
            row.default_dilution.as_deref(),
            dilution,
            "default_dilution for {name}"
        );
    }
}

// ── GET /api/development/chemicals ───────────────────────────────────

#[tokio::test]
async fn get_chemicals_returns_grouped_sorted() {
    let (app, _db) = open_app_with_db().await;

    let res = app
        .clone()
        .oneshot(get("/api/development/chemicals"))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body: Value = json_body(res).await;

    let dev_names: Vec<&str> = body["developer"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert!(dev_names.contains(&"Kodak D-76"));
    assert!(dev_names.contains(&"Kodak XTOL"));
    // Each group is name-sorted.
    let mut sorted = dev_names.clone();
    sorted.sort();
    assert_eq!(dev_names, sorted, "developer group must be name-sorted");

    // Buckets are keyed correctly, with the `type` field preserved.
    assert_eq!(body["fixer"][0]["name"], "Kodak Fixer");
    assert_eq!(body["fixer"][0]["type"], "fixer");
    assert!(
        body["clearing_agent"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["name"] == "Kodak Hypo Clearing Agent")
    );
}

// ── auto-upsert: create path ─────────────────────────────────────────

#[tokio::test]
async fn create_self_dev_auto_upserts_novel_value() {
    let (app, db) = open_app_with_db().await;
    let roll_pk = create_roll(&app, "CHEM-CREATE", "shot").await;

    assert_eq!(
        chemical_count(&db, "Test Dev XYZ", ChemicalType::Developer).await,
        0
    );

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/self",
            &json!({ "roll_id": roll_pk, "developer": "Test Dev XYZ" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    assert_eq!(
        chemical_count(&db, "Test Dev XYZ", ChemicalType::Developer).await,
        1,
        "novel developer must be learned into the reference"
    );
}

#[tokio::test]
async fn create_self_dev_does_not_duplicate_seeded_value() {
    let (app, db) = open_app_with_db().await;
    let roll_pk = create_roll(&app, "CHEM-DUP", "shot").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/self",
            &json!({ "roll_id": roll_pk, "developer": "Kodak D-76" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);

    assert_eq!(
        chemical_count(&db, "Kodak D-76", ChemicalType::Developer).await,
        1,
        "re-using a seeded value must not create a duplicate row"
    );
}

// ── auto-upsert: update path ─────────────────────────────────────────

#[tokio::test]
async fn update_self_dev_auto_upserts_novel_value() {
    let (app, db) = open_app_with_db().await;
    let roll_pk = create_roll(&app, "CHEM-UPDATE", "shot").await;

    let res = app
        .clone()
        .oneshot(post_json(
            "/api/development/self",
            &json!({ "roll_id": roll_pk, "developer": "Kodak D-76" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let self_id: i32 = json_body(res).await;

    assert_eq!(
        chemical_count(&db, "Test Fix ABC", ChemicalType::Fixer).await,
        0
    );

    let res = app
        .clone()
        .oneshot(put_json(
            &format!("/api/development/self/{self_id}"),
            &json!({ "fixer": "Test Fix ABC" }),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NO_CONTENT);

    assert_eq!(
        chemical_count(&db, "Test Fix ABC", ChemicalType::Fixer).await,
        1,
        "update path must also learn a novel value"
    );
}

// ── m..029 normalization data ────────────────────────────────────────

/// Static consistency: the normalization set is order-independent (no `to` is
/// also a `from` in the same field) and every canonical `to` is a seeded
/// chemical of the matching type (so a normalized record matches a suggestion).
#[tokio::test]
async fn normalizations_are_consistent_with_seed() {
    let (_app, db) = open_app_with_db().await;

    for (field, _from, to) in NORMALIZATIONS {
        // no chaining within a field
        let is_also_from = NORMALIZATIONS
            .iter()
            .any(|(f2, from2, _)| f2 == field && from2 == to);
        assert!(
            !is_also_from,
            "normalization target {to:?} is also a `from` in field {field} — chaining risk"
        );

        // the `to` must be a seeded chemical of the field's type
        let chemical_type = field_to_type(field);
        assert!(
            chemical_count(&db, to, chemical_type.clone()).await >= 1,
            "normalization target {to:?} ({field}) is not a seeded chemical"
        );
    }
}

/// Runtime: applying the same `apply_normalization` step the migration uses maps
/// a drifted value to canonical, and a second application is a no-op.
#[tokio::test]
async fn normalization_apply_is_idempotent() {
    let (app, db) = open_app_with_db().await;
    let roll_pk = create_roll(&app, "CHEM-NORM", "shot").await;

    // Seed a drifted row directly (bypassing the API's auto-upsert).
    let dev = entity::development_self::ActiveModel {
        roll_id: Set(roll_pk),
        developer: Set(Some("XTOL".into())),
        created_at: Set("2026-05-01T00:00:00Z".into()),
        updated_at: Set("2026-05-01T00:00:00Z".into()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let apply_all = || async {
        for (field, from, to) in NORMALIZATIONS {
            apply_normalization(&db, field, from, to).await.unwrap();
        }
    };

    apply_all().await;
    let after = entity::development_self::Entity::find_by_id(dev.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.developer.as_deref(), Some("Kodak XTOL"));

    // Second pass: the `WHERE <field> = '<from>'` no longer matches.
    apply_all().await;
    let after2 = entity::development_self::Entity::find_by_id(dev.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after2.developer.as_deref(), Some("Kodak XTOL"));
}

fn field_to_type(field: &str) -> ChemicalType {
    match field {
        "developer" => ChemicalType::Developer,
        "fixer" => ChemicalType::Fixer,
        "stop_bath" => ChemicalType::StopBath,
        "wetting_agent" => ChemicalType::WettingAgent,
        "clearing_agent" => ChemicalType::ClearingAgent,
        other => panic!("unexpected normalization field: {other}"),
    }
}
