"""
谱系区块拓扑化——一次性迁移脚本

将 177 条已结算谱系 + dag.yaml 边升格为区块拓扑。

迁移逻辑：
1. 创建 genesis 区块（type=event, source=migration）
2. 扫描 settled/*.md → 每条谱系一个 event 区块（refs 一律为空）
3. 解析 dag.yaml edges → relations.jsonl（created_by = genesis block id）
4. 生成 meta.json（id_mapping + genesis_block_id）

编排者决断：
- migration 区块 refs 一律为空（关系全走边表）
- created_by 字段类型始终为 SHA256（genesis 区块 id）
- 幂等：相同内容产生相同 hash，重复运行不产生重复区块
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

import yaml

# Add project root to path
PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.block_topology import (
    DEFAULT_BASE,
    append_relation,
    compute_block_id,
    make_block,
    make_concept_id,
    make_relation,
    read_meta,
    write_block,
    write_meta,
)


def _try_content_analysis(genealogy_id: str, text: str) -> dict | None:
    """Attempt content analysis on a genealogy file.

    Returns content_analysis dict if successful, None otherwise.
    Does not fail the migration if analysis is unavailable.
    """
    try:
        from scripts.concept_extractor import analyze_content
        ca = analyze_content(genealogy_id, text)
        return {
            "sections": [
                {"level": s.level, "number": s.number, "title": s.title}
                for s in ca.sections
            ],
            "concepts": [
                {"term": c.term, "definition": c.definition}
                for c in ca.concepts
            ],
            "references": list(ca.references),
            "new_concepts": [
                {"term": nc.term, "qualifier": nc.qualifier,
                 "definition": nc.definition}
                for nc in ca.new_concepts
            ],
            "modifications": [
                {"target_id": m.target_id, "target_desc": m.target_desc,
                 "modification": m.modification}
                for m in ca.modifications
            ],
            "conclusion_summary": ca.conclusion_summary,
        }
    except Exception:
        return None


def parse_frontmatter(text: str) -> dict:
    """Extract metadata from a Markdown file.

    Supports two formats:
    1. YAML frontmatter (--- delimited)
    2. Markdown bold metadata (**key**: value) at top of file
    """
    # Try YAML frontmatter first
    match = re.match(r"^---\s*\n(.*?)\n---", text, re.DOTALL)
    if match:
        return yaml.safe_load(match.group(1)) or {}

    # Fallback: `---` 开头但无闭合标记的纯 YAML 文件（如 679号——全文即
    # frontmatter）。整体按 YAML 解析；失败则交给下方 Markdown bold 解析。
    if text.startswith("---\n"):
        try:
            fm = yaml.safe_load(text[4:])
            if isinstance(fm, dict):
                return fm
        except yaml.YAMLError:
            raise


    # Fallback: parse Markdown bold metadata (**key**: value)
    # Key field name mapping (Chinese → English)
    KEY_MAP = {"前置": "depends_on"}

    result = {}
    for line in text.split("\n"):
        line = line.strip()
        if not line:
            continue
        # Skip heading lines
        if line.startswith("#"):
            continue
        # Match **key**: value pattern (supports Chinese and ASCII keys)
        bold_match = re.match(r"\*\*([^*]+)\*\*:\s*(.*)", line)
        if bold_match:
            key = bold_match.group(1).strip()
            value = bold_match.group(2).strip()
            # Map Chinese keys to English
            key = KEY_MAP.get(key, key)
            # Normalize known list fields
            if key in ("depends_on", "related", "negates", "negated_by",
                        "tensions_with"):
                # Could be comma-separated or bracket list
                if value.startswith("["):
                    try:
                        value = yaml.safe_load(value)
                    except Exception:
                        value = [v.strip().strip("'\"")
                                 for v in value.strip("[]").split(",")
                                 if v.strip()]
                elif "," in value:
                    value = [v.strip() for v in value.split(",") if v.strip()]
                elif value:
                    value = [value]
                else:
                    value = []
            result[key] = value
        else:
            # Stop parsing metadata when we hit non-metadata content
            if result:  # Only stop if we've already found some metadata
                break

    return result


def migrate_settled_files(
    settled_dir: Path,
    base: Path = DEFAULT_BASE,
    with_content_analysis: bool = False,
    only_ids: set[str] | None = None,
) -> tuple[dict[str, str], list[dict]]:
    """Migrate settled/*.md → event blocks.

    Args:
        settled_dir: Path to settled/ directory
        base: Block topology base directory
        with_content_analysis: If True, include content_analysis in block
            content for new files. Existing blocks are unchanged (idempotent).
        only_ids: If given, only migrate files whose genealogy id is in this
            set (incremental migration). None = migrate all (full migration).

    Returns:
        id_mapping: {old_genealogy_id: block_id}
        blocks: list of block dicts

    Files whose frontmatter can't be parsed (malformed YAML, e.g. a schema
    violation like a `related:` list with inline prose) are skipped rather
    than crashing the whole migration, so one broken file doesn't block the
    rest. The gap stays observable: any target id absent from the returned
    id_mapping was skipped and can be fixed + re-migrated. This guards both
    full and incremental callers (system-boundary error handling).
    """
    id_mapping: dict[str, str] = {}
    blocks: list[dict] = []

    for md_file in sorted(settled_dir.glob("*.md")):
        text = md_file.read_text(encoding="utf-8")
        try:
            fm = parse_frontmatter(text)
        except yaml.YAMLError:
            continue  # malformed frontmatter — skip; caller diffs id_mapping
        if not fm.get("id"):
            continue

        old_id = str(fm["id"])

        # Incremental mode keys on the FILENAME id (what ceremony_scan looks
        # up), not the frontmatter id. These are identical for normal files;
        # they diverge only for merged-node records whose frontmatter id is a
        # range (e.g. filename 651-*.md but `id: "651-652"`). Keying on the
        # filename id keeps migrator ↔ scanner aligned so the scan clears.
        # Full mode (only_ids is None) keeps the frontmatter-id convention
        # used by every existing entry — untouched.
        if only_ids is not None:
            fname_m = re.match(r"^(\d+[a-z]?)-", md_file.name)
            map_key = fname_m.group(1) if fname_m else old_id
            if map_key not in only_ids:
                continue
            # 文件名 id 与 frontmatter id 必须同源（前缀一致）。防止日期开头的
            # slug-id 文件（如 2026-06-25-claim10-*.md，id: "claim10-…"，数字
            # 编号待 /ritual 分配）被误注册为伪编号 "2026"。合并节点（文件名
            # 651、id "651-652"）满足前缀条件，不受影响。
            if not old_id.startswith(map_key):
                continue
        else:
            map_key = old_id

        # Build content from frontmatter fields
        content = {}
        for key in ("id", "title", "status", "type", "date",
                     "negation_source", "negation_form",
                     "depends_on", "related", "negates", "negated_by",
                     "provenance", "topo_effect", "topo_executed_at",
                     "tensions_with"):
            if key in fm and fm[key] is not None:
                val = fm[key]
                # Normalize list fields: ensure string ids
                if isinstance(val, list):
                    val = [str(v) for v in val]
                else:
                    val = str(val) if not isinstance(val, str) else val
                content[key] = val

        # Add source file path
        content["source_file"] = str(md_file.relative_to(
            settled_dir.parent))

        # Content analysis for new files
        if with_content_analysis:
            ca = _try_content_analysis(old_id, text)
            if ca is not None:
                content["content_analysis"] = ca

        block = make_block(
            block_type="event",
            source="migration",
            content=content,
            refs=[],  # migration blocks: refs always empty
        )

        write_block(block, base)
        id_mapping[map_key] = block["id"]
        blocks.append(block)

    return id_mapping, blocks


def migrate_edges(
    dag_path: Path,
    id_mapping: dict[str, str],
    genesis_id: str,
    base: Path = DEFAULT_BASE,
) -> int:
    """Migrate dag.yaml edges → relations.jsonl.

    Returns count of relations written.
    """
    with open(dag_path, "r", encoding="utf-8") as f:
        dag = yaml.safe_load(f)

    edges = dag.get("edges", {})
    count = 0

    # depends_on: directed {from, to}
    for edge in edges.get("depends_on", []) or []:
        from_id = id_mapping.get(str(edge["from"]))
        to_id = id_mapping.get(str(edge["to"]))
        if from_id and to_id:
            rel = make_relation(from_id, to_id, "depends_on",
                                order=1, created_by=genesis_id)
            append_relation(rel, base)
            count += 1

    # negates: directed {from, to}, may have scope
    for edge in edges.get("negates", []) or []:
        from_id = id_mapping.get(str(edge["from"]))
        to_id = id_mapping.get(str(edge["to"]))
        if from_id and to_id:
            extra = {}
            if "scope" in edge:
                extra["scope"] = str(edge["scope"])
            rel = make_relation(from_id, to_id, "negates",
                                order=1, created_by=genesis_id,
                                **extra)
            append_relation(rel, base)
            count += 1

    # related: undirected {between: [a, b]}
    for edge in edges.get("related", []) or []:
        pair = edge.get("between", [])
        if len(pair) == 2:
            a_id = id_mapping.get(str(pair[0]))
            b_id = id_mapping.get(str(pair[1]))
            if a_id and b_id:
                rel = make_relation(a_id, b_id, "related",
                                    order=1, created_by=genesis_id)
                append_relation(rel, base)
                count += 1

    # tensions_with: undirected {between: [a, b]}, may have valid_until,
    # settled_by
    for edge in edges.get("tensions_with", []) or []:
        pair = edge.get("between", [])
        if len(pair) == 2:
            a_id = id_mapping.get(str(pair[0]))
            b_id = id_mapping.get(str(pair[1]))
            if a_id and b_id:
                extra = {}
                if "valid_until" in edge:
                    extra["valid_until"] = str(edge["valid_until"])
                if "settled_by" in edge:
                    extra["settled_by"] = str(edge["settled_by"])
                rel = make_relation(a_id, b_id, "tensions_with",
                                    order=1, created_by=genesis_id,
                                    **extra)
                append_relation(rel, base)
                count += 1

    # negated_by: directed {target, by}, may have scope
    for edge in edges.get("negated_by", []) or []:
        target_id = id_mapping.get(str(edge["target"]))
        by_id = id_mapping.get(str(edge["by"]))
        if target_id and by_id:
            extra = {}
            if "scope" in edge:
                extra["scope"] = str(edge["scope"])
            rel = make_relation(target_id, by_id, "negated_by",
                                order=1, created_by=genesis_id,
                                **extra)
            append_relation(rel, base)
            count += 1

    return count


def run_migration(
    project_root: Path | None = None,
    base: Path | None = None,
) -> dict:
    """Execute full migration.

    Returns meta dict with migration results.
    """
    if project_root is None:
        project_root = PROJECT_ROOT
    if base is None:
        base = project_root / ".chanlun" / "block-topology"

    settled_dir = project_root / ".chanlun" / "genealogy" / "settled"
    dag_path = project_root / ".chanlun" / "genealogy" / "dag.yaml"

    print(f"[migration] Project root: {project_root}")
    print(f"[migration] Target: {base}")

    # Step 1: Migrate settled files → event blocks
    id_mapping, blocks = migrate_settled_files(settled_dir, base)
    print(f"[migration] Migrated {len(blocks)} genealogy records → blocks")

    # Step 2: Create genesis block (after migration, so count is known)
    genesis_content = {
        "action": "genesis",
        "migrated_from": "dag.yaml + settled/*.md",
        "description": "One-time migration from genealogy to block topology",
        "count": len(blocks),
    }
    genesis = make_block("event", "migration", genesis_content, refs=[])
    write_block(genesis, base)
    print(f"[migration] Genesis block: {genesis['id'][:16]}...")

    # Step 3: Migrate edges → relations
    relation_count = migrate_edges(dag_path, id_mapping,
                                   genesis["id"], base)
    print(f"[migration] Migrated {relation_count} edges → relations")

    # Step 4: Write meta.json
    meta = {
        "version": "1.0.0",
        "genesis_block_id": genesis["id"],
        "block_count": len(blocks) + 1,  # +1 for genesis
        "relation_count": relation_count,
        "id_mapping": id_mapping,
        "migrated_at": genesis["timestamp"],
    }
    write_meta(meta, base)
    print(f"[migration] meta.json written")
    print(f"[migration] Done. {meta['block_count']} blocks, "
          f"{relation_count} relations")

    return meta


def _compute_unmapped_ids(
    settled_dir: Path, id_mapping: dict[str, str]
) -> list[str]:
    """Return settled genealogy ids (filename ids) absent from id_mapping.

    Uses the same filename-id extraction as ceremony_scan.get_topo_context
    so the incremental entry targets exactly what the scanner reports.
    """
    mapped = set(id_mapping.keys())
    settled_ids: set[str] = set()
    for md_file in settled_dir.glob("*.md"):
        m = re.match(r"^(\d+[a-z]?)-", md_file.name)
        if m:
            settled_ids.add(m.group(1))
    return sorted(
        settled_ids - mapped,
        key=lambda x: (int(re.match(r"\d+", x).group()), x),
    )


def run_incremental_migration(
    project_root: Path | None = None,
    base: Path | None = None,
    ids: list[str] | None = None,
    dry_run: bool = False,
) -> dict:
    """Incrementally migrate newly-settled genealogy into block-topology.

    Non-destructive counterpart to run_migration: creates event blocks only
    for genealogy ids missing from the existing id_mapping, APPENDS them to
    meta.json (preserving content_enrichment / genesis_block_id / every other
    field), migrates those ids' dag edges (deduped against existing
    relations), and advances last_mapped_genealogy.

    626号: block-topology 的增量补齐入口。run_migration 是破坏性全量重建，
    不能用于新增谱系；本入口只追加、不替换、不丢弃 content_enrichment。

    Args:
        project_root: Project root directory.
        base: Block topology base directory.
        ids: Explicit genealogy ids to migrate. None = auto-detect all
            settled ids missing from id_mapping.

    Returns:
        Summary dict with migrated ids, blocks/relations written, new
        last_mapped_genealogy.
    """
    if project_root is None:
        project_root = PROJECT_ROOT
    if base is None:
        base = project_root / ".chanlun" / "block-topology"

    settled_dir = project_root / ".chanlun" / "genealogy" / "settled"
    dag_path = project_root / ".chanlun" / "genealogy" / "dag.yaml"

    meta = read_meta(base)
    if meta is None:
        return {"error": "meta.json not found — run full migration first"}
    id_mapping = meta.get("id_mapping", {})
    genesis_id = meta.get("genesis_block_id")
    if not genesis_id:
        return {"error": "genesis_block_id missing in meta.json"}

    targets = ids if ids is not None else _compute_unmapped_ids(
        settled_dir, id_mapping)
    # 幂等：显式 --ids 重跑时过滤掉已映射的 id，避免 block_count 重复累加。
    targets = [t for t in targets if t not in id_mapping]
    if not targets:
        return {"migrated": [], "blocks_created": 0, "relations_written": 0,
                "last_mapped_genealogy": meta.get("last_mapped_genealogy"),
                "note": "nothing to migrate (id_mapping already current)"}

    if dry_run:
        return {
            "dry_run": True,
            "targets": targets,
            "edges_would_be_skipped_549": _relations_is_lfs_pointer(base),
            "current_last_mapped": meta.get("last_mapped_genealogy"),
            "current_mapped_count": len(id_mapping),
        }

    # Step 1: create event blocks for the target ids, append to id_mapping.
    new_mapping, blocks = migrate_settled_files(
        settled_dir, base, only_ids=set(targets))
    id_mapping.update(new_mapping)  # append, never replace

    # Step 2: migrate dag edges for the (now-mapped) targets, deduped so the
    # entry is idempotent across reruns. Edge migration writes relations.jsonl,
    # so it data-depends on 549 (relations.jsonl is a git-lfs pointer, not
    # checked out locally). If the pointer is unresolved, appending would
    # corrupt it — skip edges and report blocked (275 局部依赖: id_mapping 层
    # 不依赖 relations.jsonl, 照常补齐; edge 层 blocked on 549, 待 LFS 恢复后
    # 重跑 --incremental 幂等补齐).
    edges_blocked_on_549 = _relations_is_lfs_pointer(base)
    if edges_blocked_on_549:
        relations_written = 0
    else:
        relations_written = _migrate_edges_deduped(
            dag_path, id_mapping, genesis_id, base,
            restrict_to=set(new_mapping.values()))

    # Step 3: advance last_mapped_genealogy (numeric ids only, matching the
    # existing integer convention; lettered ids like 566a don't move it).
    numeric_new = [int(m.group()) for k in new_mapping
                   if (m := re.match(r"\d+$", k))]
    prev_last = meta.get("last_mapped_genealogy", 0) or 0
    meta["last_mapped_genealogy"] = max([prev_last] + numeric_new)

    # Step 4: persist (content_enrichment and all other fields preserved).
    meta["id_mapping"] = id_mapping
    meta["block_count"] = meta.get("block_count", 0) + len(blocks)
    meta["relation_count"] = meta.get("relation_count", 0) + relations_written
    write_meta(meta, base)

    still_unmapped = sorted(
        set(targets) - set(new_mapping),
        key=lambda x: (int(re.match(r"\d+", x).group()), x))

    result = {
        "migrated": sorted(new_mapping.keys(),
                           key=lambda x: (int(re.match(r"\d+", x).group()), x)),
        "blocks_created": len(blocks),
        "relations_written": relations_written,
        "last_mapped_genealogy": meta["last_mapped_genealogy"],
    }
    if still_unmapped:
        result["skipped_unparseable_frontmatter"] = still_unmapped
    if edges_blocked_on_549:
        result["edges_blocked_on_549"] = (
            "relations.jsonl is an unresolved git-lfs pointer; edge migration "
            "skipped to avoid corrupting it. Rerun --incremental after LFS "
            "checkout to append edges idempotently.")
    return result


def _relations_is_lfs_pointer(base: Path) -> bool:
    """True if relations.jsonl is an unresolved git-lfs pointer (549号).

    A checked-out relations.jsonl is JSONL (each line a JSON object). An
    unresolved LFS pointer starts with the spec version line. Appending to a
    pointer would corrupt it, so edge migration must skip when this is True.
    """
    path = base / "relations.jsonl"
    if not path.exists():
        return False
    with open(path, "r", encoding="utf-8") as f:
        first = f.readline().strip()
    return first.startswith("version https://git-lfs.github.com/spec/")


def _migrate_edges_deduped(
    dag_path: Path,
    id_mapping: dict[str, str],
    genesis_id: str,
    base: Path,
    restrict_to: set[str],
) -> int:
    """Like migrate_edges but (a) only writes edges touching a target id and
    (b) skips relations already present in relations.jsonl.

    restrict_to: set of NEW block ids — an edge is written only if either
    endpoint is one of these (avoids re-touching fully-migrated 001-565 edges).
    """
    from scripts.block_topology import read_all_relations

    existing_keys: set[tuple] = set()
    try:
        for r in read_all_relations(base):
            if r.get("from") and r.get("to") and r.get("relation"):
                existing_keys.add(
                    (r["from"], r["to"], r["relation"], r.get("order", 0)))
    except FileNotFoundError:
        pass

    with open(dag_path, "r", encoding="utf-8") as f:
        dag = yaml.safe_load(f)
    edges = dag.get("edges", {})
    count = 0

    def emit(from_id: str, to_id: str, relation: str, **extra) -> None:
        nonlocal count
        if from_id not in restrict_to and to_id not in restrict_to:
            return
        key = (from_id, to_id, relation, 1)
        if key in existing_keys:
            return
        existing_keys.add(key)
        append_relation(
            make_relation(from_id, to_id, relation, order=1,
                          created_by=genesis_id, **extra), base)
        count += 1

    for edge in edges.get("depends_on", []) or []:
        f_id, t_id = id_mapping.get(str(edge["from"])), id_mapping.get(str(edge["to"]))
        if f_id and t_id:
            emit(f_id, t_id, "depends_on")

    for edge in edges.get("negates", []) or []:
        f_id, t_id = id_mapping.get(str(edge["from"])), id_mapping.get(str(edge["to"]))
        if f_id and t_id:
            extra = {"scope": str(edge["scope"])} if "scope" in edge else {}
            emit(f_id, t_id, "negates", **extra)

    for edge in edges.get("related", []) or []:
        pair = edge.get("between", [])
        if len(pair) == 2:
            a_id, b_id = id_mapping.get(str(pair[0])), id_mapping.get(str(pair[1]))
            if a_id and b_id:
                emit(a_id, b_id, "related")

    for edge in edges.get("tensions_with", []) or []:
        pair = edge.get("between", [])
        if len(pair) == 2:
            a_id, b_id = id_mapping.get(str(pair[0])), id_mapping.get(str(pair[1]))
            if a_id and b_id:
                extra = {}
                if "valid_until" in edge:
                    extra["valid_until"] = str(edge["valid_until"])
                if "settled_by" in edge:
                    extra["settled_by"] = str(edge["settled_by"])
                emit(a_id, b_id, "tensions_with", **extra)

    for edge in edges.get("negated_by", []) or []:
        t_id, b_id = id_mapping.get(str(edge["target"])), id_mapping.get(str(edge["by"]))
        if t_id and b_id:
            extra = {"scope": str(edge["scope"])} if "scope" in edge else {}
            emit(t_id, b_id, "negated_by", **extra)

    return count


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Migrate genealogy → block topology")
    parser.add_argument(
        "--incremental", action="store_true",
        help="Incremental append (626号): migrate only settled ids missing "
             "from id_mapping; preserves content_enrichment. Default is "
             "destructive full rebuild.")
    parser.add_argument(
        "--ids", nargs="*", default=None,
        help="Explicit genealogy ids for --incremental (default: auto-detect)")
    parser.add_argument(
        "--dry-run", action="store_true",
        help="With --incremental: print the migration plan (target ids, "
             "549 edge-skip status) without writing anything")
    args = parser.parse_args()

    if args.incremental:
        summary = run_incremental_migration(ids=args.ids, dry_run=args.dry_run)
        print("Incremental migration complete.")
        for k, v in summary.items():
            if k == "migrated":
                print(f"  migrated ({len(v)}): {v}")
            else:
                print(f"  {k}: {v}")
    else:
        meta = run_migration()
        print(f"\nMigration complete.")
        print(f"  Blocks: {meta['block_count']}")
        print(f"  Relations: {meta['relation_count']}")
        print(f"  Genesis: {meta['genesis_block_id'][:16]}...")
