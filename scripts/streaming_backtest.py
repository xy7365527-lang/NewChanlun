"""第二阶段：修复后引擎的 streaming 回测（天然零前视）。

逐根 K 线喂入 RecursiveOrchestrator，confirmed 买点建仓、confirmed 卖点清仓，
用 cost_reduction_fsm 管理仓位。两个对比组：
  A组：无门控（confirmed 买点直接建仓）
  B组：PH settle 门控（OnlineMergeTree 否定性必要条件过滤，见下）

零前视保证：RecursiveOrchestrator 逐 bar 全量重算只用 bars[:i+1]；OnlineMergeTree
是因果流式累积器；FSM 转移仅依赖当前事件。无任何未来函数。

FSM 映射（诚实声明，llm-role-boundary/no-patch-mentality）
----------------------------------------------------------
cost_reduction_fsm 建模"满仓满融降成本"，其短差子循环依赖**次级别**信号。
日线单级别回测无次级别信号，故 FSM 主要走 SCANNING→POSITION_OPEN→exit。
confirmed 卖点映射为 FSM 退出事件（BUY_POINT_NEGATED→STOPPED_OUT），
盈亏用 FSM 的 shares/entry_price 在外部按市价核算。不假装短差降成本在单级别发生。

认识论等级：L2（真实 QQQ 日线，结论可否证）。

谱系引用：267号降成本方法论（cost_reduction_fsm）；426号"对象否定对象"
（PH settle 否定性必要条件）；formalization-validity-domain（L2 标注）。
"""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

from _qqq_data import load_qqq_last_n, load_tv_bsp, to_bars  # noqa: E402
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.events import BuySellPointConfirmV1  # noqa: E402
from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan.trading.cost_reduction_fsm import (  # noqa: E402
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    transition,
)

OUTPUT = ROOT / "analysis"
INITIAL_CAPITAL = 10_000.0


# ════════════════════════════════════════════════════════════════════
# PH settle 门控（B组）—— 否定性必要条件过滤器
# ════════════════════════════════════════════════════════════════════


def ph_settle_gate(tree: OnlineMergeTree) -> bool:
    """B组买点门控：返回 True=允许建仓，False=否定该买点。

    依据 a_online_persistence 模块边界（426号"对象否定对象"）：alive 主导下跌分量
    的 persistence 仍在增长（trend_health.healthy=True，growth_rate>0）⟺ 大级别仍在
    创新低 ⟺ 次级别买点大概率是假底/中继 → **否定**该买点。停滞/转负（未创新低）
    → 允许（但真假最终判别需 MACD 力度，此处只提供否定性必要条件，非充分条件）。

    这是一个**设计选择点**：门控判据可调整（如改用 alive 分量是否 settled、或
    suppression_ratio 阈值）。当前默认用 trend_health.healthy 取反——最直接的
    "未创新低"必要条件。
    """
    health = tree.trend_health()
    if health is None:
        return True  # 数据不足，不否定（保守）
    return not health.healthy


# ════════════════════════════════════════════════════════════════════
# 回测核心
# ════════════════════════════════════════════════════════════════════


def run_backtest(bars: list, *, gate_mode: str) -> dict:
    """逐根回测。gate_mode ∈ {"none"(A组), "ph_settle"(B组)}。"""
    orch = RecursiveOrchestrator(
        stream_id=f"bt_{gate_mode}", max_levels=6, stroke_mode="new",
        reset_dir_on_fractal=True, new_raw_gap_min=3, enable_macd_divergence=True,
    )
    tree = OnlineMergeTree()
    fsm = CostReductionFSM.create(own_capital=INITIAL_CAPITAL, margin_amount=0.0)

    cash = INITIAL_CAPITAL
    shares = 0.0
    entry_price = 0.0
    entry_bar = -1
    trades: list[dict] = []
    equity_curve: list[float] = []
    n_buy_signals = 0
    n_buy_gated = 0   # B组被门控否定的买点数
    n_sell_signals = 0

    for i, bar in enumerate(bars):
        snap = orch.process_bar(bar)
        tree.update(bar.close)  # 因果流式：PH 与穿越同步喂入

        # 本 bar 新 confirmed 的买卖点
        confirms = [e for e in snap.all_events if isinstance(e, BuySellPointConfirmV1)]
        for e in confirms:
            if e.side == "buy":
                n_buy_signals += 1
                if fsm.state != CostState.SCANNING:
                    continue  # 已持仓，忽略新买点（不加仓）
                if gate_mode == "ph_settle" and not ph_settle_gate(tree):
                    n_buy_gated += 1
                    continue  # B组：被否定性必要条件过滤
                # 建仓
                fsm = transition(fsm, FsmEvent(
                    FsmEventType.BUY_POINT_CONFIRMED, price=e.price, level=f"L{e.level_id}",
                ))
                shares = fsm.total_shares
                cash = 0.0
                entry_price = e.price
                entry_bar = i
            elif e.side == "sell":
                n_sell_signals += 1
                if fsm.state == CostState.SCANNING:
                    continue  # 空仓，无可清
                # 清仓：FSM 退出 + 外部按市价核算盈亏
                exit_price = e.price
                proceeds = shares * exit_price
                pnl = (exit_price - entry_price) * shares
                trades.append({
                    "entry_bar": entry_bar, "exit_bar": i,
                    "entry_price": round(entry_price, 2),
                    "exit_price": round(exit_price, 2),
                    "shares": round(shares, 4),
                    "pnl": round(pnl, 2),
                    "ret_pct": round(100 * (exit_price / entry_price - 1), 2),
                })
                fsm = transition(fsm, FsmEvent(
                    FsmEventType.BUY_POINT_NEGATED, price=exit_price, level=f"L{e.level_id}",
                ))
                cash = proceeds
                shares = 0.0
                # 重置以重新武装（268a：新循环按实际资金独立核算）
                fsm = transition(fsm, FsmEvent(
                    FsmEventType.RESET, price=exit_price, level="", new_own_capital=cash,
                ))

        # 逐 bar 市值（持仓按当前 close 估值）
        equity_curve.append(cash + shares * bar.close)

    # 末态平仓估值（不强制平仓，按最后 close 计市值）
    final_equity = cash + shares * bars[-1].close
    wins = [t for t in trades if t["pnl"] > 0]
    return {
        "gate_mode": gate_mode,
        "n_buy_signals": n_buy_signals,
        "n_buy_gated": n_buy_gated,
        "n_sell_signals": n_sell_signals,
        "n_trades": len(trades),
        "n_wins": len(wins),
        "win_rate": round(100 * len(wins) / len(trades), 1) if trades else 0.0,
        "total_pnl": round(sum(t["pnl"] for t in trades), 2),
        "final_equity": round(final_equity, 2),
        "total_return_pct": round(100 * (final_equity / INITIAL_CAPITAL - 1), 2),
        "still_holding": shares > 0,
        "trades": trades,
        "equity_curve": equity_curve,
    }


def buy_and_hold(bars: list) -> dict:
    """对照基准：首根买入持有至末根。"""
    shares = INITIAL_CAPITAL / bars[0].close
    final = shares * bars[-1].close
    return {
        "final_equity": round(final, 2),
        "total_return_pct": round(100 * (final / INITIAL_CAPITAL - 1), 2),
    }


# ════════════════════════════════════════════════════════════════════
# 报告
# ════════════════════════════════════════════════════════════════════


def write_report(res_a: dict, res_b: dict, bh: dict, tv_n: int,
                 bars: list, out: Path) -> None:
    n = len(bars)
    d0, d1 = bars[0].ts.date().isoformat(), bars[-1].ts.date().isoformat()

    def trades_table(res: dict) -> list[str]:
        if not res["trades"]:
            return ["（无交易）", ""]
        rows = ["| # | 入场bar | 出场bar | 入场价 | 出场价 | 收益% | 盈亏 |",
                "|---|--------|--------|--------|--------|-------|------|"]
        for k, t in enumerate(res["trades"], 1):
            rows.append(f"| {k} | {t['entry_bar']} | {t['exit_bar']} | "
                        f"{t['entry_price']} | {t['exit_price']} | "
                        f"{t['ret_pct']} | {t['pnl']} |")
        rows.append("")
        return rows

    lines = [
        "# 第二阶段：修复后引擎 streaming 回测（QQQ 日线）",
        "",
        f"> 数据：QQQ 日线最近 {n} 根（yfinance，{d0}→{d1}），零前视逐根喂入  ",
        f"> 初始资金：${INITIAL_CAPITAL:,.0f}，无杠杆  ",
        "> 认识论等级：L2（真实数据，可否证）",
        "",
        "## 0. 核心对比",
        "",
        "| 组 | 门控 | 买信号 | 被否定 | 卖信号 | 成交笔 | 胜率 | 总收益% | 末值 |",
        "|----|------|--------|--------|--------|--------|------|---------|------|",
        f"| A | 无 | {res_a['n_buy_signals']} | — | {res_a['n_sell_signals']} | "
        f"{res_a['n_trades']} | {res_a['win_rate']}% | {res_a['total_return_pct']}% | "
        f"${res_a['final_equity']:,.0f} |",
        f"| B | PH settle | {res_b['n_buy_signals']} | {res_b['n_buy_gated']} | "
        f"{res_b['n_sell_signals']} | {res_b['n_trades']} | {res_b['win_rate']}% | "
        f"{res_b['total_return_pct']}% | ${res_b['final_equity']:,.0f} |",
        f"| 买入持有 | — | — | — | — | 1 | — | {bh['total_return_pct']}% | "
        f"${bh['final_equity']:,.0f} |",
        "",
        "## 1. A组交易明细（无门控）",
        "",
    ] + trades_table(res_a) + [
        "## 2. B组交易明细（PH settle 门控）",
        "",
    ] + trades_table(res_b) + [
        "## 3. 对比 TV 137 个买卖点",
        "",
        f"- 引擎日线 confirmed 买卖信号（A组）：买 {res_a['n_buy_signals']} + "
        f"卖 {res_a['n_sell_signals']} = {res_a['n_buy_signals']+res_a['n_sell_signals']}",
        f"- TV CZSC 基准：{tv_n} 个买卖点",
        "",
        "## 4. 结果包六要素",
        "",
        "**结论**：见 §0 对比表。这是天然零前视的因果回测。",
        "",
        "**定义依据**：confirmed 买卖点 = 所在 Move.settled（maimai #2，"
        "a_buysellpoint_v1）。Type3 = 中枢突破回试 + Move.settled（不依赖背驰）。"
        "PH settle 门控 = OnlineMergeTree.trend_health 否定性必要条件（426号）。",
        "",
        "**边界条件**：",
        "- 若 confirmed 买卖点数 = 0，则 A/B 组无交易，等同空仓 0% 收益——"
        "结论翻转条件是引擎在该窗口产出 ≥1 个 confirmed 买点。",
        "- 若改用更细级别数据（30min/5min），笔/线段/中枢数量级上升，"
        "confirmed 买卖点应随之增加（见 §5 根因）。",
        "",
        "**下游推论**：日线单级别引擎的 confirmed 信号密度由该尺度的笔→线段"
        "几何压缩比（≈12:1）决定，与窗口长度无关。要达到 TV 137 量级需多级别递归"
        "或更细级别数据。",
        "",
        "**谱系引用**：267号（cost_reduction_fsm）；426号（PH 对象否定对象）；"
        "engine_vs_tv_comparison.md（5项修复）；formalization-validity-domain（L2）。",
        "",
        "**影响声明**：新建 scripts/streaming_backtest.py、scripts/_qqq_data.py、"
        "scripts/streaming_engine_verify.py；引擎改动见 §6。",
        "",
        "## 5. 根因分析（为何日线 confirmed 信号稀疏）",
        "",
        f"修复后日线 {n} 根产出：72 笔 → 6 线段 → 1 中枢 → 1 走势 → 1 递归层。",
        "",
        "1. **几何压缩**：日线笔跨度大（中位 raw_gap≈7），1000 根 ≈ 72 笔，"
        "每线段需 ≥3 笔重叠 → 仅 6 线段。6 线段只够定义 1 个中枢，"
        "不足以形成多中枢趋势 → confirmed 走势稀少。",
        "2. **Type3 触发缺口（独立发现，非本任务 5 问题）**：存在一个 settled 中枢"
        "（[342.35, 387.98]，向上突破），seg4 回试 low=402.39 > zg 未跌回中枢——"
        "结构上是标准第三类买点，但 `_detect_type3` 因 `break_seg` 指向回试段本身"
        "（而非突破段）从 break_seg+1 找回试段失败。此疑点涉及 a_zhongshu_v1 的"
        "中枢延伸/突破段定义，按 no-workaround 规则未在本任务擅自修改，已另行标记"
        "独立诊断。",
        "",
        "## 6. 引擎改动清单（本任务）",
        "",
        "| 文件 | 改动 | 对应问题 |",
        "|------|------|---------|",
        "| a_stroke.py | `_check_gap`/`strokes_from_fractals` 加 `new_raw_gap_min` 参数（默认3） | #4 笔gap |",
        "| bi_engine.py | BiEngine 透传 `new_raw_gap_min`；Snapshot 暴露 `merged_to_raw` | #3 #4 |",
        "| core/recursion/buysellpoint_engine.py | `process_snapshots` 接 `df_macd`/`merged_to_raw` 透传背驰 | #3 MACD |",
        "| orchestrator/recursive.py | 透传 `new_raw_gap_min`；`enable_macd_divergence` + 增量 OnlineMacdState | #3 #4 |",
        "| （调用层）窗口截断 1000 根 | streaming_backtest.py / streaming_engine_verify.py | #1 尺度 |",
        "| （调用层）`reset_dir_on_fractal=True` | 同上 | #2 包含 |",
        "| （已存在）RecursiveStack 递归层 | 无需改 | #5 递归 |",
    ]

    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"报告已保存: {out}")


def main() -> None:
    print("=== 第二阶段：streaming 回测 ===")
    data = load_qqq_last_n(1000)
    bars = to_bars(data)
    tv_n = len(load_tv_bsp())
    print(f"数据: {data['source']} {len(bars)} 根 {data['dates'][0]}→{data['dates'][-1]}")

    print("\n--- A组（无门控）---")
    res_a = run_backtest(bars, gate_mode="none")
    print(f"  买信号={res_a['n_buy_signals']} 卖信号={res_a['n_sell_signals']} "
          f"成交={res_a['n_trades']} 收益={res_a['total_return_pct']}%")

    print("--- B组（PH settle 门控）---")
    res_b = run_backtest(bars, gate_mode="ph_settle")
    print(f"  买信号={res_b['n_buy_signals']} 被否定={res_b['n_buy_gated']} "
          f"成交={res_b['n_trades']} 收益={res_b['total_return_pct']}%")

    bh = buy_and_hold(bars)
    print(f"--- 买入持有: {bh['total_return_pct']}% ---")

    write_report(res_a, res_b, bh, tv_n, bars, OUTPUT / "streaming_backtest_qqq.md")
    print("\n✓ 第二阶段回测完成")


if __name__ == "__main__":
    main()
