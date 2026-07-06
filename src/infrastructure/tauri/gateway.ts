import { invoke } from "@tauri-apps/api/core";
import type {
  CreateSubtaskDraft,
  NotificationDispatchSummary,
  TaskTimerGateway,
  TaskWithSubtasks,
  WeekCalendarItem,
  WorkItemDraft,
} from "../../application/usecases/contracts";
import type { ActiveTimer, TimerSession } from "../../domain/timer/types";
import type { NotificationDisplayMode } from "../../domain/notification/types";
import type { Subtask, Task, WorkTargetRef } from "../../domain/task/types";

export const tauriTaskTimerGateway: TaskTimerGateway = {
  healthCheck: () => invoke<string>("health_check"),
  listTasks: () => invoke<TaskWithSubtasks[]>("list_tasks"),
  listWeekCalendarItems: (weekStartDate) =>
    invoke<WeekCalendarItem[]>("list_week_calendar_items", { weekStartDate }),
  getActiveTimer: () => invoke<ActiveTimer | null>("get_active_timer"),
  getNotificationDisplayMode: () =>
    invoke<NotificationDisplayMode>("get_notification_display_mode"),
  createTask: (input: WorkItemDraft) =>
    invoke<Task>("create_task", { request: input }),
  createSubtask: (input: CreateSubtaskDraft) =>
    invoke<Subtask>("create_subtask", { request: input }),
  startTimer: (target: WorkTargetRef) =>
    invoke<ActiveTimer>("start_timer", { request: { target } }),
  stopActiveTimer: () => invoke<TimerSession>("stop_active_timer"),
  completeTask: (taskId: string, allowIncompleteSubtasks: boolean) =>
    invoke<Task>("complete_task", {
      request: { taskId, allowIncompleteSubtasks },
    }),
  completeSubtask: (subtaskId: string) =>
    invoke<Subtask>("complete_subtask", { request: { subtaskId } }),
  deleteTask: (taskId: string) =>
    invoke<void>("delete_task", { request: { taskId } }),
  deleteSubtask: (subtaskId: string) =>
    invoke<void>("delete_subtask", { request: { subtaskId } }),
  updateNotificationDisplayMode: (displayMode: NotificationDisplayMode) =>
    invoke<NotificationDisplayMode>("update_notification_display_mode", {
      request: { displayMode },
    }),
  dispatchDueNotifications: () =>
    invoke<NotificationDispatchSummary>("dispatch_due_notifications"),
};
