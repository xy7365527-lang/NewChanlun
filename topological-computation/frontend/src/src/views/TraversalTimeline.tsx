/**
 * TraversalTimeline.tsx
 *
 * 穿越轨迹时间序列可视化。
 * 横轴 = 穿越步数
 * 叠加曲线：β₁（主轴）+ f 值（次轴）
 * 散点标记：encounter 类型（fold/negate/sublate/blocked）
 * 事件竖线：结晶（蓝）+ gap（琥珀）+ feed（绿）
 *
 * 数据来自 WebSocket 累积（useStore.beta1History + narrative events）。
 * 使用 Recharts ComposedChart。
 */

import { useMemo } from "react";
import {
  ComposedChart, Line, Scatter, XAxis, YAxis, Tooltip,
  ResponsiveContainer, ReferenceLine, CartesianGrid,
} from "recharts";
import { T, FONT } from "../tokens";
import type { NarrativeEvent } from "../types";

// ── 类型 ────────────────────────────────────────────────────────

interface Beta1Point {
  step: number;
  beta1: number;
}

interface TraversalPoint {
  step: number;
  beta1: number;
  fValue?: number;
  opType?: string;  // 'fold' | 'negate' | 'sublate' | 'walk' | 'blocked'
}

interface EventMarker {
  step: number;
  type: "crystallized" | "gap" | "feed";
  text: string;
}

interface Props {
  beta1History: Beta1Point[];
  narrative: NarrativeEvent[];
}

// ── 颜色映射 ─────────────────────────────────────────────────────

const OP_COLOR: Record<string, string> = {
  fold: T.fLow,          // 绿
  negate: T.fHigh,       // 红
  sublate: T.accent,     // 紫
  blocked: T.fMid,       // 琥珀（黄叉）
  walk: T.textMuted,     // 低调灰
};

const EVENT_COLOR: Record<string, string> = {
  crystallized: T.settled,  // 蓝
  gap: T.fMid,              // 琥珀
  feed: T.fLow,             // 绿
};

// ── 自定义 Dot ────────────────────────────────────────────────────

interface DotProps {
  cx?: number;
  cy?: number;
  payload?: TraversalPoint;
  [key: string]: unknown;
}

function EncounterDot({ cx = 0, cy = 0, payload }: DotProps) {
  if (!payload?.opType || payload.opType === "walk") {
    // 不可见占位（recharts 需要返回 SVG 元素而非 null）
    return <g />;
  }
  const color = OP_COLOR[payload.opType] ?? T.textMuted;
  const isBlocked = payload.opType === "blocked";

  if (isBlocked) {
    // 黄叉 ✕
    return (
      <g transform={`translate(${cx},${cy})`}>
        <line x1={-4} y1={-4} x2={4} y2={4} stroke={color} strokeWidth={2} />
        <line x1={4} y1={-4} x2={-4} y2={4} stroke={color} strokeWidth={2} />
      </g>
    );
  }

  return (
    <circle
      cx={cx} cy={cy} r={4}
      fill={color} fillOpacity={0.85}
      stroke={color} strokeWidth={0.5}
    />
  );
}

// ── 自定义 Tooltip ────────────────────────────────────────────────

interface TooltipProps {
  active?: boolean;
  payload?: Array<{ value: number; dataKey: string; payload: TraversalPoint }>;
  label?: number;
}

function CustomTooltip({ active, payload, label }: TooltipProps) {
  if (!active || !payload || payload.length === 0) return null;
  const pt = payload[0]?.payload;

  return (
    <div style={{
      background: T.bgCard + "ee",
      border: `1px solid ${T.border}`,
      borderRadius: 4, padding: "6px 10px",
      fontFamily: FONT.mono, fontSize: 10, color: T.text,
    }}>
      <div style={{ color: T.textMuted, marginBottom: 4 }}>步 {label}</div>
      {payload.map((p, i) => (
        <div key={i} style={{ color: T.textDim }}>
          {p.dataKey === "beta1"
            ? <span>β₁ = <span style={{ color: T.accent }}>{p.value}</span></span>
            : <span>f = <span style={{ color: T.fMid }}>{p.value?.toFixed(2)}</span></span>
          }
        </div>
      ))}
      {pt?.opType && pt.opType !== "walk" && (
        <div style={{
          marginTop: 4, color: OP_COLOR[pt.opType] ?? T.textMuted,
          fontWeight: 600,
        }}>
          op: {pt.opType}
        </div>
      )}
    </div>
  );
}

// ── 从 narrative 提取事件标记 ────────────────────────────────────

function extractEventMarkers(narrative: NarrativeEvent[]): EventMarker[] {
  const markers: EventMarker[] = [];
  for (const ev of narrative) {
    const text = ev.text ?? "";

    // 结晶事件
    if (text.includes("crystall") || (ev.type === "crystallize")) {
      const stepMatch = text.match(/步\s*(\d+)/);
      const step = stepMatch ? parseInt(stepMatch[1]) : -1;
      if (step >= 0) {
        markers.push({ step, type: "crystallized", text });
      }
    }

    // gap 事件
    if (ev.type === "gap" || text.startsWith("gap detected")) {
      const stepMatch = text.match(/步\s*(\d+)/);
      const step = stepMatch ? parseInt(stepMatch[1]) : -1;
      if (step >= 0) {
        markers.push({ step, type: "gap", text });
      }
    }

    // feed 事件
    if (ev.type === "feed" || text.startsWith("feed:")) {
      const stepMatch = text.match(/步\s*(\d+)/);
      const step = stepMatch ? parseInt(stepMatch[1]) : -1;
      if (step >= 0) {
        markers.push({ step, type: "feed", text });
      }
    }
  }
  return markers;
}

// ── 从 narrative 提取 encounter 操作类型 ─────────────────────────

function extractOpMap(narrative: NarrativeEvent[]): Map<number, string> {
  const map = new Map<number, string>();
  for (const ev of narrative) {
    const text = ev.text ?? "";
    const stepMatch = text.match(/\[步\s*(\d+)\]\s*(\w+)/);
    if (stepMatch) {
      const step = parseInt(stepMatch[1]);
      const op = stepMatch[2].toLowerCase();
      if (["fold", "negate", "sublate", "walk", "blocked"].includes(op)) {
        map.set(step, op);
      }
    }
  }
  return map;
}

// ── 渲染 EncounterDot（参数类型为 unknown，兼容 Recharts ScatterCustomizedShape） ─

function renderEncounterDot(props: unknown) {
  return <EncounterDot {...(props as DotProps)} />;
}

// ── 组件 ─────────────────────────────────────────────────────────

export function TraversalTimeline({ beta1History, narrative }: Props) {
  // ── 合并数据 ─────────────────────────────────────────────────
  const opMap = useMemo(() => extractOpMap(narrative), [narrative]);
  const eventMarkers = useMemo(() => extractEventMarkers(narrative), [narrative]);

  const chartData: TraversalPoint[] = useMemo(() => {
    return beta1History.map((pt) => ({
      step: pt.step,
      beta1: pt.beta1,
      opType: opMap.get(pt.step),
    }));
  }, [beta1History, opMap]);

  // 只有在有数据的情况下渲染 beta1 范围
  const beta1Values = beta1History.map((p) => p.beta1);
  const beta1Min = beta1Values.length > 0 ? Math.max(0, Math.min(...beta1Values) - 5) : 0;
  const beta1Max = beta1Values.length > 0 ? Math.max(...beta1Values) + 5 : 10;

  // encounter 散点（仅非 walk）
  const encounterPoints = useMemo(
    () => chartData.filter((p) => p.opType && p.opType !== "walk"),
    [chartData]
  );

  // ── 事件竖线（每类只取前 10 个，避免遮蔽） ──────────────────
  const crystalMarkers = eventMarkers.filter((m) => m.type === "crystallized").slice(0, 10);
  const gapMarkers = eventMarkers.filter((m) => m.type === "gap").slice(0, 10);
  const feedMarkers = eventMarkers.filter((m) => m.type === "feed").slice(0, 10);

  if (chartData.length === 0) {
    return (
      <div style={{
        width: "100%", height: "100%",
        display: "flex", flexDirection: "column",
      }}>
        <div style={{
          padding: "6px 12px", flexShrink: 0,
          fontFamily: FONT.mono, fontSize: 9, color: T.textMuted,
          letterSpacing: "0.1em",
        }}>
          TRAVERSAL TIMELINE
        </div>
        <div style={{
          flex: 1, display: "flex", alignItems: "center", justifyContent: "center",
          fontFamily: FONT.mono, fontSize: 11, color: T.textMuted,
        }}>
          等待穿越数据...
        </div>
      </div>
    );
  }

  return (
    <div style={{
      width: "100%", height: "100%",
      display: "flex", flexDirection: "column",
    }}>
      {/* 标题栏 */}
      <div style={{
        padding: "6px 12px", flexShrink: 0,
        fontFamily: FONT.mono, fontSize: 9, color: T.textMuted,
        letterSpacing: "0.1em",
        display: "flex", justifyContent: "space-between", alignItems: "center",
      }}>
        <span>TRAVERSAL TIMELINE</span>
        <div style={{ display: "flex", gap: 12, fontSize: 9 }}>
          {Object.entries(OP_COLOR).filter(([k]) => k !== "walk").map(([op, color]) => (
            <span key={op} style={{ color, fontFamily: FONT.mono }}>
              {op}
            </span>
          ))}
          <span style={{ color: EVENT_COLOR.crystallized }}>| 结晶</span>
          <span style={{ color: EVENT_COLOR.gap }}>gap</span>
          <span style={{ color: EVENT_COLOR.feed }}>feed</span>
        </div>
      </div>

      {/* 图表 */}
      <div style={{ flex: 1, overflow: "hidden" }}>
        <ResponsiveContainer width="100%" height="100%">
          <ComposedChart
            data={chartData}
            margin={{ top: 8, right: 24, bottom: 20, left: 8 }}
          >
            <CartesianGrid
              stroke={T.border}
              strokeOpacity={0.4}
              vertical={false}
            />
            <XAxis
              dataKey="step"
              type="number"
              domain={["dataMin", "dataMax"]}
              tick={{ fill: T.textMuted, fontSize: 8, fontFamily: FONT.mono }}
              tickLine={{ stroke: T.border }}
              axisLine={{ stroke: T.border }}
              label={{
                value: "步数",
                position: "insideBottomRight",
                offset: -4,
                fill: T.textMuted,
                fontSize: 8,
                fontFamily: FONT.mono,
              }}
            />
            <YAxis
              dataKey="beta1"
              domain={[beta1Min, beta1Max]}
              tick={{ fill: T.textMuted, fontSize: 8, fontFamily: FONT.mono }}
              tickLine={{ stroke: T.border }}
              axisLine={{ stroke: T.border }}
              width={36}
              label={{
                value: "β₁",
                angle: -90,
                position: "insideLeft",
                fill: T.accent,
                fontSize: 9,
                fontFamily: FONT.mono,
              }}
            />
            <Tooltip content={<CustomTooltip />} />

            {/* β₁ 主曲线 */}
            <Line
              type="monotone"
              dataKey="beta1"
              stroke={T.accent}
              strokeWidth={1.5}
              dot={false}
              isAnimationActive={false}
            />

            {/* encounter 散点叠加：独立 data 覆盖父级 chartData */}
            <Scatter
              data={encounterPoints}
              dataKey="beta1"
              shape={renderEncounterDot}
              isAnimationActive={false}
            />

            {/* 结晶事件竖线（蓝） */}
            {crystalMarkers.map((m, i) => (
              <ReferenceLine
                key={`crystal-${i}`}
                x={m.step}
                stroke={T.settled}
                strokeWidth={1}
                strokeOpacity={0.7}
                strokeDasharray="2,4"
              />
            ))}

            {/* gap 事件竖线（琥珀） */}
            {gapMarkers.map((m, i) => (
              <ReferenceLine
                key={`gap-${i}`}
                x={m.step}
                stroke={T.fMid}
                strokeWidth={1}
                strokeOpacity={0.5}
                strokeDasharray="1,6"
              />
            ))}

            {/* feed 事件竖线（绿） */}
            {feedMarkers.map((m, i) => (
              <ReferenceLine
                key={`feed-${i}`}
                x={m.step}
                stroke={T.fLow}
                strokeWidth={1}
                strokeOpacity={0.5}
                strokeDasharray="3,3"
              />
            ))}
          </ComposedChart>
        </ResponsiveContainer>
      </div>

      {/* 底部计数 */}
      <div style={{
        padding: "3px 12px", flexShrink: 0,
        fontFamily: FONT.mono, fontSize: 8, color: T.textMuted,
        display: "flex", gap: 16,
      }}>
        <span>{beta1History.length} 个步数点</span>
        <span>{encounterPoints.length} 个 encounter</span>
        <span>{crystalMarkers.length} 次结晶</span>
        <span>{gapMarkers.length} 个 gap</span>
      </div>
    </div>
  );
}
