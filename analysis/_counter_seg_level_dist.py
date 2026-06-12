"""趋势内反向运动的级别分布普查（追加任务，一次性统计脚本）。

两种反向运动存在形式（编排者洞察）：
  1. 同级别反向线段（_trend_counter_seg_stats.py 已统计）
  2. 次级别反向运动：顺向线段内部的反向笔（L1 层）；
     顺向 L1 move 内部的反向线段（L2 层）

任务：
  A. L1 趋势 move 内：顺向线段内部反向笔的密度/振幅 + 振幅总量分解
     （同级别反向线段捕获的反向振幅占比 vs 仅笔级可见占比）
  B. L2 趋势 move 内：反向 L1 move（L2 voice 对象）vs 顺向 L1 move 内部
     反向线段（L1 voice 对象）
  C. 级别×振幅汇总（判读表在报告中组装）

L2 构造说明：newchan_rust 未导出 zhongshu_from_components/
moves_from_level_zhongshus（level.rs pub fn，无 #[pyfunction]）。
但 zhongshu.rs::scan_zhongshu 与 level.rs::zhongshu_from_components 是
同构扫描（同三元重叠成形 zg>zd、同 try_extend、同 settled 续进锚
max(break-2,end)），moves 贪心分组逻辑也逐行同构。故 L2 用
zhongshu_from_segments(L1 moves 伪线段 (k,k,high,low,settled,settled))
+ moves_from_zhongshus 批量复刻——与 orchestrator 原生 L2 同构，
非原生输出（差异：组件过滤口径=settled L1 move，与 level 递归一致）。
回映射用 anchor（first_seg_s0/last_seg_s1 = 伪线段 k = L1 move 原始下标）。

振幅口径：
  - 笔振幅% = (high-low)/p0×100；线段/move 振幅% = (high-low)/起点价×100
  - 总量分解用绝对价差 Σ(high-low)（同标的内占比，避免基价漂移）

认识论等级：L2（单标的）/ L3（OKLO+BRN+CL 交叉）。L2 层样本量小（数十个
move），数字附 n。
运行：.venv/bin/python analysis/_counter_seg_level_dist.py [SYMBOLS]
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

_BI = ("wide", 5, False, 3)
OUT_JSON = _HERE / "data_cache" / "_counter_seg_level_dist.json"


def _pct(s: list[float], q: float) -> float:
    if not s:
        return float("nan")
    if len(s) == 1:
        return s[0]
    pos = q * (len(s) - 1)
    lo = int(pos)
    hi = min(lo + 1, len(s) - 1)
    return s[lo] * (1 - pos + lo) + s[hi] * (pos - lo)


def _dist(vals: list[float]) -> dict:
    s = sorted(vals)
    n = len(s)
    return {"n": n, "mean": (sum(s) / n) if n else float("nan"),
            "p25": _pct(s, 0.25), "p50": _pct(s, 0.50), "p75": _pct(s, 0.75)}


def analyze(symbol: str) -> dict:
    t0 = time.time()
    if os.environ.get("TS_MAX_BARS"):
        os.environ["BT_MAX_BARS"] = os.environ["TS_MAX_BARS"]
    opens, highs, lows, closes, _ = load_ohlc(SYMBOL_FILES[symbol])
    n_raw = len(closes)

    bi = R.BiEngine(*_BI)
    for i in range(n_raw):
        bi.process_bar(opens[i], highs[i], lows[i], closes[i])
    strokes = bi.current_strokes()  # (i0,i1,dir,high,low,p0,p1,confirmed)
    segs = R.segments_from_strokes_v1(strokes, 3, "strict")
    seg6 = [(s[0][0], s[0][1], s[0][5], s[0][6], s[0][7], s[0][8] == "settled")
            for s in segs]
    zss = R.zhongshu_from_segments(seg6)
    l1_moves = R.moves_from_zhongshus([tuple(z) for z in zss], len(segs))
    n_segs = len(segs)

    # L1 move seg_end 扩展（同 _bsp_gap_probe3.py）
    l1_heads = [list(m[0]) for m in l1_moves]
    for k, h in enumerate(l1_heads):
        h[3] = (l1_heads[k + 1][2] - 1) if k + 1 < len(l1_heads) else (n_segs - 1)

    def seg_head(j):
        return segs[j][0]  # (s0,s1,i0,i1,dir,high,low,confirmed,kind)

    def p_start_of_seg(j) -> float:
        return strokes[seg_head(j)[0]][5]

    # ════ A. L1 趋势内：顺向线段内部反向笔 + 振幅总量分解 ════
    trend_l1 = [h for h in l1_heads if h[0] == "trend" and h[6] >= 2]
    rev_strokes_per_fwd: list[float] = []
    rev_stroke_amp: list[float] = []
    n_fwd_segs = 0
    sum_rev_seg_abs = 0.0       # 同级别反向线段振幅总和（绝对价差）
    sum_rev_stk_fwd_abs = 0.0   # 顺向线段内部反向笔振幅总和（绝对价差）
    rev_strokes_per_move: list[float] = []

    for h in trend_l1:
        mdir = h[1]
        ss, se = max(int(h[2]), 0), min(int(h[3]), n_segs - 1)
        n_rev_stk_move = 0
        for j in range(ss, se + 1):
            sh = seg_head(j)
            if sh[4] != mdir:  # 同级别反向线段
                sum_rev_seg_abs += sh[5] - sh[6]
                continue
            n_fwd_segs += 1
            cnt = 0
            for t in range(sh[0], sh[1] + 1):
                st = strokes[t]
                if st[2] != mdir:  # 顺向线段内部的反向笔
                    cnt += 1
                    amp_abs = st[3] - st[4]
                    sum_rev_stk_fwd_abs += amp_abs
                    if st[5] > 0:
                        rev_stroke_amp.append(amp_abs / st[5] * 100)
            rev_strokes_per_fwd.append(float(cnt))
            n_rev_stk_move += cnt
        rev_strokes_per_move.append(float(n_rev_stk_move))

    tot_counter = sum_rev_seg_abs + sum_rev_stk_fwd_abs
    a_result = {
        "n_trend_moves": len(trend_l1),
        "n_fwd_segs": n_fwd_segs,
        "rev_strokes_per_fwd_seg": _dist(rev_strokes_per_fwd),
        "rev_strokes_in_fwd_per_move": _dist(rev_strokes_per_move),
        "rev_stroke_amp_pct": _dist(rev_stroke_amp),
        "amp_decomposition_abs": {
            "rev_segments": sum_rev_seg_abs,
            "rev_strokes_in_fwd_segs": sum_rev_stk_fwd_abs,
            "share_same_level": sum_rev_seg_abs / tot_counter if tot_counter else float("nan"),
            "share_stroke_only": sum_rev_stk_fwd_abs / tot_counter if tot_counter else float("nan"),
        },
    }

    # ════ B. L2 趋势内：反向 L1 move vs 顺向 L1 move 内部反向线段 ════
    # 伪线段：L1 move k → (k,k,high,low,settled,settled)；anchor=k 用于回映射
    pseudo = [(k, k, m[1][0], m[1][1], bool(m[0][7]), bool(m[0][7]))
              for k, m in enumerate(l1_moves)]
    zss_l2 = R.zhongshu_from_segments(pseudo)
    l2_moves = R.moves_from_zhongshus([tuple(z) for z in zss_l2], None)

    # L2 move 覆盖范围（L1 move 原始下标，anchor 空间）：
    # [m.first_seg_s0, 下一 L2 move 的 first_seg_s0 - 1]，末 move → len(l1)-1
    l2_list = []
    for k, m in enumerate(l2_moves):
        k0 = m[1][2]  # first_seg_s0 anchor = L1 move 下标
        k1 = (l2_moves[k + 1][1][2] - 1) if k + 1 < len(l2_moves) else (len(l1_moves) - 1)
        l2_list.append((m[0][0], m[0][1], m[0][6], bool(m[0][7]), k0, k1))

    trend_l2 = [x for x in l2_list if x[0] == "trend" and x[2] >= 2]
    rev_l1_per_l2: list[float] = []
    rev_l1_amp: list[float] = []
    rev_seg_in_fwd_l1_per_l2: list[float] = []
    rev_seg_in_fwd_l1_amp: list[float] = []
    n_fwd_l1 = 0

    for _kind, d2, _zc, _settled, k0, k1 in trend_l2:
        n_rev_l1 = 0
        n_rev_seg = 0
        for k in range(k0, min(k1, len(l1_heads) - 1) + 1):
            h = l1_heads[k]
            ss, se = max(int(h[2]), 0), min(int(h[3]), n_segs - 1)
            p0 = p_start_of_seg(ss) if ss < n_segs else 0.0
            if h[1] != d2:  # 反向 L1 move（L2 voice 对象）
                n_rev_l1 += 1
                if p0 > 0:
                    rev_l1_amp.append((l1_moves[k][1][0] - l1_moves[k][1][1]) / p0 * 100)
                continue
            n_fwd_l1 += 1
            for j in range(ss, se + 1):  # 顺向 L1 move 内部反向线段（L1 voice 对象）
                sh = seg_head(j)
                if sh[4] != d2:
                    n_rev_seg += 1
                    ps = p_start_of_seg(j)
                    if ps > 0:
                        rev_seg_in_fwd_l1_amp.append((sh[5] - sh[6]) / ps * 100)
        rev_l1_per_l2.append(float(n_rev_l1))
        rev_seg_in_fwd_l1_per_l2.append(float(n_rev_seg))

    b_result = {
        "n_l1_moves": len(l1_moves),
        "n_l2_zhongshus": len(zss_l2),
        "n_l2_moves": len(l2_moves),
        "n_l2_trend_moves": len(trend_l2),
        "n_fwd_l1_moves_in_l2_trend": n_fwd_l1,
        "rev_l1_moves_per_l2_trend": _dist(rev_l1_per_l2),
        "rev_l1_move_amp_pct": _dist(rev_l1_amp),
        "rev_segs_in_fwd_l1_per_l2_trend": _dist(rev_seg_in_fwd_l1_per_l2),
        "rev_seg_in_fwd_l1_amp_pct": _dist(rev_seg_in_fwd_l1_amp),
    }

    return {
        "symbol": symbol,
        "n_raw_bars": n_raw,
        "A_l1_sublevel": a_result,
        "B_l2_levels": b_result,
        "elapsed_s": round(time.time() - t0, 1),
    }


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
