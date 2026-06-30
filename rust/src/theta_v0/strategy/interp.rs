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

use super::super::classifier::recursive_tower::{ElementId, LeveledMove};
use std::rc::Rc;
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
/// ## 双坐标身份（持久身份，codex Q4 确定性 ElementId 结构映射）
///
/// 一个活动腿携**两个** L0 原始 K 序坐标 + **确定性 ElementId**（跨 bar 稳定身份）：
/// - `source_index = ρ`（右端点 / `LeveledMove.end_index` / 候选 bsp 触发点）——**会漂移**：父容器
///   走势延伸（吸收更多次级别子走势）时 `end_index` 增大。同时是 reference:16 平局键 + 结构止损
///   bsp 回查键（`k_theta_risk_gate` 按 `source_index` 查 `BspPoint`）。
/// - `lambda`（左端点 / `LeveledMove.start_index` / 候选 bsp 同点）——**稳定**：走势的**起点**在其
///   向右延伸时不变（confirmed 前缀不回写）。
/// - `id`（★codex Q4）：确定性 ElementId（跨 bar 稳定，全量/增量产同 ID）——`held_leg_tree_index`
///   按 ID 匹配当前因果树元素（spec §13 `p:C_ℓ→C_{ℓ+1}` 结构映射，**非**值比较 `(level,λ,eps)`）。
///   父延伸（ρ 漂移）仍同 ID ⟹ Exact 命中（删除旧 CoordDrift 分支）。
///
/// 候选腿（买卖点入场）`lambda == source_index`（坐在 hostOf 右端点，reference:16）；树走势腿
/// `lambda = start_index < source_index = end_index`。
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
    /// ★codex Q4：跨 bar 稳定的确定性元素身份（spec §13 结构映射对象身份）。
    /// `held_leg_tree_index` 按 ID 匹配当前因果树元素（非值比较 level,λ,eps）。
    pub id: ElementId,
    /// ★codex Q4：父容器的 ElementId（真 Compose 父，跨 bar 稳定）。None = 真边界胚元 ∂。
    /// Stale 路径**不伪造** None（发现 A 修复）：保留原 parent_id，缺失非边界父 = prune 非 root。
    pub parent_id: Option<ElementId>,
    /// ★codex Q4：真边界根 ∂ 标记（host 是根 / 缺塔）vs 未解析父腿。
    /// `is_boundary_root=true` 的 Stale 腿作根保留（parent_id=None 合法）；
    /// `is_boundary_root=false` 的 Stale 腿被 prune（不入 raw，AncOK 严格 §13）。
    pub is_boundary_root: bool,
    /// ★persistent overlay（anc.pdf §6）：操作父容器 op_parent(L)——入场时操作容器，持久。
    /// 区分 pop(e)（操作父，开腿时用，驱动持仓生命周期）vs pstr_i(e)（结构父，当前 snapshot，可变）。
    /// 更高级别涌现时 pstr 变，pop 不变 → 腿不 Stale（§6）。
    /// `op_parent=None` = 无操作父（depth=0 ambient 腿）；`op_parent=Some(c)` = 入场时容器 c。
    /// ponytail: ceiling=增量 extract_elements 时 op_parent 可从 confirmed prefix 直接取。
    pub op_parent: Option<ElementId>,
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
                id: ElementId { level: lvl, ordinal: elements.len() as u64 },
                parent_id: None,
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

/// 从真嵌套塔 + 638 附着规则构造**组合元素数组**（真元素树 ++ 附着的买卖点候选元素）。
///
/// 这是塔导出桥 (ii) **喂入段**的核心：替换 [`assemble_gamma`] 的扁平全根（独立根 ∂、V 恒 Ambient）。
/// 两段：
/// 1. `tree = extract_elements(tower)`：真嵌套元素树（`RMove::Compose` 真父子，547 铁律守护——
///    父只来自 `sub_moves` 真包含，非级别差伪造）。
/// 2. 每个 bsp 候选 → 一个 [`CoverageElement`]（[`coverage::attach_bsp_to_tree`] hostOf 附着），
///    按 638 hostOf 附着到 `tree`（继承 hostOf 的真父 + 父方向 σ_{p(g)}）。候选元素追加在 `tree` 之后。
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
    tower: &[Rc<Vec<LeveledMove>>],
) -> (Vec<CoverageElement>, usize) {
    // 委托 parts 版后合并成连续 Vec（tree ++ candidates）——非热路径（诊断/l3_fullwindow），
    // materialize 一次可接受。bit-exact == 旧版（同 extract_elements tree + 同候选序）。
    let (tree, candidates, _gamma) =
        coverage_elements_and_gamma_with_tower(classification, tower);
    let candidate_start = tree.len();
    let mut elements = (*tree).clone();
    elements.extend(candidates);
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
    tower: &[Rc<Vec<LeveledMove>>],
) -> Vec<Candidate> {
    // 委托 parts 版，丢弃 tree/candidates 仅返 gamma。bit-exact == 旧版（同候选序 + role）。
    let (_tree, _candidates, gamma) =
        coverage_elements_and_gamma_with_tower(classification, tower);
    gamma
}

/// 组合元素 + 候选集 Γ **单次建树**（H2 优化：消除 `coverage_step_classification` 每 bar 双调
/// `extract_elements` 的冗余——同 `(classification, tower)` 建树两次，第二次纯重复）。
///
/// 与分别调 [`coverage_elements_with_tower`] + [`assemble_gamma_with_tower`] **bit-exact 等价**：
/// 同一 `extract_elements(tower)` 产同一 `elements`/`candidate_start`，候选段同序追加，`gamma`
/// 的 `ci` 与候选段偏移 1:1 对齐（[`coverage_elements_with_tower`] 不变量契约）。
///
/// **认识论 L0**：纯结构组装去重，无语义变化（bit-exact，同源数据同源建树）。
pub fn coverage_elements_and_gamma_with_tower(
    classification: &Classification,
    tower: &[Rc<Vec<LeveledMove>>],
) -> (Rc<Vec<CoverageElement>>, Vec<CoverageElement>, Vec<Candidate>) {
    coverage_elements_and_gamma_with_tower_cached(classification, tower, &mut None)
}

/// **工位 K 性能：tree-prefix 缓存键**（§16 confirmed prefix immutable）。
///
/// **递归发射树指纹**（工位 O，644 第3次有损指纹否定）：覆盖 [`coverage::extract_elements`] 输出
/// 依赖的**全部** `LeveledMove` 深层字段，逐节点递归 `sub_moves`。指纹元组每节点发射
/// `(id.level, id.ordinal, start_index, end_index, dir, lo, hi, sub_moves.len())`：
///
/// | extract_elements 输出字段 | 来源 LeveledMove 字段 | 指纹覆盖 |
/// |---|---|---|
/// | `lambda` | `start_index` | ✔ start_index |
/// | `rho` | `end_index` | ✔ end_index |
/// | `eps` | `rmove` 方向（Segment.direction / Compose 外缘 first.hi vs last.hi） | ✔ dir + lo/hi（外缘判据） |
/// | `level` | `rmove.level()` | ✔ id.level（compose level == id.level） |
/// | `id`/`parent_id` | `id` | ✔ id.level + id.ordinal |
/// | `parent`/`attached_dir`/子元素全部 | `sub_moves`（递归） | ✔ 递归发射所有 sub_moves |
///
/// ★旧浅指纹漏洞（644 否定）：只发射顶层 `(level, ordinal, end_index, sub_moves.len())`——不递归
/// `sub_moves`、不含 `start_index`、不含方向。interior（非首非尾）子走势被古怪线段重划（坐标/方向变）
/// 而顶层 start/end/ordinal/子数不变 ⟹ 旧指纹相同但 extract_elements 输出发散 ⟹ 缓存返陈旧树
/// （污染 ΔSharpe，bar 1464 类）。递归发射后任何深层改写都改变指纹。
///
/// soundness 依据 §16：confirmed move 跨 bar 身份稳定不重排（TowerCache 复用同一 Vec），只有
/// frontier（最后 top move）可变。复杂度：O(tree)（递归一遍）——与 extract_elements 同阶，per-bar
/// 本就遍历 tree，不引入更差阶。
///
/// `dir`/`lo`/`hi` 用 [`coverage::extract_elements`] 的 `rmove_side` 同源外缘判据所依赖的原始坐标
/// （`RMove::lo`/`RMove::hi` 递归取 leaf min/max + Segment.direction），故指纹随任何影响 eps 的 leaf
/// 坐标/方向改写而变。
#[derive(Clone, PartialEq, Eq, Default, Debug)]
pub struct TreeKey(Vec<(u32, u32, u64, usize, usize, u8, i64, i64, usize)>);

impl TreeKey {
    fn of(tower: &[Rc<Vec<LeveledMove>>]) -> TreeKey {
        for lvl in tower.iter().rev() {
            if !lvl.is_empty() {
                let mut fp = Vec::new();
                for m in lvl.iter() {
                    TreeKey::emit(m, &mut fp);
                }
                return TreeKey(fp);
            }
        }
        TreeKey(Vec::new())
    }

    /// 递归发射一个 move 及其全部 sub_moves 的深层指纹（前序遍历，父在子前——与
    /// [`coverage::push_element_tree`] 同序，确保结构同构的两树发射序一致）。
    fn emit(m: &LeveledMove, fp: &mut Vec<(u32, u32, u64, usize, usize, u8, i64, i64, usize)>) {
        // dir：与 rmove_side 同口径——Segment.direction / Compose 外缘 first.hi vs last.hi（0=Up,1=Down）。
        let dir: u8 = match &m.rmove {
            super::super::classifier::descend::RMove::Segment { direction, .. } => {
                match direction {
                    super::super::types::Direction::Up => 0,
                    super::super::types::Direction::Down => 1,
                }
            }
            super::super::classifier::descend::RMove::Compose { subs, .. } => {
                match (subs.first(), subs.last()) {
                    (Some(f), Some(l)) if l.hi() >= f.hi() => 0,
                    (Some(_), Some(_)) => 1,
                    _ => 0,
                }
            }
        };
        fp.push((
            // ★codex审O补漏(644元模式第4次): extract_elements 的 level 输出=rmove.level()，
            // 旧指纹只发 id.level 作代理(靠构造纪律 id.level==rmove.level()，非指纹强制)。
            // 发射真实输出驱动量 rmove.level()，使指纹自包含(不替换 id.level——后者仍是 id 字段)。
            m.rmove.level(),
            m.id.level,
            m.id.ordinal,
            m.start_index,
            m.end_index,
            dir,
            m.rmove.lo(),
            m.rmove.hi(),
            m.sub_moves.len(),
        ));
        for sub in &m.sub_moves {
            TreeKey::emit(sub, fp);
        }
    }
}

/// **工位 K 性能：tree-prefix（[`coverage::extract_elements`] 输出）缓存**。
///
/// `extract_elements(tower)` 是 `tower` 的纯函数，O(confirmed tree)。runner per-bar 在两处调（一处
/// `pi_theta_step`、一处 merge），且跨 bar confirmed prefix 不变（§16）⟹ 每 bar 重建 = O(confirmed)×n
/// = O(n²)。缓存：键 [`TreeKey`] 命中 ⟹ 复用 `tree`（O(top-level count) 键比较，O(tree) clone 仅在
/// append 候选时 caller 做——candidates 才需 mutate 尾部）；未命中（CL 16K 仅 26 次）⟹ 重建。
#[derive(Default)]
pub struct TreeCache {
    key: TreeKey,
    /// ★热点② O(n²) 消除：`Rc` 共享缓存树（命中返 `Rc::clone` O(1)，旧 `Vec` clone O(tree)/bar=O(n²)）。
    /// candidate 不再 append 进树 clone，由消费者 [`coverage::ElementView`] overlay 承载（双段视图）。
    tree: Rc<Vec<CoverageElement>>,
    /// ★热点③ O(n²) 消除（工位 4c）：tree 派生的两个索引（`(level,ρ)→idx` 端点表 + `(parent,level)→idx`
    /// 兄弟表）也是 `tree` 的纯函数，§16 tree 前缀不变 ⟹ 索引不变 ⟹ 同缓存键复用。命中返 `Rc::clone`
    /// O(1)，旧每 bar `build_*_index(&tree)` O(tree)/bar=O(n²)。两表只读（candidate 段兄弟由消费者
    /// overlay 单独承载，[`coverage::operation_role_indexed_split`]），缓存不被 mutate。
    endpoint_idx: Rc<std::collections::HashMap<(u32, usize), usize>>,
    sibling_idx: Rc<std::collections::HashMap<(Option<usize>, u32), Vec<usize>>>,
    /// ★工位 4d 热点② O(n²) 消除：tree 前缀 `ElementId → idx` 索引——也是 `tree` 的纯函数，§16 前缀
    /// 不变 ⟹ 索引不变 ⟹ 同缓存键复用。命中返 `Rc::clone` O(1)，旧 [`coverage::coverage_step_from_buckets`]
    /// 每 bar `build_tree_id_index(tree_prefix)` O(tree)/bar=O(n²)。只读（held leg 对位查表，缓存不被 mutate）。
    id_idx: Rc<std::collections::HashMap<ElementId, usize>>,
    valid: bool,
}

impl TreeCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// ★工位 4d：取回当前缓存的 tree 派生索引（`Rc::clone` O(1)）供 [`coverage::ElementView`] 注入。
    /// 调用前必须先 [`coverage_elements_and_gamma_with_tower_cached`] 命中/建过同 tower（缓存已填）；
    /// `valid=false`（未建过）⟹ 返 `None`，消费者 fallback 现建（bit-exact）。
    pub(crate) fn tree_sibling_and_id(
        &self,
    ) -> Option<(
        Rc<std::collections::HashMap<(Option<usize>, u32), Vec<usize>>>,
        Rc<std::collections::HashMap<ElementId, usize>>,
    )> {
        if self.valid {
            Some((Rc::clone(&self.sibling_idx), Rc::clone(&self.id_idx)))
        } else {
            None
        }
    }
}

/// [`coverage_elements_and_gamma_with_tower`] 带可选 tree-prefix 缓存（工位 K 性能）。
///
/// bit-exact == 无缓存版：命中复用的 `tree` 与 `extract_elements(tower)` 逐字节相等（§16 + 实测 0 假命中）。
/// 候选段每次按当前 `classification` 重建 append（候选随 bar 变；§16 只保证 tree-prefix 不变）。
pub fn coverage_elements_and_gamma_with_tower_cached(
    classification: &Classification,
    tower: &[Rc<Vec<LeveledMove>>],
    cache: &mut Option<&mut TreeCache>,
) -> (Rc<Vec<CoverageElement>>, Vec<CoverageElement>, Vec<Candidate>) {
    // ★热点②③ O(n²) 消除：树前缀 + 两个派生索引 `Rc` 共享（命中返 `Rc::clone` O(1)，旧每 bar
    // `extract_elements` + `build_*_index` 全是 O(tree)/bar=O(n²)）。candidate 段不进树/索引 clone，
    // 单独 `candidates` Vec + candidate-only 兄弟 overlay 承载（消费者 ElementView + split 查询双段组装）。
    let (tree, tree_endpoint_idx, tree_sibling_idx): (
        Rc<Vec<CoverageElement>>,
        Rc<std::collections::HashMap<(u32, usize), usize>>,
        Rc<std::collections::HashMap<(Option<usize>, u32), Vec<usize>>>,
    ) = match cache {
        Some(c) => {
            let key = TreeKey::of(tower);
            if c.valid && c.key == key {
                // bit-exact 守卫（debug/test）：缓存命中必与全量 extract_elements 逐字节相等（§16 + 实测 0 假命中）。
                debug_assert_eq!(c.tree.as_ref(), &coverage::extract_elements(tower),
                    "TreeCache 假命中——§16 不变量破裂或 TreeKey 不 sound");
                (Rc::clone(&c.tree), Rc::clone(&c.endpoint_idx), Rc::clone(&c.sibling_idx)) // 三者 O(1)
            } else {
                let t = Rc::new(coverage::extract_elements(tower));
                c.endpoint_idx = Rc::new(coverage::build_tree_endpoint_index(&t));
                c.sibling_idx = Rc::new(coverage::build_prev_sibling_index(&t));
                // ★工位 4d 热点②：tree 前缀 ElementId→idx 一并缓存（held leg 对位查表，§16 不变复用）。
                c.id_idx = Rc::new(coverage::build_tree_id_index(&t));
                c.tree = t;
                c.key = key;
                c.valid = true;
                (Rc::clone(&c.tree), Rc::clone(&c.endpoint_idx), Rc::clone(&c.sibling_idx))
            }
        }
        None => {
            let t = Rc::new(coverage::extract_elements(tower));
            let ep = Rc::new(coverage::build_tree_endpoint_index(&t));
            let sb = Rc::new(coverage::build_prev_sibling_index(&t));
            (t, ep, sb)
        }
    };
    let candidate_start = tree.len();

    // ── 遍历1：构建 candidate 段（parent/id/parent_id 只读 tree 前缀，无需 candidate 段连续）。 ──
    // candidate-only 兄弟 overlay（不 mutate 缓存的 tree_sibling_idx）：候选全局 idx 按 (parent,level)
    // 分组、push 序升序。遍历2 split 查询先查此 overlay 再 fallback tree base（bit-exact，见 split 函数）。
    let mut cand_sibling_idx: std::collections::HashMap<(Option<usize>, u32), Vec<usize>> =
        std::collections::HashMap::new();
    let mut candidates: Vec<CoverageElement> = Vec::new();
    let mut gamma: Vec<Candidate> = Vec::new();
    let mut ci = candidate_start;
    for (level_idx, level) in classification.levels.iter().enumerate() {
        let lvl = level_idx as u32;
        for point in &level.bsp {
            let dir = candidate_dir(&point.bits);
            // hostOf 查表只读 tree 前缀（候选 append 期间不变）。
            let (parent, attached_dir, carrier_id) = coverage::attach_bsp_carrier_indexed(
                &tree_endpoint_idx,
                &tree,
                lvl,
                point.source_index,
            );
            candidates.push(CoverageElement {
                lambda: point.source_index,
                rho: point.source_index,
                eps: match dir {
                    VoiceSide::Flat => VoiceSide::Long,
                    d => d,
                },
                level: lvl,
                parent,
                attached_dir,
                // ★工位 H（级别容器.pdf §13/§14）：carrier 容器 hostOf(g) id（非叶子 ordinal）。
                // 退化：carrier_id=None ⟹ 叶子 ordinal id（边界胚元 ∂）。同 carrier 同 bar 多 bsp 共享 id（§14 简化）。
                id: carrier_id.unwrap_or(ElementId { level: lvl, ordinal: ci as u64 }),
                // parent_id = carrier 父容器 id（par_C(κ(u))，§9.1）；parent 指 tree 前缀 idx，读 tree。
                parent_id: parent.and_then(|pidx| tree.get(pidx).map(|e: &CoverageElement| e.id)),
            });
            // candidate-only overlay：当前候选全局 idx 追加（升序，二分查 < ci 的最大 idx）。
            cand_sibling_idx.entry((parent, lvl)).or_default().push(ci);
            ci += 1;
        }
    }

    // ── 遍历2：算 role（用完整 ElementView{base=tree, overlay=candidates}——前兄弟可能在 tree 或
    //    candidate 段，需双段连续访问；零拷贝借 tree_rc + candidates）。bit-exact == 旧单遍：
    //    operation_role_indexed_split 双段兄弟查询 == 旧合并 sibling_idx 的 partition_point。 ──
    let view = coverage::ElementView::from_parts(&tree, candidates);
    let mut ci = candidate_start;
    for (level_idx, level) in classification.levels.iter().enumerate() {
        let lvl = level_idx as u32;
        for point in &level.bsp {
            let dir = candidate_dir(&point.bits);
            gamma.push(Candidate {
                level: lvl,
                source_index: point.source_index,
                bits: point.bits,
                dir,
                bsp_class: min_class(&point.bits, dir),
                role: coverage::operation_role_indexed_split(
                    &view, ci, &tree_sibling_idx, &cand_sibling_idx,
                ),
                nest_confirmed: nest_confirm(lvl, point.source_index, &point.bits, dir),
                gamma_index: gamma.len(),
            });
            ci += 1;
        }
    }
    // 取回 candidates（view drop，tree_rc 仍 Rc owned 返回）。
    let candidates = view.into_overlay();
    (tree, candidates, gamma)
}
// ponytail: ceiling = 跨 bar 复用 extract_elements 前缀。tower.levels[k].upper_moves 前缀不可变
// （TowerCache 不变量），但 extract_elements 从最高非空级建根——当更高级新出现时根结构重构
// （旧根变新根的 sub_moves），元素索引重映射，前缀不可简单 append 复用。需检测"最高级是否变化"
// + 索引重映射表，bit-exact 风险高，超出 ponytail ultra 最小 diff 范围。当前每 bar 单次建树
// 已消除双调冗余（H2 主要收益），跨 bar 前缀复用留后续工位。

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

    // ponytail: H8 预索引——level → legs idx 列表（reverse_signal 需逐腿判 bits，无法纯 key 查表；
    // 但 level 索引把 O(|working|) 全扫缩为只遍历同 level 的腿，通常 1-2 条）。
    // bit-exact：索引只过滤同 level 候选腿，逐腿判 !closed + reverse_signal/leg.dir 与旧线性
    // position/any 等价。active 不变 ⟹ 索引建一次（关闭只标 bool，不从索引移除——旧 position 跳过 closed）。
    let level_idx: std::collections::HashMap<u32, Vec<usize>> = {
        let mut m: std::collections::HashMap<u32, Vec<usize>> = std::collections::HashMap::new();
        for (i, (leg, _)) in working.iter().enumerate() {
            m.entry(leg.level).or_default().push(i);
        }
        m
    };

    for &c in &ordered {
        // 规则1：非方向候选 ⟹ 𝒦_x。
        if c.dir == VoiceSide::Flat || c.bsp_class == u8::MAX {
            buckets.record.push(*c);
            continue;
        }
        // 规则2：反向关闭 A_t 中同级别活动腿（reverse_signal 复用 §9 closePred 反向项）。
        // ponytail: H8 查 level→idx 列表，逐腿判 reverse_signal（bits 不可 key 化，须逐腿）。
        // bit-exact：取首个未关闭且 reverse_signal 命中者 == 旧 working.iter().position(...)。
        let closed_pos = level_idx.get(&c.level).and_then(|idxs| {
            idxs.iter().copied().find(|&i| {
                let (leg, closed) = &working[i];
                !*closed && reverse_signal(leg.dir, &c.bits)
            })
        });
        if let Some(pos) = closed_pos {
            // 标记关闭（不从索引移除——bit-exact：旧 working.iter().position 也跳过已关闭腿）。
            working[pos].1 = true;
            buckets.close.push(working[pos].0);
            continue;
        }
        // 规则3/4：开启 vs 记录（slot = (level, σ_g)）。
        // ponytail: H8 slot_in_at 用 level 索引查同 level 腿里是否有未关闭且 dir==c.dir 者
        // == 旧 working.iter().any(|(leg,closed)| !closed && leg.level==c.level && leg.dir==c.dir)。
        let slot_in_at = level_idx
            .get(&c.level)
            .map_or(false, |idxs| {
                idxs.iter().any(|&i| !working[i].1 && working[i].0.dir == c.dir)
            });
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
        let active = [aleg(0, VoiceSide::Long, 0, 0 )];
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
        let active = [aleg(0, VoiceSide::Long, 0, 0 )];
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
        let active = [aleg(1, VoiceSide::Long, 0, 0 )];
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
        let active = [aleg(0, VoiceSide::Long, 0, 0 )];
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

    /// 测试用 ElementId。
    fn eid(level: u32, ordinal: u64) -> ElementId {
        ElementId { level, ordinal }
    }

    /// ★O(n) 重构测试适配：字面量塔逐级包 Rc（生产塔 = Vec<Rc<Vec<LeveledMove>>>）。
    fn rc_tower(levels: Vec<Vec<LM>>) -> Vec<Rc<Vec<LM>>> {
        levels.into_iter().map(Rc::new).collect()
    }

    /// 测试用 ActiveLeg 构造器（默认 is_boundary_root=true 真边界根 ∂，parent_id=None）。
    fn aleg(level: u32, dir: VoiceSide, source_index: usize, lambda: usize) -> ActiveLeg {
        ActiveLeg {
            level,
            dir,
            source_index,
            lambda,
            id: ElementId { level, ordinal: source_index as u64 },
            parent_id: None,
            is_boundary_root: true,
            op_parent: None,
        }
    }

    /// 单父真嵌套塔：L1 走势（外缘 Long）+ 3 个 L0 子（ρ=4/8/12，真父=L1，attached=Long）。
    fn long_parent_tower() -> Vec<Rc<Vec<LM>>> {
        let s0 = LM::from_unit(&unit_r(0, 4, Direction::Up, 0, 10), eid(0, 0));
        let s1 = LM::from_unit(&unit_r(4, 8, Direction::Down, 3, 12), eid(0, 1));
        let s2 = LM::from_unit(&unit_r(8, 12, Direction::Up, 5, 15), eid(0, 2));
        let c = Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 0, end_index: 12 };
        let l1 = LM::compose(&[s0, s1, s2], c, 1, eid(1, 0)); // 外缘 10→15 ⟹ Long（σ_p=+1）
        rc_tower(vec![Vec::new(), vec![l1]])
    }

    /// 两父塔：compose_a(Long) + compose_b(Short)，a2/b0 结构全等异 ρ（ordinal 身份测试）。
    fn two_parent_tower() -> Vec<Rc<Vec<LM>>> {
        let mk = |si, ei, d, lo, hi, ord| LM::from_unit(&unit_r(si, ei, d, lo, hi), eid(0, ord));
        let ca = LM::compose(
            &[mk(0, 4, Direction::Up, 0, 10, 0), mk(4, 8, Direction::Down, 3, 12, 1), mk(8, 12, Direction::Up, 5, 15, 2)],
            Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 0, end_index: 12 },
            1,
            eid(1, 0),
        ); // Long
        let cb = LM::compose(
            &[mk(12, 16, Direction::Up, 5, 15, 3), mk(16, 20, Direction::Down, 3, 12, 4), mk(20, 24, Direction::Down, 0, 8, 5)],
            Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 12, end_index: 24 },
            1,
            eid(1, 1),
        ); // Short
        rc_tower(vec![Vec::new(), vec![ca, cb]])
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
        let s0 = LM::from_unit(&unit_r(0, 4, Direction::Up, 0, 10), eid(0, 0));
        let s1 = LM::from_unit(&unit_r(4, 8, Direction::Down, 3, 12), eid(0, 1));
        let tower = rc_tower(vec![vec![s0, s1]]); // len()==1 < 2
        let c = classification(vec![vec![buy_point(4, 1)]]);
        let with = assemble_gamma_with_tower(&c, &tower);
        let flat = assemble_gamma(&c);
        assert_eq!(with[0].role.v, Vertical::Ambient, "缺塔 ⟹ host 是根 ⟹ Ambient（诚实退化）");
        assert_eq!(with[0].role.v, flat[0].role.v, "缺塔退化与扁平 assemble_gamma 一致");
    }

    /// ★工位 H 不变量 I1/I3/I4（级别容器.pdf §11/§13）：开仓激活的位置节点身份 = **carrier 容器**
    /// hostOf(g) 的 ElementId，**不是**买卖点叶子的新 ordinal。
    ///
    /// - **I4**（买卖点激活 position node）：候选元素 id == carrier(=hostOf(g)) 的 id，非叶子 ordinal。
    /// - **I1**（买卖点不是持仓父节点）：候选不携带独立叶子身份，而是 carrier 上的位置实例。
    /// - **I3**（子声部 AncOK 检查父 carrier）：候选 parent_id == par_C(carrier)（carrier 的父容器 id）。
    ///
    /// long_parent_tower：sell_point(8) 的 host=s1（id=(0,1)，carrier），s1 的父 = L1 compose（id=(1,0)）。
    #[test]
    fn position_node_identity_is_carrier_not_leaf_ordinal() {
        let tower = long_parent_tower();
        let c = classification(vec![vec![sell_point(8, 1)]]);
        let (elements, cstart) = coverage_elements_with_tower(&c, &tower);
        // I4/I1：候选位置节点 id == carrier hostOf(g)=s1 的 id=(0,1)（非叶子 ordinal=cstart）。
        assert_eq!(
            elements[cstart].id,
            eid(0, 1),
            "I4：开仓激活的位置节点身份 = carrier hostOf(g)=s1 的 id（非买卖点叶子新 ordinal）"
        );
        // I3：候选 parent_id == carrier 的父容器 = L1 compose id=(1,0)（§9.1 par_C(κ)）。
        assert_eq!(
            elements[cstart].parent_id,
            Some(eid(1, 0)),
            "I3：子声部父 carrier = par_C(carrier) = L1 compose id（active position parent，非裸叶子）"
        );
        // I1 推论：carrier id 与树中 s1 走势同 id ⟹ 位置节点是 carrier 上的实例（跨 bar 按 carrier 对位）。
        let s1_tree_idx = elements[..cstart].iter().position(|e| e.id == eid(0, 1));
        assert!(
            s1_tree_idx.is_some(),
            "carrier s1 走势在树前缀中存在（位置节点身份 = 该 carrier，跨 bar 对位源）"
        );
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
        let s0 = LM::from_unit(&unit_r(0, 4, Direction::Up, 0, 10), eid(0, 0));
        let s1 = LM::from_unit(&unit_r(4, 8, Direction::Down, 3, 12), eid(0, 1));
        let no_parent_tower = rc_tower(vec![vec![s0, s1]]); // len()==1：host 是根 segment，无 Compose 父 = ∂
        let c = classification(vec![vec![sell_point(8, 1)]]);
        let gamma = assemble_gamma_with_tower(&c, &no_parent_tower);
        assert_eq!(
            gamma[0].role.v,
            Vertical::Ambient,
            "父=胚元∂（无有向父容器）⟹ Ambient（639：充要条件是父=∂，非未持仓）"
        );
    }

    // ════════════════════════════════════════════════════════════════════════
    //  工位 O：TreeCache 递归指纹（644 第3次有损指纹否定）
    // ════════════════════════════════════════════════════════════════════════

    /// **L1 反例（codex 构造）**：两个 tower 顶层指纹 `(id.level, id.ordinal, end_index,
    /// sub_moves.len())` 完全相同，但 interior（非首非尾）子走势被古怪线段重划——坐标/方向变，
    /// 而顶层 start/end/ordinal/子数全不变。旧浅指纹 ⟹ TreeKey 相同（漏，缓存返陈旧树）；
    /// 递归指纹 ⟹ TreeKey 不同（深层改写被捕获）。同时断言 extract_elements 输出确实发散
    /// （证明这不是伪反例——深层改写真改输出）。
    #[test]
    fn treekey_recursive_catches_interior_rewrite() {
        // tower A：interior 子走势 s1 = Down [3,12]（end_index=8）。
        let a_s0 = LM::from_unit(&unit_r(0, 4, Direction::Up, 0, 10), eid(0, 0));
        let a_s1 = LM::from_unit(&unit_r(4, 8, Direction::Down, 3, 12), eid(0, 1));
        let a_s2 = LM::from_unit(&unit_r(8, 12, Direction::Up, 5, 15), eid(0, 2));
        let ca = Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 0, end_index: 12 };
        let a_l1 = LM::compose(&[a_s0, a_s1, a_s2], ca, 1, eid(1, 0));
        let tower_a = rc_tower(vec![Vec::new(), vec![a_l1]]);

        // tower B：interior s1 古怪线段重划——同 start/end_index（4/8）、同 ordinal、同子数，
        // 但方向 Up（≠Down）+ 坐标 [99,199]（≠[3,12]）。顶层 end_index=12/ordinal=0/子数=3 不变。
        let b_s0 = LM::from_unit(&unit_r(0, 4, Direction::Up, 0, 10), eid(0, 0));
        let b_s1 = LM::from_unit(&unit_r(4, 8, Direction::Up, 99, 199), eid(0, 1)); // 重划！
        let b_s2 = LM::from_unit(&unit_r(8, 12, Direction::Up, 5, 15), eid(0, 2));
        let cb = Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 0, end_index: 12 };
        let b_l1 = LM::compose(&[b_s0, b_s1, b_s2], cb, 1, eid(1, 0));
        let tower_b = rc_tower(vec![Vec::new(), vec![b_l1]]);

        // 前提坐实：旧浅指纹（顶层 level/ordinal/end_index/子数）两 tower 相同（漏的来源）。
        let shallow = |t: &[Rc<Vec<LM>>]| -> Vec<(u32, u64, usize, usize)> {
            t.iter().rev().find(|l| !l.is_empty()).map_or(Vec::new(), |l| {
                l.iter().map(|m| (m.id.level, m.id.ordinal, m.end_index, m.sub_moves.len())).collect()
            })
        };
        assert_eq!(shallow(&tower_a), shallow(&tower_b),
            "前提：旧浅指纹两 tower 相同（这正是漏洞——浅指纹不区分 interior 重划）");

        // 反例坐实：extract_elements 输出确实发散（深层改写真改输出，非伪反例）。
        let ea = coverage::extract_elements(&tower_a);
        let eb = coverage::extract_elements(&tower_b);
        assert_ne!(ea, eb,
            "interior 重划改变 extract_elements 输出（s1 的 eps/lambda/rho 变）——缓存返陈旧树会污染 ΔSharpe");

        // 修复坐实：递归指纹捕获深层变化 ⟹ TreeKey 不同 ⟹ 缓存不会假命中。
        assert_ne!(TreeKey::of(&tower_a), TreeKey::of(&tower_b),
            "递归发射指纹捕获 interior 子走势的坐标/方向改写（644：第3次有损指纹否定）");
    }

    /// **L2 逐 bar 对拍**（真实 CL 数据，cached vs nocache extract_elements 全字段一致）。
    /// codex 指出当前 0 假命中无回归守卫、debug_assert 仅 debug 生效——本测试在 release 下逐 bar
    /// 对比 `coverage_elements_and_gamma_with_tower_cached`（带 TreeCache）与无缓存版的 elements
    /// 输出，任何缓存假命中（陈旧树）立即被 assert_eq 捕获。
    #[test]
    #[ignore = "工位 O L2：真实 CL 逐 bar cached-vs-nocache 对拍；需 CL；--release --ignored"]
    fn bit_exact_per_bar_cached_vs_nocache() {
        use super::super::super::backtest::data;
        use super::super::super::{classifier, parser};
        use super::super::super::config::ThetaConfig;
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let n = 2000.min(oos.bars.len());
        let mut cache = TreeCache::new();
        let mut hits = 0usize;
        for i in 0..n {
            let l0 = parser::parse_layer(&oos.bars[..=i], &config);
            let (cls, tower) = classifier::classify_with_tower(&l0, &config);
            // cached 路径（带 TreeCache，跨 bar 复用 Rc 树前缀）。
            let (cached_tree, _candidates, _) =
                coverage_elements_and_gamma_with_tower_cached(&cls, &tower, &mut Some(&mut cache));
            // nocache 路径（每 bar 全量 extract_elements）。
            let nocache_elems = coverage::extract_elements(&tower);
            assert_eq!(cached_tree[..], nocache_elems[..],
                "bar {i}：cached Rc 树 ≠ nocache（TreeCache 假命中——陈旧树）");
            if cache.valid { hits += 1; }
        }
        eprintln!("bit_exact_per_bar：{n} bars 全部 cached==nocache，{hits} bars 缓存有效");
    }
}
