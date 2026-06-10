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

布尔流 / bsp_events 与原实现**逐位等价**（差分守卫内嵌：每个重算点把布尔导出与
Rust delta 接口对比，不等 → RuntimeError）。新字段默认不被旧消费者读取
（run_version_i 的 P1-P7 逐位不变）。

═══════════════════════════════════════════════════════════════════════
各 ladder 背驰来源（Rust 引擎零改动约束下的严格形式）
═══════════════════════════════════════════════════════════════════════
  ladder2（笔中枢）：引擎增量 BSP 接口不暴露中间 divergences → 在 stroke 增长 bar
    用 Rust 纯函数全链重算（confirmed 笔 → zhongshu_from_strokes →
    moves_from_zhongshus → divergences_from_moves_v1 → buysellpoints_from_level），
    与 lib.rs `current_bi_zhongshu_buysellpoints` 逐字同链 → BSP 输出逐位等价
    （布尔差分守卫复核）。BSP 事件流改由本链产出（替代原 `_inc` 全量 marshal）。
  ladder3（走势级）：bsp_epoch 门控点上用 current_segments/zhongshus/moves
    marshal + divergences_from_moves_v1(level_id=1) 重算——与引擎内部
    compute_bsps 的背驰输入逐字一致（prev_segments 全量 / inc_seg_zs.zhongshus /
    prev_moves / macd_ctx=None）。
  ladder≥4（递归层）：divergences 本就是 `_level_bsps` 的中间产物，surfacing
    而非新计算（设计 §5.1，增量成本≈0）。

⚠ 成本声明（formalization-validity-domain / 既有 O(S·B) 声明的延伸）：
  ladder2 背驰链需每个 stroke 增长 bar marshal `current_strokes()`（O(S)/次，
  摊还 O(S²)）+ 纯函数全链调用。OKLO 447K 实测单次终态 ~16ms（marshal 主导），
  摊还 ~5-6 分钟/447K。这是"盘整背驰可见性"在引擎零改动约束下的必要代价——
  增量背驰引擎在 Rust 内部存在但不暴露中间产物。BRN 2.4M 上预期 ~40-60 分钟。
  注：confirmed 笔 append-only（引擎 inc 调用契约），zs_in/seg_in 增量维护，
  消除每次 O(S) 的 Python 重构造；marshal 本身不可增量（无窗口接口）。

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
# DivergenceTuple 事件扫描器（§5.1 div_events）
# ════════════════════════════════════════════════════════════

def _scan_div_events(divs: list, seg_high_low, seen: set) -> list:
    """DivergenceTuple 列表 → 新背驰事件流（per-ladder seen-set 去重）。

    DivergenceTuple = ((kind, direction, level_id, seg_a_start, seg_a_end,
                        seg_c_start, seg_c_end, center_idx),
                       (force_a, force_c, confirmed, dif_peak_a, dif_peak_c,
                        hist_peak_a, hist_peak_c))
    事件 = (kind, direction, side, seg_idx, force_a, force_c, price)：
      direction 映射：引擎 "top"（向上段衰竭）→ "up"/side="sell"；
                      "bottom" → "down"/side="buy"。
      seg_idx = seg_c_end（背驰段锚）；去重键 = (kind, 引擎direction, seg_c_end)。
      price = 背驰段端点价（sell→段 high / buy→段 low，与 type1 BSP price 同构；
      seg_high_low(idx) -> (high, low) 由调用方按该层 segments 序列提供）。
    """
    out: list = []
    for d in divs:
        head = d[0]
        kind, ddir, seg_c_end = head[0], head[1], head[6]
        key = (kind, ddir, seg_c_end)
        if key in seen:
            continue
        seen.add(key)
        if ddir == "top":
            direction, side = "up", "sell"
        else:
            direction, side = "down", "buy"
        hl = seg_high_low(seg_c_end)
        price = (hl[0] if side == "sell" else hl[1]) if hl is not None else 0.0
        out.append((kind, direction, side, seg_c_end, d[1][0], d[1][1], price))
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
                          level_moves: list, level_id: int) -> tuple[list, list, list]:
    """递归层 N≥2 confirmed BSP + 背驰中间产物 + segments 输入。

    与 `m1_i_rust_engine._level_bsps` 逐字同链，仅把 divs/seg_in 一并返回
    （surfacing 而非新计算——divergences 本就是 BSP 的中间产物）。
    返回 (bsps, divs, seg_in)。
    """
    seg_in = [(m[0][1], m[1][0], m[1][1], m[1][2], m[1][3]) for m in prev_moves]
    zs5 = [(z[0], z[1], z[2], z[3], z[5]) for z in level_zhongshus]
    mv_in = [m[0] for m in level_moves]
    divs = R.divergences_from_moves_v1(seg_in, zs5, mv_in, level_id)
    zs7 = [(z[0], z[1], z[2], z[3], z[5], z[7], z[6]) for z in level_zhongshus]
    div_in = [(d[0][0], d[0][1], d[0][7], d[0][5], d[0][6], d[1][0], d[1][1])
              for d in divs]
    bsps = R.buysellpoints_from_level(seg_in, zs7, mv_in, div_in, level_id)
    return bsps, divs, seg_in


# ════════════════════════════════════════════════════════════
# 信号层主函数
# ════════════════════════════════════════════════════════════

def compute_organic_signals(
    opens: list[float], highs: list[float], lows: list[float], closes: list[float],
) -> list[BarSignalI]:
    """Rust 引擎驱动的 I 磁带 + bsp_events + div_events + up_move_settled。

    结构承自 interval_nesting 的 `compute_i_signals_rust_events`（PH 门控 / epoch
    门控 / 递归层 diff 门控逐字一致），布尔流与 Rust delta 接口逐重算点差分守卫。
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

    # ladder2 背驰链的增量输入（confirmed 笔 append-only，前缀冻结）：
    # zs_in2 = (i0, i1, high, low, True)；seg_in2 = (direction, high, low, i0, i1)。
    zs_in2: list = []
    seg_in2: list = []

    max_ladder = LADDER_SEG
    level_cache: dict[int, tuple] = {}
    last_rec_epoch: int = -1
    last_trend_epoch: int = -1

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

        # ── ladder1 bi PH + ladder2 笔中枢链：仅 stroke 计数增长时 ──
        bi_buy = False; bi_sell = False
        seg_buy1 = seg_sell1 = seg_sell_any = seg_buy_any = False
        sc = orch.stroke_count()
        if sc > last_stroke_n:
            for p1 in orch.strokes_p1_since(last_stroke_n):
                if p1 > 0:
                    r1d, nr1d = bi_dn.detect_settle(bi_dn.tree.update(p1))
                    r1u, nr1u = bi_up.detect_settle(bi_up.tree.update(-p1))
                    if r1d or nr1d:
                        bi_buy = True
                    if r1u or nr1u:
                        bi_sell = True
            last_stroke_n = sc
            # ── 笔中枢全链重算（与 lib.rs current_bi_zhongshu_buysellpoints 逐字同链）──
            # confirmed 笔 append-only：只追加新增（marshal O(S) 不可避免，构造 O(Δ)）。
            strokes = orch.current_strokes()
            for s in strokes[len(zs_in2):]:
                if s[7]:
                    zs_in2.append((s[0], s[1], s[3], s[4], True))
                    seg_in2.append((s[2], s[3], s[4], s[0], s[1]))
            # 运行时守卫（增量追加依赖的引擎不变量）：confirmed 笔是 strokes
            # 的稳定前缀（仅尾部 unconfirmed）。若不变量破坏，从 len(zs_in2)
            # 起的尾扫会静默跳过中间新 confirmed 笔 → 此处全量计数复核。
            n_confirmed = sum(1 for s in strokes if s[7])
            if len(zs_in2) != n_confirmed:
                raise RuntimeError(
                    f"confirmed 笔前缀不变量破坏@bar{i}: 增量缓存 "
                    f"{len(zs_in2)} ≠ 全量计数 {n_confirmed}")
            if len(zs_in2) >= 3:
                zss2 = R.zhongshu_from_strokes(zs_in2)
                mvs2 = R.moves_from_zhongshus(zss2, len(zs_in2))
                mv_in2 = [m[0] for m in mvs2]
                zs5 = [(z[0], z[1], z[2], z[3], z[5]) for z in zss2]
                divs2 = R.divergences_from_moves_v1(
                    seg_in2, zs5, mv_in2, BI_ZHONGSHU_LEVEL_ID)
                zs7 = [(z[0], z[1], z[2], z[3], z[5], z[7], z[6]) for z in zss2]
                div_in2 = [(d[0][0], d[0][1], d[0][7], d[0][5], d[0][6],
                            d[1][0], d[1][1]) for d in divs2]
                bsps2 = R.buysellpoints_from_level(
                    seg_in2, zs7, mv_in2, div_in2, BI_ZHONGSHU_LEVEL_ID)
                seg_buy1, seg_sell1, seg_sell_any, seg_buy_any, evs2 = \
                    _scan_events_rust(bsps2, seg_seen)
                if evs2:
                    ev_by_ladder[LADDER_SEG] = evs2
                dseen = div_seen.setdefault(LADDER_SEG, set())
                dl2 = _scan_div_events(
                    divs2,
                    lambda k, _s=seg_in2: ((_s[k][1], _s[k][2])
                                           if k < len(_s) else None),
                    dseen)
                if dl2:
                    div_by_ladder[LADDER_SEG] = dl2
                sseen = settled_seen.setdefault(LADDER_SEG, set())
                up_settled[LADDER_SEG] = _new_settled_up_moves(mv_in2, sseen)
            # 差分守卫：布尔导出必须与 Rust delta 接口（增量引擎，旧语义）一致——
            # 这同时守卫"Python 组合链 ≡ Rust 增量链"的逐位等价。
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
            # 背驰重算：与引擎内部 compute_bsps 的输入逐字一致
            # （segments 全量 / inc 中枢 / prev_moves / level_id=1 / macd_ctx=None）。
            segs3 = orch.current_segments()
            zss3 = orch.current_zhongshus()
            mvs3 = orch.current_moves()
            seg_in3 = [(t[0][4], t[0][5], t[0][6], t[0][2], t[0][3]) for t in segs3]
            zs5_3 = [(z[0], z[1], z[2], z[3], z[5]) for z in zss3]
            mv_in3 = [m[0] for m in mvs3]
            divs3 = R.divergences_from_moves_v1(seg_in3, zs5_3, mv_in3, 1)
            dseen3 = div_seen.setdefault(LADDER_MOVE, set())
            dl3 = _scan_div_events(
                divs3,
                lambda k, _s=seg_in3: ((_s[k][1], _s[k][2])
                                       if k < len(_s) else None),
                dseen3)
            if dl3:
                div_by_ladder[LADDER_MOVE] = dl3
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
                bsps_l, divs_l, seg_in_l = _level_bsps_with_divs(prev, zhs, mvs, lid)
                b1, s1, sa, ba, evl = _scan_events_rust(bsps_l, seen)
                buy1[ladder] = b1; sell1[ladder] = s1
                sell_any[ladder] = sa; buy_any[ladder] = ba
                if evl:
                    ev_by_ladder[ladder] = evl
                dseen_l = div_seen.setdefault(ladder, set())
                dl = _scan_div_events(
                    divs_l,
                    lambda k, _si=seg_in_l: ((_si[k][1], _si[k][2])
                                             if k < len(_si) else None),
                    dseen_l)
                if dl:
                    div_by_ladder[ladder] = dl
                sseen_l = settled_seen.setdefault(ladder, set())
                up_settled[ladder] = _new_settled_up_moves(
                    [m[0] for m in mvs], sseen_l)

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

        if i - last_progress >= 100_000:
            print(f"    [{i / n * 100:5.1f}%] organic signal bar {i:,}/{n:,}",
                  flush=True)
            last_progress = i

    return i_signals


# 兼容别名：interval_nesting_reverse_backtest 原函数名（磁带超集，
# 旧消费路径 bsp_events/布尔逐位不变）。
compute_i_signals_rust_events = compute_organic_signals
