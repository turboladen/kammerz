//! Cross-check that the canonical status-flow fixture stays in sync with the
//! backend flow constants and the `RollStatus` enum.
//!
//! The roll status ordering is hand-duplicated in four places: the
//! `LAB_FLOW`/`SELF_FLOW` constants (`src/services/roll_service.rs`), the
//! frontend `labFlow`/`selfFlow`/`undecidedFlow` + `allStatusOrder`
//! (`frontend/src/lib/utils/status.ts`), the Rust `RollStatus` enum
//! (`entity/src/roll.rs`), and the TS `RollStatus` union
//! (`frontend/src/lib/types/index.ts`). `frontend/src/lib/status-flows.json` is
//! the committed canonical fixture that ties them together: the frontend derives
//! its arrays from it, and these tests assert the real backend constants and the
//! enum ordering against it. Any drift fails `cargo test`.
//!
//! `cargo test` is the authoritative gate for the fixture's CONTENTS — a typo or
//! reordering on any backend side is caught here. The frontend's own
//! `assertStatusFixture` guard is runtime-only (the SPA build does not execute
//! it), so `svelte-check` / `bun run build` do not gate fixture content.
//!
//! Note: the fixture's `undecidedFlow` exists only on the frontend (the backend
//! has no undecided constant), so it is intentionally not asserted here.

use ::entity::roll::RollStatus;
use kammerz::services::roll_service::{LAB_FLOW, SELF_FLOW};
use sea_orm::Iterable;

/// Parsed shape of `frontend/src/lib/status-flows.json`.
#[derive(serde::Deserialize)]
struct StatusFlows {
    statuses: Vec<String>,
    #[serde(rename = "labFlow")]
    lab_flow: Vec<String>,
    #[serde(rename = "selfFlow")]
    self_flow: Vec<String>,
}

/// The fixture lives in the frontend tree; the `kammerz` crate is the workspace
/// root crate, so `CARGO_MANIFEST_DIR` is the workspace root. Reading at test
/// time (rather than `include_str!`) keeps the path resolution explicit and
/// independent of the working directory the test runner is launched from.
fn load_fixture() -> StatusFlows {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/frontend/src/lib/status-flows.json"
    );
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("reading status-flows.json at {path}: {e}"));
    serde_json::from_str(&raw).expect("status-flows.json is valid JSON matching StatusFlows")
}

/// The wire string for a `RollStatus` (e.g. `RollStatus::AtLab` -> `"at-lab"`).
/// `#[serde(rename = ...)]` on each variant matches its `#[sea_orm(string_value = ...)]`,
/// so serde serialization yields exactly the value stored in the DB and sent to
/// the frontend — the same strings the fixture lists.
fn status_str(status: &RollStatus) -> String {
    serde_json::to_value(status)
        .expect("RollStatus serializes")
        .as_str()
        .expect("RollStatus serializes to a string")
        .to_owned()
}

fn flow_strs(flow: &[RollStatus]) -> Vec<String> {
    flow.iter().map(status_str).collect()
}

/// The backend's canonical full ordering of statuses, mirrored from `RollStatus`
/// via `EnumIter`. The enum's declaration order IS the canonical order, so a
/// variant added/removed/reordered changes this automatically and forces the
/// fixture to follow.
fn enum_ordering() -> Vec<String> {
    RollStatus::iter().map(|s| status_str(&s)).collect()
}

#[test]
fn fixture_statuses_match_enum_ordering() {
    let fixture = load_fixture();
    assert_eq!(
        fixture.statuses,
        enum_ordering(),
        "status-flows.json \"statuses\" must equal the RollStatus enum's full ordering (entity/src/roll.rs). \
         A variant was added, removed, or reordered without updating the fixture."
    );
}

#[test]
fn fixture_lab_flow_matches_backend_constant() {
    let fixture = load_fixture();
    assert_eq!(
        fixture.lab_flow,
        flow_strs(LAB_FLOW),
        "status-flows.json \"labFlow\" must equal LAB_FLOW in src/services/roll_service.rs"
    );
}

#[test]
fn fixture_self_flow_matches_backend_constant() {
    let fixture = load_fixture();
    assert_eq!(
        fixture.self_flow,
        flow_strs(SELF_FLOW),
        "status-flows.json \"selfFlow\" must equal SELF_FLOW in src/services/roll_service.rs"
    );
}
