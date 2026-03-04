"""traverse_mcp.py — Genealogy + topology traversal CLI for LLM context injection.

Four commands:
- query <number>: Lookup genealogy entry by number, return content summary
- search <keyword>: Search genealogy entries by keyword (title/content)
- deps <number>: Walk depends_on chain from a genealogy entry
- block <block_id>: Query block topology relations

Read-only. Does not modify genealogy or topology files.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
GENEALOGY_DIR = REPO_ROOT / ".chanlun" / "genealogy"
SETTLED_DIR = GENEALOGY_DIR / "settled"
TOPO_BASE = REPO_ROOT / ".chanlun" / "block-topology"
META_PATH = TOPO_BASE / "meta.json"


# --- YAML frontmatter parser (minimal, no PyYAML dependency) ---


def _parse_frontmatter(text: str) -> dict[str, Any]:
    """Extract YAML frontmatter from a markdown file as a dict.

    Handles simple scalar values, quoted strings, and lists like:
      depends_on: ["001", "002"]
    """
    match = re.match(r"^---\s*\n(.*?)\n---", text, re.DOTALL)
    if not match:
        return {}
    raw = match.group(1)
    result: dict[str, Any] = {}
    for line in raw.split("\n"):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        colon_idx = line.find(":")
        if colon_idx < 0:
            continue
        key = line[:colon_idx].strip()
        value_str = line[colon_idx + 1:].strip()
        # Try JSON parse for lists and quoted strings
        if value_str.startswith("[") or value_str.startswith('"'):
            try:
                result[key] = json.loads(value_str)
            except json.JSONDecodeError:
                result[key] = value_str.strip('"').strip("'")
        else:
            result[key] = value_str.strip('"').strip("'")
    return result


def _read_genealogy_file(path: Path) -> tuple[dict[str, Any], str]:
    """Read a genealogy .md file, return (frontmatter_dict, full_text)."""
    text = path.read_text(encoding="utf-8")
    fm = _parse_frontmatter(text)
    return fm, text


# --- Genealogy index (lazy) ---


def _build_genealogy_index(
    settled_dir: Path = SETTLED_DIR,
) -> dict[str, Path]:
    """Build number -> file path mapping from settled genealogy files.

    File names follow pattern: NNN-slug.md or NNNa-slug.md
    """
    index: dict[str, Path] = {}
    if not settled_dir.exists():
        return index
    for f in sorted(settled_dir.iterdir()):
        if f.suffix != ".md":
            continue
        # Extract number prefix from filename: e.g. "001-degenerate-segment.md" -> "001"
        m = re.match(r"^(\d+[a-z]?)-", f.name)
        if m:
            index[m.group(1)] = f
    return index


def _normalize_number(raw: str) -> str:
    """Normalize genealogy number input: '1' -> '001', '5a' -> '005a'."""
    m = re.match(r"^(\d+)([a-z]?)$", raw.strip())
    if not m:
        return raw.strip()
    num_part = m.group(1).zfill(3)
    suffix = m.group(2)
    return num_part + suffix


# --- Command: query ---


def cmd_query(
    number: str,
    settled_dir: Path = SETTLED_DIR,
    topo_base: Path = TOPO_BASE,
) -> dict[str, Any] | None:
    """Query a genealogy entry by number. Returns structured dict or None."""
    number = _normalize_number(number)
    index = _build_genealogy_index(settled_dir)
    path = index.get(number)
    if path is None:
        return None
    fm, text = _read_genealogy_file(path)

    # Extract first non-frontmatter paragraph as summary
    body = re.sub(r"^---.*?---\s*", "", text, count=1, flags=re.DOTALL)
    # Take first heading + first paragraph
    lines = body.strip().split("\n")
    summary_lines: list[str] = []
    for line in lines:
        summary_lines.append(line)
        if len(summary_lines) > 10:
            break
        if len(summary_lines) > 2 and line.strip() == "":
            break

    # Block topology cross-reference
    block_id = _number_to_block_id(number, topo_base)

    return {
        "number": number,
        "title": fm.get("title", ""),
        "status": fm.get("status", ""),
        "type": fm.get("type", ""),
        "date": fm.get("date", ""),
        "depends_on": fm.get("depends_on", []),
        "related": fm.get("related", []),
        "negates": fm.get("negates", []),
        "negated_by": fm.get("negated_by", []),
        "file": str(path.relative_to(REPO_ROOT)),
        "block_id": block_id,
        "summary": "\n".join(summary_lines).strip(),
    }


# --- Command: search ---


def cmd_search(
    keyword: str,
    settled_dir: Path = SETTLED_DIR,
    max_results: int = 20,
) -> list[dict[str, Any]]:
    """Search genealogy entries by keyword (case-insensitive, matches title and body)."""
    keyword_lower = keyword.lower()
    index = _build_genealogy_index(settled_dir)
    results: list[dict[str, Any]] = []

    for number, path in index.items():
        fm, text = _read_genealogy_file(path)
        title = fm.get("title", "")
        # Score: title match is stronger
        title_match = keyword_lower in title.lower()
        body_match = keyword_lower in text.lower()
        if not (title_match or body_match):
            continue
        results.append({
            "number": number,
            "title": title,
            "status": fm.get("status", ""),
            "type": fm.get("type", ""),
            "title_match": title_match,
            "file": str(path.relative_to(REPO_ROOT)),
        })
        if len(results) >= max_results:
            break

    # Sort: title matches first, then by number
    results.sort(key=lambda r: (not r["title_match"], r["number"]))
    return results


# --- Command: deps ---


def cmd_deps(
    number: str,
    settled_dir: Path = SETTLED_DIR,
    max_depth: int = 10,
) -> dict[str, Any]:
    """Walk depends_on chain from a genealogy entry. Returns dependency tree."""
    number = _normalize_number(number)
    index = _build_genealogy_index(settled_dir)

    visited: set[str] = set()
    chain: list[dict[str, Any]] = []

    def _walk(num: str, depth: int) -> None:
        if num in visited or depth > max_depth:
            return
        visited.add(num)
        path = index.get(num)
        if path is None:
            chain.append({"number": num, "title": "(not found)", "depth": depth})
            return
        fm, _ = _read_genealogy_file(path)
        chain.append({
            "number": num,
            "title": fm.get("title", ""),
            "status": fm.get("status", ""),
            "depth": depth,
        })
        deps = fm.get("depends_on", [])
        if isinstance(deps, list):
            for dep in deps:
                _walk(_normalize_number(str(dep)), depth + 1)

    _walk(number, 0)
    return {
        "root": number,
        "chain": chain,
        "total_nodes": len(chain),
    }


# --- Command: block ---


def _number_to_block_id(
    number: str,
    topo_base: Path = TOPO_BASE,
) -> str | None:
    """Resolve genealogy number to block_id via meta.json id_mapping."""
    meta_path = topo_base / "meta.json"
    if not meta_path.exists():
        return None
    meta = json.loads(meta_path.read_text(encoding="utf-8"))
    mapping = meta.get("id_mapping", {})
    return mapping.get(number)


def cmd_block(
    block_id: str,
    topo_base: Path = TOPO_BASE,
) -> dict[str, Any] | None:
    """Query block topology: read block JSON + collect in/out relations."""
    block_path = topo_base / "blocks" / f"{block_id}.json"
    if not block_path.exists():
        # Try resolving as genealogy number
        resolved = _number_to_block_id(block_id, topo_base)
        if resolved is None:
            return None
        block_id = resolved
        block_path = topo_base / "blocks" / f"{block_id}.json"
        if not block_path.exists():
            return None

    blk = json.loads(block_path.read_text(encoding="utf-8"))
    content = blk.get("content", {})

    # Collect relations
    relations_path = topo_base / "relations.jsonl"
    out_edges: list[dict[str, str]] = []
    in_edges: list[dict[str, str]] = []

    if relations_path.exists():
        for line in relations_path.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if not line:
                continue
            rel = json.loads(line)
            if rel.get("from") == block_id:
                out_edges.append({
                    "relation": rel.get("relation", ""),
                    "to": rel["to"],
                })
            if rel.get("to") == block_id:
                in_edges.append({
                    "relation": rel.get("relation", ""),
                    "from": rel["from"],
                })

    # Reverse-map block_id to genealogy number
    meta_path = topo_base / "meta.json"
    genealogy_number = None
    if meta_path.exists():
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
        for num, bid in meta.get("id_mapping", {}).items():
            if bid == block_id:
                genealogy_number = num
                break

    return {
        "block_id": block_id,
        "genealogy_number": genealogy_number,
        "type": blk.get("type", ""),
        "title": content.get("title", ""),
        "status": content.get("status", ""),
        "source": blk.get("source", ""),
        "out_edges": out_edges,
        "in_edges": in_edges,
        "out_count": len(out_edges),
        "in_count": len(in_edges),
    }


# --- CLI formatting ---


def _format_query(data: dict) -> str:
    lines = [
        f"#{data['number']}: {data['title']}",
        f"  status: {data['status']}",
        f"  type: {data['type']}",
        f"  date: {data['date']}",
        f"  depends_on: {data['depends_on']}",
        f"  related: {data['related']}",
        f"  file: {data['file']}",
    ]
    if data.get("block_id"):
        lines.append(f"  block_id: {data['block_id']}")
    if data.get("summary"):
        lines.append(f"\n{data['summary']}")
    return "\n".join(lines)


def _format_search(results: list[dict]) -> str:
    if not results:
        return "(no matches)"
    lines: list[str] = []
    for r in results:
        marker = "*" if r["title_match"] else " "
        lines.append(f"  {marker} #{r['number']}: {r['title']} [{r['status']}]")
    return f"Found {len(results)} matches:\n" + "\n".join(lines)


def _format_deps(data: dict) -> str:
    lines = [f"Dependency tree for #{data['root']} ({data['total_nodes']} nodes):"]
    for node in data["chain"]:
        indent = "  " * node["depth"]
        status = f" [{node['status']}]" if node.get("status") else ""
        lines.append(f"  {indent}#{node['number']}: {node['title']}{status}")
    return "\n".join(lines)


def _format_block(data: dict) -> str:
    lines = [
        f"Block: {data['block_id'][:16]}...",
        f"  genealogy_number: {data['genealogy_number']}",
        f"  type: {data['type']}",
        f"  title: {data['title']}",
        f"  status: {data['status']}",
        f"  source: {data['source']}",
        f"  out_edges: {data['out_count']}",
        f"  in_edges: {data['in_count']}",
    ]
    if data["out_edges"]:
        lines.append("  out:")
        for e in data["out_edges"][:20]:
            lines.append(f"    {e['relation']} -> {e['to'][:16]}...")
    if data["in_edges"]:
        lines.append("  in:")
        for e in data["in_edges"][:20]:
            lines.append(f"    {e['relation']} <- {e['from'][:16]}...")
    return "\n".join(lines)


# --- Main ---


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(
        description="Genealogy + topology traversal CLI",
    )
    sub = parser.add_subparsers(dest="command")

    p_query = sub.add_parser("query", help="Query genealogy entry by number")
    p_query.add_argument("number", help="Genealogy number (e.g. 347, 5a)")
    p_query.add_argument("--format", choices=["json", "table"], default="table")

    p_search = sub.add_parser("search", help="Search genealogy by keyword")
    p_search.add_argument("keyword", help="Search keyword")
    p_search.add_argument("--max", type=int, default=20, help="Max results")
    p_search.add_argument("--format", choices=["json", "table"], default="table")

    p_deps = sub.add_parser("deps", help="Walk depends_on chain")
    p_deps.add_argument("number", help="Root genealogy number")
    p_deps.add_argument("--max-depth", type=int, default=10)
    p_deps.add_argument("--format", choices=["json", "table"], default="table")

    p_block = sub.add_parser("block", help="Query block topology")
    p_block.add_argument("block_id", help="Block ID (SHA256) or genealogy number")
    p_block.add_argument("--format", choices=["json", "table"], default="table")

    args = parser.parse_args(argv)

    if args.command is None:
        parser.print_help()
        sys.exit(1)

    if args.command == "query":
        result = cmd_query(args.number)
        if result is None:
            print(f"Genealogy #{args.number} not found", file=sys.stderr)
            sys.exit(1)
        if args.format == "json":
            print(json.dumps(result, ensure_ascii=False, indent=2))
        else:
            print(_format_query(result))

    elif args.command == "search":
        results = cmd_search(args.keyword, max_results=args.max)
        if args.format == "json":
            print(json.dumps(results, ensure_ascii=False, indent=2))
        else:
            print(_format_search(results))

    elif args.command == "deps":
        result = cmd_deps(args.number, max_depth=args.max_depth)
        if args.format == "json":
            print(json.dumps(result, ensure_ascii=False, indent=2))
        else:
            print(_format_deps(result))

    elif args.command == "block":
        result = cmd_block(args.block_id)
        if result is None:
            print(f"Block not found: {args.block_id}", file=sys.stderr)
            sys.exit(1)
        if args.format == "json":
            print(json.dumps(result, ensure_ascii=False, indent=2))
        else:
            print(_format_block(result))


if __name__ == "__main__":
    main()
