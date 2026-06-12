"""路径3诊断 — 有机赋格 v2 REV 腿逐腿对比：OKLO 翻正 vs QQQ/BRN 为负的根因。

背景（analysis/organic_fugue_rust_backtest.md，V1f = REV+G1 方向锚定）：
  OKLO Δ+607.5pp（rev 586 对 / 净现金 +53195 / 胜率 45.6%）
  QQQ  Δ−9.3pp  （rev 1101 / −4608 / 45.2%）
  BRN  Δ−125.6pp（rev 3396 / −32113 / 43.5%）
三标的胜率几乎相同但净现金符号相反 ⇒ 差异在盈亏不对称，不在胜率。

待检假设（用户）：QQQ/BRN 震荡多、反向笔浅，REV 反向走势深度不够。

方法：重跑 V1f（+V1 对照，隔离 G1）取 diag trace，逐 rev 腿计算：
  深度 depth = (sell−buy)/sell（带符号，正=赚）；持腿时长；payoff 不对称；
  MFE/MAE（持腿期内 low 最深可得深度 / high 最大反向暴露）→ 区分
  "反向走势不存在（浅）" vs "存在但买回时机差（捕获率低）"；
  按 home ladder 分桶；按 50K-bar 时段分桶；强制清腿（buy_bar==master exit_bar）拆分。

trace 口径（rust/src/trading/ledger.rs LegTrace）：
  diag 行 = ((py_key, leg, sell_bar, sell_price, buy_bar, buy_price),
             (shares, diff, profit, was_earning, shares_delta, cb0, cb1))
  rev 的 py_key = 200 + home_ladder；diff = sell − buy；profit = diff × shares
  （AmountConserving 腿 profit 字段同式计算但守恒律是股数回补——现金净额
  归因仍用 profit，与 _leg_stats_rust 口径一致）。
  bar 索引 = load_ohlc 清洗后序列索引（与 highs/lows/closes 对齐）。

认识论等级：L2（三标的真实数据，逐腿）。
限制：逐腿 close 原因（t5/t6/t7/struct）Rust 不暴露——用聚合 counters
（organic_fugue_rust_backtest.json）+ 强制清腿近似（buy_bar==exit_bar）。

输出：analysis/data_cache/rev_tranche_path3_diag.json（每标的增量写）
用法：cd /Users/silencehan/Projects/NewChanlun && \
      PYTHONPATH=src .venv/bin/python analysis/rev_tranche_path3_diag.py
      （env：BT_SYMBOLS=OKLO,QQQ,BRN）
"""

from __future__ import annotations

import json
import os
import sys
import time
from concurrent.futures import ProcessPoolExecutor, as_completed
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "rev_tranche_path3_diag.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO,QQQ,BRN").split(",")]
VARIANTS = ["V1f", "V1"]
BUCKET = 50_000


# ── 分布工具（无 numpy 依赖的腿级标量；MFE 扫描用 numpy）──────────────


def _pct(sorted_vals: list[float], q: float) -> float:
    """线性插值百分位（sorted_vals 已升序，q∈[0,100]）。"""
    n = len(sorted_vals)
    if n == 1:
        return sorted_vals[0]
    pos = q / 100 * (n - 1)
    lo = int(pos)
    hi = min(lo + 1, n - 1)
    return sorted_vals[lo] + (sorted_vals[hi] - sorted_vals[lo]) * (pos - lo)


def _dist(vals: list[float], nd: int = 4) -> dict | None:
    if not vals:
        return None
    sv = sorted(vals)
    return {
        "n": len(sv),
        "mean": round(sum(sv) / len(sv), nd),
        "p10": round(_pct(sv, 10), nd),
        "p25": round(_pct(sv, 25), nd),
        "p50": round(_pct(sv, 50), nd),
        "p75": round(_pct(sv, 75), nd),
        "p90": round(_pct(sv, 90), nd),
    }


def _rev_legs(diag: list) -> list[dict]:
    """diag → rev 腿扁平列表（附 master trade exit_bar → 强制清腿标记）。"""
    legs: list[dict] = []
    for header, diffs in diag:
        exit_bar = header[2]
        for (key, leg, sb, sp, bb, bp), (sh, df, pf, we, _sd, _c0, _c1) in diffs:
            if leg != "rev":
                continue
            legs.append({
                "ladder": key - 200,
                "sell_bar": sb, "sell_price": sp,
                "buy_bar": bb, "buy_price": bp,
                "shares": sh, "diff": df, "profit": pf,
                "was_earning": bool(we),
                "forced": bb == exit_bar,
            })
    return legs


def _subset_stats(legs: list[dict]) -> dict:
    n = len(legs)
    if n == 0:
        return {"n": 0}
    wins = [g for g in legs if g["diff"] > 0]
    return {"n": n,
            "net_cash": round(sum(g["profit"] for g in legs), 1),
            "win_rate": round(len(wins) / n * 100, 1),
            "med_depth_pct": round(_pct(sorted(
                g["diff"] / g["sell_price"] * 100 for g in legs), 50), 4)}


def _analyze(legs: list[dict], highs, lows, closes) -> dict:
    """逐腿诊断维度（脚本 docstring）。depth/mfe/mae 均为 sell_price 百分比。"""
    import numpy as np

    h = np.asarray(highs)
    lo = np.asarray(lows)

    depth, durs, mfes, maes, captures = [], [], [], [], []
    notional = 0.0
    for g in legs:
        d_pct = g["diff"] / g["sell_price"] * 100
        depth.append(d_pct)
        g["depth_pct"] = d_pct
        durs.append(float(g["buy_bar"] - g["sell_bar"]))
        notional += g["shares"] * g["sell_price"]
        if g["sell_bar"] < 0:  # trace 缺 open 记录（unwrap_or(-1)）
            continue
        sl = slice(g["sell_bar"], g["buy_bar"] + 1)
        mfe = (g["sell_price"] - float(lo[sl].min())) / g["sell_price"] * 100
        mae = (float(h[sl].max()) - g["sell_price"]) / g["sell_price"] * 100
        mfes.append(mfe)
        maes.append(mae)
        if mfe > 0:
            captures.append(d_pct / mfe)

    wins = [g for g in legs if g["diff"] > 0]
    losses = [g for g in legs if g["diff"] <= 0]
    gross_win = sum(g["profit"] for g in wins)
    gross_loss = sum(g["profit"] for g in losses)  # ≤0
    net = gross_win + gross_loss
    aw = (sum(g["depth_pct"] for g in wins) / len(wins)) if wins else None
    al = (sum(-g["depth_pct"] for g in losses) / len(losses)) if losses else None

    loss_sorted = sorted(losses, key=lambda g: g["profit"])  # 最差在前
    k5 = max(1, round(len(legs) * 0.05))
    worst5 = sum(g["profit"] for g in loss_sorted[:k5])
    win_sorted = sorted(wins, key=lambda g: -g["profit"])
    best5 = sum(g["profit"] for g in win_sorted[:k5])

    by_ladder = {}
    for lad in sorted({g["ladder"] for g in legs}):
        by_ladder[str(lad)] = _subset_stats([g for g in legs if g["ladder"] == lad])
    by_bucket = {}
    for g in legs:
        b = g["sell_bar"] // BUCKET
        e = by_bucket.setdefault(str(b), {"n": 0, "net_cash": 0.0})
        e["n"] += 1
        e["net_cash"] += g["profit"]
    for e in by_bucket.values():
        e["net_cash"] = round(e["net_cash"], 1)

    return {
        "n_legs": len(legs),
        "net_cash": round(net, 1),
        "win_rate": round(len(wins) / len(legs) * 100, 1) if legs else None,
        "notional": round(notional, 0),
        "net_per_notional_bps": round(net / notional * 1e4, 2) if notional else None,
        "depth_pct": _dist(depth),
        "dur_bars": _dist(durs, nd=0),
        "dur_bars_wins": _dist([float(g["buy_bar"] - g["sell_bar"]) for g in wins], nd=0),
        "dur_bars_losses": _dist([float(g["buy_bar"] - g["sell_bar"]) for g in losses], nd=0),
        "mfe_pct": _dist(mfes),
        "mae_pct": _dist(maes),
        "capture_ratio": _dist(captures),
        "payoff": {
            "avg_win_depth_pct": round(aw, 4) if aw is not None else None,
            "avg_loss_depth_pct": round(al, 4) if al is not None else None,
            "payoff_ratio": round(aw / al, 3) if aw and al else None,
            "gross_win_cash": round(gross_win, 1),
            "gross_loss_cash": round(gross_loss, 1),
            "worst5pct_cash": round(worst5, 1),
            "worst5pct_share_of_loss": round(worst5 / gross_loss * 100, 1)
            if gross_loss < 0 else None,
            "best5pct_cash": round(best5, 1),
            "best5pct_share_of_win": round(best5 / gross_win * 100, 1)
            if gross_win > 0 else None,
        },
        "forced_close": _subset_stats([g for g in legs if g["forced"]]),
        "voluntary_close": _subset_stats([g for g in legs if not g["forced"]]),
        "earning_legs": _subset_stats([g for g in legs if g["was_earning"]]),
        "by_ladder": by_ladder,
        "by_bucket_50k": by_bucket,
    }


def process_symbol(symbol: str) -> dict:
    """信号层（重）+ V1f/V1 Rust 回放（轻）+ 逐 rev 腿诊断。"""
    import newchan_rust as nr
    from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc
    from organic_fugue_rust_check import pack_tape
    from organic_signals import compute_organic_signals

    t0 = time.time()
    opens, highs, lows, closes, _years = load_ohlc(SYMBOL_FILES[symbol])
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips)
    sig_s = time.time() - t0
    rtape = pack_tape(tape, dir_flips=dir_flips)

    out: dict = {"n_bars": len(closes), "sig_elapsed_s": round(sig_s, 1),
                 "variants": {}}
    from fugue_version_i import LADDER_SEG
    for name in VARIANTS:
        res = nr.run_organic_rust(rtape, name, floor_ladder=LADDER_SEG,
                                  stop_mode="none", diag=True)
        legs = _rev_legs(res["diag"])
        out["variants"][name] = _analyze(legs, highs, lows, closes)
    return out


def main() -> None:
    results: dict = {}
    if OUT_JSON.exists():
        results = json.loads(OUT_JSON.read_text())
    todo = [s for s in SYMBOLS
            if s not in results or os.environ.get("BT_FORCE", "0") == "1"]
    if not todo:
        print("[skip] 全部标的已有结果", flush=True)
        return
    with ProcessPoolExecutor(max_workers=min(3, len(todo))) as ex:
        futs = {ex.submit(process_symbol, s): s for s in todo}
        for fut in as_completed(futs):
            sym = futs[fut]
            results[sym] = fut.result()
            OUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False))
            v = results[sym]["variants"]["V1f"]
            print(f"[{sym}] 落盘  rev腿={v['n_legs']} 净现金={v['net_cash']:+.0f}"
                  f" 中位深度={v['depth_pct']['p50']:+.4f}%"
                  f" payoff={v['payoff']['payoff_ratio']}"
                  f" (信号层 {results[sym]['sig_elapsed_s']}s)", flush=True)
    print("done", flush=True)


if __name__ == "__main__":
    main()
