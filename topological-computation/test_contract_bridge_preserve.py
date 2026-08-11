"""Regression: _contract must not collapse articulation bridge_* vertices.

Trigger (pre-fix):
  1. Articulation creates bridge_{orphan_sig} + REFERENCE edge to an anchor.
  2. Same step enqueues bridge into _step_new_vertices and calls _contract().
  3. Bridge is terrain-non-critical (leaf) → folded; REFERENCE becomes A→A self-loop;
     _sig_to_concepts points at a FOLDED id.

Introduced by f8d15ad037 contraction rewrite; not covered by open PRs
#906/#910/#921/#930/#954/#955/#956/#50/#51.
"""

from __future__ import annotations

from engine import (
    Edge,
    EdgeType,
    Graph,
    SettlementTracker,
    Vertex,
    VertexStatus,
)
from traversal import TraversalEngine


def _triangle() -> Graph:
    g = Graph()
    for vid in ("A", "B", "C"):
        g.add_vertex(Vertex(vid, VertexStatus.ACTIVE, vid, 0))
    g.add_edge(Edge("A", "B", EdgeType.DEPENDENCY, 0))
    g.add_edge(Edge("B", "C", EdgeType.DEPENDENCY, 0))
    g.add_edge(Edge("C", "A", EdgeType.DEPENDENCY, 0))
    return g


def test_contract_preserves_bridge_even_if_enqueued() -> None:
    """Defensive: bridge_* in _step_new_vertices must survive _contract."""
    eng = TraversalEngine(_triangle(), start="A")
    eng.step = 7
    eng.position = "A"
    eng.settlement = SettlementTracker()

    bridge_vid = "bridge_orphan_sig"
    eng.k_active = eng.k_active.add_vertex(
        Vertex(bridge_vid, VertexStatus.ACTIVE, "orphan_sig", 7, non_critical=True)
    )
    eng.k_active = eng.k_active.add_edge(
        Edge("A", bridge_vid, EdgeType.REFERENCE, 7)
    )
    eng._step_new_vertices.add(bridge_vid)

    collapsed = eng._contract()
    assert collapsed == 0
    assert eng.k_active.vertex(bridge_vid).status == VertexStatus.ACTIVE
    assert bridge_vid in eng.k_active._active_ids
    self_loops = [
        e for e in eng.k_active.active_edges() if e.source == e.target
    ]
    assert self_loops == []


def test_contract_still_collapses_plain_leaf() -> None:
    """Non-bridge new leaves remain collapsible."""
    eng = TraversalEngine(_triangle(), start="A")
    eng.step = 8
    eng.position = "A"
    eng.settlement = SettlementTracker()

    leaf = "tmp_leaf"
    eng.k_active = eng.k_active.add_vertex(
        Vertex(leaf, VertexStatus.ACTIVE, "leaf", 8)
    )
    eng.k_active = eng.k_active.add_edge(
        Edge("A", leaf, EdgeType.DEPENDENCY, 8)
    )
    eng._step_new_vertices.add(leaf)

    collapsed = eng._contract()
    assert collapsed == 1
    assert eng.k_active.vertex(leaf).status == VertexStatus.FOLDED


if __name__ == "__main__":
    test_contract_preserves_bridge_even_if_enqueued()
    test_contract_still_collapses_plain_leaf()
    print("PASS")
