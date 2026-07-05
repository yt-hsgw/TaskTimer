import { invoke } from "@tauri-apps/api/core";
import type {
  TaskTimerGateway,
  WeekCalendarItem,
} from "../../application/usecases/contracts";
import type { ActiveTimer } from "../../domain/timer/types";
import type { NotificationDisplayMode } from "../../domain/notification/types";

export const tauriTaskTimerGateway: TaskTimerGateway = {
  healthCheck: () => invoke<string>("health_check"),
  listWeekCalendarItems: (weekStartDate) =>
    invoke<WeekCalendarItem[]>("list_week_calendar_items", { weekStartDate }),
  getActiveTimer: () => invoke<ActiveTimer | null>("get_active_timer"),
  getNotificationDisplayMode: () =>
    invoke<NotificationDisplayMode>("get_notification_display_mode"),
};

