import { FormEvent, useState } from "react";
import type { ReactNode } from "react";
import type {
  TagItem,
  TaskListColorToken,
  TaskListItem,
} from "../../application/usecases/contracts";
import { DEFAULT_TASK_LIST_ID } from "../../domain/task/types";

export type AppView =
  | { kind: "list"; listId: string }
  | { kind: "today" }
  | { kind: "favorites" }
  | { kind: "tag"; tagId: string }
  | { kind: "board" }
  | { kind: "calendar" }
  | { kind: "settings" };

type LeftNavigationProps = {
  activeView: AppView;
  favoriteCount: number;
  todayCount: number;
  isOpen: boolean;
  taskLists: TaskListItem[];
  tags: TagItem[];
  isMutating: boolean;
  onSelectView(view: AppView): void;
  onCreateTaskList(name: string): Promise<boolean>;
  onRenameTaskList(listId: string, name: string): Promise<boolean>;
  onUpdateTaskListColor(
    listId: string,
    colorToken: TaskListColorToken,
  ): Promise<boolean>;
  onDeleteTaskList(listId: string): Promise<boolean>;
  onCreateTag(name: string): Promise<boolean>;
  onRenameTag(tagId: string, name: string): Promise<boolean>;
  onDeleteTag(tagId: string): Promise<boolean>;
  onToggle(): void;
};

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

export function LeftNavigation({
  activeView,
  favoriteCount,
  todayCount,
  isOpen,
  taskLists,
  tags,
  isMutating,
  onSelectView,
  onCreateTaskList,
  onRenameTaskList,
  onUpdateTaskListColor,
  onDeleteTaskList,
  onCreateTag,
  onRenameTag,
  onDeleteTag,
  onToggle,
}: LeftNavigationProps) {
  const [isCreateOpen, setIsCreateOpen] = useState(false);
  const [newListName, setNewListName] = useState("");
  const [editingListId, setEditingListId] = useState<string | null>(null);
  const [editingListName, setEditingListName] = useState("");
  const [isTagCreateOpen, setIsTagCreateOpen] = useState(false);
  const [newTagName, setNewTagName] = useState("");
  const [editingTagId, setEditingTagId] = useState<string | null>(null);
  const [editingTagName, setEditingTagName] = useState("");

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

  const handleRename = async (event: FormEvent<HTMLFormElement>, listId: string) => {
    event.preventDefault();
    const name = editingListName.trim();
    if (!name) {
      return;
    }
    const renamed = await onRenameTaskList(listId, name);
    if (renamed) {
      setEditingListId(null);
      setEditingListName("");
    }
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

  const handleCreateTag = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const name = newTagName.trim();
    if (!name) {
      return;
    }
    const created = await onCreateTag(name);
    if (created) {
      setNewTagName("");
      setIsTagCreateOpen(false);
    }
  };

  const handleRenameTag = async (event: FormEvent<HTMLFormElement>, tagId: string) => {
    event.preventDefault();
    const name = editingTagName.trim();
    if (!name) {
      return;
    }
    const renamed = await onRenameTag(tagId, name);
    if (renamed) {
      setEditingTagId(null);
      setEditingTagName("");
    }
  };

  const handleDeleteTag = async (tag: TagItem) => {
    const shouldDelete = window.confirm(
      `「${tag.name}」タグを削除します。タスクは削除されません。`,
    );
    if (!shouldDelete) {
      return;
    }
    await onDeleteTag(tag.id);
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
          <span className="nav-panel-icon" aria-hidden="true" />
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
                +
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
                  className="nav-list-form"
                  onSubmit={(event) => void handleRename(event, list.id)}
                >
                  <input
                    value={editingListName}
                    onChange={(event) => setEditingListName(event.target.value)}
                    maxLength={80}
                    disabled={isMutating}
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
                      <div className="nav-color-picker" aria-label={`${list.name}の色`}>
                        {taskListColorOptions.map((colorToken) => (
                          <button
                            className={`nav-color-button color-${colorToken}`}
                            type="button"
                            key={colorToken}
                            aria-label={`${list.name}の色を${taskListColorLabels[colorToken]}に変更`}
                            aria-pressed={list.colorToken === colorToken}
                            title={taskListColorLabels[colorToken]}
                            disabled={
                              isMutating || list.colorToken === colorToken
                            }
                            onClick={() =>
                              void onUpdateTaskListColor(list.id, colorToken)
                            }
                          />
                        ))}
                      </div>
                      {list.id !== DEFAULT_TASK_LIST_ID ? (
                        <>
                          <button
                            className="nav-mini-button"
                            type="button"
                            aria-label={`${list.name}の名前を変更`}
                            title="名前を変更"
                            disabled={isMutating}
                            onClick={() => {
                              setIsCreateOpen(false);
                              setEditingListId(list.id);
                              setEditingListName(list.name);
                            }}
                          >
                            ✎
                          </button>
                          <button
                            className="nav-mini-button"
                            type="button"
                            aria-label={`${list.name}を削除`}
                            title="削除"
                            disabled={isMutating}
                            onClick={() => void handleDelete(list)}
                          >
                            ×
                          </button>
                        </>
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
          {isOpen ? (
            <div className="nav-section-heading">
              <span>タグ</span>
              <button
                className="nav-mini-button"
                type="button"
                aria-label="タグを追加"
                title="タグを追加"
                disabled={isMutating}
                onClick={() => {
                  setEditingTagId(null);
                  setIsTagCreateOpen((current) => !current);
                }}
              >
                +
              </button>
            </div>
          ) : null}
          {isOpen && isTagCreateOpen ? (
            <form className="nav-list-form" onSubmit={handleCreateTag}>
              <input
                value={newTagName}
                onChange={(event) => setNewTagName(event.target.value)}
                placeholder="新しいタグ"
                maxLength={40}
                disabled={isMutating}
                autoFocus
              />
              <button type="submit" disabled={isMutating || !newTagName.trim()}>
                追加
              </button>
            </form>
          ) : null}
          {tags.map((tag) => (
            <div className={`nav-list-row ${isOpen ? "has-actions" : ""}`} key={tag.id}>
              {editingTagId === tag.id ? (
                <form
                  className="nav-list-form"
                  onSubmit={(event) => void handleRenameTag(event, tag.id)}
                >
                  <input
                    value={editingTagName}
                    onChange={(event) => setEditingTagName(event.target.value)}
                    maxLength={40}
                    disabled={isMutating}
                    autoFocus
                  />
                  <button
                    type="submit"
                    disabled={isMutating || !editingTagName.trim()}
                  >
                    保存
                  </button>
                  <button
                    type="button"
                    disabled={isMutating}
                    onClick={() => {
                      setEditingTagId(null);
                      setEditingTagName("");
                    }}
                  >
                    ×
                  </button>
                </form>
              ) : (
                <>
                  <NavButton
                    icon="#"
                    label={tag.name}
                    count={tag.taskCount}
                    isOpen={isOpen}
                    isActive={
                      activeView.kind === "tag" && activeView.tagId === tag.id
                    }
                    onClick={() => onSelectView({ kind: "tag", tagId: tag.id })}
                  />
                  {isOpen ? (
                    <div className="nav-list-actions">
                      <button
                        className="nav-mini-button"
                        type="button"
                        aria-label={`${tag.name}の名前を変更`}
                        title="名前を変更"
                        disabled={isMutating}
                        onClick={() => {
                          setIsTagCreateOpen(false);
                          setEditingTagId(tag.id);
                          setEditingTagName(tag.name);
                        }}
                      >
                        ✎
                      </button>
                      <button
                        className="nav-mini-button"
                        type="button"
                        aria-label={`${tag.name}を削除`}
                        title="削除"
                        disabled={isMutating}
                        onClick={() => void handleDeleteTag(tag)}
                      >
                        ×
                      </button>
                    </div>
                  ) : null}
                </>
              )}
            </div>
          ))}
        </div>

        <div className="nav-section">
          <NavButton
            icon="◎"
            label="今日"
            count={todayCount}
            isOpen={isOpen}
            isActive={activeView.kind === "today"}
            onClick={() => onSelectView({ kind: "today" })}
          />
          <NavButton
            icon="☆"
            label="お気に入り"
            count={favoriteCount}
            isOpen={isOpen}
            isActive={activeView.kind === "favorites"}
            onClick={() => onSelectView({ kind: "favorites" })}
          />
          <NavButton
            icon="▥"
            label="かんばん"
            isOpen={isOpen}
            isActive={activeView.kind === "board"}
            onClick={() => onSelectView({ kind: "board" })}
          />
          <NavButton
            icon="▦"
            label="カレンダー"
            isOpen={isOpen}
            isActive={activeView.kind === "calendar"}
            onClick={() => onSelectView({ kind: "calendar" })}
          />
        </div>
      </nav>

      <div className="nav-footer">
        <NavButton
          icon="⚙"
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
