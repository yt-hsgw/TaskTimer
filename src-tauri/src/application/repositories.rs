use crate::domain::{
    notification::{NotificationDisplayMode, NotificationKind, NotificationRegistrationStatus},
    recurrence::RecurrenceFrequency,
    task::WorkStatus,
    timer::{WorkTargetRef, WorkTargetType},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarMarker {
    PlannedStart,
    Due,
    ActiveTimer,
}

impl CalendarMarker {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PlannedStart => "planned_start",
            Self::Due => "due",
            Self::ActiveTimer => "active_timer",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekCalendarItem {
    pub id: String,
    pub target: WorkTargetRef,
    pub title: String,
    pub date: String,
    pub marker: CalendarMarker,
    pub status: WorkStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveTimer {
    pub id: String,
    pub target: WorkTargetRef,
    pub started_at: String,
    pub stopped_at: Option<String>,
    pub elapsed_seconds: Option<i64>,
    pub paused_at: Option<String>,
    pub deleted_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItemCreate {
    pub title: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub memo: String,
    pub now: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItemUpdate {
    pub title: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub timer_target_seconds: Option<i64>,
    pub recurrence_rule: Option<RecurrenceRuleInput>,
    pub memo: String,
    pub now: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceRuleInput {
    pub frequency: RecurrenceFrequency,
    pub interval: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceRuleRecord {
    pub id: String,
    pub target: WorkTargetRef,
    pub frequency: RecurrenceFrequency,
    pub interval: i64,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListRecord {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
    pub task_count: i64,
    pub active_task_count: i64,
    pub completed_task_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub status: WorkStatus,
    pub is_favorite: bool,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub timer_target_seconds: Option<i64>,
    pub recurrence_rule: Option<RecurrenceRuleRecord>,
    pub memo: String,
    pub sort_order: i64,
    pub completed_at: Option<String>,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtaskRecord {
    pub id: String,
    pub task_id: String,
    pub title: String,
    pub status: WorkStatus,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub timer_target_seconds: Option<i64>,
    pub recurrence_rule: Option<RecurrenceRuleRecord>,
    pub memo: String,
    pub sort_order: i64,
    pub completed_at: Option<String>,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskWithSubtasksRecord {
    pub task: TaskRecord,
    pub subtasks: Vec<SubtaskRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRowRecord {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub status: WorkStatus,
    pub is_favorite: bool,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub timer_target_seconds: Option<i64>,
    pub sort_order: i64,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub subtask_total_count: i64,
    pub completed_subtask_count: i64,
    pub active_timer_target: Option<WorkTargetRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationJob {
    pub id: String,
    pub target: WorkTargetRef,
    pub target_title: String,
    pub kind: NotificationKind,
    pub notify_at: String,
    pub registration_status: NotificationRegistrationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationDispatchSummary {
    pub attempted: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub last_error: Option<String>,
}

pub type RepositoryResult<T> = Result<T, String>;

pub trait CalendarRepository {
    fn list_week_calendar_items(
        &self,
        week_start_date: &str,
    ) -> RepositoryResult<Vec<WeekCalendarItem>>;
}

pub trait TimerRepository {
    fn get_active_timer(&self) -> RepositoryResult<Option<ActiveTimer>>;
}

pub trait TaskReadRepository {
    fn list_tasks_with_subtasks(&self, limit: i64)
        -> RepositoryResult<Vec<TaskWithSubtasksRecord>>;

    fn list_task_lists(&self) -> RepositoryResult<Vec<TaskListRecord>>;

    fn list_task_rows(
        &self,
        list_id: Option<&str>,
        limit: i64,
    ) -> RepositoryResult<Vec<TaskRowRecord>>;
}

pub trait TaskTimerCommandRepository {
    fn create_task(&self, input: WorkItemCreate) -> RepositoryResult<TaskRecord>;

    fn create_subtask(
        &self,
        task_id: String,
        input: WorkItemCreate,
    ) -> RepositoryResult<SubtaskRecord>;

    fn update_task(&self, task_id: String, input: WorkItemUpdate) -> RepositoryResult<TaskRecord>;

    fn update_subtask(
        &self,
        subtask_id: String,
        input: WorkItemUpdate,
    ) -> RepositoryResult<SubtaskRecord>;

    fn start_timer(&self, target: WorkTargetRef, now: String) -> RepositoryResult<ActiveTimer>;

    fn pause_active_timer(&self, now: String) -> RepositoryResult<ActiveTimer>;

    fn resume_active_timer(&self, now: String) -> RepositoryResult<ActiveTimer>;

    fn stop_active_timer(&self, now: String) -> RepositoryResult<ActiveTimer>;

    fn complete_task(
        &self,
        task_id: String,
        allow_incomplete_subtasks: bool,
        now: String,
    ) -> RepositoryResult<TaskRecord>;

    fn reopen_task(&self, task_id: String, now: String) -> RepositoryResult<TaskRecord>;

    fn complete_subtask(&self, subtask_id: String, now: String) -> RepositoryResult<SubtaskRecord>;

    fn toggle_task_favorite(
        &self,
        task_id: String,
        is_favorite: bool,
        now: String,
    ) -> RepositoryResult<TaskRecord>;

    fn delete_task(&self, task_id: String, now: String) -> RepositoryResult<()>;

    fn delete_subtask(&self, subtask_id: String, now: String) -> RepositoryResult<()>;
}

pub trait NotificationPreferenceRepository {
    fn get_notification_display_mode(&self) -> RepositoryResult<NotificationDisplayMode>;
}

pub trait NotificationCommandRepository {
    fn update_notification_display_mode(
        &self,
        display_mode: NotificationDisplayMode,
        now: String,
    ) -> RepositoryResult<NotificationDisplayMode>;

    fn list_due_notification_jobs(
        &self,
        now: &str,
        limit: i64,
    ) -> RepositoryResult<Vec<NotificationJob>>;

    fn mark_notification_registered(&self, id: &str, now: &str) -> RepositoryResult<()>;

    fn mark_notification_failed(&self, id: &str, error: &str, now: &str) -> RepositoryResult<()>;
}

pub fn target_ref(target_type: WorkTargetType, id: String) -> WorkTargetRef {
    WorkTargetRef { target_type, id }
}
