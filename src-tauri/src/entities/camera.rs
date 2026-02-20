use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "cameras")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub brand: String,
    pub model: String,
    pub prefix: Option<String>,
    pub format: String,
    pub lens_mount_id: i32,
    pub camera_type: Option<String>,
    pub serial_number: Option<String>,
    pub date_purchased: Option<String>,
    pub purchased_from: Option<String>,
    pub date_sold: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::camera_maintenance::Entity")]
    CameraMaintenances,
    #[sea_orm(has_many = "super::roll::Entity")]
    Rolls,
    #[sea_orm(has_many = "super::camera_lens::Entity")]
    CameraLenses,
    #[sea_orm(
        belongs_to = "super::lens_mount::Entity",
        from = "Column::LensMountId",
        to = "super::lens_mount::Column::Id"
    )]
    LensMount,
}

impl Related<super::camera_maintenance::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CameraMaintenances.def()
    }
}

impl Related<super::roll::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Rolls.def()
    }
}

impl Related<super::camera_lens::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CameraLenses.def()
    }
}

impl Related<super::lens_mount::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LensMount.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
