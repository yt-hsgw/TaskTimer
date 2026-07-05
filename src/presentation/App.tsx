import { useEffect, useMemo, useState } from "react";
import type { WeekCalendarItem } from "../application/usecases/contracts";
import type { NotificationDisplayMode } from "../domain/notification/types";
import { tauriTaskTimerGateway } from "../infrastructure/tauri/gateway";
import { WeekCalendar } from "./components/WeekCalendar";
import { TaskPanel } from "./components/TaskPanel";
import { SettingsPanel } from "./components/SettingsPanel";

const fallbackItems: WeekCalendarItem[] = [
  {
    id: "calendar-1",
    target: { type: "task", id: "task-1" },
    title: "設計レビュー",
    date: "2026-07-06",
    marker: "planned_start",
    status: "in_progress",
  },
  {
    id: "calendar-2",
    target: { type: "subtask", id: "subtask-1" },
    title: "SQLiteマイグレーション方針",
    date: "2026-07-07",
    marker: "due",
    status: "todo",
  },
];

export function App() {
  const [health, setHealth] = useState("frontend-only");
  const [items, setItems] = useState<WeekCalendarItem[]>(fallbackItems);
  const [displayMode, setDisplayMode] =
    useState<NotificationDisplayMode>("title_only");

  const weekStartDate = useMemo(() => "2026-07-06", []);
  const runtimeLabel = useMemo(() => {
    if (health === "tauri-ready") {
      return "Tauri接続済み";
    }
    if (health === "tauri-unavailable") {
      return "Tauri未接続";
    }
    if (health === "frontend-only") {
      return "フロントエンドのみ";
    }
    return health;
  }, [health]);

  useEffect(() => {
    void tauriTaskTimerGateway
      .healthCheck()
      .then(setHealth)
      .catch(() => setHealth("tauri-unavailable"));

    void tauriTaskTimerGateway
      .listWeekCalendarItems(weekStartDate)
      .then((nextItems) => {
        if (nextItems.length > 0) {
          setItems(nextItems);
        }
      })
      .catch(() => setItems(fallbackItems));

    void tauriTaskTimerGateway
      .getNotificationDisplayMode()
      .then(setDisplayMode)
      .catch(() => setDisplayMode("title_only"));
  }, [weekStartDate]);

  return (
    <main className="app-shell">
      <header className="top-bar">
        <div>
          <p className="eyebrow">オフライン対応デスクトップタスクタイマー</p>
          <h1>TaskTimer</h1>
        </div>
        <div className="runtime-status">
          <span>実行環境</span>
          <strong>{runtimeLabel}</strong>
        </div>
      </header>

      <section className="workspace-grid" aria-label="TaskTimer作業画面">
        <TaskPanel />
        <WeekCalendar weekStartDate={weekStartDate} items={items} />
        <SettingsPanel displayMode={displayMode} />
      </section>
    </main>
  );
}
