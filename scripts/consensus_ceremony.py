"""
共识仪式——Gemini/Codex 质询循环收敛时的三区块原子写入。

总方针 §21: 每次质询循环收敛产生三个区块：
- consensus: 最终达成的结论（CC 看到的表面）
- residue: 双方各自放弃了什么（objet a 的物质记录）
- tension: 双方承认未解决但选择搁置的部分

三区块通过临时目录策略原子写入，复用 block_topology.py 的底层函数。
"""

from __future__ import annotations

import json
import shutil
import tempfile
from pathlib import Path

from scripts.block_topology import (
    DEFAULT_BASE,
    list_blocks,
    make_block,
    make_relation,
    query_relations,
    read_block,
)


def _atomic_write_ceremony(
    blocks: list[dict],
    relations: list[dict],
    base: Path,
) -> None:
    """Atomic write of multiple blocks and relations.

    Strategy: prepare all files in a temp directory under base,
    then move each to official location. If anything fails mid-way,
    temp dir is cleaned up; already-moved files are idempotent
    (content-addressed), so partial moves don't corrupt state.
    """
    blocks_dir = base / "blocks"
    blocks_dir.mkdir(parents=True, exist_ok=True)

    tmp_dir = Path(tempfile.mkdtemp(prefix="cc-", dir=base))
    try:
        tmp_blocks = tmp_dir / "blocks"
        tmp_blocks.mkdir()

        # Write all blocks to temp
        for blk in blocks:
            path = tmp_blocks / f"{blk['id']}.json"
            path.write_text(
                json.dumps(blk, ensure_ascii=False, indent=2),
                encoding="utf-8",
            )

        # Write all relations to temp
        tmp_jsonl = tmp_dir / "relations.jsonl"
        with open(tmp_jsonl, "w", encoding="utf-8") as f:
            for rel in relations:
                f.write(
                    json.dumps(rel, ensure_ascii=False, separators=(",", ":"))
                    + "\n"
                )

        # Move blocks to official location
        for blk_file in tmp_blocks.iterdir():
            dest = blocks_dir / blk_file.name
            if not dest.exists():
                shutil.move(str(blk_file), str(dest))

        # Append relations to official JSONL
        official_jsonl = base / "relations.jsonl"
        with open(tmp_jsonl, "r", encoding="utf-8") as src:
            with open(official_jsonl, "a", encoding="utf-8") as dst:
                dst.write(src.read())

    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)


def write_consensus_ceremony(
    trigger_block_id: str,
    conclusion: str,
    gemini_conceded: list[str],
    codex_conceded: list[str],
    concession_reasons: dict[str, str],
    unresolved: list[str],
    source: str = "cc",
    base: Path = DEFAULT_BASE,
) -> dict[str, dict]:
    """Atomic write of the three-block consensus ceremony.

    Returns dict with keys "consensus", "residue", "tension",
    each mapped to the block dict.

    Produces 5 blocks total (3 primary + 2 rewrite) and 4 relations
    (2 order-1 + 2 order-2).
    """
    # 1. consensus block
    consensus = make_block(
        block_type="consensus",
        source=source,
        content={"conclusion": conclusion},
        refs=[trigger_block_id],
    )

    # 2. residue block
    residue = make_block(
        block_type="residue",
        source=source,
        content={
            "gemini_conceded": gemini_conceded,
            "codex_conceded": codex_conceded,
            "reasons": concession_reasons,
        },
        refs=[consensus["id"]],
    )

    # 3. tension block
    tension = make_block(
        block_type="tension",
        source=source,
        content={"unresolved": unresolved},
        refs=[consensus["id"]],
    )

    # 4. Order-1 relations
    rel_residue = make_relation(
        from_id=residue["id"],
        to_id=consensus["id"],
        relation="residue_of",
        order=1,
        created_by=residue["id"],
    )
    rel_tension = make_relation(
        from_id=tension["id"],
        to_id=consensus["id"],
        relation="tensions_with",
        order=1,
        created_by=tension["id"],
    )

    # 5. Order-1 relations trigger rewrite blocks (Q2 rule)
    rewrite_residue = make_block(
        block_type="rewrite",
        source=source,
        content={
            "action": "relation_append",
            "primary_block": residue["id"],
            "relation_count": 1,
        },
        refs=[residue["id"]],
    )
    rewrite_tension = make_block(
        block_type="rewrite",
        source=source,
        content={
            "action": "relation_append",
            "primary_block": tension["id"],
            "relation_count": 1,
        },
        refs=[tension["id"]],
    )

    # 6. Order-2 "records" relations (rewrite → primary, no further rewrite)
    records_residue = make_relation(
        from_id=rewrite_residue["id"],
        to_id=residue["id"],
        relation="records",
        order=2,
        created_by=rewrite_residue["id"],
    )
    records_tension = make_relation(
        from_id=rewrite_tension["id"],
        to_id=tension["id"],
        relation="records",
        order=2,
        created_by=rewrite_tension["id"],
    )

    # 7. Atomic write: 5 blocks + 4 relations
    all_blocks = [consensus, residue, tension, rewrite_residue, rewrite_tension]
    all_relations = [
        rel_residue, rel_tension, records_residue, records_tension,
    ]
    _atomic_write_ceremony(all_blocks, all_relations, base)

    return {
        "consensus": consensus,
        "residue": residue,
        "tension": tension,
    }


def detect_residue_accumulation(
    base: Path = DEFAULT_BASE,
    threshold: int = 3,
) -> list[dict]:
    """Detect areas where residue blocks accumulate beyond threshold.

    Scans all residue blocks, traces back through consensus to the
    trigger block, and groups by the trigger's genealogy identifier.

    Returns list of dicts: {"area": str, "count": int, "block_ids": [...]}
    for areas exceeding threshold.
    """
    residues = list_blocks(base, block_type="residue")

    # Group by trigger area
    area_map: dict[str, list[str]] = {}
    for res in residues:
        # residue.refs → consensus id
        if not res.get("refs"):
            continue
        consensus_id = res["refs"][0]
        consensus_blk = read_block(consensus_id, base)
        if consensus_blk is None or not consensus_blk.get("refs"):
            continue
        trigger_id = consensus_blk["refs"][0]
        trigger_blk = read_block(trigger_id, base)
        if trigger_blk is None:
            # Trigger block may be external; use its id as area key
            area_key = trigger_id
        else:
            # Use genealogy_id from content if available, else block id
            area_key = trigger_blk.get("content", {}).get(
                "genealogy_id", trigger_id,
            )
        area_map.setdefault(area_key, []).append(res["id"])

    return [
        {"area": area, "count": len(ids), "block_ids": ids}
        for area, ids in area_map.items()
        if len(ids) >= threshold
    ]


def list_open_tensions(base: Path = DEFAULT_BASE) -> list[dict]:
    """List all tension blocks."""
    return list_blocks(base, block_type="tension")


def get_ceremony_chain(
    consensus_block_id: str,
    base: Path = DEFAULT_BASE,
) -> dict[str, dict | None]:
    """Given a consensus block id, return the associated residue and tension.

    Returns {"consensus": dict, "residue": dict|None, "tension": dict|None}.
    """
    consensus_blk = read_block(consensus_block_id, base)

    # Find residue: relation residue_of pointing to this consensus
    residue_rels = query_relations(
        base, block_id=consensus_block_id, relation="residue_of",
    )
    residue_blk = None
    for rel in residue_rels:
        if rel["to"] == consensus_block_id:
            residue_blk = read_block(rel["from"], base)
            break

    # Find tension: relation tensions_with pointing to this consensus
    tension_rels = query_relations(
        base, block_id=consensus_block_id, relation="tensions_with",
    )
    tension_blk = None
    for rel in tension_rels:
        if rel["to"] == consensus_block_id:
            tension_blk = read_block(rel["from"], base)
            break

    return {
        "consensus": consensus_blk,
        "residue": residue_blk,
        "tension": tension_blk,
    }
