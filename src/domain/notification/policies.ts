import type { NotificationDisplayMode } from "./types";

export function buildNotificationBody(
  displayMode: NotificationDisplayMode,
  title: string,
): string {
  if (displayMode === "generic") {
    return "TaskTimerの予定時刻です";
  }
  return title.trim();
}

