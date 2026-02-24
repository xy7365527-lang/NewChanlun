"""共识仪式触发协议——质询循环收敛时提取让步轨迹并触发三区块写入。

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
from dataclasses import dataclass
from pathlib import Path
from typing import Literal

from scripts.block_topology import DEFAULT_BASE
from scripts.consensus_ceremony import write_consensus_ceremony


@dataclass(frozen=True, slots=True)
class InquiryCycleResult:
    """质询循环的结构化产出——从非结构化的质询过程中提取。

    字段语义遵循 §21 的定义：
    - trigger_block_id: 触发质询的原始 CC 产出区块 id（§21 consensus.refs）
    - conclusion: 最终达成的结论（§21 consensus.content.conclusion）
    - gemini_conceded: Gemini 在质询中放弃的立场（§21 residue 字段）
    - codex_conceded: Codex 在质询中放弃的立场（§21 residue 字段）
    - concession_reasons: 让步理由（§21 residue.content.reasons）
    - unresolved: 双方承认未解决但暂时搁置的部分（§21 tension.content.unresolved）
    """

    scenario: Literal["gemini_verify", "plan_review"]
    trigger_block_id: str
    conclusion: str
    gemini_conceded: list[str]
    codex_conceded: list[str]
    concession_reasons: dict[str, str]
    unresolved: list[str]
    source: str = "cc"


def extract_from_gemini_verify(
    verify_result_text: str,
    trigger_block_id: str,
    negation_stands: bool,
) -> InquiryCycleResult:
    """从 Gemini verify 质询结果中提取共识仪式所需数据。

    §18: Gemini 从理论一致性、逻辑严格性、概念有效性质询。
    §19: 质询到共识为止。

    此场景中只有 Gemini 参与质询，codex_conceded 始终为空。

    两种收敛结果：
    - 否定成立: CC 的产出被 Gemini 否定。共识 = "此处有问题"。
      Gemini 坚持了否定，没有让步。结论是否定结论。
    - 否定不成立: Gemini 的否定被 CC 驳回。共识 = "原产出成立"。
      Gemini 放弃了否定立场（gemini_conceded 记录这些立场）。

    Parameters
    ----------
    verify_result_text : str
        Gemini verify 的完整输出文本。
    trigger_block_id : str
        触发质询的原始 CC 产出区块 ID（§21: consensus.refs 指向此 ID）。
    negation_stands : bool
        agent 判定 Gemini 否定是否成立。

    Returns
    -------
    InquiryCycleResult
    """
    conclusion = _extract_section(verify_result_text, "结论", "结果")
    unresolved = _extract_list_items(verify_result_text, "未解决", "搁置", "悬置")

    if negation_stands:
        # Gemini 否定成立 → Gemini 坚持否定，没有让步
        # 共识 = "CC 产出的 X 被否定"
        gemini_conceded: list[str] = []
        reasons = {"consensus": "Gemini 概念层否定成立——CC 产出被否定"}
    else:
        # Gemini 否定不成立 → Gemini 放弃了否定立场
        # 共识 = "CC 产出成立"
        gemini_conceded = _extract_list_items(
            verify_result_text, "误判", "不成立", "驳回",
        )
        reasons = {"gemini": "Gemini 否定不成立——Gemini 放弃否定立场"}

    if not conclusion:
        conclusion = verify_result_text[:200].strip()

    return InquiryCycleResult(
        scenario="gemini_verify",
        trigger_block_id=trigger_block_id,
        conclusion=conclusion,
        gemini_conceded=gemini_conceded,
        codex_conceded=[],  # 此场景无 Codex 参与（§63 当前阶段形态）
        concession_reasons=reasons,
        unresolved=unresolved,
        source="cc",
    )


def extract_from_plan_review(
    review_results_dir: Path,
    trigger_block_id: str,
    plan_subject: str = "",
) -> InquiryCycleResult | None:
    """从 plan-review 多轮对审的持久化文件中提取共识仪式所需数据。

    §18: Codex 从实现可行性、代码正确性、架构一致性质询。
    §19: 质询到共识为止。
    plan-review SKILL.md: 共识 = Codex 对方案所有质疑都被回应且 Codex 明确确认满意。

    此场景中 Opus（CC 的 plan 模式）出方案，Codex 质询。
    gemini_conceded 始终为空（Gemini 不参与 plan-review）。
    codex_conceded 记录 Codex 在对审过程中放弃的质疑。

    Parameters
    ----------
    review_results_dir : Path
        review-results 目录路径。
    trigger_block_id : str
        触发 plan-review 的原始 CC 产出区块 ID。
    plan_subject : str
        方案的主题标识。

    Returns
    -------
    InquiryCycleResult | None
        提取成功返回结构化数据；如果没有找到 plan-review 文件则返回 None。
    """
    review_files = sorted(
        review_results_dir.glob("plan-review-*.md"),
        key=lambda p: p.name,
    )
    if not review_files:
        return None

    # 按轮次分组：plan-review-{timestamp}-round{N}.md
    rounds: dict[int, Path] = {}
    for f in review_files:
        match = re.search(r"round(\d+)", f.name)
        if match:
            rounds[int(match.group(1))] = f

    if not rounds:
        final_text = review_files[-1].read_text(encoding="utf-8")
        round_count = 1
    else:
        max_round = max(rounds.keys())
        final_text = rounds[max_round].read_text(encoding="utf-8")
        round_count = max_round

    # 从最终轮次提取数据
    conclusion = _extract_section(final_text, "最终方案", "结论", "共识")

    # Codex 在多轮对审中放弃的质疑（Codex 最初提出但后来被 Opus 回应后撤回的）
    codex_conceded = _extract_list_items(
        final_text, "Codex让步", "Codex撤回", "质疑解决",
    )

    # 搜集所有轮次中 Codex 提出但最终轮未再提出的质疑
    # 这些质疑被 Opus 回应后 Codex 放弃了——这就是 residue
    if not codex_conceded and len(rounds) > 1:
        codex_conceded = _extract_cross_round_concessions(rounds)

    unresolved = _extract_list_items(final_text, "未解决", "搁置", "遗留")

    if not conclusion:
        conclusion = f"plan-review {round_count} 轮对审完成"
        if plan_subject:
            conclusion = f"{plan_subject}: {conclusion}"

    reasons: dict[str, str] = {}
    if codex_conceded:
        reasons["codex"] = f"Codex 在 {round_count} 轮对审中放弃部分质疑"

    return InquiryCycleResult(
        scenario="plan_review",
        trigger_block_id=trigger_block_id,
        conclusion=conclusion,
        gemini_conceded=[],  # plan-review 不涉及 Gemini（§63 当前阶段形态）
        codex_conceded=codex_conceded,
        concession_reasons=reasons,
        unresolved=unresolved,
        source="cc",
    )


def _extract_cross_round_concessions(rounds: dict[int, Path]) -> list[str]:
    """跨轮次提取 Codex 让步：早期轮次提出但最终轮未再提出的质疑。

    §20: 共识 = 缝合，缝合必然生产剩余物。
    跨轮次消失的质疑就是被缝合排除的内容——residue 的物质来源。
    """
    if len(rounds) < 2:
        return []

    sorted_round_nums = sorted(rounds.keys())
    final_round = sorted_round_nums[-1]
    final_text = rounds[final_round].read_text(encoding="utf-8")
    final_issues = set(_extract_list_items(final_text, "质疑", "问题", "缺陷"))

    conceded: list[str] = []
    for rn in sorted_round_nums[:-1]:
        earlier_text = rounds[rn].read_text(encoding="utf-8")
        earlier_issues = _extract_list_items(earlier_text, "质疑", "问题", "缺陷")
        for issue in earlier_issues:
            if issue not in final_issues and issue not in conceded:
                conceded.append(issue)

    return conceded


def trigger_ceremony(
    cycle_result: InquiryCycleResult,
    base: Path = DEFAULT_BASE,
) -> dict[str, dict]:
    """从质询循环结果触发共识仪式三区块写入。

    §21: 三区块原子性地同时写入。
    §63: 在质询循环结束时强制触发共识仪式。

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


# ── 内部辅助函数 ──


def _extract_section(text: str, *keywords: str) -> str:
    """从文本中提取与关键词匹配的节标题下的第一段内容。"""
    for kw in keywords:
        pattern = rf"^#{{2,4}}\s+.*{re.escape(kw)}.*$"
        match = re.search(pattern, text, re.MULTILINE | re.IGNORECASE)
        if match:
            start = match.end()
            next_heading = re.search(r"^#{2,4}\s+", text[start:], re.MULTILINE)
            end = start + next_heading.start() if next_heading else len(text)
            section = text[start:end].strip()
            if section:
                return section
    return ""


def _extract_list_items(text: str, *keywords: str) -> list[str]:
    """从文本中提取包含关键词的列表项。

    策略1：找到包含关键词的节标题，提取其下所有列表项。
    策略2：扫描所有列表项，找包含关键词的。
    """
    items: list[str] = []

    for kw in keywords:
        pattern = rf"^#{{2,4}}\s+.*{re.escape(kw)}.*$"
        match = re.search(pattern, text, re.MULTILINE | re.IGNORECASE)
        if match:
            start = match.end()
            next_heading = re.search(r"^#{2,4}\s+", text[start:], re.MULTILINE)
            end = start + next_heading.start() if next_heading else len(text)
            section = text[start:end]
            for line in section.splitlines():
                stripped = line.strip()
                if stripped.startswith(("- ", "* ")):
                    items.append(stripped[2:].strip())
            if items:
                return items

    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith(("- ", "* ")):
            item_text = stripped[2:].strip()
            for kw in keywords:
                if kw.lower() in item_text.lower():
                    items.append(item_text)
                    break

    return items
