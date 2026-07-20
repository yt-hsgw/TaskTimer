import { ChevronLeft, ChevronRight } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type {
  TaskListItem,
  TaskRow,
  TaskWithSubtasks,
} from "../../application/usecases/contracts";
import type { TaskColorToken } from "../../domain/task/types";
import { usePresentationRenderProbe } from "../renderProbe";

const DAY_MS = 86_400_000;
const DATE_PATTERN = /^(\d{4})-(\d{2})-(\d{2})$/;
const DAY_COLUMN_WIDTH = 64;
const WEEK_COLUMN_WIDTH = 104;
const MONTH_COLUMN_WIDTH = 112;

export type TimelineScale = "day" | "week" | "month";

type TimelinePanelProps = {
  tasks: TaskWithSubtasks[];
  taskRows: TaskRow[];
  taskLists: TaskListItem[];
  selectedTaskId: string | null;
  todayDate: string;
  isLoading: boolean;
  isLoadingMore: boolean;
  totalTaskCount: number;
  hasMoreTasks: boolean;
  onSelectTask(taskId: string): void;
  onLoadMoreTasks(): Promise<void>;
};

type TimelineColumn = {
  key: string;
  label: string;
  startDay: number;
  endDay: number;
};

type TimelineItem = {
  row: TaskRow;
  startDay: number;
  endDay: number;
  listColor: TaskColorToken;
  taskColor: TaskColorToken;
};

const scales: { value: TimelineScale; label: string }[] = [
  { value: "day", label: "日" },
  { value: "week", label: "週" },
  { value: "month", label: "月" },
];

export function TimelinePanel({
  tasks,
  taskRows,
  taskLists,
  selectedTaskId,
  todayDate,
  isLoading,
  isLoadingMore,
  totalTaskCount,
  hasMoreTasks,
  onSelectTask,
  onLoadMoreTasks,
}: TimelinePanelProps) {
  usePresentationRenderProbe("TimelinePanel");
  const normalizedToday = parseDate(todayDate) ?? currentEpochDay();
  const [scale, setScale] = useState<TimelineScale>("week");
  const [anchorDay, setAnchorDay] = useState(normalizedToday);
  const [isUnscheduledOpen, setIsUnscheduledOpen] = useState(false);
  const { columns, columnWidth, rangeLabel } = useMemo(
    () => createTimelineWindow(scale, anchorDay),
    [anchorDay, scale],
  );
  const taskById = useMemo(
    () => new Map(tasks.map((task) => [task.id, task])),
    [tasks],
  );
  const listColorById = useMemo(
    () => new Map(taskLists.map((list) => [list.id, list.colorToken])),
    [taskLists],
  );
  const { scheduledItems, unscheduledRows, outOfRangeCount } = useMemo(() => {
    const scheduled: TimelineItem[] = [];
    const unscheduled: TaskRow[] = [];
    const rangeStart = columns[0]?.startDay ?? anchorDay;
    const rangeEnd = columns.at(-1)?.endDay ?? anchorDay + 1;
    let outside = 0;

    taskRows.forEach((row) => {
      const plannedStart = parseDate(row.plannedStartDate);
      const due = parseDate(row.dueDate);
      if (plannedStart === null && due === null) {
        unscheduled.push(row);
        return;
      }
      const firstDay = plannedStart ?? due ?? rangeStart;
      const lastDay = due ?? plannedStart ?? firstDay;
      const startDay = Math.min(firstDay, lastDay);
      const endDay = Math.max(firstDay, lastDay);
      if (endDay < rangeStart || startDay >= rangeEnd) {
        outside += 1;
        return;
      }
      const listColor = listColorById.get(row.listId) ?? "green";
      scheduled.push({
        row,
        startDay,
        endDay,
        listColor,
        taskColor: taskById.get(row.id)?.colorToken ?? listColor,
      });
    });

    scheduled.sort(
      (left, right) =>
        left.startDay - right.startDay ||
        left.endDay - right.endDay ||
        left.row.title.localeCompare(right.row.title, "ja"),
    );
    return {
      scheduledItems: scheduled,
      unscheduledRows: unscheduled,
      outOfRangeCount: outside,
    };
  }, [anchorDay, columns, listColorById, taskById, taskRows]);
  const trackWidth = columns.length * columnWidth;

  useEffect(() => {
    setIsUnscheduledOpen(unscheduledRows.length > 0);
  }, [unscheduledRows.length > 0]);

  const moveRange = (direction: -1 | 1) => {
    setAnchorDay((current) => moveAnchor(current, scale, direction));
  };

  return (
    <section className="panel timeline-panel" aria-labelledby="timeline-title">
      <div className="panel-heading timeline-heading">
        <div>
          <p className="eyebrow">タイムライン</p>
          <h2 id="timeline-title">期間比較</h2>
        </div>
        <div className="timeline-heading-actions">
          <button
            type="button"
            className="calendar-today-button timeline-today-button"
            onClick={() => setAnchorDay(normalizedToday)}
          >
            今日
          </button>
          <div className="timeline-range-navigation" aria-label="表示期間の移動">
            <button
              type="button"
              className="icon-button"
              aria-label="前の期間"
              title="前の期間"
              onClick={() => moveRange(-1)}
            >
              <ChevronLeft aria-hidden="true" size={18} />
            </button>
            <button
              type="button"
              className="icon-button"
              aria-label="次の期間"
              title="次の期間"
              onClick={() => moveRange(1)}
            >
              <ChevronRight aria-hidden="true" size={18} />
            </button>
          </div>
          <strong className="timeline-range-label">{rangeLabel}</strong>
          <div className="timeline-scale-switch" role="tablist" aria-label="粒度">
            {scales.map((item) => (
              <button
                type="button"
                role="tab"
                aria-selected={scale === item.value}
                className={scale === item.value ? "is-active" : ""}
                key={item.value}
                onClick={() => setScale(item.value)}
              >
                {item.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="timeline-summary" aria-live="polite">
        <span>{`読み込み済み ${taskRows.length}/${totalTaskCount}件`}</span>
        {outOfRangeCount > 0 ? (
          <span>{`表示期間外 ${outOfRangeCount}件`}</span>
        ) : null}
      </div>

      {isLoading ? (
        <p className="empty-state">タイムラインを読み込み中です。</p>
      ) : (
        <div className="timeline-scroll" data-testid="timeline-scroll">
          <div
            className="timeline-grid"
            role="grid"
            aria-label={`${rangeLabel}のタスク期間`}
            style={{
              gridTemplateColumns: `var(--timeline-label-width) ${trackWidth}px`,
            }}
          >
            <div className="timeline-corner" role="columnheader">
              タスク
            </div>
            <div
              className="timeline-axis"
              role="row"
              style={{ gridTemplateColumns: `repeat(${columns.length}, ${columnWidth}px)` }}
            >
              {columns.map((column) => {
                const isToday =
                  normalizedToday >= column.startDay &&
                  normalizedToday < column.endDay;
                return (
                  <div
                    className={isToday ? "is-today" : ""}
                    role="columnheader"
                    key={column.key}
                  >
                    <span>{column.label}</span>
                    {isToday ? <small>今日</small> : null}
                  </div>
                );
              })}
            </div>

            {scheduledItems.map((item) => {
              const left = projectDayToPixel(item.startDay, columns, columnWidth);
              const right = projectDayToPixel(
                item.endDay + 1,
                columns,
                columnWidth,
              );
              const barWidth = Math.max(right - left, 22);
              const dateLabel = formatTaskRange(item.startDay, item.endDay);
              const isCompleted = item.row.status === "done";
              const continuesBefore = item.startDay < columns[0].startDay;
              const continuesAfter =
                item.endDay >= (columns.at(-1)?.endDay ?? item.endDay + 1);
              return (
                <div className="timeline-row" role="row" key={item.row.id}>
                  <div
                    className={`timeline-row-label ${
                      isCompleted ? "is-completed" : ""
                    }`}
                    role="rowheader"
                  >
                    <strong>{item.row.title}</strong>
                    <small>{`${statusLabel(item.row.status)} · ${dateLabel}`}</small>
                  </div>
                  <div
                    className="timeline-row-track"
                    role="gridcell"
                    style={{
                      gridTemplateColumns: `repeat(${columns.length}, ${columnWidth}px)`,
                    }}
                  >
                    {columns.map((column) => (
                      <span
                        aria-hidden="true"
                        className={
                          normalizedToday >= column.startDay &&
                          normalizedToday < column.endDay
                            ? "timeline-track-cell is-today"
                            : "timeline-track-cell"
                        }
                        key={column.key}
                      />
                    ))}
                    <button
                      type="button"
                      className={`timeline-task-bar color-${item.taskColor} list-color-${item.listColor} ${
                        selectedTaskId === item.row.id ? "is-selected" : ""
                      } ${isCompleted ? "is-completed" : ""} ${
                        barWidth < 90 ? "is-compact" : ""
                      } ${barWidth < 44 ? "is-milestone" : ""} ${
                        continuesBefore ? "continues-before" : ""
                      } ${
                        continuesAfter ? "continues-after" : ""
                      }`}
                      style={{ left, width: barWidth }}
                      aria-label={`${item.row.title}、${dateLabel}、${statusLabel(
                        item.row.status,
                      )}`}
                      title={`${item.row.title} (${dateLabel})`}
                      onClick={() => onSelectTask(item.row.id)}
                    >
                      <span>{item.row.title}</span>
                      <small>{dateLabel}</small>
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
          {scheduledItems.length === 0 ? (
            <p className="empty-state timeline-empty">
              この期間に表示できるタスクはありません。
            </p>
          ) : null}
        </div>
      )}

      <details
        className="timeline-unscheduled"
        open={isUnscheduledOpen}
        onToggle={(event) => setIsUnscheduledOpen(event.currentTarget.open)}
      >
        <summary>{`日付未設定 ${unscheduledRows.length}`}</summary>
        {unscheduledRows.length > 0 ? (
          <div className="timeline-unscheduled-list">
            {unscheduledRows.map((row) => (
              <button
                type="button"
                className={selectedTaskId === row.id ? "is-selected" : ""}
                key={row.id}
                onClick={() => onSelectTask(row.id)}
              >
                <span>{row.title}</span>
                <small>{statusLabel(row.status)}</small>
              </button>
            ))}
          </div>
        ) : (
          <p className="empty-state">日付未設定のタスクはありません。</p>
        )}
      </details>

      {hasMoreTasks ? (
        <div className="timeline-load-more">
          <button
            type="button"
            className="secondary-button"
            disabled={isLoadingMore}
            onClick={() => void onLoadMoreTasks()}
          >
            {isLoadingMore ? "読み込み中..." : "さらに読み込む"}
          </button>
        </div>
      ) : null}
    </section>
  );
}

function createTimelineWindow(scale: TimelineScale, anchorDay: number) {
  if (scale === "day") {
    const columns = Array.from({ length: 14 }, (_, index) => {
      const startDay = anchorDay + index;
      return {
        key: formatDate(startDay),
        label: formatDayLabel(startDay),
        startDay,
        endDay: startDay + 1,
      };
    });
    return {
      columns,
      columnWidth: DAY_COLUMN_WIDTH,
      rangeLabel: formatWindowLabel(columns),
    };
  }

  if (scale === "week") {
    const firstDay = startOfWeek(anchorDay);
    const columns = Array.from({ length: 12 }, (_, index) => {
      const startDay = firstDay + index * 7;
      return {
        key: formatDate(startDay),
        label: `${formatMonthDay(startDay)}週`,
        startDay,
        endDay: startDay + 7,
      };
    });
    return {
      columns,
      columnWidth: WEEK_COLUMN_WIDTH,
      rangeLabel: formatWindowLabel(columns),
    };
  }

  const firstMonth = startOfMonth(anchorDay);
  const columns = Array.from({ length: 12 }, (_, index) => {
    const startDay = addMonths(firstMonth, index);
    return {
      key: formatDate(startDay),
      label: formatMonthLabel(startDay),
      startDay,
      endDay: addMonths(firstMonth, index + 1),
    };
  });
  return {
    columns,
    columnWidth: MONTH_COLUMN_WIDTH,
    rangeLabel: formatWindowLabel(columns),
  };
}

function moveAnchor(day: number, scale: TimelineScale, direction: -1 | 1) {
  if (scale === "day") {
    return day + direction * 14;
  }
  if (scale === "week") {
    return day + direction * 12 * 7;
  }
  return addMonths(day, direction * 12);
}

function projectDayToPixel(
  day: number,
  columns: TimelineColumn[],
  columnWidth: number,
) {
  if (columns.length === 0 || day <= columns[0].startDay) {
    return 0;
  }
  const lastColumn = columns.at(-1);
  if (!lastColumn || day >= lastColumn.endDay) {
    return columns.length * columnWidth;
  }
  const index = columns.findIndex(
    (column) => day >= column.startDay && day < column.endDay,
  );
  if (index < 0) {
    return 0;
  }
  const column = columns[index];
  const ratio = (day - column.startDay) / (column.endDay - column.startDay);
  return index * columnWidth + ratio * columnWidth;
}

function parseDate(value: string | null | undefined) {
  if (!value) {
    return null;
  }
  const match = DATE_PATTERN.exec(value);
  if (!match) {
    return null;
  }
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const date = new Date(Date.UTC(year, month - 1, day));
  if (
    date.getUTCFullYear() !== year ||
    date.getUTCMonth() !== month - 1 ||
    date.getUTCDate() !== day
  ) {
    return null;
  }
  return Math.floor(date.getTime() / DAY_MS);
}

function currentEpochDay() {
  const now = new Date();
  return Math.floor(Date.UTC(now.getFullYear(), now.getMonth(), now.getDate()) / DAY_MS);
}

function dateFromEpochDay(day: number) {
  return new Date(day * DAY_MS);
}

function formatDate(day: number) {
  const date = dateFromEpochDay(day);
  return `${date.getUTCFullYear()}-${String(date.getUTCMonth() + 1).padStart(
    2,
    "0",
  )}-${String(date.getUTCDate()).padStart(2, "0")}`;
}

function formatMonthDay(day: number) {
  const date = dateFromEpochDay(day);
  return `${date.getUTCMonth() + 1}/${date.getUTCDate()}`;
}

function formatDayLabel(day: number) {
  const date = dateFromEpochDay(day);
  const weekdays = ["日", "月", "火", "水", "木", "金", "土"];
  return `${formatMonthDay(day)} ${weekdays[date.getUTCDay()]}`;
}

function formatMonthLabel(day: number) {
  const date = dateFromEpochDay(day);
  return `${date.getUTCFullYear()}年${date.getUTCMonth() + 1}月`;
}

function formatWindowLabel(columns: TimelineColumn[]) {
  const first = columns[0];
  const last = columns.at(-1);
  if (!first || !last) {
    return "表示期間なし";
  }
  return `${formatYearMonthDay(first.startDay)} - ${formatYearMonthDay(
    last.endDay - 1,
  )}`;
}

function formatTaskRange(startDay: number, endDay: number) {
  if (startDay === endDay) {
    return formatMonthDay(startDay);
  }
  return `${formatMonthDay(startDay)} - ${formatMonthDay(endDay)}`;
}

function formatYearMonthDay(day: number) {
  const date = dateFromEpochDay(day);
  return `${date.getUTCFullYear()}年${date.getUTCMonth() + 1}/${date.getUTCDate()}`;
}

function startOfWeek(day: number) {
  const weekday = dateFromEpochDay(day).getUTCDay();
  return day - ((weekday + 6) % 7);
}

function startOfMonth(day: number) {
  const date = dateFromEpochDay(day);
  return Math.floor(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), 1) / DAY_MS);
}

function addMonths(day: number, amount: number) {
  const date = dateFromEpochDay(day);
  return Math.floor(
    Date.UTC(date.getUTCFullYear(), date.getUTCMonth() + amount, 1) / DAY_MS,
  );
}

function statusLabel(status: TaskRow["status"]) {
  switch (status) {
    case "in_progress":
      return "進行中";
    case "done":
      return "完了";
    case "archived":
      return "アーカイブ";
    default:
      return "未着手";
  }
}
