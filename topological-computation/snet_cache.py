"""snet_cache.py -- S_net 持久化缓存 + 增量摄入。

方案1: 序列化/反序列化 S_net 到磁盘，跳过重复摄入。
方案2: manifest 驱动增量摄入——只处理新增/变更文件。

缓存目录: ~/.swarm/persist/snet_cache/
  snet_cache.pkl.gz   — gzip 压缩的 pickle（S_net 完整快照）
  manifest.json       — 文件列表 + hash + mtime，用于判断缓存有效性

认识论等级: L0（序列化/反序列化是无损代数操作）

谱系引用: 首次实装。
"""

from __future__ import annotations

import gzip
import hashlib
import json
import os
import pickle
import sys
import time
from pathlib import Path
from typing import Optional

from signifier_net import SNet


# ---------------------------------------------------------------------------
# 缓存路径
# ---------------------------------------------------------------------------

_CACHE_DIR = Path.home() / ".swarm" / "persist" / "snet_cache"
_CACHE_PATH = _CACHE_DIR / "snet_cache.pkl.gz"
_MANIFEST_PATH = _CACHE_DIR / "manifest.json"


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

def save_snet_cache(snet: SNet, manifest: dict) -> Path:
    """将 S_net 序列化到缓存文件。

    使用 pickle + gzip 压缩。pickle 序列化 SNet.to_dict() 的纯 Python dict
    （不直接 pickle SNet 对象，避免类结构变更导致反序列化失败）。

    返回缓存文件路径。
    """
    _CACHE_DIR.mkdir(parents=True, exist_ok=True)

    # 序列化 SNet 为 dict，再 pickle + gzip
    snet_data = snet.to_dict()
    t0 = time.time()
    with gzip.open(_CACHE_PATH, "wb", compresslevel=4) as f:
        pickle.dump(snet_data, f, protocol=pickle.HIGHEST_PROTOCOL)
    elapsed = time.time() - t0

    # 保存 manifest
    n_edges = snet.edge_count if hasattr(snet, 'edge_count') else len(snet._edges)
    manifest["cache_info"] = {
        "n_signifiers": len(snet._signifiers),
        "n_edges": n_edges,
        "n_morphemes": len(snet._morphemes),
        "save_time": elapsed,
        "cache_path": str(_CACHE_PATH),
    }
    with open(_MANIFEST_PATH, "w", encoding="utf-8") as f:
        json.dump(manifest, f, ensure_ascii=False, indent=2)

    return _CACHE_PATH


def load_snet_cache() -> tuple[Optional[SNet], Optional[dict]]:
    """从缓存文件加载 S_net。

    返回 (snet, manifest) 或 (None, None) 如果缓存不存在或损坏。
    """
    if not _CACHE_PATH.exists() or not _MANIFEST_PATH.exists():
        return None, None

    try:
        # 加载 manifest
        with open(_MANIFEST_PATH, "r", encoding="utf-8") as f:
            manifest = json.load(f)

        # 加载 SNet
        t0 = time.time()
        with gzip.open(_CACHE_PATH, "rb") as f:
            snet_data = pickle.load(f)
        snet = SNet.from_dict(snet_data)
        elapsed = time.time() - t0

        print(
            f"S_net cache loaded: {len(snet._signifiers)} signifiers, "
            f"{len(snet._edges)} edges, {len(snet._morphemes)} morphemes "
            f"({elapsed:.1f}s)",
            file=sys.stderr,
        )
        return snet, manifest

    except Exception as exc:
        print(f"S_net cache load failed: {exc}", file=sys.stderr)
        return None, None


def invalidate_cache() -> None:
    """删除缓存文件（强制下次完整摄入）。"""
    try:
        if _CACHE_PATH.exists():
            _CACHE_PATH.unlink()
        if _MANIFEST_PATH.exists():
            _MANIFEST_PATH.unlink()
    except OSError as exc:
        print(f"S_net cache invalidation failed: {exc}", file=sys.stderr)


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
