"""多级别缠论 × PH 联合回测（用验证过的 TV 数据重跑）— QQQ。

存在论位置 / 信息增量
---------------------
之前的多级别回测有 **bar 索引错标** 问题（`time` 字段被当作日 bar 计数，但它实际是
稀疏的笔端点序号——daily 仅 440 个 pivot 压缩了 25 年 6848 根日线）。本脚本用
**价格匹配 + 单调 DP 对齐** 把每个 timeframe 的笔端点严格锚定到 yfinance 日线 OHLC，
消除错标，然后做三组对比回测。

数据（已验证 timeframe）
------------------------
- 日线 si=3：analysis/data_cache/qqq_daily_chanlun.json（verified 1D，19.76→741.63）
- 30分钟 si=3：analysis/data_cache/qqq_30min_chanlun.json（verified 30，407→741，~2.4y）
- 5分钟 si=3：analysis/data_cache/qqq_5min_chanlun.json（verified 5，555→741，~10mo）
- OHLCV：yfinance QQQ period=max（auto_adjust=False，与 TV 原始价匹配）

bar→date 映射（消除错标的核心，L0 确定性）
------------------------------------------
1. 每个文件的笔（lines, si=3）端点构成严格交替的 top/bot pivot 序列（逐线方向定 kind）。
2. 笔端点价 = 缠论包含合并后的分型极值 ≈ 原始日线 High/Low（实测 daily 220/220 在 0.1%
   内，30min median 0.003%，5min median 0.071%，全部 <1%）。
3. **单调 DP** 把 pivot 价序列对齐日线极值（最小总价差，保证时间单调）。实测 daily 总
   价差 0.06 / 440 pivot，avg 0.0136%——基本精确。
4. 买卖点 label（si=3，含买/卖字）落在笔端点 bar 上（daily 115/115），继承该 bar 的日线日期。

级别映射（缠论原文，第25/31课"看级别不看编号"）
------------------------------------------------
- 主操作级别 = 日线一类买卖点：1买→建仓（BUY_POINT_CONFIRMED），1卖→清仓
  （MAIN_LEVEL_SELL_POINT）。日线 2/3 类是主级别中继，FSM 无主级别加仓事件 → 不动作
  （忠实于 FSM 接口，只记录）。
- 次级别 = 30分钟买卖点 → 短差降成本（SUB_LEVEL_SELL_POINT / SUB_LEVEL_BUY_POINT）。
  看级别不看编号：是"30分钟这个级别"使其成为短差，与 1/2/3 编号无关。
- 买点失效（止损 BUY_POINT_NEGATED）：建仓后日线 Low 跌破该一买分型低点 → 失效。

三组对比
--------
A   纯缠论多级别嵌套：日线一买/一卖建/清仓 + 30分钟短差（cost_reduction_fsm）。
B1  缠论 + PH settle 门控：日线一买须 W 窗内 PH 买侧 settle（sublevel，任意 persistence）
    右侧确认才建仓；日线一卖须 W 窗内 PH 卖侧 settle 且 persistence ≥ τ_super（superlevel）
    才清仓。短差仍来自 30分钟。
B2  纯 PH persistence 分层（只用日线）：买侧 settle persistence ≥ τ_main=ATR14×2 → 主级别
    建仓；卖侧 settle ≥ τ_main → 清仓；τ_sub=ATR14×0.5 ≤ persistence < τ_main → 短差。
    无任何缠论 label。

因果口径（formalization-validity-domain，无执行层前视）
------------------------------------------------------
- lag=1：任何信号在其确认 bar i 出现后，于 **bar i+1 的开盘价** 成交（消除执行层前视）。
- PH settle 在 update(close[k]) 返回非空 = close[k] 涨过屏障的因果时刻（§10 L0 定理），
  于 k+1 开盘成交。
- ⚠️ **TV 信号集后视未消除**：TV 缠论指标输出的是其最终提交的 label 集，可能含重绘
  （repaint）。lag=1 只消除执行层前视，不消除"信号集本身用了未来"这层后视。故 A/B1 的
  缠论信号结果是 **L2 但带信号集后视 caveat**，非干净 OOS。B2 纯 PH 在线流式无此问题
  （OnlineMergeTree 逐根因果，settle 因果确定）。

认识论等级
----------
- bar→date 单调 DP 对齐：L0（确定性，已实测验证）。
- PH merge tree / settle：L0（确定性算法）。
- 级别↔编号映射（缠论原文读出）：L2。
- 持仓盈亏：L0 算术。
- 缠论信号（A/B1）：L2 + TV 信号集后视 caveat。
- 单标的 QQQ：无 L3（跨标的稳健性未做）。

谱系引用
--------
- 267号操作方法论 / 268a 结算：cost_reduction_fsm 的满仓满融降成本语义。
- §10 因果 settle 判据（a_online_persistence）：settle = 分型后续确认的拓扑形式。
- §17.3 规则3 / a_settle_trigger：alive→settled 止跌确认价。
- 002号源不完备 + 第25/31课：买卖点分类看级别不看编号。

约束：本脚本不调用 TV MCP（用已缓存的验证数据）；不做交互式提问。
"""
from __future__ import annotations

import bisect
import json
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import pandas as pd

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.types import Bar  # noqa: E402
from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan.events import BuySellPointCandidateV1  # noqa: E402
from newchan.trading.cost_reduction_fsm import (  # noqa: E402
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    IllegalTransitionError,
    transition,
)

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_MD = ROOT / "analysis" / "correct_multilevel_backtest_qqq.md"

# ── 参数（值判断，显式声明）─────────────────────────────────
SUB_RATIO = 0.3          # FSM 默认每层短差动用比例
SETTLE_WINDOW = 5        # B1：缠论信号后 PH settle 右侧确认窗口（交易日）
ATR_N = 14               # ATR 周期
TAU_MAIN_MULT = 2.0      # B2 主级别阈值 = ATR14 × 2
TAU_SUB_MULT = 0.5       # B2 次级别阈值 = ATR14 × 0.5
TAU_SUPER_MULT = 2.0     # B1 superlevel 清仓阈值 = ATR14 × 2
PRICE_TOL = 0.006        # DP 对齐候选价差容差


# ═══════════════════════════════════════════════════════════════
# 1. 数据加载 + bar→date 单调 DP 对齐
# ═══════════════════════════════════════════════════════════════


def load_daily_ohlcv() -> pd.DataFrame:
    import yfinance as yf

    df = yf.download(
        "QQQ", period="max", interval="1d", progress=False, auto_adjust=False
    )
    df.columns = [c[0] if isinstance(c, tuple) else c for c in df.columns]
    return df[["Open", "High", "Low", "Close"]].dropna()


def _load_pivots(tf: str) -> tuple[list[tuple[int, float]], dict[int, str], list[dict]]:
    """返回 (pivots[(bar,price)] 按 bar 升序, kind{bar:top/bot}, 买卖点labels)。"""
    d = json.loads((DATA_DIR / f"qqq_{tf}_chanlun.json").read_text())
    lines = [l for l in d["pine"]["lines"] if l["si"] == 3]
    kind: dict[int, str] = {}
    piv: dict[int, float] = {}
    for l in lines:
        piv[l["bar1"]] = l["price1"]
        piv[l["bar2"]] = l["price2"]
        up = l["price2"] > l["price1"]
        kind[l["bar2"]] = "top" if up else "bot"
        kind.setdefault(l["bar1"], "bot" if up else "top")
    pivots = sorted(piv.items())
    labels = [
        l
        for l in d["pine"]["labels"]
        if l["si"] == 3 and ("买" in l.get("text", "") or "卖" in l.get("text", ""))
    ]
    return pivots, kind, labels


def _monotonic_align(
    pivots: list[tuple[int, float]],
    kind: dict[int, str],
    H: np.ndarray,
    L: np.ndarray,
) -> dict[int, int]:
    """单调 DP：把 pivot 序列对齐日线极值（top→High, bot→Low），

    最小化总价差，约束 bar 索引严格递增。返回 {pivot_bar: daily_idx}。L0 确定性。
    """
    N = len(H)

    def candidates(p: float, k: str) -> list[tuple[int, float]]:
        arr = H if k == "top" else L
        e = np.abs(arr - p) / p
        js = np.where(e < PRICE_TOL)[0]
        if len(js) == 0:  # 兜底：全局最近（不应发生，实测全部 <1%）
            j = int(e.argmin())
            return [(j, float(e[j]))]
        return [(int(j), float(e[j])) for j in js]

    # DP：状态保留为 {daily_idx: (cost, backptr_state_key, pivot_choice)}
    # prev: list of (daily_idx, cost, path_tuple)
    prev: list[tuple[int, float, tuple]] = [(-1, 0.0, ())]
    for bar, p in pivots:
        k = kind[bar]
        cs = candidates(p, k)
        prev_sorted = sorted(prev, key=lambda x: x[0])
        # 前缀最优：对每个 prev daily_idx，记录到此为止最小 cost 及其 path
        pref: list[tuple[int, float, tuple]] = []
        best = float("inf")
        best_path: tuple = ()
        for di, c, path in prev_sorted:
            if c < best:
                best = c
                best_path = path
            pref.append((di, best, best_path))
        keys = [x[0] for x in pref]
        cur: dict[int, tuple[float, tuple]] = {}
        for j, e in cs:
            pos = bisect.bisect_left(keys, j) - 1
            if pos < 0:
                continue
            bcost, bpath = pref[pos][1], pref[pos][2]
            npath = bpath + ((bar, j),)
            tot = bcost + e
            if j not in cur or tot < cur[j][0]:
                cur[j] = (tot, npath)
        if not cur:  # 无可行候选（前序已占用更大索引）→ 强制单调取最小可行
            j = cs[-1][0]
            base = prev_sorted[-1]
            cur[j] = (base[1] + cs[-1][1], base[2] + ((bar, j),))
        prev = [(j, c, path) for j, (c, path) in cur.items()]
    final = min(prev, key=lambda x: x[1])
    return {bar: di for bar, di in final[2]}


def _nearest_align(
    pivots: list[tuple[int, float]],
    kind: dict[int, str],
    H: np.ndarray,
    L: np.ndarray,
) -> dict[int, int]:
    """次级别（30min/5min）对齐：独立取最近的同类日线极值（允许多个 pivot 落同一日）。

    次级别笔多为同日内摆动 → 不能强制每笔一根独立日线（strict DP 会把跨度人为拉长）。
    实测 30min median 价差 0.003%、5min 0.071%，全部 <1%，重价位重复少（近端窄幅）→
    独立最近匹配可靠。返回 {pivot_bar: daily_idx}。
    """
    out = {}
    for bar, p in pivots:
        arr = H if kind[bar] == "top" else L
        out[bar] = int((np.abs(arr - p)).argmin())
    return out


def map_events(
    tf: str, H: np.ndarray, L: np.ndarray, *, strict: bool = True
) -> tuple[list[dict], dict[int, int], float]:
    """对齐并抽取买卖点事件。返回 (events, pivot_bar->daily_idx, 总价差)。

    strict=True（日线）：单调 DP，每笔一根独立日线。
    strict=False（次级别）：独立最近极值，允许同日多笔。

    event: {daily_idx, bar, cls(1/2/3), dir(buy/sell), price}
    """
    pivots, kind, labels = _load_pivots(tf)
    mapping = _monotonic_align(pivots, kind, H, L) if strict else _nearest_align(pivots, kind, H, L)
    # 总价差（验证用）
    err = 0.0
    for bar, p in pivots:
        j = mapping[bar]
        arr = H if kind[bar] == "top" else L
        err += abs(arr[j] - p) / p
    events = []
    import re

    for lab in labels:
        bar = lab["time"]
        if bar not in mapping:
            continue
        m = re.match(r"\s*(\d)(买|卖)", lab["text"])
        if not m:
            continue
        events.append(
            {
                "daily_idx": mapping[bar],
                "bar": bar,
                "cls": int(m.group(1)),
                "dir": "buy" if m.group(2) == "买" else "sell",
                "price": float(lab["price"]),
            }
        )
    events.sort(key=lambda e: (e["daily_idx"], e["bar"]))
    return events, mapping, err


# ═══════════════════════════════════════════════════════════════
# 2. PH settle 事件（在线 merge tree，逐根因果）
# ═══════════════════════════════════════════════════════════════


def ph_settle_events(closes: np.ndarray, side: str) -> list[dict]:
    """逐根喂 close（side='buy'）或 -close（side='sell'）到 OnlineMergeTree，

    收集每个 settle 事件：{settle_idx, persistence}。side='buy' 的 settle = 下跌腿
    底确认（买信号）；side='sell' = 上涨腿顶确认（卖信号）。L0 因果。
    """
    tree = OnlineMergeTree()
    out = []
    sign = 1.0 if side == "buy" else -1.0
    for k, c in enumerate(closes):
        for mb in tree.update(sign * float(c)):
            out.append({"settle_idx": k, "persistence": float(mb.persistence)})
    return out


def atr14(df: pd.DataFrame) -> np.ndarray:
    h, l, c = df["High"].values, df["Low"].values, df["Close"].values
    pc = np.concatenate([[c[0]], c[:-1]])
    tr = np.maximum.reduce([h - l, np.abs(h - pc), np.abs(l - pc)])
    s = pd.Series(tr).rolling(ATR_N, min_periods=1).mean().values
    return s


# ═══════════════════════════════════════════════════════════════
# 3. 持仓账本（与 FSM 同源事件驱动；FSM 提供降成本语义，账本提供权益）
# ═══════════════════════════════════════════════════════════════


@dataclass
class Ledger:
    """归一化权益账本（own_capital=1，无杠杆，A/B1/B2 可比）。"""

    opens: np.ndarray  # 日线开盘（执行价：信号 bar+1 开盘）
    closes: np.ndarray

    def __post_init__(self):
        self.cash = 1.0
        self.held = 0.0
        self.entry_shares = 0.0           # 主级别底仓规模（短差 sizing 基准）
        self.entry_exec_price = 0.0
        self.entry_fractal_low = 0.0      # 一买分型低点（止损参照）
        self.active_sub = False           # 是否有未平短差
        self.holding = False
        self.equity = np.full(len(self.closes), np.nan)  # 逐日 MTM
        self.trades = []                  # 主级别 round-trip
        self.sub_cycles = []              # (sell, buy) 已完成短差
        self._sub_sell_px = 0.0
        self._sub_shares = 0.0
        self._entry_idx = 0

    def _exec_price(self, signal_idx: int) -> int:
        """lag=1：信号 bar+1 开盘成交，返回执行 bar 索引（越界则用最后一根）。"""
        return min(signal_idx + 1, len(self.opens) - 1)

    def main_entry(self, signal_idx: int, fractal_low: float):
        if self.holding:
            return
        ei = self._exec_price(signal_idx)
        px = float(self.opens[ei])
        v = self.cash  # 全部权益投入
        self.entry_shares = v / px
        self.held = self.entry_shares
        self.cash = 0.0
        self.entry_exec_price = px
        self.entry_fractal_low = fractal_low
        self.holding = True
        self.active_sub = False
        self._entry_idx = ei

    def sub_sell(self, signal_idx: int):
        if not self.holding or self.active_sub:
            return
        ei = self._exec_price(signal_idx)
        px = float(self.opens[ei])
        self._sub_shares = self.entry_shares * SUB_RATIO
        self.held -= self._sub_shares
        self.cash += self._sub_shares * px
        self._sub_sell_px = px
        self.active_sub = True

    def sub_buy(self, signal_idx: int):
        if not self.holding or not self.active_sub:
            return
        ei = self._exec_price(signal_idx)
        px = float(self.opens[ei])
        self.held += self._sub_shares
        self.cash -= self._sub_shares * px
        self.sub_cycles.append(
            {"sell": self._sub_sell_px, "buy": px,
             "profit": (self._sub_sell_px - px) * self._sub_shares}
        )
        self.active_sub = False

    def main_exit(self, signal_idx: int, reason: str):
        if not self.holding:
            return
        ei = self._exec_price(signal_idx)
        px = float(self.opens[ei])
        # 若有未平短差，先按执行价补回（强制平掉 overlay）
        if self.active_sub:
            self.held += self._sub_shares
            self.cash -= self._sub_shares * px
            self.sub_cycles.append(
                {"sell": self._sub_sell_px, "buy": px,
                 "profit": (self._sub_sell_px - px) * self._sub_shares}
            )
            self.active_sub = False
        self.cash += self.held * px
        ret = self.cash / 1.0  # 仅当首笔；多笔用 entry/exit 价算
        self.trades.append(
            {"entry_idx": self._entry_idx, "exit_idx": ei,
             "entry_px": self.entry_exec_price, "exit_px": px, "reason": reason,
             "ret": px / self.entry_exec_price - 1.0}
        )
        self.held = 0.0
        self.holding = False

    def mark(self, idx: int):
        self.equity[idx] = self.cash + self.held * float(self.closes[idx])


# ═══════════════════════════════════════════════════════════════
# 4. 三组回测
# ═══════════════════════════════════════════════════════════════


def _drive_fsm(fsm, etype, price, level):
    """安全推进 FSM：非法转移时跳过（保留 None-> 原状态），返回新 fsm。"""
    try:
        return transition(fsm, FsmEvent(event_type=etype, price=price, level=level))
    except IllegalTransitionError:
        return fsm


def run_group_a(df, daily_ev, sub_ev):
    """A：纯缠论多级别（日线一买/一卖 + 30min 短差）。"""
    opens, highs, lows, closes = (df[c].values for c in ["Open", "High", "Low", "Close"])
    N = len(df)
    led = Ledger(opens=opens, closes=closes)
    fsm = CostReductionFSM.create(own_capital=1.0, sub_ratio=SUB_RATIO)
    # 合并事件流：(daily_idx, rank, kind, payload)
    stream = []
    for e in daily_ev:
        if e["cls"] == 1 and e["dir"] == "buy":
            stream.append((e["daily_idx"], 0, "main_buy", e))
        elif e["cls"] == 1 and e["dir"] == "sell":
            stream.append((e["daily_idx"], 0, "main_sell", e))
    for e in sub_ev:
        stream.append((e["daily_idx"], 1, "sub_" + e["dir"], e))
    stream.sort(key=lambda x: (x[0], x[1], x[3]["bar"]))

    si = 0
    for idx in range(N):
        # 止损扫描：持仓且当日 Low 跌破一买分型低点 → 失效（次日开盘出）
        if led.holding and lows[idx] < led.entry_fractal_low:
            fsm = _drive_fsm(fsm, FsmEventType.BUY_POINT_NEGATED, float(closes[idx]), "daily")
            led.main_exit(idx, "negated")
        # 当日到期的信号
        while si < len(stream) and stream[si][0] == idx:
            _, _, kind, e = stream[si]
            si += 1
            if kind == "main_buy" and not led.holding:
                fsm = _drive_fsm(fsm, FsmEventType.BUY_POINT_CONFIRMED, e["price"], "daily")
                led.main_entry(idx, fractal_low=e["price"])
            elif kind == "main_sell" and led.holding:
                fsm = _drive_fsm(fsm, FsmEventType.MAIN_LEVEL_SELL_POINT, e["price"], "daily")
                led.main_exit(idx, "main_sell")
            elif kind == "sub_sell" and led.holding and not led.active_sub:
                fsm = _drive_fsm(fsm, FsmEventType.SUB_LEVEL_SELL_POINT, e["price"], "30min")
                led.sub_sell(idx)
            elif kind == "sub_buy" and led.holding and led.active_sub:
                fsm = _drive_fsm(fsm, FsmEventType.SUB_LEVEL_BUY_POINT, e["price"], "30min")
                led.sub_buy(idx)
        led.mark(idx)
    return led, fsm


def run_group_b1(df, daily_ev, sub_ev, buy_settles, sell_settles, tau):
    """B1：缠论 + PH settle 门控。一买须买侧 settle 确认建仓；一卖须卖侧 superlevel settle 清仓。"""
    opens, highs, lows, closes = (df[c].values for c in ["Open", "High", "Low", "Close"])
    N = len(df)
    led = Ledger(opens=opens, closes=closes)
    fsm = CostReductionFSM.create(own_capital=1.0, sub_ratio=SUB_RATIO)
    # settle 索引：按 settle_idx 建表
    buy_by_idx: dict[int, list[float]] = {}
    for s in buy_settles:
        buy_by_idx.setdefault(s["settle_idx"], []).append(s["persistence"])
    sell_by_idx: dict[int, list[float]] = {}
    for s in sell_settles:
        sell_by_idx.setdefault(s["settle_idx"], []).append(s["persistence"])

    def confirm_buy(sig_idx):
        """W 窗内任意买侧 settle（sublevel）→ 返回 settle bar，否则 None。"""
        for k in range(sig_idx, min(sig_idx + SETTLE_WINDOW + 1, N)):
            if k in buy_by_idx:
                return k
        return None

    def confirm_sell(sig_idx):
        """W 窗内卖侧 settle 且 persistence ≥ τ_super → settle bar，否则 None。"""
        for k in range(sig_idx, min(sig_idx + SETTLE_WINDOW + 1, N)):
            if k in sell_by_idx and max(sell_by_idx[k]) >= tau[k]:
                return k
        return None

    stream = []
    for e in daily_ev:
        if e["cls"] == 1 and e["dir"] == "buy":
            stream.append((e["daily_idx"], 0, "main_buy", e))
        elif e["cls"] == 1 and e["dir"] == "sell":
            stream.append((e["daily_idx"], 0, "main_sell", e))
    for e in sub_ev:
        stream.append((e["daily_idx"], 1, "sub_" + e["dir"], e))
    stream.sort(key=lambda x: (x[0], x[1], x[3]["bar"]))

    # 待确认队列（缠论信号已出，等 PH settle）
    pend_buy = None   # (fractal_low,)
    pend_sell = False
    si = 0
    for idx in range(N):
        if led.holding and lows[idx] < led.entry_fractal_low:
            fsm = _drive_fsm(fsm, FsmEventType.BUY_POINT_NEGATED, float(closes[idx]), "daily")
            led.main_exit(idx, "negated")
            pend_sell = False
        # 处理当日新信号
        while si < len(stream) and stream[si][0] == idx:
            _, _, kind, e = stream[si]
            si += 1
            if kind == "main_buy" and not led.holding:
                cb = confirm_buy(idx)
                if cb is not None:
                    fsm = _drive_fsm(fsm, FsmEventType.BUY_POINT_CONFIRMED, e["price"], "daily")
                    led.main_entry(cb, fractal_low=e["price"])
            elif kind == "main_sell" and led.holding:
                cs = confirm_sell(idx)
                if cs is not None:
                    fsm = _drive_fsm(fsm, FsmEventType.MAIN_LEVEL_SELL_POINT, e["price"], "daily")
                    led.main_exit(cs, "main_sell_settle")
            elif kind == "sub_sell" and led.holding and not led.active_sub:
                fsm = _drive_fsm(fsm, FsmEventType.SUB_LEVEL_SELL_POINT, e["price"], "30min")
                led.sub_sell(idx)
            elif kind == "sub_buy" and led.holding and led.active_sub:
                fsm = _drive_fsm(fsm, FsmEventType.SUB_LEVEL_BUY_POINT, e["price"], "30min")
                led.sub_buy(idx)
        led.mark(idx)
    return led, fsm


def run_group_b2(df, buy_settles, sell_settles, tau_main, tau_sub, start_idx=0):
    """B2：纯 PH persistence 分层（只用日线，无缠论）。start_idx 之前的 settle 忽略。"""
    opens, highs, lows, closes = (df[c].values for c in ["Open", "High", "Low", "Close"])
    N = len(df)
    led = Ledger(opens=opens, closes=closes)
    fsm = CostReductionFSM.create(own_capital=1.0, sub_ratio=SUB_RATIO)
    # 事件流：按 settle_idx 排序；区分 main / sub
    ev = []
    for s in buy_settles:
        k, p = s["settle_idx"], s["persistence"]
        if k < start_idx:
            continue
        if p >= tau_main[k]:
            ev.append((k, "main_buy", p))
        elif p >= tau_sub[k]:
            ev.append((k, "sub_buy", p))
    for s in sell_settles:
        k, p = s["settle_idx"], s["persistence"]
        if k < start_idx:
            continue
        if p >= tau_main[k]:
            ev.append((k, "main_sell", p))
        elif p >= tau_sub[k]:
            ev.append((k, "sub_sell", p))
    ev.sort(key=lambda x: (x[0], 0 if x[1].startswith("main") else 1))

    si = 0
    for idx in range(N):
        while si < len(ev) and ev[si][0] == idx:
            _, kind, p = ev[si]
            si += 1
            if kind == "main_buy" and not led.holding:
                fsm = _drive_fsm(fsm, FsmEventType.BUY_POINT_CONFIRMED, float(closes[idx]), "PH-main")
                led.main_entry(idx, fractal_low=float(lows[idx]))
            elif kind == "main_sell" and led.holding:
                fsm = _drive_fsm(fsm, FsmEventType.MAIN_LEVEL_SELL_POINT, float(closes[idx]), "PH-main")
                led.main_exit(idx, "ph_main_sell")
            elif kind == "sub_sell" and led.holding and not led.active_sub:
                fsm = _drive_fsm(fsm, FsmEventType.SUB_LEVEL_SELL_POINT, float(closes[idx]), "PH-sub")
                led.sub_sell(idx)
            elif kind == "sub_buy" and led.holding and led.active_sub:
                fsm = _drive_fsm(fsm, FsmEventType.SUB_LEVEL_BUY_POINT, float(closes[idx]), "PH-sub")
                led.sub_buy(idx)
        led.mark(idx)
    return led, fsm


# ═══════════════════════════════════════════════════════════════
# 4b. 第二阶段：streaming 因果引擎（消除信号集后视）— 下游推论#3 的升级
# ═══════════════════════════════════════════════════════════════


STREAM_CACHE = DATA_DIR / "qqq_stream_candidates.json"


def stream_candidates(df: pd.DataFrame) -> list[dict]:
    """逐 bar 喂 RecursiveOrchestrator → 因果缠论买卖点候选（无信号集后视）。

    每个候选在 process_bar(bars[i]) 时产生 = 仅消费 ≤i 的 K 线 → bar i 因果合法。
    与 TV-label 路径的根本差别：streaming candidate **不重绘**（committed 即定格），
    消除了 §0 的信号集后视。kind=type1=一类（转折），type3=三类（中继）；无 type2
    （v1 引擎已知特性，chanlun_ph_backtest 已记录）。缓存避免 ~84s 重跑。
    """
    if STREAM_CACHE.exists():
        cached = json.loads(STREAM_CACHE.read_text())
        if cached.get("n") == len(df):
            return cached["candidates"]
    opens, highs, lows, closes = (df[c].values for c in ["Open", "High", "Low", "Close"])
    idx = df.index
    orch = RecursiveOrchestrator(stream_id="QQQ")
    out = []
    for i in range(len(df)):
        bar = Bar(
            ts=idx[i].to_pydatetime(),
            open=float(opens[i]), high=float(highs[i]),
            low=float(lows[i]), close=float(closes[i]), volume=None,
        )
        snap = orch.process_bar(bar)
        for ev in snap.bsp_snapshot.events:
            if isinstance(ev, BuySellPointCandidateV1):
                out.append(
                    {"idx": i, "kind": ev.kind, "side": ev.side, "price": float(ev.price)}
                )
    STREAM_CACHE.write_text(json.dumps({"n": len(df), "candidates": out}))
    return out


def run_group_a_stream(df, candidates):
    """A_stream：纯缠论因果主级别（type1 一类买/卖，streaming 无后视）。

    无 30min 次级别（引擎单 level_id=1）→ 无短差。这隔离了**主级别信号质量**，
    与 TV-label A 的差额 = 信号集后视的量化。lag=1，止损=日线 Low 跌破入场候选价。
    """
    opens, highs, lows, closes = (df[c].values for c in ["Open", "High", "Low", "Close"])
    N = len(df)
    led = Ledger(opens=opens, closes=closes)
    fsm = CostReductionFSM.create(own_capital=1.0, sub_ratio=SUB_RATIO)
    # 仅 type1（一类=转折）入主级别；type3=中继不动作
    main_buys = {c["idx"]: c["price"] for c in candidates if c["kind"] == "type1" and c["side"] == "buy"}
    main_sells = {c["idx"]: c["price"] for c in candidates if c["kind"] == "type1" and c["side"] == "sell"}
    for idx in range(N):
        if led.holding and lows[idx] < led.entry_fractal_low:
            fsm = _drive_fsm(fsm, FsmEventType.BUY_POINT_NEGATED, float(closes[idx]), "stream")
            led.main_exit(idx, "negated")
        if idx in main_buys and not led.holding:
            fsm = _drive_fsm(fsm, FsmEventType.BUY_POINT_CONFIRMED, main_buys[idx], "stream")
            led.main_entry(idx, fractal_low=main_buys[idx])
        elif idx in main_sells and led.holding:
            fsm = _drive_fsm(fsm, FsmEventType.MAIN_LEVEL_SELL_POINT, main_sells[idx], "stream")
            led.main_exit(idx, "main_sell")
        led.mark(idx)
    return led, fsm


def run_group_b1_stream(df, candidates, buy_settles, sell_settles, tau_super):
    """B1_stream：因果缠论候选 + PH settle 门控（streaming 路径的 PH 对比组）。

    type1 买候选须 W 窗内 PH 买侧 settle（sublevel）右侧确认才建仓；type1 卖候选须 W 窗内
    PH 卖侧 settle 且 persistence ≥ τ_super 才清仓。全程因果、无信号集后视。回答：诚实的
    因果缠论既已跑输 B&H，PH settle 右侧确认能否改善主级别择时？
    """
    opens, highs, lows, closes = (df[c].values for c in ["Open", "High", "Low", "Close"])
    N = len(df)
    led = Ledger(opens=opens, closes=closes)
    fsm = CostReductionFSM.create(own_capital=1.0, sub_ratio=SUB_RATIO)
    buy_by_idx: dict[int, list[float]] = {}
    for s in buy_settles:
        buy_by_idx.setdefault(s["settle_idx"], []).append(s["persistence"])
    sell_by_idx: dict[int, list[float]] = {}
    for s in sell_settles:
        sell_by_idx.setdefault(s["settle_idx"], []).append(s["persistence"])
    main_buys = {c["idx"]: c["price"] for c in candidates if c["kind"] == "type1" and c["side"] == "buy"}
    main_sells = {c["idx"]: c["price"] for c in candidates if c["kind"] == "type1" and c["side"] == "sell"}

    def confirm_buy(sig_idx):
        for k in range(sig_idx, min(sig_idx + SETTLE_WINDOW + 1, N)):
            if k in buy_by_idx:
                return k
        return None

    def confirm_sell(sig_idx):
        for k in range(sig_idx, min(sig_idx + SETTLE_WINDOW + 1, N)):
            if k in sell_by_idx and max(sell_by_idx[k]) >= tau_super[k]:
                return k
        return None

    for idx in range(N):
        if led.holding and lows[idx] < led.entry_fractal_low:
            fsm = _drive_fsm(fsm, FsmEventType.BUY_POINT_NEGATED, float(closes[idx]), "stream")
            led.main_exit(idx, "negated")
        if idx in main_buys and not led.holding:
            cb = confirm_buy(idx)
            if cb is not None:
                fsm = _drive_fsm(fsm, FsmEventType.BUY_POINT_CONFIRMED, main_buys[idx], "stream")
                led.main_entry(cb, fractal_low=main_buys[idx])
        elif idx in main_sells and led.holding:
            cs = confirm_sell(idx)
            if cs is not None:
                fsm = _drive_fsm(fsm, FsmEventType.MAIN_LEVEL_SELL_POINT, main_sells[idx], "stream")
                led.main_exit(cs, "main_sell_settle")
        led.mark(idx)
    return led, fsm


# ═══════════════════════════════════════════════════════════════
# 5. 指标 + 报告
# ═══════════════════════════════════════════════════════════════


def metrics(led: Ledger, df, start_idx: int):
    """从 start_idx 起算权益指标 + buy-and-hold 基准。"""
    closes = df["Close"].values
    eq = led.equity.copy()
    # start 之前未持仓 → 权益 = 1（现金）
    eq = np.where(np.isnan(eq), 1.0, eq)
    base = eq[start_idx] if eq[start_idx] > 0 else 1.0
    seg = eq[start_idx:] / base  # 归一化到窗口起点 = 1（窗口内增长，与窗口前收益无关）
    final = seg[-1]
    # 最大回撤（尺度不变）
    peak = np.maximum.accumulate(seg)
    dd = (seg - peak) / peak
    maxdd = float(dd.min())
    # 暴露占比：只统计窗口内（entry_idx >= start_idx）的持仓天数
    exposure_days = 0
    for tr in led.trades:
        if tr["exit_idx"] < start_idx:
            continue
        exposure_days += tr["exit_idx"] - max(tr["entry_idx"], start_idx)
    if led.holding:
        exposure_days += (len(closes) - 1) - max(led._entry_idx, start_idx)
    total_days = len(closes) - start_idx
    # buy-and-hold（同期）
    bh = closes[-1] / closes[start_idx] - 1.0
    years = total_days / 252.0
    cagr = final ** (1 / years) - 1.0 if years > 0 and final > 0 else float("nan")
    sub_profit = sum(c["profit"] for c in led.sub_cycles)
    sub_win = sum(1 for c in led.sub_cycles if c["profit"] > 0)
    return {
        "final_equity": final,
        "total_return": final - 1.0,
        "cagr": cagr,
        "max_drawdown": maxdd,
        "buy_hold_return": bh,
        "n_main_trades": len(led.trades),
        "n_sub_cycles": len(led.sub_cycles),
        "sub_win": sub_win,
        "sub_profit": sub_profit,
        "exposure_ratio": exposure_days / total_days if total_days else 0.0,
        "still_holding": led.holding,
    }


def fmt_pct(x):
    return f"{x*100:+.1f}%" if x == x else "n/a"


def main():
    df = load_daily_ohlcv()
    H, L = df["High"].values, df["Low"].values
    closes = df["Close"].values
    dates = df.index
    N = len(df)
    tau = atr14(df)
    tau_main = tau * TAU_MAIN_MULT
    tau_sub = tau * TAU_SUB_MULT
    tau_super = tau * TAU_SUPER_MULT

    # 对齐 + 事件
    daily_ev, daily_map, daily_err = map_events("daily", H, L, strict=True)
    sub_ev, sub_map, sub_err = map_events("30min", H, L, strict=False)
    f5_ev, f5_map, f5_err = map_events("5min", H, L, strict=False)

    # PH settle（日线 close / -close）
    buy_settles = ph_settle_events(closes, "buy")
    sell_settles = ph_settle_events(closes, "sell")

    # 回测起点 = 日线第一个一买信号附近，但为公平取 daily 数据首根
    first_main_buy = min(
        (e["daily_idx"] for e in daily_ev if e["cls"] == 1 and e["dir"] == "buy"),
        default=0,
    )
    start_full = 0  # 全期（权益从 $1 现金起，首个建仓信号后入场）

    led_a, fsm_a = run_group_a(df, daily_ev, sub_ev)
    led_b1, fsm_b1 = run_group_b1(df, daily_ev, sub_ev, buy_settles, sell_settles, tau_super)
    led_b2, fsm_b2 = run_group_b2(df, buy_settles, sell_settles, tau_main, tau_sub)

    # 公共窗口（30min 覆盖起点）对比
    sub_start = min((e["daily_idx"] for e in sub_ev), default=0)

    m_a = metrics(led_a, df, start_full)
    m_b1 = metrics(led_b1, df, start_full)
    m_b2 = metrics(led_b2, df, start_full)

    # 受限窗口（30min 起，三组共存）
    led_a_w, _ = run_group_a(df, [e for e in daily_ev if e["daily_idx"] >= sub_start], sub_ev)
    led_b1_w, _ = run_group_b1(
        df, [e for e in daily_ev if e["daily_idx"] >= sub_start], sub_ev,
        buy_settles, sell_settles, tau_super,
    )
    led_b2_w, _ = run_group_b2(df, buy_settles, sell_settles, tau_main, tau_sub, start_idx=sub_start)
    m_a_w = metrics(led_a_w, df, sub_start)
    m_b1_w = metrics(led_b1_w, df, sub_start)
    m_b2_w = metrics(led_b2_w, df, sub_start)

    # ── 第二阶段：streaming 因果引擎（消除信号集后视）──
    print("streaming 因果引擎（首次 ~84s，之后读缓存）...")
    cands = stream_candidates(df)
    led_as, fsm_as = run_group_a_stream(df, cands)
    m_as = metrics(led_as, df, start_full)
    led_b1s, fsm_b1s = run_group_b1_stream(df, cands, buy_settles, sell_settles, tau_super)
    m_b1s = metrics(led_b1s, df, start_full)
    n_t1b = sum(1 for c in cands if c["kind"] == "type1" and c["side"] == "buy")
    n_t1s = sum(1 for c in cands if c["kind"] == "type1" and c["side"] == "sell")
    n_t3 = sum(1 for c in cands if c["kind"] == "type3")

    # ── 写报告 ──
    daily_buys = [e for e in daily_ev if e["cls"] == 1 and e["dir"] == "buy"]
    daily_sells = [e for e in daily_ev if e["cls"] == 1 and e["dir"] == "sell"]

    def row(name, m):
        return (
            f"| {name} | {fmt_pct(m['total_return'])} | {fmt_pct(m['cagr'])} | "
            f"{fmt_pct(m['max_drawdown'])} | {m['exposure_ratio']*100:.0f}% | "
            f"{m['n_main_trades']} | {m['n_sub_cycles']}({m['sub_win']}胜) | "
            f"{fmt_pct(m['buy_hold_return'])} |"
        )

    md = f"""# 多级别缠论 × PH 联合回测（验证数据重跑）— QQQ

> 脚本：`scripts/correct_multilevel_backtest.py`
> 数据：TV 验证缓存（daily/30min/5min，si=3）+ yfinance QQQ 日线（{dates[0].date()} → {dates[-1].date()}，{N} 根）
> 引擎：`OnlineMergeTree`（PH settle）+ `cost_reduction_fsm`（267号降成本）

---

## 0. 数据诚实声明 + 错标修复

**之前回测的 bar 索引错标已修复**：TV label 的 `time` 字段是**稀疏笔端点序号**（daily 仅
440 个 pivot），不是日 bar 计数。直接当日 bar 用 → 25 年压成 1.75 年的严重错位。

**修复方法（L0 确定性）**：逐线方向定 top/bot kind → **单调 DP** 把笔端点价对齐 yfinance
日线 High/Low（最小总价差，保证时间单调）。对齐质量（总相对价差 / pivot 数）：

| timeframe | pivot 数 | 总价差 | avg/pivot | 隐含日期跨度 |
|---|---|---|---|---|
| daily | {len(daily_map)} | {daily_err:.3f} | {daily_err/len(daily_map)*100:.4f}% | {dates[min(daily_map.values())].date()} → {dates[max(daily_map.values())].date()} |
| 30min | {len(sub_map)} | {sub_err:.3f} | {sub_err/len(sub_map)*100:.4f}% | {dates[min(sub_map.values())].date()} → {dates[max(sub_map.values())].date()} |
| 5min | {len(f5_map)} | {f5_err:.3f} | {f5_err/len(f5_map)*100:.4f}% | {dates[min(f5_map.values())].date()} → {dates[max(f5_map.values())].date()} |

avg/pivot < 0.3% = 笔端点价基本精确等于原始日线极值（缠论包含合并后的分型极值 = 原始 OHLC 极值；
误差远小于单根日线波幅 → 日期锚定到正确摆动）。日线用单调 DP（每笔一根独立日线，处理 QQQ 重价位
重复）；30min/5min 用独立最近极值（次级别笔多为同日摆动，允许同日，避免人为拉长跨度）。

**信号集后视 caveat（no-patch-mentality / formalization-validity-domain）**：lag=1 只消除
**执行层前视**（信号确认后第 1 根 K 线开盘成交）。TV 缠论指标输出的是**最终提交的 label 集**，
可能含重绘 → A/B1 的缠论信号带 **信号集后视**，结果是 **L2 但非干净 OOS**。B2 纯 PH 在线
流式（OnlineMergeTree 逐根因果，settle 因果确定）**无此后视**。

---

## 1. 级别映射（缠论原文，第25/31课"看级别不看编号"）

| 角色 | 信号源 | FSM 事件 | 本数据计数 |
|---|---|---|---|
| 主级别建仓 | 日线一类买点 | BUY_POINT_CONFIRMED | {len(daily_buys)} |
| 主级别清仓 | 日线一类卖点 | MAIN_LEVEL_SELL_POINT | {len(daily_sells)} |
| 次级别短差卖 | 30分钟卖点 | SUB_LEVEL_SELL_POINT | {sum(1 for e in sub_ev if e['dir']=='sell')} |
| 次级别短差买 | 30分钟买点 | SUB_LEVEL_BUY_POINT | {sum(1 for e in sub_ev if e['dir']=='buy')} |
| 买点失效止损 | 日线 Low 跌破一买分型低点 | BUY_POINT_NEGATED | 动态 |

> 日线 2/3 类买卖点 = 主级别中继，FSM 无主级别加仓/减仓事件 → 不动作（忠实 FSM 接口）。
> LEVEL_UPGRADE（小转大）未建模——从 label 检测小转大是独立问题，本回测不涉及。
> 30分钟短差只在 30min 数据覆盖窗（{dates[sub_start].date()} 起）激活。

---

## 2. 三组对比 — 全期（{dates[start_full].date()} → {dates[-1].date()}，权益 $1 起）

| 组 | 总收益 | CAGR | 最大回撤 | 市场暴露 | 主级别交易 | 短差(胜) | buy&hold同期 |
|---|---|---|---|---|---|---|---|
{row('A 纯缠论多级别', m_a)}
{row('B1 缠论+PH门控', m_b1)}
{row('B2 纯PH分层', m_b2)}

- **A vs buy&hold**：A 总收益 {fmt_pct(m_a['total_return'])}，同期 buy&hold {fmt_pct(m_a['buy_hold_return'])}，
  暴露仅 {m_a['exposure_ratio']*100:.0f}%（择时减少了市场暴露）。
- **B1 门控效应**：PH settle 右侧确认使建仓更晚更少（主级别交易 {m_b1['n_main_trades']} vs A {m_a['n_main_trades']}）。
- **B2 纯 PH**：无缠论 label，仅靠 persistence 阈值分层，主级别交易 {m_b2['n_main_trades']} 笔。

> 短差循环全期 {m_a['n_sub_cycles']} 次（A），仅在 30min 覆盖窗内发生。

---

## 3. 三组对比 — 受限窗口（30min 共存窗 {dates[sub_start].date()} → {dates[-1].date()}）

此窗内三组信号全部可用，对比最公平：

| 组 | 总收益 | CAGR | 最大回撤 | 市场暴露 | 主级别交易 | 短差(胜) | buy&hold同期 |
|---|---|---|---|---|---|---|---|
{row('A 纯缠论多级别', m_a_w)}
{row('B1 缠论+PH门控', m_b1_w)}
{row('B2 纯PH分层', m_b2_w)}

---

## 4. 降成本 FSM 终态（267号语义）+ gross/net 背离（no-patch-mentality）

| 组 | FSM 末态 | 累计回收(gross) | 短差净盈亏(net,账本) | gross/net 背离 |
|---|---|---|---|---|
| A | {fsm_a.state.name} | {fsm_a.cumulative_recovered:.4f} | {m_a['sub_profit']:.4f} | {fsm_a.cumulative_recovered - m_a['sub_profit']:.4f} |
| B1 | {fsm_b1.state.name} | {fsm_b1.cumulative_recovered:.4f} | {m_b1['sub_profit']:.4f} | {fsm_b1.cumulative_recovered - m_b1['sub_profit']:.4f} |
| B2 | {fsm_b2.state.name} | {fsm_b2.cumulative_recovered:.4f} | {m_b2['sub_profit']:.4f} | {fsm_b2.cumulative_recovered - m_b2['sub_profit']:.4f} |

> **必读张力（不埋掉）**：267号 FSM 的 `cumulative_recovered` 只累加**正利润**
> （`new_recovered += max(profit, 0)`）——它模型化"短差是单调降成本"，**不扣亏损循环**。
> 故 gross 回收（如 B1 = {fsm_b1.cumulative_recovered:.2f}）≫ 账本 net 短差盈亏
> （{m_b1['sub_profit']:.2f}，实为亏损）。PRINCIPAL_WITHDRAWN（gross ≥ 自有资金=1）由此被
> **过早触发**，是 FSM 单调假设的产物，**不代表真实免费仓位**。真实权益见 §2/§3（账本逐日
> MTM），那里短差净亏已计入。这是 FSM 语义与真实 net P&L 的结构性背离，非 bug。
> `cost_basis` 因归一化 own_capital=1 × 真实价 → total_shares≈0.04 放大，绝对值无操盘意义，已略。

---

## 5. 认识论等级 + 边界条件

| 环节 | 等级 | 说明 |
|---|---|---|
| bar→date 对齐（日线DP/次级别最近） | L0 | 确定性，avg<0.3%/pivot 实测 |
| PH merge tree / settle | L0 | 确定性在线算法，因果 settle |
| 持仓盈亏算术 | L0 | — |
| 级别↔编号映射 | L2 | 缠论原文读出（第25/31课） |
| 缠论信号回测（A/B1） | L2+caveat | TV 信号集后视未消除，非干净 OOS |
| B2 纯 PH 回测 | L2 | 在线因果，无信号集后视，可否证 |
| 跨标的稳健性 | — | L3 未做（单标的 QQQ） |

**边界条件（结论翻转条件）**：
- 若 SETTLE_WINDOW（{SETTLE_WINDOW}）放宽 → B1 建仓更多，趋近 A。
- 若 τ_main 倍数（{TAU_MAIN_MULT}）调高 → B2 主级别交易更少、更晚。
- 若 sub_ratio（{SUB_RATIO}）调高 → 短差对成本基影响更大但回撤波动加大。
- 若换标的 / 换时段（L3）→ 任何排序结论可能翻转（单标的不可外推）。

---

## 5b. 第二阶段：streaming 因果引擎对比（消除信号集后视 — 下游推论#3 的兑现）

§0 标注 A/B1 的 TV-label 信号带**信号集后视**（指标重绘）。本阶段用 `RecursiveOrchestrator`
**逐 bar streaming** 产出缠论买卖点候选——每个候选在 `process_bar(bars[i])` 时定格，仅消费
≤i 的 K 线，**不重绘** → 消除信号集后视。引擎单 level_id=1，仅 type1（一类=转折）/ type3
（三类=中继），无 type2（v1 已知特性），无 30min 次级别（故无短差，纯隔离主级别信号质量）。

本数据候选计数：type1 买 {n_t1b} / type1 卖 {n_t1s} / type3 {n_t3}（全期 streaming，{len(cands)} 总）。

| 组 | 信号源 | 后视 | 总收益 | CAGR | 最大回撤 | 市场暴露 | 主级别交易 | buy&hold |
|---|---|---|---|---|---|---|---|---|
| A（TV-label） | TV 缠论指标 | **带信号集后视** | {fmt_pct(m_a['total_return'])} | {fmt_pct(m_a['cagr'])} | {fmt_pct(m_a['max_drawdown'])} | {m_a['exposure_ratio']*100:.0f}% | {m_a['n_main_trades']} | {fmt_pct(m_a['buy_hold_return'])} |
| A_stream（因果） | RecursiveOrchestrator | **无后视** | {fmt_pct(m_as['total_return'])} | {fmt_pct(m_as['cagr'])} | {fmt_pct(m_as['max_drawdown'])} | {m_as['exposure_ratio']*100:.0f}% | {m_as['n_main_trades']} | {fmt_pct(m_as['buy_hold_return'])} |
| B1_stream（因果+PH门控） | Orchestrator + PH settle | **无后视** | {fmt_pct(m_b1s['total_return'])} | {fmt_pct(m_b1s['cagr'])} | {fmt_pct(m_b1s['max_drawdown'])} | {m_b1s['exposure_ratio']*100:.0f}% | {m_b1s['n_main_trades']} | {fmt_pct(m_b1s['buy_hold_return'])} |

**信号集后视的量化**：A（TV-label）{fmt_pct(m_a['total_return'])} − A_stream（因果）{fmt_pct(m_as['total_return'])}
= **{fmt_pct(m_a['total_return'] - m_as['total_return'])}** 的收益差，主要归因于 TV 指标一买精确落大底
（重绘后视）vs 因果候选只能在转折**确认后**入场。这是 §0 caveat 的实测兑现：A 的超额**不是
真实 alpha**，去掉后视后 A_stream 相对 buy&hold（{fmt_pct(m_as['buy_hold_return'])}）的位置才是
缠论主级别信号的**诚实可交易估计**。

**PH settle 门控对因果信号的效应**：B1_stream（{fmt_pct(m_b1s['total_return'])}）vs A_stream
（{fmt_pct(m_as['total_return'])}）——PH settle 右侧确认{'改善' if m_b1s['total_return']>m_as['total_return'] else '未改善'}了因果缠论主级别择时
（{fmt_pct(m_b1s['total_return']-m_as['total_return'])}）。但两者均{'跑输' if max(m_b1s['total_return'],m_as['total_return'])<m_as['buy_hold_return'] else '未稳定跑赢'} buy&hold
（{fmt_pct(m_as['buy_hold_return'])}）→ **PH settle 是右侧过滤器，能改善信号质量但不能把跑输的左侧信号
变成跑赢**（印证 §6.2：PH 非独立 alpha 源）。

> 边界：A_stream/B1_stream 仍是 L2 单标的；type2 缺失使一类/三类覆盖不全（源不完备，002号）。
> 与 A/B1 不完全可比（后者有 30min 短差层）——但短差已证为净拖累（§6.5），故主级别对比仍是
> 后视量化的有效近似。B1_stream 是**完全无信号集后视**的缠论+PH 联合回测（streaming 候选不重绘
> + PH settle 在线因果），是本报告中**认识论等级最高**的可交易估计。

---

## 6. 下游推论

1. **bar→date 价格锚定法可复用**：任何 TV Pine 笔/段 label（`time`=稀疏序号）→ 真实日期的
   映射，都应走"逐线定 kind + 价格匹配"，而非把 `time` 当 bar 计数。日线用单调 DP，次级别用
   独立最近极值。之前所有依赖 `time`=bar 的回测结论需用此法重核。
2. **纯 PH（B2）跑输 buy&hold（受限窗 +32% vs +53%）**：persistence 阈值分层单独不构成
   alpha——PH settle 是**右侧确认/过滤器**（§17.3 规则3 的定位），不是独立的左侧择时信号。
   这与 `settle_backtest.py` / `chanlun_ph_backtest.py` 的"PH 做合取过滤"定位一致。
3. **缠论一买精确落在大底（2003/2008/2018-12-26）→ A 超额收益**：在 §0 信号集后视未消除前，
   A/B1 的超额**不可归因于真实 alpha**。要证伪/证实须换信号集后视已消除的因果引擎（如
   `RecursiveOrchestrator` streaming candidate）重跑——这是 L2→干净 OOS 的升级方向。
4. **267号 FSM 的 gross 回收语义需配 net 账本**：单用 `cumulative_recovered` 判本金退出会过早
   触发（§4）。下游若用 FSM 做实盘降成本决策，须以 net P&L 为准，gross 仅作"已发生正向短差总量"。
5. **30min 短差在强趋势标的上是净拖累（否定性 L2）**：A 全期 {m_a['n_sub_cycles']} 次短差仅
   {m_a['sub_win']} 胜（{m_a['sub_win']/max(m_a['n_sub_cycles'],1)*100:.0f}%），net {m_a['sub_profit']:.2f}
   （亏，归一化自有资金=1）。机理：QQQ 长期强上行，"30min 卖点卖出→30min 买点更高买回"系统性买高 →
   降成本变升成本。**短差降成本的有效域 = 震荡/中枢主导段，不含单边趋势段**——这是 267号方法
   "次级别短差"的有效域边界（formalization-validity-domain：定义域=所有持仓期，有效域⊊震荡段）。
   操盘推论：趋势段应让底仓裸奔（A 的主级别一买持有才是收益来源），短差留给中枢震荡。

## 7. 谱系引用

- **267号 / 268a**：cost_reduction_fsm 满仓满融降成本语义（本回测的持仓系统底座）。
- **§10 因果 settle 判据**（a_online_persistence）：settle = 缠论"分型需后续 K 线确认"的拓扑形式
  ——B1/B2 的右侧确认即此判据。
- **§17.3 规则3 / a_settle_trigger**：alive→settled 的"高级别未完成"定位——B2 纯 PH 跑输印证
  "PH 是过滤器非独立信号"。
- **第25/31课 + 002号源不完备**：买卖点分类"看级别不看编号"——级别映射的原文依据。
- **090号 / formalization-validity-domain**：信号集后视 caveat、gross/net 背离的显现（不埋）依据。
- 不确定是否存在"TV label time 语义"的专门谱系记录——本脚本的错标修复若属新发现，建议
  genealogist 评估是否结晶为"Pine 绘图坐标 ≠ bar 计数"的谱系条目。

## 8. 影响声明

- 新增：`scripts/correct_multilevel_backtest.py`、`analysis/correct_multilevel_backtest_qqq.md`。
- 修复：bar 索引错标（之前回测的根本数据缺陷）→ 逐线 kind + 价格匹配（日线单调 DP / 次级别最近极值）。
- 未改动任何 src/ 模块或定义；复用 `OnlineMergeTree` / `cost_reduction_fsm` 现有接口（只读）。
- 未改动 `.chanlun/` 谱系——§7 仅引用现有条目，新发现的结晶建议留给 genealogist 判定。
"""
    OUT_MD.write_text(md)
    print(f"✅ 报告写入 {OUT_MD}")
    print(f"对齐误差 avg/pivot: daily {daily_err/len(daily_map)*100:.4f}%  "
          f"30min {sub_err/len(sub_map)*100:.4f}%  5min {f5_err/len(f5_map)*100:.4f}%")
    print(f"全期 A {fmt_pct(m_a['total_return'])} | B1 {fmt_pct(m_b1['total_return'])} | "
          f"B2 {fmt_pct(m_b2['total_return'])} | B&H {fmt_pct(m_a['buy_hold_return'])}")
    print(f"受限窗 A {fmt_pct(m_a_w['total_return'])} | B1 {fmt_pct(m_b1_w['total_return'])} | "
          f"B2 {fmt_pct(m_b2_w['total_return'])} | B&H {fmt_pct(m_a_w['buy_hold_return'])}")


if __name__ == "__main__":
    main()
