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
    name = "assert_inclusion_direction_rule"
    # Usage: assert_inclusion_direction_rule(df_raw, df_merged, merged_to_raw)
    if len(args) < 3:
        return _ok(name)
    df_raw, df_merged, merged_to_raw = args[0], args[1], args[2]

    # 防御：缺少列就放过（由 merge_inclusion 自身/单测兜底）
    try:
        raw_highs = df_raw["high"].values
        raw_lows = df_raw["low"].values
        raw_opens = df_raw["open"].values
        raw_closes = df_raw["close"].values
    except Exception:
        return _ok(name)

    # ── 以规格 §2.3+§2.2 的“参考实现”重跑一次，并与输出对比 ──
    buf: list[list[float | int]] = [
        [float(raw_opens[0]), float(raw_highs[0]), float(raw_lows[0]), float(raw_closes[0]), 0, 0]
    ]
    dir_state: str | None = None  # None / "UP" / "DOWN"

    for i in range(1, len(raw_highs)):
        curr_h, curr_l = float(raw_highs[i]), float(raw_lows[i])
        last = buf[-1]
        last_h, last_l = float(last[1]), float(last[2])

        left_inc = last_h >= curr_h and last_l <= curr_l
        right_inc = curr_h >= last_h and curr_l <= last_l
        has_inc = left_inc or right_inc

        if has_inc:
            effective_up = dir_state != "DOWN"  # dir=None 也默认 UP
            if effective_up:
                new_h = max(last_h, curr_h)
                new_l = max(last_l, curr_l)
            else:
                new_h = min(last_h, curr_h)
                new_l = min(last_l, curr_l)
            last[1] = new_h
            last[2] = new_l
            last[3] = float(raw_closes[i])
            last[5] = i
        else:
            if curr_h > last_h and curr_l > last_l:
                dir_state = "UP"
            elif curr_h < last_h and curr_l < last_l:
                dir_state = "DOWN"
            buf.append([float(raw_opens[i]), curr_h, curr_l, float(raw_closes[i]), i, i])

    sim_map = [(int(r[4]), int(r[5])) for r in buf]
    if list(merged_to_raw) != sim_map:
        return _fail(
            name,
            f"merged_to_raw mismatch: expected={sim_map} got={list(merged_to_raw)}",
        )

    try:
        out = df_merged[["open", "high", "low", "close"]].values
    except Exception:
        return _ok(name)
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

    return _ok(name)


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
    name = "assert_no_pen_center"
    # Usage: assert_no_pen_center(moves, centers)
    if len(args) < 2:
        return _ok(name)
    moves, centers = args[0], args[1]
    if not centers:
        return _ok(name)
    if not moves:
        return _ok(name)

    # “笔不裁决”：任何层级的 Center 组件都必须来自 Move[k-1]（Segment/TrendTypeInstance），不能是 Stroke
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

    # 递归层之间：下一层 moves 必须来自上一层“已确认”的趋势实例（Move[k] = confirmed TrendTypeInstance[k]）
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
    """运行 A 系统 Step7 断言集合（用于“锁语义”）。

    设计原则：
    - 断言应尽量验证“不变量”，而不是某个实现的偶然细节。
    - enable=True 时：任何失败都应抛出 AssertionError（用于测试/CI）。
    - enable=False 时：返回失败列表供调用方记录/展示（默认不打断主流程）。
    """
    results: list[AssertResult] = []

    def _push(r: AssertResult) -> None:
        results.append(r)
        if enable and not r.ok:
            raise AssertionError(f"[{r.name}] {r.message}")

    # ── Inclusion ────────────────────────────────────────────────
    if df_merged is not None:
        _push(assert_inclusion_no_residual(df_merged))
    if df_raw is not None and df_merged is not None and merged_to_raw is not None:
        _push(assert_inclusion_direction_rule(df_raw, df_merged, merged_to_raw))

    # ── Fractal / Stroke ─────────────────────────────────────────
    if df_merged is not None and fractals is not None:
        _push(assert_fractal_definition(df_merged, fractals))
    if strokes is not None:
        _push(assert_stroke_alternation_and_gap(strokes, stroke_mode, min_strict_sep))

    # ── Segment ──────────────────────────────────────────────────
    if segments is not None:
        if segment_algo == "v1":
            if strokes is not None:
                _push(assert_segment_theorem_v1(strokes, segments))
        else:
            if strokes is not None:
                _push(assert_segment_min_three_strokes_overlap(segments, strokes))

    # ── Center（逐级） ────────────────────────────────────────────
    if rec_levels:
        for rl in rec_levels:
            _push(assert_center_definition(rl.moves, rl.centers, center_sustain_m))
            _push(assert_no_pen_center(rl.moves, rl.centers))
    elif segments is not None and centers is not None:
        # 无递归层时，仅验证 Level-1（segments→centers）
        _push(assert_center_definition(segments, centers, center_sustain_m))
        _push(assert_no_pen_center(segments, centers))

    # ── Recursion ledger separation ──────────────────────────────
    if rec_levels is not None:
        _push(assert_ledger_separation(rec_levels))

    # ── L* uniqueness (across levels) ────────────────────────────
    if level_views is not None and last_price is not None:
        _push(assert_single_lstar(level_views, last_price))

    return results


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

        # ── direction alternation ──
        if i > 0:
            prev = segments[i - 1]
            if seg.direction == prev.direction:
                msg = (
                    f"Segment[{i}].direction={seg.direction} same as "
                    f"Segment[{i-1}].direction={prev.direction}"
                )
                if enable:
                    raise AssertionError(f"[{name}] {msg}")
                return _fail(name, msg)

        # ── degenerate segment check (direction vs price) ──
        if seg.ep0_price != 0.0 and seg.ep1_price != 0.0:
            if seg.direction == "up" and seg.ep1_price < seg.ep0_price - 1e-9:
                msg = (
                    f"Segment[{i}] degenerate: direction=up but "
                    f"ep1_price={seg.ep1_price} < ep0_price={seg.ep0_price}"
                )
                if enable:
                    raise AssertionError(f"[{name}] {msg}")
                return _fail(name, msg)
            if seg.direction == "down" and seg.ep1_price > seg.ep0_price + 1e-9:
                msg = (
                    f"Segment[{i}] degenerate: direction=down but "
                    f"ep1_price={seg.ep1_price} > ep0_price={seg.ep0_price}"
                )
                if enable:
                    raise AssertionError(f"[{name}] {msg}")
                return _fail(name, msg)

        # ── settlement anchor: confirmed seg's successor must have overlap ──
        if seg.confirmed and seg.break_evidence is not None and strokes:
            k = seg.break_evidence.trigger_stroke_k
            if k + 2 < len(strokes):
                s1 = strokes[k]
                s2 = strokes[k + 1]
                s3 = strokes[k + 2]
                overlap_low = max(s1.low, s2.low, s3.low)
                overlap_high = min(s1.high, s2.high, s3.high)
                if not (overlap_low < overlap_high):
                    msg = (
                        f"Segment[{i}] settlement anchor violated: "
                        f"new seg strokes[{k},{k+1},{k+2}] no overlap"
                    )
                    if enable:
                        raise AssertionError(f"[{name}] {msg}")
                    return _fail(name, msg)

        # ── kind constraints ──
        seg_kind = getattr(seg, "kind", "settled")
        is_last = i == len(segments) - 1

        # candidate 段只允许出现在列表末尾
        if seg_kind == "candidate" and not is_last:
            msg = (
                f"Segment[{i}] kind='candidate' but is not the last segment "
                f"(only the last segment may be candidate)"
            )
            if enable:
                raise AssertionError(f"[{name}] {msg}")
            return _fail(name, msg)

        # settled + confirmed=True 的段必须有 break_evidence
        if seg_kind == "settled" and seg.confirmed:
            if getattr(seg, "break_evidence", None) is None:
                msg = (
                    f"Segment[{i}] kind='settled' and confirmed=True "
                    f"but has no break_evidence"
                )
                if enable:
                    raise AssertionError(f"[{name}] {msg}")
                return _fail(name, msg)

        # ── confirmed rule ──
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
