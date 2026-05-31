use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum CameraFormat {
    #[sea_orm(string_value = "35mm")]
    #[serde(rename = "35mm")]
    ThirtyFiveMm,
    #[sea_orm(string_value = "medium format")]
    #[serde(rename = "medium format")]
    MediumFormat,
    #[sea_orm(string_value = "6x4.5")]
    #[serde(rename = "6x4.5")]
    Mf645,
    #[sea_orm(string_value = "6x6")]
    #[serde(rename = "6x6")]
    Mf66,
    #[sea_orm(string_value = "6x7")]
    #[serde(rename = "6x7")]
    Mf67,
    #[sea_orm(string_value = "6x8")]
    #[serde(rename = "6x8")]
    Mf68,
    #[sea_orm(string_value = "6x9")]
    #[serde(rename = "6x9")]
    Mf69,
    #[sea_orm(string_value = "large format")]
    #[serde(rename = "large format")]
    LargeFormat,
    #[sea_orm(string_value = "4x5")]
    #[serde(rename = "4x5")]
    Lf4x5,
    #[sea_orm(string_value = "5x7")]
    #[serde(rename = "5x7")]
    Lf5x7,
    #[sea_orm(string_value = "8x10")]
    #[serde(rename = "8x10")]
    Lf8x10,
    #[sea_orm(string_value = "instant")]
    #[serde(rename = "instant")]
    Instant,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum CameraType {
    #[sea_orm(string_value = "SLR")]
    #[serde(rename = "SLR")]
    Slr,
    #[sea_orm(string_value = "rangefinder")]
    #[serde(rename = "rangefinder")]
    Rangefinder,
    #[sea_orm(string_value = "TLR")]
    #[serde(rename = "TLR")]
    Tlr,
    #[sea_orm(string_value = "point-and-shoot")]
    #[serde(rename = "point-and-shoot")]
    PointAndShoot,
    #[sea_orm(string_value = "box")]
    #[serde(rename = "box")]
    Box,
    #[sea_orm(string_value = "view")]
    #[serde(rename = "view")]
    View,
    #[sea_orm(string_value = "instant")]
    #[serde(rename = "instant")]
    Instant,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "cameras")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub brand: String,
    pub model: String,
    pub prefix: Option<String>,
    pub format: CameraFormat,
    pub lens_mount_id: i32,
    pub default_lens_id: Option<i32>,
    pub camera_type: Option<CameraType>,
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
    #[sea_orm(
        belongs_to = "super::lens::Entity",
        from = "Column::DefaultLensId",
        to = "super::lens::Column::Id"
    )]
    DefaultLens,
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

// Note: Related<lens::Entity> via DefaultLens is intentionally omitted
// because camera already relates to lens via camera_lens junction table.
// Use the Relation::DefaultLens directly when needed.
