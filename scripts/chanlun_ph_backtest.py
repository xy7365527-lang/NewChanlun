"""缠论买卖点（左侧候选）× PH settle（右侧确认）联合回测 — QQQ 日线。

存在论位置
----------
本脚本回答一个具体问题：**缠论结构信号做左侧候选 + PH settle 做右侧确认过滤，
是否优于纯缠论左侧？** 与 `scripts/settle_backtest.py` 互补——那里 PH 是主信号、
中枢是过滤器；这里**缠论买卖点是主信号、PH settle 是过滤器**（方向相反的合取）。

关键发现（L2，已由 smoke 实测，必读）
-------------------------------------
缠论 v1 引擎的 `BuySellPoint.confirmed` 在 streaming 日线上**几乎从不翻 True**
（QQQ 全期 6848 根仅 1 个 confirmed 卖点）。根因是结构性的，非 bug：
  - type1 confirmed = 趋势背驰 confirmed，但 v1 Move.seg_end == last_zs.seg_end →
    背驰 C 段区间恒空（test_v1_pipeline_e2e 第 266 行已记录的已知特性）→ 无 type1。
  - type3 confirmed = 关联 Move.settled，但 moves_from_zhongshus 强制最后一个
    Move.settled=False → 进行中的 type3 长期 unconfirmed。
因此缠论买卖点在工程上**就是左侧候选**——右侧确认必须由别的机制补上。这正是
PH settle（"分型需后续 K 线确认"的拓扑形式）的位置。故本回测用 **candidate 事件**
（BuySellPointCandidateV1，买卖点一出现）作为缠论左侧信号。

因果性（formalization-validity-domain，无 lookahead）
----------------------------------------------------
- 缠论 candidate 在 bar i 出现 = RecursiveOrchestrator.process_bar 仅消费了 ≤i 的
  K 线（streaming 引擎逐 bar 增量）→ 在 close[i] 交易因果合法。
- PH settle 在 bar i 的 OnlineMergeTree.update(close[i]) 返回非空 = close[i] 涨过
  屏障的那一刻因果确认（a_online_persistence §10 L0 定理）。
- 合取窗口 W：缠论 candidate 在 i 出现后，等 PH settle 在 [i, i+W] 内出现才进场
  （右侧确认晚于左侧候选）。W 是显式值判断（默认 5 交易日），改 W 改信号。

策略矩阵
--------
- A（纯缠论）：缠论 buy candidate 进场，缠论 sell candidate 出场。
- B（缠论 + PH 右侧过滤）：缠论 buy candidate + W 窗口内 PH buy settle 合取进场；
  缠论 sell candidate + W 窗口内 PH sell settle 合取出场。
- buy-and-hold：全期满仓基准。

认识论等级
----------
- 缠论管线 / PH merge tree：L0（确定性算法）。
- "candidate=左侧候选""settle=右侧分型确认"：L0 算法 + 候选同构。
- QQQ 单标的回测胜率/收益：**L2**（可否证，信息增量所在；否定性结果同样有价值）。
- 跨标的：**L3 未做**（严格限定 QQQ）。
"""
from __future__ import annotations

import json
import math
import sys
import time
from datetime import datetime
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

import yfinance as yf  # noqa: E402

from newchan.types import Bar  # noqa: E402
from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan.events import (  # noqa: E402
    BuySellPointCandidateV1,
    BuySellPointConfirmV1,
)
from newchan.a_online_persistence import OnlineMergeTree  # noqa: E402
from newchan.a_persistence_barcode import atr_noise_threshold  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"
OUT_MD = ROOT / "analysis" / "chanlun_ph_backtest_qqq.md"
SIG_CACHE = CACHE / "chanlun_ph_signals_qqq.json"
SYMBOL = "QQQ"
PERIOD = "max"
CONJ_WINDOW = 5  # 合取窗口（交易日）：缠论候选后等 PH settle 的最大根数（值判断）
ZIGZAG_REV = 0.08  # 简化版 zigzag 反转阈值（8%，百分比口径，独立于 PH 绝对 τ）


# ====================================================================
# 数据
# ====================================================================


def pull(symbol: str, period: str, interval: str = "1d") -> dict:
    df = yf.download(symbol, period=period, interval=interval,
                     auto_adjust=False, progress=False)
    if df.empty:
        raise SystemExit(f"yfinance 返回空数据: {symbol}")

    def col(name: str):
        if (name,) in df.columns or name in df.columns:
            s = df[name]
        else:
            s = df.xs(name, axis=1, level=0)
        return [float(x) for x in s.to_numpy().ravel()]

    raw = {"closes": col("Close"), "highs": col("High"),
           "lows": col("Low"), "opens": col("Open"),
           "dates": [d.strftime("%Y-%m-%d") for d in df.index]}
    keep = [i for i in range(len(raw["closes"]))
            if not any(math.isnan(raw[k][i])
                       for k in ("closes", "highs", "lows", "opens"))]
    return {"symbol": symbol,
            "closes": [raw["closes"][i] for i in keep],
            "highs": [raw["highs"][i] for i in keep],
            "lows": [raw["lows"][i] for i in keep],
            "opens": [raw["opens"][i] for i in keep],
            "dates": [raw["dates"][i] for i in keep]}


# ====================================================================
# 信号提取（streaming 一遍，缓存复用）
# ====================================================================


def build_signals(data: dict) -> dict:
    """逐 bar streaming：缠论候选/确认买卖点 + PH settle 右侧确认。

    返回每根 bar 的布尔信号四元组，按 close 序列对齐（raw bar 索引）。
    缠论事件的 bar_idx 是引擎内部坐标，但事件**在 process_bar(bars[i]) 时产生**，
    故直接归属到 raw 索引 i（因果对齐，不依赖 merged 坐标转换）。
    """
    closes, highs, lows = data["closes"], data["highs"], data["lows"]
    n = len(closes)
    tau = atr_noise_threshold(highs, lows, closes, period=14, multiple=1.0)

    bars = [Bar(ts=datetime.fromisoformat(data["dates"][i]),
                open=data["opens"][i], high=highs[i],
                low=lows[i], close=closes[i], volume=None) for i in range(n)]

    orch = RecursiveOrchestrator(stream_id=SYMBOL)
    t_buy = OnlineMergeTree()    # 喂 close：下跌腿 settle = PH 买确认
    t_sell = OnlineMergeTree()   # 喂 -close：上涨腿 settle = PH 卖确认

    sig = {k: [False] * n for k in
           ("chan_buy_cand", "chan_sell_cand", "chan_buy_conf",
            "chan_sell_conf", "ph_buy_settle", "ph_sell_settle")}
    n_conf = 0

    t0 = time.time()
    for i in range(n):
        snap = orch.process_bar(bars[i])
        for ev in snap.bsp_snapshot.events:
            if isinstance(ev, BuySellPointCandidateV1):
                key = "chan_buy_cand" if ev.side == "buy" else "chan_sell_cand"
                sig[key][i] = True
            elif isinstance(ev, BuySellPointConfirmV1):
                key = "chan_buy_conf" if ev.side == "buy" else "chan_sell_conf"
                sig[key][i] = True
                n_conf += 1
        # PH settle（与 settle_backtest 同口径：close 双树，persistence≥τ）
        nb = [b for b in t_buy.update(closes[i]) if b.persistence >= tau]
        ns = [b for b in t_sell.update(-closes[i]) if b.persistence >= tau]
        sig["ph_buy_settle"][i] = bool(nb)
        sig["ph_sell_settle"][i] = bool(ns)
        if i % 1000 == 0:
            print(f"  streaming {i}/{n} ...", flush=True)
    dt = time.time() - t0

    counts = {k: sum(v) for k, v in sig.items()}
    print(f"  streaming 完成 {dt:.1f}s  信号计数: {counts}  confirm={n_conf}",
          flush=True)
    return {"tau": tau, "stream_seconds": round(dt, 1),
            "counts": counts, "signals": sig}


# ====================================================================
# 简化版独立信号源：zigzag 笔 + MACD 背驰（交叉验证仓库引擎 candidate）
# ====================================================================


def _ema(vals: list[float], span: int) -> list[float]:
    """因果 EMA（仅用历史，无 lookahead）。"""
    k = 2.0 / (span + 1)
    out: list[float] = []
    e = vals[0]
    for v in vals:
        e = v * k + e * (1 - k)
        out.append(e)
    return out


def build_zigzag_macd(data: dict, rev_pct: float = ZIGZAG_REV) -> dict:
    """zigzag 摆动 pivot + MACD(DIF) 背驰 → 买卖信号（因果，无 lookahead）。

    - zigzag pivot：价格从极值反向超过 rev_pct 时**确认**一个 pivot（右侧确认），
      信号落在确认那根 bar（不是极值根，避免 lookahead）。
    - 底背驰（买）：新低 pivot 价 < 前低 pivot 价，但 DIF 更高（动能衰减）。
    - 顶背驰（卖）：新高 pivot 价 > 前高 pivot 价，但 DIF 更低。
    DIF = EMA12 − EMA26（标准 MACD 快线），在 pivot 极值根取值比较。
    """
    closes = data["closes"]
    n = len(closes)
    dif = [a - b for a, b in zip(_ema(closes, 12), _ema(closes, 26))]

    sig_buy = [False] * n
    sig_sell = [False] * n
    last_low: tuple[int, float, float] | None = None   # (idx, price, dif)
    last_high: tuple[int, float, float] | None = None
    direction = 0      # 0=未定, 1=找高点(上升), -1=找低点(下降)
    ext_idx, ext_price = 0, closes[0]

    for i in range(1, n):
        c = closes[i]
        if direction >= 0:                       # 正在找高点
            if c > ext_price:
                ext_idx, ext_price = i, c
            elif c < ext_price * (1 - rev_pct):  # 反转确认一个 high pivot
                if (last_high is not None and ext_price > last_high[1]
                        and dif[ext_idx] < last_high[2]):
                    sig_sell[i] = True           # 顶背驰，确认根 i
                last_high = (ext_idx, ext_price, dif[ext_idx])
                direction, ext_idx, ext_price = -1, i, c
                continue
        if direction <= 0:                       # 正在找低点
            if c < ext_price:
                ext_idx, ext_price = i, c
            elif c > ext_price * (1 + rev_pct):  # 反转确认一个 low pivot
                if (last_low is not None and ext_price < last_low[1]
                        and dif[ext_idx] > last_low[2]):
                    sig_buy[i] = True            # 底背驰，确认根 i
                last_low = (ext_idx, ext_price, dif[ext_idx])
                direction, ext_idx, ext_price = 1, i, c

    return {"zz_buy": sig_buy, "zz_sell": sig_sell}


# ====================================================================
# 策略状态机（long-only，复用 settle_backtest 的权益/回撤口径）
# ====================================================================


@dataclass
class Strategy:
    name: str
    entry_fn: object   # (i, sig) -> bool
    exit_fn: object    # (i, sig) -> bool
    pos: dict | None = None
    trades: list[dict] = field(default_factory=list)
    realized_equity: float = 1.0
    curve: list[float] = field(default_factory=list)
    days_in_market: int = 0

    def step(self, i: int, closes: list[float], dates: list[str], sig: dict) -> None:
        if self.pos is None:
            if self.entry_fn(i, sig):
                self.pos = {"entry_idx": i, "entry_close": closes[i],
                            "entry_date": dates[i]}
        else:
            if self.exit_fn(i, sig):
                self._close(i, closes, dates)
        if self.pos is not None:
            self.days_in_market += 1
            eq = self.realized_equity * closes[i] / self.pos["entry_close"]
        else:
            eq = self.realized_equity
        self.curve.append(eq)

    def _close(self, i, closes, dates):
        p = self.pos
        ret = closes[i] / p["entry_close"] - 1.0
        self.realized_equity *= (1.0 + ret)
        self.trades.append({"entry_date": p["entry_date"], "exit_date": dates[i],
                            "entry": round(p["entry_close"], 2),
                            "exit": round(closes[i], 2),
                            "ret_pct": round(ret * 100, 2),
                            "days": i - p["entry_idx"]})
        self.pos = None

    def metrics(self, closes, dates) -> dict:
        wins = [t for t in self.trades if t["ret_pct"] > 0]
        n = len(self.trades)
        losses = [t for t in self.trades if t["ret_pct"] <= 0]
        open_pnl = None
        if self.pos is not None:
            open_pnl = {"entry_date": self.pos["entry_date"],
                        "entry": round(self.pos["entry_close"], 2),
                        "last": round(closes[-1], 2),
                        "float_ret_pct": round(
                            (closes[-1] / self.pos["entry_close"] - 1) * 100, 2)}
        return {"name": self.name, "n_trades": n,
                "win_rate_pct": round(len(wins) / n * 100, 1) if n else 0.0,
                "avg_ret_pct": round(sum(t["ret_pct"] for t in self.trades) / n, 2) if n else 0.0,
                "avg_win_pct": round(sum(t["ret_pct"] for t in wins) / len(wins), 2) if wins else 0.0,
                "avg_loss_pct": round(sum(t["ret_pct"] for t in losses) / len(losses), 2) if losses else 0.0,
                "avg_hold_days": round(sum(t["days"] for t in self.trades) / n, 1) if n else 0.0,
                "total_ret_pct": round((self.realized_equity - 1) * 100, 1),
                "max_drawdown_pct": round(_max_drawdown(self.curve) * 100, 1),
                "market_exposure_pct": round(self.days_in_market / len(closes) * 100, 1),
                "open_position": open_pnl, "trades": self.trades}


def _max_drawdown(curve: list[float]) -> float:
    peak, mdd = float("-inf"), 0.0
    for v in curve:
        peak = max(peak, v)
        if peak > 0:
            mdd = max(mdd, (peak - v) / peak)
    return mdd


# ── 信号合取（含 W 窗口的右侧确认） ──


def _within_window(i: int, sig_key: str, sig: dict, w: int) -> bool:
    """bar i 是否处于"某缠论候选在 [i-w, i] 出现"的待确认窗口内。"""
    lo = max(0, i - w)
    return any(sig[sig_key][j] for j in range(lo, i + 1))


def make_entry_chan(side_cand: str):
    return lambda i, sig: sig[side_cand][i]


def make_entry_conj(side_cand: str, ph_key: str, w: int):
    # 进场当根：PH settle 触发，且 [i-w, i] 内有缠论候选（右侧确认左侧候选）
    return lambda i, sig: sig[ph_key][i] and _within_window(i, side_cand, sig, w)


# ====================================================================
# 回测主流程
# ====================================================================


def run(data: dict, sigblob: dict, w: int = CONJ_WINDOW) -> dict:
    closes, dates = data["closes"], data["dates"]
    n = len(closes)
    sig = sigblob["signals"]

    strategies = [
        Strategy("A1 缠论候选(进/出)",
                 make_entry_chan("chan_buy_cand"),
                 lambda i, s: s["chan_sell_cand"][i]),
        # B 系列：进场合取(右侧 PH 过滤假买点)，出场用纯同级别卖点（用户原意）
        Strategy("B1 缠论候选+PH过滤",
                 make_entry_conj("chan_buy_cand", "ph_buy_settle", w),
                 lambda i, s: s["chan_sell_cand"][i]),
        Strategy("A2 zigzag+MACD背驰",
                 make_entry_chan("zz_buy"),
                 lambda i, s: s["zz_sell"][i]),
        Strategy("B2 zigzag背驰+PH过滤",
                 make_entry_conj("zz_buy", "ph_buy_settle", w),
                 lambda i, s: s["zz_sell"][i]),
    ]
    for i in range(n):
        for s in strategies:
            s.step(i, closes, dates, sig)

    bh_ret = closes[-1] / closes[0] - 1.0
    bh_curve = [c / closes[0] for c in closes]
    bh = {"total_ret_pct": round(bh_ret * 100, 1),
          "max_drawdown_pct": round(_max_drawdown(bh_curve) * 100, 1),
          "annualized_pct": round(((closes[-1] / closes[0]) ** (252.0 / n) - 1) * 100, 1)}

    return {"meta": {"symbol": data["symbol"], "n_bars": n,
                     "date_start": dates[0], "date_end": dates[-1],
                     "price_start": round(closes[0], 2), "price_end": round(closes[-1], 2),
                     "tau_atr": round(sigblob["tau"], 4),
                     "conj_window": w, "counts": sigblob["counts"],
                     "stream_seconds": sigblob["stream_seconds"]},
            "buy_and_hold": bh,
            "strategies": [s.metrics(closes, dates) for s in strategies]}


def main() -> None:
    rebuild = "--rebuild" in sys.argv or not SIG_CACHE.exists()
    CACHE.mkdir(parents=True, exist_ok=True)

    data_file = CACHE / "QQQ_1d_max.json"
    if data_file.exists() and not rebuild:
        data = json.loads(data_file.read_text())
    else:
        data = pull(SYMBOL, PERIOD)
        data_file.write_text(json.dumps(data, ensure_ascii=False))

    if rebuild:
        print("构建信号（streaming，约 130s）...", flush=True)
        sigblob = build_signals(data)
        SIG_CACHE.write_text(json.dumps(sigblob, ensure_ascii=False))
    else:
        sigblob = json.loads(SIG_CACHE.read_text())
        print(f"复用信号缓存  counts={sigblob['counts']}", flush=True)

    # zigzag+MACD 独立信号源（纯本地，秒级，每次现算并并入）
    zz = build_zigzag_macd(data)
    sigblob["signals"].update(zz)
    sigblob["counts"].update({k: sum(v) for k, v in zz.items()})

    res = run(data, sigblob)
    # W 敏感性扫描（合取窗口对 B 系列笔数/收益的影响，边界条件探查）
    wscan = []
    for wv in (5, 10, 20, 40, 60, 120):
        rr = run(data, sigblob, w=wv)
        b1 = next(s for s in rr["strategies"] if s["name"].startswith("B1"))
        b2 = next(s for s in rr["strategies"] if s["name"].startswith("B2"))
        wscan.append({"w": wv,
                      "B1_n": b1["n_trades"], "B1_ret": b1["total_ret_pct"],
                      "B1_dd": b1["max_drawdown_pct"], "B1_exp": b1["market_exposure_pct"],
                      "B2_n": b2["n_trades"], "B2_ret": b2["total_ret_pct"],
                      "B2_dd": b2["max_drawdown_pct"], "B2_exp": b2["market_exposure_pct"]})
    res["wscan"] = wscan
    (CACHE / "chanlun_ph_backtest_qqq.json").write_text(
        json.dumps(res, ensure_ascii=False, indent=2))

    m = res["meta"]
    bh = res["buy_and_hold"]
    print("=" * 78)
    print(f"缠论×PH 联合回测  {m['date_start']} → {m['date_end']}  (n={m['n_bars']})")
    print(f"信号计数 {m['counts']}  W={m['conj_window']}  τ={m['tau_atr']}")
    print(f"buy&hold 全期 {bh['total_ret_pct']:+.1f}%  年化 {bh['annualized_pct']:+.1f}%  "
          f"回撤 −{bh['max_drawdown_pct']:.1f}%")
    print("-" * 78)
    print(f"{'策略':<26}{'笔数':>5}{'胜率':>7}{'平均':>8}{'持有':>7}{'总收益':>10}{'回撤':>8}{'暴露':>7}")
    for s in res["strategies"]:
        print(f"{s['name']:<26}{s['n_trades']:>5}{s['win_rate_pct']:>6.0f}%"
              f"{s['avg_ret_pct']:>7.2f}%{s['avg_hold_days']:>6.0f}d"
              f"{s['total_ret_pct']:>9.1f}%{-s['max_drawdown_pct']:>7.1f}%"
              f"{s['market_exposure_pct']:>6.0f}%")
        if s["open_position"]:
            op = s["open_position"]
            print(f"    └ 末根持仓 {op['entry_date']}@{op['entry']} 浮盈 {op['float_ret_pct']:+.1f}%")
    print("-" * 78)
    print("W 敏感性（合取窗口）:  W | B1笔/收益/回撤/暴露 | B2笔/收益/回撤/暴露")
    for r in res["wscan"]:
        print(f"  W={r['w']:>3} | B1 {r['B1_n']}笔 {r['B1_ret']:+.0f}% "
              f"−{r['B1_dd']:.0f}% {r['B1_exp']:.0f}% | "
              f"B2 {r['B2_n']}笔 {r['B2_ret']:+.0f}% −{r['B2_dd']:.0f}% {r['B2_exp']:.0f}%")


if __name__ == "__main__":
    main()
