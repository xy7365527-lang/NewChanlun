import { useEffect, useRef } from "react";
import { DAEMON_WS, WS_THROTTLE_MS } from "../tokens";
import type { WsMessage } from "../types";
import { useStore } from "./useStore";

/**
 * Connects to daemon WebSocket and feeds throttled messages into the store.
 * 100ms throttle: D3 doesn't need every single step.
 */
export function useDaemonWS(): void {
  const wsRef = useRef<WebSocket | null>(null);
  const bufferRef = useRef<WsMessage[]>([]);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleBatch = useStore((s) => s.handleWsBatch);
  const setWsConnected = useStore((s) => s.setWsConnected);

  useEffect(() => {
    let destroyed = false;

    function flush() {
      if (bufferRef.current.length === 0) return;
      const batch = bufferRef.current.splice(0);
      handleBatch(batch);
    }

    function connect() {
      if (destroyed) return;

      const ws = new WebSocket(DAEMON_WS);
      wsRef.current = ws;

      ws.onopen = () => {
        setWsConnected(true);
      };

      ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data as string) as WsMessage;
          bufferRef.current.push(msg);

          // Throttle: schedule flush if not already scheduled
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
        wsRef.current = null;
        if (!destroyed) {
          // Reconnect after 3 seconds
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
        flush(); // flush any remaining
      }
      if (reconnectTimerRef.current !== null) {
        clearTimeout(reconnectTimerRef.current);
      }
      wsRef.current?.close();
    };
  }, [handleBatch, setWsConnected]);
}
