import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  NotificationDispatchSummary,
  TaskListItem,
  TaskRow,
  TaskWithSubtasks,
  WeekCalendarItem,
  WorkItemDraft,
  WorkItemUpdateDraft,
} from "../application/usecases/contracts";
import type { NotificationDisplayMode } from "../domain/notification/types";
import type { ActiveTimer, TimerSession } from "../domain/timer/types";
import type { Subtask, WorkTargetRef } from "../domain/task/types";
import { tauriTaskTimerGateway } from "../infrastructure/tauri/gateway";
import { WeekCalendar } from "./components/WeekCalendar";
import { TaskPanel } from "./components/TaskPanel";
import { TaskDetailPane } from "./components/TaskDetailPane";
import { SettingsPanel } from "./components/SettingsPanel";
import { LeftNavigation, type AppView } from "./components/LeftNavigation";

export function App() {
  const [health, setHealth] = useState("frontend-only");
  const [tasks, setTasks] = useState<TaskWithSubtasks[]>([]);
  const [taskRows, setTaskRows] = useState<TaskRow[]>([]);
  const [taskLists, setTaskLists] = useState<TaskListItem[]>([]);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [items, setItems] = useState<WeekCalendarItem[]>([]);
  const [activeTimer, setActiveTimer] = useState<ActiveTimer | null>(null);
  const [activeView, setActiveView] = useState<AppView>({
    kind: "list",
    listId: "default",
  });
  const [isNavigationOpen, setIsNavigationOpen] = useState(true);
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

  const favoriteCount = useMemo(
    () => tasks.filter((task) => task.isFavorite).length,
    [tasks],
  );

  const activeTaskList = useMemo(() => {
    if (activeView.kind !== "list") {
      return null;
    }
    return taskLists.find((list) => list.id === activeView.listId) ?? null;
  }, [activeView, taskLists]);

  const visibleTasks = useMemo(() => {
    if (activeView.kind === "favorites") {
      return tasks.filter((task) => task.isFavorite);
    }
    if (activeView.kind === "list") {
      return tasks.filter((task) => task.listId === activeView.listId);
    }
    return tasks;
  }, [activeView, tasks]);

  const visibleTaskRows = useMemo(() => {
    if (activeView.kind === "favorites") {
      return taskRows.filter((task) => task.isFavorite);
    }
    if (activeView.kind === "list") {
      return taskRows.filter((task) => task.listId === activeView.listId);
    }
    return taskRows;
  }, [activeView, taskRows]);

  const selectedTask = useMemo(() => {
    if (!selectedTaskId) {
      return null;
    }
    return visibleTasks.find((task) => task.id === selectedTaskId) ?? null;
  }, [selectedTaskId, visibleTasks]);

  const loadSnapshot = useCallback(async () => {
    setIsLoading(true);
    setErrorMessage(null);

    try {
      const nextHealth = await tauriTaskTimerGateway.healthCheck();
      const listId =
        activeView.kind === "list" ? activeView.listId : undefined;
      const [
        nextTasks,
        nextTaskRows,
        nextTaskLists,
        nextItems,
        nextActiveTimer,
        nextDisplayMode,
      ] =
        await Promise.all([
          tauriTaskTimerGateway.listTasks(),
          tauriTaskTimerGateway.listTaskRows(listId),
          tauriTaskTimerGateway.listTaskLists(),
          tauriTaskTimerGateway.listWeekCalendarItems(weekStartDate),
          tauriTaskTimerGateway.getActiveTimer(),
          tauriTaskTimerGateway.getNotificationDisplayMode(),
        ]);

      setHealth(nextHealth);
      setTasks(nextTasks);
      setTaskRows(nextTaskRows);
      setTaskLists(nextTaskLists);
      setItems(nextItems);
      setActiveTimer(nextActiveTimer);
      setDisplayMode(nextDisplayMode);
      setNotificationSummary(
        await tauriTaskTimerGateway.dispatchDueNotifications(),
      );
    } catch (error) {
      setHealth("tauri-unavailable");
      setErrorMessage(toErrorMessage(error));
    } finally {
      setIsLoading(false);
    }
  }, [activeView, weekStartDate]);

  useEffect(() => {
    void loadSnapshot();
  }, [loadSnapshot]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.ctrlKey && event.key.toLowerCase() === "b") {
        event.preventDefault();
        setIsNavigationOpen((current) => !current);
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  useEffect(() => {
    if (activeView.kind !== "list" || taskLists.length === 0) {
      return;
    }
    if (!taskLists.some((list) => list.id === activeView.listId)) {
      setActiveView({ kind: "list", listId: taskLists[0].id });
    }
  }, [activeView, taskLists]);

  useEffect(() => {
    if (activeView.kind !== "list" && activeView.kind !== "favorites") {
      setSelectedTaskId(null);
      return;
    }

    setSelectedTaskId((currentTaskId) => {
      if (visibleTaskRows.some((task) => task.id === currentTaskId)) {
        return currentTaskId;
      }
      return null;
    });
  }, [activeView.kind, visibleTaskRows]);

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

  const handleUpdateTask = useCallback(
    (taskId: string, input: WorkItemUpdateDraft) =>
      runMutation(async () => {
        const task = await tauriTaskTimerGateway.updateTask({
          ...input,
          taskId,
        });
        return task.id;
      }),
    [runMutation],
  );

  const handleUpdateSubtask = useCallback(
    (subtaskId: string, input: WorkItemUpdateDraft) =>
      runMutation(async () => {
        const subtask = await tauriTaskTimerGateway.updateSubtask({
          ...input,
          subtaskId,
        });
        return subtask.taskId;
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

  const handleToggleTaskFavorite = useCallback(
    (taskId: string, isFavorite: boolean) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.toggleTaskFavorite(taskId, isFavorite);
        return taskId;
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

  const handleSelectView = useCallback((view: AppView) => {
    setActiveView(view);
    if (window.matchMedia("(max-width: 767px)").matches) {
      setIsNavigationOpen(false);
    }
  }, []);

  return (
    <main
      className={`app-shell ${
        isNavigationOpen ? "is-nav-open" : "is-nav-collapsed"
      }`}
    >
      <header className="top-bar">
        <div className="top-bar-title">
          <button
            className="top-nav-toggle"
            type="button"
            aria-label={isNavigationOpen ? "左ペインを閉じる" : "左ペインを開く"}
            title="左ペインを開閉"
            onClick={() => setIsNavigationOpen((current) => !current)}
          >
            ☰
          </button>
          <div>
            <p className="eyebrow">オフライン対応デスクトップタスクタイマー</p>
            <h1>TaskTimer</h1>
          </div>
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

      <div className="app-layout" aria-label="TaskTimer作業画面">
        {isNavigationOpen ? (
          <button
            className="nav-backdrop"
            type="button"
            aria-label="左ペインを閉じる"
            onClick={() => setIsNavigationOpen(false)}
          />
        ) : null}
        <LeftNavigation
          activeView={activeView}
          favoriteCount={favoriteCount}
          isOpen={isNavigationOpen}
          taskLists={taskLists}
          onSelectView={handleSelectView}
          onToggle={() => setIsNavigationOpen((current) => !current)}
        />

        <section className="workspace-main" aria-label="現在のビュー">
          {(activeView.kind === "list" || activeView.kind === "favorites") ? (
            <div
              className={`task-workspace ${
                selectedTask ? "is-detail-open" : ""
              }`}
            >
              <TaskPanel
                tasks={visibleTasks}
                taskRows={visibleTaskRows}
                selectedTaskId={selectedTaskId}
                eyebrow={activeView.kind === "favorites" ? "お気に入り" : "リスト"}
                title={
                  activeView.kind === "favorites"
                    ? "お気に入り"
                    : activeTaskList?.name ?? "タスク"
                }
                emptyMessage={
                  activeView.kind === "favorites"
                    ? "お気に入りにしたタスクはまだありません。"
                    : "まだタスクはありません。"
                }
                showTaskForm={activeView.kind === "list"}
                isLoading={isLoading}
                isMutating={isMutating}
                onSelectTask={setSelectedTaskId}
                onCreateTask={handleCreateTask}
                onCompleteTask={handleCompleteTask}
                onToggleTaskFavorite={handleToggleTaskFavorite}
              />
              {selectedTask ? (
                <TaskDetailPane
                  task={selectedTask}
                  activeTimer={activeTimer}
                  displayMode={displayMode}
                  isMutating={isMutating}
                  onClose={() => setSelectedTaskId(null)}
                  onUpdateTask={handleUpdateTask}
                  onUpdateSubtask={handleUpdateSubtask}
                  onCreateSubtask={handleCreateSubtask}
                  onStartTimer={handleStartTimer}
                  onStopTimer={handleStopTimer}
                  onCompleteTask={handleCompleteTask}
                  onCompleteSubtask={handleCompleteSubtask}
                  onDeleteTask={handleDeleteTask}
                  onDeleteSubtask={handleDeleteSubtask}
                />
              ) : null}
            </div>
          ) : null}

          {activeView.kind === "calendar" ? (
            <WeekCalendar
              weekStartDate={weekStartDate}
              items={items}
              isLoading={isLoading}
              onPreviousWeek={() =>
                setWeekStartDate((current) => shiftDate(current, -7))
              }
              onNextWeek={() =>
                setWeekStartDate((current) => shiftDate(current, 7))
              }
            />
          ) : null}

          {activeView.kind === "settings" ? (
            <SettingsPanel
              displayMode={displayMode}
              isMutating={isMutating}
              notificationSummary={notificationSummary}
              onUpdateDisplayMode={handleUpdateNotificationDisplayMode}
              onRetryNotifications={handleRetryNotifications}
            />
          ) : null}
        </section>
      </div>
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
