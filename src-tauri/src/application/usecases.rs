use crate::domain::{
    notification::{build_notification_content, NotificationDisplayMode},
    recurrence::RecurrenceFrequency,
    task::{
        validate_date_range, validate_due_time_requires_due_date, validate_memo,
        validate_optional_date, validate_optional_time, validate_task_list_name, validate_title,
        DEFAULT_TASK_LIST_ID,
    },
    timer::WorkTargetRef,
};

use super::{
    clock::Clock,
    notification::{LocalNotificationGateway, LocalNotificationMessage},
    repositories::{
        ActiveTimer, DataExportCreate, DataExportRecord, DataExportRepository,
        NotificationCommandRepository, NotificationDeliveryAttemptRecord,
        NotificationDispatchSummary, NotificationHistoryRepository,
        NotificationPreferenceRepository, RecurrenceRuleInput, RepositoryResult,
        SqliteBackupCreate, SqliteBackupRecord, SqliteBackupRepository, SqliteBackupRestore,
        SqliteRestoreRecord, SubtaskRecord, TaskListCommandRepository, TaskListCreate,
        TaskListRecord, TaskListUpdate, TaskRecord, TaskTimerCommandRepository,
        UiPreferenceRepository, UiPreferencesRecord, UiPreferencesUpdate, WorkItemCreate,
        WorkItemUpdate, CURRENT_SQLITE_BACKUP_SCHEMA_VERSION,
    },
};

const NOTIFICATION_DISPATCH_LIMIT: i64 = 20;
const NOTIFICATION_HISTORY_LIMIT: i64 = 20;
const TIMER_TARGET_MAX_SECONDS: i64 = 60 * 60 * 24 * 30;
const RECURRENCE_INTERVAL_MAX: i64 = 365;
const LOCAL_PATH_MAX_CHARS: usize = 4096;

const UI_VIEW_LIST: &str = "list";
const UI_VIEW_TODAY: &str = "today";
const UI_VIEW_FAVORITES: &str = "favorites";
const UI_VIEW_CALENDAR: &str = "calendar";
const UI_VIEW_SETTINGS: &str = "settings";
const CALENDAR_VIEW_WEEK: &str = "week";
const CALENDAR_VIEW_DAY: &str = "day";
const CALENDAR_VIEW_MONTH: &str = "month";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItemDraft {
    pub list_id: Option<String>,
    pub title: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItemUpdateDraft {
    pub list_id: Option<String>,
    pub title: String,
    pub planned_start_date: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub timer_target_seconds: Option<i64>,
    pub recurrence_rule: Option<RecurrenceRuleDraft>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceRuleDraft {
    pub frequency: String,
    pub interval: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskListDraft {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteBackupCreateDraft {
    pub destination_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteBackupRestoreDraft {
    pub backup_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataExportCreateDraft {
    pub destination_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiPreferencesDraft {
    pub left_pane_open: bool,
    pub last_view: String,
    pub last_task_list_id: String,
    pub calendar_view_mode: String,
}

pub fn create_task(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    draft: WorkItemDraft,
) -> RepositoryResult<TaskRecord> {
    repository.create_task(validate_work_item_draft(draft, clock.now_utc_iso8601())?)
}

pub fn create_task_list(
    repository: &impl TaskListCommandRepository,
    clock: &impl Clock,
    draft: TaskListDraft,
) -> RepositoryResult<TaskListRecord> {
    repository.create_task_list(TaskListCreate {
        name: validate_task_list_name(&draft.name)?,
        now: clock.now_utc_iso8601(),
    })
}

pub fn update_task_list(
    repository: &impl TaskListCommandRepository,
    clock: &impl Clock,
    list_id: String,
    draft: TaskListDraft,
) -> RepositoryResult<TaskListRecord> {
    let list_id = validate_identifier(&list_id, "リストID")?;
    repository.update_task_list(
        list_id,
        TaskListUpdate {
            name: validate_task_list_name(&draft.name)?,
            now: clock.now_utc_iso8601(),
        },
    )
}

pub fn delete_task_list(
    repository: &impl TaskListCommandRepository,
    clock: &impl Clock,
    list_id: String,
) -> RepositoryResult<()> {
    let list_id = validate_identifier(&list_id, "リストID")?;
    repository.delete_task_list(list_id, clock.now_utc_iso8601())
}

pub fn create_subtask(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    task_id: String,
    draft: WorkItemDraft,
) -> RepositoryResult<SubtaskRecord> {
    let task_id = validate_identifier(&task_id, "親タスクID")?;
    repository.create_subtask(
        task_id,
        validate_work_item_draft(draft, clock.now_utc_iso8601())?,
    )
}

pub fn update_task(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    task_id: String,
    draft: WorkItemUpdateDraft,
) -> RepositoryResult<TaskRecord> {
    let task_id = validate_identifier(&task_id, "タスクID")?;
    repository.update_task(
        task_id,
        validate_work_item_update_draft(draft, clock.now_utc_iso8601())?,
    )
}

pub fn update_subtask(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    subtask_id: String,
    draft: WorkItemUpdateDraft,
) -> RepositoryResult<SubtaskRecord> {
    let subtask_id = validate_identifier(&subtask_id, "サブタスクID")?;
    repository.update_subtask(
        subtask_id,
        validate_work_item_update_draft(draft, clock.now_utc_iso8601())?,
    )
}

pub fn start_timer(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    target: WorkTargetRef,
) -> RepositoryResult<ActiveTimer> {
    let target = WorkTargetRef {
        target_type: target.target_type,
        id: validate_identifier(&target.id, "対象ID")?,
    };
    repository.start_timer(target, clock.now_utc_iso8601())
}

pub fn pause_active_timer(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
) -> RepositoryResult<ActiveTimer> {
    repository.pause_active_timer(clock.now_utc_iso8601())
}

pub fn resume_active_timer(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
) -> RepositoryResult<ActiveTimer> {
    repository.resume_active_timer(clock.now_utc_iso8601())
}

pub fn stop_active_timer(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
) -> RepositoryResult<ActiveTimer> {
    repository.stop_active_timer(clock.now_utc_iso8601())
}

pub fn complete_task(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    task_id: String,
    allow_incomplete_subtasks: bool,
) -> RepositoryResult<TaskRecord> {
    let task_id = validate_identifier(&task_id, "タスクID")?;
    repository.complete_task(task_id, allow_incomplete_subtasks, clock.now_utc_iso8601())
}

pub fn reopen_task(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    task_id: String,
) -> RepositoryResult<TaskRecord> {
    let task_id = validate_identifier(&task_id, "タスクID")?;
    repository.reopen_task(task_id, clock.now_utc_iso8601())
}

pub fn complete_subtask(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    subtask_id: String,
) -> RepositoryResult<SubtaskRecord> {
    let subtask_id = validate_identifier(&subtask_id, "サブタスクID")?;
    repository.complete_subtask(subtask_id, clock.now_utc_iso8601())
}

pub fn reopen_subtask(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    subtask_id: String,
) -> RepositoryResult<SubtaskRecord> {
    let subtask_id = validate_identifier(&subtask_id, "サブタスクID")?;
    repository.reopen_subtask(subtask_id, clock.now_utc_iso8601())
}

pub fn toggle_task_favorite(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    task_id: String,
    is_favorite: bool,
) -> RepositoryResult<TaskRecord> {
    let task_id = validate_identifier(&task_id, "タスクID")?;
    repository.toggle_task_favorite(task_id, is_favorite, clock.now_utc_iso8601())
}

pub fn archive_task(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    task_id: String,
) -> RepositoryResult<TaskRecord> {
    let task_id = validate_identifier(&task_id, "タスクID")?;
    repository.archive_task(task_id, clock.now_utc_iso8601())
}

pub fn restore_archived_task(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    task_id: String,
) -> RepositoryResult<TaskRecord> {
    let task_id = validate_identifier(&task_id, "タスクID")?;
    repository.restore_archived_task(task_id, clock.now_utc_iso8601())
}

pub fn delete_task(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    task_id: String,
) -> RepositoryResult<()> {
    let task_id = validate_identifier(&task_id, "タスクID")?;
    repository.delete_task(task_id, clock.now_utc_iso8601())
}

pub fn delete_subtask(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    subtask_id: String,
) -> RepositoryResult<()> {
    let subtask_id = validate_identifier(&subtask_id, "サブタスクID")?;
    repository.delete_subtask(subtask_id, clock.now_utc_iso8601())
}

pub fn update_notification_display_mode(
    repository: &impl NotificationCommandRepository,
    clock: &impl Clock,
    display_mode: NotificationDisplayMode,
) -> RepositoryResult<NotificationDisplayMode> {
    repository.update_notification_display_mode(display_mode, clock.now_utc_iso8601())
}

pub fn update_notifications_enabled(
    repository: &impl NotificationCommandRepository,
    clock: &impl Clock,
    enabled: bool,
) -> RepositoryResult<bool> {
    repository.update_notifications_enabled(enabled, clock.now_utc_iso8601())
}

pub fn dispatch_due_notifications(
    repository: &(impl NotificationCommandRepository + NotificationPreferenceRepository),
    notification_gateway: &impl LocalNotificationGateway,
    clock: &impl Clock,
) -> RepositoryResult<NotificationDispatchSummary> {
    let now = clock.now_utc_iso8601();
    if !repository.get_notifications_enabled()? {
        return Ok(NotificationDispatchSummary {
            attempted: 0,
            succeeded: 0,
            failed: 0,
            last_error: None,
        });
    }

    let display_mode = repository.get_notification_display_mode()?;
    let jobs = repository.list_due_notification_jobs(&now, NOTIFICATION_DISPATCH_LIMIT)?;

    let mut summary = NotificationDispatchSummary {
        attempted: jobs.len(),
        succeeded: 0,
        failed: 0,
        last_error: None,
    };

    for job in jobs {
        let content = build_notification_content(&display_mode, &job.target_title);
        let result = notification_gateway.send(&LocalNotificationMessage {
            title: content.title,
            body: content.body,
        });

        match result {
            Ok(()) => {
                repository.mark_notification_registered(&job, &now)?;
                summary.succeeded += 1;
            }
            Err(error) => {
                repository.mark_notification_failed(&job, &error, &now)?;
                summary.failed += 1;
                summary.last_error = Some(error);
            }
        }
    }

    Ok(summary)
}

pub fn list_notification_failure_history(
    repository: &impl NotificationHistoryRepository,
) -> RepositoryResult<Vec<NotificationDeliveryAttemptRecord>> {
    repository.list_notification_failure_history(NOTIFICATION_HISTORY_LIMIT)
}

pub fn create_sqlite_backup(
    repository: &impl SqliteBackupRepository,
    clock: &impl Clock,
    draft: SqliteBackupCreateDraft,
) -> RepositoryResult<SqliteBackupRecord> {
    repository.create_sqlite_backup(SqliteBackupCreate {
        destination_dir: validate_local_path(&draft.destination_dir, "バックアップ保存先")?,
        now: clock.now_utc_iso8601(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        schema_version: CURRENT_SQLITE_BACKUP_SCHEMA_VERSION,
    })
}

pub fn restore_sqlite_backup(
    repository: &impl SqliteBackupRepository,
    clock: &impl Clock,
    draft: SqliteBackupRestoreDraft,
) -> RepositoryResult<SqliteRestoreRecord> {
    repository.restore_sqlite_backup(SqliteBackupRestore {
        backup_dir: validate_local_path(&draft.backup_dir, "バックアップフォルダ")?,
        now: clock.now_utc_iso8601(),
    })
}

pub fn create_json_export(
    repository: &impl DataExportRepository,
    clock: &impl Clock,
    draft: DataExportCreateDraft,
) -> RepositoryResult<DataExportRecord> {
    repository.create_json_export(DataExportCreate {
        destination_dir: validate_local_path(&draft.destination_dir, "エクスポート保存先")?,
        now: clock.now_utc_iso8601(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
    })
}

pub fn create_csv_export(
    repository: &impl DataExportRepository,
    clock: &impl Clock,
    draft: DataExportCreateDraft,
) -> RepositoryResult<DataExportRecord> {
    repository.create_csv_export(DataExportCreate {
        destination_dir: validate_local_path(&draft.destination_dir, "エクスポート保存先")?,
        now: clock.now_utc_iso8601(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
    })
}

pub fn get_ui_preferences(
    repository: &impl UiPreferenceRepository,
) -> RepositoryResult<UiPreferencesRecord> {
    repository.get_ui_preferences()
}

pub fn update_ui_preferences(
    repository: &impl UiPreferenceRepository,
    clock: &impl Clock,
    draft: UiPreferencesDraft,
) -> RepositoryResult<UiPreferencesRecord> {
    repository.update_ui_preferences(UiPreferencesUpdate {
        left_pane_open: draft.left_pane_open,
        last_view: validate_ui_view(&draft.last_view)?,
        last_task_list_id: validate_identifier(&draft.last_task_list_id, "最後のリストID")?,
        calendar_view_mode: validate_calendar_view_mode(&draft.calendar_view_mode)?,
        now: clock.now_utc_iso8601(),
    })
}

fn validate_work_item_draft(draft: WorkItemDraft, now: String) -> RepositoryResult<WorkItemCreate> {
    let title = validate_title(&draft.title)?;
    let planned_start_date = validate_optional_date(draft.planned_start_date.as_deref(), "開始日")?;
    let due_date = validate_optional_date(draft.due_date.as_deref(), "期限日")?;
    let due_time = validate_optional_time(draft.due_time.as_deref(), "期限時刻")?;
    validate_date_range(&planned_start_date, &due_date)?;
    validate_due_time_requires_due_date(&due_date, &due_time)?;
    let memo = validate_memo(draft.memo.as_deref())?;
    let list_id = validate_create_list_id(draft.list_id.as_deref())?;

    Ok(WorkItemCreate {
        list_id,
        title,
        planned_start_date,
        due_date,
        due_time,
        memo,
        now,
    })
}

fn validate_work_item_update_draft(
    draft: WorkItemUpdateDraft,
    now: String,
) -> RepositoryResult<WorkItemUpdate> {
    let title = validate_title(&draft.title)?;
    let planned_start_date = validate_optional_date(draft.planned_start_date.as_deref(), "開始日")?;
    let due_date = validate_optional_date(draft.due_date.as_deref(), "期限日")?;
    let due_time = validate_optional_time(draft.due_time.as_deref(), "期限時刻")?;
    validate_date_range(&planned_start_date, &due_date)?;
    validate_due_time_requires_due_date(&due_date, &due_time)?;
    let timer_target_seconds = validate_timer_target_seconds(draft.timer_target_seconds)?;
    let recurrence_rule =
        validate_recurrence_rule(draft.recurrence_rule, &planned_start_date, &due_date)?;
    let memo = validate_memo(draft.memo.as_deref())?;
    let list_id = validate_update_list_id(draft.list_id.as_deref())?;

    Ok(WorkItemUpdate {
        list_id,
        title,
        planned_start_date,
        due_date,
        due_time,
        timer_target_seconds,
        recurrence_rule,
        memo,
        now,
    })
}

fn validate_create_list_id(value: Option<&str>) -> RepositoryResult<String> {
    match value {
        Some(raw_value) if !raw_value.trim().is_empty() => {
            validate_identifier(raw_value, "リストID")
        }
        _ => Ok(DEFAULT_TASK_LIST_ID.to_string()),
    }
}

fn validate_update_list_id(value: Option<&str>) -> RepositoryResult<Option<String>> {
    match value {
        Some(raw_value) if !raw_value.trim().is_empty() => {
            validate_identifier(raw_value, "リストID").map(Some)
        }
        _ => Ok(None),
    }
}

fn validate_timer_target_seconds(value: Option<i64>) -> RepositoryResult<Option<i64>> {
    let Some(seconds) = value else {
        return Ok(None);
    };
    if seconds <= 0 {
        return Err("タイマー目標時間は1秒以上で入力してください".to_string());
    }
    if seconds > TIMER_TARGET_MAX_SECONDS {
        return Err("タイマー目標時間は30日以内で入力してください".to_string());
    }
    Ok(Some(seconds))
}

fn validate_recurrence_rule(
    value: Option<RecurrenceRuleDraft>,
    planned_start_date: &Option<String>,
    due_date: &Option<String>,
) -> RepositoryResult<Option<RecurrenceRuleInput>> {
    let Some(rule) = value else {
        return Ok(None);
    };
    if planned_start_date.is_none() && due_date.is_none() {
        return Err("繰り返し設定には開始日または終了日が必要です".to_string());
    }
    if rule.interval < 1 || rule.interval > RECURRENCE_INTERVAL_MAX {
        return Err(format!(
            "繰り返し間隔は1以上{RECURRENCE_INTERVAL_MAX}以下で入力してください"
        ));
    }
    Ok(Some(RecurrenceRuleInput {
        frequency: RecurrenceFrequency::from_db(&rule.frequency)?,
        interval: rule.interval,
    }))
}

fn validate_identifier(value: &str, field_label: &str) -> RepositoryResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_label}は必須です"));
    }
    if trimmed.chars().count() > 128 {
        return Err(format!("{field_label}は128文字以内で入力してください"));
    }
    Ok(trimmed.to_string())
}

fn validate_ui_view(value: &str) -> RepositoryResult<String> {
    let trimmed = value.trim();
    match trimmed {
        UI_VIEW_LIST | UI_VIEW_TODAY | UI_VIEW_FAVORITES | UI_VIEW_CALENDAR | UI_VIEW_SETTINGS => {
            Ok(trimmed.to_string())
        }
        _ => Err("最後のビュー設定が不正です".to_string()),
    }
}

fn validate_calendar_view_mode(value: &str) -> RepositoryResult<String> {
    let trimmed = value.trim();
    match trimmed {
        CALENDAR_VIEW_WEEK | CALENDAR_VIEW_DAY | CALENDAR_VIEW_MONTH => Ok(trimmed.to_string()),
        _ => Err("カレンダー表示モード設定が不正です".to_string()),
    }
}

fn validate_local_path(value: &str, field_label: &str) -> RepositoryResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_label}は必須です"));
    }
    if trimmed.chars().count() > LOCAL_PATH_MAX_CHARS {
        return Err(format!(
            "{field_label}は{LOCAL_PATH_MAX_CHARS}文字以内で指定してください"
        ));
    }
    if trimmed.contains('\0') {
        return Err(format!("{field_label}に不正な文字が含まれています"));
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_work_item_draft_rejects_blank_title() {
        let result = validate_work_item_draft(
            WorkItemDraft {
                list_id: None,
                title: "   ".to_string(),
                planned_start_date: None,
                due_date: None,
                due_time: None,
                memo: None,
            },
            "2026-07-06T00:00:00Z".to_string(),
        );

        assert!(result.expect_err("blank title").contains("タイトル"));
    }

    #[test]
    fn validate_work_item_draft_rejects_reversed_date_range() {
        let result = validate_work_item_draft(
            WorkItemDraft {
                list_id: None,
                title: "設計レビュー".to_string(),
                planned_start_date: Some("2026-07-07".to_string()),
                due_date: Some("2026-07-06".to_string()),
                due_time: None,
                memo: None,
            },
            "2026-07-06T00:00:00Z".to_string(),
        );

        assert!(result.expect_err("reversed date range").contains("期限日"));
    }

    #[test]
    fn validate_work_item_draft_rejects_due_time_without_due_date() {
        let result = validate_work_item_draft(
            WorkItemDraft {
                list_id: None,
                title: "通知時刻だけ".to_string(),
                planned_start_date: None,
                due_date: None,
                due_time: Some("09:30".to_string()),
                memo: None,
            },
            "2026-07-06T00:00:00Z".to_string(),
        );

        assert!(result.expect_err("due date is required").contains("期限日"));
    }

    #[test]
    fn validate_ui_preferences_rejects_unknown_values() {
        assert!(validate_ui_view("board")
            .expect_err("invalid view")
            .contains("ビュー"));
        assert!(validate_calendar_view_mode("year")
            .expect_err("invalid calendar mode")
            .contains("カレンダー"));
    }
}
