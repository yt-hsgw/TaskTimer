use serde::Serialize;

use super::repositories::{ActiveTimer, WeekCalendarItem};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkTargetRefDto {
    pub r#type: String,
    pub id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekCalendarItemDto {
    pub id: String,
    pub target: WorkTargetRefDto,
    pub title: String,
    pub date: String,
    pub marker: String,
    pub status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveTimerDto {
    pub id: String,
    pub target: WorkTargetRefDto,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub elapsed_seconds: Option<i64>,
    pub deleted_at: Option<String>,
    pub created_at: String,
}

impl From<WeekCalendarItem> for WeekCalendarItemDto {
    fn from(value: WeekCalendarItem) -> Self {
        Self {
            id: value.id,
            target: WorkTargetRefDto {
                r#type: value.target.target_type.as_str().to_string(),
                id: value.target.id,
            },
            title: value.title,
            date: value.date,
            marker: value.marker.as_str().to_string(),
            status: value.status.as_str().to_string(),
        }
    }
}

impl From<ActiveTimer> for ActiveTimerDto {
    fn from(value: ActiveTimer) -> Self {
        Self {
            id: value.id,
            target: WorkTargetRefDto {
                r#type: value.target.target_type.as_str().to_string(),
                id: value.target.id,
            },
            started_at: value.started_at,
            stopped_at: value.stopped_at,
            elapsed_seconds: value.elapsed_seconds,
            deleted_at: value.deleted_at,
            created_at: value.created_at,
        }
    }
}
