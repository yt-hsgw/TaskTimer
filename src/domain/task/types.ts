import type { RecurrenceRule } from "../recurrence/types";
import type { NotificationRule } from "../notification/types";

export type WorkStatus = "todo" | "in_progress" | "done" | "archived";
export type WorkTargetType = "task" | "subtask";

export type Task = {
  id: string;
  listId: string;
  title: string;
  status: WorkStatus;
  isFavorite: boolean;
  plannedStartDate: string | null;
  dueDate: string | null;
  timerTargetSeconds: number | null;
  recurrenceRule: RecurrenceRule | null;
  notificationRules: NotificationRule[];
  memo: string;
  sortOrder: number;
  completedAt: string | null;
  deletedAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type Subtask = {
  id: string;
  taskId: string;
  title: string;
  status: WorkStatus;
  plannedStartDate: string | null;
  dueDate: string | null;
  timerTargetSeconds: number | null;
  recurrenceRule: RecurrenceRule | null;
  notificationRules: NotificationRule[];
  memo: string;
  sortOrder: number;
  completedAt: string | null;
  deletedAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type WorkTargetRef = {
  type: WorkTargetType;
  id: string;
};
