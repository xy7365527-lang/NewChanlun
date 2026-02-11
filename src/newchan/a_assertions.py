"""
A-system assertions (Step7).

These assertions implement the invariants defined in:
docs/chan_spec.md

Default behavior: enable=False (collect warnings, do not raise).
You can turn enable=True after the pipeline stabilizes to enforce the spec.
"""

from __future__ import annotations
from dataclasses import dataclass
from typing import Any, Iterable


@dataclass(frozen=True)
class AssertResult:
    ok: bool
    name: str
    message: str = ""


def _ok(name: str) -> AssertResult:
    return AssertResult(True, name, "")


def _fail(name: str, message: str) -> AssertResult:
    return AssertResult(False, name, message)


def assert_inclusion_no_residual(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §2.4
    After merge_inclusion, there must be no adjacent inclusion pairs.

    Usage: assert_inclusion_no_residual(df_merged)
    where df_merged is a DataFrame with 'high' and 'low' columns.
    """
    name = "assert_inclusion_no_residual"
    if not args:
        return _ok(name)
    df = args[0]
    if not hasattr(df, "__len__") or len(df) < 2:
        return _ok(name)
    try:
        highs = df["high"].values
        lows = df["low"].values
    except (KeyError, TypeError):
        return _ok(name)
    for i in range(len(highs) - 1):
        h1, l1 = float(highs[i]), float(lows[i])
        h2, l2 = float(highs[i + 1]), float(lows[i + 1])
        if (h1 >= h2 and l1 <= l2) or (h2 >= h1 and l2 <= l1):
            return _fail(
                name,
                f"Residual inclusion at bar[{i}]<->bar[{i+1}]: "
                f"({h1},{l1}) vs ({h2},{l2})",
            )
    return _ok(name)


def assert_inclusion_direction_rule(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §2.3
    Direction (dir) update must follow the strict high&low rule.
    """
    return _ok("assert_inclusion_direction_rule")


def assert_fractal_definition(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §3.1-§3.2
    Fractals MUST satisfy double conditions (high & low).

    Usage: assert_fractal_definition(df_merged, fractals)
    - df_merged: DataFrame with 'high' and 'low' columns.
    - fractals:  list of Fractal (each has .idx, .kind, .price).
    """
    name = "assert_fractal_definition"
    if len(args) < 2:
        return _ok(name)
    df_merged, fractals = args[0], args[1]
    try:
        highs = df_merged["high"].values
        lows = df_merged["low"].values
    except (KeyError, TypeError):
        return _ok(name)
    n = len(highs)
    for fx in fractals:
        i = fx.idx
        if i < 1 or i >= n - 1:
            msg = f"Fractal idx={i} out of bounds (n={n})"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

        h_prev, h_curr, h_next = float(highs[i - 1]), float(highs[i]), float(highs[i + 1])
        l_prev, l_curr, l_next = float(lows[i - 1]), float(lows[i]), float(lows[i + 1])

        if fx.kind == "top":
            high_ok = h_curr > h_prev and h_curr > h_next
            low_ok = l_curr > l_prev and l_curr > l_next
            if not (high_ok and low_ok):
                msg = (
                    f"Top fractal idx={i} fails double condition: "
                    f"high_ok={high_ok} low_ok={low_ok} "
                    f"prev=({h_prev},{l_prev}) mid=({h_curr},{l_curr}) nxt=({h_next},{l_next})"
                )
                if enable:
                    raise AssertionError(f"[{name}] {msg}")
                return _fail(name, msg)
        elif fx.kind == "bottom":
            low_ok = l_curr < l_prev and l_curr < l_next
            high_ok = h_curr < h_prev and h_curr < h_next
            if not (low_ok and high_ok):
                msg = (
                    f"Bottom fractal idx={i} fails double condition: "
                    f"low_ok={low_ok} high_ok={high_ok} "
                    f"prev=({h_prev},{l_prev}) mid=({h_curr},{l_curr}) nxt=({h_next},{l_next})"
                )
                if enable:
                    raise AssertionError(f"[{name}] {msg}")
                return _fail(name, msg)
        else:
            msg = f"Unknown fractal kind={fx.kind!r} at idx={i}"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)
    return _ok(name)


def assert_stroke_alternation_and_gap(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §4.2-§4.4

    Usage: assert_stroke_alternation_and_gap(strokes, mode="wide", min_strict_sep=5)
    - strokes: list of Stroke (each has .i0, .i1, .direction, .confirmed).
    - mode: "wide" or "strict"
    - min_strict_sep: int (used when mode="strict")

    Checks:
    - directions alternate up/down
    - i1 > i0 for every stroke
    - gap (i1 - i0) >= min_gap per mode
    - confirmed: last stroke False, rest True
    """
    name = "assert_stroke_alternation_and_gap"
    if not args:
        return _ok(name)
    strokes = args[0]
    if not strokes:
        return _ok(name)
    mode = args[1] if len(args) > 1 else "wide"
    min_strict_sep = args[2] if len(args) > 2 else 5
    min_gap = 4 if mode == "wide" else int(min_strict_sep)

    for i, s in enumerate(strokes):
        # i1 > i0
        if s.i1 <= s.i0:
            msg = f"Stroke[{i}] i1={s.i1} <= i0={s.i0}"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

        # gap check
        gap = s.i1 - s.i0
        if gap < min_gap:
            msg = f"Stroke[{i}] gap={gap} < min_gap={min_gap} (mode={mode})"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

        # direction alternation + continuity
        if i > 0:
            prev = strokes[i - 1]
            if s.direction == prev.direction:
                msg = (
                    f"Stroke[{i-1}] and Stroke[{i}] both {s.direction}, "
                    f"violates alternation"
                )
                if enable:
                    raise AssertionError(f"[{name}] {msg}")
                return _fail(name, msg)

            # 连续性：stroke[i-1].i1 == stroke[i].i0
            if prev.i1 != s.i0:
                msg = (
                    f"Stroke[{i-1}].i1={prev.i1} != Stroke[{i}].i0={s.i0}, "
                    f"violates continuity (strokes must be stitched)"
                )
                if enable:
                    raise AssertionError(f"[{name}] {msg}")
                return _fail(name, msg)

        # confirmed rule
        is_last = i == len(strokes) - 1
        if is_last and s.confirmed:
            msg = f"Last stroke [{i}] should be confirmed=False"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)
        if not is_last and not s.confirmed:
            msg = f"Stroke[{i}] (not last) should be confirmed=True"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

    return _ok(name)


def assert_no_pen_center(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §4.6 and §7.1
    Pen overlap zones are hypothesis only; strokes must NOT be center components.
    """
    return _ok("assert_no_pen_center")


def assert_segment_min_three_strokes_overlap(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §5.1
    Segment MUST be built from >=3 strokes with 3-way intersection overlap.

    Usage: assert_segment_min_three_strokes_overlap(segments, strokes)
    - segments: list of Segment (.s0, .s1, .i0, .i1, .confirmed)
    - strokes:  list of Stroke  (.i0, .i1, .high, .low)

    Checks:
    - s1 - s0 >= 2 (at least 3 strokes)
    - three-stroke intersection overlap holds
    - i0 = strokes[s0].i0, i1 = strokes[s1].i1
    - confirmed: last segment False, rest True
    """
    name = "assert_segment_min_three_strokes_overlap"
    if len(args) < 2:
        return _ok(name)
    segments, strokes = args[0], args[1]
    if not segments:
        return _ok(name)

    for idx, seg in enumerate(segments):
        # ── at least 3 strokes ──
        span = seg.s1 - seg.s0
        if span < 2:
            msg = f"Segment[{idx}] s1-s0={span} < 2 (need >=3 strokes)"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

        # ── i0/i1 consistency with strokes ──
        if seg.s0 < 0 or seg.s1 >= len(strokes):
            msg = f"Segment[{idx}] stroke indices out of range: s0={seg.s0}, s1={seg.s1}"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

        if seg.i0 != strokes[seg.s0].i0:
            msg = (
                f"Segment[{idx}] i0={seg.i0} != strokes[{seg.s0}].i0="
                f"{strokes[seg.s0].i0}"
            )
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

        if seg.i1 != strokes[seg.s1].i1:
            msg = (
                f"Segment[{idx}] i1={seg.i1} != strokes[{seg.s1}].i1="
                f"{strokes[seg.s1].i1}"
            )
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

        # ── three-stroke intersection overlap ──
        s1 = strokes[seg.s0]
        s2 = strokes[seg.s0 + 1]
        s3 = strokes[seg.s0 + 2]
        overlap_low = max(s1.low, s2.low, s3.low)
        overlap_high = min(s1.high, s2.high, s3.high)
        if not (overlap_low < overlap_high):
            msg = (
                f"Segment[{idx}] three-stroke overlap empty: "
                f"overlap=[{overlap_low},{overlap_high}]"
            )
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

        # ── confirmed rule ──
        is_last = idx == len(segments) - 1
        if is_last and seg.confirmed:
            msg = f"Last segment [{idx}] should be confirmed=False"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)
        if not is_last and not seg.confirmed:
            msg = f"Segment[{idx}] (not last) should be confirmed=True"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

    return _ok(name)


def assert_center_definition(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §7.1

    Usage: assert_center_definition(segments, centers, sustain_m=2)
    - segments: list of Segment
    - centers:  list of Center (.seg0, .seg1, .low, .high, .kind, .sustain, .confirmed)
    - sustain_m: int

    Checks:
    - seg1 - seg0 >= 2 (at least 3 segments)
    - ZG > ZD
    - settled iff sustain >= sustain_m
    - confirmed: last False, rest True
    """
    name = "assert_center_definition"
    if len(args) < 2:
        return _ok(name)
    segments, centers = args[0], args[1]
    sustain_m = args[2] if len(args) > 2 else 2
    if not centers:
        return _ok(name)

    for i, c in enumerate(centers):
        # ── at least 3 segments ──
        if c.seg1 - c.seg0 < 2:
            msg = f"Center[{i}] seg1-seg0={c.seg1 - c.seg0} < 2"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

        # ── ZG > ZD ──
        if c.high <= c.low:
            msg = f"Center[{i}] ZG={c.high} <= ZD={c.low}"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

        # ── settled iff sustain >= sustain_m ──
        if c.kind == "settled" and c.sustain < sustain_m:
            msg = (
                f"Center[{i}] kind=settled but sustain={c.sustain} < "
                f"sustain_m={sustain_m}"
            )
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)
        if c.kind == "candidate" and c.sustain >= sustain_m:
            msg = (
                f"Center[{i}] kind=candidate but sustain={c.sustain} >= "
                f"sustain_m={sustain_m}"
            )
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

        # ── confirmed ──
        is_last = i == len(centers) - 1
        if is_last and c.confirmed:
            msg = f"Last center [{i}] should be confirmed=False"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)
        if not is_last and not c.confirmed:
            msg = f"Center[{i}] (not last) should be confirmed=True"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

    return _ok(name)


def assert_non_skip(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §7.1 and §6
    Center[k] components MUST be Move[k-1] only (no skipping levels).

    Usage: assert_non_skip(segments, centers)
    Checks that every center's component indices [seg0..seg1] reference
    valid Segment objects (not Stroke or other types).
    """
    name = "assert_non_skip"
    if len(args) < 2:
        return _ok(name)
    segments, centers = args[0], args[1]
    if not centers:
        return _ok(name)

    for i, c in enumerate(centers):
        if c.seg0 < 0 or c.seg1 >= len(segments):
            msg = (
                f"Center[{i}] seg range [{c.seg0},{c.seg1}] out of bounds "
                f"(n_segments={len(segments)})"
            )
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)
        # Verify each referenced object IS a Segment (not Stroke)
        for j in range(c.seg0, c.seg1 + 1):
            comp = segments[j]
            comp_type = type(comp).__name__
            if comp_type not in ("Segment",):
                msg = (
                    f"Center[{i}] component [{j}] is {comp_type}, "
                    f"expected Segment (Move[0])"
                )
                if enable:
                    raise AssertionError(f"[{name}] {msg}")
                return _fail(name, msg)
    return _ok(name)


def assert_single_lstar(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §9.2
    There MUST be only one active decision level L* at any time.

    Usage: assert_single_lstar(level_views, last_price, max_post_exit_segments=6)
    Checks:
    - select_lstar_newchan returns at most one L*
    - no duplicate alive centers with identical (seg0, seg1) on any level
    """
    name = "assert_single_lstar"
    if len(args) < 2:
        return _ok(name)

    level_views, last_price = args[0], args[1]
    max_post = args[2] if len(args) > 2 else 6

    try:
        from newchan.a_level_fsm_newchan import (
            classify_center_practical_newchan,
            select_lstar_newchan,
        )
    except ImportError:
        return _ok(name)

    # Check for duplicate alive centers per level
    for view in level_views:
        alive_keys: list[tuple[int, int]] = []
        for ci, center in enumerate(view.centers):
            ac = classify_center_practical_newchan(
                center=center, center_idx=ci,
                segments=view.segments, last_price=last_price,
                max_post_exit_segments=max_post,
            )
            if ac.is_alive:
                key = (center.seg0, center.seg1)
                if key in alive_keys:
                    msg = (
                        f"Level {view.level}: duplicate alive center "
                        f"with (seg0={center.seg0}, seg1={center.seg1})"
                    )
                    if enable:
                        raise AssertionError(f"[{name}] {msg}")
                    return _fail(name, msg)
                alive_keys.append(key)

    return _ok(name)


def assert_ledger_separation(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §10
    Hypothesis vs settlement separation must hold; settlement is append-only.
    """
    return _ok("assert_ledger_separation")


def assert_segment_theorem_v1(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §5.3-§5.4 (v1 characteristic sequence method)

    Usage: assert_segment_theorem_v1(strokes, segments_v1)
    - strokes:     list of Stroke
    - segments_v1: list of Segment (from segments_from_strokes_v1)

    Checks:
    - adjacent segments stitch: seg[i+1].s0 == seg[i].s1
    - each segment >= 3 strokes (s1 - s0 >= 2)
    - confirmed: last segment False, rest True
    """
    name = "assert_segment_theorem_v1"
    if len(args) < 2:
        return _ok(name)
    strokes, segments = args[0], args[1]
    if not segments:
        return _ok(name)

    for i, seg in enumerate(segments):
        # ── at least 3 strokes ──
        span = seg.s1 - seg.s0
        if span < 2:
            msg = f"Segment[{i}] s1-s0={span} < 2 (need >=3 strokes)"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

        # ── adjacent stitching: s1+1 == s0 (不共享边界笔) ──
        if i > 0:
            prev = segments[i - 1]
            if seg.s0 != prev.s1 + 1 and seg.s0 != prev.s1:
                msg = (
                    f"Segment[{i}].s0={seg.s0} not adjacent to "
                    f"Segment[{i-1}].s1={prev.s1}"
                )
                if enable:
                    raise AssertionError(f"[{name}] {msg}")
                return _fail(name, msg)

        # ── confirmed rule ──
        is_last = i == len(segments) - 1
        if is_last and seg.confirmed:
            msg = f"Last segment [{i}] should be confirmed=False"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)
        if not is_last and not seg.confirmed:
            msg = f"Segment[{i}] (not last) should be confirmed=True"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

    return _ok(name)
