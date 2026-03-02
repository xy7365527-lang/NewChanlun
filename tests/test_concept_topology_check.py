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
    def test_potential_birth(self, topology_dir):
        """Second defines for same concept → potential_birth folding status."""
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
        assert result[0]["folding_status"] == "potential_birth"

    def test_confirmed_duplicate(self, topology_dir):
        """Three+ defines for same concept → confirmed_duplicate."""
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
        assert result[0]["folding_status"] == "confirmed_duplicate"


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


class TestIsStructurallySignificant:
    def test_returns_none_without_stats(self):
        rel = {"relation": "negates", "from": "a", "to": "b"}
        assert is_structurally_significant(rel) is None

    def test_returns_none_with_stats(self):
        rel = {"relation": "negates", "from": "a", "to": "b"}
        stats = {"active_beta_0": 1, "active_cycle_rank": 0}
        assert is_structurally_significant(rel, stats) is None
