"""螺旋引擎 v2（spiral）× 统一必然性引擎（unn）逐 bar bit-exact 对账（架构 §2.4 判据2）。

═══════════════ 验证命题（架构 §2.4 判据2）═══════════════

"操作 = 群作用" 的 L2 否证测试：spiral 引擎除**群作用路由层**（C 翻转走
`GroupAction::ChiralSeam`、E spawn 走 `σ⁻¹∘τ`）外，信号层/会计层/成本门与 unn
**完全一致**（spiral 复用 unn 的 `CenterBook`/`DepthRef` 成本门机件）。故：

  - bit-exact 通过 ⟹ 群作用抽象足以表达全部已验证操作（除 A 强平非群，gap G2）
    = "操作=群作用" 的 L2 验证。
  - bit-exact 失败 ⟹ 逐处归因：差异落在群作用层 = 命题被否证；落在已知 gap = 标注。

═══════════════ 方法（apples-to-apples）═══════════════

单个 `StreamingSignalReader` 逐 bar 产出 sig（缠论增量信号），**同一 sig** 喂给
`UnnStream` 与 `SpiralStream`（`push_signal` 因两者 `push_bar` 签名相同而通用）。
逐 bar 比对吐单 + finish 后比对完整 trade 表 + final_nav。

用法（仓库根目录）：
    PYTHONPATH=src .venv/bin/python trading_system/compare_spiral_unn.py --symbol OKLO --bars 30000
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(REPO_ROOT / "src"))
sys.path.insert(0, str(REPO_ROOT / "analysis"))

import time  # noqa: E402

import newchan_rust as nr  # noqa: E402
from nautilus_trader.model.data import BarType  # noqa: E402

from nested_recursive_fugue_final_backtest import FLOOR  # noqa: E402
from organic_signals import StreamingSignalReader, push_signal  # noqa: E402

from trading_system.backtest_unn import load_bars  # noqa: E402
from trading_system.config.instruments import INSTRUMENTS, make_instrument  # noqa: E402


def compare(sym: str, max_bars: int | None) -> int:
    spec = INSTRUMENTS[sym]
    instrument = make_instrument(spec)
    bar_type = BarType.from_str(f"{instrument.id}-1-MINUTE-LAST-EXTERNAL")
    print(f"加载 {sym}（max_bars={max_bars or '全量'}）...")
    t0 = time.time()
    _bars, opens, highs, lows, closes = load_bars(
        sym, bar_type, spec.price_precision, max_bars, instrument.size_precision)
    n = len(closes)
    print(f"  {n:,} bar  {time.time() - t0:.1f}s")

    dir_flips: list = []
    reader = StreamingSignalReader(dir_flips=dir_flips, require_settled=True)
    unn = nr.UnnStream(floor_ladder=FLOOR)
    spiral = nr.SpiralStream(floor_ladder=FLOOR)

    first_div = None       # 第一处逐 bar 吐单分歧 (bar, unn_trades, spiral_trades)
    n_bar_div = 0          # 逐 bar 吐单分歧次数
    n_unn_pushed = 0
    n_spiral_pushed = 0

    print("逐 bar 流式对账（同一 sig → UnnStream + SpiralStream）...")
    t0 = time.time()
    for i in range(n):
        n_before = len(dir_flips)
        sig = reader.process_bar(i, opens[i], highs[i], lows[i], closes[i])
        flip_rows = [(lad, d) for (_b, lad, d) in dir_flips[n_before:]]
        tu = push_signal(unn, sig, flip_rows)
        ts = push_signal(spiral, sig, flip_rows)
        n_unn_pushed += len(tu)
        n_spiral_pushed += len(ts)
        if tu != ts:
            n_bar_div += 1
            if first_div is None:
                first_div = (i, tu, ts)
    print(f"  逐 bar 完成 {time.time() - t0:.1f}s")

    ru = unn.finish()
    rs = spiral.finish()
    tu_all = ru["trades"]
    ts_all = rs["trades"]

    # ── trade 表逐行比对（bit-exact = 完全相等）──
    trade_match = tu_all == ts_all
    n_first_trade_div = None
    if not trade_match:
        m = min(len(tu_all), len(ts_all))
        for j in range(m):
            if tu_all[j] != ts_all[j]:
                n_first_trade_div = j
                break
        if n_first_trade_div is None:
            n_first_trade_div = m  # 一方更长

    nav_u = ru["final_nav"]
    nav_s = rs["final_nav"]
    nav_match = nav_u == nav_s

    print("\n========== spiral × unn bit-exact 对账 ==========")
    print(f"标的            : {sym}（{instrument.id}）")
    print(f"bars            : {n:,}")
    print(f"unn   trades    : {len(tu_all)}  (逐 bar 吐 {n_unn_pushed})")
    print(f"spiral trades   : {len(ts_all)}  (逐 bar 吐 {n_spiral_pushed})")
    print(f"逐 bar 吐单分歧 : {n_bar_div} bar")
    print(f"trade 表 bit-exact : {'✅ PASS' if trade_match else '❌ MISMATCH'}")
    print(f"final_nav       : unn={nav_u:.6f}  spiral={nav_s:.6f}  {'✅' if nav_match else '❌ Δ=%.6f' % (nav_s - nav_u)}")

    if first_div is not None:
        bar, tud, tsd = first_div
        print(f"\n第一处逐 bar 分歧 @ bar {bar}:")
        print(f"  unn    吐: {tud}")
        print(f"  spiral 吐: {tsd}")
    if n_first_trade_div is not None:
        print(f"\n第一处 trade 表分歧 @ index {n_first_trade_div}:")
        if n_first_trade_div < len(tu_all):
            print(f"  unn   : {tu_all[n_first_trade_div]}")
        if n_first_trade_div < len(ts_all):
            print(f"  spiral: {ts_all[n_first_trade_div]}")

    verdict = trade_match and nav_match
    print(f"\n判决: {'✅ BIT-EXACT —— 操作=群作用 L2 验证通过（除 A 强平非群 gap G2）' if verdict else '❌ 分歧 —— 须逐处归因（群作用层=否证 / 已知 gap=标注）'}")
    return 0 if verdict else 2


def main() -> int:
    p = argparse.ArgumentParser(description="spiral × unn bit-exact 对账")
    p.add_argument("--symbol", default="OKLO")
    p.add_argument("--bars", type=int, default=30000, help="最大 bar 数（0=全量）")
    args = p.parse_args()
    if args.symbol not in INSTRUMENTS:
        print(f"标的 {args.symbol} 未注册（可用: {list(INSTRUMENTS)}）", file=sys.stderr)
        return 1
    return compare(args.symbol, args.bars or None)


if __name__ == "__main__":
    raise SystemExit(main())
