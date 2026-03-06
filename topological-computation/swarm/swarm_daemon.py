"""Swarm-aware daemon: wraps TopologicalDaemon with shared layer + cross-instance sync.

Adds:
- Shared event layer read/write
- Cross-instance sync (every N steps)
- Operation recording to shared layer

CLI:
    python swarm/swarm_daemon.py --instance-id inst_0 --seed path/to/text.txt --shared /tmp/swarm --steps 200
    python swarm/swarm_daemon.py --instance-id inst_0 --hegel --shared /tmp/swarm --steps 500
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

from engine import Graph, compute_beta_1
from daemon import TopologicalDaemon, graph_from_dict, _build_graph_from_chapters, format_event
from traversal import StepLog
from swarm.shared_layer import SharedLayer
from swarm.cross_instance import CrossInstanceSync


class SwarmDaemon(TopologicalDaemon):
    """TopologicalDaemon extended with shared layer and cross-instance sync."""

    def __init__(
        self,
        shared_dir: str | Path,
        instance_id: str,
        sync_interval: int = 20,
        graph: Graph | None = None,
        seed_text: str | None = None,
        settlement_threshold: int = 15,
        seed: int = 42,
    ):
        super().__init__(
            graph=graph,
            seed_text=seed_text,
            settlement_threshold=settlement_threshold,
            seed=seed,
        )
        self.instance_id = instance_id
        self.sync_interval = sync_interval

        # Shared layer
        self.shared = SharedLayer(shared_dir)
        self.syncer = CrossInstanceSync(self.shared, instance_id, self)

        # Initialize known blocks with whatever is already on disk
        self.syncer.known_blocks = self.shared.all_block_hashes()

        # Track blocks written by this instance
        self._blocks_written: int = 0

    def _step(self) -> None:
        """Override: after each step, record significant events and periodically sync."""
        # Capture pre-step state for block recording
        beta_before = compute_beta_1(self.k_active) if self.k_active.active_vertex_ids() else 0

        super()._step()

        # Record significant operations to shared layer
        if self.total_steps > 0 and self.event_log:
            latest = self.event_log[-1] if self.event_log else None
            if latest and self.total_steps == len(self.event_log) + (self.total_steps - self.total_events):
                # Write block for significant events (beta_1 change or blocked)
                self._write_event_block()

        # Cross-instance sync
        if self.total_steps % self.sync_interval == 0:
            self.syncer.sync()

    def _write_event_block(self) -> None:
        """Write current graph snapshot as a content-addressed block."""
        # Only write if there was a significant event on this step
        if not self.engine or not self.engine.logs:
            return
        last_log = self.engine.logs[-1]
        if last_log.delta_beta_1 == 0 and not last_log.blocked:
            return

        # Build block content: the new vertices and edges from this step
        new_vertices = []
        new_edges = []
        for vid, v in self.k_active.vertices.items():
            if v.created_at == self.total_steps:
                new_vertices.append({
                    "id": v.id,
                    "status": v.status.value,
                    "content": v.content,
                    "created_at": v.created_at,
                })
        for e in self.k_active.edges:
            if e.created_at == self.total_steps:
                new_edges.append({
                    "source": e.source,
                    "target": e.target,
                    "edge_type": e.edge_type.value,
                    "created_at": e.created_at,
                })

        if not new_vertices and not new_edges:
            return

        block = {
            "instance": self.instance_id,
            "step": self.total_steps,
            "operation": last_log.operation,
            "beta_1_before": last_log.beta_1_before,
            "beta_1_after": last_log.beta_1_after,
            "vertices": new_vertices,
            "edges": new_edges,
        }
        block_hash = self.shared.write_block(block)
        self.syncer.known_blocks.add(block_hash)
        self._blocks_written += 1

    def swarm_status(self) -> dict:
        """Extended status with swarm-specific info."""
        base = self.status()
        base["instance_id"] = self.instance_id
        base["blocks_written"] = self._blocks_written
        base["blocks_injected"] = self.syncer.injected_count
        base["known_blocks"] = len(self.syncer.known_blocks)
        return base


def main() -> None:
    parser = argparse.ArgumentParser(description="SwarmDaemon — swarm-aware topological entity")
    parser.add_argument("--instance-id", type=str, required=True, help="Unique instance identifier")
    parser.add_argument("--seed", type=str, help="Seed text file for phi_L processing")
    parser.add_argument("--shared", type=str, required=True, help="Shared directory for cross-instance communication")
    parser.add_argument("--steps", type=int, default=200, help="Number of steps to run")
    parser.add_argument("--sync-interval", type=int, default=20, help="Sync with shared layer every N steps")
    parser.add_argument("--hegel", action="store_true", help="Build from Hegel Phenomenology chapters")
    parser.add_argument("--load", type=str, help="Load graph from JSON file")
    parser.add_argument("--output", type=str, help="Output file for results")
    args = parser.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    parent_dir = os.path.dirname(script_dir)
    os.chdir(parent_dir)

    # Build graph
    graph = None
    if args.load:
        load_path = args.load
        if not os.path.isabs(load_path):
            load_path = os.path.join(parent_dir, load_path)
        with open(load_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        if "graph_data" in data:
            graph = graph_from_dict(data["graph_data"])
        else:
            graph = graph_from_dict(data)
    elif args.seed:
        seed_path = args.seed
        if not os.path.isabs(seed_path):
            seed_path = os.path.join(parent_dir, seed_path)
        with open(seed_path, "r", encoding="utf-8") as f:
            text = f.read()
        from phi_L import phi_L
        graph = phi_L(text)
    elif args.hegel:
        graph, _ = _build_graph_from_chapters()
    else:
        graph, _ = _build_graph_from_chapters()

    # Use instance_id hash as random seed for diversity
    instance_seed = hash(args.instance_id) % (2**31)

    daemon = SwarmDaemon(
        shared_dir=args.shared,
        instance_id=args.instance_id,
        sync_interval=args.sync_interval,
        graph=graph,
        settlement_threshold=15,
        seed=instance_seed,
    )

    # Event logging
    def on_event(log: StepLog, narrative: str) -> None:
        print(f"[{args.instance_id}] {narrative}", file=sys.stderr)

    daemon.register_callback("on_event", on_event)

    n_verts = len(daemon.k_active.active_vertex_ids())
    n_edges = len(daemon.k_active.active_edges())
    beta_1 = compute_beta_1(daemon.k_active)
    print(f"[{args.instance_id}] Starting: {n_verts}V {n_edges}E beta_1={beta_1}, {args.steps} steps", file=sys.stderr)

    t0 = time.monotonic()
    daemon.run(max_steps=args.steps)
    elapsed = time.monotonic() - t0

    status = daemon.swarm_status()

    report_lines = [
        f"=== SwarmDaemon Report: {args.instance_id} ===",
        f"Steps: {args.steps}, Time: {elapsed:.2f}s",
        f"Vertices: {status['vertices_active']}, Edges: {status['edges_active']}",
        f"beta_1: {status['beta_1']}, Settled: {status['settled_cycles']}",
        f"Events: {status['total_events']}, Gaps: {status['total_gaps_detected']}",
        f"Blocks written: {status['blocks_written']}",
        f"Blocks injected: {status['blocks_injected']}",
        f"Known blocks: {status['known_blocks']}",
    ]
    report = "\n".join(report_lines)
    print(report)

    if args.output:
        out_dir = os.path.dirname(args.output)
        if out_dir:
            os.makedirs(out_dir, exist_ok=True)
        with open(args.output, "w", encoding="utf-8") as f:
            f.write(report + "\n")
            f.write("\n--- Status JSON ---\n")
            f.write(json.dumps(status, indent=2, ensure_ascii=False) + "\n")

    daemon.close()


if __name__ == "__main__":
    main()
