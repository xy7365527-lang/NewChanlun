import { useRef, useEffect } from "react";
import { Toolbar } from "./components/Toolbar";
import { StatusBadge } from "./components/StatusBadge";
import { MacdPane } from "./components/MacdPane";
import { ReplayBar } from "./components/ReplayBar";
import { useChart } from "./hooks/useChart";
import { useDatafeed } from "./hooks/useDatafeed";
import { useOverlay } from "./hooks/useOverlay";
import { useEventFeed } from "./hooks/useEventFeed";
import { useAppStore } from "./store/appStore";
import { useReplayStore } from "./store/replayStore";
import { EventMarkerManager } from "./primitives/EventMarkerPrimitive";

// 单例 marker manager
const markerManager = new EventMarkerManager();

function ChartArea() {
  const containerRef = useRef<HTMLDivElement>(null);
  const refs = useChart(containerRef);
  const { symbol, interval, tf } = useAppStore();
  const isReplaying = useReplayStore((s) => s.isReplaying);
  const events = useReplayStore((s) => s.events);
  const prevEventsLen = useRef(0);

  useDatafeed({
    chart: refs?.chart ?? null,
    candleSeries: refs?.candleSeries ?? null,
    volumeSeries: refs?.volumeSeries ?? null,
    symbol,
    interval,
    tf,
  });

  const { lstar, overlay } = useOverlay({
    candleSeries: refs?.candleSeries ?? null,
    symbol,
    interval,
    tf,
  });

  // 连接 WebSocket（回放模式启用）
  useEventFeed({ enabled: isReplaying });

  // 挂载/卸载 marker manager
  useEffect(() => {
    if (refs?.candleSeries) {
      markerManager.attach(refs.candleSeries);
    }
    return () => {
      markerManager.detach();
    };
  }, [refs?.candleSeries]);

  // 回放模式切换时清空 markers
  useEffect(() => {
    if (!isReplaying) {
      markerManager.clear();
      prevEventsLen.current = 0;
    }
  }, [isReplaying]);

  // 新事件 → 添加 marker
  useEffect(() => {
    if (events.length > prevEventsLen.current) {
      for (let i = prevEventsLen.current; i < events.length; i++) {
        markerManager.addEvent(events[i]);
      }
    }
    prevEventsLen.current = events.length;
  }, [events]);

  return (
    <>
      <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
        <div
          ref={containerRef}
          style={{ width: "100%", height: "100%", background: "#131722" }}
        />
        <StatusBadge lstar={lstar} totalLevels={overlay?.levels?.length ?? 0} />
      </div>
      <MacdPane overlay={overlay} mainChart={refs?.chart ?? null} />
    </>
  );
}

export default function App() {
  return (
    <div
      style={{
        width: "100vw",
        height: "100vh",
        display: "flex",
        flexDirection: "column",
        background: "#131722",
        color: "#d1d4dc",
        fontFamily: "'Segoe UI', system-ui, sans-serif",
        overflow: "hidden",
      }}
    >
      <Toolbar />
      <ChartArea />
      <ReplayBar />
    </div>
  );
}
