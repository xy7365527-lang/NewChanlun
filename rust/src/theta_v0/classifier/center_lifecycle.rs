//! 中枢生命周期事件机（#291 / SPEC #274 T1，ADR 0001 修正案一·补充二「中枢=事件」裁定）。
//!
//! ## 教义口径（ADR 0001 修正案一·补充二，2026-07-26 用户逐题终审）
//!
//! - **出生** = 前三个次级别段重叠完成（第三段重叠完成 ⟹ 中枢出生事件）。判据复用
//!   [`super::center::center_from_segments`]（L0 完整判据：方向交替 ∧ 全三段核心非空，口径 B，
//!   637号）/ [`super::center::center_from_window`]（上级几何判据——上级单元无内在缠论方向，
//!   见 center.rs 诚实有效域声明），与塔 compose 同一对构造算子（禁第二套中枢判据）。
//! - **破坏（死亡）** = 三类买卖点（本级别确认的 buy3/sell3 点 ⟹ 在场中枢死亡）。只消费
//!   **已确认**的点（修6：候选→区间套确认才出生，定账只消费已确认的点）。
//! - **一类点同死** = 本级一类点（buy1/sell1）⟹ **段序列与在场中枢同死**，新走势类型的中枢
//!   从全新段计数（禁横跨走势类型生死边界拼中枢）。
//!
//! ## 边界（票面 #291 范围）
//!
//! - **只产出事件，不产出动作**：本机是结构地基（狭义短差减补动作 = #292），事件不触发任何
//!   交易行为；默认零行为变化（事件产出经 env-gated 只读旁路外化，验收锚 = 既有轨迹逐位不变）。
//! - 中枢**延伸/升级不是本机事件**（塔 `tower_events` 的 extend/level_upgrade 域）：本机在场中枢
//!   保持出生时刻的 ZD/ZG 快照，直至破坏/同死。与塔口径差异在 wf8 对账中如实列出，不强行调和。
//! - 一类+三类 bit 同点：一类优先（同死吞没破坏——1B/3B 前提冲突互斥，bsp.rs `no_exclusive_
//!   trichotomy` 结构下理论不同位；防御性规定，照实标注）。
//!
//! ## 段计数口径
//!
//! - 段序列 = 自上次一类点同死（或机器构造）起喂入的段；`born_seg_ordinal` = 构成中枢的**第三段**
//!   （完成重叠那段）在当前段序列中的 1-based 序号。三类破坏**不清**段序列（趋势延续，新中枢自
//!   后续段滑动窗口出生）；一类同死**清零**段序列。
//! - 滑窗：在场中枢缺席时，每新段测试**尾 3 段**窗口（与塔窗口扫描同义——前 3 段不成立则
//!   第 2/3/4 段仍可成交）；在场中枢存在时不测出生（一中枢一场）。

use super::super::types::{BspBits, Center, Side};
use super::center::{center_from_segments, center_from_window, UnitRange};

/// 中枢生命周期事件（#291 三类：born/broken/reset）。事件含级别、中枢区间（ZD/ZG）、出生段号。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CenterLifecycleEvent {
    /// 出生：第三段重叠完成（`build` 判据接受尾 3 段窗口）。
    Born {
        level: u32,
        /// 出生时刻中枢（核心 [zd,zg] + 外缘 [dd,gg] + 源坐标 [start_index,end_index]）。
        center: Center,
        /// 出生段号：完成重叠的第三段在当前段序列（一类点同死后重新计数）中的 1-based 序号。
        born_seg_ordinal: usize,
    },
    /// 破坏：本级别确认的三类买卖点 ⟹ 在场中枢死亡（段序列**不清零**）。
    Broken {
        level: u32,
        /// 死亡中枢（出生时刻快照）。
        center: Center,
        /// 该中枢出生时的段号（溯源）。
        born_seg_ordinal: usize,
        /// 触发破坏的三类点 source_index（确认坐标）。
        breaker_source_index: usize,
        /// 三类点方向（buy3 ⟹ Long / sell3 ⟹ Short，types::Side 买卖语境）。
        breaker_side: Side,
    },
    /// 一类点同死：本级一类点 ⟹ 段序列清零 + 在场中枢（若有）同死。新中枢从全新段计数。
    Reset {
        level: u32,
        /// 同死的在场中枢（出生快照）；无在场中枢 ⟹ None（段序列仍清零）。
        died_center: Option<Center>,
        /// 同死中枢的出生段号（无在场中枢 ⟹ None）。
        died_born_seg_ordinal: Option<usize>,
        /// 被清零的段序列长度（计数清零的证据）。
        cleared_segments: usize,
        /// 触发同死的一类点 source_index（确认坐标）。
        trigger_source_index: usize,
        /// 一类点方向（buy1 ⟹ Long / sell1 ⟹ Short）。
        trigger_side: Side,
    },
}

/// 中枢生命周期事件机（每级别一台；纯结构，L0 判据，不读经验参数）。
///
/// 构造算子 `build` 按级别锁定：L0 = [`center_from_segments`]（完整判据，方向交替）；
/// ℓ≥1 = [`center_from_window`]（几何判据——上级单元无 §6.1 方向维度，center.rs 有效域）。
pub struct CenterEventMachine {
    level: u32,
    build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
    /// 段序列（自上次一类点同死/工程 resync 起）。
    segs: Vec<UnitRange>,
    /// 在场中枢：(出生快照, 出生段号)。Some = 已出生未死。
    alive: Option<(Center, usize)>,
    born_total: usize,
    broken_total: usize,
    reset_total: usize,
}

impl CenterEventMachine {
    /// 构造级别 `level` 的事件机（L0 完整判据 / ℓ≥1 几何判据，与塔 compose_level 同口径）。
    pub fn new(level: u32) -> Self {
        Self {
            level,
            build: if level == 0 {
                center_from_segments
            } else {
                center_from_window
            },
            segs: Vec::new(),
            alive: None,
            born_total: 0,
            broken_total: 0,
            reset_total: 0,
        }
    }

    /// 喂入一段（段序列尾部追加）。在场中枢缺席且段数 ≥3 ⟹ 测尾 3 段窗口，成交 ⟹ `Born`。
    pub fn push_segment(&mut self, seg: UnitRange) -> Option<CenterLifecycleEvent> {
        self.segs.push(seg);
        // 在场中枢存在 ⟹ 不测出生（一中枢一场；延伸非本机事件，见模块头边界）。
        if self.alive.is_some() {
            return None;
        }
        let n = self.segs.len();
        if n < 3 {
            return None;
        }
        // 尾 3 段窗口（滑窗：前 3 段不成立则第 2/3/4…段仍可成交，与塔窗口扫描同义）。
        let (a, b, c) = (&self.segs[n - 3], &self.segs[n - 2], &self.segs[n - 1]);
        let center = (self.build)(a, b, c)?;
        let born_seg_ordinal = n; // 1-based 出生段号 = 完成重叠的第三段在当前段序列中的序号。
        self.alive = Some((center, born_seg_ordinal));
        self.born_total += 1;
        Some(CenterLifecycleEvent::Born { level: self.level, center, born_seg_ordinal })
    }

    /// 喂入一个**已确认**买卖点（修6：只消费已确认的点）。
    ///
    /// 一类 bit（buy1/sell1）⟹ `Reset`（段序列清零 + 在场中枢同死；一类优先于三类）；
    /// 否则三类 bit（buy3/sell3）且在场中枢存在 ⟹ `Broken`。二类点不产事件（不在三类事件教义内）。
    pub fn push_point(&mut self, bits: BspBits, source_index: usize) -> Option<CenterLifecycleEvent> {
        // 一类优先（1B/3B 前提冲突互斥，理论不同位；防御性规定，模块头已标注）。
        let first = if bits.buy1 {
            Some(Side::Long)
        } else if bits.sell1 {
            Some(Side::Short)
        } else {
            None
        };
        if let Some(trigger_side) = first {
            let cleared_segments = self.segs.len();
            let (died_center, died_born_seg_ordinal) = match self.alive.take() {
                Some((c, ord)) => (Some(c), Some(ord)),
                None => (None, None),
            };
            self.segs.clear(); // 段序列与在场中枢同死（新中枢从全新段计数）。
            self.reset_total += 1;
            return Some(CenterLifecycleEvent::Reset {
                level: self.level,
                died_center,
                died_born_seg_ordinal,
                cleared_segments,
                trigger_source_index: source_index,
                trigger_side,
            });
        }
        let third = if bits.buy3 {
            Some(Side::Long)
        } else if bits.sell3 {
            Some(Side::Short)
        } else {
            None
        };
        if let Some(breaker_side) = third {
            // 无在场中枢 ⟹ 诚实 no-op（不杀不存在的中枢）；段序列不清零（趋势延续）。
            let (center, born_seg_ordinal) = self.alive.take()?;
            self.broken_total += 1;
            return Some(CenterLifecycleEvent::Broken {
                level: self.level,
                center,
                born_seg_ordinal,
                breaker_source_index: source_index,
                breaker_side,
            });
        }
        None
    }

    /// 工程性再同步（旁路喂数层专用：塔 cascade 失效/水线回缩致已喂前缀不可信时调用）。
    /// 清空段序列与在场中枢，**不产事件**（非教义生死，照实区别于 Reset）。
    pub fn resync(&mut self) {
        self.segs.clear();
        self.alive = None;
    }

    /// 在场中枢（出生快照 + 出生段号）；None = 当前无在场中枢。
    pub fn alive_center(&self) -> Option<(Center, usize)> {
        self.alive
    }

    /// 当前段序列长度（自上次一类点同死/resync 起计数）。
    pub fn segments_since_reset(&self) -> usize {
        self.segs.len()
    }

    /// (born, broken, reset) 累计事件计数（wf8 自证读数）。
    pub fn counts(&self) -> (usize, usize, usize) {
        (self.born_total, self.broken_total, self.reset_total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::{Direction, Tick};

    /// 方向交替单元（与 center.rs 测试同款构造器）。
    fn unit(si: usize, ei: usize, dir: Direction, lo: Tick, hi: Tick) -> UnitRange {
        UnitRange { start_index: si, end_index: ei, direction: dir, lo, hi }
    }

    fn up() -> Direction { Direction::Up }
    fn down() -> Direction { Direction::Down }

    fn bits_1b() -> BspBits { BspBits { buy1: true, ..Default::default() } }
    fn bits_1s() -> BspBits { BspBits { sell1: true, ..Default::default() } }
    fn bits_3b() -> BspBits { BspBits { buy3: true, ..Default::default() } }
    fn bits_3s() -> BspBits { BspBits { sell3: true, ..Default::default() } }
    fn bits_2b() -> BspBits { BspBits { buy2: true, ..Default::default() } }

    // ──────────────────────────────────────────────────────────────────────
    //  出生（3 段重叠 → born 事件，含级别/ZD/ZG/出生段号）
    // ──────────────────────────────────────────────────────────────────────

    /// ★出生正例（口径 B bit-exact，center.rs complete_center_confirmed_bit_exact 同构）：
    /// 上-下-上 a=[10,20] b=[12,20] c=[12,22]，全三段核心 [12,20] 非空 ⟹ 第三段完成时 Born。
    #[test]
    fn born_on_third_segment_overlap() {
        let mut m = CenterEventMachine::new(0);
        assert_eq!(m.push_segment(unit(0, 4, up(), 10, 20)), None, "第 1 段不出生");
        assert_eq!(m.push_segment(unit(4, 8, down(), 12, 20)), None, "第 2 段不出生");
        let ev = m.push_segment(unit(8, 12, up(), 12, 22));
        let center = Center { zd: 12, zg: 20, dd: 10, gg: 22, start_index: 0, end_index: 12 };
        assert_eq!(
            ev,
            Some(CenterLifecycleEvent::Born { level: 0, center, born_seg_ordinal: 3 }),
            "第三段重叠完成 ⟹ born 事件（含级别/ZD/ZG/出生段号）"
        );
        assert_eq!(m.alive_center(), Some((center, 3)), "出生后在场");
        assert_eq!(m.counts(), (1, 0, 0));
        // 在场后第 4 段不再产 born（一中枢一场，延伸非本机事件）。
        assert_eq!(m.push_segment(unit(12, 16, down(), 13, 21)), None, "在场中枢存在时不测出生");
        assert_eq!(m.counts(), (1, 0, 0));
    }

    /// ★滑窗出生：前 3 段核心空不出生，第 2/3/4…段窗口滑动后成交 ⟹ born（出生段号 = 完成段序号）。
    #[test]
    fn born_sliding_window_after_empty_core() {
        let mut m = CenterEventMachine::new(0);
        // (a,b,c)：zd=max(0,10,5)=10 > zg=min(4,14,9)=4 ⟹ 核心空，不出生。
        assert_eq!(m.push_segment(unit(0, 4, up(), 0, 4)), None);
        assert_eq!(m.push_segment(unit(4, 8, down(), 10, 14)), None);
        assert_eq!(m.push_segment(unit(8, 12, up(), 5, 9)), None, "前 3 段核心空 ⟹ 无 born");
        // (b,c,d)：zd=max(10,5,8)=10 > zg=min(14,9,12)=9 ⟹ 仍空。
        assert_eq!(m.push_segment(unit(12, 16, down(), 8, 12)), None, "滑窗 (b,c,d) 核心空 ⟹ 无 born");
        // (c,d,e)：zd=max(5,8,9)=9 ≤ zg=min(9,12,13)=9 ⟹ 单点核心 [9,9] 成交（闭区间合法）。
        let ev = m.push_segment(unit(16, 20, up(), 9, 13));
        let center = Center { zd: 9, zg: 9, dd: 5, gg: 13, start_index: 8, end_index: 20 };
        assert_eq!(
            ev,
            Some(CenterLifecycleEvent::Born { level: 0, center, born_seg_ordinal: 5 }),
            "滑窗 (c,d,e) 成交 ⟹ born，出生段号 = 第 5 段（完成段）"
        );
        assert_eq!(m.counts(), (1, 0, 0));
    }

    /// ★无重叠不出生：方向交替但三段两两分离 ⟹ 永不出生；同向三段（无方向交替）L0 完整判据拒绝。
    #[test]
    fn no_overlap_no_born() {
        let mut m = CenterEventMachine::new(0);
        // 方向交替但两两分离：zd=max(0,10,20)=20 > zg=min(4,14,24)=4 ⟹ 核心空。
        assert_eq!(m.push_segment(unit(0, 4, up(), 0, 4)), None);
        assert_eq!(m.push_segment(unit(4, 8, down(), 10, 14)), None);
        assert_eq!(m.push_segment(unit(8, 12, up(), 20, 24)), None, "三段两两分离 ⟹ 无 born");
        assert_eq!(m.alive_center(), None);
        assert_eq!(m.counts(), (0, 0, 0));
        assert_eq!(m.segments_since_reset(), 3, "未出生段序列保留计数（供滑窗）");

        // 同向三段（无方向交替）⟹ L0 完整判据拒绝（即使核心非空）。
        let mut m2 = CenterEventMachine::new(0);
        assert_eq!(m2.push_segment(unit(0, 4, up(), 10, 20)), None);
        assert_eq!(m2.push_segment(unit(4, 8, up(), 18, 25)), None);
        assert_eq!(m2.push_segment(unit(8, 12, up(), 18, 22)), None, "无方向交替 ⟹ L0 完整判据拒绝");
        assert_eq!(m2.counts(), (0, 0, 0));

        // 上级（ℓ≥1）几何判据不查方向：同构同向三段 ⟹ 几何成交 born（判据分域见证）。
        let mut m3 = CenterEventMachine::new(1);
        assert_eq!(m3.push_segment(unit(0, 4, up(), 10, 20)), None);
        assert_eq!(m3.push_segment(unit(4, 8, up(), 18, 25)), None);
        let ev = m3.push_segment(unit(8, 12, up(), 18, 22));
        assert!(
            matches!(ev, Some(CenterLifecycleEvent::Born { level: 1, born_seg_ordinal: 3, .. })),
            "ℓ≥1 几何判据不查方向交替 ⟹ born（与 L0 判据分域，center.rs 有效域）"
        );
    }

    // ──────────────────────────────────────────────────────────────────────
    //  破坏（三类买卖点 → broken）
    // ──────────────────────────────────────────────────────────────────────

    /// ★三类点破在场中枢：born 后确认 buy3 ⟹ Broken（含死中枢/出生段号/触发坐标/方向）；
    /// 段序列**不清零**（趋势延续，新中枢自后续段滑窗出生，出生段号连续计数）。
    #[test]
    fn broken_by_third_class_point() {
        let mut m = CenterEventMachine::new(0);
        m.push_segment(unit(0, 4, up(), 10, 20));
        m.push_segment(unit(4, 8, down(), 12, 20));
        let born = m.push_segment(unit(8, 12, up(), 12, 22));
        let center = Center { zd: 12, zg: 20, dd: 10, gg: 22, start_index: 0, end_index: 12 };
        assert!(matches!(born, Some(CenterLifecycleEvent::Born { .. })));

        // 三类买点确认 ⟹ 在场中枢死亡。
        let ev = m.push_point(bits_3b(), 100);
        assert_eq!(
            ev,
            Some(CenterLifecycleEvent::Broken {
                level: 0,
                center,
                born_seg_ordinal: 3,
                breaker_source_index: 100,
                breaker_side: Side::Long,
            }),
            "三类买点 ⟹ broken（段序列不清零）"
        );
        assert_eq!(m.alive_center(), None, "破坏后无在场中枢");
        assert_eq!(m.counts(), (1, 1, 0));
        assert_eq!(m.segments_since_reset(), 3, "三类破坏不清段序列");

        // 无在场中枢时三类点不产事件（不杀不存在的中枢，诚实 no-op）。
        assert_eq!(m.push_point(bits_3s(), 200), None, "无在场中枢 ⟹ 三类点无事件");
        assert_eq!(m.counts(), (1, 1, 0));

        // 破坏后续段滑窗再出生：段号连续（不清零），第 6 段完成新重叠。
        assert_eq!(m.push_segment(unit(12, 16, down(), 30, 40)), None);
        assert_eq!(m.push_segment(unit(16, 20, up(), 32, 42)), None);
        let ev2 = m.push_segment(unit(20, 24, down(), 31, 41));
        // (d,e,f)：方向 down/up/down 交替；zd=max(30,32,31)=32 ≤ zg=min(40,42,41)=40 ⟹ 成交。
        assert_eq!(
            ev2,
            Some(CenterLifecycleEvent::Born {
                level: 0,
                center: Center { zd: 32, zg: 40, dd: 30, gg: 42, start_index: 12, end_index: 24 },
                born_seg_ordinal: 6,
            }),
            "破坏后段序列连续 ⟹ 新中枢出生段号 = 6（不清零）"
        );
        assert_eq!(m.counts(), (2, 1, 0));
    }

    // ──────────────────────────────────────────────────────────────────────
    //  一类点同死（本级一类点 → 段序列清零 + 在场中枢死亡，新中枢不从旧段拼出）
    // ──────────────────────────────────────────────────────────────────────

    /// ★一类卖点同死：born 后确认 sell1 ⟹ Reset（段序列清零 + 在场中枢同死）；
    /// 之后只喂 2 段不出生（即使旧段若参与可成交——新中枢**不从旧段拼出**）；
    /// 喂满全新 3 段 ⟹ 新 born，出生段号重新从 3 计数。
    #[test]
    fn reset_by_first_class_point_kills_segments_and_center() {
        let mut m = CenterEventMachine::new(0);
        m.push_segment(unit(0, 4, up(), 10, 20));
        m.push_segment(unit(4, 8, down(), 12, 20));
        m.push_segment(unit(8, 12, up(), 12, 22)); // born（段 3）
        let center = Center { zd: 12, zg: 20, dd: 10, gg: 22, start_index: 0, end_index: 12 };
        // 在场期间第 4 段（旧走势段，若与新段拼可成交的诱饵）。
        assert_eq!(m.push_segment(unit(12, 16, down(), 30, 40)), None);
        assert_eq!(m.segments_since_reset(), 4);

        // 本级一类卖点确认 ⟹ 段序列与在场中枢同死。
        let ev = m.push_point(bits_1s(), 100);
        assert_eq!(
            ev,
            Some(CenterLifecycleEvent::Reset {
                level: 0,
                died_center: Some(center),
                died_born_seg_ordinal: Some(3),
                cleared_segments: 4,
                trigger_source_index: 100,
                trigger_side: Side::Short,
            }),
            "一类卖点 ⟹ reset（段序列清零 + 在场中枢同死）"
        );
        assert_eq!(m.alive_center(), None);
        assert_eq!(m.segments_since_reset(), 0, "段计数清零");
        assert_eq!(m.counts(), (1, 0, 1));

        // 新走势类型只来 2 段：与旧第 4 段 [30,40]down 若拼 (d,e,f) 本可成交
        // （zd=max(30,32,31)=32 ≤ zg=min(40,42,41)=41），但旧段已死 ⟹ 不得参与。
        assert_eq!(m.push_segment(unit(16, 20, up(), 32, 42)), None, "新序列第 1 段");
        assert_eq!(m.push_segment(unit(20, 24, down(), 31, 41)), None, "新序列第 2 段——不从旧段拼出");
        assert_eq!(m.counts(), (1, 0, 1), "2 段不出生（旧段禁拼）");
        // 全新第 3 段完成重叠 ⟹ 新 born，段号重新计数 = 3。
        let ev2 = m.push_segment(unit(24, 28, up(), 33, 43));
        assert_eq!(
            ev2,
            Some(CenterLifecycleEvent::Born {
                level: 0,
                center: Center { zd: 33, zg: 41, dd: 31, gg: 43, start_index: 16, end_index: 28 },
                born_seg_ordinal: 3,
            }),
            "全新 3 段 ⟹ 新中枢出生，段号重新计数"
        );
        assert_eq!(m.counts(), (2, 0, 1));
    }

    /// ★无在场中枢时一类点仍清零段序列（同死语义覆盖段序列本身）。
    #[test]
    fn reset_without_alive_center_still_clears_segments() {
        let mut m = CenterEventMachine::new(0);
        m.push_segment(unit(0, 4, up(), 0, 4));
        m.push_segment(unit(4, 8, down(), 10, 14));
        assert_eq!(m.segments_since_reset(), 2);
        let ev = m.push_point(bits_1b(), 50);
        assert_eq!(
            ev,
            Some(CenterLifecycleEvent::Reset {
                level: 0,
                died_center: None,
                died_born_seg_ordinal: None,
                cleared_segments: 2,
                trigger_source_index: 50,
                trigger_side: Side::Long,
            }),
            "无在场中枢 ⟹ Reset 仍清零段序列（died=None）"
        );
        assert_eq!(m.segments_since_reset(), 0);
        assert_eq!(m.counts(), (0, 0, 1));
        // 一类+三类同点（防御：理论互斥）：一类优先，同死吞没破坏。
        let mut m2 = CenterEventMachine::new(0);
        m2.push_segment(unit(0, 4, up(), 10, 20));
        m2.push_segment(unit(4, 8, down(), 12, 20));
        m2.push_segment(unit(8, 12, up(), 12, 22));
        let both = BspBits { buy1: true, buy3: true, ..Default::default() };
        let ev2 = m2.push_point(both, 60);
        assert!(
            matches!(ev2, Some(CenterLifecycleEvent::Reset { .. })),
            "一类+三类同点 ⟹ 一类优先（同死吞没破坏）"
        );
        assert_eq!(m2.counts(), (1, 0, 1));
    }

    /// ★二类点不产事件（不在 born/broken/reset 三类教义事件内）。
    #[test]
    fn second_class_point_produces_no_event() {
        let mut m = CenterEventMachine::new(0);
        m.push_segment(unit(0, 4, up(), 10, 20));
        m.push_segment(unit(4, 8, down(), 12, 20));
        m.push_segment(unit(8, 12, up(), 12, 22));
        assert_eq!(m.push_point(bits_2b(), 70), None, "二类点不产事件");
        assert_eq!(m.counts(), (1, 0, 0));
        assert!(m.alive_center().is_some(), "二类点不动在场中枢");
    }
}
