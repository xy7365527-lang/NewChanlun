import { useEffect, useRef } from "react";
import { DAEMON_WS, WS_THROTTLE_MS } from "../tokens";
import type { WsMessage } from "../types";
import { useStore } from "./useStore";

/**
 * Connects to a daemon WebSocket and feeds throttled messages into the store.
 * 100ms throttle: D3 doesn't need every single step.
 *
 * @param wsUrl - WebSocket URL (defaults to DAEMON_WS from tokens)
 * @param instanceId - Instance ID for non-primary instances (null = primary, uses handleWsBatch)
 */
export function useDaemonWS(
  wsUrl: string = DAEMON_WS,
  instanceId: string | null = null,
): void {
  const wsRef = useRef<WebSocket | null>(null);
  const bufferRef = useRef<WsMessage[]>([]);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleBatch = useStore((s) => s.handleWsBatch);
  const handleInstanceBatch = useStore((s) => s.handleInstanceWsBatch);
  const setWsConnected = useStore((s) => s.setWsConnected);
  const updateInstanceState = useStore((s) => s.updateInstanceState);

  useEffect(() => {
    let destroyed = false;

    function flush() {
      if (bufferRef.current.length === 0) return;
      const batch = bufferRef.current.splice(0);
      if (instanceId === null) {
        handleBatch(batch);
      } else {
        handleInstanceBatch(instanceId, batch);
      }
    }

    function connect() {
      if (destroyed) return;

      const ws = new WebSocket(wsUrl);
      wsRef.current = ws;

      ws.onopen = () => {
        if (instanceId === null) {
          setWsConnected(true);
        } else {
          updateInstanceState(instanceId, { wsConnected: true });
        }
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
        if (instanceId === null) {
          setWsConnected(false);
        } else {
          updateInstanceState(instanceId, { wsConnected: false });
        }
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
  }, [wsUrl, instanceId, handleBatch, handleInstanceBatch, setWsConnected, updateInstanceState]);
}
