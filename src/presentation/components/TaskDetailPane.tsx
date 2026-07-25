import { FormEvent, useEffect, useMemo, useState } from "react";
import { X } from "lucide-react";
import type {
  ActivePomodoro,
  TagItem,
  TaskWithSubtasks,
  TaskListItem,
  WorkItemDraft,
  WorkItemUpdateDraft,
} from "../../application/usecases/contracts";
import type { RecurrenceFrequency } from "../../domain/recurrence/types";
import type { ActiveTimer } from "../../domain/timer/types";
import type {
  Subtask,
  Task,
  TaskColorToken,
  WorkStatus,
  WorkTargetRef,
} from "../../domain/task/types";
import { usePresentationRenderProbe } from "../renderProbe";

type TaskDetailPaneProps = {
  task: TaskWithSubtasks;
  selectedSubtaskId: string | null;
  activeTimer: ActiveTimer | null;
  activePomodoro: ActivePomodoro | null;
  taskLists: TaskListItem[];
  tags: TagItem[];
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
  onAttachTagToTask(taskId: string, tagId: string): Promise<boolean>;
  onCreateAndAttachTagToTask(taskId: string, name: string): Promise<boolean>;
  onRenameTag(tagId: string, name: string): Promise<boolean>;
  onDetachTagFromTask(taskId: string, tagId: string): Promise<boolean>;
};

type DetailFormDraft = {
  title: string;
  listId: string;
  colorToken: TaskColorToken | null;
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

const statusLabels: Record<WorkStatus, string> = {
  todo: "未着手",
  in_progress: "進行中",
  done: "完了",
  archived: "アーカイブ",
};

const recurrenceLabels: Record<RecurrenceFrequency, string> = {
  daily: "日ごと",
  weekly: "週ごと",
  monthly: "月ごと",
};

const timerTargetPresets = ["15", "25", "30", "45", "60", "90", "120"];
const taskColorOptions: Array<{ token: TaskColorToken; label: string }> = [
  { token: "green", label: "緑" },
  { token: "blue", label: "青" },
  { token: "amber", label: "黄" },
  { token: "rose", label: "赤" },
  { token: "violet", label: "紫" },
  { token: "gray", label: "グレー" },
];

export function TaskDetailPane({
  task,
  selectedSubtaskId,
  activeTimer,
  activePomodoro,
  taskLists,
  tags,
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
  onAttachTagToTask,
  onCreateAndAttachTagToTask,
  onRenameTag,
  onDetachTagFromTask,
}: TaskDetailPaneProps) {
  usePresentationRenderProbe("TaskDetailPane");
  const selectedSubtask = useMemo(
    () =>
      selectedSubtaskId
        ? task.subtasks.find((subtask) => subtask.id === selectedSubtaskId) ??
          null
        : null,
    [selectedSubtaskId, task.subtasks],
  );
  const detailItem = selectedSubtask ?? task;
  const isTaskDetail = !selectedSubtask;
  const detailKey = `${selectedSubtask ? "subtask" : "task"}:${detailItem.id}`;
  const detailMemo = detailItem.memo.trim();
  const taskList = taskLists.find((list) => list.id === task.listId) ?? null;
  const taskListName = taskList?.name ?? "タスク";
  const [draft, setDraft] = useState(() =>
    toDetailFormDraft(
      detailItem,
      task.listId,
      isTaskDetail ? task.colorToken : null,
    ),
  );
  const [editingTitle, setEditingTitle] = useState(false);
  const [titleDraft, setTitleDraft] = useState(detailItem.title);
  const [isScheduleEditOpen, setIsScheduleEditOpen] = useState(false);
  const [editingMemo, setEditingMemo] = useState(false);
  const [memoDraft, setMemoDraft] = useState(detailItem.memo);
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
  const [isTagEditorOpen, setIsTagEditorOpen] = useState(false);
  const [tagDraft, setTagDraft] = useState("");
  const [editingTagId, setEditingTagId] = useState<string | null>(null);
  const [tagNameDraft, setTagNameDraft] = useState("");
  const completedSubtaskCount = useMemo(
    () => task.subtasks.filter((subtask) => subtask.status === "done").length,
    [task.subtasks],
  );
  const dueChipLabel = formatDueChipLabel(detailItem.dueDate, detailItem.dueTime);
  const availableTags = useMemo(
    () => tags.filter((tag) => !task.tags.some((taskTag) => taskTag.id === tag.id)),
    [tags, task.tags],
  );
  const tagSuggestions = useMemo(() => {
    const query = tagDraft.trim().toLocaleLowerCase();
    return availableTags
      .filter((tag) => !query || tag.name.toLocaleLowerCase().includes(query))
      .slice(0, 6);
  }, [availableTags, tagDraft]);

  useEffect(() => {
    setDraft(
      toDetailFormDraft(
        detailItem,
        task.listId,
        isTaskDetail ? task.colorToken : null,
      ),
    );
    setCustomDueDraft({
      dueDate: detailItem.dueDate ?? getTodayDateInputValue(),
      dueTime: detailItem.dueTime ?? "",
    });
    setTitleDraft(detailItem.title);
    setMemoDraft(detailItem.memo);
  }, [detailItem, isTaskDetail, task.colorToken, task.listId]);

  useEffect(() => {
    setEditingTitle(false);
    setIsScheduleEditOpen(false);
    setEditingMemo(false);
    setIsDuePopoverOpen(false);
    setIsSubtaskCreateOpen(false);
    setIsDeleteConfirming(false);
  }, [detailKey]);

  useEffect(() => {
    setSubtaskDraft({
      title: "",
      dueDate: "",
      dueTime: "",
      memo: "",
    });
    setIsTagEditorOpen(false);
    setTagDraft("");
    setEditingTagId(null);
    setTagNameDraft("");
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
      setIsScheduleEditOpen(false);
    }
  }

  async function handleTitleBlur() {
    const nextTitle = titleDraft.trim();
    if (!nextTitle || nextTitle === detailItem.title || isMutating) {
      setTitleDraft(detailItem.title);
      setEditingTitle(false);
      return;
    }
    const nextDraft = { ...draft, title: nextTitle };
    setDraft(nextDraft);
    const updated = await updateCurrentItem(nextDraft);
    if (!updated) {
      setTitleDraft(detailItem.title);
    }
    setEditingTitle(false);
  }

  async function handleMemoBlur() {
    if (memoDraft === detailItem.memo || isMutating) {
      setMemoDraft(detailItem.memo);
      setEditingMemo(false);
      return;
    }
    const nextDraft = { ...draft, memo: memoDraft };
    setDraft(nextDraft);
    const updated = await updateCurrentItem(nextDraft);
    if (!updated) {
      setMemoDraft(detailItem.memo);
    }
    setEditingMemo(false);
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

  async function commitTagDraft(nameSource = tagDraft) {
    const name = nameSource.trim();
    if (!name || selectedSubtask) {
      setTagDraft("");
      setIsTagEditorOpen(false);
      return;
    }
    const alreadyAttached = task.tags.some((tag) => tag.name === name);
    if (alreadyAttached) {
      setTagDraft("");
      setIsTagEditorOpen(false);
      return;
    }
    const existingTag = tags.find((tag) => tag.name === name);
    const attached = existingTag
      ? await onAttachTagToTask(task.id, existingTag.id)
      : await onCreateAndAttachTagToTask(task.id, name);
    if (attached) {
      setTagDraft("");
      setIsTagEditorOpen(false);
    }
  }

  async function handleSubmitTagEntry(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await commitTagDraft();
  }

  async function commitTagRename(tag: { id: string; name: string }) {
    const name = tagNameDraft.trim();
    if (!name || name === tag.name || isMutating) {
      setEditingTagId(null);
      setTagNameDraft("");
      return;
    }
    const renamed = await onRenameTag(tag.id, name);
    if (renamed) {
      setEditingTagId(null);
      setTagNameDraft("");
    } else {
      setTagNameDraft(tag.name);
    }
  }

  async function handleTaskListChange(listId: string) {
    if (listId === task.listId) {
      return;
    }
    const nextDraft = toDetailFormDraft(task, listId, task.colorToken);
    setDraft(nextDraft);
    await onUpdateTask(task.id, toWorkItemUpdateDraft(nextDraft));
  }

  async function handleTaskColorChange(colorToken: TaskColorToken | null) {
    if (colorToken === draft.colorToken) {
      return;
    }
    const previousColorToken = draft.colorToken;
    setDraft((current) => ({ ...current, colorToken }));
    const persistedDraft = toDetailFormDraft(task, task.listId, colorToken);
    const updated = await onUpdateTask(
      task.id,
      toWorkItemUpdateDraft(persistedDraft),
    );
    if (!updated) {
      setDraft((current) => ({
        ...current,
        colorToken: previousColorToken,
      }));
    }
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
            {editingTitle ? (
              <input
                className="detail-title-input"
                value={titleDraft}
                disabled={isMutating}
                maxLength={120}
                autoFocus
                aria-label={selectedSubtask ? "サブタスク名" : "タスク名"}
                onChange={(event) => setTitleDraft(event.target.value)}
                onBlur={() => void handleTitleBlur()}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    event.preventDefault();
                    event.currentTarget.blur();
                  }
                  if (event.key === "Escape") {
                    event.preventDefault();
                    event.stopPropagation();
                    setTitleDraft(detailItem.title);
                    setEditingTitle(false);
                  }
                }}
              />
            ) : (
              <button
                className="detail-title-display"
                type="button"
                disabled={isMutating}
                onClick={() => setEditingTitle(true)}
              >
                <h2 id="task-detail-title">{detailItem.title}</h2>
              </button>
            )}
          </div>
        </div>
        <button
          className="inline-icon-button"
          type="button"
          aria-label="詳細を閉じる"
          title="閉じる"
          onClick={onClose}
        >
          <X aria-hidden="true" size={17} />
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

      {isTaskDetail ? (
        <section
          className="detail-list-card"
          aria-label="所属リストとタスク表示色"
        >
          <label>
            <span>所属リスト</span>
            <select
              value={task.listId}
              disabled={isMutating}
              onChange={(event) => void handleTaskListChange(event.target.value)}
            >
              {taskLists.map((list) => (
                <option key={list.id} value={list.id}>
                  {list.name}
                </option>
              ))}
            </select>
          </label>
          <fieldset className="detail-task-color-field">
            <legend>タスクの表示色</legend>
            <div className="detail-task-color-picker">
              <button
                className="detail-task-color-inherit"
                type="button"
                aria-pressed={draft.colorToken === null}
                disabled={isMutating || isScheduleEditOpen}
                onClick={() => void handleTaskColorChange(null)}
              >
                未設定
                <span
                  className={`detail-task-color-swatch color-${taskList?.colorToken ?? "green"}`}
                  aria-hidden="true"
                />
              </button>
              {taskColorOptions.map(({ token, label }) => (
                <button
                  className={`detail-task-color-button color-${token}`}
                  type="button"
                  key={token}
                  aria-label={`${label}をタスクの表示色に設定`}
                  title={label}
                  aria-pressed={draft.colorToken === token}
                  disabled={isMutating || isScheduleEditOpen}
                  onClick={() => void handleTaskColorChange(token)}
                >
                  <span aria-hidden="true" />
                </button>
              ))}
            </div>
            <small>
              タスク色が未設定の場合は「{taskListName}」の色を使用します。
            </small>
          </fieldset>
        </section>
      ) : null}

      <section className="detail-tags-card" aria-label="タグ">
        <div className="detail-tags-heading">
          <span>タグ</span>
          {selectedSubtask ? <small>親タスクから継承</small> : null}
        </div>
        <div className="detail-tag-list">
          {task.tags.map((tag) => (
            <span className="detail-tag-chip" key={tag.id}>
              {!selectedSubtask && editingTagId === tag.id ? (
                <input
                  className="detail-tag-rename-input"
                  value={tagNameDraft}
                  disabled={isMutating}
                  maxLength={40}
                  autoFocus
                  aria-label={`${tag.name}タグ名`}
                  onChange={(event) => setTagNameDraft(event.target.value)}
                  onBlur={() => void commitTagRename(tag)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      if (event.nativeEvent.isComposing) {
                        return;
                      }
                      event.preventDefault();
                      event.currentTarget.blur();
                    }
                    if (event.key === "Escape") {
                      event.preventDefault();
                      event.stopPropagation();
                      setEditingTagId(null);
                      setTagNameDraft("");
                    }
                  }}
                />
              ) : (
                <>
                  {!selectedSubtask ? (
                    <button
                      className="detail-tag-name-button"
                      type="button"
                      disabled={isMutating}
                      onClick={() => {
                        setEditingTagId(tag.id);
                        setTagNameDraft(tag.name);
                      }}
                    >
                      {tag.name}
                    </button>
                  ) : (
                    <span>{tag.name}</span>
                  )}
                  {!selectedSubtask ? (
                    <button
                      type="button"
                      aria-label={`${tag.name}タグを外す`}
                      title="タグを外す"
                      disabled={isMutating}
                      onClick={() => void onDetachTagFromTask(task.id, tag.id)}
                    >
                      ×
                    </button>
                  ) : null}
                </>
              )}
            </span>
          ))}
          {task.tags.length === 0 ? (
            <span className="detail-tag-empty">
              {selectedSubtask ? "親タスクにタグはありません" : "タグなし"}
            </span>
          ) : null}
          {!selectedSubtask ? (
            isTagEditorOpen ? (
              <form className="detail-tag-entry" onSubmit={handleSubmitTagEntry}>
                <input
                  value={tagDraft}
                  onChange={(event) => setTagDraft(event.target.value)}
                  placeholder={
                    availableTags.length > 0
                      ? "既存タグまたは新規タグ"
                      : "新規タグ"
                  }
                  maxLength={40}
                  disabled={isMutating}
                  autoFocus
                  onBlur={() => void commitTagDraft()}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" && event.nativeEvent.isComposing) {
                      return;
                    }
                    if (event.key === "Escape") {
                      event.preventDefault();
                      event.stopPropagation();
                      setTagDraft("");
                      setIsTagEditorOpen(false);
                    }
                  }}
                />
                {tagSuggestions.length > 0 ? (
                  <div className="detail-tag-suggestions" role="listbox">
                    {tagSuggestions.map((tag) => (
                      <button
                        type="button"
                        role="option"
                        key={tag.id}
                        onMouseDown={(event) => event.preventDefault()}
                        onClick={() => void commitTagDraft(tag.name)}
                      >
                        {tag.name}
                      </button>
                    ))}
                  </div>
                ) : null}
              </form>
            ) : (
              <button
                className="detail-tag-add-chip"
                type="button"
                disabled={isMutating}
                onClick={() => setIsTagEditorOpen(true)}
              >
                ＋ タグを追加
              </button>
            )
          ) : null}
        </div>
      </section>

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

      <section
        className="detail-section detail-schedule-section"
        aria-label="目標時間と繰り返し"
      >
        <button
          className="detail-inline-summary"
          type="button"
          aria-expanded={isScheduleEditOpen}
          disabled={isMutating}
          onClick={() => setIsScheduleEditOpen((current) => !current)}
        >
          <span>
            <strong>目標時間</strong>
            {formatTimerTarget(detailItem.timerTargetSeconds)}
          </span>
          <span>
            <strong>繰り返し</strong>
            {formatRecurrenceFromItem(detailItem)}
          </span>
        </button>
        {isScheduleEditOpen ? (
          <form
            className="detail-form"
            onSubmit={(event) => void handleUpdateCore(event)}
          >
          <label>
            <span>目標時間（分）</span>
            <input
              list="timer-target-presets"
              type="number"
              min="1"
              max="1440"
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

          <div className="detail-actions">
            <button className="primary-button" type="submit" disabled={isMutating}>
              保存
            </button>
          </div>
          </form>
        ) : null}
      </section>

      {isTaskDetail ? (
        <section className="detail-section" aria-label="サブタスク">
          <div className="detail-static-heading">
            <h3>サブタスク</h3>
            <strong>{completedSubtaskCount}/{task.subtasks.length}</strong>
          </div>
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
                activePomodoro={activePomodoro}
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
        </section>
      ) : null}

      <section className="detail-memo-card" aria-label="メモ">
        <span>メモ</span>
        {editingMemo ? (
          <textarea
            value={memoDraft}
            disabled={isMutating}
            autoFocus
            rows={5}
            aria-label="メモ"
            onChange={(event) => setMemoDraft(event.target.value)}
            onBlur={() => void handleMemoBlur()}
            onKeyDown={(event) => {
              if (event.key === "Escape") {
                event.preventDefault();
                event.stopPropagation();
                setMemoDraft(detailItem.memo);
                setEditingMemo(false);
              }
            }}
          />
        ) : (
          <button
            className={`detail-memo-display ${detailMemo ? "" : "is-empty"}`}
            type="button"
            disabled={isMutating}
            onClick={() => setEditingMemo(true)}
          >
            {detailMemo || "メモを追加"}
          </button>
        )}
      </section>

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

type SubtaskSummaryRowProps = {
  subtask: Subtask;
  activeTimer: ActiveTimer | null;
  activePomodoro: ActivePomodoro | null;
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
  activePomodoro,
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
        activePomodoro={activePomodoro}
        isMutating={isMutating}
        onStartTimer={onStartTimer}
        onPauseTimer={onPauseTimer}
        onResumeTimer={onResumeTimer}
        onStopTimer={onStopTimer}
      />
    </article>
  );
}

type TimerControlsProps = {
  target: WorkTargetRef;
  label: string;
  status: Task["status"];
  activeTimer: ActiveTimer | null;
  activePomodoro: ActivePomodoro | null;
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
  activePomodoro,
  isMutating,
  onStartTimer,
  onPauseTimer,
  onResumeTimer,
  onStopTimer,
}: TimerControlsProps) {
  const isActive = isActiveTarget(activeTimer, target);
  const isPaused = isActive && Boolean(activeTimer?.pausedAt);
  const canStart =
    !activeTimer &&
    !activePomodoro &&
    status !== "done" &&
    status !== "archived" &&
    !isMutating;

  if (activePomodoro) {
    return (
      <button
        className="icon-button"
        type="button"
        aria-label={`${label}の通常タイマー`}
        title="ポモドーロが実行中です"
        disabled
      >
        ▶
      </button>
    );
  }

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
      title={
        activeTimer || activePomodoro
          ? "他のタイマーまたはポモドーロが実行中です"
          : "タイマーを開始"
      }
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
  colorToken: TaskColorToken | null,
): DetailFormDraft {
  return {
    title: item.title,
    listId,
    colorToken,
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
    colorToken: input.colorToken,
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
