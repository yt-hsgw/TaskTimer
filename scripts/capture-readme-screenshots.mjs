import { spawn } from "node:child_process";
import { access, mkdir, rm, writeFile } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const outputDir = path.join(repoRoot, "docs", "assets", "readme");
const outputPath = path.join(outputDir, "tasktimer-overview.png");
const kanbanOutputPath = path.join(outputDir, "tasktimer-kanban.png");
const chromePath = await resolveChromePath();

const vitePort = await getFreePort();
const debugPort = await getFreePort();
const userDataDir = await makeTempDir("tasktimer-readme-chrome-");
let viteProcess;
let chromeProcess;

try {
  await mkdir(outputDir, { recursive: true });
  viteProcess = startVite(vitePort);
  await waitForHttp(`http://127.0.0.1:${vitePort}/`);

  chromeProcess = startChrome(debugPort, userDataDir);
  const browserWsUrl = await waitForChromeWebSocket(debugPort);
  const client = await createCdpClient(browserWsUrl);
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
    { source: buildTauriInvokeMockSource() },
    sessionId,
  );
  await client.send(
    "Page.navigate",
    { url: `http://127.0.0.1:${vitePort}/` },
    sessionId,
  );

  await waitForExpression(
    client,
    sessionId,
    `Boolean(document.querySelector(".left-navigation") && document.querySelector(".task-row-content") && !document.querySelector(".app-alert"))`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector(".task-row-content")?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(document.querySelector(".task-detail-pane") && !document.querySelector(".app-alert"))`,
  );
  await waitForExpression(
    client,
    sessionId,
    `(() => {
      const workspace = document.querySelector(".task-workspace");
      const panel = document.querySelector(".task-panel");
      return Boolean(
        workspace &&
        panel &&
        document.querySelector(".detail-list-card") &&
        document.querySelector(".detail-tag-management") &&
        Math.abs(workspace.getBoundingClientRect().width - panel.getBoundingClientRect().width) < 2
      );
    })()`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector(".task-row-content")?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `!document.querySelector(".task-detail-pane")`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector('button.nav-item[aria-label="カレンダー"]')?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(
      document.querySelector(".calendar-panel") &&
      document.querySelector(".calendar-time-cell") &&
      !document.querySelector(".calendar-add-task-button") &&
      !document.querySelector(".calendar-cell-add-button")
    )`,
  );
  await waitForExpression(
    client,
    sessionId,
    `(() => {
      const block = document.querySelector(".calendar-item.marker-scheduled.is-timed");
      const handles = block?.querySelectorAll(".calendar-resize-handle") ?? [];
      const content = block?.querySelector(".calendar-item-content");
      return Boolean(
        block &&
        handles.length === 2 &&
        content &&
        block.getBoundingClientRect().height >= 130 &&
        content.scrollWidth <= content.clientWidth + 1
      );
    })()`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector(".calendar-resize-handle.is-end.is-vertical")?.dispatchEvent(
        new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true })
      )`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(
      document.querySelector(".calendar-resize-handle.is-end.is-vertical:not(:disabled)") &&
      !document.querySelector(".app-alert")
    )`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `[...document.querySelectorAll(".calendar-view-switch button")]
        .find((button) => button.textContent === "日")?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(
      document.querySelector(".calendar-time-grid.is-day-mode .marker-scheduled") &&
      document.querySelectorAll(".calendar-time-grid.is-day-mode .calendar-resize-handle").length === 2
    )`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `[...document.querySelectorAll(".calendar-view-switch button")]
        .find((button) => button.textContent === "月")?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(
      document.querySelector(".calendar-month-grid .marker-scheduled") &&
      document.querySelectorAll(".calendar-month-grid .calendar-resize-handle.is-horizontal").length === 2
    )`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `[...document.querySelectorAll(".calendar-view-switch button")]
        .find((button) => button.textContent === "週")?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(document.querySelector(".calendar-time-grid:not(.is-day-mode) .marker-scheduled"))`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector(".calendar-time-cell")?.dispatchEvent(
        new MouseEvent("dblclick", { bubbles: true })
      )`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(document.querySelector(".calendar-create-form"))`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector(".calendar-create-form-heading .inline-icon-button")?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector('button.nav-item[aria-label="設定"]')?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(
      document.querySelector(".settings-panel") &&
      document.querySelector(".notification-mode-cards") &&
      document.querySelector("#export-title") &&
      !document.querySelector(".notification-history")
    )`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector('button.nav-item[aria-label="かんばん"]')?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `(() => {
      const board = document.querySelector(".kanban-board");
      const columns = [...document.querySelectorAll(".kanban-column")];
      const cards = [...document.querySelectorAll(".kanban-card")];
      return Boolean(
        board &&
        columns.length === 2 &&
        cards.length === 3 &&
        document.querySelectorAll(".kanban-drag-handle").length === 2 &&
        document.querySelectorAll(".kanban-card-drag-handle").length === 3 &&
        document.querySelector(".kanban-card .task-check-button") &&
        !document.querySelector(".kanban-card-actions") &&
        columns.every((column) => column.scrollWidth <= column.clientWidth + 1) &&
        cards.every((card) => card.scrollWidth <= card.clientWidth + 1)
      );
    })()`,
  );
  const kanbanScreenshot = await client.send(
    "Page.captureScreenshot",
    { format: "png", fromSurface: true },
    sessionId,
  );
  await writeFile(
    kanbanOutputPath,
    Buffer.from(kanbanScreenshot.data, "base64"),
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector('button.nav-item[aria-label="タスク"]')?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(document.querySelector(".task-row-content"))`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector(".task-row-content")?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(document.querySelector(".task-detail-pane"))`,
  );
  await waitForExpression(
    client,
    sessionId,
    `(async () => {
      if (document.fonts?.ready) {
        await document.fonts.ready;
      }
      return true;
    })()`,
  );
  await client.send(
    "Runtime.evaluate",
    { expression: "window.scrollTo(0, 0)", awaitPromise: true },
    sessionId,
  );
  await sleep(300);

  const screenshot = await client.send(
    "Page.captureScreenshot",
    {
      format: "png",
      fromSurface: true,
      captureBeyondViewport: false,
    },
    sessionId,
  );
  await writeFile(outputPath, Buffer.from(screenshot.data, "base64"));
  await client.close();
  console.log(`README screenshot written: ${path.relative(repoRoot, outputPath)}`);
  console.log(
    `README screenshot written: ${path.relative(repoRoot, kanbanOutputPath)}`,
  );
} finally {
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

function startVite(port) {
  const child = spawn(
    process.execPath,
    [
      "node_modules/vite/bin/vite.js",
      "--host",
      "127.0.0.1",
      "--port",
      String(port),
      "--strictPort",
    ],
    {
      cwd: repoRoot,
      env: { ...process.env, BROWSER: "none" },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  child.stdout.on("data", (chunk) => process.stdout.write(chunk));
  child.stderr.on("data", (chunk) => process.stderr.write(chunk));
  return child;
}

function startChrome(debugPort, dataDir) {
  const child = spawn(
    chromePath,
    [
      "--headless=new",
      "--disable-gpu",
      "--disable-dev-shm-usage",
      "--hide-scrollbars",
      "--no-first-run",
      "--no-default-browser-check",
      `--remote-debugging-port=${debugPort}`,
      `--user-data-dir=${dataDir}`,
      "about:blank",
    ],
    {
      stdio: ["ignore", "ignore", "pipe"],
    },
  );
  child.stderr.on("data", (chunk) => process.stderr.write(chunk));
  return child;
}

async function createCdpClient(wsUrl) {
  const socket = new WebSocket(wsUrl);
  const callbacks = new Map();
  let nextId = 1;

  await new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });

  socket.addEventListener("message", (event) => {
    const message = JSON.parse(String(event.data));
    if (!message.id) {
      return;
    }
    const callback = callbacks.get(message.id);
    if (!callback) {
      return;
    }
    callbacks.delete(message.id);
    if (message.error) {
      callback.reject(new Error(message.error.message));
      return;
    }
    callback.resolve(message.result ?? {});
  });

  return {
    send(method, params = {}, sessionId) {
      const id = nextId++;
      const message = { id, method, params };
      if (sessionId) {
        message.sessionId = sessionId;
      }
      socket.send(JSON.stringify(message));
      return new Promise((resolve, reject) => {
        callbacks.set(id, { resolve, reject });
      });
    },
    close() {
      socket.close();
    },
  };
}

async function waitForExpression(client, sessionId, expression, timeoutMs = 10000) {
  const deadline = Date.now() + timeoutMs;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const result = await client.send(
        "Runtime.evaluate",
        {
          expression,
          awaitPromise: true,
          returnByValue: true,
        },
        sessionId,
      );
      if (result.result?.value) {
        return;
      }
    } catch (error) {
      lastError = error;
    }
    await sleep(250);
  }
  throw new Error(`Timed out waiting for page expression. ${lastError ?? ""}`);
}

async function waitForChromeWebSocket(port) {
  const endpoint = `http://127.0.0.1:${port}/json/version`;
  const deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(endpoint);
      if (response.ok) {
        const version = await response.json();
        return version.webSocketDebuggerUrl;
      }
    } catch {
      // Keep polling until Chrome exposes the debugging endpoint.
    }
    await sleep(200);
  }
  throw new Error("Chrome DevTools endpoint did not become available.");
}

async function waitForHttp(url) {
  const deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
    } catch {
      // Keep polling until Vite is ready.
    }
    await sleep(200);
  }
  throw new Error(`Vite server did not become available: ${url}`);
}

function getFreePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      const port = typeof address === "object" && address ? address.port : null;
      server.close(() => {
        if (port) {
          resolve(port);
          return;
        }
        reject(new Error("Could not allocate a free port."));
      });
    });
    server.on("error", reject);
  });
}

async function makeTempDir(prefix) {
  return fsMkTemp(path.join(os.tmpdir(), prefix));
}

async function fsMkTemp(prefix) {
  const { mkdtemp } = await import("node:fs/promises");
  return mkdtemp(prefix);
}

async function resolveChromePath() {
  if (process.env.CHROME_PATH) {
    return process.env.CHROME_PATH;
  }

  const candidates = [
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
  ];

  for (const candidate of candidates) {
    if (await fileExists(candidate)) {
      return candidate;
    }
  }

  const pathCandidate = await findExecutableOnPath([
    "google-chrome",
    "chrome",
    "chromium",
    "chromium-browser",
  ]);
  if (pathCandidate) {
    return pathCandidate;
  }

  throw new Error(
    "Chrome executable was not found. Set CHROME_PATH to generate README screenshots.",
  );
}

async function findExecutableOnPath(names) {
  const pathDirs = (process.env.PATH ?? "").split(path.delimiter).filter(Boolean);
  const extensions =
    process.platform === "win32"
      ? (process.env.PATHEXT ?? ".EXE;.CMD;.BAT;.COM").split(";")
      : [""];

  for (const dir of pathDirs) {
    for (const name of names) {
      for (const extension of extensions) {
        const candidate = path.join(dir, `${name}${extension}`);
        if (await fileExists(candidate)) {
          return candidate;
        }
      }
    }
  }
  return null;
}

async function fileExists(candidate) {
  try {
    await access(candidate);
    return true;
  } catch {
    return false;
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function waitForProcessExit(child, timeoutMs) {
  if (child.exitCode !== null || child.signalCode !== null) {
    return Promise.resolve();
  }

  return new Promise((resolve) => {
    const timeout = setTimeout(resolve, timeoutMs);
    child.once("exit", () => {
      clearTimeout(timeout);
      resolve();
    });
  });
}

async function rmWithRetry(targetPath, attempts = 5) {
  let lastError;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      await rm(targetPath, { recursive: true, force: true });
      return;
    } catch (error) {
      lastError = error;
      await sleep(200 * attempt);
    }
  }
  throw lastError;
}

function buildTauriInvokeMockSource() {
  return `
(() => {
  const now = "2026-07-08T09:00:00Z";
  const tasks = [
    {
      id: "task-weekly-review",
      listId: "default",
      title: "週次レビュー資料を作成",
      status: "in_progress",
      isFavorite: true,
      plannedStartDate: "2026-07-06",
      dueDate: "2026-07-10",
      dueTime: "16:00",
      timerTargetSeconds: 5400,
      recurrenceRule: { frequency: "weekly", interval: 1 },
      memo: "レビュー観点を整理し、会議前に共有する。",
      sortOrder: 10,
      completedAt: null,
      deletedAt: null,
      createdAt: now,
      updatedAt: now,
      tags: [{ id: "tag-priority", name: "重要" }],
      subtasks: [
        {
          id: "subtask-collect",
          taskId: "task-weekly-review",
          title: "進捗メモを集約",
          status: "done",
          plannedStartDate: "2026-07-06",
          dueDate: "2026-07-07",
          dueTime: null,
          timerTargetSeconds: 1800,
          recurrenceRule: null,
          memo: "",
          sortOrder: 10,
          completedAt: "2026-07-07T03:00:00Z",
          deletedAt: null,
          createdAt: now,
          updatedAt: now
        },
        {
          id: "subtask-summary",
          taskId: "task-weekly-review",
          title: "要点を3つにまとめる",
          status: "in_progress",
          plannedStartDate: "2026-07-08",
          dueDate: "2026-07-09",
          dueTime: "11:00",
          timerTargetSeconds: 2400,
          recurrenceRule: null,
          memo: "",
          sortOrder: 20,
          completedAt: null,
          deletedAt: null,
          createdAt: now,
          updatedAt: now
        }
      ]
    },
    {
      id: "task-release-check",
      listId: "default",
      title: "リリース前チェック",
      status: "todo",
      isFavorite: false,
      plannedStartDate: "2026-07-09",
      dueDate: "2026-07-10",
      dueTime: null,
      timerTargetSeconds: 3600,
      recurrenceRule: null,
      memo: "macOSとWindowsでインストール確認を行う。",
      sortOrder: 20,
      completedAt: null,
      deletedAt: null,
      createdAt: now,
      updatedAt: now,
      tags: [],
      subtasks: []
    },
    {
      id: "task-design-notes",
      listId: "default",
      title: "設計メモを整理",
      status: "done",
      isFavorite: false,
      plannedStartDate: null,
      dueDate: "2026-07-08",
      dueTime: null,
      timerTargetSeconds: 1800,
      recurrenceRule: null,
      memo: "決定事項を設計資料へ反映する。",
      sortOrder: 30,
      completedAt: "2026-07-08T08:00:00Z",
      deletedAt: null,
      createdAt: now,
      updatedAt: now,
      tags: [],
      subtasks: []
    }
  ];
  const activeTimer = {
    id: "timer-subtask-summary",
    target: { type: "subtask", id: "subtask-summary" },
    startedAt: "2026-07-08T08:30:00Z",
    stoppedAt: null,
    elapsedSeconds: null,
    pausedAt: null,
    deletedAt: null,
    createdAt: "2026-07-08T08:30:00Z"
  };
  const taskLists = [
    {
      id: "default",
      name: "タスク",
      colorToken: "green",
      sortOrder: 0,
      taskCount: 3,
      activeTaskCount: 2,
      completedTaskCount: 1,
      createdAt: now,
      updatedAt: now
    }
  ];
  const boardColumns = [
    {
      id: "board-todo",
      title: "未着手",
      sortOrder: 0,
      taskCount: 1,
      activeTaskCount: 1,
      completedTaskCount: 0,
      createdAt: now,
      updatedAt: now
    },
    {
      id: "board-in-progress",
      title: "進行中",
      sortOrder: 1,
      taskCount: 2,
      activeTaskCount: 1,
      completedTaskCount: 1,
      createdAt: now,
      updatedAt: now
    }
  ];
  const tags = [
    {
      id: "tag-priority",
      name: "重要",
      sortOrder: 0,
      taskCount: 1,
      createdAt: now,
      updatedAt: now
    },
    {
      id: "tag-review",
      name: "レビュー",
      sortOrder: 10,
      taskCount: 0,
      createdAt: now,
      updatedAt: now
    }
  ];
  const taskRows = [
    {
      id: "task-weekly-review",
      listId: "default",
      boardColumnId: "board-in-progress",
      title: "週次レビュー資料を作成",
      status: "in_progress",
      isFavorite: true,
      plannedStartDate: "2026-07-06",
      dueDate: "2026-07-10",
      dueTime: "16:00",
      timerTargetSeconds: 5400,
      sortOrder: 10,
      completedAt: null,
      createdAt: now,
      updatedAt: now,
      subtaskTotalCount: 2,
      completedSubtaskCount: 1,
      activeTimerTarget: { type: "subtask", id: "subtask-summary" },
      isTimerActive: true,
      tags: [{ id: "tag-priority", name: "重要" }]
    },
    {
      id: "task-release-check",
      listId: "default",
      boardColumnId: "board-todo",
      title: "リリース前チェック",
      status: "todo",
      isFavorite: false,
      plannedStartDate: "2026-07-09",
      dueDate: "2026-07-10",
      dueTime: null,
      timerTargetSeconds: 3600,
      sortOrder: 20,
      completedAt: null,
      createdAt: now,
      updatedAt: now,
      subtaskTotalCount: 0,
      completedSubtaskCount: 0,
      activeTimerTarget: null,
      isTimerActive: false,
      tags: []
    },
    {
      id: "task-design-notes",
      listId: "default",
      boardColumnId: "board-in-progress",
      title: "設計メモを整理",
      status: "done",
      isFavorite: false,
      plannedStartDate: null,
      dueDate: "2026-07-08",
      dueTime: null,
      timerTargetSeconds: 1800,
      sortOrder: 30,
      completedAt: "2026-07-08T08:00:00Z",
      createdAt: now,
      updatedAt: now,
      subtaskTotalCount: 0,
      completedSubtaskCount: 0,
      activeTimerTarget: null,
      isTimerActive: false,
      tags: []
    }
  ];

  function clone(value) {
    return JSON.parse(JSON.stringify(value));
  }

  function addDays(dateText, days) {
    const [year, month, day] = dateText.split("-").map(Number);
    const date = new Date(year, month - 1, day);
    date.setDate(date.getDate() + days);
    return [
      date.getFullYear(),
      String(date.getMonth() + 1).padStart(2, "0"),
      String(date.getDate()).padStart(2, "0")
    ].join("-");
  }

  window.__TAURI_INTERNALS__ = {
    invoke(command, args = {}) {
      const rangeStart = args.startDate ?? args.weekStartDate ?? "2026-07-06";
      const calendarItems = [
        {
          id: "cal-review-scheduled",
          target: { type: "task", id: "task-weekly-review" },
          title: "週次レビュー資料を作成",
          parentTitle: null,
          date: addDays(rangeStart, 0),
          time: "09:00",
          endDate: addDays(rangeStart, 0),
          endTime: "11:30",
          isAllDay: false,
          marker: "scheduled",
          status: "in_progress",
          colorToken: "green"
        },
        {
          id: "cal-review-start",
          target: { type: "task", id: "task-weekly-review" },
          title: "週次レビュー資料を作成",
          parentTitle: null,
          date: addDays(rangeStart, 0),
          time: null,
          endDate: null,
          endTime: null,
          isAllDay: true,
          marker: "planned_start",
          status: "in_progress",
          colorToken: "green"
        },
        {
          id: "cal-summary-active",
          target: { type: "subtask", id: "subtask-summary" },
          title: "要点を3つにまとめる",
          parentTitle: "週次レビュー資料を作成",
          date: addDays(rangeStart, 2),
          time: "10:15",
          endDate: null,
          endTime: null,
          isAllDay: false,
          marker: "active_timer",
          status: "in_progress",
          colorToken: "green"
        },
        {
          id: "cal-release-due",
          target: { type: "task", id: "task-release-check" },
          title: "リリース前チェック",
          parentTitle: null,
          date: addDays(rangeStart, 4),
          time: null,
          endDate: null,
          endTime: null,
          isAllDay: true,
          marker: "due",
          status: "todo",
          colorToken: "green"
        }
      ];
      const commands = {
        health_check: () => "tauri-ready",
        list_tasks: () => clone(tasks),
        list_task_lists: () => clone(taskLists),
        list_board_columns: () => clone(boardColumns),
        list_tags: () => clone(tags),
        list_task_rows: () => clone(taskRows),
        list_calendar_items: () => clone(calendarItems),
        list_week_calendar_items: () => clone(calendarItems),
        get_active_timer: () => clone(activeTimer),
        get_active_pomodoro: () => null,
        sync_expired_pomodoro: () => ({
          expiredPomodoro: null,
          activePomodoro: null,
          notificationSummary: {
            attempted: 0,
            succeeded: 0,
            failed: 0,
            lastError: null
          }
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
          dispatchSummary: {
            attempted: 0,
            succeeded: 0,
            failed: 0,
            lastError: null
          },
          nextSchedule: null
        }),
        process_notification_os_registrations: () => ({
          attempted: 0,
          registered: 0,
          cancelled: 0,
          skipped: 0,
          failed: 0,
          lastError: null
        }),
        dispatch_due_notifications: () => ({
          attempted: 1,
          succeeded: 1,
          failed: 0,
          lastError: null
        }),
        resize_scheduled_work_item: () => null
      };
      const handler = commands[command];
      if (!handler) {
        return Promise.reject(new Error("README screenshot mock does not implement: " + command));
      }
      return Promise.resolve(handler());
    },
    transformCallback() {
      return 1;
    },
    unregisterCallback() {},
    convertFileSrc(value) {
      return value;
    }
  };
})();
`;
}
