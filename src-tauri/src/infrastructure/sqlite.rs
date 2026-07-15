#![allow(dead_code)]

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration as StdDuration,
};

use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use time::{
    format_description::well_known::Rfc3339, macros::format_description, Date, Duration,
    OffsetDateTime,
};
use uuid::Uuid;

use crate::{
    application::repositories::{
        target_ref, ActiveTimer, CalendarMarker, CalendarRepository, DataExportCreate,
        DataExportManifestRecord, DataExportRecord, DataExportRepository,
        NotificationCommandRepository, NotificationDeliveryAttemptRecord,
        NotificationHistoryRepository, NotificationJob, NotificationPreferenceRepository,
        RecurrenceRuleInput, RecurrenceRuleRecord, RepositoryResult, SqliteBackupCreate,
        SqliteBackupManifestRecord, SqliteBackupRecord, SqliteBackupRepository,
        SqliteBackupRestore, SqliteRestoreRecord, SubtaskRecord, TaskListCommandRepository,
        TaskListCreate, TaskListRecord, TaskListUpdate, TaskReadRepository, TaskRecord,
        TaskRowRecord, TaskTimerCommandRepository, TaskWithSubtasksRecord, TimerRepository,
        WeekCalendarItem, WorkItemCreate, WorkItemUpdate, CURRENT_SQLITE_BACKUP_SCHEMA_VERSION,
    },
    domain::{
        notification::{
            NotificationDeliveryResult, NotificationDisplayMode, NotificationKind,
            NotificationRegistrationStatus,
        },
        recurrence::RecurrenceFrequency,
        task::{
            assert_completable, assert_timer_startable, WorkStatus, DEFAULT_TASK_LIST_ID,
            DEFAULT_TASK_LIST_NAME,
        },
        timer::{WorkTargetRef, WorkTargetType},
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
const DATA_EXPORT_FORMAT_VERSION: i64 = 1;
const DATA_EXPORT_COMPATIBILITY: &str = "viewing-and-migration-aid-not-restore";
const CSV_EXPORT_MANIFEST_FILE: &str = "export-manifest.json";
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
    tasks: Vec<ExportTaskRow>,
    subtasks: Vec<ExportSubtaskRow>,
    timer_sessions: Vec<ExportTimerSessionRow>,
    timer_pauses: Vec<ExportTimerPauseRow>,
    notification_rules: Vec<ExportNotificationRuleRow>,
    recurrence_rules: Vec<ExportRecurrenceRuleRow>,
}

#[derive(Debug, Clone)]
struct ExportDataset {
    task_lists: Vec<ExportTaskListRow>,
    tasks: Vec<ExportTaskRow>,
    subtasks: Vec<ExportSubtaskRow>,
    timer_sessions: Vec<ExportTimerSessionRow>,
    timer_pauses: Vec<ExportTimerPauseRow>,
    notification_rules: Vec<ExportNotificationRuleRow>,
    recurrence_rules: Vec<ExportRecurrenceRuleRow>,
}

#[derive(Debug, Clone, Serialize)]
struct ExportTaskListRow {
    id: String,
    name: String,
    sort_order: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExportTaskRow {
    id: String,
    list_id: String,
    title: String,
    status: String,
    is_favorite: bool,
    planned_start_date: Option<String>,
    due_date: Option<String>,
    due_time: Option<String>,
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
    created_at: String,
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
        let start = parse_date(start_date, "開始日")?;
        let end = parse_date(end_date, "終了日")?;
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
            collect_task_calendar_items(connection, &start_text, &end_text, &mut items)?;
            collect_subtask_calendar_items(connection, &start_text, &end_text, &mut items)?;
            collect_active_timer_calendar_item(connection, &start_text, &end_text, &mut items)?;
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
}

impl TaskReadRepository for SqliteDatabase {
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
            tasks: dataset.tasks,
            subtasks: dataset.subtasks,
            timer_sessions: dataset.timer_sessions,
            timer_pauses: dataset.timer_pauses,
            notification_rules: dataset.notification_rules,
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

    fn start_timer(&self, target: WorkTargetRef, now: String) -> RepositoryResult<ActiveTimer> {
        self.with_transaction(|transaction| {
            let status = find_target_status(transaction, &target)?.ok_or_else(|| {
                "タイマー開始対象のタスクまたはサブタスクが存在しません".to_string()
            })?;
            assert_timer_startable(&status)?;
            ensure_no_active_timer(transaction)?;

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
                        elapsed_seconds = ?2
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
                    SET status = 'todo',
                        completed_at = NULL,
                        updated_at = ?1
                    WHERE id = ?2
                      AND deleted_at IS NULL
                    ",
                    params![now, task_id],
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

            transaction
                .execute(
                    "
                    UPDATE tasks
                    SET status = 'archived',
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
                          WHEN completed_at IS NULL THEN 'todo'
                          ELSE 'done'
                        END,
                        updated_at = ?1
                    WHERE id = ?2
                      AND deleted_at IS NULL
                    ",
                    params![now, task_id],
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

impl NotificationHistoryRepository for SqliteDatabase {
    fn list_notification_failure_history(
        &self,
        limit: i64,
    ) -> RepositoryResult<Vec<NotificationDeliveryAttemptRecord>> {
        let limit = limit.clamp(1, 100);
        self.with_connection(|connection| select_notification_failure_history(connection, limit))
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

fn insert_task(
    transaction: &Transaction<'_>,
    input: WorkItemCreate,
) -> RepositoryResult<TaskRecord> {
    let id = Uuid::new_v4().to_string();
    ensure_default_task_list(transaction, &input.now)?;
    ensure_task_list_exists(transaction, &input.list_id)?;
    let sort_order = next_task_sort_order(transaction, &input.list_id)?;
    let planned_start_date = input.planned_start_date.clone();
    let due_date = input.due_date.clone();
    let due_time = input.due_time.clone();
    let now = input.now.clone();
    transaction
        .execute(
            "
            INSERT INTO tasks (
              id, list_id, title, status, is_favorite,
              planned_start_date, due_date, due_time, timer_target_seconds, memo,
              sort_order, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, 'todo', 0, ?4, ?5, ?6, NULL, ?7, ?8, ?9, ?9)
            ",
            params![
                id,
                input.list_id,
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
              id, name, sort_order, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?4)
            ",
            params![id, input.name, sort_order, input.now],
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
                memo = ?8,
                updated_at = ?9
            WHERE id = ?10
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
    ensure_custom_task_list(transaction, list_id)?;
    ensure_unique_task_list_name(transaction, &input.name, Some(list_id))?;

    let updated = transaction
        .execute(
            "
            UPDATE task_lists
            SET name = ?1,
                updated_at = ?2
            WHERE id = ?3
              AND deleted_at IS NULL
            ",
            params![input.name, input.now, list_id],
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
    )
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
            .map(|_| ())
            .map_err(|error| format!("通知ルールを更新できません: {error}"))
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
                Uuid::new_v4().to_string(),
                target.target_type.as_str(),
                target.id.as_str(),
                kind.as_str(),
                notify_at,
                now
            ],
        )
        .map(|_| ())
        .map_err(|error| format!("通知ルールを作成できません: {error}"))
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

fn select_task_list(connection: &Connection, limit: i64) -> RepositoryResult<Vec<TaskRecord>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, list_id, title, status, is_favorite,
                   planned_start_date, due_date, due_time, timer_target_seconds, memo,
                   sort_order, completed_at, deleted_at, created_at, updated_at,
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

    rows.map(|row| row.map_err(|error| format!("タスク行を読めません: {error}")))
        .collect()
}

fn select_task_lists(connection: &Connection) -> RepositoryResult<Vec<TaskListRecord>> {
    let mut statement = connection
        .prepare(
            "
            SELECT task_lists.id,
                   task_lists.name,
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
                sort_order: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                task_count: row.get(5)?,
                active_task_count: row.get(6)?,
                completed_task_count: row.get(7)?,
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
                     task_lists.sort_order,
                     task_lists.created_at,
                     task_lists.updated_at
            ",
            params![id],
            |row| {
                Ok(TaskListRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    sort_order: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    task_count: row.get(5)?,
                    active_task_count: row.get(6)?,
                    completed_task_count: row.get(7)?,
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

    rows.map(|row| row.map_err(|error| format!("タスク行Read Modelを読めません: {error}")))
        .collect()
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
    soft_delete_notification_rules_for_task_graph(transaction, task_id, now)?;
    soft_delete_recurrence_rules_for_task_graph(transaction, task_id, now)
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

fn soft_delete_notification_rules_for_task_graph(
    transaction: &Transaction<'_>,
    task_id: &str,
    now: &str,
) -> RepositoryResult<()> {
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
    let sql = match target.target_type {
        WorkTargetType::Task => {
            "
            UPDATE tasks
            SET status = 'in_progress',
                updated_at = ?1
            WHERE id = ?2
              AND deleted_at IS NULL
              AND status <> 'in_progress'
            "
        }
        WorkTargetType::Subtask => {
            "
            UPDATE subtasks
            SET status = 'in_progress',
                updated_at = ?1
            WHERE id = ?2
              AND deleted_at IS NULL
              AND status <> 'in_progress'
            "
        }
    };

    transaction
        .execute(sql, params![now, target.id])
        .map(|_| ())
        .map_err(|error| format!("作業対象を進行中に更新できません: {error}"))
}

fn select_task_by_id(connection: &Connection, id: &str) -> RepositoryResult<TaskRecord> {
    connection
        .query_row(
            "
            SELECT id, list_id, title, status, is_favorite,
                   planned_start_date, due_date, due_time, timer_target_seconds, memo,
                   sort_order, completed_at, deleted_at, created_at, updated_at,
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
        .map_err(|error| format!("タスクを取得できません: {error}"))
}

fn select_existing_task_by_id(connection: &Connection, id: &str) -> RepositoryResult<TaskRecord> {
    connection
        .query_row(
            "
            SELECT id, list_id, title, status, is_favorite,
                   planned_start_date, due_date, due_time, timer_target_seconds, memo,
                   sort_order, completed_at, deleted_at, created_at, updated_at,
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
        .map_err(|error| format!("タスクを取得できません: {error}"))
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
        recurrence_rule: map_optional_recurrence_rule(row, 15)?,
        memo: row.get(9)?,
        sort_order: row.get(10)?,
        completed_at: row.get(11)?,
        deleted_at: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
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
    let active_target_type_text: Option<String> = row.get(15)?;
    let active_target_id: Option<String> = row.get(16)?;
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
        title: row.get(2)?,
        status: WorkStatus::from_db(&row.get::<_, String>(3)?).map_err(db_value_error)?,
        is_favorite: row.get::<_, i64>(4)? != 0,
        planned_start_date: row.get(5)?,
        due_date: row.get(6)?,
        due_time: row.get(7)?,
        timer_target_seconds: row.get(8)?,
        sort_order: row.get(9)?,
        completed_at: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
        subtask_total_count: row.get(13)?,
        completed_subtask_count: row.get(14)?,
        active_timer_target,
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
                   open_pause.paused_at
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
                   open_pause.paused_at
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
                   open_pause.paused_at
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
    Ok(ActiveTimer {
        id: row.get(0)?,
        target: target_ref(target_type, row.get(2)?),
        started_at: row.get(3)?,
        stopped_at: row.get(4)?,
        elapsed_seconds: row.get(5)?,
        paused_at: row.get(8)?,
        deleted_at: row.get(6)?,
        created_at: row.get(7)?,
    })
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
    let started = OffsetDateTime::parse(started_at, &Rfc3339)
        .map_err(|error| format!("一時停止開始時刻の形式が不正です: {error}"))?;
    let stopped = OffsetDateTime::parse(stopped_at, &Rfc3339)
        .map_err(|error| format!("一時停止終了時刻の形式が不正です: {error}"))?;
    if stopped < started {
        return Ok(0);
    }

    Ok((stopped - started).whole_seconds())
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
    run_due_time_migration(connection)?;
    run_ui_read_model_migration(connection)?;
    run_notification_preference_migration(connection)?;
    run_notification_delivery_attempt_migration(connection)
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

fn run_notification_preference_migration(connection: &Connection) -> RepositoryResult<()> {
    ensure_column(
        connection,
        "notification_preferences",
        "notifications_enabled",
        "ALTER TABLE notification_preferences ADD COLUMN notifications_enabled INTEGER NOT NULL DEFAULT 1 CHECK (notifications_enabled IN (0, 1))",
    )
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

fn run_ui_read_model_migration(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS task_lists (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL CHECK (length(trim(name)) > 0),
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
              id, name, sort_order, created_at, updated_at
            )
            VALUES (?1, ?2, 0, ?3, ?3)
            ",
            params![DEFAULT_TASK_LIST_ID, DEFAULT_TASK_LIST_NAME, now],
        )
        .map(|_| ())
        .map_err(|error| format!("初期タスクリストを保存できません: {error}"))
}

fn seed_default_ui_preferences(connection: &Connection) -> RepositoryResult<()> {
    connection
        .execute(
            "
            INSERT OR IGNORE INTO ui_preferences (key, value, updated_at)
            VALUES ('left_pane_open', 'true', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                   ('last_view', 'tasks', strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                   ('last_task_list_id', ?1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            ",
            params![DEFAULT_TASK_LIST_ID],
        )
        .map(|_| ())
        .map_err(|error| format!("UI設定の初期化に失敗しました: {error}"))
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
        &["id", "name", "sort_order", "created_at", "updated_at"],
        dataset
            .task_lists
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
        &export_dir.join("tasks.csv"),
        &[
            "id",
            "list_id",
            "title",
            "status",
            "is_favorite",
            "planned_start_date",
            "due_date",
            "due_time",
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
                    row.title.clone(),
                    row.status.clone(),
                    row.is_favorite.to_string(),
                    option_text(&row.planned_start_date),
                    option_text(&row.due_date),
                    option_text(&row.due_time),
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
        "tasks.csv",
        "subtasks.csv",
        "timer_sessions.csv",
        "timer_pauses.csv",
        "notification_rules.csv",
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
        tasks: select_export_tasks(connection)?,
        subtasks: select_export_subtasks(connection)?,
        timer_sessions: select_export_timer_sessions(connection)?,
        timer_pauses: select_export_timer_pauses(connection)?,
        notification_rules: select_export_notification_rules(connection)?,
        recurrence_rules: select_export_recurrence_rules(connection)?,
    })
}

fn select_export_task_lists(connection: &Connection) -> RepositoryResult<Vec<ExportTaskListRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, name, sort_order, created_at, updated_at
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
                sort_order: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|error| format!("エクスポート用リストを取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用リストを読めません")
}

fn select_export_tasks(connection: &Connection) -> RepositoryResult<Vec<ExportTaskRow>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, list_id, title, status, is_favorite, planned_start_date,
                   due_date, due_time, timer_target_seconds, memo, sort_order,
                   completed_at, created_at, updated_at
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
                title: row.get(2)?,
                status: row.get(3)?,
                is_favorite: row.get::<_, i64>(4)? != 0,
                planned_start_date: row.get(5)?,
                due_date: row.get(6)?,
                due_time: row.get(7)?,
                timer_target_seconds: row.get(8)?,
                memo: row.get(9)?,
                sort_order: row.get(10)?,
                completed_at: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
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
                   due_time, timer_target_seconds, memo, sort_order,
                   completed_at, created_at, updated_at
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
                timer_target_seconds: row.get(7)?,
                memo: row.get(8)?,
                sort_order: row.get(9)?,
                completed_at: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
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
                   elapsed_seconds, created_at
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
                created_at: row.get(6)?,
            })
        })
        .map_err(|error| format!("エクスポート用タイマー履歴を取得できません: {error}"))?;
    collect_export_rows(rows, "エクスポート用タイマー履歴を読めません")
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
    items: &mut Vec<WeekCalendarItem>,
) -> RepositoryResult<()> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, title, planned_start_date, due_date, due_time, status
            FROM tasks
            WHERE deleted_at IS NULL
              AND status <> 'archived'
              AND (
                planned_start_date BETWEEN ?1 AND ?2
                OR due_date BETWEEN ?1 AND ?2
              )
            ",
        )
        .map_err(|error| format!("タスクカレンダークエリを準備できません: {error}"))?;

    let rows = statement
        .query_map(params![start_date, end_date], |row| {
            Ok(CalendarSourceRow {
                target_type: WorkTargetType::Task,
                id: row.get(0)?,
                title: row.get(1)?,
                planned_start_date: row.get(2)?,
                due_date: row.get(3)?,
                due_time: row.get(4)?,
                status: WorkStatus::from_db(&row.get::<_, String>(5)?).map_err(db_value_error)?,
                parent_title: None,
            })
        })
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
                   subtasks.status,
                   tasks.title AS parent_title
            FROM subtasks
            INNER JOIN tasks
              ON tasks.id = subtasks.task_id
             AND tasks.deleted_at IS NULL
             AND tasks.status <> 'archived'
            WHERE subtasks.deleted_at IS NULL
              AND subtasks.status <> 'archived'
              AND (
                subtasks.planned_start_date BETWEEN ?1 AND ?2
                OR subtasks.due_date BETWEEN ?1 AND ?2
              )
            ",
        )
        .map_err(|error| format!("サブタスクカレンダークエリを準備できません: {error}"))?;

    let rows = statement
        .query_map(params![start_date, end_date], |row| {
            Ok(CalendarSourceRow {
                target_type: WorkTargetType::Subtask,
                id: row.get(0)?,
                title: row.get(1)?,
                planned_start_date: row.get(2)?,
                due_date: row.get(3)?,
                due_time: row.get(4)?,
                status: WorkStatus::from_db(&row.get::<_, String>(5)?).map_err(db_value_error)?,
                parent_title: row.get(6)?,
            })
        })
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
                   parent_tasks.title AS parent_title
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
            LIMIT 1
            ",
            params![start_date, end_date],
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
                    marker: CalendarMarker::ActiveTimer,
                    status: WorkStatus::from_db(&row.get::<_, String>(4)?)
                        .map_err(db_value_error)?,
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
    status: WorkStatus,
    parent_title: Option<String>,
}

fn push_calendar_items(row: CalendarSourceRow, items: &mut Vec<WeekCalendarItem>) {
    let target_type_text = row.target_type.as_str().to_string();
    if let Some(date) = row.planned_start_date {
        items.push(WeekCalendarItem {
            id: format!("{target_type_text}:{}:planned_start", row.id),
            target: target_ref(row.target_type.clone(), row.id.clone()),
            title: row.title.clone(),
            parent_title: row.parent_title.clone(),
            date,
            time: None,
            marker: CalendarMarker::PlannedStart,
            status: row.status.clone(),
        });
    }

    if let Some(date) = row.due_date {
        items.push(WeekCalendarItem {
            id: format!("{target_type_text}:{}:due", row.id),
            target: target_ref(row.target_type, row.id),
            title: row.title,
            parent_title: row.parent_title,
            date,
            time: row.due_time,
            marker: CalendarMarker::Due,
            status: row.status,
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
                NotificationHistoryRepository, NotificationPreferenceRepository, TaskReadRepository,
            },
            usecases,
        },
        domain::{
            notification::{NotificationDeliveryResult, NotificationDisplayMode, NotificationKind},
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
    fn migration_backfills_ui_read_model_defaults_for_existing_database() {
        let connection = Connection::open_in_memory().expect("in-memory database");
        configure_connection(&connection).expect("configure");
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

        let (list_id, is_favorite, timer_target_seconds): (String, i64, Option<i64>) = connection
            .query_row(
                "
                SELECT list_id, is_favorite, timer_target_seconds
                FROM tasks
                WHERE id = 'legacy-task'
                ",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("migrated task");
        let task_list_name: String = connection
            .query_row(
                "SELECT name FROM task_lists WHERE id = 'default'",
                [],
                |row| row.get(0),
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

        assert_eq!(list_id, DEFAULT_TASK_LIST_ID);
        assert_eq!(is_favorite, 0);
        assert_eq!(timer_target_seconds, None);
        assert_eq!(task_list_name, DEFAULT_TASK_LIST_NAME);
        assert_eq!(ui_preference_count, 3);
        assert_eq!(timer_recurrence_table_count, 2);
        assert!(
            column_exists(&connection, "subtasks", "timer_target_seconds")
                .expect("subtask timer target column")
        );
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
                id: first_task.id,
            },
        )
        .expect("first timer");
        let first_task_after_start = database
            .with_connection(|connection| select_task_by_id(connection, &first_task_id))
            .expect("first task after start");
        assert_eq!(first_task_after_start.status, WorkStatus::InProgress);

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
        usecases::create_subtask(
            &database,
            &clock,
            task.id,
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
        assert_eq!(export.manifest.compatibility, DATA_EXPORT_COMPATIBILITY);
        assert!(export.manifest.contains_personal_data);

        let json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&export.export_path).expect("read json"))
                .expect("parse json");
        assert_eq!(json["manifest"]["format"], JSON_EXPORT_FORMAT);
        assert_eq!(json["manifest"]["containsPersonalData"], true);
        assert_eq!(json["tasks"][0]["title"], "=SUM(1,2)");
        assert_eq!(json["tasks"][0]["memo"], "1行目,2行目\n\"引用\"");
        assert_eq!(json["subtasks"][0]["title"], "サブタスク");

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
        usecases::start_timer(
            &database,
            &start_clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id,
            },
        )
        .expect("start timer");
        usecases::stop_active_timer(&database, &stop_clock).expect("stop timer");

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
            .any(|path| path.ends_with("timer_sessions.csv")));

        let tasks_csv = fs::read_to_string(PathBuf::from(&export.export_path).join("tasks.csv"))
            .expect("read tasks csv");
        assert!(tasks_csv.starts_with("id,list_id,title,status,is_favorite"));
        assert!(tasks_csv.contains("\"'=SUM(1,2)\""));
        assert!(tasks_csv.contains("\"カンマ, 改行\n\"\"引用符\"\"\""));

        fs::remove_dir_all(data_dir).expect("cleanup data");
        fs::remove_dir_all(export_root).expect("cleanup export");
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
            },
        )
        .expect("create task list");
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
            },
        )
        .expect("create task list");

        let default_update = usecases::update_task_list(
            &database,
            &clock,
            DEFAULT_TASK_LIST_ID.to_string(),
            usecases::TaskListDraft {
                name: "変更不可".to_string(),
            },
        );
        let duplicate = usecases::create_task_list(
            &database,
            &clock,
            usecases::TaskListDraft {
                name: "重複確認".to_string(),
            },
        );
        let renamed = usecases::update_task_list(
            &database,
            &clock,
            list.id,
            usecases::TaskListDraft {
                name: "重複確認 2".to_string(),
            },
        )
        .expect("rename task list");

        assert!(default_update
            .expect_err("default list update")
            .contains("初期タスクリスト"));
        assert!(duplicate
            .expect_err("duplicate list name")
            .contains("すでに存在"));
        assert_eq!(renamed.name, "重複確認 2");
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
        let parent =
            usecases::create_task(&database, &create_clock, draft("親タスク")).expect("task");
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
        assert_eq!(active_timer.target.target_type, WorkTargetType::Subtask);
        assert_eq!(active_timer.parent_title.as_deref(), Some("親タスク"));
        assert_eq!(active_timer.date, "2026-07-20");
        assert_eq!(active_timer.time.as_deref(), Some("10:15"));
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
                timer_target_seconds: Some(1_800),
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
        assert_eq!(updated.timer_target_seconds, Some(1_800));
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
    }
}
