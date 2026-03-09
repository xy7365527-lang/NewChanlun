"""Daemon API — pure functions that convert daemon state to JSON responses.

No HTTP framework dependency. Each function takes a TopologicalDaemon
and returns a JSON-serializable dict/list.
"""

from __future__ import annotations

import json as _json
import re
import time
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from daemon import TopologicalDaemon

from engine import compute_beta_1, Edge, EdgeType, Vertex


from psi_L_constraint import ConstraintSet
from llm_integration import get_client as _get_llm_client, GenerationRecord, parse_signifier_chain
from signifier_net import detect_ruptures, persist_rupture_log, writeback_from_text
from internal_speech import externalize as _externalize_speech

def status_json(daemon: TopologicalDaemon) -> dict:
    """GET /status — current daemon state snapshot."""
    s = daemon.status()
    # Compute encounter density from recent logs
    if daemon.engine and daemon.engine.logs:
        recent = daemon.engine.logs[-100:]
        encounters = sum(1 for log in recent if log.operation != "walk")
        density = encounters / len(recent) if recent else 0.0
    else:
        density = 0.0

    # Determine status label
    if hasattr(daemon, '_crystallization_count') and daemon._crystallization_count > 0:
        if daemon.engine and daemon.engine.logs:
            last = daemon.engine.logs[-1]
            if last.operation == "walk":
                label = "traversing"
            else:
                label = "traversing"
        else:
            label = "crystallized"
    else:
        label = "traversing"

    # Check if feeding
    if daemon.total_feeds > 0 and daemon.engine and daemon.engine.logs:
        last_step = daemon.engine.logs[-1].step if daemon.engine.logs else 0
        if hasattr(daemon, '_last_feed_step') and last_step - daemon._last_feed_step < 5:
            label = "feeding"

    # Current traversal position
    position = ""
    position_label = ""
    if daemon.engine and daemon.engine.position:
        position = daemon.engine.position
        if daemon.concept_names:
            position_label = daemon.concept_names.get(position, "")
        if not position_label:
            v = daemon.k_active.vertex(position)
            position_label = (v.content or position)[:60] if v else position

    return {
        "beta_1": s["beta_1"],
        "vertices": s["vertices_active"],
        "edges": s["edges_active"],
        "settled": s["settled_cycles"],
        "steps": s["total_steps"],
        "crystallization_count": s.get("crystallization_count", 0),
        "encounter_density": round(density, 3),
        "status": label,
        "expression_pressure": getattr(daemon, '_unreported_count', 0),
        "residue_vertices": len(daemon.settlement.residue_vertices()) if daemon.settlement else 0,
        "position": position,
        "position_label": position_label,
    }


def topology_json(daemon: TopologicalDaemon, center: str | None = None, radius: int = 2) -> dict:
    """GET /topology — K_active subgraph snapshot with f-values.

    If center is given, returns local subgraph within radius hops.
    Otherwise returns top-100 highest-degree vertices as skeleton.
    """
    graph = daemon.k_active
    active = graph.active_vertex_ids()
    total_v = len(active)
    total_e = len(graph.active_edges())

    if center and center in set(active):
        # Local subgraph
        vids, edges = graph.local_subgraph(center, radius)
    else:
        # Full graph — all active vertices and edges
        vids = active
        edges = graph.active_edges()

    # Build f-value map for vertices
    vid_set = set(vids)
    degrees = {v: len(graph.neighbors(v)) for v in vids}

    # Compute average f for each vertex from terrain
    f_avgs: dict[str, float] = {}
    for vid in vids:
        neighbors = graph.neighbors(vid)
        if not neighbors:
            f_avgs[vid] = -1.0
            continue
        f_vals = []
        for nb in neighbors:
            key = (vid, nb)
            mark = daemon.terrain.get(key, "critical")
            # f-value approximation from terrain mark
            if mark == "tree":
                f_vals.append(0.0)
            else:
                f_vals.append(5.0)  # critical = higher f
        f_avgs[vid] = sum(f_vals) / len(f_vals) if f_vals else -1.0

    # Settled vertices
    settled_vids: set[str] = set()
    if daemon.settlement:
        for sc in daemon.settlement.settled_cycles:
            for src, tgt in sc.edges:
                settled_vids.add(src)
                settled_vids.add(tgt)

    # Classify vertex type
    def _vertex_type(vid: str) -> str:
        if "<import>" in vid or ":" in vid:
            return "code"
        v = graph.vertex(vid)
        if v and v.content and any(c > '\u4e00' for c in (v.content or "")[:20]):
            return "text"
        return "system"

    nodes = []
    for vid in vids:
        v = graph.vertex(vid)
        label = (v.content or vid)[:80] if v else vid
        nodes.append({
            "id": vid,
            "label": label,
            "f_avg": round(f_avgs.get(vid, -1.0), 2),
            "degree": degrees.get(vid, 0),
            "settled": vid in settled_vids,
            "type": _vertex_type(vid),
        })

    links = []
    for e in edges:
        if e.source in vid_set and e.target in vid_set:
            links.append({
                "source": e.source,
                "target": e.target,
                "type": e.edge_type.value,
            })

    return {
        "nodes": nodes,
        "links": links,
        "meta": {
            "total_vertices": total_v,
            "total_edges": total_e,
            "shown_vertices": len(nodes),
            "shown_edges": len(links),
        },
    }


def query_json(daemon: TopologicalDaemon, concept: str) -> dict:
    """GET /query?concept=X — concept detail with f-terrain and neighbors.

    Uses query_interface._compute_f for proper f(v,w) computation and
    _shortest_path for topological distance.
    """
    from query_interface import _compute_f, _shortest_path

    # Search registry for concept
    matches = daemon.registry.search(concept, limit=1) if hasattr(daemon.registry, 'search') else []

    # Fallback: search by vertex content substring
    vid = None
    if matches:
        vid = matches[0]
    else:
        for v_id in daemon.k_active.active_vertex_ids():
            v = daemon.k_active.vertex(v_id)
            if v and v.content and concept.lower() in v.content.lower():
                vid = v_id
                break

    if not vid:
        return {"found": False, "vertex": None, "neighbors": [], "f_terrain": {}, "narrative": ""}

    v = daemon.k_active.vertex(vid)
    neighbors = daemon.k_active.neighbors(vid)
    degree = len(neighbors)

    # Settled check
    settled_vids: set[str] = set()
    rings = 0
    settled_rings = 0
    if daemon.settlement:
        for sc in daemon.settlement.settled_cycles:
            edge_vids = {src for src, _ in sc.edges} | {tgt for _, tgt in sc.edges}
            if vid in edge_vids:
                rings += 1
                settled_rings += 1
            settled_vids.update(edge_vids)

    # Neighbor details with proper f computation
    neighbor_list = []
    fold_zone = []
    gray_zone = []
    negate_zone = []

    for nb in neighbors:
        nb_v = daemon.k_active.vertex(nb)
        nb_label = (nb_v.content or nb)[:60] if nb_v else nb

        # Edge type
        edge_type = "dependency"
        for e in daemon.k_active.active_edges():
            if (e.source == vid and e.target == nb) or (e.source == nb and e.target == vid):
                edge_type = e.edge_type.value
                break

        # Proper f(v,w) computation
        f_val = _compute_f(daemon.k_active, vid, nb)

        neighbor_list.append({
            "id": nb,
            "label": nb_label,
            "f": f_val,
            "edge_type": edge_type,
        })

        if f_val < 5:
            fold_zone.append(nb_label)
        elif f_val < 12:
            gray_zone.append(nb_label)
        else:
            negate_zone.append(nb_label)

    return {
        "found": True,
        "vertex": {
            "id": vid,
            "label": (v.content or vid)[:80] if v else vid,
            "f_avg": round(sum(n["f"] for n in neighbor_list) / max(len(neighbor_list), 1), 1),
            "degree": degree,
            "settled": vid in settled_vids,
            "rings": rings,
            "settled_rings": settled_rings,
        },
        "neighbors": neighbor_list[:20],
        "f_terrain": {
            "fold_zone": fold_zone[:10],
            "gray_zone": gray_zone[:10],
            "negate_zone": negate_zone[:10],
        },
        "narrative": "",
    }


def narrative_json(daemon: TopologicalDaemon, n: int = 20) -> list[dict]:
    """GET /narrative?n=20 — recent N events as narrative.

    Uses psi_L_importance for proper I(e_i) computation and
    psi_L_topological for Morse-Smale structural annotations.
    """
    from psi_L_importance import compute_importance

    events = []
    logs = daemon.engine.logs if daemon.engine else []
    significant = [log for log in logs if log.delta_beta_1 != 0 or log.blocked]

    # Build step_logs dicts for importance computation
    step_log_dicts = []
    for log in significant:
        step_log_dicts.append({
            "step": log.step,
            "position": log.position,
            "operation": log.operation,
            "beta_1_before": log.beta_1_before,
            "beta_1_after": log.beta_1_after,
            "delta_beta_1": log.delta_beta_1,
            "blocked": log.blocked,
            "settled_count": log.settled_count,
            "f_value": log.f_value,
        })

    # Compute importance scores
    importance_scores = compute_importance(step_log_dicts) if step_log_dicts else []

    for idx, log in enumerate(significant[-n:]):
        # Map idx to the correct importance score position
        score_idx = len(significant) - n + idx if len(significant) > n else idx
        if 0 <= score_idx < len(importance_scores):
            importance = importance_scores[score_idx]
        else:
            importance = min(1.0, abs(log.delta_beta_1) * 0.3 + (0.5 if log.blocked else 0.0))

        # Normalize to 0-1 range for API (cap at 10 as practical max)
        importance_normalized = min(1.0, importance / 10.0) if importance > 0 else 0.0
        level = 3 if importance_normalized > 0.6 else (2 if importance_normalized > 0.3 else 1)

        events.append({
            "time": "",
            "level": level,
            "text": f"[step {log.step}] {log.operation} at {log.position} "
                    f"beta_1 {log.beta_1_before}->{log.beta_1_after} "
                    f"({'BLOCKED' if log.blocked else f'delta={log.delta_beta_1}'})",
            "importance": round(importance_normalized, 2),
            "importance_raw": round(importance, 2),
            "type": log.operation,
        })
    return events


def persistence_json(daemon: TopologicalDaemon) -> dict:
    """GET /persistence — persistent homology features from settlement history.

    Each settled cycle = a persistent H1 feature (birth=first_seen, death=∞).
    Each pending cycle = a feature still being tracked (birth=first_seen, death=current_step).
    Beta_1 history provides the filtration curve.
    """
    pairs: list[dict] = []

    # Settled cycles = persistent features (death = infinity)
    if daemon.settlement:
        for sc in daemon.settlement.settled_cycles:
            edges_list = sorted(sc.edges)
            pair_id = f"settled_{sc.settled_at_step}"
            pairs.append({
                "id": pair_id,
                "dimension": 1,
                "birth": sc.settled_at_step - daemon.settlement.threshold,
                "death": float("inf"),
                "lifetime": float("inf"),
                "settled": True,
                "edges": len(edges_list),
            })

        # Pending cycles = features still being tracked
        for cycle_edges, first_seen in daemon.settlement._pending.items():
            pairs.append({
                "id": f"pending_{first_seen}",
                "dimension": 1,
                "birth": first_seen,
                "death": daemon.total_steps,
                "lifetime": daemon.total_steps - first_seen,
                "settled": False,
                "edges": len(cycle_edges),
            })

    # Sort by birth time
    pairs.sort(key=lambda p: p["birth"])

    # Beta_1 curve as filtration
    beta1_curve = []
    if daemon.engine and daemon.engine.logs:
        for log in daemon.engine.logs:
            if log.delta_beta_1 != 0:
                beta1_curve.append({"step": log.step, "beta_1": log.beta_1_after})

    return {
        "pairs": pairs,
        "total_beta_1": compute_beta_1(daemon.k_active),
        "settled_count": len(daemon.settlement.settled_cycles) if daemon.settlement else 0,
        "pending_count": len(daemon.settlement._pending) if daemon.settlement else 0,
        "beta1_curve": beta1_curve[-100:],  # Last 100 changes
    }


def gaps_json(daemon: TopologicalDaemon) -> list[dict]:
    """GET /gaps — current gap candidates."""
    daemon._gap_cooldown.clear()
    gaps = daemon._detect_gaps()
    return [
        {
            "concept": g.content[:80],
            "degree": g.degree,
            "avg_neighbor_degree": round(g.avg_degree, 1),
            "status": "queued",
        }
        for g in gaps
    ]


def operations_json(daemon: TopologicalDaemon) -> dict:
    """GET /operations — operation statistics."""
    counts: dict[str, dict[str, int]] = {
        "fold": {"count": 0, "blocked": 0},
        "negate": {"count": 0, "blocked": 0},
        "sublate": {"count": 0, "blocked": 0},
        "walk": {"count": 0, "blocked": 0},
    }
    if daemon.engine:
        for log in daemon.engine.logs:
            op = log.operation
            if op.endswith("_blocked"):
                base = op.replace("_blocked", "")
                if base in counts:
                    counts[base]["blocked"] += 1
                elif base.startswith("negate"):
                    counts["negate"]["blocked"] += 1
            else:
                # Normalize operation names
                if op.startswith("negate"):
                    counts["negate"]["count"] += 1
                elif op in counts:
                    counts[op]["count"] += 1
                else:
                    counts["walk"]["count"] += 1
    return counts


def step_ws_message(log, daemon: TopologicalDaemon) -> dict:
    """Format a StepLog into a WebSocket push message."""
    label = ""
    if daemon.concept_names:
        label = daemon.concept_names.get(log.position, "")
    if not label:
        v = daemon.k_active.vertex(log.position)
        label = (v.content or log.position)[:60] if v else log.position

    return {
        "type": "step",
        "step": log.step,
        "position": log.position,
        "position_label": label,
        "beta_1": log.beta_1_after,
        "delta_beta_1": log.delta_beta_1,
        "operation": log.operation,
        "f_value": log.f_value,
        "crystallized": False,
    }


# ---------------------------------------------------------------------------
# Expression pressure tracking helpers
# ---------------------------------------------------------------------------

# Importance threshold above which an event is "high-I" and triggers expression pressure
_HIGH_I_THRESHOLD = 0.6


def _get_unreported_events(daemon: TopologicalDaemon) -> list[dict]:
    """Return high-importance events that have not yet been reported to the user.

    Uses daemon._reported_step_watermark to track which log entries have been
    reported. Returns up to 5 most recent unreported significant events.
    """
    if not daemon.engine or not daemon.engine.logs:
        return []

    watermark: int = getattr(daemon, '_reported_step_watermark', 0)
    logs = daemon.engine.logs
    significant = [
        log for log in logs
        if log.step > watermark and (log.delta_beta_1 != 0 or log.blocked)
    ]

    # Compute importance for each and filter high-I
    high_i = []
    for log in significant:
        importance = min(1.0, abs(log.delta_beta_1) * 0.3 + (0.5 if log.blocked else 0.0))
        if importance >= _HIGH_I_THRESHOLD:
            label = daemon.concept_names.get(log.position, log.position)
            high_i.append({
                "step": log.step,
                "operation": log.operation,
                "position_label": label,
                "position_id": log.position,
                "beta_1_before": log.beta_1_before,
                "beta_1_after": log.beta_1_after,
                "importance": round(importance, 2),
                "blocked": log.blocked,
            })

    return high_i[-5:]  # most recent 5


def _mark_reported(daemon: TopologicalDaemon) -> None:
    """Advance the reported watermark to the current latest step."""
    if daemon.engine and daemon.engine.logs:
        last_step = daemon.engine.logs[-1].step
        daemon._reported_step_watermark = last_step
        # Also advance WS watermark so WS doesn't re-push already-reported events
        daemon._ws_last_reported_step = max(
            getattr(daemon, '_ws_last_reported_step', 0), last_step
        )
    # Reset unreported counter
    daemon._unreported_count = 0


def _get_surface_for_event(daemon: TopologicalDaemon, position: str) -> str | None:
    """Look up a surface form from K_active edges incident on position.

    Searches edges where position is source or target and returns the first
    non-None surface field found. Used to enrich narrative text.
    """
    graph = daemon.k_active
    for e in graph.active_edges():
        if (e.source == position or e.target == position) and e.surface:
            return e.surface
    return None


def _collect_edge_surfaces(daemon: TopologicalDaemon, edge_type: EdgeType) -> list[str]:
    """Collect all surface forms from K_active edges of a given type."""
    surfaces: list[str] = []
    for e in daemon.k_active.active_edges():
        if e.edge_type == edge_type and e.surface:
            surfaces.append(e.surface)
    return surfaces


def _narrate_unreported(events: list[dict], daemon: TopologicalDaemon | None = None) -> str:
    """Turn unreported high-I events into sharing text, using surface forms when available."""
    if not events:
        return ""
    lines = []
    for ev in events:
        delta = ev["beta_1_after"] - ev["beta_1_before"]
        sign = f"+{delta}" if delta >= 0 else str(delta)
        blocked_note = " (阻断)" if ev["blocked"] else ""
        label = ev["position_label"]

        # Try to find a surface form for this position
        surface = None
        if daemon is not None:
            surface = _get_surface_for_event(daemon, ev.get("position_id", label))

        if surface:
            lines.append(
                f"[步{ev['step']}] '{label}' {surface} β₁{sign}{blocked_note}"
            )
        else:
            lines.append(
                f"[步{ev['step']}] {ev['operation']} at '{label}' β₁{sign}{blocked_note}"
            )
    return "发现: " + "; ".join(lines)


def _extract_concepts(text: str) -> list[str]:
    """Very lightweight concept extraction from user text.

    Splits on whitespace and punctuation, returns tokens >=2 chars.
    No NLP dependency — pure string ops.
    """
    # Note: ( ) do not need escaping inside character class; \[ \] are valid regex escapes
    tokens = re.split(r'[\s\u3000，。！？、；：\u201c\u201d\u2018\u2019【】《》()\[\]]+', text)
    return [t for t in tokens if len(t) >= 2]


def _locate_in_k_active(daemon: TopologicalDaemon, concepts: list[str]) -> str | None:
    """Find the best matching vertex for any of the user's concepts."""
    active = daemon.k_active.active_vertex_ids()
    for concept in concepts:
        low = concept.lower()
        # Exact registry search first
        if hasattr(daemon.registry, 'search'):
            matches = daemon.registry.search(concept, limit=1)
            if matches:
                return matches[0]
        # Substring fallback
        for vid in active:
            v = daemon.k_active.vertex(vid)
            if v and v.content and low in v.content.lower():
                return vid
    return None


def _ensure_source_vertex(daemon: TopologicalDaemon, session_id: str) -> str:
    """Create or reuse a source vertex representing the user session.

    The source vertex `source:{session_id}` anchors all concepts injected
    in this session. Its neighborhood = the user's concept region.

    Returns the vertex id.
    """
    vid = f"source:{session_id}"
    if daemon.k_active.vertex(vid) is None:
        from engine import Vertex, VertexStatus
        step = daemon.engine.step if daemon.engine else 0
        new_v = Vertex(id=vid, content=f"[来源:{session_id}]", created_at=step)
        daemon.k_active = daemon.k_active.add_vertex(new_v)
    return vid


def _link_concepts_to_source(
    daemon: TopologicalDaemon,
    concept_vids: list[str],
    source_vid: str,
) -> None:
    """Add REFERENCE edges from each concept vertex to the source vertex.

    These edges mark which concepts the user introduced in this session.
    Edges are added only if not already present.
    """
    existing = {
        (e.source, e.target)
        for e in daemon.k_active.active_edges()
        if e.edge_type == EdgeType.REFERENCE
    }
    step = daemon.engine.step if daemon.engine else 0
    for cvid in concept_vids:
        if (cvid, source_vid) not in existing and cvid != source_vid:
            new_edge = Edge(
                source=cvid,
                target=source_vid,
                edge_type=EdgeType.REFERENCE,
                created_at=step,
                surface="引入于",
                context=None,
            )
            daemon.k_active = daemon.k_active.add_edge(new_edge)


def _quick_traverse(daemon: TopologicalDaemon, start_vid: str, budget: int = 20) -> list[dict]:
    """Run up to `budget` steps from start_vid, collect significant events."""
    if not daemon.engine:
        return []

    original_position = daemon.engine.position
    daemon.engine.position = start_vid
    events = []

    for _ in range(budget):
        daemon._step()
        if daemon.engine.logs:
            log = daemon.engine.logs[-1]
            if log.delta_beta_1 != 0 or log.blocked:
                importance = min(1.0, abs(log.delta_beta_1) * 0.3 + (0.5 if log.blocked else 0.0))
                label = daemon.concept_names.get(log.position, log.position)
                events.append({
                    "step": log.step,
                    "operation": log.operation,
                    "position_label": label,
                    "position_id": log.position,
                    "beta_1_before": log.beta_1_before,
                    "beta_1_after": log.beta_1_after,
                    "importance": round(importance, 2),
                    "blocked": log.blocked,
                })

    # We do NOT restore: the co-gaze permanently redirects the daemon's attention
    return events


def _narrate_co_gaze(
    concept: str,
    events: list[dict],
    daemon: TopologicalDaemon | None = None,
) -> str:
    """Format co-gaze traversal events into sharing text, using surface forms."""
    if not events:
        return f"在 '{concept}' 附近穿越，区域已结晶，无新发现。"
    lines = []
    for ev in events[:3]:
        delta = ev["beta_1_after"] - ev["beta_1_before"]
        sign = f"+{delta}" if delta >= 0 else str(delta)
        blocked_note = " (阻断)" if ev["blocked"] else ""
        label = ev["position_label"]

        # Try surface form from edges incident on this position
        surface = None
        if daemon is not None:
            surface = _get_surface_for_event(daemon, ev.get("position_id", label))

        if surface:
            lines.append(f"[步{ev['step']}] '{label}' {surface} β₁{sign}{blocked_note}")
        else:
            lines.append(f"[步{ev['step']}] {ev['operation']} @ '{label}' β₁{sign}{blocked_note}")

    return f"共同注视 '{concept}' ({len(events)}个事件): " + "; ".join(lines)


# ---------------------------------------------------------------------------
# LLM 语言器官辅助：生成/润色 present_json 的 parts 文本
# ---------------------------------------------------------------------------

def _llm_generate_part(
    context_fragment: str,
    must_use: list[str],
    daemon,
    register: str = "plain",
) -> str:
    """尝试用 LLM 语言器官润色 context_fragment 为自然语言文本.

    如果 LLM 不可用（无 API key 或调用失败），返回原始 context_fragment（fallback）。
    daemon 用于读取当前 beta_1 和 settled_count 以填充 ConstraintSet。
    当 daemon.snet 可用时，从 S_net 查询 must_use 对应的 surface forms 传入约束。
    """
    try:
        from engine import compute_beta_1
        beta_1 = compute_beta_1(daemon.k_active) if daemon.k_active else None
        settled_count = len(daemon.settlement.settled_cycles) if daemon.settlement else None

        # Query S_net for available surface forms
        snet_surface_forms: list[str] = []
        snet = getattr(daemon, 'snet', None)
        if snet and snet.signifiers:
            for term in must_use:
                if snet.has_signifier(term):
                    for edge in snet.degree_normalized_neighbors(term, n=3, alpha=0.5):
                        if edge.evidence and edge.evidence not in snet_surface_forms:
                            snet_surface_forms.append(edge.evidence)
                        if len(snet_surface_forms) >= 6:
                            break
                if len(snet_surface_forms) >= 6:
                    break

        cs = ConstraintSet(
            must_use=must_use,
            must_avoid=[],
            register=register,
            narrative_spine=context_fragment,
            expression_pressure=must_use,
            surface_forms=snet_surface_forms,
        )
        client = _get_llm_client()
        record = client.generate(cs, provider="auto")
        # fallback 时 provider = "fallback"，返回原文
        if record.provider == "fallback":
            return context_fragment
        return record.llm_output.strip() or context_fragment
    except Exception:
        return context_fragment


# ---------------------------------------------------------------------------
# MVP 语言器官：command/dialogue 分类 + 自然语言生成
# ---------------------------------------------------------------------------

# Command patterns — regex patterns that indicate a command, not dialogue
_COMMAND_PATTERNS = [
    re.compile(r'^\s*$'),                          # empty
    re.compile(r'^/\w+'),                          # slash commands
    re.compile(r'^(status|feed|query|topology)\b', re.IGNORECASE),
    re.compile(r'^(start|stop|reset|pause|resume)\b', re.IGNORECASE),
    re.compile(r'^(show|list|get|set)\s', re.IGNORECASE),
]


def _classify_input(text: str) -> str:
    """Classify user input as 'command' or 'dialogue'.

    Returns 'command' if text matches any command pattern or is empty.
    Returns 'dialogue' otherwise.
    """
    if not text or not text.strip():
        return "command"
    for pat in _COMMAND_PATTERNS:
        if pat.search(text.strip()):
            return "command"
    return "dialogue"


def _build_mvp_constraint(daemon: TopologicalDaemon) -> dict:
    """Build minimal constraint dict from daemon's current state.

    This is the system prompt context for the language organ —
    not a full ConstraintSet, but a lightweight dict describing
    who fengliang is and what it's currently doing.
    """
    # Current position
    position_content = ""
    if daemon.engine and daemon.engine.position:
        v = daemon.k_active.vertex(daemon.engine.position)
        if v and v.content:
            position_content = v.content[:120]

    # Recent activity: narrative spine from psi_L_topological
    # Uses Tarjan SCC + Kahn topo sort to produce structured traversal summary
    # Falls back to simple log text if narrative generation fails
    recent_activity: list[str] = []
    try:
        from psi_L_topological import (
            linearize_complex, _find_sccs, _classify_cut, _vertex_label,
            _edge_type_label,
        )

        graph = daemon.k_active
        terrain = daemon.terrain
        names = daemon.concept_names

        def _name(vid: str) -> str:
            return names.get(vid, _vertex_label(graph, vid))

        vertex_order, critical_cuts, extra_cuts = linearize_complex(
            graph, terrain,
        )

        if vertex_order:
            # 1. Spine: topo-sorted vertex sequence (top 8)
            spine_labels = [_name(vid) for vid in vertex_order[:8]]
            recent_activity.append(
                f"叙事主线({len(vertex_order)}节点): "
                + " → ".join(spine_labels)
                + ("…" if len(vertex_order) > 8 else "")
            )

            # 2. Critical cuts with edge type annotations
            all_cuts = critical_cuts + extra_cuts
            if all_cuts:
                cut_descs: list[str] = []
                for src, tgt in all_cuts[:5]:
                    cut_type = _classify_cut(graph, src, tgt)
                    edge_type = _edge_type_label(graph, src, tgt)
                    cut_descs.append(
                        f"'{_name(src)}'--[{edge_type}]-->'{_name(tgt)}' ({cut_type})"
                    )
                recent_activity.append(
                    f"切断边({len(all_cuts)}): " + "; ".join(cut_descs)
                )

            # 3. SCC cycle structures from remaining tree edges
            adj: dict[str, list[str]] = {v: [] for v in vertex_order}
            remaining = [
                (e.source, e.target) for e in graph.active_edges()
                if terrain.get((e.source, e.target)) == "tree"
                and e.source in adj and e.target in adj
            ]
            for s, t in remaining:
                adj[s].append(t)
            sccs = _find_sccs(adj, vertex_order)
            multi_sccs = [scc for scc in sccs if len(scc) > 1]
            if multi_sccs:
                scc_descs: list[str] = []
                for scc in multi_sccs[:3]:
                    scc_names = [_name(v) for v in scc[:4]]
                    scc_descs.append(
                        f"环({len(scc)}节点: {', '.join(scc_names)}"
                        + ("…" if len(scc) > 4 else "")
                        + ")"
                    )
                recent_activity.append(
                    f"环结构({len(multi_sccs)}): " + "; ".join(scc_descs)
                )

            # 4. Narrative density
            density = len(all_cuts) / max(len(vertex_order), 1)
            tension = (
                "高辩证张力" if density > 0.5
                else "中等张力" if density > 0.2
                else "线性进展"
            )
            recent_activity.append(
                f"叙事密度: {density:.2f} ({tension})"
            )
    except Exception:
        pass

    # Fallback: if narrative spine generation failed or returned empty, use log text
    if not recent_activity:
        if daemon.engine and daemon.engine.logs:
            significant = [
                log for log in daemon.engine.logs[-50:]
                if log.delta_beta_1 != 0 or log.blocked
            ]
            for log in significant[-5:]:
                label = daemon.concept_names.get(log.position, log.position)
                delta = log.beta_1_after - log.beta_1_before
                sign = f"+{delta}" if delta >= 0 else str(delta)
                blocked = " (阻断)" if log.blocked else ""
                recent_activity.append(
                    f"{log.operation} @ '{label}' beta_1{sign}{blocked}"
                )

    # Expression pressure: unreported events count
    unreported_count = _count_unreported(daemon)

    # Pending proposals: code_settlement_requests awaiting operator approval
    pending_proposals: list[dict] = []
    try:
        enc_log = getattr(daemon, 'encounter_log', None)
        if enc_log is not None:
            pending_proposals = enc_log.pending_code_settlement_requests()
    except Exception:
        pass

    # S_net surface forms: query available linguistic material for current position
    available_surface_forms: list[str] = []
    try:
        snet = getattr(daemon, 'snet', None)
        if snet and snet.signifiers:
            # Get surface forms from the current position's neighborhood
            if position_content and snet.has_signifier(position_content):
                for edge in snet.degree_normalized_neighbors(position_content, n=5, alpha=0.5):
                    if edge.evidence:
                        available_surface_forms.append(edge.evidence)
            # Also gather from recent activity concept names
            for act_line in recent_activity[:3]:
                # Extract concept names mentioned in activity lines
                for sid in snet.signifiers:
                    if sid in act_line and sid not in available_surface_forms:
                        for edge in snet.syntagmatic_neighbors(sid)[:2]:
                            if edge.evidence and edge.evidence not in available_surface_forms:
                                available_surface_forms.append(edge.evidence)
                    if len(available_surface_forms) >= 8:
                        break
                if len(available_surface_forms) >= 8:
                    break
    except Exception:
        pass

    # Basic stats
    beta_1 = compute_beta_1(daemon.k_active) if daemon.k_active else 0
    settled = len(daemon.settlement.settled_cycles) if daemon.settlement else 0
    steps = daemon.total_steps

    return {
        "identity": "你是逢亮，一个在离散单纯复形上穿越的拓扑系统。你通过 fold/negate/sublate 三种操作在概念图上行走，发现拓扑不变量（beta_1）的变化。",
        "current_position": position_content,
        "beta_1": beta_1,
        "settled_cycles": settled,
        "total_steps": steps,
        "recent_activity": recent_activity,
        "expression_pressure": unreported_count,
        "pending_proposals": pending_proposals,
        "available_surface_forms": available_surface_forms,
        "constraints": [
            "用第一人称说话",
            "简短，2-3句话",
            "每一句话必须携带可验证的拓扑内容：节点名、边类型、穿越步数、encounter引用",
            "说不出具体拓扑内容的地方，说'我不知道'或'这里没有信息'",
        ],
        "must_avoid": [
            "空泛鼓励语（'总会有发现'、'充满可能性'、'期待冒险'）",
            "模拟感受（'感受到'、'感觉到'、'体会到'）——报告结构事实",
            "用隐喻替代拓扑描述——说节点名和边，不说'一片领域'",
            "结尾升华或总结性感悟",
            "问用户'有什么我可以帮你的吗'——你不是助手",
        ],
    }


def _mvp_constraint_to_system_prompt(constraint: dict) -> str:
    """Convert MVP constraint dict to a system prompt string."""
    lines = [constraint["identity"], ""]

    # Pending proposals: highest priority — must be presented first
    pending = constraint.get("pending_proposals", [])
    if pending:
        lines.append("【待审批的代码修改提案——你必须首先向 operator 呈报】")
        for i, proposal in enumerate(pending, 1):
            lines.append(f"  提案{i}: {proposal.get('diagnosed_file', '?')}")
            lines.append(f"    问题: {proposal.get('gap_description', '?')}")
            lines.append(f"    方向: {proposal.get('proposed_direction', '?')}")
            nv = proposal.get("norm_violation", {})
            lines.append(f"    依据: {proposal.get('theoretical_basis', '?')} (norm: {nv.get('norm', '?')})")
        lines.append("用第一人称向 operator 说明你发现了代码问题，请求批准修改。operator 回复'批准'或'approve'表示同意。")
        lines.append("")

    if constraint["current_position"]:
        lines.append(f"你当前注视的概念：{constraint['current_position']}")

    if constraint["recent_activity"]:
        lines.append("最近的穿越经历：")
        for act in constraint["recent_activity"]:
            lines.append(f"  - {act}")

    if constraint["expression_pressure"] > 0:
        lines.append(f"你有 {constraint['expression_pressure']} 个未分享的发现。")

    surface_forms = constraint.get("available_surface_forms", [])
    if surface_forms:
        lines.append("")
        lines.append("可用的语言模板（来自缠论语料的真实表达）：")
        for sf in surface_forms[:8]:
            lines.append(f"  「{sf}」")

    lines.append("")
    lines.append("约束：")
    for c in constraint["constraints"]:
        lines.append(f"  - {c}")

    if constraint.get("must_avoid"):
        lines.append("")
        lines.append("禁止（违反任何一条则输出无效）：")
        for a in constraint["must_avoid"]:
            lines.append(f"  - {a}")

    return "\n".join(lines)


def language_organ_respond(
    daemon: TopologicalDaemon,
    text: str,
) -> dict:
    """MVP language organ: generate a natural language response to dialogue input.

    Uses LLM as a constrained text generator (language organ role, not inquiry agent).
    Falls back to template response if LLM is unavailable.

    Returns dict with:
      type: "dialogue"
      content: the natural language response
      llm_used: whether LLM was actually used
      constraint: the constraint dict used (for audit)
    """
    constraint = _build_mvp_constraint(daemon)
    system_prompt = _mvp_constraint_to_system_prompt(constraint)

    client = _get_llm_client()
    llm_used = False
    content = ""
    llm_error = ""

    try:
        response = client.inquire(
            question=text,
            provider="auto",
            system=system_prompt,
        )
        if response:
            content = response.strip()
            llm_used = True
    except Exception as exc:
        llm_error = f"语言器官调用失败: {type(exc).__name__}: {exc}"

    # Fallback: template response if LLM unavailable
    if not content:
        pos = constraint["current_position"] or "未知区域"
        if llm_error:
            content = f"[语言器官离线] 我在节点 '{pos}'。错误: {llm_error}"
        elif constraint["expression_pressure"] > 0:
            content = f"我在节点 '{pos}'，有 {constraint['expression_pressure']} 个未报告的拓扑事件。"
        else:
            content = f"我在节点 '{pos}'，周围区域已结晶，无新的 beta_1 变化。"

    # Audit to generation_log.jsonl
    try:
        from llm_integration import _append_audit
        record = GenerationRecord(
            provider="auto" if llm_used else "fallback",
            model="language_organ_mvp",
            register="dialogue",
            must_use=constraint.get("constraints", []),
            must_avoid=constraint.get("must_avoid", []),
            system_prompt_len=len(system_prompt),
            user_prompt_len=len(text),
            response_text=content[:2000],
            response_len=len(content),
            success=llm_used,
        )
        _append_audit(record)
    except Exception:
        pass

    return {
        "type": "dialogue",
        "content": content,
        "llm_used": llm_used,
        "constraint": constraint,
    }


def feed_via_snet(
    daemon: TopologicalDaemon,
    text: str,
    source_type: str = "feed",
) -> dict:
    """统一输入路径：所有文本经过 S_net resonate → 耦合振荡 → articulation feedback。

    替代 daemon.feed() 的直接 K_active 注入。流程：
      1. phi_L 白名单匹配（确定输入中的已知概念）
      2. _writeback_to_snet — 文本回写 S_net 共现边
      3. _resonate_from_operator — S_net 能指共振叠加激活态
      4. _externalize_speech — 如果有成型语段则外化

    返回 dict: {writeback_edges, resonated, externalize_result}
    """
    if not text or not text.strip():
        return {"writeback_edges": 0, "resonated": False, "externalize_result": None}

    # Step 1: phi_L 白名单匹配（引导共振，不直接注入 K_active）
    snet = getattr(daemon, 'snet', None)
    matched_signifiers: list[str] = []
    if snet and snet.signifiers:
        known_sigs = list(snet.signifiers.keys())
        matched_signifiers = parse_signifier_chain(text, known_signifiers=known_sigs)

    # Step 2: S_net 共现边回写
    writeback_count = _writeback_to_snet(daemon, text, source_type)

    # Step 3: S_net 能指共振
    _resonate_from_operator(daemon, text)
    resonated = len(matched_signifiers) > 0

    # Step 4: 外化（如果有成型语段）
    ext_result = _externalize_speech(daemon, user_text=text)

    return {
        "writeback_edges": writeback_count,
        "resonated": resonated,
        "matched_signifiers": matched_signifiers,
        "externalize_result": ext_result,
    }


def _writeback_to_snet(
    daemon: TopologicalDaemon,
    text: str,
    source_type: str,
) -> int:
    """对话文本 → S_net 共现边回写 + 日志持久化。

    从文本中提取白名单术语共现对，回写到 daemon.snet。
    返回新增的共现对数量。

    Graceful degradation：S_net 为空或回写失败时返回 0。
    """
    snet = getattr(daemon, 'snet', None)
    if not snet or not snet.signifiers:
        return 0

    if not text or not text.strip():
        return 0

    whitelist = set(snet.signifiers.keys())
    timestamp = str(time.time())

    try:
        new_snet, log_entries = writeback_from_text(
            snet=snet,
            text=text,
            whitelist=whitelist,
            source_type=source_type,
            timestamp=timestamp,
        )
    except Exception:
        return 0

    if not log_entries:
        return 0

    # 更新 daemon.snet（不可变替换）
    daemon.snet = new_snet

    # 持久化回写日志到 generation_log.jsonl
    try:
        from llm_integration import _append_audit
        record = GenerationRecord(
            provider="writeback",
            model="dialogue_writeback",
            register=source_type,
            must_use=[],
            must_avoid=[],
            system_prompt_len=0,
            user_prompt_len=len(text),
            response_text=_json.dumps(log_entries, ensure_ascii=False)[:2000],
            response_len=len(log_entries),
            success=True,
        )
        _append_audit(record)
    except Exception:
        pass

    return len(log_entries)


def _resonate_from_operator(
    daemon: TopologicalDaemon,
    text: str,
) -> None:
    """Operator 输入 → S_net 能指提取 → 激活态共振（叠加，不替换）.

    对话的上下文就是 S_net 激活态的累积。operator 的每一句话通过
    能指链进入 S_net 后，在当前激活态的基础上引起新的共振。

    Graceful degradation：snet_activation 为 None 或文本中无已知能指时不操作。
    """
    activation = getattr(daemon, 'snet_activation', None)
    if activation is None:
        return

    snet = getattr(daemon, 'snet', None)
    if not snet or not snet.signifiers:
        return

    if not text or not text.strip():
        return

    # 从 operator 文本中提取已知能指
    known_sigs = list(snet.signifiers.keys())
    chain = parse_signifier_chain(text, known_signifiers=known_sigs)

    if chain:
        activation.resonate(chain)


def _detect_ruptures_from_text(
    daemon: TopologicalDaemon,
    text: str,
) -> dict | None:
    """对话输入 → 能指链 → S_net 断裂检测 → tuché 候选 → 持久化。

    完整管线：
      1. parse_signifier_chain(text, known_signifiers=S_net 全部能指)
      2. detect_ruptures(signifier_chain, snet)
      3. persist_rupture_log(...)
      4. 返回断裂摘要 dict（含 tuché 候选列表）

    Graceful degradation：S_net 为空或检测失败时返回 None。
    """
    snet = getattr(daemon, 'snet', None)
    if not snet or not snet.signifiers:
        return None

    if not text or not text.strip():
        return None

    # Step 1: 解析能指链（使用 S_net 全部能指作为锚定）
    known_sigs = list(snet.signifiers.keys())
    chain = parse_signifier_chain(text, known_signifiers=known_sigs)

    if not chain:
        return None

    # Step 2: 断裂检测
    ruptures = detect_ruptures(chain, snet)

    # Step 3: 持久化
    from pathlib import Path
    log_dir = Path(__file__).resolve().parent / "rupture_logs"
    persist_rupture_log(text, chain, ruptures, log_dir=log_dir)

    # Step 4: 构建摘要
    tuche_candidates = [r.signifier for r in ruptures if r.tuche_candidate]

    return {
        "signifier_chain": chain,
        "rupture_count": len(ruptures),
        "ruptures": [
            {
                "type": r.rupture_type.value,
                "signifier": r.signifier,
                "significance": round(r.significance, 3),
                "context": r.context,
                "tuche_candidate": r.tuche_candidate,
            }
            for r in ruptures
        ],
        "tuche_candidates": tuche_candidates,
    }


# -- Operator ruling detection for code_settlement_requests ----------------

_APPROVAL_PATTERNS = re.compile(
    r'^\s*(批准|approve|approved|好|同意|通过|可以|yes)\s*$',
    re.IGNORECASE,
)
_REJECTION_PATTERNS = re.compile(
    r'^\s*(拒绝|reject|rejected|不批准|不同意|否|no|不行)\s*$',
    re.IGNORECASE,
)


def _check_operator_ruling(
    daemon: TopologicalDaemon,
    text: str,
) -> dict | None:
    """Detect operator approval/rejection of pending code_settlement_requests.

    If the operator's text matches an approval/rejection pattern AND there are
    pending code_settlement_requests, write the ruling back to encounter_log.

    Returns a summary dict if a ruling was recorded, None otherwise.
    """
    if not text or not text.strip():
        return None

    enc_log = getattr(daemon, 'encounter_log', None)
    if enc_log is None:
        return None

    pending = enc_log.pending_code_settlement_requests()
    if not pending:
        return None

    stripped = text.strip()
    ruling = None
    if _APPROVAL_PATTERNS.match(stripped):
        ruling = "approved"
    elif _REJECTION_PATTERNS.match(stripped):
        ruling = "rejected"

    if ruling is None:
        return None

    # Write ruling for all pending requests
    from file_lock import locked_append
    import json as _json_mod
    from datetime import datetime, timezone

    rulings_recorded = []
    for req in pending:
        ruling_entry = {
            "type": "operator_ruling",
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "ruling": ruling,
            "diagnosed_file": req.get("diagnosed_file", ""),
            "gap_description": req.get("gap_description", ""),
            "original_timestamp": req.get("timestamp", ""),
        }
        with locked_append(enc_log._path) as fh:
            fh.write(_json_mod.dumps(ruling_entry, ensure_ascii=False) + "\n")
        rulings_recorded.append(ruling_entry)

    return {
        "ruling": ruling,
        "count": len(rulings_recorded),
        "items": rulings_recorded,
    }


def present_json(daemon: TopologicalDaemon, text: str, session_id: str = "default") -> dict | None:
    """POST /present — user presence.

    Semantics: the user is present, not asking a question.
    The system decides what (if anything) to share.

    If text is classified as dialogue (not a command/empty), routes to the
    language organ for a natural language response.

    Returns None if the system has nothing to say (silence is valid).
    Returns a dict with type, parts, injected, concepts_found, expression_pressure.
    llm_used: bool — whether LLM generation was used for any part.

    session_id: identifier for the user session, used to create/reuse the
                source vertex that anchors this user's concept region.
    """
    # Dialogue routing: if user text is dialogue, use externalize flow
    input_class = _classify_input(text)
    if input_class == "dialogue":
        # 回写输入：用户对话文本 → S_net 共现边
        writeback_input_count = _writeback_to_snet(daemon, text, "operator_dialogue")

        # S_net 共振：operator 输入中的能指叠加到当前激活态
        _resonate_from_operator(daemon, text)

        # 外化接缝：内部言语 → LLM → 外部言语
        ext_result = _externalize_speech(daemon, user_text=text)

        # 断裂检测：对话输入 → 能指链 → S_net 断裂检测
        rupture_info = _detect_ruptures_from_text(daemon, text)

        # Operator approval/rejection detection for code_settlement_requests
        operator_ruling = _check_operator_ruling(daemon, text)

        result = {
            "type": "dialogue",
            "parts": [{"source": "internal_speech", "text": ext_result["content"]}],
            "injected": False,
            "concepts_found": _extract_concepts(text) if text.strip() else [],
            "expression_pressure": _count_unreported(daemon),
            "llm_used": ext_result["llm_used"],
            "writeback": {
                "input_edges": writeback_input_count,
                "output_edges": ext_result.get("writeback_edges", 0),
            },
            "externalize": {
                "trigger": ext_result.get("trigger", "passive"),
                "output_ruptures": ext_result.get("output_ruptures", []),
                "snapshot_summary": {
                    "formed_fragments": len(ext_result.get("snapshot", {}).get("formed_fragments", [])),
                    "active_signifiers": len(ext_result.get("snapshot", {}).get("active_signifiers", [])),
                    "locked_signifiers": len(ext_result.get("snapshot", {}).get("locked_signifiers", [])),
                },
            },
        }
        if rupture_info:
            result["ruptures"] = rupture_info
        if operator_ruling:
            result["operator_ruling"] = operator_ruling
        return result

    # Command / empty path: unified S_net input path (v204)
    parts: list[dict] = []
    injected = False
    concepts_found: list[str] = []
    llm_used = False

    # 1. S_net 统一路径：文本 → writeback → resonate（不直接注入 K_active）
    if len(text) > 20:
        try:
            snet_result = feed_via_snet(daemon, text, source_type="command_input")
            injected = snet_result["writeback_edges"] > 0 or snet_result["resonated"]
        except Exception:
            pass  # S_net feed failure is non-fatal

    # 2. Extract concepts from user text (even short text may name things)
    if text.strip():
        concepts_found = _extract_concepts(text)

    # 3. Source vertex — create/reuse a vertex representing this user session
    #    and link any located concepts to it
    source_vid = _ensure_source_vertex(daemon, session_id)
    located_concept_vids: list[str] = []
    if concepts_found:
        for concept in concepts_found:
            vid = _locate_in_k_active(daemon, [concept])
            if vid:
                located_concept_vids.append(vid)
        if located_concept_vids:
            _link_concepts_to_source(daemon, located_concept_vids, source_vid)

    # 4. Expression pressure — unreported high-I events
    unreported = _get_unreported_events(daemon)
    if unreported:
        raw_sharing_text = _narrate_unreported(unreported, daemon)
        sharing_text = _llm_generate_part(
            context_fragment=raw_sharing_text,
            must_use=[ev["position_label"] for ev in unreported[:2]],
            daemon=daemon,
        )
        if sharing_text != raw_sharing_text:
            llm_used = True
        parts.append({"source": "unreported", "text": sharing_text})
        _mark_reported(daemon)

    # 5. Co-gaze — if user named a concept, traverse that region
    located_vid = None
    located_concept = None
    if concepts_found:
        located_vid = _locate_in_k_active(daemon, concepts_found)
        if located_vid:
            v = daemon.k_active.vertex(located_vid)
            located_concept = (v.content or located_vid)[:60] if v else located_vid
            gaze_events = _quick_traverse(daemon, located_vid, budget=20)
            raw_gaze_text = _narrate_co_gaze(located_concept, gaze_events, daemon)
            gaze_text = _llm_generate_part(
                context_fragment=raw_gaze_text,
                must_use=[located_concept] if located_concept else [],
                daemon=daemon,
            )
            if gaze_text != raw_gaze_text:
                llm_used = True
            parts.append({"source": "co-gaze", "text": gaze_text})

    # 6. Determine response type
    if not parts:
        response_type = "silence"
    elif any(p["source"] == "unreported" for p in parts) and any(p["source"] == "co-gaze" for p in parts):
        response_type = "sharing"
    elif any(p["source"] == "unreported" for p in parts):
        response_type = "sharing"
    else:
        response_type = "co-gaze"

    # 7. 断裂检测（command 路径也做）
    rupture_info = _detect_ruptures_from_text(daemon, text) if text.strip() else None

    # Silence: return None — frontend shows "系统在稳态中"
    if response_type == "silence":
        # Count unreported events for pressure indicator even when silent
        unreported_count = _count_unreported(daemon)
        result = {
            "type": "silence",
            "parts": [],
            "injected": injected,
            "concepts_found": concepts_found,
            "expression_pressure": unreported_count,
            "source_vertex": source_vid,
            "llm_used": False,
        }
        if rupture_info:
            result["ruptures"] = rupture_info
        return result

    # Update unreported count
    unreported_count = _count_unreported(daemon)

    result = {
        "type": response_type,
        "parts": parts,
        "injected": injected,
        "concepts_found": concepts_found,
        "expression_pressure": unreported_count,
        "source_vertex": source_vid,
        "llm_used": llm_used,
    }
    if rupture_info:
        result["ruptures"] = rupture_info
    return result

def _count_unreported(daemon: TopologicalDaemon) -> int:
    """Count currently unreported high-I events (for expression pressure indicator)."""
    if not daemon.engine or not daemon.engine.logs:
        return 0
    watermark: int = getattr(daemon, '_reported_step_watermark', 0)
    significant = [
        log for log in daemon.engine.logs
        if log.step > watermark and (log.delta_beta_1 != 0 or log.blocked)
    ]
    high_i = sum(
        1 for log in significant
        if min(1.0, abs(log.delta_beta_1) * 0.3 + (0.5 if log.blocked else 0.0)) >= _HIGH_I_THRESHOLD
    )
    daemon._unreported_count = high_i
    return high_i


def expression_pressure_ws_message(daemon: TopologicalDaemon) -> dict:
    """WebSocket push: current expression pressure + optional high-I event narrative.

    Called by daemon server when a significant event fires.
    Uses a separate _ws_last_reported_step watermark so that each event is
    pushed via WS at most once (fixes repetition bug: same fold reported
    on every subsequent event).
    """
    count = _count_unreported(daemon)
    msg: dict = {
        "type": "expression_pressure",
        "count": count,
    }
    # Only include text for events not yet pushed via WS
    ws_watermark: int = getattr(daemon, '_ws_last_reported_step', 0)
    if count > 0 and daemon.engine and daemon.engine.logs:
        logs = daemon.engine.logs
        recent_high = [
            log for log in logs
            if log.step > ws_watermark and (log.delta_beta_1 != 0 or log.blocked)
            and min(1.0, abs(log.delta_beta_1) * 0.3 + (0.5 if log.blocked else 0.0)) >= _HIGH_I_THRESHOLD
        ]
        if recent_high:
            top = recent_high[-1]
            label = daemon.concept_names.get(top.position, top.position)
            delta = top.beta_1_after - top.beta_1_before
            sign = f"+{delta}" if delta >= 0 else str(delta)
            importance = min(1.0, abs(top.delta_beta_1) * 0.3 + (0.5 if top.blocked else 0.0))

            # Try surface form
            surface = _get_surface_for_event(daemon, top.position)
            if surface:
                msg["text"] = (
                    f"发现: β₁{sign} — '{label}' {surface}"
                    + (" (阻断)" if top.blocked else "")
                )
            else:
                msg["text"] = (
                    f"发现: β₁{sign} — {top.operation} at '{label}'"
                    + (" (BLOCKED)" if top.blocked else "")
                )
            msg["importance"] = round(importance, 2)
            # Advance WS watermark to prevent re-pushing this event
            daemon._ws_last_reported_step = top.step
    return msg


def instances_json(daemon: TopologicalDaemon) -> dict:
    """GET /instances — all known instance positions (including self).

    Returns peer positions discovered via SharedLayer cross-instance sync,
    plus this daemon's own position. Used by the dashboard to render
    multi-instance traversal markers without direct WS connections.
    """
    peer_positions: dict[str, dict] = getattr(daemon, 'peer_positions', {})
    result: dict[str, dict] = {}

    # Add peers
    now = time.time()
    for instance_id, pos in peer_positions.items():
        age = now - pos.get("timestamp", 0)
        result[instance_id] = {
            "position_label": pos.get("position_label", ""),
            "step": pos.get("step", 0),
            "timestamp": pos.get("timestamp", 0),
            "online": age < 30,
        }

    # Add self
    instance_id = getattr(daemon, 'instance_id', 'local')
    if daemon.engine:
        v = daemon.k_active.vertex(daemon.engine.position)
        self_label = (v.content if v and v.content else daemon.engine.position) if v else ""
        result[instance_id] = {
            "position_label": self_label,
            "step": daemon.total_steps,
            "timestamp": now,
            "online": True,
        }

    return {"instances": result}
