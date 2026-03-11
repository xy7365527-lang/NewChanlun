"""Purge ghost settlements from checkpoint state.

Ghost settlements are settled cycles whose edges/vertices no longer exist
in the active graph. They block operations (negate, fold, sublate) without
protecting anything real.

This script:
1. Loads the graph from k_full.jsonl (same path daemon uses)
2. Loads the checkpoint state
3. Identifies ghost settlements (edges not in active graph)
4. Removes them from the checkpoint
5. Resets blocked_streak to 0
6. Saves the cleaned checkpoint

Usage:
    python purge_ghost_settlements.py [--dry-run]

410号谱系: ghost settlement = settled cycle whose edges reference
vertices/edges that no longer exist in the graph.
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path


def load_active_edges_from_jsonl(jsonl_path: Path) -> tuple[set[str], set[tuple[str, str]]]:
    """Replay JSONL to reconstruct active vertices and edges.

    Returns (active_vertex_ids, active_edges_as_tuples).
    """
    vertices: dict[str, str] = {}  # id -> status
    edges: set[tuple[str, str]] = set()

    with open(jsonl_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            t = rec.get("type")
            if t == "vertex":
                vertices[rec["id"]] = rec.get("status", "active")
            elif t == "edge":
                edges.add((rec["source"], rec["target"]))
            elif t == "status_change":
                vertices[rec["id"]] = rec["new_status"]
            elif t == "merge":
                vertices[rec["remove"]] = "folded"
                keep = rec["keep"]
                remove = rec["remove"]
                edges = {
                    (keep if s == remove else s, keep if tg == remove else tg)
                    for s, tg in edges
                }

    active_vids = {vid for vid, st in vertices.items() if st != "folded"}
    active_edges = {(s, t) for s, t in edges if s in active_vids and t in active_vids}
    return active_vids, active_edges


def main() -> None:
    parser = argparse.ArgumentParser(description="Purge ghost settlements from checkpoint")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be removed without modifying")
    parser.add_argument(
        "--checkpoint",
        type=str,
        default=str(Path.home() / ".swarm" / "checkpoint" / "default_state.json"),
        help="Path to checkpoint state file",
    )
    parser.add_argument(
        "--jsonl",
        type=str,
        default=str(Path.home() / ".topological-computation" / "k_full.jsonl"),
        help="Path to k_full.jsonl",
    )
    args = parser.parse_args()

    checkpoint_path = Path(args.checkpoint)
    jsonl_path = Path(args.jsonl)

    if not checkpoint_path.exists():
        print(f"Checkpoint not found: {checkpoint_path}", file=sys.stderr)
        sys.exit(1)
    if not jsonl_path.exists():
        print(f"JSONL not found: {jsonl_path}", file=sys.stderr)
        sys.exit(1)

    # Step 1: Load graph
    print(f"Loading graph from {jsonl_path}...", file=sys.stderr)
    active_vids, active_edges = load_active_edges_from_jsonl(jsonl_path)
    print(f"  Active vertices: {len(active_vids)}", file=sys.stderr)
    print(f"  Active edges: {len(active_edges)}", file=sys.stderr)

    # Step 2: Load checkpoint
    state = json.loads(checkpoint_path.read_text(encoding="utf-8"))
    settled = state.get("settled_cycles", [])
    print(f"\nCheckpoint: {len(settled)} settled cycles, blocked_streak={state.get('blocked_streak', 0)}", file=sys.stderr)

    # Step 3: Identify ghosts
    valid_cycles = []
    ghost_cycles = []
    for sc_data in settled:
        sc_edges = frozenset(tuple(e) for e in sc_data["edges"])
        if sc_edges.issubset(active_edges):
            valid_cycles.append(sc_data)
        else:
            ghost_cycles.append(sc_data)
            missing = sc_edges - active_edges
            vids = set()
            for s, t in sc_edges:
                vids.add(s)
                vids.add(t)
            missing_vids = vids - active_vids
            print(
                f"  GHOST: settled_at={sc_data['settled_at']}, "
                f"edges={len(sc_edges)} ({len(missing)} missing), "
                f"vertices={len(vids)} ({len(missing_vids)} missing)",
                file=sys.stderr,
            )

    print(f"\nResult: {len(valid_cycles)} valid, {len(ghost_cycles)} ghosts", file=sys.stderr)

    if not ghost_cycles:
        print("No ghosts found. Nothing to do.", file=sys.stderr)
        return

    if args.dry_run:
        print(f"\n[DRY RUN] Would remove {len(ghost_cycles)} ghost settlements and reset blocked_streak.", file=sys.stderr)
        return

    # Step 4: Update checkpoint
    state["settled_cycles"] = valid_cycles
    state["blocked_streak"] = 0
    state["saved_at"] = time.time()

    # Atomic write
    tmp = checkpoint_path.with_suffix(".tmp")
    tmp.write_text(json.dumps(state, ensure_ascii=False, default=str), encoding="utf-8")
    tmp.replace(checkpoint_path)

    print(
        f"\nPurged {len(ghost_cycles)} ghost settlements. "
        f"{len(valid_cycles)} valid remain. blocked_streak reset to 0.",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
