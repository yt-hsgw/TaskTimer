import { FormEvent, useEffect, useMemo, useRef, useState } from "react";
import type {
  TaskRow,
  TaskWithSubtasks,
  WorkItemDraft,
} from "../../application/usecases/contracts";
import type { ActiveTimer } from "../../domain/timer/types";
import type { Subtask, Task, WorkTargetRef } from "../../domain/task/types";

type TaskPanelProps = {
  tasks: TaskWithSubtasks[];
  taskRows: TaskRow[];
  selectedTaskId: string | null;
  activeTimer: ActiveTimer | null;
  eyebrow?: string;
  title?: string;
  emptyMessage?: string;
  showTaskForm?: boolean;
  isLoading: boolean;
  isMutating: boolean;
  onSelectTask(taskId: string): void;
  onCreateTask(input: WorkItemDraft): Promise<boolean>;
  onCreateSubtask(taskId: string, input: WorkItemDraft): Promise<boolean>;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
  onCompleteTask(task: TaskWithSubtasks): Promise<boolean>;
  onCompleteSubtask(subtask: Subtask): Promise<boolean>;
  onToggleTaskFavorite(taskId: string, isFavorite: boolean): Promise<boolean>;
  onDeleteTask(task: TaskWithSubtasks): Promise<boolean>;
  onDeleteSubtask(subtask: Subtask): Promise<boolean>;
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
  selectedTaskId,
  activeTimer,
  eyebrow = "DB接続済みタスク",
  title = "タスク",
  emptyMessage = "まだタスクはありません。",
  showTaskForm = true,
  isLoading,
  isMutating,
  onSelectTask,
  onCreateTask,
  onCreateSubtask,
  onStartTimer,
  onStopTimer,
  onCompleteTask,
  onCompleteSubtask,
  onToggleTaskFavorite,
  onDeleteTask,
  onDeleteSubtask,
}: TaskPanelProps) {
  const [isCreatingTask, setIsCreatingTask] = useState(false);
  const [isCompletedOpen, setIsCompletedOpen] = useState(true);
  const [taskDraft, setTaskDraft] = useState<WorkItemDraft>({
    title: "",
    plannedStartDate: "",
    dueDate: "",
    memo: "",
  });
  const [subtaskDraft, setSubtaskDraft] = useState<WorkItemDraft>({
    title: "",
    plannedStartDate: "",
    dueDate: "",
    memo: "",
  });
  const taskTitleInputRef = useRef<HTMLInputElement>(null);

  const taskById = useMemo(
    () => new Map(tasks.map((task) => [task.id, task])),
    [tasks],
  );
  const selectedTask = selectedTaskId ? taskById.get(selectedTaskId) ?? null : null;
  const incompleteRows = taskRows.filter((task) => task.status !== "done");
  const completedRows = taskRows.filter((task) => task.status === "done");

  useEffect(() => {
    if (!showTaskForm) {
      return;
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.ctrlKey && event.key.toLowerCase() === "n") {
        event.preventDefault();
        setIsCreatingTask(true);
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [showTaskForm]);

  useEffect(() => {
    if (isCreatingTask) {
      taskTitleInputRef.current?.focus();
    }
  }, [isCreatingTask]);

  async function handleCreateTask(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const created = await onCreateTask(normalizeDraft(taskDraft));
    if (created) {
      setTaskDraft({ title: "", plannedStartDate: "", dueDate: "", memo: "" });
      setIsCreatingTask(false);
    }
  }

  async function handleCreateSubtask(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedTask) {
      return;
    }

    const created = await onCreateSubtask(
      selectedTask.id,
      normalizeDraft(subtaskDraft),
    );
    if (created) {
      setSubtaskDraft({
        title: "",
        plannedStartDate: "",
        dueDate: "",
        memo: "",
      });
    }
  }

  function handleCompleteRow(row: TaskRow) {
    const task = taskById.get(row.id);
    if (!task) {
      return;
    }
    void onCompleteTask(task);
  }

  return (
    <section className="panel task-panel" aria-labelledby="task-panel-title">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">{eyebrow}</p>
          <h2 id="task-panel-title">{title}</h2>
        </div>
        <span className="task-count-badge">{taskRows.length}</span>
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
            isSelected={row.id === selectedTask?.id}
            isMutating={isMutating}
            onSelectTask={onSelectTask}
            onCompleteTask={handleCompleteRow}
            onToggleTaskFavorite={onToggleTaskFavorite}
          />
        ))}

        {showTaskForm ? (
          <div className="task-composer">
            {isCreatingTask ? (
              <form
                className="work-form inline-create-form"
                onSubmit={(event) => void handleCreateTask(event)}
              >
                <label>
                  <span>タスク名</span>
                  <input
                    ref={taskTitleInputRef}
                    value={taskDraft.title}
                    onChange={(event) =>
                      setTaskDraft((current) => ({
                        ...current,
                        title: event.target.value,
                      }))
                    }
                    placeholder="例: 週次レビュー"
                    disabled={isMutating}
                    maxLength={120}
                    required
                  />
                </label>
                <div className="date-fields">
                  <label>
                    <span>開始日</span>
                    <input
                      type="date"
                      value={taskDraft.plannedStartDate ?? ""}
                      onChange={(event) =>
                        setTaskDraft((current) => ({
                          ...current,
                          plannedStartDate: event.target.value,
                        }))
                      }
                      disabled={isMutating}
                    />
                  </label>
                  <label>
                    <span>終了日</span>
                    <input
                      type="date"
                      value={taskDraft.dueDate ?? ""}
                      onChange={(event) =>
                        setTaskDraft((current) => ({
                          ...current,
                          dueDate: event.target.value,
                        }))
                      }
                      disabled={isMutating}
                    />
                  </label>
                </div>
                <label>
                  <span>メモ</span>
                  <textarea
                    value={taskDraft.memo ?? ""}
                    onChange={(event) =>
                      setTaskDraft((current) => ({
                        ...current,
                        memo: event.target.value,
                      }))
                    }
                    disabled={isMutating}
                    rows={3}
                  />
                </label>
                <div className="composer-actions">
                  <button
                    className="primary-button"
                    type="submit"
                    disabled={isMutating}
                  >
                    追加
                  </button>
                  <button
                    className="secondary-button"
                    type="button"
                    disabled={isMutating}
                    onClick={() => setIsCreatingTask(false)}
                  >
                    キャンセル
                  </button>
                </div>
              </form>
            ) : (
              <button
                className="task-add-button"
                type="button"
                disabled={isMutating}
                onClick={() => setIsCreatingTask(true)}
              >
                <span aria-hidden="true">＋</span>
                タスクの追加
              </button>
            )}
          </div>
        ) : null}

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
                    isSelected={row.id === selectedTask?.id}
                    isMutating={isMutating}
                    onSelectTask={onSelectTask}
                    onCompleteTask={handleCompleteRow}
                    onToggleTaskFavorite={onToggleTaskFavorite}
                  />
                ))}
              </div>
            ) : null}
          </section>
        ) : null}
      </div>

      {selectedTask ? (
        <TaskDetail
          task={selectedTask}
          activeTimer={activeTimer}
          isMutating={isMutating}
          subtaskDraft={subtaskDraft}
          setSubtaskDraft={setSubtaskDraft}
          onCreateSubtask={handleCreateSubtask}
          onStartTimer={onStartTimer}
          onStopTimer={onStopTimer}
          onCompleteTask={onCompleteTask}
          onCompleteSubtask={onCompleteSubtask}
          onDeleteTask={onDeleteTask}
          onDeleteSubtask={onDeleteSubtask}
        />
      ) : null}
    </section>
  );
}

type TaskRowItemProps = {
  row: TaskRow;
  isSelected: boolean;
  isMutating: boolean;
  onSelectTask(taskId: string): void;
  onCompleteTask(row: TaskRow): void;
  onToggleTaskFavorite(taskId: string, isFavorite: boolean): Promise<boolean>;
};

function TaskRowItem({
  row,
  isSelected,
  isMutating,
  onSelectTask,
  onCompleteTask,
  onToggleTaskFavorite,
}: TaskRowItemProps) {
  const hasProgress = row.subtaskTotalCount > 0;
  const progressPercent = hasProgress
    ? Math.round((row.completedSubtaskCount / row.subtaskTotalCount) * 100)
    : 0;
  const isDone = row.status === "done";

  return (
    <div
      className={`task-row ${isSelected ? "is-selected" : ""} ${
        isDone ? "is-done" : ""
      }`}
    >
      <button
        className="task-check-button"
        type="button"
        aria-label={`${row.title}を完了`}
        title="完了"
        disabled={isMutating || isDone}
        onClick={() => onCompleteTask(row)}
      >
        {isDone ? "✓" : ""}
      </button>

      <button
        className="task-row-content"
        type="button"
        onClick={() => onSelectTask(row.id)}
      >
        <span className="task-row-title">{row.title}</span>
        <span className="task-row-meta">
          <span>{statusLabels[row.status]}</span>
          {row.dueDate ? (
            <span title="期限あり">◇ {formatDateLabel(row.dueDate)}</span>
          ) : null}
          {row.isTimerActive ? <span>実行中</span> : null}
        </span>
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
    </div>
  );
}

type TaskDetailProps = {
  task: TaskWithSubtasks;
  activeTimer: ActiveTimer | null;
  isMutating: boolean;
  subtaskDraft: WorkItemDraft;
  setSubtaskDraft(value: WorkItemDraft | ((current: WorkItemDraft) => WorkItemDraft)): void;
  onCreateSubtask(event: FormEvent<HTMLFormElement>): Promise<void>;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
  onCompleteTask(task: TaskWithSubtasks): Promise<boolean>;
  onCompleteSubtask(subtask: Subtask): Promise<boolean>;
  onDeleteTask(task: TaskWithSubtasks): Promise<boolean>;
  onDeleteSubtask(subtask: Subtask): Promise<boolean>;
};

function TaskDetail({
  task,
  activeTimer,
  isMutating,
  subtaskDraft,
  setSubtaskDraft,
  onCreateSubtask,
  onStartTimer,
  onStopTimer,
  onCompleteTask,
  onCompleteSubtask,
  onDeleteTask,
  onDeleteSubtask,
}: TaskDetailProps) {
  const taskTarget: WorkTargetRef = { type: "task", id: task.id };
  const isTaskActive = isActiveTarget(activeTimer, taskTarget);

  return (
    <div className="task-detail">
      <div className="task-detail-heading">
        <div>
          <p className="eyebrow">選択中</p>
          <h3>{task.title}</h3>
        </div>
        <TimerButton
          target={taskTarget}
          label={task.title}
          status={task.status}
          activeTimer={activeTimer}
          isMutating={isMutating}
          onStartTimer={onStartTimer}
          onStopTimer={onStopTimer}
        />
      </div>

      <div className="action-row">
        <button
          className="secondary-button"
          type="button"
          disabled={isMutating || task.status === "done"}
          onClick={() => void onCompleteTask(task)}
        >
          完了
        </button>
        <button
          className="danger-button"
          type="button"
          disabled={isMutating}
          onClick={() => void onDeleteTask(task)}
        >
          削除
        </button>
      </div>

      <div className="task-meta">
        <span>開始 {formatDateLabel(task.plannedStartDate)}</span>
        <span>終了 {formatDateLabel(task.dueDate)}</span>
        <span>{statusLabels[task.status]}</span>
      </div>

      <label className="check-row">
        <input
          type="checkbox"
          checked={task.status === "done"}
          disabled={isMutating || task.status === "done"}
          onChange={() => void onCompleteTask(task)}
        />
        <span>{isTaskActive ? "親タスクを計測中" : "親タスク"}</span>
      </label>

      {task.memo ? <p className="memo-text">{task.memo}</p> : null}

      <form
        className="work-form compact-form"
        onSubmit={(event) => void onCreateSubtask(event)}
      >
        <label>
          <span>サブタスク名</span>
          <input
            value={subtaskDraft.title}
            onChange={(event) =>
              setSubtaskDraft((current) => ({
                ...current,
                title: event.target.value,
              }))
            }
            placeholder="例: チェック項目を整理"
            disabled={isMutating}
            maxLength={120}
            required
          />
        </label>
        <div className="date-fields">
          <label>
            <span>開始日</span>
            <input
              type="date"
              value={subtaskDraft.plannedStartDate ?? ""}
              onChange={(event) =>
                setSubtaskDraft((current) => ({
                  ...current,
                  plannedStartDate: event.target.value,
                }))
              }
              disabled={isMutating}
            />
          </label>
          <label>
            <span>終了日</span>
            <input
              type="date"
              value={subtaskDraft.dueDate ?? ""}
              onChange={(event) =>
                setSubtaskDraft((current) => ({
                  ...current,
                  dueDate: event.target.value,
                }))
              }
              disabled={isMutating}
            />
          </label>
        </div>
        <button className="secondary-button" type="submit" disabled={isMutating}>
          サブタスク追加
        </button>
      </form>

      <div className="subtask-list" aria-label="サブタスク">
        {task.subtasks.length === 0 ? (
          <p className="empty-state">サブタスクはありません。</p>
        ) : null}
        {task.subtasks.map((subtask) => (
          <SubtaskRow
            key={subtask.id}
            subtask={subtask}
            activeTimer={activeTimer}
            isMutating={isMutating}
            onStartTimer={onStartTimer}
            onStopTimer={onStopTimer}
            onCompleteSubtask={onCompleteSubtask}
            onDeleteSubtask={onDeleteSubtask}
          />
        ))}
      </div>
    </div>
  );
}

type SubtaskRowProps = {
  subtask: Subtask;
  activeTimer: ActiveTimer | null;
  isMutating: boolean;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
  onCompleteSubtask(subtask: Subtask): Promise<boolean>;
  onDeleteSubtask(subtask: Subtask): Promise<boolean>;
};

function SubtaskRow({
  subtask,
  activeTimer,
  isMutating,
  onStartTimer,
  onStopTimer,
  onCompleteSubtask,
  onDeleteSubtask,
}: SubtaskRowProps) {
  const target: WorkTargetRef = { type: "subtask", id: subtask.id };

  return (
    <div className="subtask-row">
      <input
        type="checkbox"
        checked={subtask.status === "done"}
        disabled={isMutating || subtask.status === "done"}
        onChange={() => void onCompleteSubtask(subtask)}
      />
      <div>
        <span>{subtask.title}</span>
        <small>
          {statusLabels[subtask.status]} / 終了 {formatDateLabel(subtask.dueDate)}
        </small>
      </div>
      <TimerButton
        target={target}
        label={subtask.title}
        status={subtask.status}
        activeTimer={activeTimer}
        isMutating={isMutating}
        onStartTimer={onStartTimer}
        onStopTimer={onStopTimer}
      />
      <button
        className="inline-danger-button"
        type="button"
        disabled={isMutating}
        aria-label={`${subtask.title}を削除`}
        title="サブタスクを削除"
        onClick={() => void onDeleteSubtask(subtask)}
      >
        ×
      </button>
    </div>
  );
}

type TimerButtonProps = {
  target: WorkTargetRef;
  label: string;
  status: Task["status"];
  activeTimer: ActiveTimer | null;
  isMutating: boolean;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
};

function TimerButton({
  target,
  label,
  status,
  activeTimer,
  isMutating,
  onStartTimer,
  onStopTimer,
}: TimerButtonProps) {
  const isActive = isActiveTarget(activeTimer, target);
  const canStart =
    !activeTimer && status !== "done" && status !== "archived" && !isMutating;
  const disabled = isMutating || (!isActive && !canStart);

  return (
    <button
      className={isActive ? "stop-button" : "icon-button"}
      type="button"
      aria-label={
        isActive ? `${label}のタイマーを停止` : `${label}のタイマーを開始`
      }
      title={isActive ? "タイマーを停止" : "タイマーを開始"}
      disabled={disabled}
      onClick={() =>
        isActive ? void onStopTimer() : void onStartTimer(target)
      }
    >
      {isActive ? "■" : "▶"}
    </button>
  );
}

function normalizeDraft(input: WorkItemDraft): WorkItemDraft {
  return {
    title: input.title,
    plannedStartDate: normalizeOptionalText(input.plannedStartDate),
    dueDate: normalizeOptionalText(input.dueDate),
    memo: input.memo ?? "",
  };
}

function normalizeOptionalText(value: string | null | undefined) {
  if (!value) {
    return null;
  }
  return value;
}

function isActiveTarget(activeTimer: ActiveTimer | null, target: WorkTargetRef) {
  return (
    activeTimer?.target.type === target.type && activeTimer.target.id === target.id
  );
}

function formatDateLabel(value: string | null) {
  if (!value) {
    return "未設定";
  }
  const [, month, day] = value.split("-");
  return `${Number(month)}/${Number(day)}`;
}
