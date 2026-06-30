//! 全互斥买卖点解释器——固定优先级互斥化 `C_j = P_j ∧ ⋀_{k<j} ¬P_k`（买卖点alpha2.pdf
//! Doc2 §5 + Doc3 §6 + Doc2 §9 定理1）。
//!
//! ## 契约锚（`/Users/silencehan/Downloads/买卖点alpha2.pdf` Doc2 §5 / Doc3 §6，逐字）
//!
//! 原始谓词 `P_1..P_m`（m=8）**可重叠**（同一时刻多个 P_j 同时成立）：
//! - `P1` = 风险强平
//! - `P2` = 已有多头且出现卖点，平多
//! - `P3` = 已有空头且出现买点，平空
//! - `P4` = 父声部 active 且出现反父方向证书，开短差
//! - `P5` = 空仓/ambient 状态下出现买点，开多
//! - `P6` = 空仓/ambient 状态下出现卖点，开空
//! - `P7` = 已有同向声部，记录或加仓
//! - `P8` = 无有效动作，保持
//!
//! **固定优先级互斥化**（Doc3 §6）：
//! ```text
//! C_1 = P_1
//! C_j = P_j ∧ ⋀_{k<j} ¬P_k    (j = 2..m)
//! C_0 = ⋀_{j=1}^m ¬P_j         （兜底）
//! ```
//!
//! **定理 1（Doc2 §9 / Doc3 §6）**：`Σ_{j=0}^m 1[C_j] = 1`（全互斥 + 全定义）。
//! 证明：若无 P_j 成立则 C_0 唯一成立；否则取最小成立索引 r，则 C_r 唯一成立
//! （所有 j<r 因 P_j=0 不成立，所有 j>r 因 ¬P_r=0 不成立）。
//!
//! 全定义解释器（Doc2 §5）：`I_Θ(A_t, Γ_t) = (D_t, O_t, L_t)`——D=关闭声部 / O=开启声部 /
//! L=记录但不执行的信号。本模块产出该三元组的语义裁决码（[`MutexClass`]），下游
//! [`super::interp`] 的三桶 fold 据此分流（D↔close / O↔open / L↔record）。
//!
//! ## 与既有模块的关系（no-patch-mentality：不重造，显式化既有隐式优先级）
//!
//! - [`super::intent::action_priority`]：把 10 级优先级**压缩进 match 分支顺序**——优先级隐式编码在
//!   分支序里，输入是 `ClassLabel`（risk_mode×phase 摘要），**无显式可重叠谓词向量**，故无法机器
//!   判定「可重叠谓词互斥化恰好一个成立」（Rust match 穷尽性 = 输入域全覆盖，**不是** alpha2 定理）。
//! - [`super::interp::interpret`]：候选集 Γ 按 ≺_Θ 全序 fold（买卖点**重合时的候选唯一化**），不是
//!   动作意图谓词 P_j 的互斥化。
//!
//! 本模块补的是 alpha2 定理 1 的**显式机器判定**：给定可重叠谓词向量 `P ∈ {0,1}^8`，固定优先级
//! 互斥化为唯一 C_j，可证伪核心 = 穷举 2^8 组合断言 `Σ_j 1[C_j]=1`（[`tests`] 的 `mutex_total`）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! **L0 结构定理**（非 L2 alpha）：`Σ_{j=0}^m 1[C_j]=1` 是固定优先级互斥化的**组合逻辑恒等式**
//! （alpha2 Doc2§5 / Doc3§6 证毕，零信息增量同义反复）。Rust 穷举 2^8 验证 = **L1 管线正确性**
//! （验证互斥化实装无 bug，不验证谓词 P_j 经验有效——P_j 的市场触发率/盈利性是 L2 未覆盖）。

/// 谓词数 m=8（P_1..P_8）。
pub const M: usize = 8;

/// 互斥化裁决类号（Doc3 §6）。`C_0`=兜底（无谓词成立），`C_j`(j=1..8)=最小成立谓词索引。
///
/// `Σ_{j=0}^8 1[C_j]=1`：给定任意谓词向量恰好对应一个 `MutexClass`（全互斥 + 全定义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutexClass {
    /// C_0：⋀_j ¬P_j（无任何谓词成立，兜底）。
    C0,
    /// C_j：P_j ∧ ⋀_{k<j} ¬P_k（最小成立索引 j∈1..=8，存 1-based 谓词号）。
    Cj(u8),
}

/// 原始谓词向量 `P ∈ {0,1}^8`（可重叠——多个分量可同时为 true，alpha2 §5）。
///
/// 字段名对齐 alpha2 §5/§6 谓词语义（1-based 谓词号见各字段）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Predicates {
    /// P1：风险强平。
    pub risk_liquidate: bool,
    /// P2：已有多头且出现卖点，平多。
    pub close_long: bool,
    /// P3：已有空头且出现买点，平空。
    pub close_short: bool,
    /// P4：父声部 active 且出现反父方向证书，开短差。
    pub open_short_diff: bool,
    /// P5：空仓/ambient 出现买点，开多。
    pub open_long: bool,
    /// P6：空仓/ambient 出现卖点，开空。
    pub open_short: bool,
    /// P7：已有同向声部，记录或加仓。
    pub same_dir_record_add: bool,
    /// P8：无有效动作，保持。
    pub hold: bool,
}

impl Predicates {
    /// 谓词向量按 1-based 索引读 `P_j`（j∈1..=8）。索引越界 ⟹ panic（内部不变量，j 来自 0..M 循环）。
    fn p(&self, j: usize) -> bool {
        match j {
            1 => self.risk_liquidate,
            2 => self.close_long,
            3 => self.close_short,
            4 => self.open_short_diff,
            5 => self.open_long,
            6 => self.open_short,
            7 => self.same_dir_record_add,
            8 => self.hold,
            _ => unreachable!("谓词索引 j={j} 越界（合法 1..={M}）"),
        }
    }
}

/// 固定优先级互斥化（Doc3 §6）：`C_j = P_j ∧ ⋀_{k<j} ¬P_k`，兜底 `C_0 = ⋀_j ¬P_j`。
///
/// 实装即定理证明的构造形式——取**最小成立谓词索引** r（C_r 唯一成立）；无成立谓词 ⟹ C_0。
/// 全定义（任意输入返回唯一 [`MutexClass`]）+ 全互斥（`Σ_j 1[C_j]=1`，构造保证恰一个）。
///
/// **边界条件**：全 false ⟹ C0；存在 true ⟹ Cj(最小成立索引)。优先级**不可交换**——更小索引的
/// 谓词成立时屏蔽所有更大索引（P1 风险强平 ≻ … ≻ P8 保持，alpha2 §16 优先级链）。
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

#[cfg(test)]
mod tests {
    use super::*;

    /// ★可证伪核心（alpha2 Doc2§9 定理 1）：穷举全部 2^8=256 谓词组合，断言每个组合
    /// **恰好一个** C_j 成立（`Σ_{j=0}^8 1[C_j]=1`）。若存在组合 Σ≠1 则 fail。
    ///
    /// 互斥化由 [`mutex_class`] 产唯一 [`MutexClass`]——本测试独立重算每个 C_j 的指示函数
    /// `1[C_j]`（不调 `mutex_class`，避免循环论证），逐组合求和断言 ==1，并交叉验证求和命中的
    /// 那个 j 与 `mutex_class` 返回的类号一致（实装 == 独立定义）。
    #[test]
    fn mutex_total_exhaustive_2pow8() {
        for bits in 0u32..(1 << M) {
            let p = decode(bits);

            // 独立重算指示函数（不调 mutex_class）。
            // 1[C_j] = P_j ∧ ⋀_{k<j} ¬P_k；1[C_0] = ⋀_j ¬P_j。
            let mut sum = 0usize;
            let mut hit_class: Option<MutexClass> = None;

            // C_0 兜底。
            let any = (1..=M).any(|j| p.p(j));
            if !any {
                sum += 1;
                hit_class = Some(MutexClass::C0);
            }
            // C_j。
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
                "bits={bits:08b}: Σ_j 1[C_j]={sum} ≠ 1（互斥性/全定义破裂，alpha2 定理 1 反例）"
            );
            // 实装 == 独立定义（交叉验证）。
            assert_eq!(
                mutex_class(&p),
                hit_class.unwrap(),
                "bits={bits:08b}: mutex_class 实装 ≠ 独立重算的命中类号"
            );
        }
    }

    /// 全定义：任意输入返回唯一 MutexClass（穷举已覆盖；此处显式断言无 panic + 全 false → C0）。
    #[test]
    fn mutex_total_definedness() {
        // 全 false → 兜底 C0。
        assert_eq!(mutex_class(&Predicates::default()), MutexClass::C0);
        // 穷举不 panic（决定性全函数）。
        for bits in 0u32..(1 << M) {
            let _ = mutex_class(&decode(bits));
        }
    }

    /// 优先级屏蔽：P1（风险强平）成立时屏蔽所有更高索引（即使 P2..P8 全 true）⟹ C_1。
    #[test]
    fn priority_p1_masks_all() {
        let p = Predicates {
            risk_liquidate: true,
            close_long: true,
            close_short: true,
            open_short_diff: true,
            open_long: true,
            open_short: true,
            same_dir_record_add: true,
            hold: true,
        };
        assert_eq!(mutex_class(&p), MutexClass::Cj(1), "P1 成立 ⟹ C_1（屏蔽 P2..P8）");
    }

    /// 优先级取最小成立索引：P1 false、P4 起为 true ⟹ C_4（P2/P3 false 不屏蔽，P4 最小成立）。
    #[test]
    fn priority_min_index() {
        let p = Predicates {
            open_short_diff: true, // P4
            open_long: true,       // P5
            hold: true,            // P8
            ..Default::default()
        };
        assert_eq!(mutex_class(&p), MutexClass::Cj(4), "最小成立索引 r=4 ⟹ C_4");
    }

    /// bits → Predicates 解码（bit j-1 ↔ P_j，1-based）。
    fn decode(bits: u32) -> Predicates {
        let b = |j: usize| (bits >> (j - 1)) & 1 == 1;
        Predicates {
            risk_liquidate: b(1),
            close_long: b(2),
            close_short: b(3),
            open_short_diff: b(4),
            open_long: b(5),
            open_short: b(6),
            same_dir_record_add: b(7),
            hold: b(8),
        }
    }
}
