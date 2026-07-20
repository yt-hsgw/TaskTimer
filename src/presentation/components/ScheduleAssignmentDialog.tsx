import { CalendarClock, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { FormEvent } from "react";
import type {
  TaskRow,
  WorkScheduleDraft,
} from "../../application/usecases/contracts";

type ScheduleAssignmentDialogProps = {
  task: TaskRow;
  initialDate: string;
  isPending: boolean;
  onClose(): void;
  onSubmit(schedule: WorkScheduleDraft): Promise<boolean>;
};

export function ScheduleAssignmentDialog({
  task,
  initialDate,
  isPending,
  onClose,
  onSubmit,
}: ScheduleAssignmentDialogProps) {
  const dateInputRef = useRef<HTMLInputElement>(null);
  const [date, setDate] = useState(initialDate);
  const [isAllDay, setIsAllDay] = useState(true);
  const [startTime, setStartTime] = useState("09:00");

  useEffect(() => {
    dateInputRef.current?.focus();
  }, []);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape" && !isPending) {
        onClose();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isPending, onClose]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!date) {
      return;
    }
    const schedule = isAllDay
      ? createAllDaySchedule(date)
      : createTimedSchedule(date, startTime);
    if (schedule && (await onSubmit(schedule))) {
      onClose();
    }
  }

  return (
    <div className="schedule-dialog-backdrop" role="presentation">
      <section
        aria-labelledby="schedule-dialog-title"
        aria-modal="true"
        className="schedule-dialog"
        role="dialog"
      >
        <header>
          <div>
            <CalendarClock aria-hidden="true" size={18} />
            <div>
              <p>予定を設定</p>
              <h2 id="schedule-dialog-title">{task.title}</h2>
            </div>
          </div>
          <button
            aria-label="予定設定を閉じる"
            className="icon-button"
            disabled={isPending}
            title="閉じる"
            type="button"
            onClick={onClose}
          >
            <X aria-hidden="true" size={18} />
          </button>
        </header>
        <form onSubmit={handleSubmit}>
          <label>
            <span>日付</span>
            <input
              ref={dateInputRef}
              required
              type="date"
              value={date}
              onChange={(event) => setDate(event.target.value)}
            />
          </label>
          <label className="schedule-dialog-all-day">
            <input
              checked={isAllDay}
              type="checkbox"
              onChange={(event) => setIsAllDay(event.target.checked)}
            />
            <span>終日</span>
          </label>
          {!isAllDay ? (
            <label>
              <span>開始時刻</span>
              <input
                required
                step={900}
                type="time"
                value={startTime}
                onChange={(event) => setStartTime(event.target.value)}
              />
            </label>
          ) : null}
          <div className="schedule-dialog-actions">
            <button
              className="secondary-button"
              disabled={isPending}
              type="button"
              onClick={onClose}
            >
              キャンセル
            </button>
            <button className="primary-button" disabled={isPending} type="submit">
              {isPending ? "保存中..." : "予定を設定"}
            </button>
          </div>
        </form>
      </section>
    </div>
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

function createTimedSchedule(
  date: string,
  inputTime: string,
): WorkScheduleDraft | null {
  const match = /^(\d{2}):(\d{2})$/.exec(inputTime);
  if (!match) {
    return null;
  }
  const hours = Number(match[1]);
  const minutes = Math.floor(Number(match[2]) / 15) * 15;
  if (hours > 23 || minutes > 59) {
    return null;
  }
  const startTime = `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
  const start = new Date(`${date}T${startTime}:00`);
  if (Number.isNaN(start.getTime())) {
    return null;
  }
  const end = new Date(start.getTime() + 60 * 60 * 1000);
  return {
    startDate: date,
    startTime,
    endDate: formatLocalDate(end),
    endTime: `${String(end.getHours()).padStart(2, "0")}:${String(
      end.getMinutes(),
    ).padStart(2, "0")}`,
    isAllDay: false,
  };
}

function formatLocalDate(date: Date) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(
    2,
    "0",
  )}-${String(date.getDate()).padStart(2, "0")}`;
}
