import { T, FONT } from "../tokens";
import type { StatusResponse } from "../types";

interface Props {
  status: StatusResponse | null;
  wsConnected: boolean;
  currentPositionLabel: string;
  expressionPressure: number;
  onPressureClick: () => void;
}

export function MetricsBar({
  status,
  wsConnected,
  currentPositionLabel,
  expressionPressure,
  onPressureClick,
}: Props) {
  const s = status ?? {
    beta_1: 0, vertices: 0, edges: 0, settled: 0,
    steps: 0, encounter_density: 0, status: "traversing" as const,
    crystallization_count: 0,
  };

  const dotColor =
    !wsConnected ? T.textMuted :
    s.status === "traversing" ? T.alive :
    s.status === "feeding" ? T.feeding : T.crystallized;

  const statusLabel =
    s.status === "traversing" ? "穿越中" :
    s.status === "feeding" ? "进食中" : "结晶";

  // Expression pressure indicator color and animation
  const pressureCount = expressionPressure;
  const pressureColor =
    pressureCount === 0 ? T.textMuted :
    pressureCount <= 3 ? "#4ade80" :   // low: light green
    "#22c55e";                          // high: bright green

  const pressureLabel =
    pressureCount === 0 ? "无积压" :
    pressureCount <= 3 ? `${pressureCount}件` :
    `${pressureCount}件`;

  // Pulse animation for high pressure
  const shouldPulse = pressureCount >= 4;

  return (
    <div style={{
      display: "flex", alignItems: "center", gap: 0,
      background: T.bgPanel, borderBottom: `1px solid ${T.border}`,
      height: 44, paddingLeft: 16, paddingRight: 16,
      fontFamily: FONT.mono, fontSize: 12, flexShrink: 0,
    }}>
      {/* WS / status dot */}
      <div style={{
        width: 7, height: 7, borderRadius: "50%",
        background: dotColor,
        boxShadow: wsConnected ? `0 0 8px ${dotColor}` : "none",
        marginRight: 10,
        transition: "background 0.3s",
      }} />

      <span style={{ color: T.textDim, marginRight: 4 }}>beta_1</span>
      <span style={{ color: T.text, fontWeight: 600, marginRight: 20 }}>
        {s.beta_1.toLocaleString()}
      </span>

      <span style={{ color: T.textDim, marginRight: 4 }}>settled</span>
      <span style={{ color: T.settled, fontWeight: 600, marginRight: 20 }}>
        {s.settled}
      </span>

      <span style={{ color: T.textDim, marginRight: 4 }}>V</span>
      <span style={{ color: T.text, marginRight: 12 }}>{s.vertices.toLocaleString()}</span>

      <span style={{ color: T.textDim, marginRight: 4 }}>E</span>
      <span style={{ color: T.text, marginRight: 20 }}>{s.edges.toLocaleString()}</span>

      <span style={{ color: T.textDim, marginRight: 4 }}>步</span>
      <span style={{ color: T.text, marginRight: 20 }}>{s.steps.toLocaleString()}</span>

      <span style={{ color: T.textDim, marginRight: 4 }}>encounter</span>
      <span style={{
        color: s.encounter_density > 0.5 ? T.fMid : T.textDim,
        marginRight: 20,
      }}>
        {(s.encounter_density * 100).toFixed(0)}%
      </span>

      {currentPositionLabel && (
        <>
          <span style={{ color: T.textMuted, marginRight: 4 }}>@</span>
          <span style={{
            color: T.traversalPulse, opacity: 0.7,
            maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
            marginRight: 20,
          }}>
            {currentPositionLabel}
          </span>
        </>
      )}

      <div style={{ flex: 1 }} />

      {/* Expression pressure indicator */}
      <button
        onClick={onPressureClick}
        title={`表达压力: ${pressureCount}个未报告事件。点击触发分享。`}
        style={{
          display: "flex", alignItems: "center", gap: 5,
          background: "transparent",
          border: `1px solid ${pressureCount > 0 ? pressureColor : T.border}`,
          borderRadius: 4,
          padding: "2px 8px",
          cursor: "pointer",
          marginRight: 16,
          transition: "border-color 0.3s, box-shadow 0.3s",
          boxShadow: shouldPulse ? `0 0 8px ${pressureColor}66` : "none",
          animation: shouldPulse ? "pressure-pulse 1.5s ease-in-out infinite" : "none",
        }}
      >
        {/* Pressure dot */}
        <div style={{
          width: 6, height: 6, borderRadius: "50%",
          background: pressureColor,
          boxShadow: pressureCount > 0 ? `0 0 6px ${pressureColor}` : "none",
          transition: "background 0.3s",
        }} />
        <span style={{
          color: pressureCount > 0 ? pressureColor : T.textMuted,
          fontSize: 10, letterSpacing: "0.05em",
          transition: "color 0.3s",
        }}>
          {pressureLabel}
        </span>
      </button>

      <span style={{
        color: dotColor,
        fontSize: 10, letterSpacing: "0.1em", textTransform: "uppercase",
      }}>
        {wsConnected ? statusLabel : "连接中..."}
      </span>

      {/* Keyframe animation for high-pressure pulse */}
      <style>{`
        @keyframes pressure-pulse {
          0%, 100% { box-shadow: 0 0 4px ${pressureColor}44; }
          50% { box-shadow: 0 0 12px ${pressureColor}88; }
        }
      `}</style>
    </div>
  );
}
