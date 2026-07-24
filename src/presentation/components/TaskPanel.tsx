import { useEffect, useMemo, useRef, useState } from "react";
import { EllipsisVertical, Pause, Play, Square } from "lucide-react";
import type {
  TaskListItem,
  TaskRow,
  TaskWithSubtasks,
  WorkItemUpdateDraft,
} from "../../application/usecases/contracts";
import type { ActivePomodoro } from "../../application/usecases/contracts";
import type { ActiveTimer } from "../../domain/timer/types";
import type { Subtask, Task, WorkTargetRef } from "../../domain/task/types";
import { usePresentationRenderProbe } from "../renderProbe";

type TaskPanelProps = {
  tasks: TaskWithSubtasks[];
  taskRows: TaskRow[];
  taskLists: TaskListItem[];
  selectedTaskId: string | null;
  selectedSubtaskId: string | null;
  activeTimer: ActiveTimer | null;
  activePomodoro: ActivePomodoro | null;
  eyebrow?: string;
  title?: string;
  emptyMessage?: string;
  showTaskAdd?: boolean;
  isLoading: boolean;
  isMutating: boolean;
  isCreatingTaskPending: boolean;
  isLoadingMore: boolean;
  totalTaskCount: number;
  hasMoreTasks: boolean;
  pendingTaskActionIds: ReadonlySet<string>;
  onSelectTask(taskId: string): void;
  onSelectSubtask(taskId: string, subtaskId: string): void;
  onRequestCreateTask(): void;
  onToggleTaskCompletion(task: TaskWithSubtasks): Promise<boolean>;
  onToggleTaskFavorite(taskId: string, isFavorite: boolean): Promise<boolean>;
  onUpdateTask(taskId: string, input: WorkItemUpdateDraft): Promise<boolean>;
  onRequestCreateSubtask(taskId: string): void;
  onDeleteTask(task: TaskWithSubtasks): Promise<boolean>;
  onReorderTask(taskId: string, direction: "up" | "down"): Promise<boolean>;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onPauseTimer(): Promise<boolean>;
  onResumeTimer(): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
  onLoadMoreTasks(): Promise<void>;
};

const statusLabels: Record<Task["status"], string> = {
  todo: "未着手",
  in_progress: "進行中",
  done: "完了",
  archived: "アーカイブ",
};

export function TaskPanel({
  tasks,
  taskRows,
  taskLists,
  selectedTaskId,
  selectedSubtaskId,
  activeTimer,
  activePomodoro,
  eyebrow = "DB接続済みタスク",
  title = "タスク",
  emptyMessage = "まだタスクはありません。",
  showTaskAdd = true,
  isLoading,
  isMutating,
  isCreatingTaskPending,
  isLoadingMore,
  totalTaskCount,
  hasMoreTasks,
  pendingTaskActionIds,
  onSelectTask,
  onSelectSubtask,
  onRequestCreateTask,
  onToggleTaskCompletion,
  onToggleTaskFavorite,
  onUpdateTask,
  onRequestCreateSubtask,
  onDeleteTask,
  onReorderTask,
  onStartTimer,
  onPauseTimer,
  onResumeTimer,
  onStopTimer,
  onLoadMoreTasks,
}: TaskPanelProps) {
  usePresentationRenderProbe("TaskPanel");
  const [isCompletedOpen, setIsCompletedOpen] = useState(true);
  const [expandedTaskIds, setExpandedTaskIds] = useState<ReadonlySet<string>>(
    new Set(),
  );
  const taskById = useMemo(
    () => new Map(tasks.map((task) => [task.id, task])),
    [tasks],
  );
  const incompleteRows = taskRows.filter((task) => task.status !== "done");
  const completedRows = taskRows.filter((task) => task.status === "done");
  const isCreateDisabled = isMutating || isCreatingTaskPending;

  function handleCompleteRow(row: TaskRow) {
    const task = taskById.get(row.id);
    if (!task) {
      return;
    }
    void onToggleTaskCompletion(task);
  }

  function toggleTaskExpansion(taskId: string) {
    setExpandedTaskIds((current) => {
      const next = new Set(current);
      if (next.has(taskId)) {
        next.delete(taskId);
      } else {
        next.add(taskId);
      }
      return next;
    });
  }

  return (
    <section className="panel task-panel" aria-labelledby="task-panel-title">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">{eyebrow}</p>
          <h2 id="task-panel-title">{title}</h2>
        </div>
        <div className="panel-heading-actions">
          <span
            className="task-count-badge"
            aria-label={`タスク総件数 ${totalTaskCount}件`}
            title={`読み込み済み ${taskRows.length}件`}
          >
            {totalTaskCount}
          </span>
          {showTaskAdd ? (
            <button
              className="task-add-button"
              type="button"
              data-task-create-trigger
              aria-label="タスクを追加"
              title="タスクを追加"
              disabled={isCreateDisabled}
              onClick={(event) => {
                event.currentTarget.focus();
                onRequestCreateTask();
              }}
            >
              ＋
            </button>
          ) : null}
        </div>
      </div>

      <div className="task-board" aria-label="タスク一覧">
        {isLoading ? <p className="empty-state">タスクを読み込み中です。</p> : null}
        {!isLoading && taskRows.length === 0 ? (
          <p className="empty-state">{emptyMessage}</p>
        ) : null}

        {incompleteRows.map((row) => (
          <TaskRowItem
            key={row.id}
            row={row}
            task={taskById.get(row.id) ?? null}
            taskLists={taskLists}
            isSelected={row.id === selectedTaskId}
            selectedSubtaskId={selectedSubtaskId}
            isExpanded={expandedTaskIds.has(row.id)}
            isMutating={isMutating || pendingTaskActionIds.has(row.id)}
            onSelectTask={onSelectTask}
            onSelectSubtask={onSelectSubtask}
            onToggleExpansion={toggleTaskExpansion}
            onToggleTaskCompletion={handleCompleteRow}
            onToggleTaskFavorite={onToggleTaskFavorite}
            onUpdateTask={onUpdateTask}
            onRequestCreateSubtask={onRequestCreateSubtask}
            onDeleteTask={onDeleteTask}
            onReorderTask={onReorderTask}
            activeTimer={activeTimer}
            activePomodoro={activePomodoro}
            onStartTimer={onStartTimer}
            onPauseTimer={onPauseTimer}
            onResumeTimer={onResumeTimer}
            onStopTimer={onStopTimer}
          />
        ))}

        {completedRows.length > 0 ? (
          <section className="completed-task-section" aria-label="完了タスク">
            <button
              className="completed-toggle"
              type="button"
              aria-expanded={isCompletedOpen}
              onClick={() => setIsCompletedOpen((current) => !current)}
            >
              <span>{isCompletedOpen ? "⌄" : "›"}</span>
              完了
              <strong>{completedRows.length}</strong>
            </button>
            {isCompletedOpen ? (
              <div className="completed-task-list">
                {completedRows.map((row) => (
                  <TaskRowItem
                    key={row.id}
                    row={row}
                    task={taskById.get(row.id) ?? null}
                    taskLists={taskLists}
                    isSelected={row.id === selectedTaskId}
                    selectedSubtaskId={selectedSubtaskId}
                    isExpanded={expandedTaskIds.has(row.id)}
                    isMutating={isMutating || pendingTaskActionIds.has(row.id)}
                    onSelectTask={onSelectTask}
                    onSelectSubtask={onSelectSubtask}
                    onToggleExpansion={toggleTaskExpansion}
                    onToggleTaskCompletion={handleCompleteRow}
                    onToggleTaskFavorite={onToggleTaskFavorite}
                    onUpdateTask={onUpdateTask}
                    onRequestCreateSubtask={onRequestCreateSubtask}
                    onDeleteTask={onDeleteTask}
                    onReorderTask={onReorderTask}
                    activeTimer={activeTimer}
                    activePomodoro={activePomodoro}
                    onStartTimer={onStartTimer}
                    onPauseTimer={onPauseTimer}
                    onResumeTimer={onResumeTimer}
                    onStopTimer={onStopTimer}
                  />
                ))}
              </div>
            ) : null}
          </section>
        ) : null}

        {hasMoreTasks ? (
          <button
            className="secondary-button task-load-more"
            type="button"
            disabled={isMutating || isLoadingMore}
            onClick={() => void onLoadMoreTasks()}
          >
            {isLoadingMore ? "読み込み中..." : "さらに読み込む"}
            <span>
              {taskRows.length} / {totalTaskCount}
            </span>
          </button>
        ) : null}
      </div>
    </section>
  );
}

type TaskRowItemProps = {
  row: TaskRow;
  task: TaskWithSubtasks | null;
  taskLists: TaskListItem[];
  isSelected: boolean;
  selectedSubtaskId: string | null;
  isExpanded: boolean;
  isMutating: boolean;
  activeTimer: ActiveTimer | null;
  activePomodoro: ActivePomodoro | null;
  onSelectTask(taskId: string): void;
  onSelectSubtask(taskId: string, subtaskId: string): void;
  onToggleExpansion(taskId: string): void;
  onToggleTaskCompletion(row: TaskRow): void;
  onToggleTaskFavorite(taskId: string, isFavorite: boolean): Promise<boolean>;
  onUpdateTask(taskId: string, input: WorkItemUpdateDraft): Promise<boolean>;
  onRequestCreateSubtask(taskId: string): void;
  onDeleteTask(task: TaskWithSubtasks): Promise<boolean>;
  onReorderTask(taskId: string, direction: "up" | "down"): Promise<boolean>;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onPauseTimer(): Promise<boolean>;
  onResumeTimer(): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
};

function TaskRowItem({
  row,
  task,
  taskLists,
  isSelected,
  selectedSubtaskId,
  isExpanded,
  isMutating,
  activeTimer,
  activePomodoro,
  onSelectTask,
  onSelectSubtask,
  onToggleExpansion,
  onToggleTaskCompletion,
  onToggleTaskFavorite,
  onUpdateTask,
  onRequestCreateSubtask,
  onDeleteTask,
  onReorderTask,
  onStartTimer,
  onPauseTimer,
  onResumeTimer,
  onStopTimer,
}: TaskRowItemProps) {
  const hasProgress = row.subtaskTotalCount > 0;
  const subtasks = task?.subtasks ?? [];
  const [menuMode, setMenuMode] = useState<"actions" | "due" | "list" | null>(
    null,
  );
  const [dueDraft, setDueDraft] = useState({
    dueDate: row.dueDate ?? "",
    dueTime: row.dueTime ?? "",
  });
  const menuAnchorRef = useRef<HTMLSpanElement | null>(null);
  const progressPercent = hasProgress
    ? Math.round((row.completedSubtaskCount / row.subtaskTotalCount) * 100)
    : 0;
  const isDone = row.status === "done";
  const memoPreview = formatMemoPreview(task?.memo ?? "");
  const canEditTask = task !== null && !isMutating;

  useEffect(() => {
    if (menuMode && isMutating) {
      setMenuMode(null);
    }
  }, [isMutating, menuMode]);

  useEffect(() => {
    if (!menuMode) {
      return;
    }
    function handlePointerDown(event: PointerEvent) {
      if (
        event.target instanceof Node &&
        !menuAnchorRef.current?.contains(event.target)
      ) {
        setMenuMode(null);
      }
    }
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setMenuMode(null);
      }
    }
    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [menuMode]);

  async function applyDue(dueDate: string | null, dueTime: string | null) {
    if (!task) {
      return;
    }
    const saved = await onUpdateTask(
      task.id,
      toTaskUpdateDraft(task, {
        dueDate,
        dueTime: dueDate ? dueTime : null,
      }),
    );
    if (saved) {
      setMenuMode(null);
    }
  }

  async function applyListChange(nextListId: string) {
    if (!task || nextListId === task.listId) {
      setMenuMode(null);
      return;
    }
    const saved = await onUpdateTask(
      task.id,
      toTaskUpdateDraft(task, { listId: nextListId }),
    );
    if (saved) {
      setMenuMode(null);
    }
  }

  return (
    <div className="task-row-group">
      <div
        className={`task-row ${isSelected && !selectedSubtaskId ? "is-selected" : ""} ${
          isDone ? "is-done" : ""
        }`}
      >
        <button
          className="task-check-button"
          type="button"
          aria-label={isDone ? `${row.title}を未完了に戻す` : `${row.title}を完了`}
          title={isDone ? "未完了に戻す" : "完了"}
          disabled={isMutating}
          onClick={() => onToggleTaskCompletion(row)}
        >
          {isDone ? "✓" : ""}
        </button>

        {hasProgress ? (
          <button
            className="subtask-expand-button"
            type="button"
            aria-label={`${row.title}のサブタスクを${isExpanded ? "閉じる" : "開く"}`}
            aria-expanded={isExpanded}
            title={isExpanded ? "サブタスクを閉じる" : "サブタスクを開く"}
            onClick={() => onToggleExpansion(row.id)}
          >
            {isExpanded ? "⌄" : "›"}
          </button>
        ) : (
          <span className="subtask-expand-spacer" aria-hidden="true" />
        )}

        <button
          className="task-row-content"
          type="button"
          onClick={() => onSelectTask(row.id)}
        >
          <span className="task-row-title">{row.title}</span>
          <span className="task-row-meta">
            <span>{statusLabels[row.status]}</span>
            {row.plannedStartDate ? (
              <span className="task-start-label" title="開始予定あり">
                開始 {formatDateLabel(row.plannedStartDate)}
              </span>
            ) : null}
            {row.dueDate ? (
              <span className="task-due-label" title="期限あり">
                期限 {formatDateLabel(row.dueDate)}
                {row.dueTime ? ` ${row.dueTime}` : ""}
              </span>
            ) : null}
            {row.isTimerActive ? <span>実行中</span> : null}
          </span>
          {memoPreview ? (
            <span className="task-row-memo">{memoPreview}</span>
          ) : null}
          {row.tags.length > 0 ? (
            <span className="task-row-tags" aria-label="タグ">
              {row.tags.map((tag) => (
                <span className="task-tag-chip" key={tag.id}>
                  {tag.name}
                </span>
              ))}
            </span>
          ) : null}
          {hasProgress ? (
            <span className="task-progress">
              <span className="task-progress-bar">
                <span style={{ width: `${progressPercent}%` }} />
              </span>
              <span className="task-progress-label">
                {row.completedSubtaskCount}/{row.subtaskTotalCount}
              </span>
            </span>
          ) : null}
        </button>

        <TaskTimerControl
          target={{ type: "task", id: row.id }}
          label={row.title}
          status={row.status}
          activeTimer={activeTimer}
          activePomodoro={activePomodoro}
          isMutating={isMutating}
          onStartTimer={onStartTimer}
          onPauseTimer={onPauseTimer}
          onResumeTimer={onResumeTimer}
          onStopTimer={onStopTimer}
        />

        <button
          className={`favorite-button ${row.isFavorite ? "is-favorite" : ""}`}
          type="button"
          aria-label={
            row.isFavorite
              ? `${row.title}のお気に入りを解除`
              : `${row.title}をお気に入りに追加`
          }
          aria-pressed={row.isFavorite}
          title={row.isFavorite ? "お気に入りを解除" : "お気に入り"}
          disabled={isMutating}
          onClick={() => void onToggleTaskFavorite(row.id, !row.isFavorite)}
        >
          {row.isFavorite ? "★" : "☆"}
        </button>

        <span className="task-row-menu-anchor" ref={menuAnchorRef}>
          <button
            className="task-row-menu-trigger"
            type="button"
            aria-label={`${row.title}の操作`}
            aria-haspopup="menu"
            aria-expanded={menuMode !== null}
            title="タスクの操作"
            disabled={isMutating}
            onClick={(event) => {
              event.stopPropagation();
              setMenuMode((current) => {
                if (current) {
                  return null;
                }
                setDueDraft({
                  dueDate: row.dueDate ?? "",
                  dueTime: row.dueTime ?? "",
                });
                return "actions";
              });
            }}
          >
            <EllipsisVertical aria-hidden="true" size={17} />
          </button>
          {menuMode ? (
            <div
              className="task-row-menu"
              role={menuMode === "actions" ? "menu" : "dialog"}
              aria-label={`${row.title}の操作`}
              onClick={(event) => event.stopPropagation()}
            >
              {menuMode === "actions" ? (
                <>
                  <button
                    type="button"
                    role="menuitem"
                    disabled={!canEditTask}
                    onClick={() => setMenuMode("due")}
                  >
                    期限の設定
                  </button>
                  <button
                    type="button"
                    role="menuitem"
                    disabled={!canEditTask}
                    onClick={() => {
                      setMenuMode(null);
                      onRequestCreateSubtask(row.id);
                      if (!isExpanded) {
                        onToggleExpansion(row.id);
                      }
                    }}
                  >
                    サブタスク追加
                  </button>
                  <button
                    type="button"
                    role="menuitem"
                    disabled={!canEditTask}
                    onClick={() => setMenuMode("list")}
                  >
                    リスト選択
                  </button>
                  <button
                    type="button"
                    role="menuitem"
                    disabled={isMutating}
                    onClick={() =>
                      void onReorderTask(row.id, "up").then((moved) => {
                        if (moved) {
                          setMenuMode(null);
                        }
                      })
                    }
                  >
                    上に移動
                  </button>
                  <button
                    type="button"
                    role="menuitem"
                    disabled={isMutating}
                    onClick={() =>
                      void onReorderTask(row.id, "down").then((moved) => {
                        if (moved) {
                          setMenuMode(null);
                        }
                      })
                    }
                  >
                    下に移動
                  </button>
                  <button
                    className="is-danger"
                    type="button"
                    role="menuitem"
                    disabled={!canEditTask}
                    onClick={() => {
                      if (
                        task &&
                        window.confirm(`「${row.title}」を削除しますか？`)
                      ) {
                        void onDeleteTask(task).then((deleted) => {
                          if (deleted) {
                            setMenuMode(null);
                          }
                        });
                      }
                    }}
                  >
                    削除
                  </button>
                </>
              ) : null}

              {menuMode === "due" ? (
                <form
                  className="task-row-menu-form"
                  onSubmit={(event) => {
                    event.preventDefault();
                    void applyDue(
                      dueDraft.dueDate.trim() || null,
                      dueDraft.dueDate.trim() ? dueDraft.dueTime || null : null,
                    );
                  }}
                >
                  <label>
                    <span>期限日</span>
                    <input
                      type="date"
                      value={dueDraft.dueDate}
                      onChange={(event) =>
                        setDueDraft((current) => ({
                          ...current,
                          dueDate: event.target.value,
                        }))
                      }
                    />
                  </label>
                  <label>
                    <span>期限時刻</span>
                    <input
                      type="time"
                      value={dueDraft.dueTime}
                      disabled={!dueDraft.dueDate}
                      onChange={(event) =>
                        setDueDraft((current) => ({
                          ...current,
                          dueTime: event.target.value,
                        }))
                      }
                    />
                  </label>
                  <div className="task-row-menu-actions">
                    <button type="submit" disabled={!canEditTask}>
                      保存
                    </button>
                    <button
                      type="button"
                      disabled={!canEditTask}
                      onClick={() => void applyDue(null, null)}
                    >
                      期限なし
                    </button>
                  </div>
                </form>
              ) : null}

              {menuMode === "list" ? (
                <div className="task-row-list-options" aria-label="所属リスト">
                  {taskLists.map((list) => (
                    <button
                      className={`task-row-list-option ${
                        row.listId === list.id ? "is-selected" : ""
                      }`}
                      type="button"
                      key={list.id}
                      aria-pressed={row.listId === list.id}
                      disabled={!canEditTask}
                      onClick={() => void applyListChange(list.id)}
                    >
                      <span
                        className={`task-row-list-dot color-${list.colorToken}`}
                        aria-hidden="true"
                      />
                      <span>{list.name}</span>
                    </button>
                  ))}
                </div>
              ) : null}
            </div>
          ) : null}
        </span>
      </div>

      {isExpanded && subtasks.length > 0 ? (
        <div className="task-row-subtasks" aria-label={`${row.title}のサブタスク`}>
          {subtasks.map((subtask) => (
            <SubtaskRowButton
              key={subtask.id}
              subtask={subtask}
              isSelected={subtask.id === selectedSubtaskId}
              onSelect={() => onSelectSubtask(row.id, subtask.id)}
              activeTimer={activeTimer}
              activePomodoro={activePomodoro}
              isMutating={isMutating}
              onStartTimer={onStartTimer}
              onPauseTimer={onPauseTimer}
              onResumeTimer={onResumeTimer}
              onStopTimer={onStopTimer}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

type SubtaskRowButtonProps = {
  subtask: Subtask;
  isSelected: boolean;
  onSelect(): void;
  activeTimer: ActiveTimer | null;
  activePomodoro: ActivePomodoro | null;
  isMutating: boolean;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onPauseTimer(): Promise<boolean>;
  onResumeTimer(): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
};

function SubtaskRowButton({
  subtask,
  isSelected,
  onSelect,
  activeTimer,
  activePomodoro,
  isMutating,
  onStartTimer,
  onPauseTimer,
  onResumeTimer,
  onStopTimer,
}: SubtaskRowButtonProps) {
  return (
    <div
      className={`task-row-subtask ${isSelected ? "is-selected" : ""} ${
        subtask.status === "done" ? "is-done" : ""
      }`}
    >
      <button className="subtask-row-content" type="button" onClick={onSelect}>
        <span className="subtask-dot" aria-hidden="true" />
        <span>
          <strong>{subtask.title}</strong>
          <small>
            {statusLabels[subtask.status]}
            {subtask.dueDate ? ` / 期限 ${formatDateLabel(subtask.dueDate)}` : ""}
            {subtask.dueTime ? ` ${subtask.dueTime}` : ""}
          </small>
        </span>
      </button>
      <TaskTimerControl
        target={{ type: "subtask", id: subtask.id }}
        label={subtask.title}
        status={subtask.status}
        activeTimer={activeTimer}
        activePomodoro={activePomodoro}
        isMutating={isMutating}
        onStartTimer={onStartTimer}
        onPauseTimer={onPauseTimer}
        onResumeTimer={onResumeTimer}
        onStopTimer={onStopTimer}
      />
    </div>
  );
}

type TaskTimerControlProps = {
  target: WorkTargetRef;
  label: string;
  status: Task["status"];
  activeTimer: ActiveTimer | null;
  activePomodoro: ActivePomodoro | null;
  isMutating: boolean;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onPauseTimer(): Promise<boolean>;
  onResumeTimer(): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
};

function TaskTimerControl({
  target,
  label,
  status,
  activeTimer,
  activePomodoro,
  isMutating,
  onStartTimer,
  onPauseTimer,
  onResumeTimer,
  onStopTimer,
}: TaskTimerControlProps) {
  const isActive = isSameTarget(activeTimer?.target ?? null, target);
  const isPaused = isActive && Boolean(activeTimer?.pausedAt);
  const [now, setNow] = useState(Date.now);

  useEffect(() => {
    setNow(Date.now());
    if (!isActive || isPaused || !activeTimer?.targetSeconds) {
      return;
    }
    const intervalId = window.setInterval(() => setNow(Date.now()), 1_000);
    return () => window.clearInterval(intervalId);
  }, [activeTimer, isActive, isPaused]);

  const canStart =
    !activeTimer &&
    !activePomodoro &&
    status !== "done" &&
    status !== "archived" &&
    !isMutating;

  if (activePomodoro) {
    return (
      <span className="task-timer-slot">
        <button
          className="task-timer-icon-button"
          type="button"
          aria-label={`${label}のタイマーを開始`}
          title="ポモドーロが実行中です"
          disabled
        >
          <Play size={16} />
        </button>
      </span>
    );
  }

  if (isActive && activeTimer) {
    return (
      <span className="task-timer-slot is-active">
        <span className="task-countdown-value">
          {formatCountdownRemaining(activeTimer, now)}
        </span>
        <button
          className="task-timer-icon-button"
          type="button"
          aria-label={isPaused ? `${label}のタイマーを再開` : `${label}のタイマーを一時停止`}
          title={isPaused ? "再開" : "一時停止"}
          disabled={isMutating}
          onClick={() => void (isPaused ? onResumeTimer() : onPauseTimer())}
        >
          {isPaused ? <Play size={15} /> : <Pause size={15} />}
        </button>
        <button
          className="task-timer-icon-button"
          type="button"
          aria-label={`${label}のタイマーを終了`}
          title="終了"
          disabled={isMutating}
          onClick={() => void onStopTimer()}
        >
          <Square size={14} />
        </button>
      </span>
    );
  }

  return (
    <span className="task-timer-slot">
      <button
        className="task-timer-icon-button"
        type="button"
        aria-label={`${label}のタイマーを開始`}
        title={
          activeTimer || activePomodoro
            ? "他のタイマーまたはポモドーロが実行中です"
            : "タイマーを開始"
        }
        disabled={!canStart}
        onClick={() => void onStartTimer(target)}
      >
        <Play size={16} />
      </button>
    </span>
  );
}

function isSameTarget(current: WorkTargetRef | null, expected: WorkTargetRef) {
  return current?.type === expected.type && current.id === expected.id;
}

function formatCountdownRemaining(activeTimer: ActiveTimer, now: number) {
  if (!activeTimer.targetSeconds) {
    return "計測中";
  }
  const startedAt = new Date(activeTimer.startedAt).getTime();
  const effectiveNow = activeTimer.pausedAt
    ? new Date(activeTimer.pausedAt).getTime()
    : now;
  if (Number.isNaN(startedAt) || Number.isNaN(effectiveNow)) {
    return "--:--";
  }
  const elapsedSeconds = Math.max(
    0,
    Math.floor((effectiveNow - startedAt) / 1_000) -
      activeTimer.accumulatedPausedSeconds,
  );
  const remainingSeconds = Math.max(0, activeTimer.targetSeconds - elapsedSeconds);
  const hours = Math.floor(remainingSeconds / 3_600);
  const minutes = Math.floor((remainingSeconds % 3_600) / 60);
  const seconds = remainingSeconds % 60;
  return hours > 0
    ? `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`
    : `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

function formatDateLabel(value: string | null) {
  if (!value) {
    return "未設定";
  }
  const [, month, day] = value.split("-");
  return `${Number(month)}/${Number(day)}`;
}

function formatMemoPreview(value: string) {
  const normalized = value.trim().replace(/\s+/g, " ");
  if (!normalized) {
    return "";
  }
  return normalized.length > 48 ? `${normalized.slice(0, 48)}...` : normalized;
}

function toTaskUpdateDraft(
  task: TaskWithSubtasks,
  overrides: Partial<WorkItemUpdateDraft>,
): WorkItemUpdateDraft {
  return {
    listId: task.listId,
    title: task.title,
    plannedStartDate: task.plannedStartDate,
    dueDate: task.dueDate,
    dueTime: task.dueTime,
    timerTargetSeconds: task.timerTargetSeconds,
    colorToken: task.colorToken,
    recurrenceRule: task.recurrenceRule
      ? {
          frequency: task.recurrenceRule.frequency,
          interval: task.recurrenceRule.interval,
        }
      : null,
    memo: task.memo,
    ...overrides,
  };
}
