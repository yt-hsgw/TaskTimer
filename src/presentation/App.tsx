import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  ActivePomodoro,
  BoardColumn,
  NextNotificationSchedule,
  NotificationDispatchSummary,
  PomodoroSettings,
  PomodoroSettingsDraft,
  RecurrenceRuleDraft,
  ScheduledTaskDraft,
  TagItem,
  TaskListColorToken,
  TaskListItem,
  TaskPage,
  TaskPageCursor,
  TaskPageScope,
  TaskRow,
  TaskTimerSettings,
  TaskTimerSettingsDraft,
  TaskWithSubtasks,
  UiPreferences,
  WeekCalendarItem,
  WorkItemDraft,
  WorkItemSearchResult,
  WorkItemUpdateDraft,
  WorkScheduleDraft,
  WorkScheduleMoveDraft,
} from "../application/usecases/contracts";
import type { NotificationDisplayMode } from "../domain/notification/types";
import type { ActiveTimer } from "../domain/timer/types";
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
import {
  LeftNavigation,
  type AppView,
  type WorkspaceScope,
} from "./components/LeftNavigation";
import { PomodoroPanel } from "./components/PomodoroPanel";
import { GlobalSearch } from "./components/GlobalSearch";
import { TaskCreateDialog } from "./components/TaskCreateDialog";
import { usePresentationRenderProbe } from "./renderProbe";
import type {
  CalendarTaskCreatePreset,
  TaskCreatePreset,
  TaskCreateSubmission,
} from "./taskCreate";

const MemoizedWeekCalendar = memo(WeekCalendar);
const MemoizedTaskPanel = memo(TaskPanel);
const MemoizedKanbanBoard = memo(KanbanBoard);
const MemoizedTaskDetailPane = memo(TaskDetailPane);
const MemoizedSettingsPanel = memo(SettingsPanel);
const MemoizedLeftNavigation = memo(LeftNavigation);
const MemoizedPomodoroPanel = memo(PomodoroPanel);
const MemoizedGlobalSearch = memo(GlobalSearch);

type WorkspaceMode = "list" | "board" | "calendar";

type LoadSnapshotOptions = {
  showLoading?: boolean;
};

type ReadModelRefreshPlan = {
  taskPage?: boolean;
  taskLists?: boolean;
  boardColumns?: boolean;
  tags?: boolean;
  calendar?: boolean;
  runtime?: boolean;
  settings?: boolean;
  notifications?: boolean;
  syncExpiredPomodoro?: boolean;
  syncExpiredTaskCountdown?: boolean;
};

type MutationScope =
  | "navigation"
  | "detail"
  | "board"
  | "calendar"
  | "pomodoro"
  | "settings";

type MutationOptions = {
  refresh: ReadModelRefreshPlan;
  scope: MutationScope;
  invalidateCalendar?: boolean;
  invalidateBoardColumns?: boolean;
};

type MutationRefreshOptions = Omit<MutationOptions, "scope">;

type TaskPageViewState = {
  scopeKey: string;
  loadedCount: number;
  totalCount: number;
  nextCursor: TaskPageCursor | null;
};

const NOTIFICATION_SCHEDULER_MAX_TIMEOUT_MS = 60_000;
const NOTIFICATION_SCHEDULER_DUE_DELAY_MS = 500;
const TASK_PAGE_SIZE = 200;
const workspaceModes: { value: WorkspaceMode; label: string }[] = [
  { value: "list", label: "リスト" },
  { value: "board", label: "かんばん" },
  { value: "calendar", label: "カレンダー" },
];
const INITIAL_MUTATION_COUNTS: Record<MutationScope, number> = {
  navigation: 0,
  detail: 0,
  board: 0,
  calendar: 0,
  pomodoro: 0,
  settings: 0,
};
const TASK_CONTENT_REFRESH = {
  refresh: { taskPage: true, taskLists: true, notifications: true },
  invalidateCalendar: true,
} satisfies MutationRefreshOptions;
const TASK_LIFECYCLE_REFRESH = {
  refresh: { taskPage: true, taskLists: true, notifications: true },
  invalidateCalendar: true,
  invalidateBoardColumns: true,
} satisfies MutationRefreshOptions;
const TASK_TIMER_REFRESH = {
  refresh: { taskPage: true, runtime: true, notifications: true },
} satisfies MutationRefreshOptions;
const POMODORO_REFRESH = {
  refresh: { runtime: true, notifications: true },
} satisfies MutationRefreshOptions;

export function App() {
  usePresentationRenderProbe("App");
  const [tasks, setTasks] = useState<TaskWithSubtasks[]>([]);
  const [taskRows, setTaskRows] = useState<TaskRow[]>([]);
  const [taskPageState, setTaskPageState] = useState<TaskPageViewState>({
    scopeKey: "",
    loadedCount: 0,
    totalCount: 0,
    nextCursor: null,
  });
  const [taskNavigationCounts, setTaskNavigationCounts] = useState({
    todayCount: 0,
    favoriteCount: 0,
  });
  const [taskLists, setTaskLists] = useState<TaskListItem[]>([]);
  const [boardColumns, setBoardColumns] = useState<BoardColumn[]>([]);
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
  const [workspaceScope, setWorkspaceScope] = useState<WorkspaceScope>({
    kind: "list",
    listId: DEFAULT_TASK_LIST_ID,
  });
  const [workspaceMode, setWorkspaceMode] = useState<WorkspaceMode>("list");
  const [selectedTaskOverride, setSelectedTaskOverride] =
    useState<TaskWithSubtasks | null>(null);
  const [taskCreatePreset, setTaskCreatePreset] =
    useState<TaskCreatePreset | null>(null);
  const [taskCreateErrorMessage, setTaskCreateErrorMessage] = useState<
    string | null
  >(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<WorkItemSearchResult[]>([]);
  const [isSearchLoading, setIsSearchLoading] = useState(false);
  const [isSearchOpen, setIsSearchOpen] = useState(false);
  const [searchErrorMessage, setSearchErrorMessage] = useState<string | null>(
    null,
  );
  const [selectedCalendarTarget, setSelectedCalendarTarget] =
    useState<WorkTargetRef | null>(null);
  const [isNavigationOpen, setIsNavigationOpen] = useState(true);
  const [lastTaskListId, setLastTaskListId] = useState(DEFAULT_TASK_LIST_ID);
  const [displayMode, setDisplayMode] =
    useState<NotificationDisplayMode>("title_only");
  const [notificationsEnabled, setNotificationsEnabled] = useState(true);
  const [pomodoroSettings, setPomodoroSettings] =
    useState<PomodoroSettings | null>(null);
  const [taskTimerSettings, setTaskTimerSettings] =
    useState<TaskTimerSettings | null>(null);
  const [notificationSummary, setNotificationSummary] =
    useState<NotificationDispatchSummary | null>(null);
  const [nextNotificationSchedule, setNextNotificationSchedule] =
    useState<NextNotificationSchedule | null>(null);
  const [calendarViewMode, setCalendarViewMode] =
    useState<CalendarViewMode>("week");
  const [calendarAnchorDate, setCalendarAnchorDate] = useState(
    getTodayDateInputValue,
  );
  const [hasHydratedUiPreferences, setHasHydratedUiPreferences] =
    useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [isTaskPageLoading, setIsTaskPageLoading] = useState(false);
  const [isCalendarLoading, setIsCalendarLoading] = useState(false);
  const [mutationCounts, setMutationCounts] = useState(
    INITIAL_MUTATION_COUNTS,
  );
  const [isCreatingTaskPending, setIsCreatingTaskPending] = useState(false);
  const [isLoadingMoreTasks, setIsLoadingMoreTasks] = useState(false);
  const [pendingTaskActionIds, setPendingTaskActionIds] = useState<
    ReadonlySet<string>
  >(new Set());
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const resumeSyncRef = useRef({
    isSyncing: false,
    lastSyncedAt: Date.now(),
  });
  const lastPersistedUiPreferencesRef = useRef<string | null>(null);
  const taskPageStateRef = useRef(taskPageState);
  const taskPageRequestIdRef = useRef(0);
  const taskPageInFlightRef = useRef(false);
  const taskListsRequestIdRef = useRef(0);
  const boardColumnsRequestIdRef = useRef(0);
  const tagsRequestIdRef = useRef(0);
  const calendarRequestIdRef = useRef(0);
  const runtimeRequestIdRef = useRef(0);
  const settingsRequestIdRef = useRef(0);
  const notificationsRequestIdRef = useRef(0);
  const hasLoadedInitialSnapshotRef = useRef(false);
  const isInitialSnapshotLoadingRef = useRef(false);
  const loadedTaskScopeKeyRef = useRef<string | null>(null);
  const loadedCalendarQueryKeyRef = useRef<string | null>(null);
  const calendarInvalidationVersionRef = useRef(0);
  const loadedCalendarInvalidationVersionRef = useRef(-1);
  const boardColumnsInvalidationVersionRef = useRef(0);
  const loadedBoardColumnsInvalidationVersionRef = useRef(-1);
  const loadMoreRequestIdRef = useRef(0);
  const searchRequestIdRef = useRef(0);
  const selectedTaskOverrideRef = useRef(selectedTaskOverride);
  const activeViewRef = useRef(activeView);
  const refreshReadModelsRef = useRef<
    (plan: ReadModelRefreshPlan) => Promise<void>
  >(async () => undefined);
  activeViewRef.current = activeView;
  selectedTaskOverrideRef.current = selectedTaskOverride;

  const isNavigationMutating = mutationCounts.navigation > 0;
  const isDetailMutating = mutationCounts.detail > 0;
  const isBoardMutating = mutationCounts.board > 0;
  const isCalendarMutating = mutationCounts.calendar > 0;
  const isPomodoroMutating = mutationCounts.pomodoro > 0;
  const isSettingsMutating = mutationCounts.settings > 0;
  const visiblePendingTaskActionIds = useMemo(() => {
    if (!isDetailMutating || !selectedTaskId) {
      return pendingTaskActionIds;
    }
    const next = new Set(pendingTaskActionIds);
    next.add(selectedTaskId);
    return next;
  }, [isDetailMutating, pendingTaskActionIds, selectedTaskId]);

  const favoriteCount = taskNavigationCounts.favoriteCount;
  const todayDate = getTodayDateInputValue();
  const todayCount = taskNavigationCounts.todayCount;
  const taskPageScope = useMemo(
    () => taskPageScopeFromWorkspaceScope(workspaceScope),
    [workspaceScope],
  );
  const taskPageScopeKey = useMemo(
    () => serializeTaskPageScope(taskPageScope, todayDate),
    [taskPageScope, todayDate],
  );

  const activeTaskList = useMemo(() => {
    if (workspaceScope.kind !== "list") {
      return null;
    }
    return taskLists.find((list) => list.id === workspaceScope.listId) ?? null;
  }, [taskLists, workspaceScope]);

  const defaultTaskCreateList = useMemo(() => {
    const preferredListId =
      workspaceScope.kind === "list" ? workspaceScope.listId : lastTaskListId;
    return (
      taskLists.find((list) => list.id === preferredListId) ??
      taskLists[0] ??
      null
    );
  }, [lastTaskListId, taskLists, workspaceScope]);

  const defaultTaskCreatePreset = useMemo<
    Extract<TaskCreatePreset, { kind: "standard" }>
  >(
    () => ({
      kind: "standard",
      listId: defaultTaskCreateList?.id ?? DEFAULT_TASK_LIST_ID,
      plannedStartDate: workspaceScope.kind === "today" ? todayDate : null,
      dueDate: null,
      dueTime: null,
      sourceLabel:
        workspaceScope.kind === "today"
          ? "今日のタスクとして追加"
          : `${defaultTaskCreateList?.name ?? "タスク"}に追加`,
    }),
    [defaultTaskCreateList, todayDate, workspaceScope.kind],
  );

  const calendarRange = useMemo(
    () => getCalendarQueryRange(calendarViewMode, calendarAnchorDate),
    [calendarAnchorDate, calendarViewMode],
  );
  const calendarQueryKey = `${calendarRange.startDate}:${calendarRange.endDate}:${taskPageScopeKey}`;

  const visibleTasks = tasks;
  const visibleTaskRows = taskRows;

  const selectedTask = useMemo(() => {
    if (!selectedTaskId) {
      return null;
    }
    return (
      visibleTasks.find((task) => task.id === selectedTaskId) ??
      (selectedTaskOverride?.id === selectedTaskId ? selectedTaskOverride : null)
    );
  }, [selectedTaskId, selectedTaskOverride, visibleTasks]);

  const selectedSubtask = useMemo(() => {
    if (!selectedSubtaskId) {
      return null;
    }
    return (
      selectedTask?.subtasks.find((subtask) => subtask.id === selectedSubtaskId) ??
      null
    );
  }, [selectedSubtaskId, selectedTask]);

  const refreshTaskPage = useCallback(
    async (showLoading = false) => {
      const requestId = ++taskPageRequestIdRef.current;
      taskPageInFlightRef.current = true;
      loadMoreRequestIdRef.current += 1;
      const currentPageState = taskPageStateRef.current;
      const targetTaskCount =
        currentPageState.scopeKey === taskPageScopeKey
          ? Math.max(currentPageState.loadedCount, TASK_PAGE_SIZE)
          : TASK_PAGE_SIZE;
      if (showLoading) {
        setIsTaskPageLoading(true);
      }
      setIsLoadingMoreTasks(false);

      try {
        const nextTaskPage = await loadTaskPageWindow(
          taskPageScope,
          todayDate,
          targetTaskCount,
        );
        if (requestId !== taskPageRequestIdRef.current) {
          return;
        }
        const nextPageState = createTaskPageViewState(
          taskPageScopeKey,
          nextTaskPage,
        );
        taskPageStateRef.current = nextPageState;
        loadedTaskScopeKeyRef.current = taskPageScopeKey;
        setTasks(nextTaskPage.tasks);
        setTaskRows(nextTaskPage.rows);
        setTaskPageState(nextPageState);
        setTaskNavigationCounts(nextTaskPage.navigationCounts);
      } finally {
        if (requestId === taskPageRequestIdRef.current) {
          taskPageInFlightRef.current = false;
          setIsTaskPageLoading(false);
        }
      }
    },
    [taskPageScope, taskPageScopeKey, todayDate],
  );

  const refreshTaskLists = useCallback(async () => {
    const requestId = ++taskListsRequestIdRef.current;
    const nextTaskLists = await tauriTaskTimerGateway.listTaskLists();
    if (requestId === taskListsRequestIdRef.current) {
      setTaskLists(nextTaskLists);
    }
  }, []);

  const refreshBoardColumns = useCallback(async () => {
    const requestId = ++boardColumnsRequestIdRef.current;
    const invalidationVersion = boardColumnsInvalidationVersionRef.current;
    const nextBoardColumns = await tauriTaskTimerGateway.listBoardColumns();
    if (requestId === boardColumnsRequestIdRef.current) {
      setBoardColumns(nextBoardColumns);
      loadedBoardColumnsInvalidationVersionRef.current = invalidationVersion;
    }
  }, []);

  const refreshTags = useCallback(async () => {
    const requestId = ++tagsRequestIdRef.current;
    const nextTags = await tauriTaskTimerGateway.listTags();
    if (requestId === tagsRequestIdRef.current) {
      setTags(nextTags);
    }
  }, []);

  const refreshCalendar = useCallback(
    async (showLoading = false) => {
      const requestId = ++calendarRequestIdRef.current;
      const invalidationVersion = calendarInvalidationVersionRef.current;
      if (showLoading) {
        setIsCalendarLoading(true);
      }
      try {
        const nextItems = await tauriTaskTimerGateway.listCalendarItems(
          calendarRange.startDate,
          calendarRange.endDate,
          taskPageScope,
          todayDate,
        );
        if (requestId === calendarRequestIdRef.current) {
          setItems(nextItems);
          loadedCalendarQueryKeyRef.current = calendarQueryKey;
          loadedCalendarInvalidationVersionRef.current = invalidationVersion;
        }
      } finally {
        if (requestId === calendarRequestIdRef.current) {
          setIsCalendarLoading(false);
        }
      }
    },
    [
      calendarQueryKey,
      calendarRange.endDate,
      calendarRange.startDate,
      taskPageScope,
      todayDate,
    ],
  );

  const refreshRuntime = useCallback(async (
    syncExpiredPomodoro = false,
    syncExpiredTaskCountdown = false,
  ) => {
    const requestId = ++runtimeRequestIdRef.current;
    const [pomodoroSyncResult, countdownSyncResult] = await Promise.all([
      syncExpiredPomodoro
        ? tauriTaskTimerGateway.syncExpiredPomodoro()
        : Promise.resolve(null),
      syncExpiredTaskCountdown
        ? tauriTaskTimerGateway.syncExpiredTaskCountdown()
        : Promise.resolve(null),
    ]);
    const [nextActiveTimer, nextActivePomodoro] = await Promise.all([
      tauriTaskTimerGateway.getActiveTimer(),
      tauriTaskTimerGateway.getActivePomodoro(),
    ]);
    if (requestId !== runtimeRequestIdRef.current) {
      return null;
    }
    setActiveTimer(nextActiveTimer);
    setActivePomodoro(nextActivePomodoro);
    const pomodoroSummary = pomodoroSyncResult?.notificationSummary ?? null;
    const countdownSummary = countdownSyncResult?.notificationSummary ?? null;
    return pomodoroSummary && countdownSummary
      ? combineNotificationSummaries(pomodoroSummary, countdownSummary)
      : pomodoroSummary ?? countdownSummary;
  }, []);

  const refreshSettings = useCallback(async () => {
    const requestId = ++settingsRequestIdRef.current;
    const [
      nextPomodoroSettings,
      nextTaskTimerSettings,
      nextDisplayMode,
      nextNotificationsEnabled,
    ] =
      await Promise.all([
        tauriTaskTimerGateway.getPomodoroSettings(),
        tauriTaskTimerGateway.getTaskTimerSettings(),
        tauriTaskTimerGateway.getNotificationDisplayMode(),
        tauriTaskTimerGateway.getNotificationsEnabled(),
      ]);
    if (requestId === settingsRequestIdRef.current) {
      setPomodoroSettings(nextPomodoroSettings);
      setTaskTimerSettings(nextTaskTimerSettings);
      setDisplayMode(nextDisplayMode);
      setNotificationsEnabled(nextNotificationsEnabled);
    }
  }, []);

  const refreshNotifications = useCallback(
    async (priorSummary: NotificationDispatchSummary | null = null) => {
      const requestId = ++notificationsRequestIdRef.current;
      const notificationSyncResult =
        await tauriTaskTimerGateway.syncNotifications();
      await tauriTaskTimerGateway.processNativeNotificationRegistrations();
      if (requestId !== notificationsRequestIdRef.current) {
        return;
      }
      setNotificationSummary(
        priorSummary
          ? combineNotificationSummaries(
              priorSummary,
              notificationSyncResult.dispatchSummary,
            )
          : notificationSyncResult.dispatchSummary,
      );
      setNextNotificationSchedule(notificationSyncResult.nextSchedule);
    },
    [],
  );

  const refreshReadModels = useCallback(
    async (plan: ReadModelRefreshPlan) => {
      let runtimeSummary: NotificationDispatchSummary | null = null;
      const shouldSyncExpiredRuntime = Boolean(
        plan.syncExpiredPomodoro || plan.syncExpiredTaskCountdown,
      );
      if (shouldSyncExpiredRuntime) {
        runtimeSummary = await refreshRuntime(
          plan.syncExpiredPomodoro,
          plan.syncExpiredTaskCountdown,
        );
      }

      const refreshes: Promise<void>[] = [];
      if (plan.taskPage) {
        refreshes.push(refreshTaskPage());
      }
      if (plan.taskLists) {
        refreshes.push(refreshTaskLists());
      }
      if (plan.boardColumns) {
        refreshes.push(refreshBoardColumns());
      }
      if (plan.tags) {
        refreshes.push(refreshTags());
      }
      if (plan.calendar) {
        refreshes.push(refreshCalendar());
      }
      if (plan.runtime && !shouldSyncExpiredRuntime) {
        refreshes.push(
          refreshRuntime().then((summary) => {
            runtimeSummary = summary;
          }),
        );
      }
      if (plan.settings) {
        refreshes.push(refreshSettings());
      }
      await Promise.all(refreshes);
      if (plan.notifications) {
        await refreshNotifications(runtimeSummary);
      }
    },
    [
      refreshBoardColumns,
      refreshCalendar,
      refreshNotifications,
      refreshRuntime,
      refreshSettings,
      refreshTags,
      refreshTaskLists,
      refreshTaskPage,
    ],
  );
  refreshReadModelsRef.current = refreshReadModels;

  const loadSnapshot = useCallback(
    async (options?: LoadSnapshotOptions) => {
      const showLoading = options?.showLoading ?? true;
      if (showLoading) {
        setIsLoading(true);
      }
      setErrorMessage(null);
      try {
        await tauriTaskTimerGateway.healthCheck();
        await refreshReadModels({
          taskPage: true,
          taskLists: true,
          boardColumns: true,
          tags: true,
          calendar: true,
          runtime: true,
          settings: true,
          notifications: true,
          syncExpiredPomodoro: true,
          syncExpiredTaskCountdown: true,
        });
      } catch (error) {
        setNextNotificationSchedule(null);
        setErrorMessage(toErrorMessage(error));
      } finally {
        if (showLoading) {
          setIsLoading(false);
        }
      }
    },
    [refreshReadModels],
  );

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
        const restoredView = appViewFromPreferences(preferences);
        setActiveView(restoredView);
        setWorkspaceScope(workspaceScopeFromPreferences(preferences));
        setWorkspaceMode(workspaceModeFromView(restoredView));
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
    if (
      hasLoadedInitialSnapshotRef.current ||
      isInitialSnapshotLoadingRef.current
    ) {
      return;
    }
    isInitialSnapshotLoadingRef.current = true;
    void loadSnapshot({ showLoading: true }).finally(() => {
      isInitialSnapshotLoadingRef.current = false;
      hasLoadedInitialSnapshotRef.current = true;
    });
  }, [hasHydratedUiPreferences, loadSnapshot]);

  useEffect(() => {
    const requestId = ++searchRequestIdRef.current;
    const query = searchQuery.trim();
    if (!query) {
      setSearchResults([]);
      setSearchErrorMessage(null);
      setIsSearchLoading(false);
      setIsSearchOpen(false);
      return;
    }

    setIsSearchLoading(true);
    setSearchErrorMessage(null);
    setIsSearchOpen(true);
    const timerId = window.setTimeout(() => {
      void tauriTaskTimerGateway
        .searchWorkItems(query, 50)
        .then((results) => {
          if (requestId === searchRequestIdRef.current) {
            setSearchResults(results);
          }
        })
        .catch(() => {
          if (requestId === searchRequestIdRef.current) {
            setSearchResults([]);
            setSearchErrorMessage("検索に失敗しました。もう一度お試しください。");
          }
        })
        .finally(() => {
          if (requestId === searchRequestIdRef.current) {
            setIsSearchLoading(false);
          }
        });
    }, 200);

    return () => window.clearTimeout(timerId);
  }, [searchQuery]);

  useEffect(() => {
    if (
      !hasLoadedInitialSnapshotRef.current ||
      activeView.kind === "settings" ||
      activeView.kind === "pomodoro" ||
      loadedTaskScopeKeyRef.current === taskPageScopeKey
    ) {
      return;
    }
    setErrorMessage(null);
    void refreshTaskPage(true).catch((error) => {
      setErrorMessage(toErrorMessage(error));
    });
  }, [activeView.kind, refreshTaskPage, taskPageScopeKey]);

  useEffect(() => {
    if (!hasLoadedInitialSnapshotRef.current || activeView.kind !== "calendar") {
      return;
    }
    const isCurrentRangeLoaded =
      loadedCalendarQueryKeyRef.current === calendarQueryKey;
    const isCurrentVersionLoaded =
      loadedCalendarInvalidationVersionRef.current ===
      calendarInvalidationVersionRef.current;
    if (isCurrentRangeLoaded && isCurrentVersionLoaded) {
      return;
    }
    setErrorMessage(null);
    void refreshCalendar(true).catch((error) => {
      setErrorMessage(toErrorMessage(error));
    });
  }, [activeView.kind, calendarQueryKey, refreshCalendar]);

  useEffect(() => {
    if (
      !hasLoadedInitialSnapshotRef.current ||
      activeView.kind !== "board" ||
      loadedBoardColumnsInvalidationVersionRef.current ===
        boardColumnsInvalidationVersionRef.current
    ) {
      return;
    }
    setErrorMessage(null);
    void refreshBoardColumns().catch((error) => {
      setErrorMessage(toErrorMessage(error));
    });
  }, [activeView.kind, refreshBoardColumns]);

  useEffect(() => {
    if (workspaceScope.kind === "list") {
      setLastTaskListId(workspaceScope.listId);
    }
  }, [workspaceScope]);

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
        if (taskCreatePreset) {
          return;
        }
        setIsNavigationOpen((current) => !current);
        return;
      }
      if (
        event.ctrlKey &&
        event.key.toLowerCase() === "n" &&
        activeView.kind !== "settings" &&
        activeView.kind !== "pomodoro"
      ) {
        event.preventDefault();
        setTaskCreateErrorMessage(null);
        setTaskCreatePreset((current) =>
          current ?? { ...defaultTaskCreatePreset },
        );
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [activeView.kind, defaultTaskCreatePreset, taskCreatePreset]);

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
      void refreshReadModels({
        runtime: true,
        notifications: true,
        syncExpiredPomodoro: true,
        syncExpiredTaskCountdown: true,
      })
        .catch((error) => {
          setErrorMessage(toErrorMessage(error));
        })
        .finally(() => {
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
  }, [refreshReadModels]);

  useEffect(() => {
    const timeoutMs = getPomodoroSyncTimeoutMs(activePomodoro);
    if (timeoutMs === null) {
      return;
    }

    const timerId = window.setTimeout(() => {
      void refreshReadModels({
        runtime: true,
        notifications: true,
        syncExpiredPomodoro: true,
      }).catch((error) => {
        setErrorMessage(toErrorMessage(error));
      });
    }, timeoutMs);
    return () => window.clearTimeout(timerId);
  }, [activePomodoro, refreshReadModels]);

  useEffect(() => {
    const timeoutMs = getTaskCountdownSyncTimeoutMs(activeTimer);
    if (timeoutMs === null) {
      return;
    }

    const timerId = window.setTimeout(() => {
      void refreshReadModels({
        taskPage: true,
        runtime: true,
        notifications: true,
        syncExpiredTaskCountdown: true,
      }).catch((error) => {
        setErrorMessage(toErrorMessage(error));
      });
    }, timeoutMs);
    return () => window.clearTimeout(timerId);
  }, [activeTimer, refreshReadModels]);

  useEffect(() => {
    if (
      !hasHydratedUiPreferences ||
      !notificationsEnabled ||
      !nextNotificationSchedule
    ) {
      return;
    }

    const timerId = window.setTimeout(() => {
      void refreshReadModels({ notifications: true }).catch((error) => {
        setErrorMessage(toErrorMessage(error));
      });
    }, getNotificationScheduleTimeoutMs(nextNotificationSchedule.notifyAt));

    return () => {
      window.clearTimeout(timerId);
    };
  }, [
    hasHydratedUiPreferences,
    nextNotificationSchedule,
    notificationsEnabled,
    refreshReadModels,
  ]);

  useEffect(() => {
    if (workspaceScope.kind !== "list" || taskLists.length === 0) {
      return;
    }
    if (!taskLists.some((list) => list.id === workspaceScope.listId)) {
      const nextScope: WorkspaceScope = {
        kind: "list",
        listId: taskLists[0].id,
      };
      setWorkspaceScope(nextScope);
      if (workspaceMode === "list") {
        setActiveView(nextScope);
      }
    }
  }, [taskLists, workspaceMode, workspaceScope]);

  useEffect(() => {
    if (
      activeView.kind !== "calendar" ||
      !selectedCalendarTarget ||
      isLoading ||
      isCalendarLoading
    ) {
      return;
    }

    if (!items.some((item) => isSameTarget(item.target, selectedCalendarTarget))) {
      setSelectedTaskId(null);
      setSelectedCalendarTarget(null);
    }
  }, [
    activeView.kind,
    isCalendarLoading,
    isLoading,
    items,
    selectedCalendarTarget,
  ]);

  useEffect(() => {
    if (activeView.kind === "settings" || activeView.kind === "pomodoro") {
      setSelectedTaskId(null);
      setSelectedSubtaskId(null);
      setSelectedCalendarTarget(null);
      setSelectedTaskOverride(null);
      return;
    }

    if (activeView.kind === "calendar") {
      if (
        selectedTaskId &&
        selectedTaskOverride?.id !== selectedTaskId &&
        !tasks.some((task) => task.id === selectedTaskId)
      ) {
        setSelectedTaskId(null);
        setSelectedSubtaskId(null);
        setSelectedCalendarTarget(null);
        setSelectedTaskOverride(null);
      }
      return;
    }

    if (
      selectedTaskId &&
      selectedTaskOverride?.id !== selectedTaskId &&
      !visibleTaskRows.some((task) => task.id === selectedTaskId)
    ) {
      setSelectedTaskId(null);
      setSelectedSubtaskId(null);
      setSelectedCalendarTarget(null);
      setSelectedTaskOverride(null);
    }
  }, [
    activeView.kind,
    selectedTaskId,
    selectedTaskOverride,
    tasks,
    visibleTaskRows,
  ]);

  useEffect(() => {
    if (selectedSubtaskId && !selectedSubtask) {
      setSelectedSubtaskId(null);
    }
  }, [selectedSubtask, selectedSubtaskId]);

  const clearDetailSelection = useCallback(() => {
    setSelectedTaskId(null);
    setSelectedSubtaskId(null);
    setSelectedCalendarTarget(null);
    setSelectedTaskOverride(null);
  }, []);

  const handleToggleNavigation = useCallback(() => {
    setIsNavigationOpen((current) => !current);
  }, []);

  const handleLoadMoreTasks = useCallback(async () => {
    const currentPageState = taskPageStateRef.current;
    if (
      isLoadingMoreTasks ||
      taskPageInFlightRef.current ||
      currentPageState.scopeKey !== taskPageScopeKey ||
      !currentPageState.nextCursor
    ) {
      return;
    }

    const requestId = ++loadMoreRequestIdRef.current;
    setIsLoadingMoreTasks(true);
    setErrorMessage(null);
    try {
      const nextPage = await tauriTaskTimerGateway.listTaskPage({
        scope: taskPageScope,
        todayDate,
        cursor: currentPageState.nextCursor,
        limit: TASK_PAGE_SIZE,
      });
      if (
        requestId !== loadMoreRequestIdRef.current ||
        taskPageStateRef.current.scopeKey !== taskPageScopeKey
      ) {
        return;
      }

      setTasks((current) => appendUniqueById(current, nextPage.tasks));
      setTaskRows((current) => appendUniqueById(current, nextPage.rows));
      const nextPageState: TaskPageViewState = {
        scopeKey: taskPageScopeKey,
        loadedCount: Math.min(
          currentPageState.loadedCount + nextPage.rows.length,
          nextPage.totalCount,
        ),
        totalCount: nextPage.totalCount,
        nextCursor: nextPage.nextCursor,
      };
      taskPageStateRef.current = nextPageState;
      setTaskPageState(nextPageState);
      setTaskNavigationCounts(nextPage.navigationCounts);
    } catch (error) {
      if (requestId === loadMoreRequestIdRef.current) {
        setErrorMessage(toErrorMessage(error));
      }
    } finally {
      if (requestId === loadMoreRequestIdRef.current) {
        setIsLoadingMoreTasks(false);
      }
    }
  }, [isLoadingMoreTasks, taskPageScope, taskPageScopeKey, todayDate]);

  const updateMutationCount = useCallback(
    (scope: MutationScope, difference: 1 | -1) => {
      setMutationCounts((current) => ({
        ...current,
        [scope]: Math.max(0, current[scope] + difference),
      }));
    },
    [],
  );

  const refreshAfterMutation = useCallback(
    async (options: MutationRefreshOptions) => {
      const plan = { ...options.refresh };
      if (options.invalidateCalendar) {
        calendarInvalidationVersionRef.current += 1;
        if (activeViewRef.current.kind === "calendar") {
          plan.calendar = true;
        }
      }
      if (options.invalidateBoardColumns) {
        boardColumnsInvalidationVersionRef.current += 1;
        if (activeViewRef.current.kind === "board") {
          plan.boardColumns = true;
        }
      }
      await refreshReadModelsRef.current(plan);
    },
    [],
  );

  const refreshSelectedTaskOverride = useCallback(async () => {
    const taskId = selectedTaskOverrideRef.current?.id;
    if (!taskId) {
      return;
    }
    try {
      const task = await tauriTaskTimerGateway.getTaskDetail(taskId);
      if (selectedTaskOverrideRef.current?.id === taskId) {
        setSelectedTaskOverride(task);
      }
    } catch {
      if (selectedTaskOverrideRef.current?.id === taskId) {
        clearDetailSelection();
      }
    }
  }, [clearDetailSelection]);

  const runMutation = useCallback(
    async (
      operation: () => Promise<string | void>,
      options: MutationOptions,
    ) => {
      updateMutationCount(options.scope, 1);
      setErrorMessage(null);

      try {
        const nextSelectedTaskId = await operation();
        await refreshAfterMutation(options);
        await refreshSelectedTaskOverride();
        if (nextSelectedTaskId) {
          setSelectedTaskId(nextSelectedTaskId);
        }
        return true;
      } catch (error) {
        setErrorMessage(toErrorMessage(error));
        return false;
      } finally {
        updateMutationCount(options.scope, -1);
      }
    },
    [refreshAfterMutation, refreshSelectedTaskOverride, updateMutationCount],
  );

  const runTaskActionMutation = useCallback(
    async (
      taskId: string,
      operation: () => Promise<string | void>,
      options: MutationRefreshOptions,
    ) => {
      setPendingTaskActionIds((current) => new Set(current).add(taskId));
      setErrorMessage(null);

      try {
        const nextSelectedTaskId = await operation();
        await refreshAfterMutation(options);
        await refreshSelectedTaskOverride();
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
    [refreshAfterMutation, refreshSelectedTaskOverride],
  );

  const runCreateTaskMutation = useCallback(
    async (
      operation: () => Promise<void>,
      options: MutationRefreshOptions,
    ) => {
      setIsCreatingTaskPending(true);
      setErrorMessage(null);
      setTaskCreateErrorMessage(null);

      try {
        await operation();
        await refreshAfterMutation(options);
        return true;
      } catch (error) {
        const message = toErrorMessage(error);
        setErrorMessage(message);
        setTaskCreateErrorMessage(message);
        return false;
      } finally {
        setIsCreatingTaskPending(false);
      }
    },
    [refreshAfterMutation],
  );

  const handleCreateTask = useCallback(
    (input: WorkItemDraft, boardColumnId?: string | null) =>
      runCreateTaskMutation(async () => {
        const draft = {
          ...input,
          listId:
            input.listId ??
            (workspaceScope.kind === "list"
              ? workspaceScope.listId
              : DEFAULT_TASK_LIST_ID),
        };
        if (boardColumnId) {
          await tauriTaskTimerGateway.createTaskInBoardColumn(
            draft,
            boardColumnId,
          );
        } else {
          await tauriTaskTimerGateway.createTask(draft);
        }
      }, TASK_LIFECYCLE_REFRESH),
    [runCreateTaskMutation, workspaceScope],
  );

  const handleCreateScheduledTask = useCallback(
    (input: ScheduledTaskDraft) =>
      runCreateTaskMutation(async () => {
        await tauriTaskTimerGateway.createScheduledTask({
          ...input,
          listId: input.listId ?? DEFAULT_TASK_LIST_ID,
        });
      }, TASK_LIFECYCLE_REFRESH),
    [runCreateTaskMutation],
  );

  const handleRequestTaskCreate = useCallback(
    (boardColumnId?: string, boardColumnTitle?: string) => {
      setTaskCreateErrorMessage(null);
      setTaskCreatePreset((current) =>
        current ?? {
          ...defaultTaskCreatePreset,
          sourceLabel: boardColumnTitle
            ? `状態: ${boardColumnTitle}`
            : defaultTaskCreatePreset.sourceLabel,
          boardColumnId: boardColumnId ?? null,
        },
      );
    },
    [defaultTaskCreatePreset],
  );

  const handleRequestScheduledTaskCreate = useCallback(
    ({ schedule, sourceLabel }: CalendarTaskCreatePreset) => {
      setTaskCreateErrorMessage(null);
      setTaskCreatePreset((current) =>
        current ?? {
          kind: "scheduled",
          listId: defaultTaskCreateList?.id ?? DEFAULT_TASK_LIST_ID,
          schedule,
          sourceLabel,
        },
      );
    },
    [defaultTaskCreateList],
  );

  const handleSubmitTaskCreate = useCallback(
    (submission: TaskCreateSubmission) =>
      submission.kind === "standard"
        ? handleCreateTask(submission.input, submission.boardColumnId)
        : handleCreateScheduledTask(submission.input),
    [handleCreateScheduledTask, handleCreateTask],
  );

  const handleCloseTaskCreate = useCallback(() => {
    if (!isCreatingTaskPending) {
      setTaskCreateErrorMessage(null);
      setTaskCreatePreset(null);
    }
  }, [isCreatingTaskPending]);

  const handleCreateTaskList = useCallback(
    (name: string) =>
      runMutation(async () => {
        const list = await tauriTaskTimerGateway.createTaskList({ name });
        const nextScope: WorkspaceScope = { kind: "list", listId: list.id };
        setWorkspaceScope(nextScope);
        setWorkspaceMode("list");
        setActiveView(nextScope);
        clearDetailSelection();
      }, {
        scope: "navigation",
        refresh: { taskLists: true },
      }),
    [clearDetailSelection, runMutation],
  );

  const handleUpdateTaskList = useCallback(
    (listId: string, name: string, colorToken: TaskListColorToken) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.updateTaskList(listId, {
          name,
          colorToken,
        });
      }, {
        scope: "navigation",
        refresh: { taskLists: true },
        invalidateCalendar: true,
      }),
    [runMutation],
  );

  const handleDeleteTaskList = useCallback(
    (listId: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.deleteTaskList(listId);
        if (workspaceScope.kind === "list" && workspaceScope.listId === listId) {
          const nextScope: WorkspaceScope = {
            kind: "list",
            listId: DEFAULT_TASK_LIST_ID,
          };
          setWorkspaceScope(nextScope);
          if (workspaceMode === "list") {
            setActiveView(nextScope);
          }
          clearDetailSelection();
        }
      }, {
        scope: "navigation",
        refresh: { taskLists: true },
      }),
    [clearDetailSelection, runMutation, workspaceMode, workspaceScope],
  );

  const handleCreateTag = useCallback(
    (name: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.createTag({ name });
      }, {
        scope: "detail",
        refresh: { tags: true },
      }),
    [runMutation],
  );

  const handleRenameTag = useCallback(
    (tagId: string, name: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.updateTag(tagId, { name });
      }, {
        scope: "detail",
        refresh: { taskPage: true, tags: true },
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
      }, {
        scope: "detail",
        refresh: { taskPage: true, tags: true },
      }),
    [activeView, clearDetailSelection, runMutation],
  );

  const handleAttachTagToTask = useCallback(
    (taskId: string, tagId: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.attachTagToTask(taskId, tagId);
        return taskId;
      }, {
        scope: "detail",
        refresh: { taskPage: true, tags: true },
      }),
    [runMutation],
  );

  const handleDetachTagFromTask = useCallback(
    (taskId: string, tagId: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.detachTagFromTask(taskId, tagId);
        return taskId;
      }, {
        scope: "detail",
        refresh: { taskPage: true, tags: true },
      }),
    [runMutation],
  );

  const handleCreateSubtask = useCallback(
    (taskId: string, input: WorkItemDraft) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.createSubtask({ ...input, taskId });
        return taskId;
      }, {
        scope: "detail",
        ...TASK_CONTENT_REFRESH,
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
      }, {
        scope: "detail",
        ...TASK_CONTENT_REFRESH,
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
      }, {
        scope: "detail",
        ...TASK_CONTENT_REFRESH,
      }),
    [runMutation],
  );

  const handleStartTimer = useCallback(
    (target: WorkTargetRef) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.startTimer(target);
      }, {
        scope: "detail",
        ...TASK_TIMER_REFRESH,
      }),
    [runMutation],
  );

  const handleStartPomodoro = useCallback(
    () =>
      runMutation(async () => {
        await tauriTaskTimerGateway.startStandalonePomodoro();
      }, {
        scope: "pomodoro",
        ...POMODORO_REFRESH,
      }),
    [runMutation],
  );

  const handlePausePomodoro = useCallback(
    () =>
      runMutation(async () => {
        await tauriTaskTimerGateway.pausePomodoro();
      }, {
        scope: "pomodoro",
        ...POMODORO_REFRESH,
      }),
    [runMutation],
  );

  const handleResumePomodoro = useCallback(
    () =>
      runMutation(async () => {
        await tauriTaskTimerGateway.resumePomodoro();
      }, {
        scope: "pomodoro",
        ...POMODORO_REFRESH,
      }),
    [runMutation],
  );

  const handleCompletePomodoroWork = useCallback(
    () =>
      runMutation(async () => {
        await tauriTaskTimerGateway.completePomodoroWorkPhase();
      }, {
        scope: "pomodoro",
        ...POMODORO_REFRESH,
      }),
    [runMutation],
  );

  const handleCompletePomodoroWorkAndStartBreak = useCallback(
    () =>
      runMutation(async () => {
        const completed =
          await tauriTaskTimerGateway.completePomodoroWorkPhase();
        await tauriTaskTimerGateway.startPomodoroBreak(completed.id);
      }, {
        scope: "pomodoro",
        ...POMODORO_REFRESH,
      }),
    [runMutation],
  );

  const handleSkipPomodoroBreak = useCallback(
    (pomodoroSessionId: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.skipPomodoroBreak(pomodoroSessionId);
      }, {
        scope: "pomodoro",
        ...POMODORO_REFRESH,
      }),
    [runMutation],
  );

  const handleCompletePomodoroBreak = useCallback(
    () =>
      runMutation(async () => {
        await tauriTaskTimerGateway.completePomodoroBreak();
      }, {
        scope: "pomodoro",
        ...POMODORO_REFRESH,
      }),
    [runMutation],
  );

  const handleCompletePomodoroBreakAndStartNext = useCallback(
    () =>
      runMutation(async () => {
        const completed = await tauriTaskTimerGateway.completePomodoroBreak();
        await tauriTaskTimerGateway.skipPomodoroBreak(completed.id);
      }, {
        scope: "pomodoro",
        ...POMODORO_REFRESH,
      }),
    [runMutation],
  );

  const handleCancelPomodoro = useCallback(
    () =>
      runMutation(async () => {
        await tauriTaskTimerGateway.cancelPomodoro();
      }, {
        scope: "pomodoro",
        ...POMODORO_REFRESH,
      }),
    [runMutation],
  );

  const handlePauseTimer = useCallback(
    () =>
      runMutation(async () => {
        await tauriTaskTimerGateway.pauseActiveTimer();
      }, {
        scope: "detail",
        ...TASK_TIMER_REFRESH,
      }),
    [runMutation],
  );

  const handleResumeTimer = useCallback(
    () =>
      runMutation(async () => {
        await tauriTaskTimerGateway.resumeActiveTimer();
      }, {
        scope: "detail",
        ...TASK_TIMER_REFRESH,
      }),
    [runMutation],
  );

  const handleStopTimer = useCallback(
    () =>
      runMutation(async () => {
        await tauriTaskTimerGateway.stopActiveTimer();
      }, {
        scope: "detail",
        ...TASK_TIMER_REFRESH,
      }),
    [runMutation],
  );

  const handleToggleTaskCompletion = useCallback(
    (task: TaskWithSubtasks) => {
      if (task.status === "done") {
        return runTaskActionMutation(task.id, async () => {
          await tauriTaskTimerGateway.reopenTask(task.id);
        }, TASK_LIFECYCLE_REFRESH);
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
      }, TASK_LIFECYCLE_REFRESH);
    },
    [runTaskActionMutation],
  );

  const handleCreateBoardColumn = useCallback(
    (title: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.createBoardColumn(title);
      }, {
        scope: "board",
        refresh: { boardColumns: true },
      }),
    [runMutation],
  );

  const handleRenameBoardColumn = useCallback(
    (columnId: string, title: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.updateBoardColumn(columnId, title);
      }, {
        scope: "board",
        refresh: { boardColumns: true },
      }),
    [runMutation],
  );

  const handleReorderBoardColumns = useCallback(
    (orderedColumnIds: string[]) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.reorderBoardColumns(orderedColumnIds);
      }, {
        scope: "board",
        refresh: { boardColumns: true },
      }),
    [runMutation],
  );

  const handleDeleteBoardColumn = useCallback(
    (columnId: string, moveTasksToColumnId: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.deleteBoardColumn(
          columnId,
          moveTasksToColumnId,
        );
      }, {
        scope: "board",
        refresh: { taskPage: true, boardColumns: true },
      }),
    [runMutation],
  );

  const handleDeleteCompletedBoardColumnTasks = useCallback(
    (boardColumnId: string) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.deleteCompletedTasksInBoardColumn(
          boardColumnId,
        );
      }, {
        scope: "board",
        ...TASK_LIFECYCLE_REFRESH,
      }),
    [runMutation],
  );

  const handleMoveTaskToBoardColumn = useCallback(
    (taskId: string, boardColumnId: string) =>
      runTaskActionMutation(taskId, async () => {
        await tauriTaskTimerGateway.moveTaskToBoardColumn(
          taskId,
          boardColumnId,
        );
      }, {
        refresh: { taskPage: true, boardColumns: true },
      }),
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
      }, {
        scope: "detail",
        ...TASK_CONTENT_REFRESH,
      }),
    [runMutation],
  );

  const handleToggleTaskFavorite = useCallback(
    (taskId: string, isFavorite: boolean) =>
      runTaskActionMutation(taskId, async () => {
        await tauriTaskTimerGateway.toggleTaskFavorite(taskId, isFavorite);
      }, {
        refresh: { taskPage: true },
      }),
    [runTaskActionMutation],
  );

  const handleDeleteTask = useCallback(
    (task: TaskWithSubtasks) => {
      return runMutation(async () => {
        await tauriTaskTimerGateway.deleteTask(task.id);
      }, {
        scope: "detail",
        ...TASK_LIFECYCLE_REFRESH,
      }).then((deleted) => {
        if (deleted) {
          setSelectedTaskId(null);
          setSelectedSubtaskId(null);
          setSelectedCalendarTarget(null);
          setSelectedTaskOverride(null);
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
      }, {
        scope: "detail",
        ...TASK_CONTENT_REFRESH,
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
      }, {
        scope: "settings",
        refresh: { settings: true, notifications: true },
      }),
    [runMutation],
  );

  const handleUpdateNotificationsEnabled = useCallback(
    (enabled: boolean) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.updateNotificationsEnabled(enabled);
      }, {
        scope: "settings",
        refresh: { settings: true, notifications: true },
      }),
    [runMutation],
  );

  const handleUpdatePomodoroSettings = useCallback(
    (input: PomodoroSettingsDraft) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.updatePomodoroSettings(input);
      }, {
        scope: "settings",
        refresh: { settings: true },
      }),
    [runMutation],
  );

  const handleUpdateTaskTimerSettings = useCallback(
    (input: TaskTimerSettingsDraft) =>
      runMutation(async () => {
        await tauriTaskTimerGateway.updateTaskTimerSettings(input);
      }, {
        scope: "settings",
        refresh: { settings: true },
      }),
    [runMutation],
  );

  const handleRetryNotifications = useCallback(
    () =>
      runMutation(async () => {
        return;
      }, {
        scope: "settings",
        refresh: { notifications: true },
      }),
    [runMutation],
  );

  const runDataManagementAction = useCallback(
    async (
      action: () => Promise<DataManagementActionResult>,
    ): Promise<DataManagementActionResult> => {
      updateMutationCount("settings", 1);
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
        updateMutationCount("settings", -1);
      }
    },
    [updateMutationCount],
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
      if (view.kind === "settings" || view.kind === "pomodoro") {
        if (isSameAppView(activeView, view)) {
          return;
        }
        setActiveView(view);
        clearDetailSelection();
      } else if (
        view.kind === "list" ||
        view.kind === "today" ||
        view.kind === "favorites"
      ) {
        const nextScope: WorkspaceScope = view;
        const nextView: AppView =
          workspaceMode === "list" ? nextScope : { kind: workspaceMode };
        if (
          isSameWorkspaceScope(workspaceScope, nextScope) &&
          isSameAppView(activeView, nextView)
        ) {
          return;
        }
        setWorkspaceScope(nextScope);
        setActiveView(nextView);
        clearDetailSelection();
      } else if (view.kind === "board" || view.kind === "calendar") {
        if (workspaceMode === view.kind && isSameAppView(activeView, view)) {
          return;
        }
        setWorkspaceMode(view.kind);
        setActiveView(view);
        clearDetailSelection();
      } else {
        if (isSameAppView(activeView, view)) {
          return;
        }
        setActiveView(view);
        clearDetailSelection();
      }
      if (window.matchMedia("(max-width: 767px)").matches) {
        setIsNavigationOpen(false);
      }
    },
    [
      activeView,
      clearDetailSelection,
      workspaceMode,
      workspaceScope,
    ],
  );

  const handleSelectWorkspaceMode = useCallback(
    (mode: WorkspaceMode) => {
      const nextView: AppView =
        mode === "list" ? workspaceScope : { kind: mode };
      if (workspaceMode === mode && isSameAppView(activeView, nextView)) {
        return;
      }
      setWorkspaceMode(mode);
      setActiveView(nextView);
      clearDetailSelection();
    },
    [activeView, clearDetailSelection, workspaceMode, workspaceScope],
  );

  const handleSelectSearchResult = useCallback(
    async (result: WorkItemSearchResult) => {
      setSearchErrorMessage(null);
      try {
        const task = await tauriTaskTimerGateway.getTaskDetail(result.taskId);
        setSelectedTaskOverride(task);
        setSelectedTaskId(task.id);
        setSelectedSubtaskId(
          result.target.type === "subtask" ? result.target.id : null,
        );
        setSelectedCalendarTarget(null);
        if (activeView.kind === "settings" || activeView.kind === "pomodoro") {
          setActiveView(
            workspaceMode === "list" ? workspaceScope : { kind: workspaceMode },
          );
        }
        setIsSearchOpen(false);
      } catch {
        setSearchErrorMessage("タスク詳細を開けませんでした。");
        setIsSearchOpen(true);
      }
    },
    [activeView.kind, workspaceMode, workspaceScope],
  );

  const handleSelectTask = useCallback(
    (taskId: string) => {
      if (selectedTaskId === taskId && !selectedSubtaskId) {
        clearDetailSelection();
        return;
      }
      setSelectedTaskOverride(null);
      setSelectedTaskId(taskId);
      setSelectedSubtaskId(null);
      setSelectedCalendarTarget(null);
    },
    [clearDetailSelection, selectedSubtaskId, selectedTaskId],
  );

  const handleSelectSubtask = useCallback((taskId: string, subtaskId: string) => {
    setSelectedTaskOverride((current) =>
      current?.id === taskId ? current : null,
    );
    setSelectedTaskId(taskId);
    setSelectedSubtaskId(subtaskId);
    setSelectedCalendarTarget({ type: "subtask", id: subtaskId });
  }, []);

  const handleSelectDetailSubtask = useCallback(
    (subtaskId: string) => {
      if (selectedTaskId) {
        handleSelectSubtask(selectedTaskId, subtaskId);
      }
    },
    [handleSelectSubtask, selectedTaskId],
  );

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
      setSelectedTaskOverride(null);
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
          colorToken: task.colorToken,
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
        colorToken: null,
        recurrenceRule: toRecurrenceRuleDraft(subtask.recurrenceRule),
        memo: subtask.memo,
      });
    },
    [handleUpdateSubtask, handleUpdateTask, tasks],
  );

  const handleResizeCalendarItem = useCallback(
    (item: WeekCalendarItem, schedule: WorkScheduleDraft) => {
      if (item.marker !== "scheduled") {
        setErrorMessage("期間を変更できるのは予定ブロックだけです。");
        return Promise.resolve(false);
      }
      return runMutation(async () => {
        await tauriTaskTimerGateway.resizeScheduledWorkItem(item.target, schedule);
      }, {
        scope: "calendar",
        refresh: {
          taskPage: true,
          calendar: true,
        },
      });
    },
    [runMutation],
  );

  const handleMoveScheduledCalendarItem = useCallback(
    (item: WeekCalendarItem, destination: WorkScheduleMoveDraft) => {
      if (item.marker !== "scheduled") {
        setErrorMessage("移動できるのは予定ブロックだけです。");
        return Promise.resolve(false);
      }
      if (
        item.date === destination.startDate &&
        (item.time ?? null) === (destination.startTime ?? null)
      ) {
        return Promise.resolve(true);
      }
      return runMutation(async () => {
        await tauriTaskTimerGateway.moveScheduledWorkItem(
          item.target,
          destination,
        );
      }, {
        scope: "calendar",
        refresh: {
          taskPage: true,
          calendar: true,
        },
      });
    },
    [runMutation],
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
        <MemoizedGlobalSearch
          query={searchQuery}
          results={searchResults}
          isLoading={isSearchLoading}
          errorMessage={searchErrorMessage}
          isOpen={isSearchOpen}
          isPomodoroActive={activeView.kind === "pomodoro"}
          onChange={setSearchQuery}
          onOpenChange={setIsSearchOpen}
          onSelect={(result) => void handleSelectSearchResult(result)}
          onOpenPomodoro={() => handleSelectView({ kind: "pomodoro" })}
        />
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
        <MemoizedLeftNavigation
          activeView={activeView}
          activeScope={workspaceScope}
          favoriteCount={favoriteCount}
          todayCount={todayCount}
          isOpen={isNavigationOpen}
          taskLists={taskLists}
          isMutating={isNavigationMutating}
          onSelectView={handleSelectView}
          onCreateTaskList={handleCreateTaskList}
          onUpdateTaskList={handleUpdateTaskList}
          onDeleteTaskList={handleDeleteTaskList}
          onToggle={handleToggleNavigation}
        />

        <section className="workspace-main" aria-label="現在のビュー">
          {activeView.kind !== "settings" && activeView.kind !== "pomodoro" ? (
            <div className="workspace-mode-bar">
              <div
                className="workspace-mode-switcher"
                role="tablist"
                aria-label="表示形式"
              >
                {workspaceModes.map((mode) => (
                  <button
                    type="button"
                    role="tab"
                    aria-selected={workspaceMode === mode.value}
                    className={workspaceMode === mode.value ? "is-active" : ""}
                    key={mode.value}
                    onClick={() => handleSelectWorkspaceMode(mode.value)}
                  >
                    {mode.label}
                  </button>
                ))}
              </div>
            </div>
          ) : null}
          {activeView.kind === "list" ||
          activeView.kind === "today" ||
          activeView.kind === "favorites" ||
          activeView.kind === "tag" ? (
            <div
              className={`task-workspace ${
                selectedTask ? "is-detail-open" : ""
              }`}
            >
              <MemoizedTaskPanel
                tasks={visibleTasks}
                taskRows={visibleTaskRows}
                selectedTaskId={selectedTaskId}
                activeTimer={activeTimer}
                activePomodoro={activePomodoro}
                eyebrow={getTaskPanelEyebrow(workspaceScope)}
                title={
                  workspaceScope.kind === "today"
                    ? "今日"
                    : workspaceScope.kind === "favorites"
                      ? "お気に入り"
                      : activeTaskList?.name ?? "タスク"
                }
                emptyMessage={
                  workspaceScope.kind === "today"
                    ? "今日が開始予定または期限のタスクはありません。"
                    : workspaceScope.kind === "favorites"
                      ? "お気に入りにしたタスクはまだありません。"
                      : "まだタスクはありません。"
                }
                showTaskAdd={
                  workspaceScope.kind === "list" || workspaceScope.kind === "today"
                }
                isLoading={isLoading || isTaskPageLoading}
                isMutating={isDetailMutating}
                isCreatingTaskPending={isCreatingTaskPending}
                isLoadingMore={isLoadingMoreTasks}
                totalTaskCount={taskPageState.totalCount}
                hasMoreTasks={taskPageState.nextCursor !== null}
                pendingTaskActionIds={visiblePendingTaskActionIds}
                selectedSubtaskId={selectedSubtaskId}
                onSelectTask={handleSelectTask}
                onSelectSubtask={handleSelectSubtask}
                onRequestCreateTask={handleRequestTaskCreate}
                onToggleTaskCompletion={handleToggleTaskCompletion}
                onToggleTaskFavorite={handleToggleTaskFavorite}
                onStartTimer={handleStartTimer}
                onPauseTimer={handlePauseTimer}
                onResumeTimer={handleResumeTimer}
                onStopTimer={handleStopTimer}
                onLoadMoreTasks={handleLoadMoreTasks}
              />
              {selectedTask ? (
                <MemoizedTaskDetailPane
                  task={selectedTask}
                  selectedSubtaskId={selectedSubtaskId}
                  activeTimer={activeTimer}
                  activePomodoro={activePomodoro}
                  taskLists={taskLists}
                  tags={tags}
                  isMutating={isDetailMutating}
                  onClose={closeDetailPane}
                  onUpdateTask={handleUpdateTask}
                  onUpdateSubtask={handleUpdateSubtask}
                  onCreateSubtask={handleCreateSubtask}
                  onSelectSubtask={handleSelectDetailSubtask}
                  onSelectParentTask={handleSelectParentTask}
                  onStartTimer={handleStartTimer}
                  onPauseTimer={handlePauseTimer}
                  onResumeTimer={handleResumeTimer}
                  onStopTimer={handleStopTimer}
                  onToggleTaskCompletion={handleToggleTaskCompletion}
                  onToggleSubtaskCompletion={handleToggleSubtaskCompletion}
                  onDeleteTask={handleDeleteTask}
                  onDeleteSubtask={handleDeleteSubtask}
                  onCreateTag={handleCreateTag}
                  onRenameTag={handleRenameTag}
                  onDeleteTag={handleDeleteTag}
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
              <MemoizedKanbanBoard
                columns={boardColumns}
                tasks={visibleTasks}
                taskRows={visibleTaskRows}
                selectedTaskId={selectedTaskId}
                isLoading={isLoading || isTaskPageLoading}
                isMutating={isBoardMutating}
                isCreatingTaskPending={isCreatingTaskPending}
                isLoadingMore={isLoadingMoreTasks}
                totalTaskCount={taskPageState.totalCount}
                hasMoreTasks={taskPageState.nextCursor !== null}
                pendingTaskActionIds={visiblePendingTaskActionIds}
                onSelectTask={handleSelectTask}
                onRequestCreateTask={handleRequestTaskCreate}
                onToggleTaskCompletion={handleToggleTaskCompletion}
                onCreateColumn={handleCreateBoardColumn}
                onRenameColumn={handleRenameBoardColumn}
                onReorderColumns={handleReorderBoardColumns}
                onDeleteColumn={handleDeleteBoardColumn}
                onDeleteCompletedTasks={handleDeleteCompletedBoardColumnTasks}
                onMoveTask={handleMoveTaskToBoardColumn}
                onLoadMoreTasks={handleLoadMoreTasks}
              />
              {selectedTask ? (
                <MemoizedTaskDetailPane
                  task={selectedTask}
                  selectedSubtaskId={selectedSubtaskId}
                  activeTimer={activeTimer}
                  activePomodoro={activePomodoro}
                  taskLists={taskLists}
                  tags={tags}
                  isMutating={isDetailMutating}
                  onClose={closeDetailPane}
                  onUpdateTask={handleUpdateTask}
                  onUpdateSubtask={handleUpdateSubtask}
                  onCreateSubtask={handleCreateSubtask}
                  onSelectSubtask={handleSelectDetailSubtask}
                  onSelectParentTask={handleSelectParentTask}
                  onStartTimer={handleStartTimer}
                  onPauseTimer={handlePauseTimer}
                  onResumeTimer={handleResumeTimer}
                  onStopTimer={handleStopTimer}
                  onToggleTaskCompletion={handleToggleTaskCompletion}
                  onToggleSubtaskCompletion={handleToggleSubtaskCompletion}
                  onDeleteTask={handleDeleteTask}
                  onDeleteSubtask={handleDeleteSubtask}
                  onCreateTag={handleCreateTag}
                  onRenameTag={handleRenameTag}
                  onDeleteTag={handleDeleteTag}
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
              <MemoizedWeekCalendar
                viewMode={calendarViewMode}
                anchorDate={calendarAnchorDate}
                items={items}
                isLoading={isLoading || isCalendarLoading}
                isTaskCreateOpen={taskCreatePreset !== null}
                isReschedulingItem={isCalendarMutating || isDetailMutating}
                selectedTarget={selectedCalendarTarget}
                onChangeViewMode={handleChangeCalendarViewMode}
                onPreviousRange={handlePreviousCalendarRange}
                onNextRange={handleNextCalendarRange}
                onToday={handleTodayCalendarRange}
                onSelectItem={handleSelectCalendarItem}
                onRequestCreateTask={handleRequestScheduledTaskCreate}
                onRescheduleItem={handleRescheduleCalendarItem}
                onResizeItem={handleResizeCalendarItem}
                onMoveScheduledItem={handleMoveScheduledCalendarItem}
              />
              {selectedTask ? (
                <MemoizedTaskDetailPane
                  task={selectedTask}
                  selectedSubtaskId={selectedSubtaskId}
                  activeTimer={activeTimer}
                  activePomodoro={activePomodoro}
                  taskLists={taskLists}
                  tags={tags}
                  isMutating={isDetailMutating}
                  onClose={closeDetailPane}
                  onUpdateTask={handleUpdateTask}
                  onUpdateSubtask={handleUpdateSubtask}
                  onCreateSubtask={handleCreateSubtask}
                  onSelectSubtask={handleSelectDetailSubtask}
                  onSelectParentTask={handleSelectParentTask}
                  onStartTimer={handleStartTimer}
                  onPauseTimer={handlePauseTimer}
                  onResumeTimer={handleResumeTimer}
                  onStopTimer={handleStopTimer}
                  onToggleTaskCompletion={handleToggleTaskCompletion}
                  onToggleSubtaskCompletion={handleToggleSubtaskCompletion}
                  onDeleteTask={handleDeleteTask}
                  onDeleteSubtask={handleDeleteSubtask}
                  onCreateTag={handleCreateTag}
                  onRenameTag={handleRenameTag}
                  onDeleteTag={handleDeleteTag}
                  onAttachTagToTask={handleAttachTagToTask}
                  onDetachTagFromTask={handleDetachTagFromTask}
                />
              ) : null}
            </div>
          ) : null}

          {activeView.kind === "pomodoro" ? (
            <MemoizedPomodoroPanel
              activePomodoro={activePomodoro}
              activeTimer={activeTimer}
              settings={pomodoroSettings}
              isMutating={isPomodoroMutating}
              onStart={handleStartPomodoro}
              onPause={handlePausePomodoro}
              onResume={handleResumePomodoro}
              onCompleteWork={handleCompletePomodoroWork}
              onCompleteWorkAndStartBreak={handleCompletePomodoroWorkAndStartBreak}
              onSkipBreak={handleSkipPomodoroBreak}
              onCompleteBreak={handleCompletePomodoroBreak}
              onCompleteBreakAndStartNext={handleCompletePomodoroBreakAndStartNext}
              onCancel={handleCancelPomodoro}
            />
          ) : null}

          {activeView.kind === "settings" ? (
            <MemoizedSettingsPanel
              displayMode={displayMode}
              notificationsEnabled={notificationsEnabled}
              taskTimerSettings={taskTimerSettings}
              pomodoroSettings={pomodoroSettings}
              isMutating={isSettingsMutating}
              notificationSummary={notificationSummary}
              onUpdateDisplayMode={handleUpdateNotificationDisplayMode}
              onUpdateNotificationsEnabled={handleUpdateNotificationsEnabled}
              onUpdateTaskTimerSettings={handleUpdateTaskTimerSettings}
              onUpdatePomodoroSettings={handleUpdatePomodoroSettings}
              onRetryNotifications={handleRetryNotifications}
              onCreateJsonExport={handleCreateJsonExport}
              onCreateCsvExport={handleCreateCsvExport}
            />
          ) : null}
        </section>
      </div>
      {taskCreatePreset ? (
        <TaskCreateDialog
          preset={taskCreatePreset}
          taskLists={taskLists}
          isSubmitting={isCreatingTaskPending}
          errorMessage={taskCreateErrorMessage}
          onSubmit={handleSubmitTaskCreate}
          onClose={handleCloseTaskCreate}
        />
      ) : null}
    </main>
  );
}

function getTaskPanelEyebrow(scope: WorkspaceScope) {
  if (scope.kind === "today") {
    return "今日のタスク";
  }
  if (scope.kind === "favorites") {
    return "お気に入り";
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

function workspaceScopeFromPreferences(
  preferences: UiPreferences,
): WorkspaceScope {
  if (preferences.lastView === "today") {
    return { kind: "today" };
  }
  if (preferences.lastView === "favorites") {
    return { kind: "favorites" };
  }
  return {
    kind: "list",
    listId: normalizeTaskListId(preferences.lastTaskListId),
  };
}

function workspaceModeFromView(view: AppView): WorkspaceMode {
  if (view.kind === "board" || view.kind === "calendar") {
    return view.kind;
  }
  return "list";
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

function taskPageScopeFromWorkspaceScope(scope: WorkspaceScope): TaskPageScope {
  if (scope.kind === "list") {
    return { type: "list", listId: scope.listId };
  }
  if (scope.kind === "today") {
    return { type: "today" };
  }
  return { type: "favorites" };
}

function serializeTaskPageScope(scope: TaskPageScope, todayDate: string) {
  return JSON.stringify({ scope, todayDate });
}

async function loadTaskPageWindow(
  scope: TaskPageScope,
  todayDate: string,
  targetTaskCount: number,
): Promise<TaskPage> {
  let tasks: TaskWithSubtasks[] = [];
  let rows: TaskRow[] = [];
  let cursor: TaskPageCursor | null = null;
  let lastPage: TaskPage | null = null;

  do {
    const remainingCount = Math.max(1, targetTaskCount - rows.length);
    const page = await tauriTaskTimerGateway.listTaskPage({
      scope,
      todayDate,
      cursor,
      limit: Math.min(TASK_PAGE_SIZE, remainingCount),
    });
    tasks = appendUniqueById(tasks, page.tasks);
    rows = appendUniqueById(rows, page.rows);
    cursor = page.nextCursor;
    lastPage = page;
  } while (rows.length < targetTaskCount && cursor);

  if (!lastPage) {
    throw new Error("タスクページを取得できませんでした。");
  }
  return {
    ...lastPage,
    tasks,
    rows,
    nextCursor: cursor,
  };
}

function createTaskPageViewState(
  scopeKey: string,
  page: TaskPage,
): TaskPageViewState {
  return {
    scopeKey,
    loadedCount: page.rows.length,
    totalCount: page.totalCount,
    nextCursor: page.nextCursor,
  };
}

function appendUniqueById<T extends { id: string }>(
  current: T[],
  incoming: T[],
) {
  const existingIds = new Set(current.map((item) => item.id));
  return [
    ...current,
    ...incoming.filter((item) => !existingIds.has(item.id)),
  ];
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

function isSameWorkspaceScope(
  current: WorkspaceScope,
  next: WorkspaceScope,
) {
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

function combineNotificationSummaries(
  first: NotificationDispatchSummary,
  second: NotificationDispatchSummary,
): NotificationDispatchSummary {
  return {
    attempted: first.attempted + second.attempted,
    succeeded: first.succeeded + second.succeeded,
    failed: first.failed + second.failed,
    lastError: second.lastError ?? first.lastError,
  };
}

function getPomodoroSyncTimeoutMs(activePomodoro: ActivePomodoro | null) {
  if (!activePomodoro || activePomodoro.status !== "running") {
    return null;
  }

  const startedAt = new Date(activePomodoro.phaseStartedAt).getTime();
  if (Number.isNaN(startedAt)) {
    return 60_000;
  }

  const phaseDurationMs = activePomodoro.phaseDurationSeconds * 1_000;
  const pausedMs = activePomodoro.pausedTotalSeconds * 1_000;
  const phaseEndAt = startedAt + phaseDurationMs + pausedMs;
  const remainingMs = phaseEndAt - Date.now();
  if (remainingMs <= 0) {
    return 500;
  }
  return Math.min(remainingMs + 500, 60_000);
}

function getTaskCountdownSyncTimeoutMs(activeTimer: ActiveTimer | null) {
  if (!activeTimer?.targetSeconds || activeTimer.pausedAt) {
    return null;
  }

  const startedAt = new Date(activeTimer.startedAt).getTime();
  if (Number.isNaN(startedAt)) {
    return 60_000;
  }

  const endAt =
    startedAt +
    (activeTimer.targetSeconds + activeTimer.accumulatedPausedSeconds) * 1_000;
  const remainingMs = endAt - Date.now();
  if (remainingMs <= 0) {
    return 500;
  }
  return remainingMs + 500;
}

function getNotificationScheduleTimeoutMs(notifyAt: string) {
  const scheduledAt = new Date(notifyAt).getTime();
  if (Number.isNaN(scheduledAt)) {
    return NOTIFICATION_SCHEDULER_MAX_TIMEOUT_MS;
  }

  const remainingMs = scheduledAt - Date.now();
  if (remainingMs <= 0) {
    return NOTIFICATION_SCHEDULER_DUE_DELAY_MS;
  }
  return Math.min(
    remainingMs + NOTIFICATION_SCHEDULER_DUE_DELAY_MS,
    NOTIFICATION_SCHEDULER_MAX_TIMEOUT_MS,
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
