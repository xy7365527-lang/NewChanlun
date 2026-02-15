#!/usr/bin/env python
"""尝试用 terminated 来限制 c0 的范围。"""

from newchan.a_segment_v0 import Segment
from newchan.a_center_v0 import centers_from_segments_v0
from newchan.a_trendtype_v0 import trend_instances_from_centers, _centers_relation

def _seg(s0, s1, i0, i1, d, h, l, confirmed=True):
    return Segment(s0=s0, s1=s1, i0=i0, i1=i1, direction=d,
                   high=h, low=l, confirmed=confirmed)

# 策略：让 seg3 离开中枢，seg4 不回抽到中枢内，从而终止 c0
segs = [
    _seg(0, 0, 0, 10, "up", 20, 10),      # seg0
    _seg(1, 1, 10, 20, "down", 20, 12),   # seg1
    _seg(2, 2, 20, 30, "up", 18, 12),     # seg2，c0 初始三段
    _seg(3, 3, 30, 40, "down", 11, 8),    # seg3 完全低于 ZD=12
    _seg(4, 4, 40, 50, "up", 11, 8),      # seg4 完全低于 ZD=12，不回抽
    _seg(5, 5, 50, 60, "down", 25, 20),   # seg5
    _seg(6, 6, 60, 70, "up", 30, 20),     # seg6
    _seg(7, 7, 70, 80, "down", 30, 28),   # seg7
    _seg(8, 8, 80, 90, "up", 32, 28),     # seg8
    _seg(9, 9, 90, 100, "down", 32, 30),  # seg9
    _seg(10, 10, 100, 110, "up", 33, 30), # seg10
]

centers = centers_from_segments_v0(segs, sustain_m=0)
print("Centers:")
for i, c in enumerate(centers):
    print(f"  c{i}: seg[{c.seg0}:{c.seg1}] ZG={c.high:.1f} ZD={c.low:.1f} GG={c.gg:.1f} DD={c.dd:.1f} term={c.terminated}")

if len(centers) >= 2:
    rel = _centers_relation(centers[0], centers[1])
    print(f"\nRelation c0 -> c1: {rel}")
    c0, c1 = centers[0], centers[1]
    print(f"  c1.DD ({c1.dd:.1f}) > c0.GG ({c0.gg:.1f}): {c1.dd > c0.gg}")

trends = trend_instances_from_centers(segs, centers)
print(f"\nTrends: {len(trends)}")
for i, t in enumerate(trends):
    print(f"  t{i}: {t.kind:13s} {t.direction:4s} seg[{t.seg0}:{t.seg1}] centers={t.center_indices}")
