"""Rust 引擎驱动的 Version I 完整版信号层（替代 Python `compute_signals_i` 的 O(N²)）。

## 为何需要

`fugue_version_i.compute_signals_i` 用 Python `RecursiveOrchestrator` 逐 bar 驱动全链
（笔/线段/中枢/走势/递归层 + 每层 confirmed BSP），线段层全量重算 + per_level_bsp
全量重算 → O(N²)，447K OKLO 已是上限。10 年期货（2M–5.6M bar/标的）不可行。

本模块用 **Rust orchestrator** 跑 `process_bar`（逐位等价、快 ~23×），每 bar 取 Rust
暴露的 bit-exact **状态** tuple（`current_strokes/segments/moves/buysellpoints/recursive`），
在 tuple 空间用 **Rust 纯函数**（`zhongshu_from_strokes`/`moves_from_zhongshus`/
`divergences_from_moves_v1`/`buysellpoints_from_level`）重算每层 confirmed BSP——
这些纯函数是 bit-exact 移植（`_probe_bi_bsp_equiv.py` 证 bi-zhongshu 路径 4 cutoff 全等）。

## 严格等价（状态-事件二象性）

Rust 只暴露状态（`current_*`），不暴露事件——事件不参与状态递归（移植边界）。
Version I 的 BSP 信号本就由 **状态 + 跨 bar 去重 seen-set** 检测"新 confirmed BSP"
（`_BiZhongshuBspTracker`/`_TrendBspTracker`/`_LevelBspTracker` 均按 (kind,side,seg_idx)
去重），故无需事件——状态 diff 与事件等价（状态不变 ⟺ 无事件）。

唯一被替换的是引擎来源（Python orchestrator → Rust orchestrator）+ BSP 重算的纯函数
来源（Python per_level_bsp → Rust 纯函数，逐位等价）。bar/bi PH 门控、BSP 去重、
ladder 归属逻辑全部逐字复用 `fugue_version_i` 语义。

## 等价口径与门控（与 compute_signals_i 逐字对齐）

- ladder0 bar：close PH（`bar_dn/bar_up`），每 bar。
- ladder1 bi：新 stroke 端点 `p1` PH（`bi_dn/bi_up`），**仅 stroke 计数增长时**（与
  Python `len(strokes) > last_stroke_n` 门控逐字一致——决定"新 BSP"首报 bar）。
- ladder2 segment：笔中枢真实 BSP，**仅 stroke 计数增长时**重算（同上门控）。
- ladder3 走势：`current_buysellpoints()` 透传，**每 bar**（同 Python trend_bsp.step）。
- ladder≥4 递归：`confirmed_bsp_for_level` 等价管线，**该层状态变化时**重算（状态 diff
  ⟺ Python move_events/zhongshu_events 门控）。max_ladder 在该层存在时即更新（不依赖变化）。
- type2_buy：报告字段（仅计入 `addon_2buy_marks`，不进 trades/metrics）。Rust 无事件，
  用 current_buysellpoints 中 type2 buy 的状态-diff 首现近似（标注：报告口径，非 PnL）。

认识论等级：移植正确性 L0/L1（管线等价，由 differential test 保证）；
回测结论 L2（真实数据，run_version_i 在真实 OHLC 上产出）。
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as R  # noqa: E402

from fugue_alpha_diagnosis import PHLevelState  # noqa: E402
from fugue_version_i import (  # noqa: E402
    BarSignalI,
    LADDER_BAR,
    LADDER_BI,
    LADDER_MOVE,
    LADDER_SEG,
    MAX_LADDER,
    MAX_LEVELS,
)
from per_level_bsp import BI_ZHONGSHU_LEVEL_ID  # noqa: E402


# ════════════════════════════════════════════════════════════
# Rust 纯函数 BSP 管线（tuple 空间，逐位等价于 per_level_bsp）
# ════════════════════════════════════════════════════════════

def _level_bsps(prev_moves: list, level_zhongshus: list, level_moves: list,
                level_id: int) -> list:
    """递归层 N≥2 confirmed BSP（等价于 `confirmed_bsp_for_level`）。

    prev_moves     = 前一级别 (N-1) MoveTuple 列表（组件序列，i0=first_seg_s0…）
    level_zhongshus= 本级别 (N) LevelZhongshuTuple 列表（11 字段）
    level_moves    = 本级别 (N) MoveTuple 列表
    返回 BspTuple 列表。
    """
    # 前级别 move → SegInput：(direction, high, low, i0=first_seg_s0, i1=last_seg_s1)
    seg_in = [(m[0][1], m[1][0], m[1][1], m[1][2], m[1][3]) for m in prev_moves]
    # LevelZhongshu → divergences zs5：(zd,zg,seg_start=comp_start,seg_end=comp_end,settled)
    zs5 = [(z[0], z[1], z[2], z[3], z[5]) for z in level_zhongshus]
    mv_in = [m[0] for m in level_moves]
    divs = R.divergences_from_moves_v1(seg_in, zs5, mv_in, level_id)
    # LevelZhongshu → buysellpoints zs7：+break_direction(z[7]), break_seg=break_comp(z[6])
    zs7 = [(z[0], z[1], z[2], z[3], z[5], z[7], z[6]) for z in level_zhongshus]
    div_in = [(d[0][0], d[0][1], d[0][7], d[0][5], d[0][6], d[1][0], d[1][1]) for d in divs]
    return R.buysellpoints_from_level(seg_in, zs7, mv_in, div_in, level_id)


# ════════════════════════════════════════════════════════════
# 去重跟踪器（按 (kind,side,seg_idx)，只报本 bar 新增 confirmed）
# ════════════════════════════════════════════════════════════

def _scan_new(bsps: list, seen: set) -> tuple[bool, bool, bool, bool]:
    """从 BspTuple 列表 + seen-set 检测本 bar 新增 confirmed BSP 信号。

    BspTuple.head = (kind, side, level_id, seg_idx, move_seg_start, confirmed, settled)。
    返回 (buy1, sell1, sell_any, buy_any)——与 fugue_version_i 各 tracker.step 逐字一致。
    """
    b1 = s1 = sa = ba = False
    for bp in bsps:
        head = bp[0]
        kind, side, confirmed, seg_idx = head[0], head[1], head[5], head[3]
        if not confirmed:
            continue
        key = (kind, side, seg_idx)
        if key in seen:
            continue
        seen.add(key)
        if side == "buy":
            ba = True
            if kind == "type1":
                b1 = True
        else:
            sa = True
            if kind == "type1":
                s1 = True
    return b1, s1, sa, ba


# ════════════════════════════════════════════════════════════
# 信号层（compute-once）：单次 Rust orch pass → I 磁带
# ════════════════════════════════════════════════════════════

def compute_i_signals_rust(
    opens: list[float], highs: list[float], lows: list[float], closes: list[float],
) -> list[BarSignalI]:
    """Rust 引擎驱动，产出 I 分层信号磁带（逐位等价于 compute_signals_i 的 i_signals）。"""
    n = len(closes)
    orch = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)

    # sub-走势 PH 树（bar / bi）—— ladder 0/1 无中枢，PH proxy（逐字同 compute_signals_i）。
    bar_dn = PHLevelState.make(); bar_up = PHLevelState.make()
    bi_dn = PHLevelState.make(); bi_up = PHLevelState.make()
    last_stroke_n = 0

    # BSP 去重 seen-set（segment 笔中枢 / 走势级 / 各递归层）
    seg_seen: set = set()
    trend_seen: set = set()
    level_seen: dict[int, set] = {}
    # type2_buy 报告口径：current_buysellpoints 中 type2 buy 的状态-diff 首现
    type2_seen: set = set()

    max_ladder = LADDER_SEG  # segment 级（笔中枢）已是最低中枢承载层

    # 递归层状态缓存（diff 门控：状态不变 ⟺ 无事件 → 跳过重算）
    level_cache: dict[int, tuple] = {}

    i_signals: list[BarSignalI] = []
    last_progress = 0

    for i in range(n):
        c = closes[i]
        orch.process_bar(opens[i], highs[i], lows[i], c)

        # ── ladder0 bar：close PH（每 bar）──
        bar_buy_r1, bar_buy_nr1 = bar_dn.detect_settle(bar_dn.tree.update(c))
        bar_sell_r1, bar_sell_nr1 = bar_up.detect_settle(bar_up.tree.update(-c))
        bar_buy = bar_buy_r1 or bar_buy_nr1
        bar_sell = bar_sell_r1 or bar_sell_nr1

        # ── ladder1 bi PH + ladder2 笔中枢 BSP：仅 stroke 计数增长时 ──
        # P3 修复：用 O(1) stroke_count() 门控，避免每 bar 全量 marshal current_strokes()。
        bi_buy = False; bi_sell = False
        seg_buy1 = seg_sell1 = seg_sell_any = seg_buy_any = False
        sc = orch.stroke_count()
        if sc > last_stroke_n:
            for p1 in orch.strokes_p1_since(last_stroke_n):  # 只 marshal 新增笔的 p1
                if p1 > 0:
                    r1d, nr1d = bi_dn.detect_settle(bi_dn.tree.update(p1))
                    r1u, nr1u = bi_up.detect_settle(bi_up.tree.update(-p1))
                    if r1d or nr1d:
                        bi_buy = True
                    if r1u or nr1u:
                        bi_sell = True
            last_stroke_n = sc
            # P2 优化：合并全链单次 Rust 调用（直读内部笔，零 stroke marshal）。
            seg_buy1, seg_sell1, seg_sell_any, seg_buy_any = _scan_new(
                orch.current_bi_zhongshu_buysellpoints(BI_ZHONGSHU_LEVEL_ID), seg_seen)

        # ── ladder3 走势级：current_buysellpoints 透传（每 bar）──
        bsps_l1 = orch.current_buysellpoints()
        l1_buy1, l1_sell1, l1_sell_any, l1_buy_any = _scan_new(bsps_l1, trend_seen)
        # type2_buy（报告口径：type2 buy 状态首现）
        type2_buy = False
        for bp in bsps_l1:
            head = bp[0]
            if head[1] == "buy" and head[0] == "type2":
                key = (head[3], head[4])  # (seg_idx, move_seg_start)
                if key not in type2_seen:
                    type2_seen.add(key)
                    type2_buy = True

        # ── I 分层磁带 ──
        buy1 = [False] * MAX_LADDER
        sell1 = [False] * MAX_LADDER
        sell_any = [False] * MAX_LADDER
        buy_any = [False] * MAX_LADDER
        sell_any[LADDER_BAR] = bar_sell; buy_any[LADDER_BAR] = bar_buy
        sell_any[LADDER_BI] = bi_sell; buy_any[LADDER_BI] = bi_buy
        buy1[LADDER_SEG] = seg_buy1; sell1[LADDER_SEG] = seg_sell1
        sell_any[LADDER_SEG] = seg_sell_any; buy_any[LADDER_SEG] = seg_buy_any
        buy1[LADDER_MOVE] = l1_buy1; sell1[LADDER_MOVE] = l1_sell1
        sell_any[LADDER_MOVE] = l1_sell_any; buy_any[LADDER_MOVE] = l1_buy_any

        # ── 递归层（ladder≥4）：状态 diff 门控 + 等价 BSP 管线 ──
        recursive = orch.current_recursive()  # list[(level_id, [LZS], [Move])]
        level_moves_map = {lid: mvs for (lid, _z, mvs) in recursive}
        moves_l1 = None  # current_moves() 懒取——仅 level 2 需重算时
        for (lid, zhs, mvs) in recursive:
            ladder = lid + 2
            if ladder >= MAX_LADDER:
                continue
            if ladder > max_ladder:
                max_ladder = ladder
            # 状态 diff 门控（状态不变 ⟺ Python move_events/zhongshu_events 均空 → 无新 confirmed）
            cache_key = (tuple(zhs), tuple(mvs))
            if level_cache.get(lid) == cache_key:
                continue
            level_cache[lid] = cache_key
            if lid - 1 == 1:
                if moves_l1 is None:
                    moves_l1 = orch.current_moves()
                prev = moves_l1
            else:
                prev = level_moves_map.get(lid - 1, [])
            seen = level_seen.get(lid)
            if seen is None:
                seen = set(); level_seen[lid] = seen
            b1, s1, sa, ba = _scan_new(_level_bsps(prev, zhs, mvs, lid), seen)
            buy1[ladder] = b1; sell1[ladder] = s1
            sell_any[ladder] = sa; buy_any[ladder] = ba

        i_signals.append(BarSignalI(
            close=c, buy1=tuple(buy1), sell1=tuple(sell1),
            sell_any=tuple(sell_any), buy_any=tuple(buy_any),
            max_ladder=max_ladder, type2_buy=type2_buy))

        if i - last_progress >= 500_000:
            print(f"    [{i / n * 100:5.1f}%] rust I-signal bar {i:,}/{n:,}", flush=True)
            last_progress = i

    return i_signals
