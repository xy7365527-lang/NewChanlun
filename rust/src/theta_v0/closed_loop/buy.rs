//! 买侧 Θ 闭环——port **`Origin/ThetaInstantiation.lean`（#117）买侧见证**（消买侧 vacuous gap）。
//!
//! ## 工位定位（SG-1 买侧 vacuous gap，按需工位）
//!
//! 卖侧已 port `Origin/SellPointRecog.lean`（#120）+ `SellClosedLoop.lean`（#121）于 [`super::sell`]。
//! 但**买侧 `Origin.ThetaInstantiation` 链在 rust 此前无独立实装**——`tests/theta_v0_lean_parity.rs`
//! 模块头（:39-50）已诚实标注：grep `recogChanlun`/`decisionLedgerDelta`/`chanlunTransition` 于
//! `rust/src` 仅命中卖侧对偶 + 注释，无买侧函数；买侧 delta 在 sell.rs:406-407 是**硬编码常量**
//! `open_root_da: i64 = 1`（不是调 rust 函数）。故 `tests/theta_v0_lean_parity.rs` 对买侧的「bit-exact」
//! 是**经卖侧对偶间接**（`buy_sell_A_delta_mirror`）——逻辑 vacuous（买侧 Lean 见证经卖侧覆盖，
//! 买侧 rust 链从未被独立断言）。
//!
//! 本文件**独立实装买侧 ThetaInstantiation 链**——`recog_chanlun_buy`/`buy_decision_ledger_delta`/
//! `buy_transition`，bit-exact 镜像 Lean `ThetaInstantiation.lean` 买侧见证（`recogChanlun` :151 /
//! `decisionLedgerDelta` :256 / `chanlunTransition` :297），使买侧获得**直接** Lean↔Rust parity
//! （见 `tests/theta_v0_buy_parity.rs`），消除间接对偶的 vacuous。
//!
//! ## bit-exact 对齐 Lean（语义对应表，ThetaInstantiation.lean）
//!
//! | Rust | Lean（Origin.ThetaInstantiation） | 语义 |
//! |------|------------------------------------|------|
//! | [`BuyDecision`] | `ThetaInstantiation.ChanlunDecision`（:131） | openRoot/accreteCore/hold 三态 |
//! | [`BuyEndpoint`] | `BspClassification.BspEndpoint`（#113）买侧投影 | 买点判据输入端点 |
//! | [`recog_chanlun_buy`] | `ThetaInstantiation.recogChanlun`（:151） | 买点识别核（真 case-split） |
//! | [`is_type1_buy`] | `BspClassification.IsType1`（:152 内联判据） | 底背驰=破中枢∧背驰 |
//! | [`is_type3_buy`] | `BspClassification.IsType3Buy`（:155-156 内联判据） | 回抽不破 ZG（zg < retrace）|
//! | [`buy_decision_ledger_delta`] | `ThetaInstantiation.decisionLedgerDelta`（:256） | (0,+1,0)/(0,+2,0)/noop |
//! | [`buy_transition`] | `ThetaInstantiation.chanlunTransition`（:297） | 买点决策驱动 ledger 一步 |
//!
//! ## ★分支判据忠实 port（与卖侧的差异，不是简单对偶）
//!
//! Lean 买侧 `recogChanlun`（:151-161）的判据**不是**卖侧 `recogChanlunSell`（sell.rs:108-119）的
//! 机械镜像，二者在 side 检查上不同——必须忠实 port 买侧 Lean 的真实形式，**不可**靠对偶反推：
//! - **第一类**（:152）：`brokeCenter = true ∧ IsDivergence divPair`。★买侧 Lean **不检查 side**
//!   （第一类底背驰判据只要破中枢 + 背驰；卖侧 sell.rs:109 检查 `side==Short`——这是卖侧 port
//!   的形式，买侧 Lean :152 无 side 检查，本文件忠实保留无 side 检查）。
//! - **第三类**（:155-156）：`side = long ∧ leftCenter ∧ firstRetrace ∧ zg < retracePrice`。买侧
//!   检查 `side==Long`，且位置判据是「不破 ZG = retracePrice 高于 zg」（`zg < retracePrice`），
//!   与卖侧「不破 ZD = retrace 低于 zd」（`retrace < zd`）位置镜像。
//! - **否则**（:159）：hold。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! 本文件 = **L0/L1**（结构镜像：买点判据 + 买侧闭环与 Lean `ThetaInstantiation` 买侧定义对齐 =
//! 验证管线正确性，零信息增量）。`cargo test` 通过 = 买侧识别核真区分两类买点 + 买侧转移真改
//! ledger 且保 R=Π-A-W，**不**是缠论盈利/实盘有效声明（L2/L3）。
//!
//! ## 诚实边界（no声明膨胀，与 Lean ThetaInstantiation §诚实边界一致，:45-51/588-595）
//!
//! - ✗ 这些应对策略盈利/最优/实盘有效（L3 EmpiricalDomain）。
//! - ✗ 仓位数额（1/2）来自缠论（Θ_risk 参数——缠论分类只决定 delta 的**类型/符号**：增核 >
//!   建根仓的账本足迹序，不决定精确数额，数额下游 Θ_risk 填）。Π 在买侧恒 0（买入无实现损益，
//!   Lean decisionLedgerDelta openRoot/accreteCore 的 dΠ=0；与卖侧 closeRoot 的 dΠ=+1 时间不对称）。
//! - ✗ bsp 事件的**自动识别**（从 K 线流自动判定每端点属哪类）——本文件消费已判定的 [`BuyEndpoint`]
//!   （由 #113 BspClassification 判据层提供），不证遍历构造（still-MISSING：bspOf 全自动，#113 标）。
//! - ✗ 第二类买点（买卖点定律一「由次级别一类构成」需次级别递归，#113 still-MISSING-D）——本文件
//!   买侧闭环覆盖第一类/第三类/延续三态，与 Lean `recogChanlun` :151 三态一致（诚实留白非 workaround）。

use super::super::strategy::ledger::{ledger_step, LedgerEvent};
use super::super::types::{Side, Tick};
use super::state::AssemblyState;

/// 买点缠论决策 `BuyDecision`（port `ThetaInstantiation.ChanlunDecision`，:131，对偶卖侧三态）。
///
/// - `OpenRoot`：建根仓（第一类底背驰买点——跌破中枢后背驰，第24课，§10.1）。
/// - `AccreteCore`：增核（第三类买点——离开中枢回抽不破 ZG，§10.1）。
/// - `Hold`：保持（力度延续/非买点，§11——后段力度 ≥ 前段，趋势延续无背驰）。
///
/// 三态由真缠论买点判据区分（见 [`recog_chanlun_buy`]），与卖侧（closeRoot/reduceCore/hold）对偶。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuyDecision {
    /// 第一类底背驰买点 → 建根仓（Lean `ChanlunDecision.openRoot`，:132）。
    OpenRoot,
    /// 第三类回抽买点 → 增核（Lean `ChanlunDecision.accreteCore`，:133）。
    AccreteCore,
    /// 力度延续/非买点 → 保持（Lean `ChanlunDecision.hold`，:134）。
    Hold,
}

/// 买点端点 `BuyEndpoint`（port `BspClassification.BspEndpoint` 的买侧消费字段，#113，经 ThetaInstantiation）。
///
/// 买点判据所需的端点语义字段（对齐 Lean `BspEndpoint`，只保留 [`recog_chanlun_buy`] 消费的字段）：
/// - `side`：买卖向（第三类买点判据要求 `Side::Long`，Lean :155；第一类不检查 side，Lean :152）。
/// - `broke_center`：是否突破/跌破中枢（第一类底背驰前提，Lean `e.bsp.brokeCenter`，:152）。
/// - `is_divergence`：背驰判据（底背驰=力度背驰 forceC < forceA，Lean `IsDivergence e.bsp.divPair`，:152）。
///   ★rust 在此领先 Origin：背驰由已实装 MACD（divergence.rs `segments_diverge`）真算，Lean
///   `divPair` 是抽象力度对。本字段承载 MACD 计算后的背驰 bool（rust 真算，非外部参数）。
/// - `left_center`：是否离开中枢（第三类前提，Lean `e.bsp.leftCenter`，:155）。
/// - `first_retrace`：是否第一次回抽（第三类前提，Lean `e.bsp.firstRetrace`，:155）。
/// - `retrace_price`：回抽低点价（第三类判据 `center_zg < retrace_price`，Lean `e.bsp.retracePrice`，:156）。
/// - `center_zg`：所在中枢核心上沿 ZG（第三类参照点，Lean `e.bsp.center.zg`，:156）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuyEndpoint {
    pub side: Side,
    pub broke_center: bool,
    pub is_divergence: bool,
    pub left_center: bool,
    pub first_retrace: bool,
    pub retrace_price: Tick,
    pub center_zg: Tick,
}

/// 第一类买点判据 `is_type1_buy`（port `ThetaInstantiation.recogChanlun` :152 内联 `IsType1` 判据）。
///
/// 底背驰 = 下跌趋势向下**跌破**中枢后的背驰点：破中枢 ∧ 背驰。★忠实 Lean :152——**不检查 side**
/// （Lean 第一类分支判据 `brokeCenter = true ∧ IsDivergence divPair` 无 side 检查，与卖侧
/// sell.rs:86「side==Short ∧ ...」不同；买侧底背驰共用力度序判据，Lean `type1_buy_sell_share_divergence`）。
pub fn is_type1_buy(e: &BuyEndpoint) -> bool {
    e.broke_center && e.is_divergence
}

/// 第三类买点判据 `is_type3_buy`（port `ThetaInstantiation.recogChanlun` :155-156 内联 `IsType3Buy` 判据）。
///
/// 离开中枢后回抽低点**不破 ZG**：side=long ∧ 离开中枢 ∧ 第一次回抽 ∧ `center_zg < retrace_price`。
/// 与第三类卖点（不升破 ZD，之下）位置三态镜像（Lean `type3_buy_sell_position_mirror`）。
pub fn is_type3_buy(e: &BuyEndpoint) -> bool {
    e.side == Side::Long && e.left_center && e.first_retrace && e.center_zg < e.retrace_price
}

/// 买点识别核 `recog_chanlun_buy`（port `ThetaInstantiation.recogChanlun`，:151-161，本文件核心）。
///
/// 从买点端点真 case-split on 买点判据，识别买侧应对意图（缠论买点规则真编码，对偶卖侧 recog）：
/// - **第24课 / §10.1 第一类买点**：破中枢 ∧ 背驰 ⟹ `OpenRoot`（底背驰建根仓，Lean :152-154）。
/// - **§10.1 第三类买点**：否则 long ∧ 离开中枢 ∧ 第一次回抽 ∧ zg < retrace ⟹ `AccreteCore`（增核，Lean :155-158）。
/// - **§11**：否则（含力度延续 = 非背驰，无买点）⟹ `Hold`（保持，Lean :159-161）。
///
/// ★分支优先级与 Lean 严格一致（:151）：第一类（破中枢背驰）优先于第三类（离开中枢回抽）——
/// 第三类语境是「离开中枢后回抽」，与第一类「跌破中枢背驰」互斥分支（Lean `recog_type3_accreteCore`
/// 前提 `brokeCenter = false`，:182）。★第一类分支忠实 Lean :152 **无 side 检查**。
pub fn recog_chanlun_buy(e: &BuyEndpoint) -> BuyDecision {
    if e.broke_center && e.is_divergence {
        // 第24课 / §10.1 第一类买点：跌破中枢 + 底背驰 ⟹ 建根仓（Lean :152-154，无 side 检查）。
        BuyDecision::OpenRoot
    } else if e.side == Side::Long && e.left_center && e.first_retrace && e.center_zg < e.retrace_price {
        // §10.1 第三类买点：离开中枢 + 第一次回抽 + 不破 ZG ⟹ 增核（Lean :155-158）。
        BuyDecision::AccreteCore
    } else {
        // §11：力度延续 / 非买点 ⟹ 保持（Lean :159-161）。
        BuyDecision::Hold
    }
}

/// 买点账本 delta `buy_decision_ledger_delta`（port `ThetaInstantiation.decisionLedgerDelta`，:256-259）。
///
/// 把真缠论买点决策映为账本三元组 delta `(dΠ, dA, dW)`（喂给 [`ledger_step`]），与卖侧
/// `sellDecisionLedgerDelta`（closeRoot (1,-1,0) / reduceCore (0,-2,0) / hold 0）镜像对偶：
/// - `OpenRoot`（第一类底背驰建根仓）⟹ `(0, 1, 0)`：A += 1（资本化建仓，占用储备 R）。Lean :257。
///   与卖侧 closeRoot (1,-1,0) 在 A 分量镜像（建根仓 A+1 ↔ 清根仓 A-1）。★买侧 dΠ=0（买入无
///   实现损益——Lean openRoot dΠ=0；卖侧清仓 dΠ=+1 的时间不对称，Lean §诚实标注已审查，非定义冲突）。
/// - `AccreteCore`（第三类回抽增核）⟹ `(0, 2, 0)`：A += 2（增核分配更多，≠ 建根仓 +1，非退化）。Lean :258。
///   与卖侧 reduceCore (0,-2,0) 在 A 分量严格镜像（增核 A+2 ↔ 减核 A-2）。
/// - `Hold`（力度延续保持）⟹ `(0, 0, 0)`：账本不变（无买点不动账，与卖侧 hold 共用）。Lean :259。
///
/// ★诚实（与 Lean :253-254 一致）：数额 1/2 是 Θ_risk 占位常量（具体数额是运行时数据 L2）——缠论
/// 买点分类决定 delta 的类型/符号（allocate 增核 > 建根仓），不决定精确数额。W 恒 0（TW 端
/// still-MISSING，不硬塞进单账本买侧）。
pub fn buy_decision_ledger_delta(d: BuyDecision) -> (i64, i64, i64) {
    match d {
        BuyDecision::OpenRoot => (0, 1, 0),    // 建根仓：allocate A+1（Lean :257）
        BuyDecision::AccreteCore => (0, 2, 0), // 增核：allocate A+2（≠ 建根仓，非退化，Lean :258）
        BuyDecision::Hold => (0, 0, 0),        // 保持：noop（Lean :259）
    }
}

/// 把 `(dΠ, dA, dW)` delta 应用到 ledger（复用 [`ledger_step`]，保 R=Π-A-W）。
///
/// 与 Lean `chanlunTransition` 内 `ledgerStep z.ledger delta.1 delta.2.1 delta.2.2`（:300）对齐：
/// dΠ→Realize、dA→Allocate、dW→Withdraw。每个分量经 [`ledger_step`]（每分支重算 R）保恒等。
/// 三分量串联应用 ⟹ 终态保 R=Π-A-W（Lean `chanlunTransition_preserves_ledger_inv`，:307）。
fn apply_ledger_delta(
    l: &super::super::strategy::ledger::LedgerComp,
    d_pi: i64,
    d_a: i64,
    d_w: i64,
) -> super::super::strategy::ledger::LedgerComp {
    let l = ledger_step(l, LedgerEvent::Realize(d_pi));
    let l = ledger_step(&l, LedgerEvent::Allocate(d_a));
    ledger_step(&l, LedgerEvent::Withdraw(d_w))
}

/// ★★买侧真缠论闭环转移 `buy_transition`（port `ThetaInstantiation.chanlunTransition`，:297，本文件核心）。
///
/// 闭环数据流（真缠论买点，对偶卖侧 [`super::sell::sell_transition`]）：
/// 1. [`recog_chanlun_buy`] 用买点判据识别真缠论买点决策 d（第一类建根仓/第三类增核/延续保持）。
/// 2. [`buy_decision_ledger_delta`] 把买点决策映为账本 delta。
/// 3. [`ledger_step`]（保 R=Π-A-W）用该 delta 更新账本，写回完整 [`AssemblyState`]。
///
/// ★主动买点识别建仓（非被动 bar 驱动）：ledger 下一态由**端点的真缠论买点分类**决定，
/// 且保账本恒等。positions 同步反映建仓（建根仓 +1，增核 +2，保持不变）。
///
/// ★诚实标注（与 Lean §诚实边界一致）：只更新 ledger + positions（买侧账本足迹），micro/tw/risk/phase
/// 保持（买侧决策只动 R=Π-A-W 账本，TW 端 still-MISSING）。
pub fn buy_transition(x: &AssemblyState, e: &BuyEndpoint) -> AssemblyState {
    let d = recog_chanlun_buy(e);
    let (d_pi, d_a, d_w) = buy_decision_ledger_delta(d);
    let new_ledger = apply_ledger_delta(&x.ledger_state, d_pi, d_a, d_w);
    // positions 反映建仓（建根仓 +1；增核 +2；保持不变），与 A 分量同步。
    let new_positions = match d {
        BuyDecision::OpenRoot => x.positions + 1,
        BuyDecision::AccreteCore => x.positions + 2,
        BuyDecision::Hold => x.positions,
    };
    AssemblyState {
        ledger_state: new_ledger,
        positions: new_positions,
        ..*x
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::strategy::ledger::LedgerComp;

    /// 第一类买点见证（port Lean `eventType1`，ThetaInstantiation.lean:358-369）：破中枢 + 底背驰。
    ///
    /// Lean `eventType1.bsp`：`brokeCenter=true`，`divPair={forceA:=8, forceC:=2}`（IsDivergence: 2<8）。
    /// 注：rust `is_divergence` 直接给 Lean `IsDivergence{forceA:=8,forceC:=2}` 的判定结果 `true`（2<8）。
    fn sample_type1_buy() -> BuyEndpoint {
        BuyEndpoint {
            side: Side::Long,
            broke_center: true,    // Lean eventType1.bsp.brokeCenter = true
            is_divergence: true,   // Lean IsDivergence{forceA:=8, forceC:=2} = (2 < 8) = true
            left_center: false,    // Lean eventType1.bsp.leftCenter = false
            first_retrace: false,  // Lean eventType1.bsp.firstRetrace = false
            retrace_price: 5,      // Lean eventType1.bsp.retracePrice = 5
            center_zg: 20,         // Lean witnessCenter.zg = 20（5<20 ⟹ 不满足第三类「zg<retrace」）
        }
    }

    /// 第三类买点见证（port Lean `eventType3`，ThetaInstantiation.lean:376-387）：离开中枢 + 回抽不破 ZG。
    ///
    /// Lean `eventType3.bsp`：`side=long`，`brokeCenter=false`，`leftCenter=true`，`firstRetrace=true`，
    /// `center.zg=20`，`retracePrice=25`（20<25 ⟹ 不破 ZG），`divPair` 力度延续（forceA=3,forceC=3 非背驰）。
    fn sample_type3_buy() -> BuyEndpoint {
        BuyEndpoint {
            side: Side::Long,
            broke_center: false,   // Lean eventType3.bsp.brokeCenter = false（未破中枢，区别第一类）
            is_divergence: false,  // Lean eventType3.bsp.divPair 力度延续（forceA=3,forceC=3 ⟹ 非背驰）
            left_center: true,     // Lean eventType3.bsp.leftCenter = true
            first_retrace: true,   // Lean eventType3.bsp.firstRetrace = true
            retrace_price: 25,     // Lean eventType3.bsp.retracePrice = 25（25 > zg=20 ⟹ 不破 ZG）
            center_zg: 20,         // Lean witnessCenter.zg = 20
        }
    }

    // ── 买点判据（port ThetaInstantiation recogChanlun 内联判据 :152/:155-156）──────────

    /// sampleType1Buy 满足第一类买点判据（Lean `eventType1_isType1` :390）。
    #[test]
    fn type1_buy_judgment() {
        assert!(is_type1_buy(&sample_type1_buy()));
        // 未破中枢 ⟹ 非第一类买点（破中枢是第一类必要前件，Lean :152）。
        let no_break = BuyEndpoint { broke_center: false, ..sample_type1_buy() };
        assert!(!is_type1_buy(&no_break));
        // ★忠实 Lean :152 无 side 检查：side=Short 仍满足第一类（破中枢+背驰）。
        let short_side = BuyEndpoint { side: Side::Short, ..sample_type1_buy() };
        assert!(is_type1_buy(&short_side), "Lean :152 第一类不检查 side（底背驰共用力度序判据）");
    }

    /// sampleType3Buy 满足第三类买点判据（Lean `eventType3_isType3Buy` :394）。
    #[test]
    fn type3_buy_judgment() {
        assert!(is_type3_buy(&sample_type3_buy()));
        // 破 ZG（retrace <= zg）⟹ 非第三类买点（不破 ZG 是第三类位置前件，Lean :156）。
        let break_zg = BuyEndpoint { retrace_price: 15, ..sample_type3_buy() };
        assert!(!is_type3_buy(&break_zg), "破 ZG 重入中枢 ⟹ 非第三类买点");
        // 非 long ⟹ 非第三类买点（Lean :155 检查 side=long）。
        let short_side = BuyEndpoint { side: Side::Short, ..sample_type3_buy() };
        assert!(!is_type3_buy(&short_side), "Lean :155 第三类检查 side=long");
    }

    // ── 买点识别核 recog（port ThetaInstantiation recogChanlun :151）──────────────────

    /// recog 真消费第一类买点判据（Lean `eventType1_recog_openRoot` :398）。
    #[test]
    fn recog_type1_open_root() {
        assert_eq!(recog_chanlun_buy(&sample_type1_buy()), BuyDecision::OpenRoot);
    }

    /// recog 真消费第三类买点判据（Lean `eventType3_recog_accreteCore` :402）。
    #[test]
    fn recog_type3_accrete_core() {
        assert_eq!(recog_chanlun_buy(&sample_type3_buy()), BuyDecision::AccreteCore);
    }

    /// recog 力度延续 → hold（Lean `recog_continuation_hold` :199）：非背驰 ∧ 未离开中枢 ⟹ hold。
    #[test]
    fn recog_continuation_hold() {
        let cont = BuyEndpoint {
            side: Side::Long,
            broke_center: false,
            is_divergence: false, // 力度延续（非背驰）
            left_center: false,   // 未离开中枢
            first_retrace: false,
            retrace_price: 0,
            center_zg: 20,
        };
        assert_eq!(recog_chanlun_buy(&cont), BuyDecision::Hold);
    }

    /// recog 全定义落三态（对应 Lean recogChanlun total）：任意端点输出 BuyDecision 三态之一。
    #[test]
    fn recog_total() {
        for side in [Side::Long, Side::Short] {
            for bc in [false, true] {
                for div in [false, true] {
                    for lc in [false, true] {
                        for fr in [false, true] {
                            for rp in [5i64, 25] {
                                let e = BuyEndpoint {
                                    side, broke_center: bc, is_divergence: div,
                                    left_center: lc, first_retrace: fr,
                                    retrace_price: rp, center_zg: 20,
                                };
                                let d = recog_chanlun_buy(&e);
                                assert!(matches!(
                                    d,
                                    BuyDecision::OpenRoot | BuyDecision::AccreteCore | BuyDecision::Hold
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    /// 第一类优先于第三类（Lean 分支顺序 :151）：同时满足两类前提时识别为第一类（openRoot）。
    #[test]
    fn recog_type1_priority_over_type3() {
        let both = BuyEndpoint {
            side: Side::Long,
            broke_center: true,   // 第一类前提
            is_divergence: true,  // 第一类前提
            left_center: true,    // 第三类前提
            first_retrace: true,  // 第三类前提
            retrace_price: 25,    // > zg ⟹ 第三类前提
            center_zg: 20,
        };
        assert_eq!(recog_chanlun_buy(&both), BuyDecision::OpenRoot, "第一类优先（Lean :151 分支序）");
    }

    // ── 买侧 ledger delta（port ThetaInstantiation decisionLedgerDelta :256）─────────

    /// 买点决策 → 账本 delta（Lean `decisionLedgerDelta` :256-259）。
    #[test]
    fn buy_ledger_delta_bit_exact() {
        assert_eq!(buy_decision_ledger_delta(BuyDecision::OpenRoot), (0, 1, 0));
        assert_eq!(buy_decision_ledger_delta(BuyDecision::AccreteCore), (0, 2, 0));
        assert_eq!(buy_decision_ledger_delta(BuyDecision::Hold), (0, 0, 0));
    }

    // ── 买侧闭环 transition（port ThetaInstantiation chanlunTransition :297）──────────

    /// 买侧闭环保 R=Π-A-W（Lean `chanlunTransition_preserves_ledger_inv` :307）。
    #[test]
    fn buy_transition_preserves_ledger_inv() {
        let x0 = AssemblyState::initial(1_000_000);
        for e in [sample_type1_buy(), sample_type3_buy()] {
            let x1 = buy_transition(&x0, &e);
            assert!(x1.ledger_state.inv_holds(), "买侧闭环破坏 R=Π-A-W: {:?}", x1.ledger_state);
        }
    }

    /// 第一类建根仓真改 A（Lean `chanlunTransition_type1_allocates` :319）：A = base + 1。
    #[test]
    fn buy_transition_type1_allocates() {
        let x0 = AssemblyState {
            ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: 5, w: 0, r: -5 },
            ..AssemblyState::initial(1_000_000)
        };
        let x1 = buy_transition(&x0, &sample_type1_buy());
        assert_eq!(x1.ledger_state.a, x0.ledger_state.a + 1, "第一类建根仓 A+=1");
        // ★买侧 Π 不变（买入无实现损益，Lean openRoot dΠ=0）。
        assert_eq!(x1.ledger_state.pi, x0.ledger_state.pi, "买侧第一类 Π 不变（dΠ=0）");
    }

    /// 第三类增核真改 A（Lean `chanlunTransition_type3_allocates` :331）：A = base + 2。
    #[test]
    fn buy_transition_type3_allocates() {
        let x0 = AssemblyState {
            ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: 5, w: 0, r: -5 },
            ..AssemblyState::initial(1_000_000)
        };
        let x1 = buy_transition(&x0, &sample_type3_buy());
        assert_eq!(x1.ledger_state.a, x0.ledger_state.a + 2, "第三类增核 A+=2");
    }

    /// ★★买侧非退化总见证（Lean `chanlun_transition_distinguishes_classes` :420）：两类买点 ⟹ 不同 A delta。
    #[test]
    fn buy_transition_distinguishes_classes() {
        let x0 = AssemblyState {
            ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: 10, w: 0, r: -10 },
            ..AssemblyState::initial(1_000_000)
        };
        let a1 = buy_transition(&x0, &sample_type1_buy()).ledger_state.a; // base + 1
        let a3 = buy_transition(&x0, &sample_type3_buy()).ledger_state.a; // base + 2
        assert_ne!(a1, a3, "第一类(A+1) vs 第三类(A+2) 产生不同 ledger A delta");
        assert_eq!(a1, 11);
        assert_eq!(a3, 12);
    }

    /// hold 不改账本（力度延续保持）。
    #[test]
    fn buy_transition_hold_noop() {
        let x0 = AssemblyState {
            ledger_state: LedgerComp { i0: 1_000_000, pi: 3, a: 5, w: 1, r: -3 },
            positions: 7,
            ..AssemblyState::initial(1_000_000)
        };
        let cont = BuyEndpoint {
            side: Side::Long, broke_center: false, is_divergence: false,
            left_center: false, first_retrace: false, retrace_price: 0, center_zg: 20,
        };
        let x1 = buy_transition(&x0, &cont);
        assert_eq!(x1.ledger_state, x0.ledger_state, "hold 不改 ledger");
        assert_eq!(x1.positions, x0.positions, "hold 不改 positions");
    }

    // ── 买卖闭环对偶对称（对照卖侧 sell.rs buy_sell_A_delta_mirror）─────────────────

    /// ★★买卖账本 delta·A 分量镜像（Lean `buy_sell_A_delta_mirror`，本文件买侧 vs sell.rs 卖侧）：
    /// 买 openRoot dA=+1 ↔ 卖 closeRoot dA=-1；买 accreteCore dA=+2 ↔ 卖 reduceCore dA=-2。
    ///
    /// ★此前 sell.rs:401-410 的同名测试用**硬编码** `open_root_da: i64 = 1`（买侧无 rust 实装）。
    /// 本测试用**真买侧 rust 函数** [`buy_decision_ledger_delta`] 取 A 分量，与卖侧 rust 函数比——
    /// 消除硬编码常量的 vacuous（买卖两侧均为真 rust 实装的镜像，非 1 vs -(-1) 自指）。
    #[test]
    fn buy_sell_A_delta_mirror_real_impl() {
        use super::super::sell::{sell_decision_ledger_delta, SellDecision};
        // 买侧 A 分量（本文件真 rust 实装）。
        let open_root_da = buy_decision_ledger_delta(BuyDecision::OpenRoot).1; // +1
        let accrete_core_da = buy_decision_ledger_delta(BuyDecision::AccreteCore).1; // +2
        // 卖侧 A 分量（sell.rs 真 rust 实装）。
        let close_root_da = sell_decision_ledger_delta(SellDecision::CloseRoot).1; // -1
        let reduce_core_da = sell_decision_ledger_delta(SellDecision::ReduceCore).1; // -2
        assert_eq!(open_root_da, -close_root_da, "建根仓 A+1 ↔ 清根仓 A-1（真 rust 双侧实装）");
        assert_eq!(accrete_core_da, -reduce_core_da, "增核 A+2 ↔ 减核 A-2（真 rust 双侧实装）");
    }
}
