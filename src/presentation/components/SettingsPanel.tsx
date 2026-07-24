import { useEffect, useState } from "react";
import type { NotificationDisplayMode } from "../../domain/notification/types";
import type {
  NotificationDispatchSummary,
  TaskTimerSettings,
  TaskTimerSettingsDraft,
} from "../../application/usecases/contracts";
import { usePresentationRenderProbe } from "../renderProbe";

export type DataManagementActionResult = {
  status: "success" | "failed" | "cancelled";
  message: string;
  detail?: string;
};

type DataManagementOperation = "json-export" | "csv-export";

type SettingsPanelProps = {
  displayMode: NotificationDisplayMode;
  notificationsEnabled: boolean;
  taskTimerSettings: TaskTimerSettings | null;
  isMutating: boolean;
  notificationSummary: NotificationDispatchSummary | null;
  onUpdateDisplayMode(displayMode: NotificationDisplayMode): Promise<boolean>;
  onUpdateNotificationsEnabled(enabled: boolean): Promise<boolean>;
  onUpdateTaskTimerSettings(input: TaskTimerSettingsDraft): Promise<boolean>;
  onRetryNotifications(): Promise<boolean>;
  onCreateJsonExport(): Promise<DataManagementActionResult>;
  onCreateCsvExport(): Promise<DataManagementActionResult>;
};

export function SettingsPanel({
  displayMode,
  notificationsEnabled,
  taskTimerSettings,
  isMutating,
  notificationSummary,
  onUpdateDisplayMode,
  onUpdateNotificationsEnabled,
  onUpdateTaskTimerSettings,
  onRetryNotifications,
  onCreateJsonExport,
  onCreateCsvExport,
}: SettingsPanelProps) {
  usePresentationRenderProbe("SettingsPanel");
  const [activeDataOperation, setActiveDataOperation] =
    useState<DataManagementOperation | null>(null);
  const [dataManagementResult, setDataManagementResult] =
    useState<DataManagementActionResult | null>(null);
  const [taskTimerMinutes, setTaskTimerMinutes] = useState(() =>
    secondsToMinutesText(taskTimerSettings?.defaultTargetSeconds),
  );
  const [taskTimerSaveMessage, setTaskTimerSaveMessage] = useState<string | null>(
    null,
  );
  const isDataManagementBusy = isMutating || activeDataOperation !== null;

  useEffect(() => {
    setTaskTimerMinutes(
      secondsToMinutesText(taskTimerSettings?.defaultTargetSeconds),
    );
    setTaskTimerSaveMessage(null);
  }, [taskTimerSettings]);

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
        message: "エクスポートに失敗しました。",
        detail: "保存先の権限と空き容量を確認してください。",
      });
    } finally {
      setActiveDataOperation(null);
    }
  };

  const handleTaskTimerSubmit = async () => {
    const minutes = Number(taskTimerMinutes);
    if (!Number.isInteger(minutes) || minutes < 1 || minutes > 1440) {
      setTaskTimerSaveMessage("1分以上1,440分以内で入力してください。");
      return;
    }
    const updated = await onUpdateTaskTimerSettings({
      defaultTargetSeconds: minutes * 60,
    });
    setTaskTimerSaveMessage(
      updated
        ? "タスクタイマー設定を保存しました。"
        : "タスクタイマー設定を保存できませんでした。",
    );
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

          <fieldset className="notification-mode-group">
            <legend>表示タイプ</legend>
            <div className="notification-mode-cards">
              <label
                className={`notification-mode-card ${
                  displayMode === "title_only" ? "is-selected" : ""
                }`}
              >
                <input
                  type="radio"
                  name="notification-mode"
                  value="title_only"
                  checked={displayMode === "title_only"}
                  disabled={isMutating}
                  onChange={() => void onUpdateDisplayMode("title_only")}
                />
                <span>
                  <strong>タイトルのみ</strong>
                  <small>通知にタスク名を表示します。</small>
                </span>
              </label>
              <label
                className={`notification-mode-card ${
                  displayMode === "generic" ? "is-selected" : ""
                }`}
              >
                <input
                  type="radio"
                  name="notification-mode"
                  value="generic"
                  checked={displayMode === "generic"}
                  disabled={isMutating}
                  onChange={() => void onUpdateDisplayMode("generic")}
                />
                <span>
                  <strong>汎用メッセージ</strong>
                  <small>タスク名を通知に表示しません。</small>
                </span>
              </label>
            </div>
          </fieldset>

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

        <section
          className="settings-section"
          aria-labelledby="task-timer-settings-title"
        >
          <div className="settings-section-heading">
            <div>
              <h3 id="task-timer-settings-title">タスクタイマー</h3>
              <span>タスクに時間が未設定の場合の既定値</span>
            </div>
          </div>

          <form
            className="task-timer-settings-form"
            onSubmit={(event) => {
              event.preventDefault();
              void handleTaskTimerSubmit();
            }}
          >
            <label className="field-group" htmlFor="task-timer-default-minutes">
              既定時間（分）
              <input
                id="task-timer-default-minutes"
                type="number"
                min="1"
                max="1440"
                step="1"
                inputMode="numeric"
                value={taskTimerMinutes}
                disabled={isMutating || !taskTimerSettings}
                onChange={(event) => {
                  setTaskTimerMinutes(event.target.value);
                  setTaskTimerSaveMessage(null);
                }}
              />
            </label>
            <button
              className="primary-button"
              type="submit"
              disabled={isMutating || !taskTimerSettings}
            >
              保存
            </button>
          </form>
          {taskTimerSaveMessage ? (
            <div
              className={`settings-status ${
                taskTimerSaveMessage.includes("できません") ||
                taskTimerSaveMessage.includes("入力してください")
                  ? "is-failed"
                  : "is-success"
              }`}
              role={
                taskTimerSaveMessage.includes("できません") ||
                taskTimerSaveMessage.includes("入力してください")
                  ? "alert"
                  : "status"
              }
              aria-live="polite"
            >
              {taskTimerSaveMessage}
            </div>
          ) : null}
        </section>

        <section
          className="settings-section data-management-section"
          aria-labelledby="export-title"
          aria-busy={isDataManagementBusy}
        >
          <div className="settings-section-heading">
            <div>
              <h3 id="export-title">エクスポート</h3>
              <span>JSONまたはCSVで保存</span>
            </div>
          </div>

          <p className="settings-warning">
            エクスポートにはタスク名、メモ、タイマー履歴が含まれる可能性があります。公開IssueやPRへ添付しないでください。
          </p>

          <div className="data-action-grid">
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

function secondsToMinutes(seconds: number) {
  return String(Math.max(1, Math.floor(seconds / 60)));
}

function secondsToMinutesText(seconds: number | undefined) {
  return seconds ? secondsToMinutes(seconds) : "";
}

function isIntegerTextInRange(value: string, min: number, max: number) {
  if (!/^\d+$/.test(value)) {
    return false;
  }
  const numberValue = Number(value);
  return Number.isInteger(numberValue) && numberValue >= min && numberValue <= max;
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
