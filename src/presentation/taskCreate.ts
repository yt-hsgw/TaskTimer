import type {
  ScheduledTaskDraft,
  WorkItemDraft,
  WorkScheduleDraft,
} from "../application/usecases/contracts";

export type TaskCreatePreset =
  | {
      kind: "standard";
      listId: string;
      dueDate: string | null;
      dueTime: string | null;
      sourceLabel: string;
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
    }
  | {
      kind: "scheduled";
      input: ScheduledTaskDraft;
    };

export type CalendarTaskCreatePreset = Pick<
  Extract<TaskCreatePreset, { kind: "scheduled" }>,
  "schedule" | "sourceLabel"
>;
