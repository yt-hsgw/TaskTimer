import {
  closestCenter,
  DndContext,
  DragOverlay,
  KeyboardSensor,
  pointerWithin,
  PointerSensor,
  useDroppable,
  useSensor,
  useSensors,
  type CollisionDetection,
  type DragEndEvent,
  type DragOverEvent,
  type DragStartEvent,
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
import {
  CalendarCheck2,
  CalendarClock,
  CalendarPlus2,
  Check,
  EllipsisVertical,
  ListPlus,
  Minus,
  Plus,
  X,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { createPortal } from "react-dom";
import type {
  BoardColumn,
  TaskRow,
  TaskWithSubtasks,
  WorkScheduleDraft,
} from "../../application/usecases/contracts";
import { usePresentationRenderProbe } from "../renderProbe";
import { ScheduleAssignmentDialog } from "./ScheduleAssignmentDialog";

type KanbanBoardProps = {
  columns: BoardColumn[];
  tasks: TaskWithSubtasks[];
  taskRows: TaskRow[];
  selectedTaskId: string | null;
  isLoading: boolean;
  isMutating: boolean;
  isCreatingTaskPending: boolean;
  isLoadingMore: boolean;
  totalTaskCount: number;
  hasMoreTasks: boolean;
  pendingTaskActionIds: ReadonlySet<string>;
  todayDate: string;
  onSelectTask(taskId: string): void;
  onRequestCreateTask(boardColumnId: string, boardColumnTitle: string): void;
  onToggleTaskCompletion(task: TaskWithSubtasks): Promise<boolean>;
  onCreateColumn(title: string): Promise<boolean>;
  onRenameColumn(columnId: string, title: string): Promise<boolean>;
  onReorderColumns(orderedColumnIds: string[]): Promise<boolean>;
  onDeleteColumn(columnId: string, moveTasksToColumnId: string): Promise<boolean>;
  onDeleteCompletedTasks(boardColumnId: string): Promise<boolean>;
  onMoveTask(taskId: string, boardColumnId: string): Promise<boolean>;
  onAssignWorkSchedule(
    taskId: string,
    schedule: WorkScheduleDraft,
  ): Promise<boolean>;
  onLoadMoreTasks(): Promise<void>;
};

const columnDragId = (columnId: string) => `column:${columnId}`;
const columnDropId = (columnId: string) => `column-drop:${columnId}`;
const taskDragId = (taskId: string) => `task:${taskId}`;

type KanbanScheduleTarget = "today" | "tomorrow" | "custom";

const scheduleDropId = (target: KanbanScheduleTarget) =>
  `schedule-target:${target}`;

const kanbanCollisionDetection: CollisionDetection = (args) => {
  const pointerCollisions = pointerWithin(args);
  return pointerCollisions.length > 0 ? pointerCollisions : closestCenter(args);
};

type PendingTaskMove = {
  taskId: string;
  destinationColumnId: string;
};

type KanbanSortMode = "default" | "due" | "created-desc" | "title";

const kanbanSortOptions: { value: KanbanSortMode; label: string }[] = [
  { value: "default", label: "既定順" },
  { value: "due", label: "期限が近い順" },
  { value: "created-desc", label: "作成日の新しい順" },
  { value: "title", label: "タイトル順" },
];

function sortKanbanRows(rows: TaskRow[], sortMode: KanbanSortMode) {
  if (sortMode === "default") {
    return rows;
  }

  const sortedRows = [...rows];
  sortedRows.sort((left, right) => {
    if (sortMode === "due") {
      const leftDue = left.dueDate
        ? `${left.dueDate}T${left.dueTime ?? "23:59"}`
        : null;
      const rightDue = right.dueDate
        ? `${right.dueDate}T${right.dueTime ?? "23:59"}`
        : null;
      if (leftDue !== rightDue) {
        if (!leftDue) {
          return 1;
        }
        if (!rightDue) {
          return -1;
        }
        return leftDue.localeCompare(rightDue);
      }
    }
    if (sortMode === "created-desc" && left.createdAt !== right.createdAt) {
      return right.createdAt.localeCompare(left.createdAt);
    }
    return left.title.localeCompare(right.title, "ja", {
      numeric: true,
      sensitivity: "base",
    });
  });
  return sortedRows;
}

export function KanbanBoard({
  columns,
  tasks,
  taskRows,
  selectedTaskId,
  isLoading,
  isMutating,
  isCreatingTaskPending,
  isLoadingMore,
  totalTaskCount,
  hasMoreTasks,
  pendingTaskActionIds,
  todayDate,
  onSelectTask,
  onRequestCreateTask,
  onToggleTaskCompletion,
  onCreateColumn,
  onRenameColumn,
  onReorderColumns,
  onDeleteColumn,
  onDeleteCompletedTasks,
  onMoveTask,
  onAssignWorkSchedule,
  onLoadMoreTasks,
}: KanbanBoardProps) {
  usePresentationRenderProbe("KanbanBoard");
  const [isCreatingColumn, setIsCreatingColumn] = useState(false);
  const [newColumnTitle, setNewColumnTitle] = useState("");
  const [pendingDeleteColumnId, setPendingDeleteColumnId] = useState<
    string | null
  >(null);
  const [deleteDestinationId, setDeleteDestinationId] = useState("");
  const [activeTaskId, setActiveTaskId] = useState<string | null>(null);
  const [dragOverColumnId, setDragOverColumnId] = useState<string | null>(null);
  const [pendingTaskMove, setPendingTaskMove] = useState<PendingTaskMove | null>(
    null,
  );
  const [scheduleDialogTaskId, setScheduleDialogTaskId] = useState<string | null>(
    null,
  );
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
      const requestedColumnId =
        pendingTaskMove?.taskId === row.id
          ? pendingTaskMove.destinationColumnId
          : row.boardColumnId;
      const columnId = groups.has(requestedColumnId)
        ? requestedColumnId
        : fallbackColumnId;
      if (columnId) {
        groups.get(columnId)?.push(
          row.boardColumnId === columnId
            ? row
            : { ...row, boardColumnId: columnId },
        );
      }
    }
    return groups;
  }, [columns, pendingTaskMove, taskRows]);
  const pendingDeleteColumn =
    columns.find((column) => column.id === pendingDeleteColumnId) ?? null;
  const activeTaskRow = activeTaskId
    ? taskRows.find((row) => row.id === activeTaskId) ?? null
    : null;
  const scheduleDialogTask = scheduleDialogTaskId
    ? taskRows.find((row) => row.id === scheduleDialogTaskId) ?? null
    : null;
  const deletionDestinations = columns.filter(
    (column) => column.id !== pendingDeleteColumnId,
  );

  useEffect(() => {
    if (!pendingTaskMove) {
      return;
    }
    const persistedRow = taskRows.find(
      (row) => row.id === pendingTaskMove.taskId,
    );
    if (persistedRow?.boardColumnId === pendingTaskMove.destinationColumnId) {
      setPendingTaskMove(null);
    }
  }, [pendingTaskMove, taskRows]);

  function handleDragStart(event: DragStartEvent) {
    setDragOverColumnId(null);
    setActiveTaskId(
      event.active.data.current?.type === "task"
        ? (event.active.data.current.taskId as string)
        : null,
    );
  }

  function handleDragOver(event: DragOverEvent) {
    const isScheduleTarget =
      event.over?.data.current?.type === "schedule-target";
    setDragOverColumnId(
      event.active.data.current?.type === "task" && !isScheduleTarget
        ? ((event.over?.data.current?.columnId as string | undefined) ?? null)
        : null,
    );
  }

  function handleDragEnd(event: DragEndEvent) {
    setActiveTaskId(null);
    setDragOverColumnId(null);
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
      const activeRow = taskId
        ? taskRows.find((row) => row.id === taskId) ?? null
        : null;
      const scheduleTarget = over.data.current?.scheduleTarget as
        | KanbanScheduleTarget
        | undefined;
      if (taskId && activeRow?.schedule === null && scheduleTarget) {
        if (scheduleTarget === "custom") {
          setScheduleDialogTaskId(taskId);
        } else {
          const date =
            scheduleTarget === "today"
              ? todayDate
              : addDaysToDateInput(todayDate, 1);
          void assignAllDaySchedule(taskId, date);
        }
        return;
      }
      const sourceColumnId = active.data.current?.columnId as string | undefined;
      const destinationColumnId = over.data.current?.columnId as
        | string
        | undefined;
      if (
        taskId &&
        destinationColumnId &&
        sourceColumnId !== destinationColumnId
      ) {
        const move = { taskId, destinationColumnId };
        setPendingTaskMove(move);
        void persistTaskMove(move);
      }
    }
  }

  function assignAllDaySchedule(taskId: string, date: string) {
    return onAssignWorkSchedule(taskId, createAllDaySchedule(date));
  }

  async function persistTaskMove(move: PendingTaskMove) {
    try {
      if (await onMoveTask(move.taskId, move.destinationColumnId)) {
        return;
      }
    } catch {
      // The mutation boundary reports the user-facing error.
    }
    setPendingTaskMove((current) =>
      current?.taskId === move.taskId &&
      current.destinationColumnId === move.destinationColumnId
        ? null
        : current,
    );
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
        </div>
      </div>

      <DndContext
        sensors={sensors}
        collisionDetection={kanbanCollisionDetection}
        onDragStart={handleDragStart}
        onDragOver={handleDragOver}
        onDragCancel={() => {
          setActiveTaskId(null);
          setDragOverColumnId(null);
        }}
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
                    isTaskDragOver={dragOverColumnId === column.id}
                    isMutating={isMutating}
                    isCreatingTaskPending={isCreatingTaskPending}
                    canDelete={columns.length > 1}
                    pendingTaskActionIds={pendingTaskActionIds}
                    todayDate={todayDate}
                    onSelectTask={onSelectTask}
                    onRequestCreateTask={onRequestCreateTask}
                    onToggleTaskCompletion={onToggleTaskCompletion}
                    onRenameColumn={onRenameColumn}
                    onRequestDelete={openDeleteDialog}
                    onDeleteCompletedTasks={onDeleteCompletedTasks}
                    onAssignAllDaySchedule={assignAllDaySchedule}
                    onRequestCustomSchedule={(taskId) =>
                      setScheduleDialogTaskId(taskId)
                    }
                  />
                ))
              : null}
            {!isLoading ? (
              <div className="kanban-column-add-slot">
                {isCreatingColumn ? (
                  <form
                    className="kanban-column-create"
                    onSubmit={handleCreateColumn}
                  >
                    <input
                      autoFocus
                      value={newColumnTitle}
                      maxLength={80}
                      aria-label="新しい状態名"
                      placeholder="状態名"
                      disabled={isMutating}
                      onChange={(event) => setNewColumnTitle(event.target.value)}
                    />
                    <div className="kanban-column-create-actions">
                      <button
                        type="submit"
                        disabled={isMutating || !newColumnTitle.trim()}
                      >
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
                    </div>
                  </form>
                ) : (
                  <button
                    className="kanban-column-add-trigger"
                    type="button"
                    aria-label="状態を追加"
                    disabled={isMutating}
                    onClick={() => setIsCreatingColumn(true)}
                  >
                    <Plus aria-hidden="true" size={20} />
                    <span>状態を追加</span>
                  </button>
                )}
              </div>
            ) : null}
          </div>
        </SortableContext>
        {activeTaskRow?.schedule === null
          ? createPortal(
              <KanbanScheduleTargets
                taskTitle={activeTaskRow.title}
                todayDate={todayDate}
              />,
              document.body,
            )
          : null}
        {createPortal(
          <DragOverlay dropAnimation={null} zIndex={1200}>
            {activeTaskRow ? (
              <KanbanCardOverlay
                row={activeTaskRow}
                task={taskById.get(activeTaskRow.id) ?? null}
              />
            ) : null}
          </DragOverlay>,
          document.body,
        )}
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

      {scheduleDialogTask ? (
        <ScheduleAssignmentDialog
          initialDate={todayDate}
          isPending={
            isMutating || pendingTaskActionIds.has(scheduleDialogTask.id)
          }
          task={scheduleDialogTask}
          onClose={() => setScheduleDialogTaskId(null)}
          onSubmit={(schedule) =>
            onAssignWorkSchedule(scheduleDialogTask.id, schedule)
          }
        />
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

type KanbanScheduleTargetsProps = {
  taskTitle: string;
  todayDate: string;
};

function KanbanScheduleTargets({
  taskTitle,
  todayDate,
}: KanbanScheduleTargetsProps) {
  const tomorrowDate = addDaysToDateInput(todayDate, 1);
  return (
    <section
      aria-label={`${taskTitle}の予定設定先`}
      className="kanban-schedule-targets"
    >
      <header>
        <span>予定を設定</span>
        <strong title={taskTitle}>{taskTitle}</strong>
      </header>
      <div>
        <KanbanScheduleDropTarget
          target="today"
          label="今日"
          meta={formatDateLabel(todayDate)}
        />
        <KanbanScheduleDropTarget
          target="tomorrow"
          label="明日"
          meta={formatDateLabel(tomorrowDate)}
        />
        <KanbanScheduleDropTarget
          target="custom"
          label="日時を選択"
          meta="ダイアログ"
        />
      </div>
    </section>
  );
}

type KanbanScheduleDropTargetProps = {
  target: KanbanScheduleTarget;
  label: string;
  meta: string;
};

function KanbanScheduleDropTarget({
  target,
  label,
  meta,
}: KanbanScheduleDropTargetProps) {
  const { isOver, setNodeRef } = useDroppable({
    id: scheduleDropId(target),
    data: { type: "schedule-target", scheduleTarget: target },
  });
  return (
    <div
      ref={setNodeRef}
      className={`kanban-schedule-target ${isOver ? "is-over" : ""}`}
      data-schedule-target={target}
    >
      {target === "today" ? (
        <CalendarCheck2 aria-hidden="true" size={19} />
      ) : target === "tomorrow" ? (
        <CalendarPlus2 aria-hidden="true" size={19} />
      ) : (
        <CalendarClock aria-hidden="true" size={19} />
      )}
      <span>{label}</span>
      <small>{meta}</small>
    </div>
  );
}

type SortableKanbanColumnProps = {
  column: BoardColumn;
  rows: TaskRow[];
  taskById: ReadonlyMap<string, TaskWithSubtasks>;
  selectedTaskId: string | null;
  isTaskDragOver: boolean;
  isMutating: boolean;
  isCreatingTaskPending: boolean;
  canDelete: boolean;
  pendingTaskActionIds: ReadonlySet<string>;
  todayDate: string;
  onSelectTask(taskId: string): void;
  onRequestCreateTask(boardColumnId: string, boardColumnTitle: string): void;
  onToggleTaskCompletion(task: TaskWithSubtasks): Promise<boolean>;
  onRenameColumn(columnId: string, title: string): Promise<boolean>;
  onRequestDelete(columnId: string): void;
  onDeleteCompletedTasks(boardColumnId: string): Promise<boolean>;
  onAssignAllDaySchedule(taskId: string, date: string): Promise<boolean>;
  onRequestCustomSchedule(taskId: string): void;
};

function SortableKanbanColumn({
  column,
  rows,
  taskById,
  selectedTaskId,
  isTaskDragOver,
  isMutating,
  isCreatingTaskPending,
  canDelete,
  pendingTaskActionIds,
  todayDate,
  onSelectTask,
  onRequestCreateTask,
  onToggleTaskCompletion,
  onRenameColumn,
  onRequestDelete,
  onDeleteCompletedTasks,
  onAssignAllDaySchedule,
  onRequestCustomSchedule,
}: SortableKanbanColumnProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const [sortMode, setSortMode] = useState<KanbanSortMode>("default");
  const [title, setTitle] = useState(column.title);
  const isRenameCommittingRef = useRef(false);
  const menuTriggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
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
  const sortedRows = useMemo(
    () => sortKanbanRows(rows, sortMode),
    [rows, sortMode],
  );
  const activeRows = sortedRows.filter((row) => row.status !== "done");
  const completedRows = sortedRows.filter((row) => row.status === "done");

  useEffect(() => {
    if (!isEditing) {
      setTitle(column.title);
    }
  }, [column.title, isEditing]);

  useEffect(() => {
    if (!isMenuOpen) {
      return;
    }

    const focusFrame = window.requestAnimationFrame(() => {
      menuRef.current
        ?.querySelector<HTMLElement>(
          '[role="menuitem"]:not([disabled]), [role="menuitemradio"]:not([disabled])',
        )
        ?.focus();
    });
    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (
        target instanceof Node &&
        !menuRef.current?.contains(target) &&
        !menuTriggerRef.current?.contains(target)
      ) {
        setIsMenuOpen(false);
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setIsMenuOpen(false);
        menuTriggerRef.current?.focus();
      }
    };
    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      window.cancelAnimationFrame(focusFrame);
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isMenuOpen]);

  async function commitRename() {
    if (isRenameCommittingRef.current) {
      return;
    }

    const normalizedTitle = title.trim();
    if (!normalizedTitle) {
      setTitle(column.title);
      setIsEditing(false);
      return;
    }

    if (normalizedTitle === column.title) {
      setTitle(normalizedTitle);
      setIsEditing(false);
      return;
    }

    isRenameCommittingRef.current = true;
    try {
      if (await onRenameColumn(column.id, normalizedTitle)) {
        setTitle(normalizedTitle);
        setIsEditing(false);
      }
    } finally {
      isRenameCommittingRef.current = false;
    }
  }

  function handleRename(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    void commitRename();
  }

  function startRename() {
    setIsMenuOpen(false);
    setTitle(column.title);
    setIsEditing(true);
  }

  async function handleDeleteCompletedTasks() {
    setIsMenuOpen(false);
    if (column.completedTaskCount <= 0) {
      return;
    }
    if (
      !window.confirm(
        `「${column.title}」の完了タスク${column.completedTaskCount}件をすべて削除しますか？\n関連するサブタスクと履歴も削除されます。`,
      )
    ) {
      menuTriggerRef.current?.focus();
      return;
    }
    await onDeleteCompletedTasks(column.id);
  }

  return (
    <section
      ref={setNodeRef}
      className={`kanban-column ${isDragging ? "is-dragging" : ""} ${
        isMenuOpen ? "has-open-menu" : ""
      }`}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
      }}
    >
      <button
        className="kanban-column-drag-handle"
        type="button"
        aria-label={`${column.title}を並べ替え`}
        title="状態を並べ替え"
        disabled={isMutating}
        {...attributes}
        {...listeners}
      >
        <Minus aria-hidden="true" size={24} strokeWidth={3} />
      </button>
      <div className="kanban-column-heading">
        {isEditing ? (
          <form className="kanban-column-title-form" onSubmit={handleRename}>
            <input
              autoFocus
              value={title}
              maxLength={80}
              aria-label="状態名"
              disabled={isMutating}
              onChange={(event) => setTitle(event.target.value)}
              onBlur={() => void commitRename()}
              onKeyDown={(event) => {
                if (event.key === "Escape") {
                  setTitle(column.title);
                  setIsEditing(false);
                }
              }}
            />
          </form>
        ) : (
          <div className="kanban-column-title" title={column.title}>
            <span>{column.title}</span>
          </div>
        )}
        <span
          className="kanban-column-count"
          title={`読み込み済み ${activeRows.length}件`}
        >
          {column.activeTaskCount}
        </span>
        <button
          ref={menuTriggerRef}
          className="kanban-column-menu-trigger"
          type="button"
          aria-label={`${column.title}のメニューを開く`}
          aria-haspopup="menu"
          aria-expanded={isMenuOpen}
          title="状態の操作"
          disabled={isMutating}
          onClick={() => setIsMenuOpen((current) => !current)}
        >
          <EllipsisVertical aria-hidden="true" size={17} />
        </button>
        {isMenuOpen ? (
          <div
            ref={menuRef}
            className="kanban-column-menu"
            role="menu"
            aria-label={`${column.title}の操作`}
          >
            <button type="button" role="menuitem" onClick={startRename}>
              タイトルを編集
            </button>
            <button
              type="button"
              role="menuitem"
              disabled={column.completedTaskCount <= 0}
              onClick={() => void handleDeleteCompletedTasks()}
            >
              <span>完了タスクを全件削除</span>
              <small>{column.completedTaskCount}</small>
            </button>
            <div className="kanban-column-menu-separator" role="separator" />
            <span className="kanban-column-menu-label">タスクの並べ替え</span>
            {kanbanSortOptions.map((option) => (
              <button
                key={option.value}
                type="button"
                role="menuitemradio"
                aria-checked={sortMode === option.value}
                onClick={() => {
                  setSortMode(option.value);
                  setIsMenuOpen(false);
                  menuTriggerRef.current?.focus();
                }}
              >
                <span
                  className={`kanban-column-sort-indicator ${
                    sortMode === option.value ? "is-selected" : ""
                  }`}
                  aria-hidden="true"
                />
                {option.label}
              </button>
            ))}
            <div className="kanban-column-menu-separator" role="separator" />
            <button
              className="is-danger"
              type="button"
              role="menuitem"
              disabled={!canDelete}
              title={canDelete ? undefined : "最後の状態は削除できません"}
              onClick={() => {
                setIsMenuOpen(false);
                onRequestDelete(column.id);
              }}
            >
              状態を削除
            </button>
          </div>
        ) : null}
      </div>

      <div
        ref={setDropRef}
        className={`kanban-column-scroll ${
          isOver || isTaskDragOver ? "is-over" : ""
        }`}
      >
        <SortableContext
          items={sortedRows.map((row) => taskDragId(row.id))}
          strategy={verticalListSortingStrategy}
        >
          <div className="kanban-active-tasks">
            <button
              className="kanban-column-add-task"
              type="button"
              data-task-create-trigger
              aria-label={`${column.title}にタスクを追加`}
              disabled={isMutating || isCreatingTaskPending}
              onClick={(event) => {
                event.currentTarget.focus();
                onRequestCreateTask(column.id, column.title);
              }}
            >
              <ListPlus aria-hidden="true" size={16} />
              <span>タスクを追加</span>
            </button>
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
                todayDate={todayDate}
                onSelectTask={onSelectTask}
                onToggleTaskCompletion={onToggleTaskCompletion}
                onAssignAllDaySchedule={onAssignAllDaySchedule}
                onRequestCustomSchedule={onRequestCustomSchedule}
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
                    todayDate={todayDate}
                    onSelectTask={onSelectTask}
                    onToggleTaskCompletion={onToggleTaskCompletion}
                    onAssignAllDaySchedule={onAssignAllDaySchedule}
                    onRequestCustomSchedule={onRequestCustomSchedule}
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
  todayDate: string;
  onSelectTask(taskId: string): void;
  onToggleTaskCompletion(task: TaskWithSubtasks): Promise<boolean>;
  onAssignAllDaySchedule(taskId: string, date: string): Promise<boolean>;
  onRequestCustomSchedule(taskId: string): void;
};

function SortableKanbanCard({
  row,
  task,
  isSelected,
  isMutating,
  todayDate,
  onSelectTask,
  onToggleTaskCompletion,
  onAssignAllDaySchedule,
  onRequestCustomSchedule,
}: SortableKanbanCardProps) {
  const {
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
      } ${isDragging ? "is-dragging" : ""} ${
        row.schedule === null ? "has-schedule-menu" : ""
      }`}
      data-task-id={row.id}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
      }}
      tabIndex={isMutating ? -1 : 0}
      aria-roledescription="移動可能なタスク"
      {...listeners}
    >
      <button
        className={`task-check-button ${row.status === "done" ? "is-done" : ""}`}
        type="button"
        aria-label={row.status === "done" ? "未完了に戻す" : "タスクを完了"}
        title={row.status === "done" ? "未完了に戻す" : "完了"}
        disabled={isMutating || !task}
        onKeyDown={(event) => event.stopPropagation()}
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
        onKeyDown={(event) => event.stopPropagation()}
        onClick={() => onSelectTask(row.id)}
      >
        <KanbanCardDetails
          row={row}
          memoPreview={memoPreview}
          hasProgress={hasProgress}
          progressPercent={progressPercent}
        />
      </button>
      {row.schedule === null ? (
        <KanbanCardScheduleMenu
          taskId={row.id}
          taskTitle={row.title}
          todayDate={todayDate}
          disabled={isMutating}
          onAssignAllDaySchedule={onAssignAllDaySchedule}
          onRequestCustomSchedule={onRequestCustomSchedule}
        />
      ) : null}
    </article>
  );
}

type KanbanCardScheduleMenuProps = {
  taskId: string;
  taskTitle: string;
  todayDate: string;
  disabled: boolean;
  onAssignAllDaySchedule(taskId: string, date: string): Promise<boolean>;
  onRequestCustomSchedule(taskId: string): void;
};

function KanbanCardScheduleMenu({
  taskId,
  taskTitle,
  todayDate,
  disabled,
  onAssignAllDaySchedule,
  onRequestCustomSchedule,
}: KanbanCardScheduleMenuProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [menuPosition, setMenuPosition] = useState({ left: 0, top: 0 });
  const triggerRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    const focusFrame = window.requestAnimationFrame(() => {
      menuRef.current?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
    });
    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (
        target instanceof Node &&
        !menuRef.current?.contains(target) &&
        !triggerRef.current?.contains(target)
      ) {
        setIsOpen(false);
      }
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setIsOpen(false);
        triggerRef.current?.focus();
      }
    };
    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      window.cancelAnimationFrame(focusFrame);
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen]);

  async function assignDate(date: string) {
    setIsOpen(false);
    await onAssignAllDaySchedule(taskId, date);
    triggerRef.current?.focus();
  }

  function toggleMenu() {
    if (isOpen) {
      setIsOpen(false);
      return;
    }
    const bounds = triggerRef.current?.getBoundingClientRect();
    if (bounds) {
      const menuHeight = 118;
      const menuTop =
        bounds.bottom + 4 + menuHeight <= window.innerHeight - 8
          ? bounds.bottom + 4
          : Math.max(8, bounds.top - menuHeight - 4);
      setMenuPosition({
        left: Math.max(8, Math.min(bounds.right - 200, window.innerWidth - 208)),
        top: menuTop,
      });
    }
    setIsOpen(true);
  }

  return (
    <div
      className="kanban-card-schedule-menu-wrap"
      onKeyDown={(event) => event.stopPropagation()}
      onPointerDown={(event) => event.stopPropagation()}
    >
      <button
        ref={triggerRef}
        aria-label={`${taskTitle}の予定設定メニューを開く`}
        aria-expanded={isOpen}
        aria-haspopup="menu"
        className="kanban-card-schedule-menu-trigger"
        disabled={disabled}
        title="予定を設定"
        type="button"
        onClick={toggleMenu}
      >
        <EllipsisVertical aria-hidden="true" size={16} />
      </button>
      {isOpen
        ? createPortal(
            <div
              ref={menuRef}
              aria-label={`${taskTitle}の予定設定`}
              className="kanban-card-schedule-menu"
              role="menu"
              style={menuPosition}
            >
              <button
                type="button"
                role="menuitem"
                onClick={() => void assignDate(todayDate)}
              >
                <CalendarCheck2 aria-hidden="true" size={16} />
                <span>今日</span>
                <small>{formatDateLabel(todayDate)}</small>
              </button>
              <button
                type="button"
                role="menuitem"
                onClick={() => void assignDate(addDaysToDateInput(todayDate, 1))}
              >
                <CalendarPlus2 aria-hidden="true" size={16} />
                <span>明日</span>
                <small>{formatDateLabel(addDaysToDateInput(todayDate, 1))}</small>
              </button>
              <button
                type="button"
                role="menuitem"
                onClick={() => {
                  setIsOpen(false);
                  onRequestCustomSchedule(taskId);
                }}
              >
                <CalendarClock aria-hidden="true" size={16} />
                <span>日時を選択</span>
              </button>
            </div>,
            document.body,
          )
        : null}
    </div>
  );
}

type KanbanCardOverlayProps = {
  row: TaskRow;
  task: TaskWithSubtasks | null;
};

function KanbanCardOverlay({ row, task }: KanbanCardOverlayProps) {
  const hasProgress = row.subtaskTotalCount > 0;
  const progressPercent = hasProgress
    ? Math.round((row.completedSubtaskCount / row.subtaskTotalCount) * 100)
    : 0;

  return (
    <article
      className={`kanban-card kanban-card-overlay ${
        row.status === "done" ? "is-done" : ""
      }`}
      data-task-id={row.id}
      aria-hidden="true"
    >
      <span
        className={`task-check-button ${row.status === "done" ? "is-done" : ""}`}
      >
        {row.status === "done" ? <Check aria-hidden="true" size={15} /> : null}
      </span>
      <span className="kanban-card-main">
        <KanbanCardDetails
          row={row}
          memoPreview={formatMemoPreview(task?.memo ?? "")}
          hasProgress={hasProgress}
          progressPercent={progressPercent}
        />
      </span>
    </article>
  );
}

type KanbanCardDetailsProps = {
  row: TaskRow;
  memoPreview: string;
  hasProgress: boolean;
  progressPercent: number;
};

function KanbanCardDetails({
  row,
  memoPreview,
  hasProgress,
  progressPercent,
}: KanbanCardDetailsProps) {
  return (
    <>
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
    </>
  );
}

function createAllDaySchedule(date: string): WorkScheduleDraft {
  return {
    startDate: date,
    startTime: null,
    endDate: date,
    endTime: null,
    isAllDay: true,
  };
}

function addDaysToDateInput(value: string, amount: number) {
  const [year, month, day] = value.split("-").map(Number);
  const date = new Date(year, month - 1, day + amount);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(
    2,
    "0",
  )}-${String(date.getDate()).padStart(2, "0")}`;
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
