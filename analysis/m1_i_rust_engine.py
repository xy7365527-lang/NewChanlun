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
    # top2_only：这些 proxy PH 仅经 detect_settle 读 bars[1].birth_idx（无 rank≥2 /
    # median / ratio 消费者）→ 走 _fast_alive_top2（消除全量 sort 的 O(N^1.6) 主常数）。
    bar_dn = PHLevelState.make(top2_only=True); bar_up = PHLevelState.make(top2_only=True)
    bi_dn = PHLevelState.make(top2_only=True); bi_up = PHLevelState.make(top2_only=True)
    last_stroke_n = 0

    # BSP 去重 seen-set（递归层 N≥2，仍在 Python；走势级/笔中枢级已下沉 Rust：
    # trend_new_signals / bi_zhongshu_new_signals 各自内部维护 seen-set + epoch 门控）。
    level_seen: dict[int, set] = {}

    max_ladder = LADDER_SEG  # segment 级（笔中枢）已是最低中枢承载层

    # 递归层状态缓存（diff 门控：状态不变 ⟺ 无事件 → 跳过重算）
    level_cache: dict[int, tuple] = {}
    # 递归层 epoch 门控（O(1)）：内容未变 ⟹ 跳过整个 current_recursive() marshal+扫描。
    last_rec_epoch: int = -1

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
            # P1 优化：O(N) 全链增量 delta 接口——四层增量器 + seen-set/signal_anchor
            # 下沉 Rust，消除 marshal O(B) + 调用方扫描 O(B) → 端到端 O(N)（替代原
            # current_bi_zhongshu_buysellpoints 全量重算的 O(strokes²)）。逐位等价于
            # _scan_new(current_bi_zhongshu_buysellpoints_inc(...), seg_seen)（differential_tests 守卫）。
            # 调用契约（lib.rs sync_inc）：仅在 stroke_count 增长时调用 → 与本门控一致。
            seg_buy1, seg_sell1, seg_sell_any, seg_buy_any = \
                orch.bi_zhongshu_new_signals(BI_ZHONGSHU_LEVEL_ID)

        # ── ladder3 走势级：Rust delta 接口（trend_new_signals）──
        # seen-set（trend + type2）下沉 Rust + bsp_epoch O(1) 门控，消除旧每-bar 全量
        # current_buysellpoints() marshal + _scan_new 扫描的 B-scaling O(B·n_bsp)。
        # 逐位等价于 _scan_new(current_buysellpoints(), trend_seen) + type2 首现扫描。
        l1_buy1, l1_sell1, l1_sell_any, l1_buy_any, type2_buy = \
            orch.trend_new_signals(BI_ZHONGSHU_LEVEL_ID)

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

        # ── 递归层（ladder≥4）：recursive_epoch O(1) 门控 + 等价 BSP 管线 ──
        # 递归内容未变（epoch 不变）⟹ current_recursive() 逐位不变 ⟹ 各层 cache_key
        # 必命中 → 无新 confirmed → 所有递归层信号本 bar 必为 false，max_ladder 不变。
        # 故跳过整块（消除每-bar 全量 current_recursive() marshal + cache_key 构造的
        # B-scaling）。仅递归重算 bar（O(n_l1_moves) 次，稀疏）进入下方扫描。
        rec_epoch = orch.recursive_epoch()
        if rec_epoch != last_rec_epoch:
            last_rec_epoch = rec_epoch
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
