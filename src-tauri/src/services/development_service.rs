use sea_orm::*;

use crate::entities::dev_stage::{self, Entity as DevStage};
use crate::entities::development_lab::{self, Entity as DevelopmentLab};
use crate::entities::development_self::{self, Entity as DevelopmentSelf};

pub struct DevelopmentService;

/// Input for creating/replacing dev stages (used by set_stages).
pub struct StageInput {
    pub stage_name: String,
    pub duration_seconds: Option<i32>,
    pub notes: Option<String>,
    pub sort_order: i32,
}

impl DevelopmentService {
    // --- Lab Development ---

    pub async fn get_lab_dev_for_roll(
        db: &DatabaseConnection,
        roll_id: i32,
    ) -> Result<Option<development_lab::Model>, DbErr> {
        DevelopmentLab::find()
            .filter(development_lab::Column::RollId.eq(roll_id))
            .one(db)
            .await
    }

    pub async fn get_lab_dev_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<development_lab::Model>, DbErr> {
        DevelopmentLab::find_by_id(id).one(db).await
    }

    pub async fn create_lab_dev(
        db: &DatabaseConnection,
        model: development_lab::ActiveModel,
    ) -> Result<development_lab::Model, DbErr> {
        model.insert(db).await
    }

    pub async fn update_lab_dev(
        db: &DatabaseConnection,
        model: development_lab::ActiveModel,
    ) -> Result<development_lab::Model, DbErr> {
        model.update(db).await
    }

    pub async fn delete_lab_dev(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
        DevelopmentLab::delete_by_id(id).exec(db).await?;
        Ok(())
    }

    // --- Self Development ---

    pub async fn get_self_dev_for_roll(
        db: &DatabaseConnection,
        roll_id: i32,
    ) -> Result<Option<development_self::Model>, DbErr> {
        DevelopmentSelf::find()
            .filter(development_self::Column::RollId.eq(roll_id))
            .one(db)
            .await
    }

    pub async fn get_self_dev_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<development_self::Model>, DbErr> {
        DevelopmentSelf::find_by_id(id).one(db).await
    }

    pub async fn create_self_dev(
        db: &impl ConnectionTrait,
        model: development_self::ActiveModel,
    ) -> Result<development_self::Model, DbErr> {
        model.insert(db).await
    }

    pub async fn update_self_dev(
        db: &impl ConnectionTrait,
        model: development_self::ActiveModel,
    ) -> Result<development_self::Model, DbErr> {
        model.update(db).await
    }

    pub async fn delete_self_dev(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
        // Dev stages are cascade-deleted by FK constraint
        DevelopmentSelf::delete_by_id(id).exec(db).await?;
        Ok(())
    }

    // --- Dev Stages ---

    pub async fn list_stages(
        db: &DatabaseConnection,
        development_self_id: i32,
    ) -> Result<Vec<dev_stage::Model>, DbErr> {
        DevStage::find()
            .filter(dev_stage::Column::DevelopmentSelfId.eq(development_self_id))
            .order_by_asc(dev_stage::Column::SortOrder)
            .all(db)
            .await
    }

    /// Replace all stages for a self-development record (delete-all-then-insert).
    pub async fn set_stages(
        db: &impl ConnectionTrait,
        development_self_id: i32,
        stages: Vec<StageInput>,
    ) -> Result<(), DbErr> {
        // Delete existing stages
        DevStage::delete_many()
            .filter(dev_stage::Column::DevelopmentSelfId.eq(development_self_id))
            .exec(db)
            .await?;

        // Bulk insert new stages
        if !stages.is_empty() {
            let models: Vec<dev_stage::ActiveModel> = stages
                .into_iter()
                .map(|stage| dev_stage::ActiveModel {
                    development_self_id: Set(development_self_id),
                    stage_name: Set(stage.stage_name),
                    duration_seconds: Set(stage.duration_seconds),
                    notes: Set(stage.notes),
                    sort_order: Set(stage.sort_order),
                    ..Default::default()
                })
                .collect();
            DevStage::insert_many(models).exec(db).await?;
        }
        Ok(())
    }
}
