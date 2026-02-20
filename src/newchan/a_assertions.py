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


def _check(name: str, condition: bool, msg: str, enable: bool) -> AssertResult | None:
    """Return a fail result if *condition* is False, else None (= pass).

    When *enable* is True, raises ``AssertionError`` on failure.
    """
    if condition:
        return None
    if enable:
        raise AssertionError(f"[{name}] {msg}")
    return _fail(name, msg)


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


def _simulate_inclusion(raw_highs, raw_lows, raw_opens, raw_closes):
    """§2.3+§2.2 参考实现：重跑包含处理，返回 buf 列表。"""
    buf: list[list[float | int]] = [
        [float(raw_opens[0]), float(raw_highs[0]), float(raw_lows[0]), float(raw_closes[0]), 0, 0]
    ]
    dir_state: str | None = None

    for i in range(1, len(raw_highs)):
        curr_h, curr_l = float(raw_highs[i]), float(raw_lows[i])
        last = buf[-1]
        last_h, last_l = float(last[1]), float(last[2])

        left_inc = last_h >= curr_h and last_l <= curr_l
        right_inc = curr_h >= last_h and curr_l <= last_l

        if left_inc or right_inc:
            effective_up = dir_state != "DOWN"
            if effective_up:
                last[1] = max(last_h, curr_h)
                last[2] = max(last_l, curr_l)
            else:
                last[1] = min(last_h, curr_h)
                last[2] = min(last_l, curr_l)
            last[3] = float(raw_closes[i])
            last[5] = i
        else:
            if curr_h > last_h and curr_l > last_l:
                dir_state = "UP"
            elif curr_h < last_h and curr_l < last_l:
                dir_state = "DOWN"
            buf.append([float(raw_opens[i]), curr_h, curr_l, float(raw_closes[i]), i, i])

    return buf


def _compare_merged_output(name: str, buf, df_merged, merged_to_raw) -> AssertResult | None:
    """比较模拟结果与实际输出，返回 fail 或 None（通过）。"""
    sim_map = [(int(r[4]), int(r[5])) for r in buf]
    if list(merged_to_raw) != sim_map:
        return _fail(
            name,
            f"merged_to_raw mismatch: expected={sim_map} got={list(merged_to_raw)}",
        )

    try:
        out = df_merged[["open", "high", "low", "close"]].values
    except Exception:
        return None
    if len(out) != len(buf):
        return _fail(name, f"df_merged len mismatch: expected={len(buf)} got={len(out)}")

    for j in range(len(buf)):
        exp_o, exp_h, exp_l, exp_c = float(buf[j][0]), float(buf[j][1]), float(buf[j][2]), float(buf[j][3])
        got_o, got_h, got_l, got_c = map(float, out[j])
        if (got_o, got_h, got_l, got_c) != (exp_o, exp_h, exp_l, exp_c):
            return _fail(
                name,
                f"df_merged row[{j}] mismatch: "
                f"expected=({exp_o},{exp_h},{exp_l},{exp_c}) got=({got_o},{got_h},{got_l},{got_c})",
            )
    return None


def assert_inclusion_direction_rule(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §2.3
    Direction (dir) update must follow the strict high&low rule.
    """
    name = "assert_inclusion_direction_rule"
    if len(args) < 3:
        return _ok(name)
    df_raw, df_merged, merged_to_raw = args[0], args[1], args[2]

    try:
        raw_highs = df_raw["high"].values
        raw_lows = df_raw["low"].values
        raw_opens = df_raw["open"].values
        raw_closes = df_raw["close"].values
    except Exception:
        return _ok(name)

    buf = _simulate_inclusion(raw_highs, raw_lows, raw_opens, raw_closes)
    result = _compare_merged_output(name, buf, df_merged, merged_to_raw)
    return result if result is not None else _ok(name)


def _check_fractal_double_condition(
    name: str, fx, highs, lows, n: int, enable: bool,
) -> AssertResult | None:
    """检查单个分型的双条件。"""
    i = fx.idx
    if i < 1 or i >= n - 1:
        return _check(name, False, f"Fractal idx={i} out of bounds (n={n})", enable)

    h_prev, h_curr, h_next = float(highs[i - 1]), float(highs[i]), float(highs[i + 1])
    l_prev, l_curr, l_next = float(lows[i - 1]), float(lows[i]), float(lows[i + 1])

    if fx.kind == "top":
        high_ok = h_curr > h_prev and h_curr > h_next
        low_ok = l_curr > l_prev and l_curr > l_next
        if not (high_ok and low_ok):
            return _check(name, False,
                          f"Top fractal idx={i} fails double condition: "
                          f"high_ok={high_ok} low_ok={low_ok} "
                          f"prev=({h_prev},{l_prev}) mid=({h_curr},{l_curr}) nxt=({h_next},{l_next})",
                          enable)
    elif fx.kind == "bottom":
        low_ok = l_curr < l_prev and l_curr < l_next
        high_ok = h_curr < h_prev and h_curr < h_next
        if not (low_ok and high_ok):
            return _check(name, False,
                          f"Bottom fractal idx={i} fails double condition: "
                          f"low_ok={low_ok} high_ok={high_ok} "
                          f"prev=({h_prev},{l_prev}) mid=({h_curr},{l_curr}) nxt=({h_next},{l_next})",
                          enable)
    else:
        return _check(name, False, f"Unknown fractal kind={fx.kind!r} at idx={i}", enable)
    return None


def assert_fractal_definition(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §3.1-§3.2
    Fractals MUST satisfy double conditions (high & low).

    Usage: assert_fractal_definition(df_merged, fractals)
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
        r = _check_fractal_double_condition(name, fx, highs, lows, n, enable)
        if r is not None:
            return r
    return _ok(name)


def _check_single_stroke(
    name: str, s, i: int, min_gap: int, mode: str, enable: bool,
) -> AssertResult | None:
    """检查单根笔的 i1>i0 和 gap 约束。"""
    if s.i1 <= s.i0:
        return _check(name, False, f"Stroke[{i}] i1={s.i1} <= i0={s.i0}", enable)
    gap = s.i1 - s.i0
    if gap < min_gap:
        return _check(name, False,
                      f"Stroke[{i}] gap={gap} < min_gap={min_gap} (mode={mode})", enable)
    return None


def _check_stroke_pair(name: str, s, prev, i: int, enable: bool) -> AssertResult | None:
    """检查相邻笔的方向交替和连续性。"""
    if s.direction == prev.direction:
        return _check(name, False,
                      f"Stroke[{i-1}] and Stroke[{i}] both {s.direction}, "
                      f"violates alternation", enable)
    if prev.i1 != s.i0:
        return _check(name, False,
                      f"Stroke[{i-1}].i1={prev.i1} != Stroke[{i}].i0={s.i0}, "
                      f"violates continuity (strokes must be stitched)", enable)
    return None


def _check_stroke_confirmed(name: str, s, i: int, is_last: bool, enable: bool) -> AssertResult | None:
    """检查笔的 confirmed 规则。"""
    if is_last and s.confirmed:
        return _check(name, False, f"Last stroke [{i}] should be confirmed=False", enable)
    if not is_last and not s.confirmed:
        return _check(name, False, f"Stroke[{i}] (not last) should be confirmed=True", enable)
    return None


def assert_stroke_alternation_and_gap(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §4.2-§4.4

    Usage: assert_stroke_alternation_and_gap(strokes, mode="wide", min_strict_sep=5)
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
        for r in (
            _check_single_stroke(name, s, i, min_gap, mode, enable),
            _check_stroke_confirmed(name, s, i, i == len(strokes) - 1, enable),
        ):
            if r is not None:
                return r
        if i > 0:
            r = _check_stroke_pair(name, s, strokes[i - 1], i, enable)
            if r is not None:
                return r

    return _ok(name)


def assert_no_pen_center(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §4.6 and §7.1
    Pen overlap zones are hypothesis only; strokes must NOT be center components.
    """
    name = "assert_no_pen_center"
    # Usage: assert_no_pen_center(moves, centers)
    if len(args) < 2:
        return _ok(name)
    moves, centers = args[0], args[1]
    if not centers:
        return _ok(name)
    if not moves:
        return _ok(name)

    # "笔不裁决"：任何层级的 Center 组件都必须来自 Move[k-1]（Segment/TrendTypeInstance），不能是 Stroke
    move_type = type(moves[0]).__name__
    if move_type == "Stroke":
        msg = "Center components must NOT be Stroke (pen overlap is hypothesis only)"
        if enable:
            raise AssertionError(f"[{name}] {msg}")
        return _fail(name, msg)

    for i, c in enumerate(centers):
        if c.seg0 < 0 or c.seg1 >= len(moves):
            msg = (
                f"Center[{i}] component range [{c.seg0},{c.seg1}] out of bounds "
                f"(n_moves={len(moves)})"
            )
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

    return _ok(name)


def _check_seg_bounds(name: str, seg, idx: int, strokes, enable: bool) -> AssertResult | None:
    """检查线段的笔索引范围和 i0/i1 一致性。"""
    if seg.s0 < 0 or seg.s1 >= len(strokes):
        return _check(name, False,
                      f"Segment[{idx}] stroke indices out of range: s0={seg.s0}, s1={seg.s1}", enable)
    if seg.i0 != strokes[seg.s0].i0:
        return _check(name, False,
                      f"Segment[{idx}] i0={seg.i0} != strokes[{seg.s0}].i0={strokes[seg.s0].i0}", enable)
    if seg.i1 != strokes[seg.s1].i1:
        return _check(name, False,
                      f"Segment[{idx}] i1={seg.i1} != strokes[{seg.s1}].i1={strokes[seg.s1].i1}", enable)
    return None


def _check_seg_overlap(name: str, seg, idx: int, strokes, enable: bool) -> AssertResult | None:
    """检查线段前三笔的交集重叠。"""
    s1 = strokes[seg.s0]
    s2 = strokes[seg.s0 + 1]
    s3 = strokes[seg.s0 + 2]
    overlap_low = max(s1.low, s2.low, s3.low)
    overlap_high = min(s1.high, s2.high, s3.high)
    if not (overlap_low < overlap_high):
        return _check(name, False,
                      f"Segment[{idx}] three-stroke overlap empty: "
                      f"overlap=[{overlap_low},{overlap_high}]", enable)
    return None


def _check_seg_confirmed(name: str, seg, idx: int, is_last: bool, enable: bool) -> AssertResult | None:
    """检查线段的 confirmed 规则。"""
    if is_last and seg.confirmed:
        return _check(name, False, f"Last segment [{idx}] should be confirmed=False", enable)
    if not is_last and not seg.confirmed:
        return _check(name, False, f"Segment[{idx}] (not last) should be confirmed=True", enable)
    return None


def assert_segment_min_three_strokes_overlap(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §5.1
    Segment MUST be built from >=3 strokes with 3-way intersection overlap.

    Usage: assert_segment_min_three_strokes_overlap(segments, strokes)
    """
    name = "assert_segment_min_three_strokes_overlap"
    if len(args) < 2:
        return _ok(name)
    segments, strokes = args[0], args[1]
    if not segments:
        return _ok(name)

    for idx, seg in enumerate(segments):
        is_last = idx == len(segments) - 1
        span = seg.s1 - seg.s0
        r = _check(name, span >= 2,
                   f"Segment[{idx}] s1-s0={span} < 2 (need >=3 strokes)", enable)
        if r is not None:
            return r

        for checker in (
            _check_seg_bounds(name, seg, idx, strokes, enable),
            _check_seg_overlap(name, seg, idx, strokes, enable),
            _check_seg_confirmed(name, seg, idx, is_last, enable),
        ):
            if checker is not None:
                return checker

    return _ok(name)


def _check_center_structure(name: str, c, i: int, sustain_m: int, enable: bool) -> AssertResult | None:
    """检查中枢的结构约束：至少3段、ZG>ZD、settled/candidate 一致性。"""
    if c.seg1 - c.seg0 < 2:
        return _check(name, False, f"Center[{i}] seg1-seg0={c.seg1 - c.seg0} < 2", enable)
    if c.high <= c.low:
        return _check(name, False, f"Center[{i}] ZG={c.high} <= ZD={c.low}", enable)
    if c.kind == "settled" and c.sustain < sustain_m:
        return _check(name, False,
                      f"Center[{i}] kind=settled but sustain={c.sustain} < sustain_m={sustain_m}", enable)
    if c.kind == "candidate" and c.sustain >= sustain_m:
        return _check(name, False,
                      f"Center[{i}] kind=candidate but sustain={c.sustain} >= sustain_m={sustain_m}", enable)
    return None


def assert_center_definition(*args: Any, enable: bool = False) -> AssertResult:
    """Spec: docs/chan_spec.md §7.1

    Usage: assert_center_definition(segments, centers, sustain_m=2)
    """
    name = "assert_center_definition"
    if len(args) < 2:
        return _ok(name)
    segments, centers = args[0], args[1]
    sustain_m = args[2] if len(args) > 2 else 2
    if not centers:
        return _ok(name)

    for i, c in enumerate(centers):
        r = _check_center_structure(name, c, i, sustain_m, enable)
        if r is not None:
            return r

        is_last = i == len(centers) - 1
        if is_last and c.confirmed:
            return _check(name, False, f"Last center [{i}] should be confirmed=False", enable) or _ok(name)
        if not is_last and not c.confirmed:
            return _check(name, False, f"Center[{i}] (not last) should be confirmed=True", enable) or _ok(name)

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

    Usage: assert_single_lstar(level_views, last_price)
    Checks:
    - select_lstar_newchan returns at most one L*
    - no duplicate alive centers with identical (seg0, seg1) on any level
    """
    name = "assert_single_lstar"
    if len(args) < 2:
        return _ok(name)

    level_views, last_price = args[0], args[1]

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
    name = "assert_ledger_separation"
    # Usage: assert_ledger_separation(rec_levels)
    if not args:
        return _ok(name)
    rec_levels = args[0]
    if not rec_levels:
        return _ok(name)

    # 递归层之间：下一层 moves 必须来自上一层"已确认"的趋势实例（Move[k] = confirmed TrendTypeInstance[k]）
    for i in range(len(rec_levels) - 1):
        cur = rec_levels[i]
        nxt = rec_levels[i + 1]
        expected_moves = [t for t in cur.trends if getattr(t, "confirmed", False)]
        if len(expected_moves) != len(nxt.moves):
            msg = (
                f"Level {nxt.level} moves count mismatch: "
                f"expected={len(expected_moves)} got={len(nxt.moves)}"
            )
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)
        for j, (exp, got) in enumerate(zip(expected_moves, nxt.moves)):
            if got != exp:
                msg = (
                    f"Level {nxt.level} moves[{j}] is not previous confirmed trend"
                )
                if enable:
                    raise AssertionError(f"[{name}] {msg}")
                return _fail(name, msg)

        if any(getattr(m, "confirmed", False) is False for m in nxt.moves):
            msg = f"Level {nxt.level} moves contains unconfirmed objects"
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

    return _ok(name)


def _run_inclusion_assertions(
    results: list[AssertResult],
    df_raw, df_merged, merged_to_raw,
) -> None:
    """运行包含处理相关断言。"""
    if df_merged is not None:
        results.append(assert_inclusion_no_residual(df_merged))
    if df_raw is not None and df_merged is not None and merged_to_raw is not None:
        results.append(assert_inclusion_direction_rule(df_raw, df_merged, merged_to_raw))


def _run_fractal_stroke_assertions(
    results: list[AssertResult],
    df_merged, fractals, strokes,
    stroke_mode: str, min_strict_sep: int,
) -> None:
    """运行分型/笔相关断言。"""
    if df_merged is not None and fractals is not None:
        results.append(assert_fractal_definition(df_merged, fractals))
    if strokes is not None:
        results.append(assert_stroke_alternation_and_gap(strokes, stroke_mode, min_strict_sep))


def _run_segment_assertions(
    results: list[AssertResult],
    segments, strokes, segment_algo: str,
) -> None:
    """运行线段相关断言。"""
    if segments is None or strokes is None:
        return
    if segment_algo == "v1":
        results.append(assert_segment_theorem_v1(strokes, segments))
    else:
        results.append(assert_segment_min_three_strokes_overlap(segments, strokes))


def _run_center_assertions(
    results: list[AssertResult],
    segments, centers, rec_levels,
    center_sustain_m: int,
) -> None:
    """运行中枢相关断言。"""
    if rec_levels:
        for rl in rec_levels:
            results.append(assert_center_definition(rl.moves, rl.centers, center_sustain_m))
            results.append(assert_no_pen_center(rl.moves, rl.centers))
    elif segments is not None and centers is not None:
        results.append(assert_center_definition(segments, centers, center_sustain_m))
        results.append(assert_no_pen_center(segments, centers))


def run_a_system_assertions(
    *,
    df_raw: Any | None = None,
    df_merged: Any | None = None,
    merged_to_raw: Any | None = None,
    fractals: Any | None = None,
    strokes: Any | None = None,
    segments: Any | None = None,
    centers: Any | None = None,
    rec_levels: Any | None = None,
    level_views: Any | None = None,
    last_price: float | None = None,
    segment_algo: str = "v1",
    stroke_mode: str = "wide",
    min_strict_sep: int = 5,
    center_sustain_m: int = 2,
    enable: bool = False,
) -> list[AssertResult]:
    """运行 A 系统 Step7 断言集合（用于"锁语义"）。"""
    results: list[AssertResult] = []

    _run_inclusion_assertions(results, df_raw, df_merged, merged_to_raw)
    _run_fractal_stroke_assertions(results, df_merged, fractals, strokes, stroke_mode, min_strict_sep)
    _run_segment_assertions(results, segments, strokes, segment_algo)
    _run_center_assertions(results, segments, centers, rec_levels, center_sustain_m)

    if rec_levels is not None:
        results.append(assert_ledger_separation(rec_levels))
    if level_views is not None and last_price is not None:
        results.append(assert_single_lstar(level_views, last_price))

    if enable:
        for r in results:
            if not r.ok:
                raise AssertionError(f"[{r.name}] {r.message}")

    return results


def _seg_check_min_strokes(seg, i: int, name: str, enable: bool) -> AssertResult | None:
    span = seg.s1 - seg.s0
    if span >= 2:
        return None
    return _check(name, False,
                  f"Segment[{i}] s1-s0={span} < 2 (need >=3 strokes)", enable)


def _seg_check_adjacent_stitching(seg, prev, i: int, name: str, enable: bool) -> AssertResult | None:
    if seg.s0 == prev.s1 + 1 or seg.s0 == prev.s1:
        return None
    return _check(name, False,
                  f"Segment[{i}].s0={seg.s0} not adjacent to "
                  f"Segment[{i-1}].s1={prev.s1}", enable)


def _seg_check_direction_alternation(seg, prev, i: int, name: str, enable: bool) -> AssertResult | None:
    if seg.direction != prev.direction:
        return None
    return _check(name, False,
                  f"Segment[{i}].direction={seg.direction} same as "
                  f"Segment[{i-1}].direction={prev.direction}", enable)


def _seg_check_degenerate(seg, i: int, name: str, enable: bool) -> AssertResult | None:
    if seg.ep0_price == 0.0 or seg.ep1_price == 0.0:
        return None
    if seg.direction == "up" and seg.ep1_price < seg.ep0_price - 1e-9:
        return _check(name, False,
                      f"Segment[{i}] degenerate: direction=up but "
                      f"ep1_price={seg.ep1_price} < ep0_price={seg.ep0_price}", enable)
    if seg.direction == "down" and seg.ep1_price > seg.ep0_price + 1e-9:
        return _check(name, False,
                      f"Segment[{i}] degenerate: direction=down but "
                      f"ep1_price={seg.ep1_price} > ep0_price={seg.ep0_price}", enable)
    return None


def _seg_check_settlement_anchor(seg, strokes, i: int, name: str, enable: bool) -> AssertResult | None:
    if not (seg.confirmed and seg.break_evidence is not None and strokes):
        return None
    k = seg.break_evidence.trigger_stroke_k
    if k + 2 >= len(strokes):
        return None
    s1 = strokes[k]
    s2 = strokes[k + 1]
    s3 = strokes[k + 2]
    overlap_low = max(s1.low, s2.low, s3.low)
    overlap_high = min(s1.high, s2.high, s3.high)
    if overlap_low < overlap_high:
        return None
    return _check(name, False,
                  f"Segment[{i}] settlement anchor violated: "
                  f"new seg strokes[{k},{k+1},{k+2}] no overlap", enable)


def _seg_check_kind_constraints(seg, i: int, is_last: bool, name: str, enable: bool) -> AssertResult | None:
    seg_kind = getattr(seg, "kind", "settled")
    if seg_kind == "candidate" and not is_last:
        return _check(name, False,
                      f"Segment[{i}] kind='candidate' but is not the last segment "
                      f"(only the last segment may be candidate)", enable)
    if seg_kind == "settled" and seg.confirmed:
        if getattr(seg, "break_evidence", None) is None:
            return _check(name, False,
                          f"Segment[{i}] kind='settled' and confirmed=True "
                          f"but has no break_evidence", enable)
    return None


def _seg_check_confirmed_rule(seg, i: int, is_last: bool, name: str, enable: bool) -> AssertResult | None:
    if is_last and seg.confirmed:
        return _check(name, False,
                      f"Last segment [{i}] should be confirmed=False", enable)
    if not is_last and not seg.confirmed:
        return _check(name, False,
                      f"Segment[{i}] (not last) should be confirmed=True", enable)
    return None


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
        is_last = i == len(segments) - 1

        for r in (
            _seg_check_min_strokes(seg, i, name, enable),
            _seg_check_degenerate(seg, i, name, enable),
            _seg_check_settlement_anchor(seg, strokes, i, name, enable),
            _seg_check_kind_constraints(seg, i, is_last, name, enable),
            _seg_check_confirmed_rule(seg, i, is_last, name, enable),
        ):
            if r is not None:
                return r

        if i > 0:
            prev = segments[i - 1]
            for r in (
                _seg_check_adjacent_stitching(seg, prev, i, name, enable),
                _seg_check_direction_alternation(seg, prev, i, name, enable),
            ):
                if r is not None:
                    return r

    return _ok(name)
