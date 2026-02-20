#!/usr/bin/env python3
"""Add YAML frontmatter to settled genealogy files that lack it."""

import re
from pathlib import Path

SETTLED_DIR = Path(__file__).resolve().parent.parent / ".chanlun" / "genealogy" / "settled"
DEFAULT_DATE = "2026-02-17"


def extract_id(filename: str) -> str:
    """Extract id from filename: 005b-xxx.md -> '005b'."""
    return filename.split("-", 1)[0]


def extract_field(content: str, label: str) -> str | None:
    """Extract value from **label**: value pattern."""
    m = re.search(rf"\*\*{re.escape(label)}\*\*\s*[:：]\s*(.+)", content)
    return m.group(1).strip() if m else None


def extract_title(content: str, file_id: str) -> str:
    """Extract human-readable title from first heading."""
    m = re.search(r"^#\s+(.+)$", content, re.MULTILINE)
    if not m:
        return file_id
    raw = m.group(1).strip()
    # Strip common prefixes: "矛盾记录 001：", "概念分离 003：", "019a — ", "059: ", etc.
    raw = re.sub(r"^(?:矛盾记录|概念分离|概念结算|语法记录|对象否定对象原则)?\s*" + re.escape(file_id) + r"\s*[：:—–\-]\s*", "", raw)
    # Also handle bare numeric prefix like "# 040 — ..."
    raw = re.sub(r"^\d+[a-z]?\s*[：:—–\-]\s*", "", raw)
    return raw.strip() or file_id


def extract_refs(content: str) -> list[str]:
    """Extract genealogy reference numbers from 前置/关联/谱系链接 sections."""
    refs = set()
    for pattern in [r"\*\*前置\*\*\s*[:：]\s*(.+)", r"\*\*关联\*\*\s*[:：]\s*(.+)"]:
        m = re.search(pattern, content)
        if m:
            refs.update(re.findall(r"(\d{3}[a-z]?)", m.group(1)))
    # Also scan ## 谱系链接 section
    sec = re.search(r"##\s*谱系链接\s*\n((?:.*\n)*?)(?=\n##|\Z)", content)
    if sec:
        refs.update(re.findall(r"(\d{3}[a-z]?)", sec.group(1)))
    return sorted(refs)


def extract_depends(content: str) -> list[str]:
    """Extract depends_on from 前置 field."""
    m = re.search(r"\*\*前置\*\*\s*[:：]\s*(.+)", content)
    if not m:
        return []
    val = m.group(1)
    if re.search(r"无|首条", val):
        return []
    return sorted(set(re.findall(r"(\d{3}[a-z]?)", val)))


def extract_related(content: str) -> list[str]:
    """Extract related from 关联 field and 谱系链接 section."""
    related = set()
    m = re.search(r"\*\*关联\*\*\s*[:：]\s*(.+)", content)
    if m:
        related.update(re.findall(r"(\d{3}[a-z]?)", m.group(1)))
    sec = re.search(r"##\s*谱系链接\s*\n((?:.*\n)*?)(?=\n##|\Z)", content)
    if sec:
        related.update(re.findall(r"(\d{3}[a-z]?)", sec.group(1)))
    return sorted(related)


def build_frontmatter(file_id: str, content: str) -> str:
    title = extract_title(content, file_id)
    typ = extract_field(content, "类型") or "矛盾记录"
    date = extract_field(content, "日期") or extract_field(content, "创建时间") or DEFAULT_DATE
    # Clean date: take first date-like value
    dm = re.search(r"(\d{4}-\d{2}-\d{2})", date)
    date = dm.group(1) if dm else DEFAULT_DATE
    depends = extract_depends(content)
    related = extract_related(content)
    # Remove depends items from related
    related = sorted(set(related) - set(depends))

    def fmt_list(lst):
        if not lst:
            return "[]"
        return "[" + ", ".join(f'"{x}"' for x in lst) + "]"

    return f"""---
id: "{file_id}"
title: "{title}"
status: "已结算"
type: "{typ}"
date: "{date}"
depends_on: {fmt_list(depends)}
related: {fmt_list(related)}
negated_by: []
negates: []
---
"""


def main():
    files = sorted(SETTLED_DIR.glob("*.md"))
    modified = 0
    skipped = 0
    for f in files:
        content = f.read_text(encoding="utf-8")
        if content.startswith("---"):
            skipped += 1
            continue
        fm = build_frontmatter(extract_id(f.stem), content)
        f.write_text(fm + "\n" + content, encoding="utf-8")
        modified += 1
        print(f"  + {f.name}")
    print(f"\nDone: {modified} modified, {skipped} skipped (already had frontmatter)")


if __name__ == "__main__":
    main()
