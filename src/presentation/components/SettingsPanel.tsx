import type { NotificationDisplayMode } from "../../domain/notification/types";
import type { NotificationDispatchSummary } from "../../application/usecases/contracts";

type SettingsPanelProps = {
  displayMode: NotificationDisplayMode;
  isMutating: boolean;
  notificationSummary: NotificationDispatchSummary | null;
  onUpdateDisplayMode(displayMode: NotificationDisplayMode): Promise<boolean>;
  onRetryNotifications(): Promise<boolean>;
};

export function SettingsPanel({
  displayMode,
  isMutating,
  notificationSummary,
  onUpdateDisplayMode,
  onRetryNotifications,
}: SettingsPanelProps) {
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
        <select
          id="notification-mode"
          value={displayMode}
          disabled={isMutating}
          onChange={(event) =>
            void onUpdateDisplayMode(
              event.target.value as NotificationDisplayMode,
            )
          }
        >
          <option value="title_only">タイトルのみ</option>
          <option value="generic">汎用メッセージ</option>
        </select>
      </div>

      <div className="notification-status">
        <strong>期限到来通知</strong>
        <span>{formatSummary(notificationSummary)}</span>
      </div>

      {notificationSummary?.failed ? (
        <p className="settings-warning">
          {notificationSummary.lastError ?? "OS通知の送信に失敗しました。"}
        </p>
      ) : null}

      <button
        className="secondary-button"
        type="button"
        disabled={isMutating}
        onClick={() => void onRetryNotifications()}
      >
        通知を再試行
      </button>
    </section>
  );
}

function formatSummary(summary: NotificationDispatchSummary | null) {
  if (!summary) {
    return "未確認";
  }
  if (summary.attempted === 0) {
    return "処理対象なし";
  }
  return `処理 ${summary.attempted}件 / 成功 ${summary.succeeded}件 / 失敗 ${summary.failed}件`;
}
