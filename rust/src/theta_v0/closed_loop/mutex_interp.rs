//! 全互斥解释器 I_Θ——契约锚 **`Origin.FullDefinitionStrategy.chooseAction`**
//! （alpha2.pdf §5 全互斥解释器 + §9 定理1「π^bsp_Θ 全定义」的可执行验证）。
//!
//! ## 命题（alpha2.pdf §5-§6 + §9 定理1）
//!
//! 同一时刻可能出现多个买卖点（`B_{2,ℓ}=1 ∧ B_{3,ℓ}=1`，多级别同时出现）——买卖点信号层
//! **非互斥**（`Origin.BspClassification.no_exclusive_trichotomy`，见 types.rs）。故必须有互斥
//! 解释器把重叠的原始谓词压缩为**唯一动作**：
//!
//! - **原始谓词** P_1..P_m（alpha2 §5，可重叠）：
//!   P1=风险强平 / P2=平多 / P3=平空 / P4=开短差 / P5=开多 / P6=开空 / P7=加仓 / P8=保持。
//! - **优先级互斥化**：`C_1 = P_1`，`C_j = P_j ∧ ⋀_{k<j} ¬P_k`（j=2..m）。
//! - **兜底**：`C_0 = ⋀_{j=1}^m ¬P_j`。
//! - **全互斥定理**：`Σ_{j=0}^m 1[C_j] = 1`（恰好一个 C_j 成立）。
//!   证明（alpha2 §5）：若无 P_j 成立 ⟹ C_0 唯一成立；若有，取最小成立索引 r ⟹ C_r 唯一成立。
//!
//! 这是 alpha2 §9 定理1 假设4「I_Θ 使用固定优先级」⟹ 解释器输出 `(D_t,O_t,L_t)` 唯一的核心。
//!
//! ## 与 Lean 原型的对应（忠实镜像，非新发明）
//!
//! Lean `Origin.FullDefinitionStrategy.chooseAction (p1..p9 : Bool) : ActionClass`
//! （FullDefinitionStrategy.lean:47-59）是本互斥化的**忠实原型**：`if p1 then … else if p2 …
//! else hold` 的 if-else 链精确实现 `C_j = P_j ∧ ⋀_{k<j} ¬P_k`（落入第 j 臂 ⟺ p_j 真且 p_1..p_{j-1}
//! 全假），`else hold` 兜底 = C_0。Lean `action_priority_complete_unique`（:98-100）已证
//! `ExistsUnique (fun a => ActionSpec p1..p9 a)`——这是 `Σ1[C_j]=1` 的 **L0** 结构证明。
//!
//! Lean 用 9 谓词（p1..p9）+ hold 兜底（10 个 ActionClass），alpha2 §5 举例用 8 谓词 P1..P8。
//! 本模块**忠实镜像 Lean 的 9 谓词链**（canonical base，#97 立 Origin 为唯一权威），不取 §5 的
//! 8 谓词举例（§5「P1=… 例如」明示是举例，谓词数 m 不固定）。9 谓词 ↔ §5 语义映射见 [`ActionClass`]。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - 全互斥性 `Σ1[C_j]=1` = **L0**（纯结构定理，不依赖数据，从优先级 if-else 链的定义推导；
//!   Lean `action_priority_complete_unique` 已证）。
//! - 本模块的 property test（遍历全 2^9=512 谓词组合断言恰好一个 C_j 成立）= **L1**（验证 Rust
//!   `choose_action` 实装与 L0 定理一致 = 验证管线正确性，零信息增量——`Σ1[C_j]=1` 由定义恒成立，
//!   测试只确认 Rust if-else 链没写错，**不**验证缠论假设/盈利）。
//!
//! ## 与 intent.rs `action_priority` 的有效域关系（诚实声明，非重复）
//!
//! intent.rs 的 [`action_priority`](super::super::strategy::intent::action_priority) 是本 9 级优先级
//! 链在 `(RiskMode × Phase)` **摘要标签**上的压缩投影（match 穷尽性由 Rust 编译器静态保证互斥）——
//! 它的有效域是「给定结构摘要 (risk,phase) 的确定动作」。本模块的 [`choose_action`] 是在**原始谓词
//! 层**（p1..p9）上的全互斥解释器——它的有效域是「给定 9 个原始谓词真值的唯一动作」，直接对齐
//! alpha2 §5 的 `C_j` 互斥化与 Lean `chooseAction` 的谓词签名。两者是同一优先级链的两个粒度：摘要
//! match（intent.rs）vs 谓词链（本模块）。本模块补的缺口 = §5 `Σ1[C_j]=1` 在**谓词层**的可执行验证
//! （intent.rs 的 match 互斥性由编译器隐式保证，无显式「恰好一个 C_j」的遍历断言）。

/// 互斥动作类 `ActionClass`（契约锚 `Origin.FullDefinitionStrategy.ActionClass`，alpha2 §5 的 C_j 落点）。
///
/// 10 个互斥类对应 9 谓词优先级链的 9 个 `C_j`（j=1..9）+ 兜底 `C_0`（hold）。每个变体携带它对应的
/// 最小成立索引 j（`C_j` 中的 j）——即 alpha2 §5「取最小成立索引 r ⟹ C_r 唯一成立」的 r。
///
/// 9 谓词 ↔ alpha2 §5 的 8 谓词 P1..P8 语义映射（Lean canonical 比 §5 举例更细，多出 executionRepair
/// 与 ancestorInvalid 两类，开仓侧拆 root/reverseChild）：
///
/// | Lean idx | ActionClass            | alpha2 §5 谓词       | StrictAction |
/// |----------|------------------------|---------------------|--------------|
/// | C_1      | InsolventOrLiquidation | P1 风险强平          | Close        |
/// | C_2      | Deleverage             | （P1 细化：去杠杆）   | Reduce       |
/// | C_3      | ExecutionRepair        | （Lean 多出：执行修复）| Close        |
/// | C_4      | PhaseTwoReturnCapital  | P2 平多（阶段二取本）  | Reduce       |
/// | C_5      | RootOrAncestorInvalid  | P3 平空（根/祖先失效） | Close        |
/// | C_6      | CloseReverseChild      | （P3 细化：关反向子）  | Close        |
/// | C_7      | OpenRoot               | P5 开多（建根）       | Buy          |
/// | C_8      | OpenReverseChild       | P4/P6 开短差/开空      | Buy          |
/// | C_9      | PhaseThreeAccreteCore  | P7 加仓（阶段三增核）  | Add          |
/// | C_0      | Hold                   | P8 保持（兜底）       | Hold         |
///
/// ★诚实标注：上表的 alpha2 §5 列是**语义对齐**（§5 谓词是举例，"例如 P1=风险强平"），Lean canonical
/// 的 9 谓词是更细的操盘分层。映射依据 = 动作侧（平/开/加/保持）与优先级序一致；不强行 1:1。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionClass {
    InsolventOrLiquidation, // C_1
    Deleverage,             // C_2
    ExecutionRepair,        // C_3
    PhaseTwoReturnCapital,  // C_4
    RootOrAncestorInvalid,  // C_5
    CloseReverseChild,      // C_6
    OpenRoot,               // C_7
    OpenReverseChild,       // C_8
    PhaseThreeAccreteCore,  // C_9
    Hold,                   // C_0（兜底）
}

impl ActionClass {
    /// 最小成立索引 j（`C_j` 的 j；兜底 C_0 ⟹ 0）——alpha2 §5「取最小成立索引 r」的 r。
    pub fn index(self) -> u8 {
        match self {
            ActionClass::InsolventOrLiquidation => 1,
            ActionClass::Deleverage => 2,
            ActionClass::ExecutionRepair => 3,
            ActionClass::PhaseTwoReturnCapital => 4,
            ActionClass::RootOrAncestorInvalid => 5,
            ActionClass::CloseReverseChild => 6,
            ActionClass::OpenRoot => 7,
            ActionClass::OpenReverseChild => 8,
            ActionClass::PhaseThreeAccreteCore => 9,
            ActionClass::Hold => 0,
        }
    }

    // ponytail: ActionClass→StrictAction 映射删除（零消费者，intent.rs 走自己的摘要 match）。
    // 真有谓词层入口接 choose_action 时再加，一行 match。映射依据见类型表 StrictAction 列。
}

/// 全互斥解释器 `choose_action`（契约锚 `Origin.FullDefinitionStrategy.chooseAction`，alpha2 §5）。
///
/// 优先级互斥化 if-else 链：落入第 j 臂 ⟺ `p_j ∧ ⋀_{k<j} ¬p_k`（= `C_j`），全假 ⟹ `Hold`（= `C_0`）。
/// 这忠实镜像 Lean `chooseAction (p1..p9 : Bool) : ActionClass`（FullDefinitionStrategy.lean:47-59）。
///
/// **全定义**：9 个 bool 输入的全部 2^9=512 组合都返回唯一 `ActionClass`（Rust if-else 链穷尽）。
/// **全互斥**：`Σ_{j=0}^9 1[C_j] = 1`（恰好一个），由 [`tests::sum_indicator_is_one_over_all_2_pow_9`]
/// 遍历全组合断言（alpha2 §5 定理的 L1 可执行验证；L0 由 Lean `action_priority_complete_unique` 证）。
pub fn choose_action(
    p1: bool, p2: bool, p3: bool, p4: bool, p5: bool, p6: bool, p7: bool, p8: bool, p9: bool,
) -> ActionClass {
    if p1 {
        ActionClass::InsolventOrLiquidation
    } else if p2 {
        ActionClass::Deleverage
    } else if p3 {
        ActionClass::ExecutionRepair
    } else if p4 {
        ActionClass::PhaseTwoReturnCapital
    } else if p5 {
        ActionClass::RootOrAncestorInvalid
    } else if p6 {
        ActionClass::CloseReverseChild
    } else if p7 {
        ActionClass::OpenRoot
    } else if p8 {
        ActionClass::OpenReverseChild
    } else if p9 {
        ActionClass::PhaseThreeAccreteCore
    } else {
        ActionClass::Hold
    }
}

/// 互斥化谓词 `c_holds(j, p1..p9)`：`C_j` 是否成立（alpha2 §5 `C_j = P_j ∧ ⋀_{k<j} ¬P_k`，C_0 兜底）。
///
/// 与 [`choose_action`] 解耦的**独立**互斥化判定——用于 property test 直接计数 `Σ_{j=0}^9 1[C_j]`，
/// 避免「用 choose_action 验证 choose_action」的同义反复（测试断言 c_holds 计数 == 1 且与
/// choose_action 输出索引一致 ⟹ 两个独立实现交叉验证 §5 定理）。
///
/// `j=0` ⟹ 兜底 `C_0 = ⋀_{k=1}^9 ¬p_k`；`j∈1..9` ⟹ `C_j = p_j ∧ ⋀_{k<j} ¬p_k`。
pub fn c_holds(j: u8, ps: [bool; 9]) -> bool {
    if j == 0 {
        // C_0 兜底：所有谓词为假。
        ps.iter().all(|&p| !p)
    } else {
        let j = j as usize;
        // C_j = p_j ∧ ⋀_{k<j} ¬p_k（1-indexed → ps[j-1] 为 p_j，ps[0..j-1] 为 p_1..p_{j-1}）。
        ps[j - 1] && ps[..j - 1].iter().all(|&p| !p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 全 2^9=512 谓词组合的迭代器（每个组合是 9 个 bool）。
    fn all_predicate_combos() -> impl Iterator<Item = [bool; 9]> {
        (0u16..512).map(|bits| {
            let mut ps = [false; 9];
            for (k, p) in ps.iter_mut().enumerate() {
                *p = (bits >> k) & 1 == 1;
            }
            ps
        })
    }

    /// ★★全互斥 property test：`Σ_{j=0}^9 1[C_j] = 1`（alpha2 §5 定理 + §9 定理1 假设4）。
    ///
    /// 遍历全部 2^9=512 谓词组合，对每个组合用**独立**的 [`c_holds`] 计数恰好一个 `C_j` 成立。
    /// 这是 Lean `action_priority_complete_unique`（ExistsUnique）的 Rust **L1** 可执行验证——
    /// L0 由 Lean 证，L1 确认 Rust c_holds 互斥化实装与定理一致（零信息增量：定义恒真，测管线）。
    #[test]
    fn sum_indicator_is_one_over_all_2_pow_9() {
        let mut combos_checked = 0;
        for ps in all_predicate_combos() {
            // Σ_{j=0}^9 1[C_j]。
            let count = (0u8..=9).filter(|&j| c_holds(j, ps)).count();
            assert_eq!(
                count, 1,
                "全互斥性 Σ1[C_j]=1 被破坏：谓词 {:?} 下有 {} 个 C_j 成立（应恰好 1）",
                ps, count
            );
            combos_checked += 1;
        }
        assert_eq!(combos_checked, 512, "必须遍历全部 2^9=512 组合");
    }

    /// ★choose_action 输出索引 = 唯一成立的 C_j 索引（两个独立实现交叉验证 §5「取最小成立索引 r」）。
    ///
    /// choose_action（if-else 链）与 c_holds（互斥化判定）是两个独立实现——若 choose_action 选出的
    /// ActionClass.index() 总等于 c_holds 计数为真的那个 j，则证 if-else 链确实实现了「取最小成立索引」。
    #[test]
    fn choose_action_index_matches_unique_c_holds() {
        for ps in all_predicate_combos() {
            let [p1, p2, p3, p4, p5, p6, p7, p8, p9] = ps;
            let chosen = choose_action(p1, p2, p3, p4, p5, p6, p7, p8, p9);
            // 唯一成立的 C_j 的 j（由独立 c_holds 找出）。
            let unique_j = (0u8..=9)
                .find(|&j| c_holds(j, ps))
                .expect("必有一个 C_j 成立（全互斥定理）");
            assert_eq!(
                chosen.index(),
                unique_j,
                "谓词 {:?}：choose_action 选 C_{}，但 c_holds 唯一成立的是 C_{}",
                ps, chosen.index(), unique_j
            );
        }
    }

    /// ★全定义：全部 512 组合都返回确定 ActionClass（无 panic、无未定义）——§9 定理1「∀x_t ∃!O」的
    /// 存在性侧（唯一性由上两测试的互斥性保证）。
    #[test]
    fn choose_action_total_over_all_2_pow_9() {
        let mut count = 0;
        for ps in all_predicate_combos() {
            let [p1, p2, p3, p4, p5, p6, p7, p8, p9] = ps;
            let _ = choose_action(p1, p2, p3, p4, p5, p6, p7, p8, p9); // 不 panic ⟹ 全定义
            count += 1;
        }
        assert_eq!(count, 512);
    }

    /// 兜底 C_0：全谓词假 ⟹ Hold（alpha2 §5 `C_0 = ⋀¬P_j`）。
    #[test]
    fn all_false_yields_hold_c0() {
        assert_eq!(
            choose_action(false, false, false, false, false, false, false, false, false),
            ActionClass::Hold
        );
        assert!(c_holds(0, [false; 9]), "全假 ⟹ C_0 成立");
        assert_eq!(ActionClass::Hold.index(), 0);
    }

    /// 优先级：低索引谓词压制高索引（alpha2 §5「取最小成立索引 r」）——p1 真则无论其余如何均选 C_1。
    #[test]
    fn lowest_index_predicate_dominates() {
        // p1 真 + 所有其余真 ⟹ 仍选 C_1（最小索引压制）。
        assert_eq!(
            choose_action(true, true, true, true, true, true, true, true, true),
            ActionClass::InsolventOrLiquidation
        );
        // p1 假、p4 真、p4 之后全真 ⟹ 选 C_4（p1..p3 假，p4 是最小成立索引）。
        assert_eq!(
            choose_action(false, false, false, true, true, true, true, true, true),
            ActionClass::PhaseTwoReturnCapital
        );
    }

}
