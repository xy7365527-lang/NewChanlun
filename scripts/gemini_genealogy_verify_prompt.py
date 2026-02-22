#!/usr/bin/env python3
"""Generate structured Gemini verify context for a newly written genealogy entry.

Input:  path to a genealogy .md file (positional arg)
Output: JSON with verify context to stdout

Exit codes:
  0 — success (JSON printed)
  1 — file not found or parse failure (error JSON printed)
"""

import json
import os
import re
import sys
from pathlib import Path


def extract_frontmatter(content: str) -> dict[str, str]:
    """Extract YAML-like frontmatter fields as flat key-value pairs."""
    m = re.match(r"^---\s*\n(.*?)\n---", content, re.DOTALL)
    if not m:
        return {}
    fields: dict[str, str] = {}
    for line in m.group(1).splitlines():
        kv = line.split(":", 1)
        if len(kv) == 2:
            key = kv[0].strip()
            val = kv[1].strip().strip("'\"")
            fields[key] = val
    return fields


def extract_markdown_field(content: str, field_name: str) -> str:
    """Extract a markdown bold field like **类型**: value."""
    pattern = rf"(?:\*\*{re.escape(field_name)}\*\*|{re.escape(field_name)})\s*[:：]\s*(.*)"
    m = re.search(pattern, content)
    return m.group(1).strip() if m else ""


def extract_section(content: str, heading: str) -> str:
    """Extract the body of a ## heading section."""
    pattern = rf"##\s*{re.escape(heading)}\s*\n(.*?)(?=\n##\s|\Z)"
    m = re.search(pattern, content, re.DOTALL)
    return m.group(1).strip() if m else ""


def extract_number_from_path(file_path: str) -> str:
    """Extract genealogy number (e.g. '094') from filename like 094-gap-reposition.md."""
    basename = os.path.basename(file_path)
    m = re.match(r"(\d{3})", basename)
    return m.group(1) if m else ""


def extract_prerequisite_ids(prereq_value: str) -> list[str]:
    """Extract prerequisite genealogy IDs from the 前置 field value."""
    if not prereq_value or prereq_value in ("无", "N/A", "-", "（无）", "(无)", "(none)", "none"):
        return []
    return re.findall(r"(\d{3})", prereq_value)


def find_genealogy_file(genealogy_dir: str, number: str) -> str | None:
    """Find the genealogy file matching a given number in settled/ or pending/."""
    for subdir in ("settled", "pending"):
        dirpath = os.path.join(genealogy_dir, subdir)
        if not os.path.isdir(dirpath):
            continue
        for fname in os.listdir(dirpath):
            if fname.startswith(number) and fname.endswith(".md"):
                return os.path.join(dirpath, fname)
    return None


def extract_conclusion_summary(content: str) -> str:
    """Extract a short conclusion summary from 核心命题 or 结论 sections."""
    for heading in ("核心命题", "结论", "核心结论"):
        section = extract_section(content, heading)
        if section:
            # Return first paragraph (up to 200 chars)
            first_para = section.split("\n\n")[0].strip()
            if len(first_para) > 200:
                return first_para[:200] + "..."
            return first_para
    return ""


def build_verify_prompt(file_path: str) -> dict:
    """Build the full verify prompt JSON for a genealogy file."""
    file_path = os.path.abspath(file_path)
    if not os.path.isfile(file_path):
        return {"error": f"File not found: {file_path}"}

    with open(file_path, encoding="utf-8") as f:
        content = f.read()

    frontmatter = extract_frontmatter(content)
    number = extract_number_from_path(file_path)

    # Extract fields (prefer markdown bold format, fallback to frontmatter)
    entry_type = extract_markdown_field(content, "类型") or frontmatter.get("type", "")
    status = extract_markdown_field(content, "状态") or frontmatter.get("status", "")
    prereq_raw = extract_markdown_field(content, "前置") or ""

    # Build depends_on from frontmatter if available
    depends_on_raw = frontmatter.get("depends_on", "")
    prereq_ids = extract_prerequisite_ids(prereq_raw)
    if not prereq_ids and depends_on_raw:
        prereq_ids = re.findall(r"(\d{3})", depends_on_raw)

    conclusion_summary = extract_conclusion_summary(content)
    boundary_conditions = extract_section(content, "边界条件")
    downstream = extract_section(content, "下游推论")

    # Resolve prerequisite conclusions
    genealogy_dir = os.path.join(os.path.dirname(file_path), "..")
    genealogy_dir = os.path.normpath(genealogy_dir)
    prerequisite_files: list[str] = []
    prerequisite_conclusions: dict[str, str] = {}

    for pid in prereq_ids:
        prereq_file = find_genealogy_file(genealogy_dir, pid)
        if prereq_file is not None:
            prereq_basename = os.path.basename(prereq_file).replace(".md", "")
            prerequisite_files.append(prereq_basename)
            with open(prereq_file, encoding="utf-8") as pf:
                prereq_content = pf.read()
            prereq_conclusion = extract_conclusion_summary(prereq_content)
            if prereq_conclusion:
                prerequisite_conclusions[prereq_basename] = prereq_conclusion

    basename = os.path.basename(file_path)

    return {
        "genealogy_number": number,
        "file": basename,
        "type": entry_type,
        "status": status,
        "conclusion_summary": conclusion_summary,
        "prerequisites": prerequisite_files,
        "prerequisite_conclusions": prerequisite_conclusions,
        "boundary_conditions": boundary_conditions,
        "downstream_implications": downstream,
        "verify_questions": [
            "新谱系的结论是否与前置谱系的结论逻辑兼容？",
            "边界条件是否完备——是否有遗漏的翻转条件？",
            "下游推论是否已在系统中被消费或注册？",
        ],
    }


def main() -> None:
    if len(sys.argv) < 2:
        print(json.dumps({"error": "Usage: gemini_genealogy_verify_prompt.py <genealogy_file_path>"}, ensure_ascii=False))
        sys.exit(1)

    file_path = sys.argv[1]
    result = build_verify_prompt(file_path)

    if "error" in result:
        print(json.dumps(result, ensure_ascii=False))
        sys.exit(1)

    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
