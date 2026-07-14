use sea_orm::*;
use serde::Serialize;

use ::entity::chemical::{self, ChemicalType, Entity as Chemical};
use ::entity::development_self;

use crate::patch::now_string;

/// Canonical chemistry reference rows bucketed by type, each sorted by name.
/// A typed struct (not a `HashMap<ChemicalType, _>`) so the JSON shape is stable
/// and matches the frontend `GroupedChemicals` interface.
#[derive(Debug, Serialize)]
pub struct GroupedChemicals {
    pub developer: Vec<chemical::Model>,
    pub fixer: Vec<chemical::Model>,
    pub stop_bath: Vec<chemical::Model>,
    pub wetting_agent: Vec<chemical::Model>,
    pub clearing_agent: Vec<chemical::Model>,
}

pub struct ChemicalService;

impl ChemicalService {
    /// All chemicals grouped by type, each group name-sorted.
    pub async fn list_grouped(db: &DatabaseConnection) -> Result<GroupedChemicals, DbErr> {
        let all = Chemical::find()
            .order_by_asc(chemical::Column::Name)
            .all(db)
            .await?;

        let mut grouped = GroupedChemicals {
            developer: Vec::new(),
            fixer: Vec::new(),
            stop_bath: Vec::new(),
            wetting_agent: Vec::new(),
            clearing_agent: Vec::new(),
        };
        for c in all {
            match c.r#type {
                ChemicalType::Developer => grouped.developer.push(c),
                ChemicalType::Fixer => grouped.fixer.push(c),
                ChemicalType::StopBath => grouped.stop_bath.push(c),
                ChemicalType::WettingAgent => grouped.wetting_agent.push(c),
                ChemicalType::ClearingAgent => grouped.clearing_agent.push(c),
            }
        }
        Ok(grouped)
    }

    /// Self-learning upsert: record each non-empty chemistry value on a self-dev
    /// as a reference chemical so it becomes a future suggestion. Call inside the
    /// self-dev write transaction. `default_dilution` is left NULL — a per-roll
    /// dilution is not a canonical default.
    pub async fn upsert_from_self_dev(
        db: &impl ConnectionTrait,
        dev: &development_self::Model,
    ) -> Result<(), DbErr> {
        let fields = [
            (dev.developer.as_deref(), ChemicalType::Developer),
            (dev.fixer.as_deref(), ChemicalType::Fixer),
            (dev.stop_bath.as_deref(), ChemicalType::StopBath),
            (dev.wetting_agent.as_deref(), ChemicalType::WettingAgent),
            (dev.clearing_agent.as_deref(), ChemicalType::ClearingAgent),
        ];
        for (value, chemical_type) in fields {
            let name = value.map(str::trim).unwrap_or_default();
            if !name.is_empty() {
                Self::upsert(db, name, chemical_type).await?;
            }
        }
        Ok(())
    }

    /// `INSERT OR IGNORE` a single chemical on the UNIQUE(name, type) index — a
    /// repeat is a no-op. Parameterized so a name with an apostrophe is safe.
    async fn upsert(
        db: &impl ConnectionTrait,
        name: &str,
        chemical_type: ChemicalType,
    ) -> Result<(), DbErr> {
        let now = now_string();
        let stmt = Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT OR IGNORE INTO chemicals (name, type, default_dilution, created_at, updated_at)
             VALUES (?, ?, NULL, ?, ?)",
            [
                name.into(),
                chemical_type.to_value().into(),
                now.clone().into(),
                now.into(),
            ],
        );
        db.execute(stmt).await?;
        Ok(())
    }
}
