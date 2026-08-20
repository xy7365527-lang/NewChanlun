"""test_snet_cache.py -- S_net 持久化缓存 round-trip 测试。

验证:
  1. SNet.to_dict() / SNet.from_dict() round-trip 等价
  2. manifest 构建 + 匹配逻辑
  3. 安全 JSONL cache round-trip、摘要与 schema 校验
  4. 恶意旧 pickle 不执行、损坏/超限 cache fail-closed
  5. cache miss/拒绝后的完整重建协议

认识论等级: L1（管线正确性验证）
"""

from __future__ import annotations

import gzip
import hashlib
import json
import pickle
import sys
import tempfile
from contextlib import contextmanager
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

_MISSING = object()


@contextmanager
def _redirect_cache(snet_cache, cache_dir: Path):
    """Redirect every cache artifact to an isolated directory."""
    replacements = {
        "_CACHE_DIR": cache_dir,
        "_CACHE_PATH": cache_dir / "snet_cache.jsonl.gz",
        "_LEGACY_CACHE_PATH": cache_dir / "snet_cache.pkl.gz",
        "_MANIFEST_PATH": cache_dir / "manifest.json",
    }
    originals = {
        name: getattr(snet_cache, name, _MISSING)
        for name in replacements
    }
    for name, value in replacements.items():
        setattr(snet_cache, name, value)
    try:
        yield
    finally:
        for name, value in originals.items():
            if value is _MISSING:
                delattr(snet_cache, name)
            else:
                setattr(snet_cache, name, value)


def _base_manifest() -> dict:
    return {
        "version": 2,
        "created": 0,
        "surface_forms": None,
        "dictionaries": [],
        "corpora": [],
    }


class _TouchOnUnpickle:
    """Benign exploit probe: unsafe deserialization touches a marker file."""

    def __init__(self, marker: Path) -> None:
        self.marker = marker

    def __reduce__(self):
        return Path.touch, (self.marker,)


def _write_raw_jsonl_cache(snet_cache, records: list[dict]) -> None:
    """Write a cache with a matching digest so schema/size checks are exercised."""
    raw = b"".join(
        json.dumps(
            record,
            ensure_ascii=False,
            allow_nan=False,
            sort_keys=True,
            separators=(",", ":"),
        ).encode("utf-8") + b"\n"
        for record in records
    )
    with gzip.open(snet_cache._CACHE_PATH, "wb", compresslevel=4) as stream:
        stream.write(raw)
    footer = records[-1] if records and records[-1].get("kind") == "footer" else {}
    counts = footer.get("counts", {})
    manifest = _base_manifest()
    manifest["cache_info"] = {
        "format": "snet-jsonl-gzip",
        "schema_version": 1,
        "sha256": hashlib.sha256(snet_cache._CACHE_PATH.read_bytes()).hexdigest(),
        "compressed_bytes": snet_cache._CACHE_PATH.stat().st_size,
        "uncompressed_bytes": len(raw),
        "record_count": len(records),
        "n_signifiers": counts.get("signifiers", 0),
        "n_edges": counts.get("edges", 0),
        "n_morphemes": counts.get("morphemes", 0),
        "n_hyperedges": counts.get("hyperedges", 0),
    }
    snet_cache._MANIFEST_PATH.write_text(
        json.dumps(manifest),
        encoding="utf-8",
    )


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
    """save + load cache 应产生等价 SNet，并写入完整性元数据。"""
    import snet_cache

    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        snet = _build_test_snet()
        cache_path = snet_cache.save_snet_cache(snet, _base_manifest())
        assert cache_path == snet_cache._CACHE_PATH
        assert cache_path.name == "snet_cache.jsonl.gz"

        persisted_manifest = json.loads(snet_cache._MANIFEST_PATH.read_text(encoding="utf-8"))
        cache_info = persisted_manifest["cache_info"]
        assert cache_info["format"] == "snet-jsonl-gzip"
        assert cache_info["schema_version"] == 1
        assert cache_info["sha256"] == hashlib.sha256(cache_path.read_bytes()).hexdigest()
        assert cache_info["compressed_bytes"] == cache_path.stat().st_size
        assert cache_info["uncompressed_bytes"] > cache_info["compressed_bytes"]

        loaded_snet, loaded_manifest = snet_cache.load_snet_cache()
        assert loaded_snet is not None, "Should load SNet from cache"
        assert loaded_manifest is not None, "Should load manifest from cache"
        assert len(loaded_snet._signifiers) == len(snet._signifiers)
        assert len(loaded_snet._edges) == len(snet._edges)
        assert len(loaded_snet._morphemes) == len(snet._morphemes)

        cache_size = cache_path.stat().st_size
        assert 0 < cache_size < 1_000_000

    print("test_cache_save_load_roundtrip: PASSED")


def test_invalidate_cache():
    """invalidate_cache 应删除安全缓存、旧 pickle 和 manifest。"""
    import snet_cache

    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        snet_cache.save_snet_cache(_build_test_snet(), _base_manifest())
        snet_cache._LEGACY_CACHE_PATH.write_bytes(b"retired")

        snet_cache.invalidate_cache()
        assert not snet_cache._CACHE_PATH.exists()
        assert not snet_cache._LEGACY_CACHE_PATH.exists()
        assert not snet_cache._MANIFEST_PATH.exists()

    print("test_invalidate_cache: PASSED")


def test_legacy_pickle_is_retired_without_deserialization():
    """旧 pickle 只失效删除；任何构造期 payload 都不得执行。"""
    import snet_cache

    has_legacy_path = hasattr(snet_cache, "_LEGACY_CACHE_PATH")
    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        marker = Path(tmpdir) / "payload-executed"
        with gzip.open(snet_cache._LEGACY_CACHE_PATH, "wb") as stream:
            pickle.dump(_TouchOnUnpickle(marker), stream)
        snet_cache._MANIFEST_PATH.write_text(json.dumps(_base_manifest()), encoding="utf-8")

        # On the vulnerable implementation the legacy artifact was the active path.
        if not has_legacy_path:
            snet_cache._CACHE_PATH = snet_cache._LEGACY_CACHE_PATH

        loaded, used_cache = snet_cache.try_load_cached_snet(None, None, None)
        assert loaded is None
        assert used_cache is False
        assert not marker.exists(), "legacy cache payload executed"
        assert not snet_cache._LEGACY_CACHE_PATH.exists()
        assert not snet_cache._MANIFEST_PATH.exists()


def test_cache_corruption_fails_closed_and_requests_full_rebuild():
    """摘要不匹配时不解析 cache，并回退到完整重建。"""
    import snet_cache

    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        snet_cache.save_snet_cache(_build_test_snet(), _base_manifest())
        damaged = bytearray(snet_cache._CACHE_PATH.read_bytes())
        damaged[len(damaged) // 2] ^= 0x01
        snet_cache._CACHE_PATH.write_bytes(damaged)

        loaded, used_cache = snet_cache.try_load_cached_snet(None, None, None)
        assert loaded is None
        assert used_cache is False


def test_cache_schema_rejects_unknown_record_kind():
    """摘要正确也不能绕过严格 JSONL schema。"""
    import snet_cache

    records = [
        {"kind": "header", "format": "snet-jsonl", "schema_version": 1},
        {"kind": "execute", "command": "touch /tmp/never"},
        {
            "kind": "footer",
            "counts": {"signifiers": 0, "edges": 0, "morphemes": 0, "hyperedges": 0},
        },
    ]
    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        _write_raw_jsonl_cache(snet_cache, records)
        loaded, manifest = snet_cache.load_snet_cache()
        assert loaded is None
        assert manifest is None


def test_cache_path_swap_after_digest_cannot_change_loaded_bytes():
    """摘要校验与解析必须绑定同一已打开文件，消除路径替换 TOCTOU。"""
    import snet_cache

    def records(identifier: str) -> list[dict]:
        return [
            {"kind": "header", "format": "snet-jsonl", "schema_version": 1},
            {
                "kind": "signifier",
                "key": identifier,
                "value": {
                    "id": identifier,
                    "surface_forms": [],
                    "source": "test",
                    "lang": "",
                    "domain": "",
                },
            },
            {
                "kind": "footer",
                "counts": {"signifiers": 1, "edges": 0, "morphemes": 0, "hyperedges": 0},
            },
        ]

    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        _write_raw_jsonl_cache(snet_cache, records("safe"))
        original_manifest = snet_cache._MANIFEST_PATH.read_bytes()
        replacement_path = Path(tmpdir) / "replacement.jsonl.gz"
        active_path = snet_cache._CACHE_PATH
        snet_cache._CACHE_PATH = replacement_path
        _write_raw_jsonl_cache(snet_cache, records("evil"))
        snet_cache._CACHE_PATH = active_path
        snet_cache._MANIFEST_PATH.write_bytes(original_manifest)

        original_hash = snet_cache._sha256_file

        def hash_then_swap(path: Path) -> str:
            digest = original_hash(path)
            path.write_bytes(replacement_path.read_bytes())
            return digest

        snet_cache._sha256_file = hash_then_swap
        try:
            loaded, manifest = snet_cache.load_snet_cache()
        finally:
            snet_cache._sha256_file = original_hash

        assert loaded is not None
        assert manifest is not None
        assert "safe" in loaded._signifiers
        assert "evil" not in loaded._signifiers


def test_cache_uncompressed_size_limit_blocks_compression_bomb_shape():
    """即使压缩体积很小，解压上限也必须 fail-closed。"""
    import snet_cache

    records = [
        {"kind": "header", "format": "snet-jsonl", "schema_version": 1},
        {
            "kind": "signifier",
            "key": "x",
            "value": {
                "id": "x",
                "surface_forms": ["a" * 4096],
                "source": "test",
                "lang": "",
                "domain": "",
            },
        },
        {
            "kind": "footer",
            "counts": {"signifiers": 1, "edges": 0, "morphemes": 0, "hyperedges": 0},
        },
    ]
    original_limit = getattr(snet_cache, "_MAX_UNCOMPRESSED_BYTES", _MISSING)
    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        _write_raw_jsonl_cache(snet_cache, records)
        snet_cache._MAX_UNCOMPRESSED_BYTES = 512
        try:
            loaded, manifest = snet_cache.load_snet_cache()
        finally:
            if original_limit is _MISSING:
                delattr(snet_cache, "_MAX_UNCOMPRESSED_BYTES")
            else:
                snet_cache._MAX_UNCOMPRESSED_BYTES = original_limit
        assert loaded is None
        assert manifest is None


def test_cache_miss_requests_full_rebuild():
    """无 cache 时保持现有完整重建协议。"""
    import snet_cache

    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        loaded, used_cache = snet_cache.try_load_cached_snet(None, None, None)
        assert loaded is None
        assert used_cache is False


if __name__ == "__main__":
    test_snet_to_dict_from_dict_roundtrip()
    test_snet_to_dict_json_serializable()
    test_empty_snet_roundtrip()
    test_manifest_build_and_match()
    test_diff_manifests()
    test_cache_save_load_roundtrip()
    test_invalidate_cache()
    test_legacy_pickle_is_retired_without_deserialization()
    test_cache_corruption_fails_closed_and_requests_full_rebuild()
    test_cache_schema_rejects_unknown_record_kind()
    test_cache_path_swap_after_digest_cannot_change_loaded_bytes()
    test_cache_uncompressed_size_limit_blocks_compression_bomb_shape()
    test_cache_miss_requests_full_rebuild()
    print("\n=== All tests PASSED ===")
