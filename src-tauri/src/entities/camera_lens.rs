use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "camera_lenses")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub camera_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub lens_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::camera::Entity",
        from = "Column::CameraId",
        to = "super::camera::Column::Id"
    )]
    Camera,
    #[sea_orm(
        belongs_to = "super::lens::Entity",
        from = "Column::LensId",
        to = "super::lens::Column::Id"
    )]
    Lens,
}

impl Related<super::camera::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Camera.def()
    }
}

impl Related<super::lens::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lens.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
