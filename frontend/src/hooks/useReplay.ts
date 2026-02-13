import { useCallback } from "react";
import { useReplayStore } from "../store/replayStore";
import { useAppStore } from "../store/appStore";
import type { ReplayStatus } from "../types/events";

const API_BASE = "http://localhost:8766";

export interface UseReplayReturn {
  startReplay: () => Promise<void>;
  step: (count?: number) => Promise<void>;
  seek: (barIdx: number) => Promise<void>;
  play: (speed?: number) => Promise<void>;
  pause: () => Promise<void>;
  stop: () => void;
  isReplaying: boolean;
  status: ReplayStatus;
}

async function postJson<T = unknown>(url: string, body?: Record<string, unknown>): Promise<T> {
  const res = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${res.status} ${text}`);
  }
  return res.json() as Promise<T>;
}

export function useReplay(): UseReplayReturn {
  const { isReplaying, sessionId, status } = useReplayStore();
  const { setReplaying, setSessionId, updateStatus, clearEvents } = useReplayStore();
  const { symbol, tf } = useAppStore();

  const startReplay = useCallback(async () => {
    const data = await postJson<{ session_id: string; total_bars: number }>(
      `${API_BASE}/api/replay/start`,
      { symbol, tf },
    );
    setSessionId(data.session_id);
    setReplaying(true);
    updateStatus({ mode: "paused", currentIdx: 0, totalBars: data.total_bars, speed: 1 });
    clearEvents();
  }, [symbol, tf, setSessionId, setReplaying, updateStatus, clearEvents]);

  const step = useCallback(
    async (count = 1) => {
      if (!sessionId) return;
      await postJson(`${API_BASE}/api/replay/step`, {
        session_id: sessionId,
        count,
      });
    },
    [sessionId],
  );

  const seek = useCallback(
    async (barIdx: number) => {
      if (!sessionId) return;
      await postJson(`${API_BASE}/api/replay/seek`, {
        session_id: sessionId,
        bar_idx: barIdx,
      });
    },
    [sessionId],
  );

  const play = useCallback(
    async (speed = 1) => {
      if (!sessionId) return;
      await postJson(`${API_BASE}/api/replay/play`, {
        session_id: sessionId,
        speed,
      });
      updateStatus({ mode: "playing", speed });
    },
    [sessionId, updateStatus],
  );

  const pause = useCallback(async () => {
    if (!sessionId) return;
    await postJson(`${API_BASE}/api/replay/pause`, {
      session_id: sessionId,
    });
    updateStatus({ mode: "paused" });
  }, [sessionId, updateStatus]);

  const stop = useCallback(() => {
    setSessionId(null);
    setReplaying(false);
    updateStatus({ mode: "idle", currentIdx: 0, totalBars: 0, speed: 1 });
    clearEvents();
  }, [setSessionId, setReplaying, updateStatus, clearEvents]);

  return { startReplay, step, seek, play, pause, stop, isReplaying, status };
}
