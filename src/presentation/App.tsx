import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  ActivePomodoro,
  NotificationDeliveryAttempt,
  NotificationDispatchSummary,
  PomodoroSettings,
  PomodoroSettingsDraft,
  RecurrenceRuleDraft,
  TagItem,
  TaskListColorToken,
  TaskListItem,
  TaskRow,
  TaskWithSubtasks,
  UiPreferences,
  WeekCalendarItem,
  WorkItemDraft,
  WorkItemUpdateDraft,
} from "../application/usecases/contracts";
import type { NotificationDisplayMode } from "../domain/notification/types";
import type { ActiveTimer, TimerSession } from "../domain/timer/types";
import {
  DEFAULT_TASK_LIST_ID,
  type Task,
  type Subtask,
  type WorkTargetRef,
} from "../domain/task/types";
import { tauriTaskTimerGateway } from "../infrastructure/tauri/gateway";
import { WeekCalendar, type CalendarViewMode } from "./components/WeekCalendar";
import { TaskPanel } from "./components/TaskPanel";
import { KanbanBoard } from "./components/KanbanBoard";
import { TaskDetailPane } from "./components/TaskDetailPane";
import {
  SettingsPanel,
  type DataManagementActionResult,
} from "./components/SettingsPanel";
import { LeftNavigation, type AppView } from "./components/LeftNavigation";

type LoadSnapshotOptions = {
  showLoading?: boolean;
};

export function App() {
  const [tasks, setTasks] = useState<TaskWithSubtasks[]>([]);
  const [taskRows, setTaskRows] = useState<TaskRow[]>([]);
  const [taskLists, setTaskLists] = useState<TaskListItem[]>([]);
  const [tags, setTags] = useState<TagItem[]>([]);
  const [selectedTaskId, setSelectedTaskId] = useState<string | null>(null);
  const [selectedSubtaskId, setSelectedSubtaskId] = useState<string | null>(null);
  const [items, setItems] = useState<WeekCalendarItem[]>([]);
  const [activeTimer, setActiveTimer] = useState<ActiveTimer | null>(null);
  const [activePomodoro, setActivePomodoro] = useState<ActivePomodoro | null>(
    null,
  );
  const [activeView, setActiveView] = useState<AppView>({
    kind: "list",
    listId: DEFAULT_TASK_LIST_ID,
  });
  const [selectedCalendarTarget, setSelectedCalendarTarget] =
    useState<WorkTargetRef | null>(null);
  const [isNavigationOpen, setIsNavigationOpen] = useState(true);
  const [lastTaskListId, setLastTaskListId] = useState(DEFAULT_TASK_LIST_ID);
  const [displayMode, setDisplayMode] =
    useState<NotificationDisplayMode>("title_only");
  const [notificationsEnabled, setNotificationsEnabled] = useState(true);
  const [pomodoroSettings, setPomodoroSettings] =
    useState<PomodoroSettings | null>(null);
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
  const [hasHydratedUiPreferences, setHasHydratedUiPreferences] =
    useState(false);
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
  const lastPersistedUiPreferencesRef = useRef<string | null>(null);

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

  const activeTag = useMemo(() => {
    if (activeView.kind !== "tag") {
      return null;
    }
    return tags.find((tag) => tag.id === activeView.tagId) ?? null;
  }, [activeView, tags]);

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
    if (activeView.kind === "tag") {
      return tasks.filter((task) =>
        task.tags.some((tag) => tag.id === activeView.tagId),
      );
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
    if (activeView.kind === "tag") {
      return taskRows.filter((task) =>
        task.tags.some((tag) => tag.id === activeView.tagId),
      );
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
        nextTags,
        nextItems,
        nextActiveTimer,
        nextActivePomodoro,
        nextPomodoroSettings,
        nextDisplayMode,
        nextNotificationsEnabled,
      ] =
        await Promise.all([
          tauriTaskTimerGateway.listTasks(),
          tauriTaskTimerGateway.listTaskRows(listId),
          tauriTaskTimerGateway.listTaskLists(),
          tauriTaskTimerGateway.listTags(),
          tauriTaskTimerGateway.listCalendarItems(
            calendarRange.startDate,
            calendarRange.endDate,
          ),
          tauriTaskTimerGateway.getActiveTimer(),
          tauriTaskTimerGateway.getActivePomodoro(),
          tauriTaskTimerGateway.getPomodoroSettings(),
          tauriTaskTimerGateway.getNotificationDisplayMode(),
          tauriTaskTimerGateway.getNotificationsEnabled(),
        ]);

      setTasks(nextTasks);
      setTaskRows(nextTaskRows);
      setTaskLists(nextTaskLists);
      setTags(nextTags);
      setItems(nextItems);
      setActiveTimer(nextActiveTimer);
      setActivePomodoro(nextActivePomodoro);
      setPomodoroSettings(nextPomodoroSettings);
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
    let isCancelled = false;

    async function hydrateUiPreferences() {
      try {
        const preferences = await tauriTaskTimerGateway.getUiPreferences();
        if (isCancelled) {
          return;
        }
        setIsNavigationOpen(preferences.leftPaneOpen);
        setLastTaskListId(normalizeTaskListId(preferences.lastTaskListId));
        setCalendarViewMode(preferences.calendarViewMode);
        setActiveView(appViewFromPreferences(preferences));
        lastPersistedUiPreferencesRef.current =
          serializeUiPreferences(preferences);
      } catch {
        if (!isCancelled) {
          lastPersistedUiPreferencesRef.current = serializeUiPreferences(
            uiPreferencesFromState({
              activeView: { kind: "list", listId: DEFAULT_TASK_LIST_ID },
              isNavigationOpen: true,
              lastTaskListId: DEFAULT_TASK_LIST_ID,
              calendarViewMode: "week",
            }),
          );
        }
      } finally {
        if (!isCancelled) {
          setHasHydratedUiPreferences(true);
        }
      }
    }

    void hydrateUiPreferences();
    return () => {
      isCancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!hasHydratedUiPreferences) {
      return;
    }
    void loadSnapshot({ showLoading: true });
  }, [hasHydratedUiPreferences, loadSnapshot]);

  useEffect(() => {
    if (activeView.kind === "list") {
      setLastTaskListId(activeView.listId);
    }
  }, [activeView]);

  useEffect(() => {
    if (!hasHydratedUiPreferences) {
      return;
    }

    const preferences = uiPreferencesFromState({
      activeView,
      isNavigationOpen,
      lastTaskListId,
      calendarViewMode,
    });
    const serialized = serializeUiPreferences(preferences);
    if (lastPersistedUiPreferencesRef.current === serialized) {
      return;
    }

    lastPersistedUiPreferencesRef.current = serialized;
    void tauriTaskTimerGateway.updateUiPreferences(preferences).catch((error) => {
      lastPersistedUiPreferencesRef.current = null;
      setErrorMessage(toErrorMessage(error));
    });
  }, [
    activeView,
    calendarViewMode,
    hasHydratedUiPreferences,
    isNavigationOpen,
    lastTaskListId,
  ]);

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

  const clearDetailSelection = useCallback(() => {
    setSelectedTaskId(null);
    setSelectedSubtaskId(null);
    setSelectedCalendarTarget(null);
  }, []);

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
        await tauriTaskTimerGateway.createTask({
          ...input,
          listId:
            input.listId ??
            (activeView.kind === "list" ? activeView.listId : DEFAULT_TASK_LIST_ID),
        });
      }),
    [activeView, runCreateTaskMutation],
  );

  const handleCreateTaskList = useCallback(
    (name: string) =>
      runMutation(async () => {
        const list = await tauriTaskTimerGateway.createTaskList({ name });
        setActiveView({ kind: "list", listId: list.id });
        clearDetailSelection();
      }),
    [clearDetailSelection, runMutation],
  );

  const handleRenameTaskList = useCallback(
    (listId: string, name: string) =>
      runMutation(async () => {
        const currentList = taskLists.find((list) => list.id === listId);
        await tauriTaskTimerGateway.updateTaskList(listId, {
          name,
          colorToken: currentList?.colorToken,
        });
      }),
    [runMutation, taskLists],
  );

  const handleUpdateTaskListColor = useCallback(
    (listId: string, colorToken: TaskListColorToken) =>
      runMutation(async () => {
        const currentList = taskLists.find((list) => list.id === listId);
        if (!currentList) {
          throw new Error("色を変更するリストが見つかりません。");
        }
        await tauriTaskTimerGateway.updateTaskList(listId, {
          name: currentList.name,
          colorToken,
        });
      }),
    [runMutation, taskLists],
  );

  const handleDeleteTaskList = useCallback(
    (listId: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.deleteTaskList(listId);
        if (activeView.kind === "list" && activeView.listId === listId) {
          setActiveView({ kind: "list", listId: DEFAULT_TASK_LIST_ID });
          clearDetailSelection();
        }
      }),
    [activeView, clearDetailSelection, runMutation],
  );

  const handleCreateTag = useCallback(
    (name: string) =>
      runMutation(async () => {
        const tag = await tauriTaskTimerGateway.createTag({ name });
        setActiveView({ kind: "tag", tagId: tag.id });
        clearDetailSelection();
      }),
    [clearDetailSelection, runMutation],
  );

  const handleRenameTag = useCallback(
    (tagId: string, name: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.updateTag(tagId, { name });
      }),
    [runMutation],
  );

  const handleDeleteTag = useCallback(
    (tagId: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.deleteTag(tagId);
        if (activeView.kind === "tag" && activeView.tagId === tagId) {
          setActiveView({ kind: "list", listId: DEFAULT_TASK_LIST_ID });
          clearDetailSelection();
        }
      }),
    [activeView, clearDetailSelection, runMutation],
  );

  const handleAttachTagToTask = useCallback(
    (taskId: string, tagId: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.attachTagToTask(taskId, tagId);
        return taskId;
      }),
    [runMutation],
  );

  const handleDetachTagFromTask = useCallback(
    (taskId: string, tagId: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.detachTagFromTask(taskId, tagId);
        return taskId;
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

  const handleStartPomodoro = useCallback(
    (target: WorkTargetRef) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.startPomodoro(target);
        return target.type === "task" ? target.id : undefined;
      }),
    [runMutation],
  );

  const handlePausePomodoro = useCallback(
    () =>
      runMutation(async () => {
        await tauriTaskTimerGateway.pausePomodoro();
      }),
    [runMutation],
  );

  const handleResumePomodoro = useCallback(
    () =>
      runMutation(async () => {
        await tauriTaskTimerGateway.resumePomodoro();
      }),
    [runMutation],
  );

  const handleCompletePomodoroWork = useCallback(
    () =>
      runMutation(async () => {
        const completed =
          await tauriTaskTimerGateway.completePomodoroWorkPhase();
        return completed.target.type === "task" ? completed.target.id : undefined;
      }),
    [runMutation],
  );

  const handleCompletePomodoroWorkAndStartBreak = useCallback(
    () =>
      runMutation(async () => {
        const completed =
          await tauriTaskTimerGateway.completePomodoroWorkPhase();
        const nextBreak = await tauriTaskTimerGateway.startPomodoroBreak(
          completed.id,
        );
        return nextBreak.target.type === "task" ? nextBreak.target.id : undefined;
      }),
    [runMutation],
  );

  const handleCompletePomodoroWorkAndStartNext = useCallback(
    () =>
      runMutation(async () => {
        const completed =
          await tauriTaskTimerGateway.completePomodoroWorkPhase();
        const nextWork = await tauriTaskTimerGateway.skipPomodoroBreak(
          completed.id,
        );
        return nextWork.target.type === "task" ? nextWork.target.id : undefined;
      }),
    [runMutation],
  );

  const handleSkipPomodoroBreak = useCallback(
    (pomodoroSessionId: string) =>
      runMutation(async () => {
        const nextWork =
          await tauriTaskTimerGateway.skipPomodoroBreak(pomodoroSessionId);
        return nextWork.target.type === "task" ? nextWork.target.id : undefined;
      }),
    [runMutation],
  );

  const handleCompletePomodoroBreak = useCallback(
    () =>
      runMutation(async () => {
        const completed = await tauriTaskTimerGateway.completePomodoroBreak();
        return completed.target.type === "task" ? completed.target.id : undefined;
      }),
    [runMutation],
  );

  const handleCompletePomodoroBreakAndStartNext = useCallback(
    () =>
      runMutation(async () => {
        const completed = await tauriTaskTimerGateway.completePomodoroBreak();
        const nextWork = await tauriTaskTimerGateway.skipPomodoroBreak(
          completed.id,
        );
        return nextWork.target.type === "task" ? nextWork.target.id : undefined;
      }),
    [runMutation],
  );

  const handleCancelPomodoro = useCallback(
    () =>
      runMutation(async () => {
        const cancelled = await tauriTaskTimerGateway.cancelPomodoro();
        return cancelled.target.type === "task" ? cancelled.target.id : undefined;
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

  const handleChangeTaskStatus = useCallback(
    (task: TaskWithSubtasks, status: Exclude<Task["status"], "archived">) => {
      if (task.status === status) {
        return Promise.resolve(true);
      }

      const hasIncompleteSubtasks =
        status === "done" &&
        task.subtasks.some((subtask) => subtask.status !== "done");
      if (
        hasIncompleteSubtasks &&
        !window.confirm(
          "未完了のサブタスクがあります。サブタスクは未完了のまま、親タスクだけ完了しますか？",
        )
      ) {
        return Promise.resolve(false);
      }

      return runTaskActionMutation(task.id, async () => {
        await tauriTaskTimerGateway.updateTaskStatus(
          task.id,
          status,
          hasIncompleteSubtasks,
        );
        return task.id;
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

  const handleUpdatePomodoroSettings = useCallback(
    (input: PomodoroSettingsDraft) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.updatePomodoroSettings(input);
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

  const runDataManagementAction = useCallback(
    async (
      action: () => Promise<DataManagementActionResult>,
    ): Promise<DataManagementActionResult> => {
      setIsMutating(true);
      setErrorMessage(null);
      try {
        return await action();
      } catch (error) {
        return {
          status: "failed",
          message: "データ管理操作に失敗しました。",
          detail: toDataManagementErrorDetail(error),
        };
      } finally {
        setIsMutating(false);
      }
    },
    [],
  );

  const handleCreateSqliteBackup = useCallback(
    () =>
      runDataManagementAction(async (): Promise<DataManagementActionResult> => {
        const destinationDir = await selectDirectory(
          "SQLiteバックアップの保存先を選択",
        );
        if (!destinationDir) {
          return createCancelledResult(
            "SQLiteバックアップの作成をキャンセルしました。",
          );
        }

        const result =
          await tauriTaskTimerGateway.createSqliteBackup(destinationDir);
        return {
          status: "success",
          message: "SQLiteバックアップを作成しました。",
          detail: `保存先: ${getPathBasename(result.backupDir)}`,
        };
      }),
    [runDataManagementAction],
  );

  const handleRestoreSqliteBackup = useCallback(
    () =>
      runDataManagementAction(async (): Promise<DataManagementActionResult> => {
        const backupDir = await selectDirectory(
          "SQLiteバックアップフォルダを選択",
          false,
        );
        if (!backupDir) {
          return createCancelledResult(
            "SQLiteバックアップからの復元をキャンセルしました。",
          );
        }

        const confirmed = window.confirm(
          "選択したSQLiteバックアップで現在のデータを置き換えます。現在のDBを退避してから実行したい場合はキャンセルしてください。復元を続行しますか？",
        );
        if (!confirmed) {
          return createCancelledResult(
            "SQLiteバックアップからの復元をキャンセルしました。",
          );
        }

        const result =
          await tauriTaskTimerGateway.restoreSqliteBackup(backupDir);
        clearDetailSelection();
        await loadSnapshot({ showLoading: false });
        return {
          status: "success",
          message: "SQLiteバックアップから復元しました。",
          detail: `復元元: ${getPathBasename(result.backupDir)}`,
        };
      }),
    [clearDetailSelection, loadSnapshot, runDataManagementAction],
  );

  const handleCreateJsonExport = useCallback(
    () =>
      runDataManagementAction(async (): Promise<DataManagementActionResult> => {
        const destinationDir = await selectDirectory(
          "JSONエクスポートの保存先を選択",
        );
        if (!destinationDir) {
          return createCancelledResult("JSONエクスポートをキャンセルしました。");
        }

        const result =
          await tauriTaskTimerGateway.createJsonExport(destinationDir);
        return {
          status: "success",
          message: "JSONエクスポートを作成しました。",
          detail: `保存先: ${getPathBasename(result.exportPath)}`,
        };
      }),
    [runDataManagementAction],
  );

  const handleCreateCsvExport = useCallback(
    () =>
      runDataManagementAction(async (): Promise<DataManagementActionResult> => {
        const destinationDir = await selectDirectory(
          "CSVエクスポートの保存先を選択",
        );
        if (!destinationDir) {
          return createCancelledResult("CSVエクスポートをキャンセルしました。");
        }

        const result =
          await tauriTaskTimerGateway.createCsvExport(destinationDir);
        return {
          status: "success",
          message: "CSVエクスポートを作成しました。",
          detail: `保存先: ${getPathBasename(result.exportPath)}`,
        };
      }),
    [runDataManagementAction],
  );

  const handleSelectView = useCallback(
    (view: AppView) => {
      if (isSameAppView(activeView, view)) {
        return;
      }
      setActiveView(view);
      clearDetailSelection();
      if (window.matchMedia("(max-width: 767px)").matches) {
        setIsNavigationOpen(false);
      }
    },
    [activeView, clearDetailSelection],
  );

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

  const handleRescheduleCalendarItem = useCallback(
    (item: WeekCalendarItem, dueDate: string, dueTime: string | null) => {
      if (item.marker !== "due") {
        setErrorMessage("移動できるのは期限のカレンダー項目だけです。");
        return Promise.resolve(false);
      }

      if (item.date === dueDate && (item.time ?? null) === (dueTime ?? null)) {
        return Promise.resolve(true);
      }

      if (item.target.type === "task") {
        const task = tasks.find((candidate) => candidate.id === item.target.id);
        if (!task) {
          setErrorMessage("移動対象のタスクが見つかりません。");
          return Promise.resolve(false);
        }

        return handleUpdateTask(task.id, {
          listId: task.listId,
          title: task.title,
          plannedStartDate: task.plannedStartDate,
          dueDate,
          dueTime,
          timerTargetSeconds: task.timerTargetSeconds,
          recurrenceRule: toRecurrenceRuleDraft(task.recurrenceRule),
          memo: task.memo,
        });
      }

      const parentTask = tasks.find((task) =>
        task.subtasks.some((subtask) => subtask.id === item.target.id),
      );
      const subtask =
        parentTask?.subtasks.find((candidate) => candidate.id === item.target.id) ??
        null;
      if (!subtask) {
        setErrorMessage("移動対象のサブタスクが見つかりません。");
        return Promise.resolve(false);
      }

      return handleUpdateSubtask(subtask.id, {
        title: subtask.title,
        plannedStartDate: subtask.plannedStartDate,
        dueDate,
        dueTime,
        timerTargetSeconds: subtask.timerTargetSeconds,
        recurrenceRule: toRecurrenceRuleDraft(subtask.recurrenceRule),
        memo: subtask.memo,
      });
    },
    [handleUpdateSubtask, handleUpdateTask, tasks],
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
          tags={tags}
          isMutating={isMutating}
          onSelectView={handleSelectView}
          onCreateTaskList={handleCreateTaskList}
          onRenameTaskList={handleRenameTaskList}
          onUpdateTaskListColor={handleUpdateTaskListColor}
          onDeleteTaskList={handleDeleteTaskList}
          onCreateTag={handleCreateTag}
          onRenameTag={handleRenameTag}
          onDeleteTag={handleDeleteTag}
          onToggle={() => setIsNavigationOpen((current) => !current)}
        />

        <section className="workspace-main" aria-label="現在のビュー">
          {activeView.kind === "list" ||
          activeView.kind === "today" ||
          activeView.kind === "favorites" ||
          activeView.kind === "tag" ? (
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
                      : activeView.kind === "tag"
                        ? activeTag?.name ?? "タグ"
                        : activeTaskList?.name ?? "タスク"
                }
                emptyMessage={
                  activeView.kind === "today"
                    ? "今日が期限のタスクはありません。"
                    : activeView.kind === "favorites"
                      ? "お気に入りにしたタスクはまだありません。"
                      : activeView.kind === "tag"
                        ? "このタグが付いたタスクはありません。"
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
                  activePomodoro={activePomodoro}
                  taskLists={taskLists}
                  tags={tags}
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
                  onStartPomodoro={handleStartPomodoro}
                  onPausePomodoro={handlePausePomodoro}
                  onResumePomodoro={handleResumePomodoro}
                  onCompletePomodoroWork={handleCompletePomodoroWork}
                  onCompletePomodoroWorkAndStartBreak={
                    handleCompletePomodoroWorkAndStartBreak
                  }
                  onCompletePomodoroWorkAndStartNext={
                    handleCompletePomodoroWorkAndStartNext
                  }
                  onSkipPomodoroBreak={handleSkipPomodoroBreak}
                  onCompletePomodoroBreak={handleCompletePomodoroBreak}
                  onCompletePomodoroBreakAndStartNext={
                    handleCompletePomodoroBreakAndStartNext
                  }
                  onCancelPomodoro={handleCancelPomodoro}
                  onToggleTaskCompletion={handleToggleTaskCompletion}
                  onToggleSubtaskCompletion={handleToggleSubtaskCompletion}
                  onDeleteTask={handleDeleteTask}
                  onDeleteSubtask={handleDeleteSubtask}
                  onAttachTagToTask={handleAttachTagToTask}
                  onDetachTagFromTask={handleDetachTagFromTask}
                />
              ) : null}
            </div>
          ) : null}

          {activeView.kind === "board" ? (
            <div
              className={`task-workspace ${
                selectedTask ? "is-detail-open" : ""
              }`}
            >
              <KanbanBoard
                tasks={visibleTasks}
                taskRows={visibleTaskRows}
                selectedTaskId={selectedTaskId}
                isLoading={isLoading}
                isMutating={isMutating}
                pendingTaskActionIds={pendingTaskActionIds}
                onSelectTask={handleSelectTask}
                onChangeTaskStatus={handleChangeTaskStatus}
              />
              {selectedTask ? (
                <TaskDetailPane
                  task={selectedTask}
                  selectedSubtaskId={selectedSubtaskId}
                  activeTimer={activeTimer}
                  activePomodoro={activePomodoro}
                  taskLists={taskLists}
                  tags={tags}
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
                  onStartPomodoro={handleStartPomodoro}
                  onPausePomodoro={handlePausePomodoro}
                  onResumePomodoro={handleResumePomodoro}
                  onCompletePomodoroWork={handleCompletePomodoroWork}
                  onCompletePomodoroWorkAndStartBreak={
                    handleCompletePomodoroWorkAndStartBreak
                  }
                  onCompletePomodoroWorkAndStartNext={
                    handleCompletePomodoroWorkAndStartNext
                  }
                  onSkipPomodoroBreak={handleSkipPomodoroBreak}
                  onCompletePomodoroBreak={handleCompletePomodoroBreak}
                  onCompletePomodoroBreakAndStartNext={
                    handleCompletePomodoroBreakAndStartNext
                  }
                  onCancelPomodoro={handleCancelPomodoro}
                  onToggleTaskCompletion={handleToggleTaskCompletion}
                  onToggleSubtaskCompletion={handleToggleSubtaskCompletion}
                  onDeleteTask={handleDeleteTask}
                  onDeleteSubtask={handleDeleteSubtask}
                  onAttachTagToTask={handleAttachTagToTask}
                  onDetachTagFromTask={handleDetachTagFromTask}
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
                taskLists={taskLists}
                defaultTaskListId={lastTaskListId}
                isLoading={isLoading}
                isCreatingTaskPending={isCreatingTaskPending}
                isReschedulingItem={isMutating}
                selectedTarget={selectedCalendarTarget}
                onChangeViewMode={handleChangeCalendarViewMode}
                onPreviousRange={handlePreviousCalendarRange}
                onNextRange={handleNextCalendarRange}
                onToday={handleTodayCalendarRange}
                onSelectItem={handleSelectCalendarItem}
                onCreateTask={handleCreateTask}
                onRescheduleItem={handleRescheduleCalendarItem}
              />
              {selectedTask ? (
                <TaskDetailPane
                  task={selectedTask}
                  selectedSubtaskId={selectedSubtaskId}
                  activeTimer={activeTimer}
                  activePomodoro={activePomodoro}
                  taskLists={taskLists}
                  tags={tags}
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
                  onStartPomodoro={handleStartPomodoro}
                  onPausePomodoro={handlePausePomodoro}
                  onResumePomodoro={handleResumePomodoro}
                  onCompletePomodoroWork={handleCompletePomodoroWork}
                  onCompletePomodoroWorkAndStartBreak={
                    handleCompletePomodoroWorkAndStartBreak
                  }
                  onCompletePomodoroWorkAndStartNext={
                    handleCompletePomodoroWorkAndStartNext
                  }
                  onSkipPomodoroBreak={handleSkipPomodoroBreak}
                  onCompletePomodoroBreak={handleCompletePomodoroBreak}
                  onCompletePomodoroBreakAndStartNext={
                    handleCompletePomodoroBreakAndStartNext
                  }
                  onCancelPomodoro={handleCancelPomodoro}
                  onToggleTaskCompletion={handleToggleTaskCompletion}
                  onToggleSubtaskCompletion={handleToggleSubtaskCompletion}
                  onDeleteTask={handleDeleteTask}
                  onDeleteSubtask={handleDeleteSubtask}
                  onAttachTagToTask={handleAttachTagToTask}
                  onDetachTagFromTask={handleDetachTagFromTask}
                />
              ) : null}
            </div>
          ) : null}

          {activeView.kind === "settings" ? (
            <SettingsPanel
              displayMode={displayMode}
              notificationsEnabled={notificationsEnabled}
              pomodoroSettings={pomodoroSettings}
              isMutating={isMutating}
              notificationSummary={notificationSummary}
              notificationFailureHistory={notificationFailureHistory}
              onUpdateDisplayMode={handleUpdateNotificationDisplayMode}
              onUpdateNotificationsEnabled={handleUpdateNotificationsEnabled}
              onUpdatePomodoroSettings={handleUpdatePomodoroSettings}
              onRetryNotifications={handleRetryNotifications}
              onCreateSqliteBackup={handleCreateSqliteBackup}
              onRestoreSqliteBackup={handleRestoreSqliteBackup}
              onCreateJsonExport={handleCreateJsonExport}
              onCreateCsvExport={handleCreateCsvExport}
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
  if (activeView.kind === "tag") {
    return "タグ";
  }
  return "リスト";
}

function appViewFromPreferences(preferences: UiPreferences): AppView {
  if (preferences.lastView === "list") {
    return {
      kind: "list",
      listId: normalizeTaskListId(preferences.lastTaskListId),
    };
  }
  if (preferences.lastView === "tag") {
    return {
      kind: "list",
      listId: normalizeTaskListId(preferences.lastTaskListId),
    };
  }
  return { kind: preferences.lastView };
}

function uiPreferencesFromState({
  activeView,
  isNavigationOpen,
  lastTaskListId,
  calendarViewMode,
}: {
  activeView: AppView;
  isNavigationOpen: boolean;
  lastTaskListId: string;
  calendarViewMode: CalendarViewMode;
}): UiPreferences {
  return {
    leftPaneOpen: isNavigationOpen,
    lastView: activeView.kind === "tag" ? "list" : activeView.kind,
    lastTaskListId:
      activeView.kind === "list"
        ? normalizeTaskListId(activeView.listId)
        : normalizeTaskListId(lastTaskListId),
    calendarViewMode,
  };
}

function serializeUiPreferences(preferences: UiPreferences) {
  return JSON.stringify(preferences);
}

function normalizeTaskListId(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : DEFAULT_TASK_LIST_ID;
}

function isTaskDueOnDate(task: TaskWithSubtasks, date: string) {
  return (
    task.dueDate === date ||
    task.subtasks.some((subtask) => subtask.dueDate === date)
  );
}

function toRecurrenceRuleDraft(
  recurrenceRule: TaskWithSubtasks["recurrenceRule"] | Subtask["recurrenceRule"],
): RecurrenceRuleDraft | null {
  if (!recurrenceRule) {
    return null;
  }

  return {
    frequency: recurrenceRule.frequency,
    interval: recurrenceRule.interval,
  };
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

function isSameAppView(current: AppView, next: AppView) {
  if (current.kind !== next.kind) {
    return false;
  }
  if (current.kind === "list" && next.kind === "list") {
    return current.listId === next.listId;
  }
  return true;
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

async function selectDirectory(title: string, canCreateDirectories = true) {
  const selected = await open({
    title,
    directory: true,
    multiple: false,
    canCreateDirectories,
  });
  if (Array.isArray(selected)) {
    return selected[0] ?? null;
  }
  return selected;
}

function createCancelledResult(message: string): DataManagementActionResult {
  return {
    status: "cancelled",
    message,
  };
}

function getPathBasename(path: string) {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? "選択した場所";
}

function toDataManagementErrorDetail(error: unknown) {
  const message = toErrorMessage(error);
  const withoutUnixPaths = message.replace(/\/[^:\s]+/g, "選択した場所");
  const withoutWindowsPaths = withoutUnixPaths.replace(
    /[A-Za-z]:\\[^:\s]+/g,
    "選択した場所",
  );
  return withoutWindowsPaths.length > 180
    ? `${withoutWindowsPaths.slice(0, 177)}...`
    : withoutWindowsPaths;
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
