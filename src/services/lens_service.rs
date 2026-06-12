use sea_orm::*;

use ::entity::camera_lens::{self, Entity as CameraLens};
use ::entity::lens::{self, Entity as Lens};

pub struct LensService;

impl LensService {
    pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<lens::Model>, DbErr> {
        Lens::find()
            .order_by_asc(lens::Column::Brand)
            .order_by_asc(lens::Column::Model)
            .all(db)
            .await
    }

    pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<lens::Model>, DbErr> {
        Lens::find_by_id(id).one(db).await
    }

    pub async fn create(
        db: &impl ConnectionTrait,
        model: lens::ActiveModel,
    ) -> Result<lens::Model, DbErr> {
        model.insert(db).await
    }

    pub async fn update(
        db: &DatabaseConnection,
        model: lens::ActiveModel,
    ) -> Result<lens::Model, DbErr> {
        model.update(db).await
    }

    pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
        let res = Lens::delete_by_id(id).exec(db).await?;
        if res.rows_affected == 0 {
            return Err(DbErr::RecordNotFound(format!("Lens {id} not found")));
        }
        Ok(())
    }

    // --- Camera associations (reverse lookup) ---

    pub async fn get_cameras_for_lens(
        db: &DatabaseConnection,
        lens_id: i32,
    ) -> Result<Vec<i32>, DbErr> {
        let rows = CameraLens::find()
            .filter(camera_lens::Column::LensId.eq(lens_id))
            .all(db)
            .await?;
        Ok(rows.into_iter().map(|r| r.camera_id).collect())
    }

    // --- Distinct value helpers ---

    pub async fn distinct_brands(db: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
        #[derive(Debug, FromQueryResult)]
        struct BrandRow {
            brand: String,
        }
        let rows = BrandRow::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            "SELECT DISTINCT brand FROM lenses ORDER BY brand".to_string(),
        ))
        .all(db)
        .await?;
        Ok(rows.into_iter().map(|r| r.brand).collect())
    }

    pub async fn distinct_lens_systems(db: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
        #[derive(Debug, FromQueryResult)]
        struct SystemRow {
            lens_system: String,
        }
        let rows = SystemRow::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            "SELECT DISTINCT lens_system FROM lenses WHERE lens_system IS NOT NULL ORDER BY lens_system"
                .to_string(),
        ))
        .all(db)
        .await?;
        Ok(rows.into_iter().map(|r| r.lens_system).collect())
    }
}
