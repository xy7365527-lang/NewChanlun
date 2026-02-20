"""tests/test_orchestration_router.py — 路由表匹配逻辑测试"""

from __future__ import annotations

import textwrap
from pathlib import Path

import pytest
import yaml

from newchan.orchestration.router import Route, load_routes, match_event


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

MINIMAL_SPEC = textwrap.dedent("""\
    orchestration_protocol:
      version: "1.0"
      routes:
        - event: file_change
          pattern: "spec/theorems/*"
          target: gemini_math_verify
          action: "gemini_challenger verify --context-file {file}"
          priority: high

        - event: file_change
          pattern: "docs/chan_spec.md"
          target: genealogist
          action: "lineage_consistency_check"
          priority: high

        - event: file_create
          pattern: "**/*"
          target: manifest_guard
          action: "check_manifest_registration"
          priority: high

        - event: annotation
          pattern: "@proof-required"
          target: gemini_math_prove
          action: "gemini_challenger challenge --math"
          priority: low_async

        - event: task_queue
          condition: "independent_tasks >= 2"
          target: swarm_manager
          action: "spawn_recursive_swarm"
          priority: high

        - event: session_start
          target: meta_lead
          action: "ceremony"
          priority: critical

        - event: tool_error
          pattern: "gemini_api_*"
          target: system
          action: "fallback_local_rules"
          priority: critical

        - event: build_failure
          condition: "consecutive_failures >= 3"
          target: build_resolver
          action: "auto_fix_build"
          priority: medium
""")


@pytest.fixture()
def spec_file(tmp_path: Path) -> Path:
    p = tmp_path / "dispatch-spec.yaml"
    p.write_text(MINIMAL_SPEC, encoding="utf-8")
    return p


@pytest.fixture()
def routes(spec_file: Path) -> tuple[Route, ...]:
    return load_routes(spec_file)


# ---------------------------------------------------------------------------
# Route dataclass
# ---------------------------------------------------------------------------

class TestRouteDataclass:
    def test_frozen(self) -> None:
        r = Route(
            event="x", pattern="", target="t",
            action="a", priority="high", condition="",
        )
        with pytest.raises(AttributeError):
            r.event = "y"  # type: ignore[misc]

    def test_priority_rank_known(self) -> None:
        r = Route(event="x", pattern="", target="t",
                  action="a", priority="critical", condition="")
        assert r.priority_rank == 0

    def test_priority_rank_unknown(self) -> None:
        r = Route(event="x", pattern="", target="t",
                  action="a", priority="unknown", condition="")
        assert r.priority_rank == 99


# ---------------------------------------------------------------------------
# load_routes
# ---------------------------------------------------------------------------

class TestLoadRoutes:
    def test_loads_correct_count(self, routes: tuple[Route, ...]) -> None:
        assert len(routes) == 8

    def test_sorted_by_priority(self, routes: tuple[Route, ...]) -> None:
        ranks = [r.priority_rank for r in routes]
        assert ranks == sorted(ranks)

    def test_critical_routes_first(self, routes: tuple[Route, ...]) -> None:
        assert routes[0].priority == "critical"

    def test_file_not_found(self, tmp_path: Path) -> None:
        with pytest.raises(FileNotFoundError):
            load_routes(tmp_path / "nonexistent.yaml")

    def test_missing_section(self, tmp_path: Path) -> None:
        p = tmp_path / "empty.yaml"
        p.write_text("version: '1.0'\n", encoding="utf-8")
        with pytest.raises(ValueError, match="orchestration_protocol"):
            load_routes(p)


# ---------------------------------------------------------------------------
# match_event — file_change
# ---------------------------------------------------------------------------

class TestMatchFileChange:
    def test_match_theorem_file(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "file_change", {"path": "spec/theorems/t1.md"})
        assert len(hits) == 1
        assert hits[0].target == "gemini_math_verify"

    def test_match_chan_spec(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "file_change", {"path": "docs/chan_spec.md"})
        assert len(hits) == 1
        assert hits[0].target == "genealogist"

    def test_no_match_unrelated_path(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "file_change", {"path": "src/foo.py"})
        assert hits == []


# ---------------------------------------------------------------------------
# match_event — file_create
# ---------------------------------------------------------------------------

class TestMatchFileCreate:
    def test_wildcard_matches_any(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "file_create", {"path": "any/deep/file.txt"})
        assert len(hits) == 1
        assert hits[0].target == "manifest_guard"

    def test_no_path_no_match(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "file_create", {})
        assert hits == []


# ---------------------------------------------------------------------------
# match_event — annotation
# ---------------------------------------------------------------------------

class TestMatchAnnotation:
    def test_annotation_match(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "annotation", {"annotation": "@proof-required"})
        assert len(hits) == 1
        assert hits[0].target == "gemini_math_prove"

    def test_annotation_substring(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(
            routes, "annotation",
            {"annotation": "some text @proof-required here"},
        )
        assert len(hits) == 1

    def test_annotation_no_match(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "annotation", {"annotation": "@other"})
        assert hits == []


# ---------------------------------------------------------------------------
# match_event — condition-based
# ---------------------------------------------------------------------------

class TestMatchCondition:
    def test_task_queue_met(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "task_queue", {"independent_tasks": 3})
        assert len(hits) == 1
        assert hits[0].target == "swarm_manager"

    def test_task_queue_not_met(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "task_queue", {"independent_tasks": 1})
        assert hits == []

    def test_build_failure_met(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "build_failure", {"consecutive_failures": 5})
        assert len(hits) == 1
        assert hits[0].target == "build_resolver"

    def test_build_failure_not_met(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "build_failure", {"consecutive_failures": 2})
        assert hits == []

    def test_condition_missing_key(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "task_queue", {})
        assert hits == []


# ---------------------------------------------------------------------------
# match_event — no pattern, no condition (session_start)
# ---------------------------------------------------------------------------

class TestMatchBareEvent:
    def test_session_start(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "session_start")
        assert len(hits) == 1
        assert hits[0].target == "meta_lead"
        assert hits[0].priority == "critical"

    def test_unknown_event(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(routes, "nonexistent_event")
        assert hits == []


# ---------------------------------------------------------------------------
# match_event — tool_error with tool_name
# ---------------------------------------------------------------------------

class TestMatchToolError:
    def test_tool_error_by_tool_name(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(
            routes, "tool_error", {"tool_name": "gemini_api_call"},
        )
        assert len(hits) == 1
        assert hits[0].target == "system"

    def test_tool_error_by_path(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(
            routes, "tool_error", {"path": "gemini_api_timeout"},
        )
        assert len(hits) == 1

    def test_tool_error_no_match(self, routes: tuple[Route, ...]) -> None:
        hits = match_event(
            routes, "tool_error", {"tool_name": "other_tool"},
        )
        assert hits == []


# ---------------------------------------------------------------------------
# match_event — result ordering
# ---------------------------------------------------------------------------

class TestResultOrdering:
    def test_results_sorted_by_priority(self, routes: tuple[Route, ...]) -> None:
        """All matched routes should maintain priority ordering."""
        # session_start is critical, so it should come first if mixed
        hits = match_event(routes, "session_start")
        if len(hits) > 1:
            ranks = [h.priority_rank for h in hits]
            assert ranks == sorted(ranks)
