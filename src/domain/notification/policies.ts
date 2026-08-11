/**
 * 通知表示設定からOSへ渡す安全な文面を組み立てるドメインポリシー。
 * 汎用表示ではユーザー入力本文を返さない。
 *
 * @packageDocumentation
 */
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
