/**
 * useLiveWebSocket — 实时 WebSocket 连接 /ws/live/{symbol}
 *
 * 功能：
 * - 连接后收到当前 snapshot
 * - 每根新 bar 实时推送，增量更新 K 线图
 * - 域事件推送，驱动 marker 标注
 * - symbol 切换时自动重连
 * - 非回放模式下启用
 */
import { useEffect, useRef, useState, useCallback } from "react";
import type { ISeriesApi, Time, CandlestickData, HistogramData } from "lightweight-charts";
import type {
  WsBarMessage,
  WsServerMessage,
  WsEventMessage,
  ChanEvent,
} from "../types/events";
import { useReplayStore } from "../store/replayStore";

const RECONNECT_DELAY = 3000;

function barToCandle(msg: WsBarMessage): CandlestickData<Time> {
  return {
    time: msg.ts as unknown as Time,
    open: msg.o,
    high: msg.h,
    low: msg.l,
    close: msg.c,
  };
}

function barToVolume(msg: WsBarMessage): HistogramData<Time> {
  return {
    time: msg.ts as unknown as Time,
    value: msg.v ?? 0,
    color: msg.c >= msg.o ? "rgba(38,166,154,0.3)" : "rgba(239,83,80,0.3)",
  };
}

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

function buildWsUrl(symbol: string): string {
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}/ws/live/${symbol}`;
}

export interface UseLiveWebSocketOptions {
  symbol: string;
  enabled: boolean;
  candleSeries: ISeriesApi<"Candlestick"> | null;
  volumeSeries: ISeriesApi<"Histogram"> | null;
}

export interface UseLiveWebSocketReturn {
  connected: boolean;
}

export function useLiveWebSocket({
  symbol,
  enabled,
  candleSeries,
  volumeSeries,
}: UseLiveWebSocketOptions): UseLiveWebSocketReturn {
  const [connected, setConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const enabledRef = useRef(enabled);
  enabledRef.current = enabled;

  const pushEvent = useReplayStore((s) => s.pushEvent);

  const cleanup = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    if (reconnectTimer.current) {
      clearTimeout(reconnectTimer.current);
      reconnectTimer.current = null;
    }
    setConnected(false);
  }, []);

  useEffect(() => {
    if (!enabled || !symbol || !candleSeries || !volumeSeries) {
      cleanup();
      return;
    }

    function connect() {
      if (!enabledRef.current) return;

      const url = buildWsUrl(symbol);
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        setConnected(true);
      };

      ws.onclose = () => {
        setConnected(false);
        wsRef.current = null;
        if (enabledRef.current) {
          reconnectTimer.current = setTimeout(connect, RECONNECT_DELAY);
        }
      };

      ws.onerror = (e) => {
        console.error("[live-ws] error:", e);
      };

      ws.onmessage = (ev) => {
        let msg: WsServerMessage;
        try {
          msg = JSON.parse(ev.data) as WsServerMessage;
        } catch {
          console.error("[live-ws] bad json:", ev.data);
          return;
        }

        switch (msg.type) {
          case "bar":
            candleSeries!.update(barToCandle(msg));
            volumeSeries!.update(barToVolume(msg));
            break;

          case "event":
            pushEvent(toChanEvent(msg));
            break;

          case "snapshot":
            // snapshot 用于初始状态同步，overlay 由 useOverlay REST 轮询处理
            console.log("[live-ws] snapshot received, bar_idx:", msg.bar_idx);
            break;

          case "error":
            console.error("[live-ws] server error:", msg.code, msg.message);
            break;
        }
      };
    }

    connect();

    return cleanup;
  }, [enabled, symbol, candleSeries, volumeSeries, pushEvent, cleanup]);

  return { connected };
}
