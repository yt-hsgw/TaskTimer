import { useEffect, useState, type Dispatch, type SetStateAction } from "react";
import type { NotificationDisplayMode } from "../../domain/notification/types";
import type {
  NotificationDispatchSummary,
  PomodoroSettings,
  PomodoroSettingsDraft,
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
  pomodoroSettings: PomodoroSettings | null;
  isMutating: boolean;
  notificationSummary: NotificationDispatchSummary | null;
  onUpdateDisplayMode(displayMode: NotificationDisplayMode): Promise<boolean>;
  onUpdateNotificationsEnabled(enabled: boolean): Promise<boolean>;
  onUpdatePomodoroSettings(input: PomodoroSettingsDraft): Promise<boolean>;
  onRetryNotifications(): Promise<boolean>;
  onCreateJsonExport(): Promise<DataManagementActionResult>;
  onCreateCsvExport(): Promise<DataManagementActionResult>;
};

export function SettingsPanel({
  displayMode,
  notificationsEnabled,
  pomodoroSettings,
  isMutating,
  notificationSummary,
  onUpdateDisplayMode,
  onUpdateNotificationsEnabled,
  onUpdatePomodoroSettings,
  onRetryNotifications,
  onCreateJsonExport,
  onCreateCsvExport,
}: SettingsPanelProps) {
  usePresentationRenderProbe("SettingsPanel");
  const [activeDataOperation, setActiveDataOperation] =
    useState<DataManagementOperation | null>(null);
  const [dataManagementResult, setDataManagementResult] =
    useState<DataManagementActionResult | null>(null);
  const [pomodoroDraft, setPomodoroDraft] = useState(() =>
    createPomodoroDraft(pomodoroSettings),
  );
  const [pomodoroSaveMessage, setPomodoroSaveMessage] = useState<string | null>(
    null,
  );
  const isDataManagementBusy = isMutating || activeDataOperation !== null;
  const pomodoroValidationError = pomodoroSettings
    ? validatePomodoroDraft(pomodoroDraft)
    : null;
  const hasPomodoroChanges = pomodoroSettings
    ? hasPomodoroDraftChanges(pomodoroDraft, pomodoroSettings)
    : false;

  useEffect(() => {
    setPomodoroDraft(createPomodoroDraft(pomodoroSettings));
    setPomodoroSaveMessage(null);
  }, [pomodoroSettings]);

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

  const handlePomodoroSubmit = async () => {
    if (!pomodoroSettings) {
      return;
    }
    if (pomodoroValidationError) {
      setPomodoroSaveMessage(pomodoroValidationError);
      return;
    }

    const updated = await onUpdatePomodoroSettings({
      workSeconds: Number(pomodoroDraft.workMinutes) * 60,
      shortBreakSeconds: Number(pomodoroDraft.shortBreakMinutes) * 60,
      longBreakSeconds: Number(pomodoroDraft.longBreakMinutes) * 60,
      cyclesUntilLongBreak: Number(pomodoroDraft.cyclesUntilLongBreak),
      autoStartBreak: pomodoroSettings.autoStartBreak,
      autoStartNextWork: pomodoroSettings.autoStartNextWork,
    });
    setPomodoroSaveMessage(
      updated ? "ポモドーロ設定を保存しました。" : "ポモドーロ設定を保存できませんでした。",
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
          aria-labelledby="pomodoro-settings-title"
        >
          <div className="settings-section-heading">
            <div>
              <h3 id="pomodoro-settings-title">ポモドーロ</h3>
              <span>作業と休憩の既定値</span>
            </div>
          </div>

          <form
            className="pomodoro-settings-form"
            onSubmit={(event) => {
              event.preventDefault();
              void handlePomodoroSubmit();
            }}
          >
            <div className="pomodoro-settings-grid">
              <label className="field-group" htmlFor="pomodoro-work-minutes">
                作業時間（分）
                <input
                  id="pomodoro-work-minutes"
                  type="number"
                  min="1"
                  max="1440"
                  step="1"
                  inputMode="numeric"
                  value={pomodoroDraft.workMinutes}
                  disabled={isMutating || !pomodoroSettings}
                  onChange={(event) =>
                    updatePomodoroDraftField(
                      setPomodoroDraft,
                      "workMinutes",
                      event.target.value,
                      setPomodoroSaveMessage,
                    )
                  }
                />
              </label>

              <label
                className="field-group"
                htmlFor="pomodoro-short-break-minutes"
              >
                短い休憩（分）
                <input
                  id="pomodoro-short-break-minutes"
                  type="number"
                  min="1"
                  max="1440"
                  step="1"
                  inputMode="numeric"
                  value={pomodoroDraft.shortBreakMinutes}
                  disabled={isMutating || !pomodoroSettings}
                  onChange={(event) =>
                    updatePomodoroDraftField(
                      setPomodoroDraft,
                      "shortBreakMinutes",
                      event.target.value,
                      setPomodoroSaveMessage,
                    )
                  }
                />
              </label>

              <label
                className="field-group"
                htmlFor="pomodoro-long-break-minutes"
              >
                長い休憩（分）
                <input
                  id="pomodoro-long-break-minutes"
                  type="number"
                  min="1"
                  max="1440"
                  step="1"
                  inputMode="numeric"
                  value={pomodoroDraft.longBreakMinutes}
                  disabled={isMutating || !pomodoroSettings}
                  onChange={(event) =>
                    updatePomodoroDraftField(
                      setPomodoroDraft,
                      "longBreakMinutes",
                      event.target.value,
                      setPomodoroSaveMessage,
                    )
                  }
                />
              </label>

              <label className="field-group" htmlFor="pomodoro-cycle-count">
                長い休憩までの作業回数
                <input
                  id="pomodoro-cycle-count"
                  type="number"
                  min="1"
                  max="12"
                  step="1"
                  inputMode="numeric"
                  value={pomodoroDraft.cyclesUntilLongBreak}
                  disabled={isMutating || !pomodoroSettings}
                  onChange={(event) =>
                    updatePomodoroDraftField(
                      setPomodoroDraft,
                      "cyclesUntilLongBreak",
                      event.target.value,
                      setPomodoroSaveMessage,
                    )
                  }
                />
              </label>
            </div>

            {pomodoroValidationError ? (
              <p className="settings-warning">{pomodoroValidationError}</p>
            ) : null}

            {pomodoroSaveMessage ? (
              <div
                className={`settings-status ${
                  pomodoroSaveMessage.includes("できません")
                    ? "is-failed"
                    : "is-success"
                }`}
                role={pomodoroSaveMessage.includes("できません") ? "alert" : "status"}
                aria-live="polite"
              >
                {pomodoroSaveMessage}
              </div>
            ) : null}

            <div className="settings-form-actions">
              <button
                className="primary-button"
                type="submit"
                disabled={
                  isMutating ||
                  !pomodoroSettings ||
                  !hasPomodoroChanges ||
                  Boolean(pomodoroValidationError)
                }
              >
                保存
              </button>
            </div>
          </form>
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

type PomodoroDraftState = {
  workMinutes: string;
  shortBreakMinutes: string;
  longBreakMinutes: string;
  cyclesUntilLongBreak: string;
};

type PomodoroDraftField = keyof PomodoroDraftState;

function createPomodoroDraft(
  settings: PomodoroSettings | null,
): PomodoroDraftState {
  return {
    workMinutes: settings ? secondsToMinutes(settings.workSeconds) : "",
    shortBreakMinutes: settings ? secondsToMinutes(settings.shortBreakSeconds) : "",
    longBreakMinutes: settings ? secondsToMinutes(settings.longBreakSeconds) : "",
    cyclesUntilLongBreak: settings
      ? String(settings.cyclesUntilLongBreak)
      : "",
  };
}

function updatePomodoroDraftField(
  setDraft: Dispatch<SetStateAction<PomodoroDraftState>>,
  field: PomodoroDraftField,
  value: string,
  setMessage: Dispatch<SetStateAction<string | null>>,
) {
  setMessage(null);
  setDraft((current) => ({
    ...current,
    [field]: value,
  }));
}

function validatePomodoroDraft(draft: PomodoroDraftState) {
  const durationFields = [
    ["作業時間", draft.workMinutes],
    ["短い休憩", draft.shortBreakMinutes],
    ["長い休憩", draft.longBreakMinutes],
  ] as const;
  for (const [label, value] of durationFields) {
    if (!isIntegerTextInRange(value, 1, 1440)) {
      return `${label}は1分以上1440分以下で入力してください。`;
    }
  }
  if (!isIntegerTextInRange(draft.cyclesUntilLongBreak, 1, 12)) {
    return "長い休憩までの作業回数は1回以上12回以下で入力してください。";
  }
  return null;
}

function hasPomodoroDraftChanges(
  draft: PomodoroDraftState,
  settings: PomodoroSettings,
) {
  return (
    Number(draft.workMinutes) * 60 !== settings.workSeconds ||
    Number(draft.shortBreakMinutes) * 60 !== settings.shortBreakSeconds ||
    Number(draft.longBreakMinutes) * 60 !== settings.longBreakSeconds ||
    Number(draft.cyclesUntilLongBreak) !== settings.cyclesUntilLongBreak
  );
}

function secondsToMinutes(seconds: number) {
  return String(Math.max(1, Math.floor(seconds / 60)));
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
