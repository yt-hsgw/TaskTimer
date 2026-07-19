import { Pause, Play, RotateCcw, SkipForward, Square } from "lucide-react";
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

      <div className={`pomodoro-focus is-${phase}`}>
        <p className="pomodoro-phase-label">{formatPhase(phase)}</p>
        <strong className="pomodoro-focus-countdown" aria-live="polite">
          {formatDuration(remainingSeconds)}
        </strong>
        <p className="pomodoro-cycle-label">
          {activePomodoro
            ? `${activePomodoro.cycleCount}セット完了`
            : `作業 ${formatMinutes(settings?.workSeconds ?? 25 * 60)}分`}
        </p>

        <div className="pomodoro-focus-actions">
          {!activePomodoro ? (
            <button
              className="primary-button pomodoro-primary-action"
              type="button"
              disabled={Boolean(activeTimer) || isMutating}
              title={activeTimer ? "通常タイマーを終了してから開始してください" : "開始"}
              onClick={() => void onStart()}
            >
              <Play aria-hidden="true" size={18} />
              開始
            </button>
          ) : (
            <>
              <button
                className="icon-button"
                type="button"
                aria-label={isPaused ? "再開" : "一時停止"}
                title={isPaused ? "再開" : "一時停止"}
                disabled={isMutating}
                onClick={() => void (isPaused ? onResume() : onPause())}
              >
                {isPaused ? <Play aria-hidden="true" size={18} /> : <Pause aria-hidden="true" size={18} />}
              </button>

              {phase === "work" ? (
                <>
                  <button
                    className="secondary-button"
                    type="button"
                    disabled={isMutating}
                    onClick={() => void onCompleteWorkAndStartBreak()}
                  >
                    休憩を開始
                  </button>
                  <button
                    className="secondary-button"
                    type="button"
                    disabled={isMutating}
                    onClick={() => void onCompleteWork()}
                  >
                    作業を完了
                  </button>
                </>
              ) : (
                <>
                  <button
                    className="secondary-button"
                    type="button"
                    disabled={isMutating}
                    onClick={() => void onCompleteBreakAndStartNext()}
                  >
                    <RotateCcw aria-hidden="true" size={16} />
                    次の作業
                  </button>
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
              )}

              <button
                className="stop-button"
                type="button"
                disabled={isMutating}
                onClick={() => void onCancel()}
              >
                <Square aria-hidden="true" size={15} />
                終了
              </button>
            </>
          )}
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
