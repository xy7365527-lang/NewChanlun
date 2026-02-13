"""PR-B0.2 provenance 脚手架契约测试

验证 EventEnvelopeV1 + wrap_event + make_subject_id 的基本契约。
MVP-B0 阶段 parents 始终为空 tuple。
"""

from __future__ import annotations

from dataclasses import FrozenInstanceError

import pytest

from newchan.core.envelope import EventEnvelopeV1
from newchan.core.provenance import wrap_event, make_subject_id
from newchan.events import DomainEvent, StrokeSettled, StrokeCandidate


# ═══════════════════════════════════════════════
# wrap_event
# ═══════════════════════════════════════════════


class TestWrapEvent:
    def test_wraps_domain_event(self):
        ev = DomainEvent(
            event_type="stroke_settled",
            bar_idx=10,
            bar_ts=1000.0,
            seq=5,
            event_id="abcd1234",
            schema_version=1,
        )
        envelope = wrap_event(
            event=ev,
            stream_id="CME:BZ/1min@5m:L0/replay",
            provenance="bi_differ:v1",
            subject_id="stroke:3",
        )
        assert envelope.event_id == "abcd1234"
        assert envelope.stream_id == "CME:BZ/1min@5m:L0/replay"
        assert envelope.bar_time == 1000.0
        assert envelope.seq == 5
        assert envelope.subject_id == "stroke:3"
        assert envelope.provenance == "bi_differ:v1"
        assert envelope.schema_version == 1
        assert envelope.event is ev

    def test_default_parents_empty(self):
        ev = DomainEvent(
            event_type="test", bar_idx=0, bar_ts=0.0, seq=0,
        )
        envelope = wrap_event(event=ev)
        assert envelope.parents == ()

    def test_parents_passed_through(self):
        ev = DomainEvent(
            event_type="test", bar_idx=0, bar_ts=0.0, seq=0,
        )
        envelope = wrap_event(
            event=ev,
            parents=("parent_a", "parent_b"),
        )
        assert envelope.parents == ("parent_a", "parent_b")

    def test_envelope_is_frozen(self):
        ev = DomainEvent(event_type="test", bar_idx=0, bar_ts=0.0, seq=0)
        envelope = wrap_event(event=ev)
        with pytest.raises(FrozenInstanceError):
            envelope.stream_id = "changed"  # type: ignore[misc]


# ═══════════════════════════════════════════════
# make_subject_id
# ═══════════════════════════════════════════════


class TestMakeSubjectId:
    def test_stroke_event(self):
        ev = StrokeSettled(
            bar_idx=10, bar_ts=1000.0, seq=5,
            stroke_id=3, direction="up",
            i0=0, i1=10, p0=100.0, p1=110.0,
        )
        assert make_subject_id(ev) == "stroke:3"

    def test_candidate_event(self):
        ev = StrokeCandidate(
            bar_idx=5, bar_ts=500.0, seq=2,
            stroke_id=7, direction="down",
            i0=0, i1=5, p0=100.0, p1=90.0,
        )
        assert make_subject_id(ev) == "stroke:7"

    def test_non_stroke_event(self):
        ev = DomainEvent(
            event_type="invariant_violation",
            bar_idx=0, bar_ts=0.0, seq=0,
        )
        assert make_subject_id(ev) == "event:invariant_violation"


# ═══════════════════════════════════════════════
# EventEnvelopeV1 额外契约
# ═══════════════════════════════════════════════


class TestEnvelopeContract:
    def test_parents_immutable(self):
        """parents 是 tuple，不可修改。"""
        envelope = EventEnvelopeV1(parents=("a", "b"))
        with pytest.raises(AttributeError):
            envelope.parents.append("c")  # type: ignore[attr-defined]

    def test_provenance_freeform(self):
        """provenance 可以是任意字符串。"""
        envelope = EventEnvelopeV1(provenance="custom:v2:experimental")
        assert envelope.provenance == "custom:v2:experimental"
