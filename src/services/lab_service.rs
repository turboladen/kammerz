use sea_orm::*;

use ::entity::lab::{self, Entity as Lab};

pub struct LabService;

impl LabService {
    pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<lab::Model>, DbErr> {
        Lab::find().order_by_asc(lab::Column::Name).all(db).await
    }

    pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<lab::Model>, DbErr> {
        Lab::find_by_id(id).one(db).await
    }

    pub async fn create(
        db: &DatabaseConnection,
        model: lab::ActiveModel,
    ) -> Result<lab::Model, DbErr> {
        model.insert(db).await
    }

    pub async fn update(
        db: &DatabaseConnection,
        model: lab::ActiveModel,
    ) -> Result<lab::Model, DbErr> {
        model.update(db).await
    }

    pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
        let res = Lab::delete_by_id(id).exec(db).await?;
        if res.rows_affected == 0 {
            return Err(DbErr::RecordNotFound(format!("Lab {id} not found")));
        }
        Ok(())
    }
}
