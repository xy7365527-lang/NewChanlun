"""concept_topo_fix.py — 概念拓扑异常修复脚本

修复 ceremony_scan 报告的异常：
1. 重复概念 → 同一 concept_id 被多个区块 defines 且定义不一致
   修复方式：基于 detect_duplicate_concepts 检测结果，
   为重复概念的 defines 关系生成 block-scoped concept_id
   （不依赖静态术语列表，动态检测后修复）
2. should_be_depends_on → 补写 depends_on 关系
3. 幻影否定 → severity=info，记录但不修复（frontmatter 声明否定但正文无对应段落，
   可能是隐式否定或 enrichment 提取遗漏，需人工判断）
4. 冲突 → stale_reference 类型，记录详情供人工审查
5. 遗漏依赖 → 按 triage 分类处理

修复历史：
- v137-swarm: 修复泛化术语（结论/定义/方向等 37 个术语）75→30 重复
- v137-swarm-r2: 修复剩余 30 个同名异义概念 → 0 重复
- should_be_depends_on: 已修复至 0

谱系引用：无（纯技术性修复操作）
"""

from __future__ import annotations

import json
import sys
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.block_topology import (
    DEFAULT_BASE,
    append_relation,
    make_concept_id,
    make_relation,
    read_all_relations,
    read_meta,
)


def _relation_key(rel: dict) -> tuple:
    return (rel["from"], rel["to"], rel["relation"], rel.get("order", 0))


def fix_duplicate_concepts(
    base: Path = DEFAULT_BASE,
    dry_run: bool = True,
) -> dict:
    """Fix duplicate concepts detected by detect_duplicate_concepts.

    Detects all concept_ids that are defined by multiple unconnected blocks
    with inconsistent definitions, then replaces them with block-scoped
    concept_ids: make_concept_id(f"{block_id}:{term}").

    This approach is dynamic (no static term list needed) — it fixes
    whatever duplicates exist at runtime.
    """
    from scripts.concept_topology_check import detect_duplicate_concepts as _detect

    dups = _detect(base)
    if not dups:
        print("No duplicate concepts detected")
        return {"updated": 0, "dry_run": dry_run}

    dup_concept_ids = {d["concept_id"] for d in dups}

    relations = read_all_relations(base)
    jsonl_path = base / "relations.jsonl"

    updated_count = 0
    new_relations = []

    for rel in relations:
        if (
            rel.get("relation") == "defines"
            and rel.get("to") in dup_concept_ids
        ):
            block_id = rel["from"]
            term = rel.get("concept_term", "")
            if not term:
                new_relations.append(rel)
                continue
            new_concept_id = make_concept_id(f"{block_id}:{term}")
            new_rel = dict(rel)
            new_rel["to"] = new_concept_id
            new_relations.append(new_rel)
            updated_count += 1
        else:
            new_relations.append(rel)

    if dry_run:
        print(f"[DRY RUN] Would update {updated_count} defines relations "
              f"across {len(dup_concept_ids)} duplicate concept_ids")
        return {"updated": updated_count, "dry_run": True}

    # Atomic rewrite: write to temp, then replace
    tmp_path = jsonl_path.with_suffix(".jsonl.tmp")
    with open(tmp_path, "w", encoding="utf-8") as f:
        for rel in new_relations:
            line = json.dumps(rel, ensure_ascii=False, separators=(",", ":"))
            f.write(line + "\n")
    tmp_path.replace(jsonl_path)

    # Update meta.json relation_count
    meta_path = base / "meta.json"
    meta = read_meta(base)
    if meta:
        meta["relation_count"] = len(new_relations)
        with open(meta_path, "w", encoding="utf-8") as f:
            json.dump(meta, f, ensure_ascii=False, indent=2)
            f.write("\n")

    print(f"Updated {updated_count} defines relations with block-scoped concept_ids")
    return {"updated": updated_count, "dry_run": False}


def fix_missing_depends_on(
    base: Path = DEFAULT_BASE,
    dry_run: bool = True,
) -> dict:
    """Add missing depends_on relations for should_be_depends_on items.

    These are cases where block A has a substantive relation to block B
    (modifies/refines/revises/negates/etc) AND references B in body,
    but does not have a depends_on relation to B.
    """
    from scripts.concept_topology_check import run_all_checks

    report = run_all_checks(base)
    warn_items = report["reference_dependency_mismatch"]["warn_items"]
    should_dep = [i for i in warn_items if i.get("triage") == "should_be_depends_on"]

    if not should_dep:
        print("No should_be_depends_on items found")
        return {"added": 0, "dry_run": dry_run}

    # Load existing relations for dedup
    existing = read_all_relations(base)
    existing_keys = set()
    for r in existing:
        if r.get("from") and r.get("to") and r.get("relation"):
            existing_keys.add(_relation_key(r))

    # Find a suitable created_by block (use the block itself as creator)
    meta = read_meta(base)
    id_mapping = meta.get("id_mapping", {}) if meta else {}

    added = 0
    for item in should_dep:
        block_id = item["block"]
        target_id = item["target"]

        # Check if already exists
        key = (block_id, target_id, "depends_on", 1)
        if key in existing_keys:
            continue

        rel = make_relation(
            from_id=block_id,
            to_id=target_id,
            relation="depends_on",
            order=1,
            created_by=block_id,
        )

        if dry_run:
            print(f"[DRY RUN] Would add depends_on: {item['block_genealogy']} → {item['target_genealogy']}")
        else:
            append_relation(rel, base)
            print(f"Added depends_on: {item['block_genealogy']} → {item['target_genealogy']}")

        existing_keys.add(key)
        added += 1

    return {"added": added, "dry_run": dry_run}


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Fix concept topology anomalies")
    parser.add_argument("--dry-run", action="store_true", default=False,
                        help="Show what would be done without making changes")
    parser.add_argument("--base", type=str, default=None,
                        help="Block topology base directory")
    args = parser.parse_args()

    base = Path(args.base) if args.base else DEFAULT_BASE

    print("=" * 60)
    print("  概念拓扑异常修复")
    print("=" * 60)

    print("\n--- Phase 1: 重复概念去重（动态检测） ---")
    dup_result = fix_duplicate_concepts(base, dry_run=args.dry_run)

    print("\n--- Phase 2: 补写 depends_on ---")
    dep_result = fix_missing_depends_on(base, dry_run=args.dry_run)

    print("\n--- Phase 3: 幻影否定（记录，不修复） ---")
    from scripts.concept_topology_check import run_all_checks
    report = run_all_checks(base)
    phantom_count = report["negation_consistency"]["info_count"]
    print(f"{phantom_count} 个幻影否定为 severity=info，frontmatter 声明否定但正文无对应段落。")
    print("可能原因：隐式否定 / enrichment 提取遗漏。需人工判断，不自动修复。")

    print("\n--- Phase 4: 冲突（记录，不修复） ---")
    conflict_count = report["concept_conflicts"]["count"]
    print(f"{conflict_count} 个 stale_reference 冲突：引用方未声明对修改方的依赖。")
    print("这些是语义层面的一致性问题，需要人工审查每个冲突的上下文。")

    print("\n--- Phase 5: 遗漏依赖分类 ---")
    warn_items = report["reference_dependency_mismatch"]["warn_items"]
    from collections import Counter
    triage_counts = Counter(i.get("triage", "?") for i in warn_items)
    print(f"{len(warn_items)} 个遗漏依赖按 triage 分类：")
    sbdo = triage_counts.get("should_be_depends_on", 0)
    sbr = triage_counts.get("should_be_references", 0)
    info = triage_counts.get("info_only", 0)
    print(f"  - {sbdo} should_be_depends_on: {'已在 Phase 2 修复' if sbdo > 0 else '无'}")
    print(f"  - {sbr} should_be_references: 结构性遗漏（enrichment 设计限制）")
    print(f"  - {info} info_only: 非迁移区块引用，无需修复")

    print("\n" + "=" * 60)
    print("  修复结果")
    print("=" * 60)
    print(f"  重复概念去重: {dup_result['updated']} 条 defines 关系已更新")
    print(f"  depends_on 补写: {dep_result['added']} 条关系已添加")
    if args.dry_run:
        print("  [DRY RUN 模式 — 未实际修改]")


if __name__ == "__main__":
    main()
