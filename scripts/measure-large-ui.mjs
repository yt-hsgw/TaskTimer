import { fileURLToPath } from "node:url";
import {
  createCdpClient,
  getFreePort,
  makeTempDir,
  resolveChromePath,
  rmWithRetry,
  startChrome,
  startVite,
  waitForChromeWebSocket,
  waitForExpression,
  waitForHttp,
  waitForProcessExit,
} from "./lib/headless-chrome.mjs";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const options = parseOptions(process.argv.slice(2));
const profile = options.profile === "standard"
  ? {
      totalTaskCount: 401,
      taskCount: 401,
      subtasksPerTask: 4,
      listCount: 12,
    }
  : {
      totalTaskCount: 50,
      taskCount: 50,
      subtasksPerTask: 4,
      listCount: 4,
    };
const thresholds = {
  initial_task_list: 5000,
  task_list_load_more: 1500,
  today: 1000,
  favorites: 1000,
  kanban: 1500,
  kanban_drag: 1500,
  same_navigation: 250,
  favorite_refresh: 1000,
  completion_refresh: 1200,
  task_detail_save: 1200,
  calendar_week: 1500,
  calendar_overflow_popup: 500,
  calendar_overlap_layout: 500,
  calendar_drag_create: 1500,
  calendar_move: 1500,
  calendar_resize: 1500,
  calendar_day: 1500,
  calendar_day_move: 1500,
  calendar_month: 2000,
  calendar_month_drag_create: 1500,
  calendar_month_date_change: 1500,
  calendar_multiday_header: 2000,
  task_detail: 1000,
};
const taskPageSize = 200;
const initialTaskPageCount = Math.min(profile.taskCount, taskPageSize);
const UNRELATED_WORKSPACE_COMMANDS = [
  "list_calendar_items",
  "list_tags",
  "get_active_timer",
  "get_active_pomodoro",
  "sync_expired_pomodoro",
  "get_pomodoro_settings",
  "get_notification_display_mode",
  "get_notifications_enabled",
];
const chromePath = await resolveChromePath();
const vitePort = await getFreePort();
const debugPort = await getFreePort();
const userDataDir = await makeTempDir("tasktimer-ui-perf-chrome-");
let viteProcess;
let chromeProcess;
let client;

try {
  viteProcess = startVite(repoRoot, vitePort);
  await waitForHttp(`http://127.0.0.1:${vitePort}/`);
  chromeProcess = startChrome(chromePath, debugPort, userDataDir);
  const browserWsUrl = await waitForChromeWebSocket(debugPort);
  client = await createCdpClient(browserWsUrl);
  const { targetId } = await client.send("Target.createTarget", {
    url: "about:blank",
  });
  const { sessionId } = await client.send("Target.attachToTarget", {
    targetId,
    flatten: true,
  });

  await client.send("Page.enable", {}, sessionId);
  await client.send("Runtime.enable", {}, sessionId);
  await client.send(
    "Emulation.setDeviceMetricsOverride",
    {
      width: 1440,
      height: 900,
      deviceScaleFactor: 1,
      mobile: false,
    },
    sessionId,
  );
  await client.send(
    "Page.addScriptToEvaluateOnNewDocument",
    { source: buildTauriInvokeMockSource(profile) },
    sessionId,
  );

  const measurements = [];
  const initialStartedAt = performance.now();
  await client.send(
    "Page.navigate",
    { url: `http://127.0.0.1:${vitePort}/` },
    sessionId,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `document.querySelectorAll(".task-row").length === ${initialTaskPageCount} &&
      document.querySelector('#task-panel-title')?.textContent === "タスク" &&
      !document.querySelector(".app-alert")`,
  );
  measurements.push(
    createMeasurement(
      "initial_task_list",
      performance.now() - initialStartedAt,
      thresholds.initial_task_list,
    ),
  );

  if (profile.taskCount > taskPageSize) {
    measurements.push(
      await measureView({
        client,
        sessionId,
        name: "task_list_load_more",
        thresholdMs: thresholds.task_list_load_more,
        action: `(() => {
          document.querySelector(".subtask-expand-button")?.click();
          document.querySelector(".task-row-content")?.click();
          const board = document.querySelector(".task-board");
          if (board) {
            board.scrollTop = 120;
            window.__taskTimerPageScrollTop = board.scrollTop;
          }
          document.querySelector(".task-load-more")?.click();
        })()`,
        ready: `document.querySelectorAll(".task-row").length === ${Math.min(
          profile.taskCount,
          taskPageSize * 2,
        )} &&
          document.querySelector(".task-row.is-selected") &&
          document.querySelector('.subtask-expand-button[aria-expanded="true"]') &&
          document.querySelector(".task-row-subtasks") &&
          document.querySelector(".task-detail-pane") &&
          document.querySelector(".task-board")?.scrollTop === window.__taskTimerPageScrollTop &&
          document.querySelector(".task-load-more")?.textContent?.includes("${Math.min(
            profile.taskCount,
            taskPageSize * 2,
          )} / ${profile.taskCount}")`,
      }),
    );
  }

  measurements.push(
    await measureView({
      client,
      sessionId,
      name: "today",
      thresholdMs: thresholds.today,
      action: clickNavigation("今日"),
      ready: `document.querySelector('button.nav-item[aria-label="今日"][aria-current="page"]') &&
        document.querySelector('#task-panel-title')?.textContent === "今日" &&
        document.querySelectorAll(".task-row").length === ${Math.min(
          taskPageSize,
          Math.ceil(profile.taskCount / 2),
        )}`,
    }),
  );
  measurements.push(
    await measureView({
      client,
      sessionId,
      name: "favorites",
      thresholdMs: thresholds.favorites,
      action: clickNavigation("お気に入り"),
      ready: `document.querySelector('button.nav-item[aria-label="お気に入り"][aria-current="page"]') &&
        document.querySelector('#task-panel-title')?.textContent === "お気に入り" &&
        document.querySelectorAll(".task-row").length === ${Math.min(
          taskPageSize,
          Math.ceil(profile.taskCount / 3),
        )}`,
    }),
  );
  measurements.push(
    await measureView({
      client,
      sessionId,
      name: "kanban",
      thresholdMs: thresholds.kanban,
      action: clickNavigation("かんばん"),
      ready: `document.querySelector('button.nav-item[aria-label="かんばん"][aria-current="page"]') &&
        document.querySelectorAll(".kanban-card").length === ${initialTaskPageCount}`,
    }),
  );
  const kanbanDragStartedAt = performance.now();
  const kanbanDragResult = await verifyKanbanCardDrag(client, sessionId);
  assertCommandScope("かんばんD&D", kanbanDragResult.commands, {
    required: [
      "move_task_to_board_column",
      "list_task_page",
      "list_board_columns",
    ],
    forbidden: UNRELATED_WORKSPACE_COMMANDS,
  });
  assertComponentsDidNotRender("かんばんD&D", kanbanDragResult.renderCounts, [
    "LeftNavigation",
    "WeekCalendar",
    "SettingsPanel",
  ]);
  measurements.push(
    createMeasurement(
      "kanban_drag",
      performance.now() - kanbanDragStartedAt,
      thresholds.kanban_drag,
    ),
  );
  measurements.push(
    await measureView({
      client,
      sessionId,
      name: "calendar_week",
      thresholdMs: thresholds.calendar_week,
      action: clickNavigation("カレンダー"),
      ready: `document.querySelector('button.nav-item[aria-label="カレンダー"][aria-current="page"]') &&
        document.querySelector(".calendar-time-grid:not(.is-day-mode)") &&
        (() => {
          const visibleCount = document.querySelectorAll(".calendar-item").length;
          const hiddenCount = [...document.querySelectorAll(".calendar-more")]
            .reduce((total, element) => total + Number(element.textContent?.match(/\\d+/)?.[0] ?? 0), 0);
          return visibleCount + hiddenCount === ${profile.taskCount};
        })()`,
    }),
  );
  const calendarWeekGridResult = await verifyCalendarTimeGrid(
    client,
    sessionId,
    7,
  );
  if (calendarWeekGridResult.commands.length > 0) {
    throw new Error(
      `週カレンダーのグリッド確認中にcommandが呼ばれました: ${JSON.stringify(
        calendarWeekGridResult.commands,
      )}`,
    );
  }
  const calendarOverflowStartedAt = performance.now();
  const calendarOverflowResult = await verifyCalendarOverflowPopup(
    client,
    sessionId,
    ".calendar-all-day-cell .calendar-more",
    false,
  );
  if (calendarOverflowResult.commands.length > 0) {
    throw new Error(
      `カレンダー予定一覧の操作中にcommandが呼ばれました: ${JSON.stringify(
        calendarOverflowResult.commands,
      )}`,
    );
  }
  measurements.push(
    createMeasurement(
      "calendar_overflow_popup",
      performance.now() - calendarOverflowStartedAt,
      thresholds.calendar_overflow_popup,
    ),
  );
  const calendarOverlapLayoutStartedAt = performance.now();
  const calendarOverlapLayoutResult = await verifyCalendarOverlapLayout(
    client,
    sessionId,
  );
  if (calendarOverlapLayoutResult.commands.length > 0) {
    throw new Error(
      `カレンダー重複レイアウト確認中にcommandが呼ばれました: ${JSON.stringify(
        calendarOverlapLayoutResult.commands,
      )}`,
    );
  }
  measurements.push(
    createMeasurement(
      "calendar_overlap_layout",
      performance.now() - calendarOverlapLayoutStartedAt,
      thresholds.calendar_overlap_layout,
    ),
  );
  const calendarDragCreateStartedAt = performance.now();
  const calendarDragCreateResult = await verifyCalendarDragCreate(
    client,
    sessionId,
  );
  if (calendarDragCreateResult.commands.length > 0) {
    throw new Error(
      `カレンダードラッグ作成の保存前にcommandが呼ばれました: ${JSON.stringify(
        calendarDragCreateResult.commands,
      )}`,
    );
  }
  measurements.push(
    createMeasurement(
      "calendar_drag_create",
      performance.now() - calendarDragCreateStartedAt,
      thresholds.calendar_drag_create,
    ),
  );
  const calendarMoveStartedAt = performance.now();
  const calendarMoveResult = await verifyCalendarScheduledMove(client, sessionId);
  assertCommandScope("カレンダー予定移動", calendarMoveResult.commands, {
    required: [
      "move_scheduled_work_item",
      "list_task_page",
      "list_calendar_items",
    ],
    forbidden: [
      "update_task",
      "update_subtask",
      "resize_scheduled_work_item",
      "sync_notifications",
      "process_notification_os_registrations",
      "list_board_columns",
    ],
  });
  measurements.push(
    createMeasurement(
      "calendar_move",
      performance.now() - calendarMoveStartedAt,
      thresholds.calendar_move,
    ),
  );
  const calendarResizeStartedAt = performance.now();
  const calendarResizeResult = await verifyCalendarTimedResizePreview(
    client,
    sessionId,
  );
  assertCommandScope("カレンダー時刻調整", calendarResizeResult.commands, {
    required: [
      "resize_scheduled_work_item",
      "list_task_page",
      "list_calendar_items",
    ],
    forbidden: [
      "update_task",
      "update_subtask",
      "move_scheduled_work_item",
      "sync_notifications",
      "process_notification_os_registrations",
      "list_board_columns",
    ],
  });
  measurements.push(
    createMeasurement(
      "calendar_resize",
      performance.now() - calendarResizeStartedAt,
      thresholds.calendar_resize,
    ),
  );
  measurements.push(
    await measureView({
      client,
      sessionId,
      name: "calendar_day",
      thresholdMs: thresholds.calendar_day,
      action: clickCalendarMode("日"),
      ready: `document.querySelector(".calendar-time-grid.is-day-mode") &&
        [...document.querySelectorAll(".calendar-view-switch button")]
          .some((button) => button.textContent === "日" && button.classList.contains("is-active")) &&
        (() => {
          const visibleCount = document.querySelectorAll(".calendar-item").length;
          const hiddenCount = [...document.querySelectorAll(".calendar-more")]
            .reduce((total, element) => total + Number(element.textContent?.match(/\\d+/)?.[0] ?? 0), 0);
          return visibleCount + hiddenCount === ${profile.taskCount};
        })()`,
    }),
  );
  const calendarDayGridResult = await verifyCalendarTimeGrid(
    client,
    sessionId,
    1,
  );
  if (calendarDayGridResult.commands.length > 0) {
    throw new Error(
      `日カレンダーのグリッド確認中にcommandが呼ばれました: ${JSON.stringify(
        calendarDayGridResult.commands,
      )}`,
    );
  }
  const calendarDayOverlapLayoutResult = await verifyCalendarOverlapLayout(
    client,
    sessionId,
  );
  if (calendarDayOverlapLayoutResult.commands.length > 0) {
    throw new Error(
      `日カレンダー重複レイアウト確認中にcommandが呼ばれました: ${JSON.stringify(
        calendarDayOverlapLayoutResult.commands,
      )}`,
    );
  }
  const calendarDayMoveStartedAt = performance.now();
  const calendarDayMoveResult = await verifyCalendarDayScheduledMove(
    client,
    sessionId,
  );
  assertCommandScope("日カレンダー予定移動", calendarDayMoveResult.commands, {
    required: [
      "move_scheduled_work_item",
      "list_task_page",
      "list_calendar_items",
    ],
    forbidden: [
      "update_task",
      "update_subtask",
      "resize_scheduled_work_item",
      "sync_notifications",
      "process_notification_os_registrations",
      "list_board_columns",
    ],
  });
  measurements.push(
    createMeasurement(
      "calendar_day_move",
      performance.now() - calendarDayMoveStartedAt,
      thresholds.calendar_day_move,
    ),
  );
  measurements.push(
    await measureView({
      client,
      sessionId,
      name: "calendar_month",
      thresholdMs: thresholds.calendar_month,
      action: clickCalendarMode("月"),
      ready: `document.querySelector(".calendar-month-grid") &&
        [...document.querySelectorAll(".calendar-view-switch button")]
          .some((button) => button.textContent === "月" && button.classList.contains("is-active")) &&
        (() => {
          const visibleCount = document.querySelectorAll(".calendar-item").length;
          const hiddenCount = [...document.querySelectorAll(".calendar-more")]
            .reduce((total, element) => total + Number(element.textContent?.match(/\\d+/)?.[0] ?? 0), 0);
          return visibleCount + hiddenCount === ${profile.taskCount};
        })()`,
    }),
  );
  const calendarMonthOverflowResult = await verifyCalendarOverflowPopup(
    client,
    sessionId,
    ".calendar-month-day.is-today .calendar-more",
    true,
  );
  if (calendarMonthOverflowResult.commands.length > 0) {
    throw new Error(
      `月カレンダー予定一覧の操作中にcommandが呼ばれました: ${JSON.stringify(
        calendarMonthOverflowResult.commands,
      )}`,
    );
  }
  const calendarMonthDragCreateStartedAt = performance.now();
  const calendarMonthDragCreateResult = await verifyCalendarMonthDragCreate(
    client,
    sessionId,
  );
  if (calendarMonthDragCreateResult.commands.length > 0) {
    throw new Error(
      `月カレンダードラッグ作成の保存前にcommandが呼ばれました: ${JSON.stringify(
        calendarMonthDragCreateResult.commands,
      )}`,
    );
  }
  measurements.push(
    createMeasurement(
      "calendar_month_drag_create",
      performance.now() - calendarMonthDragCreateStartedAt,
      thresholds.calendar_month_drag_create,
    ),
  );
  const calendarMonthDateChangeStartedAt = performance.now();
  const calendarMonthDateChangeResult =
    await verifyCalendarMonthDateChange(client, sessionId);
  assertCommandScope(
    "月カレンダー日付変更",
    calendarMonthDateChangeResult.commands,
    {
      required: [
        "resize_scheduled_work_item",
        "move_scheduled_work_item",
        "list_task_page",
        "list_calendar_items",
      ],
      forbidden: [
        "update_task",
        "update_subtask",
        "sync_notifications",
        "process_notification_os_registrations",
        "list_board_columns",
      ],
    },
  );
  measurements.push(
    createMeasurement(
      "calendar_month_date_change",
      performance.now() - calendarMonthDateChangeStartedAt,
      thresholds.calendar_month_date_change,
    ),
  );
  const calendarMultiDayHeaderStartedAt = performance.now();
  const calendarMultiDayHeaderResult = await verifyCalendarMultiDayHeader(
    client,
    sessionId,
    calendarMonthDateChangeResult.destinationDate,
    calendarMonthDateChangeResult.destinationEndDate,
  );
  assertCommandScope(
    "日・週カレンダー複数日上部表示",
    calendarMultiDayHeaderResult.commands,
    {
      required: ["list_calendar_items"],
      forbidden: [
        "update_task",
        "update_subtask",
        "resize_scheduled_work_item",
        "move_scheduled_work_item",
        "sync_notifications",
        "process_notification_os_registrations",
      ],
    },
  );
  measurements.push(
    createMeasurement(
      "calendar_multiday_header",
      performance.now() - calendarMultiDayHeaderStartedAt,
      thresholds.calendar_multiday_header,
    ),
  );
  await evaluate(client, sessionId, clickNavigation("タスク"));
  await waitForPaintedExpression(
    client,
    sessionId,
    `document.querySelector('#task-panel-title')?.textContent === "タスク" &&
      document.querySelectorAll(".task-row").length === ${initialTaskPageCount}`,
  );

  await resetInvokeLog(client, sessionId);
  await resetRenderCounts(client, sessionId);
  const sameNavigationStartedAt = performance.now();
  await evaluate(client, sessionId, clickNavigation("タスク"));
  await waitForPaintedExpression(
    client,
    sessionId,
    `document.querySelector('button.nav-item[aria-label="タスク"][aria-current="page"]') &&
      document.querySelector('#task-panel-title')?.textContent === "タスク"`,
  );
  const sameNavigationCommands = await takeInvokeLog(client, sessionId);
  const sameNavigationRenderCounts = await takeRenderCounts(client, sessionId);
  assertCommandScope("同一ナビ再選択", sameNavigationCommands, {
    forbidden: ["list_task_page", "list_calendar_items", "list_board_columns"],
  });
  assertComponentsDidNotRender(
    "同一ナビ再選択",
    sameNavigationRenderCounts,
    ["App", "LeftNavigation", "TaskPanel"],
  );
  measurements.push(
    createMeasurement(
      "same_navigation",
      performance.now() - sameNavigationStartedAt,
      thresholds.same_navigation,
    ),
  );

  const favoriteButtonState = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const button = document.querySelector(".favorite-button");
      return button ? {
        label: button.getAttribute("aria-label"),
        pressed: button.getAttribute("aria-pressed") === "true"
      } : null;
    })()`,
  );
  if (!favoriteButtonState?.label) {
    throw new Error("お気に入り更新の検証対象がありません");
  }
  await resetInvokeLog(client, sessionId);
  const favoriteStartedAt = performance.now();
  await evaluate(client, sessionId, `document.querySelector(".favorite-button")?.click()`);
  await waitForPaintedExpression(
    client,
    sessionId,
    `(() => {
      const button = document.querySelector(".favorite-button");
      return button &&
        button.getAttribute("aria-pressed") === ${JSON.stringify(
          String(!favoriteButtonState.pressed),
        )} &&
        !button.disabled;
    })()`,
  );
  const favoriteCommands = await takeInvokeLog(client, sessionId);
  assertCommandScope("お気に入り更新", favoriteCommands, {
    required: ["toggle_task_favorite", "list_task_page"],
    forbidden: [
      ...UNRELATED_WORKSPACE_COMMANDS,
      "list_board_columns",
      "list_task_lists",
      "sync_notifications",
    ],
  });
  measurements.push(
    createMeasurement(
      "favorite_refresh",
      performance.now() - favoriteStartedAt,
      thresholds.favorite_refresh,
    ),
  );

  const completionLabel = await evaluateValue(
    client,
    sessionId,
    `document.querySelector(".task-check-button")?.getAttribute("aria-label")`,
  );
  if (!completionLabel) {
    throw new Error("完了更新の検証対象がありません");
  }
  const completedLabel = completionLabel.replace(/を完了$/, "を未完了に戻す");
  await resetInvokeLog(client, sessionId);
  const completionStartedAt = performance.now();
  await evaluate(
    client,
    sessionId,
    `(() => {
      window.confirm = () => true;
      document.querySelector(".task-check-button")?.click();
    })()`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `(() => {
      const button = [...document.querySelectorAll(".task-check-button")]
        .find((candidate) => candidate.getAttribute("aria-label") === ${JSON.stringify(
          completedLabel,
        )});
      return Boolean(button && !button.disabled);
    })()`,
  );
  const completionCommands = await takeInvokeLog(client, sessionId);
  assertCommandScope("タスク完了", completionCommands, {
    required: [
      "complete_task",
      "list_task_page",
      "list_task_lists",
      "sync_notifications",
    ],
    forbidden: [...UNRELATED_WORKSPACE_COMMANDS, "list_board_columns"],
  });
  measurements.push(
    createMeasurement(
      "completion_refresh",
      performance.now() - completionStartedAt,
      thresholds.completion_refresh,
    ),
  );

  measurements.push(
    await measureView({
      client,
      sessionId,
      name: "task_detail",
      thresholdMs: thresholds.task_detail,
      action: `document.querySelector(".task-row-content")?.click()`,
      ready: `document.querySelector(".task-detail-pane") &&
        document.querySelector(".detail-subtask-list") &&
        !document.querySelector(".app-alert")`,
    }),
  );

  const detailSaveMeasurement = await verifyTaskDetailSave(client, sessionId);
  assertCommandScope("タスク詳細保存", detailSaveMeasurement.commands, {
    required: [
      "update_task",
      "list_task_page",
      "list_task_lists",
      "sync_notifications",
    ],
    forbidden: [...UNRELATED_WORKSPACE_COMMANDS, "list_board_columns"],
  });
  measurements.push(
    createMeasurement(
      "task_detail_save",
      detailSaveMeasurement.durationMs,
      thresholds.task_detail_save,
    ),
  );

  printResults(options.profile, profile, measurements);
  const warningCount = measurements.filter((measurement) => !measurement.ok).length;
  if (warningCount > 0 && options.failOnWarning) {
    process.exitCode = 2;
  }
} finally {
  client?.close();
  if (chromeProcess) {
    chromeProcess.kill("SIGTERM");
    await waitForProcessExit(chromeProcess, 3000);
  }
  if (viteProcess) {
    viteProcess.kill("SIGTERM");
    await waitForProcessExit(viteProcess, 3000);
  }
  await rmWithRetry(userDataDir);
}

function parseOptions(args) {
  let profile = "smoke";
  let failOnWarning = false;
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (argument === "--profile") {
      profile = args[index + 1] ?? "";
      index += 1;
      continue;
    }
    if (argument === "--fail-on-warning") {
      failOnWarning = true;
      continue;
    }
    throw new Error(`未対応の引数です: ${argument}`);
  }
  if (!new Set(["smoke", "standard"]).has(profile)) {
    throw new Error("--profile は smoke または standard を指定してください");
  }
  return { profile, failOnWarning };
}

async function measureView({
  client,
  sessionId,
  name,
  thresholdMs,
  action,
  ready,
}) {
  const startedAt = performance.now();
  await evaluate(client, sessionId, action);
  try {
    await waitForPaintedExpression(client, sessionId, ready);
  } catch (error) {
    const diagnostics = await inspectPage(client, sessionId);
    throw new Error(`${name}の完了待機に失敗しました: ${diagnostics}`, {
      cause: error,
    });
  }
  return createMeasurement(name, performance.now() - startedAt, thresholdMs);
}

function createMeasurement(name, durationMs, thresholdMs) {
  const roundedDurationMs = Math.round(durationMs);
  return {
    name,
    durationMs: roundedDurationMs,
    thresholdMs,
    ok: roundedDurationMs <= thresholdMs,
  };
}

async function evaluate(client, sessionId, expression) {
  await client.send(
    "Runtime.evaluate",
    { expression, awaitPromise: true },
    sessionId,
  );
}

async function evaluateValue(client, sessionId, expression) {
  const result = await client.send(
    "Runtime.evaluate",
    { expression, awaitPromise: true, returnByValue: true },
    sessionId,
  );
  return result.result?.value;
}

async function resetInvokeLog(client, sessionId) {
  await evaluate(
    client,
    sessionId,
    `window.__taskTimerInvokeLog = []`,
  );
}

async function takeInvokeLog(client, sessionId) {
  return (
    (await evaluateValue(
      client,
      sessionId,
      `(() => {
        const commands = [...(window.__taskTimerInvokeLog ?? [])];
        window.__taskTimerInvokeLog = [];
        return commands;
      })()`,
    )) ?? []
  );
}

async function resetRenderCounts(client, sessionId) {
  await evaluate(
    client,
    sessionId,
    `window.__TASKTIMER_RENDER_COUNTS__ = {}`,
  );
}

async function takeRenderCounts(client, sessionId) {
  return (
    (await evaluateValue(
      client,
      sessionId,
      `(() => {
        const counts = { ...(window.__TASKTIMER_RENDER_COUNTS__ ?? {}) };
        window.__TASKTIMER_RENDER_COUNTS__ = {};
        return counts;
      })()`,
    )) ?? {}
  );
}

function assertCommandScope(label, commands, { required = [], forbidden = [] }) {
  const missing = required.filter((command) => !commands.includes(command));
  const unexpected = forbidden.filter((command) => commands.includes(command));
  if (missing.length === 0 && unexpected.length === 0) {
    return;
  }
  throw new Error(
    `${label}のRead Model更新範囲が不正です: ${JSON.stringify({
      missing,
      unexpected,
      commands,
    })}`,
  );
}

function assertComponentsDidNotRender(label, renderCounts, componentNames) {
  const unexpected = componentNames.filter(
    (componentName) => (renderCounts[componentName] ?? 0) > 0,
  );
  if (unexpected.length === 0) {
    return;
  }
  throw new Error(
    `${label}で無関係なコンポーネントが再描画されました: ${JSON.stringify({
      unexpected,
      renderCounts,
    })}`,
  );
}

async function verifyTaskDetailSave(client, sessionId) {
  await resetInvokeLog(client, sessionId);
  const startedAt = performance.now();
  await evaluate(
    client,
    sessionId,
    `(() => {
      const section = document.querySelector('.detail-section[aria-label="タスクを編集"]');
      const toggle = section?.querySelector(".detail-section-toggle");
      if (toggle?.getAttribute("aria-expanded") !== "true") {
        toggle.click();
      }
    })()`,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(document.querySelector(
      '.detail-section[aria-label="タスクを編集"] .detail-form input[required]'
    ))`,
    5000,
  );
  const nextTitle = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const section = document.querySelector('.detail-section[aria-label="タスクを編集"]');
      const input = section.querySelector('.detail-form input[required]');
      if (!input) {
        return null;
      }
      const nextValue = input.value + " 更新";
      const setter = Object.getOwnPropertyDescriptor(
        HTMLInputElement.prototype,
        "value",
      )?.set;
      setter?.call(input, nextValue);
      input.dispatchEvent(new Event("input", { bubbles: true }));
      section.querySelector('.detail-form button[type="submit"]')?.click();
      return nextValue;
    })()`,
  );
  if (!nextTitle) {
    throw new Error("タスク詳細保存の検証対象がありません");
  }
  await waitForPaintedExpression(
    client,
    sessionId,
    `document.querySelector("#task-detail-title")?.textContent === ${JSON.stringify(
      nextTitle,
    )} &&
      !document.querySelector('.detail-section[aria-label="タスクを編集"] .detail-form button[type="submit"]')?.disabled &&
      !document.querySelector(".app-alert")`,
  );
  return {
    commands: await takeInvokeLog(client, sessionId),
    durationMs: performance.now() - startedAt,
  };
}

async function verifyKanbanCardDrag(client, sessionId) {
  await evaluate(
    client,
    sessionId,
    `document.querySelector(".kanban-card-main")?.click()`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `Boolean(document.querySelector(".task-detail-pane"))`,
  );
  await evaluate(
    client,
    sessionId,
    `document.querySelector(".kanban-card-main")?.click()`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `!document.querySelector(".task-detail-pane")`,
  );
  await resetInvokeLog(client, sessionId);
  await resetRenderCounts(client, sessionId);

  const geometry = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const columns = [...document.querySelectorAll(".kanban-column")];
      const source = columns[0]?.querySelector(".kanban-card");
      const destination = columns[1]?.querySelector(".kanban-column-scroll");
      if (!source || !destination || !source.dataset.taskId) {
        return null;
      }
      const sourceRect = source.getBoundingClientRect();
      const destinationRect = destination.getBoundingClientRect();
      return {
        taskId: source.dataset.taskId,
        sourceX: sourceRect.left + sourceRect.width * 0.75,
        sourceY: sourceRect.top + Math.min(sourceRect.height / 2, 36),
        destinationX: destinationRect.left + destinationRect.width / 2,
        destinationY: destinationRect.top + Math.min(destinationRect.height / 2, 72)
      };
    })()`,
  );
  if (!geometry) {
    throw new Error("かんばんD&D検証に必要なカードまたは移動先がありません");
  }

  await client.send(
    "Input.dispatchMouseEvent",
    { type: "mouseMoved", x: geometry.sourceX, y: geometry.sourceY },
    sessionId,
  );
  await client.send(
    "Input.dispatchMouseEvent",
    {
      type: "mousePressed",
      x: geometry.sourceX,
      y: geometry.sourceY,
      button: "left",
      buttons: 1,
      clickCount: 1,
    },
    sessionId,
  );
  await client.send(
    "Input.dispatchMouseEvent",
    {
      type: "mouseMoved",
      x: geometry.sourceX + 12,
      y: geometry.sourceY + 8,
      button: "left",
      buttons: 1,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `(() => {
      const overlay = document.querySelector(".kanban-card-overlay");
      return Boolean(
        overlay &&
        overlay.dataset.taskId === ${JSON.stringify(geometry.taskId)} &&
        overlay.parentElement?.parentElement === document.body &&
        Number(getComputedStyle(overlay.parentElement).zIndex) >= 1000
      );
    })()`,
    5000,
  );
  await client.send(
    "Input.dispatchMouseEvent",
    {
      type: "mouseMoved",
      x: geometry.destinationX,
      y: geometry.destinationY,
      button: "left",
      buttons: 1,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `document.querySelectorAll(".kanban-column")[1]
      ?.querySelector(".kanban-column-scroll")
      ?.classList.contains("is-over") === true`,
    5000,
  );
  await client.send(
    "Input.dispatchMouseEvent",
    {
      type: "mouseReleased",
      x: geometry.destinationX,
      y: geometry.destinationY,
      button: "left",
      buttons: 0,
      clickCount: 1,
    },
    sessionId,
  );
  const placementAfterDrop = await evaluateValue(
    client,
    sessionId,
    `(async () => {
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const columns = [...document.querySelectorAll(".kanban-column")];
      const selector = '[data-task-id="' + CSS.escape(${JSON.stringify(
        geometry.taskId,
      )}) + '"]';
      return {
        remainsInSource: Boolean(columns[0]?.querySelector(selector)),
        appearsInDestination: Boolean(columns[1]?.querySelector(selector))
      };
    })()`,
  );
  if (
    placementAfterDrop?.remainsInSource ||
    !placementAfterDrop?.appearsInDestination
  ) {
    throw new Error(
      `ドロップ直後の楽観表示が不正です: ${JSON.stringify(placementAfterDrop)}`,
    );
  }
  await waitForPaintedExpression(
    client,
    sessionId,
    `(() => {
      const movedCard = document.querySelectorAll(".kanban-column")[1]
        ?.querySelector('[data-task-id=${JSON.stringify(geometry.taskId)}]');
      return Boolean(
        movedCard &&
        movedCard.getAttribute("tabindex") === "0" &&
        !document.querySelector(".kanban-card-overlay") &&
        !document.querySelector(".task-detail-pane") &&
        !document.querySelector(".app-alert")
      );
    })()`,
  );
  return {
    commands: await takeInvokeLog(client, sessionId),
    renderCounts: await takeRenderCounts(client, sessionId),
  };
}

async function verifyCalendarOverflowPopup(
  client,
  sessionId,
  triggerSelector,
  expectParent,
) {
  await resetInvokeLog(client, sessionId);
  const setup = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const trigger = document.querySelector(${JSON.stringify(triggerSelector)});
      if (!(trigger instanceof HTMLButtonElement)) {
        return null;
      }
      trigger.dataset.calendarOverflowTestTrigger = "true";
      const label = trigger.getAttribute("aria-label") ?? "";
      const expectedCount = Number(label.match(/予定(\\d+)件/)?.[1] ?? 0);
      trigger.click();
      return { expectedCount };
    })()`,
  );
  if (!setup?.expectedCount) {
    throw new Error(`予定一覧の起点が見つかりません: ${triggerSelector}`);
  }
  await waitForPaintedExpression(
    client,
    sessionId,
    `Boolean(document.querySelector('#calendar-overflow-popup'))`,
  );
  const opened = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const panel = document.querySelector('.calendar-panel');
      const popup = document.querySelector('#calendar-overflow-popup');
      const list = popup?.querySelector('.calendar-overflow-list');
      const panelBounds = panel?.getBoundingClientRect();
      const popupBounds = popup?.getBoundingClientRect();
      const rows = [...(popup?.querySelectorAll('.calendar-overflow-list li') ?? [])];
      return {
        rowCount: rows.length,
        focusedFirst: Boolean(
          document.activeElement?.hasAttribute('data-calendar-overflow-first')
        ),
        swatchCount: popup?.querySelectorAll('.calendar-overflow-color').length ?? 0,
        markerCount: popup?.querySelectorAll('.calendar-overflow-content small').length ?? 0,
        parentCount: [...(popup?.querySelectorAll('.calendar-overflow-content small') ?? [])]
          .filter((element) => element.textContent?.startsWith('親タスク:')).length,
        insidePanel: Boolean(
          panelBounds && popupBounds &&
          popupBounds.left >= panelBounds.left + 11 &&
          popupBounds.right <= panelBounds.right - 11 &&
          popupBounds.top >= panelBounds.top + 11 &&
          popupBounds.bottom <= panelBounds.bottom - 11
        ),
        popupHeight: popupBounds?.height ?? null,
        listScrollContained: Boolean(
          list && list.clientHeight <= 420 &&
          popup && popup.scrollHeight <= popup.clientHeight + 1
        ),
        createFormOpen: Boolean(document.querySelector('.calendar-create-form'))
      };
    })()`,
  );
  if (
    opened?.rowCount !== setup.expectedCount ||
    !opened.focusedFirst ||
    opened.swatchCount !== setup.expectedCount ||
    opened.markerCount < setup.expectedCount ||
    (expectParent && opened.parentCount < 1) ||
    !opened.insidePanel ||
    opened.popupHeight > 421 ||
    !opened.listScrollContained ||
    opened.createFormOpen
  ) {
    throw new Error(
      `カレンダー予定一覧の表示が不正です: ${JSON.stringify({ setup, opened })}`,
    );
  }

  await client.send(
    "Input.dispatchKeyEvent",
    { type: "keyDown", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 },
    sessionId,
  );
  await client.send(
    "Input.dispatchKeyEvent",
    { type: "keyUp", key: "Escape", code: "Escape", windowsVirtualKeyCode: 27 },
    sessionId,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `!document.querySelector('#calendar-overflow-popup') &&
      document.activeElement?.dataset.calendarOverflowTestTrigger === 'true'`,
  );

  await evaluate(
    client,
    sessionId,
    `document.querySelector('[data-calendar-overflow-test-trigger="true"]')?.click()`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `Boolean(document.querySelector('#calendar-overflow-popup'))`,
  );
  await evaluate(
    client,
    sessionId,
    `document.querySelector('.calendar-toolbar')?.dispatchEvent(
      new PointerEvent('pointerdown', { bubbles: true, pointerId: 1 })
    )`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `!document.querySelector('#calendar-overflow-popup') &&
      document.activeElement?.dataset.calendarOverflowTestTrigger === 'true'`,
  );

  await evaluate(
    client,
    sessionId,
    `document.querySelector('[data-calendar-overflow-test-trigger="true"]')?.click()`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `Boolean(document.querySelector('#calendar-overflow-popup'))`,
  );
  await evaluate(
    client,
    sessionId,
    `document.querySelector('.calendar-overflow-list button')?.click()`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `!document.querySelector('#calendar-overflow-popup') &&
      Boolean(document.querySelector('.task-detail-pane'))`,
  );
  await evaluate(
    client,
    sessionId,
    `document.querySelector('button[aria-label="詳細を閉じる"]')?.click()`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `!document.querySelector('.task-detail-pane')`,
  );
  return { commands: await takeInvokeLog(client, sessionId) };
}

async function verifyCalendarTimeGrid(client, sessionId, expectedDayCount) {
  await resetInvokeLog(client, sessionId);
  const result = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const labels = [...document.querySelectorAll(
        '.calendar-time-label:not(.is-time-zone)'
      )];
      const cells = [...document.querySelectorAll('.calendar-time-cell')];
      const overlays = [...document.querySelectorAll('.calendar-timed-day-overlay')];
      const hours = ${JSON.stringify(Array.from({ length: 15 }, (_, index) => 8 + index))};
      const tolerance = 1;
      const rows = hours.map((hour, hourIndex) => {
        const expectedGridRow = String(hourIndex + 3);
        const label = labels[hourIndex];
        const rowCells = cells.filter(
          (cell) => Number(cell.dataset.calendarHour) === hour
        );
        const labelStyle = label ? getComputedStyle(label) : null;
        const labelBounds = label?.getBoundingClientRect();
        return {
          hour,
          labelGridRow: labelStyle?.gridRowStart ?? null,
          cellCount: rowCells.length,
          cellsAligned: rowCells.every((cell, dayIndex) => {
            const style = getComputedStyle(cell);
            const bounds = cell.getBoundingClientRect();
            return style.gridRowStart === expectedGridRow &&
              style.gridColumnStart === String(dayIndex + 2) &&
              style.borderBottomStyle === 'solid' &&
              style.borderBottomWidth === '1px' &&
              style.borderLeftStyle === 'solid' &&
              style.borderLeftWidth === '1px' &&
              Boolean(labelBounds && Math.abs(bounds.top - labelBounds.top) <= tolerance);
          })
        };
      });
      const firstCell = cells.find(
        (cell) => Number(cell.dataset.calendarHour) === hours[0]
      );
      const lastCell = [...cells].reverse().find(
        (cell) => Number(cell.dataset.calendarHour) === hours.at(-1)
      );
      const firstBounds = firstCell?.getBoundingClientRect();
      const lastBounds = lastCell?.getBoundingClientRect();
      return {
        labelCount: labels.length,
        cellCount: cells.length,
        overlayCount: overlays.length,
        rows,
        overlaysCoverGrid: overlays.every((overlay) => {
          const bounds = overlay.getBoundingClientRect();
          return Boolean(
            firstBounds && lastBounds &&
            Math.abs(bounds.top - firstBounds.top) <= tolerance &&
            Math.abs(bounds.bottom - lastBounds.bottom) <= tolerance
          );
        })
      };
    })()`,
  );
  const expectedHourCount = 15;
  const invalidRow = result?.rows?.find(
    (row, hourIndex) =>
      row.labelGridRow !== String(hourIndex + 3) ||
      row.cellCount !== expectedDayCount ||
      !row.cellsAligned,
  );
  if (
    result?.labelCount !== expectedHourCount ||
    result?.cellCount !== expectedHourCount * expectedDayCount ||
    result?.overlayCount !== expectedDayCount ||
    invalidRow ||
    !result?.overlaysCoverGrid
  ) {
    throw new Error(
      `カレンダー時間グリッドの配置が不正です: ${JSON.stringify({
        expectedDayCount,
        result,
        invalidRow,
      })}`,
    );
  }
  return { commands: await takeInvokeLog(client, sessionId) };
}

async function verifyCalendarOverlapLayout(client, sessionId) {
  await resetInvokeLog(client, sessionId);
  const result = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const calendarScroller = document.querySelector('.calendar-time-grid');
      const initialScrollTop = calendarScroller?.scrollTop ?? 0;
      const findItem = (title) => [...document.querySelectorAll(
        '.calendar-item.marker-scheduled.is-timed:not(.is-calendar-preview)'
      )].find((item) =>
        item.querySelector('.calendar-item-title')?.textContent === title
      );
      const describe = (title) => {
        const item = findItem(title);
        const button = item?.querySelector('.calendar-item-content');
        const scroller = item?.closest('.calendar-time-grid');
        const initialBounds = item?.getBoundingClientRect();
        const scrollerBounds = scroller?.getBoundingClientRect();
        if (scroller && initialBounds && scrollerBounds) {
          scroller.scrollTop += initialBounds.top - scrollerBounds.top -
            (scroller.clientHeight - initialBounds.height) / 2;
        }
        button?.focus({ preventScroll: true });
        const bounds = item?.getBoundingClientRect();
        const containerBounds = item?.parentElement?.getBoundingClientRect();
        const hit = bounds
          ? document.elementFromPoint(
              bounds.left + bounds.width / 2,
              bounds.top + bounds.height / 2
            )
          : null;
        const lowerHit = bounds
          ? document.elementFromPoint(
              bounds.left + bounds.width / 2,
              bounds.top + bounds.height * 0.75
            )
          : null;
        return {
          exists: Boolean(item),
          laneIndex: Number(item?.dataset.calendarLaneIndex ?? -1),
          laneCount: Number(item?.dataset.calendarLaneCount ?? -1),
          left: bounds?.left ?? null,
          right: bounds?.right ?? null,
          width: bounds?.width ?? null,
          containerWidth: containerBounds?.width ?? null,
          hit: hit?.closest('.calendar-item') === item,
          lowerHit: lowerHit?.closest('.calendar-item') === item,
          hitClass: typeof hit?.className === 'string' ? hit.className : null,
          hitTitle: hit?.closest('.calendar-item')
            ?.querySelector('.calendar-item-title')?.textContent ?? null,
          focusable: document.activeElement === button,
          isSubtask: item?.classList.contains('is-subtask') ?? false
        };
      };
      const layout = {
        exactA: describe('重複予定 A'),
        exactB: describe('重複予定 B'),
        boundary: describe('境界予定 C'),
        chainD: describe('連鎖予定 D'),
        chainE: describe('連鎖予定 E'),
        chainF: describe('連鎖予定 F')
      };
      if (calendarScroller) {
        calendarScroller.scrollTop = initialScrollTop;
      }
      return layout;
    })()`,
  );
  const values = result ? Object.values(result) : [];
  const exactHasGap =
    result?.exactA?.right !== null &&
    result?.exactB?.left !== null &&
    result.exactA.right < result.exactB.left;
  const boundaryUsesFullWidth =
    result?.boundary?.width !== null &&
    result?.boundary?.containerWidth !== null &&
    result.boundary.width >= result.boundary.containerWidth - 1;
  if (
    values.length !== 6 ||
    values.some((value) => !value.exists || !value.hit || !value.focusable) ||
    result?.exactA?.laneIndex !== 0 ||
    result?.exactB?.laneIndex !== 1 ||
    result?.exactA?.laneCount !== 2 ||
    result?.exactB?.laneCount !== 2 ||
    !result?.exactB?.isSubtask ||
    !exactHasGap ||
    result?.boundary?.laneIndex !== 0 ||
    result?.boundary?.laneCount !== 1 ||
    !boundaryUsesFullWidth ||
    result?.chainD?.laneIndex !== 0 ||
    !result?.chainD?.lowerHit ||
    result?.chainE?.laneIndex !== 1 ||
    result?.chainF?.laneIndex !== 1 ||
    result?.chainD?.laneCount !== 2 ||
    result?.chainE?.laneCount !== 2 ||
    result?.chainF?.laneCount !== 2
  ) {
    throw new Error(
      `カレンダー重複レイアウトが不正です: ${JSON.stringify(result)}`,
    );
  }
  return { commands: await takeInvokeLog(client, sessionId) };
}

async function verifyCalendarDragCreate(client, sessionId) {
  await resetInvokeLog(client, sessionId);
  const geometry = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const cells = [...document.querySelectorAll(
        '.calendar-time-cell[data-calendar-create-surface="timed"]'
      )];
      const source = cells.find((cell) => {
        const hour = Number(cell.dataset.calendarHour);
        const bounds = cell.getBoundingClientRect();
        const x = bounds.left + bounds.width * 0.72;
        const y = bounds.top + bounds.height * 0.3;
        const hit = document.elementFromPoint(x, y);
        return hour <= 20 && hit?.closest('.calendar-time-cell') === cell &&
          !hit.closest('.calendar-item');
      });
      if (!source) {
        return null;
      }
      const bounds = source.getBoundingClientRect();
      const hour = Number(source.dataset.calendarHour);
      const formatTime = (minutes) =>
        String(Math.floor(minutes / 60)).padStart(2, '0') + ':' +
        String(minutes % 60).padStart(2, '0');
      return {
        date: source.dataset.calendarDate,
        startX: bounds.left + bounds.width * 0.72,
        startY: bounds.top + bounds.height * 0.3,
        endX: bounds.left + bounds.width * 0.72,
        endY: bounds.top + bounds.height * 1.8,
        expectedStartTime: formatTime(hour * 60 + 15),
        expectedEndTime: formatTime(hour * 60 + 120)
      };
    })()`,
  );
  if (!geometry?.date) {
    throw new Error("カレンダードラッグ作成に使える空き時間帯がありません");
  }

  await dragPointer(
    client,
    sessionId,
    geometry.startX,
    geometry.startY,
    geometry.endX,
    geometry.endY,
    {
      beforeRelease: async () => {
        await waitForPaintedExpression(
          client,
          sessionId,
          `(() => {
            const preview = document.querySelector(
              '.calendar-create-selection.is-timed'
            );
            return Boolean(
              preview?.textContent?.includes(${JSON.stringify(
                geometry.expectedStartTime,
              )}) &&
              preview?.textContent?.includes(${JSON.stringify(
                geometry.expectedEndTime,
              )}) &&
              !document.querySelector('.calendar-create-form') &&
              (window.__taskTimerInvokeLog?.length ?? 0) === 0
            );
          })()`,
        );
      },
    },
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `Boolean(document.querySelector('.calendar-create-form'))`,
  );
  const draft = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const form = document.querySelector('.calendar-create-form');
      const field = (label) => [...(form?.querySelectorAll('label') ?? [])]
        .find((element) => element.querySelector(':scope > span')?.textContent === label)
        ?.querySelector('input');
      return {
        startDate: field('開始日')?.value ?? null,
        startTime: field('開始時刻')?.value ?? null,
        endDate: field('終了日')?.value ?? null,
        endTime: field('終了時刻')?.value ?? null,
        isAllDay: Boolean(form?.querySelector('.calendar-all-day-toggle input')?.checked),
        sourceLabel: form?.querySelector('.calendar-create-form-heading span')?.textContent ?? null
      };
    })()`,
  );
  if (
    draft?.startDate !== geometry.date ||
    draft?.endDate !== geometry.date ||
    draft?.startTime !== geometry.expectedStartTime ||
    draft?.endTime !== geometry.expectedEndTime ||
    draft?.isAllDay ||
    !draft?.sourceLabel?.includes(geometry.expectedStartTime) ||
    !draft.sourceLabel.includes(geometry.expectedEndTime)
  ) {
    throw new Error(
      `カレンダードラッグ作成フォームの初期値が不正です: ${JSON.stringify(
        draft,
      )}`,
    );
  }
  await evaluate(
    client,
    sessionId,
    `[...document.querySelectorAll('.calendar-create-form button')]
      .find((button) => button.textContent?.trim() === 'キャンセル')?.click()`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `!document.querySelector('.calendar-create-form')`,
  );

  await dragPointer(
    client,
    sessionId,
    geometry.startX,
    geometry.startY,
    geometry.startX,
    geometry.startY + 3,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `!document.querySelector('.calendar-create-form') &&
      !document.querySelector('.calendar-create-selection')`,
  );

  const allDayGeometry = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const cells = [...document.querySelectorAll(
        '.calendar-all-day-cell[data-calendar-create-surface="all-day"]'
      )];
      for (let index = 0; index < cells.length - 2; index += 1) {
        const first = cells[index];
        const last = cells[index + 2];
        const firstBounds = first.getBoundingClientRect();
        const lastBounds = last.getBoundingClientRect();
        const firstX = firstBounds.left + 3;
        const firstY = firstBounds.top + firstBounds.height / 2;
        const lastX = lastBounds.left + 3;
        const lastY = lastBounds.top + lastBounds.height / 2;
        const firstHit = document.elementFromPoint(firstX, firstY);
        const lastHit = document.elementFromPoint(lastX, lastY);
        if (
          firstHit?.closest('.calendar-all-day-cell') === first &&
          !firstHit.closest('.calendar-item, .calendar-more') &&
          lastHit?.closest('.calendar-all-day-cell') === last
        ) {
          return {
            startDate: first.dataset.calendarDate,
            endDate: last.dataset.calendarDate,
            startX: lastX,
            startY: lastY,
            endX: firstX,
            endY: firstY
          };
        }
      }
      return null;
    })()`,
  );
  if (!allDayGeometry?.startDate || !allDayGeometry?.endDate) {
    throw new Error("上部予定行のドラッグ作成に使える日付範囲がありません");
  }
  await dragPointer(
    client,
    sessionId,
    allDayGeometry.startX,
    allDayGeometry.startY,
    allDayGeometry.endX,
    allDayGeometry.endY,
    {
      beforeRelease: async () => {
        await waitForPaintedExpression(
          client,
          sessionId,
          `document.querySelectorAll(
            '.calendar-create-selection.is-all-day'
          ).length === 3 &&
            !document.querySelector('.calendar-create-form') &&
            (window.__taskTimerInvokeLog?.length ?? 0) === 0`,
        );
      },
    },
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `Boolean(document.querySelector('.calendar-create-form'))`,
  );
  const allDayDraft = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const form = document.querySelector('.calendar-create-form');
      const field = (label) => [...(form?.querySelectorAll('label') ?? [])]
        .find((element) => element.querySelector(':scope > span')?.textContent === label)
        ?.querySelector('input');
      return {
        startDate: field('開始日')?.value ?? null,
        endDate: field('終了日')?.value ?? null,
        isAllDay: Boolean(form?.querySelector('.calendar-all-day-toggle input')?.checked)
      };
    })()`,
  );
  if (
    allDayDraft?.startDate !== allDayGeometry.startDate ||
    allDayDraft?.endDate !== allDayGeometry.endDate ||
    !allDayDraft?.isAllDay
  ) {
    throw new Error(
      `上部予定行ドラッグ作成フォームの初期値が不正です: ${JSON.stringify(
        allDayDraft,
      )}`,
    );
  }
  await evaluate(
    client,
    sessionId,
    `[...document.querySelectorAll('.calendar-create-form button')]
      .find((button) => button.textContent?.trim() === 'キャンセル')?.click()`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `!document.querySelector('.calendar-create-form')`,
  );
  return { commands: await takeInvokeLog(client, sessionId) };
}

async function verifyCalendarMonthDragCreate(client, sessionId) {
  await resetInvokeLog(client, sessionId);
  const geometry = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const cells = [...document.querySelectorAll(
        '.calendar-month-day[data-calendar-create-surface="month"]'
      )];
      for (let index = 0; index < cells.length - 2; index += 1) {
        const first = cells[index];
        const last = cells[index + 2];
        const firstBounds = first.getBoundingClientRect();
        const lastBounds = last.getBoundingClientRect();
        if (Math.abs(firstBounds.top - lastBounds.top) > 2) {
          continue;
        }
        const firstX = firstBounds.left + 3;
        const firstY = firstBounds.top + firstBounds.height / 2;
        const lastX = lastBounds.left + 3;
        const lastY = lastBounds.top + lastBounds.height / 2;
        const firstHit = document.elementFromPoint(firstX, firstY);
        const lastHit = document.elementFromPoint(lastX, lastY);
        if (
          firstHit?.closest('.calendar-month-day') === first &&
          !firstHit.closest('.calendar-item, .calendar-more') &&
          lastHit?.closest('.calendar-month-day') === last
        ) {
          return {
            startDate: first.dataset.calendarDate,
            endDate: last.dataset.calendarDate,
            startX: lastX,
            startY: lastY,
            endX: firstX,
            endY: firstY
          };
        }
      }
      return null;
    })()`,
  );
  if (!geometry?.startDate || !geometry?.endDate) {
    throw new Error("月カレンダードラッグ作成に使える日付範囲がありません");
  }

  await dragPointer(
    client,
    sessionId,
    geometry.startX,
    geometry.startY,
    geometry.endX,
    geometry.endY,
    {
      beforeRelease: async () => {
        await waitForPaintedExpression(
          client,
          sessionId,
          `document.querySelectorAll(
            '.calendar-create-selection.is-month'
          ).length === 3 &&
            !document.querySelector('.calendar-create-form') &&
            (window.__taskTimerInvokeLog?.length ?? 0) === 0`,
        );
      },
    },
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `Boolean(document.querySelector('.calendar-create-form'))`,
  );
  const draft = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const form = document.querySelector('.calendar-create-form');
      const field = (label) => [...(form?.querySelectorAll('label') ?? [])]
        .find((element) => element.querySelector(':scope > span')?.textContent === label)
        ?.querySelector('input');
      return {
        startDate: field('開始日')?.value ?? null,
        endDate: field('終了日')?.value ?? null,
        isAllDay: Boolean(form?.querySelector('.calendar-all-day-toggle input')?.checked)
      };
    })()`,
  );
  if (
    draft?.startDate !== geometry.startDate ||
    draft?.endDate !== geometry.endDate ||
    !draft?.isAllDay
  ) {
    throw new Error(
      `月カレンダードラッグ作成フォームの初期値が不正です: ${JSON.stringify(
        draft,
      )}`,
    );
  }
  await evaluate(
    client,
    sessionId,
    `[...document.querySelectorAll('.calendar-create-form button')]
      .find((button) => button.textContent?.trim() === 'キャンセル')?.click()`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `!document.querySelector('.calendar-create-form')`,
  );
  return { commands: await takeInvokeLog(client, sessionId) };
}

async function verifyCalendarScheduledMove(client, sessionId) {
  await resetInvokeLog(client, sessionId);
  const result = await evaluateValue(
    client,
    sessionId,
    `(async () => {
      const source = document.querySelector(
        '.calendar-item.marker-scheduled.is-timed .calendar-item-content'
      );
      const sourceDate = source?.closest('[data-calendar-date]')?.dataset.calendarDate;
      const destination = [...document.querySelectorAll('.calendar-time-cell')]
        .find((cell) =>
          cell.dataset.calendarDate === sourceDate &&
          cell.getAttribute('aria-label')?.includes('11:00')
        );
      if (!source || !destination) {
        return null;
      }
      const sourceTitle = source.querySelector('.calendar-item-title')?.textContent ?? null;
      const transfer = new DataTransfer();
      const destinationRect = destination.getBoundingClientRect();
      source.dispatchEvent(new DragEvent('dragstart', {
        bubbles: true,
        cancelable: true,
        dataTransfer: transfer
      }));
      await new Promise((resolve) => requestAnimationFrame(resolve));
      destination.dispatchEvent(new DragEvent('dragover', {
        bubbles: true,
        cancelable: true,
        clientY: destinationRect.top + destinationRect.height * 0.55,
        dataTransfer: transfer
      }));
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const dayOverlay = document.querySelector(
        '.calendar-timed-day-overlay[data-calendar-date="' + sourceDate + '"]'
      );
      const movePreview = dayOverlay?.querySelector(
        '.calendar-item.marker-scheduled.is-timed.is-move-preview'
      );
      const original = document.querySelector(
        '.calendar-item.marker-scheduled.is-timed:not(.is-calendar-preview)'
      );
      const previewState = {
        originalMarker: original?.querySelector('small:last-child')?.textContent ?? null,
        previewMarker: movePreview?.querySelector('small:last-child')?.textContent ?? null,
        previewLabel: movePreview?.querySelector('.calendar-preview-label')?.textContent ?? null
      };
      destination.dispatchEvent(new DragEvent('drop', {
        bubbles: true,
        cancelable: true,
        clientY: destinationRect.top + destinationRect.height * 0.55,
        dataTransfer: transfer
      }));
      source.dispatchEvent(new DragEvent('dragend', {
        bubbles: true,
        dataTransfer: transfer
      }));
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const moved = [...(dayOverlay?.querySelectorAll(
        '.calendar-item.marker-scheduled.is-timed:not(.is-calendar-preview)'
      ) ?? [])].find((item) =>
        item.querySelector('.calendar-item-title')?.textContent === sourceTitle
      );
      return {
        movedImmediately: Boolean(moved),
        marker: moved?.querySelector('small:last-child')?.textContent ?? null,
        targetDate: destination.dataset.calendarDate ?? null,
        previewState
      };
    })()`,
  );
  if (
    !result?.movedImmediately ||
    !result.marker?.includes("11:30") ||
    !result.previewState?.originalMarker?.includes("09:00") ||
    !result.previewState?.previewMarker?.includes("11:30") ||
    !result.previewState?.previewMarker?.includes("12:30") ||
    result.previewState?.previewLabel !== "移動後"
  ) {
    throw new Error(
      `カレンダードロップ直後の仮位置が不正です: ${JSON.stringify(result)}`,
    );
  }

  await waitForPaintedExpression(
    client,
    sessionId,
    `(() => {
      const dayOverlay = document.querySelector(
        '.calendar-timed-day-overlay[data-calendar-date=${JSON.stringify(
          result.targetDate,
        )}]'
      );
      const moved = dayOverlay?.querySelector(
        '.calendar-item.marker-scheduled.is-timed:not(.is-calendar-preview)'
      );
      return Boolean(
        moved?.querySelector('small:last-child')?.textContent?.includes('11:30') &&
        window.__taskTimerInvokeLog?.includes('list_task_page') &&
        window.__taskTimerInvokeLog?.includes('list_calendar_items') &&
        !document.querySelector('.app-alert')
      );
    })()`,
  );
  const dragCommands = await takeInvokeLog(client, sessionId);
  const keyboardResult = await evaluateValue(
    client,
    sessionId,
    `(async () => {
      const item = document.querySelector(
        '.calendar-item.marker-scheduled.is-timed:not(.is-calendar-preview) .calendar-item-content'
      );
      if (!item) {
        return null;
      }
      item.focus();
      item.dispatchEvent(new KeyboardEvent('keydown', {
        key: 'ArrowDown',
        bubbles: true,
        cancelable: true
      }));
      await new Promise((resolve) => requestAnimationFrame(resolve));
      return document.querySelector(
        '.calendar-item.marker-scheduled.is-timed:not(.is-calendar-preview) small:last-child'
      )?.textContent ?? null;
    })()`,
  );
  if (!keyboardResult?.includes("11:45")) {
    throw new Error(`カレンダーのキーボード仮移動が不正です: ${keyboardResult}`);
  }
  await waitForPaintedExpression(
    client,
    sessionId,
    `(() => {
      const moved = document.querySelector(
        '.calendar-item.marker-scheduled.is-timed:not(.is-calendar-preview)'
      );
      return Boolean(
        moved?.querySelector('small:last-child')?.textContent?.includes('11:45') &&
        window.__taskTimerInvokeLog?.includes('list_task_page') &&
        window.__taskTimerInvokeLog?.includes('list_calendar_items') &&
        !document.querySelector('.app-alert')
      );
    })()`,
  );
  return {
    commands: [...dragCommands, ...(await takeInvokeLog(client, sessionId))],
  };
}

async function verifyCalendarTimedResizePreview(client, sessionId) {
  await resetInvokeLog(client, sessionId);
  await evaluate(
    client,
    sessionId,
    `document.querySelector('button[aria-label="詳細を閉じる"]')?.click()`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `!document.querySelector('.task-detail-pane')`,
  );
  const geometry = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const item = document.querySelector(
        '.calendar-item.marker-scheduled.is-timed:not(.is-resize-preview)'
      );
      const handle = item?.querySelector(
        '.calendar-resize-handle.is-end.is-vertical'
      );
      if (!item || !handle) {
        return null;
      }
      const bounds = handle.getBoundingClientRect();
      const x = bounds.left + bounds.width / 2;
      const y = bounds.top + bounds.height / 2;
      const hit = document.elementFromPoint(x, y);
      return {
        x,
        y,
        handleBounds: {
          left: bounds.left,
          top: bounds.top,
          width: bounds.width,
          height: bounds.height
        },
        hitTag: hit?.tagName ?? null,
        hitClass: typeof hit?.className === 'string'
          ? hit.className
          : hit?.className?.baseVal ?? null,
        hitsHandle: hit?.closest('.calendar-resize-handle') === handle,
        originalMarker: item.querySelector('small:last-child')?.textContent ?? null
      };
    })()`,
  );
  if (
    !geometry ||
    !geometry.hitsHandle ||
    !geometry.originalMarker?.includes("12:45")
  ) {
    throw new Error(
      `時刻調整の検証対象が不正です: ${JSON.stringify(geometry)}`,
    );
  }

  await dragPointer(client, sessionId, geometry.x, geometry.y, geometry.x, geometry.y + 27, {
    beforeRelease: async () => {
      try {
        await waitForPaintedExpression(
          client,
          sessionId,
          `(() => {
            const original = document.querySelector(
              '.calendar-item.marker-scheduled.is-timed:not(.is-resize-preview)'
            );
            const preview = document.querySelector(
              '.calendar-item.marker-scheduled.is-timed.is-resize-preview'
            );
            return Boolean(
              original?.querySelector('small:last-child')?.textContent?.includes('12:45') &&
              original?.dataset.calendarLaneCount === '2' &&
              preview?.querySelector('.calendar-preview-label')?.textContent === '変更後' &&
              preview?.querySelector('small:last-child')?.textContent?.includes('13:15') &&
              preview?.dataset.calendarLaneCount === '2'
            );
          })()`,
        );
      } catch (error) {
        const diagnostics = await evaluateValue(
          client,
          sessionId,
          `(() => ({
            hitAtStart: document.elementFromPoint(${geometry.x}, ${geometry.y})?.className ?? null,
            hitAtEnd: document.elementFromPoint(${geometry.x}, ${geometry.y + 27})?.className ?? null,
            activeHandles: document.querySelectorAll('.calendar-resize-handle.is-active').length,
            previewCount: document.querySelectorAll('.calendar-item.is-resize-preview').length,
            markers: [...document.querySelectorAll('.calendar-item.marker-scheduled small:last-child')]
              .map((node) => node.textContent),
            alert: document.querySelector('.app-alert')?.textContent ?? null
          }))()`,
        );
        throw new Error(
          `時刻調整プレビューの表示待機に失敗しました: ${JSON.stringify(diagnostics)}`,
          { cause: error },
        );
      }
    },
  });

  try {
    await waitForPaintedExpression(
      client,
      sessionId,
      `(() => {
        const item = document.querySelector(
          '.calendar-item.marker-scheduled.is-timed:not(.is-resize-preview)'
        );
        return Boolean(
          item?.querySelector('small:last-child')?.textContent?.includes('13:15') &&
          !document.querySelector('.calendar-item.is-resize-preview') &&
          !document.querySelector('.task-detail-pane') &&
          window.__taskTimerInvokeLog?.includes('list_task_page') &&
          window.__taskTimerInvokeLog?.includes('list_calendar_items') &&
          !document.querySelector('.app-alert')
        );
      })()`,
    );
  } catch (error) {
    const diagnostics = await evaluateValue(
      client,
      sessionId,
      `(() => ({
        marker: document.querySelector(
          '.calendar-item.marker-scheduled.is-timed:not(.is-resize-preview) small:last-child'
        )?.textContent ?? null,
        previewCount: document.querySelectorAll('.calendar-item.is-resize-preview').length,
        detailOpen: Boolean(document.querySelector('.task-detail-pane')),
        commands: window.__taskTimerInvokeLog ?? [],
        alert: document.querySelector('.app-alert')?.textContent ?? null
      }))()`,
    );
    throw new Error(
      `時刻調整後の状態が不正です: ${JSON.stringify(diagnostics)}`,
      { cause: error },
    );
  }
  return { commands: await takeInvokeLog(client, sessionId) };
}

async function verifyCalendarDayScheduledMove(client, sessionId) {
  await resetInvokeLog(client, sessionId);
  const result = await evaluateValue(
    client,
    sessionId,
    `(async () => {
      const source = document.querySelector(
        '.calendar-time-grid.is-day-mode ' +
        '.calendar-item.marker-scheduled.is-timed:not(.is-calendar-preview) ' +
        '.calendar-item-content'
      );
      const sourceDate = source?.closest('[data-calendar-date]')?.dataset.calendarDate;
      const destination = [...document.querySelectorAll(
        '.calendar-time-grid.is-day-mode .calendar-time-cell'
      )].find((cell) =>
        cell.dataset.calendarDate === sourceDate &&
        cell.getAttribute('aria-label')?.includes('14:00')
      );
      if (!source || !destination) {
        return null;
      }
      const sourceTitle = source.querySelector('.calendar-item-title')?.textContent ?? null;
      const transfer = new DataTransfer();
      const bounds = destination.getBoundingClientRect();
      source.dispatchEvent(new DragEvent('dragstart', {
        bubbles: true,
        cancelable: true,
        dataTransfer: transfer
      }));
      await new Promise((resolve) => requestAnimationFrame(resolve));
      destination.dispatchEvent(new DragEvent('dragover', {
        bubbles: true,
        cancelable: true,
        clientY: bounds.top + bounds.height * 0.3,
        dataTransfer: transfer
      }));
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const dayOverlay = document.querySelector(
        '.calendar-timed-day-overlay[data-calendar-date="' + sourceDate + '"]'
      );
      const original = document.querySelector(
        '.calendar-time-grid.is-day-mode ' +
        '.calendar-item.marker-scheduled.is-timed:not(.is-calendar-preview)'
      );
      const preview = dayOverlay?.querySelector(
        '.calendar-item.marker-scheduled.is-timed.is-move-preview'
      );
      const previewState = {
        originalMarker: original?.querySelector('small:last-child')?.textContent ?? null,
        previewMarker: preview?.querySelector('small:last-child')?.textContent ?? null,
        previewLabel: preview?.querySelector('.calendar-preview-label')?.textContent ?? null
      };
      destination.dispatchEvent(new DragEvent('drop', {
        bubbles: true,
        cancelable: true,
        clientY: bounds.top + bounds.height * 0.3,
        dataTransfer: transfer
      }));
      source.dispatchEvent(new DragEvent('dragend', {
        bubbles: true,
        dataTransfer: transfer
      }));
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const moved = [...(dayOverlay?.querySelectorAll(
        '.calendar-item.marker-scheduled.is-timed:not(.is-calendar-preview)'
      ) ?? [])].find((item) =>
        item.querySelector('.calendar-item-title')?.textContent === sourceTitle
      );
      return {
        targetDate: destination.dataset.calendarDate ?? null,
        marker: moved?.querySelector('small:last-child')?.textContent ?? null,
        previewState
      };
    })()`,
  );
  if (
    !result?.previewState?.originalMarker?.includes("11:45") ||
    !result.previewState.originalMarker.includes("13:15") ||
    result.previewState.previewLabel !== "移動後" ||
    !result.previewState.previewMarker?.includes("14:15") ||
    !result.previewState.previewMarker.includes("15:45") ||
    !result.marker?.includes("14:15") ||
    !result.marker.includes("15:45")
  ) {
    throw new Error(`日表示の期間維持D&Dが不正です: ${JSON.stringify(result)}`);
  }
  await waitForPaintedExpression(
    client,
    sessionId,
    `(() => {
      const moved = [...document.querySelectorAll(
        '.calendar-time-grid.is-day-mode ' +
        '.calendar-item.marker-scheduled.is-timed:not(.is-calendar-preview)'
      )].find((item) =>
        item.querySelector('small:last-child')?.textContent?.includes('14:15') &&
        item.querySelector('small:last-child')?.textContent?.includes('15:45')
      );
      return Boolean(
        moved?.querySelector('small:last-child')?.textContent?.includes('14:15') &&
        moved?.querySelector('small:last-child')?.textContent?.includes('15:45') &&
        window.__taskTimerInvokeLog?.includes('list_task_page') &&
        window.__taskTimerInvokeLog?.includes('list_calendar_items') &&
        !document.querySelector('.app-alert')
      );
    })()`,
  );
  return { commands: await takeInvokeLog(client, sessionId) };
}

async function verifyCalendarMonthDateChange(client, sessionId) {
  await resetInvokeLog(client, sessionId);
  const resizeGeometry = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const item = document.querySelector(
        '.calendar-month-day .calendar-item.marker-scheduled:not(.is-calendar-preview)'
      );
      const sourceCell = item?.closest('.calendar-month-day');
      const sourceDate = sourceCell?.dataset.calendarDate;
      const handle = item?.querySelector(
        '.calendar-resize-handle.is-start.is-horizontal'
      );
      if (!item || !sourceDate || !handle) {
        return null;
      }
      const [year, month, day] = sourceDate.split('-').map(Number);
      const target = new Date(year, month - 1, day);
      target.setDate(target.getDate() - 1);
      const targetDate = [
        target.getFullYear(),
        String(target.getMonth() + 1).padStart(2, '0'),
        String(target.getDate()).padStart(2, '0')
      ].join('-');
      const targetCell = document.querySelector(
        '.calendar-month-day[data-calendar-date="' + targetDate + '"]'
      );
      if (!targetCell) {
        return null;
      }
      const handleBounds = handle.getBoundingClientRect();
      const targetBounds = targetCell.getBoundingClientRect();
      return {
        sourceDate,
        targetDate,
        startX: handleBounds.left + handleBounds.width / 2,
        startY: handleBounds.top + handleBounds.height / 2,
        endX: targetBounds.left + targetBounds.width / 2,
        endY: targetBounds.top + Math.min(64, targetBounds.height / 2)
      };
    })()`,
  );
  if (!resizeGeometry) {
    throw new Error("月表示の日付調整対象がありません");
  }

  await dragPointer(
    client,
    sessionId,
    resizeGeometry.startX,
    resizeGeometry.startY,
    resizeGeometry.endX,
    resizeGeometry.endY,
    {
      beforeRelease: async () => {
        try {
          await waitForPaintedExpression(
            client,
            sessionId,
            `(() => {
            const source = document.querySelector(
              '.calendar-month-day[data-calendar-date=${JSON.stringify(
                resizeGeometry.sourceDate,
              )}] .calendar-item.marker-scheduled:not(.is-calendar-preview)'
            );
            const previewStart = document.querySelector(
              '.calendar-month-day[data-calendar-date=${JSON.stringify(
                resizeGeometry.targetDate,
              )}] .calendar-item.marker-scheduled.is-resize-preview'
            );
            const previewEnd = document.querySelector(
              '.calendar-month-day[data-calendar-date=${JSON.stringify(
                resizeGeometry.sourceDate,
              )}] .calendar-item.marker-scheduled.is-resize-preview'
            );
            const startBounds = previewStart?.getBoundingClientRect();
            const endBounds = previewEnd?.getBoundingClientRect();
            return Boolean(
              source &&
              previewStart?.querySelector('.calendar-preview-label')?.textContent === '変更後' &&
              previewStart?.querySelector('.calendar-month-range-time')?.textContent === '14:15' &&
              previewStart?.classList.contains('connects-after') &&
              previewEnd?.classList.contains('connects-before') &&
              startBounds &&
              endBounds &&
              Math.abs(startBounds.right - endBounds.left) <= 2
            );
          })()`,
          );
        } catch (error) {
          const diagnostics = await evaluateValue(
            client,
            sessionId,
            `(() => {
              const start = document.querySelector(
                '.calendar-month-day[data-calendar-date=${JSON.stringify(
                  resizeGeometry.targetDate,
                )}] .calendar-item.marker-scheduled.is-resize-preview'
              );
              const end = document.querySelector(
                '.calendar-month-day[data-calendar-date=${JSON.stringify(
                  resizeGeometry.sourceDate,
                )}] .calendar-item.marker-scheduled.is-resize-preview'
              );
              const startBounds = start?.getBoundingClientRect();
              const endBounds = end?.getBoundingClientRect();
              return {
                previewCount: document.querySelectorAll('.calendar-item.is-resize-preview').length,
                startClass: start?.className ?? null,
                endClass: end?.className ?? null,
                label: start?.querySelector('.calendar-preview-label')?.textContent ?? null,
                time: start?.querySelector('.calendar-month-range-time')?.textContent ?? null,
                startBounds: startBounds ? {
                  left: startBounds.left,
                  right: startBounds.right,
                  top: startBounds.top
                } : null,
                endBounds: endBounds ? {
                  left: endBounds.left,
                  right: endBounds.right,
                  top: endBounds.top
                } : null,
                gap: startBounds && endBounds
                  ? Math.abs(startBounds.right - endBounds.left)
                  : null
              };
            })()`,
          );
          throw new Error(
            `月表示の期間予測が連続していません: ${JSON.stringify(diagnostics)}`,
            { cause: error },
          );
        }
      },
    },
  );

  await waitForPaintedExpression(
    client,
    sessionId,
    `(() => {
      const target = document.querySelector(
        '.calendar-month-day[data-calendar-date=${JSON.stringify(
          resizeGeometry.targetDate,
        )}] .calendar-item.marker-scheduled:not(.is-calendar-preview)'
      );
      const source = document.querySelector(
        '.calendar-month-day[data-calendar-date=${JSON.stringify(
          resizeGeometry.sourceDate,
        )}] .calendar-item.marker-scheduled:not(.is-calendar-preview)'
      );
      const sourceBounds = source?.getBoundingClientRect();
      const targetBounds = target?.getBoundingClientRect();
      return Boolean(
        target?.classList.contains('connects-after') &&
        source?.classList.contains('connects-before') &&
        sourceBounds &&
        targetBounds &&
        Math.abs(targetBounds.right - sourceBounds.left) <= 2 &&
        !document.querySelector('.calendar-item.is-resize-preview') &&
        !document.querySelector('.task-detail-pane') &&
        window.__taskTimerInvokeLog?.includes('resize_scheduled_work_item') &&
        window.__taskTimerInvokeLog?.includes('list_calendar_items') &&
        !document.querySelector('.app-alert')
      );
    })()`,
  );
  const resizeCommands = await takeInvokeLog(client, sessionId);

  const moveResult = await evaluateValue(
    client,
    sessionId,
    `(async () => {
      const sourceDate = ${JSON.stringify(resizeGeometry.sourceDate)};
      const [year, month, day] = sourceDate.split('-').map(Number);
      const destination = new Date(year, month - 1, day);
      destination.setDate(destination.getDate() + 2);
      const destinationDate = [
        destination.getFullYear(),
        String(destination.getMonth() + 1).padStart(2, '0'),
        String(destination.getDate()).padStart(2, '0')
      ].join('-');
      const source = document.querySelector(
        '.calendar-month-day[data-calendar-date="' + sourceDate + '"] ' +
        '.calendar-item.marker-scheduled:not(.is-calendar-preview) .calendar-item-content'
      );
      const destinationCell = document.querySelector(
        '.calendar-month-day[data-calendar-date="' + destinationDate + '"]'
      );
      if (!source || !destinationCell) {
        return null;
      }
      const transfer = new DataTransfer();
      source.dispatchEvent(new DragEvent('dragstart', {
        bubbles: true,
        cancelable: true,
        dataTransfer: transfer
      }));
      await new Promise((resolve) => requestAnimationFrame(resolve));
      destinationCell.dispatchEvent(new DragEvent('dragover', {
        bubbles: true,
        cancelable: true,
        dataTransfer: transfer
      }));
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const destinationEnd = new Date(destination);
      destinationEnd.setDate(destinationEnd.getDate() + 1);
      const destinationEndDate = [
        destinationEnd.getFullYear(),
        String(destinationEnd.getMonth() + 1).padStart(2, '0'),
        String(destinationEnd.getDate()).padStart(2, '0')
      ].join('-');
      const previewStart = destinationCell.querySelector(
        '.calendar-item.marker-scheduled.is-move-preview'
      );
      const previewEnd = document.querySelector(
        '.calendar-month-day[data-calendar-date="' + destinationEndDate + '"] ' +
        '.calendar-item.marker-scheduled.is-move-preview'
      );
      const previewStartBounds = previewStart?.getBoundingClientRect();
      const previewEndBounds = previewEnd?.getBoundingClientRect();
      const previewState = {
        label: previewStart?.querySelector('.calendar-preview-label')?.textContent ?? null,
        hasTwoDayRange: Boolean(
          previewStart?.classList.contains('connects-after') &&
          previewEnd?.classList.contains('connects-before')
        ),
        gap: previewStartBounds && previewEndBounds
          ? Math.abs(previewStartBounds.right - previewEndBounds.left)
          : null
      };
      destinationCell.dispatchEvent(new DragEvent('drop', {
        bubbles: true,
        cancelable: true,
        dataTransfer: transfer
      }));
      source.dispatchEvent(new DragEvent('dragend', {
        bubbles: true,
        dataTransfer: transfer
      }));
      await new Promise((resolve) => requestAnimationFrame(resolve));
      return {
        destinationDate,
        destinationEndDate,
        previewState,
        movedImmediately: Boolean(destinationCell.querySelector(
          '.calendar-item.marker-scheduled:not(.is-calendar-preview)'
        ))
      };
    })()`,
  );
  if (
    !moveResult?.movedImmediately ||
    moveResult.previewState?.label !== "移動後" ||
    !moveResult.previewState?.hasTwoDayRange ||
    moveResult.previewState?.gap === null ||
    moveResult.previewState.gap > 2
  ) {
    throw new Error(
      `月表示のドロップ直後の日付が不正です: ${JSON.stringify(moveResult)}`,
    );
  }
  await waitForPaintedExpression(
    client,
    sessionId,
    `(() => {
      const destination = document.querySelector(
        '.calendar-month-day[data-calendar-date=${JSON.stringify(
          moveResult.destinationDate,
        )}] .calendar-item.marker-scheduled:not(.is-calendar-preview)'
      );
      const destinationEnd = document.querySelector(
        '.calendar-month-day[data-calendar-date=${JSON.stringify(
          moveResult.destinationEndDate,
        )}] .calendar-item.marker-scheduled:not(.is-calendar-preview)'
      );
      return Boolean(
        destination?.classList.contains('connects-after') &&
        destinationEnd?.classList.contains('connects-before') &&
        !document.querySelector('.task-detail-pane') &&
        window.__taskTimerInvokeLog?.includes('move_scheduled_work_item') &&
        window.__taskTimerInvokeLog?.includes('list_task_page') &&
        window.__taskTimerInvokeLog?.includes('list_calendar_items') &&
        !document.querySelector('.app-alert')
      );
    })()`,
  );
  return {
    commands: [...resizeCommands, ...(await takeInvokeLog(client, sessionId))],
    destinationDate: moveResult.destinationDate,
    destinationEndDate: moveResult.destinationEndDate,
  };
}

async function verifyCalendarMultiDayHeader(
  client,
  sessionId,
  destinationDate,
  destinationEndDate,
) {
  await resetInvokeLog(client, sessionId);
  await evaluate(client, sessionId, clickCalendarMode("日"));
  await waitForPaintedExpression(
    client,
    sessionId,
    `document.querySelector('.calendar-time-grid.is-day-mode') &&
      !document.querySelector('.task-detail-pane')`,
  );

  let destinationVisible = false;
  for (let offset = 0; offset < 8; offset += 1) {
    destinationVisible = await evaluateValue(
      client,
      sessionId,
      `Boolean(document.querySelector(
        '.calendar-all-day-cell[data-calendar-date=${JSON.stringify(destinationDate)}]'
      ))`,
    );
    if (destinationVisible) {
      break;
    }
    const previousDate = await evaluateValue(
      client,
      sessionId,
      `document.querySelector('.calendar-all-day-cell')?.dataset.calendarDate ?? null`,
    );
    await evaluate(
      client,
      sessionId,
      `document.querySelector('button[aria-label="次の日"]')?.click()`,
    );
    await waitForPaintedExpression(
      client,
      sessionId,
      `document.querySelector('.calendar-all-day-cell')?.dataset.calendarDate !== ${JSON.stringify(previousDate)}`,
    );
  }
  if (!destinationVisible) {
    throw new Error(`日表示で複数日予定の開始日へ移動できません: ${destinationDate}`);
  }

  const dayLayout = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const cell = document.querySelector(
        '.calendar-all-day-cell[data-calendar-date=${JSON.stringify(destinationDate)}]'
      );
      const item = cell?.querySelector(
        '.calendar-item.marker-scheduled.is-all-day.is-scheduled-range:not(.is-calendar-preview)'
      );
      const timeLabel = document.querySelector('.calendar-time-label.is-time-zone');
      const itemBounds = item?.getBoundingClientRect();
      const labelBounds = timeLabel?.getBoundingClientRect();
      return {
        timeZone: timeLabel?.textContent?.trim() ?? null,
        marker: item?.textContent?.trim() ?? null,
        hasStartHandle: Boolean(item?.querySelector(
          '.calendar-resize-handle.is-start.is-horizontal'
        )),
        timedDuplicateCount: document.querySelectorAll(
          '.calendar-time-cell .calendar-item.marker-scheduled'
        ).length,
        sameRow: Boolean(
          itemBounds &&
          labelBounds &&
          itemBounds.top < labelBounds.bottom &&
          itemBounds.bottom > labelBounds.top
        ),
        detailOpen: Boolean(document.querySelector('.task-detail-pane'))
      };
    })()`,
  );
  if (
    !dayLayout?.timeZone?.startsWith("GMT") ||
    !dayLayout.marker?.includes("14:15") ||
    !dayLayout.hasStartHandle ||
    dayLayout.timedDuplicateCount !== 0 ||
    !dayLayout.sameRow ||
    dayLayout.detailOpen
  ) {
    throw new Error(`日表示の複数日上部予定行が不正です: ${JSON.stringify(dayLayout)}`);
  }

  await evaluate(
    client,
    sessionId,
    `document.querySelector(
      '.calendar-all-day-cell[data-calendar-date=${JSON.stringify(destinationDate)}] ' +
      '.calendar-item.marker-scheduled .calendar-item-content'
    )?.click()`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `Boolean(document.querySelector('.task-detail-pane'))`,
  );
  await evaluate(
    client,
    sessionId,
    `document.querySelector('button[aria-label="詳細を閉じる"]')?.click()`,
  );
  await waitForPaintedExpression(
    client,
    sessionId,
    `!document.querySelector('.task-detail-pane')`,
  );

  await evaluate(client, sessionId, clickCalendarMode("週"));
  await waitForPaintedExpression(
    client,
    sessionId,
    `Boolean(
      document.querySelector(
        '.calendar-all-day-cell[data-calendar-date=${JSON.stringify(destinationDate)}] ' +
        '.calendar-item.marker-scheduled.is-scheduled-range'
      ) &&
      document.querySelector(
        '.calendar-all-day-cell[data-calendar-date=${JSON.stringify(destinationEndDate)}] ' +
        '.calendar-item.marker-scheduled.is-scheduled-range'
      )
    )`,
  );
  const weekLayout = await evaluateValue(
    client,
    sessionId,
    `(() => {
      const start = document.querySelector(
        '.calendar-all-day-cell[data-calendar-date=${JSON.stringify(destinationDate)}] ' +
        '.calendar-item.marker-scheduled.is-scheduled-range:not(.is-calendar-preview)'
      );
      const end = document.querySelector(
        '.calendar-all-day-cell[data-calendar-date=${JSON.stringify(destinationEndDate)}] ' +
        '.calendar-item.marker-scheduled.is-scheduled-range:not(.is-calendar-preview)'
      );
      const startBounds = start?.getBoundingClientRect();
      const endBounds = end?.getBoundingClientRect();
      return {
        startsRange: start?.classList.contains('connects-after') ?? false,
        continuesRange: end?.classList.contains('connects-before') ?? false,
        gap: startBounds && endBounds
          ? Math.abs(startBounds.right - endBounds.left)
          : null,
        rowOffset: startBounds && endBounds
          ? Math.abs(startBounds.top - endBounds.top)
          : null,
        startContent: start?.textContent?.trim() ?? null,
        endContent: end?.textContent?.trim() ?? null,
        hasEndHandle: Boolean(end?.querySelector(
          '.calendar-resize-handle.is-end.is-horizontal'
        )),
        timedDuplicateCount: document.querySelectorAll(
          '.calendar-time-cell .calendar-item.marker-scheduled'
        ).length,
        detailOpen: Boolean(document.querySelector('.task-detail-pane'))
      };
    })()`,
  );
  if (
    !weekLayout?.startsRange ||
    !weekLayout.continuesRange ||
    weekLayout.gap === null ||
    weekLayout.gap > 2 ||
    weekLayout.rowOffset === null ||
    weekLayout.rowOffset > 1 ||
    !weekLayout.startContent?.includes("14:15") ||
    weekLayout.endContent !== "" ||
    !weekLayout.hasEndHandle ||
    weekLayout.timedDuplicateCount !== 0 ||
    weekLayout.detailOpen
  ) {
    throw new Error(`週表示の複数日上部予定行が不正です: ${JSON.stringify(weekLayout)}`);
  }
  return { commands: await takeInvokeLog(client, sessionId) };
}

async function dragPointer(
  client,
  sessionId,
  startX,
  startY,
  endX,
  endY,
  { beforeRelease } = {},
) {
  await client.send(
    "Input.dispatchMouseEvent",
    { type: "mouseMoved", x: startX, y: startY, buttons: 0 },
    sessionId,
  );
  await client.send(
    "Input.dispatchMouseEvent",
    {
      type: "mousePressed",
      x: startX,
      y: startY,
      button: "left",
      buttons: 1,
      clickCount: 1,
    },
    sessionId,
  );
  await client.send(
    "Input.dispatchMouseEvent",
    {
      type: "mouseMoved",
      x: endX,
      y: endY,
      button: "left",
      buttons: 1,
    },
    sessionId,
  );
  await beforeRelease?.();
  await client.send(
    "Input.dispatchMouseEvent",
    {
      type: "mouseReleased",
      x: endX,
      y: endY,
      button: "left",
      buttons: 0,
      clickCount: 1,
    },
    sessionId,
  );
}

async function inspectPage(client, sessionId) {
  const result = await client.send(
    "Runtime.evaluate",
    {
      expression: `JSON.stringify({
        title: document.querySelector("#task-panel-title")?.textContent ?? null,
        taskRows: document.querySelectorAll(".task-row").length,
        kanbanCards: document.querySelectorAll(".kanban-card").length,
        calendarItems: document.querySelectorAll(".calendar-item").length,
        activeNavigation: document.querySelector('button.nav-item[aria-current="page"]')?.getAttribute("aria-label") ?? null,
        alert: document.querySelector(".app-alert")?.textContent ?? null
      })`,
      returnByValue: true,
    },
    sessionId,
  );
  return result.result?.value ?? "ページ状態を取得できません";
}

function waitForPaintedExpression(client, sessionId, ready) {
  return waitForExpression(
    client,
    sessionId,
    `(async () => {
      if (!(${ready})) {
        return false;
      }
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
      return Boolean(${ready});
    })()`,
    15000,
  );
}

function clickNavigation(label) {
  return `document.querySelector('button.nav-item[aria-label=${JSON.stringify(label)}]')?.click()`;
}

function clickCalendarMode(label) {
  return `[...document.querySelectorAll(".calendar-view-switch button")]
    .find((button) => button.textContent === ${JSON.stringify(label)})?.click()`;
}

function printResults(profileName, profile, measurements) {
  console.log("TaskTimer Presentation大量データ計測");
  console.log(`profile: ${profileName}`);
  console.log(`total tasks: ${profile.totalTaskCount}`);
  console.log(`tasks: ${profile.taskCount}`);
  console.log(`subtasks: ${profile.taskCount * profile.subtasksPerTask}`);
  console.log(`lists: ${profile.listCount}`);
  console.log("");
  for (const measurement of measurements) {
    console.log(
      `${measurement.name}: ${measurement.durationMs}ms / ${measurement.thresholdMs}ms ${measurement.ok ? "OK" : "WARN"}`,
    );
  }
  const warningCount = measurements.filter((measurement) => !measurement.ok).length;
  console.log("");
  console.log(`WARN: ${warningCount}`);
}

function buildTauriInvokeMockSource(profile) {
  return `
(() => {
  const profile = ${JSON.stringify(profile)};
  const now = "2026-07-18T09:00:00Z";
  const pad = (value) => String(value).padStart(4, "0");
  const localDate = (date) => [
    date.getFullYear(),
    String(date.getMonth() + 1).padStart(2, "0"),
    String(date.getDate()).padStart(2, "0")
  ].join("-");
  const today = localDate(new Date());
  const clone = (value) => JSON.parse(JSON.stringify(value));
  const addDays = (dateText, days) => {
    const [year, month, day] = dateText.split("-").map(Number);
    const date = new Date(year, month - 1, day);
    date.setDate(date.getDate() + days);
    return localDate(date);
  };
  const daySpan = (startDate, endDate) => {
    const start = new Date(startDate + "T00:00:00");
    const end = new Date(endDate + "T00:00:00");
    return Math.max(1, Math.round((end - start) / 86400000) + 1);
  };
  const addMinutes = (dateText, timeText, minutes) => {
    const [year, month, day] = dateText.split("-").map(Number);
    const [hour, minute] = timeText.split(":").map(Number);
    const date = new Date(year, month - 1, day, hour, minute);
    date.setMinutes(date.getMinutes() + minutes);
    return {
      date: localDate(date),
      time: String(date.getHours()).padStart(2, "0") + ":" +
        String(date.getMinutes()).padStart(2, "0")
    };
  };
  const subtasksFor = (taskId, index) => Array.from(
    { length: profile.subtasksPerTask },
    (_, subtaskIndex) => ({
      id: taskId + "-subtask-" + subtaskIndex,
      taskId,
      title: "サブタスク " + pad(index) + "-" + (subtaskIndex + 1),
      status: subtaskIndex === 0 ? "done" : "todo",
      plannedStartDate: null,
      dueDate: index % 2 === 0 ? today : addDays(today, 1),
      dueTime: null,
      timerTargetSeconds: 1500,
      recurrenceRule: null,
      memo: "",
      sortOrder: subtaskIndex,
      completedAt: subtaskIndex === 0 ? now : null,
      deletedAt: null,
      createdAt: now,
      updatedAt: now
    })
  );
  const tasks = Array.from({ length: profile.taskCount }, (_, index) => {
    const id = "task-perf-" + pad(index);
    return {
      id,
      listId: "default",
      title: "性能検証タスク " + pad(index),
      status: index % 4 === 0 ? "in_progress" : "todo",
      isFavorite: index % 3 === 0,
      plannedStartDate: null,
      dueDate: index % 2 === 0 ? today : addDays(today, 1),
      dueTime: index % 5 === 0 ? "16:00" : null,
      timerTargetSeconds: 1500,
      recurrenceRule: null,
      memo: "合成された性能検証メモ " + pad(index),
      sortOrder: index,
      completedAt: null,
      deletedAt: null,
      createdAt: now,
      updatedAt: now,
      tags: index % 4 === 0 ? [{ id: "tag-perf", name: "性能" }] : [],
      subtasks: subtasksFor(id, index)
    };
  });
  let scheduledStartDate = today;
  let scheduledStartTime = "09:00";
  let scheduledEndDate = today;
  let scheduledEndTime = "10:00";
  const taskRows = tasks.map((task, index) => ({
    id: task.id,
    listId: task.listId,
    boardColumnId: index % 3 === 0 ? "board-in-progress" : "board-todo",
    title: task.title,
    status: task.status,
    isFavorite: task.isFavorite,
    plannedStartDate: task.plannedStartDate,
    dueDate: task.dueDate,
    dueTime: task.dueTime,
    timerTargetSeconds: task.timerTargetSeconds,
    sortOrder: task.sortOrder,
    completedAt: null,
    createdAt: now,
    updatedAt: now,
    subtaskTotalCount: profile.subtasksPerTask,
    completedSubtaskCount: 1,
    activeTimerTarget: null,
    isTimerActive: false,
    tags: task.tags
  }));
  const taskLists = Array.from({ length: profile.listCount }, (_, index) => ({
    id: index === 0 ? "default" : "list-" + index,
    name: index === 0 ? "タスク" : "リスト " + index,
    colorToken: ["green", "blue", "amber", "rose"][index % 4],
    sortOrder: index,
    taskCount: index === 0 ? profile.totalTaskCount : 0,
    activeTaskCount: index === 0 ? profile.totalTaskCount : 0,
    completedTaskCount: 0,
    createdAt: now,
    updatedAt: now
  }));
  const boardColumns = [
    { id: "board-todo", title: "未着手", sortOrder: 0 },
    { id: "board-in-progress", title: "進行中", sortOrder: 1 }
  ].map((column) => ({
    ...column,
    taskCount: taskRows.filter((row) => row.boardColumnId === column.id).length,
    activeTaskCount: taskRows.filter((row) => row.boardColumnId === column.id).length,
    completedTaskCount: 0,
    createdAt: now,
    updatedAt: now
  }));
  const tags = [{
    id: "tag-perf",
    name: "性能",
    sortOrder: 0,
    taskCount: Math.ceil(profile.taskCount / 4),
    createdAt: now,
    updatedAt: now
  }];
  const taskPage = (request = {}) => {
    const scope = request.scope ?? { type: "board" };
    const scopedTasks = tasks.filter((task) => {
      if (scope.type === "list") {
        return task.listId === scope.listId;
      }
      if (scope.type === "today") {
        return task.dueDate === request.todayDate ||
          task.subtasks.some((subtask) => subtask.dueDate === request.todayDate);
      }
      if (scope.type === "favorites") {
        return task.isFavorite;
      }
      if (scope.type === "tag") {
        return task.tags.some((tag) => tag.id === scope.tagId);
      }
      return true;
    });
    const cursorIndex = request.cursor
      ? scopedTasks.findIndex((task) => task.id === request.cursor.id)
      : -1;
    const startIndex = cursorIndex >= 0 ? cursorIndex + 1 : 0;
    const limit = Math.max(1, Math.min(Number(request.limit ?? 200), 200));
    const pageTasks = scopedTasks.slice(startIndex, startIndex + limit);
    const pageTaskIds = new Set(pageTasks.map((task) => task.id));
    const pageRows = taskRows.filter((row) => pageTaskIds.has(row.id));
    const hasMore = startIndex + pageTasks.length < scopedTasks.length;
    const lastTask = pageTasks.at(-1);
    return {
      tasks: clone(pageTasks),
      rows: clone(pageRows),
      totalCount: scopedTasks.length,
      nextCursor: hasMore && lastTask ? {
        completionBucket: lastTask.status === "done" ? 1 : 0,
        sortOrder: lastTask.sortOrder,
        createdAt: lastTask.createdAt,
        id: lastTask.id
      } : null,
      navigationCounts: {
        todayCount: tasks.filter((task) =>
          task.status !== "done" &&
          (task.dueDate === request.todayDate ||
            task.subtasks.some((subtask) => subtask.dueDate === request.todayDate))
        ).length,
        favoriteCount: tasks.filter((task) => task.isFavorite).length
      }
    };
  };

  window.__taskTimerInvokeLog = [];
  window.__TASKTIMER_RENDER_COUNTS__ = {};
  window.__TAURI_INTERNALS__ = {
    invoke(command, args = {}) {
      window.__taskTimerInvokeLog.push(command);
      const rangeStart = args.startDate ?? args.weekStartDate ?? today;
      const rangeEnd = args.endDate ?? addDays(rangeStart, 6);
      const span = daySpan(rangeStart, rangeEnd);
      const overlapSchedules = new Map([
        [1, { id: "calendar-overlap-a", title: "重複予定 A", start: "18:00", end: "19:00" }],
        [2, { id: "calendar-overlap-b", title: "重複予定 B", start: "18:00", end: "19:00", isSubtask: true }],
        [3, { id: "calendar-overlap-c", title: "境界予定 C", start: "19:00", end: "19:30" }],
        [4, { id: "calendar-overlap-d", title: "連鎖予定 D", start: "20:00", end: "22:00" }],
        [5, { id: "calendar-overlap-e", title: "連鎖予定 E", start: "20:00", end: "20:30" }],
        [6, { id: "calendar-overlap-f", title: "連鎖予定 F", start: "20:30", end: "21:00" }]
      ]);
      const calendarItems = tasks.map((task, index) => {
        if (index === 0) {
          return {
            id: "calendar-perf-scheduled",
            target: { type: "task", id: task.id },
            title: task.title,
            parentTitle: null,
            date: scheduledStartDate,
            time: scheduledStartTime,
            endDate: scheduledEndDate,
            endTime: scheduledEndTime,
            isAllDay: false,
            marker: "scheduled",
            status: task.status,
            colorToken: "blue"
          };
        }
        const overlap = overlapSchedules.get(index);
        if (overlap) {
          const subtask = overlap.isSubtask ? task.subtasks[1] : null;
          return {
            id: overlap.id,
            target: subtask
              ? { type: "subtask", id: subtask.id }
              : { type: "task", id: task.id },
            title: overlap.title,
            parentTitle: subtask ? task.title : null,
            date: today,
            time: overlap.start,
            endDate: today,
            endTime: overlap.end,
            isAllDay: false,
            marker: "scheduled",
            status: subtask?.status ?? task.status,
            colorToken: subtask ? "violet" : "blue"
          };
        }
        return {
          id: "calendar-perf-" + pad(index),
          target: { type: "task", id: task.id },
          title: task.title,
          parentTitle: null,
          date: addDays(rangeStart, index % span),
          time: null,
          endDate: null,
          endTime: null,
          isAllDay: true,
          marker: "due",
          status: task.status,
          colorToken: "green"
        };
      });
      const commands = {
        health_check: () => "tauri-ready",
        list_tasks: () => clone(tasks),
        list_task_page: () => taskPage(args.request),
        list_task_lists: () => clone(taskLists),
        list_board_columns: () => clone(boardColumns),
        list_tags: () => clone(tags),
        list_task_rows: () => clone(taskRows),
        list_calendar_items: () => clone(calendarItems),
        list_week_calendar_items: () => clone(calendarItems),
        toggle_task_favorite: () => {
          const task = tasks.find(
            (candidate) => candidate.id === args.request?.taskId
          );
          const row = taskRows.find(
            (candidate) => candidate.id === args.request?.taskId
          );
          if (!task || !row) {
            throw new Error("お気に入り変更対象が存在しません");
          }
          task.isFavorite = Boolean(args.request.isFavorite);
          row.isFavorite = task.isFavorite;
          task.updatedAt = now;
          row.updatedAt = now;
          return clone(task);
        },
        complete_task: () => {
          const task = tasks.find(
            (candidate) => candidate.id === args.request?.taskId
          );
          const row = taskRows.find(
            (candidate) => candidate.id === args.request?.taskId
          );
          if (!task || !row) {
            throw new Error("完了対象が存在しません");
          }
          task.status = "done";
          task.completedAt = now;
          task.updatedAt = now;
          row.status = "done";
          row.completedAt = now;
          row.updatedAt = now;
          return clone(task);
        },
        update_task: () => {
          const task = tasks.find(
            (candidate) => candidate.id === args.request?.taskId
          );
          const row = taskRows.find(
            (candidate) => candidate.id === args.request?.taskId
          );
          if (!task || !row) {
            throw new Error("更新対象が存在しません");
          }
          const request = args.request;
          task.listId = request.listId ?? task.listId;
          task.title = request.title ?? task.title;
          task.plannedStartDate = request.plannedStartDate ?? null;
          task.dueDate = request.dueDate ?? null;
          task.dueTime = request.dueTime ?? null;
          task.timerTargetSeconds = request.timerTargetSeconds ?? null;
          task.memo = request.memo ?? "";
          task.updatedAt = now;
          row.listId = task.listId;
          row.title = task.title;
          row.plannedStartDate = task.plannedStartDate;
          row.dueDate = task.dueDate;
          row.dueTime = task.dueTime;
          row.timerTargetSeconds = task.timerTargetSeconds;
          row.updatedAt = now;
          return clone(task);
        },
        move_task_to_board_column: () => new Promise((resolve, reject) => {
          setTimeout(() => {
            const row = taskRows.find(
              (candidate) => candidate.id === args.request?.taskId
            );
            if (!row) {
              reject(new Error("移動対象のタスクが存在しません"));
              return;
            }
            row.boardColumnId = args.request.boardColumnId;
            resolve(null);
          }, 120);
        }),
        move_scheduled_work_item: () => new Promise((resolve, reject) => {
          setTimeout(() => {
            const destination = args.request?.destination;
            if (!destination?.startDate || !destination?.startTime) {
              reject(new Error("移動先日時が不正です"));
              return;
            }
            const durationMinutes =
              (new Date(scheduledEndDate + "T" + scheduledEndTime) -
                new Date(scheduledStartDate + "T" + scheduledStartTime)) / 60000;
            const nextEnd = addMinutes(
              destination.startDate,
              destination.startTime,
              durationMinutes
            );
            scheduledStartDate = destination.startDate;
            scheduledStartTime = destination.startTime;
            scheduledEndDate = nextEnd.date;
            scheduledEndTime = nextEnd.time;
            resolve(null);
          }, 120);
        }),
        resize_scheduled_work_item: () => new Promise((resolve, reject) => {
          setTimeout(() => {
            const schedule = args.request?.schedule;
            if (!schedule?.startDate || !schedule?.endDate) {
              reject(new Error("変更後の予定期間が不正です"));
              return;
            }
            if (
              schedule.isAllDay === false &&
              (!schedule.startTime || !schedule.endTime)
            ) {
              reject(new Error("変更後の予定時刻が不正です"));
              return;
            }
            scheduledStartDate = schedule.startDate;
            scheduledStartTime = schedule.startTime;
            scheduledEndDate = schedule.endDate;
            scheduledEndTime = schedule.endTime;
            resolve(null);
          }, 120);
        }),
        get_active_timer: () => null,
        get_active_pomodoro: () => null,
        sync_expired_pomodoro: () => ({
          expiredPomodoro: null,
          activePomodoro: null,
          notificationSummary: { attempted: 0, succeeded: 0, failed: 0, lastError: null }
        }),
        get_pomodoro_settings: () => ({
          id: "default",
          workSeconds: 1500,
          shortBreakSeconds: 300,
          longBreakSeconds: 900,
          cyclesUntilLongBreak: 4,
          autoStartBreak: false,
          autoStartNextWork: false,
          updatedAt: now
        }),
        get_notification_display_mode: () => "title_only",
        get_notifications_enabled: () => true,
        get_ui_preferences: () => ({
          leftPaneOpen: true,
          lastView: "list",
          lastTaskListId: "default",
          calendarViewMode: "week"
        }),
        update_ui_preferences: () => ({
          leftPaneOpen: true,
          lastView: "list",
          lastTaskListId: "default",
          calendarViewMode: "week"
        }),
        sync_notifications: () => ({
          dispatchSummary: { attempted: 0, succeeded: 0, failed: 0, lastError: null },
          nextSchedule: null
        }),
        process_notification_os_registrations: () => ({
          attempted: 0,
          registered: 0,
          cancelled: 0,
          skipped: 0,
          failed: 0,
          lastError: null
        })
      };
      const handler = commands[command];
      if (!handler) {
        return Promise.reject(new Error("UI performance mock does not implement: " + command));
      }
      return Promise.resolve(handler());
    },
    transformCallback() { return 1; },
    unregisterCallback() {},
    convertFileSrc(value) { return value; }
  };
})();
`;
}
