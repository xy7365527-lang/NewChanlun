"""Regression: SwarmDaemon must not inherit default checkpoint settlements."""

from __future__ import annotations

import tempfile
from pathlib import Path

from engine import SettlementTracker
from traversal_checkpoint import TraversalCheckpoint, restore_daemon_state


def test_restore_appends_settlements_without_clear():
    """Document restore_daemon_state append semantics (why SwarmDaemon must clear)."""
    default_state = {
        "total_steps": 10,
        "total_events": 0,
        "total_feeds": 0,
        "total_gaps_detected": 0,
        "settled_cycles": [
            {
                "edges": [["A", "B"], ["B", "A"]],
                "settled_at": 5,
                "residue": [],
                "status": "active",
            },
        ],
        "pending_cycles": [],
        "crystallization_count": 1,
        "beta_1_history": [],
    }
    instance_state = {
        "total_steps": 3,
        "total_events": 0,
        "total_feeds": 0,
        "total_gaps_detected": 0,
        "settled_cycles": [
            {
                "edges": [["C", "D"], ["D", "C"]],
                "settled_at": 2,
                "residue": [],
                "status": "active",
            },
        ],
        "pending_cycles": [],
        "crystallization_count": 0,
        "beta_1_history": [],
    }

    class FakeDaemon:
        def __init__(self):
            self.total_steps = 0
            self.total_events = 0
            self.total_feeds = 0
            self.total_gaps_detected = 0
            self.settlement = SettlementTracker()
            self.engine = None
            self._crystallization_count = 0
            self._beta_1_history = []
            self.snet_activation = None

    d = FakeDaemon()
    restore_daemon_state(d, default_state)
    restore_daemon_state(d, instance_state)
    assert len(d.settlement.settled_cycles) == 2


def test_swarm_style_replace_clears_default_lockzones():
    """SwarmDaemon must clear parent default settlements before instance restore."""
    td = Path(tempfile.mkdtemp())
    default_cp = TraversalCheckpoint(checkpoint_dir=td, instance_id="default")
    default_cp.save_state(
        {
            "total_steps": 10,
            "total_events": 0,
            "total_feeds": 0,
            "total_gaps_detected": 0,
            "settled_cycles": [
                {
                    "edges": [["A", "B"], ["B", "A"]],
                    "settled_at": 5,
                    "residue": [],
                    "status": "active",
                },
            ],
            "pending_cycles": [],
            "crystallization_count": 1,
            "beta_1_history": [],
        }
    )
    inst_cp = TraversalCheckpoint(checkpoint_dir=td, instance_id="nodeX")
    inst_cp.save_state(
        {
            "total_steps": 3,
            "total_events": 0,
            "total_feeds": 0,
            "total_gaps_detected": 0,
            "settled_cycles": [
                {
                    "edges": [["C", "D"], ["D", "C"]],
                    "settled_at": 2,
                    "residue": [],
                    "status": "active",
                },
            ],
            "pending_cycles": [],
            "crystallization_count": 0,
            "beta_1_history": [],
        }
    )

    class FakeDaemon:
        def __init__(self):
            self.total_steps = 0
            self.total_events = 0
            self.total_feeds = 0
            self.total_gaps_detected = 0
            self.settlement = SettlementTracker()
            self.engine = None
            self._crystallization_count = 0
            self._beta_1_history = []
            self.snet_activation = None

    d = FakeDaemon()
    # Parent path
    restore_daemon_state(d, default_cp.load_state())
    assert len(d.settlement.settled_cycles) == 1

    # SwarmDaemon fix path
    d.settlement._settled.clear()
    d.settlement._pending.clear()
    restore_daemon_state(d, inst_cp.load_state())

    cycles = [frozenset(sc.edges) for sc in d.settlement.settled_cycles]
    assert len(cycles) == 1
    assert frozenset({("C", "D"), ("D", "C")}) in cycles
    assert frozenset({("A", "B"), ("B", "A")}) not in cycles
    assert d.total_steps == 3
