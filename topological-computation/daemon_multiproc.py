"""Multiprocess daemon: traversal in a subprocess, HTTP/WS in the main process.

Solves the GIL blocking problem: 15K graph terrain computation (0.5-2s per step)
monopolizes the GIL, starving the HTTP server thread. By running traversal in a
separate process, the HTTP/WS server gets its own GIL and responds instantly.

Architecture:
    Main process (HTTP/WS server)
        - Reads status_dict (Manager dict, updated by worker)
        - Reads topology_cache (Manager dict, periodically rebuilt by worker)
        - Puts feed requests into feed_queue
        - Puts present requests into present_queue, reads results from present_result_queue
        - Reads event_queue for WS broadcast

    Traversal subprocess
        - Runs daemon._step() in a tight loop
        - Updates status_dict every step
        - Rebuilds topology_cache every N steps
        - Drains feed_queue -> daemon.feed()
        - Drains present_queue -> present_json() -> present_result_queue
        - Pushes WS events into event_queue

No changes to engine.py, traversal.py, or daemon.py.
"""

from __future__ import annotations

import json
import multiprocessing
import os
import sys
import time
import traceback
from multiprocessing import Process, Queue
from multiprocessing.managers import SyncManager
from typing import Any

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


# ---------------------------------------------------------------------------
# Traversal worker (runs in subprocess)
# ---------------------------------------------------------------------------

def _traversal_worker(
    graph_json: dict,
    status_dict: dict,
    topology_cache: dict,
    feed_queue: Queue,
    present_queue: Queue,
    present_result_queue: Queue,
    event_queue: Queue,
    config: dict,
) -> None:
    """Subprocess entry point: run daemon traversal loop.

    All daemon state lives here. The main process only reads shared dicts
    and communicates via queues.
    """
    # Re-import inside subprocess (fresh interpreter)
    import os
    import sys
    import time

    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

    from daemon import TopologicalDaemon, graph_from_dict, _build_graph_from_chapters
    from daemon_api import (
        status_json as _status_json,
        topology_json as _topology_json,
        narrative_json as _narrative_json,
        gaps_json as _gaps_json,
        operations_json as _operations_json,
        persistence_json as _persistence_json,
        query_json as _query_json,
        step_ws_message,
        present_json,
        expression_pressure_ws_message,
        _count_unreported,
    )
    from engine import compute_beta_1

    # Build graph
    if graph_json:
        graph = graph_from_dict(graph_json)
    else:
        graph, _ = _build_graph_from_chapters()

    persist_path = config.get("persist_path")
    settlement_threshold = config.get("settlement_threshold", 15)
    seed = config.get("seed", 42)
    autonomous = config.get("autonomous", False)
    topology_refresh_interval = config.get("topology_refresh_interval", 50)

    daemon = TopologicalDaemon(
        graph=graph,
        settlement_threshold=settlement_threshold,
        seed=seed,
        persist_path=persist_path,
    )

    # If persisting, ensure initial graph is baselined in k_full.jsonl
    # This covers two cases:
    # 1. Fresh file (no recovery) — write all vertices/edges
    # 2. Recovered graph was smaller than loaded graph — rewrite baseline
    if persist_path and daemon._persist and graph is not None:
        from persistence import PersistentKFull
        recovered, _ = PersistentKFull.load(persist_path)
        recovered_size = len(recovered.active_vertex_ids())
        loaded_size = len(graph.active_vertex_ids())
        if recovered_size == 0 or loaded_size > recovered_size * 1.5:
            # Close existing handle, truncate file, rewrite with full graph
            daemon._persist.close()
            with open(persist_path, "w", encoding="utf-8") as _:
                pass  # truncate
            daemon._persist = PersistentKFull(persist_path)
            daemon._persist.open()
            for vid, v in graph.vertices.items():
                daemon._persist.append_vertex(v)
            for e in graph.edges:
                daemon._persist.append_edge(e)

    # Autonomous mode: auto-feed on gap detection
    if autonomous:
        from auto_feed import feed_from_gap
        from daemon import GapInfo

        def on_gap_feed(gap: GapInfo) -> None:
            try:
                feed_from_gap(gap, daemon)
            except Exception:
                pass

        daemon.register_callback("on_gap", on_gap_feed)

    # IPFS background uploader (graceful degradation)
    ipfs_uploader = None
    if config.get("ipfs", False):
        try:
            from swarm.swarm_daemon import IPFSUploader
            import logging
            _ipfs_logger = logging.getLogger("ipfs-uploader")
            _ipfs_logger.addHandler(logging.StreamHandler(sys.stderr))
            ipfs_uploader = IPFSUploader(_ipfs_logger)
            ipfs_uploader.start()
        except Exception as exc:
            print(f"[traversal-worker] IPFS init failed (graceful degradation): {exc}", file=sys.stderr)

    # Event callback: push significant events to event_queue for WS broadcast
    def on_event(log, narrative: str) -> None:
        try:
            msg = step_ws_message(log, daemon)
            event_queue.put_nowait(msg)
            # Expression pressure update
            pressure_msg = expression_pressure_ws_message(daemon)
            if pressure_msg.get("count", 0) > 0 or "text" in pressure_msg:
                event_queue.put_nowait(pressure_msg)
            # IPFS upload for significant events
            if ipfs_uploader and (log.delta_beta_1 != 0 or log.blocked):
                import json as _json
                block_data = {
                    "step": log.step,
                    "operation": log.operation,
                    "beta_1_before": log.beta_1_before,
                    "beta_1_after": log.beta_1_after,
                    "position": log.position,
                }
                block_hash = f"mp_{log.step}_{log.operation}"
                ipfs_uploader.enqueue(block_hash, block_data)
        except Exception:
            pass  # Queue full or other error, non-fatal

    def on_gap(gap) -> None:
        try:
            event_queue.put_nowait({
                "type": "gap",
                "concept": gap.content[:80],
                "degree": gap.degree,
                "avg_degree": round(gap.avg_degree, 1),
            })
        except Exception:
            pass

    def on_feed(sub_graph) -> None:
        try:
            event_queue.put_nowait({
                "type": "feed",
                "accepted": True,
                "source": "auto_feed",
                "verdict": "nutritious",
                "match_rate": 0.0,
                "new_vertices": len(sub_graph.active_vertex_ids()),
            })
        except Exception:
            pass

    daemon.register_callback("on_event", on_event)
    daemon.register_callback("on_gap", on_gap)
    daemon.register_callback("on_feed", on_feed)

    # Initial status + topology push
    _update_status(daemon, status_dict)
    _update_topology(daemon, topology_cache, _topology_json)
    _update_api_caches(daemon, status_dict, _narrative_json, _gaps_json,
                       _operations_json, _persistence_json)

    # Main loop
    step_count = 0
    if daemon.engine is None:
        # Nothing to traverse -- just wait for feed
        while True:
            _drain_feed_queue(daemon, feed_queue, event_queue)
            _drain_present_queue(daemon, present_queue, present_result_queue)
            if daemon.engine is not None:
                break
            time.sleep(0.1)

    while True:
        try:
            daemon._step()
            step_count += 1

            # Update shared status every step (cheap dict update)
            _update_status(daemon, status_dict)

            # Refresh topology cache periodically (expensive)
            if step_count % topology_refresh_interval == 0:
                _update_topology(daemon, topology_cache, _topology_json)
                _update_api_caches(daemon, status_dict, _narrative_json, _gaps_json,
                                   _operations_json, _persistence_json)

            # Drain feed queue (non-blocking)
            _drain_feed_queue(daemon, feed_queue, event_queue)

            # Drain present queue (non-blocking)
            _drain_present_queue(daemon, present_queue, present_result_queue)

            # Yield CPU briefly -- but since we're in a separate process,
            # this is just to avoid 100% CPU, not for GIL
            time.sleep(0.01)

        except KeyboardInterrupt:
            break
        except Exception as exc:
            # Log error but keep running
            print(f"[traversal-worker] error: {exc}", file=sys.stderr)
            traceback.print_exc(file=sys.stderr)
            time.sleep(1)

    daemon.close()


def _update_status(daemon, status_dict: dict) -> None:
    """Push daemon status into the shared dict."""
    try:
        s = daemon.status()
        # Write all keys atomically via update
        status_dict["total_steps"] = s["total_steps"]
        status_dict["total_events"] = s["total_events"]
        status_dict["total_feeds"] = s["total_feeds"]
        status_dict["total_gaps_detected"] = s["total_gaps_detected"]
        status_dict["vertices_active"] = s["vertices_active"]
        status_dict["edges_active"] = s["edges_active"]
        status_dict["beta_1"] = s["beta_1"]
        status_dict["settled_cycles"] = s["settled_cycles"]
        status_dict["crystallization_count"] = s["crystallization_count"]
        # Domain distribution
        status_dict["domain_distribution"] = json.dumps(s.get("domain_distribution", {}))
        # Encounter density from recent logs
        if daemon.engine and daemon.engine.logs:
            recent = daemon.engine.logs[-100:]
            encounters = sum(1 for log in recent if log.operation != "walk")
            status_dict["encounter_density"] = round(encounters / max(len(recent), 1), 3)
        # Expression pressure
        watermark = getattr(daemon, '_reported_step_watermark', 0)
        if daemon.engine and daemon.engine.logs:
            unreported = sum(1 for log in daemon.engine.logs
                           if log.step > watermark and (log.delta_beta_1 != 0 or log.blocked))
            status_dict["expression_pressure"] = unreported
        # Position for dashboard
        if daemon.engine:
            status_dict["position"] = daemon.engine.position
        status_dict["_updated_at"] = time.time()
    except Exception:
        pass


def _update_topology(daemon, topology_cache: dict, topology_json_fn) -> None:
    """Rebuild topology snapshot. Skip f-value computation for speed."""
    try:
        # Use skeleton (not full) to avoid blocking traversal with f-value computation
        topo = topology_json_fn(daemon)
        topology_cache["json"] = json.dumps(topo, ensure_ascii=False)
        topology_cache["_updated_at"] = time.time()
    except Exception:
        pass


def _update_api_caches(daemon, status_dict: dict,
                       narrative_fn, gaps_fn, ops_fn, persistence_fn) -> None:
    """Pre-compute expensive API responses and cache them."""
    try:
        status_dict["_narrative_json"] = json.dumps(
            narrative_fn(daemon), ensure_ascii=False)
    except Exception:
        pass
    try:
        status_dict["_gaps_json"] = json.dumps(
            gaps_fn(daemon), ensure_ascii=False)
    except Exception:
        pass
    try:
        status_dict["_operations_json"] = json.dumps(
            ops_fn(daemon), ensure_ascii=False)
    except Exception:
        pass
    try:
        status_dict["_persistence_json"] = json.dumps(
            persistence_fn(daemon), ensure_ascii=False)
    except Exception:
        pass


def _drain_feed_queue(daemon, feed_queue: Queue, event_queue: Queue) -> None:
    """Process all pending feed requests."""
    from engine import compute_beta_1

    while True:
        try:
            text = feed_queue.get_nowait()
        except Exception:
            break

        try:
            beta_before = compute_beta_1(daemon.k_active)
            v_before = len(daemon.k_active.active_vertex_ids())

            sub = daemon.feed(text)

            beta_after = compute_beta_1(daemon.k_active)
            v_after = len(daemon.k_active.active_vertex_ids())

            # Push feed result as event
            event_queue.put_nowait({
                "type": "feed_result",
                "accepted": True,
                "new_vertices": v_after - v_before,
                "new_edges": len(sub.active_edges()),
                "delta_beta_1": beta_after - beta_before,
            })
        except Exception as exc:
            event_queue.put_nowait({
                "type": "feed_result",
                "accepted": False,
                "error": str(exc),
            })


def _drain_present_queue(daemon, present_queue: Queue,
                         present_result_queue: Queue) -> None:
    """Process all pending present requests."""
    from daemon_api import present_json, _count_unreported

    while True:
        try:
            request = present_queue.get_nowait()
        except Exception:
            break

        try:
            text = request.get("text", "")
            session_id = request.get("session_id", "default")
            request_id = request.get("request_id", "")

            result = present_json(daemon, text, session_id)
            if result is None:
                result = {
                    "type": "silence",
                    "parts": [],
                    "injected": False,
                    "concepts_found": [],
                    "expression_pressure": _count_unreported(daemon),
                }

            present_result_queue.put_nowait({
                "request_id": request_id,
                "result": result,
            })
        except Exception as exc:
            present_result_queue.put_nowait({
                "request_id": request.get("request_id", ""),
                "result": {
                    "type": "error",
                    "error": str(exc),
                },
            })


# ---------------------------------------------------------------------------
# HTTP handler for multiprocess mode
# ---------------------------------------------------------------------------

class MultiprocessHTTPHandler:
    """HTTP request handler that reads from shared state instead of daemon directly.

    This is not a BaseHTTPRequestHandler subclass -- it provides the routing logic
    that is injected into the actual handler.
    """

    @staticmethod
    def handle_get(
        path: str,
        params: dict,
        status_dict: dict,
        topology_cache: dict,
    ) -> tuple[dict | list, int]:
        """Route GET requests. Returns (response_data, status_code)."""
        if path == "/status":
            return _build_status_response(status_dict), 200

        elif path == "/topology":
            # Return cached topology
            cached = topology_cache.get("json")
            if cached:
                # Return raw JSON string -- caller must handle
                return {"_raw_json": cached}, 200
            return {"nodes": [], "links": [], "meta": {}}, 200

        elif path == "/query":
            concept = params.get("concept", [""])[0]
            if not concept:
                return {"error": "missing concept parameter"}, 400
            # Search in cached topology (no daemon access needed)
            cached = topology_cache.get("json")
            if not cached:
                return {"found": False, "vertex": None, "neighbors": [], "f_terrain": {}, "narrative": ""}, 200
            import json as _json
            topo = _json.loads(cached)
            # Find matching node
            concept_lower = concept.lower()
            match = None
            for n in topo.get("nodes", []):
                if concept_lower in n.get("label", "").lower() or concept_lower in n.get("id", "").lower():
                    match = n
                    break
            if not match:
                return {"found": False, "vertex": None, "neighbors": [], "f_terrain": {}, "narrative": ""}, 200
            # Find neighbors from links
            mid = match["id"]
            neighbors = []
            for lk in topo.get("links", []):
                src = lk["source"] if isinstance(lk["source"], str) else lk["source"].get("id", "")
                tgt = lk["target"] if isinstance(lk["target"], str) else lk["target"].get("id", "")
                if src == mid:
                    # Find target node
                    for n2 in topo["nodes"]:
                        if n2["id"] == tgt:
                            neighbors.append({"id": tgt, "label": n2["label"], "f": n2.get("f_avg", 0), "edge_type": lk.get("type", "dependency")})
                            break
                elif tgt == mid:
                    for n2 in topo["nodes"]:
                        if n2["id"] == src:
                            neighbors.append({"id": src, "label": n2["label"], "f": n2.get("f_avg", 0), "edge_type": lk.get("type", "dependency")})
                            break
                if len(neighbors) >= 20:
                    break
            return {
                "found": True,
                "vertex": {
                    "id": match["id"], "label": match["label"],
                    "f_avg": match.get("f_avg", 0), "degree": match.get("degree", 0),
                    "settled": match.get("settled", False), "rings": 0, "settled_rings": 0,
                },
                "neighbors": neighbors,
                "f_terrain": {"fold_zone": [], "gray_zone": [], "negate_zone": []},
                "narrative": "",
            }, 200

        elif path == "/narrative":
            cached = status_dict.get("_narrative_json")
            if cached:
                return {"_raw_json": cached}, 200
            return [], 200

        elif path == "/gaps":
            cached = status_dict.get("_gaps_json")
            if cached:
                return {"_raw_json": cached}, 200
            return [], 200

        elif path == "/operations":
            cached = status_dict.get("_operations_json")
            if cached:
                return {"_raw_json": cached}, 200
            return {}, 200

        elif path == "/persistence":
            cached = status_dict.get("_persistence_json")
            if cached:
                return {"_raw_json": cached}, 200
            return {"pairs": [], "total_beta_1": 0, "settled_count": 0,
                    "pending_count": 0, "beta1_curve": []}, 200

        return {"error": "not found", "endpoints": [
            "/status", "/topology", "/narrative", "/gaps",
            "/operations", "/persistence", "/feed", "/present",
        ]}, 404

    @staticmethod
    def handle_post(
        path: str,
        data: dict,
        feed_queue: Queue,
        present_queue: Queue,
        present_result_queue: Queue,
        status_dict: dict | None = None,
    ) -> tuple[dict, int]:
        """Route POST requests. Returns (response_data, status_code)."""
        if path == "/feed":
            text = data.get("text", "")
            if not text:
                return {"error": "missing text parameter"}, 400
            feed_queue.put(text)
            return {"accepted": True, "queued": True}, 202

        elif path == "/present":
            text = data.get("text", "")
            session_id = data.get("session_id", "default")
            import uuid
            request_id = str(uuid.uuid4())

            present_queue.put({
                "text": text,
                "session_id": session_id,
                "request_id": request_id,
            })

            # Wait for result with timeout
            # LLM language organ calls can take up to 30s — give enough room
            deadline = time.time() + 35.0  # 35s timeout (LLM API timeout is 30s)
            while time.time() < deadline:
                try:
                    response = present_result_queue.get(timeout=0.1)
                    if response.get("request_id") == request_id:
                        return response["result"], 200
                    else:
                        # Put it back for another consumer (shouldn't happen
                        # in single-server mode, but safe)
                        present_result_queue.put(response)
                except Exception:
                    continue

            # Timeout fallback: return simplified status instead of 504
            # The traversal subprocess is busy — give the caller what we have
            return {
                "type": "silence",
                "parts": [],
                "injected": False,
                "concepts_found": [],
                "expression_pressure": status_dict.get("expression_pressure", 0),
                "timeout_fallback": True,
            }, 200

        return {"error": "not found"}, 404


def _build_status_response(status_dict: dict) -> dict:
    """Build /status response from shared dict."""
    # Parse domain distribution from JSON string
    domain_raw = status_dict.get("domain_distribution", "{}")
    try:
        domain_dist = json.loads(domain_raw) if isinstance(domain_raw, str) else domain_raw
    except (json.JSONDecodeError, TypeError):
        domain_dist = {}
    return {
        "beta_1": status_dict.get("beta_1", 0),
        "vertices": status_dict.get("vertices_active", 0),
        "edges": status_dict.get("edges_active", 0),
        "settled": status_dict.get("settled_cycles", 0),
        "steps": status_dict.get("total_steps", 0),
        "crystallization_count": status_dict.get("crystallization_count", 0),
        "encounter_density": status_dict.get("encounter_density", 0.0),
        "status": "traversing",
        "expression_pressure": status_dict.get("expression_pressure", 0),
        "domain_distribution": domain_dist,
    }


# ---------------------------------------------------------------------------
# Multiprocess HTTP server (stdlib http.server, reads shared state)
# ---------------------------------------------------------------------------

from http.server import HTTPServer, BaseHTTPRequestHandler
from socketserver import ThreadingMixIn
from urllib.parse import urlparse, parse_qs
import threading


class _MPHTTPHandler(BaseHTTPRequestHandler):
    """HTTP handler for multiprocess mode. Reads shared state, never touches daemon."""

    # These are set before server starts (class-level injection)
    status_dict: dict = {}
    topology_cache: dict = {}
    feed_queue: Queue | None = None
    present_queue: Queue | None = None
    present_result_queue: Queue | None = None

    def do_GET(self) -> None:
        parsed = urlparse(self.path)
        path = parsed.path.rstrip("/")
        params = parse_qs(parsed.query)

        data, code = MultiprocessHTTPHandler.handle_get(
            path, params, self.status_dict, self.topology_cache)

        self._json_response(data, code)

    def do_POST(self) -> None:
        parsed = urlparse(self.path)
        path = parsed.path.rstrip("/")

        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length) if content_length > 0 else b"{}"
        try:
            req_data = json.loads(body)
        except json.JSONDecodeError:
            self._json_response({"error": "invalid JSON"}, 400)
            return

        data, code = MultiprocessHTTPHandler.handle_post(
            path, req_data,
            self.feed_queue, self.present_queue, self.present_result_queue,
            self.status_dict)

        self._json_response(data, code)

    def do_OPTIONS(self) -> None:
        self.send_response(200)
        self._add_cors_headers()
        self.send_header("Content-Length", "0")
        self.end_headers()

    def _json_response(self, data: Any, status: int = 200) -> None:
        # Check for pre-serialized JSON
        if isinstance(data, dict) and "_raw_json" in data:
            body = data["_raw_json"].encode("utf-8")
        else:
            body = json.dumps(data, ensure_ascii=False).encode("utf-8")

        self.send_response(status)
        self._add_cors_headers()
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _add_cors_headers(self) -> None:
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")

    def log_message(self, format, *args) -> None:
        pass  # Suppress HTTP logging


# ---------------------------------------------------------------------------
# WS event broadcaster (main process)
# ---------------------------------------------------------------------------

def _ws_broadcast_loop(event_queue: Queue, ws_port: int) -> None:
    """Background thread: drain event_queue and broadcast via WebSocket.

    Runs in the main process. The WS server itself runs in an asyncio event loop
    within this thread.
    """
    import asyncio

    try:
        import websockets
    except ImportError:
        print("  WebSocket: DISABLED (pip install websockets)", file=sys.stderr)
        # Still drain the queue to prevent memory leak
        while True:
            try:
                event_queue.get(timeout=5)
            except Exception:
                pass
        return

    # Use a list wrapper so nested async functions can mutate the set
    # (Python 3.14 scoping: augmented assignment creates local binding)
    ws_clients: set = set()

    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)

    async def handler(ws, path=None):
        ws_clients.add(ws)
        try:
            await ws.wait_closed()
        finally:
            ws_clients.discard(ws)

    async def broadcaster():
        while True:
            # Drain queue in a non-blocking way
            messages = []
            while True:
                try:
                    msg = event_queue.get_nowait()
                    messages.append(msg)
                except Exception:
                    break

            if messages and ws_clients:
                for msg in messages:
                    data = json.dumps(msg, ensure_ascii=False)
                    dead = set()
                    for ws in list(ws_clients):
                        try:
                            await ws.send(data)
                        except Exception:
                            dead.add(ws)
                    for d in dead:
                        ws_clients.discard(d)

            await asyncio.sleep(0.05)

    async def main():
        async with websockets.serve(handler, "0.0.0.0", ws_port):
            print(f"  WebSocket: ws://localhost:{ws_port}/ws", file=sys.stderr)
            await broadcaster()

    loop.run_until_complete(main())


# ---------------------------------------------------------------------------
# Public API: start the multiprocess daemon
# ---------------------------------------------------------------------------

def start_multiprocess_daemon(
    graph_json: dict | None = None,
    port: int = 8080,
    ws_port: int = 8765,
    persist_path: str | None = None,
    settlement_threshold: int = 15,
    seed: int = 42,
    autonomous: bool = False,
    topology_refresh_interval: int = 50,
    ipfs: bool = False,
) -> None:
    """Start the multiprocess daemon: traversal subprocess + HTTP/WS in main.

    This function blocks forever (runs the HTTP server in the main thread).
    Ctrl+C to stop.
    """
    manager = multiprocessing.Manager()

    # Shared state
    status_dict = manager.dict()
    topology_cache = manager.dict()

    # Communication queues
    feed_queue: Queue = Queue()
    present_queue: Queue = Queue()
    present_result_queue: Queue = Queue()
    event_queue: Queue = Queue(maxsize=1000)

    # Config for worker
    config = {
        "persist_path": persist_path,
        "settlement_threshold": settlement_threshold,
        "seed": seed,
        "autonomous": autonomous,
        "topology_refresh_interval": topology_refresh_interval,
        "ipfs": ipfs,
    }

    # Start traversal subprocess
    worker = Process(
        target=_traversal_worker,
        args=(graph_json, status_dict, topology_cache,
              feed_queue, present_queue, present_result_queue,
              event_queue, config),
        name="traversal-worker",
        daemon=True,
    )
    worker.start()
    print(f"  Traversal worker: PID {worker.pid}", file=sys.stderr)

    # Start WS broadcast thread
    ws_thread = threading.Thread(
        target=_ws_broadcast_loop,
        args=(event_queue, ws_port),
        daemon=True,
        name="ws-broadcast",
    )
    ws_thread.start()

    # Start HTTP server in main thread
    _MPHTTPHandler.status_dict = status_dict
    _MPHTTPHandler.topology_cache = topology_cache
    _MPHTTPHandler.feed_queue = feed_queue
    _MPHTTPHandler.present_queue = present_queue
    _MPHTTPHandler.present_result_queue = present_result_queue

    class _ThreadingHTTPServer(ThreadingMixIn, HTTPServer):
        daemon_threads = True

    server = _ThreadingHTTPServer(("0.0.0.0", port), _MPHTTPHandler)
    print(f"  HTTP API: http://localhost:{port}", file=sys.stderr)
    print(f"  Mode: multiprocess (GIL-free HTTP)", file=sys.stderr)

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n  Shutting down...", file=sys.stderr)
        worker.terminate()
        worker.join(timeout=5)
        server.shutdown()


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(
        description="Multiprocess daemon: traversal in subprocess, HTTP/WS in main")
    parser.add_argument("--load", type=str, help="Load graph from JSON file")
    parser.add_argument("--hegel", action="store_true",
                        help="Build from Hegel Phenomenology chapters (default)")
    parser.add_argument("--port", type=int, default=8080, help="HTTP port")
    parser.add_argument("--ws-port", type=int, default=8765, help="WebSocket port")
    parser.add_argument("--persist", type=str, nargs="?", const="default",
                        help="Enable JSONL persistence")
    parser.add_argument("--autonomous", action="store_true",
                        help="Enable auto-feed on gap detection")
    parser.add_argument("--seed", type=int, default=42,
                        help="Random seed for traversal")
    parser.add_argument("--topology-refresh", type=int, default=50,
                        help="Rebuild topology cache every N steps")
    parser.add_argument("--ipfs", action="store_true",
                        help="Enable IPFS background upload for significant events")
    args = parser.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)

    # Load .env if available
    env_path = os.path.join(script_dir, ".env")
    if os.path.exists(env_path):
        with open(env_path, "r", encoding="utf-8") as ef:
            for line in ef:
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    key, _, value = line.partition("=")
                    os.environ.setdefault(key.strip(), value.strip())

    # Build graph JSON (serialized, to pass to subprocess)
    graph_json = None
    if args.load:
        load_path = os.path.expanduser(args.load)
        if not os.path.isabs(load_path):
            load_path = os.path.join(script_dir, load_path)
        with open(load_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        if "graph_data" in data:
            graph_json = data["graph_data"]
        else:
            graph_json = data
    # If no --load: graph_json=None means worker will build from chapters

    # Persistence
    persist_path = None
    if args.persist:
        if args.persist == "default":
            from persistence import DEFAULT_PATH
            persist_path = str(DEFAULT_PATH)
        else:
            persist_path = args.persist

    print("=" * 60, file=sys.stderr)
    print("FENGLIANG MULTIPROCESS DAEMON", file=sys.stderr)
    print("=" * 60, file=sys.stderr)

    start_multiprocess_daemon(
        graph_json=graph_json,
        port=args.port,
        ws_port=args.ws_port,
        persist_path=persist_path,
        seed=args.seed,
        autonomous=args.autonomous,
        topology_refresh_interval=args.topology_refresh,
        ipfs=args.ipfs,
    )


if __name__ == "__main__":
    multiprocessing.freeze_support()
    main()
