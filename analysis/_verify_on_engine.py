"""bit-exact 验证：day-batch O(N) σ 读法 ≡ 流式 RecursiveOrchestrator.current_moves()[-1]。

day-batch 思路：σ(day d) = _top_sigma(current_moves()) 只读末走势 kind/direction。
RecursiveOrchestrator.process_bar 内部对 stroke 前缀跑批量
  segments_from_strokes_v1 → zhongshu_from_segments → moves_from_zhongshus
（每次笔尾变化全量重算 → O(strokes²)）。
day-batch 改为：用独立 BiEngine 流式产笔（O(N)），仅在每日末对 current_strokes() 前缀
跑同一批量管线（~1900 次而非 ~strokes 次）→ O(days×strokes)=O(N)，输入相同 ⟹ 输出逐位等价。

本脚本在前缀上同时跑两条路径，断言 σ 日序列完全一致。
运行：PYTHONPATH=src .venv/bin/python analysis/_verify_on_engine.py
"""
from __future__ import annotations

import json
import sys
import time
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE.parent / "src"))
import newchan_rust  # noqa: E402

_DATA = _HERE / "data_cache"
_MAX_LEVELS = 6
_PREFIX = 400_000

# RecursiveOrchestrator 内部 BiEngine 参数（lib.rs:856 默认 stroke_mode="wide"）。
_BI = ("wide", 5, False, 3)


def _top_sigma(moves) -> int:
    """末走势方向 σ∈{+1,0,-1}。moves[-1]=((kind,direction,...),(...))。"""
    if not moves:
        return 0
    head = moves[-1][0]
    kind, direction = head[0], head[1]
    if kind == "consolidation":
        return 0
    return 1 if direction == "up" else -1


def _sigma_from_strokes(strokes) -> int:
    """day-batch：stroke 前缀 → seg → zs → move → 末走势 σ（复刻 orchestrator.process_bar）。"""
    segs = newchan_rust.segments_from_strokes_v1(strokes, 3, "strict")
    # SegmentTuple = ((s0,s1,i0,i1,dir,high,low,confirmed,kind),(ep...),(be...))
    seg_tuples = [
        (s[0][0], s[0][1], s[0][5], s[0][6], s[0][7], s[0][8] == "settled")
        for s in segs
    ]
    zss = newchan_rust.zhongshu_from_segments(seg_tuples)
    moves = newchan_rust.moves_from_zhongshus(zss, len(segs))
    return _top_sigma(moves)


def _load(sym_file: str, n: int):
    d = json.load(open(_DATA / sym_file))
    o, h, l, c = d["opens"][:n], d["highs"][:n], d["lows"][:n], d["closes"][:n]
    dates = d["dates"][:n]
    del d
    return o, h, l, c, dates


def _read_at(dates):
    last_i: dict[str, int] = {}
    for i, ts in enumerate(dates):
        last_i[ts[:10]] = i
    return {i: day for day, i in last_i.items()}


def verify_abs(sym_file: str, label: str) -> None:
    o, h, l, c, dates = _load(sym_file, _PREFIX)
    # 清洗：剔除非正/非有限。
    good = [i for i in range(len(c))
            if all(v > 0 and v == v and v not in (float("inf"), float("-inf"))
                   for v in (o[i], h[i], l[i], c[i]))]
    o, h, l, c = ([a[i] for i in good] for a in (o, h, l, c)) if len(good) < len(c) else (o, h, l, c)
    dates = [dates[i] for i in good] if len(good) < len(dates) else dates
    read_at = _read_at(dates)
    n = len(c)

    # 流式基准。
    orch = newchan_rust.RecursiveOrchestrator(max_levels=_MAX_LEVELS)
    stream: dict[str, int] = {}
    t0 = time.time()
    for i in range(n):
        orch.process_bar(o[i], h[i], l[i], c[i])
        day = read_at.get(i)
        if day is not None:
            stream[day] = _top_sigma(orch.current_moves())
    t_stream = time.time() - t0

    # day-batch。
    bi = newchan_rust.BiEngine(*_BI)
    batch: dict[str, int] = {}
    t0 = time.time()
    for i in range(n):
        bi.process_bar(o[i], h[i], l[i], c[i])
        day = read_at.get(i)
        if day is not None:
            batch[day] = _sigma_from_strokes(bi.current_strokes())
    t_batch = time.time() - t0

    diffs = [d for d in stream if stream[d] != batch.get(d, "MISSING")]
    print(f"[{label} abs] n={n} days={len(stream)} "
          f"stream={t_stream:.1f}s batch={t_batch:.1f}s "
          f"diffs={len(diffs)} {'✓ BIT-EXACT' if not diffs else '✗ MISMATCH'}")
    if diffs:
        for d in diffs[:10]:
            print(f"    {d}: stream={stream[d]} batch={batch.get(d)}")


def verify_ratio(fa: str, fb: str, label: str) -> None:
    ao, ah, al, ac, dates = _load(fa, _PREFIX)
    bo, bh, bl, bc, _ = _load(fb, _PREFIX)
    n = min(len(ac), len(bc))
    read_at = _read_at(dates[:n])

    orch = newchan_rust.RecursiveOrchestrator(max_levels=_MAX_LEVELS)
    stream: dict[str, int] = {}
    t0 = time.time()
    for i in range(n):
        orch.process_bar(ao[i] / bo[i], ah[i] / bl[i], al[i] / bh[i], ac[i] / bc[i])
        day = read_at.get(i)
        if day is not None:
            stream[day] = _top_sigma(orch.current_moves())
    t_stream = time.time() - t0

    bi = newchan_rust.BiEngine(*_BI)
    batch: dict[str, int] = {}
    t0 = time.time()
    for i in range(n):
        bi.process_bar(ao[i] / bo[i], ah[i] / bl[i], al[i] / bh[i], ac[i] / bc[i])
        day = read_at.get(i)
        if day is not None:
            batch[day] = _sigma_from_strokes(bi.current_strokes())
    t_batch = time.time() - t0

    diffs = [d for d in stream if stream[d] != batch.get(d, "MISSING")]
    print(f"[{label} ratio] n={n} days={len(stream)} "
          f"stream={t_stream:.1f}s batch={t_batch:.1f}s "
          f"diffs={len(diffs)} {'✓ BIT-EXACT' if not diffs else '✗ MISMATCH'}")
    if diffs:
        for d in diffs[:10]:
            print(f"    {d}: stream={stream[d]} batch={batch.get(d)}")


if __name__ == "__main__":
    verify_abs("dx_1m_databento_10y.json", "DX")
    verify_ratio("gc_1m_databento_10y.json", "cl_1m_databento_10y.json", "ω=GC/CL")
