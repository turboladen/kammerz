use std::collections::HashMap;

use sea_orm::*;
use serde::Serialize;

use crate::activity::{ActivitySignals, derive, phase_label};

pub struct StatsService;

/// Per-roll activity-derivation signals for the `rolls_by_phase` distribution.
#[derive(Debug, FromQueryResult)]
struct StatusSignalRow {
    date_finished: Option<String>,
    date_scanned: Option<String>,
    date_post_processed: Option<String>,
    date_archived: Option<String>,
    archive_na: bool,
    shot_count: i64,
    lab_dev_id: Option<i32>,
    lab_completion: Option<String>,
    self_dev_id: Option<i32>,
    self_completion: Option<String>,
}

#[derive(Debug, Serialize, FromQueryResult)]
pub struct CountRow {
    pub count: i64,
}

#[derive(Debug, Serialize, FromQueryResult)]
pub struct SumRow {
    pub total: f64,
}

#[derive(Debug, Serialize, FromQueryResult)]
pub struct MonthCount {
    pub month: String,
    pub count: i64,
}

#[derive(Debug, Serialize, FromQueryResult)]
pub struct RankedItem {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct CatalogStats {
    pub total_rolls: i64,
    pub total_shots: i64,
    pub total_cameras: i64,
    pub total_lenses: i64,
    pub total_lab_dev_cost: f64,
    pub total_maintenance_cost: f64,
    pub total_cost: f64,
    pub rolls_per_month: Vec<MonthCount>,
    pub top_film_stocks: Vec<RankedItem>,
    pub top_cameras: Vec<RankedItem>,
    pub top_lenses: Vec<RankedItem>,
    pub rolls_by_format: Vec<RankedItem>,
    pub rolls_by_phase: Vec<RankedItem>,
    pub rolls_by_mount: Vec<RankedItem>,
}

impl StatsService {
    pub async fn get_stats(db: &DatabaseConnection) -> Result<CatalogStats, DbErr> {
        let backend = db.get_database_backend();

        // --- Totals ---
        let total_rolls = CountRow::find_by_statement(Statement::from_string(
            backend,
            "SELECT COUNT(*) AS count FROM rolls".to_owned(),
        ))
        .one(db)
        .await?
        .map(|r| r.count)
        .unwrap_or(0);

        let total_shots = CountRow::find_by_statement(Statement::from_string(
            backend,
            "SELECT COUNT(*) AS count FROM shots".to_owned(),
        ))
        .one(db)
        .await?
        .map(|r| r.count)
        .unwrap_or(0);

        let total_cameras = CountRow::find_by_statement(Statement::from_string(
            backend,
            "SELECT COUNT(*) AS count FROM cameras".to_owned(),
        ))
        .one(db)
        .await?
        .map(|r| r.count)
        .unwrap_or(0);

        let total_lenses = CountRow::find_by_statement(Statement::from_string(
            backend,
            "SELECT COUNT(*) AS count FROM lenses".to_owned(),
        ))
        .one(db)
        .await?
        .map(|r| r.count)
        .unwrap_or(0);

        // --- Costs ---
        let total_lab_dev_cost = SumRow::find_by_statement(Statement::from_string(
            backend,
            "SELECT COALESCE(SUM(cost), 0.0) AS total FROM development_labs".to_owned(),
        ))
        .one(db)
        .await?
        .map(|r| r.total)
        .unwrap_or(0.0);

        let total_maintenance_cost = SumRow::find_by_statement(Statement::from_string(
            backend,
            "SELECT COALESCE(SUM(cost), 0.0) AS total FROM camera_maintenances".to_owned(),
        ))
        .one(db)
        .await?
        .map(|r| r.total)
        .unwrap_or(0.0);

        let total_cost = total_lab_dev_cost + total_maintenance_cost;

        // --- Rolls per month (last 12 months) ---
        // Dates are always full YYYY-MM-DD now (validate.rs rejects partials —
        // ADR-0011), but this stays defensive against a legacy partial `date_loaded`
        // in the DB: STRFTIME('%Y-%m', …) returns NULL for 'YYYY-MM' (failing the
        // non-Option `month` and 500ing the endpoint) and misparses bare 'YYYY' as a
        // Julian day number, so bucket by
        // SUBSTR instead: anything with at least a year+month prefix lands in
        // its month, year-only dates are skipped (no month to bucket by), and
        // the 12-month window compares at month precision.
        let rolls_per_month = MonthCount::find_by_statement(Statement::from_string(
            backend,
            "SELECT SUBSTR(date_loaded, 1, 7) AS month, COUNT(*) AS count \
             FROM rolls \
             WHERE date_loaded IS NOT NULL \
               AND LENGTH(date_loaded) >= 7 \
               AND SUBSTR(date_loaded, 1, 7) >= STRFTIME('%Y-%m', 'now', '-12 months') \
             GROUP BY month \
             ORDER BY month"
                .to_owned(),
        ))
        .all(db)
        .await?;

        // --- Top film stocks ---
        let top_film_stocks = RankedItem::find_by_statement(Statement::from_string(
            backend,
            "SELECT (fs.brand || ' ' || fs.name) AS label, COUNT(*) AS count \
             FROM rolls r \
             JOIN film_stocks fs ON r.film_stock_id = fs.id \
             GROUP BY r.film_stock_id \
             ORDER BY count DESC \
             LIMIT 10"
                .to_owned(),
        ))
        .all(db)
        .await?;

        // --- Top cameras ---
        let top_cameras = RankedItem::find_by_statement(Statement::from_string(
            backend,
            "SELECT (c.brand || ' ' || c.model) AS label, COUNT(*) AS count \
             FROM rolls r \
             JOIN cameras c ON r.camera_id = c.id \
             GROUP BY r.camera_id \
             ORDER BY count DESC \
             LIMIT 10"
                .to_owned(),
        ))
        .all(db)
        .await?;

        // --- Top lenses (per-shot assignments + roll-default for unassigned shots) ---
        let top_lenses = RankedItem::find_by_statement(Statement::from_string(
            backend,
            "SELECT label, SUM(count) AS count FROM ( \
                 SELECT (l.brand || ' ' || COALESCE(l.model, l.focal_length, '')) AS label, \
                        COUNT(*) AS count \
                 FROM shot_lenses sl \
                 JOIN lenses l ON sl.lens_id = l.id \
                 GROUP BY sl.lens_id \
               UNION ALL \
                 SELECT (l.brand || ' ' || COALESCE(l.model, l.focal_length, '')) AS label, \
                        COUNT(*) AS count \
                 FROM shots s \
                 JOIN rolls r ON s.roll_id = r.id \
                 JOIN lenses l ON r.lens_id = l.id \
                 WHERE NOT EXISTS ( \
                     SELECT 1 FROM shot_lenses sl WHERE sl.shot_id = s.id \
                 ) \
                 GROUP BY r.lens_id \
             ) \
             GROUP BY label \
             ORDER BY count DESC \
             LIMIT 10"
                .to_owned(),
        ))
        .all(db)
        .await?;

        // --- Rolls by format ---
        let rolls_by_format = RankedItem::find_by_statement(Statement::from_string(
            backend,
            "SELECT fs.format AS label, COUNT(*) AS count \
             FROM rolls r \
             JOIN film_stocks fs ON r.film_stock_id = fs.id \
             GROUP BY fs.format \
             ORDER BY count DESC"
                .to_owned(),
        ))
        .all(db)
        .await?;

        // --- Rolls by phase ---
        // There is no stored status (ADR-0013): derive each roll's activity view
        // from its signals and tally by lifecycle phase (the earliest-unresolved
        // `group_key`, 0..=5), ordered by count desc.
        let status_rows = StatusSignalRow::find_by_statement(Statement::from_string(
            backend,
            "SELECT r.date_finished, r.date_scanned, r.date_post_processed, \
                    r.date_archived, r.archive_na, \
                    COALESCE(sc.shot_count, 0) AS shot_count, \
                    dl.id AS lab_dev_id, dl.date_received AS lab_completion, \
                    ds.id AS self_dev_id, ds.date_processed AS self_completion \
             FROM rolls r \
             LEFT JOIN (SELECT roll_id, COUNT(*) AS shot_count FROM shots GROUP BY roll_id) sc \
                    ON sc.roll_id = r.id \
             LEFT JOIN development_labs dl ON dl.roll_id = r.id \
             LEFT JOIN development_selves ds ON ds.roll_id = r.id"
                .to_owned(),
        ))
        .all(db)
        .await?;

        let mut phase_counts: HashMap<i32, i64> = HashMap::new();
        for row in status_rows {
            let is_lab_dev = row.lab_dev_id.is_some();
            let sig = ActivitySignals {
                shot_count: row.shot_count,
                date_loaded: None,
                date_finished: row.date_finished,
                has_dev: is_lab_dev || row.self_dev_id.is_some(),
                is_lab_dev,
                dev_started: None,
                dev_completion: if is_lab_dev {
                    row.lab_completion
                } else {
                    row.self_completion
                },
                scan_started: None,
                date_scanned: row.date_scanned,
                post_processing_started: None,
                date_post_processed: row.date_post_processed,
                date_archived: row.date_archived,
                archive_na: row.archive_na,
            };
            *phase_counts.entry(derive(&sig).group_key).or_insert(0) += 1;
        }
        let mut rolls_by_phase: Vec<RankedItem> = phase_counts
            .into_iter()
            .map(|(group_key, count)| RankedItem {
                label: phase_label(group_key).to_string(),
                count,
            })
            .collect();
        // Count desc, then label asc for a deterministic order.
        rolls_by_phase.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.label.cmp(&b.label)));

        // --- Rolls by lens mount ---
        let rolls_by_mount = RankedItem::find_by_statement(Statement::from_string(
            backend,
            "SELECT lm.name AS label, COUNT(*) AS count \
             FROM rolls r \
             JOIN cameras c ON r.camera_id = c.id \
             JOIN lens_mounts lm ON c.lens_mount_id = lm.id \
             GROUP BY c.lens_mount_id \
             ORDER BY count DESC"
                .to_owned(),
        ))
        .all(db)
        .await?;

        Ok(CatalogStats {
            total_rolls,
            total_shots,
            total_cameras,
            total_lenses,
            total_lab_dev_cost,
            total_maintenance_cost,
            total_cost,
            rolls_per_month,
            top_film_stocks,
            top_cameras,
            top_lenses,
            rolls_by_format,
            rolls_by_phase,
            rolls_by_mount,
        })
    }
}
