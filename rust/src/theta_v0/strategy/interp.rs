//! R_Θ 解释器（七链 **环5**，spec §11-§12）：候选集 Γ(x) → 平移不变全序 ≺_Θ 唯一化 →
//! 三桶 (𝒟_x, ℬ_x, 𝒦_x)。
//!
//! ## 契约锚（spec `2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`）
//!
//! - **§11 局部解释器 ℐ_***（line 561-565）：`ℐ_* : 𝒮_* → (𝒟_*, ℬ_*, 𝒦_*)`，把归一化单元的
//!   语法状态映射为三元组——`𝒟_*` 应**关闭**的腿/元素、`ℬ_*` 应**开启**的腿/元素、`𝒦_*`
//!   被**记录但暂不执行**的候选。
//! - **§12 冲突互斥化 ℛ_Θ**（line 597-627）：同一时刻多买卖点构成有限候选集 `Γ(x)`
//!   （`|Γ(x)|<∞`，由所有级别×所有买卖点×所有区间套确认生成）；固定**平移不变**自相似总序
//!   `≺_Θ`（`g₁≺_Θ g₂ ⟺ S_k g₁≺_Θ S_k g₂`），解释器按序处理 `(𝒟_x,ℬ_x,𝒦_x)=ℛ_Θ(Γ(x))`，
//!   `∀x ∃!(𝒟_x,ℬ_x,𝒦_x)`（唯一三元组）。这是买卖点重合时的唯一化机制。
//! - **§13 活动集更新**（line 657）：`A^{raw}_{t+1}=(A_t∖𝒟_x)∪ℬ_x`——故 `𝒟_x⊆A_t`（应关闭的
//!   **活动腿**），`ℬ_x` 是新开启的候选。本模块据此把 `𝒟_x` 定为 [`ActiveLeg`] 子集、`ℬ_x`/`𝒦_x`
//!   定为 [`Candidate`] 子集。
//!
//! ## 唯一性三依据（§12 line 621-627，∃! 的充分条件）
//!
//! 1. `Γ(x)` 有限（[`assemble_gamma`] 返回 `Vec`，`|Γ|=len`）；
//! 2. `≺_Θ` 是**全序**（[`theta_key`] 用全确定的字典序键，含 `gamma_index` 终局键保证无不可比对）；
//! 3. 每步接收/拒绝/关闭/开启规则确定（[`interpret`] 的确定性 fold，无随机/无浮点比较）。
//!
//! ## 平移不变性（≺_Θ 自相似，§12 line 613 + §18 line 926 `ℛ_Θ(S_k Γ)=S_k ℛ_Θ(Γ)`）
//!
//! 级别平移算子 `S_k : ℓ ↦ ℓ+k` 作用于**所有**候选。[`theta_key`] 的排序键全部满足 S_k 不变或
//! S_k 序保持：`level`（S_k 平移所有级别，DESC 序保持）、`bsp_class`/`source_index`/方向/角色/
//! `gamma_index`（皆不受级别平移影响）⟹ `g₁≺_Θ g₂ ⟺ S_k g₁≺_Θ S_k g₂`（line 613 方框）。
//!
//! ## 复用既有原语（别重造，no-patch-mentality）
//!
//! - **18 类角色 [`OperationRole`]**（环4，spec §8）：[`coverage::operation_role`]（**只读**，
//!   rust-role18 完成）——本模块为每个候选建独立根 [`CoverageElement`]（`parent=None`=边界胚元 ∂，
//!   去根化）取 R(g)=(H,Ambient,δ)。
//! - **关闭谓词反向信号 [`reverse_signal`]**（§9 closePred 反向项，contract-anchored
//!   `Origin.SubVoiceOpenClose`）：[`interpret`] 的 𝒟_x 反向关闭复用之（持仓腿遇反向候选 ⟹ 关闭）。
//! - **区间套证书 [`nest::chi_bool`]**（环2，spec §6 N^δ）：[`assemble_gamma`] 的 `nest_confirmed`
//!   取证书基例 Conf^δ。
//! - **根方向消歧 [`root_sel`]**（strict §9 RootSel_Θ 镜像反对称）：候选方向 σ_g 由之定。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! **L0 操作语义结构**（非 L2 alpha）：本模块是确定性结构变换（Γ→排序→fold→三桶），不依赖
//! 经验市场数据，无择时盈利声明。`≺_Θ` 全序 + fold 确定性使 ∃! 成立是 L0 同义反复（定义内蕴）。
//! 18 类角色经验可达性（哪些组合真实出现）是 **L2 未覆盖**（spec 疑点5）——**不**声称已验证。

use super::super::classifier::Classification;
use super::super::types::BspBits;
use super::coverage::{self, CoverageElement, Dir, Horizontal, OperationRole, Vertical};
use super::exec::reverse_signal;
use super::nest::{self, Interval, NestLevel};
use super::voice::{root_sel, RootCandidates, VoiceSide};
use std::cmp::{Ordering, Reverse};

/// Γ(x) 的单个候选 g（环3：**级别 × 买卖点 × 区间套确认**的元组，spec §12 line 597-605）。
///
/// 逐字段对照 spec：
/// - `level`：候选所在级别 ℓ（环3「所有级别」）。
/// - `source_index`：买卖点触发点的 L0 原始 K 序（spec:16 平局键 + 时刻 x 分组键）。
/// - `bits`：买卖点向量 b_ℓ∈{0,1}^6（环1，**非互斥**可重合）。
/// - `dir`：σ_g（根方向消歧 [`root_sel`]：买侧→Long/卖侧→Short/双侧或空→Flat=不可交易候选）。
/// - `bsp_class`：最小成立类号 1/2/3（spec:54 同 level 类序；`u8::MAX`=该方向无类）。
/// - `role`：R(g)=(H,V,δ)（环4，18 类完全分类，复用 [`coverage::operation_role`]）。
/// - `nest_confirmed`：区间套确认 N^δ（环2，证书基例 Conf^δ，[`nest::chi_bool`]）。
/// - `gamma_index`：Γ 内原始序（≺_Θ **终局 tiebreak** 保证全序；平移不变——S_k 不重排候选）。
#[derive(Debug, Clone, Copy)]
pub struct Candidate {
    /// 级别 ℓ。
    pub level: u32,
    /// 买卖点触发点 source_index（时刻 x 分组键 + 平局键）。
    pub source_index: usize,
    /// 买卖点向量 b_ℓ（非互斥）。
    pub bits: BspBits,
    /// σ_g：候选方向（root_sel 消歧）。
    pub dir: VoiceSide,
    /// 最小成立类号 1/2/3（u8::MAX=无类）。
    pub bsp_class: u8,
    /// R(g)=(H,V,δ)：18 类角色（复用 coverage）。
    pub role: OperationRole,
    /// N^δ 区间套确认（证书基例 Conf^δ）。
    pub nest_confirmed: bool,
    /// Γ 原始序（≺_Θ 终局 tiebreak）。
    pub gamma_index: usize,
}

/// 活动集 A_t 的元素（活动腿），spec §13 `A^{raw}_{t+1}=(A_t∖𝒟_x)∪ℬ_x` 的 A_t/𝒟_x 元素。
///
/// 一个活动腿 = 某级别的一个已开持仓（方向 + 开仓信号源）。`𝒟_x⊆A_t`（关闭的是活动腿，非候选）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveLeg {
    /// 该腿级别 ℓ。
    pub level: u32,
    /// 该腿方向 σ（Long/Short；Flat 不应入活动集）。
    pub dir: VoiceSide,
    /// 该腿开仓信号源（关闭时回溯 + 平局键）。
    pub source_index: usize,
}

/// 解释器输出三桶 (𝒟_x, ℬ_x, 𝒦_x)（spec §11 line 565-571 + §12 line 617）。
///
/// 互斥分流：`𝒟_x`（关闭活动腿）∩`ℬ_x`/`𝒦_x`（候选）为空（不同类型）；`ℬ_x`∩`𝒦_x`=∅
/// （每个候选恰落一桶，[`interpret`] fold 保证）。
#[derive(Debug, Clone, Default)]
pub struct Buckets {
    /// 𝒟_x：应**关闭**的活动腿（⊆ A_t；反向信号触发，§9 closePred 反向项）。
    pub close: Vec<ActiveLeg>,
    /// ℬ_x：应**开启**的候选（新建腿）。
    pub open: Vec<Candidate>,
    /// 𝒦_x：**记录但不执行**的候选（冲突/重复/非方向）。
    pub record: Vec<Candidate>,
}

// ════════════════════════════════════════════════════════════════════════════
//  环3：候选集 Γ(x) 组装（所有级别 × 买卖点 × 区间套确认）
// ════════════════════════════════════════════════════════════════════════════

/// 组装候选集 Γ（环3，spec §12 line 597-605：「Γ(x) 由所有级别、所有买卖点、所有区间套确认生成」）。
///
/// 遍历 `classification.levels` 的每个买卖点 `BspPoint`，建一个 [`Candidate`]：方向 σ_g 由
/// [`root_sel`] 消歧、角色 R(g) 复用 [`coverage::operation_role`]（独立根 ∂ 容器 → V=Ambient）、
/// 区间套确认取 [`nest::chi_bool`] 证书基例。`gamma_index` = 遍历序（**1:1 对应 BspPoint** —— 含
/// 非方向 Flat 候选，使下游可用 `gamma_index` 索引回原始 `BspPoint`）。
///
/// **边界条件**：空 `levels`/空 `bsp` ⟹ 空 Γ。Flat 方向候选（双侧或空 bits 经 root_sel 消歧为 0）
/// 仍入 Γ（[`interpret`] 归 𝒦_x 记录不执行——不可交易候选，对齐 spec「记录但暂不执行」）。
///
/// **认识论 L0**：纯结构组装（不验证候选经验有效）。
pub fn assemble_gamma(classification: &Classification) -> Vec<Candidate> {
    // 平行 CoverageElement（独立根 parent=None=边界胚元 ∂，去根化 ⟹ V=Ambient），供 operation_role
    // 取 18 类角色（只读复用 coverage）。eps=候选方向（Flat 占位 Long——其角色不被执行，归 𝒦）。
    let mut elements: Vec<CoverageElement> = Vec::new();
    let mut raw: Vec<(u32, usize, BspBits, VoiceSide, u8, bool)> = Vec::new();
    for (level_idx, level) in classification.levels.iter().enumerate() {
        let lvl = level_idx as u32;
        for point in &level.bsp {
            let dir = candidate_dir(&point.bits);
            let cls = min_class(&point.bits, dir);
            let nest_ok = nest_confirm(lvl, point.source_index, &point.bits, dir);
            elements.push(CoverageElement {
                lambda: point.source_index,
                rho: point.source_index,
                eps: match dir {
                    VoiceSide::Flat => VoiceSide::Long, // 占位（Flat 候选归 𝒦，角色不执行）
                    d => d,
                },
                level: lvl,
                parent: None,        // 独立根（§5 多独立根；去根化父=∂）
                attached_dir: None,  // σ_{p(g)}=0 ⟹ V=Ambient
            });
            raw.push((lvl, point.source_index, point.bits, dir, cls, nest_ok));
        }
    }
    raw.iter()
        .enumerate()
        .map(|(i, &(level, source_index, bits, dir, bsp_class, nest_confirmed))| Candidate {
            level,
            source_index,
            bits,
            dir,
            bsp_class,
            role: coverage::operation_role(&elements, i),
            nest_confirmed,
            gamma_index: i,
        })
        .collect()
}

/// 候选方向 σ_g（root_sel 镜像反对称消歧）：买侧→Long、卖侧→Short、双侧/空→Flat（不可交易）。
fn candidate_dir(bits: &BspBits) -> VoiceSide {
    root_sel(RootCandidates {
        long_trigger: bits.conf_plus(),
        short_trigger: bits.conf_minus(),
    })
}

/// 最小成立类号（spec:54 同 level 1<2<3）。方向感知：Long 看买点位、Short 看卖点位；无类→u8::MAX。
fn min_class(bits: &BspBits, dir: VoiceSide) -> u8 {
    let (c1, c2, c3) = match dir {
        VoiceSide::Long => (bits.buy1, bits.buy2, bits.buy3),
        VoiceSide::Short => (bits.sell1, bits.sell2, bits.sell3),
        VoiceSide::Flat => return u8::MAX,
    };
    if c1 {
        1
    } else if c2 {
        2
    } else if c3 {
        3
    } else {
        u8::MAX
    }
}

/// 区间套确认 N^δ 证书基例 Conf^δ（环2，spec §6 line 283-285）。
///
/// 复用 [`nest::chi_bool`]：扁平 classification（无跨级塔，对齐 coverage 诚实标注）下，候选自身级别
/// 作执行级 e=ℓ 的**末级链**（base case `N^δ_{ℓ↓e}=Conf^δ_e`，spec §6 分段函数 ℓ=e 分支）。
/// `confirm_ok = Conf^δ`（买向 conf_plus / 卖向 conf_minus）。
///
/// **诚实有效域**（formalization-validity-domain）：完整 N^δ 跨级递归链（ℓ>e：Candidate∧⊆∧子证书）
/// 需 `LeveledMove` 真嵌套塔——`Classification` 不导出塔（MEMORY coverage-engine-needs-tower-export-bridge），
/// 故此处只产**证书基例**（单级末端 Conf）。Flat 方向 ⟹ false（无方向无确认）。
fn nest_confirm(level: u32, source_index: usize, bits: &BspBits, dir: VoiceSide) -> bool {
    let confirm_ok = match dir {
        VoiceSide::Long => bits.conf_plus(),
        VoiceSide::Short => bits.conf_minus(),
        VoiceSide::Flat => return false,
    };
    let chain = [NestLevel {
        lvl: level,
        cands: vec![Interval::new(source_index as u64, source_index as u64, 0)],
        candidate_ok: true,
        confirm_ok,
    }];
    nest::chi_bool(level, &chain)
}

// ════════════════════════════════════════════════════════════════════════════
//  环5：≺_Θ 平移不变全序 + 确定性 fold → 三桶 ℛ_Θ
// ════════════════════════════════════════════════════════════════════════════

/// ≺_Θ 排序键（spec §12 line 609-613：平移不变全序）。字典序：
/// 1. `Reverse(level)`：**高 level 先**（spec:54 高 level 先处理）。S_k 平移所有级别 ⟹ DESC 序保持。
/// 2. `bsp_class`：1<2<3（spec:54 同 level 类序）。S_k 无关。
/// 3. `source_index`：升序（spec:16 平局升序）。S_k（级别平移）不动时间序。
/// 4. 方向码 / 角色码（H,V,δ）：固定枚举序（确定性，S_k 无关）。
/// 5. `gamma_index`：**终局 tiebreak**——保证任意两候选可比 ⟹ ≺_Θ 是**全序**（非偏序，∃! 前提2）。
///    S_k 不重排 Γ ⟹ gamma_index 平移不变。
fn theta_key(c: &Candidate) -> (Reverse<u32>, u8, usize, u8, u8, u8, u8, usize) {
    (
        Reverse(c.level),
        c.bsp_class,
        c.source_index,
        side_code(c.dir),
        h_code(c.role.h),
        v_code(c.role.v),
        dir_code(c.role.delta),
        c.gamma_index,
    )
}

fn side_code(s: VoiceSide) -> u8 {
    match s {
        VoiceSide::Long => 0,
        VoiceSide::Short => 1,
        VoiceSide::Flat => 2,
    }
}

fn h_code(h: Horizontal) -> u8 {
    match h {
        Horizontal::First => 0,
        Horizontal::SameFollow => 1,
        Horizontal::SameReverse => 2,
    }
}

fn v_code(v: Vertical) -> u8 {
    match v {
        Vertical::Ambient => 0,
        Vertical::FollowParent => 1,
        Vertical::ShortDiff => 2,
    }
}

fn dir_code(d: Dir) -> u8 {
    match d {
        Dir::Plus => 0,
        Dir::Minus => 1,
    }
}

/// ≺_Θ 严格小于（`g₁ ≺_Θ g₂`，spec §12 line 609）：[`theta_key`] 字典序严格小于。
///
/// 全序见证：`theta_key` 末键 `gamma_index` 唯一 ⟹ 任意 g₁≠g₂ 必可比（恰一为真）。
pub fn theta_lt(a: &Candidate, b: &Candidate) -> bool {
    theta_key(a).cmp(&theta_key(b)) == Ordering::Less
}

/// **解释器 ℛ_Θ**（环5，spec §12 line 617 `(𝒟_x,ℬ_x,𝒦_x)=ℛ_Θ(Γ(x))`）：按 ≺_Θ 排序候选 +
/// 确定性 fold → 三桶 (𝒟_x close / ℬ_x open / 𝒦_x record)。
///
/// 输入 `gamma`=Γ(x)（某时刻候选集）、`active`=A_t（当前活动腿）。**不可变**（拷贝排序/拷贝活动集，
/// 不 mutate 输入，对齐 coding-style 不可变原则）。
///
/// ## 确定性 fold 规则（每候选 g 按 ≺_Θ 序，spec §11 「接收/拒绝/关闭/开启规则确定」）
///
/// 1. **非方向候选**（σ_g=Flat / 无类）⟹ `𝒦_x`（记录不执行，spec「记录但暂不执行的候选」）。
/// 2. **反向关闭**：A_t 中存在同级别、未关闭、方向被 g 反向（[`reverse_signal`]，§9 closePred 反向项
///    χ^{σ_p}）的活动腿 ⟹ 该腿入 `𝒟_x`，标记关闭，g 作为关闭触发被消费（spec §13 `A_t∖𝒟_x`）。
/// 3. **开启**：A_t 无同级别同向活动腿 **且** 本 fold 未在同 (level,σ) slot 开过 ⟹ g 入 `ℬ_x`，
///    登记 slot（spec §13 `∪ℬ_x`）。
/// 4. **冲突/重复**（slot 已被同向腿占据，或本 fold 已开同 slot）⟹ `𝒦_x`（记录不执行——
///    买卖点重合的唯一化裁决：≺_Θ 在前者赢，后者记录）。
///
/// ## 唯一性 ∃!（spec §12 line 627）
///
/// 三依据齐备：Γ 有限（len）+ ≺_Θ 全序（theta_key 终局键）+ fold 确定（无随机/浮点）⟹ 给定
/// (Γ,A_t) 唯一 (𝒟,ℬ,𝒦)。**输入顺序无关**（先排序）⟹ `interpret(Γ,A) == interpret(perm(Γ),A)`。
///
/// **边界条件**：`active` 空 ⟹ `close` 必空（无腿可关，规则2 不触发）；此时全部可交易候选按 slot
/// 唯一化分流 open/record。`gamma` 空 ⟹ 三桶皆空。
pub fn interpret(gamma: &[Candidate], active: &[ActiveLeg]) -> Buckets {
    // ① ≺_Θ 排序（拷贝引用，不 mutate 输入）。
    let mut ordered: Vec<&Candidate> = gamma.iter().collect();
    ordered.sort_by(|a, b| theta_key(a).cmp(&theta_key(b)));

    // ② 确定性 fold。working = A_t 的工作拷贝（bool=本 fold 已关闭）；opened=本 fold 已开 (level,σ)。
    let mut working: Vec<(ActiveLeg, bool)> = active.iter().map(|&l| (l, false)).collect();
    let mut opened: Vec<(u32, VoiceSide)> = Vec::new();
    let mut buckets = Buckets::default();

    for &c in &ordered {
        // 规则1：非方向候选 ⟹ 𝒦_x。
        if c.dir == VoiceSide::Flat || c.bsp_class == u8::MAX {
            buckets.record.push(*c);
            continue;
        }
        // 规则2：反向关闭 A_t 中同级别活动腿（reverse_signal 复用 §9 closePred 反向项）。
        if let Some(pos) = working.iter().position(|(leg, closed)| {
            !*closed && leg.level == c.level && reverse_signal(leg.dir, &c.bits)
        }) {
            working[pos].1 = true;
            buckets.close.push(working[pos].0);
            continue;
        }
        // 规则3/4：开启 vs 记录（slot = (level, σ_g)）。
        let slot_in_at = working
            .iter()
            .any(|(leg, closed)| !*closed && leg.level == c.level && leg.dir == c.dir);
        let slot_this_fold = opened.iter().any(|&(lv, d)| lv == c.level && d == c.dir);
        if !slot_in_at && !slot_this_fold {
            buckets.open.push(*c); // 规则3：开启
            opened.push((c.level, c.dir));
        } else {
            buckets.record.push(*c); // 规则4：冲突/重复 ⟹ 记录不执行
        }
    }
    buckets
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::classifier::bsp::BspPoint;
    use super::super::super::classifier::LevelState;
    use super::super::super::types::Center;

    fn buy_point(source_index: usize, class: u8) -> BspPoint {
        let bits = match class {
            1 => BspBits { buy1: true, ..Default::default() },
            2 => BspBits { buy2: true, ..Default::default() },
            _ => BspBits { buy3: true, ..Default::default() },
        };
        BspPoint {
            source_index,
            bits,
            pivot_low: 90,
            pivot_high: 0,
            center: Some(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 }),
        }
    }

    fn sell_point(source_index: usize, class: u8) -> BspPoint {
        let bits = match class {
            1 => BspBits { sell1: true, ..Default::default() },
            2 => BspBits { sell2: true, ..Default::default() },
            _ => BspBits { sell3: true, ..Default::default() },
        };
        BspPoint {
            source_index,
            bits,
            pivot_low: 0,
            pivot_high: 210,
            center: Some(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 }),
        }
    }

    fn classification(levels: Vec<Vec<BspPoint>>) -> Classification {
        Classification {
            levels: levels
                .into_iter()
                .map(|bsp| LevelState { bsp, ..Default::default() })
                .collect(),
        }
    }

    /// 环3 Γ 组装：每个 BspPoint → 一个候选（方向/类号/角色/区间套确认/gamma_index 1:1）。
    #[test]
    fn assemble_gamma_one_candidate_per_bsp() {
        let c = classification(vec![vec![buy_point(0, 3), sell_point(5, 1)]]);
        let gamma = assemble_gamma(&c);
        assert_eq!(gamma.len(), 2);
        assert_eq!(gamma[0].dir, VoiceSide::Long);
        assert_eq!(gamma[0].bsp_class, 3);
        assert_eq!(gamma[0].gamma_index, 0);
        assert!(gamma[0].nest_confirmed, "Conf^+ 基例确认（买侧 bits 置位）");
        assert_eq!(gamma[1].dir, VoiceSide::Short);
        assert_eq!(gamma[1].bsp_class, 1);
        // 角色去根化：独立根 parent=None ⟹ V=Ambient（spec P7，非 RootRole）。
        assert_eq!(gamma[0].role.v, Vertical::Ambient);
        assert_eq!(gamma[1].role.v, Vertical::Ambient);
    }

    /// 环3：Flat 方向候选（双侧 bits 经 root_sel 镜像反对称消歧为 0）仍入 Γ（下游归 𝒦）。
    #[test]
    fn assemble_gamma_includes_flat_dir_candidate() {
        let both = BspPoint {
            source_index: 0,
            bits: BspBits { buy1: true, sell1: true, ..Default::default() }, // (1,1) → root_sel=Flat
            pivot_low: 90,
            pivot_high: 210,
            center: None,
        };
        let gamma = assemble_gamma(&classification(vec![vec![both]]));
        assert_eq!(gamma.len(), 1);
        assert_eq!(gamma[0].dir, VoiceSide::Flat, "(1,1) 镜像不动点 ⟹ Flat（root_sel）");
        assert!(!gamma[0].nest_confirmed, "Flat ⟹ 无方向确认");
    }

    /// ★环5 𝒟_x 非空（解释器关闭机制真实坐实，非 stub）：持仓 Long 活动腿遇反向卖候选 ⟹ 关闭。
    #[test]
    fn interpret_reverse_candidate_closes_active_leg() {
        let gamma = assemble_gamma(&classification(vec![vec![sell_point(10, 1)]]));
        let active = [ActiveLeg { level: 0, dir: VoiceSide::Long, source_index: 0 }];
        let b = interpret(&gamma, &active);
        assert_eq!(b.close.len(), 1, "反向卖候选关闭持仓 Long 腿（𝒟_x 非空）");
        assert_eq!(b.close[0].dir, VoiceSide::Long);
        assert!(b.open.is_empty(), "关闭触发候选被消费，不再开启");
        assert!(b.record.is_empty());
    }

    /// 环5 ℬ_x：空活动集 ⟹ 可交易候选开启（𝒟_x 必空）。
    #[test]
    fn interpret_empty_active_opens_candidates() {
        let gamma = assemble_gamma(&classification(vec![vec![buy_point(0, 3)]]));
        let b = interpret(&gamma, &[]);
        assert!(b.close.is_empty(), "空 A_t ⟹ 𝒟_x 必空（无腿可关）");
        assert_eq!(b.open.len(), 1);
        assert_eq!(b.open[0].dir, VoiceSide::Long);
        assert!(b.record.is_empty());
    }

    /// 环5 𝒦_x：同 (level,σ) 重复候选（同向）⟹ ≺_Θ 在前者开启，后者记录不执行（重合唯一化）。
    #[test]
    fn interpret_duplicate_slot_records_loser() {
        // 同级别两个买候选（同向），≺_Θ 排序后第一个 open，第二个 record。
        let gamma = assemble_gamma(&classification(vec![vec![buy_point(0, 3), buy_point(3, 3)]]));
        let b = interpret(&gamma, &[]);
        assert_eq!(b.open.len(), 1, "同 slot 仅一个开启（唯一化）");
        assert_eq!(b.record.len(), 1, "重复同向候选记录不执行");
    }

    /// 环5 𝒦_x：持仓同向候选（已有 Long 腿遇买候选）⟹ 记录不执行（不重复开同向）。
    #[test]
    fn interpret_same_dir_as_active_records() {
        let gamma = assemble_gamma(&classification(vec![vec![buy_point(0, 1)]]));
        let active = [ActiveLeg { level: 0, dir: VoiceSide::Long, source_index: 0 }];
        let b = interpret(&gamma, &active);
        assert!(b.close.is_empty());
        assert!(b.open.is_empty());
        assert_eq!(b.record.len(), 1, "已持同向 ⟹ 候选记录不执行");
    }

    /// 环5 非方向候选（Flat）⟹ 𝒦_x。
    #[test]
    fn interpret_flat_candidate_records() {
        let both = BspPoint {
            source_index: 0,
            bits: BspBits { buy1: true, sell1: true, ..Default::default() },
            pivot_low: 90,
            pivot_high: 210,
            center: None,
        };
        let gamma = assemble_gamma(&classification(vec![vec![both]]));
        let b = interpret(&gamma, &[]);
        assert_eq!(b.record.len(), 1);
        assert!(b.open.is_empty() && b.close.is_empty());
    }

    /// ★∃! 唯一性（spec §12 line 627）：输入顺序无关——interpret(Γ,A)=interpret(perm(Γ),A)。
    #[test]
    fn interpret_deterministic_order_independent() {
        let g1 = assemble_gamma(&classification(vec![vec![buy_point(0, 3)], vec![sell_point(5, 2)]]));
        // 反序 Γ（同候选集，不同输入顺序）。
        let mut g2 = g1.clone();
        g2.reverse();
        let active = [ActiveLeg { level: 1, dir: VoiceSide::Long, source_index: 0 }];
        let b1 = interpret(&g1, &active);
        let b2 = interpret(&g2, &active);
        // 三桶逐元素相等（gamma_index 终局键 ⟹ 排序后同序）。
        assert_eq!(b1.close, b2.close);
        assert_eq!(
            b1.open.iter().map(|c| c.gamma_index).collect::<Vec<_>>(),
            b2.open.iter().map(|c| c.gamma_index).collect::<Vec<_>>()
        );
        assert_eq!(
            b1.record.iter().map(|c| c.gamma_index).collect::<Vec<_>>(),
            b2.record.iter().map(|c| c.gamma_index).collect::<Vec<_>>()
        );
    }

    /// ★≺_Θ 平移不变（spec §12 line 613：g₁≺_Θ g₂ ⟺ S_k g₁≺_Θ S_k g₂）。
    /// S_k:ℓ↦ℓ+k 作用于两候选，全序判定不变。
    #[test]
    fn theta_order_translation_invariant() {
        let g = assemble_gamma(&classification(vec![
            vec![buy_point(0, 3)],  // level 0
            vec![sell_point(5, 1)], // level 1
        ]));
        let g0 = g[0];
        let g1 = g[1];
        let before = theta_lt(&g0, &g1);
        // S_k（k=2）：所有级别 +2。
        let shift = |mut c: Candidate, k: u32| {
            c.level += k;
            c
        };
        let s0 = shift(g0, 2);
        let s1 = shift(g1, 2);
        assert_eq!(before, theta_lt(&s0, &s1), "≺_Θ 在级别平移 S_k 下不变（自相似全序）");
    }

    /// 环5 互斥分流：每个候选恰落一桶（ℬ⊎𝒦 over 候选；𝒟 over 活动腿）。计数守恒。
    #[test]
    fn interpret_buckets_partition_candidates() {
        let gamma = assemble_gamma(&classification(vec![vec![
            buy_point(0, 3),
            buy_point(3, 3),  // 同向重复 → record
            sell_point(7, 1), // 反向 → 关闭活动 Long
        ]]));
        let active = [ActiveLeg { level: 0, dir: VoiceSide::Long, source_index: 0 }];
        let b = interpret(&gamma, &active);
        // 每个候选恰落 open 或 record（关闭触发候选被消费，不入 open/record）。
        let consumed_as_close = b.close.len(); // 反向候选数（消费为关闭）
        assert_eq!(
            b.open.len() + b.record.len() + consumed_as_close,
            gamma.len(),
            "候选守恒：open + record + 关闭触发 = |Γ|"
        );
    }
}
