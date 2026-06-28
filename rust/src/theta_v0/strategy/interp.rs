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

use super::super::classifier::recursive_tower::LeveledMove;
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
///
/// ## 双坐标身份（持久身份，codex Q4 ρ 漂移修正）
///
/// 一个活动腿携**两个** L0 原始 K 序坐标，对应其覆盖走势/候选的区间端点：
/// - `source_index = ρ`（右端点 / `LeveledMove.end_index` / 候选 bsp 触发点）——**会漂移**：父容器
///   走势延伸（吸收更多次级别子走势）时 `end_index` 增大。同时是 reference:16 平局键 + 结构止损
///   bsp 回查键（`k_theta_risk_gate` 按 `source_index` 查 `BspPoint`）。
/// - `lambda`（左端点 / `LeveledMove.start_index` / 候选 bsp 同点）——**稳定**：走势的**起点**在其
///   向右延伸时不变。故 `(level, lambda, eps)` 是走势的**稳定语义身份**（独立于 ρ 漂移）。
///
/// 候选腿（买卖点入场）`lambda == source_index`（坐在 hostOf 右端点，reference:16）；树走势腿
/// `lambda = start_index < source_index = end_index`。持久身份用 `lambda` 跨 bar 对位（见
/// `coverage::held_leg_tree_index`：ρ 精确匹配失败时回落 `lambda` 稳定匹配 = coord_drift，非 stale）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActiveLeg {
    /// 该腿级别 ℓ。
    pub level: u32,
    /// 该腿方向 σ（Long/Short；Flat 不应入活动集）。
    pub dir: VoiceSide,
    /// 该腿开仓信号源 ρ=右端点（关闭时回溯 + 平局键 + 结构止损 bsp 回查；**会随父延伸漂移**）。
    pub source_index: usize,
    /// 该腿覆盖走势的左端点 λ=start_index（**稳定语义身份坐标**，父延伸不变）。候选腿 `lambda==source_index`。
    pub lambda: usize,
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
//  环3b：塔导出桥 (ii) 喂入段——真嵌套塔 + 638 附着 → 真父子候选元素（替换扁平全根）
//  谱系：638 + 547 + coverage-engine-needs-tower-export-bridge + b2s2-still-missing-tower
// ════════════════════════════════════════════════════════════════════════════

/// 一个买卖点候选的 [`CoverageElement`]，按 **638 hostOf 附着**到真元素树 `tree`。
///
/// `dir`：候选操作方向 δ_g（[`candidate_dir`] 消歧；Flat 占位 Long——Flat 候选下游归 𝒦 不执行，
/// 占位不伪装方向，与扁平 [`assemble_gamma`] 同口径）。`parent`/`attached_dir` 来自
/// [`coverage::attach_bsp_to_tree`]（hostOf 的真 Compose 父索引 + σ_{p(g)}，铁律：真父子非级别差）。
/// `lambda=rho=source_index`：候选坐在 hostOf 右端点（reference:16），无独立操作区间跨度
/// （[`coverage::operation_role`]/[`coverage::leg_target`] 只读 parent/level/eps/attached_dir）。
fn bsp_element_attached(
    tree: &[CoverageElement],
    level_g: u32,
    source_index: usize,
    dir: VoiceSide,
) -> CoverageElement {
    let (parent, attached_dir) = coverage::attach_bsp_to_tree(tree, level_g, source_index);
    CoverageElement {
        lambda: source_index,
        rho: source_index,
        eps: match dir {
            VoiceSide::Flat => VoiceSide::Long, // 占位（Flat 候选归 𝒦，角色不执行）
            d => d,
        },
        level: level_g,
        parent,
        attached_dir,
    }
}

/// 从真嵌套塔 + 638 附着规则构造**组合元素数组**（真元素树 ++ 附着的买卖点候选元素）。
///
/// 这是塔导出桥 (ii) **喂入段**的核心：替换 [`assemble_gamma`] 的扁平全根（独立根 ∂、V 恒 Ambient）。
/// 两段：
/// 1. `tree = extract_elements(tower)`：真嵌套元素树（`RMove::Compose` 真父子，547 铁律守护——
///    父只来自 `sub_moves` 真包含，非级别差伪造）。
/// 2. 每个 bsp 候选 → 一个 [`CoverageElement`]（[`bsp_element_attached`]），按 638 hostOf 附着到
///    `tree`（继承 hostOf 的真父 + 父方向 σ_{p(g)}）。候选元素追加在 `tree` 之后。
///
/// 返回 `(elements, candidate_start)`：`elements[..candidate_start]` 是真元素树，
/// `elements[candidate_start..]` 是附着候选（**按 `classification.levels` 层序 × 每层 `bsp` 序追加**
/// ——与 [`assemble_gamma_with_tower`] 候选序 1:1 对齐，这是 ci 索引对齐的不变量契约）。
/// 候选 i 的真父索引指向 `tree`（< candidate_start），其 [`coverage::operation_role`] 的 V 由真
/// σ_{p(g)} 派生（FollowParent/ShortDiff 真出，非恒 Ambient）；[`coverage::ancestor_close`]（经
/// [`coverage::active_set_step`]）在 `elements` 上对附着候选**真剪枝**（祖先=hostOf 父链，非空）。
///
/// **边界条件**：`tower.len()<2`（仅 L0）或候选 hostOf 是根 ⟹ σ_{p(g)}=0 ⟹ 候选 parent=None
/// （与扁平退化一致，tower-export-i 边界）。空 `levels`/空 bsp ⟹ 候选段空（仅 `tree`）。
///
/// **认识论 L0/L1**（formalization-validity-domain 231号）：纯结构组装（真嵌套塔 + 端点附着）。
/// #5 多声部对冲 alpha 来源**结构性就位**，alpha **未验证**（待 L2/L3，不声明 alpha）。
pub fn coverage_elements_with_tower(
    classification: &Classification,
    tower: &[Vec<LeveledMove>],
) -> (Vec<CoverageElement>, usize) {
    // 真嵌套元素树（独立 immutable 视图，供 hostOf 附着查表；铁律：真父子来自 push_element_tree）。
    let tree = coverage::extract_elements(tower);
    let candidate_start = tree.len();
    let mut elements = tree.clone();
    for (level_idx, level) in classification.levels.iter().enumerate() {
        let lvl = level_idx as u32;
        for point in &level.bsp {
            let dir = candidate_dir(&point.bits);
            elements.push(bsp_element_attached(&tree, lvl, point.source_index, dir));
        }
    }
    (elements, candidate_start)
}

/// 组装候选集 Γ（环3）**接真嵌套塔**——候选 [`Candidate`] 的角色 R(g)=(H,V,δ) 由 638 附着的真父子
/// 元素派生（**V 真出 FollowParent/ShortDiff**，非扁平 [`assemble_gamma`] 的恒 Ambient）。
///
/// 与 [`assemble_gamma`] 同产候选序（每个 `BspPoint` 1:1，含 Flat 候选，`gamma_index`=遍历序），
/// 唯一差异：`role` 从 [`coverage_elements_with_tower`] 的**真父子组合元素数组**取
/// （[`coverage::operation_role`] 在附着候选索引 `ci` 上读真 `attached_dir`=σ_{p(g)}）——hostOf
/// 有真 Compose 父的候选 V≠Ambient：FollowParent（δ_g=σ_{p(g)} 顺父）/ ShortDiff（δ_g=−σ_{p(g)}
/// 反向子声部对冲腿，spec §9）。`ci` 与候选追加序对齐（见 [`coverage_elements_with_tower`] 契约）。
///
/// **边界条件**：`tower.len()<2`（缺塔）或候选 hostOf 是根 ⟹ σ_{p(g)}=0 ⟹ V=Ambient（与
/// [`assemble_gamma`] 扁平退化逐候选一致，tower-export-i 边界）。空 `levels`/空 bsp ⟹ 空 Γ。
///
/// **认识论 L0/L1**：纯结构组装。#5 多声部对冲 alpha 来源结构性就位，alpha 未验证（待 L2/L3）。
pub fn assemble_gamma_with_tower(
    classification: &Classification,
    tower: &[Vec<LeveledMove>],
) -> Vec<Candidate> {
    // 真父子组合元素（tree ++ 附着候选）——role 单一来源（V 由真 σ_{p(g)} 派生）。
    let (elements, candidate_start) = coverage_elements_with_tower(classification, tower);
    let mut out: Vec<Candidate> = Vec::new();
    // ci 沿候选追加序遍历 elements 的候选段（与 coverage_elements_with_tower 同 level×bsp 序）。
    let mut ci = candidate_start;
    for (level_idx, level) in classification.levels.iter().enumerate() {
        let lvl = level_idx as u32;
        for point in &level.bsp {
            let dir = candidate_dir(&point.bits);
            out.push(Candidate {
                level: lvl,
                source_index: point.source_index,
                bits: point.bits,
                dir,
                bsp_class: min_class(&point.bits, dir),
                role: coverage::operation_role(&elements, ci), // 真父子 ⟹ V 真出（非恒 Ambient）
                nest_confirmed: nest_confirm(lvl, point.source_index, &point.bits, dir),
                gamma_index: out.len(),
            });
            ci += 1;
        }
    }
    out
}

// ════════════════════════════════════════════════════════════════════════════
//  环3c：执行层 σ_{p(g)} = **父容器方向**（639 settle，取代删除的"活动父腿"错口径）
//  执行层 σ_p 源 = [`assemble_gamma_with_tower`]（§3b，attach_bsp_to_tree host→真 Compose 父→
//  父 rmove_side），喂 **per-bar 前缀因果塔**（runner::pi_theta_fill_loop 的 classify_with_tower
//  前缀重分类，只用 ≤t 数据 → 因果）。父容器是结构对象（从 ≤t 自底向上闭包 D_t 自上而下查得），
//  **与是否持仓无关**——σ 来源（spec §7.2）与持仓准入（§13 AncOK）正交。
//
//  ★删除的错口径（639，no-patch 删不留 fallback）：`parent_dir_from_active`（活动持仓父腿方向）
//  + `assemble_gamma_active_parent`（σ_p=持仓父腿）。错口径把 §7.2「σ 来源=父容器方向」与 §13
//  「AncOK 持仓准入=父腿在 A_t」两个正交机制混为一谈：父有向但未持仓的逆向次级点，错口径判
//  Ambient（→建 naked 逆势仓 garbage trades），正确口径判 ShortDiff（→AncOK 未持父时剔除不开仓）。
//  Lean 落地 `Origin/ParentDirContainer.lean`（parentDirOfContainer 无持仓门控，struct_only 零公理）。
//  谱系：639（σ_p 来源修正）；638（hostOf 附着，σ_p 来源）；547（主力锚删除确认）。
// ════════════════════════════════════════════════════════════════════════════

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
        let active = [ActiveLeg { level: 0, dir: VoiceSide::Long, source_index: 0, lambda: 0 }];
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
        let active = [ActiveLeg { level: 0, dir: VoiceSide::Long, source_index: 0, lambda: 0 }];
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
        let active = [ActiveLeg { level: 1, dir: VoiceSide::Long, source_index: 0, lambda: 0 }];
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
        let active = [ActiveLeg { level: 0, dir: VoiceSide::Long, source_index: 0, lambda: 0 }];
        let b = interpret(&gamma, &active);
        // 每个候选恰落 open 或 record（关闭触发候选被消费，不入 open/record）。
        let consumed_as_close = b.close.len(); // 反向候选数（消费为关闭）
        assert_eq!(
            b.open.len() + b.record.len() + consumed_as_close,
            gamma.len(),
            "候选守恒：open + record + 关闭触发 = |Γ|"
        );
    }

    // ── 环3b 塔导出桥 (ii) 喂入段：638 附着 → 真父子候选（V 真出 FollowParent/ShortDiff）──

    use super::super::super::classifier::center::UnitRange;
    use super::super::super::classifier::recursive_tower::LeveledMove as LM;
    use super::super::super::config::VoiceConfig;
    use super::super::super::types::{Direction, Tick};

    fn unit_r(si: usize, ei: usize, dir: Direction, lo: Tick, hi: Tick) -> UnitRange {
        UnitRange { start_index: si, end_index: ei, direction: dir, lo, hi }
    }

    /// 单父真嵌套塔：L1 走势（外缘 Long）+ 3 个 L0 子（ρ=4/8/12，真父=L1，attached=Long）。
    fn long_parent_tower() -> Vec<Vec<LM>> {
        let s0 = LM::from_unit(&unit_r(0, 4, Direction::Up, 0, 10));
        let s1 = LM::from_unit(&unit_r(4, 8, Direction::Down, 3, 12));
        let s2 = LM::from_unit(&unit_r(8, 12, Direction::Up, 5, 15));
        let c = Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 0, end_index: 12 };
        let l1 = LM::compose(&[s0, s1, s2], c, 1); // 外缘 10→15 ⟹ Long（σ_p=+1）
        vec![Vec::new(), vec![l1]]
    }

    /// 两父塔：compose_a(Long) + compose_b(Short)，a2/b0 结构全等异 ρ（ordinal 身份测试）。
    fn two_parent_tower() -> Vec<Vec<LM>> {
        let mk = |si, ei, d, lo, hi| LM::from_unit(&unit_r(si, ei, d, lo, hi));
        let ca = LM::compose(
            &[mk(0, 4, Direction::Up, 0, 10), mk(4, 8, Direction::Down, 3, 12), mk(8, 12, Direction::Up, 5, 15)],
            Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 0, end_index: 12 },
            1,
        ); // Long
        let cb = LM::compose(
            &[mk(12, 16, Direction::Up, 5, 15), mk(16, 20, Direction::Down, 3, 12), mk(20, 24, Direction::Down, 0, 8)],
            Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 12, end_index: 24 },
            1,
        ); // Short
        vec![Vec::new(), vec![ca, cb]]
    }

    /// ★关键测试：买候选在 Long 父走势下 ⟹ V=FollowParent（V≠Ambient，真嵌套对冲腿结构产生）。
    #[test]
    fn gamma_with_tower_buy_under_long_parent_is_follow_parent() {
        let tower = long_parent_tower();
        let c = classification(vec![vec![buy_point(8, 1)]]); // level 0, src=8 ⟹ host=s1(父Long)
        let gamma = assemble_gamma_with_tower(&c, &tower);
        assert_eq!(gamma.len(), 1);
        assert_eq!(gamma[0].dir, VoiceSide::Long);
        assert_eq!(
            gamma[0].role.v,
            Vertical::FollowParent,
            "买候选 δ=Long == σ_{{p}}=Long ⟹ FollowParent（真父附着，非恒 Ambient）"
        );
    }

    /// ★关键测试：卖候选在 Long 父走势下 ⟹ V=ShortDiff（反向子声部对冲腿，spec §9，真出非空跑）。
    #[test]
    fn gamma_with_tower_sell_under_long_parent_is_short_diff() {
        let tower = long_parent_tower();
        let c = classification(vec![vec![sell_point(8, 1)]]); // level 0, src=8 ⟹ host=s1(父Long)
        let gamma = assemble_gamma_with_tower(&c, &tower);
        assert_eq!(gamma[0].dir, VoiceSide::Short);
        assert_eq!(
            gamma[0].role.v,
            Vertical::ShortDiff,
            "卖候选 δ=Short = −σ_{{p}}=−Long ⟹ ShortDiff（反向子声部对冲腿，真出）"
        );
    }

    /// 漏洞① 级别上下文：同 src=12 跨级共享端点，L0 候选有真父、L1 候选 host 是根 ⟹ Ambient。
    #[test]
    fn gamma_with_tower_level_context_disambiguates() {
        let tower = two_parent_tower();
        // L0、src=12 ⟹ host=a2（真父 compose_a=Long），买 ⟹ FollowParent。
        let g0 = assemble_gamma_with_tower(&classification(vec![vec![buy_point(12, 1)]]), &tower);
        assert_eq!(g0[0].role.v, Vertical::FollowParent, "L0 host=a2 真父 Long");
        // L1、src=12 ⟹ host=compose_a（顶层根）⟹ Ambient（级别防误命中 L0 host）。
        let g1 = assemble_gamma_with_tower(&classification(vec![vec![], vec![buy_point(12, 1)]]), &tower);
        assert_eq!(g1[0].role.v, Vertical::Ambient, "L1 host=compose_a 是根 ⟹ Ambient");
    }

    /// 漏洞③ ordinal 身份：结构全等 host（a2/b0）按 (level,ρ) 区分 ⟹ 角色 V 不同（非结构相等）。
    #[test]
    fn gamma_with_tower_ordinal_identity_struct_equal_hosts() {
        let tower = two_parent_tower();
        // src=12 → a2(父 Long)：卖 ⟹ ShortDiff。
        let g12 = assemble_gamma_with_tower(&classification(vec![vec![sell_point(12, 1)]]), &tower);
        assert_eq!(g12[0].role.v, Vertical::ShortDiff, "host=a2(Long父) 卖 ⟹ ShortDiff");
        // src=16 → b0(父 Short，与 a2 结构全等)：卖 ⟹ FollowParent。
        let g16 = assemble_gamma_with_tower(&classification(vec![vec![sell_point(16, 1)]]), &tower);
        assert_eq!(g16[0].role.v, Vertical::FollowParent, "host=b0(Short父) 卖 ⟹ FollowParent（ordinal 区分）");
    }

    /// guard：tower.len()<2（仅 L0 全根）⟹ V 恒 Ambient，与扁平 assemble_gamma 逐候选一致。
    #[test]
    fn gamma_with_tower_guard_matches_flat_when_no_compose_level() {
        let s0 = LM::from_unit(&unit_r(0, 4, Direction::Up, 0, 10));
        let s1 = LM::from_unit(&unit_r(4, 8, Direction::Down, 3, 12));
        let tower = vec![vec![s0, s1]]; // len()==1 < 2
        let c = classification(vec![vec![buy_point(4, 1)]]);
        let with = assemble_gamma_with_tower(&c, &tower);
        let flat = assemble_gamma(&c);
        assert_eq!(with[0].role.v, Vertical::Ambient, "缺塔 ⟹ host 是根 ⟹ Ambient（诚实退化）");
        assert_eq!(with[0].role.v, flat[0].role.v, "缺塔退化与扁平 assemble_gamma 一致");
    }

    /// ★AncOK 真剪枝（task step3）：附着候选孤儿（祖先 compose 不在 raw）被剪枝；含祖先则保留。
    /// 对照扁平（parent=None ⟹ AncOK 恒等不剪枝）——接真塔后 AncOK 翻转为真剪枝。
    #[test]
    fn coverage_elements_with_tower_ancok_prunes_attached_orphan() {
        let tower = long_parent_tower();
        let c = classification(vec![vec![sell_point(8, 1)]]);
        let (elements, cstart) = coverage_elements_with_tower(&c, &tower);
        assert_eq!(elements[cstart].parent, Some(0), "附着候选继承 host(s1) 真父 compose(idx0)");
        // raw 只含候选（不含祖先 compose idx0）⟹ AncOK 真剪枝（t=999 无 B/D，raw=active）。
        let pruned = coverage::active_set_step(&elements, &[cstart], 999);
        assert!(pruned.is_empty(), "孤儿附着候选（祖先 compose 不在场）被 AncOK 真剪枝");
        // raw 含祖先 compose(idx0) + 候选 ⟹ 保留（祖先齐全，覆盖闭合）。
        let kept = coverage::active_set_step(&elements, &[0, cstart], 999);
        assert!(kept.contains(&cstart), "祖先 compose 在场 ⟹ 附着候选保留");
    }

    /// ★ShortDiff 对冲腿结构产生（非空跑）：真嵌套深度≥1 ⟹ 深度权重 w[1]=0.30，方向反父声部。
    #[test]
    fn coverage_elements_with_tower_shortdiff_hedge_leg_produced() {
        let tower = long_parent_tower();
        let c = classification(vec![vec![sell_point(8, 1)]]); // 卖 under Long 父 ⟹ ShortDiff
        let (elements, cstart) = coverage_elements_with_tower(&c, &tower);
        assert_eq!(coverage::operation_role(&elements, cstart).v, Vertical::ShortDiff);
        let leg = coverage::leg_target(&elements, cstart, 100.0, &VoiceConfig::default());
        assert_eq!(leg.side, VoiceSide::Short, "ShortDiff 对冲腿方向 Short（反 Long 父声部）");
        assert_eq!(leg.role.v, Vertical::ShortDiff);
        assert!(
            (leg.units - 30.0).abs() < 1e-9,
            "真嵌套深度 1（沿真父链）⟹ w[1]=0.30 × 100 = 30（真深度权重，非空跑）"
        );
    }

    // ── 环3c 执行层 σ_p = 父容器方向（639 settle；σ 来源与持仓正交，非活动父腿）────────────

    /// ★639 口径修正坐实（旧错测试 `gamma_active_parent_no_position_is_ambient` 的精确反转）：
    /// 有向父容器（前缀因果塔 L1 Long 走势）+ 逆向次级卖候选 ⟹ V=**ShortDiff**（来自父容器方向）。
    /// 旧"活动父腿"错口径：未持仓 ⟹ Ambient（→建 naked 逆势仓 garbage trades）。正确口径：σ 来源是
    /// 结构对象（父容器方向），与持仓无关——`assemble_gamma_with_tower` **不接受** active 参数
    /// （σ_p 不可能依赖持仓，与 Lean `parentDirOfContainer_struct_only` 同构）。
    #[test]
    fn gamma_with_tower_shortdiff_from_container_not_position() {
        let tower = long_parent_tower(); // L1 Long 父走势（结构对象，非持仓）
        let c = classification(vec![vec![sell_point(8, 1)]]); // L0 卖候选，host=s1（真父 L1 Long）
        let gamma = assemble_gamma_with_tower(&c, &tower);
        assert_eq!(gamma[0].dir, VoiceSide::Short);
        assert_eq!(
            gamma[0].role.v,
            Vertical::ShortDiff,
            "有向父容器 + 未持仓 ⟹ ShortDiff（639：σ_p=父容器方向，非持仓父腿；旧错口径误判 Ambient）"
        );
    }

    /// ★639 Ambient 充要条件修正：Ambient ⟺ 父=胚元∂/无有向父容器（**非**"未持仓"）。
    /// 缺塔（仅 L0，host 是根 segment）⟹ 父=∂ ⟹ Ambient——与持仓无关（对齐 Lean
    /// `classifyV_ambient_iff_germ`：Ambient 的唯一来源是父=胚元，非父未持有）。
    #[test]
    fn gamma_with_tower_ambient_iff_germ_not_unheld() {
        let s0 = LM::from_unit(&unit_r(0, 4, Direction::Up, 0, 10));
        let s1 = LM::from_unit(&unit_r(4, 8, Direction::Down, 3, 12));
        let no_parent_tower = vec![vec![s0, s1]]; // len()==1：host 是根 segment，无 Compose 父 = ∂
        let c = classification(vec![vec![sell_point(8, 1)]]);
        let gamma = assemble_gamma_with_tower(&c, &no_parent_tower);
        assert_eq!(
            gamma[0].role.v,
            Vertical::Ambient,
            "父=胚元∂（无有向父容器）⟹ Ambient（639：充要条件是父=∂，非未持仓）"
        );
    }
}
