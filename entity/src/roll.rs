use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum PushPull {
    #[sea_orm(string_value = "-2")]
    #[serde(rename = "-2")]
    MinusTwo,
    #[sea_orm(string_value = "-1")]
    #[serde(rename = "-1")]
    MinusOne,
    #[sea_orm(string_value = "+1")]
    #[serde(rename = "+1")]
    PlusOne,
    #[sea_orm(string_value = "+2")]
    #[serde(rename = "+2")]
    PlusTwo,
    #[sea_orm(string_value = "+3")]
    #[serde(rename = "+3")]
    PlusThree,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "rolls")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub roll_id: String,
    pub camera_id: Option<i32>,
    pub film_stock_id: Option<i32>,
    pub lens_id: Option<i32>,
    pub frame_count: Option<i32>,
    pub date_loaded: Option<String>,
    pub date_finished: Option<String>,
    // Activity-lifecycle columns (ADR-0013): the state of each activity derives
    // from date presence — see `kammerz::activity`. `status` was dropped (m..030).
    pub scan_started: Option<String>,
    pub date_scanned: Option<String>,
    pub post_processing_started: Option<String>,
    pub date_post_processed: Option<String>,
    pub date_archived: Option<String>,
    pub archive_location: Option<String>,
    pub archive_na: bool,
    pub archive_na_reason: Option<String>,
    pub push_pull: Option<PushPull>,
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
    #[sea_orm(
        belongs_to = "super::film_stock::Entity",
        from = "Column::FilmStockId",
        to = "super::film_stock::Column::Id"
    )]
    FilmStock,
    #[sea_orm(
        belongs_to = "super::lens::Entity",
        from = "Column::LensId",
        to = "super::lens::Column::Id"
    )]
    Lens,
    #[sea_orm(has_many = "super::shot::Entity")]
    Shots,
    #[sea_orm(has_many = "super::development_lab::Entity")]
    DevelopmentLabs,
    #[sea_orm(has_many = "super::development_self::Entity")]
    DevelopmentSelfs,
    #[sea_orm(has_many = "super::roll_event::Entity")]
    RollEvents,
}

impl Related<super::camera::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Camera.def()
    }
}

impl Related<super::film_stock::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FilmStock.def()
    }
}

impl Related<super::lens::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lens.def()
    }
}

impl Related<super::shot::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Shots.def()
    }
}

impl Related<super::development_lab::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DevelopmentLabs.def()
    }
}

impl Related<super::development_self::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DevelopmentSelfs.def()
    }
}

impl Related<super::roll_event::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RollEvents.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
