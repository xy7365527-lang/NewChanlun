"""验证「缺失 type1 是结构性还是 OKLO 特例」。

公理（编排者）：走势终完美 ⟹ 每个笔端点（走势方向反转）必然有次级别 type1 背驰。
目标：对 CL/BTC 重复 OKLO 的 L1 走势反转↔type1 分类；对 OKLO 的 L2/L3 递归层
同样分类。判断失败模式（type3-only 主导）是否跨标的跨层一致。

分类（每个走势反转一桶）：
  has_t1_conf      —— 存在 confirmed type1（side 匹配反转方向）
  has_t1_cand_only —— 有 type1 candidate（背驰发射但 confirmed 门拦截），无 confirmed
  no_t1_has_t3     —— 无任何 type1，但反转处有 type3
  nothing          —— 反转处既无 type1 也无 type3

口径（锚定模型，已经 OKLO 校准复现实证基线 ~47/4/54）：
  走势反转 = current_moves()（或递归层 moves）中相邻 settled move 方向变化处。
  完成 move = 反转前 move（direction 决定反转 side：up 完成→sell；down 完成→buy）。
  type1 锚定（走势完美/结束，买卖点 AT 反转）：BSP.move_seg_start ==
    完成move.seg_start ∧ side 一致。注意不能用 seg_idx 区间——type1 的 seg_idx
    （=div.seg_c_end=C 段末走势转折段）因 B2 越界定义会落到后继 move 段域。
  type3 锚定（新走势开始/中枢突破，买卖点在突破 move）：BSP.move_seg_start ∈
    {完成move.seg_start, 后继move.seg_start}（type3 属突破段，锚在新 move）。
    side 不限——中枢突破的方向由突破段决定，反转处两侧 type3 均算"有 type3"。

数据结构（已逐行核对 rust/src/lib.rs）：
  MoveTuple.head = (kind,dir,seg_start,seg_end,zs_start,zs_end,zs_count,settled)
  BspTuple.head  = (kind,side,level_id,seg_idx,move_seg_start,confirmed,settled)
  ZhongshuTuple  = (zd,zg,seg_start,seg_end,seg_count,settled,break_seg,break_dir,
                    first_seg_s0,last_seg_s1,gg,dd)  ← 扁平 12 元组
  SegmentTuple.head=(s0,s1,i0,i1,direction,high,low,confirmed,kind)
  DivergenceTuple.head=(kind,dir,level_id,a_start,a_end,c_start,c_end,center_idx)
    .body=(force_a,force_c,confirmed,...)
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402

from fugue_v2_full_backtest import SYMBOL_FILES, load_ohlc  # noqa: E402
from organic_signals import _level_bsps_with_divs, MAX_LEVELS  # noqa: E402

TYPE1_CONFIRM_RATIO = 0.9  # = buysellpoint.rs::TYPE1_CONFIRM_RATIO

BUCKETS = ("has_t1_conf", "has_t1_cand_only", "no_t1_has_t3", "nothing")


# ════════════════════════════════════════════════════════════
# 反转检测
# ════════════════════════════════════════════════════════════

def settled_reversals(moves: list) -> list[tuple]:
    """相邻 settled move 方向变化 → [(completed_move, next_move)]。"""
    settled = [m for m in moves if m[0][7]]
    revs = []
    for k in range(len(settled) - 1):
        a, b = settled[k], settled[k + 1]
        if a[0][1] != b[0][1]:
            revs.append((a, b))
    return revs


# ════════════════════════════════════════════════════════════
# 分类
# ════════════════════════════════════════════════════════════

def classify_reversal(completed, nxt, t1_by_mss, t3_by_mss, div_rows) -> str:
    """锚定模型分类。

    t1_by_mss: dict move_seg_start → [(side, confirmed)]（type1 BSP）。
    t3_by_mss: dict move_seg_start → [side]（type3 BSP）。
    div_rows:  [(side, seg_c_end, force_a, force_c)]（该层 trend 背驰；candidate 兜底）。
    """
    comp_dir = completed[0][1]
    side = "sell" if comp_dir == "up" else "buy"
    comp_mss = completed[0][2]
    next_mss = nxt[0][2]
    comp_seg_end = completed[0][3]

    t1_rows = t1_by_mss.get(comp_mss, [])

    # confirmed type1 锚在完成 move，side 一致
    if any(s == side and conf for (s, conf) in t1_rows):
        return "has_t1_conf"

    # candidate：(a) 完成 move 上有 unconfirmed type1 同 side；
    #            (b) trend 背驰发射（seg_c_end 落在完成 move 段域）但比值未过门
    has_cand = any(s == side and (not conf) for (s, conf) in t1_rows)
    if not has_cand:
        for (s, sce, fa, fc) in div_rows:
            if s == side and comp_mss <= sce <= next_mss + (comp_seg_end - comp_mss + 1) \
                    and fa > 0.0 and fc / fa > TYPE1_CONFIRM_RATIO:
                has_cand = True
                break
    if has_cand:
        return "has_t1_cand_only"

    # type3 锚在突破段（新 move）或完成 move——side 不限
    if t3_by_mss.get(comp_mss) or t3_by_mss.get(next_mss):
        return "no_t1_has_t3"

    return "nothing"


def _index_bsps(bsps):
    """BSP 列表 → (t1_by_mss, t3_by_mss)。

    t1_by_mss[move_seg_start] = [(side, confirmed)]；t3_by_mss[move_seg_start]=[side]。
    BspTuple.head = (kind, side, level_id, seg_idx, move_seg_start, confirmed, settled)。
    """
    from collections import defaultdict
    t1 = defaultdict(list)
    t3 = defaultdict(list)
    for b in bsps:
        kind, side, mss, conf = b[0][0], b[0][1], b[0][4], b[0][5]
        if kind == "type1":
            t1[mss].append((side, conf))
        elif kind == "type3":
            t3[mss].append(side)
    return t1, t3


def bucketize(revs, t1_by_mss, t3_by_mss, div_rows) -> dict:
    counts = {b: 0 for b in BUCKETS}
    detail = {b: 0 for b in BUCKETS}  # 仅用于 trend∧zs≥2 的完成 move 子集
    n_trend_geq2 = 0
    for completed, nxt in revs:
        b = classify_reversal(completed, nxt, t1_by_mss, t3_by_mss, div_rows)
        counts[b] += 1
        # 完成 move 是否真趋势（结构上应可背驰）
        if completed[0][0] == "trend" and completed[0][6] >= 2:
            n_trend_geq2 += 1
            detail[b] += 1
    counts["total"] = len(revs)
    counts["completed_trend_zs2"] = n_trend_geq2
    counts["trend_zs2_breakdown"] = detail
    if len(revs):
        counts["t3_only_pct"] = round(
            100.0 * counts["no_t1_has_t3"] / len(revs), 1)
        counts["t1_conf_pct"] = round(
            100.0 * counts["has_t1_conf"] / len(revs), 1)
    else:
        counts["t3_only_pct"] = None
        counts["t1_conf_pct"] = None
    return counts


# ════════════════════════════════════════════════════════════
# L1（ladder3 走势级）
# ════════════════════════════════════════════════════════════

def analyze_l1(orch) -> dict:
    moves = orch.current_moves()
    bsps = orch.current_buysellpoints()
    revs = settled_reversals(moves)

    t1_by_mss, t3_by_mss = _index_bsps(bsps)

    segs = orch.current_segments()
    seg_in = [(s[0][4], s[0][5], s[0][6], s[0][2], s[0][3]) for s in segs]
    #          dir,      high,    low,     i0,      i1
    zss = orch.current_zhongshus()
    zs5 = [(z[0], z[1], z[2], z[3], z[5]) for z in zss]  # zd,zg,seg_start,seg_end,settled
    mv_in = [m[0] for m in moves]
    divs = R.divergences_from_moves_v1(seg_in, zs5, mv_in, 0, None, None, None)
    div_rows = []
    for d in divs:
        head = d[0]
        if head[0] != "trend":
            continue
        side = "sell" if head[1] == "top" else "buy"
        seg_c_end = head[6]
        fa, fc = d[1][0], d[1][1]
        div_rows.append((side, seg_c_end, fa, fc))

    return bucketize(revs, t1_by_mss, t3_by_mss, div_rows)


# ════════════════════════════════════════════════════════════
# L2/L3 递归层
# ════════════════════════════════════════════════════════════

def analyze_recursive(orch) -> dict:
    recursive = orch.current_recursive()
    moves_l1 = orch.current_moves()
    level_moves_map = {lid: mvs for (lid, _z, mvs) in recursive}

    out = {}
    for (lid, zhs, mvs) in recursive:
        if lid < 1:
            continue
        prev = moves_l1 if (lid - 1 == 1) else level_moves_map.get(lid - 1, [])
        ladder = lid + 2  # lid=1→ladder3=L1; lid=2→ladder4=L2; lid=3→ladder5=L3
        if not prev or not mvs:
            out[f"L{lid}_ladder{ladder}"] = {b: 0 for b in BUCKETS} | {
                "total": 0, "completed_trend_zs2": 0,
                "trend_zs2_breakdown": {b: 0 for b in BUCKETS},
                "t3_only_pct": None, "t1_conf_pct": None}
            continue
        bsps, div_full = _level_bsps_with_divs(prev, zhs, mvs, lid)
        t1_by_mss, t3_by_mss = _index_bsps(bsps)
        div_rows = []
        for (kind, ddir, sce, fa, fc, _price) in div_full:
            if kind != "trend":
                continue
            side = "sell" if ddir == "top" else "buy"
            div_rows.append((side, sce, fa, fc))
        revs = settled_reversals(mvs)
        out[f"L{lid}_ladder{ladder}"] = bucketize(
            revs, t1_by_mss, t3_by_mss, div_rows)
    return out


# ════════════════════════════════════════════════════════════
# 驱动
# ════════════════════════════════════════════════════════════

def run_symbol(symbol: str, want_recursive: bool) -> dict:
    o, h, l, c, _ = load_ohlc(SYMBOL_FILES[symbol])
    orch = R.RecursiveOrchestrator(max_levels=MAX_LEVELS,
                                   require_settled_subseg=True)
    n = len(c)
    for i in range(n):
        orch.process_bar(o[i], h[i], l[i], c[i])
    res = {"symbol": symbol, "n_bars": n, "L1": analyze_l1(orch)}
    if want_recursive:
        res["recursive"] = analyze_recursive(orch)
    print(f"[done] {symbol}: {n:,} bars, L1 total={res['L1']['total']}",
          file=sys.stderr, flush=True)
    return res


def main():
    results = {}
    results["OKLO"] = run_symbol("OKLO", want_recursive=True)
    for sym in ("CL", "BTC"):
        results[sym] = run_symbol(sym, want_recursive=False)
    print(json.dumps(results, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
