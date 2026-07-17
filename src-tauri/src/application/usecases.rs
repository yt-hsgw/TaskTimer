use crate::domain::{
    notification::{
        build_notification_content, NotificationDisplayMode, NotificationOsRegistrationAction,
    },
    pomodoro::{
        validate_pomodoro_cycles_until_long_break, validate_pomodoro_duration_seconds,
        PomodoroPhase,
    },
    recurrence::RecurrenceFrequency,
    task::{
        validate_date_range, validate_due_time_requires_due_date, validate_memo,
        validate_optional_date, validate_optional_time, validate_tag_name,
        validate_task_list_color_token, validate_task_list_name, validate_title, WorkStatus,
        DEFAULT_TASK_LIST_COLOR_TOKEN, DEFAULT_TASK_LIST_ID,
    },
    timer::WorkTargetRef,
};

use super::{
    clock::Clock,
    notification::{
        LocalNotificationGateway, LocalNotificationMessage, NativeNotificationRegistrationGateway,
        NativeNotificationRegistrationRequest,
    },
    repositories::{
        ActivePomodoro, ActiveTimer, DataExportCreate, DataExportRecord, DataExportRepository,
        NativeNotificationOsRegistrationRepository, NativeNotificationRegistrationSummary,
        NextNotificationSchedule, NotificationCommandRepository, NotificationDeliveryAttemptRecord,
        NotificationDispatchSummary, NotificationHistoryRepository, NotificationOsRegistrationJob,
        NotificationOsRegistrationRepository, NotificationPreferenceRepository,
        NotificationScheduleRepository, NotificationSyncResult, PomodoroRepository,
        PomodoroSettingsRecord, PomodoroSettingsUpdate, RecurrenceRuleInput, RepositoryResult,
        SqliteBackupCreate, SqliteBackupRecord, SqliteBackupRepository, SqliteBackupRestore,
        SqliteRestoreRecord, SubtaskRecord, TagCreate, TagRecord, TagRepository, TagUpdate,
        TaskListCommandRepository, TaskListCreate, TaskListRecord, TaskListUpdate, TaskRecord,
        TaskStatusUpdate, TaskTagRecord, TaskTimerCommandRepository, UiPreferenceRepository,
        UiPreferencesRecord, UiPreferencesUpdate, WorkItemCreate, WorkItemUpdate,
        CURRENT_SQLITE_BACKUP_SCHEMA_VERSION,
    },
};

const NOTIFICATION_DISPATCH_LIMIT: i64 = 20;
const NOTIFICATION_HISTORY_LIMIT: i64 = 20;
#[allow(dead_code)]
const NOTIFICATION_OS_REGISTRATION_LIMIT: i64 = 50;
#[allow(dead_code)]
const OS_REGISTRATION_ID_MAX_CHARS: usize = 256;
const TIMER_TARGET_MAX_SECONDS: i64 = 60 * 60 * 24 * 30;
const RECURRENCE_INTERVAL_MAX: i64 = 365;
const LOCAL_PATH_MAX_CHARS: usize = 4096;

const UI_VIEW_LIST: &str = "list";
const UI_VIEW_TODAY: &str = "today";
const UI_VIEW_FAVORITES: &str = "favorites";
const UI_VIEW_BOARD: &str = "board";
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
    pub color_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagDraft {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PomodoroSettingsDraft {
    pub work_seconds: i64,
    pub short_break_seconds: i64,
    pub long_break_seconds: i64,
    pub cycles_until_long_break: i64,
    pub auto_start_break: bool,
    pub auto_start_next_work: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PomodoroExpirySyncResult {
    pub expired_pomodoro: Option<ActivePomodoro>,
    pub active_pomodoro: Option<ActivePomodoro>,
    pub notification_summary: NotificationDispatchSummary,
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
        color_token: validate_task_list_color_token(
            draft
                .color_token
                .as_deref()
                .unwrap_or(DEFAULT_TASK_LIST_COLOR_TOKEN),
        )?,
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
            color_token: draft
                .color_token
                .as_deref()
                .map(validate_task_list_color_token)
                .transpose()?,
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

pub fn list_tags(repository: &impl TagRepository) -> RepositoryResult<Vec<TagRecord>> {
    repository.list_tags()
}

pub fn create_tag(
    repository: &impl TagRepository,
    clock: &impl Clock,
    draft: TagDraft,
) -> RepositoryResult<TagRecord> {
    repository.create_tag(TagCreate {
        name: validate_tag_name(&draft.name)?,
        now: clock.now_utc_iso8601(),
    })
}

pub fn update_tag(
    repository: &impl TagRepository,
    clock: &impl Clock,
    tag_id: String,
    draft: TagDraft,
) -> RepositoryResult<TagRecord> {
    let tag_id = validate_identifier(&tag_id, "タグID")?;
    repository.update_tag(
        tag_id,
        TagUpdate {
            name: validate_tag_name(&draft.name)?,
            now: clock.now_utc_iso8601(),
        },
    )
}

pub fn delete_tag(
    repository: &impl TagRepository,
    clock: &impl Clock,
    tag_id: String,
) -> RepositoryResult<()> {
    let tag_id = validate_identifier(&tag_id, "タグID")?;
    repository.delete_tag(tag_id, clock.now_utc_iso8601())
}

pub fn attach_tag_to_task(
    repository: &impl TagRepository,
    clock: &impl Clock,
    task_id: String,
    tag_id: String,
) -> RepositoryResult<TaskTagRecord> {
    let task_id = validate_identifier(&task_id, "タスクID")?;
    let tag_id = validate_identifier(&tag_id, "タグID")?;
    repository.attach_tag_to_task(task_id, tag_id, clock.now_utc_iso8601())
}

pub fn detach_tag_from_task(
    repository: &impl TagRepository,
    clock: &impl Clock,
    task_id: String,
    tag_id: String,
) -> RepositoryResult<()> {
    let task_id = validate_identifier(&task_id, "タスクID")?;
    let tag_id = validate_identifier(&tag_id, "タグID")?;
    repository.detach_tag_from_task(task_id, tag_id, clock.now_utc_iso8601())
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
    let target = validate_work_target_ref(target)?;
    repository.start_timer(target, clock.now_utc_iso8601())
}

pub fn get_pomodoro_settings(
    repository: &impl PomodoroRepository,
) -> RepositoryResult<PomodoroSettingsRecord> {
    repository.get_pomodoro_settings()
}

pub fn update_pomodoro_settings(
    repository: &impl PomodoroRepository,
    clock: &impl Clock,
    draft: PomodoroSettingsDraft,
) -> RepositoryResult<PomodoroSettingsRecord> {
    repository.update_pomodoro_settings(PomodoroSettingsUpdate {
        work_seconds: validate_pomodoro_duration_seconds(draft.work_seconds, "作業時間")?,
        short_break_seconds: validate_pomodoro_duration_seconds(
            draft.short_break_seconds,
            "短い休憩時間",
        )?,
        long_break_seconds: validate_pomodoro_duration_seconds(
            draft.long_break_seconds,
            "長い休憩時間",
        )?,
        cycles_until_long_break: validate_pomodoro_cycles_until_long_break(
            draft.cycles_until_long_break,
        )?,
        auto_start_break: draft.auto_start_break,
        auto_start_next_work: draft.auto_start_next_work,
        now: clock.now_utc_iso8601(),
    })
}

pub fn get_active_pomodoro(
    repository: &impl PomodoroRepository,
) -> RepositoryResult<Option<ActivePomodoro>> {
    repository.get_active_pomodoro()
}

pub fn start_pomodoro(
    repository: &impl PomodoroRepository,
    clock: &impl Clock,
    target: WorkTargetRef,
) -> RepositoryResult<ActivePomodoro> {
    let target = validate_work_target_ref(target)?;
    repository.start_pomodoro(target, clock.now_utc_iso8601())
}

pub fn pause_pomodoro(
    repository: &impl PomodoroRepository,
    clock: &impl Clock,
) -> RepositoryResult<ActivePomodoro> {
    repository.pause_pomodoro(clock.now_utc_iso8601())
}

pub fn resume_pomodoro(
    repository: &impl PomodoroRepository,
    clock: &impl Clock,
) -> RepositoryResult<ActivePomodoro> {
    repository.resume_pomodoro(clock.now_utc_iso8601())
}

pub fn complete_pomodoro_work_phase(
    repository: &impl PomodoroRepository,
    clock: &impl Clock,
) -> RepositoryResult<ActivePomodoro> {
    repository.complete_pomodoro_work_phase(clock.now_utc_iso8601())
}

pub fn start_pomodoro_break(
    repository: &impl PomodoroRepository,
    clock: &impl Clock,
    pomodoro_session_id: String,
) -> RepositoryResult<ActivePomodoro> {
    let pomodoro_session_id = validate_identifier(&pomodoro_session_id, "ポモドーロセッションID")?;
    repository.start_pomodoro_break(pomodoro_session_id, clock.now_utc_iso8601())
}

pub fn skip_pomodoro_break(
    repository: &impl PomodoroRepository,
    clock: &impl Clock,
    pomodoro_session_id: String,
) -> RepositoryResult<ActivePomodoro> {
    let pomodoro_session_id = validate_identifier(&pomodoro_session_id, "ポモドーロセッションID")?;
    repository.skip_pomodoro_break(pomodoro_session_id, clock.now_utc_iso8601())
}

pub fn complete_pomodoro_break(
    repository: &impl PomodoroRepository,
    clock: &impl Clock,
) -> RepositoryResult<ActivePomodoro> {
    repository.complete_pomodoro_break(clock.now_utc_iso8601())
}

pub fn cancel_pomodoro(
    repository: &impl PomodoroRepository,
    clock: &impl Clock,
) -> RepositoryResult<ActivePomodoro> {
    repository.cancel_pomodoro(clock.now_utc_iso8601())
}

pub fn sync_expired_pomodoro(
    repository: &(impl PomodoroRepository + NotificationPreferenceRepository),
    notification_gateway: &impl LocalNotificationGateway,
    clock: &impl Clock,
) -> RepositoryResult<PomodoroExpirySyncResult> {
    let now = clock.now_utc_iso8601();
    let Some(expiry) = repository.sync_expired_pomodoro(now)? else {
        return Ok(PomodoroExpirySyncResult {
            expired_pomodoro: None,
            active_pomodoro: None,
            notification_summary: empty_notification_summary(),
        });
    };

    let mut notification_summary = empty_notification_summary();
    if repository.get_notifications_enabled()? {
        notification_summary.attempted = 1;
        let display_mode = repository.get_notification_display_mode()?;
        let message = build_pomodoro_expiry_notification(
            &display_mode,
            &expiry.target_title,
            &expiry.expired_pomodoro.phase,
        );
        match notification_gateway.send(&message) {
            Ok(()) => {
                notification_summary.succeeded = 1;
            }
            Err(error) => {
                notification_summary.failed = 1;
                notification_summary.last_error = Some(error);
            }
        }
    }

    Ok(PomodoroExpirySyncResult {
        expired_pomodoro: Some(expiry.expired_pomodoro),
        active_pomodoro: expiry.active_pomodoro,
        notification_summary,
    })
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

pub fn update_task_status(
    repository: &impl TaskTimerCommandRepository,
    clock: &impl Clock,
    task_id: String,
    status: String,
    allow_incomplete_subtasks: bool,
) -> RepositoryResult<TaskRecord> {
    let task_id = validate_identifier(&task_id, "タスクID")?;
    let status = validate_board_task_status(&status)?;
    repository.update_task_status(
        task_id,
        TaskStatusUpdate {
            status,
            allow_incomplete_subtasks,
            now: clock.now_utc_iso8601(),
        },
    )
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

pub fn get_next_pending_notification(
    repository: &(impl NotificationPreferenceRepository + NotificationScheduleRepository),
    clock: &impl Clock,
) -> RepositoryResult<Option<NextNotificationSchedule>> {
    if !repository.get_notifications_enabled()? {
        return Ok(None);
    }

    repository.get_next_pending_notification(&clock.now_utc_iso8601())
}

pub fn sync_notifications<R>(
    repository: &R,
    notification_gateway: &impl LocalNotificationGateway,
    clock: &impl Clock,
) -> RepositoryResult<NotificationSyncResult>
where
    R: NotificationCommandRepository
        + NotificationPreferenceRepository
        + NotificationScheduleRepository,
{
    let dispatch_summary = dispatch_due_notifications(repository, notification_gateway, clock)?;
    let next_schedule = get_next_pending_notification(repository, clock)?;
    Ok(NotificationSyncResult {
        dispatch_summary,
        next_schedule,
    })
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

fn empty_notification_summary() -> NotificationDispatchSummary {
    NotificationDispatchSummary {
        attempted: 0,
        succeeded: 0,
        failed: 0,
        last_error: None,
    }
}

fn build_pomodoro_expiry_notification(
    mode: &NotificationDisplayMode,
    target_title: &str,
    phase: &PomodoroPhase,
) -> LocalNotificationMessage {
    let body = match phase {
        PomodoroPhase::Work => "ポモドーロの作業時間が終了しました。",
        PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak => {
            "ポモドーロの休憩時間が終了しました。"
        }
    };
    match mode {
        NotificationDisplayMode::TitleOnly => LocalNotificationMessage {
            title: target_title.trim().to_string(),
            body: body.to_string(),
        },
        NotificationDisplayMode::Generic => LocalNotificationMessage {
            title: "TaskTimer".to_string(),
            body: body.to_string(),
        },
    }
}

pub fn list_notification_failure_history(
    repository: &impl NotificationHistoryRepository,
) -> RepositoryResult<Vec<NotificationDeliveryAttemptRecord>> {
    repository.list_notification_failure_history(NOTIFICATION_HISTORY_LIMIT)
}

#[allow(dead_code)]
pub fn list_notification_os_registration_jobs(
    repository: &impl NotificationOsRegistrationRepository,
    clock: &impl Clock,
) -> RepositoryResult<Vec<NotificationOsRegistrationJob>> {
    repository.list_notification_os_registration_jobs(
        &clock.now_utc_iso8601(),
        NOTIFICATION_OS_REGISTRATION_LIMIT,
    )
}

#[allow(dead_code)]
pub fn mark_notification_os_registration_registered(
    repository: &impl NotificationOsRegistrationRepository,
    clock: &impl Clock,
    registration_id: String,
    os_registration_id: String,
) -> RepositoryResult<()> {
    let registration_id = validate_identifier(&registration_id, "通知OS登録ID")?;
    let os_registration_id = validate_os_registration_id(&os_registration_id)?;
    repository.mark_notification_os_registration_registered(
        registration_id,
        os_registration_id,
        clock.now_utc_iso8601(),
    )
}

#[allow(dead_code)]
pub fn mark_notification_os_registration_failed(
    repository: &impl NotificationOsRegistrationRepository,
    clock: &impl Clock,
    registration_id: String,
    error: &str,
) -> RepositoryResult<()> {
    let registration_id = validate_identifier(&registration_id, "通知OS登録ID")?;
    repository.mark_notification_os_registration_failed(
        registration_id,
        error,
        clock.now_utc_iso8601(),
    )
}

#[allow(dead_code)]
pub fn mark_notification_os_registration_cancelled(
    repository: &impl NotificationOsRegistrationRepository,
    clock: &impl Clock,
    registration_id: String,
) -> RepositoryResult<()> {
    let registration_id = validate_identifier(&registration_id, "通知OS登録ID")?;
    repository.mark_notification_os_registration_cancelled(registration_id, clock.now_utc_iso8601())
}

pub fn process_notification_os_registration_jobs<R>(
    repository: &R,
    native_gateway: &impl NativeNotificationRegistrationGateway,
    clock: &impl Clock,
) -> RepositoryResult<NativeNotificationRegistrationSummary>
where
    R: NativeNotificationOsRegistrationRepository + NotificationPreferenceRepository,
{
    let mut summary = NativeNotificationRegistrationSummary {
        attempted: 0,
        registered: 0,
        cancelled: 0,
        skipped: 0,
        failed: 0,
        last_error: None,
    };

    if !native_gateway.is_available() {
        return Ok(summary);
    }

    let now = clock.now_utc_iso8601();
    let notifications_enabled = repository.get_notifications_enabled()?;
    let display_mode = repository.get_notification_display_mode()?;
    let jobs = repository
        .list_native_notification_os_registration_jobs(&now, NOTIFICATION_OS_REGISTRATION_LIMIT)?;

    for job in jobs {
        match job.action {
            NotificationOsRegistrationAction::RegisterOrReplace => {
                if !notifications_enabled {
                    summary.skipped += 1;
                    continue;
                }

                summary.attempted += 1;
                let content = build_notification_content(&display_mode, &job.target_title);
                let request = NativeNotificationRegistrationRequest {
                    registration_id: job.id.clone(),
                    existing_os_registration_id: job.os_registration_id.clone(),
                    title: content.title,
                    body: content.body,
                    notify_at: job.notify_at,
                };

                match native_gateway.register_or_replace(&request) {
                    Ok(os_registration_id) => {
                        repository.mark_notification_os_registration_registered(
                            job.id,
                            os_registration_id,
                            clock.now_utc_iso8601(),
                        )?;
                        summary.registered += 1;
                    }
                    Err(error) => {
                        repository.mark_notification_os_registration_failed(
                            job.id,
                            &error,
                            clock.now_utc_iso8601(),
                        )?;
                        summary.failed += 1;
                        summary.last_error = Some(error);
                    }
                }
            }
            NotificationOsRegistrationAction::Cancel => {
                let Some(os_registration_id) = job.os_registration_id.as_deref() else {
                    repository.mark_notification_os_registration_cancelled(
                        job.id,
                        clock.now_utc_iso8601(),
                    )?;
                    summary.cancelled += 1;
                    continue;
                };

                summary.attempted += 1;
                match native_gateway.cancel(os_registration_id) {
                    Ok(()) => {
                        repository.mark_notification_os_registration_cancelled(
                            job.id,
                            clock.now_utc_iso8601(),
                        )?;
                        summary.cancelled += 1;
                    }
                    Err(error) => {
                        repository.mark_notification_os_registration_failed(
                            job.id,
                            &error,
                            clock.now_utc_iso8601(),
                        )?;
                        summary.failed += 1;
                        summary.last_error = Some(error);
                    }
                }
            }
        }
    }

    Ok(summary)
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

#[allow(dead_code)]
fn validate_os_registration_id(value: &str) -> RepositoryResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("OS登録IDは必須です".to_string());
    }
    if trimmed.chars().count() > OS_REGISTRATION_ID_MAX_CHARS {
        return Err(format!(
            "OS登録IDは{OS_REGISTRATION_ID_MAX_CHARS}文字以内で入力してください"
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err("OS登録IDに制御文字は使えません".to_string());
    }
    Ok(trimmed.to_string())
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

fn validate_work_target_ref(target: WorkTargetRef) -> RepositoryResult<WorkTargetRef> {
    Ok(WorkTargetRef {
        target_type: target.target_type,
        id: validate_identifier(&target.id, "対象ID")?,
    })
}

fn validate_ui_view(value: &str) -> RepositoryResult<String> {
    let trimmed = value.trim();
    match trimmed {
        UI_VIEW_LIST | UI_VIEW_TODAY | UI_VIEW_FAVORITES | UI_VIEW_BOARD | UI_VIEW_CALENDAR
        | UI_VIEW_SETTINGS => Ok(trimmed.to_string()),
        _ => Err("最後のビュー設定が不正です".to_string()),
    }
}

fn validate_board_task_status(value: &str) -> RepositoryResult<WorkStatus> {
    let status = WorkStatus::from_db(value.trim())?;
    match status {
        WorkStatus::Todo | WorkStatus::InProgress | WorkStatus::Done => Ok(status),
        WorkStatus::Archived => Err("かんばんからアーカイブ状態へ直接変更できません".to_string()),
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
    use std::cell::RefCell;

    use crate::{
        application::repositories::{
            NativeNotificationOsRegistrationJob, NotificationOsRegistrationJob,
        },
        domain::{
            notification::{NotificationKind, NotificationOsRegistrationStatus},
            timer::WorkTargetType,
        },
    };

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
        assert_eq!(validate_ui_view("board").expect("board view"), "board");
        assert!(validate_ui_view("unknown")
            .expect_err("invalid view")
            .contains("ビュー"));
        assert!(validate_calendar_view_mode("year")
            .expect_err("invalid calendar mode")
            .contains("カレンダー"));
    }

    #[test]
    fn validate_board_task_status_rejects_archived() {
        assert_eq!(
            validate_board_task_status("in_progress").expect("valid status"),
            WorkStatus::InProgress
        );
        assert!(validate_board_task_status("archived")
            .expect_err("archive is separate")
            .contains("アーカイブ"));
    }

    #[test]
    fn process_notification_os_registration_jobs_skips_without_native_gateway() {
        let repository = FakeNativeRegistrationRepository::new(
            NotificationDisplayMode::TitleOnly,
            true,
            vec![native_registration_job(
                "registration-1",
                NotificationOsRegistrationStatus::Pending,
                None,
                "秘密のタスク",
            )],
        );
        let gateway = FakeNativeRegistrationGateway::unavailable();
        let clock = FixedClock("2026-07-17T00:00:00Z");

        let summary =
            process_notification_os_registration_jobs(&repository, &gateway, &clock).expect("sync");

        assert_eq!(summary.attempted, 0);
        assert_eq!(summary.registered, 0);
        assert_eq!(*repository.native_list_calls.borrow(), 0);
        assert!(gateway.registered_requests.borrow().is_empty());
    }

    #[test]
    fn process_notification_os_registration_jobs_uses_generic_content_when_configured() {
        let repository = FakeNativeRegistrationRepository::new(
            NotificationDisplayMode::Generic,
            true,
            vec![native_registration_job(
                "registration-1",
                NotificationOsRegistrationStatus::Pending,
                None,
                "社外秘タスク",
            )],
        );
        let gateway = FakeNativeRegistrationGateway::available();
        let clock = FixedClock("2026-07-17T00:00:00Z");

        let summary =
            process_notification_os_registration_jobs(&repository, &gateway, &clock).expect("sync");

        assert_eq!(summary.attempted, 1);
        assert_eq!(summary.registered, 1);
        let requests = gateway.registered_requests.borrow();
        assert_eq!(requests[0].title, "TaskTimer");
        assert_eq!(requests[0].body, "予定時刻です");
        assert_eq!(
            repository.registered.borrow()[0],
            (
                "registration-1".to_string(),
                "os:registration-1".to_string(),
                "2026-07-17T00:00:00Z".to_string()
            )
        );
    }

    #[test]
    fn process_notification_os_registration_jobs_cancels_existing_os_registration() {
        let repository = FakeNativeRegistrationRepository::new(
            NotificationDisplayMode::TitleOnly,
            true,
            vec![native_registration_job(
                "registration-1",
                NotificationOsRegistrationStatus::CancelPending,
                Some("os-existing"),
                "タスク",
            )],
        );
        let gateway = FakeNativeRegistrationGateway::available();
        let clock = FixedClock("2026-07-17T00:00:00Z");

        let summary =
            process_notification_os_registration_jobs(&repository, &gateway, &clock).expect("sync");

        assert_eq!(summary.attempted, 1);
        assert_eq!(summary.cancelled, 1);
        assert_eq!(gateway.cancelled_ids.borrow()[0], "os-existing");
        assert_eq!(
            repository.cancelled.borrow()[0],
            (
                "registration-1".to_string(),
                "2026-07-17T00:00:00Z".to_string()
            )
        );
    }

    #[test]
    fn process_notification_os_registration_jobs_marks_registration_failure() {
        let repository = FakeNativeRegistrationRepository::new(
            NotificationDisplayMode::TitleOnly,
            true,
            vec![native_registration_job(
                "registration-1",
                NotificationOsRegistrationStatus::Pending,
                None,
                "タスク",
            )],
        );
        let gateway =
            FakeNativeRegistrationGateway::available_with_error("Windows通知予約を登録できません");
        let clock = FixedClock("2026-07-17T00:00:00Z");

        let summary =
            process_notification_os_registration_jobs(&repository, &gateway, &clock).expect("sync");

        assert_eq!(summary.attempted, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(
            repository.failed.borrow()[0],
            (
                "registration-1".to_string(),
                "Windows通知予約を登録できません".to_string(),
                "2026-07-17T00:00:00Z".to_string()
            )
        );
    }

    struct FixedClock(&'static str);

    impl Clock for FixedClock {
        fn now_utc_iso8601(&self) -> String {
            self.0.to_string()
        }
    }

    struct FakeNativeRegistrationRepository {
        display_mode: NotificationDisplayMode,
        notifications_enabled: bool,
        native_jobs: RefCell<Vec<NativeNotificationOsRegistrationJob>>,
        native_list_calls: RefCell<usize>,
        registered: RefCell<Vec<(String, String, String)>>,
        failed: RefCell<Vec<(String, String, String)>>,
        cancelled: RefCell<Vec<(String, String)>>,
    }

    impl FakeNativeRegistrationRepository {
        fn new(
            display_mode: NotificationDisplayMode,
            notifications_enabled: bool,
            native_jobs: Vec<NativeNotificationOsRegistrationJob>,
        ) -> Self {
            Self {
                display_mode,
                notifications_enabled,
                native_jobs: RefCell::new(native_jobs),
                native_list_calls: RefCell::new(0),
                registered: RefCell::new(Vec::new()),
                failed: RefCell::new(Vec::new()),
                cancelled: RefCell::new(Vec::new()),
            }
        }
    }

    impl NotificationPreferenceRepository for FakeNativeRegistrationRepository {
        fn get_notification_display_mode(&self) -> RepositoryResult<NotificationDisplayMode> {
            Ok(self.display_mode.clone())
        }

        fn get_notifications_enabled(&self) -> RepositoryResult<bool> {
            Ok(self.notifications_enabled)
        }
    }

    impl NotificationOsRegistrationRepository for FakeNativeRegistrationRepository {
        fn list_notification_os_registration_jobs(
            &self,
            _now: &str,
            _limit: i64,
        ) -> RepositoryResult<Vec<NotificationOsRegistrationJob>> {
            Ok(Vec::new())
        }

        fn mark_notification_os_registration_registered(
            &self,
            registration_id: String,
            os_registration_id: String,
            now: String,
        ) -> RepositoryResult<()> {
            self.registered
                .borrow_mut()
                .push((registration_id, os_registration_id, now));
            Ok(())
        }

        fn mark_notification_os_registration_failed(
            &self,
            registration_id: String,
            error: &str,
            now: String,
        ) -> RepositoryResult<()> {
            self.failed
                .borrow_mut()
                .push((registration_id, error.to_string(), now));
            Ok(())
        }

        fn mark_notification_os_registration_cancelled(
            &self,
            registration_id: String,
            now: String,
        ) -> RepositoryResult<()> {
            self.cancelled.borrow_mut().push((registration_id, now));
            Ok(())
        }
    }

    impl NativeNotificationOsRegistrationRepository for FakeNativeRegistrationRepository {
        fn list_native_notification_os_registration_jobs(
            &self,
            _now: &str,
            _limit: i64,
        ) -> RepositoryResult<Vec<NativeNotificationOsRegistrationJob>> {
            *self.native_list_calls.borrow_mut() += 1;
            Ok(self.native_jobs.borrow().clone())
        }
    }

    struct FakeNativeRegistrationGateway {
        available: bool,
        error: RefCell<Option<String>>,
        registered_requests: RefCell<Vec<NativeNotificationRegistrationRequest>>,
        cancelled_ids: RefCell<Vec<String>>,
    }

    impl FakeNativeRegistrationGateway {
        fn available() -> Self {
            Self {
                available: true,
                error: RefCell::new(None),
                registered_requests: RefCell::new(Vec::new()),
                cancelled_ids: RefCell::new(Vec::new()),
            }
        }

        fn available_with_error(error: &str) -> Self {
            Self {
                available: true,
                error: RefCell::new(Some(error.to_string())),
                registered_requests: RefCell::new(Vec::new()),
                cancelled_ids: RefCell::new(Vec::new()),
            }
        }

        fn unavailable() -> Self {
            Self {
                available: false,
                error: RefCell::new(None),
                registered_requests: RefCell::new(Vec::new()),
                cancelled_ids: RefCell::new(Vec::new()),
            }
        }
    }

    impl NativeNotificationRegistrationGateway for FakeNativeRegistrationGateway {
        fn is_available(&self) -> bool {
            self.available
        }

        fn register_or_replace(
            &self,
            request: &NativeNotificationRegistrationRequest,
        ) -> Result<String, String> {
            self.registered_requests.borrow_mut().push(request.clone());
            if let Some(error) = self.error.borrow_mut().take() {
                return Err(error);
            }
            Ok(format!("os:{}", request.registration_id))
        }

        fn cancel(&self, os_registration_id: &str) -> Result<(), String> {
            self.cancelled_ids
                .borrow_mut()
                .push(os_registration_id.to_string());
            Ok(())
        }
    }

    fn native_registration_job(
        id: &str,
        registration_status: NotificationOsRegistrationStatus,
        os_registration_id: Option<&str>,
        target_title: &str,
    ) -> NativeNotificationOsRegistrationJob {
        let action = NotificationOsRegistrationAction::from_status(&registration_status);
        NativeNotificationOsRegistrationJob {
            id: id.to_string(),
            notification_rule_id: "rule-1".to_string(),
            os_registration_id: os_registration_id.map(str::to_string),
            target: WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: "task-1".to_string(),
            },
            target_title: target_title.to_string(),
            kind: NotificationKind::Due,
            notify_at: "2026-07-17T00:05:00Z".to_string(),
            registration_status,
            action,
            last_attempted_at: None,
            last_error: None,
        }
    }
}
