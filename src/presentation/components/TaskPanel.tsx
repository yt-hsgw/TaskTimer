import { FormEvent, useEffect, useMemo, useRef, useState } from "react";
import type {
  TaskRow,
  TaskWithSubtasks,
  WorkItemDraft,
} from "../../application/usecases/contracts";
import type { Subtask, Task } from "../../domain/task/types";

type TaskPanelProps = {
  tasks: TaskWithSubtasks[];
  taskRows: TaskRow[];
  selectedTaskId: string | null;
  selectedSubtaskId: string | null;
  eyebrow?: string;
  title?: string;
  emptyMessage?: string;
  showTaskForm?: boolean;
  isLoading: boolean;
  isMutating: boolean;
  isCreatingTaskPending: boolean;
  pendingTaskActionIds: ReadonlySet<string>;
  onSelectTask(taskId: string): void;
  onSelectSubtask(taskId: string, subtaskId: string): void;
  onCreateTask(input: WorkItemDraft): Promise<boolean>;
  onToggleTaskCompletion(task: TaskWithSubtasks): Promise<boolean>;
  onToggleTaskFavorite(taskId: string, isFavorite: boolean): Promise<boolean>;
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
  selectedSubtaskId,
  eyebrow = "DB接続済みタスク",
  title = "タスク",
  emptyMessage = "まだタスクはありません。",
  showTaskForm = true,
  isLoading,
  isMutating,
  isCreatingTaskPending,
  pendingTaskActionIds,
  onSelectTask,
  onSelectSubtask,
  onCreateTask,
  onToggleTaskCompletion,
  onToggleTaskFavorite,
}: TaskPanelProps) {
  const [isCreatingTask, setIsCreatingTask] = useState(false);
  const [isCompletedOpen, setIsCompletedOpen] = useState(true);
  const [expandedTaskIds, setExpandedTaskIds] = useState<ReadonlySet<string>>(
    new Set(),
  );
  const [taskDraft, setTaskDraft] = useState<WorkItemDraft>({
    title: "",
    dueDate: "",
    dueTime: "",
    memo: "",
  });
  const taskTitleInputRef = useRef<HTMLInputElement>(null);

  const taskById = useMemo(
    () => new Map(tasks.map((task) => [task.id, task])),
    [tasks],
  );
  const incompleteRows = taskRows.filter((task) => task.status !== "done");
  const completedRows = taskRows.filter((task) => task.status === "done");
  const isCreateDisabled = isMutating || isCreatingTaskPending;

  useEffect(() => {
    if (!showTaskForm) {
      return;
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.ctrlKey && event.key.toLowerCase() === "n") {
        event.preventDefault();
        setIsCreatingTask(true);
        return;
      }
      if (event.key === "Escape") {
        setIsCreatingTask(false);
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
      setTaskDraft({ title: "", dueDate: "", dueTime: "", memo: "" });
      setIsCreatingTask(false);
    }
  }

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
          <span className="task-count-badge">{taskRows.length}</span>
          {showTaskForm ? (
            <button
              className="task-add-button"
              type="button"
              aria-label="タスクを追加"
              title="タスクを追加"
              disabled={isCreateDisabled || isCreatingTask}
              onClick={() => setIsCreatingTask(true)}
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
            isSelected={row.id === selectedTaskId}
            selectedSubtaskId={selectedSubtaskId}
            isExpanded={expandedTaskIds.has(row.id)}
            isMutating={isMutating || pendingTaskActionIds.has(row.id)}
            onSelectTask={onSelectTask}
            onSelectSubtask={onSelectSubtask}
            onToggleExpansion={toggleTaskExpansion}
            onToggleTaskCompletion={handleCompleteRow}
            onToggleTaskFavorite={onToggleTaskFavorite}
          />
        ))}

        {showTaskForm && isCreatingTask ? (
          <div className="task-composer">
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
                  disabled={isCreateDisabled}
                  maxLength={120}
                  required
                />
              </label>
              <div className="date-fields">
                <label>
                  <span>期限日</span>
                  <input
                    type="date"
                    value={taskDraft.dueDate ?? ""}
                    onChange={(event) =>
                      setTaskDraft((current) => ({
                        ...current,
                        dueDate: event.target.value,
                      }))
                    }
                    disabled={isCreateDisabled}
                  />
                </label>
                <label>
                  <span>期限時刻</span>
                  <input
                    type="time"
                    value={taskDraft.dueTime ?? ""}
                    onChange={(event) =>
                      setTaskDraft((current) => ({
                        ...current,
                        dueTime: event.target.value,
                      }))
                    }
                    disabled={isCreateDisabled || !taskDraft.dueDate}
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
                  disabled={isCreateDisabled}
                  rows={3}
                />
              </label>
              <div className="composer-actions">
                <button
                  className="primary-button"
                  type="submit"
                  disabled={isCreateDisabled}
                >
                  追加
                </button>
                <button
                  className="secondary-button"
                  type="button"
                  disabled={isCreateDisabled}
                  onClick={() => setIsCreatingTask(false)}
                >
                  キャンセル
                </button>
              </div>
            </form>
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
                    task={taskById.get(row.id) ?? null}
                    isSelected={row.id === selectedTaskId}
                    selectedSubtaskId={selectedSubtaskId}
                    isExpanded={expandedTaskIds.has(row.id)}
                    isMutating={isMutating || pendingTaskActionIds.has(row.id)}
                    onSelectTask={onSelectTask}
                    onSelectSubtask={onSelectSubtask}
                    onToggleExpansion={toggleTaskExpansion}
                    onToggleTaskCompletion={handleCompleteRow}
                    onToggleTaskFavorite={onToggleTaskFavorite}
                  />
                ))}
              </div>
            ) : null}
          </section>
        ) : null}
      </div>
    </section>
  );
}

type TaskRowItemProps = {
  row: TaskRow;
  task: TaskWithSubtasks | null;
  isSelected: boolean;
  selectedSubtaskId: string | null;
  isExpanded: boolean;
  isMutating: boolean;
  onSelectTask(taskId: string): void;
  onSelectSubtask(taskId: string, subtaskId: string): void;
  onToggleExpansion(taskId: string): void;
  onToggleTaskCompletion(row: TaskRow): void;
  onToggleTaskFavorite(taskId: string, isFavorite: boolean): Promise<boolean>;
};

function TaskRowItem({
  row,
  task,
  isSelected,
  selectedSubtaskId,
  isExpanded,
  isMutating,
  onSelectTask,
  onSelectSubtask,
  onToggleExpansion,
  onToggleTaskCompletion,
  onToggleTaskFavorite,
}: TaskRowItemProps) {
  const hasProgress = row.subtaskTotalCount > 0;
  const subtasks = task?.subtasks ?? [];
  const progressPercent = hasProgress
    ? Math.round((row.completedSubtaskCount / row.subtaskTotalCount) * 100)
    : 0;
  const isDone = row.status === "done";
  const memoPreview = formatMemoPreview(task?.memo ?? "");

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

      {isExpanded && subtasks.length > 0 ? (
        <div className="task-row-subtasks" aria-label={`${row.title}のサブタスク`}>
          {subtasks.map((subtask) => (
            <SubtaskRowButton
              key={subtask.id}
              subtask={subtask}
              isSelected={subtask.id === selectedSubtaskId}
              onSelect={() => onSelectSubtask(row.id, subtask.id)}
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
};

function SubtaskRowButton({
  subtask,
  isSelected,
  onSelect,
}: SubtaskRowButtonProps) {
  return (
    <button
      className={`task-row-subtask ${isSelected ? "is-selected" : ""} ${
        subtask.status === "done" ? "is-done" : ""
      }`}
      type="button"
      onClick={onSelect}
    >
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
  );
}

function normalizeDraft(input: WorkItemDraft): WorkItemDraft {
  const dueDate = normalizeOptionalText(input.dueDate);
  return {
    title: input.title,
    plannedStartDate: null,
    dueDate,
    dueTime: dueDate ? normalizeOptionalText(input.dueTime) : null,
    memo: input.memo ?? "",
  };
}

function normalizeOptionalText(value: string | null | undefined) {
  if (!value) {
    return null;
  }
  return value;
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
