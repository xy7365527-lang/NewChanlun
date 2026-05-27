"""模块4: 三闸门统一流程 (521 合规) — 级别(PH) + 创新低结构(PH) + 动量(MACD).

521号裁定: "三闸门统一PH"的前提(PH 统一替代 MACD 做动量)被 §14 定理级证伪
→ 降级为 **双闸门 PH(级别 + 创新低结构) + MACD 动量闸**. 动量闸必须 MACD,
不可由 PH 冒充(否则声明膨胀, 090号).

本脚本端到端演示该 521 架构, 并检验 §5 标注为"未验证"的边界假设:
    PH(创新低必要条件) ∧ MACD(力度衰竭充分线索) 联合 是否优于单独 MACD?

三闸门:
  闸1 级别(PH, σ-free): 在线因果 merge tree → detect_levels, 从 persistence 自动
       涌现大级别结构(无预设阈值/周期). 作上下文(确认大级别下跌存在).
  闸2 创新低/结构(PH): sublevel-H0 段间"背驰"(totH0(C)<totH0(A)) = 幅度必要条件
       (§5/239: H0≈幅度∈ker(D), 只能算创新低, 不能算力度衰竭).
  闸3 动量(MACD): macd_area 段间背驰 = 力度衰竭(第24课, 521强制 momentum=MACD).
统一真假买点判据: 闸3 ∧ 闸2 (联合). 对照 单独MACD / 单独PH / 联合 / 基础率.

认识论等级: L2(腾讯700单标的日线+30分, 小样本欠功效). 否定性/欠功效结果同样有效
(231号), 把噪声读成"联合有效"= 合成确认偏差(formalization-validity-domain 禁止).
承重结论是 §14/521 的 L0 定理, 不是本脚本的小样本数字.
"""

import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

from tencent_truefalse_bsp_backtest import build_rows, load_closes, precision  # noqa: E402

from newchan.a_level_detection import detect_levels  # noqa: E402
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_persistence_barcode import atr, atr_noise_threshold  # noqa: E402

import pandas as pd  # noqa: E402


def level_gate_context(name: str, ofile: str, bars_per_day: float) -> None:
    """闸1: σ-free 在线 merge tree → 自动涌现级别(无自由参数). 作架构上下文打印."""
    highs, lows, closes = load_closes(ofile)
    tree = OnlineMergeTree()
    for c in closes:
        tree.update(c)
    bc = tree.finalize()
    tau = atr_noise_threshold(highs, lows, closes, period=14)
    ls = detect_levels(bc.all_bars, bars_per_day=bars_per_day, noise_floor=tau)
    leaves = [lv.period_label for lv in ls.leaves]
    print(f"  [{name}] 闸1 级别(PH σ-free): 涌现 {len(ls.levels)} 级, "
          f"叶级别={leaves}, τ={tau:.2f}, settled特征={len(bc.all_bars)}")


def evaluate_gate(R: pd.DataFrame, flags: np.ndarray, truth: np.ndarray,
                  base: float, label: str) -> None:
    prec, n = precision(flags, truth)
    lift = prec - base if not np.isnan(prec) else float("nan")
    se = (prec * (1 - prec) / n) ** 0.5 if n > 0 and not np.isnan(prec) else float("inf")
    sig = "显著" if (not np.isnan(lift) and lift > 2 * se) else "✗不显著(噪声内)"
    recall = float((flags.astype(bool) & truth.astype(bool)).sum()) / max(1, int(truth.sum()))
    print(f"    {label:<26}{n:>6}{prec:>9.3f}{lift:>+10.3f}{recall:>9.2f}{sig:>16}")


def main() -> None:
    print("#" * 72)
    print("# 模块4: 三闸门统一流程 (521) — 级别(PH) + 创新低(PH) + 动量(MACD)")
    print("# 检验 §5 未验证假设: PH创新低 ∧ MACD力度 联合 是否优于单独 MACD")
    print("#" * 72)

    print("\n闸1 级别(PH, 上下文):")
    level_gate_context("日线", "daily_ohlcv.json", bars_per_day=1.0)
    level_gate_context("30分", "m30_ohlcv.json", bars_per_day=11.0)

    rows = build_rows("日线", "daily_ohlcv.json") + build_rows("30分", "m30_ohlcv.json")
    R = pd.DataFrame(rows)
    if R.empty:
        print("\n候选对不足。")
        return
    truth = R["truth"].to_numpy()
    base = float(truth.mean())
    macd = R["macd_div"].to_numpy(bool)        # 闸3 动量(MACD)
    ph = R["sub_div"].to_numpy(bool)           # 闸2 创新低(PH sublevel-H0)
    combined = macd & ph                        # 521 联合判据

    print(f"\n候选买点对 n={len(R)}  基础真反转率 P(真)={base:.3f}"
          f"（{int(truth.sum())}/{len(R)}）")
    print(f"\n  闸门评估  {'判据':<26}{'判真数':>6}{'精度':>9}{'lift':>10}{'召回':>9}{'显著?':>16}")
    evaluate_gate(R, macd, truth, base, "单独MACD(闸3 动量)")
    evaluate_gate(R, ph, truth, base, "单独PH(闸2 创新低)")
    evaluate_gate(R, combined, truth, base, "联合 MACD∧PH (521判据)")

    print("\n" + "=" * 72)
    print("诚实判读 (formalization-validity-domain):")
    print("- 联合判据若 lift 不显著 → 小样本欠功效, 不能声称'联合优于单独MACD'(§5假设仍未验证).")
    print("- 联合提升精度但降召回是预期(交集更严); 真正裁决需多标的大样本 L3.")
    print("- 承重结论 = §14/521 L0 定理(纯拓扑动量不可能), 非本脚本小样本数字.")
    print("- 闸1级别(PH)与闸2创新低(PH)是 PH 的合法独立贡献; 闸3动量必须 MACD(521).")
    print("=" * 72)


if __name__ == "__main__":
    main()
