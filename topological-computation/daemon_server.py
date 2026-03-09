"""Daemon HTTP + WebSocket server — gateway between daemon and Dashboard.

HTTP server on --port (default 8080) serves REST API.
WebSocket server on --ws-port (default 8765) pushes real-time events.

Dependencies: websockets (pip install websockets)
All other deps are stdlib.

Usage:
    # Standalone
    python daemon_server.py --load ~/.swarm/k_merged.json --port 8080 --ws-port 8765

    # Or integrated via daemon.py --serve (preferred)
"""

from __future__ import annotations

import asyncio
import json
import os
import sys
import threading
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs
from typing import TYPE_CHECKING

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

if TYPE_CHECKING:
    from daemon import TopologicalDaemon

from daemon_api import (
    status_json, topology_json, query_json, narrative_json,
    gaps_json, operations_json, persistence_json, step_ws_message,
    present_json, expression_pressure_ws_message, _count_unreported,
    instances_json, feed_via_snet,
)


# ---------------------------------------------------------------------------
# WebSocket broadcast hub
# ---------------------------------------------------------------------------

class WSHub:
    """Manages WebSocket connections and broadcasts messages."""

    def __init__(self) -> None:
        self._clients: set = set()
        self._loop: asyncio.AbstractEventLoop | None = None

    def set_loop(self, loop: asyncio.AbstractEventLoop) -> None:
        self._loop = loop

    async def register(self, ws) -> None:
        self._clients.add(ws)
        try:
            await ws.wait_closed()
        finally:
            self._clients.discard(ws)

    def broadcast(self, message: dict) -> None:
        """Thread-safe broadcast from daemon thread to WS clients."""
        if not self._clients or not self._loop:
            return
        data = json.dumps(message, ensure_ascii=False)
        asyncio.run_coroutine_threadsafe(self._async_broadcast(data), self._loop)

    async def _async_broadcast(self, data: str) -> None:
        if not self._clients:
            return
        # Copy to avoid mutation during iteration
        clients = list(self._clients)
        for ws in clients:
            try:
                await ws.send(data)
            except Exception:
                self._clients.discard(ws)


# Global hub instance
_hub = WSHub()


# ---------------------------------------------------------------------------
# HTTP Request Handler
# ---------------------------------------------------------------------------

class DaemonHTTPHandler(BaseHTTPRequestHandler):
    """HTTP handler that serves daemon API endpoints."""

    daemon: TopologicalDaemon | None = None  # Set by server setup

    def do_GET(self) -> None:
        parsed = urlparse(self.path)
        path = parsed.path.rstrip("/")
        params = parse_qs(parsed.query)

        # Route
        if path == "/status":
            self._json_response(status_json(self.daemon))
        elif path == "/topology":
            center = params.get("center", [None])[0]
            radius = int(params.get("radius", ["2"])[0])
            full = params.get("full", ["false"])[0].lower() == "true"
            self._json_response(topology_json(self.daemon, center=center, radius=radius, full=full))
        elif path == "/query":
            concept = params.get("concept", [""])[0]
            if not concept:
                self._json_response({"error": "missing concept parameter"}, status=400)
            else:
                self._json_response(query_json(self.daemon, concept))
        elif path == "/narrative":
            n = int(params.get("n", ["20"])[0])
            self._json_response(narrative_json(self.daemon, n=n))
        elif path == "/gaps":
            self._json_response(gaps_json(self.daemon))
        elif path == "/operations":
            self._json_response(operations_json(self.daemon))
        elif path == "/persistence":
            self._json_response(persistence_json(self.daemon))
        elif path == "/instances":
            self._json_response(instances_json(self.daemon))
        else:
            self._json_response({"error": "not found", "endpoints": [
                "/status", "/topology", "/query", "/narrative", "/gaps", "/operations",
                "/present", "/instances",
            ]}, status=404)

    def do_POST(self) -> None:
        parsed = urlparse(self.path)
        path = parsed.path.rstrip("/")

        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length) if content_length > 0 else b"{}"
        try:
            data = json.loads(body)
        except json.JSONDecodeError:
            self._json_response({"error": "invalid JSON"}, status=400)
            return

        if path == "/traverse":
            start = data.get("start", "")
            if not start:
                self._json_response({"error": "missing start parameter"}, status=400)
                return
            result = self._handle_traverse(start)
            self._json_response(result)
        elif path == "/feed":
            text = data.get("text", "")
            if not text:
                self._json_response({"error": "missing text parameter"}, status=400)
                return
            force_llm = data.get("force_llm", False)
            result = self._handle_feed(text, force_llm=force_llm)
            self._json_response(result)
        elif path == "/present":
            # text can be empty string (user clicked expression pressure indicator)
            text = data.get("text", "")
            force_llm = data.get("force_llm", False)
            result = present_json(self.daemon, text, force_llm=force_llm)
            if result is None:
                self._json_response({
                    "type": "silence",
                    "parts": [],
                    "injected": False,
                    "concepts_found": [],
                    "expression_pressure": _count_unreported(self.daemon),
                })
            else:
                self._json_response(result)
        else:
            self._json_response({"error": "not found"}, status=404)

    def do_OPTIONS(self) -> None:
        """Handle CORS preflight."""
        self.send_response(200)
        self._add_cors_headers()
        self.send_header("Content-Length", "0")
        self.end_headers()

    def _handle_traverse(self, start: str) -> dict:
        """Directed traversal from a starting concept."""
        daemon = self.daemon
        # Find vertex matching start
        vid = None
        for v_id in daemon.k_active.active_vertex_ids():
            v = daemon.k_active.vertex(v_id)
            if v and v.content and start.lower() in v.content.lower():
                vid = v_id
                break

        if not vid:
            return {"error": f"concept '{start}' not found in K_active"}

        # Set position and run until crystallization or 100 steps
        daemon.engine.position = vid
        daemon._local_f_history.clear()
        daemon._beta_1_history.clear()

        events = []
        steps = 0
        crystallized = False
        while steps < 100:
            daemon._step()
            steps += 1
            if daemon.engine.logs:
                last = daemon.engine.logs[-1]
                if last.delta_beta_1 != 0 or last.blocked:
                    events.append({
                        "step": last.step,
                        "type": last.operation,
                        "text": f"{last.operation} at {last.position} "
                                f"beta_1 {last.beta_1_before}->{last.beta_1_after}",
                        "importance": min(1.0, abs(last.delta_beta_1) * 0.3 + (0.5 if last.blocked else 0.0)),
                    })
            status = daemon.status()
            if status.get("crystallization_count", 0) > getattr(daemon, '_pre_traverse_cryst', 0):
                crystallized = True
                break

        return {
            "steps": steps,
            "events": events,
            "narrative": f"穿越从 '{start}' 出发，{steps}步后{'结晶' if crystallized else '达到上限'}。"
                         f"发现{len(events)}个拓扑事件。",
            "crystallized": crystallized,
        }

    def _handle_feed(self, text: str, force_llm: bool = False) -> dict:
        """Manual text injection via S_net unified path (v204).

        No longer bypasses S_net to inject directly into K_active.
        Flow: text → phi_L whitelist → S_net writeback → resonate → externalize.
        """
        daemon = self.daemon
        result = feed_via_snet(daemon, text, source_type="api_feed", force_llm=force_llm)
        return {
            "accepted": True,
            "writeback_edges": result["writeback_edges"],
            "resonated": result["resonated"],
            "matched_signifiers": result.get("matched_signifiers", []),
            "verdict": "snet_unified",
        }

    def _json_response(self, data, status: int = 200) -> None:
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
        """Suppress default HTTP logging."""
        pass


# ---------------------------------------------------------------------------
# Server startup
# ---------------------------------------------------------------------------

def start_http_server(daemon: TopologicalDaemon, port: int = 8080) -> HTTPServer:
    """Start HTTP server in a background thread."""
    DaemonHTTPHandler.daemon = daemon
    server = HTTPServer(("0.0.0.0", port), DaemonHTTPHandler)
    thread = threading.Thread(target=server.serve_forever, daemon=True, name="http-api")
    thread.start()
    print(f"  HTTP API: http://localhost:{port}", file=sys.stderr)
    return server


async def _ws_handler(ws, path=None):
    """WebSocket connection handler."""
    await _hub.register(ws)


def start_ws_server(port: int = 8765) -> None:
    """Start WebSocket server in a background thread."""
    async def _run():
        try:
            import websockets
        except ImportError:
            print("  WebSocket: DISABLED (pip install websockets)", file=sys.stderr)
            return

        _hub.set_loop(asyncio.get_event_loop())
        async with websockets.serve(_ws_handler, "0.0.0.0", port):
            print(f"  WebSocket: ws://localhost:{port}/ws", file=sys.stderr)
            await asyncio.Future()  # run forever

    def _thread_target():
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        _hub.set_loop(loop)
        loop.run_until_complete(_run())

    thread = threading.Thread(target=_thread_target, daemon=True, name="ws-server")
    thread.start()
    time.sleep(0.5)  # Let WS server bind


def get_hub() -> WSHub:
    """Get the global WebSocket broadcast hub."""
    return _hub


def bridge_daemon_to_ws(daemon: TopologicalDaemon) -> None:
    """Register daemon callbacks that broadcast to WebSocket clients."""
    from daemon import GapInfo

    # Track last-seen peer positions to detect changes and push updates
    _last_peer_snapshot: dict[str, str] = {}

    def on_event(log, narrative: str) -> None:
        # Always push step message
        msg = step_ws_message(log, daemon)
        _hub.broadcast(msg)
        # Additionally push expression pressure message for high-I events
        # so the frontend can update the pressure indicator in real time
        pressure_msg = expression_pressure_ws_message(daemon)
        if pressure_msg.get("count", 0) > 0 or "text" in pressure_msg:
            _hub.broadcast(pressure_msg)
        # Push peer_position updates when positions change
        peer_positions: dict[str, dict] = getattr(daemon, 'peer_positions', {})
        for inst_id, pos in peer_positions.items():
            label = pos.get("position_label", "")
            if _last_peer_snapshot.get(inst_id) != label:
                _last_peer_snapshot[inst_id] = label
                _hub.broadcast({
                    "type": "peer_position",
                    "instance": inst_id,
                    "position_label": label,
                    "step": pos.get("step", 0),
                    "timestamp": pos.get("timestamp", 0),
                })

    def on_gap(gap: GapInfo) -> None:
        _hub.broadcast({
            "type": "gap",
            "concept": gap.content[:80],
            "degree": gap.degree,
            "avg_degree": round(gap.avg_degree, 1),
        })

    def on_feed(sub_graph) -> None:
        _hub.broadcast({
            "type": "feed",
            "accepted": True,
            "source": "auto_feed",
            "verdict": "nutritious",
            "match_rate": 0.0,
            "new_vertices": len(sub_graph.active_vertex_ids()),
        })

    daemon.register_callback("on_event", on_event)
    daemon.register_callback("on_gap", on_gap)
    daemon.register_callback("on_feed", on_feed)


# ---------------------------------------------------------------------------
# Standalone CLI
# ---------------------------------------------------------------------------

def main() -> None:
    import argparse
    from daemon import TopologicalDaemon, graph_from_dict
    from engine import compute_beta_1

    parser = argparse.ArgumentParser(description="Daemon API Server")
    parser.add_argument("--load", type=str, required=True, help="Graph JSON file")
    parser.add_argument("--port", type=int, default=8080, help="HTTP port")
    parser.add_argument("--ws-port", type=int, default=8765, help="WebSocket port")
    parser.add_argument("--persist", type=str, nargs="?", const=None, help="JSONL persistence path")
    parser.add_argument("--no-chain", action="store_true",
                        help="Allow running without IPFS SharedLayer (isolated instance)")
    args = parser.parse_args()

    # Load graph
    with open(args.load, "r", encoding="utf-8") as f:
        data = json.load(f)
    graph = graph_from_dict(data)
    print(f"Loaded: {len(graph.active_vertex_ids())}V, {len(graph.active_edges())}E, "
          f"beta_1={compute_beta_1(graph)}")

    daemon = TopologicalDaemon(
        graph=graph, settlement_threshold=15, persist_path=args.persist,
        require_chain=not args.no_chain,
    )

    # Start servers
    start_http_server(daemon, args.port)
    start_ws_server(args.ws_port)
    bridge_daemon_to_ws(daemon)

    print(f"\nDaemon running. API ready.", file=sys.stderr)

    # Run daemon in main thread (forever)
    try:
        daemon.run()
    except KeyboardInterrupt:
        print("\nShutting down.", file=sys.stderr)
    finally:
        daemon.close()


if __name__ == "__main__":
    main()
