use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "shot_lenses")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub shot_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub lens_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::shot::Entity",
        from = "Column::ShotId",
        to = "super::shot::Column::Id"
    )]
    Shot,
    #[sea_orm(
        belongs_to = "super::lens::Entity",
        from = "Column::LensId",
        to = "super::lens::Column::Id"
    )]
    Lens,
}

impl Related<super::shot::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Shot.def()
    }
}

impl Related<super::lens::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lens.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
