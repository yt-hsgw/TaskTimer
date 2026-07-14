import { FormEvent, type ReactNode, useEffect, useMemo, useState } from "react";
import type {
  TaskWithSubtasks,
  TaskListItem,
  WorkItemDraft,
  WorkItemUpdateDraft,
} from "../../application/usecases/contracts";
import type { NotificationDisplayMode } from "../../domain/notification/types";
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
  selectedSubtaskId: string | null;
  activeTimer: ActiveTimer | null;
  taskLists: TaskListItem[];
  displayMode: NotificationDisplayMode;
  isMutating: boolean;
  onClose(): void;
  onUpdateTask(taskId: string, input: WorkItemUpdateDraft): Promise<boolean>;
  onUpdateSubtask(
    subtaskId: string,
    input: WorkItemUpdateDraft,
  ): Promise<boolean>;
  onCreateSubtask(taskId: string, input: WorkItemDraft): Promise<boolean>;
  onSelectSubtask(subtaskId: string): void;
  onSelectParentTask(): void;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onPauseTimer(): Promise<boolean>;
  onResumeTimer(): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
  onToggleTaskCompletion(task: TaskWithSubtasks): Promise<boolean>;
  onToggleSubtaskCompletion(subtask: Subtask): Promise<boolean>;
  onDeleteTask(task: TaskWithSubtasks): Promise<boolean>;
  onDeleteSubtask(subtask: Subtask): Promise<boolean>;
};

type DetailFormDraft = {
  title: string;
  listId: string;
  dueDate: string;
  dueTime: string;
  timerTargetMinutes: string;
  recurrenceEnabled: boolean;
  recurrenceFrequency: RecurrenceFrequency;
  recurrenceInterval: string;
  memo: string;
};

type SubtaskCreateDraft = {
  title: string;
  dueDate: string;
  dueTime: string;
  memo: string;
};

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

const timerTargetPresets = ["15", "25", "30", "45", "60", "90", "120"];

export function TaskDetailPane({
  task,
  selectedSubtaskId,
  activeTimer,
  taskLists,
  displayMode,
  isMutating,
  onClose,
  onUpdateTask,
  onUpdateSubtask,
  onCreateSubtask,
  onSelectSubtask,
  onSelectParentTask,
  onStartTimer,
  onPauseTimer,
  onResumeTimer,
  onStopTimer,
  onToggleTaskCompletion,
  onToggleSubtaskCompletion,
  onDeleteTask,
  onDeleteSubtask,
}: TaskDetailPaneProps) {
  const selectedSubtask = useMemo(
    () =>
      selectedSubtaskId
        ? task.subtasks.find((subtask) => subtask.id === selectedSubtaskId) ??
          null
        : null,
    [selectedSubtaskId, task.subtasks],
  );
  const detailItem = selectedSubtask ?? task;
  const detailTarget = useMemo<WorkTargetRef>(
    () => ({
      type: selectedSubtask ? "subtask" : "task",
      id: detailItem.id,
    }),
    [detailItem.id, selectedSubtask],
  );
  const isTaskDetail = !selectedSubtask;
  const hasSubtasks = task.subtasks.length > 0;
  const detailKey = `${selectedSubtask ? "subtask" : "task"}:${detailItem.id}`;
  const detailMemo = detailItem.memo.trim();
  const taskListName =
    taskLists.find((list) => list.id === task.listId)?.name ?? "タスク";
  const [draft, setDraft] = useState(() =>
    toDetailFormDraft(detailItem, task.listId),
  );
  const [isCoreEditOpen, setIsCoreEditOpen] = useState(false);
  const [isDuePopoverOpen, setIsDuePopoverOpen] = useState(false);
  const [isSubtaskCreateOpen, setIsSubtaskCreateOpen] = useState(false);
  const [isDeleteConfirming, setIsDeleteConfirming] = useState(false);
  const [customDueDraft, setCustomDueDraft] = useState({
    dueDate: detailItem.dueDate ?? getTodayDateInputValue(),
    dueTime: detailItem.dueTime ?? "",
  });
  const [subtaskDraft, setSubtaskDraft] = useState<SubtaskCreateDraft>({
    title: "",
    dueDate: "",
    dueTime: "",
    memo: "",
  });
  const [openSections, setOpenSections] = useState<
    Record<DetailSectionKey, boolean>
  >({
    subtasks: isTaskDetail && hasSubtasks,
    timer: false,
    notifications: false,
  });
  const completedSubtaskCount = useMemo(
    () => task.subtasks.filter((subtask) => subtask.status === "done").length,
    [task.subtasks],
  );
  const dueChipLabel = formatDueChipLabel(detailItem.dueDate, detailItem.dueTime);

  useEffect(() => {
    setDraft(toDetailFormDraft(detailItem, task.listId));
    setCustomDueDraft({
      dueDate: detailItem.dueDate ?? getTodayDateInputValue(),
      dueTime: detailItem.dueTime ?? "",
    });
  }, [detailItem, task.listId]);

  useEffect(() => {
    setIsCoreEditOpen(false);
    setIsDuePopoverOpen(false);
    setIsSubtaskCreateOpen(false);
    setIsDeleteConfirming(false);
    setOpenSections({
      subtasks: isTaskDetail && hasSubtasks,
      timer: false,
      notifications: false,
    });
  }, [detailKey, hasSubtasks, isTaskDetail]);

  useEffect(() => {
    setSubtaskDraft({
      title: "",
      dueDate: "",
      dueTime: "",
      memo: "",
    });
  }, [task.id]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        if (isDuePopoverOpen) {
          setIsDuePopoverOpen(false);
          return;
        }
        onClose();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [isDuePopoverOpen, onClose]);

  async function updateCurrentItem(nextDraft: DetailFormDraft) {
    const input = toWorkItemUpdateDraft(nextDraft);
    if (selectedSubtask) {
      return onUpdateSubtask(selectedSubtask.id, input);
    }
    return onUpdateTask(task.id, input);
  }

  async function handleUpdateCore(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const updated = await updateCurrentItem(draft);
    if (updated) {
      setIsCoreEditOpen(false);
    }
  }

  async function handleCreateSubtask(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const created = await onCreateSubtask(task.id, normalizeCreateDraft(subtaskDraft));
    if (created) {
      setSubtaskDraft({
        title: "",
        dueDate: "",
        dueTime: "",
        memo: "",
      });
      setIsSubtaskCreateOpen(false);
    }
  }

  async function applyDue(dueDate: string | null, dueTime: string | null) {
    const nextDraft = {
      ...draft,
      dueDate: dueDate ?? "",
      dueTime: dueDate ? dueTime ?? "" : "",
    };
    setDraft(nextDraft);
    const updated = await updateCurrentItem(nextDraft);
    if (updated) {
      setIsDuePopoverOpen(false);
    }
  }

  function toggleSection(section: DetailSectionKey) {
    setOpenSections((current) => ({
      ...current,
      [section]: !current[section],
    }));
  }

  function handleToggleRecurrence(enabled: boolean) {
    setDraft((current) => {
      if (enabled && !current.dueDate) {
        return {
          ...current,
          recurrenceEnabled: true,
          dueDate: getTodayDateInputValue(),
        };
      }
      if (
        !enabled &&
        !detailItem.dueDate &&
        current.dueDate === getTodayDateInputValue()
      ) {
        return {
          ...current,
          recurrenceEnabled: false,
          dueDate: "",
          dueTime: "",
        };
      }
      return {
        ...current,
        recurrenceEnabled: enabled,
      };
    });
  }

  function handleDeleteClick() {
    if (!isDeleteConfirming) {
      setIsDeleteConfirming(true);
      return;
    }

    if (selectedSubtask) {
      void onDeleteSubtask(selectedSubtask);
      return;
    }
    void onDeleteTask(task);
  }

  const isActive = isActiveTarget(activeTimer, detailTarget);

  return (
    <aside className="task-detail-pane" aria-labelledby="task-detail-title">
      <div className="detail-pane-header">
        <div className="detail-title-row">
          <button
            className={`task-check-button detail-check-button ${
              detailItem.status === "done" ? "is-done" : ""
            }`}
            type="button"
            aria-label={
              detailItem.status === "done"
                ? `${detailItem.title}を未完了に戻す`
                : `${detailItem.title}を完了`
            }
            title={detailItem.status === "done" ? "未完了に戻す" : "完了"}
            disabled={isMutating}
            onClick={() =>
              selectedSubtask
                ? void onToggleSubtaskCompletion(selectedSubtask)
                : void onToggleTaskCompletion(task)
            }
          >
            {detailItem.status === "done" ? "✓" : ""}
          </button>
          <div>
            <p className="eyebrow">
              {selectedSubtask ? "サブタスク詳細" : "タスク詳細"}
            </p>
            <h2 id="task-detail-title">{detailItem.title}</h2>
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

      {selectedSubtask ? (
        <button
          className="parent-task-link"
          type="button"
          onClick={onSelectParentTask}
        >
          ← 親タスク: {task.title}
        </button>
      ) : null}

      <section className="detail-reference-card" aria-label="現在情報">
        <div>
          <span>状態</span>
          <strong>{statusLabels[detailItem.status]}</strong>
        </div>
        {isTaskDetail ? (
          <div>
            <span>リスト</span>
            <strong>{taskListName}</strong>
          </div>
        ) : null}
        <div>
          <span>期限</span>
          <strong>{formatDue(detailItem.dueDate, detailItem.dueTime)}</strong>
        </div>
        <div>
          <span>目標時間</span>
          <strong>{formatTimerTarget(detailItem.timerTargetSeconds)}</strong>
        </div>
        <div>
          <span>繰り返し</span>
          <strong>{formatRecurrenceFromItem(detailItem)}</strong>
        </div>
      </section>

      {detailMemo ? (
        <section className="detail-memo-card" aria-label="メモ">
          <span>メモ</span>
          <p>{detailMemo}</p>
        </section>
      ) : null}

      <div className="detail-due-area" aria-label="期限クイック設定">
        <div className="detail-quick-actions">
          {detailItem.dueDate ? (
            <span className="due-selected-chip">
              {dueChipLabel}
              <button
                type="button"
                aria-label="期限を削除"
                title="期限を削除"
                disabled={isMutating}
                onClick={() => void applyDue(null, null)}
              >
                ×
              </button>
            </span>
          ) : (
            <>
              <button
                className="due-chip-button"
                type="button"
                disabled={isMutating}
                onClick={() => void applyDue(getTodayDateInputValue(), null)}
              >
                今日
              </button>
              <button
                className="due-chip-button"
                type="button"
                disabled={isMutating}
                onClick={() => void applyDue(getTomorrowDateInputValue(), null)}
              >
                明日
              </button>
              <button
                className="due-chip-button"
                type="button"
                disabled={isMutating}
                aria-expanded={isDuePopoverOpen}
                onClick={() => setIsDuePopoverOpen((current) => !current)}
              >
                ◷ 時間設定
              </button>
            </>
          )}
        </div>
        {isDuePopoverOpen && !detailItem.dueDate ? (
          <div className="due-popover-anchor">
            <form
              className="due-popover"
              onSubmit={(event) => {
                event.preventDefault();
                void applyDue(
                  normalizeOptionalText(customDueDraft.dueDate),
                  normalizeOptionalText(customDueDraft.dueTime),
                );
              }}
            >
              <label>
                <span>期限日</span>
                <input
                  type="date"
                  value={customDueDraft.dueDate}
                  onChange={(event) =>
                    setCustomDueDraft((current) => ({
                      ...current,
                      dueDate: event.target.value,
                    }))
                  }
                  required
                />
              </label>
              <label>
                <span>期限時刻</span>
                <input
                  type="time"
                  value={customDueDraft.dueTime}
                  onChange={(event) =>
                    setCustomDueDraft((current) => ({
                      ...current,
                      dueTime: event.target.value,
                    }))
                  }
                />
              </label>
              <div className="popover-actions">
                <button className="primary-button" type="submit" disabled={isMutating}>
                  保存
                </button>
                <button
                  className="secondary-button"
                  type="button"
                  disabled={isMutating}
                  onClick={() => setIsDuePopoverOpen(false)}
                >
                  キャンセル
                </button>
              </div>
            </form>
          </div>
        ) : null}
      </div>

      <DetailDisclosure
        title={selectedSubtask ? "サブタスクを編集" : "タスクを編集"}
        badge={isCoreEditOpen ? "編集中" : "参照"}
        isOpen={isCoreEditOpen}
        onToggle={() => setIsCoreEditOpen((current) => !current)}
      >
        <form
          className="detail-form"
          onSubmit={(event) => void handleUpdateCore(event)}
        >
          <label>
            <span>{selectedSubtask ? "サブタスク名" : "タスク名"}</span>
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

          {isTaskDetail ? (
            <label>
              <span>所属リスト</span>
              <select
                value={draft.listId}
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    listId: event.target.value,
                  }))
                }
                disabled={isMutating}
              >
                {taskLists.map((list) => (
                  <option key={list.id} value={list.id}>
                    {list.name}
                  </option>
                ))}
              </select>
            </label>
          ) : null}

          <label>
            <span>目標時間（分）</span>
            <input
              list="timer-target-presets"
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
            <datalist id="timer-target-presets">
              {timerTargetPresets.map((minutes) => (
                <option key={minutes} value={minutes} />
              ))}
            </datalist>
          </label>

          <label className="settings-toggle-row detail-toggle-row">
            <input
              type="checkbox"
              checked={draft.recurrenceEnabled}
              disabled={isMutating}
              onChange={(event) =>
                handleToggleRecurrence(event.currentTarget.checked)
              }
            />
            <span>
              <strong>繰り返しを有効にする</strong>
              <small>
                有効時だけ頻度と間隔を設定します。期限未設定の場合は今日を基準にします。
              </small>
            </span>
          </label>

          {draft.recurrenceEnabled ? (
            <div className="recurrence-fields">
              <label>
                <span>頻度</span>
                <select
                  value={draft.recurrenceFrequency}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      recurrenceFrequency: event.target.value as RecurrenceFrequency,
                    }))
                  }
                  disabled={isMutating}
                >
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
                  disabled={isMutating}
                  inputMode="numeric"
                />
              </label>
            </div>
          ) : null}

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
              rows={4}
            />
          </label>

          <div className="detail-actions">
            <button className="primary-button" type="submit" disabled={isMutating}>
              保存
            </button>
          </div>
        </form>
      </DetailDisclosure>

      {isTaskDetail ? (
        <DetailDisclosure
          title="サブタスク"
          badge={`${completedSubtaskCount}/${task.subtasks.length}`}
          isOpen={openSections.subtasks}
          onToggle={() => toggleSection("subtasks")}
        >
          <p className="detail-section-description">
            親タスク「{task.title}」に紐づく作業です。既存サブタスクの編集は選択して開きます。
          </p>

          {isSubtaskCreateOpen ? (
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
                  <span>期限日</span>
                  <input
                    type="date"
                    value={subtaskDraft.dueDate}
                    onChange={(event) =>
                      setSubtaskDraft((current) => ({
                        ...current,
                        dueDate: event.target.value,
                      }))
                    }
                    disabled={isMutating}
                  />
                </label>
                <label>
                  <span>期限時刻</span>
                  <input
                    type="time"
                    value={subtaskDraft.dueTime}
                    onChange={(event) =>
                      setSubtaskDraft((current) => ({
                        ...current,
                        dueTime: event.target.value,
                      }))
                    }
                    disabled={isMutating || !subtaskDraft.dueDate}
                  />
                </label>
              </div>
              <div className="subtask-create-actions">
                <button className="primary-button" type="submit" disabled={isMutating}>
                  追加
                </button>
                <button
                  className="secondary-button"
                  type="button"
                  disabled={isMutating}
                  onClick={() => setIsSubtaskCreateOpen(false)}
                >
                  キャンセル
                </button>
              </div>
            </form>
          ) : (
            <button
              className="subtask-add-button"
              type="button"
              disabled={isMutating}
              onClick={() => setIsSubtaskCreateOpen(true)}
            >
              ＋ サブタスクの追加
            </button>
          )}

          <div className="detail-subtask-list">
            {task.subtasks.length === 0 ? (
              <p className="empty-state">サブタスクはありません。</p>
            ) : null}
            {task.subtasks.map((subtask) => (
              <SubtaskSummaryRow
                key={subtask.id}
                subtask={subtask}
                activeTimer={activeTimer}
                isMutating={isMutating}
                onSelect={() => onSelectSubtask(subtask.id)}
                onToggleSubtaskCompletion={onToggleSubtaskCompletion}
                onStartTimer={onStartTimer}
                onPauseTimer={onPauseTimer}
                onResumeTimer={onResumeTimer}
                onStopTimer={onStopTimer}
              />
            ))}
          </div>
        </DetailDisclosure>
      ) : null}

      <DetailDisclosure
        title="タイマー"
        badge={isActive ? (activeTimer?.pausedAt ? "一時停止中" : "実行中") : statusLabels[detailItem.status]}
        isOpen={openSections.timer}
        onToggle={() => toggleSection("timer")}
      >
        <div className="detail-section-heading">
          <h3>{selectedSubtask ? "サブタスクのタイマー" : "親タスクのタイマー"}</h3>
          <TimerControls
            target={detailTarget}
            label={detailItem.title}
            status={detailItem.status}
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
            {isActive
              ? activeTimer?.pausedAt
                ? "一時停止中"
                : "実行中"
              : statusLabels[detailItem.status]}
          </span>
          <span>{formatTimerTarget(detailItem.timerTargetSeconds)}</span>
          <span>{formatRecurrenceFromItem(detailItem)}</span>
        </div>
      </DetailDisclosure>

      <DetailDisclosure
        title="通知"
        badge={displayModeLabels[displayMode]}
        isOpen={openSections.notifications}
        onToggle={() => toggleSection("notifications")}
      >
        <p className="detail-section-description">
          表示タイプ: {displayModeLabels[displayMode]}
        </p>
        <div className="notification-plan-grid">
          <NotificationPlan
            label="期限"
            date={detailItem.dueDate}
            time={detailItem.dueTime}
          />
        </div>
      </DetailDisclosure>

      <div className="detail-danger-zone">
        <button
          className="danger-button"
          type="button"
          disabled={isMutating}
          onClick={handleDeleteClick}
        >
          {isDeleteConfirming ? "もう一度押して削除" : "削除"}
        </button>
        {isDeleteConfirming ? (
          <button
            className="secondary-button"
            type="button"
            disabled={isMutating}
            onClick={() => setIsDeleteConfirming(false)}
          >
            キャンセル
          </button>
        ) : null}
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

type SubtaskSummaryRowProps = {
  subtask: Subtask;
  activeTimer: ActiveTimer | null;
  isMutating: boolean;
  onSelect(): void;
  onToggleSubtaskCompletion(subtask: Subtask): Promise<boolean>;
  onStartTimer(target: WorkTargetRef): Promise<boolean>;
  onPauseTimer(): Promise<boolean>;
  onResumeTimer(): Promise<boolean>;
  onStopTimer(): Promise<boolean>;
};

function SubtaskSummaryRow({
  subtask,
  activeTimer,
  isMutating,
  onSelect,
  onToggleSubtaskCompletion,
  onStartTimer,
  onPauseTimer,
  onResumeTimer,
  onStopTimer,
}: SubtaskSummaryRowProps) {
  const target: WorkTargetRef = { type: "subtask", id: subtask.id };
  return (
    <article className="subtask-summary-row">
      <button
        className={`task-check-button ${subtask.status === "done" ? "is-done" : ""}`}
        type="button"
        aria-label={
          subtask.status === "done"
            ? `${subtask.title}を未完了に戻す`
            : `${subtask.title}を完了`
        }
        title={subtask.status === "done" ? "未完了に戻す" : "完了"}
        disabled={isMutating}
        onClick={() => void onToggleSubtaskCompletion(subtask)}
      >
        {subtask.status === "done" ? "✓" : ""}
      </button>
      <button className="subtask-summary-main" type="button" onClick={onSelect}>
        <strong>{subtask.title}</strong>
        <span>
          {statusLabels[subtask.status]} / {formatDue(subtask.dueDate, subtask.dueTime)}
        </span>
      </button>
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
    </article>
  );
}

type NotificationPlanProps = {
  label: string;
  date: string | null;
  time: string | null;
};

function NotificationPlan({ label, date, time }: NotificationPlanProps) {
  return (
    <div className={`notification-plan ${date ? "is-enabled" : ""}`}>
      <span>{label}</span>
      <strong>{date ? formatDue(date, time) : "未設定"}</strong>
    </div>
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
    | "dueDate"
    | "dueTime"
    | "timerTargetSeconds"
    | "recurrenceRule"
    | "memo"
  >,
  listId: string,
): DetailFormDraft {
  return {
    title: item.title,
    listId,
    dueDate: item.dueDate ?? "",
    dueTime: item.dueTime ?? "",
    timerTargetMinutes: secondsToMinutesText(item.timerTargetSeconds),
    recurrenceEnabled: Boolean(item.recurrenceRule),
    recurrenceFrequency: item.recurrenceRule?.frequency ?? "weekly",
    recurrenceInterval: item.recurrenceRule
      ? String(item.recurrenceRule.interval)
      : "1",
    memo: item.memo,
  };
}

function toWorkItemUpdateDraft(input: DetailFormDraft): WorkItemUpdateDraft {
  const dueDate = normalizeOptionalText(input.dueDate);
  return {
    listId: input.listId,
    title: input.title,
    plannedStartDate: null,
    dueDate,
    dueTime: dueDate ? normalizeOptionalText(input.dueTime) : null,
    timerTargetSeconds: minutesToSeconds(input.timerTargetMinutes),
    recurrenceRule: input.recurrenceEnabled
      ? toRecurrenceRuleDraft(input)
      : null,
    memo: input.memo,
  };
}

function toRecurrenceRuleDraft(input: DetailFormDraft) {
  const interval = Number(input.recurrenceInterval);
  return {
    frequency: input.recurrenceFrequency,
    interval: Number.isFinite(interval) ? Math.round(interval) : 0,
  };
}

function normalizeCreateDraft(input: SubtaskCreateDraft): WorkItemDraft {
  const dueDate = normalizeOptionalText(input.dueDate);
  return {
    title: input.title,
    plannedStartDate: null,
    dueDate,
    dueTime: dueDate ? normalizeOptionalText(input.dueTime) : null,
    memo: input.memo,
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

function formatRecurrenceFromItem(
  item: Pick<Task | Subtask, "recurrenceRule">,
) {
  if (!item.recurrenceRule) {
    return "繰り返しなし";
  }
  return `${item.recurrenceRule.interval}${recurrenceLabels[item.recurrenceRule.frequency]}`;
}

function formatDue(date: string | null, time: string | null) {
  if (!date) {
    return "期限なし";
  }
  return `${formatDateLabel(date)}${time ? ` ${time}` : ""}`;
}

function formatDueChipLabel(date: string | null, time: string | null) {
  if (!date) {
    return "期限なし";
  }
  const today = getTodayDateInputValue();
  const tomorrow = getTomorrowDateInputValue();
  const label =
    date === today ? "今日" : date === tomorrow ? "明日" : formatDateLabel(date);
  return time ? `${label} ${time}` : label;
}

function formatDateLabel(value: string) {
  const [, month, day] = value.split("-");
  return `${Number(month)}/${Number(day)}`;
}

function getTodayDateInputValue() {
  return toDateInputValue(new Date());
}

function getTomorrowDateInputValue() {
  const date = new Date();
  date.setDate(date.getDate() + 1);
  return toDateInputValue(date);
}

function toDateInputValue(date: Date) {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}
