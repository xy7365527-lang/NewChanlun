"""
区块拓扑核心模块——Schema 定义 + 写入/读取函数

事件不可变，拓扑可变。
- Event Layer: SHA256 内容寻址的 JSON 区块（不可变）
- Relation Layer: JSONL 追加式关系图（可变，追加式）
- Reflexive Layer: 一阶关系触发 rewrite 区块（二阶不触发——递归恰好一步）

编排者决断：
- Q1: 升格——177条现有谱系一次性转换为区块
- Q2: 阶的区分——order 1 触发 rewrite，order 2 只记录
- Q3: 独立 SHA256 寻址空间（不用 git objects）
- refs vs relations: refs = 创建时直接因果，relations = 结构关系（边表）
- migration 区块 refs 一律为空
"""

from __future__ import annotations

import functools
import hashlib
import json
import re
import shutil
import tempfile
from datetime import datetime, timezone
from pathlib import Path

# --- Constants ---

BLOCK_TYPES = frozenset({
    "event", "consensus", "residue", "tension", "rewrite",
    "annotation",   # Phase 2 预留，Phase 3 自动生成
})

SOURCES = frozenset({
    "cc", "gemini", "codex", "migration", "serena",
})

RELATION_TYPES = frozenset({
    "depends_on", "negates", "related", "tensions_with",  # dag.yaml 迁移
    "supersedes", "residue_of", "reopens",                 # 共识/剩余关系
    "freezes", "splits", "severs",                         # 拓扑操作
    "records",                                             # rewrite→event 记录关系
    "negated_by",                                          # dag.yaml negated_by
    "defines",                                             # 区块定义概念（内容级）
    "modifies",                                            # 向后兼容（Phase 1 已写入，双栈过渡期保留）
    "refines",                                             # 折叠保持：局部修正（order=2）
    "revises",                                             # 折叠变形：实质性修改（order=1）
    "references",                                          # 正文引用另一区块（内容级）
    "annotates",                                           # annotation → 被注解 block（Phase 2 预留）
})

# 边有效性能力矩阵：哪些关系类型改变边有效性
# negates（撕裂）：改变有效性
# revises（变形）：改变有效性
# supersedes 不纳入：坍缩不可逆，不是有效性变更而是替代
VALIDITY_CAPABLE_RELATIONS = frozenset({"negates", "revises"})

# --- Layer Classification ---

LAYER_MAP: dict[str, int] = {
    # Layer 1（逻辑层）
    "depends_on": 1, "negates": 1, "negated_by": 1, "supersedes": 1,
    "residue_of": 1, "reopens": 1, "tensions_with": 1,
    "freezes": 1, "splits": 1, "severs": 1,
    # Layer 2（导航层）
    "references": 2, "defines": 2, "modifies": 2,
    "refines": 2, "revises": 2, "annotates": 2,
    # Layer 3（元数据层）
    "records": 3, "related": 3,
}


def classify_layer(relation_type: str) -> int:
    """关系类型 → 层编号。未知类型抛 ValueError。"""
    try:
        return LAYER_MAP[relation_type]
    except KeyError:
        raise ValueError(f"Unknown relation type: {relation_type}")


DEFAULT_BASE = Path(".chanlun/block-topology")

_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


@functools.lru_cache(maxsize=1)
def get_block_mapping(
    base: Path = DEFAULT_BASE,
) -> tuple[dict[str, int], dict[int, str]]:
    """block_id <-> genealogy_number 双向映射。进程级缓存。

    从 blocks 目录读取所有区块 JSON，提取 genealogy_number。
    查找顺序：content.genealogy_number → content.number → content.id（数字字符串）。
    没有 genealogy_number 的区块（如 rewrite 类型）不纳入映射。
    """
    id2num: dict[str, int] = {}
    num2id: dict[int, str] = {}
    blocks_dir = base / "blocks"
    if not blocks_dir.exists():
        return id2num, num2id
    for f in blocks_dir.iterdir():
        if f.suffix != ".json":
            continue
        blk = json.loads(f.read_text(encoding="utf-8"))
        bid = blk.get("id", f.stem)
        content = blk.get("content", {})
        num = content.get("genealogy_number") or content.get("number")
        if num is None:
            # Fallback: content.id that looks numeric (migration blocks)
            cid = content.get("id")
            if isinstance(cid, str) and cid.isdigit():
                num = cid
            elif isinstance(cid, int):
                num = cid
        if num is not None:
            try:
                num = int(num)
            except (ValueError, TypeError):
                continue
            id2num[bid] = num
            num2id[num] = bid
    return id2num, num2id


def make_concept_id(term: str) -> str:
    """Generate a deterministic SHA256 id for a concept term.

    Uses the prefix "concept:" to namespace concept ids and avoid
    collisions with block ids.  Term is stripped and NFKC-normalized
    so that stylistic whitespace / Unicode variants map to the same id.
    """
    import unicodedata
    normalized = unicodedata.normalize("NFKC", term.strip())
    return hashlib.sha256(f"concept:{normalized}".encode("utf-8")).hexdigest()


def _validate_sha256(value: str, field_name: str) -> None:
    """Validate that a string is a valid SHA256 hex digest."""
    if not _SHA256_RE.match(value):
        raise ValueError(
            f"{field_name} must be a SHA256 hex digest (64 hex chars), "
            f"got: {value!r}"
        )


# --- Schema / Hash ---

def canonical_json(obj: dict) -> bytes:
    """Deterministic JSON serialization for hash computation.

    sorted keys, no whitespace, ensure_ascii=False for Chinese content.
    """
    return json.dumps(obj, sort_keys=True, separators=(",", ":"),
                      ensure_ascii=False).encode("utf-8")


def compute_block_id(block_type: str, source: str, content: dict,
                     refs: list[str]) -> str:
    """Compute SHA256 block id from canonical content.

    timestamp and git_ref are excluded — same logical event always produces
    the same id regardless of when it's written (idempotent).
    """
    canonical = {
        "type": block_type,
        "source": source,
        "content": content,
        "refs": refs,
    }
    return hashlib.sha256(canonical_json(canonical)).hexdigest()


def make_block(block_type: str, source: str, content: dict,
               refs: list[str] | None = None,
               git_ref: str = "") -> dict:
    """Create a complete block dict (with id and timestamp)."""
    if block_type not in BLOCK_TYPES:
        raise ValueError(f"Invalid block_type: {block_type!r}. "
                         f"Must be one of {sorted(BLOCK_TYPES)}")
    if source not in SOURCES:
        raise ValueError(f"Invalid source: {source!r}. "
                         f"Must be one of {sorted(SOURCES)}")
    if refs is None:
        refs = []
    block_id = compute_block_id(block_type, source, content, refs)
    return {
        "id": block_id,
        "type": block_type,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "source": source,
        "content": content,
        "refs": refs,
        "git_ref": git_ref,
    }


VALIDITY_VALUES = frozenset({"active", "invalidated"})


def make_relation(from_id: str, to_id: str, relation: str, order: int,
                  created_by: str, **extra) -> dict:
    """Create a relation record dict.

    created_by must be a SHA256 hex digest — enforces type consistency
    (编排者决断: created_by 字段类型始终为 SHA256).

    For relations in VALIDITY_CAPABLE_RELATIONS (negates, revises),
    the following optional fields are supported (273号 schema 扩展):
      - validity: "active" | "invalidated", defaults to "active"
      - invalidated_by: SHA256 of the block that invalidated this edge
      - invalidated_at: ISO 8601 timestamp of invalidation
    """
    _validate_sha256(created_by, "created_by")
    if relation not in RELATION_TYPES:
        raise ValueError(f"Invalid relation: {relation!r}. "
                         f"Must be one of {sorted(RELATION_TYPES)}")
    if order not in (1, 2):
        raise ValueError(f"Invalid order: {order!r}. Must be 1 or 2")
    rec = {
        "from": from_id,
        "to": to_id,
        "relation": relation,
        "order": order,
        "created_by": created_by,
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }
    # 273号: VALIDITY_CAPABLE_RELATIONS 边默认 validity="active"
    if relation in VALIDITY_CAPABLE_RELATIONS:
        rec["validity"] = extra.pop("validity", "active")
        rec["invalidated_by"] = extra.pop("invalidated_by", None)
        rec["invalidated_at"] = extra.pop("invalidated_at", None)
        if rec["validity"] not in VALIDITY_VALUES:
            raise ValueError(
                f"Invalid validity: {rec['validity']!r}. "
                f"Must be one of {sorted(VALIDITY_VALUES)}"
            )
    rec.update(extra)
    return rec


def _normalize_relation(rec: dict) -> dict:
    """Normalize a relation dict for backward compatibility (273号).

    Adds default validity fields to VALIDITY_CAPABLE_RELATIONS that lack them.
    Returns a new dict — does not mutate the input.
    """
    if rec.get("relation") not in VALIDITY_CAPABLE_RELATIONS:
        return dict(rec)
    if "validity" in rec:
        return dict(rec)
    normalized = dict(rec)
    normalized["validity"] = "active"
    normalized["invalidated_by"] = None
    normalized["invalidated_at"] = None
    return normalized


def invalidate_relation(relation: dict, invalidated_by_block_id: str) -> dict:
    """Mark a validity-capable relation as invalidated (273号).

    Returns a new dict with validity="invalidated", invalidated_by and
    invalidated_at set. Does not mutate the input dict. Does not write
    to disk — the caller decides when to persist.

    Idempotent: if the relation is already invalidated, returns a copy
    without changing the state (273号边界条件3).
    """
    if relation.get("relation") not in VALIDITY_CAPABLE_RELATIONS:
        raise ValueError(
            "invalidate_relation only applies to "
            f"{sorted(VALIDITY_CAPABLE_RELATIONS)} relations, "
            f"got relation={relation.get('relation')!r}"
        )
    result = dict(relation)
    # 已经 invalidated 的边不能被再次 invalidated（幂等）
    if result.get("validity") == "invalidated":
        return result
    result["validity"] = "invalidated"
    result["invalidated_by"] = invalidated_by_block_id
    result["invalidated_at"] = datetime.now(timezone.utc).isoformat()
    return result


# --- File I/O ---

def _ensure_dirs(base: Path) -> None:
    """Ensure block-topology directory structure exists."""
    (base / "blocks").mkdir(parents=True, exist_ok=True)


def block_path(base: Path, block_id: str) -> Path:
    return base / "blocks" / f"{block_id}.json"


def write_block(block: dict, base: Path = DEFAULT_BASE) -> Path:
    """Write a single block to disk. Idempotent — same content = same file.

    Returns the path of the written block.
    """
    _ensure_dirs(base)
    path = block_path(base, block["id"])
    if path.exists():
        return path  # idempotent: already written
    content = json.dumps(block, ensure_ascii=False, indent=2)
    path.write_text(content, encoding="utf-8")
    return path


def append_relation(relation: dict, base: Path = DEFAULT_BASE) -> None:
    """Append a single relation record to relations.jsonl."""
    _ensure_dirs(base)
    jsonl_path = base / "relations.jsonl"
    line = json.dumps(relation, ensure_ascii=False, separators=(",", ":"))
    with open(jsonl_path, "a", encoding="utf-8") as f:
        f.write(line + "\n")


def write_block_with_relations(
    block_type: str,
    source: str,
    content: dict,
    refs: list[str] | None = None,
    relations: list[dict] | None = None,
    git_ref: str = "",
    base: Path = DEFAULT_BASE,
) -> dict:
    """Atomic write: block + relations + rewrite (if order 1 relations exist).

    Strategy: prepare all files in a temp directory, then move to official
    location. Failure discards temp dir.

    Returns the primary block dict.
    """
    if refs is None:
        refs = []
    if relations is None:
        relations = []

    primary = make_block(block_type, source, content, refs, git_ref)

    # Collect all blocks and relations to write
    blocks_to_write = [primary]
    relations_to_write = list(relations)

    # Check if any relation is order 1 — needs rewrite block
    has_order_1 = any(r.get("order") == 1 for r in relations_to_write)

    if has_order_1:
        # Create rewrite block recording this relation-layer change
        rewrite_content = {
            "action": "relation_append",
            "primary_block": primary["id"],
            "relation_count": len([r for r in relations_to_write
                                   if r.get("order") == 1]),
        }
        rewrite = make_block("rewrite", source, rewrite_content,
                             refs=[primary["id"]])
        blocks_to_write.append(rewrite)

        # Add order-2 "records" relation: rewrite → primary
        records_rel = make_relation(
            from_id=rewrite["id"],
            to_id=primary["id"],
            relation="records",
            order=2,
            created_by=rewrite["id"],
        )
        relations_to_write.append(records_rel)

    # Atomic write via temp directory
    _ensure_dirs(base)
    tmp_dir = Path(tempfile.mkdtemp(prefix="bt-", dir=base))
    try:
        tmp_blocks = tmp_dir / "blocks"
        tmp_blocks.mkdir()

        # Write blocks to temp
        for blk in blocks_to_write:
            path = tmp_blocks / f"{blk['id']}.json"
            path.write_text(
                json.dumps(blk, ensure_ascii=False, indent=2),
                encoding="utf-8",
            )

        # Write relations to temp
        tmp_jsonl = tmp_dir / "relations.jsonl"
        with open(tmp_jsonl, "w", encoding="utf-8") as f:
            for rel in relations_to_write:
                f.write(json.dumps(rel, ensure_ascii=False,
                                   separators=(",", ":")) + "\n")

        # Move blocks to official location
        official_blocks = base / "blocks"
        for blk_file in tmp_blocks.iterdir():
            dest = official_blocks / blk_file.name
            if not dest.exists():
                shutil.move(str(blk_file), str(dest))

        # Append relations to official JSONL
        official_jsonl = base / "relations.jsonl"
        with open(tmp_jsonl, "r", encoding="utf-8") as src:
            with open(official_jsonl, "a", encoding="utf-8") as dst:
                dst.write(src.read())

    finally:
        # Clean up temp dir
        shutil.rmtree(tmp_dir, ignore_errors=True)

    return primary


# --- Read / Query ---

def read_block(block_id: str, base: Path = DEFAULT_BASE) -> dict | None:
    """Read a block by id. Returns None if not found."""
    path = block_path(base, block_id)
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def read_all_relations(base: Path = DEFAULT_BASE) -> list[dict]:
    """Read all relations from JSONL.

    Applies backward-compatible normalization (273号): negates relations
    without validity fields get default values.
    """
    jsonl_path = base / "relations.jsonl"
    if not jsonl_path.exists():
        return []
    relations = []
    for line in jsonl_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line:
            relations.append(_normalize_relation(json.loads(line)))
    return relations


def query_relations(
    base: Path = DEFAULT_BASE,
    block_id: str | None = None,
    relation: str | None = None,
    order: int | None = None,
) -> list[dict]:
    """Query relations with optional filters.

    - block_id: match from or to
    - relation: match relation type
    - order: match order (1 or 2)
    """
    results = []
    for rec in read_all_relations(base):
        if block_id and rec["from"] != block_id and rec["to"] != block_id:
            continue
        if relation and rec["relation"] != relation:
            continue
        if order is not None and rec.get("order") != order:
            continue
        results.append(rec)
    return results


def list_blocks(base: Path = DEFAULT_BASE,
                block_type: str | None = None) -> list[dict]:
    """List all blocks, optionally filtered by type."""
    blocks_dir = base / "blocks"
    if not blocks_dir.exists():
        return []
    results = []
    for f in blocks_dir.iterdir():
        if f.suffix == ".json":
            blk = json.loads(f.read_text(encoding="utf-8"))
            if block_type is None or blk.get("type") == block_type:
                results.append(blk)
    return results


# --- Meta ---

def write_meta(meta: dict, base: Path = DEFAULT_BASE) -> None:
    """Write meta.json (topology metadata)."""
    _ensure_dirs(base)
    path = base / "meta.json"
    path.write_text(
        json.dumps(meta, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )


def read_meta(base: Path = DEFAULT_BASE) -> dict | None:
    """Read meta.json. Returns None if not found."""
    path = base / "meta.json"
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))
