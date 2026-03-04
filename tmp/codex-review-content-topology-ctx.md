# Codex Review: Content-Level Topology Upgrade

## Review Mode
Code review — logic correctness, boundary safety, schema consistency, idiomatic Python.

## Subject
Content-level concept extraction + topology enrichment for block topology system.
Four files to review. 52 tests passing.

---

## File 1: scripts/block_topology.py (relevant additions only)

### New relation types added to RELATION_TYPES:
```python
RELATION_TYPES = frozenset({
    "depends_on", "negates", "related", "tensions_with",
    "supersedes", "residue_of", "reopens",
    "freezes", "splits", "severs",
    "records",
    "negated_by",
    "defines",     # NEW: block defines a concept (content-level)
    "modifies",    # NEW: block modifies a concept in another block (content-level)
    "references",  # NEW: body text references another block (content-level)
})
```

### New function: make_concept_id
```python
def make_concept_id(term: str) -> str:
    """Generate a deterministic SHA256 id for a concept term.

    Uses the prefix "concept:" to namespace concept ids and avoid
    collisions with block ids.
    """
    return hashlib.sha256(f"concept:{term}".encode("utf-8")).hexdigest()
```

### Order semantics (existing):
- order=1 → triggers rewrite block (structural change)
- order=2 → record only (no rewrite trigger)

### make_relation signature (existing):
```python
def make_relation(from_id: str, to_id: str, relation: str, order: int,
                  created_by: str, **extra) -> dict:
    # validates: created_by must be SHA256
    # validates: relation in RELATION_TYPES
    # validates: order in (1, 2)
    rec = {"from": from_id, "to": to_id, "relation": relation, ...}
    rec.update(extra)  # extra fields like concept_term, concept_definition
    return rec
```

Note: make_relation validates `created_by` as SHA256 but does NOT validate
`from_id` and `to_id` as SHA256. This is relevant because make_concept_id()
returns a SHA256, but it's a virtual id (no corresponding block file on disk).

---

## File 2: scripts/concept_extractor.py (full)

```python
"""concept_extractor.py — 从谱系 Markdown 正文提取结构化概念信息

从谱系正文中提取：
- section 结构（## N. 标题）
- 概念定义（**术语**：定义）
- 谱系引用（NNN号）
- 新增概念（### 新增概念 下的编号列表）
- 修正声明（### 对现有概念的修正 下的列表）
- 结论摘要（## 结论/核心命题 首段）

不引入 NLP 依赖——谱系正文结构高度规范，正则+模板匹配更可靠。
"""

from __future__ import annotations
import re
from dataclasses import dataclass, field


# --- Data Structures ---

@dataclass(frozen=True)
class Section:
    level: int          # heading level (2=##, 3=###, 4=####)
    number: str         # section number like "1", "3.2", "" if none
    title: str
    line_start: int

@dataclass(frozen=True)
class ConceptDefinition:
    term: str
    definition: str
    line: int

@dataclass(frozen=True)
class NewConcept:
    term: str
    qualifier: str
    definition: str

@dataclass(frozen=True)
class Modification:
    target_id: str      # genealogy id being modified, e.g. "254"
    target_desc: str
    modification: str

@dataclass(frozen=True)
class ContentAnalysis:
    genealogy_id: str
    sections: tuple[Section, ...]
    concepts: tuple[ConceptDefinition, ...]
    references: tuple[str, ...]
    new_concepts: tuple[NewConcept, ...]
    modifications: tuple[Modification, ...]
    conclusion_summary: str


# --- Regex Patterns ---

_SECTION_RE = re.compile(
    r'^(#{2,4})\s+(?:(\d+(?:\.\d+)*)\.)?\s*(.+)$', re.MULTILINE
)

_CONCEPT_DEF_RE = re.compile(
    r'\*\*([^*]+)\*\*\s*[：:—]\s*(.+)'
)

_GENEALOGY_REF_RE = re.compile(
    r'(\d{3}[a-z]?)号'
)

_NEW_CONCEPT_RE = re.compile(
    r'^\d+\.\s+\*?\*?(.+?)\*?\*?\s*(?:[（(]([^)）]*)[)）])?\s*(?:——|—|-)\s*(.+)$'
)

_MODIFICATION_RE = re.compile(
    r'^-\s+(\d{3}[a-z]?)\s*号\s*(.+?)(?:[：:])\s*(.+)$'
)

_MODIFICATION_ALT_RE = re.compile(
    r'^-\s+[「""]?(.+?)[」""]?\s*(?:降级为|提升为|修正为|变更为)\s*(.+?)(?:[：:])\s*(.+)$'
)

_CONCLUSION_RE = re.compile(
    r'^#{2,3}\s+(?:\d+\.)?\s*(?:结论|核心命题|总判定|收敛结论)',
    re.MULTILINE
)


# --- Extraction Functions ---

def _extract_sections(body: str) -> tuple[Section, ...]:
    sections = []
    for match in _SECTION_RE.finditer(body):
        level = len(match.group(1))
        number = match.group(2) or ""
        title = match.group(3).strip()
        line_start = body[:match.start()].count('\n') + 1
        sections.append(Section(level=level, number=number, title=title, line_start=line_start))
    return tuple(sections)


def _extract_concepts(body: str) -> tuple[ConceptDefinition, ...]:
    concepts = []
    seen_terms = set()
    for i, line in enumerate(body.split('\n'), 1):
        match = _CONCEPT_DEF_RE.search(line)
        if match:
            term = match.group(1).strip()
            definition = match.group(2).strip()
            # Skip frontmatter-style metadata lines
            if term.lower() in ('状态', '创建时间', '类型', '域', '溯源',
                                '来源', 'id', 'title', 'status', 'type',
                                'date', 'source', 'link', 'archive'):
                continue
            # Skip table header/separator lines
            if '---' in definition or '|' in definition[:5]:
                continue
            # Deduplicate by term
            if term not in seen_terms:
                seen_terms.add(term)
                concepts.append(ConceptDefinition(term=term, definition=definition, line=i))
    return tuple(concepts)


def _extract_references(body: str) -> tuple[str, ...]:
    refs = set()
    for match in _GENEALOGY_REF_RE.finditer(body):
        refs.add(match.group(1))
    return tuple(sorted(refs))


def _extract_section_content(body: str, heading_pattern: str) -> str:
    pattern = re.compile(
        r'^#{2,4}\s+(?:\d+(?:\.\d+)*\.)?\s*' + re.escape(heading_pattern),
        re.MULTILINE
    )
    match = pattern.search(body)
    if not match:
        return ""
    start = match.end()
    heading_level = body[match.start():match.end()].count('#', 0,
                        body[match.start():match.end()].index(' '))
    next_heading = re.search(
        r'^#{2,' + str(heading_level) + r'}\s',
        body[start:],
        re.MULTILINE
    )
    if next_heading:
        return body[start:start + next_heading.start()].strip()
    return body[start:].strip()


def _find_section_by_keywords(body: str, keywords: list[str]) -> str:
    for kw in keywords:
        content = _extract_section_content(body, kw)
        if content:
            return content
    return ""


def _extract_new_concepts(body: str) -> tuple[NewConcept, ...]:
    section_text = _find_section_by_keywords(body, ["新增概念"])
    if not section_text:
        return ()
    concepts = []
    for line in section_text.split('\n'):
        line = line.strip()
        if not line:
            continue
        match = _NEW_CONCEPT_RE.match(line)
        if match:
            term = match.group(1).strip().strip('*')
            qualifier = (match.group(2) or "").strip()
            definition = match.group(3).strip()
            concepts.append(NewConcept(term=term, qualifier=qualifier, definition=definition))
    return tuple(concepts)


def _extract_modifications(body: str) -> tuple[Modification, ...]:
    section_text = _find_section_by_keywords(body, [
        "对现有概念的修正",
        "对现有概念的定位修正",
    ])
    if not section_text:
        return ()
    modifications = []
    for line in section_text.split('\n'):
        line = line.strip()
        if not line:
            continue
        match = _MODIFICATION_RE.match(line)
        if match:
            modifications.append(Modification(
                target_id=match.group(1),
                target_desc=match.group(2).strip(),
                modification=match.group(3).strip(),
            ))
            continue
        match = _MODIFICATION_ALT_RE.match(line)
        if match:
            modifications.append(Modification(
                target_id="",  # no explicit target id
                target_desc=match.group(1).strip(),
                modification=f"{match.group(2).strip()}：{match.group(3).strip()}",
            ))
    return tuple(modifications)


def _extract_conclusion_summary(body: str) -> str:
    match = _CONCLUSION_RE.search(body)
    if not match:
        return ""
    start = match.end()
    rest = body[start:].strip()
    para_end = rest.find('\n\n')
    if para_end == -1:
        return rest[:500]
    return rest[:para_end].strip()[:500]


def _strip_frontmatter(text: str) -> str:
    match = re.match(r'^---\s*\n.*?\n---\s*\n', text, re.DOTALL)
    if match:
        return text[match.end():]
    return text


def analyze_content(genealogy_id: str, text: str) -> ContentAnalysis:
    body = _strip_frontmatter(text)
    return ContentAnalysis(
        genealogy_id=genealogy_id,
        sections=_extract_sections(body),
        concepts=_extract_concepts(body),
        references=_extract_references(body),
        new_concepts=_extract_new_concepts(body),
        modifications=_extract_modifications(body),
        conclusion_summary=_extract_conclusion_summary(body),
    )
```

---

## File 3: scripts/content_enrichment_migration.py (full)

```python
"""content_enrichment_migration.py — 全量内容级迁移

对已有 settled/*.md 创建 content enrichment rewrite 区块：
1. 读取完整文件 → analyze_content() 获取 ContentAnalysis
2. 通过 meta.json id_mapping 找到原 event 区块 SHA
3. 创建 rewrite 区块 + defines/modifies/references 关系
4. 更新 meta.json

幂等：compute_block_id 基于确定性内容 → 重复运行不产生重复区块。
"""

from __future__ import annotations
import argparse
import json
import sys
from dataclasses import asdict
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT))

from scripts.block_topology import (
    DEFAULT_BASE, append_relation, compute_block_id, make_block,
    make_concept_id, make_relation, read_meta, write_block, write_meta,
)
from scripts.concept_extractor import analyze_content


def enrich_single_file(
    md_file: Path,
    id_mapping: dict[str, str],
    base: Path,
    dry_run: bool = False,
) -> dict | None:
    text = md_file.read_text(encoding="utf-8")

    import re
    fname_match = re.match(r'^(\d{3}[a-z]?)-', md_file.name)
    if not fname_match:
        return None
    genealogy_id = fname_match.group(1)

    original_sha = id_mapping.get(genealogy_id)
    if original_sha is None:
        try:
            original_sha = id_mapping.get(str(int(genealogy_id)))
        except ValueError:
            pass
    if original_sha is None:
        return {"genealogy_id": genealogy_id, "skipped": "no_block_mapping"}

    ca = analyze_content(genealogy_id, text)

    if dry_run:
        stats = _analysis_stats(ca)
        stats["original_block"] = original_sha[:16] + "..."
        return stats

    ca_dict = _content_analysis_to_dict(ca)
    rewrite_content = {
        "action": "content_enrichment",
        "original_block": original_sha,
        "content_analysis": ca_dict,
    }
    rewrite_block = make_block(
        block_type="rewrite",
        source="migration",
        content=rewrite_content,
        refs=[original_sha],
    )
    write_block(rewrite_block, base)

    relations_written = 0

    # records relation: rewrite → original
    records_rel = make_relation(
        from_id=rewrite_block["id"],
        to_id=original_sha,
        relation="records",
        order=2,
        created_by=rewrite_block["id"],
    )
    append_relation(records_rel, base)
    relations_written += 1

    # defines relations: original → concept_id (for each new concept)
    for nc in ca.new_concepts:
        concept_id = make_concept_id(nc.term)
        defines_rel = make_relation(
            from_id=original_sha,
            to_id=concept_id,
            relation="defines",
            order=2,
            created_by=rewrite_block["id"],
            concept_term=nc.term,
            concept_definition=nc.definition,
        )
        append_relation(defines_rel, base)
        relations_written += 1

    # Also create defines for inline concepts
    for c in ca.concepts:
        concept_id = make_concept_id(c.term)
        defines_rel = make_relation(
            from_id=original_sha,
            to_id=concept_id,
            relation="defines",
            order=2,
            created_by=rewrite_block["id"],
            concept_term=c.term,
            concept_definition=c.definition,
        )
        append_relation(defines_rel, base)
        relations_written += 1

    # modifies relations: original → target_block
    for mod in ca.modifications:
        if not mod.target_id:
            continue
        target_sha = id_mapping.get(mod.target_id)
        if target_sha is None:
            try:
                target_sha = id_mapping.get(str(int(mod.target_id)))
            except ValueError:
                pass
        if target_sha is None:
            continue
        modifies_rel = make_relation(
            from_id=original_sha,
            to_id=target_sha,
            relation="modifies",
            order=1,       # <-- ORDER 1: triggers rewrite
            created_by=rewrite_block["id"],
            target_desc=mod.target_desc,
            modification=mod.modification,
        )
        append_relation(modifies_rel, base)
        relations_written += 1

    # references relations: original → referenced_block
    for ref_id in ca.references:
        if ref_id == genealogy_id:
            continue
        ref_sha = id_mapping.get(ref_id)
        if ref_sha is None:
            try:
                ref_sha = id_mapping.get(str(int(ref_id)))
            except ValueError:
                pass
        if ref_sha is None:
            continue
        references_rel = make_relation(
            from_id=original_sha,
            to_id=ref_sha,
            relation="references",
            order=2,
            created_by=rewrite_block["id"],
        )
        append_relation(references_rel, base)
        relations_written += 1

    return {
        "genealogy_id": genealogy_id,
        "rewrite_block": rewrite_block["id"],
        "relations_written": relations_written,
        "concepts": len(ca.concepts) + len(ca.new_concepts),
        "modifications": len(ca.modifications),
        "references": len(ca.references),
    }


def run_enrichment(project_root=None, base=None, dry_run=False) -> dict:
    if project_root is None:
        project_root = PROJECT_ROOT
    if base is None:
        base = project_root / ".chanlun" / "block-topology"

    settled_dir = project_root / ".chanlun" / "genealogy" / "settled"
    meta = read_meta(base)
    if meta is None:
        return {"error": "meta.json not found"}

    id_mapping = meta.get("id_mapping", {})
    if not id_mapping:
        return {"error": "id_mapping empty"}

    results = []
    total_blocks = 0
    total_relations = 0
    total_concepts = 0

    for md_file in sorted(settled_dir.glob("*.md")):
        result = enrich_single_file(md_file, id_mapping, base, dry_run)
        if result is None:
            continue
        results.append(result)
        if not dry_run and "rewrite_block" in result:
            total_blocks += 1
            total_relations += result.get("relations_written", 0)
            total_concepts += result.get("concepts", 0)

    summary = {
        "mode": "dry_run" if dry_run else "write",
        "files_processed": len(results),
        "files_skipped": sum(1 for r in results if r.get("skipped")),
    }

    if not dry_run:
        summary["blocks_created"] = total_blocks
        summary["relations_written"] = total_relations
        summary["concepts_defined"] = total_concepts
        summary["file_results"] = results

        # meta.json update (replaces entire content_enrichment section)
        meta["content_enrichment"] = {
            "blocks_created": total_blocks,
            "relations_written": total_relations,
            "concepts_defined": total_concepts,
        }
        write_meta(meta, base)

    return summary
```

---

## File 4: scripts/concept_topology_check.py (full)

```python
"""concept_topology_check.py — 概念去重 + 矛盾检测 + 引用-依赖一致性检查

Three checks:
1. detect_duplicate_concepts() — same concept_id, multiple defines, inconsistent text
2. detect_concept_conflicts() — A modifies B, C references B but doesn't depend on A
3. detect_reference_dependency_mismatch() — references vs depends_on mismatch
"""

from collections import defaultdict

def _load_topology(base):
    relations = read_all_relations(base)
    meta = read_meta(base)
    id_mapping = meta.get("id_mapping", {}) if meta else {}
    reverse_mapping = {sha: gid for gid, sha in id_mapping.items()}
    return relations, reverse_mapping


def detect_duplicate_concepts(base=DEFAULT_BASE) -> list[dict]:
    relations, reverse_mapping = _load_topology(base)

    concept_defs = defaultdict(list)
    for rel in relations:
        if rel.get("relation") == "defines":
            concept_id = rel["to"]
            concept_defs[concept_id].append({
                "from_block": rel["from"],
                "from_genealogy": reverse_mapping.get(rel["from"], rel["from"][:16]),
                "concept_term": rel.get("concept_term", ""),
                "concept_definition": rel.get("concept_definition", ""),
            })

    modifies_pairs = set()
    for rel in relations:
        if rel.get("relation") == "modifies":
            modifies_pairs.add((rel["from"], rel["to"]))
            modifies_pairs.add((rel["to"], rel["from"]))

    duplicates = []
    for concept_id, defs in concept_defs.items():
        if len(defs) < 2:
            continue

        block_ids = [d["from_block"] for d in defs]
        all_connected_by_modifies = True
        for i in range(len(block_ids)):
            for j in range(i + 1, len(block_ids)):
                if (block_ids[i], block_ids[j]) not in modifies_pairs:
                    all_connected_by_modifies = False
                    break
            if not all_connected_by_modifies:
                break

        if all_connected_by_modifies:
            continue

        definitions = set(d["concept_definition"] for d in defs if d["concept_definition"])
        if len(definitions) <= 1:
            continue

        duplicates.append({
            "concept_id": concept_id,
            "concept_term": defs[0].get("concept_term", ""),  # BUG: always picks first
            "definitions": defs,
            "definition_count": len(definitions),
            "connected_by_modifies": all_connected_by_modifies,
        })

    return duplicates


def detect_concept_conflicts(base=DEFAULT_BASE) -> list[dict]:
    relations, reverse_mapping = _load_topology(base)

    modifications = [
        {"modifier_block": rel["from"], "modified_block": rel["to"],
         "target_desc": rel.get("target_desc", ""), "modification": rel.get("modification", "")}
        for rel in relations if rel.get("relation") == "modifies"
    ]

    if not modifications:
        return []

    references_index = defaultdict(set)
    for rel in relations:
        if rel.get("relation") == "references":
            references_index[rel["to"]].add(rel["from"])

    depends_on_index = defaultdict(set)
    for rel in relations:
        if rel.get("relation") == "depends_on":
            depends_on_index[rel["from"]].add(rel["to"])

    conflicts = []
    for mod in modifications:
        modified_block = mod["modified_block"]
        modifier_block = mod["modifier_block"]
        referencers = references_index.get(modified_block, set())

        for ref_block in referencers:
            if ref_block == modifier_block:
                continue
            deps = depends_on_index.get(ref_block, set())
            if modifier_block not in deps:
                conflicts.append({
                    "type": "stale_reference",
                    "referencing_block": ref_block,
                    "referencing_genealogy": reverse_mapping.get(ref_block, ref_block[:16]),
                    "modified_block": modified_block,
                    "modified_genealogy": reverse_mapping.get(modified_block, modified_block[:16]),
                    "modifier_block": modifier_block,
                    "modifier_genealogy": reverse_mapping.get(modifier_block, modifier_block[:16]),
                    "target_desc": mod["target_desc"],
                    "modification": mod["modification"],
                })

    return conflicts


def detect_reference_dependency_mismatch(base=DEFAULT_BASE) -> list[dict]:
    relations, reverse_mapping = _load_topology(base)

    references_by_block = defaultdict(set)
    depends_by_block = defaultdict(set)

    for rel in relations:
        if rel.get("relation") == "references":
            references_by_block[rel["from"]].add(rel["to"])
        elif rel.get("relation") == "depends_on":
            depends_by_block[rel["from"]].add(rel["to"])

    mismatches = []
    all_blocks = set(references_by_block.keys()) | set(depends_by_block.keys())

    for block_id in all_blocks:
        refs = references_by_block.get(block_id, set())
        deps = depends_by_block.get(block_id, set())

        missing = refs - deps
        for m in missing:
            mismatches.append({
                "type": "missing_dependency", "severity": "warn",
                "block": block_id, "target": m,
                "detail": "正文引用但不在 depends_on 中",
            })

        structural = deps - refs
        for s in structural:
            mismatches.append({
                "type": "structural_only", "severity": "info",
                "block": block_id, "target": s,
                "detail": "depends_on 中声明但正文未引用（可能是否定性依赖）",
            })

    return mismatches
```

---

## Schema Context

- Block: `{id: sha256, type: event|rewrite|..., source: ..., content: dict, refs: [sha256, ...]}`
- Relation: `{from: sha256_or_concept_id, to: sha256_or_concept_id, relation: str, order: 1|2, created_by: sha256, timestamp: ...}`
- `defines` relation: `from` = real block SHA, `to` = concept_id (virtual SHA from make_concept_id)
- `modifies` relation: `from` = block A SHA, `to` = block B SHA (real block)
- `references` relation: `from` = block SHA, `to` = referenced block SHA (real block)
- `records` relation: `from` = rewrite block SHA, `to` = original block SHA

## Key Questions to Review

1. **Identifier strategy**: `make_concept_id(term)` uses SHA256("concept:{term}").
   - Terms may have variants ("折叠" vs "折叠拓扑", "中枢" vs "中枢概念").
   - No normalization before hashing. Same term in different files → same concept_id.
   - Different spelling variant → different concept_id (silently creates separate concept nodes).
   - Is this identity-by-exact-term a sound strategy for 300+ documents?

2. **Schema consistency of `defines` relation**:
   - `to` field = concept_id (a virtual SHA, no corresponding .json block file).
   - `make_relation` validates `created_by` as SHA256 but not `from_id`/`to_id`.
   - Callers of `query_relations` or `read_block(concept_id)` would get None.
   - Is this "virtual node" pattern safe? Are there any code paths that assume `to` always resolves to a real block?

3. **Regex correctness in concept_extractor.py**:
   - `_CONCEPT_DEF_RE`: matches `**term**：def` and `**term**: def` and `**term**— def`.
     - False positives: markdown bold in table cells, code comments with `**`, list items with bold text.
     - The `—` (em-dash) separator — will this match `**笔**——定义` (two em-dashes)?
   - `_GENEALOGY_REF_RE`: `(\d{3}[a-z]?)号` — matches 3-digit numbers only.
     - Question 4: What happens with "号" appearing in normal prose? E.g. "第一号" would not match (only 1 digit), "100号大楼" would match. Is 3-digit constraint sufficient to prevent false positives?
   - `_NEW_CONCEPT_RE`: `r'^\d+\.\s+\*?\*?(.+?)\*?\*?\s*(?:[（(]...)...)` — optional double asterisks handling is fragile. `\*?\*?` matches 0, 1, or 2 asterisks independently, so it could match a single `*` (italic, not bold).
   - `_MODIFICATION_RE`: requires exactly 3-digit genealogy id. Will fail on "005a号" format? Check: `(\d{3}[a-z]?)` — yes, this handles letter suffixes. OK.
   - `_strip_frontmatter`: `r'^---\s*\n.*?\n---\s*\n'` with `re.DOTALL`. What if the file uses `\r\n` line endings (Windows)? The `\n` in pattern won't match `\r\n`.

4. **`modifies` order=1 semantic**:
   - The code in `write_block_with_relations` creates a rewrite block when order=1 relations exist.
   - But in `content_enrichment_migration.py`, `modifies` relations are written via `append_relation` directly, NOT via `write_block_with_relations`.
   - So `order=1` in the `modifies` relation record is stored in JSONL, but no rewrite block is created for it in this migration flow.
   - Is this intentional? The order field documents semantic intent but the migration bypasses the automated rewrite-creation mechanism.

5. **Performance — 301 files, potentially 2000+ relations**:
   - Each `append_relation` opens, writes, and closes `relations.jsonl` independently (one write = one file open).
   - For 2000+ relations, this means 2000+ file open/close operations.
   - `read_all_relations` reads the entire JSONL into memory each time concept_topology_check runs.
   - Is there a buffering/batch-write strategy? Should `run_enrichment` buffer all relations and write in one batch?

6. **Idempotency claim**:
   - `write_block` is idempotent (same content → same SHA → skips if file exists).
   - `append_relation` is NOT idempotent — running migration twice appends duplicate relation records.
   - The comment says "幂等：compute_block_id 基于确定性内容 → 重复运行不产生重复区块" — this is true for blocks but false for relations.
   - `read_all_relations` would return duplicate entries after second run.

7. **`detect_duplicate_concepts` term name display bug**:
   - `"concept_term": defs[0].get("concept_term", "")` — always picks first definition's term.
   - If the same concept_id has different term spellings (variant terms that hash to same SHA256 — extremely unlikely but...), this would show wrong term. Low severity.
   - More practically: if concept_term is empty for defs[0] but present for defs[1], the report shows empty term.

8. **`detect_concept_conflicts` temporal assumption**:
   - The logic assumes: "if C references B but doesn't depend_on A (where A modifies B), that's a conflict".
   - But it doesn't check temporal ordering. If C was written BEFORE A, this is expected — C cannot know about a future modification.
   - The function flags ALL such cases as conflicts, including historically valid references.
   - Is this a known limitation or an unintended false positive source?

Please provide your assessment of these issues, prioritized by severity.
