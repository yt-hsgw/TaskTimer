import { useEffect, useMemo, useRef, useState } from "react";
import type {
  CSSProperties,
  DragEvent,
  FormEvent,
  KeyboardEvent as ReactKeyboardEvent,
  MouseEvent as ReactMouseEvent,
  PointerEvent as ReactPointerEvent,
} from "react";
import { GripHorizontal, GripVertical } from "lucide-react";
import type {
  ScheduledTaskDraft,
  TaskListItem,
  WeekCalendarItem,
  WorkScheduleDraft,
  WorkScheduleMoveDraft,
} from "../../application/usecases/contracts";
import type { WorkTargetRef } from "../../domain/task/types";
import { usePresentationRenderProbe } from "../renderProbe";

export type CalendarViewMode = "week" | "day" | "month";

type WeekCalendarProps = {
  viewMode: CalendarViewMode;
  anchorDate: string;
  items: WeekCalendarItem[];
  taskLists: TaskListItem[];
  defaultTaskListId: string;
  isLoading: boolean;
  isCreatingTaskPending: boolean;
  isReschedulingItem: boolean;
  selectedTarget: WorkTargetRef | null;
  onChangeViewMode(viewMode: CalendarViewMode): void;
  onPreviousRange(): void;
  onNextRange(): void;
  onToday(): void;
  onSelectItem(item: WeekCalendarItem): void;
  onCreateTask(input: ScheduledTaskDraft): Promise<boolean>;
  onRescheduleItem(
    item: WeekCalendarItem,
    dueDate: string,
    dueTime: string | null,
  ): Promise<boolean>;
  onResizeItem(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft,
  ): Promise<boolean>;
  onMoveScheduledItem(
    item: WeekCalendarItem,
    destination: WorkScheduleMoveDraft,
  ): Promise<boolean>;
};

const dayLabels = ["月", "火", "水", "木", "金", "土", "日"];
const businessHours = Array.from({ length: 15 }, (_, index) => 8 + index);
const markerLabels: Record<WeekCalendarItem["marker"], string> = {
  scheduled: "予定",
  planned_start: "開始予定",
  due: "期限",
  active_timer: "実行中",
};
const viewModeLabels: Record<CalendarViewMode, string> = {
  week: "週",
  day: "日",
  month: "月",
};

type CalendarTaskDraft = {
  title: string;
  listId: string;
  startDate: string;
  startTime: string;
  endDate: string;
  endTime: string;
  isAllDay: boolean;
  memo: string;
  sourceLabel: string;
};

type CalendarDropTarget = {
  dueDate: string;
  dueTime: string | null;
  zoneId?: string;
};

type PendingCalendarMove = {
  item: WeekCalendarItem;
  isSaved: boolean;
};

type CalendarCreateSelectionSurface = "timed" | "all-day" | "month";

type CalendarCreateSelectionPreviewRect = {
  key: string;
  label: string | null;
  left: number;
  top: number;
  width: number;
  height: number;
  connectsBefore: boolean;
  connectsAfter: boolean;
};

type CalendarCreateSelectionBase = {
  surface: CalendarCreateSelectionSurface;
  pointerId: number;
  captureElement: HTMLElement;
  originX: number;
  originY: number;
  didDrag: boolean;
  previewRects: CalendarCreateSelectionPreviewRect[];
};

type TimedCalendarCreateSelection = CalendarCreateSelectionBase & {
  surface: "timed";
  date: string;
  anchorMinutes: number;
  startMinutes: number;
  endMinutes: number;
  sourceHour: number;
};

type DateCalendarCreateSelection = CalendarCreateSelectionBase & {
  surface: "all-day" | "month";
  anchorDate: string;
  startDate: string;
  endDate: string;
};

type CalendarCreateSelection =
  | TimedCalendarCreateSelection
  | DateCalendarCreateSelection;

type CalendarItemVariant = "all-day" | "timed" | "month";
type TimedCalendarItemLayout = {
  laneIndex: number;
  laneCount: number;
};
type TimedCalendarLayout = {
  itemLayouts: ReadonlyMap<string, TimedCalendarItemLayout>;
  itemsByDate: ReadonlyMap<string, WeekCalendarItem[]>;
};
const RESIZE_PREVIEW_ID_SUFFIX = ":resize-preview";
const MOVE_PREVIEW_ID_SUFFIX = ":move-preview";
const CALENDAR_VISIBLE_ITEM_LIMIT = 3;
const CREATE_SELECTION_DRAG_THRESHOLD = 6;

export function WeekCalendar({
  viewMode,
  anchorDate,
  items,
  taskLists,
  defaultTaskListId,
  isLoading,
  isCreatingTaskPending,
  isReschedulingItem,
  selectedTarget,
  onChangeViewMode,
  onPreviousRange,
  onNextRange,
  onToday,
  onSelectItem,
  onCreateTask,
  onRescheduleItem,
  onResizeItem: persistResizeItem,
  onMoveScheduledItem,
}: WeekCalendarProps) {
  usePresentationRenderProbe("WeekCalendar");
  const titleInputRef = useRef<HTMLInputElement>(null);
  const createSelectionRef = useRef<CalendarCreateSelection | null>(null);
  const [createDraft, setCreateDraft] = useState<CalendarTaskDraft | null>(null);
  const [createSelection, setCreateSelection] =
    useState<CalendarCreateSelection | null>(null);
  const [draggedItem, setDraggedItem] = useState<WeekCalendarItem | null>(null);
  const [dropTarget, setDropTarget] = useState<CalendarDropTarget | null>(null);
  const [pendingMove, setPendingMove] = useState<PendingCalendarMove | null>(null);
  const [resizePreview, setResizePreview] = useState<WeekCalendarItem | null>(null);
  const displayItems = useMemo(
    () => {
      const committedItems = pendingMove
        ? items.map((item) =>
            item.id === pendingMove.item.id ? pendingMove.item : item,
          )
        : items;
      const movePreview =
        draggedItem && dropTarget
          ? moveCalendarItemPreview(draggedItem, dropTarget)
          : null;
      const previewItems = [
        resizePreview
          ? {
              ...resizePreview,
              id: `${resizePreview.id}${RESIZE_PREVIEW_ID_SUFFIX}`,
            }
          : null,
        movePreview
          ? {
              ...movePreview,
              id: `${movePreview.id}${MOVE_PREVIEW_ID_SUFFIX}`,
            }
          : null,
      ].filter((item): item is WeekCalendarItem => item !== null);
      return previewItems.length > 0
        ? [...committedItems, ...previewItems]
        : committedItems;
    },
    [draggedItem, dropTarget, items, pendingMove, resizePreview],
  );
  const rangeDays = useMemo(
    () =>
      viewMode === "day"
        ? [buildDay(anchorDate)]
        : buildWeekDays(getWeekStartDate(anchorDate)),
    [anchorDate, viewMode],
  );
  const monthDays = useMemo(() => buildMonthDays(anchorDate), [anchorDate]);
  const headingLabel = formatCalendarHeading(viewMode, anchorDate);
  const weekBadge =
    viewMode === "week" ? `第${getIsoWeekNumber(anchorDate)}週` : null;
  const fallbackListId = useMemo(() => {
    if (taskLists.some((list) => list.id === defaultTaskListId)) {
      return defaultTaskListId;
    }
    return taskLists[0]?.id ?? defaultTaskListId;
  }, [defaultTaskListId, taskLists]);

  useEffect(() => {
    titleInputRef.current?.focus();
  }, [createDraft?.sourceLabel]);

  useEffect(() => {
    if (!createDraft && !createSelection) {
      return;
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setCreateDraft(null);
        clearCreateSelection();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [createDraft, createSelection]);

  useEffect(() => {
    clearCreateSelection();
  }, [anchorDate, viewMode]);

  useEffect(() => {
    if (
      pendingMove?.isSaved &&
      items.some((item) => isSameCalendarPosition(item, pendingMove.item))
    ) {
      setPendingMove(null);
    }
  }, [items, pendingMove]);

  function openCreateForm(startDate: string, startTime: string | null) {
    const end = startTime
      ? addMinutesToLocalDateTime(startDate, startTime, 60)
      : { date: startDate, time: "" };
    openCreateFormForSchedule({
      startDate,
      startTime: startTime ?? "",
      endDate: end.date,
      endTime: end.time,
      isAllDay: !startTime,
      sourceLabel: formatCreateSourceLabel(startDate, startTime),
    });
  }

  function openCreateFormForSchedule({
    startDate,
    startTime,
    endDate,
    endTime,
    isAllDay,
    sourceLabel,
  }: {
    startDate: string;
    startTime: string;
    endDate: string;
    endTime: string;
    isAllDay: boolean;
    sourceLabel: string;
  }) {
    clearCreateSelection();
    setCreateDraft({
      title: "",
      listId: fallbackListId,
      startDate,
      startTime,
      endDate,
      endTime,
      isAllDay,
      memo: "",
      sourceLabel,
    });
  }

  function setActiveCreateSelection(selection: CalendarCreateSelection | null) {
    createSelectionRef.current = selection;
    setCreateSelection(selection);
  }

  function clearCreateSelection() {
    const current = createSelectionRef.current;
    createSelectionRef.current = null;
    setCreateSelection(null);
    if (current?.captureElement.hasPointerCapture(current.pointerId)) {
      current.captureElement.releasePointerCapture(current.pointerId);
    }
  }

  function handleCreateSelectionPointerDown(
    event: ReactPointerEvent<HTMLElement>,
  ) {
    if (
      event.button !== 0 ||
      !event.isPrimary ||
      createDraft ||
      isLoading ||
      isCreatingTaskPending ||
      isReschedulingItem ||
      draggedItem
    ) {
      return;
    }

    const target = resolveCalendarCreateSelectionTarget(event.target);
    if (!target) {
      return;
    }

    const rootBounds = event.currentTarget.getBoundingClientRect();
    const cellBounds = target.element.getBoundingClientRect();
    const base = {
      surface: target.surface,
      pointerId: event.pointerId,
      captureElement: target.element,
      originX: event.clientX,
      originY: event.clientY,
      didDrag: false,
      previewRects: [],
    };
    let selection: CalendarCreateSelection;
    if (target.surface === "timed") {
      const anchorMinutes = getTimeSelectionSlotMinutes(
        target.hour,
        event.clientY,
        cellBounds.top,
        cellBounds.height,
      );
      selection = {
        ...base,
        surface: "timed",
        date: target.date,
        anchorMinutes,
        startMinutes: anchorMinutes,
        endMinutes: anchorMinutes + 15,
        sourceHour: target.hour,
      };
    } else {
      selection = {
        ...base,
        surface: target.surface,
        anchorDate: target.date,
        startDate: target.date,
        endDate: target.date,
      };
    }

    target.element.setPointerCapture(event.pointerId);
    setActiveCreateSelection(
      updateCalendarCreateSelectionPreview(
        selection,
        event.currentTarget,
        rootBounds,
      ),
    );
  }

  function handleCreateSelectionPointerMove(
    event: ReactPointerEvent<HTMLElement>,
  ) {
    const current = createSelectionRef.current;
    if (!current || current.pointerId !== event.pointerId) {
      return;
    }
    const next = updateCalendarCreateSelection(
      current,
      event.clientX,
      event.clientY,
      event.currentTarget,
    );
    if (next !== current) {
      setActiveCreateSelection(next);
    }
    if (next.didDrag) {
      event.preventDefault();
    }
  }

  function handleCreateSelectionPointerUp(
    event: ReactPointerEvent<HTMLElement>,
  ) {
    const current = createSelectionRef.current;
    if (!current || current.pointerId !== event.pointerId) {
      return;
    }
    const completed = updateCalendarCreateSelection(
      current,
      event.clientX,
      event.clientY,
      event.currentTarget,
    );
    createSelectionRef.current = null;
    setCreateSelection(null);
    if (current.captureElement.hasPointerCapture(current.pointerId)) {
      current.captureElement.releasePointerCapture(current.pointerId);
    }
    if (!completed.didDrag) {
      return;
    }

    event.preventDefault();
    if (completed.surface === "timed") {
      const startTime = minutesToTimeInput(completed.startMinutes);
      const endTime = minutesToTimeInput(completed.endMinutes);
      openCreateFormForSchedule({
        startDate: completed.date,
        startTime,
        endDate: completed.date,
        endTime,
        isAllDay: false,
        sourceLabel: `${formatDateLabel(completed.date)} ${startTime}-${endTime}`,
      });
      return;
    }

    openCreateFormForSchedule({
      startDate: completed.startDate,
      startTime: "",
      endDate: completed.endDate,
      endTime: "",
      isAllDay: true,
      sourceLabel:
        completed.startDate === completed.endDate
          ? formatDateLabel(completed.startDate)
          : `${formatDateLabel(completed.startDate)}-${formatDateLabel(
              completed.endDate,
            )}`,
    });
  }

  function handleCreateSelectionPointerCancel(
    event: ReactPointerEvent<HTMLElement>,
  ) {
    if (createSelectionRef.current?.pointerId === event.pointerId) {
      clearCreateSelection();
    }
  }

  function handleCreateSelectionLostPointerCapture(
    event: ReactPointerEvent<HTMLElement>,
  ) {
    if (createSelectionRef.current?.pointerId === event.pointerId) {
      setActiveCreateSelection(null);
    }
  }

  function handleDragStart(
    item: WeekCalendarItem,
    event: DragEvent<HTMLButtonElement>,
  ) {
    if (!canMoveCalendarItem(item) || isReschedulingItem) {
      event.preventDefault();
      return;
    }

    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", item.id);
    setDraggedItem(item);
  }

  function handleDragEnd() {
    setDraggedItem(null);
    setDropTarget(null);
  }

  function handleDragOver(
    target: CalendarDropTarget,
    event: DragEvent<HTMLElement>,
  ) {
    if (!draggedItem || !canMoveCalendarItem(draggedItem) || isReschedulingItem) {
      return;
    }

    event.preventDefault();
    event.dataTransfer.dropEffect = "move";
    const resolvedTarget = resolveCalendarDropTarget(draggedItem, target, event);
    setDropTarget((current) =>
      isSameCalendarDestination(current, resolvedTarget)
        ? current
        : resolvedTarget,
    );
  }

  function handleDragLeave(
    target: CalendarDropTarget,
    event: DragEvent<HTMLElement>,
  ) {
    if (
      event.currentTarget instanceof Node &&
      event.relatedTarget instanceof Node &&
      event.currentTarget.contains(event.relatedTarget)
    ) {
      return;
    }

    setDropTarget((current) =>
      isSameDropTarget(current, target) ? null : current,
    );
  }

  async function handleDrop(
    target: CalendarDropTarget,
    event: DragEvent<HTMLElement>,
  ) {
    if (!draggedItem || !canMoveCalendarItem(draggedItem) || isReschedulingItem) {
      return;
    }

    event.preventDefault();
    const item = draggedItem;
    const resolvedTarget = resolveCalendarDropTarget(item, target, event);
    setDraggedItem(null);
    setDropTarget(null);
    await moveCalendarItem(item, resolvedTarget);
  }

  async function moveCalendarItem(
    item: WeekCalendarItem,
    target: CalendarDropTarget,
  ) {
    if (
      item.date === target.dueDate &&
      (item.time ?? null) === (target.dueTime ?? null)
    ) {
      return true;
    }

    const preview = moveCalendarItemPreview(item, target);
    if (!preview) {
      return false;
    }
    setPendingMove({ item: preview, isSaved: false });
    const saved =
      item.marker === "scheduled"
        ? await onMoveScheduledItem(item, {
            startDate: target.dueDate,
            startTime: target.dueTime,
          })
        : await onRescheduleItem(item, target.dueDate, target.dueTime);
    setPendingMove((current) =>
      current?.item.id === item.id && saved
        ? { ...current, isSaved: true }
        : null,
    );
    return saved;
  }

  function previewScheduleResize(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft | null,
  ) {
    const preview = schedule ? applyScheduleToCalendarItem(item, schedule) : null;
    setResizePreview((current) =>
      current && preview && isSameCalendarPosition(current, preview)
        ? current
        : preview,
    );
  }

  async function resizeCalendarItem(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft,
  ) {
    const preview = applyScheduleToCalendarItem(item, schedule);
    setResizePreview(null);
    if (!preview) {
      return false;
    }
    setPendingMove({ item: preview, isSaved: false });
    const saved = await persistResizeItem(item, schedule);
    setPendingMove((current) =>
      current?.item.id === item.id && saved
        ? { ...current, isSaved: true }
        : null,
    );
    return saved;
  }

  function handleMoveItemKeyDown(
    item: WeekCalendarItem,
    variant: CalendarItemVariant,
    event: ReactKeyboardEvent<HTMLButtonElement>,
  ) {
    if (isReschedulingItem) {
      return;
    }
    const target = getKeyboardMoveTarget(item, variant, event.key);
    if (!target) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    void moveCalendarItem(item, target);
  }

  async function handleCreateTask(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!createDraft) {
      return;
    }

    const created = await onCreateTask({
      title: createDraft.title,
      listId: createDraft.listId || fallbackListId,
      memo: createDraft.memo,
      schedule: {
        startDate: createDraft.startDate,
        startTime: createDraft.isAllDay ? null : createDraft.startTime,
        endDate: createDraft.endDate,
        endTime: createDraft.isAllDay ? null : createDraft.endTime,
        isAllDay: createDraft.isAllDay,
      },
    });
    if (created) {
      setCreateDraft(null);
    }
  }

  return (
    <section
      className={`panel calendar-panel ${
        createSelection?.didDrag ? "is-creating-range" : ""
      }`}
      aria-labelledby="calendar-title"
      onPointerDown={handleCreateSelectionPointerDown}
      onPointerMove={handleCreateSelectionPointerMove}
      onPointerUp={handleCreateSelectionPointerUp}
      onPointerCancel={handleCreateSelectionPointerCancel}
      onLostPointerCapture={handleCreateSelectionLostPointerCapture}
    >
      {createSelection?.didDrag ? (
        <>
          <div className="calendar-create-selection-layer" aria-hidden="true">
            {createSelection.previewRects.map((rect) => (
              <div
                className={`calendar-create-selection is-${createSelection.surface} ${
                  rect.connectsBefore ? "connects-before" : ""
                } ${rect.connectsAfter ? "connects-after" : ""}`}
                key={rect.key}
                style={{
                  left: `${rect.left}px`,
                  top: `${rect.top}px`,
                  width: `${rect.width}px`,
                  height: `${rect.height}px`,
                }}
              >
                {rect.label ? <span>{rect.label}</span> : null}
              </div>
            ))}
          </div>
          <span className="visually-hidden" aria-live="polite">
            {formatCalendarCreateSelectionAnnouncement(createSelection)}
          </span>
        </>
      ) : null}
      <div className="calendar-toolbar">
        <div className="calendar-toolbar-left">
          <button
            className="calendar-today-button"
            type="button"
            onClick={onToday}
          >
            今日
          </button>
          <div className="calendar-nav-buttons" aria-label="カレンダー移動">
            <button
              type="button"
              aria-label={`前の${viewModeLabels[viewMode]}`}
              onClick={onPreviousRange}
            >
              ‹
            </button>
            <button
              type="button"
              aria-label={`次の${viewModeLabels[viewMode]}`}
              onClick={onNextRange}
            >
              ›
            </button>
          </div>
          <div className="calendar-title-group">
            <h2 id="calendar-title">{headingLabel}</h2>
            {weekBadge ? <span>{weekBadge}</span> : null}
          </div>
        </div>

        <div className="calendar-heading-controls">
          <div className="calendar-view-switch" aria-label="カレンダー表示切替">
            {(["week", "day", "month"] as const).map((mode) => (
              <button
                className={viewMode === mode ? "is-active" : undefined}
                type="button"
                key={mode}
                aria-pressed={viewMode === mode}
                onClick={() => onChangeViewMode(mode)}
              >
                {viewModeLabels[mode]}
              </button>
            ))}
          </div>
        </div>
      </div>

      {createDraft ? (
        <div className="calendar-create-form-shell">
          <form
            className="work-form calendar-create-form"
            onSubmit={(event) => void handleCreateTask(event)}
          >
            <div className="calendar-create-form-heading">
              <div>
                <strong>タスクを追加</strong>
                <span>{createDraft.sourceLabel}</span>
              </div>
              <button
                className="inline-icon-button"
                type="button"
                aria-label="作成フォームを閉じる"
                disabled={isCreatingTaskPending}
                onClick={() => setCreateDraft(null)}
              >
                ×
              </button>
            </div>

            <label>
              <span>タスク名</span>
              <input
                ref={titleInputRef}
                value={createDraft.title}
                onChange={(event) =>
                  setCreateDraft((current) =>
                    current ? { ...current, title: event.target.value } : current,
                  )
                }
                placeholder="例: 企画メモを整理"
                disabled={isCreatingTaskPending}
                maxLength={120}
                required
              />
            </label>

            <div className="calendar-create-grid">
              <label>
                <span>リスト</span>
                <select
                  value={createDraft.listId}
                  onChange={(event) =>
                    setCreateDraft((current) =>
                      current
                        ? { ...current, listId: event.target.value }
                        : current,
                    )
                  }
                  disabled={isCreatingTaskPending}
                >
                  {taskLists.length > 0 ? (
                    taskLists.map((list) => (
                      <option key={list.id} value={list.id}>
                        {list.name}
                      </option>
                    ))
                  ) : (
                    <option value={fallbackListId}>タスク</option>
                  )}
                </select>
              </label>
              <label>
                <span>開始日</span>
                <input
                  type="date"
                  value={createDraft.startDate}
                  onChange={(event) =>
                    setCreateDraft((current) =>
                      current
                        ? {
                            ...current,
                            startDate: event.target.value,
                          }
                        : current,
                    )
                  }
                  disabled={isCreatingTaskPending}
                  required
                />
              </label>
              <label>
                <span>開始時刻</span>
                <input
                  type="time"
                  step={900}
                  value={createDraft.startTime}
                  onChange={(event) =>
                    setCreateDraft((current) =>
                      current ? { ...current, startTime: event.target.value } : current,
                    )
                  }
                  disabled={isCreatingTaskPending || createDraft.isAllDay}
                  required={!createDraft.isAllDay}
                />
              </label>
              <label>
                <span>終了日</span>
                <input
                  type="date"
                  value={createDraft.endDate}
                  onChange={(event) =>
                    setCreateDraft((current) =>
                      current ? { ...current, endDate: event.target.value } : current,
                    )
                  }
                  disabled={isCreatingTaskPending}
                  required
                />
              </label>
              <label>
                <span>終了時刻</span>
                <input
                  type="time"
                  step={900}
                  value={createDraft.endTime}
                  onChange={(event) =>
                    setCreateDraft((current) =>
                      current ? { ...current, endTime: event.target.value } : current,
                    )
                  }
                  disabled={isCreatingTaskPending || createDraft.isAllDay}
                  required={!createDraft.isAllDay}
                />
              </label>
            </div>

            <label className="calendar-all-day-toggle">
              <input
                type="checkbox"
                checked={createDraft.isAllDay}
                onChange={(event) =>
                  setCreateDraft((current) =>
                    current
                      ? {
                          ...current,
                          isAllDay: event.target.checked,
                          startTime:
                            event.target.checked || current.startTime
                              ? current.startTime
                              : "09:00",
                          endTime:
                            event.target.checked || current.endTime
                              ? current.endTime
                              : "10:00",
                        }
                      : current,
                  )
                }
                disabled={isCreatingTaskPending}
              />
              <span>終日</span>
            </label>

            <label>
              <span>メモ</span>
              <textarea
                value={createDraft.memo}
                onChange={(event) =>
                  setCreateDraft((current) =>
                    current ? { ...current, memo: event.target.value } : current,
                  )
                }
                disabled={isCreatingTaskPending}
                maxLength={2000}
                rows={2}
              />
            </label>

            <div className="composer-actions">
              <button
                className="primary-button"
                type="submit"
                disabled={isCreatingTaskPending}
              >
                追加
              </button>
              <button
                className="secondary-button"
                type="button"
                disabled={isCreatingTaskPending}
                onClick={() => setCreateDraft(null)}
              >
                キャンセル
              </button>
            </div>
          </form>
        </div>
      ) : null}

      {viewMode === "month" ? (
        <MonthCalendar
          days={monthDays}
          anchorDate={anchorDate}
          items={displayItems}
          isLoading={isLoading}
          selectedTarget={selectedTarget}
          draggedItem={draggedItem}
          dropTarget={dropTarget}
          isReschedulingItem={isReschedulingItem}
          onOpenCreateTask={openCreateForm}
          onSelectItem={onSelectItem}
          onDragStart={handleDragStart}
          onDragEnd={handleDragEnd}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
          onResizeItem={resizeCalendarItem}
          onResizePreview={previewScheduleResize}
          onMoveItemKeyDown={handleMoveItemKeyDown}
        />
      ) : (
        <TimeGridCalendar
          days={rangeDays}
          items={displayItems}
          isLoading={isLoading}
          selectedTarget={selectedTarget}
          viewMode={viewMode}
          draggedItem={draggedItem}
          dropTarget={dropTarget}
          isReschedulingItem={isReschedulingItem}
          onOpenCreateTask={openCreateForm}
          onSelectItem={onSelectItem}
          onDragStart={handleDragStart}
          onDragEnd={handleDragEnd}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
          onResizeItem={resizeCalendarItem}
          onResizePreview={previewScheduleResize}
          onMoveItemKeyDown={handleMoveItemKeyDown}
        />
      )}
    </section>
  );
}

function TimeGridCalendar({
  days,
  items,
  isLoading,
  selectedTarget,
  viewMode,
  draggedItem,
  dropTarget,
  isReschedulingItem,
  onOpenCreateTask,
  onSelectItem,
  onDragStart,
  onDragEnd,
  onDragOver,
  onDragLeave,
  onDrop,
  onResizeItem,
  onResizePreview,
  onMoveItemKeyDown,
}: {
  days: CalendarDay[];
  items: WeekCalendarItem[];
  isLoading: boolean;
  selectedTarget: WorkTargetRef | null;
  viewMode: CalendarViewMode;
  draggedItem: WeekCalendarItem | null;
  dropTarget: CalendarDropTarget | null;
  isReschedulingItem: boolean;
  onOpenCreateTask(dueDate: string, dueTime: string | null): void;
  onSelectItem(item: WeekCalendarItem): void;
  onDragStart(item: WeekCalendarItem, event: DragEvent<HTMLButtonElement>): void;
  onDragEnd(): void;
  onDragOver(target: CalendarDropTarget, event: DragEvent<HTMLElement>): void;
  onDragLeave(target: CalendarDropTarget, event: DragEvent<HTMLElement>): void;
  onDrop(target: CalendarDropTarget, event: DragEvent<HTMLElement>): void;
  onResizeItem(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft,
  ): Promise<boolean>;
  onResizePreview(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft | null,
  ): void;
  onMoveItemKeyDown(
    item: WeekCalendarItem,
    variant: CalendarItemVariant,
    event: ReactKeyboardEvent<HTMLButtonElement>,
  ): void;
}) {
  const currentTime = getCurrentTimeMarker(days);
  const headerItems = useMemo(
    () =>
      items.filter((item) =>
        item.marker === "scheduled"
          ? item.isAllDay || isMultiDaySchedule(item)
          : !item.time,
      ),
    [items],
  );
  const headerDayLayouts = useMemo(
    () => buildCalendarDayLayouts(days, headerItems),
    [days, headerItems],
  );
  const timedCalendarLayout = useMemo(
    () => buildTimedCalendarLayout(days, items),
    [days, items],
  );
  const rangeBoundaryStart = days[0]?.date;
  const rangeBoundaryEnd = days.at(-1)?.date;

  return (
    <div
      className={`calendar-time-grid ${
        viewMode === "day" ? "is-day-mode" : ""
      }`}
    >
      <div className="calendar-time-zone" aria-hidden="true" />
      {days.map((day) => (
        <div
          className={`calendar-time-header ${isToday(day.date) ? "is-today" : ""}`}
          key={day.date}
        >
          <span>{day.label}</span>
          <strong>{day.dayOfMonth}</strong>
        </div>
      ))}

      <div
        className="calendar-time-label is-time-zone"
        aria-label={`${formatTimeZoneOffset(new Date())}の終日・複数日予定`}
      >
        {formatTimeZoneOffset(new Date())}
      </div>
      {days.map((day) => {
        const dayLayout = headerDayLayouts.get(day.date) ?? {
          slots: [],
          hiddenCount: 0,
        };
        const dropTargetForDay = { dueDate: day.date, dueTime: null };
        return (
          <div
            className={`calendar-all-day-cell ${
              isSameDropTarget(dropTarget, dropTargetForDay)
                ? "is-drop-target"
                : ""
            }`}
            key={`${day.date}:all-day`}
            data-calendar-date={day.date}
            data-calendar-create-surface="all-day"
            role="gridcell"
            tabIndex={0}
            aria-label={`${formatCreateSourceLabel(day.date, null)}。ダブルクリックまたはEnterでタスクを追加`}
            title="ダブルクリックでタスクを追加"
            onDoubleClick={(event) =>
              handleCreateCellDoubleClick(event, day.date, null, onOpenCreateTask)
            }
            onKeyDown={(event) =>
              handleCreateCellKeyDown(event, day.date, null, onOpenCreateTask)
            }
            onDragOver={(event) => onDragOver(dropTargetForDay, event)}
            onDragLeave={(event) => onDragLeave(dropTargetForDay, event)}
            onDrop={(event) => onDrop(dropTargetForDay, event)}
          >
            <CalendarCellItems
              isLoading={isLoading}
              items={dayLayout.slots}
              hiddenCount={dayLayout.hiddenCount}
              selectedTarget={selectedTarget}
              draggedItem={draggedItem}
              isReschedulingItem={isReschedulingItem}
              variant="all-day"
              displayDate={day.date}
              rangeBoundaryStart={rangeBoundaryStart}
              rangeBoundaryEnd={rangeBoundaryEnd}
              onSelectItem={onSelectItem}
              onDragStart={onDragStart}
              onDragEnd={onDragEnd}
              onResizeItem={onResizeItem}
              onResizePreview={onResizePreview}
              onMoveItemKeyDown={onMoveItemKeyDown}
            />
          </div>
        );
      })}

      {businessHours.map((hour) => (
        <TimeGridRow
          hour={hour}
          days={days}
          items={items}
          key={hour}
          currentTime={currentTime}
          isLoading={isLoading}
          selectedTarget={selectedTarget}
          draggedItem={draggedItem}
          dropTarget={dropTarget}
          isReschedulingItem={isReschedulingItem}
          onOpenCreateTask={onOpenCreateTask}
          onSelectItem={onSelectItem}
          onDragStart={onDragStart}
          onDragEnd={onDragEnd}
          onDragOver={onDragOver}
          onDragLeave={onDragLeave}
          onDrop={onDrop}
          onResizeItem={onResizeItem}
          onResizePreview={onResizePreview}
          onMoveItemKeyDown={onMoveItemKeyDown}
        />
      ))}
      {!isLoading
        ? days.map((day, dayIndex) => (
            <div
              className="calendar-timed-day-overlay"
              data-calendar-date={day.date}
              key={`${day.date}:timed-overlay`}
              style={{
                gridColumn: dayIndex + 2,
                gridRow: `3 / span ${businessHours.length}`,
              }}
            >
              <CalendarCellItems
                isLoading={false}
                items={timedCalendarLayout.itemsByDate.get(day.date) ?? []}
                timedItemLayouts={timedCalendarLayout.itemLayouts}
                selectedTarget={selectedTarget}
                draggedItem={draggedItem}
                isReschedulingItem={isReschedulingItem}
                variant="timed"
                displayDate={day.date}
                onSelectItem={onSelectItem}
                onDragStart={onDragStart}
                onDragEnd={onDragEnd}
                onResizeItem={onResizeItem}
                onResizePreview={onResizePreview}
                onMoveItemKeyDown={onMoveItemKeyDown}
              />
            </div>
          ))
        : null}
    </div>
  );
}

function TimeGridRow({
  hour,
  days,
  items,
  currentTime,
  isLoading,
  selectedTarget,
  draggedItem,
  dropTarget,
  isReschedulingItem,
  onOpenCreateTask,
  onSelectItem,
  onDragStart,
  onDragEnd,
  onDragOver,
  onDragLeave,
  onDrop,
  onResizeItem,
  onResizePreview,
  onMoveItemKeyDown,
}: {
  hour: number;
  days: CalendarDay[];
  items: WeekCalendarItem[];
  currentTime: CurrentTimeMarker | null;
  isLoading: boolean;
  selectedTarget: WorkTargetRef | null;
  draggedItem: WeekCalendarItem | null;
  dropTarget: CalendarDropTarget | null;
  isReschedulingItem: boolean;
  onOpenCreateTask(dueDate: string, dueTime: string | null): void;
  onSelectItem(item: WeekCalendarItem): void;
  onDragStart(item: WeekCalendarItem, event: DragEvent<HTMLButtonElement>): void;
  onDragEnd(): void;
  onDragOver(target: CalendarDropTarget, event: DragEvent<HTMLElement>): void;
  onDragLeave(target: CalendarDropTarget, event: DragEvent<HTMLElement>): void;
  onDrop(target: CalendarDropTarget, event: DragEvent<HTMLElement>): void;
  onResizeItem(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft,
  ): Promise<boolean>;
  onResizePreview(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft | null,
  ): void;
  onMoveItemKeyDown(
    item: WeekCalendarItem,
    variant: CalendarItemVariant,
    event: ReactKeyboardEvent<HTMLButtonElement>,
  ): void;
}) {
  return (
    <>
      <div className="calendar-time-label">{formatHourLabel(hour)}</div>
      {days.map((day) => {
        const hourItems = items
          .filter((item) => {
            if (item.marker === "scheduled") {
              return false;
            }
            return item.date === day.date && getDisplayHour(item) === hour;
          })
          .sort(sortCalendarItems);
        const shouldShowCurrentTime =
          currentTime?.date === day.date && currentTime.hour === hour;
        const dropTargetForHour = {
          dueDate: day.date,
          dueTime: formatHourInput(hour),
        };
        return (
          <div
            className={`calendar-time-cell ${
              shouldShowCurrentTime ? "has-current-time" : ""
            } ${
              isSameDropTarget(dropTarget, dropTargetForHour)
                ? "is-drop-target"
                : ""
            }`}
            key={`${day.date}:${hour}`}
            data-calendar-date={day.date}
            data-calendar-hour={hour}
            data-calendar-create-surface="timed"
            role="gridcell"
            tabIndex={0}
            aria-label={`${formatCreateSourceLabel(day.date, formatHourInput(hour))}。ダブルクリックまたはEnterでタスクを追加`}
            title="ダブルクリックでタスクを追加"
            onDoubleClick={(event) =>
              handleCreateCellDoubleClick(
                event,
                day.date,
                formatHourInput(hour),
                onOpenCreateTask,
              )
            }
            onKeyDown={(event) =>
              handleCreateCellKeyDown(
                event,
                day.date,
                formatHourInput(hour),
                onOpenCreateTask,
              )
            }
            onDragOver={(event) => onDragOver(dropTargetForHour, event)}
            onDragLeave={(event) => onDragLeave(dropTargetForHour, event)}
            onDrop={(event) => onDrop(dropTargetForHour, event)}
          >
            {shouldShowCurrentTime ? (
              <div
                className="calendar-current-time-line"
                style={{ top: `${currentTime.offsetPercent}%` }}
              />
            ) : null}
            <CalendarCellItems
              isLoading={isLoading}
              items={hourItems}
              selectedTarget={selectedTarget}
              draggedItem={draggedItem}
              isReschedulingItem={isReschedulingItem}
              variant="timed"
              displayDate={day.date}
              onSelectItem={onSelectItem}
              onDragStart={onDragStart}
              onDragEnd={onDragEnd}
              onResizeItem={onResizeItem}
              onResizePreview={onResizePreview}
              onMoveItemKeyDown={onMoveItemKeyDown}
            />
          </div>
        );
      })}
    </>
  );
}

function CalendarCellItems({
  isLoading,
  items,
  timedItemLayouts,
  selectedTarget,
  draggedItem,
  isReschedulingItem,
  variant,
  displayDate,
  hiddenCount = 0,
  rangeBoundaryStart,
  rangeBoundaryEnd,
  onSelectItem,
  onDragStart,
  onDragEnd,
  onResizeItem,
  onResizePreview,
  onMoveItemKeyDown,
}: {
  isLoading: boolean;
  items: Array<WeekCalendarItem | null>;
  timedItemLayouts?: ReadonlyMap<string, TimedCalendarItemLayout>;
  selectedTarget: WorkTargetRef | null;
  draggedItem: WeekCalendarItem | null;
  isReschedulingItem: boolean;
  variant: Exclude<CalendarItemVariant, "month">;
  displayDate: string;
  hiddenCount?: number;
  rangeBoundaryStart?: string;
  rangeBoundaryEnd?: string;
  onSelectItem(item: WeekCalendarItem): void;
  onDragStart(item: WeekCalendarItem, event: DragEvent<HTMLButtonElement>): void;
  onDragEnd(): void;
  onResizeItem(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft,
  ): Promise<boolean>;
  onResizePreview(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft | null,
  ): void;
  onMoveItemKeyDown(
    item: WeekCalendarItem,
    variant: CalendarItemVariant,
    event: ReactKeyboardEvent<HTMLButtonElement>,
  ): void;
}) {
  if (isLoading) {
    return <p className="calendar-empty">読み込み中</p>;
  }

  if (items.length === 0 && hiddenCount === 0) {
    return null;
  }

  return (
    <div className={`calendar-items is-${variant}`}>
      {items.map((item, slotIndex) =>
        item ? (
          <CalendarItemButton
            item={item}
            key={`${item.id}:${displayDate}`}
            displayDate={displayDate}
            selectedTarget={selectedTarget}
            draggedItem={draggedItem}
            isReschedulingItem={isReschedulingItem}
            variant={variant}
            timedLayout={timedItemLayouts?.get(
              getTimedCalendarItemLayoutKey(displayDate, item.id),
            )}
            rangeBoundaryStart={rangeBoundaryStart}
            rangeBoundaryEnd={rangeBoundaryEnd}
            onSelectItem={onSelectItem}
            onDragStart={onDragStart}
            onDragEnd={onDragEnd}
            onResizeItem={onResizeItem}
            onResizePreview={onResizePreview}
            onMoveItemKeyDown={onMoveItemKeyDown}
          />
        ) : (
          <span
            className="calendar-all-day-item-placeholder"
            aria-hidden="true"
            key={`empty:${displayDate}:${slotIndex}`}
          />
        ),
      )}
      {hiddenCount > 0 ? (
        <span className="calendar-more">他 {hiddenCount} 件</span>
      ) : null}
    </div>
  );
}

function MonthCalendar({
  days,
  anchorDate,
  items,
  isLoading,
  selectedTarget,
  draggedItem,
  dropTarget,
  isReschedulingItem,
  onOpenCreateTask,
  onSelectItem,
  onDragStart,
  onDragEnd,
  onDragOver,
  onDragLeave,
  onDrop,
  onResizeItem,
  onResizePreview,
  onMoveItemKeyDown,
}: {
  days: CalendarDay[];
  anchorDate: string;
  items: WeekCalendarItem[];
  isLoading: boolean;
  selectedTarget: WorkTargetRef | null;
  draggedItem: WeekCalendarItem | null;
  dropTarget: CalendarDropTarget | null;
  isReschedulingItem: boolean;
  onOpenCreateTask(dueDate: string, dueTime: string | null): void;
  onSelectItem(item: WeekCalendarItem): void;
  onDragStart(item: WeekCalendarItem, event: DragEvent<HTMLButtonElement>): void;
  onDragEnd(): void;
  onDragOver(target: CalendarDropTarget, event: DragEvent<HTMLElement>): void;
  onDragLeave(target: CalendarDropTarget, event: DragEvent<HTMLElement>): void;
  onDrop(target: CalendarDropTarget, event: DragEvent<HTMLElement>): void;
  onResizeItem(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft,
  ): Promise<boolean>;
  onResizePreview(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft | null,
  ): void;
  onMoveItemKeyDown(
    item: WeekCalendarItem,
    variant: CalendarItemVariant,
    event: ReactKeyboardEvent<HTMLButtonElement>,
  ): void;
}) {
  const anchor = parseDateInputValue(anchorDate);
  const currentMonth = anchor.getMonth();
  const dayLayouts = useMemo(
    () => buildCalendarDayLayouts(days, items),
    [days, items],
  );

  return (
    <div className="calendar-month-shell">
      <div className="calendar-month-weekdays" aria-hidden="true">
        {dayLabels.map((label) => (
          <span key={label}>{label}</span>
        ))}
      </div>
      <div className="calendar-month-grid">
        {days.map((day) => {
          const dayLayout = dayLayouts.get(day.date) ?? {
            slots: [],
            hiddenCount: 0,
          };
          const isOutsideMonth =
            parseDateInputValue(day.date).getMonth() !== currentMonth;
          const dropTargetForDay = { dueDate: day.date, dueTime: null };
          return (
            <div
              className={`calendar-month-day ${
                isOutsideMonth ? "is-outside-month" : ""
              } ${isToday(day.date) ? "is-today" : ""} ${
                isSameDropTarget(dropTarget, dropTargetForDay)
                  ? "is-drop-target"
                  : ""
              }`}
              key={day.date}
              data-calendar-date={day.date}
              data-calendar-create-surface="month"
              role="gridcell"
              tabIndex={0}
              aria-label={`${formatCreateSourceLabel(day.date, null)}。ダブルクリックまたはEnterでタスクを追加`}
              title="ダブルクリックでタスクを追加"
              onDoubleClick={(event) =>
                handleCreateCellDoubleClick(
                  event,
                  day.date,
                  null,
                  onOpenCreateTask,
                )
              }
              onKeyDown={(event) =>
                handleCreateCellKeyDown(event, day.date, null, onOpenCreateTask)
              }
              onDragOver={(event) => onDragOver(dropTargetForDay, event)}
              onDragLeave={(event) => onDragLeave(dropTargetForDay, event)}
              onDrop={(event) => onDrop(dropTargetForDay, event)}
            >
              <div className="calendar-month-day-heading">
                <span>{day.dayOfMonth}</span>
              </div>
              {isLoading ? (
                <p className="calendar-empty">読み込み中</p>
              ) : (
                <div className="calendar-month-day-items">
                  {dayLayout.slots.map((item, slotIndex) =>
                    item ? (
                      <CalendarItemButton
                        item={item}
                        key={`${item.id}:${day.date}`}
                        displayDate={day.date}
                        selectedTarget={selectedTarget}
                        draggedItem={draggedItem}
                        isReschedulingItem={isReschedulingItem}
                        variant="month"
                        onSelectItem={onSelectItem}
                        onDragStart={onDragStart}
                        onDragEnd={onDragEnd}
                        onResizeItem={onResizeItem}
                        onResizePreview={onResizePreview}
                        onMoveItemKeyDown={onMoveItemKeyDown}
                      />
                    ) : (
                      <span
                        className="calendar-month-item-placeholder"
                        aria-hidden="true"
                        key={`empty:${day.date}:${slotIndex}`}
                      />
                    ),
                  )}
                  {dayLayout.hiddenCount > 0 ? (
                    <span className="calendar-more">
                      他 {dayLayout.hiddenCount} 件
                    </span>
                  ) : null}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

type CalendarDayLayout = {
  slots: Array<WeekCalendarItem | null>;
  hiddenCount: number;
};

function buildCalendarDayLayouts(
  days: CalendarDay[],
  items: WeekCalendarItem[],
) {
  const layouts = new Map<string, CalendarDayLayout>();
  for (let weekIndex = 0; weekIndex < days.length; weekIndex += 7) {
    const weekDays = days.slice(weekIndex, weekIndex + 7);
    const weekStart = weekDays[0]?.date;
    const weekEnd = weekDays.at(-1)?.date;
    if (!weekStart || !weekEnd) {
      continue;
    }

    const laneEndDates: string[] = [];
    const scheduledLanes = new Map<string, number>();
    const scheduledItems = items
      .filter(
        (item) =>
          item.marker === "scheduled" &&
          item.endDate !== null &&
          item.date <= weekEnd &&
          item.endDate >= weekStart,
      )
      .sort((first, second) => {
        const firstStart = first.date < weekStart ? weekStart : first.date;
        const secondStart = second.date < weekStart ? weekStart : second.date;
        return (
          firstStart.localeCompare(secondStart) ||
          (second.endDate ?? "").localeCompare(first.endDate ?? "") ||
          sortCalendarItems(first, second)
        );
      });

    for (const item of scheduledItems) {
      const clippedStart = item.date < weekStart ? weekStart : item.date;
      const clippedEnd =
        item.endDate && item.endDate < weekEnd ? item.endDate : weekEnd;
      const reusableLane = laneEndDates.findIndex(
        (laneEndDate) => laneEndDate < clippedStart,
      );
      const lane = reusableLane >= 0 ? reusableLane : laneEndDates.length;
      laneEndDates[lane] = clippedEnd;
      scheduledLanes.set(item.id, lane);
    }

    for (const day of weekDays) {
      const slots: Array<WeekCalendarItem | null> = Array.from(
        { length: CALENDAR_VISIBLE_ITEM_LIMIT },
        () => null,
      );
      const activeScheduledItems = scheduledItems.filter((item) =>
        isDateWithinSchedule(item, day.date),
      );
      let hiddenCount = 0;
      for (const item of activeScheduledItems) {
        const lane = scheduledLanes.get(item.id);
        if (lane === undefined || lane >= CALENDAR_VISIBLE_ITEM_LIMIT) {
          hiddenCount += 1;
          continue;
        }
        slots[lane] = item;
      }

      const pointItems = items
        .filter(
          (item) => item.marker !== "scheduled" && item.date === day.date,
        )
        .sort(sortCalendarItems);
      for (const item of pointItems) {
        const emptySlot = slots.findIndex((slot) => slot === null);
        if (emptySlot < 0) {
          hiddenCount += 1;
          continue;
        }
        slots[emptySlot] = item;
      }

      let lastOccupiedSlot = -1;
      for (let slotIndex = slots.length - 1; slotIndex >= 0; slotIndex -= 1) {
        if (slots[slotIndex] !== null) {
          lastOccupiedSlot = slotIndex;
          break;
        }
      }
      layouts.set(day.date, {
        slots: slots.slice(0, lastOccupiedSlot + 1),
        hiddenCount,
      });
    }
  }
  return layouts;
}

type TimedCalendarInterval = {
  item: WeekCalendarItem;
  startMinutes: number;
  endMinutes: number;
};

type ActiveTimedCalendarLane = {
  laneIndex: number;
  endMinutes: number;
};

function buildTimedCalendarLayout(
  days: CalendarDay[],
  items: WeekCalendarItem[],
): TimedCalendarLayout {
  const visibleDates = new Set(days.map((day) => day.date));
  const intervalsByDate = new Map<string, TimedCalendarInterval[]>();
  for (const item of items) {
    if (
      item.marker !== "scheduled" ||
      item.isAllDay ||
      item.date !== item.endDate ||
      !visibleDates.has(item.date)
    ) {
      continue;
    }
    const segment = getTimedScheduleSegment(item, item.date);
    if (!segment) {
      continue;
    }
    const startMinutes = segment.startMinutes;
    const interval = {
      item,
      startMinutes,
      endMinutes: startMinutes + segment.durationMinutes,
    };
    const dateIntervals = intervalsByDate.get(item.date) ?? [];
    dateIntervals.push(interval);
    intervalsByDate.set(item.date, dateIntervals);
  }

  const layouts = new Map<string, TimedCalendarItemLayout>();
  const itemsByDate = new Map<string, WeekCalendarItem[]>();
  for (const [date, intervals] of intervalsByDate) {
    intervals.sort(
      (first, second) =>
        first.startMinutes - second.startMinutes ||
        second.endMinutes - first.endMinutes ||
        first.item.id.localeCompare(second.item.id),
    );
    itemsByDate.set(
      date,
      intervals.map((interval) => interval.item),
    );
    let group: TimedCalendarInterval[] = [];
    let groupEndMinutes = -1;
    for (const interval of intervals) {
      if (group.length > 0 && interval.startMinutes >= groupEndMinutes) {
        assignTimedCalendarLanes(date, group, layouts);
        group = [];
        groupEndMinutes = -1;
      }
      group.push(interval);
      groupEndMinutes = Math.max(groupEndMinutes, interval.endMinutes);
    }
    assignTimedCalendarLanes(date, group, layouts);
  }
  return { itemLayouts: layouts, itemsByDate };
}

function assignTimedCalendarLanes(
  date: string,
  intervals: TimedCalendarInterval[],
  layouts: Map<string, TimedCalendarItemLayout>,
) {
  if (intervals.length === 0) {
    return;
  }
  const activeLanes: ActiveTimedCalendarLane[] = [];
  const availableLanes: number[] = [];
  const assignments: Array<{ itemId: string; laneIndex: number }> = [];
  let nextLaneIndex = 0;

  for (const interval of intervals) {
    while (
      activeLanes[0] &&
      activeLanes[0].endMinutes <= interval.startMinutes
    ) {
      const available = popMinHeap(activeLanes, compareActiveTimedLanes);
      if (available) {
        pushMinHeap(availableLanes, available.laneIndex, compareNumbers);
      }
    }
    const laneIndex =
      popMinHeap(availableLanes, compareNumbers) ?? nextLaneIndex++;
    assignments.push({ itemId: interval.item.id, laneIndex });
    pushMinHeap(
      activeLanes,
      { laneIndex, endMinutes: interval.endMinutes },
      compareActiveTimedLanes,
    );
  }

  for (const assignment of assignments) {
    layouts.set(getTimedCalendarItemLayoutKey(date, assignment.itemId), {
      laneIndex: assignment.laneIndex,
      laneCount: nextLaneIndex,
    });
  }
}

function pushMinHeap<T>(
  heap: T[],
  value: T,
  compare: (first: T, second: T) => number,
) {
  heap.push(value);
  let index = heap.length - 1;
  while (index > 0) {
    const parentIndex = Math.floor((index - 1) / 2);
    if (compare(heap[parentIndex], heap[index]) <= 0) {
      break;
    }
    [heap[parentIndex], heap[index]] = [heap[index], heap[parentIndex]];
    index = parentIndex;
  }
}

function popMinHeap<T>(
  heap: T[],
  compare: (first: T, second: T) => number,
): T | undefined {
  const minimum = heap[0];
  const last = heap.pop();
  if (heap.length === 0 || last === undefined) {
    return minimum;
  }
  heap[0] = last;
  let index = 0;
  while (true) {
    const leftIndex = index * 2 + 1;
    const rightIndex = leftIndex + 1;
    let minimumIndex = index;
    if (
      leftIndex < heap.length &&
      compare(heap[leftIndex], heap[minimumIndex]) < 0
    ) {
      minimumIndex = leftIndex;
    }
    if (
      rightIndex < heap.length &&
      compare(heap[rightIndex], heap[minimumIndex]) < 0
    ) {
      minimumIndex = rightIndex;
    }
    if (minimumIndex === index) {
      break;
    }
    [heap[index], heap[minimumIndex]] = [heap[minimumIndex], heap[index]];
    index = minimumIndex;
  }
  return minimum;
}

function compareActiveTimedLanes(
  first: ActiveTimedCalendarLane,
  second: ActiveTimedCalendarLane,
) {
  return (
    first.endMinutes - second.endMinutes ||
    first.laneIndex - second.laneIndex
  );
}

function compareNumbers(first: number, second: number) {
  return first - second;
}

function getTimedCalendarItemLayoutKey(date: string, itemId: string) {
  return `${date}:${itemId}`;
}

function getTimedCalendarLaneStyle(layout: TimedCalendarItemLayout) {
  if (layout.laneCount <= 1) {
    return {
      "--schedule-lane-left": "0%",
      "--schedule-lane-width": "100%",
    };
  }
  const gapPixels = 2;
  const laneWidthPercent = 100 / layout.laneCount;
  const widthGapPixels =
    (gapPixels * (layout.laneCount - 1)) / layout.laneCount;
  const leftGapPixels = (gapPixels * layout.laneIndex) / layout.laneCount;
  return {
    "--schedule-lane-left": `calc(${(
      laneWidthPercent * layout.laneIndex
    ).toFixed(6)}% + ${leftGapPixels.toFixed(3)}px)`,
    "--schedule-lane-width": `calc(${laneWidthPercent.toFixed(
      6,
    )}% - ${widthGapPixels.toFixed(3)}px)`,
  };
}

function CalendarItemButton({
  item,
  displayDate,
  timedLayout,
  selectedTarget,
  draggedItem,
  isReschedulingItem,
  variant,
  rangeBoundaryStart,
  rangeBoundaryEnd,
  onSelectItem,
  onDragStart,
  onDragEnd,
  onResizeItem,
  onResizePreview,
  onMoveItemKeyDown,
}: {
  item: WeekCalendarItem;
  displayDate: string;
  timedLayout?: TimedCalendarItemLayout;
  selectedTarget: WorkTargetRef | null;
  draggedItem: WeekCalendarItem | null;
  isReschedulingItem: boolean;
  variant: CalendarItemVariant;
  rangeBoundaryStart?: string;
  rangeBoundaryEnd?: string;
  onSelectItem(item: WeekCalendarItem): void;
  onDragStart(item: WeekCalendarItem, event: DragEvent<HTMLButtonElement>): void;
  onDragEnd(): void;
  onResizeItem(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft,
  ): Promise<boolean>;
  onResizePreview(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft | null,
  ): void;
  onMoveItemKeyDown(
    item: WeekCalendarItem,
    variant: CalendarItemVariant,
    event: ReactKeyboardEvent<HTMLButtonElement>,
  ): void;
}) {
  const previewKind = getCalendarPreviewKind(item.id);
  const isCalendarPreview = previewKind !== null;
  const isResizePreview = previewKind === "resize";
  const isMovePreview = previewKind === "move";
  const isSelected =
    !isCalendarPreview && isSameTarget(item.target, selectedTarget);
  const isDraggable =
    !isCalendarPreview && canMoveCalendarItem(item) && !isReschedulingItem;
  const isDragging = draggedItem?.id === item.id;
  const relationLabel = item.parentTitle ? `親: ${item.parentTitle}` : null;
  const markerText = formatCalendarItemMarker(item);
  const isScheduled = item.marker === "scheduled";
  const rangeSegment =
    isScheduled &&
    (variant === "month" ||
      (variant === "all-day" && isMultiDaySchedule(item)))
      ? getCalendarRangeSegment(
          item,
          displayDate,
          rangeBoundaryStart,
          rangeBoundaryEnd,
        )
      : null;
  const hasStartHandle =
    !isCalendarPreview && isScheduled && displayDate === item.date;
  const hasEndHandle =
    !isCalendarPreview && isScheduled && displayDate === item.endDate;
  const isVerticalResize = variant === "timed" && !item.isAllDay;
  const timedSegment = isVerticalResize
    ? getTimedScheduleSegment(item, displayDate)
    : null;
  const laneStyle = timedLayout
    ? getTimedCalendarLaneStyle(timedLayout)
    : null;
  const style = timedSegment
    ? ({
        "--schedule-offset": `${timedSegment.offsetMinutes * 0.9}px`,
        "--schedule-height": `${Math.max(timedSegment.durationMinutes * 0.9, 24)}px`,
        ...laneStyle,
      } as CSSProperties)
    : undefined;

  return (
    <div
      className={`calendar-item marker-${item.marker} color-${item.colorToken} is-${variant} ${
        item.target.type === "subtask" ? "is-subtask" : ""
      } ${item.status === "done" ? "is-done" : ""} ${
        isSelected ? "is-selected" : ""
      } ${isDraggable ? "is-draggable" : ""} ${
        isDragging ? "is-dragging" : ""
      } ${isCalendarPreview ? "is-calendar-preview" : ""} ${
        isResizePreview ? "is-resize-preview" : ""
      } ${isMovePreview ? "is-move-preview" : ""} ${
        rangeSegment ? "is-scheduled-range" : ""
      } ${
        rangeSegment?.connectsBefore ? "connects-before" : ""
      } ${rangeSegment?.connectsAfter ? "connects-after" : ""}`}
      style={style}
      data-calendar-lane-index={timedLayout?.laneIndex}
      data-calendar-lane-count={timedLayout?.laneCount}
      aria-hidden={isCalendarPreview || undefined}
      data-calendar-preview={previewKind ?? undefined}
      onDoubleClick={(event) => event.stopPropagation()}
    >
      {hasStartHandle ? (
        <ScheduleResizeHandle
          edge="start"
          isVertical={isVerticalResize}
          disabled={isReschedulingItem}
          item={item}
          displayDate={displayDate}
          onResizeItem={onResizeItem}
          onResizePreview={onResizePreview}
        />
      ) : null}
      <button
        className="calendar-item-content"
        type="button"
        draggable={isDraggable}
        tabIndex={isCalendarPreview ? -1 : undefined}
        aria-pressed={isSelected}
        aria-label={`${relationLabel ? `${relationLabel}、` : ""}${item.title}の${markerText}を開く${isDraggable ? `。ドラッグまたは矢印キーで${isScheduled ? "予定期間" : "期限"}を移動できます` : ""}`}
        title={
          isDraggable
            ? `ドラッグで${isScheduled ? "予定期間" : "期限"}を移動`
            : undefined
        }
        onDragStart={(event) => onDragStart(item, event)}
        onDragEnd={onDragEnd}
        onKeyDown={(event) =>
          !isCalendarPreview && onMoveItemKeyDown(item, variant, event)
        }
        onClick={(event) => {
          event.stopPropagation();
          if (!isCalendarPreview) {
            onSelectItem(item);
          }
        }}
      >
        {isCalendarPreview &&
        (!rangeSegment || rangeSegment.showsContent) ? (
          <span className="calendar-preview-label">
            {isMovePreview ? "移動後" : "変更後"}
          </span>
        ) : null}
        {rangeSegment ? (
          rangeSegment.showsContent ? (
            <>
              {item.time ? (
                <small className="calendar-month-range-time">{item.time}</small>
              ) : null}
              <span className="calendar-item-title">{item.title}</span>
            </>
          ) : null
        ) : (
          <span className="calendar-item-title">{item.title}</span>
        )}
        {relationLabel && !rangeSegment ? (
          <small className="calendar-item-parent">{relationLabel}</small>
        ) : null}
        {!rangeSegment &&
        (variant === "timed" ||
          variant === "month" ||
          isCalendarPreview) ? (
          <small>{markerText}</small>
        ) : null}
      </button>
      {hasEndHandle ? (
        <ScheduleResizeHandle
          edge="end"
          isVertical={isVerticalResize}
          disabled={isReschedulingItem}
          item={item}
          displayDate={displayDate}
          onResizeItem={onResizeItem}
          onResizePreview={onResizePreview}
        />
      ) : null}
    </div>
  );
}

function getCalendarPreviewKind(itemId: string): "resize" | "move" | null {
  if (itemId.endsWith(RESIZE_PREVIEW_ID_SUFFIX)) {
    return "resize";
  }
  if (itemId.endsWith(MOVE_PREVIEW_ID_SUFFIX)) {
    return "move";
  }
  return null;
}

function getCalendarRangeSegment(
  item: WeekCalendarItem,
  displayDate: string,
  rangeBoundaryStart?: string,
  rangeBoundaryEnd?: string,
) {
  if (
    item.marker !== "scheduled" ||
    !item.endDate ||
    !isDateWithinSchedule(item, displayDate)
  ) {
    return null;
  }
  const boundaryStart = rangeBoundaryStart ?? getWeekStartDate(displayDate);
  const boundaryEnd = rangeBoundaryEnd ?? getWeekEndDate(displayDate);
  const connectsBefore = displayDate > item.date && displayDate > boundaryStart;
  const connectsAfter = displayDate < item.endDate && displayDate < boundaryEnd;
  return {
    connectsBefore,
    connectsAfter,
    showsContent: !connectsBefore,
  };
}

function isMultiDaySchedule(item: WeekCalendarItem) {
  return (
    item.marker === "scheduled" &&
    item.endDate !== null &&
    item.endDate > item.date
  );
}

function ScheduleResizeHandle({
  edge,
  isVertical,
  disabled,
  item,
  displayDate,
  onResizeItem,
  onResizePreview,
}: {
  edge: "start" | "end";
  isVertical: boolean;
  disabled: boolean;
  item: WeekCalendarItem;
  displayDate: string;
  onResizeItem(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft,
  ): Promise<boolean>;
  onResizePreview(
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft | null,
  ): void;
}) {
  const edgeLabel = edge === "start" ? "開始" : "終了";
  const directionLabel = isVertical ? "上下" : "左右";

  return (
    <button
      className={`calendar-resize-handle is-${edge} ${
        isVertical ? "is-vertical" : "is-horizontal"
      }`}
      type="button"
      aria-label={`${item.title}の${edgeLabel}を${directionLabel}に調整`}
      title={`${edgeLabel}を${directionLabel}にドラッグして調整`}
      disabled={disabled}
      onClick={(event) => event.stopPropagation()}
      onDoubleClick={(event) => event.stopPropagation()}
      onPointerDown={(event) =>
        beginScheduleResize(
          event,
          item,
          displayDate,
          edge,
          isVertical,
          onResizeItem,
          onResizePreview,
        )
      }
      onKeyDown={(event) =>
        handleScheduleResizeKeyDown(
          event,
          item,
          edge,
          isVertical,
          onResizeItem,
        )
      }
    >
      {isVertical ? (
        <GripHorizontal aria-hidden="true" size={14} strokeWidth={2} />
      ) : (
        <GripVertical aria-hidden="true" size={14} strokeWidth={2} />
      )}
    </button>
  );
}

function beginScheduleResize(
  event: ReactPointerEvent<HTMLButtonElement>,
  item: WeekCalendarItem,
  displayDate: string,
  edge: "start" | "end",
  isVertical: boolean,
  onResizeItem: (
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft,
  ) => Promise<boolean>,
  onResizePreview: (
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft | null,
  ) => void,
) {
  if (event.button !== 0) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const handle = event.currentTarget;
  const pointerId = event.pointerId;
  const originX = event.clientX;
  const originY = event.clientY;
  try {
    handle.setPointerCapture(pointerId);
  } catch {
    // WebViewがキャプチャを拒否しても、window上の監視で操作を継続する。
  }
  handle.classList.add("is-active");

  const cleanup = () => {
    handle.classList.remove("is-active");
    if (handle.hasPointerCapture(pointerId)) {
      handle.releasePointerCapture(pointerId);
    }
    window.removeEventListener("pointerup", handlePointerUp);
    window.removeEventListener("pointercancel", handlePointerCancel);
    window.removeEventListener("pointermove", handlePointerMove);
  };
  const resolveSchedule = (pointerEvent: PointerEvent) => {
    const delta = isVertical
      ? roundToQuarterHour(((pointerEvent.clientY - originY) / 54) * 60)
      : resolveDayResizeDelta(
          displayDate,
          pointerEvent.clientX,
          pointerEvent.clientY,
          pointerEvent.clientX - originX,
          handle,
        );
    return delta === 0 ? null : shiftScheduleEdge(item, edge, delta, isVertical);
  };
  const handlePointerMove = (pointerEvent: PointerEvent) => {
    onResizePreview(item, resolveSchedule(pointerEvent));
  };
  const handlePointerCancel = () => {
    onResizePreview(item, null);
    cleanup();
  };
  const handlePointerUp = (pointerEvent: PointerEvent) => {
    const schedule = resolveSchedule(pointerEvent);
    cleanup();
    if (schedule) {
      void onResizeItem(item, schedule);
    } else {
      onResizePreview(item, null);
    }
  };

  window.addEventListener("pointermove", handlePointerMove);
  window.addEventListener("pointerup", handlePointerUp, { once: true });
  window.addEventListener("pointercancel", handlePointerCancel, { once: true });
}

function handleScheduleResizeKeyDown(
  event: ReactKeyboardEvent<HTMLButtonElement>,
  item: WeekCalendarItem,
  edge: "start" | "end",
  isVertical: boolean,
  onResizeItem: (
    item: WeekCalendarItem,
    schedule: WorkScheduleDraft,
  ) => Promise<boolean>,
) {
  const delta = isVertical
    ? event.key === "ArrowUp"
      ? -15
      : event.key === "ArrowDown"
        ? 15
        : 0
    : event.key === "ArrowLeft"
      ? -1
      : event.key === "ArrowRight"
        ? 1
        : 0;
  if (delta === 0) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const schedule = shiftScheduleEdge(item, edge, delta, isVertical);
  if (schedule) {
    void onResizeItem(item, schedule);
  }
}

function shiftScheduleEdge(
  item: WeekCalendarItem,
  edge: "start" | "end",
  delta: number,
  isVertical: boolean,
): WorkScheduleDraft | null {
  if (item.marker !== "scheduled" || !item.endDate) {
    return null;
  }

  if (!isVertical) {
    const nextStartDate =
      edge === "start" ? addDaysToDateInput(item.date, delta) : item.date;
    const nextEndDate =
      edge === "end" ? addDaysToDateInput(item.endDate, delta) : item.endDate;
    if (nextStartDate > nextEndDate) {
      return null;
    }
    return {
      startDate: nextStartDate,
      startTime: item.isAllDay ? null : item.time,
      endDate: nextEndDate,
      endTime: item.isAllDay ? null : item.endTime,
      isAllDay: item.isAllDay,
    };
  }

  if (!item.time || !item.endTime) {
    return null;
  }
  const start = parseLocalDateTime(item.date, item.time);
  const end = parseLocalDateTime(item.endDate, item.endTime);
  if (edge === "start") {
    start.setMinutes(start.getMinutes() + delta);
  } else {
    end.setMinutes(end.getMinutes() + delta);
  }
  if (end.getTime() <= start.getTime()) {
    return null;
  }
  return {
    startDate: toDateInputValue(start),
    startTime: toTimeInputValue(start),
    endDate: toDateInputValue(end),
    endTime: toTimeInputValue(end),
    isAllDay: false,
  };
}

function applyScheduleToCalendarItem(
  item: WeekCalendarItem,
  schedule: WorkScheduleDraft,
): WeekCalendarItem | null {
  if (item.marker !== "scheduled") {
    return null;
  }
  return {
    ...item,
    date: schedule.startDate,
    time: schedule.startTime,
    endDate: schedule.endDate,
    endTime: schedule.endTime,
    isAllDay: schedule.isAllDay,
  };
}

function resolveDayResizeDelta(
  originDate: string,
  clientX: number,
  clientY: number,
  deltaX: number,
  handle: HTMLElement,
) {
  const target = document
    .elementFromPoint(clientX, clientY)
    ?.closest<HTMLElement>("[data-calendar-date]");
  const targetDate = target?.dataset.calendarDate;
  if (targetDate) {
    return differenceInCalendarDays(originDate, targetDate);
  }
  const cell = handle.closest<HTMLElement>("[data-calendar-date]");
  const cellWidth = cell?.getBoundingClientRect().width ?? 1;
  return Math.round(deltaX / Math.max(cellWidth, 1));
}

function roundToQuarterHour(minutes: number) {
  return Math.round(minutes / 15) * 15;
}

function handleCreateCellDoubleClick(
  event: ReactMouseEvent<HTMLElement>,
  dueDate: string,
  dueTime: string | null,
  onOpenCreateTask: (dueDate: string, dueTime: string | null) => void,
) {
  if ((event.target as HTMLElement).closest(".calendar-item")) {
    return;
  }
  onOpenCreateTask(dueDate, dueTime);
}

function handleCreateCellKeyDown(
  event: ReactKeyboardEvent<HTMLElement>,
  dueDate: string,
  dueTime: string | null,
  onOpenCreateTask: (dueDate: string, dueTime: string | null) => void,
) {
  if (event.target !== event.currentTarget || event.key !== "Enter") {
    return;
  }
  event.preventDefault();
  onOpenCreateTask(dueDate, dueTime);
}

function canMoveCalendarItem(item: WeekCalendarItem) {
  return item.marker === "due" || (item.marker === "scheduled" && !!item.endDate);
}

type CalendarCreateSelectionTarget =
  | {
      surface: "timed";
      element: HTMLElement;
      date: string;
      hour: number;
    }
  | {
      surface: "all-day" | "month";
      element: HTMLElement;
      date: string;
    };

function resolveCalendarCreateSelectionTarget(
  target: EventTarget | null,
): CalendarCreateSelectionTarget | null {
  const element = target instanceof Element ? target : null;
  if (!element || element.closest(".calendar-item, .calendar-more")) {
    return null;
  }
  const cell = element.closest<HTMLElement>(
    "[data-calendar-create-surface][data-calendar-date]",
  );
  if (!cell) {
    return null;
  }
  const date = cell.dataset.calendarDate;
  const surface = cell.dataset
    .calendarCreateSurface as CalendarCreateSelectionSurface | undefined;
  if (!date || !surface) {
    return null;
  }
  if (surface === "timed") {
    const hour = Number(cell.dataset.calendarHour);
    return Number.isInteger(hour)
      ? { surface, element: cell, date, hour }
      : null;
  }
  return surface === "all-day" || surface === "month"
    ? { surface, element: cell, date }
    : null;
}

function updateCalendarCreateSelection(
  current: CalendarCreateSelection,
  clientX: number,
  clientY: number,
  root: HTMLElement,
): CalendarCreateSelection {
  if (current.surface === "timed") {
    const didDrag =
      current.didDrag ||
      Math.abs(clientY - current.originY) >= CREATE_SELECTION_DRAG_THRESHOLD;
    if (!didDrag) {
      return current;
    }
    const cellBounds = current.captureElement.getBoundingClientRect();
    const currentMinutes = getTimeSelectionSlotMinutes(
      current.sourceHour,
      clientY,
      cellBounds.top,
      cellBounds.height,
    );
    const startMinutes = Math.min(current.anchorMinutes, currentMinutes);
    const endMinutes = Math.max(current.anchorMinutes, currentMinutes) + 15;
    if (
      current.didDrag &&
      current.startMinutes === startMinutes &&
      current.endMinutes === endMinutes
    ) {
      return current;
    }
    return updateCalendarCreateSelectionPreview(
      { ...current, didDrag: true, startMinutes, endMinutes },
      root,
    );
  }

  const didDrag =
    current.didDrag ||
    Math.abs(clientX - current.originX) >= CREATE_SELECTION_DRAG_THRESHOLD;
  if (!didDrag) {
    return current;
  }
  const hit = document.elementFromPoint(clientX, clientY);
  const target = hit?.closest<HTMLElement>(
    `[data-calendar-create-surface="${current.surface}"][data-calendar-date]`,
  );
  const targetDate = target?.dataset.calendarDate;
  if (!targetDate) {
    return current;
  }
  const startDate =
    current.anchorDate <= targetDate ? current.anchorDate : targetDate;
  const endDate =
    current.anchorDate <= targetDate ? targetDate : current.anchorDate;
  if (
    current.didDrag &&
    current.startDate === startDate &&
    current.endDate === endDate
  ) {
    return current;
  }
  return updateCalendarCreateSelectionPreview(
    { ...current, didDrag: true, startDate, endDate },
    root,
  );
}

function updateCalendarCreateSelectionPreview(
  selection: CalendarCreateSelection,
  root: HTMLElement,
  rootBounds = root.getBoundingClientRect(),
): CalendarCreateSelection {
  if (!selection.didDrag) {
    return selection;
  }
  if (selection.surface === "timed") {
    const cellBounds = selection.captureElement.getBoundingClientRect();
    const offsetMinutes = selection.startMinutes - selection.sourceHour * 60;
    const durationMinutes = selection.endMinutes - selection.startMinutes;
    return {
      ...selection,
      previewRects: [
        {
          key: `${selection.date}:${selection.startMinutes}`,
          label: `${minutesToTimeInput(selection.startMinutes)}-${minutesToTimeInput(
            selection.endMinutes,
          )}`,
          left: cellBounds.left - rootBounds.left + 6,
          top:
            cellBounds.top -
            rootBounds.top +
            (offsetMinutes / 60) * cellBounds.height,
          width: Math.max(cellBounds.width - 12, 1),
          height: Math.max((durationMinutes / 60) * cellBounds.height, 18),
          connectsBefore: false,
          connectsAfter: false,
        },
      ],
    };
  }

  const selector = `[data-calendar-create-surface="${selection.surface}"][data-calendar-date]`;
  const cellGeometries = [...root.querySelectorAll<HTMLElement>(selector)]
    .filter((cell) => {
      const date = cell.dataset.calendarDate;
      return !!date && date >= selection.startDate && date <= selection.endDate;
    })
    .sort((first, second) =>
      (first.dataset.calendarDate ?? "").localeCompare(
        second.dataset.calendarDate ?? "",
      ),
    )
    .map((cell) => ({
      date: cell.dataset.calendarDate ?? "",
      bounds: cell.getBoundingClientRect(),
    }));
  const rangeLabel =
    selection.startDate === selection.endDate
      ? "新規 終日"
      : `新規 ${formatShortDate(selection.startDate)}-${formatShortDate(
          selection.endDate,
        )}`;
  const previewRects = cellGeometries.map((geometry, index) => {
    const previous = cellGeometries[index - 1];
    const next = cellGeometries[index + 1];
    const connectsBefore =
      !!previous &&
      Math.abs(previous.bounds.top - geometry.bounds.top) <= 2 &&
      differenceInCalendarDays(previous.date, geometry.date) === 1;
    const connectsAfter =
      !!next &&
      Math.abs(next.bounds.top - geometry.bounds.top) <= 2 &&
      differenceInCalendarDays(geometry.date, next.date) === 1;
    const baseLeft = geometry.bounds.left - rootBounds.left + 6;
    const left = connectsBefore ? baseLeft - 7 : baseLeft;
    return {
      key: `${selection.surface}:${geometry.date}`,
      label: connectsBefore ? null : rangeLabel,
      left,
      top:
        geometry.bounds.top -
        rootBounds.top +
        (selection.surface === "month" ? 36 : 3),
      width:
        Math.max(geometry.bounds.width - 12, 1) +
        (connectsBefore ? 7 : 0) +
        (connectsAfter ? 7 : 0),
      height: 26,
      connectsBefore,
      connectsAfter,
    };
  });
  return { ...selection, previewRects };
}

function getTimeSelectionSlotMinutes(
  sourceHour: number,
  clientY: number,
  cellTop: number,
  cellHeight: number,
) {
  const businessStartMinutes = (businessHours[0] ?? 8) * 60;
  const businessEndMinutes = ((businessHours.at(-1) ?? 22) + 1) * 60;
  const relativeMinutes = ((clientY - cellTop) / Math.max(cellHeight, 1)) * 60;
  const slotMinutes =
    Math.floor((sourceHour * 60 + relativeMinutes) / 15) * 15;
  return Math.min(
    Math.max(slotMinutes, businessStartMinutes),
    businessEndMinutes - 15,
  );
}

function minutesToTimeInput(minutes: number) {
  const hour = Math.floor(minutes / 60);
  const minute = minutes % 60;
  return `${String(hour).padStart(2, "0")}:${String(minute).padStart(2, "0")}`;
}

function formatCalendarCreateSelectionAnnouncement(
  selection: CalendarCreateSelection,
) {
  if (selection.surface === "timed") {
    return `${formatDateLabel(selection.date)} ${minutesToTimeInput(
      selection.startMinutes,
    )}から${minutesToTimeInput(selection.endMinutes)}を選択中`;
  }
  return selection.startDate === selection.endDate
    ? `${formatDateLabel(selection.startDate)}の終日を選択中`
    : `${formatDateLabel(selection.startDate)}から${formatDateLabel(
        selection.endDate,
      )}の終日を選択中`;
}

function resolveCalendarDropTarget(
  item: WeekCalendarItem,
  zone: CalendarDropTarget,
  event: DragEvent<HTMLElement>,
): CalendarDropTarget {
  const zoneId = getDropZoneId(zone);
  if (!zone.dueTime) {
    return {
      dueDate: zone.dueDate,
      dueTime:
        item.marker === "scheduled" && !item.isAllDay ? item.time : null,
      zoneId,
    };
  }
  if (item.marker === "scheduled" && item.isAllDay) {
    return { dueDate: zone.dueDate, dueTime: null, zoneId };
  }

  const bounds = event.currentTarget.getBoundingClientRect();
  const relativeY = Math.max(
    0,
    Math.min(0.999, (event.clientY - bounds.top) / Math.max(bounds.height, 1)),
  );
  const minuteOffset = Math.min(45, Math.floor(relativeY * 4) * 15);
  const destination = addMinutesToLocalDateTime(
    zone.dueDate,
    zone.dueTime,
    minuteOffset,
  );
  return { dueDate: destination.date, dueTime: destination.time, zoneId };
}

function moveCalendarItemPreview(
  item: WeekCalendarItem,
  target: CalendarDropTarget,
): WeekCalendarItem | null {
  if (item.marker === "due") {
    return { ...item, date: target.dueDate, time: target.dueTime };
  }
  if (item.marker !== "scheduled" || !item.endDate) {
    return null;
  }

  if (item.isAllDay) {
    const dayDelta = differenceInCalendarDays(item.date, target.dueDate);
    return {
      ...item,
      date: target.dueDate,
      time: null,
      endDate: addDaysToDateInput(item.endDate, dayDelta),
      endTime: null,
    };
  }
  if (!item.time || !item.endTime || !target.dueTime) {
    return null;
  }

  const start = parseLocalDateTime(item.date, item.time);
  const end = parseLocalDateTime(item.endDate, item.endTime);
  const destinationStart = parseLocalDateTime(target.dueDate, target.dueTime);
  const destinationEnd = new Date(
    destinationStart.getTime() + (end.getTime() - start.getTime()),
  );
  return {
    ...item,
    date: target.dueDate,
    time: target.dueTime,
    endDate: toDateInputValue(destinationEnd),
    endTime: toTimeInputValue(destinationEnd),
  };
}

function getKeyboardMoveTarget(
  item: WeekCalendarItem,
  variant: CalendarItemVariant,
  key: string,
): CalendarDropTarget | null {
  if (!canMoveCalendarItem(item)) {
    return null;
  }

  if (variant === "timed" && item.time) {
    if (key === "ArrowUp" || key === "ArrowDown") {
      const destination = addMinutesToLocalDateTime(
        item.date,
        item.time,
        key === "ArrowUp" ? -15 : 15,
      );
      return { dueDate: destination.date, dueTime: destination.time };
    }
    if (key === "ArrowLeft" || key === "ArrowRight") {
      return {
        dueDate: addDaysToDateInput(item.date, key === "ArrowLeft" ? -1 : 1),
        dueTime: item.time,
      };
    }
    return null;
  }

  if (key !== "ArrowLeft" && key !== "ArrowRight") {
    return null;
  }
  return {
    dueDate: addDaysToDateInput(item.date, key === "ArrowLeft" ? -1 : 1),
    dueTime: item.marker === "scheduled" && item.isAllDay ? null : item.time,
  };
}

function isSameCalendarPosition(
  first: WeekCalendarItem,
  second: WeekCalendarItem,
) {
  return (
    first.id === second.id &&
    first.date === second.date &&
    (first.time ?? null) === (second.time ?? null) &&
    (first.endDate ?? null) === (second.endDate ?? null) &&
    (first.endTime ?? null) === (second.endTime ?? null) &&
    first.isAllDay === second.isAllDay
  );
}

function isDateWithinSchedule(item: WeekCalendarItem, date: string) {
  return (
    item.marker === "scheduled" &&
    item.endDate !== null &&
    item.date <= date &&
    date <= item.endDate
  );
}

function getTimedScheduleSegment(item: WeekCalendarItem, date: string) {
  if (
    item.marker !== "scheduled" ||
    item.isAllDay ||
    !item.time ||
    !item.endDate ||
    !item.endTime ||
    !isDateWithinSchedule(item, date)
  ) {
    return null;
  }

  const businessStartMinutes = (businessHours[0] ?? 8) * 60;
  const businessEndMinutes = ((businessHours.at(-1) ?? 22) + 1) * 60;
  const rawStartMinutes =
    date === item.date ? parseTimeInputMinutes(item.time) : businessStartMinutes;
  const rawEndMinutes =
    date === item.endDate
      ? parseTimeInputMinutes(item.endTime)
      : businessEndMinutes;
  const startMinutes = Math.max(rawStartMinutes, businessStartMinutes);
  const endMinutes = Math.min(rawEndMinutes, businessEndMinutes);
  if (endMinutes <= startMinutes) {
    return null;
  }

  return {
    startHour: Math.floor(startMinutes / 60),
    startMinutes,
    offsetMinutes: startMinutes - businessStartMinutes,
    durationMinutes: endMinutes - startMinutes,
  };
}

function formatCalendarItemMarker(item: WeekCalendarItem) {
  if (item.marker !== "scheduled" || !item.endDate) {
    return item.time
      ? `${item.time} ${markerLabels[item.marker]}`
      : markerLabels[item.marker];
  }
  if (item.isAllDay) {
    return item.date === item.endDate
      ? "終日予定"
      : `${formatShortDate(item.date)}-${formatShortDate(item.endDate)} 予定`;
  }
  const start = `${formatShortDate(item.date)} ${item.time ?? ""}`.trim();
  const end = `${formatShortDate(item.endDate)} ${item.endTime ?? ""}`.trim();
  return `${start}-${end}`;
}

function getDisplayHour(item: WeekCalendarItem) {
  if (!item.time) {
    return null;
  }

  const [hourText] = item.time.split(":");
  const hour = Number(hourText);
  if (!Number.isFinite(hour)) {
    return null;
  }

  return Math.min(Math.max(hour, businessHours[0]), businessHours.at(-1) ?? hour);
}

function formatHourInput(hour: number) {
  return `${String(hour).padStart(2, "0")}:00`;
}

function formatCreateSourceLabel(dueDate: string, dueTime: string | null) {
  return dueTime
    ? `${formatDateLabel(dueDate)} ${dueTime}`
    : formatDateLabel(dueDate);
}

function getCurrentTimeMarker(days: CalendarDay[]): CurrentTimeMarker | null {
  const now = new Date();
  const date = toDateInputValue(now);
  if (!days.some((day) => day.date === date)) {
    return null;
  }

  const hour = now.getHours();
  const firstHour = businessHours[0];
  const lastHour = businessHours.at(-1) ?? firstHour;
  if (hour < firstHour || hour > lastHour) {
    return null;
  }

  return {
    date,
    hour,
    offsetPercent: (now.getMinutes() / 60) * 100,
  };
}

function sortCalendarItems(first: WeekCalendarItem, second: WeekCalendarItem) {
  return (
    markerWeight(first.marker) - markerWeight(second.marker) ||
    (first.time ?? "").localeCompare(second.time ?? "") ||
    first.title.localeCompare(second.title, "ja")
  );
}

function markerWeight(marker: WeekCalendarItem["marker"]) {
  if (marker === "active_timer") {
    return 0;
  }
  if (marker === "scheduled") {
    return 1;
  }
  if (marker === "planned_start") {
    return 2;
  }
  return 3;
}

function isSameTarget(target: WorkTargetRef, selectedTarget: WorkTargetRef | null) {
  return (
    selectedTarget?.type === target.type && selectedTarget.id === target.id
  );
}

function isSameDropTarget(
  first: CalendarDropTarget | null,
  second: CalendarDropTarget | null,
) {
  if (!first || !second) {
    return first === second;
  }
  return (first.zoneId ?? getDropZoneId(first)) === getDropZoneId(second);
}

function isSameCalendarDestination(
  first: CalendarDropTarget | null,
  second: CalendarDropTarget,
) {
  return (
    first?.dueDate === second.dueDate &&
    (first.dueTime ?? null) === (second.dueTime ?? null)
  );
}

function getDropZoneId(target: CalendarDropTarget) {
  return `${target.dueDate}:${target.dueTime ?? "all-day"}`;
}

type CalendarDay = {
  label: string;
  date: string;
  dayOfMonth: number;
};

type CurrentTimeMarker = {
  date: string;
  hour: number;
  offsetPercent: number;
};

function buildDay(dateValue: string): CalendarDay {
  const date = parseDateInputValue(dateValue);
  return {
    label: dayLabels[(date.getDay() + 6) % 7],
    date: toDateInputValue(date),
    dayOfMonth: date.getDate(),
  };
}

function buildWeekDays(weekStartDate: string) {
  const start = parseDateInputValue(weekStartDate);
  return dayLabels.map((label, index) => {
    const date = new Date(start);
    date.setDate(start.getDate() + index);
    return {
      label,
      date: toDateInputValue(date),
      dayOfMonth: date.getDate(),
    };
  });
}

function buildMonthDays(anchorDate: string) {
  const anchor = parseDateInputValue(anchorDate);
  const firstDay = new Date(anchor.getFullYear(), anchor.getMonth(), 1);
  const lastDay = new Date(anchor.getFullYear(), anchor.getMonth() + 1, 0);
  const start = parseDateInputValue(getWeekStartDate(toDateInputValue(firstDay)));
  const end = parseDateInputValue(getWeekEndDate(toDateInputValue(lastDay)));
  const days: CalendarDay[] = [];
  const current = new Date(start);

  while (current <= end) {
    days.push(buildDay(toDateInputValue(current)));
    current.setDate(current.getDate() + 1);
  }

  return days;
}

function getWeekStartDate(value: string) {
  const date = parseDateInputValue(value);
  const mondayBasedDay = (date.getDay() + 6) % 7;
  date.setDate(date.getDate() - mondayBasedDay);
  return toDateInputValue(date);
}

function getWeekEndDate(value: string) {
  const date = parseDateInputValue(value);
  const mondayBasedDay = (date.getDay() + 6) % 7;
  date.setDate(date.getDate() + (6 - mondayBasedDay));
  return toDateInputValue(date);
}

function parseDateInputValue(value: string) {
  const [year, month, day] = value.split("-").map(Number);
  return new Date(year, month - 1, day);
}

function parseLocalDateTime(dateValue: string, timeValue: string) {
  const date = parseDateInputValue(dateValue);
  const [hour, minute] = timeValue.split(":").map(Number);
  date.setHours(hour, minute, 0, 0);
  return date;
}

function parseTimeInputMinutes(value: string) {
  const [hour, minute] = value.split(":").map(Number);
  return hour * 60 + minute;
}

function addMinutesToLocalDateTime(
  dateValue: string,
  timeValue: string,
  minutes: number,
) {
  const date = parseLocalDateTime(dateValue, timeValue);
  date.setMinutes(date.getMinutes() + minutes);
  return { date: toDateInputValue(date), time: toTimeInputValue(date) };
}

function addDaysToDateInput(value: string, days: number) {
  const date = parseDateInputValue(value);
  date.setDate(date.getDate() + days);
  return toDateInputValue(date);
}

function differenceInCalendarDays(startValue: string, endValue: string) {
  const start = parseDateInputValue(startValue);
  const end = parseDateInputValue(endValue);
  return Math.round((end.getTime() - start.getTime()) / 86_400_000);
}

function toDateInputValue(date: Date) {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function toTimeInputValue(date: Date) {
  return `${String(date.getHours()).padStart(2, "0")}:${String(
    date.getMinutes(),
  ).padStart(2, "0")}`;
}

function formatShortDate(value: string) {
  const date = parseDateInputValue(value);
  return `${date.getMonth() + 1}/${date.getDate()}`;
}

function formatCalendarHeading(viewMode: CalendarViewMode, anchorDate: string) {
  const date = parseDateInputValue(anchorDate);
  if (viewMode === "day") {
    return `${date.getFullYear()}年${date.getMonth() + 1}月${date.getDate()}日`;
  }
  return `${date.getFullYear()}年${date.getMonth() + 1}月`;
}

function formatDateLabel(value: string) {
  const date = parseDateInputValue(value);
  return `${date.getFullYear()}年${date.getMonth() + 1}月${date.getDate()}日`;
}

function formatHourLabel(hour: number) {
  if (hour < 12) {
    return `午前${hour}時`;
  }
  if (hour === 12) {
    return "午後12時";
  }
  return `午後${hour - 12}時`;
}

function formatTimeZoneOffset(date: Date) {
  const offsetMinutes = -date.getTimezoneOffset();
  const sign = offsetMinutes >= 0 ? "+" : "-";
  const absoluteMinutes = Math.abs(offsetMinutes);
  const absoluteHours = Math.floor(absoluteMinutes / 60);
  const minutes = absoluteMinutes % 60;
  if (minutes === 0) {
    return `GMT${sign}${String(absoluteHours).padStart(2, "0")}`;
  }
  return `GMT${sign}${String(absoluteHours).padStart(2, "0")}:${String(
    minutes,
  ).padStart(2, "0")}`;
}

function isToday(value: string) {
  return value === toDateInputValue(new Date());
}

function getIsoWeekNumber(value: string) {
  const date = parseDateInputValue(value);
  date.setHours(0, 0, 0, 0);
  date.setDate(date.getDate() + 3 - ((date.getDay() + 6) % 7));

  const weekOne = new Date(date.getFullYear(), 0, 4);
  return (
    1 +
    Math.round(
      ((date.getTime() - weekOne.getTime()) / 86_400_000 -
        3 +
        ((weekOne.getDay() + 6) % 7)) /
        7,
    )
  );
}
