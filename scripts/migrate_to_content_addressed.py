#!/usr/bin/env python
"""区块拓扑迁移至 Content-Addressed Storage.

将现有区块（索引式 block_id = SHA256(metadata)）迁移为
content-addressed 区块（block_id = SHA256(full_text)）。

流程:
  1. 遍历所有现有区块
  2. 对每个区块，定位关联的谱系 .md 文件
  3. 将完整文本写入 content.full_text
  4. 重新计算 block_id = SHA256(full_text)
  5. 创建新区块文件（以新 hash 命名），删除旧区块文件
  6. 更新 meta.json 的 id_mapping
  7. 更新 relations.jsonl（旧 hash → 新 hash）
  8. 输出迁移报告

用法:
  python scripts/migrate_to_content_addressed.py                # dry-run（只报告）
  python scripts/migrate_to_content_addressed.py --execute      # 实际执行迁移
  python scripts/migrate_to_content_addressed.py --json         # JSON 输出
  python scripts/migrate_to_content_addressed.py --root /path   # 指定根目录

约束:
  - 幂等：可安全多次运行
  - .md 文件保留（工作副本）
  - 向后兼容：处理新旧两种区块格式

谱系位置: 347号——content-addressed block topology
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from scripts.block_ops import compute_content_hash
from scripts.verify_block_integrity import (
    get_block_id,
    get_genealogy_number,
    resolve_source_file,
    scan_blocks,
)


def _is_already_content_addressed(block: dict) -> bool:
    """Check if a block is already in content-addressed format.

    A content-addressed block has content.full_text and its id == SHA256(full_text).
    """
    content = block.get("content", {})
    if not isinstance(content, dict):
        return False
    full_text = content.get("full_text")
    if not full_text:
        return False
    expected_id = compute_content_hash(full_text)
    return block.get("id") == expected_id


def plan_migration(root: Path) -> dict:
    """Analyze blocks and plan migration without making changes.

    Returns a report dict with:
      - total: total blocks
      - to_migrate: blocks that need migration
      - already_migrated: blocks already content-addressed
      - no_source: blocks without .md files (will be kept as-is)
      - missing_file: blocks referencing missing .md files
      - details: list of per-block migration plans
    """
    blocks_dir = root / ".chanlun" / "block-topology" / "blocks"
    genealogy_dir = root / ".chanlun" / "genealogy"

    blocks = scan_blocks(blocks_dir)

    report = {
        "total": len(blocks),
        "to_migrate": 0,
        "already_migrated": 0,
        "no_source": 0,
        "missing_file": 0,
        "details": [],
    }

    for block in blocks:
        old_id = get_block_id(block)
        gen_num = get_genealogy_number(block)

        # Already content-addressed?
        if _is_already_content_addressed(block):
            report["already_migrated"] += 1
            report["details"].append({
                "action": "skip_already_migrated",
                "old_id": old_id,
                "genealogy": gen_num,
            })
            continue

        # Resolve source file
        source_file = resolve_source_file(block, genealogy_dir)

        if source_file is None:
            # Check if the block has a source reference at all
            content = block.get("content", {})
            has_ref = (
                (isinstance(content, dict) and content.get("source_file"))
                or block.get("genealogy_id")
            )
            if has_ref:
                report["missing_file"] += 1
                report["details"].append({
                    "action": "error_missing_file",
                    "old_id": old_id,
                    "genealogy": gen_num,
                })
            else:
                # No source file reference — keep block as-is
                report["no_source"] += 1
                report["details"].append({
                    "action": "skip_no_source",
                    "old_id": old_id,
                    "genealogy": gen_num,
                })
            continue

        # Needs migration
        full_text = source_file.read_text(encoding="utf-8")
        new_id = compute_content_hash(full_text)

        report["to_migrate"] += 1
        report["details"].append({
            "action": "migrate",
            "old_id": old_id,
            "new_id": new_id,
            "genealogy": gen_num,
            "source_file": str(source_file.relative_to(root)),
            "id_changed": old_id != new_id,
        })

    return report


def execute_migration(root: Path) -> dict:
    """Execute the content-addressed migration.

    Returns a result dict with migration statistics.
    """
    blocks_dir = root / ".chanlun" / "block-topology" / "blocks"
    genealogy_dir = root / ".chanlun" / "genealogy"
    meta_path = root / ".chanlun" / "block-topology" / "meta.json"
    relations_path = root / ".chanlun" / "block-topology" / "relations.jsonl"

    blocks = scan_blocks(blocks_dir)

    # Build old_id → new_id mapping
    id_remap: dict[str, str] = {}
    migrated = 0
    skipped_already = 0
    skipped_no_source = 0
    errors: list[str] = []

    for block in blocks:
        old_id = get_block_id(block)

        # Already content-addressed?
        if _is_already_content_addressed(block):
            skipped_already += 1
            continue

        # Resolve source file
        source_file = resolve_source_file(block, genealogy_dir)
        if source_file is None:
            skipped_no_source += 1
            continue

        # Read full text and compute new ID
        full_text = source_file.read_text(encoding="utf-8")
        new_id = compute_content_hash(full_text)

        # Build the new block
        new_block = dict(block)
        new_block["id"] = new_id

        # Embed full_text into content
        content = new_block.get("content", {})
        if not isinstance(content, dict):
            content = {}
        content["full_text"] = full_text
        new_block["content"] = content

        # Remove legacy content_hash (superseded by content-addressing)
        new_block.pop("content_hash", None)

        # Write new block file
        new_path = blocks_dir / f"{new_id}.json"
        if not new_path.is_file():
            new_path.write_text(
                json.dumps(new_block, ensure_ascii=False, indent=2),
                encoding="utf-8",
            )

        # Delete old block file if ID changed
        if old_id != new_id:
            old_path = blocks_dir / f"{old_id}.json"
            if old_path.is_file():
                old_path.unlink()
            id_remap[old_id] = new_id

        migrated += 1

    # Update meta.json id_mapping
    meta_updated = False
    if meta_path.is_file() and id_remap:
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
        id_mapping = meta.get("id_mapping", {})
        updated_mapping = {}
        for gen_id, block_id in id_mapping.items():
            updated_mapping[gen_id] = id_remap.get(block_id, block_id)
        meta["id_mapping"] = updated_mapping

        # Update genesis_block_id if it was remapped
        genesis = meta.get("genesis_block_id", "")
        if genesis in id_remap:
            meta["genesis_block_id"] = id_remap[genesis]

        meta_path.write_text(
            json.dumps(meta, ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
        meta_updated = True

    # Update relations.jsonl
    relations_updated = 0
    if relations_path.is_file() and id_remap:
        lines = relations_path.read_text(encoding="utf-8").strip().split("\n")
        new_lines = []
        for line in lines:
            if not line.strip():
                continue
            rel = json.loads(line)
            changed = False
            for field in ("from", "to", "created_by"):
                old_val = rel.get(field, "")
                if old_val in id_remap:
                    rel[field] = id_remap[old_val]
                    changed = True
            new_lines.append(
                json.dumps(rel, ensure_ascii=False, separators=(",", ":"))
            )
            if changed:
                relations_updated += 1
        relations_path.write_text(
            "\n".join(new_lines) + "\n",
            encoding="utf-8",
        )

    return {
        "migrated": migrated,
        "skipped_already_content_addressed": skipped_already,
        "skipped_no_source": skipped_no_source,
        "id_remapped": len(id_remap),
        "meta_updated": meta_updated,
        "relations_updated": relations_updated,
        "errors": errors,
    }


def format_report(report: dict, *, is_plan: bool = True) -> str:
    """Format migration report as human-readable text."""
    lines = []
    if is_plan:
        lines.append("=== Content-Addressed Migration Plan (dry-run) ===")
        lines.append("")
        lines.append(f"Total blocks: {report['total']}")
        lines.append(f"To migrate: {report['to_migrate']}")
        lines.append(f"Already content-addressed: {report['already_migrated']}")
        lines.append(f"No source file (keep as-is): {report['no_source']}")
        lines.append(f"Missing source file (error): {report['missing_file']}")

        id_changes = sum(
            1 for d in report["details"]
            if d.get("action") == "migrate" and d.get("id_changed")
        )
        lines.append(f"ID changes: {id_changes}")
    else:
        lines.append("=== Content-Addressed Migration Result ===")
        lines.append("")
        lines.append(f"Migrated: {report['migrated']}")
        lines.append(
            f"Skipped (already content-addressed): "
            f"{report['skipped_already_content_addressed']}"
        )
        lines.append(f"Skipped (no source): {report['skipped_no_source']}")
        lines.append(f"IDs remapped: {report['id_remapped']}")
        lines.append(f"Meta updated: {report['meta_updated']}")
        lines.append(f"Relations updated: {report['relations_updated']}")
        if report["errors"]:
            lines.append("")
            lines.append("Errors:")
            for e in report["errors"]:
                lines.append(f"  - {e}")

    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(
        description="区块拓扑迁移至 content-addressed storage"
    )
    parser.add_argument(
        "--execute",
        action="store_true",
        help="实际执行迁移（不加此参数为 dry-run）",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        dest="json_output",
        help="JSON 格式输出",
    )
    parser.add_argument(
        "--root",
        default=".",
        help="项目根目录（默认当前目录）",
    )

    args = parser.parse_args()
    root = Path(args.root).resolve()

    if args.execute:
        result = execute_migration(root)
        if args.json_output:
            print(json.dumps(result, ensure_ascii=False, indent=2))
        else:
            print(format_report(result, is_plan=False))
    else:
        report = plan_migration(root)
        if args.json_output:
            print(json.dumps(report, ensure_ascii=False, indent=2))
        else:
            print(format_report(report, is_plan=True))


if __name__ == "__main__":
    main()
