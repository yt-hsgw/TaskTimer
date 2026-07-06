import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  NotificationDispatchSummary,
  TaskWithSubtasks,
  WeekCalendarItem,
  WorkItemDraft,
} from "../application/usecases/contracts";
import type { NotificationDisplayMode } from "../domain/notification/types";
import type { ActiveTimer, TimerSession } from "../domain/timer/types";
import type { Subtask, WorkTargetRef } from "../domain/task/types";
import { tauriTaskTimerGateway } from "../infrastructure/tauri/gateway";
import { WeekCalendar } from "./components/WeekCalendar";
import { TaskPanel } from "./components/TaskPanel";
import { SettingsPanel } from "./components/SettingsPanel";

export function App() {
  const [health, setHealth] = useState("frontend-only");
  const [tasks, setTasks] = useState<TaskWithSubtasks[]>([]);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [items, setItems] = useState<WeekCalendarItem[]>([]);
  const [activeTimer, setActiveTimer] = useState<ActiveTimer | null>(null);
  const [displayMode, setDisplayMode] =
    useState<NotificationDisplayMode>("title_only");
  const [notificationSummary, setNotificationSummary] =
    useState<NotificationDispatchSummary | null>(null);
  const [weekStartDate, setWeekStartDate] = useState(getCurrentWeekStartDate);
  const [isLoading, setIsLoading] = useState(true);
  const [isMutating, setIsMutating] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

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

  const loadSnapshot = useCallback(async () => {
    setIsLoading(true);
    setErrorMessage(null);

    try {
      const nextHealth = await tauriTaskTimerGateway.healthCheck();
      const [nextTasks, nextItems, nextActiveTimer, nextDisplayMode] =
        await Promise.all([
          tauriTaskTimerGateway.listTasks(),
          tauriTaskTimerGateway.listWeekCalendarItems(weekStartDate),
          tauriTaskTimerGateway.getActiveTimer(),
          tauriTaskTimerGateway.getNotificationDisplayMode(),
        ]);

      setHealth(nextHealth);
      setTasks(nextTasks);
      setItems(nextItems);
      setActiveTimer(nextActiveTimer);
      setDisplayMode(nextDisplayMode);
      setNotificationSummary(
        await tauriTaskTimerGateway.dispatchDueNotifications(),
      );
      setSelectedTaskId((currentTaskId) => {
        if (nextTasks.some((task) => task.id === currentTaskId)) {
          return currentTaskId;
        }
        return nextTasks[0]?.id ?? null;
      });
    } catch (error) {
      setHealth("tauri-unavailable");
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsLoading(false);
    }
  }, [weekStartDate]);

  useEffect(() => {
    void loadSnapshot();
  }, [loadSnapshot]);

  const runMutation = useCallback(
    async (operation: () => Promise<string | void>) => {
      setIsMutating(true);
      setErrorMessage(null);

      try {
        const nextSelectedTaskId = await operation();
        await loadSnapshot();
        if (nextSelectedTaskId) {
          setSelectedTaskId(nextSelectedTaskId);
        }
        return true;
      } catch (error) {
        setErrorMessage(toErrorMessage(error));
        return false;
      } finally {
        setIsMutating(false);
      }
    },
    [loadSnapshot],
  );

  const handleCreateTask = useCallback(
    (input: WorkItemDraft) =>
      runMutation(async () => {
        const task = await tauriTaskTimerGateway.createTask(input);
        return task.id;
      }),
    [runMutation],
  );

  const handleCreateSubtask = useCallback(
    (taskId: string, input: WorkItemDraft) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.createSubtask({ ...input, taskId });
        return taskId;
      }),
    [runMutation],
  );

  const handleStartTimer = useCallback(
    (target: WorkTargetRef) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.startTimer(target);
        return target.type === "task" ? target.id : undefined;
      }),
    [runMutation],
  );

  const handleStopTimer = useCallback(
    () =>
      runMutation(async () => {
        const stoppedTimer: TimerSession =
          await tauriTaskTimerGateway.stopActiveTimer();
        return stoppedTimer.target.type === "task"
          ? stoppedTimer.target.id
          : undefined;
      }),
    [runMutation],
  );

  const handleCompleteTask = useCallback(
    (task: TaskWithSubtasks) => {
      const hasIncompleteSubtasks = task.subtasks.some(
        (subtask) => subtask.status !== "done",
      );
      if (
        hasIncompleteSubtasks &&
        !window.confirm(
          "未完了のサブタスクがあります。サブタスクは未完了のまま、親タスクだけ完了しますか？",
        )
      ) {
        return Promise.resolve(false);
      }

      return runMutation(async () => {
        await tauriTaskTimerGateway.completeTask(task.id, hasIncompleteSubtasks);
        return task.id;
      });
    },
    [runMutation],
  );

  const handleCompleteSubtask = useCallback(
    (subtask: Subtask) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.completeSubtask(subtask.id);
        return subtask.taskId;
      }),
    [runMutation],
  );

  const handleDeleteTask = useCallback(
    (task: TaskWithSubtasks) => {
      if (
        !window.confirm(
          "このタスクを削除します。サブタスク、タイマー履歴、通知ルールもソフト削除されます。",
        )
      ) {
        return Promise.resolve(false);
      }

      return runMutation(async () => {
        await tauriTaskTimerGateway.deleteTask(task.id);
      });
    },
    [runMutation],
  );

  const handleDeleteSubtask = useCallback(
    (subtask: Subtask) => {
      if (
        !window.confirm(
          "このサブタスクを削除します。タイマー履歴と通知ルールもソフト削除されます。",
        )
      ) {
        return Promise.resolve(false);
      }

      return runMutation(async () => {
        await tauriTaskTimerGateway.deleteSubtask(subtask.id);
        return subtask.taskId;
      });
    },
    [runMutation],
  );

  const handleUpdateNotificationDisplayMode = useCallback(
    (nextDisplayMode: NotificationDisplayMode) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.updateNotificationDisplayMode(nextDisplayMode);
      }),
    [runMutation],
  );

  const handleRetryNotifications = useCallback(
    () =>
      runMutation(async () => {
        setNotificationSummary(
          await tauriTaskTimerGateway.dispatchDueNotifications(),
        );
      }),
    [runMutation],
  );

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

      {errorMessage ? (
        <div className="app-alert" role="alert">
          <strong>処理に失敗しました</strong>
          <span>{errorMessage}</span>
          <button type="button" onClick={() => void loadSnapshot()}>
            再読み込み
          </button>
        </div>
      ) : null}

      <section className="workspace-grid" aria-label="TaskTimer作業画面">
        <TaskPanel
          tasks={tasks}
          selectedTaskId={selectedTaskId}
          activeTimer={activeTimer}
          isLoading={isLoading}
          isMutating={isMutating}
          onSelectTask={setSelectedTaskId}
          onCreateTask={handleCreateTask}
          onCreateSubtask={handleCreateSubtask}
          onStartTimer={handleStartTimer}
          onStopTimer={handleStopTimer}
          onCompleteTask={handleCompleteTask}
          onCompleteSubtask={handleCompleteSubtask}
          onDeleteTask={handleDeleteTask}
          onDeleteSubtask={handleDeleteSubtask}
        />
        <WeekCalendar
          weekStartDate={weekStartDate}
          items={items}
          isLoading={isLoading}
          onPreviousWeek={() =>
            setWeekStartDate((current) => shiftDate(current, -7))
          }
          onNextWeek={() => setWeekStartDate((current) => shiftDate(current, 7))}
        />
        <SettingsPanel
          displayMode={displayMode}
          isMutating={isMutating}
          notificationSummary={notificationSummary}
          onUpdateDisplayMode={handleUpdateNotificationDisplayMode}
          onRetryNotifications={handleRetryNotifications}
        />
      </section>
    </main>
  );
}

function getCurrentWeekStartDate() {
  const today = new Date();
  const mondayBasedDay = (today.getDay() + 6) % 7;
  today.setDate(today.getDate() - mondayBasedDay);
  return toDateInputValue(today);
}

function shiftDate(value: string, days: number) {
  const date = parseDateInputValue(value);
  date.setDate(date.getDate() + days);
  return toDateInputValue(date);
}

function parseDateInputValue(value: string) {
  const [year, month, day] = value.split("-").map(Number);
  return new Date(year, month - 1, day);
}

function toDateInputValue(date: Date) {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function toErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "string") {
    return error;
  }
  return "不明なエラーが発生しました";
}
