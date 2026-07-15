-- TaskTimer MVP SQLiteスキーマ案。
-- このファイルは設計上の意図を記録する。
-- 実行時マイグレーションはこの設計から生成・更新する。

PRAGMA foreign_keys = ON;

CREATE TABLE task_lists (
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

CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  list_id TEXT NOT NULL DEFAULT 'default',
  title TEXT NOT NULL CHECK (length(trim(title)) > 0),
  status TEXT NOT NULL CHECK (status IN ('todo', 'in_progress', 'done', 'archived')),
  is_favorite INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0, 1)),
  planned_start_date TEXT NULL,
  due_date TEXT NULL,
  due_time TEXT NULL CHECK (
    due_time IS NULL OR (
      length(due_time) = 5
      AND substr(due_time, 3, 1) = ':'
      AND substr(due_time, 1, 2) BETWEEN '00' AND '23'
      AND substr(due_time, 4, 2) BETWEEN '00' AND '59'
    )
  ),
  timer_target_seconds INTEGER NULL CHECK (
    timer_target_seconds IS NULL OR timer_target_seconds >= 0
  ),
  memo TEXT NOT NULL DEFAULT '',
  sort_order INTEGER NOT NULL DEFAULT 0,
  completed_at TEXT NULL,
  deleted_at TEXT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (list_id) REFERENCES task_lists(id) ON DELETE RESTRICT,
  CHECK (
    planned_start_date IS NULL
    OR due_date IS NULL
    OR due_date >= planned_start_date
  )
);

CREATE TABLE subtasks (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  title TEXT NOT NULL CHECK (length(trim(title)) > 0),
  status TEXT NOT NULL CHECK (status IN ('todo', 'in_progress', 'done', 'archived')),
  planned_start_date TEXT NULL,
  due_date TEXT NULL,
  due_time TEXT NULL CHECK (
    due_time IS NULL OR (
      length(due_time) = 5
      AND substr(due_time, 3, 1) = ':'
      AND substr(due_time, 1, 2) BETWEEN '00' AND '23'
      AND substr(due_time, 4, 2) BETWEEN '00' AND '59'
    )
  ),
  timer_target_seconds INTEGER NULL CHECK (
    timer_target_seconds IS NULL OR timer_target_seconds >= 0
  ),
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

CREATE TABLE timer_sessions (
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

CREATE UNIQUE INDEX one_active_timer
ON timer_sessions ((stopped_at IS NULL))
WHERE stopped_at IS NULL AND deleted_at IS NULL;

CREATE INDEX timer_sessions_target_idx
ON timer_sessions (target_type, target_id, started_at)
WHERE deleted_at IS NULL;

CREATE TABLE timer_pauses (
  id TEXT PRIMARY KEY,
  timer_session_id TEXT NOT NULL,
  paused_at TEXT NOT NULL,
  resumed_at TEXT NULL,
  deleted_at TEXT NULL,
  created_at TEXT NOT NULL,
  FOREIGN KEY (timer_session_id) REFERENCES timer_sessions(id) ON DELETE RESTRICT,
  CHECK (resumed_at IS NULL OR resumed_at >= paused_at)
);

CREATE UNIQUE INDEX one_open_pause_per_timer
ON timer_pauses (timer_session_id)
WHERE resumed_at IS NULL AND deleted_at IS NULL;

CREATE INDEX timer_pauses_session_idx
ON timer_pauses (timer_session_id, paused_at)
WHERE deleted_at IS NULL;

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

CREATE INDEX notification_rules_schedule_idx
ON notification_rules (enabled, notify_at)
WHERE deleted_at IS NULL;

CREATE INDEX notification_rules_target_idx
ON notification_rules (target_type, target_id)
WHERE deleted_at IS NULL;

CREATE TABLE notification_delivery_attempts (
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

CREATE INDEX notification_delivery_attempts_recent_idx
ON notification_delivery_attempts (attempted_at DESC, created_at DESC);

CREATE INDEX notification_delivery_attempts_rule_idx
ON notification_delivery_attempts (notification_rule_id, attempted_at DESC);

CREATE TABLE notification_preferences (
  id TEXT PRIMARY KEY CHECK (id = 'default'),
  display_mode TEXT NOT NULL CHECK (display_mode IN ('title_only', 'generic')),
  notifications_enabled INTEGER NOT NULL DEFAULT 1 CHECK (notifications_enabled IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE ui_preferences (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE recurrence_rules (
  id TEXT PRIMARY KEY,
  target_type TEXT NOT NULL CHECK (target_type IN ('task', 'subtask')),
  target_id TEXT NOT NULL,
  frequency TEXT NOT NULL CHECK (frequency IN ('daily', 'weekly', 'monthly')),
  interval INTEGER NOT NULL CHECK (interval >= 1 AND interval <= 365),
  deleted_at TEXT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX recurrence_rules_active_target_idx
ON recurrence_rules (target_type, target_id)
WHERE deleted_at IS NULL;

CREATE INDEX recurrence_rules_target_idx
ON recurrence_rules (target_type, target_id, frequency)
WHERE deleted_at IS NULL;

CREATE INDEX task_lists_order_idx
ON task_lists (sort_order, created_at)
WHERE deleted_at IS NULL;

CREATE INDEX tasks_list_status_idx
ON tasks (list_id, status, sort_order, created_at)
WHERE deleted_at IS NULL;

CREATE INDEX tasks_favorite_idx
ON tasks (is_favorite, sort_order, created_at)
WHERE deleted_at IS NULL AND is_favorite = 1;

CREATE INDEX tasks_calendar_idx
ON tasks (planned_start_date, due_date)
WHERE deleted_at IS NULL;

CREATE INDEX tasks_due_time_idx
ON tasks (due_date, due_time)
WHERE deleted_at IS NULL;

CREATE INDEX subtasks_task_status_idx
ON subtasks (task_id, status)
WHERE deleted_at IS NULL;

CREATE INDEX subtasks_calendar_idx
ON subtasks (planned_start_date, due_date)
WHERE deleted_at IS NULL;

CREATE INDEX subtasks_due_time_idx
ON subtasks (due_date, due_time)
WHERE deleted_at IS NULL;
