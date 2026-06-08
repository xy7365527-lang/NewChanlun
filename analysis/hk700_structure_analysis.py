"""700.HK 日线完整结构分析 — 笔/线段/中枢/走势类型/递归级别。

分析三层结构：
1. 笔中枢（直接从 strokes 构造，不依赖线段划分）
2. v1 线段 + 递归级别
3. 聚焦 677→427 下跌走势的内部结构
"""
from __future__ import annotations

import json
import logging
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))

from newchan.a_center_v0 import Center, centers_from_segments_v0  # noqa: E402
from newchan.a_fractal import fractals_from_merged  # noqa: E402
from newchan.a_inclusion import merge_inclusion  # noqa: E402
from newchan.a_recursive_engine import build_recursive_levels  # noqa: E402
from newchan.a_segment_v0 import Segment  # noqa: E402
from newchan.a_segment_v1 import segments_from_strokes_v1  # noqa: E402
from newchan.a_stroke import Stroke, strokes_from_fractals  # noqa: E402
from newchan.a_trendtype_v0 import trend_instances_from_centers  # noqa: E402

import numpy as np  # noqa: E402
import pandas as pd  # noqa: E402

logging.basicConfig(level=logging.WARNING, format="%(name)s %(message)s")

CACHE = ROOT / "analysis" / "data_cache"
CACHE_FILE = CACHE / "HK700_1d_2y.json"


def load_data() -> pd.DataFrame:
    with open(CACHE_FILE) as f:
        raw = json.load(f)
    n = len(raw["closes"])
    dates = [datetime.strptime(d, "%Y-%m-%d").replace(tzinfo=timezone.utc)
             for d in raw["dates"]]
    df = pd.DataFrame({
        "open": raw["opens"],
        "high": raw["highs"],
        "low": raw["lows"],
        "close": raw["closes"],
    }, index=pd.DatetimeIndex(dates, name="time"))
    print(f"数据: {n} bars, {raw['dates'][0]} ~ {raw['dates'][-1]}")
    return df


def run_pipeline(df: pd.DataFrame):
    df_merged, merged_to_raw = merge_inclusion(df)
    fractals = fractals_from_merged(df_merged)
    strokes = strokes_from_fractals(
        df_merged, fractals, mode="new", new_raw_gap_min=3, merged_to_raw=merged_to_raw,
    )
    segments_v1 = segments_from_strokes_v1(strokes)
    levels = build_recursive_levels(segments_v1, sustain_m=1)

    print(f"管线: {len(df)}→{len(df_merged)} merged, "
          f"{len(fractals)} fractals, {len(strokes)} strokes, "
          f"{len(segments_v1)} v1-segments, {len(levels)} recursive levels")
    return df_merged, merged_to_raw, strokes, segments_v1, levels


def strokes_to_pseudo_segments(strokes: list[Stroke]) -> list[Segment]:
    """将每根笔包装为伪 Segment，用于 centers_from_segments_v0。"""
    result = []
    for i, s in enumerate(strokes):
        result.append(Segment(
            s0=i, s1=i, i0=s.i0, i1=s.i1,
            direction=s.direction,
            high=s.high, low=s.low,
            confirmed=s.confirmed,
            kind="settled",
            ep0_i=s.i0, ep0_price=s.p0, ep0_type="bottom" if s.direction == "up" else "top",
            ep1_i=s.i1, ep1_price=s.p1, ep1_type="top" if s.direction == "up" else "bottom",
            p0=s.p0, p1=s.p1,
        ))
    return result


def fmt_date(df_merged, idx):
    if 0 <= idx < len(df_merged):
        return str(df_merged.index[idx])[:10]
    return "?"


def print_strokes_range(strokes, df_merged, start=0, end=None, prefix=""):
    if end is None:
        end = len(strokes)
    for i in range(start, end):
        s = strokes[i]
        conf = "✓" if s.confirmed else "○"
        print(f"{prefix}笔{i:3d} [{conf}] {s.direction:>4s}  "
              f"i[{s.i0:3d}→{s.i1:3d}]  "
              f"H={s.high:7.1f}  L={s.low:7.1f}  "
              f"p[{s.p0:7.1f}→{s.p1:7.1f}]  "
              f"{fmt_date(df_merged,s.i0)}→{fmt_date(df_merged,s.i1)}")


def print_centers(centers, prefix="  "):
    for i, c in enumerate(centers):
        settled = "S" if c.kind == "settled" else "C"
        conf = "✓" if c.confirmed else "○"
        term = f" TERM({c.termination_side})" if c.terminated else ""
        dev = f" [{c.development}]" if c.development else ""
        print(f"{prefix}中枢{i:2d} [{conf}/{settled}]  "
              f"seg[{c.seg0}→{c.seg1}]  "
              f"ZD={c.low:7.1f}  ZG={c.high:7.1f}  "
              f"sustain={c.sustain}  dir={c.direction}"
              f"{dev}{term}")


def print_trends(trends, prefix="  "):
    for i, t in enumerate(trends):
        conf = "✓" if t.confirmed else "○"
        print(f"{prefix}走势{i:2d} [{conf}] {t.kind:15s} {t.direction:>4s}  "
              f"seg[{t.seg0}→{t.seg1}]  "
              f"H={t.high:7.1f}  L={t.low:7.1f}  "
              f"centers={t.center_indices}")


def analyze_decline_structure(strokes, df_merged, df):
    """从笔中枢层面分析 677→427 的下跌结构。"""
    print("\n" + "="*80)
    print("677→427 下跌结构分析（笔级别中枢）")
    print("="*80)

    peak_price = df["high"].max()
    peak_date = df["high"].idxmax()
    latest_price = df["close"].iloc[-1]
    latest_date = df.index[-1]
    print(f"历史高点: {peak_price:.1f} @ {str(peak_date)[:10]}")
    print(f"最新价格: {latest_price:.1f} @ {str(latest_date)[:10]}")
    print(f"跌幅: {(peak_price - latest_price) / peak_price * 100:.1f}%")

    # 找到高点对应的笔 — 笔31的 H=683 包含了 peak
    peak_stroke_idx = None
    for i, s in enumerate(strokes):
        if s.high >= peak_price * 0.99:
            peak_stroke_idx = i
    if peak_stroke_idx is None:
        print("无法定位高点笔")
        return

    # 从高点笔开始的所有笔 = 下跌窗口
    decline_strokes = strokes[peak_stroke_idx:]
    decline_start = peak_stroke_idx
    print(f"\n高点笔: 笔{peak_stroke_idx} (H={strokes[peak_stroke_idx].high:.1f})")
    print(f"下跌窗口: 笔{decline_start}→{len(strokes)-1} ({len(decline_strokes)} 笔)")

    print(f"\n{'─'*60}")
    print(f"下跌窗口笔列表:")
    print_strokes_range(strokes, df_merged, decline_start, prefix="  ")

    # 笔级别中枢分析
    pseudo_segs = strokes_to_pseudo_segments(decline_strokes)
    bi_centers = centers_from_segments_v0(pseudo_segs, sustain_m=1)

    print(f"\n{'─'*60}")
    print(f"笔中枢 ({len(bi_centers)} 个):")
    print_centers(bi_centers)

    bi_trends = trend_instances_from_centers(pseudo_segs, bi_centers)
    print(f"\n笔级别走势类型 ({len(bi_trends)} 个):")
    print_trends(bi_trends)

    # 递归
    if len(pseudo_segs) >= 3:
        bi_levels = build_recursive_levels(pseudo_segs, sustain_m=1)
        if bi_levels:
            print(f"\n{'─'*60}")
            print(f"笔级别递归 ({len(bi_levels)} 层):")
            for rl in bi_levels:
                n_conf = sum(1 for t in rl.trends if t.confirmed)
                print(f"  Level {rl.level}: {len(rl.moves)} moves → "
                      f"{len(rl.centers)} centers → "
                      f"{len(rl.trends)} trends ({n_conf} confirmed)")
                print_centers(rl.centers, prefix="    ")
                print_trends(rl.trends, prefix="    ")

    # ─── 诊断分析 ───
    print(f"\n{'═'*60}")
    print("结构诊断")
    print(f"{'═'*60}")

    if not bi_centers:
        print("  无中枢形成。下跌走势尚未形成内部结构。")
        return

    settled_centers = [c for c in bi_centers if c.kind == "settled"]
    candidate_centers = [c for c in bi_centers if c.kind == "candidate"]
    terminated_centers = [c for c in bi_centers if c.terminated]

    print(f"  中枢统计: {len(bi_centers)} 总计, "
          f"{len(settled_centers)} settled, "
          f"{len(candidate_centers)} candidate, "
          f"{len(terminated_centers)} terminated")

    # 检查走势类型
    down_trends = [t for t in bi_trends if t.direction == "down"]
    up_trends = [t for t in bi_trends if t.direction == "up"]
    print(f"  走势统计: {len(down_trends)} 下跌, {len(up_trends)} 上涨")

    # 趋势判定：≥2 同向中枢 = 趋势
    n_settled = len(settled_centers)
    if n_settled >= 2:
        # 检查是否有同向中枢
        down_settled = [c for c in settled_centers
                       if c.terminated and c.termination_side == "below"]
        print(f"\n  趋势判定: {n_settled} 个 settled 中枢")
        for c in settled_centers:
            print(f"    中枢 seg[{c.seg0}→{c.seg1}] ZD={c.low:.1f} ZG={c.high:.1f} "
                  f"{'TERM('+c.termination_side+')' if c.terminated else 'active'}")

    # 最后一个中枢的状态 → 走势是否结束
    last_center = bi_centers[-1]
    last_stroke = decline_strokes[-1]
    print(f"\n  最后中枢: ZD={last_center.low:.1f} ZG={last_center.high:.1f}")
    print(f"  最新笔: {last_stroke.direction} p[{last_stroke.p0:.1f}→{last_stroke.p1:.1f}]")

    if last_stroke.p1 < last_center.low:
        print(f"  → 最新笔低于中枢下沿，下跌走势可能延续")
    elif last_stroke.p1 > last_center.high:
        print(f"  → 最新笔突破中枢上沿，下跌走势可能结束")
    else:
        print(f"  → 最新笔在中枢区间内，走势未定")

    # 背驰检查（简化：比较前后走势段的力度）
    if len(bi_trends) >= 2:
        last_trend = bi_trends[-1]
        prev_trend = bi_trends[-2]
        if last_trend.direction == "down" and prev_trend.direction == "down":
            last_range = abs(last_trend.high - last_trend.low)
            prev_range = abs(prev_trend.high - prev_trend.low)
            print(f"\n  背驰初步检查:")
            print(f"    前段下跌力度: {prev_range:.1f} (H={prev_trend.high:.1f} L={prev_trend.low:.1f})")
            print(f"    后段下跌力度: {last_range:.1f} (H={last_trend.high:.1f} L={last_trend.low:.1f})")
            if last_range < prev_range:
                print(f"    → 力度递减，存在背驰迹象")
            else:
                print(f"    → 力度未递减，无背驰")


def main():
    df = load_data()
    df_merged, merged_to_raw, strokes, segments_v1, levels = run_pipeline(df)

    # 全量笔列表
    print(f"\n{'='*80}")
    print(f"全量笔列表 ({len(strokes)} 笔)")
    print(f"{'='*80}")
    print_strokes_range(strokes, df_merged)

    # v1 线段
    print(f"\n{'='*80}")
    print(f"v1 线段 ({len(segments_v1)} 段)")
    print(f"{'='*80}")
    for i, seg in enumerate(segments_v1):
        conf = "✓" if seg.confirmed else "○"
        n_strokes = seg.s1 - seg.s0 + 1
        print(f"  段{i:2d} [{conf}/{seg.kind[0].upper()}] {seg.direction:>4s}  "
              f"s[{seg.s0:3d}→{seg.s1:3d}] ({n_strokes}笔)  "
              f"H={seg.high:7.1f}  L={seg.low:7.1f}  "
              f"ep[{seg.ep0_price:7.1f}→{seg.ep1_price:7.1f}]  "
              f"{fmt_date(df_merged,seg.i0)}→{fmt_date(df_merged,seg.i1)}")

    # 全量笔中枢
    print(f"\n{'='*80}")
    print(f"全量笔中枢")
    print(f"{'='*80}")
    all_pseudo = strokes_to_pseudo_segments(strokes)
    all_bi_centers = centers_from_segments_v0(all_pseudo, sustain_m=1)
    print_centers(all_bi_centers)
    all_bi_trends = trend_instances_from_centers(all_pseudo, all_bi_centers)
    print(f"\n全量笔级别走势类型 ({len(all_bi_trends)} 个):")
    print_trends(all_bi_trends)

    # 聚焦下跌分析
    analyze_decline_structure(strokes, df_merged, df)


if __name__ == "__main__":
    main()
