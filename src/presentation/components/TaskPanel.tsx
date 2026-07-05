const subtasks = [
  {
    id: "subtask-1",
    title: "SQLiteマイグレーション方針",
    status: "todo",
    dueDate: "7/7",
  },
  {
    id: "subtask-2",
    title: "通知表示モード設定",
    status: "todo",
    dueDate: "7/8",
  },
];

export function TaskPanel() {
  return (
    <section className="panel task-panel" aria-labelledby="task-panel-title">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">作業中のタスク</p>
          <h2 id="task-panel-title">MVP実装</h2>
        </div>
        <button className="icon-button" type="button" aria-label="タイマーを開始">
          ▶
        </button>
      </div>

      <div className="task-meta">
        <span>開始 7/6</span>
        <span>終了 7/10</span>
        <span>進行中</span>
      </div>

      <label className="check-row">
        <input type="checkbox" />
        <span>親タスクを完了する</span>
      </label>

      <div className="subtask-list" aria-label="サブタスク">
        {subtasks.map((subtask) => (
          <label className="subtask-row" key={subtask.id}>
            <input type="checkbox" />
            <span>{subtask.title}</span>
            <small>{subtask.dueDate}</small>
            <button
              className="inline-icon-button"
              type="button"
              aria-label={`${subtask.title}のタイマーを開始`}
            >
              ▶
            </button>
          </label>
        ))}
      </div>
    </section>
  );
}
