//! GUARD-ROLE: organic-fugue-v2-trading-layer——名分：现役（详见 `trading/mod.rs` 头部 GUARD-ROLE 块，#763 C7-E4 核定；零删除/零移入 legacy/）。
//!
//! TrendExhaustion — 41课门的父级别走势衰竭追踪器（P1 双门任务，2026-06-11）。
//!
//! 41课原文："大级别走势没有任何衰竭迹象时参与反向小级别买卖点是刀口舔血"。
//! 可编码判据（38课原文的结构化形式）：**相邻同向段创新低 ∧ 无盘整背驰
//! = 趋势未完 = 不做反向**。Fractal 子腿在父级别（子腿 ladder+1）向下走势
//! 窗口内操作——窗口本身可能只是父级别下跌趋势的一段：若父级别相邻 Down
//! 段持续创新低且当前段窗口内无盘整背驰，下跌趋势未完，子腿短差的
//! 顶/底分型节奏不可靠（趋势中无可靠"底"）。
//!
//! 与 FatigueGate（C7 rev_gate 的 41课门 v2 点态）的范畴区分：
//! - FatigueGate 守 **REV 开腿**（本级别 Up 走势的衰竭），数据基础 run_high
//!   行——当前磁带未产出 ⇒ rev_gate 变体 fail-fast；
//! - 本追踪器守 **Fractal 子腿开腿**（父级别 Down 走势的衰竭），数据基础
//!   D3 方向行 + close 序列 + div 事件流——当前磁带已产出，无新磁带依赖。
//!
//! 存在论位置：市场性质（与 CenterBook/DepthRef 同级），跨 trade / 跨 REV
//! 窗口持续——"相邻同向段"的前一 Down 段必然终结于当前 REV 窗口之前，
//! 窗口内（VoiceUnit/SubLou 生命期内）的记忆不足以构成判据定义域。
//!
//! 盘整背驰记忆作用域 = **当前 Down 段窗口**（段起点重置）：背驰判定在
//! 缠论中逐"段对"独立进行（前一段对的背驰不外推到下一段对），故每个新
//! Down 段以无衰竭证据起步，段内（含其后的回拉 Up 段，直到下一 Down 段
//! 开始）观测到的 Consolidation×Down 事件累积为该段的衰竭证据。
//!
//! 分辨率诚实声明：创新低比较用 close 序列（low_since_open 同先例——
//! close 分辨率，非 raw bar low）；方向行是 bar 末状态，同 bar 多翻转折叠
//! （Fractal 子腿同声明：漏单非错单）。

use crate::divergence::DivKind;
use crate::stroke::Direction;

use super::types::{DivEvent, MAX_LADDER};

/// 单层双向段运行状态（Down 侧守 Fractal 子腿；Up 侧守 REV 开腿——
/// θ 相对化落地任务 2026-06-11，41课门的镜像形式）。
#[derive(Debug, Clone, Copy, Default)]
struct LadderState {
    /// 该层方向行上一观测（段起点检测，双向共用）。
    last_dir: Option<Direction>,
    /// 上一个已完成 Down 段的 close 最低价。
    prev_down_low: Option<f64>,
    /// 当前（或最近完成）Down 段的 close 最低价。
    cur_down_low: Option<f64>,
    /// 当前 Down 段窗口内已观测盘整背驰（Consolidation×Down）。
    consol_div_seen: bool,
    /// 上一个已完成 Up 段的 close 最高价。
    prev_up_high: Option<f64>,
    /// 当前（或最近完成）Up 段的 close 最高价。
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

    /// 41课判据：该层向下走势**无衰竭迹象**（趋势未完——子腿拒开条件）。
    /// = 相邻同向（Down）段创新低 ∧ 当前段窗口内无盘整背驰。
    /// 相邻段对不可定义（不足两个 Down 段观测）⇒ false：判据是正面的
    /// 趋势延续证据，证据缺失不构成"未衰竭"陈述（门只在证据成立时关）。
    pub fn down_unexhausted(&self, ladder: usize) -> bool {
        let st = &self.states[ladder];
        !st.consol_div_seen
            && matches!(
                (st.prev_down_low, st.cur_down_low),
                (Some(prev), Some(cur)) if cur < prev
            )
    }

    /// 41课判据镜像：该层向上走势**无衰竭迹象**（上涨趋势未完——REV 反向腿
    /// 拒开条件，θ 相对化落地任务）。= 相邻同向（Up）段创新高 ∧ 当前段窗口
    /// 内无盘整背驰（Consolidation×Up）。ladder 越界（无父级别可观测）或
    /// 段对不可定义 ⇒ false——判据是正面的趋势延续证据，证据缺失不拒开。
    pub fn up_unexhausted(&self, ladder: usize) -> bool {
        let Some(st) = self.states.get(ladder) else {
            return false;
        };
        !st.consol_up_seen
            && matches!(
                (st.prev_up_high, st.cur_up_high),
                (Some(prev), Some(cur)) if cur > prev
            )
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn new_low_without_div_is_unexhausted() {
        let mut devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();
        let mut dir: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        let mut te = TrendExhaustion::new();
        // Down 段1：低点 9.0
        dir[2] = Some(Direction::Down);
        te.observe(&dir, &devs, 10.0);
        te.observe(&dir, &devs, 9.0);
        // 段对不足：不可判未衰竭
        assert!(!te.down_unexhausted(2));
        // 回拉 Up 段
        dir[2] = Some(Direction::Up);
        te.observe(&dir, &devs, 9.5);
        // Down 段2：创新低 8.0，无盘整背驰 → 趋势未完
        dir[2] = Some(Direction::Down);
        te.observe(&dir, &devs, 9.2);
        assert!(!te.down_unexhausted(2)); // 9.2 > 9.0 尚未创新低
        te.observe(&dir, &devs, 8.0);
        assert!(te.down_unexhausted(2));
        // 段内出现盘整背驰（Consolidation×Down）→ 衰竭证据成立，门开
        devs[2] = vec![consol_down()];
        te.observe(&dir, &devs, 7.9);
        assert!(!te.down_unexhausted(2));
        devs[2].clear();
        // 下一个 Down 段起点重置衰竭证据（背驰逐段对判定）
        dir[2] = Some(Direction::Up);
        te.observe(&dir, &devs, 8.5);
        dir[2] = Some(Direction::Down);
        te.observe(&dir, &devs, 7.0);
        assert!(te.down_unexhausted(2)); // 7.0 < 7.9 创新低且证据已重置
    }

    #[test]
    fn up_mirror_new_high_without_div_is_unexhausted() {
        let mut devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();
        let mut dir: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        let mut te = TrendExhaustion::new();
        // Up 段1：高点 10.0
        dir[3] = Some(Direction::Up);
        te.observe(&dir, &devs, 9.0);
        te.observe(&dir, &devs, 10.0);
        assert!(!te.up_unexhausted(3)); // 段对不足
                                        // 回调 Down 段
        dir[3] = Some(Direction::Down);
        te.observe(&dir, &devs, 9.5);
        // Up 段2：创新高 11.0，无盘整背驰 → 上涨趋势未完
        dir[3] = Some(Direction::Up);
        te.observe(&dir, &devs, 9.8);
        assert!(!te.up_unexhausted(3)); // 尚未创新高
        te.observe(&dir, &devs, 11.0);
        assert!(te.up_unexhausted(3));
        // 段内盘整背驰（Consolidation×Up）→ 衰竭证据成立
        devs[3] = vec![DivEvent {
            kind: DivKind::Consolidation,
            direction: Direction::Up,
            seg_idx: 0,
            force_a: 0.0,
            force_c: 0.0,
            price: 0.0,
        }];
        te.observe(&dir, &devs, 11.1);
        assert!(!te.up_unexhausted(3));
        // 越界 ladder：无父级别可观测 ⇒ false（门放行）
        assert!(!te.up_unexhausted(MAX_LADDER + 5));
        // Down 侧状态不受 Up 侧推进污染
        assert!(!te.down_unexhausted(3));
    }

    #[test]
    fn higher_low_is_not_unexhausted() {
        let devs: [Vec<DivEvent>; MAX_LADDER] = Default::default();
        let mut dir: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        let mut te = TrendExhaustion::new();
        dir[3] = Some(Direction::Down);
        te.observe(&dir, &devs, 9.0);
        dir[3] = Some(Direction::Up);
        te.observe(&dir, &devs, 9.5);
        dir[3] = Some(Direction::Down);
        te.observe(&dir, &devs, 9.3); // 低点抬高（上行结构）
        assert!(!te.down_unexhausted(3));
    }
}
