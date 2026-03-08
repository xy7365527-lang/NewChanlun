/**
 * TopologyViewSwitcher.tsx
 *
 * 左侧面板的拓扑视图切换器。
 * 统一管理五个视图的渲染：
 *   - 2D 力导向（原有 TopologyView，作为 fallback）
 *   - 宇宙星系图（GalaxyView）—— 默认 3D 视图（替换旧"深空观测站"）
 *   - 持久同调（PersistenceDiagram）
 *   - f 地形热力图（FTerrainHeatmap）
 *   - 穿越轨迹（TraversalTimeline）
 *
 * 自适应视图推荐（★）：
 *   - β₁ 高且增长中 → 推荐 Persistence Diagram
 *   - encounter 密集 → 推荐 f 地形
 *   - 穿越稳定（walk 为主）→ 推荐 Traversal Timeline
 *   - 默认 → 推荐宇宙星系图（galaxy）
 */

import { useState, useMemo } from "react";
import { T, FONT } from "../tokens";
import { TopologyView } from "../components/TopologyView";
import type { InstanceTraversal } from "../components/TopologyView";
import { GalaxyView } from "./GalaxyView";
import { PersistenceDiagram } from "./PersistenceDiagram";
import { FTerrainHeatmap } from "./FTerrainHeatmap";
import { TraversalTimeline } from "./TraversalTimeline";
import type {
  TopologyResponse, TopologyNode, StatusResponse, NarrativeEvent,
} from "../types";

// ── 类型 ────────────────────────────────────────────────────────

type ViewId = "2d" | "galaxy" | "persistence" | "terrain" | "timeline";

interface Beta1Point {
  step: number;
  beta1: number;
}

interface Props {
  data: TopologyResponse | null;
  traversalPosition?: string;
  instanceTraversals?: InstanceTraversal[];
  focusConcept?: string | null;
  onSelectNode?: (node: TopologyNode) => void;
  onSelectConcept?: (concept: string) => void;
  status: StatusResponse | null;
  beta1History: Beta1Point[];
  narrative: NarrativeEvent[];
}

// ── 视图推荐算法 ────────────────────────────────────────────────

function recommendView(
  status: StatusResponse | null,
  beta1History: Beta1Point[],
  narrative: NarrativeEvent[]
): ViewId {
  if (!status) return "galaxy";

  // β₁ 高（>100）且在增长 → Persistence Diagram
  if (status.beta_1 > 100) {
    if (beta1History.length >= 2) {
      const last = beta1History[beta1History.length - 1].beta1;
      const prev = beta1History[Math.max(0, beta1History.length - 10)].beta1;
      if (last > prev) return "persistence";
    }
    return "persistence";
  }

  // encounter 密集（encounter_density 高）→ f 地形
  if (status.encounter_density > 0.7) return "terrain";

  // 穿越稳定（walk 为主，从 narrative 判断） → Traversal Timeline
  const recentNarrative = narrative.slice(0, 30);
  const walkCount = recentNarrative.filter(
    (ev) => ev.text?.includes("] walk ")
  ).length;
  const encounterCount = recentNarrative.length - walkCount;
  if (walkCount > encounterCount * 3) return "timeline";

  return "galaxy";
}

// ── 视图 tab 定义 ────────────────────────────────────────────────

const VIEW_TABS: Array<{ id: ViewId; label: string; shortLabel: string }> = [
  { id: "2d",          label: "2D 力导向",   shortLabel: "2D"   },
  { id: "galaxy",      label: "宇宙星系图",   shortLabel: "星系" },
  { id: "persistence", label: "持久同调",    shortLabel: "同调" },
  { id: "terrain",     label: "f 地形",      shortLabel: "地形" },
  { id: "timeline",    label: "穿越轨迹",    shortLabel: "轨迹" },
];

// ── 组件 ─────────────────────────────────────────────────────────

export function TopologyViewSwitcher({
  data, traversalPosition, instanceTraversals, focusConcept,
  onSelectNode, onSelectConcept,
  status, beta1History, narrative,
}: Props) {
  const [activeView, setActiveView] = useState<ViewId>("galaxy");

  const recommended = useMemo(
    () => recommendView(status, beta1History, narrative),
    [status, beta1History, narrative]
  );

  // 包装 onSelectNode 为 onSelectConcept（对热力图）
  const handleSelectConcept = (concept: string) => {
    onSelectConcept?.(concept);
    // 同时通过 onSelectNode 触发（构造假节点，让 App 处理）
    if (onSelectNode) {
      const syntheticNode: TopologyNode = {
        id: concept,
        label: concept,
        f_avg: -1,
        degree: 0,
        settled: false,
        type: "text",
      };
      onSelectNode(syntheticNode);
    }
  };

  return (
    <div style={{
      width: "100%", height: "100%",
      display: "flex", flexDirection: "column",
    }}>
      {/* Tab 切换栏 */}
      <div style={{
        display: "flex",
        borderBottom: `1px solid ${T.border}`,
        background: T.bgPanel,
        flexShrink: 0,
        overflowX: "auto",
      }}>
        {VIEW_TABS.map((tab) => {
          const isActive      = activeView === tab.id;
          const isRecommended = recommended === tab.id;
          return (
            <button
              key={tab.id}
              onClick={() => setActiveView(tab.id)}
              title={isRecommended ? `★ 推荐当前视图：${tab.label}` : tab.label}
              style={{
                background: "transparent",
                border: "none",
                padding: "8px 12px",
                fontFamily: FONT.mono,
                fontSize: 10,
                cursor: "pointer",
                color: isActive ? T.text : T.textDim,
                borderBottom: isActive
                  ? `2px solid ${T.accent}`
                  : "2px solid transparent",
                letterSpacing: "0.04em",
                transition: "all 0.12s",
                whiteSpace: "nowrap",
                position: "relative",
                display: "flex",
                alignItems: "center",
                gap: 4,
              }}
            >
              {isRecommended && (
                <span style={{ color: T.fMid, fontSize: 8, lineHeight: 1 }}>
                  ★
                </span>
              )}
              {tab.shortLabel}
            </button>
          );
        })}

        {/* 右侧推荐状态说明 */}
        <div style={{
          marginLeft: "auto",
          display: "flex", alignItems: "center",
          paddingRight: 10,
          fontFamily: FONT.mono, fontSize: 8,
          color: T.textMuted,
          whiteSpace: "nowrap",
        }}>
          ★ = 推荐
        </div>
      </div>

      {/* 视图内容区域 */}
      <div style={{ flex: 1, overflow: "hidden", position: "relative" }}>
        {activeView === "2d" && (
          <TopologyView
            data={data}
            traversalPosition={traversalPosition}
            instanceTraversals={instanceTraversals}
            focusConcept={focusConcept}
            onSelectNode={onSelectNode}
          />
        )}
        {activeView === "galaxy" && (
          <GalaxyView
            data={data}
            traversalPosition={traversalPosition}
            focusConcept={focusConcept}
            onSelectNode={onSelectNode}
          />
        )}
        {activeView === "persistence" && (
          <PersistenceDiagram status={status} />
        )}
        {activeView === "terrain" && (
          <FTerrainHeatmap
            focusConcept={focusConcept ?? null}
            onSelectConcept={handleSelectConcept}
          />
        )}
        {activeView === "timeline" && (
          <TraversalTimeline
            beta1History={beta1History}
            narrative={narrative}
          />
        )}
      </div>
    </div>
  );
}
