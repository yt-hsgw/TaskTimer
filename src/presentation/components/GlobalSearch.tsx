import { useEffect, useRef } from "react";
import { Search, Timer, X } from "lucide-react";
import type { WorkItemSearchResult } from "../../application/usecases/contracts";

type GlobalSearchProps = {
  query: string;
  results: WorkItemSearchResult[];
  isLoading: boolean;
  errorMessage: string | null;
  isOpen: boolean;
  isPomodoroActive: boolean;
  onChange(query: string): void;
  onOpenChange(isOpen: boolean): void;
  onSelect(result: WorkItemSearchResult): void;
  onOpenPomodoro(): void;
};

export function GlobalSearch({
  query,
  results,
  isLoading,
  errorMessage,
  isOpen,
  isPomodoroActive,
  onChange,
  onOpenChange,
  onSelect,
  onOpenPomodoro,
}: GlobalSearchProps) {
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handlePointerDown(event: PointerEvent) {
      if (!rootRef.current?.contains(event.target as Node)) {
        onOpenChange(false);
      }
    }

    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, [onOpenChange]);

  const trimmedQuery = query.trim();
  return (
    <div className="top-bar-actions">
      <div className="global-search" ref={rootRef}>
        <Search className="global-search-icon" aria-hidden="true" size={17} />
        <input
          aria-label="タスクを検索"
          aria-autocomplete="list"
          aria-controls="global-search-results"
          aria-expanded={isOpen && trimmedQuery.length > 0}
          role="combobox"
          type="search"
          value={query}
          maxLength={120}
          placeholder="タスクを検索"
          onChange={(event) => onChange(event.target.value)}
          onFocus={() => {
            if (trimmedQuery) {
              onOpenChange(true);
            }
          }}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              onOpenChange(false);
              event.currentTarget.blur();
            }
          }}
        />
        {query ? (
          <button
            className="global-search-clear"
            type="button"
            aria-label="検索語を消去"
            title="検索語を消去"
            onClick={() => onChange("")}
          >
            <X aria-hidden="true" size={15} />
          </button>
        ) : null}

        {isOpen && trimmedQuery ? (
          <div
            className="global-search-popover"
            id="global-search-results"
            role="listbox"
            aria-label="検索結果"
          >
            {isLoading ? (
              <p className="global-search-message">検索中...</p>
            ) : errorMessage ? (
              <p className="global-search-message is-error">{errorMessage}</p>
            ) : results.length === 0 ? (
              <p className="global-search-message">一致するタスクはありません。</p>
            ) : (
              results.map((result) => (
                <button
                  className="global-search-result"
                  type="button"
                  role="option"
                  aria-selected="false"
                  key={`${result.target.type}:${result.target.id}`}
                  onClick={() => onSelect(result)}
                >
                  <span className="global-search-result-heading">
                    <strong>{result.title}</strong>
                    <small>
                      {result.target.type === "subtask" ? "サブタスク" : "タスク"}
                    </small>
                  </span>
                  {result.parentTitle ? (
                    <span className="global-search-parent">
                      親: {result.parentTitle}
                    </span>
                  ) : null}
                  <span className="global-search-meta">
                    <span>{result.listName}</span>
                    <span>{statusLabel(result.status)}</span>
                    {result.dueDate ? (
                      <span>
                        期限 {formatDate(result.dueDate)}
                        {result.dueTime ? ` ${result.dueTime}` : ""}
                      </span>
                    ) : null}
                  </span>
                  {result.tags.length > 0 ? (
                    <span className="global-search-tags">
                      {result.tags.slice(0, 3).map((tag) => (
                        <span key={tag.id}>#{tag.name}</span>
                      ))}
                    </span>
                  ) : null}
                </button>
              ))
            )}
          </div>
        ) : null}
      </div>

      <button
        className={`top-bar-icon-button ${isPomodoroActive ? "is-active" : ""}`}
        type="button"
        aria-label="ポモドーロ"
        aria-pressed={isPomodoroActive}
        title="ポモドーロ"
        onClick={onOpenPomodoro}
      >
        <Timer aria-hidden="true" size={20} strokeWidth={1.8} />
      </button>
    </div>
  );
}

function statusLabel(status: WorkItemSearchResult["status"]) {
  if (status === "done") {
    return "完了";
  }
  if (status === "in_progress") {
    return "進行中";
  }
  return "未着手";
}

function formatDate(value: string) {
  const [, month, day] = value.split("-");
  if (!month || !day) {
    return value;
  }
  return `${Number(month)}/${Number(day)}`;
}
