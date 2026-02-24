"""Tests for scripts/consensus_trigger.py — 共识仪式触发协议

测试设计从总方针的形式要求出发：
- §21: residue.gemini_conceded = Gemini 放弃的立场，codex_conceded = Codex 放弃的立场
- §17: CC 是被质询的主体，不在 conceded 字段中
- §63: 当前阶段两个独立场景（Gemini verify / plan-review）
"""

from __future__ import annotations

import pytest

from scripts.block_topology import make_block, write_block
from scripts.consensus_trigger import (
    InquiryCycleResult,
    detect_convergence_from_review_file,
    extract_from_gemini_verify,
    extract_from_plan_review,
    find_trigger_block_for_review,
    trigger_ceremony,
    _extract_cross_round_concessions,
    _extract_list_items,
    _extract_section,
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


@pytest.fixture
def review_dir(tmp_path):
    """Provide a temporary review-results directory."""
    d = tmp_path / "review-results"
    d.mkdir()
    return d


# ── _extract_section tests ──


class TestExtractSection:
    def test_extracts_matching_section(self):
        text = "## 结论\n\n这是结论内容。\n\n## 下一节\n\n其他内容。"
        result = _extract_section(text, "结论")
        assert result == "这是结论内容。"

    def test_returns_empty_when_no_match(self):
        text = "## 无关标题\n\n内容"
        result = _extract_section(text, "结论")
        assert result == ""

    def test_tries_multiple_keywords(self):
        text = "### 结果\n\n最终结果在这里。"
        result = _extract_section(text, "结论", "结果")
        assert result == "最终结果在这里。"

    def test_extracts_until_end_if_no_next_heading(self):
        text = "## 结论\n\n内容到文件末尾"
        result = _extract_section(text, "结论")
        assert result == "内容到文件末尾"


# ── _extract_list_items tests ──


class TestExtractListItems:
    def test_extracts_items_under_keyword_heading(self):
        text = "## 被否定\n\n- 点A\n- 点B\n\n## 其他\n\n- 无关"
        result = _extract_list_items(text, "被否定")
        assert result == ["点A", "点B"]

    def test_extracts_items_containing_keyword(self):
        text = "- 这个包含误判信息\n- 无关项\n- 另一个误判"
        result = _extract_list_items(text, "误判")
        assert "这个包含误判信息" in result
        assert "另一个误判" in result
        assert "无关项" not in result

    def test_returns_empty_when_no_match(self):
        text = "## 无关\n\n- 项目1"
        result = _extract_list_items(text, "不存在的关键词")
        assert result == []

    def test_handles_star_prefix(self):
        text = "## CC让步\n\n* 让步点1\n* 让步点2"
        result = _extract_list_items(text, "CC让步")
        assert result == ["让步点1", "让步点2"]


# ── extract_from_gemini_verify tests ──
# §17: Gemini 是概念层质询者。§21: gemini_conceded = Gemini 放弃的立场。
# CC 是被质询的主体，不在 conceded 字段中。


class TestExtractFromGeminiVerify:
    def test_negation_stands_gemini_did_not_concede(self):
        """否定成立时，Gemini 坚持否定——没有让步。codex_conceded=[] 因为 Codex 不参与。"""
        text = (
            "## 结论\n\n定义X存在矛盾。\n\n"
            "## 未解决\n\n- 条件1与条件3的边界"
        )
        result = extract_from_gemini_verify(text, TRIGGER_SHA, negation_stands=True)

        assert result.scenario == "gemini_verify"
        assert result.trigger_block_id == TRIGGER_SHA
        assert "矛盾" in result.conclusion
        # §21: Gemini 坚持否定，没有让步
        assert result.gemini_conceded == []
        # §17: Codex 不参与此场景
        assert result.codex_conceded == []
        assert len(result.unresolved) == 1

    def test_negation_not_stands_gemini_conceded(self):
        """否定不成立时，Gemini 放弃了否定立场——gemini_conceded 记录这些立场。"""
        text = (
            "## 结论\n\n否定不成立，原定义正确。\n\n"
            "## 误判\n\n- Gemini 误读了上下文\n"
        )
        result = extract_from_gemini_verify(text, TRIGGER_SHA, negation_stands=False)

        # §21: Gemini 放弃了否定立场
        assert result.gemini_conceded == ["Gemini 误读了上下文"]
        # Codex 不参与
        assert result.codex_conceded == []
        assert "gemini" in result.concession_reasons

    def test_fallback_conclusion_from_text(self):
        text = "没有标题结构的纯文本回复"
        result = extract_from_gemini_verify(text, TRIGGER_SHA, negation_stands=True)
        assert result.conclusion == "没有标题结构的纯文本回复"

    def test_source_is_cc(self):
        """§17: CC 是主体（生产者）——source 始终为 cc。"""
        text = "## 结论\n\n结论内容"
        result = extract_from_gemini_verify(text, TRIGGER_SHA, negation_stands=True)
        assert result.source == "cc"


# ── extract_from_plan_review tests ──
# §18: Codex 从代码层质询。plan-review SKILL.md: Codex 在多轮对审中放弃的质疑。


class TestExtractFromPlanReview:
    def test_single_round(self, review_dir):
        (review_dir / "plan-review-20260224-0100-round1.md").write_text(
            "## 最终方案\n\n采用方案A。\n\n"
            "## Codex让步\n\n- 撤回对架构的质疑\n\n"
            "## 未解决\n\n- 性能基准待测\n",
            encoding="utf-8",
        )
        result = extract_from_plan_review(review_dir, TRIGGER_SHA)

        assert result is not None
        assert result.scenario == "plan_review"
        assert "方案A" in result.conclusion
        # §21: Gemini 不参与 plan-review
        assert result.gemini_conceded == []
        # §21: Codex 在对审中放弃的质疑
        assert len(result.codex_conceded) >= 1
        assert len(result.unresolved) >= 1

    def test_multiple_rounds_uses_last(self, review_dir):
        (review_dir / "plan-review-20260224-0100-round1.md").write_text(
            "## 结论\n\n初始方案。",
            encoding="utf-8",
        )
        (review_dir / "plan-review-20260224-0100-round2.md").write_text(
            "## 最终方案\n\n修正后方案B。\n\n"
            "## Codex让步\n\n- 接受重构方案\n",
            encoding="utf-8",
        )
        result = extract_from_plan_review(review_dir, TRIGGER_SHA)

        assert result is not None
        assert "方案B" in result.conclusion

    def test_no_files_returns_none(self, review_dir):
        result = extract_from_plan_review(review_dir, TRIGGER_SHA)
        assert result is None

    def test_gemini_conceded_always_empty(self, review_dir):
        """§63 当前阶段：plan-review 不涉及 Gemini。"""
        (review_dir / "plan-review-20260224-0200-round1.md").write_text(
            "## 结论\n\n方案定稿。",
            encoding="utf-8",
        )
        result = extract_from_plan_review(review_dir, TRIGGER_SHA)
        assert result is not None
        assert result.gemini_conceded == []


# ── _extract_cross_round_concessions tests ──
# §20: 共识=缝合，缝合必然生产剩余物。跨轮次消失的质疑 = 被缝合排除的内容。


class TestExtractCrossRoundConcessions:
    def test_detects_dropped_issues(self, tmp_path):
        r1 = tmp_path / "round1.md"
        r1.write_text("## 质疑\n\n- 架构问题\n- 性能问题\n", encoding="utf-8")
        r2 = tmp_path / "round2.md"
        r2.write_text("## 质疑\n\n- 性能问题\n", encoding="utf-8")

        rounds = {1: r1, 2: r2}
        conceded = _extract_cross_round_concessions(rounds)
        assert "架构问题" in conceded
        assert "性能问题" not in conceded

    def test_single_round_returns_empty(self, tmp_path):
        r1 = tmp_path / "round1.md"
        r1.write_text("## 质疑\n\n- 问题X\n", encoding="utf-8")
        assert _extract_cross_round_concessions({1: r1}) == []


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
        """§21: consensus.refs → 触发质询的原始 CC 产出区块 id。"""
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


# ── End-to-end: extract + trigger ──
# 从总方针出发验证完整路径：CC 产出 → 质询 → 共识仪式 → 三区块


class TestEndToEnd:
    def test_gemini_verify_negation_stands_e2e(self, tmp_base, trigger_block):
        """§19 共识="此处有问题"。Gemini 没有让步。residue 反映这个事实。"""
        verify_text = (
            "## 结论\n\n谱系 042 的依赖链存在循环引用。\n\n"
            "## 未解决\n\n- 038 号本身是否需要重审\n"
        )
        cycle = extract_from_gemini_verify(verify_text, trigger_block, negation_stands=True)
        result = trigger_ceremony(cycle, base=tmp_base)

        assert "042" in result["consensus"]["content"]["conclusion"]
        # Gemini 坚持否定，没有让步
        assert result["residue"]["content"]["gemini_conceded"] == []
        # Codex 不参与
        assert result["residue"]["content"]["codex_conceded"] == []
        assert len(result["tension"]["content"]["unresolved"]) >= 1

    def test_gemini_verify_negation_rejected_e2e(self, tmp_base, trigger_block):
        """§19 共识="原产出成立"。Gemini 放弃否定立场。"""
        verify_text = (
            "## 结论\n\n否定不成立，原定义正确。\n\n"
            "## 误判\n\n- Gemini 误读了包含关系定义\n"
        )
        cycle = extract_from_gemini_verify(verify_text, trigger_block, negation_stands=False)
        result = trigger_ceremony(cycle, base=tmp_base)

        assert result["residue"]["content"]["gemini_conceded"] == ["Gemini 误读了包含关系定义"]
        assert result["residue"]["content"]["codex_conceded"] == []

    def test_plan_review_e2e(self, tmp_base, trigger_block, review_dir):
        """§18 Codex 代码层质询 → §19 共识 → §21 三区块。"""
        (review_dir / "plan-review-20260224-0200-round1.md").write_text(
            "## 最终方案\n\n实现共识仪式触发协议。\n\n"
            "## Codex让步\n\n- 撤回对 hook 设计的质疑\n\n"
            "## 遗留\n\n- 需要集成测试\n",
            encoding="utf-8",
        )
        cycle = extract_from_plan_review(review_dir, trigger_block)
        assert cycle is not None

        result = trigger_ceremony(cycle, base=tmp_base)
        assert result["consensus"]["type"] == "consensus"
        assert trigger_block in result["consensus"]["refs"]
        # Gemini 不参与 plan-review
        assert result["residue"]["content"]["gemini_conceded"] == []
        # Codex 的让步被正确记录
        assert len(result["residue"]["content"]["codex_conceded"]) >= 1
