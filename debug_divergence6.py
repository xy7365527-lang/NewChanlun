#!/usr/bin/env python
"""调试背驰检测逻辑。"""

from tests.test_divergence import _make_uptrend_segments
from newchan.a_center_v0 import centers_from_segments_v0
from newchan.a_trendtype_v0 import trend_instances_from_centers
from newchan.a_divergence import divergences_from_level, _compute_force, _move_merged_range

segs = _make_uptrend_segments()
centers = centers_from_segments_v0(segs, sustain_m=0)
trends = trend_instances_from_centers(segs, centers)

print("趋势:", trends[0])
t = trends[0]
print(f"  kind={t.kind}, direction={t.direction}, seg[{t.seg0}:{t.seg1}], centers={t.center_indices}")

if len(t.center_indices) >= 2:
    ci_prev = t.center_indices[-2]
    ci_last = t.center_indices[-1]
    c_prev = centers[ci_prev]
    c_last = centers[ci_last]

    print(f"\nc_prev: seg[{c_prev.seg0}:{c_prev.seg1}]")
    print(f"c_last: seg[{c_last.seg0}:{c_last.seg1}]")

    a_start = c_prev.seg1 + 1
    a_end = c_last.seg0 - 1
    print(f"\nA段: seg[{a_start}:{a_end}]")
    if a_start > a_end:
        print("  A段退化（中枢紧邻）")
        a_start = c_prev.seg1
        a_end = c_prev.seg1
        print(f"  调整为 seg[{a_start}:{a_end}]")

    i0_a, i1_a = _move_merged_range(segs, a_start, a_end)
    force_a = _compute_force(segs, a_start, a_end, t.direction, None, None)
    print(f"  merged_range=[{i0_a}:{i1_a}], force={force_a:.1f}")

    c_start = c_last.seg1 + 1
    c_end = t.seg1
    print(f"\nC段: seg[{c_start}:{c_end}]")
    if c_start > c_end:
        print("  C段尚未形成")
    else:
        i0_c, i1_c = _move_merged_range(segs, c_start, c_end)
        force_c = _compute_force(segs, c_start, c_end, t.direction, None, None)
        print(f"  merged_range=[{i0_c}:{i1_c}], force={force_c:.1f}")
        print(f"\n背驰判定: force_c ({force_c:.1f}) < force_a ({force_a:.1f}): {force_c < force_a}")

divs = divergences_from_level(segs, centers, trends, level_id=1)
print(f"\n实际检出背驰: {len(divs)} 个")
