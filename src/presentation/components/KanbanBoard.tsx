import {
  closestCenter,
  DndContext,
  KeyboardSensor,
  PointerSensor,
  useDroppable,
  useSensor,
  useSensors,
  type DragEndEvent,
} from "@dnd-kit/core";
import {
  arrayMove,
  horizontalListSortingStrategy,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Check, GripVertical, Pencil, Plus, Trash2, X } from "lucide-react";
import { useMemo, useState, type FormEvent } from "react";
import type {
  BoardColumn,
  TaskRow,
  TaskWithSubtasks,
} from "../../application/usecases/contracts";

type KanbanBoardProps = {
  columns: BoardColumn[];
  tasks: TaskWithSubtasks[];
  taskRows: TaskRow[];
  selectedTaskId: string | null;
  isLoading: boolean;
  isMutating: boolean;
  isLoadingMore: boolean;
  totalTaskCount: number;
  hasMoreTasks: boolean;
  pendingTaskActionIds: ReadonlySet<string>;
  onSelectTask(taskId: string): void;
  onToggleTaskCompletion(task: TaskWithSubtasks): Promise<boolean>;
  onCreateColumn(title: string): Promise<boolean>;
  onRenameColumn(columnId: string, title: string): Promise<boolean>;
  onReorderColumns(orderedColumnIds: string[]): Promise<boolean>;
  onDeleteColumn(columnId: string, moveTasksToColumnId: string): Promise<boolean>;
  onMoveTask(taskId: string, boardColumnId: string): Promise<boolean>;
  onLoadMoreTasks(): Promise<void>;
};

const columnDragId = (columnId: string) => `column:${columnId}`;
const columnDropId = (columnId: string) => `column-drop:${columnId}`;
const taskDragId = (taskId: string) => `task:${taskId}`;

export function KanbanBoard({
  columns,
  tasks,
  taskRows,
  selectedTaskId,
  isLoading,
  isMutating,
  isLoadingMore,
  totalTaskCount,
  hasMoreTasks,
  pendingTaskActionIds,
  onSelectTask,
  onToggleTaskCompletion,
  onCreateColumn,
  onRenameColumn,
  onReorderColumns,
  onDeleteColumn,
  onMoveTask,
  onLoadMoreTasks,
}: KanbanBoardProps) {
  const [isCreatingColumn, setIsCreatingColumn] = useState(false);
  const [newColumnTitle, setNewColumnTitle] = useState("");
  const [pendingDeleteColumnId, setPendingDeleteColumnId] = useState<
    string | null
  >(null);
  const [deleteDestinationId, setDeleteDestinationId] = useState("");
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );
  const taskById = useMemo(
    () => new Map(tasks.map((task) => [task.id, task])),
    [tasks],
  );
  const rowsByColumn = useMemo(() => {
    const groups = new Map(columns.map((column) => [column.id, [] as TaskRow[]]));
    const fallbackColumnId = columns[0]?.id;
    for (const row of taskRows) {
      const columnId = groups.has(row.boardColumnId)
        ? row.boardColumnId
        : fallbackColumnId;
      if (columnId) {
        groups.get(columnId)?.push(row);
      }
    }
    return groups;
  }, [columns, taskRows]);
  const pendingDeleteColumn =
    columns.find((column) => column.id === pendingDeleteColumnId) ?? null;
  const deletionDestinations = columns.filter(
    (column) => column.id !== pendingDeleteColumnId,
  );

  function handleDragEnd(event: DragEndEvent) {
    const { active, over } = event;
    if (!over || active.id === over.id) {
      return;
    }

    const activeType = active.data.current?.type;
    if (activeType === "column") {
      const activeColumnId = active.data.current?.columnId as string | undefined;
      const overColumnId = over.data.current?.columnId as string | undefined;
      if (!activeColumnId || !overColumnId || activeColumnId === overColumnId) {
        return;
      }
      const oldIndex = columns.findIndex((column) => column.id === activeColumnId);
      const newIndex = columns.findIndex((column) => column.id === overColumnId);
      if (oldIndex < 0 || newIndex < 0) {
        return;
      }
      const orderedIds = arrayMove(columns, oldIndex, newIndex).map(
        (column) => column.id,
      );
      void onReorderColumns(orderedIds);
      return;
    }

    if (activeType === "task") {
      const taskId = active.data.current?.taskId as string | undefined;
      const sourceColumnId = active.data.current?.columnId as string | undefined;
      const destinationColumnId = over.data.current?.columnId as
        | string
        | undefined;
      if (
        taskId &&
        destinationColumnId &&
        sourceColumnId !== destinationColumnId
      ) {
        void onMoveTask(taskId, destinationColumnId);
      }
    }
  }

  async function handleCreateColumn(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const title = newColumnTitle.trim();
    if (!title) {
      return;
    }
    if (await onCreateColumn(title)) {
      setNewColumnTitle("");
      setIsCreatingColumn(false);
    }
  }

  function openDeleteDialog(columnId: string) {
    const columnIndex = columns.findIndex((column) => column.id === columnId);
    const destination = columns[columnIndex + 1] ?? columns[columnIndex - 1];
    setPendingDeleteColumnId(columnId);
    setDeleteDestinationId(destination?.id ?? "");
  }

  async function handleDeleteColumn() {
    if (!pendingDeleteColumnId || !deleteDestinationId) {
      return;
    }
    if (await onDeleteColumn(pendingDeleteColumnId, deleteDestinationId)) {
      setPendingDeleteColumnId(null);
      setDeleteDestinationId("");
    }
  }

  return (
    <section className="panel kanban-panel" aria-labelledby="kanban-title">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">かんばん</p>
          <h2 id="kanban-title">状態別ビュー</h2>
        </div>
        <div className="kanban-heading-actions">
          <span
            className="task-count-badge"
            aria-label={`タスク総件数 ${totalTaskCount}件`}
            title={`読み込み済み ${taskRows.length}件`}
          >
            {totalTaskCount}
          </span>
          <button
            className="icon-button"
            type="button"
            aria-label="状態を追加"
            title="状態を追加"
            disabled={isMutating}
            onClick={() => setIsCreatingColumn(true)}
          >
            <Plus aria-hidden="true" size={19} />
          </button>
        </div>
      </div>

      {isCreatingColumn ? (
        <form className="kanban-column-create" onSubmit={handleCreateColumn}>
          <input
            autoFocus
            value={newColumnTitle}
            maxLength={80}
            aria-label="新しい状態名"
            placeholder="状態名"
            disabled={isMutating}
            onChange={(event) => setNewColumnTitle(event.target.value)}
          />
          <button type="submit" disabled={isMutating || !newColumnTitle.trim()}>
            追加
          </button>
          <button
            className="icon-button"
            type="button"
            aria-label="追加をキャンセル"
            title="キャンセル"
            disabled={isMutating}
            onClick={() => {
              setNewColumnTitle("");
              setIsCreatingColumn(false);
            }}
          >
            <X aria-hidden="true" size={18} />
          </button>
        </form>
      ) : null}

      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={handleDragEnd}
      >
        <SortableContext
          items={columns.map((column) => columnDragId(column.id))}
          strategy={horizontalListSortingStrategy}
        >
          <div className="kanban-board" aria-label="かんばん">
            {isLoading ? (
              <p className="empty-state">タスクを読み込み中です。</p>
            ) : null}
            {!isLoading && columns.length === 0 ? (
              <p className="empty-state">利用できる状態がありません。</p>
            ) : null}
            {!isLoading
              ? columns.map((column) => (
                  <SortableKanbanColumn
                    key={column.id}
                    column={column}
                    rows={rowsByColumn.get(column.id) ?? []}
                    taskById={taskById}
                    selectedTaskId={selectedTaskId}
                    isMutating={isMutating}
                    canDelete={columns.length > 1}
                    pendingTaskActionIds={pendingTaskActionIds}
                    onSelectTask={onSelectTask}
                    onToggleTaskCompletion={onToggleTaskCompletion}
                    onRenameColumn={onRenameColumn}
                    onRequestDelete={openDeleteDialog}
                  />
                ))
              : null}
          </div>
        </SortableContext>
      </DndContext>

      {hasMoreTasks ? (
        <div className="kanban-load-more">
          <button
            className="secondary-button"
            type="button"
            disabled={isMutating || isLoadingMore}
            onClick={() => void onLoadMoreTasks()}
          >
            {isLoadingMore ? "読み込み中..." : "さらに読み込む"}
            <span>
              {taskRows.length} / {totalTaskCount}
            </span>
          </button>
        </div>
      ) : null}

      {pendingDeleteColumn ? (
        <div className="kanban-dialog-backdrop">
          <section
            className="kanban-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="delete-column-title"
          >
            <h3 id="delete-column-title">状態を削除</h3>
            <p>
              「{pendingDeleteColumn.title}」のタスクを移動してから削除します。
            </p>
            <label>
              移動先
              <select
                value={deleteDestinationId}
                disabled={isMutating}
                onChange={(event) => setDeleteDestinationId(event.target.value)}
              >
                {deletionDestinations.map((column) => (
                  <option key={column.id} value={column.id}>
                    {column.title}
                  </option>
                ))}
              </select>
            </label>
            <div className="kanban-dialog-actions">
              <button
                type="button"
                disabled={isMutating}
                onClick={() => setPendingDeleteColumnId(null)}
              >
                キャンセル
              </button>
              <button
                className="danger-button"
                type="button"
                disabled={isMutating || !deleteDestinationId}
                onClick={() => void handleDeleteColumn()}
              >
                削除
              </button>
            </div>
          </section>
        </div>
      ) : null}
    </section>
  );
}

type SortableKanbanColumnProps = {
  column: BoardColumn;
  rows: TaskRow[];
  taskById: ReadonlyMap<string, TaskWithSubtasks>;
  selectedTaskId: string | null;
  isMutating: boolean;
  canDelete: boolean;
  pendingTaskActionIds: ReadonlySet<string>;
  onSelectTask(taskId: string): void;
  onToggleTaskCompletion(task: TaskWithSubtasks): Promise<boolean>;
  onRenameColumn(columnId: string, title: string): Promise<boolean>;
  onRequestDelete(columnId: string): void;
};

function SortableKanbanColumn({
  column,
  rows,
  taskById,
  selectedTaskId,
  isMutating,
  canDelete,
  pendingTaskActionIds,
  onSelectTask,
  onToggleTaskCompletion,
  onRenameColumn,
  onRequestDelete,
}: SortableKanbanColumnProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [title, setTitle] = useState(column.title);
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({
    id: columnDragId(column.id),
    data: { type: "column", columnId: column.id },
    disabled: isMutating,
  });
  const { setNodeRef: setDropRef, isOver } = useDroppable({
    id: columnDropId(column.id),
    data: { type: "column-drop", columnId: column.id },
    disabled: isMutating,
  });
  const activeRows = rows.filter((row) => row.status !== "done");
  const completedRows = rows.filter((row) => row.status === "done");

  async function handleRename(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const normalizedTitle = title.trim();
    if (!normalizedTitle) {
      return;
    }
    if (normalizedTitle === column.title || (await onRenameColumn(column.id, normalizedTitle))) {
      setTitle(normalizedTitle);
      setIsEditing(false);
    }
  }

  return (
    <section
      ref={setNodeRef}
      className={`kanban-column ${isDragging ? "is-dragging" : ""}`}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
      }}
    >
      <div className="kanban-column-heading">
        <button
          className="kanban-drag-handle"
          type="button"
          aria-label={`${column.title}を並べ替え`}
          title="状態を並べ替え"
          disabled={isMutating}
          {...attributes}
          {...listeners}
        >
          <GripVertical aria-hidden="true" size={17} />
        </button>
        {isEditing ? (
          <form className="kanban-column-title-form" onSubmit={handleRename}>
            <input
              autoFocus
              value={title}
              maxLength={80}
              aria-label="状態名"
              disabled={isMutating}
              onChange={(event) => setTitle(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Escape") {
                  setTitle(column.title);
                  setIsEditing(false);
                }
              }}
            />
          </form>
        ) : (
          <button
            className="kanban-column-title"
            type="button"
            title="状態名を編集"
            disabled={isMutating}
            onClick={() => setIsEditing(true)}
          >
            <span>{column.title}</span>
            <Pencil aria-hidden="true" size={13} />
          </button>
        )}
        <span
          className="kanban-column-count"
          title={`読み込み済み ${activeRows.length}件`}
        >
          {column.activeTaskCount}
        </span>
        <button
          className="kanban-column-delete"
          type="button"
          aria-label={`${column.title}を削除`}
          title={canDelete ? "状態を削除" : "最後の状態は削除できません"}
          disabled={isMutating || !canDelete}
          onClick={() => onRequestDelete(column.id)}
        >
          <Trash2 aria-hidden="true" size={15} />
        </button>
      </div>

      <div
        ref={setDropRef}
        className={`kanban-column-scroll ${isOver ? "is-over" : ""}`}
      >
        <SortableContext
          items={rows.map((row) => taskDragId(row.id))}
          strategy={verticalListSortingStrategy}
        >
          <div className="kanban-active-tasks">
            {activeRows.length === 0 ? (
              <p className="kanban-empty">タスクはありません。</p>
            ) : null}
            {activeRows.map((row) => (
              <SortableKanbanCard
                key={row.id}
                row={row}
                task={taskById.get(row.id) ?? null}
                isSelected={row.id === selectedTaskId}
                isMutating={isMutating || pendingTaskActionIds.has(row.id)}
                onSelectTask={onSelectTask}
                onToggleTaskCompletion={onToggleTaskCompletion}
              />
            ))}
          </div>

          {column.completedTaskCount > 0 ? (
            <details className="kanban-completed-section">
              <summary>
                <span>完了</span>
                <span title={`読み込み済み ${completedRows.length}件`}>
                  {column.completedTaskCount}
                </span>
              </summary>
              <div className="kanban-completed-list">
                {completedRows.length === 0 ? (
                  <p className="kanban-empty">
                    完了タスクを表示するには、続きを読み込んでください。
                  </p>
                ) : null}
                {completedRows.map((row) => (
                  <SortableKanbanCard
                    key={row.id}
                    row={row}
                    task={taskById.get(row.id) ?? null}
                    isSelected={row.id === selectedTaskId}
                    isMutating={isMutating || pendingTaskActionIds.has(row.id)}
                    onSelectTask={onSelectTask}
                    onToggleTaskCompletion={onToggleTaskCompletion}
                  />
                ))}
              </div>
            </details>
          ) : null}
        </SortableContext>
      </div>
    </section>
  );
}

type SortableKanbanCardProps = {
  row: TaskRow;
  task: TaskWithSubtasks | null;
  isSelected: boolean;
  isMutating: boolean;
  onSelectTask(taskId: string): void;
  onToggleTaskCompletion(task: TaskWithSubtasks): Promise<boolean>;
};

function SortableKanbanCard({
  row,
  task,
  isSelected,
  isMutating,
  onSelectTask,
  onToggleTaskCompletion,
}: SortableKanbanCardProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({
    id: taskDragId(row.id),
    data: { type: "task", taskId: row.id, columnId: row.boardColumnId },
    disabled: isMutating,
  });
  const hasProgress = row.subtaskTotalCount > 0;
  const progressPercent = hasProgress
    ? Math.round((row.completedSubtaskCount / row.subtaskTotalCount) * 100)
    : 0;
  const memoPreview = formatMemoPreview(task?.memo ?? "");

  return (
    <article
      ref={setNodeRef}
      className={`kanban-card ${isSelected ? "is-selected" : ""} ${
        row.status === "done" ? "is-done" : ""
      } ${isDragging ? "is-dragging" : ""}`}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
      }}
    >
      <button
        className="kanban-card-drag-handle"
        type="button"
        aria-label={`${row.title}を移動`}
        title="タスクを移動"
        disabled={isMutating}
        {...attributes}
        {...listeners}
      >
        <GripVertical aria-hidden="true" size={15} />
      </button>
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
        {row.status === "done" ? <Check aria-hidden="true" size={15} /> : null}
      </button>
      <button
        className="kanban-card-main"
        type="button"
        aria-label={`${row.title}の詳細を開く`}
        onClick={() => onSelectTask(row.id)}
      >
        <span className="kanban-card-title">{row.title}</span>
        <span className="kanban-card-meta">
          {row.dueDate ? (
            <span className="task-due-label">
              期限 {formatDateLabel(row.dueDate)}
              {row.dueTime ? ` ${row.dueTime}` : ""}
            </span>
          ) : null}
          {row.isTimerActive ? <span>実行中</span> : null}
        </span>
        {memoPreview ? <span className="kanban-card-memo">{memoPreview}</span> : null}
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
    </article>
  );
}

function formatDateLabel(value: string) {
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
