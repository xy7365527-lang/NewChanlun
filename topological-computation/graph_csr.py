"""CSR (Compressed Sparse Row) view of Graph — scipy.sparse verification layer.

Migration path step 1: Python dict -> scipy CSR (CPU) -> cupy CSR (GPU, S_net only).
This module provides a read-only CSR view of Graph for query verification and benchmarking.
It does NOT replace Graph — it is an附属视图 that mirrors Graph's topology.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np
from scipy.sparse import csr_matrix
from scipy.sparse.csgraph import connected_components as sp_connected_components

if TYPE_CHECKING:
    from engine import Graph

from engine import EdgeType, CONCEPT_EDGE_TYPES, _MATERIAL_EDGE_TYPES


class VertexIndex:
    """Bidirectional mapping between vertex id (str) and integer index."""

    __slots__ = ("_str_to_int", "_int_to_str")

    def __init__(self, vertex_ids: list[str]) -> None:
        self._str_to_int: dict[str, int] = {vid: i for i, vid in enumerate(vertex_ids)}
        self._int_to_str: list[str] = list(vertex_ids)

    def __len__(self) -> int:
        return len(self._int_to_str)

    def to_int(self, vid: str) -> int:
        return self._str_to_int[vid]

    def to_str(self, idx: int) -> str:
        return self._int_to_str[idx]

    def has(self, vid: str) -> bool:
        return vid in self._str_to_int

    @property
    def ids(self) -> list[str]:
        return list(self._int_to_str)


class GraphCSR:
    """Read-only CSR view of a Graph instance.

    Maintains two separate CSR matrices:
    - concept_matrix: edges in CONCEPT_EDGE_TYPES (used for beta_1, settlement)
    - material_matrix: edges in _MATERIAL_EDGE_TYPES (COOCCURRENCE, TRAVERSAL_ASSOCIATION)

    Both are directed adjacency matrices. For undirected queries (beta_1),
    the symmetrized version is computed on demand.
    """

    __slots__ = (
        "_index",
        "_concept_csr",
        "_material_csr",
        "_n",
        "_concept_undirected",
    )

    def __init__(self, graph: Graph) -> None:
        active_ids = sorted(graph._active_ids)
        self._index = VertexIndex(active_ids)
        self._n = len(active_ids)

        concept_rows: list[int] = []
        concept_cols: list[int] = []
        material_rows: list[int] = []
        material_cols: list[int] = []

        idx = self._index
        active_set = graph._active_ids

        for e in graph._edges:
            if e.source not in active_set or e.target not in active_set:
                continue
            src = idx.to_int(e.source)
            tgt = idx.to_int(e.target)
            if e.edge_type in CONCEPT_EDGE_TYPES:
                concept_rows.append(src)
                concept_cols.append(tgt)
            elif e.edge_type in _MATERIAL_EDGE_TYPES:
                material_rows.append(src)
                material_cols.append(tgt)

        ones_c = np.ones(len(concept_rows), dtype=np.int8)
        ones_m = np.ones(len(material_rows), dtype=np.int8)

        self._concept_csr = csr_matrix(
            (ones_c, (concept_rows, concept_cols)),
            shape=(self._n, self._n),
            dtype=np.int8,
        )
        self._material_csr = csr_matrix(
            (ones_m, (material_rows, material_cols)),
            shape=(self._n, self._n),
            dtype=np.int8,
        )
        self._concept_undirected: csr_matrix | None = None

    @property
    def index(self) -> VertexIndex:
        return self._index

    @property
    def n_vertices(self) -> int:
        return self._n

    @property
    def concept_matrix(self) -> csr_matrix:
        return self._concept_csr

    @property
    def material_matrix(self) -> csr_matrix:
        return self._material_csr

    def _get_concept_undirected(self) -> csr_matrix:
        """Symmetrized concept matrix (no self-loops, binary)."""
        if self._concept_undirected is None:
            c = self._concept_csr
            # Remove diagonal (self-loops handled separately)
            c_no_diag = c.copy()
            c_no_diag.setdiag(0)
            c_no_diag.eliminate_zeros()
            # Symmetrize: A + A^T, then binarize
            sym = c_no_diag + c_no_diag.T
            sym.data[:] = 1
            self._concept_undirected = sym
        return self._concept_undirected

    # -- query methods -------------------------------------------------------

    def neighbors_csr(self, vid: str) -> set[str]:
        """Return set of active neighbor ids (outgoing + incoming, all edge types)."""
        if not self._index.has(vid):
            return set()
        i = self._index.to_int(vid)
        combined = self._concept_csr + self._material_csr
        # outgoing: row i
        out_indices = set(combined.getrow(i).indices.tolist())
        # incoming: column i (= row i of transpose)
        in_indices = set(combined.T.getrow(i).indices.tolist())
        all_indices = out_indices | in_indices
        return {self._index.to_str(j) for j in all_indices}

    def concept_neighbors_csr(self, vid: str) -> set[str]:
        """Return set of active neighbor ids (concept edges only)."""
        if not self._index.has(vid):
            return set()
        i = self._index.to_int(vid)
        out_indices = set(self._concept_csr.getrow(i).indices.tolist())
        in_indices = set(self._concept_csr.T.getrow(i).indices.tolist())
        all_indices = out_indices | in_indices
        return {self._index.to_str(j) for j in all_indices}

    def out_neighbors_csr(self, vid: str) -> set[str]:
        """Return set of outgoing neighbor ids (all edge types)."""
        if not self._index.has(vid):
            return set()
        i = self._index.to_int(vid)
        combined = self._concept_csr + self._material_csr
        indices = set(combined.getrow(i).indices.tolist())
        return {self._index.to_str(j) for j in indices}

    def in_neighbors_csr(self, vid: str) -> set[str]:
        """Return set of incoming neighbor ids (all edge types)."""
        if not self._index.has(vid):
            return set()
        i = self._index.to_int(vid)
        combined = self._concept_csr + self._material_csr
        indices = set(combined.T.getrow(i).indices.tolist())
        return {self._index.to_str(j) for j in indices}

    def connected_components_csr(self) -> int:
        """Number of connected components in the undirected concept graph."""
        if self._n == 0:
            return 0
        und = self._get_concept_undirected()
        n_components, _ = sp_connected_components(und, directed=False)
        return n_components

    def beta_1_csr(self) -> int:
        """Compute beta_1 via CSR: |E_undirected| - |V| + C + self_loops.

        Matches compute_beta_1 in engine.py.
        """
        if self._n == 0:
            return 0
        und = self._get_concept_undirected()
        # Number of undirected edges = nnz / 2 (symmetric matrix, each edge stored twice)
        n_e = und.nnz // 2
        n_v = self._n
        c = self.connected_components_csr()
        # Self-loops: diagonal entries in concept_csr
        n_loops = self._concept_csr.diagonal().astype(bool).sum()
        return n_e - n_v + c + int(n_loops)


def from_graph(graph: Graph) -> GraphCSR:
    """Create a CSR view from a Graph instance."""
    return GraphCSR(graph)
