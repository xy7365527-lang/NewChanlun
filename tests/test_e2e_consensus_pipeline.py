"""端到端集成测试：质询回复 → 立场解析 → 差分计算 → 共识仪式写入。

验证整个管道从 Gemini/Codex 回复文本到 block-topology 三区块的完整连通性。
每个测试从构造原始回复文本开始，经过全部中间层，最终验证磁盘上的区块状态。
"""

from __future__ import annotations

import pytest

from scripts.block_topology import list_blocks, make_block, read_block, write_block
from scripts.consensus_trigger import (
    extract_from_gemini_verify,
    extract_from_plan_review,
    trigger_ceremony,
)
from scripts.stance_parser import parse_stance_declaration, parse_stance_sequence


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
    blk = make_block("event", "cc", {"genealogy_id": "g-e2e-test"}, [])
    write_block(blk, tmp_base)
    return blk["id"]


# ── Test 1: Gemini verify 场景端到端 ──


class TestGeminiVerifyE2E:
    """从 Gemini 回复原文 → parse → extract → trigger → 验证 block-topology。"""

    ROUND_1_REPLY = """\
## 质询分析

经审查，谱系 042 的依赖链定义存在循环引用问题。
definition_a 依赖 definition_b，而 definition_b 又引用了 definition_a。
此外，上下文读取方式与原始博文第 67 课的描述不一致。

---stance-declaration---
verdict: fail
stances:
  dep_chain_circular: contradictory
  context_read: incorrect
  definition_completeness: accept
concessions: []
---end-stance---
"""

    ROUND_2_REPLY = """\
## 重新审视

经 CC 方回应后，dep_chain_circular 的循环引用问题确认不存在——
编纂版遗漏了第 67 课的补充说明。但 context_read 问题仍然成立。

---stance-declaration---
verdict: conditional
stances:
  context_read: incorrect
  definition_completeness: accept
concessions:
  - dep_chain_circular
---end-stance---
"""

    def test_full_pipeline_negation_not_stands(self, tmp_base, trigger_block):
        """Gemini 否定不成立路径：回复文本 → 解析 → 差分 → 仪式 → 区块验证。"""
        # Step 1: 解析两轮回复文本
        texts = [self.ROUND_1_REPLY, self.ROUND_2_REPLY]
        stance_seq = parse_stance_sequence(texts, start_round=1)

        assert len(stance_seq) == 2
        assert stance_seq[0].verdict == "fail"
        assert stance_seq[1].verdict == "conditional"
        assert "dep_chain_circular" in stance_seq[0].stances
        assert "dep_chain_circular" not in stance_seq[1].stances

        # Step 2: 从立场序列提取质询循环结果
        cycle = extract_from_gemini_verify(
            stance_sequence=stance_seq,
            trigger_block_id=trigger_block,
            negation_stands=False,
            conclusion="dep_chain_circular 否定不成立，context_read 待进一步审查",
            unresolved=["context_read 仍存在争议"],
        )

        assert cycle.scenario == "gemini_verify"
        assert cycle.trigger_block_id == trigger_block
        # dep_chain_circular 从 R1 消失于 R2 → computed concession
        assert "dep_chain_circular" in cycle.gemini_conceded
        assert cycle.codex_conceded == []

        # Step 3: concession_trace 的 divergence 检查
        trace = cycle.concession_trace
        assert trace is not None
        # dep_chain_circular 在自我报告和计算中都有 → 无 divergence
        assert "dep_chain_circular" in trace.computed_concessions
        assert "dep_chain_circular" in trace.self_reported_concessions

        # Step 4: 触发共识仪式写入
        result = trigger_ceremony(cycle, base=tmp_base)

        # Step 5: 验证 block-topology 中的区块
        consensus_blocks = list_blocks(tmp_base, block_type="consensus")
        residue_blocks = list_blocks(tmp_base, block_type="residue")
        tension_blocks = list_blocks(tmp_base, block_type="tension")

        assert len(consensus_blocks) == 1
        assert len(residue_blocks) == 1
        assert len(tension_blocks) == 1

        # consensus.refs 指向 trigger block
        consensus = read_block(result["consensus"]["id"], tmp_base)
        assert consensus is not None
        assert trigger_block in consensus["refs"]
        assert "dep_chain_circular" in consensus["content"]["conclusion"]

        # residue 记录让步
        residue = read_block(result["residue"]["id"], tmp_base)
        assert residue is not None
        assert "dep_chain_circular" in residue["content"]["gemini_conceded"]
        assert residue["content"]["codex_conceded"] == []

        # tension 记录未解决项
        tension = read_block(result["tension"]["id"], tmp_base)
        assert tension is not None
        assert len(tension["content"]["unresolved"]) >= 1

    def test_full_pipeline_negation_stands(self, tmp_base, trigger_block):
        """Gemini 否定成立路径：单轮回复 → 无让步 → 仪式写入。"""
        texts = [self.ROUND_1_REPLY]
        stance_seq = parse_stance_sequence(texts, start_round=1)
        assert len(stance_seq) == 1

        cycle = extract_from_gemini_verify(
            stance_sequence=stance_seq,
            trigger_block_id=trigger_block,
            negation_stands=True,
            conclusion="谱系 042 依赖链循环引用确认存在",
        )

        assert cycle.gemini_conceded == []
        assert cycle.codex_conceded == []

        result = trigger_ceremony(cycle, base=tmp_base)

        residue = read_block(result["residue"]["id"], tmp_base)
        assert residue is not None
        assert residue["content"]["gemini_conceded"] == []
        assert residue["content"]["codex_conceded"] == []


# ── Test 2: Plan-review 场景端到端 ──


class TestPlanReviewE2E:
    """从 Codex 多轮对审回复 → parse → extract → trigger → 验证 block-topology。"""

    ROUND_1_CODEX = """\
## Code Review — Round 1

hook_design 存在严重问题：当前的 PreToolUse hook 在错误场景下不会触发。
error_handling 中 try/except 过于宽泛，可能吞掉关键异常。
api_surface 过大，建议拆分。

---stance-declaration---
verdict: fail
stances:
  hook_design: reject
  error_handling: needs_work
  api_surface: too_broad
concessions: []
---end-stance---
"""

    ROUND_2_CODEX = """\
## Code Review — Round 2

Opus 方回应后，hook_design 的 PreToolUse 触发路径已补全。
error_handling 仍需修改，但问题范围缩小——只有 consensus_ceremony 中的 shutil.move 需要改。
api_surface 的拆分理由不充分，撤回此质疑。

---stance-declaration---
verdict: conditional
stances:
  error_handling: needs_work
concessions:
  - hook_design
  - api_surface
---end-stance---
"""

    ROUND_3_CODEX = """\
## Code Review — Round 3

error_handling 已修复。全部质疑解决。

---stance-declaration---
verdict: pass
stances: {}
concessions:
  - error_handling
---end-stance---
"""

    def test_three_round_plan_review(self, tmp_base, trigger_block):
        """三轮 Codex 对审的完整管道。"""
        texts = [self.ROUND_1_CODEX, self.ROUND_2_CODEX, self.ROUND_3_CODEX]
        stance_seq = parse_stance_sequence(texts, start_round=1)

        assert len(stance_seq) == 3
        assert stance_seq[0].verdict == "fail"
        assert stance_seq[1].verdict == "conditional"
        assert stance_seq[2].verdict == "pass"

        # Step 2: 提取
        cycle = extract_from_plan_review(
            stance_sequence=stance_seq,
            trigger_block_id=trigger_block,
            conclusion="三轮对审后方案定稿",
            unresolved=[],
        )

        assert cycle.scenario == "plan_review"
        assert cycle.gemini_conceded == []
        # R1→R2: hook_design removed, api_surface removed, error_handling changed value
        # R2→R3: error_handling removed
        # 全部在 codex_conceded 中
        assert "hook_design" in cycle.codex_conceded
        assert "api_surface" in cycle.codex_conceded
        assert "error_handling" in cycle.codex_conceded

        # Step 3: 自我报告匹配度
        trace = cycle.concession_trace
        assert trace is not None
        # hook_design 自我报告 + 计算都有
        assert "hook_design" in trace.self_reported_concessions
        assert "hook_design" in trace.computed_concessions

        # Step 4: 触发仪式
        result = trigger_ceremony(cycle, base=tmp_base)

        # Step 5: 验证磁盘
        consensus_blocks = list_blocks(tmp_base, block_type="consensus")
        residue_blocks = list_blocks(tmp_base, block_type="residue")
        tension_blocks = list_blocks(tmp_base, block_type="tension")

        assert len(consensus_blocks) == 1
        assert len(residue_blocks) == 1
        assert len(tension_blocks) == 1

        residue = read_block(result["residue"]["id"], tmp_base)
        assert residue is not None
        assert residue["content"]["gemini_conceded"] == []
        assert len(residue["content"]["codex_conceded"]) >= 3

        consensus = read_block(result["consensus"]["id"], tmp_base)
        assert consensus is not None
        assert trigger_block in consensus["refs"]


# ── Test 3: Divergence 信号端到端 ──


class TestDivergenceSignalE2E:
    """自我报告的让步与实际立场变化不一致时 divergence 被正确捕获。"""

    ROUND_1 = """\
质询开始。

---stance-declaration---
verdict: fail
stances:
  issue_alpha: reject
  issue_beta: needs_work
  issue_gamma: reject
concessions: []
---end-stance---
"""

    ROUND_2 = """\
部分问题已解决。

---stance-declaration---
verdict: conditional
stances:
  issue_gamma: reject
concessions:
  - issue_alpha
  - issue_delta
---end-stance---
"""
    # issue_alpha: removed (computed concession) + self-reported → match
    # issue_beta: removed (computed concession) but NOT self-reported → divergence
    # issue_delta: self-reported but NOT in stances at all → divergence

    def test_divergence_detection(self, tmp_base, trigger_block):
        texts = [self.ROUND_1, self.ROUND_2]
        stance_seq = parse_stance_sequence(texts, start_round=1)
        assert len(stance_seq) == 2

        cycle = extract_from_gemini_verify(
            stance_sequence=stance_seq,
            trigger_block_id=trigger_block,
            negation_stands=False,
            conclusion="部分否定不成立",
            unresolved=["issue_gamma 仍争议中"],
        )

        trace = cycle.concession_trace
        assert trace is not None

        # issue_alpha: computed + self-reported → no divergence for this item
        assert "issue_alpha" in trace.computed_concessions
        assert "issue_alpha" in trace.self_reported_concessions

        # issue_beta: computed but NOT self-reported → divergence
        assert "issue_beta" in trace.computed_concessions
        assert "issue_beta" not in trace.self_reported_concessions
        divergence_texts = " ".join(trace.divergence)
        assert "issue_beta" in divergence_texts

        # issue_delta: self-reported but NOT computed → divergence
        assert "issue_delta" not in trace.computed_concessions
        assert "issue_delta" in trace.self_reported_concessions
        assert "issue_delta" in divergence_texts

        # 尽管有 divergence，仪式仍然正常写入
        result = trigger_ceremony(cycle, base=tmp_base)
        assert result["consensus"]["type"] == "consensus"
        assert result["residue"]["type"] == "residue"
        assert result["tension"]["type"] == "tension"

        # block-topology 中三类区块各一个
        assert len(list_blocks(tmp_base, block_type="consensus")) == 1
        assert len(list_blocks(tmp_base, block_type="residue")) == 1
        assert len(list_blocks(tmp_base, block_type="tension")) == 1


# ── Test 4: 无立场声明块的回复（降级场景） ──


class TestGracefulDegradation:
    """Gemini/Codex 回复不包含 ---stance-declaration--- 块时的降级行为。"""

    REPLY_NO_BLOCK = """\
## 审查结论

经审查，该谱系定义在逻辑上是自洽的。无否定意见。

总体评价：通过。
"""

    REPLY_MALFORMED_YAML = """\
审查结论。

---stance-declaration---
verdict: fail
stances: {{{invalid yaml
---end-stance---
"""

    REPLY_INVALID_VERDICT = """\
---stance-declaration---
verdict: maybe
stances:
  a: b
---end-stance---
"""

    def test_no_stance_block_returns_none(self):
        """回复中不包含声明块 → parse 返回 None。"""
        sd = parse_stance_declaration(self.REPLY_NO_BLOCK, round_number=1)
        assert sd is None

    def test_sequence_with_no_blocks_returns_empty(self):
        """多轮回复全部无声明块 → 空序列。"""
        texts = [self.REPLY_NO_BLOCK, self.REPLY_NO_BLOCK]
        seq = parse_stance_sequence(texts, start_round=1)
        assert seq == []

    def test_malformed_yaml_returns_none(self):
        """声明块内的 YAML 格式错误 → parse 返回 None。"""
        sd = parse_stance_declaration(self.REPLY_MALFORMED_YAML, round_number=1)
        assert sd is None

    def test_invalid_verdict_returns_none(self):
        """verdict 值不在合法范围内 → parse 返回 None。"""
        sd = parse_stance_declaration(self.REPLY_INVALID_VERDICT, round_number=1)
        assert sd is None

    def test_mixed_valid_and_invalid_rounds(self):
        """部分轮次有效、部分无效 → 只返回有效的。"""
        valid_reply = """\
回复。

---stance-declaration---
verdict: pass
stances:
  overall: no_issues
concessions: []
---end-stance---
"""
        texts = [
            self.REPLY_NO_BLOCK,
            valid_reply,
            self.REPLY_MALFORMED_YAML,
            self.REPLY_INVALID_VERDICT,
        ]
        seq = parse_stance_sequence(texts, start_round=1)
        assert len(seq) == 1
        assert seq[0].round_number == 2
        assert seq[0].verdict == "pass"

    def test_empty_sequence_still_produces_valid_cycle(self, tmp_base, trigger_block):
        """即使解析出空序列，extract 函数不崩溃——产出空让步的结果。"""
        empty_seq = parse_stance_sequence(
            [self.REPLY_NO_BLOCK, self.REPLY_NO_BLOCK],
            start_round=1,
        )
        assert empty_seq == []

        # extract_from_gemini_verify 接受空序列
        cycle = extract_from_gemini_verify(
            stance_sequence=empty_seq,
            trigger_block_id=trigger_block,
            negation_stands=True,
            conclusion="无立场声明可用，默认否定成立",
        )
        assert cycle.gemini_conceded == []
        assert cycle.codex_conceded == []
        assert cycle.concession_trace is not None
        assert cycle.concession_trace.computed_concessions == []

        # trigger_ceremony 正常写入
        result = trigger_ceremony(cycle, base=tmp_base)
        assert result["consensus"]["type"] == "consensus"
        assert result["residue"]["type"] == "residue"
        assert result["tension"]["type"] == "tension"

        # 磁盘验证
        assert len(list_blocks(tmp_base, block_type="consensus")) == 1

    def test_empty_sequence_plan_review(self, tmp_base, trigger_block):
        """plan-review 场景空序列同样不崩溃。"""
        empty_seq = []

        cycle = extract_from_plan_review(
            stance_sequence=empty_seq,
            trigger_block_id=trigger_block,
            conclusion="无立场声明可用",
        )
        assert cycle.codex_conceded == []
        assert cycle.gemini_conceded == []

        result = trigger_ceremony(cycle, base=tmp_base)
        assert len(list_blocks(tmp_base, block_type="consensus")) == 1
