/**
 * 開発・性能検証時にPresentationの再描画回数を観測する補助フック。
 * ユーザー入力本文を計測ログへ含めない。
 *
 * @packageDocumentation
 */
import { useEffect } from "react";

type RenderProbeGlobal = typeof globalThis & {
  __TASKTIMER_RENDER_COUNTS__?: Record<string, number>;
};

export function usePresentationRenderProbe(componentName: string) {
  useEffect(() => {
    const counts = (globalThis as RenderProbeGlobal)
      .__TASKTIMER_RENDER_COUNTS__;
    if (!counts) {
      return;
    }
    counts[componentName] = (counts[componentName] ?? 0) + 1;
  });
}
