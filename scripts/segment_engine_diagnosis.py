"""段引擎诊断脚本 — 248号谱系。

诊断 71 strokes → 1 segment 压缩是 bug 还是缠论定义的自然结果。
同时诊断 daily 36 strokes → 3 segments 案例。

用法:
    python scripts/segment_engine_diagnosis.py

产出:
    tmp/segment-engine-diagnosis.json — 结构化诊断结果
    stdout — 人类可读的逐步分析

概念溯源:
  - 247号: SPY 真实数据验证发现 71→1 压缩
  - 237号: D2 递归压缩根因
"""

from __future__ import annotations

import json
import logging
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Literal

import yfinance as yf

PROJECT_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(PROJECT_ROOT / "src"))

from newchan.a_segment_v1 import (
    _FeatureSeqState,
    _find_overlap_start,
    _is_fractal_and_gap,
    _make_segment,
    _segment_endpoint_types,
    _stroke_endpoint_by_type,
    _three_stroke_overlap,
    _top_above_bottom,
    _try_trigger_segment,
    segments_from_strokes_v1,
)
from newchan.a_stroke import Stroke
from newchan.bi_engine import BiEngine
from newchan.core.recursion.segment_engine import SegmentEngine
from newchan.types import Bar

logging.basicConfig(
    level=logging.DEBUG,
    format="%(asctime)s [%(name)s] %(levelname)s: %(message)s",
)
logger = logging.getLogger("segment_diagnosis")


# ── 数据获取 ──────────────────────────────────────


def _df_to_bars(df) -> list[Bar]:
    bars: list[Bar] = []
    for idx, row in df.iterrows():
        ts = idx.to_pydatetime()
        if ts.tzinfo is None:
            ts = ts.replace(tzinfo=timezone.utc)
        bars.append(
            Bar(
                ts=ts,
                open=float(row["Open"]),
                high=float(row["High"]),
                low=float(row["Low"]),
                close=float(row["Close"]),
                volume=float(row.get("Volume", 0)),
            )
        )
    return bars


def fetch_data() -> tuple[list[Bar], list[Bar]]:
    """拉取 SPY 60min 和 daily 数据。"""
    ticker = yf.Ticker("SPY")

    logger.info("拉取 SPY 60min 数据 (6 月)...")
    intraday_df = ticker.history(period="6mo", interval="60m")
    logger.info("60min bar 数: %d", len(intraday_df))

    logger.info("拉取 SPY 日线数据 (2 年)...")
    daily_df = ticker.history(period="2y", interval="1d")
    logger.info("日线 bar 数: %d", len(daily_df))

    return _df_to_bars(intraday_df), _df_to_bars(daily_df)


# ── 笔引擎运行 ──────────────────────────────────


def run_bi_engine(bars: list[Bar], stroke_mode: str = "wide") -> list[Stroke]:
    """运行笔引擎获取笔序列。"""
    bi_engine = BiEngine(stroke_mode=stroke_mode)
    snap = None
    for bar in bars:
        snap = bi_engine.process_bar(bar)
    if snap is None:
        return []
    return snap.strokes


# ── 笔序列打印 ──────────────────────────────────


def print_strokes(strokes: list[Stroke], label: str) -> list[dict]:
    """打印笔序列并返回结构化数据。"""
    print(f"\n{'='*70}")
    print(f" {label}: {len(strokes)} 笔")
    print(f"{'='*70}")
    stroke_data = []
    for i, s in enumerate(strokes):
        info = {
            "index": i,
            "direction": s.direction,
            "i0": s.i0,
            "i1": s.i1,
            "high": round(s.high, 4),
            "low": round(s.low, 4),
            "p0": round(s.p0, 4),
            "p1": round(s.p1, 4),
            "confirmed": s.confirmed,
        }
        stroke_data.append(info)
        arrow = "↑" if s.direction == "up" else "↓"
        print(
            f"  笔{i:3d} {arrow} [{s.i0:4d}→{s.i1:4d}] "
            f"p0={s.p0:10.4f} → p1={s.p1:10.4f}  "
            f"H={s.high:10.4f} L={s.low:10.4f}  "
            f"{'confirmed' if s.confirmed else 'UNCONFIRMED'}"
        )
    return stroke_data


# ── 段引擎逐步诊断 ──────────────────────────────


def diagnose_segment_construction(
    strokes: list[Stroke],
    label: str,
) -> dict:
    """逐步诊断段构造过程，记录每个决策点。"""
    n = len(strokes)
    print(f"\n{'='*70}")
    print(f" 段构造逐步诊断: {label} ({n} 笔)")
    print(f"{'='*70}")

    result = {
        "label": label,
        "stroke_count": n,
        "steps": [],
        "segments_produced": [],
        "diagnosis": "",
    }

    if n < 3:
        result["diagnosis"] = f"笔数 {n} < 3, 无法构造线段"
        print(f"  笔数 {n} < 3, 无法构造线段")
        return result

    # Step 1: 找初始重叠起点
    seg_start = _find_overlap_start(strokes, 0)
    if seg_start is None:
        result["diagnosis"] = "未找到满足三笔重叠的起点"
        print("  未找到满足三笔重叠的起点")
        return result

    print(f"\n  [Step 1] 初始重叠起点: 笔{seg_start}")
    print(f"    三笔: {seg_start}, {seg_start+1}, {seg_start+2}")
    s0, s1, s2 = strokes[seg_start], strokes[seg_start + 1], strokes[seg_start + 2]
    overlap_low = max(s0.low, s1.low, s2.low)
    overlap_high = min(s0.high, s1.high, s2.high)
    print(f"    重叠区间: [{overlap_low:.4f}, {overlap_high:.4f}]")
    result["steps"].append({
        "type": "overlap_start",
        "seg_start": seg_start,
        "overlap": [round(overlap_low, 4), round(overlap_high, 4)],
    })

    # Step 2: 确定初始段方向
    seg_dir: Literal["up", "down"] = strokes[seg_start].direction
    print(f"    初始段方向: {seg_dir} (第一笔方向)")
    result["steps"].append({
        "type": "initial_direction",
        "direction": seg_dir,
    })

    # Step 3: 逐步构建特征序列并检测分型
    print(f"\n  [Step 2] 增量构建特征序列（反向笔 = {'down' if seg_dir == 'up' else 'up'} 笔）")

    feat = _FeatureSeqState(seg_dir)
    cursor = seg_start
    seg_count = 0
    trigger_attempts = []

    while cursor < n:
        sk = strokes[cursor]
        opposite: Literal["up", "down"] = "down" if seg_dir == "up" else "up"

        if sk.direction != opposite:
            cursor += 1
            continue

        # 这是一个反向笔，加入特征序列
        prev_len = len(feat.std)
        feat.append(cursor, sk.high, sk.low)
        new_len = len(feat.std)

        action = "merged" if new_len == prev_len else "appended"
        print(
            f"\n    笔{cursor:3d} ({sk.direction}) "
            f"H={sk.high:.4f} L={sk.low:.4f} → {action} "
            f"(特征序列长度: {new_len})"
        )

        if new_len >= 3:
            # 打印当前特征序列末尾3个元素
            tail = feat.std[-3:]
            for j, elem in enumerate(tail):
                eidx = new_len - 3 + j
                print(f"      特征序列[{eidx}]: H={elem[0]:.4f} L={elem[1]:.4f} stroke_idx={int(elem[2])}")

        # 尝试触发断段
        attempt_info = {
            "cursor": cursor,
            "stroke_direction": sk.direction,
            "stroke_high": round(sk.high, 4),
            "stroke_low": round(sk.low, 4),
            "feat_action": action,
            "feat_len": new_len,
            "seg_dir": seg_dir,
            "seg_start": seg_start,
            "trigger_result": None,
            "rejection_reason": None,
        }

        trig = feat.scan_trigger(seg_dir, strokes)
        if trig is not None:
            k, fractal_abc, gap_type = trig
            end_stroke = k - 1

            print(f"      >>> 特征序列分型触发! k={k}, fractal_abc={fractal_abc}, gap={gap_type}")
            print(f"          候选段: 笔{seg_start}→笔{end_stroke}")

            attempt_info["trigger_result"] = {
                "k": k,
                "fractal_abc": list(fractal_abc),
                "gap_type": gap_type,
                "end_stroke": end_stroke,
            }

            # 检查各个拒绝条件
            # 条件1: min_seg_strokes
            min_seg_strokes = 3
            if end_stroke - seg_start < min_seg_strokes - 1:
                reason = f"min_seg_strokes 不满足: span={end_stroke - seg_start} < {min_seg_strokes - 1}"
                print(f"          REJECTED: {reason}")
                attempt_info["rejection_reason"] = reason
                feat.skip_trigger(k)
            else:
                # 条件2: L78 顶高于底
                start_type, end_type = _segment_endpoint_types(seg_dir)
                _, ep0_price = _stroke_endpoint_by_type(strokes[seg_start], start_type)
                _, ep1_price = _stroke_endpoint_by_type(strokes[end_stroke], end_type)
                l78_ok = _top_above_bottom(seg_dir, ep0_price, ep1_price)

                if not l78_ok:
                    reason = (
                        f"L78 顶高于底违反: dir={seg_dir}, "
                        f"ep0={ep0_price:.4f}({start_type}), "
                        f"ep1={ep1_price:.4f}({end_type})"
                    )
                    print(f"          REJECTED: {reason}")
                    attempt_info["rejection_reason"] = reason
                    feat.skip_trigger(k)
                else:
                    # 条件3: 结算锚（新段前三笔重叠）
                    if k + 2 >= n:
                        reason = f"结算锚不满足: k={k}, k+2={k+2} >= n={n} (剩余笔不足3)"
                        print(f"          REJECTED: {reason}")
                        attempt_info["rejection_reason"] = reason
                        feat.skip_trigger(k)
                    elif not _three_stroke_overlap(strokes[k], strokes[k + 1], strokes[k + 2]):
                        s_k = strokes[k]
                        s_k1 = strokes[k + 1]
                        s_k2 = strokes[k + 2]
                        overlap_l = max(s_k.low, s_k1.low, s_k2.low)
                        overlap_h = min(s_k.high, s_k1.high, s_k2.high)
                        reason = (
                            f"结算锚不满足: 笔{k},{k+1},{k+2} 三笔无重叠 "
                            f"(overlap [{overlap_l:.4f}, {overlap_h:.4f}])"
                        )
                        print(f"          REJECTED: {reason}")
                        attempt_info["rejection_reason"] = reason
                        feat.skip_trigger(k)
                    else:
                        # 断段成功!
                        print(f"          ACCEPTED: 断段! 段{seg_count}: 笔{seg_start}→笔{end_stroke} ({seg_dir})")
                        attempt_info["rejection_reason"] = None
                        seg_count += 1
                        result["segments_produced"].append({
                            "seg_index": seg_count - 1,
                            "s0": seg_start,
                            "s1": end_stroke,
                            "direction": seg_dir,
                            "break_k": k,
                            "gap_type": gap_type,
                        })

                        # 开始新段
                        seg_start = k
                        seg_dir = opposite
                        feat.reset(seg_dir)
                        cursor = k
                        trigger_attempts.append(attempt_info)
                        cursor += 1
                        continue

        trigger_attempts.append(attempt_info)
        cursor += 1

    # 最后一段（未确认）
    if seg_start < n:
        last_end = n - 1
        print(f"\n  [最终段] 未确认段: 笔{seg_start}→笔{last_end} ({seg_dir}), span={last_end - seg_start}")
        result["segments_produced"].append({
            "seg_index": seg_count,
            "s0": seg_start,
            "s1": last_end,
            "direction": seg_dir,
            "confirmed": False,
            "note": "last_unconfirmed",
        })

    # 对比引擎实际输出
    print(f"\n  [Step 3] 引擎实际输出对比")
    actual_segments = segments_from_strokes_v1(strokes)
    print(f"    引擎产出线段数: {len(actual_segments)}")
    for i, seg in enumerate(actual_segments):
        print(
            f"    段{i}: 笔{seg.s0}→笔{seg.s1} ({seg.direction}) "
            f"confirmed={seg.confirmed} kind={seg.kind} "
            f"ep0={seg.ep0_price:.4f}({seg.ep0_type}) → ep1={seg.ep1_price:.4f}({seg.ep1_type})"
        )
        if seg.break_evidence:
            be = seg.break_evidence
            print(f"         break_evidence: k={be.trigger_stroke_k}, fractal={be.fractal_abc}, gap={be.gap_type}")

    result["actual_segments"] = [
        {
            "s0": seg.s0, "s1": seg.s1,
            "direction": seg.direction,
            "confirmed": seg.confirmed,
            "kind": seg.kind,
            "ep0_price": round(seg.ep0_price, 4),
            "ep1_price": round(seg.ep1_price, 4),
            "ep0_type": seg.ep0_type,
            "ep1_type": seg.ep1_type,
            "break_evidence": {
                "trigger_stroke_k": seg.break_evidence.trigger_stroke_k,
                "fractal_abc": list(seg.break_evidence.fractal_abc),
                "gap_type": seg.break_evidence.gap_type,
            } if seg.break_evidence else None,
        }
        for seg in actual_segments
    ]

    # 汇总触发尝试统计
    total_triggers = sum(1 for a in trigger_attempts if a["trigger_result"] is not None)
    rejected = sum(1 for a in trigger_attempts if a["trigger_result"] is not None and a["rejection_reason"] is not None)
    accepted = total_triggers - rejected

    # 分类拒绝原因
    rejection_reasons = {}
    for a in trigger_attempts:
        if a["trigger_result"] is not None and a["rejection_reason"] is not None:
            reason_type = a["rejection_reason"].split(":")[0].strip()
            rejection_reasons[reason_type] = rejection_reasons.get(reason_type, 0) + 1

    print(f"\n  [统计]")
    print(f"    特征序列分型触发次数: {total_triggers}")
    print(f"    被拒绝: {rejected}")
    print(f"    被接受: {accepted}")
    print(f"    拒绝原因分布:")
    for reason, count in sorted(rejection_reasons.items()):
        print(f"      {reason}: {count}")

    result["trigger_statistics"] = {
        "total_triggers": total_triggers,
        "rejected": rejected,
        "accepted": accepted,
        "rejection_reasons": rejection_reasons,
    }
    result["trigger_attempts"] = [
        a for a in trigger_attempts if a["trigger_result"] is not None
    ]

    # 诊断结论
    if len(actual_segments) == 1 and not actual_segments[0].confirmed:
        if total_triggers == 0:
            result["diagnosis"] = (
                "特征序列从未产生分型 — 71笔的反向笔序列经包含处理后,"
                "标准特征序列不足3个元素或无分型。段引擎行为正确。"
            )
        elif rejected > 0 and accepted == 0:
            result["diagnosis"] = (
                f"特征序列产生了 {total_triggers} 次分型触发,"
                f"但全部被拒绝。拒绝原因: {rejection_reasons}。"
                "需要逐个检查每个拒绝是否合理。"
            )
        else:
            result["diagnosis"] = f"意外: 有 {accepted} 次接受但只产出1段"
    elif len(actual_segments) > 1:
        result["diagnosis"] = f"段引擎产出 {len(actual_segments)} 段, 不是71→1。数据可能已变。"
    else:
        result["diagnosis"] = "意外情况"

    print(f"\n  [诊断结论] {result['diagnosis']}")
    return result


# ── 特征序列详细分析 ──────────────────────────────


def analyze_feature_sequence_detail(
    strokes: list[Stroke],
    label: str,
) -> dict:
    """深度分析特征序列的构建过程——记录每个包含处理的决策。"""
    n = len(strokes)

    seg_start = _find_overlap_start(strokes, 0)
    if seg_start is None:
        return {"error": "no overlap start"}

    seg_dir: Literal["up", "down"] = strokes[seg_start].direction
    opposite: Literal["up", "down"] = "down" if seg_dir == "up" else "up"

    print(f"\n{'='*70}")
    print(f" 特征序列详细分析: {label}")
    print(f" 段方向: {seg_dir}, 特征序列取 {opposite} 笔")
    print(f"{'='*70}")

    # 收集所有反向笔
    reverse_strokes = []
    for i in range(seg_start, n):
        if strokes[i].direction == opposite:
            reverse_strokes.append((i, strokes[i]))

    print(f"  反向笔数量: {len(reverse_strokes)}")
    print()

    # 手动构建特征序列
    elements: list[dict] = []  # {high, low, stroke_idx, merged_from}
    dir_state: str | None = "DOWN" if seg_dir == "down" else None

    for stroke_idx, sk in reverse_strokes:
        h, l = sk.high, sk.low
        if not elements:
            elements.append({"high": h, "low": l, "stroke_idx": stroke_idx, "merged_from": [stroke_idx]})
            print(f"  反向笔{stroke_idx}: H={h:.4f} L={l:.4f} → 初始元素 [0]")
            continue

        last = elements[-1]
        last_h, last_l = last["high"], last["low"]

        left_inc = last_h >= h and last_l <= l
        right_inc = h >= last_h and l <= last_l
        has_inclusion = left_inc or right_inc

        if has_inclusion:
            effective_up = dir_state != "DOWN"
            old_h, old_l = last["high"], last["low"]
            if effective_up:
                last["high"] = max(last_h, h)
                last["low"] = max(last_l, l)
            else:
                last["high"] = min(last_h, h)
                last["low"] = min(last_l, l)
            last["stroke_idx"] = stroke_idx
            last["merged_from"].append(stroke_idx)
            inc_type = "left" if left_inc else "right"
            merge_dir = "UP" if effective_up else "DOWN"
            print(
                f"  反向笔{stroke_idx}: H={h:.4f} L={l:.4f} → "
                f"包含合并({inc_type}, dir={merge_dir}) 到元素[{len(elements)-1}] "
                f"({old_h:.4f},{old_l:.4f})→({last['high']:.4f},{last['low']:.4f}) "
                f"merged_from={last['merged_from']}"
            )
        else:
            # 更新方向状态
            if h > last_h and l > last_l:
                dir_state = "UP"
            elif h < last_h and l < last_l:
                dir_state = "DOWN"
            elements.append({"high": h, "low": l, "stroke_idx": stroke_idx, "merged_from": [stroke_idx]})
            print(
                f"  反向笔{stroke_idx}: H={h:.4f} L={l:.4f} → "
                f"新元素 [{len(elements)-1}] dir_state={dir_state}"
            )

        # 检查是否有分型
        if len(elements) >= 3:
            a = elements[-3]
            b = elements[-2]
            c = elements[-1]

            # 检查顶分型(向上段)或底分型(向下段)
            if seg_dir == "up":
                is_top = (b["high"] > a["high"] and b["high"] > c["high"] and
                          b["low"] > a["low"] and b["low"] > c["low"])
                if is_top:
                    # 检查缺口
                    has_gap = b["low"] >= a["high"]
                    gap_str = "有缺口(第二种)" if has_gap else "无缺口(第一种)"
                    print(
                        f"      >>> 顶分型! 元素[{len(elements)-3},{len(elements)-2},{len(elements)-1}] "
                        f"a=({a['high']:.4f},{a['low']:.4f}) "
                        f"b=({b['high']:.4f},{b['low']:.4f}) "
                        f"c=({c['high']:.4f},{c['low']:.4f}) {gap_str}"
                    )
            else:
                is_bottom = (b["low"] < a["low"] and b["low"] < c["low"] and
                             b["high"] < a["high"] and b["high"] < c["high"])
                if is_bottom:
                    has_gap = a["low"] >= b["high"]
                    gap_str = "有缺口(第二种)" if has_gap else "无缺口(第一种)"
                    print(
                        f"      >>> 底分型! 元素[{len(elements)-3},{len(elements)-2},{len(elements)-1}] "
                        f"a=({a['high']:.4f},{a['low']:.4f}) "
                        f"b=({b['high']:.4f},{b['low']:.4f}) "
                        f"c=({c['high']:.4f},{c['low']:.4f}) {gap_str}"
                    )

    print(f"\n  最终标准特征序列长度: {len(elements)}")
    print(f"  标准特征序列元素:")
    for i, elem in enumerate(elements):
        print(
            f"    [{i}] H={elem['high']:.4f} L={elem['low']:.4f} "
            f"stroke_idx={elem['stroke_idx']} merged_from={elem['merged_from']}"
        )

    # 全面扫描所有分型
    print(f"\n  全面分型扫描:")
    fractals_found = []
    for i in range(1, len(elements) - 1):
        a = elements[i - 1]
        b = elements[i]
        c = elements[i + 1]

        is_top = (b["high"] > a["high"] and b["high"] > c["high"] and
                  b["low"] > a["low"] and b["low"] > c["low"])
        is_bottom = (b["low"] < a["low"] and b["low"] < c["low"] and
                     b["high"] < a["high"] and b["high"] < c["high"])

        if is_top or is_bottom:
            ftype = "顶" if is_top else "底"
            fractals_found.append({
                "index": i,
                "type": ftype,
                "a": {"high": a["high"], "low": a["low"]},
                "b": {"high": b["high"], "low": b["low"]},
                "c": {"high": c["high"], "low": c["low"]},
                "b_stroke_idx": b["stroke_idx"],
            })
            target = "顶" if seg_dir == "up" else "底"
            relevant = "→ 目标分型!" if ftype == target else "(非目标分型)"
            print(
                f"    元素[{i-1},{i},{i+1}] {ftype}分型 "
                f"b_stroke={b['stroke_idx']} {relevant}"
            )

    if not fractals_found:
        print("    无分型")

    return {
        "seg_dir": seg_dir,
        "reverse_stroke_count": len(reverse_strokes),
        "standard_feature_seq_length": len(elements),
        "elements": [
            {
                "index": i,
                "high": round(e["high"], 4),
                "low": round(e["low"], 4),
                "stroke_idx": e["stroke_idx"],
                "merged_from": e["merged_from"],
            }
            for i, e in enumerate(elements)
        ],
        "fractals_found": fractals_found,
    }


# ── 主流程 ──────────────────────────────────────


def main() -> None:
    results = {}

    # 拉取数据
    print("拉取 SPY 数据...")
    intraday_bars, daily_bars = fetch_data()
    print(f"60min bars: {len(intraday_bars)}, daily bars: {len(daily_bars)}")

    # ── 60min 案例 ──
    print("\n" + "#" * 70)
    print("# 60min 案例: 预期 ~71 strokes → 1 segment")
    print("#" * 70)

    strokes_60m = run_bi_engine(intraday_bars, stroke_mode="wide")
    stroke_data_60m = print_strokes(strokes_60m, "SPY 60min")

    # 先运行引擎看实际结果
    actual_segs_60m = segments_from_strokes_v1(strokes_60m)
    print(f"\n引擎输出: {len(strokes_60m)} strokes → {len(actual_segs_60m)} segments")

    # 逐步诊断
    diag_60m = diagnose_segment_construction(strokes_60m, "SPY 60min")

    # 特征序列详细分析
    feat_detail_60m = analyze_feature_sequence_detail(strokes_60m, "SPY 60min")

    results["60min"] = {
        "bar_count": len(intraday_bars),
        "stroke_count": len(strokes_60m),
        "segment_count": len(actual_segs_60m),
        "strokes": stroke_data_60m,
        "diagnosis": diag_60m,
        "feature_sequence_detail": feat_detail_60m,
    }

    # ── Daily 案例 ──
    print("\n" + "#" * 70)
    print("# Daily 案例: 预期 ~36 strokes → 3 segments")
    print("#" * 70)

    strokes_daily = run_bi_engine(daily_bars, stroke_mode="wide")
    stroke_data_daily = print_strokes(strokes_daily, "SPY Daily")

    actual_segs_daily = segments_from_strokes_v1(strokes_daily)
    print(f"\n引擎输出: {len(strokes_daily)} strokes → {len(actual_segs_daily)} segments")

    diag_daily = diagnose_segment_construction(strokes_daily, "SPY Daily")
    feat_detail_daily = analyze_feature_sequence_detail(strokes_daily, "SPY Daily")

    results["daily"] = {
        "bar_count": len(daily_bars),
        "stroke_count": len(strokes_daily),
        "segment_count": len(actual_segs_daily),
        "strokes": stroke_data_daily,
        "diagnosis": diag_daily,
        "feature_sequence_detail": feat_detail_daily,
    }

    # ── 价格趋势分析 ──
    print("\n" + "#" * 70)
    print("# 价格趋势 vs 段方向 一致性检查")
    print("#" * 70)

    if strokes_60m:
        first_price = strokes_60m[0].p0
        last_price = strokes_60m[-1].p1
        price_change = (last_price - first_price) / first_price * 100
        print(f"\n  60min: 首笔起点={first_price:.4f}, 末笔终点={last_price:.4f}, "
              f"变化={price_change:+.2f}%")
        if actual_segs_60m:
            seg = actual_segs_60m[0]
            print(f"  唯一段方向: {seg.direction}")
            if seg.direction == "down" and price_change > 5:
                print("  >>> 矛盾: 市场上涨 >5% 但段方向为 down")
                results["price_direction_conflict"] = True
            elif seg.direction == "up" and price_change < -5:
                print("  >>> 矛盾: 市场下跌 >5% 但段方向为 up")
                results["price_direction_conflict"] = True
            else:
                results["price_direction_conflict"] = False

    # ── TAIL_WINDOW 检查 ──
    print("\n" + "#" * 70)
    print("# TAIL_WINDOW 限制检查")
    print("#" * 70)
    if feat_detail_60m.get("standard_feature_seq_length", 0) > _FeatureSeqState.TAIL_WINDOW:
        print(f"\n  标准特征序列长度 ({feat_detail_60m['standard_feature_seq_length']}) "
              f"> TAIL_WINDOW ({_FeatureSeqState.TAIL_WINDOW})")
        print("  >>> 这意味着只有最近 TAIL_WINDOW 个元素会被扫描分型!")
        print("  >>> 如果分型出现在更早的位置，会被遗漏!")
        results["tail_window_issue"] = True

        # 检查是否有目标分型在 TAIL_WINDOW 之外
        target_type = "顶" if feat_detail_60m["seg_dir"] == "up" else "底"
        seq_len = feat_detail_60m["standard_feature_seq_length"]
        window_start = seq_len - _FeatureSeqState.TAIL_WINDOW
        early_fractals = [
            f for f in feat_detail_60m.get("fractals_found", [])
            if f["type"] == target_type and f["index"] < window_start
        ]
        if early_fractals:
            print(f"  >>> 发现 {len(early_fractals)} 个目标分型在 TAIL_WINDOW 之外!")
            for f in early_fractals:
                print(f"      元素[{f['index']}] {f['type']}分型 b_stroke={f['b_stroke_idx']}")
            results["missed_fractals_outside_window"] = [
                {"element_index": f["index"], "b_stroke_idx": f["b_stroke_idx"]}
                for f in early_fractals
            ]
        else:
            print(f"  在 TAIL_WINDOW 之外无目标分型 (目标: {target_type})")
    else:
        print(f"\n  标准特征序列长度 ({feat_detail_60m.get('standard_feature_seq_length', 0)}) "
              f"<= TAIL_WINDOW ({_FeatureSeqState.TAIL_WINDOW})")
        print("  TAIL_WINDOW 不构成限制")
        results["tail_window_issue"] = False

    # ── 写入结果 ──
    output_path = PROJECT_ROOT / "tmp" / "segment-engine-diagnosis.json"
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # 清理不可序列化的内容
    def _clean(obj):
        if isinstance(obj, dict):
            return {k: _clean(v) for k, v in obj.items()}
        if isinstance(obj, list):
            return [_clean(v) for v in obj]
        if isinstance(obj, float):
            if obj != obj:  # NaN
                return None
            return round(obj, 6)
        return obj

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(_clean(results), f, indent=2, ensure_ascii=False, default=str)

    print(f"\n结果已写入: {output_path}")


if __name__ == "__main__":
    main()
