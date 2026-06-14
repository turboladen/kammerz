use sea_orm::*;

use ::entity::roll::RollStatus;
use ::entity::roll_event::{self, RefKind, RollEventType};

use crate::patch::now_string;

pub struct RollEventService;

impl RollEventService {
    /// Append one event. Takes `&impl ConnectionTrait` so it runs inside the
    /// same transaction as the mutation it records (atomic with it).
    #[allow(clippy::too_many_arguments)]
    pub async fn record(
        db: &impl ConnectionTrait,
        roll_id: i32,
        event_type: RollEventType,
        from_status: Option<RollStatus>,
        to_status: Option<RollStatus>,
        ref_kind: Option<RefKind>,
        ref_id: Option<i32>,
        summary: String,
    ) -> Result<(), DbErr> {
        let now = now_string();
        let model = roll_event::ActiveModel {
            roll_id: Set(roll_id),
            event_type: Set(event_type),
            from_status: Set(from_status),
            to_status: Set(to_status),
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

    /// Convenience for the most common event.
    pub async fn record_status_change(
        db: &impl ConnectionTrait,
        roll_id: i32,
        from: RollStatus,
        to: RollStatus,
    ) -> Result<(), DbErr> {
        let summary = format!("Status changed to {}", status_label(&to));
        Self::record(
            db,
            roll_id,
            RollEventType::StatusChanged,
            Some(from),
            Some(to),
            None,
            None,
            summary,
        )
        .await
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

/// Human label for a status, for the denormalized `summary` fallback. The
/// frontend renders its own label from `to_status`; this keeps the row readable
/// without a frontend.
fn status_label(s: &RollStatus) -> &'static str {
    match s {
        RollStatus::Loaded => "Loaded",
        RollStatus::Shooting => "Shooting",
        RollStatus::Shot => "Shot",
        RollStatus::AtLab => "At Lab",
        RollStatus::LabDone => "Lab Done",
        RollStatus::Developing => "Developing",
        RollStatus::Developed => "Developed",
        RollStatus::Scanned => "Scanned",
        RollStatus::PostProcessed => "Post-processed",
        RollStatus::Archived => "Archived",
    }
}
