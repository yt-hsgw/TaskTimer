use serde::{Deserialize, Serialize};

use crate::domain::{
    notification::NotificationDisplayMode,
    timer::{WorkTargetRef, WorkTargetType},
};

use super::{
    repositories::{
        ActivePomodoro, ActiveTimer, BoardColumnRecord, DataExportManifestRecord, DataExportRecord,
        NativeNotificationRegistrationSummary, NextNotificationSchedule,
        NotificationDeliveryAttemptRecord, NotificationDispatchSummary, NotificationSyncResult,
        PomodoroSettingsRecord, RecurrenceRuleRecord, SqliteBackupManifestRecord,
        SqliteBackupRecord, SqliteRestoreRecord, SubtaskRecord, TagRecord, TaskListRecord,
        TaskNavigationCountsRecord, TaskPageCursor, TaskPageRecord, TaskRecord, TaskRowRecord,
        TaskTagRecord, TaskTimerSettingsRecord, TaskWithSubtasksRecord, UiPreferencesRecord,
        WeekCalendarItem, WorkItemSearchResultRecord,
    },
    usecases::{
        BoardColumnDraft, CalendarItemsDraft, DataExportCreateDraft, PomodoroSettingsDraft,
        RecurrenceRuleDraft, SqliteBackupCreateDraft, SqliteBackupRestoreDraft, TagDraft,
        TaskListDraft, TaskPageCursorDraft, TaskPageDraft, TaskPageScopeDraft,
        TaskTimerSettingsDraft, UiPreferencesDraft, WorkItemDraft, WorkItemSearchDraft,
        WorkItemUpdateDraft, WorkScheduleDraft, WorkScheduleMoveDraft,
    },
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
    pub list_id: Option<String>,
    pub title: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskInBoardColumnRequestDto {
    pub task: CreateTaskRequestDto,
    pub board_column_id: String,
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
pub struct WorkScheduleRequestDto {
    pub start_date: String,
    pub start_time: Option<String>,
    pub end_date: String,
    pub end_time: Option<String>,
    pub is_all_day: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateScheduledTaskRequestDto {
    pub list_id: Option<String>,
    pub title: String,
    pub memo: Option<String>,
    pub schedule: WorkScheduleRequestDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResizeScheduledWorkItemRequestDto {
    pub target: WorkTargetRefDto,
    pub schedule: WorkScheduleRequestDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkScheduleMoveRequestDto {
    pub start_date: String,
    pub start_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveScheduledWorkItemRequestDto {
    pub target: WorkTargetRefDto,
    pub destination: WorkScheduleMoveRequestDto,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskListRequestDto {
    pub name: String,
    pub color_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskListRequestDto {
    pub list_id: String,
    pub name: String,
    pub color_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTaskListRequestDto {
    pub list_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBoardColumnRequestDto {
    pub title: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBoardColumnRequestDto {
    pub column_id: String,
    pub title: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderBoardColumnsRequestDto {
    pub ordered_column_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteBoardColumnRequestDto {
    pub column_id: String,
    pub move_tasks_to_column_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveTaskToBoardColumnRequestDto {
    pub task_id: String,
    pub board_column_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTagRequestDto {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTagRequestDto {
    pub tag_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTagRequestDto {
    pub tag_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachTaskTagRequestDto {
    pub task_id: String,
    pub tag_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachTaskTagRequestDto {
    pub task_id: String,
    pub tag_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskRequestDto {
    pub task_id: String,
    pub list_id: Option<String>,
    pub title: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub timer_target_seconds: Option<i64>,
    pub color_token: Option<String>,
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
pub struct UpdatePomodoroSettingsRequestDto {
    pub work_seconds: i64,
    pub short_break_seconds: i64,
    pub long_break_seconds: i64,
    pub cycles_until_long_break: i64,
    pub auto_start_break: bool,
    pub auto_start_next_work: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskTimerSettingsRequestDto {
    pub default_target_seconds: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PomodoroSessionRequestDto {
    pub pomodoro_session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteTaskRequestDto {
    pub task_id: String,
    pub allow_incomplete_subtasks: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskStatusRequestDto {
    pub task_id: String,
    pub status: String,
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
pub struct ReopenSubtaskRequestDto {
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
pub struct ArchiveTaskRequestDto {
    pub task_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreArchivedTaskRequestDto {
    pub task_id: String,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSqliteBackupRequestDto {
    pub destination_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreSqliteBackupRequestDto {
    pub backup_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDataExportRequestDto {
    pub destination_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUiPreferencesRequestDto {
    pub left_pane_open: bool,
    pub last_view: String,
    pub last_task_list_id: String,
    pub calendar_view_mode: String,
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
    pub end_date: Option<String>,
    pub end_time: Option<String>,
    pub is_all_day: bool,
    pub marker: String,
    pub status: String,
    pub color_token: String,
    pub list_color_token: String,
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
    pub target_seconds: Option<i64>,
    pub accumulated_paused_seconds: i64,
    pub completion_reason: Option<String>,
    pub completion_notified_at: Option<String>,
    pub deleted_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskTimerSettingsDto {
    pub id: String,
    pub default_target_seconds: i64,
    pub updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PomodoroSettingsDto {
    pub id: String,
    pub work_seconds: i64,
    pub short_break_seconds: i64,
    pub long_break_seconds: i64,
    pub cycles_until_long_break: i64,
    pub auto_start_break: bool,
    pub auto_start_next_work: bool,
    pub updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePomodoroDto {
    pub id: String,
    pub scope: String,
    pub target: Option<WorkTargetRefDto>,
    pub timer_session_id: Option<String>,
    pub phase: String,
    pub status: String,
    pub cycle_count: i64,
    pub phase_started_at: String,
    pub phase_duration_seconds: i64,
    pub paused_at: Option<String>,
    pub paused_total_seconds: i64,
    pub completed_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PomodoroExpirySyncDto {
    pub expired_pomodoro: Option<ActivePomodoroDto>,
    pub active_pomodoro: Option<ActivePomodoroDto>,
    pub notification_summary: NotificationDispatchSummaryDto,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskCountdownExpirySyncDto {
    pub expired_timer: Option<ActiveTimerDto>,
    pub notification_summary: NotificationDispatchSummaryDto,
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
pub struct TaskTagDto {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDto {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
    pub task_count: i64,
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
    pub color_token: Option<String>,
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
    pub tags: Vec<TaskTagDto>,
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
    pub color_token: Option<String>,
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
    pub tags: Vec<TaskTagDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListDto {
    pub id: String,
    pub name: String,
    pub color_token: String,
    pub sort_order: i64,
    pub task_count: i64,
    pub active_task_count: i64,
    pub completed_task_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TaskPageScopeRequestDto {
    List {
        #[serde(rename = "listId")]
        list_id: String,
    },
    Today,
    Favorites,
    Tag {
        #[serde(rename = "tagId")]
        tag_id: String,
    },
    Board,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCalendarItemsRequestDto {
    pub start_date: String,
    pub end_date: String,
    pub scope: TaskPageScopeRequestDto,
    pub today_date: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPageCursorDto {
    pub completion_bucket: i64,
    pub sort_order: i64,
    pub created_at: String,
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTaskPageRequestDto {
    pub scope: TaskPageScopeRequestDto,
    pub today_date: String,
    pub cursor: Option<TaskPageCursorDto>,
    pub limit: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchWorkItemsRequestDto {
    pub query: String,
    pub limit: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemSearchResultDto {
    pub target: WorkTargetRefDto,
    pub task_id: String,
    pub title: String,
    pub parent_title: Option<String>,
    pub list_id: String,
    pub list_name: String,
    pub status: String,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub tags: Vec<TaskTagDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNavigationCountsDto {
    pub today_count: i64,
    pub favorite_count: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPageDto {
    pub tasks: Vec<TaskWithSubtasksDto>,
    pub rows: Vec<TaskRowDto>,
    pub total_count: i64,
    pub next_cursor: Option<TaskPageCursorDto>,
    pub navigation_counts: TaskNavigationCountsDto,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRowDto {
    pub id: String,
    pub list_id: String,
    pub board_column_id: String,
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
    pub tags: Vec<TaskTagDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDispatchSummaryDto {
    pub attempted: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub last_error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NextNotificationScheduleDto {
    pub notification_rule_id: String,
    pub notify_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSyncResultDto {
    pub dispatch_summary: NotificationDispatchSummaryDto,
    pub next_schedule: Option<NextNotificationScheduleDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeNotificationRegistrationSummaryDto {
    pub attempted: usize,
    pub registered: usize,
    pub cancelled: usize,
    pub skipped: usize,
    pub failed: usize,
    pub last_error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDeliveryAttemptDto {
    pub id: String,
    pub notification_rule_id: String,
    pub target: WorkTargetRefDto,
    pub kind: String,
    pub notify_at: String,
    pub attempted_at: String,
    pub result: String,
    pub error_message: Option<String>,
    pub attempt_count: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteBackupManifestDto {
    pub format: String,
    pub format_version: i64,
    pub app_version: String,
    pub schema_version: i64,
    pub created_at: String,
    pub platform: String,
    pub database_file: String,
    pub integrity_check: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteBackupDto {
    pub backup_dir: String,
    pub database_file: String,
    pub manifest_file: String,
    pub manifest: SqliteBackupManifestDto,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteRestoreDto {
    pub backup_dir: String,
    pub restored_at: String,
    pub previous_database_file: String,
    pub manifest: SqliteBackupManifestDto,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataExportManifestDto {
    pub format: String,
    pub format_version: i64,
    pub app_version: String,
    pub created_at: String,
    pub platform: String,
    pub compatibility: String,
    pub contains_personal_data: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataExportDto {
    pub export_path: String,
    pub files: Vec<String>,
    pub manifest: DataExportManifestDto,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPreferencesDto {
    pub left_pane_open: bool,
    pub last_view: String,
    pub last_task_list_id: String,
    pub calendar_view_mode: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoardColumnDto {
    pub id: String,
    pub title: String,
    pub sort_order: i64,
    pub task_count: i64,
    pub active_task_count: i64,
    pub completed_task_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ListTaskPageRequestDto> for TaskPageDraft {
    fn from(value: ListTaskPageRequestDto) -> Self {
        let scope = value.scope.into();
        Self {
            scope,
            today_date: value.today_date,
            cursor: value.cursor.map(|cursor| TaskPageCursorDraft {
                completion_bucket: cursor.completion_bucket,
                sort_order: cursor.sort_order,
                created_at: cursor.created_at,
                id: cursor.id,
            }),
            limit: value.limit,
        }
    }
}

impl From<TaskPageScopeRequestDto> for TaskPageScopeDraft {
    fn from(value: TaskPageScopeRequestDto) -> Self {
        match value {
            TaskPageScopeRequestDto::List { list_id } => Self::List { list_id },
            TaskPageScopeRequestDto::Today => Self::Today,
            TaskPageScopeRequestDto::Favorites => Self::Favorites,
            TaskPageScopeRequestDto::Tag { tag_id } => Self::Tag { tag_id },
            TaskPageScopeRequestDto::Board => Self::Board,
        }
    }
}

impl From<ListCalendarItemsRequestDto> for CalendarItemsDraft {
    fn from(value: ListCalendarItemsRequestDto) -> Self {
        Self {
            start_date: value.start_date,
            end_date: value.end_date,
            scope: value.scope.into(),
            today_date: value.today_date,
        }
    }
}

impl From<SearchWorkItemsRequestDto> for WorkItemSearchDraft {
    fn from(value: SearchWorkItemsRequestDto) -> Self {
        Self {
            query: value.query,
            limit: value.limit,
        }
    }
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
            list_id: value.list_id,
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
            list_id: None,
            title: value.title,
            planned_start_date: value.planned_start_date,
            due_date: value.due_date,
            due_time: value.due_time,
            memo: value.memo,
        }
    }
}

impl From<WorkScheduleRequestDto> for WorkScheduleDraft {
    fn from(value: WorkScheduleRequestDto) -> Self {
        Self {
            start_date: value.start_date,
            start_time: value.start_time,
            end_date: value.end_date,
            end_time: value.end_time,
            is_all_day: value.is_all_day,
        }
    }
}

impl From<WorkScheduleMoveRequestDto> for WorkScheduleMoveDraft {
    fn from(value: WorkScheduleMoveRequestDto) -> Self {
        Self {
            start_date: value.start_date,
            start_time: value.start_time,
        }
    }
}

impl From<CreateScheduledTaskRequestDto> for (WorkItemDraft, WorkScheduleDraft) {
    fn from(value: CreateScheduledTaskRequestDto) -> Self {
        (
            WorkItemDraft {
                list_id: value.list_id,
                title: value.title,
                planned_start_date: None,
                due_date: None,
                due_time: None,
                memo: value.memo,
            },
            value.schedule.into(),
        )
    }
}

impl From<UpdateTaskRequestDto> for WorkItemUpdateDraft {
    fn from(value: UpdateTaskRequestDto) -> Self {
        Self {
            list_id: value.list_id,
            title: value.title,
            planned_start_date: value.planned_start_date,
            due_date: value.due_date,
            due_time: value.due_time,
            timer_target_seconds: value.timer_target_seconds,
            color_token: value.color_token,
            recurrence_rule: value.recurrence_rule.map(Into::into),
            memo: value.memo,
        }
    }
}

impl From<UpdateSubtaskRequestDto> for WorkItemUpdateDraft {
    fn from(value: UpdateSubtaskRequestDto) -> Self {
        Self {
            list_id: None,
            title: value.title,
            planned_start_date: value.planned_start_date,
            due_date: value.due_date,
            due_time: value.due_time,
            timer_target_seconds: value.timer_target_seconds,
            color_token: None,
            recurrence_rule: value.recurrence_rule.map(Into::into),
            memo: value.memo,
        }
    }
}

impl From<CreateTaskListRequestDto> for TaskListDraft {
    fn from(value: CreateTaskListRequestDto) -> Self {
        Self {
            name: value.name,
            color_token: value.color_token,
        }
    }
}

impl From<CreateBoardColumnRequestDto> for BoardColumnDraft {
    fn from(value: CreateBoardColumnRequestDto) -> Self {
        Self { title: value.title }
    }
}

impl From<UpdateBoardColumnRequestDto> for BoardColumnDraft {
    fn from(value: UpdateBoardColumnRequestDto) -> Self {
        Self { title: value.title }
    }
}

impl From<UpdateTaskListRequestDto> for TaskListDraft {
    fn from(value: UpdateTaskListRequestDto) -> Self {
        Self {
            name: value.name,
            color_token: value.color_token,
        }
    }
}

impl From<CreateTagRequestDto> for TagDraft {
    fn from(value: CreateTagRequestDto) -> Self {
        Self { name: value.name }
    }
}

impl From<UpdateTagRequestDto> for TagDraft {
    fn from(value: UpdateTagRequestDto) -> Self {
        Self { name: value.name }
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

impl From<CreateSqliteBackupRequestDto> for SqliteBackupCreateDraft {
    fn from(value: CreateSqliteBackupRequestDto) -> Self {
        Self {
            destination_dir: value.destination_dir,
        }
    }
}

impl From<RestoreSqliteBackupRequestDto> for SqliteBackupRestoreDraft {
    fn from(value: RestoreSqliteBackupRequestDto) -> Self {
        Self {
            backup_dir: value.backup_dir,
        }
    }
}

impl From<CreateDataExportRequestDto> for DataExportCreateDraft {
    fn from(value: CreateDataExportRequestDto) -> Self {
        Self {
            destination_dir: value.destination_dir,
        }
    }
}

impl From<UpdateUiPreferencesRequestDto> for UiPreferencesDraft {
    fn from(value: UpdateUiPreferencesRequestDto) -> Self {
        Self {
            left_pane_open: value.left_pane_open,
            last_view: value.last_view,
            last_task_list_id: value.last_task_list_id,
            calendar_view_mode: value.calendar_view_mode,
        }
    }
}

impl From<UpdatePomodoroSettingsRequestDto> for PomodoroSettingsDraft {
    fn from(value: UpdatePomodoroSettingsRequestDto) -> Self {
        Self {
            work_seconds: value.work_seconds,
            short_break_seconds: value.short_break_seconds,
            long_break_seconds: value.long_break_seconds,
            cycles_until_long_break: value.cycles_until_long_break,
            auto_start_break: value.auto_start_break,
            auto_start_next_work: value.auto_start_next_work,
        }
    }
}

impl From<UpdateTaskTimerSettingsRequestDto> for TaskTimerSettingsDraft {
    fn from(value: UpdateTaskTimerSettingsRequestDto) -> Self {
        Self {
            default_target_seconds: value.default_target_seconds,
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
            end_date: value.end_date,
            end_time: value.end_time,
            is_all_day: value.is_all_day,
            marker: value.marker.as_str().to_string(),
            status: value.status.as_str().to_string(),
            color_token: value.color_token,
            list_color_token: value.list_color_token,
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
            color_token: value.color_token,
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
            tags: value.tags.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<TaskTagRecord> for TaskTagDto {
    fn from(value: TaskTagRecord) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<TagRecord> for TagDto {
    fn from(value: TagRecord) -> Self {
        Self {
            id: value.id,
            name: value.name,
            sort_order: value.sort_order,
            task_count: value.task_count,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<BoardColumnRecord> for BoardColumnDto {
    fn from(value: BoardColumnRecord) -> Self {
        Self {
            id: value.id,
            title: value.title,
            sort_order: value.sort_order,
            task_count: value.task_count,
            active_task_count: value.active_task_count,
            completed_task_count: value.completed_task_count,
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
            color_token: value.task.color_token,
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
            tags: value.task.tags.into_iter().map(Into::into).collect(),
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
            color_token: value.color_token,
            sort_order: value.sort_order,
            task_count: value.task_count,
            active_task_count: value.active_task_count,
            completed_task_count: value.completed_task_count,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<TaskPageCursor> for TaskPageCursorDto {
    fn from(value: TaskPageCursor) -> Self {
        Self {
            completion_bucket: value.completion_bucket,
            sort_order: value.sort_order,
            created_at: value.created_at,
            id: value.id,
        }
    }
}

impl From<TaskNavigationCountsRecord> for TaskNavigationCountsDto {
    fn from(value: TaskNavigationCountsRecord) -> Self {
        Self {
            today_count: value.today_count,
            favorite_count: value.favorite_count,
        }
    }
}

impl From<TaskPageRecord> for TaskPageDto {
    fn from(value: TaskPageRecord) -> Self {
        Self {
            tasks: value.tasks.into_iter().map(Into::into).collect(),
            rows: value.rows.into_iter().map(Into::into).collect(),
            total_count: value.total_count,
            next_cursor: value.next_cursor.map(Into::into),
            navigation_counts: value.navigation_counts.into(),
        }
    }
}

impl From<WorkItemSearchResultRecord> for WorkItemSearchResultDto {
    fn from(value: WorkItemSearchResultRecord) -> Self {
        Self {
            target: WorkTargetRefDto {
                r#type: value.target.target_type.as_str().to_string(),
                id: value.target.id,
            },
            task_id: value.task_id,
            title: value.title,
            parent_title: value.parent_title,
            list_id: value.list_id,
            list_name: value.list_name,
            status: value.status.as_str().to_string(),
            due_date: value.due_date,
            due_time: value.due_time,
            tags: value.tags.into_iter().map(Into::into).collect(),
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
            board_column_id: value.board_column_id,
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
            tags: value.tags.into_iter().map(Into::into).collect(),
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
            target_seconds: value.target_seconds,
            accumulated_paused_seconds: value.accumulated_paused_seconds,
            completion_reason: value
                .completion_reason
                .map(|reason| reason.as_str().to_string()),
            completion_notified_at: value.completion_notified_at,
            deleted_at: value.deleted_at,
            created_at: value.created_at,
        }
    }
}

impl From<TaskTimerSettingsRecord> for TaskTimerSettingsDto {
    fn from(value: TaskTimerSettingsRecord) -> Self {
        Self {
            id: value.id,
            default_target_seconds: value.default_target_seconds,
            updated_at: value.updated_at,
        }
    }
}

impl From<PomodoroSettingsRecord> for PomodoroSettingsDto {
    fn from(value: PomodoroSettingsRecord) -> Self {
        Self {
            id: value.id,
            work_seconds: value.work_seconds,
            short_break_seconds: value.short_break_seconds,
            long_break_seconds: value.long_break_seconds,
            cycles_until_long_break: value.cycles_until_long_break,
            auto_start_break: value.auto_start_break,
            auto_start_next_work: value.auto_start_next_work,
            updated_at: value.updated_at,
        }
    }
}

impl From<ActivePomodoro> for ActivePomodoroDto {
    fn from(value: ActivePomodoro) -> Self {
        Self {
            id: value.id,
            scope: value.scope.as_str().to_string(),
            target: value.target.map(|target| WorkTargetRefDto {
                r#type: target.target_type.as_str().to_string(),
                id: target.id,
            }),
            timer_session_id: value.timer_session_id,
            phase: value.phase.as_str().to_string(),
            status: value.status.as_str().to_string(),
            cycle_count: value.cycle_count,
            phase_started_at: value.phase_started_at,
            phase_duration_seconds: value.phase_duration_seconds,
            paused_at: value.paused_at,
            paused_total_seconds: value.paused_total_seconds,
            completed_at: value.completed_at,
            cancelled_at: value.cancelled_at,
            deleted_at: value.deleted_at,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<super::usecases::PomodoroExpirySyncResult> for PomodoroExpirySyncDto {
    fn from(value: super::usecases::PomodoroExpirySyncResult) -> Self {
        Self {
            expired_pomodoro: value.expired_pomodoro.map(Into::into),
            active_pomodoro: value.active_pomodoro.map(Into::into),
            notification_summary: value.notification_summary.into(),
        }
    }
}

impl From<super::usecases::TaskCountdownExpirySyncResult> for TaskCountdownExpirySyncDto {
    fn from(value: super::usecases::TaskCountdownExpirySyncResult) -> Self {
        Self {
            expired_timer: value.expired_timer.map(Into::into),
            notification_summary: value.notification_summary.into(),
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

impl From<NextNotificationSchedule> for NextNotificationScheduleDto {
    fn from(value: NextNotificationSchedule) -> Self {
        Self {
            notification_rule_id: value.notification_rule_id,
            notify_at: value.notify_at,
        }
    }
}

impl From<NotificationSyncResult> for NotificationSyncResultDto {
    fn from(value: NotificationSyncResult) -> Self {
        Self {
            dispatch_summary: value.dispatch_summary.into(),
            next_schedule: value.next_schedule.map(Into::into),
        }
    }
}

impl From<NativeNotificationRegistrationSummary> for NativeNotificationRegistrationSummaryDto {
    fn from(value: NativeNotificationRegistrationSummary) -> Self {
        Self {
            attempted: value.attempted,
            registered: value.registered,
            cancelled: value.cancelled,
            skipped: value.skipped,
            failed: value.failed,
            last_error: value.last_error,
        }
    }
}

impl From<NotificationDeliveryAttemptRecord> for NotificationDeliveryAttemptDto {
    fn from(value: NotificationDeliveryAttemptRecord) -> Self {
        Self {
            id: value.id,
            notification_rule_id: value.notification_rule_id,
            target: WorkTargetRefDto {
                r#type: value.target.target_type.as_str().to_string(),
                id: value.target.id,
            },
            kind: value.kind.as_str().to_string(),
            notify_at: value.notify_at,
            attempted_at: value.attempted_at,
            result: value.result.as_str().to_string(),
            error_message: value.error_message,
            attempt_count: value.attempt_count,
        }
    }
}

impl From<SqliteBackupManifestRecord> for SqliteBackupManifestDto {
    fn from(value: SqliteBackupManifestRecord) -> Self {
        Self {
            format: value.format,
            format_version: value.format_version,
            app_version: value.app_version,
            schema_version: value.schema_version,
            created_at: value.created_at,
            platform: value.platform,
            database_file: value.database_file,
            integrity_check: value.integrity_check,
        }
    }
}

impl From<SqliteBackupRecord> for SqliteBackupDto {
    fn from(value: SqliteBackupRecord) -> Self {
        Self {
            backup_dir: value.backup_dir,
            database_file: value.database_file,
            manifest_file: value.manifest_file,
            manifest: value.manifest.into(),
        }
    }
}

impl From<SqliteRestoreRecord> for SqliteRestoreDto {
    fn from(value: SqliteRestoreRecord) -> Self {
        Self {
            backup_dir: value.backup_dir,
            restored_at: value.restored_at,
            previous_database_file: value.previous_database_file,
            manifest: value.manifest.into(),
        }
    }
}

impl From<DataExportManifestRecord> for DataExportManifestDto {
    fn from(value: DataExportManifestRecord) -> Self {
        Self {
            format: value.format,
            format_version: value.format_version,
            app_version: value.app_version,
            created_at: value.created_at,
            platform: value.platform,
            compatibility: value.compatibility,
            contains_personal_data: value.contains_personal_data,
        }
    }
}

impl From<DataExportRecord> for DataExportDto {
    fn from(value: DataExportRecord) -> Self {
        Self {
            export_path: value.export_path,
            files: value.files,
            manifest: value.manifest.into(),
        }
    }
}

impl From<UiPreferencesRecord> for UiPreferencesDto {
    fn from(value: UiPreferencesRecord) -> Self {
        Self {
            left_pane_open: value.left_pane_open,
            last_view: value.last_view,
            last_task_list_id: value.last_task_list_id,
            calendar_view_mode: value.calendar_view_mode,
        }
    }
}
