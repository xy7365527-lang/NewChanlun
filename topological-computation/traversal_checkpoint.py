"""Traversal checkpoint — persistent traversal state across restarts.

Three layers:
1. encounter_log.jsonl   — append-only, immutable (tuché events)
2. settlement_history.jsonl — append-only, immutable (settlement rulings)
3. settlement_state.json — mutable projection of settlement_history
   (current settled cycles, visit_history, position, beta_1_history, etc.)
   Can be rebuilt from settlement_history.jsonl if corrupted.

Startup: load graph → load checkpoint → resume from last position.
If no checkpoint: start from step 0 (first run).
"""

from __future__ import annotations

import json
import os
import time
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Any


DEFAULT_CHECKPOINT_DIR = str(Path.home() / ".swarm" / "checkpoint")


class EncounterLogWriter:
    """Append-only log of tuché events. Immutable once written."""

    def __init__(self, path: str | Path) -> None:
        self._path = Path(path)
        self._path.parent.mkdir(parents=True, exist_ok=True)
        self._fh = open(self._path, "a", encoding="utf-8")

    def record(self, step: int, operation: str, position: str,
               beta_1_before: int, beta_1_after: int,
               f_value: float = -99, blocked: bool = False,
               context: str = "") -> None:
        entry = {
            "ts": time.time(),
            "step": step,
            "operation": operation,
            "position": position,
            "beta_1_before": beta_1_before,
            "beta_1_after": beta_1_after,
            "f_value": f_value,
            "blocked": blocked,
            "context": context[:200],
        }
        self._fh.write(json.dumps(entry, ensure_ascii=False) + "\n")
        self._fh.flush()

    def load_all(self) -> list[dict]:
        if not self._path.exists():
            return []
        entries = []
        with open(self._path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line:
                    try:
                        entries.append(json.loads(line))
                    except json.JSONDecodeError:
                        continue
        return entries

    def close(self) -> None:
        self._fh.close()


class SettlementHistoryWriter:
    """Append-only log of settlement rulings. Immutable once written."""

    def __init__(self, path: str | Path) -> None:
        self._path = Path(path)
        self._path.parent.mkdir(parents=True, exist_ok=True)
        self._fh = open(self._path, "a", encoding="utf-8")

    def record_settlement(self, step: int, cycle_edges: list[tuple[str, str]],
                          encounter_refs: list[int] | None = None) -> None:
        entry = {
            "ts": time.time(),
            "type": "settle",
            "step": step,
            "cycle_edges": cycle_edges,
            "encounter_refs": encounter_refs or [],
        }
        self._fh.write(json.dumps(entry, ensure_ascii=False) + "\n")
        self._fh.flush()

    def record_blocked(self, step: int, operation: str,
                       blocked_by_edges: list[tuple[str, str]]) -> None:
        entry = {
            "ts": time.time(),
            "type": "blocked",
            "step": step,
            "operation": operation,
            "blocked_by_edges": blocked_by_edges,
        }
        self._fh.write(json.dumps(entry, ensure_ascii=False) + "\n")
        self._fh.flush()

    def load_all(self) -> list[dict]:
        if not self._path.exists():
            return []
        entries = []
        with open(self._path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line:
                    try:
                        entries.append(json.loads(line))
                    except json.JSONDecodeError:
                        continue
        return entries

    def close(self) -> None:
        self._fh.close()


class TraversalCheckpoint:
    """Mutable traversal state — saved periodically, restored on startup."""

    def __init__(self, checkpoint_dir: str | Path = DEFAULT_CHECKPOINT_DIR,
                 instance_id: str = "default") -> None:
        self._dir = Path(checkpoint_dir)
        self._dir.mkdir(parents=True, exist_ok=True)
        self._instance_id = instance_id

        # File paths
        self._state_path = self._dir / f"{instance_id}_state.json"
        self._encounter_path = self._dir / f"{instance_id}_encounters.jsonl"
        self._settlement_path = self._dir / f"{instance_id}_settlements.jsonl"

        # Writers
        self.encounters = EncounterLogWriter(self._encounter_path)
        self.settlements = SettlementHistoryWriter(self._settlement_path)

    def save_state(self, state: dict) -> None:
        """Save mutable traversal state (call on settlement or session exit)."""
        state["saved_at"] = time.time()
        state["instance_id"] = self._instance_id
        # Atomic write
        tmp = self._state_path.with_suffix(".tmp")
        tmp.write_text(json.dumps(state, ensure_ascii=False, default=str), encoding="utf-8")
        tmp.replace(self._state_path)

    def load_state(self) -> dict | None:
        """Load last saved traversal state. Returns None if no checkpoint."""
        if not self._state_path.exists():
            return None
        try:
            return json.loads(self._state_path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            return None

    def exists(self) -> bool:
        return self._state_path.exists()

    def close(self) -> None:
        self.encounters.close()
        self.settlements.close()


def extract_daemon_state(daemon) -> dict:
    """Extract saveable state from a running TopologicalDaemon."""
    state: dict[str, Any] = {
        "total_steps": daemon.total_steps,
        "total_events": daemon.total_events,
        "total_feeds": daemon.total_feeds,
        "total_gaps_detected": daemon.total_gaps_detected,
    }

    # Settlement state
    if daemon.settlement:
        state["settled_cycles"] = [
            {"edges": sorted(sc.edges), "settled_at": sc.settled_at_step}
            for sc in daemon.settlement.settled_cycles
        ]
        state["pending_cycles"] = [
            {"edges": sorted(edges), "first_seen": step}
            for edges, step in daemon.settlement._pending.items()
        ]
    else:
        state["settled_cycles"] = []
        state["pending_cycles"] = []

    # Traversal position
    if daemon.engine:
        state["position"] = daemon.engine.position
        state["visit_history"] = daemon.engine.visit_history[-500:]  # Last 500
        state["nothing_streak"] = daemon.engine._nothing_streak
        state["pending_negations"] = [
            [t, a] for t, a in daemon.engine._pending_negations
        ]

    # Crystallization
    state["crystallization_count"] = getattr(daemon, '_crystallization_count', 0)
    state["beta_1_history"] = list(getattr(daemon, '_beta_1_history', []))[-200:]

    return state


def restore_daemon_state(daemon, state: dict) -> None:
    """Restore saved state into a running TopologicalDaemon."""
    daemon.total_steps = state.get("total_steps", 0)
    daemon.total_events = state.get("total_events", 0)
    daemon.total_feeds = state.get("total_feeds", 0)
    daemon.total_gaps_detected = state.get("total_gaps_detected", 0)

    # Settlement
    if daemon.settlement and state.get("settled_cycles"):
        from engine import SettledCycle
        for sc_data in state["settled_cycles"]:
            edges = frozenset(tuple(e) for e in sc_data["edges"])
            sc = SettledCycle(edges=edges, settled_at_step=sc_data["settled_at"])
            if sc not in daemon.settlement._settled:
                daemon.settlement._settled.append(sc)

        for pc_data in state.get("pending_cycles", []):
            edges = frozenset(tuple(e) for e in pc_data["edges"])
            daemon.settlement._pending[edges] = pc_data["first_seen"]

    # Traversal position
    if daemon.engine and state.get("position"):
        active = set(daemon.k_active.active_vertex_ids())
        if state["position"] in active:
            daemon.engine.position = state["position"]
        if state.get("visit_history"):
            daemon.engine.visit_history = [v for v in state["visit_history"] if v in active]
        daemon.engine._nothing_streak = state.get("nothing_streak", 0)
        daemon.engine.step = state.get("total_steps", 0)
        # Restore pending negations (only pairs where both vertices still active)
        if state.get("pending_negations"):
            daemon.engine._pending_negations = [
                (t, a) for t, a in state["pending_negations"]
                if t in active and a in active
            ]

    # Crystallization
    daemon._crystallization_count = state.get("crystallization_count", 0)
    if state.get("beta_1_history"):
        daemon._beta_1_history = list(state["beta_1_history"])
