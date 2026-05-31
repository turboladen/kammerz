use sea_orm::*;

use ::entity::setting::{self, Entity as Setting};

pub struct SettingsService;

impl SettingsService {
    pub async fn get_setting(
        db: &DatabaseConnection,
        key: &str,
    ) -> Result<Option<String>, DbErr> {
        Ok(Setting::find_by_id(key).one(db).await?.map(|s| s.value))
    }

    pub async fn set_setting(
        db: &DatabaseConnection,
        key: String,
        value: String,
    ) -> Result<(), DbErr> {
        let model = setting::ActiveModel {
            key: Set(key),
            value: Set(value),
        };
        Setting::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(setting::Column::Key)
                    .update_column(setting::Column::Value)
                    .to_owned(),
            )
            .exec(db)
            .await?;
        Ok(())
    }
}
