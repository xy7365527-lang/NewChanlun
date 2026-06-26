"""临时交叉验证（#84 Lead 验证门点3）：Rust theta_v0 SecondKind 线段层 vs Python a_segment_v1。

用**同一笔序列**（Rust 导出的 OKLO 前 2000 笔）喂 Python segments_from_strokes_v1，
对比 Rust 的 divide_segments 输出（隔离笔层口径差异，纯比线段层算法定义一致性）。

Lead 点3：不一致追溯 reference:22 + 67课博文。若来自 Python 启发式（extend_mode/MAX_SCAN）→ Rust 对。

运行：PYTHONPATH=src .venv/bin/python analysis/_xcheck_secondkind_v1.py
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE.parent / "src"))

from newchan.a_stroke import Stroke  # noqa: E402
from newchan.a_segment_v1 import segments_from_strokes_v1  # noqa: E402

_XCHECK = _HERE / "data_cache" / "_xcheck_oklo_strokes.json"


def main() -> int:
    data = json.loads(_XCHECK.read_text())
    raw_strokes = data["strokes"]
    rust_segs = [tuple(s) for s in data["rust_segments"]]  # [start_index, end_index]

    # Rust 笔 → Python Stroke（同一笔序列，confirmed=True 除最后一笔）。
    py_strokes = []
    for i, s in enumerate(raw_strokes):
        py_strokes.append(
            Stroke(
                i0=s["i0"],
                i1=s["i1"],
                direction=s["direction"],
                high=s["high"],
                low=s["low"],
                p0=s["p0"],
                p1=s["p1"],
                confirmed=(i < len(raw_strokes) - 1),
            )
        )

    # Python v1（strict + optimized 两种 extend_mode 都跑，看哪个更接近 Rust）。
    for mode in ("strict", "optimized"):
        py_segs = segments_from_strokes_v1(py_strokes, min_seg_strokes=3, extend_mode=mode)
        # Python Segment → (start_index, end_index) 用 i0/i1（与 Rust start_index/end_index 同口径）。
        py_pairs = [(seg.i0, seg.i1) for seg in py_segs]
        print(f"\n=== extend_mode={mode} ===")
        print(f"Python 段数={len(py_pairs)}  Rust 段数={len(rust_segs)}")

        # 端点对比（前 20 段）。
        n_cmp = min(len(py_pairs), len(rust_segs), 20)
        match = 0
        for k in range(n_cmp):
            r = tuple(rust_segs[k])
            p = py_pairs[k]
            same = (r[0] == p[0] and r[1] == p[1])
            if same:
                match += 1
            elif k < 10:
                print(f"  段{k}: Rust=({r[0]},{r[1]}) Python=({p[0]},{p[1]}) {'✓' if same else '✗'}")
        print(f"前 {n_cmp} 段端点一致: {match}/{n_cmp}")

    # 诊断：Rust 段数 / Python 段数比值（量级一致性）。
    return 0


if __name__ == "__main__":
    sys.exit(main())
