import { useEffect, useRef, useState } from "react";
import * as d3 from "d3";
import { T, FONT, fToColor } from "../tokens";
import type { TopologyNode, TopologyLink, TopologyResponse } from "../types";

export interface InstanceTraversal {
  instanceId: string;
  instanceName: string;
  position: string;         // vertex id
  color: string;
  history: string[];        // recent position ids for path rendering
}

interface Props {
  data: TopologyResponse | null;
  traversalPosition?: string;               // single-instance fallback
  instanceTraversals?: InstanceTraversal[];  // multi-instance markers
  focusConcept?: string | null;
  onSelectNode?: (node: TopologyNode) => void;
}

export function TopologyView({
  data, traversalPosition, instanceTraversals, focusConcept, onSelectNode,
}: Props) {
  const svgRef = useRef<SVGSVGElement>(null);
  const simRef = useRef<d3.Simulation<TopologyNode, TopologyLink> | null>(null);
  const transformRef = useRef<d3.ZoomTransform>(d3.zoomIdentity);
  const nodePositionsRef = useRef<Map<string, { x: number; y: number }>>(new Map());
  const [hoveredNode, setHoveredNode] = useState<TopologyNode | null>(null);

  // Build traversal lookup: nodeId -> list of instances at that node
  const traversalMap = new Map<string, InstanceTraversal[]>();
  if (instanceTraversals && instanceTraversals.length > 0) {
    for (const it of instanceTraversals) {
      if (!it.position) continue;
      const existing = traversalMap.get(it.position) ?? [];
      existing.push(it);
      traversalMap.set(it.position, existing);
    }
  } else if (traversalPosition) {
    // Fallback: single instance with default color
    traversalMap.set(traversalPosition, [{
      instanceId: "default",
      instanceName: "local",
      position: traversalPosition,
      color: T.traversalPulse,
      history: [],
    }]);
  }

  // Collect all history node IDs per instance for path highlighting
  const historyMap = new Map<string, { nodeIds: Set<string>; color: string }>();
  if (instanceTraversals) {
    for (const it of instanceTraversals) {
      historyMap.set(it.instanceId, {
        nodeIds: new Set(it.history),
        color: it.color,
      });
    }
  }

  useEffect(() => {
    if (!svgRef.current || !data) return;

    const svg = d3.select<SVGSVGElement, unknown>(svgRef.current);

    // Save current node positions before teardown
    if (simRef.current) {
      const posMap = new Map<string, { x: number; y: number }>();
      for (const n of simRef.current.nodes()) {
        if (n.x != null && n.y != null) {
          posMap.set(n.id, { x: n.x, y: n.y });
        }
      }
      nodePositionsRef.current = posMap;
    }

    svg.selectAll("*").remove();

    const width = svgRef.current.clientWidth || 800;
    const height = svgRef.current.clientHeight || 600;
    const hasOldPositions = nodePositionsRef.current.size > 0;

    // Check if node is a traversal target for any instance
    const isTraversalNode = (id: string) => traversalMap.has(id);

    // ── Performance: filter low-degree nodes for large graphs ────
    const PERF_THRESHOLD = 5000; // only filter when graph is large
    const MIN_RENDER_DEGREE = data.nodes.length > PERF_THRESHOLD ? 2 : 0;

    const nodeIdSet = new Set<string>();
    const filteredDataNodes = data.nodes.filter((n) => {
      // Always keep: traversal targets, focus, settled, high-degree
      if (isTraversalNode(n.id) || n.label === focusConcept || n.settled || n.degree >= MIN_RENDER_DEGREE) {
        nodeIdSet.add(n.id);
        return true;
      }
      return false;
    });

    // Mark traversal node + restore previous positions
    const nodes: TopologyNode[] = filteredDataNodes.map((n) => {
      const prev = nodePositionsRef.current.get(n.id);
      return {
        ...n,
        isTraversal: isTraversalNode(n.id),
        ...(prev ? { x: prev.x, y: prev.y } : {}),
      };
    });

    // Deep-copy links — only keep links where both endpoints are rendered
    const links: TopologyLink[] = data.links
      .filter((l) => {
        const src = typeof l.source === "string" ? l.source : (l.source as TopologyNode).id;
        const tgt = typeof l.target === "string" ? l.target : (l.target as TopologyNode).id;
        return nodeIdSet.has(src) && nodeIdSet.has(tgt);
      })
      .map((l) => ({
      source: typeof l.source === "string" ? l.source : (l.source as TopologyNode).id,
      target: typeof l.target === "string" ? l.target : (l.target as TopologyNode).id,
      type: l.type,
      f: l.f,
    }));

    // ── Defs ───────────────────────────────────────────────────
    const defs = svg.append("defs");

    const settledFilter = defs.append("filter").attr("id", "settledGlow");
    settledFilter.append("feGaussianBlur").attr("stdDeviation", "4").attr("result", "blur");
    const settledMerge = settledFilter.append("feMerge");
    settledMerge.append("feMergeNode").attr("in", "blur");
    settledMerge.append("feMergeNode").attr("in", "SourceGraphic");

    const focusFilter = defs.append("filter").attr("id", "focusGlow");
    focusFilter.append("feGaussianBlur").attr("stdDeviation", "6").attr("result", "blur");
    const focusMerge = focusFilter.append("feMerge");
    focusMerge.append("feMergeNode").attr("in", "blur");
    focusMerge.append("feMergeNode").attr("in", "SourceGraphic");

    // Per-instance glow filters
    const instanceColors = new Set<string>();
    for (const entries of traversalMap.values()) {
      for (const e of entries) instanceColors.add(e.color);
    }
    for (const entries of historyMap.values()) {
      instanceColors.add(entries.color);
    }
    for (const color of instanceColors) {
      const filterId = `glow-${color.replace("#", "")}`;
      const f = defs.append("filter").attr("id", filterId);
      f.append("feGaussianBlur").attr("stdDeviation", "8").attr("result", "blur");
      f.append("feFlood").attr("flood-color", color).attr("flood-opacity", "0.6").attr("result", "color");
      f.append("feComposite").attr("in", "color").attr("in2", "blur").attr("operator", "in").attr("result", "colorBlur");
      const merge = f.append("feMerge");
      merge.append("feMergeNode").attr("in", "colorBlur");
      merge.append("feMergeNode").attr("in", "SourceGraphic");
    }

    // ── Container with zoom ───────────────────────────────────
    const g = svg.append("g");

    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.2, 8])
      .on("zoom", (event) => {
        transformRef.current = event.transform;
        g.attr("transform", event.transform.toString());
      });
    svg.call(zoom);

    // Restore previous zoom transform
    if (transformRef.current !== d3.zoomIdentity) {
      svg.call(zoom.transform, transformRef.current);
    }

    // ── Links ─────────────────────────────────────────────────
    const link = g.append("g")
      .selectAll<SVGLineElement, TopologyLink>("line")
      .data(links)
      .enter()
      .append("line")
      .attr("stroke", (d) =>
        d.type === "negation" ? T.fHigh + "66" : T.textMuted + "33"
      )
      .attr("stroke-width", (d) => d.type === "negation" ? 1.5 : 0.5)
      .attr("stroke-dasharray", (d) => d.type === "negation" ? "4,4" : "none");

    // ── Settled rings ─────────────────────────────────────────
    const settledRings = g.append("g")
      .selectAll<SVGCircleElement, TopologyNode>("circle")
      .data(nodes.filter((n) => n.settled))
      .enter()
      .append("circle")
      .attr("r", 18)
      .attr("fill", "none")
      .attr("stroke", T.settled)
      .attr("stroke-width", 1.5)
      .attr("opacity", 0.6)
      .attr("filter", "url(#settledGlow)");

    // ── Nodes ─────────────────────────────────────────────────
    const node = g.append("g")
      .selectAll<SVGCircleElement, TopologyNode>("circle")
      .data(nodes)
      .enter()
      .append("circle")
      .attr("r", (d) => {
        if (d.isTraversal) return 9;
        if (d.label === focusConcept) return 8;
        return 2 + Math.sqrt(Math.min(d.degree, 100)) * 0.8;
      })
      .attr("fill", (d) => {
        if (d.isTraversal) {
          const instances = traversalMap.get(d.id);
          if (instances && instances.length === 1) return instances[0].color;
          if (instances && instances.length > 1) return T.traversalPulse; // multi: white
          return T.traversalPulse;
        }
        return fToColor(d.f_avg);
      })
      .attr("opacity", (d) => d.isTraversal ? 1 : 0.7 + Math.min(d.degree / 12, 0.3))
      .attr("filter", (d) => {
        if (d.isTraversal) {
          const instances = traversalMap.get(d.id);
          if (instances && instances.length === 1) {
            return `url(#glow-${instances[0].color.replace("#", "")})`;
          }
          return "none"; // multi-instance overlap: no single glow
        }
        if (d.label === focusConcept) return "url(#focusGlow)";
        return "none";
      })
      .attr("cursor", "pointer")
      .on("mouseenter", (_event, d) => setHoveredNode(d))
      .on("mouseleave", () => setHoveredNode(null))
      .on("click", (_event, d) => onSelectNode?.(d))
      .call(
        d3.drag<SVGCircleElement, TopologyNode>()
          .on("start", (event, d) => {
            if (!event.active) simRef.current?.alphaTarget(0.3).restart();
            d.fx = d.x;
            d.fy = d.y;
          })
          .on("drag", (event, d) => {
            d.fx = event.x;
            d.fy = event.y;
          })
          .on("end", (event, d) => {
            if (!event.active) simRef.current?.alphaTarget(0);
            d.fx = null;
            d.fy = null;
          })
      );

    // ── Multi-instance traversal rings (when multiple instances at same node) ──
    const multiTraversalNodes = nodes.filter((n) => {
      const instances = traversalMap.get(n.id);
      return instances && instances.length > 1;
    });

    // Draw concentric colored rings for multi-instance overlap
    const multiRingGroup = g.append("g");
    for (const n of multiTraversalNodes) {
      const instances = traversalMap.get(n.id)!;
      instances.forEach((inst, idx) => {
        multiRingGroup.append("circle")
          .datum(n)
          .attr("r", 12 + idx * 4)
          .attr("fill", "none")
          .attr("stroke", inst.color)
          .attr("stroke-width", 2)
          .attr("opacity", 0.8)
          .attr("filter", `url(#glow-${inst.color.replace("#", "")})`)
          .attr("class", `multi-ring-${n.id}`);
      });
    }

    // ── Instance traversal path highlights ────────────────────
    // For each visible instance, draw faint colored rings on history nodes
    const pathGroup = g.append("g");
    const renderedNodeIds = new Set(nodes.map((n) => n.id));
    for (const [, { nodeIds, color }] of historyMap) {
      for (const nodeId of nodeIds) {
        if (!renderedNodeIds.has(nodeId) || traversalMap.has(nodeId)) continue;
        const matchNode = nodes.find((n) => n.id === nodeId);
        if (!matchNode) continue;
        pathGroup.append("circle")
          .datum(matchNode)
          .attr("r", 6)
          .attr("fill", "none")
          .attr("stroke", color)
          .attr("stroke-width", 1)
          .attr("opacity", 0.3)
          .attr("class", "path-marker");
      }
    }

    // ── Labels ────────────────────────────────────────────────
    const degreeSorted = [...nodes].sort((a, b) => b.degree - a.degree);
    const topN = new Set(degreeSorted.slice(0, 10).map((n) => n.id));
    const labelNodes = nodes.filter(
      (n) => n.isTraversal || n.label === focusConcept || topN.has(n.id)
    );

    const labelBg = g.append("g")
      .selectAll<SVGRectElement, TopologyNode>("rect")
      .data(labelNodes)
      .enter()
      .append("rect")
      .attr("fill", T.bg + "cc")
      .attr("rx", 2)
      .attr("pointer-events", "none");

    const label = g.append("g")
      .selectAll<SVGTextElement, TopologyNode>("text")
      .data(labelNodes)
      .enter()
      .append("text")
      .text((d) => {
        let lbl = d.label;
        if (lbl.startsWith("syn_") || lbl.startsWith("anti_")) lbl = lbl.slice(0, 12) + "\u2026";
        else if (lbl.length > 24) lbl = lbl.slice(0, 22) + "\u2026";
        return lbl;
      })
      .attr("fill", (d) => {
        if (d.isTraversal) {
          const instances = traversalMap.get(d.id);
          if (instances && instances.length === 1) return instances[0].color;
          return T.text;
        }
        return T.textDim;
      })
      .attr("font-size", "8px")
      .attr("font-family", FONT.mono)
      .attr("text-anchor", "middle")
      .attr("dy", (d) => -(6 + Math.min(d.degree, 8) * 0.5 + 4))
      .attr("pointer-events", "none");

    // ── Simulation ────────────────────────────────────────────
    const sim = d3.forceSimulation<TopologyNode, TopologyLink>(nodes)
      .force(
        "link",
        d3.forceLink<TopologyNode, TopologyLink>(links)
          .id((d) => d.id)
          .distance(80)
          .strength(0.3)
      )
      .force("charge", d3.forceManyBody().strength(-200).distanceMax(400))
      .force("center", d3.forceCenter(width / 2, height / 2).strength(0.05))
      .force("collision", d3.forceCollide(16))
      .alpha(hasOldPositions ? 0.1 : 1)
      .on("tick", () => {
        link
          .attr("x1", (d) => (d.source as TopologyNode).x ?? 0)
          .attr("y1", (d) => (d.source as TopologyNode).y ?? 0)
          .attr("x2", (d) => (d.target as TopologyNode).x ?? 0)
          .attr("y2", (d) => (d.target as TopologyNode).y ?? 0);

        node
          .attr("cx", (d) => d.x ?? 0)
          .attr("cy", (d) => d.y ?? 0);

        settledRings
          .attr("cx", (d) => d.x ?? 0)
          .attr("cy", (d) => d.y ?? 0);

        // Update multi-instance rings
        for (const n of multiTraversalNodes) {
          svg.selectAll(`.multi-ring-${CSS.escape(n.id)}`)
            .attr("cx", n.x ?? 0)
            .attr("cy", n.y ?? 0);
        }

        // Update path markers
        pathGroup.selectAll<SVGCircleElement, TopologyNode>(".path-marker")
          .attr("cx", (d) => d.x ?? 0)
          .attr("cy", (d) => d.y ?? 0);

        label
          .attr("x", (d) => d.x ?? 0)
          .attr("y", (d) => d.y ?? 0);

        labelBg.each(function (d, i) {
          const textEl = label.nodes()[i];
          if (textEl) {
            const bbox = textEl.getBBox();
            d3.select(this)
              .attr("x", bbox.x - 2)
              .attr("y", bbox.y - 1)
              .attr("width", bbox.width + 4)
              .attr("height", bbox.height + 2);
          }
        });
      });

    simRef.current = sim;
    return () => { sim.stop(); };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data, focusConcept, traversalPosition, instanceTraversals, onSelectNode]);

  // Build legend entries for instances
  const instanceLegend = instanceTraversals && instanceTraversals.length > 0
    ? instanceTraversals
    : traversalPosition
      ? [{ instanceId: "default", instanceName: "local", color: T.traversalPulse, position: traversalPosition, history: [] }]
      : [];

  return (
    <div style={{ position: "relative", width: "100%", height: "100%" }}>
      <svg
        ref={svgRef}
        width="100%"
        height="100%"
        style={{ background: T.bg, display: "block" }}
      />

      {/* Hover tooltip */}
      {hoveredNode && (
        <div style={{
          position: "absolute", bottom: 12, left: 12,
          background: T.bgCard + "ee",
          border: `1px solid ${T.border}`,
          borderRadius: 6, padding: "8px 12px",
          fontFamily: FONT.mono, fontSize: 11, color: T.text,
          backdropFilter: "blur(8px)",
          pointerEvents: "none",
        }}>
          <span style={{ color: fToColor(hoveredNode.f_avg), fontWeight: 600 }}>
            {hoveredNode.label.slice(0, 50)}
          </span>
          <span style={{ color: T.textDim, marginLeft: 12 }}>
            f={hoveredNode.f_avg.toFixed(1)}
          </span>
          <span style={{ color: T.textDim, marginLeft: 8 }}>
            deg={hoveredNode.degree}
          </span>
          {hoveredNode.settled && (
            <span style={{ color: T.settled, marginLeft: 8 }}>settled</span>
          )}
          {/* Show which instances are at this node */}
          {traversalMap.has(hoveredNode.id) && (
            <span style={{ marginLeft: 8 }}>
              {traversalMap.get(hoveredNode.id)!.map((it) => (
                <span key={it.instanceId} style={{ color: it.color, marginLeft: 4 }}>
                  [{it.instanceName}]
                </span>
              ))}
            </span>
          )}
        </div>
      )}

      {/* Legend */}
      <div style={{
        position: "absolute", top: 12, right: 12,
        background: T.bgCard + "cc",
        border: `1px solid ${T.border}`,
        borderRadius: 6, padding: "8px 12px",
        fontFamily: FONT.mono, fontSize: 9, color: T.textDim,
        display: "flex", flexDirection: "column", gap: 6,
        pointerEvents: "none",
      }}>
        <div style={{ display: "flex", gap: 12 }}>
          <span><span style={{ color: T.fZero }}>&#9679;</span> f=0</span>
          <span><span style={{ color: T.fLow }}>&#9679;</span> fold</span>
          <span><span style={{ color: T.fMid }}>&#9679;</span> gray</span>
          <span><span style={{ color: T.fHigh }}>&#9679;</span> negate</span>
          <span><span style={{ color: T.settled }}>&#9675;</span> settled</span>
        </div>
        {instanceLegend.length > 0 && (
          <div style={{ display: "flex", gap: 10, borderTop: `1px solid ${T.border}`, paddingTop: 4 }}>
            {instanceLegend.map((it) => (
              <span key={it.instanceId}>
                <span style={{ color: it.color }}>&#9673;</span> {it.instanceName}
              </span>
            ))}
          </div>
        )}
      </div>

      {/* Meta info */}
      {data?.meta && (
        <div style={{
          position: "absolute", bottom: 12, right: 12,
          background: T.bgCard + "cc",
          border: `1px solid ${T.border}`,
          borderRadius: 4, padding: "4px 8px",
          fontFamily: FONT.mono, fontSize: 9, color: T.textMuted,
          pointerEvents: "none",
        }}>
          {data.meta.total_edges === -1 ? "skeleton " : ""}
          {data.meta.shown_vertices.toLocaleString()}V /&nbsp;
          {data.meta.total_vertices.toLocaleString()} total
        </div>
      )}
    </div>
  );
}
