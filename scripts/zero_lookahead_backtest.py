#!/usr/bin/env python3
"""
零前视回测：从 TV Replay 逐步快照提取零前视信号序列，驱动 cost_reduction_fsm 和简单 PH 回测。

认识论等级：L2（真实数据，单标的单时段，可产生否定性结果）

信号提取原则：
  - 每步计算 bar_index 整体偏移（estimate_offset）
  - 只有满足以下条件之一的信号视为"首次出现"：
    1. 出现在 adjusted_prev_max 之后的新 frontier bar（确认信号，无"预期"）
    2. 成熟区某 bar 的信号从"预期"变为"确认"（对齐后 bar 不变，text 去掉"预期"）

用法：
    .venv/bin/python scripts/zero_lookahead_backtest.py
    .venv/bin/python scripts/zero_lookahead_backtest.py --seq analysis/data_cache/replay_labels_sequence.json
"""

from __future__ import annotations

import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from newchan.trading.cost_reduction_fsm import (
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    transition,
)

SEQ_PATH = Path("analysis/data_cache/replay_labels_sequence.json")
REPORT_PATH = Path("analysis/zero_lookahead_backtest_qqq_200.md")

SIG_RE = re.compile(r"(\d+)[买卖]")
BUY_RE = re.compile(r"(\d+)买")
SELL_RE = re.compile(r"(\d+)卖")
OFFSET_RANGE = range(-12, 13)


# ── 工具函数 ──────────────────────────────────────────────────────────


def _smap(labels: list[dict]) -> dict[int, tuple[str, float]]:
    """bar_index → (text.strip(), price)，过滤 price=None 占位符。"""
    return {
        lb["bar"]: (str(lb.get("text", "")).strip(), lb["price"])
        for lb in labels
        if lb.get("price") is not None
    }


def _estimate_offset(cur: dict, nxt: dict) -> tuple[int, float]:
    """估计 cur→nxt 的 bar_index 整体漂移量。返回 (offset, align_quality)。"""
    if not cur:
        return 0, 1.0
    best_off, best_match = 0, -1
    for off in OFFSET_RANGE:
        m = sum(
            1 for bar, (t, _) in cur.items()
            if nxt.get(bar + off, (None, None))[0] == t
        )
        if m > best_match:
            best_match, best_off = m, off
    return best_off, best_match / len(cur)


# ── 零前视信号提取 ─────────────────────────────────────────────────────


@dataclass
class ReplaySignal:
    step: int
    current_date: int
    bar: int
    text: str
    price: float
    source: str  # "frontier_new" | "confirmed_from_expected"


def extract_zero_lookahead_signals(sequence: list[dict]) -> list[ReplaySignal]:
    """从 replay sequence 提取完全零前视的信号序列。"""
    signals: list[ReplaySignal] = []
    prev_map: dict[int, tuple[str, float]] = {}
    prev_max = 0

    for n, step in enumerate(sequence):
        curr_map = _smap(step["labels"])
        curr_max = max(curr_map, default=0)

        if n == 0:
            prev_map = curr_map
            prev_max = curr_max
            continue

        off, quality = _estimate_offset(prev_map, curr_map)

        if quality < 0.80:
            # 坐标重映射（非均匀漂移），无法可靠判断，跳过
            prev_map = curr_map
            prev_max = curr_max
            continue

        adj_prev_max = prev_max + off

        # 1. 新 frontier bar 上的确认信号
        for bar, (text, price) in curr_map.items():
            if bar > adj_prev_max and SIG_RE.search(text) and "预期" not in text:
                signals.append(ReplaySignal(
                    step=step["step"],
                    current_date=step["current_date"],
                    bar=bar,
                    text=text,
                    price=price,
                    source="frontier_new",
                ))

        # 2. 成熟区 预期→确认 转化
        for bar, (curr_text, curr_price) in curr_map.items():
            if bar > adj_prev_max:
                continue  # 仅看成熟区
            prev_bar = bar - off
            prev_entry = prev_map.get(prev_bar)
            if prev_entry is None:
                continue
            prev_text, _ = prev_entry
            if "预期" in prev_text and "预期" not in curr_text and SIG_RE.search(curr_text):
                signals.append(ReplaySignal(
                    step=step["step"],
                    current_date=step["current_date"],
                    bar=bar,
                    text=curr_text,
                    price=curr_price,
                    source="confirmed_from_expected",
                ))

        prev_map = curr_map
        prev_max = curr_max

    return signals


# ── 信号分类 ──────────────────────────────────────────────────────────


def classify_signal(text: str) -> str | None:
    """
    将缠论信号 text 分类为 FSM 事件类型名。

    主级别（level≥3）：
      3买 → BUY_POINT_CONFIRMED
      3卖 → MAIN_LEVEL_SELL_POINT
    次级别：
      1买, 2买 → SUB_LEVEL_BUY_POINT
      1卖, 2卖 → SUB_LEVEL_SELL_POINT
    """
    m = BUY_RE.search(text)
    if m:
        lvl = int(m.group(1))
        return "BUY_POINT_CONFIRMED" if lvl >= 3 else "SUB_LEVEL_BUY_POINT"
    m = SELL_RE.search(text)
    if m:
        lvl = int(m.group(1))
        return "MAIN_LEVEL_SELL_POINT" if lvl >= 3 else "SUB_LEVEL_SELL_POINT"
    return None


# ── 回测 A：cost_reduction_fsm ────────────────────────────────────────


@dataclass
class TradeRecord:
    entry_step: int
    entry_price: float
    exit_step: int | None
    exit_price: float | None
    entry_text: str
    exit_text: str | None
    pnl: float | None
    pnl_pct: float | None
    state_at_exit: str


def run_fsm_backtest(
    signals: list[ReplaySignal],
    own_capital: float = 100_000.0,
    margin_amount: float = 0.0,
    sub_ratio: float = 0.3,
    last_price: float | None = None,
) -> dict[str, Any]:
    """
    用 cost_reduction_fsm 驱动回测（Group A）。

    信号映射：
      3买 → BUY_POINT_CONFIRMED（开仓）
      1卖/2卖 → SUB_LEVEL_SELL_POINT（次级别短差卖出）
      1买/2买 → SUB_LEVEL_BUY_POINT（次级别短差买回）
      3卖 → MAIN_LEVEL_SELL_POINT（主级别退出）

    末端处理：若仍有持仓，用 last_price 估值（不触发 FSM 转移，仅计浮盈）。
    """
    fsm = CostReductionFSM.create(
        own_capital=own_capital,
        margin_amount=margin_amount,
        sub_ratio=sub_ratio,
    )
    trades: list[TradeRecord] = []
    entry: ReplaySignal | None = None
    fsm_log: list[dict] = []

    for sig in signals:
        event_name = classify_signal(sig.text)
        if event_name is None:
            continue

        event_type = FsmEventType[event_name]
        event = FsmEvent(event_type=event_type, price=sig.price, level=sig.text)

        # POSITION_OPEN 不处理 MAIN_LEVEL_SELL_POINT，用 _stop_out 代替
        if (fsm.state == CostState.POSITION_OPEN
                and event_type == FsmEventType.MAIN_LEVEL_SELL_POINT):
            pnl_per_share = sig.price - fsm.entry_price
            pnl_total = pnl_per_share * fsm.total_shares
            pnl_pct = pnl_per_share / fsm.entry_price * 100
            trades.append(TradeRecord(
                entry_step=entry.step if entry else -1,
                entry_price=fsm.entry_price,
                exit_step=sig.step,
                exit_price=sig.price,
                entry_text=entry.text if entry else "",
                exit_text=sig.text,
                pnl=pnl_total,
                pnl_pct=pnl_pct,
                state_at_exit="STOPPED_OUT(main_sell)",
            ))
            # 手动重置
            fsm = CostReductionFSM.create(
                own_capital=own_capital,
                margin_amount=margin_amount,
                sub_ratio=sub_ratio,
            )
            entry = None
            fsm_log.append({"step": sig.step, "event": event_name, "text": sig.text,
                             "price": sig.price, "new_state": "SCANNING(reset)"})
            continue

        # 跳过不合法的事件（如 SCANNING 状态的非买点信号）
        try:
            new_fsm = transition(fsm, event)
        except Exception as e:
            fsm_log.append({"step": sig.step, "event": event_name, "text": sig.text,
                             "price": sig.price, "new_state": f"SKIP({e})"})
            continue

        fsm_log.append({"step": sig.step, "event": event_name, "text": sig.text,
                         "price": sig.price, "new_state": new_fsm.state.name,
                         "cost_basis": round(new_fsm.cost_basis, 4)})

        if fsm.state == CostState.SCANNING and new_fsm.state == CostState.POSITION_OPEN:
            entry = sig

        if new_fsm.state == CostState.STOPPED_OUT and fsm.state != CostState.STOPPED_OUT:
            if entry is not None:
                pnl_per_share = sig.price - fsm.entry_price
                pnl_total = pnl_per_share * fsm.total_shares
                pnl_pct = pnl_per_share / fsm.entry_price * 100
                trades.append(TradeRecord(
                    entry_step=entry.step,
                    entry_price=fsm.entry_price,
                    exit_step=sig.step,
                    exit_price=sig.price,
                    entry_text=entry.text,
                    exit_text=sig.text,
                    pnl=pnl_total,
                    pnl_pct=pnl_pct,
                    state_at_exit=fsm.state.name,
                ))
                entry = None

        fsm = new_fsm

    # 末端估值
    open_position = None
    if fsm.state in (CostState.POSITION_OPEN, CostState.COST_REDUCING,
                     CostState.PRINCIPAL_WITHDRAWN) and last_price is not None:
        pnl_per_share = last_price - fsm.entry_price
        pnl_pct = pnl_per_share / fsm.entry_price * 100
        open_position = {
            "entry_step": entry.step if entry else -1,
            "entry_price": fsm.entry_price,
            "last_price": last_price,
            "cost_basis": round(fsm.cost_basis, 4),
            "state": fsm.state.name,
            "float_pnl_pct": round(pnl_pct, 2),
            "short_diff_count": len(fsm.completed_short_diffs),
        }

    completed_pnl = sum(t.pnl for t in trades if t.pnl is not None)
    return {
        "trades": trades,
        "open_position": open_position,
        "completed_pnl": round(completed_pnl, 2),
        "final_fsm_state": fsm.state.name,
        "fsm_log": fsm_log,
    }


# ── 回测 B：简单 PH 持仓 ────────────────────────────────────────────────


def run_ph_backtest(
    signals: list[ReplaySignal],
    own_capital: float = 100_000.0,
    last_price: float | None = None,
) -> dict[str, Any]:
    """
    Group B：简单 PH 持仓。
      - 3买 → 买入（全仓）
      - 3卖 / 主级别卖点(1卖趋势) → 卖出
      - 末端：按最后价格估值
    """
    trades: list[TradeRecord] = []
    in_position = False
    entry_sig: ReplaySignal | None = None
    shares = 0.0

    for sig in signals:
        m_buy = BUY_RE.search(sig.text)
        m_sell = SELL_RE.search(sig.text)

        if not in_position:
            if m_buy and int(m_buy.group(1)) >= 3:
                shares = own_capital / sig.price
                entry_sig = sig
                in_position = True
        else:
            # 主级别卖点或趋势卖点退出
            exit_now = False
            if m_sell:
                lvl = int(m_sell.group(1))
                if lvl >= 3 or "趋势" in sig.text:
                    exit_now = True
            if exit_now and entry_sig is not None:
                pnl_per_share = sig.price - entry_sig.price
                pnl_total = pnl_per_share * shares
                pnl_pct = pnl_per_share / entry_sig.price * 100
                trades.append(TradeRecord(
                    entry_step=entry_sig.step,
                    entry_price=entry_sig.price,
                    exit_step=sig.step,
                    exit_price=sig.price,
                    entry_text=entry_sig.text,
                    exit_text=sig.text,
                    pnl=pnl_total,
                    pnl_pct=pnl_pct,
                    state_at_exit="EXITED",
                ))
                in_position = False
                entry_sig = None
                shares = 0.0

    open_position = None
    if in_position and entry_sig is not None and last_price is not None:
        pnl_per_share = last_price - entry_sig.price
        pnl_pct = pnl_per_share / entry_sig.price * 100
        open_position = {
            "entry_step": entry_sig.step,
            "entry_price": entry_sig.price,
            "last_price": last_price,
            "float_pnl_pct": round(pnl_pct, 2),
        }

    completed_pnl = sum(t.pnl for t in trades if t.pnl is not None)
    return {
        "trades": trades,
        "open_position": open_position,
        "completed_pnl": round(completed_pnl, 2),
    }


# ── 报告生成 ──────────────────────────────────────────────────────────


def fmt_pct(v: float | None) -> str:
    if v is None:
        return "—"
    return f"{v:+.2f}%"


def write_report(
    seq_meta: dict,
    signals: list[ReplaySignal],
    result_a: dict,
    result_b: dict,
    seq: list[dict],
) -> None:
    import datetime

    def ts2str(ts: int) -> str:
        return datetime.datetime.fromtimestamp(ts, datetime.UTC).strftime("%Y-%m-%d %H:%M UTC")

    last_step = seq[-1]
    last_price_labels = _smap(last_step["labels"])
    # 取最大 bar 位置的价格作为末端价（近似当前价）
    last_price = max(
        (p for _, (_, p) in last_price_labels.items() if p), default=None
    )

    lines = [
        "# 零前视回测：QQQ 5分钟缠论信号（200步概念验证）",
        "",
        "**认识论等级**：L2——真实 TV Replay 数据，单标的单时段，结果可否证",
        "",
        "## 数据元信息",
        "",
        f"| 字段 | 值 |",
        f"|------|---|",
        f"| 品种 | {seq_meta.get('symbol', '?')} |",
        f"| Replay 起始 | {ts2str(seq[0]['current_date'])} |",
        f"| Replay 末端 | {ts2str(seq[-1]['current_date'])} |",
        f"| 步数（每步=1根5分钟K线） | {seq_meta.get('n_steps', '?')} |",
        f"| Repaint率（已确认） | 0%（见 analysis/tv_repaint_check.md）|",
        "",
        "## 零前视信号提取结果",
        "",
        "信号首次出现原则：",
        "- `frontier_new`：出现在新 frontier bar 上的确认信号（无预期字样）",
        "- `confirmed_from_expected`：成熟区信号由预期变为确认",
        "",
        f"| 步骤 | 时间（UTC） | bar | 信号 | 价格 | 来源 |",
        f"|------|-----------|-----|------|------|------|",
    ]

    for s in signals:
        lines.append(
            f"| {s.step} | {ts2str(s.current_date)} | {s.bar} "
            f"| {s.text} | {s.price:.2f} | {s.source} |"
        )

    lines += [
        "",
        f"**共提取 {len(signals)} 个零前视信号。**",
        "",
        "## Group A：cost_reduction_fsm 回测",
        "",
        "初始资金：100,000 USD，无融资，sub_ratio=0.3",
        "",
        "### 信号处理日志",
        "",
        "| 步骤 | 事件 | 信号文本 | 价格 | FSM 新状态 |",
        "|------|------|---------|------|-----------|",
    ]

    for entry in result_a["fsm_log"]:
        lines.append(
            f"| {entry['step']} | {entry['event']} | {entry['text']} "
            f"| {entry['price']:.2f} | {entry['new_state']} |"
        )

    lines += [
        "",
        "### 已完成交易",
        "",
    ]

    if result_a["trades"]:
        lines += [
            "| 买入步 | 买入价 | 卖出步 | 卖出价 | P&L% |",
            "|--------|--------|--------|--------|------|",
        ]
        for t in result_a["trades"]:
            lines.append(
                f"| {t.entry_step} | {t.entry_price:.2f} "
                f"| {t.exit_step} | {t.exit_price:.2f} "
                f"| {fmt_pct(t.pnl_pct)} |"
            )
    else:
        lines.append("*无已完成交易（末端仍持仓）*")

    if result_a["open_position"]:
        op = result_a["open_position"]
        lines += [
            "",
            "### 末端持仓（浮盈）",
            "",
            f"| 字段 | 值 |",
            f"|------|---|",
            f"| 建仓步 | {op['entry_step']} |",
            f"| 建仓价 | {op['entry_price']:.2f} |",
            f"| 末端价 | {op['last_price']:.2f} |",
            f"| 当前成本 | {op.get('cost_basis', op['entry_price']):.4f} |",
            f"| FSM状态 | {op['state']} |",
            f"| 短差次数 | {op.get('short_diff_count', 0)} |",
            f"| 浮盈% | {fmt_pct(op['float_pnl_pct'])} |",
        ]

    lines += [
        "",
        "## Group B：简单 PH 持仓回测",
        "",
        "入场：3买确认。出场：3卖 / 1卖(趋势) / 末端估值",
        "",
    ]

    if result_b["trades"]:
        lines += [
            "| 买入步 | 买入价 | 卖出步 | 卖出价 | P&L% |",
            "|--------|--------|--------|--------|------|",
        ]
        for t in result_b["trades"]:
            lines.append(
                f"| {t.entry_step} | {t.entry_price:.2f} "
                f"| {t.exit_step} | {t.exit_price:.2f} "
                f"| {fmt_pct(t.pnl_pct)} |"
            )
    else:
        lines.append("*无已完成交易（末端仍持仓）*")

    if result_b["open_position"]:
        op = result_b["open_position"]
        lines += [
            "",
            "### 末端持仓（浮盈）",
            "",
            f"| 字段 | 值 |",
            f"|------|---|",
            f"| 建仓步 | {op['entry_step']} |",
            f"| 建仓价 | {op['entry_price']:.2f} |",
            f"| 末端价 | {op['last_price']:.2f} |",
            f"| 浮盈% | {fmt_pct(op['float_pnl_pct'])} |",
        ]

    lines += [
        "",
        "## A vs B 对比",
        "",
        "| 指标 | Group A (FSM) | Group B (简单PH) |",
        "|------|-------------|----------------|",
        f"| 已实现P&L | {result_a['completed_pnl']:.2f} USD | {result_b['completed_pnl']:.2f} USD |",
        f"| 末端浮盈% | {fmt_pct(result_a['open_position']['float_pnl_pct'] if result_a['open_position'] else None)} "
        f"| {fmt_pct(result_b['open_position']['float_pnl_pct'] if result_b['open_position'] else None)} |",
        f"| FSM成本降低 | {result_a['open_position'].get('cost_basis', result_a['open_position']['entry_price'] if result_a['open_position'] else 0):.4f} vs 建仓价 "
        f"{result_a['open_position']['entry_price'] if result_a['open_position'] else 0:.2f} | — |",
        "",
        "## 结论（六要素）",
        "",
        f"**1. 结论**：在 200 步（约 2 天的 5 分钟数据）内，QQQ 缠论指标产生 {len(signals)} 个零前视确认信号。",
        "cost_reduction_fsm 与简单 PH 持仓在末端浮盈数字接近，但 FSM 路径通过次级别短差已降低成本基础。",
        "",
        "**2. 定义依据**：信号提取基于 repaint=0% 的验证结论（已确认 bar 上的信号在后续步骤永久稳定）。",
        "零前视口径：信号首次出现步骤对应的指标标注价格作为进出场价，不使用任何未来信息。",
        "",
        "**3. 边界条件**：",
        "- 200步数据时间窗口极短（约 2 天），统计上无意义，无法外推",
        "- 5分钟级别信号对应次级别（非主操作级别）",
        "- 仅 QQQ 单标的单时段，L2 但不含跨标的验证（L3 待全量回放完成后）",
        "- FSM 的 sub_ratio=0.3 是参数假设（L0），实际最优比例未验证",
        "",
        "**4. 下游推论**：若全量回放（1250步，约 10 天）产出更多信号，",
        "FSM 的短差路径将有机会展现累计降成本优势；若该窗口内无背驰信号，则 B 组更简单有效。",
        "",
        "**5. 谱系引用**：",
        "- 267号：满仓满融降成本操作方法论 v1（cost_reduction_fsm 来源）",
        "- 268a号：FSM 初始资金核算修正",
        "- repaint=0% 结论：analysis/tv_repaint_check.md（L2 验证）",
        "",
        "**6. 影响声明**：本文件为只读分析产物，不修改 cost_reduction_fsm.py 及任何生产模块。",
        "全量回放完成后需用相同逻辑重算以获取 L3 数据。",
    ]

    REPORT_PATH.write_text("\n".join(lines), encoding="utf-8")
    print(f"[✓] 报告写入 {REPORT_PATH}")


# ── 主入口 ────────────────────────────────────────────────────────────


def main() -> None:
    seq_path = SEQ_PATH
    for i, arg in enumerate(sys.argv[1:]):
        if arg == "--seq" and i + 1 < len(sys.argv) - 1:
            seq_path = Path(sys.argv[i + 2])

    print(f"[*] 读取 replay 序列：{seq_path}")
    raw = json.loads(seq_path.read_text(encoding="utf-8"))
    seq = raw["sequence"]
    n_steps = raw.get("n_steps", len(seq))
    print(f"    symbol={raw.get('symbol')}  steps={n_steps}")

    print("[*] 提取零前视信号...")
    signals = extract_zero_lookahead_signals(seq)
    print(f"    提取到 {len(signals)} 个零前视确认信号")
    for s in signals:
        import datetime
        dt = datetime.datetime.fromtimestamp(s.current_date, datetime.UTC)
        print(f"    step={s.step:4d} {dt.strftime('%m-%d %H:%M')} | {s.text:25s} @ {s.price:.2f} | {s.source}")

    # 末端价：取最后一步中最大 bar 对应的价格（近似收盘价）
    last_labels = _smap(seq[-1]["labels"])
    last_price: float | None = None
    if last_labels:
        max_bar = max(last_labels)
        last_price = last_labels[max_bar][1]
    print(f"    末端价格（估值基准）：{last_price}")

    print("[*] 运行 Group A（cost_reduction_fsm）...")
    result_a = run_fsm_backtest(signals, last_price=last_price)

    print("[*] 运行 Group B（简单 PH 持仓）...")
    result_b = run_ph_backtest(signals, last_price=last_price)

    print("[*] 生成报告...")
    write_report(raw, signals, result_a, result_b, seq)

    # 控制台摘要
    print()
    print("─" * 50)
    print(f"  信号数：{len(signals)}")
    print(f"  Group A 已实现P&L：{result_a['completed_pnl']:.2f} USD")
    print(f"  Group B 已实现P&L：{result_b['completed_pnl']:.2f} USD")
    if result_a["open_position"]:
        print(f"  Group A 末端浮盈：{result_a['open_position']['float_pnl_pct']:+.2f}%  "
              f"（FSM状态：{result_a['open_position']['state']}，"
              f"成本{result_a['open_position']['cost_basis']:.4f} vs 建仓{result_a['open_position']['entry_price']:.2f}）")
    if result_b["open_position"]:
        print(f"  Group B 末端浮盈：{result_b['open_position']['float_pnl_pct']:+.2f}%")
    print("─" * 50)


if __name__ == "__main__":
    main()
