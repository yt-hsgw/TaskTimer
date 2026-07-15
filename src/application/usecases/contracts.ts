import type { ActiveTimer, TimerSession } from "../../domain/timer/types";
import type { NotificationDisplayMode } from "../../domain/notification/types";
import type { RecurrenceFrequency } from "../../domain/recurrence/types";
import type { Subtask, Task, WorkTargetRef } from "../../domain/task/types";

export type WorkItemDraft = {
  listId?: string | null;
  title: string;
  plannedStartDate?: string | null;
  dueDate?: string | null;
  dueTime?: string | null;
  memo?: string | null;
};

export type WorkItemUpdateDraft = WorkItemDraft & {
  timerTargetSeconds?: number | null;
  recurrenceRule?: RecurrenceRuleDraft | null;
};

export type RecurrenceRuleDraft = {
  frequency: RecurrenceFrequency;
  interval: number;
};

export type TaskListDraft = {
  name: string;
};

export type CreateSubtaskDraft = WorkItemDraft & {
  taskId: string;
};

export type UpdateTaskDraft = WorkItemUpdateDraft & {
  taskId: string;
};

export type UpdateSubtaskDraft = WorkItemUpdateDraft & {
  subtaskId: string;
};

export type WeekCalendarItem = {
  id: string;
  target: WorkTargetRef;
  title: string;
  parentTitle: string | null;
  date: string;
  time: string | null;
  marker: "planned_start" | "due" | "active_timer";
  status: Task["status"];
};

export type NotificationDispatchSummary = {
  attempted: number;
  succeeded: number;
  failed: number;
  lastError: string | null;
};

export type NotificationDeliveryAttempt = {
  id: string;
  notificationRuleId: string;
  target: WorkTargetRef;
  kind: "planned_start" | "due";
  notifyAt: string;
  attemptedAt: string;
  result: "success" | "failed";
  errorMessage: string | null;
  attemptCount: number;
};

export type SqliteBackupManifest = {
  format: string;
  formatVersion: number;
  appVersion: string;
  schemaVersion: number;
  createdAt: string;
  platform: string;
  databaseFile: string;
  integrityCheck: string;
};

export type SqliteBackupResult = {
  backupDir: string;
  databaseFile: string;
  manifestFile: string;
  manifest: SqliteBackupManifest;
};

export type SqliteRestoreResult = {
  backupDir: string;
  restoredAt: string;
  previousDatabaseFile: string;
  manifest: SqliteBackupManifest;
};

export type DataExportManifest = {
  format: string;
  formatVersion: number;
  appVersion: string;
  createdAt: string;
  platform: string;
  compatibility: string;
  containsPersonalData: boolean;
};

export type DataExportResult = {
  exportPath: string;
  files: string[];
  manifest: DataExportManifest;
};

export type TaskListItem = {
  id: string;
  name: string;
  sortOrder: number;
  taskCount: number;
  activeTaskCount: number;
  completedTaskCount: number;
  createdAt: string;
  updatedAt: string;
};

export type TaskRow = {
  id: string;
  listId: string;
  title: string;
  status: Task["status"];
  isFavorite: boolean;
  plannedStartDate: string | null;
  dueDate: string | null;
  dueTime: string | null;
  timerTargetSeconds: number | null;
  sortOrder: number;
  completedAt: string | null;
  createdAt: string;
  updatedAt: string;
  subtaskTotalCount: number;
  completedSubtaskCount: number;
  activeTimerTarget: WorkTargetRef | null;
  isTimerActive: boolean;
};

export type TaskTimerGateway = {
  healthCheck(): Promise<string>;
  listTasks(): Promise<TaskWithSubtasks[]>;
  listTaskLists(): Promise<TaskListItem[]>;
  listTaskRows(listId?: string | null): Promise<TaskRow[]>;
  listArchivedTaskRows(): Promise<TaskRow[]>;
  listCalendarItems(startDate: string, endDate: string): Promise<WeekCalendarItem[]>;
  listWeekCalendarItems(weekStartDate: string): Promise<WeekCalendarItem[]>;
  getActiveTimer(): Promise<ActiveTimer | null>;
  getNotificationDisplayMode(): Promise<NotificationDisplayMode>;
  createTask(input: WorkItemDraft): Promise<Task>;
  createTaskList(input: TaskListDraft): Promise<TaskListItem>;
  updateTaskList(listId: string, input: TaskListDraft): Promise<TaskListItem>;
  deleteTaskList(listId: string): Promise<void>;
  createSubtask(input: CreateSubtaskDraft): Promise<Subtask>;
  updateTask(input: UpdateTaskDraft): Promise<Task>;
  updateSubtask(input: UpdateSubtaskDraft): Promise<Subtask>;
  startTimer(target: WorkTargetRef): Promise<ActiveTimer>;
  pauseActiveTimer(): Promise<ActiveTimer>;
  resumeActiveTimer(): Promise<ActiveTimer>;
  stopActiveTimer(): Promise<TimerSession>;
  completeTask(taskId: string, allowIncompleteSubtasks: boolean): Promise<Task>;
  reopenTask(taskId: string): Promise<Task>;
  completeSubtask(subtaskId: string): Promise<Subtask>;
  reopenSubtask(subtaskId: string): Promise<Subtask>;
  toggleTaskFavorite(taskId: string, isFavorite: boolean): Promise<Task>;
  archiveTask(taskId: string): Promise<Task>;
  restoreArchivedTask(taskId: string): Promise<Task>;
  deleteTask(taskId: string): Promise<void>;
  deleteSubtask(subtaskId: string): Promise<void>;
  updateNotificationDisplayMode(
    displayMode: NotificationDisplayMode,
  ): Promise<NotificationDisplayMode>;
  getNotificationsEnabled(): Promise<boolean>;
  updateNotificationsEnabled(enabled: boolean): Promise<boolean>;
  dispatchDueNotifications(): Promise<NotificationDispatchSummary>;
  listNotificationFailureHistory(): Promise<NotificationDeliveryAttempt[]>;
  createSqliteBackup(destinationDir: string): Promise<SqliteBackupResult>;
  restoreSqliteBackup(backupDir: string): Promise<SqliteRestoreResult>;
  createJsonExport(destinationDir: string): Promise<DataExportResult>;
  createCsvExport(destinationDir: string): Promise<DataExportResult>;
};

export type TaskWithSubtasks = Task & {
  subtasks: Subtask[];
};
