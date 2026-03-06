"""Cross-instance operation injection.

Periodically scans the shared layer for new blocks written by other instances.
New blocks are injected into the local K_active as external operations.
"""

from __future__ import annotations

import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

from engine import Graph, Vertex, Edge, EdgeType, VertexStatus
from swarm.shared_layer import SharedLayer


class CrossInstanceSync:
    """Sync operations from other instances via shared block store."""

    def __init__(
        self,
        shared_layer: SharedLayer,
        instance_id: str,
        daemon,  # SwarmDaemon — forward reference to avoid circular import
    ):
        self.shared = shared_layer
        self.instance_id = instance_id
        self.daemon = daemon
        self.known_blocks: set[str] = set()
        self.injected_count: int = 0

    def sync(self) -> int:
        """Scan for new blocks, inject those from other instances.

        Returns the number of blocks injected.
        """
        new_blocks = self.shared.read_new_blocks(self.known_blocks)
        injected = 0
        for block in new_blocks:
            block_hash = block.get("hash", "")
            self.known_blocks.add(block_hash)
            if block.get("instance") != self.instance_id:
                self._inject_external_operation(block)
                injected += 1
        self.injected_count += injected
        return injected

    def _inject_external_operation(self, block: dict) -> None:
        """Inject an external block's vertices and edges into the local daemon."""
        graph = self.daemon.k_active

        # Extract vertices from block
        for vd in block.get("vertices", []):
            vid = vd["id"]
            if graph.vertex(vid) is None:
                v = Vertex(
                    id=vid,
                    status=VertexStatus(vd.get("status", "active")),
                    content=vd.get("content"),
                    created_at=vd.get("created_at", 0),
                )
                graph = graph.add_vertex(v)

        # Extract edges from block
        existing_edges = {
            (e.source, e.target, e.edge_type.value) for e in graph.edges
        }
        for ed in block.get("edges", []):
            key = (ed["source"], ed["target"], ed["edge_type"])
            if key not in existing_edges:
                src_v = graph.vertex(ed["source"])
                tgt_v = graph.vertex(ed["target"])
                if src_v is not None and tgt_v is not None:
                    e = Edge(
                        source=ed["source"],
                        target=ed["target"],
                        edge_type=EdgeType(ed["edge_type"]),
                        created_at=ed.get("created_at", 0),
                    )
                    graph = graph.add_edge(e)
                    existing_edges.add(key)

        # Update daemon state
        self.daemon.k_active = graph
        self.daemon.k_full = graph
        self.daemon._initialize_engine()
