import type {
  ScheduledTaskDraft,
  WorkItemDraft,
  WorkScheduleDraft,
} from "../application/usecases/contracts";

export type TaskCreatePreset =
  | {
      kind: "standard";
      listId: string;
      plannedStartDate: string | null;
      dueDate: string | null;
      dueTime: string | null;
      sourceLabel: string;
      boardColumnId?: string | null;
    }
  | {
      kind: "scheduled";
      listId: string;
      schedule: WorkScheduleDraft;
      sourceLabel: string;
    }
  | {
      kind: "subtask";
      taskId: string;
      parentTitle: string;
      listId: string;
      dueDate: string | null;
      dueTime: string | null;
      sourceLabel: string;
    };

export type TaskCreateSubmission =
  | {
      kind: "standard";
      input: WorkItemDraft;
      boardColumnId?: string | null;
    }
  | {
      kind: "scheduled";
      input: ScheduledTaskDraft;
    }
  | {
      kind: "subtask";
      taskId: string;
      input: WorkItemDraft;
    };

export type CalendarTaskCreatePreset = Pick<
  Extract<TaskCreatePreset, { kind: "scheduled" }>,
  "schedule" | "sourceLabel"
>;
