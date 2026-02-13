import { useEffect, useRef, useState, useCallback } from "react";
import type {
  ChanEvent,
  WsBarMessage,
  WsServerMessage,
  WsCommand,
  WsEventMessage,
} from "../types/events";
import { useReplayStore } from "../store/replayStore";

// ── helpers ──

const WS_URL = "ws://localhost:8766/ws/feed";
const RECONNECT_DELAY = 3000;

/** 将 WsEventMessage 的 payload 展开为 ChanEvent */
function toChanEvent(msg: WsEventMessage): ChanEvent {
  return {
    event_type: msg.event_type,
    bar_idx: msg.bar_idx,
    bar_ts: msg.bar_ts,
    seq: msg.seq,
    event_id: msg.event_id,
    schema_version: msg.schema_version,
    ...msg.payload,
  } as ChanEvent;
}

// ── hook ──

export interface UseEventFeedOptions {
  enabled: boolean;
}

export interface EventFeedState {
  connected: boolean;
  latestBar: WsBarMessage | null;
  sendCommand: (cmd: WsCommand) => void;
}

export function useEventFeed({ enabled }: UseEventFeedOptions): EventFeedState {
  const [connected, setConnected] = useState(false);
  const [latestBar, setLatestBar] = useState<WsBarMessage | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const enabledRef = useRef(enabled);
  enabledRef.current = enabled;

  const pushEvent = useReplayStore((s) => s.pushEvent);
  const updateStatus = useReplayStore((s) => s.updateStatus);

  // sendCommand exposed to consumers
  const sendCommand = useCallback((cmd: WsCommand) => {
    const ws = wsRef.current;
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(cmd));
    } else {
      console.warn("[eventfeed] cannot send — ws not open");
    }
  }, []);

  useEffect(() => {
    if (!enabled) {
      // 关闭已有连接
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
      if (reconnectTimer.current) {
        clearTimeout(reconnectTimer.current);
        reconnectTimer.current = null;
      }
      setConnected(false);
      return;
    }

    function connect() {
      if (!enabledRef.current) return;

      const ws = new WebSocket(WS_URL);
      wsRef.current = ws;

      ws.onopen = () => {
        setConnected(true);
      };

      ws.onclose = () => {
        setConnected(false);
        wsRef.current = null;
        // 自动重连
        if (enabledRef.current) {
          reconnectTimer.current = setTimeout(connect, RECONNECT_DELAY);
        }
      };

      ws.onerror = (e) => {
        console.error("[eventfeed] ws error:", e);
        // onclose will fire after onerror
      };

      ws.onmessage = (ev) => {
        let msg: WsServerMessage;
        try {
          msg = JSON.parse(ev.data) as WsServerMessage;
        } catch {
          console.error("[eventfeed] bad json:", ev.data);
          return;
        }

        switch (msg.type) {
          case "bar":
            setLatestBar(msg);
            break;

          case "event":
            pushEvent(toChanEvent(msg));
            break;

          case "snapshot":
            // snapshot 暂不做复杂处理，仅 log
            console.log("[eventfeed] snapshot:", msg.bar_idx, "events:", msg.event_count);
            break;

          case "replay_status":
            updateStatus({
              mode: msg.mode,
              currentIdx: msg.current_idx,
              totalBars: msg.total_bars,
              speed: msg.speed,
            });
            break;

          case "error":
            console.error("[eventfeed] server error:", msg.code, msg.message);
            break;
        }
      };
    }

    connect();

    return () => {
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }
      if (reconnectTimer.current) {
        clearTimeout(reconnectTimer.current);
        reconnectTimer.current = null;
      }
      setConnected(false);
    };
  }, [enabled, pushEvent, updateStatus]);

  return { connected, latestBar, sendCommand };
}
