"""全量普适组合 1s 口径对照（BTC 两周窗口）——a0 分辨率差异标注。

任务（2026-06-12 全量普适组合）第三模块"1s a0"的 fallback 实测：BTC 1s
全史不可得（仅 btc_1s_2week.json，2026-05-29 → 2026-06-11，1,209,600 bar），
不支撑 P1-P6 全史判据 ⇒ 本探针是**口径标注**非判决。

对照设计：同一两周日历窗口，1s vs 1min 双 a0，各跑
{fusion_btra_s34, fusion_btran_s34}。读数：
  - strat/MDD（窗口太短，仅上下文）
  - nest 观测面（arms/fires/lead bars）——1s 塔下移后 nest 正向定位的
    触发密度与领先量变化（在册参照：1s 条件化嵌套确认加速 2.8-3.0×，
    `1s_nesting_confirmation` 在册）
  - 翻空/平空活动（s34 = move/recL2 层在 1min 两周窗几乎无事件——
    级别塔整体下移是 1s a0 的本质，秒级 a0 在册判决"纯观测分辨率"）

认识论等级：L2 单窗口 n=1 观测。不扩展不否证任何有效域。

用法：PYTHONPATH=src .venv/bin/python analysis/universal_combo_1s_probe.py
输出：data_cache/universal_combo_1s_probe.json
"""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_version_i import LADDER_SEG  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402
from universal_combination_backtest import analyze  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
MODES = ["fusion_btra_s34", "fusion_btran_s34"]


def load_1s():
    d = json.loads((DATA_DIR / "btc_1s_2week.json").read_text())
    return d


def run_face(label: str, opens, highs, lows, closes) -> dict:
    n = len(closes)
    bh = (closes[-1] / closes[0] - 1) * 100
    print(f"[{label}] bars={n:,} BH={bh:+.2f}%", flush=True)
    t0 = time.time()
    dir_flips: list = []
    trend_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips,
                                   trend_flips=trend_flips)
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)
    print(f"[{label}] 信号层 {time.time() - t0:.1f}s", flush=True)
    out = {"n_bars": n, "bh_pct": round(bh, 2), "modes": {}}
    for mode in MODES:
        res = nr.run_positional_rust(rtape, floor_ladder=LADDER_SEG, mode=mode)
        a = analyze(res, closes, None)
        out["modes"][mode] = a
        g = a["gates"]
        print(f"[{label}][{mode}] strat={a['strat_pct']:+.2f}% "
              f"trades={a['n_trades']} flips={sum(g['flips'])} "
              f"nest_arms={sum(g['nest_arms'])} "
              f"nest_fires={sum(g['nest_fire_sell']) + sum(g['nest_fire_buy'])} "
              f"lead={g['nest_lead_avg_bars']}", flush=True)
    return out


def main() -> None:
    raw = load_1s()
    # 数据格式探测（在册两种：bars 数组 / opens 列族）
    if isinstance(raw, dict) and "opens" in raw:
        o1, h1, l1, c1 = (raw["opens"], raw["highs"], raw["lows"], raw["closes"])
        ts = raw.get("timestamps") or raw.get("ts")
    else:
        rows = raw["bars"] if isinstance(raw, dict) else raw
        o1 = [r["open"] if isinstance(r, dict) else r[1] for r in rows]
        h1 = [r["high"] if isinstance(r, dict) else r[2] for r in rows]
        l1 = [r["low"] if isinstance(r, dict) else r[3] for r in rows]
        c1 = [r["close"] if isinstance(r, dict) else r[4] for r in rows]
        ts = ([r["ts"] for r in rows] if isinstance(rows[0], dict) and "ts" in rows[0]
              else [r[0] for r in rows] if not isinstance(rows[0], dict) else None)
    out = {"design": "1s vs 1min 同窗口口径对照（L2 单窗口观测，非判决）",
           "faces": {}}
    out["faces"]["1s"] = run_face("BTC-1s", o1, h1, l1, c1)

    # 1min 同窗：从 1s 聚合（零外部依赖，窗口严格相同）
    o2, h2, l2, c2 = [], [], [], []
    for i in range(0, len(c1) - 59, 60):
        o2.append(o1[i])
        h2.append(max(h1[i:i + 60]))
        l2.append(min(l1[i:i + 60]))
        c2.append(c1[i + 59])
    out["faces"]["1min_agg"] = run_face("BTC-1min(agg)", o2, h2, l2, c2)

    (DATA_DIR / "universal_combo_1s_probe.json").write_text(
        json.dumps(out, ensure_ascii=False, indent=1))
    print(json.dumps({k: {m: v["modes"][m]["strat_pct"] for m in MODES}
                      for k, v in out["faces"].items()}, ensure_ascii=False),
          flush=True)


if __name__ == "__main__":
    main()
