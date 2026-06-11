"""P0-c — 逐 ladder 中枢振幅 / 交易成本比值表.

目的：一张表回答"在哪个级别做短差划算"。35课依据："交易成本+交易误差
相对波幅不够小的级别，长期操作没有意义"。

方法（全部末态批量读，无逐bar marshal——O(N²)陷阱规避）：
  1. RecursiveOrchestrator 流式跑完整递归塔（segment级增量化后 ~O(N)，
     CL 5.5M bar 实测可前台完成）。
  2. 末态一次性读各 ladder 中枢：
     - ladder2（笔中枢）：confirmed strokes → zhongshu_from_strokes 批量
       （orchestrator 不直接暴露笔中枢列表；与 _c_seg_fix_capture.py 同口径）
     - ladder3（走势级/线段中枢）：orch.current_zhongshus()
     - ladder4+（递归层）：orch.current_recursive() → ladder = lid + 2
  3. 中枢振幅 = (ZG−ZD) / ((ZG+ZD)/2) × 100（相对值 %）。
     ZG/ZD 由前三构件固定（中枢定义），生长不改变——unsettled 尾中枢
     振幅同样良定义，全部计入（每层至多 1 个 unsettled，N 大时可忽略）。
  4. 比值 = 振幅P50 / 往返成本(10bps=0.10%)；比值<2 → 应被成本门关闭。

ladder 编号沿用蜂群口径（organic_signals.py）：ladder0=bar, ladder1=笔,
ladder2=笔中枢, ladder3=走势级, ladder4+=递归层。任务文中"bi级(ladder0/
a0层)的笔中枢" = 本口径 ladder2。

输出：analysis/data_cache/p0c_ladder_cost.json（增量续跑，按标的落盘）
用法：PYTHONPATH=src .venv/bin/python analysis/p0c_ladder_cost.py
      （env：P0C_SYMBOLS=OKLO,QQQ  P0C_FORCE=1 强制重跑）
"""

from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
OUT_JSON = DATA_DIR / "p0c_ladder_cost.json"

SYMBOLS = [s.strip().upper()
           for s in os.environ.get("P0C_SYMBOLS", "OKLO,QQQ,BRN,CL").split(",")]
MAX_LEVELS = 8
COST_RT_PCT = 0.10  # 往返成本假设 10bps


def percentile(sorted_vals: list[float], q: float) -> float:
    """线性插值分位数（与 numpy 'linear' 等价）；输入须已升序。"""
    n = len(sorted_vals)
    if n == 1:
        return sorted_vals[0]
    pos = q * (n - 1)
    lo = int(pos)
    hi = min(lo + 1, n - 1)
    frac = pos - lo
    return sorted_vals[lo] * (1 - frac) + sorted_vals[hi] * frac


def amp_stats(amps: list[float]) -> dict | None:
    """振幅（相对 %）分布统计：N、P25/P50/P75、成本比值。"""
    if not amps:
        return None
    s = sorted(amps)
    p25, p50, p75 = (percentile(s, 0.25), percentile(s, 0.50),
                     percentile(s, 0.75))
    return {
        "n": len(s),
        "p25": round(p25, 4),
        "p50": round(p50, 4),
        "p75": round(p75, 4),
        "ratio_p50": round(p50 / COST_RT_PCT, 2),
        "gated": bool(p50 / COST_RT_PCT < 2.0),
    }


def rel_amp(zd: float, zg: float) -> float | None:
    """中枢相对振幅 % = (ZG−ZD)/中点。退化中枢（中点≤0/倒挂）剔除。"""
    mid = (zg + zd) / 2.0
    if mid <= 0 or zg <= zd:
        return None
    return (zg - zd) / mid * 100.0


def process_symbol(symbol: str) -> dict:
    print(f"\n{'=' * 60}\n  {symbol} — P0-c 递归塔中枢振幅\n{'=' * 60}", flush=True)
    opens, highs, lows, closes, _years = load_ohlc(SYMBOL_FILES[symbol])
    n = len(closes)
    print(f"  bars={n:,}", flush=True)

    t0 = time.time()
    orch = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)
    for i in range(n):
        orch.process_bar(opens[i], highs[i], lows[i], closes[i])
        if i % 500_000 == 0 and i > 0:
            print(f"    bar {i:,}/{n:,}  {time.time() - t0:.0f}s", flush=True)
    engine_s = time.time() - t0
    print(f"  引擎 {engine_s:.0f}s  strokes={orch.stroke_count():,}", flush=True)

    ladders: dict[str, dict] = {}

    # ── ladder2：笔中枢（confirmed strokes → 批量 zhongshu_from_strokes）──
    strokes = orch.current_strokes()
    confirmed = [s for s in strokes if s[7]]
    zs_in = [(s[0], s[1], s[3], s[4], True) for s in confirmed]
    bi_zhs = R.zhongshu_from_strokes(zs_in)
    amps2 = [a for z in bi_zhs if (a := rel_amp(z[0], z[1])) is not None]
    st2 = amp_stats(amps2)
    if st2:
        st2["n_raw"] = len(bi_zhs)
        ladders["ladder2"] = st2

    # ── ladder3：走势级（线段中枢）──
    seg_zhs = orch.current_zhongshus()
    amps3 = [a for z in seg_zhs if (a := rel_amp(z[0], z[1])) is not None]
    st3 = amp_stats(amps3)
    if st3:
        st3["n_raw"] = len(seg_zhs)
        ladders["ladder3"] = st3

    # ── ladder4+：递归层泛化中枢 ──
    for (lid, zhs, _mvs) in orch.current_recursive():
        amps = [a for z in zhs if (a := rel_amp(z[0], z[1])) is not None]
        st = amp_stats(amps)
        if st:
            st["n_raw"] = len(zhs)
            ladders[f"ladder{lid + 2}"] = st

    for name in sorted(ladders, key=lambda k: int(k.replace("ladder", ""))):
        s = ladders[name]
        print(f"  {name}: N={s['n']:>6,}  P25={s['p25']:.3f}%  "
              f"P50={s['p50']:.3f}%  P75={s['p75']:.3f}%  "
              f"比值={s['ratio_p50']:.2f}  {'关' if s['gated'] else '开'}",
              flush=True)

    return {"n_bars": n, "n_strokes": orch.stroke_count(),
            "engine_s": round(engine_s, 1), "cost_rt_pct": COST_RT_PCT,
            "ladders": ladders}


def main() -> None:
    results: dict = {}
    if OUT_JSON.exists() and not os.environ.get("P0C_FORCE"):
        results = json.loads(OUT_JSON.read_text())
    for sym in SYMBOLS:
        if sym in results:
            print(f"[增量] {sym} 已在册，跳过", flush=True)
            continue
        results[sym] = process_symbol(sym)
        OUT_JSON.write_text(json.dumps(results, indent=1))
        print(f"  [落盘] {OUT_JSON.name} ← {sym}", flush=True)
    print("\n全部完成。", flush=True)


if __name__ == "__main__":
    main()
