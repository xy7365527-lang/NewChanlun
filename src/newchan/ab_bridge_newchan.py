"""A→B 桥接：构建新缠论 overlay 输出

将 A 系统全管线（包含→分型→笔→段→中枢→走势类型→L*）
的输出组装为前端可消费的 JSON schema。

输出 schema_version: "newchan_overlay_v1"
"""

from __future__ import annotations

import math

import pandas as pd

from newchan.a_inclusion import merge_inclusion
from newchan.a_fractal import fractals_from_merged
from newchan.a_stroke import strokes_from_fractals
from newchan.a_segment_v0 import Segment, segments_from_strokes_v0
from newchan.a_segment_v1 import segments_from_strokes_v1
from newchan.a_center_v0 import centers_from_segments_v0
from newchan.a_trendtype_v0 import trend_instances_from_centers
from newchan.a_recursive_engine import build_recursive_levels, levels_to_level_views
from newchan.a_macd import compute_macd, macd_area_for_range
from newchan.a_level_fsm_newchan import (
    LevelView,
    classify_center_practical_newchan,
    select_lstar_newchan,
)


# ====================================================================
# 时间工具
# ====================================================================

def _ts_to_epoch(ts) -> int:
    """将 pandas Timestamp / datetime 转为 UTC epoch 秒。"""
    if hasattr(ts, "timestamp"):
        return int(ts.timestamp())
    return int(ts)


def _merged_idx_to_raw_range(
    merged_idx: int,
    merged_to_raw: list[tuple[int, int]],
) -> tuple[int, int]:
    """将 merged idx 映射到 raw idx 范围。"""
    if 0 <= merged_idx < len(merged_to_raw):
        return merged_to_raw[merged_idx]
    return (0, 0)


def _obj_raw_range(
    i0: int, i1: int,
    merged_to_raw: list[tuple[int, int]],
) -> tuple[int, int]:
    """对象（stroke/segment）端点 merged i0,i1 → raw 位置。

    统一取每个 merged bar 的 raw_end（= 该 merged bar 的逻辑时间戳，
    因为包含处理规定 ts = 最后一根 raw bar 的时间）。
    这确保共享分型点的两笔映射到相同的 raw 时间。
    """
    raw_i0 = _merged_idx_to_raw_range(i0, merged_to_raw)[1]  # END = logical time
    raw_i1 = _merged_idx_to_raw_range(i1, merged_to_raw)[1]  # END = logical time
    return raw_i0, raw_i1


def _merged_idx_to_epoch(
    merged_idx: int,
    merged_to_raw: list[tuple[int, int]],
    raw_index: pd.Index,
) -> int:
    """单个 merged idx 映射到 epoch 秒（取 raw_end 逻辑时间）。"""
    raw_i = _merged_idx_to_raw_range(merged_idx, merged_to_raw)[1]
    n = len(raw_index)
    i = max(0, min(raw_i, n - 1))
    return _ts_to_epoch(raw_index[i])


def _epoch_pair(
    raw_i0: int, raw_i1: int,
    raw_index: pd.Index,
) -> tuple[int, int]:
    """raw 位置索引 → (t0_epoch, t1_epoch)。"""
    n = len(raw_index)
    i0 = max(0, min(raw_i0, n - 1))
    i1 = max(0, min(raw_i1, n - 1))
    return _ts_to_epoch(raw_index[i0]), _ts_to_epoch(raw_index[i1])


# ====================================================================
# 主函数
# ====================================================================

def build_overlay_newchan(
    df_raw: pd.DataFrame,
    *,
    symbol: str = "",
    tf: str = "",
    detail: str = "full",
    segment_algo: str = "v1",
    stroke_mode: str = "wide",
    min_strict_sep: int = 5,
    center_sustain_m: int = 2,
    macd_fast: int = 12,
    macd_slow: int = 26,
    macd_signal: int = 9,
) -> dict:
    """构建新缠论 overlay 完整输出。

    Parameters
    ----------
    df_raw : pd.DataFrame
        原始 OHLC 数据（DateTimeIndex）。
    detail : ``"min"`` | ``"full"``
        min → anchors=null；full → 输出完整 AnchorSet。
    segment_algo : ``"v0"`` | ``"v1"``
    其余参数见各模块文档。

    Returns
    -------
    dict
        schema_version="newchan_overlay_v1" 的完整 overlay JSON。
    """
    if len(df_raw) < 3:
        return _empty_overlay(symbol, tf, detail, macd_fast, macd_slow, macd_signal)

    raw_index = df_raw.index

    # ── A 管线 ──
    df_merged, merged_to_raw = merge_inclusion(df_raw)
    fractals = fractals_from_merged(df_merged)
    strokes = strokes_from_fractals(
        df_merged, fractals, mode=stroke_mode, min_strict_sep=min_strict_sep,
    )

    if segment_algo == "v1":
        segments = segments_from_strokes_v1(strokes)
    else:
        segments = segments_from_strokes_v0(strokes)

    # ── 自下而上递归构造全部层级 ──
    rec_levels = build_recursive_levels(
        segments, sustain_m=center_sustain_m,
    )

    # Level-1 数据（向后兼容：顶层 centers/trends 取第一层）
    if rec_levels:
        centers = rec_levels[0].centers
        trends = rec_levels[0].trends
    else:
        centers = centers_from_segments_v0(segments, sustain_m=center_sustain_m)
        trends = trend_instances_from_centers(segments, centers)

    # ── MACD ──
    df_macd = compute_macd(df_raw, fast=macd_fast, slow=macd_slow, signal=macd_signal)

    # ── 自上而下裁决 L*（扫描全部层级，取最高存活层） ──
    last_price = float(df_raw["close"].iloc[-1])
    level_views = levels_to_level_views(rec_levels)
    if not level_views:
        level_views = [LevelView(level=1, segments=list(segments), centers=list(centers))]
    lstar_obj = select_lstar_newchan(level_views, last_price)

    # ── 规格断言（可选，用于“锁语义”） ─────────────────────────────
    # 通过 .env / 环境变量控制：NEWCHAN_ASSERT=1/true/yes/on
    from newchan import config
    if config.env_flag("NEWCHAN_ASSERT", False):
        from newchan.a_assertions import run_a_system_assertions
        run_a_system_assertions(
            df_raw=df_raw,
            df_merged=df_merged,
            merged_to_raw=merged_to_raw,
            fractals=fractals,
            strokes=strokes,
            segments=segments,
            centers=centers,
            rec_levels=rec_levels,
            level_views=level_views,
            last_price=last_price,
            segment_algo=segment_algo,
            stroke_mode=stroke_mode,
            min_strict_sep=min_strict_sep,
            center_sustain_m=center_sustain_m,
            enable=True,
        )

    # ── 构建输出 ──
    out_strokes = _build_strokes(strokes, merged_to_raw, raw_index, df_macd, df_merged)
    out_segments = _build_segments(segments, strokes, merged_to_raw, raw_index, df_macd, df_merged)
    out_centers = _build_centers(centers, segments, merged_to_raw, raw_index, df_macd)
    out_trends = _build_trends(trends, segments, strokes, merged_to_raw, raw_index, df_macd, df_merged)
    out_lstar = _build_lstar(lstar_obj, centers, segments, last_price, detail)
    out_macd = _build_macd_series(df_macd, raw_index, macd_fast, macd_slow, macd_signal)

    # ── 多级别数据 ──
    out_levels = _build_levels(rec_levels, segments, strokes, merged_to_raw, raw_index, df_macd, df_merged)

    return {
        "schema_version": "newchan_overlay_v2",
        "symbol": symbol,
        "tf": tf,
        "detail": detail,
        "lstar": out_lstar,
        "strokes": out_strokes,
        "segments": out_segments,
        "centers": out_centers,
        "trends": out_trends,
        "levels": out_levels,
        "macd": out_macd,
    }


# ====================================================================
# 内部构建函数
# ====================================================================

def _empty_overlay(symbol, tf, detail, mf, ms, msig):
    return {
        "schema_version": "newchan_overlay_v2",
        "symbol": symbol, "tf": tf, "detail": detail,
        "lstar": None,
        "strokes": [], "segments": [], "centers": [], "trends": [],
        "levels": [],
        "macd": {"fast": mf, "slow": ms, "signal": msig, "series": []},
    }


def _stroke_p0p1(s, merged_highs=None, merged_lows=None):
    """端点价：直接取 Stroke 上的分型价 p0/p1。

    Stroke 在构造时已记录分型极值，无需再从 merged bar 反查。
    保留 merged_highs/merged_lows 参数签名以兼容旧调用。
    """
    return float(s.p0), float(s.p1)


def _build_strokes(strokes, m2r, raw_index, df_macd, df_merged):
    merged_highs = df_merged["high"].values
    merged_lows = df_merged["low"].values
    result = []
    for i, s in enumerate(strokes):
        raw_i0, raw_i1 = _obj_raw_range(s.i0, s.i1, m2r)
        t0, t1 = _epoch_pair(raw_i0, raw_i1, raw_index)
        area = macd_area_for_range(df_macd, raw_i0, raw_i1)
        p0, p1 = _stroke_p0p1(s, merged_highs, merged_lows)
        result.append({
            "id": i, "t0": t0, "t1": t1,
            "dir": s.direction, "confirmed": s.confirmed,
            "high": s.high, "low": s.low,
            "p0": p0, "p1": p1,
            **{f"macd_{k}": v for k, v in area.items()},
        })
    return result


def _build_segments(segments, strokes, m2r, raw_index, df_macd, df_merged):
    """Map Segment 到前端 JSON：桥接层只做映射，不重算端点。"""
    merged_highs = df_merged["high"].values
    merged_lows = df_merged["low"].values
    result = []
    start_miss = 0
    end_miss = 0
    endpoint_time_reversed_count = 0
    endpoint_override_count = 0
    endpoint_override_samples = []
    miss_samples = []
    for i, seg in enumerate(segments):
        raw_i0, raw_i1 = _obj_raw_range(seg.i0, seg.i1, m2r)
        area = macd_area_for_range(df_macd, raw_i0, raw_i1)

        # 兼容历史字段：若缺失 ep*，退回旧字段。
        legacy_p0 = float(getattr(seg, "p0", seg.low))
        legacy_p1 = float(getattr(seg, "p1", seg.high))
        ep0_i = int(getattr(seg, "ep0_i", -1))
        ep1_i = int(getattr(seg, "ep1_i", -1))
        ep0_price = float(getattr(seg, "ep0_price", legacy_p0))
        ep1_price = float(getattr(seg, "ep1_price", legacy_p1))
        ep0_type = getattr(seg, "ep0_type", "bottom" if seg.direction == "up" else "top")
        ep1_type = getattr(seg, "ep1_type", "top" if seg.direction == "up" else "bottom")
        if ep0_i < 0:
            ep0_i = int(seg.i0)
        if ep1_i < 0:
            ep1_i = int(seg.i1)

        t0_render = int(_merged_idx_to_epoch(ep0_i, m2r, raw_index))
        t1_render = int(_merged_idx_to_epoch(ep1_i, m2r, raw_index))
        p0_render = float(ep0_price)
        p1_render = float(ep1_price)
        if t1_render < t0_render:
            endpoint_time_reversed_count += 1

        # stroke_points 仅用于调试/教学层，不参与端点语义计算。
        stroke_pts = []
        seg_strokes_range = range(seg.s0, min(seg.s1 + 1, len(strokes)))
        for si in seg_strokes_range:
            sk = strokes[si]
            sk_p0, sk_p1 = _stroke_p0p1(sk, merged_highs, merged_lows)
            _ri0, _ri1 = _obj_raw_range(sk.i0, sk.i1, m2r)
            _t0, _t1 = _epoch_pair(_ri0, _ri1, raw_index)
            stroke_pts.append({"time": int(_t0), "value": float(sk_p0)})
            if si == seg.s1:
                stroke_pts.append({"time": int(_t1), "value": float(sk_p1)})

        start_match = any(
            int(pt["time"]) == int(t0_render) and abs(float(pt["value"]) - p0_render) < 1e-9
            for pt in stroke_pts
        )
        end_match = any(
            int(pt["time"]) == int(t1_render) and abs(float(pt["value"]) - p1_render) < 1e-9
            for pt in stroke_pts
        )
        if not start_match:
            start_miss += 1
        if not end_match:
            end_miss += 1
        if (not start_match or not end_match) and len(miss_samples) < 3:
            miss_samples.append({
                "id": i,
                "s0": int(seg.s0),
                "s1": int(seg.s1),
                "t0": int(t0_render),
                "p0": float(p0_render),
                "t1": int(t1_render),
                "p1": float(p1_render),
                "orig_p0": float(legacy_p0),
                "orig_p1": float(legacy_p1),
                "stroke_points_len": len(stroke_pts),
                "first_pt": stroke_pts[0] if stroke_pts else None,
                "last_pt": stroke_pts[-1] if stroke_pts else None,
                "start_match": start_match,
                "end_match": end_match,
            })

        endpoint_overridden = (
            abs(float(p0_render) - float(legacy_p0)) > 1e-9
            or abs(float(p1_render) - float(legacy_p1)) > 1e-9
        )
        if endpoint_overridden:
            endpoint_override_count += 1
            if len(endpoint_override_samples) < 5:
                endpoint_override_samples.append({
                    "id": i,
                    "dir": seg.direction,
                    "s0": int(seg.s0),
                    "s1": int(seg.s1),
                    "legacy_p0": float(legacy_p0),
                    "legacy_p1": float(legacy_p1),
                    "render_p0": float(p0_render),
                    "render_p1": float(p1_render),
                })

        result.append({
            "id": i,
            "t0": int(t0_render),
            "t1": int(t1_render),
            "s0": seg.s0,
            "s1": seg.s1,
            "dir": seg.direction,
            "confirmed": seg.confirmed,
            "kind": getattr(seg, "kind", "settled"),
            "high": float(max(seg.high, p0_render, p1_render)),
            "low": float(min(seg.low, p0_render, p1_render)),
            "p0": float(p0_render),  # backward-compatible alias of ep0.price
            "p1": float(p1_render),  # backward-compatible alias of ep1.price
            "ep0": {
                "merged_i": int(ep0_i),
                "time": int(t0_render),
                "price": float(p0_render),
                "type": ep0_type,
            },
            "ep1": {
                "merged_i": int(ep1_i),
                "time": int(t1_render),
                "price": float(p1_render),
                "type": ep1_type,
            },
            "stroke_points": stroke_pts,
            **{f"macd_{k}": v for k, v in area.items()},
        })

    return result


def _build_centers(centers, segments, m2r, raw_index, df_macd):
    result = []
    for i, c in enumerate(centers):
        # center 时间范围取 segments[seg0] .. segments[seg1]
        if c.seg0 < len(segments) and c.seg1 < len(segments):
            seg_start = segments[c.seg0]
            seg_end = segments[c.seg1]
            raw_i0, _ = _obj_raw_range(seg_start.i0, seg_start.i1, m2r)
            _, raw_i1 = _obj_raw_range(seg_end.i0, seg_end.i1, m2r)
        else:
            raw_i0, raw_i1 = 0, 0
        t0, t1 = _epoch_pair(raw_i0, raw_i1, raw_index)
        area = macd_area_for_range(df_macd, raw_i0, raw_i1)
        result.append({
            "id": i, "t0": t0, "t1": t1,
            "ZD": c.low, "ZG": c.high,
            "kind": c.kind, "confirmed": c.confirmed, "sustain": c.sustain,
            "direction": getattr(c, "direction", ""),
            "GG": getattr(c, "gg", 0.0),
            "DD": getattr(c, "dd", 0.0),
            "G": getattr(c, "g", 0.0),
            "D": getattr(c, "d", 0.0),
            "development": getattr(c, "development", ""),
            "level_id": getattr(c, "level_id", 0),
            "terminated": getattr(c, "terminated", False),
            "termination_side": getattr(c, "termination_side", ""),
            **{f"macd_{k}": v for k, v in area.items()},
        })
    return result


def _build_trends(trends, segments, strokes, m2r, raw_index, df_macd, df_merged):
    result = []
    for i, tr in enumerate(trends):
        if tr.seg0 < len(segments) and tr.seg1 < len(segments):
            seg_start = segments[tr.seg0]
            seg_end = segments[tr.seg1]
            raw_i0, _ = _obj_raw_range(seg_start.i0, seg_start.i1, m2r)
            _, raw_i1 = _obj_raw_range(seg_end.i0, seg_end.i1, m2r)
        else:
            raw_i0, raw_i1 = 0, 0
        t0, t1 = _epoch_pair(raw_i0, raw_i1, raw_index)
        area = macd_area_for_range(df_macd, raw_i0, raw_i1)
        # p0/p1：用段内笔的端点价
        p0, p1 = None, None
        if tr.seg0 < len(segments) and tr.seg1 < len(segments):
            first_s_idx = segments[tr.seg0].s0
            last_s_idx = segments[tr.seg1].s1
            if first_s_idx < len(strokes) and last_s_idx < len(strokes):
                p0 = _stroke_p0p1(strokes[first_s_idx])[0]
                p1 = _stroke_p0p1(strokes[last_s_idx])[1]
        result.append({
            "id": i, "t0": t0, "t1": t1,
            "kind": tr.kind, "dir": tr.direction,
            "confirmed": tr.confirmed,
            "high": tr.high, "low": tr.low,
            "p0": p0, "p1": p1,
            "level_id": getattr(tr, "level_id", 0),
            **{f"macd_{k}": v for k, v in area.items()},
        })
    return result


def _build_lstar(lstar_obj, centers, segments, last_price, detail):
    if lstar_obj is None:
        return None

    center = centers[lstar_obj.center_idx]
    ac = classify_center_practical_newchan(
        center=center, center_idx=lstar_obj.center_idx,
        segments=segments, last_price=last_price,
    )

    out = {
        "level": lstar_obj.level,
        "center_id": lstar_obj.center_idx,
        "regime": lstar_obj.regime.value,
        "is_alive": ac.is_alive,
        "death_reason": ac.anchors.death_reason,
    }

    if detail == "full":
        out["anchors"] = {
            "settle_core_low": ac.anchors.settle_core_low,
            "settle_core_high": ac.anchors.settle_core_high,
            "run_exit_idx": ac.anchors.run_exit_idx,
            "run_exit_side": ac.anchors.run_exit_side.value if ac.anchors.run_exit_side else None,
            "run_exit_extreme": ac.anchors.run_exit_extreme,
            "event_seen_pullback": ac.anchors.event_seen_pullback,
            "event_pullback_settled": ac.anchors.event_pullback_settled,
        }
    else:
        out["anchors"] = None

    return out


def _build_macd_series(df_macd, raw_index, fast, slow, signal):
    series = []
    for i in range(len(df_macd)):
        t = _ts_to_epoch(raw_index[i])
        row = df_macd.iloc[i]
        rec = {"time": t}
        for col in ("macd", "signal", "hist"):
            v = row[col]
            if pd.notna(v) and not (isinstance(v, float) and math.isnan(v)):
                rec[col] = round(float(v), 6)
            else:
                rec[col] = 0.0
        series.append(rec)
    return {
        "fast": fast, "slow": slow, "signal": signal,
        "series": series,
    }


def _build_levels(rec_levels, segments, strokes, m2r, raw_index, df_macd, df_merged):
    """构建多级别输出。每层包含该层的 centers 和 trends。"""
    result = []
    for rl in rec_levels:
        level_centers = []
        for i, c in enumerate(rl.centers):
            # 中枢时间范围：映射 Move 的 i0/i1 到 raw 时间
            if c.seg0 < len(rl.moves) and c.seg1 < len(rl.moves):
                move_start = rl.moves[c.seg0]
                move_end = rl.moves[c.seg1]
                raw_i0, _ = _obj_raw_range(move_start.i0, move_start.i1, m2r)
                _, raw_i1 = _obj_raw_range(move_end.i0, move_end.i1, m2r)
            else:
                raw_i0, raw_i1 = 0, 0
            t0, t1 = _epoch_pair(raw_i0, raw_i1, raw_index)
            level_centers.append({
                "id": i, "t0": t0, "t1": t1,
                "ZD": c.low, "ZG": c.high,
                "kind": c.kind, "confirmed": c.confirmed,
                "sustain": c.sustain,
                "direction": getattr(c, "direction", ""),
                "GG": getattr(c, "gg", 0.0),
                "DD": getattr(c, "dd", 0.0),
                "G": getattr(c, "g", 0.0),
                "D": getattr(c, "d", 0.0),
            })

        level_trends = []
        for i, tr in enumerate(rl.trends):
            # 走势类型实例时间范围
            raw_i0, _ = _obj_raw_range(tr.i0, tr.i1, m2r)
            _, raw_i1 = _obj_raw_range(tr.i0, tr.i1, m2r)
            t0, t1 = _epoch_pair(raw_i0, raw_i1, raw_index)
            level_trends.append({
                "id": i, "t0": t0, "t1": t1,
                "kind": tr.kind, "dir": tr.direction,
                "confirmed": tr.confirmed,
                "high": tr.high, "low": tr.low,
            })

        result.append({
            "level": rl.level,
            "n_moves": len(rl.moves),
            "centers": level_centers,
            "trends": level_trends,
        })
    return result
