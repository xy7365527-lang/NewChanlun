"""Deduplicate k_full.jsonl — removes exact duplicate lines.

Usage:
    python dedup_jsonl.py [path_to_k_full.jsonl]

Default path: ~/.swarm/k_full.jsonl

Writes deduplicated output to the same file (atomic: write tmp then rename).
Prints stats to stderr.
"""

import json
import sys
import tempfile
from pathlib import Path


def dedup_jsonl(path: Path) -> tuple[int, int]:
    """Remove duplicate JSONL records, preserving order of first occurrence.

    For vertex records: dedup by (type, id)
    For edge records: dedup by (type, source, target, edge_type)
    For other records: dedup by full JSON content

    Returns (original_count, deduped_count).
    """
    if not path.exists():
        print(f"File not found: {path}", file=sys.stderr)
        return 0, 0

    seen_vertices: set[str] = set()      # vertex id
    seen_edges: set[tuple] = set()        # (source, target, edge_type)
    seen_other: set[str] = set()          # full json string
    kept: list[str] = []
    original = 0

    with open(path, encoding="utf-8") as f:
        for line in f:
            stripped = line.strip()
            if not stripped:
                continue
            original += 1
            try:
                record = json.loads(stripped)
            except json.JSONDecodeError:
                kept.append(stripped)
                continue

            rtype = record.get("type")
            if rtype == "vertex":
                key = record["id"]
                if key in seen_vertices:
                    continue
                seen_vertices.add(key)
            elif rtype == "edge":
                key = (record["source"], record["target"], record["edge_type"])
                if key in seen_edges:
                    continue
                seen_edges.add(key)
            else:
                # operation, merge, vertex_status, settlement — dedup by content
                canonical = json.dumps(record, sort_keys=True, ensure_ascii=False)
                if canonical in seen_other:
                    continue
                seen_other.add(canonical)

            kept.append(stripped)

    # Atomic write: tmp file then rename
    tmp_fd, tmp_path = tempfile.mkstemp(
        dir=str(path.parent), suffix=".jsonl.tmp"
    )
    try:
        with open(tmp_fd, "w", encoding="utf-8") as tmp_f:
            for line in kept:
                tmp_f.write(line + "\n")
        Path(tmp_path).replace(path)
    except Exception:
        Path(tmp_path).unlink(missing_ok=True)
        raise

    return original, len(kept)


def main():
    default = Path.home() / ".swarm" / "k_full.jsonl"
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else default

    print(f"Deduplicating: {path}", file=sys.stderr)
    original, deduped = dedup_jsonl(path)
    removed = original - deduped
    if original == 0:
        print("File empty or not found.", file=sys.stderr)
    else:
        ratio = removed / original * 100
        print(
            f"Original: {original} lines, Kept: {deduped}, "
            f"Removed: {removed} ({ratio:.1f}%)",
            file=sys.stderr,
        )


if __name__ == "__main__":
    main()
