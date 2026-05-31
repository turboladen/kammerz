use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum RollStatus {
    #[sea_orm(string_value = "loaded")]
    #[serde(rename = "loaded")]
    Loaded,
    #[sea_orm(string_value = "shooting")]
    #[serde(rename = "shooting")]
    Shooting,
    #[sea_orm(string_value = "shot")]
    #[serde(rename = "shot")]
    Shot,
    #[sea_orm(string_value = "at-lab")]
    #[serde(rename = "at-lab")]
    AtLab,
    #[sea_orm(string_value = "lab-done")]
    #[serde(rename = "lab-done")]
    LabDone,
    #[sea_orm(string_value = "developing")]
    #[serde(rename = "developing")]
    Developing,
    #[sea_orm(string_value = "developed")]
    #[serde(rename = "developed")]
    Developed,
    #[sea_orm(string_value = "scanned")]
    #[serde(rename = "scanned")]
    Scanned,
    #[sea_orm(string_value = "archived")]
    #[serde(rename = "archived")]
    Archived,
}

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
    pub status: RollStatus,
    pub frame_count: Option<i32>,
    pub date_loaded: Option<String>,
    pub date_finished: Option<String>,
    pub date_fuzzy: Option<String>,
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

impl ActiveModelBehavior for ActiveModel {}
