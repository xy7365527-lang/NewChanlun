#!/usr/bin/env python
"""检查 c1 的详细信息。"""

from tests.test_divergence import _make_uptrend_segments
from newchan.a_center_v0 import centers_from_segments_v0

segs = _make_uptrend_segments()
centers = centers_from_segments_v0(segs, sustain_m=0)

print("Centers:")
for i, c in enumerate(centers):
    print(f"  c{i}: seg[{c.seg0}:{c.seg1}] dir={c.direction} ZG={c.high:.1f} ZD={c.low:.1f} GG={c.gg:.1f} DD={c.dd:.1f}")
    print(f"       g={c.g:.1f} d={c.d:.1f} terminated={c.terminated}")
