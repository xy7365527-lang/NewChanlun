"""Integration test: launch 2 SwarmDaemon instances with different seeds,
shared directory, 200 steps each. Verify cross-instance block sharing and sync.

This runs in-process (no subprocess) for faster, more reliable testing.
"""

from __future__ import annotations

import os
import sys
import shutil
import tempfile
import time

# Ensure the project root is on path
project_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
sys.path.insert(0, project_dir)

from engine import compute_beta_1
from swarm.shared_layer import SharedLayer
from swarm.swarm_daemon import SwarmDaemon


def run_test() -> str:
    """Run the swarm integration test. Returns report text."""
    lines: list[str] = []
    lines.append("=" * 70)
    lines.append("SWARM INTEGRATION TEST")
    lines.append("=" * 70)
    lines.append("")

    # Create temp shared directory
    shared_dir = tempfile.mkdtemp(prefix="swarm_test_")
    lines.append(f"Shared directory: {shared_dir}")
    lines.append("")

    try:
        # Build graph from Hegel chapters (both instances use same base graph
        # but different random seeds for traversal diversity)
        from daemon import _build_graph_from_chapters
        graph_a, _ = _build_graph_from_chapters()
        graph_b, _ = _build_graph_from_chapters()

        n_verts = len(graph_a.active_vertex_ids())
        n_edges = len(graph_a.active_edges())
        beta_1_init = compute_beta_1(graph_a)
        lines.append(f"Initial graph: {n_verts} vertices, {n_edges} edges, beta_1={beta_1_init}")
        lines.append("")

        # Create two SwarmDaemon instances
        daemon_a = SwarmDaemon(
            shared_dir=shared_dir,
            instance_id="inst_A",
            sync_interval=20,
            graph=graph_a,
            settlement_threshold=15,
            seed=42,
        )

        daemon_b = SwarmDaemon(
            shared_dir=shared_dir,
            instance_id="inst_B",
            sync_interval=20,
            graph=graph_b,
            settlement_threshold=15,
            seed=137,  # Different seed for traversal diversity
        )

        steps = 200

        # Run interleaved: 20 steps A, 20 steps B, repeat
        # This simulates concurrent execution with sync points
        lines.append(f"Running {steps} steps per instance (interleaved, 20-step chunks)...")
        t0 = time.monotonic()

        chunk = 20
        for offset in range(0, steps, chunk):
            remaining = min(chunk, steps - offset)
            daemon_a.run(max_steps=remaining)
            daemon_b.run(max_steps=remaining)
            # Force sync at each boundary
            daemon_a.syncer.sync()
            daemon_b.syncer.sync()

        elapsed = time.monotonic() - t0
        lines.append(f"Elapsed: {elapsed:.2f}s")
        lines.append("")

        # Collect results
        status_a = daemon_a.swarm_status()
        status_b = daemon_b.swarm_status()

        lines.append("--- Instance A (seed=42) ---")
        lines.append(f"  Vertices: {status_a['vertices_active']}, Edges: {status_a['edges_active']}")
        lines.append(f"  beta_1: {status_a['beta_1']}, Settled: {status_a['settled_cycles']}")
        lines.append(f"  Events: {status_a['total_events']}")
        lines.append(f"  Blocks written: {status_a['blocks_written']}")
        lines.append(f"  Blocks injected: {status_a['blocks_injected']}")
        lines.append("")

        lines.append("--- Instance B (seed=137) ---")
        lines.append(f"  Vertices: {status_b['vertices_active']}, Edges: {status_b['edges_active']}")
        lines.append(f"  beta_1: {status_b['beta_1']}, Settled: {status_b['settled_cycles']}")
        lines.append(f"  Events: {status_b['total_events']}")
        lines.append(f"  Blocks written: {status_b['blocks_written']}")
        lines.append(f"  Blocks injected: {status_b['blocks_injected']}")
        lines.append("")

        # Shared layer stats
        shared = SharedLayer(shared_dir)
        all_hashes = shared.all_block_hashes()
        lines.append("--- Shared Layer ---")
        lines.append(f"  Total blocks on disk: {len(all_hashes)}")
        lines.append("")

        # Verification
        lines.append("--- Verification ---")
        checks: list[tuple[str, bool]] = []

        # 1. Both instances wrote blocks
        a_wrote = status_a["blocks_written"] > 0
        b_wrote = status_b["blocks_written"] > 0
        checks.append(("Instance A wrote blocks to shared dir", a_wrote))
        checks.append(("Instance B wrote blocks to shared dir", b_wrote))

        # 2. Blocks exist in shared directory
        blocks_exist = len(all_hashes) > 0
        checks.append(("Shared directory contains blocks", blocks_exist))

        # 3. Cross-instance sync: A reads B's blocks (or vice versa)
        a_injected = status_a["blocks_injected"] > 0
        b_injected = status_b["blocks_injected"] > 0
        cross_sync = a_injected or b_injected
        checks.append(("Cross-instance sync occurred (A injected B's blocks or vice versa)", cross_sync))

        # 4. Both instances completed 200 steps
        a_steps_ok = status_a["total_steps"] == steps
        b_steps_ok = status_b["total_steps"] == steps
        checks.append(("Instance A completed all steps", a_steps_ok))
        checks.append(("Instance B completed all steps", b_steps_ok))

        # 5. Traversal diversity: different beta_1 or different event counts
        # (different seeds should lead to different traversal paths)
        diverse = (status_a["beta_1"] != status_b["beta_1"] or
                   status_a["total_events"] != status_b["total_events"] or
                   status_a["settled_cycles"] != status_b["settled_cycles"])
        checks.append(("Traversal diversity (different outcomes from different seeds)", diverse))

        all_pass = True
        for desc, result in checks:
            status_str = "PASS" if result else "FAIL"
            if not result:
                all_pass = False
            lines.append(f"  [{status_str}] {desc}")

        lines.append("")
        lines.append("=" * 70)
        lines.append(f"OVERALL: {'ALL PASS' if all_pass else 'SOME FAILED'}")
        lines.append("=" * 70)

        daemon_a.close()
        daemon_b.close()

    finally:
        # Clean up temp directory
        shutil.rmtree(shared_dir, ignore_errors=True)

    return "\n".join(lines)


if __name__ == "__main__":
    report = run_test()
    print(report)

    # Write to tmp/swarm_test.txt
    output_path = os.path.join(project_dir, "..", "tmp", "swarm_test.txt")
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        f.write(report)
    print(f"\nReport saved to {output_path}")
