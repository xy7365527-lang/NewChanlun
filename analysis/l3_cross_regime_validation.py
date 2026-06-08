"""L3 多标的跨 regime 验证 — 笔中枢路径（525号）+ 双向操作 + PH 方向裁决。

认识论等级：L3（多标的 × 多 regime × 真实日线数据，可产生否定性结果）。

设计
====
4 标的 × 3 regime（bull/bear/sideways）× 2 对比组（A: candidate+PH / B: confirmed）。
**双向操作**：PH 裁决方向，上涨做多、下跌做空。

PH 方向裁决（零参数，纯拓扑事件驱动）
--------------------------------------
方向 = dominant alive component 被 settle 时翻转：
- buy_tree dominant settle（sublevel dominant 闭合）→ 翻空，只接受卖点做空
- sell_tree dominant settle（superlevel dominant 闭合）→ 翻多，只接受买点做多
- 非 dominant settle 不翻转方向（仅做降成本）
- 方向在两次 dominant settle 之间维持

退出逻辑分层（267号操作方法论）
-------------------------------
- 方向翻转（dominant settle）→ 全平
- 主级别反向信号（chanlun sell/buy）→ 全平
- 反向 invalidate → 止损全平
- 非 dominant settle → 降成本短差（不平主仓）

标的
----
SPY（大盘股）、GLD（黄金）、TLT（长期国债）、QQQ（科技股）。

谱系：525号（笔中枢）、candidate-fix、PH settle、267号（降成本）。
"""

from __future__ import annotations

import json
import os
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from dotenv import load_dotenv

load_dotenv(ROOT / ".env.local")
load_dotenv(ROOT / ".env")

from newchan.a_buysellpoint_v1 import buysellpoints_from_level
from newchan.a_divergence_v1 import divergences_from_moves_v1
from newchan.a_move_v1 import moves_from_zhongshus
from newchan.a_online_persistence import OnlineMergeTree
from newchan.a_zhongshu_v1 import zhongshu_from_strokes
from newchan.bi_engine import BiEngine
from newchan.core.recursion.buysellpoint_state import diff_buysellpoints
from newchan.types import Bar

CACHE_DIR = ROOT / "analysis" / "data_cache"

SYMBOLS = ["SPY", "GLD", "TLT", "QQQ"]
LEVEL_ID = 1
SUB_TRIM_FRAC = 0.3
PENDING_EXPIRY = 10


# ════════════════════════════════════════════════════════════
# 数据获取 + 缓存
# ════════════════════════════════════════════════════════════


def _fetch_daily_av(symbol: str) -> list[Bar]:
    """从 AV 获取日线数据（full），带限流。"""
    api_key = os.getenv("ALPHA_VANTAGE_API_KEY") or os.getenv("ALPHAVANTAGE_API_KEY", "")
    if not api_key:
        raise RuntimeError("ALPHA_VANTAGE_API_KEY 未配置（.env.local 或环境变量）")

    import requests
    time.sleep(13)
    params = {
        "function": "TIME_SERIES_DAILY",
        "symbol": symbol,
        "outputsize": "full",
        "datatype": "json",
        "apikey": api_key,
    }
    resp = requests.get("https://www.alphavantage.co/query", params=params, timeout=30)
    resp.raise_for_status()
    data = resp.json()
    if "Error Message" in data:
        raise RuntimeError(f"AV error for {symbol}: {data['Error Message']}")
    if "Information" in data:
        raise RuntimeError(f"AV rate limit for {symbol}: {data['Information']}")

    series = data.get("Time Series (Daily)", {})
    bars: list[Bar] = []
    for ts_str, values in sorted(series.items()):
        bars.append(Bar(
            ts=datetime.strptime(ts_str, "%Y-%m-%d"),
            open=float(values["1. open"]),
            high=float(values["2. high"]),
            low=float(values["3. low"]),
            close=float(values["4. close"]),
            volume=float(values["5. volume"]),
        ))
    return bars


def load_daily_bars(symbol: str, force: bool = False) -> list[dict]:
    """加载日线数据（缓存优先）。"""
    cache_path = CACHE_DIR / f"l3_{symbol}_daily.json"
    if cache_path.exists() and not force:
        raw = json.loads(cache_path.read_text())
        print(f"  缓存命中：{cache_path.name} ({len(raw)} bars)")
        return raw

    print(f"  从 AV 获取 {symbol} 日线数据（full）...")
    bars = _fetch_daily_av(symbol)
    raw = [
        {"ts": b.ts.isoformat(), "open": b.open, "high": b.high,
         "low": b.low, "close": b.close, "volume": b.volume}
        for b in bars
    ]
    cache_path.write_text(json.dumps(raw))
    print(f"  已缓存：{cache_path.name} ({len(raw)} bars)")
    return raw


# ════════════════════════════════════════════════════════════
# Regime 分割（SMA200 + 趋势方向）
# ════════════════════════════════════════════════════════════


def classify_regimes(
    closes: list[float], dates: list[str],
) -> list[dict]:
    """将日线序列按 SMA200 + 近期趋势分割成 regime 区间。

    bull: close > SMA200 且 SMA200 上升
    bear: close < SMA200 且 SMA200 下降
    sideways: 其余
    """
    n = len(closes)
    sma_window = 200
    if n < sma_window + 50:
        return [{"regime": "all", "start": 0, "end": n - 1,
                 "start_date": dates[0], "end_date": dates[-1]}]

    sma = [0.0] * n
    running = sum(closes[:sma_window])
    for i in range(sma_window, n):
        sma[i] = running / sma_window
        running += closes[i] - closes[i - sma_window]

    labels = [""] * n
    for i in range(sma_window + 20, n):
        sma_slope = sma[i] - sma[i - 20]
        if closes[i] > sma[i] and sma_slope > 0:
            labels[i] = "bull"
        elif closes[i] < sma[i] and sma_slope < 0:
            labels[i] = "bear"
        else:
            labels[i] = "sideways"

    regimes: list[dict] = []
    cur_label = labels[sma_window + 20]
    cur_start = sma_window + 20
    for i in range(sma_window + 21, n):
        if labels[i] != cur_label and labels[i] != "":
            if i - cur_start >= 40:
                regimes.append({
                    "regime": cur_label, "start": cur_start, "end": i - 1,
                    "start_date": dates[cur_start], "end_date": dates[i - 1],
                })
            cur_label = labels[i]
            cur_start = i
    if n - cur_start >= 40:
        regimes.append({
            "regime": cur_label, "start": cur_start, "end": n - 1,
            "start_date": dates[cur_start], "end_date": dates[-1],
        })
    return regimes


# ════════════════════════════════════════════════════════════
# 阶段1：chanlun 笔级别事件流
# ════════════════════════════════════════════════════════════


def generate_chanlun_events(bars_raw: list[dict]) -> dict:
    """streaming 逐 bar 生成笔级别买卖点事件流（零前视）。"""
    n = len(bars_raw)
    bi = BiEngine()
    prev_bsps: list = []
    event_seq = 0
    events: list[dict] = []
    closes: list[float] = []
    dates: list[str] = []

    t0 = time.time()
    for i, b in enumerate(bars_raw):
        ts = datetime.fromisoformat(b["ts"])
        bar = Bar(ts=ts, open=b["open"], high=b["high"],
                  low=b["low"], close=b["close"], volume=b.get("volume"))
        bi_snap = bi.process_bar(bar)
        strokes = bi_snap.strokes
        closes.append(b["close"])
        dates.append(b["ts"])

        zhongshus = zhongshu_from_strokes(strokes)
        moves = moves_from_zhongshus(zhongshus, num_segments=len(strokes))
        divs = divergences_from_moves_v1(strokes, zhongshus, moves, LEVEL_ID)
        curr_bsps = buysellpoints_from_level(strokes, zhongshus, moves, divs, LEVEL_ID)

        evs = diff_buysellpoints(
            prev_bsps, curr_bsps, bar_idx=i, bar_ts=b["close"], seq_start=event_seq,
        )
        event_seq += len(evs)
        for e in evs:
            name = type(e).__name__
            if "Candidate" in name:
                etype = "candidate"
            elif "Confirm" in name:
                etype = "confirm"
            elif "Invalidate" in name:
                etype = "invalidate"
            else:
                continue
            events.append({
                "bar_idx": i,
                "type": etype,
                "kind": getattr(e, "kind", ""),
                "side": getattr(e, "side", ""),
                "price": float(getattr(e, "price", 0.0) or 0.0),
                "bsp_id": int(getattr(e, "bsp_id", 0)),
            })
        prev_bsps = curr_bsps

        if (i + 1) % 1000 == 0:
            elapsed = time.time() - t0
            print(f"    [{i+1}/{n}] {elapsed:.0f}s  strokes={len(strokes)} "
                  f"zs={len(zhongshus)} events={len(events)}")

    confirmed_strokes = [s for s in bi.current_strokes if s.confirmed]
    amplitudes = sorted([s.high - s.low for s in confirmed_strokes])
    tau = amplitudes[len(amplitudes) // 2] if amplitudes else 1.0

    return {
        "n_bars": n,
        "gen_seconds": round(time.time() - t0, 1),
        "closes": closes,
        "dates": dates,
        "events": events,
        "n_strokes": len(bi.current_strokes),
        "n_zhongshu": len(zhongshu_from_strokes(bi.current_strokes)),
        "tau": tau,
    }


def load_or_generate(symbol: str, bars_raw: list[dict], force: bool) -> dict:
    cache = CACHE_DIR / f"l3_{symbol}_events.json"
    if cache.exists() and not force:
        data = json.loads(cache.read_text())
        print(f"  事件缓存命中：{cache.name} (events={len(data['events'])})")
        return data
    print(f"  生成 {symbol} chanlun 事件流（笔中枢路径）...")
    data = generate_chanlun_events(bars_raw)
    cache.write_text(json.dumps(data))
    print(f"  已缓存：{cache.name} (events={len(data['events'])}, {data['gen_seconds']}s)")
    return data


# ════════════════════════════════════════════════════════════
# ATR + PH Gate
# ════════════════════════════════════════════════════════════



@dataclass
class PHGate:
    """PH settle 门控 + dominant settle 方向裁决。

    buy_tree（喂 close）追踪底（valleys），sell_tree（喂 -close）追踪顶（peaks）。
    dominant = max_alive_persistence 对应的 alive component。
    dominant settle = 该 component 被合并死亡 = 最大结构闭合。
    """

    buy_tree: OnlineMergeTree = field(
        default_factory=lambda: OnlineMergeTree(track_dominant=False))
    sell_tree: OnlineMergeTree = field(
        default_factory=lambda: OnlineMergeTree(track_dominant=False))

    @staticmethod
    def _dominant_birth(tree: OnlineMergeTree) -> int | None:
        if not tree._stack:
            return None
        return min(tree._stack, key=lambda c: c.val).idx

    def step(self, close: float) -> tuple[bool, bool, bool, bool]:
        """返回 (buy_any, sell_any, buy_dom_settled, sell_dom_settled)。

        buy_dom_settled: sublevel dominant settle（最大下跌结构闭合）→ 翻空
        sell_dom_settled: superlevel dominant settle（最大上涨结构闭合）→ 翻多
        """
        buy_dom_idx = self._dominant_birth(self.buy_tree)
        sell_dom_idx = self._dominant_birth(self.sell_tree)

        buy_bars = self.buy_tree.update(close)
        sell_bars = self.sell_tree.update(-close)

        buy_dom_settled = (
            buy_dom_idx is not None
            and any(mb.birth_idx == buy_dom_idx for mb in buy_bars)
        )
        sell_dom_settled = (
            sell_dom_idx is not None
            and any(mb.birth_idx == sell_dom_idx for mb in sell_bars)
        )

        return bool(buy_bars), bool(sell_bars), buy_dom_settled, sell_dom_settled


# ════════════════════════════════════════════════════════════
# 阶段2：交易模拟（双向操作 + PH 方向裁决 + 分层退出）
# ════════════════════════════════════════════════════════════


@dataclass(frozen=True)
class Trade:
    entry_bar: int
    entry_price: float
    exit_bar: int
    exit_price: float
    reason: str
    pnl_pct: float
    holding_bars: int
    cost_reduction_pct: float = 0.0
    direction: str = "long"


def _index_events(events: list[dict]) -> dict[int, list[dict]]:
    by_bar: dict[int, list[dict]] = {}
    for e in events:
        by_bar.setdefault(e["bar_idx"], []).append(e)
    return by_bar


def _close_position(
    pos_dir: str, entry_bar: int, entry_price: float,
    exit_bar: int, exit_price: float, reason: str, cost_reduction: float,
) -> Trade:
    if pos_dir == "long":
        pnl = (exit_price - entry_price) / entry_price * 100 + cost_reduction
    else:
        pnl = (entry_price - exit_price) / entry_price * 100 + cost_reduction
    return Trade(
        entry_bar, entry_price, exit_bar, exit_price, reason,
        round(pnl, 3), exit_bar - entry_bar, round(cost_reduction, 3), pos_dir,
    )


def run_group(
    closes: list[float],
    events_by_bar: dict[int, list[dict]],
    start: int,
    end: int,
    *,
    entry_on: str,
    use_ph_gate: bool,
) -> list[Trade]:
    """单组回测主循环 — 双向操作 + PH dominant settle 方向裁决 + 分层退出。

    PH 方向（零参数，纯拓扑事件驱动）：
    - buy_tree dominant settle（sublevel dominant 闭合）→ direction = "short"
    - sell_tree dominant settle（superlevel dominant 闭合）→ direction = "long"
    - 非 dominant settle 不翻转方向（仅做降成本）
    - 方向在两次 dominant settle 之间维持

    退出分层（267号）：
    - 方向翻转（dominant settle）→ 全平
    - 主级别反向 chanlun 信号 → 全平
    - 买/卖点 invalidate → 止损全平
    - 非 dominant settle → 降成本短差（不平主仓）
    """
    ph = PHGate()
    trades: list[Trade] = []

    direction: str | None = None
    in_pos = False
    pos_dir: str = ""
    entry_bar = 0
    entry_price = 0.0
    entry_bsp_ids: set[int] = set()
    pending: dict | None = None
    cost_reduction = 0.0
    sub_open: float | None = None

    for i in range(start, end + 1):
        buy_any, sell_any, buy_dom_settled, sell_dom_settled = ph.step(closes[i])
        bar_events = events_by_bar.get(i, [])

        # ── PH direction: dominant settle flips ──
        old_dir = direction
        if buy_dom_settled:
            direction = "short"
        if sell_dom_settled:
            direction = "long"
        if direction != old_dir and not in_pos:
            pending = None

        # ── Entry gates: any settle event (零参数) ──
        buy_ph_ok = buy_any
        sell_ph_ok = sell_any

        # ── Cost reduction: any non-dominant settle ──
        sell_cr = sell_any and not sell_dom_settled
        buy_cr = buy_any and not buy_dom_settled

        # ── Position management ──
        if in_pos:
            # Direction flip → close position
            if direction is not None and direction != pos_dir:
                trades.append(_close_position(
                    pos_dir, entry_bar, entry_price, i, closes[i],
                    "direction_flip", cost_reduction,
                ))
                in_pos = False
                entry_bsp_ids = set()
                cost_reduction = 0.0
                sub_open = None
                pending = None
                continue

            if pos_dir == "long":
                invalidated = any(
                    e["type"] == "invalidate" and e["side"] == "buy"
                    and e["bsp_id"] in entry_bsp_ids
                    for e in bar_events
                )
                main_sell = any(
                    e["type"] in ("candidate", "confirm") and e["side"] == "sell"
                    for e in bar_events
                )
                # Cost reduction: sell high (sell_cr), buy low (buy_cr)
                if sell_cr and sub_open is None:
                    sub_open = closes[i]
                if sub_open is not None and buy_cr:
                    gain = (sub_open - closes[i]) / entry_price * 100 * SUB_TRIM_FRAC
                    if gain > 0:
                        cost_reduction += gain
                    sub_open = None

                if (invalidated or main_sell) and i > entry_bar:
                    trades.append(_close_position(
                        "long", entry_bar, entry_price, i, closes[i],
                        "stop_invalidate" if invalidated else "main_sell",
                        cost_reduction,
                    ))
                    in_pos = False
                    entry_bsp_ids = set()
                    cost_reduction = 0.0
                    sub_open = None
                    pending = None

            elif pos_dir == "short":
                invalidated = any(
                    e["type"] == "invalidate" and e["side"] == "sell"
                    and e["bsp_id"] in entry_bsp_ids
                    for e in bar_events
                )
                main_buy = any(
                    e["type"] in ("candidate", "confirm") and e["side"] == "buy"
                    for e in bar_events
                )
                # Cost reduction for shorts: buy low (buy_cr), sell high (sell_cr)
                if buy_cr and sub_open is None:
                    sub_open = closes[i]
                if sub_open is not None and sell_cr:
                    gain = (closes[i] - sub_open) / entry_price * 100 * SUB_TRIM_FRAC
                    if gain > 0:
                        cost_reduction += gain
                    sub_open = None

                if (invalidated or main_buy) and i > entry_bar:
                    trades.append(_close_position(
                        "short", entry_bar, entry_price, i, closes[i],
                        "stop_invalidate" if invalidated else "main_buy",
                        cost_reduction,
                    ))
                    in_pos = False
                    entry_bsp_ids = set()
                    cost_reduction = 0.0
                    sub_open = None
                    pending = None

            continue

        # ── No position: entry logic ──
        if direction == "long":
            triggers = [e for e in bar_events
                        if e["type"] == entry_on and e["side"] == "buy"]
            if entry_on == "candidate":
                if pending is not None:
                    inv = any(
                        e["type"] == "invalidate" and e["side"] == "buy"
                        and e["bsp_id"] in pending["bsp_ids"]
                        for e in bar_events
                    )
                    if inv or (i - pending["bar"]) > PENDING_EXPIRY:
                        pending = None
                if triggers:
                    pending = {"bar": i, "bsp_ids": {e["bsp_id"] for e in triggers}}
                if pending is not None and (buy_ph_ok if use_ph_gate else True):
                    in_pos = True
                    pos_dir = "long"
                    entry_bar = i
                    entry_price = closes[i]
                    entry_bsp_ids = set(pending["bsp_ids"])
                    pending = None
            else:
                if triggers:
                    in_pos = True
                    pos_dir = "long"
                    entry_bar = i
                    entry_price = closes[i]
                    entry_bsp_ids = {e["bsp_id"] for e in triggers}

        elif direction == "short":
            triggers = [e for e in bar_events
                        if e["type"] == entry_on and e["side"] == "sell"]
            if entry_on == "candidate":
                if pending is not None:
                    inv = any(
                        e["type"] == "invalidate" and e["side"] == "sell"
                        and e["bsp_id"] in pending["bsp_ids"]
                        for e in bar_events
                    )
                    if inv or (i - pending["bar"]) > PENDING_EXPIRY:
                        pending = None
                if triggers:
                    pending = {"bar": i, "bsp_ids": {e["bsp_id"] for e in triggers}}
                if pending is not None and (sell_ph_ok if use_ph_gate else True):
                    in_pos = True
                    pos_dir = "short"
                    entry_bar = i
                    entry_price = closes[i]
                    entry_bsp_ids = set(pending["bsp_ids"])
                    pending = None
            else:
                if triggers:
                    in_pos = True
                    pos_dir = "short"
                    entry_bar = i
                    entry_price = closes[i]
                    entry_bsp_ids = {e["bsp_id"] for e in triggers}

    if in_pos:
        trades.append(_close_position(
            pos_dir, entry_bar, entry_price, end, closes[end],
            "eod_close", cost_reduction,
        ))
    return trades


# ════════════════════════════════════════════════════════════
# 指标
# ════════════════════════════════════════════════════════════


def metrics(trades: list[Trade], closes: list[float], start: int, end: int) -> dict:
    bh = (closes[end] - closes[start]) / closes[start] * 100 if closes[start] > 0 else 0.0
    if not trades:
        return {"n": 0, "win_rate": 0.0, "avg": 0.0, "compound": 0.0,
                "max_dd": 0.0, "avg_hold": 0.0, "avg_cr": 0.0, "buy_hold": bh,
                "n_long": 0, "n_short": 0, "long_wr": 0.0, "short_wr": 0.0}
    wins = sum(1 for t in trades if t.pnl_pct > 0)
    eq = 1.0
    peak = 1.0
    max_dd = 0.0
    for t in trades:
        eq *= (1 + t.pnl_pct / 100)
        peak = max(peak, eq)
        max_dd = min(max_dd, (eq - peak) / peak)
    longs = [t for t in trades if t.direction == "long"]
    shorts = [t for t in trades if t.direction == "short"]
    long_wins = sum(1 for t in longs if t.pnl_pct > 0)
    short_wins = sum(1 for t in shorts if t.pnl_pct > 0)
    return {
        "n": len(trades),
        "win_rate": round(wins / len(trades) * 100, 1),
        "avg": round(sum(t.pnl_pct for t in trades) / len(trades), 3),
        "compound": round((eq - 1) * 100, 2),
        "max_dd": round(max_dd * 100, 2),
        "avg_hold": round(sum(t.holding_bars for t in trades) / len(trades), 1),
        "avg_cr": round(sum(t.cost_reduction_pct for t in trades) / len(trades), 3),
        "buy_hold": round(bh, 2),
        "n_long": len(longs),
        "n_short": len(shorts),
        "long_wr": round(long_wins / len(longs) * 100, 1) if longs else 0.0,
        "short_wr": round(short_wins / len(shorts) * 100, 1) if shorts else 0.0,
    }


# ════════════════════════════════════════════════════════════
# 报告
# ════════════════════════════════════════════════════════════


def write_report(all_results: dict, output: Path) -> None:
    L: list[str] = []
    L.append("# L3 多标的跨 regime 验证 — 笔中枢路径（525号）+ 双向操作\n")
    L.append(f"- 认识论等级：**L3**（多标的 × 多 regime × 真实日线数据）")
    L.append(f"- 标的：{', '.join(SYMBOLS)}")
    L.append(f"- 路径：zhongshu_from_strokes（笔中枢 525号）")
    L.append(f"- **双向操作**：PH dominant settle 方向裁决（零参数，纯拓扑事件驱动）")
    L.append(f"- 退出分层：dominant settle 翻转全平 / 主级别反向信号全平 / 非dominant settle 仅降成本")
    L.append(f"- 参数：**零参数方向**（dominant alive settle = 唯一方向翻转条件）"
             f" SUB_TRIM={SUB_TRIM_FRAC}\n")

    L.append("## 1. 全局汇总\n")
    L.append("| 标的 | Regime | 期间 | Bars | "
             "A n(L/S) | A 胜率 | A 复利% | A MaxDD% | "
             "B n(L/S) | B 胜率 | B 复利% | B MaxDD% | "
             "Buy&Hold% |")
    L.append("|" + "|".join(["---"] * 13) + "|")

    a_total_compound = 1.0
    b_total_compound = 1.0
    total_a_wins = 0
    total_a_n = 0
    total_b_wins = 0
    total_b_n = 0
    total_a_long = 0
    total_a_short = 0
    total_b_long = 0
    total_b_short = 0

    for sym in SYMBOLS:
        if sym not in all_results:
            continue
        for r in all_results[sym]:
            ma = r["A"]
            mb = r["B"]
            L.append(
                f"| {sym} | {r['regime']} | {r['start_date'][:10]}→{r['end_date'][:10]} | "
                f"{r['n_bars']} | "
                f"{ma['n']}({ma['n_long']}L/{ma['n_short']}S) | {ma['win_rate']}% | "
                f"{ma['compound']:+.2f} | {ma['max_dd']:.2f} | "
                f"{mb['n']}({mb['n_long']}L/{mb['n_short']}S) | {mb['win_rate']}% | "
                f"{mb['compound']:+.2f} | {mb['max_dd']:.2f} | "
                f"{ma['buy_hold']:+.2f} |"
            )
            if ma["n"] > 0:
                a_total_compound *= (1 + ma["compound"] / 100)
                total_a_wins += int(ma["n"] * ma["win_rate"] / 100)
                total_a_n += ma["n"]
                total_a_long += ma["n_long"]
                total_a_short += ma["n_short"]
            if mb["n"] > 0:
                b_total_compound *= (1 + mb["compound"] / 100)
                total_b_wins += int(mb["n"] * mb["win_rate"] / 100)
                total_b_n += mb["n"]
                total_b_long += mb["n_long"]
                total_b_short += mb["n_short"]

    L.append("")
    L.append("## 2. 聚合统计\n")
    a_wr = round(total_a_wins / total_a_n * 100, 1) if total_a_n else 0.0
    b_wr = round(total_b_wins / total_b_n * 100, 1) if total_b_n else 0.0
    L.append(f"| 组 | 总交易 | Long | Short | 总胜率 | 聚合复利% |")
    L.append("|---|-------|------|-------|-------|----------|")
    L.append(f"| A (candidate+PH) | {total_a_n} | {total_a_long} | {total_a_short} | "
             f"{a_wr}% | {(a_total_compound - 1) * 100:+.2f}% |")
    L.append(f"| B (confirmed) | {total_b_n} | {total_b_long} | {total_b_short} | "
             f"{b_wr}% | {(b_total_compound - 1) * 100:+.2f}% |")
    L.append("")

    L.append("## 3. Regime 对比分析\n")
    regime_agg: dict[str, dict[str, list]] = {}
    for sym in SYMBOLS:
        if sym not in all_results:
            continue
        for r in all_results[sym]:
            reg = r["regime"]
            if reg not in regime_agg:
                regime_agg[reg] = {"a_compounds": [], "b_compounds": [], "bh": []}
            regime_agg[reg]["a_compounds"].append(r["A"]["compound"])
            regime_agg[reg]["b_compounds"].append(r["B"]["compound"])
            regime_agg[reg]["bh"].append(r["A"]["buy_hold"])

    L.append("| Regime | A 均复利% | B 均复利% | BH 均% | A vs BH | B vs BH |")
    L.append("|--------|----------|----------|--------|---------|---------|")
    for reg in ("bull", "bear", "sideways"):
        if reg not in regime_agg:
            continue
        d = regime_agg[reg]
        a_avg = sum(d["a_compounds"]) / len(d["a_compounds"])
        b_avg = sum(d["b_compounds"]) / len(d["b_compounds"])
        bh_avg = sum(d["bh"]) / len(d["bh"])
        a_vs = "胜" if a_avg > bh_avg else "负"
        b_vs = "胜" if b_avg > bh_avg else "负"
        L.append(f"| {reg} | {a_avg:+.2f} | {b_avg:+.2f} | {bh_avg:+.2f} | {a_vs} | {b_vs} |")
    L.append("")

    L.append("## 4. 信号密度统计\n")
    for sym in SYMBOLS:
        if sym not in all_results or not all_results[sym]:
            continue
        r0 = all_results[sym][0]
        if "signal_stats" in r0:
            ss = r0["signal_stats"]
            L.append(f"### {sym}")
            L.append(f"- 笔数：{ss.get('n_strokes', '?')}")
            L.append(f"- 笔中枢数：{ss.get('n_zhongshu', '?')}")
            L.append(f"- candidate 买点：{ss.get('n_cand_buy', '?')}")
            L.append(f"- confirm 买点：{ss.get('n_conf_buy', '?')}")
            L.append(f"- candidate 卖点：{ss.get('n_cand_sell', '?')}")
            L.append(f"- invalidate：{ss.get('n_inv', '?')}\n")

    L.append("## 5. 结果包六要素\n")
    L.append("**结论**：L3 多标的跨 regime 验证（笔中枢路径 525号 + 双向操作 + PH 方向裁决 + 分层退出 267号）。")
    L.append(f"4 标的 × 多 regime 真实日线数据。"
             f"聚合复利：A={((a_total_compound-1)*100):+.2f}%, B={((b_total_compound-1)*100):+.2f}%。"
             f"方向分布：A({total_a_long}L/{total_a_short}S), B({total_b_long}L/{total_b_short}S)。\n")
    L.append("**定义依据**：525号（zhongshu_from_strokes 笔中枢退化基底）；"
             "candidate-fix（买卖点 candidate/confirmed 时间分离）；"
             "PH dominant settle = 方向裁决（零参数，纯拓扑事件）；"
             "267号（降成本 FSM 分层退出）。\n")
    L.append("**边界条件**：①dominant settle 频率依赖标的波动结构，低波标的翻转稀疏；"
             "②日线级别笔中枢密度依赖标的波动性，低波标的可能中枢不足；"
             "③dominant 定义 = max_alive_persistence 对应的 component，"
             "若多 component persistence 接近则 dominant 身份不稳定。\n")
    L.append("**下游推论**：（待数据产出后填写）。\n")
    L.append("**谱系引用**：525号、candidate-fix、267号、形式化有效域规则"
             "（L3 多标的交叉验证）。\n")
    L.append("**影响声明**：新增 analysis/l3_cross_regime_validation.py + 本报告；"
             "事件流缓存于 data_cache/l3_*。双向操作修正了只做多版本在下跌段的系统性偏差。\n")

    output.write_text("\n".join(L))
    print(f"\n报告已写入：{output}")


# ════════════════════════════════════════════════════════════
# main
# ════════════════════════════════════════════════════════════


def main() -> None:
    import argparse
    p = argparse.ArgumentParser(description="L3 多标的跨 regime 验证（笔中枢路径）")
    p.add_argument("--symbols", nargs="+", default=SYMBOLS)
    p.add_argument("--force-data", action="store_true", help="强制重新获取 AV 数据")
    p.add_argument("--force-events", action="store_true", help="强制重新生成事件流")
    p.add_argument("--output", type=str, default=str(ROOT / "analysis" / "l3_cross_regime_validation.md"))
    args = p.parse_args()

    all_results: dict[str, list] = {}

    for sym in args.symbols:
        print(f"\n{'='*60}")
        print(f"标的：{sym}")
        print(f"{'='*60}")

        bars_raw = load_daily_bars(sym, force=args.force_data)
        data = load_or_generate(sym, bars_raw, force=args.force_events)

        closes = data["closes"]
        dates = data["dates"]
        events_by_bar = _index_events(data["events"])

        regimes = classify_regimes(closes, dates)
        print(f"  Regime 分割：{len(regimes)} 区间")
        for r in regimes:
            print(f"    {r['regime']}: {r['start_date'][:10]} → {r['end_date'][:10]} "
                  f"({r['end'] - r['start'] + 1} bars)")

        all_trades_a = run_group(closes, events_by_bar, 0, len(closes) - 1,
                                  entry_on="candidate", use_ph_gate=True)
        all_trades_b = run_group(closes, events_by_bar, 0, len(closes) - 1,
                                  entry_on="confirm", use_ph_gate=False)
        print(f"  全时间线交易：A={len(all_trades_a)}笔 B={len(all_trades_b)}笔")

        sym_results: list[dict] = []
        for r in regimes:
            s, e = r["start"], r["end"]
            trades_a = [t for t in all_trades_a if s <= t.entry_bar <= e]
            trades_b = [t for t in all_trades_b if s <= t.entry_bar <= e]
            ma = metrics(trades_a, closes, s, e)
            mb = metrics(trades_b, closes, s, e)
            result = {
                "regime": r["regime"],
                "start_date": r["start_date"],
                "end_date": r["end_date"],
                "n_bars": e - s + 1,
                "A": ma,
                "B": mb,
            }
            if not sym_results:
                n_cand_buy = sum(1 for ev in data["events"]
                                if ev["type"] == "candidate" and ev["side"] == "buy")
                n_conf_buy = sum(1 for ev in data["events"]
                                if ev["type"] == "confirm" and ev["side"] == "buy")
                n_cand_sell = sum(1 for ev in data["events"]
                                 if ev["type"] == "candidate" and ev["side"] == "sell")
                n_inv = sum(1 for ev in data["events"] if ev["type"] == "invalidate")
                result["signal_stats"] = {
                    "n_strokes": data.get("n_strokes", 0),
                    "n_zhongshu": data.get("n_zhongshu", 0),
                    "n_cand_buy": n_cand_buy,
                    "n_conf_buy": n_conf_buy,
                    "n_cand_sell": n_cand_sell,
                    "n_inv": n_inv,
                }
            sym_results.append(result)

            print(f"\n  [{sym} / {r['regime']}] {r['start_date'][:10]}→{r['end_date'][:10]}")
            print(f"    A(cand+PH): n={ma['n']}(L{ma['n_long']}/S{ma['n_short']}) "
                  f"win={ma['win_rate']}% compound={ma['compound']:+.2f}% maxDD={ma['max_dd']:.2f}%")
            print(f"    B(confirm):  n={mb['n']}(L{mb['n_long']}/S{mb['n_short']}) "
                  f"win={mb['win_rate']}% compound={mb['compound']:+.2f}% maxDD={mb['max_dd']:.2f}%")
            print(f"    Buy&Hold:    {ma['buy_hold']:+.2f}%")

        all_results[sym] = sym_results

    write_report(all_results, Path(args.output))

    results_cache = CACHE_DIR / "l3_cross_regime_results.json"
    results_cache.write_text(json.dumps(all_results, indent=2))
    print(f"结果 JSON 已写入：{results_cache}")


if __name__ == "__main__":
    main()
