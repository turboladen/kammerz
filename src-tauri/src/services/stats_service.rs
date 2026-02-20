use sea_orm::*;
use serde::Serialize;

pub struct StatsService;

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
    pub rolls_by_status: Vec<RankedItem>,
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
            "SELECT COALESCE(SUM(cost), 0.0) AS total FROM development_lab".to_owned(),
        ))
        .one(db)
        .await?
        .map(|r| r.total)
        .unwrap_or(0.0);

        let total_maintenance_cost = SumRow::find_by_statement(Statement::from_string(
            backend,
            "SELECT COALESCE(SUM(cost), 0.0) AS total FROM camera_maintenance".to_owned(),
        ))
        .one(db)
        .await?
        .map(|r| r.total)
        .unwrap_or(0.0);

        let total_cost = total_lab_dev_cost + total_maintenance_cost;

        // --- Rolls per month (last 12 months) ---
        let rolls_per_month = MonthCount::find_by_statement(Statement::from_string(
            backend,
            "SELECT STRFTIME('%Y-%m', date_loaded) AS month, COUNT(*) AS count \
             FROM rolls \
             WHERE date_loaded IS NOT NULL \
               AND date_loaded >= DATE('now', '-12 months') \
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
                 SELECT (l.brand || ' ' || COALESCE(l.name_on_lens, l.focal_length, '')) AS label, \
                        COUNT(*) AS count \
                 FROM shot_lenses sl \
                 JOIN lenses l ON sl.lens_id = l.id \
                 GROUP BY sl.lens_id \
               UNION ALL \
                 SELECT (l.brand || ' ' || COALESCE(l.name_on_lens, l.focal_length, '')) AS label, \
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

        // --- Rolls by status ---
        let rolls_by_status = RankedItem::find_by_statement(Statement::from_string(
            backend,
            "SELECT status AS label, COUNT(*) AS count \
             FROM rolls \
             GROUP BY status \
             ORDER BY count DESC"
                .to_owned(),
        ))
        .all(db)
        .await?;

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
            rolls_by_status,
            rolls_by_mount,
        })
    }
}
