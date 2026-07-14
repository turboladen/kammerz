use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// The chemistry category a `chemicals` row belongs to. String values match the
/// five `development_selves` chemistry column names so the frontend can key
/// suggestions by field.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum ChemicalType {
    #[sea_orm(string_value = "developer")]
    #[serde(rename = "developer")]
    Developer,
    #[sea_orm(string_value = "fixer")]
    #[serde(rename = "fixer")]
    Fixer,
    #[sea_orm(string_value = "stop_bath")]
    #[serde(rename = "stop_bath")]
    StopBath,
    #[sea_orm(string_value = "wetting_agent")]
    #[serde(rename = "wetting_agent")]
    WettingAgent,
    #[sea_orm(string_value = "clearing_agent")]
    #[serde(rename = "clearing_agent")]
    ClearingAgent,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "chemicals")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    // `type` is a Rust keyword; the raw identifier keeps the DB column and the
    // JSON/TS field both named `type`.
    #[sea_orm(column_name = "type")]
    #[serde(rename = "type")]
    pub r#type: ChemicalType,
    pub default_dilution: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
