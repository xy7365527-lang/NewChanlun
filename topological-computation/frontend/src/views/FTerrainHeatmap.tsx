/**
 * FTerrainHeatmap.tsx
 *
 * f 值地形热力图。
 * 不可视化整个图——只可视化焦点概念局部邻域的 f 值分布。
 *
 * 矩阵热力图：行 = 焦点概念的邻居，列 = 邻居的邻居（或同集合）
 * 颜色：cyan(0) → green(<5) → amber(<12) → red(≥12)
 * settled 区域蓝色边框标记
 * 点击 cell → 触发概念查询
 *
 * 数据来自 /query?concept=X（返回 neighbors 和 f_terrain）。
 */

import { useEffect, useState, useCallback } from "react";
import { T, FONT, fToColor, DAEMON_HTTP } from "../tokens";
import type { QueryResponse, NeighborInfo } from "../types";

// ── 类型 ────────────────────────────────────────────────────────

interface TerrainCell {
  rowLabel: string;       // 行概念标签
  colLabel: string;       // 列概念标签
  f: number;              // f 值（-1 = 无数据）
  rowSettled: boolean;
  colSettled: boolean;
  isHighlighted: boolean; // 焦点概念自身的邻居
}

interface Props {
  focusConcept: string | null;
  onSelectConcept: (concept: string) => void;
}

// ── 工具 ────────────────────────────────────────────────────────

async function queryNeighbors(concept: string): Promise<QueryResponse | null> {
  try {
    const res = await fetch(
      `${DAEMON_HTTP}/query?concept=${encodeURIComponent(concept)}`
    );
    if (!res.ok) return null;
    return res.json();
  } catch {
    return null;
  }
}

/** 截断标签用于显示 */
function shortLabel(label: string, maxLen = 16): string {
  if (label.length <= maxLen) return label;
  return label.slice(0, maxLen - 1) + "…";
}

/** f 值到热力颜色（不透明度变化版，用于背景 fill） */
function fToHeatColor(f: number): string {
  if (f < 0) return T.textMuted + "22";
  if (f <= 0.5) return T.fZero + "cc";
  if (f < 5) return T.fLow + "99";
  if (f < 12) return T.fMid + "99";
  return T.fHigh + "88";
}

// ── 组件 ────────────────────────────────────────────────────────

export function FTerrainHeatmap({ focusConcept, onSelectConcept }: Props) {
  const [queryResult, setQueryResult] = useState<QueryResponse | null>(null);
  const [neighborDetails, setNeighborDetails] = useState<Map<string, QueryResponse>>(new Map());
  const [hoveredCell, setHoveredCell] = useState<TerrainCell | null>(null);
  const [loading, setLoading] = useState(false);

  // ── 加载焦点概念数据 ─────────────────────────────────────────
  useEffect(() => {
    if (!focusConcept) return;

    let alive = true;
    setLoading(true);

    queryNeighbors(focusConcept).then((result) => {
      if (!alive) return;
      setQueryResult(result);
      setLoading(false);
    });

    return () => { alive = false; };
  }, [focusConcept]);

  // ── 加载邻居的邻居（二阶，限制 5 个） ──────────────────────
  useEffect(() => {
    if (!queryResult?.found || !queryResult.neighbors) return;

    const topNeighbors = queryResult.neighbors.slice(0, 8);
    let alive = true;

    async function loadNeighborDetails() {
      const details = new Map<string, QueryResponse>();
      for (const nb of topNeighbors.slice(0, 5)) {
        if (!alive) break;
        const res = await queryNeighbors(nb.label);
        if (res?.found) details.set(nb.label, res);
      }
      if (alive) setNeighborDetails(details);
    }

    loadNeighborDetails();
    return () => { alive = false; };
  }, [queryResult]);

  // ── 构建热力图矩阵 ───────────────────────────────────────────
  const { rowLabels, colLabels, matrix } = (() => {
    if (!queryResult?.found || !queryResult.neighbors) {
      return { rowLabels: [], colLabels: [], matrix: [] as TerrainCell[][] };
    }

    const neighbors = queryResult.neighbors.slice(0, 12);
    const rowLabels = neighbors.map((n) => n.label);

    // 列：邻居 + 它们的邻居（去重，最多 12 列）
    const colSet = new Set<string>(rowLabels);
    neighborDetails.forEach((detail) => {
      detail.neighbors.slice(0, 6).forEach((nb) => colSet.add(nb.label));
    });
    const colLabels = [...colSet].slice(0, 12);

    // 构建 f 值查找表：neighbor.label → f
    const fMap = new Map<string, number>();
    neighbors.forEach((nb) => fMap.set(nb.label, nb.f));

    // 邻居的邻居的 f 值
    neighborDetails.forEach((detail, _key) => {
      detail.neighbors.forEach((nb) => {
        if (!fMap.has(nb.label)) fMap.set(nb.label, nb.f);
      });
    });

    // settled 概念集合（从 f_terrain 推断：fold_zone 中的）
    const foldSet = new Set(queryResult.f_terrain.fold_zone);

    const matrix: TerrainCell[][] = rowLabels.map((rowLabel) => {
      return colLabels.map((colLabel) => {
        // 两个概念之间的 f 值（用可用的最好估计）
        let f = -1;
        if (rowLabel === colLabel) {
          f = 0;
        } else {
          // 从 neighborDetails 查找
          const rowDetail = neighborDetails.get(rowLabel);
          if (rowDetail) {
            const found = rowDetail.neighbors.find((nb) => nb.label === colLabel);
            if (found) f = found.f;
          }
          if (f < 0 && fMap.has(colLabel)) {
            f = fMap.get(colLabel)!;
          }
        }

        return {
          rowLabel,
          colLabel,
          f,
          rowSettled: foldSet.has(rowLabel),
          colSettled: foldSet.has(colLabel),
          isHighlighted: rowLabels.includes(colLabel),
        };
      });
    });

    return { rowLabels, colLabels, matrix };
  })();

  const handleCellClick = useCallback(
    (concept: string) => {
      onSelectConcept(concept);
    },
    [onSelectConcept]
  );

  // ── 无焦点状态 ───────────────────────────────────────────────
  if (!focusConcept) {
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
          F-TERRAIN HEATMAP
        </div>
        <div style={{
          flex: 1, display: "flex", alignItems: "center", justifyContent: "center",
          fontFamily: FONT.mono, fontSize: 11, color: T.textMuted,
        }}>
          点击拓扑图中的概念节点，或在对话框查询概念
        </div>
      </div>
    );
  }

  return (
    <div style={{
      width: "100%", height: "100%",
      display: "flex", flexDirection: "column",
      overflow: "hidden",
    }}>
      {/* 标题栏 */}
      <div style={{
        padding: "6px 12px", flexShrink: 0,
        fontFamily: FONT.mono, fontSize: 9, color: T.textMuted,
        letterSpacing: "0.1em",
        display: "flex", justifyContent: "space-between", alignItems: "center",
      }}>
        <span>F-TERRAIN HEATMAP</span>
        <span style={{ color: T.accent }}>
          {loading ? "加载中..." : focusConcept}
        </span>
        <div style={{ display: "flex", gap: 8, fontSize: 9 }}>
          <span style={{ color: T.fZero }}>■ f=0</span>
          <span style={{ color: T.fLow }}>■ fold</span>
          <span style={{ color: T.fMid }}>■ gray</span>
          <span style={{ color: T.fHigh }}>■ negate</span>
          <span style={{ color: T.settled }}>□ settled</span>
        </div>
      </div>

      {/* 热力图 */}
      <div style={{
        flex: 1, overflow: "auto",
        padding: "8px 12px",
      }}>
        {matrix.length === 0 ? (
          <div style={{
            display: "flex", alignItems: "center", justifyContent: "center",
            height: "100%",
            fontFamily: FONT.mono, fontSize: 11, color: T.textMuted,
          }}>
            {loading ? "查询中..." : "概念未找到"}
          </div>
        ) : (
          <table style={{
            borderCollapse: "collapse",
            fontFamily: FONT.mono, fontSize: 8,
          }}>
            {/* 列标头 */}
            <thead>
              <tr>
                <th style={{
                  width: 90, minWidth: 90,
                  textAlign: "right",
                  paddingRight: 6,
                  color: T.textMuted,
                  fontWeight: "normal",
                  verticalAlign: "bottom",
                }}>
                  {/* 左上角空白 */}
                </th>
                {colLabels.map((col) => (
                  <th
                    key={col}
                    style={{
                      width: 52, minWidth: 52,
                      writingMode: "vertical-rl",
                      transform: "rotate(180deg)",
                      height: 72,
                      textAlign: "left",
                      padding: "4px 4px 0 4px",
                      color: T.textDim,
                      fontWeight: "normal",
                      cursor: "pointer",
                      whiteSpace: "nowrap",
                    }}
                    onClick={() => handleCellClick(col)}
                    title={col}
                  >
                    {shortLabel(col, 14)}
                  </th>
                ))}
              </tr>
            </thead>

            {/* 矩阵行 */}
            <tbody>
              {matrix.map((row, ri) => (
                <tr key={rowLabels[ri]}>
                  {/* 行标签 */}
                  <td style={{
                    textAlign: "right",
                    paddingRight: 8,
                    color: T.textDim,
                    whiteSpace: "nowrap",
                    maxWidth: 90,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    cursor: "pointer",
                    verticalAlign: "middle",
                  }}
                    onClick={() => handleCellClick(rowLabels[ri])}
                    title={rowLabels[ri]}
                  >
                    {shortLabel(rowLabels[ri])}
                  </td>

                  {/* 热力格子 */}
                  {row.map((cell, ci) => {
                    const isHovered = hoveredCell?.rowLabel === cell.rowLabel &&
                      hoveredCell?.colLabel === cell.colLabel;
                    const isSettled = cell.rowSettled || cell.colSettled;
                    const isDiagonal = cell.rowLabel === cell.colLabel;

                    return (
                      <td
                        key={colLabels[ci]}
                        onMouseEnter={() => setHoveredCell(cell)}
                        onMouseLeave={() => setHoveredCell(null)}
                        onClick={() => !isDiagonal && handleCellClick(cell.colLabel)}
                        title={cell.f >= 0
                          ? `${cell.rowLabel} ↔ ${cell.colLabel}: f=${cell.f.toFixed(1)}`
                          : `${cell.rowLabel} ↔ ${cell.colLabel}: 无数据`
                        }
                        style={{
                          width: 48,
                          height: 36,
                          background: isDiagonal ? T.textMuted + "11" : fToHeatColor(cell.f),
                          border: isSettled
                            ? `1px solid ${T.settled}`
                            : `1px solid ${T.border}33`,
                          cursor: isDiagonal ? "default" : "pointer",
                          textAlign: "center",
                          verticalAlign: "middle",
                          transition: "opacity 0.1s",
                          opacity: isHovered ? 0.85 : 1,
                          outline: isHovered ? `2px solid ${T.accent}66` : "none",
                          outlineOffset: -2,
                          position: "relative",
                        }}
                      >
                        {isDiagonal ? (
                          <span style={{ color: T.textMuted, fontSize: 10 }}>—</span>
                        ) : cell.f >= 0 ? (
                          <span style={{
                            color: T.text,
                            fontWeight: cell.f <= 0.5 ? 700 : undefined,
                            fontSize: cell.f >= 10 ? 7 : 8,
                          }}>
                            {cell.f < 100 ? cell.f.toFixed(1) : "99+"}
                          </span>
                        ) : (
                          <span style={{ color: T.textMuted, fontSize: 9 }}>?</span>
                        )}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* 悬浮详情条 */}
      {hoveredCell && (
        <div style={{
          padding: "4px 12px", flexShrink: 0,
          fontFamily: FONT.mono, fontSize: 9,
          background: T.bgCard,
          borderTop: `1px solid ${T.border}`,
          display: "flex", gap: 16, alignItems: "center",
        }}>
          <span style={{ color: T.textDim }}>{hoveredCell.rowLabel}</span>
          <span style={{ color: T.textMuted }}>↔</span>
          <span style={{ color: T.textDim }}>{hoveredCell.colLabel}</span>
          <span style={{
            color: hoveredCell.f >= 0 ? fToColor(hoveredCell.f) : T.textMuted,
            fontWeight: 600,
          }}>
            {hoveredCell.f >= 0 ? `f = ${hoveredCell.f.toFixed(2)}` : "无数据"}
          </span>
          {(hoveredCell.rowSettled || hoveredCell.colSettled) && (
            <span style={{ color: T.settled }}>● settled</span>
          )}
          <span style={{ color: T.textMuted, marginLeft: "auto" }}>
            点击查询概念
          </span>
        </div>
      )}

      {/* f_terrain 区域摘要 */}
      {queryResult?.found && (
        <div style={{
          padding: "4px 12px", flexShrink: 0,
          fontFamily: FONT.mono, fontSize: 8, color: T.textMuted,
          borderTop: `1px solid ${T.border}`,
          display: "flex", gap: 12,
        }}>
          <span style={{ color: T.fLow }}>
            fold: {queryResult.f_terrain.fold_zone.length}
          </span>
          <span style={{ color: T.fMid }}>
            gray: {queryResult.f_terrain.gray_zone.length}
          </span>
          <span style={{ color: T.fHigh }}>
            negate: {queryResult.f_terrain.negate_zone.length}
          </span>
          <span style={{ color: T.textMuted }}>
            邻居: {queryResult.neighbors.length}
          </span>
        </div>
      )}
    </div>
  );
}
