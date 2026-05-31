use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "development_selves")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub roll_id: i32,
    pub date_processed: Option<String>,
    pub developer: Option<String>,
    pub developer_dilution: Option<String>,
    pub fixer: Option<String>,
    pub fixer_dilution: Option<String>,
    pub stop_bath: Option<String>,
    pub wetting_agent: Option<String>,
    pub clearing_agent: Option<String>,
    pub temperature: Option<String>,
    pub agitation_notes: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::roll::Entity",
        from = "Column::RollId",
        to = "super::roll::Column::Id"
    )]
    Roll,
    #[sea_orm(has_many = "super::dev_stage::Entity")]
    DevStages,
}

impl Related<super::roll::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Roll.def()
    }
}

impl Related<super::dev_stage::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DevStages.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
