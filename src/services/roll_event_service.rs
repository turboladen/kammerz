use sea_orm::*;

use ::entity::roll_event::{self, RefKind, RollEventType};

use crate::patch::now_string;

pub struct RollEventService;

impl RollEventService {
    /// Append one event. Takes `&impl ConnectionTrait` so it runs inside the
    /// same transaction as the mutation it records (atomic with it).
    ///
    /// The activity model (ADR-0013) has no stored status, so new events never
    /// populate `from_status`/`to_status` — those columns survive only to render
    /// historical `status_changed` rows in the activity journal.
    pub async fn record(
        db: &impl ConnectionTrait,
        roll_id: i32,
        event_type: RollEventType,
        ref_kind: Option<RefKind>,
        ref_id: Option<i32>,
        summary: String,
    ) -> Result<(), DbErr> {
        let now = now_string();
        let model = roll_event::ActiveModel {
            roll_id: Set(roll_id),
            event_type: Set(event_type),
            ref_kind: Set(ref_kind),
            ref_id: Set(ref_id),
            summary: Set(summary),
            occurred_at: Set(now.clone()),
            created_at: Set(now),
            ..Default::default()
        };
        model.insert(db).await?;
        Ok(())
    }

    /// Newest first. `id` desc tie-breaks events sharing an `occurred_at`.
    pub async fn list_for_roll(
        db: &impl ConnectionTrait,
        roll_id: i32,
    ) -> Result<Vec<roll_event::Model>, DbErr> {
        roll_event::Entity::find()
            .filter(roll_event::Column::RollId.eq(roll_id))
            .order_by_desc(roll_event::Column::OccurredAt)
            .order_by_desc(roll_event::Column::Id)
            .all(db)
            .await
    }
}
