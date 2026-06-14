use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::roll::RollStatus;

/// What kind of thing happened to a roll. Total enum — adding a variant forces
/// every match to be updated (mirrors RollStatus).
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum RollEventType {
    #[sea_orm(string_value = "roll_loaded")]
    #[serde(rename = "roll_loaded")]
    RollLoaded,
    #[sea_orm(string_value = "status_changed")]
    #[serde(rename = "status_changed")]
    StatusChanged,
    #[sea_orm(string_value = "shot_logged")]
    #[serde(rename = "shot_logged")]
    ShotLogged,
    #[sea_orm(string_value = "shot_edited")]
    #[serde(rename = "shot_edited")]
    ShotEdited,
    #[sea_orm(string_value = "shot_deleted")]
    #[serde(rename = "shot_deleted")]
    ShotDeleted,
    #[sea_orm(string_value = "lab_dev_added")]
    #[serde(rename = "lab_dev_added")]
    LabDevAdded,
    #[sea_orm(string_value = "lab_dev_edited")]
    #[serde(rename = "lab_dev_edited")]
    LabDevEdited,
    #[sea_orm(string_value = "lab_dev_removed")]
    #[serde(rename = "lab_dev_removed")]
    LabDevRemoved,
    #[sea_orm(string_value = "self_dev_added")]
    #[serde(rename = "self_dev_added")]
    SelfDevAdded,
    #[sea_orm(string_value = "self_dev_edited")]
    #[serde(rename = "self_dev_edited")]
    SelfDevEdited,
    #[sea_orm(string_value = "self_dev_removed")]
    #[serde(rename = "self_dev_removed")]
    SelfDevRemoved,
}

/// What record `ref_id` points to, so the frontend journal can deep-link an
/// event to its editor.
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum RefKind {
    #[sea_orm(string_value = "lab_dev")]
    #[serde(rename = "lab_dev")]
    LabDev,
    #[sea_orm(string_value = "self_dev")]
    #[serde(rename = "self_dev")]
    SelfDev,
    #[sea_orm(string_value = "shot")]
    #[serde(rename = "shot")]
    Shot,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "roll_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub roll_id: i32,
    pub event_type: RollEventType,
    pub from_status: Option<RollStatus>,
    pub to_status: Option<RollStatus>,
    pub ref_kind: Option<RefKind>,
    pub ref_id: Option<i32>,
    pub summary: String,
    pub occurred_at: String,
    pub created_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::roll::Entity",
        from = "Column::RollId",
        to = "super::roll::Column::Id"
    )]
    Roll,
}

impl Related<super::roll::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Roll.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
