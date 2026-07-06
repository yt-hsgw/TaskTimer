import type { NotificationDisplayMode } from "./types";

export type NotificationContent = {
  title: string;
  body: string;
};

export function buildNotificationContent(
  displayMode: NotificationDisplayMode,
  title: string,
): NotificationContent {
  if (displayMode === "generic") {
    return {
      title: "TaskTimer",
      body: "予定時刻です",
    };
  }
  return {
    title: title.trim(),
    body: "",
  };
}
