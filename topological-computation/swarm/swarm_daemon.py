"""Swarm-aware daemon: wraps TopologicalDaemon with shared layer + cross-instance sync.

Adds:
- Shared event layer read/write
- Cross-instance sync (every N steps)
- Operation recording to shared layer
- PID file management (~/.swarm/daemon.pid)
- File logging (~/.swarm/output/daemon.log)
- IPFS background upload thread (graceful degradation)
- HTTP/WS API server (--serve mode)
- Crash retry wrapper
- Persistence enabled by default

CLI:
    python swarm/swarm_daemon.py --instance-id inst_0 --seed path/to/text.txt --shared /tmp/swarm
    python swarm/swarm_daemon.py --instance-id inst_0 --hegel --shared /tmp/swarm
    python swarm/swarm_daemon.py --instance-id node0 --hegel --shared ~/.swarm --persist
    python swarm/swarm_daemon.py --instance-id node0 --hegel --shared ~/.swarm --serve --port 8080 --ws-port 8765
"""

from __future__ import annotations

import argparse
import json
import logging
import os
import signal
import sys
import threading
import time
from pathlib import Path
from queue import Queue, Empty

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

from engine import Graph, compute_beta_1
from daemon import TopologicalDaemon, graph_from_dict, _build_graph_from_chapters, format_event
from persistence import DEFAULT_PATH as PERSIST_DEFAULT_PATH
from traversal import StepLog
from swarm.shared_layer import SharedLayer
from swarm.cross_instance import CrossInstanceSync


# ---------------------------------------------------------------------------
# Logging setup
# ---------------------------------------------------------------------------

def _setup_logging(log_path: str | Path, instance_id: str) -> logging.Logger:
    """Create logger that writes to file and stderr."""
    logger = logging.getLogger(f"swarm.{instance_id}")
    logger.setLevel(logging.INFO)

    formatter = logging.Formatter(
        f"[%(asctime)s] [{instance_id}] %(levelname)s: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    )

    # File handler
    log_dir = Path(log_path).parent
    log_dir.mkdir(parents=True, exist_ok=True)
    fh = logging.FileHandler(str(log_path), encoding="utf-8")
    fh.setLevel(logging.INFO)
    fh.setFormatter(formatter)
    logger.addHandler(fh)

    # Stderr handler
    sh = logging.StreamHandler(sys.stderr)
    sh.setLevel(logging.INFO)
    sh.setFormatter(formatter)
    logger.addHandler(sh)

    return logger


# ---------------------------------------------------------------------------
# PID file management
# ---------------------------------------------------------------------------

def _write_pid(pid_path: str | Path) -> None:
    """Write current PID to file."""
    path = Path(pid_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(str(os.getpid()), encoding="utf-8")


def _remove_pid(pid_path: str | Path) -> None:
    """Remove PID file if it exists."""
    path = Path(pid_path)
    if path.exists():
        path.unlink()


def _check_running(pid_path: str | Path) -> int | None:
    """Check if daemon is already running. Returns PID or None."""
    path = Path(pid_path)
    if not path.exists():
        return None
    try:
        pid = int(path.read_text(encoding="utf-8").strip())
        # Check if process exists (cross-platform)
        os.kill(pid, 0)
        return pid
    except (ValueError, OSError):
        # Stale PID file
        return None


# ---------------------------------------------------------------------------
# IPFS background uploader
# ---------------------------------------------------------------------------

class IPFSUploader:
    """Background thread that uploads blocks to IPFS when available."""

    def __init__(self, logger: logging.Logger):
        self._queue: Queue[tuple[str, dict]] = Queue()
        self._thread: threading.Thread | None = None
        self._stop = threading.Event()
        self._logger = logger
        self._client = None
        self._uploaded: int = 0

    def start(self) -> None:
        """Start the upload thread. Attempts to connect to IPFS."""
        try:
            from chain.ipfs_client import IPFSClient
            client = IPFSClient()
            if client.is_available():
                self._client = client
                self._logger.info("IPFS daemon detected — background upload enabled")
            else:
                self._logger.info("IPFS daemon not available — uploads disabled, local persistence only")
        except ImportError:
            self._logger.info("IPFS client not available — uploads disabled")

        self._thread = threading.Thread(target=self._run, daemon=True, name="ipfs-uploader")
        self._thread.start()

    def enqueue(self, block_hash: str, block_data: dict) -> None:
        """Add a block to the upload queue."""
        self._queue.put((block_hash, block_data))

    def stop(self) -> None:
        """Signal the upload thread to stop."""
        self._stop.set()
        if self._thread:
            self._thread.join(timeout=5)

    @property
    def uploaded_count(self) -> int:
        return self._uploaded

    def _run(self) -> None:
        """Upload loop: drain queue, upload to IPFS."""
        while not self._stop.is_set():
            try:
                block_hash, block_data = self._queue.get(timeout=1)
            except Empty:
                continue

            if self._client and self._client.is_available():
                try:
                    data = json.dumps(block_data, sort_keys=True, ensure_ascii=False)
                    cid = self._client.upload(data)
                    self._client.pin(cid)
                    self._uploaded += 1
                    self._logger.info(f"IPFS uploaded block {block_hash[:12]}... -> {cid}")
                except Exception as exc:
                    self._logger.warning(f"IPFS upload failed for {block_hash[:12]}...: {exc}")


# ---------------------------------------------------------------------------
# SwarmDaemon
# ---------------------------------------------------------------------------

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
        persist_path: str | Path | None = None,
        logger: logging.Logger | None = None,
        ipfs_uploader: IPFSUploader | None = None,
    ):
        super().__init__(
            graph=graph,
            seed_text=seed_text,
            settlement_threshold=settlement_threshold,
            seed=seed,
            persist_path=persist_path,
        )
        self.instance_id = instance_id
        self.sync_interval = sync_interval
        self._logger = logger

        # Override checkpoint instance_id to use swarm's instance_id
        self._checkpoint.close()
        from traversal_checkpoint import TraversalCheckpoint, restore_daemon_state
        self._checkpoint = TraversalCheckpoint(instance_id=instance_id)

        # If checkpoint exists, restore state
        saved = self._checkpoint.load_state()
        if saved:
            restore_daemon_state(self, saved)
            if self._logger:
                self._logger.info(f"Restored: step={self.total_steps}, settled={len(self.settlement.settled_cycles)}")

        # Shared layer
        self.shared = SharedLayer(shared_dir)
        self.syncer = CrossInstanceSync(self.shared, instance_id, self)

        # Initialize known blocks with whatever is already on disk
        self.syncer.known_blocks = self.shared.all_block_hashes()

        # Track blocks written by this instance
        self._blocks_written: int = 0

        # IPFS background uploader
        self._ipfs_uploader = ipfs_uploader

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

        # Enqueue for IPFS upload if available
        if self._ipfs_uploader:
            self._ipfs_uploader.enqueue(block_hash, block)

    def swarm_status(self) -> dict:
        """Extended status with swarm-specific info."""
        base = self.status()
        base["instance_id"] = self.instance_id
        base["blocks_written"] = self._blocks_written
        base["blocks_injected"] = self.syncer.injected_count
        base["known_blocks"] = len(self.syncer.known_blocks)
        if self._ipfs_uploader:
            base["ipfs_uploaded"] = self._ipfs_uploader.uploaded_count
        return base


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def _build_graph(args, parent_dir: str) -> Graph | None:
    """Build graph from CLI arguments."""
    if args.load:
        load_path = args.load
        if not os.path.isabs(load_path):
            load_path = os.path.join(parent_dir, load_path)
        with open(load_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        if "graph_data" in data:
            return graph_from_dict(data["graph_data"])
        return graph_from_dict(data)
    elif args.seed:
        seed_path = args.seed
        if not os.path.isabs(seed_path):
            seed_path = os.path.join(parent_dir, seed_path)
        with open(seed_path, "r", encoding="utf-8") as f:
            text = f.read()
        from phi_L import phi_L
        return phi_L(text)
    else:
        # Default: Hegel Phenomenology
        graph, _ = _build_graph_from_chapters()
        return graph


def _run_once(args, parent_dir: str, logger: logging.Logger) -> dict:
    """Run one daemon session. Returns status dict."""
    shared_dir = args.shared
    if not os.path.isabs(shared_dir):
        shared_dir = os.path.join(parent_dir, shared_dir)

    # Build graph
    graph = _build_graph(args, parent_dir)

    # PID file
    pid_path = Path(shared_dir) / "daemon.pid"
    existing_pid = _check_running(pid_path)
    if existing_pid:
        logger.warning(f"Daemon already running (PID {existing_pid}), proceeding anyway")
    _write_pid(pid_path)

    # IPFS uploader
    ipfs_uploader = IPFSUploader(logger)
    ipfs_uploader.start()

    # Persistence path — each instance gets its own file to avoid conflicts
    persist_path = None
    if args.persist:
        if isinstance(args.persist, str) and args.persist != "True" and args.persist != str(PERSIST_DEFAULT_PATH):
            # User specified an explicit path
            persist_path = args.persist
        else:
            # Default: instance-specific path under shared directory
            persist_path = str(Path(shared_dir) / "persist" / f"{args.instance_id}.jsonl")

    # Use instance_id hash as random seed for diversity
    instance_seed = hash(args.instance_id) % (2**31)

    daemon = SwarmDaemon(
        shared_dir=shared_dir,
        instance_id=args.instance_id,
        sync_interval=args.sync_interval,
        graph=graph,
        settlement_threshold=15,
        seed=instance_seed,
        persist_path=persist_path,
        logger=logger,
        ipfs_uploader=ipfs_uploader,
    )

    # If persisting, ensure initial graph is baselined in persistence file
    # Covers: fresh file OR recovered graph smaller than loaded graph
    if persist_path and daemon._persist and graph is not None:
        from persistence import PersistentKFull
        recovered, _ = PersistentKFull.load(persist_path)
        recovered_size = len(recovered.active_vertex_ids())
        loaded_size = len(graph.active_vertex_ids())
        if recovered_size == 0 or loaded_size > recovered_size * 1.5:
            daemon._persist.close()
            with open(persist_path, "w", encoding="utf-8") as _:
                pass  # truncate
            daemon._persist = PersistentKFull(persist_path)
            daemon._persist.open()
            for vid, v in graph.vertices.items():
                daemon._persist.append_vertex(v)
            for e in graph.edges:
                daemon._persist.append_edge(e)

    # Event logging
    def on_event(log: StepLog, narrative: str) -> None:
        logger.info(narrative)

    daemon.register_callback("on_event", on_event)

    # Multiprocess mode: serve HTTP in main process, traversal in subprocess
    # This eliminates GIL contention — HTTP responds instantly even during
    # heavy terrain computation (15K graph, 0.5-2s per step)
    if args.serve and getattr(args, 'multiproc', False):
        logger.info("Multiprocess mode: traversal in subprocess, HTTP/WS in main process")

        # Serialize graph to JSON for subprocess
        graph_json = None
        if graph is not None:
            from daemon import graph_to_dict
            graph_json = graph_to_dict(graph)

        from daemon_multiproc import start_multiprocess_daemon
        start_multiprocess_daemon(
            graph_json=graph_json,
            port=args.port,
            ws_port=args.ws_port,
            persist_path=persist_path,
            settlement_threshold=15,
            seed=instance_seed,
            autonomous=args.autonomous,
        )

        # start_multiprocess_daemon blocks forever, cleanup on return
        ipfs_uploader.stop()
        _remove_pid(pid_path)
        return {"mode": "multiproc", "instance_id": args.instance_id}

    # HTTP/WS API server (--serve mode, single-process with threads)
    http_server = None
    if args.serve:
        from daemon_server import start_http_server, start_ws_server, bridge_daemon_to_ws
        http_server = start_http_server(daemon, args.port)
        start_ws_server(args.ws_port)
        bridge_daemon_to_ws(daemon)
        logger.info(f"API server started: HTTP={args.port}, WS={args.ws_port}")

    # Autonomous mode: auto-feed on gap detection
    if args.autonomous:
        from auto_feed import feed_from_gap
        from daemon import GapInfo

        def on_gap_feed(gap: GapInfo) -> None:
            record = feed_from_gap(gap, daemon)
            logger.info(
                f"[auto-feed] '{gap.content}' -> {record.verdict} "
                f"(source={record.source}, accepted={record.accepted})"
            )

        daemon.register_callback("on_gap", on_gap_feed)

    n_verts = len(daemon.k_active.active_vertex_ids())
    n_edges = len(daemon.k_active.active_edges())
    beta_1 = compute_beta_1(daemon.k_active)
    logger.info(f"Starting: {n_verts}V {n_edges}E beta_1={beta_1}, self-driven (no step limit)")

    t0 = time.monotonic()
    daemon.run()
    elapsed = time.monotonic() - t0

    status = daemon.swarm_status()
    status["elapsed_seconds"] = round(elapsed, 2)

    report_lines = [
        f"=== SwarmDaemon Report: {args.instance_id} ===",
        f"Steps: {status['total_steps']}, Time: {elapsed:.2f}s",
        f"Vertices: {status['vertices_active']}, Edges: {status['edges_active']}",
        f"beta_1: {status['beta_1']}, Settled: {status['settled_cycles']}",
        f"Events: {status['total_events']}, Gaps: {status['total_gaps_detected']}",
        f"Blocks written: {status['blocks_written']}",
        f"Blocks injected: {status['blocks_injected']}",
        f"Known blocks: {status['known_blocks']}",
    ]
    if "ipfs_uploaded" in status:
        report_lines.append(f"IPFS uploaded: {status['ipfs_uploaded']}")

    report = "\n".join(report_lines)
    logger.info(report)
    print(report)

    if args.output:
        out_dir = os.path.dirname(args.output)
        if out_dir:
            os.makedirs(out_dir, exist_ok=True)
        with open(args.output, "w", encoding="utf-8") as f:
            f.write(report + "\n")
            f.write("\n--- Status JSON ---\n")
            f.write(json.dumps(status, indent=2, ensure_ascii=False) + "\n")

    # Cleanup
    ipfs_uploader.stop()
    if http_server:
        http_server.shutdown()
    daemon.close()
    _remove_pid(pid_path)

    return status


def main() -> None:
    parser = argparse.ArgumentParser(description="SwarmDaemon — swarm-aware topological entity")
    parser.add_argument("--instance-id", type=str, required=True, help="Unique instance identifier")
    parser.add_argument("--seed", type=str, help="Seed text file for phi_L processing")
    parser.add_argument("--shared", type=str, required=True, help="Shared directory for cross-instance communication")
    parser.add_argument("--sync-interval", type=int, default=20, help="Sync with shared layer every N steps")
    parser.add_argument("--hegel", action="store_true", help="Build from Hegel Phenomenology chapters")
    parser.add_argument("--load", type=str, help="Load graph from JSON file")
    parser.add_argument("--output", type=str, help="Output file for results")
    parser.add_argument("--persist", type=str, nargs="?", const=str(PERSIST_DEFAULT_PATH),
                        help="Enable JSONL persistence (default on)")
    parser.add_argument("--serve", action="store_true",
                        help="Start HTTP/WS API server for Dashboard")
    parser.add_argument("--port", type=int, default=8080,
                        help="HTTP API port (with --serve)")
    parser.add_argument("--ws-port", type=int, default=8765,
                        help="WebSocket port (with --serve)")
    parser.add_argument("--autonomous", action="store_true",
                        help="Enable autonomous gap detection + auto-feed")
    parser.add_argument("--multiproc", action="store_true",
                        help="Multiprocess mode: traversal in subprocess, HTTP/WS in main (solves GIL blocking)")
    parser.add_argument("--max-retries", type=int, default=3,
                        help="Maximum crash retries (0 = no retry)")
    parser.add_argument("--retry-delay", type=int, default=5,
                        help="Seconds between retries")

    args = parser.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    parent_dir = os.path.dirname(script_dir)
    os.chdir(parent_dir)

    # Setup logging
    shared_dir = args.shared
    if not os.path.isabs(shared_dir):
        shared_dir = os.path.join(parent_dir, shared_dir)
    log_path = Path(shared_dir) / "output" / "daemon.log"
    logger = _setup_logging(log_path, args.instance_id)

    # Crash retry wrapper
    retries = 0
    while True:
        try:
            _run_once(args, parent_dir, logger)
            break  # Clean exit
        except KeyboardInterrupt:
            logger.info("Interrupted by user")
            break
        except Exception as exc:
            retries += 1
            logger.error(f"Daemon crashed: {exc}", exc_info=True)
            if retries > args.max_retries:
                logger.error(f"Max retries ({args.max_retries}) exceeded, giving up")
                sys.exit(1)
            logger.info(f"Retrying in {args.retry_delay}s (attempt {retries}/{args.max_retries})")
            time.sleep(args.retry_delay)


if __name__ == "__main__":
    main()
