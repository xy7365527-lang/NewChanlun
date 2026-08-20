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
import os
import sqlite3
import sys
import tempfile
import types
import pytest
from contextlib import contextmanager
from pathlib import Path

# Add topological-computation to path
script_dir = Path(__file__).resolve().parent
sys.path.insert(0, str(script_dir))

from signifier_net import (
    SNet, Signifier, SignifierEdge, AxisType,
    Morpheme, MorphemeStructure,
)
from cooccurrence_hyperedge import CooccurrenceHyperedge


# ---------------------------------------------------------------------------
# Test helpers
# ---------------------------------------------------------------------------

_MISSING = object()


@contextmanager
def _redirect_cache(snet_cache, cache_dir: Path, *, resolve: bool = True):
    """Redirect every cache artifact to an isolated directory."""
    if resolve:
        cache_dir = cache_dir.resolve()
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
        Signifier(id="Aufgabe", surface_forms=(), source="dictionary", lang="de", domain="hegel"),
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

    snet = snet.add_hyperedge(CooccurrenceHyperedge(
        vertices=frozenset({"中枢", "走势"}),
        source="test-corpus.md",
        domain="chanlun",
        timestamp="2026-08-20T00:00:00Z",
        ingest_param_refs=("snet_param:window",),
        evidence_tag="corpus:chanlun:test-corpus.md",
    ))

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
        assert loaded_snet.hyperedges == snet.hyperedges

        cache_size = cache_path.stat().st_size
        assert 0 < cache_size < 1_000_000

    print("test_cache_save_load_roundtrip: PASSED")


def test_duplicate_hyperedges_roundtrip_through_jsonl_sqlite_and_lazy():
    """字段完全相同的超边是两个有序事件，所有持久化路径必须保持 2→2。"""
    import snet_cache
    from snet_lazy import SNetLazy
    from snet_persistence import SNetPersistence

    original = _build_test_snet()
    duplicate = original.hyperedges[0]
    original = original.add_hyperedge(duplicate)

    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        snet_cache.save_snet_cache(original, _base_manifest())
        jsonl, _manifest = snet_cache.load_snet_cache()
        assert jsonl is not None
        assert jsonl.hyperedges == [duplicate, duplicate]

    with tempfile.TemporaryDirectory() as tmpdir:
        persistence = SNetPersistence(Path(tmpdir) / "snet.db")
        try:
            persistence.save_full(original)
            full = persistence.load_full()
            lazy = SNetLazy.from_persistence(persistence)
            assert full is not None
            assert full.hyperedges == [duplicate, duplicate]
            assert lazy.hyperedges == [duplicate, duplicate]
        finally:
            persistence.close()


def test_jsonl_duplicate_vertices_inside_one_hyperedge_remain_invalid():
    """超边事件可重复，但单条 hyperedge.vertices 内部仍不得重复。"""
    import snet_cache

    records = [
        {"kind": "header", "format": "snet-jsonl", "schema_version": 1},
        {
            "kind": "signifier", "key": "A",
            "value": {"id": "A", "surface_forms": [], "source": "", "lang": "", "domain": ""},
        },
        {
            "kind": "hyperedge",
            "value": {
                "vertices": ["A", "A"], "source": "corpus.md", "domain": "test",
                "timestamp": "0", "ingest_param_refs": [], "evidence_tag": "",
            },
        },
        {
            "kind": "footer",
            "counts": {"signifiers": 1, "edges": 0, "morphemes": 0, "hyperedges": 1},
        },
    ]
    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        _write_raw_jsonl_cache(snet_cache, records)
        assert snet_cache.load_snet_cache() == (None, None)


def test_lazy_cooccurrence_matches_eager_hyperedge_priority_and_multiplicity():
    """lazy、SQLite 全量回读与 eager 都先数共同超边，零共同超边才查成对边。"""
    from snet_lazy import SNetLazy
    from snet_persistence import SNetPersistence

    signifiers = {
        sid: Signifier(id=sid)
        for sid in ("A", "B", "C", "D")
    }
    pairwise = SignifierEdge(
        source="C", target="D", axis=AxisType.SYNTAGMATIC, weight=7.0,
    )
    base = SNet(signifiers=signifiers, edges=[pairwise])
    hyperedge = CooccurrenceHyperedge(
        vertices=frozenset({"A", "B"}),
        source="corpus.md",
        domain="test",
        timestamp="2026-08-20T00:00:00Z",
    )

    with tempfile.TemporaryDirectory() as tmpdir:
        persistence = SNetPersistence(Path(tmpdir) / "snet.db")
        try:
            for count, eager in (
                (1, base.add_hyperedge(hyperedge)),
                (2, base.add_hyperedges([hyperedge, hyperedge])),
            ):
                persistence.save_full(eager)
                sqlite_eager = persistence.load_full()
                lazy = SNetLazy.from_persistence(persistence)
                assert sqlite_eager is not None
                assert eager.cooccurrence_weight("A", "B") == float(count)
                assert sqlite_eager.cooccurrence_weight("A", "B") == float(count)
                assert lazy.cooccurrence_weight("A", "B") == float(count)
                assert eager.cooccurrence_weight("B", "A") == float(count)
                assert lazy.cooccurrence_weight("B", "A") == float(count)
                assert eager.cooccurrence_weight("C", "D") == 7.0
                assert sqlite_eager.cooccurrence_weight("C", "D") == 7.0
                assert lazy.cooccurrence_weight("C", "D") == 7.0
        finally:
            persistence.close()


def test_lazy_runtime_hyperedge_add_invalidates_all_ordered_pair_caches():
    """已缓存的零值必须在运行时追加一条/重复两条超边后更新为 1/2。"""
    from snet_lazy import SNetLazy
    from snet_persistence import SNetPersistence

    signifiers = {sid: Signifier(id=sid) for sid in ("A", "B", "C")}
    hyperedge = CooccurrenceHyperedge(
        vertices=frozenset(signifiers), source="runtime", domain="test", timestamp="0",
    )
    ordered_pairs = [
        ("A", "B"), ("B", "A"),
        ("A", "C"), ("C", "A"),
        ("B", "C"), ("C", "B"),
    ]

    with tempfile.TemporaryDirectory() as tmpdir:
        persistence = SNetPersistence(Path(tmpdir) / "snet.db")
        try:
            persistence.save_full(SNet(signifiers=signifiers))
            lazy = SNetLazy.from_persistence(persistence)
            assert [lazy.cooccurrence_weight(a, b) for a, b in ordered_pairs] == [0.0] * 6

            lazy.add_hyperedge(hyperedge)
            assert [lazy.cooccurrence_weight(a, b) for a, b in ordered_pairs] == [1.0] * 6

            lazy.add_hyperedges([hyperedge])
            assert [lazy.cooccurrence_weight(a, b) for a, b in ordered_pairs] == [2.0] * 6
            persisted = persistence.load_full()
            assert persisted is not None
            assert persisted.cooccurrence_weight("A", "B") == 2.0
        finally:
            persistence.close()


def test_corpus_hyperedge_only_cooccurrence_survives_lazy_persistence():
    """语料摄入只产超边时，切换 lazy 后共现权重不得归零。"""
    from signifier_net_ingest import ingest_text_passage_batch
    from snet_lazy import SNetLazy
    from snet_persistence import SNetPersistence

    signifiers = {
        sid: Signifier(id=sid, surface_forms=(sid,))
        for sid in ("走势", "级别", "中枢")
    }
    eager, _log = ingest_text_passage_batch(
        SNet(signifiers=signifiers),
        ["走势由多个级别的中枢构成。"],
        "chanlun",
        "corpus.md",
    )
    assert eager.edges == []
    assert eager.cooccurrence_weight("走势", "级别") == 1.0

    with tempfile.TemporaryDirectory() as tmpdir:
        persistence = SNetPersistence(Path(tmpdir) / "snet.db")
        try:
            persistence.save_full(eager)
            lazy = SNetLazy.from_persistence(persistence)
            assert lazy.cooccurrence_weight("走势", "级别") == 1.0
        finally:
            persistence.close()


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

        original_hash = snet_cache._sha256_stream
        swapped = False

        def hash_then_swap(stream, hard_limit) -> str:
            nonlocal swapped
            digest = original_hash(stream, hard_limit)
            os.replace(replacement_path, active_path)
            swapped = True
            return digest

        snet_cache._sha256_stream = hash_then_swap
        try:
            loaded, manifest = snet_cache.load_snet_cache()
        finally:
            snet_cache._sha256_stream = original_hash

        assert swapped, "test did not traverse production _sha256_stream"
        assert loaded is None
        assert manifest is None


def test_cache_uncompressed_size_limit_blocks_compression_bomb_shape(monkeypatch):
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
        manifest = json.loads(snet_cache._MANIFEST_PATH.read_text(encoding="utf-8"))
        # Lie low versus the actual stream, but keep the lie above the runtime
        # cap installed *after* manifest validation.  This proves the live
        # decompressor enforces the real-byte cap instead of trusting metadata.
        manifest["cache_info"]["uncompressed_bytes"] = 1024
        snet_cache._MANIFEST_PATH.write_text(json.dumps(manifest), encoding="utf-8")
        original_hash = snet_cache._sha256_stream

        def hash_then_lower_runtime_cap(stream, hard_limit):
            digest = original_hash(stream, hard_limit)
            snet_cache._MAX_UNCOMPRESSED_BYTES = 512
            return digest

        monkeypatch.setattr(
            snet_cache,
            "_sha256_stream",
            hash_then_lower_runtime_cap,
        )
        try:
            loaded, manifest = snet_cache.load_snet_cache()
        finally:
            if original_limit is _MISSING:
                delattr(snet_cache, "_MAX_UNCOMPRESSED_BYTES")
            else:
                snet_cache._MAX_UNCOMPRESSED_BYTES = original_limit
        assert loaded is None
        assert manifest is None


def test_loader_memory_budget_counts_decoded_list_items():
    """合法的巨大数组也必须纳入全局解码对象预算。"""
    import snet_cache

    records = [
        {"kind": "header", "format": "snet-jsonl", "schema_version": 1},
        {
            "kind": "signifier", "key": "x",
            "value": {
                "id": "x",
                "surface_forms": [f"surface-{index}" for index in range(1_000)],
                "source": "test", "lang": "", "domain": "",
            },
        },
        {"kind": "footer", "counts": {"signifiers": 1, "edges": 0, "morphemes": 0, "hyperedges": 0}},
    ]
    original_budget = snet_cache._MAX_ESTIMATED_LOAD_BYTES
    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        _write_raw_jsonl_cache(snet_cache, records)
        info = json.loads(snet_cache._MANIFEST_PATH.read_text(encoding="utf-8"))["cache_info"]
        original_hash = snet_cache._sha256_stream

        def hash_then_install_budget(stream, hard_limit):
            digest = original_hash(stream, hard_limit)
            # The old raw + 1 KiB/record estimate fits; decoded list items must not.
            snet_cache._MAX_ESTIMATED_LOAD_BYTES = (
                info["uncompressed_bytes"] + info["record_count"] * 1024 + 100
            )
            return digest

        snet_cache._sha256_stream = hash_then_install_budget
        try:
            assert snet_cache.load_snet_cache() == (None, None)
        finally:
            snet_cache._sha256_stream = original_hash
            snet_cache._MAX_ESTIMATED_LOAD_BYTES = original_budget


def test_known_production_shape_fits_bounded_cache_budget(monkeypatch):
    """已知 263K signifiers + 778K edges 必须能通过有界 cache round-trip。"""
    import snet_cache

    # .chanlun/genealogy/settled/456-*:35 records 263K signifiers, while
    # snet_cache.py's measured production profile records about 778K edges.
    # Scale the workload and every aggregate byte/record budget together so
    # this remains a fast public-seam regression instead of a million-record
    # test or a brittle assertion about one private constant.
    scale = 1_000
    n_signifiers = 263_000 // scale
    n_edges = 778_000 // scale
    identifiers = [f"s{index}" for index in range(n_signifiers)]
    signifiers = {
        identifier: Signifier(id=identifier, surface_forms=(identifier,))
        for identifier in identifiers
    }
    edges = [
        SignifierEdge(
            source=identifiers[index % n_signifiers],
            target=identifiers[index // n_signifiers],
            axis=AxisType.SYNTAGMATIC,
            weight=1.0,
        )
        for index in range(n_edges)
    ]
    snet = SNet(signifiers=signifiers, edges=edges)

    for limit_name in (
        "_MAX_COMPRESSED_BYTES",
        "_MAX_UNCOMPRESSED_BYTES",
        "_MAX_RECORDS",
        "_MAX_ESTIMATED_LOAD_BYTES",
    ):
        monkeypatch.setattr(
            snet_cache,
            limit_name,
            getattr(snet_cache, limit_name) // scale,
        )

    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        snet_cache.save_snet_cache(snet, _base_manifest())
        stored = json.loads(snet_cache._MANIFEST_PATH.read_text(encoding="utf-8"))
        assert stored["cache_info"]["record_count"] == n_signifiers + n_edges + 2
        loaded, _ = snet_cache.load_snet_cache()
        assert loaded is not None
        assert len(loaded._signifiers) == n_signifiers
        assert len(loaded._edges) == n_edges


def test_loader_rejects_cache_directory_manifest_and_cache_symlinks():
    """生产 loader 不得跟随缓存目录、manifest 或 cache 的符号链接。"""
    import snet_cache

    with tempfile.TemporaryDirectory() as tmpdir:
        root = Path(tmpdir)
        real = root / "real"
        real.mkdir()
        with _redirect_cache(snet_cache, real):
            snet_cache.save_snet_cache(_build_test_snet(), _base_manifest())

            manifest_real = real / "manifest.real"
            snet_cache._MANIFEST_PATH.rename(manifest_real)
            snet_cache._MANIFEST_PATH.symlink_to(manifest_real)
            assert snet_cache.load_snet_cache() == (None, None)
            snet_cache._MANIFEST_PATH.unlink()
            manifest_real.rename(snet_cache._MANIFEST_PATH)

            cache_real = real / "cache.real.gz"
            snet_cache._CACHE_PATH.rename(cache_real)
            snet_cache._CACHE_PATH.symlink_to(cache_real)
            assert snet_cache.load_snet_cache() == (None, None)

        linked = root / "linked"
        linked.symlink_to(real, target_is_directory=True)
        with _redirect_cache(snet_cache, linked, resolve=False):
            assert snet_cache.load_snet_cache() == (None, None)


def test_loader_and_invalidator_reject_ancestor_directory_symlink():
    """cache dir 的任一祖先分量都不得跟随 symlink。"""
    import snet_cache

    with tempfile.TemporaryDirectory() as tmpdir:
        root = Path(tmpdir)
        real_parent = root / "real-parent"
        real_cache = real_parent / "cache"
        real_cache.mkdir(parents=True)
        with _redirect_cache(snet_cache, real_cache):
            snet_cache.save_snet_cache(_build_test_snet(), _base_manifest())

        linked_parent = root / "linked-parent"
        linked_parent.symlink_to(real_parent, target_is_directory=True)
        with _redirect_cache(snet_cache, linked_parent / "cache", resolve=False):
            assert snet_cache.load_snet_cache() == (None, None)
            snet_cache.invalidate_cache()

        assert (real_cache / "snet_cache.jsonl.gz").is_file()
        assert (real_cache / "manifest.json").is_file()


def test_loader_passes_fstat_size_as_hard_stream_window(monkeypatch):
    """loader 必须把同一 fd 的初始尺寸传给生产 hash 读取窗。"""
    import snet_cache

    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        snet_cache.save_snet_cache(_build_test_snet(), _base_manifest())
        expected_size = snet_cache._CACHE_PATH.stat().st_size
        original_hash = snet_cache._sha256_stream
        observed = []

        def require_bound(stream, hard_limit):
            observed.append(hard_limit)
            return original_hash(stream, hard_limit)

        monkeypatch.setattr(snet_cache, "_sha256_stream", require_bound)
        loaded, manifest = snet_cache.load_snet_cache()
        assert observed == [expected_size]
        assert loaded is not None
        assert manifest is not None


def test_manifest_path_swap_after_fd_open_cannot_change_validated_bytes(monkeypatch):
    """manifest 的校验和解析绑定同一 fd，路径替换不能注入另一份字节。"""
    import snet_cache

    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        snet_cache.save_snet_cache(_build_test_snet(), _base_manifest())
        original_inode = snet_cache._MANIFEST_PATH.stat().st_ino
        replacement = Path(tmpdir) / "replacement-manifest.json"
        replacement.write_text("{}", encoding="utf-8")
        real_read = os.read
        swapped = False

        def read_then_swap(fd, size):
            nonlocal swapped
            data = real_read(fd, size)
            if not swapped and os.fstat(fd).st_ino == original_inode:
                os.replace(replacement, snet_cache._MANIFEST_PATH)
                swapped = True
            return data

        monkeypatch.setattr(os, "read", read_then_swap)
        loaded, manifest = snet_cache.load_snet_cache()
        assert swapped, "test did not swap manifest in the production fd-read window"
        assert (loaded, manifest) == (None, None)


def test_schema_rejects_bool_integer_duplicate_ids_and_references():
    """严格 schema 拒绝 bool 冒充 int，以及重复 ID/引用。"""
    import snet_cache

    base = [
        {"kind": "header", "format": "snet-jsonl", "schema_version": 1},
        {
            "kind": "signifier", "key": "a",
            "value": {"id": "a", "surface_forms": [], "source": "t", "lang": "", "domain": ""},
        },
        {"kind": "footer", "counts": {"signifiers": 1, "edges": 0, "morphemes": 0, "hyperedges": 0}},
    ]
    variants = []
    bool_header = json.loads(json.dumps(base))
    bool_header[0]["schema_version"] = True
    variants.append(bool_header)
    duplicate_id = json.loads(json.dumps(base))
    duplicate_id.insert(2, {
        "kind": "signifier", "key": "b",
        "value": {"id": "a", "surface_forms": [], "source": "t", "lang": "", "domain": ""},
    })
    duplicate_id[-1]["counts"]["signifiers"] = 2
    variants.append(duplicate_id)
    duplicate_ref = json.loads(json.dumps(base))
    duplicate_ref[1]["value"]["surface_forms"] = ["A", "A"]
    variants.append(duplicate_ref)

    for records in variants:
        with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
            _write_raw_jsonl_cache(snet_cache, records)
            assert snet_cache.load_snet_cache() == (None, None)


def test_schema_rejects_dangling_morpheme_shared_with_reference():
    """Morpheme.shared_with 是 signifier ID 引用，不得悬空。"""
    import snet_cache

    records = [
        {"kind": "header", "format": "snet-jsonl", "schema_version": 1},
        {
            "kind": "signifier", "key": "a",
            "value": {"id": "a", "surface_forms": [], "source": "t", "lang": "", "domain": ""},
        },
        {
            "kind": "morpheme", "key": "a",
            "value": {
                "signifier_id": "a",
                "morphemes": [
                    {"form": "a", "meaning": "a", "lang": "", "shared_with": ["missing"]},
                ],
                "etymology": "",
            },
        },
        {"kind": "footer", "counts": {"signifiers": 1, "edges": 0, "morphemes": 1, "hyperedges": 0}},
    ]
    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        _write_raw_jsonl_cache(snet_cache, records)
        assert snet_cache.load_snet_cache() == (None, None)


def test_manifest_rejects_duplicate_source_paths():
    import snet_cache

    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        snet_cache.save_snet_cache(_build_test_snet(), _base_manifest())
        manifest = json.loads(snet_cache._MANIFEST_PATH.read_text(encoding="utf-8"))
        entry = {"path": "/tmp/a.jsonl", "hash": "a", "mtime": 0}
        manifest["dictionaries"] = [entry, dict(entry)]
        snet_cache._MANIFEST_PATH.write_text(json.dumps(manifest), encoding="utf-8")
        assert snet_cache.load_snet_cache() == (None, None)


def test_sqlite_and_lazy_hyperedge_roundtrip_is_lossless():
    """生产 SQLite→lazy 路径必须保持 hyperedge 1→1。"""
    from snet_lazy import SNetLazy
    from snet_persistence import SNetPersistence, encode_hyperedge_row

    with tempfile.TemporaryDirectory() as tmpdir:
        persistence = SNetPersistence(Path(tmpdir) / "snet.db")
        try:
            contract_hyperedge = CooccurrenceHyperedge(
                vertices=frozenset({"走势", "中枢"}),
                source="编码契约.md",
                domain="测试",
                timestamp="2026-08-20T12:34:56Z",
                ingest_param_refs=("snet_param:z", "snet_param:a"),
                evidence_tag="语料:编码契约",
            )
            expected_row = (
                '["中枢", "走势"]',
                "编码契约.md",
                "测试",
                "2026-08-20T12:34:56Z",
                '["snet_param:z", "snet_param:a"]',
                "语料:编码契约",
            )
            assert encode_hyperedge_row(contract_hyperedge) == expected_row

            original = _build_test_snet().add_hyperedge(contract_hyperedge)
            persistence.save_full(original)
            full = persistence.load_full()
            lazy = SNetLazy.from_persistence(persistence)
            assert full is not None
            assert full.hyperedges == original.hyperedges
            assert lazy.hyperedges == original.hyperedges

            with sqlite3.connect(persistence.db_path) as reader:
                full_row = reader.execute(
                    "SELECT vertices_json, source, domain, timestamp, "
                    "ingest_param_refs_json, evidence_tag FROM hyperedges ORDER BY id DESC LIMIT 1"
                ).fetchone()
            assert full_row == expected_row

            assert persistence.save_incremental(new_hyperedges=[contract_hyperedge]) == 1
            with sqlite3.connect(persistence.db_path) as reader:
                incremental_row = reader.execute(
                    "SELECT vertices_json, source, domain, timestamp, "
                    "ingest_param_refs_json, evidence_tag FROM hyperedges ORDER BY id DESC LIMIT 1"
                ).fetchone()
            assert incremental_row == expected_row
            assert incremental_row == full_row
            assert persistence.load_hyperedges()[-1] == contract_hyperedge
        finally:
            persistence.close()


def test_sqlite_hyperedge_schema_is_backward_compatible_and_rejects_malformed_json():
    from snet_persistence import SNetPersistence

    with tempfile.TemporaryDirectory() as tmpdir:
        db_path = Path(tmpdir) / "legacy.db"
        sqlite3.connect(db_path).execute(
            "CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL)"
        ).connection.close()
        persistence = SNetPersistence(db_path)
        try:
            assert persistence.load_hyperedges() == []
            persistence._conn.execute(
                "INSERT INTO hyperedges "
                "(vertices_json, source, domain, timestamp, ingest_param_refs_json, evidence_tag) "
                "VALUES (?, ?, ?, ?, ?, ?)",
                ('["a", "a"]', "source", "domain", "time", "[]", "evidence"),
            )
            with pytest.raises(ValueError, match="duplicate hyperedge vertices"):
                persistence.load_hyperedges()
        finally:
            persistence.close()


@pytest.mark.parametrize("corruption", ["ghost_hyperedge", "invalid_axis"])
def test_sqlite_lazy_validation_rejects_corrupt_graph_before_publish(corruption):
    """SQLite lazy 发布前必须流式验证引用和 edge schema。"""
    from snet_lazy import SNetLazy
    from snet_persistence import SNetPersistence

    with tempfile.TemporaryDirectory() as tmpdir:
        persistence = SNetPersistence(Path(tmpdir) / "snet.db")
        try:
            persistence.save_full(_build_test_snet())
            if corruption == "ghost_hyperedge":
                persistence._conn.execute(
                    "INSERT INTO hyperedges "
                    "(vertices_json, source, domain, timestamp, ingest_param_refs_json, evidence_tag) "
                    "VALUES (?, ?, ?, ?, ?, ?)",
                    ('["missing"]', "source", "domain", "time", "[]", "evidence"),
                )
            else:
                persistence._conn.execute(
                    "INSERT INTO edges "
                    "(source, target, axis, weight, evidence, relation, differential) "
                    "VALUES (?, ?, ?, ?, ?, ?, ?)",
                    ("中枢", "走势", "evil", 1.0, "", "", ""),
                )
            with pytest.raises((ValueError, TypeError)):
                SNetLazy.from_persistence(persistence)
        finally:
            persistence.close()


def test_daemon_healthy_sqlite_early_return_still_retires_legacy_pickle(monkeypatch):
    """生产 bootstrap 的健康 SQLite 快路径也必须只 unlink 旧 pickle。"""
    import daemon
    import snet_cache
    import snet_lazy
    import snet_persistence

    class Persistence:
        def stats(self): return {"signifiers": 1, "edges": 0}
        def manifest_valid(self, manifest): return True
        def load_signifiers(self): return {"fresh": Signifier(id="fresh")}
        def load_morphemes(self): return {}
        def load_hyperedges(self): return []

    monkeypatch.setattr(snet_persistence, "SNetPersistence", Persistence)
    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        snet_cache._LEGACY_CACHE_PATH.write_bytes(b"pickle")
        target = types.SimpleNamespace(k_active=None, snet=SNet(), _snet_persistence=None)
        daemon.TopologicalDaemon._bootstrap_snet(target)
        assert not snet_cache._LEGACY_CACHE_PATH.exists()


def test_daemon_manifest_mismatch_rebuilds_before_publishing_lazy(monkeypatch):
    """旧 SQLite manifest 不匹配时不得先发布 stale lazy，必须完整重建。"""
    import daemon
    import snet_cache
    import snet_lazy
    import snet_persistence
    import snet_bootstrap

    fresh = _build_test_snet()

    class Persistence:
        saved = False
        def __init__(self): self.manifest_writes = 0
        def stats(self): return {"signifiers": 1, "edges": 1}
        def manifest_valid(self, manifest): return False
        def set_manifest(self, manifest): self.manifest_writes += 1
        def save_full(self, snet): self.saved = True
        def load_signifiers(self): return {"stale": Signifier(id="stale")}
        def load_morphemes(self): return {}
        def load_hyperedges(self): return []
        def iter_all_edges(self): return iter(())

    class Lazy:
        @classmethod
        def from_persistence(cls, persistence, **kwargs):
            marker = "fresh" if persistence.saved else "stale"
            return SNet(signifiers={marker: Signifier(id=marker)})

    monkeypatch.setattr(snet_persistence, "SNetPersistence", Persistence)
    monkeypatch.setattr(snet_lazy, "SNetLazy", Lazy)
    monkeypatch.setattr(snet_bootstrap, "bootstrap_snet", lambda **kwargs: (fresh, {}))
    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        target = types.SimpleNamespace(
            k_active=None, snet=SNet(), _snet_persistence=None,
            _ingest_dictionaries=lambda: None,
            _ingest_text_corpora=lambda: None,
            _ingest_dialogue_sessions=lambda: None,
            _ingest_ceremony_texts=lambda: None,
        )
        daemon.TopologicalDaemon._bootstrap_snet(target)
        assert target._snet_persistence.saved
        assert "fresh" in target.snet._signifiers
        assert "stale" not in target.snet._signifiers


def test_daemon_sqlite_validation_failure_skips_jsonl_and_rebuilds(monkeypatch):
    """非空 SQLite 验证失败后必须直达全量重建，不再信任 fallback。"""
    import daemon, snet_cache, snet_persistence, snet_bootstrap

    fresh = SNet(signifiers={"fresh": Signifier(id="fresh")})
    jsonl_called = []

    class Persistence:
        def stats(self): return {"signifiers": 1, "edges": 1}
        def manifest_valid(self, manifest): return False
        def save_full(self, snet): self.saved = snet
        def set_manifest(self, manifest): pass

    def unexpected_jsonl(**kwargs):
        jsonl_called.append(True)
        return SNet(signifiers={"stale": Signifier(id="stale")}), True

    monkeypatch.setattr(snet_persistence, "SNetPersistence", Persistence)
    monkeypatch.setattr(snet_cache, "try_load_cached_snet", unexpected_jsonl)
    monkeypatch.setattr(snet_cache, "save_after_full_ingest", lambda *args, **kwargs: None)
    monkeypatch.setattr(snet_bootstrap, "bootstrap_snet", lambda **kwargs: (fresh, {}))
    target = types.SimpleNamespace(
        k_active=None, snet=SNet(), _snet_persistence=None,
        _ingest_dictionaries=lambda: None, _ingest_text_corpora=lambda: None,
        _ingest_dialogue_sessions=lambda: None, _ingest_ceremony_texts=lambda: None,
    )
    daemon.TopologicalDaemon._bootstrap_snet(target)
    assert jsonl_called == []
    assert "fresh" in target.snet._signifiers


def test_daemon_real_sqlite_edge_validation_failure_rebuilds(monkeypatch):
    """生产 SQLite validator/lazy/daemon 链路遇非法 edge 必须全量重建。"""
    import daemon, snet_cache, snet_persistence, snet_bootstrap

    fresh = SNet(signifiers={"fresh": Signifier(id="fresh")})
    jsonl_called = []
    with tempfile.TemporaryDirectory() as tmpdir:
        persistence = snet_persistence.SNetPersistence(Path(tmpdir) / "snet.db")
        try:
            persistence.save_full(_build_test_snet())
            persistence._conn.execute(
                "INSERT INTO edges "
                "(source, target, axis, weight, evidence, relation, differential) "
                "VALUES (?, ?, ?, ?, ?, ?, ?)",
                ("中枢", "走势", "evil", 1.0, "", "", ""),
            )
            persistence.manifest_valid = lambda manifest: True

            monkeypatch.setattr(snet_persistence, "SNetPersistence", lambda: persistence)
            monkeypatch.setattr(
                snet_cache,
                "try_load_cached_snet",
                lambda **kwargs: (jsonl_called.append(True), True),
            )
            monkeypatch.setattr(snet_cache, "save_after_full_ingest", lambda *args, **kwargs: None)
            monkeypatch.setattr(snet_bootstrap, "bootstrap_snet", lambda **kwargs: (fresh, {}))
            target = types.SimpleNamespace(
                k_active=None, snet=SNet(), _snet_persistence=None,
                _ingest_dictionaries=lambda: None, _ingest_text_corpora=lambda: None,
                _ingest_dialogue_sessions=lambda: None, _ingest_ceremony_texts=lambda: None,
            )
            daemon.TopologicalDaemon._bootstrap_snet(target)
            assert jsonl_called == []
            assert "fresh" in target.snet._signifiers
        finally:
            persistence.close()


def test_manifest_mismatch_forces_full_rebuild_without_incremental_stale(monkeypatch):
    """source manifest 任何不匹配都必须返回 full-rebuild miss。"""
    import snet_cache

    cached = SNet(signifiers={"stale": Signifier(id="stale")})
    old_manifest = _base_manifest()
    new_manifest = _base_manifest()
    new_manifest["dictionaries"] = [{"path": "/new", "hash": "new", "mtime": 0}]
    monkeypatch.setattr(snet_cache, "build_manifest", lambda *args, **kwargs: new_manifest)
    monkeypatch.setattr(snet_cache, "load_snet_cache", lambda: (cached, old_manifest))

    loaded, used_cache = snet_cache.try_load_cached_snet(None, None, None)
    assert loaded is None
    assert used_cache is False


@pytest.mark.parametrize("failure_mode", ["save_full", "set_manifest"])
def test_daemon_jsonl_migration_failure_completes_full_rebuild(monkeypatch, failure_mode):
    """迁移事务任一步失败都不能发布 cache lazy，必须进入完整 bootstrap。"""
    import daemon, snet_cache, snet_lazy, snet_persistence, snet_bootstrap

    cached = SNet(signifiers={"cached": Signifier(id="cached")})
    fresh = SNet(signifiers={"fresh": Signifier(id="fresh")})

    class Persistence:
        def __init__(self): self.save_calls = 0; self.manifest_calls = 0
        def stats(self): return {"signifiers": 0, "edges": 0}
        def save_full(self, snet):
            self.save_calls += 1
            if failure_mode == "save_full" and self.save_calls == 1: raise sqlite3.Error("boom")
            self.saved = snet
        def set_manifest(self, manifest):
            self.manifest_calls += 1
            if failure_mode == "set_manifest" and self.manifest_calls == 1: raise sqlite3.Error("boom")
        def iter_all_edges(self): return iter(())

    class Lazy:
        @classmethod
        def from_persistence(cls, persistence, **kwargs):
            return SNet(signifiers=dict(persistence.saved._signifiers))

    monkeypatch.setattr(snet_persistence, "SNetPersistence", Persistence)
    monkeypatch.setattr(snet_lazy, "SNetLazy", Lazy)
    monkeypatch.setattr(snet_cache, "try_load_cached_snet", lambda **kwargs: (cached, True))
    monkeypatch.setattr(snet_cache, "save_after_full_ingest", lambda *args, **kwargs: None)
    monkeypatch.setattr(snet_bootstrap, "bootstrap_snet", lambda **kwargs: (fresh, {}))
    target = types.SimpleNamespace(
        k_active=None, snet=SNet(), _snet_persistence=None,
        _ingest_dictionaries=lambda: None, _ingest_text_corpora=lambda: None,
        _ingest_dialogue_sessions=lambda: None, _ingest_ceremony_texts=lambda: None,
    )
    target._migrate_snet_to_sqlite = types.MethodType(
        daemon.TopologicalDaemon._migrate_snet_to_sqlite, target
    )
    daemon.TopologicalDaemon._bootstrap_snet(target)
    assert "fresh" in target.snet._signifiers
    assert "cached" not in target.snet._signifiers


def test_daemon_full_ingest_sqlite_failure_keeps_complete_in_memory_snet(monkeypatch):
    """完整摄入后的 SQLite 保存失败不得切到空/半成品 lazy。"""
    import daemon, snet_cache, snet_lazy, snet_persistence, snet_bootstrap
    fresh = _build_test_snet()

    class Persistence:
        def stats(self): return {"signifiers": 0, "edges": 0}
        def save_full(self, snet): raise sqlite3.Error("disk full")

    class Lazy:
        @classmethod
        def from_persistence(cls, *args, **kwargs):
            raise AssertionError("lazy published after failed SQLite save")

    monkeypatch.setattr(snet_persistence, "SNetPersistence", Persistence)
    monkeypatch.setattr(snet_lazy, "SNetLazy", Lazy)
    monkeypatch.setattr(snet_cache, "try_load_cached_snet", lambda **kwargs: (None, False))
    monkeypatch.setattr(snet_cache, "save_after_full_ingest", lambda *args, **kwargs: None)
    monkeypatch.setattr(snet_bootstrap, "bootstrap_snet", lambda **kwargs: (fresh, {}))
    target = types.SimpleNamespace(
        k_active=None, snet=SNet(), _snet_persistence=None,
        _ingest_dictionaries=lambda: None, _ingest_text_corpora=lambda: None,
        _ingest_dialogue_sessions=lambda: None, _ingest_ceremony_texts=lambda: None,
    )
    daemon.TopologicalDaemon._bootstrap_snet(target)
    assert target.snet is fresh


def test_daemon_invalid_real_jsonl_cache_completes_full_rebuild(monkeypatch):
    """真实 loader 摘要验证失败后，生产 bootstrap 必须落到 fresh 全量重建。"""
    import daemon, snet_cache, snet_lazy, snet_persistence, snet_bootstrap
    fresh = SNet(signifiers={"fresh": Signifier(id="fresh")})

    class Persistence:
        def stats(self): return {"signifiers": 0, "edges": 0}
        def save_full(self, snet): self.saved = snet
        def set_manifest(self, manifest): pass
        def iter_all_edges(self): return iter(())

    class Lazy:
        @classmethod
        def from_persistence(cls, persistence, **kwargs): return persistence.saved

    monkeypatch.setattr(snet_persistence, "SNetPersistence", Persistence)
    monkeypatch.setattr(snet_lazy, "SNetLazy", Lazy)
    monkeypatch.setattr(snet_bootstrap, "bootstrap_snet", lambda **kwargs: (fresh, {}))
    monkeypatch.setattr(snet_cache, "save_after_full_ingest", lambda *args, **kwargs: None)
    with tempfile.TemporaryDirectory() as tmpdir, _redirect_cache(snet_cache, Path(tmpdir)):
        snet_cache.save_snet_cache(_build_test_snet(), _base_manifest())
        damaged = bytearray(snet_cache._CACHE_PATH.read_bytes())
        damaged[len(damaged) // 2] ^= 1
        snet_cache._CACHE_PATH.write_bytes(damaged)
        target = types.SimpleNamespace(
            k_active=None, snet=SNet(), _snet_persistence=None,
            _ingest_dictionaries=lambda: None, _ingest_text_corpora=lambda: None,
            _ingest_dialogue_sessions=lambda: None, _ingest_ceremony_texts=lambda: None,
        )
        daemon.TopologicalDaemon._bootstrap_snet(target)
        assert "fresh" in target.snet._signifiers


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
    test_cache_save_load_roundtrip()
    test_invalidate_cache()
    test_legacy_pickle_is_retired_without_deserialization()
    test_cache_corruption_fails_closed_and_requests_full_rebuild()
    test_cache_schema_rejects_unknown_record_kind()
    test_cache_path_swap_after_digest_cannot_change_loaded_bytes()
    test_cache_uncompressed_size_limit_blocks_compression_bomb_shape()
    test_cache_miss_requests_full_rebuild()
    print("\n=== All tests PASSED ===")
