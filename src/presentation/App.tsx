import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  NotificationDeliveryAttempt,
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
import { WeekCalendar, type CalendarViewMode } from "./components/WeekCalendar";
import { TaskPanel } from "./components/TaskPanel";
import { TaskDetailPane } from "./components/TaskDetailPane";
import { SettingsPanel } from "./components/SettingsPanel";
import { LeftNavigation, type AppView } from "./components/LeftNavigation";

type LoadSnapshotOptions = {
  showLoading?: boolean;
};

export function App() {
  const [tasks, setTasks] = useState<TaskWithSubtasks[]>([]);
  const [taskRows, setTaskRows] = useState<TaskRow[]>([]);
  const [taskLists, setTaskLists] = useState<TaskListItem[]>([]);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [selectedSubtaskId, setSelectedSubtaskId] = useState<string | null>(null);
  const [items, setItems] = useState<WeekCalendarItem[]>([]);
  const [activeTimer, setActiveTimer] = useState<ActiveTimer | null>(null);
  const [activeView, setActiveView] = useState<AppView>({
    kind: "list",
    listId: "default",
  });
  const [selectedCalendarTarget, setSelectedCalendarTarget] =
    useState<WorkTargetRef | null>(null);
  const [isNavigationOpen, setIsNavigationOpen] = useState(true);
  const [displayMode, setDisplayMode] =
    useState<NotificationDisplayMode>("title_only");
  const [notificationsEnabled, setNotificationsEnabled] = useState(true);
  const [notificationSummary, setNotificationSummary] =
    useState<NotificationDispatchSummary | null>(null);
  const [notificationFailureHistory, setNotificationFailureHistory] = useState<
    NotificationDeliveryAttempt[]
  >([]);
  const [calendarViewMode, setCalendarViewMode] =
    useState<CalendarViewMode>("week");
  const [calendarAnchorDate, setCalendarAnchorDate] = useState(
    getTodayDateInputValue,
  );
  const [isLoading, setIsLoading] = useState(true);
  const [isMutating, setIsMutating] = useState(false);
  const [isCreatingTaskPending, setIsCreatingTaskPending] = useState(false);
  const [pendingTaskActionIds, setPendingTaskActionIds] = useState<
    ReadonlySet<string>
  >(new Set());
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const resumeSyncRef = useRef({
    isSyncing: false,
    lastSyncedAt: Date.now(),
  });

  const favoriteCount = useMemo(
    () => tasks.filter((task) => task.isFavorite).length,
    [tasks],
  );
  const todayDate = getTodayDateInputValue();
  const todayTaskIds = useMemo(
    () =>
      new Set(
        tasks
          .filter((task) => isTaskDueOnDate(task, todayDate))
          .map((task) => task.id),
      ),
    [tasks, todayDate],
  );
  const todayCount = useMemo(
    () =>
      tasks.filter(
        (task) => todayTaskIds.has(task.id) && task.status !== "done",
      ).length,
    [tasks, todayTaskIds],
  );

  const activeTaskList = useMemo(() => {
    if (activeView.kind !== "list") {
      return null;
    }
    return taskLists.find((list) => list.id === activeView.listId) ?? null;
  }, [activeView, taskLists]);

  const calendarRange = useMemo(
    () => getCalendarQueryRange(calendarViewMode, calendarAnchorDate),
    [calendarAnchorDate, calendarViewMode],
  );

  const visibleTasks = useMemo(() => {
    if (activeView.kind === "today") {
      return tasks.filter((task) => todayTaskIds.has(task.id));
    }
    if (activeView.kind === "favorites") {
      return tasks.filter((task) => task.isFavorite);
    }
    if (activeView.kind === "list") {
      return tasks.filter((task) => task.listId === activeView.listId);
    }
    return tasks;
  }, [activeView, tasks, todayTaskIds]);

  const visibleTaskRows = useMemo(() => {
    if (activeView.kind === "today") {
      return taskRows.filter((task) => todayTaskIds.has(task.id));
    }
    if (activeView.kind === "favorites") {
      return taskRows.filter((task) => task.isFavorite);
    }
    if (activeView.kind === "list") {
      return taskRows.filter((task) => task.listId === activeView.listId);
    }
    return taskRows;
  }, [activeView, taskRows, todayTaskIds]);

  const selectedTask = useMemo(() => {
    if (!selectedTaskId) {
      return null;
    }
    return visibleTasks.find((task) => task.id === selectedTaskId) ?? null;
  }, [selectedTaskId, visibleTasks]);

  const selectedSubtask = useMemo(() => {
    if (!selectedSubtaskId) {
      return null;
    }
    return (
      selectedTask?.subtasks.find((subtask) => subtask.id === selectedSubtaskId) ??
      null
    );
  }, [selectedSubtaskId, selectedTask]);

  const loadSnapshot = useCallback(async (options?: LoadSnapshotOptions) => {
    const showLoading = options?.showLoading ?? true;
    if (showLoading) {
      setIsLoading(true);
    }
    setErrorMessage(null);

    try {
      await tauriTaskTimerGateway.healthCheck();
      const listId =
        activeView.kind === "list" ? activeView.listId : undefined;
      const [
        nextTasks,
        nextTaskRows,
        nextTaskLists,
        nextItems,
        nextActiveTimer,
        nextDisplayMode,
        nextNotificationsEnabled,
      ] =
        await Promise.all([
          tauriTaskTimerGateway.listTasks(),
          tauriTaskTimerGateway.listTaskRows(listId),
          tauriTaskTimerGateway.listTaskLists(),
          tauriTaskTimerGateway.listCalendarItems(
            calendarRange.startDate,
            calendarRange.endDate,
          ),
          tauriTaskTimerGateway.getActiveTimer(),
          tauriTaskTimerGateway.getNotificationDisplayMode(),
          tauriTaskTimerGateway.getNotificationsEnabled(),
        ]);

      setTasks(nextTasks);
      setTaskRows(nextTaskRows);
      setTaskLists(nextTaskLists);
      setItems(nextItems);
      setActiveTimer(nextActiveTimer);
      setDisplayMode(nextDisplayMode);
      setNotificationsEnabled(nextNotificationsEnabled);
      setNotificationSummary(
        await tauriTaskTimerGateway.dispatchDueNotifications(),
      );
      setNotificationFailureHistory(
        await tauriTaskTimerGateway.listNotificationFailureHistory(),
      );
    } catch (error) {
      setErrorMessage(toErrorMessage(error));
    } finally {
      if (showLoading) {
        setIsLoading(false);
      }
    }
  }, [activeView, calendarRange.endDate, calendarRange.startDate]);

  useEffect(() => {
    void loadSnapshot({ showLoading: true });
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
    const syncAfterResume = () => {
      const now = Date.now();
      if (
        resumeSyncRef.current.isSyncing ||
        now - resumeSyncRef.current.lastSyncedAt < 2_000
      ) {
        return;
      }

      resumeSyncRef.current.isSyncing = true;
      resumeSyncRef.current.lastSyncedAt = now;
      void loadSnapshot({ showLoading: false }).finally(() => {
        resumeSyncRef.current.isSyncing = false;
      });
    };

    const handleVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        syncAfterResume();
      }
    };

    window.addEventListener("focus", syncAfterResume);
    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () => {
      window.removeEventListener("focus", syncAfterResume);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [loadSnapshot]);

  useEffect(() => {
    if (activeView.kind !== "list" || taskLists.length === 0) {
      return;
    }
    if (!taskLists.some((list) => list.id === activeView.listId)) {
      setActiveView({ kind: "list", listId: taskLists[0].id });
    }
  }, [activeView, taskLists]);

  useEffect(() => {
    if (
      activeView.kind !== "calendar" ||
      !selectedCalendarTarget ||
      isLoading
    ) {
      return;
    }

    if (!items.some((item) => isSameTarget(item.target, selectedCalendarTarget))) {
      setSelectedTaskId(null);
      setSelectedCalendarTarget(null);
    }
  }, [activeView.kind, isLoading, items, selectedCalendarTarget]);

  useEffect(() => {
    if (activeView.kind === "settings") {
      setSelectedTaskId(null);
      setSelectedSubtaskId(null);
      setSelectedCalendarTarget(null);
      return;
    }

    if (activeView.kind === "calendar") {
      if (selectedTaskId && !tasks.some((task) => task.id === selectedTaskId)) {
        setSelectedTaskId(null);
        setSelectedSubtaskId(null);
        setSelectedCalendarTarget(null);
      }
      return;
    }

    if (
      selectedTaskId &&
      !visibleTaskRows.some((task) => task.id === selectedTaskId)
    ) {
      setSelectedTaskId(null);
      setSelectedSubtaskId(null);
      setSelectedCalendarTarget(null);
    }
  }, [activeView.kind, selectedTaskId, tasks, visibleTaskRows]);

  useEffect(() => {
    if (selectedSubtaskId && !selectedSubtask) {
      setSelectedSubtaskId(null);
    }
  }, [selectedSubtask, selectedSubtaskId]);

  const runMutation = useCallback(
    async (operation: () => Promise<string | void>) => {
      setIsMutating(true);
      setErrorMessage(null);

      try {
        const nextSelectedTaskId = await operation();
        await loadSnapshot({ showLoading: false });
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

  const runTaskActionMutation = useCallback(
    async (taskId: string, operation: () => Promise<string | void>) => {
      setPendingTaskActionIds((current) => new Set(current).add(taskId));
      setErrorMessage(null);

      try {
        const nextSelectedTaskId = await operation();
        await loadSnapshot({ showLoading: false });
        if (nextSelectedTaskId) {
          setSelectedTaskId(nextSelectedTaskId);
        }
        return true;
      } catch (error) {
        setErrorMessage(toErrorMessage(error));
        return false;
      } finally {
        setPendingTaskActionIds((current) => {
          const next = new Set(current);
          next.delete(taskId);
          return next;
        });
      }
    },
    [loadSnapshot],
  );

  const runCreateTaskMutation = useCallback(
    async (operation: () => Promise<void>) => {
      setIsCreatingTaskPending(true);
      setErrorMessage(null);

      try {
        await operation();
        await loadSnapshot({ showLoading: false });
        return true;
      } catch (error) {
        setErrorMessage(toErrorMessage(error));
        return false;
      } finally {
        setIsCreatingTaskPending(false);
      }
    },
    [loadSnapshot],
  );

  const handleCreateTask = useCallback(
    (input: WorkItemDraft) =>
      runCreateTaskMutation(async () => {
        await tauriTaskTimerGateway.createTask(input);
      }),
    [runCreateTaskMutation],
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

  const handlePauseTimer = useCallback(
    () =>
      runMutation(async () => {
        const pausedTimer = await tauriTaskTimerGateway.pauseActiveTimer();
        return pausedTimer.target.type === "task"
          ? pausedTimer.target.id
          : undefined;
      }),
    [runMutation],
  );

  const handleResumeTimer = useCallback(
    () =>
      runMutation(async () => {
        const resumedTimer = await tauriTaskTimerGateway.resumeActiveTimer();
        return resumedTimer.target.type === "task"
          ? resumedTimer.target.id
          : undefined;
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

  const handleToggleTaskCompletion = useCallback(
    (task: TaskWithSubtasks) => {
      if (task.status === "done") {
        return runTaskActionMutation(task.id, async () => {
          await tauriTaskTimerGateway.reopenTask(task.id);
        });
      }

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

      return runTaskActionMutation(task.id, async () => {
        await tauriTaskTimerGateway.completeTask(task.id, hasIncompleteSubtasks);
      });
    },
    [runTaskActionMutation],
  );

  const handleToggleSubtaskCompletion = useCallback(
    (subtask: Subtask) =>
      runMutation(async () => {
        if (subtask.status === "done") {
          await tauriTaskTimerGateway.reopenSubtask(subtask.id);
        } else {
          await tauriTaskTimerGateway.completeSubtask(subtask.id);
        }
        return subtask.taskId;
      }),
    [runMutation],
  );

  const handleToggleTaskFavorite = useCallback(
    (taskId: string, isFavorite: boolean) =>
      runTaskActionMutation(taskId, async () => {
        await tauriTaskTimerGateway.toggleTaskFavorite(taskId, isFavorite);
      }),
    [runTaskActionMutation],
  );

  const handleDeleteTask = useCallback(
    (task: TaskWithSubtasks) => {
      return runMutation(async () => {
        await tauriTaskTimerGateway.deleteTask(task.id);
      }).then((deleted) => {
        if (deleted) {
          setSelectedTaskId(null);
          setSelectedSubtaskId(null);
          setSelectedCalendarTarget(null);
        }
        return deleted;
      });
    },
    [runMutation],
  );

  const handleDeleteSubtask = useCallback(
    (subtask: Subtask) => {
      return runMutation(async () => {
        await tauriTaskTimerGateway.deleteSubtask(subtask.id);
        return subtask.taskId;
      }).then((deleted) => {
        if (deleted) {
          setSelectedSubtaskId(null);
          setSelectedCalendarTarget({ type: "task", id: subtask.taskId });
        }
        return deleted;
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

  const handleUpdateNotificationsEnabled = useCallback(
    (enabled: boolean) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.updateNotificationsEnabled(enabled);
      }),
    [runMutation],
  );

  const handleRetryNotifications = useCallback(
    () =>
      runMutation(async () => {
        return;
      }),
    [runMutation],
  );

  const clearDetailSelection = useCallback(() => {
    setSelectedTaskId(null);
    setSelectedSubtaskId(null);
    setSelectedCalendarTarget(null);
  }, []);

  const handleSelectView = useCallback((view: AppView) => {
    setActiveView(view);
    clearDetailSelection();
    if (window.matchMedia("(max-width: 767px)").matches) {
      setIsNavigationOpen(false);
    }
  }, [clearDetailSelection]);

  const handleSelectTask = useCallback((taskId: string) => {
    setSelectedTaskId(taskId);
    setSelectedSubtaskId(null);
    setSelectedCalendarTarget(null);
  }, []);

  const handleSelectSubtask = useCallback((taskId: string, subtaskId: string) => {
    setSelectedTaskId(taskId);
    setSelectedSubtaskId(subtaskId);
    setSelectedCalendarTarget({ type: "subtask", id: subtaskId });
  }, []);

  const handleSelectParentTask = useCallback(() => {
    setSelectedSubtaskId(null);
    if (selectedTaskId) {
      setSelectedCalendarTarget({ type: "task", id: selectedTaskId });
    }
  }, [selectedTaskId]);

  const handleSelectCalendarItem = useCallback(
    (item: WeekCalendarItem) => {
      const nextTaskId = resolveTaskIdForTarget(tasks, item.target);
      if (!nextTaskId) {
        setErrorMessage("カレンダー項目の対象タスクが見つかりません。");
        setSelectedTaskId(null);
        setSelectedCalendarTarget(null);
        return;
      }
      setErrorMessage(null);
      setSelectedTaskId(nextTaskId);
      setSelectedSubtaskId(item.target.type === "subtask" ? item.target.id : null);
      setSelectedCalendarTarget(item.target);
    },
    [tasks],
  );

  const handleChangeCalendarViewMode = useCallback(
    (viewMode: CalendarViewMode) => {
      setCalendarViewMode(viewMode);
      clearDetailSelection();
    },
    [clearDetailSelection],
  );

  const handlePreviousCalendarRange = useCallback(() => {
    setCalendarAnchorDate((current) =>
      shiftCalendarAnchorDate(current, calendarViewMode, -1),
    );
    clearDetailSelection();
  }, [calendarViewMode, clearDetailSelection]);

  const handleNextCalendarRange = useCallback(() => {
    setCalendarAnchorDate((current) =>
      shiftCalendarAnchorDate(current, calendarViewMode, 1),
    );
    clearDetailSelection();
  }, [calendarViewMode, clearDetailSelection]);

  const handleTodayCalendarRange = useCallback(() => {
    setCalendarAnchorDate(getTodayDateInputValue());
    clearDetailSelection();
  }, [clearDetailSelection]);

  const closeDetailPane = clearDetailSelection;

  return (
    <main
      className={`app-shell ${
        isNavigationOpen ? "is-nav-open" : "is-nav-collapsed"
      }`}
    >
      <header className="top-bar">
        <div className="top-bar-title">
          <h1>TaskTimer</h1>
        </div>
      </header>

      {errorMessage ? (
        <div className="app-alert" role="alert">
          <strong>処理に失敗しました</strong>
          <span>{errorMessage}</span>
          <button
            type="button"
            onClick={() => void loadSnapshot({ showLoading: true })}
          >
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
          todayCount={todayCount}
          isOpen={isNavigationOpen}
          taskLists={taskLists}
          onSelectView={handleSelectView}
          onToggle={() => setIsNavigationOpen((current) => !current)}
        />

        <section className="workspace-main" aria-label="現在のビュー">
          {activeView.kind === "list" ||
          activeView.kind === "today" ||
          activeView.kind === "favorites" ? (
            <div
              className={`task-workspace ${
                selectedTask ? "is-detail-open" : ""
              }`}
            >
              <TaskPanel
                tasks={visibleTasks}
                taskRows={visibleTaskRows}
                selectedTaskId={selectedTaskId}
                eyebrow={getTaskPanelEyebrow(activeView)}
                title={
                  activeView.kind === "today"
                    ? "今日"
                    : activeView.kind === "favorites"
                      ? "お気に入り"
                      : activeTaskList?.name ?? "タスク"
                }
                emptyMessage={
                  activeView.kind === "today"
                    ? "今日が期限のタスクはありません。"
                    : activeView.kind === "favorites"
                      ? "お気に入りにしたタスクはまだありません。"
                      : "まだタスクはありません。"
                }
                showTaskForm={activeView.kind === "list"}
                isLoading={isLoading}
                isMutating={isMutating}
                isCreatingTaskPending={isCreatingTaskPending}
                pendingTaskActionIds={pendingTaskActionIds}
                selectedSubtaskId={selectedSubtaskId}
                onSelectTask={handleSelectTask}
                onSelectSubtask={handleSelectSubtask}
                onCreateTask={handleCreateTask}
                onToggleTaskCompletion={handleToggleTaskCompletion}
                onToggleTaskFavorite={handleToggleTaskFavorite}
              />
              {selectedTask ? (
                <TaskDetailPane
                  task={selectedTask}
                  selectedSubtaskId={selectedSubtaskId}
                  activeTimer={activeTimer}
                  displayMode={displayMode}
                  isMutating={isMutating}
                  onClose={closeDetailPane}
                  onUpdateTask={handleUpdateTask}
                  onUpdateSubtask={handleUpdateSubtask}
                  onCreateSubtask={handleCreateSubtask}
                  onSelectSubtask={(subtaskId) =>
                    handleSelectSubtask(selectedTask.id, subtaskId)
                  }
                  onSelectParentTask={handleSelectParentTask}
                  onStartTimer={handleStartTimer}
                  onPauseTimer={handlePauseTimer}
                  onResumeTimer={handleResumeTimer}
                  onStopTimer={handleStopTimer}
                  onToggleTaskCompletion={handleToggleTaskCompletion}
                  onToggleSubtaskCompletion={handleToggleSubtaskCompletion}
                  onDeleteTask={handleDeleteTask}
                  onDeleteSubtask={handleDeleteSubtask}
                />
              ) : null}
            </div>
          ) : null}

          {activeView.kind === "calendar" ? (
            <div
              className={`task-workspace calendar-workspace ${
                selectedTask ? "is-detail-open" : ""
              }`}
            >
              <WeekCalendar
                viewMode={calendarViewMode}
                anchorDate={calendarAnchorDate}
                items={items}
                isLoading={isLoading}
                selectedTarget={selectedCalendarTarget}
                onChangeViewMode={handleChangeCalendarViewMode}
                onPreviousRange={handlePreviousCalendarRange}
                onNextRange={handleNextCalendarRange}
                onToday={handleTodayCalendarRange}
                onSelectItem={handleSelectCalendarItem}
              />
              {selectedTask ? (
                <TaskDetailPane
                  task={selectedTask}
                  selectedSubtaskId={selectedSubtaskId}
                  activeTimer={activeTimer}
                  displayMode={displayMode}
                  isMutating={isMutating}
                  onClose={closeDetailPane}
                  onUpdateTask={handleUpdateTask}
                  onUpdateSubtask={handleUpdateSubtask}
                  onCreateSubtask={handleCreateSubtask}
                  onSelectSubtask={(subtaskId) =>
                    handleSelectSubtask(selectedTask.id, subtaskId)
                  }
                  onSelectParentTask={handleSelectParentTask}
                  onStartTimer={handleStartTimer}
                  onPauseTimer={handlePauseTimer}
                  onResumeTimer={handleResumeTimer}
                  onStopTimer={handleStopTimer}
                  onToggleTaskCompletion={handleToggleTaskCompletion}
                  onToggleSubtaskCompletion={handleToggleSubtaskCompletion}
                  onDeleteTask={handleDeleteTask}
                  onDeleteSubtask={handleDeleteSubtask}
                />
              ) : null}
            </div>
          ) : null}

          {activeView.kind === "settings" ? (
            <SettingsPanel
              displayMode={displayMode}
              notificationsEnabled={notificationsEnabled}
              isMutating={isMutating}
              notificationSummary={notificationSummary}
              notificationFailureHistory={notificationFailureHistory}
              onUpdateDisplayMode={handleUpdateNotificationDisplayMode}
              onUpdateNotificationsEnabled={handleUpdateNotificationsEnabled}
              onRetryNotifications={handleRetryNotifications}
            />
          ) : null}
        </section>
      </div>
    </main>
  );
}

function getTaskPanelEyebrow(activeView: AppView) {
  if (activeView.kind === "today") {
    return "今日のタスク";
  }
  if (activeView.kind === "favorites") {
    return "お気に入り";
  }
  return "リスト";
}

function isTaskDueOnDate(task: TaskWithSubtasks, date: string) {
  return (
    task.dueDate === date ||
    task.subtasks.some((subtask) => subtask.dueDate === date)
  );
}

function getTodayDateInputValue() {
  return toDateInputValue(new Date());
}

function getCalendarQueryRange(
  viewMode: CalendarViewMode,
  anchorDate: string,
) {
  const anchor = parseDateInputValue(anchorDate);
  if (viewMode === "day") {
    return {
      startDate: toDateInputValue(anchor),
      endDate: toDateInputValue(anchor),
    };
  }

  if (viewMode === "month") {
    const firstDay = new Date(anchor.getFullYear(), anchor.getMonth(), 1);
    const lastDay = new Date(anchor.getFullYear(), anchor.getMonth() + 1, 0);
    return {
      startDate: toDateInputValue(getMondayOfWeek(firstDay)),
      endDate: toDateInputValue(getSundayOfWeek(lastDay)),
    };
  }

  const weekStart = getMondayOfWeek(anchor);
  const weekEnd = new Date(weekStart);
  weekEnd.setDate(weekStart.getDate() + 6);
  return {
    startDate: toDateInputValue(weekStart),
    endDate: toDateInputValue(weekEnd),
  };
}

function shiftCalendarAnchorDate(
  value: string,
  viewMode: CalendarViewMode,
  direction: -1 | 1,
) {
  if (viewMode === "day") {
    return shiftDate(value, direction);
  }

  if (viewMode === "month") {
    const date = parseDateInputValue(value);
    date.setDate(1);
    date.setMonth(date.getMonth() + direction);
    return toDateInputValue(date);
  }

  return shiftDate(value, 7 * direction);
}

function getMondayOfWeek(value: Date) {
  const date = new Date(value);
  const mondayBasedDay = (date.getDay() + 6) % 7;
  date.setDate(date.getDate() - mondayBasedDay);
  return date;
}

function getSundayOfWeek(value: Date) {
  const date = new Date(value);
  const mondayBasedDay = (date.getDay() + 6) % 7;
  date.setDate(date.getDate() + (6 - mondayBasedDay));
  return date;
}

function isSameTarget(
  target: WorkTargetRef,
  selectedTarget: WorkTargetRef | null,
) {
  return (
    selectedTarget?.type === target.type && selectedTarget.id === target.id
  );
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

function resolveTaskIdForTarget(
  tasks: TaskWithSubtasks[],
  target: WorkTargetRef,
) {
  if (target.type === "task") {
    return tasks.some((task) => task.id === target.id) ? target.id : null;
  }

  return (
    tasks.find((task) =>
      task.subtasks.some((subtask) => subtask.id === target.id),
    )?.id ?? null
  );
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
