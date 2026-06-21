"""K_full persistence: append-only JSONL storage + recovery + snapshot.

Every topological operation appends a record to disk. On startup, the JSONL
file is replayed to reconstruct the Graph. K_full is append-only by nature,
so JSONL is a natural fit — no updates, no deletes, just appends.

Snapshot mechanism: periodically dump the full Graph state (vertices + edges)
to a `.snapshot.jsonl` file while preserving the append-only JSONL operation
history. On restart, load snapshot first, then replay JSONL records so merge
and provenance events can repair/complete the snapshot state.

Pure Python, no external dependencies.
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus

DEFAULT_PATH = Path.home() / ".topological-computation" / "k_full.jsonl"


class PersistentKFull:
    """Append-only K_full with disk persistence."""

    def __init__(self, path: str | Path = DEFAULT_PATH):
        self.path = Path(path)
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._file = None

    def open(self):
        """Open file for appending."""
        self._file = open(self.path, "a", encoding="utf-8")
        return self

    def close(self):
        """Close the file handle."""
        if self._file:
            self._file.close()
            self._file = None

    def __enter__(self):
        return self.open()

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
        return False

    def append_vertex(self, vertex: Vertex):
        """Append a vertex record."""
        record = {
            "type": "vertex",
            "id": vertex.id,
            "status": vertex.status.value,
            "content": vertex.content,
            "created_at": vertex.created_at,
        }
        self._write(record)

    def append_edge(self, edge: Edge):
        """Append an edge record."""
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
        self._write(record)

    def append_operation(self, step: int, operation: str, details: dict):
        """Append an operation record (fold/negate/sublate)."""
        record = {
            "type": "operation",
            "step": step,
            "operation": operation,
            **details,
        }
        self._write(record)

    def append_vertex_status(self, vertex_id: str, status: str, step: int):
        """Append a vertex status change record (e.g. fold → FOLDED)."""
        record = {
            "type": "vertex_status",
            "id": vertex_id,
            "status": status,
            "step": step,
        }
        self._write(record)

    def append_merge(self, keep: str, remove: str, step: int):
        """Append a merge record (fold: keep absorbs remove)."""
        record = {
            "type": "merge",
            "keep": keep,
            "remove": remove,
            "step": step,
        }
        self._write(record)

    def append_settlement(self, step: int, cycle_edges: list):
        """Append a settlement record."""
        record = {
            "type": "settlement",
            "step": step,
            "edges": cycle_edges,
        }
        self._write(record)

    def _write(self, record: dict):
        if self._file:
            self._file.write(json.dumps(record, ensure_ascii=False) + "\n")
            self._file.flush()

    @classmethod
    def load(cls, path: str | Path = DEFAULT_PATH) -> tuple[Graph, list[dict]]:
        """Recover graph from JSONL file.

        Replays vertex additions, edge additions, and vertex status changes
        (e.g. fold → FOLDED) in order, so the recovered graph reflects the
        final state including all fold/negate/sublate operations.

        Returns:
            (graph, operations_log) — the reconstructed Graph and the
            sequence of operation/settlement records for replay.
        """
        path = Path(path)
        if not path.exists():
            return Graph(), []

        graph = Graph()
        operations: list[dict] = []

        with open(path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                record = json.loads(line)

                if record["type"] == "vertex":
                    v = Vertex(
                        id=record["id"],
                        status=VertexStatus(record["status"]),
                        content=record.get("content"),
                        created_at=record.get("created_at", 0),
                    )
                    graph = graph.add_vertex(v)

                elif record["type"] == "edge":
                    e = Edge(
                        source=record["source"],
                        target=record["target"],
                        edge_type=EdgeType(record["edge_type"]),
                        created_at=record.get("created_at", 0),
                        surface=record.get("surface"),
                        context=record.get("context"),
                    )
                    if not graph.has_edge_key(e.source, e.target, e.edge_type):
                        graph = graph.add_edge(e)

                elif record["type"] == "vertex_status":
                    # Replay vertex status change (e.g. contested)
                    vid = record["id"]
                    new_status = VertexStatus(record["status"])
                    old_v = graph.vertex(vid)
                    if old_v is not None:
                        new_verts = dict(graph._vertices)
                        new_verts[vid] = Vertex(
                            old_v.id, new_status, old_v.content, old_v.created_at,
                        )
                        graph = Graph(new_verts, list(graph._edges))

                elif record["type"] == "merge":
                    # Replay fold merge: keep absorbs remove
                    keep = record["keep"]
                    remove = record["remove"]
                    if graph.vertex(keep) is not None and graph.vertex(remove) is not None:
                        graph = graph.merge_vertices(keep, remove)

                elif record["type"] in ("operation", "settlement"):
                    operations.append(record)

        return graph, operations

    # -- snapshot methods ---------------------------------------------------

    @staticmethod
    def snapshot_path_for(jsonl_path: str | Path) -> Path:
        """Derive the snapshot file path from a JSONL path.

        e.g. /root/.swarm/persist/fengliang_1.jsonl
          -> /root/.swarm/persist/fengliang_1.snapshot.jsonl
        """
        p = Path(jsonl_path)
        return p.with_suffix(".snapshot.jsonl")

    @staticmethod
    def dump_snapshot(graph: Graph, path: Path) -> int:
        """Write current Graph state to a snapshot file.

        Only writes vertex and edge records (final state). No operation or
        settlement history — those are derivable from the graph structure.

        Edges are deduplicated by (source, target, edge_type) key.

        The snapshot file is written atomically: write to a temp file first,
        then rename. This prevents corruption if the process is killed mid-write.

        Returns the number of records written.
        """
        tmp_path = path.parent / (path.stem + ".tmp")
        path.parent.mkdir(parents=True, exist_ok=True)
        count = 0

        # Deduplicate edges by key (Graph may contain duplicate edges from
        # append-only JSONL replay without dedup checks)
        # 类型约束：memory: 前缀顶点是 settlement 产物，不写入 snapshot
        _MEMORY_PREFIX = "memory:"
        memory_vids: set[str] = set()
        for vid in graph.vertices:
            if vid.startswith(_MEMORY_PREFIX):
                memory_vids.add(vid)

        seen_edges: set[tuple[str, str, str]] = set()
        unique_edges: list[Edge] = []
        for e in graph.edges:
            if e.source in memory_vids or e.target in memory_vids:
                continue
            key = (e.source, e.target, e.edge_type.value)
            if key not in seen_edges:
                seen_edges.add(key)
                unique_edges.append(e)

        snapshot_vertices = {
            vid: v for vid, v in graph.vertices.items()
            if vid not in memory_vids
        }

        header = {
            "type": "snapshot_header",
            "version": 1,
            "timestamp": time.time(),
            "vertex_count": len(snapshot_vertices),
            "edge_count": len(unique_edges),
        }

        with open(tmp_path, "w", encoding="utf-8") as f:
            f.write(json.dumps(header, ensure_ascii=False) + "\n")
            count += 1

            for v in snapshot_vertices.values():
                record = {
                    "type": "vertex",
                    "id": v.id,
                    "status": v.status.value,
                    "content": v.content,
                    "created_at": v.created_at,
                }
                f.write(json.dumps(record, ensure_ascii=False) + "\n")
                count += 1

            for e in unique_edges:
                record = {
                    "type": "edge",
                    "source": e.source,
                    "target": e.target,
                    "edge_type": e.edge_type.value,
                    "created_at": e.created_at,
                }
                if e.surface is not None:
                    record["surface"] = e.surface
                if e.context is not None:
                    record["context"] = e.context
                f.write(json.dumps(record, ensure_ascii=False) + "\n")
                count += 1

        # Atomic rename (same filesystem)
        tmp_path.replace(path)
        return count

    @classmethod
    def load_snapshot_then_incremental(
        cls,
        snapshot_path: Path,
        incremental_path: Path,
    ) -> tuple[Graph, list[dict]]:
        """Load graph from snapshot, then replay incremental JSONL.

        1. Read snapshot file → build base Graph (vertices + edges only)
        2. Read incremental JSONL → replay on top of base Graph
        3. Return (graph, operations_log)

        If snapshot does not exist, falls back to full JSONL load.
        If incremental does not exist or is empty, returns snapshot graph as-is.
        """
        if not snapshot_path.exists():
            return cls.load(incremental_path)

        # Phase 1: load snapshot
        graph = Graph()
        snapshot_timestamp = 0.0

        with open(snapshot_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                record = json.loads(line)

                if record["type"] == "snapshot_header":
                    snapshot_timestamp = record.get("timestamp", 0.0)
                    continue

                if record["type"] == "vertex":
                    v = Vertex(
                        id=record["id"],
                        status=VertexStatus(record["status"]),
                        content=record.get("content"),
                        created_at=record.get("created_at", 0),
                    )
                    graph = graph.add_vertex(v)

                elif record["type"] == "edge":
                    e = Edge(
                        source=record["source"],
                        target=record["target"],
                        edge_type=EdgeType(record["edge_type"]),
                        created_at=record.get("created_at", 0),
                        surface=record.get("surface"),
                        context=record.get("context"),
                    )
                    graph = graph.add_edge(e)

        snapshot_v = len(graph.vertices)
        snapshot_e = len(graph.edges)
        print(
            f"Snapshot loaded: {snapshot_v} vertices, {snapshot_e} edges "
            f"(timestamp={snapshot_timestamp:.0f})",
            file=sys.stderr,
        )

        # Phase 2: replay incremental JSONL (if any)
        operations: list[dict] = []
        if not incremental_path.exists() or incremental_path.stat().st_size == 0:
            return graph, operations

        incremental_count = 0
        with open(incremental_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                record = json.loads(line)

                if record["type"] == "vertex":
                    vid = record["id"]
                    if graph.vertex(vid) is None:
                        v = Vertex(
                            id=vid,
                            status=VertexStatus(record["status"]),
                            content=record.get("content"),
                            created_at=record.get("created_at", 0),
                        )
                        graph = graph.add_vertex(v)
                        incremental_count += 1

                elif record["type"] == "edge":
                    e = Edge(
                        source=record["source"],
                        target=record["target"],
                        edge_type=EdgeType(record["edge_type"]),
                        created_at=record.get("created_at", 0),
                        surface=record.get("surface"),
                        context=record.get("context"),
                    )
                    if not graph.has_edge_key(e.source, e.target, e.edge_type):
                        graph = graph.add_edge(e)
                        incremental_count += 1

                elif record["type"] == "vertex_status":
                    vid = record["id"]
                    new_status = VertexStatus(record["status"])
                    old_v = graph.vertex(vid)
                    if old_v is not None and old_v.status != new_status:
                        new_verts = dict(graph._vertices)
                        new_verts[vid] = Vertex(
                            old_v.id, new_status, old_v.content, old_v.created_at,
                        )
                        graph = Graph(new_verts, list(graph._edges))
                        incremental_count += 1

                elif record["type"] == "merge":
                    keep = record["keep"]
                    remove = record["remove"]
                    if (graph.vertex(keep) is not None
                            and graph.vertex(remove) is not None
                            and graph.vertex(remove).status != VertexStatus.FOLDED):
                        graph = graph.merge_vertices(keep, remove)
                        incremental_count += 1

                elif record["type"] in ("operation", "settlement"):
                    operations.append(record)

        if incremental_count > 0:
            print(
                f"Incremental replay: {incremental_count} new records applied",
                file=sys.stderr,
            )

        return graph, operations
