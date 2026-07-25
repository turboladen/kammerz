use sea_orm::*;

use ::entity::shot::{self, Entity as Shot};
use ::entity::shot_lens::{self, Entity as ShotLens};

pub struct ShotService;

impl ShotService {
    pub async fn list_for_roll(
        db: &DatabaseConnection,
        roll_id: i32,
    ) -> Result<Vec<shot::Model>, DbErr> {
        shot::Model::find_by_statement(Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"SELECT * FROM shots WHERE roll_id = $1
               ORDER BY CAST(frame_number AS INTEGER), frame_number, id"#,
            vec![roll_id.into()],
        ))
        .all(db)
        .await
    }

    pub async fn get_by_id(db: &DatabaseConnection, id: i32) -> Result<Option<shot::Model>, DbErr> {
        Shot::find_by_id(id).one(db).await
    }

    pub async fn create(
        db: &impl ConnectionTrait,
        model: shot::ActiveModel,
    ) -> Result<shot::Model, DbErr> {
        model.insert(db).await
    }

    pub async fn update(
        db: &impl ConnectionTrait,
        model: shot::ActiveModel,
    ) -> Result<shot::Model, DbErr> {
        model.update(db).await
    }

    // --- Shot-Lens junction ---

    pub async fn get_lenses_for_shot(
        db: &DatabaseConnection,
        shot_id: i32,
    ) -> Result<Vec<i32>, DbErr> {
        let rows = ShotLens::find()
            .filter(shot_lens::Column::ShotId.eq(shot_id))
            .all(db)
            .await?;
        Ok(rows.into_iter().map(|r| r.lens_id).collect())
    }

    pub async fn set_lenses_for_shot(
        db: &impl ConnectionTrait,
        shot_id: i32,
        lens_ids: Vec<i32>,
    ) -> Result<(), DbErr> {
        // Delete all existing
        ShotLens::delete_many()
            .filter(shot_lens::Column::ShotId.eq(shot_id))
            .exec(db)
            .await?;

        // Bulk insert new
        if !lens_ids.is_empty() {
            let models: Vec<shot_lens::ActiveModel> = lens_ids
                .into_iter()
                .map(|lens_id| shot_lens::ActiveModel {
                    shot_id: Set(shot_id),
                    lens_id: Set(lens_id),
                })
                .collect();
            ShotLens::insert_many(models).exec(db).await?;
        }
        Ok(())
    }

    /// Batch-load all shot-lens associations for every shot in a roll (single query).
    pub async fn get_lenses_for_roll_shots(
        db: &DatabaseConnection,
        roll_id: i32,
    ) -> Result<Vec<(i32, i32)>, DbErr> {
        #[derive(Debug, FromQueryResult)]
        struct ShotLensRow {
            shot_id: i32,
            lens_id: i32,
        }
        let rows = ShotLensRow::find_by_statement(Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"SELECT sl.shot_id, sl.lens_id
               FROM shot_lenses sl
               JOIN shots s ON s.id = sl.shot_id
               WHERE s.roll_id = $1"#,
            vec![roll_id.into()],
        ))
        .all(db)
        .await?;
        Ok(rows.into_iter().map(|r| (r.shot_id, r.lens_id)).collect())
    }

    // --- Helpers ---

    pub async fn suggest_next_frame(
        db: &DatabaseConnection,
        roll_id: i32,
    ) -> Result<String, DbErr> {
        #[derive(Debug, FromQueryResult)]
        struct MaxRow {
            max_frame: Option<i32>,
        }
        let row = MaxRow::find_by_statement(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT MAX(CAST(frame_number AS INTEGER)) AS max_frame FROM shots WHERE roll_id = $1",
            [roll_id.into()],
        ))
        .one(db)
        .await?;

        let max_num = row.and_then(|r| r.max_frame).unwrap_or(0);
        Ok((max_num + 1).to_string())
    }

    pub async fn count_for_roll(db: &DatabaseConnection, roll_id: i32) -> Result<u64, DbErr> {
        Shot::find()
            .filter(shot::Column::RollId.eq(roll_id))
            .count(db)
            .await
    }
}
