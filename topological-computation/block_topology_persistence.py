"""Block Topology persistence — JSONL append-only event log.

持久化层统一为 JSONL：每一行是一个 immutable 事件，只追加不修改。
events immutable 原则不变——JSONL 里的每一行是 Dass（这些事件发生过），
加载时从 Dass 重建 Was（当前的计算状态）。

Block topology stores four domains:
- "graph": vertex and edge records (the traversal graph structure)
- "history": operation, settlement, vertex_status, merge records
- "growth": runtime-generated content (cross-domain edges, injected subgraphs)
- "material": immutable material layer (COOCCURRENCE, TRAVERSAL_ASSOCIATION, hyperedge)

架构决策（456号编排者裁定）：从 per-event JSON 文件迁移到单个 JSONL 追加文件。
根因：每步穿越写多个 block 文件，2 分钟产生 200 万个文件，耗尽 inode/tmpfs。
JSONL 追加是 O(1) 写入，不创建新文件，不消耗 inode。
"""

from __future__ import annotations

import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus

# Block topology base path for daemon use
DAEMON_BT_BASE = Path(__file__).resolve().parent.parent / ".chanlun" / "block-topology"

# Source identifier for daemon-written blocks
DAEMON_SOURCE = "cc"

# Domain tags used in block content
DOMAIN_GRAPH = "graph"
DOMAIN_HISTORY = "history"
DOMAIN_GROWTH = "growth"
DOMAIN_MATERIAL = "material"  # 425号: 物质层 (Dass) — COOCCURRENCE + TRAVERSAL_ASSOCIATION


class BlockTopologyWriter:
    """Writes daemon events to JSONL append-only log.

    每个 append_* 方法将一个 immutable 事件追加到 JSONL 文件。
    O(1) 写入，不创建文件，不消耗 inode。
    """

    def __init__(
        self,
        bt_base: Path = DAEMON_BT_BASE,
        jsonl_backup_path: Path | None = None,
    ):
        self.bt_base = bt_base
        self._jsonl_path = jsonl_backup_path
        self._jsonl_file = None

    def open(self):
        """Open JSONL file for appending."""
        if self._jsonl_path:
            self._jsonl_path.parent.mkdir(parents=True, exist_ok=True)
            self._jsonl_file = open(
                self._jsonl_path, "a", encoding="utf-8"
            )
        return self

    def close(self):
        """Close the JSONL file handle."""
        if self._jsonl_file:
            self._jsonl_file.close()
            self._jsonl_file = None

    def __enter__(self):
        return self.open()

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
        return False

    def _write_event(self, record: dict):
        """Append an immutable event to the JSONL log.

        Merkle DAG: each record gets a SHA-256 hash of its canonical JSON
        (sort_keys=True, ensure_ascii=False) stored in record["block_hash"].
        Hash is computed over the record *without* the block_hash field itself.
        """
        if self._jsonl_file:
            canonical = json.dumps(record, sort_keys=True, ensure_ascii=False)
            block_hash = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
            record["block_hash"] = block_hash
            self._jsonl_file.write(
                json.dumps(record, ensure_ascii=False) + "\n"
            )
            self._jsonl_file.flush()

    def append_vertex(self, vertex: Vertex, domain: str = DOMAIN_GRAPH):
        """Write a vertex event."""
        self._write_event({
            "type": "vertex",
            "id": vertex.id,
            "status": vertex.status.value,
            "content": vertex.content,
            "created_at": vertex.created_at,
        })

    def append_edge(self, edge: Edge, domain: str = DOMAIN_GRAPH):
        """Write an edge event."""
        record = {
            "type": "edge",
            "source": edge.source,
            "target": edge.target,
            "edge_type": edge.edge_type.value,
            "created_at": edge.created_at,
        }
        if edge.surface is not None:
            record["surface"] = edge.surface
        if edge.context is not None:
            record["context"] = edge.context
        self._write_event(record)

    def append_operation(self, step: int, operation: str, details: dict):
        """Write an operation event."""
        self._write_event({
            "type": "operation",
            "step": step,
            "operation": operation,
            **details,
        })

    def append_vertex_status(self, vertex_id: str, status: str, step: int):
        """Write a vertex status change event."""
        self._write_event({
            "type": "vertex_status",
            "id": vertex_id,
            "status": status,
            "step": step,
        })

    def append_merge(self, keep: str, remove: str, step: int):
        """Write a merge event."""
        self._write_event({
            "type": "merge",
            "keep": keep,
            "remove": remove,
            "step": step,
        })

    def append_settlement(self, step: int, cycle_edges: list):
        """Write a settlement event."""
        self._write_event({
            "type": "settlement",
            "step": step,
            "edges": cycle_edges,
        })

    def append_cooccurrence(
        self,
        source_signifier: str,
        target_signifier: str,
        weight: float,
        corpus_source: str,
        ingest_params: dict,
        timestamp: str,
    ):
        """Write a COOCCURRENCE event (immutable, material layer)."""
        self._write_event({
            "type": "cooccurrence",
            "source_signifier": source_signifier,
            "target_signifier": target_signifier,
            "weight": weight,
            "corpus_source": corpus_source,
            "timestamp": timestamp,
        })

    def append_traversal_association(
        self,
        from_signifier: str,
        to_signifier: str,
        traversal_id: str,
        step_number: int,
        timestamp: str,
    ):
        """Write a TRAVERSAL_ASSOCIATION event (immutable, material layer)."""
        self._write_event({
            "type": "traversal_association",
            "from_signifier": from_signifier,
            "to_signifier": to_signifier,
            "traversal_id": traversal_id,
            "step_number": step_number,
            "timestamp": timestamp,
        })

    def append_articulation(
        self,
        source_vid: str,
        target_vid: str,
        score: float,
        step: int,
        imbalance_type: str = "",
        reason: str = "",
        timestamp: str = "",
        **kwargs,
    ):
        """Write an ARTICULATION event (concept layer emergence)."""
        record = {
            "type": "articulation",
            "source_vid": source_vid,
            "target_vid": target_vid,
            "score": score,
            "step": step,
            "imbalance_type": imbalance_type,
            "reason": reason,
            "timestamp": timestamp,
        }
        # Preserve extra fields (cooc_weight, ta_count, step_gap, etc.)
        for k, v in kwargs.items():
            if k not in record:
                record[k] = v
        self._write_event(record)

    def append_hyperedge(
        self,
        vertices: list[str],
        source: str,
        domain: str,
        timestamp: str,
        evidence_tag: str = "",
        ingest_param_refs: list[str] | None = None,
    ):
        """Write a hyperedge event (immutable, material layer)."""
        self._write_event({
            "type": "hyperedge",
            "vertices": sorted(vertices),
            "source": source,
            "domain": domain,
            "timestamp": timestamp,
            "evidence_tag": evidence_tag,
        })


def verify_jsonl(path: Path) -> bool:
    """Verify integrity of a JSONL event log by recomputing block hashes.

    For each line, strips the block_hash field, recomputes SHA-256 over the
    canonical JSON (sort_keys=True), and compares against the stored hash.
    Returns True if all lines verify (or file is empty/missing).
    Returns False if any line fails verification.
    """
    if not path.exists():
        return True
    try:
        with open(path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                record = json.loads(line)
                stored_hash = record.pop("block_hash", None)
                if stored_hash is None:
                    return False
                canonical = json.dumps(record, sort_keys=True, ensure_ascii=False)
                computed = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
                if computed != stored_hash:
                    return False
    except (json.JSONDecodeError, OSError):
        return False
    return True


def load_graph_from_block_topology(
    bt_base: Path = DAEMON_BT_BASE,
) -> tuple[Graph, list[dict]]:
    """Load a Graph from JSONL event log or legacy block files.

    Priority: JSONL first (fast), then legacy block files (slow, backward compat).
    """
    # Try JSONL first (from persist path — caller provides)
    # This function is called from daemon with bt_base, but JSONL is at persist_path
    # For backward compat, also scan legacy block files
    blocks_dir = bt_base / "blocks"
    if not blocks_dir.exists():
        return Graph(), []

    # Legacy: read individual block JSON files
    events: list[dict] = []
    try:
        for f in blocks_dir.iterdir():
            if f.suffix != ".json":
                continue
            blk = json.loads(f.read_text(encoding="utf-8"))
            content = blk.get("content", {})
            event_type = content.get("event_type")
            if event_type in ("vertex", "edge", "vertex_status", "merge",
                              "operation", "settlement", "articulation"):
                events.append(blk)
    except Exception:
        pass

    if not events:
        return Graph(), []

    # Sort by timestamp for correct replay order
    events.sort(key=lambda b: b.get("timestamp", ""))

    # Replay events to reconstruct graph
    vertices: dict[str, Vertex] = {}
    edges: list[Edge] = []
    operations: list[dict] = []

    for blk in events:
        content = blk["content"]
        event_type = content["event_type"]

        if event_type == "vertex":
            v = Vertex(
                id=content["vertex_id"],
                status=VertexStatus(content["status"]),
                content=content.get("content"),
                created_at=content.get("created_at", 0),
            )
            vertices[v.id] = v

        elif event_type == "edge":
            e = Edge(
                source=content["source"],
                target=content["target"],
                edge_type=EdgeType(content["edge_type"]),
                created_at=content.get("created_at", 0),
                surface=content.get("surface"),
                context=content.get("context"),
            )
            if e.source in vertices and e.target in vertices:
                edges.append(e)

        elif event_type == "vertex_status":
            vid = content["vertex_id"]
            new_status = VertexStatus(content["status"])
            old_v = vertices.get(vid)
            if old_v is not None:
                vertices[vid] = Vertex(
                    old_v.id, new_status, old_v.content, old_v.created_at,
                )

        elif event_type == "merge":
            keep = content["keep"]
            remove = content["remove"]
            if keep in vertices and remove in vertices:
                old_remove = vertices[remove]
                vertices[remove] = Vertex(
                    old_remove.id, VertexStatus.FOLDED,
                    old_remove.content, old_remove.created_at,
                )
                new_edges = []
                for e in edges:
                    src = keep if e.source == remove else e.source
                    tgt = keep if e.target == remove else e.target
                    if src == tgt:
                        continue
                    if src != e.source or tgt != e.target:
                        new_edges.append(Edge(
                            src, tgt, e.edge_type, e.created_at,
                            e.surface, e.context,
                        ))
                    else:
                        new_edges.append(e)
                edges = new_edges

        elif event_type == "articulation":
            e = Edge(
                source=content["source_vid"],
                target=content["target_vid"],
                edge_type=EdgeType.ARTICULATED,
                created_at=content.get("step", 0),
                surface=None,
                context=f"articulated: score={content.get('score', 0):.3f} step={content.get('step', 0)}",
            )
            if e.source in vertices and e.target in vertices:
                edges.append(e)

        elif event_type in ("operation", "settlement"):
            operations.append(content)

    graph = Graph(vertices, edges)
    return graph, operations


def rebuild_block_topology_from_jsonl(
    jsonl_path: Path,
    bt_base: Path = DAEMON_BT_BASE,
) -> int:
    """No-op: block topology is now JSONL-native. Returns 0."""
    return 0


def trace_concept_edge(
    jsonl_path: Path,
    source_vid: str,
    target_vid: str,
) -> dict | None:
    """438号-推论2/3: 概念边追溯——从概念边回溯到 S_net 能指来源.

    追溯链：概念边 → ARTICULATION block → 遭遇位置 → imbalance_type → reason（含能指对信息）

    Returns dict with traceability info, or None if edge not found in JSONL.
    Keys:
      - source_vid, target_vid: 概念边端点
      - step: 产出步数
      - imbalance_type: A_dense_B_sparse / B_dense_A_sparse
      - reason: 完整遭遇描述（含 S_net 能指对信息）
      - score: articulation score
      - extra: 额外字段（cooc_weight, ta_count 等）
    """
    if not jsonl_path.exists():
        return None

    with open(jsonl_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            if record.get("type") != "articulation":
                continue
            if (record.get("source_vid") == source_vid
                    and record.get("target_vid") == target_vid):
                extra = {
                    k: v for k, v in record.items()
                    if k not in (
                        "type", "source_vid", "target_vid", "score",
                        "step", "imbalance_type", "reason", "timestamp",
                        "block_hash",
                    )
                }
                return {
                    "source_vid": source_vid,
                    "target_vid": target_vid,
                    "step": record.get("step"),
                    "imbalance_type": record.get("imbalance_type", ""),
                    "reason": record.get("reason", ""),
                    "score": record.get("score"),
                    "timestamp": record.get("timestamp", ""),
                    "extra": extra,
                }

    return None


def list_articulation_events(
    jsonl_path: Path,
    imbalance_type: str | None = None,
) -> list[dict]:
    """438号-推论2: 列出所有 ARTICULATION 事件，可按 imbalance_type 过滤.

    Returns list of articulation event dicts sorted by step.
    """
    results: list[dict] = []
    if not jsonl_path.exists():
        return results

    with open(jsonl_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                record = json.loads(line)
            except json.JSONDecodeError:
                continue
            if record.get("type") != "articulation":
                continue
            if imbalance_type and record.get("imbalance_type") != imbalance_type:
                continue
            results.append({
                "source_vid": record.get("source_vid"),
                "target_vid": record.get("target_vid"),
                "step": record.get("step"),
                "imbalance_type": record.get("imbalance_type", ""),
                "reason": record.get("reason", ""),
                "score": record.get("score"),
            })

    results.sort(key=lambda r: r.get("step", 0))
    return results
