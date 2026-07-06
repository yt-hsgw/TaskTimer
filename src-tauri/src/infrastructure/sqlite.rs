#![allow(dead_code)]

use std::{
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
        target_ref, ActiveTimer, CalendarMarker, CalendarRepository,
        NotificationPreferenceRepository, RepositoryResult, SubtaskRecord, TaskRecord,
        TaskTimerCommandRepository, TimerRepository, WeekCalendarItem, WorkItemCreate,
    },
    domain::{
        notification::NotificationDisplayMode,
        task::{assert_timer_startable, WorkStatus},
        timer::{WorkTargetRef, WorkTargetType},
    },
};

pub const INITIAL_SCHEMA: &str = include_str!("../../migrations/0001_initial.sql");

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
        self.with_connection(|connection| {
            connection
                .query_row(
                    "
                    SELECT id, target_type, target_id, started_at, stopped_at,
                           elapsed_seconds, deleted_at, created_at
                    FROM timer_sessions
                    WHERE stopped_at IS NULL
                      AND deleted_at IS NULL
                    LIMIT 1
                    ",
                    [],
                    |row| {
                        let target_type_text: String = row.get(1)?;
                        let target_type =
                            WorkTargetType::from_db(&target_type_text).map_err(db_value_error)?;
                        Ok(ActiveTimer {
                            id: row.get(0)?,
                            target: target_ref(target_type, row.get(2)?),
                            started_at: row.get(3)?,
                            stopped_at: row.get(4)?,
                            elapsed_seconds: row.get(5)?,
                            deleted_at: row.get(6)?,
                            created_at: row.get(7)?,
                        })
                    },
                )
                .optional()
                .map_err(|error| format!("アクティブタイマーを取得できません: {error}"))
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

fn insert_task(
    transaction: &Transaction<'_>,
    input: WorkItemCreate,
) -> RepositoryResult<TaskRecord> {
    let id = Uuid::new_v4().to_string();
    let sort_order = next_task_sort_order(transaction)?;
    transaction
        .execute(
            "
            INSERT INTO tasks (
              id, title, status, planned_start_date, due_date, memo,
              sort_order, created_at, updated_at
            )
            VALUES (?1, ?2, 'todo', ?3, ?4, ?5, ?6, ?7, ?7)
            ",
            params![
                id,
                input.title,
                input.planned_start_date,
                input.due_date,
                input.memo,
                sort_order,
                input.now
            ],
        )
        .map_err(|error| format!("タスクを作成できません: {error}"))?;

    select_task_by_id(transaction, &id)
}

fn insert_subtask(
    transaction: &Transaction<'_>,
    task_id: &str,
    input: WorkItemCreate,
) -> RepositoryResult<SubtaskRecord> {
    let id = Uuid::new_v4().to_string();
    let sort_order = next_subtask_sort_order(transaction, task_id)?;
    transaction
        .execute(
            "
            INSERT INTO subtasks (
              id, task_id, title, status, planned_start_date, due_date, memo,
              sort_order, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, 'todo', ?4, ?5, ?6, ?7, ?8, ?8)
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

    select_subtask_by_id(transaction, &id)
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

fn next_task_sort_order(connection: &Connection) -> RepositoryResult<i64> {
    connection
        .query_row(
            "
            SELECT COALESCE(MAX(sort_order), -1) + 1
            FROM tasks
            WHERE deleted_at IS NULL
            ",
            [],
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
            SELECT id, title, status, planned_start_date, due_date, memo,
                   sort_order, completed_at, deleted_at, created_at, updated_at
            FROM tasks
            WHERE id = ?1
            ",
            params![id],
            |row| {
                Ok(TaskRecord {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    status: WorkStatus::from_db(&row.get::<_, String>(2)?)
                        .map_err(db_value_error)?,
                    planned_start_date: row.get(3)?,
                    due_date: row.get(4)?,
                    memo: row.get(5)?,
                    sort_order: row.get(6)?,
                    completed_at: row.get(7)?,
                    deleted_at: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )
        .map_err(|error| format!("タスクを取得できません: {error}"))
}

fn select_subtask_by_id(connection: &Connection, id: &str) -> RepositoryResult<SubtaskRecord> {
    connection
        .query_row(
            "
            SELECT id, task_id, title, status, planned_start_date, due_date, memo,
                   sort_order, completed_at, deleted_at, created_at, updated_at
            FROM subtasks
            WHERE id = ?1
            ",
            params![id],
            |row| {
                Ok(SubtaskRecord {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    title: row.get(2)?,
                    status: WorkStatus::from_db(&row.get::<_, String>(3)?)
                        .map_err(db_value_error)?,
                    planned_start_date: row.get(4)?,
                    due_date: row.get(5)?,
                    memo: row.get(6)?,
                    sort_order: row.get(7)?,
                    completed_at: row.get(8)?,
                    deleted_at: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            },
        )
        .map_err(|error| format!("サブタスクを取得できません: {error}"))
}

fn select_active_timer(connection: &Connection) -> RepositoryResult<Option<ActiveTimer>> {
    connection
        .query_row(
            "
            SELECT id, target_type, target_id, started_at, stopped_at,
                   elapsed_seconds, deleted_at, created_at
            FROM timer_sessions
            WHERE stopped_at IS NULL
              AND deleted_at IS NULL
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
        .map_err(|error| format!("SQLite初期マイグレーションに失敗しました: {error}"))
}

fn seed_default_preferences(connection: &Connection) -> RepositoryResult<()> {
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
        application::{clock::Clock, usecases},
        domain::{task::WorkStatus, timer::WorkTargetType},
    };

    struct FixedClock {
        now: &'static str,
    }

    impl Clock for FixedClock {
        fn now_utc_iso8601(&self) -> String {
            self.now.to_string()
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
}
