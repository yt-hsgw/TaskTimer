import type { WeekCalendarItem } from "../../application/usecases/contracts";
import type { WorkTargetRef } from "../../domain/task/types";

type WeekCalendarProps = {
  weekStartDate: string;
  items: WeekCalendarItem[];
  isLoading: boolean;
  selectedTarget: WorkTargetRef | null;
  onPreviousWeek(): void;
  onNextWeek(): void;
  onSelectItem(item: WeekCalendarItem): void;
};

const dayLabels = ["月", "火", "水", "木", "金", "土", "日"];
const markerLabels: Record<WeekCalendarItem["marker"], string> = {
  planned_start: "開始予定",
  due: "期限",
  active_timer: "実行中",
};

export function WeekCalendar({
  weekStartDate,
  items,
  isLoading,
  selectedTarget,
  onPreviousWeek,
  onNextWeek,
  onSelectItem,
}: WeekCalendarProps) {
  const days = buildWeekDays(weekStartDate);

  return (
    <section className="panel calendar-panel" aria-labelledby="calendar-title">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">
            {formatDateLabel(days[0].date)} - {formatDateLabel(days[6].date)}
          </p>
          <h2 id="calendar-title">週カレンダー</h2>
        </div>
        <div className="calendar-heading-controls">
          <div className="calendar-view-switch" aria-label="カレンダー表示切替">
            <button className="is-active" type="button" aria-pressed="true">
              週
            </button>
            <button type="button" disabled title="今後対応">
              日
            </button>
            <button type="button" disabled title="今後対応">
              月
            </button>
          </div>
          <div className="segmented-control" aria-label="週カレンダー移動">
            <button type="button" aria-label="前の週" onClick={onPreviousWeek}>
              ‹
            </button>
            <button type="button" aria-label="次の週" onClick={onNextWeek}>
              ›
            </button>
          </div>
        </div>
      </div>

      <div className="week-grid">
        {days.map((day) => {
          const dayItems = items.filter((item) => item.date === day.date);
          return (
            <div className="day-column" key={day.date}>
              <div className="day-heading">
                <span>{day.label}</span>
                <strong>{day.dayOfMonth}</strong>
              </div>
              <div className="calendar-items">
                {isLoading ? (
                  <p className="calendar-empty">読み込み中</p>
                ) : null}
                {!isLoading && dayItems.length === 0 ? (
                  <p className="calendar-empty">予定なし</p>
                ) : null}
                {dayItems.map((item) => (
                  <button
                    className={`calendar-item marker-${item.marker} ${
                      isSameTarget(item.target, selectedTarget)
                        ? "is-selected"
                        : ""
                    }`}
                    type="button"
                    key={item.id}
                    aria-pressed={isSameTarget(item.target, selectedTarget)}
                    aria-label={`${item.title}の${markerLabels[item.marker]}を開く`}
                    onClick={() => onSelectItem(item)}
                  >
                    <span>{item.title}</span>
                    <small>{markerLabels[item.marker]}</small>
                  </button>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}

function isSameTarget(target: WorkTargetRef, selectedTarget: WorkTargetRef | null) {
  return (
    selectedTarget?.type === target.type && selectedTarget.id === target.id
  );
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

function formatDateLabel(value: string) {
  const [, month, day] = value.split("-");
  return `${Number(month)}/${Number(day)}`;
}
