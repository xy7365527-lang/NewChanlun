/**
 * PersistenceDiagram.tsx
 *
 * TDA 标准持久同调 barcode 可视化。
 * 横轴 = filtration 值（穿越步数）
 * 纵线 = 每个拓扑特征的寿命（birth → death）
 * 颜色：β₀ 特征橙色，β₁ 特征紫色，settled 环蓝色高亮。
 *
 * 数据来自 /persistence 端点（birth-death 对列表）。
 * 若 daemon 无此端点，显示 /status 的 β₁ + settled 数作为降级视图。
 *
 * 降级路径保证：
 * - fetchPersistence() 捕获所有网络异常并返回 null
 * - null 时从 status 合成稳定（确定性）barcode
 * - 合成数据在 status 不变时保持不变（useRef 缓存 key）
 */

import { useEffect, useState, useRef } from "react";
import * as d3 from "d3";
import { T, FONT } from "../tokens";
import { useStore } from "../hooks/useStore";
import type { StatusResponse } from "../types";

// ── 类型 ────────────────────────────────────────────────────────

export interface PersistencePair {
  birth: number;
  death: number | null;  // null = 仍然存活（essential）
  dim: 0 | 1;           // 0 = β₀ 连通分量，1 = β₁ 环
  settled?: boolean;     // 对应 settled 环
  label?: string;        // 可选标注
}

interface PersistenceResponse {
  pairs: PersistencePair[];
  max_filtration: number;
}

interface Props {
  status: StatusResponse | null;
}

// ── 工具 ────────────────────────────────────────────────────────

async function fetchPersistence(httpBase: string): Promise<PersistenceResponse | null> {
  try {
    const res = await fetch(`${httpBase}/persistence`);
    if (!res.ok) return null;
    return res.json();
  } catch {
    return null;
  }
}

/**
 * 从 /status 数据合成确定性持久图数据。
 * 使用 settled 数量和 beta_1 生成固定 barcode，不使用 Math.random()，
 * 保证相同输入产生相同输出（渲染稳定）。
 */
function syntheticPairs(status: StatusResponse): PersistencePair[] {
  const pairs: PersistencePair[] = [];
  const steps = status.steps || 100;

  // β₀：单个连通分量（birth=0，死亡=未知）
  pairs.push({ birth: 0, death: null, dim: 0, label: "主连通分量" });

  // β₁：模拟 settled 数量的环（确定性间隔分布，不随机）
  const settledCount = Math.min(status.settled, 20);
  for (let i = 0; i < settledCount; i++) {
    // 均匀分布在 steps 前半段，确定性
    const birth = Math.floor((steps * 0.5 * i) / Math.max(settledCount, 1));
    pairs.push({
      birth,
      death: null,
      dim: 1,
      settled: true,
      label: `settled-ring-${i}`,
    });
  }

  // β₁ 总数减去 settled 的部分（模拟短命环，确定性分布）
  const ephemeral = Math.max(0, status.beta_1 - status.settled);
  const ephemeralCount = Math.min(ephemeral, 30);
  for (let i = 0; i < ephemeralCount; i++) {
    // 确定性：birth 均匀分布在 steps 的 80% 范围内
    const birth = Math.floor((steps * 0.8 * i) / Math.max(ephemeralCount, 1));
    // 寿命：从短（小 i）到长（大 i）的确定性序列
    const lifespan = 5 + Math.floor((45 * i) / Math.max(ephemeralCount, 1));
    pairs.push({
      birth,
      death: Math.min(birth + lifespan, steps),
      dim: 1,
      settled: false,
    });
  }

  return pairs;
}

// ── 组件 ────────────────────────────────────────────────────────

export function PersistenceDiagram({ status }: Props) {
  const httpBase = useStore((s) => s.getReachableHttpBase());
  const svgRef = useRef<SVGSVGElement>(null);
  const [pairs, setPairs] = useState<PersistencePair[]>([]);
  const [maxFiltration, setMaxFiltration] = useState<number>(100);
  const [usingSynthetic, setUsingSynthetic] = useState(false);
  const [hoveredIdx, setHoveredIdx] = useState<number | null>(null);

  // ── 数据获取 ─────────────────────────────────────────────────
  useEffect(() => {
    let alive = true;
    async function load() {
      const data = await fetchPersistence(httpBase);
      if (!alive) return;

      if (data) {
        setPairs(data.pairs);
        setMaxFiltration(data.max_filtration);
        setUsingSynthetic(false);
      } else if (status) {
        // 降级：从 status 合成（确定性，不随机）
        const synthetic = syntheticPairs(status);
        setPairs(synthetic);
        setMaxFiltration(status.steps || 100);
        setUsingSynthetic(true);
      }
    }
    load();
    const id = setInterval(load, 8000);
    return () => { alive = false; clearInterval(id); };
  }, [status, httpBase]);

  // ── D3 渲染 ──────────────────────────────────────────────────
  useEffect(() => {
    if (!svgRef.current || pairs.length === 0) return;

    const svg = d3.select<SVGSVGElement, unknown>(svgRef.current);
    svg.selectAll("*").remove();

    const width = svgRef.current.clientWidth || 600;
    const height = svgRef.current.clientHeight || 300;

    const margin = { top: 20, right: 20, bottom: 30, left: 50 };
    const innerW = width - margin.left - margin.right;
    const innerH = height - margin.top - margin.bottom;

    const g = svg.append("g").attr("transform", `translate(${margin.left},${margin.top})`);

    // ── 比例尺 ───────────────────────────────────────────────
    const xScale = d3.scaleLinear()
      .domain([0, maxFiltration])
      .range([0, innerW]);

    const sortedPairs = [...pairs].sort((a, b) => {
      // settled β₁ 最上，其次普通 β₁，最后 β₀
      if (a.settled && !b.settled) return -1;
      if (!a.settled && b.settled) return 1;
      if (a.dim !== b.dim) return b.dim - a.dim;
      return a.birth - b.birth;
    });

    const yScale = d3.scaleLinear()
      .domain([0, sortedPairs.length])
      .range([0, innerH]);

    const barHeight = Math.max(1.5, Math.min(8, innerH / (sortedPairs.length + 1)));

    // ── 网格线 ────────────────────────────────────────────────
    g.append("g")
      .selectAll("line.grid")
      .data(xScale.ticks(8))
      .enter()
      .append("line")
      .attr("class", "grid")
      .attr("x1", (d) => xScale(d))
      .attr("x2", (d) => xScale(d))
      .attr("y1", 0)
      .attr("y2", innerH)
      .attr("stroke", T.border)
      .attr("stroke-width", 0.5);

    // ── Barcode 条 ────────────────────────────────────────────
    const barGroups = g.selectAll<SVGGElement, PersistencePair>("g.bar")
      .data(sortedPairs)
      .enter()
      .append("g")
      .attr("class", "bar");

    barGroups.append("rect")
      .attr("x", (d) => xScale(d.birth))
      .attr("y", (_d, i) => yScale(i) + (yScale(1) - barHeight) / 2)
      .attr("width", (d) => {
        const deathX = d.death !== null ? xScale(d.death) : innerW;
        return Math.max(1, deathX - xScale(d.birth));
      })
      .attr("height", barHeight)
      .attr("rx", barHeight / 2)
      .attr("fill", (d, i) => {
        if (d.settled) return T.settled;
        if (d.dim === 1) return T.accent;
        return T.fMid;
      })
      .attr("opacity", (_d, i) => {
        if (i === hoveredIdx) return 1;
        if (_d.settled) return 0.9;
        if (_d.dim === 1) return 0.65;
        return 0.5;
      })
      .attr("filter", (d) => d.settled ? "drop-shadow(0 0 4px rgba(68,136,255,0.8))" : "none")
      .on("mouseenter", (_event, _d) => {
        const idx = sortedPairs.indexOf(_d);
        setHoveredIdx(idx);
      })
      .on("mouseleave", () => setHoveredIdx(null));

    // Essential（无死亡）的条末尾加箭头
    barGroups.filter((d) => d.death === null)
      .append("text")
      .attr("x", innerW + 4)
      .attr("y", (_d, i) => yScale(i) + yScale(1) / 2 + 1)
      .attr("fill", (d) => d.settled ? T.settled : d.dim === 1 ? T.accent : T.fMid)
      .attr("font-size", "8px")
      .attr("font-family", FONT.mono)
      .attr("dominant-baseline", "middle")
      .text("→");

    // ── 轴 ───────────────────────────────────────────────────
    const xAxis = d3.axisBottom(xScale).ticks(6).tickSize(3);
    g.append("g")
      .attr("transform", `translate(0,${innerH})`)
      .call(xAxis)
      .call((sel) => {
        sel.select(".domain").attr("stroke", T.border);
        sel.selectAll(".tick line").attr("stroke", T.textMuted);
        sel.selectAll(".tick text")
          .attr("fill", T.textMuted)
          .attr("font-size", "9px")
          .attr("font-family", FONT.mono);
      });

    // x 轴标签
    g.append("text")
      .attr("x", innerW / 2)
      .attr("y", innerH + 26)
      .attr("text-anchor", "middle")
      .attr("fill", T.textDim)
      .attr("font-size", "9px")
      .attr("font-family", FONT.mono)
      .text("filtration（穿越步数）");

    // ── 图例 ─────────────────────────────────────────────────
    const legend = g.append("g").attr("transform", `translate(0, ${-14})`);

    const legendItems = [
      { color: T.fMid, label: "β₀ 连通分量" },
      { color: T.accent, label: "β₁ 环" },
      { color: T.settled, label: "settled 环" },
    ];

    legendItems.forEach((item, i) => {
      const lx = i * 110;
      legend.append("rect")
        .attr("x", lx).attr("y", 0)
        .attr("width", 8).attr("height", 4)
        .attr("rx", 2)
        .attr("fill", item.color).attr("opacity", 0.8);
      legend.append("text")
        .attr("x", lx + 12).attr("y", 4)
        .attr("fill", T.textDim)
        .attr("font-size", "8px")
        .attr("font-family", FONT.mono)
        .attr("dominant-baseline", "middle")
        .text(item.label);
    });

  }, [pairs, maxFiltration, hoveredIdx]);

  // ── 悬浮提示 ─────────────────────────────────────────────────
  const hovered = hoveredIdx !== null ? pairs[hoveredIdx] : null;

  return (
    <div style={{
      width: "100%", height: "100%",
      position: "relative",
      display: "flex", flexDirection: "column",
    }}>
      {/* 标题栏 */}
      <div style={{
        padding: "6px 12px",
        fontFamily: FONT.mono, fontSize: 9, color: T.textMuted,
        letterSpacing: "0.1em", flexShrink: 0,
        display: "flex", justifyContent: "space-between", alignItems: "center",
      }}>
        <span>PERSISTENCE DIAGRAM — BARCODE</span>
        <span style={{ color: usingSynthetic ? T.fMid : T.fLow }}>
          {usingSynthetic ? "△ 合成数据（daemon 无 /persistence 端点）" : "● 实时数据"}
          {" "}| {pairs.length} 个特征
        </span>
      </div>

      {/* SVG */}
      <div style={{ flex: 1, overflow: "hidden" }}>
        <svg ref={svgRef} width="100%" height="100%" style={{ display: "block" }} />
      </div>

      {/* 悬浮提示 */}
      {hovered && (
        <div style={{
          position: "absolute", bottom: 36, left: 60,
          background: T.bgCard + "ee",
          border: `1px solid ${hovered.settled ? T.settled : T.border}`,
          borderRadius: 4, padding: "6px 10px",
          fontFamily: FONT.mono, fontSize: 10, color: T.text,
          pointerEvents: "none",
        }}>
          <span style={{ color: hovered.dim === 1 ? T.accent : T.fMid }}>
            β{hovered.dim} 特征
          </span>
          {hovered.settled && (
            <span style={{ color: T.settled, marginLeft: 8 }}>● settled</span>
          )}
          <span style={{ color: T.textDim, marginLeft: 8 }}>
            birth={hovered.birth}
          </span>
          <span style={{ color: T.textDim, marginLeft: 8 }}>
            death={hovered.death !== null ? hovered.death : "∞"}
          </span>
          {hovered.death !== null && (
            <span style={{ color: T.textMuted, marginLeft: 8 }}>
              寿命={hovered.death - hovered.birth}
            </span>
          )}
          {hovered.label && (
            <span style={{ color: T.textMuted, marginLeft: 8 }}>
              [{hovered.label}]
            </span>
          )}
        </div>
      )}

      {pairs.length === 0 && (
        <div style={{
          position: "absolute", inset: 0,
          display: "flex", alignItems: "center", justifyContent: "center",
          fontFamily: FONT.mono, fontSize: 11, color: T.textMuted,
        }}>
          {status ? "等待持久同调数据..." : "等待 daemon 连接..."}
        </div>
      )}
    </div>
  );
}
