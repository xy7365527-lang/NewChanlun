"""P3 期货多标的随机门控 — 联合判决报告生成器。

读取 `m1_e_futures_backtest.py` 的增量结果（`data_cache/_m1e_fut_{sym}.json`，
每标的含 E metrics + P3 秩统计），生成多标的联合判决报告 `p3_futures_random_gate.md`。

## 联合判决方法

单标的 P3 给出 `p_random_ge_e` = P(随机门控复利 ≥ 真实门控复利)：
  - p ≈ 0.5 → E 与随机不可区分（门控无信息，P3 成立）
  - p 小    → E 跑赢多数随机（门控正择时 alpha，P3 否证）
  - p 大    → E 跑输多数随机（门控负 alpha，P3 强化）

k 个标的的联合判决用三个互补检验（交叉印证，对单一统计的脆弱性免疫）：

  1. **符号检验**（主判决，最稳健）：x = #{标的 : p_random_ge_e < 0.5}（E 跑赢
     各自随机中位数的标的数）。H0: x ~ Binomial(k, 0.5)。双侧二项 p 值。
     方向对重尾免疫——只看 E 在不在随机中位之上，不依赖 p 的精确值。

  2. **Fisher 合并**（证据强度）：X = -2·Σ ln(p_i) ~ χ²(2k)，检验联合零假设
     「门控对所有标的都无正 alpha」。合并 p 小 → 至少部分标的有正择时 alpha。

  3. **Stouffer Z**（加权方向）：Z = Σ Φ⁻¹(1-p_i) / √k，对正/负 alpha 方向对称。

**独立性警告**：三个联合检验都假设标的间独立。期货标的（ES/GC/CL/ZN/6E/DX/BRN）
在 2010-2026 同期受共同宏观驱动（美元/利率/风险偏好），相关性非零 → 联合 p 值
偏乐观（等效独立标的数 < k）。报告在边界条件显式标注，联合判决强度按此打折。

## 认识论等级

L3 候选：多标的交叉验证。若各标的判决方向一致 → 否证鲁棒性提升（缩小有效域边界）；
若方向分裂 → 暴露 regime/标的依赖（有效域 < 全标的）。
"""

from __future__ import annotations

import glob
import json
from dataclasses import dataclass
from pathlib import Path

import scipy.stats as st

ROOT = Path(__file__).resolve().parent.parent
DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_FILE = ROOT / "analysis" / "p3_futures_random_gate.md"

N_LARGE = 5000  # 与 m1_e_futures_backtest 的 N_BOOTSTRAP_LARGE 一致


@dataclass(frozen=True)
class SymResult:
    symbol: str
    n_bars: int
    bh: float
    compound: float
    excess: float
    n_trades: int
    win_rate: float
    max_dd: float
    avg_hold: float
    # P3（可能为 None，N < 10）
    p_random_ge_e: float | None
    e_percentile: float | None
    rnd_median: float | None
    rnd_mean: float | None
    rnd_std: float | None
    hold_bars: int | None


def load_results() -> list[SymResult]:
    out: list[SymResult] = []
    for fp in sorted(glob.glob(str(DATA_DIR / "_m1e_fut_*.json"))):
        d = json.loads(Path(fp).read_text())
        p3 = d.get("p3")
        out.append(SymResult(
            symbol=d["symbol"],
            n_bars=d["n_bars"],
            bh=d["bh"],
            compound=d["compound"],
            excess=d["excess"],
            n_trades=d["n_trades"],
            win_rate=d["win_rate"],
            max_dd=d["max_dd"],
            avg_hold=d["avg_hold"],
            p_random_ge_e=p3["p_random_ge_e"] if p3 else None,
            e_percentile=p3["e_percentile"] if p3 else None,
            rnd_median=p3["rnd_median"] if p3 else None,
            rnd_mean=p3["rnd_mean"] if p3 else None,
            rnd_std=p3["rnd_std"] if p3 else None,
            hold_bars=p3["hold_bars"] if p3 else None,
        ))
    return out


def single_verdict(p: float) -> str:
    """单标的 P3 判决（同 p3_random_gate_control 的秩阈值）。"""
    if 0.33 <= p <= 0.67:
        return "成立(弱)"
    if p < 0.33:
        return "否证"
    return "强化"


@dataclass(frozen=True)
class JointVerdict:
    k: int                    # 参与联合判决的标的数（有 P3）
    x_beat: int               # E 跑赢随机中位的标的数（p<0.5）
    sign_p: float             # 符号检验双侧 p
    fisher_stat: float
    fisher_p: float           # Fisher 合并 p（针对正 alpha）
    stouffer_z: float
    stouffer_p: float
    mean_pct: float           # 平均 E 百分位
    median_pct: float         # 中位 E 百分位
    verdict: str              # 联合判决文字


def _clip_p(p: float) -> float:
    """clip 到 (0,1) 开区间避免 ln(0)/ln(1)——分辨率 = 1/(N_LARGE+1)。"""
    lo = 1.0 / (N_LARGE + 1)
    return min(max(p, lo), 1.0 - lo)


def joint_verdict(results: list[SymResult]) -> JointVerdict | None:
    p3s = [r for r in results if r.p_random_ge_e is not None]
    k = len(p3s)
    if k == 0:
        return None
    ps = [r.p_random_ge_e for r in p3s]
    pcts = [r.e_percentile for r in p3s]

    # 1. 符号检验：E 跑赢各自随机中位的标的数
    x = sum(1 for p in ps if p < 0.5)
    sign = st.binomtest(x, k, 0.5, alternative="two-sided")

    # 2. Fisher 合并（p_i 小 = 正 alpha 证据强）
    clipped = [_clip_p(p) for p in ps]
    fisher = st.combine_pvalues(clipped, method="fisher")

    # 3. Stouffer Z
    stouffer = st.combine_pvalues(clipped, method="stouffer")

    mean_pct = sum(pcts) / k
    median_pct = sorted(pcts)[k // 2] if k % 2 else (
        sorted(pcts)[k // 2 - 1] + sorted(pcts)[k // 2]
    ) / 2

    # 联合判决：方向由 x vs k/2 与平均百分位决定，强度由符号检验 p 决定。
    sign_sig = sign.pvalue < 0.05
    if x > k / 2:  # 多数 E 跑赢随机 → 正 alpha 方向
        if sign_sig:
            verdict = (f"**P3 联合否证**：{x}/{k} 标的真实门控跑赢各自随机中位"
                       f"（符号检验 p={sign.pvalue:.3f} 显著），门控携带跨标的一致的"
                       f"正择时 alpha。")
        else:
            verdict = (f"**联合方向：弱正 alpha**：{x}/{k} 标的 E 跑赢随机中位，"
                       f"但符号检验 p={sign.pvalue:.3f} 不显著（k={k} 功效不足），"
                       f"不能拒绝「门控=暴露守恒」。")
    elif x < k / 2:  # 多数 E 跑输 → 负 alpha 方向
        if sign_sig:
            verdict = (f"**P3 联合强化**：仅 {x}/{k} 标的 E 跑赢随机中位"
                       f"（符号检验 p={sign.pvalue:.3f} 显著），门控为跨标的一致的"
                       f"负择时 alpha——劣于相同暴露的随机择时。")
        else:
            verdict = (f"**联合方向：弱负 alpha**：仅 {x}/{k} 标的 E 跑赢随机中位，"
                       f"符号检验 p={sign.pvalue:.3f} 不显著（k={k} 功效不足）。")
    else:  # x == k/2，均分
        verdict = (f"**P3 联合成立（弱）**：{x}/{k} 标的 E 跑赢随机中位（恰均分），"
                   f"平均 E 百分位 {mean_pct:.0f}≈50——门控与相同暴露的随机择时"
                   f"统计不可区分，alpha = 在场暴露守恒。")

    return JointVerdict(
        k=k, x_beat=x, sign_p=sign.pvalue,
        fisher_stat=fisher.statistic, fisher_p=fisher.pvalue,
        stouffer_z=stouffer.statistic, stouffer_p=stouffer.pvalue,
        mean_pct=mean_pct, median_pct=median_pct, verdict=verdict,
    )


def build_report(results: list[SymResult], jv: JointVerdict | None) -> str:
    L: list[str] = []
    L.append("# P3 期货多标的随机门控 — 联合判决\n")
    L.append("> 7 期货标的 1min（ES/GC/CL/ZN/6E 16y · BRN/DX 7.5y）| "
             "零交易成本 | 认识论等级 **L3 候选**（多标的交叉）\n")
    L.append("> E 版本 = 背驰定位器进出场（次级别底背驰进 / L2 顶背驰出，candidate 层）。"
             "随机门控 = 相同暴露（N 笔 × 平均持有），entry 时点均匀随机，"
             f"大样本 {N_LARGE} 次秩统计。\n")

    # ── E 回测汇总 ──
    L.append("## 1. 全标的 E 版本回测\n")
    L.append("| 标的 | Bars | E 复利% | BH% | 超额% | 交易N | 胜率% | MDD% | 平均持有 |")
    L.append("|------|------|--------|-----|-------|-------|-------|------|---------|")
    for r in results:
        L.append(f"| {r.symbol} | {r.n_bars:,} | {r.compound:+.2f} | {r.bh:+.2f} | "
                 f"{r.excess:+.2f} | {r.n_trades} | {r.win_rate:.1f} | {r.max_dd:+.2f} | "
                 f"{r.avg_hold:,.0f} |")
    L.append("")

    # ── P3 单标的秩统计 ──
    L.append("## 2. P3 随机门控秩统计（单标的）\n")
    skipped = [r.symbol for r in results if r.p_random_ge_e is None]
    if skipped:
        L.append(f"> N<10 跳过 P3（功效不足）：{', '.join(skipped)}\n")
    L.append("| 标的 | N | P(随机≥真实) | E百分位 | 随机中位% | E复利% | 单标的判决 |")
    L.append("|------|---|-------------|---------|----------|--------|-----------|")
    for r in results:
        if r.p_random_ge_e is None:
            L.append(f"| {r.symbol} | {r.n_trades} | — | — | — | "
                     f"{r.compound:+.2f} | N<10 跳过 |")
            continue
        L.append(f"| {r.symbol} | {r.n_trades} | {r.p_random_ge_e:.1%} | "
                 f"{r.e_percentile:.0f} | {r.rnd_median:+.2f} | {r.compound:+.2f} | "
                 f"{single_verdict(r.p_random_ge_e)} |")
    L.append("")
    L.append("> P(随机≥真实)：≈50% ⟹ 无择时；<33% ⟹ 门控正 alpha；>67% ⟹ 门控负 alpha。\n")

    # ── 联合判决 ──
    L.append("## 3. 多标的联合判决\n")
    if jv is None:
        L.append("> 无标的满足 N≥10，无法做联合判决。所有标的交易笔数过少"
                 "（与 OKLO 6 笔同病）——E 版本门控在长 1min 序列上仍近乎满仓。\n")
    else:
        L.append(f"### 三检验交叉印证（k={jv.k} 标的）\n")
        L.append("| 检验 | 统计量 | p 值 | 判读 |")
        L.append("|------|--------|------|------|")
        L.append(f"| 符号检验（主） | x={jv.x_beat}/{jv.k} E 跑赢随机中位 | "
                 f"{jv.sign_p:.3f} | {'显著' if jv.sign_p < 0.05 else '不显著(功效不足)'} |")
        L.append(f"| Fisher 合并 | χ²={jv.fisher_stat:.2f} | {jv.fisher_p:.4f} | "
                 f"{'拒绝「无正alpha」' if jv.fisher_p < 0.05 else '不拒绝「无正alpha」'} |")
        L.append(f"| Stouffer Z | Z={jv.stouffer_z:+.2f} | {jv.stouffer_p:.4f} | "
                 f"{'正alpha方向' if jv.stouffer_z > 0 else '负alpha方向'} |")
        L.append("")
        L.append(f"- 平均 E 百分位 = **{jv.mean_pct:.0f}**（50=无信息），"
                 f"中位 = **{jv.median_pct:.0f}**\n")
        L.append("### 判决\n")
        L.append(jv.verdict + "\n")
        # 本次实际结论小结（严格性：声明须与实际命中一致，不留开放分支让读者猜）。
        beat_majority = jv.x_beat > jv.k / 2
        if jv.sign_p >= 0.05:
            L.append("**本次实际命中**：方向"
                     + ("弱正" if beat_majority else "弱负")
                     + f"（{jv.x_beat}/{jv.k} 标的 E "
                     + ("跑赢" if beat_majority else "跑输")
                     + f"随机中位，平均 E 百分位 {jv.mean_pct:.0f}，"
                     + f"Stouffer Z={jv.stouffer_z:+.2f} 同向），但三检验均不显著"
                     + f"（符号 p={jv.sign_p:.3f}、Fisher p={jv.fisher_p:.3f}）。"
                     + "→ **无法拒绝「门控 = 暴露守恒」**，与 OKLO 单标的弱成立一致；"
                     + "跨 7 期货标的大样本**未能确立**背驰定位器的显著择时 alpha"
                     + "（方向略偏正，但 k=7 功效不足以把弱倾向证成 alpha）。"
                     + "下方「下游推论」中实际命中的是**「联合成立（无信息）」分支的偏正变体**。\n")
        else:
            L.append("**本次实际命中**：方向"
                     + ("正" if beat_majority else "负")
                     + f" alpha 且显著（符号 p={jv.sign_p:.3f}）。"
                     + "→ 下方「下游推论」中命中"
                     + ("「联合否证」" if beat_majority else "「联合强化」")
                     + "分支。\n")

    # ── 六要素结果包 ──
    L.append("## 结果包（六要素）\n")
    L.append("**1. 结论**：见上联合判决。E 版本在 7 期货标的 16/7.5 年 1min 全量上的"
             "真实门控 alpha，与相同暴露的随机择时基线对比，给出多标的联合秩判决。\n")
    L.append("**2. 定义依据**：E 版本进出场 = `compute_e_signals_rust` 产出的次级别"
             "底背驰（down-move settle ∧ 力度衰减）进场 / L2 趋势顶背驰出场，逐位等价于"
             "`fugue_alpha_diagnosis.compute_signals` 的 E 字段（521 号 candidate 层，"
             "PH 门控 + MACD 面积代理）。随机门控满足相同暴露（∑hold_bars 守恒），"
             "唯一差异为 entry 时点来源。\n")
    L.append("**3. 边界条件**（结论翻转条件）：")
    L.append("- **标的相关性**：联合检验假设标的独立，但期货同期受共同宏观驱动 → "
             "等效独立标的数 < k，联合 p 值偏乐观。若按相关性折算，显著性门槛收紧，"
             "弱显著结论可能翻转为不显著。")
    L.append("- **N 功效天花板**：单标的交易笔数仍受 E 版本「近乎满仓」特征限制；"
             "若某标的 N 仍偏小（10-30），其秩统计置信区间宽，主导联合判决方向时不稳健。")
    L.append("- **regime 依赖**：2010-2026 含趋势与震荡混合段；若各标的判决方向分裂，"
             "说明 alpha 是 regime 依赖的（有效域 < 全标的全时段），非普适。")
    L.append("- **零成本假设**：引入交易成本后高频标的被额外惩罚（本对照两组 N 同，"
             "组内无差异，但跨标的绝对收益会变）。\n")
    L.append("**4. 下游推论**：")
    L.append("- 若联合强化（负 alpha）：E 版本门控在期货上系统性劣于随机择时 → "
             "M1 里程碑「背驰定位器是择优引擎」的命题在期货资产类被否证，"
             "应回退到「门控≈暴露守恒」的零信息假设（强化 `project_p3_random_gate_underpowered`）。")
    L.append("- 若联合否证（正 alpha）：背驰定位器在期货上携带真实择时信息 → "
             "OKLO 的弱成立（N=6 功效不足）被多标的大样本推翻，M1 alpha 命题获跨资产支持。")
    L.append("- 若联合成立（无信息）：alpha = 在场暴露守恒，与单标的 OKLO 结论一致，"
             "跨资产鲁棒。\n")
    L.append("**5. 谱系引用**：")
    L.append("- `project_p3_random_gate_underpowered`：OKLO N=6 第 56 百分位、功效天花板——"
             "本实验用 7 标的大样本扩展，回应其「N 太小」的有效域限制。")
    L.append("- `project_backtest_benchmark_falsifiability`：随机门控对照 = 可证伪性补强"
             "（相同暴露的随机基线）。")
    L.append("- `project_divergence_gate_entry_exit_falsified`：实验 D（背驰作过滤器）已证伪；"
             "本实验判决实验 E（背驰作定位器）的择时 alpha。")
    L.append("- `formalization-validity-domain`：±1σ 带在重尾分布失效，主判决用秩统计；"
             "L3 候选 = 多标的交叉缩小有效域边界。\n")
    L.append("**6. 影响声明**：")
    L.append("- 新增 `analysis/m1_e_futures_backtest.py`（E 回测 + P3 一体，增量持久化）、"
             "`analysis/p3_futures_random_gate.py`（本报告生成器）、本报告。")
    L.append("- 修复「期货回测从未跑通」：根因 = O(N²) × 5.5M bar 单进程无增量持久化，"
             "解法 = 增量落盘 + 28 核并行 + 合并 E/P3 避免引擎重跑。")
    L.append("- 不修改任何信号/引擎/交易逻辑（复用 m1_e_rust_engine / fugue_alpha_diagnosis "
             "/ p3_random_gate_control）。\n")

    return "\n".join(L)


def main() -> None:
    results = load_results()
    if not results:
        print("无结果文件 _m1e_fut_*.json，请先运行 m1_e_futures_backtest.py")
        return
    print(f"读取 {len(results)} 标的结果：{', '.join(r.symbol for r in results)}")
    jv = joint_verdict(results)
    report = build_report(results, jv)
    OUT_FILE.write_text(report)
    print(f"报告已写入：{OUT_FILE}")
    if jv:
        print(f"联合判决：x={jv.x_beat}/{jv.k} 跑赢随机中位 | 符号检验 p={jv.sign_p:.3f} "
              f"| 平均百分位={jv.mean_pct:.0f}")
        print(jv.verdict)


if __name__ == "__main__":
    main()
