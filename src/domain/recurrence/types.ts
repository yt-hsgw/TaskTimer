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
