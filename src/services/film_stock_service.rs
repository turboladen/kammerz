use sea_orm::*;

use ::entity::film_stock::{self, Entity as FilmStock};

pub struct FilmStockService;

impl FilmStockService {
    pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<film_stock::Model>, DbErr> {
        FilmStock::find()
            .order_by_asc(film_stock::Column::Brand)
            .order_by_asc(film_stock::Column::Name)
            .order_by_asc(film_stock::Column::Format)
            .all(db)
            .await
    }

    pub async fn get_by_id(
        db: &DatabaseConnection,
        id: i32,
    ) -> Result<Option<film_stock::Model>, DbErr> {
        FilmStock::find_by_id(id).one(db).await
    }

    pub async fn create(
        db: &DatabaseConnection,
        model: film_stock::ActiveModel,
    ) -> Result<film_stock::Model, DbErr> {
        model.insert(db).await
    }

    pub async fn update(
        db: &DatabaseConnection,
        model: film_stock::ActiveModel,
    ) -> Result<film_stock::Model, DbErr> {
        model.update(db).await
    }

    pub async fn delete(db: &DatabaseConnection, id: i32) -> Result<(), DbErr> {
        let res = FilmStock::delete_by_id(id).exec(db).await?;
        if res.rows_affected == 0 {
            return Err(DbErr::RecordNotFound(format!("Film stock {id} not found")));
        }
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
            "SELECT DISTINCT brand FROM film_stocks ORDER BY brand".to_string(),
        ))
        .all(db)
        .await?;
        Ok(rows.into_iter().map(|r| r.brand).collect())
    }
}
