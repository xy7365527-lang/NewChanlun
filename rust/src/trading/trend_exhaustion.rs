//! GUARD-ROLE: organic-fugue-v2-trading-layer——名分：现役（详见 `trading/mod.rs` 头部 GUARD-ROLE 块，#763 C7-E4 核定；零删除/零移入 legacy/）。
//!
//! TrendExhaustion — 41课门的父级别走势衰竭追踪器（P1 双门任务，2026-06-11）。
//!
//! 41课原文："大级别走势没有任何衰竭迹象时参与反向小级别买卖点是刀口舔血"。
//! 判据链（#1232 第三条裁定 a，2026-08-27）：**走势未完成 ∧ 无盘整背驰
//! = 趋势未完 = 不做反向**。走势未完成 = 走势类型延续 / 中枢未死 / 三买卖
//! 未坐实（038 课结构语言；生产结构分类：中枢三态 021:28【正文】+ 三买卖
//! 点坐实 108:14/020:60【正文】，点名函数不另造）：
//! - 走势类型延续 = D3 方向行该层仍 Down（021:28「走势的延续或转折」）；
//! - 中枢未死 = `CenterBook::alive`（021:28「中枢有三种情况：延续、扩张与
//!   新生」的生命面）；
//! - 三买卖未坐实 = `!CenterBook::is_frozen`（三买侧）/ 当前 last 中枢未被
//!   confirmed Sell3 终结（三卖侧；108:14 底部/顶部构造 + 020:60 三买卖
//!   定理的 confirmed 侧）。
//!
//! 价格创新低/新高已降为观测（`down_new_low_observed`/`up_new_high_observed`
//! 诊断读数），不参判据。Fractal 子腿在父级别（子腿 ladder+1）向下走势窗口
//! 内操作——窗口本身可能只是父级别下跌走势的一段：若父级别走势未完成且
//! 当前段窗口内无盘整背驰，下跌走势未完，子腿短差的顶/底分型节奏不可靠
//! （趋势中无可靠"底"）。
//!
//! 与 FatigueGate（C7 rev_gate 的 41课门 v2 点态）的范畴区分：
//! - FatigueGate 守 **REV 开腿**（本级别 Up 走势的衰竭），数据基础 run_high
//!   行——当前磁带未产出 ⇒ rev_gate 变体 fail-fast；
//! - 本追踪器守 **Fractal 子腿开腿**（父级别 Down 走势的衰竭），判据面数据
//!   基础 D3 方向行 + div 事件流 + CenterBook 中枢态——close 序列仅降观测
//!   用，当前磁带已产出，无新磁带依赖。
//!
//! 存在论位置：市场性质（与 CenterBook/DepthRef 同级），跨 trade / 跨 REV
//! 窗口持续——判据的中枢态与方向行由全 run 账本（CenterBook）供读，窗口内
//! （VoiceUnit/SubLou 生命期内）的记忆不足以构成判据定义域。
//!
//! 盘整背驰记忆作用域 = **当前 Down 段窗口**（段起点重置）：背驰判定在
//! 缠论中逐"段对"独立进行（前一段对的背驰不外推到下一段对），故每个新
//! Down 段以无衰竭证据起步，段内（含其后的回拉 Up 段，直到下一 Down 段
//! 开始）观测到的 Consolidation×Down 事件累积为该段的衰竭证据。
//!
//! 分辨率诚实声明：降观测的创新低/新高比较仍用 close 序列（low_since_open
//! 同先例——close 分辨率，非 raw bar low）；方向行是 bar 末状态，同 bar 多
//! 翻转折叠（Fractal 子腿同声明：漏单非错单）。

use crate::divergence::DivKind;
use crate::stroke::Direction;

use super::center_book::CenterBook;
use super::types::{DivEvent, MAX_LADDER};

/// 单层双向段运行状态（Down 侧守 Fractal 子腿；Up 侧守 REV 开腿——
/// θ 相对化落地任务 2026-06-11，41课门的镜像形式）。
#[derive(Debug, Clone, Copy, Default)]
struct LadderState {
    /// 该层方向行上一观测（段起点检测，双向共用）。
    last_dir: Option<Direction>,
    /// 上一个已完成 Down 段的 close 最低价（诊断观测，不参判据，#1232 裁定 a）。
    prev_down_low: Option<f64>,
    /// 当前（或最近完成）Down 段的 close 最低价（诊断观测，不参判据）。
    cur_down_low: Option<f64>,
    /// 当前 Down 段窗口内已观测盘整背驰（Consolidation×Down）。
    consol_div_seen: bool,
    /// 上一个已完成 Up 段的 close 最高价（诊断观测，不参判据）。
    prev_up_high: Option<f64>,
    /// 当前（或最近完成）Up 段的 close 最高价（诊断观测，不参判据）。
    cur_up_high: Option<f64>,
    /// 当前 Up 段窗口内已观测盘整背驰（Consolidation×Up）。
    consol_up_seen: bool,
}

/// 全层位走势衰竭追踪器。runner 每 bar 驱动（仅 sub_l41_gate 变体），
/// BarRows 持只读借用供 SubLou 开腿门控查询。
#[derive(Debug)]
pub struct TrendExhaustion {
    states: [LadderState; MAX_LADDER],
}

impl TrendExhaustion {
    pub fn new() -> Self {
        TrendExhaustion {
            states: [LadderState::default(); MAX_LADDER],
        }
    }

    /// 每 bar 观测（市场性质，与持仓状态无关——runner 在 FLAT/ARMED/LONG
    /// 全态驱动，与 CenterBook/DepthRef 同序）。
    ///
    /// 推进序：先记盘整背驰，再做段起点/段内推进——段起点重置发生在同 bar
    /// 背驰记录之后（同 bar 共现时事件描述的是已完成的上一段对，归旧段）。
    pub fn observe(
        &mut self,
        dir_row: &[Option<Direction>; MAX_LADDER],
        devs: &[Vec<DivEvent>; MAX_LADDER],
        c: f64,
    ) {
        for lad in 0..MAX_LADDER {
            let st = &mut self.states[lad];
            if devs[lad]
                .iter()
                .any(|d| d.kind == DivKind::Consolidation && d.direction == Direction::Down)
            {
                st.consol_div_seen = true;
            }
            if devs[lad]
                .iter()
                .any(|d| d.kind == DivKind::Consolidation && d.direction == Direction::Up)
            {
                st.consol_up_seen = true;
            }
            let now = dir_row[lad];
            if now == Some(Direction::Down) {
                if st.last_dir != Some(Direction::Down) {
                    // 新 Down 段起点：上一段低点归档，衰竭证据按段对重置
                    st.prev_down_low = st.cur_down_low.take();
                    st.cur_down_low = Some(c);
                    st.consol_div_seen = false;
                } else {
                    match st.cur_down_low.as_mut() {
                        Some(low) if c < *low => *low = c,
                        Some(_) => {}
                        None => st.cur_down_low = Some(c),
                    }
                }
            } else if now == Some(Direction::Up) {
                if st.last_dir != Some(Direction::Up) {
                    // 新 Up 段起点：上一段高点归档，衰竭证据按段对重置（镜像）
                    st.prev_up_high = st.cur_up_high.take();
                    st.cur_up_high = Some(c);
                    st.consol_up_seen = false;
                } else {
                    match st.cur_up_high.as_mut() {
                        Some(high) if c > *high => *high = c,
                        Some(_) => {}
                        None => st.cur_up_high = Some(c),
                    }
                }
            }
            if now.is_some() {
                st.last_dir = now;
            }
        }
    }

    /// 41课判据：该层向下走势**无衰竭迹象**（走势未完成——子腿拒开条件）。
    ///
    /// 判据链（#1232 第三条裁定 a，2026-08-27）：结构判据「走势未完成」∧
    /// 当前段窗口内无盘整背驰。结构判据三项任一成立即走势未完成：
    /// - 走势类型延续：D3 方向行该层仍 Down（`dir == Some(Down)`，
    ///   021:28【正文】「走势的延续或转折」）；
    /// - 中枢未死：`CenterBook::alive`（021:28【正文】中枢三态的生命面）；
    /// - 三买未坐实：`!CenterBook::is_frozen`（108:14/020:60【正文】三买
    ///   定理的 confirmed 侧——底部构造未完成）。
    ///
    /// 价格创新低已降观测（[`Self::down_new_low_observed`]），不参判据。
    /// 证据缺失（方向行与中枢态均无正面证据）⇒ false：判据是正面的走势
    /// 延续证据，证据缺失不构成"未衰竭"陈述（门只在证据成立时关）。
    pub fn down_unexhausted(
        &self,
        ladder: usize,
        book: &CenterBook,
        dir: Option<Direction>,
    ) -> bool {
        let st = &self.states[ladder];
        !st.consol_div_seen
            && (dir == Some(Direction::Down)
                || book.alive(ladder).is_some()
                || !book.is_frozen(ladder))
    }

    /// 41课判据镜像：该层向上走势**无衰竭迹象**（走势未完成——REV 反向腿
    /// 拒开条件，θ 相对化落地任务）。判据链同 [`Self::down_unexhausted`]，
    /// 方向镜像（#1232 第三条裁定 a）：结构判据三项任一成立即走势未完成——
    /// - 走势类型延续：D3 方向行该层仍 Up（`dir == Some(Up)`）；
    /// - 中枢未死：`CenterBook::alive`；
    /// - 三卖未坐实：当前 last 中枢未被 confirmed Sell3 终结（108:14/020:60
    ///   顶部构造未完成的 confirmed 侧）。
    ///
    /// 价格创新高已降观测（[`Self::up_new_high_observed`]），不参判据。
    /// ladder 越界（无父级别可观测）或证据缺失 ⇒ false——判据是正面的
    /// 走势延续证据，证据缺失不拒开。
    pub fn up_unexhausted(&self, ladder: usize, book: &CenterBook, dir: Option<Direction>) -> bool {
        let Some(st) = self.states.get(ladder) else {
            return false;
        };
        let sell3_confirmed = book
            .last(ladder)
            .is_some_and(|lc| book.is_dead_down(ladder, lc.seg_start));
        !st.consol_up_seen
            && (dir == Some(Direction::Up) || book.alive(ladder).is_some() || !sell3_confirmed)
    }

    /// 诊断观测（不参判据，#1232 第三条裁定 a）：相邻同向 Down 段是否创新低
    /// （close 口径）。原判据价格分量降观测后保留；`#[allow(dead_code)]` =
    /// 观测位（仅测试消费，生产无读数点）。
    #[allow(dead_code)]
    pub fn down_new_low_observed(&self, ladder: usize) -> bool {
        let st = &self.states[ladder];
        matches!(
            (st.prev_down_low, st.cur_down_low),
            (Some(prev), Some(cur)) if cur < prev
        )
    }

    /// 诊断观测（不参判据，#1232 第三条裁定 a）：相邻同向 Up 段是否创新高
    /// （close 口径）。原判据价格分量降观测后保留；`#[allow(dead_code)]` =
    /// 观测位（仅测试消费，生产无读数点）。
    #[allow(dead_code)]
    pub fn up_new_high_observed(&self, ladder: usize) -> bool {
        let Some(st) = self.states.get(ladder) else {
            return false;
        };
        matches!(
            (st.prev_up_high, st.cur_up_high),
            (Some(prev), Some(cur)) if cur > prev
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::center_book::CenterBook;
    use super::super::types::{BspClass, BspEvent};
    use super::*;

    fn consol_down() -> DivEvent {
        DivEvent {
            kind: DivKind::Consolidation,
            direction: Direction::Down,
            seg_idx: 0,
            force_a: 0.0,
            force_c: 0.0,
            price: 0.0,
        }
    }

    fn consol_up() -> DivEvent {
        DivEvent {
            kind: DivKind::Consolidation,
            direction: Direction::Up,
            seg_idx: 0,
            force_a: 0.0,
            force_c: 0.0,
            price: 0.0,
        }
    }

    fn anchored(class: BspClass, confirmed: bool, cs: i64, zd: f64, zg: f64) -> BspEvent {
        BspEvent {
            class,
            seg_idx: 0,
            confirmed,
            cs: Some(cs),
            zd: Some(zd),
            zg: Some(zg),
            price: 0.0,
        }
    }

    #[test]
    fn down_structural_incomplete_without_div_is_unexhausted() {
        // 判据链 = 走势未完成（结构）∧ 无盘整背驰。结构判据三项任一成立即
        // 未完成：走势类型延续（dir Down）/ 中枢未死（alive）/ 三买未坐实
        // （!frozen）。
        let mut devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();
        let mut dir: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        let mut te = TrendExhaustion::new();
        let mut book = CenterBook::new();
        // 方向行 Down、无中枢 → 走势类型延续 ⇒ 未完成
        dir[2] = Some(Direction::Down);
        te.observe(&dir, &devs, 10.0);
        assert!(te.down_unexhausted(2, &book, dir[2]));
        // 盘整背驰出现 → 无盘背分量被否定 ⇒ 衰竭证据成立，门开
        devs[2] = vec![consol_down()];
        te.observe(&dir, &devs, 9.0);
        assert!(!te.down_unexhausted(2, &book, dir[2]));
        devs[2].clear();
        // 新 Down 段起点重置衰竭证据（背驰逐段对判定），方向行 Down ⇒ 未完成
        dir[2] = Some(Direction::Up);
        te.observe(&dir, &devs, 9.5);
        dir[2] = Some(Direction::Down);
        te.observe(&dir, &devs, 9.2);
        assert!(te.down_unexhausted(2, &book, dir[2]));
        // 方向行翻 Up、无中枢 → 三买未坐实（!frozen）⇒ 仍未完成
        dir[2] = Some(Direction::Up);
        te.observe(&dir, &devs, 9.6);
        assert!(te.down_unexhausted(2, &book, dir[2]));
        // 方向行 Up、中枢未死（alive）→ 中枢未死 ⇒ 未完成
        book.ingest(
            2,
            &[anchored(BspClass::Sell1, true, 10, 9.0, 9.5)],
            true,
            None,
        );
        assert!(book.alive(2).is_some());
        assert!(te.down_unexhausted(2, &book, dir[2]));
        // 三买坐实（confirmed hard Buy3 杀中枢 + frozen）→ 三项皆假 ⇒ 完成
        book.ingest(
            2,
            &[anchored(BspClass::Buy3, true, 10, 9.0, 9.5)],
            true,
            None,
        );
        assert!(book.is_frozen(2));
        assert!(book.alive(2).is_none());
        assert!(!te.down_unexhausted(2, &book, dir[2]));
        // 三买坐实后方向行仍 Down（时序错位）→ 走势类型延续项仍成立（OR 结构）
        dir[2] = Some(Direction::Down);
        te.observe(&dir, &devs, 9.0);
        assert!(te.down_unexhausted(2, &book, dir[2]));
    }

    #[test]
    fn up_mirror_structural_incomplete_without_div_is_unexhausted() {
        let mut devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();
        let mut dir: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        let mut te = TrendExhaustion::new();
        let mut book = CenterBook::new();
        // 方向行 Up、无中枢 → 走势类型延续 ⇒ 未完成
        dir[3] = Some(Direction::Up);
        te.observe(&dir, &devs, 10.0);
        assert!(te.up_unexhausted(3, &book, dir[3]));
        // 盘整背驰（Consolidation×Up）→ 无盘背分量被否定 ⇒ 门开
        devs[3] = vec![consol_up()];
        te.observe(&dir, &devs, 11.0);
        assert!(!te.up_unexhausted(3, &book, dir[3]));
        devs[3].clear();
        // 新 Up 段起点重置衰竭证据（背驰逐段对判定），方向行 Up ⇒ 未完成
        dir[3] = Some(Direction::Down);
        te.observe(&dir, &devs, 10.5);
        dir[3] = Some(Direction::Up);
        te.observe(&dir, &devs, 11.2);
        assert!(te.up_unexhausted(3, &book, dir[3]));
        // 方向行翻 Down、无中枢 → 三卖未坐实 ⇒ 仍未完成
        dir[3] = Some(Direction::Down);
        te.observe(&dir, &devs, 10.8);
        assert!(te.up_unexhausted(3, &book, dir[3]));
        // 方向行 Down、中枢未死 → 中枢未死 ⇒ 未完成
        book.ingest(
            3,
            &[anchored(BspClass::Buy1, true, 10, 9.0, 9.5)],
            true,
            None,
        );
        assert!(book.alive(3).is_some());
        assert!(te.up_unexhausted(3, &book, dir[3]));
        // 三卖坐实（confirmed Sell3 杀中枢 + dead_down）→ 三项皆假 ⇒ 完成
        book.ingest(
            3,
            &[anchored(BspClass::Sell3, true, 10, 9.0, 9.5)],
            true,
            None,
        );
        assert!(book.alive(3).is_none());
        assert!(book.is_dead_down(3, 10));
        assert!(!te.up_unexhausted(3, &book, dir[3]));
        // 越界 ladder：无父级别可观测 ⇒ false（门放行）
        assert!(!te.up_unexhausted(MAX_LADDER + 5, &book, None));
        // Down 侧状态不受 Up 侧推进污染（方向行 Down、无盘背 ⇒ 走势类型延续）
        assert!(te.down_unexhausted(3, &book, dir[3]));
    }

    #[test]
    fn diagnostic_new_extremes_are_observations_only() {
        // 价格创新低/新高降为诊断观测：读数不影响判据（判据只看结构 + 无盘背）。
        let devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();
        let mut dir: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        let mut te = TrendExhaustion::new();
        let book = CenterBook::new();
        // 两个 Down 段创新低（诊断观测成立）→ 诊断读数 true
        dir[2] = Some(Direction::Down);
        te.observe(&dir, &devs, 10.0);
        te.observe(&dir, &devs, 9.0);
        dir[2] = Some(Direction::Up);
        te.observe(&dir, &devs, 9.5);
        dir[2] = Some(Direction::Down);
        te.observe(&dir, &devs, 9.2);
        te.observe(&dir, &devs, 8.0);
        assert!(te.down_new_low_observed(2));
        // 方向行 Down → 判据由结构项成立；创新低观测与判据分离（观测不推翻判据）
        assert!(te.down_unexhausted(2, &book, dir[2]));
        // 方向行翻 Up、无中枢 → 判据仍成立（三买未坐实），诊断读数保持
        dir[2] = Some(Direction::Up);
        te.observe(&dir, &devs, 9.6);
        assert!(te.down_new_low_observed(2));
        assert!(te.down_unexhausted(2, &book, dir[2]));
        // Up 侧镜像：创新高观测成立，方向行 Down、无中枢 → 判据仍成立
        dir[3] = Some(Direction::Up);
        te.observe(&dir, &devs, 10.0);
        te.observe(&dir, &devs, 11.0);
        dir[3] = Some(Direction::Down);
        te.observe(&dir, &devs, 10.5);
        dir[3] = Some(Direction::Up);
        te.observe(&dir, &devs, 11.5);
        assert!(te.up_new_high_observed(3));
        assert!(te.up_unexhausted(3, &book, dir[3]));
        dir[3] = Some(Direction::Down);
        te.observe(&dir, &devs, 11.0);
        assert!(te.up_new_high_observed(3));
        assert!(te.up_unexhausted(3, &book, dir[3]));
    }
}
