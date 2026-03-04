#!/usr/bin/env python
"""区块拓扑内容完整性验证。

功能:
  1. 遍历所有区块，定位关联的谱系文件
  2. 计算谱系文件的 SHA256，与区块中的 content_hash 比较
  3. 首次运行时为缺少 content_hash 的区块写入哈希（--stamp 模式）
  4. 报告不一致（篡改/合法修改）的区块

用法:
  python scripts/verify_block_integrity.py                # 验证模式（只读）
  python scripts/verify_block_integrity.py --stamp        # 盖章模式：为缺少 hash 的区块写入 content_hash
  python scripts/verify_block_integrity.py --resign 042   # 重签名：合法修改后更新 content_hash
  python scripts/verify_block_integrity.py --json         # JSON 输出（供 ceremony_scan 消费）

谱系位置: 345号——区块拓扑内容完整性验证机制
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path


def compute_file_hash(file_path: Path) -> str:
    """Compute SHA256 of a file's content."""
    return hashlib.sha256(file_path.read_bytes()).hexdigest()


def resolve_source_file(block: dict, genealogy_dir: Path) -> Path | None:
    """Resolve a block to its genealogy source file.

    Handles both formats:
    - Legacy: content.source_file = "settled/NNN-title.md"
    - New: genealogy_id = "NNN" (scan settled/ for matching file)
    """
    # Legacy format: content.source_file
    content = block.get("content", {})
    if isinstance(content, dict):
        sf = content.get("source_file", "")
        if sf:
            # Normalize backslashes to forward slashes
            sf_norm = sf.replace("\\", "/")
            candidate = genealogy_dir / sf_norm
            if candidate.is_file():
                return candidate
            return None

    # New format: genealogy_id
    gid = block.get("genealogy_id")
    if gid:
        settled_dir = genealogy_dir / "settled"
        if settled_dir.is_dir():
            # Try both raw gid and zero-padded variants as prefix
            candidates = {f"{gid}-"}
            try:
                num = int(gid)
                candidates.add(f"{num:03d}-")
                candidates.add(f"{num}-")
            except (ValueError, TypeError):
                pass
            for f in settled_dir.iterdir():
                if f.suffix == ".md" and any(
                    f.name.startswith(p) for p in candidates
                ):
                    return f
    return None


def scan_blocks(blocks_dir: Path) -> list[dict]:
    """Read all block JSON files."""
    blocks = []
    if not blocks_dir.is_dir():
        return blocks
    for f in sorted(blocks_dir.iterdir()):
        if f.suffix == ".json":
            blocks.append(json.loads(f.read_text(encoding="utf-8")))
    return blocks


def get_block_id(block: dict) -> str:
    """Extract block ID, handling both formats."""
    return block.get("id", block.get("block_id", ""))


def get_genealogy_number(block: dict) -> str:
    """Extract genealogy number from a block for display."""
    gid = block.get("genealogy_id", "")
    if gid:
        return str(gid)
    content = block.get("content", {})
    if isinstance(content, dict):
        cid = content.get("id", "")
        if cid:
            return str(cid)
    return ""


def _is_content_addressed(block: dict) -> bool:
    """Check if a block uses content-addressed format (has content.full_text)."""
    content = block.get("content", {})
    return isinstance(content, dict) and "full_text" in content


def _verify_content_addressed_block(
    block: dict, block_id: str, gen_num: str, source_file, root: Path
) -> tuple[str, dict | None]:
    """Verify a content-addressed block.

    Returns (status, detail_or_none) where status is one of:
    "verified", "mismatched", "working_copy_drift"
    """
    full_text = block["content"]["full_text"]
    expected_id = hashlib.sha256(full_text.encode("utf-8")).hexdigest()

    if block_id != expected_id:
        return "mismatched", {
            "block_id": block_id,
            "genealogy": gen_num,
            "file": str(source_file.relative_to(root)) if source_file else "",
            "stored_hash": block_id,
            "computed_hash": expected_id,
            "reason": "block_id != SHA256(full_text)",
        }

    # If there's a working copy, check drift
    if source_file is not None and source_file.is_file():
        file_hash = compute_file_hash(source_file)
        text_hash = hashlib.sha256(full_text.encode("utf-8")).hexdigest()
        if file_hash != text_hash:
            return "working_copy_drift", {
                "block_id": block_id,
                "genealogy": gen_num,
                "file": str(source_file.relative_to(root)),
                "block_text_hash": text_hash,
                "file_hash": file_hash,
                "reason": "working copy differs from block full_text",
            }

    return "verified", None


def verify_all(
    root: Path,
) -> dict:
    """Verify content integrity of all blocks with source files.

    Handles both legacy (content_hash) and content-addressed (full_text) blocks.

    Returns a result dict:
    {
        "verified": int,       # blocks with matching hash
        "mismatched": [...],   # blocks where hash != file content
        "missing_hash": [...], # blocks without content_hash (stampable) — legacy only
        "missing_file": [...], # blocks pointing to nonexistent files
        "working_copy_drift": [...], # content-addressed blocks where .md != full_text
        "skipped": int,        # blocks without source files
        "total": int,
    }
    """
    blocks_dir = root / ".chanlun" / "block-topology" / "blocks"
    genealogy_dir = root / ".chanlun" / "genealogy"

    result = {
        "verified": 0,
        "mismatched": [],
        "missing_hash": [],
        "missing_file": [],
        "working_copy_drift": [],
        "skipped": 0,
        "total": 0,
    }

    blocks = scan_blocks(blocks_dir)
    result["total"] = len(blocks)

    for block in blocks:
        source_file = resolve_source_file(block, genealogy_dir)
        block_id = get_block_id(block)
        gen_num = get_genealogy_number(block)

        # Content-addressed block: verify full_text integrity
        if _is_content_addressed(block):
            status, detail = _verify_content_addressed_block(
                block, block_id, gen_num, source_file, root
            )
            if status == "verified":
                result["verified"] += 1
            elif status == "mismatched":
                result["mismatched"].append(detail)
            elif status == "working_copy_drift":
                result["working_copy_drift"].append(detail)
            continue

        # Legacy block verification below

        # No source file reference — skip (rewrite/tension/consensus blocks)
        if source_file is None:
            # Check if block should have a source file but it's missing
            content = block.get("content", {})
            has_sf_ref = (
                (isinstance(content, dict) and content.get("source_file"))
                or block.get("genealogy_id")
            )
            if has_sf_ref:
                result["missing_file"].append({
                    "block_id": block_id,
                    "genealogy": gen_num,
                    "reason": "source file not found on disk",
                })
            else:
                result["skipped"] += 1
            continue

        # Compute current file hash
        file_hash = compute_file_hash(source_file)
        stored_hash = block.get("content_hash", "")

        if not stored_hash:
            result["missing_hash"].append({
                "block_id": block_id,
                "genealogy": gen_num,
                "file": str(source_file.relative_to(root)),
                "computed_hash": file_hash,
            })
            continue

        if stored_hash == file_hash:
            result["verified"] += 1
        else:
            result["mismatched"].append({
                "block_id": block_id,
                "genealogy": gen_num,
                "file": str(source_file.relative_to(root)),
                "stored_hash": stored_hash,
                "computed_hash": file_hash,
            })

    return result


def stamp_blocks(root: Path) -> dict:
    """Write content_hash to all blocks that have source files but no hash.

    Returns: {"stamped": int, "errors": [...]}
    """
    blocks_dir = root / ".chanlun" / "block-topology" / "blocks"
    genealogy_dir = root / ".chanlun" / "genealogy"

    stamped = 0
    errors = []

    blocks = scan_blocks(blocks_dir)
    for block in blocks:
        # Skip if already has content_hash
        if block.get("content_hash"):
            continue

        source_file = resolve_source_file(block, genealogy_dir)
        if source_file is None:
            continue

        file_hash = compute_file_hash(source_file)
        block_id = get_block_id(block)

        # Write content_hash into the block
        block["content_hash"] = file_hash
        block_path = blocks_dir / f"{block_id}.json"
        if not block_path.is_file():
            # New format blocks may not have 'id' as filename
            # Try to find by iterating
            found = False
            for f in blocks_dir.iterdir():
                if f.suffix == ".json":
                    d = json.loads(f.read_text(encoding="utf-8"))
                    if get_block_id(d) == block_id or d == block:
                        f.write_text(
                            json.dumps(block, ensure_ascii=False, indent=2),
                            encoding="utf-8",
                        )
                        stamped += 1
                        found = True
                        break
            if not found:
                errors.append(f"Cannot find block file for {block_id}")
        else:
            block_path.write_text(
                json.dumps(block, ensure_ascii=False, indent=2),
                encoding="utf-8",
            )
            stamped += 1

    return {"stamped": stamped, "errors": errors}


def resign_block(root: Path, genealogy_num: str) -> dict:
    """Re-sign a specific block after legitimate modification.

    Returns: {"success": bool, "block_id": str, "new_hash": str, "error": str}
    """
    blocks_dir = root / ".chanlun" / "block-topology" / "blocks"
    genealogy_dir = root / ".chanlun" / "genealogy"

    blocks = scan_blocks(blocks_dir)
    for block in blocks:
        gen_num = get_genealogy_number(block)
        if gen_num != genealogy_num:
            continue

        source_file = resolve_source_file(block, genealogy_dir)
        if source_file is None:
            return {
                "success": False,
                "block_id": get_block_id(block),
                "new_hash": "",
                "error": f"Source file not found for genealogy {genealogy_num}",
            }

        new_hash = compute_file_hash(source_file)
        old_hash = block.get("content_hash", "")
        block["content_hash"] = new_hash
        block_id = get_block_id(block)

        block_path = blocks_dir / f"{block_id}.json"
        if block_path.is_file():
            block_path.write_text(
                json.dumps(block, ensure_ascii=False, indent=2),
                encoding="utf-8",
            )
        else:
            # Search for block file
            for f in blocks_dir.iterdir():
                if f.suffix == ".json":
                    d = json.loads(f.read_text(encoding="utf-8"))
                    if get_block_id(d) == block_id:
                        f.write_text(
                            json.dumps(block, ensure_ascii=False, indent=2),
                            encoding="utf-8",
                        )
                        break

        return {
            "success": True,
            "block_id": block_id,
            "old_hash": old_hash,
            "new_hash": new_hash,
            "error": "",
        }

    return {
        "success": False,
        "block_id": "",
        "new_hash": "",
        "error": f"No block found for genealogy {genealogy_num}",
    }


def format_report(result: dict) -> str:
    """Format verification result as human-readable report."""
    lines = ["=== 区块拓扑内容完整性验证 ===", ""]
    lines.append(f"总区块数: {result['total']}")
    lines.append(f"已验证 (hash 匹配): {result['verified']}")
    lines.append(f"跳过 (无源文件引用): {result['skipped']}")
    lines.append(f"缺少 content_hash: {len(result['missing_hash'])}")
    lines.append(f"hash 不匹配: {len(result['mismatched'])}")
    lines.append(f"工作副本漂移: {len(result.get('working_copy_drift', []))}")
    lines.append(f"源文件缺失: {len(result['missing_file'])}")
    lines.append("")

    if result["mismatched"]:
        lines.append("--- 不匹配 (可能被篡改或合法修改) ---")
        for m in result["mismatched"]:
            lines.append(
                f"  [{m['genealogy']}] {m['file']}"
                f"  stored={m['stored_hash'][:16]}..."
                f"  actual={m['computed_hash'][:16]}..."
            )
        lines.append("")

    if result["missing_file"]:
        lines.append("--- 源文件缺失 ---")
        for m in result["missing_file"]:
            lines.append(f"  [{m['genealogy']}] {m['reason']}")
        lines.append("")

    drift = result.get("working_copy_drift", [])
    if drift:
        lines.append(f"--- 工作副本漂移 ({len(drift)} 个) ---")
        for d in drift:
            lines.append(
                f"  [{d['genealogy']}] {d['file']}"
                f"  reason={d['reason']}"
            )
        lines.append("")

    if result["missing_hash"]:
        lines.append(
            f"--- 缺少 content_hash ({len(result['missing_hash'])} 个) ---"
        )
        lines.append("  运行 --stamp 写入初始哈希")
        lines.append("")

    # Summary verdict
    if result["mismatched"]:
        lines.append("结论: FAIL — 存在 hash 不匹配的区块")
    elif result.get("working_copy_drift"):
        lines.append("结论: WARN — 工作副本与区块内容不一致（可能需要 rebuild）")
    elif result["missing_hash"]:
        lines.append("结论: WARN — 部分区块缺少 content_hash，需要 --stamp")
    else:
        lines.append("结论: PASS — 所有有源文件的区块完整性验证通过")

    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(
        description="区块拓扑内容完整性验证"
    )
    parser.add_argument(
        "--stamp",
        action="store_true",
        help="为缺少 content_hash 的区块写入哈希",
    )
    parser.add_argument(
        "--resign",
        metavar="NUM",
        help="重签名指定谱系编号的区块（合法修改后）",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        dest="json_output",
        help="JSON 格式输出（供 ceremony_scan 消费）",
    )
    parser.add_argument(
        "--root",
        default=".",
        help="项目根目录（默认当前目录）",
    )

    args = parser.parse_args()
    root = Path(args.root).resolve()

    if args.resign:
        result = resign_block(root, args.resign)
        if args.json_output:
            print(json.dumps(result, ensure_ascii=False, indent=2))
        elif result["success"]:
            print(
                f"重签名成功: 谱系 {args.resign}"
                f"  old={result['old_hash'][:16]}..."
                f"  new={result['new_hash'][:16]}..."
            )
        else:
            print(f"重签名失败: {result['error']}", file=sys.stderr)
            sys.exit(1)
        return

    if args.stamp:
        stamp_result = stamp_blocks(root)
        if args.json_output:
            print(json.dumps(stamp_result, ensure_ascii=False, indent=2))
        else:
            print(f"盖章完成: {stamp_result['stamped']} 个区块已写入 content_hash")
            if stamp_result["errors"]:
                for e in stamp_result["errors"]:
                    print(f"  错误: {e}", file=sys.stderr)
        return

    # Default: verify mode
    result = verify_all(root)

    if args.json_output:
        print(json.dumps(result, ensure_ascii=False, indent=2))
    else:
        print(format_report(result))

    # Exit code: 0=pass, 1=mismatch, 2=warn(missing hash)
    if result["mismatched"]:
        sys.exit(1)
    elif result["missing_hash"]:
        sys.exit(2)


if __name__ == "__main__":
    main()
