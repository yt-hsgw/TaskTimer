import { invoke } from "@tauri-apps/api/core";
import type {
  ActivePomodoro,
  BoardColumn,
  CreateSubtaskDraft,
  DataExportResult,
  NotificationDeliveryAttempt,
  NotificationDispatchSummary,
  NativeNotificationRegistrationSummary,
  NextNotificationSchedule,
  NotificationSyncResult,
  PomodoroExpirySyncResult,
  PomodoroSettings,
  PomodoroSettingsDraft,
  SqliteBackupResult,
  SqliteRestoreResult,
  TagDraft,
  TagItem,
  TaskTag,
  TaskTimerGateway,
  TaskListItem,
  TaskRow,
  TaskWithSubtasks,
  TaskListDraft,
  UiPreferences,
  UpdateSubtaskDraft,
  UpdateTaskDraft,
  WeekCalendarItem,
  WorkItemDraft,
} from "../../application/usecases/contracts";
import type { ActiveTimer, TimerSession } from "../../domain/timer/types";
import type { NotificationDisplayMode } from "../../domain/notification/types";
import type { Subtask, Task, WorkTargetRef } from "../../domain/task/types";

export const tauriTaskTimerGateway: TaskTimerGateway = {
  healthCheck: () => invoke<string>("health_check"),
  listTasks: () => invoke<TaskWithSubtasks[]>("list_tasks"),
  listTaskLists: () => invoke<TaskListItem[]>("list_task_lists"),
  listBoardColumns: () => invoke<BoardColumn[]>("list_board_columns"),
  listTags: () => invoke<TagItem[]>("list_tags"),
  listTaskRows: (listId?: string | null) =>
    invoke<TaskRow[]>("list_task_rows", { listId: listId ?? null }),
  listArchivedTaskRows: () => invoke<TaskRow[]>("list_archived_task_rows"),
  listCalendarItems: (startDate, endDate) =>
    invoke<WeekCalendarItem[]>("list_calendar_items", { startDate, endDate }),
  listWeekCalendarItems: (weekStartDate) =>
    invoke<WeekCalendarItem[]>("list_week_calendar_items", { weekStartDate }),
  getActiveTimer: () => invoke<ActiveTimer | null>("get_active_timer"),
  getActivePomodoro: () => invoke<ActivePomodoro | null>("get_active_pomodoro"),
  syncExpiredPomodoro: () =>
    invoke<PomodoroExpirySyncResult>("sync_expired_pomodoro"),
  getPomodoroSettings: () => invoke<PomodoroSettings>("get_pomodoro_settings"),
  updatePomodoroSettings: (input: PomodoroSettingsDraft) =>
    invoke<PomodoroSettings>("update_pomodoro_settings", { request: input }),
  getNotificationDisplayMode: () =>
    invoke<NotificationDisplayMode>("get_notification_display_mode"),
  getUiPreferences: () => invoke<UiPreferences>("get_ui_preferences"),
  updateUiPreferences: (input: UiPreferences) =>
    invoke<UiPreferences>("update_ui_preferences", { request: input }),
  createTask: (input: WorkItemDraft) =>
    invoke<Task>("create_task", { request: input }),
  createTaskList: (input: TaskListDraft) =>
    invoke<TaskListItem>("create_task_list", { request: input }),
  updateTaskList: (listId: string, input: TaskListDraft) =>
    invoke<TaskListItem>("update_task_list", { request: { ...input, listId } }),
  deleteTaskList: (listId: string) =>
    invoke<void>("delete_task_list", { request: { listId } }),
  createBoardColumn: (title: string) =>
    invoke<BoardColumn>("create_board_column", { request: { title } }),
  updateBoardColumn: (columnId: string, title: string) =>
    invoke<BoardColumn>("update_board_column", {
      request: { columnId, title },
    }),
  reorderBoardColumns: (orderedColumnIds: string[]) =>
    invoke<BoardColumn[]>("reorder_board_columns", {
      request: { orderedColumnIds },
    }),
  deleteBoardColumn: (columnId: string, moveTasksToColumnId: string) =>
    invoke<void>("delete_board_column", {
      request: { columnId, moveTasksToColumnId },
    }),
  moveTaskToBoardColumn: (taskId: string, boardColumnId: string) =>
    invoke<void>("move_task_to_board_column", {
      request: { taskId, boardColumnId },
    }),
  createTag: (input: TagDraft) => invoke<TagItem>("create_tag", { request: input }),
  updateTag: (tagId: string, input: TagDraft) =>
    invoke<TagItem>("update_tag", { request: { ...input, tagId } }),
  deleteTag: (tagId: string) =>
    invoke<void>("delete_tag", { request: { tagId } }),
  attachTagToTask: (taskId: string, tagId: string) =>
    invoke<TaskTag>("attach_tag_to_task", { request: { taskId, tagId } }),
  detachTagFromTask: (taskId: string, tagId: string) =>
    invoke<void>("detach_tag_from_task", { request: { taskId, tagId } }),
  createSubtask: (input: CreateSubtaskDraft) =>
    invoke<Subtask>("create_subtask", { request: input }),
  updateTask: (input: UpdateTaskDraft) =>
    invoke<Task>("update_task", { request: input }),
  updateSubtask: (input: UpdateSubtaskDraft) =>
    invoke<Subtask>("update_subtask", { request: input }),
  startTimer: (target: WorkTargetRef) =>
    invoke<ActiveTimer>("start_timer", { request: { target } }),
  startPomodoro: (target: WorkTargetRef) =>
    invoke<ActivePomodoro>("start_pomodoro", { request: { target } }),
  pausePomodoro: () => invoke<ActivePomodoro>("pause_pomodoro"),
  resumePomodoro: () => invoke<ActivePomodoro>("resume_pomodoro"),
  completePomodoroWorkPhase: () =>
    invoke<ActivePomodoro>("complete_pomodoro_work_phase"),
  startPomodoroBreak: (pomodoroSessionId: string) =>
    invoke<ActivePomodoro>("start_pomodoro_break", {
      request: { pomodoroSessionId },
    }),
  skipPomodoroBreak: (pomodoroSessionId: string) =>
    invoke<ActivePomodoro>("skip_pomodoro_break", {
      request: { pomodoroSessionId },
    }),
  completePomodoroBreak: () => invoke<ActivePomodoro>("complete_pomodoro_break"),
  cancelPomodoro: () => invoke<ActivePomodoro>("cancel_pomodoro"),
  pauseActiveTimer: () => invoke<ActiveTimer>("pause_active_timer"),
  resumeActiveTimer: () => invoke<ActiveTimer>("resume_active_timer"),
  stopActiveTimer: () => invoke<TimerSession>("stop_active_timer"),
  completeTask: (taskId: string, allowIncompleteSubtasks: boolean) =>
    invoke<Task>("complete_task", {
      request: { taskId, allowIncompleteSubtasks },
    }),
  updateTaskStatus: (
    taskId: string,
    status: Task["status"],
    allowIncompleteSubtasks: boolean,
  ) =>
    invoke<Task>("update_task_status", {
      request: { taskId, status, allowIncompleteSubtasks },
    }),
  reopenTask: (taskId: string) =>
    invoke<Task>("reopen_task", { request: { taskId } }),
  completeSubtask: (subtaskId: string) =>
    invoke<Subtask>("complete_subtask", { request: { subtaskId } }),
  reopenSubtask: (subtaskId: string) =>
    invoke<Subtask>("reopen_subtask", { request: { subtaskId } }),
  toggleTaskFavorite: (taskId: string, isFavorite: boolean) =>
    invoke<Task>("toggle_task_favorite", {
      request: { taskId, isFavorite },
    }),
  archiveTask: (taskId: string) =>
    invoke<Task>("archive_task", { request: { taskId } }),
  restoreArchivedTask: (taskId: string) =>
    invoke<Task>("restore_archived_task", { request: { taskId } }),
  deleteTask: (taskId: string) =>
    invoke<void>("delete_task", { request: { taskId } }),
  deleteSubtask: (subtaskId: string) =>
    invoke<void>("delete_subtask", { request: { subtaskId } }),
  updateNotificationDisplayMode: (displayMode: NotificationDisplayMode) =>
    invoke<NotificationDisplayMode>("update_notification_display_mode", {
      request: { displayMode },
    }),
  getNotificationsEnabled: () => invoke<boolean>("get_notifications_enabled"),
  updateNotificationsEnabled: (enabled: boolean) =>
    invoke<boolean>("update_notifications_enabled", {
      request: { enabled },
    }),
  getNextPendingNotification: () =>
    invoke<NextNotificationSchedule | null>("get_next_pending_notification"),
  syncNotifications: () =>
    invoke<NotificationSyncResult>("sync_notifications"),
  processNativeNotificationRegistrations: () =>
    invoke<NativeNotificationRegistrationSummary>(
      "process_notification_os_registrations",
    ),
  dispatchDueNotifications: () =>
    invoke<NotificationDispatchSummary>("dispatch_due_notifications"),
  listNotificationFailureHistory: () =>
    invoke<NotificationDeliveryAttempt[]>("list_notification_failure_history"),
  createSqliteBackup: (destinationDir: string) =>
    invoke<SqliteBackupResult>("create_sqlite_backup", {
      request: { destinationDir },
    }),
  restoreSqliteBackup: (backupDir: string) =>
    invoke<SqliteRestoreResult>("restore_sqlite_backup", {
      request: { backupDir },
    }),
  createJsonExport: (destinationDir: string) =>
    invoke<DataExportResult>("create_json_export", {
      request: { destinationDir },
    }),
  createCsvExport: (destinationDir: string) =>
    invoke<DataExportResult>("create_csv_export", {
      request: { destinationDir },
    }),
};
