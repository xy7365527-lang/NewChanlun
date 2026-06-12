//! CenterBook — 中枢生命周期账本（市场性质，跨 trade 持续）。
//!
//! `ingest` 逐字移植 Python `run_organic` 主循环的中枢账本段（last_center /
//! dead_centers / frozen / center_version，含 not-in 守卫的版本号语义）。
//!
//! v2 增量（C6/D2）：ingest 同时在**唯一 diff 点**派生 `CenterEvent` 三态流——
//! 多消费者各自 diff 是分歧温床（C6 候选 D 的否定论证）。数据源边界声明见
//! `types::CenterEvent` docstring（BSP 事件锚派生，非信号层 zhongshus diff）。

use std::collections::HashSet;

use super::types::*;
use crate::buysellpoint::{BspKind, Side};
use crate::stroke::Direction;

/// H2 力度收敛门的三态判据读数（49课行38；`up_strength_verdict`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpStrengthVerdict {
    /// 向上离开段力度历史 <2 条——无"震荡依旧"证据，保守拒开。
    Newborn,
    /// 最近一次力度 > 前一次——扩张，三类点预警，拒开。
    Expanding,
    /// 最近 ≤ 前次——"中枢震荡逐步收敛"成立，放行。
    Converged,
}

/// 存活中枢快照（账本视角的"最后已知中枢"）。
#[derive(Debug, Clone, Copy)]
pub struct LiveCenter {
    pub seg_start: i64,
    pub zd: f64,
    pub zg: f64,
}

#[derive(Debug, Default)]
pub struct CenterBook {
    /// ladder → 最后已知中枢（Python `last_center: dict[int, tuple]`）。
    last: [Option<LiveCenter>; MAX_LADDER],
    /// ladder → 死亡中枢 seg_start 集（Python `dead_centers`）。
    dead: [Option<HashSet<i64>>; MAX_LADDER],
    /// ladder → 向下终结（confirmed Sell3 = 三卖）死亡中枢 seg_start 子集
    /// （49课严格形式 osc_sell3_no_recover 的方向判据；首杀方向）。
    dead_down: [Option<HashSet<i64>>; MAX_LADDER],
    /// ladder → 冻结中枢 seg_start（Python `frozen`；hard_type3 buy 置位）。
    frozen: [Option<i64>; MAX_LADDER],
    /// ladder → 未决离开段 (锚中枢 seg_start, 离开方向 side)——49课禁令窗口
    /// （H1 candidate 冻结，2026-06-12 任务）。
    ///
    /// 49课行52："中枢完成后的向上移动时的差价是不能做的"；行68 给出当下
    /// 判据——次级别走势**离开**中枢（candidate 三类买卖点出现）即启动
    /// "向上移动"语义，不需要等回抽确认（confirmed type3）。candidate
    /// type3 事件置位本窗口；窗口由价格回中枢否定（`negate_pending_departure`，
    /// 49课行52 前提"前提是中枢震荡依旧"的对称否定——回试跌回边界内 =
    /// 仍是中枢震荡）、中枢死亡（confirmed type3，现行 frozen/dead 语义
    /// 接管）或新中枢形成（cs 变化）解除。每个新离开段（新 seg_idx 的
    /// candidate 事件）重新置位——逐段窗口语义。
    pending_departure: [Option<(i64, Side)>; MAX_LADDER],
    /// H2（osc_strength_gate，49课行38）：未决**向上**离开段窗口内的运行
    /// max excursion（max(c) − 当时 ZG——力度 = 价格振幅在册口径对离开段
    /// 的直读，零 surfacing）。仅 Side::Buy 窗口有意义；窗口置位时重置为
    /// NEG_INFINITY。中枢延伸时 ZG 取当下值（已推入的历史记录不回溯修订）。
    pending_excursion: [f64; MAX_LADDER],
    /// H2：per-center 向上离开段力度历史 (锚中枢 seg_start, [前次, 最近], 条数)。
    /// 环形容量 2——49课行38"后面的向下离开力度一定比前一个小"是相邻两次
    /// 比较的字面（扩窗即引入参数，须回原文重审——设计预注册边界 (b)）。
    /// **仅价格否定的窗口推入**（回试跌回边界内 = "如果继续是中枢震荡"的
    /// 完成样本）；中枢死亡/新中枢解除的窗口不推入——该离开段终结了中枢，
    /// 不属于"继续是中枢震荡"的序列（行38 判据的参照系内生于震荡序列本身）。
    up_strength: [Option<(i64, [f64; 2], u8)>; MAX_LADDER],
    /// 禁令窗口置位次数（G3 可观测性：candidate 离开段事件数）。
    pub cf_windows: u64,
    /// 禁令窗口价格否定次数（回试跌回边界内解冻数；与 cf_windows 之差 =
    /// 由死亡/新中枢解除或持续到结束的窗口数）。
    pub cf_negations: u64,
    /// 中枢生死/边界事件版本号（SizeAllocator 重算门控）。
    pub version: u64,
}

impl CenterBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// 消费一层的本 bar BSP 事件流。`events_out` 非 None 时收集本层 CenterEvent
    /// （v2 消费者：T4b 加码 / FatigueGate 清空路径(2) / 域腿解冻观测）。
    ///
    /// Python 逐字对应（含分支顺序与版本号自增点）：
    /// ```python
    /// kind, side, confirmed, cs = ev[0], ev[1], ev[3], ev[4]
    /// if cs is None: continue
    /// dead = dead_centers.setdefault(lad, set())
    /// if confirmed and kind == "type3":
    ///     if cs not in dead: dead.add(cs); center_version += 1
    ///     if hard_type3 and side == "buy": frozen[lad] = cs
    /// elif cs not in dead:
    ///     if last_center.get(lad) != (cs, ev[5], ev[6]): center_version += 1
    ///     last_center[lad] = (cs, ev[5], ev[6])
    ///     if lad in frozen and frozen[lad] != cs: del frozen[lad]
    /// ```
    pub fn ingest(
        &mut self,
        ladder: usize,
        evs: &[BspEvent],
        hard_type3: bool,
        mut events_out: Option<&mut Vec<CenterEvent>>,
    ) {
        for ev in evs {
            let Some(cs) = ev.cs else { continue };
            let dead = self.dead[ladder].get_or_insert_with(HashSet::new);
            if ev.confirmed && ev.class.kind() == BspKind::Type3 {
                if !dead.contains(&cs) {
                    dead.insert(cs);
                    if ev.class.side() == Side::Sell {
                        // 三卖终结（向下离开）——49课"不能回补"的方向判据
                        self.dead_down[ladder]
                            .get_or_insert_with(HashSet::new)
                            .insert(cs);
                    }
                    self.version += 1;
                    if let Some(out) = events_out.as_deref_mut() {
                        // Buy3 = 向上离开后回抽不破 ZG → 中枢向上终结；Sell3 反之。
                        let direction = match ev.class.side() {
                            Side::Buy => Direction::Up,
                            Side::Sell => Direction::Down,
                        };
                        out.push(CenterEvent::Terminated { seg_start: cs, direction });
                    }
                }
                if hard_type3 && ev.class.side() == Side::Buy {
                    self.frozen[ladder] = Some(cs);
                }
                // H1：中枢死亡 ⇒ 该中枢的未决离开段窗口解除（confirmed
                // type3 的 dead/frozen 语义接管，窗口对象已不存在）。
                if self.pending_departure[ladder].is_some_and(|(p, _)| p == cs) {
                    self.pending_departure[ladder] = None;
                }
            } else if !dead.contains(&cs) {
                // Python 元组比较 (cs, zd, zg)；zd/zg 为 Option<f64>，
                // Python None==None 与 float== 语义由 Option<f64> 等值精确对应。
                let prev = self.last[ladder];
                let changed = match prev {
                    None => true,
                    Some(lc) => {
                        lc.seg_start != cs
                            || Some(lc.zd) != ev.zd
                            || Some(lc.zg) != ev.zg
                    }
                };
                if changed {
                    self.version += 1;
                    if let Some(out) = events_out.as_deref_mut() {
                        // cs 变化 = Formed（新中枢）；同 cs 边界更新 = Extended。
                        match prev {
                            Some(lc) if lc.seg_start == cs => {
                                out.push(CenterEvent::Extended { seg_start: cs });
                            }
                            _ => out.push(CenterEvent::Formed {
                                seg_start: cs,
                                zd: ev.zd.unwrap_or(f64::NAN),
                                zg: ev.zg.unwrap_or(f64::NAN),
                            }),
                        }
                    }
                }
                // Python last_center 存事件原始 zd/zg（可为 None——但 osc 开腿读
                // lc.zd/zg 做算术，None 在 Python 会 TypeError ⇒ 生产磁带上恒非
                // None。fail-fast 同构：None 时存 NaN，算术比较恒 False 显式化。
                self.last[ladder] = Some(LiveCenter {
                    seg_start: cs,
                    zd: ev.zd.unwrap_or(f64::NAN),
                    zg: ev.zg.unwrap_or(f64::NAN),
                });
                if let Some(f) = self.frozen[ladder] {
                    if f != cs {
                        self.frozen[ladder] = None;
                    }
                }
                // H1（49课行68）：candidate type3 = 次级别走势离开中枢——
                // 禁令窗口置位。confirmed type3 不走本分支（上方 kill 分支），
                // 故此处 kind==Type3 必为 candidate。每个新离开段（事件流
                // 按 (kind,side,seg_idx,confirmed) 去重，新 seg_idx 重发）
                // 重新置位窗口。
                if ev.class.kind() == BspKind::Type3 {
                    self.pending_departure[ladder] = Some((cs, ev.class.side()));
                    // H2：新窗口 excursion 归零位（运行 max 从无穷小起，
                    // 同 bar 的 negate 驱动即折入当 bar close）。
                    self.pending_excursion[ladder] = f64::NEG_INFINITY;
                    self.cf_windows += 1;
                }
                // H1：新中枢形成（cs 变化）⇒ 旧中枢的未决离开段窗口解除
                // （frozen 解除同构——窗口挂在锚中枢上，锚已被覆盖）。
                else if self.pending_departure[ladder].is_some_and(|(p, _)| p != cs) {
                    self.pending_departure[ladder] = None;
                }
            }
        }
    }

    /// H1 禁令窗口的价格否定（每 bar 驱动，市场性质——与持仓/配置无关）。
    ///
    /// 49课行52 的前提是"中枢震荡依旧"：向上离开段（candidate Buy3）在
    /// 价格回到 ZG 之下时被否定（回试跌回中枢 = 仍是中枢震荡，三买不成立
    /// ——38课答疑"能回到中枢就不是第三类买点"）；向下离开段（candidate
    /// Sell3）对称地在价格回到 ZD 之上时被否定。边界取 last 中枢的当前
    /// 边界（中枢延伸时随之更新）。NaN 边界比较恒 false ⇒ 不否定（保守
    /// 方向，与 osc 开腿 NaN 语义同构——生产磁带 zd/zg 恒非 None）。
    pub fn negate_pending_departure(&mut self, ladder: usize, c: f64) {
        let Some((cs, side)) = self.pending_departure[ladder] else { return };
        let Some(lc) = self.last[ladder] else { return };
        debug_assert_eq!(
            lc.seg_start, cs,
            "pending_departure 与 last 中枢不一致——ingest 的清除路径有缺口"
        );
        // H2：向上离开段力度观测（excursion = max(c) − 当时 ZG）。NaN ZG
        // 比较恒 false ⇒ 不更新（同窗口否定的 NaN 语义——生产磁带恒非 None）。
        if side == Side::Buy {
            let exc = c - lc.zg;
            if exc > self.pending_excursion[ladder] {
                self.pending_excursion[ladder] = exc;
            }
        }
        let negated = match side {
            Side::Buy => c < lc.zg,
            Side::Sell => c > lc.zd,
        };
        if negated {
            // H2：价格否定 = 回试跌回边界内 = "继续是中枢震荡"——本次向上
            // 离开段完成，力度推入 per-center 历史。Buy 侧否定（c < ZG）必经
            // 上方 excursion 更新 ⇒ 推入值恒有限（最差为本 bar 的 c − ZG < 0，
            // 设计预注册边界 (a) 的"单 bar 越界"退化形态，按原值记录）。
            if side == Side::Buy {
                self.push_up_strength(ladder, cs, self.pending_excursion[ladder]);
            }
            self.pending_departure[ladder] = None;
            self.cf_negations += 1;
        }
    }

    /// H2：向上离开段力度推入 per-center 环形历史（容量 2）。锚中枢变更
    /// （cs 不匹配）即重开历史——历史与中枢同生命周期，无跨中枢继承。
    fn push_up_strength(&mut self, ladder: usize, cs: i64, strength: f64) {
        match &mut self.up_strength[ladder] {
            Some((s, hist, len)) if *s == cs => {
                if *len < 2 {
                    hist[*len as usize] = strength;
                    *len += 1;
                } else {
                    hist[0] = hist[1];
                    hist[1] = strength;
                }
            }
            slot => *slot = Some((cs, [strength, 0.0], 1)),
        }
    }

    /// H2 力度收敛门判据（osc_strength_gate，开腿时刻只读——不等任何未来
    /// 事件，49课行52"用中枢震荡力度判断的方法，完全可以避开"的当下形式）：
    /// - 该锚中枢向上力度历史 <2 条 ⇒ Newborn（新生保守默认：行38"逐步
    ///   收敛"语义下无"震荡依旧"证据）；
    /// - 最近一次 > 前一次 ⇒ Expanding（行38 扩张 ⇒ 三类点预警）；
    /// - 最近 ≤ 前次 ⇒ Converged（放行）。
    pub fn up_strength_verdict(&self, ladder: usize, cs: i64) -> UpStrengthVerdict {
        match self.up_strength[ladder] {
            Some((s, hist, len)) if s == cs && len >= 2 => {
                if hist[1] > hist[0] {
                    UpStrengthVerdict::Expanding
                } else {
                    UpStrengthVerdict::Converged
                }
            }
            _ => UpStrengthVerdict::Newborn,
        }
    }

    /// H1 禁令窗口查询：该层存在未被否定的 candidate 离开段（49课行52
    /// "中枢完成后的向上移动时的差价是不能做的"——窗口内 osc 不开腿）。
    pub fn has_pending_departure(&self, ladder: usize) -> bool {
        self.pending_departure[ladder].is_some()
    }

    /// 该层最后已知中枢（Python `last_center.get(k)`——注意：**不查 dead**，
    /// 存活判定由调用方组合 `is_dead`，与 Python 调用面逐字一致）。
    pub fn last(&self, ladder: usize) -> Option<LiveCenter> {
        self.last[ladder]
    }

    /// 该层当前存活中枢（last 存在 ∧ 不在 dead）。
    pub fn alive(&self, ladder: usize) -> Option<LiveCenter> {
        let lc = self.last[ladder]?;
        if self.is_dead(ladder, lc.seg_start) {
            None
        } else {
            Some(lc)
        }
    }

    pub fn is_dead(&self, ladder: usize, seg_start: i64) -> bool {
        self.dead[ladder].as_ref().is_some_and(|d| d.contains(&seg_start))
    }

    /// 该中枢是否被三卖（confirmed Sell3，向下离开）终结。49课严格形式的
    /// 方向判据："一旦出现第三类卖点，就不能回补了"只覆盖向下终结——
    /// 三买（向上终结）按"中枢向上移动时就应该满仓"必须立即回补。
    pub fn is_dead_down(&self, ladder: usize, seg_start: i64) -> bool {
        self.dead_down[ladder].as_ref().is_some_and(|d| d.contains(&seg_start))
    }

    pub fn is_frozen(&self, ladder: usize) -> bool {
        self.frozen[ladder].is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(class: BspClass, confirmed: bool, cs: i64, zd: f64, zg: f64) -> BspEvent {
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
    fn formed_extended_terminated_lifecycle() {
        let mut book = CenterBook::new();
        let mut out = Vec::new();
        // 新中枢 → Formed + version+1
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, Some(&mut out));
        assert_eq!(book.version, 1);
        assert!(matches!(out[0], CenterEvent::Formed { seg_start: 10, .. }));
        assert!(book.alive(2).is_some());
        // 同 cs 边界更新 → Extended
        out.clear();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.5)], true, Some(&mut out));
        assert!(matches!(out[0], CenterEvent::Extended { seg_start: 10 }));
        // 同 cs 同边界 → 无事件无版本号
        out.clear();
        let v = book.version;
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.5)], true, Some(&mut out));
        assert!(out.is_empty());
        assert_eq!(book.version, v);
        // confirmed Buy3 → Terminated{Up} + 冻结
        out.clear();
        book.ingest(2, &[ev(BspClass::Buy3, true, 10, 1.0, 2.5)], true, Some(&mut out));
        assert!(matches!(
            out[0],
            CenterEvent::Terminated { seg_start: 10, direction: Direction::Up }
        ));
        assert!(book.is_frozen(2));
        assert!(book.alive(2).is_none()); // last 仍在但已死
        // 新中枢出现 → 解冻 + Formed
        out.clear();
        book.ingest(2, &[ev(BspClass::Sell1, true, 20, 3.0, 4.0)], true, Some(&mut out));
        assert!(!book.is_frozen(2));
        assert!(matches!(out[0], CenterEvent::Formed { seg_start: 20, .. }));
    }

    #[test]
    fn dead_center_events_ignored() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Buy3, true, 10, 1.0, 2.0)], true, None);
        let v = book.version;
        // 死中枢的后续事件不更新 last（Python elif cs not in dead）
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        assert_eq!(book.version, v);
        assert!(book.last(2).is_none());
    }

    #[test]
    fn candidate_type3_does_not_kill() {
        let mut book = CenterBook::new();
        // candidate type3 走 elif 分支（确认才杀）——Python `confirmed and kind==type3`
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        assert!(book.alive(2).is_some());
        assert!(!book.is_frozen(2));
    }

    #[test]
    fn h1_candidate_departure_window_lifecycle() {
        let mut book = CenterBook::new();
        // 中枢形成 → 无窗口
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        assert!(!book.has_pending_departure(2));
        // candidate Buy3（向上离开）→ 窗口置位（49课行68）
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        assert!(book.has_pending_departure(2));
        assert_eq!(book.cf_windows, 1);
        // 价格仍在 ZG 之上 → 窗口保持
        book.negate_pending_departure(2, 2.5);
        assert!(book.has_pending_departure(2));
        // 回试跌回 ZG 下 → 否定解冻（"能回到中枢就不是第三类买点"）
        book.negate_pending_departure(2, 1.9);
        assert!(!book.has_pending_departure(2));
        assert_eq!(book.cf_negations, 1);
        // 新离开段（新 seg_idx 的 candidate 事件）→ 重新置位
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        assert!(book.has_pending_departure(2));
        // confirmed Buy3（中枢死亡）→ 窗口解除，dead/frozen 语义接管
        book.ingest(2, &[ev(BspClass::Buy3, true, 10, 1.0, 2.0)], true, None);
        assert!(!book.has_pending_departure(2));
        assert!(book.is_frozen(2));
    }

    #[test]
    fn h1_sell_side_departure_negated_above_zd() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // candidate Sell3（向下离开）→ 窗口置位
        book.ingest(2, &[ev(BspClass::Sell3, false, 10, 1.0, 2.0)], true, None);
        assert!(book.has_pending_departure(2));
        // 价格仍在 ZD 之下 → 保持
        book.negate_pending_departure(2, 0.8);
        assert!(book.has_pending_departure(2));
        // 回到 ZD 之上 → 否定解冻
        book.negate_pending_departure(2, 1.2);
        assert!(!book.has_pending_departure(2));
    }

    #[test]
    fn h2_up_strength_lifecycle() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // 零历史 → Newborn（新生保守默认）
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Newborn);
        // 第一次向上离开：窗口内 excursion 峰值 0.5（c=2.5），价格否定推入
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 2.5);
        book.negate_pending_departure(2, 1.9); // 否定 → 推入 0.5
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Newborn); // 仅1条
        // 第二次离开力度 0.3 < 0.5 → 收敛放行
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 2.3);
        book.negate_pending_departure(2, 1.8);
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Converged);
        // 第三次离开力度 0.9 > 0.3（环形最近两次比较）→ 扩张拒开
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 2.9);
        book.negate_pending_departure(2, 1.5);
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Expanding);
    }

    #[test]
    fn h2_excursion_uses_current_zg() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 3.0); // exc = 1.0（ZG=2.0）
        // 中枢延伸 ZG → 2.5：后续 excursion 相对当下 ZG（当下性声明）
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.5)], true, None);
        book.negate_pending_departure(2, 3.2); // exc = 0.7 < 1.0 不更新
        book.negate_pending_departure(2, 2.4); // c < 2.5 否定 → 推入 1.0
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.5)], true, None);
        book.negate_pending_departure(2, 3.4); // exc = 0.9 < 1.0
        book.negate_pending_departure(2, 2.0); // 推入 0.9 → 收敛
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Converged);
    }

    #[test]
    fn h2_death_dismissed_window_not_recorded() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 2.5);
        // confirmed Buy3 杀中枢 → 窗口解除但不推入（离开段终结中枢，
        // 不是"继续是中枢震荡"的样本）
        book.ingest(2, &[ev(BspClass::Buy3, true, 10, 1.0, 2.0)], true, None);
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Newborn);
    }

    #[test]
    fn h2_new_center_resets_history() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        for c_peak in [2.9, 2.3] {
            book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
            book.negate_pending_departure(2, c_peak);
            book.negate_pending_departure(2, 1.5);
        }
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Converged);
        // 新中枢（cs=20）→ 历史与中枢同生命周期，新锚查询回 Newborn
        book.ingest(2, &[ev(BspClass::Sell1, true, 20, 3.0, 4.0)], true, None);
        assert_eq!(book.up_strength_verdict(2, 20), UpStrengthVerdict::Newborn);
    }

    #[test]
    fn h2_sell_side_window_not_recorded() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        // 向下离开窗口（candidate Sell3）价格否定——不进向上力度历史
        // （设计 §2 方向声明：只比较向上离开段序列）
        book.ingest(2, &[ev(BspClass::Sell3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 0.5);
        book.negate_pending_departure(2, 1.2);
        book.ingest(2, &[ev(BspClass::Sell3, false, 10, 1.0, 2.0)], true, None);
        book.negate_pending_departure(2, 0.7);
        book.negate_pending_departure(2, 1.3);
        assert_eq!(book.up_strength_verdict(2, 10), UpStrengthVerdict::Newborn);
    }

    #[test]
    fn h1_new_center_clears_window() {
        let mut book = CenterBook::new();
        book.ingest(2, &[ev(BspClass::Sell1, true, 10, 1.0, 2.0)], true, None);
        book.ingest(2, &[ev(BspClass::Buy3, false, 10, 1.0, 2.0)], true, None);
        assert!(book.has_pending_departure(2));
        // 新中枢形成（cs 变化）→ 旧锚窗口解除
        book.ingest(2, &[ev(BspClass::Sell1, true, 20, 3.0, 4.0)], true, None);
        assert!(!book.has_pending_departure(2));
    }
}
