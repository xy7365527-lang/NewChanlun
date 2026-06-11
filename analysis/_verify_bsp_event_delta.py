"""delta 事件接口 ≡ 全量 marshal+扫描 差分验证（2026-06-11 O(S×B) 修复守卫）。

双 orchestrator 同步驱动真实数据：A 走新 delta 接口
（take_bi_zhongshu_bsp_events / take_bi_zhongshu_div_events），B 走旧路径
（current_bi_zhongshu_buysellpoints_inc 全量 marshal + _scan_events_rust /
current_bi_zhongshu_divergences_inc + seen 过滤）。每个 stroke 增长 bar
逐位比较布尔 + 事件行 + 背驰行。任何不等 → 立即报错退出。

用法：PYTHONPATH=src .venv/bin/python analysis/_verify_bsp_event_delta.py
      （env：VER_SYMBOL=BRN VER_BARS=600000）
"""
from __future__ import annotations

import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from organic_signals import _scan_events_rust  # noqa: E402
from per_level_bsp import BI_ZHONGSHU_LEVEL_ID  # noqa: E402

MAX_LEVELS = 6


def main() -> None:
    sym = os.environ.get("VER_SYMBOL", "BRN")
    cap = int(os.environ.get("VER_BARS", "600000"))
    o, h, l, c = load_ohlc(SYMBOL_FILES[sym])[:4]
    n = min(cap, len(c))
    print(f"{sym}: 差分验证前 {n:,} bars")

    oa = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)
    ob = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)
    seen_b: set = set()
    div_seen_b: set = set()
    last_sc = 0
    n_cmp = n_ev = n_dv = 0

    for i in range(n):
        oa.process_bar(o[i], h[i], l[i], c[i])
        ob.process_bar(o[i], h[i], l[i], c[i])
        sc = oa.stroke_count()
        if sc <= last_sc:
            continue
        last_sc = sc
        # ── A：delta 接口 ──
        a_b1, a_s1, a_sa, a_ba, a_evs = \
            oa.take_bi_zhongshu_bsp_events(BI_ZHONGSHU_LEVEL_ID)
        a_dvs = oa.take_bi_zhongshu_div_events(BI_ZHONGSHU_LEVEL_ID)
        # ── B：旧路径（全量 marshal + Python 扫描）──
        bsps = ob.current_bi_zhongshu_buysellpoints_inc(BI_ZHONGSHU_LEVEL_ID)
        b_b1, b_s1, b_sa, b_ba, b_evs = _scan_events_rust(bsps, seen_b)
        b_dvs = []
        for row in ob.current_bi_zhongshu_divergences_inc(BI_ZHONGSHU_LEVEL_ID):
            key = (row[0], row[1], row[2])
            if key in div_seen_b:
                continue
            div_seen_b.add(key)
            b_dvs.append(row)
        # ── 逐位比较 ──
        if (a_b1, a_s1, a_sa, a_ba) != (b_b1, b_s1, b_sa, b_ba):
            raise SystemExit(
                f"FAIL@bar{i} 布尔: delta={(a_b1, a_s1, a_sa, a_ba)} "
                f"ref={(b_b1, b_s1, b_sa, b_ba)}")
        if list(a_evs) != list(b_evs):
            raise SystemExit(
                f"FAIL@bar{i} 事件行: delta={a_evs}\nref={b_evs}")
        if list(a_dvs) != list(b_dvs):
            raise SystemExit(
                f"FAIL@bar{i} 背驰行: delta={a_dvs}\nref={b_dvs}")
        n_cmp += 1
        n_ev += len(a_evs)
        n_dv += len(a_dvs)
        if n_cmp % 10000 == 0:
            print(f"  [{i / n * 100:5.1f}%] 比较点 {n_cmp:,} "
                  f"事件 {n_ev:,} 背驰 {n_dv:,}", flush=True)

    print(f"PASS: {n_cmp:,} 个比较点全等（事件 {n_ev:,} 行 / 背驰 {n_dv:,} 行）")


if __name__ == "__main__":
    main()
