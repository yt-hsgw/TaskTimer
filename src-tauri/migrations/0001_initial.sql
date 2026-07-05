PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS tasks (
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
  updated_at TEXT NOT NULL,
  CHECK (
    planned_start_date IS NULL
    OR due_date IS NULL
    OR due_date >= planned_start_date
  )
);

CREATE TABLE IF NOT EXISTS subtasks (
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
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE RESTRICT,
  CHECK (
    planned_start_date IS NULL
    OR due_date IS NULL
    OR due_date >= planned_start_date
  )
);

CREATE TABLE IF NOT EXISTS timer_sessions (
  id TEXT PRIMARY KEY,
  target_type TEXT NOT NULL CHECK (target_type IN ('task', 'subtask')),
  target_id TEXT NOT NULL,
  started_at TEXT NOT NULL,
  stopped_at TEXT NULL,
  elapsed_seconds INTEGER NULL CHECK (elapsed_seconds IS NULL OR elapsed_seconds >= 0),
  deleted_at TEXT NULL,
  created_at TEXT NOT NULL,
  CHECK (stopped_at IS NULL OR stopped_at >= started_at)
);

CREATE UNIQUE INDEX IF NOT EXISTS one_active_timer
ON timer_sessions ((stopped_at IS NULL))
WHERE stopped_at IS NULL AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS timer_sessions_target_idx
ON timer_sessions (target_type, target_id, started_at)
WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS notification_rules (
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

CREATE INDEX IF NOT EXISTS notification_rules_schedule_idx
ON notification_rules (enabled, notify_at)
WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS notification_rules_target_idx
ON notification_rules (target_type, target_id)
WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS notification_preferences (
  id TEXT PRIMARY KEY CHECK (id = 'default'),
  display_mode TEXT NOT NULL CHECK (display_mode IN ('title_only', 'generic')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS tasks_calendar_idx
ON tasks (planned_start_date, due_date)
WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS subtasks_calendar_idx
ON subtasks (planned_start_date, due_date)
WHERE deleted_at IS NULL;
