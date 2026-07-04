//! 全互斥买卖点解释器——固定优先级互斥化 `C_j = P_j ∧ ⋀_{k<j} ¬P_k`（《完整的策略.pdf》
//! §7 P1..P10 + 买卖点alpha2.pdf Doc2 §9 定理1 的互斥化构造）。
//!
//! ## 契约锚（`docs/formal-chain/完整的策略.pdf` §7，#124 真统一重分解）
//!
//! 原始谓词 `P_1..P_m`（m=10）**可重叠**（同一时刻多个 P_j 同时成立）：
//! - `P1` = 风险强平（force_flat ⟹ K_Θ={0} + 活动腿全 RiskExit）
//! - `P2` = TW StageII ∧ H>0，CloseOverlay（关 legacy ShortDiff 重叠腿，真产订单）
//! - `P3` = TW 可退本金，Withdraw（无订单账本事件 RecoverCapital）
//! - `P4` = TW EnterEarning（无订单相变事件）
//! - `P5` = 正规 CloseRoot（一/二类反向 ⟹ 根清仓）
//! - `P6` = 正规 ReduceCore（三类反向 ⟹ 核心仓减仓）
//! - `P7` = Close ShortDiff（短差子声部反向确认关闭）
//! - `P8` = Open Root（Ambient/root 候选，slot 空）
//! - `P9` = Open ShortDiff（ShortDiff 角色候选，父声部 active）
//! - `P10` = Record StructBreak（Flat/无类/slot 冲突 ⟹ 记录不执行）
//! - `P0` = Hold（无谓词命中，C_0 兜底）
//!
//! **历史**：本模块原锚 alpha2 §5 的 P1..P8 分解（P2/P3=close_long/close_short 未 typed、
//! 无 TW 谓词）。#124 裁定4「真统一」后按 PDF §7 重分解——不是「加两谓词」，是 close 桶
//! typed 拆（P5/P6/P7 经 [`reverse_exit_type`] 单源）+ TW 三阶段进链（P2/P3/P4）+ open
//! 拆 root/shortdiff（P8/P9）。alpha2 定理 1 的互斥化构造形式不变。
//!
//! **固定优先级互斥化**（alpha2 Doc3 §6 构造，PDF §7 同型）：
//! ```text
//! C_1 = P_1
//! C_j = P_j ∧ ⋀_{k<j} ¬P_k    (j = 2..m)
//! C_0 = ⋀_{j=1}^m ¬P_j         （兜底 Hold）
//! ```
//!
//! **定理 1（alpha2 Doc2 §9）**：`Σ_{j=0}^m 1[C_j] = 1`（全互斥 + 全定义）。
//! 证明：若无 P_j 成立则 C_0 唯一成立；否则取最小成立索引 r，则 C_r 唯一成立
//! （所有 j<r 因 P_j=0 不成立，所有 j>r 因 ¬P_r=0 不成立）。
//!
//! 全定义解释器（PDF §16）：`I_Θ(A_t, Γ_t, TW_t, Risk_t) = (D_t, O_t, L_t, TWEvent_t)`。
//! 生产实装 = [`super::interp::interpret`]（候选级三桶 fold，P5..P10）+
//! [`super::coverage::pi_theta_step_traced`] I_Θ 组合层（bar 级 P1..P4 全局分支：P1 上游
//! 短路 RiskExit / P2 合成 close 桶 CloseOverlay / P3/P4 产 TWEvent_t 消耗当步裁决）。
//!
//! ## 两级谓词结构（bar 级 × 候选级）
//!
//! PDF §7 的裁决链在生产中分两层兑现，本 oracle 相应分两层对拍：
//! - **bar 级 P1..P4**（[`StepPredicateCtx`]）：不依赖单个候选，成立时屏蔽全部候选的
//!   P5..P10（组合层短路/合成桶/消耗裁决）。谓词向量中每个候选同值注入 ⟹ 每个候选的
//!   互斥化裁决都落 C_1..C_4 ⟹ 候选不产 close/open 动作——与组合层「gamma 全部推迟
//!   record」行为一致。对拍锚：coverage tests `pi_theta_step_traced_p1_*`/`_p2_*`/`_p3_*`/`_p4_*`。
//! - **候选级 P5..P10**（[`predicates_of`]）：ctx 全 false 时逐候选从 PDF 谓词语义独立
//!   重推导（不引用 interp 规则序），typed close 经 [`reverse_exit_type`] 单源。对拍锚：
//!   [`tests`] 的 `shadow_fold_bucket_equivalence`（桶级 + typed 精确类号）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! **L0 结构定理**（非 L2 alpha）：`Σ_{j=0}^m 1[C_j]=1` 是固定优先级互斥化的**组合逻辑恒等式**
//! （alpha2 Doc2§9 证毕，零信息增量同义反复）。Rust 穷举 2^10 验证 = **L1 管线正确性**
//! （验证互斥化实装无 bug，不验证谓词 P_j 经验有效——P_j 的市场触发率/盈利性是 L2 未覆盖；
//! P2/P3/P4 生产触发在 codex GAP3 裁定 A' 后**现实可达**——已实现利润经 `TwEvent::Realize`
//! 入 free，可达性见证 runner `pi_loop_realized_profit_reaches_earning_shares`；L0 同价下仍
//! 不可达，见 `earning_shares_unreachable_l0_same_price_zero_pnl`）。
//!
//! ## 身份：D1 等价测试 oracle，**非生产路径**（codex-q2-d1 §Q2 裁定 + 裁定4 反装饰约束）
//!
//! 本模块 `mutex_class` **零生产消费者**——生产裁决走 interp fold + I_Θ 组合层。保留用途 =
//! P1..P10 等价 property test 对拍参照。裁定4 明文：本 oracle 扩展**不得先于生产 typed 接线
//! 单独落**（否则为装饰性 oracle）——本次扩展与生产接线（f9333e21b2 TW 进 fold + c4c2a027ad
//! P1 全局分支 + 8150acb97f typed close 归因）同批，P1..P4 谓词均有真实生产分支对应。
//! 曾并存的 `closed_loop/mutex_interp.rs`（9 谓词 Lean 镜像）已按 §Q2 删除。

use super::coverage::Vertical;
use super::interp::{reverse_exit_type, ActiveLeg, Candidate, ExitType};
use super::voice::VoiceSide;

/// 谓词数 m=10（P_1..P_10，PDF §7）。
pub const M: usize = 10;

/// 互斥化裁决类号。`C_0`=兜底 Hold（无谓词成立），`C_j`(j=1..10)=最小成立谓词索引。
///
/// `Σ_{j=0}^10 1[C_j]=1`：给定任意谓词向量恰好对应一个 `MutexClass`（全互斥 + 全定义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutexClass {
    /// C_0：⋀_j ¬P_j（无任何谓词成立，Hold 兜底）。
    C0,
    /// C_j：P_j ∧ ⋀_{k<j} ¬P_k（最小成立索引 j∈1..=10，存 1-based 谓词号）。
    Cj(u8),
}

/// 原始谓词向量 `P ∈ {0,1}^10`（可重叠——多个分量可同时为 true）。
///
/// 字段名对齐 PDF §7 谓词语义（1-based 谓词号见各字段）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Predicates {
    /// P1：风险强平（`KThetaRiskGate.force_flat`——同一 RiskState 派生，裁定4 单权威源）。
    pub risk_liquidate: bool,
    /// P2：TW StageII ∧ H>0 ⟹ CloseOverlay（关 legacy ShortDiff 重叠腿，真产订单）。
    pub close_overlay: bool,
    /// P3：TW 退本金 Withdraw（`stage_progression` → RecoverCapital，无订单账本事件）。
    pub tw_withdraw: bool,
    /// P4：TW EnterEarning（`stage_progression` → EnterEarning，无订单相变）。
    pub tw_enter_earning: bool,
    /// P5：正规 CloseRoot（一/二类反向 ⟹ 根清仓；[`reverse_exit_type`] 单源判据）。
    pub close_root: bool,
    /// P6：正规 ReduceCore（三类反向 ⟹ 核心仓减仓）。
    pub reduce_core: bool,
    /// P7：Close ShortDiff（短差子声部反向确认关闭——入场角色压过触发类）。
    pub close_short_diff: bool,
    /// P8：Open Root（非 ShortDiff 角色候选，slot 空 ⟹ 开根/级联腿）。
    pub open_root: bool,
    /// P9：Open ShortDiff（ShortDiff 角色候选，slot 空 ⟹ 开短差子声部）。
    pub open_short_diff: bool,
    /// P10：Record StructBreak——**诚实口径**：覆盖 interp record 桶全部三源（Flat 无向 /
    /// 无类 / slot 冲突「记录不加仓」，codex-q2-d1 §4 加仓子动作不存在的声明沿袭）。
    pub record_struct_break: bool,
}

impl Predicates {
    /// 谓词向量按 1-based 索引读 `P_j`（j∈1..=10）。索引越界 ⟹ panic（内部不变量）。
    fn p(&self, j: usize) -> bool {
        match j {
            1 => self.risk_liquidate,
            2 => self.close_overlay,
            3 => self.tw_withdraw,
            4 => self.tw_enter_earning,
            5 => self.close_root,
            6 => self.reduce_core,
            7 => self.close_short_diff,
            8 => self.open_root,
            9 => self.open_short_diff,
            10 => self.record_struct_break,
            _ => unreachable!("谓词索引 j={j} 越界（合法 1..={M}）"),
        }
    }
}

/// 固定优先级互斥化：`C_j = P_j ∧ ⋀_{k<j} ¬P_k`，兜底 `C_0 = ⋀_j ¬P_j`。
///
/// 实装即定理证明的构造形式——取**最小成立谓词索引** r（C_r 唯一成立）；无成立谓词 ⟹ C_0。
/// 全定义（任意输入返回唯一 [`MutexClass`]）+ 全互斥（`Σ_j 1[C_j]=1`，构造保证恰一个）。
///
/// **边界条件**：全 false ⟹ C0；存在 true ⟹ Cj(最小成立索引)。优先级**不可交换**——更小索引的
/// 谓词成立时屏蔽所有更大索引（P1 强平 ≻ P2 CloseOverlay ≻ P3/P4 TW ≻ P5..P7 typed close ≻
/// P8/P9 open ≻ P10 record，PDF §7 优先级链）。
///
/// **认识论 L0**：纯组合逻辑（无数据依赖），互斥性是定义内蕴的同义反复。
pub fn mutex_class(p: &Predicates) -> MutexClass {
    for j in 1..=M {
        if p.p(j) {
            return MutexClass::Cj(j as u8);
        }
    }
    MutexClass::C0
}

/// bar 级谓词上下文（P1..P4——不依赖单个候选，成立时屏蔽全部候选的 P5..P10）。
///
/// 生产对应物（裁定4 真统一，f9333e21b2）：
/// - `force_flat` ↔ `KThetaRiskGate.force_flat`（组合层上游短路，StepTrace.risk_exits）。
/// - `tw_close_overlay` ↔ 组合层 P2 分支（`stage==CapitalRecovered ∧ shortdiff 活动腿非空`）。
/// - `tw_withdraw`/`tw_enter_earning` ↔ `stage_progression` 派生事件的 pattern match。
///
/// [`StepPredicateCtx::from_tw`] 把判据从 TW 账本态显式重推导（oracle 独立性——与组合层
/// if 条件对拍的第二实现，分叉即测试红）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StepPredicateCtx {
    /// P1：风险强平。
    pub force_flat: bool,
    /// P2：TW StageII ∧ H>0（legacy ShortDiff 重叠腿仍开）。
    pub tw_close_overlay: bool,
    /// P3：TW 退本金 ready（CostReduction ∧ holding≥notional_in ∧ free 足额）。
    pub tw_withdraw: bool,
    /// P4：TW EnterEarning ready（EnterReady 五合取）。
    pub tw_enter_earning: bool,
}

impl StepPredicateCtx {
    /// 从 TW 账本态独立重推导 bar 级谓词（PDF §7 语义显式化，oracle 第二实现）。
    ///
    /// - P2 = `stage==CapitalRecovered ∧ has_overlay_legs`（H>0 = 生产 legacy ShortDiff
    ///   活动腿非空，与 `TwState.open_legacy_legs` 计数同源）。
    /// - P3/P4 = [`stage_progression`](super::super::closed_loop::transition::stage_progression)
    ///   派生事件（单源判据——此处不重写阶段推进逻辑，只做事件→谓词投影）。
    pub fn from_tw(
        force_flat: bool,
        tw: &super::ledger::TwState,
        policy: &super::ledger::RiskPolicy,
        risk_mode: super::super::closed_loop::state::RiskMode,
        has_overlay_legs: bool,
    ) -> StepPredicateCtx {
        use super::super::closed_loop::transition::stage_progression;
        use super::ledger::{TStage, TwEvent};
        let tw_close_overlay = tw.stage == TStage::CapitalRecovered && has_overlay_legs;
        let (tw_withdraw, tw_enter_earning) = match stage_progression(policy, tw, risk_mode) {
            Some(TwEvent::RecoverCapital(_)) => (true, false),
            Some(TwEvent::EnterEarning) => (false, true),
            _ => (false, false),
        };
        StepPredicateCtx { force_flat, tw_close_overlay, tw_withdraw, tw_enter_earning }
    }
}

/// interp 三桶动作类（候选级裁决 C5..C10/C0 的桶归属：close/open/record）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionBucket {
    /// 𝒟_x：关闭活动腿（interp 规则2 反向平仓，typed P5/P6/P7）。
    Close,
    /// ℬ_x：开启新 slot（interp 规则3，P8/P9）。
    Open,
    /// 𝒦_x：记录不执行（interp 规则1 无类/无向、规则4 slot 冲突，P10；C0 Hold 同桶）。
    Record,
}

/// `MutexClass` → 候选级三桶（`None` = bar 级全局分支 C1..C4，无候选桶归属——组合层
/// 短路/合成桶/消耗裁决，候选不经 interp fold 分流；对拍锚在 coverage tests，非本桥）。
pub fn bridge_bucket(c: MutexClass) -> Option<ActionBucket> {
    match c {
        MutexClass::Cj(1) | MutexClass::Cj(2) | MutexClass::Cj(3) | MutexClass::Cj(4) => None,
        MutexClass::Cj(5) | MutexClass::Cj(6) | MutexClass::Cj(7) => Some(ActionBucket::Close),
        MutexClass::Cj(8) | MutexClass::Cj(9) => Some(ActionBucket::Open),
        _ => Some(ActionBucket::Record), // Cj(10) 记录 / C0 Hold 兜底
    }
}

/// **D1 桥接：生产候选 + fold 状态 + bar 级 ctx → PDF 谓词向量 P∈{0,1}^10**。
///
/// 独立于 `interp::interpret` 的规则序，从 PDF §7 谓词语义**重新推导**每个 P_j——这样
/// `bridge_bucket(mutex_class(predicates_of(..)))` 与 interp 实际桶归属的对拍才非循环（若
/// interp 的 ≺_Θ 序与 PDF 优先级分叉，对拍会红，见 `tests::shadow_fold_bucket_equivalence`）。
///
/// 参数：
/// - `ctx`：bar 级 P1..P4（成立 ⟹ 该候选的裁决落 C_1..C_4，P5..P10 被互斥化屏蔽——
///   与组合层「消耗当步裁决」一致）。
/// - `working`/`opened_slots`：interp fold 的**同一份**内部状态快照。
/// - `entry_v_of`：被关腿的入场角色查询（腿声部身份入场固定；生产对应 runner 在飞表
///   `LedgerOpen.entry_v`）——P5/P6/P7 typed 拆分经 [`reverse_exit_type`] 单源。
pub fn predicates_of(
    ctx: &StepPredicateCtx,
    c: &Candidate,
    working: &[(ActiveLeg, bool)],
    opened_slots: &[(u32, VoiceSide)],
    entry_v_of: &dyn Fn(&ActiveLeg) -> Vertical,
) -> Predicates {
    let mut p = Predicates {
        risk_liquidate: ctx.force_flat,
        close_overlay: ctx.tw_close_overlay,
        tw_withdraw: ctx.tw_withdraw,
        tw_enter_earning: ctx.tw_enter_earning,
        ..Predicates::default()
    };
    // 规则1 语义：非方向 / 无类候选 ⟹ 无买卖点动作 ⟹ P10（记录）。
    if c.dir == VoiceSide::Flat || c.bsp_class == u8::MAX {
        p.record_struct_break = true;
        return p;
    }
    // P5/P6/P7 typed close：同级别首个未关闭且被 g 反向的腿（interp 规则2 同判据），
    // typed 经 reverse_exit_type(入场角色, 触发类) 单源——与 runner ledger 消费端同函数。
    let closed_leg = working
        .iter()
        .find(|(l, closed)| !closed && l.level == c.level && super::exec::reverse_signal(l.dir, &c.bits))
        .map(|(l, _)| l);
    if let Some(leg) = closed_leg {
        match reverse_exit_type(entry_v_of(leg), c.bsp_class) {
            ExitType::CloseShortDiff => p.close_short_diff = true, // P7
            ExitType::ReduceCore => p.reduce_core = true,          // P6
            ExitType::CloseRoot => p.close_root = true,            // P5
            ExitType::RiskExit | ExitType::Hold => {
                unreachable!("reverse_exit_type 只产三 close 枚举")
            }
        }
        return p;
    }
    // slot 占用：同级别同向未关闭腿 ∨ 本 fold 已开同 slot ⟹ P10（记录，无加仓子动作）。
    let slot_occupied = working
        .iter()
        .any(|(l, closed)| !closed && l.level == c.level && l.dir == c.dir)
        || opened_slots.iter().any(|&(lv, d)| lv == c.level && d == c.dir);
    if slot_occupied {
        p.record_struct_break = true;
        return p;
    }
    // slot 空 ⟹ 开仓：P9 Open ShortDiff（ShortDiff 角色）/ P8 Open Root（其余）。
    match c.role.v {
        Vertical::ShortDiff => p.open_short_diff = true, // P9
        _ => p.open_root = true,                         // P8
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★可证伪核心（alpha2 Doc2§9 定理 1，m=10）：穷举全部 2^10=1024 谓词组合，断言每个
    /// 组合**恰好一个** C_j 成立（`Σ_{j=0}^10 1[C_j]=1`）。若存在组合 Σ≠1 则 fail。
    ///
    /// 本测试独立重算每个 C_j 的指示函数 `1[C_j]`（不调 `mutex_class`，避免循环论证），
    /// 逐组合求和断言 ==1，并交叉验证命中类号与 `mutex_class` 一致（实装 == 独立定义）。
    #[test]
    fn mutex_total_exhaustive_2pow10() {
        for bits in 0u32..(1 << M) {
            let p = decode(bits);

            let mut sum = 0usize;
            let mut hit_class: Option<MutexClass> = None;

            // C_0 兜底。
            let any = (1..=M).any(|j| p.p(j));
            if !any {
                sum += 1;
                hit_class = Some(MutexClass::C0);
            }
            // C_j = P_j ∧ ⋀_{k<j} ¬P_k。
            for j in 1..=M {
                let prefix_all_false = (1..j).all(|k| !p.p(k));
                let cj = p.p(j) && prefix_all_false;
                if cj {
                    sum += 1;
                    hit_class = Some(MutexClass::Cj(j as u8));
                }
            }

            assert_eq!(
                sum, 1,
                "bits={bits:010b}: Σ_j 1[C_j]={sum} ≠ 1（互斥性/全定义破裂，定理 1 反例）"
            );
            assert_eq!(
                mutex_class(&p),
                hit_class.unwrap(),
                "bits={bits:010b}: mutex_class 实装 ≠ 独立重算的命中类号"
            );
        }
    }

    /// 全定义：任意输入返回唯一 MutexClass（穷举已覆盖；显式断言无 panic + 全 false → C0）。
    #[test]
    fn mutex_total_definedness() {
        assert_eq!(mutex_class(&Predicates::default()), MutexClass::C0);
        for bits in 0u32..(1 << M) {
            let _ = mutex_class(&decode(bits));
        }
    }

    /// 优先级屏蔽：P1（风险强平）成立时屏蔽所有更高索引（即使 P2..P10 全 true）⟹ C_1。
    #[test]
    fn priority_p1_masks_all() {
        let p = decode((1 << M) - 1); // 全 true
        assert_eq!(mutex_class(&p), MutexClass::Cj(1), "P1 成立 ⟹ C_1（屏蔽 P2..P10）");
    }

    /// bar 级链序：P2 CloseOverlay 屏蔽 P3/P4（TW 事件）与 P5..P10；P3 屏蔽 P4..P10。
    #[test]
    fn priority_bar_level_chain() {
        let p2 = Predicates {
            close_overlay: true,
            tw_withdraw: true,
            close_root: true,
            ..Default::default()
        };
        assert_eq!(mutex_class(&p2), MutexClass::Cj(2), "P2 ≻ P3 ≻ P5");
        let p3 = Predicates { tw_withdraw: true, tw_enter_earning: true, open_root: true, ..Default::default() };
        assert_eq!(mutex_class(&p3), MutexClass::Cj(3), "P3 ≻ P4 ≻ P8");
    }

    /// 优先级取最小成立索引：typed close P5..P7 内部与 open P8/P9 的链序。
    #[test]
    fn priority_min_index() {
        let p = Predicates {
            reduce_core: true,      // P6
            open_short_diff: true,  // P9
            record_struct_break: true, // P10
            ..Default::default()
        };
        assert_eq!(mutex_class(&p), MutexClass::Cj(6), "最小成立索引 r=6 ⟹ C_6");
    }

    /// bits → Predicates 解码（bit j-1 ↔ P_j，1-based）。
    fn decode(bits: u32) -> Predicates {
        let b = |j: usize| (bits >> (j - 1)) & 1 == 1;
        Predicates {
            risk_liquidate: b(1),
            close_overlay: b(2),
            tw_withdraw: b(3),
            tw_enter_earning: b(4),
            close_root: b(5),
            reduce_core: b(6),
            close_short_diff: b(7),
            open_root: b(8),
            open_short_diff: b(9),
            record_struct_break: b(10),
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  bar 级谓词 ctx 派生对拍（StepPredicateCtx::from_tw vs 组合层分支判据）
    // ──────────────────────────────────────────────────────────────────────
    use super::super::super::closed_loop::state::RiskMode;
    use super::super::ledger::{RiskPolicy, TStage, TwState};

    /// ★P2/P3/P4 判据派生：from_tw 与组合层 pi_theta_step_traced 分支使用同一判据源
    /// （stage==II∧H>0 / stage_progression），此处对拍谓词投影正确性。
    #[test]
    fn step_ctx_from_tw_predicates() {
        let pol = RiskPolicy::baseline();
        // P2：StageII + 有重叠腿。
        let s2 = TwState { stage: TStage::CapitalRecovered, open_legacy_legs: 1, ..TwState::initial() };
        let c2 = StepPredicateCtx::from_tw(false, &s2, &pol, RiskMode::Normal, true);
        assert!(c2.tw_close_overlay && !c2.tw_withdraw && !c2.tw_enter_earning);
        // P3：CostReduction + holding≥notional_in + free 足额。
        let s3 = TwState { free: 100, holding: 100, notional_in: 100, ..TwState::initial() };
        let c3 = StepPredicateCtx::from_tw(false, &s3, &pol, RiskMode::Normal, false);
        assert!(c3.tw_withdraw && !c3.tw_close_overlay && !c3.tw_enter_earning);
        // P4：EnterReady 五合取。
        let s4 = TwState {
            withdrawn: 100,
            notional_in: 100,
            stage: TStage::CapitalRecovered,
            ..TwState::initial()
        };
        let c4 = StepPredicateCtx::from_tw(false, &s4, &pol, RiskMode::Normal, false);
        assert!(c4.tw_enter_earning && !c4.tw_withdraw);
        // inert：initial（notional_in=0）全 false。
        let c0 = StepPredicateCtx::from_tw(false, &TwState::initial(), &pol, RiskMode::Normal, false);
        assert_eq!(c0, StepPredicateCtx::default());
    }

    /// ★bar 级谓词注入 ⟹ 任意候选的裁决落 C_1..C_4（P5..P10 被互斥化屏蔽）——与组合层
    /// 「消耗当步裁决」行为对应（行为侧见 coverage tests `pi_theta_step_traced_p2/p3/p4_*`）。
    #[test]
    fn bar_level_ctx_masks_candidate_predicates() {
        let c = cand(0, 0, 10, VoiceSide::Long, 1, buy(1), Vertical::Ambient);
        let ambient = |_: &ActiveLeg| Vertical::Ambient;
        for (ctx, expect) in [
            (StepPredicateCtx { force_flat: true, ..Default::default() }, MutexClass::Cj(1)),
            (StepPredicateCtx { tw_close_overlay: true, ..Default::default() }, MutexClass::Cj(2)),
            (StepPredicateCtx { tw_withdraw: true, ..Default::default() }, MutexClass::Cj(3)),
            (StepPredicateCtx { tw_enter_earning: true, ..Default::default() }, MutexClass::Cj(4)),
        ] {
            let cls = mutex_class(&predicates_of(&ctx, &c, &[], &[], &ambient));
            assert_eq!(cls, expect, "bar 级谓词 ⟹ 候选裁决落对应 C_j（open 候选被屏蔽）");
            assert_eq!(bridge_bucket(cls), None, "C_1..C_4 = bar 级全局分支，无候选桶归属");
        }
    }

    // ──────────────────────────────────────────────────────────────────────
    //  D1 逐候选级等价 shadow-fold property test（ctx 全 false：P5..P10 vs interp 三桶）
    //  桶级一致必须过；typed close 场景另断言精确 Cj（P5/P6/P7 与消费端判据同源见证）。
    // ──────────────────────────────────────────────────────────────────────
    use super::super::super::classifier::recursive_tower::ElementId;
    use super::super::coverage::{Dir, Horizontal, OperationRole};
    use super::super::exec::reverse_signal;
    use super::super::interp::{interpret, theta_key};
    use super::super::super::types::BspBits;
    use std::collections::HashSet;

    fn role(v: Vertical) -> OperationRole {
        OperationRole { h: Horizontal::First, v, delta: Dir::Plus }
    }

    #[allow(clippy::too_many_arguments)]
    fn cand(
        gi: usize,
        level: u32,
        src: usize,
        dir: VoiceSide,
        bsp_class: u8,
        bits: BspBits,
        v: Vertical,
    ) -> Candidate {
        Candidate {
            level,
            source_index: src,
            bits,
            dir,
            bsp_class,
            role: role(v),
            nest_confirmed: true,
            gamma_index: gi,
            force: None,
        }
    }

    fn leg(level: u32, dir: VoiceSide, src: usize) -> ActiveLeg {
        ActiveLeg {
            level,
            dir,
            source_index: src,
            lambda: src,
            id: ElementId { level, ordinal: src as u64 },
            parent_id: None,
            is_boundary_root: true,
            op_parent: None,
        }
    }

    fn buy(k: u8) -> BspBits {
        match k {
            1 => BspBits { buy1: true, ..Default::default() },
            2 => BspBits { buy2: true, ..Default::default() },
            _ => BspBits { buy3: true, ..Default::default() },
        }
    }
    fn sell(k: u8) -> BspBits {
        match k {
            1 => BspBits { sell1: true, ..Default::default() },
            2 => BspBits { sell2: true, ..Default::default() },
            _ => BspBits { sell3: true, ..Default::default() },
        }
    }

    /// shadow fold：按 theta_key 排序，逐候选对拍 mutex 桶 vs 真 interp 桶（ctx 全 false）。
    /// `entry_v_of` 给腿入场角色（typed P5/P6/P7 判据输入）；`expect_cj` 非空时按 gamma_index
    /// 另断言精确类号（typed close 见证）。shadow `working`/`opened` 用 authoritative interp
    /// 桶推进（镜像 interp 规则2/3）。
    fn assert_bucket_equiv_typed(
        gamma: &[Candidate],
        active: &[ActiveLeg],
        entry_v_of: &dyn Fn(&ActiveLeg) -> Vertical,
        expect_cj: &[(usize, u8)],
    ) {
        let ctx = StepPredicateCtx::default();
        let real = interpret(gamma, active);
        let open_idx: HashSet<usize> = real.open.iter().map(|c| c.gamma_index).collect();
        let rec_idx: HashSet<usize> = real.record.iter().map(|c| c.gamma_index).collect();

        let mut ordered: Vec<&Candidate> = gamma.iter().collect();
        ordered.sort_by(|a, b| theta_key(a).cmp(&theta_key(b)));

        let mut working: Vec<(ActiveLeg, bool)> = active.iter().map(|&l| (l, false)).collect();
        let mut opened: Vec<(u32, VoiceSide)> = Vec::new();

        for c in ordered {
            let cls = mutex_class(&predicates_of(&ctx, c, &working, &opened, entry_v_of));
            let mbucket = bridge_bucket(cls)
                .expect("ctx 全 false ⟹ 候选级裁决 C5..C10/C0，恒有桶归属");
            let ibucket = if open_idx.contains(&c.gamma_index) {
                ActionBucket::Open
            } else if rec_idx.contains(&c.gamma_index) {
                ActionBucket::Record
            } else {
                ActionBucket::Close // 候选被消费为关闭者（不在 open/record 桶）
            };
            assert_eq!(
                mbucket, ibucket,
                "候选 gamma_index={} 桶级分叉：mutex(P1..P10)={:?} vs interp(≺_Θ)={:?}（真矛盾 ⟹ /escalate）",
                c.gamma_index, mbucket, ibucket
            );
            if let Some(&(_, want)) = expect_cj.iter().find(|&&(gi, _)| gi == c.gamma_index) {
                assert_eq!(
                    cls,
                    MutexClass::Cj(want),
                    "候选 gamma_index={} 精确类号分叉（typed close 判据 vs 消费端）",
                    c.gamma_index
                );
            }
            // 推进 shadow 状态镜像 interp（authoritative = ibucket）。
            match ibucket {
                ActionBucket::Close => {
                    if let Some(i) = working
                        .iter()
                        .position(|(l, cl)| !cl && l.level == c.level && reverse_signal(l.dir, &c.bits))
                    {
                        working[i].1 = true;
                    }
                }
                ActionBucket::Open => opened.push((c.level, c.dir)),
                ActionBucket::Record => {}
            }
        }
    }

    fn ambient(_: &ActiveLeg) -> Vertical {
        Vertical::Ambient
    }

    /// ★D1 桶级等价（生成域覆盖：空/同级同向/同级反向/双向腿/跨级/fold内重复slot/ShortDiff/
    /// 无类无向）+ typed close 精确类号（P5 一类反向根清仓）。
    #[test]
    fn shadow_fold_bucket_equivalence() {
        use Vertical::{Ambient, ShortDiff};
        let l = VoiceSide::Long;
        let s = VoiceSide::Short;
        let f = VoiceSide::Flat;

        // S1 空 active，两 slot 开仓（跨级 + 多空）。
        assert_bucket_equiv_typed(
            &[cand(0, 0, 10, l, 1, buy(1), Ambient), cand(1, 1, 11, s, 1, sell(1), Ambient)],
            &[],
            &ambient,
            &[(0, 8), (1, 8)], // P8 Open Root
        );
        // S2 fold 内重复同 slot：第一开（P8），第二记录（P10 slot_this_fold）。
        assert_bucket_equiv_typed(
            &[cand(0, 0, 10, l, 1, buy(1), Ambient), cand(1, 0, 12, l, 2, buy(2), Ambient)],
            &[],
            &ambient,
            &[(0, 8), (1, 10)],
        );
        // S3 反向平仓：持多 L0（Ambient 根）+ 一类卖候选 ⟹ close，typed=P5 CloseRoot。
        assert_bucket_equiv_typed(
            &[cand(0, 0, 20, s, 1, sell(1), Ambient)],
            &[leg(0, l, 5)],
            &ambient,
            &[(0, 5)],
        );
        // S4 同向 slot 占用 ⟹ record（P10）。
        assert_bucket_equiv_typed(
            &[cand(0, 0, 20, l, 1, buy(1), Ambient)],
            &[leg(0, l, 5)],
            &ambient,
            &[(0, 10)],
        );
        // S5 无类无向候选 ⟹ record（P10）。
        assert_bucket_equiv_typed(
            &[cand(0, 0, 20, f, u8::MAX, BspBits::default(), Ambient)],
            &[],
            &ambient,
            &[(0, 10)],
        );
        // S6 ShortDiff 角色开仓 ⟹ open，typed=P9 Open ShortDiff。
        assert_bucket_equiv_typed(
            &[cand(0, 0, 20, s, 1, sell(1), ShortDiff)],
            &[],
            &ambient,
            &[(0, 9)],
        );
        // S7 跨级同向不冲突：持多 L0 + buy L1 ⟹ open（不同 slot，P8）。
        assert_bucket_equiv_typed(
            &[cand(0, 1, 20, l, 1, buy(1), Ambient)],
            &[leg(0, l, 5)],
            &ambient,
            &[(0, 8)],
        );
        // S8 双向腿共存 + 反向候选：sell 候选反向平掉 Long 腿 ⟹ close（P5）。
        assert_bucket_equiv_typed(
            &[cand(0, 0, 20, s, 1, sell(1), Ambient)],
            &[leg(0, l, 5), leg(0, s, 6)],
            &ambient,
            &[(0, 5)],
        );
        // S9 多候选同刻混合：L0 反向平（P5）+ L1 开（P8）+ L0 无类记录（P10）。
        assert_bucket_equiv_typed(
            &[
                cand(0, 0, 30, s, 1, sell(1), Ambient),
                cand(1, 1, 31, l, 1, buy(1), Ambient),
                cand(2, 0, 32, f, u8::MAX, BspBits::default(), Ambient),
            ],
            &[leg(0, l, 5)],
            &ambient,
            &[(0, 5), (1, 8), (2, 10)],
        );
        // S10 三类反向 ⟹ P6 ReduceCore（typed 拆分：trigger_class=3 减核，非根清仓）。
        assert_bucket_equiv_typed(
            &[cand(0, 0, 20, s, 3, sell(3), Ambient)],
            &[leg(0, l, 5)],
            &ambient,
            &[(0, 6)],
        );
        // S11 ShortDiff 入场腿被反向关 ⟹ P7 CloseShortDiff（入场角色压过触发类——
        //     即使一类触发也归 P7，reverse_exit_type 单源语义）。
        assert_bucket_equiv_typed(
            &[cand(0, 0, 20, l, 1, buy(1), Ambient)],
            &[leg(0, s, 5)],
            &|_| Vertical::ShortDiff,
            &[(0, 7)],
        );
        // S12 二类反向 ⟹ P5 CloseRoot（二类是一类的次级确认，同属根反转——G4 判据表）。
        assert_bucket_equiv_typed(
            &[cand(0, 0, 20, s, 2, sell(2), Ambient)],
            &[leg(0, l, 5)],
            &ambient,
            &[(0, 5)],
        );
    }
}
