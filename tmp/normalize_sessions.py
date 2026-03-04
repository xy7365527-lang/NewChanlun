"""Normalize session file formats to a canonical structure.

Canonical format:
  # Session: {id}
  **时间**: ...
  **基底 commit**: ...
  **蜂群**: ...
  ## 定义基底
  ## 谱系状态
  ## 本轮工作
  ## 测试基线
  ## 中断点
  ## 下一轮方向
  ## 恢复指引
"""

import re
import sys
from pathlib import Path

SESSIONS_DIR = Path(r"C:\Users\hanju\NewChanlun\.chanlun\sessions")

# Canonical section order (used for sorting)
SECTION_ORDER = [
    "定义基底",
    "谱系状态",
    "本轮工作",
    "蜂群执行",
    "Gemini",  # prefix match for various Gemini sections
    "测试基线",
    "Quality-Guard",
    "走势结构",
    "范式转换",
    "数字",
    "中断点",
    "遗留项",
    "下一轮",
    "阻塞项",
    "恢复指引",
    "Kiro",
]

# Section name normalization map
SECTION_RENAMES = {
    "本次 session 完成项": "本轮工作",
    "本轮进展": "本轮工作",
    "产出": "本轮工作",
}


def extract_session_id(filename: str, content: str) -> str:
    """Extract session ID from filename or content."""
    # From YAML frontmatter
    m = re.search(r"^session_id:\s*(.+)$", content, re.MULTILINE)
    if m:
        return m.group(1).strip().strip('"').strip("'")
    # From filename
    stem = filename.replace("-session.md", "").replace("-session", "")
    return stem


def strip_yaml_frontmatter(content: str) -> tuple[str, dict]:
    """Remove YAML frontmatter, return (remaining_content, frontmatter_dict)."""
    fm = {}
    if content.startswith("---"):
        end = content.find("---", 3)
        if end != -1:
            block = content[3:end].strip()
            for line in block.split("\n"):
                if ":" in line:
                    k, v = line.split(":", 1)
                    fm[k.strip()] = v.strip().strip('"').strip("'")
            content = content[end + 3:].lstrip("\n")
    return content, fm


def extract_metadata(content: str) -> tuple[dict, str]:
    """Extract **key**: value metadata lines from top of content."""
    meta = {}
    lines = content.split("\n")
    remaining_start = 0

    for i, line in enumerate(lines):
        m = re.match(r"^\*\*(.+?)\*\*[:：]\s*(.+)$", line)
        if m:
            meta[m.group(1)] = m.group(2)
            remaining_start = i + 1
        elif line.strip() == "" and not meta:
            remaining_start = i + 1
        elif line.strip() == "" and meta:
            remaining_start = i + 1
            # Check if next non-empty line is also metadata
            continue
        else:
            if not meta:
                remaining_start = i
            break

    return meta, "\n".join(lines[remaining_start:])


def parse_sections(content: str) -> list[tuple[str, str]]:
    """Parse content into (header, body) pairs. First pair has header='' for preamble."""
    sections = []
    current_header = ""
    current_lines = []

    for line in content.split("\n"):
        if line.startswith("## ") and not line.startswith("### "):
            if current_header or current_lines:
                sections.append((current_header, "\n".join(current_lines)))
            current_header = line[3:].strip()
            current_lines = []
        else:
            current_lines.append(line)

    if current_header or current_lines:
        sections.append((current_header, "\n".join(current_lines)))

    return sections


def deduplicate_sections(sections: list[tuple[str, str]]) -> list[tuple[str, str]]:
    """Merge sections with identical headers, keeping the longest body."""
    seen = {}
    result = []
    for header, body in sections:
        if header == "":
            result.append((header, body))
            continue
        if header in seen:
            idx = seen[header]
            old_body = result[idx][1]
            # Keep the one with more content
            if len(body.strip()) > len(old_body.strip()):
                result[idx] = (header, body)
        else:
            seen[header] = len(result)
            result.append((header, body))
    return result


def normalize_section_name(name: str) -> str:
    """Normalize section header variants."""
    # Strip parenthetical suffixes for matching
    base = re.sub(r"[（(].+?[）)]$", "", name).strip()
    if base in SECTION_RENAMES:
        # Preserve parenthetical if present
        suffix = name[len(base):]
        return SECTION_RENAMES[base] + suffix
    return name


def section_sort_key(header: str) -> int:
    """Return sort index for a section header."""
    for i, prefix in enumerate(SECTION_ORDER):
        if header.startswith(prefix):
            return i
    return len(SECTION_ORDER)  # unknown sections go before 恢复指引


def build_metadata_block(meta: dict, session_id: str, fm: dict) -> str:
    """Build canonical metadata lines."""
    lines = []

    # 时间
    time_val = meta.get("时间") or meta.get("日期") or fm.get("start_time", session_id)
    lines.append(f"**时间**: {time_val}")

    # 基底 commit
    commit_val = meta.get("基底 commit") or meta.get("最新提交") or ""
    if commit_val:
        lines.append(f"**基底 commit**: {commit_val}")

    # 蜂群
    swarm_val = meta.get("蜂群", "")
    if swarm_val:
        lines.append(f"**蜂群**: {swarm_val}")

    # Preserve other metadata that doesn't fit canonical fields
    skip_keys = {"时间", "日期", "基底 commit", "最新提交", "蜂群", "分支", "工作树", "触发", "模型", "走势结构"}
    extras = {k: v for k, v in meta.items() if k not in skip_keys}

    # Include some useful extras
    for key in ["触发", "模型", "走势结构", "分支"]:
        if key in meta:
            lines.append(f"**{key}**: {meta[key]}")

    return "\n".join(lines)


def normalize_file(filepath: Path, dry_run: bool = False) -> str:
    """Normalize a single session file. Returns the normalized content."""
    content = filepath.read_text(encoding="utf-8")
    filename = filepath.name
    session_id = extract_session_id(filename, content)

    # Strip YAML frontmatter
    content, fm = strip_yaml_frontmatter(content)

    # Strip existing title line
    title_match = re.match(r"^#\s+.+\n+", content)
    if title_match:
        content = content[title_match.end():]

    # Extract metadata
    meta, body = extract_metadata(content)

    # Parse sections
    sections = parse_sections(body)

    # Remove empty preamble
    if sections and sections[0][0] == "" and not sections[0][1].strip():
        sections = sections[1:]
    elif sections and sections[0][0] == "":
        # Non-empty preamble - might contain stray content, keep it
        pass

    # Normalize section names
    sections = [(normalize_section_name(h), b) for h, b in sections]

    # Deduplicate
    sections = deduplicate_sections(sections)

    # Remove `→ 来源:` annotation lines
    cleaned_sections = []
    for header, body_text in sections:
        body_lines = body_text.split("\n")
        body_lines = [l for l in body_lines if not l.strip().startswith("→ 来源:")]
        cleaned_sections.append((header, "\n".join(body_lines)))
    sections = cleaned_sections

    # Sort sections by canonical order
    preamble = []
    sortable = []
    for header, body_text in sections:
        if header == "":
            preamble.append((header, body_text))
        else:
            sortable.append((header, body_text))

    sortable.sort(key=lambda x: section_sort_key(x[0]))

    # Build output
    out_lines = [f"# Session: {session_id}", ""]
    out_lines.append(build_metadata_block(meta, session_id, fm))
    out_lines.append("")

    # Add preamble content if any
    for _, body_text in preamble:
        stripped = body_text.strip()
        if stripped:
            out_lines.append(stripped)
            out_lines.append("")

    # Add sections
    for header, body_text in sortable:
        out_lines.append(f"## {header}")
        # Ensure body starts with blank line after header
        body_stripped = body_text.strip()
        if body_stripped:
            out_lines.append("")
            out_lines.append(body_stripped)
        out_lines.append("")

    result = "\n".join(out_lines).rstrip() + "\n"

    if not dry_run:
        filepath.write_text(result, encoding="utf-8")

    return result


def main():
    dry_run = "--dry-run" in sys.argv
    target = sys.argv[-1] if len(sys.argv) > 1 and not sys.argv[-1].startswith("--") else None

    files = sorted(SESSIONS_DIR.glob("*.md"))
    if target:
        files = [f for f in files if target in f.name]

    print(f"{'[DRY RUN] ' if dry_run else ''}Normalizing {len(files)} session files...")

    for f in files:
        try:
            normalize_file(f, dry_run=dry_run)
            print(f"  OK: {f.name}")
        except Exception as e:
            print(f"  FAIL: {f.name}: {e}")

    print(f"Done. {len(files)} files processed.")


if __name__ == "__main__":
    main()
