"""事件 delta 接口 bit-exact 验证（一次性脚本）。

两个 orchestrator 锁步喂同一数据：A 走新 delta 接口，B 走旧全量 marshal+扫描。
每个 stroke 增长 bar 比对 (布尔×4, BSP事件流, 背驰行流)，不等即 RuntimeError。

用法：PYTHONPATH=src .venv/bin/python analysis/_verify_event_delta.py OKLO 120000
"""
from __future__ import annotations

import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from organic_signals import _scan_events_rust  # noqa: E402
from per_level_bsp import BI_ZHONGSHU_LEVEL_ID  # noqa: E402

SYM = sys.argv[1]
N = int(sys.argv[2])
MAX_LEVELS = 8

opens, highs, lows, closes, _years = load_ohlc(SYMBOL_FILES[SYM])
opens, highs, lows, closes = opens[:N], highs[:N], lows[:N], closes[:N]
print(f"{SYM} bars={len(closes):,}", flush=True)

orch_new = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)
orch_old = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)

seen_old: set = set()
div_seen_old: set = set()
trend_seen_old: set = set()
trend_div_seen_old: set = set()
last_sc = 0
last_epoch = -1
n_cmp = n_ev = n_dv = n_tev = n_tdv = 0
t0 = time.time()


def _dedup_rows(rows, seen):
    out = []
    for row in rows:
        key = (row[0], row[1], row[2])
        if key in seen:
            continue
        seen.add(key)
        out.append(tuple(row))
    return out


for i in range(len(closes)):
    o, h, l, c = opens[i], highs[i], lows[i], closes[i]
    orch_new.process_bar(o, h, l, c)
    orch_old.process_bar(o, h, l, c)

    # ── ladder3 走势级（bsp_epoch 门控，与 organic_signals 同步点）──
    epoch = orch_new.bsp_epoch()
    if epoch != last_epoch:
        last_epoch = epoch
        tb1, ts1, tsa, tba, tev_new = orch_new.take_trend_bsp_events()
        ob = _scan_events_rust(orch_old.current_buysellpoints(), trend_seen_old)
        if (tb1, ts1, tsa, tba) != ob[:4]:
            raise RuntimeError(f"bar{i}: trend布尔不等 new={(tb1,ts1,tsa,tba)} old={ob[:4]}")
        if [tuple(e) for e in tev_new] != [tuple(e) for e in ob[4]]:
            raise RuntimeError(f"bar{i}: trend事件流不等\n new={tev_new}\n old={ob[4]}")
        tdv_new = [tuple(r) for r in orch_new.take_trend_div_events()]
        tdv_old = _dedup_rows(orch_old.current_trend_divergences(), trend_div_seen_old)
        if tdv_new != tdv_old:
            raise RuntimeError(f"bar{i}: trend背驰行不等\n new={tdv_new}\n old={tdv_old}")
        n_tev += len(tev_new)
        n_tdv += len(tdv_new)

    sc = orch_new.stroke_count()
    if sc <= last_sc:
        continue
    last_sc = sc

    new_pack = orch_new.take_bi_zhongshu_bsp_events(BI_ZHONGSHU_LEVEL_ID)
    b1, s1, sa, ba, evs_new = new_pack
    bsps_old = orch_old.current_bi_zhongshu_buysellpoints_inc(BI_ZHONGSHU_LEVEL_ID)
    ob1, os1, osa, oba, evs_old = _scan_events_rust(bsps_old, seen_old)
    if (b1, s1, sa, ba) != (ob1, os1, osa, oba):
        raise RuntimeError(f"bar{i}: 布尔不等 new={(b1,s1,sa,ba)} old={(ob1,os1,osa,oba)}")
    if [tuple(e) for e in evs_new] != [tuple(e) for e in evs_old]:
        raise RuntimeError(
            f"bar{i}: 事件流不等\n new={evs_new}\n old={evs_old}")

    dv_new = [tuple(r) for r in orch_new.take_bi_zhongshu_div_events(BI_ZHONGSHU_LEVEL_ID)]
    dv_old = _dedup_rows(
        orch_old.current_bi_zhongshu_divergences_inc(BI_ZHONGSHU_LEVEL_ID),
        div_seen_old)
    if dv_new != dv_old:
        raise RuntimeError(f"bar{i}: 背驰行不等\n new={dv_new}\n old={dv_old}")

    n_cmp += 1
    n_ev += len(evs_new)
    n_dv += len(dv_new)

print(f"PASS 对比点={n_cmp:,} L2事件={n_ev:,} L2背驰={n_dv:,} "
      f"L3事件={n_tev:,} L3背驰={n_tdv:,} 耗时{time.time()-t0:.1f}s", flush=True)
