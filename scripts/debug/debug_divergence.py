#!/usr/bin/env python
"""调试背驰测试数据。"""

from tests.test_divergence import _make_uptrend_segments
from newchan.a_center_v0 import centers_from_segments_v0
from newchan.a_trendtype_v0 import trend_instances_from_centers
from newchan.a_divergence import divergences_from_level

segs = _make_uptrend_segments()
print(f'Segments: {len(segs)}')
for i, s in enumerate(segs):
    print(f'  seg{i}: {s.direction:4s} i={s.i0:3d}-{s.i1:3d} H={s.high:.1f} L={s.low:.1f}')

centers = centers_from_segments_v0(segs, sustain_m=0)
print(f'\nCenters: {len(centers)}')
for i, c in enumerate(centers):
    print(f'  c{i}: seg[{c.seg0}:{c.seg1}] ZG={c.high:.1f} ZD={c.low:.1f}')

trends = trend_instances_from_centers(segs, centers)
print(f'\nTrends: {len(trends)}')
for i, t in enumerate(trends):
    print(f'  t{i}: {t.kind:13s} {t.direction:4s} seg[{t.seg0}:{t.seg1}] centers={t.center_indices}')

divs = divergences_from_level(segs, centers, trends, level_id=1)
print(f'\nDivergences: {len(divs)}')
for d in divs:
    print(f'  {d.kind} {d.direction} A[{d.seg_a_start}:{d.seg_a_end}]={d.force_a:.1f} C[{d.seg_c_start}:{d.seg_c_end}]={d.force_c:.1f}')
