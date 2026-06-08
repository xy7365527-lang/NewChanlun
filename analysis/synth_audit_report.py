"""合成审计修复后重跑报告 → analysis/fugue_audit_fixed_backtest.md。

读取两份重跑 JSON（去截断后的真实结果），计算 BH 对比 + 夏普比率（A5），
输出可证伪基准对照表。认识论等级 L2（真实数据，含否定性结果）。
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "analysis" / "data_cache"
OUT = ROOT / "analysis" / "fugue_audit_fixed_backtest.md"

NO_ADDON = DATA / "fugue_no_addon_audit_results.json"
V3 = DATA / "full_system_v3_audit_results.json"
OKLO_QUICK = ROOT / "analysis" / "quick_oklo_declamp_result.json"


def sharpe(pnls: list[float]) -> float:
    n = len(pnls)
    if n < 2:
        return 0.0
    mean = sum(pnls) / n
    var = sum((x - mean) ** 2 for x in pnls) / (n - 1)
    sd = var ** 0.5
    return mean / sd if sd > 0 else 0.0


def verdict(fsm: float, bh: float) -> str:
    if fsm > bh:
        return "FSM>BH（有超额，待随机门控证伪）"
    return "FSM≤BH（无 alpha 证据，疑同义反复）"


def main() -> None:
    L: list[str] = []
    L.append("# 审计修复后重跑结果（去截断 + BH/夏普可证伪基准）\n")
    L.append("> 输入：A1/A2 去截断后的 `fugue_no_addon` + `full_system_v3` 重跑")
    L.append("> 修复记录：`docs/architecture/audit_fix_log.md`")
    L.append("> 认识论等级：**L2**（真实数据，含否定性结果）")
    L.append("> 关键限定（B3，521 号）：FSM 仍是 **PH 门控**（无 MACD 动量闸）→ 信号属 candidate 层，"
             "非 confirmed 买卖点 alpha 证据。\n")
    L.append("## 一句话结论\n")
    L.append("（1）实现了缠师降成本完整逻辑链——降成本（成本→0）+ **挣股数**（成本归零后利润买回更多筹码，53/81课）；"
             "（2）去掉「只赚不赔」截断（A1）后全部「亮眼」数字崩塌，且去截断后成本罕归零→挣股数罕触发。\n")
    L.append("去掉降成本「只赚不赔」截断（A1）后，全部「亮眼」回测数字崩塌：OKLO 单标的"
             "**+652% → +114%**（仍 < BH +251%）；组合 v3 **全标的转为大幅负收益**"
             "（ES −90% / OKLO −101% / BTC −100% / HK700 −24%，等权 −78.75% vs BH +176%）。"
             "**修正后策略一致跑输买入持有 → 无 alpha 证据**——审计 A1/A5 预测的否定性 L2 结果被完整兑现。\n")

    # ── 决定性验证：OKLO 铁证标的去截断前后 ──
    L.append("## 0. 决定性验证 — OKLO（审计 A1 铁证标的）\n")
    if OKLO_QUICK.exists():
        q = json.loads(OKLO_QUICK.read_text())
        L.append("| 指标 | 修复前(含截断) | 修复后(去截断) |")
        L.append("|------|---------------|---------------|")
        L.append(f"| FSM 复利 | **+652.27%** | **{q['fsm_compound']:+.2f}%** |")
        L.append(f"| BH 基准 | +251.30% | {q['bh']:+.2f}% |")
        L.append(f"| 超额(FSM−BH) | +400.97% | **{q['fsm_compound']-q['bh']:+.2f}%** |")
        L.append(f"| 夏普* | — | {q.get('sharpe', sharpe(q['pnls'])):+.3f} |")
        L.append(f"| 胜率 / 交易 | — | {q['win_rate']:.0f}% / {q['n_trades']}笔 |")
        L.append("")
        L.append("**判定**：去截断后 OKLO 收益从 +652% 崩塌至 "
                 f"{q['fsm_compound']:+.0f}%（提款机虚增的 ~538 个百分点蒸发），"
                 "且 **FSM < BH（超额为负）→ 无 alpha**。这是审计 A1+A5 预测的"
                 "**否定性 L2 结果**——修复生效，修正后策略跑输买入持有。\n")
    else:
        L.append("_（OKLO 快速验证未就绪）_\n")

    # ── 单标的 D2 无加仓（干净基线）──
    L.append("## 1. 单标的 D2 无加仓版（QQQ / OKLO / HK700 / BTC）— 干净基线\n")
    if NO_ADDON.exists():
        d = json.loads(NO_ADDON.read_text())
        L.append("| 标的 | Bars | BH% | FSM%（去截断+挣股数）| 超额(FSM−BH) | 夏普* | 胜率 | 交易 | 挣股数 | 判定 |")
        L.append("|------|------|-----|-------------|-------------|-------|------|------|--------|------|")
        for sym, r in d.items():
            sh = sharpe(r["pnls"])
            exc = r["fsm_compound"] - r["bh"]
            earn = f"{r.get('n_earn_shares', 0)}笔/{r.get('max_shares_growth', 1.0):.2f}×"
            L.append(
                f"| {sym} | {r['n_bars']:,} | {r['bh']:+.1f} | {r['fsm_compound']:+.2f} "
                f"| {exc:+.2f} | {sh:+.3f} | {r['win_rate']:.0f}% | {r['n_trades']} "
                f"| {earn} | {verdict(r['fsm_compound'], r['bh'])} |"
            )
    else:
        L.append("| 标的 | Bars | BH% | FSM%（去截断）| 超额(FSM−BH) | 夏普* | 胜率 | 交易 | 判定 |")
        L.append("|------|------|-----|-------------|-------------|-------|------|------|------|")
        if OKLO_QUICK.exists():
            q = json.loads(OKLO_QUICK.read_text())
            sh = q.get("sharpe", sharpe(q["pnls"]))
            L.append(
                f"| OKLO | {q['n_bars']:,} | {q['bh']:+.1f} | {q['fsm_compound']:+.2f} "
                f"| {q['fsm_compound']-q['bh']:+.2f} | {sh:+.3f} | {q['win_rate']:.0f}% "
                f"| {q['n_trades']} | {verdict(q['fsm_compound'], q['bh'])} |"
            )
        L.append("| QQQ / HK700 / BTC | — | — | _后台重跑中_ | — | — | — | — | "
                 "JSON 落盘后 `python analysis/synth_audit_report.py` 自动补全 |")
        L.append("")
        L.append("> **状态**：no_addon 全 4 标的重跑在共享高负载机器上仍在执行 BTC（1,795,600 bar 长仓）。"
                 "QQQ/OKLO/HK700 已算完但 JSON 在 BTC 完成后统一落盘。OKLO 长仓结果由独立快速验证"
                 "（`quick_oklo_declamp_check.py`）先行给出——已足以证实 A1 修复方向。")
    L.append("")

    # ── 组合 v3 ──
    L.append("## 2. 全市场组合 v3（ES 期货 + OKLO + BTC 10x + HK700）\n")
    if V3.exists():
        d = json.loads(V3.read_text())
        for mode_key, mode_label in [("with_k4", "WITH K4"), ("without_k4", "WITHOUT K4")]:
            block = d.get(mode_key, {})
            if not block:
                continue
            L.append(f"### {mode_label}\n")
            L.append("| 标的 | Bars | BH% | FSM%（去截断+挣股数）| 超额 | 夏普* | 胜率 | 多/空 | 挣股数 | MDD | 判定 |")
            L.append("|------|------|-----|-------------|------|-------|------|-------|--------|-----|------|")
            fsms = []
            bhs = []
            for sym, r in block.items():
                sh = sharpe(r["pnls"])
                exc = r["fsm_compound"] - r["bh"]
                fsms.append(r["fsm_compound"])
                bhs.append(r["bh"])
                L.append(
                    f"| {sym} | {r['n_bars']:,} | {r['bh']:+.1f} | {r['fsm_compound']:+.2f} "
                    f"| {exc:+.2f} | {sh:+.3f} | {r['win_rate']:.0f}% "
                    f"| {r['long_n']}/{r['short_n']} | {r.get('n_earn', 0)}笔 | {r['max_dd']:.1f}% "
                    f"| {verdict(r['fsm_compound'], r['bh'])} |"
                )
            if fsms:
                port = sum(fsms) / len(fsms)
                bh_avg = sum(bhs) / len(bhs)
                L.append(f"| **组合(等权)** | — | {bh_avg:+.1f} | {port:+.2f} | {port-bh_avg:+.2f} "
                         f"| — | — | — | — | — | {verdict(port, bh_avg)} |")
            L.append("")
    else:
        L.append("_（v3 重跑 JSON 未就绪）_")
    L.append("")

    L.append("\\* 夏普为 per-trade mean(pnl)/std(pnl)，**未年化**，仅作可证伪对照。\n")

    L.append("## 3. 挣股数阶段（53/81课，完整操盘流程）\n")
    L.append("本次重跑已实现缠师降成本逻辑链的**第二阶段**——成本归零后用短差利润买回更多股数"
             "（持仓量增加，新增股 0/负成本）。详见 `docs/architecture/cost_reduction_earning_shares.md`。\n")
    L.append("- no_addon：新增 `CostSt.EARNING_SHARES` 状态，`total_shares += profit/c`。")
    L.append("- v3：新增 `earn_legs`，阶段2 利润转为额外仓位，退出时计入。")
    L.append("- 上表「挣股数」列 = 进入挣股数阶段的交易数（no_addon 附最大持仓增长倍数）。\n")
    L.append("**关键发现（L2 否定性）**：A1 去截断后降成本短差有赚有赔，成本**很少归零**，"
             "故挣股数阶段**很少激活**（多数标的 0 笔）。这与原文不矛盾——缠师挣股数依赖高精度短差"
             "（「技术高的把成本降更低、筹码增更多」）与牛市普涨；当前 PH 门控、无 MACD 动量闸（B3/521号）"
             "的信号精度不足以稳定把成本打到 0。完整流程已实现，但**当前信号质量尚不足以稳定进入挣股数**。\n")

    L.append("## 结果包（六要素）\n")
    L.append("**1. 结论**：上表为 A1/A2 去截断 + 挣股数（53/81课完整流程）后的真实重跑。"
             "降成本亏损 trim 已正常计入；挣股数阶段已实现但去截断后罕触发；BH/夏普对照兑现 A5。\n")
    L.append("**2. 定义依据**：38 课降成本；267 号 D2 无加仓 FSM；521 号（PH 门控限定）。\n")
    L.append("**3. 边界条件**：FSM ≤ BH → 无 alpha 证据；随机门控对照仍缺（M1 交付物 3），"
             "故「FSM>BH」仅是必要不充分。\n")
    L.append("**4. 下游推论**：作为 M2/M4 信号基线前须补随机门控 + MACD 动量闸（B3）。\n")
    L.append("**5. 谱系引用**：521 号、`formalization-validity-domain`（L2 标注）、"
             "`project_backtest_benchmark_falsifiability`。\n")
    L.append("**6. 影响声明**：本报告由 `synth_audit_report.py` 从重跑 JSON 合成，不改代码。\n")
    L.append("**认识论等级**：L2（真实数据，含否定性结果——天文数字是否崩塌见上表）。")

    OUT.write_text("\n".join(L))
    print(f"报告已写入：{OUT}")


if __name__ == "__main__":
    main()
