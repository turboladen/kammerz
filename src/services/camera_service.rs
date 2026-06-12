use sea_orm::*;

use ::entity::camera::{self, Entity as Camera};
use ::entity::camera_lens::{self, Entity as CameraLens};
use ::entity::camera_maintenance::{self, Entity as CameraMaintenance};

pub struct CameraService;

impl CameraService {
    pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<camera::Model>, DbErr> {
        Camera::find()
            .order_by_asc(camera::Column::Brand)
            .order_by_asc(camera::Column::Model)
            .all(db)
            .await
    }

    pub async fn get_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<camera::Model>, DbErr> {
        Camera::find_by_id(id).one(db).await
    }

    pub async fn create(
        db: &impl ConnectionTrait,
        model: camera::ActiveModel,
    ) -> Result<camera::Model, DbErr> {
        model.insert(db).await
    }

    pub async fn update(
        db: &impl ConnectionTrait,
        model: camera::ActiveModel,
    ) -> Result<camera::Model, DbErr> {
        model.update(db).await
    }

    pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
        let res = Camera::delete_by_id(id).exec(db).await?;
        if res.rows_affected == 0 {
            return Err(DbErr::RecordNotFound(format!("Camera {id} not found")));
        }
        Ok(())
    }

    // --- Maintenance ---

    pub async fn list_maintenance(
        db: &DatabaseConnection,
        camera_id: i32,
    ) -> Result<Vec<camera_maintenance::Model>, DbErr> {
        CameraMaintenance::find()
            .filter(camera_maintenance::Column::CameraId.eq(camera_id))
            .order_by_desc(camera_maintenance::Column::DateDone)
            .all(db)
            .await
    }

    pub async fn create_maintenance(
        db: &DatabaseConnection,
        model: camera_maintenance::ActiveModel,
    ) -> Result<camera_maintenance::Model, DbErr> {
        model.insert(db).await
    }

    pub async fn update_maintenance(
        db: &DatabaseConnection,
        model: camera_maintenance::ActiveModel,
    ) -> Result<camera_maintenance::Model, DbErr> {
        model.update(db).await
    }

    pub async fn delete_maintenance(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
        let res = CameraMaintenance::delete_by_id(id).exec(db).await?;
        if res.rows_affected == 0 {
            return Err(DbErr::RecordNotFound(format!(
                "Maintenance record {id} not found"
            )));
        }
        Ok(())
    }

    // --- Camera-Lens associations ---

    pub async fn get_lenses_for_camera(
        db: &DatabaseConnection,
        camera_id: i32,
    ) -> Result<Vec<i32>, DbErr> {
        let rows = CameraLens::find()
            .filter(camera_lens::Column::CameraId.eq(camera_id))
            .all(db)
            .await?;
        Ok(rows.into_iter().map(|r| r.lens_id).collect())
    }

    pub async fn link_lens(
        db: &impl ConnectionTrait,
        camera_id: i32,
        lens_id: i32,
    ) -> Result<(), DbErr> {
        // INSERT OR IGNORE equivalent
        let existing = CameraLens::find()
            .filter(camera_lens::Column::CameraId.eq(camera_id))
            .filter(camera_lens::Column::LensId.eq(lens_id))
            .one(db)
            .await?;

        if existing.is_none() {
            let model = camera_lens::ActiveModel {
                camera_id: Set(camera_id),
                lens_id: Set(lens_id),
            };
            model.insert(db).await?;
        }
        Ok(())
    }

    pub async fn unlink_lens(
        db: &DatabaseConnection,
        camera_id: i32,
        lens_id: i32,
    ) -> Result<(), DbErr> {
        CameraLens::delete_many()
            .filter(camera_lens::Column::CameraId.eq(camera_id))
            .filter(camera_lens::Column::LensId.eq(lens_id))
            .exec(db)
            .await?;
        Ok(())
    }

    // --- Distinct value helpers ---

    pub async fn distinct_brands(db: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
        #[derive(Debug, FromQueryResult)]
        struct BrandRow {
            brand: String,
        }
        let rows = BrandRow::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            "SELECT DISTINCT brand FROM cameras ORDER BY brand".to_string(),
        ))
        .all(db)
        .await?;
        Ok(rows.into_iter().map(|r| r.brand).collect())
    }

    pub async fn distinct_vendors(db: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
        #[derive(Debug, FromQueryResult)]
        struct VendorRow {
            purchased_from: String,
        }
        let rows = VendorRow::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            "SELECT DISTINCT purchased_from FROM cameras WHERE purchased_from IS NOT NULL \
             UNION \
             SELECT DISTINCT purchased_from FROM lenses WHERE purchased_from IS NOT NULL \
             ORDER BY 1"
                .to_string(),
        ))
        .all(db)
        .await?;
        Ok(rows.into_iter().map(|r| r.purchased_from).collect())
    }

    pub async fn distinct_maint_providers(db: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
        #[derive(Debug, FromQueryResult)]
        struct ProviderRow {
            done_by: String,
        }
        let rows = ProviderRow::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            "SELECT DISTINCT done_by FROM camera_maintenances WHERE done_by IS NOT NULL ORDER BY done_by"
                .to_string(),
        ))
        .all(db)
        .await?;
        Ok(rows.into_iter().map(|r| r.done_by).collect())
    }
}
