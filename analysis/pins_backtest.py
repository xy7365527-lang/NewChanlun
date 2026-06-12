"""PINS（Pinterest）1min 全历史 V2oa25_ht 回测 — BH / V0(P5) / V2oa25 / V2oa25_ht。

数据：pins_1m_databento_full.json（XNAS.ITCH ohlcv-1m，2019-04-18 IPO 起，
未拆股原始价，fetch_pins_databento.py 产出，load_ohlc parallel-array 分支）。

变体（单链消融，与 master_exit_mode_backtest 同磁带链路）：
  BH        = 买入持有基准（首 bar 买入末 bar 清仓）
  V0        = P5 逐位（有机赋格基线，无 REV）
  V2oa25    = 满仓入场 + θ自适应(q=0.25/w=50) + 成本门(2×10bps下界)
              + 41课门 + 配对 REV（Signal 出场）
  V2oa25_ht = V2oa25 + HoldTrend 出场（父级别趋势未衰竭不出）

认识论等级：L2（单标的真实数据，新资产域——社交媒体股，IPO 后破发长熊 +
2021 泡沫顶回落，与 OKLO 强趋势/BTC 单边形成 regime 对照）。

输出：analysis/data_cache/pins_backtest.json（增量续跑）
用法：PYTHONPATH=src .venv/bin/python analysis/pins_backtest.py
"""

from __future__ import annotations

import json
import os
import sys
import time
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_v2_full_backtest import load_ohlc  # noqa: E402
from fugue_version_i import LADDER_SEG, extended_metrics  # noqa: E402
from organic_fugue_rust_backtest import _trades_from_rust  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
DATA_JSON = DATA_DIR / "pins_1m_databento_full.json"
OUT_JSON = DATA_DIR / "pins_backtest.json"

FLOOR = int(os.environ.get("BT_FLOOR", str(LADDER_SEG)))
VARIANTS = ["V0", "V2oa25", "V2oa25_ht"]


def run_cell(rtape, variant: str, years, bh: float, n_bars: int) -> dict:
    t0 = time.time()
    res = nr.run_organic_rust(rtape, variant, floor_ladder=FLOOR,
                              stop_mode="none", diag=False)
    el = time.time() - t0
    trades = _trades_from_rust(res["trades"])
    m = extended_metrics(trades, years)
    raw = res["trades"]
    hold_bars = sum(t[2] - t[0] for t in raw)
    return {
        "metrics": {k: m[k] for k in ("n", "win_rate", "total_compound",
                                      "sharpe", "max_dd")},
        "delta_vs_bh": round(m["total_compound"] - bh, 1),
        "first_entry_bar": raw[0][0] if raw else None,
        "exit_reasons": dict(Counter(t[5] for t in raw)),
        "avg_hold_bars": round(hold_bars / len(raw), 1) if raw else None,
        "exposure": round(hold_bars / n_bars, 4),
        "n_exit_trend_holds": res["counters"].get("n_exit_trend_holds", 0),
        "n_short_diffs_total": sum(t[6] for t in raw),
        "elapsed_s": round(el, 3),
    }


def main() -> None:
    prev: dict = json.loads(OUT_JSON.read_text()) if OUT_JSON.exists() else {}
    force = os.environ.get("BT_FORCE", "0") == "1"

    opens, highs, lows, closes, years = load_ohlc(DATA_JSON)
    n = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"PINS bars={n:,}  BH={bh:+.2f}%  floor={FLOOR}", flush=True)

    t0 = time.time()
    dir_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips)
    sig_s = time.time() - t0
    print(f"信号层 {sig_s:.1f}s  D3 翻转 {len(dir_flips):,}", flush=True)

    fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                     for l in range(11)),
          "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                     for l in range(11)),
          "flips": len(dir_flips)}
    if prev.get("tape_fp") not in (None, fp):
        raise SystemExit(f"磁带指纹不匹配：在册 {prev['tape_fp']} vs 本次 {fp}"
                         f"——引擎/信号层语义已变，BT_FORCE=1 全量重跑。")

    rtape = pack_tape(tape, dir_flips=dir_flips)

    out = prev
    out.update({"n_bars": n, "bh": round(bh, 2),
                "sig_elapsed": round(sig_s, 1), "tape_fp": fp,
                "floor": FLOOR})
    out.setdefault("cells", {})
    base_pack = None
    for variant in VARIANTS:
        if variant in out["cells"] and not force:
            pack = out["cells"][variant]
        else:
            pack = run_cell(rtape, variant, years, bh, n)
            out["cells"][variant] = pack
        if variant == "V2oa25":
            base_pack = pack
        elif variant == "V2oa25_ht" and base_pack is not None:
            # 首笔入场 bar 同一性守卫（exit_mode 单轴，入场侧零改动）
            if pack["first_entry_bar"] != base_pack["first_entry_bar"]:
                raise SystemExit(
                    f"FAIL 首笔入场同一性：V2oa25 {base_pack['first_entry_bar']}"
                    f" vs _ht {pack['first_entry_bar']}")
        pack["delta_vs_v2oa25"] = (
            round(pack["metrics"]["total_compound"]
                  - base_pack["metrics"]["total_compound"], 1)
            if base_pack is not None and variant != "V2oa25" else None)
        m = pack["metrics"]
        print(f"[{variant:10s}] 复利={m['total_compound']:+10.2f}%"
              f" Δ(BH)={pack['delta_vs_bh']:+8.1f}pp 笔={m['n']:4d}"
              f" 胜率={m['win_rate']:.2f} 夏普={m['sharpe']:+.3f}"
              f" MDD={m['max_dd']:.1f}% 暴露={pack['exposure']:.2f}"
              f" trend_holds={pack['n_exit_trend_holds']}"
              f" [{pack['elapsed_s']:.2f}s]", flush=True)
        OUT_JSON.write_text(json.dumps(out, indent=2, ensure_ascii=False))
    print("[done] 已落盘", flush=True)


if __name__ == "__main__":
    main()
