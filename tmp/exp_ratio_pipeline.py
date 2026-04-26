"""金油比日线缠论管线分析脚本。

数据构造: GC_close / BZ_close → 退化K线 (open=high=low=close=ratio)
管线: RecursiveOrchestrator (口径 A) → NestedDivergenceSearch
"""
import sys
from datetime import datetime

sys.path.insert(0, "src")

import pandas as pd

from newchan.cache import load_df
from newchan.nested_pipeline import run_nested_search
from newchan.types import Bar

# ── 从原始 GC/BZ 构造退化K线 ──
gc = load_df("GC_1day_raw")
bz = load_df("BZ_1day_raw")
if gc is None or bz is None:
    print("ERROR: GC 或 BZ 缓存文件不存在")
    sys.exit(1)

idx = gc.index.intersection(bz.index)
gc, bz = gc.loc[idx], bz.loc[idx]
ratio_close = gc["close"] / bz["close"]

# 退化K线: open=high=low=close=ratio
df = pd.DataFrame({
    "open": ratio_close,
    "high": ratio_close,
    "low": ratio_close,
    "close": ratio_close,
    "volume": gc["volume"].values if "volume" in gc.columns else 0,
}, index=idx)

print(f"GC 日线: {len(gc)}, BZ 日线: {len(bz)}, 交集: {len(idx)}")
print(f"金油比范围: {ratio_close.min():.4f} ~ {ratio_close.max():.4f}")

# ── 转为 Bar 列表 ──
bars = [
    Bar(
        ts=ts.to_pydatetime() if hasattr(ts, "to_pydatetime") else ts,
        open=row["open"],
        high=row["high"],
        low=row["low"],
        close=row["close"],
        volume=float(row.get("volume", 0)),
    )
    for ts, row in df.iterrows()
]

print(f"总K线数: {len(bars)}")
print(f"时间范围: {bars[0].ts} ~ {bars[-1].ts}")
print()

# ── 跑管线 ──
results, snap = run_nested_search(
    bars=bars,
    stroke_mode="wide",
    max_levels=6,
)

if snap is None:
    print("ERROR: 管线返回 None")
    sys.exit(1)

# ── Level 1 结构统计 ──
print("=" * 60)
print("Level 1 (基于日线笔)")
print("=" * 60)
n_strokes = len(snap.bi_snapshot.strokes)
n_segments = len(snap.seg_snapshot.segments)
n_zhongshus = len(snap.zs_snapshot.zhongshus)
n_moves = len(snap.move_snapshot.moves)
n_bsps = len(snap.bsp_snapshot.buysellpoints)
print(f"  笔={n_strokes}, 线段={n_segments}, 中枢={n_zhongshus}, "
      f"走势={n_moves}, 买卖点={n_bsps}")

# ── 递归层统计 ──
for rs in snap.recursive_snapshots:
    print(f"Level {rs.level_id}: 中枢={len(rs.zhongshus)}, 走势={len(rs.moves)}")

print()

# ── 辅助: 从 bar index 获取时间 ──
def bar_ts_str(idx: int) -> str:
    """从 bar index 获取日期字符串。"""
    if idx < len(bars):
        ts = bars[idx].ts
        return str(ts.date()) if hasattr(ts, "date") else str(ts)
    return f"idx={idx}"


# ── 最近买卖点 ──
print("=" * 60)
print("买卖点（全部）")
print("=" * 60)
if snap.bsp_snapshot.buysellpoints:
    for bsp in snap.bsp_snapshot.buysellpoints[-10:]:
        confirmed_str = "已确认" if bsp.confirmed else "待确认"
        settled_str = "已结算" if bsp.settled else "未结算"
        print(f"  {bsp.kind} {bsp.side} | price={bsp.price:.4f} "
              f"| bar_idx={bsp.bar_idx} ({bar_ts_str(bsp.bar_idx)}) "
              f"| {confirmed_str} {settled_str}")
else:
    print("  无")

print()

# ── 最近走势 (Level 1) ──
print("=" * 60)
print("走势类型 (Level 1, 最近10个)")
print("=" * 60)
for m in snap.move_snapshot.moves[-10:]:
    settled_str = "settled" if m.settled else "active"
    # 通过 seg→stroke→bar 获取时间范围
    segs = snap.seg_snapshot.segments
    strokes = snap.bi_snapshot.strokes
    t_start = t_end = "?"
    if m.seg_start < len(segs) and m.seg_end < len(segs):
        s0 = segs[m.seg_start].s0
        s1 = segs[m.seg_end].s1
        if s0 < len(strokes):
            t_start = bar_ts_str(strokes[s0].i0)
        if s1 < len(strokes):
            t_end = bar_ts_str(strokes[s1].i1)
    print(f"  {m.kind:15s} {m.direction:5s} | "
          f"seg[{m.seg_start}..{m.seg_end}] zs_count={m.zs_count} | "
          f"high={m.high:.4f} low={m.low:.4f} | {settled_str} "
          f"| {t_start}~{t_end}")

print()

# ── 最近中枢 (Level 1) ──
print("=" * 60)
print("中枢 (Level 1, 最近10个)")
print("=" * 60)
for zs in snap.zs_snapshot.zhongshus[-10:]:
    settled_str = "settled" if zs.settled else "active"
    segs = snap.seg_snapshot.segments
    strokes = snap.bi_snapshot.strokes
    t_start = t_end = "?"
    if zs.seg_start < len(segs) and zs.seg_end < len(segs):
        s0 = segs[zs.seg_start].s0
        s1 = segs[zs.seg_end].s1
        if s0 < len(strokes):
            t_start = bar_ts_str(strokes[s0].i0)
        if s1 < len(strokes):
            t_end = bar_ts_str(strokes[s1].i1)
    print(f"  [ZD={zs.zd:.4f}, ZG={zs.zg:.4f}] "
          f"seg[{zs.seg_start}..{zs.seg_end}] count={zs.seg_count} | "
          f"DD={zs.dd:.4f} GG={zs.gg:.4f} | {settled_str} "
          f"break={zs.break_direction} | {t_start}~{t_end}")

print()

# ── 最近线段 ──
print("=" * 60)
print("线段 (Level 1, 最近15段)")
print("=" * 60)
for seg in snap.seg_snapshot.segments[-15:]:
    confirmed_str = "confirmed" if seg.confirmed else "unconfirmed"
    strokes = snap.bi_snapshot.strokes
    t_start = t_end = "?"
    if seg.s0 < len(strokes):
        t_start = bar_ts_str(strokes[seg.s0].i0)
    if seg.s1 < len(strokes):
        t_end = bar_ts_str(strokes[seg.s1].i1)
    print(f"  seg s0={seg.s0} s1={seg.s1} {seg.direction:5s} | "
          f"high={seg.high:.4f} low={seg.low:.4f} | "
          f"{confirmed_str} kind={seg.kind} | {t_start}~{t_end}")

print()

# ── 最近笔 ──
print("=" * 60)
print("笔 (最近20笔)")
print("=" * 60)
for st in snap.bi_snapshot.strokes[-20:]:
    confirmed_str = "confirmed" if st.confirmed else "extending"
    t0 = bar_ts_str(st.i0)
    t1 = bar_ts_str(st.i1)
    print(f"  i0={st.i0:4d}({t0}) i1={st.i1:4d}({t1}) {st.direction:5s} | "
          f"p0={st.p0:.4f} p1={st.p1:.4f} | {confirmed_str}")

print()

# ── 区间套背驰 ──
print("=" * 60)
print(f"区间套背驰: {len(results)} 条")
print("=" * 60)
for i, nd in enumerate(results):
    levels_str = ", ".join(
        f"L{lvl}={'有' if div is not None else '无'}"
        for lvl, div in nd.chain
    )
    print(f"  背驰{i+1}: bar_range={nd.bar_range} | {levels_str}")

print()

# ── 2025-02-20 附近结构 ──
print("=" * 60)
print("2025-02-20 附近结构")
print("=" * 60)

target = datetime(2025, 2, 20)
target_idx = None
for i, b in enumerate(bars):
    ts = b.ts
    if hasattr(ts, "date"):
        if ts.date() >= target.date():
            target_idx = i
            break
    else:
        if ts >= target:
            target_idx = i
            break

if target_idx is not None:
    print(f"  2025-02-20 对应 bar_idx ≈ {target_idx} ({bars[target_idx].ts})")
    print(f"  close = {bars[target_idx].close:.4f}")

    # 找附近的笔端点 (±50 bar)
    print("\n  附近笔端点 (±50 bar):")
    strokes = snap.bi_snapshot.strokes
    for st in strokes:
        if abs(st.i0 - target_idx) < 50 or abs(st.i1 - target_idx) < 50:
            print(f"    笔 i0={st.i0}({bar_ts_str(st.i0)}) "
                  f"i1={st.i1}({bar_ts_str(st.i1)}) "
                  f"{st.direction} p0={st.p0:.4f} p1={st.p1:.4f} "
                  f"{'confirmed' if st.confirmed else 'extending'}")

    # 找附近的线段端点
    print("\n  附近线段端点 (±150 bar):")
    for seg in snap.seg_snapshot.segments:
        strokes = snap.bi_snapshot.strokes
        if seg.s0 < len(strokes) and seg.s1 < len(strokes):
            bar_s0 = strokes[seg.s0].i0
            bar_s1 = strokes[seg.s1].i1
            if abs(bar_s0 - target_idx) < 150 or abs(bar_s1 - target_idx) < 150:
                print(f"    seg s0={seg.s0}(bar≈{bar_s0},{bar_ts_str(bar_s0)}) "
                      f"s1={seg.s1}(bar≈{bar_s1},{bar_ts_str(bar_s1)}) "
                      f"{seg.direction} high={seg.high:.4f} low={seg.low:.4f} "
                      f"{'confirmed' if seg.confirmed else 'unconfirmed'}")

    # 找附近的中枢
    print("\n  附近中枢 (±200 bar):")
    for zs in snap.zs_snapshot.zhongshus:
        segs = snap.seg_snapshot.segments
        if zs.seg_start < len(segs) and zs.seg_end < len(segs):
            strokes = snap.bi_snapshot.strokes
            s0_stroke = segs[zs.seg_start].s0
            s1_stroke = segs[zs.seg_end].s1
            if s0_stroke < len(strokes) and s1_stroke < len(strokes):
                bar_start = strokes[s0_stroke].i0
                bar_end = strokes[s1_stroke].i1
                if bar_start <= target_idx + 200 and bar_end >= target_idx - 200:
                    print(f"    中枢 [ZD={zs.zd:.4f}, ZG={zs.zg:.4f}] "
                          f"bar≈[{bar_start},{bar_end}] "
                          f"({bar_ts_str(bar_start)}~{bar_ts_str(bar_end)}) "
                          f"{'settled' if zs.settled else 'active'}")

    # 找附近的买卖点
    print("\n  附近买卖点 (±200 bar):")
    for bsp in snap.bsp_snapshot.buysellpoints:
        if abs(bsp.bar_idx - target_idx) < 200:
            print(f"    {bsp.kind} {bsp.side} price={bsp.price:.4f} "
                  f"bar_idx={bsp.bar_idx} ({bar_ts_str(bsp.bar_idx)}) "
                  f"{'confirmed' if bsp.confirmed else 'pending'}")
else:
    print("  未找到 2025-02-20 对应的 bar")

print()

# ── 当前走势状态 ──
print("=" * 60)
print("当前走势状态")
print("=" * 60)
if snap.move_snapshot.moves:
    last_move = snap.move_snapshot.moves[-1]
    print(f"  类型: {last_move.kind} ({last_move.direction})")
    print(f"  中枢数: {last_move.zs_count}")
    print(f"  区间: [{last_move.low:.4f}, {last_move.high:.4f}]")
    print(f"  settled: {last_move.settled}")
    if not last_move.settled:
        print(f"  → 当前走势仍在延伸中")

if snap.lstar is not None:
    print(f"  LStar: {snap.lstar}")
