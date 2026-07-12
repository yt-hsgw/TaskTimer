import type { NotificationDisplayMode } from "../../domain/notification/types";
import type { NotificationDispatchSummary } from "../../application/usecases/contracts";

type SettingsPanelProps = {
  displayMode: NotificationDisplayMode;
  notificationsEnabled: boolean;
  isMutating: boolean;
  notificationSummary: NotificationDispatchSummary | null;
  onUpdateDisplayMode(displayMode: NotificationDisplayMode): Promise<boolean>;
  onUpdateNotificationsEnabled(enabled: boolean): Promise<boolean>;
  onRetryNotifications(): Promise<boolean>;
};

export function SettingsPanel({
  displayMode,
  notificationsEnabled,
  isMutating,
  notificationSummary,
  onUpdateDisplayMode,
  onUpdateNotificationsEnabled,
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

      <label className="settings-toggle-row">
        <input
          type="checkbox"
          checked={notificationsEnabled}
          disabled={isMutating}
          onChange={(event) =>
            void onUpdateNotificationsEnabled(event.currentTarget.checked)
          }
        />
        <span>
          <strong>通知を有効にする</strong>
          <small>
            OFFの間は期限到来通知を送信しません。タスクの日付と通知ルールは保持します。
          </small>
        </span>
      </label>

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
        <span>{formatSummary(notificationSummary, notificationsEnabled)}</span>
      </div>

      {notificationSummary?.failed ? (
        <p className="settings-warning">
          {notificationSummary.lastError ?? "OS通知の送信に失敗しました。"}
        </p>
      ) : null}

      <button
        className="secondary-button"
        type="button"
        disabled={isMutating || !notificationsEnabled}
        onClick={() => void onRetryNotifications()}
      >
        通知を再試行
      </button>
    </section>
  );
}

function formatSummary(
  summary: NotificationDispatchSummary | null,
  notificationsEnabled: boolean,
) {
  if (!notificationsEnabled) {
    return "全体設定OFF";
  }
  if (!summary) {
    return "未確認";
  }
  if (summary.attempted === 0) {
    return "処理対象なし";
  }
  return `処理 ${summary.attempted}件 / 成功 ${summary.succeeded}件 / 失敗 ${summary.failed}件`;
}
