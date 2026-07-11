import type { TaskListItem } from "../../application/usecases/contracts";

export type AppView =
  | { kind: "list"; listId: string }
  | { kind: "favorites" }
  | { kind: "calendar" }
  | { kind: "settings" };

type LeftNavigationProps = {
  activeView: AppView;
  favoriteCount: number;
  isOpen: boolean;
  taskLists: TaskListItem[];
  onSelectView(view: AppView): void;
  onToggle(): void;
};

export function LeftNavigation({
  activeView,
  favoriteCount,
  isOpen,
  taskLists,
  onSelectView,
  onToggle,
}: LeftNavigationProps) {
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
          ☰
        </button>
      </div>

      <nav className="nav-sections" aria-label="ビュー">
        <div className="nav-section">
          {taskLists.map((list) => (
            <NavButton
              key={list.id}
              icon="□"
              label={list.name}
              count={list.activeTaskCount}
              isOpen={isOpen}
              isActive={activeView.kind === "list" && activeView.listId === list.id}
              onClick={() => onSelectView({ kind: "list", listId: list.id })}
            />
          ))}
          {taskLists.length === 0 ? (
            <NavButton
              icon="□"
              label="タスク"
              count={0}
              isOpen={isOpen}
              isActive={activeView.kind === "list"}
              onClick={() => onSelectView({ kind: "list", listId: "default" })}
            />
          ) : null}
        </div>

        <div className="nav-section">
          <NavButton
            icon="☆"
            label="お気に入り"
            count={favoriteCount}
            isOpen={isOpen}
            isActive={activeView.kind === "favorites"}
            onClick={() => onSelectView({ kind: "favorites" })}
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
  icon: string;
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
