"""Tests for scripts/consensus_trigger.py — 立场差分架构

编排者决断：
- 让步 = 从位置 A 到位置 B 的移动（énonciation），不是陈述"我让步了"（énoncé）
- 正则匹配只能抓陈述，抓不到言说行为
- 立场差分架构：每轮质询输出结构化立场声明，让步 = 相邻轮次立场清单的差分

测试设计从总方针的形式要求出发：
- §21: residue.gemini_conceded = Gemini 放弃的立场，codex_conceded = Codex 放弃的立场
- §17: CC 是被质询的主体，不在 conceded 字段中
- §63: 当前阶段两个独立场景（Gemini verify / plan-review）
"""

from __future__ import annotations

import pytest

from scripts.block_topology import make_block, write_block
from scripts.consensus_trigger import (
    ConcessionTrace,
    InquiryCycleResult,
    StanceDeclaration,
    StanceDiff,
    compute_stance_diff,
    derive_concession_trace,
    detect_convergence_from_review_file,
    extract_from_gemini_verify,
    extract_from_plan_review,
    find_trigger_block_for_review,
    trigger_ceremony,
)

TRIGGER_SHA = "a" * 64


@pytest.fixture
def tmp_base(tmp_path):
    """Provide a temporary block-topology directory."""
    base = tmp_path / "block-topology"
    base.mkdir()
    (base / "blocks").mkdir()
    return base


@pytest.fixture
def trigger_block(tmp_base):
    """Write a trigger block and return its id."""
    blk = make_block("event", "cc", {"genealogy_id": "g-test"}, [])
    write_block(blk, tmp_base)
    return blk["id"]


# ── StanceDeclaration construction tests ──


class TestStanceDeclaration:
    def test_basic_construction(self):
        sd = StanceDeclaration(
            verdict="pass",
            stances={"point_a": "accept", "point_b": "reject"},
            round_number=1,
        )
        assert sd.verdict == "pass"
        assert sd.stances == {"point_a": "accept", "point_b": "reject"}
        assert sd.round_number == 1
        assert sd.self_reported_concessions == []

    def test_frozen(self):
        sd = StanceDeclaration(verdict="fail", stances={}, round_number=0)
        with pytest.raises(AttributeError):
            sd.verdict = "pass"  # type: ignore[misc]

    def test_with_self_reported_concessions(self):
        sd = StanceDeclaration(
            verdict="conditional",
            stances={"arch": "needs_rework"},
            round_number=2,
            self_reported_concessions=["I concede point X"],
        )
        assert sd.self_reported_concessions == ["I concede point X"]

    def test_defaults(self):
        sd = StanceDeclaration(verdict="pass", stances={})
        assert sd.round_number == 0
        assert sd.self_reported_concessions == []


# ── compute_stance_diff tests ──


class TestComputeStanceDiff:
    def test_no_change(self):
        earlier = StanceDeclaration(
            verdict="fail",
            stances={"a": "reject", "b": "accept"},
            round_number=1,
        )
        later = StanceDeclaration(
            verdict="fail",
            stances={"a": "reject", "b": "accept"},
            round_number=2,
        )
        diff = compute_stance_diff(earlier, later)
        assert diff.round_from == 1
        assert diff.round_to == 2
        assert diff.changed == {}
        assert diff.added == {}
        assert diff.removed == {}

    def test_stance_changed(self):
        earlier = StanceDeclaration(
            verdict="fail",
            stances={"arch": "reject", "perf": "accept"},
            round_number=1,
        )
        later = StanceDeclaration(
            verdict="pass",
            stances={"arch": "accept", "perf": "accept"},
            round_number=2,
        )
        diff = compute_stance_diff(earlier, later)
        assert diff.changed == {"arch": ("reject", "accept")}
        assert diff.added == {}
        assert diff.removed == {}

    def test_stance_removed(self):
        """立场消失 = 让步的直接证据"""
        earlier = StanceDeclaration(
            verdict="fail",
            stances={"issue_1": "reject", "issue_2": "needs_work"},
            round_number=1,
        )
        later = StanceDeclaration(
            verdict="pass",
            stances={"issue_2": "needs_work"},
            round_number=2,
        )
        diff = compute_stance_diff(earlier, later)
        assert diff.removed == {"issue_1": "reject"}
        assert diff.changed == {}
        assert diff.added == {}

    def test_stance_added(self):
        earlier = StanceDeclaration(
            verdict="fail",
            stances={"a": "reject"},
            round_number=1,
        )
        later = StanceDeclaration(
            verdict="fail",
            stances={"a": "reject", "b": "needs_work"},
            round_number=2,
        )
        diff = compute_stance_diff(earlier, later)
        assert diff.added == {"b": "needs_work"}
        assert diff.changed == {}
        assert diff.removed == {}

    def test_mixed_changes(self):
        earlier = StanceDeclaration(
            verdict="fail",
            stances={"a": "reject", "b": "accept", "c": "needs_work"},
            round_number=1,
        )
        later = StanceDeclaration(
            verdict="conditional",
            stances={"a": "accept", "d": "new_issue"},
            round_number=2,
        )
        diff = compute_stance_diff(earlier, later)
        # a changed from reject to accept
        assert diff.changed == {"a": ("reject", "accept")}
        # b and c removed
        assert diff.removed == {"b": "accept", "c": "needs_work"}
        # d added
        assert diff.added == {"d": "new_issue"}


# ── derive_concession_trace tests ──


class TestDeriveConcessionTrace:
    def test_single_round_empty_trace(self):
        """单轮无差分可算 -> 空 trace"""
        seq = [
            StanceDeclaration(
                verdict="pass",
                stances={"a": "accept"},
                round_number=1,
            ),
        ]
        trace = derive_concession_trace(seq)
        assert trace.computed_concessions == []
        assert trace.self_reported_concessions == []
        assert trace.divergence == []

    def test_two_rounds_with_concession(self):
        """两轮有让步 -> computed_concessions 有值。
        无自我报告时 divergence 记录差异（computed_not_self_reported）。"""
        seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"issue_1": "reject", "issue_2": "needs_work"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="pass",
                stances={"issue_2": "needs_work"},
                round_number=2,
            ),
        ]
        trace = derive_concession_trace(seq)
        # issue_1 was removed -> computed concession
        assert "issue_1" in trace.computed_concessions
        assert trace.self_reported_concessions == []
        # No self-report but computed concession -> divergence signals this
        assert len(trace.divergence) == 1
        assert any("issue_1" in d for d in trace.divergence)

    def test_changed_stance_is_concession(self):
        """立场从 reject 变为 accept = 让步"""
        seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"arch": "reject"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="pass",
                stances={"arch": "accept"},
                round_number=2,
            ),
        ]
        trace = derive_concession_trace(seq)
        assert "arch" in trace.computed_concessions

    def test_self_reported_matches_computed(self):
        """自我报告与计算一致 -> divergence 为空"""
        seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"issue_x": "reject"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="pass",
                stances={},
                round_number=2,
                self_reported_concessions=["issue_x"],
            ),
        ]
        trace = derive_concession_trace(seq)
        assert "issue_x" in trace.computed_concessions
        assert "issue_x" in trace.self_reported_concessions
        assert trace.divergence == []

    def test_self_reported_exceeds_computed(self):
        """自我报告多于计算 -> divergence 记录 agent 声称让步但数据未变"""
        seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"a": "reject"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="fail",
                stances={"a": "reject"},
                round_number=2,
                self_reported_concessions=["a"],
            ),
        ]
        trace = derive_concession_trace(seq)
        assert trace.computed_concessions == []
        assert "a" in trace.self_reported_concessions
        assert len(trace.divergence) >= 1
        # divergence should mention that agent claimed concession but data unchanged
        assert any("a" in d for d in trace.divergence)

    def test_computed_exceeds_self_reported(self):
        """计算多于自我报告 -> divergence 记录数据变化但 agent 未报告"""
        seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"x": "reject", "y": "needs_work"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="pass",
                stances={},
                round_number=2,
                self_reported_concessions=["x"],
            ),
        ]
        trace = derive_concession_trace(seq)
        assert "x" in trace.computed_concessions
        assert "y" in trace.computed_concessions
        assert "x" in trace.self_reported_concessions
        # y was computed but not self-reported
        assert len(trace.divergence) >= 1
        assert any("y" in d for d in trace.divergence)

    def test_three_rounds_cumulative(self):
        """三轮立场序列——让步在多个差分中累积"""
        seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"a": "reject", "b": "reject", "c": "reject"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="fail",
                stances={"b": "reject", "c": "accept"},
                round_number=2,
            ),
            StanceDeclaration(
                verdict="pass",
                stances={"c": "accept"},
                round_number=3,
            ),
        ]
        trace = derive_concession_trace(seq)
        # a removed in round 1->2, b removed in round 2->3, c changed in 1->2
        assert "a" in trace.computed_concessions
        assert "b" in trace.computed_concessions
        assert "c" in trace.computed_concessions

    def test_empty_sequence(self):
        trace = derive_concession_trace([])
        assert trace.computed_concessions == []
        assert trace.self_reported_concessions == []
        assert trace.divergence == []


# ── extract_from_gemini_verify tests ──
# §17: Gemini 是概念层质询者。§21: gemini_conceded = Gemini 放弃的立场。
# 立场差分架构：接收 stance_sequence 而不是 verify_result_text。


class TestExtractFromGeminiVerify:
    def test_negation_stands_no_concession(self):
        """否定成立时，Gemini 坚持否定——没有让步。codex_conceded=[] 因为 Codex 不参与。"""
        stance_seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"definition_x": "contradictory"},
                round_number=1,
            ),
        ]
        result = extract_from_gemini_verify(
            stance_sequence=stance_seq,
            trigger_block_id=TRIGGER_SHA,
            negation_stands=True,
            conclusion="定义X存在矛盾",
        )
        assert result.scenario == "gemini_verify"
        assert result.trigger_block_id == TRIGGER_SHA
        assert result.conclusion == "定义X存在矛盾"
        assert result.gemini_conceded == []
        assert result.codex_conceded == []
        assert result.concession_trace is not None

    def test_negation_not_stands_gemini_conceded(self):
        """否定不成立时，Gemini 放弃了否定立场——gemini_conceded 来自差分。"""
        stance_seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"inclusion_def": "incorrect", "context_read": "wrong"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="pass",
                stances={"context_read": "wrong"},
                round_number=2,
            ),
        ]
        result = extract_from_gemini_verify(
            stance_sequence=stance_seq,
            trigger_block_id=TRIGGER_SHA,
            negation_stands=False,
            conclusion="否定不成立，原定义正确",
        )
        # inclusion_def was removed -> concession
        assert "inclusion_def" in result.gemini_conceded
        assert result.codex_conceded == []
        assert result.concession_trace is not None

    def test_source_is_cc(self):
        """§17: CC 是主体（生产者）——source 始终为 cc。"""
        stance_seq = [
            StanceDeclaration(verdict="pass", stances={}, round_number=1),
        ]
        result = extract_from_gemini_verify(
            stance_sequence=stance_seq,
            trigger_block_id=TRIGGER_SHA,
            negation_stands=True,
            conclusion="test",
        )
        assert result.source == "cc"

    def test_stance_diffs_populated(self):
        """多轮时 stance_diffs 有值"""
        stance_seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"a": "reject"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="pass",
                stances={},
                round_number=2,
            ),
        ]
        result = extract_from_gemini_verify(
            stance_sequence=stance_seq,
            trigger_block_id=TRIGGER_SHA,
            negation_stands=False,
            conclusion="否定不成立",
        )
        assert len(result.stance_diffs) == 1
        assert result.stance_diffs[0].round_from == 1
        assert result.stance_diffs[0].round_to == 2

    def test_unresolved_from_last_stance(self):
        """unresolved 从最后一轮的 stances 中仍持有的非 pass 立场推导"""
        stance_seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"issue_1": "reject", "issue_2": "needs_work"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="conditional",
                stances={"issue_2": "needs_work"},
                round_number=2,
            ),
        ]
        result = extract_from_gemini_verify(
            stance_sequence=stance_seq,
            trigger_block_id=TRIGGER_SHA,
            negation_stands=False,
            conclusion="部分否定不成立",
            unresolved=["issue_2 待进一步审查"],
        )
        assert "issue_2 待进一步审查" in result.unresolved


# ── extract_from_plan_review tests ──
# §18: Codex 从代码层质询。plan-review: Codex 在多轮对审中放弃的质疑。
# 立场差分架构：接收 stance_sequence 而不是从文件系统读取。


class TestExtractFromPlanReview:
    def test_single_round_no_concession(self):
        stance_seq = [
            StanceDeclaration(
                verdict="pass",
                stances={"arch": "acceptable"},
                round_number=1,
            ),
        ]
        result = extract_from_plan_review(
            stance_sequence=stance_seq,
            trigger_block_id=TRIGGER_SHA,
            conclusion="方案定稿",
        )
        assert result.scenario == "plan_review"
        assert result.codex_conceded == []
        assert result.gemini_conceded == []

    def test_multi_round_codex_conceded(self):
        """Codex 在多轮对审中放弃质疑 -> codex_conceded 来自差分"""
        stance_seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"hook_design": "reject", "error_handling": "needs_work"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="pass",
                stances={"error_handling": "accept"},
                round_number=2,
            ),
        ]
        result = extract_from_plan_review(
            stance_sequence=stance_seq,
            trigger_block_id=TRIGGER_SHA,
            conclusion="修正后方案B",
        )
        # hook_design was removed -> concession
        assert "hook_design" in result.codex_conceded
        # error_handling changed from needs_work to accept -> also concession
        assert "error_handling" in result.codex_conceded
        assert result.gemini_conceded == []

    def test_gemini_conceded_always_empty(self):
        """§63 当前阶段：plan-review 不涉及 Gemini。"""
        stance_seq = [
            StanceDeclaration(verdict="pass", stances={}, round_number=1),
        ]
        result = extract_from_plan_review(
            stance_sequence=stance_seq,
            trigger_block_id=TRIGGER_SHA,
            conclusion="方案定稿",
        )
        assert result.gemini_conceded == []

    def test_concession_trace_attached(self):
        stance_seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"x": "reject"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="pass",
                stances={},
                round_number=2,
                self_reported_concessions=["x"],
            ),
        ]
        result = extract_from_plan_review(
            stance_sequence=stance_seq,
            trigger_block_id=TRIGGER_SHA,
            conclusion="完成",
        )
        assert result.concession_trace is not None
        assert "x" in result.concession_trace.computed_concessions
        assert "x" in result.concession_trace.self_reported_concessions
        assert result.concession_trace.divergence == []


# ── trigger_ceremony tests ──


class TestTriggerCeremony:
    def test_produces_three_blocks(self, tmp_base, trigger_block):
        cycle = InquiryCycleResult(
            scenario="gemini_verify",
            trigger_block_id=trigger_block,
            conclusion="测试结论",
            gemini_conceded=["放弃的否定立场"],
            codex_conceded=[],
            concession_reasons={"gemini": "否定不成立"},
            unresolved=["遗留问题"],
        )
        result = trigger_ceremony(cycle, base=tmp_base)

        assert "consensus" in result
        assert "residue" in result
        assert "tension" in result
        assert result["consensus"]["type"] == "consensus"
        assert result["residue"]["type"] == "residue"
        assert result["tension"]["type"] == "tension"

    def test_consensus_refs_trigger(self, tmp_base, trigger_block):
        """§21: consensus.refs -> 触发质询的原始 CC 产出区块 id。"""
        cycle = InquiryCycleResult(
            scenario="plan_review",
            trigger_block_id=trigger_block,
            conclusion="plan review 共识",
            gemini_conceded=[],
            codex_conceded=["撤回的质疑"],
            concession_reasons={"codex": "放弃质疑"},
            unresolved=[],
        )
        result = trigger_ceremony(cycle, base=tmp_base)
        assert trigger_block in result["consensus"]["refs"]

    def test_residue_preserves_concession_semantics(self, tmp_base, trigger_block):
        """§21: residue.gemini_conceded 和 codex_conceded 分别记录两个质询者的让步。"""
        cycle = InquiryCycleResult(
            scenario="gemini_verify",
            trigger_block_id=trigger_block,
            conclusion="否定不成立",
            gemini_conceded=["误读上下文"],
            codex_conceded=[],
            concession_reasons={"gemini": "误判"},
            unresolved=[],
        )
        result = trigger_ceremony(cycle, base=tmp_base)
        residue_content = result["residue"]["content"]
        assert residue_content["gemini_conceded"] == ["误读上下文"]
        assert residue_content["codex_conceded"] == []

    def test_with_stance_diff_data(self, tmp_base, trigger_block):
        """InquiryCycleResult 的新字段不影响 trigger_ceremony 的输出"""
        trace = ConcessionTrace(
            computed_concessions=["x"],
            self_reported_concessions=["x"],
            divergence=[],
        )
        diff = StanceDiff(
            round_from=1,
            round_to=2,
            changed={},
            added={},
            removed={"x": "reject"},
        )
        cycle = InquiryCycleResult(
            scenario="gemini_verify",
            trigger_block_id=trigger_block,
            conclusion="否定不成立",
            gemini_conceded=["x"],
            codex_conceded=[],
            concession_reasons={"gemini": "误判"},
            unresolved=[],
            concession_trace=trace,
            stance_diffs=[diff],
        )
        result = trigger_ceremony(cycle, base=tmp_base)
        # trigger_ceremony still calls write_consensus_ceremony with list[str]
        assert result["residue"]["content"]["gemini_conceded"] == ["x"]


# ── detect_convergence_from_review_file tests ──


class TestDetectConvergence:
    def test_plan_review_converged(self, tmp_path):
        f = tmp_path / "review.md"
        f.write_text("方案经过 3 轮对审，Codex 确认满意。[plan-reviewed]", encoding="utf-8")
        assert detect_convergence_from_review_file(f) is True

    def test_gemini_verify_converged_pass(self, tmp_path):
        f = tmp_path / "review.md"
        f.write_text("---\nresult: pass\n---\n正文", encoding="utf-8")
        assert detect_convergence_from_review_file(f) is True

    def test_gemini_verify_converged_fail(self, tmp_path):
        f = tmp_path / "review.md"
        f.write_text("---\nresult: fail\n---\n正文", encoding="utf-8")
        assert detect_convergence_from_review_file(f) is True

    def test_not_converged(self, tmp_path):
        f = tmp_path / "review.md"
        f.write_text("## 进行中\n\n第 2 轮质疑尚未回应。", encoding="utf-8")
        assert detect_convergence_from_review_file(f) is False

    def test_nonexistent_file(self, tmp_path):
        f = tmp_path / "nonexistent.md"
        assert detect_convergence_from_review_file(f) is False


# ── find_trigger_block_for_review tests ──


class TestFindTriggerBlock:
    def test_from_yaml_frontmatter_trigger(self, tmp_path):
        sha = "b" * 64
        f = tmp_path / "review.md"
        f.write_text(f"---\ntrigger: {sha}\nresult: pass\n---\n正文", encoding="utf-8")
        assert find_trigger_block_for_review(f) == sha

    def test_from_yaml_frontmatter_target(self, tmp_path):
        sha = "c" * 64
        f = tmp_path / "review.md"
        f.write_text(f"---\ntarget: {sha}\nresult: pass\n---\n正文", encoding="utf-8")
        assert find_trigger_block_for_review(f) == sha

    def test_no_frontmatter_returns_none(self, tmp_path):
        f = tmp_path / "review.md"
        f.write_text("# 无 frontmatter\n\n正文", encoding="utf-8")
        assert find_trigger_block_for_review(f) is None


# ── End-to-end: stance_sequence -> InquiryCycleResult -> trigger_ceremony -> 三区块 ──


class TestEndToEnd:
    def test_gemini_verify_negation_stands_e2e(self, tmp_base, trigger_block):
        """§19 共识="此处有问题"。Gemini 没有让步。residue 反映这个事实。"""
        stance_seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"dep_chain_042": "circular_reference"},
                round_number=1,
            ),
        ]
        cycle = extract_from_gemini_verify(
            stance_sequence=stance_seq,
            trigger_block_id=trigger_block,
            negation_stands=True,
            conclusion="谱系 042 的依赖链存在循环引用",
        )
        result = trigger_ceremony(cycle, base=tmp_base)

        assert "042" in result["consensus"]["content"]["conclusion"]
        assert result["residue"]["content"]["gemini_conceded"] == []
        assert result["residue"]["content"]["codex_conceded"] == []

    def test_gemini_verify_negation_rejected_e2e(self, tmp_base, trigger_block):
        """§19 共识="原产出成立"。Gemini 放弃否定立场。"""
        stance_seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"inclusion_def": "misread"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="pass",
                stances={},
                round_number=2,
            ),
        ]
        cycle = extract_from_gemini_verify(
            stance_sequence=stance_seq,
            trigger_block_id=trigger_block,
            negation_stands=False,
            conclusion="否定不成立，原定义正确",
        )
        result = trigger_ceremony(cycle, base=tmp_base)

        assert "inclusion_def" in result["residue"]["content"]["gemini_conceded"]
        assert result["residue"]["content"]["codex_conceded"] == []

    def test_plan_review_e2e(self, tmp_base, trigger_block):
        """§18 Codex 代码层质询 -> §19 共识 -> §21 三区块。"""
        stance_seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"hook_design": "reject", "api_surface": "too_broad"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="conditional",
                stances={"api_surface": "acceptable"},
                round_number=2,
            ),
            StanceDeclaration(
                verdict="pass",
                stances={},
                round_number=3,
            ),
        ]
        cycle = extract_from_plan_review(
            stance_sequence=stance_seq,
            trigger_block_id=trigger_block,
            conclusion="实现共识仪式触发协议",
        )

        result = trigger_ceremony(cycle, base=tmp_base)
        assert result["consensus"]["type"] == "consensus"
        assert trigger_block in result["consensus"]["refs"]
        assert result["residue"]["content"]["gemini_conceded"] == []
        assert len(result["residue"]["content"]["codex_conceded"]) >= 1

    def test_divergence_signal_preserved_e2e(self, tmp_base, trigger_block):
        """自我报告 vs 计算出的让步之间的差异被保留在 concession_trace 中"""
        stance_seq = [
            StanceDeclaration(
                verdict="fail",
                stances={"x": "reject", "y": "reject"},
                round_number=1,
            ),
            StanceDeclaration(
                verdict="pass",
                stances={},
                round_number=2,
                self_reported_concessions=["x"],
                # y also removed but not self-reported
            ),
        ]
        cycle = extract_from_gemini_verify(
            stance_sequence=stance_seq,
            trigger_block_id=trigger_block,
            negation_stands=False,
            conclusion="否定不成立",
        )
        # Verify divergence is captured
        assert cycle.concession_trace is not None
        assert "y" in cycle.concession_trace.computed_concessions
        assert "y" not in cycle.concession_trace.self_reported_concessions
        assert len(cycle.concession_trace.divergence) >= 1

        # Still produces valid ceremony
        result = trigger_ceremony(cycle, base=tmp_base)
        assert result["consensus"]["type"] == "consensus"
