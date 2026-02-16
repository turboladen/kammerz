use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "labs")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub location: Option<String>,
    pub website: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::development_lab::Entity")]
    DevelopmentLabs,
}

impl Related<super::development_lab::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DevelopmentLabs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
