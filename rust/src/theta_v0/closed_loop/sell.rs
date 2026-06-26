//! 卖侧 Θ 闭环——port **`Origin/SellPointRecog.lean`（#120）+ `Origin/SellClosedLoop.lean`（#121）**。
//!
//! ## 工位定位（G1 卖侧闭环缺口，task #126）
//!
//! 闭环引擎的 [`policy_output`](super::transition::policy_output) 只跑买侧 classify（RiskMode×Phase →
//! action_priority），卖出动作由 [`schedule_adapter`](super::transition::schedule_adapter) 被动从动作
//! 派生——**只进不出半闭环**，无独立卖点识别。本文件 port Origin 已形式化的卖点对偶 + 卖侧闭环：
//! 主动卖点识别（顶背驰=突破中枢后背驰∧short / 回升不破 ZD / §11 力度延续）+ 卖侧 transition
//! （closeRoot→Π↑/A↓、reduceCore→A↓），bit-exact 对齐 Lean 的 A-1/A-2 delta + 保 R=Π-A-W。
//!
//! ## bit-exact 对齐 Lean（语义对应表）
//!
//! | Rust | Lean（Origin） | 语义 |
//! |------|---------------|------|
//! | [`SellDecision`] | `SellPointRecog.SellDecision`（#120） | closeRoot/reduceCore/hold 三态 |
//! | [`SellEndpoint`] | `BspClassification.BspEndpoint`（#113）卖侧投影 | 卖点判据输入端点 |
//! | [`recog_chanlun_sell`] | `SellPointRecog.recogChanlunSell`（#120） | 卖点识别核（真 case-split） |
//! | [`is_type1_sell`] | `SellPointRecog.IsType1Sell`（#120） | 顶背驰=破中枢∧背驰∧short |
//! | [`is_type3_sell`] | `BspClassification.IsType3Sell`（#113） | 回升不破 ZD（retrace < zd）|
//! | [`sell_decision_ledger_delta`] | `SellClosedLoop.sellDecisionLedgerDelta`（#121） | (Π+1,A-1)/(A-2)/noop |
//! | [`sell_transition`] | `SellClosedLoop.sellTransition`（#121） | 卖点决策驱动 ledger 一步 |
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! 本文件 = **L0/L1**（结构镜像：卖点判据 + 卖侧闭环与 Lean SellPointRecog/SellClosedLoop 定义
//! 对齐 = 验证管线正确性，零信息增量）。`cargo test` 通过 = 卖侧识别核真区分两类卖点 + 卖侧
//! 转移真改 ledger 且保 R=Π-A-W + 买卖镜像对偶，**不**是缠论盈利/实盘有效声明（L2/L3）。
//!
//! ## 诚实边界（no声明膨胀，与 Lean SellClosedLoop §诚实边界一致）
//!
//! - ✗ 卖侧应对策略盈利/最优/实盘有效（L3）。
//! - ✗ 仓位减量/利润数额（1/2/1）来自缠论（Θ_risk 参数——卖点分类只决定 delta 的类型/符号）。
//! - ✗ TW 端（取本金/提现 W）对接——本文件用 R=Π-A-W 单账本卖侧（W 在卖侧 delta 中恒 0）。
//!   卖侧顶背驰「实现利润」记 Π↑（账面），提现（W↑）是另一决策层（TW 端，still-MISSING）。
//! - ✗ 第二类卖点闭环（committed `IsType2Sell` 本级别可观测，定律一「由次级别一类构成」需次级别
//!   递归，见 descend.rs）——本文件卖侧闭环覆盖第一类/第三类/延续三态，与 Lean #121 对偶一致。

use super::super::strategy::ledger::{ledger_step, LedgerEvent};
use super::super::types::{Side, Tick};
use super::state::AssemblyState;

/// 卖点缠论决策 `SellDecision`（port `SellPointRecog.SellDecision`，#120，对偶买侧三态）。
///
/// - `CloseRoot`：清根仓 / 反向建空（第一类顶背驰卖点——突破中枢后背驰∧short，§10.1）。
/// - `ReduceCore`：减核（第三类卖点——向下离开中枢回抽不破 ZD，§10.1）。
/// - `Hold`：保持（力度延续/非卖点，§11——后段力度 ≥ 前段，趋势延续无背驰）。
///
/// 三态由真缠论卖点判据区分（见 [`recog_chanlun_sell`]），与买侧（openRoot/accreteCore/hold）对偶。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SellDecision {
    /// 第一类顶背驰卖点 → 清根仓（Lean `SellDecision.closeRoot`）。
    CloseRoot,
    /// 第三类回升卖点 → 减核（Lean `SellDecision.reduceCore`）。
    ReduceCore,
    /// 力度延续/非卖点 → 保持（Lean `SellDecision.hold`）。
    Hold,
}

/// 卖点端点 `SellEndpoint`（port `BspClassification.BspEndpoint` 的卖侧消费字段，#113）。
///
/// 卖点判据所需的端点语义字段（对齐 Lean `BspEndpoint`，只保留 [`recog_chanlun_sell`] 消费的字段）：
/// - `side`：买卖向（卖点判据要求 `Side::Short`）。
/// - `broke_center`：是否突破中枢（第一类顶背驰前提，Lean `e.brokeCenter`）。
/// - `is_divergence`：背驰判据（顶背驰=力度背驰 forceC < forceA，Lean `IsDivergence e.divPair`）。
///   ★rust 在此领先 Origin：背驰由已实装 MACD（divergence.rs `segments_diverge`）真算，Lean
///   `divPair` 是抽象力度对。本字段承载 MACD 计算后的背驰 bool（rust 真算，非外部参数）。
/// - `left_center`：是否向下离开中枢（第三类前提，Lean `e.leftCenter`）。
/// - `first_retrace`：是否第一次回抽（第三类前提，Lean `e.firstRetrace`）。
/// - `retrace_price`：回抽高点价（第三类判据 `retrace_price < center_zd`，Lean `e.retracePrice`）。
/// - `center_zd`：所在中枢核心下沿 ZD（第三类参照点，Lean `e.center.zd`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SellEndpoint {
    pub side: Side,
    pub broke_center: bool,
    pub is_divergence: bool,
    pub left_center: bool,
    pub first_retrace: bool,
    pub retrace_price: Tick,
    pub center_zd: Tick,
}

/// 第一类卖点判据 `is_type1_sell`（port `SellPointRecog.IsType1Sell`，#120）。
///
/// 顶背驰 = 上涨趋势向上**突破**中枢后的背驰点：side=short ∧ 破中枢 ∧ 背驰。
/// 与底背驰（买点）**共用同一力度序判据**（背驰方向无关，Lean `type1_buy_sell_share_divergence`）。
pub fn is_type1_sell(e: &SellEndpoint) -> bool {
    e.side == Side::Short && e.broke_center && e.is_divergence
}

/// 第三类卖点判据 `is_type3_sell`（port `BspClassification.IsType3Sell`，#113）。
///
/// 向下离开中枢后回抽高点**不升破 ZD**：side=short ∧ 离开中枢 ∧ 第一次回抽 ∧ `retrace_price < zd`。
/// 与第三类买点（不破 ZG，之上）位置三态镜像（Lean `type3_buy_sell_position_mirror`）。
pub fn is_type3_sell(e: &SellEndpoint) -> bool {
    e.side == Side::Short && e.left_center && e.first_retrace && e.retrace_price < e.center_zd
}

/// 卖点识别核 `recog_chanlun_sell`（port `SellPointRecog.recogChanlunSell`，#120，本文件核心）。
///
/// 从卖点端点真 case-split on 卖点判据，识别卖侧应对意图（缠论卖点规则真编码，对偶买侧 recog）：
/// - **§10.1 第一类卖点**：破中枢 ∧ 背驰 ∧ short ⟹ `CloseRoot`（顶背驰清仓）。
/// - **§10.1 第三类卖点**：否则 short ∧ 离开中枢 ∧ 第一次回抽 ∧ retrace < ZD ⟹ `ReduceCore`（减核）。
/// - **§11**：否则（含力度延续 = 非背驰，无卖点）⟹ `Hold`（保持）。
///
/// ★分支优先级与 Lean 严格一致：第一类（破中枢背驰）优先于第三类（离开中枢回抽）——第三类语境
/// 是「向下离开中枢后回升」，与第一类「突破中枢背驰」互斥分支（Lean `recogSell_type3_reduceCore`
/// 前提 `¬brokeCenter`）。
pub fn recog_chanlun_sell(e: &SellEndpoint) -> SellDecision {
    if e.side == Side::Short && e.broke_center && e.is_divergence {
        // §10.1 第一类卖点：突破中枢 + 顶背驰 ⟹ 清根仓。
        SellDecision::CloseRoot
    } else if e.side == Side::Short && e.left_center && e.first_retrace && e.retrace_price < e.center_zd {
        // §10.1 第三类卖点：向下离开中枢 + 第一次回抽 + 不破 ZD ⟹ 减核。
        SellDecision::ReduceCore
    } else {
        // §11：力度延续 / 非卖点 ⟹ 保持。
        SellDecision::Hold
    }
}

/// 卖点账本 delta `sell_decision_ledger_delta`（port `SellClosedLoop.sellDecisionLedgerDelta`，#121）。
///
/// 把真缠论卖点决策映为账本三元组 delta `(dΠ, dA, dW)`（喂给 [`ledger_step`]），与买侧
/// `decisionLedgerDelta`（openRoot (0,+1,0) / accreteCore (0,+2,0) / hold 0）镜像对偶：
/// - `CloseRoot`（第一类顶背驰清根仓）⟹ `(1, -1, 0)`：Π += 1（实现利润）∧ A -= 1（清根仓位）。
///   与买侧 openRoot (0,+1,0) 在 A 分量镜像（建根仓 A+1 ↔ 清根仓 A-1），额外携 Π↑（卖出实现损益
///   的时间不对称——买入无 Π 变化，Lean §诚实标注已审查，非定义冲突）。
/// - `ReduceCore`（第三类回升减核）⟹ `(0, -2, 0)`：A -= 2（减核，与第一类清仓 -1 不同量，非退化）。
///   与买侧 accreteCore (0,+2,0) 在 A 分量严格镜像（增核 A+2 ↔ 减核 A-2）。
/// - `Hold`（力度延续保持）⟹ `(0, 0, 0)`：账本不变（无卖点不动账，与买侧 hold 共用）。
///
/// ★诚实（与 Lean 一致）：数额 1/2 是 Θ_risk 占位常量（具体数额是运行时数据 L2）——缠论卖点分类
/// 决定 delta 的类型/符号（清仓 realize+deallocate / 减核 deallocate），不决定精确数额。W 恒 0（TW 端
/// still-MISSING，不硬塞进单账本卖侧）。
pub fn sell_decision_ledger_delta(d: SellDecision) -> (i64, i64, i64) {
    match d {
        SellDecision::CloseRoot => (1, -1, 0),   // 清根仓：realize Π+1 ∧ deallocate A-1
        SellDecision::ReduceCore => (0, -2, 0),  // 减核：deallocate A-2（≠ 清根仓，非退化）
        SellDecision::Hold => (0, 0, 0),         // 保持：noop
    }
}

/// 把 `(dΠ, dA, dW)` delta 应用到 ledger（复用买侧 [`ledger_step`]，保 R=Π-A-W）。
///
/// 与 Lean `sellTransition` 内 `ledgerStep z.ledger delta.1 delta.2.1 delta.2.2` 对齐：dΠ→Realize、
/// dA→Allocate（负 dA 即 deallocate）、dW→Withdraw。每个分量经 [`ledger_step`]（每分支重算 R）
/// 保恒等。三分量串联应用 ⟹ 终态保 R=Π-A-W（Lean `sellTransition_preserves_ledger_inv`）。
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

/// ★★卖侧真缠论闭环转移 `sell_transition`（port `SellClosedLoop.sellTransition`，#121，本文件核心）。
///
/// 闭环数据流（真缠论卖点，镜像买侧 [`hybrid_step`](super::transition::hybrid_step)）：
/// 1. [`recog_chanlun_sell`] 用卖点判据识别真缠论卖点决策 d（第一类清仓/第三类减核/延续保持）。
/// 2. [`sell_decision_ledger_delta`] 把卖点决策映为账本 delta。
/// 3. [`ledger_step`]（保 R=Π-A-W）用该 delta 更新账本，写回完整 [`AssemblyState`]。
///
/// ★主动卖点识别出场（非被动 bar 驱动）：ledger 下一态由**端点的真缠论卖点分类**决定，
/// 且保账本恒等。positions 同步反映卖出（清仓→0 占位，减核→饱和减 1）。
///
/// ★诚实标注（与 Lean §诚实边界一致）：只更新 ledger + positions（卖侧账本足迹），micro/tw/risk/phase
/// 保持（卖侧决策只动 R=Π-A-W 账本，TW 端 still-MISSING）。
pub fn sell_transition(x: &AssemblyState, e: &SellEndpoint) -> AssemblyState {
    let d = recog_chanlun_sell(e);
    let (d_pi, d_a, d_w) = sell_decision_ledger_delta(d);
    let new_ledger = apply_ledger_delta(&x.ledger_state, d_pi, d_a, d_w);
    // positions 反映卖出（清仓→0；减核→饱和减 2 单位的占位，与 A 分量同步；保持→不变）。
    let new_positions = match d {
        SellDecision::CloseRoot => 0,
        SellDecision::ReduceCore => x.positions.saturating_sub(2),
        SellDecision::Hold => x.positions,
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

    /// 第一类卖点见证（port Lean `sampleType1Sell`）：突破中枢 + 顶背驰 + short。
    fn sample_type1_sell() -> SellEndpoint {
        SellEndpoint {
            side: Side::Short,
            broke_center: true,    // 突破中枢（第一类前提）
            is_divergence: true,   // 顶背驰（MACD forceC < forceA）
            left_center: false,
            first_retrace: false,
            retrace_price: 30,     // 无关第三类
            center_zd: 10,
        }
    }

    /// 第三类卖点见证（port Lean `sampleType3Sell`）：离开中枢 + 第一次回升 + 不破 ZD + short。
    fn sample_type3_sell() -> SellEndpoint {
        SellEndpoint {
            side: Side::Short,
            broke_center: false,   // 未破中枢（区别第一类）
            is_divergence: false,  // 力度延续（非背驰）
            left_center: true,     // 离开中枢（第三类前提）
            first_retrace: true,
            retrace_price: 5,      // 5 < zd=10 ⟹ 不破 ZD ⟹ 第三类卖点
            center_zd: 10,
        }
    }

    // ── 卖点判据（port SellPointRecog §1-§3）─────────────────────────────────

    /// sampleType1Sell 满足第一类卖点判据（Lean `sampleType1Sell_isType1Sell`）。
    #[test]
    fn type1_sell_judgment() {
        assert!(is_type1_sell(&sample_type1_sell()));
        // 非 short ⟹ 非第一类卖点（side 唯一）。
        let buy_side = SellEndpoint { side: Side::Long, ..sample_type1_sell() };
        assert!(!is_type1_sell(&buy_side));
    }

    /// sampleType3Sell 满足第三类卖点判据（Lean `sampleType3Sell_isType3Sell`）。
    #[test]
    fn type3_sell_judgment() {
        assert!(is_type3_sell(&sample_type3_sell()));
        // 升破 ZD（retrace >= zd）⟹ 非第三类卖点（Lean `type3Sell_rejects_reenter`）。
        let reenter = SellEndpoint { retrace_price: 15, ..sample_type3_sell() };
        assert!(!is_type3_sell(&reenter), "升破 ZD 重入中枢 ⟹ 非第三类卖点");
    }

    // ── 卖点识别核 recog（port SellPointRecog §4-§5）─────────────────────────

    /// recog 真消费第一类卖点判据（Lean `recogSell_type1_closeRoot`）。
    #[test]
    fn recog_type1_close_root() {
        assert_eq!(recog_chanlun_sell(&sample_type1_sell()), SellDecision::CloseRoot);
    }

    /// recog 真消费第三类卖点判据（Lean `recogSell_type3_reduceCore`）。
    #[test]
    fn recog_type3_reduce_core() {
        assert_eq!(recog_chanlun_sell(&sample_type3_sell()), SellDecision::ReduceCore);
    }

    /// recog 力度延续 → hold（Lean `recogSell_continuation_hold`）：非背驰 ∧ 未离开中枢 ⟹ hold。
    #[test]
    fn recog_continuation_hold() {
        let cont = SellEndpoint {
            side: Side::Short,
            broke_center: false,
            is_divergence: false, // 力度延续（非背驰）
            left_center: false,   // 未离开中枢
            first_retrace: false,
            retrace_price: 0,
            center_zd: 10,
        };
        assert_eq!(recog_chanlun_sell(&cont), SellDecision::Hold);
    }

    /// recog 全定义落三态（Lean `recogSell_total`）：任意端点输出 SellDecision 三态之一。
    #[test]
    fn recog_total() {
        for side in [Side::Long, Side::Short] {
            for bc in [false, true] {
                for div in [false, true] {
                    for lc in [false, true] {
                        for fr in [false, true] {
                            for rp in [5i64, 15] {
                                let e = SellEndpoint {
                                    side, broke_center: bc, is_divergence: div,
                                    left_center: lc, first_retrace: fr,
                                    retrace_price: rp, center_zd: 10,
                                };
                                let d = recog_chanlun_sell(&e);
                                assert!(matches!(
                                    d,
                                    SellDecision::CloseRoot | SellDecision::ReduceCore | SellDecision::Hold
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    /// 第一类优先于第三类（Lean 分支顺序）：同时满足两类前提时识别为第一类（closeRoot）。
    #[test]
    fn recog_type1_priority_over_type3() {
        let both = SellEndpoint {
            side: Side::Short,
            broke_center: true,   // 第一类前提
            is_divergence: true,  // 第一类前提
            left_center: true,    // 第三类前提
            first_retrace: true,  // 第三类前提
            retrace_price: 5,     // < zd ⟹ 第三类前提
            center_zd: 10,
        };
        assert_eq!(recog_chanlun_sell(&both), SellDecision::CloseRoot, "第一类优先");
    }

    // ── 卖侧 ledger delta（port SellClosedLoop §1）──────────────────────────

    /// 卖点决策 → 账本 delta（Lean `sellDecisionLedgerDelta`）。
    #[test]
    fn sell_ledger_delta_bit_exact() {
        assert_eq!(sell_decision_ledger_delta(SellDecision::CloseRoot), (1, -1, 0));
        assert_eq!(sell_decision_ledger_delta(SellDecision::ReduceCore), (0, -2, 0));
        assert_eq!(sell_decision_ledger_delta(SellDecision::Hold), (0, 0, 0));
    }

    // ── 卖侧闭环 transition（port SellClosedLoop §2-§3）──────────────────────

    /// 卖侧闭环保 R=Π-A-W（Lean `sellTransition_preserves_ledger_inv`）。
    #[test]
    fn sell_transition_preserves_ledger_inv() {
        let x0 = AssemblyState::initial(1_000_000);
        for e in [sample_type1_sell(), sample_type3_sell()] {
            let x1 = sell_transition(&x0, &e);
            assert!(x1.ledger_state.inv_holds(), "卖侧闭环破坏 R=Π-A-W: {:?}", x1.ledger_state);
        }
    }

    /// 第一类清仓真改 A（Lean `sellTransition_type1_deallocates`）：A = base - 1。
    #[test]
    fn sell_transition_type1_deallocates() {
        let x0 = AssemblyState {
            ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: 5, w: 0, r: -5 },
            ..AssemblyState::initial(1_000_000)
        };
        let x1 = sell_transition(&x0, &sample_type1_sell());
        assert_eq!(x1.ledger_state.a, x0.ledger_state.a - 1, "第一类清根仓 A-=1");
    }

    /// 第一类清仓真实现利润 Π（Lean `sellTransition_type1_realizes`）：Π = base + 1。
    #[test]
    fn sell_transition_type1_realizes() {
        let x0 = AssemblyState::initial(1_000_000);
        let x1 = sell_transition(&x0, &sample_type1_sell());
        assert_eq!(x1.ledger_state.pi, x0.ledger_state.pi + 1, "第一类实现利润 Π+=1");
    }

    /// 第三类减核真改 A（Lean `sellTransition_type3_deallocates`）：A = base - 2。
    #[test]
    fn sell_transition_type3_deallocates() {
        let x0 = AssemblyState {
            ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: 5, w: 0, r: -5 },
            ..AssemblyState::initial(1_000_000)
        };
        let x1 = sell_transition(&x0, &sample_type3_sell());
        assert_eq!(x1.ledger_state.a, x0.ledger_state.a - 2, "第三类减核 A-=2");
    }

    /// ★★卖侧非退化总见证（Lean `sell_transition_distinguishes_classes`）：两类卖点 ⟹ 不同 A delta。
    #[test]
    fn sell_transition_distinguishes_classes() {
        let x0 = AssemblyState {
            ledger_state: LedgerComp { i0: 1_000_000, pi: 0, a: 10, w: 0, r: -10 },
            ..AssemblyState::initial(1_000_000)
        };
        let a1 = sell_transition(&x0, &sample_type1_sell()).ledger_state.a; // base - 1
        let a3 = sell_transition(&x0, &sample_type3_sell()).ledger_state.a; // base - 2
        assert_ne!(a1, a3, "第一类(A-1) vs 第三类(A-2) 产生不同 ledger A delta");
        assert_eq!(a1, 9);
        assert_eq!(a3, 8);
    }

    /// hold 不改账本（力度延续保持）。
    #[test]
    fn sell_transition_hold_noop() {
        let x0 = AssemblyState {
            ledger_state: LedgerComp { i0: 1_000_000, pi: 3, a: 5, w: 1, r: -3 },
            positions: 7,
            ..AssemblyState::initial(1_000_000)
        };
        let cont = SellEndpoint {
            side: Side::Short, broke_center: false, is_divergence: false,
            left_center: false, first_retrace: false, retrace_price: 0, center_zd: 10,
        };
        let x1 = sell_transition(&x0, &cont);
        assert_eq!(x1.ledger_state, x0.ledger_state, "hold 不改 ledger");
        assert_eq!(x1.positions, x0.positions, "hold 不改 positions");
    }

    // ── 买卖闭环对偶对称（port SellClosedLoop §4）───────────────────────────

    /// ★★买卖账本 delta·A 分量镜像（Lean `buy_sell_A_delta_mirror`）：
    /// 买 openRoot dA=+1 ↔ 卖 closeRoot dA=-1；买 accreteCore dA=+2 ↔ 卖 reduceCore dA=-2。
    ///
    /// 买侧 delta 引 closed_loop/transition.rs schedule_adapter 的语义（Allocate(1)=建仓 A+1）+
    /// strategy 增核（A+2）。本测试坐实卖侧 delta 与买侧严格相反号（A 分量镜像）。
    #[test]
    fn buy_sell_A_delta_mirror() {
        // 卖侧 A 分量（本文件）。
        let close_root_da = sell_decision_ledger_delta(SellDecision::CloseRoot).1; // -1
        let reduce_core_da = sell_decision_ledger_delta(SellDecision::ReduceCore).1; // -2
        // 买侧 A 分量（Lean decisionLedgerDelta：openRoot dA=+1, accreteCore dA=+2）。
        let open_root_da: i64 = 1;
        let accrete_core_da: i64 = 2;
        assert_eq!(open_root_da, -close_root_da, "建根仓 A+1 ↔ 清根仓 A-1");
        assert_eq!(accrete_core_da, -reduce_core_da, "增核 A+2 ↔ 减核 A-2");
    }

    /// ★★买卖闭环 A 分量镜像（Lean `buy_sell_closed_loop_A_mirror`）：
    /// 从同一初始态，买侧第一类建仓后 A+1，卖侧第一类清仓后 A-1，相对 base 偏移严格相反号。
    #[test]
    fn buy_sell_closed_loop_A_mirror() {
        use super::super::transition::{hybrid_step, AssemblyEvent};
        use super::super::state::MicroEvent;
        // 买侧：初始 Normal/PhaseI ⟹ Buy ⟹ Allocate(1) ⟹ A+1。
        let x0 = AssemblyState::initial(1_000_000);
        let buy_e = AssemblyEvent { parse_event: MicroEvent::NewBar(true) };
        let buy_a = hybrid_step(&x0, &buy_e).ledger_state.a; // base + 1
        // 卖侧：第一类清仓 ⟹ A-1。
        let sell_a = sell_transition(&x0, &sample_type1_sell()).ledger_state.a; // base - 1
        assert_eq!(buy_a - x0.ledger_state.a, -(sell_a - x0.ledger_state.a),
            "买侧建仓 A+1 ↔ 卖侧清仓 A-1 镜像");
    }
}
