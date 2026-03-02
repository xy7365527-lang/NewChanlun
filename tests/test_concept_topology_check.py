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
    detect_negation_consistency,
    compute_graph_invariants,
    is_structurally_significant,
    run_all_checks,
    _is_evolution_relation,
    _should_check_conflict,
    EVOLUTION_RELATIONS,
    TOPOLOGICAL_RELATIONS,
    LOGICAL_RELATIONS,
    NAVIGATIONAL_RELATIONS,
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


# ==================== Phase 2 Tests ====================


class TestEvolutionHelpers:
    def test_is_evolution_relation(self):
        assert _is_evolution_relation({"relation": "modifies"}) is True
        assert _is_evolution_relation({"relation": "refines"}) is True
        assert _is_evolution_relation({"relation": "revises"}) is True
        assert _is_evolution_relation({"relation": "depends_on"}) is False
        assert _is_evolution_relation({"relation": "negates"}) is False

    def test_should_check_conflict_revises(self):
        assert _should_check_conflict({"relation": "revises"}) is True

    def test_should_check_conflict_modifies(self):
        assert _should_check_conflict({"relation": "modifies"}) is True

    def test_should_check_conflict_refines_unknown(self):
        assert _should_check_conflict({"relation": "refines", "modification_kind": "unknown"}) is True

    def test_should_check_conflict_refines_known(self):
        assert _should_check_conflict({"relation": "refines", "modification_kind": "refine"}) is False

    def test_should_check_conflict_other(self):
        assert _should_check_conflict({"relation": "depends_on"}) is False
        assert _should_check_conflict({"relation": "references"}) is False


class TestDetectNegationConsistency:
    def test_no_issues_when_empty(self, topology_dir):
        base, blocks = topology_dir
        result = detect_negation_consistency(base)
        assert result == []

    def test_undeclared_negation(self, topology_dir):
        """Body negates (order=2) exist but no order=1 counterpart."""
        base, blocks = topology_dir

        # Only order=2 negates (from content enrichment)
        rel = make_relation(
            from_id=blocks["a"]["id"],
            to_id=blocks["b"]["id"],
            relation="negates",
            order=2,
            created_by=blocks["genesis"]["id"],
            negation_type="negated_concept",
        )
        append_relation(rel, base)

        result = detect_negation_consistency(base)
        undeclared = [r for r in result if r["type"] == "undeclared_negation"]
        assert len(undeclared) == 1
        assert undeclared[0]["severity"] == "warn"

    def test_phantom_negation(self, topology_dir):
        """Order=1 negates exists but no order=2 body counterpart."""
        base, blocks = topology_dir

        # Only order=1 negates (from dag.yaml migration)
        rel = make_relation(
            from_id=blocks["a"]["id"],
            to_id=blocks["b"]["id"],
            relation="negates",
            order=1,
            created_by=blocks["genesis"]["id"],
        )
        append_relation(rel, base)

        result = detect_negation_consistency(base)
        phantom = [r for r in result if r["type"] == "phantom_negation"]
        assert len(phantom) == 1
        assert phantom[0]["severity"] == "info"

    def test_consistent_negation(self, topology_dir):
        """Both order=1 and order=2 negates exist → no issues."""
        base, blocks = topology_dir

        for order in (1, 2):
            rel = make_relation(
                from_id=blocks["a"]["id"],
                to_id=blocks["b"]["id"],
                relation="negates",
                order=order,
                created_by=blocks["genesis"]["id"],
            )
            append_relation(rel, base)

        result = detect_negation_consistency(base)
        # Both orders present → no undeclared or phantom
        a_b_issues = [r for r in result
                      if r["block"] == blocks["a"]["id"]
                      and r["target"] == blocks["b"]["id"]]
        assert len(a_b_issues) == 0


class TestRefinesRevisesConflict:
    def test_revises_always_checked(self, topology_dir):
        """revises relation should always be included in conflict detection."""
        base, blocks = topology_dir

        # B revises A
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="revises", order=1, created_by=blocks["genesis"]["id"],
            target_desc="K4 划分", modification="根本性重定义",
        ), base)

        # C references A but doesn't depend on B
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_concept_conflicts(base)
        assert len(result) >= 1
        assert result[0]["type"] == "stale_reference"

    def test_refines_unknown_checked(self, topology_dir):
        """refines with kind=unknown should be checked."""
        base, blocks = topology_dir

        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="refines", order=2, created_by=blocks["genesis"]["id"],
            modification_kind="unknown",
        ), base)

        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_concept_conflicts(base)
        assert len(result) >= 1

    def test_refines_known_not_checked(self, topology_dir):
        """refines with kind=refine should NOT be checked."""
        base, blocks = topology_dir

        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="refines", order=2, created_by=blocks["genesis"]["id"],
            modification_kind="refine",
        ), base)

        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_concept_conflicts(base)
        assert len(result) == 0


class TestEvolutionRelationsInDuplicateDetection:
    def test_refines_excludes_duplicate(self, topology_dir):
        """Same concept defined by A and B, B refines A → evolution, not duplicate."""
        base, blocks = topology_dir
        concept_id = make_concept_id("测试概念")

        for key, defn in [("a", "定义1"), ("b", "定义2")]:
            append_relation(make_relation(
                from_id=blocks[key]["id"], to_id=concept_id,
                relation="defines", order=2, created_by=blocks["genesis"]["id"],
                concept_term="测试概念", concept_definition=defn,
            ), base)

        # B refines A
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="refines", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_duplicate_concepts(base)
        assert len(result) == 0

    def test_revises_excludes_duplicate(self, topology_dir):
        """Same concept defined by A and B, B revises A → evolution, not duplicate."""
        base, blocks = topology_dir
        concept_id = make_concept_id("另一概念")

        for key, defn in [("a", "旧定义"), ("b", "新定义")]:
            append_relation(make_relation(
                from_id=blocks[key]["id"], to_id=concept_id,
                relation="defines", order=2, created_by=blocks["genesis"]["id"],
                concept_term="另一概念", concept_definition=defn,
            ), base)

        # B revises A
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="revises", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_duplicate_concepts(base)
        assert len(result) == 0


class TestFoldingBirthProxy:
    def test_two_defines_is_duplicate(self, topology_dir):
        """Second defines for same concept → duplicate folding status with needs_review."""
        base, blocks = topology_dir
        concept_id = make_concept_id("跨域概念")

        for key, defn in [("a", "域1定义"), ("b", "域2定义")]:
            append_relation(make_relation(
                from_id=blocks[key]["id"], to_id=concept_id,
                relation="defines", order=2, created_by=blocks["genesis"]["id"],
                concept_term="跨域概念", concept_definition=defn,
            ), base)

        result = detect_duplicate_concepts(base)
        assert len(result) == 1
        assert result[0]["folding_status"] == "duplicate"
        assert result[0]["needs_review"] is True

    def test_three_defines_is_duplicate(self, topology_dir):
        """Three+ defines for same concept → duplicate (same as N=2)."""
        base, blocks = topology_dir
        concept_id = make_concept_id("多重概念")

        for key, defn in [("a", "定义A"), ("b", "定义B"), ("c", "定义C")]:
            append_relation(make_relation(
                from_id=blocks[key]["id"], to_id=concept_id,
                relation="defines", order=2, created_by=blocks["genesis"]["id"],
                concept_term="多重概念", concept_definition=defn,
            ), base)

        result = detect_duplicate_concepts(base)
        assert len(result) == 1
        assert result[0]["folding_status"] == "duplicate"
        assert result[0]["needs_review"] is True


class TestDirectionalMismatch:
    def test_missing_dependency_is_divergent(self, topology_dir):
        base, blocks = topology_dir

        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_reference_dependency_mismatch(base)
        missing = [r for r in result if r["type"] == "missing_dependency"]
        assert len(missing) == 1
        assert missing[0]["direction"] == "divergent"
        assert missing[0]["signal"] == "potential_depends_on"

    def test_structural_only_is_convergent(self, topology_dir):
        base, blocks = topology_dir

        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_reference_dependency_mismatch(base)
        structural = [r for r in result if r["type"] == "structural_only"]
        assert len(structural) == 1
        assert structural[0]["direction"] == "convergent"
        assert structural[0]["signal"] == "potential_negates"


class TestMissingDependencyTriage:
    """Tests for missing_dependency triage classification (309号裁定)."""

    def test_should_be_depends_on_when_substantive_relation_exists(self, topology_dir):
        """If from has a substantive relation to target, triage=should_be_depends_on."""
        base, blocks = topology_dir

        # C references A (creates missing_dependency)
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)
        # C also negates A (substantive relation → should_be_depends_on)
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="negates", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_reference_dependency_mismatch(base)
        missing = [r for r in result if r["type"] == "missing_dependency"]
        assert len(missing) == 1
        assert missing[0]["triage"] == "should_be_depends_on"

    def test_should_be_references_when_no_substantive_relation(self, topology_dir):
        """No substantive relation but target has genealogy ID → should_be_references."""
        base, blocks = topology_dir

        # C references A (creates missing_dependency, no substantive relation)
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_reference_dependency_mismatch(base)
        missing = [r for r in result if r["type"] == "missing_dependency"]
        assert len(missing) == 1
        assert missing[0]["triage"] == "should_be_references"

    def test_triage_with_modifies_relation(self, topology_dir):
        """modifies is a substantive relation → should_be_depends_on."""
        base, blocks = topology_dir

        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="modifies", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_reference_dependency_mismatch(base)
        missing = [r for r in result if r["type"] == "missing_dependency"]
        assert len(missing) == 1
        assert missing[0]["triage"] == "should_be_depends_on"

    def test_triage_with_revises_relation(self, topology_dir):
        """revises is a substantive relation → should_be_depends_on."""
        base, blocks = topology_dir

        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="revises", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_reference_dependency_mismatch(base)
        missing = [r for r in result if r["type"] == "missing_dependency"]
        assert len(missing) == 1
        assert missing[0]["triage"] == "should_be_depends_on"

    def test_structural_only_has_no_triage(self, topology_dir):
        """structural_only mismatches should NOT have triage field."""
        base, blocks = topology_dir

        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_reference_dependency_mismatch(base)
        structural = [r for r in result if r["type"] == "structural_only"]
        assert len(structural) == 1
        assert "triage" not in structural[0]


class TestAnnotationProposal:
    def test_convergent_mismatch_generates_annotation(self, topology_dir):
        base, blocks = topology_dir

        # Create structural_only mismatch (convergent)
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        report = run_all_checks(base)
        annotations = report.get("proposed_annotations", [])
        assert len(annotations) >= 1
        assert annotations[0]["type"] == "annotation"
        assert annotations[0]["status"] == "proposed"

    def test_report_includes_phase2_fields(self, topology_dir):
        base, blocks = topology_dir
        report = run_all_checks(base)
        assert report["invariant_status"] == "graph_invariants_computed"
        assert report["formalization_scope"] == "directed_graph_only"
        assert report["ruling_273_ack"] is True
        assert "negation_consistency" in report


class TestMalformedRelationDefense:
    def test_malformed_modifies_skipped(self, topology_dir):
        """Relation record missing 'from' or 'to' should not crash."""
        base, blocks = topology_dir

        # Write a malformed relation directly to JSONL
        import json
        jsonl_path = base / "relations.jsonl"
        malformed = {"relation": "modifies", "from": "", "to": blocks["a"]["id"],
                     "order": 1, "created_by": blocks["genesis"]["id"],
                     "timestamp": "2026-01-01T00:00:00+00:00"}
        with open(jsonl_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(malformed, ensure_ascii=False) + "\n")

        # Should not raise
        result = detect_concept_conflicts(base)
        assert isinstance(result, list)

    def test_missing_from_in_negates(self, topology_dir):
        """negates record without from should not crash negation consistency."""
        base, blocks = topology_dir

        import json
        jsonl_path = base / "relations.jsonl"
        malformed = {"relation": "negates", "from": "", "to": blocks["a"]["id"],
                     "order": 1, "created_by": blocks["genesis"]["id"],
                     "timestamp": "2026-01-01T00:00:00+00:00",
                     "validity": "active", "invalidated_by": None, "invalidated_at": None}
        with open(jsonl_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(malformed, ensure_ascii=False) + "\n")

        result = detect_negation_consistency(base)
        assert isinstance(result, list)


class TestTransitiveDependencyInConflicts:
    def test_transitive_depends_on_resolves_conflict(self, topology_dir):
        """B modifies A. C→D→B depends_on chain. C references A → no conflict."""
        base, blocks = topology_dir

        # B modifies A
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="modifies", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        # D depends_on B
        append_relation(make_relation(
            from_id=blocks["d"]["id"], to_id=blocks["b"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        # C depends_on D (transitive: C→D→B)
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["d"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        # C references A
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_concept_conflicts(base)
        # C transitively depends on B → no stale reference
        assert len(result) == 0


class TestGraphInvariants:
    def test_empty_graph(self, topology_dir):
        """Empty graph (no topological relations) → all zeros."""
        base, blocks = topology_dir
        result = compute_graph_invariants(base)
        assert result["active_beta_0"] == 0
        assert result["active_cycle_rank"] == 0
        assert result["full_beta_0"] == 0
        assert result["full_cycle_rank"] == 0

    def test_simple_chain(self, topology_dir):
        """A→B→C linear chain → β₀=1, cycle_rank=0."""
        base, blocks = topology_dir

        append_relation(make_relation(
            from_id=blocks["a"]["id"], to_id=blocks["b"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["c"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        result = compute_graph_invariants(base)
        assert result["active_beta_0"] == 1
        assert result["active_cycle_rank"] == 0
        assert result["full_beta_0"] == 1
        assert result["full_cycle_rank"] == 0

    def test_two_disconnected_components(self, topology_dir):
        """A→B and C→D → β₀=2, cycle_rank=0."""
        base, blocks = topology_dir

        append_relation(make_relation(
            from_id=blocks["a"]["id"], to_id=blocks["b"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["d"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = compute_graph_invariants(base)
        assert result["active_beta_0"] == 2
        assert result["active_cycle_rank"] == 0

    def test_cycle_detection(self, topology_dir):
        """A→B→C→A cycle → β₀=1, cycle_rank=1."""
        base, blocks = topology_dir

        append_relation(make_relation(
            from_id=blocks["a"]["id"], to_id=blocks["b"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["c"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        result = compute_graph_invariants(base)
        assert result["active_beta_0"] == 1
        # SCC {A,B,C}: 3 edges, 3 vertices → cycle_rank = 3-3+1 = 1
        assert result["active_cycle_rank"] == 1

    def test_active_vs_full_with_invalidated(self, topology_dir):
        """Invalidated negates edge: active graph excludes it, full graph includes it."""
        base, blocks = topology_dir

        # A→B depends_on (always active)
        append_relation(make_relation(
            from_id=blocks["a"]["id"], to_id=blocks["b"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        # B negates C (invalidated)
        import json
        jsonl_path = base / "relations.jsonl"
        invalidated_rel = {
            "from": blocks["b"]["id"],
            "to": blocks["c"]["id"],
            "relation": "negates",
            "order": 1,
            "created_by": blocks["genesis"]["id"],
            "timestamp": "2026-01-01T00:00:00+00:00",
            "validity": "invalidated",
            "invalidated_by": blocks["d"]["id"],
            "invalidated_at": "2026-01-02T00:00:00+00:00",
        }
        with open(jsonl_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(invalidated_rel, ensure_ascii=False) + "\n")

        result = compute_graph_invariants(base)
        # Full graph: A→B, B→C → β₀=1 (all connected)
        assert result["full_beta_0"] == 1
        # Active graph: only A→B (negates invalidated) → β₀=2 (A-B component + C isolated... wait C has no edge)
        # Actually C only appears in the invalidated edge.
        # Active edges: just A→B. Active nodes: {A, B}. β₀=1
        # Full edges: A→B, B→C. Full nodes: {A, B, C}. β₀=1
        assert result["active_beta_0"] == 1
        # But full graph has C connected via B→C
        assert result["full_beta_0"] == 1

    def test_non_topological_relations_excluded(self, topology_dir):
        """records/defines/annotates relations should not be counted as edges."""
        base, blocks = topology_dir

        concept_id = make_concept_id("测试")
        # defines relation (non-topological)
        append_relation(make_relation(
            from_id=blocks["a"]["id"], to_id=concept_id,
            relation="defines", order=2, created_by=blocks["genesis"]["id"],
            concept_term="测试", concept_definition="test",
        ), base)

        # records relation (non-topological)
        rewrite = make_block("rewrite", "migration", {"action": "test"}, refs=[blocks["a"]["id"]])
        write_block(rewrite, base)
        append_relation(make_relation(
            from_id=rewrite["id"], to_id=blocks["a"]["id"],
            relation="records", order=2, created_by=rewrite["id"],
        ), base)

        result = compute_graph_invariants(base)
        # No topological edges → all zeros
        assert result["active_beta_0"] == 0
        assert result["active_cycle_rank"] == 0

    def test_complex_scc_cycle_rank(self, topology_dir):
        """A→B→C→A with extra A→C → cycle_rank=2."""
        base, blocks = topology_dir

        # Triangle A→B→C→A
        append_relation(make_relation(
            from_id=blocks["a"]["id"], to_id=blocks["b"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["c"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)
        # Extra edge A→C
        append_relation(make_relation(
            from_id=blocks["a"]["id"], to_id=blocks["c"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = compute_graph_invariants(base)
        # SCC {A,B,C}: 4 edges, 3 vertices → cycle_rank = 4-3+1 = 2
        assert result["active_cycle_rank"] == 2

    def test_run_all_checks_includes_invariants(self, topology_dir):
        """run_all_checks report should include graph_invariants."""
        base, blocks = topology_dir
        report = run_all_checks(base)
        assert "graph_invariants" in report
        gi = report["graph_invariants"]
        assert "active_beta_0" in gi
        assert "active_cycle_rank" in gi
        assert "full_beta_0" in gi
        assert "full_cycle_rank" in gi
        assert report["invariant_status"] == "graph_invariants_computed"


class TestStaleReferenceDedup:
    """Stale reference conflicts with same source relation should be folded."""

    def test_same_source_relation_folded(self, topology_dir):
        """B modifies A; C and D both reference A without depending on B.
        Two raw stale_reference entries should be folded into one."""
        base, blocks = topology_dir

        # B(200) modifies A(100)
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="modifies", order=1, created_by=blocks["genesis"]["id"],
            target_desc="K4 划分", modification="提升为折叠根据",
        ), base)

        # C(300) references A(100) — no depends_on B
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        # D(400) references A(100) — no depends_on B
        append_relation(make_relation(
            from_id=blocks["d"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_concept_conflicts(base)
        # Should be folded into 1 entry (same source relation: B modifies A)
        assert len(result) == 1
        entry = result[0]
        assert entry["type"] == "stale_reference"
        assert entry["affected_count"] == 2
        assert len(entry["affected_blocks"]) == 2

    def test_different_source_relations_not_folded(self, topology_dir):
        """B modifies A, C modifies A independently.
        D references A — triggers two distinct source relations.
        Each should remain as a separate entry."""
        base, blocks = topology_dir

        # B(200) modifies A(100)
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="modifies", order=1, created_by=blocks["genesis"]["id"],
            target_desc="K4 划分", modification="修改1",
        ), base)

        # C(300) modifies A(100) — different modifier
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="modifies", order=1, created_by=blocks["genesis"]["id"],
            target_desc="K4 划分", modification="修改2",
        ), base)

        # D(400) references A(100) — newer than both B(200) and C(300)
        append_relation(make_relation(
            from_id=blocks["d"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_concept_conflicts(base)
        # Two distinct source relations: (A, B) and (A, C)
        # D triggers stale_reference for both → 2 folded entries, each with affected_count=1
        assert len(result) == 2
        for entry in result:
            assert entry["type"] == "stale_reference"
            assert entry["affected_count"] == 1
            assert len(entry["affected_blocks"]) == 1

    def test_affected_count_and_blocks_correct(self, topology_dir):
        """Verify affected_count matches len(affected_blocks) and contents are correct."""
        base, blocks = topology_dir

        # B(200) modifies A(100)
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="modifies", order=1, created_by=blocks["genesis"]["id"],
            target_desc="desc", modification="mod",
        ), base)

        # C(300) references A — stale
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        # D(400) references A — stale
        append_relation(make_relation(
            from_id=blocks["d"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_concept_conflicts(base)
        assert len(result) == 1
        entry = result[0]
        assert entry["affected_count"] == len(entry["affected_blocks"])
        assert entry["affected_count"] == 2
        # affected_blocks should contain genealogy ids for C and D
        assert "300" in entry["affected_blocks"]
        assert "400" in entry["affected_blocks"]

    def test_non_stale_reference_not_affected(self, topology_dir):
        """If there were non-stale_reference conflicts, they would not be folded.

        Currently the only conflict type is stale_reference, so we verify that
        the dedup logic passes through non-stale types unchanged by checking
        that a single stale_reference still gets affected_count/affected_blocks.
        """
        base, blocks = topology_dir

        # B(200) modifies A(100)
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="modifies", order=1, created_by=blocks["genesis"]["id"],
            target_desc="desc", modification="mod",
        ), base)

        # Only C(300) references A — single stale ref
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = detect_concept_conflicts(base)
        assert len(result) == 1
        entry = result[0]
        assert entry["type"] == "stale_reference"
        assert entry["affected_count"] == 1
        assert entry["affected_blocks"] == ["300"]

    def test_run_all_checks_uses_deduped_count(self, topology_dir):
        """run_all_checks summary.conflicts should reflect deduped count."""
        base, blocks = topology_dir

        # B(200) modifies A(100)
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["a"]["id"],
            relation="modifies", order=1, created_by=blocks["genesis"]["id"],
            target_desc="desc", modification="mod",
        ), base)

        # C(300) and D(400) both reference A — 2 raw, 1 deduped
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)
        append_relation(make_relation(
            from_id=blocks["d"]["id"], to_id=blocks["a"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        report = run_all_checks(base)
        assert report["summary"]["conflicts"] == 1
        assert report["concept_conflicts"]["count"] == 1


class TestIsStructurallySignificant:
    def test_returns_none_without_stats(self):
        rel = {"relation": "negates", "from": "a", "to": "b"}
        assert is_structurally_significant(rel) is None

    def test_returns_none_with_stats(self):
        rel = {"relation": "negates", "from": "a", "to": "b"}
        stats = {"active_beta_0": 1, "active_cycle_rank": 0}
        assert is_structurally_significant(rel, stats) is None


class TestRelationLayerSets:
    """Verify LOGICAL_RELATIONS ⊂ NAVIGATIONAL_RELATIONS ⊂ TOPOLOGICAL_RELATIONS."""

    def test_logical_subset_of_navigational(self):
        assert LOGICAL_RELATIONS <= NAVIGATIONAL_RELATIONS

    def test_navigational_subset_of_topological(self):
        # NAVIGATIONAL_RELATIONS 中的 "defines" 不在 TOPOLOGICAL_RELATIONS 中
        # 因为 defines 是拓扑意义上的非拓扑关系，但在导航层有意义
        # 验证除 defines 外全部在 TOPOLOGICAL_RELATIONS 中
        nav_topo = NAVIGATIONAL_RELATIONS - {"defines"}
        assert nav_topo <= TOPOLOGICAL_RELATIONS

    def test_logical_is_strict_subset(self):
        assert LOGICAL_RELATIONS < NAVIGATIONAL_RELATIONS

    def test_logical_contents(self):
        assert "depends_on" in LOGICAL_RELATIONS
        assert "negates" in LOGICAL_RELATIONS
        assert "revises" in LOGICAL_RELATIONS
        assert "supersedes" in LOGICAL_RELATIONS
        assert "references" not in LOGICAL_RELATIONS

    def test_navigational_contents(self):
        assert "references" in NAVIGATIONAL_RELATIONS
        assert "defines" in NAVIGATIONAL_RELATIONS
        assert "refines" in NAVIGATIONAL_RELATIONS


class TestLayeredGraphInvariants:
    """Tests for layer1/layer2 graph invariants."""

    def test_empty_graph_has_zero_layered_invariants(self, topology_dir):
        """Empty graph → all layered invariants zero."""
        base, blocks = topology_dir
        result = compute_graph_invariants(base)
        assert result["layer1_beta_0"] == 0
        assert result["layer1_cycle_rank"] == 0
        assert result["layer2_beta_0"] == 0
        assert result["layer2_cycle_rank"] == 0

    def test_layer1_only_logical_edges(self, topology_dir):
        """depends_on + references: layer1 sees only depends_on, layer2 sees both."""
        base, blocks = topology_dir

        # depends_on: A→B (logical, in both layers)
        append_relation(make_relation(
            from_id=blocks["a"]["id"], to_id=blocks["b"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)

        # references: B→C (navigational only, not in layer1)
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["c"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = compute_graph_invariants(base)

        # Layer 1: only A→B edge → β₀=1, cycle_rank=0
        assert result["layer1_beta_0"] == 1
        assert result["layer1_cycle_rank"] == 0

        # Layer 2: A→B + B→C → β₀=1, cycle_rank=0
        assert result["layer2_beta_0"] == 1
        assert result["layer2_cycle_rank"] == 0

        # Active (full topo): same as layer2 in this case
        assert result["active_beta_0"] == 1
        assert result["active_cycle_rank"] == 0

    def test_layer1_cycle_rank_less_than_active(self, topology_dir):
        """A→B→C→A cycle via depends_on + extra references edge.
        Layer1 sees the cycle; active sees cycle + extra edge → higher cycle_rank."""
        base, blocks = topology_dir

        # Triangle A→B→C→A via depends_on (logical)
        append_relation(make_relation(
            from_id=blocks["a"]["id"], to_id=blocks["b"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)
        append_relation(make_relation(
            from_id=blocks["b"]["id"], to_id=blocks["c"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)
        append_relation(make_relation(
            from_id=blocks["c"]["id"], to_id=blocks["a"]["id"],
            relation="depends_on", order=1, created_by=blocks["genesis"]["id"],
        ), base)
        # Extra edge A→C via references (navigational, not logical)
        append_relation(make_relation(
            from_id=blocks["a"]["id"], to_id=blocks["c"]["id"],
            relation="references", order=2, created_by=blocks["genesis"]["id"],
        ), base)

        result = compute_graph_invariants(base)

        # Layer 1: 3 edges (depends_on only), 3 vertices → cycle_rank=1
        assert result["layer1_cycle_rank"] == 1
        # Active: 4 edges, 3 vertices → cycle_rank=2
        assert result["active_cycle_rank"] == 2
        # layer1 < active
        assert result["layer1_cycle_rank"] < result["active_cycle_rank"]

    def test_run_all_checks_includes_layered_invariants(self, topology_dir):
        """run_all_checks report should include layer1/layer2 invariants."""
        base, blocks = topology_dir
        report = run_all_checks(base)
        gi = report["graph_invariants"]
        assert "layer1_beta_0" in gi
        assert "layer1_cycle_rank" in gi
        assert "layer2_beta_0" in gi
        assert "layer2_cycle_rank" in gi
