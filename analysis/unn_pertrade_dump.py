"""unn 逐笔交易导出（交易行为分析用）。

复用 backtest_unn 的 batch 路径（compute_organic_signals → pack_tape →
run_positional_rust(mode="unn") → analyze），导出**每笔** trade 的
source_ladder / 持仓时间 / 年份 / pnl / exit_reason，供 BTC/ES 踏空归因。

每笔 trade = 一个 voice 的生命周期（entry→exit）。exit_reason 映射操作：
  flip_short=C翻空 / flip_long=C翻多 / recover=D回补 / liq=A强平 / eod=收尾。
ladder = voice 的 source 层（F 根入场层 / E spawn 子层 / 翻转层）。
分年 yearly + by_ladder 提供时间分辨（踏空发生在哪一段）。

用法（仓库根目录）：
    PYTHONPATH=src .venv/bin/python analysis/unn_pertrade_dump.py BTC ES
输出：analysis/data_cache/unn_pertrade_<SYM>.json
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT))
sys.path.insert(0, str(REPO_ROOT / "src"))
sys.path.insert(0, str(REPO_ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from nested_recursive_fugue_final_backtest import FLOOR, LADDER_NAMES, analyze  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

OUT = REPO_ROOT / "analysis" / "data_cache"


def dump(sym: str) -> None:
    o, h, l, c, years = load_ohlc(SYMBOL_FILES[sym])
    n = len(c)
    y0 = years[0] if years else "?"
    y1 = years[-1] if years else "?"
    print(f"{sym}: {n:,} bars  years {y0}..{y1}")
    dir_flips: list = []
    t0 = time.time()
    tape = compute_organic_signals(o, h, l, c, dir_flips=dir_flips, require_settled=True)
    rtape = pack_tape(tape, dir_flips=dir_flips)
    res = nr.run_positional_rust(rtape, floor_ladder=FLOOR, mode="unn")
    a = analyze(res, c, years=years)
    print(f"  signals+engine {time.time() - t0:.1f}s  "
          f"strat={a['strat_pct']:+.1f}%  n_trades={len(res['trades'])}")

    per_trade = []
    for (lad, eb, ep, xb, xp, sh, _w, dfr, part, reason, pol) in res["trades"]:
        pnl = sh * (xp - ep) if pol == "long" else sh * (ep - xp)
        per_trade.append({
            "ladder": lad,
            "name": LADDER_NAMES.get(lad, str(lad)),
            "pol": pol,
            "entry_bar": eb,
            "exit_bar": xb,
            "held": xb - eb,
            "entry_year": years[min(eb, n - 1)] if years else None,
            "exit_year": years[min(xb, n - 1)] if years else None,
            "entry_px": round(ep, 4),
            "exit_px": round(xp, 4),
            "shares": round(sh, 6),
            "pnl": round(pnl, 1),
            "reason": reason,
            "deferred": dfr,
            "partial": part,
        })

    out = {
        "symbol": sym,
        "n_bars": n,
        "year_range": [y0, y1],
        "strat_pct": a["strat_pct"],
        "mdd_pct": a["mdd_pct"],
        "n_trades": a["n_trades"],
        "exit_reasons": a["exit_reasons"],
        "yearly": a["yearly"],
        "by_ladder": a["by_ladder"],
        "nrf_counters": a["nrf_counters"],
        "trades": per_trade,
    }
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / f"unn_pertrade_{sym}.json"
    path.write_text(json.dumps(out, ensure_ascii=False, indent=1))
    print(f"  → {path}")


if __name__ == "__main__":
    syms = sys.argv[1:] or ["BTC", "ES"]
    for s in syms:
        if s not in SYMBOL_FILES:
            print(f"标的 {s} 无数据文件映射（可用: {list(SYMBOL_FILES)}）", file=sys.stderr)
            continue
        dump(s)
