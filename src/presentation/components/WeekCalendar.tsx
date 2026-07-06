import type { WeekCalendarItem } from "../../application/usecases/contracts";

type WeekCalendarProps = {
  weekStartDate: string;
  items: WeekCalendarItem[];
  isLoading: boolean;
  onPreviousWeek(): void;
  onNextWeek(): void;
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
  onPreviousWeek,
  onNextWeek,
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
        <div className="segmented-control" aria-label="週カレンダー移動">
          <button type="button" aria-label="前の週" onClick={onPreviousWeek}>
            ‹
          </button>
          <button type="button" aria-label="次の週" onClick={onNextWeek}>
            ›
          </button>
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
                    className={`calendar-item marker-${item.marker}`}
                    type="button"
                    key={item.id}
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
