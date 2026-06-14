"""买卖点配对方向正确率（Dim3 修正——编排者 2026-06-14）。

错误的旧测法：固定 N bar 前瞻方向（@10/50/100）——测的是任意时间窗方向，
不是走势结构定义的方向。
正确测法：每个买点配同级别**下一个卖点**，算 buy_price→sell_price；每个卖点配
同级别下一个买点，算 sell_price→buy_price。走势结构保证买点到卖点价格必然上涨——
若不成立则是 BSP 引擎 bug；若成立则"方向≈随机"结论需修正（损失在操作层非信号层）。

BSP 事件（confirmed）来自 BarSignalI.bsp_events[ladder]，元组 =
  (kind, side, seg_idx, confirmed, center_seg_start, center_zd, center_zg, price)
价格用 BSP 结构价（price 字段）为主，close-at-bar 为交叉校验。

附加：配对平均幅度、扣除摩擦（0.1%/腿，往返 0.2%）后净幅、哪个级别幅度最大。

用法: PYTHONPATH=src .venv/bin/python analysis/_bsp_pairing_diag.py [SYM ...]
输出: analysis/data_cache/_bsp_pairing_<SYM>.json
"""
from __future__ import annotations

import json
import math
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))
from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from nested_recursive_fugue_final_backtest import LADDER_NAMES  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
SYMBOLS = sys.argv[1:] or ["OKLO", "QQQ", "CL"]
FRICTION_LEG = 0.001  # 0.1% 每腿；往返(开+平)=0.2%


def collect_bsps(tape) -> dict:
    """{ladder: [(bar, side, kind, price), ...]}（仅 confirmed，按 bar 升序）。"""
    by_lev: dict[int, list] = {}
    for i, s in enumerate(tape):
        evs = s.bsp_events
        for lad in range(len(evs)):
            row = evs[lad]
            if not row:
                continue
            for ev in row:
                if len(ev) < 8:
                    continue
                kind, side, _seg, confirmed = ev[0], ev[1], ev[2], ev[3]
                price = ev[7]
                if not confirmed:
                    continue
                if price is None or (isinstance(price, float) and math.isnan(price)):
                    continue
                by_lev.setdefault(lad, []).append((i, side, kind, float(price)))
    for lad in by_lev:
        by_lev[lad].sort(key=lambda r: r[0])
    return by_lev


def pair_amplitudes(seq: list, closes: list, frm: str, to: str) -> dict:
    """seq 内每个 frm 点配下一个 to 点，算 (to_price - frm_price)/frm_price。
    frm="buy",to="sell" → 期望正（买→卖涨）；frm="sell",to="buy" → 期望负（卖→买跌）。
    同时用 close-at-bar 交叉校验。返回幅度分布。"""
    n = len(seq)
    # 预计算每位置之后第一个 to-side 的索引
    next_to = [None] * n
    nxt = None
    for k in range(n - 1, -1, -1):
        if seq[k][1] == to:
            nxt = k
        next_to[k] = nxt if seq[k][1] != to else None
    amps_price, amps_close, held = [], [], []
    for k in range(n):
        if seq[k][1] != frm:
            continue
        j = next_to[k]
        if j is None:
            continue
        fb, _, _, fp = seq[k]
        tb, _, _, tp = seq[j]
        amps_price.append((tp - fp) / fp)
        amps_close.append((closes[tb] - closes[fb]) / closes[fb])
        held.append(tb - fb)
    if not amps_price:
        return {"n": 0}
    # 对 buy→sell：方向正确 = 涨(>0)；对 sell→buy：方向正确 = 跌(<0)
    want_up = (frm == "buy")
    correct = sum(1 for a in amps_price if (a > 0) == want_up)
    gross = sum(amps_price) / len(amps_price)
    gross_c = sum(amps_close) / len(amps_close)
    # 净幅：对 buy→sell 取 +gross，对 sell→buy 取 -gross（做空收益=跌幅），各扣往返 0.2%
    directional_gross = gross if want_up else -gross
    net = directional_gross - 2 * FRICTION_LEG
    absamps = sorted(abs(a) for a in amps_price)
    med = absamps[len(absamps) // 2]
    return {
        "n": len(amps_price),
        "dir_correct_rate": round(correct / len(amps_price), 4),
        "gross_amp_price_pct": round(gross * 100, 4),
        "gross_amp_close_pct": round(gross_c * 100, 4),
        "directional_gross_pct": round(directional_gross * 100, 4),
        "net_after_friction_pct": round(net * 100, 4),
        "median_abs_amp_pct": round(med * 100, 4),
        "avg_held_bars": round(sum(held) / len(held), 0),
        "frac_profitable_after_friction": round(
            sum(1 for a in amps_price if (a if want_up else -a) > 2 * FRICTION_LEG) / len(amps_price), 4),
    }


def z_vs_random(n: int, rate: float) -> float:
    if n == 0:
        return 0.0
    return (rate - 0.5) / math.sqrt(0.25 / n)


def analyze_symbol(sym: str) -> dict:
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, _years = load_ohlc(path)
    n = len(closes)
    t0 = time.time()
    tape = compute_organic_signals(opens, highs, lows, closes, dir_flips=[], require_settled=True)
    print(f"[{sym}] signal layer {time.time()-t0:.1f}s bars={n:,}", flush=True)
    by_lev = collect_bsps(tape)

    levels = {}
    for lad, seq in sorted(by_lev.items()):
        name = LADDER_NAMES.get(lad, str(lad))
        n_buy = sum(1 for r in seq if r[1] == "buy")
        n_sell = sum(1 for r in seq if r[1] == "sell")
        bs = pair_amplitudes(seq, closes, "buy", "sell")   # 买→卖（期望涨）
        sb = pair_amplitudes(seq, closes, "sell", "buy")   # 卖→买（期望跌）
        # 分 type 的 buy→sell（只看买点 type）
        by_type = {}
        for kt in ("type1", "type2", "type3"):
            sub = [r for r in seq if not (r[1] == "buy" and r[2] != kt)]
            # 保留所有 sell + 仅该 type 的 buy
            sub = [r for r in seq if r[1] == "sell" or (r[1] == "buy" and r[2] == kt)]
            res = pair_amplitudes(sub, closes, "buy", "sell")
            if res["n"]:
                by_type[kt] = res
        levels[name] = {
            "n_buy": n_buy, "n_sell": n_sell,
            "buy_to_sell": bs, "sell_to_buy": sb,
            "buy_to_sell_z": round(z_vs_random(bs.get("n", 0), bs.get("dir_correct_rate", 0.5)), 2),
            "buy_to_sell_by_type": by_type,
        }
        if bs.get("n"):
            print(f"  [{sym}] {name:10} buy→sell n={bs['n']} dir_correct={bs['dir_correct_rate']} "
                  f"z={levels[name]['buy_to_sell_z']} gross={bs['gross_amp_price_pct']}% "
                  f"net={bs['net_after_friction_pct']}% med_abs={bs['median_abs_amp_pct']}%", flush=True)
    return {"symbol": sym, "n_bars": n, "levels": levels}


def main() -> None:
    for sym in SYMBOLS:
        try:
            out = analyze_symbol(sym)
        except Exception as e:  # noqa: BLE001
            import traceback
            traceback.print_exc()
            out = {"symbol": sym, "failed": f"{type(e).__name__}: {e}"}
        (DATA_DIR / f"_bsp_pairing_{sym}.json").write_text(
            json.dumps(out, ensure_ascii=False, indent=1))
    print("DONE", flush=True)


if __name__ == "__main__":
    main()
