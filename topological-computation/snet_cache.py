"""snet_cache.py -- S_net 安全持久化缓存 + 增量摄入。

方案1: 将 S_net 的 Dass 记录流式写为 gzip JSONL，加载时重建运行时索引。
方案2: manifest 驱动增量摄入，只处理新增/变更文件。

缓存目录: ~/.swarm/persist/snet_cache/
  snet_cache.jsonl.gz — 有界、严格 schema 的 gzip JSONL 记录
  manifest.json       — 来源清单 + cache SHA-256/大小/记录数

旧 ``snet_cache.pkl.gz`` 永不反序列化。首次发现时直接删除并完整重建。
SHA-256 防止损坏文件进入解析；JSONL 本身不含可执行对象。相邻 manifest
不提供对同一用户攻击者的真实性保证，因此加载器仍按非信任输入校验。

认识论等级: L0（只持久化 Dass；Was/索引由正常构造流程重建）

谱系引用: #456（放弃 pickle）/ #1116（安全迁移）。
"""

from __future__ import annotations

import gzip
import hashlib
import json
import math
import os
import stat
import sys
import tempfile
import time
from pathlib import Path
from typing import Iterator, Optional

from cooccurrence_hyperedge import hyperedge_from_dict, hyperedge_to_dict
from signifier_net import (
    AxisType,
    Morpheme,
    MorphemeStructure,
    SNet,
    Signifier,
    SignifierEdge,
)


# ---------------------------------------------------------------------------
# 缓存路径
# ---------------------------------------------------------------------------

_CACHE_DIR = Path.home() / ".swarm" / "persist" / "snet_cache"
_CACHE_PATH = _CACHE_DIR / "snet_cache.jsonl.gz"
_LEGACY_CACHE_PATH = _CACHE_DIR / "snet_cache.pkl.gz"
_MANIFEST_PATH = _CACHE_DIR / "manifest.json"

_CACHE_FORMAT = "snet-jsonl-gzip"
_JSONL_FORMAT = "snet-jsonl"
_CACHE_SCHEMA_VERSION = 1

# Resource ceilings are intentionally well above the documented 778K-edge corpus.
# They turn malformed JSONL and gzip bombs into cache misses instead of unbounded work.
_MAX_MANIFEST_BYTES = 64 * 1024 * 1024
_MAX_COMPRESSED_BYTES = 4 * 1024 * 1024 * 1024
_MAX_UNCOMPRESSED_BYTES = 8 * 1024 * 1024 * 1024
_MAX_RECORD_BYTES = 16 * 1024 * 1024
_MAX_RECORDS = 5_000_000
_MAX_STRING_CHARS = 8 * 1024 * 1024
_MAX_LIST_ITEMS = 1_000_000


# ---------------------------------------------------------------------------
# Manifest: 文件清单 + 内容 hash
# ---------------------------------------------------------------------------

def _file_hash(path: Path) -> str:
    """计算文件的 SHA-256 hash（前 64KB 快速 hash + 文件大小）。

    使用前 64KB + 文件大小作为快速 fingerprint。对于字典/语料文件，
    文件头变化意味着内容变化，不需要全文件 hash。
    """
    h = hashlib.sha256()
    try:
        size = path.stat().st_size
        h.update(str(size).encode())
        with open(path, "rb") as f:
            h.update(f.read(65536))
    except OSError:
        return ""
    return h.hexdigest()


def _file_mtime(path: Path) -> float:
    """返回文件的 mtime（秒级精度）。"""
    try:
        return path.stat().st_mtime
    except OSError:
        return 0.0


def build_manifest(
    dict_dir: Optional[Path],
    corpus_root: Optional[Path],
    surface_forms_path: Optional[Path],
) -> dict:
    """构建当前文件系统状态的 manifest。

    manifest 结构:
    {
        "version": 2,
        "created": <unix timestamp>,
        "surface_forms": {"path": str, "hash": str} | null,
        "dictionaries": [{"path": str, "hash": str, "mtime": float}, ...],
        "corpora": [{"path": str, "hash": str, "mtime": float, "domain": str}, ...],
    }
    """
    manifest: dict = {
        "version": 2,
        "created": time.time(),
        "surface_forms": None,
        "dictionaries": [],
        "corpora": [],
    }

    # Surface forms
    if surface_forms_path and surface_forms_path.exists():
        manifest["surface_forms"] = {
            "path": str(surface_forms_path),
            "hash": _file_hash(surface_forms_path),
        }

    # Dictionary files
    if dict_dir and dict_dir.is_dir():
        for fpath in sorted(dict_dir.iterdir()):
            if fpath.suffix == ".jsonl" and fpath.is_file():
                manifest["dictionaries"].append({
                    "path": str(fpath),
                    "hash": _file_hash(fpath),
                    "mtime": _file_mtime(fpath),
                })

    # Corpus files
    if corpus_root and corpus_root.is_dir():
        for domain_dir in sorted(corpus_root.iterdir()):
            if not domain_dir.is_dir():
                continue
            domain = domain_dir.name
            for fpath in sorted(domain_dir.iterdir()):
                if fpath.suffix in (".txt", ".md") and fpath.is_file():
                    manifest["corpora"].append({
                        "path": str(fpath),
                        "hash": _file_hash(fpath),
                        "mtime": _file_mtime(fpath),
                        "domain": domain,
                    })

    return manifest


def manifests_match(cached: dict, current: dict) -> bool:
    """判断缓存 manifest 与当前文件系统是否一致。

    比较规则:
    - surface_forms hash 必须匹配
    - dictionaries 列表的 (path, hash) 集合必须相等
    - corpora 列表的 (path, hash) 集合必须相等

    不比较 mtime（hash 一致即内容一致）。
    不比较 created 时间戳。
    """
    if cached.get("version") != current.get("version"):
        return False

    # Surface forms
    cs = cached.get("surface_forms")
    ns = current.get("surface_forms")
    if (cs is None) != (ns is None):
        return False
    if cs and ns and cs.get("hash") != ns.get("hash"):
        return False

    # Dictionaries: compare (path, hash) sets
    cached_dicts = {(d["path"], d["hash"]) for d in cached.get("dictionaries", [])}
    current_dicts = {(d["path"], d["hash"]) for d in current.get("dictionaries", [])}
    if cached_dicts != current_dicts:
        return False

    # Corpora: compare (path, hash) sets
    cached_corpora = {(c["path"], c["hash"]) for c in cached.get("corpora", [])}
    current_corpora = {(c["path"], c["hash"]) for c in current.get("corpora", [])}
    if cached_corpora != current_corpora:
        return False

    return True


def diff_manifests(
    cached: dict,
    current: dict,
) -> dict:
    """计算两个 manifest 之间的差异（增量摄入用）。

    返回:
    {
        "new_dicts": [path, ...],       # 新增或变更的字典文件
        "removed_dicts": [path, ...],   # 删除的字典文件
        "new_corpora": [{"path": ..., "domain": ...}, ...],  # 新增或变更的语料文件
        "removed_corpora": [path, ...], # 删除的语料文件
        "surface_forms_changed": bool,  # surface_forms 是否变更
    }
    """
    diff: dict = {
        "new_dicts": [],
        "removed_dicts": [],
        "new_corpora": [],
        "removed_corpora": [],
        "surface_forms_changed": False,
    }

    # Surface forms
    cs = cached.get("surface_forms")
    ns = current.get("surface_forms")
    if (cs is None) != (ns is None):
        diff["surface_forms_changed"] = True
    elif cs and ns and cs.get("hash") != ns.get("hash"):
        diff["surface_forms_changed"] = True

    # Dictionaries
    cached_dict_map = {d["path"]: d["hash"] for d in cached.get("dictionaries", [])}
    current_dict_map = {d["path"]: d["hash"] for d in current.get("dictionaries", [])}

    for path, h in current_dict_map.items():
        if path not in cached_dict_map or cached_dict_map[path] != h:
            diff["new_dicts"].append(path)
    for path in cached_dict_map:
        if path not in current_dict_map:
            diff["removed_dicts"].append(path)

    # Corpora
    cached_corpus_map = {c["path"]: c["hash"] for c in cached.get("corpora", [])}
    current_corpus_entries = {c["path"]: c for c in current.get("corpora", [])}

    for path, entry in current_corpus_entries.items():
        if path not in cached_corpus_map or cached_corpus_map[path] != entry["hash"]:
            diff["new_corpora"].append({"path": path, "domain": entry["domain"]})
    for path in cached_corpus_map:
        if path not in current_corpus_entries:
            diff["removed_corpora"].append(path)

    return diff


# ---------------------------------------------------------------------------
# 缓存读写
# ---------------------------------------------------------------------------

class _CacheFormatError(ValueError):
    """Cache bytes or metadata do not satisfy the closed schema."""


def _path_exists(path: Path) -> bool:
    """Return True for regular files and dangling symlinks alike."""
    return os.path.lexists(path)


def _reject_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise _CacheFormatError(f"duplicate JSON key: {key!r}")
        result[key] = value
    return result


def _expect_object(value, label: str) -> dict:
    if type(value) is not dict:
        raise _CacheFormatError(f"{label} must be an object")
    return value


def _expect_exact_keys(value: dict, keys: set[str], label: str) -> None:
    actual = set(value)
    if actual != keys:
        raise _CacheFormatError(
            f"{label} keys mismatch: expected {sorted(keys)}, got {sorted(actual)}"
        )


def _checked_int(value, label: str, *, maximum: int | None = None) -> int:
    if type(value) is not int or value < 0:
        raise _CacheFormatError(f"{label} must be a non-negative integer")
    if maximum is not None and value > maximum:
        raise _CacheFormatError(f"{label} limit exceeded")
    return value


def _checked_number(value, label: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise _CacheFormatError(f"{label} must be a finite number")
    result = float(value)
    if not math.isfinite(result):
        raise _CacheFormatError(f"{label} must be a finite number")
    return result


def _checked_string(value, label: str) -> str:
    if type(value) is not str:
        raise _CacheFormatError(f"{label} must be a string")
    if len(value) > _MAX_STRING_CHARS:
        raise _CacheFormatError(f"{label} string limit exceeded")
    return value


def _checked_string_list(value, label: str) -> list[str]:
    if type(value) is not list:
        raise _CacheFormatError(f"{label} must be an array")
    if len(value) > _MAX_LIST_ITEMS:
        raise _CacheFormatError(f"{label} item limit exceeded")
    return [
        _checked_string(item, f"{label}[{index}]")
        for index, item in enumerate(value)
    ]


def _validate_source_manifest(manifest: dict) -> None:
    """Validate fields consumed later by manifests_match/diff_manifests."""
    if manifest.get("version") != 2:
        raise _CacheFormatError("unsupported source manifest version")
    _checked_number(manifest.get("created"), "manifest.created")

    surface_forms = manifest.get("surface_forms")
    if surface_forms is not None:
        surface_forms = _expect_object(surface_forms, "manifest.surface_forms")
        _checked_string(surface_forms.get("path"), "manifest.surface_forms.path")
        _checked_string(surface_forms.get("hash"), "manifest.surface_forms.hash")

    for field, required in (
        ("dictionaries", ("path", "hash")),
        ("corpora", ("path", "hash", "domain")),
    ):
        entries = manifest.get(field)
        if type(entries) is not list:
            raise _CacheFormatError(f"manifest.{field} must be an array")
        if len(entries) > _MAX_LIST_ITEMS:
            raise _CacheFormatError(f"manifest.{field} item limit exceeded")
        for index, entry in enumerate(entries):
            entry = _expect_object(entry, f"manifest.{field}[{index}]")
            for key in required:
                _checked_string(entry.get(key), f"manifest.{field}[{index}].{key}")
            if "mtime" in entry:
                _checked_number(entry["mtime"], f"manifest.{field}[{index}].mtime")


def _validate_cache_info(manifest: dict) -> dict:
    _validate_source_manifest(manifest)
    info = _expect_object(manifest.get("cache_info"), "manifest.cache_info")
    if info.get("format") != _CACHE_FORMAT:
        raise _CacheFormatError("unsupported cache format")
    if info.get("schema_version") != _CACHE_SCHEMA_VERSION:
        raise _CacheFormatError("unsupported cache schema version")

    digest = _checked_string(info.get("sha256"), "manifest.cache_info.sha256")
    if len(digest) != 64 or any(ch not in "0123456789abcdef" for ch in digest):
        raise _CacheFormatError("manifest.cache_info.sha256 must be lowercase SHA-256")

    compressed_bytes = _checked_int(
        info.get("compressed_bytes"),
        "cache compressed size",
        maximum=_MAX_COMPRESSED_BYTES,
    )
    uncompressed_bytes = _checked_int(
        info.get("uncompressed_bytes"),
        "cache uncompressed size",
        maximum=_MAX_UNCOMPRESSED_BYTES,
    )
    record_count = _checked_int(
        info.get("record_count"),
        "cache record count",
        maximum=_MAX_RECORDS,
    )
    counts = {
        "signifiers": _checked_int(
            info.get("n_signifiers"), "cache signifier count", maximum=_MAX_RECORDS
        ),
        "edges": _checked_int(
            info.get("n_edges"), "cache edge count", maximum=_MAX_RECORDS
        ),
        "morphemes": _checked_int(
            info.get("n_morphemes"), "cache morpheme count", maximum=_MAX_RECORDS
        ),
        "hyperedges": _checked_int(
            info.get("n_hyperedges"), "cache hyperedge count", maximum=_MAX_RECORDS
        ),
    }
    if sum(counts.values()) + 2 != record_count:
        raise _CacheFormatError("cache record count does not match manifest counts")
    if compressed_bytes == 0 or uncompressed_bytes == 0:
        raise _CacheFormatError("cache sizes must be non-zero")
    return {
        "sha256": digest,
        "compressed_bytes": compressed_bytes,
        "uncompressed_bytes": uncompressed_bytes,
        "record_count": record_count,
        "counts": counts,
    }


def _load_manifest() -> dict:
    size = _MANIFEST_PATH.stat().st_size
    if size <= 0 or size > _MAX_MANIFEST_BYTES:
        raise _CacheFormatError("cache manifest size limit exceeded")
    raw = _MANIFEST_PATH.read_bytes()
    if len(raw) != size:
        raise _CacheFormatError("cache manifest changed while reading")
    manifest = json.loads(
        raw.decode("utf-8"),
        object_pairs_hook=_reject_duplicate_keys,
    )
    return _expect_object(manifest, "manifest")


def _sha256_stream(stream) -> str:
    digest = hashlib.sha256()
    stream.seek(0)
    for chunk in iter(lambda: stream.read(1024 * 1024), b""):
        digest.update(chunk)
    stream.seek(0)
    return digest.hexdigest()


def _sha256_file(path: Path) -> str:
    with path.open("rb") as stream:
        return _sha256_stream(stream)


def _retire_legacy_cache() -> bool:
    """Delete the old executable cache without opening its contents."""
    if not _path_exists(_LEGACY_CACHE_PATH):
        return False
    _LEGACY_CACHE_PATH.unlink()
    if not _path_exists(_CACHE_PATH) and _path_exists(_MANIFEST_PATH):
        _MANIFEST_PATH.unlink()
    print(
        "S_net cache: retired legacy pickle without deserializing",
        file=sys.stderr,
    )
    return True


def _iter_snet_edges(snet: SNet) -> Iterator[SignifierEdge]:
    """Stream edges from either in-memory SNet or SQLite-backed SNetLazy."""
    persistence = getattr(snet, "_persistence", None)
    if persistence is not None and hasattr(persistence, "iter_all_edges"):
        yield from persistence.iter_all_edges()
    else:
        yield from snet._edges


def _write_jsonl_record(stream, record: dict) -> int:
    encoded = json.dumps(
        record,
        ensure_ascii=False,
        allow_nan=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8") + b"\n"
    if len(encoded) > _MAX_RECORD_BYTES:
        raise ValueError("S_net cache record exceeds the configured size limit")
    stream.write(encoded)
    return len(encoded)


def _write_jsonl_cache(stream, snet: SNet) -> tuple[int, int, dict[str, int]]:
    counts = {"signifiers": 0, "edges": 0, "morphemes": 0, "hyperedges": 0}
    uncompressed_bytes = 0
    record_count = 0

    def write(record: dict) -> None:
        nonlocal uncompressed_bytes, record_count
        uncompressed_bytes += _write_jsonl_record(stream, record)
        record_count += 1
        if uncompressed_bytes > _MAX_UNCOMPRESSED_BYTES:
            raise ValueError("S_net cache exceeds the configured uncompressed size limit")
        if record_count > _MAX_RECORDS:
            raise ValueError("S_net cache exceeds the configured record limit")

    write({
        "kind": "header",
        "format": _JSONL_FORMAT,
        "schema_version": _CACHE_SCHEMA_VERSION,
    })

    for key, sig in snet._signifiers.items():
        write({
            "kind": "signifier",
            "key": key,
            "value": {
                "id": sig.id,
                "surface_forms": list(sig.surface_forms),
                "source": sig.source,
                "lang": sig.lang,
                "domain": sig.domain,
            },
        })
        counts["signifiers"] += 1

    for edge in _iter_snet_edges(snet):
        write({
            "kind": "edge",
            "value": {
                "source": edge.source,
                "target": edge.target,
                "axis": edge.axis.value,
                "weight": edge.weight,
                "evidence": edge.evidence,
                "relation": edge.relation,
                "differential": edge.differential,
            },
        })
        counts["edges"] += 1

    for key, structure in snet._morphemes.items():
        write({
            "kind": "morpheme",
            "key": key,
            "value": {
                "signifier_id": structure.signifier_id,
                "morphemes": [
                    {
                        "form": morpheme.form,
                        "meaning": morpheme.meaning,
                        "lang": morpheme.lang,
                        "shared_with": list(morpheme.shared_with),
                    }
                    for morpheme in structure.morphemes
                ],
                "etymology": structure.etymology,
            },
        })
        counts["morphemes"] += 1

    for hyperedge in snet._hyperedges:
        write({"kind": "hyperedge", "value": hyperedge_to_dict(hyperedge)})
        counts["hyperedges"] += 1

    write({"kind": "footer", "counts": counts})
    return uncompressed_bytes, record_count, counts


def _atomic_write_json(path: Path, value: dict) -> None:
    temp_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=path.parent,
            prefix=f".{path.name}.",
            suffix=".tmp",
            delete=False,
        ) as stream:
            temp_path = Path(stream.name)
            os.fchmod(stream.fileno(), 0o600)
            json.dump(value, stream, ensure_ascii=False, indent=2, allow_nan=False)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temp_path, path)
        temp_path = None
    finally:
        if temp_path is not None and _path_exists(temp_path):
            temp_path.unlink()


def save_snet_cache(snet: SNet, manifest: dict) -> Path:
    """Stream S_net Dass to integrity-checked gzip JSONL.

    The cache file is replaced before the manifest. A crash between the two
    replacements leaves a digest mismatch, which the loader treats as a miss.
    """
    _CACHE_DIR.mkdir(parents=True, exist_ok=True, mode=0o700)
    os.chmod(_CACHE_DIR, 0o700)
    _retire_legacy_cache()

    t0 = time.time()
    temp_path: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w+b",
            dir=_CACHE_DIR,
            prefix=".snet-cache.",
            suffix=".tmp",
            delete=False,
        ) as raw_stream:
            temp_path = Path(raw_stream.name)
            os.fchmod(raw_stream.fileno(), 0o600)
            with gzip.GzipFile(
                filename="",
                mode="wb",
                compresslevel=4,
                fileobj=raw_stream,
                mtime=0,
            ) as compressed_stream:
                uncompressed_bytes, record_count, counts = _write_jsonl_cache(
                    compressed_stream, snet
                )
            raw_stream.flush()
            os.fsync(raw_stream.fileno())

        compressed_bytes = temp_path.stat().st_size
        if compressed_bytes <= 0 or compressed_bytes > _MAX_COMPRESSED_BYTES:
            raise ValueError("S_net cache exceeds the configured compressed size limit")
        digest = _sha256_file(temp_path)
        os.replace(temp_path, _CACHE_PATH)
        temp_path = None

        elapsed = time.time() - t0
        stored_manifest = dict(manifest)
        stored_manifest["cache_info"] = {
            "format": _CACHE_FORMAT,
            "schema_version": _CACHE_SCHEMA_VERSION,
            "sha256": digest,
            "compressed_bytes": compressed_bytes,
            "uncompressed_bytes": uncompressed_bytes,
            "record_count": record_count,
            "n_signifiers": counts["signifiers"],
            "n_edges": counts["edges"],
            "n_morphemes": counts["morphemes"],
            "n_hyperedges": counts["hyperedges"],
            "save_time": elapsed,
            "cache_path": str(_CACHE_PATH),
        }
        _atomic_write_json(_MANIFEST_PATH, stored_manifest)
        return _CACHE_PATH
    finally:
        if temp_path is not None and _path_exists(temp_path):
            temp_path.unlink()


def _parse_signifier(record: dict) -> tuple[str, Signifier]:
    _expect_exact_keys(record, {"kind", "key", "value"}, "signifier record")
    key = _checked_string(record["key"], "signifier.key")
    value = _expect_object(record["value"], "signifier.value")
    _expect_exact_keys(
        value,
        {"id", "surface_forms", "source", "lang", "domain"},
        "signifier.value",
    )
    return key, Signifier(
        id=_checked_string(value["id"], "signifier.id"),
        surface_forms=tuple(
            _checked_string_list(value["surface_forms"], "signifier.surface_forms")
        ),
        source=_checked_string(value["source"], "signifier.source"),
        lang=_checked_string(value["lang"], "signifier.lang"),
        domain=_checked_string(value["domain"], "signifier.domain"),
    )


def _parse_edge(record: dict) -> SignifierEdge:
    _expect_exact_keys(record, {"kind", "value"}, "edge record")
    value = _expect_object(record["value"], "edge.value")
    _expect_exact_keys(
        value,
        {"source", "target", "axis", "weight", "evidence", "relation", "differential"},
        "edge.value",
    )
    axis_value = _checked_string(value["axis"], "edge.axis")
    try:
        axis = AxisType(axis_value)
    except ValueError as exc:
        raise _CacheFormatError(f"unsupported edge axis: {axis_value!r}") from exc
    return SignifierEdge(
        source=_checked_string(value["source"], "edge.source"),
        target=_checked_string(value["target"], "edge.target"),
        axis=axis,
        weight=_checked_number(value["weight"], "edge.weight"),
        evidence=_checked_string(value["evidence"], "edge.evidence"),
        relation=_checked_string(value["relation"], "edge.relation"),
        differential=_checked_string(value["differential"], "edge.differential"),
    )


def _parse_morpheme(record: dict) -> tuple[str, MorphemeStructure]:
    _expect_exact_keys(record, {"kind", "key", "value"}, "morpheme record")
    key = _checked_string(record["key"], "morpheme.key")
    value = _expect_object(record["value"], "morpheme.value")
    _expect_exact_keys(
        value,
        {"signifier_id", "morphemes", "etymology"},
        "morpheme.value",
    )
    items = value["morphemes"]
    if type(items) is not list:
        raise _CacheFormatError("morpheme.morphemes must be an array")
    if len(items) > _MAX_LIST_ITEMS:
        raise _CacheFormatError("morpheme.morphemes item limit exceeded")
    morphemes = []
    for index, item in enumerate(items):
        item = _expect_object(item, f"morpheme.morphemes[{index}]")
        _expect_exact_keys(
            item,
            {"form", "meaning", "lang", "shared_with"},
            f"morpheme.morphemes[{index}]",
        )
        morphemes.append(Morpheme(
            form=_checked_string(item["form"], f"morpheme[{index}].form"),
            meaning=_checked_string(item["meaning"], f"morpheme[{index}].meaning"),
            lang=_checked_string(item["lang"], f"morpheme[{index}].lang"),
            shared_with=tuple(
                _checked_string_list(item["shared_with"], f"morpheme[{index}].shared_with")
            ),
        ))
    return key, MorphemeStructure(
        signifier_id=_checked_string(value["signifier_id"], "morpheme.signifier_id"),
        morphemes=tuple(morphemes),
        etymology=_checked_string(value["etymology"], "morpheme.etymology"),
    )


def _parse_hyperedge(record: dict):
    _expect_exact_keys(record, {"kind", "value"}, "hyperedge record")
    value = _expect_object(record["value"], "hyperedge.value")
    _expect_exact_keys(
        value,
        {"vertices", "source", "domain", "timestamp", "ingest_param_refs", "evidence_tag"},
        "hyperedge.value",
    )
    vertices = _checked_string_list(value["vertices"], "hyperedge.vertices")
    if len(set(vertices)) != len(vertices):
        raise _CacheFormatError("hyperedge.vertices must not contain duplicates")
    checked = {
        "vertices": vertices,
        "source": _checked_string(value["source"], "hyperedge.source"),
        "domain": _checked_string(value["domain"], "hyperedge.domain"),
        "timestamp": _checked_string(value["timestamp"], "hyperedge.timestamp"),
        "ingest_param_refs": _checked_string_list(
            value["ingest_param_refs"], "hyperedge.ingest_param_refs"
        ),
        "evidence_tag": _checked_string(value["evidence_tag"], "hyperedge.evidence_tag"),
    }
    return hyperedge_from_dict(checked)


def _parse_jsonl_cache(info: dict, raw_stream) -> SNet:
    signifiers: dict[str, Signifier] = {}
    edges: list[SignifierEdge] = []
    morphemes: dict[str, MorphemeStructure] = {}
    hyperedges = []
    actual_counts = {"signifiers": 0, "edges": 0, "morphemes": 0, "hyperedges": 0}
    footer_counts: dict | None = None
    total_bytes = 0
    record_count = 0

    raw_stream.seek(0)
    with gzip.GzipFile(fileobj=raw_stream, mode="rb") as stream:
        while True:
            line = stream.readline(_MAX_RECORD_BYTES + 1)
            if not line:
                break
            if len(line) > _MAX_RECORD_BYTES:
                raise _CacheFormatError("cache record size limit exceeded")
            if not line.endswith(b"\n"):
                raise _CacheFormatError("cache record is not newline terminated")
            total_bytes += len(line)
            record_count += 1
            if total_bytes > _MAX_UNCOMPRESSED_BYTES:
                raise _CacheFormatError("cache uncompressed size limit exceeded")
            if total_bytes > info["uncompressed_bytes"]:
                raise _CacheFormatError("cache exceeds manifest uncompressed size")
            if record_count > _MAX_RECORDS:
                raise _CacheFormatError("cache record count limit exceeded")
            if record_count > info["record_count"]:
                raise _CacheFormatError("cache exceeds manifest record count")

            record = json.loads(line, object_pairs_hook=_reject_duplicate_keys)
            record = _expect_object(record, f"record {record_count}")
            kind = record.get("kind")

            if record_count == 1:
                _expect_exact_keys(
                    record,
                    {"kind", "format", "schema_version"},
                    "cache header",
                )
                if (
                    kind != "header"
                    or record["format"] != _JSONL_FORMAT
                    or record["schema_version"] != _CACHE_SCHEMA_VERSION
                ):
                    raise _CacheFormatError("invalid cache header")
                continue

            if footer_counts is not None:
                raise _CacheFormatError("cache contains records after footer")

            if kind == "signifier":
                key, signifier = _parse_signifier(record)
                if key in signifiers:
                    raise _CacheFormatError(f"duplicate signifier key: {key!r}")
                signifiers[key] = signifier
                actual_counts["signifiers"] += 1
            elif kind == "edge":
                edges.append(_parse_edge(record))
                actual_counts["edges"] += 1
            elif kind == "morpheme":
                key, structure = _parse_morpheme(record)
                if key in morphemes:
                    raise _CacheFormatError(f"duplicate morpheme key: {key!r}")
                morphemes[key] = structure
                actual_counts["morphemes"] += 1
            elif kind == "hyperedge":
                hyperedges.append(_parse_hyperedge(record))
                actual_counts["hyperedges"] += 1
            elif kind == "footer":
                _expect_exact_keys(record, {"kind", "counts"}, "cache footer")
                footer = _expect_object(record["counts"], "cache footer counts")
                _expect_exact_keys(
                    footer,
                    {"signifiers", "edges", "morphemes", "hyperedges"},
                    "cache footer counts",
                )
                footer_counts = {
                    name: _checked_int(
                        footer[name], f"cache footer {name}", maximum=_MAX_RECORDS
                    )
                    for name in actual_counts
                }
            else:
                raise _CacheFormatError(f"unknown cache record kind: {kind!r}")

    if footer_counts is None:
        raise _CacheFormatError("cache footer missing")
    if total_bytes != info["uncompressed_bytes"]:
        raise _CacheFormatError("cache uncompressed size does not match manifest")
    if record_count != info["record_count"]:
        raise _CacheFormatError("cache record count does not match manifest")
    if actual_counts != footer_counts:
        raise _CacheFormatError("cache footer counts do not match records")
    if actual_counts != info["counts"]:
        raise _CacheFormatError("cache counts do not match manifest")

    # This constructor rebuilds all Was indexes from the validated Dass records.
    return SNet(
        signifiers=signifiers,
        edges=edges,
        morphemes=morphemes,
        hyperedges=hyperedges,
    )


def load_snet_cache() -> tuple[Optional[SNet], Optional[dict]]:
    """Load a validated JSONL cache, otherwise return a fail-closed miss."""
    try:
        retired = _retire_legacy_cache()
    except OSError as exc:
        print(f"S_net legacy cache retirement failed: {exc}", file=sys.stderr)
        return None, None

    if retired and not _path_exists(_CACHE_PATH):
        print("S_net cache: full rebuild required after legacy retirement", file=sys.stderr)
        return None, None
    if not _path_exists(_CACHE_PATH) or not _path_exists(_MANIFEST_PATH):
        return None, None

    try:
        manifest = _load_manifest()
        info = _validate_cache_info(manifest)
        open_flags = (
            os.O_RDONLY
            | getattr(os, "O_CLOEXEC", 0)
            | getattr(os, "O_NONBLOCK", 0)
        )
        cache_fd = os.open(_CACHE_PATH, open_flags)
        with os.fdopen(cache_fd, "rb") as cache_stream:
            before = os.fstat(cache_stream.fileno())
            if not stat.S_ISREG(before.st_mode):
                raise _CacheFormatError("cache path must resolve to a regular file")
            compressed_bytes = before.st_size
            if compressed_bytes != info["compressed_bytes"]:
                raise _CacheFormatError("cache compressed size does not match manifest")
            if compressed_bytes > _MAX_COMPRESSED_BYTES:
                raise _CacheFormatError("cache compressed size limit exceeded")
            if _sha256_stream(cache_stream) != info["sha256"]:
                raise _CacheFormatError("cache digest mismatch")

            t0 = time.time()
            snet = _parse_jsonl_cache(info, cache_stream)
            after = os.fstat(cache_stream.fileno())
            identity_fields = (
                "st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns"
            )
            if any(
                getattr(before, field) != getattr(after, field)
                for field in identity_fields
            ):
                raise _CacheFormatError("cache changed while reading")
            elapsed = time.time() - t0
        print(
            f"S_net cache loaded: {len(snet._signifiers)} signifiers, "
            f"{len(snet._edges)} edges, {len(snet._morphemes)} morphemes "
            f"({elapsed:.1f}s)",
            file=sys.stderr,
        )
        return snet, manifest
    except Exception as exc:
        print(f"S_net cache load failed (fail-closed): {exc}", file=sys.stderr)
        return None, None


def invalidate_cache() -> None:
    """Delete current, legacy, and manifest artifacts to force a full ingest."""
    for path in (_CACHE_PATH, _LEGACY_CACHE_PATH, _MANIFEST_PATH):
        try:
            if _path_exists(path):
                path.unlink()
        except OSError as exc:
            print(f"S_net cache invalidation failed for {path}: {exc}", file=sys.stderr)


# ---------------------------------------------------------------------------
# 增量摄入（方案2）
# ---------------------------------------------------------------------------

def incremental_ingest_dicts(
    snet: SNet,
    new_dict_paths: list[str],
    graph=None,
) -> SNet:
    """增量摄入新增/变更的字典文件。

    只处理 new_dict_paths 中列出的文件，不重新摄入已有文件。
    """
    from signifier_net_ingest import (
        ingest_all_dictionaries,
        ingest_bilingual_dict,
        ingest_morpheme_dict,
        ingest_synonym_dict,
        ingest_collocation_dict,
        ingest_thesaurus_dict,
        ingest_wiktionary_dict,
        ingest_idiom_dict,
        ingest_wortschatz_dict,
        ingest_code_dict,
    )

    _type_dispatch: dict[str, callable] = {
        "bilingual_": ingest_bilingual_dict,
        "morpheme_": ingest_morpheme_dict,
        "synonym_": ingest_synonym_dict,
        "collocations_": ingest_collocation_dict,
        "thesaurus_": ingest_thesaurus_dict,
        "wiktionary_": ingest_wiktionary_dict,
        "idioms_": ingest_idiom_dict,
        "wortschatz_": ingest_wortschatz_dict,
    }

    for path_str in new_dict_paths:
        fpath = Path(path_str)
        if not fpath.exists():
            continue
        fname = fpath.name

        try:
            if fname.startswith("code_dict_"):
                snet, _ = ingest_code_dict(snet, fpath, graph=graph)
            elif fname.startswith("dict_") or fname.startswith("text_"):
                # Monolingual dict or text passage: ingest individually
                from signifier_net_ingest import ingest_dictionary
                snet, _ = ingest_dictionary(snet, fpath)
            else:
                matched = False
                for prefix, ingest_fn in _type_dispatch.items():
                    if fname.startswith(prefix):
                        snet, _ = ingest_fn(snet, fpath)
                        matched = True
                        break
                if not matched:
                    # Unknown dict type, try generic monolingual ingest
                    from signifier_net_ingest import ingest_dictionary
                    snet, _ = ingest_dictionary(snet, fpath)
            print(f"  incremental dict: {fname} OK", file=sys.stderr)
        except Exception as exc:
            print(f"  incremental dict: {fname} ERROR - {exc}", file=sys.stderr)

    return snet


def incremental_ingest_corpora(
    snet: SNet,
    new_corpora: list[dict],
) -> SNet:
    """增量摄入新增/变更的语料文件。

    new_corpora: [{"path": str, "domain": str}, ...]
    """
    from text_corpus_loader import load_text_file

    for entry in new_corpora:
        fpath = Path(entry["path"])
        domain = entry["domain"]
        if not fpath.exists():
            continue
        try:
            snet, log_entries = load_text_file(snet, fpath, domain)
            n = len(log_entries)
            print(f"  incremental corpus: {fpath.name} ({domain}): {n} entries", file=sys.stderr)
        except Exception as exc:
            print(f"  incremental corpus: {fpath.name} ERROR - {exc}", file=sys.stderr)

    return snet


# ---------------------------------------------------------------------------
# 主入口: try_load_or_ingest
# ---------------------------------------------------------------------------

def try_load_cached_snet(
    dict_dir: Optional[Path],
    corpus_root: Optional[Path],
    surface_forms_path: Optional[Path],
    graph=None,
) -> tuple[Optional[SNet], bool]:
    """尝试从缓存加载 S_net，如果缓存无效则返回 None。

    返回:
      (snet, used_cache)
      - snet: 加载的 SNet（缓存命中时），或 None（缓存未命中）
      - used_cache: 是否使用了缓存

    如果缓存存在但 manifest 不完全匹配，尝试增量摄入:
      - 加载缓存的 SNet
      - 只摄入新增/变更的文件
      - 保存更新后的缓存
    """
    # 构建当前 manifest
    current_manifest = build_manifest(dict_dir, corpus_root, surface_forms_path)

    # 尝试加载缓存
    cached_snet, cached_manifest = load_snet_cache()
    if cached_snet is None or cached_manifest is None:
        return None, False

    # 完全匹配: 直接使用缓存
    if manifests_match(cached_manifest, current_manifest):
        print("S_net cache: manifest match, using cached S_net", file=sys.stderr)
        return cached_snet, True

    # 部分匹配: 增量摄入
    if not current_manifest.get("surface_forms"):
        sf_changed = False
    else:
        sf_changed = False
        cs = cached_manifest.get("surface_forms")
        ns = current_manifest.get("surface_forms")
        if (cs is None) != (ns is None):
            sf_changed = True
        elif cs and ns and cs.get("hash") != ns.get("hash"):
            sf_changed = True

    if sf_changed:
        # surface_forms 变更 => 需要完整重建（bootstrap_layer_b 依赖它）
        print("S_net cache: surface_forms changed, full rebuild needed", file=sys.stderr)
        return None, False

    diff = diff_manifests(cached_manifest, current_manifest)
    has_changes = (
        diff["new_dicts"]
        or diff["removed_dicts"]
        or diff["new_corpora"]
        or diff["removed_corpora"]
    )

    if not has_changes:
        # manifest format changed but content identical
        print("S_net cache: content match, using cached S_net", file=sys.stderr)
        return cached_snet, True

    # 增量摄入
    n_new_dicts = len(diff["new_dicts"])
    n_new_corpora = len(diff["new_corpora"])
    n_removed = len(diff["removed_dicts"]) + len(diff["removed_corpora"])

    if n_removed > 0:
        # 有文件被删除: stale signifiers 不删除（方案2 要求），但需要标记
        print(
            f"S_net cache: {n_removed} files removed (stale signifiers kept)",
            file=sys.stderr,
        )

    print(
        f"S_net cache: incremental ingest — {n_new_dicts} dict(s), "
        f"{n_new_corpora} corpus file(s)",
        file=sys.stderr,
    )

    snet = cached_snet
    if diff["new_dicts"]:
        snet = incremental_ingest_dicts(snet, diff["new_dicts"], graph=graph)
    if diff["new_corpora"]:
        snet = incremental_ingest_corpora(snet, diff["new_corpora"])

    # 保存更新后的缓存
    save_snet_cache(snet, current_manifest)
    print("S_net cache: incremental update saved", file=sys.stderr)

    return snet, True


def save_after_full_ingest(
    snet: SNet,
    dict_dir: Optional[Path],
    corpus_root: Optional[Path],
    surface_forms_path: Optional[Path],
) -> None:
    """完整摄入后保存缓存。

    在 daemon._bootstrap_snet 完成所有摄入后调用。
    """
    manifest = build_manifest(dict_dir, corpus_root, surface_forms_path)
    cache_path = save_snet_cache(snet, manifest)
    cache_size_mb = cache_path.stat().st_size / (1024 * 1024)
    print(
        f"S_net cache saved: {cache_size_mb:.1f}MB at {cache_path}",
        file=sys.stderr,
    )
