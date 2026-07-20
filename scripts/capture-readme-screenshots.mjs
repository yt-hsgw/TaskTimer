import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  createCdpClient,
  getFreePort,
  makeTempDir,
  resolveChromePath,
  rmWithRetry,
  sleep,
  startChrome,
  startVite,
  waitForChromeWebSocket,
  waitForExpression,
  waitForHttp,
  waitForProcessExit,
} from "./lib/headless-chrome.mjs";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const outputDir = path.join(repoRoot, "docs", "assets", "readme");
const outputPath = path.join(outputDir, "tasktimer-overview.png");
const kanbanOutputPath = path.join(outputDir, "tasktimer-kanban.png");
const taskCreateOutputPath = path.join(
  outputDir,
  "tasktimer-task-create.png",
);
const chromePath = await resolveChromePath();

const vitePort = await getFreePort();
const debugPort = await getFreePort();
const userDataDir = await makeTempDir("tasktimer-readme-chrome-");
let viteProcess;
let chromeProcess;

try {
  await mkdir(outputDir, { recursive: true });
  viteProcess = startVite(repoRoot, vitePort);
  await waitForHttp(`http://127.0.0.1:${vitePort}/`);

  chromeProcess = startChrome(chromePath, debugPort, userDataDir);
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
    `Boolean(
      document.querySelector(".left-navigation") &&
      document.querySelector(".task-row-content") &&
      document.querySelector(".global-search input") &&
      document.querySelector('.nav-section-toggle[aria-expanded="true"]') &&
      document.querySelector(".nav-list-add-button") &&
      [...document.querySelectorAll('.nav-sections .nav-item')]
        .slice(0, 2)
        .map((button) => button.getAttribute("aria-label"))
        .join(",") === "今日,お気に入り" &&
      document.querySelectorAll(".workspace-mode-switcher [role=tab]").length === 3 &&
      !document.querySelector('button.nav-item[aria-label="カレンダー"]') &&
      !document.querySelector('button.nav-item[aria-label="かんばん"]') &&
      !document.querySelector('button.nav-item[aria-label="ポモドーロ"]') &&
      ![...document.querySelectorAll(".nav-section-heading")]
        .some((heading) => heading.textContent?.trim() === "タグ") &&
      !document.querySelector(".app-alert")
    )`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector(".nav-section-toggle")?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(
      document.querySelector('.nav-section-toggle[aria-expanded="false"]') &&
      document.querySelector("#navigation-task-lists")?.hidden
    )`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector(".nav-section-toggle")?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(
      document.querySelector('.nav-section-toggle[aria-expanded="true"]') &&
      !document.querySelector("#navigation-task-lists")?.hidden
    )`,
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
    `Boolean(
      document.querySelector(".task-detail-pane") &&
      !document.querySelector('.detail-section[aria-label="タイマー"]') &&
      !document.querySelector('.detail-section[aria-label="通知"]') &&
      !document.querySelector(".detail-color-button") &&
      !document.querySelector(".app-alert")
    )`,
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
      expression: `(() => {
        const input = document.querySelector(".global-search input");
        const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;
        setter?.call(input, "レビュー");
        input?.dispatchEvent(new Event("input", { bubbles: true }));
      })()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(document.querySelector(".global-search-result"))`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector(".global-search-result")?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(document.querySelector(".task-detail-pane"))`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector('button[aria-label="詳細を閉じる"]')?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector(".global-search-clear")?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `[...document.querySelectorAll('.workspace-mode-switcher [role="tab"]')]
        .find((button) => button.textContent === "カレンダー")?.click()`,
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
        block.getBoundingClientRect().height >= 100 &&
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
    `Boolean(document.querySelector(".task-create-dialog"))`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector(".task-create-dialog-heading .inline-icon-button")?.click()`,
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
      expression: `document.querySelector('button.nav-item[aria-label="タスク"]')?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `[...document.querySelectorAll('.workspace-mode-switcher [role="tab"]')]
        .find((button) => button.textContent === "リスト")?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(document.querySelector('.workspace-mode-switcher [role="tab"]'))`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `[...document.querySelectorAll('.workspace-mode-switcher [role="tab"]')]
        .find((button) => button.textContent === "かんばん")?.click()`,
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
      const handles = [...document.querySelectorAll(".kanban-column-drag-handle")];
      return Boolean(
        board &&
        columns.length === 2 &&
        cards.length === 3 &&
        handles.length === 2 &&
        document.querySelectorAll(".kanban-column-add-task").length === 2 &&
        document.querySelectorAll(".kanban-column-add-slot").length === 1 &&
        document.querySelector(".kanban-column-add-trigger") &&
        document.querySelectorAll(".kanban-column-menu-trigger").length === 2 &&
        document.querySelector(".kanban-card .task-check-button") &&
        !document.querySelector(".kanban-card-actions") &&
        handles.every((handle, index) =>
          Math.abs(
            handle.getBoundingClientRect().width -
            columns[index].getBoundingClientRect().width
          ) <= 2
        ) &&
        columns.every((column) => column.scrollWidth <= column.clientWidth + 1) &&
        cards.every((card) => card.scrollWidth <= card.clientWidth + 1)
      );
    })()`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector(".kanban-column-menu-trigger")?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `(() => {
      const menu = document.querySelector(".kanban-column-menu");
      const labels = [...(menu?.querySelectorAll("button") ?? [])]
        .map((button) => button.textContent?.trim());
      return Boolean(
        menu &&
        labels.includes("タイトルを編集") &&
        labels.some((label) => label?.startsWith("完了タスクを全件削除")) &&
        labels.includes("既定順") &&
        labels.includes("期限が近い順") &&
        labels.includes("作成日の新しい順") &&
        labels.includes("タイトル順") &&
        labels.includes("状態を削除")
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
      expression: `document.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true })
      )`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `!document.querySelector(".kanban-column-menu")`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelectorAll(".kanban-column-add-task")[1]?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `document.querySelector("#task-create-dialog-source")?.textContent === "状態: 進行中"`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector('.task-create-dialog-heading .inline-icon-button')?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `!document.querySelector('.task-create-dialog')`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector('button.nav-item[aria-label="タスク"]')?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `[...document.querySelectorAll('.workspace-mode-switcher [role="tab"]')]
        .find((button) => button.textContent === "リスト")?.click()`,
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
      expression: `document.querySelector('.task-panel .task-add-button')?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `Boolean(
      document.querySelector('.task-create-dialog') &&
      document.activeElement === document.querySelector(
        '.task-create-dialog input[placeholder="例: 週次レビュー"]'
      )
    )`,
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `(() => {
        const dialog = document.querySelector('.task-create-dialog');
        const setValue = (input, value) => {
          if (!(input instanceof HTMLInputElement) &&
              !(input instanceof HTMLTextAreaElement)) return;
          const prototype = input instanceof HTMLTextAreaElement
            ? HTMLTextAreaElement.prototype
            : HTMLInputElement.prototype;
          Object.getOwnPropertyDescriptor(prototype, 'value')?.set?.call(input, value);
          input.dispatchEvent(new Event('input', { bubbles: true }));
        };
        const field = (label) => [...(dialog?.querySelectorAll('label') ?? [])]
          .find((element) => element.querySelector(':scope > span')?.textContent === label)
          ?.querySelector('input, textarea');
        setValue(field('タスク名'), '次回スプリントを計画');
        setValue(field('期限日'), '2026-07-15');
        setValue(field('期限時刻'), '17:00');
        setValue(field('メモ'), '優先順位と担当を整理して、チームへ共有する。');
      })()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `document.querySelector(
      '.task-create-dialog input[placeholder="例: 週次レビュー"]'
    )?.value === '次回スプリントを計画'`,
  );
  await sleep(200);
  const taskCreateScreenshot = await client.send(
    "Page.captureScreenshot",
    { format: "png", fromSurface: true, captureBeyondViewport: false },
    sessionId,
  );
  await writeFile(
    taskCreateOutputPath,
    Buffer.from(taskCreateScreenshot.data, "base64"),
  );
  await client.send(
    "Runtime.evaluate",
    {
      expression: `document.querySelector('.task-create-dialog-heading .inline-icon-button')?.click()`,
      awaitPromise: true,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `!document.querySelector('.task-create-dialog')`,
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
  await client.send(
    "Emulation.setDeviceMetricsOverride",
    {
      width: 1024,
      height: 768,
      deviceScaleFactor: 1,
      mobile: false,
    },
    sessionId,
  );
  await waitForExpression(
    client,
    sessionId,
    `(() => {
      const shell = document.querySelector(".app-shell");
      const navigation = document.querySelector(".left-navigation");
      return Boolean(
        shell &&
        navigation &&
        document.documentElement.scrollWidth <= window.innerWidth + 1 &&
        navigation.scrollWidth <= navigation.clientWidth + 1
      );
    })()`,
  );
  await client.close();
  console.log(`README screenshot written: ${path.relative(repoRoot, outputPath)}`);
  console.log(
    `README screenshot written: ${path.relative(repoRoot, kanbanOutputPath)}`,
  );
  console.log(
    `README screenshot written: ${path.relative(repoRoot, taskCreateOutputPath)}`,
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
      colorToken: "blue",
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
      colorToken: null,
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
      colorToken: null,
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

  function taskPage(request = {}) {
    const scope = request.scope ?? { type: "board" };
    const scopedTasks = tasks.filter((task) => {
      if (scope.type === "list") {
        return task.listId === scope.listId;
      }
      if (scope.type === "today") {
        return task.plannedStartDate === request.todayDate ||
          task.dueDate === request.todayDate ||
          task.subtasks.some((subtask) =>
            subtask.plannedStartDate === request.todayDate ||
            subtask.dueDate === request.todayDate
          );
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
          (task.plannedStartDate === request.todayDate ||
            task.dueDate === request.todayDate ||
            task.subtasks.some((subtask) =>
              subtask.plannedStartDate === request.todayDate ||
              subtask.dueDate === request.todayDate
            ))
        ).length,
        favoriteCount: tasks.filter((task) => task.isFavorite).length
      }
    };
  }

  window.__TAURI_INTERNALS__ = {
    invoke(command, args = {}) {
      const rangeStart =
        args.request?.startDate ?? args.startDate ?? args.weekStartDate ?? "2026-07-06";
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
          colorToken: "blue",
          listColorToken: "green"
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
          colorToken: "blue",
          listColorToken: "green"
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
          colorToken: "blue",
          listColorToken: "green"
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
          colorToken: "green",
          listColorToken: "green"
        }
      ];
      const commands = {
        health_check: () => "tauri-ready",
        list_tasks: () => clone(tasks),
        list_task_page: () => taskPage(args.request),
        get_task_detail: () =>
          clone(tasks.find((task) => task.id === args.taskId) ?? tasks[0]),
        search_work_items: () => [{
          target: { type: "task", id: "task-weekly-review" },
          taskId: "task-weekly-review",
          title: "週次レビュー資料を作成",
          parentTitle: null,
          listId: "default",
          listName: "タスク",
          status: "in_progress",
          dueDate: "2026-07-10",
          dueTime: "16:00",
          tags: [{ id: "tag-priority", name: "重要" }]
        }],
        list_task_lists: () => clone(taskLists),
        list_board_columns: () => clone(boardColumns),
        list_tags: () => clone(tags),
        list_task_rows: () => clone(taskRows),
        list_calendar_items: () => clone(calendarItems),
        list_week_calendar_items: () => clone(calendarItems),
        get_active_timer: () => clone(activeTimer),
        sync_expired_task_countdown: () => ({
          expiredTimer: null,
          notificationSummary: {
            attempted: 0,
            succeeded: 0,
            failed: 0,
            lastError: null
          }
        }),
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
        get_task_timer_settings: () => ({
          id: "default",
          defaultTargetSeconds: 1800,
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
