"""Trajectory cluster — 434号(轨迹簇) + 451号(能指链分析) implementation.

434号: 逢亮 thinking = 多路径穿越轨迹簇。轨迹簇记录多条路径的汇聚结构，
汇聚结构不动点（C(n) ≅ C(N)）是 thinking 循环的终止条件。

451号: 原始能指链分析 = 凝缩点（多路径汇聚密节点）+ 移置（共现边跳跃）。
SUBLATED 事件在 fold 中注入凝缩逻辑，消灭事件成为命名材料。
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional


@dataclass(frozen=True, slots=True)
class TrajectoryPath:
    """A single traversal path segment within a trajectory cluster.

    Immutable record of a contiguous path through the graph.
    """
    vertices: tuple[str, ...]
    start_step: int
    end_step: int


@dataclass(frozen=True, slots=True)
class CondensationPoint:
    """451号-1: 凝缩点 — 多路径汇聚的密节点。

    A vertex where multiple trajectory paths converge.
    path_count = number of distinct paths passing through this vertex.
    overdetermination: the vertex's meaning is over-determined by too many
    converging paths (none can singularly determine it).
    """
    vertex_id: str
    path_count: int
    contributing_paths: tuple[int, ...]  # indices into TrajectoryCluster.paths


@dataclass(frozen=True, slots=True)
class DisplacementEvent:
    """451号-2: 移置 — 共现边上的跳跃。

    A jump along a COOCCURRENCE edge during traversal (metonymy).
    The signification shifts along the combinatory axis without
    paradigmatic substitution.
    """
    from_vertex: str
    to_vertex: str
    step: int
    edge_type: str  # "cooccurrence" or "traversal_association"


@dataclass(frozen=True, slots=True)
class SublatedCondensation:
    """451号-3/4: SUBLATED 事件成为凝缩材料。

    When a fold destroys a vertex, the SUBLATED record (if present)
    becomes available as material for condensation — the destruction
    event itself can be named.
    """
    sublated_vertex_id: str
    surviving_vertex_id: str
    fold_step: int
    condensation_vertex: Optional[str] = None  # vertex where this event condensed into


@dataclass
class ConvergenceStructure:
    """434号: 汇聚结构 C(n) — 哪些区域被多条路径共同经过。

    The convergence structure captures the topology of path convergence:
    which vertices are convergence points (visited by multiple paths)
    and how they connect. Two convergence structures are topologically
    equivalent if they have the same convergence points with the same
    path multiplicities.
    """
    convergence_points: frozenset[tuple[str, int]]  # (vertex_id, path_count)

    def is_equivalent(self, other: ConvergenceStructure) -> bool:
        """Check topological equivalence: same convergence points with same multiplicities."""
        return self.convergence_points == other.convergence_points


def _empty_convergence() -> ConvergenceStructure:
    return ConvergenceStructure(convergence_points=frozenset())


@dataclass
class TrajectoryCluster:
    """434号: 轨迹簇 — 多条穿越路径的集合及其汇聚结构。

    Records multiple traversal paths and computes their convergence
    structure. The convergence structure's fixpoint (C(n) ≅ C(N))
    is the termination condition for the thinking loop.

    451号 integration: marks condensation points and displacement
    events within the cluster.
    """
    paths: list[TrajectoryPath] = field(default_factory=list)
    displacements: list[DisplacementEvent] = field(default_factory=list)
    sublated_condensations: list[SublatedCondensation] = field(default_factory=list)
    _current_path_vertices: list[str] = field(default_factory=list)
    _current_path_start: int = 0
    _last_convergence: ConvergenceStructure = field(default_factory=_empty_convergence)
    _fixpoint_count: int = 0  # consecutive steps where convergence didn't change
    fixpoint_threshold: int = 10  # N consecutive unchanged steps → fixpoint reached

    def begin_path(self, start_vertex: str, step: int) -> None:
        """Start recording a new path segment."""
        if self._current_path_vertices:
            self._seal_current_path(step - 1)
        self._current_path_vertices = [start_vertex]
        self._current_path_start = step

    def extend_path(self, vertex: str) -> None:
        """Add a vertex to the current path."""
        self._current_path_vertices.append(vertex)

    def _seal_current_path(self, end_step: int) -> None:
        """Seal the current path into an immutable TrajectoryPath."""
        if len(self._current_path_vertices) >= 2:
            path = TrajectoryPath(
                vertices=tuple(self._current_path_vertices),
                start_step=self._current_path_start,
                end_step=end_step,
            )
            self.paths.append(path)
        self._current_path_vertices = []

    def record_displacement(self, from_v: str, to_v: str, step: int, edge_type: str) -> None:
        """451号-2: Record a displacement event (metonymic jump)."""
        self.displacements.append(DisplacementEvent(
            from_vertex=from_v,
            to_vertex=to_v,
            step=step,
            edge_type=edge_type,
        ))

    def record_sublated_condensation(
        self,
        sublated_id: str,
        surviving_id: str,
        fold_step: int,
    ) -> None:
        """451号-3/4: Record a SUBLATED event as condensation material."""
        self.sublated_condensations.append(SublatedCondensation(
            sublated_vertex_id=sublated_id,
            surviving_vertex_id=surviving_id,
            fold_step=fold_step,
        ))

    def compute_convergence(self) -> ConvergenceStructure:
        """Compute current convergence structure C(n).

        Convergence points are vertices visited by >= 2 paths.
        Returns a ConvergenceStructure for comparison.
        """
        vertex_path_counts: dict[str, set[int]] = {}
        for i, path in enumerate(self.paths):
            for v in path.vertices:
                if v not in vertex_path_counts:
                    vertex_path_counts[v] = set()
                vertex_path_counts[v].add(i)

        # Also count the current (unsealed) path
        current_idx = len(self.paths)
        for v in self._current_path_vertices:
            if v not in vertex_path_counts:
                vertex_path_counts[v] = set()
            vertex_path_counts[v].add(current_idx)

        convergence_points = frozenset(
            (vid, len(path_indices))
            for vid, path_indices in vertex_path_counts.items()
            if len(path_indices) >= 2
        )
        return ConvergenceStructure(convergence_points=convergence_points)

    def condensation_points(self) -> list[CondensationPoint]:
        """451号-1: Compute condensation points — vertices where multiple paths converge.

        Returns vertices sorted by path_count descending (most over-determined first).
        """
        vertex_path_indices: dict[str, list[int]] = {}
        for i, path in enumerate(self.paths):
            for v in path.vertices:
                if v not in vertex_path_indices:
                    vertex_path_indices[v] = []
                if i not in vertex_path_indices[v]:
                    vertex_path_indices[v].append(i)

        points = [
            CondensationPoint(
                vertex_id=vid,
                path_count=len(indices),
                contributing_paths=tuple(indices),
            )
            for vid, indices in vertex_path_indices.items()
            if len(indices) >= 2
        ]
        points.sort(key=lambda p: p.path_count, reverse=True)
        return points

    def check_fixpoint(self, step: int) -> bool:
        """434号: Check if convergence structure has reached a fixpoint.

        Computes C(n), compares with C(n-1). If unchanged for
        fixpoint_threshold consecutive checks, returns True.

        Also seals the current path at EXPLORATION_INTERVAL boundaries
        to create new path segments (each segment between exploration
        jumps is a separate path).
        """
        current = self.compute_convergence()
        if current.is_equivalent(self._last_convergence):
            self._fixpoint_count += 1
        else:
            self._fixpoint_count = 0
            self._last_convergence = current
        return self._fixpoint_count >= self.fixpoint_threshold

    def snapshot(self) -> dict:
        """Return a serializable snapshot of the cluster state."""
        condensations = self.condensation_points()
        return {
            "path_count": len(self.paths),
            "current_path_length": len(self._current_path_vertices),
            "convergence_point_count": len(self._last_convergence.convergence_points),
            "fixpoint_streak": self._fixpoint_count,
            "fixpoint_reached": self._fixpoint_count >= self.fixpoint_threshold,
            "condensation_points": [
                {"vertex_id": c.vertex_id, "path_count": c.path_count}
                for c in condensations[:10]  # top 10 most over-determined
            ],
            "displacement_count": len(self.displacements),
            "sublated_condensation_count": len(self.sublated_condensations),
        }


def compare_convergence(
    c1: ConvergenceStructure,
    c2: ConvergenceStructure,
) -> dict:
    """434号-2: Compare two convergence structures.

    Returns a dict describing what changed:
    - added: new convergence points in c2 not in c1
    - removed: convergence points in c1 not in c2
    - changed: points present in both but with different path counts
    - equivalent: bool
    """
    c1_dict = dict(c1.convergence_points)
    c2_dict = dict(c2.convergence_points)

    c1_vids = set(c1_dict.keys())
    c2_vids = set(c2_dict.keys())

    added = {vid: c2_dict[vid] for vid in c2_vids - c1_vids}
    removed = {vid: c1_dict[vid] for vid in c1_vids - c2_vids}
    changed = {
        vid: (c1_dict[vid], c2_dict[vid])
        for vid in c1_vids & c2_vids
        if c1_dict[vid] != c2_dict[vid]
    }

    return {
        "added": added,
        "removed": removed,
        "changed": changed,
        "equivalent": not added and not removed and not changed,
    }
