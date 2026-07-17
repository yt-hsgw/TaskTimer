use std::{env, error::Error, fs, path::PathBuf};

use rusqlite::{params, Connection};
use time::{format_description::FormatItem, macros::format_description, Date, Duration, Month};

const INITIAL_SCHEMA: &str = include_str!("../../migrations/0001_initial.sql");
const DATE_FORMAT: &[FormatItem<'_>] = format_description!("[year]-[month]-[day]");

#[derive(Debug)]
struct Config {
    out: PathBuf,
    task_count: usize,
    subtask_count: usize,
    timer_session_count: usize,
    list_count: usize,
    start_date: Date,
    day_span: i64,
    force: bool,
}

impl Config {
    fn default() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            out: PathBuf::from("tmp/perf/tasktimer-large.sqlite3"),
            task_count: 5_000,
            subtask_count: 20_000,
            timer_session_count: 50_000,
            list_count: 12,
            start_date: Date::from_calendar_date(2026, Month::July, 1)?,
            day_span: 90,
            force: false,
        })
    }

    fn validate(&self) -> Result<(), String> {
        if self.task_count == 0 {
            return Err("--tasks は1以上にしてください".to_string());
        }
        if self.list_count == 0 {
            return Err("--lists は1以上にしてください".to_string());
        }
        if self.list_count > 200 {
            return Err("--lists は200以下にしてください".to_string());
        }
        if self.day_span <= 0 || self.day_span > 366 {
            return Err("--days は1から366の範囲にしてください".to_string());
        }
        if self.task_count > 100_000 {
            return Err("--tasks は100000以下にしてください".to_string());
        }
        if self.subtask_count > 500_000 {
            return Err("--subtasks は500000以下にしてください".to_string());
        }
        if self.timer_session_count > 1_000_000 {
            return Err("--timers は1000000以下にしてください".to_string());
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = parse_args()?;
    config
        .validate()
        .map_err(|message| format!("引数が不正です: {message}"))?;

    if config.out.exists() {
        if config.force {
            fs::remove_file(&config.out)?;
        } else {
            return Err(format!(
                "{} は既に存在します。上書きする場合は --force を指定してください。",
                config.out.display()
            )
            .into());
        }
    }

    if let Some(parent) = config.out.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut connection = Connection::open(&config.out)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.execute_batch(INITIAL_SCHEMA)?;
    create_performance_indexes(&connection)?;

    seed_database(&mut connection, &config)?;
    connection.execute_batch("PRAGMA optimize;")?;

    println!("大量データ検証DBを作成しました: {}", config.out.display());
    println!(
        "tasks={}, subtasks={}, stopped_timer_sessions={}, active_pomodoro_sessions=1, task_lists={}",
        config.task_count, config.subtask_count, config.timer_session_count, config.list_count
    );
    println!(
        "アプリで使う場合は既存DBをバックアップしてから tasktimer.sqlite3 として配置してください。"
    );

    Ok(())
}

fn parse_args() -> Result<Config, Box<dyn Error>> {
    let mut config = Config::default()?;
    let args = env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "--force" => {
                config.force = true;
                index += 1;
            }
            "--out" => {
                config.out = PathBuf::from(next_value(&args, index, "--out")?);
                index += 2;
            }
            "--tasks" => {
                config.task_count = parse_usize(next_value(&args, index, "--tasks")?, "--tasks")?;
                index += 2;
            }
            "--subtasks" => {
                config.subtask_count =
                    parse_usize(next_value(&args, index, "--subtasks")?, "--subtasks")?;
                index += 2;
            }
            "--timers" => {
                config.timer_session_count =
                    parse_usize(next_value(&args, index, "--timers")?, "--timers")?;
                index += 2;
            }
            "--lists" => {
                config.list_count = parse_usize(next_value(&args, index, "--lists")?, "--lists")?;
                index += 2;
            }
            "--start-date" => {
                config.start_date = parse_date(next_value(&args, index, "--start-date")?)?;
                index += 2;
            }
            "--days" => {
                config.day_span = parse_i64(next_value(&args, index, "--days")?, "--days")?;
                index += 2;
            }
            value if value.starts_with("--out=") => {
                config.out = PathBuf::from(value.trim_start_matches("--out="));
                index += 1;
            }
            value if value.starts_with("--tasks=") => {
                config.task_count = parse_usize(value.trim_start_matches("--tasks="), "--tasks")?;
                index += 1;
            }
            value if value.starts_with("--subtasks=") => {
                config.subtask_count =
                    parse_usize(value.trim_start_matches("--subtasks="), "--subtasks")?;
                index += 1;
            }
            value if value.starts_with("--timers=") => {
                config.timer_session_count =
                    parse_usize(value.trim_start_matches("--timers="), "--timers")?;
                index += 1;
            }
            value if value.starts_with("--lists=") => {
                config.list_count = parse_usize(value.trim_start_matches("--lists="), "--lists")?;
                index += 1;
            }
            value if value.starts_with("--start-date=") => {
                config.start_date = parse_date(value.trim_start_matches("--start-date="))?;
                index += 1;
            }
            value if value.starts_with("--days=") => {
                config.day_span = parse_i64(value.trim_start_matches("--days="), "--days")?;
                index += 1;
            }
            _ => {
                return Err(format!("未対応の引数です: {arg}").into());
            }
        }
    }

    Ok(config)
}

fn seed_database(connection: &mut Connection, config: &Config) -> Result<(), Box<dyn Error>> {
    let transaction = connection.transaction()?;
    let created_at = datetime_for(config, 0, 9, 0)?;

    transaction.execute(
        "
        INSERT INTO notification_preferences (
          id, display_mode, notifications_enabled, created_at, updated_at
        ) VALUES ('default', 'title_only', 1, ?1, ?1)
        ON CONFLICT(id) DO NOTHING
        ",
        params![created_at],
    )?;
    transaction.execute(
        "
        INSERT INTO ui_preferences (key, value, updated_at)
        VALUES ('last_task_list_id', 'default', ?1)
        ON CONFLICT(key) DO NOTHING
        ",
        params![created_at],
    )?;

    {
        let mut statement = transaction.prepare(
            "
            INSERT INTO task_lists (
              id, name, color_token, sort_order, deleted_at, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?5)
            ",
        )?;

        for index in 0..config.list_count {
            let (id, name) = task_list_identity(index);
            statement.execute(params![
                id,
                name,
                task_list_color_token(index),
                index as i64,
                created_at
            ])?;
        }
    }

    {
        let mut statement = transaction.prepare(
            "
            INSERT INTO tasks (
              id, list_id, title, status, is_favorite,
              planned_start_date, due_date, due_time, timer_target_seconds,
              memo, sort_order, completed_at, deleted_at, created_at, updated_at
            ) VALUES (
              ?1, ?2, ?3, ?4, ?5,
              ?6, ?7, ?8, ?9,
              ?10, ?11, ?12, NULL, ?13, ?13
            )
            ",
        )?;

        for index in 0..config.task_count {
            let due = task_due_date(config, index)?;
            let planned = task_planned_date(config, index)?;
            let completed_at = if task_status(index) == "done" {
                Some(datetime_for(config, index as i64, 17, 0)?)
            } else {
                None
            };
            let created_at = datetime_for(config, index as i64, 8, (index % 60) as u8)?;

            statement.execute(params![
                task_id(index),
                task_list_id(index % config.list_count),
                format!("性能検証タスク {:05}", index + 1),
                task_status(index),
                if index % 11 == 0 { 1 } else { 0 },
                planned,
                due,
                due_time(index, due.is_some()),
                timer_target_seconds(index),
                task_memo(index),
                index as i64,
                completed_at,
                created_at,
            ])?;
        }
    }

    {
        let mut statement = transaction.prepare(
            "
            INSERT INTO subtasks (
              id, task_id, title, status,
              planned_start_date, due_date, due_time, timer_target_seconds,
              memo, sort_order, completed_at, deleted_at, created_at, updated_at
            ) VALUES (
              ?1, ?2, ?3, ?4,
              ?5, ?6, ?7, ?8,
              ?9, ?10, ?11, NULL, ?12, ?12
            )
            ",
        )?;

        for index in 0..config.subtask_count {
            let parent_index = index % config.task_count;
            let due = subtask_due_date(config, index)?;
            let planned = subtask_planned_date(config, index)?;
            let completed_at = if subtask_status(index) == "done" {
                Some(datetime_for(config, index as i64, 16, 30)?)
            } else {
                None
            };
            let created_at = datetime_for(config, index as i64, 9, (index % 60) as u8)?;

            statement.execute(params![
                subtask_id(index),
                task_id(parent_index),
                format!("性能検証サブタスク {:05}", index + 1),
                subtask_status(index),
                planned,
                due,
                due_time(index, due.is_some()),
                timer_target_seconds(index + 3),
                subtask_memo(index),
                (index / config.task_count) as i64,
                completed_at,
                created_at,
            ])?;
        }
    }

    {
        let mut statement = transaction.prepare(
            "
            INSERT INTO timer_sessions (
              id, target_type, target_id, started_at, stopped_at,
              elapsed_seconds, deleted_at, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?4)
            ",
        )?;

        for index in 0..config.timer_session_count {
            let (target_type, target_id) = timer_target(config, index);
            let started_at = datetime_for(config, index as i64, 10, (index % 60) as u8)?;
            let stopped_at = datetime_for(config, index as i64, 11, ((index + 25) % 60) as u8)?;
            let elapsed_seconds = 600 + ((index % 12) as i64 * 300);

            statement.execute(params![
                format!("perf-timer-{index:06}"),
                target_type,
                target_id,
                started_at,
                stopped_at,
                elapsed_seconds,
            ])?;
        }
    }

    let active_pomodoro_task_id = task_id(active_pomodoro_task_index(config));
    let active_started_at = datetime_for(config, 0, 12, 0)?;
    transaction.execute(
        "
        UPDATE tasks
        SET status = 'in_progress',
            completed_at = NULL,
            updated_at = ?2
        WHERE id = ?1
          AND deleted_at IS NULL
        ",
        params![active_pomodoro_task_id.as_str(), active_started_at.as_str()],
    )?;
    transaction.execute(
        "
        INSERT INTO timer_sessions (
          id, target_type, target_id, started_at, stopped_at,
          elapsed_seconds, deleted_at, created_at
        ) VALUES ('perf-active-pomodoro-timer', 'task', ?1, ?2, NULL, NULL, NULL, ?2)
        ",
        params![active_pomodoro_task_id.as_str(), active_started_at.as_str()],
    )?;
    transaction.execute(
        "
        INSERT INTO pomodoro_sessions (
          id, target_type, target_id, timer_session_id, phase, status,
          cycle_count, phase_started_at, phase_duration_seconds,
          paused_at, paused_total_seconds, completed_at, cancelled_at,
          deleted_at, created_at, updated_at
        ) VALUES (
          'perf-active-pomodoro', 'task', ?1, 'perf-active-pomodoro-timer', 'work', 'running',
          0, ?2, 1500,
          NULL, 0, NULL, NULL,
          NULL, ?2, ?2
        )
        ",
        params![active_pomodoro_task_id.as_str(), active_started_at.as_str()],
    )?;

    transaction.commit()?;
    Ok(())
}

fn create_performance_indexes(connection: &Connection) -> Result<(), Box<dyn Error>> {
    connection.execute_batch(
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

        CREATE INDEX IF NOT EXISTS tasks_due_time_idx
        ON tasks (due_date, due_time)
        WHERE deleted_at IS NULL;

        CREATE INDEX IF NOT EXISTS subtasks_task_status_idx
        ON subtasks (task_id, status)
        WHERE deleted_at IS NULL;

        CREATE INDEX IF NOT EXISTS subtasks_due_time_idx
        ON subtasks (due_date, due_time)
        WHERE deleted_at IS NULL;
        ",
    )?;
    Ok(())
}

fn next_value<'a>(args: &'a [String], index: usize, name: &str) -> Result<&'a str, Box<dyn Error>> {
    args.get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| format!("{name} の値がありません").into())
}

fn parse_usize(value: &str, name: &str) -> Result<usize, Box<dyn Error>> {
    value
        .parse::<usize>()
        .map_err(|error| format!("{name} は整数で指定してください: {error}").into())
}

fn parse_i64(value: &str, name: &str) -> Result<i64, Box<dyn Error>> {
    value
        .parse::<i64>()
        .map_err(|error| format!("{name} は整数で指定してください: {error}").into())
}

fn parse_date(value: &str) -> Result<Date, Box<dyn Error>> {
    Date::parse(value, DATE_FORMAT)
        .map_err(|error| format!("--start-date はYYYY-MM-DDで指定してください: {error}").into())
}

fn task_list_identity(index: usize) -> (String, String) {
    if index == 0 {
        ("default".to_string(), "タスク".to_string())
    } else {
        (
            task_list_id(index),
            format!("性能検証リスト {:03}", index + 1),
        )
    }
}

fn task_list_color_token(index: usize) -> &'static str {
    const COLORS: &[&str] = &["green", "blue", "amber", "rose", "violet", "gray"];
    COLORS[index % COLORS.len()]
}

fn task_list_id(index: usize) -> String {
    if index == 0 {
        "default".to_string()
    } else {
        format!("perf-list-{index:03}")
    }
}

fn task_id(index: usize) -> String {
    format!("perf-task-{index:05}")
}

fn subtask_id(index: usize) -> String {
    format!("perf-subtask-{index:05}")
}

fn active_pomodoro_task_index(config: &Config) -> usize {
    if config.task_count > 1 {
        1
    } else {
        0
    }
}

fn task_status(index: usize) -> &'static str {
    if index.is_multiple_of(5) {
        "done"
    } else if index.is_multiple_of(7) {
        "in_progress"
    } else {
        "todo"
    }
}

fn subtask_status(index: usize) -> &'static str {
    if index.is_multiple_of(4) {
        "done"
    } else if index.is_multiple_of(13) {
        "in_progress"
    } else {
        "todo"
    }
}

fn task_planned_date(config: &Config, index: usize) -> Result<Option<String>, Box<dyn Error>> {
    if index.is_multiple_of(4) {
        Ok(Some(format_date(date_for(config, index as i64)?)?))
    } else {
        Ok(None)
    }
}

fn task_due_date(config: &Config, index: usize) -> Result<Option<String>, Box<dyn Error>> {
    if index % 3 == 1 {
        return Ok(None);
    }
    let offset = index as i64
        + if index.is_multiple_of(4) {
            (index % 5) as i64
        } else {
            2
        };
    Ok(Some(format_date(date_for(config, offset)?)?))
}

fn subtask_planned_date(config: &Config, index: usize) -> Result<Option<String>, Box<dyn Error>> {
    if index.is_multiple_of(6) {
        Ok(Some(format_date(date_for(config, index as i64)?)?))
    } else {
        Ok(None)
    }
}

fn subtask_due_date(config: &Config, index: usize) -> Result<Option<String>, Box<dyn Error>> {
    if index % 2 == 1 {
        return Ok(None);
    }
    let offset = index as i64
        + if index.is_multiple_of(6) {
            (index % 4) as i64
        } else {
            1
        };
    Ok(Some(format_date(date_for(config, offset)?)?))
}

fn due_time(index: usize, has_due_date: bool) -> Option<String> {
    if has_due_date && index.is_multiple_of(3) {
        Some(format!(
            "{:02}:{:02}",
            8 + (index % 10),
            if index.is_multiple_of(2) { 0 } else { 30 }
        ))
    } else {
        None
    }
}

fn timer_target_seconds(index: usize) -> Option<i64> {
    if index.is_multiple_of(3) {
        Some(900 + ((index % 8) as i64 * 900))
    } else {
        None
    }
}

fn task_memo(index: usize) -> String {
    if index.is_multiple_of(4) {
        "性能検証用の短いメモです。個人情報や実業務データは含みません。".to_string()
    } else {
        String::new()
    }
}

fn subtask_memo(index: usize) -> String {
    if index.is_multiple_of(10) {
        "サブタスクの性能検証メモです。".to_string()
    } else {
        String::new()
    }
}

fn timer_target(config: &Config, index: usize) -> (&'static str, String) {
    if config.subtask_count > 0 && index % 2 == 1 {
        ("subtask", subtask_id(index % config.subtask_count))
    } else {
        ("task", task_id(index % config.task_count))
    }
}

fn date_for(config: &Config, offset: i64) -> Result<Date, Box<dyn Error>> {
    Ok(config.start_date + Duration::days(offset.rem_euclid(config.day_span)))
}

fn format_date(value: Date) -> Result<String, Box<dyn Error>> {
    value
        .format(DATE_FORMAT)
        .map_err(|error| format!("日付を整形できません: {error}").into())
}

fn datetime_for(
    config: &Config,
    offset: i64,
    hour: u8,
    minute: u8,
) -> Result<String, Box<dyn Error>> {
    let date = date_for(config, offset)?;
    Ok(format!(
        "{}T{:02}:{:02}:00.000Z",
        date.format(DATE_FORMAT)?,
        hour,
        minute
    ))
}

fn print_help() {
    println!(
        "\
TaskTimer 大量データ検証DB生成

Usage:
  cargo run --manifest-path src-tauri/Cargo.toml --bin seed_large_dataset -- [options]

Options:
  --out <path>         出力DBパス。default: tmp/perf/tasktimer-large.sqlite3
  --tasks <count>      タスク件数。default: 5000
  --subtasks <count>   サブタスク件数。default: 20000
  --timers <count>     停止済みタイマー履歴件数。default: 50000
  --lists <count>      タスクリスト件数。default: 12
  --start-date <date>  分布開始日。YYYY-MM-DD。default: 2026-07-01
  --days <count>       予定日を分布させる日数。default: 90
  --force              既存の出力DBを上書きする
  -h, --help           このヘルプを表示する
"
    );
}
