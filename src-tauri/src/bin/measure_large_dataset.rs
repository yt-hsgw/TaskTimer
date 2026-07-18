use std::{
    env,
    error::Error,
    path::PathBuf,
    time::{Duration, Instant},
};

use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug)]
struct Config {
    db: PathBuf,
    threshold: Duration,
    fail_on_warning: bool,
}

#[derive(Debug)]
struct Measurement {
    name: &'static str,
    rows: i64,
    elapsed: Duration,
    threshold: Duration,
}

impl Config {
    fn default() -> Self {
        Self {
            db: PathBuf::from("tmp/perf/tasktimer-large.sqlite3"),
            threshold: Duration::from_millis(250),
            fail_on_warning: false,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let config = parse_args()?;
    if !config.db.is_file() {
        return Err(format!(
            "{} が見つかりません。先に `npm run perf:seed -- --force` を実行してください。",
            config.db.display()
        )
        .into());
    }

    let connection = Connection::open(&config.db)?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    let integrity: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(format!("PRAGMA integrity_check が失敗しました: {integrity}").into());
    }

    let counts = read_counts(&connection)?;
    println!("大量データ性能DB計測");
    println!("DB: {}", config.db.display());
    println!(
        "件数: task_lists={}, board_columns={}, tasks={}, subtasks={}, timer_sessions={}, pomodoro_sessions={}",
        counts.task_lists,
        counts.board_columns,
        counts.tasks,
        counts.subtasks,
        counts.timer_sessions,
        counts.pomodoro_sessions
    );
    println!(
        "しきい値: {}ms / 超過時{}",
        config.threshold.as_millis(),
        if config.fail_on_warning {
            "に失敗扱い"
        } else {
            "は警告のみ"
        }
    );
    println!();

    let measurements = vec![
        measure_task_lists(&connection, config.threshold)?,
        measure_board_columns(&connection, config.threshold)?,
        measure_initial_task_tree(&connection, config.threshold)?,
        measure_task_rows(
            &connection,
            "task_rows_default_list",
            Some("default"),
            config.threshold,
        )?,
        measure_task_rows(&connection, "task_rows_all_lists", None, config.threshold)?,
        measure_calendar_items(
            &connection,
            "calendar_week_2026-07-13",
            "2026-07-13",
            "2026-07-19",
            config.threshold,
        )?,
        measure_calendar_items(
            &connection,
            "calendar_month_2026-07",
            "2026-07-01",
            "2026-07-31",
            config.threshold,
        )?,
        measure_active_timer(&connection, config.threshold)?,
        measure_active_pomodoro(&connection, config.threshold)?,
        measure_notification_dispatch_candidates(&connection, config.threshold)?,
        measure_task_detail(&connection, config.threshold)?,
    ];

    println!("| 対象 | 行数 | 時間 | 判定 |");
    println!("| --- | ---: | ---: | --- |");
    let mut warning_count = 0;
    for measurement in &measurements {
        let status = if measurement.elapsed > measurement.threshold {
            warning_count += 1;
            "WARN"
        } else {
            "OK"
        };
        println!(
            "| {} | {} | {}ms | {} |",
            measurement.name,
            measurement.rows,
            measurement.elapsed.as_millis(),
            status
        );
    }

    if warning_count > 0 {
        println!();
        println!(
            "{warning_count}件の計測がしきい値を超過しました。GUIで再現する場合はGitHub #72へ結果と端末情報を追記してください。"
        );
        if config.fail_on_warning {
            return Err("しきい値超過があるため失敗扱いにしました".into());
        }
    }

    Ok(())
}

fn parse_args() -> Result<Config, Box<dyn Error>> {
    let mut config = Config::default();
    let args = env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "--db" => {
                config.db = PathBuf::from(next_value(&args, index, "--db")?);
                index += 2;
            }
            "--threshold-ms" => {
                config.threshold =
                    Duration::from_millis(parse_u64(next_value(&args, index, "--threshold-ms")?)?);
                index += 2;
            }
            "--fail-on-warning" => {
                config.fail_on_warning = true;
                index += 1;
            }
            value if value.starts_with("--db=") => {
                config.db = PathBuf::from(value.trim_start_matches("--db="));
                index += 1;
            }
            value if value.starts_with("--threshold-ms=") => {
                config.threshold =
                    Duration::from_millis(parse_u64(value.trim_start_matches("--threshold-ms="))?);
                index += 1;
            }
            _ => return Err(format!("未対応の引数です: {arg}").into()),
        }
    }

    if config.threshold.is_zero() {
        return Err("--threshold-ms は1以上にしてください".into());
    }

    Ok(config)
}

fn next_value<'a>(args: &'a [String], index: usize, name: &str) -> Result<&'a str, Box<dyn Error>> {
    args.get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| format!("{name} の値がありません").into())
}

fn parse_u64(value: &str) -> Result<u64, Box<dyn Error>> {
    value
        .parse::<u64>()
        .map_err(|error| format!("整数で指定してください: {error}").into())
}

#[derive(Debug)]
struct Counts {
    task_lists: i64,
    board_columns: i64,
    tasks: i64,
    subtasks: i64,
    timer_sessions: i64,
    pomodoro_sessions: i64,
}

fn read_counts(connection: &Connection) -> Result<Counts, Box<dyn Error>> {
    Ok(Counts {
        task_lists: count_table(connection, "task_lists")?,
        board_columns: count_table(connection, "board_columns")?,
        tasks: count_table(connection, "tasks")?,
        subtasks: count_table(connection, "subtasks")?,
        timer_sessions: count_table(connection, "timer_sessions")?,
        pomodoro_sessions: count_table(connection, "pomodoro_sessions")?,
    })
}

fn count_table(connection: &Connection, table_name: &str) -> Result<i64, Box<dyn Error>> {
    let sql = format!("SELECT COUNT(*) FROM {table_name}");
    Ok(connection.query_row(&sql, [], |row| row.get(0))?)
}

fn measure_step(
    name: &'static str,
    threshold: Duration,
    run: impl FnOnce() -> Result<i64, Box<dyn Error>>,
) -> Result<Measurement, Box<dyn Error>> {
    let started = Instant::now();
    let rows = run()?;
    Ok(Measurement {
        name,
        rows,
        elapsed: started.elapsed(),
        threshold,
    })
}

fn measure_count_query(
    connection: &Connection,
    name: &'static str,
    sql: &str,
    threshold: Duration,
) -> Result<Measurement, Box<dyn Error>> {
    measure_step(name, threshold, || {
        Ok(connection.query_row(sql, [], |row| row.get(0))?)
    })
}

fn measure_task_lists(
    connection: &Connection,
    threshold: Duration,
) -> Result<Measurement, Box<dyn Error>> {
    measure_count_query(
        connection,
        "task_lists_with_counts",
        "
        SELECT COUNT(*)
        FROM (
          SELECT task_lists.id,
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
        )
        ",
        threshold,
    )
}

fn measure_board_columns(
    connection: &Connection,
    threshold: Duration,
) -> Result<Measurement, Box<dyn Error>> {
    measure_count_query(
        connection,
        "board_columns_with_lifecycle_counts",
        "
        SELECT COUNT(*)
        FROM (
          SELECT board_columns.id,
                 COUNT(tasks.id) AS task_count,
                 COALESCE(SUM(CASE WHEN tasks.lifecycle_status = 'active' THEN 1 ELSE 0 END), 0)
                   AS active_task_count,
                 COALESCE(SUM(CASE WHEN tasks.lifecycle_status = 'done' THEN 1 ELSE 0 END), 0)
                   AS completed_task_count
          FROM board_columns
          LEFT JOIN tasks
            ON tasks.board_column_id = board_columns.id
           AND tasks.deleted_at IS NULL
           AND tasks.lifecycle_status <> 'archived'
          WHERE board_columns.deleted_at IS NULL
          GROUP BY board_columns.id,
                   board_columns.title,
                   board_columns.sort_order,
                   board_columns.created_at,
                   board_columns.updated_at
          ORDER BY board_columns.sort_order,
                   board_columns.created_at
        )
        ",
        threshold,
    )
}

fn measure_initial_task_tree(
    connection: &Connection,
    threshold: Duration,
) -> Result<Measurement, Box<dyn Error>> {
    measure_step("initial_task_tree_200", threshold, || {
        let task_count: i64 = connection.query_row(
            "
            WITH selected_tasks AS (
              SELECT id
              FROM tasks
              WHERE deleted_at IS NULL
                AND status <> 'archived'
              ORDER BY sort_order ASC, created_at ASC
              LIMIT 200
            )
            SELECT COUNT(*) FROM selected_tasks
            ",
            [],
            |row| row.get(0),
        )?;
        let subtask_count: i64 = connection.query_row(
            "
            WITH selected_tasks AS (
              SELECT id, sort_order
              FROM tasks
              WHERE deleted_at IS NULL
                AND status <> 'archived'
              ORDER BY sort_order ASC, created_at ASC
              LIMIT 200
            )
            SELECT COUNT(*)
            FROM subtasks
            INNER JOIN selected_tasks
              ON selected_tasks.id = subtasks.task_id
            LEFT JOIN recurrence_rules
              ON recurrence_rules.target_type = 'subtask'
             AND recurrence_rules.target_id = subtasks.id
             AND recurrence_rules.deleted_at IS NULL
            WHERE subtasks.deleted_at IS NULL
            ",
            [],
            |row| row.get(0),
        )?;
        Ok(task_count + subtask_count)
    })
}

fn measure_task_rows(
    connection: &Connection,
    name: &'static str,
    list_id: Option<&str>,
    threshold: Duration,
) -> Result<Measurement, Box<dyn Error>> {
    measure_step(name, threshold, || {
        Ok(connection.query_row(
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
            ),
            task_rows AS (
              SELECT tasks.id,
                     tasks.board_column_id,
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
                AND tasks.status <> 'archived'
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
              LIMIT 200
            )
            SELECT COUNT(*) FROM task_rows
            ",
            params![list_id],
            |row| row.get(0),
        )?)
    })
}

fn measure_calendar_items(
    connection: &Connection,
    name: &'static str,
    start_date: &str,
    end_date: &str,
    threshold: Duration,
) -> Result<Measurement, Box<dyn Error>> {
    measure_step(name, threshold, || {
        let task_items: i64 = connection.query_row(
            "
            SELECT COUNT(*)
            FROM tasks
            INNER JOIN task_lists
              ON task_lists.id = tasks.list_id
             AND task_lists.deleted_at IS NULL
            WHERE tasks.deleted_at IS NULL
              AND tasks.status <> 'archived'
              AND (
                tasks.planned_start_date BETWEEN ?1 AND ?2
                OR tasks.due_date BETWEEN ?1 AND ?2
              )
            ",
            params![start_date, end_date],
            |row| row.get(0),
        )?;
        let subtask_items: i64 = connection.query_row(
            "
            SELECT COUNT(*)
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
                subtasks.planned_start_date BETWEEN ?1 AND ?2
                OR subtasks.due_date BETWEEN ?1 AND ?2
              )
            ",
            params![start_date, end_date],
            |row| row.get(0),
        )?;
        Ok(task_items + subtask_items)
    })
}

fn measure_active_timer(
    connection: &Connection,
    threshold: Duration,
) -> Result<Measurement, Box<dyn Error>> {
    measure_count_query(
        connection,
        "active_timer_lookup",
        "
        SELECT COUNT(*)
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
        ",
        threshold,
    )
}

fn measure_active_pomodoro(
    connection: &Connection,
    threshold: Duration,
) -> Result<Measurement, Box<dyn Error>> {
    measure_count_query(
        connection,
        "active_pomodoro_lookup",
        "
        SELECT COUNT(*)
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
            (
              pomodoro_sessions.target_type = 'task'
              AND task_targets.id IS NOT NULL
            )
            OR (
              pomodoro_sessions.target_type = 'subtask'
              AND subtask_targets.id IS NOT NULL
              AND parent_tasks.id IS NOT NULL
            )
          )
        ",
        threshold,
    )
}

fn measure_notification_dispatch_candidates(
    connection: &Connection,
    threshold: Duration,
) -> Result<Measurement, Box<dyn Error>> {
    measure_count_query(
        connection,
        "notification_dispatch_candidates",
        "
        SELECT COUNT(*)
        FROM (
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
          LEFT JOIN tasks AS parent_tasks
            ON notification_rules.target_type = 'subtask'
           AND subtasks.task_id = parent_tasks.id
           AND parent_tasks.deleted_at IS NULL
           AND parent_tasks.status <> 'archived'
          WHERE notification_rules.enabled = 1
            AND notification_rules.deleted_at IS NULL
            AND notification_rules.notify_at <= '2026-07-31T23:59:59Z'
            AND notification_rules.registration_status IN ('pending', 'failed')
            AND (
              (
                notification_rules.target_type = 'task'
                AND tasks.id IS NOT NULL
              )
              OR (
                notification_rules.target_type = 'subtask'
                AND subtasks.id IS NOT NULL
                AND parent_tasks.id IS NOT NULL
              )
            )
          ORDER BY notification_rules.notify_at ASC,
                   notification_rules.created_at ASC
          LIMIT 20
        )
        ",
        threshold,
    )
}

fn measure_task_detail(
    connection: &Connection,
    threshold: Duration,
) -> Result<Measurement, Box<dyn Error>> {
    measure_step("task_detail_subtasks", threshold, || {
        let task_id = connection
            .query_row(
                "
                SELECT id
                FROM tasks
                WHERE deleted_at IS NULL
                  AND status <> 'archived'
                ORDER BY sort_order ASC
                LIMIT 1
                ",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        let Some(task_id) = task_id else {
            return Ok(0);
        };
        Ok(connection.query_row(
            "
            SELECT COUNT(*)
            FROM subtasks
            LEFT JOIN recurrence_rules
              ON recurrence_rules.target_type = 'subtask'
             AND recurrence_rules.target_id = subtasks.id
             AND recurrence_rules.deleted_at IS NULL
            WHERE subtasks.task_id = ?1
              AND subtasks.deleted_at IS NULL
            ",
            params![task_id],
            |row| row.get(0),
        )?)
    })
}

fn print_help() {
    println!(
        "\
TaskTimer 大量データRead Model計測

Usage:
  cargo run --manifest-path src-tauri/Cargo.toml --bin measure_large_dataset -- [options]

Options:
  --db <path>             計測対象DB。default: tmp/perf/tasktimer-large.sqlite3
  --threshold-ms <ms>     1計測あたりの警告しきい値。default: 250
  --fail-on-warning       しきい値超過を終了コード1として扱う
  -h, --help              このヘルプを表示する
"
    );
}
