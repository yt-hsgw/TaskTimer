import { FormEvent, type ReactNode, useEffect, useMemo, useState } from "react";
import type {
  TaskWithSubtasks,
  WorkItemDraft,
  WorkItemUpdateDraft,
} from "../../application/usecases/contracts";
import type {
  NotificationDisplayMode,
  NotificationKind,
  NotificationRule,
} from "../../domain/notification/types";
import type { RecurrenceFrequency } from "../../domain/recurrence/types";
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
  onPauseTimer(): Promise<boolean>;
  onResumeTimer(): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
  onToggleTaskCompletion(task: TaskWithSubtasks): Promise<boolean>;
  onCompleteSubtask(subtask: Subtask): Promise<boolean>;
  onSetNotificationRuleEnabled(
    ruleId: string,
    enabled: boolean,
  ): Promise<boolean>;
  onDeleteTask(task: TaskWithSubtasks): Promise<boolean>;
  onDeleteSubtask(subtask: Subtask): Promise<boolean>;
};

type DetailFormDraft = {
  title: string;
  plannedStartDate: string;
  dueDate: string;
  timerTargetMinutes: string;
  recurrenceFrequency: RecurrenceFormFrequency;
  recurrenceInterval: string;
  memo: string;
};

type RecurrenceFormFrequency = "none" | RecurrenceFrequency;
type DetailSectionKey = "subtasks" | "timer" | "notifications";

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

const recurrenceLabels: Record<RecurrenceFrequency, string> = {
  daily: "日ごと",
  weekly: "週ごと",
  monthly: "月ごと",
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
  onPauseTimer,
  onResumeTimer,
  onStopTimer,
  onToggleTaskCompletion,
  onCompleteSubtask,
  onSetNotificationRuleEnabled,
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
  const [openSections, setOpenSections] = useState<
    Record<DetailSectionKey, boolean>
  >({
    subtasks: false,
    timer: false,
    notifications: false,
  });
  const taskTarget = useMemo<WorkTargetRef>(
    () => ({ type: "task", id: task.id }),
    [task.id],
  );
  const isTaskActive = isActiveTarget(activeTimer, taskTarget);
  const completedSubtaskCount = useMemo(
    () => task.subtasks.filter((subtask) => subtask.status === "done").length,
    [task.subtasks],
  );

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
    setOpenSections({
      subtasks: false,
      timer: false,
      notifications: false,
    });
  }, [task.id]);

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

  function toggleSection(section: DetailSectionKey) {
    setOpenSections((current) => ({
      ...current,
      [section]: !current[section],
    }));
  }

  return (
    <aside className="task-detail-pane" aria-labelledby="task-detail-title">
      <div className="detail-pane-header">
        <div className="detail-title-row">
          <button
            className={`task-check-button detail-check-button ${
              task.status === "done" ? "is-done" : ""
            }`}
            type="button"
            aria-label={
              task.status === "done"
                ? `${task.title}を未完了に戻す`
                : `${task.title}を完了`
            }
            title={task.status === "done" ? "未完了に戻す" : "完了"}
            disabled={isMutating}
            onClick={() => void onToggleTaskCompletion(task)}
          >
            {task.status === "done" ? "✓" : ""}
          </button>
          <div>
            <p className="eyebrow">タスク詳細</p>
            <h2 id="task-detail-title">{task.title}</h2>
          </div>
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

        <div className="recurrence-fields">
          <label>
            <span>繰り返し</span>
            <select
              value={taskDraft.recurrenceFrequency}
              onChange={(event) =>
                setTaskDraft((current) => ({
                  ...current,
                  recurrenceFrequency: event.target
                    .value as RecurrenceFormFrequency,
                }))
              }
              disabled={isMutating}
            >
              <option value="none">なし</option>
              <option value="daily">毎日</option>
              <option value="weekly">毎週</option>
              <option value="monthly">毎月</option>
            </select>
          </label>
          <label>
            <span>間隔</span>
            <input
              type="number"
              min="1"
              max="365"
              step="1"
              value={taskDraft.recurrenceInterval}
              onChange={(event) =>
                setTaskDraft((current) => ({
                  ...current,
                  recurrenceInterval: event.target.value,
                }))
              }
              disabled={isMutating || taskDraft.recurrenceFrequency === "none"}
              inputMode="numeric"
            />
          </label>
        </div>

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
        </div>
      </form>

      <DetailDisclosure
        title="サブタスク"
        badge={`${completedSubtaskCount}/${task.subtasks.length}`}
        isOpen={openSections.subtasks}
        onToggle={() => toggleSection("subtasks")}
      >
        <p className="detail-section-description">
          親タスク「{task.title}」に紐づく作業です。
        </p>

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
              onPauseTimer={onPauseTimer}
              onResumeTimer={onResumeTimer}
              onStopTimer={onStopTimer}
              onCompleteSubtask={onCompleteSubtask}
              onSetNotificationRuleEnabled={onSetNotificationRuleEnabled}
              onDeleteSubtask={onDeleteSubtask}
            />
          ))}
        </div>
      </DetailDisclosure>

      <DetailDisclosure
        title="タイマー"
        badge={isTaskActive ? (activeTimer?.pausedAt ? "一時停止中" : "実行中") : statusLabels[task.status]}
        isOpen={openSections.timer}
        onToggle={() => toggleSection("timer")}
      >
        <div className="detail-section-heading">
          <h3>親タスクのタイマー</h3>
          <TimerControls
            target={taskTarget}
            label={task.title}
            status={task.status}
            activeTimer={activeTimer}
            isMutating={isMutating}
            onStartTimer={onStartTimer}
            onPauseTimer={onPauseTimer}
            onResumeTimer={onResumeTimer}
            onStopTimer={onStopTimer}
          />
        </div>
        <div className="detail-metrics">
          <span>
            {isTaskActive
              ? activeTimer?.pausedAt
                ? "一時停止中"
                : "実行中"
              : statusLabels[task.status]}
          </span>
          <span>{formatTimerTarget(task.timerTargetSeconds)}</span>
          <span>{formatRecurrence(taskDraft)}</span>
        </div>
      </DetailDisclosure>

      <DetailDisclosure
        title="通知"
        badge={formatNotificationSummary(task.notificationRules)}
        isOpen={openSections.notifications}
        onToggle={() => toggleSection("notifications")}
      >
        <p className="detail-section-description">
          表示タイプ: {displayModeLabels[displayMode]}
        </p>
        <NotificationRuleToggles
          label="親タスク"
          plannedStartDate={task.plannedStartDate}
          dueDate={task.dueDate}
          rules={task.notificationRules}
          isMutating={isMutating}
          onSetNotificationRuleEnabled={onSetNotificationRuleEnabled}
        />
      </DetailDisclosure>

      <div className="detail-danger-zone">
        <button
          className="danger-button"
          type="button"
          disabled={isMutating}
          onClick={() => void onDeleteTask(task)}
        >
          削除
        </button>
      </div>
    </aside>
  );
}

type DetailDisclosureProps = {
  title: string;
  badge: string;
  isOpen: boolean;
  children: ReactNode;
  onToggle(): void;
};

function DetailDisclosure({
  title,
  badge,
  isOpen,
  children,
  onToggle,
}: DetailDisclosureProps) {
  return (
    <section className="detail-section" aria-label={title}>
      <button
        className="completed-toggle detail-section-toggle"
        type="button"
        aria-expanded={isOpen}
        onClick={onToggle}
      >
        <span>{isOpen ? "⌄" : "›"}</span>
        {title}
        <strong>{badge}</strong>
      </button>
      {isOpen ? <div className="detail-section-body">{children}</div> : null}
    </section>
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
  onPauseTimer(): Promise<boolean>;
  onResumeTimer(): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
  onCompleteSubtask(subtask: Subtask): Promise<boolean>;
  onSetNotificationRuleEnabled(
    ruleId: string,
    enabled: boolean,
  ): Promise<boolean>;
  onDeleteSubtask(subtask: Subtask): Promise<boolean>;
};

function SubtaskEditor({
  subtask,
  activeTimer,
  isMutating,
  onUpdateSubtask,
  onStartTimer,
  onPauseTimer,
  onResumeTimer,
  onStopTimer,
  onCompleteSubtask,
  onSetNotificationRuleEnabled,
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
          <TimerControls
            target={target}
            label={subtask.title}
            status={subtask.status}
            activeTimer={activeTimer}
            isMutating={isMutating}
            onStartTimer={onStartTimer}
            onPauseTimer={onPauseTimer}
            onResumeTimer={onResumeTimer}
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
        <NotificationRuleToggles
          label="サブタスク"
          plannedStartDate={subtask.plannedStartDate}
          dueDate={subtask.dueDate}
          rules={subtask.notificationRules}
          isMutating={isMutating}
          onSetNotificationRuleEnabled={onSetNotificationRuleEnabled}
        />
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
        <div className="recurrence-fields">
          <label>
            <span>繰り返し</span>
            <select
              value={draft.recurrenceFrequency}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  recurrenceFrequency: event.target
                    .value as RecurrenceFormFrequency,
                }))
              }
              disabled={isMutating}
            >
              <option value="none">なし</option>
              <option value="daily">毎日</option>
              <option value="weekly">毎週</option>
              <option value="monthly">毎月</option>
            </select>
          </label>
          <label>
            <span>間隔</span>
            <input
              type="number"
              min="1"
              max="365"
              step="1"
              value={draft.recurrenceInterval}
              onChange={(event) =>
                setDraft((current) => ({
                  ...current,
                  recurrenceInterval: event.target.value,
                }))
              }
              disabled={isMutating || draft.recurrenceFrequency === "none"}
              inputMode="numeric"
            />
          </label>
        </div>
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

type NotificationRuleTogglesProps = {
  label: string;
  plannedStartDate: string | null;
  dueDate: string | null;
  rules: NotificationRule[];
  isMutating: boolean;
  onSetNotificationRuleEnabled(
    ruleId: string,
    enabled: boolean,
  ): Promise<boolean>;
};

function NotificationRuleToggles({
  label,
  plannedStartDate,
  dueDate,
  rules,
  isMutating,
  onSetNotificationRuleEnabled,
}: NotificationRuleTogglesProps) {
  return (
    <div className="notification-rule-group" aria-label={`${label}の通知`}>
      <NotificationRuleSwitch
        label="開始日"
        date={plannedStartDate}
        rule={findNotificationRule(rules, "planned_start")}
        isMutating={isMutating}
        onSetNotificationRuleEnabled={onSetNotificationRuleEnabled}
      />
      <NotificationRuleSwitch
        label="終了日"
        date={dueDate}
        rule={findNotificationRule(rules, "due")}
        isMutating={isMutating}
        onSetNotificationRuleEnabled={onSetNotificationRuleEnabled}
      />
    </div>
  );
}

type NotificationRuleSwitchProps = {
  label: string;
  date: string | null;
  rule: NotificationRule | null;
  isMutating: boolean;
  onSetNotificationRuleEnabled(
    ruleId: string,
    enabled: boolean,
  ): Promise<boolean>;
};

function NotificationRuleSwitch({
  label,
  date,
  rule,
  isMutating,
  onSetNotificationRuleEnabled,
}: NotificationRuleSwitchProps) {
  const isAvailable = Boolean(date && rule);
  const isEnabled = Boolean(rule?.enabled && isAvailable);
  const statusLabel = !date ? "日付未設定" : isEnabled ? "ON" : "OFF";

  return (
    <label
      className={`notification-switch ${isEnabled ? "is-enabled" : ""} ${
        isAvailable ? "" : "is-unavailable"
      }`}
    >
      <input
        type="checkbox"
        checked={isEnabled}
        disabled={isMutating || !rule}
        onChange={(event) => {
          if (!rule) {
            return;
          }
          void onSetNotificationRuleEnabled(rule.id, event.currentTarget.checked);
        }}
      />
      <span className="notification-switch-main">
        <strong>{label}</strong>
        <span>{date ? formatDateLabel(date) : "未設定"}</span>
      </span>
      <span className="notification-switch-status">{statusLabel}</span>
    </label>
  );
}

type TimerControlsProps = {
  target: WorkTargetRef;
  label: string;
  status: Task["status"];
  activeTimer: ActiveTimer | null;
  isMutating: boolean;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onPauseTimer(): Promise<boolean>;
  onResumeTimer(): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
};

function TimerControls({
  target,
  label,
  status,
  activeTimer,
  isMutating,
  onStartTimer,
  onPauseTimer,
  onResumeTimer,
  onStopTimer,
}: TimerControlsProps) {
  const isActive = isActiveTarget(activeTimer, target);
  const isPaused = isActive && Boolean(activeTimer?.pausedAt);
  const canStart =
    !activeTimer && status !== "done" && status !== "archived" && !isMutating;

  if (isActive) {
    return (
      <div className="timer-control-group">
        <button
          className="icon-button"
          type="button"
          aria-label={isPaused ? `${label}のタイマーを再開` : `${label}のタイマーを一時停止`}
          title={isPaused ? "再開" : "一時停止"}
          disabled={isMutating}
          onClick={() =>
            isPaused ? void onResumeTimer() : void onPauseTimer()
          }
        >
          {isPaused ? "▶" : "Ⅱ"}
        </button>
        <button
          className="stop-button"
          type="button"
          aria-label={`${label}のタイマーを終了`}
          title="タイマーを終了"
          disabled={isMutating}
          onClick={() => void onStopTimer()}
        >
          ■
        </button>
      </div>
    );
  }

  return (
    <button
      className="icon-button"
      type="button"
      aria-label={`${label}のタイマーを開始`}
      title={activeTimer ? "他のタイマーが実行中です" : "タイマーを開始"}
      disabled={!canStart}
      onClick={() => void onStartTimer(target)}
    >
      ▶
    </button>
  );
}

function toDetailFormDraft(
  item: Pick<
    Task | Subtask,
    | "title"
    | "plannedStartDate"
    | "dueDate"
    | "timerTargetSeconds"
    | "recurrenceRule"
    | "memo"
  >,
): DetailFormDraft {
  return {
    title: item.title,
    plannedStartDate: item.plannedStartDate ?? "",
    dueDate: item.dueDate ?? "",
    timerTargetMinutes: secondsToMinutesText(item.timerTargetSeconds),
    recurrenceFrequency: item.recurrenceRule?.frequency ?? "none",
    recurrenceInterval: item.recurrenceRule
      ? String(item.recurrenceRule.interval)
      : "1",
    memo: item.memo,
  };
}

function toWorkItemUpdateDraft(input: DetailFormDraft): WorkItemUpdateDraft {
  return {
    title: input.title,
    plannedStartDate: normalizeOptionalText(input.plannedStartDate),
    dueDate: normalizeOptionalText(input.dueDate),
    timerTargetSeconds: minutesToSeconds(input.timerTargetMinutes),
    recurrenceRule: toRecurrenceRuleDraft(input),
    memo: input.memo,
  };
}

function toRecurrenceRuleDraft(input: DetailFormDraft) {
  if (input.recurrenceFrequency === "none") {
    return null;
  }
  const interval = Number(input.recurrenceInterval);
  return {
    frequency: input.recurrenceFrequency,
    interval: Number.isFinite(interval) ? Math.round(interval) : 0,
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

function findNotificationRule(
  rules: NotificationRule[],
  kind: NotificationKind,
) {
  return rules.find((rule) => rule.kind === kind && !rule.deletedAt) ?? null;
}

function formatNotificationSummary(rules: NotificationRule[]) {
  if (rules.length === 0) {
    return "0/0";
  }
  const enabledCount = rules.filter((rule) => rule.enabled).length;
  return `${enabledCount}/${rules.length}`;
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

function formatRecurrence(input: DetailFormDraft) {
  if (input.recurrenceFrequency === "none") {
    return "繰り返しなし";
  }
  return `${input.recurrenceInterval || "1"}${recurrenceLabels[input.recurrenceFrequency]}`;
}

function formatDateLabel(value: string) {
  const [, month, day] = value.split("-");
  return `${Number(month)}/${Number(day)}`;
}
