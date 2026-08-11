/**
 * タスクとサブタスクで共有する繰り返しルールのドメイン型。
 *
 * @packageDocumentation
 */
export type RecurrenceFrequency = "daily" | "weekly" | "monthly";

export type RecurrenceRule = {
  id: string;
  target: {
    type: "task" | "subtask";
    id: string;
  };
  frequency: RecurrenceFrequency;
  interval: number;
  deletedAt: string | null;
  createdAt: string;
  updatedAt: string;
};
