#![allow(dead_code)]

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration as StdDuration,
};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use tauri::{AppHandle, Manager};
use time::{
    format_description::well_known::Rfc3339, macros::format_description, Date, Duration,
    OffsetDateTime,
};
use uuid::Uuid;

use crate::{
    application::repositories::{
        target_ref, ActiveTimer, CalendarMarker, CalendarRepository, NotificationCommandRepository,
        NotificationJob, NotificationPreferenceRepository, RepositoryResult, SubtaskRecord,
        TaskListRecord, TaskReadRepository, TaskRecord, TaskRowRecord, TaskTimerCommandRepository,
        TaskWithSubtasksRecord, TimerRepository, WeekCalendarItem, WorkItemCreate, WorkItemUpdate,
    },
    domain::{
        notification::{NotificationDisplayMode, NotificationKind, NotificationRegistrationStatus},
        task::{assert_completable, assert_timer_startable, WorkStatus},
        timer::{WorkTargetRef, WorkTargetType},
    },
};

pub const INITIAL_SCHEMA: &str = include_str!("../../migrations/0001_initial.sql");

const DEFAULT_TASK_LIST_ID: &str = "default";
const DEFAULT_TASK_LIST_NAME: &str = "タスク";
const DATE_FORMAT: &[time::format_description::FormatItem<'_>] =
    format_description!("[year]-[month]-[day]");

pub struct SqliteDatabase {
    path: PathBuf,
    connection: Mutex<Connection>,
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
    fn list_week_calendar_items(
        &self,
        week_start_date: &str,
    ) -> RepositoryResult<Vec<WeekCalendarItem>> {
        let start = parse_date(week_start_date)?;
        let end = start + Duration::days(6);
        let start_text = format_date(start)?;
        let end_text = format_date(end)?;

        self.with_connection(|connection| {
            let mut items = Vec::new();
            collect_task_calendar_items(connection, &start_text, &end_text, &mut items)?;
            collect_subtask_calendar_items(connection, &start_text, &end_text, &mut items)?;
            collect_active_timer_calendar_item(connection, &mut items)?;
            items.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.title.cmp(&b.title)));
            Ok(items)
        })
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
        let list_id = normalize_list_id(list_id);

        self.with_connection(|connection| select_task_rows(connection, &list_id, limit))
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

    fn stop_active_timer(&self, now: String) -> RepositoryResult<ActiveTimer> {
        self.with_transaction(|transaction| {
            let active_timer = select_active_timer(transaction)?
                .ok_or_else(|| "開始中のタイマーがありません".to_string())?;
            let (stopped_at, elapsed_seconds) =
                calculate_stop_values(&active_timer.started_at, &now)?;

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

    fn list_due_notification_jobs(
        &self,
        now: &str,
        limit: i64,
    ) -> RepositoryResult<Vec<NotificationJob>> {
        let limit = limit.clamp(1, 100);
        self.with_connection(|connection| select_due_notification_jobs(connection, now, limit))
    }

    fn mark_notification_registered(&self, id: &str, now: &str) -> RepositoryResult<()> {
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
                    params![now, id],
                )
                .map(|_| ())
                .map_err(|error| format!("通知登録成功状態を保存できません: {error}"))
        })
    }

    fn mark_notification_failed(&self, id: &str, error: &str, now: &str) -> RepositoryResult<()> {
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
                    params![truncate_error(error), now, id],
                )
                .map(|_| ())
                .map_err(|error| format!("通知登録失敗状態を保存できません: {error}"))
        })
    }
}

fn insert_task(
    transaction: &Transaction<'_>,
    input: WorkItemCreate,
) -> RepositoryResult<TaskRecord> {
    let id = Uuid::new_v4().to_string();
    ensure_default_task_list(transaction, &input.now)?;
    let sort_order = next_task_sort_order(transaction, DEFAULT_TASK_LIST_ID)?;
    let planned_start_date = input.planned_start_date.clone();
    let due_date = input.due_date.clone();
    let now = input.now.clone();
    transaction
        .execute(
            "
            INSERT INTO tasks (
              id, list_id, title, status, is_favorite,
              planned_start_date, due_date, timer_target_seconds, memo,
              sort_order, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, 'todo', 0, ?4, ?5, NULL, ?6, ?7, ?8, ?8)
            ",
            params![
                id,
                DEFAULT_TASK_LIST_ID,
                input.title,
                input.planned_start_date,
                input.due_date,
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
        &now,
    )?;

    select_task_by_id(transaction, &id)
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
    let now = input.now.clone();
    transaction
        .execute(
            "
            INSERT INTO subtasks (
              id, task_id, title, status, planned_start_date, due_date,
              timer_target_seconds, memo, sort_order, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, 'todo', ?4, ?5, NULL, ?6, ?7, ?8, ?8)
            ",
            params![
                id,
                task_id,
                input.title,
                input.planned_start_date,
                input.due_date,
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
    let planned_start_date = input.planned_start_date.clone();
    let due_date = input.due_date.clone();
    let now = input.now.clone();

    let updated = transaction
        .execute(
            "
            UPDATE tasks
            SET title = ?1,
                planned_start_date = ?2,
                due_date = ?3,
                timer_target_seconds = ?4,
                memo = ?5,
                updated_at = ?6
            WHERE id = ?7
              AND deleted_at IS NULL
            ",
            params![
                input.title,
                input.planned_start_date,
                input.due_date,
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
        &now,
    )?;

    select_existing_task_by_id(transaction, task_id)
}

fn update_subtask_detail(
    transaction: &Transaction<'_>,
    subtask_id: &str,
    input: WorkItemUpdate,
) -> RepositoryResult<SubtaskRecord> {
    ensure_subtask_exists(transaction, subtask_id)?;
    let planned_start_date = input.planned_start_date.clone();
    let due_date = input.due_date.clone();
    let now = input.now.clone();

    let updated = transaction
        .execute(
            "
            UPDATE subtasks
            SET title = ?1,
                planned_start_date = ?2,
                due_date = ?3,
                timer_target_seconds = ?4,
                memo = ?5,
                updated_at = ?6
            WHERE id = ?7
              AND deleted_at IS NULL
            ",
            params![
                input.title,
                input.planned_start_date,
                input.due_date,
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
        &now,
    )?;

    select_existing_subtask_by_id(transaction, subtask_id)
}

fn insert_notification_rules_for_target(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    planned_start_date: Option<&str>,
    due_date: Option<&str>,
    now: &str,
) -> RepositoryResult<()> {
    if let Some(date) = planned_start_date {
        insert_notification_rule(
            transaction,
            target,
            NotificationKind::PlannedStart,
            &notification_time_for_date(date),
            now,
        )?;
    }

    if let Some(date) = due_date {
        insert_notification_rule(
            transaction,
            target,
            NotificationKind::Due,
            &notification_time_for_date(date),
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
    now: &str,
) -> RepositoryResult<()> {
    sync_notification_rule_for_kind(
        transaction,
        target,
        NotificationKind::PlannedStart,
        planned_start_date,
        now,
    )?;
    sync_notification_rule_for_kind(transaction, target, NotificationKind::Due, due_date, now)
}

fn sync_notification_rule_for_kind(
    transaction: &Transaction<'_>,
    target: &WorkTargetRef,
    kind: NotificationKind,
    date: Option<&str>,
    now: &str,
) -> RepositoryResult<()> {
    let existing = select_active_notification_rule_for_kind(transaction, target, &kind)?;
    let Some(date) = date else {
        return disable_notification_rules_for_kind(transaction, target, &kind, now);
    };
    let notify_at = notification_time_for_date(date);

    if let Some(existing) = existing {
        disable_duplicate_notification_rules_for_kind(
            transaction,
            target,
            &kind,
            &existing.id,
            now,
        )?;
        if existing.notify_at == notify_at && existing.enabled {
            return Ok(());
        }

        transaction
            .execute(
                "
                UPDATE notification_rules
                SET notify_at = ?1,
                    enabled = 1,
                    registration_status = 'pending',
                    last_error = NULL,
                    updated_at = ?2
                WHERE id = ?3
                  AND deleted_at IS NULL
                ",
                params![notify_at, now, existing.id],
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

fn notification_time_for_date(date: &str) -> String {
    format!("{date}T00:00:00Z")
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
            LEFT JOIN subtasks
              ON notification_rules.target_type = 'subtask'
             AND notification_rules.target_id = subtasks.id
             AND subtasks.deleted_at IS NULL
            WHERE notification_rules.enabled = 1
              AND notification_rules.deleted_at IS NULL
              AND notification_rules.notify_at <= ?1
              AND notification_rules.registration_status IN ('pending', 'failed')
              AND COALESCE(tasks.id, subtasks.id) IS NOT NULL
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

fn truncate_error(error: &str) -> String {
    error.chars().take(500).collect()
}

fn select_task_list(connection: &Connection, limit: i64) -> RepositoryResult<Vec<TaskRecord>> {
    let mut statement = connection
        .prepare(
            "
            SELECT id, list_id, title, status, is_favorite,
                   planned_start_date, due_date, timer_target_seconds, memo,
                   sort_order, completed_at, deleted_at, created_at, updated_at
            FROM tasks
            WHERE deleted_at IS NULL
            ORDER BY sort_order ASC, created_at ASC
            LIMIT ?1
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

fn select_task_rows(
    connection: &Connection,
    list_id: &str,
    limit: i64,
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
              AND tasks.list_id = ?1
            GROUP BY tasks.id,
                     tasks.list_id,
                     tasks.title,
                     tasks.status,
                     tasks.is_favorite,
                     tasks.planned_start_date,
                     tasks.due_date,
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
            LIMIT ?2
            ",
        )
        .map_err(|error| format!("タスク行Read Modelクエリを準備できません: {error}"))?;

    let rows = statement
        .query_map(params![list_id, limit], map_task_read_model_row)
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
              ORDER BY sort_order ASC, created_at ASC
              LIMIT ?1
            )
            SELECT subtasks.id, subtasks.task_id, subtasks.title, subtasks.status,
                   subtasks.planned_start_date, subtasks.due_date,
                   subtasks.timer_target_seconds, subtasks.memo, subtasks.sort_order,
                   subtasks.completed_at, subtasks.deleted_at, subtasks.created_at,
                   subtasks.updated_at
            FROM subtasks
            INNER JOIN selected_tasks
              ON selected_tasks.id = subtasks.task_id
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
    soft_delete_notification_rules_for_task_graph(transaction, task_id, now)
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
        .map_err(|error| format!("サブタスクの通知ルールを削除できません: {error}"))
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
            "SELECT status FROM subtasks WHERE id = ?1 AND deleted_at IS NULL",
            &target.id,
        ),
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
                   planned_start_date, due_date, timer_target_seconds, memo,
                   sort_order, completed_at, deleted_at, created_at, updated_at
            FROM tasks
            WHERE id = ?1
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
                   planned_start_date, due_date, timer_target_seconds, memo,
                   sort_order, completed_at, deleted_at, created_at, updated_at
            FROM tasks
            WHERE id = ?1
              AND deleted_at IS NULL
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
            SELECT id, task_id, title, status, planned_start_date, due_date,
                   timer_target_seconds, memo, sort_order, completed_at, deleted_at,
                   created_at, updated_at
            FROM subtasks
            WHERE id = ?1
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
            SELECT id, task_id, title, status, planned_start_date, due_date,
                   timer_target_seconds, memo, sort_order, completed_at, deleted_at,
                   created_at, updated_at
            FROM subtasks
            WHERE id = ?1
              AND deleted_at IS NULL
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
        timer_target_seconds: row.get(7)?,
        memo: row.get(8)?,
        sort_order: row.get(9)?,
        completed_at: row.get(10)?,
        deleted_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
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
        timer_target_seconds: row.get(6)?,
        memo: row.get(7)?,
        sort_order: row.get(8)?,
        completed_at: row.get(9)?,
        deleted_at: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

fn map_task_read_model_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRowRecord> {
    let active_target_type_text: Option<String> = row.get(14)?;
    let active_target_id: Option<String> = row.get(15)?;
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
        timer_target_seconds: row.get(7)?,
        sort_order: row.get(8)?,
        completed_at: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
        subtask_total_count: row.get(12)?,
        completed_subtask_count: row.get(13)?,
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
                   timer_sessions.created_at
            FROM timer_sessions
            LEFT JOIN tasks
              ON timer_sessions.target_type = 'task'
             AND timer_sessions.target_id = tasks.id
             AND tasks.deleted_at IS NULL
            LEFT JOIN subtasks
              ON timer_sessions.target_type = 'subtask'
             AND timer_sessions.target_id = subtasks.id
             AND subtasks.deleted_at IS NULL
            WHERE stopped_at IS NULL
              AND timer_sessions.deleted_at IS NULL
              AND COALESCE(tasks.id, subtasks.id) IS NOT NULL
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
            SELECT id, target_type, target_id, started_at, stopped_at,
                   elapsed_seconds, deleted_at, created_at
            FROM timer_sessions
            WHERE id = ?1
              AND stopped_at IS NULL
              AND deleted_at IS NULL
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
            SELECT id, target_type, target_id, started_at, stopped_at,
                   elapsed_seconds, deleted_at, created_at
            FROM timer_sessions
            WHERE id = ?1
              AND deleted_at IS NULL
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
        deleted_at: row.get(6)?,
        created_at: row.get(7)?,
    })
}

fn calculate_stop_values(started_at: &str, now: &str) -> RepositoryResult<(String, i64)> {
    let started = OffsetDateTime::parse(started_at, &Rfc3339)
        .map_err(|error| format!("タイマー開始時刻の形式が不正です: {error}"))?;
    let stopped = OffsetDateTime::parse(now, &Rfc3339)
        .map_err(|error| format!("タイマー停止時刻の形式が不正です: {error}"))?;
    if stopped < started {
        return Ok((started_at.to_string(), 0));
    }

    Ok((now.to_string(), (stopped - started).whole_seconds()))
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
    run_ui_read_model_migration(connection)
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

fn normalize_list_id(list_id: Option<&str>) -> String {
    list_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_TASK_LIST_ID)
        .to_string()
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
              id, display_mode, created_at, updated_at
            )
            VALUES (
              'default',
              'title_only',
              strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
              strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            )
            ",
            [],
        )
        .map(|_| ())
        .map_err(|error| format!("通知表示設定の初期化に失敗しました: {error}"))
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
            SELECT id, title, planned_start_date, due_date, status
            FROM tasks
            WHERE deleted_at IS NULL
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
                status: WorkStatus::from_db(&row.get::<_, String>(4)?).map_err(db_value_error)?,
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
            SELECT id, title, planned_start_date, due_date, status
            FROM subtasks
            WHERE deleted_at IS NULL
              AND (
                planned_start_date BETWEEN ?1 AND ?2
                OR due_date BETWEEN ?1 AND ?2
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
                status: WorkStatus::from_db(&row.get::<_, String>(4)?).map_err(db_value_error)?,
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
    items: &mut Vec<WeekCalendarItem>,
) -> RepositoryResult<()> {
    let active = connection
        .query_row(
            "
            SELECT timer_sessions.target_type,
                   timer_sessions.target_id,
                   timer_sessions.started_at,
                   COALESCE(tasks.title, subtasks.title) AS title,
                   COALESCE(tasks.status, subtasks.status) AS status
            FROM timer_sessions
            LEFT JOIN tasks
              ON timer_sessions.target_type = 'task'
             AND timer_sessions.target_id = tasks.id
             AND tasks.deleted_at IS NULL
            LEFT JOIN subtasks
              ON timer_sessions.target_type = 'subtask'
             AND timer_sessions.target_id = subtasks.id
             AND subtasks.deleted_at IS NULL
            WHERE timer_sessions.stopped_at IS NULL
              AND timer_sessions.deleted_at IS NULL
              AND COALESCE(tasks.id, subtasks.id) IS NOT NULL
            LIMIT 1
            ",
            [],
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
                    date,
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
    status: WorkStatus,
}

fn push_calendar_items(row: CalendarSourceRow, items: &mut Vec<WeekCalendarItem>) {
    let target_type_text = row.target_type.as_str().to_string();
    if let Some(date) = row.planned_start_date {
        items.push(WeekCalendarItem {
            id: format!("{target_type_text}:{}:planned_start", row.id),
            target: target_ref(row.target_type.clone(), row.id.clone()),
            title: row.title.clone(),
            date,
            marker: CalendarMarker::PlannedStart,
            status: row.status.clone(),
        });
    }

    if let Some(date) = row.due_date {
        items.push(WeekCalendarItem {
            id: format!("{target_type_text}:{}:due", row.id),
            target: target_ref(row.target_type, row.id),
            title: row.title,
            date,
            marker: CalendarMarker::Due,
            status: row.status,
        });
    }
}

fn parse_date(value: &str) -> RepositoryResult<Date> {
    Date::parse(value, DATE_FORMAT).map_err(|error| format!("週開始日の形式が不正です: {error}"))
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
            repositories::TaskReadRepository,
            usecases,
        },
        domain::{notification::NotificationDisplayMode, task::WorkStatus, timer::WorkTargetType},
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
            title: title.to_string(),
            planned_start_date: None,
            due_date: None,
            memo: None,
        }
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

    #[test]
    fn migration_initializes_notification_preference() {
        let connection = Connection::open_in_memory().expect("in-memory database");
        configure_connection(&connection).expect("configure");
        run_initial_migration(&connection).expect("migrate");
        seed_default_preferences(&connection).expect("seed");

        let display_mode: String = connection
            .query_row(
                "SELECT display_mode FROM notification_preferences WHERE id = 'default'",
                [],
                |row| row.get(0),
            )
            .expect("default preference");

        assert_eq!(display_mode, "title_only");
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

        assert_eq!(list_id, DEFAULT_TASK_LIST_ID);
        assert_eq!(is_favorite, 0);
        assert_eq!(timer_target_seconds, None);
        assert_eq!(task_list_name, DEFAULT_TASK_LIST_NAME);
        assert_eq!(ui_preference_count, 3);
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
                    title: "画面に表示".to_string(),
                    planned_start_date: Some("2026-07-06".to_string()),
                    due_date: Some("2026-07-07".to_string()),
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
                title: "UI設計".to_string(),
                planned_start_date: Some("2026-07-06".to_string()),
                due_date: Some("2026-07-07".to_string()),
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
                title: "更新前".to_string(),
                planned_start_date: Some("2026-07-06".to_string()),
                due_date: Some("2026-07-07".to_string()),
                memo: Some("古いメモ".to_string()),
            },
        )
        .expect("create task");

        let updated = usecases::update_task(
            &database,
            &update_clock,
            task.id.clone(),
            usecases::WorkItemUpdateDraft {
                title: "更新後".to_string(),
                planned_start_date: Some("2026-07-08".to_string()),
                due_date: None,
                timer_target_seconds: Some(1_800),
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
                title: "子タスク更新".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-09".to_string()),
                timer_target_seconds: Some(900),
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
            ("2026-07-09T00:00:00Z".to_string(), "pending".to_string())
        );
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
                title: "目標時間".to_string(),
                planned_start_date: None,
                due_date: None,
                timer_target_seconds: Some(0),
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

        usecases::start_timer(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Task,
                id: task.id.clone(),
            },
        )
        .expect("start task timer");

        usecases::delete_task(&database, &clock, task.id.clone()).expect("delete task");

        let tasks = database
            .list_tasks_with_subtasks(200)
            .expect("list task tree");
        let active_timer = database.get_active_timer().expect("active timer");
        let (deleted_tasks, deleted_subtasks, deleted_timers, disabled_notifications): (
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
                    disabled_notifications,
                ))
            })
            .expect("deleted graph counts");

        assert!(tasks.is_empty());
        assert!(active_timer.is_none());
        assert_eq!(deleted_tasks, 1);
        assert_eq!(deleted_subtasks, 1);
        assert_eq!(deleted_timers, 1);
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

        usecases::start_timer(
            &database,
            &clock,
            WorkTargetRef {
                target_type: WorkTargetType::Subtask,
                id: subtask.id.clone(),
            },
        )
        .expect("start subtask timer");

        usecases::delete_subtask(&database, &clock, subtask.id.clone()).expect("delete subtask");

        let tasks = database
            .list_tasks_with_subtasks(200)
            .expect("list task tree");
        let active_timer = database.get_active_timer().expect("active timer");
        let (deleted_subtasks, deleted_timers, disabled_notifications): (i64, i64, i64) = database
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
                Ok((deleted_subtasks, deleted_timers, disabled_notifications))
            })
            .expect("deleted subtask graph counts");

        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].subtasks.is_empty());
        assert!(active_timer.is_none());
        assert_eq!(deleted_subtasks, 1);
        assert_eq!(deleted_timers, 1);
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
                title: "秘密の顧客タスク".to_string(),
                planned_start_date: Some("2026-07-06".to_string()),
                due_date: Some("2026-07-07".to_string()),
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
        let notification_gateway = RecordingNotificationGateway::failing("permission denied");

        usecases::create_task(
            &database,
            &clock,
            usecases::WorkItemDraft {
                title: "通知失敗確認".to_string(),
                planned_start_date: None,
                due_date: Some("2026-07-06".to_string()),
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
    }
}
