import type { WorkStatus } from "./types";

export type DateRangeInput = {
  plannedStartDate: string | null;
  dueDate: string | null;
};

export function validateTitle(title: string): string {
  const trimmed = title.trim();
  if (trimmed.length === 0) {
    throw new Error("タイトルは必須です。");
  }
  if (trimmed.length > 120) {
    throw new Error("タイトルは120文字以内で入力してください。");
  }
  return trimmed;
}

export function validateDateRange(input: DateRangeInput): void {
  if (
    input.plannedStartDate !== null &&
    input.dueDate !== null &&
    input.dueDate < input.plannedStartDate
  ) {
    throw new Error("期限日は開始予定日より前にできません。");
  }
}

export function assertTimerStartable(status: WorkStatus): void {
  if (status === "done" || status === "archived") {
    throw new Error("完了済みまたはアーカイブ済みの対象はタイマーを開始できません。");
  }
}
