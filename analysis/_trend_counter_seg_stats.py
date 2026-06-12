"""趋势走势内反向线段的结构密度普查（一次性统计脚本）。

问题：趋势（≥2 同向中枢的 move）内部的反向线段能否承载短差操作——
反向线段有多少笔、有没有笔级中枢、振幅够不够覆盖摩擦。

管线（全 batch，O(N)）：
  bars → BiEngine("wide",5,False,3) 流式产笔（与 RecursiveOrchestrator L1 同参，
  见 _verify_on_engine.py）→ 终态 strokes 一次性跑
  segments_from_strokes_v1 → zhongshu_from_segments → moves_from_zhongshus。
  反向线段内部用 zhongshu_from_strokes 扫笔中枢（525号退化基底路径）。

坐标口径：
  - stroke/segment 的 i0/i1 是 K 线包含处理后的合并坐标，非 raw bar
    （记忆 project_current_strokes_i1_merged_coord）。bar 跨度同时报
    合并坐标值与 raw 估算值（scale = n_raw / (max_i1+1)）。
  - move 的 seg_end 截到 last_zs.seg_end，不含末中枢后的 C 段
    （记忆 project_bsp_gap_root_cause）→ 按 _bsp_gap_probe3.py 同款扩展：
    seg_end_ext = 下一 move 的 seg_start - 1，末 move = n_segs - 1。

确认滞后（任务4）是**规则估计**非流式实测：线段 j 的 break_evidence.
trigger_stroke_k = 特征序列分型 B 元素的笔下标（= s1+1）；C 元素笔下标未
导出，按无包含合并的最小情形近似 C ≈ trigger_k + 2（包含关系会延长，
gap_type=second 另有第二序列要求 → 本估计是滞后的下界）。

认识论等级：L2（OKLO 单标的）/ L3（OKLO+BRN+CL 交叉）真实数据统计。
运行：.venv/bin/python analysis/_trend_counter_seg_stats.py [SYMBOLS]
  环境变量 TS_MAX_BARS=50000 冒烟测试。
"""
from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parent / "src"))

import newchan_rust as R  # noqa: E402
from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402

_BI = ("wide", 5, False, 3)  # = RecursiveOrchestrator L1 内部 BiEngine 参数
OUT_JSON = _HERE / "data_cache" / "_trend_counter_seg_stats.json"


def _pct(sorted_vals: list[float], q: float) -> float:
    """线性插值分位数（sorted 输入）。"""
    if not sorted_vals:
        return float("nan")
    if len(sorted_vals) == 1:
        return sorted_vals[0]
    pos = q * (len(sorted_vals) - 1)
    lo = int(pos)
    hi = min(lo + 1, len(sorted_vals) - 1)
    frac = pos - lo
    return sorted_vals[lo] * (1 - frac) + sorted_vals[hi] * frac


def _dist(vals: list[float]) -> dict:
    s = sorted(vals)
    n = len(s)
    return {
        "n": n,
        "mean": (sum(s) / n) if n else float("nan"),
        "p25": _pct(s, 0.25),
        "p50": _pct(s, 0.50),
        "p75": _pct(s, 0.75),
    }


def analyze(symbol: str) -> dict:
    t0 = time.time()
    if os.environ.get("TS_MAX_BARS"):
        os.environ["BT_MAX_BARS"] = os.environ["TS_MAX_BARS"]
    opens, highs, lows, closes, _years = load_ohlc(SYMBOL_FILES[symbol])
    n_raw = len(closes)
    t_load = time.time() - t0

    # ── 流式产笔（O(N)）──
    t0 = time.time()
    bi = R.BiEngine(*_BI)
    for i in range(n_raw):
        bi.process_bar(opens[i], highs[i], lows[i], closes[i])
    strokes = bi.current_strokes()  # (i0,i1,dir,high,low,p0,p1,confirmed)
    t_bi = time.time() - t0

    # ── 终态 batch：seg → zs → move ──
    t0 = time.time()
    segs = R.segments_from_strokes_v1(strokes, 3, "strict")
    # SegmentTuple head = (s0,s1,i0,i1,dir,high,low,confirmed,kind)
    seg6 = [(s[0][0], s[0][1], s[0][5], s[0][6], s[0][7], s[0][8] == "settled")
            for s in segs]
    zss = R.zhongshu_from_segments(seg6)
    moves = R.moves_from_zhongshus([tuple(z) for z in zss], len(segs))
    t_batch = time.time() - t0

    n_strokes = len(strokes)
    n_segs = len(segs)
    max_i1 = strokes[-1][1] if strokes else 0
    scale = n_raw / (max_i1 + 1) if max_i1 else float("nan")  # raw/merged 估算系数

    # 全标的笔振幅 P25（θ 门对照基准；全局静态分位，非因果滚动——下界对照用）
    stroke_amps = [abs(s[6] - s[5]) / s[5] * 100 for s in strokes if s[7] and s[5] > 0]
    stroke_amp_p25 = _pct(sorted(stroke_amps), 0.25)

    # ── move 的 seg_end 扩展（同 _bsp_gap_probe3.py）──
    heads = [list(m[0]) for m in moves]
    for k, h in enumerate(heads):
        h[3] = (heads[k + 1][2] - 1) if k + 1 < len(heads) else (n_segs - 1)

    # ── 趋势 move 普查 ──
    trend = [h for h in heads if h[0] == "trend" and h[6] >= 2]
    trend_spans_merged: list[float] = []
    trend_zs_counts: list[float] = []

    def seg_stats_bucket():
        return {"strokes": [], "amp_pct": [], "zs_per_seg": [],
                "zs_amp_pct": [], "zs_over_p25": 0, "zs_total": 0,
                "segs_with_zs": 0, "lag_strokes": [], "lag_merged": []}

    rev = seg_stats_bucket()   # 反向线段
    fwd = seg_stats_bucket()   # 顺向线段
    rev_per_move: list[int] = []

    for h in trend:
        _kind, mdir, ss, se, _zs_s, _zs_e, zs_c, _settled = h
        ss = max(int(ss), 0)
        se = min(int(se), n_segs - 1)
        if se < ss:
            continue
        trend_zs_counts.append(zs_c)
        s0_first = segs[ss][0][0]
        s1_last = segs[se][0][1]
        trend_spans_merged.append(strokes[s1_last][1] - strokes[s0_first][0])

        n_rev_in_move = 0
        for j in range(ss, se + 1):
            head = segs[j][0]
            s0, s1, _i0, _i1, sdir, shigh, slow, _conf, _kind2 = head
            bucket = rev if sdir != mdir else fwd
            if sdir != mdir:
                n_rev_in_move += 1
            nstk = s1 - s0 + 1
            p_start = strokes[s0][5]
            bucket["strokes"].append(nstk)
            if p_start > 0:
                bucket["amp_pct"].append((shigh - slow) / p_start * 100)
            # 笔中枢扫描（线段内笔序列）
            zs_in = [(strokes[t][0], strokes[t][1], strokes[t][3], strokes[t][4], True)
                     for t in range(s0, s1 + 1)]
            bzs = R.zhongshu_from_strokes(zs_in)
            bucket["zs_per_seg"].append(len(bzs))
            if bzs:
                bucket["segs_with_zs"] += 1
            for z in bzs:
                zd, zg = z[0], z[1]
                if zd > 0:
                    amp = (zg - zd) / zd * 100
                    bucket["zs_amp_pct"].append(amp)
                    bucket["zs_total"] += 1
                    if amp > stroke_amp_p25:
                        bucket["zs_over_p25"] += 1
            # 确认滞后估计（规则下界：C ≈ trigger_k + 2）
            be = segs[j][2]
            if be is not None:
                trig_k = be[0]
                c_approx = min(trig_k + 2, n_strokes - 1)
                bucket["lag_strokes"].append(c_approx - s1)
                bucket["lag_merged"].append(strokes[c_approx][1] - strokes[s1][1])
        rev_per_move.append(n_rev_in_move)

    def finalize(b: dict) -> dict:
        out = {
            "n_segs": len(b["strokes"]),
            "strokes": _dist([float(x) for x in b["strokes"]]),
            "amp_pct": _dist(b["amp_pct"]),
            "zs_per_seg_mean": (sum(b["zs_per_seg"]) / len(b["zs_per_seg"]))
            if b["zs_per_seg"] else float("nan"),
            "segs_with_zs_share": (b["segs_with_zs"] / len(b["strokes"]))
            if b["strokes"] else float("nan"),
            "zs_amp_pct": _dist(b["zs_amp_pct"]),
            "zs_over_stroke_p25_share": (b["zs_over_p25"] / b["zs_total"])
            if b["zs_total"] else float("nan"),
            "lag_strokes": _dist([float(x) for x in b["lag_strokes"]]),
            "lag_merged_bars": _dist([float(x) for x in b["lag_merged"]]),
        }
        return out

    result = {
        "symbol": symbol,
        "n_raw_bars": n_raw,
        "n_strokes": n_strokes,
        "n_segments": n_segs,
        "n_moves": len(heads),
        "merged_to_raw_scale": scale,
        "stroke_amp_p25_pct": stroke_amp_p25,
        "trend_moves": {
            "count": len(trend),
            "settled_count": sum(1 for h in trend if h[7]),
            "avg_span_merged_bars": (sum(trend_spans_merged) / len(trend_spans_merged))
            if trend_spans_merged else float("nan"),
            "avg_span_raw_bars_est": (sum(trend_spans_merged) / len(trend_spans_merged) * scale)
            if trend_spans_merged else float("nan"),
            "avg_zs_count": (sum(trend_zs_counts) / len(trend_zs_counts))
            if trend_zs_counts else float("nan"),
            "rev_segs_per_move": _dist([float(x) for x in rev_per_move]),
        },
        "reverse_segments": finalize(rev),
        "forward_segments": finalize(fwd),
        "timing_s": {"load": round(t_load, 1), "bi_stream": round(t_bi, 1),
                     "batch": round(t_batch, 1)},
    }
    return result


def main() -> None:
    symbols = sys.argv[1].split(",") if len(sys.argv) > 1 else ["OKLO"]
    all_res = {}
    if OUT_JSON.exists():
        all_res = json.loads(OUT_JSON.read_text())
    for sym in symbols:
        print(f"== {sym} ==", flush=True)
        res = analyze(sym)
        all_res[sym] = res
        OUT_JSON.write_text(json.dumps(all_res, indent=1))
        print(json.dumps(res, indent=1), flush=True)


if __name__ == "__main__":
    main()
