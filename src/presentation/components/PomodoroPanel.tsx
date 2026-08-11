/**
 * タスクから独立したポモドーロ集中画面。
 * 通常タイマーとは機能を分けるが、同時にアクティブな作業は1件という制約を共有する。
 *
 * @packageDocumentation
 */
import {
  Coffee,
  Pause,
  Play,
  RotateCcw,
  SkipForward,
  Square,
} from "lucide-react";
import { useEffect, useState, type CSSProperties } from "react";
import type {
  ActivePomodoro,
  PomodoroSettings,
  PomodoroSettingsDraft,
} from "../../application/usecases/contracts";
import type { ActiveTimer } from "../../domain/timer/types";
import { usePresentationRenderProbe } from "../renderProbe";

type PomodoroPanelProps = {
  activePomodoro: ActivePomodoro | null;
  activeTimer: ActiveTimer | null;
  settings: PomodoroSettings | null;
  isMutating: boolean;
  onStart(): Promise<boolean>;
  onPause(): Promise<boolean>;
  onResume(): Promise<boolean>;
  onCompleteWork(): Promise<boolean>;
  onCompleteWorkAndStartBreak(): Promise<boolean>;
  onSkipBreak(pomodoroSessionId: string): Promise<boolean>;
  onCompleteBreak(): Promise<boolean>;
  onCompleteBreakAndStartNext(): Promise<boolean>;
  onCancel(): Promise<boolean>;
  onUpdateSettings(input: PomodoroSettingsDraft): Promise<boolean>;
};

export function PomodoroPanel({
  activePomodoro,
  activeTimer,
  settings,
  isMutating,
  onStart,
  onPause,
  onResume,
  onCompleteWork,
  onCompleteWorkAndStartBreak,
  onSkipBreak,
  onCompleteBreak,
  onCompleteBreakAndStartNext,
  onCancel,
  onUpdateSettings,
}: PomodoroPanelProps) {
  usePresentationRenderProbe("PomodoroPanel");
  const [now, setNow] = useState(Date.now());
  const [settingsDraft, setSettingsDraft] = useState(() =>
    createPomodoroDraft(settings),
  );
  const [settingsMessage, setSettingsMessage] = useState<string | null>(null);

  useEffect(() => {
    setNow(Date.now());
    if (!activePomodoro || activePomodoro.status !== "running") {
      return;
    }
    const timerId = window.setInterval(() => setNow(Date.now()), 1_000);
    return () => window.clearInterval(timerId);
  }, [activePomodoro]);

  useEffect(() => {
    setSettingsDraft(createPomodoroDraft(settings));
    setSettingsMessage(null);
  }, [settings]);

  const phase = activePomodoro?.phase ?? "work";
  const isPaused = activePomodoro?.status === "paused";
  const remainingSeconds = activePomodoro
    ? getRemainingSeconds(activePomodoro, now)
    : settings?.workSeconds ?? 25 * 60;
  const durationSeconds = activePomodoro?.phaseDurationSeconds ?? remainingSeconds;
  const remainingPercent = getRemainingPercent(durationSeconds, remainingSeconds);
  const phaseLabel = formatPhase(phase);
  const primaryActionLabel = !activePomodoro
    ? "開始"
    : phase === "work"
      ? "休憩を開始"
      : "次の作業";
  const primaryActionDisabled = isMutating || (!activePomodoro && Boolean(activeTimer));
  const settingsValidationError = settings
    ? validatePomodoroDraft(settingsDraft)
    : null;
  const hasSettingsChanges = settings
    ? hasPomodoroDraftChanges(settingsDraft, settings)
    : false;

  const runPrimaryAction = () => {
    if (!activePomodoro) return onStart();
    if (phase === "work") return onCompleteWorkAndStartBreak();
    return onCompleteBreakAndStartNext();
  };

  const updateSettingsDraft = (
    field: keyof PomodoroDraftState,
    value: string | boolean,
  ) => {
    setSettingsMessage(null);
    setSettingsDraft((current) => ({ ...current, [field]: value }));
  };

  const handleSettingsSubmit = async () => {
    if (!settings) {
      return;
    }
    if (settingsValidationError) {
      setSettingsMessage(settingsValidationError);
      return;
    }

    const updated = await onUpdateSettings({
      workSeconds: Number(settingsDraft.workMinutes) * 60,
      shortBreakSeconds: Number(settingsDraft.shortBreakMinutes) * 60,
      longBreakSeconds: Number(settingsDraft.longBreakMinutes) * 60,
      cyclesUntilLongBreak: Number(settingsDraft.cyclesUntilLongBreak),
      autoStartBreak: settingsDraft.autoStartBreak,
      autoStartNextWork: settingsDraft.autoStartNextWork,
    });
    setSettingsMessage(
      updated ? "ポモドーロ設定を保存しました。" : "ポモドーロ設定を保存できませんでした。",
    );
  };

  return (
    <section className="pomodoro-panel" aria-labelledby="pomodoro-title">
      <header className="panel-heading pomodoro-panel-heading">
        <div>
          <span>集中</span>
          <h2 id="pomodoro-title">ポモドーロ</h2>
        </div>
        {activePomodoro?.scope === "task_linked" ? (
          <span className="legacy-session-badge">旧形式・タスク連携</span>
        ) : null}
      </header>

      <div className={`pomodoro-focus is-${phase}${isPaused ? " is-paused" : ""}`}>
        <div
          className="pomodoro-progress"
          style={
            {
              "--pomodoro-remaining-angle": `${remainingPercent * 3.6}deg`,
            } as CSSProperties
          }
          role="progressbar"
          aria-label={`${phaseLabel}の時間進捗`}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(remainingPercent)}
          aria-valuetext={`${phaseLabel} 残り${formatDuration(remainingSeconds)}`}
        >
          <span className="pomodoro-progress-face" aria-hidden="true" />
          <div className="pomodoro-progress-content">
            <p className="pomodoro-phase-label">
              {phaseLabel}
              <span
                className={isPaused ? "" : "is-hidden"}
                aria-hidden={!isPaused}
              >
                一時停止
              </span>
            </p>
            <strong className="pomodoro-focus-countdown" aria-live="polite">
              {formatDuration(remainingSeconds)}
            </strong>
            <p className="pomodoro-cycle-label">
              {activePomodoro
                ? `${activePomodoro.cycleCount}セット完了`
                : `作業 ${formatMinutes(settings?.workSeconds ?? 25 * 60)}分`}
            </p>
          </div>
        </div>

        <div className="pomodoro-focus-actions">
          <div className="pomodoro-control-grid" aria-label="ポモドーロの主要操作">
            {activePomodoro ? (
              <button
                className="icon-button pomodoro-control-button"
                type="button"
                aria-label={isPaused ? "再開" : "一時停止"}
                title={isPaused ? "再開" : "一時停止"}
                disabled={isMutating}
                onClick={() => void (isPaused ? onResume() : onPause())}
              >
                {isPaused ? (
                  <Play aria-hidden="true" size={18} />
                ) : (
                  <Pause aria-hidden="true" size={18} />
                )}
              </button>
            ) : (
              <span className="pomodoro-control-placeholder" aria-hidden="true" />
            )}

            <button
              className="primary-button pomodoro-primary-action"
              type="button"
              disabled={primaryActionDisabled}
              title={
                activeTimer && !activePomodoro
                  ? "通常タイマーを終了してから開始してください"
                  : primaryActionLabel
              }
              aria-describedby={
                activeTimer && !activePomodoro
                  ? "pomodoro-start-disabled-reason"
                  : undefined
              }
              onClick={() => void runPrimaryAction()}
            >
              {!activePomodoro ? (
                <Play aria-hidden="true" size={18} />
              ) : phase === "work" ? (
                <Coffee aria-hidden="true" size={18} />
              ) : (
                <RotateCcw aria-hidden="true" size={18} />
              )}
              {primaryActionLabel}
            </button>

            {activePomodoro ? (
              <button
                className="stop-button pomodoro-control-button"
                type="button"
                aria-label="終了"
                title="終了"
                disabled={isMutating}
                onClick={() => void onCancel()}
              >
                <Square aria-hidden="true" size={15} />
              </button>
            ) : (
              <span className="pomodoro-control-placeholder" aria-hidden="true" />
            )}
          </div>

          <div className="pomodoro-secondary-actions" aria-label="ポモドーロの補助操作">
            {phase === "work" && activePomodoro ? (
              <>
                <button
                  className="secondary-button"
                  type="button"
                  disabled={isMutating}
                  onClick={() => void onCompleteWork()}
                >
                  作業を完了
                </button>
                <span className="pomodoro-secondary-placeholder" aria-hidden="true" />
              </>
            ) : null}

            {phase !== "work" && activePomodoro ? (
              <>
                <button
                  className="secondary-button"
                  type="button"
                  disabled={isMutating}
                  onClick={() => void onCompleteBreak()}
                >
                  休憩を完了
                </button>
                <button
                  className="secondary-button"
                  type="button"
                  disabled={isMutating}
                  onClick={() => void onSkipBreak(activePomodoro.id)}
                >
                  <SkipForward aria-hidden="true" size={16} />
                  スキップ
                </button>
              </>
            ) : null}

            {!activePomodoro ? (
              <>
                <span className="pomodoro-secondary-placeholder" aria-hidden="true" />
                <span className="pomodoro-secondary-placeholder" aria-hidden="true" />
              </>
            ) : null}
          </div>

          {activeTimer && !activePomodoro ? (
            <span id="pomodoro-start-disabled-reason" className="visually-hidden">
              通常タイマーを終了してから開始してください
            </span>
          ) : null}
        </div>
      </div>

      <form
        className="pomodoro-settings-form pomodoro-panel-settings"
        aria-labelledby="pomodoro-panel-settings-title"
        onSubmit={(event) => {
          event.preventDefault();
          void handleSettingsSubmit();
        }}
      >
        <div className="pomodoro-panel-settings-heading">
          <div>
            <h3 id="pomodoro-panel-settings-title">設定</h3>
            <span>作業と休憩の既定値</span>
          </div>
          <button
            className="primary-button"
            type="submit"
            disabled={
              isMutating ||
              !settings ||
              !hasSettingsChanges ||
              Boolean(settingsValidationError)
            }
          >
            保存
          </button>
        </div>

        <div className="pomodoro-settings-grid">
          <label className="field-group" htmlFor="pomodoro-panel-work-minutes">
            作業時間（分）
            <input
              id="pomodoro-panel-work-minutes"
              type="number"
              min="1"
              max="1440"
              step="1"
              inputMode="numeric"
              value={settingsDraft.workMinutes}
              disabled={isMutating || !settings}
              onChange={(event) =>
                updateSettingsDraft("workMinutes", event.target.value)
              }
            />
          </label>

          <label
            className="field-group"
            htmlFor="pomodoro-panel-short-break-minutes"
          >
            短い休憩（分）
            <input
              id="pomodoro-panel-short-break-minutes"
              type="number"
              min="1"
              max="1440"
              step="1"
              inputMode="numeric"
              value={settingsDraft.shortBreakMinutes}
              disabled={isMutating || !settings}
              onChange={(event) =>
                updateSettingsDraft("shortBreakMinutes", event.target.value)
              }
            />
          </label>

          <label
            className="field-group"
            htmlFor="pomodoro-panel-long-break-minutes"
          >
            長い休憩（分）
            <input
              id="pomodoro-panel-long-break-minutes"
              type="number"
              min="1"
              max="1440"
              step="1"
              inputMode="numeric"
              value={settingsDraft.longBreakMinutes}
              disabled={isMutating || !settings}
              onChange={(event) =>
                updateSettingsDraft("longBreakMinutes", event.target.value)
              }
            />
          </label>

          <label className="field-group" htmlFor="pomodoro-panel-cycle-count">
            長い休憩までの作業回数
            <input
              id="pomodoro-panel-cycle-count"
              type="number"
              min="1"
              max="12"
              step="1"
              inputMode="numeric"
              value={settingsDraft.cyclesUntilLongBreak}
              disabled={isMutating || !settings}
              onChange={(event) =>
                updateSettingsDraft("cyclesUntilLongBreak", event.target.value)
              }
            />
          </label>
        </div>

        <div className="pomodoro-settings-toggles">
          <label>
            <input
              type="checkbox"
              checked={settingsDraft.autoStartBreak}
              disabled={isMutating || !settings}
              onChange={(event) =>
                updateSettingsDraft("autoStartBreak", event.target.checked)
              }
            />
            作業後に休憩を自動開始
          </label>
          <label>
            <input
              type="checkbox"
              checked={settingsDraft.autoStartNextWork}
              disabled={isMutating || !settings}
              onChange={(event) =>
                updateSettingsDraft("autoStartNextWork", event.target.checked)
              }
            />
            休憩後に次の作業を自動開始
          </label>
        </div>

        {settingsValidationError ? (
          <p className="settings-warning">{settingsValidationError}</p>
        ) : null}

        {settingsMessage ? (
          <div
            className={`settings-status ${
              settingsMessage.includes("できません") ||
              settingsMessage.includes("入力してください")
                ? "is-failed"
                : "is-success"
            }`}
            role={
              settingsMessage.includes("できません") ||
              settingsMessage.includes("入力してください")
                ? "alert"
                : "status"
            }
            aria-live="polite"
          >
            {settingsMessage}
          </div>
        ) : null}
      </form>
    </section>
  );
}

function getRemainingSeconds(active: ActivePomodoro, now: number) {
  const startedAt = new Date(active.phaseStartedAt).getTime();
  const pausedAt = active.pausedAt ? new Date(active.pausedAt).getTime() : null;
  const effectiveNow = active.status === "paused" && pausedAt ? pausedAt : now;
  if (Number.isNaN(startedAt) || Number.isNaN(effectiveNow)) {
    return active.phaseDurationSeconds;
  }
  const elapsed = Math.max(
    0,
    Math.floor((effectiveNow - startedAt) / 1_000) - active.pausedTotalSeconds,
  );
  return Math.max(0, active.phaseDurationSeconds - elapsed);
}

function getRemainingPercent(durationSeconds: number, remainingSeconds: number) {
  if (!Number.isFinite(durationSeconds) || durationSeconds <= 0) return 0;
  const boundedRemainingSeconds = Math.min(
    durationSeconds,
    Math.max(0, remainingSeconds),
  );
  return (boundedRemainingSeconds / durationSeconds) * 100;
}

function formatPhase(phase: ActivePomodoro["phase"]) {
  if (phase === "work") return "作業";
  if (phase === "long_break") return "長い休憩";
  return "短い休憩";
}

function formatDuration(totalSeconds: number) {
  const seconds = Math.max(0, Math.floor(totalSeconds));
  const minutes = Math.floor(seconds / 60);
  return `${String(minutes).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}

function formatMinutes(totalSeconds: number) {
  return Math.round(totalSeconds / 60);
}

type PomodoroDraftState = {
  workMinutes: string;
  shortBreakMinutes: string;
  longBreakMinutes: string;
  cyclesUntilLongBreak: string;
  autoStartBreak: boolean;
  autoStartNextWork: boolean;
};

function createPomodoroDraft(
  settings: PomodoroSettings | null,
): PomodoroDraftState {
  return {
    workMinutes: settings ? String(formatMinutes(settings.workSeconds)) : "",
    shortBreakMinutes: settings
      ? String(formatMinutes(settings.shortBreakSeconds))
      : "",
    longBreakMinutes: settings
      ? String(formatMinutes(settings.longBreakSeconds))
      : "",
    cyclesUntilLongBreak: settings
      ? String(settings.cyclesUntilLongBreak)
      : "",
    autoStartBreak: settings?.autoStartBreak ?? false,
    autoStartNextWork: settings?.autoStartNextWork ?? false,
  };
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
    Number(draft.cyclesUntilLongBreak) !== settings.cyclesUntilLongBreak ||
    draft.autoStartBreak !== settings.autoStartBreak ||
    draft.autoStartNextWork !== settings.autoStartNextWork
  );
}

function isIntegerTextInRange(value: string, min: number, max: number) {
  if (!/^\d+$/.test(value)) {
    return false;
  }
  const numberValue = Number(value);
  return Number.isInteger(numberValue) && numberValue >= min && numberValue <= max;
}
