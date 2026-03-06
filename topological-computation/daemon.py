"""TopologicalDaemon — autonomous running topological entity.

The engine runs as a persistent process: traverse -> encounter -> operate ->
terrain update -> gap detect -> (feed) -> continue traversing.

I/O through callbacks — the engine doesn't know callbacks exist, only that
topology changes after certain edge traversals.

CLI:
    python daemon.py --load experiment_phenomenology_full.json --steps 200
    python daemon.py --seed path/to/text.txt --steps 500
    python daemon.py --interactive --load experiment_phenomenology_full.json
    python daemon.py --autonomous --load experiment_phenomenology_full.json --steps 5000
    python daemon.py --persist --hegel --steps 1000

Pure Python, no external dependencies beyond this project's modules.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from engine import (
    Graph, Vertex, Edge, EdgeType, VertexStatus,
    SettlementTracker, compute_beta_1,
)
from morse import compute_terrain
from traversal import TraversalEngine, StepLog
from concept_registry import Registry, _tokenize
from psi_L_narrative import generate_narrative, _ENCOUNTER_OPS
from persistence import PersistentKFull, DEFAULT_PATH


# ---------------------------------------------------------------------------
# Graph serialization (JSON round-trip)
# ---------------------------------------------------------------------------

def graph_to_dict(graph: Graph) -> dict:
    """Serialize a Graph to a JSON-compatible dict."""
    vertices = []
    for vid, v in graph.vertices.items():
        vertices.append({
            "id": v.id,
            "status": v.status.value,
            "content": v.content,
            "created_at": v.created_at,
        })
    edges = []
    for e in graph.edges:
        edges.append({
            "source": e.source,
            "target": e.target,
            "edge_type": e.edge_type.value,
            "created_at": e.created_at,
        })
    return {"vertices": vertices, "edges": edges}


def graph_from_dict(data: dict) -> Graph:
    """Deserialize a Graph from a dict."""
    g = Graph()
    for vd in data["vertices"]:
        v = Vertex(
            id=vd["id"],
            status=VertexStatus(vd["status"]),
            content=vd.get("content"),
            created_at=vd.get("created_at", 0),
        )
        g = g.add_vertex(v)
    for ed in data["edges"]:
        e = Edge(
            source=ed["source"],
            target=ed["target"],
            edge_type=EdgeType(ed["edge_type"]),
            created_at=ed.get("created_at", 0),
        )
        if g.vertex(e.source) is not None and g.vertex(e.target) is not None:
            g = g.add_edge(e)
    return g


# ---------------------------------------------------------------------------
# Registry builder (from graph vertex content, no blocks dir needed)
# ---------------------------------------------------------------------------

def _build_registry(graph: Graph) -> Registry:
    """Build a Registry from graph vertex content."""
    summaries: dict[str, str] = {}
    index: dict[str, list[str]] = {}
    for vid, vertex in graph.vertices.items():
        if vertex.status == VertexStatus.FOLDED:
            continue
        summary = vertex.content or vid
        summaries[vid] = summary
        for token in set(_tokenize(summary)):
            index.setdefault(token, []).append(vid)
    return Registry(_summaries=summaries, _index=index)


# ---------------------------------------------------------------------------
# Single-event narrative formatter
# ---------------------------------------------------------------------------

def format_event(log: StepLog, concept_names: dict[str, str] | None = None) -> str:
    """Format a single StepLog into a human-readable narrative line."""
    names = concept_names or {}
    pos_name = names.get(log.position, log.position)

    if log.operation == "walk":
        return f"[step {log.step}] walk -> '{pos_name}'"

    delta = log.beta_1_after - log.beta_1_before
    sign = f"+{delta}" if delta >= 0 else str(delta)
    blocked = " BLOCKED" if log.blocked else ""

    return (
        f"[step {log.step}] {log.operation} at '{pos_name}' "
        f"(f={log.f_value}) beta_1 {log.beta_1_before}->{log.beta_1_after} "
        f"({sign}){blocked}"
    )


# ---------------------------------------------------------------------------
# TopologicalDaemon
# ---------------------------------------------------------------------------

@dataclass
class GapInfo:
    """A detected topological gap."""
    vertex_id: str
    content: str
    degree: int
    avg_degree: float
    search_query: str


class TopologicalDaemon:
    """Autonomous running topological entity."""

    def __init__(
        self,
        graph: Graph | None = None,
        seed_text: str | None = None,
        settlement_threshold: int = 15,
        seed: int = 42,
        persist_path: str | Path | None = None,
    ) -> None:
        # Persistence: if persist_path given, try to recover from JSONL first
        self._persist: PersistentKFull | None = None
        recovered_graph: Graph | None = None

        if persist_path is not None:
            recovered_graph, _ = PersistentKFull.load(persist_path)
            if recovered_graph.active_vertex_ids():
                # Successfully recovered — use recovered graph
                graph = recovered_graph
            self._persist = PersistentKFull(persist_path)
            self._persist.open()

        if graph is not None:
            self.k_active = graph
        elif seed_text is not None:
            from phi_L import phi_L
            self.k_active = phi_L(seed_text)
        else:
            self.k_active = Graph()

        self.k_full = self.k_active
        self.settlement = SettlementTracker(threshold=settlement_threshold)
        self.terrain: dict[tuple[str, str], str] = {}
        self.registry: Registry = Registry()
        self.concept_names: dict[str, str] = {}

        # Engine state
        self.engine: TraversalEngine | None = None
        self._seed = seed

        # Callbacks
        self._callbacks: dict[str, list] = {
            "on_gap": [],
            "on_event": [],
            "on_feed": [],
        }

        # Gap detection state
        self._gap_cooldown: dict[str, int] = {}
        self._gap_check_interval = 50

        # Statistics
        self.total_steps = 0
        self.total_feeds = 0
        self.total_events = 0
        self.total_gaps_detected = 0
        self.event_log: list[str] = []

        # Initialize if graph is non-empty
        if self.k_active.active_vertex_ids():
            self._initialize_engine()

    def _initialize_engine(self) -> None:
        """Initialize or reinitialize the traversal engine from current graph."""
        active = self.k_active.active_vertex_ids()
        if not active:
            return

        self.terrain = compute_terrain(self.k_active)
        self.registry = _build_registry(self.k_active)

        # Build concept names
        self.concept_names = {}
        for vid, v in self.k_active.vertices.items():
            if v.content:
                self.concept_names[vid] = v.content

        # Pick start: highest degree vertex
        degrees = {v: len(self.k_active.neighbors(v)) for v in active}
        start = max(active, key=lambda v: (degrees.get(v, 0), v))

        self.engine = TraversalEngine(
            self.k_active,
            start=start,
            settlement_threshold=self.settlement.threshold,
            seed=self._seed,
        )
        # Share settlement tracker
        self.engine.settlement = self.settlement

    def register_callback(self, event_type: str, callback) -> None:
        """Register a callback for an event type."""
        if event_type not in self._callbacks:
            raise ValueError(f"Unknown event type: {event_type}")
        self._callbacks[event_type].append(callback)

    def run(self, max_steps: int | None = None) -> None:
        """Main loop. max_steps=None means run forever."""
        if self.engine is None:
            raise RuntimeError("No graph loaded — nothing to traverse")

        step_count = 0
        while max_steps is None or step_count < max_steps:
            self._step()
            step_count += 1

    def _step(self) -> None:
        """One step: traverse -> encounter -> operate -> terrain -> gap detect."""
        self.total_steps += 1

        # Capture pre-step state for persistence diff
        if self._persist:
            pre_vids = set(self.k_full.vertices.keys())
            pre_edges = set(self.k_full.edges)
            # Track K_active vertex statuses to detect fold state changes
            pre_active_statuses = {
                vid: v.status for vid, v in self.k_active.vertices.items()
            }

        log = self.engine.run_step()

        # Sync graph state from engine
        self.k_active = self.engine.k_active
        self.k_full = self.engine.k_full
        self.terrain = self.engine.terrain

        # Persist graph state changes (always, not just for significant events)
        if self._persist:
            # Write new vertices created by this step (in K_full)
            for vid, v in self.k_full.vertices.items():
                if vid not in pre_vids:
                    self._persist.append_vertex(v)
            # Write new edges created by this step (in K_full)
            for e in self.k_full.edges:
                if e not in pre_edges:
                    self._persist.append_edge(e)
            # Detect and write fold merges (ACTIVE → FOLDED with edge redirect)
            for vid, v in self.k_active.vertices.items():
                old_status = pre_active_statuses.get(vid)
                if old_status is not None and old_status != v.status:
                    if v.status == VertexStatus.FOLDED:
                        # Find which vertex absorbed this one:
                        # the kept vertex is position after fold
                        self._persist.append_merge(log.position, vid, log.step)
                    else:
                        # Other status changes (e.g. CONTESTED)
                        self._persist.append_vertex_status(vid, v.status.value, log.step)

        # Persist operation log for significant events
        if self._persist and self._is_significant(log):
            self._persist.append_operation(log.step, log.operation, {
                "position": log.position,
                "beta_1_before": log.beta_1_before,
                "beta_1_after": log.beta_1_after,
                "blocked": log.blocked,
            })

        # Significant event callback
        if self._is_significant(log):
            narrative = format_event(log, self.concept_names)
            self.event_log.append(narrative)
            self._fire("on_event", log, narrative)
            self.total_events += 1

        # Gap detection (periodic)
        if self.total_steps % self._gap_check_interval == 0:
            gaps = self._detect_gaps()
            for gap in gaps:
                self._fire("on_gap", gap)
                self.total_gaps_detected += 1

    def feed(self, text: str) -> Graph:
        """External text injection. phi_L processes, then inject into K_active."""
        from phi_L import phi_L
        sub_graph = phi_L(text)
        self._inject(sub_graph)
        self._fire("on_feed", sub_graph)
        self.total_feeds += 1
        return sub_graph

    def ask(self, question: str) -> str:
        """External question. Directed traversal + answer generation."""
        from interactive import InteractiveTraversal
        it = InteractiveTraversal(
            self.k_active, self.registry, self.terrain,
            self.concept_names,
        )
        answer = it.ask(question)
        return answer.narrative

    def _detect_gaps(self) -> list[GapInfo]:
        """Detect topological gaps — low-degree vertices, isolated regions."""
        active = self.k_active.active_vertex_ids()
        if not active:
            return []

        degrees = {v: len(self.k_active.neighbors(v)) for v in active}
        avg_degree = sum(degrees.values()) / len(degrees) if degrees else 0
        threshold = max(1, avg_degree / 3)

        gaps: list[GapInfo] = []
        for v in active:
            if degrees[v] <= threshold and v not in self._gap_cooldown:
                vertex = self.k_active.vertex(v)
                if vertex and vertex.content:
                    gaps.append(GapInfo(
                        vertex_id=v,
                        content=vertex.content,
                        degree=degrees[v],
                        avg_degree=avg_degree,
                        search_query=vertex.content,
                    ))
                    self._gap_cooldown[v] = self.total_steps

        return gaps[:3]

    def _inject(self, sub_graph: Graph) -> None:
        """Inject a sub-graph into K_active. Merge by vertex ID."""
        existing_vids = set(self.k_active.active_vertex_ids())
        existing_edges = {
            (e.source, e.target, e.edge_type.value)
            for e in self.k_active.edges
        }

        for vid in sub_graph.active_vertex_ids():
            if vid not in existing_vids:
                v = sub_graph.vertex(vid)
                self.k_active = self.k_active.add_vertex(v)
                self.k_full = self.k_full.add_vertex(v)
                if self._persist:
                    self._persist.append_vertex(v)

        for e in sub_graph.active_edges():
            key = (e.source, e.target, e.edge_type.value)
            if key not in existing_edges:
                if (self.k_active.vertex(e.source) is not None
                        and self.k_active.vertex(e.target) is not None):
                    self.k_active = self.k_active.add_edge(e)
                    self.k_full = self.k_full.add_edge(e)
                    existing_edges.add(key)
                    if self._persist:
                        self._persist.append_edge(e)

        # Rebuild terrain, registry, engine
        self._initialize_engine()
        # Preserve step count
        if self.engine:
            self.engine.step = self.total_steps

    def _is_significant(self, log: StepLog) -> bool:
        """Event is significant if beta_1 changed or operation was blocked."""
        return log.delta_beta_1 != 0 or log.blocked

    def _fire(self, event_type: str, *args) -> None:
        """Fire all callbacks for an event type."""
        for cb in self._callbacks.get(event_type, []):
            cb(*args)

    def status(self) -> dict:
        """Return current status snapshot."""
        active = self.k_active.active_vertex_ids()
        return {
            "total_steps": self.total_steps,
            "total_events": self.total_events,
            "total_feeds": self.total_feeds,
            "total_gaps_detected": self.total_gaps_detected,
            "vertices_active": len(active),
            "edges_active": len(self.k_active.active_edges()),
            "beta_1": compute_beta_1(self.k_active),
            "settled_cycles": len(self.settlement.settled_cycles),
        }

    def close(self):
        """Close persistence handle if open."""
        if self._persist:
            self._persist.close()


# ---------------------------------------------------------------------------
# Graph building from experiment chapter functions
# ---------------------------------------------------------------------------

def _build_graph_from_chapters() -> tuple[Graph, dict[str, str]]:
    """Build the Hegel Phenomenology graph from chapter functions."""
    from experiment_phenomenology_full import ALL_CHAPTERS

    graph = Graph()
    for ch_fn in ALL_CHAPTERS:
        _, ch_vertices, ch_edges = ch_fn()
        existing_vids = set(graph.vertices.keys())
        for v in ch_vertices:
            if v.id not in existing_vids:
                graph = graph.add_vertex(v)
        existing_edges = {(e.source, e.target, e.edge_type) for e in graph.edges}
        for e in ch_edges:
            key = (e.source, e.target, e.edge_type)
            if key not in existing_edges:
                if e.source in graph.vertices and e.target in graph.vertices:
                    graph = graph.add_edge(e)
                    existing_edges.add(key)

    concept_names = {}
    for vid, v in graph.vertices.items():
        if v.content:
            concept_names[vid] = v.content

    return graph, concept_names


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description="TopologicalDaemon — autonomous topological entity")
    parser.add_argument("--load", type=str, help="Load graph from JSON file (graph_data key)")
    parser.add_argument("--seed", type=str, help="Seed text file for phi_L processing")
    parser.add_argument("--steps", type=int, default=200, help="Number of steps to run")
    parser.add_argument("--interactive", action="store_true", help="Interactive mode (traverse + dialogue)")
    parser.add_argument("--autonomous", action="store_true", help="Full autonomous mode (traverse + gap detect + feed)")
    parser.add_argument("--output", type=str, help="Output file for test results")
    parser.add_argument("--hegel", action="store_true", help="Build from Hegel Phenomenology chapters")
    parser.add_argument("--persist", type=str, nargs="?", const=str(DEFAULT_PATH),
                        help="Enable JSONL persistence (optional path, default: ~/.topological-computation/k_full.jsonl)")
    args = parser.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)

    # Build graph
    graph = None
    concept_names: dict[str, str] = {}

    if args.load:
        load_path = args.load
        if not os.path.isabs(load_path):
            load_path = os.path.join(script_dir, load_path)
        with open(load_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        if "graph_data" in data:
            graph = graph_from_dict(data["graph_data"])
        else:
            # Assume it's a direct graph dict
            graph = graph_from_dict(data)
        for vid, v in graph.vertices.items():
            if v.content:
                concept_names[vid] = v.content
    elif args.seed:
        seed_path = args.seed
        if not os.path.isabs(seed_path):
            seed_path = os.path.join(script_dir, seed_path)
        with open(seed_path, "r", encoding="utf-8") as f:
            text = f.read()
        from phi_L import phi_L
        graph = phi_L(text)
        for vid, v in graph.vertices.items():
            if v.content:
                concept_names[vid] = v.content
    elif args.hegel:
        graph, concept_names = _build_graph_from_chapters()
    else:
        # Default: build from Hegel Phenomenology
        graph, concept_names = _build_graph_from_chapters()

    daemon = TopologicalDaemon(
        graph=graph, settlement_threshold=15, seed=42,
        persist_path=args.persist,
    )

    # If persisting with a fresh file (no recovery), write initial graph
    if args.persist and daemon._persist and graph is not None:
        recovered, _ = PersistentKFull.load(args.persist)
        if not recovered.active_vertex_ids():
            for vid, v in graph.vertices.items():
                daemon._persist.append_vertex(v)
            for e in graph.edges:
                daemon._persist.append_edge(e)

    # Event log collector
    event_lines: list[str] = []
    gap_records: list[dict] = []

    def on_event(log: StepLog, narrative: str) -> None:
        event_lines.append(narrative)
        print(narrative, file=sys.stderr)

    def on_gap(gap: GapInfo) -> None:
        record = {
            "vertex_id": gap.vertex_id,
            "content": gap.content,
            "degree": gap.degree,
            "avg_degree": round(gap.avg_degree, 2),
        }
        gap_records.append(record)
        print(f"  [gap] '{gap.content}' (degree={gap.degree}, avg={gap.avg_degree:.1f})",
              file=sys.stderr)

    daemon.register_callback("on_event", on_event)
    daemon.register_callback("on_gap", on_gap)

    if args.interactive:
        _run_interactive(daemon, args.steps)
    elif args.autonomous:
        _run_autonomous(daemon, args.steps, event_lines, gap_records, args.output)
    else:
        _run_standard(daemon, args.steps, event_lines, gap_records, args.output)

    daemon.close()


def _run_standard(
    daemon: TopologicalDaemon,
    steps: int,
    event_lines: list[str],
    gap_records: list[dict],
    output_path: str | None,
) -> None:
    """Standard mode: run N steps, output results."""
    n_verts = len(daemon.k_active.active_vertex_ids())
    n_edges = len(daemon.k_active.active_edges())
    beta_1 = compute_beta_1(daemon.k_active)

    print(f"TopologicalDaemon starting", file=sys.stderr)
    print(f"  Complex: {n_verts} vertices, {n_edges} edges, beta_1={beta_1}", file=sys.stderr)
    print(f"  Running {steps} steps...", file=sys.stderr)

    t0 = time.monotonic()
    daemon.run(max_steps=steps)
    elapsed = time.monotonic() - t0

    status = daemon.status()

    report_lines = [
        "=" * 70,
        "TOPOLOGICAL DAEMON TEST REPORT",
        "=" * 70,
        "",
        f"Initial state: {n_verts} vertices, {n_edges} edges, beta_1={beta_1}",
        f"Steps run: {steps}",
        f"Time elapsed: {elapsed:.2f}s",
        "",
        "--- Final Status ---",
        f"  Vertices (active): {status['vertices_active']}",
        f"  Edges (active): {status['edges_active']}",
        f"  beta_1: {status['beta_1']}",
        f"  Settled cycles: {status['settled_cycles']}",
        f"  Total events: {status['total_events']}",
        f"  Total gaps detected: {status['total_gaps_detected']}",
        "",
    ]

    if event_lines:
        report_lines.append(f"--- Events ({len(event_lines)}) ---")
        for line in event_lines:
            report_lines.append(f"  {line}")
        report_lines.append("")

    if gap_records:
        report_lines.append(f"--- Gaps Detected ({len(gap_records)}) ---")
        for g in gap_records:
            report_lines.append(
                f"  '{g['content']}' (degree={g['degree']}, avg_degree={g['avg_degree']})"
            )
        report_lines.append("")

    report = "\n".join(report_lines)
    print(report)

    if output_path:
        out_dir = os.path.dirname(output_path)
        if out_dir:
            os.makedirs(out_dir, exist_ok=True)
        with open(output_path, "w", encoding="utf-8") as f:
            f.write(report)
        print(f"Report saved to {output_path}", file=sys.stderr)


def _run_interactive(daemon: TopologicalDaemon, steps_per_round: int) -> None:
    """Interactive mode: traverse N steps, then accept questions."""
    n_verts = len(daemon.k_active.active_vertex_ids())
    n_edges = len(daemon.k_active.active_edges())
    beta_1 = compute_beta_1(daemon.k_active)

    print(f"TopologicalDaemon Interactive Mode")
    print(f"Complex: {n_verts} vertices, {n_edges} edges, beta_1={beta_1}")
    print(f"Commands: 'traverse N' to run N steps, 'quit' to exit, or ask a question.\n")

    while True:
        try:
            user_input = input("> ").strip()
        except (EOFError, KeyboardInterrupt):
            print("\nExiting.")
            break

        if not user_input:
            continue
        if user_input.lower() in ("quit", "exit", "q"):
            break

        if user_input.lower().startswith("traverse"):
            parts = user_input.split()
            n = int(parts[1]) if len(parts) > 1 else steps_per_round
            print(f"Traversing {n} steps...")
            daemon.run(max_steps=n)
            status = daemon.status()
            print(f"  After {n} steps: beta_1={status['beta_1']}, "
                  f"events={status['total_events']}, "
                  f"settled={status['settled_cycles']}")
        else:
            answer = daemon.ask(user_input)
            print()
            print(answer)
            print()


def _run_autonomous(
    daemon: TopologicalDaemon,
    steps: int,
    event_lines: list[str],
    gap_records: list[dict],
    output_path: str | None,
) -> None:
    """Autonomous mode: traverse + gap detect + auto-feed (mocked)."""
    # In autonomous mode, gap detection triggers mock feed
    feed_log: list[str] = []

    def on_gap_feed(gap: GapInfo) -> None:
        # Mock feed: generate a simple sentence about the gap concept
        mock_text = f"{gap.content} is a concept that requires further investigation."
        daemon.feed(mock_text)
        feed_log.append(f"Fed mock text for gap '{gap.content}'")

    daemon.register_callback("on_gap", on_gap_feed)

    _run_standard(daemon, steps, event_lines, gap_records, output_path)

    if feed_log:
        print(f"\n--- Auto-Feed Log ({len(feed_log)}) ---")
        for line in feed_log:
            print(f"  {line}")


if __name__ == "__main__":
    main()
