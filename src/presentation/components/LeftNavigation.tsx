import { FormEvent, useState } from "react";
import type { ReactNode } from "react";
import {
  CalendarDays,
  CircleDot,
  Columns3,
  PanelLeftClose,
  PanelLeftOpen,
  Pencil,
  Plus,
  Settings,
  Star,
  Timer,
  Trash2,
} from "lucide-react";
import type {
  TaskListColorToken,
  TaskListItem,
} from "../../application/usecases/contracts";
import { DEFAULT_TASK_LIST_ID } from "../../domain/task/types";
import { usePresentationRenderProbe } from "../renderProbe";

export type AppView =
  | { kind: "list"; listId: string }
  | { kind: "today" }
  | { kind: "favorites" }
  | { kind: "tag"; tagId: string }
  | { kind: "board" }
  | { kind: "calendar" }
  | { kind: "pomodoro" }
  | { kind: "settings" };

type LeftNavigationProps = {
  activeView: AppView;
  favoriteCount: number;
  todayCount: number;
  isOpen: boolean;
  taskLists: TaskListItem[];
  isMutating: boolean;
  onSelectView(view: AppView): void;
  onCreateTaskList(name: string): Promise<boolean>;
  onUpdateTaskList(
    listId: string,
    name: string,
    colorToken: TaskListColorToken,
  ): Promise<boolean>;
  onDeleteTaskList(listId: string): Promise<boolean>;
  onToggle(): void;
};

export function LeftNavigation({
  activeView,
  favoriteCount,
  todayCount,
  isOpen,
  taskLists,
  isMutating,
  onSelectView,
  onCreateTaskList,
  onUpdateTaskList,
  onDeleteTaskList,
  onToggle,
}: LeftNavigationProps) {
  usePresentationRenderProbe("LeftNavigation");
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [newListName, setNewListName] = useState("");
  const [editingListId, setEditingListId] = useState<string | null>(null);
  const [editingListName, setEditingListName] = useState("");
  const [editingListColor, setEditingListColor] =
    useState<TaskListColorToken>("green");

  const handleCreate = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const name = newListName.trim();
    if (!name) {
      return;
    }
    const created = await onCreateTaskList(name);
    if (created) {
      setNewListName("");
      setIsCreateOpen(false);
    }
  };

  const handleEdit = async (
    event: FormEvent<HTMLFormElement>,
    listId: string,
  ) => {
    event.preventDefault();
    const list = taskLists.find((candidate) => candidate.id === listId);
    const name = editingListName.trim();
    if (!list || !name) {
      return;
    }

    const nextName = list.id === DEFAULT_TASK_LIST_ID ? list.name : name;
    if (
      (nextName !== list.name || editingListColor !== list.colorToken) &&
      !(await onUpdateTaskList(listId, nextName, editingListColor))
    ) {
      return;
    }

    setEditingListId(null);
    setEditingListName("");
  };

  const startEditing = (list: TaskListItem) => {
    setIsCreateOpen(false);
    setEditingListId(list.id);
    setEditingListName(list.name);
    setEditingListColor(list.colorToken);
  };

  const handleDelete = async (list: TaskListItem) => {
    const shouldDelete = window.confirm(
      `「${list.name}」を削除します。所属タスクは「タスク」へ移動します。`,
    );
    if (!shouldDelete) {
      return;
    }
    await onDeleteTaskList(list.id);
  };

  return (
    <aside className="left-navigation" aria-label="主要ナビゲーション">
      <div className="nav-header">
        {isOpen ? (
          <div className="nav-brand">
            <strong>TaskTimer</strong>
            <span>ローカルタスク</span>
          </div>
        ) : null}
        <button
          className="nav-icon-button"
          type="button"
          aria-label={isOpen ? "左ペインを閉じる" : "左ペインを開く"}
          title="左ペインを開閉"
          aria-expanded={isOpen}
          onClick={onToggle}
        >
          {isOpen ? (
            <PanelLeftClose aria-hidden="true" size={20} strokeWidth={1.8} />
          ) : (
            <PanelLeftOpen aria-hidden="true" size={20} strokeWidth={1.8} />
          )}
        </button>
      </div>

      <nav className="nav-sections" aria-label="ビュー">
        <div className="nav-section">
          {isOpen ? (
            <div className="nav-section-heading">
              <span>リスト</span>
              <button
                className="nav-mini-button"
                type="button"
                aria-label="リストを追加"
                title="リストを追加"
                disabled={isMutating}
                onClick={() => {
                  setEditingListId(null);
                  setIsCreateOpen((current) => !current);
                }}
              >
                <Plus aria-hidden="true" size={17} />
              </button>
            </div>
          ) : null}
          {isOpen && isCreateOpen ? (
            <form className="nav-list-form" onSubmit={handleCreate}>
              <input
                value={newListName}
                onChange={(event) => setNewListName(event.target.value)}
                placeholder="新しいリスト"
                maxLength={80}
                disabled={isMutating}
                autoFocus
              />
              <button type="submit" disabled={isMutating || !newListName.trim()}>
                追加
              </button>
            </form>
          ) : null}
          {taskLists.map((list) => (
            <div
              className={`nav-list-row ${isOpen ? "has-actions" : ""}`}
              key={list.id}
            >
              {editingListId === list.id ? (
                <form
                  className="nav-list-form nav-list-editor"
                  onSubmit={(event) => void handleEdit(event, list.id)}
                >
                  <input
                    value={editingListName}
                    onChange={(event) => setEditingListName(event.target.value)}
                    maxLength={80}
                    disabled={isMutating}
                    readOnly={list.id === DEFAULT_TASK_LIST_ID}
                    aria-label={`${list.name}の名前`}
                    autoFocus
                  />
                  <button
                    type="submit"
                    disabled={isMutating || !editingListName.trim()}
                  >
                    保存
                  </button>
                  <button
                    type="button"
                    disabled={isMutating}
                    onClick={() => {
                      setEditingListId(null);
                      setEditingListName("");
                    }}
                  >
                    ×
                  </button>
                  <div className="nav-list-color-field">
                    <span>リストの色</span>
                    <div className="nav-list-color-picker" aria-label="リストの色">
                      {taskListColorOptions.map((colorToken) => (
                        <button
                          className={`nav-list-color-button color-${colorToken}`}
                          type="button"
                          key={colorToken}
                          aria-label={`${taskListColorLabels[colorToken]}に変更`}
                          aria-pressed={editingListColor === colorToken}
                          title={taskListColorLabels[colorToken]}
                          disabled={isMutating}
                          onClick={() => setEditingListColor(colorToken)}
                        />
                      ))}
                    </div>
                  </div>
                </form>
              ) : (
                <>
                  <NavButton
                    icon={<ColorSwatch colorToken={list.colorToken} />}
                    label={list.name}
                    count={list.activeTaskCount}
                    isOpen={isOpen}
                    isActive={
                      activeView.kind === "list" && activeView.listId === list.id
                    }
                    onClick={() => onSelectView({ kind: "list", listId: list.id })}
                  />
                  {isOpen ? (
                    <div className="nav-list-actions">
                      <button
                        className="nav-mini-button"
                        type="button"
                        aria-label={`${list.name}を編集`}
                        title="リストを編集"
                        disabled={isMutating}
                        onClick={() => startEditing(list)}
                      >
                        <Pencil aria-hidden="true" size={14} />
                      </button>
                      {list.id !== DEFAULT_TASK_LIST_ID ? (
                        <button
                          className="nav-mini-button"
                          type="button"
                          aria-label={`${list.name}を削除`}
                          title="削除"
                          disabled={isMutating}
                          onClick={() => void handleDelete(list)}
                        >
                          <Trash2 aria-hidden="true" size={14} />
                        </button>
                      ) : null}
                    </div>
                  ) : null}
                </>
              )}
            </div>
          ))}
          {taskLists.length === 0 ? (
            <NavButton
              icon={<ColorSwatch colorToken="green" />}
              label="タスク"
              count={0}
              isOpen={isOpen}
              isActive={activeView.kind === "list"}
              onClick={() =>
                onSelectView({ kind: "list", listId: DEFAULT_TASK_LIST_ID })
              }
            />
          ) : null}
        </div>

        <div className="nav-section">
          <NavButton
            icon={<CircleDot aria-hidden="true" size={18} strokeWidth={1.8} />}
            label="今日"
            count={todayCount}
            isOpen={isOpen}
            isActive={activeView.kind === "today"}
            onClick={() => onSelectView({ kind: "today" })}
          />
          <NavButton
            icon={<Star aria-hidden="true" size={18} strokeWidth={1.8} />}
            label="お気に入り"
            count={favoriteCount}
            isOpen={isOpen}
            isActive={activeView.kind === "favorites"}
            onClick={() => onSelectView({ kind: "favorites" })}
          />
          <NavButton
            icon={<Columns3 aria-hidden="true" size={18} strokeWidth={1.8} />}
            label="かんばん"
            isOpen={isOpen}
            isActive={activeView.kind === "board"}
            onClick={() => onSelectView({ kind: "board" })}
          />
          <NavButton
            icon={<CalendarDays aria-hidden="true" size={18} strokeWidth={1.8} />}
            label="カレンダー"
            isOpen={isOpen}
            isActive={activeView.kind === "calendar"}
            onClick={() => onSelectView({ kind: "calendar" })}
          />
          <NavButton
            icon={<Timer aria-hidden="true" size={18} strokeWidth={1.8} />}
            label="ポモドーロ"
            isOpen={isOpen}
            isActive={activeView.kind === "pomodoro"}
            onClick={() => onSelectView({ kind: "pomodoro" })}
          />
        </div>
      </nav>

      <div className="nav-footer">
        <NavButton
          icon={<Settings aria-hidden="true" size={21} strokeWidth={1.9} />}
          label="設定"
          isOpen={isOpen}
          isActive={activeView.kind === "settings"}
          onClick={() => onSelectView({ kind: "settings" })}
        />
      </div>
    </aside>
  );
}

type NavButtonProps = {
  icon: string | ReactNode;
  label: string;
  count?: number;
  isActive: boolean;
  isOpen: boolean;
  onClick(): void;
};

function NavButton({
  icon,
  label,
  count,
  isActive,
  isOpen,
  onClick,
}: NavButtonProps) {
  return (
    <button
      className={`nav-item ${isActive ? "is-active" : ""}`}
      type="button"
      aria-current={isActive ? "page" : undefined}
      aria-label={label}
      title={label}
      onClick={onClick}
    >
      <span className="nav-item-icon" aria-hidden="true">
        {icon}
      </span>
      {isOpen ? <span className="nav-item-label">{label}</span> : null}
      {isOpen && typeof count === "number" ? (
        <span className="nav-item-count">{count}</span>
      ) : null}
    </button>
  );
}

function ColorSwatch({ colorToken }: { colorToken: TaskListColorToken }) {
  return <span className={`nav-list-color-swatch color-${colorToken}`} />;
}

const taskListColorOptions: TaskListColorToken[] = [
  "green",
  "blue",
  "amber",
  "rose",
  "violet",
  "gray",
];

const taskListColorLabels: Record<TaskListColorToken, string> = {
  green: "緑",
  blue: "青",
  amber: "黄",
  rose: "赤",
  violet: "紫",
  gray: "灰",
};
