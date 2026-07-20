#![allow(dead_code)]

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration as StdDuration,
};

use rusqlite::{params, params_from_iter, Connection, OpenFlags, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use time::{
    format_description::well_known::Rfc3339, macros::format_description, Date, Duration,
    OffsetDateTime,
};
use uuid::Uuid;

use crate::{
    application::repositories::{
        target_ref, ActivePomodoro, ActiveTimer, BoardColumnCreate, BoardColumnDelete,
        BoardColumnRecord, BoardColumnReorder, BoardColumnRepository, BoardColumnUpdate,
        BoardTaskMove, CalendarMarker, CalendarRepository, DataExportCreate,
        DataExportManifestRecord, DataExportRecord, DataExportRepository,
        NativeNotificationOsRegistrationJob, NativeNotificationOsRegistrationRepository,
        NextNotificationSchedule, NotificationCommandRepository, NotificationDeliveryAttemptRecord,
        NotificationHistoryRepository, NotificationJob, NotificationOsRegistrationJob,
        NotificationOsRegistrationRepository, NotificationPreferenceRepository,
        NotificationScheduleRepository, PomodoroExpiry, PomodoroRepository, PomodoroSettingsRecord,
        PomodoroSettingsUpdate, RecurrenceRuleInput, RecurrenceRuleRecord, RepositoryResult,
        SqliteBackupCreate, SqliteBackupManifestRecord, SqliteBackupRecord, SqliteBackupRepository,
        SqliteBackupRestore, SqliteRestoreRecord, SubtaskRecord, TagCreate, TagRecord,
        TagRepository, TagUpdate, TaskCountdownExpiry, TaskListCommandRepository, TaskListCreate,
        TaskListRecord, TaskListUpdate, TaskNavigationCountsRecord, TaskPageCursor, TaskPageQuery,
        TaskPageRecord, TaskPageScope, TaskReadRepository, TaskRecord, TaskRowRecord,
        TaskStatusUpdate, TaskTagRecord, TaskTimerCommandRepository, TaskTimerSettingsRecord,
        TaskTimerSettingsUpdate, TaskWithSubtasksRecord, TimerRepository, UiPreferenceRepository,
        UiPreferencesRecord, UiPreferencesUpdate, WeekCalendarItem, WorkItemCreate,
        WorkItemSearchQuery, WorkItemSearchResultRecord, WorkItemUpdate, WorkScheduleMove,
        WorkScheduleUpdate, CURRENT_SQLITE_BACKUP_SCHEMA_VERSION,
    },
    domain::{
        notification::{
            NotificationDeliveryResult, NotificationDisplayMode, NotificationKind,
            NotificationOsRegistrationAction, NotificationOsRegistrationStatus,
            NotificationRegistrationStatus,
        },
        pomodoro::{
            next_break_phase, PomodoroPhase, PomodoroScope, PomodoroStatus,
            DEFAULT_POMODORO_CYCLES_UNTIL_LONG_BREAK, DEFAULT_POMODORO_LONG_BREAK_SECONDS,
            DEFAULT_POMODORO_SETTINGS_ID, DEFAULT_POMODORO_SHORT_BREAK_SECONDS,
            DEFAULT_POMODORO_WORK_SECONDS,
        },
        recurrence::RecurrenceFrequency,
        task::{
            assert_completable, assert_timer_startable, WorkSchedule, WorkStatus,
            DEFAULT_BOARD_COLUMN_ID, DEFAULT_TASK_LIST_COLOR_TOKEN, DEFAULT_TASK_LIST_ID,
            DEFAULT_TASK_LIST_NAME, IN_PROGRESS_BOARD_COLUMN_ID,
        },
        timer::{
            TimerCompletionReason, WorkTargetRef, WorkTargetType, DEFAULT_TASK_TIMER_SETTINGS_ID,
            DEFAULT_TASK_TIMER_TARGET_SECONDS, MAX_TASK_TIMER_TARGET_SECONDS,
            MIN_TASK_TIMER_TARGET_SECONDS,
        },
    },
};

pub const INITIAL_SCHEMA: &str = include_str!("../../migrations/0001_initial.sql");

const DATE_FORMAT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day]");
const BACKUP_FORMAT: &str = "tasktimer-sqlite-backup";
const BACKUP_FORMAT_VERSION: i64 = 1;
const BACKUP_DATABASE_FILE: &str = "tasktimer.sqlite3";
const BACKUP_MANIFEST_FILE: &str = "backup-manifest.json";
const JSON_EXPORT_FORMAT: &str = "tasktimer-json-export";
const CSV_EXPORT_FORMAT: &str = "tasktimer-csv-export";
const DATA_EXPORT_FORMAT_VERSION: i64 = 6;
const DATA_EXPORT_COMPATIBILITY: &str = "viewing-and-migration-aid-not-restore";
const CSV_EXPORT_MANIFEST_FILE: &str = "export-manifest.json";
const UI_PREF_LEFT_PANE_OPEN: &str = "left_pane_open";
const UI_PREF_LAST_VIEW: &str = "last_view";
const UI_PREF_LAST_TASK_LIST_ID: &str = "last_task_list_id";
const UI_PREF_CALENDAR_VIEW_MODE: &str = "calendar_view_mode";
const UI_VIEW_LIST: &str = "list";
const UI_VIEW_TODAY: &str = "today";
const UI_VIEW_FAVORITES: &str = "favorites";
const UI_VIEW_BOARD: &str = "board";
const UI_VIEW_CALENDAR: &str = "calendar";
const UI_VIEW_POMODORO: &str = "pomodoro";
const UI_VIEW_SETTINGS: &str = "settings";
const UI_VIEW_LEGACY_TASKS: &str = "tasks";
const CALENDAR_VIEW_WEEK: &str = "week";
const CALENDAR_VIEW_DAY: &str = "day";
const CALENDAR_VIEW_MONTH: &str = "month";
const REQUIRED_RESTORE_TABLES: &[&str] = &[
    "task_lists",
    "tasks",
    "subtasks",
    "timer_sessions",
    "timer_pauses",
    "notification_rules",
    "notification_delivery_attempts",
    "notification_preferences",
    "ui_preferences",
    "recurrence_rules",
];

#[derive(Debug, Clone, Copy)]
enum TaskRowScope {
    Normal,
    Archived,
}

impl TaskRowScope {
    fn as_query_value(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Archived => "archived",
        }
    }
}

pub struct SqliteDatabase {
    path: PathBuf,
    connection: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupManifestFile {
    format: String,
    format_version: i64,
    app_version: String,
    schema_version: i64,
    created_at: String,
    platform: String,
    database_file: String,
    integrity_check: String,
}

impl BackupManifestFile {
    fn to_record(&self) -> SqliteBackupManifestRecord {
        SqliteBackupManifestRecord {
            format: self.format.clone(),
            format_version: self.format_version,
            app_version: self.app_version.clone(),
            schema_version: self.schema_version,
            created_at: self.created_at.clone(),
            platform: self.platform.clone(),
            database_file: self.database_file.clone(),
            integrity_check: self.integrity_check.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportManifestFile {
    format: String,
    format_version: i64,
    app_version: String,
    created_at: String,
    platform: String,
    compatibility: String,
    contains_personal_data: bool,
}

impl ExportManifestFile {
    fn to_record(&self) -> DataExportManifestRecord {
        DataExportManifestRecord {
            format: self.format.clone(),
            format_version: self.format_version,
            app_version: self.app_version.clone(),
            created_at: self.created_at.clone(),
            platform: self.platform.clone(),
            compatibility: self.compatibility.clone(),
            contains_personal_data: self.contains_personal_data,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct JsonExportFile {
    manifest: ExportManifestFile,
    task_lists: Vec<ExportTaskListRow>,
    board_columns: Vec<ExportBoardColumnRow>,
    tags: Vec<ExportTagRow>,
    task_tags: Vec<ExportTaskTagRow>,
    tasks: Vec<ExportTaskRow>,
    subtasks: Vec<ExportSubtaskRow>,
    timer_sessions: Vec<ExportTimerSessionRow>,
    timer_pauses: Vec<ExportTimerPauseRow>,
    task_timer_settings: Vec<ExportTaskTimerSettingsRow>,
    pomodoro_settings: Vec<ExportPomodoroSettingsRow>,
    pomodoro_sessions: Vec<ExportPomodoroSessionRow>,
    notification_rules: Vec<ExportNotificationRuleRow>,
    notification_os_registrations: Vec<ExportNotificationOsRegistrationRow>,
    recurrence_rules: Vec<ExportRecurrenceRuleRow>,
}

#[derive(Debug, Clone)]
struct ExportDataset {
    task_lists: Vec<ExportTaskListRow>,
    board_columns: Vec<ExportBoardColumnRow>,
    tags: Vec<ExportTagRow>,
    task_tags: Vec<ExportTaskTagRow>,
    tasks: Vec<ExportTaskRow>,
    subtasks: Vec<ExportSubtaskRow>,
    timer_sessions: Vec<ExportTimerSessionRow>,
    timer_pauses: Vec<ExportTimerPauseRow>,
    task_timer_settings: Vec<ExportTaskTimerSettingsRow>,
    pomodoro_settings: Vec<ExportPomodoroSettingsRow>,
    pomodoro_sessions: Vec<ExportPomodoroSessionRow>,
    notification_rules: Vec<ExportNotificationRuleRow>,
    notification_os_registrations: Vec<ExportNotificationOsRegistrationRow>,
    recurrence_rules: Vec<ExportRecurrenceRuleRow>,
}

#[derive(Debug, Clone, Serialize)]
struct ExportTaskListRow {
    id: String,
    name: String,
    color_token: String,
    sort_order: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportBoardColumnRow {
    id: String,
    title: String,
    sort_order: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportTagRow {
    id: String,
    name: String,
    sort_order: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportTaskTagRow {
    task_id: String,
    tag_id: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportTaskRow {
    id: String,
    list_id: String,
    board_column_id: String,
    title: String,
    status: String,
    lifecycle_status: String,
    is_favorite: bool,
    color_token: Option<String>,
    planned_start_date: Option<String>,
    due_date: Option<String>,
    due_time: Option<String>,
    scheduled_start_date: Option<String>,
    scheduled_start_time: Option<String>,
    scheduled_end_date: Option<String>,
    scheduled_end_time: Option<String>,
    scheduled_is_all_day: bool,
    timer_target_seconds: Option<i64>,
    memo: String,
    sort_order: i64,
    completed_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportSubtaskRow {
    id: String,
    task_id: String,
    title: String,
    status: String,
    planned_start_date: Option<String>,
    due_date: Option<String>,
    due_time: Option<String>,
    scheduled_start_date: Option<String>,
    scheduled_start_time: Option<String>,
    scheduled_end_date: Option<String>,
    scheduled_end_time: Option<String>,
    scheduled_is_all_day: bool,
    timer_target_seconds: Option<i64>,
    memo: String,
    sort_order: i64,
    completed_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportTimerSessionRow {
    id: String,
    target_type: String,
    target_id: String,
    started_at: String,
    stopped_at: Option<String>,
    elapsed_seconds: Option<i64>,
    target_seconds: Option<i64>,
    completion_reason: Option<String>,
    completion_notified_at: Option<String>,
    created_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportTaskTimerSettingsRow {
    id: String,
    default_target_seconds: i64,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportTimerPauseRow {
    id: String,
    timer_session_id: String,
    paused_at: String,
    resumed_at: Option<String>,
    created_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportPomodoroSettingsRow {
    id: String,
    work_seconds: i64,
    short_break_seconds: i64,
    long_break_seconds: i64,
    cycles_until_long_break: i64,
    auto_start_break: bool,
    auto_start_next_work: bool,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportPomodoroSessionRow {
    id: String,
    scope: String,
    target_type: Option<String>,
    target_id: Option<String>,
    timer_session_id: Option<String>,
    phase: String,
    status: String,
    cycle_count: i64,
    phase_started_at: String,
    phase_duration_seconds: i64,
    paused_at: Option<String>,
    paused_total_seconds: i64,
    completed_at: Option<String>,
    cancelled_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportNotificationRuleRow {
    id: String,
    target_type: String,
    target_id: String,
    kind: String,
    notify_at: String,
    enabled: bool,
    registration_status: String,
    last_error: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportNotificationOsRegistrationRow {
    id: String,
    notification_rule_id: String,
    os_registration_id: Option<String>,
    registration_status: String,
    last_attempted_at: Option<String>,
    last_error: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportRecurrenceRuleRow {
    id: String,
    target_type: String,
    target_id: String,
    frequency: String,
    interval: i64,
    created_at: String,
    updated_at: String,
}

impl SqliteDatabase {
    pub fn open(app_handle: &AppHandle) -> RepositoryResult<Self> {
        let data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|error| format!("アプリデータディレクトリを取得できません: {error}"))?;
        Self::open_in_dir(data_dir)
    }

    pub fn open_in_dir(data_dir: PathBuf) -> RepositoryResult<Self> {
        fs::create_dir_all(&data_dir)
            .map_err(|error| format!("アプリデータディレクトリを作成できません: {error}"))?;

        let path = data_dir.join("tasktimer.sqlite3");
        let connection = Connection::open(&path)
            .map_err(|error| format!("SQLiteデータベースを開けません: {error}"))?;

        configure_connection(&connection)?;
        run_initial_migration(&connection)?;
        seed_default_preferences(&connection)?;

        Ok(Self {
            path,
            connection: Mutex::new(connection),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> RepositoryResult<T>,
    ) -> RepositoryResult<T> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "SQLite接続ロックの取得に失敗しました".to_string())?;
        operation(&connection)
    }

    fn with_transaction<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> RepositoryResult<T>,
    ) -> RepositoryResult<T> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "SQLite接続ロックの取得に失敗しました".to_string())?;
        let transaction = connection
            .transaction()
            .map_err(|error| format!("SQLiteトランザクションを開始できません: {error}"))?;
        let value = operation(&transaction)?;
        transaction
            .commit()
            .map_err(|error| format!("SQLiteトランザクションをコミットできません: {error}"))?;
        Ok(value)
    }
}

impl CalendarRepository for SqliteDatabase {
    fn list_calendar_items(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> RepositoryResult<Vec<WeekCalendarItem>> {
        self.list_calendar_items_for_scope(start_date, end_date, &TaskPageScope::Board, start_date)
    }

    fn list_calendar_items_for_scope(
        &self,
        start_date: &str,
        end_date: &str,
        scope: &TaskPageScope,
        today_date: &str,
    ) -> RepositoryResult<Vec<WeekCalendarItem>> {
        let start = parse_date(start_date, "開始日")?;
        let end = parse_date(end_date, "終了日")?;
        parse_date(today_date, "今日の日付")?;
        if end < start {
            return Err("カレンダー終了日は開始日以降にしてください".to_string());
        }
        if (end - start).whole_days() > 93 {
            return Err("カレンダー取得範囲は93日以内にしてください".to_string());
        }

        let start_text = format_date(start)?;
        let end_text = format_date(end)?;

        self.with_connection(|connection| {
            let mut items = Vec::new();
            collect_task_calendar_items(
                connection,
                &start_text,
                &end_text,
                scope,
                today_date,
                &mut items,
            )?;
            collect_subtask_calendar_items(
                connection,
                &start_text,
                &end_text,
                scope,
                today_date,
                &mut items,
            )?;
            collect_active_timer_calendar_item(
                connection,
                &start_text,
                &end_text,
                scope,
                today_date,
                &mut items,
            )?;
            items.sort_by(|a, b| {
                a.date
                    .cmp(&b.date)
                    .then_with(|| a.time.cmp(&b.time))
                    .then_with(|| a.title.cmp(&b.title))
            });
            Ok(items)
        })
    }

    fn list_week_calendar_items(
        &self,
        week_start_date: &str,
    ) -> RepositoryResult<Vec<WeekCalendarItem>> {
        let start = parse_date(week_start_date, "週開始日")?;
        let end = start + Duration::days(6);
        let start_text = format_date(start)?;
        let end_text = format_date(end)?;

        self.list_calendar_items(&start_text, &end_text)
    }
}

impl TimerRepository for SqliteDatabase {
    fn get_active_timer(&self) -> RepositoryResult<Option<ActiveTimer>> {
        self.with_connection(select_active_timer)
    }

    fn get_task_timer_settings(&self) -> RepositoryResult<TaskTimerSettingsRecord> {
        self.with_connection(select_task_timer_settings)
    }

    fn update_task_timer_settings(
        &self,
        input: TaskTimerSettingsUpdate,
    ) -> RepositoryResult<TaskTimerSettingsRecord> {
        self.with_transaction(|transaction| {
            let updated = transaction
                .execute(
                    "
                    UPDATE task_timer_settings
                    SET default_target_seconds = ?1,
                        updated_at = ?2
                    WHERE id = ?3
                    ",
                    params![
                        input.default_target_seconds,
                        input.now,
                        DEFAULT_TASK_TIMER_SETTINGS_ID
                    ],
                )
                .map_err(|error| format!("タスクタイマー設定を保存できません: {error}"))?;
            if updated != 1 {
                return Err("タスクタイマー設定を保存できませんでした".to_string());
            }
            select_task_timer_settings(transaction)
        })
    }

    fn sync_expired_task_countdown(
        &self,
        now: String,
    ) -> RepositoryResult<Option<TaskCountdownExpiry>> {
        self.with_transaction(|transaction| {
            let Some(active_timer) = select_active_timer(transaction)? else {
                return Ok(None);
            };
            let Some(target_seconds) = active_timer.target_seconds else {
                return Ok(None);
            };
            if active_timer.paused_at.is_some() {
                return Ok(None);
            }

            let paused_seconds = total_pause_seconds(transaction, &active_timer.id, &now)?;
            let (_, elapsed_seconds) =
                calculate_stop_values(&active_timer.started_at, &now, paused_seconds)?;
            if elapsed_seconds < target_seconds {
                return Ok(None);
            }

            let target_title = select_target_title(transaction, &active_timer.target)?;
            let updated = transaction
                .execute(
                    "
                    UPDATE timer_sessions
                    SET stopped_at = ?1,
                        elapsed_seconds = target_seconds,
                        completion_reason = 'countdown_expired'
                    WHERE id = ?2
                      AND stopped_at IS NULL
                      AND target_seconds IS NOT NULL
                      AND deleted_at IS NULL
                    ",
                    params![now, active_timer.id.as_str()],
                )
                .map_err(|error| format!("終了したタスクタイマーを保存できません: {error}"))?;
            if updated != 1 {
                return Ok(None);
            }

            Ok(Some(TaskCountdownExpiry {
                expired_timer: select_timer_by_id(transaction, &active_timer.id)?,
                target_title,
            }))
        })
    }

    fn mark_task_countdown_notification_sent(
        &self,
        timer_session_id: String,
        now: String,
    ) -> RepositoryResult<()> {
        self.with_transaction(|transaction| {
            let updated = transaction
                .execute(
                    "
                    UPDATE timer_sessions
                    SET completion_notified_at = COALESCE(completion_notified_at, ?1)
                    WHERE id = ?2
                      AND completion_reason = 'countdown_expired'
                      AND deleted_at IS NULL
                    ",
                    params![now, timer_session_id],
                )
                .map_err(|error| format!("タイマー通知済み時刻を保存できません: {error}"))?;
            if updated != 1 {
                return Err("通知対象のタイマー履歴が見つかりません".to_string());
            }
            Ok(())
        })
    }
}

impl PomodoroRepository for SqliteDatabase {
    fn get_pomodoro_settings(&self) -> RepositoryResult<PomodoroSettingsRecord> {
        self.with_connection(select_pomodoro_settings)
    }

    fn update_pomodoro_settings(
        &self,
        input: PomodoroSettingsUpdate,
    ) -> RepositoryResult<PomodoroSettingsRecord> {
        self.with_transaction(|transaction| {
            let updated = transaction
                .execute(
                    "
                    UPDATE pomodoro_settings
                    SET work_seconds = ?1,
                        short_break_seconds = ?2,
                        long_break_seconds = ?3,
                        cycles_until_long_break = ?4,
                        auto_start_break = ?5,
                        auto_start_next_work = ?6,
                        updated_at = ?7
                    WHERE id = ?8
                    ",
                    params![
                        input.work_seconds,
                        input.short_break_seconds,
                        input.long_break_seconds,
                        input.cycles_until_long_break,
                        input.auto_start_break,
                        input.auto_start_next_work,
                        input.now,
                        DEFAULT_POMODORO_SETTINGS_ID
                    ],
                )
                .map_err(|error| format!("ポモドーロ設定を保存できません: {error}"))?;
            if updated != 1 {
                return Err("ポモドーロ設定を保存できませんでした".to_string());
            }

            select_pomodoro_settings(transaction)
        })
    }

    fn get_active_pomodoro(&self) -> RepositoryResult<Option<ActivePomodoro>> {
        self.with_connection(select_active_pomodoro)
    }

    #[cfg(test)]
    fn start_legacy_task_linked_pomodoro(
        &self,
        target: WorkTargetRef,
        now: String,
    ) -> RepositoryResult<ActivePomodoro> {
        self.with_transaction(|transaction| {
            let status = find_target_status(transaction, &target)?.ok_or_else(|| {
                "ポモドーロ開始対象のタスクまたはサブタスクが存在しません".to_string()
            })?;
            assert_timer_startable(&status)?;
            ensure_no_active_timer(transaction)?;
            ensure_no_active_pomodoro(transaction)?;
            let settings = select_pomodoro_settings(transaction)?;

            let timer_id = Uuid::new_v4().to_string();
            transaction
                .execute(
                    "
                    INSERT INTO timer_sessions (
                      id, target_type, target_id, started_at, created_at
                    )
                    VALUES (?1, ?2, ?3, ?4, ?4)
                    ",
                    params![timer_id, target.target_type.as_str(), target.id, now],
                )
                .map_err(|error| format!("ポモドーロ用タイマーを開始できません: {error}"))?;

            let pomodoro_id = Uuid::new_v4().to_string();
            transaction
                .execute(
                    "
                    INSERT INTO pomodoro_sessions (
                      id, scope, target_type, target_id, timer_session_id, phase, status,
                      cycle_count, phase_started_at, phase_duration_seconds,
                      paused_total_seconds, created_at, updated_at
                    )
                    VALUES (?1, 'task_linked', ?2, ?3, ?4, 'work', 'running', 0, ?5, ?6, 0, ?5, ?5)
                    ",
                    params![
                        pomodoro_id,
                        target.target_type.as_str(),
                        target.id,
                        timer_id,
                        now,
                        settings.work_seconds
                    ],
                )
                .map_err(|error| format!("ポモドーロを開始できません: {error}"))?;

            mark_target_in_progress(transaction, &target, &now)?;
            select_active_pomodoro_by_id(transaction, &pomodoro_id)
        })
    }

    fn start_standalone_pomodoro(&self, now: String) -> RepositoryResult<ActivePomodoro> {
        self.with_transaction(|transaction| {
            ensure_no_active_timer(transaction)?;
            ensure_no_active_pomodoro(transaction)?;
            let settings = select_pomodoro_settings(transaction)?;
            let pomodoro_id = Uuid::new_v4().to_string();
            transaction
                .execute(
                    "
                    INSERT INTO pomodoro_sessions (
                      id, scope, target_type, target_id, timer_session_id, phase, status,
                      cycle_count, phase_started_at, phase_duration_seconds,
                      paused_total_seconds, created_at, updated_at
                    )
                    VALUES (?1, 'standalone', NULL, NULL, NULL, 'work', 'running',
                            0, ?2, ?3, 0, ?2, ?2)
                    ",
                    params![pomodoro_id, now, settings.work_seconds],
                )
                .map_err(|error| format!("ポモドーロを開始できません: {error}"))?;

            select_active_pomodoro_by_id(transaction, &pomodoro_id)
        })
    }

    fn pause_pomodoro(&self, now: String) -> RepositoryResult<ActivePomodoro> {
        self.with_transaction(|transaction| {
            let active = select_active_pomodoro(transaction)?
                .ok_or_else(|| "開始中のポモドーロがありません".to_string())?;
            if active.status == PomodoroStatus::Paused {
                return Ok(active);
            }

            match (&active.scope, &active.phase) {
                (PomodoroScope::TaskLinked, PomodoroPhase::Work) => {
                    let timer_id = active
                        .timer_session_id
                        .as_deref()
                        .ok_or_else(|| "作業フェーズにタイマー履歴がありません".to_string())?;
                    let active_timer = select_active_timer_by_id(transaction, timer_id)?;
                    if active_timer.paused_at.is_none() {
                        transaction
                            .execute(
                                "
                                INSERT INTO timer_pauses (
                                  id, timer_session_id, paused_at, created_at
                                )
                                VALUES (?1, ?2, ?3, ?3)
                                ",
                                params![Uuid::new_v4().to_string(), timer_id, now],
                            )
                            .map_err(|error| {
                                format!("ポモドーロ用タイマーを一時停止できません: {error}")
                            })?;
                    }
                    pause_pomodoro_for_timer(transaction, timer_id, &now)?;
                }
                (PomodoroScope::Standalone, PomodoroPhase::Work)
                | (_, PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak) => {
                    let updated = transaction
                        .execute(
                            "
                            UPDATE pomodoro_sessions
                            SET status = 'paused',
                                paused_at = ?1,
                                updated_at = ?1
                            WHERE id = ?2
                              AND status = 'running'
                              AND deleted_at IS NULL
                            ",
                            params![now, active.id.as_str()],
                        )
                        .map_err(|error| format!("ポモドーロを一時停止できません: {error}"))?;
                    if updated != 1 {
                        return Err("開始中のポモドーロを一時停止できませんでした".to_string());
                    }
                }
            }

            select_active_pomodoro_by_id(transaction, &active.id)
        })
    }

    fn resume_pomodoro(&self, now: String) -> RepositoryResult<ActivePomodoro> {
        self.with_transaction(|transaction| {
            let active = select_active_pomodoro(transaction)?
                .ok_or_else(|| "開始中のポモドーロがありません".to_string())?;
            if active.status == PomodoroStatus::Running {
                return Ok(active);
            }

            match (&active.scope, &active.phase) {
                (PomodoroScope::TaskLinked, PomodoroPhase::Work) => {
                    let timer_id = active
                        .timer_session_id
                        .as_deref()
                        .ok_or_else(|| "作業フェーズにタイマー履歴がありません".to_string())?;
                    let updated = transaction
                        .execute(
                            "
                            UPDATE timer_pauses
                            SET resumed_at = ?1
                            WHERE timer_session_id = ?2
                              AND resumed_at IS NULL
                              AND deleted_at IS NULL
                            ",
                            params![now, timer_id],
                        )
                        .map_err(|error| {
                            format!("ポモドーロ用タイマーを再開できません: {error}")
                        })?;
                    if updated != 1 {
                        return Err(
                            "一時停止中のポモドーロ用タイマーを再開できませんでした".to_string()
                        );
                    }
                    resume_pomodoro_for_timer(transaction, timer_id, &now)?;
                }
                (PomodoroScope::Standalone, PomodoroPhase::Work)
                | (_, PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak) => {
                    let paused_at = active.paused_at.as_deref().ok_or_else(|| {
                        "一時停止中のポモドーロを取得できませんでした".to_string()
                    })?;
                    let paused_seconds = calculate_duration_seconds(paused_at, &now)?;
                    let updated = transaction
                        .execute(
                            "
                            UPDATE pomodoro_sessions
                            SET status = 'running',
                                paused_at = NULL,
                                paused_total_seconds = paused_total_seconds + ?1,
                                updated_at = ?2
                            WHERE id = ?3
                              AND status = 'paused'
                              AND deleted_at IS NULL
                            ",
                            params![paused_seconds, now, active.id.as_str()],
                        )
                        .map_err(|error| format!("ポモドーロを再開できません: {error}"))?;
                    if updated != 1 {
                        return Err("一時停止中のポモドーロを再開できませんでした".to_string());
                    }
                }
            }

            select_active_pomodoro_by_id(transaction, &active.id)
        })
    }

    fn complete_pomodoro_work_phase(&self, now: String) -> RepositoryResult<ActivePomodoro> {
        self.with_transaction(|transaction| {
            let active = select_active_pomodoro(transaction)?
                .ok_or_else(|| "開始中のポモドーロがありません".to_string())?;
            if active.phase != PomodoroPhase::Work {
                return Err("作業フェーズのポモドーロだけ完了できます".to_string());
            }

            let paused_seconds = finish_pomodoro_work(transaction, &active, &now)?;
            let completed_cycle_count = active.cycle_count + 1;
            let updated = transaction
                .execute(
                    "
                    UPDATE pomodoro_sessions
                    SET status = 'completed',
                        cycle_count = ?1,
                        paused_at = NULL,
                        paused_total_seconds = ?2,
                        completed_at = ?3,
                        updated_at = ?3
                    WHERE id = ?4
                      AND status IN ('running', 'paused')
                      AND deleted_at IS NULL
                    ",
                    params![
                        completed_cycle_count,
                        paused_seconds,
                        now,
                        active.id.as_str()
                    ],
                )
                .map_err(|error| format!("ポモドーロ作業フェーズを完了できません: {error}"))?;
            if updated != 1 {
                return Err("開始中のポモドーロ作業フェーズを完了できませんでした".to_string());
            }

            select_pomodoro_session_by_id(transaction, &active.id)
        })
    }

    fn start_pomodoro_break(
        &self,
        pomodoro_session_id: String,
        now: String,
    ) -> RepositoryResult<ActivePomodoro> {
        self.with_transaction(|transaction| {
            let work_session = select_pomodoro_session_by_id(transaction, &pomodoro_session_id)?;
            if work_session.phase != PomodoroPhase::Work
                || work_session.status != PomodoroStatus::Completed
            {
                return Err(
                    "完了済みのポモドーロ作業フェーズから休憩を開始してください".to_string()
                );
            }
            ensure_pomodoro_target_available(transaction, &work_session)?;
            ensure_no_active_timer(transaction)?;
            ensure_no_active_pomodoro(transaction)?;

            let settings = select_pomodoro_settings(transaction)?;
            insert_pomodoro_break_phase(transaction, &work_session, &settings, &now)
        })
    }

    fn skip_pomodoro_break(
        &self,
        pomodoro_session_id: String,
        now: String,
    ) -> RepositoryResult<ActivePomodoro> {
        self.with_transaction(|transaction| {
            let source = select_pomodoro_session_by_id(transaction, &pomodoro_session_id)?;
            let next_cycle_count = source.cycle_count;

            match (&source.phase, &source.status) {
                (PomodoroPhase::Work, PomodoroStatus::Completed)
                | (PomodoroPhase::ShortBreak, PomodoroStatus::Running)
                | (PomodoroPhase::ShortBreak, PomodoroStatus::Paused)
                | (PomodoroPhase::ShortBreak, PomodoroStatus::Completed)
                | (PomodoroPhase::LongBreak, PomodoroStatus::Running)
                | (PomodoroPhase::LongBreak, PomodoroStatus::Paused)
                | (PomodoroPhase::LongBreak, PomodoroStatus::Completed) => {}
                _ => {
                    return Err(
                        "完了済み作業フェーズ、または休憩フェーズから次の作業を開始してください"
                            .to_string(),
                    );
                }
            }

            ensure_pomodoro_work_startable(transaction, &source)?;
            ensure_no_active_timer(transaction)?;
            ensure_no_active_pomodoro_except(transaction, &source.id)?;

            if source.phase != PomodoroPhase::Work && source.status != PomodoroStatus::Completed {
                let paused_seconds = accumulated_pomodoro_pause_seconds(&source, &now)?;
                let updated = transaction
                    .execute(
                        "
                        UPDATE pomodoro_sessions
                        SET status = 'cancelled',
                            paused_at = NULL,
                            paused_total_seconds = ?1,
                            cancelled_at = ?2,
                            updated_at = ?2
                        WHERE id = ?3
                          AND status IN ('running', 'paused')
                          AND deleted_at IS NULL
                        ",
                        params![paused_seconds, now, source.id.as_str()],
                    )
                    .map_err(|error| format!("ポモドーロ休憩をスキップできません: {error}"))?;
                if updated != 1 {
                    return Err("開始中のポモドーロ休憩をスキップできませんでした".to_string());
                }
            }

            let settings = select_pomodoro_settings(transaction)?;
            insert_pomodoro_work_phase(
                transaction,
                &source,
                next_cycle_count,
                &now,
                settings.work_seconds,
            )
        })
    }

    fn complete_pomodoro_break(&self, now: String) -> RepositoryResult<ActivePomodoro> {
        self.with_transaction(|transaction| {
            let active = select_active_pomodoro(transaction)?
                .ok_or_else(|| "開始中のポモドーロがありません".to_string())?;
            if active.phase == PomodoroPhase::Work {
                return Err("休憩フェーズのポモドーロだけ完了できます".to_string());
            }

            let paused_seconds = accumulated_pomodoro_pause_seconds(&active, &now)?;
            let updated = transaction
                .execute(
                    "
                    UPDATE pomodoro_sessions
                    SET status = 'completed',
                        paused_at = NULL,
                        paused_total_seconds = ?1,
                        completed_at = ?2,
                        updated_at = ?2
                    WHERE id = ?3
                      AND status IN ('running', 'paused')
                      AND deleted_at IS NULL
                    ",
                    params![paused_seconds, now, active.id.as_str()],
                )
                .map_err(|error| format!("ポモドーロ休憩を完了できません: {error}"))?;
            if updated != 1 {
                return Err("開始中のポモドーロ休憩を完了できませんでした".to_string());
            }

            select_pomodoro_session_by_id(transaction, &active.id)
        })
    }

    fn cancel_pomodoro(&self, now: String) -> RepositoryResult<ActivePomodoro> {
        self.with_transaction(|transaction| {
            let active = select_active_pomodoro(transaction)?
                .ok_or_else(|| "開始中のポモドーロがありません".to_string())?;
            let paused_seconds = match &active.phase {
                PomodoroPhase::Work => finish_pomodoro_work(transaction, &active, &now)?,
                PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak => {
                    accumulated_pomodoro_pause_seconds(&active, &now)?
                }
            };

            let updated = transaction
                .execute(
                    "
                    UPDATE pomodoro_sessions
                    SET status = 'cancelled',
                        paused_at = NULL,
                        paused_total_seconds = ?1,
                        cancelled_at = ?2,
                        updated_at = ?2
                    WHERE id = ?3
                      AND status IN ('running', 'paused')
                      AND deleted_at IS NULL
                    ",
                    params![paused_seconds, now, active.id.as_str()],
                )
                .map_err(|error| format!("ポモドーロをキャンセルできません: {error}"))?;
            if updated != 1 {
                return Err("開始中のポモドーロをキャンセルできませんでした".to_string());
            }

            select_pomodoro_session_by_id(transaction, &active.id)
        })
    }

    fn sync_expired_pomodoro(&self, now: String) -> RepositoryResult<Option<PomodoroExpiry>> {
        self.with_transaction(|transaction| {
            let Some(active) = select_active_pomodoro(transaction)? else {
                return Ok(None);
            };
            if active.status == PomodoroStatus::Paused {
                return Ok(None);
            }

            let phase_end_at = pomodoro_phase_end_at(&active)?;
            let now_at = parse_rfc3339_timestamp(&now, "現在時刻")?;
            let phase_end_timestamp = parse_rfc3339_timestamp(&phase_end_at, "ポモドーロ終了時刻")?;
            if now_at < phase_end_timestamp {
                return Ok(None);
            }

            let settings = select_pomodoro_settings(transaction)?;
            let notification_title = pomodoro_notification_title(transaction, &active)?;

            let expired_pomodoro = match active.phase {
                PomodoroPhase::Work => {
                    let paused_seconds = finish_pomodoro_work(transaction, &active, &phase_end_at)?;
                    let completed_cycle_count = active.cycle_count + 1;
                    let updated = transaction
                        .execute(
                            "
                            UPDATE pomodoro_sessions
                            SET status = 'completed',
                                cycle_count = ?1,
                                paused_at = NULL,
                                paused_total_seconds = ?2,
                                completed_at = ?3,
                                updated_at = ?4
                            WHERE id = ?5
                              AND status = 'running'
                              AND deleted_at IS NULL
                            ",
                            params![
                                completed_cycle_count,
                                paused_seconds,
                                phase_end_at.as_str(),
                                now.as_str(),
                                active.id.as_str()
                            ],
                        )
                        .map_err(|error| {
                            format!("期限到達したポモドーロ作業を完了できません: {error}")
                        })?;
                    if updated != 1 {
                        return Err("期限到達したポモドーロ作業を完了できませんでした".to_string());
                    }
                    select_pomodoro_session_by_id(transaction, &active.id)?
                }
                PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak => {
                    let updated = transaction
                        .execute(
                            "
                            UPDATE pomodoro_sessions
                            SET status = 'completed',
                                paused_at = NULL,
                                completed_at = ?1,
                                updated_at = ?2
                            WHERE id = ?3
                              AND status = 'running'
                              AND deleted_at IS NULL
                            ",
                            params![phase_end_at.as_str(), now.as_str(), active.id.as_str()],
                        )
                        .map_err(|error| {
                            format!("期限到達したポモドーロ休憩を完了できません: {error}")
                        })?;
                    if updated != 1 {
                        return Err("期限到達したポモドーロ休憩を完了できませんでした".to_string());
                    }
                    select_pomodoro_session_by_id(transaction, &active.id)?
                }
            };

            let active_pomodoro = match expired_pomodoro.phase {
                PomodoroPhase::Work if settings.auto_start_break => {
                    Some(insert_pomodoro_break_phase(
                        transaction,
                        &expired_pomodoro,
                        &settings,
                        &phase_end_at,
                    )?)
                }
                PomodoroPhase::ShortBreak | PomodoroPhase::LongBreak
                    if settings.auto_start_next_work =>
                {
                    if can_start_pomodoro_work_phase(transaction, &expired_pomodoro)? {
                        Some(insert_pomodoro_work_phase(
                            transaction,
                            &expired_pomodoro,
                            expired_pomodoro.cycle_count,
                            &phase_end_at,
                            settings.work_seconds,
                        )?)
                    } else {
                        None
                    }
                }
                _ => None,
            };

            Ok(Some(PomodoroExpiry {
                expired_pomodoro,
                active_pomodoro,
                notification_title,
            }))
        })
    }
}

impl TaskReadRepository for SqliteDatabase {
    fn list_task_page(&self, query: TaskPageQuery) -> RepositoryResult<TaskPageRecord> {
        if !(1..=200).contains(&query.limit) {
            return Err("ページ件数は1以上200以下で指定してください".to_string());
        }

        self.with_connection(|connection| select_task_page(connection, &query))
    }

    fn get_task_with_subtasks(&self, task_id: &str) -> RepositoryResult<TaskWithSubtasksRecord> {
        self.with_connection(|connection| {
            let task = select_existing_task_by_id(connection, task_id)?;
            if task.status == WorkStatus::Archived {
                return Err("アーカイブ済みタスクは詳細表示できません".to_string());
            }
            let subtasks = select_subtasks_for_task_ids(connection, &[task_id.to_string()])?;
            build_task_tree(vec![task], subtasks)
                .into_iter()
                .next()
                .ok_or_else(|| "タスク詳細を取得できません".to_string())
        })
    }

    fn search_work_items(
        &self,
        query: WorkItemSearchQuery,
    ) -> RepositoryResult<Vec<WorkItemSearchResultRecord>> {
        self.with_connection(|connection| select_work_item_search_results(connection, &query))
    }

    fn list_tasks_with_subtasks(
        &self,
        limit: i64,
    ) -> RepositoryResult<Vec<TaskWithSubtasksRecord>> {
        let limit = limit.clamp(1, 500);

        self.with_connection(|connection| {
            let tasks = select_task_list(connection, limit)?;
            if tasks.is_empty() {
                return Ok(Vec::new());
            }

            let subtasks = select_subtasks_for_task_list(connection, limit)?;
            Ok(build_task_tree(tasks, subtasks))
        })
    }

    fn list_task_lists(&self) -> RepositoryResult<Vec<TaskListRecord>> {
        self.with_connection(select_task_lists)
    }

    fn list_task_rows(
        &self,
        list_id: Option<&str>,
        limit: i64,
    ) -> RepositoryResult<Vec<TaskRowRecord>> {
        let limit = limit.clamp(1, 500);
        let list_id = normalize_optional_list_id(list_id);

        self.with_connection(|connection| {
            select_task_rows(connection, list_id.as_deref(), limit, TaskRowScope::Normal)
        })
    }

    fn list_archived_task_rows(&self, limit: i64) -> RepositoryResult<Vec<TaskRowRecord>> {
        let limit = limit.clamp(1, 500);

        self.with_connection(|connection| {
            select_task_rows(connection, None, limit, TaskRowScope::Archived)
        })
    }
}

impl BoardColumnRepository for SqliteDatabase {
    fn list_board_columns(&self) -> RepositoryResult<Vec<BoardColumnRecord>> {
        self.with_connection(select_board_columns)
    }

    fn create_board_column(&self, input: BoardColumnCreate) -> RepositoryResult<BoardColumnRecord> {
        self.with_transaction(|transaction| {
            ensure_unique_board_column_title(transaction, &input.title, None)?;
            let id = Uuid::new_v4().to_string();
            let sort_order: i64 = transaction
                .query_row(
                    "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM board_columns WHERE deleted_at IS NULL",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| format!("状態の並び順を取得できません: {error}"))?;
            transaction
                .execute(
                    "
                    INSERT INTO board_columns (id, title, sort_order, created_at, updated_at)
                    VALUES (?1, ?2, ?3, ?4, ?4)
                    ",
                    params![id, input.title, sort_order, input.now],
                )
                .map_err(|error| format!("状態を作成できません: {error}"))?;
            select_board_column_by_id(transaction, &id)
        })
    }

    fn update_board_column(
        &self,
        column_id: String,
        input: BoardColumnUpdate,
    ) -> RepositoryResult<BoardColumnRecord> {
        self.with_transaction(|transaction| {
            ensure_board_column_exists(transaction, &column_id)?;
            ensure_unique_board_column_title(transaction, &input.title, Some(&column_id))?;
            let updated = transaction
                .execute(
                    "
                    UPDATE board_columns
                    SET title = ?1, updated_at = ?2
                    WHERE id = ?3 AND deleted_at IS NULL
                    ",
                    params![input.title, input.now, column_id],
                )
                .map_err(|error| format!("状態名を更新できません: {error}"))?;
            if updated != 1 {
                return Err("更新する状態が存在しません".to_string());
            }
            select_board_column_by_id(transaction, &column_id)
        })
    }

    fn reorder_board_columns(
        &self,
        input: BoardColumnReorder,
    ) -> RepositoryResult<Vec<BoardColumnRecord>> {
        self.with_transaction(|transaction| {
            let existing_ids = select_active_board_column_ids(transaction)?;
            let requested_ids = input
                .ordered_column_ids
                .iter()
                .cloned()
                .collect::<HashSet<_>>();
            let existing_id_set = existing_ids.iter().cloned().collect::<HashSet<_>>();
            if requested_ids.len() != input.ordered_column_ids.len()
                || requested_ids != existing_id_set
            {
                return Err("状態の並び順には現在の状態を重複なくすべて指定してください".to_string());
            }

            for (sort_order, column_id) in input.ordered_column_ids.iter().enumerate() {
                transaction
                    .execute(
                        "UPDATE board_columns SET sort_order = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
                        params![sort_order as i64, input.now, column_id],
                    )
                    .map_err(|error| format!("状態の並び順を更新できません: {error}"))?;
            }
            select_board_columns(transaction)
        })
    }

    fn delete_board_column(
        &self,
        column_id: String,
        input: BoardColumnDelete,
    ) -> RepositoryResult<()> {
        self.with_transaction(|transaction| {
            let active_ids = select_active_board_column_ids(transaction)?;
            if active_ids.len() <= 1 {
                return Err("最後の状態は削除できません".to_string());
            }
            ensure_board_column_exists(transaction, &column_id)?;
            ensure_board_column_exists(transaction, &input.move_tasks_to_column_id)?;
            if column_id == input.move_tasks_to_column_id {
                return Err("削除する状態と移動先状態は別にしてください".to_string());
            }

            let active_status = legacy_active_status_for_column(&input.move_tasks_to_column_id);
            transaction
                .execute(
                    "
                    UPDATE tasks
                    SET board_column_id = ?1,
                        status = CASE
                          WHEN lifecycle_status = 'active' THEN ?2
                          ELSE status
                        END,
                        updated_at = ?3
                    WHERE board_column_id = ?4
                      AND deleted_at IS NULL
                    ",
                    params![
                        input.move_tasks_to_column_id,
                        active_status,
                        input.now,
                        column_id
                    ],
                )
                .map_err(|error| format!("削除する状態のタスクを移動できません: {error}"))?;
            transaction
                .execute(
                    "UPDATE board_columns SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
                    params![input.now, column_id],
                )
                .map_err(|error| format!("状態を削除できません: {error}"))?;
            normalize_board_column_sort_order(transaction, &input.now)
        })
    }

    fn move_task_to_board_column(
        &self,
        task_id: String,
        input: BoardTaskMove,
    ) -> RepositoryResult<()> {
        self.with_transaction(|transaction| {
            ensure_task_exists(transaction, &task_id)?;
            ensure_board_column_exists(transaction, &input.board_column_id)?;
            let active_status = legacy_active_status_for_column(&input.board_column_id);
            let updated = transaction
                .execute(
                    "
                    UPDATE tasks
                    SET board_column_id = ?1,
                        status = CASE
                          WHEN lifecycle_status = 'active' THEN ?2
                          ELSE status
                        END,
                        updated_at = ?3
                    WHERE id = ?4
                      AND deleted_at IS NULL
                    ",
                    params![input.board_column_id, active_status, input.now, task_id],
                )
                .map_err(|error| format!("タスクの状態を移動できません: {error}"))?;
            if updated != 1 {
                return Err("移動するタスクが存在しません".to_string());
            }
            Ok(())
        })
    }
}

impl SqliteBackupRepository for SqliteDatabase {
    fn create_sqlite_backup(
        &self,
        input: SqliteBackupCreate,
    ) -> RepositoryResult<SqliteBackupRecord> {
        let destination_dir = PathBuf::from(&input.destination_dir);
        let package_dir =
            destination_dir.join(format!("TaskTimer-backup-{}", backup_label(&input.now)));
        let backup_database_path = package_dir.join(BACKUP_DATABASE_FILE);
        let manifest_path = package_dir.join(BACKUP_MANIFEST_FILE);

        if package_dir.exists() {
            return Err("同名のバックアップフォルダが既に存在します".to_string());
        }
        fs::create_dir_all(&package_dir)
            .map_err(|error| format!("バックアップフォルダを作成できません: {error}"))?;

        if let Err(error) = self.with_connection(|connection| {
            verify_integrity_check(connection)?;
            create_sqlite_snapshot(connection, &backup_database_path)
        }) {
            let _ = fs::remove_dir_all(&package_dir);
            return Err(error);
        }

        if let Err(error) = validate_readonly_backup_database(&backup_database_path) {
            let _ = fs::remove_dir_all(&package_dir);
            return Err(error);
        }

        let manifest = BackupManifestFile {
            format: BACKUP_FORMAT.to_string(),
            format_version: BACKUP_FORMAT_VERSION,
            app_version: input.app_version,
            schema_version: input.schema_version,
            created_at: input.now,
            platform: input.platform,
            database_file: BACKUP_DATABASE_FILE.to_string(),
            integrity_check: "ok".to_string(),
        };
        if let Err(error) = write_backup_manifest(&manifest_path, &manifest) {
            let _ = fs::remove_dir_all(&package_dir);
            return Err(error);
        }

        Ok(SqliteBackupRecord {
            backup_dir: package_dir.to_string_lossy().to_string(),
            database_file: backup_database_path.to_string_lossy().to_string(),
            manifest_file: manifest_path.to_string_lossy().to_string(),
            manifest: manifest.to_record(),
        })
    }

    fn restore_sqlite_backup(
        &self,
        input: SqliteBackupRestore,
    ) -> RepositoryResult<SqliteRestoreRecord> {
        let backup_dir = PathBuf::from(input.backup_dir);
        let manifest = read_backup_manifest(&backup_dir)?;
        validate_backup_manifest(&manifest)?;

        let backup_database_path = backup_dir.join(BACKUP_DATABASE_FILE);
        validate_readonly_backup_database(&backup_database_path)?;

        let data_dir = self
            .path
            .parent()
            .ok_or_else(|| "アプリデータディレクトリを解決できません".to_string())?;
        let label = backup_label(&input.now);
        let restore_candidate_path = data_dir.join(format!("tasktimer-restore-{label}.sqlite3"));
        let previous_database_path =
            data_dir.join(format!("tasktimer-before-restore-{label}.sqlite3"));

        if restore_candidate_path.exists() || previous_database_path.exists() {
            return Err("復元用の一時ファイルが既に存在します".to_string());
        }

        fs::copy(&backup_database_path, &restore_candidate_path)
            .map_err(|error| format!("バックアップDBを一時領域へコピーできません: {error}"))?;

        if let Err(error) = validate_restore_candidate_database(&restore_candidate_path) {
            let _ = fs::remove_file(&restore_candidate_path);
            return Err(error);
        }

        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "SQLite接続ロックの取得に失敗しました".to_string())?;
        let placeholder = Connection::open_in_memory()
            .map_err(|error| format!("SQLite退避用接続を開けません: {error}"))?;
        let active_connection = std::mem::replace(&mut *connection, placeholder);
        drop(active_connection);

        if let Err(error) =
            replace_database_file(&self.path, &restore_candidate_path, &previous_database_path)
        {
            let _ = fs::remove_file(&restore_candidate_path);
            *connection = open_live_database(&self.path).map_err(|reopen_error| {
                format!("{error}; 現在のDBの再接続にも失敗しました: {reopen_error}")
            })?;
            return Err(error);
        }

        match open_live_database(&self.path) {
            Ok(reopened) => {
                *connection = reopened;
            }
            Err(error) => {
                restore_previous_database_file(&self.path, &previous_database_path);
                *connection = open_live_database(&self.path).map_err(|reopen_error| {
                    format!("{error}; 退避DBの再接続にも失敗しました: {reopen_error}")
                })?;
                return Err(error);
            }
        }

        Ok(SqliteRestoreRecord {
            backup_dir: backup_dir.to_string_lossy().to_string(),
            restored_at: input.now,
            previous_database_file: previous_database_path.to_string_lossy().to_string(),
            manifest: manifest.to_record(),
        })
    }
}

impl DataExportRepository for SqliteDatabase {
    fn create_json_export(&self, input: DataExportCreate) -> RepositoryResult<DataExportRecord> {
        let destination_dir = PathBuf::from(&input.destination_dir);
        fs::create_dir_all(&destination_dir)
            .map_err(|error| format!("エクスポート保存先を作成できません: {error}"))?;
        let export_path = destination_dir.join(format!(
            "TaskTimer-export-{}.json",
            backup_label(&input.now)
        ));
        if export_path.exists() {
            return Err("同名のJSONエクスポートファイルが既に存在します".to_string());
        }

        let manifest = create_export_manifest(JSON_EXPORT_FORMAT, &input);
        let dataset = self.with_transaction(|transaction| select_export_dataset(transaction))?;
        let export_file = JsonExportFile {
            manifest: manifest.clone(),
            task_lists: dataset.task_lists,
            board_columns: dataset.board_columns,
            tags: dataset.tags,
            task_tags: dataset.task_tags,
            tasks: dataset.tasks,
            subtasks: dataset.subtasks,
            timer_sessions: dataset.timer_sessions,
            timer_pauses: dataset.timer_pauses,
            task_timer_settings: dataset.task_timer_settings,
            pomodoro_settings: dataset.pomodoro_settings,
            pomodoro_sessions: dataset.pomodoro_sessions,
            notification_rules: dataset.notification_rules,
            notification_os_registrations: dataset.notification_os_registrations,
            recurrence_rules: dataset.recurrence_rules,
        };
        if let Err(error) = write_json_export_file(&export_path, &export_file) {
            let _ = fs::remove_file(&export_path);
            return Err(error);
        }

        Ok(DataExportRecord {
            export_path: export_path.to_string_lossy().to_string(),
            files: vec![export_path.to_string_lossy().to_string()],
            manifest: manifest.to_record(),
        })
    }

    fn create_csv_export(&self, input: DataExportCreate) -> RepositoryResult<DataExportRecord> {
        let destination_dir = PathBuf::from(&input.destination_dir);
        let export_dir =
            destination_dir.join(format!("TaskTimer-export-{}-csv", backup_label(&input.now)));
        if export_dir.exists() {
            return Err("同名のCSVエクスポートフォルダが既に存在します".to_string());
        }
        fs::create_dir_all(&export_dir)
            .map_err(|error| format!("CSVエクスポートフォルダを作成できません: {error}"))?;

        let manifest = create_export_manifest(CSV_EXPORT_FORMAT, &input);
        let dataset = self.with_transaction(|transaction| select_export_dataset(transaction))?;
        let result = write_csv_export_files(&export_dir, &manifest, dataset);
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&export_dir);
            return Err(error);
        }

        let files = csv_export_file_names()
            .into_iter()
            .map(|name| export_dir.join(name).to_string_lossy().to_string())
            .collect();

        Ok(DataExportRecord {
            export_path: export_dir.to_string_lossy().to_string(),
            files,
            manifest: manifest.to_record(),
        })
    }
}

impl TaskTimerCommandRepository for SqliteDatabase {
    fn create_task(&self, input: WorkItemCreate) -> RepositoryResult<TaskRecord> {
        self.with_transaction(|transaction| insert_task(transaction, input))
    }

    fn create_task_in_board_column(
        &self,
        input: WorkItemCreate,
        board_column_id: String,
    ) -> RepositoryResult<TaskRecord> {
        self.with_transaction(|transaction| {
            insert_task_in_board_column(transaction, input, Some(&board_column_id))
        })
    }

    fn create_scheduled_task(
        &self,
        input: WorkItemCreate,
        schedule: WorkScheduleUpdate,
    ) -> RepositoryResult<TaskRecord> {
        self.with_transaction(|transaction| {
            let task = insert_task(transaction, input)?;
            update_work_schedule_row(
                transaction,
                &WorkTargetRef {
                    target_type: WorkTargetType::Task,
                    id: task.id.clone(),
                },
                schedule,
            )?;
            select_existing_task_by_id(transaction, &task.id)
        })
    }

    fn create_subtask(
        &self,
        task_id: String,
        input: WorkItemCreate,
    ) -> RepositoryResult<SubtaskRecord> {
        self.with_transaction(|transaction| {
            ensure_task_exists(transaction, &task_id)?;
            insert_subtask(transaction, &task_id, input)
        })
    }

    fn update_task(&self, task_id: String, input: WorkItemUpdate) -> RepositoryResult<TaskRecord> {
        self.with_transaction(|transaction| update_task_detail(transaction, &task_id, input))
    }

    fn update_subtask(
        &self,
        subtask_id: String,
        input: WorkItemUpdate,
    ) -> RepositoryResult<SubtaskRecord> {
        self.with_transaction(|transaction| update_subtask_detail(transaction, &subtask_id, input))
    }

    fn resize_scheduled_work_item(
        &self,
        target: WorkTargetRef,
        input: WorkScheduleUpdate,
    ) -> RepositoryResult<()> {
        self.with_transaction(|transaction| update_work_schedule_row(transaction, &target, input))
    }

    fn move_scheduled_work_item(
        &self,
        target: WorkTargetRef,
        input: WorkScheduleMove,
    ) -> RepositoryResult<()> {
        self.with_transaction(|transaction| {
            let current = select_work_schedule_for_move(transaction, &target)?;
            let moved = current.move_to(&input.destination)?;
            update_work_schedule_row(
                transaction,
                &target,
                WorkScheduleUpdate {
                    schedule: moved,
                    now: input.now,
                },
            )
        })
    }

    fn start_timer(&self, target: WorkTargetRef, now: String) -> RepositoryResult<ActiveTimer> {
        self.with_transaction(|transaction| {
            let status = find_target_status(transaction, &target)?.ok_or_else(|| {
                "タイマー開始対象のタスクまたはサブタスクが存在しません".to_string()
            })?;
            assert_timer_startable(&status)?;
            ensure_no_active_timer(transaction)?;
            ensure_no_active_pomodoro(transaction)?;
            let target_seconds = select_effective_task_timer_seconds(transaction, &target)?;

            let timer_id = Uuid::new_v4().to_string();
            transaction
                .execute(
                    "
                    INSERT INTO timer_sessions (
                      id, target_type, target_id, started_at, target_seconds, created_at
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?4)
                    ",
                    params![
                        timer_id,
                        target.target_type.as_str(),
                        target.id,
                        now,
                        target_seconds
                    ],
                )
                .map_err(|error| format!("タイマーを開始できません: {error}"))?;

            mark_target_in_progress(transaction, &target, &now)?;
            select_active_timer_by_id(transaction, &timer_id)
        })
    }

    fn pause_active_timer(&self, now: String) -> RepositoryResult<ActiveTimer> {
        self.with_transaction(|transaction| {
            let active_timer = select_active_timer(transaction)?
                .ok_or_else(|| "開始中のタイマーがありません".to_string())?;
            if active_timer.paused_at.is_some() {
                return Ok(active_timer);
            }

            transaction
                .execute(
                    "
                    INSERT INTO timer_pauses (
                      id, timer_session_id, paused_at, created_at
                    )
                    VALUES (?1, ?2, ?3, ?3)
                    ",
                    params![Uuid::new_v4().to_string(), active_timer.id.as_str(), now],
                )
                .map_err(|error| format!("タイマーを一時停止できません: {error}"))?;

            pause_pomodoro_for_timer(transaction, &active_timer.id, &now)?;
            select_active_timer_by_id(transaction, &active_timer.id)
        })
    }

    fn resume_active_timer(&self, now: String) -> RepositoryResult<ActiveTimer> {
        self.with_transaction(|transaction| {
            let active_timer = select_active_timer(transaction)?
                .ok_or_else(|| "開始中のタイマーがありません".to_string())?;
            if active_timer.paused_at.is_none() {
                return Ok(active_timer);
            }

            let updated = transaction
                .execute(
                    "
                    UPDATE timer_pauses
                    SET resumed_at = ?1
                    WHERE timer_session_id = ?2
                      AND resumed_at IS NULL
                      AND deleted_at IS NULL
                    ",
                    params![now, active_timer.id.as_str()],
                )
                .map_err(|error| format!("タイマーを再開できません: {error}"))?;
            if updated != 1 {
                return Err("一時停止中のタイマーを再開できませんでした".to_string());
            }

            resume_pomodoro_for_timer(transaction, &active_timer.id, &now)?;
            select_active_timer_by_id(transaction, &active_timer.id)
        })
    }

    fn stop_active_timer(&self, now: String) -> RepositoryResult<ActiveTimer> {
        self.with_transaction(|transaction| {
            let active_timer = select_active_timer(transaction)?
                .ok_or_else(|| "開始中のタイマーがありません".to_string())?;
            close_open_pause_for_timer(transaction, &active_timer.id, &now)?;
            let paused_seconds = total_pause_seconds(transaction, &active_timer.id, &now)?;
            let (stopped_at, elapsed_seconds) =
                calculate_stop_values(&active_timer.started_at, &now, paused_seconds)?;

            let updated = transaction
                .execute(
                    "
                    UPDATE timer_sessions
                    SET stopped_at = ?1,
                        elapsed_seconds = ?2,
                        completion_reason = 'manual'
                    WHERE id = ?3
                      AND stopped_at IS NULL
                      AND deleted_at IS NULL
                    ",
                    params![stopped_at, elapsed_seconds, active_timer.id],
                )
                .map_err(|error| format!("タイマーを停止できません: {error}"))?;
            if updated != 1 {
                return Err("開始中のタイマーを停止できませんでした".to_string());
            }

            resume_pomodoro_for_timer(transaction, &active_timer.id, &now)?;
            cancel_pomodoro_for_timer(transaction, &active_timer.id, &now)?;
            select_timer_by_id(transaction, &active_timer.id)
        })
    }

    fn complete_task(
        &self,
        task_id: String,
        allow_incomplete_subtasks: bool,
        now: String,
    ) -> RepositoryResult<TaskRecord> {
        self.with_transaction(|transaction| {
            let task = select_existing_task_by_id(transaction, &task_id)?;
            assert_completable(&task.status)?;
            if task.status == WorkStatus::Done {
                return Ok(task);
            }

            let incomplete_subtasks = count_incomplete_subtasks(transaction, &task_id)?;
            if incomplete_subtasks > 0 && !allow_incomplete_subtasks {
                return Err(format!(
                    "未完了のサブタスクが{incomplete_subtasks}件あります。確認後に完了してください"
                ));
            }

            transaction
                .execute(
                    "
                    UPDATE tasks
                    SET status = 'done',
                        lifecycle_status = 'done',
                        completed_at = COALESCE(completed_at, ?1),
                        updated_at = ?1
                    WHERE id = ?2
                      AND deleted_at IS NULL
                    ",
                    params![now, task_id],
                )
                .map_err(|error| format!("タスクを完了できません: {error}"))?;

            select_existing_task_by_id(transaction, &task_id)
        })
    }

    fn update_task_status(
        &self,
        task_id: String,
        input: TaskStatusUpdate,
    ) -> RepositoryResult<TaskRecord> {
        self.with_transaction(|transaction| {
            let task = select_existing_task_by_id(transaction, &task_id)?;
            if task.status == WorkStatus::Archived {
                return Err("アーカイブ済みタスクの状態はかんばんから変更できません".to_string());
            }
            if task.status == input.status {
                return Ok(task);
            }

            if input.status == WorkStatus::Done {
                assert_completable(&task.status)?;
                let incomplete_subtasks = count_incomplete_subtasks(transaction, &task_id)?;
                if incomplete_subtasks > 0 && !input.allow_incomplete_subtasks {
                    return Err(format!(
                        "未完了のサブタスクが{incomplete_subtasks}件あります。確認後に完了してください"
                    ));
                }
            }

            let board_column_id = match input.status {
                WorkStatus::Todo => Some(select_preferred_active_board_column_id(
                    transaction,
                    DEFAULT_BOARD_COLUMN_ID,
                )?),
                WorkStatus::InProgress => Some(select_preferred_active_board_column_id(
                    transaction,
                    IN_PROGRESS_BOARD_COLUMN_ID,
                )?),
                WorkStatus::Done | WorkStatus::Archived => None,
            };
            let lifecycle_status = if input.status == WorkStatus::Done {
                "done"
            } else {
                "active"
            };
            transaction
                .execute(
                    "
                    UPDATE tasks
                    SET status = ?1,
                        lifecycle_status = ?2,
                        board_column_id = COALESCE(?3, board_column_id),
                        completed_at = CASE
                          WHEN ?1 = 'done' THEN COALESCE(completed_at, ?4)
                          ELSE NULL
                        END,
                        updated_at = ?4
                    WHERE id = ?5
                      AND deleted_at IS NULL
                    ",
                    params![
                        input.status.as_str(),
                        lifecycle_status,
                        board_column_id.as_deref(),
                        input.now,
                        task_id
                    ],
                )
                .map_err(|error| format!("タスク状態を更新できません: {error}"))?;

            select_existing_task_by_id(transaction, &task_id)
        })
    }

    fn reopen_task(&self, task_id: String, now: String) -> RepositoryResult<TaskRecord> {
        self.with_transaction(|transaction| {
            let task = select_existing_task_by_id(transaction, &task_id)?;
            if task.status == WorkStatus::Archived {
                return Err("アーカイブ済みタスクは未完了に戻せません".to_string());
            }
            if task.status != WorkStatus::Done {
                return Ok(task);
            }

            transaction
                .execute(
                    "
                    UPDATE tasks
                    SET status = CASE
                          WHEN board_column_id = ?1 THEN 'todo'
                          ELSE 'in_progress'
                        END,
                        lifecycle_status = 'active',
                        completed_at = NULL,
                        updated_at = ?2
                    WHERE id = ?3
                      AND deleted_at IS NULL
                    ",
                    params![DEFAULT_BOARD_COLUMN_ID, now, task_id],
                )
                .map_err(|error| format!("タスクを未完了に戻せません: {error}"))?;

            select_existing_task_by_id(transaction, &task_id)
        })
    }

    fn complete_subtask(&self, subtask_id: String, now: String) -> RepositoryResult<SubtaskRecord> {
        self.with_transaction(|transaction| {
            let subtask = select_existing_subtask_by_id(transaction, &subtask_id)?;
            assert_completable(&subtask.status)?;
            if subtask.status == WorkStatus::Done {
                return Ok(subtask);
            }

            transaction
                .execute(
                    "
                    UPDATE subtasks
                    SET status = 'done',
                        completed_at = COALESCE(completed_at, ?1),
                        updated_at = ?1
                    WHERE id = ?2
                      AND deleted_at IS NULL
                    ",
                    params![now, subtask_id],
                )
                .map_err(|error| format!("サブタスクを完了できません: {error}"))?;

            select_existing_subtask_by_id(transaction, &subtask_id)
        })
    }

    fn reopen_subtask(&self, subtask_id: String, now: String) -> RepositoryResult<SubtaskRecord> {
        self.with_transaction(|transaction| {
            let subtask = select_existing_subtask_by_id(transaction, &subtask_id)?;
            if subtask.status == WorkStatus::Archived {
                return Err("アーカイブ済みサブタスクは未完了に戻せません".to_string());
            }
            if subtask.status != WorkStatus::Done {
                return Ok(subtask);
            }

            transaction
                .execute(
                    "
                    UPDATE subtasks
                    SET status = 'todo',
                        completed_at = NULL,
                        updated_at = ?1
                    WHERE id = ?2
                      AND deleted_at IS NULL
                    ",
                    params![now, subtask_id],
                )
                .map_err(|error| format!("サブタスクを未完了に戻せません: {error}"))?;

            select_existing_subtask_by_id(transaction, &subtask_id)
        })
    }

    fn toggle_task_favorite(
        &self,
        task_id: String,
        is_favorite: bool,
        now: String,
    ) -> RepositoryResult<TaskRecord> {
        self.with_transaction(|transaction| {
            let updated = transaction
                .execute(
                    "
                    UPDATE tasks
                    SET is_favorite = ?1,
                        updated_at = ?2
                    WHERE id = ?3
                      AND deleted_at IS NULL
                    ",
                    params![is_favorite, now, task_id],
                )
                .map_err(|error| format!("お気に入り状態を更新できません: {error}"))?;

            if updated != 1 {
                return Err("お気に入り更新対象のタスクが存在しません".to_string());
            }

            select_existing_task_by_id(transaction, &task_id)
        })
    }

    fn archive_task(&self, task_id: String, now: String) -> RepositoryResult<TaskRecord> {
        self.with_transaction(|transaction| {
            let task = select_existing_task_by_id(transaction, &task_id)?;
            if task.status == WorkStatus::Archived {
                return Ok(task);
            }
            ensure_no_active_timer_for_task_graph(transaction, &task_id)?;
            ensure_no_active_pomodoro_for_task_graph(transaction, &task_id)?;

            transaction
                .execute(
                    "
                    UPDATE tasks
                    SET status = 'archived',
                        lifecycle_status = 'archived',
                        updated_at = ?1
                    WHERE id = ?2
                      AND deleted_at IS NULL
                    ",
                    params![now, task_id],
                )
                .map_err(|error| format!("タスクをアーカイブできません: {error}"))?;

            select_existing_task_by_id(transaction, &task_id)
        })
    }

    fn restore_archived_task(&self, task_id: String, now: String) -> RepositoryResult<TaskRecord> {
        self.with_transaction(|transaction| {
            let task = select_existing_task_by_id(transaction, &task_id)?;
            if task.status != WorkStatus::Archived {
                return Ok(task);
            }

            transaction
                .execute(
                    "
                    UPDATE tasks
                    SET status = CASE
                          WHEN completed_at IS NULL AND board_column_id = ?1 THEN 'todo'
                          WHEN completed_at IS NULL THEN 'in_progress'
                          ELSE 'done'
                        END,
                        lifecycle_status = CASE
                          WHEN completed_at IS NULL THEN 'active'
                          ELSE 'done'
                        END,
                        updated_at = ?2
                    WHERE id = ?3
                      AND deleted_at IS NULL
                    ",
                    params![DEFAULT_BOARD_COLUMN_ID, now, task_id],
                )
                .map_err(|error| format!("アーカイブ済みタスクを復元できません: {error}"))?;

            select_existing_task_by_id(transaction, &task_id)
        })
    }

    fn delete_task(&self, task_id: String, now: String) -> RepositoryResult<()> {
        self.with_transaction(|transaction| {
            ensure_task_exists(transaction, &task_id)?;
            soft_delete_task_graph(transaction, &task_id, &now)
        })
    }

    fn delete_subtask(&self, subtask_id: String, now: String) -> RepositoryResult<()> {
        self.with_transaction(|transaction| {
            ensure_subtask_exists(transaction, &subtask_id)?;
            soft_delete_subtask_graph(transaction, &subtask_id, &now)
        })
    }
}

impl TaskListCommandRepository for SqliteDatabase {
    fn create_task_list(&self, input: TaskListCreate) -> RepositoryResult<TaskListRecord> {
        self.with_transaction(|transaction| insert_task_list(transaction, input))
    }

    fn update_task_list(
        &self,
        list_id: String,
        input: TaskListUpdate,
    ) -> RepositoryResult<TaskListRecord> {
        self.with_transaction(|transaction| update_task_list_detail(transaction, &list_id, input))
    }

    fn delete_task_list(&self, list_id: String, now: String) -> RepositoryResult<()> {
        self.with_transaction(|transaction| soft_delete_task_list(transaction, &list_id, &now))
    }
}

impl TagRepository for SqliteDatabase {
    fn list_tags(&self) -> RepositoryResult<Vec<TagRecord>> {
        self.with_connection(select_tags)
    }

    fn create_tag(&self, input: TagCreate) -> RepositoryResult<TagRecord> {
        self.with_transaction(|transaction| insert_tag(transaction, input))
    }

    fn update_tag(&self, tag_id: String, input: TagUpdate) -> RepositoryResult<TagRecord> {
        self.with_transaction(|transaction| update_tag_detail(transaction, &tag_id, input))
    }

    fn delete_tag(&self, tag_id: String, now: String) -> RepositoryResult<()> {
        self.with_transaction(|transaction| soft_delete_tag(transaction, &tag_id, &now))
    }

    fn attach_tag_to_task(
        &self,
        task_id: String,
        tag_id: String,
        now: String,
    ) -> RepositoryResult<TaskTagRecord> {
        self.with_transaction(|transaction| {
            attach_tag_to_task_row(transaction, &task_id, &tag_id, &now)
        })
    }

    fn detach_tag_from_task(
        &self,
        task_id: String,
        tag_id: String,
        now: String,
    ) -> RepositoryResult<()> {
        self.with_transaction(|transaction| {
            detach_tag_from_task_row(transaction, &task_id, &tag_id, &now)
        })
    }
}

impl NotificationPreferenceRepository for SqliteDatabase {
    fn get_notification_display_mode(&self) -> RepositoryResult<NotificationDisplayMode> {
        self.with_connection(|connection| {
            let display_mode: String = connection
                .query_row(
                    "
                    SELECT display_mode
                    FROM notification_preferences
                    WHERE id = 'default'
                    ",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| format!("通知表示設定を取得できません: {error}"))?;

            NotificationDisplayMode::from_db(&display_mode)
        })
    }

    fn get_notifications_enabled(&self) -> RepositoryResult<bool> {
        self.with_connection(|connection| {
            connection
                .query_row(
                    "
                    SELECT notifications_enabled
                    FROM notification_preferences
                    WHERE id = 'default'
                    ",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .map(|value| value != 0)
                .map_err(|error| format!("通知有効設定を取得できません: {error}"))
        })
    }
}

impl NotificationScheduleRepository for SqliteDatabase {
    fn get_next_pending_notification(
        &self,
        now: &str,
    ) -> RepositoryResult<Option<NextNotificationSchedule>> {
        self.with_connection(|connection| select_next_pending_notification(connection, now))
    }
}

impl NotificationHistoryRepository for SqliteDatabase {
    fn list_notification_failure_history(
        &self,
        limit: i64,
    ) -> RepositoryResult<Vec<NotificationDeliveryAttemptRecord>> {
        let limit = limit.clamp(1, 100);
        self.with_connection(|connection| select_notification_failure_history(connection, limit))
    }
}

impl NotificationOsRegistrationRepository for SqliteDatabase {
    fn list_notification_os_registration_jobs(
        &self,
        now: &str,
        limit: i64,
    ) -> RepositoryResult<Vec<NotificationOsRegistrationJob>> {
        let limit = limit.clamp(1, 100);
        self.with_connection(|connection| {
            select_notification_os_registration_jobs(connection, now, limit)
        })
    }

    fn mark_notification_os_registration_registered(
        &self,
        registration_id: String,
        os_registration_id: String,
        now: String,
    ) -> RepositoryResult<()> {
        self.with_transaction(|transaction| {
            let updated = transaction
                .execute(
                    "
                    UPDATE notification_os_registrations
                    SET os_registration_id = ?1,
                        registration_status = 'registered',
                        last_attempted_at = ?2,
                        last_error = NULL,
                        updated_at = ?2
                    WHERE id = ?3
                      AND deleted_at IS NULL
                    ",
                    params![os_registration_id, now, registration_id],
                )
                .map_err(|error| format!("通知OS登録成功状態を保存できません: {error}"))?;
            if updated == 1 {
                Ok(())
            } else {
                Err("通知OS登録状態が存在しません".to_string())
            }
        })
    }

    fn mark_notification_os_registration_failed(
        &self,
        registration_id: String,
        error: &str,
        now: String,
    ) -> RepositoryResult<()> {
        self.with_transaction(|transaction| {
            let updated = transaction
                .execute(
                    "
                    UPDATE notification_os_registrations
                    SET registration_status = 'failed',
                        last_attempted_at = ?1,
                        last_error = ?2,
                        updated_at = ?1
                    WHERE id = ?3
                      AND deleted_at IS NULL
                    ",
                    params![now, truncate_error(error), registration_id],
                )
                .map_err(|error| format!("通知OS登録失敗状態を保存できません: {error}"))?;
            if updated == 1 {
                Ok(())
            } else {
                Err("通知OS登録状態が存在しません".to_string())
            }
        })
    }

    fn mark_notification_os_registration_cancelled(
        &self,
        registration_id: String,
        now: String,
    ) -> RepositoryResult<()> {
        self.with_transaction(|transaction| {
            let updated = transaction
                .execute(
                    "
                    UPDATE notification_os_registrations
                    SET os_registration_id = NULL,
                        registration_status = 'disabled',
                        last_attempted_at = ?1,
                        last_error = NULL,
                        deleted_at = ?1,
                        updated_at = ?1
                    WHERE id = ?2
                      AND deleted_at IS NULL
                    ",
                    params![now, registration_id],
                )
                .map_err(|error| format!("通知OS登録解除状態を保存できません: {error}"))?;
            if updated == 1 {
                Ok(())
            } else {
                Err("通知OS登録状態が存在しません".to_string())
            }
        })
    }
}

impl NativeNotificationOsRegistrationRepository for SqliteDatabase {
    fn list_native_notification_os_registration_jobs(
        &self,
        now: &str,
        limit: i64,
    ) -> RepositoryResult<Vec<NativeNotificationOsRegistrationJob>> {
        let limit = limit.clamp(1, 100);
        self.with_connection(|connection| {
            select_native_notification_os_registration_jobs(connection, now, limit)
        })
    }
}

impl NotificationCommandRepository for SqliteDatabase {
    fn update_notification_display_mode(
        &self,
        display_mode: NotificationDisplayMode,
        now: String,
    ) -> RepositoryResult<NotificationDisplayMode> {
        self.with_transaction(|transaction| {
            transaction
                .execute(
                    "
                    UPDATE notification_preferences
                    SET display_mode = ?1,
                        updated_at = ?2
                    WHERE id = 'default'
                    ",
                    params![display_mode.as_str(), now],
                )
                .map_err(|error| format!("通知表示設定を保存できません: {error}"))?;

            mark_future_notification_os_registrations_pending(transaction, &now)?;

            let display_mode: String = transaction
                .query_row(
                    "
                    SELECT display_mode
                    FROM notification_preferences
                    WHERE id = 'default'
                    ",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| format!("通知表示設定を取得できません: {error}"))?;
            NotificationDisplayMode::from_db(&display_mode)
        })
    }

    fn update_notifications_enabled(&self, enabled: bool, now: String) -> RepositoryResult<bool> {
        self.with_transaction(|transaction| {
            let updated = transaction
                .execute(
                    "
                    UPDATE notification_preferences
                    SET notifications_enabled = ?1,
                        updated_at = ?2
                    WHERE id = 'default'
                    ",
                    params![enabled, now],
                )
                .map_err(|error| format!("通知有効設定を保存できません: {error}"))?;
            if updated != 1 {
                return Err("通知有効設定を保存できませんでした".to_string());
            }

            if enabled {
                reactivate_future_notification_os_registrations(transaction, &now)?;
            } else {
                disable_future_notification_os_registrations(transaction, &now)?;
            }

            let enabled: i64 = transaction
                .query_row(
                    "
                    SELECT notifications_enabled
                    FROM notification_preferences
                    WHERE id = 'default'
                    ",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| format!("通知有効設定を取得できません: {error}"))?;
            Ok(enabled != 0)
        })
    }

    fn list_due_notification_jobs(
        &self,
        now: &str,
        limit: i64,
    ) -> RepositoryResult<Vec<NotificationJob>> {
        let limit = limit.clamp(1, 100);
        self.with_connection(|connection| select_due_notification_jobs(connection, now, limit))
    }

    fn mark_notification_registered(
        &self,
        job: &NotificationJob,
        now: &str,
    ) -> RepositoryResult<()> {
        self.with_transaction(|transaction| {
            transaction
                .execute(
                    "
                    UPDATE notification_rules
                    SET registration_status = 'registered',
                        last_error = NULL,
                        updated_at = ?1
                    WHERE id = ?2
                      AND deleted_at IS NULL
                    ",
                    params![now, job.id.as_str()],
                )
                .map_err(|error| format!("通知登録成功状態を保存できません: {error}"))?;

            insert_notification_delivery_attempt(
                transaction,
                job,
                NotificationDeliveryResult::Success,
                None,
                now,
            )
        })
    }

    fn mark_notification_failed(
        &self,
        job: &NotificationJob,
        error: &str,
        now: &str,
    ) -> RepositoryResult<()> {
        self.with_transaction(|transaction| {
            transaction
                .execute(
                    "
                    UPDATE notification_rules
                    SET registration_status = 'failed',
                        last_error = ?1,
                        updated_at = ?2
                    WHERE id = ?3
                      AND deleted_at IS NULL
                    ",
                    params![truncate_error(error), now, job.id.as_str()],
                )
                .map_err(|error| format!("通知登録失敗状態を保存できません: {error}"))?;

            insert_notification_delivery_attempt(
                transaction,
                job,
                NotificationDeliveryResult::Failed,
                Some(error),
                now,
            )
        })
    }
}

impl UiPreferenceRepository for SqliteDatabase {
    fn get_ui_preferences(&self) -> RepositoryResult<UiPreferencesRecord> {
        self.with_connection(select_ui_preferences)
    }

    fn update_ui_preferences(
        &self,
        input: UiPreferencesUpdate,
    ) -> RepositoryResult<UiPreferencesRecord> {
        self.with_transaction(|transaction| {
            upsert_ui_preference(
                transaction,
                UI_PREF_LEFT_PANE_OPEN,
                if input.left_pane_open {
                    "true"
                } else {
                    "false"
                },
                &input.now,
            )?;
            upsert_ui_preference(transaction, UI_PREF_LAST_VIEW, &input.last_view, &input.now)?;
            upsert_ui_preference(
                transaction,
                UI_PREF_LAST_TASK_LIST_ID,
                &input.last_task_list_id,
                &input.now,
            )?;
            upsert_ui_preference(
                transaction,
                UI_PREF_CALENDAR_VIEW_MODE,
                &input.calendar_view_mode,
                &input.now,
            )?;
            select_ui_preferences(transaction)
        })
    }
}

fn insert_task(
    transaction: &Transaction<'_>,
    input: WorkItemCreate,
) -> RepositoryResult<TaskRecord> {
    insert_task_in_board_column(transaction, input, None)
}

fn insert_task_in_board_column(
    transaction: &Transaction<'_>,
    input: WorkItemCreate,
    requested_board_column_id: Option<&str>,
) -> RepositoryResult<TaskRecord> {
    let id = Uuid::new_v4().to_string();
    ensure_default_task_list(transaction, &input.now)?;
    ensure_task_list_exists(transaction, &input.list_id)?;
    ensure_default_board_columns(transaction, &input.now)?;
    let board_column_id = match requested_board_column_id {
        Some(column_id) => {
            ensure_board_column_exists(transaction, column_id)?;
            column_id.to_string()
        }
        None => select_preferred_active_board_column_id(transaction, DEFAULT_BOARD_COLUMN_ID)?,
    };
    let sort_order = next_task_sort_order(transaction, &input.list_id)?;
    let planned_start_date = input.planned_start_date.clone();
    let due_date = input.due_date.clone();
    let due_time = input.due_time.clone();
    let now = input.now.clone();
    transaction
        .execute(
            "
            INSERT INTO tasks (
              id, list_id, board_column_id, title, status, lifecycle_status, is_favorite,
              planned_start_date, due_date, due_time, timer_target_seconds, memo,
              sort_order, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, 'todo', 'active', 0, ?5, ?6, ?7, NULL, ?8, ?9, ?10, ?10)
            ",
            params![
                id,
                input.list_id,
                board_column_id,
                input.title,
                input.planned_start_date,
                input.due_date,
                input.due_time,
                input.memo,
                sort_order,
                input.now
            ],
        )
        .map_err(|error| format!("タスクを作成できません: {error}"))?;
    insert_notification_rules_for_target(
        transaction,
        &WorkTargetRef {
            target_type: WorkTargetType::Task,
            id: id.clone(),
        },
        planned_start_date.as_deref(),
        due_date.as_deref(),
        due_time.as_deref(),
        &now,
    )?;

    select_task_by_id(transaction, &id)
}

fn insert_task_list(
    transaction: &Transaction<'_>,
    input: TaskListCreate,
) -> RepositoryResult<TaskListRecord> {
    ensure_default_task_list(transaction, &input.now)?;
    ensure_unique_task_list_name(transaction, &input.name, None)?;

    let id = Uuid::new_v4().to_string();
    let sort_order = next_task_list_sort_order(transaction)?;

    transaction
        .execute(
            "
            INSERT INTO task_lists (
              id, name, color_token, sort_order, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?5)
            ",
            params![id, input.name, input.color_token, sort_order, input.now],
        )
        .map_err(|error| format!("タスクリストを作成できません: {error}"))?;

    select_task_list_by_id(transaction, &id)
}

fn insert_subtask(
    transaction: &Transaction<'_>,
    task_id: &str,
    input: WorkItemCreate,
) -> RepositoryResult<SubtaskRecord> {
    let id = Uuid::new_v4().to_string();
    let sort_order = next_subtask_sort_order(transaction, task_id)?;
    let planned_start_date = input.planned_start_date.clone();
    let due_date = input.due_date.clone();
    let due_time = input.due_time.clone();
    let now = input.now.clone();
    transaction
        .execute(
            "
            INSERT INTO subtasks (
              id, task_id, title, status, planned_start_date, due_date, due_time,
              timer_target_seconds, memo, sort_order, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, 'todo', ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?9)
            ",
            params![
                id,
                task_id,
                input.title,
                input.planned_start_date,
                input.due_date,
                input.due_time,
                input.memo,
                sort_order,
                input.now
            ],
        )
        .map_err(|error| format!("サブタスクを作成できません: {error}"))?;
    insert_notification_rules_for_target(
        transaction,
        &WorkTargetRef {
            target_type: WorkTargetType::Subtask,
            id: id.clone(),
        },
        planned_start_date.as_deref(),
        due_date.as_deref(),
        due_time.as_deref(),
        &now,
    )?;

    select_subtask_by_id(transaction, &id)
}

fn update_task_detail(
    transaction: &Transaction<'_>,
    task_id: &str,
    input: WorkItemUpdate,
) -> RepositoryResult<TaskRecord> {
    ensure_task_exists(transaction, task_id)?;
    let current_list_id: String = transaction
        .query_row(
            "
            SELECT list_id
            FROM tasks
            WHERE id = ?1
              AND deleted_at IS NULL
            ",
            params![task_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("タスクの所属リストを取得できません: {error}"))?;
    let next_sort_order = if let Some(list_id) = input.list_id.as_deref() {
        ensure_task_list_exists(transaction, list_id)?;
        if list_id != current_list_id {
            Some(next_task_sort_order(transaction, list_id)?)
        } else {
            None
        }
    } else {
        None
    };
    let planned_start_date = input.planned_start_date.clone();
    let due_date = input.due_date.clone();
    let due_time = input.due_time.clone();
    let recurrence_rule = input.recurrence_rule.clone();
    let now = input.now.clone();

    let updated = transaction
        .execute(
            "
            UPDATE tasks
            SET list_id = COALESCE(?1, list_id),
                sort_order = COALESCE(?2, sort_order),
                title = ?3,
                planned_start_date = ?4,
                due_date = ?5,
                due_time = ?6,
                timer_target_seconds = ?7,
                color_token = ?8,
                memo = ?9,
                updated_at = ?10
            WHERE id = ?11
              AND deleted_at IS NULL
            ",
            params![
                input.list_id,
                next_sort_order,
                input.title,
                input.planned_start_date,
                input.due_date,
                input.due_time,
                input.timer_target_seconds,
                input.color_token,
                input.memo,
                input.now,
                task_id
            ],
        )
        .map_err(|error| format!("タスク詳細を更新できません: {error}"))?;
    if updated != 1 {
        return Err("更新対象のタスクが存在しません".to_string());
    }

    sync_notification_rules_for_target(
        transaction,
        &WorkTargetRef {
            target_type: WorkTargetType::Task,
            id: task_id.to_string(),
        },
        planned_start_date.as_deref(),
        due_date.as_deref(),
        due_time.as_deref(),
        &now,
    )?;
    sync_recurrence_rule_for_target(
        transaction,
        &WorkTargetRef {
            target_type: WorkTargetType::Task,
            id: task_id.to_string(),
        },
        recurrence_rule,
        &now,
    )?;

    select_existing_task_by_id(transaction, task_id)
}

fn update_task_list_detail(
    transaction: &Transaction<'_>,
    list_id: &str,
    input: TaskListUpdate,
) -> RepositoryResult<TaskListRecord> {
    ensure_task_list_exists(transaction, list_id)?;
    if list_id == DEFAULT_TASK_LIST_ID {
        if input.name != DEFAULT_TASK_LIST_NAME {
            return Err("初期リスト名は変更できません".to_string());
        }
    } else {
        ensure_unique_task_list_name(transaction, &input.name, Some(list_id))?;
    }

    let updated = transaction
        .execute(
            "
            UPDATE task_lists
            SET name = ?1,
                color_token = COALESCE(?2, color_token),
                updated_at = ?3
            WHERE id = ?4
              AND deleted_at IS NULL
            ",
            params![input.name, input.color_token, input.now, list_id],
        )
        .map_err(|error| format!("タスクリストを更新できません: {error}"))?;
    if updated != 1 {
        return Err("更新対象のタスクリストが存在しません".to_string());
    }

    select_task_list_by_id(transaction, list_id)
}

fn soft_delete_task_list(
    transaction: &Transaction<'_>,
    list_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    ensure_default_task_list(transaction, now)?;
    ensure_custom_task_list(transaction, list_id)?;

    transaction
        .execute(
            "
            UPDATE tasks
            SET list_id = ?1,
                updated_at = ?2
            WHERE list_id = ?3
              AND deleted_at IS NULL
            ",
            params![DEFAULT_TASK_LIST_ID, now, list_id],
        )
        .map_err(|error| format!("タスクリスト内のタスクを移動できません: {error}"))?;

    let updated = transaction
        .execute(
            "
            UPDATE task_lists
            SET deleted_at = ?1,
                updated_at = ?1
            WHERE id = ?2
              AND deleted_at IS NULL
            ",
            params![now, list_id],
        )
        .map_err(|error| format!("タスクリストを削除できません: {error}"))?;
    if updated != 1 {
        return Err("削除対象のタスクリストが存在しません".to_string());
    }

    Ok(())
}

fn select_tags(connection: &Connection) -> RepositoryResult<Vec<TagRecord>> {
    let mut statement = connection
        .prepare(
            "
            SELECT tags.id,
                   tags.name,
                   tags.sort_order,
                   tags.created_at,
                   tags.updated_at,
                   COUNT(tasks.id) AS task_count
            FROM tags
            LEFT JOIN task_tags
              ON task_tags.tag_id = tags.id
             AND task_tags.deleted_at IS NULL
            LEFT JOIN tasks
              ON tasks.id = task_tags.task_id
             AND tasks.deleted_at IS NULL
             AND tasks.status <> 'archived'
            WHERE tags.deleted_at IS NULL
            GROUP BY tags.id,
                     tags.name,
                     tags.sort_order,
                     tags.created_at,
                     tags.updated_at
            ORDER BY tags.sort_order ASC, tags.created_at ASC
            ",
        )
        .map_err(|error| format!("タグ一覧クエリを準備できません: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok(TagRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                sort_order: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                task_count: row.get(5)?,
            })
        })
        .map_err(|error| format!("タグ一覧を取得できません: {error}"))?;

    rows.map(|row| row.map_err(|error| format!("タグ行を読めません: {error}")))
        .collect()
}

fn insert_tag(transaction: &Transaction<'_>, input: TagCreate) -> RepositoryResult<TagRecord> {
    ensure_unique_tag_name(transaction, &input.name, None)?;

    let id = Uuid::new_v4().to_string();
    let sort_order = next_tag_sort_order(transaction)?;
    transaction
        .execute(
            "
            INSERT INTO tags (id, name, sort_order, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?4)
            ",
            params![id, input.name, sort_order, input.now],
        )
        .map_err(|error| format!("タグを作成できません: {error}"))?;

    select_tag_by_id(transaction, &id)
}

fn update_tag_detail(
    transaction: &Transaction<'_>,
    tag_id: &str,
    input: TagUpdate,
) -> RepositoryResult<TagRecord> {
    ensure_tag_exists(transaction, tag_id)?;
    ensure_unique_tag_name(transaction, &input.name, Some(tag_id))?;

    let updated = transaction
        .execute(
            "
            UPDATE tags
            SET name = ?1,
                updated_at = ?2
            WHERE id = ?3
              AND deleted_at IS NULL
            ",
            params![input.name, input.now, tag_id],
        )
        .map_err(|error| format!("タグを更新できません: {error}"))?;
    if updated != 1 {
        return Err("更新対象のタグが存在しません".to_string());
    }

    select_tag_by_id(transaction, tag_id)
}

fn soft_delete_tag(transaction: &Transaction<'_>, tag_id: &str, now: &str) -> RepositoryResult<()> {
    ensure_tag_exists(transaction, tag_id)?;

    transaction
        .execute(
            "
            UPDATE task_tags
            SET deleted_at = ?1
            WHERE tag_id = ?2
              AND deleted_at IS NULL
            ",
            params![now, tag_id],
        )
        .map_err(|error| format!("タグ関連を削除できません: {error}"))?;

    let updated = transaction
        .execute(
            "
            UPDATE tags
            SET deleted_at = ?1,
                updated_at = ?1
            WHERE id = ?2
              AND deleted_at IS NULL
            ",
            params![now, tag_id],
        )
        .map_err(|error| format!("タグを削除できません: {error}"))?;
    if updated != 1 {
        return Err("削除対象のタグが存在しません".to_string());
    }

    Ok(())
}

fn attach_tag_to_task_row(
    transaction: &Transaction<'_>,
    task_id: &str,
    tag_id: &str,
    now: &str,
) -> RepositoryResult<TaskTagRecord> {
    ensure_task_exists(transaction, task_id)?;
    ensure_tag_exists(transaction, tag_id)?;

    transaction
        .execute(
            "
            INSERT INTO task_tags (task_id, tag_id, created_at, deleted_at)
            VALUES (?1, ?2, ?3, NULL)
            ON CONFLICT(task_id, tag_id) DO UPDATE SET deleted_at = NULL
            ",
            params![task_id, tag_id, now],
        )
        .map_err(|error| format!("タスクへタグを追加できません: {error}"))?;

    select_task_tag_by_id(transaction, tag_id)
}

fn detach_tag_from_task_row(
    transaction: &Transaction<'_>,
    task_id: &str,
    tag_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    ensure_task_exists(transaction, task_id)?;
    ensure_tag_exists(transaction, tag_id)?;

    transaction
        .execute(
            "
            UPDATE task_tags
            SET deleted_at = ?1
            WHERE task_id = ?2
              AND tag_id = ?3
              AND deleted_at IS NULL
            ",
            params![now, task_id, tag_id],
        )
        .map(|_| ())
        .map_err(|error| format!("タスクからタグを外せません: {error}"))
}

fn update_subtask_detail(
    transaction: &Transaction<'_>,
    subtask_id: &str,
    input: WorkItemUpdate,
) -> RepositoryResult<SubtaskRecord> {
    ensure_subtask_exists(transaction, subtask_id)?;
    let planned_start_date = input.planned_start_date.clone();
    let due_date = input.due_date.clone();
    let due_time = input.due_time.clone();
    let recurrence_rule = input.recurrence_rule.clone();
    let now = input.now.clone();

    let updated = transaction
        .execute(
            "
            UPDATE subtasks
            SET title = ?1,
                planned_start_date = ?2,
                due_date = ?3,
                due_time = ?4,
                timer_target_seconds = ?5,
                memo = ?6,
                updated_at = ?7
            WHERE id = ?8
              AND deleted_at IS NULL
            ",
            params![
                input.title,
                input.planned_start_date,
                input.due_date,
                input.due_time,
                input.timer_target_seconds,
                input.memo,
                input.now,
                subtask_id
            ],
        )
        .map_err(|error| format!("サブタスク詳細を更新できません: {error}"))?;
    if updated != 1 {
        return Err("更新対象のサブタスクが存在しません".to_string());
    }

    sync_notification_rules_for_target(
        transaction,
        &WorkTargetRef {
            target_type: WorkTargetType::Subtask,
            id: subtask_id.to_string(),
        },
        planned_start_date.as_deref(),
        due_date.as_deref(),
        due_time.as_deref(),
        &now,
    )?;
    sync_recurrence_rule_for_target(
        transaction,
        &WorkTargetRef {
            target_type: WorkTargetType::Subtask,
            id: subtask_id.to_string(),
        },
        recurrence_rule,
        &now,
    )?;

    select_existing_subtask_by_id(transaction, subtask_id)
}

fn update_work_schedule_row(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    input: WorkScheduleUpdate,
) -> RepositoryResult<()> {
    let schedule = input.schedule;
    let (table_name, target_label) = match target.target_type {
        WorkTargetType::Task => ("tasks", "タスク"),
        WorkTargetType::Subtask => ("subtasks", "サブタスク"),
    };
    let sql = format!(
        "
        UPDATE {table_name}
        SET scheduled_start_date = ?1,
            scheduled_start_time = ?2,
            scheduled_end_date = ?3,
            scheduled_end_time = ?4,
            scheduled_is_all_day = ?5,
            updated_at = ?6
        WHERE id = ?7
          AND deleted_at IS NULL
          AND status <> 'archived'
        "
    );
    let updated = transaction
        .execute(
            &sql,
            params![
                schedule.start_date,
                schedule.start_time,
                schedule.end_date,
                schedule.end_time,
                i64::from(schedule.is_all_day),
                input.now,
                target.id
            ],
        )
        .map_err(|error| format!("{target_label}の予定期間を更新できません: {error}"))?;
    if updated != 1 {
        return Err(format!("更新対象の{target_label}が存在しません"));
    }
    Ok(())
}

fn select_work_schedule_for_move(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
) -> RepositoryResult<WorkSchedule> {
    let (table_name, target_label) = match target.target_type {
        WorkTargetType::Task => ("tasks", "タスク"),
        WorkTargetType::Subtask => ("subtasks", "サブタスク"),
    };
    let sql = format!(
        "
        SELECT scheduled_start_date,
               scheduled_start_time,
               scheduled_end_date,
               scheduled_end_time,
               scheduled_is_all_day
        FROM {table_name}
        WHERE id = ?1
          AND deleted_at IS NULL
          AND status <> 'archived'
        "
    );
    let schedule = transaction
        .query_row(&sql, params![target.id], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .optional()
        .map_err(|error| format!("{target_label}の予定期間を取得できません: {error}"))?
        .ok_or_else(|| format!("更新対象の{target_label}が存在しません"))?;

    let (start_date, start_time, end_date, end_time, is_all_day) = schedule;
    let start_date = start_date.ok_or_else(|| format!("{target_label}に予定期間がありません"))?;
    let end_date = end_date.ok_or_else(|| format!("{target_label}の予定期間データが不正です"))?;
    WorkSchedule::parse(
        &start_date,
        start_time.as_deref(),
        &end_date,
        end_time.as_deref(),
        is_all_day != 0,
    )
}

fn insert_notification_rules_for_target(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    planned_start_date: Option<&str>,
    due_date: Option<&str>,
    due_time: Option<&str>,
    now: &str,
) -> RepositoryResult<()> {
    if let Some(date) = planned_start_date {
        insert_notification_rule(
            transaction,
            target,
            NotificationKind::PlannedStart,
            &notification_time_for_date(date, None),
            now,
        )?;
    }

    if let Some(date) = due_date {
        insert_notification_rule(
            transaction,
            target,
            NotificationKind::Due,
            &notification_time_for_date(date, due_time),
            now,
        )?;
    }

    Ok(())
}

fn sync_notification_rules_for_target(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    planned_start_date: Option<&str>,
    due_date: Option<&str>,
    due_time: Option<&str>,
    now: &str,
) -> RepositoryResult<()> {
    sync_notification_rule_for_kind(
        transaction,
        target,
        NotificationKind::PlannedStart,
        planned_start_date,
        None,
        now,
    )?;
    sync_notification_rule_for_kind(
        transaction,
        target,
        NotificationKind::Due,
        due_date,
        due_time,
        now,
    )?;
    mark_notification_os_registrations_pending_for_target(transaction, target, now)
}

fn sync_notification_rule_for_kind(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    kind: NotificationKind,
    date: Option<&str>,
    time: Option<&str>,
    now: &str,
) -> RepositoryResult<()> {
    let existing = select_active_notification_rule_for_kind(transaction, target, &kind)?;
    let Some(date) = date else {
        return disable_notification_rules_for_kind(transaction, target, &kind, now);
    };
    let notify_at = notification_time_for_date(date, time);

    if let Some(existing) = existing {
        disable_duplicate_notification_rules_for_kind(
            transaction,
            target,
            &kind,
            &existing.id,
            now,
        )?;
        if existing.notify_at == notify_at {
            return Ok(());
        }

        let enabled = existing.enabled;
        let registration_status = if enabled { "pending" } else { "disabled" };
        transaction
            .execute(
                "
                UPDATE notification_rules
                SET notify_at = ?1,
                    enabled = ?2,
                    registration_status = ?3,
                    last_error = NULL,
                    updated_at = ?4
                WHERE id = ?5
                  AND deleted_at IS NULL
                ",
                params![notify_at, enabled, registration_status, now, existing.id],
            )
            .map_err(|error| format!("通知ルールを更新できません: {error}"))?;
        mark_notification_os_registration_pending_for_rule(transaction, &existing.id, now)
    } else {
        insert_notification_rule(transaction, target, kind, &notify_at, now)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExistingNotificationRule {
    id: String,
    notify_at: String,
    enabled: bool,
}

fn select_active_notification_rule_for_kind(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    kind: &NotificationKind,
) -> RepositoryResult<Option<ExistingNotificationRule>> {
    transaction
        .query_row(
            "
            SELECT id, notify_at, enabled
            FROM notification_rules
            WHERE target_type = ?1
              AND target_id = ?2
              AND kind = ?3
              AND deleted_at IS NULL
            ORDER BY created_at DESC
            LIMIT 1
            ",
            params![
                target.target_type.as_str(),
                target.id.as_str(),
                kind.as_str()
            ],
            |row| {
                Ok(ExistingNotificationRule {
                    id: row.get(0)?,
                    notify_at: row.get(1)?,
                    enabled: row.get::<_, i64>(2)? != 0,
                })
            },
        )
        .optional()
        .map_err(|error| format!("通知ルールを取得できません: {error}"))
}

fn disable_notification_rules_for_kind(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    kind: &NotificationKind,
    now: &str,
) -> RepositoryResult<()> {
    mark_notification_os_registrations_cancel_pending_for_target_kind(
        transaction,
        target,
        kind,
        None,
        now,
    )?;
    transaction
        .execute(
            "
            UPDATE notification_rules
            SET enabled = 0,
                registration_status = 'disabled',
                deleted_at = ?1,
                updated_at = ?1
            WHERE target_type = ?2
              AND target_id = ?3
              AND kind = ?4
              AND deleted_at IS NULL
            ",
            params![
                now,
                target.target_type.as_str(),
                target.id.as_str(),
                kind.as_str()
            ],
        )
        .map(|_| ())
        .map_err(|error| format!("通知ルールを無効化できません: {error}"))
}

fn disable_duplicate_notification_rules_for_kind(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    kind: &NotificationKind,
    keep_rule_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    mark_notification_os_registrations_cancel_pending_for_target_kind(
        transaction,
        target,
        kind,
        Some(keep_rule_id),
        now,
    )?;
    transaction
        .execute(
            "
            UPDATE notification_rules
            SET enabled = 0,
                registration_status = 'disabled',
                deleted_at = ?1,
                updated_at = ?1
            WHERE target_type = ?2
              AND target_id = ?3
              AND kind = ?4
              AND id <> ?5
              AND deleted_at IS NULL
            ",
            params![
                now,
                target.target_type.as_str(),
                target.id.as_str(),
                kind.as_str(),
                keep_rule_id
            ],
        )
        .map(|_| ())
        .map_err(|error| format!("重複通知ルールを無効化できません: {error}"))
}

fn sync_recurrence_rule_for_target(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    recurrence_rule: Option<RecurrenceRuleInput>,
    now: &str,
) -> RepositoryResult<()> {
    let Some(recurrence_rule) = recurrence_rule else {
        return disable_recurrence_rule_for_target(transaction, target, now);
    };

    if let Some(existing) = select_recurrence_rule_for_target(transaction, target)? {
        transaction
            .execute(
                "
                UPDATE recurrence_rules
                SET frequency = ?1,
                    interval = ?2,
                    updated_at = ?3
                WHERE id = ?4
                  AND deleted_at IS NULL
                ",
                params![
                    recurrence_rule.frequency.as_str(),
                    recurrence_rule.interval,
                    now,
                    existing.id
                ],
            )
            .map(|_| ())
            .map_err(|error| format!("繰り返し設定を更新できません: {error}"))
    } else {
        transaction
            .execute(
                "
                INSERT INTO recurrence_rules (
                  id, target_type, target_id, frequency, interval, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
                ",
                params![
                    Uuid::new_v4().to_string(),
                    target.target_type.as_str(),
                    target.id.as_str(),
                    recurrence_rule.frequency.as_str(),
                    recurrence_rule.interval,
                    now
                ],
            )
            .map(|_| ())
            .map_err(|error| format!("繰り返し設定を保存できません: {error}"))
    }
}

fn disable_recurrence_rule_for_target(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE recurrence_rules
            SET deleted_at = ?1,
                updated_at = ?1
            WHERE target_type = ?2
              AND target_id = ?3
              AND deleted_at IS NULL
            ",
            params![now, target.target_type.as_str(), target.id.as_str()],
        )
        .map(|_| ())
        .map_err(|error| format!("繰り返し設定を無効化できません: {error}"))
}

fn select_recurrence_rule_for_target(
    connection: &Connection,
    target: &WorkTargetRef,
) -> RepositoryResult<Option<RecurrenceRuleRecord>> {
    connection
        .query_row(
            "
            SELECT id, target_type, target_id, frequency, interval,
                   deleted_at, created_at, updated_at
            FROM recurrence_rules
            WHERE target_type = ?1
              AND target_id = ?2
              AND deleted_at IS NULL
            LIMIT 1
            ",
            params![target.target_type.as_str(), target.id.as_str()],
            |row| {
                let target_type_text: String = row.get(1)?;
                let frequency_text: String = row.get(3)?;
                Ok(RecurrenceRuleRecord {
                    id: row.get(0)?,
                    target: target_ref(
                        WorkTargetType::from_db(&target_type_text).map_err(db_value_error)?,
                        row.get(2)?,
                    ),
                    frequency: RecurrenceFrequency::from_db(&frequency_text)
                        .map_err(db_value_error)?,
                    interval: row.get(4)?,
                    deleted_at: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(|error| format!("繰り返し設定を取得できません: {error}"))
}

fn insert_notification_rule(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    kind: NotificationKind,
    notify_at: &str,
    now: &str,
) -> RepositoryResult<()> {
    let notification_rule_id = Uuid::new_v4().to_string();
    transaction
        .execute(
            "
            INSERT INTO notification_rules (
              id, target_type, target_id, kind, notify_at, enabled,
              registration_status, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, 1, 'pending', ?6, ?6)
            ",
            params![
                notification_rule_id.as_str(),
                target.target_type.as_str(),
                target.id.as_str(),
                kind.as_str(),
                notify_at,
                now
            ],
        )
        .map_err(|error| format!("通知ルールを作成できません: {error}"))?;

    ensure_notification_os_registration_for_rule(transaction, &notification_rule_id, now)
}

fn ensure_notification_os_registration_for_rule(
    transaction: &Transaction<'_>,
    notification_rule_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            INSERT INTO notification_os_registrations (
              id, notification_rule_id, os_registration_id, registration_status,
              created_at, updated_at
            )
            SELECT ?1, ?2, NULL, 'pending', ?3, ?3
            WHERE NOT EXISTS (
              SELECT 1
              FROM notification_os_registrations
              WHERE notification_rule_id = ?2
                AND deleted_at IS NULL
            )
            ",
            params![Uuid::new_v4().to_string(), notification_rule_id, now],
        )
        .map(|_| ())
        .map_err(|error| format!("通知OS登録状態を作成できません: {error}"))
}

fn mark_notification_os_registration_pending_for_rule(
    transaction: &Transaction<'_>,
    notification_rule_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    ensure_notification_os_registration_for_rule(transaction, notification_rule_id, now)?;
    transaction
        .execute(
            "
            UPDATE notification_os_registrations
            SET registration_status = 'pending',
                last_attempted_at = NULL,
                last_error = NULL,
                updated_at = ?1
            WHERE notification_rule_id = ?2
              AND deleted_at IS NULL
            ",
            params![now, notification_rule_id],
        )
        .map(|_| ())
        .map_err(|error| format!("通知OS登録状態を再同期待ちにできません: {error}"))
}

fn mark_notification_os_registrations_pending_for_target(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE notification_os_registrations
            SET registration_status = 'pending',
                last_attempted_at = NULL,
                last_error = NULL,
                updated_at = ?1
            WHERE deleted_at IS NULL
              AND notification_rule_id IN (
                SELECT id
                FROM notification_rules
                WHERE target_type = ?2
                  AND target_id = ?3
                  AND enabled = 1
                  AND deleted_at IS NULL
              )
            ",
            params![now, target.target_type.as_str(), target.id.as_str()],
        )
        .map(|_| ())
        .map_err(|error| format!("通知OS登録状態を再同期待ちにできません: {error}"))
}

fn mark_notification_os_registrations_cancel_pending_for_target_kind(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    kind: &NotificationKind,
    keep_rule_id: Option<&str>,
    now: &str,
) -> RepositoryResult<()> {
    match keep_rule_id {
        Some(keep_rule_id) => transaction.execute(
            "
            UPDATE notification_os_registrations
            SET registration_status = CASE
                    WHEN os_registration_id IS NULL THEN 'disabled'
                    ELSE 'cancel_pending'
                END,
                last_attempted_at = NULL,
                last_error = NULL,
                deleted_at = CASE
                    WHEN os_registration_id IS NULL THEN ?1
                    ELSE NULL
                END,
                updated_at = ?1
            WHERE deleted_at IS NULL
              AND notification_rule_id IN (
                SELECT id
                FROM notification_rules
                WHERE target_type = ?2
                  AND target_id = ?3
                  AND kind = ?4
                  AND id <> ?5
                  AND deleted_at IS NULL
              )
            ",
            params![
                now,
                target.target_type.as_str(),
                target.id.as_str(),
                kind.as_str(),
                keep_rule_id
            ],
        ),
        None => transaction.execute(
            "
            UPDATE notification_os_registrations
            SET registration_status = CASE
                    WHEN os_registration_id IS NULL THEN 'disabled'
                    ELSE 'cancel_pending'
                END,
                last_attempted_at = NULL,
                last_error = NULL,
                deleted_at = CASE
                    WHEN os_registration_id IS NULL THEN ?1
                    ELSE NULL
                END,
                updated_at = ?1
            WHERE deleted_at IS NULL
              AND notification_rule_id IN (
                SELECT id
                FROM notification_rules
                WHERE target_type = ?2
                  AND target_id = ?3
                  AND kind = ?4
                  AND deleted_at IS NULL
              )
            ",
            params![
                now,
                target.target_type.as_str(),
                target.id.as_str(),
                kind.as_str()
            ],
        ),
    }
    .map(|_| ())
    .map_err(|error| format!("通知OS登録状態を解除待ちにできません: {error}"))
}

fn mark_notification_os_registrations_cancel_pending_for_subtask(
    transaction: &Transaction<'_>,
    subtask_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE notification_os_registrations
            SET registration_status = CASE
                    WHEN os_registration_id IS NULL THEN 'disabled'
                    ELSE 'cancel_pending'
                END,
                last_attempted_at = NULL,
                last_error = NULL,
                deleted_at = CASE
                    WHEN os_registration_id IS NULL THEN ?1
                    ELSE NULL
                END,
                updated_at = ?1
            WHERE deleted_at IS NULL
              AND notification_rule_id IN (
                SELECT id
                FROM notification_rules
                WHERE target_type = 'subtask'
                  AND target_id = ?2
                  AND deleted_at IS NULL
              )
            ",
            params![now, subtask_id],
        )
        .map(|_| ())
        .map_err(|error| format!("通知OS登録状態を解除待ちにできません: {error}"))
}

fn mark_notification_os_registrations_cancel_pending_for_task_graph(
    transaction: &Transaction<'_>,
    task_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE notification_os_registrations
            SET registration_status = CASE
                    WHEN os_registration_id IS NULL THEN 'disabled'
                    ELSE 'cancel_pending'
                END,
                last_attempted_at = NULL,
                last_error = NULL,
                deleted_at = CASE
                    WHEN os_registration_id IS NULL THEN ?1
                    ELSE NULL
                END,
                updated_at = ?1
            WHERE deleted_at IS NULL
              AND notification_rule_id IN (
                SELECT id
                FROM notification_rules
                WHERE deleted_at IS NULL
                  AND (
                    (target_type = 'task' AND target_id = ?2)
                    OR (
                      target_type = 'subtask'
                      AND target_id IN (
                        SELECT id
                        FROM subtasks
                        WHERE task_id = ?2
                      )
                    )
                  )
              )
            ",
            params![now, task_id],
        )
        .map(|_| ())
        .map_err(|error| format!("通知OS登録状態を解除待ちにできません: {error}"))
}

fn mark_future_notification_os_registrations_pending(
    transaction: &Transaction<'_>,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE notification_os_registrations
            SET registration_status = 'pending',
                last_attempted_at = NULL,
                last_error = NULL,
                updated_at = ?1
            WHERE deleted_at IS NULL
              AND notification_rule_id IN (
                SELECT notification_rules.id
                FROM notification_rules
                LEFT JOIN tasks
                  ON notification_rules.target_type = 'task'
                 AND notification_rules.target_id = tasks.id
                 AND tasks.deleted_at IS NULL
                 AND tasks.status <> 'archived'
                LEFT JOIN subtasks
                  ON notification_rules.target_type = 'subtask'
                 AND notification_rules.target_id = subtasks.id
                 AND subtasks.deleted_at IS NULL
                 AND subtasks.status <> 'archived'
                LEFT JOIN tasks AS subtask_parent_tasks
                  ON notification_rules.target_type = 'subtask'
                 AND subtasks.task_id = subtask_parent_tasks.id
                 AND subtask_parent_tasks.deleted_at IS NULL
                 AND subtask_parent_tasks.status <> 'archived'
                WHERE notification_rules.enabled = 1
                  AND notification_rules.deleted_at IS NULL
                  AND notification_rules.notify_at > ?1
                  AND (
                    (
                      notification_rules.target_type = 'task'
                      AND tasks.id IS NOT NULL
                    )
                    OR (
                      notification_rules.target_type = 'subtask'
                      AND subtasks.id IS NOT NULL
                      AND subtask_parent_tasks.id IS NOT NULL
                    )
                  )
              )
            ",
            params![now],
        )
        .map(|_| ())
        .map_err(|error| format!("通知OS登録状態を再同期待ちにできません: {error}"))
}

fn disable_future_notification_os_registrations(
    transaction: &Transaction<'_>,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE notification_os_registrations
            SET registration_status = CASE
                    WHEN os_registration_id IS NULL THEN 'disabled'
                    ELSE 'cancel_pending'
                END,
                last_attempted_at = NULL,
                last_error = NULL,
                deleted_at = CASE
                    WHEN os_registration_id IS NULL THEN ?1
                    ELSE NULL
                END,
                updated_at = ?1
            WHERE deleted_at IS NULL
              AND notification_rule_id IN (
                SELECT id
                FROM notification_rules
                WHERE enabled = 1
                  AND deleted_at IS NULL
                  AND notify_at > ?1
              )
            ",
            params![now],
        )
        .map(|_| ())
        .map_err(|error| format!("通知OS登録状態を通知OFFへ同期できません: {error}"))
}

fn reactivate_future_notification_os_registrations(
    transaction: &Transaction<'_>,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE notification_os_registrations
            SET registration_status = 'pending',
                last_attempted_at = NULL,
                last_error = NULL,
                updated_at = ?1
            WHERE deleted_at IS NULL
              AND registration_status IN ('disabled', 'cancel_pending')
              AND notification_rule_id IN (
                SELECT notification_rules.id
                FROM notification_rules
                LEFT JOIN tasks
                  ON notification_rules.target_type = 'task'
                 AND notification_rules.target_id = tasks.id
                 AND tasks.deleted_at IS NULL
                 AND tasks.status <> 'archived'
                LEFT JOIN subtasks
                  ON notification_rules.target_type = 'subtask'
                 AND notification_rules.target_id = subtasks.id
                 AND subtasks.deleted_at IS NULL
                 AND subtasks.status <> 'archived'
                LEFT JOIN tasks AS subtask_parent_tasks
                  ON notification_rules.target_type = 'subtask'
                 AND subtasks.task_id = subtask_parent_tasks.id
                 AND subtask_parent_tasks.deleted_at IS NULL
                 AND subtask_parent_tasks.status <> 'archived'
                WHERE notification_rules.enabled = 1
                  AND notification_rules.deleted_at IS NULL
                  AND notification_rules.notify_at > ?1
                  AND (
                    (
                      notification_rules.target_type = 'task'
                      AND tasks.id IS NOT NULL
                    )
                    OR (
                      notification_rules.target_type = 'subtask'
                      AND subtasks.id IS NOT NULL
                      AND subtask_parent_tasks.id IS NOT NULL
                    )
                  )
              )
            ",
            params![now],
        )
        .map_err(|error| format!("通知OS登録状態を通知ONへ同期できません: {error}"))?;

    let mut statement = transaction
        .prepare(
            "
            SELECT notification_rules.id
            FROM notification_rules
            LEFT JOIN notification_os_registrations AS registrations
              ON registrations.notification_rule_id = notification_rules.id
             AND registrations.deleted_at IS NULL
            LEFT JOIN tasks
              ON notification_rules.target_type = 'task'
             AND notification_rules.target_id = tasks.id
             AND tasks.deleted_at IS NULL
             AND tasks.status <> 'archived'
            LEFT JOIN subtasks
              ON notification_rules.target_type = 'subtask'
             AND notification_rules.target_id = subtasks.id
             AND subtasks.deleted_at IS NULL
             AND subtasks.status <> 'archived'
            LEFT JOIN tasks AS subtask_parent_tasks
              ON notification_rules.target_type = 'subtask'
             AND subtasks.task_id = subtask_parent_tasks.id
             AND subtask_parent_tasks.deleted_at IS NULL
             AND subtask_parent_tasks.status <> 'archived'
            WHERE notification_rules.enabled = 1
              AND notification_rules.deleted_at IS NULL
              AND notification_rules.notify_at > ?1
              AND registrations.id IS NULL
              AND (
                (
                  notification_rules.target_type = 'task'
                  AND tasks.id IS NOT NULL
                )
                OR (
                  notification_rules.target_type = 'subtask'
                  AND subtasks.id IS NOT NULL
                  AND subtask_parent_tasks.id IS NOT NULL
                )
              )
            ",
        )
        .map_err(|error| format!("通知OS登録再作成クエリを準備できません: {error}"))?;

    let rule_ids = statement
        .query_map(params![now], |row| row.get::<_, String>(0))
        .map_err(|error| format!("通知OS登録再作成対象を取得できません: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("通知OS登録再作成対象行を読めません: {error}"))?;

    drop(statement);

    for rule_id in rule_ids {
        ensure_notification_os_registration_for_rule(transaction, &rule_id, now)?;
    }

    Ok(())
}

fn notification_time_for_date(date: &str, time: Option<&str>) -> String {
    let time = time.unwrap_or("00:00");
    format!("{date}T{time}:00Z")
}

fn select_due_notification_jobs(
    connection: &Connection,
    now: &str,
    limit: i64,
) -> RepositoryResult<Vec<NotificationJob>> {
    let mut statement = connection
        .prepare(
            "
            SELECT notification_rules.id,
                   notification_rules.target_type,
                   notification_rules.target_id,
                   notification_rules.kind,
                   notification_rules.notify_at,
                   notification_rules.registration_status,
                   COALESCE(tasks.title, subtasks.title) AS target_title
            FROM notification_rules
            LEFT JOIN tasks
              ON notification_rules.target_type = 'task'
             AND notification_rules.target_id = tasks.id
             AND tasks.deleted_at IS NULL
             AND tasks.status <> 'archived'
            LEFT JOIN subtasks
              ON notification_rules.target_type = 'subtask'
             AND notification_rules.target_id = subtasks.id
             AND subtasks.deleted_at IS NULL
             AND subtasks.status <> 'archived'
            LEFT JOIN tasks AS subtask_parent_tasks
              ON notification_rules.target_type = 'subtask'
             AND subtasks.task_id = subtask_parent_tasks.id
             AND subtask_parent_tasks.deleted_at IS NULL
             AND subtask_parent_tasks.status <> 'archived'
            WHERE notification_rules.enabled = 1
              AND notification_rules.deleted_at IS NULL
              AND notification_rules.notify_at <= ?1
              AND notification_rules.registration_status IN ('pending', 'failed')
              AND (
                (
                  notification_rules.target_type = 'task'
                  AND tasks.id IS NOT NULL
                )
                OR (
                  notification_rules.target_type = 'subtask'
                  AND subtasks.id IS NOT NULL
                  AND subtask_parent_tasks.id IS NOT NULL
                )
              )
            ORDER BY notification_rules.notify_at ASC,
                     notification_rules.created_at ASC
            LIMIT ?2
            ",
        )
        .map_err(|error| format!("通知ジョブクエリを準備できません: {error}"))?;

    let rows = statement
        .query_map(params![now, limit], |row| {
            let target_type_text: String = row.get(1)?;
            let target_type = WorkTargetType::from_db(&target_type_text).map_err(db_value_error)?;
            let kind_text: String = row.get(3)?;
            let registration_status_text: String = row.get(5)?;
            Ok(NotificationJob {
                id: row.get(0)?,
                target: target_ref(target_type, row.get(2)?),
                kind: NotificationKind::from_db(&kind_text).map_err(db_value_error)?,
                notify_at: row.get(4)?,
                registration_status: NotificationRegistrationStatus::from_db(
                    &registration_status_text,
                )
                .map_err(db_value_error)?,
                target_title: row.get(6)?,
            })
        })
        .map_err(|error| format!("通知ジョブを取得できません: {error}"))?;

    rows.map(|row| row.map_err(|error| format!("通知ジョブ行を読めません: {error}")))
        .collect()
}

fn select_next_pending_notification(
    connection: &Connection,
    now: &str,
) -> RepositoryResult<Option<NextNotificationSchedule>> {
    connection
        .query_row(
            "
            SELECT notification_rules.id,
                   notification_rules.notify_at
            FROM notification_rules
            LEFT JOIN tasks
              ON notification_rules.target_type = 'task'
             AND notification_rules.target_id = tasks.id
             AND tasks.deleted_at IS NULL
             AND tasks.status <> 'archived'
            LEFT JOIN subtasks
              ON notification_rules.target_type = 'subtask'
             AND notification_rules.target_id = subtasks.id
             AND subtasks.deleted_at IS NULL
             AND subtasks.status <> 'archived'
            LEFT JOIN tasks AS subtask_parent_tasks
              ON notification_rules.target_type = 'subtask'
             AND subtasks.task_id = subtask_parent_tasks.id
             AND subtask_parent_tasks.deleted_at IS NULL
             AND subtask_parent_tasks.status <> 'archived'
            WHERE notification_rules.enabled = 1
              AND notification_rules.deleted_at IS NULL
              AND notification_rules.notify_at > ?1
              AND notification_rules.registration_status IN ('pending', 'failed')
              AND (
                (
                  notification_rules.target_type = 'task'
                  AND tasks.id IS NOT NULL
                )
                OR (
                  notification_rules.target_type = 'subtask'
                  AND subtasks.id IS NOT NULL
                  AND subtask_parent_tasks.id IS NOT NULL
                )
              )
            ORDER BY notification_rules.notify_at ASC,
                     notification_rules.created_at ASC
            LIMIT 1
            ",
            params![now],
            |row| {
                Ok(NextNotificationSchedule {
                    notification_rule_id: row.get(0)?,
                    notify_at: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(|error| format!("次回通知予定を取得できません: {error}"))
}

fn select_notification_os_registration_jobs(
    connection: &Connection,
    now: &str,
    limit: i64,
) -> RepositoryResult<Vec<NotificationOsRegistrationJob>> {
    let mut statement = connection
        .prepare(
            "
            SELECT registrations.id,
                   registrations.notification_rule_id,
                   registrations.os_registration_id,
                   notification_rules.target_type,
                   notification_rules.target_id,
                   notification_rules.kind,
                   notification_rules.notify_at,
                   registrations.registration_status,
                   registrations.last_attempted_at,
                   registrations.last_error
            FROM notification_os_registrations AS registrations
            INNER JOIN notification_rules
              ON notification_rules.id = registrations.notification_rule_id
            WHERE registrations.deleted_at IS NULL
              AND (
                (
                  registrations.registration_status IN ('pending', 'failed')
                  AND notification_rules.enabled = 1
                  AND notification_rules.deleted_at IS NULL
                  AND notification_rules.notify_at > ?1
                )
                OR registrations.registration_status = 'cancel_pending'
              )
            ORDER BY CASE
                       WHEN registrations.registration_status = 'cancel_pending' THEN 0
                       ELSE 1
                     END,
                     notification_rules.notify_at ASC,
                     registrations.updated_at ASC,
                     registrations.id ASC
            LIMIT ?2
            ",
        )
        .map_err(|error| format!("通知OS登録ジョブクエリを準備できません: {error}"))?;

    let rows = statement
        .query_map(params![now, limit], |row| {
            let target_type_text: String = row.get(3)?;
            let target_type = WorkTargetType::from_db(&target_type_text).map_err(db_value_error)?;
            let kind_text: String = row.get(5)?;
            let registration_status_text: String = row.get(7)?;
            let registration_status =
                NotificationOsRegistrationStatus::from_db(&registration_status_text)
                    .map_err(db_value_error)?;
            let action = NotificationOsRegistrationAction::from_status(&registration_status);
            Ok(NotificationOsRegistrationJob {
                id: row.get(0)?,
                notification_rule_id: row.get(1)?,
                os_registration_id: row.get(2)?,
                target: target_ref(target_type, row.get(4)?),
                kind: NotificationKind::from_db(&kind_text).map_err(db_value_error)?,
                notify_at: row.get(6)?,
                registration_status,
                action,
                last_attempted_at: row.get(8)?,
                last_error: row.get(9)?,
            })
        })
        .map_err(|error| format!("通知OS登録ジョブを取得できません: {error}"))?;

    rows.map(|row| row.map_err(|error| format!("通知OS登録ジョブ行を読めません: {error}")))
        .collect()
}

fn select_native_notification_os_registration_jobs(
    connection: &Connection,
    now: &str,
    limit: i64,
) -> RepositoryResult<Vec<NativeNotificationOsRegistrationJob>> {
    let mut statement = connection
        .prepare(
            "
            SELECT registrations.id,
                   registrations.notification_rule_id,
                   registrations.os_registration_id,
                   notification_rules.target_type,
                   notification_rules.target_id,
                   notification_rules.kind,
                   COALESCE(tasks.title, subtasks.title, '') AS target_title,
                   notification_rules.notify_at,
                   registrations.registration_status,
                   registrations.last_attempted_at,
                   registrations.last_error
            FROM notification_os_registrations AS registrations
            INNER JOIN notification_rules
              ON notification_rules.id = registrations.notification_rule_id
            LEFT JOIN tasks
              ON notification_rules.target_type = 'task'
             AND notification_rules.target_id = tasks.id
             AND tasks.deleted_at IS NULL
             AND tasks.status <> 'archived'
            LEFT JOIN subtasks
              ON notification_rules.target_type = 'subtask'
             AND notification_rules.target_id = subtasks.id
             AND subtasks.deleted_at IS NULL
             AND subtasks.status <> 'archived'
            LEFT JOIN tasks AS subtask_parent_tasks
              ON notification_rules.target_type = 'subtask'
             AND subtasks.task_id = subtask_parent_tasks.id
             AND subtask_parent_tasks.deleted_at IS NULL
             AND subtask_parent_tasks.status <> 'archived'
            WHERE registrations.deleted_at IS NULL
              AND (
                (
                  registrations.registration_status IN ('pending', 'failed')
                  AND notification_rules.enabled = 1
                  AND notification_rules.deleted_at IS NULL
                  AND notification_rules.notify_at > ?1
                  AND (
                    (
                      notification_rules.target_type = 'task'
                      AND tasks.id IS NOT NULL
                    )
                    OR (
                      notification_rules.target_type = 'subtask'
                      AND subtasks.id IS NOT NULL
                      AND subtask_parent_tasks.id IS NOT NULL
                    )
                  )
                )
                OR registrations.registration_status = 'cancel_pending'
              )
            ORDER BY CASE
                       WHEN registrations.registration_status = 'cancel_pending' THEN 0
                       ELSE 1
                     END,
                     notification_rules.notify_at ASC,
                     registrations.updated_at ASC,
                     registrations.id ASC
            LIMIT ?2
            ",
        )
        .map_err(|error| format!("ネイティブ通知OS登録ジョブクエリを準備できません: {error}"))?;

    let rows = statement
        .query_map(params![now, limit], |row| {
            let target_type_text: String = row.get(3)?;
            let target_type = WorkTargetType::from_db(&target_type_text).map_err(db_value_error)?;
            let kind_text: String = row.get(5)?;
            let registration_status_text: String = row.get(8)?;
            let registration_status =
                NotificationOsRegistrationStatus::from_db(&registration_status_text)
                    .map_err(db_value_error)?;
            let action = NotificationOsRegistrationAction::from_status(&registration_status);
            Ok(NativeNotificationOsRegistrationJob {
                id: row.get(0)?,
                notification_rule_id: row.get(1)?,
                os_registration_id: row.get(2)?,
                target: target_ref(target_type, row.get(4)?),
                kind: NotificationKind::from_db(&kind_text).map_err(db_value_error)?,
                target_title: row.get(6)?,
                notify_at: row.get(7)?,
                registration_status,
                action,
                last_attempted_at: row.get(9)?,
                last_error: row.get(10)?,
            })
        })
        .map_err(|error| format!("ネイティブ通知OS登録ジョブを取得できません: {error}"))?;

    rows.map(|row| {
        row.map_err(|error| format!("ネイティブ通知OS登録ジョブ行を読めません: {error}"))
    })
    .collect()
}

fn select_notification_failure_history(
    connection: &Connection,
    limit: i64,
) -> RepositoryResult<Vec<NotificationDeliveryAttemptRecord>> {
    let mut statement = connection
        .prepare(
            "
            SELECT attempts.id,
                   attempts.notification_rule_id,
                   attempts.target_type,
                   attempts.target_id,
                   attempts.kind,
                   attempts.notify_at,
                   attempts.attempted_at,
                   attempts.result,
                   attempts.error_message,
                   (
                     SELECT COUNT(*)
                     FROM notification_delivery_attempts AS counted_attempts
                     WHERE counted_attempts.notification_rule_id = attempts.notification_rule_id
                       AND (
                         counted_attempts.attempted_at < attempts.attempted_at
                         OR (
                           counted_attempts.attempted_at = attempts.attempted_at
                           AND counted_attempts.rowid <= attempts.rowid
                         )
                       )
                   ) AS attempt_count
            FROM notification_delivery_attempts AS attempts
            WHERE EXISTS (
              SELECT 1
              FROM notification_delivery_attempts AS failed_attempts
              WHERE failed_attempts.notification_rule_id = attempts.notification_rule_id
                AND failed_attempts.result = 'failed'
            )
            ORDER BY attempts.attempted_at DESC,
                     attempts.created_at DESC
            LIMIT ?1
            ",
        )
        .map_err(|error| format!("通知失敗履歴クエリを準備できません: {error}"))?;

    let rows = statement
        .query_map(params![limit], |row| {
            let target_type_text: String = row.get(2)?;
            let target_type = WorkTargetType::from_db(&target_type_text).map_err(db_value_error)?;
            let kind_text: String = row.get(4)?;
            let result_text: String = row.get(7)?;
            Ok(NotificationDeliveryAttemptRecord {
                id: row.get(0)?,
                notification_rule_id: row.get(1)?,
                target: target_ref(target_type, row.get(3)?),
                kind: NotificationKind::from_db(&kind_text).map_err(db_value_error)?,
                notify_at: row.get(5)?,
                attempted_at: row.get(6)?,
                result: NotificationDeliveryResult::from_db(&result_text)
                    .map_err(db_value_error)?,
                error_message: row.get(8)?,
                attempt_count: row.get(9)?,
            })
        })
        .map_err(|error| format!("通知失敗履歴を取得できません: {error}"))?;

    rows.map(|row| row.map_err(|error| format!("通知失敗履歴行を読めません: {error}")))
        .collect()
}

fn insert_notification_delivery_attempt(
    transaction: &Transaction<'_>,
    job: &NotificationJob,
    result: NotificationDeliveryResult,
    error: Option<&str>,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            INSERT INTO notification_delivery_attempts (
              id, notification_rule_id, target_type, target_id, kind, notify_at,
              attempted_at, result, error_message, created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?7)
            ",
            params![
                Uuid::new_v4().to_string(),
                job.id.as_str(),
                job.target.target_type.as_str(),
                job.target.id.as_str(),
                job.kind.as_str(),
                job.notify_at.as_str(),
                now,
                result.as_str(),
                error.map(truncate_error),
            ],
        )
        .map(|_| ())
        .map_err(|error| format!("通知送信履歴を保存できません: {error}"))
}

fn truncate_error(error: &str) -> String {
    error.chars().take(500).collect()
}

fn select_ui_preferences(connection: &Connection) -> RepositoryResult<UiPreferencesRecord> {
    let mut statement = connection
        .prepare(
            "
            SELECT key, value
            FROM ui_preferences
            WHERE key IN (?1, ?2, ?3, ?4)
            ",
        )
        .map_err(|error| format!("UI設定クエリを準備できません: {error}"))?;
    let rows = statement
        .query_map(
            params![
                UI_PREF_LEFT_PANE_OPEN,
                UI_PREF_LAST_VIEW,
                UI_PREF_LAST_TASK_LIST_ID,
                UI_PREF_CALENDAR_VIEW_MODE
            ],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .map_err(|error| format!("UI設定を取得できません: {error}"))?;

    let mut values = HashMap::new();
    for row in rows {
        let (key, value) = row.map_err(|error| format!("UI設定行を読めません: {error}"))?;
        values.insert(key, value);
    }

    Ok(UiPreferencesRecord {
        left_pane_open: normalize_ui_bool(values.get(UI_PREF_LEFT_PANE_OPEN)),
        last_view: normalize_ui_view(values.get(UI_PREF_LAST_VIEW)),
        last_task_list_id: normalize_ui_identifier(values.get(UI_PREF_LAST_TASK_LIST_ID)),
        calendar_view_mode: normalize_calendar_view_mode(values.get(UI_PREF_CALENDAR_VIEW_MODE)),
    })
}

fn upsert_ui_preference(
    transaction: &Transaction<'_>,
    key: &str,
    value: &str,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            INSERT INTO ui_preferences (key, value, updated_at)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET
              value = excluded.value,
              updated_at = excluded.updated_at
            ",
            params![key, value, now],
        )
        .map(|_| ())
        .map_err(|error| format!("UI設定を保存できません: {error}"))
}

fn normalize_ui_bool(value: Option<&String>) -> bool {
    match value.map(String::as_str) {
        Some("false") => false,
        Some("true") => true,
        _ => true,
    }
}

fn normalize_ui_view(value: Option<&String>) -> String {
    match value.map(String::as_str) {
        Some(UI_VIEW_LIST | UI_VIEW_LEGACY_TASKS) => UI_VIEW_LIST.to_string(),
        Some(UI_VIEW_TODAY) => UI_VIEW_TODAY.to_string(),
        Some(UI_VIEW_FAVORITES) => UI_VIEW_FAVORITES.to_string(),
        Some(UI_VIEW_BOARD) => UI_VIEW_BOARD.to_string(),
        Some(UI_VIEW_CALENDAR) => UI_VIEW_CALENDAR.to_string(),
        Some(UI_VIEW_POMODORO) => UI_VIEW_POMODORO.to_string(),
        Some(UI_VIEW_SETTINGS) => UI_VIEW_SETTINGS.to_string(),
        _ => UI_VIEW_LIST.to_string(),
    }
}

fn normalize_ui_identifier(value: Option<&String>) -> String {
    value
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.chars().count() <= 128)
        .unwrap_or(DEFAULT_TASK_LIST_ID)
        .to_string()
}

fn normalize_calendar_view_mode(value: Option<&String>) -> String {
    match value.map(String::as_str) {
        Some(CALENDAR_VIEW_DAY) => CALENDAR_VIEW_DAY.to_string(),
        Some(CALENDAR_VIEW_MONTH) => CALENDAR_VIEW_MONTH.to_string(),
        Some(CALENDAR_VIEW_WEEK) => CALENDAR_VIEW_WEEK.to_string(),
        _ => CALENDAR_VIEW_WEEK.to_string(),
    }
}

fn select_task_page(
    connection: &Connection,
    query: &TaskPageQuery,
) -> RepositoryResult<TaskPageRecord> {
    let mut tasks = select_task_page_tasks(connection, query)?;
    let has_more = tasks.len() > query.limit as usize;
    if has_more {
        tasks.truncate(query.limit as usize);
    }
    let next_cursor = has_more.then(|| {
        let task = tasks.last().expect("ページ上限は1以上");
        TaskPageCursor {
            completion_bucket: i64::from(task.status == WorkStatus::Done),
            sort_order: task.sort_order,
            created_at: task.created_at.clone(),
            id: task.id.clone(),
        }
    });
    let task_ids = tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
    let subtasks = select_subtasks_for_task_ids(connection, &task_ids)?;
    let rows = select_task_rows_by_ids(connection, &task_ids)?;

    Ok(TaskPageRecord {
        tasks: build_task_tree(tasks, subtasks),
        rows,
        total_count: select_task_page_count(connection, query)?,
        next_cursor,
        navigation_counts: select_task_navigation_counts(connection, &query.today_date)?,
    })
}

fn select_work_item_search_results(
    connection: &Connection,
    query: &WorkItemSearchQuery,
) -> RepositoryResult<Vec<WorkItemSearchResultRecord>> {
    let escaped_query = escape_like_pattern(&query.query.to_lowercase());
    let pattern = format!("%{escaped_query}%");
    let mut statement = connection
        .prepare(
            "
            SELECT matches.target_type,
                   matches.target_id,
                   matches.task_id,
                   matches.title,
                   matches.parent_title,
                   matches.list_id,
                   matches.list_name,
                   matches.status,
                   matches.due_date,
                   matches.due_time
            FROM (
              SELECT 'task' AS target_type,
                     tasks.id AS target_id,
                     tasks.id AS task_id,
                     tasks.title AS title,
                     NULL AS parent_title,
                     tasks.list_id AS list_id,
                     task_lists.name AS list_name,
                     tasks.status AS status,
                     tasks.due_date AS due_date,
                     tasks.due_time AS due_time,
                     tasks.updated_at AS updated_at,
                     0 AS result_order
              FROM tasks
              INNER JOIN task_lists
                ON task_lists.id = tasks.list_id
               AND task_lists.deleted_at IS NULL
              WHERE tasks.deleted_at IS NULL
                AND tasks.status <> 'archived'
                AND (
                  LOWER(tasks.title) LIKE ?1 ESCAPE '\\'
                  OR LOWER(tasks.memo) LIKE ?1 ESCAPE '\\'
                  OR EXISTS (
                    SELECT 1
                    FROM task_tags
                    INNER JOIN tags
                      ON tags.id = task_tags.tag_id
                     AND tags.deleted_at IS NULL
                    WHERE task_tags.task_id = tasks.id
                      AND task_tags.deleted_at IS NULL
                      AND LOWER(tags.name) LIKE ?1 ESCAPE '\\'
                  )
                )
              UNION ALL
              SELECT 'subtask' AS target_type,
                     subtasks.id AS target_id,
                     tasks.id AS task_id,
                     subtasks.title AS title,
                     tasks.title AS parent_title,
                     tasks.list_id AS list_id,
                     task_lists.name AS list_name,
                     subtasks.status AS status,
                     subtasks.due_date AS due_date,
                     subtasks.due_time AS due_time,
                     subtasks.updated_at AS updated_at,
                     1 AS result_order
              FROM subtasks
              INNER JOIN tasks
                ON tasks.id = subtasks.task_id
               AND tasks.deleted_at IS NULL
               AND tasks.status <> 'archived'
              INNER JOIN task_lists
                ON task_lists.id = tasks.list_id
               AND task_lists.deleted_at IS NULL
              WHERE subtasks.deleted_at IS NULL
                AND subtasks.status <> 'archived'
                AND (
                  LOWER(subtasks.title) LIKE ?1 ESCAPE '\\'
                  OR LOWER(subtasks.memo) LIKE ?1 ESCAPE '\\'
                )
            ) AS matches
            ORDER BY matches.result_order ASC,
                     matches.updated_at DESC,
                     matches.target_id ASC
            LIMIT ?2
            ",
        )
        .map_err(|error| format!("ローカル検索クエリを準備できません: {error}"))?;

    let rows = statement
        .query_map(params![pattern, query.limit], |row| {
            let target_type_text: String = row.get(0)?;
            let target_type = WorkTargetType::from_db(&target_type_text).map_err(db_value_error)?;
            Ok(WorkItemSearchResultRecord {
                target: target_ref(target_type, row.get(1)?),
                task_id: row.get(2)?,
                title: row.get(3)?,
                parent_title: row.get(4)?,
                list_id: row.get(5)?,
                list_name: row.get(6)?,
                status: WorkStatus::from_db(&row.get::<_, String>(7)?).map_err(db_value_error)?,
                due_date: row.get(8)?,
                due_time: row.get(9)?,
                tags: Vec::new(),
            })
        })
        .map_err(|error| format!("ローカル検索を実行できません: {error}"))?;
    let mut results = rows
        .map(|row| row.map_err(|error| format!("ローカル検索結果を読めません: {error}")))
        .collect::<RepositoryResult<Vec<_>>>()?;
    let tags_by_task_id = select_task_tags_by_task_ids(
        connection,
        results
            .iter()
            .map(|result| result.task_id.clone())
            .collect(),
    )?;
    for result in &mut results {
        result.tags = tags_by_task_id
            .get(&result.task_id)
            .cloned()
            .unwrap_or_default();
    }
    Ok(results)
}

fn escape_like_pattern(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn select_task_page_tasks(
    connection: &Connection,
    query: &TaskPageQuery,
) -> RepositoryResult<Vec<TaskRecord>> {
    let mut statement = connection
        .prepare(
            "
            SELECT tasks.id,
                   tasks.list_id,
                   tasks.title,
                   tasks.status,
                   tasks.is_favorite,
                   tasks.planned_start_date,
                   tasks.due_date,
                   tasks.due_time,
                   tasks.timer_target_seconds,
                   tasks.memo,
                   tasks.sort_order,
                   tasks.completed_at,
                   tasks.deleted_at,
                   tasks.created_at,
                   tasks.updated_at,
                   tasks.color_token,
                   recurrence_rules.id AS recurrence_rule_id,
                   recurrence_rules.target_type AS recurrence_target_type,
                   recurrence_rules.target_id AS recurrence_target_id,
                   recurrence_rules.frequency AS recurrence_frequency,
                   recurrence_rules.interval AS recurrence_interval,
                   recurrence_rules.deleted_at AS recurrence_deleted_at,
                   recurrence_rules.created_at AS recurrence_created_at,
                   recurrence_rules.updated_at AS recurrence_updated_at
            FROM tasks
            LEFT JOIN recurrence_rules
              ON recurrence_rules.target_type = 'task'
             AND recurrence_rules.target_id = tasks.id
             AND recurrence_rules.deleted_at IS NULL
            WHERE tasks.deleted_at IS NULL
              AND tasks.status <> 'archived'
              AND (
                (?1 = 'list' AND tasks.list_id = ?2)
                OR (?1 = 'today' AND (
                  tasks.planned_start_date = ?4
                  OR tasks.due_date = ?4
                  OR EXISTS (
                    SELECT 1
                    FROM subtasks
                    WHERE subtasks.task_id = tasks.id
                      AND subtasks.deleted_at IS NULL
                      AND (
                        subtasks.planned_start_date = ?4
                        OR subtasks.due_date = ?4
                      )
                  )
                ))
                OR (?1 = 'favorites' AND tasks.is_favorite = 1)
                OR (?1 = 'tag' AND EXISTS (
                  SELECT 1
                  FROM task_tags
                  INNER JOIN tags
                    ON tags.id = task_tags.tag_id
                   AND tags.deleted_at IS NULL
                  WHERE task_tags.task_id = tasks.id
                    AND task_tags.tag_id = ?3
                    AND task_tags.deleted_at IS NULL
                ))
                OR ?1 = 'board'
              )
              AND (
                ?5 IS NULL
                OR CASE WHEN tasks.status = 'done' THEN 1 ELSE 0 END > ?5
                OR (
                  CASE WHEN tasks.status = 'done' THEN 1 ELSE 0 END = ?5
                  AND tasks.sort_order > ?6
                )
                OR (
                  CASE WHEN tasks.status = 'done' THEN 1 ELSE 0 END = ?5
                  AND tasks.sort_order = ?6
                  AND tasks.created_at > ?7
                )
                OR (
                  CASE WHEN tasks.status = 'done' THEN 1 ELSE 0 END = ?5
                  AND tasks.sort_order = ?6
                  AND tasks.created_at = ?7
                  AND tasks.id > ?8
                )
              )
            ORDER BY CASE WHEN tasks.status = 'done' THEN 1 ELSE 0 END ASC,
                     tasks.sort_order ASC,
                     tasks.created_at ASC,
                     tasks.id ASC
            LIMIT ?9
            ",
        )
        .map_err(|error| format!("タスクページクエリを準備できません: {error}"))?;

    let cursor_completion = query.cursor.as_ref().map(|cursor| cursor.completion_bucket);
    let cursor_sort_order = query.cursor.as_ref().map(|cursor| cursor.sort_order);
    let cursor_created_at = query
        .cursor
        .as_ref()
        .map(|cursor| cursor.created_at.as_str());
    let cursor_id = query.cursor.as_ref().map(|cursor| cursor.id.as_str());
    let rows = statement
        .query_map(
            params![
                query.scope.as_str(),
                query.scope.list_id(),
                query.scope.tag_id(),
                query.today_date,
                cursor_completion,
                cursor_sort_order,
                cursor_created_at,
                cursor_id,
                query.limit + 1,
            ],
            map_task_row,
        )
        .map_err(|error| format!("タスクページを取得できません: {error}"))?;

    let mut tasks = rows
        .map(|row| row.map_err(|error| format!("タスクページ行を読めません: {error}")))
        .collect::<RepositoryResult<Vec<_>>>()?;
    attach_tags_to_tasks(connection, &mut tasks)?;
    Ok(tasks)
}

fn select_task_page_count(connection: &Connection, query: &TaskPageQuery) -> RepositoryResult<i64> {
    connection
        .query_row(
            "
            SELECT COUNT(*)
            FROM tasks
            WHERE tasks.deleted_at IS NULL
              AND tasks.status <> 'archived'
              AND (
                (?1 = 'list' AND tasks.list_id = ?2)
                OR (?1 = 'today' AND (
                  tasks.planned_start_date = ?4
                  OR tasks.due_date = ?4
                  OR EXISTS (
                    SELECT 1
                    FROM subtasks
                    WHERE subtasks.task_id = tasks.id
                      AND subtasks.deleted_at IS NULL
                      AND (
                        subtasks.planned_start_date = ?4
                        OR subtasks.due_date = ?4
                      )
                  )
                ))
                OR (?1 = 'favorites' AND tasks.is_favorite = 1)
                OR (?1 = 'tag' AND EXISTS (
                  SELECT 1
                  FROM task_tags
                  INNER JOIN tags
                    ON tags.id = task_tags.tag_id
                   AND tags.deleted_at IS NULL
                  WHERE task_tags.task_id = tasks.id
                    AND task_tags.tag_id = ?3
                    AND task_tags.deleted_at IS NULL
                ))
                OR ?1 = 'board'
              )
            ",
            params![
                query.scope.as_str(),
                query.scope.list_id(),
                query.scope.tag_id(),
                query.today_date,
            ],
            |row| row.get(0),
        )
        .map_err(|error| format!("タスクページ総件数を取得できません: {error}"))
}

fn select_task_navigation_counts(
    connection: &Connection,
    today_date: &str,
) -> RepositoryResult<TaskNavigationCountsRecord> {
    connection
        .query_row(
            "
            SELECT COALESCE(SUM(CASE
                     WHEN tasks.status <> 'done'
                      AND (
                        tasks.planned_start_date = ?1
                        OR tasks.due_date = ?1
                        OR EXISTS (
                          SELECT 1
                          FROM subtasks
                          WHERE subtasks.task_id = tasks.id
                            AND subtasks.deleted_at IS NULL
                            AND (
                              subtasks.planned_start_date = ?1
                              OR subtasks.due_date = ?1
                            )
                        )
                      )
                     THEN 1 ELSE 0 END), 0) AS today_count,
                   COALESCE(SUM(CASE WHEN tasks.is_favorite = 1 THEN 1 ELSE 0 END), 0)
                     AS favorite_count
            FROM tasks
            WHERE tasks.deleted_at IS NULL
              AND tasks.status <> 'archived'
            ",
            params![today_date],
            |row| {
                Ok(TaskNavigationCountsRecord {
                    today_count: row.get(0)?,
                    favorite_count: row.get(1)?,
                })
            },
        )
        .map_err(|error| format!("タスクナビゲーション件数を取得できません: {error}"))
}

fn select_task_rows_by_ids(
    connection: &Connection,
    task_ids: &[String],
) -> RepositoryResult<Vec<TaskRowRecord>> {
    if task_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = (1..=task_ids.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "
        WITH active_timer AS (
          SELECT timer_sessions.target_type,
                 timer_sessions.target_id,
                 subtasks.task_id AS parent_task_id
          FROM timer_sessions
          LEFT JOIN subtasks
            ON timer_sessions.target_type = 'subtask'
           AND timer_sessions.target_id = subtasks.id
           AND subtasks.deleted_at IS NULL
          WHERE timer_sessions.stopped_at IS NULL
            AND timer_sessions.deleted_at IS NULL
          LIMIT 1
        )
        SELECT tasks.id,
               tasks.list_id,
               COALESCE(tasks.board_column_id, 'board-todo') AS board_column_id,
               tasks.title,
               tasks.status,
               tasks.is_favorite,
               tasks.planned_start_date,
               tasks.due_date,
               tasks.due_time,
               tasks.timer_target_seconds,
               tasks.sort_order,
               tasks.completed_at,
               tasks.created_at,
               tasks.updated_at,
               COUNT(subtasks.id) AS subtask_total_count,
               COALESCE(SUM(CASE WHEN subtasks.status = 'done' THEN 1 ELSE 0 END), 0)
                 AS completed_subtask_count,
               active_timer.target_type AS active_target_type,
               active_timer.target_id AS active_target_id
        FROM tasks
        LEFT JOIN subtasks
          ON subtasks.task_id = tasks.id
         AND subtasks.deleted_at IS NULL
        LEFT JOIN active_timer
          ON (
            active_timer.target_type = 'task'
            AND active_timer.target_id = tasks.id
          )
          OR (
            active_timer.target_type = 'subtask'
            AND active_timer.parent_task_id = tasks.id
          )
        WHERE tasks.id IN ({placeholders})
        GROUP BY tasks.id,
                 tasks.list_id,
                 tasks.board_column_id,
                 tasks.title,
                 tasks.status,
                 tasks.is_favorite,
                 tasks.planned_start_date,
                 tasks.due_date,
                 tasks.due_time,
                 tasks.timer_target_seconds,
                 tasks.sort_order,
                 tasks.completed_at,
                 tasks.created_at,
                 tasks.updated_at,
                 active_timer.target_type,
                 active_timer.target_id
        ORDER BY CASE WHEN tasks.status = 'done' THEN 1 ELSE 0 END ASC,
                 tasks.sort_order ASC,
                 tasks.created_at ASC,
                 tasks.id ASC
        "
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("タスクページ行クエリを準備できません: {error}"))?;
    let rows = statement
        .query_map(params_from_iter(task_ids.iter()), map_task_read_model_row)
        .map_err(|error| format!("タスクページ行を取得できません: {error}"))?;
    let mut task_rows = rows
        .map(|row| row.map_err(|error| format!("タスクページ行を読めません: {error}")))
        .collect::<RepositoryResult<Vec<_>>>()?;
    attach_tags_to_task_rows(connection, &mut task_rows)?;
    Ok(task_rows)
}

fn select_subtasks_for_task_ids(
    connection: &Connection,
    task_ids: &[String],
) -> RepositoryResult<Vec<SubtaskRecord>> {
    if task_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = (1..=task_ids.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "
        SELECT subtasks.id, subtasks.task_id, subtasks.title, subtasks.status,
               subtasks.planned_start_date, subtasks.due_date,
               subtasks.due_time, subtasks.timer_target_seconds, subtasks.memo,
               subtasks.sort_order, subtasks.completed_at, subtasks.deleted_at,
               subtasks.created_at, subtasks.updated_at,
               recurrence_rules.id AS recurrence_rule_id,
               recurrence_rules.target_type AS recurrence_target_type,
               recurrence_rules.target_id AS recurrence_target_id,
               recurrence_rules.frequency AS recurrence_frequency,
               recurrence_rules.interval AS recurrence_interval,
               recurrence_rules.deleted_at AS recurrence_deleted_at,
               recurrence_rules.created_at AS recurrence_created_at,
               recurrence_rules.updated_at AS recurrence_updated_at
        FROM subtasks
        LEFT JOIN recurrence_rules
          ON recurrence_rules.target_type = 'subtask'
         AND recurrence_rules.target_id = subtasks.id
         AND recurrence_rules.deleted_at IS NULL
        WHERE subtasks.deleted_at IS NULL
          AND subtasks.task_id IN ({placeholders})
        ORDER BY subtasks.task_id ASC,
                 subtasks.sort_order ASC,
                 subtasks.created_at ASC,
                 subtasks.id ASC
        "
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("ページ対象サブタスククエリを準備できません: {error}"))?;
    let rows = statement
        .query_map(params_from_iter(task_ids.iter()), map_subtask_row)
        .map_err(|error| format!("ページ対象サブタスクを取得できません: {error}"))?;
    rows.map(|row| row.map_err(|error| format!("ページ対象サブタスク行を読めません: {error}")))
        .collect()
}

fn select_task_list(connection: &Connection, limit: i64) -> RepositoryResult<Vec<TaskRecord>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, list_id, title, status, is_favorite,
                   planned_start_date, due_date, due_time, timer_target_seconds, memo,
                   sort_order, completed_at, deleted_at, created_at, updated_at,
                   color_token,
                   recurrence_rule_id, recurrence_target_type, recurrence_target_id,
                   recurrence_frequency, recurrence_interval, recurrence_deleted_at,
                   recurrence_created_at, recurrence_updated_at
            FROM (
              SELECT tasks.id,
                     tasks.list_id,
                     tasks.title,
                     tasks.status,
                     tasks.is_favorite,
                     tasks.planned_start_date,
                     tasks.due_date,
                     tasks.due_time,
                     tasks.timer_target_seconds,
                     tasks.memo,
                     tasks.sort_order,
                     tasks.completed_at,
                     tasks.deleted_at,
                     tasks.created_at,
                     tasks.updated_at,
                     tasks.color_token,
                     recurrence_rules.id AS recurrence_rule_id,
                     recurrence_rules.target_type AS recurrence_target_type,
                     recurrence_rules.target_id AS recurrence_target_id,
                     recurrence_rules.frequency AS recurrence_frequency,
                     recurrence_rules.interval AS recurrence_interval,
                     recurrence_rules.deleted_at AS recurrence_deleted_at,
                     recurrence_rules.created_at AS recurrence_created_at,
                     recurrence_rules.updated_at AS recurrence_updated_at
              FROM tasks
              LEFT JOIN recurrence_rules
                ON recurrence_rules.target_type = 'task'
               AND recurrence_rules.target_id = tasks.id
               AND recurrence_rules.deleted_at IS NULL
              WHERE tasks.deleted_at IS NULL
                AND tasks.status <> 'archived'
              ORDER BY tasks.sort_order ASC, tasks.created_at ASC
              LIMIT ?1
            )
            ",
        )
        .map_err(|error| format!("タスク一覧クエリを準備できません: {error}"))?;

    let rows = statement
        .query_map(params![limit], map_task_row)
        .map_err(|error| format!("タスク一覧を取得できません: {error}"))?;

    let mut tasks = rows
        .map(|row| row.map_err(|error| format!("タスク行を読めません: {error}")))
        .collect::<RepositoryResult<Vec<_>>>()?;
    attach_tags_to_tasks(connection, &mut tasks)?;
    Ok(tasks)
}

fn select_task_lists(connection: &Connection) -> RepositoryResult<Vec<TaskListRecord>> {
    let mut statement = connection
        .prepare(
            "
            SELECT task_lists.id,
                   task_lists.name,
                   task_lists.color_token,
                   task_lists.sort_order,
                   task_lists.created_at,
                   task_lists.updated_at,
                   COUNT(tasks.id) AS task_count,
                   COALESCE(SUM(CASE WHEN tasks.status NOT IN ('done', 'archived') THEN 1 ELSE 0 END), 0)
                     AS active_task_count,
                   COALESCE(SUM(CASE WHEN tasks.status = 'done' THEN 1 ELSE 0 END), 0)
                     AS completed_task_count
            FROM task_lists
            LEFT JOIN tasks
              ON tasks.list_id = task_lists.id
             AND tasks.deleted_at IS NULL
             AND tasks.status <> 'archived'
            WHERE task_lists.deleted_at IS NULL
            GROUP BY task_lists.id,
                     task_lists.name,
                     task_lists.color_token,
                     task_lists.sort_order,
                     task_lists.created_at,
                     task_lists.updated_at
            ORDER BY task_lists.sort_order ASC,
                     task_lists.created_at ASC
            ",
        )
        .map_err(|error| format!("タスクリスト一覧クエリを準備できません: {error}"))?;

    let rows = statement
        .query_map([], |row| {
            Ok(TaskListRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                color_token: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                task_count: row.get(6)?,
                active_task_count: row.get(7)?,
                completed_task_count: row.get(8)?,
            })
        })
        .map_err(|error| format!("タスクリスト一覧を取得できません: {error}"))?;

    rows.map(|row| row.map_err(|error| format!("タスクリスト行を読めません: {error}")))
        .collect()
}

fn select_task_list_by_id(connection: &Connection, id: &str) -> RepositoryResult<TaskListRecord> {
    connection
        .query_row(
            "
            SELECT task_lists.id,
                   task_lists.name,
                   task_lists.color_token,
                   task_lists.sort_order,
                   task_lists.created_at,
                   task_lists.updated_at,
                   COUNT(tasks.id) AS task_count,
                   COALESCE(SUM(CASE WHEN tasks.status NOT IN ('done', 'archived') THEN 1 ELSE 0 END), 0)
                     AS active_task_count,
                   COALESCE(SUM(CASE WHEN tasks.status = 'done' THEN 1 ELSE 0 END), 0)
                     AS completed_task_count
            FROM task_lists
            LEFT JOIN tasks
              ON tasks.list_id = task_lists.id
             AND tasks.deleted_at IS NULL
             AND tasks.status <> 'archived'
            WHERE task_lists.id = ?1
              AND task_lists.deleted_at IS NULL
            GROUP BY task_lists.id,
                     task_lists.name,
                     task_lists.color_token,
                     task_lists.sort_order,
                     task_lists.created_at,
                     task_lists.updated_at
            ",
            params![id],
            |row| {
                Ok(TaskListRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color_token: row.get(2)?,
                    sort_order: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    task_count: row.get(6)?,
                    active_task_count: row.get(7)?,
                    completed_task_count: row.get(8)?,
                })
            },
        )
        .map_err(|error| format!("タスクリストを取得できません: {error}"))
}

fn select_task_rows(
    connection: &Connection,
    list_id: Option<&str>,
    limit: i64,
    scope: TaskRowScope,
) -> RepositoryResult<Vec<TaskRowRecord>> {
    let mut statement = connection
        .prepare(
            "
            WITH active_timer AS (
              SELECT timer_sessions.target_type,
                     timer_sessions.target_id,
                     subtasks.task_id AS parent_task_id
              FROM timer_sessions
              LEFT JOIN subtasks
                ON timer_sessions.target_type = 'subtask'
               AND timer_sessions.target_id = subtasks.id
               AND subtasks.deleted_at IS NULL
              WHERE timer_sessions.stopped_at IS NULL
                AND timer_sessions.deleted_at IS NULL
              LIMIT 1
            )
            SELECT tasks.id,
                   tasks.list_id,
                   COALESCE(tasks.board_column_id, 'board-todo') AS board_column_id,
                   tasks.title,
                   tasks.status,
                   tasks.is_favorite,
                   tasks.planned_start_date,
                   tasks.due_date,
                   tasks.due_time,
                   tasks.timer_target_seconds,
                   tasks.sort_order,
                   tasks.completed_at,
                   tasks.created_at,
                   tasks.updated_at,
                   COUNT(subtasks.id) AS subtask_total_count,
                   COALESCE(SUM(CASE WHEN subtasks.status = 'done' THEN 1 ELSE 0 END), 0)
                     AS completed_subtask_count,
                   active_timer.target_type AS active_target_type,
                   active_timer.target_id AS active_target_id
            FROM tasks
            LEFT JOIN subtasks
              ON subtasks.task_id = tasks.id
             AND subtasks.deleted_at IS NULL
            LEFT JOIN active_timer
              ON (
                active_timer.target_type = 'task'
                AND active_timer.target_id = tasks.id
              )
              OR (
                active_timer.target_type = 'subtask'
                AND active_timer.parent_task_id = tasks.id
            )
            WHERE tasks.deleted_at IS NULL
              AND (?1 IS NULL OR tasks.list_id = ?1)
              AND (
                (?2 = 'normal' AND tasks.status <> 'archived')
                OR (?2 = 'archived' AND tasks.status = 'archived')
              )
            GROUP BY tasks.id,
                     tasks.list_id,
                     tasks.board_column_id,
                     tasks.title,
                     tasks.status,
                     tasks.is_favorite,
                     tasks.planned_start_date,
                     tasks.due_date,
                     tasks.due_time,
                     tasks.timer_target_seconds,
                     tasks.sort_order,
                     tasks.completed_at,
                     tasks.created_at,
                     tasks.updated_at,
                     active_timer.target_type,
                     active_timer.target_id
            ORDER BY CASE WHEN tasks.status = 'done' THEN 1 ELSE 0 END ASC,
                     tasks.sort_order ASC,
                     tasks.created_at ASC
            LIMIT ?3
            ",
        )
        .map_err(|error| format!("タスク行Read Modelクエリを準備できません: {error}"))?;

    let rows = statement
        .query_map(
            params![list_id, scope.as_query_value(), limit],
            map_task_read_model_row,
        )
        .map_err(|error| format!("タスク行Read Modelを取得できません: {error}"))?;

    let mut task_rows = rows
        .map(|row| row.map_err(|error| format!("タスク行Read Modelを読めません: {error}")))
        .collect::<RepositoryResult<Vec<_>>>()?;
    attach_tags_to_task_rows(connection, &mut task_rows)?;
    Ok(task_rows)
}

fn select_subtasks_for_task_list(
    connection: &Connection,
    limit: i64,
) -> RepositoryResult<Vec<SubtaskRecord>> {
    let mut statement = connection
        .prepare(
            "
            WITH selected_tasks AS (
              SELECT id, sort_order
              FROM tasks
              WHERE deleted_at IS NULL
                AND status <> 'archived'
              ORDER BY sort_order ASC, created_at ASC
              LIMIT ?1
            )
            SELECT subtasks.id, subtasks.task_id, subtasks.title, subtasks.status,
                   subtasks.planned_start_date, subtasks.due_date,
                   subtasks.due_time, subtasks.timer_target_seconds, subtasks.memo, subtasks.sort_order,
                   subtasks.completed_at, subtasks.deleted_at, subtasks.created_at,
                   subtasks.updated_at,
                   recurrence_rules.id AS recurrence_rule_id,
                   recurrence_rules.target_type AS recurrence_target_type,
                   recurrence_rules.target_id AS recurrence_target_id,
                   recurrence_rules.frequency AS recurrence_frequency,
                   recurrence_rules.interval AS recurrence_interval,
                   recurrence_rules.deleted_at AS recurrence_deleted_at,
                   recurrence_rules.created_at AS recurrence_created_at,
                   recurrence_rules.updated_at AS recurrence_updated_at
            FROM subtasks
            INNER JOIN selected_tasks
              ON selected_tasks.id = subtasks.task_id
            LEFT JOIN recurrence_rules
              ON recurrence_rules.target_type = 'subtask'
             AND recurrence_rules.target_id = subtasks.id
             AND recurrence_rules.deleted_at IS NULL
            WHERE subtasks.deleted_at IS NULL
            ORDER BY selected_tasks.sort_order ASC,
                     subtasks.sort_order ASC,
                     subtasks.created_at ASC
            ",
        )
        .map_err(|error| format!("サブタスク一覧クエリを準備できません: {error}"))?;

    let rows = statement
        .query_map(params![limit], map_subtask_row)
        .map_err(|error| format!("サブタスク一覧を取得できません: {error}"))?;

    rows.map(|row| row.map_err(|error| format!("サブタスク行を読めません: {error}")))
        .collect()
}

fn attach_tags_to_tasks(connection: &Connection, tasks: &mut [TaskRecord]) -> RepositoryResult<()> {
    let task_ids = tasks.iter().map(|task| task.id.clone()).collect::<Vec<_>>();
    let mut tags_by_task_id = select_task_tags_by_task_ids(connection, task_ids)?;
    for task in tasks {
        task.tags = tags_by_task_id.remove(&task.id).unwrap_or_default();
    }
    Ok(())
}

fn attach_tags_to_task_rows(
    connection: &Connection,
    task_rows: &mut [TaskRowRecord],
) -> RepositoryResult<()> {
    let task_ids = task_rows
        .iter()
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    let mut tags_by_task_id = select_task_tags_by_task_ids(connection, task_ids)?;
    for task_row in task_rows {
        task_row.tags = tags_by_task_id.remove(&task_row.id).unwrap_or_default();
    }
    Ok(())
}

fn select_task_tags_by_task_ids(
    connection: &Connection,
    mut task_ids: Vec<String>,
) -> RepositoryResult<HashMap<String, Vec<TaskTagRecord>>> {
    task_ids.sort();
    task_ids.dedup();
    if task_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = (1..=task_ids.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "
        SELECT task_tags.task_id,
               tags.id,
               tags.name
        FROM task_tags
        INNER JOIN tags
          ON tags.id = task_tags.tag_id
         AND tags.deleted_at IS NULL
        WHERE task_tags.deleted_at IS NULL
          AND task_tags.task_id IN ({placeholders})
        ORDER BY task_tags.task_id ASC,
                 tags.sort_order ASC,
                 tags.created_at ASC
        "
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("タスクタグクエリを準備できません: {error}"))?;
    let rows = statement
        .query_map(params_from_iter(task_ids.iter()), |row| {
            Ok((
                row.get::<_, String>(0)?,
                TaskTagRecord {
                    id: row.get(1)?,
                    name: row.get(2)?,
                },
            ))
        })
        .map_err(|error| format!("タスクタグを取得できません: {error}"))?;

    let mut tags_by_task_id: HashMap<String, Vec<TaskTagRecord>> = HashMap::new();
    for row in rows {
        let (task_id, tag) = row.map_err(|error| format!("タスクタグ行を読めません: {error}"))?;
        tags_by_task_id.entry(task_id).or_default().push(tag);
    }
    Ok(tags_by_task_id)
}

fn build_task_tree(
    tasks: Vec<TaskRecord>,
    subtasks: Vec<SubtaskRecord>,
) -> Vec<TaskWithSubtasksRecord> {
    let mut task_index = HashMap::with_capacity(tasks.len());
    let mut task_tree = tasks
        .into_iter()
        .enumerate()
        .map(|(index, task)| {
            task_index.insert(task.id.clone(), index);
            TaskWithSubtasksRecord {
                task,
                subtasks: Vec::new(),
            }
        })
        .collect::<Vec<_>>();

    for subtask in subtasks {
        if let Some(index) = task_index.get(&subtask.task_id) {
            task_tree[*index].subtasks.push(subtask);
        }
    }

    task_tree
}

fn ensure_task_exists(connection: &Connection, task_id: &str) -> RepositoryResult<()> {
    let exists: bool = connection
        .query_row(
            "
            SELECT EXISTS(
              SELECT 1
              FROM tasks
              WHERE id = ?1
                AND deleted_at IS NULL
            )
            ",
            params![task_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("親タスクを確認できません: {error}"))?;

    if exists {
        Ok(())
    } else {
        Err("親タスクが存在しません".to_string())
    }
}

fn ensure_task_list_exists(connection: &Connection, list_id: &str) -> RepositoryResult<()> {
    let exists: bool = connection
        .query_row(
            "
            SELECT EXISTS(
              SELECT 1
              FROM task_lists
              WHERE id = ?1
                AND deleted_at IS NULL
            )
            ",
            params![list_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("タスクリストを確認できません: {error}"))?;

    if exists {
        Ok(())
    } else {
        Err("タスクリストが存在しません".to_string())
    }
}

fn ensure_board_column_exists(connection: &Connection, column_id: &str) -> RepositoryResult<()> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM board_columns WHERE id = ?1 AND deleted_at IS NULL)",
            params![column_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("状態を確認できません: {error}"))?;
    if exists {
        Ok(())
    } else {
        Err("状態が存在しません".to_string())
    }
}

fn ensure_unique_board_column_title(
    connection: &Connection,
    title: &str,
    except_id: Option<&str>,
) -> RepositoryResult<()> {
    let existing_id = connection
        .query_row(
            "SELECT id FROM board_columns WHERE lower(title) = lower(?1) AND deleted_at IS NULL LIMIT 1",
            params![title],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("状態名の重複を確認できません: {error}"))?;
    if let Some(existing_id) = existing_id {
        if except_id != Some(existing_id.as_str()) {
            return Err("同じ名前の状態がすでに存在します".to_string());
        }
    }
    Ok(())
}

fn select_active_board_column_ids(connection: &Connection) -> RepositoryResult<Vec<String>> {
    let mut statement = connection
        .prepare(
            "SELECT id FROM board_columns WHERE deleted_at IS NULL ORDER BY sort_order, created_at",
        )
        .map_err(|error| format!("状態ID一覧を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("状態ID一覧を取得できません: {error}"))?;
    rows.map(|row| row.map_err(|error| format!("状態IDを読めません: {error}")))
        .collect()
}

fn select_preferred_active_board_column_id(
    connection: &Connection,
    preferred_id: &str,
) -> RepositoryResult<String> {
    connection
        .query_row(
            "
            SELECT id
            FROM board_columns
            WHERE deleted_at IS NULL
            ORDER BY CASE WHEN id = ?1 THEN 0 ELSE 1 END,
                     sort_order,
                     created_at
            LIMIT 1
            ",
            params![preferred_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("タスクの初期状態を取得できません: {error}"))
}

fn select_board_columns(connection: &Connection) -> RepositoryResult<Vec<BoardColumnRecord>> {
    let mut statement = connection
        .prepare(
            "
            SELECT board_columns.id, board_columns.title, board_columns.sort_order,
                   board_columns.created_at, board_columns.updated_at,
                   COUNT(tasks.id),
                   COALESCE(SUM(CASE WHEN tasks.lifecycle_status = 'active' THEN 1 ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN tasks.lifecycle_status = 'done' THEN 1 ELSE 0 END), 0)
            FROM board_columns
            LEFT JOIN tasks
              ON tasks.board_column_id = board_columns.id
             AND tasks.deleted_at IS NULL
             AND tasks.lifecycle_status <> 'archived'
            WHERE board_columns.deleted_at IS NULL
            GROUP BY board_columns.id, board_columns.title, board_columns.sort_order,
                     board_columns.created_at, board_columns.updated_at
            ORDER BY board_columns.sort_order, board_columns.created_at
            ",
        )
        .map_err(|error| format!("状態一覧クエリを準備できません: {error}"))?;
    let rows = statement
        .query_map([], map_board_column_row)
        .map_err(|error| format!("状態一覧を取得できません: {error}"))?;
    rows.map(|row| row.map_err(|error| format!("状態一覧を読めません: {error}")))
        .collect()
}

fn select_board_column_by_id(
    connection: &Connection,
    column_id: &str,
) -> RepositoryResult<BoardColumnRecord> {
    select_board_columns(connection)?
        .into_iter()
        .find(|column| column.id == column_id)
        .ok_or_else(|| "状態が存在しません".to_string())
}

fn map_board_column_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BoardColumnRecord> {
    Ok(BoardColumnRecord {
        id: row.get(0)?,
        title: row.get(1)?,
        sort_order: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
        task_count: row.get(5)?,
        active_task_count: row.get(6)?,
        completed_task_count: row.get(7)?,
    })
}

fn normalize_board_column_sort_order(
    transaction: &Transaction<'_>,
    now: &str,
) -> RepositoryResult<()> {
    for (sort_order, column_id) in select_active_board_column_ids(transaction)?
        .iter()
        .enumerate()
    {
        transaction
            .execute(
                "UPDATE board_columns SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
                params![sort_order as i64, now, column_id],
            )
            .map_err(|error| format!("状態の並び順を正規化できません: {error}"))?;
    }
    Ok(())
}

fn legacy_active_status_for_column(column_id: &str) -> &'static str {
    if column_id == DEFAULT_BOARD_COLUMN_ID {
        "todo"
    } else {
        "in_progress"
    }
}

fn ensure_custom_task_list(connection: &Connection, list_id: &str) -> RepositoryResult<()> {
    if list_id == DEFAULT_TASK_LIST_ID {
        return Err("初期タスクリストは変更または削除できません".to_string());
    }
    ensure_task_list_exists(connection, list_id)
}

fn ensure_unique_task_list_name(
    connection: &Connection,
    name: &str,
    except_id: Option<&str>,
) -> RepositoryResult<()> {
    let existing_id = connection
        .query_row(
            "
            SELECT id
            FROM task_lists
            WHERE name = ?1
              AND deleted_at IS NULL
            LIMIT 1
            ",
            params![name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("タスクリスト名の重複を確認できません: {error}"))?;

    if let Some(existing_id) = existing_id {
        if except_id != Some(existing_id.as_str()) {
            return Err("同じ名前のタスクリストがすでに存在します".to_string());
        }
    }

    Ok(())
}

fn select_tag_by_id(connection: &Connection, tag_id: &str) -> RepositoryResult<TagRecord> {
    connection
        .query_row(
            "
            SELECT tags.id,
                   tags.name,
                   tags.sort_order,
                   tags.created_at,
                   tags.updated_at,
                   COUNT(tasks.id) AS task_count
            FROM tags
            LEFT JOIN task_tags
              ON task_tags.tag_id = tags.id
             AND task_tags.deleted_at IS NULL
            LEFT JOIN tasks
              ON tasks.id = task_tags.task_id
             AND tasks.deleted_at IS NULL
             AND tasks.status <> 'archived'
            WHERE tags.id = ?1
              AND tags.deleted_at IS NULL
            GROUP BY tags.id,
                     tags.name,
                     tags.sort_order,
                     tags.created_at,
                     tags.updated_at
            ",
            params![tag_id],
            |row| {
                Ok(TagRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    sort_order: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    task_count: row.get(5)?,
                })
            },
        )
        .map_err(|error| format!("タグを取得できません: {error}"))
}

fn select_task_tag_by_id(connection: &Connection, tag_id: &str) -> RepositoryResult<TaskTagRecord> {
    connection
        .query_row(
            "
            SELECT id, name
            FROM tags
            WHERE id = ?1
              AND deleted_at IS NULL
            ",
            params![tag_id],
            |row| {
                Ok(TaskTagRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            },
        )
        .map_err(|error| format!("タスクタグを取得できません: {error}"))
}

fn ensure_tag_exists(connection: &Connection, tag_id: &str) -> RepositoryResult<()> {
    let exists: bool = connection
        .query_row(
            "
            SELECT EXISTS(
              SELECT 1
              FROM tags
              WHERE id = ?1
                AND deleted_at IS NULL
            )
            ",
            params![tag_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("タグを確認できません: {error}"))?;

    if exists {
        Ok(())
    } else {
        Err("タグが存在しません".to_string())
    }
}

fn ensure_unique_tag_name(
    connection: &Connection,
    name: &str,
    except_id: Option<&str>,
) -> RepositoryResult<()> {
    let existing_id = connection
        .query_row(
            "
            SELECT id
            FROM tags
            WHERE lower(name) = lower(?1)
              AND deleted_at IS NULL
            LIMIT 1
            ",
            params![name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("タグ名の重複を確認できません: {error}"))?;

    if let Some(existing_id) = existing_id {
        if except_id != Some(existing_id.as_str()) {
            return Err("同じ名前のタグがすでに存在します".to_string());
        }
    }

    Ok(())
}

fn ensure_subtask_exists(connection: &Connection, subtask_id: &str) -> RepositoryResult<()> {
    let exists: bool = connection
        .query_row(
            "
            SELECT EXISTS(
              SELECT 1
              FROM subtasks
              WHERE id = ?1
                AND deleted_at IS NULL
            )
            ",
            params![subtask_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("サブタスクを確認できません: {error}"))?;

    if exists {
        Ok(())
    } else {
        Err("サブタスクが存在しません".to_string())
    }
}

fn count_incomplete_subtasks(connection: &Connection, task_id: &str) -> RepositoryResult<i64> {
    connection
        .query_row(
            "
            SELECT COUNT(*)
            FROM subtasks
            WHERE task_id = ?1
              AND status <> 'done'
              AND deleted_at IS NULL
            ",
            params![task_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("未完了サブタスク数を取得できません: {error}"))
}

fn soft_delete_task_graph(
    transaction: &Transaction<'_>,
    task_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE tasks
            SET deleted_at = ?1,
                updated_at = ?1
            WHERE id = ?2
              AND deleted_at IS NULL
            ",
            params![now, task_id],
        )
        .map_err(|error| format!("タスクを削除できません: {error}"))?;

    transaction
        .execute(
            "
            UPDATE subtasks
            SET deleted_at = ?1,
                updated_at = ?1
            WHERE task_id = ?2
              AND deleted_at IS NULL
            ",
            params![now, task_id],
        )
        .map_err(|error| format!("子サブタスクを削除できません: {error}"))?;

    soft_delete_timer_sessions_for_task_graph(transaction, task_id, now)?;
    soft_delete_timer_pauses_for_task_graph(transaction, task_id, now)?;
    soft_delete_pomodoro_sessions_for_task_graph(transaction, task_id, now)?;
    soft_delete_notification_rules_for_task_graph(transaction, task_id, now)?;
    soft_delete_recurrence_rules_for_task_graph(transaction, task_id, now)?;
    transaction
        .execute(
            "
            UPDATE task_tags
            SET deleted_at = ?1
            WHERE task_id = ?2
              AND deleted_at IS NULL
            ",
            params![now, task_id],
        )
        .map(|_| ())
        .map_err(|error| format!("タスクのタグ関連を削除できません: {error}"))
}

fn soft_delete_subtask_graph(
    transaction: &Transaction<'_>,
    subtask_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE subtasks
            SET deleted_at = ?1,
                updated_at = ?1
            WHERE id = ?2
              AND deleted_at IS NULL
            ",
            params![now, subtask_id],
        )
        .map_err(|error| format!("サブタスクを削除できません: {error}"))?;

    transaction
        .execute(
            "
            UPDATE timer_sessions
            SET deleted_at = ?1
            WHERE target_type = 'subtask'
              AND target_id = ?2
              AND deleted_at IS NULL
            ",
            params![now, subtask_id],
        )
        .map_err(|error| format!("サブタスクのタイマー履歴を削除できません: {error}"))?;

    transaction
        .execute(
            "
            UPDATE timer_pauses
            SET deleted_at = ?1
            WHERE timer_session_id IN (
                SELECT id
                FROM timer_sessions
                WHERE target_type = 'subtask'
                  AND target_id = ?2
            )
              AND deleted_at IS NULL
            ",
            params![now, subtask_id],
        )
        .map_err(|error| format!("サブタスクの一時停止履歴を削除できません: {error}"))?;

    mark_notification_os_registrations_cancel_pending_for_subtask(transaction, subtask_id, now)?;
    transaction
        .execute(
            "
            UPDATE notification_rules
            SET enabled = 0,
                registration_status = 'disabled',
                deleted_at = ?1,
                updated_at = ?1
            WHERE target_type = 'subtask'
              AND target_id = ?2
              AND deleted_at IS NULL
            ",
            params![now, subtask_id],
        )
        .map(|_| ())
        .map_err(|error| format!("サブタスクの通知ルールを削除できません: {error}"))?;

    transaction
        .execute(
            "
            UPDATE pomodoro_sessions
            SET deleted_at = ?1,
                updated_at = ?1
            WHERE target_type = 'subtask'
              AND target_id = ?2
              AND deleted_at IS NULL
            ",
            params![now, subtask_id],
        )
        .map_err(|error| format!("サブタスクのポモドーロ履歴を削除できません: {error}"))?;

    transaction
        .execute(
            "
            UPDATE recurrence_rules
            SET deleted_at = ?1,
                updated_at = ?1
            WHERE target_type = 'subtask'
              AND target_id = ?2
              AND deleted_at IS NULL
            ",
            params![now, subtask_id],
        )
        .map(|_| ())
        .map_err(|error| format!("サブタスクの繰り返し設定を削除できません: {error}"))
}

fn soft_delete_timer_sessions_for_task_graph(
    transaction: &Transaction<'_>,
    task_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE timer_sessions
            SET deleted_at = ?1
            WHERE deleted_at IS NULL
              AND (
                (target_type = 'task' AND target_id = ?2)
                OR (
                  target_type = 'subtask'
                  AND target_id IN (
                    SELECT id
                    FROM subtasks
                    WHERE task_id = ?2
                  )
                )
              )
            ",
            params![now, task_id],
        )
        .map(|_| ())
        .map_err(|error| format!("関連タイマー履歴を削除できません: {error}"))
}

fn soft_delete_pomodoro_sessions_for_task_graph(
    transaction: &Transaction<'_>,
    task_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE pomodoro_sessions
            SET deleted_at = ?1,
                updated_at = ?1
            WHERE deleted_at IS NULL
              AND (
                (target_type = 'task' AND target_id = ?2)
                OR (
                  target_type = 'subtask'
                  AND target_id IN (
                    SELECT id
                    FROM subtasks
                    WHERE task_id = ?2
                  )
                )
              )
            ",
            params![now, task_id],
        )
        .map(|_| ())
        .map_err(|error| format!("関連ポモドーロ履歴を削除できません: {error}"))
}

fn soft_delete_notification_rules_for_task_graph(
    transaction: &Transaction<'_>,
    task_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    mark_notification_os_registrations_cancel_pending_for_task_graph(transaction, task_id, now)?;
    transaction
        .execute(
            "
            UPDATE notification_rules
            SET enabled = 0,
                registration_status = 'disabled',
                deleted_at = ?1,
                updated_at = ?1
            WHERE deleted_at IS NULL
              AND (
                (target_type = 'task' AND target_id = ?2)
                OR (
                  target_type = 'subtask'
                  AND target_id IN (
                    SELECT id
                    FROM subtasks
                    WHERE task_id = ?2
                  )
                )
              )
            ",
            params![now, task_id],
        )
        .map(|_| ())
        .map_err(|error| format!("関連通知ルールを削除できません: {error}"))
}

fn soft_delete_timer_pauses_for_task_graph(
    transaction: &Transaction<'_>,
    task_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE timer_pauses
            SET deleted_at = ?1
            WHERE deleted_at IS NULL
              AND timer_session_id IN (
                SELECT id
                FROM timer_sessions
                WHERE (target_type = 'task' AND target_id = ?2)
                   OR (
                     target_type = 'subtask'
                     AND target_id IN (
                       SELECT id
                       FROM subtasks
                       WHERE task_id = ?2
                     )
                   )
              )
            ",
            params![now, task_id],
        )
        .map(|_| ())
        .map_err(|error| format!("関連一時停止履歴を削除できません: {error}"))
}

fn soft_delete_recurrence_rules_for_task_graph(
    transaction: &Transaction<'_>,
    task_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE recurrence_rules
            SET deleted_at = ?1,
                updated_at = ?1
            WHERE deleted_at IS NULL
              AND (
                (target_type = 'task' AND target_id = ?2)
                OR (
                  target_type = 'subtask'
                  AND target_id IN (
                    SELECT id
                    FROM subtasks
                    WHERE task_id = ?2
                  )
                )
              )
            ",
            params![now, task_id],
        )
        .map(|_| ())
        .map_err(|error| format!("関連繰り返し設定を削除できません: {error}"))
}

fn ensure_no_active_timer(connection: &Connection) -> RepositoryResult<()> {
    let exists: bool = connection
        .query_row(
            "
            SELECT EXISTS(
              SELECT 1
              FROM timer_sessions
              WHERE stopped_at IS NULL
                AND deleted_at IS NULL
            )
            ",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("アクティブタイマーを確認できません: {error}"))?;

    if exists {
        Err("すでに開始中のタイマーがあります".to_string())
    } else {
        Ok(())
    }
}

fn ensure_no_active_pomodoro(connection: &Connection) -> RepositoryResult<()> {
    let exists: bool = connection
        .query_row(
            "
            SELECT EXISTS(
              SELECT 1
              FROM pomodoro_sessions
              WHERE status IN ('running', 'paused')
                AND deleted_at IS NULL
            )
            ",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("アクティブポモドーロを確認できません: {error}"))?;

    if exists {
        Err("すでに開始中のポモドーロがあります".to_string())
    } else {
        Ok(())
    }
}

fn ensure_no_active_pomodoro_except(
    connection: &Connection,
    allowed_id: &str,
) -> RepositoryResult<()> {
    let active_id: Option<String> = connection
        .query_row(
            "
            SELECT id
            FROM pomodoro_sessions
            WHERE status IN ('running', 'paused')
              AND deleted_at IS NULL
            LIMIT 1
            ",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("アクティブポモドーロを確認できません: {error}"))?;

    match active_id {
        Some(active_id) if active_id != allowed_id => {
            Err("すでに開始中のポモドーロがあります".to_string())
        }
        _ => Ok(()),
    }
}

fn next_task_sort_order(connection: &Connection, list_id: &str) -> RepositoryResult<i64> {
    connection
        .query_row(
            "
            SELECT COALESCE(MAX(sort_order), -1) + 1
            FROM tasks
            WHERE list_id = ?1
              AND deleted_at IS NULL
            ",
            params![list_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("タスクの並び順を取得できません: {error}"))
}

fn next_task_list_sort_order(connection: &Connection) -> RepositoryResult<i64> {
    connection
        .query_row(
            "
            SELECT COALESCE(MAX(sort_order), -1) + 1
            FROM task_lists
            WHERE deleted_at IS NULL
            ",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("タスクリストの並び順を取得できません: {error}"))
}

fn next_tag_sort_order(connection: &Connection) -> RepositoryResult<i64> {
    connection
        .query_row(
            "
            SELECT COALESCE(MAX(sort_order), -1) + 1
            FROM tags
            WHERE deleted_at IS NULL
            ",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("タグの並び順を取得できません: {error}"))
}

fn next_subtask_sort_order(connection: &Connection, task_id: &str) -> RepositoryResult<i64> {
    connection
        .query_row(
            "
            SELECT COALESCE(MAX(sort_order), -1) + 1
            FROM subtasks
            WHERE task_id = ?1
              AND deleted_at IS NULL
            ",
            params![task_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("サブタスクの並び順を取得できません: {error}"))
}

fn find_target_status(
    connection: &Connection,
    target: &WorkTargetRef,
) -> RepositoryResult<Option<WorkStatus>> {
    match target.target_type {
        WorkTargetType::Task => query_work_status(
            connection,
            "SELECT status FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
            &target.id,
        ),
        WorkTargetType::Subtask => query_work_status(
            connection,
            "
            SELECT CASE
                     WHEN tasks.status = 'archived' THEN 'archived'
                     ELSE subtasks.status
                   END
            FROM subtasks
            INNER JOIN tasks
              ON tasks.id = subtasks.task_id
             AND tasks.deleted_at IS NULL
            WHERE subtasks.id = ?1
              AND subtasks.deleted_at IS NULL
            ",
            &target.id,
        ),
    }
}

fn select_effective_task_timer_seconds(
    connection: &Connection,
    target: &WorkTargetRef,
) -> RepositoryResult<i64> {
    let target_seconds = match target.target_type {
        WorkTargetType::Task => connection.query_row(
            "SELECT timer_target_seconds FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
            params![target.id.as_str()],
            |row| row.get::<_, Option<i64>>(0),
        ),
        WorkTargetType::Subtask => connection.query_row(
            "SELECT timer_target_seconds FROM subtasks WHERE id = ?1 AND deleted_at IS NULL",
            params![target.id.as_str()],
            |row| row.get::<_, Option<i64>>(0),
        ),
    }
    .map_err(|error| format!("タイマー目標時間を取得できません: {error}"))?;

    let effective_seconds =
        target_seconds.unwrap_or(select_task_timer_settings(connection)?.default_target_seconds);
    if !(MIN_TASK_TIMER_TARGET_SECONDS..=MAX_TASK_TIMER_TARGET_SECONDS).contains(&effective_seconds)
    {
        return Err("タイマー目標時間は1分以上24時間以内で設定してください".to_string());
    }
    Ok(effective_seconds)
}

fn ensure_no_active_timer_for_task_graph(
    connection: &Connection,
    task_id: &str,
) -> RepositoryResult<()> {
    let exists: bool = connection
        .query_row(
            "
            SELECT EXISTS(
              SELECT 1
              FROM timer_sessions
              LEFT JOIN subtasks
                ON timer_sessions.target_type = 'subtask'
               AND timer_sessions.target_id = subtasks.id
               AND subtasks.deleted_at IS NULL
              WHERE timer_sessions.stopped_at IS NULL
                AND timer_sessions.deleted_at IS NULL
                AND (
                  (
                    timer_sessions.target_type = 'task'
                    AND timer_sessions.target_id = ?1
                  )
                  OR (
                    timer_sessions.target_type = 'subtask'
                    AND subtasks.task_id = ?1
                  )
                )
            )
            ",
            params![task_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("関連アクティブタイマーを確認できません: {error}"))?;

    if exists {
        Err("タイマー開始中のタスクはアーカイブできません".to_string())
    } else {
        Ok(())
    }
}

fn ensure_no_active_pomodoro_for_task_graph(
    connection: &Connection,
    task_id: &str,
) -> RepositoryResult<()> {
    let exists: bool = connection
        .query_row(
            "
            SELECT EXISTS(
              SELECT 1
              FROM pomodoro_sessions
              LEFT JOIN subtasks
                ON pomodoro_sessions.target_type = 'subtask'
               AND pomodoro_sessions.target_id = subtasks.id
               AND subtasks.deleted_at IS NULL
              WHERE pomodoro_sessions.status IN ('running', 'paused')
                AND pomodoro_sessions.deleted_at IS NULL
                AND (
                  (
                    pomodoro_sessions.target_type = 'task'
                    AND pomodoro_sessions.target_id = ?1
                  )
                  OR (
                    pomodoro_sessions.target_type = 'subtask'
                    AND subtasks.task_id = ?1
                  )
                )
            )
            ",
            params![task_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("関連アクティブポモドーロを確認できません: {error}"))?;

    if exists {
        Err("ポモドーロ開始中のタスクはアーカイブできません".to_string())
    } else {
        Ok(())
    }
}

fn query_work_status(
    connection: &Connection,
    sql: &str,
    id: &str,
) -> RepositoryResult<Option<WorkStatus>> {
    connection
        .query_row(sql, params![id], |row| {
            WorkStatus::from_db(&row.get::<_, String>(0)?).map_err(db_value_error)
        })
        .optional()
        .map_err(|error| format!("作業対象の状態を取得できません: {error}"))
}

fn mark_target_in_progress(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    now: &str,
) -> RepositoryResult<()> {
    match target.target_type {
        WorkTargetType::Task => {
            let progress_column_id =
                select_preferred_active_board_column_id(transaction, IN_PROGRESS_BOARD_COLUMN_ID)?;
            let progress_status = legacy_active_status_for_column(&progress_column_id);
            transaction
                .execute(
                    "
                    UPDATE tasks
                    SET status = CASE
                          WHEN board_column_id IS NULL OR board_column_id = ?3 THEN ?5
                          ELSE 'in_progress'
                        END,
                        lifecycle_status = 'active',
                        board_column_id = CASE
                          WHEN board_column_id IS NULL OR board_column_id = ?3 THEN ?4
                          ELSE board_column_id
                        END,
                        updated_at = ?1
                    WHERE id = ?2
                      AND deleted_at IS NULL
                    ",
                    params![
                        now,
                        target.id,
                        DEFAULT_BOARD_COLUMN_ID,
                        progress_column_id,
                        progress_status
                    ],
                )
                .map(|_| ())
                .map_err(|error| format!("作業対象を進行中に更新できません: {error}"))
        }
        WorkTargetType::Subtask => transaction
            .execute(
                "
                    UPDATE subtasks
                    SET status = 'in_progress',
                        updated_at = ?1
                    WHERE id = ?2
                      AND deleted_at IS NULL
                      AND status <> 'in_progress'
                    ",
                params![now, target.id],
            )
            .map(|_| ())
            .map_err(|error| format!("作業対象を進行中に更新できません: {error}")),
    }
}

fn select_task_by_id(connection: &Connection, id: &str) -> RepositoryResult<TaskRecord> {
    let mut task = connection
        .query_row(
            "
            SELECT id, list_id, title, status, is_favorite,
                   planned_start_date, due_date, due_time, timer_target_seconds, memo,
                   sort_order, completed_at, deleted_at, created_at, updated_at,
                   color_token,
                   recurrence_rule_id, recurrence_target_type, recurrence_target_id,
                   recurrence_frequency, recurrence_interval, recurrence_deleted_at,
                   recurrence_created_at, recurrence_updated_at
            FROM (
              SELECT tasks.id,
                     tasks.list_id,
                     tasks.title,
                     tasks.status,
                     tasks.is_favorite,
                     tasks.planned_start_date,
                     tasks.due_date,
                     tasks.due_time,
                     tasks.timer_target_seconds,
                     tasks.memo,
                     tasks.sort_order,
                     tasks.completed_at,
                     tasks.deleted_at,
                     tasks.created_at,
                     tasks.updated_at,
                     tasks.color_token,
                     recurrence_rules.id AS recurrence_rule_id,
                     recurrence_rules.target_type AS recurrence_target_type,
                     recurrence_rules.target_id AS recurrence_target_id,
                     recurrence_rules.frequency AS recurrence_frequency,
                     recurrence_rules.interval AS recurrence_interval,
                     recurrence_rules.deleted_at AS recurrence_deleted_at,
                     recurrence_rules.created_at AS recurrence_created_at,
                     recurrence_rules.updated_at AS recurrence_updated_at
              FROM tasks
              LEFT JOIN recurrence_rules
                ON recurrence_rules.target_type = 'task'
               AND recurrence_rules.target_id = tasks.id
               AND recurrence_rules.deleted_at IS NULL
              WHERE tasks.id = ?1
            )
            ",
            params![id],
            map_task_row,
        )
        .map_err(|error| format!("タスクを取得できません: {error}"))?;
    let mut tags = select_task_tags_by_task_ids(connection, vec![id.to_string()])?;
    task.tags = tags.remove(id).unwrap_or_default();
    Ok(task)
}

fn select_existing_task_by_id(connection: &Connection, id: &str) -> RepositoryResult<TaskRecord> {
    let mut task = connection
        .query_row(
            "
            SELECT id, list_id, title, status, is_favorite,
                   planned_start_date, due_date, due_time, timer_target_seconds, memo,
                   sort_order, completed_at, deleted_at, created_at, updated_at,
                   color_token,
                   recurrence_rule_id, recurrence_target_type, recurrence_target_id,
                   recurrence_frequency, recurrence_interval, recurrence_deleted_at,
                   recurrence_created_at, recurrence_updated_at
            FROM (
              SELECT tasks.id,
                     tasks.list_id,
                     tasks.title,
                     tasks.status,
                     tasks.is_favorite,
                     tasks.planned_start_date,
                     tasks.due_date,
                     tasks.due_time,
                     tasks.timer_target_seconds,
                     tasks.memo,
                     tasks.sort_order,
                     tasks.completed_at,
                     tasks.deleted_at,
                     tasks.created_at,
                     tasks.updated_at,
                     tasks.color_token,
                     recurrence_rules.id AS recurrence_rule_id,
                     recurrence_rules.target_type AS recurrence_target_type,
                     recurrence_rules.target_id AS recurrence_target_id,
                     recurrence_rules.frequency AS recurrence_frequency,
                     recurrence_rules.interval AS recurrence_interval,
                     recurrence_rules.deleted_at AS recurrence_deleted_at,
                     recurrence_rules.created_at AS recurrence_created_at,
                     recurrence_rules.updated_at AS recurrence_updated_at
              FROM tasks
              LEFT JOIN recurrence_rules
                ON recurrence_rules.target_type = 'task'
               AND recurrence_rules.target_id = tasks.id
               AND recurrence_rules.deleted_at IS NULL
              WHERE tasks.id = ?1
                AND tasks.deleted_at IS NULL
            )
            ",
            params![id],
            map_task_row,
        )
        .map_err(|error| format!("タスクを取得できません: {error}"))?;
    let mut tags = select_task_tags_by_task_ids(connection, vec![id.to_string()])?;
    task.tags = tags.remove(id).unwrap_or_default();
    Ok(task)
}

fn select_subtask_by_id(connection: &Connection, id: &str) -> RepositoryResult<SubtaskRecord> {
    connection
        .query_row(
            "
            SELECT id, task_id, title, status, planned_start_date, due_date, due_time,
                   timer_target_seconds, memo, sort_order, completed_at, deleted_at,
                   created_at, updated_at,
                   recurrence_rule_id, recurrence_target_type, recurrence_target_id,
                   recurrence_frequency, recurrence_interval, recurrence_deleted_at,
                   recurrence_created_at, recurrence_updated_at
            FROM (
              SELECT subtasks.id,
                     subtasks.task_id,
                     subtasks.title,
                     subtasks.status,
                     subtasks.planned_start_date,
                     subtasks.due_date,
                     subtasks.due_time,
                     subtasks.timer_target_seconds,
                     subtasks.memo,
                     subtasks.sort_order,
                     subtasks.completed_at,
                     subtasks.deleted_at,
                     subtasks.created_at,
                     subtasks.updated_at,
                     recurrence_rules.id AS recurrence_rule_id,
                     recurrence_rules.target_type AS recurrence_target_type,
                     recurrence_rules.target_id AS recurrence_target_id,
                     recurrence_rules.frequency AS recurrence_frequency,
                     recurrence_rules.interval AS recurrence_interval,
                     recurrence_rules.deleted_at AS recurrence_deleted_at,
                     recurrence_rules.created_at AS recurrence_created_at,
                     recurrence_rules.updated_at AS recurrence_updated_at
              FROM subtasks
              LEFT JOIN recurrence_rules
                ON recurrence_rules.target_type = 'subtask'
               AND recurrence_rules.target_id = subtasks.id
               AND recurrence_rules.deleted_at IS NULL
              WHERE subtasks.id = ?1
            )
            ",
            params![id],
            map_subtask_row,
        )
        .map_err(|error| format!("サブタスクを取得できません: {error}"))
}

fn select_existing_subtask_by_id(
    connection: &Connection,
    id: &str,
) -> RepositoryResult<SubtaskRecord> {
    connection
        .query_row(
            "
            SELECT id, task_id, title, status, planned_start_date, due_date, due_time,
                   timer_target_seconds, memo, sort_order, completed_at, deleted_at,
                   created_at, updated_at,
                   recurrence_rule_id, recurrence_target_type, recurrence_target_id,
                   recurrence_frequency, recurrence_interval, recurrence_deleted_at,
                   recurrence_created_at, recurrence_updated_at
            FROM (
              SELECT subtasks.id,
                     subtasks.task_id,
                     subtasks.title,
                     subtasks.status,
                     subtasks.planned_start_date,
                     subtasks.due_date,
                     subtasks.due_time,
                     subtasks.timer_target_seconds,
                     subtasks.memo,
                     subtasks.sort_order,
                     subtasks.completed_at,
                     subtasks.deleted_at,
                     subtasks.created_at,
                     subtasks.updated_at,
                     recurrence_rules.id AS recurrence_rule_id,
                     recurrence_rules.target_type AS recurrence_target_type,
                     recurrence_rules.target_id AS recurrence_target_id,
                     recurrence_rules.frequency AS recurrence_frequency,
                     recurrence_rules.interval AS recurrence_interval,
                     recurrence_rules.deleted_at AS recurrence_deleted_at,
                     recurrence_rules.created_at AS recurrence_created_at,
                     recurrence_rules.updated_at AS recurrence_updated_at
              FROM subtasks
              LEFT JOIN recurrence_rules
                ON recurrence_rules.target_type = 'subtask'
               AND recurrence_rules.target_id = subtasks.id
               AND recurrence_rules.deleted_at IS NULL
              WHERE subtasks.id = ?1
                AND subtasks.deleted_at IS NULL
            )
            ",
            params![id],
            map_subtask_row,
        )
        .map_err(|error| format!("サブタスクを取得できません: {error}"))
}

fn map_task_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRecord> {
    Ok(TaskRecord {
        id: row.get(0)?,
        list_id: row.get(1)?,
        title: row.get(2)?,
        status: WorkStatus::from_db(&row.get::<_, String>(3)?).map_err(db_value_error)?,
        is_favorite: row.get::<_, i64>(4)? != 0,
        planned_start_date: row.get(5)?,
        due_date: row.get(6)?,
        due_time: row.get(7)?,
        timer_target_seconds: row.get(8)?,
        recurrence_rule: map_optional_recurrence_rule(row, 16)?,
        memo: row.get(9)?,
        sort_order: row.get(10)?,
        completed_at: row.get(11)?,
        deleted_at: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
        color_token: row.get(15)?,
        tags: Vec::new(),
    })
}

fn map_subtask_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SubtaskRecord> {
    Ok(SubtaskRecord {
        id: row.get(0)?,
        task_id: row.get(1)?,
        title: row.get(2)?,
        status: WorkStatus::from_db(&row.get::<_, String>(3)?).map_err(db_value_error)?,
        planned_start_date: row.get(4)?,
        due_date: row.get(5)?,
        due_time: row.get(6)?,
        timer_target_seconds: row.get(7)?,
        recurrence_rule: map_optional_recurrence_rule(row, 14)?,
        memo: row.get(8)?,
        sort_order: row.get(9)?,
        completed_at: row.get(10)?,
        deleted_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn map_optional_recurrence_rule(
    row: &rusqlite::Row<'_>,
    start_index: usize,
) -> rusqlite::Result<Option<RecurrenceRuleRecord>> {
    let id: Option<String> = row.get(start_index)?;
    let Some(id) = id else {
        return Ok(None);
    };
    let target_type_text: String = row.get(start_index + 1)?;
    let target_type = WorkTargetType::from_db(&target_type_text).map_err(db_value_error)?;
    let frequency_text: String = row.get(start_index + 3)?;
    Ok(Some(RecurrenceRuleRecord {
        id,
        target: target_ref(target_type, row.get(start_index + 2)?),
        frequency: RecurrenceFrequency::from_db(&frequency_text).map_err(db_value_error)?,
        interval: row.get(start_index + 4)?,
        deleted_at: row.get(start_index + 5)?,
        created_at: row.get(start_index + 6)?,
        updated_at: row.get(start_index + 7)?,
    }))
}

fn map_task_read_model_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRowRecord> {
    let active_target_type_text: Option<String> = row.get(16)?;
    let active_target_id: Option<String> = row.get(17)?;
    let active_timer_target = match (active_target_type_text, active_target_id) {
        (Some(target_type_text), Some(target_id)) => Some(target_ref(
            WorkTargetType::from_db(&target_type_text).map_err(db_value_error)?,
            target_id,
        )),
        _ => None,
    };

    Ok(TaskRowRecord {
        id: row.get(0)?,
        list_id: row.get(1)?,
        board_column_id: row.get(2)?,
        title: row.get(3)?,
        status: WorkStatus::from_db(&row.get::<_, String>(4)?).map_err(db_value_error)?,
        is_favorite: row.get::<_, i64>(5)? != 0,
        planned_start_date: row.get(6)?,
        due_date: row.get(7)?,
        due_time: row.get(8)?,
        timer_target_seconds: row.get(9)?,
        sort_order: row.get(10)?,
        completed_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        subtask_total_count: row.get(14)?,
        completed_subtask_count: row.get(15)?,
        active_timer_target,
        tags: Vec::new(),
    })
}

fn select_active_timer(connection: &Connection) -> RepositoryResult<Option<ActiveTimer>> {
    connection
        .query_row(
            "
            SELECT timer_sessions.id,
                   timer_sessions.target_type,
                   timer_sessions.target_id,
                   timer_sessions.started_at,
                   timer_sessions.stopped_at,
                   timer_sessions.elapsed_seconds,
                   timer_sessions.deleted_at,
                   timer_sessions.created_at,
                   open_pause.paused_at,
                   timer_sessions.target_seconds,
                   COALESCE((
                     SELECT SUM(
                       CAST(strftime('%s', pauses.resumed_at) AS INTEGER) -
                       CAST(strftime('%s', pauses.paused_at) AS INTEGER)
                     )
                     FROM timer_pauses AS pauses
                     WHERE pauses.timer_session_id = timer_sessions.id
                       AND pauses.resumed_at IS NOT NULL
                       AND pauses.deleted_at IS NULL
                   ), 0),
                   timer_sessions.completion_reason,
                   timer_sessions.completion_notified_at
            FROM timer_sessions
            LEFT JOIN tasks AS task_targets
              ON timer_sessions.target_type = 'task'
             AND timer_sessions.target_id = task_targets.id
             AND task_targets.deleted_at IS NULL
             AND task_targets.status <> 'archived'
            LEFT JOIN subtasks AS subtask_targets
              ON timer_sessions.target_type = 'subtask'
             AND timer_sessions.target_id = subtask_targets.id
             AND subtask_targets.deleted_at IS NULL
             AND subtask_targets.status <> 'archived'
            LEFT JOIN tasks AS parent_tasks
              ON subtask_targets.task_id = parent_tasks.id
             AND parent_tasks.deleted_at IS NULL
             AND parent_tasks.status <> 'archived'
            LEFT JOIN timer_pauses AS open_pause
              ON open_pause.timer_session_id = timer_sessions.id
             AND open_pause.resumed_at IS NULL
             AND open_pause.deleted_at IS NULL
            WHERE stopped_at IS NULL
              AND timer_sessions.deleted_at IS NULL
              AND (
                (
                  timer_sessions.target_type = 'task'
                  AND task_targets.id IS NOT NULL
                )
                OR (
                  timer_sessions.target_type = 'subtask'
                  AND subtask_targets.id IS NOT NULL
                  AND parent_tasks.id IS NOT NULL
                )
              )
            LIMIT 1
            ",
            [],
            map_active_timer_row,
        )
        .optional()
        .map_err(|error| format!("アクティブタイマーを取得できません: {error}"))
}

fn select_active_timer_by_id(connection: &Connection, id: &str) -> RepositoryResult<ActiveTimer> {
    connection
        .query_row(
            "
            SELECT timer_sessions.id,
                   timer_sessions.target_type,
                   timer_sessions.target_id,
                   timer_sessions.started_at,
                   timer_sessions.stopped_at,
                   timer_sessions.elapsed_seconds,
                   timer_sessions.deleted_at,
                   timer_sessions.created_at,
                   open_pause.paused_at,
                   timer_sessions.target_seconds,
                   COALESCE((
                     SELECT SUM(
                       CAST(strftime('%s', pauses.resumed_at) AS INTEGER) -
                       CAST(strftime('%s', pauses.paused_at) AS INTEGER)
                     )
                     FROM timer_pauses AS pauses
                     WHERE pauses.timer_session_id = timer_sessions.id
                       AND pauses.resumed_at IS NOT NULL
                       AND pauses.deleted_at IS NULL
                   ), 0),
                   timer_sessions.completion_reason,
                   timer_sessions.completion_notified_at
            FROM timer_sessions
            LEFT JOIN timer_pauses AS open_pause
              ON open_pause.timer_session_id = timer_sessions.id
             AND open_pause.resumed_at IS NULL
             AND open_pause.deleted_at IS NULL
            WHERE timer_sessions.id = ?1
              AND timer_sessions.stopped_at IS NULL
              AND timer_sessions.deleted_at IS NULL
            ",
            params![id],
            map_active_timer_row,
        )
        .map_err(|error| format!("開始したタイマーを取得できません: {error}"))
}

fn select_timer_by_id(connection: &Connection, id: &str) -> RepositoryResult<ActiveTimer> {
    connection
        .query_row(
            "
            SELECT timer_sessions.id,
                   timer_sessions.target_type,
                   timer_sessions.target_id,
                   timer_sessions.started_at,
                   timer_sessions.stopped_at,
                   timer_sessions.elapsed_seconds,
                   timer_sessions.deleted_at,
                   timer_sessions.created_at,
                   open_pause.paused_at,
                   timer_sessions.target_seconds,
                   COALESCE((
                     SELECT SUM(
                       CAST(strftime('%s', pauses.resumed_at) AS INTEGER) -
                       CAST(strftime('%s', pauses.paused_at) AS INTEGER)
                     )
                     FROM timer_pauses AS pauses
                     WHERE pauses.timer_session_id = timer_sessions.id
                       AND pauses.resumed_at IS NOT NULL
                       AND pauses.deleted_at IS NULL
                   ), 0),
                   timer_sessions.completion_reason,
                   timer_sessions.completion_notified_at
            FROM timer_sessions
            LEFT JOIN timer_pauses AS open_pause
              ON open_pause.timer_session_id = timer_sessions.id
             AND open_pause.resumed_at IS NULL
             AND open_pause.deleted_at IS NULL
            WHERE timer_sessions.id = ?1
              AND timer_sessions.deleted_at IS NULL
            ",
            params![id],
            map_active_timer_row,
        )
        .map_err(|error| format!("停止したタイマーを取得できません: {error}"))
}

fn map_active_timer_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActiveTimer> {
    let target_type_text: String = row.get(1)?;
    let target_type = WorkTargetType::from_db(&target_type_text).map_err(db_value_error)?;
    let completion_reason_text: Option<String> = row.get(11)?;
    let completion_reason = completion_reason_text
        .as_deref()
        .map(TimerCompletionReason::from_db)
        .transpose()
        .map_err(db_value_error)?;
    Ok(ActiveTimer {
        id: row.get(0)?,
        target: target_ref(target_type, row.get(2)?),
        started_at: row.get(3)?,
        stopped_at: row.get(4)?,
        elapsed_seconds: row.get(5)?,
        paused_at: row.get(8)?,
        target_seconds: row.get(9)?,
        accumulated_paused_seconds: row.get(10)?,
        completion_reason,
        completion_notified_at: row.get(12)?,
        deleted_at: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn select_task_timer_settings(
    connection: &Connection,
) -> RepositoryResult<TaskTimerSettingsRecord> {
    connection
        .query_row(
            "
            SELECT id, default_target_seconds, updated_at
            FROM task_timer_settings
            WHERE id = ?1
            ",
            params![DEFAULT_TASK_TIMER_SETTINGS_ID],
            |row| {
                Ok(TaskTimerSettingsRecord {
                    id: row.get(0)?,
                    default_target_seconds: row.get(1)?,
                    updated_at: row.get(2)?,
                })
            },
        )
        .map_err(|error| format!("タスクタイマー設定を取得できません: {error}"))
}

fn select_pomodoro_settings(connection: &Connection) -> RepositoryResult<PomodoroSettingsRecord> {
    connection
        .query_row(
            "
            SELECT id,
                   work_seconds,
                   short_break_seconds,
                   long_break_seconds,
                   cycles_until_long_break,
                   auto_start_break,
                   auto_start_next_work,
                   updated_at
            FROM pomodoro_settings
            WHERE id = ?1
            ",
            params![DEFAULT_POMODORO_SETTINGS_ID],
            map_pomodoro_settings_row,
        )
        .map_err(|error| format!("ポモドーロ設定を取得できません: {error}"))
}

fn map_pomodoro_settings_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PomodoroSettingsRecord> {
    Ok(PomodoroSettingsRecord {
        id: row.get(0)?,
        work_seconds: row.get(1)?,
        short_break_seconds: row.get(2)?,
        long_break_seconds: row.get(3)?,
        cycles_until_long_break: row.get(4)?,
        auto_start_break: row.get::<_, i64>(5)? != 0,
        auto_start_next_work: row.get::<_, i64>(6)? != 0,
        updated_at: row.get(7)?,
    })
}

fn select_active_pomodoro(connection: &Connection) -> RepositoryResult<Option<ActivePomodoro>> {
    connection
        .query_row(
            "
            SELECT pomodoro_sessions.id,
                   pomodoro_sessions.scope,
                   pomodoro_sessions.target_type,
                   pomodoro_sessions.target_id,
                   pomodoro_sessions.timer_session_id,
                   pomodoro_sessions.phase,
                   pomodoro_sessions.status,
                   pomodoro_sessions.cycle_count,
                   pomodoro_sessions.phase_started_at,
                   pomodoro_sessions.phase_duration_seconds,
                   pomodoro_sessions.paused_at,
                   pomodoro_sessions.paused_total_seconds,
                   pomodoro_sessions.completed_at,
                   pomodoro_sessions.cancelled_at,
                   pomodoro_sessions.deleted_at,
                   pomodoro_sessions.created_at,
                   pomodoro_sessions.updated_at
            FROM pomodoro_sessions
            LEFT JOIN tasks AS task_targets
              ON pomodoro_sessions.target_type = 'task'
             AND pomodoro_sessions.target_id = task_targets.id
             AND task_targets.deleted_at IS NULL
             AND task_targets.status <> 'archived'
            LEFT JOIN subtasks AS subtask_targets
              ON pomodoro_sessions.target_type = 'subtask'
             AND pomodoro_sessions.target_id = subtask_targets.id
             AND subtask_targets.deleted_at IS NULL
             AND subtask_targets.status <> 'archived'
            LEFT JOIN tasks AS parent_tasks
              ON subtask_targets.task_id = parent_tasks.id
             AND parent_tasks.deleted_at IS NULL
             AND parent_tasks.status <> 'archived'
            WHERE pomodoro_sessions.status IN ('running', 'paused')
              AND pomodoro_sessions.deleted_at IS NULL
              AND (
                pomodoro_sessions.scope = 'standalone'
                OR
                (
                  pomodoro_sessions.scope = 'task_linked'
                  AND
                  pomodoro_sessions.target_type = 'task'
                  AND task_targets.id IS NOT NULL
                )
                OR (
                  pomodoro_sessions.scope = 'task_linked'
                  AND
                  pomodoro_sessions.target_type = 'subtask'
                  AND subtask_targets.id IS NOT NULL
                  AND parent_tasks.id IS NOT NULL
                )
              )
            LIMIT 1
            ",
            [],
            map_active_pomodoro_row,
        )
        .optional()
        .map_err(|error| format!("アクティブポモドーロを取得できません: {error}"))
}

fn select_active_pomodoro_by_id(
    connection: &Connection,
    id: &str,
) -> RepositoryResult<ActivePomodoro> {
    connection
        .query_row(
            "
            SELECT id,
                   scope,
                   target_type,
                   target_id,
                   timer_session_id,
                   phase,
                   status,
                   cycle_count,
                   phase_started_at,
                   phase_duration_seconds,
                   paused_at,
                   paused_total_seconds,
                   completed_at,
                   cancelled_at,
                   deleted_at,
                   created_at,
                   updated_at
            FROM pomodoro_sessions
            WHERE id = ?1
              AND status IN ('running', 'paused')
              AND deleted_at IS NULL
            ",
            params![id],
            map_active_pomodoro_row,
        )
        .map_err(|error| format!("開始したポモドーロを取得できません: {error}"))
}

fn select_pomodoro_session_by_id(
    connection: &Connection,
    id: &str,
) -> RepositoryResult<ActivePomodoro> {
    connection
        .query_row(
            "
            SELECT id,
                   scope,
                   target_type,
                   target_id,
                   timer_session_id,
                   phase,
                   status,
                   cycle_count,
                   phase_started_at,
                   phase_duration_seconds,
                   paused_at,
                   paused_total_seconds,
                   completed_at,
                   cancelled_at,
                   deleted_at,
                   created_at,
                   updated_at
            FROM pomodoro_sessions
            WHERE id = ?1
              AND deleted_at IS NULL
            ",
            params![id],
            map_active_pomodoro_row,
        )
        .map_err(|error| format!("ポモドーロセッションを取得できません: {error}"))
}

fn map_active_pomodoro_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActivePomodoro> {
    let scope_text: String = row.get(1)?;
    let scope = PomodoroScope::from_db(&scope_text).map_err(db_value_error)?;
    let target_type_text: Option<String> = row.get(2)?;
    let target_id: Option<String> = row.get(3)?;
    let target = match (target_type_text, target_id) {
        (Some(target_type), Some(target_id)) => Some(target_ref(
            WorkTargetType::from_db(&target_type).map_err(db_value_error)?,
            target_id,
        )),
        (None, None) => None,
        _ => {
            return Err(db_value_error(
                "ポモドーロ対象の列が矛盾しています".to_string(),
            ))
        }
    };
    if (scope == PomodoroScope::TaskLinked) != target.is_some() {
        return Err(db_value_error(
            "ポモドーロscopeと対象が矛盾しています".to_string(),
        ));
    }
    let phase_text: String = row.get(5)?;
    let status_text: String = row.get(6)?;
    Ok(ActivePomodoro {
        id: row.get(0)?,
        scope,
        target,
        timer_session_id: row.get(4)?,
        phase: PomodoroPhase::from_db(&phase_text).map_err(db_value_error)?,
        status: PomodoroStatus::from_db(&status_text).map_err(db_value_error)?,
        cycle_count: row.get(7)?,
        phase_started_at: row.get(8)?,
        phase_duration_seconds: row.get(9)?,
        paused_at: row.get(10)?,
        paused_total_seconds: row.get(11)?,
        completed_at: row.get(12)?,
        cancelled_at: row.get(13)?,
        deleted_at: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

fn ensure_pomodoro_target_available(
    connection: &Connection,
    pomodoro: &ActivePomodoro,
) -> RepositoryResult<()> {
    if pomodoro.scope == PomodoroScope::Standalone {
        return Ok(());
    }
    let target = pomodoro
        .target
        .as_ref()
        .ok_or_else(|| "タスク連携ポモドーロに対象がありません".to_string())?;
    let status = find_target_status(connection, target)?
        .ok_or_else(|| "ポモドーロ対象のタスクまたはサブタスクが存在しません".to_string())?;
    if status == WorkStatus::Archived {
        return Err("アーカイブ済みのタスクではポモドーロを継続できません".to_string());
    }

    Ok(())
}

fn ensure_pomodoro_work_startable(
    connection: &Connection,
    pomodoro: &ActivePomodoro,
) -> RepositoryResult<()> {
    if pomodoro.scope == PomodoroScope::Standalone {
        return Ok(());
    }
    let target = pomodoro
        .target
        .as_ref()
        .ok_or_else(|| "タスク連携ポモドーロに対象がありません".to_string())?;
    let status = find_target_status(connection, target)?
        .ok_or_else(|| "ポモドーロ対象のタスクまたはサブタスクが存在しません".to_string())?;
    assert_timer_startable(&status)
}

fn can_start_pomodoro_work_phase(
    connection: &Connection,
    pomodoro: &ActivePomodoro,
) -> RepositoryResult<bool> {
    if pomodoro.scope == PomodoroScope::Standalone {
        return Ok(true);
    }
    let Some(target) = pomodoro.target.as_ref() else {
        return Ok(false);
    };
    let Some(status) = find_target_status(connection, target)? else {
        return Ok(false);
    };

    Ok(assert_timer_startable(&status).is_ok())
}

fn pomodoro_phase_duration_seconds(
    settings: &PomodoroSettingsRecord,
    phase: &PomodoroPhase,
) -> i64 {
    match phase {
        PomodoroPhase::Work => settings.work_seconds,
        PomodoroPhase::ShortBreak => settings.short_break_seconds,
        PomodoroPhase::LongBreak => settings.long_break_seconds,
    }
}

fn pomodoro_phase_end_at(pomodoro: &ActivePomodoro) -> RepositoryResult<String> {
    let started_at = parse_rfc3339_timestamp(&pomodoro.phase_started_at, "ポモドーロ開始時刻")?;
    let duration_seconds = pomodoro.phase_duration_seconds + pomodoro.paused_total_seconds.max(0);
    format_rfc3339_timestamp(started_at + Duration::seconds(duration_seconds))
}

fn accumulated_pomodoro_pause_seconds(
    pomodoro: &ActivePomodoro,
    now: &str,
) -> RepositoryResult<i64> {
    let open_pause_seconds = pomodoro
        .paused_at
        .as_deref()
        .map(|paused_at| calculate_duration_seconds(paused_at, now))
        .transpose()?
        .unwrap_or(0);

    Ok(pomodoro.paused_total_seconds + open_pause_seconds)
}

fn finish_pomodoro_work(
    transaction: &Transaction<'_>,
    pomodoro: &ActivePomodoro,
    now: &str,
) -> RepositoryResult<i64> {
    if pomodoro.phase != PomodoroPhase::Work {
        return Err("作業フェーズ以外は作業タイマーを停止できません".to_string());
    }
    if pomodoro.scope == PomodoroScope::Standalone {
        return accumulated_pomodoro_pause_seconds(pomodoro, now);
    }
    let timer_id = pomodoro.timer_session_id.as_deref().ok_or_else(|| {
        "タスク連携ポモドーロの作業フェーズにタイマー履歴がありません".to_string()
    })?;
    let active_timer = select_active_timer_by_id(transaction, timer_id)?;
    close_open_pause_for_timer(transaction, timer_id, now)?;
    let paused_seconds = total_pause_seconds(transaction, timer_id, now)?;
    let (stopped_at, elapsed_seconds) =
        calculate_stop_values(&active_timer.started_at, now, paused_seconds)?;

    let updated = transaction
        .execute(
            "
            UPDATE timer_sessions
            SET stopped_at = ?1,
                elapsed_seconds = ?2
            WHERE id = ?3
              AND stopped_at IS NULL
              AND deleted_at IS NULL
            ",
            params![stopped_at, elapsed_seconds, timer_id],
        )
        .map_err(|error| format!("ポモドーロ用タイマーを停止できません: {error}"))?;
    if updated != 1 {
        return Err("開始中のポモドーロ用タイマーを停止できませんでした".to_string());
    }

    Ok(paused_seconds)
}

fn insert_pomodoro_break_phase(
    transaction: &Transaction<'_>,
    work_session: &ActivePomodoro,
    settings: &PomodoroSettingsRecord,
    now: &str,
) -> RepositoryResult<ActivePomodoro> {
    let phase = next_break_phase(work_session.cycle_count, settings.cycles_until_long_break);
    let phase_duration_seconds = pomodoro_phase_duration_seconds(settings, &phase);
    let pomodoro_id = Uuid::new_v4().to_string();
    transaction
        .execute(
            "
            INSERT INTO pomodoro_sessions (
              id, scope, target_type, target_id, timer_session_id, phase, status,
              cycle_count, phase_started_at, phase_duration_seconds,
              paused_total_seconds, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, NULL, ?5, 'running', ?6, ?7, ?8, 0, ?7, ?7)
            ",
            params![
                pomodoro_id,
                work_session.scope.as_str(),
                work_session
                    .target
                    .as_ref()
                    .map(|target| target.target_type.as_str()),
                work_session
                    .target
                    .as_ref()
                    .map(|target| target.id.as_str()),
                phase.as_str(),
                work_session.cycle_count,
                now,
                phase_duration_seconds
            ],
        )
        .map_err(|error| format!("ポモドーロ休憩を開始できません: {error}"))?;

    select_active_pomodoro_by_id(transaction, &pomodoro_id)
}

fn insert_pomodoro_work_phase(
    transaction: &Transaction<'_>,
    source: &ActivePomodoro,
    cycle_count: i64,
    now: &str,
    work_seconds: i64,
) -> RepositoryResult<ActivePomodoro> {
    let timer_id = if source.scope == PomodoroScope::TaskLinked {
        let target = source
            .target
            .as_ref()
            .ok_or_else(|| "タスク連携ポモドーロに対象がありません".to_string())?;
        let timer_id = Uuid::new_v4().to_string();
        transaction
            .execute(
                "
                INSERT INTO timer_sessions (
                  id, target_type, target_id, started_at, created_at
                )
                VALUES (?1, ?2, ?3, ?4, ?4)
                ",
                params![
                    timer_id,
                    target.target_type.as_str(),
                    target.id.as_str(),
                    now
                ],
            )
            .map_err(|error| format!("ポモドーロ用タイマーを開始できません: {error}"))?;
        Some(timer_id)
    } else {
        None
    };

    let pomodoro_id = Uuid::new_v4().to_string();
    transaction
        .execute(
            "
            INSERT INTO pomodoro_sessions (
              id, scope, target_type, target_id, timer_session_id, phase, status,
              cycle_count, phase_started_at, phase_duration_seconds,
              paused_total_seconds, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, 'work', 'running', ?6, ?7, ?8, 0, ?7, ?7)
            ",
            params![
                pomodoro_id,
                source.scope.as_str(),
                source
                    .target
                    .as_ref()
                    .map(|target| target.target_type.as_str()),
                source.target.as_ref().map(|target| target.id.as_str()),
                timer_id,
                cycle_count,
                now,
                work_seconds
            ],
        )
        .map_err(|error| format!("ポモドーロ作業フェーズを開始できません: {error}"))?;

    if let Some(target) = source.target.as_ref() {
        mark_target_in_progress(transaction, target, now)?;
    }
    select_active_pomodoro_by_id(transaction, &pomodoro_id)
}

fn pomodoro_notification_title(
    connection: &Connection,
    pomodoro: &ActivePomodoro,
) -> RepositoryResult<String> {
    match pomodoro.target.as_ref() {
        Some(target) => select_target_title(connection, target),
        None => Ok("ポモドーロ".to_string()),
    }
}

fn select_target_title(
    connection: &Connection,
    target: &WorkTargetRef,
) -> RepositoryResult<String> {
    match target.target_type {
        WorkTargetType::Task => connection
            .query_row(
                "
                SELECT title
                FROM tasks
                WHERE id = ?1
                  AND deleted_at IS NULL
                  AND status <> 'archived'
                ",
                params![target.id.as_str()],
                |row| row.get(0),
            )
            .map_err(|error| format!("タイマー対象タスク名を取得できません: {error}")),
        WorkTargetType::Subtask => connection
            .query_row(
                "
                SELECT subtasks.title
                FROM subtasks
                INNER JOIN tasks
                  ON tasks.id = subtasks.task_id
                 AND tasks.deleted_at IS NULL
                 AND tasks.status <> 'archived'
                WHERE subtasks.id = ?1
                  AND subtasks.deleted_at IS NULL
                  AND subtasks.status <> 'archived'
                ",
                params![target.id.as_str()],
                |row| row.get(0),
            )
            .map_err(|error| format!("タイマー対象サブタスク名を取得できません: {error}")),
    }
}

fn close_open_pause_for_timer(
    transaction: &Transaction<'_>,
    timer_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE timer_pauses
            SET resumed_at = ?1
            WHERE timer_session_id = ?2
              AND resumed_at IS NULL
              AND deleted_at IS NULL
            ",
            params![now, timer_id],
        )
        .map(|_| ())
        .map_err(|error| format!("一時停止区間を閉じられません: {error}"))
}

fn pause_pomodoro_for_timer(
    transaction: &Transaction<'_>,
    timer_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE pomodoro_sessions
            SET status = 'paused',
                paused_at = ?1,
                updated_at = ?1
            WHERE timer_session_id = ?2
              AND status = 'running'
              AND deleted_at IS NULL
            ",
            params![now, timer_id],
        )
        .map(|_| ())
        .map_err(|error| format!("ポモドーロを一時停止状態に更新できません: {error}"))
}

fn resume_pomodoro_for_timer(
    transaction: &Transaction<'_>,
    timer_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    let paused_at = transaction
        .query_row(
            "
            SELECT paused_at
            FROM pomodoro_sessions
            WHERE timer_session_id = ?1
              AND status = 'paused'
              AND deleted_at IS NULL
            ",
            params![timer_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|error| format!("ポモドーロ一時停止状態を取得できません: {error}"))?
        .flatten();

    let Some(paused_at) = paused_at else {
        return Ok(());
    };
    let paused_seconds = calculate_duration_seconds(&paused_at, now)?;
    transaction
        .execute(
            "
            UPDATE pomodoro_sessions
            SET status = 'running',
                paused_at = NULL,
                paused_total_seconds = paused_total_seconds + ?1,
                updated_at = ?2
            WHERE timer_session_id = ?3
              AND status = 'paused'
              AND deleted_at IS NULL
            ",
            params![paused_seconds, now, timer_id],
        )
        .map(|_| ())
        .map_err(|error| format!("ポモドーロを再開状態に更新できません: {error}"))
}

fn cancel_pomodoro_for_timer(
    transaction: &Transaction<'_>,
    timer_id: &str,
    now: &str,
) -> RepositoryResult<()> {
    transaction
        .execute(
            "
            UPDATE pomodoro_sessions
            SET status = 'cancelled',
                paused_at = NULL,
                cancelled_at = ?1,
                updated_at = ?1
            WHERE timer_session_id = ?2
              AND status IN ('running', 'paused')
              AND deleted_at IS NULL
            ",
            params![now, timer_id],
        )
        .map(|_| ())
        .map_err(|error| format!("ポモドーロを終了状態に更新できません: {error}"))
}

fn total_pause_seconds(
    connection: &Connection,
    timer_id: &str,
    stop_time: &str,
) -> RepositoryResult<i64> {
    let mut statement = connection
        .prepare(
            "
            SELECT paused_at, COALESCE(resumed_at, ?2)
            FROM timer_pauses
            WHERE timer_session_id = ?1
              AND deleted_at IS NULL
            ",
        )
        .map_err(|error| format!("一時停止区間クエリを準備できません: {error}"))?;
    let rows = statement
        .query_map(params![timer_id, stop_time], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("一時停止区間を取得できません: {error}"))?;

    let mut total_seconds = 0;
    for row in rows {
        let (paused_at, resumed_at) =
            row.map_err(|error| format!("一時停止区間を読めません: {error}"))?;
        total_seconds += calculate_duration_seconds(&paused_at, &resumed_at)?;
    }

    Ok(total_seconds)
}

fn calculate_stop_values(
    started_at: &str,
    now: &str,
    paused_seconds: i64,
) -> RepositoryResult<(String, i64)> {
    let started = OffsetDateTime::parse(started_at, &Rfc3339)
        .map_err(|error| format!("タイマー開始時刻の形式が不正です: {error}"))?;
    let stopped = OffsetDateTime::parse(now, &Rfc3339)
        .map_err(|error| format!("タイマー停止時刻の形式が不正です: {error}"))?;
    if stopped < started {
        return Ok((started_at.to_string(), 0));
    }

    let elapsed_seconds = (stopped - started).whole_seconds() - paused_seconds.max(0);
    Ok((now.to_string(), elapsed_seconds.max(0)))
}

fn calculate_duration_seconds(started_at: &str, stopped_at: &str) -> RepositoryResult<i64> {
    let started = parse_rfc3339_timestamp(started_at, "一時停止開始時刻")?;
    let stopped = parse_rfc3339_timestamp(stopped_at, "一時停止終了時刻")?;
    if stopped < started {
        return Ok(0);
    }

    Ok((stopped - started).whole_seconds())
}

fn parse_rfc3339_timestamp(value: &str, field_name: &str) -> RepositoryResult<OffsetDateTime> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|error| format!("{field_name}の形式が不正です: {error}"))
}

fn format_rfc3339_timestamp(value: OffsetDateTime) -> RepositoryResult<String> {
    value
        .format(&Rfc3339)
        .map_err(|error| format!("時刻を保存形式へ変換できません: {error}"))
}

fn configure_connection(connection: &Connection) -> RepositoryResult<()> {
    connection
        .busy_timeout(StdDuration::from_secs(5))
        .map_err(|error| format!("SQLite busy timeout設定に失敗しました: {error}"))?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|error| format!("SQLite foreign_keys設定に失敗しました: {error}"))?;
    Ok(())
}

fn run_initial_migration(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute_batch(INITIAL_SCHEMA)
        .map_err(|error| format!("SQLite初期マイグレーションに失敗しました: {error}"))?;
    run_timer_recurrence_migration(connection)?;
    run_task_countdown_migration(connection)?;
    run_pomodoro_migration(connection)?;
    run_due_time_migration(connection)?;
    run_work_schedule_migration(connection)?;
    run_ui_read_model_migration(connection)?;
    run_task_display_color_migration(connection)?;
    run_board_column_migration(connection)?;
    run_tag_migration(connection)?;
    run_notification_preference_migration(connection)?;
    run_notification_os_registration_migration(connection)?;
    run_notification_delivery_attempt_migration(connection)
}

fn run_task_display_color_migration(connection: &Connection) -> RepositoryResult<()> {
    ensure_column(
        connection,
        "tasks",
        "color_token",
        "ALTER TABLE tasks ADD COLUMN color_token TEXT NULL CHECK (color_token IS NULL OR color_token IN ('green', 'blue', 'amber', 'rose', 'violet', 'gray'))",
    )
}

fn run_task_countdown_migration(connection: &Connection) -> RepositoryResult<()> {
    ensure_column(
        connection,
        "timer_sessions",
        "target_seconds",
        "ALTER TABLE timer_sessions ADD COLUMN target_seconds INTEGER NULL CHECK (target_seconds IS NULL OR (target_seconds >= 60 AND target_seconds <= 86400))",
    )?;
    ensure_column(
        connection,
        "timer_sessions",
        "completion_reason",
        "ALTER TABLE timer_sessions ADD COLUMN completion_reason TEXT NULL CHECK (completion_reason IS NULL OR completion_reason IN ('manual', 'countdown_expired'))",
    )?;
    ensure_column(
        connection,
        "timer_sessions",
        "completion_notified_at",
        "ALTER TABLE timer_sessions ADD COLUMN completion_notified_at TEXT NULL",
    )?;
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS task_timer_settings (
              id TEXT PRIMARY KEY CHECK (id = 'default'),
              default_target_seconds INTEGER NOT NULL CHECK (
                default_target_seconds >= 60 AND default_target_seconds <= 86400
              ),
              updated_at TEXT NOT NULL
            );
            ",
        )
        .map_err(|error| format!("タスクタイマー設定テーブルを作成できません: {error}"))?;
    seed_default_task_timer_settings(connection)
}

fn run_board_column_migration(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS board_columns (
              id TEXT PRIMARY KEY,
              title TEXT NOT NULL CHECK (length(trim(title)) > 0 AND length(title) <= 80),
              sort_order INTEGER NOT NULL DEFAULT 0,
              deleted_at TEXT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE UNIQUE INDEX IF NOT EXISTS board_columns_active_title_unique_idx
            ON board_columns (lower(title))
            WHERE deleted_at IS NULL;

            CREATE INDEX IF NOT EXISTS board_columns_order_idx
            ON board_columns (sort_order, created_at)
            WHERE deleted_at IS NULL;
            ",
        )
        .map_err(|error| format!("かんばん状態テーブルを作成できません: {error}"))?;

    ensure_column(
        connection,
        "tasks",
        "board_column_id",
        "ALTER TABLE tasks ADD COLUMN board_column_id TEXT NULL REFERENCES board_columns(id) ON DELETE RESTRICT",
    )?;
    ensure_column(
        connection,
        "tasks",
        "lifecycle_status",
        "ALTER TABLE tasks ADD COLUMN lifecycle_status TEXT NOT NULL DEFAULT 'active' CHECK (lifecycle_status IN ('active', 'done', 'archived'))",
    )?;

    let now = current_timestamp_value(connection)?;
    ensure_default_board_columns(connection, &now)?;
    connection
        .execute(
            "
            UPDATE tasks
            SET lifecycle_status = CASE
              WHEN status = 'done' THEN 'done'
              WHEN status = 'archived' THEN 'archived'
              ELSE 'active'
            END
            WHERE lifecycle_status <> CASE
              WHEN status = 'done' THEN 'done'
              WHEN status = 'archived' THEN 'archived'
              ELSE 'active'
            END
            ",
            [],
        )
        .map_err(|error| format!("既存タスクの完了状態を移行できません: {error}"))?;
    connection
        .execute(
            "
            UPDATE tasks
            SET board_column_id = CASE
              WHEN status = 'in_progress' THEN ?1
              ELSE ?2
            END
            WHERE board_column_id IS NULL
            ",
            params![IN_PROGRESS_BOARD_COLUMN_ID, DEFAULT_BOARD_COLUMN_ID],
        )
        .map_err(|error| format!("既存タスクのかんばん状態を移行できません: {error}"))?;

    connection
        .execute_batch(
            "
            CREATE INDEX IF NOT EXISTS tasks_board_column_lifecycle_idx
            ON tasks (board_column_id, lifecycle_status, sort_order, created_at)
            WHERE deleted_at IS NULL;
            ",
        )
        .map_err(|error| format!("かんばんRead Model用インデックスを作成できません: {error}"))
}

fn run_due_time_migration(connection: &Connection) -> RepositoryResult<()> {
    ensure_column(
        connection,
        "tasks",
        "due_time",
        "ALTER TABLE tasks ADD COLUMN due_time TEXT NULL CHECK (due_time IS NULL OR (length(due_time) = 5 AND substr(due_time, 3, 1) = ':' AND substr(due_time, 1, 2) BETWEEN '00' AND '23' AND substr(due_time, 4, 2) BETWEEN '00' AND '59'))",
    )?;
    ensure_column(
        connection,
        "subtasks",
        "due_time",
        "ALTER TABLE subtasks ADD COLUMN due_time TEXT NULL CHECK (due_time IS NULL OR (length(due_time) = 5 AND substr(due_time, 3, 1) = ':' AND substr(due_time, 1, 2) BETWEEN '00' AND '23' AND substr(due_time, 4, 2) BETWEEN '00' AND '59'))",
    )?;
    connection
        .execute_batch(
            "
            CREATE INDEX IF NOT EXISTS tasks_due_time_idx
            ON tasks (due_date, due_time)
            WHERE deleted_at IS NULL;

            CREATE INDEX IF NOT EXISTS subtasks_due_time_idx
            ON subtasks (due_date, due_time)
            WHERE deleted_at IS NULL;
            ",
        )
        .map_err(|error| format!("期限時刻用インデックスを作成できません: {error}"))
}

fn run_work_schedule_migration(connection: &Connection) -> RepositoryResult<()> {
    for table_name in ["tasks", "subtasks"] {
        ensure_column(
            connection,
            table_name,
            "scheduled_start_date",
            &format!("ALTER TABLE {table_name} ADD COLUMN scheduled_start_date TEXT NULL"),
        )?;
        ensure_column(
            connection,
            table_name,
            "scheduled_start_time",
            &format!("ALTER TABLE {table_name} ADD COLUMN scheduled_start_time TEXT NULL"),
        )?;
        ensure_column(
            connection,
            table_name,
            "scheduled_end_date",
            &format!("ALTER TABLE {table_name} ADD COLUMN scheduled_end_date TEXT NULL"),
        )?;
        ensure_column(
            connection,
            table_name,
            "scheduled_end_time",
            &format!("ALTER TABLE {table_name} ADD COLUMN scheduled_end_time TEXT NULL"),
        )?;
        ensure_column(
            connection,
            table_name,
            "scheduled_is_all_day",
            &format!(
                "ALTER TABLE {table_name} ADD COLUMN scheduled_is_all_day INTEGER NOT NULL DEFAULT 0 CHECK (scheduled_is_all_day IN (0, 1))"
            ),
        )?;
    }
    connection
        .execute_batch(
            "
            CREATE INDEX IF NOT EXISTS tasks_schedule_range_idx
            ON tasks (scheduled_start_date, scheduled_end_date)
            WHERE deleted_at IS NULL AND scheduled_start_date IS NOT NULL;

            CREATE INDEX IF NOT EXISTS subtasks_schedule_range_idx
            ON subtasks (scheduled_start_date, scheduled_end_date)
            WHERE deleted_at IS NULL AND scheduled_start_date IS NOT NULL;
            ",
        )
        .map_err(|error| format!("予定期間用インデックスを作成できません: {error}"))
}

fn run_notification_preference_migration(connection: &Connection) -> RepositoryResult<()> {
    ensure_column(
        connection,
        "notification_preferences",
        "notifications_enabled",
        "ALTER TABLE notification_preferences ADD COLUMN notifications_enabled INTEGER NOT NULL DEFAULT 1 CHECK (notifications_enabled IN (0, 1))",
    )
}

fn run_notification_os_registration_migration(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS notification_os_registrations (
              id TEXT PRIMARY KEY,
              notification_rule_id TEXT NOT NULL,
              os_registration_id TEXT NULL CHECK (
                os_registration_id IS NULL OR length(trim(os_registration_id)) > 0
              ),
              registration_status TEXT NOT NULL CHECK (
                registration_status IN (
                  'pending',
                  'registered',
                  'failed',
                  'cancel_pending',
                  'disabled'
                )
              ),
              last_attempted_at TEXT NULL,
              last_error TEXT NULL,
              deleted_at TEXT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              FOREIGN KEY (notification_rule_id) REFERENCES notification_rules(id) ON DELETE RESTRICT
            );

            CREATE UNIQUE INDEX IF NOT EXISTS notification_os_registrations_rule_active_idx
            ON notification_os_registrations (notification_rule_id)
            WHERE deleted_at IS NULL;

            CREATE INDEX IF NOT EXISTS notification_os_registrations_status_idx
            ON notification_os_registrations (registration_status, updated_at)
            WHERE deleted_at IS NULL;

            INSERT INTO notification_os_registrations (
              id, notification_rule_id, os_registration_id, registration_status,
              last_attempted_at, last_error, deleted_at, created_at, updated_at
            )
            SELECT lower(hex(randomblob(16))),
                   notification_rules.id,
                   NULL,
                   CASE
                     WHEN notification_rules.enabled = 1
                      AND notification_rules.deleted_at IS NULL
                     THEN 'pending'
                     ELSE 'disabled'
                   END,
                   NULL,
                   NULL,
                   CASE
                     WHEN notification_rules.enabled = 1
                      AND notification_rules.deleted_at IS NULL
                     THEN NULL
                     ELSE notification_rules.updated_at
                   END,
                   notification_rules.created_at,
                   notification_rules.updated_at
            FROM notification_rules
            WHERE NOT EXISTS (
              SELECT 1
              FROM notification_os_registrations AS registrations
              WHERE registrations.notification_rule_id = notification_rules.id
                AND registrations.deleted_at IS NULL
            );
            ",
        )
        .map(|_| ())
        .map_err(|error| format!("通知OS登録状態テーブルを作成できません: {error}"))
}

fn run_tag_migration(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS tags (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL CHECK (length(trim(name)) > 0 AND length(name) <= 40),
              sort_order INTEGER NOT NULL DEFAULT 0,
              deleted_at TEXT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS task_tags (
              task_id TEXT NOT NULL,
              tag_id TEXT NOT NULL,
              created_at TEXT NOT NULL,
              deleted_at TEXT NULL,
              PRIMARY KEY (task_id, tag_id),
              FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE RESTRICT,
              FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE RESTRICT
            );

            CREATE UNIQUE INDEX IF NOT EXISTS tags_active_name_unique_idx
            ON tags (lower(name))
            WHERE deleted_at IS NULL;

            CREATE INDEX IF NOT EXISTS tags_order_idx
            ON tags (sort_order, created_at)
            WHERE deleted_at IS NULL;

            CREATE INDEX IF NOT EXISTS task_tags_task_idx
            ON task_tags (task_id, created_at)
            WHERE deleted_at IS NULL;

            CREATE INDEX IF NOT EXISTS task_tags_tag_idx
            ON task_tags (tag_id, task_id)
            WHERE deleted_at IS NULL;
            ",
        )
        .map_err(|error| format!("タグ用テーブルを作成できません: {error}"))
}

fn run_notification_delivery_attempt_migration(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS notification_delivery_attempts (
              id TEXT PRIMARY KEY,
              notification_rule_id TEXT NOT NULL,
              target_type TEXT NOT NULL CHECK (target_type IN ('task', 'subtask')),
              target_id TEXT NOT NULL,
              kind TEXT NOT NULL CHECK (kind IN ('planned_start', 'due')),
              notify_at TEXT NOT NULL,
              attempted_at TEXT NOT NULL,
              result TEXT NOT NULL CHECK (result IN ('success', 'failed')),
              error_message TEXT NULL,
              created_at TEXT NOT NULL,
              FOREIGN KEY (notification_rule_id) REFERENCES notification_rules(id) ON DELETE RESTRICT
            );

            CREATE INDEX IF NOT EXISTS notification_delivery_attempts_recent_idx
            ON notification_delivery_attempts (attempted_at DESC, created_at DESC);

            CREATE INDEX IF NOT EXISTS notification_delivery_attempts_rule_idx
            ON notification_delivery_attempts (notification_rule_id, attempted_at DESC);

            INSERT INTO notification_delivery_attempts (
              id, notification_rule_id, target_type, target_id, kind, notify_at,
              attempted_at, result, error_message, created_at
            )
            SELECT lower(hex(randomblob(16))),
                   notification_rules.id,
                   notification_rules.target_type,
                   notification_rules.target_id,
                   notification_rules.kind,
                   notification_rules.notify_at,
                   notification_rules.updated_at,
                   'failed',
                   notification_rules.last_error,
                   notification_rules.updated_at
            FROM notification_rules
            WHERE notification_rules.deleted_at IS NULL
              AND notification_rules.registration_status = 'failed'
              AND NOT EXISTS (
                SELECT 1
                FROM notification_delivery_attempts AS attempts
                WHERE attempts.notification_rule_id = notification_rules.id
              );
            ",
        )
        .map(|_| ())
        .map_err(|error| format!("通知送信履歴テーブルを作成できません: {error}"))
}

fn run_timer_recurrence_migration(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS timer_pauses (
              id TEXT PRIMARY KEY,
              timer_session_id TEXT NOT NULL,
              paused_at TEXT NOT NULL,
              resumed_at TEXT NULL,
              deleted_at TEXT NULL,
              created_at TEXT NOT NULL,
              FOREIGN KEY (timer_session_id) REFERENCES timer_sessions(id) ON DELETE RESTRICT,
              CHECK (resumed_at IS NULL OR resumed_at >= paused_at)
            );

            CREATE UNIQUE INDEX IF NOT EXISTS one_open_pause_per_timer
            ON timer_pauses (timer_session_id)
            WHERE resumed_at IS NULL AND deleted_at IS NULL;

            CREATE INDEX IF NOT EXISTS timer_pauses_session_idx
            ON timer_pauses (timer_session_id, paused_at)
            WHERE deleted_at IS NULL;

            CREATE TABLE IF NOT EXISTS recurrence_rules (
              id TEXT PRIMARY KEY,
              target_type TEXT NOT NULL CHECK (target_type IN ('task', 'subtask')),
              target_id TEXT NOT NULL,
              frequency TEXT NOT NULL CHECK (frequency IN ('daily', 'weekly', 'monthly')),
              interval INTEGER NOT NULL CHECK (interval >= 1 AND interval <= 365),
              deleted_at TEXT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE UNIQUE INDEX IF NOT EXISTS recurrence_rules_active_target_idx
            ON recurrence_rules (target_type, target_id)
            WHERE deleted_at IS NULL;

            CREATE INDEX IF NOT EXISTS recurrence_rules_target_idx
            ON recurrence_rules (target_type, target_id, frequency)
            WHERE deleted_at IS NULL;
            ",
        )
        .map(|_| ())
        .map_err(|error| format!("タイマー一時停止/繰り返し用テーブルを作成できません: {error}"))
}

fn run_pomodoro_migration(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS pomodoro_settings (
              id TEXT PRIMARY KEY CHECK (id = 'default'),
              work_seconds INTEGER NOT NULL CHECK (work_seconds >= 60 AND work_seconds <= 86400),
              short_break_seconds INTEGER NOT NULL CHECK (short_break_seconds >= 60 AND short_break_seconds <= 86400),
              long_break_seconds INTEGER NOT NULL CHECK (long_break_seconds >= 60 AND long_break_seconds <= 86400),
              cycles_until_long_break INTEGER NOT NULL CHECK (cycles_until_long_break >= 1 AND cycles_until_long_break <= 12),
              auto_start_break INTEGER NOT NULL DEFAULT 0 CHECK (auto_start_break IN (0, 1)),
              auto_start_next_work INTEGER NOT NULL DEFAULT 0 CHECK (auto_start_next_work IN (0, 1)),
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS pomodoro_sessions (
              id TEXT PRIMARY KEY,
              scope TEXT NOT NULL CHECK (scope IN ('task_linked', 'standalone')),
              target_type TEXT NULL CHECK (target_type IS NULL OR target_type IN ('task', 'subtask')),
              target_id TEXT NULL,
              timer_session_id TEXT NULL,
              phase TEXT NOT NULL CHECK (phase IN ('work', 'short_break', 'long_break')),
              status TEXT NOT NULL CHECK (status IN ('running', 'paused', 'completed', 'cancelled')),
              cycle_count INTEGER NOT NULL DEFAULT 0 CHECK (cycle_count >= 0),
              phase_started_at TEXT NOT NULL,
              phase_duration_seconds INTEGER NOT NULL CHECK (phase_duration_seconds >= 60 AND phase_duration_seconds <= 86400),
              paused_at TEXT NULL,
              paused_total_seconds INTEGER NOT NULL DEFAULT 0 CHECK (paused_total_seconds >= 0),
              completed_at TEXT NULL,
              cancelled_at TEXT NULL,
              deleted_at TEXT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              FOREIGN KEY (timer_session_id) REFERENCES timer_sessions(id) ON DELETE RESTRICT,
              CHECK (
                (scope = 'task_linked' AND target_type IS NOT NULL AND target_id IS NOT NULL)
                OR (scope = 'standalone' AND target_type IS NULL AND target_id IS NULL)
              ),
              CHECK (
                (scope = 'task_linked' AND (phase <> 'work' OR timer_session_id IS NOT NULL))
                OR (scope = 'standalone' AND timer_session_id IS NULL)
              ),
              CHECK (completed_at IS NULL OR completed_at >= phase_started_at),
              CHECK (cancelled_at IS NULL OR cancelled_at >= phase_started_at)
            );

            CREATE UNIQUE INDEX IF NOT EXISTS one_active_pomodoro_session
            ON pomodoro_sessions ((status IN ('running', 'paused')))
            WHERE status IN ('running', 'paused') AND deleted_at IS NULL;

            CREATE INDEX IF NOT EXISTS pomodoro_sessions_target_idx
            ON pomodoro_sessions (target_type, target_id, created_at)
            WHERE deleted_at IS NULL;

            CREATE INDEX IF NOT EXISTS pomodoro_sessions_timer_idx
            ON pomodoro_sessions (timer_session_id)
            WHERE timer_session_id IS NOT NULL AND deleted_at IS NULL;
            ",
        )
        .map_err(|error| format!("ポモドーロ用テーブルを作成できません: {error}"))?;

    migrate_pomodoro_sessions_to_scoped_model(connection)?;
    seed_default_pomodoro_settings(connection)
}

fn migrate_pomodoro_sessions_to_scoped_model(connection: &Connection) -> RepositoryResult<()> {
    if column_exists(connection, "pomodoro_sessions", "scope")? {
        return Ok(());
    }

    let migration_result = connection.execute_batch(
            "
            SAVEPOINT migrate_scoped_pomodoro;
            DROP INDEX IF EXISTS one_active_pomodoro_session;
            DROP INDEX IF EXISTS pomodoro_sessions_target_idx;
            DROP INDEX IF EXISTS pomodoro_sessions_timer_idx;
            ALTER TABLE pomodoro_sessions RENAME TO pomodoro_sessions_task_linked_legacy;

            CREATE TABLE pomodoro_sessions (
              id TEXT PRIMARY KEY,
              scope TEXT NOT NULL CHECK (scope IN ('task_linked', 'standalone')),
              target_type TEXT NULL CHECK (target_type IS NULL OR target_type IN ('task', 'subtask')),
              target_id TEXT NULL,
              timer_session_id TEXT NULL,
              phase TEXT NOT NULL CHECK (phase IN ('work', 'short_break', 'long_break')),
              status TEXT NOT NULL CHECK (status IN ('running', 'paused', 'completed', 'cancelled')),
              cycle_count INTEGER NOT NULL DEFAULT 0 CHECK (cycle_count >= 0),
              phase_started_at TEXT NOT NULL,
              phase_duration_seconds INTEGER NOT NULL CHECK (phase_duration_seconds >= 60 AND phase_duration_seconds <= 86400),
              paused_at TEXT NULL,
              paused_total_seconds INTEGER NOT NULL DEFAULT 0 CHECK (paused_total_seconds >= 0),
              completed_at TEXT NULL,
              cancelled_at TEXT NULL,
              deleted_at TEXT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL,
              FOREIGN KEY (timer_session_id) REFERENCES timer_sessions(id) ON DELETE RESTRICT,
              CHECK (
                (scope = 'task_linked' AND target_type IS NOT NULL AND target_id IS NOT NULL)
                OR (scope = 'standalone' AND target_type IS NULL AND target_id IS NULL)
              ),
              CHECK (
                (scope = 'task_linked' AND (phase <> 'work' OR timer_session_id IS NOT NULL))
                OR (scope = 'standalone' AND timer_session_id IS NULL)
              ),
              CHECK (completed_at IS NULL OR completed_at >= phase_started_at),
              CHECK (cancelled_at IS NULL OR cancelled_at >= phase_started_at)
            );

            INSERT INTO pomodoro_sessions (
              id, scope, target_type, target_id, timer_session_id, phase, status,
              cycle_count, phase_started_at, phase_duration_seconds, paused_at,
              paused_total_seconds, completed_at, cancelled_at, deleted_at,
              created_at, updated_at
            )
            SELECT id, 'task_linked', target_type, target_id, timer_session_id, phase, status,
                   cycle_count, phase_started_at, phase_duration_seconds, paused_at,
                   paused_total_seconds, completed_at, cancelled_at, deleted_at,
                   created_at, updated_at
            FROM pomodoro_sessions_task_linked_legacy;

            DROP TABLE pomodoro_sessions_task_linked_legacy;

            CREATE UNIQUE INDEX one_active_pomodoro_session
            ON pomodoro_sessions ((status IN ('running', 'paused')))
            WHERE status IN ('running', 'paused') AND deleted_at IS NULL;

            CREATE INDEX pomodoro_sessions_target_idx
            ON pomodoro_sessions (target_type, target_id, created_at)
            WHERE deleted_at IS NULL;

            CREATE INDEX pomodoro_sessions_timer_idx
            ON pomodoro_sessions (timer_session_id)
            WHERE timer_session_id IS NOT NULL AND deleted_at IS NULL;
            RELEASE SAVEPOINT migrate_scoped_pomodoro;
            ",
        );

    if let Err(error) = migration_result {
        let _ = connection.execute_batch(
            "ROLLBACK TO SAVEPOINT migrate_scoped_pomodoro; RELEASE SAVEPOINT migrate_scoped_pomodoro;",
        );
        return Err(format!(
            "ポモドーロ履歴を独立モデルへ移行できません: {error}"
        ));
    }

    Ok(())
}

fn run_ui_read_model_migration(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS task_lists (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL CHECK (length(trim(name)) > 0),
              color_token TEXT NOT NULL DEFAULT 'green' CHECK (
                color_token IN ('green', 'blue', 'amber', 'rose', 'violet', 'gray')
              ),
              sort_order INTEGER NOT NULL DEFAULT 0,
              deleted_at TEXT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS ui_preferences (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            ",
        )
        .map_err(|error| format!("UI Read Model用テーブルを作成できません: {error}"))?;

    ensure_column(
        connection,
        "task_lists",
        "color_token",
        "ALTER TABLE task_lists ADD COLUMN color_token TEXT NOT NULL DEFAULT 'green' CHECK (color_token IN ('green', 'blue', 'amber', 'rose', 'violet', 'gray'))",
    )?;
    ensure_column(
        connection,
        "tasks",
        "list_id",
        "ALTER TABLE tasks ADD COLUMN list_id TEXT NULL",
    )?;
    ensure_column(
        connection,
        "tasks",
        "is_favorite",
        "ALTER TABLE tasks ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0, 1))",
    )?;
    ensure_column(
        connection,
        "tasks",
        "timer_target_seconds",
        "ALTER TABLE tasks ADD COLUMN timer_target_seconds INTEGER NULL CHECK (timer_target_seconds IS NULL OR timer_target_seconds >= 0)",
    )?;
    ensure_column(
        connection,
        "subtasks",
        "timer_target_seconds",
        "ALTER TABLE subtasks ADD COLUMN timer_target_seconds INTEGER NULL CHECK (timer_target_seconds IS NULL OR timer_target_seconds >= 0)",
    )?;

    let now = current_timestamp_value(connection)?;
    ensure_default_task_list(connection, &now)?;
    connection
        .execute(
            "
            UPDATE tasks
            SET list_id = ?1
            WHERE list_id IS NULL
               OR length(trim(list_id)) = 0
            ",
            params![DEFAULT_TASK_LIST_ID],
        )
        .map_err(|error| format!("既存タスクの初期リスト移行に失敗しました: {error}"))?;

    connection
        .execute_batch(
            "
            CREATE INDEX IF NOT EXISTS task_lists_order_idx
            ON task_lists (sort_order, created_at)
            WHERE deleted_at IS NULL;

            CREATE INDEX IF NOT EXISTS tasks_list_status_idx
            ON tasks (list_id, status, sort_order, created_at)
            WHERE deleted_at IS NULL;

            CREATE INDEX IF NOT EXISTS tasks_favorite_idx
            ON tasks (is_favorite, sort_order, created_at)
            WHERE deleted_at IS NULL AND is_favorite = 1;

            CREATE INDEX IF NOT EXISTS tasks_page_order_idx
            ON tasks (
              CASE WHEN status = 'done' THEN 1 ELSE 0 END,
              sort_order,
              created_at,
              id
            )
            WHERE deleted_at IS NULL AND status <> 'archived';

            CREATE INDEX IF NOT EXISTS subtasks_task_status_idx
            ON subtasks (task_id, status)
            WHERE deleted_at IS NULL;
            ",
        )
        .map_err(|error| format!("UI Read Model用インデックスを作成できません: {error}"))?;

    seed_default_ui_preferences(connection)
}

fn ensure_column(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
    alter_sql: &str,
) -> RepositoryResult<()> {
    if column_exists(connection, table_name, column_name)? {
        return Ok(());
    }

    connection
        .execute(alter_sql, [])
        .map(|_| ())
        .map_err(|error| format!("{table_name}.{column_name} カラムを追加できません: {error}"))
}

fn column_exists(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
) -> RepositoryResult<bool> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .map_err(|error| format!("{table_name} のカラム一覧を取得できません: {error}"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("{table_name} のカラム一覧を読めません: {error}"))?;

    for column in columns {
        if column.map_err(|error| format!("{table_name} のカラム名を読めません: {error}"))?
            == column_name
        {
            return Ok(true);
        }
    }

    Ok(false)
}

fn ensure_default_task_list(connection: &Connection, now: &str) -> RepositoryResult<()> {
    connection
        .execute(
            "
            INSERT OR IGNORE INTO task_lists (
              id, name, color_token, sort_order, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, 0, ?4, ?4)
            ",
            params![
                DEFAULT_TASK_LIST_ID,
                DEFAULT_TASK_LIST_NAME,
                DEFAULT_TASK_LIST_COLOR_TOKEN,
                now
            ],
        )
        .map(|_| ())
        .map_err(|error| format!("初期タスクリストを保存できません: {error}"))
}

fn ensure_default_board_columns(connection: &Connection, now: &str) -> RepositoryResult<()> {
    connection
        .execute(
            "
            INSERT OR IGNORE INTO board_columns (id, title, sort_order, created_at, updated_at)
            VALUES (?1, '未着手', 0, ?3, ?3), (?2, '進行中', 1, ?3, ?3)
            ",
            params![DEFAULT_BOARD_COLUMN_ID, IN_PROGRESS_BOARD_COLUMN_ID, now],
        )
        .map(|_| ())
        .map_err(|error| format!("初期状態を保存できません: {error}"))
}

fn seed_default_task_timer_settings(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute(
            "
            INSERT OR IGNORE INTO task_timer_settings (
              id, default_target_seconds, updated_at
            )
            VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            ",
            params![
                DEFAULT_TASK_TIMER_SETTINGS_ID,
                DEFAULT_TASK_TIMER_TARGET_SECONDS
            ],
        )
        .map(|_| ())
        .map_err(|error| format!("タスクタイマー設定を初期化できません: {error}"))
}

fn seed_default_ui_preferences(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute(
            "
            INSERT OR IGNORE INTO ui_preferences (key, value, updated_at)
            VALUES ('left_pane_open', 'true', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                   ('last_view', 'list', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                   ('last_task_list_id', ?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                   ('calendar_view_mode', 'week', strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            ",
            params![DEFAULT_TASK_LIST_ID],
        )
        .map(|_| ())
        .map_err(|error| format!("UI設定の初期化に失敗しました: {error}"))
}

fn seed_default_pomodoro_settings(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute(
            "
            INSERT OR IGNORE INTO pomodoro_settings (
              id,
              work_seconds,
              short_break_seconds,
              long_break_seconds,
              cycles_until_long_break,
              auto_start_break,
              auto_start_next_work,
              updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            ",
            params![
                DEFAULT_POMODORO_SETTINGS_ID,
                DEFAULT_POMODORO_WORK_SECONDS,
                DEFAULT_POMODORO_SHORT_BREAK_SECONDS,
                DEFAULT_POMODORO_LONG_BREAK_SECONDS,
                DEFAULT_POMODORO_CYCLES_UNTIL_LONG_BREAK
            ],
        )
        .map(|_| ())
        .map_err(|error| format!("ポモドーロ設定の初期化に失敗しました: {error}"))
}

fn normalize_optional_list_id(list_id: Option<&str>) -> Option<String> {
    list_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn current_timestamp_value(connection: &Connection) -> RepositoryResult<String> {
    connection
        .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
            row.get(0)
        })
        .map_err(|error| format!("現在時刻を取得できません: {error}"))
}

fn seed_default_preferences(connection: &Connection) -> RepositoryResult<()> {
    let now = current_timestamp_value(connection)?;
    ensure_default_task_list(connection, &now)?;
    seed_default_ui_preferences(connection)?;
    connection
        .execute(
            "
            INSERT OR IGNORE INTO notification_preferences (
              id, display_mode, notifications_enabled, created_at, updated_at
            )
            VALUES (
              'default',
              'title_only',
              1,
              strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
              strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )
            ",
            [],
        )
        .map(|_| ())
        .map_err(|error| format!("通知表示設定の初期化に失敗しました: {error}"))
}

fn backup_label(now: &str) -> String {
    let digits: String = now
        .chars()
        .filter(|character| character.is_ascii_digit())
        .collect();
    if digits.len() >= 14 {
        return format!("{}-{}", &digits[0..8], &digits[8..14]);
    }

    let fallback: String = now
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(32)
        .collect();
    if fallback.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        fallback
    }
}

fn create_sqlite_snapshot(connection: &Connection, target_path: &Path) -> RepositoryResult<()> {
    let target = target_path.to_string_lossy().to_string();
    connection
        .execute("VACUUM INTO ?1", params![target])
        .map(|_| ())
        .map_err(|error| format!("SQLiteバックアップを作成できません: {error}"))
}

fn write_backup_manifest(path: &Path, manifest: &BackupManifestFile) -> RepositoryResult<()> {
    let content = serde_json::to_string_pretty(manifest)
        .map_err(|error| format!("バックアップmanifestを生成できません: {error}"))?;
    fs::write(path, content)
        .map_err(|error| format!("バックアップmanifestを書き込めません: {error}"))
}

fn read_backup_manifest(backup_dir: &Path) -> RepositoryResult<BackupManifestFile> {
    let path = backup_dir.join(BACKUP_MANIFEST_FILE);
    if !path.is_file() {
        return Err("バックアップmanifestが見つかりません".to_string());
    }
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("バックアップmanifestを読めません: {error}"))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("バックアップmanifestの形式が不正です: {error}"))
}

fn validate_backup_manifest(manifest: &BackupManifestFile) -> RepositoryResult<()> {
    if manifest.format != BACKUP_FORMAT {
        return Err("バックアップ形式がTaskTimer SQLiteバックアップではありません".to_string());
    }
    if manifest.format_version != BACKUP_FORMAT_VERSION {
        return Err("バックアップ形式バージョンに対応していません".to_string());
    }
    if manifest.schema_version > CURRENT_SQLITE_BACKUP_SCHEMA_VERSION {
        return Err(
            "新しいTaskTimerで作成されたバックアップの可能性があるため復元できません".to_string(),
        );
    }
    if manifest.database_file != BACKUP_DATABASE_FILE {
        return Err("バックアップDBファイル名が不正です".to_string());
    }
    if manifest.integrity_check != "ok" {
        return Err("バックアップ作成時の整合性確認がokではありません".to_string());
    }
    OffsetDateTime::parse(&manifest.created_at, &Rfc3339)
        .map_err(|error| format!("バックアップ作成日時の形式が不正です: {error}"))?;
    Ok(())
}

fn validate_readonly_backup_database(path: &Path) -> RepositoryResult<()> {
    if !path.is_file() {
        return Err("バックアップDBが見つかりません".to_string());
    }
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("バックアップDBを開けません: {error}"))?;
    configure_connection(&connection)?;
    verify_integrity_check(&connection)?;
    ensure_required_restore_tables(&connection)
}

fn validate_restore_candidate_database(path: &Path) -> RepositoryResult<()> {
    let connection =
        Connection::open(path).map_err(|error| format!("復元候補DBを開けません: {error}"))?;
    configure_connection(&connection)?;
    run_initial_migration(&connection)?;
    seed_default_preferences(&connection)?;
    verify_integrity_check(&connection)?;
    ensure_required_restore_tables(&connection)
}

fn verify_integrity_check(connection: &Connection) -> RepositoryResult<()> {
    let result: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| format!("SQLite整合性確認を実行できません: {error}"))?;
    if result == "ok" {
        Ok(())
    } else {
        Err("SQLite整合性確認に失敗しました".to_string())
    }
}

fn ensure_required_restore_tables(connection: &Connection) -> RepositoryResult<()> {
    for table_name in REQUIRED_RESTORE_TABLES {
        let exists: Option<String> = connection
            .query_row(
                "
                SELECT name
                FROM sqlite_master
                WHERE type = 'table'
                  AND name = ?1
                ",
                params![table_name],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("必須テーブル一覧を確認できません: {error}"))?;
        if exists.is_none() {
            return Err(format!(
                "バックアップDBに必須テーブル {table_name} がありません"
            ));
        }
    }
    Ok(())
}

fn open_live_database(path: &Path) -> RepositoryResult<Connection> {
    let connection = Connection::open(path)
        .map_err(|error| format!("SQLiteデータベースを開けません: {error}"))?;
    configure_connection(&connection)?;
    run_initial_migration(&connection)?;
    seed_default_preferences(&connection)?;
    Ok(connection)
}

fn replace_database_file(
    current_path: &Path,
    candidate_path: &Path,
    previous_path: &Path,
) -> RepositoryResult<()> {
    if previous_path.exists() {
        return Err("復元前DBの退避先が既に存在します".to_string());
    }

    if current_path.exists() {
        fs::rename(current_path, previous_path)
            .map_err(|error| format!("現在のDBを退避できません: {error}"))?;
    }

    if let Err(error) = fs::rename(candidate_path, current_path) {
        if previous_path.exists() {
            let _ = fs::rename(previous_path, current_path);
        }
        return Err(format!("検証済みDBへ入れ替えできません: {error}"));
    }

    Ok(())
}

fn restore_previous_database_file(current_path: &Path, previous_path: &Path) {
    if previous_path.exists() {
        let _ = fs::remove_file(current_path);
        let _ = fs::rename(previous_path, current_path);
    }
}

fn create_export_manifest(format: &str, input: &DataExportCreate) -> ExportManifestFile {
    ExportManifestFile {
        format: format.to_string(),
        format_version: DATA_EXPORT_FORMAT_VERSION,
        app_version: input.app_version.clone(),
        created_at: input.now.clone(),
        platform: input.platform.clone(),
        compatibility: DATA_EXPORT_COMPATIBILITY.to_string(),
        contains_personal_data: true,
    }
}

fn write_json_export_file(path: &Path, export: &JsonExportFile) -> RepositoryResult<()> {
    let content = serde_json::to_string_pretty(export)
        .map_err(|error| format!("JSONエクスポートを生成できません: {error}"))?;
    fs::write(path, content).map_err(|error| format!("JSONエクスポートを書き込めません: {error}"))
}

fn write_csv_export_files(
    export_dir: &Path,
    manifest: &ExportManifestFile,
    dataset: ExportDataset,
) -> RepositoryResult<()> {
    write_export_manifest_file(&export_dir.join(CSV_EXPORT_MANIFEST_FILE), manifest)?;
    write_csv_file(
        &export_dir.join("task_lists.csv"),
        &[
            "id",
            "name",
            "color_token",
            "sort_order",
            "created_at",
            "updated_at",
        ],
        dataset
            .task_lists
            .iter()
            .map(|row| {
                vec![
                    row.id.clone(),
                    row.name.clone(),
                    row.color_token.clone(),
                    row.sort_order.to_string(),
                    row.created_at.clone(),
                    row.updated_at.clone(),
                ]
            })
            .collect(),
    )?;
    write_csv_file(
        &export_dir.join("board_columns.csv"),
        &["id", "title", "sort_order", "created_at", "updated_at"],
        dataset
            .board_columns
            .iter()
            .map(|row| {
                vec![
                    row.id.clone(),
                    row.title.clone(),
                    row.sort_order.to_string(),
                    row.created_at.clone(),
                    row.updated_at.clone(),
                ]
            })
            .collect(),
    )?;
    write_csv_file(
        &export_dir.join("tags.csv"),
        &["id", "name", "sort_order", "created_at", "updated_at"],
        dataset
            .tags
            .iter()
            .map(|row| {
                vec![
                    row.id.clone(),
                    row.name.clone(),
                    row.sort_order.to_string(),
                    row.created_at.clone(),
                    row.updated_at.clone(),
                ]
            })
            .collect(),
    )?;
    write_csv_file(
        &export_dir.join("task_tags.csv"),
        &["task_id", "tag_id", "created_at"],
        dataset
            .task_tags
            .iter()
            .map(|row| {
                vec![
                    row.task_id.clone(),
                    row.tag_id.clone(),
                    row.created_at.clone(),
                ]
            })
            .collect(),
    )?;
    write_csv_file(
        &export_dir.join("tasks.csv"),
        &[
            "id",
            "list_id",
            "board_column_id",
            "title",
            "status",
            "lifecycle_status",
            "is_favorite",
            "color_token",
            "planned_start_date",
            "due_date",
            "due_time",
            "scheduled_start_date",
            "scheduled_start_time",
            "scheduled_end_date",
            "scheduled_end_time",
            "scheduled_is_all_day",
            "timer_target_seconds",
            "memo",
            "sort_order",
            "completed_at",
            "created_at",
            "updated_at",
        ],
        dataset
            .tasks
            .iter()
            .map(|row| {
                vec![
                    row.id.clone(),
                    row.list_id.clone(),
                    row.board_column_id.clone(),
                    row.title.clone(),
                    row.status.clone(),
                    row.lifecycle_status.clone(),
                    row.is_favorite.to_string(),
                    option_text(&row.color_token),
                    option_text(&row.planned_start_date),
                    option_text(&row.due_date),
                    option_text(&row.due_time),
                    option_text(&row.scheduled_start_date),
                    option_text(&row.scheduled_start_time),
                    option_text(&row.scheduled_end_date),
                    option_text(&row.scheduled_end_time),
                    row.scheduled_is_all_day.to_string(),
                    option_i64_text(row.timer_target_seconds),
                    row.memo.clone(),
                    row.sort_order.to_string(),
                    option_text(&row.completed_at),
                    row.created_at.clone(),
                    row.updated_at.clone(),
                ]
            })
            .collect(),
    )?;
    write_csv_file(
        &export_dir.join("subtasks.csv"),
        &[
            "id",
            "task_id",
            "title",
            "status",
            "planned_start_date",
            "due_date",
            "due_time",
            "scheduled_start_date",
            "scheduled_start_time",
            "scheduled_end_date",
            "scheduled_end_time",
            "scheduled_is_all_day",
            "timer_target_seconds",
            "memo",
            "sort_order",
            "completed_at",
            "created_at",
            "updated_at",
        ],
        dataset
            .subtasks
            .iter()
            .map(|row| {
                vec![
                    row.id.clone(),
                    row.task_id.clone(),
                    row.title.clone(),
                    row.status.clone(),
                    option_text(&row.planned_start_date),
                    option_text(&row.due_date),
                    option_text(&row.due_time),
                    option_text(&row.scheduled_start_date),
                    option_text(&row.scheduled_start_time),
                    option_text(&row.scheduled_end_date),
                    option_text(&row.scheduled_end_time),
                    row.scheduled_is_all_day.to_string(),
                    option_i64_text(row.timer_target_seconds),
                    row.memo.clone(),
                    row.sort_order.to_string(),
                    option_text(&row.completed_at),
                    row.created_at.clone(),
                    row.updated_at.clone(),
                ]
            })
            .collect(),
    )?;
    write_csv_file(
        &export_dir.join("timer_sessions.csv"),
        &[
            "id",
            "target_type",
            "target_id",
            "started_at",
            "stopped_at",
            "elapsed_seconds",
            "target_seconds",
            "completion_reason",
            "completion_notified_at",
            "created_at",
        ],
        dataset
            .timer_sessions
            .iter()
            .map(|row| {
                vec![
                    row.id.clone(),
                    row.target_type.clone(),
                    row.target_id.clone(),
                    row.started_at.clone(),
                    option_text(&row.stopped_at),
                    option_i64_text(row.elapsed_seconds),
                    option_i64_text(row.target_seconds),
                    option_text(&row.completion_reason),
                    option_text(&row.completion_notified_at),
                    row.created_at.clone(),
                ]
            })
            .collect(),
    )?;
    write_csv_file(
        &export_dir.join("timer_pauses.csv"),
        &[
            "id",
            "timer_session_id",
            "paused_at",
            "resumed_at",
            "created_at",
        ],
        dataset
            .timer_pauses
            .iter()
            .map(|row| {
                vec![
                    row.id.clone(),
                    row.timer_session_id.clone(),
                    row.paused_at.clone(),
                    option_text(&row.resumed_at),
                    row.created_at.clone(),
                ]
            })
            .collect(),
    )?;
    write_csv_file(
        &export_dir.join("task_timer_settings.csv"),
        &["id", "default_target_seconds", "updated_at"],
        dataset
            .task_timer_settings
            .iter()
            .map(|row| {
                vec![
                    row.id.clone(),
                    row.default_target_seconds.to_string(),
                    row.updated_at.clone(),
                ]
            })
            .collect(),
    )?;
    write_csv_file(
        &export_dir.join("pomodoro_settings.csv"),
        &[
            "id",
            "work_seconds",
            "short_break_seconds",
            "long_break_seconds",
            "cycles_until_long_break",
            "auto_start_break",
            "auto_start_next_work",
            "updated_at",
        ],
        dataset
            .pomodoro_settings
            .iter()
            .map(|row| {
                vec![
                    row.id.clone(),
                    row.work_seconds.to_string(),
                    row.short_break_seconds.to_string(),
                    row.long_break_seconds.to_string(),
                    row.cycles_until_long_break.to_string(),
                    row.auto_start_break.to_string(),
                    row.auto_start_next_work.to_string(),
                    row.updated_at.clone(),
                ]
            })
            .collect(),
    )?;
    write_csv_file(
        &export_dir.join("pomodoro_sessions.csv"),
        &[
            "id",
            "scope",
            "target_type",
            "target_id",
            "timer_session_id",
            "phase",
            "status",
            "cycle_count",
            "phase_started_at",
            "phase_duration_seconds",
            "paused_at",
            "paused_total_seconds",
            "completed_at",
            "cancelled_at",
            "created_at",
            "updated_at",
        ],
        dataset
            .pomodoro_sessions
            .iter()
            .map(|row| {
                vec![
                    row.id.clone(),
                    row.scope.clone(),
                    option_text(&row.target_type),
                    option_text(&row.target_id),
                    option_text(&row.timer_session_id),
                    row.phase.clone(),
                    row.status.clone(),
                    row.cycle_count.to_string(),
                    row.phase_started_at.clone(),
                    row.phase_duration_seconds.to_string(),
                    option_text(&row.paused_at),
                    row.paused_total_seconds.to_string(),
                    option_text(&row.completed_at),
                    option_text(&row.cancelled_at),
                    row.created_at.clone(),
                    row.updated_at.clone(),
                ]
            })
            .collect(),
    )?;
    write_csv_file(
        &export_dir.join("notification_rules.csv"),
        &[
            "id",
            "target_type",
            "target_id",
            "kind",
            "notify_at",
            "enabled",
            "registration_status",
            "last_error",
            "created_at",
            "updated_at",
        ],
        dataset
            .notification_rules
            .iter()
            .map(|row| {
                vec![
                    row.id.clone(),
                    row.target_type.clone(),
                    row.target_id.clone(),
                    row.kind.clone(),
                    row.notify_at.clone(),
                    row.enabled.to_string(),
                    row.registration_status.clone(),
                    option_text(&row.last_error),
                    row.created_at.clone(),
                    row.updated_at.clone(),
                ]
            })
            .collect(),
    )?;
    write_csv_file(
        &export_dir.join("notification_os_registrations.csv"),
        &[
            "id",
            "notification_rule_id",
            "os_registration_id",
            "registration_status",
            "last_attempted_at",
            "last_error",
            "created_at",
            "updated_at",
        ],
        dataset
            .notification_os_registrations
            .iter()
            .map(|row| {
                vec![
                    row.id.clone(),
                    row.notification_rule_id.clone(),
                    option_text(&row.os_registration_id),
                    row.registration_status.clone(),
                    option_text(&row.last_attempted_at),
                    option_text(&row.last_error),
                    row.created_at.clone(),
                    row.updated_at.clone(),
                ]
            })
            .collect(),
    )?;
    write_csv_file(
        &export_dir.join("recurrence_rules.csv"),
        &[
            "id",
            "target_type",
            "target_id",
            "frequency",
            "interval",
            "created_at",
            "updated_at",
        ],
        dataset
            .recurrence_rules
            .iter()
            .map(|row| {
                vec![
                    row.id.clone(),
                    row.target_type.clone(),
                    row.target_id.clone(),
                    row.frequency.clone(),
                    row.interval.to_string(),
                    row.created_at.clone(),
                    row.updated_at.clone(),
                ]
            })
            .collect(),
    )
}

fn write_export_manifest_file(path: &Path, manifest: &ExportManifestFile) -> RepositoryResult<()> {
    let content = serde_json::to_string_pretty(manifest)
        .map_err(|error| format!("エクスポートmanifestを生成できません: {error}"))?;
    fs::write(path, content)
        .map_err(|error| format!("エクスポートmanifestを書き込めません: {error}"))
}

fn csv_export_file_names() -> Vec<&'static str> {
    vec![
        CSV_EXPORT_MANIFEST_FILE,
        "task_lists.csv",
        "board_columns.csv",
        "tags.csv",
        "task_tags.csv",
        "tasks.csv",
        "subtasks.csv",
        "timer_sessions.csv",
        "timer_pauses.csv",
        "task_timer_settings.csv",
        "pomodoro_settings.csv",
        "pomodoro_sessions.csv",
        "notification_rules.csv",
        "notification_os_registrations.csv",
        "recurrence_rules.csv",
    ]
}

fn write_csv_file(path: &Path, headers: &[&str], rows: Vec<Vec<String>>) -> RepositoryResult<()> {
    let mut content = String::new();
    content.push_str(
        &headers
            .iter()
            .map(|value| csv_cell(value))
            .collect::<Vec<_>>()
            .join(","),
    );
    content.push('\n');

    for row in rows {
        content.push_str(
            &row.iter()
                .map(|value| csv_cell(value))
                .collect::<Vec<_>>()
                .join(","),
        );
        content.push('\n');
    }

    fs::write(path, content).map_err(|error| format!("CSVエクスポートを書き込めません: {error}"))
}

fn csv_cell(value: &str) -> String {
    let safe_value = neutralize_csv_formula(value);
    if safe_value.contains(',')
        || safe_value.contains('"')
        || safe_value.contains('\n')
        || safe_value.contains('\r')
    {
        format!("\"{}\"", safe_value.replace('"', "\"\""))
    } else {
        safe_value
    }
}

fn neutralize_csv_formula(value: &str) -> String {
    match value.chars().next() {
        Some('=') | Some('+') | Some('-') | Some('@') | Some('\t') | Some('\r') => {
            format!("'{value}")
        }
        _ => value.to_string(),
    }
}

fn option_text(value: &Option<String>) -> String {
    value.clone().unwrap_or_default()
}

fn option_i64_text(value: Option<i64>) -> String {
    value.map(|number| number.to_string()).unwrap_or_default()
}

fn select_export_dataset(connection: &Connection) -> RepositoryResult<ExportDataset> {
    Ok(ExportDataset {
        task_lists: select_export_task_lists(connection)?,
        board_columns: select_export_board_columns(connection)?,
        tags: select_export_tags(connection)?,
        task_tags: select_export_task_tags(connection)?,
        tasks: select_export_tasks(connection)?,
        subtasks: select_export_subtasks(connection)?,
        timer_sessions: select_export_timer_sessions(connection)?,
        timer_pauses: select_export_timer_pauses(connection)?,
        task_timer_settings: select_export_task_timer_settings(connection)?,
        pomodoro_settings: select_export_pomodoro_settings(connection)?,
        pomodoro_sessions: select_export_pomodoro_sessions(connection)?,
        notification_rules: select_export_notification_rules(connection)?,
        notification_os_registrations: select_export_notification_os_registrations(connection)?,
        recurrence_rules: select_export_recurrence_rules(connection)?,
    })
}

fn select_export_board_columns(
    connection: &Connection,
) -> RepositoryResult<Vec<ExportBoardColumnRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, title, sort_order, created_at, updated_at
            FROM board_columns
            WHERE deleted_at IS NULL
            ORDER BY sort_order, created_at, id
            ",
        )
        .map_err(|error| format!("エクスポート用かんばん状態取得を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportBoardColumnRow {
                id: row.get(0)?,
                title: row.get(1)?,
                sort_order: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|error| format!("エクスポート用かんばん状態を取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用かんばん状態を読めません")
}

fn select_export_task_lists(connection: &Connection) -> RepositoryResult<Vec<ExportTaskListRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, name, color_token, sort_order, created_at, updated_at
            FROM task_lists
            WHERE deleted_at IS NULL
            ORDER BY sort_order, created_at, id
            ",
        )
        .map_err(|error| format!("エクスポート用リスト取得を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportTaskListRow {
                id: row.get(0)?,
                name: row.get(1)?,
                color_token: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|error| format!("エクスポート用リストを取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用リストを読めません")
}

fn select_export_tags(connection: &Connection) -> RepositoryResult<Vec<ExportTagRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, name, sort_order, created_at, updated_at
            FROM tags
            WHERE deleted_at IS NULL
            ORDER BY sort_order, created_at, id
            ",
        )
        .map_err(|error| format!("エクスポート用タグ取得を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportTagRow {
                id: row.get(0)?,
                name: row.get(1)?,
                sort_order: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|error| format!("エクスポート用タグを取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用タグを読めません")
}

fn select_export_task_tags(connection: &Connection) -> RepositoryResult<Vec<ExportTaskTagRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT task_tags.task_id, task_tags.tag_id, task_tags.created_at
            FROM task_tags
            INNER JOIN tasks
              ON tasks.id = task_tags.task_id
             AND tasks.deleted_at IS NULL
            INNER JOIN tags
              ON tags.id = task_tags.tag_id
             AND tags.deleted_at IS NULL
            WHERE task_tags.deleted_at IS NULL
            ORDER BY task_tags.task_id, task_tags.created_at, task_tags.tag_id
            ",
        )
        .map_err(|error| format!("エクスポート用タスクタグ取得を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportTaskTagRow {
                task_id: row.get(0)?,
                tag_id: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .map_err(|error| format!("エクスポート用タスクタグを取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用タスクタグを読めません")
}

fn select_export_tasks(connection: &Connection) -> RepositoryResult<Vec<ExportTaskRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, list_id, board_column_id, title, status, lifecycle_status,
                   is_favorite, color_token, planned_start_date, due_date, due_time,
                   scheduled_start_date, scheduled_start_time,
                   scheduled_end_date, scheduled_end_time, scheduled_is_all_day,
                   timer_target_seconds, memo, sort_order, completed_at, created_at, updated_at
            FROM tasks
            WHERE deleted_at IS NULL
            ORDER BY sort_order, created_at, id
            ",
        )
        .map_err(|error| format!("エクスポート用タスク取得を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportTaskRow {
                id: row.get(0)?,
                list_id: row.get(1)?,
                board_column_id: row.get(2)?,
                title: row.get(3)?,
                status: row.get(4)?,
                lifecycle_status: row.get(5)?,
                is_favorite: row.get::<_, i64>(6)? != 0,
                color_token: row.get(7)?,
                planned_start_date: row.get(8)?,
                due_date: row.get(9)?,
                due_time: row.get(10)?,
                scheduled_start_date: row.get(11)?,
                scheduled_start_time: row.get(12)?,
                scheduled_end_date: row.get(13)?,
                scheduled_end_time: row.get(14)?,
                scheduled_is_all_day: row.get::<_, i64>(15)? != 0,
                timer_target_seconds: row.get(16)?,
                memo: row.get(17)?,
                sort_order: row.get(18)?,
                completed_at: row.get(19)?,
                created_at: row.get(20)?,
                updated_at: row.get(21)?,
            })
        })
        .map_err(|error| format!("エクスポート用タスクを取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用タスクを読めません")
}

fn select_export_subtasks(connection: &Connection) -> RepositoryResult<Vec<ExportSubtaskRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, task_id, title, status, planned_start_date, due_date,
                   due_time, scheduled_start_date, scheduled_start_time,
                   scheduled_end_date, scheduled_end_time, scheduled_is_all_day,
                   timer_target_seconds, memo, sort_order, completed_at, created_at, updated_at
            FROM subtasks
            WHERE deleted_at IS NULL
            ORDER BY task_id, sort_order, created_at, id
            ",
        )
        .map_err(|error| format!("エクスポート用サブタスク取得を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportSubtaskRow {
                id: row.get(0)?,
                task_id: row.get(1)?,
                title: row.get(2)?,
                status: row.get(3)?,
                planned_start_date: row.get(4)?,
                due_date: row.get(5)?,
                due_time: row.get(6)?,
                scheduled_start_date: row.get(7)?,
                scheduled_start_time: row.get(8)?,
                scheduled_end_date: row.get(9)?,
                scheduled_end_time: row.get(10)?,
                scheduled_is_all_day: row.get::<_, i64>(11)? != 0,
                timer_target_seconds: row.get(12)?,
                memo: row.get(13)?,
                sort_order: row.get(14)?,
                completed_at: row.get(15)?,
                created_at: row.get(16)?,
                updated_at: row.get(17)?,
            })
        })
        .map_err(|error| format!("エクスポート用サブタスクを取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用サブタスクを読めません")
}

fn select_export_timer_sessions(
    connection: &Connection,
) -> RepositoryResult<Vec<ExportTimerSessionRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, target_type, target_id, started_at, stopped_at,
                   elapsed_seconds, target_seconds, completion_reason,
                   completion_notified_at, created_at
            FROM timer_sessions
            WHERE deleted_at IS NULL
            ORDER BY started_at, created_at, id
            ",
        )
        .map_err(|error| format!("エクスポート用タイマー履歴取得を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportTimerSessionRow {
                id: row.get(0)?,
                target_type: row.get(1)?,
                target_id: row.get(2)?,
                started_at: row.get(3)?,
                stopped_at: row.get(4)?,
                elapsed_seconds: row.get(5)?,
                target_seconds: row.get(6)?,
                completion_reason: row.get(7)?,
                completion_notified_at: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|error| format!("エクスポート用タイマー履歴を取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用タイマー履歴を読めません")
}

fn select_export_task_timer_settings(
    connection: &Connection,
) -> RepositoryResult<Vec<ExportTaskTimerSettingsRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, default_target_seconds, updated_at
            FROM task_timer_settings
            ORDER BY id
            ",
        )
        .map_err(|error| {
            format!("エクスポート用タスクタイマー設定取得を準備できません: {error}")
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportTaskTimerSettingsRow {
                id: row.get(0)?,
                default_target_seconds: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })
        .map_err(|error| format!("エクスポート用タスクタイマー設定を取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用タスクタイマー設定を読めません")
}

fn select_export_timer_pauses(
    connection: &Connection,
) -> RepositoryResult<Vec<ExportTimerPauseRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, timer_session_id, paused_at, resumed_at, created_at
            FROM timer_pauses
            WHERE deleted_at IS NULL
            ORDER BY paused_at, created_at, id
            ",
        )
        .map_err(|error| format!("エクスポート用一時停止履歴取得を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportTimerPauseRow {
                id: row.get(0)?,
                timer_session_id: row.get(1)?,
                paused_at: row.get(2)?,
                resumed_at: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|error| format!("エクスポート用一時停止履歴を取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用一時停止履歴を読めません")
}

fn select_export_pomodoro_settings(
    connection: &Connection,
) -> RepositoryResult<Vec<ExportPomodoroSettingsRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, work_seconds, short_break_seconds, long_break_seconds,
                   cycles_until_long_break, auto_start_break, auto_start_next_work,
                   updated_at
            FROM pomodoro_settings
            ORDER BY id
            ",
        )
        .map_err(|error| format!("エクスポート用ポモドーロ設定取得を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportPomodoroSettingsRow {
                id: row.get(0)?,
                work_seconds: row.get(1)?,
                short_break_seconds: row.get(2)?,
                long_break_seconds: row.get(3)?,
                cycles_until_long_break: row.get(4)?,
                auto_start_break: row.get::<_, i64>(5)? != 0,
                auto_start_next_work: row.get::<_, i64>(6)? != 0,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|error| format!("エクスポート用ポモドーロ設定を取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用ポモドーロ設定を読めません")
}

fn select_export_pomodoro_sessions(
    connection: &Connection,
) -> RepositoryResult<Vec<ExportPomodoroSessionRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, scope, target_type, target_id, timer_session_id, phase, status,
                   cycle_count, phase_started_at, phase_duration_seconds, paused_at,
                   paused_total_seconds, completed_at, cancelled_at, created_at, updated_at
            FROM pomodoro_sessions
            WHERE deleted_at IS NULL
            ORDER BY created_at, id
            ",
        )
        .map_err(|error| format!("エクスポート用ポモドーロ履歴取得を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportPomodoroSessionRow {
                id: row.get(0)?,
                scope: row.get(1)?,
                target_type: row.get(2)?,
                target_id: row.get(3)?,
                timer_session_id: row.get(4)?,
                phase: row.get(5)?,
                status: row.get(6)?,
                cycle_count: row.get(7)?,
                phase_started_at: row.get(8)?,
                phase_duration_seconds: row.get(9)?,
                paused_at: row.get(10)?,
                paused_total_seconds: row.get(11)?,
                completed_at: row.get(12)?,
                cancelled_at: row.get(13)?,
                created_at: row.get(14)?,
                updated_at: row.get(15)?,
            })
        })
        .map_err(|error| format!("エクスポート用ポモドーロ履歴を取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用ポモドーロ履歴を読めません")
}

fn select_export_notification_rules(
    connection: &Connection,
) -> RepositoryResult<Vec<ExportNotificationRuleRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, target_type, target_id, kind, notify_at, enabled,
                   registration_status, last_error, created_at, updated_at
            FROM notification_rules
            WHERE deleted_at IS NULL
            ORDER BY notify_at, created_at, id
            ",
        )
        .map_err(|error| format!("エクスポート用通知ルール取得を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportNotificationRuleRow {
                id: row.get(0)?,
                target_type: row.get(1)?,
                target_id: row.get(2)?,
                kind: row.get(3)?,
                notify_at: row.get(4)?,
                enabled: row.get::<_, i64>(5)? != 0,
                registration_status: row.get(6)?,
                last_error: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|error| format!("エクスポート用通知ルールを取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用通知ルールを読めません")
}

fn select_export_notification_os_registrations(
    connection: &Connection,
) -> RepositoryResult<Vec<ExportNotificationOsRegistrationRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, notification_rule_id, os_registration_id, registration_status,
                   last_attempted_at, last_error, created_at, updated_at
            FROM notification_os_registrations
            WHERE deleted_at IS NULL
            ORDER BY updated_at, created_at, id
            ",
        )
        .map_err(|error| format!("エクスポート用通知OS登録状態取得を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportNotificationOsRegistrationRow {
                id: row.get(0)?,
                notification_rule_id: row.get(1)?,
                os_registration_id: row.get(2)?,
                registration_status: row.get(3)?,
                last_attempted_at: row.get(4)?,
                last_error: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })
        .map_err(|error| format!("エクスポート用通知OS登録状態を取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用通知OS登録状態を読めません")
}

fn select_export_recurrence_rules(
    connection: &Connection,
) -> RepositoryResult<Vec<ExportRecurrenceRuleRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, target_type, target_id, frequency, interval, created_at, updated_at
            FROM recurrence_rules
            WHERE deleted_at IS NULL
            ORDER BY created_at, id
            ",
        )
        .map_err(|error| format!("エクスポート用繰り返し設定取得を準備できません: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ExportRecurrenceRuleRow {
                id: row.get(0)?,
                target_type: row.get(1)?,
                target_id: row.get(2)?,
                frequency: row.get(3)?,
                interval: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|error| format!("エクスポート用繰り返し設定を取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用繰り返し設定を読めません")
}

fn collect_export_rows<T>(
    rows: impl Iterator<Item = rusqlite::Result<T>>,
    error_message: &str,
) -> RepositoryResult<Vec<T>> {
    rows.map(|row| row.map_err(|error| format!("{error_message}: {error}")))
        .collect()
}

fn collect_task_calendar_items(
    connection: &Connection,
    start_date: &str,
    end_date: &str,
    scope: &TaskPageScope,
    today_date: &str,
    items: &mut Vec<WeekCalendarItem>,
) -> RepositoryResult<()> {
    let mut statement = connection
        .prepare(
            "
            SELECT tasks.id,
                   tasks.title,
                   tasks.planned_start_date,
                   tasks.due_date,
                   tasks.due_time,
                   tasks.scheduled_start_date,
                   tasks.scheduled_start_time,
                   tasks.scheduled_end_date,
                   tasks.scheduled_end_time,
                   tasks.scheduled_is_all_day,
                   tasks.status,
                   COALESCE(tasks.color_token, task_lists.color_token) AS color_token,
                   task_lists.color_token AS list_color_token
            FROM tasks
            INNER JOIN task_lists
              ON task_lists.id = tasks.list_id
             AND task_lists.deleted_at IS NULL
            WHERE tasks.deleted_at IS NULL
              AND tasks.status <> 'archived'
              AND (
                (?3 = 'list' AND tasks.list_id = ?4)
                OR (?3 = 'today' AND (
                  tasks.planned_start_date = ?6
                  OR tasks.due_date = ?6
                  OR EXISTS (
                    SELECT 1
                    FROM subtasks
                    WHERE subtasks.task_id = tasks.id
                      AND subtasks.deleted_at IS NULL
                      AND (
                        subtasks.planned_start_date = ?6
                        OR subtasks.due_date = ?6
                      )
                  )
                ))
                OR (?3 = 'favorites' AND tasks.is_favorite = 1)
                OR (?3 = 'tag' AND EXISTS (
                  SELECT 1
                  FROM task_tags
                  INNER JOIN tags
                    ON tags.id = task_tags.tag_id
                   AND tags.deleted_at IS NULL
                  WHERE task_tags.task_id = tasks.id
                    AND task_tags.tag_id = ?5
                    AND task_tags.deleted_at IS NULL
                ))
                OR ?3 = 'board'
              )
              AND (
                tasks.planned_start_date BETWEEN ?1 AND ?2
                OR tasks.due_date BETWEEN ?1 AND ?2
                OR (
                  tasks.scheduled_start_date <= ?2
                  AND tasks.scheduled_end_date >= ?1
                )
              )
            ",
        )
        .map_err(|error| format!("タスクカレンダークエリを準備できません: {error}"))?;

    let rows = statement
        .query_map(
            params![
                start_date,
                end_date,
                scope.as_str(),
                scope.list_id(),
                scope.tag_id(),
                today_date,
            ],
            |row| {
                Ok(CalendarSourceRow {
                    target_type: WorkTargetType::Task,
                    id: row.get(0)?,
                    title: row.get(1)?,
                    planned_start_date: row.get(2)?,
                    due_date: row.get(3)?,
                    due_time: row.get(4)?,
                    scheduled_start_date: row.get(5)?,
                    scheduled_start_time: row.get(6)?,
                    scheduled_end_date: row.get(7)?,
                    scheduled_end_time: row.get(8)?,
                    scheduled_is_all_day: row.get::<_, i64>(9)? != 0,
                    status: WorkStatus::from_db(&row.get::<_, String>(10)?)
                        .map_err(db_value_error)?,
                    color_token: row.get(11)?,
                    list_color_token: row.get(12)?,
                    parent_title: None,
                })
            },
        )
        .map_err(|error| format!("タスクカレンダーを取得できません: {error}"))?;

    for row in rows {
        push_calendar_items(
            row.map_err(|error| format!("タスク行を読めません: {error}"))?,
            items,
        );
    }
    Ok(())
}

fn collect_subtask_calendar_items(
    connection: &Connection,
    start_date: &str,
    end_date: &str,
    scope: &TaskPageScope,
    today_date: &str,
    items: &mut Vec<WeekCalendarItem>,
) -> RepositoryResult<()> {
    let mut statement = connection
        .prepare(
            "
            SELECT subtasks.id,
                   subtasks.title,
                   subtasks.planned_start_date,
                   subtasks.due_date,
                   subtasks.due_time,
                   subtasks.scheduled_start_date,
                   subtasks.scheduled_start_time,
                   subtasks.scheduled_end_date,
                   subtasks.scheduled_end_time,
                   subtasks.scheduled_is_all_day,
                   subtasks.status,
                   tasks.title AS parent_title,
                   COALESCE(tasks.color_token, task_lists.color_token) AS color_token,
                   task_lists.color_token AS list_color_token
            FROM subtasks
            INNER JOIN tasks
              ON tasks.id = subtasks.task_id
             AND tasks.deleted_at IS NULL
             AND tasks.status <> 'archived'
            INNER JOIN task_lists
              ON task_lists.id = tasks.list_id
             AND task_lists.deleted_at IS NULL
            WHERE subtasks.deleted_at IS NULL
              AND subtasks.status <> 'archived'
              AND (
                (?3 = 'list' AND tasks.list_id = ?4)
                OR (?3 = 'today' AND (
                  tasks.planned_start_date = ?6
                  OR tasks.due_date = ?6
                  OR EXISTS (
                    SELECT 1
                    FROM subtasks AS scope_subtasks
                    WHERE scope_subtasks.task_id = tasks.id
                      AND scope_subtasks.deleted_at IS NULL
                      AND (
                        scope_subtasks.planned_start_date = ?6
                        OR scope_subtasks.due_date = ?6
                      )
                  )
                ))
                OR (?3 = 'favorites' AND tasks.is_favorite = 1)
                OR (?3 = 'tag' AND EXISTS (
                  SELECT 1
                  FROM task_tags
                  INNER JOIN tags
                    ON tags.id = task_tags.tag_id
                   AND tags.deleted_at IS NULL
                  WHERE task_tags.task_id = tasks.id
                    AND task_tags.tag_id = ?5
                    AND task_tags.deleted_at IS NULL
                ))
                OR ?3 = 'board'
              )
              AND (
                subtasks.planned_start_date BETWEEN ?1 AND ?2
                OR subtasks.due_date BETWEEN ?1 AND ?2
                OR (
                  subtasks.scheduled_start_date <= ?2
                  AND subtasks.scheduled_end_date >= ?1
                )
              )
            ",
        )
        .map_err(|error| format!("サブタスクカレンダークエリを準備できません: {error}"))?;

    let rows = statement
        .query_map(
            params![
                start_date,
                end_date,
                scope.as_str(),
                scope.list_id(),
                scope.tag_id(),
                today_date,
            ],
            |row| {
                Ok(CalendarSourceRow {
                    target_type: WorkTargetType::Subtask,
                    id: row.get(0)?,
                    title: row.get(1)?,
                    planned_start_date: row.get(2)?,
                    due_date: row.get(3)?,
                    due_time: row.get(4)?,
                    scheduled_start_date: row.get(5)?,
                    scheduled_start_time: row.get(6)?,
                    scheduled_end_date: row.get(7)?,
                    scheduled_end_time: row.get(8)?,
                    scheduled_is_all_day: row.get::<_, i64>(9)? != 0,
                    status: WorkStatus::from_db(&row.get::<_, String>(10)?)
                        .map_err(db_value_error)?,
                    parent_title: row.get(11)?,
                    color_token: row.get(12)?,
                    list_color_token: row.get(13)?,
                })
            },
        )
        .map_err(|error| format!("サブタスクカレンダーを取得できません: {error}"))?;

    for row in rows {
        push_calendar_items(
            row.map_err(|error| format!("サブタスク行を読めません: {error}"))?,
            items,
        );
    }
    Ok(())
}

fn collect_active_timer_calendar_item(
    connection: &Connection,
    start_date: &str,
    end_date: &str,
    scope: &TaskPageScope,
    today_date: &str,
    items: &mut Vec<WeekCalendarItem>,
) -> RepositoryResult<()> {
    let active = connection
        .query_row(
            "
            SELECT timer_sessions.target_type,
                   timer_sessions.target_id,
                   timer_sessions.started_at,
                   COALESCE(task_targets.title, subtask_targets.title) AS title,
                   COALESCE(task_targets.status, subtask_targets.status) AS status,
                   parent_tasks.title AS parent_title,
                   COALESCE(
                     task_targets.color_token,
                     parent_tasks.color_token,
                     task_target_lists.color_token,
                     parent_task_lists.color_token,
                     ?7
                   ) AS color_token,
                   COALESCE(task_target_lists.color_token, parent_task_lists.color_token, ?7)
                     AS list_color_token
            FROM timer_sessions
            LEFT JOIN tasks AS task_targets
              ON timer_sessions.target_type = 'task'
             AND timer_sessions.target_id = task_targets.id
             AND task_targets.deleted_at IS NULL
             AND task_targets.status <> 'archived'
            LEFT JOIN subtasks AS subtask_targets
              ON timer_sessions.target_type = 'subtask'
             AND timer_sessions.target_id = subtask_targets.id
             AND subtask_targets.deleted_at IS NULL
             AND subtask_targets.status <> 'archived'
            LEFT JOIN tasks AS parent_tasks
              ON subtask_targets.task_id = parent_tasks.id
             AND parent_tasks.deleted_at IS NULL
             AND parent_tasks.status <> 'archived'
            LEFT JOIN task_lists AS task_target_lists
              ON task_targets.list_id = task_target_lists.id
             AND task_target_lists.deleted_at IS NULL
            LEFT JOIN task_lists AS parent_task_lists
              ON parent_tasks.list_id = parent_task_lists.id
             AND parent_task_lists.deleted_at IS NULL
            WHERE timer_sessions.stopped_at IS NULL
              AND timer_sessions.deleted_at IS NULL
              AND (
                (
                  timer_sessions.target_type = 'task'
                  AND task_targets.id IS NOT NULL
                )
                OR (
                  timer_sessions.target_type = 'subtask'
                  AND subtask_targets.id IS NOT NULL
                  AND parent_tasks.id IS NOT NULL
                )
              )
              AND substr(timer_sessions.started_at, 1, 10) BETWEEN ?1 AND ?2
              AND (
                (?3 = 'list' AND COALESCE(task_targets.list_id, parent_tasks.list_id) = ?4)
                OR (?3 = 'today' AND (
                  COALESCE(task_targets.planned_start_date, parent_tasks.planned_start_date) = ?6
                  OR COALESCE(task_targets.due_date, parent_tasks.due_date) = ?6
                  OR EXISTS (
                    SELECT 1
                    FROM subtasks AS scope_subtasks
                    WHERE scope_subtasks.task_id = COALESCE(task_targets.id, parent_tasks.id)
                      AND scope_subtasks.deleted_at IS NULL
                      AND (
                        scope_subtasks.planned_start_date = ?6
                        OR scope_subtasks.due_date = ?6
                      )
                  )
                ))
                OR (?3 = 'favorites' AND COALESCE(task_targets.is_favorite, parent_tasks.is_favorite) = 1)
                OR (?3 = 'tag' AND EXISTS (
                  SELECT 1
                  FROM task_tags
                  INNER JOIN tags
                    ON tags.id = task_tags.tag_id
                   AND tags.deleted_at IS NULL
                  WHERE task_tags.task_id = COALESCE(task_targets.id, parent_tasks.id)
                    AND task_tags.tag_id = ?5
                    AND task_tags.deleted_at IS NULL
                ))
                OR ?3 = 'board'
              )
            LIMIT 1
            ",
            params![
                start_date,
                end_date,
                scope.as_str(),
                scope.list_id(),
                scope.tag_id(),
                today_date,
                DEFAULT_TASK_LIST_COLOR_TOKEN,
            ],
            |row| {
                let target_type_text: String = row.get(0)?;
                let target_type =
                    WorkTargetType::from_db(&target_type_text).map_err(db_value_error)?;
                let started_at: String = row.get(2)?;
                let date = started_at.chars().take(10).collect::<String>();
                Ok(WeekCalendarItem {
                    id: format!(
                        "active:{}:{}",
                        target_type.as_str(),
                        row.get::<_, String>(1)?
                    ),
                    target: target_ref(target_type, row.get(1)?),
                    title: row.get(3)?,
                    parent_title: row.get(5)?,
                    date,
                    time: extract_iso_time(&started_at),
                    end_date: None,
                    end_time: None,
                    is_all_day: false,
                    marker: CalendarMarker::ActiveTimer,
                    status: WorkStatus::from_db(&row.get::<_, String>(4)?)
                        .map_err(db_value_error)?,
                    color_token: row.get(6)?,
                    list_color_token: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(|error| format!("アクティブタイマーのカレンダー項目を取得できません: {error}"))?;

    if let Some(item) = active {
        items.push(item);
    }
    Ok(())
}

struct CalendarSourceRow {
    target_type: WorkTargetType,
    id: String,
    title: String,
    planned_start_date: Option<String>,
    due_date: Option<String>,
    due_time: Option<String>,
    scheduled_start_date: Option<String>,
    scheduled_start_time: Option<String>,
    scheduled_end_date: Option<String>,
    scheduled_end_time: Option<String>,
    scheduled_is_all_day: bool,
    status: WorkStatus,
    parent_title: Option<String>,
    color_token: String,
    list_color_token: String,
}

fn push_calendar_items(row: CalendarSourceRow, items: &mut Vec<WeekCalendarItem>) {
    let target_type_text = row.target_type.as_str().to_string();
    if let (Some(date), Some(end_date)) = (
        row.scheduled_start_date.clone(),
        row.scheduled_end_date.clone(),
    ) {
        items.push(WeekCalendarItem {
            id: format!("{target_type_text}:{}:scheduled", row.id),
            target: target_ref(row.target_type.clone(), row.id.clone()),
            title: row.title.clone(),
            parent_title: row.parent_title.clone(),
            date,
            time: row.scheduled_start_time.clone(),
            end_date: Some(end_date),
            end_time: row.scheduled_end_time.clone(),
            is_all_day: row.scheduled_is_all_day,
            marker: CalendarMarker::Scheduled,
            status: row.status.clone(),
            color_token: row.color_token.clone(),
            list_color_token: row.list_color_token.clone(),
        });
    }
    if let Some(date) = row.planned_start_date {
        items.push(WeekCalendarItem {
            id: format!("{target_type_text}:{}:planned_start", row.id),
            target: target_ref(row.target_type.clone(), row.id.clone()),
            title: row.title.clone(),
            parent_title: row.parent_title.clone(),
            date,
            time: None,
            end_date: None,
            end_time: None,
            is_all_day: true,
            marker: CalendarMarker::PlannedStart,
            status: row.status.clone(),
            color_token: row.color_token.clone(),
            list_color_token: row.list_color_token.clone(),
        });
    }

    if let Some(date) = row.due_date {
        let due_time = row.due_time;
        let is_all_day = due_time.is_none();
        items.push(WeekCalendarItem {
            id: format!("{target_type_text}:{}:due", row.id),
            target: target_ref(row.target_type, row.id),
            title: row.title,
            parent_title: row.parent_title,
            date,
            time: due_time,
            end_date: None,
            end_time: None,
            is_all_day,
            marker: CalendarMarker::Due,
            status: row.status,
            color_token: row.color_token,
            list_color_token: row.list_color_token,
        });
    }
}

fn extract_iso_time(value: &str) -> Option<String> {
    value.get(11..16).map(ToString::to_string)
}

fn parse_date(value: &str, field_name: &str) -> RepositoryResult<Date> {
    Date::parse(value, DATE_FORMAT)
        .map_err(|error| format!("{field_name}の形式が不正です: {error}"))
}

fn format_date(value: Date) -> RepositoryResult<String> {
    value
        .format(DATE_FORMAT)
        .map_err(|error| format!("日付の整形に失敗しました: {error}"))
}

fn db_value_error(error: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{
            clock::Clock,
            notification::{LocalNotificationGateway, LocalNotificationMessage},
            repositories::{
                NotificationHistoryRepository, NotificationPreferenceRepository, TagRepository,
                TaskReadRepository, UiPreferenceRepository,
            },
            usecases,
        },
        domain::{
            notification::{
                NotificationDeliveryResult, NotificationDisplayMode, NotificationKind,
                NotificationOsRegistrationAction, NotificationOsRegistrationStatus,
            },
            pomodoro::{PomodoroPhase, PomodoroStatus},
            task::WorkStatus,
            timer::WorkTargetType,
        },
    };

    struct FixedClock {
        now: &'static str,
    }

    impl Clock for FixedClock {
        fn now_utc_iso8601(&self) -> String {
            self.now.to_string()
        }
    }

    struct RecordingNotificationGateway {
        messages: Mutex<Vec<LocalNotificationMessage>>,
        error: Option<&'static str>,
    }

    impl RecordingNotificationGateway {
        fn ok() -> Self {
            Self {
                messages: Mutex::new(Vec::new()),
                error: None,
            }
        }

        fn failing(error: &'static str) -> Self {
            Self {
                messages: Mutex::new(Vec::new()),
                error: Some(error),
            }
        }

        fn messages(&self) -> Vec<LocalNotificationMessage> {
            self.messages.lock().expect("messages").clone()
        }
    }

    impl LocalNotificationGateway for RecordingNotificationGateway {
        fn send(&self, message: &LocalNotificationMessage) -> Result<(), String> {
            self.messages
                .lock()
                .expect("messages")
                .push(message.clone());
            if let Some(error) = self.error {
                Err(error.to_string())
            } else {
                Ok(())
            }
        }
    }

    fn in_memory_database() -> SqliteDatabase {
        let connection = Connection::open_in_memory().expect("in-memory database");
        configure_connection(&connection).expect("configure");
        run_initial_migration(&connection).expect("migrate");
        seed_default_preferences(&connection).expect("seed");

        SqliteDatabase {
            path: PathBuf::from(":memory:"),
            connection: Mutex::new(connection),
        }
    }

    #[test]
    fn task_countdown_uses_start_time_snapshot_of_default_duration() {
        let database = in_memory_database();
        let start_clock = FixedClock {
            now: "2026-07-19T00:00:00Z",
        };
        let task = usecases::create_task(&database, &start_clock, draft("集中作業")).expect("task");
        usecases::update_task_timer_settings(
            &database,
            &start_clock,
            usecases::TaskTimerSettingsDraft {
                default_target_seconds: 120,
            },
        )
        .expect("settings");

        let active = usecases::start_timer(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start countdown");
        usecases::update_task_timer_settings(
            &database,
            &FixedClock {
                now: "2026-07-19T00:00:10Z",
            },
            usecases::TaskTimerSettingsDraft {
                default_target_seconds: 300,
            },
        )
        .expect("updated settings");

        assert_eq!(active.target_seconds, Some(120));
        assert_eq!(
            database
                .get_active_timer()
                .expect("active timer")
                .expect("timer")
                .target_seconds,
            Some(120)
        );
    }

    #[test]
    fn task_countdown_expires_once_after_completed_pause_time() {
        let database = in_memory_database();
        let notification_gateway = RecordingNotificationGateway::ok();
        let start_clock = FixedClock {
            now: "2026-07-19T00:00:00Z",
        };
        let task =
            usecases::create_task(&database, &start_clock, draft("見積書を確認")).expect("task");
        usecases::update_task_timer_settings(
            &database,
            &start_clock,
            usecases::TaskTimerSettingsDraft {
                default_target_seconds: 60,
            },
        )
        .expect("settings");
        usecases::start_timer(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start countdown");
        usecases::pause_active_timer(
            &database,
            &FixedClock {
                now: "2026-07-19T00:00:30Z",
            },
        )
        .expect("pause");

        let paused_sync = usecases::sync_expired_task_countdown(
            &database,
            &notification_gateway,
            &FixedClock {
                now: "2026-07-19T00:02:00Z",
            },
        )
        .expect("sync while paused");
        assert!(paused_sync.expired_timer.is_none());

        let resumed = usecases::resume_active_timer(
            &database,
            &FixedClock {
                now: "2026-07-19T00:02:00Z",
            },
        )
        .expect("resume");
        assert_eq!(resumed.accumulated_paused_seconds, 90);

        let result = usecases::sync_expired_task_countdown(
            &database,
            &notification_gateway,
            &FixedClock {
                now: "2026-07-19T00:02:30Z",
            },
        )
        .expect("sync expired countdown");
        let expired = result.expired_timer.expect("expired timer");
        assert_eq!(expired.elapsed_seconds, Some(60));
        assert_eq!(
            expired.completion_reason,
            Some(TimerCompletionReason::CountdownExpired)
        );
        assert_eq!(result.notification_summary.succeeded, 1);
        assert_eq!(notification_gateway.messages().len(), 1);
        assert_eq!(notification_gateway.messages()[0].title, "見積書を確認");

        let second = usecases::sync_expired_task_countdown(
            &database,
            &notification_gateway,
            &FixedClock {
                now: "2026-07-19T00:03:00Z",
            },
        )
        .expect("second sync");
        assert!(second.expired_timer.is_none());
        assert_eq!(notification_gateway.messages().len(), 1);
    }

    #[test]
    fn task_countdown_completes_without_notification_when_notifications_are_disabled() {
        let database = in_memory_database();
        let notification_gateway = RecordingNotificationGateway::ok();
        let start_clock = FixedClock {
            now: "2026-07-19T00:00:00Z",
        };
        let task = usecases::create_task(&database, &start_clock, draft("通知なしの集中作業"))
            .expect("task");
        usecases::update_task_timer_settings(
            &database,
            &start_clock,
            usecases::TaskTimerSettingsDraft {
                default_target_seconds: 60,
            },
        )
        .expect("settings");
        usecases::update_notifications_enabled(&database, &start_clock, false)
            .expect("disable notifications");
        usecases::start_timer(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start countdown");

        let result = usecases::sync_expired_task_countdown(
            &database,
            &notification_gateway,
            &FixedClock {
                now: "2026-07-19T00:01:00Z",
            },
        )
        .expect("sync expired countdown");

        assert_eq!(
            result
                .expired_timer
                .expect("expired timer")
                .completion_reason,
            Some(TimerCompletionReason::CountdownExpired)
        );
        assert_eq!(result.notification_summary.attempted, 0);
        assert!(notification_gateway.messages().is_empty());
        assert!(database.get_active_timer().expect("active timer").is_none());
    }

    fn draft(title: &str) -> usecases::WorkItemDraft {
        usecases::WorkItemDraft {
            list_id: None,
            title: title.to_string(),
            planned_start_date: None,
            due_date: None,
            due_time: None,
            memo: None,
        }
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4()))
    }

    fn create_legacy_task_schema(connection: &Connection) {
        connection
            .execute_batch(
                "
                CREATE TABLE tasks (
                  id TEXT PRIMARY KEY,
                  title TEXT NOT NULL CHECK (length(trim(title)) > 0),
                  status TEXT NOT NULL CHECK (status IN ('todo', 'in_progress', 'done', 'archived')),
                  planned_start_date TEXT NULL,
                  due_date TEXT NULL,
                  memo TEXT NOT NULL DEFAULT '',
                  sort_order INTEGER NOT NULL DEFAULT 0,
                  completed_at TEXT NULL,
                  deleted_at TEXT NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );

                CREATE TABLE subtasks (
                  id TEXT PRIMARY KEY,
                  task_id TEXT NOT NULL,
                  title TEXT NOT NULL CHECK (length(trim(title)) > 0),
                  status TEXT NOT NULL CHECK (status IN ('todo', 'in_progress', 'done', 'archived')),
                  planned_start_date TEXT NULL,
                  due_date TEXT NULL,
                  memo TEXT NOT NULL DEFAULT '',
                  sort_order INTEGER NOT NULL DEFAULT 0,
                  completed_at TEXT NULL,
                  deleted_at TEXT NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL,
                  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE RESTRICT
                );
                ",
            )
            .expect("legacy task schema");
    }

    fn insert_notification_rule(
        database: &SqliteDatabase,
        target_type: WorkTargetType,
        target_id: &str,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        database
            .with_connection(|connection| {
                connection
                    .execute(
                        "
                        INSERT INTO notification_rules (
                          id, target_type, target_id, kind, notify_at, enabled,
                          registration_status, created_at, updated_at
                        )
                        VALUES (?1, ?2, ?3, 'due', ?4, 1, 'registered', ?4, ?4)
                        ",
                        params![id, target_type.as_str(), target_id, "2026-07-06T00:00:00Z"],
                    )
                    .map_err(|error| format!("insert notification rule: {error}"))?;
                Ok(())
            })
            .expect("insert notification rule");
        id
    }

    fn insert_recurrence_rule(
        database: &SqliteDatabase,
        target_type: WorkTargetType,
        target_id: &str,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        database
            .with_connection(|connection| {
                connection
                    .execute(
                        "
                        INSERT INTO recurrence_rules (
                          id, target_type, target_id, frequency, interval, created_at, updated_at
                        )
                        VALUES (?1, ?2, ?3, 'weekly', 1, ?4, ?4)
                        ",
                        params![id, target_type.as_str(), target_id, "2026-07-06T00:00:00Z"],
                    )
                    .map_err(|error| format!("insert recurrence rule: {error}"))?;
                Ok(())
            })
            .expect("insert recurrence rule");
        id
    }

    #[test]
    fn migration_initializes_notification_preference() {
        let connection = Connection::open_in_memory().expect("in-memory database");
        configure_connection(&connection).expect("configure");
        run_initial_migration(&connection).expect("migrate");
        seed_default_preferences(&connection).expect("seed");

        let (display_mode, notifications_enabled): (String, i64) = connection
            .query_row(
                "
                SELECT display_mode, notifications_enabled
                FROM notification_preferences
                WHERE id = 'default'
                ",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("default preference");

        assert_eq!(display_mode, "title_only");
        assert_eq!(notifications_enabled, 1);
    }

    #[test]
    fn migration_initializes_pomodoro_settings() {
        let connection = Connection::open_in_memory().expect("in-memory database");
        configure_connection(&connection).expect("configure");
        run_initial_migration(&connection).expect("migrate");

        let (
            work_seconds,
            short_break_seconds,
            long_break_seconds,
            cycles_until_long_break,
            auto_start_break,
            auto_start_next_work,
        ): (i64, i64, i64, i64, i64, i64) = connection
            .query_row(
                "
                SELECT work_seconds, short_break_seconds, long_break_seconds,
                       cycles_until_long_break, auto_start_break, auto_start_next_work
                FROM pomodoro_settings
                WHERE id = 'default'
                ",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("default pomodoro settings");
        let table_count: i64 = connection
            .query_row(
                "
                SELECT COUNT(*)
                FROM sqlite_master
                WHERE type = 'table'
                  AND name IN ('pomodoro_settings', 'pomodoro_sessions')
                ",
                [],
                |row| row.get(0),
            )
            .expect("pomodoro tables");

        assert_eq!(work_seconds, DEFAULT_POMODORO_WORK_SECONDS);
        assert_eq!(short_break_seconds, DEFAULT_POMODORO_SHORT_BREAK_SECONDS);
        assert_eq!(long_break_seconds, DEFAULT_POMODORO_LONG_BREAK_SECONDS);
        assert_eq!(
            cycles_until_long_break,
            DEFAULT_POMODORO_CYCLES_UNTIL_LONG_BREAK
        );
        assert_eq!(auto_start_break, 0);
        assert_eq!(auto_start_next_work, 0);
        assert_eq!(table_count, 2);
    }

    #[test]
    fn schema_allows_only_one_active_pomodoro() {
        let connection = Connection::open_in_memory().expect("in-memory database");
        configure_connection(&connection).expect("configure");
        run_initial_migration(&connection).expect("migrate");

        connection
            .execute(
                "
                INSERT INTO pomodoro_sessions (
                  id, scope, target_type, target_id, phase, status, cycle_count,
                  phase_started_at, phase_duration_seconds, paused_total_seconds,
                  created_at, updated_at
                )
                VALUES (
                  'pomodoro-1', 'task_linked', 'task', 'task-1', 'short_break', 'running', 1,
                  '2026-07-05T00:00:00Z', 300, 0,
                  '2026-07-05T00:00:00Z', '2026-07-05T00:00:00Z'
                )
                ",
                [],
            )
            .expect("first active pomodoro");

        let result = connection.execute(
            "
            INSERT INTO pomodoro_sessions (
              id, scope, target_type, target_id, phase, status, cycle_count,
              phase_started_at, phase_duration_seconds, paused_total_seconds,
              created_at, updated_at
            )
            VALUES (
              'pomodoro-2', 'task_linked', 'task', 'task-2', 'short_break', 'paused', 1,
              '2026-07-05T00:01:00Z', 300, 0,
              '2026-07-05T00:01:00Z', '2026-07-05T00:01:00Z'
            )
            ",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn standalone_pomodoro_scope_rejects_task_target_columns() {
        let database = in_memory_database();
        let result = database.with_connection(|connection| {
            connection
                .execute(
                    "
                    INSERT INTO pomodoro_sessions (
                      id, scope, target_type, target_id, phase, status, cycle_count,
                      phase_started_at, phase_duration_seconds, paused_total_seconds,
                      created_at, updated_at
                    )
                    VALUES (
                      'invalid-standalone', 'standalone', 'task', 'task-1',
                      'short_break', 'completed', 1, '2026-07-05T00:00:00Z', 300, 0,
                      '2026-07-05T00:00:00Z', '2026-07-05T00:00:00Z'
                    )
                    ",
                    [],
                )
                .map(|_| ())
                .map_err(|error| error.to_string())
        });

        assert!(result.is_err());
    }

    #[test]
    fn legacy_pomodoro_rows_migrate_to_task_linked_scope() {
        let connection = Connection::open_in_memory().expect("in-memory database");
        configure_connection(&connection).expect("configure");
        run_initial_migration(&connection).expect("initial migrate");
        connection
            .execute_batch(
                "
                DROP TABLE pomodoro_sessions;
                CREATE TABLE pomodoro_sessions (
                  id TEXT PRIMARY KEY,
                  target_type TEXT NOT NULL CHECK (target_type IN ('task', 'subtask')),
                  target_id TEXT NOT NULL,
                  timer_session_id TEXT NULL,
                  phase TEXT NOT NULL CHECK (phase IN ('work', 'short_break', 'long_break')),
                  status TEXT NOT NULL CHECK (status IN ('running', 'paused', 'completed', 'cancelled')),
                  cycle_count INTEGER NOT NULL DEFAULT 0,
                  phase_started_at TEXT NOT NULL,
                  phase_duration_seconds INTEGER NOT NULL,
                  paused_at TEXT NULL,
                  paused_total_seconds INTEGER NOT NULL DEFAULT 0,
                  completed_at TEXT NULL,
                  cancelled_at TEXT NULL,
                  deleted_at TEXT NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );
                INSERT INTO pomodoro_sessions (
                  id, target_type, target_id, phase, status, cycle_count,
                  phase_started_at, phase_duration_seconds, paused_total_seconds,
                  completed_at, created_at, updated_at
                ) VALUES (
                  'legacy-break', 'task', 'legacy-task', 'short_break', 'completed', 1,
                  '2026-07-05T00:00:00Z', 300, 0, '2026-07-05T00:05:00Z',
                  '2026-07-05T00:00:00Z', '2026-07-05T00:05:00Z'
                );
                ",
            )
            .expect("legacy schema");

        migrate_pomodoro_sessions_to_scoped_model(&connection).expect("scoped migration");
        let row: (String, String, String) = connection
            .query_row(
                "SELECT scope, target_type, target_id FROM pomodoro_sessions WHERE id = 'legacy-break'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("migrated row");
        assert_eq!(
            row,
            ("task_linked".into(), "task".into(), "legacy-task".into())
        );

        connection
            .execute(
                "
                INSERT INTO pomodoro_sessions (
                  id, scope, phase, status, cycle_count, phase_started_at,
                  phase_duration_seconds, paused_total_seconds, created_at, updated_at
                ) VALUES (
                  'standalone-after-migration', 'standalone', 'work', 'completed', 1,
                  '2026-07-05T01:00:00Z', 1500, 0,
                  '2026-07-05T01:00:00Z', '2026-07-05T01:25:00Z'
                )
                ",
                [],
            )
            .expect("standalone row");
    }

    #[test]
    fn update_pomodoro_settings_validates_ranges() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };

        let updated = usecases::update_pomodoro_settings(
            &database,
            &clock,
            usecases::PomodoroSettingsDraft {
                work_seconds: 30 * 60,
                short_break_seconds: 3 * 60,
                long_break_seconds: 20 * 60,
                cycles_until_long_break: 3,
                auto_start_break: true,
                auto_start_next_work: false,
            },
        )
        .expect("update pomodoro settings");
        assert_eq!(updated.work_seconds, 30 * 60);
        assert_eq!(updated.short_break_seconds, 3 * 60);
        assert_eq!(updated.long_break_seconds, 20 * 60);
        assert_eq!(updated.cycles_until_long_break, 3);
        assert!(updated.auto_start_break);
        assert!(!updated.auto_start_next_work);

        let invalid = usecases::update_pomodoro_settings(
            &database,
            &clock,
            usecases::PomodoroSettingsDraft {
                work_seconds: 0,
                short_break_seconds: 3 * 60,
                long_break_seconds: 20 * 60,
                cycles_until_long_break: 3,
                auto_start_break: false,
                auto_start_next_work: false,
            },
        );
        assert!(invalid
            .expect_err("invalid work seconds")
            .contains("作業時間"));
    }

    #[test]
    fn ui_preferences_defaults_update_and_fallback_are_safe() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };

        let initial = database
            .get_ui_preferences()
            .expect("initial ui preferences");
        assert!(initial.left_pane_open);
        assert_eq!(initial.last_view, UI_VIEW_LIST);
        assert_eq!(initial.last_task_list_id, DEFAULT_TASK_LIST_ID);
        assert_eq!(initial.calendar_view_mode, CALENDAR_VIEW_WEEK);

        let updated = usecases::update_ui_preferences(
            &database,
            &clock,
            usecases::UiPreferencesDraft {
                left_pane_open: false,
                last_view: UI_VIEW_BOARD.to_string(),
                last_task_list_id: "custom-list".to_string(),
                calendar_view_mode: CALENDAR_VIEW_MONTH.to_string(),
            },
        )
        .expect("update ui preferences");
        assert!(!updated.left_pane_open);
        assert_eq!(updated.last_view, UI_VIEW_BOARD);
        assert_eq!(updated.last_task_list_id, "custom-list");
        assert_eq!(updated.calendar_view_mode, CALENDAR_VIEW_MONTH);

        database
            .with_connection(|connection| {
                connection
                    .execute(
                        "
                        UPDATE ui_preferences
                        SET value = 'broken'
                        WHERE key IN ('left_pane_open', 'last_view', 'calendar_view_mode')
                        ",
                        [],
                    )
                    .map_err(|error| format!("break ui preferences: {error}"))?;
                connection
                    .execute(
                        "
                        UPDATE ui_preferences
                        SET value = ''
                        WHERE key = 'last_task_list_id'
                        ",
                        [],
                    )
                    .map_err(|error| format!("break last list: {error}"))?;
                Ok(())
            })
            .expect("break preferences");

        let fallback = database.get_ui_preferences().expect("fallback preferences");
        assert!(fallback.left_pane_open);
        assert_eq!(fallback.last_view, UI_VIEW_LIST);
        assert_eq!(fallback.last_task_list_id, DEFAULT_TASK_LIST_ID);
        assert_eq!(fallback.calendar_view_mode, CALENDAR_VIEW_WEEK);

        let invalid = usecases::update_ui_preferences(
            &database,
            &clock,
            usecases::UiPreferencesDraft {
                left_pane_open: true,
                last_view: "network".to_string(),
                last_task_list_id: DEFAULT_TASK_LIST_ID.to_string(),
                calendar_view_mode: CALENDAR_VIEW_DAY.to_string(),
            },
        );
        assert!(invalid.expect_err("invalid ui view").contains("ビュー"));
    }

    #[test]
    fn migration_backfills_failed_notification_delivery_attempts() {
        let connection = Connection::open_in_memory().expect("in-memory database");
        configure_connection(&connection).expect("configure");
        connection
            .execute_batch(
                "
                CREATE TABLE notification_rules (
                  id TEXT PRIMARY KEY,
                  target_type TEXT NOT NULL CHECK (target_type IN ('task', 'subtask')),
                  target_id TEXT NOT NULL,
                  kind TEXT NOT NULL CHECK (kind IN ('planned_start', 'due')),
                  notify_at TEXT NOT NULL,
                  enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
                  registration_status TEXT NOT NULL CHECK (
                    registration_status IN ('pending', 'registered', 'failed', 'disabled')
                  ),
                  last_error TEXT NULL,
                  deleted_at TEXT NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );

                INSERT INTO notification_rules (
                  id, target_type, target_id, kind, notify_at, enabled,
                  registration_status, last_error, created_at, updated_at
                )
                VALUES (
                  'failed-rule', 'task', 'task-1', 'due', '2026-07-06T00:00:00Z',
                  1, 'failed', 'permission denied',
                  '2026-07-06T00:00:00Z', '2026-07-06T00:05:00Z'
                );
                ",
            )
            .expect("legacy notification rules");

        run_initial_migration(&connection).expect("migrate");

        let (count, error_message): (i64, String) = connection
            .query_row(
                "
                SELECT COUNT(*), MAX(error_message)
                FROM notification_delivery_attempts
                WHERE notification_rule_id = 'failed-rule'
                  AND result = 'failed'
                ",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("backfilled attempts");

        assert_eq!(count, 1);
        assert_eq!(error_message, "permission denied");
    }

    #[test]
    fn migration_backfills_notification_os_registrations() {
        let connection = Connection::open_in_memory().expect("in-memory database");
        configure_connection(&connection).expect("configure");
        connection
            .execute_batch(
                "
                CREATE TABLE notification_rules (
                  id TEXT PRIMARY KEY,
                  target_type TEXT NOT NULL CHECK (target_type IN ('task', 'subtask')),
                  target_id TEXT NOT NULL,
                  kind TEXT NOT NULL CHECK (kind IN ('planned_start', 'due')),
                  notify_at TEXT NOT NULL,
                  enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
                  registration_status TEXT NOT NULL CHECK (
                    registration_status IN ('pending', 'registered', 'failed', 'disabled')
                  ),
                  last_error TEXT NULL,
                  deleted_at TEXT NULL,
                  created_at TEXT NOT NULL,
                  updated_at TEXT NOT NULL
                );

                INSERT INTO notification_rules (
                  id, target_type, target_id, kind, notify_at, enabled,
                  registration_status, created_at, updated_at
                )
                VALUES
                  (
                    'active-rule', 'task', 'task-1', 'due',
                    '2026-07-06T09:00:00Z', 1, 'pending',
                    '2026-07-06T00:00:00Z', '2026-07-06T00:00:00Z'
                  ),
                  (
                    'disabled-rule', 'task', 'task-2', 'due',
                    '2026-07-06T09:00:00Z', 0, 'disabled',
                    '2026-07-06T00:00:00Z', '2026-07-06T00:05:00Z'
                  );
                ",
            )
            .expect("legacy notification rules");

        run_initial_migration(&connection).expect("migrate");

        let active_status: String = connection
            .query_row(
                "
                SELECT registration_status
                FROM notification_os_registrations
                WHERE notification_rule_id = 'active-rule'
                  AND deleted_at IS NULL
                ",
                [],
                |row| row.get(0),
            )
            .expect("active os registration");
        let disabled_count: i64 = connection
            .query_row(
                "
                SELECT COUNT(*)
                FROM notification_os_registrations
                WHERE notification_rule_id = 'disabled-rule'
                  AND registration_status = 'disabled'
                  AND deleted_at IS NOT NULL
                ",
                [],
                |row| row.get(0),
            )
            .expect("disabled os registration");

        assert_eq!(active_status, "pending");
        assert_eq!(disabled_count, 1);
    }

    #[test]
    fn migration_backfills_ui_read_model_defaults_for_existing_database() {
        let connection = Connection::open_in_memory().expect("in-memory database");
        configure_connection(&connection).expect("configure");
        create_legacy_task_schema(&connection);
        connection
            .execute_batch(
                "
                INSERT INTO tasks (
                  id, title, status, memo, sort_order, created_at, updated_at
                )
                VALUES (
                  'legacy-task', '既存タスク', 'todo', '', 0,
                  '2026-07-06T00:00:00Z', '2026-07-06T00:00:00Z'
                );
                ",
            )
            .expect("legacy schema");

        run_initial_migration(&connection).expect("migrate legacy database");

        let (
            list_id,
            board_column_id,
            lifecycle_status,
            is_favorite,
            timer_target_seconds,
            color_token,
        ): (String, String, String, i64, Option<i64>, Option<String>) = connection
            .query_row(
                "
                SELECT list_id, board_column_id, lifecycle_status,
                       is_favorite, timer_target_seconds, color_token
                FROM tasks
                WHERE id = 'legacy-task'
                ",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("migrated task");
        let (task_list_name, task_list_color_token): (String, String) = connection
            .query_row(
                "SELECT name, color_token FROM task_lists WHERE id = 'default'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("default task list");
        let ui_preference_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM ui_preferences", [], |row| row.get(0))
            .expect("ui preferences");
        let timer_recurrence_table_count: i64 = connection
            .query_row(
                "
                SELECT COUNT(*)
                FROM sqlite_master
                WHERE type = 'table'
                  AND name IN ('timer_pauses', 'recurrence_rules')
                ",
                [],
                |row| row.get(0),
            )
            .expect("timer recurrence tables");
        let default_task_timer_seconds: i64 = connection
            .query_row(
                "SELECT default_target_seconds FROM task_timer_settings WHERE id = 'default'",
                [],
                |row| row.get(0),
            )
            .expect("default task timer settings");

        assert_eq!(list_id, DEFAULT_TASK_LIST_ID);
        assert_eq!(board_column_id, DEFAULT_BOARD_COLUMN_ID);
        assert_eq!(lifecycle_status, "active");
        assert_eq!(is_favorite, 0);
        assert_eq!(timer_target_seconds, None);
        assert_eq!(color_token, None);
        assert_eq!(task_list_name, DEFAULT_TASK_LIST_NAME);
        assert_eq!(task_list_color_token, DEFAULT_TASK_LIST_COLOR_TOKEN);
        assert_eq!(ui_preference_count, 4);
        assert_eq!(timer_recurrence_table_count, 2);
        assert_eq!(
            default_task_timer_seconds,
            DEFAULT_TASK_TIMER_TARGET_SECONDS
        );
        assert!(
            column_exists(&connection, "subtasks", "timer_target_seconds")
                .expect("subtask timer target column")
        );
        for column_name in [
            "target_seconds",
            "completion_reason",
            "completion_notified_at",
        ] {
            assert!(column_exists(&connection, "timer_sessions", column_name)
                .expect("task countdown timer session column"));
        }
    }

    #[test]
    fn opening_existing_app_database_adds_schedule_columns_before_indexes() {
        let data_dir = temp_dir("tasktimer-schedule-startup-migration");
        fs::create_dir_all(&data_dir).expect("create data directory");
        let database_path = data_dir.join("tasktimer.sqlite3");
        {
            let connection = Connection::open(&database_path).expect("open legacy database");
            configure_connection(&connection).expect("configure legacy database");
            create_legacy_task_schema(&connection);
        }

        {
            let database = SqliteDatabase::open_in_dir(data_dir.clone())
                .expect("open and migrate existing app database");
            database
                .with_connection(|connection| {
                    for table_name in ["tasks", "subtasks"] {
                        for column_name in [
                            "scheduled_start_date",
                            "scheduled_start_time",
                            "scheduled_end_date",
                            "scheduled_end_time",
                            "scheduled_is_all_day",
                        ] {
                            assert!(column_exists(connection, table_name, column_name)?);
                        }
                    }
                    let schedule_index_count: i64 = connection
                        .query_row(
                            "
                            SELECT COUNT(*)
                            FROM sqlite_master
                            WHERE type = 'index'
                              AND name IN (
                                'tasks_schedule_range_idx',
                                'subtasks_schedule_range_idx'
                              )
                            ",
                            [],
                            |row| row.get(0),
                        )
                        .map_err(|error| {
                            format!("予定期間インデックスを確認できません: {error}")
                        })?;
                    assert_eq!(schedule_index_count, 2);
                    Ok(())
                })
                .expect("verify schedule migration");
        }

        fs::remove_dir_all(data_dir).expect("remove data directory");
    }

    #[test]
    fn schema_allows_only_one_active_timer() {
        let connection = Connection::open_in_memory().expect("in-memory database");
        configure_connection(&connection).expect("configure");
        run_initial_migration(&connection).expect("migrate");

        connection
            .execute(
                "
                INSERT INTO timer_sessions (
                  id, target_type, target_id, started_at, created_at
                )
                VALUES ('timer-1', 'task', 'task-1', '2026-07-05T00:00:00Z', '2026-07-05T00:00:00Z')
                ",
                [],
            )
            .expect("first active timer");

        let result = connection.execute(
            "
            INSERT INTO timer_sessions (
              id, target_type, target_id, started_at, created_at
            )
            VALUES ('timer-2', 'task', 'task-2', '2026-07-05T00:01:00Z', '2026-07-05T00:01:00Z')
            ",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn create_subtask_rejects_missing_parent() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };

        let result =
            usecases::create_subtask(&database, &clock, "missing-task".to_string(), draft("調査"));

        assert!(result.expect_err("missing parent").contains("親タスク"));
    }

    #[test]
    fn start_timer_rejects_second_active_timer() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let first_task =
            usecases::create_task(&database, &clock, draft("実装")).expect("first task");
        let second_task =
            usecases::create_task(&database, &clock, draft("レビュー")).expect("second task");
        let first_task_id = first_task.id.clone();

        usecases::start_timer(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: first_task.id.clone(),
            },
        )
        .expect("first timer");
        let first_task_after_start = database
            .with_connection(|connection| select_task_by_id(connection, &first_task_id))
            .expect("first task after start");
        let first_task_row = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("task rows")
            .into_iter()
            .find(|row| row.id == first_task_id)
            .expect("first task row");
        assert_eq!(first_task_after_start.status, WorkStatus::InProgress);
        assert_eq!(first_task_row.board_column_id, IN_PROGRESS_BOARD_COLUMN_ID);

        let result = usecases::start_timer(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: second_task.id,
            },
        );

        assert!(result.expect_err("second active timer").contains("開始中"));
    }

    #[test]
    fn standalone_pomodoro_does_not_create_task_timer_history() {
        let database = in_memory_database();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };

        let active = usecases::start_standalone_pomodoro(&database, &start_clock)
            .expect("start standalone pomodoro");
        let timer_count: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM timer_sessions", [], |row| row.get(0))
                    .map_err(|error| error.to_string())
            })
            .expect("timer count");

        assert_eq!(active.scope, PomodoroScope::Standalone);
        assert_eq!(active.target, None);
        assert_eq!(active.timer_session_id, None);
        assert_eq!(active.phase, PomodoroPhase::Work);
        assert_eq!(active.status, PomodoroStatus::Running);
        assert_eq!(timer_count, 0);

        let paused = usecases::pause_pomodoro(
            &database,
            &FixedClock {
                now: "2026-07-06T00:05:00Z",
            },
        )
        .expect("pause standalone pomodoro");
        assert_eq!(paused.status, PomodoroStatus::Paused);

        let resumed = usecases::resume_pomodoro(
            &database,
            &FixedClock {
                now: "2026-07-06T00:07:00Z",
            },
        )
        .expect("resume standalone pomodoro");
        assert_eq!(resumed.paused_total_seconds, 120);

        let completed = usecases::complete_pomodoro_work_phase(
            &database,
            &FixedClock {
                now: "2026-07-06T00:12:00Z",
            },
        )
        .expect("complete standalone work");
        let break_session = usecases::start_pomodoro_break(
            &database,
            &FixedClock {
                now: "2026-07-06T00:12:00Z",
            },
            completed.id,
        )
        .expect("start standalone break");
        let next_work = usecases::skip_pomodoro_break(
            &database,
            &FixedClock {
                now: "2026-07-06T00:13:00Z",
            },
            break_session.id,
        )
        .expect("start next standalone work");
        let final_timer_count: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM timer_sessions", [], |row| row.get(0))
                    .map_err(|error| error.to_string())
            })
            .expect("final timer count");

        assert_eq!(next_work.scope, PomodoroScope::Standalone);
        assert_eq!(next_work.target, None);
        assert_eq!(next_work.timer_session_id, None);
        assert_eq!(final_timer_count, 0);
    }

    #[test]
    fn standalone_pomodoro_auto_transitions_without_task_timer() {
        let database = in_memory_database();
        let notification_gateway = RecordingNotificationGateway::ok();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        usecases::update_pomodoro_settings(
            &database,
            &start_clock,
            usecases::PomodoroSettingsDraft {
                work_seconds: 60,
                short_break_seconds: 60,
                long_break_seconds: 120,
                cycles_until_long_break: 4,
                auto_start_break: true,
                auto_start_next_work: true,
            },
        )
        .expect("settings");
        usecases::start_standalone_pomodoro(&database, &start_clock).expect("start work");

        let work_expiry = usecases::sync_expired_pomodoro(
            &database,
            &notification_gateway,
            &FixedClock {
                now: "2026-07-06T00:01:10Z",
            },
        )
        .expect("sync work");
        let active_break = work_expiry.active_pomodoro.expect("active break");
        assert_eq!(active_break.scope, PomodoroScope::Standalone);
        assert_eq!(active_break.phase, PomodoroPhase::ShortBreak);
        assert_eq!(active_break.target, None);

        let break_expiry = usecases::sync_expired_pomodoro(
            &database,
            &notification_gateway,
            &FixedClock {
                now: "2026-07-06T00:02:10Z",
            },
        )
        .expect("sync break");
        let active_work = break_expiry.active_pomodoro.expect("next work");
        let timer_count: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM timer_sessions", [], |row| row.get(0))
                    .map_err(|error| error.to_string())
            })
            .expect("timer count");

        assert_eq!(active_work.scope, PomodoroScope::Standalone);
        assert_eq!(active_work.phase, PomodoroPhase::Work);
        assert_eq!(active_work.target, None);
        assert_eq!(active_work.timer_session_id, None);
        assert_eq!(timer_count, 0);
        assert_eq!(notification_gateway.messages().len(), 2);
    }

    #[test]
    fn start_pomodoro_creates_work_timer_and_active_session() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task =
            usecases::create_task(&database, &clock, draft("ポモドーロ作業")).expect("create task");

        let active = usecases::start_legacy_task_linked_pomodoro(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id.clone(),
            },
        )
        .expect("start pomodoro");

        assert_eq!(active.scope, PomodoroScope::TaskLinked);
        assert_eq!(
            active.target.as_ref().map(|target| &target.target_type),
            Some(&WorkTargetType::Task)
        );
        assert_eq!(
            active.target.as_ref().map(|target| target.id.as_str()),
            Some(task.id.as_str())
        );
        assert_eq!(active.phase, PomodoroPhase::Work);
        assert_eq!(active.status, PomodoroStatus::Running);
        assert_eq!(active.phase_started_at, "2026-07-06T00:00:00Z");
        assert_eq!(active.phase_duration_seconds, DEFAULT_POMODORO_WORK_SECONDS);
        let timer_session_id = active
            .timer_session_id
            .as_deref()
            .expect("work phase timer session");
        let active_timer = database
            .get_active_timer()
            .expect("active timer")
            .expect("pomodoro work timer");
        assert_eq!(active_timer.id, timer_session_id);

        let active_pomodoro = usecases::get_active_pomodoro(&database)
            .expect("active pomodoro")
            .expect("active pomodoro");
        assert_eq!(active_pomodoro.id, active.id);
        let updated_task = database
            .with_connection(|connection| select_task_by_id(connection, &task.id))
            .expect("updated task");
        assert_eq!(updated_task.status, WorkStatus::InProgress);
    }

    #[test]
    fn timer_pause_resume_and_stop_syncs_active_pomodoro_work_phase() {
        let database = in_memory_database();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let pause_clock = FixedClock {
            now: "2026-07-06T00:02:00Z",
        };
        let resume_clock = FixedClock {
            now: "2026-07-06T00:05:00Z",
        };
        let stop_clock = FixedClock {
            now: "2026-07-06T00:07:00Z",
        };
        let task =
            usecases::create_task(&database, &start_clock, draft("ポモドーロ同期")).expect("task");
        let pomodoro = usecases::start_legacy_task_linked_pomodoro(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start pomodoro");

        usecases::pause_active_timer(&database, &pause_clock).expect("pause pomodoro timer");
        let paused = usecases::get_active_pomodoro(&database)
            .expect("active pomodoro")
            .expect("paused pomodoro");
        assert_eq!(paused.status, PomodoroStatus::Paused);
        assert_eq!(paused.paused_at.as_deref(), Some("2026-07-06T00:02:00Z"));

        usecases::resume_active_timer(&database, &resume_clock).expect("resume pomodoro timer");
        let resumed = usecases::get_active_pomodoro(&database)
            .expect("active pomodoro")
            .expect("resumed pomodoro");
        assert_eq!(resumed.status, PomodoroStatus::Running);
        assert_eq!(resumed.paused_at, None);
        assert_eq!(resumed.paused_total_seconds, 180);

        let stopped =
            usecases::stop_active_timer(&database, &stop_clock).expect("stop pomodoro timer");
        let active_pomodoro = usecases::get_active_pomodoro(&database).expect("active pomodoro");
        let (status, cancelled_at, paused_total_seconds): (String, String, i64) = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "
                        SELECT status, cancelled_at, paused_total_seconds
                        FROM pomodoro_sessions
                        WHERE id = ?1
                        ",
                        params![pomodoro.id.as_str()],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_err(|error| format!("pomodoro row: {error}"))
            })
            .expect("pomodoro row");

        assert_eq!(stopped.elapsed_seconds, Some(240));
        assert!(active_pomodoro.is_none());
        assert_eq!(status, PomodoroStatus::Cancelled.as_str());
        assert_eq!(cancelled_at, "2026-07-06T00:07:00Z");
        assert_eq!(paused_total_seconds, 180);
    }

    #[test]
    fn pomodoro_work_can_pause_resume_complete_and_start_short_break() {
        let database = in_memory_database();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let pause_clock = FixedClock {
            now: "2026-07-06T00:05:00Z",
        };
        let resume_clock = FixedClock {
            now: "2026-07-06T00:07:00Z",
        };
        let complete_clock = FixedClock {
            now: "2026-07-06T00:12:00Z",
        };
        let break_clock = FixedClock {
            now: "2026-07-06T00:12:30Z",
        };
        let task =
            usecases::create_task(&database, &start_clock, draft("ポモドーロ完了")).expect("task");
        let pomodoro = usecases::start_legacy_task_linked_pomodoro(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start pomodoro");
        let timer_id = pomodoro.timer_session_id.clone().expect("work phase timer");

        let paused =
            usecases::pause_pomodoro(&database, &pause_clock).expect("pause pomodoro work");
        assert_eq!(paused.status, PomodoroStatus::Paused);
        assert_eq!(paused.paused_at.as_deref(), Some("2026-07-06T00:05:00Z"));

        let resumed =
            usecases::resume_pomodoro(&database, &resume_clock).expect("resume pomodoro work");
        assert_eq!(resumed.status, PomodoroStatus::Running);
        assert_eq!(resumed.paused_at, None);
        assert_eq!(resumed.paused_total_seconds, 120);

        let completed = usecases::complete_pomodoro_work_phase(&database, &complete_clock)
            .expect("complete pomodoro work");
        let stopped_timer = database
            .with_connection(|connection| select_timer_by_id(connection, &timer_id))
            .expect("stopped pomodoro timer");
        assert_eq!(completed.status, PomodoroStatus::Completed);
        assert_eq!(completed.phase, PomodoroPhase::Work);
        assert_eq!(completed.cycle_count, 1);
        assert_eq!(completed.paused_total_seconds, 120);
        assert_eq!(
            completed.completed_at.as_deref(),
            Some("2026-07-06T00:12:00Z")
        );
        assert_eq!(stopped_timer.elapsed_seconds, Some(600));
        assert!(database.get_active_timer().expect("active timer").is_none());
        assert!(usecases::get_active_pomodoro(&database)
            .expect("active pomodoro")
            .is_none());

        let break_session = usecases::start_pomodoro_break(&database, &break_clock, completed.id)
            .expect("start break");
        assert_eq!(break_session.phase, PomodoroPhase::ShortBreak);
        assert_eq!(break_session.status, PomodoroStatus::Running);
        assert_eq!(break_session.cycle_count, 1);
        assert_eq!(
            break_session.phase_duration_seconds,
            DEFAULT_POMODORO_SHORT_BREAK_SECONDS
        );
        assert_eq!(break_session.timer_session_id, None);
        assert!(database.get_active_timer().expect("active timer").is_none());
    }

    #[test]
    fn sync_expired_pomodoro_completes_work_at_phase_end_and_notifies() {
        let database = in_memory_database();
        let notification_gateway = RecordingNotificationGateway::ok();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        usecases::update_pomodoro_settings(
            &database,
            &start_clock,
            usecases::PomodoroSettingsDraft {
                work_seconds: 60,
                short_break_seconds: 60,
                long_break_seconds: 120,
                cycles_until_long_break: 4,
                auto_start_break: false,
                auto_start_next_work: false,
            },
        )
        .expect("settings");
        let task =
            usecases::create_task(&database, &start_clock, draft("期限到達作業")).expect("task");
        let pomodoro = usecases::start_legacy_task_linked_pomodoro(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start pomodoro");
        let timer_id = pomodoro.timer_session_id.clone().expect("timer id");

        let result = usecases::sync_expired_pomodoro(
            &database,
            &notification_gateway,
            &FixedClock {
                now: "2026-07-06T00:01:30Z",
            },
        )
        .expect("sync expired pomodoro");
        let expired = result.expired_pomodoro.expect("expired pomodoro");
        let stopped_timer = database
            .with_connection(|connection| select_timer_by_id(connection, &timer_id))
            .expect("stopped timer");
        let messages = notification_gateway.messages();

        assert_eq!(expired.phase, PomodoroPhase::Work);
        assert_eq!(expired.status, PomodoroStatus::Completed);
        assert_eq!(
            expired.completed_at.as_deref(),
            Some("2026-07-06T00:01:00Z")
        );
        assert_eq!(
            stopped_timer.stopped_at.as_deref(),
            Some("2026-07-06T00:01:00Z")
        );
        assert_eq!(stopped_timer.elapsed_seconds, Some(60));
        assert!(result.active_pomodoro.is_none());
        assert!(usecases::get_active_pomodoro(&database)
            .expect("active pomodoro")
            .is_none());
        assert!(database.get_active_timer().expect("active timer").is_none());
        assert_eq!(result.notification_summary.attempted, 1);
        assert_eq!(result.notification_summary.succeeded, 1);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].title, "期限到達作業");
        assert_eq!(messages[0].body, "ポモドーロの作業時間が終了しました。");
    }

    #[test]
    fn sync_expired_pomodoro_does_not_complete_paused_phase() {
        let database = in_memory_database();
        let notification_gateway = RecordingNotificationGateway::ok();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        usecases::update_pomodoro_settings(
            &database,
            &start_clock,
            usecases::PomodoroSettingsDraft {
                work_seconds: 60,
                short_break_seconds: 60,
                long_break_seconds: 120,
                cycles_until_long_break: 4,
                auto_start_break: false,
                auto_start_next_work: false,
            },
        )
        .expect("settings");
        let task =
            usecases::create_task(&database, &start_clock, draft("一時停止中")).expect("task");
        usecases::start_legacy_task_linked_pomodoro(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start pomodoro");
        usecases::pause_pomodoro(
            &database,
            &FixedClock {
                now: "2026-07-06T00:00:30Z",
            },
        )
        .expect("pause pomodoro");

        let result = usecases::sync_expired_pomodoro(
            &database,
            &notification_gateway,
            &FixedClock {
                now: "2026-07-06T00:05:00Z",
            },
        )
        .expect("sync paused pomodoro");
        let active = usecases::get_active_pomodoro(&database)
            .expect("active pomodoro")
            .expect("paused pomodoro");

        assert!(result.expired_pomodoro.is_none());
        assert_eq!(result.notification_summary.attempted, 0);
        assert_eq!(active.status, PomodoroStatus::Paused);
        assert_eq!(active.paused_at.as_deref(), Some("2026-07-06T00:00:30Z"));
        assert!(notification_gateway.messages().is_empty());
    }

    #[test]
    fn sync_expired_pomodoro_auto_starts_break_when_enabled() {
        let database = in_memory_database();
        let notification_gateway = RecordingNotificationGateway::ok();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        usecases::update_pomodoro_settings(
            &database,
            &start_clock,
            usecases::PomodoroSettingsDraft {
                work_seconds: 60,
                short_break_seconds: 60,
                long_break_seconds: 120,
                cycles_until_long_break: 4,
                auto_start_break: true,
                auto_start_next_work: false,
            },
        )
        .expect("settings");
        let task = usecases::create_task(&database, &start_clock, draft("自動休憩")).expect("task");
        usecases::start_legacy_task_linked_pomodoro(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start pomodoro");

        let result = usecases::sync_expired_pomodoro(
            &database,
            &notification_gateway,
            &FixedClock {
                now: "2026-07-06T00:01:30Z",
            },
        )
        .expect("sync expired pomodoro");
        let active = result.active_pomodoro.expect("auto break");

        assert_eq!(active.phase, PomodoroPhase::ShortBreak);
        assert_eq!(active.status, PomodoroStatus::Running);
        assert_eq!(active.phase_started_at, "2026-07-06T00:01:00Z");
        assert_eq!(active.cycle_count, 1);
        assert!(database.get_active_timer().expect("active timer").is_none());
        assert_eq!(result.notification_summary.succeeded, 1);
    }

    #[test]
    fn pomodoro_uses_long_break_after_configured_cycle_count() {
        let database = in_memory_database();
        let settings_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        usecases::update_pomodoro_settings(
            &database,
            &settings_clock,
            usecases::PomodoroSettingsDraft {
                work_seconds: 60,
                short_break_seconds: 60,
                long_break_seconds: 120,
                cycles_until_long_break: 2,
                auto_start_break: false,
                auto_start_next_work: false,
            },
        )
        .expect("settings");
        let task =
            usecases::create_task(&database, &settings_clock, draft("長い休憩")).expect("task");

        usecases::start_legacy_task_linked_pomodoro(
            &database,
            &settings_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start first work");
        let first_completed = usecases::complete_pomodoro_work_phase(
            &database,
            &FixedClock {
                now: "2026-07-06T00:01:00Z",
            },
        )
        .expect("complete first work");
        usecases::skip_pomodoro_break(
            &database,
            &FixedClock {
                now: "2026-07-06T00:01:30Z",
            },
            first_completed.id,
        )
        .expect("skip first break");
        let second_completed = usecases::complete_pomodoro_work_phase(
            &database,
            &FixedClock {
                now: "2026-07-06T00:02:30Z",
            },
        )
        .expect("complete second work");

        let break_session = usecases::start_pomodoro_break(
            &database,
            &FixedClock {
                now: "2026-07-06T00:03:00Z",
            },
            second_completed.id,
        )
        .expect("start long break");

        assert_eq!(break_session.phase, PomodoroPhase::LongBreak);
        assert_eq!(break_session.cycle_count, 2);
        assert_eq!(break_session.phase_duration_seconds, 120);
    }

    #[test]
    fn pomodoro_break_pause_resume_and_complete_tracks_pause_seconds() {
        let database = in_memory_database();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task = usecases::create_task(&database, &start_clock, draft("休憩完了")).expect("task");
        usecases::start_legacy_task_linked_pomodoro(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start work");
        let completed_work = usecases::complete_pomodoro_work_phase(
            &database,
            &FixedClock {
                now: "2026-07-06T00:25:00Z",
            },
        )
        .expect("complete work");
        usecases::start_pomodoro_break(
            &database,
            &FixedClock {
                now: "2026-07-06T00:25:00Z",
            },
            completed_work.id,
        )
        .expect("start break");

        let paused = usecases::pause_pomodoro(
            &database,
            &FixedClock {
                now: "2026-07-06T00:26:00Z",
            },
        )
        .expect("pause break");
        assert_eq!(paused.status, PomodoroStatus::Paused);
        assert_eq!(paused.paused_at.as_deref(), Some("2026-07-06T00:26:00Z"));

        let resumed = usecases::resume_pomodoro(
            &database,
            &FixedClock {
                now: "2026-07-06T00:28:00Z",
            },
        )
        .expect("resume break");
        assert_eq!(resumed.status, PomodoroStatus::Running);
        assert_eq!(resumed.paused_total_seconds, 120);

        let completed_break = usecases::complete_pomodoro_break(
            &database,
            &FixedClock {
                now: "2026-07-06T00:30:00Z",
            },
        )
        .expect("complete break");

        assert_eq!(completed_break.status, PomodoroStatus::Completed);
        assert_eq!(completed_break.phase, PomodoroPhase::ShortBreak);
        assert_eq!(completed_break.paused_total_seconds, 120);
        assert!(usecases::get_active_pomodoro(&database)
            .expect("active pomodoro")
            .is_none());
        assert!(database.get_active_timer().expect("active timer").is_none());
    }

    #[test]
    fn pomodoro_skip_break_cancels_break_and_starts_next_work() {
        let database = in_memory_database();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task =
            usecases::create_task(&database, &start_clock, draft("休憩スキップ")).expect("task");
        usecases::start_legacy_task_linked_pomodoro(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start work");
        let completed_work = usecases::complete_pomodoro_work_phase(
            &database,
            &FixedClock {
                now: "2026-07-06T00:25:00Z",
            },
        )
        .expect("complete work");
        let break_session = usecases::start_pomodoro_break(
            &database,
            &FixedClock {
                now: "2026-07-06T00:25:00Z",
            },
            completed_work.id,
        )
        .expect("start break");

        let next_work = usecases::skip_pomodoro_break(
            &database,
            &FixedClock {
                now: "2026-07-06T00:26:00Z",
            },
            break_session.id.clone(),
        )
        .expect("skip break");
        let skipped_break = database
            .with_connection(|connection| {
                select_pomodoro_session_by_id(connection, &break_session.id)
            })
            .expect("skipped break");
        let active_timer = database
            .get_active_timer()
            .expect("active timer")
            .expect("next work timer");

        assert_eq!(skipped_break.status, PomodoroStatus::Cancelled);
        assert_eq!(
            skipped_break.cancelled_at.as_deref(),
            Some("2026-07-06T00:26:00Z")
        );
        assert_eq!(next_work.phase, PomodoroPhase::Work);
        assert_eq!(next_work.status, PomodoroStatus::Running);
        assert_eq!(next_work.cycle_count, 1);
        assert_eq!(
            next_work.timer_session_id.as_deref(),
            Some(active_timer.id.as_str())
        );
    }

    #[test]
    fn sync_expired_pomodoro_auto_starts_next_work_when_enabled() {
        let database = in_memory_database();
        let notification_gateway = RecordingNotificationGateway::ok();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        usecases::update_pomodoro_settings(
            &database,
            &start_clock,
            usecases::PomodoroSettingsDraft {
                work_seconds: 60,
                short_break_seconds: 60,
                long_break_seconds: 120,
                cycles_until_long_break: 4,
                auto_start_break: false,
                auto_start_next_work: true,
            },
        )
        .expect("settings");
        let task =
            usecases::create_task(&database, &start_clock, draft("自動次作業")).expect("task");
        usecases::start_legacy_task_linked_pomodoro(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start work");
        let completed_work = usecases::complete_pomodoro_work_phase(
            &database,
            &FixedClock {
                now: "2026-07-06T00:01:00Z",
            },
        )
        .expect("complete work");
        usecases::start_pomodoro_break(
            &database,
            &FixedClock {
                now: "2026-07-06T00:01:00Z",
            },
            completed_work.id,
        )
        .expect("start break");

        let result = usecases::sync_expired_pomodoro(
            &database,
            &notification_gateway,
            &FixedClock {
                now: "2026-07-06T00:02:30Z",
            },
        )
        .expect("sync expired break");
        let expired = result.expired_pomodoro.expect("expired break");
        let active = result.active_pomodoro.expect("next work");
        let active_timer = database
            .get_active_timer()
            .expect("active timer")
            .expect("next work timer");

        assert_eq!(expired.phase, PomodoroPhase::ShortBreak);
        assert_eq!(
            expired.completed_at.as_deref(),
            Some("2026-07-06T00:02:00Z")
        );
        assert_eq!(active.phase, PomodoroPhase::Work);
        assert_eq!(active.phase_started_at, "2026-07-06T00:02:00Z");
        assert_eq!(active.cycle_count, 1);
        assert_eq!(active_timer.started_at, "2026-07-06T00:02:00Z");
        assert_eq!(result.notification_summary.succeeded, 1);
    }

    #[test]
    fn sync_expired_pomodoro_completes_break_without_next_work_when_target_done() {
        let database = in_memory_database();
        let notification_gateway = RecordingNotificationGateway::ok();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        usecases::update_pomodoro_settings(
            &database,
            &start_clock,
            usecases::PomodoroSettingsDraft {
                work_seconds: 60,
                short_break_seconds: 60,
                long_break_seconds: 120,
                cycles_until_long_break: 4,
                auto_start_break: false,
                auto_start_next_work: true,
            },
        )
        .expect("settings");
        let task =
            usecases::create_task(&database, &start_clock, draft("完了後の休憩")).expect("task");
        usecases::start_legacy_task_linked_pomodoro(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id.clone(),
            },
        )
        .expect("start work");
        let completed_work = usecases::complete_pomodoro_work_phase(
            &database,
            &FixedClock {
                now: "2026-07-06T00:01:00Z",
            },
        )
        .expect("complete work");
        usecases::start_pomodoro_break(
            &database,
            &FixedClock {
                now: "2026-07-06T00:01:00Z",
            },
            completed_work.id,
        )
        .expect("start break");
        usecases::complete_task(
            &database,
            &FixedClock {
                now: "2026-07-06T00:01:30Z",
            },
            task.id,
            true,
        )
        .expect("complete task");

        let result = usecases::sync_expired_pomodoro(
            &database,
            &notification_gateway,
            &FixedClock {
                now: "2026-07-06T00:02:30Z",
            },
        )
        .expect("sync expired break");

        let expired = result.expired_pomodoro.expect("expired break");
        assert_eq!(expired.phase, PomodoroPhase::ShortBreak);
        assert!(result.active_pomodoro.is_none());
        assert!(usecases::get_active_pomodoro(&database)
            .expect("active pomodoro")
            .is_none());
        assert!(database.get_active_timer().expect("active timer").is_none());
        assert_eq!(result.notification_summary.succeeded, 1);
    }

    #[test]
    fn cancel_pomodoro_work_cancels_session_and_stops_timer() {
        let database = in_memory_database();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let cancel_clock = FixedClock {
            now: "2026-07-06T00:03:00Z",
        };
        let task =
            usecases::create_task(&database, &start_clock, draft("キャンセル")).expect("task");
        let pomodoro = usecases::start_legacy_task_linked_pomodoro(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start pomodoro");
        let timer_id = pomodoro.timer_session_id.clone().expect("work phase timer");

        let cancelled =
            usecases::cancel_pomodoro(&database, &cancel_clock).expect("cancel pomodoro");
        let stopped_timer = database
            .with_connection(|connection| select_timer_by_id(connection, &timer_id))
            .expect("stopped timer");

        assert_eq!(cancelled.status, PomodoroStatus::Cancelled);
        assert_eq!(cancelled.phase, PomodoroPhase::Work);
        assert_eq!(
            cancelled.cancelled_at.as_deref(),
            Some("2026-07-06T00:03:00Z")
        );
        assert_eq!(stopped_timer.elapsed_seconds, Some(180));
        assert!(usecases::get_active_pomodoro(&database)
            .expect("active pomodoro")
            .is_none());
        assert!(database.get_active_timer().expect("active timer").is_none());
    }

    #[test]
    fn start_timer_and_start_pomodoro_are_mutually_exclusive() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let first_task =
            usecases::create_task(&database, &clock, draft("通常タイマー")).expect("first task");

        usecases::start_timer(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: first_task.id.clone(),
            },
        )
        .expect("start normal timer");
        let pomodoro_result = usecases::start_standalone_pomodoro(&database, &clock);
        assert!(pomodoro_result
            .expect_err("active timer blocks pomodoro")
            .contains("開始中"));

        usecases::stop_active_timer(&database, &clock).expect("stop normal timer");
        usecases::start_standalone_pomodoro(&database, &clock).expect("start pomodoro");
        let normal_timer_result = usecases::start_timer(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: first_task.id,
            },
        );
        assert!(normal_timer_result
            .expect_err("active pomodoro blocks timer")
            .contains("開始中"));
    }

    #[test]
    fn stop_active_timer_persists_elapsed_seconds() {
        let database = in_memory_database();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let stop_clock = FixedClock {
            now: "2026-07-06T00:02:03Z",
        };
        let task = usecases::create_task(&database, &start_clock, draft("計測")).expect("task");

        usecases::start_timer(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start timer");

        let stopped =
            usecases::stop_active_timer(&database, &stop_clock).expect("stop active timer");

        assert_eq!(stopped.stopped_at.as_deref(), Some("2026-07-06T00:02:03Z"));
        assert_eq!(stopped.elapsed_seconds, Some(123));
        assert!(database.get_active_timer().expect("active timer").is_none());
    }

    #[test]
    fn active_timer_survives_database_reopen_and_counts_wall_clock_gap() {
        let data_dir = std::env::temp_dir().join(format!("tasktimer-test-{}", Uuid::new_v4()));
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let stop_clock = FixedClock {
            now: "2026-07-06T10:30:00Z",
        };
        let task_id;
        let timer_id;

        {
            let database = SqliteDatabase::open_in_dir(data_dir.clone()).expect("open database");
            let task = usecases::create_task(&database, &start_clock, draft("復帰確認"))
                .expect("create task");
            let active = usecases::start_timer(
                &database,
                &start_clock,
                WorkTargetRef {
                    target_type: WorkTargetType::Task,
                    id: task.id.clone(),
                },
            )
            .expect("start timer");
            task_id = task.id;
            timer_id = active.id;
        }

        let reopened = SqliteDatabase::open_in_dir(data_dir.clone()).expect("reopen database");
        let active = reopened
            .get_active_timer()
            .expect("active timer")
            .expect("persisted active timer");
        assert_eq!(active.id, timer_id);
        assert_eq!(active.target.target_type, WorkTargetType::Task);
        assert_eq!(active.target.id, task_id);
        assert_eq!(active.started_at, "2026-07-06T00:00:00Z");

        let stopped =
            usecases::stop_active_timer(&reopened, &stop_clock).expect("stop active timer");
        assert_eq!(stopped.stopped_at.as_deref(), Some("2026-07-06T10:30:00Z"));
        assert_eq!(stopped.elapsed_seconds, Some(37_800));
        assert!(reopened.get_active_timer().expect("active timer").is_none());

        drop(reopened);
        fs::remove_dir_all(data_dir).expect("cleanup");
    }

    #[test]
    fn paused_timer_survives_database_reopen_and_excludes_wall_clock_gap() {
        let data_dir = std::env::temp_dir().join(format!("tasktimer-test-{}", Uuid::new_v4()));
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let pause_clock = FixedClock {
            now: "2026-07-06T00:02:00Z",
        };
        let stop_clock = FixedClock {
            now: "2026-07-06T10:00:00Z",
        };
        let timer_id;

        {
            let database = SqliteDatabase::open_in_dir(data_dir.clone()).expect("open database");
            let task = usecases::create_task(&database, &start_clock, draft("一時停止復帰確認"))
                .expect("create task");
            let active = usecases::start_timer(
                &database,
                &start_clock,
                WorkTargetRef {
                    target_type: WorkTargetType::Task,
                    id: task.id,
                },
            )
            .expect("start timer");
            usecases::pause_active_timer(&database, &pause_clock).expect("pause timer");
            timer_id = active.id;
        }

        let reopened = SqliteDatabase::open_in_dir(data_dir.clone()).expect("reopen database");
        let active = reopened
            .get_active_timer()
            .expect("active timer")
            .expect("persisted paused timer");
        assert_eq!(active.id, timer_id);
        assert_eq!(active.paused_at.as_deref(), Some("2026-07-06T00:02:00Z"));

        let stopped =
            usecases::stop_active_timer(&reopened, &stop_clock).expect("stop paused timer");
        let resumed_at: String = reopened
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT resumed_at FROM timer_pauses WHERE timer_session_id = ?1",
                        params![stopped.id.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(|error| format!("pause row: {error}"))
            })
            .expect("pause row");

        assert_eq!(stopped.elapsed_seconds, Some(120));
        assert_eq!(stopped.paused_at, None);
        assert_eq!(resumed_at, "2026-07-06T10:00:00Z");
        assert!(reopened.get_active_timer().expect("active timer").is_none());

        drop(reopened);
        fs::remove_dir_all(data_dir).expect("cleanup");
    }

    #[test]
    fn pause_resume_and_stop_active_timer_excludes_paused_seconds() {
        let database = in_memory_database();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let pause_clock = FixedClock {
            now: "2026-07-06T00:02:00Z",
        };
        let resume_clock = FixedClock {
            now: "2026-07-06T00:05:00Z",
        };
        let stop_clock = FixedClock {
            now: "2026-07-06T00:07:00Z",
        };
        let task =
            usecases::create_task(&database, &start_clock, draft("一時停止対象")).expect("task");
        let second_task =
            usecases::create_task(&database, &start_clock, draft("同時開始不可")).expect("task");

        usecases::start_timer(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start timer");

        let paused = usecases::pause_active_timer(&database, &pause_clock).expect("pause timer");
        assert_eq!(paused.paused_at.as_deref(), Some("2026-07-06T00:02:00Z"));
        let active = database.get_active_timer().expect("active timer");
        assert_eq!(
            active.expect("paused active timer").paused_at.as_deref(),
            Some("2026-07-06T00:02:00Z")
        );

        let start_second = usecases::start_timer(
            &database,
            &pause_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: second_task.id,
            },
        );
        assert!(start_second
            .expect_err("paused timer is still active")
            .contains("開始中"));

        let resumed =
            usecases::resume_active_timer(&database, &resume_clock).expect("resume timer");
        assert_eq!(resumed.paused_at, None);

        let stopped =
            usecases::stop_active_timer(&database, &stop_clock).expect("stop active timer");
        let resumed_at: String = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT resumed_at FROM timer_pauses WHERE timer_session_id = ?1",
                        params![stopped.id.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(|error| format!("pause row: {error}"))
            })
            .expect("pause row");

        assert_eq!(stopped.elapsed_seconds, Some(240));
        assert_eq!(resumed_at, "2026-07-06T00:05:00Z");
        assert!(database.get_active_timer().expect("active timer").is_none());
    }

    #[test]
    fn stop_active_timer_closes_open_pause_until_stop_time() {
        let database = in_memory_database();
        let start_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let pause_clock = FixedClock {
            now: "2026-07-06T00:02:00Z",
        };
        let stop_clock = FixedClock {
            now: "2026-07-06T00:05:00Z",
        };
        let task =
            usecases::create_task(&database, &start_clock, draft("停止時クローズ")).expect("task");

        usecases::start_timer(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start timer");
        usecases::pause_active_timer(&database, &pause_clock).expect("pause timer");

        let stopped =
            usecases::stop_active_timer(&database, &stop_clock).expect("stop active timer");
        let resumed_at: String = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT resumed_at FROM timer_pauses WHERE timer_session_id = ?1",
                        params![stopped.id.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(|error| format!("pause row: {error}"))
            })
            .expect("pause row");

        assert_eq!(stopped.elapsed_seconds, Some(120));
        assert_eq!(stopped.paused_at, None);
        assert_eq!(resumed_at, "2026-07-06T00:05:00Z");
    }

    #[test]
    fn list_tasks_with_subtasks_reopens_persisted_data() {
        let data_dir = std::env::temp_dir().join(format!("tasktimer-test-{}", Uuid::new_v4()));
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };

        {
            let database = SqliteDatabase::open_in_dir(data_dir.clone()).expect("open database");
            let task =
                usecases::create_task(&database, &clock, draft("DB接続")).expect("create task");
            usecases::create_subtask(
                &database,
                &clock,
                task.id,
                usecases::WorkItemDraft {
                    list_id: None,
                    title: "画面に表示".to_string(),
                    planned_start_date: Some("2026-07-06".to_string()),
                    due_date: Some("2026-07-07".to_string()),
                    due_time: None,
                    memo: Some("Reactではテキストとして表示する".to_string()),
                },
            )
            .expect("create subtask");
        }

        let reopened = SqliteDatabase::open_in_dir(data_dir.clone()).expect("reopen database");
        let tasks = reopened
            .list_tasks_with_subtasks(200)
            .expect("list task tree");

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task.title, "DB接続");
        assert_eq!(tasks[0].subtasks.len(), 1);
        assert_eq!(tasks[0].subtasks[0].title, "画面に表示");

        drop(reopened);
        fs::remove_dir_all(data_dir).expect("cleanup");
    }

    #[test]
    fn create_sqlite_backup_writes_manifest_and_consistent_database() {
        let data_dir = temp_dir("tasktimer-backup-db");
        let backup_root = temp_dir("tasktimer-backup-root");
        let database = SqliteDatabase::open_in_dir(data_dir.clone()).expect("open database");
        let clock = FixedClock {
            now: "2026-07-15T00:00:00Z",
        };
        let task = usecases::create_task(&database, &clock, draft("バックアップ対象"))
            .expect("create task");

        let backup = usecases::create_sqlite_backup(
            &database,
            &clock,
            usecases::SqliteBackupCreateDraft {
                destination_dir: backup_root.to_string_lossy().to_string(),
            },
        )
        .expect("create backup");

        assert!(PathBuf::from(&backup.database_file).is_file());
        assert!(PathBuf::from(&backup.manifest_file).is_file());
        assert_eq!(backup.manifest.format, BACKUP_FORMAT);
        assert_eq!(backup.manifest.format_version, BACKUP_FORMAT_VERSION);
        assert_eq!(
            backup.manifest.schema_version,
            CURRENT_SQLITE_BACKUP_SCHEMA_VERSION
        );
        assert_eq!(backup.manifest.database_file, BACKUP_DATABASE_FILE);
        assert_eq!(backup.manifest.integrity_check, "ok");

        let copied = Connection::open(PathBuf::from(&backup.database_file)).expect("open backup");
        verify_integrity_check(&copied).expect("backup integrity");
        ensure_required_restore_tables(&copied).expect("backup tables");
        let copied_title: String = copied
            .query_row(
                "SELECT title FROM tasks WHERE id = ?1",
                params![task.id.as_str()],
                |row| row.get(0),
            )
            .expect("backup task");
        assert_eq!(copied_title, "バックアップ対象");

        drop(copied);
        drop(database);
        fs::remove_dir_all(data_dir).expect("cleanup data");
        fs::remove_dir_all(backup_root).expect("cleanup backup");
    }

    #[test]
    fn restore_sqlite_backup_replaces_database_and_keeps_previous_copy() {
        let data_dir = temp_dir("tasktimer-restore-db");
        let backup_root = temp_dir("tasktimer-restore-root");
        let database = SqliteDatabase::open_in_dir(data_dir.clone()).expect("open database");
        let backup_clock = FixedClock {
            now: "2026-07-15T00:00:00Z",
        };
        let restore_clock = FixedClock {
            now: "2026-07-15T00:01:00Z",
        };
        usecases::create_task(&database, &backup_clock, draft("復元で残る"))
            .expect("create original task");
        let backup = usecases::create_sqlite_backup(
            &database,
            &backup_clock,
            usecases::SqliteBackupCreateDraft {
                destination_dir: backup_root.to_string_lossy().to_string(),
            },
        )
        .expect("create backup");
        usecases::create_task(&database, &backup_clock, draft("復元で消える"))
            .expect("create later task");

        let restored = usecases::restore_sqlite_backup(
            &database,
            &restore_clock,
            usecases::SqliteBackupRestoreDraft {
                backup_dir: backup.backup_dir.clone(),
            },
        )
        .expect("restore backup");

        assert!(PathBuf::from(&restored.previous_database_file).is_file());
        let tasks = database
            .list_tasks_with_subtasks(200)
            .expect("list restored tasks");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task.title, "復元で残る");

        drop(database);
        fs::remove_dir_all(data_dir).expect("cleanup data");
        fs::remove_dir_all(backup_root).expect("cleanup backup");
    }

    #[test]
    fn restore_sqlite_backup_rejects_corrupted_database_and_keeps_current_database() {
        let data_dir = temp_dir("tasktimer-corrupt-db");
        let backup_root = temp_dir("tasktimer-corrupt-root");
        let backup_dir = backup_root.join("TaskTimer-backup-corrupt");
        fs::create_dir_all(&backup_dir).expect("backup dir");
        let database = SqliteDatabase::open_in_dir(data_dir.clone()).expect("open database");
        let clock = FixedClock {
            now: "2026-07-15T00:00:00Z",
        };
        usecases::create_task(&database, &clock, draft("既存DB")).expect("create current task");

        fs::write(
            backup_dir.join(BACKUP_MANIFEST_FILE),
            serde_json::json!({
                "format": BACKUP_FORMAT,
                "formatVersion": BACKUP_FORMAT_VERSION,
                "appVersion": env!("CARGO_PKG_VERSION"),
                "schemaVersion": CURRENT_SQLITE_BACKUP_SCHEMA_VERSION,
                "createdAt": "2026-07-15T00:00:00Z",
                "platform": "test",
                "databaseFile": BACKUP_DATABASE_FILE,
                "integrityCheck": "ok"
            })
            .to_string(),
        )
        .expect("write manifest");
        fs::write(backup_dir.join(BACKUP_DATABASE_FILE), b"not sqlite").expect("write corrupt db");

        let result = usecases::restore_sqlite_backup(
            &database,
            &clock,
            usecases::SqliteBackupRestoreDraft {
                backup_dir: backup_dir.to_string_lossy().to_string(),
            },
        );

        assert!(result.expect_err("corrupt backup").contains("SQLite"));
        let tasks = database
            .list_tasks_with_subtasks(200)
            .expect("list current tasks");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task.title, "既存DB");

        drop(database);
        fs::remove_dir_all(data_dir).expect("cleanup data");
        fs::remove_dir_all(backup_root).expect("cleanup backup");
    }

    #[test]
    fn restore_sqlite_backup_rejects_future_schema_version() {
        let data_dir = temp_dir("tasktimer-future-schema-db");
        let backup_root = temp_dir("tasktimer-future-schema-root");
        let database = SqliteDatabase::open_in_dir(data_dir.clone()).expect("open database");
        let backup_clock = FixedClock {
            now: "2026-07-15T00:00:00Z",
        };
        usecases::create_task(&database, &backup_clock, draft("既存タスク"))
            .expect("create original task");
        let backup = usecases::create_sqlite_backup(
            &database,
            &backup_clock,
            usecases::SqliteBackupCreateDraft {
                destination_dir: backup_root.to_string_lossy().to_string(),
            },
        )
        .expect("create backup");
        let manifest_path = PathBuf::from(&backup.manifest_file);
        let mut manifest: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&manifest_path).expect("read manifest"))
                .expect("manifest json");
        manifest["schemaVersion"] = serde_json::json!(CURRENT_SQLITE_BACKUP_SCHEMA_VERSION + 1);
        fs::write(&manifest_path, manifest.to_string()).expect("write future manifest");
        usecases::create_task(&database, &backup_clock, draft("現在だけのタスク"))
            .expect("create current task");

        let result = usecases::restore_sqlite_backup(
            &database,
            &backup_clock,
            usecases::SqliteBackupRestoreDraft {
                backup_dir: backup.backup_dir,
            },
        );

        assert!(result
            .expect_err("future schema")
            .contains("新しいTaskTimer"));
        let tasks = database
            .list_tasks_with_subtasks(200)
            .expect("list current tasks");
        assert_eq!(tasks.len(), 2);

        drop(database);
        fs::remove_dir_all(data_dir).expect("cleanup data");
        fs::remove_dir_all(backup_root).expect("cleanup backup");
    }

    #[test]
    fn create_sqlite_backup_uses_committed_snapshot_during_external_write() {
        let data_dir = temp_dir("tasktimer-write-backup-db");
        let backup_root = temp_dir("tasktimer-write-backup-root");
        let database = SqliteDatabase::open_in_dir(data_dir.clone()).expect("open database");
        let clock = FixedClock {
            now: "2026-07-15T00:00:00Z",
        };
        usecases::create_task(&database, &clock, draft("コミット済み"))
            .expect("create committed task");

        let external = Connection::open(database.path()).expect("external connection");
        configure_connection(&external).expect("configure external");
        external
            .execute_batch("BEGIN IMMEDIATE")
            .expect("begin external write");
        external
            .execute(
                "
                INSERT INTO tasks (
                  id, list_id, title, status, memo, sort_order, created_at, updated_at
                )
                VALUES ('uncommitted-task', ?1, '未コミット', 'todo', '', 0, ?2, ?2)
                ",
                params![DEFAULT_TASK_LIST_ID, clock.now],
            )
            .expect("insert uncommitted task");

        let backup = usecases::create_sqlite_backup(
            &database,
            &clock,
            usecases::SqliteBackupCreateDraft {
                destination_dir: backup_root.to_string_lossy().to_string(),
            },
        )
        .expect("create backup during write");
        external
            .execute_batch("ROLLBACK")
            .expect("rollback external write");

        let copied = Connection::open(PathBuf::from(&backup.database_file)).expect("open backup");
        let titles: Vec<String> = copied
            .prepare("SELECT title FROM tasks ORDER BY title")
            .expect("prepare titles")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query titles")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect titles");
        assert_eq!(titles, vec!["コミット済み".to_string()]);

        drop(copied);
        drop(external);
        drop(database);
        fs::remove_dir_all(data_dir).expect("cleanup data");
        fs::remove_dir_all(backup_root).expect("cleanup backup");
    }

    #[test]
    fn create_json_export_writes_manifest_and_exact_user_values() {
        let data_dir = temp_dir("tasktimer-json-export-db");
        let export_root = temp_dir("tasktimer-json-export-root");
        let database = SqliteDatabase::open_in_dir(data_dir.clone()).expect("open database");
        let clock = FixedClock {
            now: "2026-07-15T00:00:00Z",
        };
        let task = usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "=SUM(1,2)".to_string(),
                planned_start_date: Some("2026-07-15".to_string()),
                due_date: Some("2026-07-16".to_string()),
                due_time: Some("09:30".to_string()),
                memo: Some("1行目,2行目\n\"引用\"".to_string()),
            },
        )
        .expect("create task");
        let task = usecases::update_task(
            &database,
            &clock,
            task.id.clone(),
            usecases::WorkItemUpdateDraft {
                list_id: Some(task.list_id.clone()),
                title: task.title.clone(),
                planned_start_date: task.planned_start_date.clone(),
                due_date: task.due_date.clone(),
                due_time: task.due_time.clone(),
                timer_target_seconds: task.timer_target_seconds,
                color_token: Some("blue".to_string()),
                recurrence_rule: None,
                memo: Some(task.memo.clone()),
            },
        )
        .expect("set task export color");
        let tag = usecases::create_tag(
            &database,
            &clock,
            usecases::TagDraft {
                name: "移行確認".to_string(),
            },
        )
        .expect("create tag");
        usecases::attach_tag_to_task(&database, &clock, task.id.clone(), tag.id.clone())
            .expect("attach tag");
        usecases::create_subtask(
            &database,
            &clock,
            task.id.clone(),
            usecases::WorkItemDraft {
                list_id: None,
                title: "サブタスク".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-16".to_string()),
                due_time: None,
                memo: Some("JSONでは値を変えない".to_string()),
            },
        )
        .expect("create subtask");
        usecases::start_standalone_pomodoro(&database, &clock).expect("start standalone pomodoro");

        let export = usecases::create_json_export(
            &database,
            &clock,
            usecases::DataExportCreateDraft {
                destination_dir: export_root.to_string_lossy().to_string(),
            },
        )
        .expect("create json export");

        assert!(export
            .export_path
            .ends_with("TaskTimer-export-20260715-000000.json"));
        assert_eq!(export.manifest.format, JSON_EXPORT_FORMAT);
        assert_eq!(export.manifest.format_version, DATA_EXPORT_FORMAT_VERSION);
        assert_eq!(export.manifest.compatibility, DATA_EXPORT_COMPATIBILITY);
        assert!(export.manifest.contains_personal_data);

        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&export.export_path).expect("read json"))
                .expect("parse json");
        assert_eq!(json["manifest"]["format"], JSON_EXPORT_FORMAT);
        assert_eq!(json["manifest"]["containsPersonalData"], true);
        assert_eq!(json["tasks"][0]["title"], "=SUM(1,2)");
        assert_eq!(json["tasks"][0]["board_column_id"], DEFAULT_BOARD_COLUMN_ID);
        assert_eq!(json["tasks"][0]["lifecycle_status"], "active");
        assert_eq!(json["tasks"][0]["color_token"], "blue");
        assert_eq!(
            json["board_columns"]
                .as_array()
                .expect("board columns")
                .len(),
            2
        );
        assert_eq!(json["tasks"][0]["memo"], "1行目,2行目\n\"引用\"");
        assert_eq!(json["tags"][0]["name"], "移行確認");
        assert_eq!(json["task_tags"][0]["task_id"], task.id);
        assert_eq!(json["task_tags"][0]["tag_id"], tag.id);
        assert_eq!(json["subtasks"][0]["title"], "サブタスク");
        assert_eq!(
            json["pomodoro_settings"][0]["work_seconds"],
            DEFAULT_POMODORO_WORK_SECONDS
        );
        assert_eq!(
            json["pomodoro_sessions"]
                .as_array()
                .expect("sessions")
                .len(),
            1
        );
        assert_eq!(json["pomodoro_sessions"][0]["scope"], "standalone");
        assert_eq!(
            json["pomodoro_sessions"][0]["target_type"],
            serde_json::Value::Null
        );
        assert_eq!(
            json["pomodoro_sessions"][0]["target_id"],
            serde_json::Value::Null
        );
        assert_eq!(
            json["pomodoro_sessions"][0]["timer_session_id"],
            serde_json::Value::Null
        );

        drop(database);
        fs::remove_dir_all(data_dir).expect("cleanup data");
        fs::remove_dir_all(export_root).expect("cleanup export");
    }

    #[test]
    fn create_csv_export_escapes_memo_and_neutralizes_formula_cells() {
        let data_dir = temp_dir("tasktimer-csv-export-db");
        let export_root = temp_dir("tasktimer-csv-export-root");
        let database = SqliteDatabase::open_in_dir(data_dir.clone()).expect("open database");
        let start_clock = FixedClock {
            now: "2026-07-15T00:00:00Z",
        };
        let stop_clock = FixedClock {
            now: "2026-07-15T00:10:00Z",
        };
        let task = usecases::create_task(
            &database,
            &start_clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "=SUM(1,2)".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-15".to_string()),
                due_time: Some("10:00".to_string()),
                memo: Some("カンマ, 改行\n\"引用符\"".to_string()),
            },
        )
        .expect("create task");
        let task = usecases::update_task(
            &database,
            &start_clock,
            task.id.clone(),
            usecases::WorkItemUpdateDraft {
                list_id: Some(task.list_id.clone()),
                title: task.title.clone(),
                planned_start_date: task.planned_start_date.clone(),
                due_date: task.due_date.clone(),
                due_time: task.due_time.clone(),
                timer_target_seconds: task.timer_target_seconds,
                color_token: Some("rose".to_string()),
                recurrence_rule: None,
                memo: Some(task.memo.clone()),
            },
        )
        .expect("set task csv color");
        let tag = usecases::create_tag(
            &database,
            &start_clock,
            usecases::TagDraft {
                name: "+tag".to_string(),
            },
        )
        .expect("create tag");
        usecases::attach_tag_to_task(&database, &start_clock, task.id.clone(), tag.id.clone())
            .expect("attach tag");
        usecases::start_timer(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id.clone(),
            },
        )
        .expect("start timer");
        usecases::stop_active_timer(&database, &stop_clock).expect("stop timer");
        usecases::start_standalone_pomodoro(&database, &start_clock)
            .expect("start standalone pomodoro");

        let export = usecases::create_csv_export(
            &database,
            &start_clock,
            usecases::DataExportCreateDraft {
                destination_dir: export_root.to_string_lossy().to_string(),
            },
        )
        .expect("create csv export");

        assert!(export
            .export_path
            .ends_with("TaskTimer-export-20260715-000000-csv"));
        assert_eq!(export.manifest.format, CSV_EXPORT_FORMAT);
        assert!(PathBuf::from(&export.export_path)
            .join(CSV_EXPORT_MANIFEST_FILE)
            .is_file());
        assert!(export.files.iter().any(|path| path.ends_with("tasks.csv")));
        assert!(export
            .files
            .iter()
            .any(|path| path.ends_with("board_columns.csv")));
        assert!(export.files.iter().any(|path| path.ends_with("tags.csv")));
        assert!(export
            .files
            .iter()
            .any(|path| path.ends_with("task_tags.csv")));
        assert!(export
            .files
            .iter()
            .any(|path| path.ends_with("timer_sessions.csv")));
        assert!(export
            .files
            .iter()
            .any(|path| path.ends_with("pomodoro_settings.csv")));
        assert!(export
            .files
            .iter()
            .any(|path| path.ends_with("pomodoro_sessions.csv")));

        let tasks_csv = fs::read_to_string(PathBuf::from(&export.export_path).join("tasks.csv"))
            .expect("read tasks csv");
        let pomodoro_settings_csv =
            fs::read_to_string(PathBuf::from(&export.export_path).join("pomodoro_settings.csv"))
                .expect("read pomodoro settings csv");
        let tags_csv = fs::read_to_string(PathBuf::from(&export.export_path).join("tags.csv"))
            .expect("read tags csv");
        let pomodoro_sessions_csv =
            fs::read_to_string(PathBuf::from(&export.export_path).join("pomodoro_sessions.csv"))
                .expect("read pomodoro sessions csv");
        let task_tags_csv =
            fs::read_to_string(PathBuf::from(&export.export_path).join("task_tags.csv"))
                .expect("read task tags csv");
        assert!(tasks_csv.starts_with(
            "id,list_id,board_column_id,title,status,lifecycle_status,is_favorite,color_token"
        ));
        assert!(tasks_csv.contains(",rose,"));
        assert!(tasks_csv.contains(IN_PROGRESS_BOARD_COLUMN_ID));
        assert!(tasks_csv.contains("\"'=SUM(1,2)\""));
        assert!(tasks_csv.contains("\"カンマ, 改行\n\"\"引用符\"\"\""));
        assert!(pomodoro_settings_csv.starts_with("id,work_seconds,short_break_seconds"));
        assert!(pomodoro_sessions_csv.starts_with("id,scope,target_type,target_id"));
        assert!(pomodoro_sessions_csv.contains(",standalone,,,"));
        assert!(tags_csv.contains("'+tag"));
        assert!(task_tags_csv.contains(&task.id));
        assert!(task_tags_csv.contains(&tag.id));

        drop(database);
        fs::remove_dir_all(data_dir).expect("cleanup data");
        fs::remove_dir_all(export_root).expect("cleanup export");
    }

    #[test]
    fn task_tags_are_visible_and_tag_delete_keeps_task_and_timer_history() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-16T00:00:00Z",
        };
        let stop_clock = FixedClock {
            now: "2026-07-16T00:15:00Z",
        };
        let task =
            usecases::create_task(&database, &clock, draft("タグ対象")).expect("create task");
        let tag = usecases::create_tag(
            &database,
            &clock,
            usecases::TagDraft {
                name: "案件A".to_string(),
            },
        )
        .expect("create tag");

        usecases::attach_tag_to_task(&database, &clock, task.id.clone(), tag.id.clone())
            .expect("attach tag");
        usecases::start_timer(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id.clone(),
            },
        )
        .expect("start timer");
        usecases::stop_active_timer(&database, &stop_clock).expect("stop timer");

        let rows = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("task rows");
        assert_eq!(rows[0].tags[0].name, "案件A");
        let tasks = database
            .list_tasks_with_subtasks(200)
            .expect("tasks with subtasks");
        assert_eq!(tasks[0].task.tags[0].id, tag.id);
        assert_eq!(database.list_tags().expect("list tags")[0].task_count, 1);

        usecases::delete_tag(&database, &clock, tag.id.clone()).expect("delete tag");

        let rows_after_delete = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("task rows after tag delete");
        assert_eq!(rows_after_delete.len(), 1);
        assert!(rows_after_delete[0].tags.is_empty());
        assert!(database
            .list_tags()
            .expect("list tags after delete")
            .is_empty());

        let timer_count: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "
                        SELECT COUNT(*)
                        FROM timer_sessions
                        WHERE target_type = 'task'
                          AND target_id = ?1
                          AND deleted_at IS NULL
                        ",
                        params![task.id.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(|error| format!("timer count: {error}"))
            })
            .expect("timer count");
        assert_eq!(timer_count, 1);
    }

    #[test]
    fn create_tag_rejects_duplicate_name_case_insensitively() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-16T00:00:00Z",
        };

        usecases::create_tag(
            &database,
            &clock,
            usecases::TagDraft {
                name: "Project".to_string(),
            },
        )
        .expect("create tag");

        let result = usecases::create_tag(
            &database,
            &clock,
            usecases::TagDraft {
                name: "project".to_string(),
            },
        );

        assert!(result.expect_err("duplicate tag").contains("同じ名前"));
    }

    #[test]
    fn custom_task_list_filters_task_rows_and_keeps_default_list_separate() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let list = usecases::create_task_list(
            &database,
            &clock,
            usecases::TaskListDraft {
                name: "仕事".to_string(),
                color_token: Some("blue".to_string()),
            },
        )
        .expect("create task list");
        assert_eq!(list.color_token, "blue");
        let custom_task = usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: Some(list.id.clone()),
                title: "リスト所属タスク".to_string(),
                planned_start_date: None,
                due_date: None,
                due_time: None,
                memo: None,
            },
        )
        .expect("create task in custom list");
        usecases::create_task(&database, &clock, draft("既定リストタスク"))
            .expect("create default task");

        let custom_rows = database
            .list_task_rows(Some(&list.id), 200)
            .expect("custom rows");
        let default_rows = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("default rows");
        let all_rows = database.list_task_rows(None, 200).expect("all rows");

        assert_eq!(custom_rows.len(), 1);
        assert_eq!(custom_rows[0].id, custom_task.id);
        assert_eq!(default_rows.len(), 1);
        assert_eq!(all_rows.len(), 2);
    }

    #[test]
    fn task_page_cursor_is_stable_across_ties_and_completion_bucket() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-18T00:00:00Z",
        };
        let mut task_ids = Vec::new();
        for title in ["A", "B", "C", "D", "E"] {
            task_ids.push(
                usecases::create_task(&database, &clock, draft(title))
                    .expect("create paged task")
                    .id,
            );
        }
        database
            .with_connection(|connection| {
                connection
                    .execute(
                        "UPDATE tasks SET sort_order = 0, created_at = '2026-07-18T00:00:00Z'",
                        [],
                    )
                    .map_err(|error| format!("normalize task order: {error}"))?;
                Ok(())
            })
            .expect("normalize task order");
        usecases::complete_task(&database, &clock, task_ids[2].clone(), true)
            .expect("complete task");

        let mut cursor = None;
        let mut loaded_ids = Vec::new();
        loop {
            let page = usecases::list_task_page(
                &database,
                usecases::TaskPageDraft {
                    scope: usecases::TaskPageScopeDraft::Board,
                    today_date: "2026-07-18".to_string(),
                    cursor,
                    limit: 2,
                },
            )
            .expect("list task page");
            assert_eq!(page.total_count, 5);
            assert_eq!(page.tasks.len(), page.rows.len());
            assert_eq!(
                page.tasks
                    .iter()
                    .map(|task| task.task.id.as_str())
                    .collect::<Vec<_>>(),
                page.rows
                    .iter()
                    .map(|row| row.id.as_str())
                    .collect::<Vec<_>>()
            );
            loaded_ids.extend(page.rows.iter().map(|row| row.id.clone()));
            cursor = page
                .next_cursor
                .map(|cursor| usecases::TaskPageCursorDraft {
                    completion_bucket: cursor.completion_bucket,
                    sort_order: cursor.sort_order,
                    created_at: cursor.created_at,
                    id: cursor.id,
                });
            if cursor.is_none() {
                break;
            }
        }

        assert_eq!(loaded_ids.len(), 5);
        assert_eq!(loaded_ids.iter().collect::<HashSet<_>>().len(), 5);
        assert_eq!(loaded_ids.last(), Some(&task_ids[2]));
        let mut expected_active_ids = task_ids
            .into_iter()
            .filter(|id| id != loaded_ids.last().expect("completed task id"))
            .collect::<Vec<_>>();
        expected_active_ids.sort();
        assert_eq!(&loaded_ids[..4], expected_active_ids.as_slice());
    }

    #[test]
    fn task_page_filters_scopes_and_validates_application_input() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-18T00:00:00Z",
        };
        let due_task = usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "今日が期限".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-18".to_string()),
                due_time: None,
                memo: None,
            },
        )
        .expect("create due task");
        let subtask_due_task = usecases::create_task(&database, &clock, draft("サブタスクが期限"))
            .expect("create subtask parent");
        usecases::create_subtask(
            &database,
            &clock,
            subtask_due_task.id.clone(),
            usecases::WorkItemDraft {
                list_id: None,
                title: "今日のサブタスク".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-18".to_string()),
                due_time: None,
                memo: None,
            },
        )
        .expect("create due subtask");
        let planned_task = usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "今日から開始".to_string(),
                planned_start_date: Some("2026-07-18".to_string()),
                due_date: None,
                due_time: None,
                memo: None,
            },
        )
        .expect("create planned task");
        let subtask_planned_task =
            usecases::create_task(&database, &clock, draft("サブタスクが開始"))
                .expect("create planned subtask parent");
        usecases::create_subtask(
            &database,
            &clock,
            subtask_planned_task.id.clone(),
            usecases::WorkItemDraft {
                list_id: None,
                title: "今日開始のサブタスク".to_string(),
                planned_start_date: Some("2026-07-18".to_string()),
                due_date: None,
                due_time: None,
                memo: None,
            },
        )
        .expect("create planned subtask");
        let favorite_task = usecases::create_task(&database, &clock, draft("お気に入り"))
            .expect("create favorite task");
        usecases::toggle_task_favorite(&database, &clock, favorite_task.id.clone(), true)
            .expect("favorite task");
        let tagged_task = usecases::create_task(&database, &clock, draft("タグ対象"))
            .expect("create tagged task");
        let tag = usecases::create_tag(
            &database,
            &clock,
            usecases::TagDraft {
                name: "ページ対象".to_string(),
            },
        )
        .expect("create tag");
        usecases::attach_tag_to_task(&database, &clock, tagged_task.id.clone(), tag.id.clone())
            .expect("attach tag");
        let custom_list = usecases::create_task_list(
            &database,
            &clock,
            usecases::TaskListDraft {
                name: "ページ対象リスト".to_string(),
                color_token: Some("blue".to_string()),
            },
        )
        .expect("create custom list");
        let custom_list_task = usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: Some(custom_list.id.clone()),
                title: "リスト対象".to_string(),
                planned_start_date: None,
                due_date: None,
                due_time: None,
                memo: None,
            },
        )
        .expect("create custom list task");

        let list_page = usecases::list_task_page(
            &database,
            usecases::TaskPageDraft {
                scope: usecases::TaskPageScopeDraft::List {
                    list_id: custom_list.id,
                },
                today_date: "2026-07-18".to_string(),
                cursor: None,
                limit: 200,
            },
        )
        .expect("list scope page");
        assert_eq!(list_page.total_count, 1);
        assert_eq!(list_page.rows[0].id, custom_list_task.id);

        let today_page = usecases::list_task_page(
            &database,
            usecases::TaskPageDraft {
                scope: usecases::TaskPageScopeDraft::Today,
                today_date: "2026-07-18".to_string(),
                cursor: None,
                limit: 200,
            },
        )
        .expect("today page");
        assert_eq!(today_page.total_count, 4);
        assert_eq!(today_page.navigation_counts.today_count, 4);
        assert_eq!(
            today_page
                .rows
                .iter()
                .map(|row| row.id.as_str())
                .collect::<HashSet<_>>(),
            HashSet::from([
                due_task.id.as_str(),
                subtask_due_task.id.as_str(),
                planned_task.id.as_str(),
                subtask_planned_task.id.as_str(),
            ])
        );

        let favorite_page = usecases::list_task_page(
            &database,
            usecases::TaskPageDraft {
                scope: usecases::TaskPageScopeDraft::Favorites,
                today_date: "2026-07-18".to_string(),
                cursor: None,
                limit: 200,
            },
        )
        .expect("favorite page");
        assert_eq!(favorite_page.total_count, 1);
        assert_eq!(favorite_page.rows[0].id, favorite_task.id);
        assert_eq!(favorite_page.navigation_counts.favorite_count, 1);

        let tag_page = usecases::list_task_page(
            &database,
            usecases::TaskPageDraft {
                scope: usecases::TaskPageScopeDraft::Tag { tag_id: tag.id },
                today_date: "2026-07-18".to_string(),
                cursor: None,
                limit: 200,
            },
        )
        .expect("tag page");
        assert_eq!(tag_page.total_count, 1);
        assert_eq!(tag_page.rows[0].id, tagged_task.id);

        let invalid_limit = usecases::list_task_page(
            &database,
            usecases::TaskPageDraft {
                scope: usecases::TaskPageScopeDraft::Board,
                today_date: "2026-07-18".to_string(),
                cursor: None,
                limit: 201,
            },
        );
        assert!(invalid_limit
            .expect_err("invalid page limit")
            .contains("200以下"));

        let invalid_cursor = usecases::list_task_page(
            &database,
            usecases::TaskPageDraft {
                scope: usecases::TaskPageScopeDraft::Board,
                today_date: "2026-07-18".to_string(),
                cursor: Some(usecases::TaskPageCursorDraft {
                    completion_bucket: 2,
                    sort_order: 0,
                    created_at: "invalid".to_string(),
                    id: "task".to_string(),
                }),
                limit: 200,
            },
        );
        assert!(invalid_cursor
            .expect_err("invalid page cursor")
            .contains("完了区分"));

        let invalid_cursor_date = usecases::list_task_page(
            &database,
            usecases::TaskPageDraft {
                scope: usecases::TaskPageScopeDraft::Board,
                today_date: "2026-07-18".to_string(),
                cursor: Some(usecases::TaskPageCursorDraft {
                    completion_bucket: 0,
                    sort_order: 0,
                    created_at: "invalid".to_string(),
                    id: "task".to_string(),
                }),
                limit: 200,
            },
        );
        assert!(invalid_cursor_date
            .expect_err("invalid cursor date")
            .contains("RFC 3339"));
    }

    #[test]
    fn delete_custom_task_list_moves_tasks_to_default_without_deleting_timer_history() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let stop_clock = FixedClock {
            now: "2026-07-06T00:05:00Z",
        };
        let list = usecases::create_task_list(
            &database,
            &clock,
            usecases::TaskListDraft {
                name: "案件A".to_string(),
                color_token: None,
            },
        )
        .expect("create task list");
        let task = usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: Some(list.id.clone()),
                title: "移動対象".to_string(),
                planned_start_date: None,
                due_date: None,
                due_time: None,
                memo: None,
            },
        )
        .expect("create custom task");
        usecases::start_timer(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id.clone(),
            },
        )
        .expect("start timer");
        usecases::stop_active_timer(&database, &stop_clock).expect("stop timer");

        usecases::delete_task_list(&database, &clock, list.id.clone()).expect("delete list");

        let default_rows = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("default rows");
        let lists = database.list_task_lists().expect("task lists");
        let timer_count: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "
                        SELECT COUNT(*)
                        FROM timer_sessions
                        WHERE target_type = 'task'
                          AND target_id = ?1
                          AND deleted_at IS NULL
                        ",
                        params![task.id.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(|error| format!("timer count: {error}"))
            })
            .expect("timer count");

        assert!(default_rows.iter().any(|row| row.id == task.id));
        assert!(!lists.iter().any(|item| item.id == list.id));
        assert_eq!(timer_count, 1);
    }

    #[test]
    fn task_list_name_policy_rejects_default_update_and_duplicate_names() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let list = usecases::create_task_list(
            &database,
            &clock,
            usecases::TaskListDraft {
                name: "重複確認".to_string(),
                color_token: Some("rose".to_string()),
            },
        )
        .expect("create task list");

        let default_update = usecases::update_task_list(
            &database,
            &clock,
            DEFAULT_TASK_LIST_ID.to_string(),
            usecases::TaskListDraft {
                name: "変更不可".to_string(),
                color_token: None,
            },
        );
        let default_color_update = usecases::update_task_list(
            &database,
            &clock,
            DEFAULT_TASK_LIST_ID.to_string(),
            usecases::TaskListDraft {
                name: DEFAULT_TASK_LIST_NAME.to_string(),
                color_token: Some("gray".to_string()),
            },
        )
        .expect("update default list color");
        let duplicate = usecases::create_task_list(
            &database,
            &clock,
            usecases::TaskListDraft {
                name: "重複確認".to_string(),
                color_token: None,
            },
        );
        let invalid_color = usecases::create_task_list(
            &database,
            &clock,
            usecases::TaskListDraft {
                name: "不正色".to_string(),
                color_token: Some("javascript:alert(1)".to_string()),
            },
        );
        let renamed = usecases::update_task_list(
            &database,
            &clock,
            list.id,
            usecases::TaskListDraft {
                name: "重複確認 2".to_string(),
                color_token: Some("violet".to_string()),
            },
        )
        .expect("rename task list");

        assert!(default_update
            .expect_err("default list update")
            .contains("初期リスト名"));
        assert!(duplicate
            .expect_err("duplicate list name")
            .contains("すでに存在"));
        assert!(invalid_color
            .expect_err("invalid color")
            .contains("許可済み"));
        assert_eq!(default_color_update.color_token, "gray");
        assert_eq!(renamed.name, "重複確認 2");
        assert_eq!(renamed.color_token, "violet");
    }

    #[test]
    fn task_color_validates_persists_and_survives_list_move() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let list = usecases::create_task_list(
            &database,
            &clock,
            usecases::TaskListDraft {
                name: "色分離".to_string(),
                color_token: Some("violet".to_string()),
            },
        )
        .expect("create colored list");
        let task = usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: Some(list.id),
                title: "個別色タスク".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-20".to_string()),
                due_time: None,
                memo: None,
            },
        )
        .expect("create task");
        assert_eq!(task.color_token, None);

        let colored = usecases::update_task(
            &database,
            &clock,
            task.id.clone(),
            usecases::WorkItemUpdateDraft {
                list_id: Some(DEFAULT_TASK_LIST_ID.to_string()),
                title: task.title.clone(),
                planned_start_date: task.planned_start_date.clone(),
                due_date: task.due_date.clone(),
                due_time: task.due_time.clone(),
                timer_target_seconds: task.timer_target_seconds,
                color_token: Some("blue".to_string()),
                recurrence_rule: None,
                memo: Some(task.memo.clone()),
            },
        )
        .expect("set task color and move list");
        assert_eq!(colored.list_id, DEFAULT_TASK_LIST_ID);
        assert_eq!(colored.color_token.as_deref(), Some("blue"));

        let invalid = usecases::update_task(
            &database,
            &clock,
            task.id.clone(),
            usecases::WorkItemUpdateDraft {
                list_id: Some(colored.list_id.clone()),
                title: colored.title.clone(),
                planned_start_date: colored.planned_start_date.clone(),
                due_date: colored.due_date.clone(),
                due_time: colored.due_time.clone(),
                timer_target_seconds: colored.timer_target_seconds,
                color_token: Some("linear-gradient(red, blue)".to_string()),
                recurrence_rule: None,
                memo: Some(colored.memo.clone()),
            },
        );
        assert!(invalid
            .expect_err("invalid task color")
            .contains("許可済み"));
        assert_eq!(
            select_existing_task_by_id(&database.connection.lock().expect("connection"), &task.id,)
                .expect("persisted task")
                .color_token
                .as_deref(),
            Some("blue")
        );

        let inherited = usecases::update_task(
            &database,
            &clock,
            task.id,
            usecases::WorkItemUpdateDraft {
                list_id: Some(colored.list_id),
                title: colored.title,
                planned_start_date: colored.planned_start_date,
                due_date: colored.due_date,
                due_time: colored.due_time,
                timer_target_seconds: colored.timer_target_seconds,
                color_token: None,
                recurrence_rule: None,
                memo: Some(colored.memo),
            },
        )
        .expect("inherit list color");
        assert_eq!(inherited.color_token, None);
    }

    #[test]
    fn list_calendar_items_returns_range_parent_title_and_active_timer_time() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T09:00:00Z",
        };
        let timer_clock = FixedClock {
            now: "2026-07-20T10:15:00Z",
        };
        let list = usecases::create_task_list(
            &database,
            &create_clock,
            usecases::TaskListDraft {
                name: "色付きリスト".to_string(),
                color_token: Some("violet".to_string()),
            },
        )
        .expect("create colored task list");
        let parent = usecases::create_task(
            &database,
            &create_clock,
            usecases::WorkItemDraft {
                list_id: Some(list.id.clone()),
                title: "親タスク".to_string(),
                planned_start_date: None,
                due_date: None,
                due_time: None,
                memo: None,
            },
        )
        .expect("task");
        let parent = usecases::update_task(
            &database,
            &create_clock,
            parent.id.clone(),
            usecases::WorkItemUpdateDraft {
                list_id: Some(parent.list_id.clone()),
                title: parent.title.clone(),
                planned_start_date: parent.planned_start_date.clone(),
                due_date: parent.due_date.clone(),
                due_time: parent.due_time.clone(),
                timer_target_seconds: parent.timer_target_seconds,
                color_token: Some("blue".to_string()),
                recurrence_rule: None,
                memo: Some(parent.memo.clone()),
            },
        )
        .expect("set parent task color");
        let subtask = usecases::create_subtask(
            &database,
            &create_clock,
            parent.id,
            usecases::WorkItemDraft {
                list_id: None,
                title: "調査サブタスク".to_string(),
                planned_start_date: Some("2026-07-20".to_string()),
                due_date: Some("2026-07-21".to_string()),
                due_time: Some("14:30".to_string()),
                memo: None,
            },
        )
        .expect("subtask");
        usecases::create_task(
            &database,
            &create_clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "範囲外".to_string(),
                planned_start_date: Some("2026-10-01".to_string()),
                due_date: None,
                due_time: None,
                memo: None,
            },
        )
        .expect("out of range task");

        usecases::start_timer(
            &database,
            &timer_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Subtask,
                id: subtask.id.clone(),
            },
        )
        .expect("start subtask timer");

        let items = database
            .list_calendar_items("2026-07-01", "2026-07-31")
            .expect("calendar items");
        let planned_subtask = items
            .iter()
            .find(|item| item.id == format!("subtask:{}:planned_start", subtask.id))
            .expect("planned subtask item");
        let active_timer = items
            .iter()
            .find(|item| item.marker == CalendarMarker::ActiveTimer)
            .expect("active timer item");

        assert_eq!(planned_subtask.title, "調査サブタスク");
        assert_eq!(planned_subtask.parent_title.as_deref(), Some("親タスク"));
        assert_eq!(planned_subtask.time, None);
        assert_eq!(planned_subtask.color_token, "blue");
        assert_eq!(planned_subtask.list_color_token, "violet");
        assert_eq!(active_timer.target.target_type, WorkTargetType::Subtask);
        assert_eq!(active_timer.parent_title.as_deref(), Some("親タスク"));
        assert_eq!(active_timer.date, "2026-07-20");
        assert_eq!(active_timer.time.as_deref(), Some("10:15"));
        assert_eq!(active_timer.color_token, "blue");
        assert_eq!(active_timer.list_color_token, "violet");
        assert!(!items.iter().any(|item| item.title == "範囲外"));

        assert!(database
            .list_calendar_items("2026-07-31", "2026-07-01")
            .expect_err("reversed range")
            .contains("終了日は開始日以降"));
        assert!(database
            .list_calendar_items("2026-01-01", "2026-05-01")
            .expect_err("too wide range")
            .contains("93日以内"));
    }

    #[test]
    fn task_row_read_model_aggregates_subtask_progress_and_active_timer() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let start_clock = FixedClock {
            now: "2026-07-06T00:05:00Z",
        };
        let task = usecases::create_task(
            &database,
            &create_clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "UI設計".to_string(),
                planned_start_date: Some("2026-07-06".to_string()),
                due_date: Some("2026-07-07".to_string()),
                due_time: None,
                memo: Some("Read Modelに含めない".to_string()),
            },
        )
        .expect("create task");
        let first_subtask =
            usecases::create_subtask(&database, &create_clock, task.id.clone(), draft("調査"))
                .expect("create first subtask");
        let second_subtask =
            usecases::create_subtask(&database, &create_clock, task.id.clone(), draft("反映"))
                .expect("create second subtask");

        usecases::complete_subtask(&database, &create_clock, first_subtask.id)
            .expect("complete first subtask");
        usecases::start_timer(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Subtask,
                id: second_subtask.id.clone(),
            },
        )
        .expect("start subtask timer");

        let lists = database.list_task_lists().expect("list task lists");
        let rows = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("list task rows");

        assert_eq!(lists.len(), 1);
        assert_eq!(lists[0].id, DEFAULT_TASK_LIST_ID);
        assert_eq!(lists[0].task_count, 1);
        assert_eq!(lists[0].active_task_count, 1);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "UI設計");
        assert_eq!(rows[0].list_id, DEFAULT_TASK_LIST_ID);
        assert_eq!(rows[0].subtask_total_count, 2);
        assert_eq!(rows[0].completed_subtask_count, 1);
        assert_eq!(
            rows[0].active_timer_target,
            Some(WorkTargetRef {
                target_type: WorkTargetType::Subtask,
                id: second_subtask.id,
            })
        );
    }

    #[test]
    fn reopen_subtask_moves_done_subtask_back_to_active_progress() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let reopen_clock = FixedClock {
            now: "2026-07-06T01:00:00Z",
        };
        let task = usecases::create_task(&database, &create_clock, draft("親タスク"))
            .expect("create task");
        let subtask =
            usecases::create_subtask(&database, &create_clock, task.id.clone(), draft("戻す作業"))
                .expect("create subtask");

        usecases::complete_subtask(&database, &create_clock, subtask.id.clone())
            .expect("complete subtask");
        let reopened =
            usecases::reopen_subtask(&database, &reopen_clock, subtask.id).expect("reopen subtask");
        let rows = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("list task rows");

        assert_eq!(reopened.status, WorkStatus::Todo);
        assert_eq!(reopened.completed_at, None);
        assert_eq!(rows[0].completed_subtask_count, 0);
        assert_eq!(rows[0].subtask_total_count, 1);
    }

    #[test]
    fn complete_task_requires_confirmation_for_incomplete_subtasks() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task =
            usecases::create_task(&database, &clock, draft("親タスク")).expect("create task");
        let subtask = usecases::create_subtask(
            &database,
            &clock,
            task.id.clone(),
            draft("未完了サブタスク"),
        )
        .expect("create subtask");

        let rejected = usecases::complete_task(&database, &clock, task.id.clone(), false);
        assert!(rejected
            .expect_err("confirmation required")
            .contains("未完了"));

        let completed =
            usecases::complete_task(&database, &clock, task.id.clone(), true).expect("complete");
        let unchanged_subtask = database
            .with_connection(|connection| select_existing_subtask_by_id(connection, &subtask.id))
            .expect("subtask");

        assert_eq!(completed.status, WorkStatus::Done);
        assert_eq!(
            completed.completed_at.as_deref(),
            Some("2026-07-06T00:00:00Z")
        );
        assert_eq!(unchanged_subtask.status, WorkStatus::Todo);
    }

    #[test]
    fn reopen_task_moves_done_task_back_to_active_rows() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let reopen_clock = FixedClock {
            now: "2026-07-06T00:10:00Z",
        };
        let task =
            usecases::create_task(&database, &create_clock, draft("戻せるタスク")).expect("task");

        usecases::complete_task(&database, &create_clock, task.id.clone(), true).expect("complete");
        let reopened =
            usecases::reopen_task(&database, &reopen_clock, task.id.clone()).expect("reopen");
        let rows = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("task rows");

        assert_eq!(reopened.status, WorkStatus::Todo);
        assert_eq!(reopened.completed_at, None);
        assert_eq!(reopened.updated_at, "2026-07-06T00:10:00Z");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, WorkStatus::Todo);
        assert_eq!(rows[0].completed_at, None);
    }

    #[test]
    fn update_task_status_preserves_completion_rules_for_board() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let done_clock = FixedClock {
            now: "2026-07-06T01:00:00Z",
        };
        let progress_clock = FixedClock {
            now: "2026-07-06T02:00:00Z",
        };
        let task = usecases::create_task(&database, &create_clock, draft("かんばん親タスク"))
            .expect("create task");
        usecases::create_subtask(
            &database,
            &create_clock,
            task.id.clone(),
            draft("未完了サブタスク"),
        )
        .expect("create subtask");

        let rejected = usecases::update_task_status(
            &database,
            &done_clock,
            task.id.clone(),
            "done".to_string(),
            false,
        );
        assert!(rejected
            .expect_err("confirmation required")
            .contains("未完了"));

        let completed = usecases::update_task_status(
            &database,
            &done_clock,
            task.id.clone(),
            "done".to_string(),
            true,
        )
        .expect("complete from board");
        assert_eq!(completed.status, WorkStatus::Done);
        assert_eq!(
            completed.completed_at.as_deref(),
            Some("2026-07-06T01:00:00Z")
        );

        let moved = usecases::update_task_status(
            &database,
            &progress_clock,
            task.id.clone(),
            "in_progress".to_string(),
            false,
        )
        .expect("move to progress");
        let rows = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("task rows");

        assert_eq!(moved.status, WorkStatus::InProgress);
        assert_eq!(moved.completed_at, None);
        assert_eq!(moved.updated_at, "2026-07-06T02:00:00Z");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, WorkStatus::InProgress);
        assert_eq!(rows[0].completed_at, None);
    }

    #[test]
    fn create_task_in_board_column_is_transactional() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-18T00:00:00Z",
        };
        let review = usecases::create_board_column(
            &database,
            &clock,
            usecases::BoardColumnDraft {
                title: "レビュー".to_string(),
            },
        )
        .expect("create review column");

        let task = usecases::create_task_in_board_column(
            &database,
            &clock,
            draft("列指定タスク"),
            review.id.clone(),
        )
        .expect("create task in review column");
        let persisted_column_id: String = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT board_column_id FROM tasks WHERE id = ?1",
                        params![task.id],
                        |row| row.get(0),
                    )
                    .map_err(|error| format!("task board column: {error}"))
            })
            .expect("task board column");
        assert_eq!(persisted_column_id, review.id);

        usecases::delete_board_column(
            &database,
            &clock,
            review.id.clone(),
            DEFAULT_BOARD_COLUMN_ID.to_string(),
        )
        .expect("delete review column");

        let before_count = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("rows before failure")
            .len();
        let rejected = usecases::create_task_in_board_column(
            &database,
            &clock,
            draft("保存されないタスク"),
            review.id,
        );
        assert!(rejected.expect_err("deleted column").contains("状態"));
        let after_count = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("rows after failure")
            .len();
        assert_eq!(after_count, before_count);
    }

    #[test]
    fn board_columns_support_create_rename_reorder_move_and_transactional_delete() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-18T00:00:00Z",
        };
        let update_clock = FixedClock {
            now: "2026-07-18T00:10:00Z",
        };
        let task = usecases::create_task(&database, &create_clock, draft("レビュー対象"))
            .expect("create task");
        let review = usecases::create_board_column(
            &database,
            &create_clock,
            usecases::BoardColumnDraft {
                title: "レビュー".to_string(),
            },
        )
        .expect("create review column");

        let renamed = usecases::update_board_column(
            &database,
            &update_clock,
            review.id.clone(),
            usecases::BoardColumnDraft {
                title: "確認中".to_string(),
            },
        )
        .expect("rename column");
        assert_eq!(renamed.title, "確認中");

        let reordered = usecases::reorder_board_columns(
            &database,
            &update_clock,
            vec![
                review.id.clone(),
                DEFAULT_BOARD_COLUMN_ID.to_string(),
                IN_PROGRESS_BOARD_COLUMN_ID.to_string(),
            ],
        )
        .expect("reorder columns");
        assert_eq!(reordered[0].id, review.id);

        usecases::move_task_to_board_column(
            &database,
            &update_clock,
            task.id.clone(),
            review.id.clone(),
        )
        .expect("move task");
        usecases::complete_task(&database, &update_clock, task.id.clone(), true)
            .expect("complete task");

        let before_delete: (String, String, String) = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT board_column_id, lifecycle_status, completed_at FROM tasks WHERE id = ?1",
                        params![task.id.as_str()],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_err(|error| format!("task state before delete: {error}"))
            })
            .expect("task state before delete");
        assert_eq!(before_delete.0, review.id);
        assert_eq!(before_delete.1, "done");
        assert_eq!(before_delete.2, update_clock.now);

        usecases::delete_board_column(
            &database,
            &update_clock,
            review.id.clone(),
            IN_PROGRESS_BOARD_COLUMN_ID.to_string(),
        )
        .expect("delete column");

        let after_delete: (String, String, String, String) = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT board_column_id, lifecycle_status, status, completed_at FROM tasks WHERE id = ?1",
                        params![task.id.as_str()],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                    )
                    .map_err(|error| format!("task state after delete: {error}"))
            })
            .expect("task state after delete");
        assert_eq!(after_delete.0, IN_PROGRESS_BOARD_COLUMN_ID);
        assert_eq!(after_delete.1, "done");
        assert_eq!(after_delete.2, "done");
        assert_eq!(after_delete.3, update_clock.now);
        assert!(!usecases::list_board_columns(&database)
            .expect("list columns")
            .iter()
            .any(|column| column.id == review.id));

        let reopened =
            usecases::reopen_task(&database, &update_clock, task.id).expect("reopen moved task");
        assert_eq!(reopened.status, WorkStatus::InProgress);
    }

    #[test]
    fn deleting_default_column_keeps_new_tasks_visible_and_last_column_is_protected() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-18T00:00:00Z",
        };

        usecases::delete_board_column(
            &database,
            &clock,
            DEFAULT_BOARD_COLUMN_ID.to_string(),
            IN_PROGRESS_BOARD_COLUMN_ID.to_string(),
        )
        .expect("delete default column");
        let task = usecases::create_task(&database, &clock, draft("削除後の新規タスク"))
            .expect("create task after deleting default column");
        let rows = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("list rows");
        assert_eq!(rows[0].id, task.id);
        assert_eq!(rows[0].board_column_id, IN_PROGRESS_BOARD_COLUMN_ID);

        let error = usecases::delete_board_column(
            &database,
            &clock,
            IN_PROGRESS_BOARD_COLUMN_ID.to_string(),
            "missing-column".to_string(),
        )
        .expect_err("last column must be protected");
        assert!(error.contains("最後の状態"));
    }

    #[test]
    fn archive_task_hides_from_normal_rows_calendar_and_notifications() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-05T00:00:00Z",
        };
        let archive_clock = FixedClock {
            now: "2026-07-05T00:10:00Z",
        };
        let dispatch_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task = usecases::create_task(
            &database,
            &create_clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "アーカイブ対象".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-06".to_string()),
                due_time: None,
                memo: None,
            },
        )
        .expect("create task");
        let subtask = usecases::create_subtask(
            &database,
            &create_clock,
            task.id.clone(),
            usecases::WorkItemDraft {
                list_id: None,
                title: "配下サブタスク".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-06".to_string()),
                due_time: None,
                memo: None,
            },
        )
        .expect("create subtask");

        let archived =
            usecases::archive_task(&database, &archive_clock, task.id.clone()).expect("archive");
        let normal_rows = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("normal rows");
        let archived_rows = database
            .list_archived_task_rows(200)
            .expect("archived rows");
        let task_tree = database.list_tasks_with_subtasks(200).expect("task tree");
        let task_lists = database.list_task_lists().expect("task lists");
        let calendar_items = database
            .list_calendar_items("2026-07-06", "2026-07-06")
            .expect("calendar items");
        let notification_gateway = RecordingNotificationGateway::ok();
        let notification_summary =
            usecases::dispatch_due_notifications(&database, &notification_gateway, &dispatch_clock)
                .expect("dispatch notifications");
        let start_subtask_timer = usecases::start_timer(
            &database,
            &dispatch_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Subtask,
                id: subtask.id,
            },
        );
        let notification_rule_count: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM notification_rules WHERE deleted_at IS NULL",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| format!("notification rule count: {error}"))
            })
            .expect("notification rule count");

        assert_eq!(archived.status, WorkStatus::Archived);
        assert!(normal_rows.is_empty());
        assert_eq!(archived_rows.len(), 1);
        assert_eq!(archived_rows[0].id, task.id);
        assert!(task_tree.is_empty());
        assert_eq!(task_lists[0].task_count, 0);
        assert_eq!(task_lists[0].active_task_count, 0);
        assert_eq!(task_lists[0].completed_task_count, 0);
        assert!(calendar_items.is_empty());
        assert_eq!(notification_summary.attempted, 0);
        assert!(notification_gateway.messages().is_empty());
        assert_eq!(notification_rule_count, 2);
        assert!(start_subtask_timer
            .expect_err("archived parent blocks subtask timer")
            .contains("アーカイブ"));
    }

    #[test]
    fn restore_archived_task_returns_to_normal_rows_without_changing_subtasks() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let restore_clock = FixedClock {
            now: "2026-07-06T01:00:00Z",
        };
        let task =
            usecases::create_task(&database, &create_clock, draft("復元対象")).expect("task");
        let subtask = usecases::create_subtask(
            &database,
            &create_clock,
            task.id.clone(),
            draft("完了済み子"),
        )
        .expect("subtask");

        usecases::complete_subtask(&database, &create_clock, subtask.id.clone())
            .expect("complete subtask");
        usecases::complete_task(&database, &create_clock, task.id.clone(), true)
            .expect("complete task");
        usecases::archive_task(&database, &create_clock, task.id.clone()).expect("archive");
        let restored = usecases::restore_archived_task(&database, &restore_clock, task.id.clone())
            .expect("restore");
        let rows = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("normal rows");
        let archived_rows = database
            .list_archived_task_rows(200)
            .expect("archived rows");
        let tree = database.list_tasks_with_subtasks(200).expect("task tree");

        assert_eq!(restored.status, WorkStatus::Done);
        assert_eq!(
            restored.completed_at.as_deref(),
            Some("2026-07-06T00:00:00Z")
        );
        assert_eq!(restored.updated_at, "2026-07-06T01:00:00Z");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, task.id);
        assert!(archived_rows.is_empty());
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].subtasks.len(), 1);
        assert_eq!(tree[0].subtasks[0].status, WorkStatus::Done);
    }

    #[test]
    fn archive_task_rejects_when_child_subtask_timer_is_active() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task = usecases::create_task(&database, &clock, draft("タイマー中の親")).expect("task");
        let subtask =
            usecases::create_subtask(&database, &clock, task.id.clone(), draft("タイマー中の子"))
                .expect("subtask");

        usecases::start_timer(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Subtask,
                id: subtask.id,
            },
        )
        .expect("start subtask timer");
        let rejected = usecases::archive_task(&database, &clock, task.id.clone());
        let rows = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("normal rows");
        let active_timer = database.get_active_timer().expect("active timer");

        assert!(rejected
            .expect_err("archive active child timer")
            .contains("タイマー開始中"));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, WorkStatus::Todo);
        assert!(rows[0].active_timer_target.is_some());
        assert!(active_timer.is_some());
    }

    #[test]
    fn toggle_task_favorite_updates_task_and_read_model() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task = usecases::create_task(&database, &clock, draft("お気に入り対象")).expect("task");

        let favorited = usecases::toggle_task_favorite(&database, &clock, task.id.clone(), true)
            .expect("favorite task");
        let rows = database
            .list_task_rows(Some(DEFAULT_TASK_LIST_ID), 200)
            .expect("task rows");

        assert!(favorited.is_favorite);
        assert_eq!(favorited.updated_at, "2026-07-06T00:00:00Z");
        assert_eq!(rows.len(), 1);
        assert!(rows[0].is_favorite);

        let unfavorited = usecases::toggle_task_favorite(&database, &clock, task.id.clone(), false)
            .expect("unfavorite task");

        assert!(!unfavorited.is_favorite);
    }

    #[test]
    fn update_task_detail_syncs_notification_rules_and_timer_target() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let update_clock = FixedClock {
            now: "2026-07-06T00:10:00Z",
        };
        let task = usecases::create_task(
            &database,
            &create_clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "更新前".to_string(),
                planned_start_date: Some("2026-07-06".to_string()),
                due_date: Some("2026-07-07".to_string()),
                due_time: None,
                memo: Some("古いメモ".to_string()),
            },
        )
        .expect("create task");

        let updated = usecases::update_task(
            &database,
            &update_clock,
            task.id.clone(),
            usecases::WorkItemUpdateDraft {
                list_id: None,
                title: "更新後".to_string(),
                planned_start_date: Some("2026-07-08".to_string()),
                due_date: None,
                due_time: None,
                timer_target_seconds: Some(900),
                color_token: None,
                recurrence_rule: None,
                memo: Some("新しいメモ".to_string()),
            },
        )
        .expect("update task");

        let (active_rules, disabled_due_rules): (Vec<(String, String, String)>, i64) = database
            .with_connection(|connection| {
                let mut statement = connection
                    .prepare(
                        "
                        SELECT kind, notify_at, registration_status
                        FROM notification_rules
                        WHERE target_type = 'task'
                          AND target_id = ?1
                          AND deleted_at IS NULL
                        ORDER BY kind ASC
                        ",
                    )
                    .map_err(|error| format!("active rules query: {error}"))?;
                let active_rules = statement
                    .query_map(params![task.id.as_str()], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })
                    .map_err(|error| format!("active rules: {error}"))?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| format!("active rule row: {error}"))?;
                let disabled_due_rules = connection
                    .query_row(
                        "
                        SELECT COUNT(*)
                        FROM notification_rules
                        WHERE target_type = 'task'
                          AND target_id = ?1
                          AND kind = 'due'
                          AND enabled = 0
                          AND registration_status = 'disabled'
                          AND deleted_at IS NOT NULL
                        ",
                        params![task.id.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(|error| format!("disabled due count: {error}"))?;
                Ok((active_rules, disabled_due_rules))
            })
            .expect("notification rules");

        assert_eq!(updated.title, "更新後");
        assert_eq!(updated.planned_start_date.as_deref(), Some("2026-07-08"));
        assert_eq!(updated.due_date, None);
        assert_eq!(updated.timer_target_seconds, Some(900));
        assert_eq!(updated.memo, "新しいメモ");
        assert_eq!(active_rules.len(), 1);
        assert_eq!(
            active_rules[0],
            (
                "planned_start".to_string(),
                "2026-07-08T00:00:00Z".to_string(),
                "pending".to_string(),
            )
        );
        assert_eq!(disabled_due_rules, 1);

        let active_timer = usecases::start_timer(
            &database,
            &update_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start timer with task target");
        assert_eq!(active_timer.target_seconds, Some(900));
    }

    #[test]
    fn task_update_marks_os_registration_pending_without_reopening_dispatch_status() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T09:00:00Z",
        };
        let update_clock = FixedClock {
            now: "2026-07-06T09:05:00Z",
        };
        let task = usecases::create_task(
            &database,
            &create_clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "OS登録状態更新".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-07".to_string()),
                due_time: Some("10:00".to_string()),
                memo: None,
            },
        )
        .expect("create task");

        database
            .with_connection(|connection| {
                connection
                    .execute(
                        "
                        UPDATE notification_rules
                        SET registration_status = 'registered'
                        WHERE target_type = 'task'
                          AND target_id = ?1
                          AND kind = 'due'
                        ",
                        params![task.id.as_str()],
                    )
                    .map_err(|error| format!("mark rule registered: {error}"))?;
                connection
                    .execute(
                        "
                        UPDATE notification_os_registrations
                        SET os_registration_id = 'windows-notification-id',
                            registration_status = 'registered',
                            last_attempted_at = '2026-07-06T09:01:00Z'
                        WHERE notification_rule_id IN (
                          SELECT id
                          FROM notification_rules
                          WHERE target_type = 'task'
                            AND target_id = ?1
                            AND kind = 'due'
                        )
                        ",
                        params![task.id.as_str()],
                    )
                    .map_err(|error| format!("mark os registered: {error}"))?;
                Ok(())
            })
            .expect("prepare registered state");

        usecases::update_task(
            &database,
            &update_clock,
            task.id.clone(),
            usecases::WorkItemUpdateDraft {
                list_id: None,
                title: "OS登録状態更新 タイトル変更".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-07".to_string()),
                due_time: Some("10:00".to_string()),
                timer_target_seconds: None,
                color_token: None,
                recurrence_rule: None,
                memo: None,
            },
        )
        .expect("update task");

        let (rule_status, os_status, os_registration_id): (String, String, String) = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "
                        SELECT notification_rules.registration_status,
                               notification_os_registrations.registration_status,
                               notification_os_registrations.os_registration_id
                        FROM notification_rules
                        INNER JOIN notification_os_registrations
                          ON notification_os_registrations.notification_rule_id = notification_rules.id
                         AND notification_os_registrations.deleted_at IS NULL
                        WHERE notification_rules.target_type = 'task'
                          AND notification_rules.target_id = ?1
                          AND notification_rules.kind = 'due'
                        ",
                        params![task.id.as_str()],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .map_err(|error| format!("select statuses: {error}"))
            })
            .expect("statuses");
        let jobs = usecases::list_notification_os_registration_jobs(&database, &update_clock)
            .expect("os registration jobs");

        assert_eq!(rule_status, "registered");
        assert_eq!(os_status, "pending");
        assert_eq!(os_registration_id, "windows-notification-id");
        assert_eq!(jobs.len(), 1);
        assert_eq!(
            jobs[0].action,
            NotificationOsRegistrationAction::RegisterOrReplace
        );
        assert_eq!(
            jobs[0].registration_status,
            NotificationOsRegistrationStatus::Pending
        );
        assert_eq!(
            jobs[0].os_registration_id.as_deref(),
            Some("windows-notification-id")
        );
    }

    #[test]
    fn deleting_task_keeps_os_registration_cancel_job_until_cancelled() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T09:00:00Z",
        };
        let delete_clock = FixedClock {
            now: "2026-07-06T09:05:00Z",
        };
        let cancelled_clock = FixedClock {
            now: "2026-07-06T09:06:00Z",
        };
        let task = usecases::create_task(
            &database,
            &create_clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "削除時OS解除".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-07".to_string()),
                due_time: Some("10:00".to_string()),
                memo: None,
            },
        )
        .expect("create task");

        database
            .with_connection(|connection| {
                connection
                    .execute(
                        "
                        UPDATE notification_os_registrations
                        SET os_registration_id = 'windows-delete-id',
                            registration_status = 'registered',
                            last_attempted_at = '2026-07-06T09:01:00Z'
                        WHERE notification_rule_id IN (
                          SELECT id
                          FROM notification_rules
                          WHERE target_type = 'task'
                            AND target_id = ?1
                            AND kind = 'due'
                        )
                        ",
                        params![task.id.as_str()],
                    )
                    .map(|_| ())
                    .map_err(|error| format!("mark os registered: {error}"))
            })
            .expect("prepare os registration");

        usecases::delete_task(&database, &delete_clock, task.id.clone()).expect("delete task");
        let jobs = usecases::list_notification_os_registration_jobs(&database, &delete_clock)
            .expect("cancel jobs");

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].action, NotificationOsRegistrationAction::Cancel);
        assert_eq!(
            jobs[0].registration_status,
            NotificationOsRegistrationStatus::CancelPending
        );
        assert_eq!(
            jobs[0].os_registration_id.as_deref(),
            Some("windows-delete-id")
        );

        usecases::mark_notification_os_registration_cancelled(
            &database,
            &cancelled_clock,
            jobs[0].id.clone(),
        )
        .expect("mark cancelled");
        let remaining =
            usecases::list_notification_os_registration_jobs(&database, &cancelled_clock)
                .expect("remaining jobs");
        assert!(remaining.is_empty());
    }

    #[test]
    fn update_notifications_enabled_skips_and_resumes_dispatch() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let enable_clock = FixedClock {
            now: "2026-07-06T00:10:00Z",
        };
        let notification_gateway = RecordingNotificationGateway::ok();
        let task = usecases::create_task(
            &database,
            &create_clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "通知切替対象".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-06".to_string()),
                due_time: None,
                memo: None,
            },
        )
        .expect("create task");
        usecases::update_notifications_enabled(&database, &create_clock, false)
            .expect("disable notifications");
        let skipped_summary =
            usecases::dispatch_due_notifications(&database, &notification_gateway, &create_clock)
                .expect("dispatch disabled");
        let pending_count_after_skip: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "
                        SELECT COUNT(*)
                        FROM notification_rules
                        WHERE target_id = ?1
                          AND registration_status = 'pending'
                          AND enabled = 1
                          AND deleted_at IS NULL
                        ",
                        params![task.id.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(|error| format!("pending count: {error}"))
            })
            .expect("pending notifications");

        assert!(!database
            .get_notifications_enabled()
            .expect("enabled setting"));
        assert_eq!(skipped_summary.attempted, 0);
        assert!(notification_gateway.messages().is_empty());
        assert_eq!(pending_count_after_skip, 1);

        let enabled = usecases::update_notifications_enabled(&database, &enable_clock, true)
            .expect("enable notifications");
        let delivered_summary =
            usecases::dispatch_due_notifications(&database, &notification_gateway, &enable_clock)
                .expect("dispatch reenabled");

        assert!(enabled);
        assert_eq!(delivered_summary.attempted, 1);
        assert_eq!(delivered_summary.succeeded, 1);
        assert_eq!(notification_gateway.messages().len(), 1);
    }

    #[test]
    fn get_next_pending_notification_returns_earliest_future_rule() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T09:00:00Z",
        };

        for (title, due_time) in [
            ("過去の通知", "08:00"),
            ("後の通知", "10:00"),
            ("最初の未来通知", "09:30"),
        ] {
            usecases::create_task(
                &database,
                &clock,
                usecases::WorkItemDraft {
                    list_id: None,
                    title: title.to_string(),
                    planned_start_date: None,
                    due_date: Some("2026-07-06".to_string()),
                    due_time: Some(due_time.to_string()),
                    memo: None,
                },
            )
            .expect("create task");
        }

        let next = usecases::get_next_pending_notification(&database, &clock)
            .expect("next pending notification")
            .expect("future notification");

        assert_eq!(next.notify_at, "2026-07-06T09:30:00Z");
    }

    #[test]
    fn get_next_pending_notification_respects_notifications_enabled() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T09:00:00Z",
        };

        usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "通知OFF確認".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-06".to_string()),
                due_time: Some("10:00".to_string()),
                memo: None,
            },
        )
        .expect("create task");

        usecases::update_notifications_enabled(&database, &clock, false)
            .expect("disable notifications");

        let next = usecases::get_next_pending_notification(&database, &clock)
            .expect("next pending notification");

        assert_eq!(next, None);
    }

    #[test]
    fn sync_notifications_dispatches_due_after_resume_and_reschedules_future() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T09:00:00Z",
        };
        let resume_clock = FixedClock {
            now: "2026-07-06T09:06:00Z",
        };
        let second_resume_clock = FixedClock {
            now: "2026-07-06T09:07:00Z",
        };
        let notification_gateway = RecordingNotificationGateway::ok();

        for (title, due_time) in [("復帰時に送る通知", "09:05"), ("次に予約する通知", "09:10")]
        {
            usecases::create_task(
                &database,
                &create_clock,
                usecases::WorkItemDraft {
                    list_id: None,
                    title: title.to_string(),
                    planned_start_date: None,
                    due_date: Some("2026-07-06".to_string()),
                    due_time: Some(due_time.to_string()),
                    memo: None,
                },
            )
            .expect("create task");
        }

        let initial_sync =
            usecases::sync_notifications(&database, &notification_gateway, &create_clock)
                .expect("initial sync");
        let resume_sync =
            usecases::sync_notifications(&database, &notification_gateway, &resume_clock)
                .expect("resume sync");
        let duplicate_guard_sync =
            usecases::sync_notifications(&database, &notification_gateway, &second_resume_clock)
                .expect("duplicate guard sync");

        assert_eq!(initial_sync.dispatch_summary.attempted, 0);
        assert_eq!(
            initial_sync
                .next_schedule
                .as_ref()
                .map(|schedule| schedule.notify_at.as_str()),
            Some("2026-07-06T09:05:00Z")
        );
        assert_eq!(resume_sync.dispatch_summary.attempted, 1);
        assert_eq!(resume_sync.dispatch_summary.succeeded, 1);
        assert_eq!(
            resume_sync
                .next_schedule
                .as_ref()
                .map(|schedule| schedule.notify_at.as_str()),
            Some("2026-07-06T09:10:00Z")
        );
        assert_eq!(duplicate_guard_sync.dispatch_summary.attempted, 0);
        assert_eq!(
            duplicate_guard_sync
                .next_schedule
                .as_ref()
                .map(|schedule| schedule.notify_at.as_str()),
            Some("2026-07-06T09:10:00Z")
        );
        assert_eq!(notification_gateway.messages().len(), 1);
    }

    #[test]
    fn sync_notifications_skips_dispatch_and_schedule_when_disabled() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T09:00:00Z",
        };
        let notification_gateway = RecordingNotificationGateway::ok();

        usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "通知OFF同期".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-06".to_string()),
                due_time: Some("09:05".to_string()),
                memo: None,
            },
        )
        .expect("create task");
        usecases::update_notifications_enabled(&database, &clock, false)
            .expect("disable notifications");

        let sync_result = usecases::sync_notifications(&database, &notification_gateway, &clock)
            .expect("notification sync");

        assert_eq!(sync_result.dispatch_summary.attempted, 0);
        assert_eq!(sync_result.next_schedule, None);
        assert!(notification_gateway.messages().is_empty());
    }

    #[test]
    fn update_task_detail_saves_and_disables_recurrence_rule() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let update_clock = FixedClock {
            now: "2026-07-06T00:10:00Z",
        };
        let task =
            usecases::create_task(&database, &create_clock, draft("繰り返し対象")).expect("task");

        let updated = usecases::update_task(
            &database,
            &update_clock,
            task.id.clone(),
            usecases::WorkItemUpdateDraft {
                list_id: None,
                title: "繰り返し対象".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-10".to_string()),
                due_time: None,
                timer_target_seconds: Some(1_200),
                color_token: None,
                recurrence_rule: Some(usecases::RecurrenceRuleDraft {
                    frequency: "weekly".to_string(),
                    interval: 2,
                }),
                memo: Some("隔週で確認".to_string()),
            },
        )
        .expect("update recurrence");
        let recurrence = updated.recurrence_rule.expect("recurrence rule");

        assert_eq!(recurrence.frequency, RecurrenceFrequency::Weekly);
        assert_eq!(recurrence.interval, 2);
        assert_eq!(recurrence.target.id, task.id);

        let disabled = usecases::update_task(
            &database,
            &update_clock,
            recurrence.target.id.clone(),
            usecases::WorkItemUpdateDraft {
                list_id: None,
                title: "繰り返し対象".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-10".to_string()),
                due_time: None,
                timer_target_seconds: Some(1_200),
                color_token: None,
                recurrence_rule: None,
                memo: Some("繰り返し解除".to_string()),
            },
        )
        .expect("disable recurrence");
        let deleted_recurrence_count: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "
                        SELECT COUNT(*)
                        FROM recurrence_rules
                        WHERE target_type = 'task'
                          AND target_id = ?1
                          AND deleted_at IS NOT NULL
                        ",
                        params![disabled.id.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(|error| format!("deleted recurrence count: {error}"))
            })
            .expect("deleted recurrence count");

        assert!(disabled.recurrence_rule.is_none());
        assert_eq!(deleted_recurrence_count, 1);
    }

    #[test]
    fn update_work_item_rejects_recurrence_without_base_date() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task = usecases::create_task(&database, &clock, draft("基準日なし")).expect("task");

        let result = usecases::update_task(
            &database,
            &clock,
            task.id,
            usecases::WorkItemUpdateDraft {
                list_id: None,
                title: "基準日なし".to_string(),
                planned_start_date: None,
                due_date: None,
                due_time: None,
                timer_target_seconds: None,
                color_token: None,
                recurrence_rule: Some(usecases::RecurrenceRuleDraft {
                    frequency: "daily".to_string(),
                    interval: 1,
                }),
                memo: None,
            },
        );

        assert!(result
            .expect_err("missing recurrence base date")
            .contains("繰り返し設定"));
    }

    #[test]
    fn update_subtask_detail_syncs_due_notification_and_timer_target() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task =
            usecases::create_task(&database, &clock, draft("親タスク")).expect("create task");
        let subtask =
            usecases::create_subtask(&database, &clock, task.id.clone(), draft("子タスク"))
                .expect("create subtask");

        let updated = usecases::update_subtask(
            &database,
            &clock,
            subtask.id.clone(),
            usecases::WorkItemUpdateDraft {
                list_id: None,
                title: "子タスク更新".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-09".to_string()),
                due_time: Some("16:45".to_string()),
                timer_target_seconds: Some(900),
                color_token: None,
                recurrence_rule: None,
                memo: Some("サブタスクメモ".to_string()),
            },
        )
        .expect("update subtask");

        let due_rule: (String, String) = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "
                        SELECT notify_at, registration_status
                        FROM notification_rules
                        WHERE target_type = 'subtask'
                          AND target_id = ?1
                          AND kind = 'due'
                          AND deleted_at IS NULL
                        ",
                        params![subtask.id.as_str()],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|error| format!("subtask due rule: {error}"))
            })
            .expect("subtask due rule");

        assert_eq!(updated.title, "子タスク更新");
        assert_eq!(updated.due_date.as_deref(), Some("2026-07-09"));
        assert_eq!(updated.timer_target_seconds, Some(900));
        assert_eq!(
            due_rule,
            ("2026-07-09T16:45:00Z".to_string(), "pending".to_string())
        );
    }

    #[test]
    fn update_subtask_detail_saves_recurrence_rule() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task =
            usecases::create_task(&database, &clock, draft("親タスク")).expect("create task");
        let subtask = usecases::create_subtask(&database, &clock, task.id, draft("月次サブタスク"))
            .expect("create subtask");

        let updated = usecases::update_subtask(
            &database,
            &clock,
            subtask.id.clone(),
            usecases::WorkItemUpdateDraft {
                list_id: None,
                title: "月次サブタスク".to_string(),
                planned_start_date: Some("2026-07-09".to_string()),
                due_date: None,
                due_time: None,
                timer_target_seconds: Some(600),
                color_token: None,
                recurrence_rule: Some(usecases::RecurrenceRuleDraft {
                    frequency: "monthly".to_string(),
                    interval: 1,
                }),
                memo: Some("毎月確認".to_string()),
            },
        )
        .expect("update subtask recurrence");
        let recurrence = updated.recurrence_rule.expect("recurrence rule");

        assert_eq!(recurrence.target.id, subtask.id);
        assert_eq!(recurrence.frequency, RecurrenceFrequency::Monthly);
        assert_eq!(recurrence.interval, 1);
    }

    #[test]
    fn update_task_rejects_invalid_timer_target() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task =
            usecases::create_task(&database, &clock, draft("目標時間")).expect("create task");

        let result = usecases::update_task(
            &database,
            &clock,
            task.id,
            usecases::WorkItemUpdateDraft {
                list_id: None,
                title: "目標時間".to_string(),
                planned_start_date: None,
                due_date: None,
                due_time: None,
                timer_target_seconds: Some(0),
                color_token: None,
                recurrence_rule: None,
                memo: None,
            },
        );

        assert!(result
            .expect_err("invalid target")
            .contains("タイマー目標時間"));
    }

    #[test]
    fn delete_task_soft_deletes_children_active_timer_and_notifications() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task =
            usecases::create_task(&database, &clock, draft("削除対象")).expect("create task");
        let subtask =
            usecases::create_subtask(&database, &clock, task.id.clone(), draft("子サブタスク"))
                .expect("create subtask");
        insert_notification_rule(&database, WorkTargetType::Task, &task.id);
        insert_notification_rule(&database, WorkTargetType::Subtask, &subtask.id);
        insert_recurrence_rule(&database, WorkTargetType::Task, &task.id);
        insert_recurrence_rule(&database, WorkTargetType::Subtask, &subtask.id);

        usecases::start_timer(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id.clone(),
            },
        )
        .expect("start task timer");
        usecases::pause_active_timer(&database, &clock).expect("pause active timer");

        usecases::delete_task(&database, &clock, task.id.clone()).expect("delete task");

        let tasks = database
            .list_tasks_with_subtasks(200)
            .expect("list task tree");
        let active_timer = database.get_active_timer().expect("active timer");
        let (
            deleted_tasks,
            deleted_subtasks,
            deleted_timers,
            deleted_timer_pauses,
            deleted_recurrences,
            disabled_notifications,
        ): (
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
        ) = database
            .with_connection(|connection| {
                let deleted_tasks = connection
                    .query_row(
                        "SELECT COUNT(*) FROM tasks WHERE id = ?1 AND deleted_at IS NOT NULL",
                        params![task.id],
                        |row| row.get(0),
                    )
                    .expect("deleted tasks");
                let deleted_subtasks = connection
                    .query_row(
                        "SELECT COUNT(*) FROM subtasks WHERE task_id = ?1 AND deleted_at IS NOT NULL",
                        params![task.id],
                        |row| row.get(0),
                    )
                    .expect("deleted subtasks");
                let deleted_timers = connection
                    .query_row(
                        "SELECT COUNT(*) FROM timer_sessions WHERE deleted_at IS NOT NULL",
                        [],
                        |row| row.get(0),
                    )
                    .expect("deleted timers");
                let deleted_timer_pauses = connection
                    .query_row(
                        "SELECT COUNT(*) FROM timer_pauses WHERE deleted_at IS NOT NULL",
                        [],
                        |row| row.get(0),
                    )
                    .expect("deleted timer pauses");
                let deleted_recurrences = connection
                    .query_row(
                        "SELECT COUNT(*) FROM recurrence_rules WHERE deleted_at IS NOT NULL",
                        [],
                        |row| row.get(0),
                    )
                    .expect("deleted recurrences");
                let disabled_notifications = connection
                    .query_row(
                        "
                        SELECT COUNT(*)
                        FROM notification_rules
                        WHERE deleted_at IS NOT NULL
                          AND enabled = 0
                          AND registration_status = 'disabled'
                        ",
                        [],
                        |row| row.get(0),
                    )
                    .expect("disabled notifications");
                Ok((
                    deleted_tasks,
                    deleted_subtasks,
                    deleted_timers,
                    deleted_timer_pauses,
                    deleted_recurrences,
                    disabled_notifications,
                ))
            })
            .expect("deleted graph counts");

        assert!(tasks.is_empty());
        assert!(active_timer.is_none());
        assert_eq!(deleted_tasks, 1);
        assert_eq!(deleted_subtasks, 1);
        assert_eq!(deleted_timers, 1);
        assert_eq!(deleted_timer_pauses, 1);
        assert_eq!(deleted_recurrences, 2);
        assert_eq!(disabled_notifications, 2);
    }

    #[test]
    fn delete_task_soft_deletes_active_pomodoro() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task = usecases::create_task(&database, &clock, draft("ポモドーロ削除")).expect("task");

        usecases::start_legacy_task_linked_pomodoro(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id.clone(),
            },
        )
        .expect("start pomodoro");
        usecases::delete_task(&database, &clock, task.id).expect("delete task");

        let active_timer = database.get_active_timer().expect("active timer");
        let active_pomodoro = usecases::get_active_pomodoro(&database).expect("active pomodoro");
        let (deleted_timers, deleted_pomodoros): (i64, i64) = database
            .with_connection(|connection| {
                let deleted_timers = connection
                    .query_row(
                        "SELECT COUNT(*) FROM timer_sessions WHERE deleted_at IS NOT NULL",
                        [],
                        |row| row.get(0),
                    )
                    .expect("deleted timers");
                let deleted_pomodoros = connection
                    .query_row(
                        "SELECT COUNT(*) FROM pomodoro_sessions WHERE deleted_at IS NOT NULL",
                        [],
                        |row| row.get(0),
                    )
                    .expect("deleted pomodoros");
                Ok((deleted_timers, deleted_pomodoros))
            })
            .expect("deleted counts");

        assert!(active_timer.is_none());
        assert!(active_pomodoro.is_none());
        assert_eq!(deleted_timers, 1);
        assert_eq!(deleted_pomodoros, 1);
    }

    #[test]
    fn delete_subtask_soft_deletes_active_timer_and_notification() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let task =
            usecases::create_task(&database, &clock, draft("親タスク")).expect("create task");
        let subtask =
            usecases::create_subtask(&database, &clock, task.id.clone(), draft("削除対象"))
                .expect("create subtask");
        insert_notification_rule(&database, WorkTargetType::Subtask, &subtask.id);
        insert_recurrence_rule(&database, WorkTargetType::Subtask, &subtask.id);

        usecases::start_timer(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Subtask,
                id: subtask.id.clone(),
            },
        )
        .expect("start subtask timer");
        usecases::pause_active_timer(&database, &clock).expect("pause active timer");

        usecases::delete_subtask(&database, &clock, subtask.id.clone()).expect("delete subtask");

        let tasks = database
            .list_tasks_with_subtasks(200)
            .expect("list task tree");
        let active_timer = database.get_active_timer().expect("active timer");
        let (
            deleted_subtasks,
            deleted_timers,
            deleted_timer_pauses,
            deleted_recurrences,
            disabled_notifications,
        ): (i64, i64, i64, i64, i64) = database
            .with_connection(|connection| {
                let deleted_subtasks = connection
                    .query_row(
                        "SELECT COUNT(*) FROM subtasks WHERE id = ?1 AND deleted_at IS NOT NULL",
                        params![subtask.id],
                        |row| row.get(0),
                    )
                    .expect("deleted subtasks");
                let deleted_timers = connection
                    .query_row(
                        "SELECT COUNT(*) FROM timer_sessions WHERE deleted_at IS NOT NULL",
                        [],
                        |row| row.get(0),
                    )
                    .expect("deleted timers");
                let deleted_timer_pauses = connection
                    .query_row(
                        "SELECT COUNT(*) FROM timer_pauses WHERE deleted_at IS NOT NULL",
                        [],
                        |row| row.get(0),
                    )
                    .expect("deleted timer pauses");
                let deleted_recurrences = connection
                    .query_row(
                        "SELECT COUNT(*) FROM recurrence_rules WHERE deleted_at IS NOT NULL",
                        [],
                        |row| row.get(0),
                    )
                    .expect("deleted recurrences");
                let disabled_notifications = connection
                    .query_row(
                        "
                        SELECT COUNT(*)
                        FROM notification_rules
                        WHERE deleted_at IS NOT NULL
                          AND enabled = 0
                          AND registration_status = 'disabled'
                        ",
                        [],
                        |row| row.get(0),
                    )
                    .expect("disabled notifications");
                Ok((
                    deleted_subtasks,
                    deleted_timers,
                    deleted_timer_pauses,
                    deleted_recurrences,
                    disabled_notifications,
                ))
            })
            .expect("deleted subtask graph counts");

        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].subtasks.is_empty());
        assert!(active_timer.is_none());
        assert_eq!(deleted_subtasks, 1);
        assert_eq!(deleted_timers, 1);
        assert_eq!(deleted_timer_pauses, 1);
        assert_eq!(deleted_recurrences, 1);
        assert_eq!(disabled_notifications, 1);
    }

    #[test]
    fn dispatch_due_notifications_hides_title_in_generic_mode() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let dispatch_clock = FixedClock {
            now: "2026-07-07T00:00:00Z",
        };
        let notification_gateway = RecordingNotificationGateway::ok();

        usecases::create_task(
            &database,
            &create_clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "秘密の顧客タスク".to_string(),
                planned_start_date: Some("2026-07-06".to_string()),
                due_date: Some("2026-07-07".to_string()),
                due_time: None,
                memo: Some("通知に出してはいけないメモ".to_string()),
            },
        )
        .expect("create task");
        usecases::update_notification_display_mode(
            &database,
            &create_clock,
            NotificationDisplayMode::Generic,
        )
        .expect("update preference");

        let summary =
            usecases::dispatch_due_notifications(&database, &notification_gateway, &dispatch_clock)
                .expect("dispatch notifications");
        let messages = notification_gateway.messages();
        let registered_count: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "
                        SELECT COUNT(*)
                        FROM notification_rules
                        WHERE registration_status = 'registered'
                        ",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| format!("registered count: {error}"))
            })
            .expect("registered count");

        assert_eq!(summary.attempted, 2);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.failed, 0);
        assert_eq!(messages.len(), 2);
        assert!(messages.iter().all(|message| message.title == "TaskTimer"));
        assert!(messages
            .iter()
            .all(|message| !message.title.contains("秘密")
                && !message.body.contains("秘密")
                && !message.body.contains("メモ")));
        assert_eq!(registered_count, 2);
    }

    #[test]
    fn dispatch_due_notifications_records_failures_for_retry() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let retry_clock = FixedClock {
            now: "2026-07-06T00:05:00Z",
        };
        let notification_gateway = RecordingNotificationGateway::failing("permission denied");
        let retry_gateway = RecordingNotificationGateway::ok();

        usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "通知失敗確認".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-06".to_string()),
                due_time: None,
                memo: None,
            },
        )
        .expect("create task");

        let summary =
            usecases::dispatch_due_notifications(&database, &notification_gateway, &clock)
                .expect("dispatch notifications");
        let (failed_count, last_error): (i64, String) = database
            .with_connection(|connection| {
                let failed_count = connection
                    .query_row(
                        "
                        SELECT COUNT(*)
                        FROM notification_rules
                        WHERE registration_status = 'failed'
                        ",
                        [],
                        |row| row.get(0),
                    )
                    .expect("failed count");
                let last_error = connection
                    .query_row(
                        "
                        SELECT last_error
                        FROM notification_rules
                        WHERE registration_status = 'failed'
                        LIMIT 1
                        ",
                        [],
                        |row| row.get(0),
                    )
                    .expect("last error");
                Ok((failed_count, last_error))
            })
            .expect("failed state");

        assert_eq!(summary.attempted, 1);
        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.last_error.as_deref(), Some("permission denied"));
        assert_eq!(failed_count, 1);
        assert_eq!(last_error, "permission denied");

        let failed_history = usecases::list_notification_failure_history(&database)
            .expect("failed notification history");
        assert_eq!(failed_history.len(), 1);
        assert_eq!(failed_history[0].target.target_type, WorkTargetType::Task);
        assert_eq!(failed_history[0].kind, NotificationKind::Due);
        assert_eq!(failed_history[0].result, NotificationDeliveryResult::Failed);
        assert_eq!(failed_history[0].attempt_count, 1);
        assert_eq!(
            failed_history[0].error_message.as_deref(),
            Some("permission denied")
        );

        let retry_summary =
            usecases::dispatch_due_notifications(&database, &retry_gateway, &retry_clock)
                .expect("retry notifications");
        let retry_history = database
            .list_notification_failure_history(20)
            .expect("retry notification history");

        assert_eq!(retry_summary.attempted, 1);
        assert_eq!(retry_summary.succeeded, 1);
        assert_eq!(retry_summary.failed, 0);
        assert_eq!(retry_history.len(), 2);
        assert_eq!(retry_history[0].result, NotificationDeliveryResult::Success);
        assert_eq!(retry_history[0].attempt_count, 2);
        assert_eq!(retry_history[0].error_message, None);
        assert_eq!(retry_history[1].result, NotificationDeliveryResult::Failed);
        assert_eq!(retry_history[1].attempt_count, 1);
    }

    #[test]
    fn dispatch_due_notifications_skips_registered_rules_after_resume_sync() {
        let database = in_memory_database();
        let create_clock = FixedClock {
            now: "2026-07-06T00:00:00Z",
        };
        let resume_clock = FixedClock {
            now: "2026-07-06T03:00:00Z",
        };
        let notification_gateway = RecordingNotificationGateway::ok();

        usecases::create_task(
            &database,
            &create_clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "復帰時通知".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-06".to_string()),
                due_time: None,
                memo: None,
            },
        )
        .expect("create task");

        let first_summary =
            usecases::dispatch_due_notifications(&database, &notification_gateway, &create_clock)
                .expect("first dispatch");
        let second_summary =
            usecases::dispatch_due_notifications(&database, &notification_gateway, &resume_clock)
                .expect("resume dispatch");
        let next_notification = usecases::get_next_pending_notification(&database, &resume_clock)
            .expect("next pending notification");
        let registered_count: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "
                        SELECT COUNT(*)
                        FROM notification_rules
                        WHERE registration_status = 'registered'
                        ",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| format!("registered count: {error}"))
            })
            .expect("registered count");

        assert_eq!(first_summary.attempted, 1);
        assert_eq!(first_summary.succeeded, 1);
        assert_eq!(second_summary.attempted, 0);
        assert_eq!(second_summary.succeeded, 0);
        assert_eq!(notification_gateway.messages().len(), 1);
        assert_eq!(registered_count, 1);
        assert_eq!(next_notification, None);
    }

    #[test]
    fn scheduled_ranges_support_tasks_subtasks_calendar_and_recurrence() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-18T00:00:00Z",
        };
        let task = usecases::create_scheduled_task(
            &database,
            &clock,
            draft("期間付きタスク"),
            usecases::WorkScheduleDraft {
                start_date: "2026-07-19".to_string(),
                start_time: Some("22:00".to_string()),
                end_date: "2026-07-20".to_string(),
                end_time: Some("10:00".to_string()),
                is_all_day: false,
            },
        )
        .expect("create scheduled task");
        let subtask = usecases::create_subtask(
            &database,
            &clock,
            task.id.clone(),
            usecases::WorkItemDraft {
                list_id: None,
                title: "終日サブタスク".to_string(),
                planned_start_date: Some("2026-07-20".to_string()),
                due_date: None,
                due_time: None,
                memo: None,
            },
        )
        .expect("create subtask");
        usecases::update_subtask(
            &database,
            &clock,
            subtask.id.clone(),
            usecases::WorkItemUpdateDraft {
                list_id: None,
                title: subtask.title.clone(),
                planned_start_date: Some("2026-07-20".to_string()),
                due_date: None,
                due_time: None,
                timer_target_seconds: None,
                color_token: None,
                recurrence_rule: Some(usecases::RecurrenceRuleDraft {
                    frequency: "weekly".to_string(),
                    interval: 1,
                }),
                memo: None,
            },
        )
        .expect("add recurrence");
        usecases::resize_scheduled_work_item(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Subtask,
                id: subtask.id.clone(),
            },
            usecases::WorkScheduleDraft {
                start_date: "2026-07-20".to_string(),
                start_time: None,
                end_date: "2026-07-22".to_string(),
                end_time: None,
                is_all_day: true,
            },
        )
        .expect("schedule subtask");

        let notification_before_move: (String, String) = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT kind, notify_at FROM notification_rules LIMIT 1",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|error| format!("notification before move: {error}"))
            })
            .expect("notification before move");

        usecases::move_scheduled_work_item(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id.clone(),
            },
            usecases::WorkScheduleMoveDraft {
                start_date: "2026-07-21".to_string(),
                start_time: Some("09:30".to_string()),
            },
        )
        .expect("move timed task");
        usecases::move_scheduled_work_item(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Subtask,
                id: subtask.id.clone(),
            },
            usecases::WorkScheduleMoveDraft {
                start_date: "2026-07-23".to_string(),
                start_time: None,
            },
        )
        .expect("move all-day subtask");

        let items = database
            .list_calendar_items("2026-07-21", "2026-07-25")
            .expect("calendar items");
        let scheduled_task = items
            .iter()
            .find(|item| item.id == format!("task:{}:scheduled", task.id))
            .expect("scheduled task item");
        assert_eq!(scheduled_task.date, "2026-07-21");
        assert_eq!(scheduled_task.time.as_deref(), Some("09:30"));
        assert_eq!(scheduled_task.end_date.as_deref(), Some("2026-07-21"));
        assert_eq!(scheduled_task.end_time.as_deref(), Some("21:30"));
        let scheduled_subtask = items
            .iter()
            .find(|item| item.id == format!("subtask:{}:scheduled", subtask.id))
            .expect("scheduled subtask item");
        assert!(scheduled_subtask.is_all_day);
        assert_eq!(scheduled_subtask.date, "2026-07-23");
        assert_eq!(scheduled_subtask.end_date.as_deref(), Some("2026-07-25"));
        assert_eq!(
            scheduled_subtask.parent_title.as_deref(),
            Some("期間付きタスク")
        );

        let tree = database
            .list_tasks_with_subtasks(20)
            .expect("task tree after schedule");
        assert!(tree[0].subtasks[0].recurrence_rule.is_some());
        let notification_count: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM notification_rules", [], |row| {
                        row.get(0)
                    })
                    .map_err(|error| format!("notification count: {error}"))
            })
            .expect("notification count");
        assert_eq!(
            notification_count, 1,
            "planned-start notification is unchanged"
        );
        let notification_after_move: (String, String) = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT kind, notify_at FROM notification_rules LIMIT 1",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|error| format!("notification after move: {error}"))
            })
            .expect("notification after move");
        assert_eq!(notification_after_move, notification_before_move);

        let invalid_move = usecases::move_scheduled_work_item(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id.clone(),
            },
            usecases::WorkScheduleMoveDraft {
                start_date: "2026-07-22".to_string(),
                start_time: Some("09:10".to_string()),
            },
        );
        assert!(invalid_move.expect_err("invalid move").contains("15分"));

        let invalid = usecases::resize_scheduled_work_item(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id.clone(),
            },
            usecases::WorkScheduleDraft {
                start_date: "2026-07-20".to_string(),
                start_time: Some("11:00".to_string()),
                end_date: "2026-07-20".to_string(),
                end_time: Some("10:00".to_string()),
                is_all_day: false,
            },
        );
        assert!(invalid.expect_err("invalid resize").contains("後"));
        let saved_end_time: String = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT scheduled_end_time FROM tasks WHERE id = ?1",
                        params![task.id.as_str()],
                        |row| row.get(0),
                    )
                    .map_err(|error| format!("saved schedule: {error}"))
            })
            .expect("saved schedule");
        assert_eq!(saved_end_time, "21:30");
    }

    #[test]
    fn moving_work_item_without_schedule_is_rejected() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-18T00:00:00Z",
        };
        let task =
            usecases::create_task(&database, &clock, draft("予定なしタスク")).expect("create task");

        let result = usecases::move_scheduled_work_item(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
            usecases::WorkScheduleMoveDraft {
                start_date: "2026-07-21".to_string(),
                start_time: Some("09:00".to_string()),
            },
        );

        assert!(result.expect_err("schedule required").contains("予定期間"));
    }

    #[test]
    fn scheduled_ranges_are_included_in_json_and_csv_exports() {
        let database = in_memory_database();
        let export_root = temp_dir("tasktimer-schedule-export");
        fs::create_dir_all(&export_root).expect("export root");
        let clock = FixedClock {
            now: "2026-07-18T00:00:00Z",
        };
        usecases::create_scheduled_task(
            &database,
            &clock,
            draft("エクスポート予定"),
            usecases::WorkScheduleDraft {
                start_date: "2026-07-20".to_string(),
                start_time: Some("09:00".to_string()),
                end_date: "2026-07-20".to_string(),
                end_time: Some("10:30".to_string()),
                is_all_day: false,
            },
        )
        .expect("create scheduled task");

        let json_export = usecases::create_json_export(
            &database,
            &clock,
            usecases::DataExportCreateDraft {
                destination_dir: export_root.to_string_lossy().to_string(),
            },
        )
        .expect("json export");
        let json: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&json_export.export_path).expect("read json export"),
        )
        .expect("parse json export");
        assert_eq!(
            json["manifest"]["formatVersion"],
            DATA_EXPORT_FORMAT_VERSION
        );
        assert_eq!(json["tasks"][0]["scheduled_start_date"], "2026-07-20");
        assert_eq!(json["tasks"][0]["scheduled_end_time"], "10:30");

        let csv_export = usecases::create_csv_export(
            &database,
            &clock,
            usecases::DataExportCreateDraft {
                destination_dir: export_root.to_string_lossy().to_string(),
            },
        )
        .expect("csv export");
        let tasks_csv = fs::read_to_string(Path::new(&csv_export.export_path).join("tasks.csv"))
            .expect("read tasks csv");
        assert!(tasks_csv
            .lines()
            .next()
            .expect("csv header")
            .contains("scheduled_start_date"));
        assert!(tasks_csv.contains("2026-07-20,09:00,2026-07-20,10:30,false"));

        fs::remove_dir_all(export_root).expect("cleanup export");
    }

    #[test]
    fn local_search_finds_titles_memos_tags_and_subtasks_without_wildcard_expansion() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-20T00:00:00Z",
        };
        let task = usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "設計レビュー".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-21".to_string()),
                due_time: Some("10:00".to_string()),
                memo: Some("進捗 50%_確認".to_string()),
            },
        )
        .expect("create searchable task");
        let tag = usecases::create_tag(
            &database,
            &clock,
            usecases::TagDraft {
                name: "重要案件".to_string(),
            },
        )
        .expect("create tag");
        usecases::attach_tag_to_task(&database, &clock, task.id.clone(), tag.id)
            .expect("attach tag");
        let subtask = usecases::create_subtask(
            &database,
            &clock,
            task.id.clone(),
            usecases::WorkItemDraft {
                list_id: None,
                title: "検索対象サブ".to_string(),
                planned_start_date: None,
                due_date: None,
                due_time: None,
                memo: Some("子のメモ".to_string()),
            },
        )
        .expect("create subtask");
        let archived = usecases::create_task(&database, &clock, draft("検索対象アーカイブ"))
            .expect("create archived task");
        usecases::archive_task(&database, &clock, archived.id).expect("archive task");

        let title_results = usecases::search_work_items(
            &database,
            usecases::WorkItemSearchDraft {
                query: "設計レビュー".to_string(),
                limit: 50,
            },
        )
        .expect("search title");
        assert_eq!(title_results.len(), 1);
        assert_eq!(title_results[0].task_id, task.id);
        assert_eq!(title_results[0].tags[0].name, "重要案件");

        let tag_results = usecases::search_work_items(
            &database,
            usecases::WorkItemSearchDraft {
                query: "重要案件".to_string(),
                limit: 50,
            },
        )
        .expect("search tag");
        assert_eq!(tag_results.len(), 1);
        assert_eq!(tag_results[0].target.target_type, WorkTargetType::Task);

        let subtask_results = usecases::search_work_items(
            &database,
            usecases::WorkItemSearchDraft {
                query: "検索対象".to_string(),
                limit: 50,
            },
        )
        .expect("search subtask");
        assert_eq!(subtask_results.len(), 1);
        assert_eq!(subtask_results[0].target.id, subtask.id);
        assert_eq!(
            subtask_results[0].parent_title.as_deref(),
            Some("設計レビュー")
        );

        for literal in ["%", "_"] {
            let results = usecases::search_work_items(
                &database,
                usecases::WorkItemSearchDraft {
                    query: literal.to_string(),
                    limit: 50,
                },
            )
            .expect("search literal wildcard");
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].task_id, task.id);
        }

        let detail = usecases::get_task_detail(&database, task.id).expect("task detail");
        assert_eq!(detail.subtasks.len(), 1);

        let blank = usecases::search_work_items(
            &database,
            usecases::WorkItemSearchDraft {
                query: "   ".to_string(),
                limit: 50,
            },
        )
        .expect("blank search");
        assert!(blank.is_empty());
        let too_long = usecases::search_work_items(
            &database,
            usecases::WorkItemSearchDraft {
                query: "a".repeat(121),
                limit: 50,
            },
        );
        assert!(too_long.expect_err("long query").contains("120文字"));
    }

    #[test]
    fn scoped_calendar_filters_by_list_today_and_favorite() {
        let database = in_memory_database();
        let clock = FixedClock {
            now: "2026-07-20T00:00:00Z",
        };
        let custom_list = usecases::create_task_list(
            &database,
            &clock,
            usecases::TaskListDraft {
                name: "仕事".to_string(),
                color_token: Some("blue".to_string()),
            },
        )
        .expect("create list");
        let custom_task = usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: Some(custom_list.id.clone()),
                title: "仕事の期限".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-20".to_string()),
                due_time: None,
                memo: None,
            },
        )
        .expect("create custom task");
        let planned_task = usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "今日から開始".to_string(),
                planned_start_date: Some("2026-07-20".to_string()),
                due_date: None,
                due_time: None,
                memo: None,
            },
        )
        .expect("create planned task");
        let favorite_task = usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                list_id: None,
                title: "お気に入り期限".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-21".to_string()),
                due_time: None,
                memo: None,
            },
        )
        .expect("create favorite task");
        usecases::toggle_task_favorite(&database, &clock, favorite_task.id.clone(), true)
            .expect("favorite task");

        let list_items = usecases::list_calendar_items(
            &database,
            usecases::CalendarItemsDraft {
                start_date: "2026-07-20".to_string(),
                end_date: "2026-07-21".to_string(),
                scope: usecases::TaskPageScopeDraft::List {
                    list_id: custom_list.id,
                },
                today_date: "2026-07-20".to_string(),
            },
        )
        .expect("list calendar");
        assert_eq!(list_items.len(), 1);
        assert_eq!(list_items[0].target.id, custom_task.id);

        let today_items = usecases::list_calendar_items(
            &database,
            usecases::CalendarItemsDraft {
                start_date: "2026-07-20".to_string(),
                end_date: "2026-07-21".to_string(),
                scope: usecases::TaskPageScopeDraft::Today,
                today_date: "2026-07-20".to_string(),
            },
        )
        .expect("today calendar");
        assert_eq!(today_items.len(), 2);
        assert_eq!(
            today_items
                .iter()
                .map(|item| item.target.id.as_str())
                .collect::<HashSet<_>>(),
            HashSet::from([custom_task.id.as_str(), planned_task.id.as_str()])
        );

        let favorite_items = usecases::list_calendar_items(
            &database,
            usecases::CalendarItemsDraft {
                start_date: "2026-07-20".to_string(),
                end_date: "2026-07-21".to_string(),
                scope: usecases::TaskPageScopeDraft::Favorites,
                today_date: "2026-07-20".to_string(),
            },
        )
        .expect("favorite calendar");
        assert_eq!(favorite_items.len(), 1);
        assert_eq!(favorite_items[0].target.id, favorite_task.id);
    }
}
