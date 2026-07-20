import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type FormEvent,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import { CalendarDays, X } from "lucide-react";
import type { TaskListItem } from "../../application/usecases/contracts";
import type {
  TaskCreatePreset,
  TaskCreateSubmission,
} from "../taskCreate";

type TaskCreateDialogProps = {
  preset: TaskCreatePreset;
  taskLists: TaskListItem[];
  isSubmitting: boolean;
  errorMessage: string | null;
  onSubmit(submission: TaskCreateSubmission): Promise<boolean>;
  onClose(): void;
};

type TaskCreateDraft = {
  title: string;
  listId: string;
  memo: string;
  dueDate: string;
  dueTime: string;
  startDate: string;
  startTime: string;
  endDate: string;
  endTime: string;
  isAllDay: boolean;
};

const focusableSelector = [
  "button:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  "textarea:not([disabled])",
].join(",");

export function TaskCreateDialog({
  preset,
  taskLists,
  isSubmitting,
  errorMessage,
  onSubmit,
  onClose,
}: TaskCreateDialogProps) {
  const initialDraft = useMemo(() => createDraftFromPreset(preset), [preset]);
  const [draft, setDraft] = useState(initialDraft);
  const dialogRef = useRef<HTMLElement>(null);
  const titleInputRef = useRef<HTMLInputElement>(null);
  const openerRef = useRef<HTMLElement | null>(null);
  const shouldRestoreTaskCreateTriggerRef = useRef(false);
  const isDirty = !isSameTaskCreateDraft(draft, initialDraft);

  useEffect(() => {
    setDraft(initialDraft);
  }, [initialDraft]);

  useEffect(() => {
    openerRef.current =
      document.activeElement instanceof HTMLElement ? document.activeElement : null;
    shouldRestoreTaskCreateTriggerRef.current = Boolean(
      openerRef.current?.closest("[data-task-create-trigger]"),
    );
    const focusFrame = window.requestAnimationFrame(() => {
      titleInputRef.current?.focus();
    });
    return () => {
      window.cancelAnimationFrame(focusFrame);
      window.requestAnimationFrame(() => {
        const opener = openerRef.current;
        const fallback = shouldRestoreTaskCreateTriggerRef.current
          ? document.querySelector<HTMLElement>("[data-task-create-trigger]")
          : null;
        (opener?.isConnected ? opener : fallback)?.focus();
      });
    };
  }, []);

  const requestImplicitClose = useCallback(() => {
    if (isSubmitting) {
      return;
    }
    if (
      isDirty &&
      !window.confirm("入力中の内容を破棄してタスク作成を閉じますか？")
    ) {
      return;
    }
    onClose();
  }, [isDirty, isSubmitting, onClose]);

  useEffect(() => {
    function handleDocumentKeyDown(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        requestImplicitClose();
      }
    }

    document.addEventListener("keydown", handleDocumentKeyDown);
    return () => document.removeEventListener("keydown", handleDocumentKeyDown);
  }, [requestImplicitClose]);

  function handleDialogKeyDown(event: KeyboardEvent<HTMLElement>) {
    if (event.key !== "Tab") {
      return;
    }
    const focusable = dialogRef.current?.querySelectorAll<HTMLElement>(
      focusableSelector,
    );
    if (!focusable || focusable.length === 0) {
      event.preventDefault();
      return;
    }
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last?.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first?.focus();
    }
  }

  function handleBackdropPointerDown(event: PointerEvent<HTMLDivElement>) {
    if (event.target === event.currentTarget) {
      requestImplicitClose();
    }
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (isSubmitting) {
      return;
    }

    const submission: TaskCreateSubmission =
      preset.kind === "scheduled"
        ? {
            kind: "scheduled",
            input: {
              title: draft.title,
              listId: draft.listId,
              memo: draft.memo,
              schedule: {
                startDate: draft.startDate,
                startTime: draft.isAllDay ? null : draft.startTime,
                endDate: draft.endDate,
                endTime: draft.isAllDay ? null : draft.endTime,
                isAllDay: draft.isAllDay,
              },
            },
          }
        : {
            kind: "standard",
            input: {
              title: draft.title,
              listId: draft.listId,
              plannedStartDate: preset.plannedStartDate,
              dueDate: draft.dueDate || null,
              dueTime: draft.dueDate && draft.dueTime ? draft.dueTime : null,
              memo: draft.memo,
            },
            boardColumnId: preset.boardColumnId,
          };

    if (await onSubmit(submission)) {
      onClose();
    }
  }

  const listOptions =
    taskLists.length > 0
      ? taskLists.map((list) => ({ id: list.id, name: list.name }))
      : [{ id: preset.listId, name: "タスク" }];

  return (
    <div
      className="task-create-dialog-backdrop"
      onPointerDown={handleBackdropPointerDown}
    >
      <section
        ref={dialogRef}
        className="task-create-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="task-create-dialog-title"
        aria-describedby="task-create-dialog-source"
        onKeyDown={handleDialogKeyDown}
      >
        <header className="task-create-dialog-heading">
          <div>
            <p id="task-create-dialog-source">{preset.sourceLabel}</p>
            <h2 id="task-create-dialog-title">タスクを追加</h2>
          </div>
          <button
            className="inline-icon-button"
            type="button"
            aria-label="タスク作成を閉じる"
            title="閉じる"
            disabled={isSubmitting}
            onClick={onClose}
          >
            <X aria-hidden="true" size={18} />
          </button>
        </header>

        <form
          className="work-form task-create-dialog-form"
          onSubmit={(event) => void handleSubmit(event)}
        >
          {errorMessage ? (
            <p className="task-create-dialog-error" role="alert">
              {errorMessage}
            </p>
          ) : null}
          <label>
            <span>タスク名</span>
            <input
              ref={titleInputRef}
              value={draft.title}
              maxLength={120}
              placeholder="例: 週次レビュー"
              disabled={isSubmitting}
              required
              onChange={(event) =>
                setDraft((current) => ({ ...current, title: event.target.value }))
              }
            />
          </label>

          <label>
            <span>リスト</span>
            <select
              value={draft.listId}
              disabled={isSubmitting}
              onChange={(event) =>
                setDraft((current) => ({ ...current, listId: event.target.value }))
              }
            >
              {listOptions.map((list) => (
                <option key={list.id} value={list.id}>
                  {list.name}
                </option>
              ))}
            </select>
          </label>

          {preset.kind === "standard" && preset.plannedStartDate ? (
            <p className="task-create-planned-start">
              <CalendarDays aria-hidden="true" size={16} />
              <span>
                開始予定 今日（{preset.plannedStartDate.replaceAll("-", "/")}）
              </span>
            </p>
          ) : null}

          {preset.kind === "scheduled" ? (
            <>
              <div className="task-create-dialog-grid">
                <label>
                  <span>開始日</span>
                  <input
                    type="date"
                    value={draft.startDate}
                    disabled={isSubmitting}
                    required
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        startDate: event.target.value,
                      }))
                    }
                  />
                </label>
                <label>
                  <span>開始時刻</span>
                  <input
                    type="time"
                    step={900}
                    value={draft.startTime}
                    disabled={isSubmitting || draft.isAllDay}
                    required={!draft.isAllDay}
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        startTime: event.target.value,
                      }))
                    }
                  />
                </label>
                <label>
                  <span>終了日</span>
                  <input
                    type="date"
                    value={draft.endDate}
                    disabled={isSubmitting}
                    required
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        endDate: event.target.value,
                      }))
                    }
                  />
                </label>
                <label>
                  <span>終了時刻</span>
                  <input
                    type="time"
                    step={900}
                    value={draft.endTime}
                    disabled={isSubmitting || draft.isAllDay}
                    required={!draft.isAllDay}
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        endTime: event.target.value,
                      }))
                    }
                  />
                </label>
              </div>
              <label className="task-create-all-day-toggle">
                <input
                  type="checkbox"
                  checked={draft.isAllDay}
                  disabled={isSubmitting}
                  onChange={(event) =>
                    setDraft((current) => ({
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
                    }))
                  }
                />
                <span>終日</span>
              </label>
            </>
          ) : (
            <div className="task-create-dialog-grid is-due-grid">
              <label>
                <span>期限日</span>
                <input
                  type="date"
                  value={draft.dueDate}
                  disabled={isSubmitting}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      dueDate: event.target.value,
                      dueTime: event.target.value ? current.dueTime : "",
                    }))
                  }
                />
              </label>
              <label>
                <span>期限時刻</span>
                <input
                  type="time"
                  value={draft.dueTime}
                  disabled={isSubmitting || !draft.dueDate}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      dueTime: event.target.value,
                    }))
                  }
                />
              </label>
            </div>
          )}

          <label>
            <span>メモ</span>
            <textarea
              value={draft.memo}
              maxLength={2000}
              rows={3}
              disabled={isSubmitting}
              onChange={(event) =>
                setDraft((current) => ({ ...current, memo: event.target.value }))
              }
            />
          </label>

          <footer className="task-create-dialog-actions">
            <button
              className="secondary-button"
              type="button"
              disabled={isSubmitting}
              onClick={onClose}
            >
              キャンセル
            </button>
            <button
              className="primary-button"
              type="submit"
              disabled={isSubmitting || !draft.title.trim()}
            >
              {isSubmitting ? "追加中..." : "追加"}
            </button>
          </footer>
        </form>
      </section>
    </div>
  );
}

function createDraftFromPreset(preset: TaskCreatePreset): TaskCreateDraft {
  if (preset.kind === "scheduled") {
    return {
      title: "",
      listId: preset.listId,
      memo: "",
      dueDate: "",
      dueTime: "",
      startDate: preset.schedule.startDate,
      startTime: preset.schedule.startTime ?? "",
      endDate: preset.schedule.endDate,
      endTime: preset.schedule.endTime ?? "",
      isAllDay: preset.schedule.isAllDay,
    };
  }
  return {
    title: "",
    listId: preset.listId,
    memo: "",
    dueDate: preset.dueDate ?? "",
    dueTime: preset.dueDate ? (preset.dueTime ?? "") : "",
    startDate: "",
    startTime: "",
    endDate: "",
    endTime: "",
    isAllDay: false,
  };
}

function isSameTaskCreateDraft(first: TaskCreateDraft, second: TaskCreateDraft) {
  return (
    first.title === second.title &&
    first.listId === second.listId &&
    first.memo === second.memo &&
    first.dueDate === second.dueDate &&
    first.dueTime === second.dueTime &&
    first.startDate === second.startDate &&
    first.startTime === second.startTime &&
    first.endDate === second.endDate &&
    first.endTime === second.endTime &&
    first.isAllDay === second.isAllDay
  );
}
