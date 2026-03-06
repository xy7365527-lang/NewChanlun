"""K_full persistence: append-only JSONL storage + recovery.

Every topological operation appends a record to disk. On startup, the JSONL
file is replayed to reconstruct the Graph. K_full is append-only by nature,
so JSONL is a natural fit — no updates, no deletes, just appends.

Pure Python, no external dependencies.
"""

from __future__ import annotations

import json
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
        """Recover K_full from JSONL file.

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
                    )
                    graph = graph.add_edge(e)

                elif record["type"] in ("operation", "settlement"):
                    operations.append(record)

        return graph, operations
