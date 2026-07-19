import {
  Coffee,
  Pause,
  Play,
  RotateCcw,
  SkipForward,
  Square,
} from "lucide-react";
import { useEffect, useState } from "react";
import type {
  ActivePomodoro,
  PomodoroSettings,
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
}: PomodoroPanelProps) {
  usePresentationRenderProbe("PomodoroPanel");
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    setNow(Date.now());
    if (!activePomodoro || activePomodoro.status !== "running") {
      return;
    }
    const timerId = window.setInterval(() => setNow(Date.now()), 1_000);
    return () => window.clearInterval(timerId);
  }, [activePomodoro]);

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

  const runPrimaryAction = () => {
    if (!activePomodoro) return onStart();
    if (phase === "work") return onCompleteWorkAndStartBreak();
    return onCompleteBreakAndStartNext();
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
          role="progressbar"
          aria-label={`${phaseLabel}の時間進捗`}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(remainingPercent)}
          aria-valuetext={`${phaseLabel} 残り${formatDuration(remainingSeconds)}`}
        >
          <svg
            className="pomodoro-progress-ring"
            viewBox="0 0 240 240"
            aria-hidden="true"
          >
            <circle className="pomodoro-progress-track" cx="120" cy="120" r="104" />
            <circle
              className="pomodoro-progress-value"
              cx="120"
              cy="120"
              r="104"
              pathLength="100"
              strokeDasharray="100"
              strokeDashoffset={100 - remainingPercent}
            />
          </svg>
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

      <div className="pomodoro-settings-summary" aria-label="現在のポモドーロ設定">
        <span>作業 <strong>{formatMinutes(settings?.workSeconds ?? 25 * 60)}分</strong></span>
        <span>短い休憩 <strong>{formatMinutes(settings?.shortBreakSeconds ?? 5 * 60)}分</strong></span>
        <span>長い休憩 <strong>{formatMinutes(settings?.longBreakSeconds ?? 15 * 60)}分</strong></span>
        <span>長い休憩まで <strong>{settings?.cyclesUntilLongBreak ?? 4}セット</strong></span>
      </div>
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
