"""Tests for topo_event_detector.py -- topology event detection.

Detects: cycle formation, cycle_rank changes, new connected components.
All tests use synthetic fixtures (tmp_path), no real block-topology data.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import Any

import pytest

# Will be importable after implementation
from scripts.topo_event_detector import (
    build_graph,
    detect_cycles,
    detect_new_components,
    diff_cycle_rank,
    load_relations,
    parse_relation,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _write_relations(path: Path, records: list[dict[str, Any]]) -> Path:
    """Write a relations.jsonl file and return its path."""
    rel_file = path / "relations.jsonl"
    with open(rel_file, "w", encoding="utf-8") as f:
        for rec in records:
            f.write(json.dumps(rec) + "\n")
    return rel_file


def _rel(src: str, tgt: str, rtype: str = "depends_on", **extra: Any) -> dict:
    """Shorthand for a relation record (old-style key 'relation')."""
    return {"from": src, "to": tgt, "relation": rtype, **extra}


def _rel_new(src: str, tgt: str, rtype: str = "depends_on", **extra: Any) -> dict:
    """Shorthand for a relation record (new-style key 'type')."""
    return {"from": src, "to": tgt, "type": rtype, **extra}


# ---------------------------------------------------------------------------
# parse_relation
# ---------------------------------------------------------------------------

class TestParseRelation:
    """parse_relation normalizes heterogeneous relation record schemas."""

    def test_old_format_relation_key(self):
        rec = {"from": "A", "to": "B", "relation": "depends_on"}
        src, tgt, rtype = parse_relation(rec)
        assert (src, tgt, rtype) == ("A", "B", "depends_on")

    def test_new_format_type_key(self):
        rec = {"from": "A", "to": "B", "type": "references"}
        src, tgt, rtype = parse_relation(rec)
        assert (src, tgt, rtype) == ("A", "B", "references")

    def test_source_target_keys(self):
        rec = {"source": "100", "target": "99", "type": "depends_on",
               "from": "hash_a", "to": "hash_b"}
        src, tgt, rtype = parse_relation(rec)
        # 'from'/'to' are the canonical hash-based endpoints
        assert (src, tgt, rtype) == ("hash_a", "hash_b", "depends_on")

    def test_relation_key_takes_precedence_over_type(self):
        rec = {"from": "A", "to": "B", "relation": "negates", "type": "depends_on"}
        src, tgt, rtype = parse_relation(rec)
        assert rtype == "negates"

    def test_missing_relation_and_type_returns_unknown(self):
        rec = {"from": "A", "to": "B"}
        src, tgt, rtype = parse_relation(rec)
        assert rtype == "unknown"


# ---------------------------------------------------------------------------
# load_relations
# ---------------------------------------------------------------------------

class TestLoadRelations:
    """load_relations reads a jsonl file into a list of (src, tgt, type) tuples."""

    def test_loads_all_records(self, tmp_path: Path):
        _write_relations(tmp_path, [
            _rel("A", "B"),
            _rel("B", "C", "references"),
            _rel_new("C", "A", "negates"),
        ])
        rels = load_relations(tmp_path / "relations.jsonl")
        assert len(rels) == 3

    def test_empty_file(self, tmp_path: Path):
        _write_relations(tmp_path, [])
        rels = load_relations(tmp_path / "relations.jsonl")
        assert rels == []

    def test_filters_by_last_n(self, tmp_path: Path):
        _write_relations(tmp_path, [
            _rel("A", "B"),
            _rel("B", "C"),
            _rel("C", "D"),
        ])
        rels = load_relations(tmp_path / "relations.jsonl", last_n=2)
        assert len(rels) == 2
        # Should be the last 2 records
        assert rels[0] == ("B", "C", "depends_on")
        assert rels[1] == ("C", "D", "depends_on")


# ---------------------------------------------------------------------------
# build_graph
# ---------------------------------------------------------------------------

class TestBuildGraph:
    """build_graph turns relation tuples into an adjacency dict."""

    def test_simple_graph(self):
        rels = [("A", "B", "depends_on"), ("B", "C", "references")]
        g = build_graph(rels)
        assert "B" in g["A"]
        assert "C" in g["B"]

    def test_empty(self):
        g = build_graph([])
        assert g == {}

    def test_self_loop(self):
        rels = [("A", "A", "depends_on")]
        g = build_graph(rels)
        assert "A" in g["A"]


# ---------------------------------------------------------------------------
# detect_cycles
# ---------------------------------------------------------------------------

class TestDetectCycles:
    """detect_cycles finds all elementary cycles in a directed graph."""

    def test_no_cycle(self):
        # A -> B -> C (DAG, no cycle)
        g = {"A": {"B"}, "B": {"C"}, "C": set()}
        cycles = detect_cycles(g)
        assert cycles == []

    def test_simple_cycle(self):
        # A -> B -> C -> A
        g = {"A": {"B"}, "B": {"C"}, "C": {"A"}}
        cycles = detect_cycles(g)
        assert len(cycles) >= 1
        # The cycle should contain A, B, C
        found = False
        for c in cycles:
            if set(c) == {"A", "B", "C"}:
                found = True
        assert found

    def test_self_loop(self):
        g = {"A": {"A"}}
        cycles = detect_cycles(g)
        assert len(cycles) == 1
        assert cycles[0] == ["A"]

    def test_multiple_cycles(self):
        # A->B->A and B->C->B
        g = {"A": {"B"}, "B": {"A", "C"}, "C": {"B"}}
        cycles = detect_cycles(g)
        assert len(cycles) >= 2

    def test_large_dag_no_cycle(self):
        # Linear chain of 20 nodes
        g = {}
        for i in range(20):
            g[str(i)] = {str(i + 1)} if i < 19 else set()
        cycles = detect_cycles(g)
        assert cycles == []


# ---------------------------------------------------------------------------
# diff_cycle_rank
# ---------------------------------------------------------------------------

class TestDiffCycleRank:
    """diff_cycle_rank computes cycle_rank change between two graph states."""

    def test_no_change(self):
        g = {"A": {"B"}, "B": set()}
        old_rank, new_rank, delta = diff_cycle_rank(g, g)
        assert delta == 0

    def test_new_cycle_increases_rank(self):
        old_g = {"A": {"B"}, "B": {"C"}, "C": set()}
        new_g = {"A": {"B"}, "B": {"C"}, "C": {"A"}}
        old_rank, new_rank, delta = diff_cycle_rank(old_g, new_g)
        assert new_rank > old_rank
        assert delta > 0

    def test_removing_edge_decreases_rank(self):
        old_g = {"A": {"B"}, "B": {"C"}, "C": {"A"}}
        new_g = {"A": {"B"}, "B": {"C"}, "C": set()}
        old_rank, new_rank, delta = diff_cycle_rank(old_g, new_g)
        assert delta < 0


# ---------------------------------------------------------------------------
# detect_new_components
# ---------------------------------------------------------------------------

class TestDetectNewComponents:
    """detect_new_components finds newly formed connected components."""

    def test_no_change(self):
        old_g = {"A": {"B"}, "B": set()}
        new_g = {"A": {"B"}, "B": set()}
        new_comps = detect_new_components(old_g, new_g)
        assert new_comps == []

    def test_new_isolated_node(self):
        old_g = {"A": {"B"}, "B": set()}
        new_g = {"A": {"B"}, "B": set(), "C": set()}
        new_comps = detect_new_components(old_g, new_g)
        assert len(new_comps) == 1
        assert "C" in new_comps[0]

    def test_new_connected_pair(self):
        old_g = {"A": {"B"}, "B": set()}
        new_g = {"A": {"B"}, "B": set(), "C": {"D"}, "D": set()}
        new_comps = detect_new_components(old_g, new_g)
        assert len(new_comps) == 1
        assert set(new_comps[0]) == {"C", "D"}

    def test_merged_components_not_reported(self):
        # old: {A,B} and {C,D} separate; new: A->C merges them
        old_g = {"A": {"B"}, "B": set(), "C": {"D"}, "D": set()}
        new_g = {"A": {"B", "C"}, "B": set(), "C": {"D"}, "D": set()}
        new_comps = detect_new_components(old_g, new_g)
        # Merging is not "new component" -- the merged component existed before
        assert new_comps == []


# ---------------------------------------------------------------------------
# Integration: full pipeline on synthetic data
# ---------------------------------------------------------------------------

class TestIntegration:
    """End-to-end test: load relations, detect events."""

    def test_cycle_detection_from_file(self, tmp_path: Path):
        # A->B->C->A = cycle
        _write_relations(tmp_path, [
            _rel("A", "B"),
            _rel("B", "C"),
            _rel("C", "A"),
        ])
        rels = load_relations(tmp_path / "relations.jsonl")
        g = build_graph(rels)
        cycles = detect_cycles(g)
        assert len(cycles) >= 1

    def test_component_detection_from_file(self, tmp_path: Path):
        _write_relations(tmp_path, [
            _rel("A", "B"),
            _rel("C", "D"),  # separate component
        ])
        rels = load_relations(tmp_path / "relations.jsonl")
        g = build_graph(rels)
        # If we compare to an empty prior graph, both components are new
        new_comps = detect_new_components({}, g)
        assert len(new_comps) == 2

    def test_diff_rank_from_file(self, tmp_path: Path):
        old_rels = [_rel("A", "B"), _rel("B", "C")]
        new_rels = old_rels + [_rel("C", "A")]
        old_g = build_graph([parse_relation(r) for r in old_rels])
        new_g = build_graph([parse_relation(r) for r in new_rels])
        _, _, delta = diff_cycle_rank(old_g, new_g)
        assert delta > 0

    def test_layer_filtering(self, tmp_path: Path):
        """Relations can be filtered by layer for layer-specific analysis."""
        from scripts.topo_event_detector import load_relations_by_layer
        _write_relations(tmp_path, [
            _rel("A", "B", "depends_on"),    # Layer 1
            _rel("C", "D", "references"),    # Layer 2
            _rel("E", "F", "records"),        # Layer 3
        ])
        l1 = load_relations_by_layer(tmp_path / "relations.jsonl", layer=1)
        l2 = load_relations_by_layer(tmp_path / "relations.jsonl", layer=2)
        assert len(l1) == 1
        assert l1[0] == ("A", "B", "depends_on")
        assert len(l2) == 1
        assert l2[0] == ("C", "D", "references")


# ---------------------------------------------------------------------------
# CLI smoke test
# ---------------------------------------------------------------------------

class TestCLI:
    """Smoke test for CLI invocation."""

    def test_last_n_flag(self, tmp_path: Path):
        _write_relations(tmp_path, [
            _rel("A", "B"),
            _rel("B", "C"),
            _rel("C", "A"),
        ])
        result = subprocess.run(
            [sys.executable, "scripts/topo_event_detector.py",
             "--last-n", "10",
             "--base", str(tmp_path)],
            capture_output=True, text=True,
            cwd=str(Path(__file__).resolve().parent.parent),
        )
        assert result.returncode == 0
        output = json.loads(result.stdout)
        assert "events" in output
        assert isinstance(output["events"], list)

    def test_empty_relations(self, tmp_path: Path):
        _write_relations(tmp_path, [])
        result = subprocess.run(
            [sys.executable, "scripts/topo_event_detector.py",
             "--last-n", "5",
             "--base", str(tmp_path)],
            capture_output=True, text=True,
            cwd=str(Path(__file__).resolve().parent.parent),
        )
        assert result.returncode == 0
        output = json.loads(result.stdout)
        assert output["events"] == []
