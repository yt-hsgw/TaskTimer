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
    };

export type CalendarTaskCreatePreset = Pick<
  Extract<TaskCreatePreset, { kind: "scheduled" }>,
  "schedule" | "sourceLabel"
>;
