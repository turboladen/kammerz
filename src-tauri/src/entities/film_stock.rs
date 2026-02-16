use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "film_stocks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub brand: String,
    pub name: String,
    pub format: String,
    pub exposure_count: Option<i32>,
    pub stock_type: String,
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
