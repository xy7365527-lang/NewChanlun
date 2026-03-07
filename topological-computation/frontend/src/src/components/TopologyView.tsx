import { useEffect, useRef, useState } from "react";
import * as d3 from "d3";
import { T, FONT, fToColor } from "../tokens";
import type { TopologyNode, TopologyLink, TopologyResponse } from "../types";

interface Props {
  data: TopologyResponse | null;
  traversalPosition?: string;   // vertex id of current daemon position
  focusConcept?: string | null;
  onSelectNode?: (node: TopologyNode) => void;
}

export function TopologyView({
  data, traversalPosition, focusConcept, onSelectNode,
}: Props) {
  const svgRef = useRef<SVGSVGElement>(null);
  const simRef = useRef<d3.Simulation<TopologyNode, TopologyLink> | null>(null);
  const [hoveredNode, setHoveredNode] = useState<TopologyNode | null>(null);

  useEffect(() => {
    if (!svgRef.current || !data) return;

    const svg = d3.select<SVGSVGElement, unknown>(svgRef.current);
    svg.selectAll("*").remove();

    const width = svgRef.current.clientWidth || 800;
    const height = svgRef.current.clientHeight || 600;

    // Mark traversal node
    const nodes: TopologyNode[] = data.nodes.map((n) => ({
      ...n,
      isTraversal: n.id === traversalPosition,
    }));

    // Deep-copy links because d3 mutates source/target from string → object
    const links: TopologyLink[] = data.links.map((l) => ({
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

    const traversalFilter = defs.append("filter").attr("id", "traversalGlow");
    traversalFilter.append("feGaussianBlur").attr("stdDeviation", "8").attr("result", "blur");
    const traversalMerge = traversalFilter.append("feMerge");
    traversalMerge.append("feMergeNode").attr("in", "blur");
    traversalMerge.append("feMergeNode").attr("in", "SourceGraphic");

    const focusFilter = defs.append("filter").attr("id", "focusGlow");
    focusFilter.append("feGaussianBlur").attr("stdDeviation", "6").attr("result", "blur");
    const focusMerge = focusFilter.append("feMerge");
    focusMerge.append("feMergeNode").attr("in", "blur");
    focusMerge.append("feMergeNode").attr("in", "SourceGraphic");

    // ── Container with zoom ───────────────────────────────────
    const g = svg.append("g");

    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.2, 8])
      .on("zoom", (event) => g.attr("transform", event.transform.toString()));
    svg.call(zoom);

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
        // Scale by degree: sqrt for natural visual weight
        return 2 + Math.sqrt(Math.min(d.degree, 100)) * 0.8;
      })
      .attr("fill", (d) => {
        if (d.isTraversal) return T.traversalPulse;
        return fToColor(d.f_avg);
      })
      .attr("opacity", (d) => d.isTraversal ? 1 : 0.7 + Math.min(d.degree / 12, 0.3))
      .attr("filter", (d) => {
        if (d.isTraversal) return "url(#traversalGlow)";
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

    // ── Labels ────────────────────────────────────────────────
    // Only label top-10 by degree + traversal + focus to avoid overlap
    const degreeSorted = [...nodes].sort((a, b) => b.degree - a.degree);
    const topN = new Set(degreeSorted.slice(0, 10).map((n) => n.id));
    const labelNodes = nodes.filter(
      (n) => n.isTraversal || n.label === focusConcept || topN.has(n.id)
    );

    // Label backgrounds for readability
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
        // Clean label: trim hash prefixes, show meaningful part
        let lbl = d.label;
        if (lbl.startsWith("syn_") || lbl.startsWith("anti_")) lbl = lbl.slice(0, 12) + "…";
        else if (lbl.length > 24) lbl = lbl.slice(0, 22) + "…";
        return lbl;
      })
      .attr("fill", (d) => d.isTraversal ? T.text : T.textDim)
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

        label
          .attr("x", (d) => d.x ?? 0)
          .attr("y", (d) => d.y ?? 0);

        // Update label backgrounds
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
  }, [data, focusConcept, traversalPosition, onSelectNode]);

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
            <span style={{ color: T.settled, marginLeft: 8 }}>● settled</span>
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
        display: "flex", gap: 12,
        pointerEvents: "none",
      }}>
        <span><span style={{ color: T.fZero }}>●</span> f=0</span>
        <span><span style={{ color: T.fLow }}>●</span> fold</span>
        <span><span style={{ color: T.fMid }}>●</span> gray</span>
        <span><span style={{ color: T.fHigh }}>●</span> negate</span>
        <span><span style={{ color: T.settled }}>○</span> settled</span>
        <span><span style={{ color: T.traversalPulse }}>◉</span> here</span>
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
          showing {data.meta.shown_vertices.toLocaleString()}V /&nbsp;
          {data.meta.total_vertices.toLocaleString()} total
        </div>
      )}
    </div>
  );
}
