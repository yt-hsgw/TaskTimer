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
      totalTaskCount: 5000,
      taskCount: 200,
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
  today: 1000,
  favorites: 1000,
  kanban: 1500,
  calendar_week: 1500,
  calendar_day: 1500,
  calendar_month: 2000,
  task_detail: 1000,
};
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
    `document.querySelectorAll(".task-row").length === ${profile.taskCount} &&
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

  measurements.push(
    await measureView({
      client,
      sessionId,
      name: "today",
      thresholdMs: thresholds.today,
      action: clickNavigation("今日"),
      ready: `document.querySelector('button.nav-item[aria-label="今日"][aria-current="page"]') &&
        document.querySelector('#task-panel-title')?.textContent === "今日" &&
        document.querySelectorAll(".task-row").length === ${Math.ceil(profile.taskCount / 2)}`,
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
        document.querySelectorAll(".task-row").length === ${Math.ceil(profile.taskCount / 3)}`,
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
        document.querySelectorAll(".kanban-card").length === ${profile.taskCount}`,
    }),
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

  await evaluate(client, sessionId, clickNavigation("タスク"));
  await waitForPaintedExpression(
    client,
    sessionId,
    `document.querySelector('#task-panel-title')?.textContent === "タスク" &&
      document.querySelectorAll(".task-row").length === ${profile.taskCount}`,
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

  window.__TAURI_INTERNALS__ = {
    invoke(command, args = {}) {
      const rangeStart = args.startDate ?? args.weekStartDate ?? today;
      const rangeEnd = args.endDate ?? addDays(rangeStart, 6);
      const span = daySpan(rangeStart, rangeEnd);
      const calendarItems = tasks.map((task, index) => ({
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
      }));
      const commands = {
        health_check: () => "tauri-ready",
        list_tasks: () => clone(tasks),
        list_task_lists: () => clone(taskLists),
        list_board_columns: () => clone(boardColumns),
        list_tags: () => clone(tags),
        list_task_rows: () => clone(taskRows),
        list_calendar_items: () => clone(calendarItems),
        list_week_calendar_items: () => clone(calendarItems),
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
