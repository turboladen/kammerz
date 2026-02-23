use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum FilmFormat {
    #[sea_orm(string_value = "135")]
    #[serde(rename = "135")]
    F135,
    #[sea_orm(string_value = "120")]
    #[serde(rename = "120")]
    F120,
    #[sea_orm(string_value = "4x5")]
    #[serde(rename = "4x5")]
    Sheet4x5,
    #[sea_orm(string_value = "5x7")]
    #[serde(rename = "5x7")]
    Sheet5x7,
    #[sea_orm(string_value = "8x10")]
    #[serde(rename = "8x10")]
    Sheet8x10,
    #[sea_orm(string_value = "instant")]
    #[serde(rename = "instant")]
    Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum FilmStockType {
    #[sea_orm(string_value = "color-negative")]
    #[serde(rename = "color-negative")]
    ColorNegative,
    #[sea_orm(string_value = "bw-negative")]
    #[serde(rename = "bw-negative")]
    BwNegative,
    #[sea_orm(string_value = "color-slide")]
    #[serde(rename = "color-slide")]
    ColorSlide,
    #[sea_orm(string_value = "bw-slide")]
    #[serde(rename = "bw-slide")]
    BwSlide,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "film_stocks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub brand: String,
    pub name: String,
    pub format: FilmFormat,
    pub exposure_count: Option<i32>,
    pub stock_type: FilmStockType,
    pub iso: Option<i32>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::roll::Entity")]
    Rolls,
}

impl Related<super::roll::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Rolls.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
