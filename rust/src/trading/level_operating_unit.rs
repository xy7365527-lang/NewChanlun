//! LevelOperatingUnit — 38课操盘程式 FSM（voice 实例），v2 转换表 T1-T7（§5.1）。
//!
//! ## v2 矛盾修正的代码落点
//! - **C2 osc 正交**：osc 腿不在任何相位变体内——相位转换代码路径不触碰 osc，
//!   "REV 开腿截断 osc"在 v2 没有代码位（v1 的 M-b 截断 bug 类型层根治）。
//!   osc 闭腿条件 = P5 原版（触 ZD / 锚中枢死亡 / master 强平），T3 相位限制取消。
//! - **C3 G1 锚定**：REV 开腿守卫枚举化（RevOpenVerdict 拒因分计数），求值顺序
//!   即拒因优先序 G1 → G2 → G3a → G3b。
//! - **C4 tranche 递归建仓**：RevLeg.tranches 按 level 升序 append-only；
//!   加码事件 T4b（a/b/c 三类）；每级别至多一个开放 tranche。
//! - **C5 区间套级别读出**：`nesting_buy_level` 纯函数（零状态机——确认级别是
//!   磁带的每 bar 纯函数）；T5 逐级回补 close_upto（前缀截断）。
//! - **C6 穷举义务**：触发谓词全部 match BspClass 六变体——漏一类编译不过。
//!
//! 同 bar 优先级（§5.1）：T7 > T6 > T5/T5b > T1/T4b > T3/T4。
//!
//! ## T4b 实现注记（设计精度边界的诚实声明，090号）
//! v2 设计给出 (a)/(b)/(c) 三类加码事件但未给 (a)/(b) 的重复触发语义与 tranche
//! 目标级别的显式公式。本实现的解读（doc 注释逐点标注，预注册判据2 的验证对象）：
//!   (a) 反向 run 内**首个** Formed → 加码 level = 当前最高+1（结构确认一次）；
//!   (b) Terminated{Down} 后同 run **第二个** Formed → 再加一级（趋势确认一次）；
//!   (c) ℓ′ > 当前最高且 dir_row[ℓ′]==Down ∧ run_anchor[ℓ′] ≥ open_bar−margin
//!       → 加码 level = ℓ′（级别涌现直读）。
//! 全部 T4b 路径依赖 D3 磁带行（dir_row/run_anchor）——行缺失时 `tranche=true`
//! 配置在 runner 入口被 capability guard 拒绝。

use super::center_book::CenterBook;
use super::config::{OrganicConfig, RevClose, SubAnchor};
use super::fatigue_gate::FatigueGate;
use super::ledger::OrganicLedger;
use super::types::*;
use crate::buysellpoint::BspKind;
use crate::divergence::DivKind;
use crate::stroke::Direction;

/// 38课两段韵律（v1 RIDE/REV 更名，语义同）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoicePhase {
    UpLeg,
    DownLeg,
}

/// 单个 REV tranche：反向走势在某级别获得结构确认时开出的一份规模。
/// 股数/卖价记账在账本（SlotKey::rev(home, level) 槽）；本结构只持级别坐标。
#[derive(Debug, Clone, Copy)]
pub struct RevTranche {
    /// 确认级别。首 tranche = home；加码单调递增（级别涌现 append-only）。
    pub level: usize,
    /// T5b 防悬挂：该级别上一 bar 的方向行（翻 Up 检测需要前值）。
    pub last_dir: Option<Direction>,
}

/// REV 腿：home 声部的反向段操作（先卖后买），tranche 化（C4）。
#[derive(Debug, Clone)]
pub struct RevLeg {
    /// 按 level 升序 append-only；close_upto 只截断前缀（C4 不变式）。
    pub tranches: Vec<RevTranche>,
    pub open_bar: i64,
    /// rev_paired：开腿信号类型标注（任务要求"震荡型/逃逸型"可观测）。
    /// None = legacy 腿（段终结三触发开，无 kind 概念——声明=能力）。
    pub open_kind: Option<RevOpenKind>,
    /// rev_paired：开腿锚中枢 seg_start（震荡型=存活中枢快照；逃逸型=被
    /// 终结中枢——T5 同锚判定的比较基准）。legacy 腿恒 None。
    pub anchor_cs: Option<i64>,
    /// rev_paired：ZD 触线回补价。仅震荡型携带——逃逸型开腿时价格已在
    /// 中枢下方，"触线"在开腿 bar 即真，作为闭腿条件会退化为零深度 churn。
    pub zd: Option<f64>,
    // ── T4b (a)/(b) 一次性触发标记（模块 docstring 解读注记）──
    structure_added: bool,
    terminated_down_seen: bool,
    trend_added: bool,
}

impl RevLeg {
    fn new(home: usize, bar: i64, dir: Option<Direction>) -> Self {
        RevLeg {
            tranches: vec![RevTranche { level: home, last_dir: dir }],
            open_bar: bar,
            open_kind: None,
            anchor_cs: None,
            zd: None,
            structure_added: false,
            terminated_down_seen: false,
            trend_added: false,
        }
    }

    fn new_paired(
        home: usize,
        bar: i64,
        dir: Option<Direction>,
        kind: RevOpenKind,
        anchor_cs: Option<i64>,
        zd: Option<f64>,
    ) -> Self {
        RevLeg { open_kind: Some(kind), anchor_cs, zd, ..RevLeg::new(home, bar, dir) }
    }

    pub fn max_level(&self) -> usize {
        self.tranches.last().map(|t| t.level).expect("RevLeg 不变式：tranches 非空")
    }

    /// 逐级回补：移除全部 level ≤ confirm 的 tranche，返回被关闭者（level 升序）。
    pub fn close_upto(&mut self, confirm: usize) -> Vec<RevTranche> {
        let keep: Vec<RevTranche> =
            self.tranches.iter().copied().filter(|t| t.level > confirm).collect();
        let closed: Vec<RevTranche> =
            self.tranches.iter().copied().filter(|t| t.level <= confirm).collect();
        self.tranches = keep;
        closed
    }
}

/// REV 开腿守卫求值结果（C3：bool 合取扁平化丢拒因——枚举化使拒绝携带原因）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevOpenVerdict {
    Open,
    /// G1：次级别锚定不成立。
    RejectedSubAnchor,
    /// G2：41课门关。
    RejectedGate,
    /// G3a：本级别冻结。
    RejectedFrozen,
}

/// 每 bar 各层行视图（runner 预组装；LOU 不持磁带引用）。
pub struct BarRows<'a> {
    pub evs: &'a [Vec<BspEvent>],
    pub devs: &'a [Vec<DivEvent>],
    pub buy_any: LadderMask,
    pub sell_any: LadderMask,
    /// D3 行（当前磁带 None；tranche/sub_anchor/T5b 的数据依赖）。
    pub dir_row: Option<&'a [Option<Direction>; MAX_LADDER]>,
    pub run_anchor: Option<&'a [i64; MAX_LADDER]>,
}

impl BarRows<'_> {
    fn dir(&self, k: usize) -> Option<Direction> {
        self.dir_row.and_then(|row| row[k])
    }
}

/// 声部操盘单元。osc 槽占用与否在账本（单一真相源）；本结构只持相位 + REV 腿。
#[derive(Debug)]
pub struct VoiceUnit {
    pub ladder: usize,
    pub phase: VoicePhase,
    /// 与 phase 的运行时不变量（debug_assert）：UpLeg ⇒ rev.is_none()。
    /// 不放进 DownLeg 变体：tranche 逐级闭合与相位转换非同步（C4/C5）。
    pub rev: Option<RevLeg>,
}

impl VoiceUnit {
    pub fn new(ladder: usize) -> Self {
        VoiceUnit { ladder, phase: VoicePhase::UpLeg, rev: None }
    }

    // ════════════════════════════════════════════════════════
    // 触发谓词（BspClass 穷举义务，C6）
    // ════════════════════════════════════════════════════════

    /// 38课段终结三触发（K9 零改动 + R2 消融位）：
    /// confirmed Sell1 ∨ DivSell（任意 kind）∨ confirmed Sell3 [∨ confirmed Sell2]。
    pub fn seg_end_trigger(cfg: &OrganicConfig, evs: &[BspEvent], devs: &[DivEvent]) -> bool {
        evs.iter().any(|e| match e.class {
            BspClass::Sell1 | BspClass::Sell3 => e.confirmed,
            // §5.3 矩阵 Sell2 格：默认 no-op（触发密度每增一档 churn 一档，M-a）；
            // 消融轴 R2 显式表态位。
            BspClass::Sell2 => cfg.sell2_trigger && e.confirmed,
            BspClass::Buy1 | BspClass::Buy2 | BspClass::Buy3 => false,
        }) || devs.iter().any(|d| d.side() == crate::buysellpoint::Side::Sell)
    }

    /// T5 单层买侧三岔（38课）：confirmed Buy1（按 rev_close 轴调整）∨ confirmed
    /// Buy2 ∨ DivBuy(consolidation)。DivBuy(trend) no-op：趋势底背驰是 Buy1 的
    /// 引擎前体，等 Buy1 事件本体（§5.3 矩阵）。
    pub(crate) fn buy_trigger_at(
        cfg: &OrganicConfig,
        evs: &[BspEvent],
        devs: &[DivEvent],
        sub_buy: bool,
    ) -> bool {
        evs.iter().any(|e| match e.class {
            BspClass::Buy1 => match cfg.rev_close {
                RevClose::Conf => e.confirmed,
                RevClose::Cand => true,
                // candidate type1 买 ∧ 次级别买点（27课区间套）
                RevClose::Nested => sub_buy,
            },
            BspClass::Buy2 => e.confirmed,
            // Buy3 走 T6/T7 逃逸通道，不入 T5（v1 同序）
            BspClass::Buy3 => false,
            BspClass::Sell1 | BspClass::Sell2 | BspClass::Sell3 => false,
        }) || devs
            .iter()
            .any(|d| d.kind == DivKind::Consolidation && d.direction == Direction::Down)
    }

    /// 区间套确认级别（C5/D1 的全部机制——纯函数，没有状态机）。
    /// emerged = 当前 RevLeg 的 tranche 级别集 ∪ {home}——只在已涌现级别上读，
    /// 未涌现层的事件不构成本腿的关腿证据（§7 级别坐标诚实声明）。
    /// 大级别买点必然伴随小级别买点共现（27课多重背驰嵌套）⇒ max 即收缩读数。
    pub fn nesting_buy_level(
        cfg: &OrganicConfig,
        rows: &BarRows,
        rev: &RevLeg,
    ) -> Option<usize> {
        let mut best: Option<usize> = None;
        for t in &rev.tranches {
            let l = t.level;
            if l >= MAX_LADDER {
                continue;
            }
            let sub_buy = l >= 1 && rows.buy_any.get(l - 1);
            if Self::buy_trigger_at(cfg, &rows.evs[l], &rows.devs[l], sub_buy) {
                best = Some(best.map_or(l, |b: usize| b.max(l)));
            }
        }
        best
    }

    // ════════════════════════════════════════════════════════
    // step：每 bar 一步（仅 Long 态由 runner 调用）
    // ════════════════════════════════════════════════════════

    /// 转换表 T1-T7 顺序 match 块，优先级由代码顺序表达（T6 陷阱纪律）：
    /// T7 > T6 > T5/T5b > T1/T4b > T3/T4。
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        cfg: &OrganicConfig,
        rows: &BarRows,
        center_evs_home: &[CenterEvent],
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        book: &CenterBook,
        gate: &FatigueGate,
        entry_ladder: usize,
        frac_of: &dyn Fn(usize) -> f64,
        counters: &mut Counters,
    ) {
        let k = self.ladder;
        debug_assert!(
            self.phase == VoicePhase::DownLeg || self.rev.is_none(),
            "不变量：UpLeg ⇒ rev 为空"
        );
        let evs = &rows.evs[k];
        let devs = &rows.devs[k];

        // ── DownLeg：T7 > T6 > T5 > T5b（关腿端）──
        if self.phase == VoicePhase::DownLeg && cfg.rev_paired {
            // 配对闭腿（任务 2026-06-10）：闭腿集 = {Buy3 回补, 同锚 confirmed
            // Buy1, ZD 触线}，恰好三条——T5b 防悬挂不在集合内（闭腿集穷举）。
            self.step_down_paired(cfg, rows, c, bar, ledger, counters);
        } else if self.phase == VoicePhase::DownLeg {
            let hard = evs.iter().any(|e| matches!(e.class, BspClass::Buy3) && e.confirmed);
            let pre = cfg.pre_type3
                && evs.iter().any(|e| matches!(e.class, BspClass::Buy3) && !e.confirmed);
            if hard {
                // T7：confirmed Buy3 → 全 tranche 逃逸（冻结由 CenterBook hard_type3 置位）
                self.close_all_tranches(c, bar, ledger, counters);
                counters.n_rev_close_t7 += 1;
                self.phase = VoicePhase::UpLeg;
            } else if pre {
                // T6：candidate Buy3 → 全 tranche 预逃逸（消融轴 E 保留槽，K9）
                self.close_all_tranches(c, bar, ledger, counters);
                counters.n_rev_close_t6 += 1;
                self.phase = VoicePhase::UpLeg;
            } else {
                // T5：区间套级别读出 → 逐级回补
                let confirm = self
                    .rev
                    .as_ref()
                    .and_then(|rev| Self::nesting_buy_level(cfg, rows, rev));
                if let Some(lstar) = confirm {
                    let rev = self.rev.as_mut().expect("DownLeg ⇒ rev 存在");
                    let closed = rev.close_upto(lstar);
                    for t in &closed {
                        ledger.close_diff(SlotKey::rev(k, t.level), c, bar);
                        counters.n_rev_tranche_closes += 1;
                    }
                    counters.n_rev_close_t5 += 1;
                    if rev.tranches.is_empty() {
                        self.rev = None;
                        self.phase = VoicePhase::UpLeg;
                    }
                } else if rows.dir_row.is_some() {
                    // T5b 防悬挂兜底（C5 步骤5-vi）：ℓ 层方向行翻 Up 且无买点事件
                    // 被消费 → 市价回补 level ≤ ℓ。
                    self.t5b_struct_close(rows, c, bar, ledger, counters);
                }
            }
        }

        // ── UpLeg：T1（REV 开腿）/ T2（拒因分计数）──
        if self.phase == VoicePhase::UpLeg && cfg.rev_mode && cfg.rev_paired {
            // 配对开腿（任务 2026-06-10）：kind 展开 + 深度门槛。
            self.rev_paired_open(cfg, rows, book, gate, entry_ladder, c, bar, ledger, frac_of, counters);
        } else if self.phase == VoicePhase::UpLeg
            && cfg.rev_mode
            && (!evs.is_empty() || !devs.is_empty())
            && Self::seg_end_trigger(cfg, evs, devs)
        {
            counters.n_rev_attempts += 1;
            match Self::rev_open_verdict(cfg, k, rows, book, gate, entry_ladder) {
                RevOpenVerdict::RejectedSubAnchor => counters.n_rev_sub_anchor_rejects += 1,
                RevOpenVerdict::RejectedGate => counters.n_rev_gate_rejects += 1,
                RevOpenVerdict::RejectedFrozen => counters.n_rev_frozen_rejects += 1,
                RevOpenVerdict::Open => {
                    // C2：osc 不动——此处没有也不可能有 CLOSE_OSC 代码位。
                    if ledger.open_diff(
                        SlotKey::rev(k, k),
                        frac_of(k),
                        c,
                        bar,
                        LegAnchor::SegmentScale,
                    ) {
                        counters.n_rev_open += 1;
                        self.rev = Some(RevLeg::new(k, bar, rows.dir(k)));
                        self.phase = VoicePhase::DownLeg;
                    } else {
                        // G3b：INV-1 预算（shares≤0，账本已计 n_open_rejects_zero）
                        counters.n_rev_budget_rejects += 1;
                    }
                }
            }
        }

        // ── DownLeg：T4b 加码（tranche 轴；不过 G2 门——结构发展本身就是
        //    衰竭证据的市场确认，重复守卫 = 冗余合取，C4 步骤6）──
        if self.phase == VoicePhase::DownLeg && cfg.tranche {
            self.t4b_addons(cfg, rows, center_evs_home, c, bar, ledger, entry_ladder, frac_of, counters);
        }

        // ── 域腿子循环 T3/T4（P5 逐字；C2：相位限制取消，任意相位运行）──
        if cfg.osc_mode {
            let okey = SlotKey::osc(k);
            // anchor 是 Copy——复制快照规避 open_slot 借用与 close_diff &mut 冲突
            match ledger.open_slot(okey).map(|l| l.anchor) {
                Some(anchor) => match anchor {
                    LegAnchor::Center { cs, boundary, .. } => {
                        let o_dead = cs.is_some_and(|s| book.is_dead(k, s));
                        let sub_buy = k >= 1 && rows.buy_any.get(k - 1);
                        if o_dead {
                            ledger.close_diff(okey, c, bar); // 中枢死亡 → 强制回补
                        } else if boundary.is_some_and(|b| c <= b)
                            && (!cfg.osc_buy_sub || sub_buy)
                        {
                            ledger.close_diff(okey, c, bar);
                            counters.n_osc_zd_close += 1;
                        }
                    }
                    LegAnchor::SegmentScale => unreachable!(
                        "osc 槽内不可能有段尺度锚——open 路径只以 Center 锚开 osc 腿；\
                         此 arm 是类型完备性要求，到达即 bug"
                    ),
                },
                None => {
                    if let Some(lc) = book.alive(k) {
                        let sub_sell = k >= 1 && rows.sell_any.get(k - 1);
                        if !book.is_frozen(k) && k >= 1 && c >= lc.zg && sub_sell
                            && ledger.open_diff(
                                okey,
                                frac_of(k),
                                c,
                                bar,
                                LegAnchor::Center {
                                    cs: Some(lc.seg_start),
                                    boundary: Some(lc.zd),
                                    kind: AnchorKind::Osc,
                                },
                            )
                        {
                            counters.n_osc_open += 1;
                        }
                    }
                }
            }
        }
    }

    // ════════════════════════════════════════════════════════
    // 配对修正机制（rev_paired，2026-06-10 编排者任务）
    // ════════════════════════════════════════════════════════

    /// 开腿信号谓词（runner per-ladder attempt 计数与 step 内部共用同一谓词）。
    /// 配对模式 kind 展开：confirmed Sell1 / 盘背卖（震荡型候选）∨ confirmed
    /// Sell3（逃逸型候选）。legacy 模式 = 段终结三触发原样。
    /// 注：趋势背驰卖（DivKind::Trend）不再是独立触发——它是 Sell1 的引擎
    /// 前体，等 Sell1 事件本体（与 T5 的 DivBuy(trend) no-op 同一纪律）；
    /// "段终结瞬间盲开"的主通道（无确认 div 即开）在配对模式不存在。
    pub fn rev_open_signal(cfg: &OrganicConfig, evs: &[BspEvent], devs: &[DivEvent]) -> bool {
        if !cfg.rev_paired {
            return Self::seg_end_trigger(cfg, evs, devs);
        }
        evs.iter().any(|e| {
            e.confirmed
                && match e.class {
                    BspClass::Sell1 => true,
                    BspClass::Sell3 => cfg.rev_escape_open,
                    _ => false,
                }
        }) || devs
            .iter()
            .any(|d| d.kind == DivKind::Consolidation && d.direction == Direction::Up)
    }

    /// 配对开腿：kind 展开（震荡型/逃逸型）→ 共通守卫（G1/G2/G3a，与 legacy
    /// 同序）→ 中枢/锚解析 → 深度门槛 → open。
    ///
    /// 锚解析裁决（单一真相源纪律）：震荡型"该卖点所在中枢存活"以
    /// `CenterBook::alive(k)` 判定并取其快照为锚——与 P5 域腿开腿的锚来源
    /// 完全一致（事件自带 cs 可能指向陈旧中枢，账本是当前真相）。逃逸型的
    /// 锚 = Sell3 事件自带的被终结中枢（confirmed Sell3 在本 bar ingest 已
    /// 使其死亡，alive(k) 必为 None——两分支天然互斥）。
    #[allow(clippy::too_many_arguments)]
    fn rev_paired_open(
        &mut self,
        cfg: &OrganicConfig,
        rows: &BarRows,
        book: &CenterBook,
        gate: &FatigueGate,
        entry_ladder: usize,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        frac_of: &dyn Fn(usize) -> f64,
        counters: &mut Counters,
    ) {
        let k = self.ladder;
        let evs = &rows.evs[k];
        let devs = &rows.devs[k];
        if evs.is_empty() && devs.is_empty() {
            return;
        }
        let esc_ev = if cfg.rev_escape_open {
            evs.iter().find(|e| e.confirmed && matches!(e.class, BspClass::Sell3))
        } else {
            // V2o/V2of 消融：逃逸型开腿关（kind 标注首跑数据：逃逸型三标的
            // 一致为负）。Sell3 仍照常进 CenterBook（中枢死亡 ⇒ 震荡型分支
            // 的 alive 检查自然拒绝——不开任何腿，非降级为震荡型）。
            None
        };
        let osc_sig = evs.iter().any(|e| e.confirmed && matches!(e.class, BspClass::Sell1))
            || devs
                .iter()
                .any(|d| d.kind == DivKind::Consolidation && d.direction == Direction::Up);
        if esc_ev.is_none() && !osc_sig {
            return;
        }
        counters.n_rev_attempts += 1;
        match Self::rev_open_verdict(cfg, k, rows, book, gate, entry_ladder) {
            RevOpenVerdict::RejectedSubAnchor => {
                counters.n_rev_sub_anchor_rejects += 1;
                return;
            }
            RevOpenVerdict::RejectedGate => {
                counters.n_rev_gate_rejects += 1;
                return;
            }
            RevOpenVerdict::RejectedFrozen => {
                counters.n_rev_frozen_rejects += 1;
                return;
            }
            RevOpenVerdict::Open => {}
        }
        // kind 解析：逃逸型优先（Sell3 是更强的结构陈述；同 bar 共现时中枢
        // 已死，震荡型分支的 alive 检查本就不可达）。
        let (kind, anchor_cs, zd_line, depth_ok) = if let Some(e) = esc_ev {
            let (Some(zd), Some(zg)) = (e.zd, e.zg) else {
                counters.n_rev_nocenter_rejects += 1;
                return;
            };
            if !(zd.is_finite() && zg.is_finite()) {
                counters.n_rev_nocenter_rejects += 1;
                return;
            }
            (RevOpenKind::Escape, e.cs, None, (zg - zd) / c >= cfg.theta_depth)
        } else {
            let Some(lc) = book.alive(k) else {
                counters.n_rev_nocenter_rejects += 1;
                return;
            };
            if !(lc.zd.is_finite() && lc.zg.is_finite()) {
                counters.n_rev_nocenter_rejects += 1;
                return;
            }
            // c ≤ ZD：触线条件在开腿 bar 即真 = 零利润空间（退化深度），并入深度拒。
            let ok = (lc.zg - lc.zd) / c >= cfg.theta_depth && c > lc.zd;
            (RevOpenKind::Oscillation, Some(lc.seg_start), Some(lc.zd), ok)
        };
        if !depth_ok {
            counters.n_rev_depth_rejects += 1;
            return;
        }
        let akind = AnchorKind::Bsp(match kind {
            RevOpenKind::Oscillation => BspKind::Type1,
            RevOpenKind::Escape => BspKind::Type3,
        });
        if ledger.open_diff(
            SlotKey::rev(k, k),
            frac_of(k),
            c,
            bar,
            LegAnchor::Center { cs: anchor_cs, boundary: zd_line, kind: akind },
        ) {
            counters.n_rev_open += 1;
            match kind {
                RevOpenKind::Oscillation => counters.n_rev_open_osc += 1,
                RevOpenKind::Escape => counters.n_rev_open_esc += 1,
            }
            self.rev = Some(RevLeg::new_paired(k, bar, rows.dir(k), kind, anchor_cs, zd_line));
            self.phase = VoicePhase::DownLeg;
        } else {
            counters.n_rev_budget_rejects += 1;
        }
    }

    /// 配对闭腿：T7（confirmed Buy3 回补位）> T6（candidate Buy3 预回补，
    /// pre_type3 轴）> T5（kind 配对 confirmed Buy1：震荡型同锚 / 逃逸型趋势
    /// 配对）> ZD 触线（震荡型专属几何闭腿）。闭腿集穷举——Buy2 / 盘背买 /
    /// 异锚 Buy1 不闭腿（"不接受 kind 不匹配的买点"），其反事实以
    /// n_rev_mismatch_holds 计数可观测。
    fn step_down_paired(
        &mut self,
        cfg: &OrganicConfig,
        rows: &BarRows,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        counters: &mut Counters,
    ) {
        let k = self.ladder;
        let evs = &rows.evs[k];
        let devs = &rows.devs[k];
        let Some(rev) = self.rev.as_ref() else {
            debug_assert!(false, "不变量：DownLeg ⇒ rev 存在");
            return;
        };
        let kind = rev.open_kind.expect("不变量：rev_paired 腿必携带 open_kind");
        let anchor_cs = rev.anchor_cs;
        let zd = rev.zd;
        let hard = evs.iter().any(|e| matches!(e.class, BspClass::Buy3) && e.confirmed);
        let pre = cfg.pre_type3
            && evs.iter().any(|e| matches!(e.class, BspClass::Buy3) && !e.confirmed);
        let buy1_paired = evs.iter().any(|e| {
            e.confirmed
                && matches!(e.class, BspClass::Buy1)
                && match kind {
                    // 同锚中枢内的 confirmed type1 买（Option 等值含 None==None，
                    // 但震荡型 anchor_cs 恒 Some——开腿锚取自存活中枢快照）
                    RevOpenKind::Oscillation => e.cs == anchor_cs,
                    // 逃逸型：锚中枢已被终结，反向趋势的底背驰买点必然携带
                    // 新中枢锚（或无锚）——同锚要求会使腿除逃逸外不可闭合，
                    // 镜像复刻盲配对死锁。趋势配对 = Sell3 逃逸 ↔ Buy1 衰竭。
                    RevOpenKind::Escape => true,
                }
        });
        let zd_touch = zd.is_some_and(|z| c <= z);
        if hard {
            self.close_all_paired(kind, c, bar, ledger, counters);
            counters.n_rev_close_t7 += 1;
        } else if pre {
            self.close_all_paired(kind, c, bar, ledger, counters);
            counters.n_rev_close_t6 += 1;
        } else if buy1_paired {
            self.close_all_paired(kind, c, bar, ledger, counters);
            counters.n_rev_close_t5 += 1;
        } else if zd_touch {
            self.close_all_paired(kind, c, bar, ledger, counters);
            counters.n_rev_zd_close += 1;
        } else {
            // kind/锚不匹配反事实：legacy T5 谓词会在本 bar 闭腿而配对谓词拒。
            let sub_buy = k >= 1 && rows.buy_any.get(k - 1);
            if Self::buy_trigger_at(cfg, evs, devs, sub_buy) {
                counters.n_rev_mismatch_holds += 1;
            }
        }
    }

    /// 配对模式全腿闭合 + 按开腿类型记账（win = profit > 0）。
    /// 配对闭腿信号是腿级的（反向段终结陈述），全 tranche 一次闭合；
    /// tranche×rev_paired 组合已在 runner 入口拒绝（每 tranche 同锚语义未定义）。
    fn close_all_paired(
        &mut self,
        kind: RevOpenKind,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        counters: &mut Counters,
    ) {
        if let Some(rev) = self.rev.take() {
            for t in &rev.tranches {
                let n0 = ledger.completed.len();
                ledger.close_diff(SlotKey::rev(self.ladder, t.level), c, bar);
                if ledger.completed.len() > n0 {
                    counters.n_rev_tranche_closes += 1;
                    let (_, cyc) = ledger.completed.last().expect("close_diff 刚 push");
                    let profit = cyc.profit();
                    match kind {
                        RevOpenKind::Oscillation => {
                            counters.rev_osc_pairs += 1;
                            if profit > 0.0 {
                                counters.rev_osc_wins += 1;
                            }
                            counters.rev_osc_cash += profit;
                        }
                        RevOpenKind::Escape => {
                            counters.rev_esc_pairs += 1;
                            if profit > 0.0 {
                                counters.rev_esc_wins += 1;
                            }
                            counters.rev_esc_cash += profit;
                        }
                    }
                }
            }
            self.phase = VoicePhase::UpLeg;
        }
    }

    // ════════════════════════════════════════════════════════
    // 内部机制
    // ════════════════════════════════════════════════════════

    /// REV 开腿守卫（求值顺序即拒因优先序：G1 → G2 → G3a；G3b 在 open_diff）。
    fn rev_open_verdict(
        cfg: &OrganicConfig,
        k: usize,
        rows: &BarRows,
        book: &CenterBook,
        gate: &FatigueGate,
        entry_ladder: usize,
    ) -> RevOpenVerdict {
        let g1_ok = match cfg.sub_anchor {
            SubAnchor::Off => true,
            // G1："必然有向下的第二段"的最低存在性核对（C3 方案 C）
            SubAnchor::Direction => k >= 1 && rows.dir(k - 1) == Some(Direction::Down),
            // 消融轴 S（C3 方案 D）：次级别反向中枢已形成。次级别无中枢流
            // （k−1 < FIRST_BSP_LADDER）⇒ 保守拒绝（§7 级别坐标诚实声明）。
            SubAnchor::CenterFormed => {
                k >= 1
                    && rows.dir(k - 1) == Some(Direction::Down)
                    && k - 1 >= FIRST_BSP_LADDER
                    && book.alive(k - 1).is_some()
            }
        };
        if !g1_ok {
            return RevOpenVerdict::RejectedSubAnchor;
        }
        if cfg.rev_gate && !gate.gate_open(k, entry_ladder) {
            return RevOpenVerdict::RejectedGate;
        }
        if book.is_frozen(k) {
            return RevOpenVerdict::RejectedFrozen;
        }
        RevOpenVerdict::Open
    }

    /// T4b 三类加码事件（模块 docstring 解读注记；全部依赖 D3 行）。
    #[allow(clippy::too_many_arguments)]
    fn t4b_addons(
        &mut self,
        cfg: &OrganicConfig,
        rows: &BarRows,
        center_evs_home: &[CenterEvent],
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        entry_ladder: usize,
        frac_of: &dyn Fn(usize) -> f64,
        counters: &mut Counters,
    ) {
        let k = self.ladder;
        let Some(rev) = self.rev.as_mut() else { return };
        let (Some(dir_row), Some(run_anchor)) = (rows.dir_row, rows.run_anchor) else {
            return; // capability guard 已在 runner 入口拒绝；此处防御性短路
        };
        // tranche 级别上限：voices 域为 [floor, entry)，加码不越入 master 音域
        let cap = entry_ladder.saturating_sub(1);
        let in_rev_run = dir_row[k] == Some(Direction::Down)
            && run_anchor[k] >= rev.open_bar - cfg.t4b_anchor_margin;

        let mut targets: Vec<usize> = Vec::new();
        if in_rev_run {
            for ce in center_evs_home {
                match ce {
                    CenterEvent::Formed { .. } => {
                        if !rev.structure_added {
                            // (a) 反向 run 内首个 Formed → 结构确认加一级
                            rev.structure_added = true;
                            targets.push(rev.max_level() + 1);
                        } else if rev.terminated_down_seen && !rev.trend_added {
                            // (b) Terminated{Down} 后同 run 第二个 Formed → 趋势确认
                            rev.trend_added = true;
                            targets.push(rev.max_level() + 1);
                        }
                    }
                    CenterEvent::Terminated { direction: Direction::Down, .. } => {
                        rev.terminated_down_seen = true;
                    }
                    CenterEvent::Terminated { direction: Direction::Up, .. }
                    | CenterEvent::Extended { .. } => {} // §5.3 矩阵 no-op
                }
            }
        }
        // (c) ℓ′ > 当前最高 tranche 级别的层出现反向 run
        for lp in (rev.max_level() + 1)..=cap.min(MAX_LADDER - 1) {
            if dir_row[lp] == Some(Direction::Down)
                && run_anchor[lp] >= rev.open_bar - cfg.t4b_anchor_margin
            {
                targets.push(lp);
            }
        }
        for level in targets {
            if level > cap || level >= MAX_LADDER {
                continue;
            }
            // 每级别至多一个开放 tranche
            if rev.tranches.iter().any(|t| t.level == level) {
                continue;
            }
            if ledger.open_diff(
                SlotKey::rev(k, level),
                frac_of(level),
                c,
                bar,
                LegAnchor::SegmentScale,
            ) {
                rev.tranches.push(RevTranche { level, last_dir: dir_row[level] });
                rev.tranches.sort_by_key(|t| t.level); // 升序不变式
                counters.n_rev_tranche_adds += 1;
            }
        }
    }

    /// T5b：tranche 级别的方向行 Down→Up 翻转 → 市价回补 level ≤ ℓ（防悬挂）。
    fn t5b_struct_close(
        &mut self,
        rows: &BarRows,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        counters: &mut Counters,
    ) {
        let k = self.ladder;
        let Some(rev) = self.rev.as_mut() else { return };
        let mut flip_level: Option<usize> = None;
        for t in rev.tranches.iter_mut() {
            let now = rows.dir_row.and_then(|row| row[t.level]);
            if t.last_dir == Some(Direction::Down) && now == Some(Direction::Up) {
                flip_level = Some(flip_level.map_or(t.level, |f: usize| f.max(t.level)));
            }
            t.last_dir = now;
        }
        if let Some(l) = flip_level {
            let closed = rev.close_upto(l);
            for t in &closed {
                ledger.close_diff(SlotKey::rev(k, t.level), c, bar);
                counters.n_rev_tranche_closes += 1;
            }
            counters.n_rev_struct_close += 1;
            if rev.tranches.is_empty() {
                self.rev = None;
                self.phase = VoicePhase::UpLeg;
            }
        }
    }

    /// T6/T7 公共动作：全 tranche 回补（级别无关——反向假设在域层面否证）。
    fn close_all_tranches(
        &mut self,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        counters: &mut Counters,
    ) {
        if let Some(rev) = self.rev.take() {
            for t in &rev.tranches {
                ledger.close_diff(SlotKey::rev(self.ladder, t.level), c, bar);
                counters.n_rev_tranche_closes += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_rev() -> OrganicConfig {
        OrganicConfig { rev_mode: true, ..OrganicConfig::default() }
    }

    fn ev(class: BspClass, confirmed: bool) -> BspEvent {
        BspEvent { class, seg_idx: 0, confirmed, cs: None, zd: None, zg: None, price: 10.0 }
    }

    fn ev_anchored(class: BspClass, confirmed: bool, cs: i64, zd: f64, zg: f64) -> BspEvent {
        BspEvent {
            class,
            seg_idx: 0,
            confirmed,
            cs: Some(cs),
            zd: Some(zd),
            zg: Some(zg),
            price: 10.0,
        }
    }

    fn empty_rows<'a>(
        evs: &'a [Vec<BspEvent>; MAX_LADDER],
        devs: &'a [Vec<DivEvent>; MAX_LADDER],
    ) -> BarRows<'a> {
        BarRows {
            evs,
            devs,
            buy_any: LadderMask(0),
            sell_any: LadderMask(0),
            dir_row: None,
            run_anchor: None,
        }
    }

    struct Fixture {
        evs: [Vec<BspEvent>; MAX_LADDER],
        devs: [Vec<DivEvent>; MAX_LADDER],
        ledger: OrganicLedger,
        book: CenterBook,
        gate: FatigueGate,
        counters: Counters,
    }

    impl Fixture {
        fn new() -> Self {
            Fixture {
                evs: Default::default(),
                devs: Default::default(),
                ledger: OrganicLedger::new(10.0, 100.0, 0.5, false),
                book: CenterBook::new(),
                gate: FatigueGate::new(),
                counters: Counters::default(),
            }
        }
    }

    #[test]
    fn t1_opens_rev_without_touching_osc() {
        // C2 验证：osc 开放时 REV 开腿，osc 槽不被截断
        let mut fx = Fixture::new();
        // 先放一个存活中枢并开 osc 腿
        fx.book.ingest(2, &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)], true, None);
        assert!(fx.ledger.open_diff(
            SlotKey::osc(2),
            0.5,
            10.0,
            0,
            LegAnchor::Center { cs: Some(1), boundary: Some(9.0), kind: AnchorKind::Osc },
        ));
        // 段终结触发（confirmed Sell1）
        fx.evs[2] = vec![ev(BspClass::Sell1, true)];
        let mut v = VoiceUnit::new(2);
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_rev(), &rows, &[], 9.6, 1, &mut fx.ledger, &fx.book, &fx.gate, 4,
            &|_| 0.5, &mut fx.counters,
        );
        assert_eq!(v.phase, VoicePhase::DownLeg);
        assert_eq!(fx.counters.n_rev_open, 1);
        // osc 槽仍开放（v1 的"先 CLOSE_OSC"在 v2 没有代码位）
        assert!(fx.ledger.open_slot(SlotKey::osc(2)).is_some());
        assert!(fx.ledger.open_slot(SlotKey::rev(2, 2)).is_some());
    }

    #[test]
    fn t7_closes_all_tranches_and_returns_upleg() {
        let mut fx = Fixture::new();
        fx.evs[2] = vec![ev(BspClass::Sell1, true)];
        let mut v = VoiceUnit::new(2);
        {
            let rows = empty_rows(&fx.evs, &fx.devs);
            v.step(
                &cfg_rev(), &rows, &[], 10.0, 1, &mut fx.ledger, &fx.book, &fx.gate, 4,
                &|_| 0.5, &mut fx.counters,
            );
        }
        assert_eq!(v.phase, VoicePhase::DownLeg);
        // confirmed Buy3 → T7
        fx.evs[2] = vec![ev(BspClass::Buy3, true)];
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_rev(), &rows, &[], 9.0, 2, &mut fx.ledger, &fx.book, &fx.gate, 4,
            &|_| 0.5, &mut fx.counters,
        );
        assert_eq!(v.phase, VoicePhase::UpLeg);
        assert!(v.rev.is_none());
        assert_eq!(fx.counters.n_rev_close_t7, 1);
        assert!(fx.ledger.open_slot(SlotKey::rev(2, 2)).is_none());
    }

    #[test]
    fn t5_nesting_closes_prefix_only() {
        // 双 tranche（level 2 + 3），level 2 买点 → 只回补 level 2，高层存续
        let mut fx = Fixture::new();
        let mut v = VoiceUnit::new(2);
        v.phase = VoicePhase::DownLeg;
        let mut leg = RevLeg::new(2, 0, None);
        leg.tranches.push(RevTranche { level: 3, last_dir: None });
        v.rev = Some(leg);
        assert!(fx.ledger.open_diff(SlotKey::rev(2, 2), 0.3, 10.0, 0, LegAnchor::SegmentScale));
        assert!(fx.ledger.open_diff(SlotKey::rev(2, 3), 0.3, 10.0, 0, LegAnchor::SegmentScale));
        // level 2 confirmed Buy1
        fx.evs[2] = vec![ev(BspClass::Buy1, true)];
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_rev(), &rows, &[], 9.0, 1, &mut fx.ledger, &fx.book, &fx.gate, 5,
            &|_| 0.3, &mut fx.counters,
        );
        assert_eq!(v.phase, VoicePhase::DownLeg); // 高层 tranche 存续
        assert!(fx.ledger.open_slot(SlotKey::rev(2, 2)).is_none());
        assert!(fx.ledger.open_slot(SlotKey::rev(2, 3)).is_some());
        assert_eq!(fx.counters.n_rev_tranche_closes, 1);
        // level 3 confirmed Buy1 同时出现 level 2 买点 → max 读数全回补
        fx.evs[2] = vec![ev(BspClass::Buy1, true)];
        fx.evs[3] = vec![ev(BspClass::Buy1, true)];
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_rev(), &rows, &[], 8.5, 2, &mut fx.ledger, &fx.book, &fx.gate, 5,
            &|_| 0.3, &mut fx.counters,
        );
        assert_eq!(v.phase, VoicePhase::UpLeg);
        assert!(fx.ledger.open_slot(SlotKey::rev(2, 3)).is_none());
    }

    #[test]
    fn g1_direction_anchor_rejects_when_sub_not_down() {
        let mut fx = Fixture::new();
        fx.evs[2] = vec![ev(BspClass::Sell1, true)];
        let cfg = OrganicConfig {
            rev_mode: true,
            sub_anchor: SubAnchor::Direction,
            ..OrganicConfig::default()
        };
        let mut v = VoiceUnit::new(2);
        let dir_row: [Option<Direction>; MAX_LADDER] = [Some(Direction::Up); MAX_LADDER];
        let anchors = [0i64; MAX_LADDER];
        let rows = BarRows {
            evs: &fx.evs,
            devs: &fx.devs,
            buy_any: LadderMask(0),
            sell_any: LadderMask(0),
            dir_row: Some(&dir_row),
            run_anchor: Some(&anchors),
        };
        v.step(
            &cfg, &rows, &[], 10.0, 1, &mut fx.ledger, &fx.book, &fx.gate, 4,
            &|_| 0.5, &mut fx.counters,
        );
        assert_eq!(v.phase, VoicePhase::UpLeg);
        assert_eq!(fx.counters.n_rev_sub_anchor_rejects, 1);
        assert_eq!(fx.counters.n_rev_open, 0);
    }

    #[test]
    fn t4b_adds_tranche_on_center_formed_in_rev_run() {
        let mut fx = Fixture::new();
        let cfg = OrganicConfig {
            rev_mode: true,
            tranche: true,
            ..OrganicConfig::default()
        };
        let mut v = VoiceUnit::new(2);
        v.phase = VoicePhase::DownLeg;
        v.rev = Some(RevLeg::new(2, 0, Some(Direction::Down)));
        assert!(fx.ledger.open_diff(SlotKey::rev(2, 2), 0.3, 10.0, 0, LegAnchor::SegmentScale));
        let mut dir_row: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        dir_row[2] = Some(Direction::Down);
        let anchors = [0i64; MAX_LADDER];
        let rows = BarRows {
            evs: &fx.evs,
            devs: &fx.devs,
            buy_any: LadderMask(0),
            sell_any: LadderMask(0),
            dir_row: Some(&dir_row),
            run_anchor: Some(&anchors),
        };
        let formed = CenterEvent::Formed { seg_start: 9, zd: 8.0, zg: 9.0 };
        v.step(
            &cfg, &rows, &[formed], 9.0, 3, &mut fx.ledger, &fx.book, &fx.gate, 5,
            &|_| 0.2, &mut fx.counters,
        );
        assert_eq!(fx.counters.n_rev_tranche_adds, 1);
        assert_eq!(v.rev.as_ref().unwrap().max_level(), 3);
        assert!(fx.ledger.open_slot(SlotKey::rev(2, 3)).is_some());
        // 第二个 Formed（未见 Terminated{Down}）不再加码（(a) 一次性）
        v.step(
            &cfg, &rows, &[formed], 9.0, 4, &mut fx.ledger, &fx.book, &fx.gate, 5,
            &|_| 0.2, &mut fx.counters,
        );
        assert_eq!(fx.counters.n_rev_tranche_adds, 1);
    }

    fn cfg_paired(theta: f64) -> OrganicConfig {
        OrganicConfig {
            rev_mode: true,
            rev_paired: true,
            theta_depth: theta,
            ..OrganicConfig::default()
        }
    }

    #[test]
    fn paired_osc_open_anchors_alive_center_and_closes_at_zd() {
        // 震荡型：confirmed Sell1 × 存活中枢 [9.0, 9.5] → 开腿锚 ZD=9.0；
        // 触线 c≤9.0 → 闭腿（n_rev_zd_close），按类型记账。
        let mut fx = Fixture::new();
        fx.book.ingest(2, &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)], true, None);
        fx.evs[2] = vec![ev(BspClass::Sell1, true)];
        let mut v = VoiceUnit::new(2);
        {
            let rows = empty_rows(&fx.evs, &fx.devs);
            v.step(
                &cfg_paired(0.0), &rows, &[], 9.6, 1, &mut fx.ledger, &fx.book, &fx.gate, 4,
                &|_| 0.5, &mut fx.counters,
            );
        }
        assert_eq!(v.phase, VoicePhase::DownLeg);
        assert_eq!(fx.counters.n_rev_open_osc, 1);
        let leg = v.rev.as_ref().unwrap();
        assert_eq!(leg.open_kind, Some(RevOpenKind::Oscillation));
        assert_eq!(leg.anchor_cs, Some(1));
        assert_eq!(leg.zd, Some(9.0));
        // 无事件 bar：c=9.0 触线 → 闭腿
        fx.evs[2].clear();
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_paired(0.0), &rows, &[], 9.0, 2, &mut fx.ledger, &fx.book, &fx.gate, 4,
            &|_| 0.5, &mut fx.counters,
        );
        assert_eq!(v.phase, VoicePhase::UpLeg);
        assert_eq!(fx.counters.n_rev_zd_close, 1);
        assert_eq!(fx.counters.rev_osc_pairs, 1);
        assert_eq!(fx.counters.rev_osc_wins, 1); // 卖9.6买9.0
        assert!(fx.counters.rev_osc_cash > 0.0);
    }

    #[test]
    fn paired_rejects_mismatched_buys_accepts_same_anchor_buy1() {
        // 闭腿配对：Buy2 / 盘背买 / 异锚 Buy1 不闭（mismatch_holds 计数），
        // 同锚 confirmed Buy1 闭（T5）。
        let mut fx = Fixture::new();
        fx.book.ingest(2, &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)], true, None);
        fx.evs[2] = vec![ev(BspClass::Sell1, true)];
        let mut v = VoiceUnit::new(2);
        {
            let rows = empty_rows(&fx.evs, &fx.devs);
            v.step(
                &cfg_paired(0.0), &rows, &[], 9.6, 1, &mut fx.ledger, &fx.book, &fx.gate, 4,
                &|_| 0.5, &mut fx.counters,
            );
        }
        assert_eq!(v.phase, VoicePhase::DownLeg);
        // Buy2（kind 不匹配）+ 异锚 Buy1（cs=99 ≠ 锚 1）→ 不闭
        fx.evs[2] = vec![
            ev(BspClass::Buy2, true),
            ev_anchored(BspClass::Buy1, true, 99, 8.0, 8.5),
        ];
        {
            let rows = empty_rows(&fx.evs, &fx.devs);
            v.step(
                &cfg_paired(0.0), &rows, &[], 9.3, 2, &mut fx.ledger, &fx.book, &fx.gate, 4,
                &|_| 0.5, &mut fx.counters,
            );
        }
        assert_eq!(v.phase, VoicePhase::DownLeg);
        assert_eq!(fx.counters.n_rev_mismatch_holds, 1);
        // 同锚 confirmed Buy1（cs=1）→ T5 闭腿
        fx.evs[2] = vec![ev_anchored(BspClass::Buy1, true, 1, 9.0, 9.5)];
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_paired(0.0), &rows, &[], 9.2, 3, &mut fx.ledger, &fx.book, &fx.gate, 4,
            &|_| 0.5, &mut fx.counters,
        );
        assert_eq!(v.phase, VoicePhase::UpLeg);
        assert_eq!(fx.counters.n_rev_close_t5, 1);
        assert_eq!(fx.counters.rev_osc_pairs, 1);
    }

    #[test]
    fn paired_escape_opens_on_sell3_no_zd_line_closes_on_any_buy1() {
        // 逃逸型：confirmed Sell3（中枢死亡）→ 开腿无 ZD 线；
        // c 低于死中枢 ZD 不触发闭腿；任意锚 confirmed Buy1 闭（趋势配对）。
        let mut fx = Fixture::new();
        // 中枢 [9.0,9.5] 先存活，confirmed Sell3 将其向下终结
        fx.book.ingest(2, &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)], true, None);
        fx.book.ingest(2, &[ev_anchored(BspClass::Sell3, true, 1, 9.0, 9.5)], true, None);
        assert!(fx.book.alive(2).is_none());
        fx.evs[2] = vec![ev_anchored(BspClass::Sell3, true, 1, 9.0, 9.5)];
        let mut v = VoiceUnit::new(2);
        {
            let rows = empty_rows(&fx.evs, &fx.devs);
            v.step(
                &cfg_paired(0.0), &rows, &[], 8.8, 1, &mut fx.ledger, &fx.book, &fx.gate, 4,
                &|_| 0.5, &mut fx.counters,
            );
        }
        assert_eq!(v.phase, VoicePhase::DownLeg);
        assert_eq!(fx.counters.n_rev_open_esc, 1);
        let leg = v.rev.as_ref().unwrap();
        assert_eq!(leg.open_kind, Some(RevOpenKind::Escape));
        assert_eq!(leg.zd, None);
        // 无事件 + 价格更低 → 无 ZD 线，不闭
        fx.evs[2].clear();
        {
            let rows = empty_rows(&fx.evs, &fx.devs);
            v.step(
                &cfg_paired(0.0), &rows, &[], 8.0, 2, &mut fx.ledger, &fx.book, &fx.gate, 4,
                &|_| 0.5, &mut fx.counters,
            );
        }
        assert_eq!(v.phase, VoicePhase::DownLeg);
        // 异锚 confirmed Buy1（新中枢 cs=7）→ 趋势配对闭腿
        fx.evs[2] = vec![ev_anchored(BspClass::Buy1, true, 7, 7.5, 7.9)];
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_paired(0.0), &rows, &[], 7.8, 3, &mut fx.ledger, &fx.book, &fx.gate, 4,
            &|_| 0.5, &mut fx.counters,
        );
        assert_eq!(v.phase, VoicePhase::UpLeg);
        assert_eq!(fx.counters.n_rev_close_t5, 1);
        assert_eq!(fx.counters.rev_esc_pairs, 1);
        assert_eq!(fx.counters.rev_esc_wins, 1); // 卖8.8买7.8
    }

    #[test]
    fn paired_depth_gate_rejects_shallow_center() {
        // 深度门：中枢 [9.0, 9.005] 振幅/价 ≈0.05% < θ=1% → 拒，计数。
        let mut fx = Fixture::new();
        fx.book.ingest(2, &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.005)], true, None);
        fx.evs[2] = vec![ev(BspClass::Sell1, true)];
        let mut v = VoiceUnit::new(2);
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_paired(0.01), &rows, &[], 9.1, 1, &mut fx.ledger, &fx.book, &fx.gate, 4,
            &|_| 0.5, &mut fx.counters,
        );
        assert_eq!(v.phase, VoicePhase::UpLeg);
        assert_eq!(fx.counters.n_rev_depth_rejects, 1);
        assert_eq!(fx.counters.n_rev_open, 0);
    }

    #[test]
    fn paired_osc_requires_alive_center() {
        // 震荡型无存活中枢（孤证 confirmed Sell1）→ nocenter 拒。
        let mut fx = Fixture::new();
        fx.evs[2] = vec![ev(BspClass::Sell1, true)];
        let mut v = VoiceUnit::new(2);
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_paired(0.0), &rows, &[], 9.6, 1, &mut fx.ledger, &fx.book, &fx.gate, 4,
            &|_| 0.5, &mut fx.counters,
        );
        assert_eq!(v.phase, VoicePhase::UpLeg);
        assert_eq!(fx.counters.n_rev_nocenter_rejects, 1);
        assert_eq!(fx.counters.n_rev_open, 0);
    }

    #[test]
    fn paired_escape_off_does_not_open_on_sell3() {
        // V2o 消融：rev_escape_open=false 时 confirmed Sell3 不开腿，
        // 也不降级为震荡型（中枢已死 ⇒ alive 检查拒）。
        let mut fx = Fixture::new();
        fx.book.ingest(2, &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)], true, None);
        fx.book.ingest(2, &[ev_anchored(BspClass::Sell3, true, 1, 9.0, 9.5)], true, None);
        fx.evs[2] = vec![ev_anchored(BspClass::Sell3, true, 1, 9.0, 9.5)];
        let cfg = OrganicConfig { rev_escape_open: false, ..cfg_paired(0.0) };
        let mut v = VoiceUnit::new(2);
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg, &rows, &[], 8.8, 1, &mut fx.ledger, &fx.book, &fx.gate, 4,
            &|_| 0.5, &mut fx.counters,
        );
        assert_eq!(v.phase, VoicePhase::UpLeg);
        assert_eq!(fx.counters.n_rev_open, 0);
        assert_eq!(fx.counters.n_rev_attempts, 0); // 信号谓词同步排除 Sell3
    }

    #[test]
    fn paired_buy3_escape_close_still_works() {
        // Buy3 回补位闭腿（恢复暴露）在配对模式保留（T7）。
        let mut fx = Fixture::new();
        fx.book.ingest(2, &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)], true, None);
        fx.evs[2] = vec![ev(BspClass::Sell1, true)];
        let mut v = VoiceUnit::new(2);
        {
            let rows = empty_rows(&fx.evs, &fx.devs);
            v.step(
                &cfg_paired(0.0), &rows, &[], 9.6, 1, &mut fx.ledger, &fx.book, &fx.gate, 4,
                &|_| 0.5, &mut fx.counters,
            );
        }
        assert_eq!(v.phase, VoicePhase::DownLeg);
        fx.evs[2] = vec![ev(BspClass::Buy3, true)];
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg_paired(0.0), &rows, &[], 9.8, 2, &mut fx.ledger, &fx.book, &fx.gate, 4,
            &|_| 0.5, &mut fx.counters,
        );
        assert_eq!(v.phase, VoicePhase::UpLeg);
        assert_eq!(fx.counters.n_rev_close_t7, 1);
        assert_eq!(fx.counters.rev_osc_pairs, 1);
        assert_eq!(fx.counters.rev_osc_wins, 0); // 卖9.6买9.8 亏损回补
    }

    #[test]
    fn osc_subloop_p5_open_close() {
        // P5 域腿逐字：c≥ZG ∧ sub_sell 开；c≤锚ZD 关
        let mut fx = Fixture::new();
        fx.book.ingest(2, &[ev_anchored(BspClass::Sell1, true, 1, 9.0, 9.5)], true, None);
        let cfg = OrganicConfig::default(); // O0：rev 关
        let mut v = VoiceUnit::new(2);
        {
            let mut rows = empty_rows(&fx.evs, &fx.devs);
            rows.sell_any = LadderMask(1 << 1); // sub_sell = sell_any[k-1]
            v.step(
                &cfg, &rows, &[], 9.6, 1, &mut fx.ledger, &fx.book, &fx.gate, 4,
                &|_| 0.5, &mut fx.counters,
            );
        }
        assert_eq!(fx.counters.n_osc_open, 1);
        assert!(fx.ledger.open_slot(SlotKey::osc(2)).is_some());
        // 触 ZD 回补
        let rows = empty_rows(&fx.evs, &fx.devs);
        v.step(
            &cfg, &rows, &[], 9.0, 2, &mut fx.ledger, &fx.book, &fx.gate, 4,
            &|_| 0.5, &mut fx.counters,
        );
        assert_eq!(fx.counters.n_osc_zd_close, 1);
        assert!(fx.ledger.open_slot(SlotKey::osc(2)).is_none());
    }
}
