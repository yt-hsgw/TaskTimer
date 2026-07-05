import type { NotificationDisplayMode } from "../../domain/notification/types";

type SettingsPanelProps = {
  displayMode: NotificationDisplayMode;
};

export function SettingsPanel({ displayMode }: SettingsPanelProps) {
  return (
    <section className="panel settings-panel" aria-labelledby="settings-title">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">ローカル設定</p>
          <h2 id="settings-title">通知</h2>
        </div>
      </div>

      <div className="field-group">
        <label htmlFor="notification-mode">表示タイプ</label>
        <select id="notification-mode" value={displayMode} disabled>
          <option value="title_only">タイトルのみ</option>
          <option value="generic">汎用メッセージ</option>
        </select>
      </div>

      <label className="toggle-row">
        <input type="checkbox" defaultChecked />
        <span>ローカル通知を有効化</span>
      </label>
    </section>
  );
}
