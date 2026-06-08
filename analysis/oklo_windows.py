"""OKLO 多窗口 PH 分析 —— 在三个时间窗口上跑同一管线，隔离 SPAC 污染。

为什么分窗口（数据有效域，formalization-validity-domain）
--------------------------------------------------------
OKLO 经 SPAC（AltC/ALCC）借壳，2021-07~2024-04 共 712 根是空壳期（≈$10 横盘）。
全历史窗口 58% 是横盘噪声，会扭曲 ATR(τ)、persistence 分布与级别结构——这是
**数据污染**，不是 OKLO 的真实递归。故：
- W_full   : 全历史（含 SPAC，仅作污染对照，结论不采纳）
- W_despac : 借壳后真实交易（2024-05-01 起，521 根）—— 主分析窗口
- W_1y     : 近 1 年（2025-05-29 起，252 根）—— 与 MOS(1y/251 根) 严格可比

复用 oklo_ph_driver.analyze()，仅切片 data 字典。
"""
from __future__ import annotations

import bisect
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "analysis"))
sys.path.insert(0, str(ROOT / "src"))

from oklo_ph_driver import analyze  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"


def slice_from(data: dict, start_date: str) -> dict:
    i = bisect.bisect_left(data["dates"], start_date)
    return {
        "symbol": data["symbol"],
        "closes": data["closes"][i:],
        "highs": data["highs"][i:],
        "lows": data["lows"][i:],
        "opens": data["opens"][i:],
        "dates": data["dates"][i:],
    }


def brief(tag: str, r: dict) -> None:
    m = r["meta"]
    pct = (m["price_end"] / m["price_start"] - 1) * 100
    lr, la = r["levels_recursive"], r["levels_adaptive"]
    o, bp = r["online"], r["buypoint_score"]
    th = o["trend_health"]
    print(f"\n{'='*72}\n{tag}  n={m['n_bars']}  {m['date_start']}→{m['date_end']}\n{'='*72}")
    print(f"价格 {m['price_start']}→{m['price_end']} [{m['price_min']},{m['price_max']}] {pct:+.1f}%  τ={m['tau_atr']}")
    print(f"H0: {r['barcode_h0']['n_total']}特征 {r['barcode_h0']['n_active']}活跃 max={r['barcode_h0']['max_persistence']}")
    print(f"H1 loop: {r['barcode_h1_loop']['n_total']} {r['barcode_h1_loop']['n_active']}活跃 max={r['barcode_h1_loop']['max_persistence']}")
    print(f"级别-递归 {lr['n_levels']}级 叶={[L['label'] for L in lr['detail'] if not L['children']]}")
    print(f"级别-自适应 {la['n_levels']}级 {[L['label'] for L in la['detail']]}")
    print(f"中枢 {r['zhongshu']['count']}个: " +
          "; ".join(f"{z['label']}[{z['zd']},{z['zg']}]{z['date_lo']}→{z['date_hi']}({z['n_members']})"
                    for z in r["zhongshu"]["detail"]))
    print(f"在线 alive={o['n_alive_live']}/settled={o['n_settled_live']} "
          f"growth={th['persistence_growth_rate'] if th else '?'} healthy={th['healthy'] if th else '?'}")
    st = r["settle_triggers"]
    print(f"settle 近端={st['nearest']}")
    print(f"       主导 valley={st['dominant']['valley']} can_rebound={st['dominant']['can_settle_by_rebound']} "
          f"reversal_amp={st['dominant']['reversal_amplitude']}")
    print(f"买点 加权={bp['composite_weighted']} 乘积={bp['composite_product']} 五维={bp['dimensions']}")


def main() -> None:
    data = json.load(open(CACHE / "OKLO_1d_max.json"))
    windows = {
        "W_full (含SPAC, 污染对照)": data,
        "W_despac (借壳后, 主分析)": slice_from(data, "2024-05-01"),
        "W_1y (近1年, MOS可比)": slice_from(data, "2025-05-29"),
    }
    out: dict = {}
    for tag, wd in windows.items():
        r = analyze(wd)
        out[tag.split()[0]] = r
        brief(tag, r)
    (CACHE / "oklo_windows_result.json").write_text(
        json.dumps(out, ensure_ascii=False, indent=2))
    print(f"\n已存 analysis/data_cache/oklo_windows_result.json")


if __name__ == "__main__":
    main()
