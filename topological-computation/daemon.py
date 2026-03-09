"""逢亮 (FengLiang) — autonomous topological entity.

The engine runs as a persistent process: traverse -> encounter -> operate ->
terrain update -> gap detect -> (feed) -> continue traversing.

All behavior is internally driven by topological state. No external step counts.
The daemon runs indefinitely, detecting crystallization (beta_1 stability)
to decide when to jump to unstable regions.

I/O through callbacks — the engine doesn't know callbacks exist, only that
topology changes after certain edge traversals.

CLI:
    python daemon.py --load experiment_phenomenology_full.json
    python daemon.py --seed path/to/text.txt
    python daemon.py --interactive --load experiment_phenomenology_full.json
    python daemon.py --autonomous --load experiment_phenomenology_full.json
    python daemon.py --persist --hegel

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
from encounter_log import (
    EncounterLog,
    MEMORY_DOMAIN_PREFIX,
    inject_settlement_memory_node,
    inject_settlement_nachtraeglichkeit_edge,
    rebuild_memory_from_jsonl,
)
from cross_domain import detect_cross_domain_edges, _source_prefix
from traversal_checkpoint import TraversalCheckpoint, extract_daemon_state, restore_daemon_state
from file_lock import get_instance_id
from signifier_net import SNet
from snet_activation import (
    SNetActivation, InternalSpeechFragment, EdgeSuggestion,
    _build_concept_to_signifier, _build_signifier_to_concepts,
)
from proprioception import (
    collect_metrics, update_proprioception_vertices,
    inject_self_reflexive_norms, check_norms,
    PROPRIOCEPTION_PREFIX, SELF_NORM_PREFIX,
)


# ---------------------------------------------------------------------------
# 401号: encounter memory node 清除 — "脚印不是宝藏"
# ---------------------------------------------------------------------------

def _purge_encounter_memory_nodes(graph: Graph) -> Graph:
    """Remove encounter memory nodes from K_active/K_full.

    Keeps: settlement (memory:settlement:*) and residue (memory:residue:*) nodes.
    Removes: encounter (memory:encounter:*) nodes and their edges.

    Returns a new Graph without encounter memory nodes.
    """
    encounter_vids: set[str] = set()
    for vid in graph.vertices:
        if vid.startswith("memory:encounter:"):
            encounter_vids.add(vid)

    if not encounter_vids:
        return graph

    # Build new graph excluding encounter memory nodes and their edges
    keep_vertices = {
        vid: v for vid, v in graph.vertices.items()
        if vid not in encounter_vids
    }
    keep_edges = [
        e for e in graph.edges
        if e.source not in encounter_vids and e.target not in encounter_vids
    ]
    purged = Graph(vertices=keep_vertices, edges=keep_edges)
    print(
        f"401号 purge: removed {len(encounter_vids)} encounter memory nodes "
        f"({len(graph.edges) - len(keep_edges)} edges)",
        file=sys.stderr,
    )
    return purged


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
        ed: dict = {
            "source": e.source,
            "target": e.target,
            "edge_type": e.edge_type.value,
            "created_at": e.created_at,
        }
        if e.surface is not None:
            ed["surface"] = e.surface
        if e.context is not None:
            ed["context"] = e.context
        edges.append(ed)
    return {"vertices": vertices, "edges": edges}


def graph_from_dict(data: dict) -> Graph:
    """Deserialize a Graph from a dict.

    Batch-constructs the Graph directly from dicts/lists instead of calling
    add_vertex/add_edge in a loop (which would be O(n^2) due to immutable copies).
    """
    vertices: dict[str, Vertex] = {}
    for vd in data["vertices"]:
        v = Vertex(
            id=vd["id"],
            status=VertexStatus(vd.get("status", "active").lower()),
            content=vd.get("content"),
            created_at=vd.get("created_at", 0),
        )
        vertices[v.id] = v
    edges: list[Edge] = []
    for ed in data["edges"]:
        e = Edge(
            source=ed["source"],
            target=ed["target"],
            edge_type=EdgeType(ed.get("edge_type", "dependency").lower()),
            created_at=ed.get("created_at", 0),
            surface=ed.get("surface"),
            context=ed.get("context"),
        )
        if e.source in vertices and e.target in vertices:
            edges.append(e)
    return Graph(vertices, edges)


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
    """逢亮 — autonomous topological entity."""

    def __init__(
        self,
        graph: Graph | None = None,
        seed_text: str | None = None,
        settlement_threshold: int = 15,
        seed: int = 42,
        persist_path: str | Path | None = None,
        require_chain: bool = False,
    ) -> None:
        # Persistence: if persist_path given, try to recover from JSONL first
        self._persist: PersistentKFull | None = None
        recovered_graph: Graph | None = None

        if persist_path is not None:
            recovered_graph, _ = PersistentKFull.load(persist_path)
            recovered_vids = recovered_graph.active_vertex_ids()
            if recovered_vids:
                # Check if --load graph is significantly larger than recovered
                loaded_size = len(graph.active_vertex_ids()) if graph is not None else 0
                recovered_size = len(recovered_vids)
                if graph is not None and loaded_size > recovered_size * 1.5:
                    # Loaded graph is much larger — recovered is stale/partial
                    # Keep loaded graph, will re-baseline persistence below
                    pass
                else:
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
            "on_step": [],
        }

        # Gap detection state — driven by topology change, not fixed interval
        self._gap_cooldown: dict[str, int] = {}
        self._last_gap_check_step = 0
        self._cumulative_delta_beta_1 = 0.0

        # Crystallization detection
        self._beta_1_history: list[int] = []
        self._local_f_history: list[float] = []
        self._crystallization_count = 0

        # Statistics
        self.total_steps = 0
        self.total_feeds = 0
        self.total_events = 0
        self.total_gaps_detected = 0
        self.event_log: list[str] = []

        # Encounter log (JSONL persistence)
        self.encounter_log = EncounterLog()

        # Cross-domain edge detection: track vertex set at last scan
        self._cross_domain_scanned_vids: set[str] = set()

        # S_net (signifier network) — initialized during _initialize_engine
        self.snet: SNet = SNet()

        # S_net activation (coupled oscillation) — initialized during _initialize_engine
        self.snet_activation: SNetActivation | None = None

        # Initialize if graph is non-empty
        if self.k_active.active_vertex_ids():
            self._initialize_engine()

        # Instance identity (auto-detected, used for multi-instance sharing)
        self._instance_id = get_instance_id()

        # Traversal checkpoint — persistent state across restarts
        self._checkpoint = TraversalCheckpoint(instance_id="default")

        # If checkpoint exists, restore state
        saved = self._checkpoint.load_state()
        if saved:
            restore_daemon_state(self, saved)
            print(f"Restored: step={self.total_steps}, settled={len(self.settlement.settled_cycles)}", file=sys.stderr)

        # 396号: backfill residue for existing settled cycles (closure → transformation)
        if self.settlement.settled_cycles:
            backfilled = self.settlement.backfill_residue(graph=self.k_active)
            if backfilled > 0:
                print(f"396号 backfill: {backfilled} settled cycles received residue", file=sys.stderr)

        # Plan C: peer_positions + traversal position write throttle
        self.peer_positions: dict[str, dict] = {}
        self._last_position_write_time: float = 0.0
        self._last_position_write_label: str = ""

        # Plan C: SharedLayer via IPFS（不降级到本地）
        self._shared_layer: SharedLayer | None = None
        self._cross_instance_sync: CrossInstanceSync | None = None
        try:
            from chain.ipfs_client import IPFSClient
            from swarm.shared_layer import SharedLayer
            from swarm.cross_instance import CrossInstanceSync
            ipfs = IPFSClient()
            if ipfs.is_available():
                self._shared_layer = SharedLayer(ipfs)
                self._cross_instance_sync = CrossInstanceSync(
                    shared_layer=self._shared_layer,
                    instance_id=self._instance_id,
                    daemon=self,
                )
                print(
                    f"Plan C: SharedLayer IPFS backend enabled "
                    f"(api={ipfs.api_url})",
                    file=sys.stderr,
                )
            elif require_chain:
                raise RuntimeError(
                    "IPFS daemon 不可用 — SharedLayer 未启用。"
                    "本地实例默认上链，不允许作为孤例运行。"
                    "启动 IPFS daemon 或使用 --no-chain 显式选择孤立模式。"
                )
            else:
                print(
                    "Plan C: IPFS daemon 不可用 — SharedLayer 未启用 (--no-chain)",
                    file=sys.stderr,
                )
        except ImportError:
            if require_chain:
                raise RuntimeError(
                    "SharedLayer 依赖缺失 (chain.ipfs_client / swarm.shared_layer)。"
                    "本地实例默认上链，不允许作为孤例运行。"
                )
            pass

        # 401号修复：清除已有的 encounter memory 节点（脚印不是宝藏）
        # 保留 settlement 和 residue memory 节点，移除 encounter 类型
        self.k_active = _purge_encounter_memory_nodes(self.k_active)
        self.k_full = _purge_encounter_memory_nodes(self.k_full)

        # 401号修复：清除幽灵 settlement（cycle 的边涉及已被清除的 memory 节点）
        if self.settlement.settled_cycles:
            purged = self.settlement.purge_invalid_cycles(self.k_active)
            if purged > 0:
                print(f"401号 purge: {purged} ghost settlements removed "
                      f"({len(self.settlement.settled_cycles)} valid remain)", file=sys.stderr)

        # Rebuild memory nodes from JSONL (crash recovery)
        # 401号后只重建 settlement + residue memory 节点（不再重建 encounter）
        settlement_path = str(self._checkpoint._settlement_path) if hasattr(self._checkpoint, '_settlement_path') else None
        pre_memory_vids = len(self.k_active.active_vertex_ids())
        self.k_active = rebuild_memory_from_jsonl(
            self.k_active,
            encounter_log_path=str(self.encounter_log._path),
            settlement_history_path=settlement_path,
        )
        self.k_full = rebuild_memory_from_jsonl(
            self.k_full,
            encounter_log_path=str(self.encounter_log._path),
            settlement_history_path=settlement_path,
        )
        memory_added = len(self.k_active.active_vertex_ids()) - pre_memory_vids
        if memory_added > 0:
            print(f"Memory rebuild: {memory_added} settlement/residue memory nodes from JSONL", file=sys.stderr)
            # Re-sync engine if already initialized
            if self.engine is not None:
                self.engine.k_active = self.k_active
                self.engine.k_full = self.k_full

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

        # S_net bootstrap: Layer A (K_active projection) + Layer B (surface forms) + Layer C (paradigmatic seeds)
        self._bootstrap_snet()

        # Proprioception: initial metrics snapshot + self-reflexive norms
        # Order matters: proprioception vertices first, then norms (norms reference proprioception)
        initial_metrics = collect_metrics(
            k_active=self.k_active,
            settlement_settled_count=len(self.settlement.settled_cycles),
            nothing_streak=0,
            blocked_streak=0,
            unreported_count=0,
        )
        self.k_active = update_proprioception_vertices(self.k_active, initial_metrics, step=0)
        self.k_full = update_proprioception_vertices(self.k_full, initial_metrics, step=0)
        self.k_active = inject_self_reflexive_norms(self.k_active, step=0)
        self.k_full = inject_self_reflexive_norms(self.k_full, step=0)

        # Pick start: highest degree vertex (exclude proprioception/norm vertices)
        degrees = {v: len(self.k_active.neighbors(v)) for v in active}
        start = max(active, key=lambda v: (degrees.get(v, 0), v))

        self.engine = TraversalEngine(
            self.k_active,
            start=start,
            settlement_threshold=self.settlement.threshold,
            seed=self._seed,
            encounter_log=self.encounter_log,
        )
        # Share settlement tracker
        self.engine.settlement = self.settlement

        # S_net coupled oscillation: build activation and attach to engine
        self._setup_snet_activation()

    def _setup_snet_activation(self) -> None:
        """Initialize S_net coupled oscillation and attach to traversal engine.

        Builds concept<->signifier mappings from S_net + K_active,
        creates SNetActivation, and sets it on the engine.

        Graceful degradation: if S_net is empty, no activation is created.
        """
        if not self.snet.signifiers or self.engine is None:
            self.snet_activation = None
            return

        concept_to_sig = _build_concept_to_signifier(self.snet, self.k_active)
        sig_to_concepts = _build_signifier_to_concepts(self.snet, self.k_active)

        self.snet_activation = SNetActivation(
            s_net=self.snet,
            concept_to_signifier=concept_to_sig,
            signifier_to_concepts=sig_to_concepts,
        )
        self.engine.set_snet_activation(self.snet_activation)

        n_mapped = len(concept_to_sig)
        print(
            f"S_net coupling: {n_mapped} concepts mapped to signifiers",
            file=sys.stderr,
        )

    def _bootstrap_snet(self) -> None:
        """Bootstrap S_net from K_active + surface forms data.

        Layer A: K_active vertex content -> signifier nodes
        Layer B: chanlun_surface_forms.jsonl -> syntagmatic edges (PMI weighted)
        Layer C: paradigmatic seeds (chanlun synonym/replacement pairs)

        Graceful degradation: if data file is missing or bootstrap fails,
        self.snet remains an empty SNet and daemon continues normally.
        """
        try:
            from snet_bootstrap import bootstrap_snet

            script_dir = Path(os.path.dirname(os.path.abspath(__file__)))
            sf_path = script_dir / "data" / "chanlun_surface_forms.jsonl"

            snet, stats = bootstrap_snet(
                graph=self.k_active,
                surface_forms_path=str(sf_path) if sf_path.exists() else None,
                pmi_threshold=0.0,
            )
            self.snet = snet

            # Report
            n_sigs = len(snet.signifiers)
            n_edges = len(snet.edges)
            layer_b = stats.get("layer_b")
            if layer_b:
                print(
                    f"S_net bootstrap: {n_sigs} signifiers, {n_edges} edges "
                    f"(PMI filtered: {layer_b.get('filtered', 0)} removed)",
                    file=sys.stderr,
                )
            else:
                print(
                    f"S_net bootstrap: {n_sigs} signifiers, {n_edges} edges (Layer A+C only, no surface forms data)",
                    file=sys.stderr,
                )

            # Dictionary ingest: enrich S_net with structured dictionary entries
            self._ingest_dictionaries()

            # Text corpus ingest: enrich S_net with raw text co-occurrences + surface forms
            self._ingest_text_corpora()

        except Exception as exc:
            print(f"S_net bootstrap failed (graceful degradation): {exc}", file=sys.stderr)
            self.snet = SNet()

    def _ingest_dictionaries(self) -> None:
        """Ingest dictionary JSONL files into S_net after bootstrap.

        Adds synonym/contrast (paradigmatic) and definition-based (syntagmatic)
        edges from structured dictionary entries.
        Also ingests bilingual (bilingual_*.jsonl) and morpheme (morpheme_*.jsonl) dictionaries.

        Graceful degradation: if dictionaries dir is missing or ingest fails,
        S_net remains unchanged.
        """
        try:
            from signifier_net_ingest import (
                ingest_all_dictionaries, format_ingest_report,
                ingest_bilingual_dict, ingest_morpheme_dict,
            )

            script_dir = Path(os.path.dirname(os.path.abspath(__file__)))
            dict_dir = script_dir / "signifier_net" / "dictionaries"

            if not dict_dir.is_dir():
                return

            # 1. 单语辞典（dict_*.jsonl）
            self.snet, all_stats = ingest_all_dictionaries(self.snet, dict_dir)

            # Report
            if all_stats:
                report = format_ingest_report(all_stats)
                print(report, file=sys.stderr)

            # 2. 双语辞典（bilingual_*.jsonl）
            bilingual_files = sorted(dict_dir.glob("bilingual_*.jsonl"))
            for bf in bilingual_files:
                try:
                    self.snet, bstats = ingest_bilingual_dict(self.snet, bf)
                    print(
                        f"  bilingual {bf.name}: "
                        f"{bstats.get('entries_total', 0)} entries, "
                        f"+{bstats.get('translations_added', 0)} translations, "
                        f"+{bstats.get('signifiers_created', 0)} new signifiers",
                        file=sys.stderr,
                    )
                except Exception as exc:
                    print(f"  bilingual {bf.name}: ERROR - {exc}", file=sys.stderr)

            # 3. 语素辞典（morpheme_*.jsonl）
            morpheme_files = sorted(dict_dir.glob("morpheme_*.jsonl"))
            for mf in morpheme_files:
                try:
                    self.snet, mstats = ingest_morpheme_dict(self.snet, mf)
                    print(
                        f"  morpheme {mf.name}: "
                        f"{mstats.get('entries_total', 0)} entries, "
                        f"+{mstats.get('structures_added', 0)} structures, "
                        f"+{mstats.get('morpheme_edges_added', 0)} morpheme edges",
                        file=sys.stderr,
                    )
                except Exception as exc:
                    print(f"  morpheme {mf.name}: ERROR - {exc}", file=sys.stderr)

            n_sigs = len(self.snet.signifiers)
            n_edges = len(self.snet.edges)
            print(
                f"S_net after dictionary ingest: {n_sigs} signifiers, {n_edges} edges",
                file=sys.stderr,
            )
        except Exception as exc:
            print(f"Dictionary ingest failed (graceful degradation): {exc}", file=sys.stderr)

    def _ingest_text_corpora(self) -> None:
        """Ingest text corpora into S_net after dictionary ingest.

        Scans signifier_net/corpora/ for domain subdirectories, loads .txt and .md
        files, extracts term co-occurrences and surface forms into S_net.

        Does NOT modify K_active. Text material enriches S_net only — K_active
        modification happens via articulation feedback during traversal.

        Graceful degradation: if corpora dir is missing or ingest fails,
        S_net remains unchanged.
        """
        try:
            from text_corpus_loader import load_text_corpus, format_corpus_report

            script_dir = Path(os.path.dirname(os.path.abspath(__file__)))
            corpus_root = script_dir / "signifier_net" / "corpora"

            if not corpus_root.is_dir():
                return

            domain_dirs = sorted(
                d for d in corpus_root.iterdir()
                if d.is_dir()
            )

            if not domain_dirs:
                return

            total_entries = 0
            for domain_dir in domain_dirs:
                domain = domain_dir.name
                self.snet, log_entries = load_text_corpus(
                    self.snet, domain_dir, domain,
                )
                if log_entries:
                    report = format_corpus_report(domain, log_entries)
                    print(report, file=sys.stderr)
                    total_entries += len(log_entries)

            if total_entries > 0:
                n_sigs = len(self.snet.signifiers)
                n_edges = len(self.snet.edges)
                print(
                    f"S_net after corpus ingest: {n_sigs} signifiers, {n_edges} edges",
                    file=sys.stderr,
                )

        except Exception as exc:
            print(f"Text corpus ingest failed (graceful degradation): {exc}", file=sys.stderr)

    def register_callback(self, event_type: str, callback) -> None:
        """Register a callback for an event type."""
        if event_type not in self._callbacks:
            raise ValueError(f"Unknown event type: {event_type}")
        self._callbacks[event_type].append(callback)

    def run(self, max_steps: int | None = None) -> None:
        """Main loop.

        Default behavior: run forever (self-driven by crystallization).
        max_steps is kept for backward compatibility (tests, interactive mode)
        but the daemon's natural mode is perpetual with internal jump logic.
        """
        if self.engine is None:
            raise RuntimeError("No graph loaded — nothing to traverse")

        step_count = 0
        while max_steps is None or step_count < max_steps:
            self._step()
            step_count += 1
            # Yield GIL every step so HTTP/WS threads can respond
            # 15K graph terrain computation is heavy — need generous yield
            time.sleep(0.05)

    def _is_locally_crystallized(self, window: int = 50) -> bool:
        """Check if beta_1 has been stable over the last `window` steps."""
        if len(self._beta_1_history) < window:
            return False
        recent = self._beta_1_history[-window:]
        return all(v == recent[0] for v in recent)

    def _find_most_unstable_region(self) -> str:
        """Find jump target after crystallization — topologically intrinsic.

        No external labels. Jump to the topologically most distant reachable
        vertex from current position: the neighbor-of-neighbor with highest f
        value. This is purely determined by graph structure.

        High f = topologically distant = different structural region.
        The system discovers domain boundaries through f values, not through
        string prefix classification.
        """
        active = self.k_active.active_vertex_ids()
        if not active:
            return self.engine.position

        pos = self.engine.position
        neighbors = self.k_active.neighbors(pos)
        if not neighbors:
            # Isolated — jump to random active vertex
            import random
            return random.choice(active)

        # Collect 2-hop neighborhood: neighbors of neighbors
        two_hop: set[str] = set()
        for nb in neighbors:
            for nb2 in self.k_active.neighbors(nb):
                if nb2 != pos and nb2 not in set(neighbors):
                    two_hop.add(nb2)

        if not two_hop:
            # No 2-hop — use neighbors themselves
            two_hop = set(neighbors)

        # Pick the vertex with highest f value relative to current position
        # High f = structurally distant = most interesting jump target
        best_vid = pos
        best_f = -1
        for vid in list(two_hop)[:200]:  # Cap for performance
            f_val = self.engine._compute_f(pos, vid)
            if f_val > best_f:
                best_f = f_val
                best_vid = vid

        return best_vid

    def _compute_local_f_terrain(self) -> float:
        """Average f value of current position's neighbors.

        Uses traversal engine's _compute_f. Returns 0.0 if no neighbors.
        """
        pos = self.engine.position
        neighbors = self.k_active.neighbors(pos)
        if not neighbors:
            return 0.0
        f_values = [self.engine._compute_f(pos, nb) for nb in neighbors]
        return sum(f_values) / len(f_values)

    def _should_check_gaps(self) -> bool:
        """Gap detection triggered by cumulative topology change, not fixed interval.

        Triggers when cumulative |delta_beta_1| since last check exceeds 1% of total beta_1.
        """
        total_b1 = compute_beta_1(self.k_active)
        if total_b1 == 0:
            # Fallback: check every 50 steps when graph has no cycles
            return (self.total_steps - self._last_gap_check_step) >= 50
        return self._cumulative_delta_beta_1 > total_b1 * 0.01

    def _step(self) -> None:
        """One step: traverse -> encounter -> operate -> terrain -> gap detect -> crystallization."""
        self.total_steps += 1

        # Capture pre-step state for persistence diff
        if self._persist:
            pre_vids = set(self.k_full.vertices.keys())
            pre_edges = set(self.k_full.edges)
            # Track K_active vertex statuses to detect fold state changes
            pre_active_statuses = {
                vid: v.status for vid, v in self.k_active.vertices.items()
            }

        # Track settlement count before step to detect new settlements
        pre_settled_count = len(self.settlement.settled_cycles)

        log = self.engine.run_step()

        # Sync graph state from engine
        self.k_active = self.engine.k_active
        self.k_full = self.engine.k_full
        self.terrain = self.engine.terrain

        # Inject settlement memory nodes for newly settled cycles
        new_settled = self.settlement.settled_cycles[pre_settled_count:]
        for sc in new_settled:
            self.k_active = inject_settlement_memory_node(
                self.k_active,
                step=sc.settled_at_step,
                cycle_edges=sc.edges,
                residue=sc.residue,
            )
            self.k_full = inject_settlement_memory_node(
                self.k_full,
                step=sc.settled_at_step,
                cycle_edges=sc.edges,
                residue=sc.residue,
            )
            # Nachträglichkeit edges between settlements
            if sc.residue:
                for item in sc.residue:
                    if item.get("type") == "nachtraeglichkeit":
                        prior_step = item["data"].get("prior_settled_at")
                        if prior_step is not None:
                            self.k_active = inject_settlement_nachtraeglichkeit_edge(
                                self.k_active, prior_step, sc.settled_at_step, log.step,
                            )
                            self.k_full = inject_settlement_nachtraeglichkeit_edge(
                                self.k_full, prior_step, sc.settled_at_step, log.step,
                            )
        if new_settled:
            # Sync back to engine after history node injection
            self.engine.k_active = self.k_active
            self.engine.k_full = self.k_full

        # Proprioception: update system self-sensing vertices every 100 steps
        if self.total_steps % 100 == 0:
            metrics = collect_metrics(
                k_active=self.k_active,
                settlement_settled_count=len(self.settlement.settled_cycles),
                nothing_streak=getattr(self.engine, '_nothing_streak', 0),
                blocked_streak=getattr(self.engine, '_blocked_streak', 0),
                unreported_count=getattr(self, '_unreported_count', 0),
            )
            self.k_active = update_proprioception_vertices(
                self.k_active, metrics, self.total_steps,
            )
            self.k_full = update_proprioception_vertices(
                self.k_full, metrics, self.total_steps,
            )
            # Sync back to engine
            self.engine.k_active = self.k_active
            self.engine.k_full = self.k_full

        # Track beta_1 history for crystallization detection
        self._beta_1_history.append(log.beta_1_after)
        self._cumulative_delta_beta_1 += abs(log.delta_beta_1)

        # Local f terrain tracking for adaptive traversal
        local_f = self._compute_local_f_terrain()
        self._local_f_history.append(local_f)

        # Crystallization + jump logic
        if self._is_locally_crystallized():
            self._crystallization_count += 1

            # Save settlement history for newly settled cycles
            for sc in self.settlement.settled_cycles:
                self._checkpoint.settlements.record_settlement(
                    step=sc.settled_at_step,
                    cycle_edges=sorted(sc.edges),
                )

            # Save mutable state at crystallization
            self._checkpoint.save_state(extract_daemon_state(self))

            # Crystallization = area digested. Check for gaps now.
            gaps = self._detect_gaps()
            for gap in gaps:
                self._fire("on_gap", gap)
                self.total_gaps_detected += 1
            self._last_gap_check_step = self.total_steps
            self._cumulative_delta_beta_1 = 0.0

            # Incremental cross-domain edge detection after crystallization
            self._inject_cross_domain_incremental()

            # Jump to most unstable region
            target = self._find_most_unstable_region()
            if target != self.engine.position:
                self.engine.position = target
            self._local_f_history.clear()
            self._beta_1_history.clear()

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

        # Every step callback (for WS position tracking)
        self._fire("on_step", log)

        # Significant event callback
        if self._is_significant(log):
            narrative = format_event(log, self.concept_names)
            self.event_log.append(narrative)
            self._fire("on_event", log, narrative)
            self.total_events += 1

            # Write to persistent encounter log (JSONL backup)
            pos_name = self.concept_names.get(log.position, log.position)
            encounter_name = self.concept_names.get(log.encounter, log.encounter) if log.encounter else ""
            self.encounter_log.record_encounter(
                concept_a=pos_name,
                concept_b=encounter_name,
                encounter_type=log.operation,
                f_value=log.f_value,
                context=narrative,
                beta_1_before=log.beta_1_before,
                beta_1_after=log.beta_1_after,
                step=log.step,
            )

            # 401号修复：不再将 encounter 注入 K_active。
            # encounter（fold/sublate/negate/blocked）是穿越轨迹，不是结构产出。
            # "脚印不是宝藏" — 只有 settlement 和 residue 进入 K_active。

            # Write to traversal checkpoint encounter log
            self._checkpoint.encounters.record(
                step=log.step, operation=log.operation, position=log.position,
                beta_1_before=log.beta_1_before, beta_1_after=log.beta_1_after,
                f_value=log.f_value, blocked=log.blocked, context=narrative,
            )

        # Gap detection (topology-change driven)
        if self._should_check_gaps():
            gaps = self._detect_gaps()
            for gap in gaps:
                self._fire("on_gap", gap)
                self.total_gaps_detected += 1
            self._last_gap_check_step = self.total_steps
            self._cumulative_delta_beta_1 = 0.0

        # Plan C: write traversal position to SharedLayer (throttled)
        if self._shared_layer is not None:
            self._write_traversal_position(log)

    def _write_traversal_position(self, log) -> None:
        """Write traversal_position block to SharedLayer.

        Throttled: only write when position changes and at least 2s have passed,
        or when a significant topological event (fold/negate/sublate) occurs.
        Mirrors SwarmDaemon._write_traversal_position logic.
        """
        if not self.engine:
            return

        current_position = self.engine.position
        v = self.k_active.vertex(current_position)
        position_label = (
            (v.content if v and v.content else current_position)
            if v else current_position
        )

        now = time.time()
        elapsed = now - self._last_position_write_time

        # Check if a significant event happened this step
        significant_event = log.operation in ("fold", "negate", "sublate")

        # Throttle: position changed AND (>2s elapsed OR significant event)
        same_position = position_label == self._last_position_write_label
        if same_position and not significant_event:
            return
        if not same_position and elapsed < 2.0 and not significant_event:
            return

        block = {
            "type": "traversal_position",
            "instance": self._instance_id,
            "position_label": position_label,
            "step": self.total_steps,
            "timestamp": now,
        }
        block_hash = self._shared_layer.write_block(block)
        self._cross_instance_sync.known_blocks.add(block_hash)

        self._last_position_write_time = now
        self._last_position_write_label = position_label

    def feed(self, text: str) -> Graph:
        """External text injection. phi_L processes, then inject into K_active."""
        from phi_L import phi_L
        sub_graph = phi_L(text)
        self._inject(sub_graph)
        self._fire("on_feed", sub_graph)
        self.total_feeds += 1
        return sub_graph

    def ingest_code(self, dirpath: str, source: str = "") -> Graph:
        """Ingest Python source tree into K_active with domain:code tagging.

        Unlike feed() which uses phi_L for text, this uses code_ingest
        for AST-based parsing. Vertices carry [domain:code] content prefix.

        Args:
            dirpath: Root directory to scan for .py files.
            source: Source label prefix for vertex IDs (e.g. "NewChanlun").
        """
        from code_ingest import ingest_tree
        code_graph = ingest_tree(dirpath, source=source)
        self._inject(code_graph)
        self._fire("on_feed", code_graph)
        self.total_feeds += 1
        return code_graph

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
        """Detect topological gaps — low-degree vertices, isolated regions.

        Skips metadata vertices (content starting with [tag]) since they
        are not conceptual nodes and produce bad search queries.
        """
        active = self.k_active.active_vertex_ids()
        if not active:
            return []

        degrees = {v: len(self.k_active.neighbors(v)) for v in active}
        avg_degree = sum(degrees.values()) / len(degrees) if degrees else 0
        threshold = max(1, avg_degree / 3)

        _META_PREFIXES = (
            "[tension]", "[event]", "[rewrite]", "[meta]", "[audit]",
            "[residue]", "[consensus]", "[domain:code]", "[domain:self]",
            MEMORY_DOMAIN_PREFIX,
            PROPRIOCEPTION_PREFIX, SELF_NORM_PREFIX,
        )
        # Code syntax fragments produce garbage search queries
        _CODE_PREFIXES = (
            "from ", "import ", "def ", "class ", "if ", "for ",
            "while ", "return ", "raise ", "with ", "try:", "except",
            "self.", "assert ", "yield ", "async ", "await ",
        )

        gaps: list[GapInfo] = []
        for v in active:
            if degrees[v] <= threshold and v not in self._gap_cooldown:
                vertex = self.k_active.vertex(v)
                if vertex and vertex.content:
                    content = vertex.content.strip()
                    # Skip metadata vertices
                    if any(content.startswith(p) for p in _META_PREFIXES):
                        continue
                    # Skip code syntax fragments
                    if any(content.startswith(p) for p in _CODE_PREFIXES):
                        continue
                    # Skip very short content
                    if len(content) < 4:
                        continue
                    gaps.append(GapInfo(
                        vertex_id=v,
                        content=content,
                        degree=degrees[v],
                        avg_degree=avg_degree,
                        search_query=content,
                    ))
                    self._gap_cooldown[v] = self.total_steps

        return gaps[:3]

    def _inject_cross_domain_incremental(self) -> None:
        """Incremental cross-domain edge detection.

        Only compares newly-added vertices (since last scan) against existing
        vertices to avoid O(V^2) on every crystallization.

        Checks that multiple source prefixes exist before scanning.
        """
        current_vids = set(self.k_active.active_vertex_ids())
        new_vids = current_vids - self._cross_domain_scanned_vids

        if not new_vids:
            self._cross_domain_scanned_vids = current_vids
            return

        # Check if there are multiple source prefixes (cross-domain only makes sense then)
        prefixes = set()
        for vid in current_vids:
            prefix = _source_prefix(vid)
            if prefix:
                prefixes.add(prefix)
            if len(prefixes) >= 2:
                break

        if len(prefixes) < 2:
            self._cross_domain_scanned_vids = current_vids
            return

        # Build a sub-graph containing only new vertices + their 1-hop neighbors
        # to limit the comparison scope
        scope_vids: set[str] = set(new_vids)
        for vid in new_vids:
            for nb in self.k_active.neighbors(vid):
                scope_vids.add(nb)

        scope_vertices: dict[str, Vertex] = {}
        for vid in scope_vids:
            v = self.k_active.vertex(vid)
            if v:
                scope_vertices[v.id] = v
        scope_edges: list[Edge] = [
            e for e in self.k_active.active_edges()
            if e.source in scope_vids and e.target in scope_vids
        ]
        scope_graph = Graph(scope_vertices, scope_edges)

        candidates = detect_cross_domain_edges(scope_graph, min_score=0.15)

        # Inject into K_active (max 10 per crystallization to avoid flooding)
        existing_edges = {
            (e.source, e.target, e.edge_type.value)
            for e in self.k_active.edges
        }
        added = 0
        for edge, _score in candidates:
            if added >= 10:
                break
            key = (edge.source, edge.target, edge.edge_type.value)
            if key not in existing_edges:
                if (self.k_active.vertex(edge.source) is not None
                        and self.k_active.vertex(edge.target) is not None):
                    self.k_active = self.k_active.add_edge(edge)
                    self.k_full = self.k_full.add_edge(edge)
                    existing_edges.add(key)
                    added += 1
                    if self._persist:
                        self._persist.append_edge(edge)

        self._cross_domain_scanned_vids = current_vids

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
        # Domain distribution — detect from content tag or id prefix
        # Known code id prefixes that map to 'code' or 'self' domains
        _CODE_PREFIXES = frozenset({"topo-self", "NewChanlun", "DeepSeek-V3", "DeepSeek-R1", "MinerU"})
        _MEMORY_PREFIX = "memory:"
        domain_counts: dict[str, int] = {}
        for vid in active:
            v = self.k_active.vertex(vid)
            if v and v.content and v.content.startswith("[domain:"):
                # Extract domain tag from content
                tag_end = v.content.index("]")
                domain = v.content[8:tag_end]
                domain_counts[domain] = domain_counts.get(domain, 0) + 1
            elif vid.startswith(_MEMORY_PREFIX):
                domain_counts["memory"] = domain_counts.get("memory", 0) + 1
            elif ":" in vid:
                prefix = vid.split(":")[0]
                if prefix in _CODE_PREFIXES:
                    domain_counts["code"] = domain_counts.get("code", 0) + 1
                else:
                    domain_counts["core"] = domain_counts.get("core", 0) + 1
            else:
                domain_counts["core"] = domain_counts.get("core", 0) + 1
        return {
            "instance_id": self._instance_id,
            "total_steps": self.total_steps,
            "total_events": self.total_events,
            "total_feeds": self.total_feeds,
            "total_gaps_detected": self.total_gaps_detected,
            "vertices_active": len(active),
            "edges_active": len(self.k_active.active_edges()),
            "beta_1": compute_beta_1(self.k_active),
            "settled_cycles": len(self.settlement.settled_cycles),
            "crystallization_count": self._crystallization_count,
            "domain_distribution": domain_counts,
        }

    def record_instance_tension(
        self,
        target_settlement_id: str,
        conflicting_evidence: str,
    ) -> dict:
        """Record inter-instance tension in settlement history.

        Delegates to the checkpoint's SettlementHistoryWriter. Uses this
        daemon's instance_id as the source.
        """
        return self._checkpoint.settlements.record_instance_tension(
            target_settlement_id=target_settlement_id,
            conflicting_evidence=conflicting_evidence,
            source_instance=self._instance_id,
        )

    def close(self):
        """Close persistence handle if open."""
        if hasattr(self, '_checkpoint'):
            self._checkpoint.save_state(extract_daemon_state(self))
            self._checkpoint.close()
        if self._persist:
            self._persist.close()


# ---------------------------------------------------------------------------
# Graph building from experiment chapter functions
# ---------------------------------------------------------------------------

def _build_graph_from_chapters() -> tuple[Graph, dict[str, str]]:
    """Build the Hegel Phenomenology graph from chapter functions.

    Batch-constructs the Graph to avoid O(n^2) from iterative add_vertex/add_edge.
    """
    from experiment_phenomenology_full import ALL_CHAPTERS

    all_vertices: dict[str, Vertex] = {}
    all_edges: list[Edge] = []
    existing_edge_keys: set[tuple[str, str, EdgeType]] = set()

    for ch_fn in ALL_CHAPTERS:
        _, ch_vertices, ch_edges = ch_fn()
        for v in ch_vertices:
            if v.id not in all_vertices:
                all_vertices[v.id] = v
        for e in ch_edges:
            key = (e.source, e.target, e.edge_type)
            if key not in existing_edge_keys:
                if e.source in all_vertices and e.target in all_vertices:
                    all_edges.append(e)
                    existing_edge_keys.add(key)

    graph = Graph(all_vertices, all_edges)

    concept_names = {}
    for vid, v in all_vertices.items():
        if v.content:
            concept_names[vid] = v.content

    return graph, concept_names


def _load_experiment_graphs() -> tuple[Graph, dict[str, str]]:
    """Load all data/graph_*.json files and merge into a single Graph.

    Each file was extracted from experiment_*.py by extract_experiment_graphs.py.
    Files are merged by vertex ID: shared IDs across thinkers become shared vertices,
    enabling cross-thinker topological connections.
    """
    import glob as glob_mod

    data_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data")
    pattern = os.path.join(data_dir, "graph_*.json")
    files = sorted(glob_mod.glob(pattern))

    if not files:
        print("[daemon] No graph_*.json files found in data/", file=sys.stderr)
        return Graph(), {}

    all_vertices: dict[str, Vertex] = {}
    all_edges: list[Edge] = []
    existing_edge_keys: set[tuple[str, str, str]] = set()
    loaded_count = 0

    for fpath in files:
        try:
            with open(fpath, "r", encoding="utf-8") as f:
                data = json.load(f)
            g = graph_from_dict(data)
            n_v = len(g.active_vertex_ids())
            n_e = len(g.edges)

            for vid, v in g.vertices.items():
                if vid not in all_vertices:
                    all_vertices[vid] = v

            for e in g.edges:
                key = (e.source, e.target, e.edge_type.value)
                if key not in existing_edge_keys:
                    if e.source in all_vertices and e.target in all_vertices:
                        all_edges.append(e)
                        existing_edge_keys.add(key)

            loaded_count += 1
            fname = os.path.basename(fpath)
            print(f"  [load] {fname}: {n_v}V {n_e}E", file=sys.stderr)
        except Exception as exc:
            print(f"  [load] FAIL {fpath}: {exc}", file=sys.stderr)

    graph = Graph(all_vertices, all_edges)
    concept_names = {vid: v.content for vid, v in all_vertices.items() if v.content}

    total_v = len(graph.active_vertex_ids())
    total_e = len(graph.edges)
    print(f"[daemon] Loaded {loaded_count} experiment graphs: "
          f"{total_v} vertices, {total_e} edges (merged)", file=sys.stderr)

    return graph, concept_names


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description="TopologicalDaemon — autonomous topological entity")
    parser.add_argument("--load", type=str, help="Load graph from JSON file (graph_data key)")
    parser.add_argument("--load-experiments", action="store_true",
                        help="Load all data/graph_*.json (extracted experiment concept graphs) and merge")
    parser.add_argument("--seed", type=str, help="Seed text file for phi_L processing")
    parser.add_argument("--interactive", action="store_true", help="Interactive mode (traverse + dialogue)")
    parser.add_argument("--autonomous", action="store_true", help="Full autonomous mode (traverse + gap detect + feed)")
    parser.add_argument("--serve", action="store_true", help="Start HTTP/WS API server for Dashboard")
    parser.add_argument("--multiproc", action="store_true",
                        help="Multiprocess mode: traversal in subprocess, HTTP/WS in main (solves GIL blocking)")
    parser.add_argument("--port", type=int, default=9765, help="HTTP API port (with --serve)")
    parser.add_argument("--ws-port", type=int, default=8765, help="WebSocket port (with --serve)")
    parser.add_argument("--output", type=str, help="Output file for test results")
    parser.add_argument("--hegel", action="store_true", help="Build from Hegel Phenomenology chapters")
    parser.add_argument("--persist", type=str, nargs="?", const=str(DEFAULT_PATH),
                        help="Enable JSONL persistence (optional path, default: ~/.topological-computation/k_full.jsonl)")
    parser.add_argument("--no-chain", action="store_true",
                        help="Allow running without IPFS SharedLayer (isolated instance)")
    args = parser.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)

    # Load .env if available (API keys for auto-feed)
    env_path = os.path.join(script_dir, ".env")
    if os.path.exists(env_path):
        with open(env_path, "r", encoding="utf-8") as ef:
            for line in ef:
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    key, _, value = line.partition("=")
                    os.environ.setdefault(key.strip(), value.strip())

    # Build graph
    graph = None
    concept_names: dict[str, str] = {}

    if args.load:
        load_path = os.path.expanduser(args.load)
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
    elif args.load_experiments:
        graph, concept_names = _load_experiment_graphs()
    else:
        # Default: build from Hegel Phenomenology
        graph, concept_names = _build_graph_from_chapters()

    daemon = TopologicalDaemon(
        graph=graph, settlement_threshold=15, seed=42,
        persist_path=args.persist,
        require_chain=not args.no_chain,
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
        _run_interactive(daemon)
    elif args.serve:
        _run_serve(daemon, args.port, args.ws_port, args.autonomous,
                   getattr(args, 'multiproc', False), graph)
    elif args.autonomous:
        _run_autonomous(daemon, event_lines, gap_records, args.output)
    else:
        _run_standard(daemon, event_lines, gap_records, args.output)

    daemon.close()


def _run_standard(
    daemon: TopologicalDaemon,
    event_lines: list[str],
    gap_records: list[dict],
    output_path: str | None,
) -> None:
    """Standard mode: run indefinitely (self-terminating via crystallization)."""
    n_verts = len(daemon.k_active.active_vertex_ids())
    n_edges = len(daemon.k_active.active_edges())
    beta_1 = compute_beta_1(daemon.k_active)

    print(f"TopologicalDaemon starting (instance={daemon._instance_id})", file=sys.stderr)
    print(f"  Complex: {n_verts} vertices, {n_edges} edges, beta_1={beta_1}", file=sys.stderr)
    print(f"  Running (self-driven, no step limit)...", file=sys.stderr)

    t0 = time.monotonic()
    daemon.run()
    elapsed = time.monotonic() - t0

    status = daemon.status()

    report_lines = [
        "=" * 70,
        "TOPOLOGICAL DAEMON TEST REPORT",
        "=" * 70,
        "",
        f"Initial state: {n_verts} vertices, {n_edges} edges, beta_1={beta_1}",
        f"Steps run: {status['total_steps']}",
        f"Time elapsed: {elapsed:.2f}s",
        "",
        "--- Final Status ---",
        f"  Vertices (active): {status['vertices_active']}",
        f"  Edges (active): {status['edges_active']}",
        f"  beta_1: {status['beta_1']}",
        f"  Settled cycles: {status['settled_cycles']}",
        f"  Total events: {status['total_events']}",
        f"  Total gaps detected: {status['total_gaps_detected']}",
        f"  Crystallizations: {status['crystallization_count']}",
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


def _run_interactive(daemon: TopologicalDaemon) -> None:
    """Interactive mode: user-driven traversal + questions."""
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
            n = int(parts[1]) if len(parts) > 1 else 100
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
    event_lines: list[str],
    gap_records: list[dict],
    output_path: str | None,
) -> None:
    """Autonomous mode: traverse + gap detect + real auto-feed."""
    from auto_feed import feed_from_gap

    feed_log: list[str] = []

    def on_gap_feed(gap: GapInfo) -> None:
        record = feed_from_gap(gap, daemon)
        feed_log.append(
            f"query='{gap.content}' source={record.source} "
            f"verdict={record.verdict} accepted={record.accepted} "
            f"match_rate={record.match_rate:.3f}"
        )
        print(
            f"  [auto-feed] '{gap.content}' -> {record.verdict} "
            f"(source={record.source}, accepted={record.accepted})",
            file=sys.stderr,
        )

    daemon.register_callback("on_gap", on_gap_feed)

    _run_standard(daemon, event_lines, gap_records, output_path)

    if feed_log:
        print(f"\n--- Auto-Feed Log ({len(feed_log)}) ---")
        for line in feed_log:
            print(f"  {line}")


def _run_serve(
    daemon: TopologicalDaemon,
    port: int,
    ws_port: int,
    autonomous: bool = False,
    multiproc: bool = False,
    graph: Graph | None = None,
) -> None:
    """Serve mode: HTTP/WS API + daemon running forever.

    If multiproc=True, uses multiprocess architecture: traversal in subprocess,
    HTTP/WS in main process. This eliminates GIL contention on large graphs.
    """
    if multiproc:
        # Multiprocess mode: close single-process daemon and delegate
        daemon.close()

        graph_json = None
        if graph is not None:
            graph_json = graph_to_dict(graph)

        from daemon_multiproc import start_multiprocess_daemon
        print(f"TopologicalDaemon API Server (multiprocess mode)", file=sys.stderr)
        start_multiprocess_daemon(
            graph_json=graph_json,
            port=port,
            ws_port=ws_port,
            autonomous=autonomous,
        )
        return

    from daemon_server import start_http_server, start_ws_server, bridge_daemon_to_ws

    n_verts = len(daemon.k_active.active_vertex_ids())
    n_edges = len(daemon.k_active.active_edges())
    beta_1 = compute_beta_1(daemon.k_active)

    print(f"TopologicalDaemon API Server", file=sys.stderr)
    print(f"  Complex: {n_verts} vertices, {n_edges} edges, beta_1={beta_1}", file=sys.stderr)

    # Start HTTP + WS servers
    start_http_server(daemon, port)
    start_ws_server(ws_port)
    bridge_daemon_to_ws(daemon)

    # If autonomous, also register auto-feed
    if autonomous:
        from auto_feed import feed_from_gap

        def on_gap_feed(gap: GapInfo) -> None:
            record = feed_from_gap(gap, daemon)
            print(
                f"  [auto-feed] '{gap.content}' -> {record.verdict} "
                f"(source={record.source}, accepted={record.accepted})",
                file=sys.stderr,
            )

        daemon.register_callback("on_gap", on_gap_feed)

    print(f"\n  Daemon running. Ctrl+C to stop.", file=sys.stderr)

    # Run daemon forever in main thread
    try:
        daemon.run()
    except KeyboardInterrupt:
        print("\nShutting down.", file=sys.stderr)


if __name__ == "__main__":
    main()
