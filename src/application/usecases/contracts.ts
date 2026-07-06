import type { ActiveTimer, TimerSession } from "../../domain/timer/types";
import type { NotificationDisplayMode } from "../../domain/notification/types";
import type { Subtask, Task, WorkTargetRef } from "../../domain/task/types";

export type WorkItemDraft = {
  title: string;
  plannedStartDate?: string | null;
  dueDate?: string | null;
  memo?: string | null;
};

export type CreateSubtaskDraft = WorkItemDraft & {
  taskId: string;
};

export type WeekCalendarItem = {
  id: string;
  target: WorkTargetRef;
  title: string;
  date: string;
  marker: "planned_start" | "due" | "active_timer";
  status: Task["status"];
};

export type TaskTimerGateway = {
  healthCheck(): Promise<string>;
  listWeekCalendarItems(weekStartDate: string): Promise<WeekCalendarItem[]>;
  getActiveTimer(): Promise<ActiveTimer | null>;
  getNotificationDisplayMode(): Promise<NotificationDisplayMode>;
  createTask(input: WorkItemDraft): Promise<Task>;
  createSubtask(input: CreateSubtaskDraft): Promise<Subtask>;
  startTimer(target: WorkTargetRef): Promise<ActiveTimer>;
  stopActiveTimer(): Promise<TimerSession>;
};

export type TaskWithSubtasks = Task & {
  subtasks: Subtask[];
};
