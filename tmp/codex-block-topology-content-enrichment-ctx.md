# Codex Review Context: Block Topology Content Enrichment Implementation

## Review Subject
Five Python modules implementing a content-level enrichment layer on top of a
block topology graph. Review mode: architectural code review.

---

## File 1: scripts/concept_extractor.py (329 lines)

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


def _normalize_newlines(text: str) -> str:
    """Normalize CRLF to LF for consistent regex matching."""
    return text.replace('\r\n', '\n').replace('\r', '\n')


# --- Data Structures ---

@dataclass(frozen=True)
class Section:
    """A section heading in the genealogy document."""
    level: int          # heading level (2=##, 3=###, 4=####)
    number: str         # section number like "1", "3.2", "" if none
    title: str          # section title text
    line_start: int     # line number where section starts


@dataclass(frozen=True)
class ConceptDefinition:
    """A concept defined inline via **term**: definition pattern."""
    term: str
    definition: str
    line: int


@dataclass(frozen=True)
class NewConcept:
    """A concept from the ### 新增概念 section."""
    term: str
    qualifier: str      # parenthetical qualifier, e.g. "编排者完成"
    definition: str


@dataclass(frozen=True)
class Modification:
    """A modification to an existing concept from ### 对现有概念的修正."""
    target_id: str      # genealogy id being modified, e.g. "254"
    target_desc: str    # what aspect is modified
    modification: str   # the modification content


@dataclass(frozen=True)
class ContentAnalysis:
    """Complete content analysis result for a genealogy file."""
    genealogy_id: str
    sections: tuple[Section, ...]
    concepts: tuple[ConceptDefinition, ...]
    references: tuple[str, ...]            # unique genealogy ids referenced
    new_concepts: tuple[NewConcept, ...]
    modifications: tuple[Modification, ...]
    conclusion_summary: str                # first paragraph of conclusion section


# --- Regex Patterns ---

# Section heading: ## N. Title or ### N.N Title or #### Title
_SECTION_RE = re.compile(
    r'^(#{2,4})\s+(?:(\d+(?:\.\d+)*)\.)?\s*(.+)$', re.MULTILINE
)

# Inline concept definition: **term**：definition or **term**: definition
# Also handles **term**— definition
# Anchored to line start with optional list prefix (-, *, N.) to reduce false positives
_CONCEPT_DEF_RE = re.compile(
    r'^\s*(?:[-*]|\d+\.)?\s*\*\*([^*]+)\*\*\s*[：:—]\s*(.+)', re.MULTILINE
)

# Genealogy reference: NNN号 (3-digit with optional letter suffix)
_GENEALOGY_REF_RE = re.compile(
    r'(\d{3}[a-z]?)号'
)

# New concept entry patterns (see comments in source)
_NEW_CONCEPT_WITH_SEP_RE = re.compile(
    r'^\d+\.\s+\*{0,2}(.+?)\*{0,2}'
    r'\s*(?:[（(]([^)）]*)[)）])?\s*'
    r'(?:——|—|[：:])\s*(.+)$'
)

_NEW_CONCEPT_BARE_RE = re.compile(
    r'^\d+\.\s+\*{0,2}(.+?)\*{0,2}'
    r'\s*(?:[（(]([^)）]*)[)）])?\s*$'
)

# Modification entry: "- 254号 description：modification"
_MODIFICATION_RE = re.compile(
    r'^-\s+(\d{3}[a-z]?)\s*号\s*(.+?)(?:[：:])\s*(.+)$'
)

# Modification entry alternate format
_MODIFICATION_ALT_RE = re.compile(
    r'^-\s+[「""]?(.+?)[」""]?\s*(?:降级为|提升为|修正为|变更为)\s*(.+?)(?:[：:])\s*(.+)$'
)

# Conclusion section heading
_CONCLUSION_RE = re.compile(
    r'^#{2,3}\s+(?:\d+\.)?\s*(?:结论|核心命题|总判定|收敛结论)',
    re.MULTILINE
)


# --- Extraction Functions ---

def _extract_sections(body: str) -> tuple[Section, ...]:
    """Extract all section headings with their level, number, and title."""
    sections = []
    for match in _SECTION_RE.finditer(body):
        level = len(match.group(1))
        number = match.group(2) or ""
        title = match.group(3).strip()
        line_start = body[:match.start()].count('\n') + 1
        sections.append(Section(
            level=level, number=number, title=title, line_start=line_start
        ))
    return tuple(sections)


def _extract_concepts(body: str) -> tuple[ConceptDefinition, ...]:
    """Extract inline concept definitions (**term**: definition)."""
    concepts = []
    seen_terms = set()
    for i, line in enumerate(body.split('\n'), 1):
        match = _CONCEPT_DEF_RE.search(line)
        if match:
            term = match.group(1).strip()
            definition = match.group(2).strip()
            # Skip frontmatter-style metadata lines (comprehensive list)
            if term.lower() in (
                '状态', '创建时间', '类型', '域', '溯源', '来源',
                '日期', '前置', '关联', '更新时间', '结论', '结论/判定',
                'negation_source', 'negation_form',
                'id', 'title', 'status', 'type', 'date', 'source',
                'link', 'archive',
            ):
                continue
            # Skip table header/separator lines
            if '---' in definition or '|' in definition[:5]:
                continue
            # Deduplicate by term
            if term not in seen_terms:
                seen_terms.add(term)
                concepts.append(ConceptDefinition(
                    term=term, definition=definition, line=i
                ))
    return tuple(concepts)


def _extract_references(body: str) -> tuple[str, ...]:
    """Extract unique genealogy id references (NNN号 pattern)."""
    refs = set()
    for match in _GENEALOGY_REF_RE.finditer(body):
        refs.add(match.group(1))
    return tuple(sorted(refs))


def _extract_section_content(body: str, heading_pattern: str) -> str:
    """Extract content under a specific section heading until next heading."""
    pattern = re.compile(
        r'^#{2,4}\s+(?:\d+(?:\.\d+)*\.)?\s*' + re.escape(heading_pattern),
        re.MULTILINE
    )
    match = pattern.search(body)
    if not match:
        return ""
    start = match.end()
    # Find next heading at same or higher level
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
    """Find section content by trying multiple heading keywords."""
    for kw in keywords:
        content = _extract_section_content(body, kw)
        if content:
            return content
    return ""


def _extract_new_concepts(body: str) -> tuple[NewConcept, ...]:
    """Extract concepts from ### 新增概念 section."""
    section_text = _find_section_by_keywords(body, ["新增概念"])
    if not section_text:
        return ()

    concepts = []
    for line in section_text.split('\n'):
        line = line.strip()
        if not line:
            continue
        match = _NEW_CONCEPT_WITH_SEP_RE.match(line)
        if match:
            term = match.group(1).strip().strip('*')
            qualifier = (match.group(2) or "").strip()
            definition = match.group(3).strip()
            concepts.append(NewConcept(
                term=term, qualifier=qualifier, definition=definition
            ))
            continue
        match = _NEW_CONCEPT_BARE_RE.match(line)
        if match:
            term = match.group(1).strip().strip('*')
            qualifier = (match.group(2) or "").strip()
            concepts.append(NewConcept(
                term=term, qualifier=qualifier, definition=""
            ))
    return tuple(concepts)


def _extract_modifications(body: str) -> tuple[Modification, ...]:
    """Extract modifications from ### 对现有概念的修正 section."""
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
    """Extract first paragraph of conclusion/核心命题 section."""
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
    """Remove YAML frontmatter from markdown text."""
    match = re.match(r'^---\s*\n.*?\n---\s*\n', text, re.DOTALL)
    if match:
        return text[match.end():]
    return text


def analyze_content(genealogy_id: str, text: str) -> ContentAnalysis:
    """Analyze a genealogy file's content, returning structured extraction."""
    text = _normalize_newlines(text)
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

## File 2: scripts/content_enrichment_migration.py (374 lines, key excerpts)

Key design decisions in this file:

### Dedup mechanism
```python
def _relation_dedup_key(rel: dict) -> tuple:
    """Build a dedup key for a relation (from, to, relation, order)."""
    return (rel["from"], rel["to"], rel["relation"], rel.get("order", 0))

def _build_existing_keys(base: Path) -> set[tuple]:
    """Load existing relations and return their dedup keys."""
    try:
        existing = read_all_relations(base)
    except Exception:
        return set()
    return {_relation_dedup_key(r) for r in existing}
```

### Full enrich_single_file function
```python
def enrich_single_file(
    md_file: Path,
    id_mapping: dict[str, str],
    base: Path,
    dry_run: bool = False,
    existing_keys: set[tuple] | None = None,
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

    def _write_rel(rel: dict) -> None:
        nonlocal relations_written
        if existing_keys is not None:
            key = _relation_dedup_key(rel)
            if key in existing_keys:
                return
            existing_keys.add(key)
        append_relation(rel, base)
        relations_written += 1

    # records relation: rewrite -> original
    records_rel = make_relation(
        from_id=rewrite_block["id"],
        to_id=original_sha,
        relation="records",
        order=2,
        created_by=rewrite_block["id"],
    )
    _write_rel(records_rel)

    # defines relations for new_concepts
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
        _write_rel(defines_rel)

    # defines for inline concepts
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
        _write_rel(defines_rel)

    # modifies relations
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
            order=1,
            created_by=rewrite_block["id"],
            target_desc=mod.target_desc,
            modification=mod.modification,
        )
        _write_rel(modifies_rel)

    # references relations
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
        _write_rel(references_rel)

    return {
        "genealogy_id": genealogy_id,
        "rewrite_block": rewrite_block["id"],
        "relations_written": relations_written,
        "concepts": len(ca.concepts) + len(ca.new_concepts),
        "modifications": len(ca.modifications),
        "references": len(ca.references),
    }
```

### run_enrichment function
```python
def run_enrichment(...) -> dict:
    # ...
    existing_keys = _build_existing_keys(base) if not dry_run else None

    for md_file in sorted(settled_dir.glob("*.md")):
        result = enrich_single_file(
            md_file, id_mapping, base, dry_run, existing_keys
        )
```

---

## File 3: scripts/concept_topology_check.py (310 lines, key functions)

```python
def detect_duplicate_concepts(base: Path = DEFAULT_BASE) -> list[dict]:
    """Detect concept definitions that may be duplicates.
    Groups by concept_id. Same concept_id, multiple blocks, different
    definition text → potential duplicate. Excludes modifies-connected pairs.
    """
    relations, reverse_mapping = _load_topology(base)
    concept_defs: dict[str, list[dict]] = defaultdict(list)
    for rel in relations:
        if rel.get("relation") == "defines":
            concept_id = rel["to"]
            concept_defs[concept_id].append({...})

    modifies_pairs: set[tuple[str, str]] = set()
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
        duplicates.append({...})
    return duplicates


def detect_concept_conflicts(base: Path = DEFAULT_BASE) -> list[dict]:
    """O(n*m) where n=modifications, m=referencers.
    A modifies B, C references B but doesn't depend_on A → stale_reference.
    """
    relations, reverse_mapping = _load_topology(base)
    modifications = []
    for rel in relations:
        if rel.get("relation") == "modifies":
            modifications.append({...})
    if not modifications:
        return []
    references_index: dict[str, set[str]] = defaultdict(set)
    for rel in relations:
        if rel.get("relation") == "references":
            references_index[rel["to"]].add(rel["from"])
    depends_on_index: dict[str, set[str]] = defaultdict(set)
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
                conflicts.append({...})
    return conflicts
```

---

## File 4: scripts/block_topology.py (420 lines, key additions)

```python
RELATION_TYPES = frozenset({
    "depends_on", "negates", "related", "tensions_with",
    "supersedes", "residue_of", "reopens",
    "freezes", "splits", "severs",
    "records",
    "negated_by",
    "defines",    # NEW: block defines concept (content-level)
    "modifies",   # NEW: block modifies another block's concept (content-level)
    "references", # NEW: body text references another block (content-level)
})

def make_concept_id(term: str) -> str:
    """Generate a deterministic SHA256 id for a concept term.
    Uses the prefix "concept:" to namespace concept ids and avoid
    collisions with block ids.
    """
    return hashlib.sha256(f"concept:{term}".encode("utf-8")).hexdigest()
```

---

## Specific Design Questions for Review

### Q1: Dedup key sufficiency
The dedup key is `(from, to, relation, order)`. For `defines` relations:
- `from` = block SHA (same block defining multiple concepts → different `to`)
- `to` = `make_concept_id(term)` = SHA256("concept:{term}")
- So different terms → different `to` → different key ✓
- But `extra` fields like `concept_definition` are NOT in the key.
  Question: if the same block redefines the same term with different definition
  text in a re-run, the key matches → the relation is skipped (old definition wins).
  Is this the right behavior?

### Q2: concept_id collision risk
`SHA256("concept:{term}")` shares the 64-hex address space with block ids.
Block ids = SHA256(canonical_json({type, source, content, refs})).
The only collision risk is SHA256("concept:{term}") == SHA256(canonical_json({...})).
With ~300 concepts and ~500 blocks: birthday paradox gives collision prob ≈ 0.
But architecturally: should concept ids use a separate prefix in storage
(e.g., stored in a separate `concepts.jsonl` rather than inline in relations)?

### Q3: _extract_section_content heading level parsing
```python
heading_level = body[match.start():match.end()].count('#', 0,
                    body[match.start():match.end()].index(' '))
```
This computes the heading level by counting '#' up to the first space.
Edge cases:
- "## Section with # in title" → `index(' ')` finds first space after ##,
  `count('#', 0, 2)` = 2 ✓ (works correctly because count is up to first space)
- "##NoSpace" → `index(' ')` raises ValueError (no space after hashes)
  This would crash. Is "##NoSpace" reachable in the genealogy files?

### Q4: detect_concept_conflicts complexity
O(modifications × avg_referencers_per_block).
Currently: ~50 modifies relations × ~10 referencers = ~500 iterations → fine.
At 10k relations: ~500 modifies × ~100 referencers = ~50k iterations → still fine.
The real scaling concern: `read_all_relations()` loads all of relations.jsonl
into memory on every call. Three topology checks each call `read_all_relations()`
independently (3× full file reads). This is the actual bottleneck, not the
in-memory loop complexity.

### Q5: _build_existing_keys memory model
Loads all relations into a set of 4-tuples. At 1300 relations: trivial.
At 10k relations: each 4-tuple is ~4 Python strings averaging ~70 chars each
(SHA256=64 chars). Memory ≈ 10k × 4 × 70 bytes = ~2.8 MB. Fine.
At 1M relations: ~280 MB. The JSONL linear scan becomes the bottleneck before
memory becomes critical.

---

## Context: What This System Does

This is a knowledge graph for a trading theory (缠论 Chanlun). Each "settled"
genealogy file (.md) represents a concluded reasoning event (001–301 files exist).
The block topology is a content-addressable DAG:
- Event blocks: immutable SHA256-addressed JSON files
- Relations: append-only JSONL edge table
- Concept IDs: SHA256("concept:{term}") — virtual nodes in the relation graph

The content enrichment migration:
1. Reads each settled .md file
2. Extracts concepts, modifications, references via regex
3. Writes defines/modifies/references relations to relations.jsonl

All 3363 existing tests pass. 52 new concept tests pass.
```
