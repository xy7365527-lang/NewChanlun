"""Daemon API — pure functions that convert daemon state to JSON responses.

No HTTP framework dependency. Each function takes a TopologicalDaemon
and returns a JSON-serializable dict/list.
"""

from __future__ import annotations

import time
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from daemon import TopologicalDaemon

from engine import compute_beta_1, Edge, EdgeType, Vertex


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
    }


def topology_json(daemon: TopologicalDaemon, center: str | None = None, radius: int = 2, full: bool = False) -> dict:
    """GET /topology — K_active subgraph snapshot with f-values.

    Default: top-50 skeleton. ?full=true for all. ?center=X&radius=N for local.
    """
    graph = daemon.k_active
    active = graph.active_vertex_ids()
    total_v = len(active)
    total_e = len(graph.active_edges())

    if center and center in set(active):
        vids, edges = graph.local_subgraph(center, radius)
    elif full:
        vids = active
        edges = graph.active_edges()
    else:
        degrees = {v: len(graph.neighbors(v)) for v in active}
        top = sorted(active, key=lambda v: degrees.get(v, 0), reverse=True)[:50]
        vids = top
        top_set = set(top)
        edges = [e for e in graph.active_edges() if e.source in top_set and e.target in top_set]

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
    import re
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
# /present endpoint — user presence handler
# ---------------------------------------------------------------------------

def present_json(daemon: TopologicalDaemon, text: str, session_id: str = "default") -> dict | None:
    """POST /present — user presence.

    Semantics: the user is present, not asking a question.
    The system decides what (if anything) to share.

    Returns None if the system has nothing to say (silence is valid).
    Returns a dict with type, parts, injected, concepts_found, expression_pressure.

    session_id: identifier for the user session, used to create/reuse the
                source vertex that anchors this user's concept region.
    """
    parts: list[dict] = []
    injected = False
    concepts_found: list[str] = []

    # 1. Inject — if text is substantial, feed it into K_active permanently
    if len(text) > 20:
        try:
            daemon.feed(text)
            injected = True
        except Exception:
            pass  # Feed failure is non-fatal

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
        sharing_text = _narrate_unreported(unreported, daemon)
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
            gaze_text = _narrate_co_gaze(located_concept, gaze_events, daemon)
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

    # Silence: return None — frontend shows "系统在稳态中"
    if response_type == "silence":
        # Count unreported events for pressure indicator even when silent
        unreported_count = _count_unreported(daemon)
        return {
            "type": "silence",
            "parts": [],
            "injected": injected,
            "concepts_found": concepts_found,
            "expression_pressure": unreported_count,
            "source_vertex": source_vid,
        }

    # Update unreported count
    unreported_count = _count_unreported(daemon)

    return {
        "type": response_type,
        "parts": parts,
        "injected": injected,
        "concepts_found": concepts_found,
        "expression_pressure": unreported_count,
        "source_vertex": source_vid,
    }


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
    """
    count = _count_unreported(daemon)
    msg: dict = {
        "type": "expression_pressure",
        "count": count,
    }
    # If count just crossed a threshold, include the most recent event narrative
    if count > 0 and daemon.engine and daemon.engine.logs:
        logs = daemon.engine.logs
        watermark: int = getattr(daemon, '_reported_step_watermark', 0)
        recent_high = [
            log for log in logs
            if log.step > watermark and (log.delta_beta_1 != 0 or log.blocked)
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
    return msg
