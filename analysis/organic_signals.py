"""有机赋格信号层（公共模块）— Rust 引擎 I 磁带 + BSP 事件流 + 背驰事件流。

═══════════════════════════════════════════════════════════════════════
来源与职责
═══════════════════════════════════════════════════════════════════════
从 `interval_nesting_reverse_backtest.compute_i_signals_rust_events` 抽出为公共
模块（organic_fugue_design §5.1/§6），并扩展两个字段：

  - `BarSignalI.div_events[ladder]`：该层本 bar 新背驰事件流。盘整背驰
    （kind="consolidation"）首次对下游可见——此前被 BSP 管线折叠（type1 BSP 只
    携带趋势背驰），这是 E10（信号层不折叠信息）的最后一块。
  - `BarSignalI.up_move_settled[ladder]`：该层本 bar 是否有新向上 move settle
    （41课 FatigueMonitor 的证据清空触发：创新动力 = 衰竭被市场否定）。
    域声明：ladder2（笔中枢层）该行恒 False——其唯一消费者 FatigueMonitor
    的 u(k) ≥ FIRST_BSP_LADDER+1 = 3 不可达 ladder2（u(k) = k 之上最近承载层，
    k ≥ 2）；笔中枢 move 层未在引擎暴露面（若未来需要，经增量引擎 move 层
    加只读 accessor，同 divergences 先例）。

布尔流 / bsp_events 与原实现**逐位等价**（差分守卫内嵌：每个重算点把布尔导出与
Rust delta 接口对比，不等 → RuntimeError）。新字段默认不被旧消费者读取
（run_version_i 的 P1-P7 逐位不变）。

═══════════════════════════════════════════════════════════════════════
各 ladder 背驰来源（全部增量缓存直读，零重算——O(N) 修复后形态）
═══════════════════════════════════════════════════════════════════════
  ladder2（笔中枢）：`current_bi_zhongshu_divergences_inc`——增量引擎
    IncrementalBiZhongshuBsp 的 divs 层缓存直读（背驰本就是 BSP 链中间产物，
    此前算完即弃；接口为本次新增的加法式只读 marshal，O(n_div)/次）。
    调用门控与 BSP 事件流相同（stroke 增长 bar）。
  ladder3（走势级）：`current_trend_divergences`——orchestrator inc_seg_div
    增量缓存直读，bsp_epoch 门控（与 current_buysellpoints 同步点）。
  ladder≥4（递归层）：divergences 本就是 `_level_bsps` 的中间产物，surfacing
    而非新计算（设计 §5.1，增量成本≈0）。

发生史（性能修订）：首版 ladder2 在引擎"零改动"约束下用 current_strokes()
marshal + 纯函数全链重算（O(S²) 摊还，OKLO 447K 信号层 17.8s→402s）。编排者
否定该成本（O(N) 硬约束："用 epoch 门控或 delta 接口"）——"零改动"前提
（设计 §6"全部所需事件已可从状态/纯函数得出"）对盘整背驰不成立，故修订为
加法式只读接口（不触碰任何现有计算路径，引擎 37 项差分测试不变）。
事件流与 O(S²) 版逐位等价（增量引擎 current() ≡ 全量纯函数的既有契约 +
本模块 120K 新旧实现流式对比守卫）。

认识论等级：管线等价性 L1（差分守卫 + 新旧磁带逐位对比）；事件流语义 L0
（透传引擎定义，零新定义）。

用法：PYTHONPATH=src python -c "from organic_signals import compute_organic_signals"
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
    LADDER_BAR,
    LADDER_BI,
    LADDER_MOVE,
    LADDER_SEG,
    MAX_LADDER,
    MAX_LEVELS,
    NO_LADDER_DIVS,
    NO_LADDER_EVENTS,
    NO_UP_SETTLED,
    BarSignalI,
)
from per_level_bsp import BI_ZHONGSHU_LEVEL_ID  # noqa: E402

__all__ = [
    "compute_organic_signals",
    "compute_i_signals_rust_events",
]


# ════════════════════════════════════════════════════════════
# BspTuple 事件扫描器（逐字承自 interval_nesting_reverse_backtest）
# ════════════════════════════════════════════════════════════

def _scan_events_rust(bsps: list, seen: set) -> tuple[bool, bool, bool, bool, list]:
    """BspTuple 列表 → (buy1, sell1, sell_any, buy_any, events)。

    去重键 (kind, side, seg_idx, confirmed)：candidate/confirmed 分别入流。
    布尔仅由新 confirmed 事件置位 → 与旧键 (kind,side,seg_idx) confirmed-only
    扫描逐位等价（证明见 fugue_version_i._scan_bsp_events docstring；
    差分守卫逐重算点复核）。

    BspTuple = ((kind, side, level_id, seg_idx, move_seg_start, confirmed, settled),
                (center_zd, center_zg, price, bar_idx),
                divergence_key, center_seg_start, overlaps_with)
    事件 = (kind, side, seg_idx, confirmed, center_seg_start, center_zd,
            center_zg, price)
    """
    b1 = s1 = sa = ba = False
    events: list = []
    for bp in bsps:
        head = bp[0]
        kind, side, seg_idx, confirmed = head[0], head[1], head[3], head[5]
        key = (kind, side, seg_idx, confirmed)
        if key in seen:
            continue
        seen.add(key)
        mid = bp[1]
        events.append((kind, side, seg_idx, confirmed, bp[3], mid[0], mid[1], mid[2]))
        if not confirmed:
            continue
        if side == "buy":
            ba = True
            if kind == "type1":
                b1 = True
        else:
            sa = True
            if kind == "type1":
                s1 = True
    return b1, s1, sa, ba, events


# ════════════════════════════════════════════════════════════
# 背驰事件扫描器（§5.1 div_events；输入 = 扁平 6 元组）
# ════════════════════════════════════════════════════════════

def _scan_div_events(div_rows: list, seen: set) -> list:
    """扁平背驰行 → 新背驰事件流（per-ladder seen-set 去重）。

    输入行 = (kind, direction, seg_c_end, force_a, force_c, price)
    （Rust `current_*_divergences*` 接口的输出格式；递归层由
    `_level_bsps_with_divs` 组装为同格式）。
    事件 = (kind, direction, side, seg_idx, force_a, force_c, price)：
      direction 映射：引擎 "top"（向上段衰竭）→ "up"/side="sell"；
                      "bottom" → "down"/side="buy"。
      seg_idx = seg_c_end（背驰段锚）；去重键 = (kind, 引擎direction, seg_c_end)。
      price = 背驰段端点价（sell→段 high / buy→段 low，与 type1 BSP price 同构，
      由产出方按该层 segments 序列计算）。
    """
    out: list = []
    for kind, ddir, seg_c_end, fa, fc, price in div_rows:
        key = (kind, ddir, seg_c_end)
        if key in seen:
            continue
        seen.add(key)
        if ddir == "top":
            out.append((kind, "up", "sell", seg_c_end, fa, fc, price))
        else:
            out.append((kind, "down", "buy", seg_c_end, fa, fc, price))
    return out


def _new_settled_up_moves(mv_heads: list, seen: set) -> bool:
    """move head 列表 → 本次是否出现新 settled 向上 move（身份键 (direction, seg_start)）。

    head = (kind, direction, seg_start, seg_end, zs_start, zs_end, zs_count, settled)。
    settled move 身份在 append-only 组件序列上稳定（settled 单调，前缀冻结）。
    """
    new_up = False
    for h in mv_heads:
        if not h[7]:
            continue
        key = (h[1], h[2])
        if key in seen:
            continue
        seen.add(key)
        if h[1] == "up":
            new_up = True
    return new_up


# ════════════════════════════════════════════════════════════
# 递归层 BSP+背驰管线（_level_bsps 的 surfacing 版）
# ════════════════════════════════════════════════════════════

def _level_bsps_with_divs(prev_moves: list, level_zhongshus: list,
                          level_moves: list, level_id: int) -> tuple[list, list]:
    """递归层 N≥2 confirmed BSP + 背驰中间产物（扁平 6 元组行）。

    与 `m1_i_rust_engine._level_bsps` 逐字同链，仅把 divs 一并返回并按
    `_scan_div_events` 输入格式扁平化（surfacing 而非新计算——divergences
    本就是 BSP 的中间产物）。返回 (bsps, div_rows)。
    """
    seg_in = [(m[0][1], m[1][0], m[1][1], m[1][2], m[1][3]) for m in prev_moves]
    zs5 = [(z[0], z[1], z[2], z[3], z[5]) for z in level_zhongshus]
    mv_in = [m[0] for m in level_moves]
    divs = R.divergences_from_moves_v1(seg_in, zs5, mv_in, level_id)
    zs7 = [(z[0], z[1], z[2], z[3], z[5], z[7], z[6]) for z in level_zhongshus]
    div_in = [(d[0][0], d[0][1], d[0][7], d[0][5], d[0][6], d[1][0], d[1][1])
              for d in divs]
    bsps = R.buysellpoints_from_level(seg_in, zs7, mv_in, div_in, level_id)
    div_rows: list = []
    for d in divs:
        idx = d[0][6]
        if idx < len(seg_in):
            price = seg_in[idx][1] if d[0][1] == "top" else seg_in[idx][2]
        else:
            price = 0.0
        div_rows.append((d[0][0], d[0][1], idx, d[1][0], d[1][1], price))
    return bsps, div_rows


# ════════════════════════════════════════════════════════════
# 信号层主函数
# ════════════════════════════════════════════════════════════

def compute_organic_signals(
    opens: list[float], highs: list[float], lows: list[float], closes: list[float],
    dir_flips: list | None = None,
) -> list[BarSignalI]:
    """Rust 引擎驱动的 I 磁带 + bsp_events + div_events + up_move_settled。

    结构承自 interval_nesting 的 `compute_i_signals_rust_events`（PH 门控 / epoch
    门控 / 递归层 diff 门控逐字一致），布尔流与 Rust delta 接口逐重算点差分守卫。
    背驰来源全部为增量缓存直读（见模块 docstring）。

    dir_flips（v2 D3 方向行，可选收集器；None=零行为变化，磁带逐位不变）：
    传入 list 时，同一引擎 pass 内追加方向翻转行 (bar, ladder, "up"/"down")：
      ladder1（笔）：confirmed 笔 p1 差分符号（笔严格交替，2nd 笔起精确）；
      ladder2（笔中枢）：`bi_zhongshu_last_move_dir`（moves 尾元素只读 surfacing，
        stroke 增长 bar 门控——与本层事件流同步点）；
      ladder3（走势级）：`trend_last_move_dir`（move_epoch 门控 O(1)）；
      ladder≥4（递归层）：current_recursive mvs 尾元素方向（cache-diff 门控——
        尾 move 方向变化 ⊆ (zhs, mvs) cache 变化，门控完备）。
    锚语义声明：翻转 bar = 方向的**信号观测时点**（确认滞后与全系统事件时间
    口径一致）；run_anchor 由消费方（Rust runner）从翻转 bar 导出。
    """
    n = len(closes)
    orch = R.RecursiveOrchestrator(max_levels=MAX_LEVELS)

    bar_dn = PHLevelState.make(top2_only=True); bar_up = PHLevelState.make(top2_only=True)
    bi_dn = PHLevelState.make(top2_only=True); bi_up = PHLevelState.make(top2_only=True)
    last_stroke_n = 0

    seg_seen: set = set()        # ladder2 BSP 事件去重（键含 confirmed）
    trend_seen: set = set()      # ladder3
    level_seen: dict[int, set] = {}
    div_seen: dict[int, set] = {}        # ladder → 背驰事件去重
    settled_seen: dict[int, set] = {}    # ladder → settled move 身份键

    max_ladder = LADDER_SEG
    level_cache: dict[int, tuple] = {}
    last_rec_epoch: int = -1
    last_trend_epoch: int = -1

    # ── D3 方向行收集状态（dir_flips 非 None 时启用）──
    d3 = dir_flips is not None
    last_dir: list = [None] * MAX_LADDER     # ladder → 最后已知方向
    prev_p1: float | None = None             # ladder1 笔 p1 差分
    last_move_epoch_d3: int = -1

    def _d3_flip(bar: int, ladder: int, dirn: str | None) -> None:
        if dirn is not None and dirn != last_dir[ladder]:
            last_dir[ladder] = dirn
            dir_flips.append((bar, ladder, dirn))

    i_signals: list[BarSignalI] = []
    last_progress = 0

    for i in range(n):
        c = closes[i]
        orch.process_bar(opens[i], highs[i], lows[i], c)
        ev_by_ladder: dict[int, list] = {}    # 级别隔离：每 ladder 独立事件流
        div_by_ladder: dict[int, list] = {}
        up_settled = [False] * MAX_LADDER

        # ── ladder0 bar：close PH（每 bar）──
        bar_buy_r1, bar_buy_nr1 = bar_dn.detect_settle(bar_dn.tree.update(c))
        bar_sell_r1, bar_sell_nr1 = bar_up.detect_settle(bar_up.tree.update(-c))
        bar_buy = bar_buy_r1 or bar_buy_nr1
        bar_sell = bar_sell_r1 or bar_sell_nr1

        # ── ladder1 bi PH + ladder2 笔中枢事件/背驰：仅 stroke 计数增长时 ──
        bi_buy = False; bi_sell = False
        seg_buy1 = seg_sell1 = seg_sell_any = seg_buy_any = False
        sc = orch.stroke_count()
        if sc > last_stroke_n:
            d1: str | None = None
            for p1 in orch.strokes_p1_since(last_stroke_n):
                if d3:
                    if prev_p1 is not None:
                        d1 = "up" if p1 > prev_p1 else "down"
                    prev_p1 = p1
                if p1 > 0:
                    r1d, nr1d = bi_dn.detect_settle(bi_dn.tree.update(p1))
                    r1u, nr1u = bi_up.detect_settle(bi_up.tree.update(-p1))
                    if r1d or nr1d:
                        bi_buy = True
                    if r1u or nr1u:
                        bi_sell = True
            last_stroke_n = sc
            if d3:
                _d3_flip(i, LADDER_BI, d1)
                _d3_flip(i, LADDER_SEG,
                         orch.bi_zhongshu_last_move_dir(BI_ZHONGSHU_LEVEL_ID))
            # BSP 事件流：增量引擎缓存全量 marshal（O(B)/次——事件流的必要
            # 代价，interval_nesting 既有声明）。
            bsps2 = orch.current_bi_zhongshu_buysellpoints_inc(BI_ZHONGSHU_LEVEL_ID)
            seg_buy1, seg_sell1, seg_sell_any, seg_buy_any, evs2 = \
                _scan_events_rust(bsps2, seg_seen)
            if evs2:
                ev_by_ladder[LADDER_SEG] = evs2
            # 背驰事件流：增量引擎 divs 层缓存直读（O(n_div)/次，零重算）
            dl2 = _scan_div_events(
                orch.current_bi_zhongshu_divergences_inc(BI_ZHONGSHU_LEVEL_ID),
                div_seen.setdefault(LADDER_SEG, set()))
            if dl2:
                div_by_ladder[LADDER_SEG] = dl2
            # 差分守卫：布尔导出必须与 Rust delta 接口（旧语义逐位移植）一致
            rb = orch.bi_zhongshu_new_signals(BI_ZHONGSHU_LEVEL_ID)
            if rb != (seg_buy1, seg_sell1, seg_sell_any, seg_buy_any):
                raise RuntimeError(
                    f"差分守卫失败@bar{i} ladder2: events布尔="
                    f"{(seg_buy1, seg_sell1, seg_sell_any, seg_buy_any)} rust={rb}")

        # ── ladder3 走势级：bsp_epoch 门控 marshal + 事件/背驰扫描 + 差分守卫 ──
        l1_buy1 = l1_sell1 = l1_sell_any = l1_buy_any = False
        epoch = orch.bsp_epoch()
        if epoch != last_trend_epoch:
            last_trend_epoch = epoch
            l1_buy1, l1_sell1, l1_sell_any, l1_buy_any, evs3 = \
                _scan_events_rust(orch.current_buysellpoints(), trend_seen)
            if evs3:
                ev_by_ladder[LADDER_MOVE] = evs3
            # 背驰：inc_seg_div 增量缓存直读（与引擎内部 BSP 同源同步点）
            dl3 = _scan_div_events(
                orch.current_trend_divergences(),
                div_seen.setdefault(LADDER_MOVE, set()))
            if dl3:
                div_by_ladder[LADDER_MOVE] = dl3
        # D3 ladder3：move_epoch 门控的尾 move 方向读数（O(1)）
        if d3:
            me = orch.move_epoch()
            if me != last_move_epoch_d3:
                last_move_epoch_d3 = me
                _d3_flip(i, LADDER_MOVE, orch.trend_last_move_dir())
        # 走势级 move settle（O(1) delta 接口；move_epoch 未变返回空）
        settled_mvs = orch.take_move_settle_events()
        if settled_mvs:
            sseen3 = settled_seen.setdefault(LADDER_MOVE, set())
            up_settled[LADDER_MOVE] = _new_settled_up_moves(
                [m[0] for m in settled_mvs], sseen3)
        rt = orch.trend_new_signals(BI_ZHONGSHU_LEVEL_ID)
        type2_buy = rt[4]
        if rt[:4] != (l1_buy1, l1_sell1, l1_sell_any, l1_buy_any):
            raise RuntimeError(
                f"差分守卫失败@bar{i} ladder3: events布尔="
                f"{(l1_buy1, l1_sell1, l1_sell_any, l1_buy_any)} rust={rt[:4]}")

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

        # ── 递归层（ladder≥4）：recursive_epoch 门控 + 状态 diff 门控 ──
        rec_epoch = orch.recursive_epoch()
        if rec_epoch != last_rec_epoch:
            last_rec_epoch = rec_epoch
            recursive = orch.current_recursive()
            level_moves_map = {lid: mvs for (lid, _z, mvs) in recursive}
            moves_l1 = None
            for (lid, zhs, mvs) in recursive:
                ladder = lid + 2
                if ladder >= MAX_LADDER:
                    continue
                if ladder > max_ladder:
                    max_ladder = ladder
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
                bsps_l, div_rows_l = _level_bsps_with_divs(prev, zhs, mvs, lid)
                b1, s1, sa, ba, evl = _scan_events_rust(bsps_l, seen)
                buy1[ladder] = b1; sell1[ladder] = s1
                sell_any[ladder] = sa; buy_any[ladder] = ba
                if evl:
                    ev_by_ladder[ladder] = evl
                dl = _scan_div_events(
                    div_rows_l, div_seen.setdefault(ladder, set()))
                if dl:
                    div_by_ladder[ladder] = dl
                sseen_l = settled_seen.setdefault(ladder, set())
                up_settled[ladder] = _new_settled_up_moves(
                    [m[0] for m in mvs], sseen_l)
                # D3 递归层：尾 move 方向（cache-diff 门控——方向变化 ⊆ cache 变化）
                if d3 and mvs:
                    _d3_flip(i, ladder, mvs[-1][0][1])

        if ev_by_ladder:
            rows = [()] * MAX_LADDER
            for lad, evl in ev_by_ladder.items():
                rows[lad] = tuple(evl)
            bsp_events = tuple(rows)
        else:
            bsp_events = NO_LADDER_EVENTS
        if div_by_ladder:
            drows = [()] * MAX_LADDER
            for lad, dvl in div_by_ladder.items():
                drows[lad] = tuple(dvl)
            div_events = tuple(drows)
        else:
            div_events = NO_LADDER_DIVS
        ums = tuple(up_settled) if any(up_settled) else NO_UP_SETTLED
        i_signals.append(BarSignalI(
            close=c, buy1=tuple(buy1), sell1=tuple(sell1),
            sell_any=tuple(sell_any), buy_any=tuple(buy_any),
            max_ladder=max_ladder, type2_buy=type2_buy,
            bsp_events=bsp_events, div_events=div_events,
            up_move_settled=ums))

        if i - last_progress >= 200_000:
            print(f"    [{i / n * 100:5.1f}%] organic signal bar {i:,}/{n:,}",
                  flush=True)
            last_progress = i

    return i_signals


# 兼容别名：interval_nesting_reverse_backtest 原函数名（磁带超集，
# 旧消费路径 bsp_events/布尔逐位不变）。
compute_i_signals_rust_events = compute_organic_signals
