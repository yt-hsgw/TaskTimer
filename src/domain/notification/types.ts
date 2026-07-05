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
  createdAt: string;
  updatedAt: string;
};

