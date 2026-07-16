import type { RecurrenceRule } from "../recurrence/types";

export const DEFAULT_TASK_LIST_ID = "default";
export const DEFAULT_TASK_LIST_NAME = "タスク";

export type WorkStatus = "todo" | "in_progress" | "done" | "archived";
export type WorkTargetType = "task" | "subtask";

export type TaskTag = {
  id: string;
  name: string;
};

export type Task = {
  id: string;
  listId: string;
  title: string;
  status: WorkStatus;
  isFavorite: boolean;
  plannedStartDate: string | null;
  dueDate: string | null;
  dueTime: string | null;
  timerTargetSeconds: number | null;
  recurrenceRule: RecurrenceRule | null;
  memo: string;
  sortOrder: number;
  completedAt: string | null;
  deletedAt: string | null;
  createdAt: string;
  updatedAt: string;
  tags: TaskTag[];
};

export type Subtask = {
  id: string;
  taskId: string;
  title: string;
  status: WorkStatus;
  plannedStartDate: string | null;
  dueDate: string | null;
  dueTime: string | null;
  timerTargetSeconds: number | null;
  recurrenceRule: RecurrenceRule | null;
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
