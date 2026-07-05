#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration as StdDuration,
};

use rusqlite::{params, Connection, OptionalExtension};
use tauri::{AppHandle, Manager};
use time::{macros::format_description, Date, Duration};

use crate::{
    application::repositories::{
        target_ref, ActiveTimer, CalendarMarker, CalendarRepository,
        NotificationPreferenceRepository, RepositoryResult, TimerRepository, WeekCalendarItem,
    },
    domain::{notification::NotificationDisplayMode, task::WorkStatus, timer::WorkTargetType},
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
}
