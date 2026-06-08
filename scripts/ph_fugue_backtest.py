"""PH persistence 分层赋格 FSM 回测 —— QQQ 5min 2年，零前视 streaming。

persistence 是级别本身（不是过滤器）。一次双树 streaming 跑完，所有尺度的 settle
事件同时产出；按 ATR 自适应 τ 分层为 L0/L1/L2，驱动三个并行 CostReductionFSM 声部。

对比组（同一信号流，同一总资金，公平对比）
-------------------------------------------
- A 组：纯 PH 三层赋格 FSM（L0+L1+L2 并行，子带短差降成本 + 小转大赋格分裂）。
- B 组：A + 缠论买卖点分类门控（每带一个因果分类器，主仓开仓须 ∈ {一/二/三买}）。
- C 组：单层 settle baseline（仅 band2 进出，无短差、无赋格）。

零前视：每根 bar 喂入双在线 merge tree，settle 因果实时确认，τ 用滚动 ATR（只用过去）。
信号在当前 bar 收盘可交易——未来数据零参与。

认识论等级：**L2**（QQQ 5min 单标的单时段，结论可否证）。管线正确性见
tests/test_ph_fugue.py（L0/L1）。

用法：.venv/bin/python scripts/ph_fugue_backtest.py
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.trading.ph_fugue import (  # noqa: E402
    ChanBspType,
    ChanlunBspClassifier,
    DualTreeStream,
    FugueVoice,
    PersistenceBands,
    Side,
    Signal,
    rolling_atr,
)

DATA = ROOT / "analysis" / "data_cache" / "qqq_5m_2y.json"
OUT_JSON = ROOT / "analysis" / "data_cache" / "ph_fugue_backtest_qqq.json"
REPORT = ROOT / "analysis" / "ph_fugue_backtest_qqq.md"

# ── 参数（认识论 L2：QQQ 5min 待验证，非普适常数）──
ATR_WINDOW = 270          # 滚动 ATR 窗口（≈3.5 个交易日 of extended-hours 5min）
WARMUP = 270              # 预热：ATR 稳定前不交易
K0, K1, K2 = 1.0, 3.0, 8.0  # τ₀/τ₁/τ₂ = k·ATR（级别分界）
SUB_RATIO = 0.3           # 每层短差减仓比例
TOTAL_CAPITAL = 100_000.0
EQUITY_SAMPLE = 50        # equity 曲线落盘降采样（每 N 根取 1 点）


# ====================================================================
# 数据
# ====================================================================


def load_bars() -> tuple[list[float], list[float], list[float], list[str]]:
    d = json.loads(DATA.read_text())
    bars = d["bars"]
    closes = [float(b["close"]) for b in bars]
    highs = [float(b["high"]) for b in bars]
    lows = [float(b["low"]) for b in bars]
    ts = [b["ts"] for b in bars]
    return closes, highs, lows, ts


# ====================================================================
# 单次双树 streaming → 信号流（A/B/C 共享）
# ====================================================================


def build_signal_stream(
    closes: list[float], atr: list[float], bands: PersistenceBands
) -> tuple[list[list[Signal]], list[float]]:
    """跑一遍双树，返回 (每根 bar 的信号列表, 每根 bar 的 buy-side 主导 alive persistence)。

    dom_alive 用于 LEVEL_UPGRADE（小转大）检测。完全因果。
    """
    stream = DualTreeStream(bands)
    sigs_by_bar: list[list[Signal]] = []
    dom_alive: list[float] = []
    for i, c in enumerate(closes):
        a = atr[i] if i >= WARMUP else 0.0   # 预热期 atr=0 → classify 返回 None → 无信号
        sigs_by_bar.append(stream.step(c, a))
        dom_alive.append(stream.dominant_alive_buy_persistence())
    return sigs_by_bar, dom_alive


# ====================================================================
# 回测核心（replay 信号流）
# ====================================================================


def signal_forward_returns(
    sigs_by_bar: list[list[Signal]], closes: list[float], horizons=(12, 48, 156)
) -> dict:
    """诊断：每个 (band, side) 信号后 h 根 bar 的平均前向收益 vs 随机基线。

    决定性诊断——区分"信号无 edge / 反预测" vs "长多在牛市"两种失败模式：
    若 SELL 信号后前向收益**高于**基线 → 该信号在此 regime 下极性是反的（卖在涨前）。
    完全因果度量（仅描述信号质量，不参与交易）。
    """
    import statistics as st

    n = len(closes)
    flat = [s for sl in sigs_by_bar for s in sl]

    def fwd(sig: Signal, h: int) -> float | None:
        j = sig.bar_idx + h
        return (closes[j] / closes[sig.bar_idx] - 1.0) * 100 if j < n else None

    out: dict = {"horizons": list(horizons), "by_signal": {}, "baseline": {}}
    for band in (2, 1, 0):
        for side in (Side.BUY, Side.SELL):
            key = f"band{band}_{side.name}"
            entry: dict = {"n": 0}
            for h in horizons:
                rs = [fwd(s, h) for s in flat if s.band == band and s.side is side]
                rs = [r for r in rs if r is not None]
                entry["n"] = len(rs)
                entry[f"mean_fwd_{h}"] = round(st.mean(rs), 4) if rs else None
            out["by_signal"][key] = entry
    # 基线：扣除信号、等步长的全样本平均前向收益（牛市漂移基准）
    for h in horizons:
        rs = [(closes[i + h] / closes[i] - 1.0) * 100 for i in range(WARMUP, n - h)]
        out["baseline"][f"mean_fwd_{h}"] = round(st.mean(rs), 4) if rs else None
    return out


def _max_drawdown(curve: list[float]) -> float:
    peak = curve[0] if curve else 0.0
    mdd = 0.0
    for v in curve:
        peak = max(peak, v)
        if peak > 0:
            mdd = max(mdd, (peak - v) / peak)
    return mdd


def _sharpe(curve: list[float], bars_per_year: float) -> float:
    if len(curve) < 3:
        return 0.0
    rets = [(curve[i] / curve[i - 1] - 1.0) for i in range(1, len(curve)) if curve[i - 1] > 0]
    if not rets:
        return 0.0
    mean = sum(rets) / len(rets)
    var = sum((r - mean) ** 2 for r in rets) / len(rets)
    sd = var ** 0.5
    if sd == 0:
        return 0.0
    return (mean / sd) * (bars_per_year ** 0.5)


def run_group(
    name: str,
    sigs_by_bar: list[list[Signal]],
    dom_alive: list[float],
    closes: list[float],
    atr: list[float],
    *,
    mode: str,                  # 'fugue' | 'gated' | 'baseline'
    capital: float = TOTAL_CAPITAL,
) -> dict:
    """replay 信号流到一个对比组，返回交易记录 + equity 曲线 + 指标。"""
    if mode == "baseline":
        voices = [FugueVoice.create("L2", 2, None, capital, sub_ratio=SUB_RATIO)]
        classifiers: dict[int, ChanlunBspClassifier] = {}
    else:
        per = capital / 3.0
        voices = [
            FugueVoice.create("L2", 2, 1, per, sub_ratio=SUB_RATIO),
            FugueVoice.create("L1", 1, 0, per, sub_ratio=SUB_RATIO),
            FugueVoice.create("L0", 0, None, per, sub_ratio=SUB_RATIO),
        ]
        classifiers = {0: ChanlunBspClassifier(), 1: ChanlunBspClassifier(),
                       2: ChanlunBspClassifier()} if mode == "gated" else {}

    by_main = {v.main_band: v for v in voices}
    tau2_k = K2
    equity_curve: list[float] = []
    layer_curves: dict[str, list[float]] = {v.name: [] for v in voices}
    n = len(closes)
    for i in range(n):
        price = closes[i]
        a = atr[i]
        # 小转大：buy-side 主导 alive persistence 越过 τ₂ → 非 L2 声部赋格分裂
        if mode != "baseline":
            tau2 = tau2_k * a
            for v in voices:
                v.maybe_upgrade(i, dom_alive[i], tau2, price)
        # 信号路由
        for sig in sigs_by_bar[i]:
            allow_entry = True
            if mode == "gated" and sig.side is Side.BUY and sig.band in classifiers:
                bsp = classifiers[sig.band].classify(sig)
                # 门控仅作用于该带主仓开仓：分类为 NONE 则不允许新开主仓
                main_voice = by_main.get(sig.band)
                if main_voice is not None and main_voice.fsm.state.name == "SCANNING":
                    allow_entry = bsp is not ChanBspType.NONE
            for v in voices:
                if (not allow_entry and sig.side is Side.BUY
                        and sig.band == v.main_band
                        and v.fsm.state.name == "SCANNING"):
                    continue   # 门控拦截此声部的本次主仓开仓
                v.on_signal(sig)
        # MTM
        total = sum(v.equity(price) for v in voices)
        equity_curve.append(total)
        for v in voices:
            layer_curves[v.name].append(v.equity(price))

    # 指标
    start_eq = equity_curve[WARMUP] if len(equity_curve) > WARMUP else equity_curve[0]
    final_eq = equity_curve[-1]
    bars_per_year = n / 2.0   # 2 年数据
    trades = [t for v in voices for t in v.trades]
    n_open = sum(1 for t in trades if t.action == "OPEN")
    closes_recs = [t for t in trades if t.action == "CLOSE"]
    wins = sum(1 for t in closes_recs if t.realized_pnl > 0)
    metrics = {
        "final_equity": round(final_eq, 2),
        "total_return_pct": round((final_eq / capital - 1.0) * 100, 3),
        "max_drawdown_pct": round(_max_drawdown(equity_curve[WARMUP:]) * 100, 3),
        "sharpe": round(_sharpe(equity_curve[WARMUP:], bars_per_year), 3),
        "n_open_trades": n_open,
        "n_close_trades": len(closes_recs),
        "win_rate_pct": round(wins / len(closes_recs) * 100, 2) if closes_recs else 0.0,
        "n_short_diff": sum(1 for t in trades if t.action == "SHORT_SELL"),
        "n_upgrade": sum(1 for t in trades if t.action == "UPGRADE"),
    }
    layer_final = {v.name: round(v.equity(closes[-1]), 2) for v in voices}
    layer_trade_counts = {
        v.name: {
            "open": sum(1 for t in v.trades if t.action == "OPEN"),
            "close": sum(1 for t in v.trades if t.action == "CLOSE"),
            "short_sell": sum(1 for t in v.trades if t.action == "SHORT_SELL"),
            "short_buy": sum(1 for t in v.trades if t.action == "SHORT_BUY"),
            "upgrade": sum(1 for t in v.trades if t.action == "UPGRADE"),
            "realized": round(v.ledger.realized, 2),
        }
        for v in voices
    }
    return {
        "name": name,
        "mode": mode,
        "metrics": metrics,
        "layer_final_equity": layer_final,
        "layer_trade_counts": layer_trade_counts,
        "equity_curve": [round(equity_curve[k], 2) for k in range(0, n, EQUITY_SAMPLE)],
        "layer_curves": {
            ln: [round(c[k], 2) for k in range(0, n, EQUITY_SAMPLE)]
            for ln, c in layer_curves.items()
        },
        "sample_trades": [
            {"bar": t.bar_idx, "layer": t.layer, "action": t.action, "band": t.band,
             "side": t.side, "price": round(t.price, 2), "shares": round(t.shares, 2),
             "pnl": round(t.realized_pnl, 2), "pers": round(t.persistence, 3),
             "note": t.note}
            for t in trades[:60]
        ],
        "_full_trade_count": len(trades),
    }


# ====================================================================
# main
# ====================================================================


def main() -> None:
    closes, highs, lows, ts = load_bars()
    n = len(closes)
    print(f"[data] {n} bars  {ts[0]} → {ts[-1]}")
    atr = rolling_atr(highs, lows, closes, window=ATR_WINDOW)
    bands = PersistenceBands(K0, K1, K2)

    print("[stream] 双树 streaming（一次，三组共享）...")
    sigs_by_bar, dom_alive = build_signal_stream(closes, atr, bands)
    total_sigs = sum(len(s) for s in sigs_by_bar)
    band_counts = {0: 0, 1: 0, 2: 0}
    side_counts = {Side.BUY: 0, Side.SELL: 0}
    for slist in sigs_by_bar:
        for s in slist:
            band_counts[s.band] += 1
            side_counts[s.side] += 1
    print(f"[stream] {total_sigs} signals  band={band_counts}  "
          f"buy={side_counts[Side.BUY]} sell={side_counts[Side.SELL]}")

    groups = {
        "A_fugue": run_group("A 组：纯 PH 三层赋格", sigs_by_bar, dom_alive, closes, atr,
                             mode="fugue"),
        "B_gated": run_group("B 组：PH 赋格 + 缠论买卖点门控", sigs_by_bar, dom_alive,
                             closes, atr, mode="gated"),
        "C_baseline": run_group("C 组：单层 settle baseline", sigs_by_bar, dom_alive,
                                closes, atr, mode="baseline"),
    }

    print("[diag] 信号前向收益诊断...")
    fwd_diag = signal_forward_returns(sigs_by_bar, closes)

    # buy & hold 基准（从 WARMUP 起）
    bh_start, bh_end = closes[WARMUP], closes[-1]
    bh_ret = (bh_end / bh_start - 1.0) * 100

    result = {
        "symbol": "QQQ", "interval": "5min", "n_bars": n,
        "period": [ts[0], ts[-1]],
        "params": {"atr_window": ATR_WINDOW, "warmup": WARMUP,
                   "k0": K0, "k1": K1, "k2": K2, "sub_ratio": SUB_RATIO,
                   "total_capital": TOTAL_CAPITAL, "equity_sample": EQUITY_SAMPLE},
        "signal_stats": {"total": total_sigs, "band": band_counts,
                         "buy": side_counts[Side.BUY], "sell": side_counts[Side.SELL]},
        "buy_hold_return_pct": round(bh_ret, 3),
        "signal_forward_returns": fwd_diag,
        "groups": groups,
        "epistemic_level": "L2",
    }
    OUT_JSON.write_text(json.dumps(result, ensure_ascii=False))
    print(f"[out] {OUT_JSON}")
    for gk, g in groups.items():
        m = g["metrics"]
        print(f"  {gk:12s} ret={m['total_return_pct']:8.2f}%  "
              f"mdd={m['max_drawdown_pct']:6.2f}%  sharpe={m['sharpe']:6.2f}  "
              f"trades={m['n_open_trades']:4d}  short={m['n_short_diff']:4d}  "
              f"up={m['n_upgrade']}")
    print(f"  buy&hold     ret={bh_ret:8.2f}%")


if __name__ == "__main__":
    main()
