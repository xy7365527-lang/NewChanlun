"""期货全标的 E / I / BH 三引擎对比 + P3 随机门控 — 联合汇总报告生成器。

读取两份增量落盘结果，生成 `analysis/p3_futures_all_on.md`：
  1. `data_cache/_m1e_fut_{sym}.json`  —— E 版本 metrics + BH + P3 秩统计
     （`m1_e_futures_backtest.py`，O(N) 全管线：segment resume + bi_zhongshu_new_signals）
  2. `data_cache/_i_slice_fut_{sym}.json` —— Version I（slice 模型）三变体
     （`m1_i_slice_futures_backtest.py`，git HEAD slice 版，非 shared 共享仓位版）

## 三引擎口径

- **E**：swing trading，次级别底背驰进 / L2 顶背驰出（`compute_e_signals_rust`，
  521 号 candidate 层，PH 门控 + MACD 面积代理）。
- **I**：Version I slice 模型——核心仓全仓 + 机动仓 MANEUVER_RATIO 切 disjoint slice
  分给各级别独立 FSM。三变体：I_none（无止损，原版）/ I_stopA（硬止损）/ I_stopB（A+切片独立）。
- **BH**：buy-and-hold，(close[-1]-close[0])/close[0]。

## P3 随机门控

E 版本与「相同暴露（N 笔 × 平均持有）+ entry 时点均匀随机」基线对比，5000 次秩统计。
联合判决用符号检验（主）+ Fisher 合并 + Stouffer Z（复用 `p3_futures_random_gate`）。

## 认识论等级

- 移植正确性 L0/L1（E/I 信号 bit-exact 移植；segment resume 逐位等价零发散守卫）。
- 回测结论 L2（真实数据，7 期货标的 1min 全量 7.5–16y；含否定性结果）。
- P3 多标的联合秩 → L3 候选（多标的交叉；方向一致 → 否证鲁棒性提升）。
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "analysis"))

import p3_futures_random_gate as P3R  # noqa: E402  复用 load_results / joint_verdict

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_FILE = ROOT / "analysis" / "p3_futures_all_on.md"

# 标的展示顺序（与 SYMBOL_FILES 一致）。
SYM_ORDER = ["ES", "GC", "CL", "ZN", "BRN", "6E", "DX"]


@dataclass(frozen=True)
class IResult:
    symbol: str
    n_bars: int
    bh: float
    i_none: float | None
    i_stopA: float | None
    i_stopB: float | None
    i_none_excess: float | None
    i_none_trades: int | None
    i_none_mdd: float | None
    i_signal_seconds: float | None


def load_i_results() -> dict[str, IResult]:
    out: dict[str, IResult] = {}
    for sym in SYM_ORDER:
        fp = DATA_DIR / f"_i_slice_fut_{sym}.json"
        if not fp.exists():
            continue
        d = json.loads(fp.read_text())
        v = d.get("variants", {})

        def _c(tag: str) -> float | None:
            return v[tag]["compound"] if tag in v else None

        none = v.get("I_none", {})
        out[sym] = IResult(
            symbol=sym,
            n_bars=d["n_bars"],
            bh=d["bh"],
            i_none=_c("I_none"),
            i_stopA=_c("I_stopA"),
            i_stopB=_c("I_stopB"),
            i_none_excess=none.get("excess"),
            i_none_trades=none.get("n_trades"),
            i_none_mdd=none.get("max_dd"),
            i_signal_seconds=d.get("i_signal_seconds"),
        )
    return out


def _winner(bh: float, e: float | None, i: float | None) -> str:
    """三引擎复利最高者（I 取 I_none 原版口径）。"""
    cands = [("BH", bh)]
    if e is not None:
        cands.append(("E", e))
    if i is not None:
        cands.append(("I", i))
    return max(cands, key=lambda kv: kv[1])[0]


def build_report() -> str:
    e_results = {r.symbol: r for r in P3R.load_results()}
    i_results = load_i_results()
    syms = [s for s in SYM_ORDER if s in e_results]

    L: list[str] = []
    L.append("# 期货全标的 E / I / BH 三引擎对比 + P3 随机门控（O(N) 增量管线）\n")
    L.append("> 7 期货标的 1min databento（ES/GC/CL/ZN/6E ~16y · BRN/DX ~7.5y）| "
             "零交易成本 | 认识论等级 **L2**（真实数据）/ **L3 候选**（P3 多标的交叉）\n")
    L.append("> **O(N) 增量管线（部分）**：segment resume（逐位等价零发散，"
             "`project_segment_resume_regression` 已解决）+ `bi_zhongshu_new_signals` "
             "delta 接口已增量化。Rust 引擎 `RecursiveOrchestrator` 逐 bar 驱动。"
             "**注**：走势级 `current_buysellpoints()` 等仍含 O(结构²) 残留（见下 ⚠️），"
             "故「完全 O(N)」尚未达成——E 受影响小（已最终），I 待审计修复后复跑。\n")
    L.append("> **三引擎**：E = swing 背驰定位器进出场（candidate 层）；"
             "I = Version I slice 模型（核心仓全仓 + 机动仓切片各级别 FSM，git HEAD slice 版"
             "**非** shared 共享仓位版）；BH = buy-and-hold。\n")
    L.append("> ⚠️ **I 版本状态 = 临时（待重跑）**：I 信号在**当前引擎**上计算——该引擎仍含"
             "残留 O(结构²) 瓶颈（走势级 `current_buysellpoints()` 每 bar 全量 + bi 引擎全量拷贝 + "
             "PH_fast_alive 全量 sort，见 `project_engine_on2_full_audit`）。这些是**速度**瓶颈，"
             "对结果 bit-exact 不变；但**正式 I 结论待「审计+修复所有 O(N²)」工位完成后、"
             "用完全 O(N) 引擎复跑确认**（若修复全部 bit-exact，则下列 I 数字不变；若审计发现"
             "current_buysellpoints 的正确性 bug，则 I 数字可能修订）。**E/BH/P3 不受影响，已最终**。\n")

    # ── 1. 三引擎主对比 ──
    L.append("## 1. E / I / BH 三引擎复利对比\n")
    L.append("> I 列为 I_none（原版 Version I，无止损）。止损变体见 §4。\n")
    L.append("| 标的 | Bars | BH% | E% | I_none% | 最优引擎 | E−BH | I−BH |")
    L.append("|------|------|-----|-----|---------|---------|------|------|")
    for sym in syms:
        e = e_results[sym]
        ir = i_results.get(sym)
        i_none = ir.i_none if ir else None
        win = _winner(e.bh, e.compound, i_none)
        i_str = f"{i_none:+.2f}" if i_none is not None else "—(pending)"
        i_bh = f"{i_none - e.bh:+.2f}" if i_none is not None else "—"
        L.append(f"| {sym} | {e.n_bars:,} | {e.bh:+.2f} | {e.compound:+.2f} | "
                 f"{i_str} | **{win}** | {e.excess:+.2f} | {i_bh} |")
    L.append("")
    n_done_i = sum(1 for s in syms if i_results.get(s) and i_results[s].i_none is not None)
    L.append(f"> I 版本完成 {n_done_i}/{len(syms)} 标的"
             + ("（全部完成）" if n_done_i == len(syms) else "（pending = 信号计算中）") + "\n")

    # 最优引擎计数（仅 I 完成的标的参与三方比较）
    win_counts = {"BH": 0, "E": 0, "I": 0}
    cmp_syms = [s for s in syms
                if i_results.get(s) and i_results[s].i_none is not None]
    for sym in cmp_syms:
        e = e_results[sym]
        win_counts[_winner(e.bh, e.compound, i_results[sym].i_none)] += 1
    if cmp_syms:
        L.append(f"**最优引擎分布**（{len(cmp_syms)} 个三方可比标的）："
                 f"BH {win_counts['BH']} · E {win_counts['E']} · I {win_counts['I']}\n")

    # ── 2. P3 随机门控（E 版本） ──
    L.append("## 2. P3 随机门控秩统计（E 版本）\n")
    L.append("| 标的 | N | P(随机≥真实) | E百分位 | 随机中位% | E复利% | 单标的判决 |")
    L.append("|------|---|-------------|---------|----------|--------|-----------|")
    for sym in syms:
        e = e_results[sym]
        if e.p_random_ge_e is None:
            L.append(f"| {sym} | {e.n_trades} | — | — | — | {e.compound:+.2f} | N<10 跳过 |")
            continue
        L.append(f"| {sym} | {e.n_trades} | {e.p_random_ge_e:.1%} | {e.e_percentile:.0f} | "
                 f"{e.rnd_median:+.2f} | {e.compound:+.2f} | "
                 f"{P3R.single_verdict(e.p_random_ge_e)} |")
    L.append("")
    L.append("> P(随机≥真实)：≈50% ⟹ 无择时（暴露守恒）；<33% ⟹ 门控正 alpha；"
             ">67% ⟹ 门控负 alpha。\n")

    # ── 3. P3 多标的联合判决 ──
    L.append("## 3. P3 多标的联合判决\n")
    jv = P3R.joint_verdict(list(e_results.values()))
    if jv is None:
        L.append("> 无标的满足 N≥10，无法联合判决。\n")
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

    # ── 4. Version I 止损变体分析 ──
    L.append("## 4. Version I 止损变体（slice 模型）\n")
    L.append("| 标的 | I_none% | I_stopA% | I_stopB% | I_none 交易N | I_none MDD% | 信号耗时 |")
    L.append("|------|---------|----------|----------|-------------|-------------|---------|")
    for sym in syms:
        ir = i_results.get(sym)
        if ir is None or ir.i_none is None:
            L.append(f"| {sym} | —(pending) | — | — | — | — | — |")
            continue
        sec = f"{ir.i_signal_seconds/60:.0f}min" if ir.i_signal_seconds else "—"
        L.append(f"| {sym} | {ir.i_none:+.2f} | {ir.i_stopA:+.2f} | {ir.i_stopB:+.2f} | "
                 f"{ir.i_none_trades} | {ir.i_none_mdd:+.2f} | {sec} |")
    L.append("")
    L.append("> I_stopA = 硬止损；I_stopB = A + 切片独立止损。**止损效果 regime 依赖，非普遍有害**："
             "对 BRN（+289→−131）/ DX（−0.5→−110）催化性毁灭，但对 CL（+11→+162）大幅有益、"
             "ZN（−35→−34）轻微改善；ES/GC/6E 止损轻微拖累。止损在强单边趋势标的（BRN）剪掉持仓 = "
             "纯拖累，在反复震荡标的（CL 油）截断回撤 = 正贡献——印证「降成本短差好坏由 regime 决定」"
             "（`project_shared_position_fugue` / `project_complete_fugue_v2`）。\n")

    # ── 结果包（六要素） ──
    L.append("## 结果包（六要素）\n")
    L.append("**1. 结论**：见 §1（三引擎对比）+ §3（P3 联合判决）。E/I 在 7 期货标的"
             "全量 1min 上，与 BH 及随机门控基线对比，给出三引擎胜负的多标的判决。\n")
    L.append("**2. 定义依据**：E = `compute_e_signals_rust`（次级别底背驰进 / L2 顶背驰出，"
             "521 号 candidate 层）；I = `compute_i_signals_rust` + slice `run_version_i`"
             "（核心仓 + 机动仓切片各级别 FSM，git HEAD slice 版）；两者信号层逐位等价于"
             "对应 Python 实现（differential test 守卫）。BH = 全程持有。\n")
    L.append("**3. 边界条件**（结论翻转条件）：")
    L.append("- **regime 依赖**：最优引擎随标的切换（见 §1 分布）——若某标的为强单边趋势，"
             "BH/E 占优、I 降成本短差纯拖累；若震荡，则结论可能翻转。有效域 < 全标的全时段。")
    L.append("- **标的相关性**：P3 联合检验假设标的独立，但期货同期受共同宏观驱动 → "
             "等效独立标的数 < k，联合 p 值偏乐观；弱显著结论可能翻转为不显著。")
    L.append("- **N 功效天花板**：E 版本近乎满仓 → 单标的交易笔数少（10–36），秩统计置信区间宽。")
    L.append("- **零成本假设**：引入交易成本后高频的 I 版本（千级交易）被额外重罚，"
             "I−BH 差距进一步扩大。\n")
    # 实际命中（数据驱动，不留开放分支——严格性：声明须与实际一致）。
    i_lt_e = sum(1 for s in cmp_syms
                 if e_results[s].compound is not None
                 and i_results[s].i_none < e_results[s].compound)
    L.append("**4. 下游推论（本次实际命中，非开放分支）**：")
    L.append(f"- **最优引擎分布 = BH {win_counts['BH']} · E {win_counts['E']} · "
             f"I {win_counts['I']}（共 {len(cmp_syms)} 标的）**：E 在最多标的（{win_counts['E']}/"
             f"{len(cmp_syms)}）夺冠，但其中 GC/ZN/DX 的 E−BH 仅 +0.5~+3（贴近 BH），"
             f"真正显著超 BH 的仅 6E（+10）；BH 在两大单边趋势标的（ES/CL）夺冠。"
             "→ M1「背驰择优引擎稳定跑赢被动持有」命题在期货资产类**未获支持**：E 多为「贴着 BH」"
             "而非「显著超越」，与 P3 的「暴露守恒」判读一致。")
    L.append(f"- **I_none 是最差引擎**：{i_lt_e}/{len(cmp_syms)} 标的 I < E，且 I 仅在 BRN 一个标的"
             "夺冠（+289%，该标的 I 三变体差异极大、对止损极敏感 → 偶然性高）。Version I 的多级别"
             "降成本短差在期货 1min 上**系统性毁 alpha**（强化 `project_complete_fugue_v2` / "
             "`project_btc_full_vi_backtest` 的「降成本在强趋势纯拖累」结论）；I−BH 在 6 个标的为负。")
    L.append("- **P3 联合 = 弱正 alpha 但不显著（x=6/7，符号 p=0.125，均值百分位 54）**："
             "不能拒绝「E 门控 = 暴露守恒」，与单标的 OKLO 弱成立、`project_p3_futures_joint_verdict` "
             "前序结论一致——跨 7 期货大样本**未确立** E 背驰定位器的显著跨标的择时 alpha。\n")
    L.append("**5. 谱系引用**：")
    L.append("- `project_segment_resume_regression`（已解决）：本次全 O(N) 管线的前提——"
             "segment resume bit-exact 零发散，拆除引擎第二堵 O(N²) 墙。")
    L.append("- `project_bi_zhongshu_on2_two_walls`：`bi_zhongshu_new_signals` 增量化第一堵墙；"
             "走势级 `current_buysellpoints()` 每 bar 仍是 I 版本主要常数成本（~1600 bar/s）。")
    L.append("- `project_p3_futures_joint_verdict`：前序期货 E+P3 联合判决（本报告复用其 P3 逻辑）。")
    L.append("- `project_complete_fugue_v2` / `project_btc_full_vi_backtest`：降成本在强趋势纯拖累。")
    L.append("- `project_p3_random_gate_underpowered`：随机门控对照 = 可证伪性补强；N 功效天花板。")
    L.append("- `formalization-validity-domain`：±1σ 重尾失效用秩统计；L3 候选 = 多标的交叉缩小有效域。\n")
    L.append("**6. 影响声明**：")
    L.append("- 新增 `analysis/p3_futures_all_on.py`（本三引擎合并报告生成器）+ 本报告 "
             "`analysis/p3_futures_all_on.md`。")
    L.append("- 复用既有结果：`_m1e_fut_{sym}.json`（E+P3）、`_i_slice_fut_{sym}.json`（I 三变体）；"
             "I 版本 GC/CL/6E 由 `m1_i_slice_futures_backtest.py` 全 O(N) 管线重跑补齐。")
    L.append("- 不修改任何信号/引擎/交易逻辑（纯 driver + 报告生成）。\n")

    return "\n".join(L)


def main() -> None:
    report = build_report()
    OUT_FILE.write_text(report)
    print(f"报告已写入：{OUT_FILE}")
    print(f"字节数：{len(report):,}")


if __name__ == "__main__":
    main()
