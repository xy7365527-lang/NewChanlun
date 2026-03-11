"""test_snet_cache.py -- S_net 持久化缓存 round-trip 测试。

验证:
  1. SNet.to_dict() / SNet.from_dict() round-trip 等价
  2. manifest 构建 + 匹配逻辑
  3. cache save / load round-trip
  4. diff_manifests 增量检测

认识论等级: L1（管线正确性验证）
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
from pathlib import Path

# Add topological-computation to path
script_dir = Path(__file__).resolve().parent
sys.path.insert(0, str(script_dir))

from signifier_net import (
    SNet, Signifier, SignifierEdge, AxisType,
    Morpheme, MorphemeStructure,
)


# ---------------------------------------------------------------------------
# Test helpers
# ---------------------------------------------------------------------------

def _build_test_snet() -> SNet:
    """Build a non-trivial SNet for testing."""
    snet = SNet()

    # Add signifiers
    sigs = [
        Signifier(id="中枢", surface_forms=("zhongshu",), source="k_active_projection", lang="zh", domain="chanlun"),
        Signifier(id="走势", surface_forms=("zoushi",), source="k_active_projection", lang="zh", domain="chanlun"),
        Signifier(id="笔", surface_forms=("bi",), source="corpus", lang="zh", domain="chanlun"),
        Signifier(id="Aufhebung", surface_forms=("aufheben", "sublation"), source="dictionary", lang="de", domain="hegel"),
    ]
    snet = snet.add_signifiers(sigs)

    # Add edges
    edges = [
        SignifierEdge(source="中枢", target="走势", axis=AxisType.SYNTAGMATIC, weight=3.5, evidence="走势中枢是走势的核心"),
        SignifierEdge(source="笔", target="走势", axis=AxisType.SYNTAGMATIC, weight=1.2, evidence="笔构成走势"),
        SignifierEdge(source="中枢", target="走势", axis=AxisType.PARADIGMATIC, weight=0.8, evidence="", relation="contrast"),
        SignifierEdge(source="Aufhebung", target="中枢", axis=AxisType.MORPHEME, weight=1.0, evidence="", differential="跨域映射"),
    ]
    snet = snet.add_edges(edges)

    # Add morpheme structure
    ms = MorphemeStructure(
        signifier_id="Aufhebung",
        morphemes=(
            Morpheme(form="Auf", meaning="up/on", lang="de", shared_with=("Aufgabe",)),
            Morpheme(form="heben", meaning="to lift", lang="de", shared_with=()),
        ),
        etymology="Hegel's term for dialectical sublation",
    )
    snet = snet.add_morpheme_structure(ms)

    return snet


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

def test_snet_to_dict_from_dict_roundtrip():
    """to_dict -> from_dict 应产生等价 SNet。"""
    original = _build_test_snet()
    data = original.to_dict()
    restored = SNet.from_dict(data)

    # Signifier count
    assert len(restored._signifiers) == len(original._signifiers), \
        f"Signifier count mismatch: {len(restored._signifiers)} != {len(original._signifiers)}"

    # Edge count
    assert len(restored._edges) == len(original._edges), \
        f"Edge count mismatch: {len(restored._edges)} != {len(original._edges)}"

    # Morpheme count
    assert len(restored._morphemes) == len(original._morphemes), \
        f"Morpheme count mismatch: {len(restored._morphemes)} != {len(original._morphemes)}"

    # Signifier content equality
    for sid in original._signifiers:
        orig_sig = original._signifiers[sid]
        rest_sig = restored._signifiers[sid]
        assert orig_sig.id == rest_sig.id
        assert orig_sig.surface_forms == rest_sig.surface_forms
        assert orig_sig.source == rest_sig.source
        assert orig_sig.lang == rest_sig.lang
        assert orig_sig.domain == rest_sig.domain

    # Edge content equality
    for i, (orig_e, rest_e) in enumerate(zip(original._edges, restored._edges)):
        assert orig_e.source == rest_e.source, f"Edge {i} source mismatch"
        assert orig_e.target == rest_e.target, f"Edge {i} target mismatch"
        assert orig_e.axis == rest_e.axis, f"Edge {i} axis mismatch"
        assert abs(orig_e.weight - rest_e.weight) < 1e-10, f"Edge {i} weight mismatch"
        assert orig_e.evidence == rest_e.evidence, f"Edge {i} evidence mismatch"
        assert orig_e.relation == rest_e.relation, f"Edge {i} relation mismatch"
        assert orig_e.differential == rest_e.differential, f"Edge {i} differential mismatch"

    # Morpheme structure equality
    for sid in original._morphemes:
        orig_ms = original._morphemes[sid]
        rest_ms = restored._morphemes[sid]
        assert orig_ms.signifier_id == rest_ms.signifier_id
        assert orig_ms.etymology == rest_ms.etymology
        assert len(orig_ms.morphemes) == len(rest_ms.morphemes)
        for j, (om, rm) in enumerate(zip(orig_ms.morphemes, rest_ms.morphemes)):
            assert om.form == rm.form, f"Morpheme {j} form mismatch"
            assert om.meaning == rm.meaning, f"Morpheme {j} meaning mismatch"
            assert om.lang == rm.lang, f"Morpheme {j} lang mismatch"
            assert om.shared_with == rm.shared_with, f"Morpheme {j} shared_with mismatch"

    # Index integrity: syntagmatic neighbors should work the same
    orig_neighbors = original.syntagmatic_neighbors("中枢")
    rest_neighbors = restored.syntagmatic_neighbors("中枢")
    assert len(orig_neighbors) == len(rest_neighbors), "Syntagmatic neighbors count mismatch"

    print("test_snet_to_dict_from_dict_roundtrip: PASSED")


def test_snet_to_dict_json_serializable():
    """to_dict 输出应可被 JSON 序列化。"""
    snet = _build_test_snet()
    data = snet.to_dict()
    json_str = json.dumps(data, ensure_ascii=False)
    assert len(json_str) > 100
    # Round-trip through JSON
    data2 = json.loads(json_str)
    restored = SNet.from_dict(data2)
    assert len(restored._signifiers) == len(snet._signifiers)
    assert len(restored._edges) == len(snet._edges)
    print("test_snet_to_dict_json_serializable: PASSED")


def test_empty_snet_roundtrip():
    """空 SNet 的 round-trip 应正常工作。"""
    empty = SNet()
    data = empty.to_dict()
    restored = SNet.from_dict(data)
    assert len(restored._signifiers) == 0
    assert len(restored._edges) == 0
    assert len(restored._morphemes) == 0
    print("test_empty_snet_roundtrip: PASSED")


def test_manifest_build_and_match():
    """manifest 构建和匹配逻辑。"""
    from snet_cache import build_manifest, manifests_match

    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        dict_dir = tmpdir / "dicts"
        dict_dir.mkdir()
        corpus_root = tmpdir / "corpora"
        corpus_root.mkdir()

        # Create test files
        (dict_dir / "dict_test.jsonl").write_text('{"term": "test"}\n', encoding="utf-8")
        domain_dir = corpus_root / "test_domain"
        domain_dir.mkdir()
        (domain_dir / "test.txt").write_text("test content", encoding="utf-8")

        m1 = build_manifest(dict_dir, corpus_root, None)
        m2 = build_manifest(dict_dir, corpus_root, None)
        assert manifests_match(m1, m2), "Identical manifests should match"

        # Add a new dict file
        (dict_dir / "dict_new.jsonl").write_text('{"term": "new"}\n', encoding="utf-8")
        m3 = build_manifest(dict_dir, corpus_root, None)
        assert not manifests_match(m1, m3), "Manifests with different files should not match"

    print("test_manifest_build_and_match: PASSED")


def test_diff_manifests():
    """diff_manifests 应正确检测新增/删除。"""
    from snet_cache import build_manifest, diff_manifests

    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        dict_dir = tmpdir / "dicts"
        dict_dir.mkdir()

        (dict_dir / "dict_a.jsonl").write_text('{"term": "a"}\n', encoding="utf-8")
        m_before = build_manifest(dict_dir, None, None)

        (dict_dir / "dict_b.jsonl").write_text('{"term": "b"}\n', encoding="utf-8")
        m_after = build_manifest(dict_dir, None, None)

        diff = diff_manifests(m_before, m_after)
        assert len(diff["new_dicts"]) == 1, f"Expected 1 new dict, got {len(diff['new_dicts'])}"
        assert "dict_b.jsonl" in diff["new_dicts"][0]
        assert len(diff["removed_dicts"]) == 0

    print("test_diff_manifests: PASSED")


def test_cache_save_load_roundtrip():
    """save + load cache 应产生等价 SNet。"""
    import snet_cache

    # Temporarily redirect cache to temp dir
    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        original_cache_dir = snet_cache._CACHE_DIR
        original_cache_path = snet_cache._CACHE_PATH
        original_manifest_path = snet_cache._MANIFEST_PATH

        snet_cache._CACHE_DIR = tmpdir
        snet_cache._CACHE_PATH = tmpdir / "snet_cache.pkl.gz"
        snet_cache._MANIFEST_PATH = tmpdir / "manifest.json"

        try:
            snet = _build_test_snet()
            manifest = {"version": 2, "created": 0, "surface_forms": None, "dictionaries": [], "corpora": []}

            # Save
            cache_path = snet_cache.save_snet_cache(snet, manifest)
            assert cache_path.exists(), "Cache file should exist after save"

            # Load
            loaded_snet, loaded_manifest = snet_cache.load_snet_cache()
            assert loaded_snet is not None, "Should load SNet from cache"
            assert loaded_manifest is not None, "Should load manifest from cache"

            # Verify counts
            assert len(loaded_snet._signifiers) == len(snet._signifiers), \
                f"Signifier count: {len(loaded_snet._signifiers)} != {len(snet._signifiers)}"
            assert len(loaded_snet._edges) == len(snet._edges), \
                f"Edge count: {len(loaded_snet._edges)} != {len(snet._edges)}"
            assert len(loaded_snet._morphemes) == len(snet._morphemes), \
                f"Morpheme count: {len(loaded_snet._morphemes)} != {len(snet._morphemes)}"

            # Verify cache size is reasonable
            cache_size = cache_path.stat().st_size
            assert cache_size > 0, "Cache file should not be empty"
            assert cache_size < 1_000_000, "Test cache should be small"

        finally:
            snet_cache._CACHE_DIR = original_cache_dir
            snet_cache._CACHE_PATH = original_cache_path
            snet_cache._MANIFEST_PATH = original_manifest_path

    print("test_cache_save_load_roundtrip: PASSED")


def test_invalidate_cache():
    """invalidate_cache 应删除缓存文件。"""
    import snet_cache

    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        original_cache_dir = snet_cache._CACHE_DIR
        original_cache_path = snet_cache._CACHE_PATH
        original_manifest_path = snet_cache._MANIFEST_PATH

        snet_cache._CACHE_DIR = tmpdir
        snet_cache._CACHE_PATH = tmpdir / "snet_cache.pkl.gz"
        snet_cache._MANIFEST_PATH = tmpdir / "manifest.json"

        try:
            snet = _build_test_snet()
            manifest = {"version": 2, "created": 0, "surface_forms": None, "dictionaries": [], "corpora": []}

            snet_cache.save_snet_cache(snet, manifest)
            assert snet_cache._CACHE_PATH.exists()
            assert snet_cache._MANIFEST_PATH.exists()

            snet_cache.invalidate_cache()
            assert not snet_cache._CACHE_PATH.exists(), "Cache file should be deleted"
            assert not snet_cache._MANIFEST_PATH.exists(), "Manifest file should be deleted"

        finally:
            snet_cache._CACHE_DIR = original_cache_dir
            snet_cache._CACHE_PATH = original_cache_path
            snet_cache._MANIFEST_PATH = original_manifest_path

    print("test_invalidate_cache: PASSED")


if __name__ == "__main__":
    test_snet_to_dict_from_dict_roundtrip()
    test_snet_to_dict_json_serializable()
    test_empty_snet_roundtrip()
    test_manifest_build_and_match()
    test_diff_manifests()
    test_cache_save_load_roundtrip()
    test_invalidate_cache()
    print("\n=== All tests PASSED ===")
