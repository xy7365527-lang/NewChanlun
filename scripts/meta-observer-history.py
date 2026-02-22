#!/usr/bin/env python3
"""Scan genealogy directories for meta-rule type records.

Outputs a JSON array of meta-rule genealogy entries sorted by number,
used by meta-observer to perform self-loop checks (comparing historical
observations against current ones).
"""

import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
GENEALOGY_DIRS = [
    REPO_ROOT / ".chanlun" / "genealogy" / "settled",
    REPO_ROOT / ".chanlun" / "genealogy" / "pending",
]

# Patterns to detect meta-rule type in both YAML frontmatter and body metadata
META_RULE_PATTERNS = [
    re.compile(r'^type:\s*["\']?meta-rule', re.MULTILINE),
    re.compile(r'^\*\*type\*\*:\s*meta-rule', re.MULTILINE),
    re.compile(r'^\*\*类型\*\*:\s*meta-rule', re.MULTILINE),
    re.compile(r'^类型:\s*meta-rule', re.MULTILINE),
    re.compile(r'^-\s+\*\*type\*\*:\s*meta-rule', re.MULTILINE),
    re.compile(r'^-\s+\*\*类型\*\*:\s*meta-rule', re.MULTILINE),
]

# Extract number from filename like "021-decentralization-audit.md" or "019a-xxx.md"
NUMBER_RE = re.compile(r'^(\d+[a-z]?)-')

# Extract date from frontmatter or body
DATE_PATTERNS = [
    re.compile(r'^date:\s*["\']?(\d{4}-\d{2}-\d{2})', re.MULTILINE),
    re.compile(r'^\*\*date\*\*:\s*(\d{4}-\d{2}-\d{2})', re.MULTILINE),
    re.compile(r'^\*\*日期\*\*:\s*(\d{4}-\d{2}-\d{2})', re.MULTILINE),
    re.compile(r'^-\s+\*\*date\*\*:\s*(\d{4}-\d{2}-\d{2})', re.MULTILINE),
    re.compile(r'^-\s+\*\*日期\*\*:\s*(\d{4}-\d{2}-\d{2})', re.MULTILINE),
]

# Extract summary: prefer "## 结论" section's first non-empty line,
# then fall back to title from frontmatter or first heading
CONCLUSION_RE = re.compile(r'^##\s*结论[^\n]*\n+(.+)', re.MULTILINE)
TITLE_FM_RE = re.compile(r'^title:\s*["\']?(.+?)["\']?\s*$', re.MULTILINE)
HEADING_RE = re.compile(r'^#\s+\d+\S*\s*[—–\-：:]\s*(.+)', re.MULTILINE)


def is_meta_rule(content: str) -> bool:
    return any(p.search(content) for p in META_RULE_PATTERNS)


def extract_date(content: str) -> str:
    for pattern in DATE_PATTERNS:
        m = pattern.search(content)
        if m:
            return m.group(1)
    return "unknown"


def extract_summary(content: str) -> str:
    """Extract a one-line summary from the genealogy record.

    Priority: "## 结论" section first sentence > frontmatter title > first heading.
    """
    m = CONCLUSION_RE.search(content)
    if m:
        line = m.group(1).strip().strip("*").strip()
        # Take first sentence (up to period, or full line if no period)
        sentence_end = re.search(r'[。．.]\s', line)
        if sentence_end:
            return line[:sentence_end.end()].strip()
        return line

    m = TITLE_FM_RE.search(content)
    if m:
        return m.group(1).strip()

    m = HEADING_RE.search(content)
    if m:
        return m.group(1).strip()

    return "(no summary)"


def extract_number(filename: str) -> str:
    m = NUMBER_RE.match(filename)
    if m:
        return m.group(1)
    return filename


def sort_key(number_str: str):
    """Sort by numeric part first, then alphabetic suffix."""
    m = re.match(r'^(\d+)([a-z]?)$', number_str)
    if m:
        return (int(m.group(1)), m.group(2))
    return (999, number_str)


def scan() -> list[dict]:
    results = []

    for directory in GENEALOGY_DIRS:
        if not directory.exists():
            continue
        for filepath in directory.glob("*.md"):
            content = filepath.read_text(encoding="utf-8")
            if not is_meta_rule(content):
                continue

            number = extract_number(filepath.name)
            date = extract_date(content)
            summary = extract_summary(content)

            results.append({
                "number": number,
                "file": filepath.name,
                "date": date,
                "summary": summary,
            })

    results.sort(key=lambda r: sort_key(r["number"]))
    return results


def main():
    entries = scan()
    print(json.dumps(entries, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
