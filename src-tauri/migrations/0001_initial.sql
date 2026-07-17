PRAGMA foreign_keys = ON;

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

CREATE TABLE IF NOT EXISTS tags (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL CHECK (length(trim(name)) > 0 AND length(name) <= 40),
  sort_order INTEGER NOT NULL DEFAULT 0,
  deleted_at TEXT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tasks (
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

CREATE TABLE IF NOT EXISTS task_tags (
  task_id TEXT NOT NULL,
  tag_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  deleted_at TEXT NULL,
  PRIMARY KEY (task_id, tag_id),
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE RESTRICT,
  FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS subtasks (
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
VALUES (
  'default',
  1500,
  300,
  900,
  4,
  0,
  0,
  strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
);

CREATE TABLE IF NOT EXISTS pomodoro_sessions (
  id TEXT PRIMARY KEY,
  target_type TEXT NOT NULL CHECK (target_type IN ('task', 'subtask')),
  target_id TEXT NOT NULL,
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
  CHECK (phase <> 'work' OR timer_session_id IS NOT NULL),
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

CREATE TABLE IF NOT EXISTS notification_preferences (
  id TEXT PRIMARY KEY CHECK (id = 'default'),
  display_mode TEXT NOT NULL CHECK (display_mode IN ('title_only', 'generic')),
  notifications_enabled INTEGER NOT NULL DEFAULT 1 CHECK (notifications_enabled IN (0, 1)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ui_preferences (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

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

CREATE INDEX IF NOT EXISTS tasks_calendar_idx
ON tasks (planned_start_date, due_date)
WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS subtasks_calendar_idx
ON subtasks (planned_start_date, due_date)
WHERE deleted_at IS NULL;

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
