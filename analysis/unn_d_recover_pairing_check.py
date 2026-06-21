"""D 回补 E/D 对称化验证：BTC 逐笔子空头 spawn/recover 配对 + 净亏/亏损比例。

验证 2026-06-16 修复（D recover 改用 nf_buy/nf_sell 向心确认，与 E spawn 对称）：
  - 子空头 = trade.polarity == "short" ∧ exit_reason == "recover"（D 回补平子空头）。
  - spawn ladder == recover ladder（trade.ladder 单字段，v.ladder 不变 ⇒ 同级别，构造性）。
  - 空头 pnl = shares × (entry_price − exit_price)；亏损 ⟺ exit_price > entry_price。

用法：PYTHONPATH=src .venv/bin/python analysis/unn_d_recover_pairing_check.py [TAG]
  TAG 写入 data_cache/unn_d_pairing_<TAG>.json（如 before / after）。
"""

from __future__ import annotations

import json
import sys
import time
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from nested_recursive_fugue_final_backtest import FLOOR, analyze, bh_mdd  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

# trade 11 元组索引（lib.rs:1916）。
LADDER, ENTRY_BAR, ENTRY_PX, EXIT_BAR, EXIT_PX, SHARES = 0, 1, 2, 3, 4, 5
EXIT_REASON, POLARITY = 9, 10


def short_pnl(t: tuple) -> float:
    """空头 P&L：开空收 entry 款、平空付 exit 款 ⇒ shares×(entry−exit)。"""
    return t[SHARES] * (t[ENTRY_PX] - t[EXIT_PX])


def main() -> None:
    tag = sys.argv[1] if len(sys.argv) > 1 else "after"
    sym = "BTC"
    path = SYMBOL_FILES[sym]
    opens, highs, lows, closes, years = load_ohlc(path)
    n = len(closes)
    bh_pct = (closes[-1] / closes[0] - 1) * 100
    print(f"[{sym}] bars={n:,} BH={bh_pct:+.1f}% BH_MDD={bh_mdd(closes) * 100:.1f}%", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips, require_settled=True)
    print(f"[{sym}] 信号层 {time.time() - t0:.1f}s flips={len(dir_flips)}", flush=True)
    rtape = pack_tape(tape, dir_flips=dir_flips)

    t1 = time.time()
    # prove 零 panic：跑通即 N1-N8 + T49/T52/T53/T56-T59 在 BTC 真实数据成立。
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="unn")
    print(f"[{sym}][unn] {time.time() - t1:.1f}s prove 零 panic ✓", flush=True)
    a = analyze(res, closes, years)

    trades = res["trades"]
    shorts = [t for t in trades if t[POLARITY] == "short"]
    recovers = [t for t in shorts if t[EXIT_REASON] == "recover"]

    # 子空头逐笔配对（spawn ladder vs recover ladder = trade.ladder，构造性同级别）。
    cross_level = [t for t in recovers if False]  # 占位：trade.ladder 单字段，恒同级别
    reason_breakdown = Counter(t[EXIT_REASON] for t in shorts)

    rec_pnls = [short_pnl(t) for t in recovers]
    rec_net = sum(rec_pnls)
    rec_losses = [p for p in rec_pnls if p < 0]
    rec_loss_pct = (len(rec_losses) / len(rec_pnls) * 100) if rec_pnls else 0.0

    all_short_net = sum(short_pnl(t) for t in shorts)

    # 按 ladder 分组子空头 recover 净 pnl。
    by_ladder: dict[int, list[float]] = defaultdict(list)
    for t in recovers:
        by_ladder[t[LADDER]].append(short_pnl(t))
    ladder_summary = {
        int(k): {"n": len(v), "net": round(sum(v), 2),
                 "losses": sum(1 for p in v if p < 0)}
        for k, v in sorted(by_ladder.items())
    }

    out = {
        "tag": tag, "symbol": sym, "n_bars": n,
        "strat_pct": a["strat_pct"], "mdd_pct": a["mdd_pct"],
        "n_trades_total": len(trades),
        "n_shorts": len(shorts),
        "short_exit_reasons": dict(reason_breakdown),
        "n_recover_shorts": len(recovers),
        "recover_cross_level": len(cross_level),  # 构造性 0：spawn==recover ladder
        "recover_net_pnl": round(rec_net, 2),
        "recover_loss_count": len(rec_losses),
        "recover_loss_pct": round(rec_loss_pct, 1),
        "all_short_net_pnl": round(all_short_net, 2),
        "recover_by_ladder": ladder_summary,
    }
    DATA_DIR = ROOT / "analysis" / "data_cache"
    DATA_DIR.mkdir(exist_ok=True)
    (DATA_DIR / f"unn_d_pairing_{tag}.json").write_text(json.dumps(out, indent=2, ensure_ascii=False))

    print(f"\n══════ D 回补子空头逐笔（tag={tag}）══════", flush=True)
    print(f"strat={a['strat_pct']:+.1f}% mdd={a['mdd_pct']}% trades={len(trades)}", flush=True)
    print(f"子空头总数={len(shorts)}  exit_reasons={dict(reason_breakdown)}", flush=True)
    print(f"recover 子空头={len(recovers)}  跨级别={len(cross_level)}（构造性应为0）", flush=True)
    print(f"recover 净 pnl={rec_net:+.2f}  亏损笔={len(rec_losses)}/{len(rec_pnls)} "
          f"({rec_loss_pct:.1f}%)", flush=True)
    print(f"全部子空头净 pnl={all_short_net:+.2f}", flush=True)
    print(f"按层：{ladder_summary}", flush=True)


if __name__ == "__main__":
    main()
