"""Encounter log — record topological events during conversational traversal.

Writes JSONL to .chanlun/traversal-events.jsonl (append-only backup).
Primary access path: memory nodes in K_active (domain:memory).
"""

from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

from file_lock import locked_append


# Memory node content prefix — matches the [domain:xxx] convention
MEMORY_DOMAIN_PREFIX = "[domain:memory]"


def _make_encounter_vertex_id(step: int, operation: str) -> str:
    """Generate vertex_id for an encounter memory node."""
    return f"memory:encounter:{step}:{operation}"


def _make_settlement_vertex_id(step: int) -> str:
    """Generate vertex_id for a settlement memory node."""
    return f"memory:settlement:{step}"


def _make_residue_vertex_id(residue_type: str, index: int, step: int) -> str:
    """Generate vertex_id for a residue memory node."""
    return f"memory:residue:{residue_type}:{index}:{step}"


class EncounterLog:
    """Append-only log of topological encounter events.

    Uses file locking for safe concurrent writes from multiple instances.
    JSONL file is kept as append-only backup (crash recovery).
    Primary access path is through memory nodes injected into K_active.
    """

    def __init__(self, log_path: str = ".chanlun/traversal-events.jsonl") -> None:
        self._path = Path(log_path)
        self._path.parent.mkdir(parents=True, exist_ok=True)

    def record_encounter(
        self,
        concept_a: str,
        concept_b: str,
        encounter_type: str,
        f_value: int,
        context: str,
        beta_1_before: int,
        beta_1_after: Optional[int] = None,
        session: Optional[str] = None,
        step: Optional[int] = None,
        graph_id: Optional[str] = None,
        session_id: Optional[str] = None,
        g_value: int = -99,
        jaccard_similarity: float = -1.0,
    ) -> dict:
        """Record one encounter event. Returns the event dict.

        Args:
            step: traversal step number (used for history node vertex_id).
                  If None, no history node metadata is attached.
            graph_id: instance identity (hostname-PID) for session isolation.
            session_id: walker session identifier (unique per daemon run).
            g_value: vertex-disjoint path count g(v,w) — 392·2 annotation.
            jaccard_similarity: Jaccard similarity of neighbor sets.
        """
        event = {
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "concept_a": concept_a,
            "concept_b": concept_b,
            "encounter_type": encounter_type,
            "f_value": f_value,
            "g_value": g_value,
            "jaccard_similarity": round(jaccard_similarity, 4),
            "context": context,
            "beta_1_before": beta_1_before,
            "beta_1_after": beta_1_after,
            "session": session,
            "graph_id": graph_id,
            "session_id": session_id,
        }
        if step is not None:
            event["step"] = step
        with locked_append(self._path) as fh:
            fh.write(json.dumps(event, ensure_ascii=False) + "\n")
        return event

    def _load_all(self) -> list[dict]:
        """Load all events from the JSONL file."""
        if not self._path.is_file():
            return []
        events: list[dict] = []
        with open(self._path, encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if line:
                    events.append(json.loads(line))
        return events

    def recent(self, n: int = 10) -> list[dict]:
        """Return the most recent n encounter records."""
        events = self._load_all()
        return events[-n:]

    def by_concept(self, keyword: str) -> list[dict]:
        """Return all encounters involving a concept (substring match on concept_a/concept_b)."""
        keyword_lower = keyword.lower()
        return [
            ev for ev in self._load_all()
            if keyword_lower in ev.get("concept_a", "").lower()
            or keyword_lower in ev.get("concept_b", "").lower()
        ]

    def record_code_settlement_request(
        self,
        diagnosed_file: str,
        gap_description: str,
        proposed_direction: str,
        theoretical_basis: str,
        norm_violation: dict,
    ) -> dict:
        """Record a code_settlement_request — 逢亮的提案权.

        When self-diagnosis (proprioception → self domain traversal) identifies
        a gap in the codebase, this writes a code_settlement_request to the
        encounter log. The request carries proposal authority only, not
        execution authority.

        Args:
            diagnosed_file: relative path of the file containing the gap
            gap_description: what the gap is
            proposed_direction: suggested fix direction
            theoretical_basis: which norm/genealogy justifies the request
            norm_violation: dict with keys norm, actual_state, severity
        """
        request = {
            "type": "code_settlement_request",
            "timestamp": datetime.now(timezone.utc).isoformat(),
            "diagnosed_file": diagnosed_file,
            "gap_description": gap_description,
            "proposed_direction": proposed_direction,
            "theoretical_basis": theoretical_basis,
            "norm_violation": norm_violation,
            "status": "pending",
        }
        with locked_append(self._path) as fh:
            fh.write(json.dumps(request, ensure_ascii=False) + "\n")
        return request

    def pending_code_settlement_requests(self) -> list[dict]:
        """Return all code_settlement_request entries with status='pending'."""
        return [
            ev for ev in self._load_all()
            if ev.get("type") == "code_settlement_request"
            and ev.get("status") == "pending"
        ]

    def summary(self) -> dict:
        """Return statistical summary: total, by-type distribution, beta_1 trend."""
        events = self._load_all()
        total = len(events)

        type_counts: dict[str, int] = {}
        beta_changes: list[int] = []
        for ev in events:
            et = ev.get("encounter_type", "unknown")
            type_counts[et] = type_counts.get(et, 0) + 1
            before = ev.get("beta_1_before")
            after = ev.get("beta_1_after")
            if before is not None and after is not None:
                beta_changes.append(after - before)

        return {
            "total_encounters": total,
            "by_type": type_counts,
            "beta_1_changes": beta_changes,
            "net_beta_1_change": sum(beta_changes) if beta_changes else 0,
        }


# ---------------------------------------------------------------------------
# Memory node injection into K_active
# ---------------------------------------------------------------------------

def inject_encounter_memory_node(
    graph,
    step: int,
    operation: str,
    concept_a: str,
    concept_b: str,
    context: str,
    beta_1_before: int,
    beta_1_after: int,
    f_value: int = -99,
):
    """Inject an encounter record as a domain:memory node into K_active.

    Creates a vertex with domain:memory content and REFERENCE edges
    to the concept vertices involved in the encounter.

    Returns the new Graph (immutable pattern — original unchanged).
    """
    from engine import Graph, Vertex, Edge, EdgeType, VertexStatus

    vid = _make_encounter_vertex_id(step, operation)
    content = (
        f"{MEMORY_DOMAIN_PREFIX} encounter step={step} op={operation}: "
        f"{context[:200]}"
    )
    vertex = Vertex(id=vid, status=VertexStatus.ACTIVE, content=content, created_at=step)
    result = graph.add_vertex(vertex)

    # Connect to involved concept vertices (if they exist in the graph)
    for concept_id in (concept_a, concept_b):
        if concept_id and graph.vertex(concept_id) is not None:
            edge = Edge(
                source=vid, target=concept_id,
                edge_type=EdgeType.REFERENCE, created_at=step,
            )
            result = result.add_edge(edge)

    return result


def inject_settlement_memory_node(
    graph,
    step: int,
    cycle_edges: frozenset[tuple[str, str]],
    residue: tuple[dict, ...] = (),
):
    """Inject a settlement record as a domain:memory node into K_active.

    Creates a vertex for the settlement itself, plus vertices for each
    residue item. Connects settlement to cycle vertices and residue nodes
    to their referenced vertices via REFERENCE edges.

    Returns the new Graph.
    """
    from engine import Graph, Vertex, Edge, EdgeType, VertexStatus

    # Settlement node
    settle_vid = _make_settlement_vertex_id(step)
    cycle_desc = ", ".join(f"{s}->{t}" for s, t in sorted(cycle_edges))
    content = (
        f"{MEMORY_DOMAIN_PREFIX} settlement step={step} "
        f"cycle=[{cycle_desc[:150]}]"
    )
    vertex = Vertex(id=settle_vid, status=VertexStatus.ACTIVE, content=content, created_at=step)
    result = graph.add_vertex(vertex)

    # Collect all edges to add in batch
    new_edges: list[Edge] = []

    # Connect settlement to cycle vertices
    cycle_vids: set[str] = set()
    for src, tgt in cycle_edges:
        cycle_vids.add(src)
        cycle_vids.add(tgt)
    for cvid in sorted(cycle_vids):
        if graph.vertex(cvid) is not None:
            new_edges.append(Edge(
                source=settle_vid, target=cvid,
                edge_type=EdgeType.REFERENCE, created_at=step,
            ))

    # Residue nodes
    for idx, item in enumerate(residue):
        r_type = item.get("type", "unknown")
        r_data = item.get("data", {})
        r_vid = _make_residue_vertex_id(r_type, idx, step)
        r_content = (
            f"{MEMORY_DOMAIN_PREFIX} residue type={r_type} "
            f"settlement_step={step}"
        )
        r_vertex = Vertex(id=r_vid, status=VertexStatus.ACTIVE, content=r_content, created_at=step)
        result = result.add_vertex(r_vertex)

        # Connect residue to settlement
        new_edges.append(Edge(
            source=settle_vid, target=r_vid,
            edge_type=EdgeType.REFERENCE, created_at=step,
        ))

        # Connect residue to referenced concept vertices
        referenced_vids: set[str] = set()
        if r_type == "edge":
            for v in r_data.get("shared_vertices", []):
                referenced_vids.add(v)
        elif r_type == "tension":
            if r_data.get("vertex_a"):
                referenced_vids.add(r_data["vertex_a"])
            if r_data.get("vertex_b"):
                referenced_vids.add(r_data["vertex_b"])
        elif r_type == "boundary_edge":
            if r_data.get("internal_vertex"):
                referenced_vids.add(r_data["internal_vertex"])
            if r_data.get("external_vertex"):
                referenced_vids.add(r_data["external_vertex"])
        elif r_type == "nachtraeglichkeit":
            for v in r_data.get("affected_vertices", []):
                referenced_vids.add(v)
        elif r_type == "expression_pressure":
            for v in r_data.get("cycle_vertices", []):
                referenced_vids.add(v)

        for ref_vid in sorted(referenced_vids):
            if graph.vertex(ref_vid) is not None:
                new_edges.append(Edge(
                    source=r_vid, target=ref_vid,
                    edge_type=EdgeType.REFERENCE, created_at=step,
                ))

    # Batch add all edges in one operation
    if new_edges:
        result = result.add_edges_batch(new_edges)

    return result


def inject_settlement_nachtraeglichkeit_edge(
    graph,
    settlement_step_a: int,
    settlement_step_b: int,
    step: int,
):
    """Connect two settlement memory nodes via a Nachträglichkeit edge.

    settlement_step_b retroactively reinterprets settlement_step_a.
    Returns the new Graph, or the original if either node doesn't exist.
    """
    from engine import Edge, EdgeType

    vid_a = _make_settlement_vertex_id(settlement_step_a)
    vid_b = _make_settlement_vertex_id(settlement_step_b)

    if graph.vertex(vid_a) is None or graph.vertex(vid_b) is None:
        return graph

    edge = Edge(
        source=vid_b, target=vid_a,
        edge_type=EdgeType.REFERENCE, created_at=step,
        surface="nachtraeglichkeit",
    )
    return graph.add_edge(edge)


def rebuild_memory_from_jsonl(
    graph,
    encounter_log_path: str = ".chanlun/traversal-events.jsonl",
    settlement_history_path: str | None = None,
):
    """Rebuild memory nodes from JSONL files into K_active (crash recovery).

    Reads encounter events and settlement history from JSONL, injects
    corresponding domain:memory nodes. Idempotent — skips nodes that
    already exist in the graph.

    Returns the new Graph.
    """
    from pathlib import Path

    result = graph

    # 401号修复：不再从 encounter log 重建 encounter memory 节点。
    # encounter（fold/sublate/negate/blocked）是穿越轨迹，不是结构性事件。
    # "脚印不是宝藏" — 只有 settlement 和 residue 进入 K_active。
    # encounter log JSONL 保留为 append-only 备份（事件历史），但不注入 K_active。

    # Rebuild settlement memory nodes
    if settlement_history_path is not None:
        settle_path = Path(settlement_history_path)
        if settle_path.is_file():
            with open(settle_path, encoding="utf-8") as fh:
                for line in fh:
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        entry = json.loads(line)
                    except json.JSONDecodeError:
                        continue
                    if entry.get("type") != "settle":
                        continue
                    step = entry.get("step")
                    if step is None:
                        continue
                    vid = _make_settlement_vertex_id(step)
                    if result.vertex(vid) is not None:
                        continue
                    cycle_edges = frozenset(
                        tuple(e) for e in entry.get("cycle_edges", [])
                    )
                    result = inject_settlement_memory_node(
                        result,
                        step=step,
                        cycle_edges=cycle_edges,
                    )

    return result
