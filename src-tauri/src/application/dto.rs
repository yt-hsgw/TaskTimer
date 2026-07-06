use serde::{Deserialize, Serialize};

use crate::domain::timer::{WorkTargetRef, WorkTargetType};

use super::{
    repositories::{ActiveTimer, SubtaskRecord, TaskRecord, WeekCalendarItem},
    usecases::WorkItemDraft,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkTargetRefDto {
    pub r#type: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequestDto {
    pub title: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubtaskRequestDto {
    pub task_id: String,
    pub title: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTimerRequestDto {
    pub target: WorkTargetRefDto,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDto {
    pub id: String,
    pub title: String,
    pub status: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub memo: String,
    pub sort_order: i64,
    pub completed_at: Option<String>,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtaskDto {
    pub id: String,
    pub task_id: String,
    pub title: String,
    pub status: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub memo: String,
    pub sort_order: i64,
    pub completed_at: Option<String>,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl TryFrom<WorkTargetRefDto> for WorkTargetRef {
    type Error = String;

    fn try_from(value: WorkTargetRefDto) -> Result<Self, Self::Error> {
        Ok(Self {
            target_type: WorkTargetType::from_db(&value.r#type)?,
            id: value.id,
        })
    }
}

impl From<CreateTaskRequestDto> for WorkItemDraft {
    fn from(value: CreateTaskRequestDto) -> Self {
        Self {
            title: value.title,
            planned_start_date: value.planned_start_date,
            due_date: value.due_date,
            memo: value.memo,
        }
    }
}

impl From<CreateSubtaskRequestDto> for WorkItemDraft {
    fn from(value: CreateSubtaskRequestDto) -> Self {
        Self {
            title: value.title,
            planned_start_date: value.planned_start_date,
            due_date: value.due_date,
            memo: value.memo,
        }
    }
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

impl From<TaskRecord> for TaskDto {
    fn from(value: TaskRecord) -> Self {
        Self {
            id: value.id,
            title: value.title,
            status: value.status.as_str().to_string(),
            planned_start_date: value.planned_start_date,
            due_date: value.due_date,
            memo: value.memo,
            sort_order: value.sort_order,
            completed_at: value.completed_at,
            deleted_at: value.deleted_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<SubtaskRecord> for SubtaskDto {
    fn from(value: SubtaskRecord) -> Self {
        Self {
            id: value.id,
            task_id: value.task_id,
            title: value.title,
            status: value.status.as_str().to_string(),
            planned_start_date: value.planned_start_date,
            due_date: value.due_date,
            memo: value.memo,
            sort_order: value.sort_order,
            completed_at: value.completed_at,
            deleted_at: value.deleted_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
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
