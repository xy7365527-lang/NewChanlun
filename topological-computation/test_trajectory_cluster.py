"""Tests for trajectory_cluster.py — 434号(轨迹簇) + 451号(能指链分析)."""

import pytest
from trajectory_cluster import (
    TrajectoryCluster,
    TrajectoryPath,
    ConvergenceStructure,
    CondensationPoint,
    DisplacementEvent,
    SublatedCondensation,
    compare_convergence,
)


class TestTrajectoryPath:
    def test_immutable(self):
        p = TrajectoryPath(vertices=("a", "b", "c"), start_step=0, end_step=2)
        assert p.vertices == ("a", "b", "c")
        with pytest.raises(AttributeError):
            p.vertices = ("x",)


class TestConvergenceStructure:
    def test_equivalent_same_points(self):
        c1 = ConvergenceStructure(convergence_points=frozenset([("a", 2), ("b", 3)]))
        c2 = ConvergenceStructure(convergence_points=frozenset([("a", 2), ("b", 3)]))
        assert c1.is_equivalent(c2)

    def test_not_equivalent_different_counts(self):
        c1 = ConvergenceStructure(convergence_points=frozenset([("a", 2)]))
        c2 = ConvergenceStructure(convergence_points=frozenset([("a", 3)]))
        assert not c1.is_equivalent(c2)

    def test_not_equivalent_different_points(self):
        c1 = ConvergenceStructure(convergence_points=frozenset([("a", 2)]))
        c2 = ConvergenceStructure(convergence_points=frozenset([("b", 2)]))
        assert not c1.is_equivalent(c2)

    def test_empty_equivalent(self):
        c1 = ConvergenceStructure(convergence_points=frozenset())
        c2 = ConvergenceStructure(convergence_points=frozenset())
        assert c1.is_equivalent(c2)


class TestCompareConvergence:
    def test_identical(self):
        c = ConvergenceStructure(convergence_points=frozenset([("a", 2)]))
        result = compare_convergence(c, c)
        assert result["equivalent"] is True
        assert result["added"] == {}
        assert result["removed"] == {}
        assert result["changed"] == {}

    def test_added_point(self):
        c1 = ConvergenceStructure(convergence_points=frozenset([("a", 2)]))
        c2 = ConvergenceStructure(convergence_points=frozenset([("a", 2), ("b", 3)]))
        result = compare_convergence(c1, c2)
        assert result["equivalent"] is False
        assert result["added"] == {"b": 3}

    def test_removed_point(self):
        c1 = ConvergenceStructure(convergence_points=frozenset([("a", 2), ("b", 3)]))
        c2 = ConvergenceStructure(convergence_points=frozenset([("a", 2)]))
        result = compare_convergence(c1, c2)
        assert result["equivalent"] is False
        assert result["removed"] == {"b": 3}

    def test_changed_count(self):
        c1 = ConvergenceStructure(convergence_points=frozenset([("a", 2)]))
        c2 = ConvergenceStructure(convergence_points=frozenset([("a", 5)]))
        result = compare_convergence(c1, c2)
        assert result["equivalent"] is False
        assert result["changed"] == {"a": (2, 5)}


class TestTrajectoryCluster:
    def test_path_recording(self):
        tc = TrajectoryCluster()
        tc.begin_path("a", 0)
        tc.extend_path("b")
        tc.extend_path("c")
        # Begin new path seals the previous one
        tc.begin_path("d", 3)
        assert len(tc.paths) == 1
        assert tc.paths[0].vertices == ("a", "b", "c")
        assert tc.paths[0].start_step == 0
        assert tc.paths[0].end_step == 2

    def test_single_vertex_path_not_sealed(self):
        tc = TrajectoryCluster()
        tc.begin_path("a", 0)
        # Single vertex path is not sealed (needs >= 2 vertices)
        tc.begin_path("b", 1)
        assert len(tc.paths) == 0

    def test_convergence_no_paths(self):
        tc = TrajectoryCluster()
        c = tc.compute_convergence()
        assert len(c.convergence_points) == 0

    def test_convergence_two_paths_shared_vertex(self):
        tc = TrajectoryCluster()
        # Path 1: a -> b -> c
        tc.begin_path("a", 0)
        tc.extend_path("b")
        tc.extend_path("c")
        # Path 2: d -> b -> e
        tc.begin_path("d", 3)
        tc.extend_path("b")
        tc.extend_path("e")
        # Seal path 2
        tc.begin_path("f", 6)

        c = tc.compute_convergence()
        # "b" is visited by both paths → convergence point with count 2
        conv_dict = dict(c.convergence_points)
        assert "b" in conv_dict
        assert conv_dict["b"] == 2

    def test_condensation_points(self):
        tc = TrajectoryCluster()
        # Path 1: a -> b -> c
        tc.begin_path("a", 0)
        tc.extend_path("b")
        tc.extend_path("c")
        # Path 2: d -> b -> c
        tc.begin_path("d", 3)
        tc.extend_path("b")
        tc.extend_path("c")
        # Seal
        tc.begin_path("e", 6)

        points = tc.condensation_points()
        # Both b and c appear in 2 paths
        vids = {p.vertex_id for p in points}
        assert "b" in vids
        assert "c" in vids
        for p in points:
            assert p.path_count == 2

    def test_condensation_sorted_by_count(self):
        tc = TrajectoryCluster()
        # Path 1: a -> x -> b
        tc.begin_path("a", 0)
        tc.extend_path("x")
        tc.extend_path("b")
        # Path 2: c -> x -> b
        tc.begin_path("c", 3)
        tc.extend_path("x")
        tc.extend_path("b")
        # Path 3: d -> x
        tc.begin_path("d", 6)
        tc.extend_path("x")
        # Seal
        tc.begin_path("e", 9)

        points = tc.condensation_points()
        # x appears in 3 paths, b in 2
        assert points[0].vertex_id == "x"
        assert points[0].path_count == 3

    def test_displacement_recording(self):
        tc = TrajectoryCluster()
        tc.record_displacement("a", "b", 5, "cooccurrence")
        assert len(tc.displacements) == 1
        assert tc.displacements[0].from_vertex == "a"
        assert tc.displacements[0].to_vertex == "b"
        assert tc.displacements[0].edge_type == "cooccurrence"

    def test_sublated_condensation_recording(self):
        tc = TrajectoryCluster()
        tc.record_sublated_condensation("dead_v", "alive_v", 10)
        assert len(tc.sublated_condensations) == 1
        assert tc.sublated_condensations[0].sublated_vertex_id == "dead_v"
        assert tc.sublated_condensations[0].surviving_vertex_id == "alive_v"
        assert tc.sublated_condensations[0].fold_step == 10

    def test_fixpoint_detection(self):
        tc = TrajectoryCluster(fixpoint_threshold=3)
        # No paths → empty convergence → always equivalent
        assert not tc.check_fixpoint(1)  # streak=1
        assert not tc.check_fixpoint(2)  # streak=2
        assert tc.check_fixpoint(3)      # streak=3 → fixpoint

    def test_fixpoint_resets_on_change(self):
        tc = TrajectoryCluster(fixpoint_threshold=3)
        tc.check_fixpoint(1)  # streak=1
        tc.check_fixpoint(2)  # streak=2
        # Add a path that changes convergence
        tc.begin_path("a", 0)
        tc.extend_path("b")
        tc.begin_path("c", 1)
        tc.extend_path("b")
        tc.begin_path("d", 2)  # seal path 2
        # Now convergence has changed
        assert not tc.check_fixpoint(3)  # streak reset to 0
        assert not tc.check_fixpoint(4)  # streak=1
        assert not tc.check_fixpoint(5)  # streak=2
        assert tc.check_fixpoint(6)      # streak=3

    def test_snapshot(self):
        tc = TrajectoryCluster()
        tc.begin_path("a", 0)
        tc.extend_path("b")
        tc.begin_path("c", 2)
        tc.extend_path("b")
        tc.begin_path("d", 4)
        tc.record_displacement("x", "y", 1, "cooccurrence")
        tc.record_sublated_condensation("z", "w", 3)

        snap = tc.snapshot()
        assert snap["path_count"] == 2
        assert snap["displacement_count"] == 1
        assert snap["sublated_condensation_count"] == 1
        assert isinstance(snap["condensation_points"], list)


class TestCondensationPoint:
    def test_immutable(self):
        cp = CondensationPoint(vertex_id="a", path_count=3, contributing_paths=(0, 1, 2))
        assert cp.path_count == 3
        with pytest.raises(AttributeError):
            cp.vertex_id = "b"


class TestDisplacementEvent:
    def test_immutable(self):
        de = DisplacementEvent(from_vertex="a", to_vertex="b", step=5, edge_type="cooccurrence")
        with pytest.raises(AttributeError):
            de.step = 10


class TestSublatedCondensation:
    def test_default_condensation_vertex(self):
        sc = SublatedCondensation(
            sublated_vertex_id="dead",
            surviving_vertex_id="alive",
            fold_step=7,
        )
        assert sc.condensation_vertex is None
