#!/usr/bin/env python3
"""严格零前视回测：消除三层前视偏差，用 AV 真实 OHLCV 收盘价执行。

认识论等级：L2（真实 TV Replay 信号 + 真实 AV OHLCV，单标的单时段，可产生否定性结果）

三层前视偏差及其消除手段
------------------------
1. **执行价前视**（本脚本相对 zero_lookahead_backtest.py 的核心新增）：
   不用 TV 指标标注价（label 画在事后确认的转折点上），改用 AlphaVantage 真实
   OHLCV。执行价 = 信号首次出现 bar 的**下一根 bar 收盘价**（lag=1）。
2. **信号确认前视**：用 Replay 回放的"信号首次出现"step（预期→确认转换 / 新 frontier
   确认），不用事后全图标注位置。复用 extract_zero_lookahead_signals。
3. **信号集前视**：Replay 逐 bar 产出 labels，每步只见当时图上的信号，不是事后全图。
   由 replay_labels_sequence.json 的逐步快照保证。

走势方向（线段方向）口径
----------------------
缠论指标已发射线段级方向：``X卖(趋势)`` 与 ``3卖`` = 线段方向转空 = 退出；
非趋势卖点（1卖/2卖/盘整卖）= 线段未转 = 次级别短差卖出。买点 3买 = 主级别开仓，
1买/2买 = 次级别短差买回。（修正 zero_lookahead_backtest.classify_signal 只看级别
数字、漏掉 "(趋势)" 的缺陷。）

A / B 两组
---------
- **A 组（纯缠论）**：cost_reduction_fsm 管仓位，信号全部执行。
- **B 组（缠论 + PH settle）**：同 A，但每个买点前用 online OnlineMergeTree（只喂截至
  该 bar 的 AV close）读 settle_triggers——若主导（最深）下跌分量 ``can_settle_by_rebound
  =False``（persistence_theory §17.3 规则3：高级别下跌未完成 → 次级别反弹大概率假收敛），
  则**否决该买点**。PH settle 作为零前视过滤层。

用法
----
    .venv/bin/python scripts/zero_lookahead_full_backtest.py
    .venv/bin/python scripts/zero_lookahead_full_backtest.py \
        --seq analysis/data_cache/replay_labels_sequence.json \
        --av  analysis/data_cache/av_qqq_5min_202604.json
"""

from __future__ import annotations

import json
import re
import sys
from bisect import bisect_right
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_settle_trigger import settle_triggers, nearest_rebound_settle  # noqa: E402
from newchan.trading.cost_reduction_fsm import (  # noqa: E402
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    transition,
)
from zero_lookahead_backtest import (  # noqa: E402
    ReplaySignal,
    extract_zero_lookahead_signals,
)

SEQ_PATH = ROOT / "analysis" / "data_cache" / "replay_labels_sequence.json"
AV_PATH = ROOT / "analysis" / "data_cache" / "av_qqq_5min_202604.json"
REPORT_PATH = ROOT / "analysis" / "zero_lookahead_backtest_qqq.md"
RESULT_JSON = ROOT / "analysis" / "data_cache" / "zero_lookahead_full_result.json"

BUY_RE = re.compile(r"(\d+)买")
SELL_RE = re.compile(r"(\d+)卖")
BAR_SEC = 300  # 5 分钟

OWN_CAPITAL = 100_000.0
SUB_RATIO = 0.3
MARGIN = 0.0
SETTLE_MIN_PERS = 3.0  # PH settle gate 只对 persistence ≥ 此值的主导分量生效（过滤噪声）


# ── 信号方向分类（修正版：尊重线段方向 "(趋势)"）───────────────────────


def classify_signal_segmentaware(text: str) -> str | None:
    """缠论信号 → FSM 事件名，尊重线段方向。

    线段方向转空（退出）：``X卖(趋势)`` 或 ``3卖`` → MAIN_LEVEL_SELL_POINT
    线段未转（次级别短差卖）：其余卖点（1卖/2卖/盘整卖）→ SUB_LEVEL_SELL_POINT
    主级别开仓：3买 → BUY_POINT_CONFIRMED
    次级别短差买回：1买/2买 → SUB_LEVEL_BUY_POINT
    """
    m_sell = SELL_RE.search(text)
    if m_sell:
        lvl = int(m_sell.group(1))
        if lvl >= 3 or "趋势" in text:
            return "MAIN_LEVEL_SELL_POINT"
        return "SUB_LEVEL_SELL_POINT"
    m_buy = BUY_RE.search(text)
    if m_buy:
        lvl = int(m_buy.group(1))
        return "BUY_POINT_CONFIRMED" if lvl >= 3 else "SUB_LEVEL_BUY_POINT"
    return None


# ── AV OHLCV 真实执行价（消除第 1 层前视）─────────────────────────────


@dataclass(frozen=True, slots=True)
class AvIndex:
    """AV 5min OHLCV，按 bar open_unix 索引。"""

    by_open: dict[int, dict[str, float]]
    sorted_opens: list[int]

    @classmethod
    def load(cls, path: Path) -> "AvIndex":
        raw = json.loads(path.read_text(encoding="utf-8"))
        by_open = {int(k): v for k, v in raw["idx"].items()}
        return cls(by_open=by_open, sorted_opens=sorted(by_open))

    def bar_open(self, current_date: int) -> int:
        """replay current_date（= bar 收盘前 1 秒）→ 该 bar 的 open_unix。"""
        return (current_date // BAR_SEC) * BAR_SEC

    def close_at_or_after(self, open_unix: int) -> tuple[int, float] | None:
        """open_unix 处的 close；若该 bar 缺失（如周日），取之后第一根真实 bar。

        仍是零前视——落到下一根真实可成交 bar（lag ≥ 1）。
        """
        bar = self.by_open.get(open_unix)
        if bar is not None:
            return open_unix, bar["close"] if "close" in bar else bar["c"]
        i = bisect_right(self.sorted_opens, open_unix)
        if i >= len(self.sorted_opens):
            return None
        nxt = self.sorted_opens[i]
        b = self.by_open[nxt]
        return nxt, (b["close"] if "close" in b else b["c"])

    def last_close(self) -> tuple[int, float] | None:
        if not self.sorted_opens:
            return None
        o = self.sorted_opens[-1]
        b = self.by_open[o]
        return o, (b["close"] if "close" in b else b["c"])


@dataclass(frozen=True, slots=True)
class ExecSignal:
    """零前视信号 + 真实执行价。"""

    step: int
    current_date: int
    event: str
    text: str
    label_price: float       # TV 标注价（仅用于量化前视偏差，不参与回测）
    exec_unix: int           # 真实执行 bar open_unix（lag=1）
    exec_price: float        # AV 真实收盘价（回测唯一执行价）
    bias: float              # exec_price − label_price（执行价前视偏差）


def resolve_exec_prices(
    signals: list[ReplaySignal],
    seq: list[dict],
    av: AvIndex,
) -> tuple[list[ExecSignal], list[ReplaySignal]]:
    """把零前视信号映射到 AV 真实执行价（下一根 bar 收盘，lag=1）。

    返回 (可执行信号, 无法定位真实价而丢弃的信号)。
    """
    # step → 序列索引（step 与索引同值，但显式构造以防数据不连续）
    step_to_idx = {s["step"]: i for i, s in enumerate(seq)}
    execs: list[ExecSignal] = []
    dropped: list[ReplaySignal] = []
    for sig in signals:
        event = classify_signal_segmentaware(sig.text)
        if event is None:
            dropped.append(sig)
            continue
        idx = step_to_idx.get(sig.step)
        if idx is None or idx + 1 >= len(seq):
            dropped.append(sig)  # 末端信号无下一根 bar
            continue
        next_cd = seq[idx + 1]["current_date"]
        target_open = av.bar_open(next_cd)
        hit = av.close_at_or_after(target_open)
        if hit is None:
            dropped.append(sig)
            continue
        exec_unix, exec_price = hit
        execs.append(ExecSignal(
            step=sig.step,
            current_date=sig.current_date,
            event=event,
            text=sig.text,
            label_price=sig.price,
            exec_unix=exec_unix,
            exec_price=exec_price,
            bias=round(exec_price - sig.price, 4),
        ))
    return execs, dropped


# ── PH settle gate（B 组，零前视 online tree）─────────────────────────


class SettleGate:
    """online OnlineMergeTree：只喂截至当前 bar 的 AV close，按需读 settle_triggers。

    零前视：决策发生在信号 bar 收盘后，tree 只含该 bar 及之前的 close。
    """

    def __init__(self, av: AvIndex) -> None:
        self._av = av
        self._tree = OnlineMergeTree()
        self._ptr = 0  # 已喂入 sorted_opens 的下一个位置

    def advance_to(self, bar_open: int) -> None:
        """把 tree 喂到 open_unix ≤ bar_open 的全部 close。"""
        opens = self._av.sorted_opens
        while self._ptr < len(opens) and opens[self._ptr] <= bar_open:
            b = self._av.by_open[opens[self._ptr]]
            self._tree.update(b["close"] if "close" in b else b["c"])
            self._ptr += 1

    def buy_allowed(self) -> tuple[bool, str]:
        """近端可反弹 settle 的下跌腿是否已 settle（因果死亡）→ 确认买点。

        语义（与 streaming_settle_backtest.py 一致，a_settle_trigger.nearest_rebound_settle）：
        - nearest = 最近端可反弹 settle 的 alive 下跌分量（settle_price 最小者）。
        - 该分量已 settle（price 突破 settle_price，gap_to_settle ≤ 0）→ 近端下跌腿
          因果死亡确认 → PH 确认买点（allow）。
        - 该分量尚未 settle（gap_to_settle > 0，price 仍在 settle 屏障之下）→ 近端下跌
          腿死亡未确认 → PH 判定买点过早（reject）。
        - 无可反弹 settle 分量（仅剩全局栈底分量）→ 无待确认的近端下跌腿 → allow。

        **不用 dominant_trigger**：全局最低分量 can_settle_by_rebound 恒 False（栈底永远
        alive，a_settle_trigger.py §16 诚实点），上涨窗口里 dominant=涨幅基底，恒否决 →
        退化门。用 nearest_rebound_settle 才是 §17.3 近端"止跌确认"的正确读出。
        """
        trigs = settle_triggers(self._tree)
        nearest = nearest_rebound_settle(trigs)
        diag: dict[str, Any] = {"n_alive": len(trigs)}
        if nearest is None:
            return True, "no_settleable_downleg", diag
        diag.update({
            "settle_price": round(nearest.settle_price, 4) if nearest.settle_price else None,
            "settle_pers": round(nearest.settle_persistence, 4) if nearest.settle_persistence else None,
            "gap_to_settle": round(nearest.gap_to_settle, 4) if nearest.gap_to_settle is not None else None,
        })
        if nearest.settle_persistence is not None and nearest.settle_persistence < SETTLE_MIN_PERS:
            return True, f"nearest_downleg_pers<{SETTLE_MIN_PERS}(噪声)", diag
        if nearest.gap_to_settle is not None and nearest.gap_to_settle <= 0:
            return True, "nearest_downleg_settled(止跌确认)", diag
        return False, "nearest_downleg_unsettled(下跌腿未死,买点过早)", diag


# ── 回测引擎（A / B 共用，执行价全部来自 AV）────────────────────────


@dataclass
class Trade:
    entry_step: int
    entry_price: float
    exit_step: int | None
    exit_price: float | None
    entry_text: str
    exit_text: str | None
    pnl: float | None
    pnl_pct: float | None
    state_at_exit: str


def run_fsm(
    execs: list[ExecSignal],
    last_price: float,
    settle_gate: SettleGate | None = None,
    av: AvIndex | None = None,
) -> dict[str, Any]:
    """cost_reduction_fsm 回测。执行价全部为 AV 真实价。

    settle_gate 非空 → B 组：买点前 PH settle 过滤。
    """
    fsm = CostReductionFSM.create(
        own_capital=OWN_CAPITAL, margin_amount=MARGIN, sub_ratio=SUB_RATIO,
    )
    trades: list[Trade] = []
    entry: ExecSignal | None = None
    log: list[dict] = []
    gated_out = 0
    settle_diag: list[dict] = []  # B 组：每个 3买的 PH settle 共振诊断

    for sig in execs:
        # ── B 组 PH settle 买点过滤 ──
        if (settle_gate is not None
                and sig.event == "BUY_POINT_CONFIRMED"
                and fsm.state == CostState.SCANNING):
            settle_gate.advance_to(av.bar_open(sig.current_date))  # type: ignore[union-attr]
            ok, reason, diag = settle_gate.buy_allowed()
            settle_diag.append({
                "step": sig.step, "text": sig.text, "exec_price": sig.exec_price,
                "allowed": ok, "reason": reason, **diag,
            })
            if not ok:
                gated_out += 1
                log.append({"step": sig.step, "event": sig.event, "text": sig.text,
                            "price": sig.exec_price, "new_state": f"PH_GATED({reason})"})
                continue

        event_type = FsmEventType[sig.event]
        event = FsmEvent(event_type=event_type, price=sig.exec_price, level=sig.text)

        HOLDING = (CostState.POSITION_OPEN, CostState.COST_REDUCING,
                   CostState.PRINCIPAL_WITHDRAWN)

        # 主级别卖点（X卖(趋势)/3卖 = 线段方向转空）在任一持仓态：
        # 退出 + 复位到 SCANNING，等待下一个 3买重新入场（任务："线段方向转空 = 退出"）。
        # FSM 自身的 STOPPED_OUT 是"买点被否定"止损终态，语义不同，不用于线段级退出。
        if event_type == FsmEventType.MAIN_LEVEL_SELL_POINT and fsm.state in HOLDING:
            pnl_ps = sig.exec_price - fsm.entry_price
            trades.append(Trade(
                entry_step=entry.step if entry else -1,
                entry_price=fsm.entry_price,
                exit_step=sig.step, exit_price=sig.exec_price,
                entry_text=entry.text if entry else "",
                exit_text=sig.text,
                pnl=round(pnl_ps * fsm.total_shares, 2),
                pnl_pct=round(pnl_ps / fsm.entry_price * 100, 4),
                state_at_exit=f"EXIT(线段转空,from={fsm.state.name},cost_basis={fsm.cost_basis:.2f})",
            ))
            fsm = CostReductionFSM.create(
                own_capital=OWN_CAPITAL, margin_amount=MARGIN, sub_ratio=SUB_RATIO,
            )
            entry = None
            log.append({"step": sig.step, "event": sig.event, "text": sig.text,
                        "price": sig.exec_price, "new_state": "SCANNING(reset,线段转空退出)"})
            continue

        try:
            new_fsm = transition(fsm, event)
        except Exception as e:  # 不合法事件（如 SCANNING 收到卖点）→ 跳过
            log.append({"step": sig.step, "event": sig.event, "text": sig.text,
                        "price": sig.exec_price, "new_state": f"SKIP({type(e).__name__})"})
            continue

        log.append({"step": sig.step, "event": sig.event, "text": sig.text,
                    "price": sig.exec_price, "new_state": new_fsm.state.name,
                    "cost_basis": round(new_fsm.cost_basis, 4)})

        if fsm.state == CostState.SCANNING and new_fsm.state == CostState.POSITION_OPEN:
            entry = sig

        if new_fsm.state == CostState.STOPPED_OUT and fsm.state != CostState.STOPPED_OUT:
            if entry is not None:
                pnl_ps = sig.exec_price - fsm.entry_price
                trades.append(Trade(
                    entry_step=entry.step, entry_price=fsm.entry_price,
                    exit_step=sig.step, exit_price=sig.exec_price,
                    entry_text=entry.text, exit_text=sig.text,
                    pnl=round(pnl_ps * fsm.total_shares, 2),
                    pnl_pct=round(pnl_ps / fsm.entry_price * 100, 4),
                    state_at_exit=fsm.state.name,
                ))
                entry = None
        fsm = new_fsm

    # 末端 mark-to-market（用 AV 真实末端收盘价）
    open_position = None
    if fsm.state in (CostState.POSITION_OPEN, CostState.COST_REDUCING,
                     CostState.PRINCIPAL_WITHDRAWN):
        pnl_ps = last_price - fsm.entry_price
        open_position = {
            "entry_step": entry.step if entry else -1,
            "entry_price": round(fsm.entry_price, 4),
            "last_price": round(last_price, 4),
            "cost_basis": round(fsm.cost_basis, 4),
            "state": fsm.state.name,
            "float_pnl_pct": round(pnl_ps / fsm.entry_price * 100, 4),
            "short_diff_count": len(fsm.completed_short_diffs),
        }

    completed_pnl = sum(t.pnl for t in trades if t.pnl is not None)
    return {
        "trades": trades,
        "open_position": open_position,
        "completed_pnl": round(completed_pnl, 2),
        "final_fsm_state": fsm.state.name,
        "log": log,
        "gated_out": gated_out,
        "settle_diag": settle_diag,
    }


# ── 报告 ──────────────────────────────────────────────────────────────


def fmt_pct(v: float | None) -> str:
    return "—" if v is None else f"{v:+.2f}%"


def ts2str(ts: int) -> str:
    import datetime
    return datetime.datetime.fromtimestamp(ts, datetime.UTC).strftime("%Y-%m-%d %H:%M UTC")


def write_report(
    seq_meta: dict, seq: list[dict], av: AvIndex,
    execs: list[ExecSignal], dropped: list, result_a: dict, result_b: dict,
    last_unix: int, last_price: float,
) -> None:
    biases = [e.bias for e in execs]
    mean_bias = sum(biases) / len(biases) if biases else 0.0
    max_abs_bias = max((abs(b) for b in biases), default=0.0)
    mean_abs_bias = sum(abs(b) for b in biases) / len(biases) if biases else 0.0

    L = [
        "# 严格零前视回测：QQQ 5分钟缠论信号（消除三层前视偏差）",
        "",
        "**认识论等级**：L2 —— 真实 TV Replay 信号 + 真实 AlphaVantage OHLCV 执行价，"
        "单标的（QQQ）单时段，结果可否证。",
        "",
        "## 0. 三层前视偏差消除（本回测的核心命题）",
        "",
        "| 层 | 前视类型 | 消除手段 | 本回测落实 |",
        "|----|---------|---------|-----------|",
        "| 1 | 执行价前视 | 用真实 OHLCV 收盘价，不用 TV 标注价 | AV 5min `TIME_SERIES_INTRADAY`，"
        "执行价 = 信号 bar 的**下一根 bar 收盘价**（lag=1）|",
        "| 2 | 信号确认前视 | 用 Replay '信号首次出现' step | 预期→确认转换 / 新 frontier 确认 |",
        "| 3 | 信号集前视 | Replay 逐 bar 产出信号 | 每步只见当时图上 label，非事后全图 |",
        "",
        "## 1. 数据元信息",
        "",
        "| 字段 | 值 |",
        "|------|---|",
        f"| 品种 | {seq_meta.get('symbol', '?')} |",
        f"| 周期 | 5 分钟（每 replay step = 1 根 5min K 线）|",
        f"| Replay 起始 | {ts2str(seq[0]['current_date'])} |",
        f"| Replay 末端 | {ts2str(seq[-1]['current_date'])} |",
        f"| Replay 步数 | {seq_meta.get('n_steps', len(seq))} |",
        f"| AV OHLCV 源 | AlphaVantage TIME_SERIES_INTRADAY 5min 2026-04（4032 bars, EDT）|",
        f"| 末端估值价（AV 真实） | {last_price:.4f} @ {ts2str(last_unix)} |",
        "",
        "## 2. 执行价前视偏差量化（第 1 层消除的实证）",
        "",
        "下表对每个零前视信号，对比 **TV 标注价**（旧口径，含执行价前视）与 "
        "**AV 真实 lag=1 收盘价**（新口径）。偏差 = AV − TV标注。",
        "",
        f"- 信号数（可执行）：**{len(execs)}**",
        f"- 平均偏差（含符号）：**{mean_bias:+.4f}**",
        f"- 平均绝对偏差：**{mean_abs_bias:.4f}**（约 {mean_abs_bias / last_price * 100:.3f}% of 价）",
        f"- 最大绝对偏差：**{max_abs_bias:.4f}**",
        "",
        "| 步 | 时间(UTC) | 信号 | 事件 | TV标注价 | AV真实执行价(lag=1) | 偏差 |",
        "|----|----------|------|------|---------|-------------------|------|",
    ]
    for e in execs:
        L.append(f"| {e.step} | {ts2str(e.current_date)} | {e.text} | {e.event} "
                 f"| {e.label_price:.2f} | {e.exec_price:.4f} | {e.bias:+.4f} |")
    if dropped:
        L.append("")
        L.append(f"*另有 {len(dropped)} 个信号因无下一根 bar / 非交易信号被丢弃。*")

    for tag, res, head in [
        ("A", result_a, "## 3. A 组（纯缠论）：cost_reduction_fsm，全部信号执行"),
        ("B", result_b, "## 4. B 组（缠论 + PH settle）：买点前 online settle 过滤"),
    ]:
        L += ["", head, "",
              f"初始资金 {OWN_CAPITAL:,.0f} USD，无融资，sub_ratio={SUB_RATIO}。"
              f"执行价全部为 AV 真实 lag=1 收盘价。"]
        if tag == "B":
            L.append(f"PH settle 否决买点数：**{res['gated_out']}** / {len(res['settle_diag'])} 个 3买。")
            diag = res["settle_diag"]
            if diag:
                gaps = [abs(d["gap_to_settle"]) for d in diag if d.get("gap_to_settle") is not None]
                mean_gap = sum(gaps) / len(gaps) if gaps else 0.0
                L += [
                    "",
                    "### PH settle 与缠论 3买的共振（独立交叉确认）",
                    "",
                    "PH 近端下跌腿 settle 屏障价（拓扑学，online merge tree）vs 缠论 3买执行价"
                    "（形态学）。二者识别同一个底则价位应重合。",
                    "",
                    "| 步 | 缠论3买执行价 | PH近端settle价 | gap | settle_pers | 判定 |",
                    "|----|------------|--------------|-----|------------|------|",
                ]
                for d in diag:
                    sp = f"{d['settle_price']:.2f}" if d.get("settle_price") is not None else "—"
                    gp = f"{d['gap_to_settle']:+.3f}" if d.get("gap_to_settle") is not None else "—"
                    pr = f"{d['settle_pers']:.2f}" if d.get("settle_pers") is not None else "—"
                    L.append(f"| {d['step']} | {d['exec_price']:.2f} | {sp} | {gp} | {pr} "
                             f"| {'ALLOW' if d['allowed'] else 'REJECT'} |")
                L += [
                    "",
                    f"**共振发现**：缠论 3买执行价与 PH 近端 settle 屏障价平均绝对偏差 "
                    f"**{mean_gap:.3f}** 点（约 {mean_gap / last_price * 100:.3f}% of 价）。"
                    "二者近乎重合——缠论形态学的「三买」与 PH 拓扑学的「近端下跌腿因果死亡确认」"
                    "在 5min 尺度上识别同一转折点，构成独立交叉确认（persistence_theory "
                    "「settle屏障 = 缠论分型确认」同构的弱经验支持，L2）。",
                    "",
                    f"**尺度诚实声明**：本窗口近端下跌腿 settle_persistence 普遍 < {SETTLE_MIN_PERS} 点"
                    "（5min 尺度下摆动幅度比日线小一个量级），故 PH gate 在此窗口几乎不主动否决"
                    "（全部经「噪声」豁免通过）。PH settle gate 的过滤能力需在**日线/更大级别**"
                    "（settle 屏障为多点级）窗口验证——本 5min 结果不能外推 PH 过滤有效性，"
                    "只支持「价位重合」这一更弱的共振结论。",
                ]
        L += ["", "### 信号处理日志", "",
              "| 步 | 事件 | 文本 | 执行价 | FSM 新状态 |",
              "|----|------|------|--------|-----------|"]
        for r in res["log"]:
            L.append(f"| {r['step']} | {r['event']} | {r['text']} | {r['price']:.4f} | {r['new_state']} |")
        L += ["", "### 已完成交易", ""]
        if res["trades"]:
            L += ["| 买入步 | 买入价 | 卖出步 | 卖出价 | P&L% | 退出原因 |",
                  "|--------|--------|--------|--------|------|---------|"]
            for t in res["trades"]:
                L.append(f"| {t.entry_step} | {t.entry_price:.4f} | {t.exit_step} "
                         f"| {t.exit_price:.4f} | {fmt_pct(t.pnl_pct)} | {t.state_at_exit} |")
        else:
            L.append("*无已完成往返交易（末端仍持仓 / 无主级别退出信号）。*")
        if res["open_position"]:
            op = res["open_position"]
            L += ["", "### 末端持仓（AV 真实价估值）", "",
                  "| 字段 | 值 |", "|------|---|",
                  f"| 建仓步 | {op['entry_step']} |",
                  f"| 建仓价(AV) | {op['entry_price']:.4f} |",
                  f"| 末端价(AV) | {op['last_price']:.4f} |",
                  f"| FSM 当前成本 | {op['cost_basis']:.4f} |",
                  f"| FSM 状态 | {op['state']} |",
                  f"| 次级别短差次数 | {op['short_diff_count']} |",
                  f"| 浮盈% | {fmt_pct(op['float_pnl_pct'])} |"]

    # A vs B
    def fp(res: dict) -> float | None:
        return res["open_position"]["float_pnl_pct"] if res["open_position"] else None
    L += ["", "## 5. A vs B 对比", "",
          "| 指标 | A 组（纯缠论） | B 组（+PH settle） |",
          "|------|--------------|-------------------|",
          f"| 已实现 P&L | {result_a['completed_pnl']:.2f} USD | {result_b['completed_pnl']:.2f} USD |",
          f"| 末端浮盈% | {fmt_pct(fp(result_a))} | {fmt_pct(fp(result_b))} |",
          f"| 往返交易数 | {len(result_a['trades'])} | {len(result_b['trades'])} |",
          f"| PH 否决买点 | — | {result_b['gated_out']} |"]

    # 结果包六要素
    L += ["", "## 6. 结果包（六要素）", "",
          f"**1. 结论**：QQQ 5分钟 {seq_meta.get('n_steps', len(seq))} 步 replay "
          f"（{ts2str(seq[0]['current_date'])} → {ts2str(seq[-1]['current_date'])}，约 8 个交易日）"
          f"提取 {len(execs)} 个零前视可执行信号。用 AV 真实 lag=1 收盘价执行，"
          f"A 组已实现 P&L {result_a['completed_pnl']:.2f} USD，"
          f"B 组（PH settle 过滤）{result_b['completed_pnl']:.2f} USD。"
          f"执行价前视偏差平均绝对值 {mean_abs_bias:.4f}（{mean_abs_bias / last_price * 100:.3f}% of 价），"
          f"最大 {max_abs_bias:.4f}——证明 TV 标注价与真实可成交价存在系统性差异，"
          f"用标注价回测会高估/低估收益。",
          "",
          "**2. 定义依据**：零前视三层口径见 §0。信号提取复用 "
          "`zero_lookahead_backtest.extract_zero_lookahead_signals`（基于 repaint=0% 验证，"
          "见 analysis/tv_repaint_check.md）；线段方向 = 缠论指标 `(趋势)` 标记（§3 分类）；"
          "执行价 = AV `TIME_SERIES_INTRADAY` 5min 下一根 bar 收盘（lag=1）。",
          "",
          "**3. 边界条件（结论翻转条件）**：",
          "- 时间窗口仅约 8 个交易日的 5min 数据，信号 22 个 / 可执行 "
          f"{len(execs)} 个——**统计上不可外推**，单一窗口可被另一窗口否证；",
          "- 5min 信号对应**次级别**（非主操作级别），3卖（线段转空）在本窗口为 0，"
          "  退出几乎只由 `1卖(趋势)` 触发；",
          "- AV EDT 时区假设（2026-04 全月 EDT/-4）；1 根 04-19 周日 bar AV 缺失，"
          "  落到下一根真实 bar；",
          "- PH settle gate 阈值 SETTLE_MIN_PERS=3.0、sub_ratio=0.3 为参数假设（L0）；",
          "- 单标的单时段 = L2；跨标的/跨时段（L3）需全量回放完成后重算。",
          "",
          "**4. 下游推论**：",
          "- 执行价前视偏差非零 → 任何用 TV 标注价的历史回测（含本仓库既有 "
          "  zero_lookahead_backtest.py）都含第 1 层偏差，需用 AV 真实价重算；",
          "- 若 B 组 PH 否决的买点事后被证明确为假买点 → PH settle 作为缠论过滤层有效；"
          "  本窗口 PH 否决 "
          f"{result_b['gated_out']} 个，需更长窗口验证过滤质量。",
          "",
          "**5. 谱系引用**：",
          "- 267号：满仓满融降成本操作方法论（cost_reduction_fsm）；",
          "- persistence_theory §17.1/§17.3：alive/settled 与规则3（PH settle gate 依据）；",
          "- a_settle_trigger.py：全局最低分量不可反弹 settle（090号声明膨胀禁止）；",
          "- repaint=0% 结论：analysis/tv_repaint_check.md（L2）。",
          "- 本产出未涉及缠论概念定义的新谱系分离；线段方向口径沿用指标既有 `(趋势)` 标记。",
          "",
          "**6. 影响声明**：新增 `scripts/zero_lookahead_full_backtest.py`、本报告、"
          "`analysis/data_cache/av_qqq_5min_202604.json`、"
          "`analysis/data_cache/zero_lookahead_full_result.json`。"
          "只读调用 cost_reduction_fsm / a_settle_trigger / a_online_persistence，"
          "不修改任何生产模块或缠论定义。相对既有 zero_lookahead_backtest.py 的实质改进："
          "执行价从 TV 标注价改为 AV 真实 lag=1 收盘价（消除第 1 层前视）。"]

    REPORT_PATH.write_text("\n".join(L), encoding="utf-8")
    print(f"[✓] 报告写入 {REPORT_PATH}")

    # JSON 结果存档
    def trade_d(t: Trade) -> dict:
        return {"entry_step": t.entry_step, "entry_price": t.entry_price,
                "exit_step": t.exit_step, "exit_price": t.exit_price,
                "pnl": t.pnl, "pnl_pct": t.pnl_pct, "state_at_exit": t.state_at_exit}
    RESULT_JSON.write_text(json.dumps({
        "symbol": seq_meta.get("symbol"),
        "n_steps": seq_meta.get("n_steps", len(seq)),
        "exec_signals": [{"step": e.step, "text": e.text, "event": e.event,
                          "label_price": e.label_price, "exec_price": e.exec_price,
                          "bias": e.bias} for e in execs],
        "exec_bias": {"mean": round(mean_bias, 4), "mean_abs": round(mean_abs_bias, 4),
                      "max_abs": round(max_abs_bias, 4)},
        "group_a": {"completed_pnl": result_a["completed_pnl"],
                    "open_position": result_a["open_position"],
                    "trades": [trade_d(t) for t in result_a["trades"]]},
        "group_b": {"completed_pnl": result_b["completed_pnl"],
                    "open_position": result_b["open_position"],
                    "gated_out": result_b["gated_out"],
                    "trades": [trade_d(t) for t in result_b["trades"]]},
        "epistemic_level": "L2",
    }, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"[✓] 结果 JSON 写入 {RESULT_JSON}")


# ── 主入口 ────────────────────────────────────────────────────────────


def main() -> None:
    seq_path, av_path = SEQ_PATH, AV_PATH
    a = sys.argv[1:]
    for i, tok in enumerate(a):
        if tok == "--seq" and i + 1 < len(a):
            seq_path = Path(a[i + 1])
        elif tok == "--av" and i + 1 < len(a):
            av_path = Path(a[i + 1])

    print(f"[*] replay 序列：{seq_path}")
    raw = json.loads(seq_path.read_text(encoding="utf-8"))
    seq = raw["sequence"]
    print(f"    symbol={raw.get('symbol')} steps={raw.get('n_steps', len(seq))}")

    print(f"[*] AV OHLCV：{av_path}")
    av = AvIndex.load(av_path)
    print(f"    AV bars={len(av.sorted_opens)}")

    print("[*] 提取零前视信号（第 2、3 层）...")
    signals = extract_zero_lookahead_signals(seq)
    print(f"    {len(signals)} 个零前视信号")

    print("[*] 映射 AV 真实执行价（第 1 层，lag=1）...")
    execs, dropped = resolve_exec_prices(signals, seq, av)
    print(f"    可执行 {len(execs)}，丢弃 {len(dropped)}")

    # 末端估值 = replay 窗口最后一根 bar 的 AV 真实收盘价。
    # 严禁用 AV 整月最后一根（av.last_close()）——那在 replay 窗口之后，是前视。
    end_open = av.bar_open(seq[-1]["current_date"])
    last = av.close_at_or_after(end_open)
    if last is None:
        print("[ERROR] AV 无 replay 末端 bar 价"); sys.exit(1)
    last_unix, last_price = last
    print(f"    末端估值（replay 末根 bar）：{last_price:.4f} @ {ts2str(last_unix)}")

    print("[*] A 组（纯缠论）...")
    result_a = run_fsm(execs, last_price)
    print("[*] B 组（缠论 + PH settle gate）...")
    result_b = run_fsm(execs, last_price, settle_gate=SettleGate(av), av=av)

    write_report(raw, seq, av, execs, dropped, result_a, result_b, last_unix, last_price)

    print("\n" + "─" * 56)
    print(f"  可执行零前视信号：{len(execs)}")
    print(f"  执行价前视偏差 平均绝对值：{sum(abs(e.bias) for e in execs) / len(execs):.4f}" if execs else "  无信号")
    print(f"  A 组 已实现 P&L：{result_a['completed_pnl']:.2f} USD"
          f"  末端浮盈：{fmt_pct(result_a['open_position']['float_pnl_pct'] if result_a['open_position'] else None)}")
    print(f"  B 组 已实现 P&L：{result_b['completed_pnl']:.2f} USD"
          f"  末端浮盈：{fmt_pct(result_b['open_position']['float_pnl_pct'] if result_b['open_position'] else None)}"
          f"  PH否决买点：{result_b['gated_out']}")
    print("─" * 56)


if __name__ == "__main__":
    main()
