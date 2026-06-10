"""有机赋格 Rust 交易层 O0≡P5 bit-exact 对账（v2 M1 对账门 1）。

对账面（v1R §7.1 三道面）：
  1. trades 全字段（entry/exit bar+price、pnl_pct、exit_reason、n_short_diffs、
     cost_basis_at_exit）——Python O0 vs Rust O0 逐元组 ==（浮点 bit 级）。
  2. diag trace 12 字段逐条（账本算术逐步对账——trades 相同而 trace 不同 =
     误差抵消假象）。
  3. counters 18 键逐位（控制流逐分支同构证明）。
传递性：Python O0 ≡ P5 已有守卫（organic_fugue_backtest）⇒ Rust O0 ≡ P5。

用法：PYTHONPATH=src .venv/bin/python analysis/organic_fugue_rust_check.py
      （env：BT_SYMBOLS=OKLO 默认；逗号分隔多标的）
"""

from __future__ import annotations

import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from fugue_version_i import LADDER_SEG, MAX_LADDER  # noqa: E402
from organic_fugue import ORGANIC_VARIANTS, run_organic  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("BT_SYMBOLS", "OKLO").split(",")]
FLOOR = LADDER_SEG


def pack_tape(tape, dir_flips: list | None = None) -> "nr.OrganicTape":
    """BarSignalI 列表 → 列式数组 → Rust OrganicTape（一次 marshal）。

    dir_flips：v2 D3 稀疏方向行（compute_organic_signals 收集器产出），
    None = 不传（O0/V3p 不需要；REV 变体在 Rust 侧被 capability guard 拒绝）。
    """
    n = len(tape)
    closes = [s.close for s in tape]

    def mask(rows) -> int:
        m = 0
        for k, v in enumerate(rows):
            if v:
                m |= 1 << k
        return m

    buy1 = [mask(s.buy1) for s in tape]
    sell1 = [mask(s.sell1) for s in tape]
    sell_any = [mask(s.sell_any) for s in tape]
    buy_any = [mask(s.buy_any) for s in tape]
    up_settled = [mask(s.up_move_settled) if s.up_move_settled else 0 for s in tape]
    max_ladder = [s.max_ladder for s in tape]
    type2 = [bool(s.type2_buy) for s in tape]

    bsp_flat = []
    div_flat = []
    for i, s in enumerate(tape):
        if s.bsp_events:
            for lad in range(MAX_LADDER):
                for e in s.bsp_events[lad]:
                    # (kind, side, seg_idx, confirmed, cs, zd, zg, price)
                    bsp_flat.append((i, lad, e[0], e[1], e[2], bool(e[3]),
                                     e[4], e[5], e[6], e[7]))
        if s.div_events:
            for lad in range(MAX_LADDER):
                for d in s.div_events[lad]:
                    # (kind, direction, side, seg_idx, fa, fc, price) — side 不传
                    div_flat.append((i, lad, d[0], d[1], d[3], d[4], d[5], d[6]))
    print(f"  打包：{n:,} bars，bsp 事件 {len(bsp_flat):,}，div 事件 {len(div_flat):,}",
          flush=True)
    return nr.OrganicTape.from_columns(
        closes, buy1, sell1, sell_any, buy_any, up_settled, max_ladder, type2,
        bsp_flat, div_flat, dir_flips=dir_flips)


def check_symbol(symbol: str) -> None:
    print(f"\n{'=' * 64}\n  {symbol} — Rust O0 ≡ Python O0 对账\n{'=' * 64}", flush=True)
    opens, highs, lows, closes, _years = load_ohlc(SYMBOL_FILES[symbol])
    t0 = time.time()
    tape = compute_organic_signals(opens, highs, lows, closes)
    print(f"  信号层 {time.time() - t0:.1f}s（{len(tape):,} bars）", flush=True)

    # ── Python O0（oracle）──
    t0 = time.time()
    diag_py: list = []
    trades_py, extra_py = run_organic(
        tape, floor_ladder=FLOOR, config=ORGANIC_VARIANTS["O0"], diag=diag_py)
    print(f"  Python O0 {time.time() - t0:.1f}s：{len(trades_py)} 笔", flush=True)

    # ── Rust O0 ──
    rtape = pack_tape(tape)
    t0 = time.time()
    res = nr.run_organic_rust(rtape, "O0", floor_ladder=FLOOR,
                              stop_mode="none", diag=True)
    print(f"  Rust  O0 {time.time() - t0:.3f}s：{len(res['trades'])} 笔", flush=True)

    # ── 面1：trades 全字段 ──
    if len(trades_py) != len(res["trades"]):
        raise SystemExit(
            f"FAIL trades 笔数：py {len(trades_py)} vs rust {len(res['trades'])}")
    for ti, (tp, tr) in enumerate(zip(trades_py, res["trades"])):
        py_tuple = (tp.entry_bar, tp.entry_price, tp.exit_bar, tp.exit_price,
                    tp.pnl_pct, tp.exit_reason, tp.n_short_diffs,
                    tp.cost_basis_at_exit)
        if py_tuple != tuple(tr):
            raise SystemExit(f"FAIL trade#{ti}:\n  py   {py_tuple}\n  rust {tuple(tr)}")
    print(f"  [面1 trades] PASS（{len(trades_py)} 笔逐字段 ==）", flush=True)

    # ── 面2：diag trace 12 字段逐条 ──
    rd = res["diag"]
    if len(diag_py) != len(rd):
        raise SystemExit(f"FAIL diag 条数：py {len(diag_py)} vs rust {len(rd)}")
    n_rows = 0
    for ti, (dp, (header, diffs)) in enumerate(zip(diag_py, rd)):
        if dp["n_diffs"] != len(diffs):
            raise SystemExit(
                f"FAIL trade#{ti} 短差数：py {dp['n_diffs']} vs rust {len(diffs)}")
        hd = (dp["entry_bar"], dp["entry_price"], dp["exit_bar"], dp["exit_price"],
              dp["exit_reason"], dp["entry_ladder"], dp["pnl_pct"],
              dp["cost_basis_exit"], dp["total_shares_exit"],
              bool(dp["reached_earning"]))
        if hd != tuple(header):
            raise SystemExit(f"FAIL trade#{ti} 头部:\n  py   {hd}\n  rust {tuple(header)}")
        for ri, (rp, ((key, leg, sb, sp, bb, bp), (sh, df, pf, we, sd, cb0, cb1))) \
                in enumerate(zip(dp["diffs"], diffs)):
            rust_row = {"ladder": key, "leg": leg, "sell_bar": sb, "sell_price": sp,
                        "buy_bar": bb, "buy_price": bp, "shares": sh, "diff": df,
                        "profit": pf, "was_earning": we, "shares_delta": sd,
                        "cost_basis_before": cb0, "cost_basis_after": cb1}
            for k in rp:
                if rp[k] != rust_row[k]:
                    raise SystemExit(
                        f"FAIL trade#{ti} diff#{ri} 字段 {k}: "
                        f"py {rp[k]!r} != rust {rust_row[k]!r}")
            n_rows += 1
    print(f"  [面2 trace ] PASS（{n_rows} 行 × 12 字段逐位 ==）", flush=True)

    # ── 面3：counters 18 键 ──
    cpy = extra_py["counters"]
    crs = res["counters"]
    for k, v in cpy.items():
        if v != crs[k]:
            raise SystemExit(f"FAIL counter {k}: py {v} vs rust {crs[k]}")
    print(f"  [面3 counters] PASS（{len(cpy)} 键逐位 ==）", flush=True)

    # ── 附加：归因表对账 ──
    att_py = extra_py["ladder_attribution"]
    from fugue_version_i import ladder_name
    att_rs = {ladder_name(k): v for k, v in enumerate(res["ladder_attribution"]) if v}
    if att_py != att_rs:
        raise SystemExit(f"FAIL ladder_attribution: py {att_py} vs rust {att_rs}")
    lc_py = extra_py["leg_contribution"]
    lc_rs: dict = {}
    for key, cash, cnt in res["leg_contribution"]:
        name = f"{'rev' if key >= 200 else 'osc' if key >= 100 else 'main'}:" \
               f"{ladder_name(key % 100)}"
        d = lc_rs.setdefault(name, {"recovered_cash": 0.0, "n_short_diffs": 0})
        d["recovered_cash"] = round(d["recovered_cash"] + cash, 1)
        d["n_short_diffs"] += cnt
    if lc_py != lc_rs:
        raise SystemExit(f"FAIL leg_contribution:\n  py   {lc_py}\n  rust {lc_rs}")
    print("  [附加 归因 ] PASS（ladder_attribution + leg_contribution）", flush=True)
    print(f"\n  ✅ {symbol}: Rust O0 ≡ Python O0 ≡ P5（传递性）", flush=True)


if __name__ == "__main__":
    for sym in SYMBOLS:
        check_symbol(sym)
