import type { WeekCalendarItem } from "../../application/usecases/contracts";

type WeekCalendarProps = {
  weekStartDate: string;
  items: WeekCalendarItem[];
};

const days = ["月", "火", "水", "木", "金", "土", "日"];
const markerLabels: Record<WeekCalendarItem["marker"], string> = {
  planned_start: "開始予定",
  due: "期限",
  active_timer: "実行中",
};

export function WeekCalendar({ weekStartDate, items }: WeekCalendarProps) {
  return (
    <section className="panel calendar-panel" aria-labelledby="calendar-title">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">{weekStartDate}</p>
          <h2 id="calendar-title">週カレンダー</h2>
        </div>
        <div className="segmented-control" aria-label="週カレンダー移動">
          <button type="button" aria-label="前の週">
            ‹
          </button>
          <button type="button" aria-label="次の週">
            ›
          </button>
        </div>
      </div>

      <div className="week-grid">
        {days.map((day, index) => {
          const dayItems = items.filter((item) => item.date.endsWith(`0${6 + index}`));
          return (
            <div className="day-column" key={day}>
              <div className="day-heading">
                <span>{day}</span>
                <strong>{6 + index}</strong>
              </div>
              <div className="calendar-items">
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
