import { useMemo } from "react";
import type {
  TaskRow,
  TaskWithSubtasks,
} from "../../application/usecases/contracts";
import type { Task } from "../../domain/task/types";

type BoardStatus = "todo" | "in_progress" | "done";

type KanbanBoardProps = {
  tasks: TaskWithSubtasks[];
  taskRows: TaskRow[];
  selectedTaskId: string | null;
  isLoading: boolean;
  isMutating: boolean;
  pendingTaskActionIds: ReadonlySet<string>;
  onSelectTask(taskId: string): void;
  onToggleTaskCompletion(task: TaskWithSubtasks): Promise<boolean>;
  onChangeTaskStatus(
    task: TaskWithSubtasks,
    status: BoardStatus,
  ): Promise<boolean>;
};

const columns: { status: BoardStatus; title: string; emptyMessage: string }[] = [
  {
    status: "todo",
    title: "未着手",
    emptyMessage: "未着手のタスクはありません。",
  },
  {
    status: "in_progress",
    title: "進行中",
    emptyMessage: "進行中のタスクはありません。",
  },
  {
    status: "done",
    title: "完了",
    emptyMessage: "完了タスクはありません。",
  },
];

const statusActionLabels: Record<BoardStatus, string> = {
  todo: "未着手へ",
  in_progress: "進行中へ",
  done: "完了へ",
};

const statusLabels: Record<Task["status"], string> = {
  todo: "未着手",
  in_progress: "進行中",
  done: "完了",
  archived: "アーカイブ",
};

export function KanbanBoard({
  tasks,
  taskRows,
  selectedTaskId,
  isLoading,
  isMutating,
  pendingTaskActionIds,
  onSelectTask,
  onToggleTaskCompletion,
  onChangeTaskStatus,
}: KanbanBoardProps) {
  const taskById = useMemo(
    () => new Map(tasks.map((task) => [task.id, task])),
    [tasks],
  );
  const rowsByStatus = useMemo(() => {
    const groups = new Map<BoardStatus, TaskRow[]>(
      columns.map((column) => [column.status, []]),
    );
    for (const row of taskRows) {
      if (row.status === "todo" || row.status === "in_progress" || row.status === "done") {
        groups.get(row.status)?.push(row);
      }
    }
    return groups;
  }, [taskRows]);

  return (
    <section className="panel kanban-panel" aria-labelledby="kanban-title">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">かんばん</p>
          <h2 id="kanban-title">状態別ビュー</h2>
        </div>
        <span className="task-count-badge">{taskRows.length}</span>
      </div>

      <div className="kanban-board" aria-label="かんばん">
        {isLoading ? (
          <p className="empty-state">タスクを読み込み中です。</p>
        ) : null}
        {!isLoading
          ? columns.map((column) => {
              const rows = rowsByStatus.get(column.status) ?? [];
              return (
                <section className="kanban-column" key={column.status}>
                  <div className="kanban-column-heading">
                    <h3>{column.title}</h3>
                    <span>{rows.length}</span>
                  </div>
                  <div className="kanban-column-scroll">
                    {rows.length === 0 ? (
                      <p className="kanban-empty">{column.emptyMessage}</p>
                    ) : null}
                    {rows.map((row) => {
                      const task = taskById.get(row.id) ?? null;
                      return (
                        <KanbanCard
                          key={row.id}
                          row={row}
                          task={task}
                          isSelected={row.id === selectedTaskId}
                          isMutating={isMutating || pendingTaskActionIds.has(row.id)}
                          onSelectTask={onSelectTask}
                          onToggleTaskCompletion={onToggleTaskCompletion}
                          onChangeTaskStatus={onChangeTaskStatus}
                        />
                      );
                    })}
                  </div>
                </section>
              );
            })
          : null}
      </div>
    </section>
  );
}

type KanbanCardProps = {
  row: TaskRow;
  task: TaskWithSubtasks | null;
  isSelected: boolean;
  isMutating: boolean;
  onSelectTask(taskId: string): void;
  onToggleTaskCompletion(task: TaskWithSubtasks): Promise<boolean>;
  onChangeTaskStatus(
    task: TaskWithSubtasks,
    status: BoardStatus,
  ): Promise<boolean>;
};

function KanbanCard({
  row,
  task,
  isSelected,
  isMutating,
  onSelectTask,
  onToggleTaskCompletion,
  onChangeTaskStatus,
}: KanbanCardProps) {
  const hasProgress = row.subtaskTotalCount > 0;
  const progressPercent = hasProgress
    ? Math.round((row.completedSubtaskCount / row.subtaskTotalCount) * 100)
    : 0;
  const memoPreview = formatMemoPreview(task?.memo ?? "");

  return (
    <article
      className={`kanban-card ${isSelected ? "is-selected" : ""} ${
        row.status === "done" ? "is-done" : ""
      }`}
    >
      <div className="kanban-card-content">
        <button
          className={`task-check-button ${row.status === "done" ? "is-done" : ""}`}
          type="button"
          aria-label={row.status === "done" ? "未完了に戻す" : "タスクを完了"}
          title={row.status === "done" ? "未完了に戻す" : "完了"}
          disabled={isMutating || !task}
          onClick={() => {
            if (task) {
              void onToggleTaskCompletion(task);
            }
          }}
        >
          {row.status === "done" ? "✓" : ""}
        </button>
        <button
          className="kanban-card-main"
          type="button"
          aria-label={`${row.title}の詳細を開く`}
          onClick={() => onSelectTask(row.id)}
        >
          <span className="kanban-card-title">{row.title}</span>
          <span className="kanban-card-meta">
            <span>{statusLabels[row.status]}</span>
            {row.dueDate ? (
              <span className="task-due-label">
                期限 {formatDateLabel(row.dueDate)}
                {row.dueTime ? ` ${row.dueTime}` : ""}
              </span>
            ) : null}
            {row.isTimerActive ? <span>実行中</span> : null}
          </span>
          {memoPreview ? (
            <span className="kanban-card-memo">{memoPreview}</span>
          ) : null}
          {row.tags.length > 0 ? (
            <span className="kanban-card-tags" aria-label="タグ">
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
      </div>

      <div className="kanban-card-actions" aria-label={`${row.title}の状態変更`}>
        {columns
          .filter((column) => column.status !== row.status)
          .map((column) => (
            <button
              type="button"
              key={column.status}
              disabled={isMutating || !task}
              onClick={() => {
                if (!task) {
                  return;
                }
                void onChangeTaskStatus(task, column.status);
              }}
            >
              {statusActionLabels[column.status]}
            </button>
          ))}
      </div>
    </article>
  );
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
  return normalized.length > 56 ? `${normalized.slice(0, 56)}...` : normalized;
}
