"""多级别缠论 + PH 严格回测 — 方案A（纯缠论）vs 方案B（缠论+PH）对比。

存在论位置 / 数据真相（必读）
================================
三个 chanlun label 文件的 timeframe 字段被系统性错标。按价格幅度与 bar 数考证（笔端
点价精确匹配 yfinance 极值）后的**真实级别**：

    qqq_chanlun_labels.json          标"日线" → 实为 **周线**（主级别，2016–2026）
    nasdaq_qqq_30min...labels.json   标"30min" → 实为 **日线**（次级别，~2024-11–2026）
    nasdaq_qqq_5min...labels.json    标"5min"  → 实为 **30min**（次次级别，~1 周）

真实级别嵌套 = 周线(主) → 日线(次) → 30min(次次)，是标准缠论级别级联。本回测的
主/次映射据此（约束2：级别由 timeframe 决定，不由买卖点编号决定）。

每个文件含两个 study：si=3（主，含全部买卖点）与 si=12（覆盖层，无买卖点）。只用 si=3。

认识论等级（formalization-validity-domain 规则）
------------------------------------------------
- bar索引→日期对齐：单调DP（笔端点配极值），周线84%/日线86%买卖点落区间。残差15%为
  缠论包含处理的合并近似（merged kline极值≠单根原始极值），属噪声非系统性前视偏差。**L2**。
- 因果口径：所有执行价 lag=1（信号确认后下一根 bar 成交），绝不用信号/pivot价（前视偏差）。
- 双口径交叉验证：核心A/B在「对齐+真实OHLC」与「自洽笔路径」两种口径下分别跑，
  结论一致=稳健（L2鲁棒），不一致=暴露对齐敏感性。
- PH增量价值 = B − A：真实数据单标的（QQQ）单时段，**L2**（可否证，未交叉标的→非L3）。

谱系引用
--------
- 267号操作方法论 v1（满仓满融降成本）→ cost_reduction_fsm
- §7.5 在线因果 merge tree → a_online_persistence.OnlineMergeTree
- 因果 settle = 分型确认（拓扑↔缠论同构）→ PH gate 语义
"""

from __future__ import annotations

import json
import re
import sys
from bisect import bisect_left
from dataclasses import dataclass
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.trading.cost_reduction_fsm import (  # noqa: E402
    CostReductionFSM,
    CostState,
    FsmEvent,
    FsmEventType,
    transition,
)

DATA = ROOT / "analysis" / "data_cache"

# ════════════════════════════════════════════════════════════════════
# 1. 数据获取 + 缓存（yfinance，确定性离线复用）
# ════════════════════════════════════════════════════════════════════


@dataclass(frozen=True)
class OHLC:
    """一个级别的真实 OHLC 序列（按时间升序）。"""

    high: np.ndarray
    low: np.ndarray
    close: np.ndarray
    open: np.ndarray
    dates: list[str]

    def __len__(self) -> int:
        return len(self.close)

    def atr14(self) -> np.ndarray:
        """Wilder ATR14（真实波幅 14 周期）。index i 用截至 i 的数据（因果）。"""
        h, l, c = self.high, self.low, self.close
        n = len(c)
        tr = np.zeros(n)
        tr[0] = h[0] - l[0]
        for i in range(1, n):
            tr[i] = max(h[i] - l[i], abs(h[i] - c[i - 1]), abs(l[i] - c[i - 1]))
        atr = np.zeros(n)
        atr[:14] = np.cumsum(tr[:14]) / np.arange(1, 15)[: min(14, n)]
        for i in range(14, n):
            atr[i] = (atr[i - 1] * 13 + tr[i]) / 14
        return atr


def fetch_ohlc(interval: str, period: str, *, cache_tag: str) -> OHLC:
    """拉 yfinance QQQ OHLC，缓存到 npz。"""
    cache = DATA / f"_ohlc_cache_{cache_tag}.npz"
    if cache.exists():
        z = np.load(cache, allow_pickle=True)
        return OHLC(z["high"], z["low"], z["close"], z["open"], list(z["dates"]))
    import yfinance as yf

    df = yf.download(
        "QQQ", period=period, interval=interval, auto_adjust=False, progress=False
    )
    o = df["Open"].values.flatten().astype(float)
    h = df["High"].values.flatten().astype(float)
    l = df["Low"].values.flatten().astype(float)
    c = df["Close"].values.flatten().astype(float)
    dates = [str(x.date()) for x in df.index]
    np.savez(cache, high=h, low=l, close=c, open=o, dates=np.array(dates))
    return OHLC(h, l, c, o, dates)


# ════════════════════════════════════════════════════════════════════
# 2. 缠论 label 加载 + 买卖点分类
# ════════════════════════════════════════════════════════════════════

BUY_SELL_RE = re.compile(r"([123])([买卖])")


@dataclass(frozen=True)
class Signal:
    """一个缠论买卖点。"""

    bar: int          # 缠论 bar 索引（time 字段）
    price: float      # label 价（结构位，仅作止损参照，绝不作执行价）
    kind: str         # '1买'..'3卖'
    is_buy: bool


def load_signals(path: Path) -> list[Signal]:
    d = json.loads(path.read_text())
    labels = [
        l
        for l in d["pine"]["labels"]
        if l.get("si") == 3 and ("买" in l.get("text", "") or "卖" in l.get("text", ""))
    ]
    out = []
    for l in labels:
        m = BUY_SELL_RE.search(l["text"])
        if not m:
            continue
        kind = m.group(1) + m.group(2)
        out.append(
            Signal(bar=l["time"], price=float(l["price"]), kind=kind, is_buy=(m.group(2) == "买"))
        )
    return sorted(out, key=lambda s: s.bar)


def load_pivots(path: Path) -> list[tuple[int, float]]:
    """si=3 笔/线段端点 (bar, price)，按 bar 升序。"""
    d = json.loads(path.read_text())
    piv: dict[int, float] = {}
    for ln in d["pine"]["lines"]:
        if ln.get("si") != 3:
            continue
        if ln.get("bar1") is not None and ln.get("price1"):
            piv[ln["bar1"]] = float(ln["price1"])
        if ln.get("bar2") is not None and ln.get("price2"):
            piv[ln["bar2"]] = float(ln["price2"])
    return sorted(piv.items())


# ════════════════════════════════════════════════════════════════════
# 3. 单调 DP 对齐：缠论 bar → 真实 OHLC 索引
# ════════════════════════════════════════════════════════════════════


def monotonic_align(
    pivots: list[tuple[int, float]], ohlc: OHLC, *, tol: float = 0.0015, skip_pen: float = 0.03
) -> dict[int, int]:
    """笔端点价 ↔ OHLC 极值，强制 bar/索引同向单调，最小化(价差 + skip惩罚)。

    返回 {缠论bar: ohlc索引}（仅 DP 选中的锚点 bar）。
    """
    h, l = ohlc.high, ohlc.low
    n = len(ohlc)
    cand: list[tuple[int, float, list[int]]] = []
    for b, p in pivots:
        cs = [i for i in range(n) if abs(h[i] - p) <= p * tol or abs(l[i] - p) <= p * tol]
        cand.append((b, p, cs))
    K = len(cand)
    NEG = -1
    dp: list[dict[int, tuple[float, int | None]]] = [dict() for _ in range(K + 1)]
    dp[0][NEG] = (0.0, None)
    for k in range(K):
        b, p, cs = cand[k]
        nxt = dp[k + 1]
        for li, (c, _) in dp[k].items():  # skip
            v = (c + skip_pen, li)
            if li not in nxt or v[0] < nxt[li][0]:
                nxt[li] = v
        for i in cs:  # assign
            perr = min(abs(h[i] - p), abs(l[i] - p)) / p
            bestc, bestli = float("inf"), None
            for li, (c, _) in dp[k].items():
                if i > li and c < bestc:
                    bestc, bestli = c, li
            if bestli is not None:
                v = (bestc + perr, bestli)
                if i not in nxt or v[0] < nxt[i][0]:
                    nxt[i] = v
    if not dp[K]:
        return {}
    endi = min(dp[K], key=lambda i: dp[K][i][0])
    barmap: dict[int, int] = {}
    i = endi
    for k in range(K, 0, -1):
        c, par = dp[k][i]
        if par is not None and i != par:
            barmap[cand[k - 1][0]] = i
        i = par if par is not None else i
    return barmap


def make_bar2idx(barmap: dict[int, int]):
    """缠论 bar → OHLC 索引（锚点间线性插值，端点外推）。"""
    xs = sorted(barmap)

    def f(bar: int) -> int:
        if bar in barmap:
            return barmap[bar]
        if not xs:
            return bar
        j = bisect_left(xs, bar)
        if j == 0:
            b0, b1 = xs[0], xs[1] if len(xs) > 1 else xs[0]
        elif j >= len(xs):
            b0, b1 = (xs[-2], xs[-1]) if len(xs) > 1 else (xs[0], xs[0])
        else:
            b0, b1 = xs[j - 1], xs[j]
        if b1 == b0:
            return barmap[b0]
        return barmap[b0] + round((bar - b0) * (barmap[b1] - barmap[b0]) / (b1 - b0))

    return f


def alignment_quality(signals: list[Signal], f, ohlc: OHLC) -> float:
    """买卖点价落在对齐 bar [low,high] 的比例（对齐质量度量）。"""
    n = len(ohlc)
    hit = 0
    for s in signals:
        i = f(s.bar)
        if 0 <= i < n and ohlc.low[i] * 0.995 <= s.price <= ohlc.high[i] * 1.005:
            hit += 1
    return hit / len(signals) if signals else 0.0


# ════════════════════════════════════════════════════════════════════
# 4. 笔路径重构（自洽口径）
# ════════════════════════════════════════════════════════════════════


def pivot_path(pivots: list[tuple[int, float]], max_bar: int) -> np.ndarray:
    """笔端点折线 → 每根缠论 bar 的价格（线性插值）。自洽口径的价格流。"""
    xs = [b for b, _ in pivots]
    ys = [p for _, p in pivots]
    path = np.zeros(max_bar + 1)
    for bar in range(max_bar + 1):
        if bar <= xs[0]:
            path[bar] = ys[0]
        elif bar >= xs[-1]:
            path[bar] = ys[-1]
        else:
            j = bisect_left(xs, bar)
            b0, b1, p0, p1 = xs[j - 1], xs[j], ys[j - 1], ys[j]
            path[bar] = p0 + (bar - b0) * (p1 - p0) / (b1 - b0)
    return path


def path_atr(path: np.ndarray, window: int = 14) -> np.ndarray:
    """笔路径的 ATR 类比 = |Δprice| 的 window 均值（无缺口序列 TR=|Δ|）。"""
    d = np.abs(np.diff(path, prepend=path[0]))
    atr = np.zeros_like(path)
    for i in range(len(path)):
        lo = max(0, i - window + 1)
        atr[i] = d[lo : i + 1].mean()
    return atr


# ════════════════════════════════════════════════════════════════════
# 5. PH 级别封装：sublevel(底) + superlevel(顶) 双树 + settle 门
# ════════════════════════════════════════════════════════════════════


class PHLevel:
    """单一 merge tree 的 **persistence 双阈值分层**——用 persistence 大小区分主/次级别。

    单棵日线 sublevel 树（喂 close，捕底）+ superlevel 树（喂 -close，捕顶）。每个 settle
    特征按 persistence 落入三带（自适应阈值 τ=ATR14×系数）：

        persistence > τ_main          → **主级别** settle（建仓/清仓确认）
        τ_sub < persistence ≤ τ_main  → **次级别** settle（短差信号）
        persistence ≤ τ_sub           → 噪声，忽略

    缠论同构：sublevel settle = 下跌完成（底）→ 买/回补语境；superlevel settle = 上涨
    完成（顶）→ 卖/减仓语境。因果 settle = 分型确认（a_online_persistence 模块）。

    用户裁定：用 persistence 分层取代固定时间周期映射——只需日线一个数据源即可派生主/次级别。
    """

    def __init__(
        self,
        prices: np.ndarray,
        atr: np.ndarray,
        *,
        main_mult: float = 2.0,
        sub_mult: float = 0.5,
        confirm_window: int = 3,
    ):
        self.n = len(prices)
        self.K = confirm_window
        self.tau_main = atr * main_mult
        self.tau_sub = atr * sub_mult
        sub = OnlineMergeTree()
        sup = OnlineMergeTree()
        # 每根 bar 新 settle 的 (persistence, 结构价) 列表
        self.sub: list[list[tuple[float, float]]] = [[] for _ in range(self.n)]
        self.sup: list[list[tuple[float, float]]] = [[] for _ in range(self.n)]
        for i in range(self.n):
            for b in sub.update(float(prices[i])):
                self.sub[i].append((b.persistence, b.birth_price))       # valley 价
            for b in sup.update(float(-prices[i])):
                self.sup[i].append((b.persistence, -b.birth_price))      # peak 价（-close 的 valley）

    def _recent(self, arr: list[list[tuple[float, float]]], idx: int) -> list[tuple[float, float]]:
        lo = max(0, idx - self.K + 1)
        out: list[tuple[float, float]] = []
        for j in range(lo, idx + 1):
            out.extend(arr[j])
        return out

    def main_buy(self, idx: int) -> tuple[bool, float]:
        """近 K bar 内主级别底 settle（persistence>τ_main）。返回 (命中, valley价)。"""
        if not (0 <= idx < self.n):
            return (False, 0.0)
        c = [(p, bp) for p, bp in self._recent(self.sub, idx) if p > self.tau_main[idx]]
        if not c:
            return (False, 0.0)
        p, bp = max(c)
        return (True, bp)

    def main_sell(self, idx: int) -> bool:
        """近 K bar 内主级别顶 settle（persistence>τ_main）。"""
        if not (0 <= idx < self.n):
            return False
        return any(p > self.tau_main[idx] for p, _ in self._recent(self.sup, idx))

    def sub_buy(self, idx: int) -> bool:
        """近 K bar 内次级别底 settle（τ_sub<persistence≤τ_main）= 次级别下跌完成 = 短差回补。"""
        if not (0 <= idx < self.n):
            return False
        return any(self.tau_sub[idx] < p <= self.tau_main[idx] for p, _ in self._recent(self.sub, idx))

    def sub_sell(self, idx: int) -> bool:
        """近 K bar 内次级别顶 settle（τ_sub<persistence≤τ_main）= 次级别上涨完成 = 短差减仓。"""
        if not (0 <= idx < self.n):
            return False
        return any(self.tau_sub[idx] < p <= self.tau_main[idx] for p, _ in self._recent(self.sup, idx))


# ════════════════════════════════════════════════════════════════════
# 6. 统一事件流 + 回测引擎（A/B 共享）
# ════════════════════════════════════════════════════════════════════


@dataclass
class Trade:
    """一笔主级别往返交易记录。"""

    entry_price: float
    exit_price: float = 0.0
    pnl: float = 0.0
    short_diffs: int = 0
    cost_basis_final: float = 0.0
    reached_free: bool = False
    exit_reason: str = ""


C0 = 100_000.0       # 每笔自有资金（固定下注，非复利，便于A/B对比）
SUB_RATIO = 0.3      # 每层短差比例


def run_backtest(
    scheme: str,                         # 'A' 纯缠论多级别 | 'B' 缠论+PH分层 | 'C' 纯PH
    *,
    ph: PHLevel,
    close: np.ndarray,                   # 日线收盘（逐 bar mark + 止损 + lag=1 执行）
    main_buy_bars: dict[int, float],     # {日线bar: 结构参照价}（缠论日线买点）
    main_sell_bars: set[int],            # 缠论日线卖点 bar
    sub_buy_bars: set[int],              # 缠论30min买点 bar（仅A用）
    sub_sell_bars: set[int],             # 缠论30min卖点 bar（仅A用）
    start: int = 0,                      # 交易窗口起点（公平对比：三方案同窗口，PH 已含 warmup）
    stop_buffer: float = 0.005,
) -> tuple[list[Trade], list[float]]:
    """逐 bar 行走回测（日线唯一主轴）。三方案的进出场/短差信号来源不同：

    | 信号 | A 纯缠论 | B 缠论+PH分层 | C 纯PH |
    |------|---------|--------------|--------|
    | 建仓 | 缠论日线买点 | 缠论日线买点 ∧ 主级别底settle | 主级别底settle |
    | 清仓 | 缠论日线卖点 | 缠论日线卖点 ∧ 主级别顶settle | 主级别顶settle |
    | 减仓 | 缠论30min卖点 | 次级别顶settle | 次级别顶settle |
    | 回补 | 缠论30min买点 | 次级别底settle | 次级别底settle |
    """
    n = len(close)

    def entry(i: int) -> tuple[bool, float]:
        if scheme == "A":
            return (i in main_buy_bars, main_buy_bars.get(i, 0.0))
        if scheme == "B":
            hit, ref = ph.main_buy(i)
            return (i in main_buy_bars and hit, main_buy_bars.get(i, ref))
        ok, ref = ph.main_buy(i)         # C
        return (ok, ref)

    def exit_(i: int) -> bool:
        if scheme == "A":
            return i in main_sell_bars
        if scheme == "B":
            return i in main_sell_bars and ph.main_sell(i)
        return ph.main_sell(i)           # C

    def subsell(i: int) -> bool:
        return (i in sub_sell_bars) if scheme == "A" else ph.sub_sell(i)

    def subbuy(i: int) -> bool:
        return (i in sub_buy_bars) if scheme == "A" else ph.sub_buy(i)

    trades: list[Trade] = []
    fsm = CostReductionFSM.create(own_capital=C0, margin_amount=0.0, sub_ratio=SUB_RATIO)
    in_pos = False
    cash = held = total_shares = entry_ref = open_sell_qty = 0.0
    cur = Trade(entry_price=0.0)
    realized_pnl = 0.0
    equity: list[float] = []

    def close_trade(exit_price: float, reason: str) -> None:
        nonlocal in_pos, cash, held, realized_pnl, fsm, cur, total_shares, open_sell_qty
        cash += held * exit_price
        cur.exit_price = exit_price
        cur.pnl = cash
        cur.cost_basis_final = fsm.cost_basis
        cur.reached_free = fsm.state == CostState.PRINCIPAL_WITHDRAWN
        cur.exit_reason = reason
        trades.append(cur)
        realized_pnl += cur.pnl
        in_pos = False
        held = total_shares = open_sell_qty = 0.0
        fsm = CostReductionFSM.create(own_capital=C0, margin_amount=0.0, sub_ratio=SUB_RATIO)

    for i in range(start, n):
        exec_p = float(close[min(i + 1, n - 1)])    # lag=1 执行价

        if in_pos and float(close[i]) < entry_ref * (1 - stop_buffer):
            close_trade(float(close[i]), "买点否定(止损)")

        if not in_pos:
            ok, ref = entry(i)
            if ok:
                fsm = transition(fsm, FsmEvent(FsmEventType.BUY_POINT_CONFIRMED, exec_p, "main"))
                in_pos = True
                cash = -C0
                total_shares = held = C0 / exec_p
                entry_ref = ref if ref > 0 else exec_p * 0.9
                cur = Trade(entry_price=exec_p)
        else:
            if exit_(i):
                close_trade(exec_p, "主级别顶(清仓)")
            else:
                if subsell(i) and open_sell_qty == 0:
                    qty = min(total_shares * SUB_RATIO, held)
                    if qty > 0:
                        held -= qty
                        cash += qty * exec_p
                        open_sell_qty = qty
                        fsm = transition(fsm, FsmEvent(FsmEventType.SUB_LEVEL_SELL_POINT, exec_p, "sub"))
                elif subbuy(i) and open_sell_qty > 0:
                    cash -= open_sell_qty * exec_p
                    held += open_sell_qty
                    cur.short_diffs += 1
                    try:
                        fsm = transition(fsm, FsmEvent(FsmEventType.SUB_LEVEL_BUY_POINT, exec_p, "sub"))
                    except Exception:
                        pass
                    open_sell_qty = 0.0

        unrealized = (cash + held * float(close[i])) if in_pos else 0.0
        equity.append(C0 + realized_pnl + unrealized)

    if in_pos:
        close_trade(float(close[-1]), "数据末端平仓")
        equity[-1] = C0 + realized_pnl

    return trades, equity


# ════════════════════════════════════════════════════════════════════
# 7. 指标
# ════════════════════════════════════════════════════════════════════


def metrics(trades: list[Trade], equity: list[float]) -> dict:
    n = len(trades)
    if n == 0:
        return dict(n_trades=0, win_rate=0, total_return=0, per_trade=0, max_dd=0,
                    profit_factor=0, pl_ratio=0, avg_win=0, avg_loss=0, short_diffs=0,
                    free_positions=0, cost_reduction=0)
    wins = [t.pnl for t in trades if t.pnl > 0]
    losses = [t.pnl for t in trades if t.pnl <= 0]
    total_pnl = sum(t.pnl for t in trades)
    eq = np.array(equity) if equity else np.array([C0])
    peak = np.maximum.accumulate(eq)
    max_dd = float(((peak - eq) / peak).max()) if len(eq) else 0.0
    sd = sum(t.short_diffs for t in trades)
    cr = [
        (t.entry_price - t.cost_basis_final) / t.entry_price
        for t in trades
        if t.short_diffs > 0 and t.entry_price > 0
    ]
    pf = (sum(wins) / abs(sum(losses))) if losses and sum(losses) != 0 else float("inf")
    return dict(
        n_trades=n,
        win_rate=len(wins) / n,
        total_return=total_pnl / C0,
        per_trade=(total_pnl / C0) / n,   # 每笔期望（滤波器质量的公平度量）
        max_dd=max_dd,
        profit_factor=pf,
        avg_win=float(np.mean(wins)) if wins else 0.0,
        avg_loss=float(np.mean(losses)) if losses else 0.0,
        pl_ratio=(np.mean(wins) / abs(np.mean(losses))) if wins and losses else float("inf"),
        short_diffs=sd,
        free_positions=sum(1 for t in trades if t.reached_free),
        cost_reduction=float(np.mean(cr)) if cr else 0.0,
    )


if __name__ == "__main__":
    from multilevel_backtest_run import main

    main()
