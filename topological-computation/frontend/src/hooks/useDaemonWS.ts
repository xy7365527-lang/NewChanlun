import { useEffect, useRef } from "react";
import { DAEMON_WS, WS_THROTTLE_MS } from "../tokens";
import type { WsMessage } from "../types";
import { useStore } from "./useStore";

/**
 * Connects to a daemon WebSocket and feeds throttled messages into the store.
 * 100ms throttle: D3 doesn't need every single step.
 *
 * All instances are peers — every WS connection feeds into handleWsBatch(instanceId, msgs).
 *
 * @param wsUrl - WebSocket URL (defaults to DAEMON_WS from tokens)
 * @param instanceId - Instance ID for attribution in the shared store
 */
export function useDaemonWS(
  wsUrl: string = DAEMON_WS,
  instanceId: string = "default",
): void {
  const wsRef = useRef<WebSocket | null>(null);
  const bufferRef = useRef<WsMessage[]>([]);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleBatch = useStore((s) => s.handleWsBatch);
  const setWsConnected = useStore((s) => s.setWsConnected);
  const updateInstanceState = useStore((s) => s.updateInstanceState);

  useEffect(() => {
    let destroyed = false;

    function flush() {
      if (bufferRef.current.length === 0) return;
      const batch = bufferRef.current.splice(0);
      handleBatch(instanceId, batch);
    }

    function connect() {
      if (destroyed) return;

      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        setWsConnected(true);
        updateInstanceState(instanceId, { wsConnected: true });
      };

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data as string) as WsMessage;
          bufferRef.current.push(msg);

          if (timerRef.current === null) {
            timerRef.current = setTimeout(() => {
              timerRef.current = null;
              flush();
            }, WS_THROTTLE_MS);
          }
        } catch {
          // ignore malformed messages
        }
      };

      ws.onclose = () => {
        setWsConnected(false);
        updateInstanceState(instanceId, { wsConnected: false });
        wsRef.current = null;
        if (!destroyed) {
          reconnectTimerRef.current = setTimeout(connect, 3000);
        }
      };

      ws.onerror = () => {
        ws.close();
      };
    }

    connect();

    return () => {
      destroyed = true;
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
        flush();
      }
      if (reconnectTimerRef.current !== null) {
        clearTimeout(reconnectTimerRef.current);
      }
      wsRef.current?.close();
    };
  }, [wsUrl, instanceId, handleBatch, setWsConnected, updateInstanceState]);
}
