"""共识仪式触发协议——立场差分架构。

编排者决断（本体论位置）：
- 让步 ≠ 陈述"我让步了"（énoncé），让步 = 从位置 A 到位置 B 的移动（énonciation）
- 正则匹配只能抓陈述，抓不到言说行为
- 立场差分架构替代事后正则提取：
  - 每轮质询输出结构化格式：判定 + 立场清单（KV 对）
  - 让步 = 相邻轮次立场清单的差分
  - Agent 不自我报告让步，系统从立场差分中推导
  - 自我报告的让步 vs 被计算出的让步之间的差异本身也是信号

推导路径（从总方针出发）：

§17: 三个主体位置——CC（主体/生产者）、Gemini（概念层质询者）、Codex（代码层质询者）。
§18: CC 的每一个产出进入双重质询循环。质询追溯性地改写 CC 产出在谱系中的位置。
§19: 质询到共识为止。共识可能一方被说服，可能发现第三条路。
§20: Real 在共识处。共识 = 缝合 = 必然生产剩余物。
§21: 共识仪式三区块原子写入——consensus(结论) + residue(双方让步) + tension(未解决)。
     consensus.refs → 触发质询的原始 CC 产出区块 id。
     residue.content.gemini_conceded → Gemini 放弃了什么。
     residue.content.codex_conceded → Codex 放弃了什么。
§63: 当前缺口——让步轨迹在对话历史中但未被结构化提取为 residue 区块。

当前阶段的实际形态（§63 承认）：
  - 双重质询循环尚未完全联动——Gemini 和 Codex 由不同事件独立触发
  - 场景 A: genealogy_settlement → Gemini verify（概念层，单方质询）
  - 场景 B: plan_review → Codex review 多轮对审（代码层，Opus-Codex 协商）
  - 场景 C: Gemini decide（单方决策，无质询对手方，不触发共识仪式）

residue 字段语义（§21 严格定义）：
  - gemini_conceded: Gemini 在质询过程中放弃的立场
  - codex_conceded: Codex 在质询过程中放弃的立场
  - 只有 Gemini 参与时 codex_conceded = []
  - 只有 Codex 参与时 gemini_conceded = []
  - CC 不在 conceded 字段中——CC 是被质询的主体（§17），不是质询者
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Literal

from scripts.block_topology import DEFAULT_BASE
from scripts.consensus_ceremony import write_consensus_ceremony


# ── 立场差分数据结构 ──


@dataclass(frozen=True, slots=True)
class StanceDeclaration:
    """单轮质询的结构化立场声明。

    编排者决断：至少两个字段——判定 + 立场清单（KV 对）。
    """

    verdict: Literal["pass", "fail", "conditional"]
    stances: dict[str, str]  # 具体点 → 持有的立场
    round_number: int = 0
    self_reported_concessions: list[str] = field(default_factory=list)
    # 自我报告的让步（可选）——与系统计算的让步做对比时用


@dataclass(frozen=True, slots=True)
class StanceDiff:
    """相邻轮次之间的立场变化。"""

    round_from: int
    round_to: int
    changed: dict[str, tuple[str, str]]  # key → (old_stance, new_stance)
    added: dict[str, str]  # 新增的立场条目
    removed: dict[str, str]  # 消失的立场条目（= 让步的直接证据）


@dataclass(frozen=True, slots=True)
class ConcessionTrace:
    """从立场差分推导出的让步轨迹。"""

    computed_concessions: list[str]  # 系统计算：从差分推导
    self_reported_concessions: list[str]  # 自我报告：agent 声称的
    divergence: list[str]  # 两者的差异（本身是信号）


@dataclass(frozen=True, slots=True)
class InquiryCycleResult:
    """质询循环的结构化产出——从立场差分中推导。

    字段语义遵循 §21 的定义：
    - trigger_block_id: 触发质询的原始 CC 产出区块 id（§21 consensus.refs）
    - conclusion: 最终达成的结论（§21 consensus.content.conclusion）
    - gemini_conceded: Gemini 在质询中放弃的立场（§21 residue 字段）
    - codex_conceded: Codex 在质询中放弃的立场（§21 residue 字段）
    - concession_reasons: 让步理由（§21 residue.content.reasons）
    - unresolved: 双方承认未解决但暂时搁置的部分（§21 tension.content.unresolved）
    - concession_trace: 从立场差分推导的让步轨迹（含 divergence 信号）
    - stance_diffs: 所有相邻轮次的差分列表
    """

    scenario: Literal["gemini_verify", "plan_review"]
    trigger_block_id: str
    conclusion: str
    gemini_conceded: list[str]
    codex_conceded: list[str]
    concession_reasons: dict[str, str]
    unresolved: list[str]
    source: str = "cc"
    concession_trace: ConcessionTrace | None = None
    stance_diffs: list[StanceDiff] = field(default_factory=list)


# ── 差分计算 ──


def compute_stance_diff(
    earlier: StanceDeclaration,
    later: StanceDeclaration,
) -> StanceDiff:
    """计算两轮立场声明之间的差分。

    changed = 同一 key 但 value 不同
    removed = earlier 有 later 没有（= 让步的直接证据）
    added = later 有 earlier 没有
    """
    earlier_keys = set(earlier.stances)
    later_keys = set(later.stances)

    changed: dict[str, tuple[str, str]] = {}
    for key in earlier_keys & later_keys:
        if earlier.stances[key] != later.stances[key]:
            changed[key] = (earlier.stances[key], later.stances[key])

    removed: dict[str, str] = {
        key: earlier.stances[key] for key in earlier_keys - later_keys
    }

    added: dict[str, str] = {
        key: later.stances[key] for key in later_keys - earlier_keys
    }

    return StanceDiff(
        round_from=earlier.round_number,
        round_to=later.round_number,
        changed=changed,
        added=added,
        removed=removed,
    )


def derive_concession_trace(
    stance_sequence: list[StanceDeclaration],
) -> ConcessionTrace:
    """从完整立场序列推导让步轨迹。

    编排者决断：
    - computed_concessions 来自差分（removed + changed 中放弃的立场）
    - self_reported_concessions 来自 StanceDeclaration 中 agent 自我报告的
    - divergence 是两者的差集（本身是信号——agent 说自己让步了但数据没变化，
      或数据变了但 agent 没报告）
    """
    if len(stance_sequence) < 2:
        all_self_reported: list[str] = []
        for sd in stance_sequence:
            all_self_reported.extend(sd.self_reported_concessions)
        return ConcessionTrace(
            computed_concessions=[],
            self_reported_concessions=all_self_reported,
            divergence=[],
        )

    computed_set: set[str] = set()
    for i in range(len(stance_sequence) - 1):
        diff = compute_stance_diff(stance_sequence[i], stance_sequence[i + 1])
        # removed keys = 让步（立场完全消失）
        computed_set.update(diff.removed)
        # changed keys = 让步（立场发生变化）
        computed_set.update(diff.changed)

    computed = sorted(computed_set)

    # 收集所有轮次的自我报告
    all_self_reported = []
    for sd in stance_sequence:
        all_self_reported.extend(sd.self_reported_concessions)
    self_reported_set = set(all_self_reported)

    # 计算 divergence
    divergence: list[str] = []
    # 自我报告但未计算出 -> agent 声称让步但数据未变化
    for item in sorted(self_reported_set - computed_set):
        divergence.append(
            f"self_reported_not_computed: {item}"
        )
    # 计算出但未自我报告 -> 数据变化但 agent 未报告
    for item in sorted(computed_set - self_reported_set):
        divergence.append(
            f"computed_not_self_reported: {item}"
        )

    return ConcessionTrace(
        computed_concessions=computed,
        self_reported_concessions=all_self_reported,
        divergence=divergence,
    )


# ── 提取函数（立场差分架构） ──


def extract_from_gemini_verify(
    stance_sequence: list[StanceDeclaration],
    trigger_block_id: str,
    negation_stands: bool,
    conclusion: str,
    unresolved: list[str] | None = None,
) -> InquiryCycleResult:
    """从 Gemini verify 质询的立场序列中推导共识仪式所需数据。

    §18: Gemini 从理论一致性、逻辑严格性、概念有效性质询。
    §19: 质询到共识为止。

    此场景中只有 Gemini 参与质询，codex_conceded 始终为空。

    两种收敛结果：
    - 否定成立: CC 的产出被 Gemini 否定。共识 = "此处有问题"。
      Gemini 坚持了否定，没有让步。
    - 否定不成立: Gemini 的否定被 CC 驳回。共识 = "原产出成立"。
      Gemini 放弃了否定立场（从立场差分推导）。

    Parameters
    ----------
    stance_sequence : list[StanceDeclaration]
        Gemini verify 各轮的结构化立场声明。
    trigger_block_id : str
        触发质询的原始 CC 产出区块 ID（§21: consensus.refs 指向此 ID）。
    negation_stands : bool
        agent 判定 Gemini 否定是否成立。
    conclusion : str
        质询结论。
    unresolved : list[str] | None
        未解决项。

    Returns
    -------
    InquiryCycleResult
    """
    trace = derive_concession_trace(stance_sequence)

    # 计算相邻轮次的 diffs
    diffs: list[StanceDiff] = []
    for i in range(len(stance_sequence) - 1):
        diffs.append(compute_stance_diff(stance_sequence[i], stance_sequence[i + 1]))

    if negation_stands:
        # Gemini 否定成立 → Gemini 坚持否定，没有让步
        gemini_conceded: list[str] = []
        reasons = {"consensus": "Gemini 概念层否定成立——CC 产出被否定"}
    else:
        # Gemini 否定不成立 → 让步来自差分
        gemini_conceded = trace.computed_concessions
        reasons = {"gemini": "Gemini 否定不成立——Gemini 放弃否定立场"}

    return InquiryCycleResult(
        scenario="gemini_verify",
        trigger_block_id=trigger_block_id,
        conclusion=conclusion,
        gemini_conceded=gemini_conceded,
        codex_conceded=[],  # 此场景无 Codex 参与（§63 当前阶段形态）
        concession_reasons=reasons,
        unresolved=unresolved if unresolved is not None else [],
        source="cc",
        concession_trace=trace,
        stance_diffs=diffs,
    )


def extract_from_plan_review(
    stance_sequence: list[StanceDeclaration],
    trigger_block_id: str,
    conclusion: str,
    unresolved: list[str] | None = None,
) -> InquiryCycleResult:
    """从 plan-review 多轮对审的立场序列中推导共识仪式所需数据。

    §18: Codex 从实现可行性、代码正确性、架构一致性质询。
    §19: 质询到共识为止。

    此场景中 Opus（CC 的 plan 模式）出方案，Codex 质询。
    gemini_conceded 始终为空（Gemini 不参与 plan-review）。
    codex_conceded 来自立场差分推导。

    Parameters
    ----------
    stance_sequence : list[StanceDeclaration]
        plan-review 各轮的结构化立场声明。
    trigger_block_id : str
        触发 plan-review 的原始 CC 产出区块 ID。
    conclusion : str
        质询结论。
    unresolved : list[str] | None
        未解决项。

    Returns
    -------
    InquiryCycleResult
    """
    trace = derive_concession_trace(stance_sequence)

    # 计算相邻轮次的 diffs
    diffs: list[StanceDiff] = []
    for i in range(len(stance_sequence) - 1):
        diffs.append(compute_stance_diff(stance_sequence[i], stance_sequence[i + 1]))

    codex_conceded = trace.computed_concessions

    reasons: dict[str, str] = {}
    if codex_conceded:
        round_count = len(stance_sequence)
        reasons["codex"] = f"Codex 在 {round_count} 轮对审中放弃部分质疑"

    return InquiryCycleResult(
        scenario="plan_review",
        trigger_block_id=trigger_block_id,
        conclusion=conclusion,
        gemini_conceded=[],  # plan-review 不涉及 Gemini（§63 当前阶段形态）
        codex_conceded=codex_conceded,
        concession_reasons=reasons,
        unresolved=unresolved if unresolved is not None else [],
        source="cc",
        concession_trace=trace,
        stance_diffs=diffs,
    )


# ── 仪式触发 ──


def trigger_ceremony(
    cycle_result: InquiryCycleResult,
    base: Path = DEFAULT_BASE,
) -> dict[str, dict]:
    """从质询循环结果触发共识仪式三区块写入。

    §21: 三区块原子性地同时写入。
    §63: 在质询循环结束时强制触发共识仪式。

    立场差分数据通过 InquiryCycleResult 的 concession_trace 和 stance_diffs
    字段传递，但最终调用 write_consensus_ceremony 时仍然传 list[str] 格式的
    conceded 字段（下游接口约束）。

    Parameters
    ----------
    cycle_result : InquiryCycleResult
        从 extract_from_* 函数产出的结构化质询数据。
    base : Path
        block-topology 目录。

    Returns
    -------
    dict[str, dict]
        {"consensus": ..., "residue": ..., "tension": ...}
    """
    return write_consensus_ceremony(
        trigger_block_id=cycle_result.trigger_block_id,
        conclusion=cycle_result.conclusion,
        gemini_conceded=cycle_result.gemini_conceded,
        codex_conceded=cycle_result.codex_conceded,
        concession_reasons=cycle_result.concession_reasons,
        unresolved=cycle_result.unresolved,
        source=cycle_result.source,
        base=base,
    )


# ── 保留的辅助函数（不涉及让步提取） ──


def detect_convergence_from_review_file(review_file: Path) -> bool:
    """检测 review-results 文件是否表明质询循环已收敛。

    §19: 质询到共识为止。
    收敛 = 质询循环结束（不论共识内容是什么）。

    收敛信号：
    - plan-review: "确认满意" / "共识达成" / "方案定稿" / "[plan-reviewed]"
    - gemini verify: YAML frontmatter 中 result: pass/fail/escalate
    """
    if not review_file.exists():
        return False

    text = review_file.read_text(encoding="utf-8")

    plan_review_converged = any(
        marker in text
        for marker in ("确认满意", "共识达成", "方案定稿", "[plan-reviewed]")
    )

    gemini_converged = bool(
        re.search(r"^result:\s*(pass|fail|escalate)", text, re.MULTILINE)
    )

    return plan_review_converged or gemini_converged


def find_trigger_block_for_review(
    review_file: Path,
    base: Path = DEFAULT_BASE,
) -> str | None:
    """从 review-results 文件中提取关联的 trigger block ID。

    §21: consensus.refs 指向"触发质询的原始 CC 产出区块 id"。
    本函数从 review-results 文件的 YAML frontmatter 中提取此 ID。
    """
    text = review_file.read_text(encoding="utf-8")

    fm_match = re.match(r"^---\s*\n(.*?)\n---", text, re.DOTALL)
    if not fm_match:
        return None

    fm_text = fm_match.group(1)

    # trigger 字段优先
    trigger_match = re.search(r"^trigger:\s*(.+)$", fm_text, re.MULTILINE)
    if trigger_match:
        trigger_val = trigger_match.group(1).strip().strip('"').strip("'")
        if len(trigger_val) == 64:
            return trigger_val

    # target 字段备选（gemini genealogy review 格式）
    target_match = re.search(r"^target:\s*(.+)$", fm_text, re.MULTILINE)
    if target_match:
        target_val = target_match.group(1).strip().strip('"').strip("'")
        if len(target_val) == 64:
            return target_val

    return None
