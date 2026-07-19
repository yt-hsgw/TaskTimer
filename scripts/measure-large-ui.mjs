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
  calendar_move: 1500,
  calendar_resize: 1500,
  calendar_day: 1500,
  calendar_month: 2000,
  calendar_month_date_change: 1500,
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
        document.querySelectorAll(".calendar-item").length === ${profile.taskCount}`,
    }),
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
        document.querySelectorAll(".calendar-item").length === ${profile.taskCount}`,
    }),
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

async function verifyCalendarScheduledMove(client, sessionId) {
  await resetInvokeLog(client, sessionId);
  const result = await evaluateValue(
    client,
    sessionId,
    `(async () => {
      const source = document.querySelector(
        '.calendar-item.marker-scheduled.is-timed .calendar-item-content'
      );
      const sourceDate = source?.closest('.calendar-time-cell')?.dataset.calendarDate;
      const destination = [...document.querySelectorAll('.calendar-time-cell')]
        .find((cell) =>
          cell.dataset.calendarDate === sourceDate &&
          cell.getAttribute('aria-label')?.includes('11:00')
        );
      if (!source || !destination) {
        return null;
      }
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
      const moved = destination.querySelector(
        '.calendar-item.marker-scheduled.is-timed'
      );
      return {
        movedImmediately: Boolean(moved),
        marker: moved?.querySelector('small:last-child')?.textContent ?? null,
        targetDate: destination.dataset.calendarDate ?? null
      };
    })()`,
  );
  if (!result?.movedImmediately || !result.marker?.includes("11:30")) {
    throw new Error(
      `カレンダードロップ直後の仮位置が不正です: ${JSON.stringify(result)}`,
    );
  }

  await waitForPaintedExpression(
    client,
    sessionId,
    `(() => {
      const destination = [...document.querySelectorAll('.calendar-time-cell')]
        .find((cell) =>
          cell.dataset.calendarDate === ${JSON.stringify(result.targetDate)} &&
          cell.getAttribute('aria-label')?.includes('11:00')
        );
      const moved = destination?.querySelector(
        '.calendar-item.marker-scheduled.is-timed'
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
        '.calendar-item.marker-scheduled.is-timed .calendar-item-content'
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
        '.calendar-item.marker-scheduled.is-timed small:last-child'
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
        '.calendar-item.marker-scheduled.is-timed'
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
              preview?.querySelector('.calendar-resize-preview-label')?.textContent === '変更後' &&
              preview?.querySelector('small:last-child')?.textContent?.includes('13:15')
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
        '.calendar-month-day .calendar-item.marker-scheduled:not(.is-resize-preview)'
      );
      const sourceCell = item?.closest('.calendar-month-day');
      const sourceDate = sourceCell?.dataset.calendarDate;
      const handle = item?.querySelector(
        '.calendar-resize-handle.is-end.is-horizontal'
      );
      if (!item || !sourceDate || !handle) {
        return null;
      }
      const [year, month, day] = sourceDate.split('-').map(Number);
      const target = new Date(year, month - 1, day);
      target.setDate(target.getDate() + 1);
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
        await waitForPaintedExpression(
          client,
          sessionId,
          `(() => {
            const source = document.querySelector(
              '.calendar-month-day[data-calendar-date=${JSON.stringify(
                resizeGeometry.sourceDate,
              )}] .calendar-item.marker-scheduled:not(.is-resize-preview)'
            );
            const preview = document.querySelector(
              '.calendar-month-day[data-calendar-date=${JSON.stringify(
                resizeGeometry.targetDate,
              )}] .calendar-item.marker-scheduled.is-resize-preview'
            );
            return Boolean(
              source &&
              preview?.querySelector('.calendar-resize-preview-label')?.textContent === '変更後' &&
              preview?.querySelector('small:last-child')?.textContent?.includes('13:15')
            );
          })()`,
        );
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
        )}] .calendar-item.marker-scheduled:not(.is-resize-preview)'
      );
      return Boolean(
        target?.querySelector('small:last-child')?.textContent?.includes('13:15') &&
        !document.querySelector('.calendar-item.is-resize-preview') &&
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
        '.calendar-item.marker-scheduled:not(.is-resize-preview) .calendar-item-content'
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
        movedImmediately: Boolean(destinationCell.querySelector(
          '.calendar-item.marker-scheduled:not(.is-resize-preview)'
        ))
      };
    })()`,
  );
  if (!moveResult?.movedImmediately) {
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
        )}] .calendar-item.marker-scheduled:not(.is-resize-preview)'
      );
      return Boolean(
        destination &&
        window.__taskTimerInvokeLog?.includes('move_scheduled_work_item') &&
        window.__taskTimerInvokeLog?.includes('list_task_page') &&
        window.__taskTimerInvokeLog?.includes('list_calendar_items') &&
        !document.querySelector('.app-alert')
      );
    })()`,
  );
  return {
    commands: [...resizeCommands, ...(await takeInvokeLog(client, sessionId))],
  };
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
      const calendarItems = tasks.map((task, index) => index === 0 ? {
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
      } : {
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
