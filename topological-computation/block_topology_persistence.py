"""Block Topology persistence — replaces k_full.jsonl as primary storage.

k_full.jsonl is demoted to a synchronous backup: every write goes to
block topology first, then appends to jsonl as a crash-recovery fallback.

Block topology stores three domains:
- "graph": vertex and edge records (the traversal graph structure)
- "history": operation, settlement, vertex_status, merge records
- "growth": runtime-generated content (cross-domain edges, injected subgraphs)

All writes are atomic at the individual event level: one event = one block + relations.
"""

from __future__ import annotations

import json
import sys
from datetime import datetime, timezone
from pathlib import Path

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus

# Import block topology core (from project scripts/)
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
from block_topology import (
    make_block,
    write_block,
    make_relation,
    append_relation,
    read_block,
    list_blocks,
    read_all_relations,
    BLOCK_TYPES,
    SOURCES,
    DEFAULT_BASE,
)

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
    """Writes daemon events to block topology + jsonl backup.

    Replaces PersistentKFull's append methods. Each event creates:
    - A block in .chanlun/block-topology/blocks/
    - Appropriate relations in relations.jsonl
    - A backup line in the jsonl file (if provided)
    """

    def __init__(
        self,
        bt_base: Path = DAEMON_BT_BASE,
        jsonl_backup_path: Path | None = None,
    ):
        self.bt_base = bt_base
        self._jsonl_backup_path = jsonl_backup_path
        self._jsonl_file = None

        # Ensure block topology dirs exist
        (self.bt_base / "blocks").mkdir(parents=True, exist_ok=True)

        # Track written block ids for relation linking
        self._vertex_block_ids: dict[str, str] = {}  # vertex_id -> block_id

    def open(self):
        """Open jsonl backup file for appending."""
        if self._jsonl_backup_path:
            self._jsonl_backup_path.parent.mkdir(parents=True, exist_ok=True)
            self._jsonl_file = open(
                self._jsonl_backup_path, "a", encoding="utf-8"
            )
        return self

    def close(self):
        """Close the jsonl backup file handle."""
        if self._jsonl_file:
            self._jsonl_file.close()
            self._jsonl_file = None

    def __enter__(self):
        return self.open()

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
        return False

    def _write_jsonl_backup(self, record: dict):
        """Append a record to the jsonl backup file."""
        if self._jsonl_file:
            self._jsonl_file.write(
                json.dumps(record, ensure_ascii=False) + "\n"
            )
            self._jsonl_file.flush()

    def append_vertex(self, vertex: Vertex, domain: str = DOMAIN_GRAPH):
        """Write a vertex to block topology + jsonl backup."""
        content = {
            "event_type": "vertex",
            "domain": domain,
            "vertex_id": vertex.id,
            "status": vertex.status.value,
            "content": vertex.content,
            "created_at": vertex.created_at,
        }
        block = make_block("event", DAEMON_SOURCE, content)
        write_block(block, base=self.bt_base)
        self._vertex_block_ids[vertex.id] = block["id"]

        # jsonl backup
        self._write_jsonl_backup({
            "type": "vertex",
            "id": vertex.id,
            "status": vertex.status.value,
            "content": vertex.content,
            "created_at": vertex.created_at,
        })

    def append_edge(self, edge: Edge, domain: str = DOMAIN_GRAPH):
        """Write an edge to block topology + jsonl backup."""
        content = {
            "event_type": "edge",
            "domain": domain,
            "source": edge.source,
            "target": edge.target,
            "edge_type": edge.edge_type.value,
            "created_at": edge.created_at,
        }
        if edge.surface is not None:
            content["surface"] = edge.surface
        if edge.context is not None:
            content["context"] = edge.context
        block = make_block("event", DAEMON_SOURCE, content)
        write_block(block, base=self.bt_base)

        # Link to source/target vertex blocks if known
        for vid in (edge.source, edge.target):
            src_block_id = self._vertex_block_ids.get(vid)
            if src_block_id:
                rel = make_relation(
                    from_id=block["id"],
                    to_id=src_block_id,
                    relation="related",
                    order=2,
                    created_by=block["id"],
                )
                append_relation(rel, base=self.bt_base)

        # jsonl backup
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
        self._write_jsonl_backup(record)

    def append_operation(self, step: int, operation: str, details: dict):
        """Write an operation record to block topology + jsonl backup."""
        content = {
            "event_type": "operation",
            "domain": DOMAIN_HISTORY,
            "step": step,
            "operation": operation,
            **details,
        }
        block = make_block("event", DAEMON_SOURCE, content)
        write_block(block, base=self.bt_base)

        # Link to involved vertex blocks
        position = details.get("position")
        if position:
            pos_block_id = self._vertex_block_ids.get(position)
            if pos_block_id:
                rel = make_relation(
                    from_id=block["id"],
                    to_id=pos_block_id,
                    relation="related",
                    order=2,
                    created_by=block["id"],
                )
                append_relation(rel, base=self.bt_base)

        # jsonl backup
        self._write_jsonl_backup({
            "type": "operation",
            "step": step,
            "operation": operation,
            **details,
        })

    def append_vertex_status(self, vertex_id: str, status: str, step: int):
        """Write a vertex status change to block topology + jsonl backup."""
        content = {
            "event_type": "vertex_status",
            "domain": DOMAIN_HISTORY,
            "vertex_id": vertex_id,
            "status": status,
            "step": step,
        }
        block = make_block("event", DAEMON_SOURCE, content)
        write_block(block, base=self.bt_base)

        # Link to vertex block
        vid_block = self._vertex_block_ids.get(vertex_id)
        if vid_block:
            rel = make_relation(
                from_id=block["id"],
                to_id=vid_block,
                relation="related",
                order=2,
                created_by=block["id"],
            )
            append_relation(rel, base=self.bt_base)

        # jsonl backup
        self._write_jsonl_backup({
            "type": "vertex_status",
            "id": vertex_id,
            "status": status,
            "step": step,
        })

    def append_merge(self, keep: str, remove: str, step: int):
        """Write a merge record to block topology + jsonl backup."""
        content = {
            "event_type": "merge",
            "domain": DOMAIN_HISTORY,
            "keep": keep,
            "remove": remove,
            "step": step,
        }
        block = make_block("event", DAEMON_SOURCE, content)
        write_block(block, base=self.bt_base)

        # Link to vertex blocks
        for vid in (keep, remove):
            vid_block = self._vertex_block_ids.get(vid)
            if vid_block:
                rel = make_relation(
                    from_id=block["id"],
                    to_id=vid_block,
                    relation="related",
                    order=2,
                    created_by=block["id"],
                )
                append_relation(rel, base=self.bt_base)

        # jsonl backup
        self._write_jsonl_backup({
            "type": "merge",
            "keep": keep,
            "remove": remove,
            "step": step,
        })

    def append_settlement(self, step: int, cycle_edges: list):
        """Write a settlement record to block topology + jsonl backup."""
        content = {
            "event_type": "settlement",
            "domain": DOMAIN_HISTORY,
            "step": step,
            "edges": cycle_edges,
        }
        block = make_block("event", DAEMON_SOURCE, content)
        write_block(block, base=self.bt_base)

        # jsonl backup
        self._write_jsonl_backup({
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
        """Write a COOCCURRENCE block to block topology (immutable, material layer).

        Each S_net co-occurrence edge becomes one block. Old blocks are never deleted;
        new ingestion produces new blocks.
        """
        content = {
            "event_type": "cooccurrence",
            "domain": DOMAIN_MATERIAL,
            "source_signifier": source_signifier,
            "target_signifier": target_signifier,
            "weight": weight,
            "corpus_source": corpus_source,
            "ingest_params": ingest_params,
            "timestamp": timestamp,
        }
        block = make_block("event", DAEMON_SOURCE, content)
        write_block(block, base=self.bt_base)

        self._write_jsonl_backup({
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
        """Write a TRAVERSAL_ASSOCIATION block to block topology (immutable, material layer).

        Records traversal path as material-layer sediment. Each step that moves
        between vertices with signifier mappings produces one block.
        """
        content = {
            "event_type": "traversal_association",
            "domain": DOMAIN_MATERIAL,
            "from_signifier": from_signifier,
            "to_signifier": to_signifier,
            "traversal_id": traversal_id,
            "step_number": step_number,
            "timestamp": timestamp,
        }
        block = make_block("event", DAEMON_SOURCE, content)
        write_block(block, base=self.bt_base)

        self._write_jsonl_backup({
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
        imbalance_type: str,
        reason: str,
        timestamp: str,
    ):
        """Write an ARTICULATION block to block topology (concept layer emergence).

        431号: 拓扑不一致判据——A密B疏 / B密A疏。
        Records the topological inconsistency that triggered articulation.
        """
        content = {
            "event_type": "articulation",
            "domain": DOMAIN_HISTORY,
            "source_vid": source_vid,
            "target_vid": target_vid,
            "score": score,
            "step": step,
            "imbalance_type": imbalance_type,
            "reason": reason,
            "timestamp": timestamp,
        }
        block = make_block("event", DAEMON_SOURCE, content)
        write_block(block, base=self.bt_base)

        self._write_jsonl_backup({
            "type": "articulation",
            "source_vid": source_vid,
            "target_vid": target_vid,
            "score": score,
            "step": step,
            "imbalance_type": imbalance_type,
            "reason": reason,
            "timestamp": timestamp,
        })

    def append_hyperedge(
        self,
        vertices: list[str],
        source: str,
        domain: str,
        timestamp: str,
        evidence_tag: str = "",
        ingest_param_refs: list[str] | None = None,
    ):
        """Write a hyperedge block to block topology (immutable, material layer).

        Each CooccurrenceHyperedge becomes one block. Old blocks are never deleted;
        new ingestion produces new blocks.

        Parameters:
            vertices: sorted list of signifier IDs in the hyperedge
            source: corpus source identifier
            domain: domain tag (e.g. "hegel")
            timestamp: ingestion timestamp
            evidence_tag: corpus reference tag
            ingest_param_refs: references to snet_param concept nodes
        """
        content = {
            "event_type": "hyperedge",
            "domain": DOMAIN_MATERIAL,
            "vertices": sorted(vertices),
            "source": source,
            "hyperedge_domain": domain,
            "timestamp": timestamp,
            "evidence_tag": evidence_tag,
            "ingest_param_refs": list(ingest_param_refs) if ingest_param_refs else [],
        }
        block = make_block("event", DAEMON_SOURCE, content)
        write_block(block, base=self.bt_base)

        self._write_jsonl_backup({
            "type": "hyperedge",
            "vertices": sorted(vertices),
            "source": source,
            "domain": domain,
            "timestamp": timestamp,
            "evidence_tag": evidence_tag,
        })


def load_graph_from_block_topology(
    bt_base: Path = DAEMON_BT_BASE,
) -> tuple[Graph, list[dict]]:
    """Load a Graph from block topology blocks.

    Reads all event blocks with event_type in {vertex, edge, vertex_status, merge},
    replays them in timestamp order to reconstruct the Graph.

    Returns:
        (graph, operations_log) — same interface as PersistentKFull.load()
    """
    blocks_dir = bt_base / "blocks"
    if not blocks_dir.exists():
        return Graph(), []

    # Collect daemon event blocks
    events: list[dict] = []
    for f in blocks_dir.iterdir():
        if f.suffix != ".json":
            continue
        blk = json.loads(f.read_text(encoding="utf-8"))
        content = blk.get("content", {})
        event_type = content.get("event_type")
        if event_type in ("vertex", "edge", "vertex_status", "merge",
                          "operation", "settlement", "articulation"):
            events.append(blk)

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
                # Merge: redirect edges from remove to keep, mark remove as FOLDED
                old_remove = vertices[remove]
                vertices[remove] = Vertex(
                    old_remove.id, VertexStatus.FOLDED,
                    old_remove.content, old_remove.created_at,
                )
                # Redirect edges
                new_edges = []
                for e in edges:
                    src = keep if e.source == remove else e.source
                    tgt = keep if e.target == remove else e.target
                    if src == tgt:
                        continue  # skip self-loops
                    if src != e.source or tgt != e.target:
                        new_edges.append(Edge(
                            src, tgt, e.edge_type, e.created_at,
                            e.surface, e.context,
                        ))
                    else:
                        new_edges.append(e)
                edges = new_edges

        elif event_type == "articulation":
            # Rebuild ARTICULATED edge from articulation provenance block
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
    """Crash recovery: rebuild block topology from k_full.jsonl backup.

    Reads the jsonl file line by line and writes corresponding blocks.
    Returns the number of blocks written.
    """
    if not jsonl_path.exists():
        return 0

    writer = BlockTopologyWriter(bt_base=bt_base)
    count = 0

    with open(jsonl_path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            record = json.loads(line)
            rtype = record.get("type")

            if rtype == "vertex":
                v = Vertex(
                    id=record["id"],
                    status=VertexStatus(record["status"]),
                    content=record.get("content"),
                    created_at=record.get("created_at", 0),
                )
                writer.append_vertex(v)
                count += 1

            elif rtype == "edge":
                e = Edge(
                    source=record["source"],
                    target=record["target"],
                    edge_type=EdgeType(record["edge_type"]),
                    created_at=record.get("created_at", 0),
                    surface=record.get("surface"),
                    context=record.get("context"),
                )
                writer.append_edge(e)
                count += 1

            elif rtype == "operation":
                step = record.get("step", 0)
                operation = record.get("operation", "")
                details = {
                    k: v for k, v in record.items()
                    if k not in ("type", "step", "operation")
                }
                writer.append_operation(step, operation, details)
                count += 1

            elif rtype == "vertex_status":
                writer.append_vertex_status(
                    record["id"], record["status"], record.get("step", 0),
                )
                count += 1

            elif rtype == "merge":
                writer.append_merge(
                    record["keep"], record["remove"], record.get("step", 0),
                )
                count += 1

            elif rtype == "settlement":
                writer.append_settlement(
                    record.get("step", 0), record.get("edges", []),
                )
                count += 1

            elif rtype == "articulation":
                writer.append_articulation(
                    source_vid=record["source_vid"],
                    target_vid=record["target_vid"],
                    score=record.get("score", 0.0),
                    step=record.get("step", 0),
                    cooc_weight=record.get("cooc_weight", 0.0),
                    ta_count=record.get("ta_count", 0),
                    step_gap=record.get("step_gap", 1),
                    timestamp=record.get("timestamp", ""),
                )
                count += 1

    return count
