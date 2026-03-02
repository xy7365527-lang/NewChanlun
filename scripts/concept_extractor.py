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
        # Try standard format: - 254号 X：Y
        match = _MODIFICATION_RE.match(line)
        if match:
            modifications.append(Modification(
                target_id=match.group(1),
                target_desc=match.group(2).strip(),
                modification=match.group(3).strip(),
            ))
            continue
        # Try alternate format: - "X" 降级为Y：Z
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
    # Take first paragraph (up to blank line)
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
