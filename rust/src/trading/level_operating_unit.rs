//! GUARD-ROLE: organic-fugue-v2-trading-layer——名分：现役（详见 `trading/mod.rs` 头部 GUARD-ROLE 块，#763 C7-E4 核定；零删除/零移入 legacy/）。
//!
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

use super::center_book::{CenterBook, UpStrengthVerdict};
use super::config::{
    OrganicConfig, OscDomain, RevClose, RevCycle, RevCycleClose, SubAnchor, SubMode, ThetaMode,
};
use super::depth_ref::DepthRef;
use super::fatigue_gate::FatigueGate;
use super::ledger::OrganicLedger;
use super::types::*;
use crate::buysellpoint::BspKind;
use crate::divergence::DivKind;
use crate::stroke::Direction;

#[path = "level_sub_lou.rs"]
mod level_sub_lou;

pub use level_sub_lou::SubLou;

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
    /// R2 开（r2_anchor_zg）时降为条件延伸目标（兑现锚移至 zg）。
    pub zd: Option<f64>,
    /// 锚中枢上沿。仅震荡型携带。R2 兑现锚（c ≤ zg 即回入原文保证域）+
    /// R3"回拉低点 > ZG"的比较基准。
    pub zg: Option<f64>,
    /// R2 延伸档资格（开腿时中枢全振幅 (ZG−ZD)/c ≥ θ——原深度门保留为
    /// 延伸档独立门，调研报告 §4.3）。非 R2 腿恒 false。
    pub ext_allowed: bool,
    /// 开腿触发位掩码（bit0=Sell1 / bit1=盘背 / bit2=Sell3，与 RevOpenLog
    /// 同编码）。R2/R3 的作用域判别：仅 trigger==2（盘背独占触发）的腿消费
    /// ——type1 卖（Sell1 参与触发）的 REV 逻辑不可被改动（任务硬约束）。
    pub open_trigger: u8,
    /// 腿生命期内 close 最低价（R3"回拉低点 > ZG"判据；close 分辨率）。
    pub low_since_open: f64,
    /// Seq38n（rev_seq_nobreak）：开腿时冻结的第一段低点参照 =
    /// VoiceUnit::rev_run_low（上次配对闭腿以来 close 运行最低——38课:36
    /// "不跌破第一段低点"的比较基准；Sequence38 子腿 seg1_low 的主腿同构，
    /// close 分辨率是诚实近似残留）。legacy / Cycle38 腿恒 None。
    pub seg1_low: Option<f64>,
    /// 递归子 LOU（rev_sub_depth ≥ 1 时在 REV 窗口内实例化；38课程式的
    /// 方向镜像实例——反弹腿先买后卖）。父腿闭合时级联强闭全部子树腿。
    pub sub: Option<Box<SubLou>>,
    // ── T4b (a)/(b) 一次性触发标记（模块 docstring 解读注记）──
    structure_added: bool,
    terminated_down_seen: bool,
    trend_added: bool,
}

impl RevLeg {
    fn new(home: usize, bar: i64, dir: Option<Direction>) -> Self {
        RevLeg {
            tranches: vec![RevTranche {
                level: home,
                last_dir: dir,
            }],
            open_bar: bar,
            open_kind: None,
            anchor_cs: None,
            zd: None,
            zg: None,
            ext_allowed: false,
            open_trigger: 0,
            low_since_open: f64::INFINITY,
            seg1_low: None,
            sub: None,
            structure_added: false,
            terminated_down_seen: false,
            trend_added: false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn new_paired(
        home: usize,
        bar: i64,
        dir: Option<Direction>,
        kind: RevOpenKind,
        anchor_cs: Option<i64>,
        zd: Option<f64>,
        zg: Option<f64>,
        ext_allowed: bool,
        open_trigger: u8,
        open_price: f64,
    ) -> Self {
        RevLeg {
            open_kind: Some(kind),
            anchor_cs,
            zd,
            zg,
            ext_allowed,
            open_trigger,
            low_since_open: open_price,
            ..RevLeg::new(home, bar, dir)
        }
    }

    pub fn max_level(&self) -> usize {
        self.tranches
            .last()
            .map(|t| t.level)
            .expect("RevLeg 不变式：tranches 非空")
    }

    /// 逐级回补：移除全部 level ≤ confirm 的 tranche，返回被关闭者（level 升序）。
    pub fn close_upto(&mut self, confirm: usize) -> Vec<RevTranche> {
        let keep: Vec<RevTranche> = self
            .tranches
            .iter()
            .copied()
            .filter(|t| t.level > confirm)
            .collect();
        let closed: Vec<RevTranche> = self
            .tranches
            .iter()
            .copied()
            .filter(|t| t.level <= confirm)
            .collect();
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
    /// 锚中枢振幅因果参照（θ 自适应深度门的数据依赖；runner 恒提供——
    /// None 仅测试夹具；theta_mode=AdaptiveQuantile 读取时 None ⇒ panic）。
    pub depth: Option<&'a DepthRef>,
    /// 父级别走势衰竭追踪器（P1 41课门的数据依赖；仅 sub_l41_gate 变体由
    /// runner 提供——sub_l41_gate 读取时 None ⇒ panic，capability 同 depth）。
    pub l41: Option<&'a super::trend_exhaustion::TrendExhaustion>,
    /// 趋势态行（rev_cycle=Cycle38 的数据依赖）：trend_row[k] ⟺ 该层尾 move
    /// kind==Trend（≥2 同向中枢，17课趋势定义）。runner 从磁带 trend_flips
    /// 稀疏翻转行滚动导出；None = 信号层未产出（Cycle38 读取时 panic，
    /// capability guard 在 runner 入口拒绝）。
    pub trend_row: Option<&'a [bool; MAX_LADDER]>,
}

impl BarRows<'_> {
    pub(crate) fn dir(&self, k: usize) -> Option<Direction> {
        self.dir_row.and_then(|row| row[k])
    }

    /// 次级别确认统一谓词（27课区间套延拓到全部操作点，2026-06-11 任务）。
    ///
    /// 确认 = **同 bar 事件证据** ∨ **D3 方向行结构证据**：
    /// - 事件证据：次级别（k−1）本 bar 买/卖侧事件（buy_any/sell_any 掩码 ∪
    ///   div 事件）——T5 nesting_buy_level 的已验证共现形式（27课"大级别买点
    ///   必然伴随小级别买点共现"），非 R1 的窗口记忆形式（已否证：反选）。
    /// - 结构证据：dir_row[k−1] 已翻向操作方向（Sell→Down / Buy→Up）。方向行
    ///   是状态非事件；bi 层（ladder 1）无 BSP/div 事件流但有 D3 行——这消除
    ///   R1 的空定义域陷阱（k=2 腿在纯事件证据下必然保守拒绝，消融判决 §3.1）。
    ///
    /// 调用方保证 dir_row 存在（runner capability guard：SC 位 ⇒ dir_flips 行）。
    pub fn sub_confirm(&self, k: usize, side: crate::buysellpoint::Side) -> bool {
        use crate::buysellpoint::Side;
        if k == 0 {
            return false; // 无次级别（FIRST_BSP_LADDER=2 下不可达；保守拒绝）
        }
        let sub = k - 1;
        let (mask_hit, dir_want) = match side {
            Side::Sell => (self.sell_any.get(sub), Direction::Down),
            Side::Buy => (self.buy_any.get(sub), Direction::Up),
        };
        mask_hit
            || self.devs[sub].iter().any(|d| d.side() == side)
            || self.dir(sub) == Some(dir_want)
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
    // ── 区间套证据记忆（R1/R2/R3，仅 rev_paired × R 位路径更新）──
    // 边界声明（090号）：VoiceUnit 仅在 LONG 态存在且逐 trade 重建——记忆
    // 覆盖域 = 本 trade 的 LONG 区间；入场前的次级别证据不可见。
    /// 最近一次次级别（k−1）卖侧证据 bar（卖侧背驰 ∪ confirmed Sell1，
    /// 调研报告 §4.1 公式逐字）。
    sub_sell_bar: Option<i64>,
    /// 最近一次次级别（k−1）买侧证据 bar（买侧背驰 ∪ 任意 confirmed 买点）。
    sub_buy_bar: Option<i64>,
    /// 次级别最近 Up run 起点（D3 run_anchor；≈ 本级别 C 段窗口起点——C 段
    /// 是 k−1 层结构，其 bar 窗口 = k−1 层最近一段 Up run。run 翻 Down 后
    /// 锚保留：盘背触发可晚于次级别拉回起点）。
    sub_up_anchor: Option<i64>,
    /// 38课循环态（rev_cycle=Cycle38 专用）：true = 宿主（ladder+1）趋势
    /// 存续期间的循环短差窗口开放。Single 模式恒 false（零接触）。
    cycle38_on: bool,
    /// Seq38n（rev_seq_nobreak）：close 运行最低——节点创建/上次配对闭腿
    /// 以来（含本 bar）。开腿时冻结进 RevLeg::seg1_low；闭腿后复位为闭腿
    /// close（38课中间循环：新一轮第一段低点重新累计）。覆盖域 = 本 trade
    /// 的 LONG 区间（VoiceUnit 逐 trade 重建——与区间套证据记忆同边界）。
    rev_run_low: f64,
}

impl VoiceUnit {
    pub fn new(ladder: usize) -> Self {
        VoiceUnit {
            ladder,
            phase: VoicePhase::UpLeg,
            rev: None,
            sub_sell_bar: None,
            sub_buy_bar: None,
            sub_up_anchor: None,
            cycle38_on: false,
            rev_run_low: f64::INFINITY,
        }
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
        }) || devs
            .iter()
            .any(|d| d.side() == crate::buysellpoint::Side::Sell)
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
    pub fn nesting_buy_level(cfg: &OrganicConfig, rows: &BarRows, rev: &RevLeg) -> Option<usize> {
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
        // 第一段低点参照推进（Seq38n；含本 bar——与 Sequence38 子腿
        // run_low 同语义。无条件维护：纯状态跟踪，仅 rev_seq_nobreak 消费）。
        self.rev_run_low = self.rev_run_low.min(c);
        let evs = &rows.evs[k];
        let devs = &rows.devs[k];

        // ── 38课循环模式（rev_cycle=Cycle38）：替换单次 REV 腿的全部相位
        //    转换路径（T1/T5/T6/T7/T4b/递归子树都不运行——模式互斥，非叠加）；
        //    osc 域腿正交（C2）照常运行 ──
        if cfg.rev_cycle == RevCycle::Cycle38 {
            self.step_cycle38(cfg, rows, book, c, bar, ledger, frac_of, counters);
            self.step_osc(cfg, rows, book, c, bar, ledger, frac_of, counters);
            return;
        }

        // ── 区间套证据记忆推进（R1/R2/R3；每 bar，先于一切判定）──
        if cfg.rev_paired
            && (cfg.r1_sub_sell_open || cfg.r2_anchor_zg || cfg.r3_t6_sub_confirm)
            && k >= 1
        {
            let sub_devs = &rows.devs[k - 1];
            if cfg.r1_sub_sell_open {
                let sell_ev = sub_devs
                    .iter()
                    .any(|d| d.side() == crate::buysellpoint::Side::Sell)
                    || rows.evs[k - 1]
                        .iter()
                        .any(|e| e.confirmed && matches!(e.class, BspClass::Sell1));
                if sell_ev {
                    self.sub_sell_bar = Some(bar);
                }
                if rows.dir(k - 1) == Some(Direction::Up) {
                    if let Some(ra) = rows.run_anchor {
                        self.sub_up_anchor = Some(ra[k - 1]);
                    }
                }
            }
            if cfg.r2_anchor_zg || cfg.r3_t6_sub_confirm {
                let buy_ev = rows.buy_any.get(k - 1)
                    || sub_devs
                        .iter()
                        .any(|d| d.side() == crate::buysellpoint::Side::Buy);
                if buy_ev {
                    self.sub_buy_bar = Some(bar);
                }
            }
        }
        // R3 回拉低点推进（close 分辨率；持腿期每 bar）
        if let Some(rev) = self.rev.as_mut() {
            if c < rev.low_since_open {
                rev.low_since_open = c;
            }
        }

        // ── DownLeg：T7 > T6 > T5 > T5b（关腿端）──
        if self.phase == VoicePhase::DownLeg && cfg.rev_paired {
            // 配对闭腿（任务 2026-06-10）：闭腿集 = {Buy3 回补, 同锚 confirmed
            // Buy1, ZD 触线}，恰好三条——T5b 防悬挂不在集合内（闭腿集穷举）。
            self.step_down_paired(cfg, rows, c, bar, ledger, counters);
        } else if self.phase == VoicePhase::DownLeg {
            // SC7（legacy 路径同形式）：T7 回补的次级别买侧确认。
            let hard_raw = evs
                .iter()
                .any(|e| matches!(e.class, BspClass::Buy3) && e.confirmed);
            let sc7_ok = !cfg.sc_t7_close || rows.sub_confirm(k, crate::buysellpoint::Side::Buy);
            let hard = hard_raw && sc7_ok;
            if hard_raw && !sc7_ok {
                counters.n_sc_t7_holds += 1;
            }
            let pre = cfg.pre_type3
                && evs
                    .iter()
                    .any(|e| matches!(e.class, BspClass::Buy3) && !e.confirmed);
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

        // ── DownLeg：递归子 LOU（rev_sub_depth ≥ 1）——38课程式的方向镜像
        //    实例在 REV 窗口内运行。本块在 UpLeg 开腿块之前：父腿开窗 bar
        //    子层不步进（与 voice 自身"入场 bar 不步进"同语义）；父腿本 bar
        //    闭合时子树已被级联强闭，phase 已回 UpLeg，本块自然跳过。──
        if cfg.rev_sub_depth > 0 && cfg.rev_paired && self.phase == VoicePhase::DownLeg {
            self.step_sub_tree(cfg, rows, book, c, bar, ledger, counters);
        }

        // ── UpLeg：T1（REV 开腿）/ T2（拒因分计数）──
        if self.phase == VoicePhase::UpLeg && cfg.rev_mode && cfg.rev_paired {
            // 配对开腿（任务 2026-06-10）：kind 展开 + 深度门槛。
            self.rev_paired_open(
                cfg,
                rows,
                book,
                gate,
                entry_ladder,
                c,
                bar,
                ledger,
                frac_of,
                counters,
            );
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
            self.t4b_addons(
                cfg,
                rows,
                center_evs_home,
                c,
                bar,
                ledger,
                entry_ladder,
                frac_of,
                counters,
            );
        }

        // ── 域腿子循环 T3/T4（P5 逐字；C2：相位限制取消，任意相位运行）──
        self.step_osc(cfg, rows, book, c, bar, ledger, frac_of, counters);
    }

    /// 域腿子循环 T3/T4（P5 逐字，从 step 尾块原样抽出——Single/Cycle38 两模式
    /// 共用；C2 正交：osc 不在任何相位/循环变体内）。
    #[allow(clippy::too_many_arguments)]
    /// H4 滚动振幅准入判定（osc_amp_gate，纯读零副作用）：锚层典型中枢
    /// 相对振幅 θ_q（DepthRef 因果滚动 P50）≥ theta_cost_k × friction_rt
    /// ⇒ 准入。参照不可定义（warm-up，θ_q=None）⇒ 不准入（保守拒绝，
    /// 拒因归因在调用方）。
    fn osc_amp_admitted(cfg: &OrganicConfig, rows: &BarRows, k: usize) -> bool {
        use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
        let dr = rows
            .depth
            .expect("osc_amp_gate ⇒ 调用方必提供 DepthRef（capability，runner 恒提供）");
        dr.theta(k, None, SUB_COST_Q, SUB_COST_MIN_OBS)
            .is_some_and(|theta_q| theta_q >= cfg.theta_cost_k * cfg.friction_rt)
    }

    fn step_osc(
        &mut self,
        cfg: &OrganicConfig,
        rows: &BarRows,
        book: &CenterBook,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        frac_of: &dyn Fn(usize) -> f64,
        counters: &mut Counters,
    ) {
        if !cfg.osc_mode {
            return;
        }
        let k = self.ladder;
        let okey = SlotKey::osc(k);
        // anchor 是 Copy——复制快照规避 open_slot 借用与 close_diff &mut 冲突
        match ledger.open_slot(okey).map(|l| l.anchor) {
            Some(anchor) => match anchor {
                LegAnchor::Center {
                    cs,
                    boundary,
                    zg,
                    kind,
                } => {
                    // H3 锚层解码（osc_domain=TrendUpshift）：OscUp 腿锚在
                    // k+1 层中枢——全部出口判据（死亡/边界 sub_buy/上移出口）
                    // 跟随锚层运行（出口跟随锚，不跟随槽）。非上移腿
                    // alad==k 逐位不变（O0≡P5 零接触面）。
                    let alad = if kind == AnchorKind::OscUp { k + 1 } else { k };
                    let o_dead = cs.is_some_and(|s| book.is_dead(alad, s));
                    let sub_buy = alad >= 1 && rows.buy_any.get(alad - 1);
                    // 49课方向判据：只有三卖（向下终结）触发"不能回补"；
                    // 三买（向上终结）按"中枢向上移动时就应该满仓"立即回补。
                    let o_dead_down =
                        cfg.osc_sell3_no_recover && cs.is_some_and(|s| book.is_dead_down(alad, s));
                    // ── 出口1：中枢向上移动（osc_shift_close，49课"中枢向上
                    // 移动时就应该满仓"）——锚层新中枢形成（alive.seg_start
                    // 晚于锚 cs）且新中枢 ZD > 锚中枢 ZG ⇒ 旧中枢的震荡空腿
                    // 立即回补。补的是 type3 出口（中枢死亡强闭）在单边趋势中
                    // 失效的情况：回抽不发生 ⇒ confirmed type3 不来 ⇒ 锚中枢
                    // 不死 ⇒ 腿僵尸化（最终归宿 master 强平）。中枢上移是
                    // CenterBook.last 覆盖事件（无需 type3），出口在结构层
                    // 而非确认层。NaN 边界（zd/zg 缺失）比较恒 false——拒触发。
                    let shifted_up = cfg.osc_shift_close
                        && book.alive(alad).is_some_and(|lc| {
                            cs.is_some_and(|s| lc.seg_start > s) && zg.is_some_and(|g| lc.zd > g)
                        });
                    if o_dead && !o_dead_down {
                        ledger.close_diff(okey, c, bar); // 中枢死亡 → 强制回补
                    } else if boundary.is_some_and(|b| c <= b) && (!cfg.osc_buy_sub || sub_buy) {
                        ledger.close_diff(okey, c, bar);
                        counters.n_osc_zd_close += 1;
                    } else if shifted_up {
                        ledger.close_diff(okey, c, bar); // 中枢上移 → 满仓回补
                        counters.n_osc_shift_close += 1;
                    } else if o_dead_down {
                        // 49课严格形式：三卖后不能回补——保持卖出状态，
                        // 回补只在更低处（旧 ZD 触线）或 master 清算时发生。
                        counters.n_osc_dead_holds += 1;
                    }
                }
                LegAnchor::SegmentScale => unreachable!(
                    "osc 槽内不可能有段尺度锚——open 路径只以 Center 锚开 osc 腿；\
                     此 arm 是类型完备性要求，到达即 bug"
                ),
            },
            None => {
                // ── H3 级别上移分流（osc_domain=TrendUpshift，26课行183
                // "最好别按1分钟弄，5分钟甚至更长都可以"）：趋势态下 osc
                // 操作级别整体上移到 k+1——触发判据/锚/出口全按 k+1 级别
                // 中枢运行（操作级别上移是整个操作的级别上移，不是 k 层
                // 信号挂 k+1 床位）。趋势态下 k 层永不开（重路由不是
                // fallback）；k+1 越塔顶/无中枢/无触发 ⇒ 自然不开（机制
                // 预测：负域浅回调在 k+1 无信号）。盘整态走 k 层原路径
                // （ConsolidationOnly 放行分支同语义）。──
                let upshift = cfg.osc_domain == OscDomain::TrendUpshift
                    && rows.trend_row.expect(
                        "osc_domain=TrendUpshift ⇒ 调用方必提供趋势态行\
                         （capability，runner guard 恒拒缺行）",
                    )[k];
                if upshift {
                    if k + 1 < MAX_LADDER {
                        self.try_open_osc_at(
                            cfg,
                            rows,
                            book,
                            c,
                            bar,
                            ledger,
                            frac_of,
                            counters,
                            k + 1,
                            true,
                        );
                    }
                } else {
                    self.try_open_osc_at(
                        cfg, rows, book, c, bar, ledger, frac_of, counters, k, false,
                    );
                }
            }
        }
    }

    /// osc 开腿尝试（锚层参数化——H3 级别上移重构，2026-06-12）。
    ///
    /// `alad` = 锚层（本层路径 alad==k 逐位不变——O0≡P5 零接触面；H3 上移
    /// 路径 alad==k+1）。`upshift` = H3 重路由路径标记（锚 kind=OscUp +
    /// 路径归属计数）。触发判据/全部门链（域/H4/H1/H2/41课）以 alad 为参：
    /// 门的对象是开腿的锚中枢，锚在哪层门就查哪层（定义一致性，非可选）。
    /// 量 = frac_of(槽层 k)：量是物理归属（k 层声部的份额），上移改变操作
    /// 节奏不改资金归属（44课禁令对象=响应量错配非物理归属在册）。
    #[allow(clippy::too_many_arguments)]
    fn try_open_osc_at(
        &mut self,
        cfg: &OrganicConfig,
        rows: &BarRows,
        book: &CenterBook,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        frac_of: &dyn Fn(usize) -> f64,
        counters: &mut Counters,
        alad: usize,
        upshift: bool,
    ) {
        let k = self.ladder;
        let okey = SlotKey::osc(k);
        if let Some(lc) = book.alive(alad) {
            let sub_sell = alad >= 1 && rows.sell_any.get(alad - 1);
            if !book.is_frozen(alad) && alad >= 1 && c >= lc.zg && sub_sell {
                // ── osc 操作域（osc_domain=ConsolidationOnly，操作对象
                // 定义严格化）：38课中枢震荡操作的隐含前提是盘整走势
                // （价格在确立中枢内反复震荡）；锚中枢所在走势（本层
                // 尾 move，trend_row[k]）kind==Trend ⇒ 操作对象不存在，
                // 不开（49课"中枢向上移动时就应该满仓"；26课"单边上扬
                // 走势，短线最好别做"）。域判定先于一切点态门——对象
                // 不存在时门无可拦截之物。H3 上移路径恒域内（趋势态
                // 本身就是重路由条件，调用点已分流）；TrendUpshift 本层
                // 路径到达此处必为盘整态（显式判定保持不变式自证）。──
                let in_domain = upshift
                    || match cfg.osc_domain {
                        OscDomain::Any => true,
                        OscDomain::ConsolidationOnly | OscDomain::TrendUpshift => {
                            !rows.trend_row.expect(
                                "osc_domain≠Any ⇒ 调用方必提供趋势态行\
                                 （capability，runner guard 恒拒缺行）",
                            )[alad]
                        }
                    };
                if !in_domain {
                    counters.n_osc_domain_rejects += 1;
                    counters.osc_domain_reject_log.push((alad as u8, bar));
                }
                // ── H4 滚动振幅准入（osc_amp_gate，38课行32+35课
                // 行30）：锚层典型中枢相对振幅 θ_q（DepthRef 因果
                // 滚动 P50，50中枢窗，零前瞻）< theta_cost_k ×
                // friction_rt ⇒ "成本相对波幅不够小"，该级别该时段
                // osc 不准入（38课"选择历史上某级别平均震荡幅度
                // 最大的"的运行时形式——结构量替代回测盈亏符号
                // 白名单）。准入是域范畴（级别×时段可操作性），
                // 排在对象域之后、逐中枢时机门（H1/H2/41课）之前。
                // 参照不可定义（warm-up）⇒ 保守拒绝独立计数
                // （sub_cost_gate 先例）。只挡开腿，闭腿零接触。──
                else if cfg.osc_amp_gate && !Self::osc_amp_admitted(cfg, rows, alad) {
                    // 拒因归因（双拒因可分离——G3；H2 同构：条件
                    // 纯读，块内重读归因）
                    use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
                    let dr = rows.depth.expect(
                        "osc_amp_gate ⇒ 调用方必提供 DepthRef\
                         （capability，runner 恒提供）",
                    );
                    match dr.theta(alad, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
                        Some(_) => {
                            counters.n_osc_amp_rejects += 1;
                            counters.osc_amp_reject_log.push((alad as u8, bar, 1));
                        }
                        None => {
                            counters.n_osc_amp_noref_rejects += 1;
                            counters.osc_amp_reject_log.push((alad as u8, bar, 0));
                        }
                    }
                }
                // ── H1 candidate 冻结（osc_candidate_freeze，49课
                // 禁令窗口）：锚中枢存在未决离开段（candidate type3
                // 出现且价格未回边界内）⇒ 不开。49课行52"中枢完成后
                // 的向上移动时的差价是不能做的"，行68 当下判据 =
                // 次级别走势离开即启动窗口（不等 confirmed——在册
                // is_frozen 挂 confirmed Buy3，单边趋势中回抽不发生
                // ⇒ 永不触发 ⇒ 僵尸腿全在窗口内开出）。窗口是状态
                // 范畴（candidate 未决期间恒成立），先于点态门；
                // 只挡开腿——闭腿路径（Some(anchor) 分支）零接触。──
                else if cfg.osc_candidate_freeze && book.has_pending_departure(alad) {
                    counters.n_osc_cf_rejects += 1;
                    counters.osc_cf_reject_log.push((alad as u8, bar));
                }
                // ── H2 力度收敛门（osc_strength_gate，49课行38/52）：
                // 锚中枢最近两次向上离开段力度（H1 窗口 excursion
                // 直读）非收敛 ⇒ 拒开。历史 <2 条 = 新生保守默认
                // （无"震荡依旧"证据）；最近 > 前次 = 扩张 ⇒ 三类点
                // 预警（行38"还有些特殊的中枢震荡，会出现扩张的情况
                // ……最终形成第三类卖点"）。判据时点 = 开腿时刻，
                // 只读已完成历史段——覆盖 H1 的"窗口前"盲区（533号
                // BRN 裁决）。H1 检查之后（设计 §4 链序）；只挡开腿，
                // 闭腿路径零接触。──
                else if cfg.osc_strength_gate
                    && book.up_strength_verdict(alad, lc.seg_start) != UpStrengthVerdict::Converged
                {
                    match book.up_strength_verdict(alad, lc.seg_start) {
                        UpStrengthVerdict::Newborn => {
                            counters.n_osc_sg_newborn_rejects += 1;
                            counters.osc_sg_reject_log.push((alad as u8, bar, 0));
                        }
                        UpStrengthVerdict::Expanding => {
                            counters.n_osc_sg_expand_rejects += 1;
                            counters.osc_sg_reject_log.push((alad as u8, bar, 1));
                        }
                        UpStrengthVerdict::Converged => {
                            unreachable!("外层条件已排除 Converged——到达即 bug")
                        }
                    }
                }
                // ── 41课门（osc_l41_gate，域腿形态）：锚层直接父级别（alad+1）
                // 向上走势未完成（走势类型延续/中枢未死/三卖未坐实 ∧ 段窗口
                // 内无盘整背驰）⇒ 拒开逆向短差——强趋势中价格不回 ZD，中枢
                // 死亡后高位强制买回是结构性亏损（49课"中枢向上移动时就应该
                // 满仓"）。判据/越界语义同 rev_l41_gate（up_unexhausted
                // 对 alad+1 ≥ MAX_LADDER 返回 false 放行）。──
                else if cfg.osc_l41_gate
                    && rows
                        .l41
                        .expect(
                            "osc_l41_gate ⇒ 调用方必提供 TrendExhaustion\
                             （capability，runner 恒提供）",
                        )
                        .up_unexhausted(
                            alad + 1,
                            book,
                            (alad + 1 < MAX_LADDER)
                                .then(|| rows.dir(alad + 1))
                                .flatten(),
                        )
                {
                    counters.n_osc_l41_rejects += 1;
                    counters.osc_l41_reject_log.push((alad as u8, bar));
                } else if ledger.open_diff(
                    okey,
                    frac_of(k),
                    c,
                    bar,
                    LegAnchor::Center {
                        cs: Some(lc.seg_start),
                        boundary: Some(lc.zd),
                        zg: Some(lc.zg),
                        kind: if upshift {
                            AnchorKind::OscUp
                        } else {
                            AnchorKind::Osc
                        },
                    },
                ) {
                    counters.n_osc_open += 1;
                    if upshift {
                        counters.n_osc_upshift_open += 1;
                        counters.osc_upshift_open_log.push((k as u8, bar));
                    }
                }
            }
        }
    }

    // ════════════════════════════════════════════════════════
    // 38课循环模式（rev_cycle=Cycle38，2026-06-11 任务）
    // ════════════════════════════════════════════════════════

    /// 38课循环每 bar 一步。
    ///
    /// 严格形式声明（调研报告 §1.1/§4.1，090号近似声明义务）：38课原文的
    /// 操作对象是**同级别分解段**（该级别走势类型段），进出判据是段内部
    /// 结构的背驰/盘整背驰 + 位置分支。本实现是其事件流近似：
    /// - 卖出 = 本级别卖点（confirmed Sell1 ∨ 盘背卖）——与原文"根据其内部
    ///   结构判断其背驰或盘整背驰结束点，先卖出"**同型**（type1 卖 = 顺向段
    ///   衰竭的背驰本体，段终结前可见，调研报告 §2.2）；
    /// - 买回 = rev_cycle_close 消融轴（2026-06-11 任务）：SubAny（在册
    ///   C38base）= 次级别（k−1）买点——已否证（死因 = 买回级别错配，次级别
    ///   买点太快）；Buy1/Zd/Paired = 本级别判据（同锚 confirmed Buy1 / ZD
    ///   触线 / V2oa25 完整配对集）——区间套双向性：买回端也先过本级别定
    ///   时机，次级别只管精度（编排者原则）。位置分支（跌破后盘整背驰）
    ///   仍未实装（近似边界不变）；
    /// - 循环终止 = 宿主（k+1）趋势态翻落——原文"直到下一段向上的走势类型
    ///   相对前一段不创新高或盘整背驰为止"的 move 层读数：宿主尾 move
    ///   kind 离开 Trend ∨ direction 离开 Up。
    ///
    /// 同 bar 优先级：终止强闭 > 买回 > 卖出（终止是窗口存在性陈述，先于
    /// 窗口内操作；闭/开在 phase 上自然互斥，同 bar 不开即闭的零深度 churn）。
    #[allow(clippy::too_many_arguments)]
    fn step_cycle38(
        &mut self,
        cfg: &OrganicConfig,
        rows: &BarRows,
        book: &CenterBook,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        frac_of: &dyn Fn(usize) -> f64,
        counters: &mut Counters,
    ) {
        let k = self.ladder;
        let host = k + 1;
        let trow = rows
            .trend_row
            .expect("rev_cycle=Cycle38 ⇒ 调用方必提供趋势态行（capability，runner 恒提供）");
        // 宿主趋势态：尾 move 是趋势（≥2 同向中枢）∧ 方向向上（向上段运作，
        // 38课先卖后买短差的存续域；host 越界 = 无宿主可观测 ⇒ 循环不开）。
        let host_trend = host < MAX_LADDER && trow[host] && rows.dir(host) == Some(Direction::Up);

        if !self.cycle38_on {
            if self.phase == VoicePhase::UpLeg && host_trend {
                self.cycle38_on = true;
                counters.n_c38_enter += 1;
            } else {
                return;
            }
        } else if !host_trend {
            // 循环终止：未决腿强闭（卖了必须买回——38课程序内置追价买回），
            // 退回 RIDE。
            if self.phase == VoicePhase::DownLeg {
                self.c38_close_leg(1, c, bar, ledger, counters);
                counters.n_c38_forced_close += 1;
            }
            self.cycle38_on = false;
            counters.n_c38_exit += 1;
            return;
        }

        match self.phase {
            VoicePhase::DownLeg => {
                // 买回判据消融轴（rev_cycle_close，2026-06-11 任务）。编排者
                // 原则：区间套无论正反都存在——SubAny（在册 C38base）用次级别
                // 任意买点跳过本级别判据，违反区间套（已否证：OKLO −472.8/
                // BRN −70.4pp，胜率 33-36%）；Buy1/Zd/Paired 先过本级别判据
                // （同锚 Buy1 / ZD 触线），比较基准 = 开腿锚快照（RevLeg 携带）。
                // reason 编码与 RevCloseLog 同义（7/6/5/8），SubAny=0 无分解位。
                let (anchor_cs, zd) = match self.rev.as_ref() {
                    Some(r) => (r.anchor_cs, r.zd),
                    None => {
                        debug_assert!(false, "不变量：DownLeg ⇒ rev 存在");
                        return;
                    }
                };
                let evs = &rows.evs[k];
                let conf =
                    |class: BspClass| evs.iter().any(move |e| e.confirmed && e.class == class);
                let buy1_paired = || {
                    evs.iter().any(|e| {
                        e.confirmed && matches!(e.class, BspClass::Buy1) && e.cs == anchor_cs
                    })
                };
                let zd_touch = || zd.is_some_and(|z| c <= z);
                let reason: Option<u8> = match cfg.rev_cycle_close {
                    // 在册 C38base：次级别（k−1）买点 = 本级别回调段的次级别
                    // 结束确认（38课答疑：'不跌破'靠次级别内部结构确认）。
                    RevCycleClose::SubAny => (k >= 1 && rows.buy_any.get(k - 1)).then_some(0),
                    RevCycleClose::Buy1 => buy1_paired().then_some(5),
                    RevCycleClose::Zd => zd_touch().then_some(8),
                    // V2oa25 配对闭腿集的精确退化形式（RevCycleClose docstring
                    // 推导）：T7 > T6 > 同锚 Buy1 > ZD（step_down_paired 同序；
                    // 闭腿动作全同——同价全闭，优先序只决定 reason 归因）。
                    RevCycleClose::Paired => {
                        let pre = cfg.pre_type3
                            && evs
                                .iter()
                                .any(|e| !e.confirmed && matches!(e.class, BspClass::Buy3));
                        if conf(BspClass::Buy3) {
                            Some(7)
                        } else if pre {
                            Some(6)
                        } else if buy1_paired() {
                            Some(5)
                        } else if zd_touch() {
                            Some(8)
                        } else {
                            None
                        }
                    }
                    // 递归因果臂（编排者纠正 2026-06-11）：一卖→回落→二买/
                    // 三买涌现即买回机会——本级别任意 confirmed 买点（任意锚）。
                    // 归因优先序按结构强度 Buy3（回补位本体）> Buy2（底部确认）
                    // > Buy1（可能结束）；candidate Buy3 不入集（t6 在册负槽）。
                    RevCycleClose::BspAny | RevCycleClose::BspAnyZd => {
                        if conf(BspClass::Buy3) {
                            Some(7)
                        } else if conf(BspClass::Buy2) {
                            Some(10)
                        } else if conf(BspClass::Buy1) {
                            Some(11)
                        } else if cfg.rev_cycle_close == RevCycleClose::BspAnyZd && zd_touch() {
                            Some(8)
                        } else {
                            None
                        }
                    }
                    // post-hoc 探索臂（七臂数据驱动）：兑现型闭因子集——
                    // buy1_any（两标的唯二正闭因之一）+ ZD 几何兑现；
                    // buy3/t6 止损型负槽显式排除（其反事实入 holds 计数）。
                    RevCycleClose::Buy1AnyZd => {
                        if conf(BspClass::Buy1) {
                            Some(11)
                        } else if zd_touch() {
                            Some(8)
                        } else {
                            None
                        }
                    }
                };
                if let Some(reason) = reason {
                    self.c38_close_leg(reason, c, bar, ledger, counters);
                    counters.n_c38_close += 1;
                    match reason {
                        7 => counters.n_c38_close_t7 += 1,
                        6 => counters.n_c38_close_t6 += 1,
                        5 => counters.n_c38_close_buy1 += 1,
                        8 => counters.n_c38_close_zd += 1,
                        10 => counters.n_c38_close_buy2 += 1,
                        11 => counters.n_c38_close_buy1any += 1,
                        _ => {} // 0 = SubAny（在册计数语义，无分解位）
                    }
                } else {
                    // 反事实可观测（递归因果诊断）：本级别 confirmed 买点出现
                    // 但本 bar 未闭腿——被当前闭腿集漏掉的买回机会逐 bar 计数。
                    if conf(BspClass::Buy1) {
                        counters.n_c38_buy1_holds += 1;
                    }
                    if conf(BspClass::Buy2) {
                        counters.n_c38_buy2_holds += 1;
                    }
                    if conf(BspClass::Buy3) {
                        counters.n_c38_buy3_holds += 1;
                    }
                }
            }
            VoicePhase::UpLeg => {
                let evs = &rows.evs[k];
                let devs = &rows.devs[k];
                // 卖出：本级别卖点（震荡型触发集——confirmed Sell1 ∨ 盘背卖；
                // 逃逸型 Sell3 不入循环，V2o 裁决先验继承）。分量分离供
                // trigger 掩码（RevOpenLog 同编码：bit0=Sell1 / bit1=盘背）。
                let sell1 = evs
                    .iter()
                    .any(|e| e.confirmed && matches!(e.class, BspClass::Sell1));
                let consol = devs
                    .iter()
                    .any(|d| d.kind == DivKind::Consolidation && d.direction == Direction::Up);
                if !(sell1 || consol) {
                    return;
                }
                if book.is_frozen(k) {
                    counters.n_c38_frozen_rejects += 1;
                    return;
                }
                // 成本门（35课，任务第三步：每条循环短差腿都过）：该层典型
                // 中枢振幅 θ_q（因果滚动中位数，零前瞻）≥ k 倍往返摩擦。
                // 参照不可定义 ⇒ 保守拒绝并独立计数（不静默放行先例）。
                // 41课门（rev_l41_gate）显式不消费——见 RevCycle docstring。
                use super::config::{SUB_COST_MIN_OBS, SUB_COST_Q};
                let dr = rows.depth.expect(
                    "rev_cycle=Cycle38 ⇒ 调用方必提供 DepthRef（capability，runner 恒提供）",
                );
                match dr.theta(k, None, SUB_COST_Q, SUB_COST_MIN_OBS) {
                    Some(theta_q) => {
                        if theta_q < cfg.theta_cost_k * cfg.friction_rt {
                            counters.n_c38_cost_rejects += 1;
                            return;
                        }
                    }
                    None => {
                        counters.n_c38_cost_noref_rejects += 1;
                        return;
                    }
                }
                // 锚解析（needs_anchor 臂的数据依赖）：本级别买回判据的比较
                // 基准 = 卖点所在存活中枢快照——与 rev_paired_open 震荡型
                // 同一锚来源（账本是当前真相，事件自带 cs 可能陈旧）。
                // 锚不可定义 ⇒ 保守拒绝并计数（不静默放行先例）。SubAny/
                // BspAny 无锚依赖（任意锚买点不消费锚快照），不捕获不拒绝
                // （声明=能力；SubAny 开腿路径逐位不变，在册零接触）。
                let anchor = if !cfg.rev_cycle_close.needs_anchor() {
                    None
                } else {
                    match book.alive(k) {
                        Some(lc) if lc.zd.is_finite() && lc.zg.is_finite() => {
                            Some((lc.seg_start, lc.zd, lc.zg))
                        }
                        _ => {
                            counters.n_c38_nocenter_rejects += 1;
                            return;
                        }
                    }
                };
                let leg_anchor = match anchor {
                    // rev_paired_open 震荡型同形（Center 锚 + Type1 标注）
                    Some((cs, zd, _)) => LegAnchor::Center {
                        cs: Some(cs),
                        boundary: Some(zd),
                        zg: None,
                        kind: AnchorKind::Bsp(BspKind::Type1),
                    },
                    None => LegAnchor::SegmentScale,
                };
                if ledger.open_diff(SlotKey::rev(k, k), frac_of(k), c, bar, leg_anchor) {
                    counters.n_c38_open += 1;
                    self.rev = Some(match anchor {
                        Some((cs, zd, zg)) => RevLeg::new_paired(
                            k,
                            bar,
                            rows.dir(k),
                            RevOpenKind::Oscillation,
                            Some(cs),
                            Some(zd),
                            Some(zg),
                            false,
                            (sell1 as u8) | ((consol as u8) << 1),
                            c,
                        ),
                        None => RevLeg::new(k, bar, rows.dir(k)),
                    });
                    self.phase = VoicePhase::DownLeg;
                } else {
                    counters.n_rev_budget_rejects += 1;
                }
            }
        }
    }

    /// 循环短差腿闭合 + 对账面计数（win = profit > 0）+ 逐腿 (reason, profit)
    /// 日志（按买点类型的 payoff 分布读数；reason 编码见 Counters 分解位
    /// docstring，1 = 循环终止强闭）。
    fn c38_close_leg(
        &mut self,
        reason: u8,
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
                    counters.c38_pairs += 1;
                    let (_, cyc) = ledger.completed.last().expect("close_diff 刚 push");
                    let profit = cyc.profit();
                    if profit > 0.0 {
                        counters.c38_wins += 1;
                    }
                    counters.c38_cash += profit;
                    counters.c38_close_profits.push((reason, profit));
                }
            }
        }
        self.phase = VoicePhase::UpLeg;
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
                    BspClass::Sell2 => cfg.sell2_open,
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
            evs.iter()
                .find(|e| e.confirmed && matches!(e.class, BspClass::Sell3))
        } else {
            // V2o/V2of 消融：逃逸型开腿关（kind 标注首跑数据：逃逸型三标的
            // 一致为负）。Sell3 仍照常进 CenterBook（中枢死亡 ⇒ 震荡型分支
            // 的 alive 检查自然拒绝——不开任何腿，非降级为震荡型）。
            None
        };
        // 触发源分解（rev_open_log.trigger 位掩码：bit0=Sell1 / bit1=盘背卖 /
        // bit3=Sell2，T2o 轴）。
        let osc_sell1 = evs
            .iter()
            .any(|e| e.confirmed && matches!(e.class, BspClass::Sell1));
        // T2o：confirmed Sell2 = type1 卖后回升不创新高的顶部确认（17课对称）
        // ——震荡型触发源，锚解析与 Sell1 同路径（alive 中枢）。
        let osc_sell2 = cfg.sell2_open
            && evs
                .iter()
                .any(|e| e.confirmed && matches!(e.class, BspClass::Sell2));
        let osc_consol_raw = devs
            .iter()
            .any(|d| d.kind == DivKind::Consolidation && d.direction == Direction::Up);
        // R1：盘背触发要求次级别 Sell 证据落在 C 段窗口内（次级别最近 Up run，
        // 27课区间套一步收缩）。仅约束盘背触发源——Sell1/Sell3 触发不受影响。
        let r1_ok = !cfg.r1_sub_sell_open
            || self
                .sub_sell_bar
                .zip(self.sub_up_anchor)
                .is_some_and(|(b, a)| b >= a);
        let osc_consol = osc_consol_raw && r1_ok;
        if osc_consol_raw && !r1_ok {
            counters.n_rev_r1_rejects += 1;
        }
        let osc_sig = osc_sell1 || osc_consol || osc_sell2;
        if esc_ev.is_none() && !osc_sig {
            return;
        }
        // 触发掩码（RevOpenLog 同编码）。R2/R3 作用域 = trigger==2（盘背独占）
        // ——Sell1 参与触发的腿是 type1 卖 REV 腿，逻辑不可被改动（任务硬约束）；
        // Sell2 共现（bit3）同样使掩码 ≠ 2，腿自动退出 R2/R3 作用域。
        let trigger_mask: u8 = if esc_ev.is_some() {
            4
        } else {
            (osc_sell1 as u8) | ((osc_consol as u8) << 1) | ((osc_sell2 as u8) << 3)
        };
        let consol_only = trigger_mask == 2;
        counters.n_rev_attempts += 1;
        // SCr：全触发源的次级别卖侧确认（27课"每个卖点用次级别把握"的开腿形式）。
        // 与 R1 的差异见 sub_confirm docstring；约束全部触发源（Sell1/盘背/Sell3），
        // 非 R1 的盘背独占作用域。
        if cfg.sc_rev_open && !rows.sub_confirm(k, crate::buysellpoint::Side::Sell) {
            counters.n_sc_rev_open_rejects += 1;
            return;
        }
        // ── 41课门（rev_l41_gate，REV 主腿形态）：直接父级别（k+1）向上走势
        // 未完成（走势类型延续/中枢未死/三卖未坐实 ∧ 段窗口内无盘整背驰）
        // ⇒ 拒开反向腿——"大级别走势没有任何衰竭时参与反向小级别买卖点
        // 是刀口舔血"。父级别越界（k+1 ≥ MAX_LADDER）由 up_unexhausted
        // 返回 false 放行（无父级别可观测 = 证据缺失，门只在正面证据成立时关）。
        if cfg.rev_l41_gate {
            let te = rows
                .l41
                .expect("rev_l41_gate ⇒ 调用方必提供 TrendExhaustion（capability，runner 恒提供）");
            let parent_dir = (k + 1 < MAX_LADDER).then(|| rows.dir(k + 1)).flatten();
            if te.up_unexhausted(k + 1, book, parent_dir) {
                counters.n_rev_l41_rejects += 1;
                return;
            }
        }
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
        let (kind, anchor_cs, zd_line, zg_line, ext_allowed, log_zd, log_zg, depth_ok) =
            if let Some(e) = esc_ev {
                let (Some(zd), Some(zg)) = (e.zd, e.zg) else {
                    counters.n_rev_nocenter_rejects += 1;
                    return;
                };
                if !(zd.is_finite() && zg.is_finite()) {
                    counters.n_rev_nocenter_rejects += 1;
                    return;
                }
                let th = Self::effective_theta(cfg, rows, k, e.cs, counters);
                (
                    RevOpenKind::Escape,
                    e.cs,
                    None,
                    None,
                    false,
                    Some(zd),
                    Some(zg),
                    (zg - zd) / c >= th,
                )
            } else {
                let Some(lc) = book.alive(k) else {
                    counters.n_rev_nocenter_rejects += 1;
                    return;
                };
                if !(lc.zd.is_finite() && lc.zg.is_finite()) {
                    counters.n_rev_nocenter_rejects += 1;
                    return;
                }
                let th = Self::effective_theta(cfg, rows, k, Some(lc.seg_start), counters);
                let r2_leg = cfg.r2_anchor_zg && consol_only;
                let ok = if r2_leg {
                    // R2 深度门语义：可兑现段 = 开腿价到 ZG（原文保证域），
                    // c ≤ ZG 即零兑现空间（开腿点已在保证域内），并入深度拒。
                    c > lc.zg && (c - lc.zg) / c >= th
                } else {
                    // c ≤ ZD：触线条件在开腿 bar 即真 = 零利润空间，并入深度拒。
                    (lc.zg - lc.zd) / c >= th && c > lc.zd
                };
                // R2 延伸档独立门：中枢全振幅过 θ 才允许 ZG 后延伸持有至 ZD。
                let ext = r2_leg && (lc.zg - lc.zd) / c >= th;
                (
                    RevOpenKind::Oscillation,
                    Some(lc.seg_start),
                    Some(lc.zd),
                    Some(lc.zg),
                    ext,
                    Some(lc.zd),
                    Some(lc.zg),
                    ok,
                )
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
            LegAnchor::Center {
                cs: anchor_cs,
                boundary: zd_line,
                zg: None,
                kind: akind,
            },
        ) {
            counters.n_rev_open += 1;
            match kind {
                RevOpenKind::Oscillation => counters.n_rev_open_osc += 1,
                RevOpenKind::Escape => counters.n_rev_open_esc += 1,
            }
            if trigger_mask & 8 != 0 {
                counters.n_rev_sell2_open += 1;
            }
            counters.rev_open_log.push(RevOpenLog {
                ladder: k as u8,
                bar,
                kind: match kind {
                    RevOpenKind::Oscillation => 0,
                    RevOpenKind::Escape => 1,
                },
                trigger: trigger_mask,
                anchor_cs,
                zd: log_zd,
                zg: log_zg,
                price: c,
            });
            let mut leg = RevLeg::new_paired(
                k,
                bar,
                rows.dir(k),
                kind,
                anchor_cs,
                zd_line,
                zg_line,
                ext_allowed,
                trigger_mask,
                c,
            );
            // Seq38n：冻结第一段低点（含本 bar——开腿 bar 的 close 已计入
            // rev_run_low，与 Sequence38 子腿 seg1_low=run_low 冻结同语义）
            leg.seg1_low = Some(self.rev_run_low);
            self.rev = Some(leg);
            self.phase = VoicePhase::DownLeg;
        } else {
            counters.n_rev_budget_rejects += 1;
        }
    }

    /// 深度门 θ 的逐次取值（2026-06-11 θ 自适应任务）。
    ///
    /// Fixed = cfg.theta_depth 原语义（V2f/V2of/V2r 逐位不变）；
    /// AdaptiveQuantile = 该层因果滚动参照的 q 分位（排除当前锚自身——门槛
    /// 决策不得是被检对象自身振幅的函数），下界为成本门
    /// θ_eff = max(θ_quantile, theta_cost_k × friction_rt)（θ 相对化落地
    /// 任务：分位数可低至任意小，低于 k 倍往返摩擦的门放行期望必负的腿——
    /// 下界是相对化语义的严格组成部分，n_rev_theta_cost_floor 可观测）；
    /// 参照样本 < min_obs ⇒ 回退 cfg.theta_depth 并计 n_rev_theta_fallbacks
    /// （warm-up 不静默放行，固定回退值 1% 本就高于下界）。
    fn effective_theta(
        cfg: &OrganicConfig,
        rows: &BarRows,
        k: usize,
        anchor_cs: Option<i64>,
        counters: &mut Counters,
    ) -> f64 {
        match cfg.theta_mode {
            ThetaMode::Fixed => cfg.theta_depth,
            ThetaMode::AdaptiveQuantile { q, min_obs, .. } => {
                let dr = rows
                    .depth
                    .expect("theta_mode=AdaptiveQuantile ⇒ 调用方必提供 DepthRef（capability）");
                match dr.theta(k, anchor_cs, q, min_obs) {
                    Some(t) => {
                        let floor = cfg.theta_cost_k * cfg.friction_rt;
                        if t < floor {
                            counters.n_rev_theta_cost_floor += 1;
                            floor
                        } else {
                            t
                        }
                    }
                    None => {
                        counters.n_rev_theta_fallbacks += 1;
                        cfg.theta_depth
                    }
                }
            }
        }
    }

    /// 配对闭腿：T7（confirmed Buy3 回补位）> T6（candidate Buy3 预回补，
    /// pre_type3 轴）> T5（kind 配对 confirmed Buy1：震荡型同锚 / 逃逸型趋势
    /// 配对）> T2c（confirmed Buy2 同规则配对，buy2_close 轴，reason=10）
    /// > Seq38n（rev_seq_nobreak 轴，reason=11：不跌破第一段低点 ∧ 次级别
    /// 结构确认——38课:36 分岔1 + 答疑:296，事件证据之后几何触线之前）
    /// > ZD 触线（震荡型专属几何闭腿）。闭腿集穷举——盘背买 / 异锚 Buy1 /
    /// （buy2_close 关时）Buy2 不闭腿（"不接受 kind 不匹配的买点"），其反
    /// 事实以 n_rev_mismatch_holds 计数可观测。
    /// T2c 优先序依据：Buy2 是 Buy1 的确认（定律一——type1 说"可能结束"，
    /// type2 说"确实结束"），证据强度弱于 Buy1 本体、强于纯几何触线。
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
        let kind = rev
            .open_kind
            .expect("不变量：rev_paired 腿必携带 open_kind");
        let anchor_cs = rev.anchor_cs;
        let zd = rev.zd;
        let zg = rev.zg;
        let ext_allowed = rev.ext_allowed;
        let low_since_open = rev.low_since_open;
        let open_bar = rev.open_bar;
        // 腿生命期 ≈ 回拉走势窗口（腿在盘背/卖点处开，回拉自此展开）——
        // 次级别买侧证据 ∈ 此窗口 = "次级别回拉走势已完成"的可用判据。
        let sub_pullback_done = self.sub_buy_bar.is_some_and(|b| b >= open_bar);
        // SC7：T7 回补的次级别买侧确认（24课"回抽不破"的次级别形式）。
        // 确认缺失时本 bar 持有，延迟可观测（R3 同构风险的预注册计数）。
        let hard_raw = evs
            .iter()
            .any(|e| matches!(e.class, BspClass::Buy3) && e.confirmed);
        let sc7_ok = !cfg.sc_t7_close || rows.sub_confirm(k, crate::buysellpoint::Side::Buy);
        let hard = hard_raw && sc7_ok;
        if hard_raw && !sc7_ok {
            counters.n_sc_t7_holds += 1;
        }
        let pre_raw = cfg.pre_type3
            && evs
                .iter()
                .any(|e| matches!(e.class, BspClass::Buy3) && !e.confirmed);
        // R3：t6 预回补要求次级别"回跌不重回中枢"已成立证据（24课三买正典
        // 定义的次级别形式）：回拉走势完成证据 ∧ 回拉低点 > ZG。证据缺失时
        // candidate Buy3 不触发预回补（T7 confirmed 不变）。仅震荡型消费。
        let r3_ok = !cfg.r3_t6_sub_confirm
            || rev.open_trigger != 2 // 作用域：仅盘背独占触发腿（type1 卖不碰）
            || (sub_pullback_done && zg.is_some_and(|z| low_since_open > z));
        let pre = pre_raw && r3_ok;
        if pre_raw && !r3_ok && !hard {
            counters.n_rev_r3_holds += 1;
        }
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
        // T2c：confirmed Buy2 配对闭腿（镜像 Buy1 规则）。type2 事件的 cs/zd/zg
        // 从其 type1 前体逐字段复制（buysellpoint.rs _make_type2_point 移植）
        // ——同锚判据与 Buy1 同语义可比。
        let buy2_paired = cfg.buy2_close
            && evs.iter().any(|e| {
                e.confirmed
                    && matches!(e.class, BspClass::Buy2)
                    && match kind {
                        RevOpenKind::Oscillation => e.cs == anchor_cs,
                        RevOpenKind::Escape => true,
                    }
            });
        // Seq38n：38课位置分支——"不跌破第一段低点，重新买入"（38课:36
        // 分岔1）× 次级别结构确认（答疑:296"不跌破靠次级别判断？——对，
        // 需要该段内部结构的确认"⇒ 非纯几何触线）。第二段（本腿的向下
        // 回拉）未破第一段起点低点 ∧ 次级别买侧结构证据本 bar 共现 = 第二
        // 段完成且整体上涨延续 ⇒ 买回。Sequence38 子腿 nobreak 岔的主腿
        // 同构（低点参照换 rev_run_low 冻结值）。
        let seq_nobreak = cfg.rev_seq_nobreak
            && rev.seg1_low.is_some_and(|s1| low_since_open > s1)
            && rows.sub_confirm(k, crate::buysellpoint::Side::Buy);
        let zd_touch = zd.is_some_and(|z| c <= z);
        // R2 兑现锚：价格回入原文保证域 [ZD, ZG]（"理论只能保证其回拉原来的
        // 走势中枢"）。zd ≤ zg ⇒ zd_touch ⊆ zg_touch，下方分支序无重叠。
        let zg_touch = cfg.r2_anchor_zg
            && rev.open_trigger == 2 // 作用域：仅盘背独占触发腿（type1 卖不碰）
            && zg.is_some_and(|z| c <= z);
        let log_close = |reason: u8, counters: &mut Counters| {
            counters.rev_close_log.push(RevCloseLog {
                ladder: k as u8,
                open_bar,
                bar,
                reason,
                price: c,
            });
        };
        if hard {
            log_close(7, counters);
            self.close_all_paired(kind, c, bar, ledger, counters);
            counters.n_rev_close_t7 += 1;
        } else if pre {
            log_close(6, counters);
            self.close_all_paired(kind, c, bar, ledger, counters);
            counters.n_rev_close_t6 += 1;
        } else if buy1_paired {
            log_close(5, counters);
            self.close_all_paired(kind, c, bar, ledger, counters);
            counters.n_rev_close_t5 += 1;
        } else if buy2_paired {
            log_close(10, counters);
            self.close_all_paired(kind, c, bar, ledger, counters);
            counters.n_rev_buy2_close += 1;
        } else if seq_nobreak {
            log_close(11, counters);
            self.close_all_paired(kind, c, bar, ledger, counters);
            counters.n_rev_seq_nobreak_close += 1;
        } else if zg_touch {
            if sub_pullback_done || !ext_allowed {
                // 基础兑现：保证域内 ∧（次级别回拉已完成 ∨ 无延伸档资格）
                log_close(9, counters);
                self.close_all_paired(kind, c, bar, ledger, counters);
                counters.n_rev_zg_close += 1;
            } else if zd_touch {
                // 延伸目标兑现：次级别下跌仍在生长，价格已穿越全中枢
                log_close(8, counters);
                self.close_all_paired(kind, c, bar, ledger, counters);
                counters.n_rev_zd_close += 1;
            }
            // else：延伸持有（次级别回拉未完成 ∧ 延伸档门过，目标 ZD）
        } else if zd_touch {
            log_close(8, counters);
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

    /// 递归子 LOU 驱动（存在论守卫 + 预算基读取 + 惰性实例化）。
    fn step_sub_tree(
        &mut self,
        cfg: &OrganicConfig,
        rows: &BarRows,
        book: &CenterBook,
        c: f64,
        bar: i64,
        ledger: &mut OrganicLedger,
        counters: &mut Counters,
    ) {
        let k = self.ladder;
        // 存在论下限按模式分流：Zhongshu 需 k−1 有中枢/买卖点概念
        // （FIRST_BSP_LADDER）；Fractal 只需 k−1 有方向行——bi 级（1）即下限。
        let sub_floor = match cfg.sub_mode {
            SubMode::Zhongshu | SubMode::Sequence38 | SubMode::CounterSeg => FIRST_BSP_LADDER,
            SubMode::Fractal => 1,
        };
        if k < 1 || k - 1 < sub_floor {
            return; // 存在论终止：k−1 子域不可定义（§4.1）
        }
        let Some(rev) = self.rev.as_mut() else { return };
        // 配对腿恒单 tranche（tranche×rev_paired 在 runner 入口拒绝）
        let parent_path = RevPath::single(
            rev.tranches
                .first()
                .expect("RevLeg 不变式：tranches 非空")
                .level,
        );
        let parent_key = SlotKey::rev_path(k, parent_path);
        let Some(pshares) = ledger.open_slot(parent_key).map(|l| l.cycle.shares) else {
            return; // 父腿槽不在（开腿被预算拒后的相位残留）——无预算基
        };
        let sub = rev
            .sub
            .get_or_insert_with(|| Box::new(SubLou::new(k - 1, parent_path.child(k - 1))));
        sub.step(cfg, rows, book, c, bar, ledger, k, pshares, counters);
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
        // 级联不变式（递归赋格）：父腿闭合先平掉全部子树腿——子腿的域
        // （REV 窗口）随父腿闭合而消失。开放问题 Q2 的本实验裁决：父腿
        // 兑现优先，未决子腿同价强闭（n_sub_forced_close 可观测）。
        if let Some(mut sub) = self.rev.as_mut().and_then(|r| r.sub.take()) {
            sub.cascade_close_with_self(self.ladder, c, bar, ledger, counters);
        }
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
        // Seq38n 中间循环复位：新一轮第一段低点从闭腿 close 重新累计
        // （38课程式在 LONG 区间内反复——Sequence38 子腿 run_low=c 同语义）。
        self.rev_run_low = c;
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
                    CenterEvent::Terminated {
                        direction: Direction::Down,
                        ..
                    } => {
                        rev.terminated_down_seen = true;
                    }
                    CenterEvent::Terminated {
                        direction: Direction::Up,
                        ..
                    }
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
                rev.tranches.push(RevTranche {
                    level,
                    last_dir: dir_row[level],
                });
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
#[path = "level_operating_unit_tests.rs"]
mod tests;
