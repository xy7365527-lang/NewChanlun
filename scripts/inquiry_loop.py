"""多轮质询调度器——audit_needed → API 桥接 + 收敛判断 + 三区块写入。

将 gangju_analysis.py 输出的 audit_needed: true 信号转化为实际的
Gemini + Codex 多轮质询循环，通过同主体差分 + Key 空间冻结检测收敛，
收敛后输出 trajectories（互斥破裂轨迹）并触发共识仪式三区块写入。

收敛算法来自十五轮 Gemini 严格质询（188号谱系）：
- 同主体 diff：比较 diff(主体X_{t-2}, 主体X_t) 避免交替震荡
- Key 冻结：连续 N 轮无新 Key 注册
- N = 参与主体数 + 1 = 3
- 理论地位：当前架构下的最优解，边界开放（非理论极限）
- 收敛后输出 trajectories（互斥破裂轨迹），非 residue 列表

四个不可消去的 residue（R15 Gemini 确认完整）：
  a) 语义碎片化（多主体命名不一致）
  b) (stance, null) 判定（沉默的歧义性）
  c) 精确数学 vs LLM 二律背反
  d) 收敛算法对自身假死的结构性盲区（元层面）

系统公理：盲目的破裂轨迹优于静止。

谱系引用：
- 183号目A：多轮质询管道
- 187号：架构分离伪诊断
- 188号：多轮质询管道收敛算法——十五轮 Gemini 质询结算
- 189号：元观察——十五轮质询的方法论洞察
- 161号：务实否定
"""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Literal

from scripts.block_topology import DEFAULT_BASE
from scripts.consensus_trigger import (
    InquiryCycleResult,
    StanceDeclaration,
    compute_stance_diff,
    derive_concession_trace,
    trigger_ceremony,
)
from scripts.stance_parser import parse_stance_declaration


# ── 数据结构 ──


@dataclass(frozen=True, slots=True)
class RoundSnapshot:
    """单轮质询快照。"""

    round_number: int
    speaker: Literal["gemini", "codex"]
    response_text: str
    stance: StanceDeclaration | None
    registered_keys: frozenset[str]  # 该轮时的已注册 Key 集合
    new_keys: frozenset[str]  # 该轮新注册的 Key


@dataclass(frozen=True, slots=True)
class ConvergenceVerdict:
    """收敛判断。"""

    converged: bool
    key_frozen: bool  # Key 空间连续 N 轮无新增
    same_subject_stable: bool  # 同主体 diff 连续 N 轮为空
    reason: str


@dataclass(frozen=True, slots=True)
class Trajectory:
    """单条破裂轨迹——沿某个 lack 破裂 + 预期 tension 方向。

    trajectories 而非 residue 列表是 R14 的核心发现：
    Distinguish 的本体论从认识论（判断对错）转向实践（选择破裂方向）。
    列表预设了选出正确项的认识论框架，trajectories 预设的是选择系统
    在哪个方向上承受破裂。前者有对错之分，后者只有后果之分。
    """

    lack_key: str  # 沿哪个 lack 破裂
    lack_description: str  # lack 的描述
    expected_tension: str  # 预期 tension 方向


@dataclass(frozen=True, slots=True)
class InquiryResult:
    """质询循环的完整产出。"""

    converged: bool
    verdict: ConvergenceVerdict
    snapshots: tuple[RoundSnapshot, ...]
    trajectories: tuple[Trajectory, ...]  # 互斥破裂轨迹
    total_rounds: int
    registered_keys: frozenset[str]
    cycle_result: InquiryCycleResult | None  # 收敛后的仪式输入


# ── 收敛检测 ──


def _extract_keys_from_stance(stance: StanceDeclaration | None) -> frozenset[str]:
    """从 StanceDeclaration 中提取 stance keys。"""
    if stance is None:
        return frozenset()
    return frozenset(stance.stances.keys())


def check_key_frozen(
    snapshots: list[RoundSnapshot],
    window: int,
) -> bool:
    """检查 Key 空间是否连续 window 轮无新增。

    Parameters
    ----------
    snapshots : list[RoundSnapshot]
        已有快照序列。
    window : int
        稳定窗口大小。

    Returns
    -------
    bool
        如果最近 window 轮都没有新 Key 注册，返回 True。
    """
    if len(snapshots) < window:
        return False
    recent = snapshots[-window:]
    return all(len(s.new_keys) == 0 for s in recent)


def check_same_subject_stable(
    snapshots: list[RoundSnapshot],
    speaker: str,
    window: int,
) -> bool:
    """检查同一主体在最近 window 次发言中 stance 是否稳定。

    比较 stance_{t-2} 和 stance_t（同一主体，间隔一轮）。
    避免 Gemini=reject, Codex=accept 的交替震荡导致 diff 永不为空。

    Parameters
    ----------
    snapshots : list[RoundSnapshot]
        已有快照序列。
    speaker : str
        要检查的主体（"gemini" 或 "codex"）。
    window : int
        需要连续稳定的轮数。

    Returns
    -------
    bool
        同主体 stance 连续 window 次无变化则返回 True。
    """
    speaker_snapshots = [s for s in snapshots if s.speaker == speaker]
    if len(speaker_snapshots) < window:
        return False

    recent = speaker_snapshots[-window:]
    for i in range(1, len(recent)):
        prev_stance = recent[i - 1].stance
        curr_stance = recent[i].stance
        # 两者都 None = 稳定（都解析失败）
        if prev_stance is None and curr_stance is None:
            continue
        # 一个 None 一个不是 = 不稳定
        if prev_stance is None or curr_stance is None:
            return False
        diff = compute_stance_diff(prev_stance, curr_stance)
        if diff.changed or diff.added or diff.removed:
            return False
    return True


def check_convergence(
    snapshots: list[RoundSnapshot],
    stability_window: int = 3,
) -> ConvergenceVerdict:
    """检查收敛条件。

    收敛 = Key 空间冻结 AND 同主体 stance 稳定

    Parameters
    ----------
    snapshots : list[RoundSnapshot]
        完整快照序列。
    stability_window : int
        稳定窗口大小，默认 3（参与主体数 + 1 = 3）。

    Returns
    -------
    ConvergenceVerdict
    """
    key_frozen = check_key_frozen(snapshots, stability_window)
    gemini_stable = check_same_subject_stable(
        snapshots, "gemini", stability_window,
    )
    codex_stable = check_same_subject_stable(
        snapshots, "codex", stability_window,
    )
    same_subject_stable = gemini_stable and codex_stable

    converged = key_frozen and same_subject_stable

    if converged:
        reason = (
            f"收敛：Key 空间冻结 + 同主体 diff 连续 {stability_window} 轮为空"
        )
    else:
        parts = []
        if not key_frozen:
            parts.append("Key 空间未冻结")
        if not gemini_stable:
            parts.append("Gemini stance 未稳定")
        if not codex_stable:
            parts.append("Codex stance 未稳定")
        reason = "未收敛：" + "、".join(parts)

    return ConvergenceVerdict(
        converged=converged,
        key_frozen=key_frozen,
        same_subject_stable=same_subject_stable,
        reason=reason,
    )


# ── Trajectories 构建 ──


def build_trajectories(
    snapshots: list[RoundSnapshot],
    key_registry: frozenset[str],
) -> tuple[Trajectory, ...]:
    """从快照序列构建互斥破裂轨迹。

    trajectories 而非 residue 列表（R14）：
    - unresolved keys = 收敛时双方 stance 不同的 Key
    - 每个 unresolved key 生成一条 trajectory
    - trajectory 包含：沿哪个 lack 破裂 + 预期 tension 方向

    Parameters
    ----------
    snapshots : list[RoundSnapshot]
        完整快照序列。
    key_registry : frozenset[str]
        完整注册 Key 集合。

    Returns
    -------
    tuple[Trajectory, ...]
    """
    # 收集每个 speaker 在每个 key 上的最终 stance
    final_stances: dict[str, dict[str, str]] = {}  # key -> {speaker: stance}
    for snap in reversed(snapshots):
        if snap.stance is None:
            continue
        for key, stance_val in snap.stance.stances.items():
            if key not in final_stances:
                final_stances[key] = {}
            if snap.speaker not in final_stances[key]:
                final_stances[key][snap.speaker] = stance_val

    trajectories: list[Trajectory] = []
    for key in sorted(key_registry):
        stances = final_stances.get(key, {})
        gemini_stance = stances.get("gemini", "")
        codex_stance = stances.get("codex", "")

        # unresolved = 双方最终 stance 不同
        if gemini_stance and codex_stance and gemini_stance != codex_stance:
            trajectories.append(Trajectory(
                lack_key=key,
                lack_description=(
                    f"Gemini: {gemini_stance}, Codex: {codex_stance}"
                ),
                expected_tension=(
                    f"沿 {key} 破裂 → Gemini 方向({gemini_stance}) "
                    f"vs Codex 方向({codex_stance})"
                ),
            ))

        # 单方 null = 沉默的歧义性（residue b）
        elif bool(gemini_stance) != bool(codex_stance):
            present_speaker = "Gemini" if gemini_stance else "Codex"
            present_val = gemini_stance or codex_stance
            trajectories.append(Trajectory(
                lack_key=key,
                lack_description=(
                    f"单方发言：{present_speaker} = {present_val}, "
                    f"另一方沉默"
                ),
                expected_tension=(
                    f"沿 {key} 破裂 → 沉默的歧义性"
                    f"（(stance, null) 判定——residue b）"
                ),
            ))

    return tuple(trajectories)


# ── 循环结果组装 ──


def build_cycle_result(
    snapshots: list[RoundSnapshot],
    trigger_block_id: str,
    key_registry: frozenset[str],
    trajectories: tuple[Trajectory, ...],
) -> InquiryCycleResult:
    """从快照序列组装质询循环结果。

    residue 来源：
    - gemini_conceded / codex_conceded → 从 derive_concession_trace() 推导
    - unresolved → trajectories 中的 lack_key 列表

    Parameters
    ----------
    snapshots : list[RoundSnapshot]
        完整快照序列。
    trigger_block_id : str
        触发质询的原始区块 ID。
    key_registry : frozenset[str]
        完整注册 Key 集合。
    trajectories : tuple[Trajectory, ...]
        互斥破裂轨迹。

    Returns
    -------
    InquiryCycleResult
    """
    # 分离 gemini 和 codex 的 stance 序列
    gemini_stances = [
        s.stance for s in snapshots
        if s.speaker == "gemini" and s.stance is not None
    ]
    codex_stances = [
        s.stance for s in snapshots
        if s.speaker == "codex" and s.stance is not None
    ]

    gemini_trace = derive_concession_trace(gemini_stances)
    codex_trace = derive_concession_trace(codex_stances)

    # 组装 unresolved
    unresolved = [t.lack_key for t in trajectories]

    # 组装 conclusion（含 trajectories 描述）
    if trajectories:
        traj_lines = []
        for i, t in enumerate(trajectories):
            traj_lines.append(
                f"  轨迹{chr(65 + i)}: 沿 {t.lack_key} 破裂 "
                f"→ {t.expected_tension}"
            )
        conclusion = (
            f"质询循环收敛（{len(snapshots)} 轮）。"
            f"产出 {len(trajectories)} 条互斥破裂轨迹：\n"
            + "\n".join(traj_lines)
            + "\n→ 选择一条轨迹。系统沿该方向继续。"
        )
    else:
        conclusion = f"质询循环收敛（{len(snapshots)} 轮），无 unresolved Key。"

    # 组装 concession_reasons
    concession_reasons: dict[str, str] = {}
    if gemini_trace.computed_concessions:
        concession_reasons["gemini"] = (
            f"Gemini 在质询中放弃 {len(gemini_trace.computed_concessions)} 个立场"
        )
    if codex_trace.computed_concessions:
        concession_reasons["codex"] = (
            f"Codex 在质询中放弃 {len(codex_trace.computed_concessions)} 个立场"
        )

    return InquiryCycleResult(
        scenario="gemini_verify",
        trigger_block_id=trigger_block_id,
        conclusion=conclusion,
        gemini_conceded=gemini_trace.computed_concessions,
        codex_conceded=codex_trace.computed_concessions,
        concession_reasons=concession_reasons,
        unresolved=unresolved,
        source="cc",
        concession_trace=gemini_trace,
        stance_diffs=[],
    )


# ── 主循环 ──


def _build_round_context(
    subject: str,
    context: str,
    snapshots: list[RoundSnapshot],
) -> str:
    """构建下一轮的 context，包含之前各轮的回复摘要。"""
    if not snapshots:
        return context

    parts = [context, "\n\n=== 质询历史 ===\n"]
    for snap in snapshots:
        verdict_str = ""
        if snap.stance is not None:
            verdict_str = f" [verdict: {snap.stance.verdict}]"
        # 截断过长的回复
        text_preview = snap.response_text[:500]
        if len(snap.response_text) > 500:
            text_preview += "..."
        parts.append(
            f"\nR{snap.round_number} ({snap.speaker}){verdict_str}:\n"
            f"{text_preview}\n"
        )
    return "".join(parts)


def run_inquiry_loop(
    subject: str,
    context: str,
    trigger_block_id: str,
    base: Path = DEFAULT_BASE,
    stability_window: int = 3,
    gemini_challenger: object | None = None,
    codex_challenger: object | None = None,
) -> InquiryResult:
    """执行多轮质询循环。

    每轮：
    1. Gemini challenge → 解析 stance → 检测新 Key → 更新 Key registry
    2. Codex review → 解析 stance → 检测新 Key → 更新 Key registry
    3. 收敛检查：Key 冻结 AND 同主体 diff 为空 → 收敛
    4. 否则 → 组合双方回复为下一轮 context → 继续
    5. 收敛后 → 组装 trajectories → 组装 InquiryCycleResult → trigger_ceremony()

    Parameters
    ----------
    subject : str
        质询主题。
    context : str
        质询背景。
    trigger_block_id : str
        触发质询的原始区块 ID。
    base : Path
        block-topology 目录。
    stability_window : int
        稳定窗口大小，默认 3。
    gemini_challenger : object | None
        Gemini challenger 实例。None 时自动创建。
    codex_challenger : object | None
        Codex challenger 实例。None 时自动创建。

    Returns
    -------
    InquiryResult
    """
    if gemini_challenger is None:
        from newchan.gemini.modes import GeminiChallenger
        gemini_challenger = GeminiChallenger()
    if codex_challenger is None:
        from newchan.codex.modes import CodexChallenger
        codex_challenger = CodexChallenger()

    snapshots: list[RoundSnapshot] = []
    key_registry: set[str] = set()
    round_number = 0

    while True:
        round_number += 1
        round_context = _build_round_context(subject, context, snapshots)

        # ── Gemini challenge ──
        # STANCE_OUTPUT_PROTOCOL 已在 challenge template 中追加，
        # 不再在 subject 中重复（193号诊断）
        gemini_result = gemini_challenger.challenge(  # type: ignore[union-attr]
            subject, round_context,
        )
        gemini_stance = parse_stance_declaration(
            gemini_result.response, round_number=round_number,
        )
        gemini_keys = _extract_keys_from_stance(gemini_stance)
        new_gemini_keys = gemini_keys - key_registry
        key_registry.update(new_gemini_keys)

        gemini_snap = RoundSnapshot(
            round_number=round_number,
            speaker="gemini",
            response_text=gemini_result.response,
            stance=gemini_stance,
            registered_keys=frozenset(key_registry),
            new_keys=frozenset(new_gemini_keys),
        )
        snapshots.append(gemini_snap)

        # ── Codex review ──
        # STANCE_OUTPUT_PROTOCOL 已在 review template 中追加，
        # 不再在 subject 中重复（193号诊断：双重追加 + 长 context 导致忽略）
        codex_context = round_context + (
            f"\n\n=== Gemini R{round_number} 回复 ===\n"
            f"{gemini_result.response}\n"
        )
        codex_result = codex_challenger.review(  # type: ignore[union-attr]
            subject, codex_context,
        )
        codex_stance = parse_stance_declaration(
            codex_result.response, round_number=round_number,
        )
        codex_keys = _extract_keys_from_stance(codex_stance)
        new_codex_keys = codex_keys - key_registry
        key_registry.update(new_codex_keys)

        codex_snap = RoundSnapshot(
            round_number=round_number,
            speaker="codex",
            response_text=codex_result.response,
            stance=codex_stance,
            registered_keys=frozenset(key_registry),
            new_keys=frozenset(new_codex_keys),
        )
        snapshots.append(codex_snap)

        # ── 收敛检查 ──
        verdict = check_convergence(snapshots, stability_window)
        if verdict.converged:
            frozen_keys = frozenset(key_registry)
            trajectories = build_trajectories(snapshots, frozen_keys)
            cycle_result = build_cycle_result(
                snapshots, trigger_block_id, frozen_keys, trajectories,
            )
            # 触发共识仪式三区块写入
            trigger_ceremony(cycle_result, base=base)

            return InquiryResult(
                converged=True,
                verdict=verdict,
                snapshots=tuple(snapshots),
                trajectories=trajectories,
                total_rounds=round_number,
                registered_keys=frozen_keys,
                cycle_result=cycle_result,
            )


# ── CLI 入口 ──


def main() -> None:
    """CLI 入口——从命令行参数启动多轮质询循环。"""
    parser = argparse.ArgumentParser(
        description="多轮质询调度器——audit_needed → API 桥接 + 收敛判断",
    )
    parser.add_argument("--subject", required=True, help="质询主题")
    parser.add_argument("--context", default="", help="质询背景")
    parser.add_argument(
        "--trigger", required=True, help="触发质询的原始区块 ID",
    )
    parser.add_argument(
        "--base", default=str(DEFAULT_BASE), help="block-topology 目录",
    )
    parser.add_argument(
        "--stability-window", type=int, default=3,
        help="稳定窗口大小（默认 3 = 参与主体数 + 1）",
    )
    args = parser.parse_args()

    result = run_inquiry_loop(
        subject=args.subject,
        context=args.context,
        trigger_block_id=args.trigger,
        base=Path(args.base),
        stability_window=args.stability_window,
    )

    print(f"收敛: {result.converged}")
    print(f"总轮数: {result.total_rounds}")
    print(f"注册 Key 数: {len(result.registered_keys)}")
    print(f"破裂轨迹数: {len(result.trajectories)}")

    if result.trajectories:
        print("\n=== 互斥破裂轨迹 ===")
        print("（系统公理：盲目的破裂轨迹优于静止）\n")
        for i, traj in enumerate(result.trajectories):
            label = chr(65 + i)
            print(f"  轨迹{label}: 沿 {traj.lack_key} 破裂")
            print(f"    描述: {traj.lack_description}")
            print(f"    预期 tension: {traj.expected_tension}")
            print()
        print("→ 选择一条轨迹。系统沿该方向继续。")

    print(f"\n收敛理由: {result.verdict.reason}")


if __name__ == "__main__":
    main()
