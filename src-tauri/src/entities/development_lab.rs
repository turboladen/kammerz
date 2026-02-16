use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "development_lab")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub roll_id: i32,
    pub lab_id: Option<i32>,
    pub date_dropped_off: Option<String>,
    pub date_received: Option<String>,
    pub cost: Option<f64>,
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
    #[sea_orm(
        belongs_to = "super::lab::Entity",
        from = "Column::LabId",
        to = "super::lab::Column::Id"
    )]
    Lab,
}

impl Related<super::roll::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Roll.def()
    }
}

impl Related<super::lab::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lab.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
