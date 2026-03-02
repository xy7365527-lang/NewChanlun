"""tests for concept_topology_check.py

Tests the three detection functions using a synthetic block topology.
"""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

import pytest

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.block_topology import (
    make_block,
    make_concept_id,
    make_relation,
    write_block,
    append_relation,
    write_meta,
    _ensure_dirs,
)
from scripts.concept_topology_check import (
    detect_duplicate_concepts,
    detect_concept_conflicts,
    detect_reference_dependency_mismatch,
    run_all_checks,
)


@pytest.fixture
def topology_dir(tmp_path):
    """Create a temporary block topology directory with test data."""
    base = tmp_path / "block-topology"
    _ensure_dirs(base)

    # Create 4 event blocks simulating genealogy entries
    block_a = make_block("event", "migration", {"id": "100", "title": "Block A"})
    block_b = make_block("event", "migration", {"id": "200", "title": "Block B"})
    block_c = make_block("event", "migration", {"id": "300", "title": "Block C"})
    block_d = make_block("event", "migration", {"id": "400", "title": "Block D"})

    for b in [block_a, block_b, block_c, block_d]:
        write_block(b, base)

    # Create genesis block for created_by
    genesis = make_block("event", "migration", {"action": "genesis"})
    write_block(genesis, base)

    # Create meta.json
    meta = {
        "version": "1.0.0",
        "genesis_block_id": genesis["id"],
        "id_mapping": {
            "100": block_a["id"],
            "200": block_b["id"],
            "300": block_c["id"],
            "400": block_d["id"],
        },
        "content_enrichment": {"blocks_created": 4},
    }
    write_meta(meta, base)

    return base, {
        "a": block_a,
        "b": block_b,
        "c": block_c,
        "d": block_d,
        "genesis": genesis,
    }


class TestDetectDuplicateConcepts:
    def test_no_duplicates_when_empty(self, topology_dir):
        base, blocks = topology_dir
        result = detect_duplicate_concepts(base)
        assert result == []

    def test_no_duplicates_single_defines(self, topology_dir):
        base, blocks = topology_dir
        concept_id = make_concept_id("折叠")
        rel = make_relation(
            from_id=blocks["a"]["id"],
            to_id=concept_id,
            relation="defines",
            order=2,
            created_by=blocks["genesis"]["id"],
            concept_term="折叠",
            concept_definition="同一个对象在不同域中呈现不同的范畴身份",
        )
        append_relation(rel, base)
        result = detect_duplicate_concepts(base)
        assert result == []

    def test_detects_duplicate_different_definitions(self, topology_dir):
        base, blocks = topology_dir
        concept_id = make_concept_id("折叠")

        # Block A defines "折叠" with definition 1
        rel1 = make_relation(
            from_id=blocks["a"]["id"],
            to_id=concept_id,
            relation="defines",
            order=2,
            created_by=blocks["genesis"]["id"],
            concept_term="折叠",
            concept_definition="定义版本1",
        )
        append_relation(rel1, base)

        # Block B defines "折叠" with different definition
        rel2 = make_relation(
            from_id=blocks["b"]["id"],
            to_id=concept_id,
            relation="defines",
            order=2,
            created_by=blocks["genesis"]["id"],
            concept_term="折叠",
            concept_definition="定义版本2",
        )
        append_relation(rel2, base)

        result = detect_duplicate_concepts(base)
        assert len(result) == 1
        assert result[0]["concept_term"] == "折叠"
        assert result[0]["definition_count"] == 2

    def test_excludes_evolution_via_modifies(self, topology_dir):
        """Same concept defined by A and B, but B modifies A → evolution, not duplicate."""
        base, blocks = topology_dir
        concept_id = make_concept_id("K4顶点划分")

        # Block A defines concept
        rel1 = make_relation(
            from_id=blocks["a"]["id"],
            to_id=concept_id,
            relation="defines",
            order=2,
            created_by=blocks["genesis"]["id"],
            concept_term="K4顶点划分",
            concept_definition="操作便利驱动",
        )
        append_relation(rel1, base)

        # Block B defines same concept with different definition
        rel2 = make_relation(
            from_id=blocks["b"]["id"],
            to_id=concept_id,
            relation="defines",
            order=2,
            created_by=blocks["genesis"]["id"],
            concept_term="K4顶点划分",
            concept_definition="折叠根据驱动",
        )
        append_relation(rel2, base)

        # Block B modifies Block A
        mod_rel = make_relation(
            from_id=blocks["b"]["id"],
            to_id=blocks["a"]["id"],
            relation="modifies",
            order=1,
            created_by=blocks["genesis"]["id"],
        )
        append_relation(mod_rel, base)

        result = detect_duplicate_concepts(base)
        assert len(result) == 0  # evolution, not duplication

    def test_same_definition_not_flagged(self, topology_dir):
        base, blocks = topology_dir
        concept_id = make_concept_id("内在否定")

        for b_key in ("a", "b"):
            rel = make_relation(
                from_id=blocks[b_key]["id"],
                to_id=concept_id,
                relation="defines",
                order=2,
                created_by=blocks["genesis"]["id"],
                concept_term="内在否定",
                concept_definition="对象自身产生的否定",
            )
            append_relation(rel, base)

        result = detect_duplicate_concepts(base)
        assert len(result) == 0  # same definition text


class TestDetectConceptConflicts:
    def test_no_conflicts_when_empty(self, topology_dir):
        base, blocks = topology_dir
        result = detect_concept_conflicts(base)
        assert result == []

    def test_detects_stale_reference(self, topology_dir):
        """B modifies A, C references A but doesn't depend on B → stale."""
        base, blocks = topology_dir

        # B modifies A
        mod_rel = make_relation(
            from_id=blocks["b"]["id"],
            to_id=blocks["a"]["id"],
            relation="modifies",
            order=1,
            created_by=blocks["genesis"]["id"],
            target_desc="K4 划分",
            modification="提升为折叠根据",
        )
        append_relation(mod_rel, base)

        # C references A (but doesn't depend on B)
        ref_rel = make_relation(
            from_id=blocks["c"]["id"],
            to_id=blocks["a"]["id"],
            relation="references",
            order=2,
            created_by=blocks["genesis"]["id"],
        )
        append_relation(ref_rel, base)

        result = detect_concept_conflicts(base)
        assert len(result) == 1
        assert result[0]["type"] == "stale_reference"

    def test_no_conflict_when_depends_on_modifier(self, topology_dir):
        """B modifies A, C references A and depends_on B → no conflict."""
        base, blocks = topology_dir

        # B modifies A
        mod_rel = make_relation(
            from_id=blocks["b"]["id"],
            to_id=blocks["a"]["id"],
            relation="modifies",
            order=1,
            created_by=blocks["genesis"]["id"],
        )
        append_relation(mod_rel, base)

        # C references A
        ref_rel = make_relation(
            from_id=blocks["c"]["id"],
            to_id=blocks["a"]["id"],
            relation="references",
            order=2,
            created_by=blocks["genesis"]["id"],
        )
        append_relation(ref_rel, base)

        # C depends_on B
        dep_rel = make_relation(
            from_id=blocks["c"]["id"],
            to_id=blocks["b"]["id"],
            relation="depends_on",
            order=1,
            created_by=blocks["genesis"]["id"],
        )
        append_relation(dep_rel, base)

        result = detect_concept_conflicts(base)
        assert len(result) == 0


class TestDetectReferenceDependencyMismatch:
    def test_no_mismatch_when_empty(self, topology_dir):
        base, blocks = topology_dir
        result = detect_reference_dependency_mismatch(base)
        assert result == []

    def test_detects_missing_dependency(self, topology_dir):
        """C references A in body but has no depends_on A in frontmatter."""
        base, blocks = topology_dir

        ref_rel = make_relation(
            from_id=blocks["c"]["id"],
            to_id=blocks["a"]["id"],
            relation="references",
            order=2,
            created_by=blocks["genesis"]["id"],
        )
        append_relation(ref_rel, base)

        result = detect_reference_dependency_mismatch(base)
        missing = [r for r in result if r["type"] == "missing_dependency"]
        assert len(missing) == 1
        assert missing[0]["severity"] == "warn"

    def test_detects_structural_only(self, topology_dir):
        """C depends_on A in frontmatter but doesn't reference A in body."""
        base, blocks = topology_dir

        dep_rel = make_relation(
            from_id=blocks["c"]["id"],
            to_id=blocks["a"]["id"],
            relation="depends_on",
            order=1,
            created_by=blocks["genesis"]["id"],
        )
        append_relation(dep_rel, base)

        result = detect_reference_dependency_mismatch(base)
        structural = [r for r in result if r["type"] == "structural_only"]
        assert len(structural) == 1
        assert structural[0]["severity"] == "info"

    def test_consistent_ref_and_dep(self, topology_dir):
        """C references A and depends_on A → no mismatch."""
        base, blocks = topology_dir

        ref_rel = make_relation(
            from_id=blocks["c"]["id"],
            to_id=blocks["a"]["id"],
            relation="references",
            order=2,
            created_by=blocks["genesis"]["id"],
        )
        append_relation(ref_rel, base)

        dep_rel = make_relation(
            from_id=blocks["c"]["id"],
            to_id=blocks["a"]["id"],
            relation="depends_on",
            order=1,
            created_by=blocks["genesis"]["id"],
        )
        append_relation(dep_rel, base)

        result = detect_reference_dependency_mismatch(base)
        # No missing_dependency or structural_only for c→a pair
        c_a_results = [
            r for r in result
            if r["block"] == blocks["c"]["id"]
            and r["target"] == blocks["a"]["id"]
        ]
        assert len(c_a_results) == 0


class TestRunAllChecks:
    def test_clean_topology(self, topology_dir):
        base, blocks = topology_dir
        report = run_all_checks(base)
        assert report["summary"]["health"] == "clean"
        assert report["summary"]["duplicates"] == 0
        assert report["summary"]["conflicts"] == 0

    def test_with_issues(self, topology_dir):
        base, blocks = topology_dir

        # Create a conflict
        mod_rel = make_relation(
            from_id=blocks["b"]["id"],
            to_id=blocks["a"]["id"],
            relation="modifies",
            order=1,
            created_by=blocks["genesis"]["id"],
        )
        append_relation(mod_rel, base)

        ref_rel = make_relation(
            from_id=blocks["c"]["id"],
            to_id=blocks["a"]["id"],
            relation="references",
            order=2,
            created_by=blocks["genesis"]["id"],
        )
        append_relation(ref_rel, base)

        report = run_all_checks(base)
        assert report["summary"]["health"] == "issues_found"
        assert report["summary"]["conflicts"] >= 1


class TestEvolutionChainConnectivity:
    """R3: modifies evolution chains A→B→C should be recognized via connected components."""

    def test_transitive_evolution_not_flagged(self, topology_dir):
        """A→B→C modifies chain: all three define same concept → evolution, not duplicate."""
        base, blocks = topology_dir
        concept_id = make_concept_id("K4顶点划分")

        # A, B, C all define the concept with different definitions
        for key, defn in [("a", "版本1"), ("b", "版本2"), ("c", "版本3")]:
            rel = make_relation(
                from_id=blocks[key]["id"],
                to_id=concept_id,
                relation="defines",
                order=2,
                created_by=blocks["genesis"]["id"],
                concept_term="K4顶点划分",
                concept_definition=defn,
            )
            append_relation(rel, base)

        # B modifies A, C modifies B (chain: A←B←C)
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="modifies", order=2, created_by=blocks["genesis"]["id"],
        ), base)
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["b"]["id"],
            relation="modifies", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_duplicate_concepts(base)
        # All three are in the same modifies component → evolution
        assert len(result) == 0

    def test_disconnected_still_flagged(self, topology_dir):
        """A→B modifies chain + C isolated: C's definition is a real duplicate."""
        base, blocks = topology_dir
        concept_id = make_concept_id("折叠")

        for key, defn in [("a", "版本1"), ("b", "版本2"), ("c", "版本3")]:
            rel = make_relation(
                from_id=blocks[key]["id"],
                to_id=concept_id,
                relation="defines",
                order=2,
                created_by=blocks["genesis"]["id"],
                concept_term="折叠",
                concept_definition=defn,
            )
            append_relation(rel, base)

        # Only B modifies A — C is disconnected
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="modifies", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_duplicate_concepts(base)
        # C is not in the A-B component → flagged as duplicate
        assert len(result) == 1
        assert result[0]["concept_term"] == "折叠"


class TestTemporalOrderingGuard:
    """R4: detect_concept_conflicts should not flag blocks older than the modifier."""

    def test_older_block_not_flagged(self, topology_dir):
        """B(200) modifies A(100), C(100 — but wait, that's A).

        Use A(100) referencing itself scenario doesn't apply.
        Let's use: B(200) modifies A(100). A(100) references A(100) — skip (self).
        Need a block with lower genealogy_id than modifier.

        Scenario: C(300) modifies A(100). B(200) references A(100).
        B is older than C (200 < 300) → B should NOT be flagged.
        """
        base, blocks = topology_dir

        # C(300) modifies A(100)
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="modifies", order=2, created_by=blocks["genesis"]["id"],
            target_desc="K4 划分", modification="提升为折叠根据",
        ), base)

        # B(200) references A(100) — B is older than modifier C(300)
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_concept_conflicts(base)
        # B(200) < C(300) → temporal guard excludes B
        assert len(result) == 0

    def test_newer_block_still_flagged(self, topology_dir):
        """B(200) modifies A(100). D(400) references A(100) but doesn't depend on B.
        D is newer than B (400 > 200) → D should be flagged.
        """
        base, blocks = topology_dir

        # B(200) modifies A(100)
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="modifies", order=2, created_by=blocks["genesis"]["id"],
            target_desc="K4 划分", modification="提升为折叠根据",
        ), base)

        # D(400) references A(100) — D is newer than modifier B(200)
        append_relation(make_relation(
            from_id=blocks["d"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_concept_conflicts(base)
        # D(400) > B(200) → flagged as stale reference
        assert len(result) == 1
        assert result[0]["type"] == "stale_reference"


class TestMakeConceptId:
    def test_deterministic(self):
        id1 = make_concept_id("折叠")
        id2 = make_concept_id("折叠")
        assert id1 == id2

    def test_different_terms(self):
        id1 = make_concept_id("折叠")
        id2 = make_concept_id("内在否定")
        assert id1 != id2

    def test_sha256_format(self):
        cid = make_concept_id("test")
        assert len(cid) == 64
        assert all(c in "0123456789abcdef" for c in cid)

    def test_strip_whitespace(self):
        """W1: Leading/trailing whitespace should not affect concept id."""
        assert make_concept_id("折叠") == make_concept_id("  折叠  ")
        assert make_concept_id("折叠") == make_concept_id("折叠\n")

    def test_nfkc_normalization(self):
        """W1: NFKC normalization — fullwidth chars map to same id as ASCII."""
        import unicodedata
        # Fullwidth 'K' (U+FF2B) should NFKC-normalize to ASCII 'K'
        fullwidth_k = "\uff2b4"
        normal_k = "K4"
        assert make_concept_id(fullwidth_k) == make_concept_id(normal_k)
