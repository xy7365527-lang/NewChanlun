"""Launch multiple daemon instances with shared layer.

Usage:
    python swarm/swarm_launcher.py --instances 3 --shared /tmp/swarm --steps 200
    python swarm/swarm_launcher.py --instances 2 --seeds "philosophy,code" --shared /tmp/swarm --steps 500
    python swarm/swarm_launcher.py --instances 2 --hegel --shared /tmp/swarm --steps 200
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import time
from pathlib import Path


def launch(
    n_instances: int,
    shared_dir: str,
    steps: int = 200,
    sync_interval: int = 20,
    seeds: list[str] | None = None,
    hegel: bool = False,
    load: str | None = None,
) -> list[subprocess.Popen]:
    """Launch N daemon instances, each as a subprocess.

    Returns the list of Popen objects.
    """
    script_dir = os.path.dirname(os.path.abspath(__file__))
    daemon_script = os.path.join(script_dir, "swarm_daemon.py")

    processes: list[subprocess.Popen] = []
    for i in range(n_instances):
        instance_id = f"inst_{i}"
        cmd = [
            sys.executable, daemon_script,
            f"--instance-id={instance_id}",
            f"--shared={shared_dir}",
            f"--steps={steps}",
            f"--sync-interval={sync_interval}",
        ]

        if seeds and i < len(seeds):
            cmd.append(f"--seed={seeds[i]}")
        elif load:
            cmd.append(f"--load={load}")
        elif hegel:
            cmd.append("--hegel")

        output_path = os.path.join(shared_dir, f"{instance_id}_report.txt")
        cmd.append(f"--output={output_path}")

        print(f"Launching {instance_id}: {' '.join(cmd)}", file=sys.stderr)
        p = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        processes.append(p)

    return processes


def wait_all(processes: list[subprocess.Popen], timeout: float = 600.0) -> list[dict]:
    """Wait for all processes to complete. Returns list of result dicts."""
    results: list[dict] = []
    deadline = time.monotonic() + timeout

    for i, p in enumerate(processes):
        remaining = max(0, deadline - time.monotonic())
        try:
            stdout, stderr = p.communicate(timeout=remaining)
            results.append({
                "instance": f"inst_{i}",
                "returncode": p.returncode,
                "stdout": stdout.decode("utf-8", errors="replace"),
                "stderr": stderr.decode("utf-8", errors="replace"),
            })
        except subprocess.TimeoutExpired:
            p.kill()
            stdout, stderr = p.communicate()
            results.append({
                "instance": f"inst_{i}",
                "returncode": -1,
                "stdout": stdout.decode("utf-8", errors="replace"),
                "stderr": stderr.decode("utf-8", errors="replace"),
                "error": "timeout",
            })

    return results


def main() -> None:
    parser = argparse.ArgumentParser(description="SwarmLauncher — launch multiple daemon instances")
    parser.add_argument("--instances", type=int, default=2, help="Number of instances")
    parser.add_argument("--shared", type=str, required=True, help="Shared directory")
    parser.add_argument("--steps", type=int, default=200, help="Steps per instance")
    parser.add_argument("--sync-interval", type=int, default=20, help="Sync interval")
    parser.add_argument("--seeds", type=str, help="Comma-separated seed text files")
    parser.add_argument("--hegel", action="store_true", help="Use Hegel Phenomenology chapters")
    parser.add_argument("--load", type=str, help="Load graph from JSON file")
    parser.add_argument("--timeout", type=float, default=600.0, help="Timeout in seconds")
    args = parser.parse_args()

    # Ensure shared directory exists
    Path(args.shared).mkdir(parents=True, exist_ok=True)

    seed_files = args.seeds.split(",") if args.seeds else None

    processes = launch(
        n_instances=args.instances,
        shared_dir=args.shared,
        steps=args.steps,
        sync_interval=args.sync_interval,
        seeds=seed_files,
        hegel=args.hegel,
        load=args.load,
    )

    print(f"Launched {len(processes)} instances. Waiting for completion...", file=sys.stderr)

    results = wait_all(processes, timeout=args.timeout)

    print("\n" + "=" * 70)
    print("SWARM LAUNCH RESULTS")
    print("=" * 70)

    for r in results:
        print(f"\n--- {r['instance']} (exit={r['returncode']}) ---")
        if r.get("error"):
            print(f"  ERROR: {r['error']}")
        if r["stdout"].strip():
            for line in r["stdout"].strip().split("\n"):
                print(f"  {line}")

    # Print shared layer summary (from IPFS)
    try:
        from chain.ipfs_client import IPFSClient
        from swarm.shared_layer import SharedLayer
        ipfs = IPFSClient()
        if ipfs.is_available():
            shared_layer = SharedLayer(ipfs)
            block_count = len(shared_layer.all_block_hashes())
            rel_count = len(shared_layer.read_relations())
            print(f"\nShared blocks total (IPFS): {block_count}")
            print(f"Shared relations total (IPFS): {rel_count}")
        else:
            print("\nIPFS daemon 不可用——共享层统计不可获取")
    except Exception as exc:
        print(f"\n共享层统计失败: {exc}")


if __name__ == "__main__":
    main()
