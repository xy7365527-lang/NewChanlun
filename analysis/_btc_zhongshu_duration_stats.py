"""BTC 各递归级别中枢持续时间/振幅统计 —— 期权到期日选择的结构锚数据。

数据：analysis/data_cache/btc_1m_full.json（Binance 1min 全历史，a0=1min）。
引擎：newchan_rust.RecursiveOrchestrator（原生流式，max_levels=6，enable_bsp=True
      ——orchestrator.rs:753 递归层与 BSP 层同门控，enable_bsp=False 会跳过递归栈，
      故必须开启；增量路径 use_inc_bsp 在默认参数下激活）。

级别口径：
  L0_bi : 笔中枢（zhongshu_from_strokes，confirmed 笔；附加参考行，非递归链）
  L1    : 线段中枢（orch.current_zhongshus()）
  L2+   : 递归级别中枢（orch.current_recursive()，level_id 从 2 起）

时间锚定链（在册陷阱：stroke i0/i1 是包含合并坐标非 raw bar；
MoveTuple first_seg_s0/last_seg_s1 是 stroke/组件索引非 bar；L2+ 索引层内局部）：
  merged idx → raw idx     : merge_inclusion(df_raw) 的 merged_to_raw
                             （nested_pipeline.py 在册同款配对，reset_dir_on_fractal=False
                              与引擎默认一致；对齐性由全笔端点价格逐一核验）
  stroke t  → raw span     : (m2r[i0][0], m2r[i1][1])
  segment j → raw span     : stroke 链（head s0/s1 = stroke 索引）
  L1 zs     → raw span     : ZhongshuTuple[8]/[9] = first_seg_s0/last_seg_s1（stroke 索引）
  L1 move   → raw span     : head seg_start/seg_end（segment 索引，含修复A末组扩展）
  Lk zs/move(k≥2) → raw span: comp_start/comp_end 索引 settled 下级 move 列表
                             （orchestrator.rs LevelEngine：comps = settled 过滤后
                              enumerate，component_idx = settled 列表位置）→ 逐级下钻
  L0_bi zs  → raw span     : first_seg_s0/last_seg_s1 = merged bar 坐标（zhongshu.rs
                              anchor_start=i0）→ m2r 直查

统计量（仅 settled=已完成对象）：
  n、持续时间 P25/P50/P75/P90（bar=raw 1min bar；天=时间戳差，crypto 24/7）、
  相对振幅 (ZG−ZD)/中点 P25/P50/P75；move 同口径持续时间。

运行：.venv/bin/python analysis/_btc_zhongshu_duration_stats.py
      BT_MAX_BARS=200000 可裁前缀探针。
认识论等级：L2（单标的 BTC 真实数据全历史）。
"""
from __future__ import annotations

import json
import math
import os
import sys
import time
from datetime import datetime
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(ROOT / "src"))

import pandas as pd  # noqa: E402
import newchan_rust as R  # noqa: E402
from newchan.a_inclusion import merge_inclusion  # noqa: E402

DATA = HERE / "data_cache" / "btc_1m_full.json"
OUT_MD = HERE / "_btc_zhongshu_duration_stats.md"
OUT_JSON = HERE / "_btc_zhongshu_duration_stats.json"

BARS_PER_DAY = 1440  # 1min × 24/7


def log(msg: str) -> None:
    print(f"[{time.strftime('%H:%M:%S')}] {msg}", flush=True)


def load_ohlc(path: Path):
    """加载并清洗：删除任一 OHLC 为 nan/非正的 bar，dates 同步对齐。"""
    raw = json.loads(path.read_text())
    o_in, h_in, l_in, c_in = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
    d_in = raw["dates"]
    _max = int(os.environ.get("BT_MAX_BARS", "0"))
    if _max > 0:
        o_in, h_in, l_in, c_in, d_in = (
            o_in[:_max], h_in[:_max], l_in[:_max], c_in[:_max], d_in[:_max])
    opens, highs, lows, closes, dates = [], [], [], [], []
    for idx in range(len(c_in)):
        o, h, l, c = (float(o_in[idx]), float(h_in[idx]),
                      float(l_in[idx]), float(c_in[idx]))
        if any(math.isnan(v) for v in (o, h, l, c)):
            continue
        if o <= 0 or h <= 0 or l <= 0 or c <= 0:
            continue
        opens.append(o)
        highs.append(h)
        lows.append(l)
        closes.append(c)
        dates.append(str(d_in[idx]))
    return opens, highs, lows, closes, dates


def pct(sorted_vals: list[float], q: float) -> float:
    """线性插值分位数（输入须已排序）。"""
    n = len(sorted_vals)
    if n == 0:
        return float("nan")
    if n == 1:
        return sorted_vals[0]
    pos = q * (n - 1)
    lo = int(pos)
    hi = min(lo + 1, n - 1)
    return sorted_vals[lo] * (1 - (pos - lo)) + sorted_vals[hi] * (pos - lo)


def dur_stats(spans: list[tuple[int, int]], dates: list[str]) -> dict:
    """持续时间分位数：bar 数（raw）与日历天（时间戳差，含数据缺口）。"""
    bars = sorted(e - s for s, e in spans)
    days = sorted(
        (datetime.fromisoformat(dates[e]) - datetime.fromisoformat(dates[s]))
        .total_seconds() / 86400.0
        for s, e in spans
    )
    return {
        "n": len(spans),
        "dur_bars": {f"p{int(q*100)}": round(pct(bars, q), 1)
                     for q in (0.25, 0.50, 0.75, 0.90)},
        "dur_days": {f"p{int(q*100)}": round(pct(days, q), 3)
                     for q in (0.25, 0.50, 0.75, 0.90)},
    }


def amp_stats(amps: list[float]) -> dict:
    s = sorted(amps)
    return {f"p{int(q*100)}": round(pct(s, q) * 100, 3)
            for q in (0.25, 0.50, 0.75)}  # 百分比


def main() -> None:
    t0 = time.time()
    opens, highs, lows, closes, dates = load_ohlc(DATA)
    n = len(closes)
    log(f"数据加载完成 n={n} 窗口 {dates[0]} → {dates[-1]}")

    # ── merged→raw 映射（与引擎同口径：reset_dir_on_fractal=False）──
    df = pd.DataFrame({"open": opens, "high": highs, "low": lows, "close": closes})
    df_merged, m2r = merge_inclusion(df)
    mh = df_merged["high"].values
    ml = df_merged["low"].values
    log(f"merge_inclusion 完成 merged={len(m2r)} ({time.time()-t0:.0f}s)")

    # ── 原生流式 RecursiveOrchestrator ──
    orch = R.RecursiveOrchestrator(max_levels=6, enable_bsp=True)
    t1 = time.time()
    for i in range(n):
        orch.process_bar(opens[i], highs[i], lows[i], closes[i])
        if (i + 1) % 200_000 == 0:
            log(f"  process_bar {i+1}/{n} ({time.time()-t1:.0f}s)")
    log(f"流式完成 ({time.time()-t1:.0f}s)")

    strokes = orch.current_strokes()   # (i0,i1,dir,high,low,p0,p1,confirmed) merged 坐标
    segs = orch.current_segments()     # head=(s0,s1,i0,i1,dir,high,low,confirmed,kind)
    l1_zs = orch.current_zhongshus()   # ZhongshuTuple 12 字段
    l1_moves = orch.current_moves()    # MoveTuple
    rec = sorted(orch.current_recursive(), key=lambda t: t[0])
    log(f"结构: strokes={len(strokes)} segs={len(segs)} l1_zs={len(l1_zs)} "
        f"l1_moves={len(l1_moves)} rec_levels={[t[0] for t in rec]}")

    # ── merged_to_raw 对齐核验（fail-fast，非抽样）──
    max_i1 = max(s[1] for s in strokes)
    if max_i1 >= len(m2r):
        raise RuntimeError(
            f"merged_to_raw 与引擎合并坐标不对齐: max stroke i1={max_i1} "
            f">= len(m2r)={len(m2r)}")
    mismatch = 0
    for s in strokes:
        i0, i1, d = s[0], s[1], s[2]
        # up 笔: 起点=低点 p0=ml[i0], 终点=高点 p1=mh[i1]; down 笔反之
        if d == "up":
            ok = (s[5] == ml[i0]) and (s[6] == mh[i1])
        else:
            ok = (s[5] == mh[i0]) and (s[6] == ml[i1])
        if not ok:
            mismatch += 1
    if mismatch:
        raise RuntimeError(
            f"merged_to_raw 价格核验失败: {mismatch}/{len(strokes)} 笔端点价格"
            f"与 merge_inclusion 合并序列不一致——映射不可用，拒绝产出")
    log(f"对齐核验通过: {len(strokes)} 笔端点价格全部一致")

    # ── 锚定链 ──
    def stroke_span(t: int) -> tuple[int, int]:
        s = strokes[t]
        return m2r[s[0]][0], m2r[s[1]][1]

    def seg_span(j: int) -> tuple[int, int]:
        h = segs[j][0]
        return stroke_span(h[0])[0], stroke_span(h[1])[1]

    n_segs = len(segs)

    # L1 move spans（head: kind,dir,seg_start,seg_end,...,settled@7）
    l1_move_spans: list[tuple[int, int]] = []
    for m in l1_moves:
        ss = max(int(m[0][2]), 0)
        se = min(int(m[0][3]), n_segs - 1)
        l1_move_spans.append((seg_span(ss)[0], seg_span(se)[1]))

    levels_out: dict[str, dict] = {}

    # ── L0_bi 笔中枢（参考行；first_seg_s0/last_seg_s1 = merged 坐标直查 m2r）──
    bi_zs = R.zhongshu_from_strokes(
        [(s[0], s[1], s[3], s[4], s[7]) for s in strokes])
    spans = [(m2r[z[8]][0], m2r[z[9]][1]) for z in bi_zs if z[5]]
    amps = [(z[1] - z[0]) / ((z[1] + z[0]) / 2) for z in bi_zs if z[5]]
    levels_out["L0_bi"] = {"zhongshu": {**dur_stats(spans, dates),
                                        "amp_pct": amp_stats(amps)}}

    # ── L1 线段中枢 + L1 move ──
    spans = [(stroke_span(z[8])[0], stroke_span(z[9])[1]) for z in l1_zs if z[5]]
    amps = [(z[1] - z[0]) / ((z[1] + z[0]) / 2) for z in l1_zs if z[5]]
    mv_spans = [sp for m, sp in zip(l1_moves, l1_move_spans) if m[0][7]]
    levels_out["L1"] = {
        "zhongshu": {**dur_stats(spans, dates), "amp_pct": amp_stats(amps)},
        "move": dur_stats(mv_spans, dates),
    }

    # ── L2+ 递归级别（comps = settled 下级 moves，层内局部索引逐级下钻）──
    prev_moves, prev_spans = l1_moves, l1_move_spans
    for lid, zss, mvs in rec:
        comp_spans = [sp for m, sp in zip(prev_moves, prev_spans) if m[0][7]]
        if not comp_spans:
            break
        nc = len(comp_spans)
        # LevelZhongshuTuple: (zd,zg,comp_start,comp_end,comp_count,settled,...)
        zs_spans, zs_amps = [], []
        for z in zss:
            if not z[5]:
                continue
            zs_spans.append((comp_spans[z[2]][0], comp_spans[z[3]][1]))
            zs_amps.append((z[1] - z[0]) / ((z[1] + z[0]) / 2))
        this_move_spans: list[tuple[int, int]] = []
        for m in mvs:
            ss = max(int(m[0][2]), 0)
            se = min(int(m[0][3]), nc - 1)
            this_move_spans.append((comp_spans[ss][0], comp_spans[se][1]))
        mv_spans = [sp for m, sp in zip(mvs, this_move_spans) if m[0][7]]
        entry: dict = {}
        if zs_spans:
            entry["zhongshu"] = {**dur_stats(zs_spans, dates),
                                 "amp_pct": amp_stats(zs_amps)}
        if mv_spans:
            entry["move"] = dur_stats(mv_spans, dates)
        if entry:
            levels_out[f"L{lid}"] = entry
        prev_moves, prev_spans = mvs, this_move_spans

    # ── 合理性核验：中枢数随级别递减、持续时间 P50 随级别递增 ──
    chain = [k for k in levels_out if k != "L0_bi" and "zhongshu" in levels_out[k]]
    warns: list[str] = []
    for a, b in zip(chain, chain[1:]):
        za, zb = levels_out[a]["zhongshu"], levels_out[b]["zhongshu"]
        if zb["n"] >= za["n"]:
            warns.append(f"中枢数未递减: {a} n={za['n']} → {b} n={zb['n']}")
        if zb["dur_bars"]["p50"] <= za["dur_bars"]["p50"]:
            warns.append(f"持续时间P50未递增: {a}={za['dur_bars']['p50']} → "
                         f"{b}={zb['dur_bars']['p50']}")
    for w in warns:
        log(f"WARN {w}")

    result = {
        "symbol": "BTCUSDT (Binance 1min)",
        "a0": "1min",
        "window": {"start": dates[0], "end": dates[-1], "n_bars": n},
        "engine": ("newchan_rust.RecursiveOrchestrator(max_levels=6, "
                   "stroke_mode=wide, enable_bsp=True) 原生流式"),
        "anchoring": "merged→raw via merge_inclusion; 全笔端点价格核验通过",
        "settled_only": True,
        "levels": levels_out,
        "sanity_warnings": warns,
        "epistemic_level": "L2 (单标的 BTC 真实数据全历史)",
        "elapsed_s": round(time.time() - t0, 1),
    }
    OUT_JSON.write_text(json.dumps(result, ensure_ascii=False, indent=2))

    # ── markdown ──
    lines = [
        "# BTC 各递归级别中枢持续时间/振幅统计（期权到期日结构锚）",
        "",
        f"- 数据：Binance BTCUSDT 1min 全历史，窗口 **{dates[0]} → {dates[-1]}**"
        f"（清洗后 {n:,} bars，crypto 24/7）",
        "- 引擎：`newchan_rust.RecursiveOrchestrator(max_levels=6)` 原生流式；"
        "a0=1min；仅统计 **settled（已完成）** 对象",
        "- 时间锚定：merged→raw 经 `merge_inclusion`（全笔端点价格逐一核验一致）；"
        "天 = 起止 bar 时间戳差（含数据缺口的真实日历时间）",
        "- 振幅 = (ZG−ZD)/中点 ×100%；认识论等级 **L2**（单标的真实数据）",
        "",
        "## 中枢（zhongshu）",
        "",
        "| 级别 | n | 时长P25 | P50 | P75 | P90 (天) | P50(bar) | "
        "振幅P25 | P50 | P75 (%) |",
        "|------|---|---------|-----|-----|----------|----------|"
        "---------|-----|---------|",
    ]
    for name, entry in levels_out.items():
        if "zhongshu" not in entry:
            continue
        z = entry["zhongshu"]
        d, b, a = z["dur_days"], z["dur_bars"], z["amp_pct"]
        lines.append(
            f"| {name} | {z['n']} | {d['p25']} | {d['p50']} | {d['p75']} | "
            f"{d['p90']} | {b['p50']:,} | {a['p25']} | {a['p50']} | {a['p75']} |")
    lines += ["", "## 走势（move，settled）", "",
              "| 级别 | n | 时长P25 | P50 | P75 | P90 (天) | P50(bar) |",
              "|------|---|---------|-----|-----|----------|----------|"]
    for name, entry in levels_out.items():
        if "move" not in entry:
            continue
        mv = entry["move"]
        d, b = mv["dur_days"], mv["dur_bars"]
        lines.append(f"| {name} | {mv['n']} | {d['p25']} | {d['p50']} | "
                     f"{d['p75']} | {d['p90']} | {b['p50']:,} |")
    if warns:
        lines += ["", "## 合理性核验告警", ""] + [f"- {w}" for w in warns]
    else:
        lines += ["", "合理性核验：中枢数随级别递减 ✓ 持续时间P50随级别递增 ✓"]
    lines += ["", f"运行耗时 {result['elapsed_s']}s；产出 "
              f"`{OUT_JSON.name}` / 本文件。"]
    OUT_MD.write_text("\n".join(lines) + "\n")
    log(f"完成 → {OUT_MD}")
    print(json.dumps({k: {kk: (vv["n"] if isinstance(vv, dict) and "n" in vv
                               else None)
                          for kk, vv in v.items()}
                      for k, v in levels_out.items()}, ensure_ascii=False))


if __name__ == "__main__":
    main()
