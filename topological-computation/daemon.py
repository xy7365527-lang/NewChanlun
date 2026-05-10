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
from datetime import datetime
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from engine import (
    Graph, Vertex, Edge, EdgeType, VertexStatus,
    SettlementTracker, compute_beta_1, fold,
)
from morse import compute_terrain
from traversal import TraversalEngine, StepLog
from concept_registry import Registry, _tokenize
from psi_L_narrative import generate_narrative, _ENCOUNTER_OPS
from persistence import PersistentKFull, DEFAULT_PATH
from block_topology_persistence import (
    BlockTopologyWriter,
    load_graph_from_block_topology,
    rebuild_block_topology_from_jsonl,
    DAEMON_BT_BASE,
)
from encounter_log import (
    EncounterLog,
    MEMORY_DOMAIN_PREFIX,
)
from cross_domain import detect_cross_domain_edges, _source_prefix
from traversal_checkpoint import TraversalCheckpoint, extract_daemon_state, restore_daemon_state
from file_lock import get_instance_id
from signifier_net import SNet, AxisType
from snet_activation import (
    SNetActivation, InternalSpeechFragment, EdgeSuggestion,
    OrphanExplorationHint,
    _build_concept_to_signifier, _build_signifier_to_concepts,
    _expand_mappings,
)
from proprioception import (
    collect_metrics, update_proprioception_vertices,
    inject_self_reflexive_norms, check_norms,
    PROPRIOCEPTION_PREFIX, SELF_NORM_PREFIX,
)


# ---------------------------------------------------------------------------
# 401号: encounter memory node 清除 — "脚印不是宝藏"
# ---------------------------------------------------------------------------

def _purge_memory_nodes(graph: Graph) -> Graph:
    """Remove ALL memory: prefix nodes from K_active/K_full.

    类型约束（P0 修复的启动时清理）：memory: 前缀顶点是 settlement 的产物，
    不是 settlement 的对象。产物不能回流为输入。历史遗留的 8.5M memory 顶点
    （正反馈循环的病理产物）在此一次性清除。

    Returns a new Graph without any memory: prefix nodes.
    """
    _PREFIX = "memory:"
    memory_vids: set[str] = set()
    for vid in graph.vertices:
        if vid.startswith(_PREFIX):
            memory_vids.add(vid)

    if not memory_vids:
        return graph

    keep_vertices = {
        vid: v for vid, v in graph.vertices.items()
        if vid not in memory_vids
    }
    keep_edges = [
        e for e in graph.edges
        if e.source not in memory_vids and e.target not in memory_vids
    ]
    purged = Graph(vertices=keep_vertices, edges=keep_edges)
    print(
        f"P0 purge: removed {len(memory_vids)} memory nodes "
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

def _friendly_pos_name(raw: str) -> str:
    """Make memory/metadata vertex names more readable for narrative output."""
    if raw.startswith("memory:"):
        # "memory:residue:boundary_edge:4:244424" -> "[记忆节点] boundary_edge"
        parts = raw.split(":")
        key = parts[2] if len(parts) > 2 else parts[-1]
        return f"[记忆节点] {key}"
    if raw.startswith("[") and "]" in raw:
        # "[tension] ..." or "[event] ..." -> keep the tag, trim long tails
        bracket_end = raw.index("]") + 1
        tag = raw[:bracket_end]
        rest = raw[bracket_end:].strip()
        if len(rest) > 30:
            rest = rest[:27] + "..."
        return f"{tag} {rest}" if rest else tag
    return raw


def format_event(log: StepLog, concept_names: dict[str, str] | None = None) -> str:
    """Format a single StepLog into a human-readable narrative line."""
    names = concept_names or {}
    pos_name = _friendly_pos_name(names.get(log.position, log.position))

    if log.operation == "walk":
        return f"[step {log.step}] walk -> '{pos_name}'"

    delta = log.beta_1_after - log.beta_1_before
    sign = f"+{delta}" if delta >= 0 else str(delta)
    blocked = " BLOCKED" if log.blocked else ""
    # 473号: Morse 命名门槛标注
    crit_mark = ""
    if log.critical is True:
        crit_mark = " CRITICAL"
    elif log.critical is False:
        crit_mark = " non-critical"

    return (
        f"[step {log.step}] {log.operation} at '{pos_name}' "
        f"(f={log.f_value}) beta_1 {log.beta_1_before}->{log.beta_1_after} "
        f"({sign}){blocked}{crit_mark}"
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
        # Persistence: block topology is primary, jsonl is backup
        self._persist: BlockTopologyWriter | None = None
        self._persist_path: Path | None = None
        recovered_graph: Graph | None = None

        if persist_path is not None:
            jsonl_p = Path(persist_path)
            snapshot_p = PersistentKFull.snapshot_path_for(jsonl_p)

            # Primary: load from block topology
            bt_graph, _ = load_graph_from_block_topology(DAEMON_BT_BASE)
            bt_vids = bt_graph.active_vertex_ids()

            if bt_vids:
                recovered_graph = bt_graph
                print(f"Block topology: loaded {len(bt_vids)} active vertices", file=sys.stderr)
            elif snapshot_p.exists():
                # Snapshot + incremental replay (fast path)
                snap_graph, _ = PersistentKFull.load_snapshot_then_incremental(
                    snapshot_p, jsonl_p,
                )
                snap_vids = snap_graph.active_vertex_ids()
                if snap_vids:
                    recovered_graph = snap_graph
                    print(
                        f"Snapshot recovery: {len(snap_vids)} active vertices",
                        file=sys.stderr,
                    )
            else:
                # Fallback: full JSONL replay (slow path for legacy data)
                jsonl_graph, _ = PersistentKFull.load(persist_path)
                jsonl_vids = jsonl_graph.active_vertex_ids()
                if jsonl_vids:
                    recovered_graph = jsonl_graph
                    print(
                        f"Block topology empty, recovered {len(jsonl_vids)} vertices from jsonl. "
                        f"Rebuilding block topology...",
                        file=sys.stderr,
                    )
                    count = rebuild_block_topology_from_jsonl(
                        Path(persist_path), DAEMON_BT_BASE,
                    )
                    print(f"Block topology rebuilt: {count} records", file=sys.stderr)

            if recovered_graph is not None:
                recovered_vids = recovered_graph.active_vertex_ids()
                loaded_size = len(graph.active_vertex_ids()) if graph is not None else 0
                recovered_size = len(recovered_vids)
                if graph is not None and loaded_size > recovered_size * 1.5:
                    pass
                else:
                    graph = recovered_graph

            # BlockTopologyWriter — JSONL append-only (456号裁定)
            self._persist_path = Path(persist_path)
            self._persist = BlockTopologyWriter(
                bt_base=DAEMON_BT_BASE,
                jsonl_backup_path=Path(persist_path),
            )
            self._persist.open()

        if graph is not None:
            self.k_active = graph
        elif seed_text is not None:
            from phi_L import phi_L
            self.k_active = phi_L(seed_text)
        else:
            self.k_active = Graph()

        self.k_full = self.k_active
        self.k_active = self.k_active.copy()  # independent copy: k_active mutates on fold/negate/sublate
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

        # S_net SQLite persistence layer (423号: replace pickle with SQLite)
        self._snet_persistence = None  # initialized in _bootstrap_snet

        # S_net → block topology watermark: tracks edge count at last block write
        # Used for incremental delta detection (only new edges get written)
        self._snet_block_watermark: int = 0
        # S_net → block topology hyperedge watermark: tracks hyperedge count
        self._snet_hyperedge_watermark: int = 0

        # S_net activation (coupled oscillation) — initialized during _initialize_engine
        self.snet_activation: SNetActivation | None = None

        # Initialize if graph is non-empty
        if self.k_active.active_vertex_ids():
            self._initialize_engine()

        # Instance identity (auto-detected, used for multi-instance sharing)
        self._instance_id = get_instance_id()

        # Walker session identity (unique per daemon run, for events.jsonl isolation)
        import uuid
        self._session_id = f"{self._instance_id}-{uuid.uuid4().hex[:8]}"

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

        # Plan C: peer_positions + sync throttle state
        self.peer_positions: dict[str, dict] = {}
        self._last_position_write_time: float = 0.0
        self._last_position_write_label: str = ""
        self._last_graph_delta_write_time: float = 0.0
        self._last_settlement_write_count: int = 0
        self._last_snet_write_time: float = 0.0
        self._last_snet_active_snapshot: set[str] = set()

        # Plan C: SharedLayer via IPFS（不降级到本地）
        self._shared_layer: SharedLayer | None = None
        self._cross_instance_sync: CrossInstanceSync | None = None
        try:
            from chain.ipfs_client import IPFSClient
            from swarm.shared_layer import SharedLayer
            from swarm.cross_instance import CrossInstanceSync
            ipfs = IPFSClient()
            if ipfs.is_available():
                try:
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
                except Exception as e:
                    self._shared_layer = None
                    self._cross_instance_sync = None
                    if require_chain:
                        raise RuntimeError(
                            "SharedLayer 初始化失败 — 本地实例默认上链，"
                            "不允许静默降级为孤例运行。"
                        ) from e
                    print(
                        f"Plan C: SharedLayer init failed ({e}), "
                        f"degrading to no shared layer",
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
        self.k_active = _purge_memory_nodes(self.k_active)
        self.k_full = _purge_memory_nodes(self.k_full)

        # 401号/415号: 幽灵 settlement 标记为 SUBLATED（不再删除）
        if self.settlement.settled_cycles:
            sublated = self.settlement.mark_sublated_cycles(
                self.k_active, step=self.total_steps, operation="encounter_purge",
            )
            if sublated > 0:
                active_count = sum(
                    1 for sc in self.settlement.settled_cycles if sc.status == "active"
                )
                print(f"415号: {sublated} ghost settlements marked SUBLATED "
                      f"({active_count} active remain)", file=sys.stderr)

        # Memory node rebuild disabled: settlement records live in JSONL persistence layer,
        # no longer injected into K_active/K_full (memory: vertices were 99% of K_active).
        # Re-sync engine if already initialized; otherwise the first step restores
        # the pre-purge graph from TraversalEngine back onto the daemon.
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

        # Register S_net parameter nodes (442: params as concept nodes)
        self._register_snet_param_nodes()

        # Set watermark to current S_net edge count after bootstrap.
        # Bootstrap edges (corpus/dictionary) are already persisted by their
        # respective ingest paths — only runtime-new edges should be written.
        # Use edge_count if available (SNetLazy) to avoid full edge load.
        if hasattr(self.snet, 'edge_count'):
            self._snet_block_watermark = self.snet.edge_count
        else:
            self._snet_block_watermark = len(self.snet._edges)
        # Set hyperedge watermark similarly
        if hasattr(self.snet, 'hyperedge_count'):
            self._snet_hyperedge_watermark = self.snet.hyperedge_count
        else:
            self._snet_hyperedge_watermark = len(self.snet.hyperedges)

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
        # Use Rust-side max_degree_vertex — single FFI call replaces N*_deg() calls
        if hasattr(self.k_active, '_rg') and hasattr(self.k_active._rg, 'max_degree_vertex'):
            start = self.k_active._rg.max_degree_vertex(list(active))
            if start is None:
                start = next(iter(active))
        else:
            # Fallback for pure-Python Graph
            _aout = self.k_active._adj_out
            _ain = self.k_active._adj_in
            _act = self.k_active._active_ids
            def _deg(v: str) -> int:
                out = sum(1 for e in _aout.get(v, ()) if e.target in _act)
                inc = sum(1 for e in _ain.get(v, ()) if e.source in _act)
                return out + inc
            start = max(active, key=lambda v: (_deg(v), v))

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
        expands mappings via paradigmatic edges and substring matching,
        creates SNetActivation, and sets it on the engine.

        Graceful degradation: if S_net is empty, no activation is created.
        """
        if not self.snet._signifiers or self.engine is None:
            self.snet_activation = None
            return

        concept_to_sig = _build_concept_to_signifier(self.snet, self.k_active)
        sig_to_concepts = _build_signifier_to_concepts(self.snet, self.k_active)

        # Expand mappings via paradigmatic edges + substring matching
        concept_to_sig, sig_to_concepts, expansion_stats = _expand_mappings(
            self.snet, self.k_active, concept_to_sig, sig_to_concepts,
        )

        self.snet_activation = SNetActivation(
            s_net=self.snet,
            concept_to_signifier=concept_to_sig,
            signifier_to_concepts=sig_to_concepts,
        )
        self.engine.set_snet_activation(self.snet_activation)

        n_total = expansion_stats.get("total_signifiers", len(self.snet._signifiers))
        n_before = expansion_stats.get("before", 0)
        n_after = expansion_stats.get("after", 0)
        n_par = expansion_stats.get("paradigmatic_added", 0)
        n_sub = expansion_stats.get("substring_added", 0)
        print(
            f"S_net coupling: {n_after}/{n_total} signifiers mapped "
            f"(base={n_before}, +paradigmatic={n_par}, +substring={n_sub})",
            file=sys.stderr,
        )

    def _bootstrap_snet(self) -> None:
        """Bootstrap S_net from K_active + surface forms data.

        Layer A: K_active vertex content -> signifier nodes
        Layer B: chanlun_surface_forms.jsonl -> syntagmatic edges (PMI weighted)
        Layer C: paradigmatic seeds (chanlun synonym/replacement pairs)
        + Dictionary ingest + Text corpus ingest

        Persistence strategy (priority order):
          1. SQLite (snet_persistence.py) — primary, WAL mode, incremental updates
          2. pickle+gzip (snet_cache.py) — fallback if SQLite fails
          3. Full ingest from source files

        Cache validation uses manifest hash (same as snet_cache.py).

        Graceful degradation: if data file is missing or bootstrap fails,
        self.snet remains an empty SNet and daemon continues normally.
        """
        try:
            import time as _time

            script_dir = Path(os.path.dirname(os.path.abspath(__file__)))
            sf_path = script_dir / "data" / "chanlun_surface_forms.jsonl"
            dict_dir = script_dir / "signifier_net" / "dictionaries"
            corpus_root = script_dir / "signifier_net" / "corpora"

            # Build current manifest for cache validation
            # (reuse snet_cache.build_manifest for consistency)
            current_manifest = None
            try:
                from snet_cache import build_manifest
                current_manifest = build_manifest(
                    dict_dir=dict_dir if dict_dir.is_dir() else None,
                    corpus_root=corpus_root if corpus_root.is_dir() else None,
                    surface_forms_path=sf_path if sf_path.exists() else None,
                )
            except Exception:
                pass

            # --- Try SQLite persistence first (primary) ---
            # SNetLazy mode: load signifiers+morphemes to memory, edges stay in SQLite
            try:
                from snet_persistence import SNetPersistence
                from snet_lazy import SNetLazy
                persistence = SNetPersistence()
                self._snet_persistence = persistence

                db_stats = persistence.stats()
                has_data = db_stats.get("signifiers", 0) > 0

                if has_data:
                    t0 = _time.time()
                    # Lazy load: only signifiers + morphemes to memory
                    lazy_snet = SNetLazy.from_persistence(persistence)
                    self.snet = lazy_snet
                    elapsed = _time.time() - t0
                    n_sigs = len(self.snet._signifiers)
                    n_edges = db_stats.get("edges", 0)

                    manifest_ok = (
                        current_manifest is not None
                        and persistence.manifest_valid(current_manifest)
                    )
                    reason = "match" if manifest_ok else (
                        "missing" if current_manifest is None else "mismatch"
                    )
                    print(
                        f"S_net lazy from SQLite (manifest {reason}): "
                        f"{n_sigs} signifiers, {n_edges} edges "
                        f"({elapsed:.1f}s — edges stay in SQLite)",
                        file=sys.stderr,
                    )
                    # Update manifest to current so next restart is clean
                    if not manifest_ok and current_manifest is not None:
                        persistence.set_manifest(current_manifest)
                    return
                else:
                    print("S_net SQLite: DB empty, proceeding to fallback", file=sys.stderr)
            except Exception as sqlite_exc:
                print(f"S_net SQLite load failed (trying pickle fallback): {sqlite_exc}", file=sys.stderr)

            # --- Try pickle cache as fallback ---
            try:
                from snet_cache import try_load_cached_snet, save_after_full_ingest

                t0 = _time.time()
                cached_snet, used_cache = try_load_cached_snet(
                    dict_dir=dict_dir if dict_dir.is_dir() else None,
                    corpus_root=corpus_root if corpus_root.is_dir() else None,
                    surface_forms_path=sf_path if sf_path.exists() else None,
                    graph=self.k_active,
                )
                if used_cache and cached_snet is not None:
                    elapsed = _time.time() - t0
                    n_sigs = len(cached_snet._signifiers)
                    n_edges = len(cached_snet._edges)
                    print(
                        f"S_net from pickle cache: {n_sigs} signifiers, {n_edges} edges "
                        f"({elapsed:.1f}s — migrating to SQLite lazy mode)",
                        file=sys.stderr,
                    )
                    # Migrate pickle cache to SQLite, then switch to SNetLazy
                    self.snet = cached_snet  # temp: full SNet for migration
                    self._migrate_snet_to_sqlite(current_manifest)
                    # Now switch to lazy mode
                    try:
                        from snet_lazy import SNetLazy
                        if self._snet_persistence is not None:
                            lazy_snet = SNetLazy.from_persistence(
                                self._snet_persistence,
                                signifiers=dict(cached_snet._signifiers),
                                morphemes=dict(cached_snet._morphemes),
                            )
                            self.snet = lazy_snet
                            print(
                                f"S_net switched to lazy mode after pickle→SQLite migration",
                                file=sys.stderr,
                            )
                    except Exception as lazy_exc:
                        print(f"S_net lazy switch failed (keeping full SNet): {lazy_exc}", file=sys.stderr)
                    return
            except Exception as cache_exc:
                print(f"S_net pickle cache failed (proceeding with full ingest): {cache_exc}", file=sys.stderr)

            # --- Full ingest (both caches missed) ---
            t0 = _time.time()
            from snet_bootstrap import bootstrap_snet

            snet, stats = bootstrap_snet(
                graph=self.k_active,
                surface_forms_path=str(sf_path) if sf_path.exists() else None,
                pmi_threshold=0.0,
            )
            self.snet = snet

            # Report
            n_sigs = len(snet._signifiers)
            n_edges = len(snet._edges)
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

            # Dialogue session ingest: CC session text → S_net co-occurrences → block topology
            self._ingest_dialogue_sessions()

            # Ceremony text ingest: genealogy/ceremony text → S_net co-occurrences → block topology
            # (437-2: ceremony output ingest hook + 441: material channel)
            self._ingest_ceremony_texts()

            elapsed = _time.time() - t0
            print(f"S_net full ingest completed in {elapsed:.1f}s", file=sys.stderr)

            # --- Save to SQLite (primary) ---
            try:
                if self._snet_persistence is not None:
                    self._snet_persistence.save_full(self.snet)
                    if current_manifest is not None:
                        self._snet_persistence.set_manifest(current_manifest)
            except Exception as sqlite_save_exc:
                print(f"S_net SQLite save failed (non-fatal): {sqlite_save_exc}", file=sys.stderr)

            # --- Switch to lazy mode after full ingest + SQLite save ---
            try:
                from snet_lazy import SNetLazy
                if self._snet_persistence is not None:
                    lazy_snet = SNetLazy.from_persistence(
                        self._snet_persistence,
                        signifiers=dict(self.snet._signifiers),
                        morphemes=dict(self.snet._morphemes),
                    )
                    self.snet = lazy_snet
                    print(
                        "S_net switched to lazy mode after full ingest",
                        file=sys.stderr,
                    )
            except Exception as lazy_exc:
                print(f"S_net lazy switch failed (keeping full SNet): {lazy_exc}", file=sys.stderr)

            # --- Also save pickle cache (fallback) ---
            try:
                from snet_cache import save_after_full_ingest
                save_after_full_ingest(
                    self.snet,
                    dict_dir=dict_dir if dict_dir.is_dir() else None,
                    corpus_root=corpus_root if corpus_root.is_dir() else None,
                    surface_forms_path=sf_path if sf_path.exists() else None,
                )
            except Exception as save_exc:
                print(f"S_net pickle cache save failed (non-fatal): {save_exc}", file=sys.stderr)

        except Exception as exc:
            print(f"S_net bootstrap failed (graceful degradation): {exc}", file=sys.stderr)
            self.snet = SNet()

    def _migrate_snet_to_sqlite(self, manifest: dict | None) -> None:
        """Migrate S_net from pickle cache to SQLite (one-time migration).

        Called when pickle cache hit but SQLite was empty/stale.
        Non-fatal: if migration fails, SQLite will be populated on next full ingest.
        """
        try:
            if self._snet_persistence is None:
                from snet_persistence import SNetPersistence
                self._snet_persistence = SNetPersistence()
            self._snet_persistence.save_full(self.snet)
            if manifest is not None:
                self._snet_persistence.set_manifest(manifest)
            n_sigs = len(self.snet._signifiers)
            n_edges = len(self.snet._edges)
            print(
                f"S_net migrated pickle→SQLite: {n_sigs} signifiers, {n_edges} edges",
                file=sys.stderr,
            )
        except Exception as exc:
            print(f"S_net pickle→SQLite migration failed (non-fatal): {exc}", file=sys.stderr)

    def _ingest_dictionaries(self) -> None:
        """Ingest dictionary JSONL files into S_net after bootstrap.

        Delegates to signifier_net_ingest.ingest_all_dict_types — the single
        authoritative dict ingest loop shared with corpus_ingest_batch.py.

        Graceful degradation: if dictionaries dir is missing or ingest fails,
        S_net remains unchanged.
        """
        try:
            from signifier_net_ingest import ingest_all_dict_types

            script_dir = Path(os.path.dirname(os.path.abspath(__file__)))
            dict_dir = script_dir / "signifier_net" / "dictionaries"

            self.snet = ingest_all_dict_types(
                self.snet, dict_dir, graph=self.k_active,
            )

            n_sigs = len(self.snet._signifiers)
            n_edges = len(self.snet._edges)
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

        预构建白名单一次，跨所有域复用（避免32域×263K术语排序）。

        Does NOT modify K_active. Text material enriches S_net only — K_active
        modification happens via articulation feedback during traversal.

        Graceful degradation: if corpora dir is missing or ingest fails,
        S_net remains unchanged.
        """
        try:
            from text_corpus_loader import load_text_corpus, format_corpus_report, _split_paragraphs, _SUPPORTED_EXTENSIONS
            from signifier_net_ingest import _build_augmented_whitelist, ingest_text_passage_batch

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

            # 预构建白名单——一次，跨所有域复用
            whitelist, head_to_sid = _build_augmented_whitelist(self.snet)
            sorted_terms = sorted(whitelist, key=len, reverse=True)
            prebuilt = (sorted_terms, head_to_sid)

            total_entries = 0
            for domain_dir in domain_dirs:
                domain = domain_dir.name
                # 收集该域所有段落
                text_files = sorted(
                    f for f in domain_dir.iterdir()
                    if f.is_file() and f.suffix.lower() in _SUPPORTED_EXTENSIONS
                )
                if not text_files:
                    continue

                all_paragraphs: list[str] = []
                source_parts: list[str] = []
                for text_file in text_files:
                    try:
                        text = text_file.read_text(encoding="utf-8")
                        if not text.strip():
                            continue
                        paragraphs = _split_paragraphs(text)
                        if paragraphs:
                            all_paragraphs.extend(paragraphs)
                            source_parts.append(text_file.name)
                    except Exception as exc:
                        print(f"文本读取失败 ({text_file.name}): {exc}", file=sys.stderr)

                if not all_paragraphs:
                    continue

                source = "+".join(source_parts[:5])
                if len(source_parts) > 5:
                    source += f"+...({len(source_parts)} files)"

                self.snet, log_entries = ingest_text_passage_batch(
                    self.snet, all_paragraphs, domain, source,
                    _prebuilt_whitelist=prebuilt,
                )
                if log_entries:
                    report = format_corpus_report(domain, log_entries)
                    print(report, file=sys.stderr)
                    total_entries += len(log_entries)

            if total_entries > 0:
                n_sigs = len(self.snet._signifiers)
                n_edges = len(self.snet._edges)
                print(
                    f"S_net after corpus ingest: {n_sigs} signifiers, {n_edges} edges",
                    file=sys.stderr,
                )

        except Exception as exc:
            print(f"Text corpus ingest failed (graceful degradation): {exc}", file=sys.stderr)

    def _ingest_dialogue_sessions(self) -> None:
        """Ingest CC session dialogue text into S_net + block topology.

        Scans .chanlun/sessions/*.md for dialogue text, extracts semantic
        paragraphs, and feeds them through ingest_text_passage_batch.
        Dedup via state file tracks processed file hashes.

        Graceful degradation: if sessions dir is missing or ingest fails,
        S_net remains unchanged.
        """
        try:
            from dialogue_ingest import ingest_sessions

            script_dir = Path(os.path.dirname(os.path.abspath(__file__)))
            repo_root = script_dir.parent
            sessions_dir = repo_root / ".chanlun" / "sessions"
            state_file = script_dir / "signifier_net" / ".dialogue_ingest_state.json"

            if not sessions_dir.is_dir():
                return

            self.snet, result = ingest_sessions(
                snet=self.snet,
                bt_writer=self._persist,
                sessions_dir=sessions_dir,
                state_file=state_file,
            )

            if result.files_processed > 0:
                print(
                    f"  dialogue ingest: {result.files_processed} files, "
                    f"{result.paragraphs_ingested} paragraphs, "
                    f"{result.cooccurrence_entries} cooccurrences",
                    file=sys.stderr,
                )
            if result.errors:
                for err in result.errors:
                    print(f"  dialogue ingest error: {err}", file=sys.stderr)

        except Exception as exc:
            print(f"Dialogue session ingest failed (graceful degradation): {exc}", file=sys.stderr)

    def _ingest_ceremony_texts(self) -> None:
        """Ingest genealogy/ceremony text into S_net + block topology (437-2 + 441).

        Scans .chanlun/genealogy/ for settled/candidate genealogy .md files,
        extracts semantic paragraphs, and feeds them through ingest_text_passage_batch.

        This is the material channel (441): ceremony language enters S_net as
        linguistic material, changing FengLiang's traversal terrain.

        Graceful degradation: if genealogy dir is missing or ingest fails,
        S_net remains unchanged.
        """
        try:
            from ceremony_ingest import ingest_ceremony_texts

            script_dir = Path(os.path.dirname(os.path.abspath(__file__)))
            repo_root = script_dir.parent
            genealogy_dir = repo_root / ".chanlun" / "genealogy"
            state_file = script_dir / "signifier_net" / ".ceremony_ingest_state.json"

            if not genealogy_dir.is_dir():
                return

            self.snet, result = ingest_ceremony_texts(
                snet=self.snet,
                bt_writer=self._persist,
                genealogy_dir=genealogy_dir,
                state_file=state_file,
            )

            if result.files_processed > 0:
                print(
                    f"  ceremony ingest: {result.files_processed} files, "
                    f"{result.paragraphs_ingested} paragraphs, "
                    f"{result.cooccurrence_entries} cooccurrences",
                    file=sys.stderr,
                )
            if result.errors:
                for err in result.errors:
                    print(f"  ceremony ingest error: {err}", file=sys.stderr)

        except Exception as exc:
            print(f"Ceremony text ingest failed (graceful degradation): {exc}", file=sys.stderr)

    def _register_snet_param_nodes(self) -> None:
        """Register S_net processing parameters as concept-layer nodes (442).

        Each S_net parameter (degree_alpha, pmi_threshold, etc.) becomes a
        Signifier node with source="snet_parameter". FengLiang can negate these
        nodes during traversal to modify its own perception organ.

        Graceful degradation: if registration fails, S_net continues without
        parameter nodes.
        """
        try:
            from snet_params import register_param_nodes

            self.snet, registered = register_param_nodes(self.snet)

            if registered:
                print(
                    f"  S_net param nodes registered: {len(registered)}",
                    file=sys.stderr,
                )
        except Exception as exc:
            print(f"S_net param node registration failed (graceful degradation): {exc}", file=sys.stderr)

    def persist_snet_incremental(
        self,
        new_signifiers: list | None = None,
        new_edges: list | None = None,
        new_morphemes: list | None = None,
    ) -> None:
        """Incrementally persist S_net changes to SQLite.

        Called after any runtime modification to self.snet (e.g., dialogue
        writeback, articulation feedback edges). Uses INSERT OR IGNORE for
        edge dedup via UNIQUE INDEX on (source, target, axis).

        Graceful degradation: if SQLite persistence is not available, silently skips.

        Performance: < 1ms per call (typically 0-3 rows).
        """
        if self._snet_persistence is None:
            return
        try:
            self._snet_persistence.save_incremental(
                new_signifiers=new_signifiers,
                new_edges=new_edges,
                new_morphemes=new_morphemes,
            )
        except Exception as exc:
            print(f"S_net incremental persist failed (non-fatal): {exc}", file=sys.stderr)

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

    def _absorb_process_vertices(self, crystallized_region: set[str]) -> int:
        """Absorb process-state vertices (anti_*, syn_* intermediates) into nearest settled vertex.

        After crystallization, intermediate products from negate (anti_*) and
        sublate (syn_*) that are inside the crystallized region get folded
        into the nearest settled vertex, reducing topological noise.

        Returns the number of absorbed vertices.
        """
        if not crystallized_region:
            return 0

        # Collect settled vertex IDs
        settled_vids: set[str] = set()
        for sc in self.settlement.settled_cycles:
            if sc.status != "active":
                continue
            for src, tgt in sc.edges:
                settled_vids.add(src)
                settled_vids.add(tgt)

        if not settled_vids:
            return 0

        # Find process vertices: anti_* and syn_* prefixed, inside crystallized region
        process_vids = [
            vid for vid in crystallized_region
            if (vid.startswith("anti_") or vid.startswith("syn_"))
            and vid in self.k_active._active_ids
            and vid not in settled_vids
        ]
        if not process_vids:
            return 0

        absorbed = 0
        for vid in process_vids:
            # Find nearest settled vertex: check direct neighbors first
            best_target = None
            neighbors = self.k_active.neighbors(vid)
            for nb in neighbors:
                if nb in settled_vids and nb in self.k_active._active_ids:
                    best_target = nb
                    break

            if best_target is None:
                continue

            # Execute fold: merge process vertex into settled vertex
            result = fold(
                self.k_active, [best_target, vid],
                self.total_steps, self.settlement,
            )
            if not result.blocked:
                self.k_active = result.graph
                # Propagate to k_full
                for e in result.graph.edges:
                    if e.created_at == self.total_steps and not self.k_full.has_edge_key(
                        e.source, e.target, e.edge_type
                    ):
                        self.k_full = self.k_full.add_edge(e)
                absorbed += 1

        if absorbed > 0:
            # Sync back to engine
            self.engine.k_active = self.k_active
            self.engine.k_full = self.k_full
            print(f"[contraction] absorbed {absorbed} process vertices at step {self.total_steps}", file=sys.stderr)

        return absorbed

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
        neighbors = [n for n in self.k_active.neighbors(pos) if not n.startswith("memory:")]
        if not neighbors:
            # Isolated — jump to random concept-layer active vertex
            import random
            concept = [v for v in active if not v.startswith("memory:")]
            return random.choice(concept) if concept else self.engine.position

        # Collect 2-hop neighborhood: neighbors of neighbors
        neighbor_set = set(neighbors)  # build once, not per-iteration
        two_hop: set[str] = set()
        for nb in neighbors:
            for nb2 in self.k_active.neighbors(nb):
                if nb2 != pos and nb2 not in neighbor_set and not nb2.startswith("memory:"):
                    two_hop.add(nb2)

        if not two_hop:
            # No 2-hop — use neighbors themselves
            two_hop = set(neighbors)

        # Pick the vertex with highest f value relative to current position
        # High f = structurally distant = most interesting jump target
        best_vid = pos
        best_f = -1
        import random
        sample = random.sample(list(two_hop), min(20, len(two_hop)))
        for vid in sample:
            f_val = self.engine._compute_f(pos, vid)
            if f_val > best_f:
                best_f = f_val
                best_vid = vid

        return best_vid

    def _compute_local_f_terrain(self) -> float:
        """Average f value of current position's neighbors.

        Uses traversal engine's _compute_f. Returns 0.0 if no neighbors.
        Samples up to 10 neighbors when count exceeds 10 for O(degree) budget.
        """
        pos = self.engine.position
        neighbors = self.k_active.neighbors(pos)
        if not neighbors:
            return 0.0
        import random
        sample = random.sample(neighbors, min(10, len(neighbors)))
        f_values = [self.engine._compute_f(pos, nb) for nb in sample]
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

        # Capture pre-step state for persistence diff and SharedLayer sync
        # Use count-based diff (O(1)) instead of set-based diff (O(E))
        # Graph.add_edge appends to _edges list, so new edges are always at tail
        _need_diff = self._persist or self._shared_layer is not None
        if _need_diff:
            pre_vid_count = len(self.k_full._vertices)
            pre_edge_count = len(self.k_full._edges)
            pre_settled_count = len(self.settlement.settled_cycles)
            pre_vid_keys = set(self.k_full._vertices.keys())  # snapshot: mutable Graph needs copy
            # Track K_active vertex statuses to detect fold state changes
            pre_active_statuses = {
                vid: v.status for vid, v in self.k_active.vertices.items()
            }

        log = self.engine.run_step()

        # Sync graph state from engine
        self.k_active = self.engine.k_active
        self.k_full = self.engine.k_full
        self.terrain = self.engine.terrain

        # Sync S_net from engine (traversal co-occurrence writeback may have updated it)
        if self.snet_activation is not None and self.snet_activation.s_net is not self.snet:
            self.snet = self.snet_activation.s_net

        # S_net → block topology: write new co-occurrence edges as material layer blocks
        self._write_snet_cooccurrence_blocks()

        # Settlement memory node injection disabled: settlement records live in JSONL
        # persistence layer, no longer injected into K_active/K_full.
        # Function definitions preserved in encounter_log.py for future contraction mechanism.

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
        # Compute every 100 steps to avoid O(N*E) cost per step on large graphs
        # (125K+ edges makes per-step computation infeasible)
        if self.total_steps % 100 == 0:
            local_f = self._compute_local_f_terrain()
            self._local_f_history.append(local_f)
        elif not self._local_f_history:
            self._local_f_history.append(0.0)

        # Crystallization + jump logic
        if self._is_locally_crystallized():
            self._crystallization_count += 1

            # Contraction: absorb process vertices in crystallized region
            # Crystallized region = current position's 1-hop neighborhood
            pos = self.engine.position
            crystallized_region = set(self.k_active.neighbors(pos))
            crystallized_region.add(pos)
            self._absorb_process_vertices(crystallized_region)

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

        # Compute graph diffs using count-based tail slice — O(new) instead of O(E)
        new_vids: set[str] = set()
        new_edges_list: list = []
        new_settled: list = []
        if _need_diff:
            # New vertices: keys not in pre snapshot
            for vid in self.k_full._vertices:
                if vid not in pre_vid_keys:
                    new_vids.add(vid)
            # New edges: tail slice (Graph.add_edge appends)
            new_edges_list = self.k_full._edges[pre_edge_count:]
            new_settled = self.settlement.settled_cycles[pre_settled_count:]

        # Persist graph state changes (always, not just for significant events)
        if self._persist:
            for vid in new_vids:
                v = self.k_full.vertices.get(vid)
                if v is not None:
                    self._persist.append_vertex(v)
            # Persist ARTICULATED edges with full provenance via append_articulation
            art_meta = self.engine._last_articulation
            for e in new_edges_list:
                if e.edge_type == EdgeType.ARTICULATED and art_meta is not None \
                        and e.source == art_meta["source_vid"] \
                        and e.target == art_meta["target_vid"]:
                    self._persist.append_articulation(
                        source_vid=art_meta["source_vid"],
                        target_vid=art_meta["target_vid"],
                        score=art_meta["score"],
                        step=art_meta["step"],
                        imbalance_type=art_meta["imbalance_type"],
                        reason=art_meta["reason"],
                        timestamp=datetime.utcnow().isoformat() + "Z",
                    )
                else:
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

        # Persist operation log for every step (not just significant events)
        # Full step record: operation + position + beta_1 + delta + encounter
        if self._persist:
            self._persist.append_operation(log.step, log.operation, {
                "position": log.position,
                "encounter": log.encounter or "",
                "beta_1_before": log.beta_1_before,
                "beta_1_after": log.beta_1_after,
                "delta_beta_1": log.delta_beta_1,
                "blocked": log.blocked,
                "vertices_active": log.vertices_active,
                "edges_active": log.edges_active,
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

            # Compute jaccard similarity between position and encounter target
            jaccard_sim = -1.0
            if log.encounter and log.position != log.encounter:
                nbs_a = set(self.k_active.neighbors(log.position))
                nbs_b = set(self.k_active.neighbors(log.encounter))
                union = nbs_a | nbs_b
                if union:
                    jaccard_sim = len(nbs_a & nbs_b) / len(union)

            self.encounter_log.record_encounter(
                concept_a=pos_name,
                concept_b=encounter_name,
                encounter_type=log.operation,
                f_value=log.f_value,
                g_value=log.g_value,
                jaccard_similarity=jaccard_sim,
                context=narrative,
                beta_1_before=log.beta_1_before,
                beta_1_after=log.beta_1_after,
                step=log.step,
                graph_id=self._instance_id,
                session_id=self._session_id,
            )

            # 401号修复：不再将 encounter 注入 K_active。
            # encounter（fold/sublate/negate/blocked）是穿越轨迹，不是结构产出。
            # "脚印不是宝藏" — 只有 settlement 和 residue 进入 K_active。

            # Write to traversal checkpoint encounter log
            self._checkpoint.encounters.record(
                step=log.step, operation=log.operation, position=log.position,
                beta_1_before=log.beta_1_before, beta_1_after=log.beta_1_after,
                f_value=log.f_value, g_value=log.g_value,
                blocked=log.blocked, context=narrative,
            )

        # Gap detection (topology-change driven)
        if self._should_check_gaps():
            gaps = self._detect_gaps()
            for gap in gaps:
                self._fire("on_gap", gap)
                self.total_gaps_detected += 1
            self._last_gap_check_step = self.total_steps
            self._cumulative_delta_beta_1 = 0.0

        # Plan C: write sync blocks to SharedLayer (each method self-throttles)
        if self._shared_layer is not None:
            self._write_traversal_position(log)
            self._write_graph_delta(log, new_vids, new_edges_list)
            self._write_settlement_event(new_settled)
            self._write_snet_update()

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

    def _write_graph_delta(self, log, new_vids: set[str], new_edges: set) -> None:
        """Write graph delta block to SharedLayer on significant events.

        Only fires on fold/negate/sublate — the operations that change topology.
        Throttled: at most once per 3 seconds (unless forced by sublate).
        Includes new vertices and edges created by this step.
        """
        if not log.operation in ("fold", "negate", "sublate"):
            return
        if not new_vids and not new_edges:
            return

        now = time.time()
        elapsed = now - self._last_graph_delta_write_time
        if elapsed < 3.0 and log.operation != "sublate":
            return

        vertices_data = []
        for vid in new_vids:
            v = self.k_active.vertex(vid)
            if v is not None:
                vertices_data.append({
                    "id": v.id,
                    "status": v.status.value,
                    "content": v.content,
                    "created_at": v.created_at,
                })

        edges_data = []
        for e in new_edges:
            edges_data.append({
                "source": e.source,
                "target": e.target,
                "edge_type": e.edge_type.value,
                "created_at": e.created_at,
            })

        block = {
            "type": "graph_delta",
            "instance": self._instance_id,
            "step": self.total_steps,
            "timestamp": now,
            "operation": log.operation,
            "position": log.position,
            "encounter": log.encounter or "",
            "f_value": log.f_value,
            "vertices": vertices_data,
            "edges": edges_data,
        }
        try:
            block_hash = self._shared_layer.write_block(block)
            self._cross_instance_sync.known_blocks.add(block_hash)
            self._last_graph_delta_write_time = now
        except Exception:
            pass  # graceful degradation: log failure, don't crash

    def _write_settlement_event(self, new_settled: list) -> None:
        """Write settlement event blocks to SharedLayer for newly settled cycles.

        One block per newly settled cycle. No throttle — settlements are rare
        and each one is significant.
        """
        if not new_settled:
            return

        now = time.time()
        for sc in new_settled:
            block = {
                "type": "settlement_event",
                "instance": self._instance_id,
                "step": self.total_steps,
                "timestamp": now,
                "settled_at_step": sc.settled_at_step,
                "cycle_edges": sorted(sc.edges),
                "residue": sc.residue or [],
            }
            try:
                block_hash = self._shared_layer.write_block(block)
                self._cross_instance_sync.known_blocks.add(block_hash)
            except Exception:
                pass

    def _write_snet_update(self) -> None:
        """Write S_net activation state to SharedLayer when activation set changes.

        Throttled: at most once per 5 seconds AND only when the active set changed.
        """
        if self.snet_activation is None:
            return

        current_active = frozenset(self.snet_activation.currently_active)
        if current_active == self._last_snet_active_snapshot:
            return

        now = time.time()
        elapsed = now - self._last_snet_write_time
        if elapsed < 5.0:
            return

        block = {
            "type": "snet_update",
            "instance": self._instance_id,
            "step": self.total_steps,
            "timestamp": now,
            "currently_active": sorted(current_active),
            "dialogue_focus_set": sorted(self.snet_activation._dialogue_focus_set),
            "edge_suggestions": [
                s.to_dict() for s in self.snet_activation.edge_suggestions[-10:]
            ],
            "internal_speech_buffer": [
                f.to_dict() for f in self.snet_activation.internal_speech_buffer[-5:]
            ],
        }
        try:
            block_hash = self._shared_layer.write_block(block)
            self._cross_instance_sync.known_blocks.add(block_hash)
            self._last_snet_write_time = now
            self._last_snet_active_snapshot = set(current_active)
        except Exception:
            pass

    def _write_snet_cooccurrence_blocks(self) -> None:
        """Write new S_net co-occurrence edges and hyperedges to block topology as material layer blocks.

        Incremental: for SNetLazy, uses drain_runtime_new_edges() buffer.
        For full SNet, uses _snet_block_watermark to track the last-written edge index,
        and _snet_hyperedge_watermark for hyperedges.

        Source type classification via evidence field prefix:
          - "traversal:{step}" → source_type "traversal"
          - "dialogue:{...}"  → source_type "dialogue"
          - everything else   → source_type "corpus"

        Throttled: batch writes at most 200 edges and 100 hyperedges per step.
        """
        if self._persist is None:
            return

        timestamp = datetime.utcnow().isoformat() + "Z"

        # --- Edge writing (existing logic) ---
        # SNetLazy: use runtime buffer (no full edge load)
        from snet_lazy import SNetLazy
        if isinstance(self.snet, SNetLazy):
            new_edges = self.snet.drain_runtime_new_edges()
            if new_edges:
                batch = new_edges[:200]
                # Put remaining edges back if batch was capped
                if len(new_edges) > 200:
                    self.snet._runtime_new_edges = new_edges[200:] + self.snet._runtime_new_edges

                for edge in batch:
                    if edge.axis != AxisType.SYNTAGMATIC:
                        continue
                    evidence = edge.evidence or ""
                    if evidence.startswith("traversal:"):
                        source_type = "traversal"
                    elif evidence.startswith("dialogue:"):
                        source_type = "dialogue"
                    else:
                        source_type = "corpus"
                    self._persist.append_cooccurrence(
                        source_signifier=edge.source,
                        target_signifier=edge.target,
                        weight=edge.weight,
                        corpus_source=source_type,
                        ingest_params={
                            "evidence": evidence,
                            "relation": edge.relation,
                        },
                        timestamp=timestamp,
                    )
        else:
            current_edges = self.snet._edges
            current_count = len(current_edges)
            watermark = self._snet_block_watermark
            if current_count > watermark:
                new_edges = current_edges[watermark:]
                batch = new_edges[:200]
                self._snet_block_watermark = watermark + len(batch)

                for edge in batch:
                    if edge.axis != AxisType.SYNTAGMATIC:
                        continue
                    evidence = edge.evidence or ""
                    if evidence.startswith("traversal:"):
                        source_type = "traversal"
                    elif evidence.startswith("dialogue:"):
                        source_type = "dialogue"
                    else:
                        source_type = "corpus"
                    self._persist.append_cooccurrence(
                        source_signifier=edge.source,
                        target_signifier=edge.target,
                        weight=edge.weight,
                        corpus_source=source_type,
                        ingest_params={
                            "evidence": evidence,
                            "relation": edge.relation,
                        },
                        timestamp=timestamp,
                    )

        # --- Hyperedge writing (v243 新增) ---
        current_hyperedges = self.snet.hyperedges
        he_count = len(current_hyperedges)
        he_watermark = self._snet_hyperedge_watermark
        if he_count > he_watermark:
            new_hes = current_hyperedges[he_watermark:]
            he_batch = new_hes[:100]
            self._snet_hyperedge_watermark = he_watermark + len(he_batch)

            for he in he_batch:
                self._persist.append_hyperedge(
                    vertices=sorted(he.vertices),
                    source=he.source,
                    domain=he.domain,
                    timestamp=timestamp,
                    evidence_tag=he.evidence_tag,
                    ingest_param_refs=list(he.ingest_param_refs) if he.ingest_param_refs else None,
                )

    def ingest_code(self, dirpath: str, source: str = "") -> Graph:
        """Ingest Python source tree into K_active with domain:code tagging.

        Unlike feed() which uses phi_L for text, this uses code_ingest
        for AST-based parsing. Vertices carry [domain:code] content prefix.

        S_net 界面原则：代码摄入后，将代码顶点的 content 回写 S_net 共现边，
        确保所有外部输入都经过 S_net。K_active 注入是代码图的结构需求（AST 关系），
        S_net 回写是语言材料的摄入（函数名/类名作为能指）。

        Args:
            dirpath: Root directory to scan for .py files.
            source: Source label prefix for vertex IDs (e.g. "NewChanlun").
        """
        from code_ingest import ingest_tree
        code_graph = ingest_tree(dirpath, source=source)
        self._inject(code_graph)
        # S_net 回写：将代码顶点的 content 作为语言材料摄入 S_net
        if self.snet and self.snet._signifiers:
            try:
                from signifier_net import writeback_from_text
                code_contents = [
                    v.content for v in code_graph.vertices.values()
                    if v.content and not v.content.startswith("[")
                ]
                if code_contents:
                    combined_text = "\n".join(code_contents[:100])  # cap to avoid flooding
                    new_snet, _log = writeback_from_text(
                        snet=self.snet,
                        text=combined_text,
                        source_label=f"code_ingest:{source or 'unknown'}",
                    )
                    self.snet = new_snet
            except Exception:
                pass  # S_net writeback failure is non-fatal
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

        S_net 界面原则审计：此操作是**K_active 内部拓扑发现**——在已有顶点之间
        检测跨域语义关系并添加边。不引入外部输入，不创建新概念。
        属于穿越引擎的内在行为（类似 fold/negate 修改 K_active），合法直接写入。
        """
        current_vids = set(self.k_active.active_vertex_ids())
        new_vids = current_vids - self._cross_domain_scanned_vids
        # 类型约束：memory: 前缀顶点是 settlement 产物，不参与 cross-domain 扫描
        new_vids = {v for v in new_vids if not v.startswith("memory:")}

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
        existing_edges = set(self.k_active.edge_keys)  # mutable copy for .add()
        new_edges: list = []
        for edge, _score in candidates:
            if len(new_edges) >= 10:
                break
            key = (edge.source, edge.target, edge.edge_type)
            if key not in existing_edges:
                if (self.k_active.vertex(edge.source) is not None
                        and self.k_active.vertex(edge.target) is not None):
                    new_edges.append(edge)
                    existing_edges.add(key)
                    # Persistence handled by _step diff logic (998-1016)
                    # to avoid double-write (was root cause of 3847x duplication bug)
        if new_edges:
            self.k_active = self.k_active.add_vertices_and_edges_batch([], new_edges)
            self.k_full = self.k_full.add_vertices_and_edges_batch([], new_edges)

        self._cross_domain_scanned_vids = current_vids

    def _inject(self, sub_graph: Graph) -> None:
        """Inject a sub-graph into K_active. Merge by vertex ID.

        S_net 界面原则：此方法是底层注入工具，调用者负责确保外部输入
        已通过 S_net 回写。内部拓扑操作（settlement memory, proprioception 等）
        可直接调用。外部摄入（如 ingest_code）必须在调用前/后回写 S_net。
        """
        existing_vids = set(self.k_active.active_vertex_ids())
        existing_edges = set(self.k_active.edge_keys)  # mutable copy for .add()

        new_vertices: list = []
        for vid in sub_graph.active_vertex_ids():
            if vid not in existing_vids:
                new_vertices.append(sub_graph.vertex(vid))
                # Persistence handled by _step diff logic (998-1016)
                # to avoid double-write (was root cause of 3847x duplication bug)

        if new_vertices:
            self.k_active = self.k_active.add_vertices_and_edges_batch(new_vertices, [])
            self.k_full = self.k_full.add_vertices_and_edges_batch(new_vertices, [])

        new_edges: list = []
        for e in sub_graph.active_edges():
            key = (e.source, e.target, e.edge_type)
            if key not in existing_edges:
                if (self.k_active.vertex(e.source) is not None
                        and self.k_active.vertex(e.target) is not None):
                    new_edges.append(e)
                    existing_edges.add(key)
                    # Persistence handled by _step diff logic (998-1016)
                    # to avoid double-write (was root cause of 3847x duplication bug)

        if new_edges:
            self.k_active = self.k_active.add_vertices_and_edges_batch([], new_edges)
            self.k_full = self.k_full.add_vertices_and_edges_batch([], new_edges)

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
        """Close persistence handle. Dump snapshot + truncate JSONL if persisting."""
        if hasattr(self, '_checkpoint'):
            self._checkpoint.save_state(extract_daemon_state(self))
            self._checkpoint.close()
        # Snapshot: dump current k_full state, then truncate incremental JSONL
        if self._persist_path is not None:
            snapshot_p = PersistentKFull.snapshot_path_for(self._persist_path)
            try:
                count = PersistentKFull.dump_snapshot(self.k_full, snapshot_p)
                print(
                    f"Snapshot dumped: {count} records to {snapshot_p}",
                    file=sys.stderr,
                )
                # Close JSONL handle before truncating
                if self._persist:
                    self._persist.close()
                # Truncate incremental JSONL — snapshot has the full state
                self._persist_path.write_text("", encoding="utf-8")
                print(
                    f"JSONL truncated: {self._persist_path}",
                    file=sys.stderr,
                )
                self._persist = None  # already closed
            except Exception as e:
                print(
                    f"Snapshot dump failed: {e} — JSONL preserved",
                    file=sys.stderr,
                )
        if self._persist:
            self._persist.close()
        if hasattr(self, '_snet_persistence') and self._snet_persistence is not None:
            self._snet_persistence.close()


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

    # If persisting with a fresh topology (no recovery), write initial graph
    if args.persist and daemon._persist and graph is not None:
        bt_check, _ = load_graph_from_block_topology(DAEMON_BT_BASE)
        if not bt_check.active_vertex_ids():
            # Truncate JSONL backup before writing initial graph to prevent
            # cumulative duplication across restarts (3847x bug root cause #3)
            jsonl_path = Path(args.persist)
            if jsonl_path.exists() and jsonl_path.stat().st_size > 0:
                print(
                    f"Block topology empty but JSONL exists ({jsonl_path.stat().st_size} bytes). "
                    f"Truncating stale JSONL before fresh write.",
                    file=sys.stderr,
                )
                # Close, truncate, reopen
                daemon._persist.close()
                jsonl_path.write_text("", encoding="utf-8")
                daemon._persist.open()
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

    # Register auto-feed (always active in serve mode)
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
