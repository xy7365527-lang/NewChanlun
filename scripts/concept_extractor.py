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
    kind: str = "unknown"  # "refine" | "revise" | "unknown"


@dataclass(frozen=True)
class Negation:
    """A negation extracted from body text."""
    target_id: str       # "041" or "" if no explicit id
    target_desc: str     # description of the negated item
    negation_type: str   # "rejected_plan" | "negated_concept" | "heterogeneous_source"


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
    negations: tuple[Negation, ...] = ()   # body-extracted negations
    frontmatter_negation: dict = field(default_factory=dict)  # frontmatter negation data


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

# New concept entry in numbered list (### 新增概念 section):
# Format A: "1. **term**（qualifier）——definition"
# Format B: "1. term——definition" (no bold)
# Format C: "1. **term**：definition" (colon separator)
# Format D: "1. term（description）" (no separator — term only, no definition)
# Format E: "1. term" (bare term, no separator)
#
# Strategy: first try structured formats (with separator), then fallback
# to term-only extraction for items without separators.
_NEW_CONCEPT_WITH_SEP_RE = re.compile(
    r'^\d+\.\s+\*{0,2}(.+?)\*{0,2}'
    r'\s*(?:[（(]([^)）]*)[)）])?\s*'
    r'(?:——|—|[：:])\s*(.+)$'
)

# Fallback: numbered item without separator (term only or term + qualifier)
_NEW_CONCEPT_BARE_RE = re.compile(
    r'^\d+\.\s+\*{0,2}(.+?)\*{0,2}'
    r'\s*(?:[（(]([^)）]*)[)）])?\s*$'
)

# Modification entry:
# "- 254号 description：modification" or "- 254号 description: modification"
_MODIFICATION_RE = re.compile(
    r'^-\s+(\d{3}[a-z]?)\s*号\s*(.+?)(?:[：:])\s*(.+)$'
)

# Modification entry alternate format (没有谱系编号前缀):
# "- "X" 降级为Y：description"
_MODIFICATION_ALT_RE = re.compile(
    r'^-\s+[「""]?(.+?)[」""]?\s*(?:降级为|提升为|修正为|变更为)\s*(.+?)(?:[：:])\s*(.+)$'
)

# Dynamic heading: "## 对NNN号的修正" or "### 对NNN号的XXX修正"
# or "## 对NNNx的修正" (号 optional) or "## NNN号 ... 修正分析"
_MODIFICATION_DYNAMIC_HEADING_RE = re.compile(
    r'^(#{2,4})\s+(?:\d+(?:\.\d+)*\.)?\s*(?:对\s*)?(\d{3}[a-z]?)\s*号?.*?修正',
    re.MULTILINE
)

# Before/after pairs in body text:
# "**修正前**：..." / "**修正后**：..." or "**原表述**：..." / "**修正为**：..."
_BEFORE_AFTER_RE = re.compile(
    r'^\s*[-*]*\s*\*\*(?:修正前|原表述)\*\*\s*(?:[：:]|[（(].*?[）)])\s*(.+?)$\s*'
    r'^\s*[-*]*\s*\*\*(?:修正后|修正为)\*\*\s*(?:[：:]|[（(].*?[）)])\s*(.+?)$',
    re.MULTILINE
)

# Old/new pairs:
# "- 旧：..." / "- 新：..."
_OLD_NEW_RE = re.compile(
    r'^-\s+旧[：:]\s*(.+?)$\s*^-\s+新[：:]\s*(.+?)$',
    re.MULTILINE
)

# Conclusion section heading
_CONCLUSION_RE = re.compile(
    r'^#{2,3}\s+(?:\d+\.)?\s*(?:结论|核心命题|总判定|收敛结论)',
    re.MULTILINE
)


# Negation heading patterns
_NEGATION_HEADING_RE = re.compile(
    r'^#{2,4}\s+(?:\d+(?:\.\d+)*\.)?\s*(?:'
    r'被否定的|否定了什么|否定记录|否定关系'
    r'|结算[：:]\s*否定'
    r'|编排者\s*(?:INTERRUPT|否定)'
    r'|否定\s+\d{3}'           # "否定 073 的理由" etc.
    r'|完整否定记录'
    r')',
    re.MULTILINE
)

# Inline **negates**: NNN pattern (for files without standard YAML frontmatter)
_INLINE_NEGATES_RE = re.compile(
    r'^\*\*negates\*\*[：:]\s*(.+)$', re.MULTILINE
)

# Negation list item: - **"term"**: description
_NEGATION_ITEM_BOLD_RE = re.compile(
    r'^-\s+\*\*[「""]?(.+?)[」""]?\*\*\s*[：:]\s*(.+)$'
)

# Negation list item with genealogy id: - 041号 description
_NEGATION_ITEM_ID_RE = re.compile(
    r'^-\s+(\d{3}[a-z]?)\s*号\s*(.+)$'
)

# Modification kind classification keywords
_REVISE_KEYWORDS = re.compile(
    r'替换|取消|废除|重定义|扬弃|根本性|废弃|否定|推翻|颠覆', re.IGNORECASE
)
_REFINE_KEYWORDS = re.compile(
    r'降级|修正|精化|补充|提升为|细化|澄清|限定|收窄', re.IGNORECASE
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
    heading_text = body[match.start():match.end()]
    ws_pos = re.search(r'\s', heading_text)
    heading_level = heading_text.count('#', 0, ws_pos.start() if ws_pos else len(heading_text))
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
        # Try structured format first (with separator: ——, —, ：, :)
        match = _NEW_CONCEPT_WITH_SEP_RE.match(line)
        if match:
            term = match.group(1).strip().strip('*')
            qualifier = (match.group(2) or "").strip()
            definition = match.group(3).strip()
            concepts.append(NewConcept(
                term=term, qualifier=qualifier, definition=definition
            ))
            continue
        # Fallback: bare term (no separator)
        match = _NEW_CONCEPT_BARE_RE.match(line)
        if match:
            term = match.group(1).strip().strip('*')
            qualifier = (match.group(2) or "").strip()
            concepts.append(NewConcept(
                term=term, qualifier=qualifier, definition=""
            ))
    return tuple(concepts)


def _parse_modification_lines(section_text: str,
                              default_target_id: str = "") -> list[Modification]:
    """Parse modification entries from a section's text content.

    Reuses _MODIFICATION_RE and _MODIFICATION_ALT_RE patterns.
    Falls back to default_target_id when the line has no explicit id.
    """
    modifications: list[Modification] = []
    for line in section_text.split('\n'):
        line = line.strip()
        if not line:
            continue
        # Skip before/after formatted lines — these are handled by
        # _parse_modification_content's before/after scanner
        if re.match(
            r'^[-*]*\s*\*\*(?:修正前|修正后|修正为|原表述)\*\*',
            line,
        ):
            continue
        # Skip old/new formatted lines — handled separately
        if re.match(r'^-\s+(?:旧|新)[：:]', line):
            continue
        # Try standard format: - 254号 X：Y
        match = _MODIFICATION_RE.match(line)
        if match:
            target_id = match.group(1)
            target_desc = match.group(2).strip()
            modification_text = match.group(3).strip()
            modifications.append(Modification(
                target_id=target_id,
                target_desc=target_desc,
                modification=modification_text,
                kind=_classify_modification_kind(target_desc, modification_text),
            ))
            continue
        # Try alternate format: - "X" 降级为Y：Z
        match = _MODIFICATION_ALT_RE.match(line)
        if match:
            target_desc = match.group(1).strip()
            mod_action = match.group(2).strip()
            mod_detail = match.group(3).strip()
            modification_text = f"{mod_action}：{mod_detail}"
            modifications.append(Modification(
                target_id=default_target_id,
                target_desc=target_desc,
                modification=modification_text,
                kind=_classify_modification_kind(target_desc, modification_text),
            ))
    return modifications


def _extract_section_from_heading(body: str, heading_match: re.Match) -> str:
    """Extract section content from a heading match to the next same-or-higher level heading."""
    heading_text = body[heading_match.start():heading_match.end()]
    heading_level = 0
    for ch in heading_text:
        if ch == '#':
            heading_level += 1
        else:
            break
    start = heading_match.end()
    next_heading = re.search(
        r'^#{2,' + str(heading_level) + r'}\s',
        body[start:],
        re.MULTILINE
    )
    if next_heading:
        return body[start:start + next_heading.start()].strip()
    return body[start:].strip()


# Table header patterns that indicate a modification table
_MODIFICATION_TABLE_HEADERS = re.compile(
    r'原描述|原表述|修正前|修正方向|修正后|修正的声明|之前|之后',
)


def _parse_modification_table(section_text: str,
                              default_target_id: str = "") -> list[Modification]:
    """Parse modification table rows (| before | after | format).

    Detects tables with headers containing modification-related keywords
    (原描述, 修正方向, 修正前, 修正后, 之前, 之后, etc.) and extracts
    data rows as modifications.
    """
    modifications: list[Modification] = []
    lines = section_text.split('\n')
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        # Look for table header row
        if line.startswith('|') and _MODIFICATION_TABLE_HEADERS.search(line):
            cells = [c.strip() for c in line.split('|')]
            cells = [c for c in cells if c]  # Remove empty from leading/trailing |
            if len(cells) < 2:
                i += 1
                continue
            # Find which columns are "before" and "after"
            before_col = -1
            after_col = -1
            for ci, cell in enumerate(cells):
                if re.search(r'原描述|原表述|修正前|之前', cell):
                    before_col = ci
                if re.search(r'修正方向|修正后|修正的声明|之后', cell):
                    after_col = ci
            if before_col == -1 or after_col == -1:
                i += 1
                continue
            # Skip separator row (|---|---|)
            i += 1
            if i < len(lines) and re.match(r'^\s*\|[\s\-|]+\|\s*$', lines[i]):
                i += 1
            # Parse data rows
            while i < len(lines):
                row = lines[i].strip()
                if not row.startswith('|'):
                    break
                row_cells = [c.strip() for c in row.split('|')]
                row_cells = [c for c in row_cells if c]
                if len(row_cells) > max(before_col, after_col):
                    before_text = row_cells[before_col].strip().strip('"')
                    after_text = row_cells[after_col].strip().strip('"')
                    if before_text and after_text and before_text != '---':
                        modifications.append(Modification(
                            target_id=default_target_id,
                            target_desc=before_text,
                            modification=after_text,
                            kind=_classify_modification_kind(
                                before_text, after_text
                            ),
                        ))
                i += 1
            continue
        i += 1
    return modifications


def _parse_modification_content(section_text: str,
                                default_target_id: str = "") -> list[Modification]:
    """Parse modification content from a section, supporting multiple formats.

    Handles:
    - Standard list items (- NNN号 desc：mod)
    - Alternate list items (- "X" 降级为Y：Z)
    - Before/after pairs (修正前/修正後)
    - Old/new pairs (旧/新)
    - Table rows with 原描述/修正方向 pattern
    """
    modifications = _parse_modification_lines(section_text, default_target_id)

    # Also scan for before/after pairs within this section
    for m in _BEFORE_AFTER_RE.finditer(section_text):
        before_text = m.group(1).strip()
        after_text = m.group(2).strip()
        modifications.append(Modification(
            target_id=default_target_id,
            target_desc=before_text,
            modification=after_text,
            kind=_classify_modification_kind(before_text, after_text),
        ))

    # Also scan for old/new pairs within this section
    for m in _OLD_NEW_RE.finditer(section_text):
        old_text = m.group(1).strip()
        new_text = m.group(2).strip()
        modifications.append(Modification(
            target_id=default_target_id,
            target_desc=old_text,
            modification=new_text,
            kind=_classify_modification_kind(old_text, new_text),
        ))

    # Scan for modification tables: | 原描述/before | 修正方向/after |
    # or | 105号原文 | 保留的意图 | 修正的声明 |
    modifications.extend(
        _parse_modification_table(section_text, default_target_id)
    )

    return modifications


def _find_nearest_genealogy_id(body: str, pos: int) -> str:
    """Search backwards from pos to find the nearest NNN号 reference."""
    # Search in the 500 chars before pos
    search_start = max(0, pos - 500)
    chunk = body[search_start:pos]
    # Find all refs, take the last one (closest to pos)
    refs = list(_GENEALOGY_REF_RE.finditer(chunk))
    if refs:
        return refs[-1].group(1)
    return ""


def _extract_before_after_pairs(body: str) -> list[Modification]:
    """Scan full body for 修正前/修正后 and 旧/新 pairs as fallback."""
    modifications: list[Modification] = []

    for m in _BEFORE_AFTER_RE.finditer(body):
        before_text = m.group(1).strip()
        after_text = m.group(2).strip()
        target_id = _find_nearest_genealogy_id(body, m.start())
        modifications.append(Modification(
            target_id=target_id,
            target_desc=before_text,
            modification=after_text,
            kind=_classify_modification_kind(before_text, after_text),
        ))

    for m in _OLD_NEW_RE.finditer(body):
        old_text = m.group(1).strip()
        new_text = m.group(2).strip()
        target_id = _find_nearest_genealogy_id(body, m.start())
        modifications.append(Modification(
            target_id=target_id,
            target_desc=old_text,
            modification=new_text,
            kind=_classify_modification_kind(old_text, new_text),
        ))

    return modifications


def _deduplicate_modifications(
    mods: list[Modification],
) -> list[Modification]:
    """Deduplicate modifications by (target_id, modification) pair."""
    seen: set[tuple[str, str]] = set()
    result: list[Modification] = []
    for mod in mods:
        key = (mod.target_id, mod.modification)
        if key not in seen:
            seen.add(key)
            result.append(mod)
    return result


def _extract_modifications(body: str) -> tuple[Modification, ...]:
    """Extract modifications with three-layer strategy.

    Layer 1: Standard section headings (对现有概念的修正, 术语修正记录, etc.)
    Layer 2: Dynamic "对NNN号的修正" headings anywhere in body
    Layer 3: Before/after pairs as full-body fallback (only if layers 1+2 empty)
    """
    modifications: list[Modification] = []

    # Layer 1: Standard section headings
    section_text = _find_section_by_keywords(body, [
        "对现有概念的修正",
        "对现有概念的定位修正",
        "术语修正记录",
        "完整否定记录",
    ])
    if section_text:
        modifications.extend(_parse_modification_lines(section_text))

    # Layer 2: Dynamic "对NNN号的修正" headings
    for heading_match in _MODIFICATION_DYNAMIC_HEADING_RE.finditer(body):
        target_id = heading_match.group(2)
        section_content = _extract_section_from_heading(body, heading_match)
        modifications.extend(
            _parse_modification_content(section_content, default_target_id=target_id)
        )

    # Layer 3: Before/after pairs (full-body fallback, only if layers 1+2 empty)
    if not modifications:
        modifications.extend(_extract_before_after_pairs(body))

    return tuple(_deduplicate_modifications(modifications))


def _extract_conclusion_summary(body: str) -> str:
    """Extract first paragraph of conclusion/核心命题 section."""
    match = _CONCLUSION_RE.search(body)
    if not match:
        return ""
    start = match.end()
    rest = body[start:].strip()
    # Take first paragraph (up to blank line)
    para_end = rest.find('\n\n')
    if para_end == -1:
        return rest[:500]
    return rest[:para_end].strip()[:500]


def _classify_modification_kind(target_desc: str, modification: str) -> str:
    """Classify modification as refine/revise/unknown based on keywords.

    Conservative direction principle: unknown → refine (low intensity side).
    """
    combined = f"{target_desc} {modification}"
    if _REVISE_KEYWORDS.search(combined):
        return "revise"
    if _REFINE_KEYWORDS.search(combined):
        return "refine"
    return "unknown"


def _extract_negations(body: str) -> tuple[Negation, ...]:
    """Extract negations from body text under negation-related headings.

    Four-layer strategy:
    - Layer 1: heading anchors (expanded pattern set)
    - Layer 2: list item parsing within matched sections
    - Layer 3: inline **negates**: NNN pattern (files without YAML frontmatter)
    - Layer 4: frontmatter (handled separately in _parse_frontmatter)
    """
    negations = []
    seen_targets = set()

    # Layer 1+2: Find negation sections and parse list items
    for heading_match in _NEGATION_HEADING_RE.finditer(body):
        start = heading_match.end()
        heading_text = body[heading_match.start():heading_match.end()]
        ws_pos = re.search(r'\s', heading_text)
        heading_level = heading_text.count(
            '#', 0, ws_pos.start() if ws_pos else len(heading_text)
        )
        next_heading = re.search(
            r'^#{2,' + str(heading_level) + r'}\s',
            body[start:],
            re.MULTILINE
        )
        section_end = start + next_heading.start() if next_heading else len(body)
        section_text = body[start:section_end]

        for line in section_text.split('\n'):
            line = line.strip()
            if not line or not line.startswith('-'):
                continue

            # Try ID format: - 041号 description
            match = _NEGATION_ITEM_ID_RE.match(line)
            if match:
                target_id = match.group(1)
                target_desc = match.group(2).strip()
                key = (target_id, target_desc)
                if key not in seen_targets:
                    seen_targets.add(key)
                    negations.append(Negation(
                        target_id=target_id,
                        target_desc=target_desc,
                        negation_type="negated_concept",
                    ))
                continue

            # Try bold format: - **"term"**: description
            match = _NEGATION_ITEM_BOLD_RE.match(line)
            if match:
                target_desc = match.group(1).strip()
                detail = match.group(2).strip()
                key = ("", target_desc)
                if key not in seen_targets:
                    seen_targets.add(key)
                    negations.append(Negation(
                        target_id="",
                        target_desc=target_desc,
                        negation_type="rejected_plan",
                    ))
                continue

            # Fallback: whole line as description
            clean = line.lstrip('- ').strip()
            if clean:
                key = ("", clean)
                if key not in seen_targets:
                    seen_targets.add(key)
                    negations.append(Negation(
                        target_id="",
                        target_desc=clean,
                        negation_type="rejected_plan",
                    ))

    # Layer 3: Scan for inline **negates**: NNN patterns
    _id_re = re.compile(r'\d{3}[a-z]?')
    for inline_match in _INLINE_NEGATES_RE.finditer(body):
        value = inline_match.group(1).strip()
        for id_match in _id_re.finditer(value):
            target_id = id_match.group(0)
            key = (target_id, "")
            if key not in seen_targets:
                seen_targets.add(key)
                negations.append(Negation(
                    target_id=target_id,
                    target_desc="",
                    negation_type="negated_concept",
                ))

    return tuple(negations)


def _parse_frontmatter(text: str) -> tuple[dict, str]:
    """Parse YAML frontmatter and return (frontmatter_dict, body).

    YAML parse failure gracefully degrades: returns ({}, full_text).
    """
    match = re.match(r'^---\s*\n(.*?)\n---\s*\n', text, re.DOTALL)
    if not match:
        return {}, text
    try:
        import yaml
        fm = yaml.safe_load(match.group(1)) or {}
    except Exception:
        fm = {}
    return fm, text[match.end():]


def _extract_frontmatter_negation(fm: dict) -> dict:
    """Extract negation-related fields from frontmatter dict.

    Returns a dict with negation_source, negation_form, negates, negated_by.
    Handles Chinese annotations in negates/negated_by values,
    e.g. "073a号（depth_budget 基因废除）" → "073a".
    """
    result = {}
    for key in ("negation_source", "negation_form"):
        if key in fm:
            result[key] = fm[key]
    for key in ("negates", "negated_by"):
        val = fm.get(key)
        if val and isinstance(val, list) and any(v for v in val):
            cleaned = []
            for v in val:
                if not v:
                    continue
                s = str(v)
                m = re.match(r'(\d{3}[a-z]?)', s)
                if m:
                    cleaned.append(m.group(1))
                else:
                    cleaned.append(s)
            if cleaned:
                result[key] = cleaned
    return result


def _strip_frontmatter(text: str) -> str:
    """Remove YAML frontmatter from markdown text."""
    match = re.match(r'^---\s*\n.*?\n---\s*\n', text, re.DOTALL)
    if match:
        return text[match.end():]
    return text


# --- Main Entry Point ---

def analyze_content(genealogy_id: str, text: str) -> ContentAnalysis:
    """Analyze a genealogy file's content, returning structured extraction.

    Args:
        genealogy_id: The genealogy id (e.g. "292", "005b")
        text: The full file content (including frontmatter)

    Returns:
        ContentAnalysis with all extracted information
    """
    text = _normalize_newlines(text)
    fm, body = _parse_frontmatter(text)

    return ContentAnalysis(
        genealogy_id=genealogy_id,
        sections=_extract_sections(body),
        concepts=_extract_concepts(body),
        references=_extract_references(body),
        new_concepts=_extract_new_concepts(body),
        modifications=_extract_modifications(body),
        conclusion_summary=_extract_conclusion_summary(body),
        negations=_extract_negations(body),
        frontmatter_negation=_extract_frontmatter_negation(fm),
    )
