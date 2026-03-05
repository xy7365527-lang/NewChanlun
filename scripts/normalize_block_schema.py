"""
区块拓扑 Schema 统一迁移脚本

将 .chanlun/block-topology/blocks/ 中所有区块统一到规范 schema。

规范 schema（必需字段）：
  - id: str (SHA256)
  - type: str
  - timestamp: str (ISO 8601)
  - source: str
  - content: dict
  - refs: list[str]
  - git_ref: str

附加字段（从 content 或 top-level 提取，如果存在）：
  - genealogy_id: str | None
  - title: str | None
  - depends_on: list[str] | None

迁移原则：
  1. content_hash（文件名/id）不变
  2. 不丢失任何信息——非规范字段合并入 content
  3. 幂等——多次运行结果相同
"""
from __future__ import annotations

import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

BLOCKS_DIR = Path(".chanlun/block-topology/blocks")

CANONICAL_KEYS = {"id", "type", "timestamp", "source", "content", "refs", "git_ref"}

# 合法的 block types（与 block_topology.py 一致 + 谱系映射引入的扩展类型）
KNOWN_BLOCK_TYPES = {
    "event", "consensus", "residue", "tension", "rewrite", "annotation",
}

# 谱系映射写入的非 block_topology.py 类型——视为 event 的子类
GENEALOGY_TYPES = {
    "meta-rule", "概念定义", "研究线建立", "验证报告", "meta-rule（知识结晶）",
    "知识结晶", "概念扩展", "概念否定", "概念分离", "形式化", "工程规范",
    "审计报告", "元规则", "方法论", "语法规则", "结算", "扬弃",
}

DEFAULT_TIMESTAMP = "1970-01-01T00:00:00+00:00"
DEFAULT_SOURCE = "migration"


_FRONTMATTER_ID_RE = re.compile(r"^id:\s*['\"]?(\d+[a-z]?)['\"]?\s*$", re.MULTILINE)


def _extract_id_from_frontmatter(full_text: str) -> str | None:
    """从 YAML frontmatter 中提取 id 字段。"""
    m = _FRONTMATTER_ID_RE.search(full_text)
    return m.group(1) if m else None


def extract_genealogy_id(block: dict) -> str | None:
    """从区块中提取 genealogy_id（多路径查找）。"""
    # 直接 top-level
    gid = block.get("genealogy_id")
    if gid is not None:
        return str(gid)

    # content 内
    content = block.get("content", {})
    if isinstance(content, dict):
        gid = content.get("genealogy_id") or content.get("id")
        if isinstance(gid, str) and not len(gid) == 64:
            return gid
        if isinstance(gid, int):
            return str(gid)
        gid = content.get("number")
        if gid is not None:
            return str(gid)

        # frontmatter 内（full_text YAML header）
        full_text = content.get("full_text", "")
        if full_text and "---" in full_text:
            fm_id = _extract_id_from_frontmatter(full_text)
            if fm_id is not None:
                return fm_id

    # top-level number
    num = block.get("number")
    if num is not None:
        return str(num)

    return None


def extract_title(block: dict) -> str | None:
    """从区块中提取 title。"""
    title = block.get("title")
    if title:
        return title
    content = block.get("content", {})
    if isinstance(content, dict):
        return content.get("title")
    return None


def extract_depends_on(block: dict) -> list[str] | None:
    """从区块中提取 depends_on。"""
    deps = block.get("depends_on")
    if isinstance(deps, list):
        return [str(d) for d in deps]
    content = block.get("content", {})
    if isinstance(content, dict):
        deps = content.get("depends_on")
        if isinstance(deps, list):
            return [str(d) for d in deps]
    return None


def normalize_block(block: dict) -> dict:
    """将任意 schema 的区块统一到规范格式。

    不修改原始 dict——返回新 dict。
    """
    block_id = block.get("id", "")

    # --- type ---
    block_type = block.get("type", block.get("block_type", "event"))
    if block_type not in KNOWN_BLOCK_TYPES:
        # 保留原始 type 在 content 中，顶层归一化为 event
        original_type = block_type
        block_type = "event"
    else:
        original_type = None

    # --- timestamp ---
    timestamp = block.get("timestamp") or block.get("date") or block.get("created")
    if timestamp is None:
        timestamp = DEFAULT_TIMESTAMP
    elif not timestamp.endswith("+00:00") and "T" not in timestamp:
        # date-only string like "2026-03-04" -> ISO
        timestamp = f"{timestamp}T00:00:00+00:00"

    # --- source ---
    source = block.get("source", DEFAULT_SOURCE)
    # source 字段可能是 dict、文件路径或其他非标准值
    if not isinstance(source, str) or source not in {"cc", "gemini", "codex", "migration", "serena"}:
        source = DEFAULT_SOURCE

    # --- content ---
    content = block.get("content", {})
    if not isinstance(content, dict):
        content = {"raw": content}

    # 合并非规范 top-level 字段到 content（信息不丢失）
    extra_keys = set(block.keys()) - CANONICAL_KEYS - {
        "genealogy_id", "title", "depends_on",  # 提升为附加字段
        "block_type",  # 已处理
        "number", "date", "created",  # 已处理
        "status", "epistemological_level", "file", "source_file",  # 元数据
    }
    for key in sorted(extra_keys):
        if key not in content:
            content[f"_migrated_{key}"] = block[key]

    # 保存被归一化的 type 信息
    if original_type and "original_type" not in content:
        content["original_type"] = original_type

    # 保存 status/epistemological_level/file 到 content 如果存在
    for meta_key in ("status", "epistemological_level", "file", "source_file"):
        val = block.get(meta_key)
        if val is not None and meta_key not in content:
            content[meta_key] = val

    # --- refs ---
    refs = block.get("refs", [])
    if not isinstance(refs, list):
        refs = []

    # --- git_ref ---
    git_ref = block.get("git_ref", "")

    # --- 附加字段 ---
    genealogy_id = extract_genealogy_id(block)
    title = extract_title(block)
    depends_on = extract_depends_on(block)

    # --- 构建规范区块 ---
    normalized = {
        "id": block_id,
        "type": block_type,
        "timestamp": timestamp,
        "source": source,
        "content": content,
        "refs": refs,
        "git_ref": git_ref,
    }

    # 附加字段（如果存在）
    if genealogy_id is not None:
        normalized["genealogy_id"] = genealogy_id
    if title is not None:
        normalized["title"] = title
    if depends_on is not None:
        normalized["depends_on"] = depends_on

    return normalized


def is_already_canonical(block: dict) -> bool:
    """判断区块是否已经是规范格式。"""
    required = {"id", "type", "timestamp", "source", "content", "refs", "git_ref"}
    return required.issubset(block.keys())


def run(base: Path | None = None, dry_run: bool = False) -> dict:
    """执行迁移。返回统计信息。"""
    blocks_dir = (base or Path(".")) / BLOCKS_DIR if base else BLOCKS_DIR
    if not blocks_dir.exists():
        return {"error": f"blocks directory not found: {blocks_dir}"}

    stats = {
        "total": 0,
        "already_canonical": 0,
        "migrated": 0,
        "errors": [],
    }

    for f in sorted(blocks_dir.iterdir()):
        if f.suffix != ".json":
            continue
        stats["total"] += 1

        try:
            block = json.loads(f.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            stats["errors"].append({"file": f.name, "error": str(e)})
            continue

        if is_already_canonical(block):
            # 即使已有规范字段，也检查是否需要补充附加字段
            normalized = normalize_block(block)
            if json.dumps(block, sort_keys=True) == json.dumps(normalized, sort_keys=True):
                stats["already_canonical"] += 1
                continue

        normalized = normalize_block(block)

        # 验证 id 不变
        assert normalized["id"] == block.get("id", f.stem), \
            f"id mismatch: {normalized['id']} != {block.get('id', f.stem)}"

        if not dry_run:
            f.write_text(
                json.dumps(normalized, ensure_ascii=False, indent=2),
                encoding="utf-8",
            )

        stats["migrated"] += 1

    return stats


def main():
    dry_run = "--dry-run" in sys.argv
    base = None

    # 找到项目根目录
    cwd = Path.cwd()
    if (cwd / ".chanlun").exists():
        base = None  # 使用默认路径
    else:
        # 尝试向上查找
        for parent in cwd.parents:
            if (parent / ".chanlun").exists():
                base = parent
                break

    print(f"{'[DRY RUN] ' if dry_run else ''}Normalizing block schema...")
    stats = run(base=base, dry_run=dry_run)

    print(f"Total blocks: {stats['total']}")
    print(f"Already canonical: {stats['already_canonical']}")
    print(f"Migrated: {stats['migrated']}")
    if stats.get("errors"):
        print(f"Errors: {len(stats['errors'])}")
        for err in stats["errors"]:
            print(f"  {err['file']}: {err['error']}")

    if stats["migrated"] > 0 and not dry_run:
        print("\nMigration complete. Run with --dry-run to preview.")
    elif dry_run and stats["migrated"] > 0:
        print(f"\n{stats['migrated']} blocks would be migrated. Run without --dry-run to apply.")


if __name__ == "__main__":
    main()
