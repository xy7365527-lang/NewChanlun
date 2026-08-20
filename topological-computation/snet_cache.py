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
import secrets
import stat
import sys
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

# 2026-08-20 measurement: the checked-in production corpus is 538 files,
# 9,924,711 lines and 552,802,053 bytes; the production graph is about 778K
# edges. A production-format 100,003-record sample measured 14,544,739 raw
# bytes and 120,537,088-byte peak RSS. Its calibrated base estimate
# (raw + 1 KiB/record) is 116,947,739 bytes, within 3.1% of measured RSS.
# A worst-shape 200K-item surface_forms record measured ~235 bytes of extra
# RSS per decoded item, so the global budget charges 256 bytes per list item.
# The settled large-graph observation records 263K signifiers; together with
# ~778K edges plus header/footer, the known floor is 1,041,002 records.  The
# measured 14,544,739 bytes / 100,003 records projects to 151,406,482 bytes
# (144.39 MiB), leaving 15.61 MiB (10.8%) below the 160 MiB stream cap.  A
# 1.2M record ceiling leaves 15% record headroom, while the calibrated
# 1.25 GiB object budget keeps decoded-object growth bounded.
_MAX_MANIFEST_BYTES = 1024 * 1024
_MAX_COMPRESSED_BYTES = 64 * 1024 * 1024
_MAX_UNCOMPRESSED_BYTES = 160 * 1024 * 1024
_MAX_RECORD_BYTES = 4 * 1024 * 1024
_MAX_RECORDS = 1_200_000
_MAX_STRING_CHARS = 2 * 1024 * 1024
_MAX_LIST_ITEMS = 200_000
_ESTIMATED_RECORD_OVERHEAD = 1024
_ESTIMATED_LIST_ITEM_BYTES = 256
_MAX_ESTIMATED_LOAD_BYTES = 1280 * 1024 * 1024


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


# ---------------------------------------------------------------------------
# 缓存读写
# ---------------------------------------------------------------------------

class _CacheFormatError(ValueError):
    """Cache bytes or metadata do not satisfy the closed schema."""


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
    if type(value) not in (int, float):
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
    checked = [
        _checked_string(item, f"{label}[{index}]")
        for index, item in enumerate(value)
    ]
    if len(set(checked)) != len(checked):
        raise _CacheFormatError(f"{label} must not contain duplicates")
    return checked


def _count_decoded_list_items(value) -> int:
    """Count all nested JSON array items for the calibrated object budget."""
    total = 0
    pending = [value]
    while pending:
        current = pending.pop()
        if type(current) is list:
            total += len(current)
            pending.extend(current)
        elif type(current) is dict:
            pending.extend(current.values())
    return total


def _validate_source_manifest(manifest: dict) -> None:
    """Validate fields consumed later by manifests_match/diff_manifests."""
    if type(manifest.get("version")) is not int or manifest["version"] != 2:
        raise _CacheFormatError("unsupported source manifest version")
    allowed = {"version", "created", "surface_forms", "dictionaries", "corpora", "cache_info"}
    if not set(manifest).issubset(allowed) or not {"version", "created", "surface_forms", "dictionaries", "corpora"}.issubset(manifest):
        raise _CacheFormatError("manifest keys mismatch")
    _checked_number(manifest.get("created"), "manifest.created")

    surface_forms = manifest.get("surface_forms")
    if surface_forms is not None:
        surface_forms = _expect_object(surface_forms, "manifest.surface_forms")
        _expect_exact_keys(surface_forms, {"path", "hash"}, "manifest.surface_forms")
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
        seen_paths: set[str] = set()
        for index, entry in enumerate(entries):
            entry = _expect_object(entry, f"manifest.{field}[{index}]")
            allowed_entry = set(required) | {"mtime"}
            if set(entry) != allowed_entry:
                raise _CacheFormatError(f"manifest.{field}[{index}] keys mismatch")
            for key in required:
                _checked_string(entry.get(key), f"manifest.{field}[{index}].{key}")
            path = entry["path"]
            if path in seen_paths:
                raise _CacheFormatError(f"manifest.{field} contains duplicate path: {path!r}")
            seen_paths.add(path)
            if "mtime" in entry:
                _checked_number(entry["mtime"], f"manifest.{field}[{index}].mtime")


def _validate_cache_info(manifest: dict) -> dict:
    _validate_source_manifest(manifest)
    info = _expect_object(manifest.get("cache_info"), "manifest.cache_info")
    required_info = {
        "format", "schema_version", "sha256", "compressed_bytes",
        "uncompressed_bytes", "record_count", "n_signifiers", "n_edges",
        "n_morphemes", "n_hyperedges",
    }
    optional_info = {"save_time", "cache_path"}
    if not required_info.issubset(info) or not set(info).issubset(required_info | optional_info):
        raise _CacheFormatError("manifest.cache_info keys mismatch")
    if "save_time" in info:
        _checked_number(info["save_time"], "manifest.cache_info.save_time")
    if "cache_path" in info:
        _checked_string(info["cache_path"], "manifest.cache_info.cache_path")
    if info.get("format") != _CACHE_FORMAT:
        raise _CacheFormatError("unsupported cache format")
    if (type(info.get("schema_version")) is not int
            or info["schema_version"] != _CACHE_SCHEMA_VERSION):
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


def _open_cache_dir(*, create: bool = False) -> int:
    """Open every path component with openat/O_NOFOLLOW."""
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0)
    flags |= getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0)
    path = _CACHE_DIR if _CACHE_DIR.is_absolute() else _CACHE_DIR.absolute()
    current_fd: int | None = None
    try:
        current_fd = os.open(path.anchor or ".", flags)
        parts = path.parts[1:] if path.anchor else path.parts
        for part in parts:
            if part in ("", "."):
                continue
            if part == "..":
                raise _CacheFormatError("cache directory contains parent traversal")
            if create:
                try:
                    os.mkdir(part, mode=0o700, dir_fd=current_fd)
                except FileExistsError:
                    pass
            next_fd = os.open(part, flags, dir_fd=current_fd)
            try:
                info = os.fstat(next_fd)
                if not stat.S_ISDIR(info.st_mode):
                    raise _CacheFormatError("cache path component is not a directory")
            except Exception:
                os.close(next_fd)
                raise
            os.close(current_fd)
            current_fd = next_fd
        return current_fd
    except Exception as exc:
        if current_fd is not None:
            os.close(current_fd)
        if isinstance(exc, _CacheFormatError):
            raise
        raise _CacheFormatError("cache directory is unavailable or unsafe") from exc


def _reject_unsafe_target_at(dir_fd: int, name: str) -> None:
    try:
        info = os.stat(name, dir_fd=dir_fd, follow_symlinks=False)
    except FileNotFoundError:
        return
    if not stat.S_ISREG(info.st_mode):
        raise _CacheFormatError(f"{name} target must be a regular file")


def _open_regular_at(dir_fd: int, name: str) -> tuple[int, os.stat_result]:
    flags = os.O_RDONLY | getattr(os, "O_CLOEXEC", 0)
    flags |= getattr(os, "O_NONBLOCK", 0) | getattr(os, "O_NOFOLLOW", 0)
    fd = os.open(name, flags, dir_fd=dir_fd)
    try:
        info = os.fstat(fd)
    except Exception:
        os.close(fd)
        raise
    if not stat.S_ISREG(info.st_mode):
        os.close(fd)
        raise _CacheFormatError(f"{name} must be a regular file")
    return fd, info


def _read_bounded_fd(fd: int, expected_size: int, maximum: int) -> bytes:
    if expected_size <= 0 or expected_size > maximum:
        raise _CacheFormatError("cache manifest size limit exceeded")
    chunks: list[bytes] = []
    remaining = expected_size
    while remaining:
        chunk = os.read(fd, min(remaining, 1024 * 1024))
        if not chunk:
            break
        chunks.append(chunk)
        remaining -= len(chunk)
    if remaining or os.read(fd, 1):
        raise _CacheFormatError("cache manifest changed while reading")
    return b"".join(chunks)


def _load_manifest(dir_fd: int) -> dict:
    fd, before = _open_regular_at(dir_fd, _MANIFEST_PATH.name)
    try:
        raw = _read_bounded_fd(fd, before.st_size, _MAX_MANIFEST_BYTES)
        after = os.fstat(fd)
    finally:
        os.close(fd)
    size = before.st_size
    if size <= 0 or size > _MAX_MANIFEST_BYTES:
        raise _CacheFormatError("cache manifest size limit exceeded")
    identity = ("st_dev", "st_ino", "st_size", "st_mtime_ns", "st_ctime_ns")
    if any(getattr(before, key) != getattr(after, key) for key in identity):
        raise _CacheFormatError("cache manifest changed while reading")
    manifest = json.loads(
        raw.decode("utf-8"),
        object_pairs_hook=_reject_duplicate_keys,
    )
    return _expect_object(manifest, "manifest")


class _HardLimitReader:
    """Seekable view that never reads beyond the initial regular-file size."""

    def __init__(self, stream, hard_limit: int) -> None:
        self._stream = stream
        self._hard_limit = hard_limit

    def read(self, size: int = -1) -> bytes:
        remaining = self._hard_limit - self._stream.tell()
        if remaining <= 0:
            return b""
        if size is None or size < 0 or size > remaining:
            size = remaining
        return self._stream.read(size)

    def seek(self, offset: int, whence: int = os.SEEK_SET) -> int:
        if whence == os.SEEK_SET:
            target = offset
        elif whence == os.SEEK_CUR:
            target = self._stream.tell() + offset
        elif whence == os.SEEK_END:
            target = self._hard_limit + offset
        else:
            raise ValueError("invalid whence")
        if target < 0 or target > self._hard_limit:
            raise _CacheFormatError("cache read attempted outside initial file window")
        return self._stream.seek(target, os.SEEK_SET)

    def tell(self) -> int:
        return self._stream.tell()

    def fileno(self) -> int:
        return self._stream.fileno()


def _sha256_stream(stream, hard_limit: int) -> str:
    digest = hashlib.sha256()
    stream.seek(0)
    remaining = hard_limit
    while remaining:
        chunk = stream.read(min(remaining, 1024 * 1024))
        if not chunk:
            raise _CacheFormatError("cache truncated while hashing")
        digest.update(chunk)
        remaining -= len(chunk)
    stream.seek(0)
    return digest.hexdigest()


def _retire_legacy_cache(dir_fd: int) -> bool:
    """Delete the old executable cache without opening its contents."""
    try:
        os.stat(_LEGACY_CACHE_PATH.name, dir_fd=dir_fd, follow_symlinks=False)
    except FileNotFoundError:
        return False
    os.unlink(_LEGACY_CACHE_PATH.name, dir_fd=dir_fd)
    try:
        os.stat(_CACHE_PATH.name, dir_fd=dir_fd, follow_symlinks=False)
        cache_exists = True
    except FileNotFoundError:
        cache_exists = False
    if not cache_exists:
        try:
            os.unlink(_MANIFEST_PATH.name, dir_fd=dir_fd)
        except FileNotFoundError:
            pass
    print(
        "S_net cache: retired legacy pickle without deserializing",
        file=sys.stderr,
    )
    return True


def retire_legacy_cache() -> bool:
    """Securely unlink the retired pickle artifact; never open its contents."""
    try:
        dir_fd = _open_cache_dir()
    except _CacheFormatError:
        return False
    try:
        return _retire_legacy_cache(dir_fd)
    finally:
        os.close(dir_fd)


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


def _atomic_write_json(path: Path, value: dict, dir_fd: int) -> None:
    encoded = (json.dumps(
        value, ensure_ascii=False, indent=2, allow_nan=False
    ) + "\n").encode("utf-8")
    temp_name = f".{path.name}.{secrets.token_hex(8)}.tmp"
    fd: int | None = None
    try:
        flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_CLOEXEC", 0)
        flags |= getattr(os, "O_NOFOLLOW", 0)
        fd = os.open(temp_name, flags, 0o600, dir_fd=dir_fd)
        view = memoryview(encoded)
        while view:
            view = view[os.write(fd, view):]
        os.fsync(fd)
        os.close(fd)
        fd = None
        os.replace(temp_name, path.name, src_dir_fd=dir_fd, dst_dir_fd=dir_fd)
        os.fsync(dir_fd)
    finally:
        if fd is not None:
            os.close(fd)
        try:
            os.unlink(temp_name, dir_fd=dir_fd)
        except FileNotFoundError:
            pass


def save_snet_cache(snet: SNet, manifest: dict) -> Path:
    """Stream S_net Dass to integrity-checked gzip JSONL.

    The cache file is replaced before the manifest. A crash between the two
    replacements leaves a digest mismatch, which the loader treats as a miss.
    """
    t0 = time.time()
    temp_name = f".snet-cache.{secrets.token_hex(8)}.tmp"
    temp_exists = False
    temp_fd: int | None = None
    dir_fd = _open_cache_dir(create=True)
    try:
        os.fchmod(dir_fd, 0o700)
        _retire_legacy_cache(dir_fd)
        _reject_unsafe_target_at(dir_fd, _CACHE_PATH.name)
        _reject_unsafe_target_at(dir_fd, _MANIFEST_PATH.name)
        flags = os.O_RDWR | os.O_CREAT | os.O_EXCL | getattr(os, "O_CLOEXEC", 0)
        flags |= getattr(os, "O_NOFOLLOW", 0)
        temp_fd = os.open(temp_name, flags, 0o600, dir_fd=dir_fd)
        temp_exists = True
        with os.fdopen(temp_fd, "w+b", closefd=False) as raw_stream:
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
            compressed_bytes = os.fstat(raw_stream.fileno()).st_size
            digest = _sha256_stream(raw_stream, compressed_bytes)
        if compressed_bytes <= 0 or compressed_bytes > _MAX_COMPRESSED_BYTES:
            raise ValueError("S_net cache exceeds the configured compressed size limit")
        os.close(temp_fd)
        temp_fd = None
        os.replace(temp_name, _CACHE_PATH.name, src_dir_fd=dir_fd, dst_dir_fd=dir_fd)
        temp_exists = False
        os.fsync(dir_fd)

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
        _atomic_write_json(_MANIFEST_PATH, stored_manifest, dir_fd)
        return _CACHE_PATH
    finally:
        if temp_fd is not None:
            os.close(temp_fd)
        if temp_exists:
            try:
                os.unlink(temp_name, dir_fd=dir_fd)
            except FileNotFoundError:
                pass
        os.close(dir_fd)


def _parse_signifier(record: dict) -> tuple[str, Signifier]:
    _expect_exact_keys(record, {"kind", "key", "value"}, "signifier record")
    key = _checked_string(record["key"], "signifier.key")
    value = _expect_object(record["value"], "signifier.value")
    _expect_exact_keys(
        value,
        {"id", "surface_forms", "source", "lang", "domain"},
        "signifier.value",
    )
    identifier = _checked_string(value["id"], "signifier.id")
    if identifier != key:
        raise _CacheFormatError("signifier.key must equal signifier.id")
    return key, Signifier(
        id=identifier,
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
    signifier_ids: set[str] = set()
    edge_ids: set[tuple[str, str, AxisType]] = set()
    hyperedge_ids: set[tuple] = set()
    total_bytes = 0
    record_count = 0
    decoded_list_items = 0

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
            decoded_list_items += _count_decoded_list_items(record)
            estimated_load = (
                total_bytes
                + record_count * _ESTIMATED_RECORD_OVERHEAD
                + decoded_list_items * _ESTIMATED_LIST_ITEM_BYTES
            )
            if estimated_load > _MAX_ESTIMATED_LOAD_BYTES:
                raise _CacheFormatError("cache estimated memory limit exceeded")
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
                    or type(record["schema_version"]) is not int
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
                if signifier.id in signifier_ids:
                    raise _CacheFormatError(f"duplicate signifier id: {signifier.id!r}")
                signifiers[key] = signifier
                signifier_ids.add(signifier.id)
                actual_counts["signifiers"] += 1
            elif kind == "edge":
                edge = _parse_edge(record)
                edge_id = (edge.source, edge.target, edge.axis)
                if edge_id in edge_ids:
                    raise _CacheFormatError(f"duplicate edge id: {edge_id!r}")
                edge_ids.add(edge_id)
                edges.append(edge)
                actual_counts["edges"] += 1
            elif kind == "morpheme":
                key, structure = _parse_morpheme(record)
                if key in morphemes:
                    raise _CacheFormatError(f"duplicate morpheme key: {key!r}")
                morphemes[key] = structure
                actual_counts["morphemes"] += 1
            elif kind == "hyperedge":
                hyperedge = _parse_hyperedge(record)
                hyperedge_id = (
                    hyperedge.vertices, hyperedge.source, hyperedge.domain,
                    hyperedge.timestamp, hyperedge.ingest_param_refs,
                    hyperedge.evidence_tag,
                )
                if hyperedge_id in hyperedge_ids:
                    raise _CacheFormatError("duplicate hyperedge id")
                hyperedge_ids.add(hyperedge_id)
                hyperedges.append(hyperedge)
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

    for edge in edges:
        if edge.source not in signifier_ids or edge.target not in signifier_ids:
            raise _CacheFormatError("edge references unknown signifier")
    for key, structure in morphemes.items():
        if key != structure.signifier_id or key not in signifier_ids:
            raise _CacheFormatError("morpheme references unknown or mismatched signifier")
        if any(
            ref not in signifier_ids
            for morpheme in structure.morphemes
            for ref in morpheme.shared_with
        ):
            raise _CacheFormatError("morpheme.shared_with references unknown signifier")
    for hyperedge in hyperedges:
        if not hyperedge.vertices.issubset(signifier_ids):
            raise _CacheFormatError("hyperedge references unknown signifier")

    # This constructor rebuilds all Was indexes from the validated Dass records.
    return SNet(
        signifiers=signifiers,
        edges=edges,
        morphemes=morphemes,
        hyperedges=hyperedges,
    )


def load_snet_cache() -> tuple[Optional[SNet], Optional[dict]]:
    """Load a validated JSONL cache, otherwise return a fail-closed miss."""
    dir_fd: int | None = None
    try:
        dir_fd = _open_cache_dir()
        retired = _retire_legacy_cache(dir_fd)
        try:
            os.stat(_CACHE_PATH.name, dir_fd=dir_fd, follow_symlinks=False)
            os.stat(_MANIFEST_PATH.name, dir_fd=dir_fd, follow_symlinks=False)
        except FileNotFoundError:
            if retired:
                print("S_net cache: full rebuild required after legacy retirement", file=sys.stderr)
            return None, None
        manifest = _load_manifest(dir_fd)
        info = _validate_cache_info(manifest)
        cache_fd, before = _open_regular_at(dir_fd, _CACHE_PATH.name)
        with os.fdopen(cache_fd, "rb") as cache_stream:
            compressed_bytes = before.st_size
            if compressed_bytes != info["compressed_bytes"]:
                raise _CacheFormatError("cache compressed size does not match manifest")
            if compressed_bytes > _MAX_COMPRESSED_BYTES:
                raise _CacheFormatError("cache compressed size limit exceeded")
            if _sha256_stream(cache_stream, compressed_bytes) != info["sha256"]:
                raise _CacheFormatError("cache digest mismatch")

            t0 = time.time()
            bounded_stream = _HardLimitReader(cache_stream, compressed_bytes)
            snet = _parse_jsonl_cache(info, bounded_stream)
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
    finally:
        if dir_fd is not None:
            os.close(dir_fd)


def invalidate_cache() -> None:
    """Delete current, legacy, and manifest artifacts to force a full ingest."""
    try:
        dir_fd = _open_cache_dir()
    except _CacheFormatError as exc:
        print(f"S_net cache invalidation refused unsafe directory: {exc}", file=sys.stderr)
        return
    try:
        for path in (_CACHE_PATH, _LEGACY_CACHE_PATH, _MANIFEST_PATH):
            try:
                os.unlink(path.name, dir_fd=dir_fd)
            except FileNotFoundError:
                pass
            except OSError as exc:
                print(f"S_net cache invalidation failed for {path}: {exc}", file=sys.stderr)
    finally:
        os.close(dir_fd)


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

    任何 manifest 不匹配都返回 miss，由 daemon 执行完整重建。
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

    print("S_net cache: manifest mismatch, full rebuild needed", file=sys.stderr)
    return None, False


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
