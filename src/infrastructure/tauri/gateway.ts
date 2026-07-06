import { invoke } from "@tauri-apps/api/core";
import type {
  CreateSubtaskDraft,
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
};
