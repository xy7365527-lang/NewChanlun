import type { StatusResponse } from "../types";
import { createDaemonAPI } from "./daemonApi.ts";

export interface StatusPollingInstance {
  id: string;
  httpBase: string;
}

export interface StatusPollingOptions {
  instances: readonly StatusPollingInstance[];
  intervalMs: number;
  timeoutMs: number;
  onStatus: (
    instance: StatusPollingInstance,
    status: StatusResponse,
    firstReachable: boolean,
  ) => void;
  onError: (instance: StatusPollingInstance, error: unknown) => void;
  onCycleComplete: (reachable: boolean) => void;
}

interface CompatibleAbortScope {
  signal: AbortSignal;
  cleanup: () => void;
}

function createCompatibleAbortScope(
  timeoutMs: number,
  externalSignal?: AbortSignal,
): CompatibleAbortScope {
  const controller = new AbortController();
  const abort = () => controller.abort();
  const timeout = setTimeout(abort, timeoutMs);

  if (externalSignal) {
    if (externalSignal.aborted) abort();
    else externalSignal.addEventListener("abort", abort, { once: true });
  }

  return {
    signal: controller.signal,
    cleanup: () => {
      clearTimeout(timeout);
      externalSignal?.removeEventListener("abort", abort);
    },
  };
}

export function startStatusPolling(options: StatusPollingOptions): () => void {
  const {
    instances,
    intervalMs,
    timeoutMs,
    onStatus,
    onError,
    onCycleComplete,
  } = options;
  let disposed = false;
  let nextCycleTimer: ReturnType<typeof setTimeout> | null = null;
  let activeController: AbortController | null = null;

  function startCycle(): void {
    void runCycle().catch(() => {
      // Callback failures must not become unhandled rejections or stop polling.
    });
  }

  function scheduleNextCycle(): void {
    if (disposed || nextCycleTimer !== null) return;
    nextCycleTimer = setTimeout(() => {
      nextCycleTimer = null;
      startCycle();
    }, intervalMs);
  }

  async function runCycle(): Promise<void> {
    if (disposed || activeController !== null) return;

    const controller = new AbortController();
    activeController = controller;
    const abortScope = createCompatibleAbortScope(timeoutMs, controller.signal);
    let reachable = false;

    try {
      await Promise.allSettled(instances.map(async (instance) => {
        try {
          const status = await createDaemonAPI(instance.httpBase).status(abortScope.signal);
          if (disposed || activeController !== controller) return;
          const firstReachable = !reachable;
          reachable = true;
          onStatus(instance, status, firstReachable);
        } catch (error) {
          if (disposed || activeController !== controller) return;
          onError(instance, error);
        }
      }));
    } finally {
      abortScope.cleanup();
      if (disposed || activeController !== controller) return;
      activeController = null;
      try {
        onCycleComplete(reachable);
      } finally {
        scheduleNextCycle();
      }
    }
  }

  startCycle();

  return () => {
    if (disposed) return;
    disposed = true;
    if (nextCycleTimer !== null) {
      clearTimeout(nextCycleTimer);
      nextCycleTimer = null;
    }
    activeController?.abort();
    activeController = null;
  };
}
