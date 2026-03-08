#!/usr/bin/env python3
"""start_fengliang.py -- launch the FengLiang RTAS swarm.

Starts N SwarmDaemon instances as subprocesses:
  - Instance 0: --serve (exposes HTTP/WS API for the Dashboard)
  - Instance 1..N-1: pure traversal (no API server)
  - All instances share ~/.swarm/ directory for cross-instance sync
  - IPFS background upload enabled on all instances

Usage:
    python start_fengliang.py                     # 3 instances, default
    python start_fengliang.py --instances 5       # 5 instances
    python start_fengliang.py --port 9090         # custom API port
    python start_fengliang.py --hegel             # Hegel seed (default)
    python start_fengliang.py --load graph.json   # custom seed graph
    python start_fengliang.py --seed text.txt     # phi_L seed

Ctrl+C to stop all instances.
"""

from __future__ import annotations

import argparse
import os
import signal
import subprocess
import sys
import time
from pathlib import Path


# Default shared directory
DEFAULT_SHARED = str(Path.home() / ".swarm")

# Instance seeds: different hash seeds for traversal diversity
INSTANCE_SEEDS = [42, 137, 271, 409, 547, 683, 821, 953, 1087, 1223]


def _build_cmd(
    instance_id: str,
    shared_dir: str,
    daemon_script: str,
    serve: bool = False,
    port: int = 8080,
    ws_port: int = 8765,
    autonomous: bool = False,
    hegel: bool = False,
    load: str | None = None,
    seed: str | None = None,
    persist: bool = True,
    multiproc: bool = False,
) -> list[str]:
    """Build the command line for a single SwarmDaemon instance."""
    cmd = [
        sys.executable, daemon_script,
        f"--instance-id={instance_id}",
        f"--shared={shared_dir}",
        "--sync-interval=20",
    ]

    if serve:
        cmd.append("--serve")
        cmd.append(f"--port={port}")
        cmd.append(f"--ws-port={ws_port}")
        if multiproc:
            cmd.append("--multiproc")

    if autonomous:
        cmd.append("--autonomous")

    if seed:
        cmd.append(f"--seed={seed}")
    elif load:
        cmd.append(f"--load={load}")
    elif hegel:
        cmd.append("--hegel")
    # default: swarm_daemon defaults to Hegel if no seed/load specified

    if persist:
        cmd.append("--persist")

    output_path = os.path.join(shared_dir, "output", f"{instance_id}_report.txt")
    cmd.append(f"--output={output_path}")

    return cmd


def launch_swarm(
    n_instances: int = 3,
    shared_dir: str = DEFAULT_SHARED,
    port: int = 8080,
    ws_port: int = 8765,
    autonomous: bool = False,
    hegel: bool = False,
    load: str | None = None,
    seed: str | None = None,
    multiproc: bool = False,
) -> list[subprocess.Popen]:
    """Launch N SwarmDaemon instances.

    Instance 0 gets --serve (HTTP/WS API).
    If multiproc=True, instance 0 uses multiprocess mode (GIL-free HTTP).
    All others are pure traversal workers.
    """
    script_dir = os.path.dirname(os.path.abspath(__file__))
    daemon_script = os.path.join(script_dir, "swarm", "swarm_daemon.py")

    # Ensure shared directory structure exists (output/persist for logs, blocks on IPFS)
    Path(shared_dir).mkdir(parents=True, exist_ok=True)
    (Path(shared_dir) / "output").mkdir(parents=True, exist_ok=True)

    processes: list[subprocess.Popen] = []

    for i in range(n_instances):
        instance_id = f"fengliang_{i}"
        is_api_instance = (i == 0)

        cmd = _build_cmd(
            instance_id=instance_id,
            shared_dir=shared_dir,
            daemon_script=daemon_script,
            serve=is_api_instance,
            port=port,
            ws_port=ws_port,
            autonomous=autonomous,
            hegel=hegel,
            load=load,
            seed=seed,
            multiproc=multiproc and is_api_instance,
        )

        # Log file per instance
        log_path = Path(shared_dir) / "output" / f"{instance_id}.log"

        print(f"  [{instance_id}] {'API+traverse' if is_api_instance else 'traverse'}", file=sys.stderr)
        if is_api_instance:
            print(f"    HTTP: http://localhost:{port}", file=sys.stderr)
            print(f"    WS:   ws://localhost:{ws_port}/ws", file=sys.stderr)

        log_file = open(str(log_path), "w", encoding="utf-8")
        p = subprocess.Popen(
            cmd,
            stdout=log_file,
            stderr=subprocess.STDOUT,
            # On Windows, CREATE_NEW_PROCESS_GROUP allows Ctrl+C propagation
        )
        # Store log_file handle on the Popen object for cleanup
        p._log_file = log_file  # type: ignore[attr-defined]
        processes.append(p)

    return processes


def _check_ipfs() -> bool:
    """Check if IPFS daemon is reachable."""
    try:
        import urllib.request
        req = urllib.request.Request("http://localhost:5001/api/v0/id", method="POST")
        with urllib.request.urlopen(req, timeout=3) as resp:
            resp.read()
        return True
    except Exception:
        return False


def main() -> None:
    parser = argparse.ArgumentParser(
        description="start_fengliang.py -- launch the FengLiang RTAS swarm"
    )
    parser.add_argument("--instances", type=int, default=3,
                        help="Number of SwarmDaemon instances (default: 3)")
    parser.add_argument("--shared", type=str, default=DEFAULT_SHARED,
                        help=f"Shared directory (default: {DEFAULT_SHARED})")
    parser.add_argument("--port", type=int, default=8080,
                        help="HTTP API port for instance 0 (default: 8080)")
    parser.add_argument("--ws-port", type=int, default=8765,
                        help="WebSocket port for instance 0 (default: 8765)")
    parser.add_argument("--autonomous", action="store_true",
                        help="Enable autonomous gap detection + auto-feed")
    parser.add_argument("--hegel", action="store_true",
                        help="Use Hegel Phenomenology chapters as seed")
    parser.add_argument("--load", type=str,
                        help="Load graph from JSON file")
    parser.add_argument("--seed", type=str,
                        help="Seed text file for phi_L processing")
    parser.add_argument("--multiproc", action="store_true",
                        help="Use multiprocess mode for API instance (solves GIL blocking on large graphs)")
    args = parser.parse_args()

    print("=" * 60, file=sys.stderr)
    print("FENGLIANG RTAS SWARM", file=sys.stderr)
    print("=" * 60, file=sys.stderr)
    print(f"  Instances: {args.instances}", file=sys.stderr)
    print(f"  Shared:    {args.shared}", file=sys.stderr)

    # Check IPFS
    ipfs_ok = _check_ipfs()
    print(f"  IPFS:      {'connected' if ipfs_ok else 'not available (local only)'}", file=sys.stderr)
    print("", file=sys.stderr)

    # Launch
    processes = launch_swarm(
        n_instances=args.instances,
        shared_dir=args.shared,
        port=args.port,
        ws_port=args.ws_port,
        autonomous=args.autonomous,
        hegel=args.hegel,
        load=args.load,
        seed=args.seed,
        multiproc=args.multiproc,
    )

    print("", file=sys.stderr)
    print(f"  {len(processes)} instances launched. Ctrl+C to stop all.", file=sys.stderr)
    print(f"  Logs: {args.shared}/output/", file=sys.stderr)
    print("=" * 60, file=sys.stderr)

    # Wait for all processes, handle Ctrl+C
    try:
        while True:
            # Check if any process has died
            all_alive = True
            for i, p in enumerate(processes):
                ret = p.poll()
                if ret is not None:
                    print(f"  [fengliang_{i}] exited with code {ret}", file=sys.stderr)
                    all_alive = False

            if not all_alive:
                # Check if ALL have exited
                if all(p.poll() is not None for p in processes):
                    print("  All instances exited.", file=sys.stderr)
                    break

            time.sleep(2)

    except KeyboardInterrupt:
        print("\n  Shutting down all instances...", file=sys.stderr)
        for p in processes:
            if p.poll() is None:
                p.terminate()

        # Wait for graceful shutdown
        for p in processes:
            try:
                p.wait(timeout=10)
            except subprocess.TimeoutExpired:
                p.kill()

    finally:
        # Close log file handles
        for p in processes:
            log_file = getattr(p, '_log_file', None)
            if log_file:
                log_file.close()

    # Print final status
    print("", file=sys.stderr)
    print("--- Final Status ---", file=sys.stderr)
    for i, p in enumerate(processes):
        ret = p.returncode if p.returncode is not None else "?"
        print(f"  [fengliang_{i}] exit={ret}", file=sys.stderr)

    # Print shared layer summary (from IPFS)
    try:
        from chain.ipfs_client import IPFSClient
        from swarm.shared_layer import SharedLayer
        ipfs = IPFSClient()
        if ipfs.is_available():
            shared_layer = SharedLayer(ipfs)
            block_count = len(shared_layer.all_block_hashes())
            print(f"  Shared blocks (IPFS): {block_count}", file=sys.stderr)
        else:
            print("  IPFS 不可用——共享层统计跳过", file=sys.stderr)
    except Exception:
        pass


if __name__ == "__main__":
    main()
