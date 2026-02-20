use sea_orm::*;

use crate::entities::lens_mount;

pub struct LensMountService;

impl LensMountService {
    pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<lens_mount::Model>, DbErr> {
        lens_mount::Entity::find()
            .order_by_asc(lens_mount::Column::Name)
            .all(db)
            .await
    }

    pub async fn create(
        db: &DatabaseConnection,
        model: lens_mount::ActiveModel,
    ) -> Result<lens_mount::Model, DbErr> {
        model.insert(db).await
    }
}
