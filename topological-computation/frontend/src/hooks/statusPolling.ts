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

  async function runCycle(): Promise<void> {
    if (disposed) return;

    const controller = new AbortController();
    activeController = controller;
    const signal = AbortSignal.any([
      controller.signal,
      AbortSignal.timeout(timeoutMs),
    ]);
    let reachable = false;

    await Promise.all(instances.map(async (instance) => {
      try {
        const status = await createDaemonAPI(instance.httpBase).status(signal);
        if (disposed || activeController !== controller) return;
        const firstReachable = !reachable;
        reachable = true;
        onStatus(instance, status, firstReachable);
      } catch (error) {
        if (disposed || activeController !== controller) return;
        onError(instance, error);
      }
    }));

    if (disposed || activeController !== controller) return;
    activeController = null;
    onCycleComplete(reachable);
    if (!disposed) {
      nextCycleTimer = setTimeout(() => {
        nextCycleTimer = null;
        void runCycle();
      }, intervalMs);
    }
  }

  void runCycle();

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
