import type { WorkTargetRef } from "../task/types";

export type TimerSession = {
  id: string;
  target: WorkTargetRef;
  startedAt: string;
  stoppedAt: string | null;
  elapsedSeconds: number | null;
  pausedAt: string | null;
  deletedAt: string | null;
  createdAt: string;
};

export type ActiveTimer = TimerSession & {
  stoppedAt: null;
};
