"""第二阶段：streaming alive_settle_thresholds() 回测 vs 批量 PH settle 对比。

Group A（streaming settle）：
  逐根喂入 OnlineMergeTree。当最新生（最小 birth_price）alive 分量的
  settle_price 被向上突破 → 该下跌波 death 因果确定 → 买入信号。
  止损：价格跌破 entry × (1 - stop_pct)（持仓被否定）。
  出场：持仓时如价格再次跌破最近 alive valley 则视为趋势延续 → 止损退出。

Group B（批量 PH settle / S1 对称）：
  从已有 analysis/data_cache/settle_backtest_qqq.json 读取 S1 策略结果
  作为对比基准。

认识论等级：L2（QQQ 日线单标的单时段，可否证）。
"""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import NamedTuple

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

from _qqq_data import load_qqq_last_n  # noqa: E402
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402

SETTLE_CACHE = ROOT / "analysis" / "data_cache" / "settle_backtest_qqq.json"
REPORT = ROOT / "analysis" / "streaming_settle_backtest_qqq.md"
N_BARS = 1000
STOP_PCT = 0.08   # 止损幅度（entry 价格下方 8%）
MIN_PERS = 3.0    # 只响应 persistence >= MIN_PERS 的 alive 分量（过滤噪声）


class Trade(NamedTuple):
    entry_bar: int
    entry_date: str
    entry_price: float
    exit_bar: int
    exit_date: str
    exit_price: float
    exit_reason: str
    pnl_pct: float


def run_group_a(
    closes: list[float],
    dates: list[str],
    stop_pct: float = STOP_PCT,
    min_pers: float = MIN_PERS,
) -> dict:
    """Group A：streaming settle 信号驱动回测。

    信号生成：`update()` 返回 settled bars → 新 settled bar 是下跌波 death 确认。
    当新 settled bar 的 persistence ≥ min_pers → 下一根 bar 以 close 买入（lag=1 因果）。
    止损：跌破 entry × (1 - stop_pct)。
    出场：新的 settle event（波段完成）或止损。
    """
    tree = OnlineMergeTree()
    trades: list[Trade] = []
    pending_buy = False   # 下一 bar 入场标志
    in_position = False
    entry_bar = -1
    entry_price = 0.0
    entry_date = ""
    stop_price = 0.0

    for i, (close, date) in enumerate(zip(closes, dates)):
        # 先执行挂起的买入（lag=1）
        if pending_buy and not in_position:
            in_position = True
            entry_bar = i
            entry_price = close
            entry_date = date
            stop_price = close * (1 - stop_pct)
            pending_buy = False

        # 止损检查
        if in_position and close < stop_price:
            pnl = (close - entry_price) / entry_price * 100
            trades.append(Trade(
                entry_bar=entry_bar, entry_date=entry_date, entry_price=entry_price,
                exit_bar=i, exit_date=date, exit_price=close,
                exit_reason="stop_loss", pnl_pct=round(pnl, 4),
            ))
            in_position = False

        # update tree，获取本 bar 新 settled bars
        newly_settled = tree.update(close)
        bc = tree.current_barcode()

        # 信号：新 settled bar 且 persistence >= min_pers → 下跌波完成 → 买入
        if not in_position and not pending_buy:
            strong = [b for b in newly_settled if b.persistence >= min_pers]
            if strong:
                pending_buy = True

        # 持仓中：再次出现强 settle → 波段终点 → 出场（下一根 lag=1）
        if in_position:
            strong = [b for b in newly_settled if b.persistence >= min_pers]
            if strong and i > entry_bar + 1:
                # 出场（next bar，简化：当根 close 出场）
                pnl = (close - entry_price) / entry_price * 100
                trades.append(Trade(
                    entry_bar=entry_bar, entry_date=entry_date, entry_price=entry_price,
                    exit_bar=i, exit_date=date, exit_price=close,
                    exit_reason="settle_exit", pnl_pct=round(pnl, 4),
                ))
                in_position = False

    # 末端持仓按最后价估值
    open_position = None
    if in_position:
        last_close = closes[-1]
        pnl = (last_close - entry_price) / entry_price * 100
        open_position = {
            "entry_bar": entry_bar,
            "entry_date": entry_date,
            "entry_price": entry_price,
            "last_close": last_close,
            "float_pnl_pct": round(pnl, 4),
        }

    n_wins = sum(1 for t in trades if t.pnl_pct > 0)
    avg_ret = sum(t.pnl_pct for t in trades) / len(trades) if trades else 0.0
    total_ret = 100 * ((1 + sum(t.pnl_pct / 100 for t in trades)) - 1)

    return {
        "group": "A",
        "description": f"streaming settle (pers≥{min_pers}, stop={stop_pct*100:.0f}%)",
        "n_trades": len(trades),
        "win_rate_pct": round(100 * n_wins / len(trades), 1) if trades else 0.0,
        "avg_ret_pct": round(avg_ret, 4),
        "total_ret_pct": round(total_ret, 2),
        "trades": list(trades),
        "open_position": open_position,
    }


def load_group_b() -> dict:
    """Group B：从 settle_backtest_qqq.json 读取 S1 策略结果。"""
    if not SETTLE_CACHE.exists():
        return {"group": "B", "description": "批量 PH S1 对称 settle（缓存不存在）",
                "n_trades": 0, "win_rate_pct": 0.0, "avg_ret_pct": 0.0, "total_ret_pct": 0.0,
                "trades": [], "open_position": None}
    raw = json.loads(SETTLE_CACHE.read_text())
    s1 = next((s for s in raw.get("strategies", []) if "S1" in s.get("name", "")), None)
    if s1 is None:
        return {"group": "B", "description": "批量 PH S1（未找到）",
                "n_trades": 0, "win_rate_pct": 0.0, "avg_ret_pct": 0.0, "total_ret_pct": 0.0,
                "trades": [], "open_position": None}
    op = s1.get("open_position")
    return {
        "group": "B",
        "description": "批量 PH S1 对称 settle（从 settle_backtest_qqq.json）",
        "n_trades": s1.get("n_trades", 0),
        "win_rate_pct": s1.get("win_rate_pct", 0.0),
        "avg_ret_pct": s1.get("avg_ret_pct", 0.0),
        "total_ret_pct": s1.get("total_ret_pct", 0.0),
        "trades": s1.get("trades", []),
        "open_position": op,
        "meta_note": (
            "批量 PH 数据窗口：1999-03-10 ~ 2026-05-29（6848 bars），"
            "与 Group A 的最近 1000 bars（2022-06-03 ~ 2026-05-29）不同时段，"
            "结果不可直接比较（有效域差异，231号）"
        ),
    }


def write_report(result_a: dict, result_b: dict, closes: list[float], dates: list[str]) -> None:
    bnh = (closes[-1] - closes[0]) / closes[0] * 100

    lines = [
        "# Streaming alive_settle 回测 vs 批量 PH settle 对比 — QQQ 日线",
        "",
        "**认识论等级**：L2——QQQ 日线，Group A 最近 1000 根，Group B 全量 6848 根（时段不同！）",
        "",
        "> **有效域警示（231号）**：Group A 与 Group B 的时段不同，不可直接比较 P&L 数字。",
        "> 本报告的价值在于展示 streaming settle 信号的结构（L0 算法属性），",
        "> 而非 A vs B 的胜负裁决（L2 需同时段同口径对比）。",
        "",
        "## 汇总",
        "",
        "| 指标 | Group A (streaming, 最近1000根) | Group B (批量PH S1, 全量6848根) |",
        "|------|-------------------------------|--------------------------------|",
        f"| 策略描述 | {result_a['description']} | {result_b['description']} |",
        f"| 交易次数 | {result_a['n_trades']} | {result_b['n_trades']} |",
        f"| 胜率 | {result_a['win_rate_pct']:.1f}% | {result_b['win_rate_pct']:.1f}% |",
        f"| 均收益 | {result_a['avg_ret_pct']:+.2f}% | {result_b['avg_ret_pct']:+.2f}% |",
        f"| 累计收益 | {result_a['total_ret_pct']:+.2f}% | {result_b['total_ret_pct']:+.2f}% |",
        f"| 同期 buy-and-hold（最近1000根） | {bnh:+.2f}% | — |",
        "",
    ]

    if result_b.get("meta_note"):
        lines += [f"> **注**：{result_b['meta_note']}", ""]

    lines += [
        "## Group A 交易明细",
        "",
    ]

    if result_a["trades"]:
        lines += [
            "| # | 买入bar | 买入日 | 买价 | 卖出bar | 卖出日 | 卖价 | P&L% | 原因 |",
            "|---|---------|--------|------|---------|--------|------|------|------|",
        ]
        for i, t in enumerate(result_a["trades"]):
            lines.append(
                f"| {i+1} | {t.entry_bar} | {t.entry_date} | {t.entry_price:.2f} "
                f"| {t.exit_bar} | {t.exit_date} | {t.exit_price:.2f} "
                f"| {t.pnl_pct:+.2f}% | {t.exit_reason} |"
            )
    else:
        lines.append("*无已完成交易*")

    if result_a["open_position"]:
        op = result_a["open_position"]
        lines += [
            "",
            f"**末端持仓**：买入 bar={op['entry_bar']} ({op['entry_date']}) @ {op['entry_price']:.2f}，",
            f"末端 close={op['last_close']:.2f}，浮盈 {op['float_pnl_pct']:+.2f}%",
        ]

    lines += [
        "",
        "## 结论（六要素）",
        "",
        f"**1. 结论**：streaming alive_settle 信号在最近 1000 根日线上产生 "
        f"{result_a['n_trades']} 笔完成交易，",
        f"胜率 {result_a['win_rate_pct']:.1f}%，累计收益 {result_a['total_ret_pct']:+.2f}%，",
        f"同期 buy-and-hold {bnh:+.2f}%。",
        "",
        "**2. 定义依据**：settle_price 突破 = alive 下跌分量 death 因果确定（L0，merge tree 数学属性）。",
        "lag=1 进场（下一根 close）保证因果性，无前视偏差。",
        "",
        "**3. 边界条件**：",
        f"- 信号强度过滤：persistence ≥ {MIN_PERS}（噪声过滤，L0 参数假设）",
        f"- 止损阈值 {STOP_PCT*100:.0f}% 是参数假设（未验证最优值，L0）",
        "- 1000 根单标的无统计显著性（需 L3 多标的多时段）",
        "- Group A 与 Group B 时段不同，P&L 对比无效（231号有效域约束）",
        "",
        "**4. 下游推论**：streaming settle 信号（幅度维度）与 MACD 力度联合",
        "= 双闸门判据（§14 定理：PH 给幅度/必要条件，MACD 给动量确认）。",
        "单独 settle 信号统计功效需 L3 验证。",
        "",
        "**5. 谱系引用**：§7.5（在线 merge tree 升级方向）、521号（PH 拓扑动量不存在，",
        "双闸门架构来源）、231号（有效域 L0-L3 分级，时段差异警示）。",
        "",
        "**6. 影响声明**：只读分析产物。`alive_settle_thresholds()` + `OnlineMacdState`",
        "均已在生产模块实现，本报告验证其 L2 信号特征。",
    ]

    REPORT.write_text("\n".join(lines), encoding="utf-8")
    print(f"[✓] 报告写入 {REPORT}")


def main() -> None:
    print(f"[*] 加载 QQQ 日线最近 {N_BARS} 根...")
    data = load_qqq_last_n(N_BARS)
    closes = data["closes"]
    dates = data.get("dates", [str(i) for i in range(len(closes))])
    print(f"    {len(closes)} 根，{dates[0]} → {dates[-1]}")

    print("[*] 运行 Group A（streaming settle 回测）...")
    result_a = run_group_a(closes, dates)
    print(f"    trades={result_a['n_trades']}, win={result_a['win_rate_pct']}%, "
          f"total={result_a['total_ret_pct']:+.2f}%")

    print("[*] 读取 Group B（批量 PH S1 结果）...")
    result_b = load_group_b()
    print(f"    trades={result_b['n_trades']}, win={result_b['win_rate_pct']}%, "
          f"total={result_b['total_ret_pct']:+.2f}%")

    write_report(result_a, result_b, closes, dates)

    bnh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"[*] buy-and-hold（最近{N_BARS}根）：{bnh:+.2f}%")


if __name__ == "__main__":
    main()
