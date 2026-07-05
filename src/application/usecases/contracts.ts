import type { ActiveTimer } from "../../domain/timer/types";
import type { NotificationDisplayMode } from "../../domain/notification/types";
import type { Subtask, Task, WorkTargetRef } from "../../domain/task/types";

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
};

export type TaskWithSubtasks = Task & {
  subtasks: Subtask[];
};

