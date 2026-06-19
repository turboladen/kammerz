use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "shots")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub roll_id: i32,
    pub frame_number: String,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub date: Option<String>,
    pub date_fuzzy: Option<String>,
    pub time: Option<String>,
    pub location: Option<String>,
    pub gps_lat: Option<f64>,
    pub gps_lon: Option<f64>,
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
    #[sea_orm(has_many = "super::shot_lens::Entity")]
    ShotLenses,
}

impl Related<super::roll::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Roll.def()
    }
}

impl Related<super::shot_lens::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ShotLenses.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
