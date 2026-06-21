#!/usr/bin/env python3
"""BTC unn 流式回测（segment 黑洞修复 + MACD 开启）交易行为分析。

数据源：trading_system/data_cache/unn_stream_BTC.json
该 JSON 由 backtest_unn_stream.py 的 analyze() 写出，**只含聚合统计，无 per-trade trades 数组**
（trades 仅在内存 res['trades'] 用于计数，未落盘）。本脚本据此做可得维度的完整分解，
并对不可得维度（逐笔时序/逐笔最大亏损分布/强平时段）显式标注数据源限制。

输出：analysis/btc_trade_behavior_macd.md
"""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "trading_system" / "data_cache" / "unn_stream_BTC.json"
OUT = ROOT / "analysis" / "btc_trade_behavior_macd.md"

# ladder index → 级别名（与 nested_recursive_fugue_final_backtest.LADDER_NAMES 一致）
LADDER_NAMES = {0: "bar", 1: "bi", 2: "segment", 3: "move(L1)", 4: "recL2",
                5: "recL3", 6: "recL4", 7: "recL5", 8: "recL6", 9: "recL7", 10: "recL8"}


def fmt_arr(arr: list, names: dict = LADDER_NAMES) -> str:
    """把按 ladder index 的数组渲染为 '名称=值' 的非零项列表。"""
    parts = [f"{names.get(i, str(i))}={v}" for i, v in enumerate(arr) if v]
    return ", ".join(parts) if parts else "（全零）"


def main() -> None:
    d = json.loads(SRC.read_text())
    s = d["unn_stream"]
    nrf = s["nrf_counters"]

    n_bars = d["n_bars"]
    bh_pct = d["bh_pct"]
    bh_mdd = d["bh_mdd_pct"]
    strat_pct = s["strat_pct"]
    strat_mdd = s["mdd_pct"]
    n_trades = s["n_trades"]
    by_ladder = s["by_ladder"]

    # ── 方向分解 ──
    long_n = long_pnl = long_wins = 0
    short_n = short_pnl = short_wins = 0
    long_rows, short_rows = [], []
    for key, v in by_ladder.items():
        lvl, pol = key.rsplit("/", 1)
        row = (lvl, v["n"], v["pnl_cash"], v["wins"], v["avg_held_bars"])
        if pol == "long":
            long_n += v["n"]; long_pnl += v["pnl_cash"]; long_wins += v["wins"]
            long_rows.append(row)
        else:
            short_n += v["n"]; short_pnl += v["pnl_cash"]; short_wins += v["wins"]
            short_rows.append(row)

    # ── 暴露/入场延迟 ──
    phys_long_bars = nrf["phys_long_bars"]
    long_exposure = phys_long_bars / n_bars * 100
    depth_bars = nrf["depth_bars"]
    pre_struct_bars = depth_bars[0]  # bar 层（结构涌现前）= 无持仓窗口
    entry_delay_pct = pre_struct_bars / n_bars * 100

    # ── E spawn（子voice/降成本）──
    spawns = nrf["spawns"]
    total_spawns = sum(spawns)
    total_roots = sum(nrf["root_entries"])
    total_liq = sum(nrf["liquidations"])
    total_flips = sum(nrf["flips"])

    L: list[str] = []
    w = L.append

    w("# BTC unn 流式回测交易行为分析（segment 黑洞修复 + MACD 开启）\n")
    w(f"> 数据源：`{SRC.relative_to(ROOT)}`（{n_bars:,} bar，1min，"
      f"BTC-USD-PERP.HYPERLIQUID）  \n")
    w(f"> bit-exact vs 批量基线：**{d['bit_exact_vs_batch']['status']}**"
      f"（{d['bit_exact_vs_batch']['n_keys']} 键全等）  \n")
    w(f"> 认识论等级：**L2**（单标的真实数据假设检验，可证伪）\n")

    # ── 数据源限制声明（严格性：不伪造 per-trade）──
    w("## 0. 数据源限制声明（强制前置）\n")
    w("该 JSON 由 `analyze()` 写出，**只含聚合统计，不含 per-trade `trades` 数组**"
      "（trades 仅在内存 `res['trades']` 用于计数，未落盘）。因此：\n")
    w("| 维度 | 可得性 | 说明 |")
    w("|------|--------|------|")
    w("| 方向/级别/盈亏聚合 | ✅ 可得 | `by_ladder` 提供层×极性分解 |")
    w("| 强平次数/级别/方向 | ✅ 可得 | `liquidations` 按 ladder index |")
    w("| E spawn 计数/净现金 | ✅ 可得 | `spawns` / `short_net_cash` |")
    w("| 暴露率/入场延迟 | ✅ 可得 | `phys_long_bars` / `depth_bars` |")
    w("| **逐笔时序（哪些时段交易/强平时段）** | ❌ 不可得 | 无 trade 的 entry/exit bar |")
    w("| **逐笔最大盈利/亏损（精确单笔）** | ⚠️ 部分 | 仅能给层级聚合 + 唯一单笔的 long |")
    w("| 分年收益曲线 | ❌ 不可得 | `yearly: null`（运行时 years=None）|")
    w("\n下文对不可得维度显式标注 **[数据源不含]**，不以聚合数据冒充逐笔。\n")

    # ── 1. 总览 ──
    w("## 1. 总览\n")
    w("| 指标 | unn 策略 | Buy&Hold | 差 |")
    w("|------|---------|----------|----|")
    w(f"| 总收益 | **{strat_pct:+.1f}%** | {bh_pct:+.1f}% | "
      f"**{strat_pct - bh_pct:+.1f}pp** |")
    w(f"| 最大回撤 | **{strat_mdd:.1f}%** | {bh_mdd:.1f}% | "
      f"**{strat_mdd - bh_mdd:+.1f}pp（更优）** |")
    w(f"| 总交易数 | {n_trades} 笔 | — | — |")
    w(f"| 退出原因 | {s['exit_reasons']} | — | — |")
    w(f"| 多头暴露率 | {long_exposure:.1f}%（{phys_long_bars:,}/{n_bars:,} bar）| 100% | — |")
    w(f"| N1 max_children | {s['nrf_max_children']} | — | — |")
    w(f"\nP1（≥BH）：**{'PASS' if d['P1_ge_bh'] else 'FAIL'}**"
      f"（差 {strat_pct - bh_pct:+.1f}pp）。MDD 8/8 优于 BH（-63% vs -84%）。\n")

    # ── 2. 方向分解 ──
    w("## 2. 按方向分解\n")
    w("> ⚠️ **`by_ladder` 的 pnl_cash 是「层视图账」，跨层不可加**"
      "（代码注释：「同一物理持仓的多级别身份并存，视图间 P&L 不可加——物理 NAV "
      "唯一真值=equity 序列」）。下表的方向小计仅作**定性归因**，其和**不等于** "
      f"+{strat_pct:.1f}% 的物理 NAV。\n")
    w("| 方向 | 笔数 | 层视图 P&L | wins | losses | 胜率 |")
    w("|------|------|-----------|------|--------|------|")
    w(f"| 多头 | {long_n} | {long_pnl:+,.0f} | {long_wins} | "
      f"{long_n - long_wins} | {long_wins / long_n * 100:.1f}% |")
    w(f"| 空头 | {short_n} | {short_pnl:+,.0f} | {short_wins} | "
      f"{short_n - short_wins} | {short_wins / short_n * 100:.1f}% |")
    w("\n**核心定性结论**：\n")
    w(f"- 唯一的 **1 笔多头**（根仓，recL4 层视图）贡献 **{long_pnl:+,.0f}** 现金——"
      "策略全部正收益来源。\n")
    w(f"- **{short_n} 笔空头全部净亏 {short_pnl:+,.0f}**（胜率仅 "
      f"{short_wins / short_n * 100:.1f}%），其中 recL2 层 2 笔 0 胜。\n")
    w("- 空头 = E spawn 降成本子voice（见 §5）。在 BTC 史上最强单边牛市中做空 = "
      "**借 units 稀释主升浪 = 踏空税**（复现谱系 539 号「空头操作域 ⊂ 非上行 regime」）。\n")

    # ── 3. 级别分解 ──
    w("## 3. 按级别（ladder）分解\n")
    w("| 级别/方向 | 笔数 | 层视图 P&L | wins | avg 持仓 bar | 角色 |")
    w("|-----------|------|-----------|------|-------------|------|")
    role = {
        "recL4/long": "根多头（move 入场→涌现 recL4，全程持有）",
        "move(L1)/short": "E spawn 降成本短差主体",
        "recL2/short": "E spawn 高层短差（长持·重亏）",
    }
    for key, v in sorted(by_ladder.items(), key=lambda kv: -abs(kv[1]["pnl_cash"])):
        w(f"| {key} | {v['n']} | {v['pnl_cash']:+,.0f} | {v['wins']} | "
          f"{v['avg_held_bars']:,.0f} | {role.get(key, '')} |")
    w(f"\n- **交易最多级别**：move(L1)/short（{by_ladder['move(L1)/short']['n']} 笔 = "
      f"{by_ladder['move(L1)/short']['n'] / n_trades * 100:.1f}%）。\n")
    w(f"- **贡献利润最多**：recL4/long（{by_ladder['recL4/long']['pnl_cash']:+,.0f}，"
      "唯一正贡献层）。\n")
    w(f"- **贡献亏损最多**：recL2/short（{by_ladder['recL2/short']['pnl_cash']:+,.0f}，"
      f"仅 2 笔却亏最多，单笔均 {by_ladder['recL2/short']['pnl_cash'] / 2:+,.0f}，"
      f"avg 持仓 {by_ladder['recL2/short']['avg_held_bars']:,.0f} bar = 高层长持空头逆势失血）。\n")
    w(f"- **信号双重性印证**：根多头 `root_entries[move(L1)]=1` 入场，"
      f"却以 recL4/long 记账——同一物理仓从 move 层涌现到 recL4 层"
      f"（持有 {by_ladder['recL4/long']['avg_held_bars']:,.0f} bar ≈ 全历史）。\n")

    # ── 4. 强平分析 ──
    w("## 4. 强平分析\n")
    w(f"- **强平总次数：{total_liq}**（按 ladder：{fmt_arr(nrf['liquidations'])}）。\n")
    if total_liq == 0:
        w("- ✅ **segment 黑洞修复后强平归零**。对照谱系 `project_unn_btc_spawn_throwback`："
          "早期 BTC 曾有 -511K 单笔强平 + 333 子空头；本次 spawn 降至 202、强平降至 0。\n")
        w("- 强平时段 / 强平 voice 级别·方向：**[数据源不含 + 本次为 0，N/A]**。\n")
    else:
        w(f"- 强平时段 / voice 级别·方向：**[数据源不含逐笔时序]**。\n")
    w(f"- 根仓翻转（flips）：{total_flips}（{fmt_arr(nrf['flips'])}）——"
      "无根翻空，符合 T49 confirm 改动（根多头不翻转，避免强牛中翻空踏空）。\n")

    # ── 5. E spawn 分析 ──
    w("## 5. E spawn（子voice/降成本）分析\n")
    w(f"- **子voice 总数：{total_spawns}**（按 ladder：{fmt_arr(spawns)}）。\n")
    w(f"- 根入场（root_entries）：{total_roots}（{fmt_arr(nrf['root_entries'])}）"
      "——唯一 1 个根多头。\n")
    w(f"- **{total_spawns} 个子voice 全部是空头降成本短差**，与 §2 的 {short_n} 笔空头一致。\n")
    w("- 子voice 净贡献（层视图 short_net_cash）：")
    w(f"  {fmt_arr(nrf['short_net_cash'])} → 合计 **{sum(nrf['short_net_cash']):+,.0f}（负）**。\n")
    w("- nest 武装/触发（move(L1) 层）：")
    w(f"  arms={nrf['nest_arms'][3]:,}, fire_sell={nrf['nest_fire_sell'][3]:,}, "
      f"fire_buy={nrf['nest_fire_buy'][3]:,}, breaks={nrf['nest_breaks'][3]:,}。\n")
    w("- **结论：子voice 净贡献为负**。E spawn 在强牛中过度做空，是 +436.5% 跑不赢 BH 的"
      "**直接机制**——每个空头子voice 临时削减净多头暴露，在单边上行中即兑现踏空损失。\n")

    # ── 6. 时间分析 ──
    w("## 6. 时间分析\n")
    w("- **交易时段集中度**：**[数据源不含逐笔 entry/exit bar，不可得]**。\n")
    w(f"- **最大单笔盈利**：recL4/long 根仓 **{by_ladder['recL4/long']['pnl_cash']:+,.0f}**"
      "（唯一多头，可精确确定）。\n")
    w("- **最大单笔亏损**：**[数据源不含逐笔，无法给精确单笔]**。可得上界参考——"
      f"recL2/short 2 笔合计 {by_ladder['recL2/short']['pnl_cash']:+,.0f}，单笔均约 "
      f"{by_ladder['recL2/short']['pnl_cash'] / 2:+,.0f}。\n")
    w(f"- **入场延迟**：结构涌现前 bar 层无持仓窗口 = {pre_struct_bars:,} bar "
      f"= 全历史 {entry_delay_pct:.1f}%（≈ {pre_struct_bars / 1440:.0f} 个交易日@1min）。"
      "入场延迟对 BH 差距贡献**小**（仅 ~2% 时间）。\n")
    w("- 各深度驻留 bar：" + fmt_arr(depth_bars) + "。\n")

    # ── 7. 与 BH 差距归因 ──
    gap = strat_pct - bh_pct
    w("## 7. 与 Buy&Hold 差距归因\n")
    w(f"**差距：{strat_pct:+.1f}% vs {bh_pct:+.1f}% = {gap:+.1f}pp（跑输）。**\n")
    w("分解三个候选原因，按证据排序：\n")
    w("| 候选 | 证据 | 量级判断 |")
    w("|------|------|---------|")
    w(f"| ① 空头稀释（E spawn 踏空）| {short_n} 笔空头净亏，子voice 削减净多头暴露 | "
      "**主因** |")
    w(f"| ② 仓位 sizing（非满仓）| 多头暴露 {long_exposure:.1f}% 时间，但 spawn 期间"
      "净敞口<100% | **主因** |")
    w(f"| ③ 入场晚 | 仅 {entry_delay_pct:.1f}% 时间（{pre_struct_bars / 1440:.0f} 日）无仓 | 次要 |")
    w("| ④ 出场早 | 根多头全程持有（{0:,} bar≈全历史），未提前出场 | 否（非原因）|"
      .format(by_ladder["recL4/long"]["avg_held_bars"]))
    w("\n**结论**：差距**不是入场晚或出场早**（根多头几乎全程持有），而是\n")
    w("1. **空头子voice 在强牛中稀释主升浪**（①）——降成本逻辑在单边上行 regime 中"
      "结构性错误：每次「降成本」做空 = 卖飞 = 踏空；\n")
    w("2. **net 敞口 < 100%**（②）——spawn 短差期间净多头暴露被压低，无法吃满 BTC "
      "史上最强单边。\n")
    w("\nBTC 是全库最强单边牛市（谱系：「BTC 史上最强单边」「E 首次跑赢 BH」仅在 E 版纯多头时成立）。"
      "本 unn 配置引入降成本空头 → 必然跑输纯 BH。**这是 regime 函数，不是 bug**："
      "同一机制在震荡/下行标的（CL/BRN 等）反而优于 BH（谱系 539 号有效域读数）。\n")

    # ── 边界条件 / 下游推论 ──
    w("## 8. 结果包六要素\n")
    w("- **结论**：+436.5% 全部来自单根多头（recL4，move 入场涌现）；202 笔空头子voice "
      "净亏 = 跑输 BH 943.9pp 的直接机制；强平归零（segment 修复成效）。\n")
    w("- **定义依据**：`by_ladder` 层×极性分解 + `nrf_counters`（root_entries/spawns/"
      "liquidations/short_net_cash）；ladder index 映射 LADDER_NAMES；层视图账非可加（analyze 注释）。\n")
    w(f"- **边界条件**：结论在 BTC 单边上行 regime 成立；若标的 regime 转为震荡/下行"
      "（CL/BRN），空头子voice 净贡献可翻正、策略可超 BH（谱系 539）。若 `short_net_cash` "
      "转正或 `liquidations`>0 则需重判。\n")
    w("- **下游推论**：若要在 BTC 上逼近 BH，需在强单边 regime 抑制 E spawn 做空"
      "（或将降成本限定于已确认的次级别回调）；当前 spawn 333→202 已使收益 340→436.5，"
      "继续抑制 spawn 应进一步收窄差距。\n")
    w("- **谱系引用**：`project_unn_btc_spawn_throwback`（E spawn 踏空真凶）、"
      "539 号（根翻空/空头有效域⊂非上行）、信号双重性（segment/move 双层身份）。\n")
    w("- **影响声明**：本分析不改动任何代码/定义，仅读取 unn_stream_BTC.json 产出报告。\n")

    OUT.write_text("\n".join(L), encoding="utf-8")
    print(f"报告写入 {OUT}")
    print(f"\n=== 速览 ===")
    print(f"strat {strat_pct:+.1f}% vs BH {bh_pct:+.1f}% (gap {gap:+.1f}pp)")
    print(f"多头 {long_n}笔 层视图{long_pnl:+,.0f} | 空头 {short_n}笔 层视图{short_pnl:+,.0f} "
          f"(胜率{short_wins/short_n*100:.1f}%)")
    print(f"强平 {total_liq} | spawn {total_spawns} | root {total_roots} | flips {total_flips}")
    print(f"多头暴露 {long_exposure:.1f}% | 入场延迟 {entry_delay_pct:.1f}%")


if __name__ == "__main__":
    main()
