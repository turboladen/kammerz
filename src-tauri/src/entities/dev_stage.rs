use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "dev_stages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub development_self_id: i32,
    pub stage_name: String,
    pub duration_seconds: Option<i32>,
    pub notes: Option<String>,
    pub sort_order: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::development_self::Entity",
        from = "Column::DevelopmentSelfId",
        to = "super::development_self::Column::Id"
    )]
    DevelopmentSelf,
}

impl Related<super::development_self::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DevelopmentSelf.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
