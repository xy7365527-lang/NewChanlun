#!/usr/bin/env python3
"""Generate a snapshot from an existing JSONL file.

Loads the full JSONL, builds the Graph, then writes a compact snapshot.
Run on VPS where the JSONL files live:

    python scripts/dump_snapshot.py /root/.swarm/persist/fengliang_1.jsonl

After snapshot is written, optionally truncate the JSONL:

    python scripts/dump_snapshot.py /root/.swarm/persist/fengliang_1.jsonl --truncate
"""

from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

# Add topological-computation to path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "topological-computation"))

from persistence import PersistentKFull


def main():
    parser = argparse.ArgumentParser(description="Generate snapshot from JSONL")
    parser.add_argument("jsonl_path", help="Path to the JSONL file")
    parser.add_argument(
        "--truncate",
        action="store_true",
        help="Truncate JSONL after snapshot (default: keep)",
    )
    parser.add_argument(
        "--output",
        help="Snapshot output path (default: derived from JSONL path)",
    )
    args = parser.parse_args()

    jsonl_path = Path(args.jsonl_path)
    if not jsonl_path.exists():
        print(f"JSONL file not found: {jsonl_path}", file=sys.stderr)
        sys.exit(1)

    jsonl_size_mb = jsonl_path.stat().st_size / (1024 * 1024)
    print(f"Loading JSONL: {jsonl_path} ({jsonl_size_mb:.1f} MB)", file=sys.stderr)

    t0 = time.time()
    graph, operations = PersistentKFull.load(jsonl_path)
    t_load = time.time() - t0

    v_count = len(graph.vertices)
    e_count = len(graph.edges)
    active_count = len(graph.active_vertex_ids())
    print(
        f"Graph loaded in {t_load:.1f}s: {v_count} vertices ({active_count} active), "
        f"{e_count} edges, {len(operations)} operations",
        file=sys.stderr,
    )

    if args.output:
        snapshot_path = Path(args.output)
    else:
        snapshot_path = PersistentKFull.snapshot_path_for(jsonl_path)

    t0 = time.time()
    count = PersistentKFull.dump_snapshot(graph, snapshot_path)
    t_dump = time.time() - t0

    snapshot_size_mb = snapshot_path.stat().st_size / (1024 * 1024)
    print(
        f"Snapshot written in {t_dump:.1f}s: {count} records, "
        f"{snapshot_size_mb:.1f} MB → {snapshot_path}",
        file=sys.stderr,
    )
    print(
        f"Compression: {jsonl_size_mb:.1f} MB → {snapshot_size_mb:.1f} MB "
        f"({snapshot_size_mb / jsonl_size_mb * 100:.1f}%)",
        file=sys.stderr,
    )

    if args.truncate:
        jsonl_path.write_text("", encoding="utf-8")
        print(f"JSONL truncated: {jsonl_path}", file=sys.stderr)

    print(json.dumps({
        "jsonl_path": str(jsonl_path),
        "snapshot_path": str(snapshot_path),
        "jsonl_size_mb": round(jsonl_size_mb, 1),
        "snapshot_size_mb": round(snapshot_size_mb, 1),
        "vertices": v_count,
        "active_vertices": active_count,
        "edges": e_count,
        "operations": len(operations),
        "records_written": count,
        "load_time_s": round(t_load, 1),
        "dump_time_s": round(t_dump, 1),
        "truncated": args.truncate,
    }, indent=2))


if __name__ == "__main__":
    import json
    main()
