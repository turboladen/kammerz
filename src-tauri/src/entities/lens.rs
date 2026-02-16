use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "lenses")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub brand: String,
    pub lens_system: Option<String>,
    pub name_on_lens: Option<String>,
    pub focal_length: Option<String>,
    pub max_aperture: Option<String>,
    pub min_aperture: Option<String>,
    pub filter_thread_front_mm: Option<i32>,
    pub filter_thread_rear_mm: Option<i32>,
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
    #[sea_orm(has_many = "super::camera_lens::Entity")]
    CameraLenses,
    #[sea_orm(has_many = "super::shot_lens::Entity")]
    ShotLenses,
}

impl Related<super::camera_lens::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CameraLenses.def()
    }
}

impl Related<super::shot_lens::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ShotLenses.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
