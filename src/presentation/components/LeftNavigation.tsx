import {
  FormEvent,
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from "react";
import { createPortal } from "react-dom";
import {
  CircleDot,
  EllipsisVertical,
  PanelLeftClose,
  PanelLeftOpen,
  Pencil,
  Plus,
  Settings,
  Star,
  Trash2,
} from "lucide-react";
import type {
  TaskListColorToken,
  TaskListItem,
} from "../../application/usecases/contracts";
import { DEFAULT_TASK_LIST_ID } from "../../domain/task/types";
import { usePresentationRenderProbe } from "../renderProbe";

const LIST_MENU_GAP = 4;
const LIST_MENU_MARGIN = 8;
const LIST_MENU_WIDTH = 148;

type ListMenuState = {
  listId: string;
  left: number;
  top: number;
};

export type AppView =
  | { kind: "list"; listId: string }
  | { kind: "today" }
  | { kind: "favorites" }
  | { kind: "tag"; tagId: string }
  | { kind: "board" }
  | { kind: "calendar" }
  | { kind: "pomodoro" }
  | { kind: "settings" };

export type WorkspaceScope =
  | { kind: "list"; listId: string }
  | { kind: "today" }
  | { kind: "favorites" };

type LeftNavigationProps = {
  activeView: AppView;
  activeScope: WorkspaceScope;
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
  activeScope,
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
  const [listMenu, setListMenu] = useState<ListMenuState | null>(null);
  const navigationRef = useRef<HTMLElement>(null);
  const listMenuRef = useRef<HTMLDivElement>(null);
  const listMenuTriggerRefs = useRef(new Map<string, HTMLButtonElement>());
  const activeMenuList = listMenu
    ? taskLists.find((list) => list.id === listMenu.listId)
    : undefined;
  const navigationContextKey = `${activeView.kind}:${activeScope.kind}:${
    activeScope.kind === "list" ? activeScope.listId : ""
  }`;
  const previousNavigationContextKey = useRef(navigationContextKey);

  const focusListMenuTrigger = useCallback((listId: string) => {
    window.requestAnimationFrame(() => {
      const trigger = listMenuTriggerRefs.current.get(listId);
      if (trigger?.isConnected) {
        trigger.focus();
      }
    });
  }, []);

  const closeListMenu = useCallback(
    (restoreFocus = false) => {
      const listId = listMenu?.listId;
      setListMenu(null);
      if (restoreFocus && listId) {
        focusListMenuTrigger(listId);
      }
    },
    [focusListMenuTrigger, listMenu?.listId],
  );

  const openListMenu = (listId: string, trigger: HTMLButtonElement) => {
    if (listMenu?.listId === listId) {
      closeListMenu(true);
      return;
    }

    setIsCreateOpen(false);
    setEditingListId(null);
    const triggerRect = trigger.getBoundingClientRect();
    setListMenu({
      listId,
      left: Math.max(
        LIST_MENU_MARGIN,
        Math.min(
          triggerRect.right - LIST_MENU_WIDTH,
          window.innerWidth - LIST_MENU_WIDTH - LIST_MENU_MARGIN,
        ),
      ),
      top: triggerRect.bottom + LIST_MENU_GAP,
    });
  };

  useLayoutEffect(() => {
    if (!listMenu || !listMenuRef.current) {
      return;
    }

    const trigger = listMenuTriggerRefs.current.get(listMenu.listId);
    if (!trigger?.isConnected) {
      setListMenu(null);
      return;
    }

    const triggerRect = trigger.getBoundingClientRect();
    const menuRect = listMenuRef.current.getBoundingClientRect();
    const spaceBelow = window.innerHeight - triggerRect.bottom - LIST_MENU_MARGIN;
    const top =
      menuRect.height > spaceBelow && triggerRect.top > menuRect.height
        ? triggerRect.top - menuRect.height - LIST_MENU_GAP
        : Math.min(
            triggerRect.bottom + LIST_MENU_GAP,
            window.innerHeight - menuRect.height - LIST_MENU_MARGIN,
          );
    const left = Math.max(
      LIST_MENU_MARGIN,
      Math.min(
        triggerRect.right - menuRect.width,
        window.innerWidth - menuRect.width - LIST_MENU_MARGIN,
      ),
    );

    if (top !== listMenu.top || left !== listMenu.left) {
      setListMenu((current) =>
        current ? { ...current, left, top: Math.max(LIST_MENU_MARGIN, top) } : null,
      );
    }
  }, [listMenu]);

  useEffect(() => {
    if (!listMenu) {
      return;
    }

    listMenuRef.current
      ?.querySelector<HTMLButtonElement>("[role='menuitem']:not(:disabled)")
      ?.focus();

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target;
      const trigger = listMenuTriggerRefs.current.get(listMenu.listId);
      if (
        !(target instanceof Node) ||
        listMenuRef.current?.contains(target) ||
        trigger?.contains(target)
      ) {
        return;
      }
      closeListMenu();
    };
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        closeListMenu(true);
      }
    };
    const handlePositionChange = () => closeListMenu();

    document.addEventListener("pointerdown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    window.addEventListener("resize", handlePositionChange);
    const navigation = navigationRef.current;
    navigation?.addEventListener("scroll", handlePositionChange);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("resize", handlePositionChange);
      navigation?.removeEventListener("scroll", handlePositionChange);
    };
  }, [closeListMenu, listMenu]);

  useEffect(() => {
    if (!isOpen && listMenu) {
      closeListMenu();
    }
  }, [closeListMenu, isOpen, listMenu]);

  useEffect(() => {
    if (listMenu && !activeMenuList) {
      closeListMenu();
    }
  }, [activeMenuList, closeListMenu, listMenu]);

  useEffect(() => {
    if (previousNavigationContextKey.current !== navigationContextKey) {
      previousNavigationContextKey.current = navigationContextKey;
      closeListMenu();
    }
  }, [closeListMenu, navigationContextKey]);

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
    focusListMenuTrigger(listId);
  };

  const startEditing = (list: TaskListItem) => {
    setIsCreateOpen(false);
    closeListMenu();
    setEditingListId(list.id);
    setEditingListName(list.name);
    setEditingListColor(list.colorToken);
  };

  const handleDelete = async (list: TaskListItem) => {
    const shouldDelete = window.confirm(
      `「${list.name}」を削除します。所属タスクは「タスク」へ移動します。`,
    );
    if (!shouldDelete) {
      return false;
    }
    return onDeleteTaskList(list.id);
  };

  const handleMenuKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
      return;
    }
    const items = Array.from(
      event.currentTarget.querySelectorAll<HTMLButtonElement>(
        "[role='menuitem']:not(:disabled)",
      ),
    );
    if (items.length === 0) {
      return;
    }
    event.preventDefault();
    const currentIndex = items.indexOf(document.activeElement as HTMLButtonElement);
    const nextIndex =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? items.length - 1
          : event.key === "ArrowDown"
            ? (currentIndex + 1 + items.length) % items.length
            : (currentIndex - 1 + items.length) % items.length;
    items[nextIndex]?.focus();
  };

  const selectView = (view: AppView) => {
    closeListMenu();
    onSelectView(view);
  };

  return (
    <>
    <aside
      ref={navigationRef}
      className="left-navigation"
      aria-label="主要ナビゲーション"
    >
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
          onClick={() => {
            closeListMenu();
            onToggle();
          }}
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
                  closeListMenu();
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
              className={`nav-list-row ${isOpen ? "has-menu" : ""}`}
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
                      focusListMenuTrigger(list.id);
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
                      activeScope.kind === "list" && activeScope.listId === list.id
                    }
                    onClick={() => selectView({ kind: "list", listId: list.id })}
                  />
                  {isOpen ? (
                    <button
                      ref={(node) => {
                        if (node) {
                          listMenuTriggerRefs.current.set(list.id, node);
                        } else {
                          listMenuTriggerRefs.current.delete(list.id);
                        }
                      }}
                      className="nav-list-menu-trigger"
                      type="button"
                      aria-label={`${list.name}の操作`}
                      aria-haspopup="menu"
                      aria-expanded={listMenu?.listId === list.id}
                      title="リストの操作"
                      disabled={isMutating}
                      onClick={(event) => openListMenu(list.id, event.currentTarget)}
                    >
                      <EllipsisVertical aria-hidden="true" size={18} />
                    </button>
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
              isActive={activeScope.kind === "list"}
              onClick={() =>
                selectView({ kind: "list", listId: DEFAULT_TASK_LIST_ID })
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
            isActive={activeScope.kind === "today"}
            onClick={() => selectView({ kind: "today" })}
          />
          <NavButton
            icon={<Star aria-hidden="true" size={18} strokeWidth={1.8} />}
            label="お気に入り"
            count={favoriteCount}
            isOpen={isOpen}
            isActive={activeScope.kind === "favorites"}
            onClick={() => selectView({ kind: "favorites" })}
          />
        </div>
      </nav>

      <div className="nav-footer">
        <NavButton
          icon={<Settings aria-hidden="true" size={21} strokeWidth={1.9} />}
          label="設定"
          isOpen={isOpen}
          isActive={activeView.kind === "settings"}
          onClick={() => selectView({ kind: "settings" })}
        />
      </div>
    </aside>
    {listMenu && activeMenuList
      ? createPortal(
          <div
            ref={listMenuRef}
            className="nav-list-menu"
            role="menu"
            aria-label={`${activeMenuList.name}の操作`}
            style={{ left: listMenu.left, top: listMenu.top }}
            onKeyDown={handleMenuKeyDown}
          >
            <button
              type="button"
              role="menuitem"
              disabled={isMutating}
              onClick={() => startEditing(activeMenuList)}
            >
              <Pencil aria-hidden="true" size={15} />
              <span>編集</span>
            </button>
            {activeMenuList.id !== DEFAULT_TASK_LIST_ID ? (
              <button
                className="is-danger"
                type="button"
                role="menuitem"
                disabled={isMutating}
                onClick={() => {
                  const list = activeMenuList;
                  closeListMenu();
                  void handleDelete(list).then((deleted) => {
                    if (!deleted) {
                      focusListMenuTrigger(list.id);
                    }
                  });
                }}
              >
                <Trash2 aria-hidden="true" size={15} />
                <span>削除</span>
              </button>
            ) : null}
          </div>,
          document.body,
        )
      : null}
    </>
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
