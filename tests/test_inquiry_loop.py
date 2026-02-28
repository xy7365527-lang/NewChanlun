"""Tests for scripts/inquiry_loop.py — 多轮质询调度器

测试设计遵循十五轮 Gemini 质询的结算规范（188号谱系）：
- 同主体 diff 收敛：比较同一主体相邻轮次（非交叉主体）
- Key 冻结：连续 N 轮无新 Key 注册
- Trajectories 输出：unresolved Key → 互斥破裂轨迹（非 residue 列表）

所有 API 调用通过 mock 拦截——不依赖 Gemini/Codex API key。
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest

from scripts.consensus_trigger import (
    InquiryCycleResult,
    StanceDeclaration,
)
from scripts.inquiry_loop import (
    ConvergenceVerdict,
    InquiryResult,
    RoundSnapshot,
    Trajectory,
    _check_stance_repetition,
    build_cycle_result,
    build_trajectories,
    check_convergence,
    check_key_frozen,
    check_same_subject_stable,
    run_inquiry_loop,
)


# ── Helpers ──


def _make_stance(
    verdict: str = "pass",
    stances: dict[str, str] | None = None,
    round_number: int = 1,
    concessions: list[str] | None = None,
) -> StanceDeclaration:
    return StanceDeclaration(
        verdict=verdict,
        stances=stances or {},
        round_number=round_number,
        self_reported_concessions=concessions or [],
    )


def _make_snapshot(
    round_number: int,
    speaker: str,
    stance: StanceDeclaration | None = None,
    new_keys: frozenset[str] | None = None,
    registered_keys: frozenset[str] | None = None,
) -> RoundSnapshot:
    return RoundSnapshot(
        round_number=round_number,
        speaker=speaker,
        response_text=f"R{round_number} {speaker} response",
        stance=stance,
        registered_keys=registered_keys or frozenset(),
        new_keys=new_keys or frozenset(),
    )


STANCE_BLOCK_TEMPLATE = """\
Some analysis text here.

---stance-declaration---
verdict: {verdict}
stances:
{stances_yaml}
concessions: {concessions}
---end-stance---
"""


def _make_response_with_stance(
    verdict: str = "pass",
    stances: dict[str, str] | None = None,
    concessions: list[str] | None = None,
) -> str:
    stances = stances or {}
    stances_yaml = "\n".join(f"  {k}: {v}" for k, v in stances.items())
    if not stances_yaml:
        stances_yaml = "  {}"
    conc = concessions or []
    conc_yaml = str(conc) if conc else "[]"
    return STANCE_BLOCK_TEMPLATE.format(
        verdict=verdict,
        stances_yaml=stances_yaml,
        concessions=conc_yaml,
    )


@pytest.fixture
def tmp_base(tmp_path):
    """Provide a temporary block-topology directory."""
    base = tmp_path / "block-topology"
    base.mkdir()
    (base / "blocks").mkdir()
    return base


# ── RoundSnapshot tests ──


class TestRoundSnapshot:
    def test_immutable(self):
        snap = _make_snapshot(1, "gemini")
        with pytest.raises(AttributeError):
            snap.round_number = 2  # type: ignore[misc]

    def test_fields(self):
        stance = _make_stance(stances={"a": "reject"})
        snap = _make_snapshot(
            round_number=3,
            speaker="codex",
            stance=stance,
            new_keys=frozenset({"a"}),
            registered_keys=frozenset({"a", "b"}),
        )
        assert snap.round_number == 3
        assert snap.speaker == "codex"
        assert snap.stance is stance
        assert snap.new_keys == frozenset({"a"})
        assert snap.registered_keys == frozenset({"a", "b"})


# ── ConvergenceVerdict tests ──


class TestConvergenceVerdict:
    def test_immutable(self):
        v = ConvergenceVerdict(
            converged=True,
            key_frozen=True,
            same_subject_stable=True,
            reason="test",
        )
        with pytest.raises(AttributeError):
            v.converged = False  # type: ignore[misc]

    def test_fields(self):
        v = ConvergenceVerdict(
            converged=False,
            key_frozen=True,
            same_subject_stable=False,
            reason="Codex 未稳定",
        )
        assert v.converged is False
        assert v.key_frozen is True
        assert v.same_subject_stable is False


# ── Trajectory tests ──


class TestTrajectory:
    def test_immutable(self):
        t = Trajectory(
            lack_key="k", lack_description="d", expected_tension="t",
        )
        with pytest.raises(AttributeError):
            t.lack_key = "other"  # type: ignore[misc]


# ── Key frozen tests ──


class TestKeyFrozen:
    def test_frozen_when_no_new_keys(self):
        snaps = [
            _make_snapshot(i, "gemini", new_keys=frozenset())
            for i in range(1, 4)
        ]
        assert check_key_frozen(snaps, window=3) is True

    def test_not_frozen_with_new_key(self):
        snaps = [
            _make_snapshot(1, "gemini", new_keys=frozenset()),
            _make_snapshot(2, "codex", new_keys=frozenset({"new_key"})),
            _make_snapshot(3, "gemini", new_keys=frozenset()),
        ]
        assert check_key_frozen(snaps, window=3) is False

    def test_not_frozen_insufficient_rounds(self):
        snaps = [_make_snapshot(1, "gemini", new_keys=frozenset())]
        assert check_key_frozen(snaps, window=3) is False

    def test_new_key_prevents_convergence(self):
        """新 Key 出现 → 不收敛"""
        snaps = [
            _make_snapshot(1, "gemini", new_keys=frozenset({"k1"})),
            _make_snapshot(1, "codex", new_keys=frozenset()),
            _make_snapshot(2, "gemini", new_keys=frozenset()),
            _make_snapshot(2, "codex", new_keys=frozenset()),
            _make_snapshot(3, "gemini", new_keys=frozenset({"k2"})),  # 新 Key
            _make_snapshot(3, "codex", new_keys=frozenset()),
        ]
        assert check_key_frozen(snaps, window=3) is False


# ── Same subject stable tests ──


class TestSameSubjectStable:
    def test_stable_when_identical_stances(self):
        stance = _make_stance(stances={"a": "reject"})
        snaps = [
            _make_snapshot(1, "gemini", stance=stance),
            _make_snapshot(1, "codex"),
            _make_snapshot(2, "gemini", stance=stance),
            _make_snapshot(2, "codex"),
            _make_snapshot(3, "gemini", stance=stance),
        ]
        assert check_same_subject_stable(snaps, "gemini", window=3) is True

    def test_not_stable_with_change(self):
        stance_a = _make_stance(stances={"a": "reject"}, round_number=1)
        stance_b = _make_stance(stances={"a": "accept"}, round_number=3)
        stance_c = _make_stance(stances={"a": "accept"}, round_number=5)
        snaps = [
            _make_snapshot(1, "gemini", stance=stance_a),
            _make_snapshot(2, "gemini", stance=stance_b),
            _make_snapshot(3, "gemini", stance=stance_c),
        ]
        assert check_same_subject_stable(snaps, "gemini", window=3) is False

    def test_alternating_oscillation_no_false_convergence(self):
        """Gemini=reject, Codex=accept 交替 → 同主体 diff 为空 → 正确收敛。

        这是死锁 = 不动点的情况。同主体 diff 检测到双方各自稳定。
        """
        g_stance = _make_stance(stances={"a": "reject"})
        c_stance = _make_stance(stances={"a": "accept"})
        snaps = [
            _make_snapshot(1, "gemini", stance=g_stance),
            _make_snapshot(1, "codex", stance=c_stance),
            _make_snapshot(2, "gemini", stance=g_stance),
            _make_snapshot(2, "codex", stance=c_stance),
            _make_snapshot(3, "gemini", stance=g_stance),
            _make_snapshot(3, "codex", stance=c_stance),
        ]
        assert check_same_subject_stable(snaps, "gemini", window=3) is True
        assert check_same_subject_stable(snaps, "codex", window=3) is True

    def test_insufficient_rounds(self):
        stance = _make_stance(stances={"a": "reject"})
        snaps = [_make_snapshot(1, "gemini", stance=stance)]
        assert check_same_subject_stable(snaps, "gemini", window=3) is False

    def test_none_stance_stable(self):
        """两轮都 parse 失败（stance=None）= 稳定"""
        snaps = [
            _make_snapshot(1, "gemini", stance=None),
            _make_snapshot(2, "gemini", stance=None),
            _make_snapshot(3, "gemini", stance=None),
        ]
        assert check_same_subject_stable(snaps, "gemini", window=3) is True

    def test_none_to_some_unstable(self):
        """一轮 None 一轮有 stance = 不稳定"""
        stance = _make_stance(stances={"a": "reject"})
        snaps = [
            _make_snapshot(1, "gemini", stance=None),
            _make_snapshot(2, "gemini", stance=stance),
            _make_snapshot(3, "gemini", stance=stance),
        ]
        assert check_same_subject_stable(snaps, "gemini", window=3) is False


# ── Full convergence check ──


class TestCheckConvergence:
    def test_converges(self):
        """同主体 diff 为空 + Key 冻结 → converged=True"""
        g_stance = _make_stance(stances={"a": "reject"})
        c_stance = _make_stance(stances={"a": "accept"})
        snaps = [
            _make_snapshot(i, "gemini", stance=g_stance, new_keys=frozenset())
            for i in range(1, 4)
        ] + [
            _make_snapshot(i, "codex", stance=c_stance, new_keys=frozenset())
            for i in range(1, 4)
        ]
        v = check_convergence(snaps, stability_window=3)
        assert v.converged is True
        assert v.key_frozen is True
        assert v.same_subject_stable is True

    def test_not_converges_key_not_frozen(self):
        """新 Key 在最近 window 轮内出现 → key_frozen=False"""
        g_stance = _make_stance(stances={"a": "reject"})
        c_stance = _make_stance(stances={"a": "accept"})
        snaps = [
            _make_snapshot(1, "gemini", stance=g_stance, new_keys=frozenset()),
            _make_snapshot(1, "codex", stance=c_stance, new_keys=frozenset()),
            _make_snapshot(2, "gemini", stance=g_stance, new_keys=frozenset()),
            # new key in last 3 snapshots → key NOT frozen
            _make_snapshot(
                2, "codex", stance=c_stance,
                new_keys=frozenset({"new"}),
            ),
            _make_snapshot(3, "gemini", stance=g_stance, new_keys=frozenset()),
            _make_snapshot(3, "codex", stance=c_stance, new_keys=frozenset()),
        ]
        v = check_convergence(snaps, stability_window=3)
        assert v.converged is False
        assert v.key_frozen is False

    def test_convergence_verdict_structure(self):
        """ConvergenceVerdict 各字段正确"""
        snaps = [
            _make_snapshot(i, "gemini", new_keys=frozenset())
            for i in range(1, 4)
        ] + [
            _make_snapshot(i, "codex", new_keys=frozenset())
            for i in range(1, 4)
        ]
        # All None stances → stable (both None)
        v = check_convergence(snaps, stability_window=3)
        assert isinstance(v.converged, bool)
        assert isinstance(v.key_frozen, bool)
        assert isinstance(v.same_subject_stable, bool)
        assert isinstance(v.reason, str)


# ── Stance repetition detection tests (270号) ──


class TestCheckStanceRepetition:
    def test_no_repetition_with_different_stances(self):
        """每轮 stance 不同 → 不重复"""
        s1 = _make_stance(stances={"a": "reject"}, round_number=1)
        s2 = _make_stance(stances={"a": "accept"}, round_number=2)
        s3 = _make_stance(stances={"a": "needs_work"}, round_number=3)
        snaps = [
            _make_snapshot(1, "gemini", stance=s1),
            _make_snapshot(2, "gemini", stance=s2),
            _make_snapshot(3, "gemini", stance=s3),
        ]
        detected, keys = _check_stance_repetition(snaps, "gemini", window=2)
        assert detected is False
        assert keys == []

    def test_repetition_detected_key_and_value_same(self):
        """连续 3 轮（基准+2轮重复）Key+Value 完全相同 → 检测到重复"""
        stance = _make_stance(stances={"attack_a": "reject", "attack_b": "contradictory"})
        snaps = [
            _make_snapshot(1, "gemini", stance=stance),
            _make_snapshot(2, "gemini", stance=stance),
            _make_snapshot(3, "gemini", stance=stance),
        ]
        detected, keys = _check_stance_repetition(snaps, "gemini", window=2)
        assert detected is True
        assert sorted(keys) == ["attack_a", "attack_b"]

    def test_same_key_different_value_not_repetition(self):
        """Key 相同但 Value 不同 = 有实质进展，不算重复"""
        s1 = _make_stance(stances={"a": "reject"}, round_number=1)
        s2 = _make_stance(stances={"a": "reject"}, round_number=2)
        s3 = _make_stance(stances={"a": "needs_work"}, round_number=3)
        snaps = [
            _make_snapshot(1, "gemini", stance=s1),
            _make_snapshot(2, "gemini", stance=s2),
            _make_snapshot(3, "gemini", stance=s3),
        ]
        detected, keys = _check_stance_repetition(snaps, "gemini", window=2)
        assert detected is False
        assert keys == []

    def test_insufficient_rounds(self):
        """轮数不足 → 不检测"""
        stance = _make_stance(stances={"a": "reject"})
        snaps = [
            _make_snapshot(1, "gemini", stance=stance),
            _make_snapshot(2, "gemini", stance=stance),
        ]
        # window=2 需要至少 3 轮（1基准 + 2重复）
        detected, keys = _check_stance_repetition(snaps, "gemini", window=2)
        assert detected is False

    def test_filters_by_speaker(self):
        """只检测指定 speaker，忽略其他 speaker"""
        g_stance = _make_stance(stances={"a": "reject"})
        c_stance = _make_stance(stances={"a": "accept"})
        snaps = [
            _make_snapshot(1, "gemini", stance=g_stance),
            _make_snapshot(1, "codex", stance=c_stance),
            _make_snapshot(2, "gemini", stance=g_stance),
            _make_snapshot(2, "codex", stance=c_stance),
            _make_snapshot(3, "gemini", stance=g_stance),
            _make_snapshot(3, "codex", stance=c_stance),
        ]
        # Gemini 连续 3 轮相同 → 重复
        detected, keys = _check_stance_repetition(snaps, "gemini", window=2)
        assert detected is True
        assert keys == ["a"]
        # Codex 也连续 3 轮相同 → 重复
        detected_c, keys_c = _check_stance_repetition(snaps, "codex", window=2)
        assert detected_c is True
        assert keys_c == ["a"]

    def test_none_stance_not_treated_as_repetition(self):
        """stance 解析失败（None）不视为重复"""
        snaps = [
            _make_snapshot(1, "gemini", stance=None),
            _make_snapshot(2, "gemini", stance=None),
            _make_snapshot(3, "gemini", stance=None),
        ]
        detected, keys = _check_stance_repetition(snaps, "gemini", window=2)
        assert detected is False
        assert keys == []

    def test_empty_stances_dict_is_repetition(self):
        """空 stances dict 连续出现 = verdict 重复但无 key 内容。

        空 stance_snapshot 是空 frozenset，baseline 为空 → 不进入重复
        （因为空集不携带攻击信息）。
        """
        s = _make_stance(stances={}, round_number=1)
        snaps = [
            _make_snapshot(1, "gemini", stance=s),
            _make_snapshot(2, "gemini", stance=s),
            _make_snapshot(3, "gemini", stance=s),
        ]
        detected, keys = _check_stance_repetition(snaps, "gemini", window=2)
        # 空 stance = 无攻击内容，不应被检测为重复
        assert detected is False

    def test_window_1(self):
        """window=1 时，连续 2 轮相同即检测到重复"""
        stance = _make_stance(stances={"x": "reject"})
        snaps = [
            _make_snapshot(1, "gemini", stance=stance),
            _make_snapshot(2, "gemini", stance=stance),
        ]
        detected, keys = _check_stance_repetition(snaps, "gemini", window=1)
        assert detected is True
        assert keys == ["x"]

    def test_repetition_broken_by_change_in_middle(self):
        """中间有变化，最近2轮相同但基准不同 → 不重复"""
        s1 = _make_stance(stances={"a": "reject"}, round_number=1)
        s2 = _make_stance(stances={"a": "accept"}, round_number=2)
        s3 = _make_stance(stances={"a": "reject"}, round_number=3)
        s4 = _make_stance(stances={"a": "reject"}, round_number=4)
        snaps = [
            _make_snapshot(1, "gemini", stance=s1),
            _make_snapshot(2, "gemini", stance=s2),
            _make_snapshot(3, "gemini", stance=s3),
            _make_snapshot(4, "gemini", stance=s4),
        ]
        # 最近 3 轮 = s2, s3, s4。s2 != s3 → 不重复
        detected, keys = _check_stance_repetition(snaps, "gemini", window=2)
        assert detected is False


# ── Trajectories tests ──


class TestBuildTrajectories:
    def test_unresolved_from_divergent_stances(self):
        """双方 stance 不同的 Key → unresolved → trajectory"""
        g_stance = _make_stance(stances={"a": "reject", "b": "accept"})
        c_stance = _make_stance(stances={"a": "accept", "b": "accept"})
        snaps = [
            _make_snapshot(1, "gemini", stance=g_stance),
            _make_snapshot(1, "codex", stance=c_stance),
        ]
        keys = frozenset({"a", "b"})
        trajs = build_trajectories(snaps, keys)
        assert len(trajs) == 1  # only 'a' is divergent
        assert trajs[0].lack_key == "a"
        assert "reject" in trajs[0].lack_description
        assert "accept" in trajs[0].lack_description

    def test_no_trajectory_when_agreed(self):
        """双方 stance 相同 → 无 trajectory"""
        stance = _make_stance(stances={"a": "accept"})
        snaps = [
            _make_snapshot(1, "gemini", stance=stance),
            _make_snapshot(1, "codex", stance=stance),
        ]
        trajs = build_trajectories(snaps, frozenset({"a"}))
        assert len(trajs) == 0

    def test_single_speaker_generates_silence_trajectory(self):
        """单方有 stance 另一方沉默 → (stance, null) trajectory"""
        g_stance = _make_stance(stances={"a": "reject"})
        snaps = [
            _make_snapshot(1, "gemini", stance=g_stance),
            _make_snapshot(1, "codex", stance=None),
        ]
        trajs = build_trajectories(snaps, frozenset({"a"}))
        assert len(trajs) == 1
        assert "沉默" in trajs[0].lack_description

    def test_empty_snapshots(self):
        trajs = build_trajectories([], frozenset())
        assert trajs == ()


# ── Build cycle result tests ──


class TestBuildCycleResult:
    def test_basic_structure(self):
        """_build_cycle_result 正确组装 InquiryCycleResult"""
        g1 = _make_stance(
            stances={"a": "reject"}, round_number=1,
        )
        g2 = _make_stance(
            stances={"a": "accept"}, round_number=2,
            concessions=["a"],
        )
        c1 = _make_stance(stances={"a": "accept"}, round_number=1)
        c2 = _make_stance(stances={"a": "accept"}, round_number=2)

        snaps = [
            _make_snapshot(1, "gemini", stance=g1),
            _make_snapshot(1, "codex", stance=c1),
            _make_snapshot(2, "gemini", stance=g2),
            _make_snapshot(2, "codex", stance=c2),
        ]
        keys = frozenset({"a"})
        trajs = build_trajectories(snaps, keys)
        result = build_cycle_result(snaps, "trigger-123", keys, trajs)

        assert isinstance(result, InquiryCycleResult)
        assert result.trigger_block_id == "trigger-123"
        assert result.scenario == "gemini_verify"
        # Gemini changed a: reject → accept → computed concession
        assert len(result.gemini_conceded) > 0

    def test_conceded_from_stance_change(self):
        """stance 从 reject→accept → conceded"""
        g1 = _make_stance(stances={"issue_x": "reject"}, round_number=1)
        g2 = _make_stance(stances={"issue_x": "accept"}, round_number=2)
        snaps = [
            _make_snapshot(1, "gemini", stance=g1),
            _make_snapshot(2, "gemini", stance=g2),
        ]
        result = build_cycle_result(
            snaps, "t", frozenset({"issue_x"}), (),
        )
        assert "issue_x" in result.gemini_conceded


# ── Full loop mock tests ──


class TestRunInquiryLoop:
    def _make_mock_challengers(
        self,
        gemini_responses: list[str],
        codex_responses: list[str],
    ):
        """Create mock Gemini and Codex challengers."""

        @dataclass
        class FakeResult:
            response: str
            mode: str = "challenge"
            subject: str = ""
            model: str = "mock"

        gemini_mock = MagicMock()
        gemini_mock.challenge.side_effect = [
            FakeResult(response=r) for r in gemini_responses
        ]

        codex_mock = MagicMock()
        codex_mock.review.side_effect = [
            FakeResult(response=r, mode="review") for r in codex_responses
        ]

        return gemini_mock, codex_mock

    def test_same_subject_stable_converges(self, tmp_base):
        """同主体 diff 为空 + Key 冻结 → converged=True"""
        # 3 轮：每轮相同的 stance → 收敛
        g_response = _make_response_with_stance(
            verdict="fail",
            stances={"point_a": "reject"},
        )
        c_response = _make_response_with_stance(
            verdict="pass",
            stances={"point_a": "accept"},
        )
        gemini_mock, codex_mock = self._make_mock_challengers(
            [g_response] * 3, [c_response] * 3,
        )

        result = run_inquiry_loop(
            subject="test subject",
            context="test context",
            trigger_block_id="a" * 64,
            base=tmp_base,
            stability_window=3,
            gemini_challenger=gemini_mock,
            codex_challenger=codex_mock,
        )

        assert result.converged is True
        assert result.total_rounds == 3
        assert result.verdict.key_frozen is True
        assert result.verdict.same_subject_stable is True
        assert gemini_mock.challenge.call_count == 3
        assert codex_mock.review.call_count == 3

    def test_new_key_delays_convergence(self, tmp_base):
        """R2 引入新 Key → 至少需要再跑 stability_window 轮"""
        g_r1 = _make_response_with_stance(
            verdict="fail", stances={"k1": "reject"},
        )
        g_r2 = _make_response_with_stance(
            verdict="fail", stances={"k1": "reject", "k2": "reject"},
        )
        g_r3 = _make_response_with_stance(
            verdict="fail", stances={"k1": "reject", "k2": "reject"},
        )
        g_r4 = _make_response_with_stance(
            verdict="fail", stances={"k1": "reject", "k2": "reject"},
        )
        c_response = _make_response_with_stance(
            verdict="pass", stances={"k1": "accept", "k2": "accept"},
        )
        c_r1 = _make_response_with_stance(
            verdict="pass", stances={"k1": "accept"},
        )

        gemini_mock, codex_mock = self._make_mock_challengers(
            [g_r1, g_r2, g_r3, g_r4],
            [c_r1, c_response, c_response, c_response],
        )

        result = run_inquiry_loop(
            subject="test",
            context="",
            trigger_block_id="b" * 64,
            base=tmp_base,
            stability_window=3,
            gemini_challenger=gemini_mock,
            codex_challenger=codex_mock,
        )

        assert result.converged is True
        # Must have run more than 3 rounds due to new key in R2
        assert result.total_rounds > 2

    def test_full_loop_triggers_ceremony(self, tmp_base):
        """完整循环 → 收敛 → trigger_ceremony 被调用"""
        g_response = _make_response_with_stance(
            verdict="fail", stances={"a": "reject"},
        )
        c_response = _make_response_with_stance(
            verdict="pass", stances={"a": "accept"},
        )
        gemini_mock, codex_mock = self._make_mock_challengers(
            [g_response] * 3, [c_response] * 3,
        )

        result = run_inquiry_loop(
            subject="test",
            context="",
            trigger_block_id="c" * 64,
            base=tmp_base,
            stability_window=3,
            gemini_challenger=gemini_mock,
            codex_challenger=codex_mock,
        )

        assert result.converged is True
        assert result.cycle_result is not None
        # Verify ceremony blocks were written
        blocks_dir = tmp_base / "blocks"
        block_files = list(blocks_dir.glob("*.json"))
        assert len(block_files) >= 3  # consensus + residue + tension

    def test_stance_parse_failure_continues(self, tmp_base):
        """stance 解析失败不崩溃，round 记录 stance=None"""
        # R1-R2: no stance block → parse returns None
        # R3: stance block → parse succeeds, but still needs 3 rounds of stability
        # So we need rounds with no stance, then 3 rounds with stable stance
        no_stance_response = "Just some text without stance declaration."
        with_stance = _make_response_with_stance(
            verdict="pass", stances={"a": "accept"},
        )
        gemini_responses = [
            no_stance_response,  # R1: no stance
            with_stance,  # R2: has stance
            with_stance,  # R3: same stance
            with_stance,  # R4: same stance
        ]
        codex_responses = [
            no_stance_response,  # R1: no stance
            with_stance,  # R2: has stance
            with_stance,  # R3: same
            with_stance,  # R4: same
        ]
        gemini_mock, codex_mock = self._make_mock_challengers(
            gemini_responses, codex_responses,
        )

        result = run_inquiry_loop(
            subject="test",
            context="",
            trigger_block_id="d" * 64,
            base=tmp_base,
            stability_window=3,
            gemini_challenger=gemini_mock,
            codex_challenger=codex_mock,
        )

        assert result.converged is True
        # R1 should have stance=None
        r1_gemini = [
            s for s in result.snapshots
            if s.round_number == 1 and s.speaker == "gemini"
        ]
        assert len(r1_gemini) == 1
        assert r1_gemini[0].stance is None

    def test_trajectories_output(self, tmp_base):
        """收敛后产出 trajectories（互斥破裂轨迹）"""
        g_response = _make_response_with_stance(
            verdict="fail",
            stances={"issue_a": "reject", "issue_b": "accept"},
        )
        c_response = _make_response_with_stance(
            verdict="pass",
            stances={"issue_a": "accept", "issue_b": "accept"},
        )
        gemini_mock, codex_mock = self._make_mock_challengers(
            [g_response] * 3, [c_response] * 3,
        )

        result = run_inquiry_loop(
            subject="test",
            context="",
            trigger_block_id="e" * 64,
            base=tmp_base,
            stability_window=3,
            gemini_challenger=gemini_mock,
            codex_challenger=codex_mock,
        )

        assert result.converged is True
        # issue_a: Gemini=reject, Codex=accept → divergent → trajectory
        # issue_b: both accept → no trajectory
        assert len(result.trajectories) == 1
        assert result.trajectories[0].lack_key == "issue_a"

    def test_no_api_key_raises(self):
        """无 API key 时明确报错"""
        import os

        # Clear API keys to force error
        old_google = os.environ.pop("GOOGLE_API_KEY", None)
        old_openai = os.environ.pop("OPENAI_API_KEY", None)
        try:
            with pytest.raises(ValueError, match="GOOGLE_API_KEY"):
                run_inquiry_loop(
                    subject="test",
                    context="",
                    trigger_block_id="f" * 64,
                )
        finally:
            if old_google is not None:
                os.environ["GOOGLE_API_KEY"] = old_google
            if old_openai is not None:
                os.environ["OPENAI_API_KEY"] = old_openai
