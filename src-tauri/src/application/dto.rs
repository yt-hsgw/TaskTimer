use serde::{Deserialize, Serialize};

use crate::domain::{
    notification::NotificationDisplayMode,
    timer::{WorkTargetRef, WorkTargetType},
};

use super::{
    repositories::{
        ActiveTimer, NotificationDispatchSummary, RecurrenceRuleRecord, SubtaskRecord,
        TaskListRecord, TaskRecord, TaskRowRecord, TaskWithSubtasksRecord, WeekCalendarItem,
    },
    usecases::{RecurrenceRuleDraft, WorkItemDraft, WorkItemUpdateDraft},
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
    pub due_time: Option<String>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubtaskRequestDto {
    pub task_id: String,
    pub title: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequestDto {
    pub task_id: String,
    pub title: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub timer_target_seconds: Option<i64>,
    pub recurrence_rule: Option<RecurrenceRuleRequestDto>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubtaskRequestDto {
    pub subtask_id: String,
    pub title: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub timer_target_seconds: Option<i64>,
    pub recurrence_rule: Option<RecurrenceRuleRequestDto>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceRuleRequestDto {
    pub frequency: String,
    pub interval: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTimerRequestDto {
    pub target: WorkTargetRefDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteTaskRequestDto {
    pub task_id: String,
    pub allow_incomplete_subtasks: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReopenTaskRequestDto {
    pub task_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSubtaskRequestDto {
    pub subtask_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleTaskFavoriteRequestDto {
    pub task_id: String,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTaskRequestDto {
    pub task_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSubtaskRequestDto {
    pub subtask_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotificationDisplayModeRequestDto {
    pub display_mode: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotificationsEnabledRequestDto {
    pub enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekCalendarItemDto {
    pub id: String,
    pub target: WorkTargetRefDto,
    pub title: String,
    pub parent_title: Option<String>,
    pub date: String,
    pub time: Option<String>,
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
    pub paused_at: Option<String>,
    pub deleted_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceRuleDto {
    pub id: String,
    pub target: WorkTargetRefDto,
    pub frequency: String,
    pub interval: i64,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDto {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub status: String,
    pub is_favorite: bool,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub timer_target_seconds: Option<i64>,
    pub recurrence_rule: Option<RecurrenceRuleDto>,
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
    pub due_time: Option<String>,
    pub timer_target_seconds: Option<i64>,
    pub recurrence_rule: Option<RecurrenceRuleDto>,
    pub memo: String,
    pub sort_order: i64,
    pub completed_at: Option<String>,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskWithSubtasksDto {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub status: String,
    pub is_favorite: bool,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub timer_target_seconds: Option<i64>,
    pub recurrence_rule: Option<RecurrenceRuleDto>,
    pub memo: String,
    pub sort_order: i64,
    pub completed_at: Option<String>,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub subtasks: Vec<SubtaskDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListDto {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
    pub task_count: i64,
    pub active_task_count: i64,
    pub completed_task_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRowDto {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub status: String,
    pub is_favorite: bool,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub timer_target_seconds: Option<i64>,
    pub sort_order: i64,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub subtask_total_count: i64,
    pub completed_subtask_count: i64,
    pub active_timer_target: Option<WorkTargetRefDto>,
    pub is_timer_active: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDispatchSummaryDto {
    pub attempted: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub last_error: Option<String>,
}

impl TryFrom<UpdateNotificationDisplayModeRequestDto> for NotificationDisplayMode {
    type Error = String;

    fn try_from(value: UpdateNotificationDisplayModeRequestDto) -> Result<Self, Self::Error> {
        NotificationDisplayMode::from_db(&value.display_mode)
    }
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
            due_time: value.due_time,
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
            due_time: value.due_time,
            memo: value.memo,
        }
    }
}

impl From<UpdateTaskRequestDto> for WorkItemUpdateDraft {
    fn from(value: UpdateTaskRequestDto) -> Self {
        Self {
            title: value.title,
            planned_start_date: value.planned_start_date,
            due_date: value.due_date,
            due_time: value.due_time,
            timer_target_seconds: value.timer_target_seconds,
            recurrence_rule: value.recurrence_rule.map(Into::into),
            memo: value.memo,
        }
    }
}

impl From<UpdateSubtaskRequestDto> for WorkItemUpdateDraft {
    fn from(value: UpdateSubtaskRequestDto) -> Self {
        Self {
            title: value.title,
            planned_start_date: value.planned_start_date,
            due_date: value.due_date,
            due_time: value.due_time,
            timer_target_seconds: value.timer_target_seconds,
            recurrence_rule: value.recurrence_rule.map(Into::into),
            memo: value.memo,
        }
    }
}

impl From<RecurrenceRuleRequestDto> for RecurrenceRuleDraft {
    fn from(value: RecurrenceRuleRequestDto) -> Self {
        Self {
            frequency: value.frequency,
            interval: value.interval,
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
            parent_title: value.parent_title,
            date: value.date,
            time: value.time,
            marker: value.marker.as_str().to_string(),
            status: value.status.as_str().to_string(),
        }
    }
}

impl From<TaskRecord> for TaskDto {
    fn from(value: TaskRecord) -> Self {
        Self {
            id: value.id,
            list_id: value.list_id,
            title: value.title,
            status: value.status.as_str().to_string(),
            is_favorite: value.is_favorite,
            planned_start_date: value.planned_start_date,
            due_date: value.due_date,
            due_time: value.due_time,
            timer_target_seconds: value.timer_target_seconds,
            recurrence_rule: value.recurrence_rule.map(Into::into),
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
            due_time: value.due_time,
            timer_target_seconds: value.timer_target_seconds,
            recurrence_rule: value.recurrence_rule.map(Into::into),
            memo: value.memo,
            sort_order: value.sort_order,
            completed_at: value.completed_at,
            deleted_at: value.deleted_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<TaskWithSubtasksRecord> for TaskWithSubtasksDto {
    fn from(value: TaskWithSubtasksRecord) -> Self {
        Self {
            id: value.task.id,
            list_id: value.task.list_id,
            title: value.task.title,
            status: value.task.status.as_str().to_string(),
            is_favorite: value.task.is_favorite,
            planned_start_date: value.task.planned_start_date,
            due_date: value.task.due_date,
            due_time: value.task.due_time,
            timer_target_seconds: value.task.timer_target_seconds,
            recurrence_rule: value.task.recurrence_rule.map(Into::into),
            memo: value.task.memo,
            sort_order: value.task.sort_order,
            completed_at: value.task.completed_at,
            deleted_at: value.task.deleted_at,
            created_at: value.task.created_at,
            updated_at: value.task.updated_at,
            subtasks: value.subtasks.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<RecurrenceRuleRecord> for RecurrenceRuleDto {
    fn from(value: RecurrenceRuleRecord) -> Self {
        Self {
            id: value.id,
            target: WorkTargetRefDto {
                r#type: value.target.target_type.as_str().to_string(),
                id: value.target.id,
            },
            frequency: value.frequency.as_str().to_string(),
            interval: value.interval,
            deleted_at: value.deleted_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<TaskListRecord> for TaskListDto {
    fn from(value: TaskListRecord) -> Self {
        Self {
            id: value.id,
            name: value.name,
            sort_order: value.sort_order,
            task_count: value.task_count,
            active_task_count: value.active_task_count,
            completed_task_count: value.completed_task_count,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<TaskRowRecord> for TaskRowDto {
    fn from(value: TaskRowRecord) -> Self {
        let active_timer_target = value.active_timer_target.map(|target| WorkTargetRefDto {
            r#type: target.target_type.as_str().to_string(),
            id: target.id,
        });
        let is_timer_active = active_timer_target.is_some();

        Self {
            id: value.id,
            list_id: value.list_id,
            title: value.title,
            status: value.status.as_str().to_string(),
            is_favorite: value.is_favorite,
            planned_start_date: value.planned_start_date,
            due_date: value.due_date,
            due_time: value.due_time,
            timer_target_seconds: value.timer_target_seconds,
            sort_order: value.sort_order,
            completed_at: value.completed_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
            subtask_total_count: value.subtask_total_count,
            completed_subtask_count: value.completed_subtask_count,
            active_timer_target,
            is_timer_active,
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
            paused_at: value.paused_at,
            deleted_at: value.deleted_at,
            created_at: value.created_at,
        }
    }
}

impl From<NotificationDispatchSummary> for NotificationDispatchSummaryDto {
    fn from(value: NotificationDispatchSummary) -> Self {
        Self {
            attempted: value.attempted,
            succeeded: value.succeeded,
            failed: value.failed,
            last_error: value.last_error,
        }
    }
}
