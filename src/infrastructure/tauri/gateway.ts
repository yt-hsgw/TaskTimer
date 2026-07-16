import { invoke } from "@tauri-apps/api/core";
import type {
  CreateSubtaskDraft,
  DataExportResult,
  NotificationDeliveryAttempt,
  NotificationDispatchSummary,
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
  listTags: () => invoke<TagItem[]>("list_tags"),
  listTaskRows: (listId?: string | null) =>
    invoke<TaskRow[]>("list_task_rows", { listId: listId ?? null }),
  listArchivedTaskRows: () => invoke<TaskRow[]>("list_archived_task_rows"),
  listCalendarItems: (startDate, endDate) =>
    invoke<WeekCalendarItem[]>("list_calendar_items", { startDate, endDate }),
  listWeekCalendarItems: (weekStartDate) =>
    invoke<WeekCalendarItem[]>("list_week_calendar_items", { weekStartDate }),
  getActiveTimer: () => invoke<ActiveTimer | null>("get_active_timer"),
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
