#!/usr/bin/env python
"""调试中枢关系判定。"""

from tests.test_divergence import _make_uptrend_segments
from newchan.a_center_v0 import centers_from_segments_v0
from newchan.a_trendtype_v0 import _centers_relation

segs = _make_uptrend_segments()
centers = centers_from_segments_v0(segs, sustain_m=0)

print("Centers:")
for i, c in enumerate(centers):
    print(f"  c{i}: seg[{c.seg0}:{c.seg1}] ZG={c.high:.1f} ZD={c.low:.1f} GG={c.gg:.1f} DD={c.dd:.1f}")

if len(centers) >= 2:
    rel = _centers_relation(centers[0], centers[1])
    print(f"\nRelation c0 -> c1: {rel}")

    # 手动验证条件
    c0, c1 = centers[0], centers[1]
    print(f"\n判定条件:")
    print(f"  c1.DD ({c1.dd:.1f}) > c0.GG ({c0.gg:.1f}): {c1.dd > c0.gg} → should be 'up'")
    print(f"  c1.GG ({c1.gg:.1f}) < c0.DD ({c0.dd:.1f}): {c1.gg < c0.dd} → should be 'down'")
    print(f"  c1.ZG ({c1.high:.1f}) > c0.ZG ({c0.high:.1f}): {c1.high > c0.high}")
    print(f"  c1.ZD ({c1.low:.1f}) > c0.ZD ({c0.low:.1f}): {c1.low > c0.low}")
