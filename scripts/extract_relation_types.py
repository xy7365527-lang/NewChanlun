#!/usr/bin/env python3
"""
从谱系 frontmatter 中提取 supersedes/reopens/modifies 关系，写入 relations.jsonl。

提取规则：
- negates: [X]         → from=current, to=X, relation=supersedes
- negated_by: [X]      → from=X, to=current, relation=supersedes
- replaces: X          → from=current, to=X, relation=supersedes
- reopens: [X]         → from=current, to=X, relation=reopens
- modifies: [X]        → from=current, to=X, relation=modifies

只追加新关系，不修改已有的 depends_on 等关系。
去重：不写入已存在的 (from, to, relation) 三元组。
"""

from __future__ import annotations

import json
import os
import re
from datetime import datetime, timezone
from pathlib import Path

GENEALOGY_DIR = Path(".chanlun/genealogy/settled")
TOPOLOGY_DIR = Path(".chanlun/block-topology")
META_PATH = TOPOLOGY_DIR / "meta.json"
RELATIONS_PATH = TOPOLOGY_DIR / "relations.jsonl"

# 谱系编号提取正则：匹配 "089" 或 "073a" 或 "005b" 等
_NUM_RE = re.compile(r"^(\d{3}[a-z]?)")


def load_id_mapping() -> dict[str, str]:
    """从 meta.json 加载 genealogy_number → block_id 映射。"""
    meta = json.loads(META_PATH.read_text(encoding="utf-8"))
    return meta["id_mapping"]


def load_existing_relations() -> set[tuple[str, str, str]]:
    """读取已有关系的 (from, to, relation) 三元组集合，用于去重。

    兼容旧格式：type→relation, source→from, target→to。
    """
    existing = set()
    if not RELATIONS_PATH.exists():
        return existing
    with open(RELATIONS_PATH, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            rec = json.loads(line)
            from_id = rec.get("from") or rec.get("source", "")
            to_id = rec.get("to") or rec.get("target", "")
            relation = rec.get("relation") or rec.get("type", "")
            if from_id and to_id and relation:
                existing.add((from_id, to_id, relation))
    return existing


def parse_frontmatter(filepath: Path) -> dict | None:
    """从 markdown 文件提取 YAML frontmatter。"""
    text = filepath.read_text(encoding="utf-8")
    m = re.match(r"^---\n(.*?)\n---", text, re.DOTALL)
    if not m:
        return None
    import yaml
    try:
        return yaml.safe_load(m.group(1))
    except Exception:
        return None


def _normalize_id(raw: str) -> str:
    """从各种格式中提取纯谱系编号。

    Examples:
        "086"               → "086"
        "073a"              → "073a"
        "073a号（depth_budget 基因废除）" → "073a"
        "218号（扩展：...）" → "218"
        "269号验证方法（...）" → "269"
        "323号theoretical_settlement.md（2026-03-03版）" → "323"
    """
    raw = str(raw).strip()
    # 先尝试匹配开头的数字+可选字母
    m = re.match(r"(\d{1,3}[a-z]?)", raw)
    if m:
        num_part = m.group(1)
        # 左补零到3位
        digits = re.match(r"(\d+)", num_part).group(1)
        suffix = num_part[len(digits):]
        return digits.zfill(3) + suffix
    return ""


def _to_list(val) -> list[str]:
    """将 frontmatter 值统一为字符串列表。"""
    if val is None or val == []:
        return []
    if isinstance(val, str):
        return [val]
    if isinstance(val, list):
        result = []
        for item in val:
            if isinstance(item, str):
                result.append(item)
            elif isinstance(item, dict):
                # 例如 137号的 negates 是 [{"target": "...", "content": "..."}]
                target = item.get("target", "")
                if target:
                    result.append(target)
            elif isinstance(item, (int, float)):
                result.append(str(int(item)))
        return result
    if isinstance(val, (int, float)):
        return [str(int(val))]
    return []


def extract_relations(genealogy_dir: Path = GENEALOGY_DIR) -> list[dict]:
    """从所有谱系文件中提取 supersedes/reopens/modifies 关系。

    Returns:
        list of dicts with keys: from_num, to_num, relation, source_file
    """
    relations = []

    for fn in sorted(os.listdir(genealogy_dir)):
        if not fn.endswith(".md"):
            continue

        filepath = genealogy_dir / fn
        fm = parse_frontmatter(filepath)
        if not fm:
            continue

        gid = str(fm.get("id", fn.split("-")[0])).strip("'\"")

        # negates → supersedes (current supersedes target)
        for target_raw in _to_list(fm.get("negates")):
            target_id = _normalize_id(target_raw)
            if target_id:
                relations.append({
                    "from_num": gid,
                    "to_num": target_id,
                    "relation": "supersedes",
                    "source_file": fn,
                })

        # negated_by → supersedes (negator supersedes current)
        for negator_raw in _to_list(fm.get("negated_by")):
            negator_id = _normalize_id(negator_raw)
            if negator_id:
                relations.append({
                    "from_num": negator_id,
                    "to_num": gid,
                    "relation": "supersedes",
                    "source_file": fn,
                })

        # replaces → supersedes
        for target_raw in _to_list(fm.get("replaces")):
            target_id = _normalize_id(target_raw)
            if target_id:
                relations.append({
                    "from_num": gid,
                    "to_num": target_id,
                    "relation": "supersedes",
                    "source_file": fn,
                })

        # reopens → reopens
        for target_raw in _to_list(fm.get("reopens")):
            target_id = _normalize_id(target_raw)
            if target_id:
                relations.append({
                    "from_num": gid,
                    "to_num": target_id,
                    "relation": "reopens",
                    "source_file": fn,
                })

        # modifies/amends → modifies
        for field in ("modifies", "amends"):
            for target_raw in _to_list(fm.get(field)):
                target_id = _normalize_id(target_raw)
                if target_id:
                    relations.append({
                        "from_num": gid,
                        "to_num": target_id,
                        "relation": "modifies",
                        "source_file": fn,
                    })

    return relations


def deduplicate_relations(
    extracted: list[dict],
    id_mapping: dict[str, str],
    existing: set[tuple[str, str, str]],
) -> list[dict]:
    """将提取的关系转为 block-id 格式并去重。

    Returns:
        list of new relation records ready for writing (with block IDs)
    """
    new_relations = []
    seen = set()

    for rel in extracted:
        from_bid = id_mapping.get(rel["from_num"])
        to_bid = id_mapping.get(rel["to_num"])

        if not from_bid or not to_bid:
            continue

        key = (from_bid, to_bid, rel["relation"])
        if key in existing or key in seen:
            continue
        seen.add(key)

        new_relations.append({
            "from_bid": from_bid,
            "to_bid": to_bid,
            "relation": rel["relation"],
            "from_num": rel["from_num"],
            "to_num": rel["to_num"],
            "source_file": rel["source_file"],
        })

    return new_relations


def write_relations(
    new_relations: list[dict],
    created_by: str,
    relations_path: Path = RELATIONS_PATH,
) -> int:
    """追加新关系到 relations.jsonl。

    Returns:
        写入的关系数量。
    """
    timestamp = datetime.now(timezone.utc).isoformat()
    count = 0

    with open(relations_path, "a", encoding="utf-8") as f:
        for rel in new_relations:
            rec = {
                "from": rel["from_bid"],
                "to": rel["to_bid"],
                "relation": rel["relation"],
                "order": 1,
                "created_by": created_by,
                "timestamp": timestamp,
            }
            f.write(json.dumps(rec, ensure_ascii=False, separators=(",", ":")) + "\n")
            count += 1

    return count


def update_meta_relation_count(meta_path: Path = META_PATH) -> int:
    """重新计算并更新 meta.json 的 relation_count。

    Returns:
        新的 relation_count。
    """
    count = 0
    with open(RELATIONS_PATH, "r", encoding="utf-8") as f:
        for line in f:
            if line.strip():
                count += 1

    meta = json.loads(meta_path.read_text(encoding="utf-8"))
    meta["relation_count"] = count
    meta_path.write_text(
        json.dumps(meta, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return count


def main():
    """主流程：提取 → 去重 → 写入 → 更新 meta。"""
    id_mapping = load_id_mapping()
    existing = load_existing_relations()

    extracted = extract_relations()
    print(f"从谱系中提取到 {len(extracted)} 条候选关系")

    # 按类型统计
    from collections import Counter
    type_counts = Counter(r["relation"] for r in extracted)
    for rtype, cnt in type_counts.most_common():
        print(f"  {rtype}: {cnt}")

    new_relations = deduplicate_relations(extracted, id_mapping, existing)
    print(f"去重后新关系: {len(new_relations)} 条")

    if not new_relations:
        print("无新关系需要写入。")
        return

    # 按类型统计新关系
    new_type_counts = Counter(r["relation"] for r in new_relations)
    for rtype, cnt in new_type_counts.most_common():
        print(f"  新 {rtype}: {cnt}")

    # 使用 genesis block 作为 created_by
    meta = json.loads(META_PATH.read_text(encoding="utf-8"))
    created_by = meta["genesis_block_id"]

    written = write_relations(new_relations, created_by)
    print(f"已写入 {written} 条新关系到 relations.jsonl")

    new_count = update_meta_relation_count()
    print(f"meta.json relation_count 已更新为 {new_count}")


if __name__ == "__main__":
    main()
