import { useState } from "react";
import type { NotificationDisplayMode } from "../../domain/notification/types";
import type {
  NotificationDeliveryAttempt,
  NotificationDispatchSummary,
} from "../../application/usecases/contracts";

export type DataManagementActionResult = {
  status: "success" | "failed" | "cancelled";
  message: string;
  detail?: string;
};

type DataManagementOperation =
  | "sqlite-backup"
  | "sqlite-restore"
  | "json-export"
  | "csv-export";

type SettingsPanelProps = {
  displayMode: NotificationDisplayMode;
  notificationsEnabled: boolean;
  isMutating: boolean;
  notificationSummary: NotificationDispatchSummary | null;
  notificationFailureHistory: NotificationDeliveryAttempt[];
  onUpdateDisplayMode(displayMode: NotificationDisplayMode): Promise<boolean>;
  onUpdateNotificationsEnabled(enabled: boolean): Promise<boolean>;
  onRetryNotifications(): Promise<boolean>;
  onCreateSqliteBackup(): Promise<DataManagementActionResult>;
  onRestoreSqliteBackup(): Promise<DataManagementActionResult>;
  onCreateJsonExport(): Promise<DataManagementActionResult>;
  onCreateCsvExport(): Promise<DataManagementActionResult>;
};

export function SettingsPanel({
  displayMode,
  notificationsEnabled,
  isMutating,
  notificationSummary,
  notificationFailureHistory,
  onUpdateDisplayMode,
  onUpdateNotificationsEnabled,
  onRetryNotifications,
  onCreateSqliteBackup,
  onRestoreSqliteBackup,
  onCreateJsonExport,
  onCreateCsvExport,
}: SettingsPanelProps) {
  const [activeDataOperation, setActiveDataOperation] =
    useState<DataManagementOperation | null>(null);
  const [dataManagementResult, setDataManagementResult] =
    useState<DataManagementActionResult | null>(null);
  const isDataManagementBusy = isMutating || activeDataOperation !== null;

  const runDataManagementAction = async (
    operation: DataManagementOperation,
    action: () => Promise<DataManagementActionResult>,
  ) => {
    setActiveDataOperation(operation);
    setDataManagementResult(null);
    try {
      setDataManagementResult(await action());
    } catch {
      setDataManagementResult({
        status: "failed",
        message: "データ管理操作に失敗しました。",
        detail: "保存先の権限、空き容量、バックアップ形式を確認してください。",
      });
    } finally {
      setActiveDataOperation(null);
    }
  };

  return (
    <section className="panel settings-panel" aria-labelledby="settings-title">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">ローカル設定</p>
          <h2 id="settings-title">設定</h2>
        </div>
      </div>

      <div className="settings-content">
        <section
          className="settings-section"
          aria-labelledby="notification-settings-title"
        >
          <div className="settings-section-heading">
            <div>
              <h3 id="notification-settings-title">通知</h3>
              <span>期限到来通知と表示タイプ</span>
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

          <div
            className="notification-history"
            aria-labelledby="notification-history-title"
          >
            <div className="notification-history-heading">
              <div>
                <strong id="notification-history-title">通知失敗履歴</strong>
                <span>失敗が絡む通知の最新20件</span>
              </div>
            </div>

            {notificationFailureHistory.length > 0 ? (
              <ol className="notification-history-list">
                {notificationFailureHistory.map((attempt) => (
                  <li
                    className={`notification-history-item is-${attempt.result}`}
                    key={attempt.id}
                  >
                    <div className="notification-history-item-header">
                      <span>{formatResult(attempt.result)}</span>
                      <strong>
                        {formatTarget(attempt.target.type)} /{" "}
                        {formatKind(attempt.kind)}
                      </strong>
                    </div>
                    <dl>
                      <div>
                        <dt>予定</dt>
                        <dd>{formatDateTime(attempt.notifyAt)}</dd>
                      </div>
                      <div>
                        <dt>試行</dt>
                        <dd>
                          {formatDateTime(attempt.attemptedAt)} /{" "}
                          {attempt.attemptCount}回目
                        </dd>
                      </div>
                      {attempt.errorMessage ? (
                        <div>
                          <dt>理由</dt>
                          <dd>{attempt.errorMessage}</dd>
                        </div>
                      ) : null}
                    </dl>
                  </li>
                ))}
              </ol>
            ) : (
              <p className="notification-history-empty">
                現在確認が必要な通知失敗履歴はありません。
              </p>
            )}
          </div>
        </section>

        <section
          className="settings-section data-management-section"
          aria-labelledby="data-management-title"
          aria-busy={isDataManagementBusy}
        >
          <div className="settings-section-heading">
            <div>
              <h3 id="data-management-title">データ管理</h3>
              <span>バックアップ、復元、エクスポート</span>
            </div>
          </div>

          <p className="settings-warning">
            バックアップとエクスポートにはタスク名、メモ、タイマー履歴が含まれる可能性があります。公開IssueやPRへ添付しないでください。
          </p>

          <div className="data-action-grid">
            <button
              className="secondary-button"
              type="button"
              disabled={isDataManagementBusy}
              onClick={() =>
                void runDataManagementAction(
                  "sqlite-backup",
                  onCreateSqliteBackup,
                )
              }
            >
              {activeDataOperation === "sqlite-backup"
                ? "作成中"
                : "SQLiteバックアップを作成"}
            </button>
            <button
              className="danger-button"
              type="button"
              disabled={isDataManagementBusy}
              onClick={() =>
                void runDataManagementAction(
                  "sqlite-restore",
                  onRestoreSqliteBackup,
                )
              }
            >
              {activeDataOperation === "sqlite-restore"
                ? "復元中"
                : "SQLiteバックアップから復元"}
            </button>
            <button
              className="secondary-button"
              type="button"
              disabled={isDataManagementBusy}
              onClick={() =>
                void runDataManagementAction("json-export", onCreateJsonExport)
              }
            >
              {activeDataOperation === "json-export"
                ? "作成中"
                : "JSONエクスポート"}
            </button>
            <button
              className="secondary-button"
              type="button"
              disabled={isDataManagementBusy}
              onClick={() =>
                void runDataManagementAction("csv-export", onCreateCsvExport)
              }
            >
              {activeDataOperation === "csv-export"
                ? "作成中"
                : "CSVエクスポート"}
            </button>
          </div>

          {dataManagementResult ? (
            <div
              className={`data-management-status is-${dataManagementResult.status}`}
              role={dataManagementResult.status === "failed" ? "alert" : "status"}
              aria-live="polite"
            >
              <strong>{dataManagementResult.message}</strong>
              {dataManagementResult.detail ? (
                <span>{dataManagementResult.detail}</span>
              ) : null}
            </div>
          ) : null}
        </section>
      </div>
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

function formatTarget(targetType: NotificationDeliveryAttempt["target"]["type"]) {
  return targetType === "subtask" ? "サブタスク" : "タスク";
}

function formatKind(kind: NotificationDeliveryAttempt["kind"]) {
  return kind === "planned_start" ? "開始予定" : "期限";
}

function formatResult(result: NotificationDeliveryAttempt["result"]) {
  return result === "success" ? "成功" : "失敗";
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return new Intl.DateTimeFormat("ja-JP", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}
