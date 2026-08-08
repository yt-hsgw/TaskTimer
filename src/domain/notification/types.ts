/**
 * ローカル通知ルールと表示設定のドメイン型。
 * タスク名やメモ本文を不要なDTOやログへ含めない。
 *
 * @packageDocumentation
 */
import type { WorkTargetRef } from "../task/types";

export type NotificationKind = "planned_start" | "due";
export type NotificationDisplayMode = "title_only" | "generic";
export type NotificationRegistrationStatus =
  | "pending"
  | "registered"
  | "failed"
  | "disabled";

export type NotificationRule = {
  id: string;
  target: WorkTargetRef;
  kind: NotificationKind;
  notifyAt: string;
  enabled: boolean;
  registrationStatus: NotificationRegistrationStatus;
  lastError: string | null;
  deletedAt: string | null;
  createdAt: string;
  updatedAt: string;
};

export type NotificationPreference = {
  displayMode: NotificationDisplayMode;
  notificationsEnabled: boolean;
  createdAt: string;
  updatedAt: string;
};
