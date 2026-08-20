import type { WsMessage } from "../types";

export const DEFAULT_WS_RECONNECT_MS = 3_000;

export interface DaemonWebSocketDefaultTarget {
  id: string;
  wsUrl: string;
}

export interface DaemonWebSocketTarget {
  instanceId: string;
  wsUrl: string;
}

export function resolveDaemonWebSocketTarget(
  defaults: readonly DaemonWebSocketDefaultTarget[],
  explicit: Partial<DaemonWebSocketTarget> = {},
): DaemonWebSocketTarget {
  const first = defaults[0];
  const instanceId = explicit.instanceId ?? first?.id;
  const wsUrl = explicit.wsUrl ?? first?.wsUrl;
  if (!instanceId || !wsUrl) {
    throw new Error("A daemon WebSocket target requires both instanceId and wsUrl");
  }
  return { instanceId, wsUrl };
}

export interface DaemonWebSocketState {
  wsConnected: boolean;
  wsError: string | null;
}

export interface DaemonWebSocketOptions {
  wsUrl: string;
  instanceId: string;
  throttleMs: number;
  reconnectMs: number;
  describeError: (url: string, cause?: unknown) => string;
  onBatch: (instanceId: string, messages: WsMessage[]) => void;
  onState: (instanceId: string, state: DaemonWebSocketState) => void;
}

export function connectDaemonWebSocket(options: DaemonWebSocketOptions): () => void {
  const {
    wsUrl,
    instanceId,
    throttleMs,
    reconnectMs,
    describeError,
    onBatch,
    onState,
  } = options;
  let disposed = false;
  let generation = 0;
  let socket: WebSocket | null = null;
  let flushTimer: ReturnType<typeof setTimeout> | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  const buffer: WsMessage[] = [];

  function detach(target: WebSocket): void {
    target.onopen = null;
    target.onmessage = null;
    target.onerror = null;
    target.onclose = null;
  }

  function retire(target: WebSocket, close: boolean): void {
    if (socket === target) socket = null;
    detach(target);
    if (close) {
      try {
        target.close();
      } catch {
        // The socket is already unusable; reconnect still proceeds.
      }
    }
  }

  function isCurrent(target: WebSocket, targetGeneration: number): boolean {
    return !disposed && socket === target && generation === targetGeneration;
  }

  function flush(): void {
    if (disposed) {
      buffer.length = 0;
      return;
    }
    if (buffer.length === 0) return;
    onBatch(instanceId, buffer.splice(0));
  }

  function scheduleReconnect(): void {
    if (disposed || reconnectTimer !== null) return;
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      connect();
    }, reconnectMs);
  }

  function connect(): void {
    if (disposed || socket !== null) return;
    const targetGeneration = ++generation;
    let next: WebSocket;
    try {
      next = new WebSocket(wsUrl);
    } catch (error) {
      onState(instanceId, {
        wsConnected: false,
        wsError: describeError(wsUrl, error),
      });
      scheduleReconnect();
      return;
    }
    socket = next;

    next.onopen = () => {
      if (!isCurrent(next, targetGeneration)) return;
      onState(instanceId, { wsConnected: true, wsError: null });
    };

    next.onmessage = (event) => {
      if (!isCurrent(next, targetGeneration)) return;
      try {
        buffer.push(JSON.parse(event.data as string) as WsMessage);
      } catch {
        return;
      }
      if (flushTimer === null) {
        flushTimer = setTimeout(() => {
          flushTimer = null;
          flush();
        }, throttleMs);
      }
    };

    next.onerror = () => {
      if (!isCurrent(next, targetGeneration)) return;
      onState(instanceId, {
        wsConnected: false,
        wsError: describeError(wsUrl),
      });
      retire(next, true);
      scheduleReconnect();
    };

    next.onclose = () => {
      if (!isCurrent(next, targetGeneration)) return;
      retire(next, false);
      onState(instanceId, {
        wsConnected: false,
        wsError: describeError(wsUrl),
      });
      scheduleReconnect();
    };
  }

  connect();

  return () => {
    if (disposed) return;
    disposed = true;
    generation += 1;
    if (flushTimer !== null) {
      clearTimeout(flushTimer);
      flushTimer = null;
    }
    if (reconnectTimer !== null) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }
    buffer.length = 0;
    const current = socket;
    socket = null;
    if (current) retire(current, true);
  };
}
