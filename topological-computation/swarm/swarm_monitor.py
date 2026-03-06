"""Real-time swarm monitor: block count, instance activity, beta_1 per instance.

Usage:
    python swarm/swarm_monitor.py --shared /tmp/swarm
    python swarm/swarm_monitor.py --shared /tmp/swarm --watch --interval 2
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

from swarm.shared_layer import SharedLayer


def snapshot(shared_dir: str) -> dict:
    """Take a snapshot of the swarm state from the shared directory."""
    shared = SharedLayer(shared_dir)

    # Read all blocks
    all_hashes = shared.all_block_hashes()
    blocks_by_instance: dict[str, list[dict]] = defaultdict(list)
    total_vertices = 0
    total_edges = 0

    for block_hash in all_hashes:
        block = shared.read_block(block_hash)
        if block:
            instance_id = block.get("instance", "unknown")
            blocks_by_instance[instance_id].append(block)
            total_vertices += len(block.get("vertices", []))
            total_edges += len(block.get("edges", []))

    # Read relations
    relations = shared.read_relations()
    relations_by_instance: dict[str, int] = defaultdict(int)
    for r in relations:
        relations_by_instance[r.get("instance", "unknown")] += 1

    # Per-instance stats
    instance_stats: dict[str, dict] = {}
    for inst_id, blocks in blocks_by_instance.items():
        if not blocks:
            continue
        latest = max(blocks, key=lambda b: b.get("step", 0))
        instance_stats[inst_id] = {
            "blocks": len(blocks),
            "latest_step": latest.get("step", 0),
            "latest_operation": latest.get("operation", "?"),
            "latest_beta_1": latest.get("beta_1_after", "?"),
            "relations": relations_by_instance.get(inst_id, 0),
        }

    # Read instance reports if available
    shared_path = Path(shared_dir)
    for report_file in shared_path.glob("inst_*_report.txt"):
        inst_id = report_file.stem.replace("_report", "")
        if inst_id not in instance_stats:
            instance_stats[inst_id] = {}
        content = report_file.read_text(encoding="utf-8")
        instance_stats[inst_id]["report_available"] = True
        # Extract key metrics from report
        for line in content.split("\n"):
            if line.startswith("beta_1:"):
                parts = line.split(",")
                for part in parts:
                    part = part.strip()
                    if part.startswith("beta_1:"):
                        instance_stats[inst_id]["final_beta_1"] = part.split(":")[1].strip()
                    elif part.startswith("Settled:"):
                        instance_stats[inst_id]["final_settled"] = part.split(":")[1].strip()

    return {
        "total_blocks": len(all_hashes),
        "total_vertices_in_blocks": total_vertices,
        "total_edges_in_blocks": total_edges,
        "total_relations": len(relations),
        "instances": instance_stats,
    }


def print_snapshot(snap: dict) -> None:
    """Pretty-print a swarm snapshot."""
    print("=" * 60)
    print("SWARM MONITOR")
    print("=" * 60)
    print(f"Total blocks: {snap['total_blocks']}")
    print(f"Total vertices in blocks: {snap['total_vertices_in_blocks']}")
    print(f"Total edges in blocks: {snap['total_edges_in_blocks']}")
    print(f"Total relations: {snap['total_relations']}")
    print()

    if snap["instances"]:
        print("--- Per-Instance ---")
        for inst_id in sorted(snap["instances"]):
            stats = snap["instances"][inst_id]
            line = f"  {inst_id}: "
            parts = []
            if "blocks" in stats:
                parts.append(f"blocks={stats['blocks']}")
            if "latest_step" in stats:
                parts.append(f"step={stats['latest_step']}")
            if "latest_beta_1" in stats:
                parts.append(f"beta_1={stats['latest_beta_1']}")
            if "latest_operation" in stats:
                parts.append(f"op={stats['latest_operation']}")
            if "final_beta_1" in stats:
                parts.append(f"final_beta_1={stats['final_beta_1']}")
            if "final_settled" in stats:
                parts.append(f"settled={stats['final_settled']}")
            print(line + ", ".join(parts))
    else:
        print("  (no instances detected)")
    print()


def main() -> None:
    parser = argparse.ArgumentParser(description="SwarmMonitor — real-time swarm status")
    parser.add_argument("--shared", type=str, required=True, help="Shared directory")
    parser.add_argument("--watch", action="store_true", help="Continuous monitoring")
    parser.add_argument("--interval", type=float, default=2.0, help="Watch interval in seconds")
    parser.add_argument("--json", action="store_true", help="Output as JSON")
    args = parser.parse_args()

    if args.watch:
        try:
            while True:
                if not args.json:
                    # Clear screen (cross-platform)
                    os.system("cls" if os.name == "nt" else "clear")
                snap = snapshot(args.shared)
                if args.json:
                    print(json.dumps(snap, indent=2, ensure_ascii=False))
                else:
                    print_snapshot(snap)
                    print(f"(refreshing every {args.interval}s, Ctrl+C to stop)")
                time.sleep(args.interval)
        except KeyboardInterrupt:
            print("\nMonitor stopped.")
    else:
        snap = snapshot(args.shared)
        if args.json:
            print(json.dumps(snap, indent=2, ensure_ascii=False))
        else:
            print_snapshot(snap)


if __name__ == "__main__":
    main()
