"""端到端归因 harness（#84 真因定位）：在 Python 侧复刻 theta_v0 算法，逐变量切换到参考。

目的：theta_v0=406 段 vs Python 参考=237 段（71% 过分段）。Lead 假设 inclusion 外包络是
根因，但 inclusion 探针显示外包络产生**更少**分型（275<307），方向与过分段矛盾。本 harness
隔离三个候选变量，单变量切换定位真因：

  V1 inclusion：外包络(theta_v0) vs 方向性(Python)
  V2 假设转折点（71课）：批处理无此逻辑(theta_v0) vs 增量先试不合并(Python)
  V3 段端定位 + 缺口判据：批量找首分型(theta_v0) vs 增量 scan_trigger+min_seg_strokes(Python)

复刻 theta_v0 的 divide_segments 算法（segment.rs:282-329 + analyze_termination:215-259），
用同一笔序列，看能否复现 406。能复现 → 逐变量切回参考，看哪个变量使段数收敛到 237。

参考语义 = a_segment_v1.segments_from_strokes_v1（编排者裁定唯一口径）。

## 实测结论（认识论诚实，formalization-validity-domain）

复刻成功复现 406（bit-exact 复刻 theta_v0 批处理算法）。但**三变量局部消融无法线性分解
169 段 gap**：
- V1 (inclusion 外包络→方向性)：406→404，仅 Δ=-2（1.2%）。**否定 codex「inclusion 是
  根因」断言**。
- V2 (参考侧关假设转折点)：237→221，Δ=-16（方向相反，且在参考侧而非 theta_v0 侧）。
- V3 (参考侧 min_seg=1)：237→237，Δ=0（OKLO 数据上无约束）。

真因不是任一单变量，而是**算法范式整体性差异**：theta_v0 用「无状态批处理找首个分型即断段」，
参考用「增量逐笔 + 71课假设转折点状态机」。二者是不同算法，169 gap 来自范式本身，
不能用零件替换桥接。修复 = 重写 theta_v0 divide_segments 为参考增量状态机的忠实移植
（非在批处理上调 inclusion——那是补丁思维，no-patch-mentality）。

第71课博文（一级权威）:38/:42 确认参考「假设转折点」逻辑是正确缠论语义。

运行：PYTHONPATH=src .venv/bin/python analysis/_attrib_segment_divergence.py
"""
from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

_HERE = Path(__file__).resolve().parent
_XCHECK = _HERE / "data_cache" / "_xcheck_oklo_strokes.json"


@dataclass(frozen=True)
class Stk:
    direction: str
    i0: int
    i1: int
    p0: int
    p1: int
    high: int
    low: int

    @property
    def lo(self) -> int:
        return min(self.p0, self.p1)

    @property
    def hi(self) -> int:
        return max(self.p0, self.p1)


def load_strokes() -> list[Stk]:
    data = json.loads(_XCHECK.read_text())
    out = []
    for s in data["strokes"]:
        out.append(Stk(s["direction"], s["i0"], s["i1"], s["p0"], s["p1"], s["high"], s["low"]))
    return out, len(data["rust_segments"])


# ---- theta_v0 原语复刻（segment.rs 移植，[lo,hi] = stroke_interval 几何投影）----

def stroke_iv(s: Stk) -> tuple[int, int]:
    return (s.lo, s.hi)  # (lo, hi)


def feature_elements(seg_dir: str, strokes: list[Stk]) -> list[tuple[int, int]]:
    rev = "down" if seg_dir == "up" else "up"
    return [stroke_iv(s) for s in strokes if s.direction == rev]


def incl_envelope(elems: list[tuple[int, int]]) -> list[tuple[int, int]]:
    """theta_v0 外包络（contains 双向判定 → [min(lo),max(hi)]）。"""
    out: list[list[int]] = []
    for lo, hi in elems:
        if out:
            plo, phi = out[-1]
            prev_contains = plo <= lo and hi <= phi
            e_contains = lo <= plo and phi <= hi
            if prev_contains or e_contains:
                out[-1][0] = min(plo, lo)
                out[-1][1] = max(phi, hi)
                continue
        out.append([lo, hi])
    return [(lo, hi) for lo, hi in out]


def incl_directional(seg_dir: str, elems: list[tuple[int, int]]) -> list[tuple[int, int]]:
    """Python 方向性合并（向上 max/max，向下 min/min）。"""
    out: list[list[int]] = []
    dir_state = "DOWN" if seg_dir == "down" else None
    for lo, hi in elems:
        if not out:
            out.append([lo, hi])
            continue
        plo, phi = out[-1]
        left_inc = phi >= hi and plo <= lo
        right_inc = hi >= phi and lo <= plo
        if left_inc or right_inc:
            effective_up = dir_state != "DOWN"
            if effective_up:
                out[-1][1] = max(phi, hi)
                out[-1][0] = max(plo, lo)
            else:
                out[-1][1] = min(phi, hi)
                out[-1][0] = min(plo, lo)
        else:
            if hi > phi and lo > plo:
                dir_state = "UP"
            elif hi < phi and lo < plo:
                dir_state = "DOWN"
            out.append([lo, hi])
    return [(lo, hi) for lo, hi in out]


def find_feature_fractal(seg_dir: str, std: list[tuple[int, int]]):
    """theta_v0 find_feature_fractal：首个目标方向分型 → (i, e1, e2)。"""
    if len(std) < 3:
        return None
    for i in range(1, len(std) - 1):
        l, m, r = std[i - 1], std[i], std[i + 1]
        is_top = m[1] > l[1] and m[1] > r[1]
        is_bottom = m[0] < l[0] and m[0] < r[0]
        matched = is_top if seg_dir == "up" else is_bottom
        if matched:
            return (i, l, m)
    return None


def iv_gap(e1: tuple[int, int], e2: tuple[int, int]) -> bool:
    """缺口 = 无重叠。overlaps = a.lo<=b.hi && b.lo<=a.hi。"""
    overlaps = e1[0] <= e2[1] and e2[0] <= e1[1]
    return not overlaps


def three_stroke_overlap(a: Stk, b: Stk, c: Stk) -> bool:
    max_lo = max(a.lo, b.lo, c.lo)
    min_hi = min(a.hi, b.hi, c.hi)
    return max_lo < min_hi


def find_overlap_start(strokes: list[Stk], frm: int):
    n = len(strokes)
    j = frm
    while j + 2 < n:
        if three_stroke_overlap(strokes[j], strokes[j + 1], strokes[j + 2]):
            return j
        j += 1
    return None


def second_kind_confirmed(strokes: list[Stk], apex_off: int, incl_fn) -> bool:
    """theta_v0 second_kind：apex 后同向笔构第二特征序列，任意分型即确认。"""
    if not strokes:
        return False
    seg_dir = strokes[0].direction
    second = [stroke_iv(s) for s in strokes[apex_off + 1:] if s.direction == seg_dir]
    std = incl_fn(seg_dir, second) if incl_fn.__code__.co_argcount == 2 else incl_fn(second)
    if len(std) < 3:
        return False
    for i in range(1, len(std) - 1):
        l, m, r = std[i - 1], std[i], std[i + 1]
        if (m[1] > l[1] and m[1] > r[1]) or (m[0] < l[0] and m[0] < r[0]):
            return True
    return False


def analyze_termination(strokes: list[Stk], incl_mode: str):
    """theta_v0 analyze_termination 复刻。返回 ('first'|'second'|'pending'|'none', end_offset)。"""
    if not strokes:
        return ("none", 0)
    seg_dir = strokes[0].direction
    feat = feature_elements(seg_dir, strokes)
    if incl_mode == "envelope":
        std = incl_envelope(feat)
    else:
        std = incl_directional(seg_dir, feat)
    ff = find_feature_fractal(seg_dir, std)
    if ff is None:
        return ("none", 0)
    _, e1, e2 = ff
    rev = "down" if seg_dir == "up" else "up"
    # 定位 apex 反向笔（与 e2 重叠的反向笔）。
    apex_off = None
    for off, s in enumerate(strokes):
        if s.direction == rev:
            iv = stroke_iv(s)
            if iv[0] <= e2[1] and e2[0] <= iv[1]:
                apex_off = off
                break
    if apex_off is None:
        return ("none", 0)
    if not iv_gap(e1, e2):
        # FirstKind：段端 = apex_off - 1。
        if apex_off == 0:
            return ("none", 0)
        return ("first", apex_off - 1)
    # SecondKind 动态确认。
    incl_fn = incl_directional if incl_mode == "directional" else incl_envelope
    if second_kind_confirmed(strokes, apex_off, incl_fn):
        if apex_off == 0:
            return ("pending", 0)
        return ("second", apex_off - 1)
    return ("pending", 0)


def divide_segments_theta(strokes: list[Stk], incl_mode: str) -> list[tuple[int, int]]:
    """theta_v0 divide_segments_with_tail 复刻（无 min_seg 门控 = theta_v0 原状）。"""
    segs = []
    if not strokes:
        return segs
    start = find_overlap_start(strokes, 0)
    if start is None:
        return segs
    while start + 2 < len(strokes):
        seg_dir = strokes[start].direction
        rest = strokes[start:]
        kind, off = analyze_termination(rest, incl_mode)
        if kind in ("pending", "none"):
            break
        end_idx = start + off
        end_stroke = strokes[end_idx]
        segs.append((strokes[start].i0, max(end_stroke.i1, end_stroke.i0)))
        start = max(end_idx + 1, start + 1)
    return segs


def main() -> int:
    strokes, rust_seg_count = load_strokes()
    print(f"笔数={len(strokes)}  导出的 Rust 段数(参考)={rust_seg_count}")

    # 复刻 theta_v0（外包络）。
    segs_env = divide_segments_theta(strokes, "envelope")
    print(f"\n[复刻] theta_v0 外包络批处理:  {len(segs_env)} 段  (目标复现 Rust 406)")

    # 单变量切换 V1：inclusion 改方向性，其他不变（仍批处理、无假设转折点）。
    segs_dir = divide_segments_theta(strokes, "directional")
    print(f"[V1切换] inclusion→方向性(其他不变): {len(segs_dir)} 段")

    # Python 完整参考。
    from newchan.a_stroke import Stroke as PyStroke
    from newchan.a_segment_v1 import segments_from_strokes_v1
    py_strokes = [
        PyStroke(i0=s.i0, i1=s.i1, direction=s.direction, high=s.high, low=s.low,
                 p0=s.p0, p1=s.p1, confirmed=(i < len(strokes) - 1))
        for i, s in enumerate(strokes)
    ]
    py_segs = segments_from_strokes_v1(py_strokes, min_seg_strokes=3, extend_mode="strict")
    print(f"[参考] Python 完整 a_segment_v1: {len(py_segs)} 段  (V1+V2+V3 全部参考)")

    # 消融 V2：Python 参考侧关闭 71课「假设转折点」（包含时总是合并，不试探不合并）。
    import newchan.a_segment_v1 as seg_mod
    orig_append = seg_mod._FeatureSeqState.append

    def append_no_assume(self, stroke_idx, high, low, seg_direction="up", strokes=None):
        # 强制 strokes=None → 跳过 scan_trigger 试探（:332），即总是合并（关闭假设转折点）。
        return orig_append(self, stroke_idx, high, low, seg_direction, None)

    seg_mod._FeatureSeqState.append = append_no_assume
    try:
        py_segs_no_assume = segments_from_strokes_v1(py_strokes, min_seg_strokes=3, extend_mode="strict")
    finally:
        seg_mod._FeatureSeqState.append = orig_append
    print(f"[消融V2] Python 关闭假设转折点(总合并): {len(py_segs_no_assume)} 段")

    # 消融 V3：Python min_seg_strokes=1（关闭最小段长门控）。
    py_segs_min1 = segments_from_strokes_v1(py_strokes, min_seg_strokes=1, extend_mode="strict")
    print(f"[消融V3] Python min_seg_strokes=1(关门控): {len(py_segs_min1)} 段")

    print("\n--- 归因 ---")
    print(f"V1 (inclusion 外包络→方向性) 单独: {len(segs_env)} → {len(segs_dir)} "
          f"(Δ={len(segs_dir)-len(segs_env)}, {100*abs(len(segs_dir)-len(segs_env))/(len(segs_env)-len(py_segs)):.1f}% of total gap)")
    print(f"V2 (假设转折点) 在参考侧关闭使段数: {len(py_segs)} → {len(py_segs_no_assume)} "
          f"(Δ={len(py_segs_no_assume)-len(py_segs)})")
    print(f"V3 (min_seg 门控) 在参考侧关闭使段数: {len(py_segs)} → {len(py_segs_min1)} "
          f"(Δ={len(py_segs_min1)-len(py_segs)})")
    print(f"\n总 gap (theta_v0 - 参考) = {len(segs_env) - len(py_segs)} 段")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
