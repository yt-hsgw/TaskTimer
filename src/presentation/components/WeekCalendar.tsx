import type { WeekCalendarItem } from "../../application/usecases/contracts";
import type { WorkTargetRef } from "../../domain/task/types";

export type CalendarViewMode = "week" | "day" | "month";

type WeekCalendarProps = {
  viewMode: CalendarViewMode;
  anchorDate: string;
  items: WeekCalendarItem[];
  isLoading: boolean;
  selectedTarget: WorkTargetRef | null;
  onChangeViewMode(viewMode: CalendarViewMode): void;
  onPreviousRange(): void;
  onNextRange(): void;
  onToday(): void;
  onSelectItem(item: WeekCalendarItem): void;
};

const dayLabels = ["月", "火", "水", "木", "金", "土", "日"];
const businessHours = Array.from({ length: 10 }, (_, index) => 9 + index);
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

export function WeekCalendar({
  viewMode,
  anchorDate,
  items,
  isLoading,
  selectedTarget,
  onChangeViewMode,
  onPreviousRange,
  onNextRange,
  onToday,
  onSelectItem,
}: WeekCalendarProps) {
  const rangeDays =
    viewMode === "day"
      ? [buildDay(anchorDate)]
      : buildWeekDays(getWeekStartDate(anchorDate));
  const monthDays = buildMonthDays(anchorDate);
  const headingLabel =
    viewMode === "month"
      ? formatMonthHeading(anchorDate)
      : formatRangeHeading(rangeDays);

  return (
    <section className="panel calendar-panel" aria-labelledby="calendar-title">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">{headingLabel}</p>
          <h2 id="calendar-title">カレンダー</h2>
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
          <button
            className="calendar-today-button"
            type="button"
            onClick={onToday}
          >
            今日
          </button>
          <div className="segmented-control" aria-label="カレンダー移動">
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
        </div>
      </div>

      {viewMode === "month" ? (
        <MonthCalendar
          days={monthDays}
          anchorDate={anchorDate}
          items={items}
          isLoading={isLoading}
          selectedTarget={selectedTarget}
          onSelectItem={onSelectItem}
        />
      ) : (
        <TimeGridCalendar
          days={rangeDays}
          items={items}
          isLoading={isLoading}
          selectedTarget={selectedTarget}
          viewMode={viewMode}
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
  onSelectItem,
}: {
  days: CalendarDay[];
  items: WeekCalendarItem[];
  isLoading: boolean;
  selectedTarget: WorkTargetRef | null;
  viewMode: CalendarViewMode;
  onSelectItem(item: WeekCalendarItem): void;
}) {
  return (
    <div
      className={`calendar-time-grid ${
        viewMode === "day" ? "is-day-mode" : ""
      }`}
    >
      <div className="calendar-time-label" aria-hidden="true" />
      {days.map((day) => (
        <div className="calendar-time-header" key={day.date}>
          <span>{day.label}</span>
          <strong>{day.dayOfMonth}</strong>
        </div>
      ))}

      <div className="calendar-time-label">日付のみ</div>
      {days.map((day) => {
        const dateOnlyItems = items
          .filter((item) => item.date === day.date && !item.time)
          .sort(sortCalendarItems);
        return (
          <div className="calendar-all-day-cell" key={`${day.date}:all-day`}>
            <CalendarCellItems
              emptyLabel="予定なし"
              isLoading={isLoading}
              items={dateOnlyItems}
              selectedTarget={selectedTarget}
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
          isLoading={isLoading}
          selectedTarget={selectedTarget}
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
  isLoading,
  selectedTarget,
  onSelectItem,
}: {
  hour: number;
  days: CalendarDay[];
  items: WeekCalendarItem[];
  isLoading: boolean;
  selectedTarget: WorkTargetRef | null;
  onSelectItem(item: WeekCalendarItem): void;
}) {
  return (
    <>
      <div className="calendar-time-label">{hour}:00</div>
      {days.map((day) => {
        const hourItems = items
          .filter((item) => item.date === day.date && getDisplayHour(item) === hour)
          .sort(sortCalendarItems);
        return (
          <div className="calendar-time-cell" key={`${day.date}:${hour}`}>
            <CalendarCellItems
              emptyLabel=""
              isLoading={isLoading}
              items={hourItems}
              selectedTarget={selectedTarget}
              onSelectItem={onSelectItem}
            />
          </div>
        );
      })}
    </>
  );
}

function CalendarCellItems({
  emptyLabel,
  isLoading,
  items,
  selectedTarget,
  onSelectItem,
}: {
  emptyLabel: string;
  isLoading: boolean;
  items: WeekCalendarItem[];
  selectedTarget: WorkTargetRef | null;
  onSelectItem(item: WeekCalendarItem): void;
}) {
  if (isLoading) {
    return <p className="calendar-empty">読み込み中</p>;
  }

  if (items.length === 0) {
    return emptyLabel ? <p className="calendar-empty">{emptyLabel}</p> : null;
  }

  return (
    <div className="calendar-items">
      {items.map((item) => (
        <CalendarItemButton
          item={item}
          key={item.id}
          selectedTarget={selectedTarget}
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
  onSelectItem,
}: {
  days: CalendarDay[];
  anchorDate: string;
  items: WeekCalendarItem[];
  isLoading: boolean;
  selectedTarget: WorkTargetRef | null;
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
              }`}
              key={day.date}
            >
              <div className="calendar-month-day-heading">
                <span>{day.dayOfMonth}</span>
              </div>
              {isLoading ? (
                <p className="calendar-empty">読み込み中</p>
              ) : (
                <div className="calendar-month-day-items">
                  {visibleItems.map((item) => (
                    <CalendarItemButton
                      compact
                      item={item}
                      key={item.id}
                      selectedTarget={selectedTarget}
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
  compact = false,
  selectedTarget,
  onSelectItem,
}: {
  item: WeekCalendarItem;
  compact?: boolean;
  selectedTarget: WorkTargetRef | null;
  onSelectItem(item: WeekCalendarItem): void;
}) {
  const isSelected = isSameTarget(item.target, selectedTarget);
  const relationLabel = item.parentTitle ? `親: ${item.parentTitle}` : null;
  const markerText = item.time
    ? `${item.time} ${markerLabels[item.marker]}`
    : markerLabels[item.marker];

  return (
    <button
      className={`calendar-item marker-${item.marker} ${
        item.target.type === "subtask" ? "is-subtask" : ""
      } ${item.status === "done" ? "is-done" : ""} ${
        isSelected ? "is-selected" : ""
      } ${compact ? "is-compact" : ""}`}
      type="button"
      aria-pressed={isSelected}
      aria-label={`${relationLabel ? `${relationLabel}、` : ""}${item.title}の${markerText}を開く`}
      onClick={() => onSelectItem(item)}
    >
      <span className="calendar-item-title">{item.title}</span>
      {relationLabel ? (
        <small className="calendar-item-parent">{relationLabel}</small>
      ) : null}
      <small>{markerText}</small>
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

function formatRangeHeading(days: CalendarDay[]) {
  const firstDay = days[0];
  const lastDay = days.at(-1);
  if (!firstDay || !lastDay || firstDay.date === lastDay.date) {
    return firstDay ? formatDateLabel(firstDay.date) : "";
  }
  return `${formatDateLabel(firstDay.date)} - ${formatDateLabel(lastDay.date)}`;
}

function formatMonthHeading(value: string) {
  const date = parseDateInputValue(value);
  return `${date.getFullYear()}年${date.getMonth() + 1}月`;
}

function formatDateLabel(value: string) {
  const [, month, day] = value.split("-");
  return `${Number(month)}/${Number(day)}`;
}
