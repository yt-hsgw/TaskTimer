import { FormEvent, useEffect, useMemo, useState } from "react";
import type {
  TaskWithSubtasks,
  WorkItemDraft,
  WorkItemUpdateDraft,
} from "../../application/usecases/contracts";
import type { NotificationDisplayMode } from "../../domain/notification/types";
import type { ActiveTimer } from "../../domain/timer/types";
import type {
  Subtask,
  Task,
  WorkStatus,
  WorkTargetRef,
} from "../../domain/task/types";

type TaskDetailPaneProps = {
  task: TaskWithSubtasks;
  activeTimer: ActiveTimer | null;
  displayMode: NotificationDisplayMode;
  isMutating: boolean;
  onClose(): void;
  onUpdateTask(
    taskId: string,
    input: WorkItemUpdateDraft,
  ): Promise<boolean>;
  onUpdateSubtask(
    subtaskId: string,
    input: WorkItemUpdateDraft,
  ): Promise<boolean>;
  onCreateSubtask(taskId: string, input: WorkItemDraft): Promise<boolean>;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
  onCompleteTask(task: TaskWithSubtasks): Promise<boolean>;
  onCompleteSubtask(subtask: Subtask): Promise<boolean>;
  onDeleteTask(task: TaskWithSubtasks): Promise<boolean>;
  onDeleteSubtask(subtask: Subtask): Promise<boolean>;
};

type DetailFormDraft = {
  title: string;
  plannedStartDate: string;
  dueDate: string;
  timerTargetMinutes: string;
  memo: string;
};

const statusLabels: Record<WorkStatus, string> = {
  todo: "未着手",
  in_progress: "進行中",
  done: "完了",
  archived: "アーカイブ",
};

const displayModeLabels: Record<NotificationDisplayMode, string> = {
  title_only: "タイトルのみ",
  generic: "汎用メッセージ",
};

export function TaskDetailPane({
  task,
  activeTimer,
  displayMode,
  isMutating,
  onClose,
  onUpdateTask,
  onUpdateSubtask,
  onCreateSubtask,
  onStartTimer,
  onStopTimer,
  onCompleteTask,
  onCompleteSubtask,
  onDeleteTask,
  onDeleteSubtask,
}: TaskDetailPaneProps) {
  const [taskDraft, setTaskDraft] = useState(() => toDetailFormDraft(task));
  const [subtaskDraft, setSubtaskDraft] = useState<WorkItemDraft>({
    title: "",
    plannedStartDate: "",
    dueDate: "",
    memo: "",
  });
  const taskTarget = useMemo<WorkTargetRef>(
    () => ({ type: "task", id: task.id }),
    [task.id],
  );
  const isTaskActive = isActiveTarget(activeTimer, taskTarget);

  useEffect(() => {
    setTaskDraft(toDetailFormDraft(task));
    setSubtaskDraft({
      title: "",
      plannedStartDate: "",
      dueDate: "",
      memo: "",
    });
  }, [task]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  async function handleUpdateTask(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await onUpdateTask(task.id, toWorkItemUpdateDraft(taskDraft));
  }

  async function handleCreateSubtask(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const created = await onCreateSubtask(
      task.id,
      normalizeCreateDraft(subtaskDraft),
    );
    if (created) {
      setSubtaskDraft({
        title: "",
        plannedStartDate: "",
        dueDate: "",
        memo: "",
      });
    }
  }

  return (
    <aside className="task-detail-pane" aria-labelledby="task-detail-title">
      <div className="detail-pane-header">
        <div>
          <p className="eyebrow">タスク詳細</p>
          <h2 id="task-detail-title">{task.title}</h2>
        </div>
        <button
          className="inline-icon-button"
          type="button"
          aria-label="詳細を閉じる"
          title="閉じる"
          onClick={onClose}
        >
          ×
        </button>
      </div>

      <form
        className="detail-form"
        onSubmit={(event) => void handleUpdateTask(event)}
      >
        <label>
          <span>タスク名</span>
          <input
            value={taskDraft.title}
            onChange={(event) =>
              setTaskDraft((current) => ({
                ...current,
                title: event.target.value,
              }))
            }
            disabled={isMutating}
            maxLength={120}
            required
          />
        </label>

        <div className="date-fields">
          <label>
            <span>開始日</span>
            <input
              type="date"
              value={taskDraft.plannedStartDate}
              onChange={(event) =>
                setTaskDraft((current) => ({
                  ...current,
                  plannedStartDate: event.target.value,
                }))
              }
              disabled={isMutating}
            />
          </label>
          <label>
            <span>終了日</span>
            <input
              type="date"
              value={taskDraft.dueDate}
              onChange={(event) =>
                setTaskDraft((current) => ({
                  ...current,
                  dueDate: event.target.value,
                }))
              }
              disabled={isMutating}
            />
          </label>
        </div>

        <label>
          <span>目標時間（分）</span>
          <input
            type="number"
            min="1"
            max="43200"
            step="1"
            value={taskDraft.timerTargetMinutes}
            onChange={(event) =>
              setTaskDraft((current) => ({
                ...current,
                timerTargetMinutes: event.target.value,
              }))
            }
            disabled={isMutating}
            inputMode="numeric"
          />
        </label>

        <label>
          <span>メモ</span>
          <textarea
            value={taskDraft.memo}
            onChange={(event) =>
              setTaskDraft((current) => ({
                ...current,
                memo: event.target.value,
              }))
            }
            disabled={isMutating}
            rows={4}
          />
        </label>

        <div className="detail-actions">
          <button className="primary-button" type="submit" disabled={isMutating}>
            保存
          </button>
          <button
            className="secondary-button"
            type="button"
            disabled={isMutating || task.status === "done"}
            onClick={() => void onCompleteTask(task)}
          >
            完了
          </button>
          <button
            className="danger-button"
            type="button"
            disabled={isMutating}
            onClick={() => void onDeleteTask(task)}
          >
            削除
          </button>
        </div>
      </form>

      <section className="detail-section" aria-label="タイマー">
        <div className="detail-section-heading">
          <h3>タイマー</h3>
          <TimerButton
            target={taskTarget}
            label={task.title}
            status={task.status}
            activeTimer={activeTimer}
            isMutating={isMutating}
            onStartTimer={onStartTimer}
            onStopTimer={onStopTimer}
          />
        </div>
        <div className="detail-metrics">
          <span>{isTaskActive ? "実行中" : statusLabels[task.status]}</span>
          <span>{formatTimerTarget(task.timerTargetSeconds)}</span>
        </div>
      </section>

      <section className="detail-section" aria-label="通知">
        <div className="detail-section-heading">
          <h3>通知</h3>
          <span className="detail-chip">{displayModeLabels[displayMode]}</span>
        </div>
        <div className="notification-plan-grid">
          <NotificationPlan label="開始日" date={task.plannedStartDate} />
          <NotificationPlan label="終了日" date={task.dueDate} />
        </div>
      </section>

      <section className="detail-section" aria-label="サブタスク">
        <div className="detail-section-heading">
          <h3>サブタスク</h3>
          <span className="detail-chip">{task.subtasks.length}</span>
        </div>

        <form
          className="detail-form subtask-create-form"
          onSubmit={(event) => void handleCreateSubtask(event)}
        >
          <label>
            <span>サブタスク名</span>
            <input
              value={subtaskDraft.title}
              onChange={(event) =>
                setSubtaskDraft((current) => ({
                  ...current,
                  title: event.target.value,
                }))
              }
              placeholder="例: チェック項目を整理"
              disabled={isMutating}
              maxLength={120}
              required
            />
          </label>
          <div className="date-fields">
            <label>
              <span>開始日</span>
              <input
                type="date"
                value={subtaskDraft.plannedStartDate ?? ""}
                onChange={(event) =>
                  setSubtaskDraft((current) => ({
                    ...current,
                    plannedStartDate: event.target.value,
                  }))
                }
                disabled={isMutating}
              />
            </label>
            <label>
              <span>終了日</span>
              <input
                type="date"
                value={subtaskDraft.dueDate ?? ""}
                onChange={(event) =>
                  setSubtaskDraft((current) => ({
                    ...current,
                    dueDate: event.target.value,
                  }))
                }
                disabled={isMutating}
              />
            </label>
          </div>
          <button className="secondary-button" type="submit" disabled={isMutating}>
            追加
          </button>
        </form>

        <div className="detail-subtask-list">
          {task.subtasks.length === 0 ? (
            <p className="empty-state">サブタスクはありません。</p>
          ) : null}
          {task.subtasks.map((subtask) => (
            <SubtaskEditor
              key={subtask.id}
              subtask={subtask}
              activeTimer={activeTimer}
              isMutating={isMutating}
              onUpdateSubtask={onUpdateSubtask}
              onStartTimer={onStartTimer}
              onStopTimer={onStopTimer}
              onCompleteSubtask={onCompleteSubtask}
              onDeleteSubtask={onDeleteSubtask}
            />
          ))}
        </div>
      </section>
    </aside>
  );
}

type SubtaskEditorProps = {
  subtask: Subtask;
  activeTimer: ActiveTimer | null;
  isMutating: boolean;
  onUpdateSubtask(
    subtaskId: string,
    input: WorkItemUpdateDraft,
  ): Promise<boolean>;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
  onCompleteSubtask(subtask: Subtask): Promise<boolean>;
  onDeleteSubtask(subtask: Subtask): Promise<boolean>;
};

function SubtaskEditor({
  subtask,
  activeTimer,
  isMutating,
  onUpdateSubtask,
  onStartTimer,
  onStopTimer,
  onCompleteSubtask,
  onDeleteSubtask,
}: SubtaskEditorProps) {
  const [draft, setDraft] = useState(() => toDetailFormDraft(subtask));
  const target: WorkTargetRef = { type: "subtask", id: subtask.id };

  useEffect(() => {
    setDraft(toDetailFormDraft(subtask));
  }, [subtask]);

  async function handleUpdateSubtask(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await onUpdateSubtask(subtask.id, toWorkItemUpdateDraft(draft));
  }

  return (
    <article className="subtask-editor">
      <div className="subtask-editor-header">
        <label className="round-check-row">
          <input
            type="checkbox"
            checked={subtask.status === "done"}
            disabled={isMutating || subtask.status === "done"}
            onChange={() => void onCompleteSubtask(subtask)}
          />
          <span>{statusLabels[subtask.status]}</span>
        </label>
        <div className="subtask-editor-tools">
          <TimerButton
            target={target}
            label={subtask.title}
            status={subtask.status}
            activeTimer={activeTimer}
            isMutating={isMutating}
            onStartTimer={onStartTimer}
            onStopTimer={onStopTimer}
          />
          <button
            className="inline-danger-button"
            type="button"
            disabled={isMutating}
            aria-label={`${subtask.title}を削除`}
            title="サブタスクを削除"
            onClick={() => void onDeleteSubtask(subtask)}
          >
            ×
          </button>
        </div>
      </div>

      <form
        className="detail-form subtask-edit-form"
        onSubmit={(event) => void handleUpdateSubtask(event)}
      >
        <label>
          <span>サブタスク名</span>
          <input
            value={draft.title}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                title: event.target.value,
              }))
            }
            disabled={isMutating}
            maxLength={120}
            required
          />
        </label>
        <div className="date-fields">
          <label>
            <span>開始日</span>
            <input
              type="date"
              value={draft.plannedStartDate}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  plannedStartDate: event.target.value,
                }))
              }
              disabled={isMutating}
            />
          </label>
          <label>
            <span>終了日</span>
            <input
              type="date"
              value={draft.dueDate}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  dueDate: event.target.value,
                }))
              }
              disabled={isMutating}
            />
          </label>
        </div>
        <label>
          <span>目標時間（分）</span>
          <input
            type="number"
            min="1"
            max="43200"
            step="1"
            value={draft.timerTargetMinutes}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                timerTargetMinutes: event.target.value,
              }))
            }
            disabled={isMutating}
            inputMode="numeric"
          />
        </label>
        <label>
          <span>メモ</span>
          <textarea
            value={draft.memo}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                memo: event.target.value,
              }))
            }
            disabled={isMutating}
            rows={2}
          />
        </label>
        <button className="secondary-button" type="submit" disabled={isMutating}>
          保存
        </button>
      </form>
    </article>
  );
}

type NotificationPlanProps = {
  label: string;
  date: string | null;
};

function NotificationPlan({ label, date }: NotificationPlanProps) {
  return (
    <div className={`notification-plan ${date ? "is-enabled" : ""}`}>
      <span>{label}</span>
      <strong>{date ? formatDateLabel(date) : "未設定"}</strong>
    </div>
  );
}

type TimerButtonProps = {
  target: WorkTargetRef;
  label: string;
  status: Task["status"];
  activeTimer: ActiveTimer | null;
  isMutating: boolean;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
};

function TimerButton({
  target,
  label,
  status,
  activeTimer,
  isMutating,
  onStartTimer,
  onStopTimer,
}: TimerButtonProps) {
  const isActive = isActiveTarget(activeTimer, target);
  const canStart =
    !activeTimer && status !== "done" && status !== "archived" && !isMutating;
  const disabled = isMutating || (!isActive && !canStart);

  return (
    <button
      className={isActive ? "stop-button" : "icon-button"}
      type="button"
      aria-label={
        isActive ? `${label}のタイマーを終了` : `${label}のタイマーを開始`
      }
      title={isActive ? "タイマーを終了" : "タイマーを開始"}
      disabled={disabled}
      onClick={() =>
        isActive ? void onStopTimer() : void onStartTimer(target)
      }
    >
      {isActive ? "■" : "▶"}
    </button>
  );
}

function toDetailFormDraft(
  item: Pick<
    Task | Subtask,
    "title" | "plannedStartDate" | "dueDate" | "timerTargetSeconds" | "memo"
  >,
): DetailFormDraft {
  return {
    title: item.title,
    plannedStartDate: item.plannedStartDate ?? "",
    dueDate: item.dueDate ?? "",
    timerTargetMinutes: secondsToMinutesText(item.timerTargetSeconds),
    memo: item.memo,
  };
}

function toWorkItemUpdateDraft(input: DetailFormDraft): WorkItemUpdateDraft {
  return {
    title: input.title,
    plannedStartDate: normalizeOptionalText(input.plannedStartDate),
    dueDate: normalizeOptionalText(input.dueDate),
    timerTargetSeconds: minutesToSeconds(input.timerTargetMinutes),
    memo: input.memo,
  };
}

function normalizeCreateDraft(input: WorkItemDraft): WorkItemDraft {
  return {
    title: input.title,
    plannedStartDate: normalizeOptionalText(input.plannedStartDate),
    dueDate: normalizeOptionalText(input.dueDate),
    memo: input.memo ?? "",
  };
}

function normalizeOptionalText(value: string | null | undefined) {
  if (!value) {
    return null;
  }
  return value;
}

function secondsToMinutesText(seconds: number | null) {
  if (!seconds) {
    return "";
  }
  return String(Math.max(1, Math.round(seconds / 60)));
}

function minutesToSeconds(minutesText: string) {
  if (!minutesText.trim()) {
    return null;
  }
  const minutes = Number(minutesText);
  if (!Number.isFinite(minutes)) {
    return null;
  }
  return Math.round(minutes) * 60;
}

function isActiveTarget(activeTimer: ActiveTimer | null, target: WorkTargetRef) {
  return (
    activeTimer?.target.type === target.type && activeTimer.target.id === target.id
  );
}

function formatTimerTarget(value: number | null) {
  if (!value) {
    return "目標未設定";
  }
  const totalMinutes = Math.round(value / 60);
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (hours > 0 && minutes > 0) {
    return `${hours}時間${minutes}分`;
  }
  if (hours > 0) {
    return `${hours}時間`;
  }
  return `${minutes}分`;
}

function formatDateLabel(value: string) {
  const [, month, day] = value.split("-");
  return `${Number(month)}/${Number(day)}`;
}
