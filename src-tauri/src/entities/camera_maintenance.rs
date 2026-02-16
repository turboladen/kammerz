use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "camera_maintenance")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub camera_id: i32,
    pub maintenance_type: String,
    pub done_by: Option<String>,
    pub date_done: Option<String>,
    pub cost: Option<f64>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::camera::Entity",
        from = "Column::CameraId",
        to = "super::camera::Column::Id"
    )]
    Camera,
}

impl Related<super::camera::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Camera.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
