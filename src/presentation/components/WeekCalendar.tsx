import { FormEvent, useEffect, useMemo, useRef, useState } from "react";
import type {
  TaskListItem,
  WeekCalendarItem,
  WorkItemDraft,
} from "../../application/usecases/contracts";
import type { WorkTargetRef } from "../../domain/task/types";

export type CalendarViewMode = "week" | "day" | "month";

type WeekCalendarProps = {
  viewMode: CalendarViewMode;
  anchorDate: string;
  items: WeekCalendarItem[];
  taskLists: TaskListItem[];
  defaultTaskListId: string;
  isLoading: boolean;
  isCreatingTaskPending: boolean;
  selectedTarget: WorkTargetRef | null;
  onChangeViewMode(viewMode: CalendarViewMode): void;
  onPreviousRange(): void;
  onNextRange(): void;
  onToday(): void;
  onSelectItem(item: WeekCalendarItem): void;
  onCreateTask(input: WorkItemDraft): Promise<boolean>;
};

const dayLabels = ["月", "火", "水", "木", "金", "土", "日"];
const businessHours = Array.from({ length: 15 }, (_, index) => 8 + index);
const markerLabels: Record<WeekCalendarItem["marker"], string> = {
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
  dueDate: string;
  dueTime: string;
  memo: string;
  sourceLabel: string;
};

export function WeekCalendar({
  viewMode,
  anchorDate,
  items,
  taskLists,
  defaultTaskListId,
  isLoading,
  isCreatingTaskPending,
  selectedTarget,
  onChangeViewMode,
  onPreviousRange,
  onNextRange,
  onToday,
  onSelectItem,
  onCreateTask,
}: WeekCalendarProps) {
  const titleInputRef = useRef<HTMLInputElement>(null);
  const [createDraft, setCreateDraft] = useState<CalendarTaskDraft | null>(null);
  const rangeDays =
    viewMode === "day"
      ? [buildDay(anchorDate)]
      : buildWeekDays(getWeekStartDate(anchorDate));
  const monthDays = buildMonthDays(anchorDate);
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
    if (!createDraft) {
      return;
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setCreateDraft(null);
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [createDraft]);

  function openCreateForm(dueDate: string, dueTime: string | null) {
    setCreateDraft({
      title: "",
      listId: fallbackListId,
      dueDate,
      dueTime: dueTime ?? "",
      memo: "",
      sourceLabel: formatCreateSourceLabel(dueDate, dueTime),
    });
  }

  async function handleCreateTask(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!createDraft) {
      return;
    }

    const dueDate = normalizeOptionalText(createDraft.dueDate);
    const created = await onCreateTask({
      title: createDraft.title,
      listId: createDraft.listId || fallbackListId,
      plannedStartDate: null,
      dueDate,
      dueTime: dueDate ? normalizeOptionalText(createDraft.dueTime) : null,
      memo: createDraft.memo,
    });
    if (created) {
      setCreateDraft(null);
    }
  }

  return (
    <section className="panel calendar-panel" aria-labelledby="calendar-title">
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
          <button
            className="calendar-add-task-button"
            type="button"
            aria-label={`${formatDateLabel(anchorDate)}にタスクを追加`}
            title="タスクを追加"
            disabled={isCreatingTaskPending}
            onClick={() => openCreateForm(anchorDate, null)}
          >
            ＋
          </button>
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
                <span>期限日</span>
                <input
                  type="date"
                  value={createDraft.dueDate}
                  onChange={(event) =>
                    setCreateDraft((current) =>
                      current
                        ? {
                            ...current,
                            dueDate: event.target.value,
                            dueTime: event.target.value ? current.dueTime : "",
                          }
                        : current,
                    )
                  }
                  disabled={isCreatingTaskPending}
                />
              </label>
              <label>
                <span>期限時刻</span>
                <input
                  type="time"
                  value={createDraft.dueTime}
                  onChange={(event) =>
                    setCreateDraft((current) =>
                      current ? { ...current, dueTime: event.target.value } : current,
                    )
                  }
                  disabled={isCreatingTaskPending || !createDraft.dueDate}
                />
              </label>
            </div>

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
          items={items}
          isLoading={isLoading}
          selectedTarget={selectedTarget}
          onOpenCreateTask={openCreateForm}
          onSelectItem={onSelectItem}
        />
      ) : (
        <TimeGridCalendar
          days={rangeDays}
          items={items}
          isLoading={isLoading}
          selectedTarget={selectedTarget}
          viewMode={viewMode}
          onOpenCreateTask={openCreateForm}
          onSelectItem={onSelectItem}
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
  onOpenCreateTask,
  onSelectItem,
}: {
  days: CalendarDay[];
  items: WeekCalendarItem[];
  isLoading: boolean;
  selectedTarget: WorkTargetRef | null;
  viewMode: CalendarViewMode;
  onOpenCreateTask(dueDate: string, dueTime: string | null): void;
  onSelectItem(item: WeekCalendarItem): void;
}) {
  const currentTime = getCurrentTimeMarker(days);

  return (
    <div
      className={`calendar-time-grid ${
        viewMode === "day" ? "is-day-mode" : ""
      }`}
    >
      <div className="calendar-time-zone">{formatTimeZoneOffset(new Date())}</div>
      {days.map((day) => (
        <div
          className={`calendar-time-header ${isToday(day.date) ? "is-today" : ""}`}
          key={day.date}
        >
          <span>{day.label}</span>
          <strong>{day.dayOfMonth}</strong>
        </div>
      ))}

      <div className="calendar-time-label">終日</div>
      {days.map((day) => {
        const dateOnlyItems = items
          .filter((item) => item.date === day.date && !item.time)
          .sort(sortCalendarItems);
        return (
          <div className="calendar-all-day-cell" key={`${day.date}:all-day`}>
            <CalendarAddButton
              dueDate={day.date}
              dueTime={null}
              onOpenCreateTask={onOpenCreateTask}
            />
            <CalendarCellItems
              isLoading={isLoading}
              items={dateOnlyItems}
              selectedTarget={selectedTarget}
              variant="all-day"
              onSelectItem={onSelectItem}
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
          onOpenCreateTask={onOpenCreateTask}
          onSelectItem={onSelectItem}
        />
      ))}
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
  onOpenCreateTask,
  onSelectItem,
}: {
  hour: number;
  days: CalendarDay[];
  items: WeekCalendarItem[];
  currentTime: CurrentTimeMarker | null;
  isLoading: boolean;
  selectedTarget: WorkTargetRef | null;
  onOpenCreateTask(dueDate: string, dueTime: string | null): void;
  onSelectItem(item: WeekCalendarItem): void;
}) {
  return (
    <>
      <div className="calendar-time-label">{formatHourLabel(hour)}</div>
      {days.map((day) => {
        const hourItems = items
          .filter((item) => item.date === day.date && getDisplayHour(item) === hour)
          .sort(sortCalendarItems);
        const shouldShowCurrentTime =
          currentTime?.date === day.date && currentTime.hour === hour;
        return (
          <div
            className={`calendar-time-cell ${
              shouldShowCurrentTime ? "has-current-time" : ""
            }`}
            key={`${day.date}:${hour}`}
          >
            <CalendarAddButton
              dueDate={day.date}
              dueTime={formatHourInput(hour)}
              onOpenCreateTask={onOpenCreateTask}
            />
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
              variant="timed"
              onSelectItem={onSelectItem}
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
  selectedTarget,
  variant,
  onSelectItem,
}: {
  isLoading: boolean;
  items: WeekCalendarItem[];
  selectedTarget: WorkTargetRef | null;
  variant: "all-day" | "timed";
  onSelectItem(item: WeekCalendarItem): void;
}) {
  if (isLoading) {
    return <p className="calendar-empty">読み込み中</p>;
  }

  if (items.length === 0) {
    return null;
  }

  return (
    <div className={`calendar-items is-${variant}`}>
      {items.map((item) => (
        <CalendarItemButton
          item={item}
          key={item.id}
          selectedTarget={selectedTarget}
          variant={variant}
          onSelectItem={onSelectItem}
        />
      ))}
    </div>
  );
}

function MonthCalendar({
  days,
  anchorDate,
  items,
  isLoading,
  selectedTarget,
  onOpenCreateTask,
  onSelectItem,
}: {
  days: CalendarDay[];
  anchorDate: string;
  items: WeekCalendarItem[];
  isLoading: boolean;
  selectedTarget: WorkTargetRef | null;
  onOpenCreateTask(dueDate: string, dueTime: string | null): void;
  onSelectItem(item: WeekCalendarItem): void;
}) {
  const anchor = parseDateInputValue(anchorDate);
  const currentMonth = anchor.getMonth();

  return (
    <div className="calendar-month-shell">
      <div className="calendar-month-weekdays" aria-hidden="true">
        {dayLabels.map((label) => (
          <span key={label}>{label}</span>
        ))}
      </div>
      <div className="calendar-month-grid">
        {days.map((day) => {
          const dayItems = items
            .filter((item) => item.date === day.date)
            .sort(sortCalendarItems);
          const visibleItems = dayItems.slice(0, 3);
          const hiddenCount = dayItems.length - visibleItems.length;
          const isOutsideMonth =
            parseDateInputValue(day.date).getMonth() !== currentMonth;
          return (
            <div
              className={`calendar-month-day ${
                isOutsideMonth ? "is-outside-month" : ""
              } ${isToday(day.date) ? "is-today" : ""}`}
              key={day.date}
            >
              <div className="calendar-month-day-heading">
                <CalendarAddButton
                  dueDate={day.date}
                  dueTime={null}
                  variant="month"
                  onOpenCreateTask={onOpenCreateTask}
                />
                <span>{day.dayOfMonth}</span>
              </div>
              {isLoading ? (
                <p className="calendar-empty">読み込み中</p>
              ) : (
                <div className="calendar-month-day-items">
                  {visibleItems.map((item) => (
                    <CalendarItemButton
                      item={item}
                      key={item.id}
                      selectedTarget={selectedTarget}
                      variant="month"
                      onSelectItem={onSelectItem}
                    />
                  ))}
                  {hiddenCount > 0 ? (
                    <span className="calendar-more">他 {hiddenCount} 件</span>
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

function CalendarItemButton({
  item,
  selectedTarget,
  variant,
  onSelectItem,
}: {
  item: WeekCalendarItem;
  selectedTarget: WorkTargetRef | null;
  variant: "all-day" | "timed" | "month";
  onSelectItem(item: WeekCalendarItem): void;
}) {
  const isSelected = isSameTarget(item.target, selectedTarget);
  const relationLabel = item.parentTitle ? `親: ${item.parentTitle}` : null;
  const markerText = item.time
    ? `${item.time} ${markerLabels[item.marker]}`
    : markerLabels[item.marker];

  return (
    <button
      className={`calendar-item marker-${item.marker} is-${variant} ${
        item.target.type === "subtask" ? "is-subtask" : ""
      } ${item.status === "done" ? "is-done" : ""} ${
        isSelected ? "is-selected" : ""
      }`}
      type="button"
      aria-pressed={isSelected}
      aria-label={`${relationLabel ? `${relationLabel}、` : ""}${item.title}の${markerText}を開く`}
      onClick={(event) => {
        event.stopPropagation();
        onSelectItem(item);
      }}
    >
      <span className="calendar-item-title">{item.title}</span>
      {relationLabel ? (
        <small className="calendar-item-parent">{relationLabel}</small>
      ) : null}
      {variant === "timed" || variant === "month" ? (
        <small>{markerText}</small>
      ) : null}
    </button>
  );
}

function CalendarAddButton({
  dueDate,
  dueTime,
  variant = "cell",
  onOpenCreateTask,
}: {
  dueDate: string;
  dueTime: string | null;
  variant?: "cell" | "month";
  onOpenCreateTask(dueDate: string, dueTime: string | null): void;
}) {
  const label = `${formatCreateSourceLabel(dueDate, dueTime)}にタスクを追加`;
  return (
    <button
      className={`calendar-cell-add-button is-${variant}`}
      type="button"
      aria-label={label}
      title="タスクを追加"
      onClick={(event) => {
        event.stopPropagation();
        onOpenCreateTask(dueDate, dueTime);
      }}
    >
      ＋
    </button>
  );
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

function normalizeOptionalText(value: string | null | undefined) {
  if (!value) {
    return null;
  }
  return value;
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
  if (marker === "planned_start") {
    return 1;
  }
  return 2;
}

function isSameTarget(target: WorkTargetRef, selectedTarget: WorkTargetRef | null) {
  return (
    selectedTarget?.type === target.type && selectedTarget.id === target.id
  );
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

function toDateInputValue(date: Date) {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
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
