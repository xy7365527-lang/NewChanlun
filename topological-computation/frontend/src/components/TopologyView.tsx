import { useEffect, useRef, useState, useCallback } from "react";
import * as d3 from "d3";
import { T, FONT, fToColor } from "../tokens";
import type { TopologyNode, TopologyLink, TopologyResponse } from "../types";

// Deep navy background instead of pure black
const BG_COLOR = "#0a0a1a";

interface Props {
  data: TopologyResponse | null;
  traversalPosition?: string;   // vertex id of current daemon position
  focusConcept?: string | null;
  onSelectNode?: (node: TopologyNode) => void;
}

// ---------------------------------------------------------------------------
// Spatial index: uniform grid for O(1) hit testing
// ---------------------------------------------------------------------------

interface SpatialGrid {
  cellSize: number;
  cells: Map<string, number[]>;  // "col,row" -> array of node indices
}

function buildSpatialGrid(
  nodes: TopologyNode[],
  cellSize: number,
): SpatialGrid {
  const cells = new Map<string, number[]>();
  for (let i = 0; i < nodes.length; i++) {
    const n = nodes[i];
    const col = Math.floor((n.x ?? 0) / cellSize);
    const row = Math.floor((n.y ?? 0) / cellSize);
    const key = `${col},${row}`;
    const bucket = cells.get(key);
    if (bucket) {
      bucket.push(i);
    } else {
      cells.set(key, [i]);
    }
  }
  return { cellSize, cells };
}

function findNearest(
  grid: SpatialGrid,
  nodes: TopologyNode[],
  mx: number,
  my: number,
  maxDist: number,
): TopologyNode | null {
  const col = Math.floor(mx / grid.cellSize);
  const row = Math.floor(my / grid.cellSize);
  const radius = Math.ceil(maxDist / grid.cellSize);
  let best: TopologyNode | null = null;
  let bestDist = maxDist * maxDist;

  for (let dc = -radius; dc <= radius; dc++) {
    for (let dr = -radius; dr <= radius; dr++) {
      const bucket = grid.cells.get(`${col + dc},${row + dr}`);
      if (!bucket) continue;
      for (const idx of bucket) {
        const n = nodes[idx];
        const dx = (n.x ?? 0) - mx;
        const dy = (n.y ?? 0) - my;
        const d2 = dx * dx + dy * dy;
        if (d2 < bestDist) {
          bestDist = d2;
          best = n;
        }
      }
    }
  }
  return best;
}

// ---------------------------------------------------------------------------
// Node radius helper (shared between simulation and rendering)
// ---------------------------------------------------------------------------

function nodeRadius(
  d: TopologyNode,
  focusConcept: string | null | undefined,
): number {
  if (d.isTraversal) return 5;
  if (d.label === focusConcept) return 4.5;
  return 1 + Math.sqrt(Math.min(d.degree, 100)) * 0.5;
}

// ---------------------------------------------------------------------------
// Edge color helpers
// ---------------------------------------------------------------------------

function parseHexAlpha(hex: string, alpha: number): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r},${g},${b},${alpha})`;
}

// ---------------------------------------------------------------------------
// Synthetic edge generation (for when backend returns links=[])
// ---------------------------------------------------------------------------
// When the backend skips edge serialization for performance, we generate
// lightweight synthetic edges from node proximity within the same cluster.
// These are purely visual — they capture the cluster structure that would
// exist in the real graph.

function generateSyntheticEdges(
  nodes: TopologyNode[],
  maxEdges: number,
): TopologyLink[] {
  if (nodes.length === 0) return [];

  // Group by type (cluster key)
  const byType: Record<string, TopologyNode[]> = {};
  for (const n of nodes) {
    const k = n.type || "other";
    if (!byType[k]) byType[k] = [];
    byType[k].push(n);
  }

  const edges: TopologyLink[] = [];
  const edgesPerCluster = Math.ceil(maxEdges / Math.max(Object.keys(byType).length, 1));

  for (const members of Object.values(byType)) {
    // Sort by degree descending — connect high-degree nodes to form a spine
    const sorted = [...members].sort((a, b) => b.degree - a.degree);

    // Spine: top nodes connected linearly
    const spineLen = Math.min(sorted.length - 1, Math.ceil(edgesPerCluster * 0.6));
    for (let i = 0; i < spineLen; i++) {
      edges.push({
        source: sorted[i].id,
        target: sorted[i + 1].id,
        type: sorted[i].f_avg > 10 ? "negation" : "inclusion",
      });
    }

    // Leaf edges: connect remaining nodes to nearest spine member
    const leafLen = Math.min(
      members.length - spineLen - 1,
      edgesPerCluster - spineLen,
    );
    for (let i = spineLen + 1; i < spineLen + 1 + leafLen; i++) {
      if (i >= sorted.length) break;
      // Connect to a random spine node
      const spineIdx = Math.floor(Math.random() * Math.max(spineLen, 1));
      edges.push({
        source: sorted[spineIdx].id,
        target: sorted[i].id,
        type: sorted[i].f_avg > 10 ? "negation" : "inclusion",
      });
    }

    if (edges.length >= maxEdges) break;
  }

  // Resolve string IDs to node objects in-place (d3 forceLink style)
  const nodeById = new Map(nodes.map((n) => [n.id, n]));
  for (const e of edges) {
    if (typeof e.source === "string") {
      const n = nodeById.get(e.source);
      if (n) e.source = n;
    }
    if (typeof e.target === "string") {
      const n = nodeById.get(e.target);
      if (n) e.target = n;
    }
  }

  return edges;
}

// ---------------------------------------------------------------------------
// Top-20 high-degree node indices (for glow effect)
// ---------------------------------------------------------------------------

function buildTopDegreeSet(nodes: TopologyNode[], n: number): Set<string> {
  const sorted = [...nodes].sort((a, b) => b.degree - a.degree);
  return new Set(sorted.slice(0, n).map((nd) => nd.id));
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function TopologyView({
  data, traversalPosition, focusConcept, onSelectNode,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const overlayRef = useRef<HTMLDivElement>(null);
  const nodesRef = useRef<TopologyNode[]>([]);
  const linksRef = useRef<TopologyLink[]>([]);
  const gridRef = useRef<SpatialGrid | null>(null);
  const transformRef = useRef<d3.ZoomTransform>(d3.zoomIdentity);
  const rafRef = useRef<number>(0);
  const startTimeRef = useRef<number>(performance.now());
  const topDegreeSetRef = useRef<Set<string>>(new Set());
  const [hoveredNode, setHoveredNode] = useState<TopologyNode | null>(null);

  // Track previous node ID set for incremental update detection
  const prevNodeIdsRef = useRef<Set<string>>(new Set());

  // Precomputed label set (top-N by degree + traversal + focus)
  const labelSetRef = useRef<Set<string>>(new Set());

  // ---------------------------------------------------------------------------
  // Draw frame (time-driven for pulse animation)
  // ---------------------------------------------------------------------------
  const drawFrame = useCallback((timestamp?: number) => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth;
    const h = canvas.clientHeight;

    // Resize backing store if needed
    if (canvas.width !== w * dpr || canvas.height !== h * dpr) {
      canvas.width = w * dpr;
      canvas.height = h * dpr;
    }

    const nodes = nodesRef.current;
    const links = linksRef.current;
    const t = transformRef.current;

    // Time for pulse animation (seconds)
    const elapsed = ((timestamp ?? performance.now()) - startTimeRef.current) / 1000;

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, w, h);

    // ── Background: deep navy, not pure black ──────────────────
    ctx.fillStyle = BG_COLOR;
    ctx.fillRect(0, 0, w, h);

    // Apply zoom transform
    ctx.save();
    ctx.translate(t.x, t.y);
    ctx.scale(t.k, t.k);

    // ── Edges (only when zoomed in enough or graph is small) ───
    const showEdges = t.k > 0.3 || nodes.length < 2000;

    if (showEdges && links.length > 0) {
      const negColor = parseHexAlpha(T.fHigh, 0.35);
      const defColor = parseHexAlpha(T.textMuted, 0.15);

      for (const link of links) {
        const src = link.source as TopologyNode;
        const tgt = link.target as TopologyNode;
        if (src.x == null || tgt.x == null) continue;

        ctx.beginPath();
        ctx.moveTo(src.x, src.y!);
        ctx.lineTo(tgt.x, tgt.y!);

        if (link.type === "negation") {
          // negation edge: red dashed with pulse opacity
          const pulseAlpha = 0.25 + 0.25 * Math.sin(elapsed * 3.0);
          ctx.strokeStyle = `rgba(255,68,102,${pulseAlpha})`;
          ctx.lineWidth = 1.4 / t.k;
          ctx.setLineDash([4 / t.k, 4 / t.k]);
        } else {
          ctx.strokeStyle = defColor;
          ctx.lineWidth = 0.4 / t.k;
          ctx.setLineDash([]);
        }
        ctx.stroke();
      }
      ctx.setLineDash([]);
    }

    // ── Settled rings (blue glow halo) ─────────────────────────
    for (const n of nodes) {
      if (!n.settled || n.x == null) continue;
      const r = nodeRadius(n, focusConcept);

      // Outer glow using radialGradient
      const ringR = r + 10 / t.k;
      const glow = ctx.createRadialGradient(n.x, n.y!, r, n.x, n.y!, ringR + 4 / t.k);
      glow.addColorStop(0, "rgba(68,136,255,0.5)");
      glow.addColorStop(1, "rgba(68,136,255,0)");
      ctx.fillStyle = glow;
      ctx.beginPath();
      ctx.arc(n.x, n.y!, ringR + 4 / t.k, 0, Math.PI * 2);
      ctx.fill();

      // Crisp ring stroke
      ctx.strokeStyle = T.settled;
      ctx.lineWidth = 1.0 / t.k;
      ctx.globalAlpha = 0.7;
      ctx.beginPath();
      ctx.arc(n.x, n.y!, ringR, 0, Math.PI * 2);
      ctx.stroke();
      ctx.globalAlpha = 1.0;
    }

    // ── High-degree node glow (top 20) ─────────────────────────
    for (const n of nodes) {
      if (!topDegreeSetRef.current.has(n.id) || n.x == null || n.isTraversal) continue;
      const r = nodeRadius(n, focusConcept);
      const glowR = r + 12 / t.k;
      const col = fToColor(n.f_avg);

      const grad = ctx.createRadialGradient(n.x, n.y!, r * 0.5, n.x, n.y!, glowR);
      grad.addColorStop(0, col + "66");
      grad.addColorStop(1, col + "00");

      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.arc(n.x, n.y!, glowR, 0, Math.PI * 2);
      ctx.fill();
    }

    // ── Nodes ──────────────────────────────────────────────────
    for (const n of nodes) {
      if (n.x == null) continue;
      const r = nodeRadius(n, focusConcept);
      ctx.beginPath();
      ctx.arc(n.x, n.y!, r, 0, Math.PI * 2);

      if (n.isTraversal) {
        ctx.fillStyle = T.traversalPulse;
        ctx.globalAlpha = 1.0;
      } else {
        ctx.fillStyle = fToColor(n.f_avg);
        ctx.globalAlpha = 0.7 + Math.min(n.degree / 12, 0.3);
      }
      ctx.fill();
    }
    ctx.globalAlpha = 1.0;

    // ── Traversal marker: white dot + expanding pulse rings ────
    for (const n of nodes) {
      if (!n.isTraversal || n.x == null) continue;

      // Pulse phase: two rings expanding outward, offset by half period
      for (let ring = 0; ring < 2; ring++) {
        const phase = (elapsed * 0.7 + ring * 0.5) % 1.0;  // 0..1
        const pulseR = (8 + phase * 22) / t.k;
        const alpha = (1 - phase) * 0.5;

        ctx.strokeStyle = `rgba(255,255,255,${alpha})`;
        ctx.lineWidth = (1.5 * (1 - phase)) / t.k;
        ctx.beginPath();
        ctx.arc(n.x, n.y!, pulseR, 0, Math.PI * 2);
        ctx.stroke();
      }

      // Static glow aura
      const auraR = 18 / t.k;
      const grad = ctx.createRadialGradient(n.x, n.y!, 2 / t.k, n.x, n.y!, auraR);
      grad.addColorStop(0, "rgba(255,255,255,0.5)");
      grad.addColorStop(0.4, "rgba(255,255,255,0.2)");
      grad.addColorStop(1, "rgba(255,255,255,0)");
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.arc(n.x, n.y!, auraR, 0, Math.PI * 2);
      ctx.fill();

      // Bright white center dot
      const r = nodeRadius(n, focusConcept);
      ctx.fillStyle = "#ffffff";
      ctx.globalAlpha = 1.0;
      ctx.beginPath();
      ctx.arc(n.x, n.y!, r, 0, Math.PI * 2);
      ctx.fill();

      // Small inner core — pure white highlight
      ctx.fillStyle = "#ffffff";
      ctx.globalAlpha = 1.0;
      ctx.beginPath();
      ctx.arc(n.x, n.y!, r * 0.4, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.globalAlpha = 1.0;

    // ── Labels (only top-N + traversal + focus) ────────────────
    if (t.k > 0.5) {
      ctx.font = `${8 / t.k}px ${FONT.mono}`;
      ctx.textAlign = "center";
      ctx.textBaseline = "bottom";

      for (const n of nodes) {
        if (n.x == null) continue;
        if (!labelSetRef.current.has(n.id)) continue;

        let lbl = n.label;
        if (lbl.startsWith("syn_") || lbl.startsWith("anti_")) {
          lbl = lbl.slice(0, 12) + "...";
        } else if (lbl.length > 24) {
          lbl = lbl.slice(0, 22) + "...";
        }

        const yOff = -(nodeRadius(n, focusConcept) + 4 / t.k);

        // Background pill
        const metrics = ctx.measureText(lbl);
        const tw = metrics.width;
        const th = 8 / t.k;
        ctx.fillStyle = BG_COLOR + "dd";
        ctx.fillRect(n.x - tw / 2 - 2 / t.k, n.y! + yOff - th, tw + 4 / t.k, th + 2 / t.k);

        // Text
        ctx.fillStyle = n.isTraversal ? "#ffffff" : T.textDim;
        ctx.fillText(lbl, n.x, n.y! + yOff);
      }
    }

    ctx.restore();

    // ── Canvas legend (top-right, fixed screen space) ──────────
    drawLegend(ctx, w, h);
  }, [focusConcept]);

  // ---------------------------------------------------------------------------
  // Canvas legend (drawn in screen space, outside transform)
  // ---------------------------------------------------------------------------
  function drawLegend(ctx: CanvasRenderingContext2D, w: number, h: number) {
    const items = [
      { color: T.fZero,         symbol: "circle", label: "f=0" },
      { color: T.fLow,          symbol: "circle", label: "fold" },
      { color: T.fMid,          symbol: "circle", label: "gray" },
      { color: T.fHigh,         symbol: "circle", label: "negate" },
      { color: T.settled,       symbol: "ring",   label: "settled" },
      { color: T.traversalPulse, symbol: "glow",  label: "here" },
    ];

    const PAD = 10;
    const ROW = 18;
    const ITEM_W = 68;
    const boxW = ITEM_W * 3 + PAD * 2;
    const boxH = ROW * 2 + PAD * 2 + 4;
    const bx = w - boxW - PAD;
    const by = PAD;

    // Background
    ctx.fillStyle = "rgba(10,10,26,0.88)";
    ctx.strokeStyle = "rgba(255,255,255,0.12)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.roundRect(bx, by, boxW, boxH, 6);
    ctx.fill();
    ctx.stroke();

    ctx.font = `9px ${FONT.mono}`;
    ctx.textAlign = "left";
    ctx.textBaseline = "middle";

    for (let i = 0; i < items.length; i++) {
      const col = i % 3;
      const row = Math.floor(i / 3);
      const x = bx + PAD + col * ITEM_W;
      const y = by + PAD + row * ROW + ROW / 2;

      const { color, symbol } = items[i];

      // Symbol
      if (symbol === "ring") {
        ctx.strokeStyle = color;
        ctx.lineWidth = 1.5;
        ctx.globalAlpha = 0.9;
        ctx.beginPath();
        ctx.arc(x + 5, y, 5, 0, Math.PI * 2);
        ctx.stroke();
        ctx.globalAlpha = 1.0;
      } else if (symbol === "glow") {
        const grad = ctx.createRadialGradient(x + 5, y, 1, x + 5, y, 7);
        grad.addColorStop(0, "rgba(255,255,255,0.9)");
        grad.addColorStop(1, "rgba(255,255,255,0)");
        ctx.fillStyle = grad;
        ctx.beginPath();
        ctx.arc(x + 5, y, 7, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = "#ffffff";
        ctx.beginPath();
        ctx.arc(x + 5, y, 2.5, 0, Math.PI * 2);
        ctx.fill();
      } else {
        ctx.fillStyle = color;
        ctx.globalAlpha = 0.9;
        ctx.beginPath();
        ctx.arc(x + 5, y, 4, 0, Math.PI * 2);
        ctx.fill();
        ctx.globalAlpha = 1.0;
      }

      ctx.fillStyle = "rgba(180,180,210,0.8)";
      ctx.fillText(items[i].label, x + 14, y);
    }
  }

  // ---------------------------------------------------------------------------
  // Animation loop (requestAnimationFrame — time-driven for traversal pulse)
  // ---------------------------------------------------------------------------
  const animate = useCallback(() => {
    const ts = performance.now();
    drawFrame(ts);
    rafRef.current = requestAnimationFrame(animate);
  }, [drawFrame]);

  // ---------------------------------------------------------------------------
  // Rebuild layout: runs full d3 simulation + resets zoom.
  // Called only when the node ID set changes.
  // ---------------------------------------------------------------------------
  const rebuildLayout = useCallback((
    canvas: HTMLCanvasElement,
    incomingData: TopologyResponse,
    currentFocusConcept: string | null | undefined,
    currentTraversalPosition: string | undefined,
  ) => {
    const width = canvas.clientWidth || 800;
    const height = canvas.clientHeight || 600;

    // Mark traversal node
    const nodes: TopologyNode[] = incomingData.nodes.map((n) => ({
      ...n,
      isTraversal: n.id === currentTraversalPosition,
    }));

    // Deep-copy links (d3 mutates source/target from string to object)
    let links: TopologyLink[] = incomingData.links.map((l) => ({
      source: typeof l.source === "string" ? l.source : (l.source as TopologyNode).id,
      target: typeof l.target === "string" ? l.target : (l.target as TopologyNode).id,
      type: l.type,
      f: l.f,
    }));

    nodesRef.current = nodes;

    // Precompute label set
    const degreeSorted = [...nodes].sort((a, b) => b.degree - a.degree);
    const topN = new Set(degreeSorted.slice(0, 15).map((n) => n.id));
    for (const n of nodes) {
      if (n.isTraversal || n.label === currentFocusConcept) topN.add(n.id);
    }
    labelSetRef.current = topN;

    // Precompute top-degree set for glow
    topDegreeSetRef.current = buildTopDegreeSet(nodes, 20);

    // ── Layout: clustered scatter (no force simulation for large graphs) ──
    const useForceLayout = nodes.length < 2000;

    if (useForceLayout) {
      if (links.length === 0) {
        links = generateSyntheticEdges(nodes, Math.min(nodes.length * 3, 4000));
        links = links.map((e) => ({
          source: typeof e.source === "string" ? e.source : (e.source as TopologyNode).id,
          target: typeof e.target === "string" ? e.target : (e.target as TopologyNode).id,
          type: e.type,
        }));
      }

      const sim = d3.forceSimulation<TopologyNode, TopologyLink>(nodes)
        .force(
          "link",
          d3.forceLink<TopologyNode, TopologyLink>(links)
            .id((d) => d.id)
            .distance(60)
            .strength(0.3),
        )
        .force("charge", d3.forceManyBody().strength(-120).distanceMax(300))
        .force("center", d3.forceCenter(width / 2, height / 2).strength(0.05))
        .force("collision", d3.forceCollide(8))
        .stop();

      const iterations = Math.min(300, Math.max(100, Math.ceil(1200 / Math.sqrt(nodes.length))));
      for (let i = 0; i < iterations; i++) {
        sim.tick();
      }
      linksRef.current = links;
    } else {
      const clusters: Record<string, TopologyNode[]> = {};
      for (const n of nodes) {
        const key = n.type || "other";
        if (!clusters[key]) clusters[key] = [];
        clusters[key].push(n);
      }

      const clusterKeys = Object.keys(clusters);
      const cx = width / 2;
      const cy = height / 2;
      const clusterRadius = Math.min(width, height) * 0.35;

      clusterKeys.forEach((key, ci) => {
        const angle = (ci / clusterKeys.length) * Math.PI * 2;
        const centerX = cx + Math.cos(angle) * clusterRadius;
        const centerY = cy + Math.sin(angle) * clusterRadius;
        const members = clusters[key];
        members.sort((a, b) => b.degree - a.degree);

        members.forEach((n, ni) => {
          const tParam = ni / Math.max(members.length - 1, 1);
          const r = tParam * clusterRadius * 0.6;
          const a2 = ni * 2.399963; // golden angle
          n.x = centerX + Math.cos(a2) * r;
          n.y = centerY + Math.sin(a2) * r;
        });
      });

      if (links.length === 0) {
        links = generateSyntheticEdges(nodes, 8000);
      } else {
        const nodeById = new Map(nodes.map((n) => [n.id, n]));
        for (const e of links) {
          if (typeof e.source === "string") {
            const nd = nodeById.get(e.source);
            if (nd) e.source = nd;
          }
          if (typeof e.target === "string") {
            const nd = nodeById.get(e.target);
            if (nd) e.target = nd;
          }
        }
      }
      linksRef.current = links;
    }

    // Build spatial grid for hit testing
    gridRef.current = buildSpatialGrid(nodes, 40);

    // Reset animation clock so pulse starts from 0 on new data
    startTimeRef.current = performance.now();

    // ── Zoom via d3-zoom on canvas ──────────────────────────
    const d3Canvas = d3.select<HTMLCanvasElement, unknown>(canvas);

    const zoom = d3.zoom<HTMLCanvasElement, unknown>()
      .scaleExtent([0.05, 12])
      .on("zoom", (event) => {
        transformRef.current = event.transform;
      });

    d3Canvas.call(zoom);

    // Set initial zoom to fit
    const initialScale = Math.min(
      width / (width * 1.2),
      height / (height * 1.2),
    );
    d3Canvas.call(zoom.transform, d3.zoomIdentity.translate(0, 0).scale(initialScale));

    return () => {
      d3Canvas.on(".zoom", null);
    };
  }, []);

  // ---------------------------------------------------------------------------
  // Attribute-only update: node properties changed but topology is the same.
  // Updates degree, settled, f_avg, isTraversal in-place on existing node objects.
  // Does NOT touch x/y coordinates or zoom transform.
  // ---------------------------------------------------------------------------
  const updateNodeAttributes = useCallback((
    incomingData: TopologyResponse,
    currentTraversalPosition: string | undefined,
    currentFocusConcept: string | null | undefined,
  ) => {
    const nodes = nodesRef.current;
    if (nodes.length === 0) return;

    // Build lookup map from incoming data
    const incoming = new Map(incomingData.nodes.map((n) => [n.id, n]));

    let labelChanged = false;
    for (const n of nodes) {
      const src = incoming.get(n.id);
      if (!src) continue;

      n.degree = src.degree;
      n.settled = src.settled;
      n.f_avg = src.f_avg;

      const shouldBeTraversal = n.id === currentTraversalPosition;
      if (n.isTraversal !== shouldBeTraversal) {
        n.isTraversal = shouldBeTraversal;
        labelChanged = true;
      }
    }

    // Recompute top-degree set (degrees may have changed)
    topDegreeSetRef.current = buildTopDegreeSet(nodes, 20);

    // Update label set: add new traversal node if needed
    if (labelChanged && currentTraversalPosition) {
      labelSetRef.current = new Set([...labelSetRef.current, currentTraversalPosition]);
    }

    // Rebuild spatial grid is not needed (coordinates unchanged)
    // The draw loop will pick up the mutated attributes on the next frame.

    // Re-add focus concept to label set if it's missing
    if (currentFocusConcept) {
      const focusNode = nodes.find((n) => n.label === currentFocusConcept);
      if (focusNode) {
        labelSetRef.current = new Set([...labelSetRef.current, focusNode.id]);
      }
    }
  }, []);

  // ---------------------------------------------------------------------------
  // Main data effect: decides between full rebuild and attribute-only update.
  // ---------------------------------------------------------------------------
  useEffect(() => {
    if (!canvasRef.current || !data) return;

    const canvas = canvasRef.current;
    const newIds = new Set(data.nodes.map((n) => n.id));
    const prevIds = prevNodeIdsRef.current;

    const sameNodes =
      newIds.size === prevIds.size &&
      [...newIds].every((id) => prevIds.has(id));

    if (!sameNodes) {
      // Node topology changed — full rebuild with zoom reset
      prevNodeIdsRef.current = newIds;
      return rebuildLayout(canvas, data, focusConcept, traversalPosition);
    } else {
      // Same nodes — only update attributes, preserve zoom/pan
      updateNodeAttributes(data, traversalPosition, focusConcept);
    }
  }, [data, focusConcept, traversalPosition, rebuildLayout, updateNodeAttributes]);

  // ---------------------------------------------------------------------------
  // Traversal position update — when only traversalPosition prop changes
  // without a data update, sync isTraversal flags on existing nodes.
  // (This handles the case where App updates traversalPosition independently.)
  // ---------------------------------------------------------------------------
  useEffect(() => {
    const nodes = nodesRef.current;
    if (nodes.length === 0) return;

    let changed = false;
    for (const n of nodes) {
      const shouldBe = n.id === traversalPosition;
      if (n.isTraversal !== shouldBe) {
        n.isTraversal = shouldBe;
        changed = true;
      }
    }

    // Also update labelSet to include new traversal node
    if (changed && traversalPosition) {
      const traversalNode = nodes.find((n) => n.id === traversalPosition);
      if (traversalNode) {
        labelSetRef.current = new Set([...labelSetRef.current, traversalPosition]);
      }
    }
  }, [traversalPosition]);

  // ── Start/stop animation loop ────────────────────────────────
  useEffect(() => {
    if (!data) return;
    rafRef.current = requestAnimationFrame(animate);
    return () => {
      cancelAnimationFrame(rafRef.current);
    };
  }, [data, animate]);

  // ---------------------------------------------------------------------------
  // Mouse interaction: hover hit test + click
  // ---------------------------------------------------------------------------
  const handleMouseMove = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas || !gridRef.current) return;

    const rect = canvas.getBoundingClientRect();
    const screenX = e.clientX - rect.left;
    const screenY = e.clientY - rect.top;

    // Invert zoom transform to get world coordinates
    const t = transformRef.current;
    const worldX = (screenX - t.x) / t.k;
    const worldY = (screenY - t.y) / t.k;

    const hit = findNearest(gridRef.current, nodesRef.current, worldX, worldY, 20 / t.k);
    setHoveredNode(hit);
    canvas.style.cursor = hit ? "pointer" : "default";
  }, []);

  const handleClick = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas || !gridRef.current) return;

    const rect = canvas.getBoundingClientRect();
    const screenX = e.clientX - rect.left;
    const screenY = e.clientY - rect.top;

    const t = transformRef.current;
    const worldX = (screenX - t.x) / t.k;
    const worldY = (screenY - t.y) / t.k;

    const hit = findNearest(gridRef.current, nodesRef.current, worldX, worldY, 20 / t.k);
    if (hit) onSelectNode?.(hit);
  }, [onSelectNode]);

  return (
    <div ref={overlayRef} style={{ position: "relative", width: "100%", height: "100%" }}>
      <canvas
        ref={canvasRef}
        style={{ width: "100%", height: "100%", display: "block", background: BG_COLOR }}
        onMouseMove={handleMouseMove}
        onClick={handleClick}
      />

      {/* Hover tooltip */}
      {hoveredNode && (
        <div style={{
          position: "absolute", bottom: 12, left: 12,
          background: "rgba(10,10,26,0.92)",
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
          {hoveredNode.isTraversal && (
            <span style={{ color: "#ffffff", marginLeft: 8, fontWeight: 700 }}>HERE</span>
          )}
        </div>
      )}

      {/* Meta info */}
      {data?.meta && (
        <div style={{
          position: "absolute", bottom: 12, right: 12,
          background: "rgba(10,10,26,0.88)",
          border: `1px solid ${T.border}`,
          borderRadius: 4, padding: "4px 8px",
          fontFamily: FONT.mono, fontSize: 9, color: T.textMuted,
          pointerEvents: "none",
        }}>
          {data.meta.shown_vertices.toLocaleString()}V /&nbsp;
          {data.meta.total_vertices.toLocaleString()} total
          {linksRef.current.length > 0 && (
            <span style={{ marginLeft: 8 }}>
              {linksRef.current.length.toLocaleString()}E
              {data.meta.shown_edges === 0 && " (synth)"}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
