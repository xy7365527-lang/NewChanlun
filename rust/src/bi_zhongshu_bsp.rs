//! bi_zhongshu_bsp.rs — 笔中枢买卖点**全链增量引擎**。
//!
//! 组合四层增量器，bit-exact 等价于逐前缀全量
//! `confirmed 笔 → zhongshu_from_strokes → moves_from_zhongshus →
//!  divergences_from_moves_v1(None) → buysellpoints_from_level`，
//! 但摊还成本远低于全量重算的 O(strokes²)。
//!
//! ## 层与复杂度
//! | 层 | 增量器 | 笔级成本（消除前→后） |
//! |----|--------|----------------------|
//! | 中枢 | `IncrementalBiZhongshu` | O(S) scan → 摊还 O(ΔS) |
//! | 走势 | `IncrementalMoves` | O(Z) greedy+gen → 摊还 O(Δ) |
//! | 背驰 | `IncrementalDivergences` | O(S) 段遍历 → 摊还 O(尾部段) |
//! | 买卖点 | `IncrementalBsp` | O(S) lookup+detect → 摊还 O(window) |
//!
//! ## bit-exact 契约
//! `current() == buysellpoints_from_level(confirmed 笔视图, 笔中枢, ...)`，
//! 由 `differential_tests` 逐前缀（合成 LCG + 真实数据 fixture）守卫。
//!
//! ## append-only 前提
//! 仅 push **永久固定**的 confirmed 笔（调用方保证：BiEngine 的 cp_strokes 前缀，
//! 最后一笔 unconfirmed 不传入）。settled 中枢恒为 zhongshus[..stable_count] 前缀，
//! 上层稳定性由此导出（见各层 docstring）。

use std::collections::HashSet;

use crate::buysellpoint::{BspKind, BuySellPoint, IncrementalBsp, Side};
use crate::divergence::{Divergence, IncrementalDivergences, MoveView, SegView, ZsView};
use crate::moves::IncrementalMoves;
use crate::stroke::Direction;
use crate::zhongshu::{BreakDir, IncrementalBiZhongshu};

/// 笔中枢买卖点全链增量引擎。
#[derive(Debug, Clone)]
pub struct IncrementalBiZhongshuBsp {
    level_id: i64,
    /// confirmed 笔的 SegView（append-only；笔原生暴露 direction/high/low/i0/i1）。
    segs: Vec<SegView>,
    zhongshu: IncrementalBiZhongshu,
    moves: IncrementalMoves,
    divs: IncrementalDivergences,
    bsp: IncrementalBsp,
    /// moves 的 MoveView 镜像（div/bsp 入参；与 moves.current() 同步增量维护）。
    move_views: Vec<MoveView>,
    /// 上次并入 move_views 的 closed move 数（增量拼接锚）。
    prev_closed_views: usize,
    /// 中枢 ZsView 镜像（div/bsp 入参；增量维护，避免每次 O(Z) 重建）。
    zviews: Vec<ZsView>,
    /// 中枢 break 视图镜像（bsp type3 入参；同 zviews 增量维护）。
    zbreak: Vec<(bool, BreakDir, i64)>,
    /// 上次镜像的 settled 中枢前缀（zviews/zbreak 的固定锚）。
    prev_zs_stable: usize,
    /// 信号去重集合（已触发的 confirmed bsp key，复刻调用方 _scan_new 的 seen）。
    seen: HashSet<(BspKind, Side, i64)>,
    /// 信号扫描锚（seg_idx < signal_anchor 的 bsp confirmed 终态已扫，不重扫）。
    signal_anchor: i64,
    /// 事件流去重集合（复刻调用方 _scan_events_rust 的 seen；键含 confirmed——
    /// candidate/confirmed 分别入流一次）。
    events_seen: HashSet<(BspKind, Side, i64, bool)>,
    /// 事件流扫描锚（语义同 signal_anchor；独立推进——两接口调用时序解耦）。
    events_anchor: i64,
    /// 背驰事件流去重集合（复刻调用方 _scan_div_events 的 seen）。
    div_seen: HashSet<(crate::divergence::DivKind, crate::divergence::DivDir, i64)>,
    /// 背驰事件流扫描锚（= 上次 divs.stable_len()，closed 前缀冻结边界）。
    div_anchor: usize,
}

impl IncrementalBiZhongshuBsp {
    pub fn new(level_id: i64) -> Self {
        IncrementalBiZhongshuBsp {
            level_id,
            segs: Vec::new(),
            zhongshu: IncrementalBiZhongshu::new(),
            moves: IncrementalMoves::new(),
            divs: IncrementalDivergences::new(),
            bsp: IncrementalBsp::new(level_id, false), // 笔级全 confirmed ⟹ settle 门无效
            move_views: Vec::new(),
            prev_closed_views: 0,
            zviews: Vec::new(),
            zbreak: Vec::new(),
            prev_zs_stable: 0,
            seen: HashSet::new(),
            signal_anchor: 0,
            events_seen: HashSet::new(),
            events_anchor: 0,
            div_seen: HashSet::new(),
            div_anchor: 0,
        }
    }

    /// 追加一根**永久固定**的 confirmed 笔，增量推进全链。
    ///
    /// `(i0, i1, high, low, direction)`：笔的 merged 区间端点 + 极值 + 方向。
    pub fn push_confirmed_stroke(
        &mut self,
        i0: usize,
        i1: usize,
        high: f64,
        low: f64,
        direction: Direction,
    ) {
        // 1. 笔 SegView（append-only）。笔此处全 confirmed ⟹ settled=true（settle 门无效）。
        self.segs.push(SegView {
            direction,
            high,
            low,
            i0,
            i1,
            settled: true,
        });
        // 2. 笔中枢增量（anchor=i0/i1，复刻 zhongshu_from_strokes）。
        self.zhongshu.push_confirmed(high, low, i0, i1);
        let zss = self.zhongshu.current();
        let num_segments = self.segs.len();

        // 3. 走势增量。
        self.moves.update(zss, num_segments);

        // 4. 同步 move_views 镜像（增量：truncate 旧 pending → extend 新 closed → push pending）。
        let full = self.moves.current();
        let n_closed = full.len().saturating_sub(1);
        self.move_views.truncate(self.prev_closed_views);
        for m in &full[self.prev_closed_views..n_closed] {
            self.move_views.push(MoveView {
                kind: m.kind,
                direction: m.direction,
                seg_start: m.seg_start,
                seg_end: m.seg_end,
                zs_start: m.zs_start,
                zs_end: m.zs_end,
                zs_count: m.zs_count,
                settled: m.settled,
            });
        }
        self.prev_closed_views = n_closed;
        if let Some(m) = full.last() {
            self.move_views.push(MoveView {
                kind: m.kind,
                direction: m.direction,
                seg_start: m.seg_start,
                seg_end: m.seg_end,
                zs_start: m.zs_start,
                zs_end: m.zs_end,
                zs_count: m.zs_count,
                settled: m.settled,
            });
        }

        // 5. 中枢视图 / break 视图增量镜像（settled 前缀固定 → 只重建 [prev_zs_stable..]）。
        let zs_stable = self.zhongshu.stable_count();
        self.zviews.truncate(self.prev_zs_stable);
        self.zbreak.truncate(self.prev_zs_stable);
        for z in &zss[self.prev_zs_stable..] {
            self.zviews.push(ZsView {
                zd: z.zd,
                zg: z.zg,
                seg_start: z.seg_start,
                seg_end: z.seg_end,
                settled: z.settled,
            });
            self.zbreak.push((z.settled, z.break_direction, z.break_seg));
        }
        self.prev_zs_stable = zs_stable;
        let zviews = &self.zviews;
        let zbreak = &self.zbreak;

        // 6. 背驰增量（closed move / pending 分离）。
        let n = self.move_views.len();
        let (closed_mvs, pending_mv): (&[MoveView], Option<&MoveView>) = if n == 0 {
            (&[], None)
        } else {
            (&self.move_views[..n - 1], Some(&self.move_views[n - 1]))
        };
        self.divs
            .update(closed_mvs, pending_mv, &self.segs, &zviews, self.level_id);

        // 7. 买卖点增量（divs_stable_len = closed div 数，frontier 推进边界）。
        self.bsp.update(
            &self.segs,
            &zviews,
            &zbreak,
            &self.move_views,
            self.divs.current(),
            self.divs.stable_len(),
        );
    }

    /// 当前全量笔中枢买卖点（逐位等价于全量 `buysellpoints_from_level`）。
    pub fn current(&self) -> &[BuySellPoint] {
        self.bsp.current()
    }

    /// 当前全量背驰（divs 层增量缓存直读；逐位等价于全量
    /// `divergences_from_moves_v1(.., None)`——见 IncrementalDivergences 文档）。
    ///
    /// 只读 surfacing：背驰本就是 BSP 链的中间产物（步骤 6），此前算完即弃。
    /// 盘整背驰（kind=Consolidation）不构造任何 BSP → 不经此接口对调用方不可见。
    pub fn divergences(&self) -> &[Divergence] {
        self.divs.current()
    }

    /// confirmed 笔 SegView 序列（append-only；背驰段端点价锚定用）。
    pub fn stroke_views(&self) -> &[SegView] {
        &self.segs
    }

    /// 笔级走势尾元素方向（有机赋格 v2 §7 D3 dir_row[2] 镜像读数）。
    ///
    /// 只读 surfacing：moves 层增量缓存的尾元素直读，不触碰任何计算路径
    /// （divergences 接口同先例）。无走势 → None（"无 move 时最后段/笔"的
    /// 笔侧 fallback 由调用方在 ladder1 行自行提供）。
    pub fn last_move_direction(&self) -> Option<crate::stroke::Direction> {
        self.moves.current().last().map(|m| m.direction)
    }

    /// 笔级走势尾元素 kind（38课循环 voice 趋势态行 trend_row[2]；2026-06-11）。
    ///
    /// 只读 surfacing 同 `last_move_direction`。趋势态 = 尾 move kind==Trend
    /// （≥2 个同向中枢，17课趋势定义的 move 层读数）。无走势 → None。
    pub fn last_move_kind(&self) -> Option<crate::moves::MoveKind> {
        self.moves.current().last().map(|m| m.kind)
    }

    /// **delta 信号接口**——返回自上次以来新触发的 confirmed 买卖点信号，O(window) 摊还。
    ///
    /// 逐位等价于调用方 `_scan_new(current(), seg_seen)`：遍历 bsp，对 confirmed 且
    /// (kind,side,seg_idx) 未见的，更新 seen 并置信号位。返回 (buy1, sell1, sell_any, buy_any)，
    /// 其中 `*1` 为 type1 信号。**消除 marshal O(B) + 调用方扫描 O(B)** → 端到端 O(N)。
    ///
    /// ## O(window) 原理
    /// seg_idx < signal_anchor（已 finalize）的 bsp confirmed 终态已扫且 seen 记录；
    /// 只重扫 [signal_anchor, end] 尾部（confirmed 可能翻转区）。signal_anchor 单调推进到
    /// bsp 层 finalize 边界 `stable_anchor`（finalize ⟹ confirmed 永久固定）。
    pub fn take_new_signals(&mut self) -> (bool, bool, bool, bool) {
        let bsps = self.bsp.current();
        let (mut b1, mut s1, mut sa, mut ba) = (false, false, false, false);
        // 后缀起点：第一个 seg_idx >= signal_anchor（bsps 按 seg_idx 升序）。
        let start = bsps.partition_point(|bp| bp.seg_idx < self.signal_anchor);
        for bp in &bsps[start..] {
            if !bp.confirmed {
                continue;
            }
            let key = (bp.kind, bp.side, bp.seg_idx);
            if self.seen.contains(&key) {
                continue;
            }
            self.seen.insert(key);
            match bp.side {
                Side::Buy => {
                    ba = true;
                    if bp.kind == BspKind::Type1 {
                        b1 = true;
                    }
                }
                Side::Sell => {
                    sa = true;
                    if bp.kind == BspKind::Type1 {
                        s1 = true;
                    }
                }
            }
        }
        // finalize 边界之前 confirmed 终态确定 → 推进信号锚，下次不重扫。
        self.signal_anchor = self.bsp.stable_anchor();
        (b1, s1, sa, ba)
    }

    /// **事件流 delta 接口**——返回 (buy1, sell1, sell_any, buy_any, 新事件行)，O(window) 摊还。
    ///
    /// 逐位等价于调用方 `_scan_events_rust(全量 marshal, seg_seen)`（organic_signals）：
    /// 去重键 (kind, side, seg_idx, confirmed)——candidate/confirmed 分别入流一次；
    /// 事件行 = (kind, side, seg_idx, confirmed, center_seg_start, center_zd, center_zg, price)；
    /// 布尔仅由新 confirmed 事件置位。消除每-stroke-bar 全量 marshal O(B) + 调用方
    /// 扫描 O(B) 的 O(S×B) 项（期货长序列主导项，BRN 1.2M profile 实测 ~95% wall）。
    ///
    /// ## 尾窗等价性
    /// seg_idx < events_anchor 的 bsp 行已 finalize（stable_anchor 之前字段冻结）⟹
    /// 其曾出现过的全部 (kind,side,seg_idx,confirmed) 键均已在历次调用入 seen ⟹
    /// 全量重扫必然全部命中 seen（零新事件）。bsps 按 seg_idx 升序 → 尾窗输出顺序
    /// 与全量扫描的新事件子序列逐位一致。
    #[allow(clippy::type_complexity)]
    pub fn take_new_events(
        &mut self,
    ) -> (
        bool,
        bool,
        bool,
        bool,
        Vec<(&'static str, &'static str, i64, bool, Option<usize>, f64, f64, f64)>,
    ) {
        let bsps = self.bsp.current();
        let (mut b1, mut s1, mut sa, mut ba) = (false, false, false, false);
        let mut events = Vec::new();
        let start = bsps.partition_point(|bp| bp.seg_idx < self.events_anchor);
        for bp in &bsps[start..] {
            let key = (bp.kind, bp.side, bp.seg_idx, bp.confirmed);
            if self.events_seen.contains(&key) {
                continue;
            }
            self.events_seen.insert(key);
            events.push((
                bp.kind.as_str(),
                bp.side.as_str(),
                bp.seg_idx,
                bp.confirmed,
                bp.center_seg_start,
                bp.center_zd,
                bp.center_zg,
                bp.price,
            ));
            if !bp.confirmed {
                continue;
            }
            match bp.side {
                Side::Buy => {
                    ba = true;
                    if bp.kind == BspKind::Type1 {
                        b1 = true;
                    }
                }
                Side::Sell => {
                    sa = true;
                    if bp.kind == BspKind::Type1 {
                        s1 = true;
                    }
                }
            }
        }
        self.events_anchor = self.bsp.stable_anchor();
        (b1, s1, sa, ba, events)
    }

    /// **背驰事件流 delta 接口**——返回新背驰行，O(window) 摊还。
    ///
    /// 行格式 = (kind, direction, seg_c_end, force_a, force_c, price)，与
    /// `current_bi_zhongshu_divergences_inc` marshal 同构；去重键
    /// (kind, direction, seg_c_end)（复刻调用方 _scan_div_events 的 seen）。
    /// 尾窗 = [div_anchor..]：divs[..stable_len] 为 closed 冻结前缀（行不可变 ⟹
    /// 其键已在历次调用入 seen），div_anchor 推进到本次 stable_len。
    pub fn take_new_div_rows(
        &mut self,
    ) -> Vec<(&'static str, &'static str, i64, f64, f64, f64)> {
        let divs = self.divs.current();
        let mut out = Vec::new();
        for d in &divs[self.div_anchor.min(divs.len())..] {
            let key = (d.kind, d.direction, d.seg_c_end);
            if self.div_seen.contains(&key) {
                continue;
            }
            self.div_seen.insert(key);
            let idx = d.seg_c_end as usize;
            let price = if idx < self.segs.len() {
                match d.direction {
                    crate::divergence::DivDir::Top => self.segs[idx].high,
                    crate::divergence::DivDir::Bottom => self.segs[idx].low,
                }
            } else {
                0.0
            };
            out.push((
                d.kind.as_str(),
                d.direction.as_str(),
                d.seg_c_end,
                d.force_a,
                d.force_c,
                price,
            ));
        }
        self.div_anchor = self.divs.stable_len();
        out
    }
}

#[cfg(test)]
mod differential_tests {
    use super::*;
    use crate::buysellpoint::buysellpoints_from_level;
    use crate::divergence::divergences_from_moves_v1;
    use crate::moves::moves_from_zhongshus;
    use crate::zhongshu::zhongshu_from_strokes;

    /// 全量参照：从 confirmed 笔重算 bi-zhongshu 买卖点（复刻 lib.rs
    /// current_bi_zhongshu_buysellpoints 的纯 Rust 链）。
    fn full_bsps(
        strokes: &[(usize, usize, f64, f64, Direction)],
        level_id: i64,
    ) -> Vec<BuySellPoint> {
        if strokes.len() < 3 {
            return Vec::new();
        }
        let zs_in: Vec<(usize, usize, f64, f64, bool)> = strokes
            .iter()
            .map(|&(i0, i1, high, low, _)| (i0, i1, high, low, true))
            .collect();
        let zhongshus = zhongshu_from_strokes(&zs_in);
        let moves = moves_from_zhongshus(&zhongshus, Some(strokes.len()));
        let segs: Vec<SegView> = strokes
            .iter()
            .map(|&(i0, i1, high, low, direction)| SegView {
                direction,
                high,
                low,
                i0,
                i1,
                settled: true, // 笔全 confirmed
            })
            .collect();
        let zss: Vec<ZsView> = zhongshus
            .iter()
            .map(|z| ZsView {
                zd: z.zd,
                zg: z.zg,
                seg_start: z.seg_start,
                seg_end: z.seg_end,
                settled: z.settled,
            })
            .collect();
        let mvs: Vec<MoveView> = moves
            .iter()
            .map(|m| MoveView {
                kind: m.kind,
                direction: m.direction,
                seg_start: m.seg_start,
                seg_end: m.seg_end,
                zs_start: m.zs_start,
                zs_end: m.zs_end,
                zs_count: m.zs_count,
                settled: m.settled,
            })
            .collect();
        let divs = divergences_from_moves_v1(&segs, &zss, &mvs, level_id, None);
        let zs_break: Vec<(bool, BreakDir, i64)> = zhongshus
            .iter()
            .map(|z| (z.settled, z.break_direction, z.break_seg))
            .collect();
        buysellpoints_from_level(&segs, &zss, &zs_break, &mvs, &divs, level_id, false)
    }

    fn assert_bsp_eq(inc: &[BuySellPoint], full: &[BuySellPoint], prefix: usize) {
        assert_eq!(
            inc.len(),
            full.len(),
            "前缀 {} bsp 数发散 inc={} full={}",
            prefix,
            inc.len(),
            full.len()
        );
        for (a, b) in inc.iter().zip(full.iter()) {
            assert_eq!(a.kind, b.kind, "前缀{} kind", prefix);
            assert_eq!(a.side, b.side, "前缀{} side", prefix);
            assert_eq!(a.seg_idx, b.seg_idx, "前缀{} seg_idx", prefix);
            assert_eq!(a.move_seg_start, b.move_seg_start, "前缀{} move_seg_start", prefix);
            assert_eq!(a.confirmed, b.confirmed, "前缀{} confirmed", prefix);
            assert_eq!(a.settled, b.settled, "前缀{} settled", prefix);
            assert_eq!(a.overlaps_with, b.overlaps_with, "前缀{} overlap", prefix);
            assert_eq!(a.divergence_key, b.divergence_key, "前缀{} div_key", prefix);
            assert_eq!(a.price.to_bits(), b.price.to_bits(), "前缀{} price", prefix);
            assert_eq!(a.bar_idx, b.bar_idx, "前缀{} bar_idx", prefix);
            assert_eq!(a.center_zd.to_bits(), b.center_zd.to_bits(), "前缀{} zd", prefix);
            assert_eq!(a.center_zg.to_bits(), b.center_zg.to_bits(), "前缀{} zg", prefix);
            assert_eq!(a.center_seg_start, b.center_seg_start, "前缀{} center_seg_start", prefix);
        }
    }

    /// 逐前缀断言增量 == 全量。strokes 已含 direction（依 high/low 单调推断）。
    fn assert_inc_matches_full(strokes: &[(usize, usize, f64, f64, Direction)]) {
        let mut eng = IncrementalBiZhongshuBsp::new(1);
        for (k, &(i0, i1, high, low, dir)) in strokes.iter().enumerate() {
            eng.push_confirmed_stroke(i0, i1, high, low, dir);
            let full = full_bsps(&strokes[..=k], 1);
            assert_bsp_eq(eng.current(), &full, k + 1);
        }
    }

    /// 确定性 LCG 生成 zig-zag 笔（方向交替，端点随机游走），覆盖中枢/趋势/背驰/各类买卖点。
    fn lcg_strokes(n: usize, seed: u64) -> Vec<(usize, usize, f64, f64, Direction)> {
        let mut state = seed;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as f64 / (1u64 << 31) as f64
        };
        let mut out = Vec::new();
        let mut price = 100.0_f64;
        let mut bar = 0usize;
        let mut up = true;
        for _ in 0..n {
            let mag = 1.0 + next() * 8.0;
            let i0 = bar;
            bar += 4 + (next() * 6.0) as usize; // 笔间隔（≥ min_gap）
            let i1 = bar;
            let (high, low, p0, p1);
            if up {
                p0 = price;
                p1 = price + mag;
                high = p1;
                low = p0;
            } else {
                p0 = price;
                p1 = price - mag;
                high = p0;
                low = p1;
            }
            let _ = (p0, p1);
            out.push((i0, i1, high, low, if up { Direction::Up } else { Direction::Down }));
            price = if up { price + mag } else { price - mag };
            up = !up;
        }
        out
    }

    #[test]
    fn diff_lcg_small() {
        let s = lcg_strokes(60, 0x1234_5678_9ABC_DEF0);
        assert_inc_matches_full(&s);
    }

    #[test]
    fn diff_lcg_medium() {
        let s = lcg_strokes(300, 0xDEAD_BEEF_CAFE_1234);
        assert_inc_matches_full(&s);
    }

    #[test]
    fn diff_lcg_seeds() {
        for seed in [1u64, 42, 0xFFFF_FFFF, 0x9E37_79B9_7F4A_7C15] {
            let s = lcg_strokes(200, seed);
            assert_inc_matches_full(&s);
        }
    }
}

