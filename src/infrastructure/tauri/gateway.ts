import { invoke } from "@tauri-apps/api/core";
import type {
  CreateSubtaskDraft,
  NotificationDispatchSummary,
  TaskTimerGateway,
  TaskListItem,
  TaskRow,
  TaskWithSubtasks,
  UpdateSubtaskDraft,
  UpdateTaskDraft,
  WeekCalendarItem,
  WorkItemDraft,
} from "../../application/usecases/contracts";
import type { ActiveTimer, TimerSession } from "../../domain/timer/types";
import type {
  NotificationDisplayMode,
  NotificationRule,
} from "../../domain/notification/types";
import type { Subtask, Task, WorkTargetRef } from "../../domain/task/types";

export const tauriTaskTimerGateway: TaskTimerGateway = {
  healthCheck: () => invoke<string>("health_check"),
  listTasks: () => invoke<TaskWithSubtasks[]>("list_tasks"),
  listTaskLists: () => invoke<TaskListItem[]>("list_task_lists"),
  listTaskRows: (listId?: string | null) =>
    invoke<TaskRow[]>("list_task_rows", { listId: listId ?? null }),
  listCalendarItems: (startDate, endDate) =>
    invoke<WeekCalendarItem[]>("list_calendar_items", { startDate, endDate }),
  listWeekCalendarItems: (weekStartDate) =>
    invoke<WeekCalendarItem[]>("list_week_calendar_items", { weekStartDate }),
  getActiveTimer: () => invoke<ActiveTimer | null>("get_active_timer"),
  getNotificationDisplayMode: () =>
    invoke<NotificationDisplayMode>("get_notification_display_mode"),
  createTask: (input: WorkItemDraft) =>
    invoke<Task>("create_task", { request: input }),
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
  reopenTask: (taskId: string) =>
    invoke<Task>("reopen_task", { request: { taskId } }),
  completeSubtask: (subtaskId: string) =>
    invoke<Subtask>("complete_subtask", { request: { subtaskId } }),
  toggleTaskFavorite: (taskId: string, isFavorite: boolean) =>
    invoke<Task>("toggle_task_favorite", {
      request: { taskId, isFavorite },
    }),
  deleteTask: (taskId: string) =>
    invoke<void>("delete_task", { request: { taskId } }),
  deleteSubtask: (subtaskId: string) =>
    invoke<void>("delete_subtask", { request: { subtaskId } }),
  updateNotificationDisplayMode: (displayMode: NotificationDisplayMode) =>
    invoke<NotificationDisplayMode>("update_notification_display_mode", {
      request: { displayMode },
    }),
  setNotificationRuleEnabled: (ruleId: string, enabled: boolean) =>
    invoke<NotificationRule>("set_notification_rule_enabled", {
      request: { ruleId, enabled },
    }),
  dispatchDueNotifications: () =>
    invoke<NotificationDispatchSummary>("dispatch_due_notifications"),
};
