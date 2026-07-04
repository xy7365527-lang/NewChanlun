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
use super::super::classifier::bsp::BspPoint;
use super::super::classifier::divergence::ForceProxies;
use super::super::types::{BspBits, Side};
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
/// - `force`：β^div A/C 段力度 proxy 对（A6 #159 透传，源=[`BspPoint::force`] 单一来源）。
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
    /// ★A6（#159）：β^div A/C 段力度 proxy 对，classifier→strategy 透传（源=[`BspPoint::force`]，
    /// 一类趋势背驰候选 `Some`，二/三类/无 A/C 对 `None`——组装层纯透传，不改值不兜底）。
    /// **不进** [`theta_key`]/排序/任何相等比较——唯一消费点是 z 装配
    /// （`selector::z_of_candidate` 调 [`ForceProxies::force_state`] 填 `MuClass.force_state` 第 8 维）。
    pub force: Option<ForceProxies>,
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

/// 正规出场类型 `Exit_Θ`（《完整的策略.pdf》§9 typed exit）——close 桶的类型化出场理由。
///
/// `Exit_Θ(v,x_t) ∈ {CloseRoot, ReduceCore, CloseShortDiff, RiskExit, Hold}`。这是 G4（统计层
/// typed exit：生产 π loop 输出 `TypedTradeLedger.exit_type`）与 G5（解释器 P5/P6/P7/P1 typed
/// close 拆分）的**单一来源**枚举（team-lead 2026-07-03 裁定：由 interp.rs 定义，G4 工位复用），
/// 避免两权威镜像（codex-q2-d1 删 `closed_loop/mutex_interp.rs` 同款矛盾）。P1..P10↔ExitType
/// 映射见 `.chanlun/review-results/g5-interpreter-mapping-20260703.md` §6.1。
///
/// **接线状态**：G4（#134）已在统计层接线——[`reverse_exit_type`] 单源判据 + 生产 π fill loop
/// 的 `TypedTradeLedger`（runner.rs）消费本枚举；interp close 桶本体仍单一未 typed（生产订单流
/// bit-exact 不变），P5/P6/P7 生产拆分在 G5 实装阶段（#124，须复用 [`reverse_exit_type`]）。
/// `closed_loop/sell.rs::SellDecision` 已有 CloseRoot/ReduceCore 重叠（disjoint 路径，G4 把 μ 管线
/// 重接生产 π 后该路径废）——统一收敛到本枚举，届时删 SellDecision 侧（升级路径，非现在做）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitType {
    /// P5 CloseRoot：关 depth-0 根腿（一类反向点=根清仓）。
    CloseRoot,
    /// P6 ReduceCore：减核心仓（三类反向点=核心仓减仓）。
    ReduceCore,
    /// P7 CloseShortDiff：关 ShortDiff carrier 子声部（短差反向确认）。
    CloseShortDiff,
    /// P1 RiskExit：保证金/强平（`KThetaRiskGate.force_flat`/stop）。
    RiskExit,
    /// P0 Hold：无出场。
    Hold,
}

/// 反向关闭的 typed exit 判据（PDF §9 / G5 映射 §6.1，**G4/G5 单源**——统计层 ledger 与
/// 生产 P5/P6/P7 拆分必须共用本函数，不得镜像）。
///
/// 输入 = 被关腿的**入场角色垂直轴** `entry_v`（腿声部身份在入场时固定，不随塔演化漂移）+
/// 触发关闭的反向候选的 `bsp_class`（[`interpret`] 规则2 的触发者，最小成立类 1<2<3）：
/// - `entry_v == ShortDiff` ⟹ [`ExitType::CloseShortDiff`]（P7：短差子声部反向确认关闭，
///   优先于触发类判定——子声部关闭语义压过触发信号语义）。
/// - 否则 `trigger_class == 3` ⟹ [`ExitType::ReduceCore`]（P6：三类反向点=核心仓减仓）。
/// - 否则 ⟹ [`ExitType::CloseRoot`]（P5：一类反向点=根清仓；**二类反向归 CloseRoot**——
///   PDF §9 五枚举无二类单列，二类是一类的次级确认，同属根反转语义；三类才是中枢离开
///   确认=减仓语义。此读法已向 ws-g5interp 征求意见，翻转条件见 g4-impl 结果包边界条件）。
///
/// `FollowParent` 子腿被反向关闭按触发类走 P5/P6（跟随父方向的级联核心仓，非短差对冲腿）。
pub fn reverse_exit_type(entry_v: Vertical, trigger_class: u8) -> ExitType {
    if entry_v == Vertical::ShortDiff {
        ExitType::CloseShortDiff
    } else if trigger_class == 3 {
        ExitType::ReduceCore
    } else {
        ExitType::CloseRoot
    }
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
    let mut raw: Vec<(u32, usize, BspBits, VoiceSide, u8, bool, Option<ForceProxies>)> = Vec::new();
    for (level_idx, level) in classification.levels.iter().enumerate() {
        let lvl = level_idx as u32;
        for point in level.bsp.iter() {
            let dir = candidate_dir(point);
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
            raw.push((lvl, point.source_index, point.bits, dir, cls, nest_ok, point.force));
        }
    }
    raw.iter()
        .enumerate()
        .map(|(i, &(level, source_index, bits, dir, bsp_class, nest_confirmed, force))| Candidate {
            level,
            source_index,
            bits,
            dir,
            bsp_class,
            role: coverage::operation_role(&elements, i),
            nest_confirmed,
            gamma_index: i,
            force,
        })
        .collect()
}

/// 候选方向 σ_g（root_sel 镜像反对称消歧）：买侧→Long、卖侧→Short、双侧/空→Flat（不可交易）。
///
/// ★P2-R2 回退（p2-plan §2 + codex-review-20260701-2251 **护栏1 [致命]**）：当**且仅当**六 bit
/// 完全无方向（`!conf_plus() && !conf_minus()`——严格零 bit，**不是** `root_sel(...)==Flat`）且
/// `struct_break_dir=Some(s)` 时，用破中枢结构方向 s 恢复方向。这让零 bit 破中枢候选（MACD C≥A
/// 未背驰但几何破了最后中枢）进 μ 样本（消选择偏差），不再被 `dir==Flat` 预删。
///
/// ★护栏1 严格性（**为什么禁用 `root_sel(...)==Flat`**）：`root_sel` 对 `(1,1)` 双触发（buy 位
/// 与 sell 位同时置，非互斥可重合）也返回 `Flat`（镜像不动点反对称消歧，voice.rs:270）。若用
/// `root_sel(...)==Flat` 作回退条件，则**双触发冲突候选**（有六 bit 方向但被消歧为 Flat）会被
/// struct_break_dir 误改向——那是错的（冲突候选应保持 Flat 归 𝒦 记录，不该被结构方向覆盖）。
/// 严格零 bit `!conf_plus() && !conf_minus()` 只匹配 `(0,0)` 真无方向，排除 `(1,1)` 冲突。
///
/// bit-exact：`struct_break_dir` 不进 `class_index()`/`MuClass`/桶 key（只在本消歧层读）——
/// 有六 bit 方向的候选（`conf_plus() || conf_minus()`）走原 `root_sel` 路径，dir 逐字段不变。
fn candidate_dir(point: &BspPoint) -> VoiceSide {
    let bits = &point.bits;
    let base = root_sel(RootCandidates {
        long_trigger: bits.conf_plus(),
        short_trigger: bits.conf_minus(),
    });
    // 严格零 bit（(0,0)，排除 (1,1) 双触发冲突）+ 破中枢结构方向 ⟹ 恢复方向（护栏1）。
    if !bits.conf_plus() && !bits.conf_minus() {
        if let Some(side) = point.struct_break_dir {
            return match side {
                Side::Long => VoiceSide::Long,
                Side::Short => VoiceSide::Short,
            };
        }
    }
    base
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

/// 从真嵌套塔 + 638 附着规则构造**组合元素数组**（操作 carrier forest ++ 附着的买卖点候选元素）。
///
/// 这是塔导出桥 (ii) **喂入段**的核心：替换 [`assemble_gamma`] 的扁平全根（独立根 ∂、V 恒 Ambient）。
/// 两段：
/// 1. ★A12（648 裁决 D）：`tree = extract_carrier_forest(tower)`：**K_i 操作 carrier forest**
///    （endpoint-complete，host^op 定义域）——所有级别全部走势真嵌套展开+dedup（`RMove::Compose`
///    真父子，547 铁律守护——父只来自 `sub_moves` 真包含，非级别差伪造）。旧宇宙 T_i
///    （`extract_elements`，只最高级根，覆盖 ~36-42% L0）是 host^op=host^struct 混用（676/648
///    第四根因：orphan frontier bsp 92-98% host-miss ⟹ 子声部结构性恒零）。T_i 仍由
///    `extract_elements` 承载（结构可视化/bit-exact extract 侧，648 保留项，本路径不再消费）。
/// 2. 每个 bsp 候选 → 一个 [`CoverageElement`]（host^op：[`coverage::attach_bsp_carrier_indexed`]
///    严格右端点命中 **K_i**，P2a 保留/P2b 宇宙切换），继承 hostOf 的真父 + 父方向 σ_{p(g)}。
///    候选元素追加在 `tree` 之后。
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
    /// 指纹长度（诊断用：≈tree 节点数，量化 `of` 的 O(tree) 工作量标度）。
    pub(crate) fn fp_len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn of(tower: &[Rc<Vec<LeveledMove>>]) -> TreeKey {
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

    /// ★A12 双视图（648 裁决 D）：**全级别**森林指纹——[`coverage::extract_carrier_forest`]
    /// （K_i 操作 carrier forest）的缓存键。
    ///
    /// 与 [`TreeKey::of`]（只指纹**最高非空级**，T_i=↓r_i 只依赖最高级根 ⟹ 够）的区别：K_i 遍历
    /// tower **所有级别**（K_i=U_i 全量 tower 元素），低级 `level_moves` 的变化（尤其 **L0 段尾部
    /// 古怪线段重划**——`TowerCache::generation` 维护点不覆盖，CandidateCache bar 3020 坐实
    /// 「gen 不变但 L0 内容变」）不改最高级指纹 ⟹ `of` 对 K_i 会**假命中返陈旧森林**。本函数
    /// 逐级发射所有 moves（级间以 level_moves 边界顺序天然分隔——emit 首字段 rmove.level() 已
    /// 区分级别，同级内前序遍历），覆盖 K_i 输出的全部决定字段 ⟹ 指纹相等 ⟹ K_i 输出逐字节相等。
    pub(crate) fn of_forest(tower: &[Rc<Vec<LeveledMove>>]) -> TreeKey {
        let mut fp = Vec::new();
        for lvl in tower.iter().rev() {
            for m in lvl.iter() {
                TreeKey::emit(m, &mut fp);
            }
        }
        TreeKey(fp)
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
        for sub in m.sub_moves.iter() {
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
    // ★A12：旧 4g `gen: Option<u64>` 代次快路字段已删——K_i（全塔含 L0）下 gen 命中 ≠ 森林不变
    // （L0 尾段古怪线段重划不 bump gen，bar 3020 坐实），唯一命中判据 = TreeKey::of_forest。
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

/// ★操作宇宙段缓存（[`coverage_elements_and_gamma_with_tower_cached_gen`] 与 PART1 gamma-free 路径
/// [`coverage_elements_with_tower_cached_gen`] **单一来源**，no-patch 不复制缓存逻辑）。
///
/// ★A12 双视图切换（648 裁决 D，settled）：生产操作路径的元素宇宙从 T_i（[`coverage::extract_elements`]
/// 结构视图树）切到 **K_i（[`coverage::extract_carrier_forest`] 操作 carrier forest，endpoint-complete）**
/// ——host^op 的定义域（PDF「子声部.pdf」§16 完整状态 x_i=(T_i,K_i,…)，解释器消费 Γ^K 与 K_i）。
/// host^op 仍严格右端点命中（P2a 保留，不违 638），只改 host 宇宙（P2b：T_i→K_i）。T_i/
/// `extract_elements` 本身**不动**（结构可视化/bit-exact extract/compose chain 侧，648 保留项）。
///
/// ★gen 快路移除（K_i soundness）：`TowerCache::generation` 维护点只覆盖 upper 级
/// cascade/extend/clear——**L0 段尾部古怪线段重划不 bump gen**（CandidateCache bar 3020 坐实）。
/// T_i 只读最高非空级 ⟹ gen 快路 sound；K_i 读全塔含 L0 ⟹ gen 命中 ≠ K_i 不变 ⟹ 假命中返陈旧
/// 森林。故 K_i 缓存唯一命中判据 = [`TreeKey::of_forest`]（全级别指纹，覆盖 K_i 输出全部决定字段）。
/// ponytail: 每 bar O(全塔) 指纹构造是已知 ceiling（4g 消除的热点部分回归）；升级路径 = 把 parser
/// `segments_confirmed_len` 证书传入 TreeCache 做 L0 增量指纹（前缀免重扫）。
fn tree_segment_cached_gen(
    classification: &Classification,
    tower: &[Rc<Vec<LeveledMove>>],
    cache: &mut Option<&mut TreeCache>,
) -> (
    Rc<Vec<CoverageElement>>,
    Rc<std::collections::HashMap<(u32, usize), usize>>,
    Rc<std::collections::HashMap<(Option<usize>, u32), Vec<usize>>>,
) {
    let _ = classification; // 操作宇宙段不消费 classification（candidate 段才用），保签名一致。
    match cache {
        Some(c) => {
            let key = TreeKey::of_forest(tower);
            if c.valid && c.key == key {
                debug_assert_eq!(c.tree.as_ref(), &coverage::extract_carrier_forest(tower),
                    "TreeCache 假命中——§16 不变量破裂或 TreeKey::of_forest 不 sound");
                (Rc::clone(&c.tree), Rc::clone(&c.endpoint_idx), Rc::clone(&c.sibling_idx))
            } else {
                let t = Rc::new(coverage::extract_carrier_forest(tower));
                // A12 双向映射一致性（debug 构建，仅重建分支——miss 稀少不进热路径）：
                // T_i↪K_i 嵌入保 (id,λ,ρ,ε,ℓ,parent_id) + K_i id/(level,ρ) 唯一。
                debug_assert!(
                    coverage::dual_view_consistency(&coverage::extract_elements(tower), &t).is_ok(),
                    "A12 双视图一致性破裂：{:?}",
                    coverage::dual_view_consistency(&coverage::extract_elements(tower), &t)
                );
                c.endpoint_idx = Rc::new(coverage::build_tree_endpoint_index(&t));
                c.sibling_idx = Rc::new(coverage::build_prev_sibling_index(&t));
                c.id_idx = Rc::new(coverage::build_tree_id_index(&t));
                c.tree = t;
                c.key = key;
                c.valid = true;
                (Rc::clone(&c.tree), Rc::clone(&c.endpoint_idx), Rc::clone(&c.sibling_idx))
            }
        }
        None => {
            let t = Rc::new(coverage::extract_carrier_forest(tower));
            let ep = Rc::new(coverage::build_tree_endpoint_index(&t));
            let sb = Rc::new(coverage::build_prev_sibling_index(&t));
            (t, ep, sb)
        }
    }
}

/// 单个 bsp → candidate `CoverageElement`（遍历1 核心，PART1 gamma-free 路径与全路径共享）。
///
/// 纯查表（`attach_bsp_carrier_indexed` 只读 tree 前缀 + endpoint_idx，O(1)）：bit-exact == 遍历1
/// line 560-583 内联。`ci` = 该 candidate 的全局 idx（fallback id 用）。
fn build_candidate_element(
    tree: &[CoverageElement],
    tree_endpoint_idx: &std::collections::HashMap<(u32, usize), usize>,
    lvl: u32,
    point: &BspPoint,
    ci: usize,
) -> CoverageElement {
    let dir = candidate_dir(point);
    let (parent, attached_dir, carrier_id) =
        coverage::attach_bsp_carrier_indexed(tree_endpoint_idx, tree, lvl, point.source_index);
    CoverageElement {
        lambda: point.source_index,
        rho: point.source_index,
        eps: match dir {
            VoiceSide::Flat => VoiceSide::Long,
            d => d,
        },
        level: lvl,
        parent,
        attached_dir,
        id: carrier_id.unwrap_or(ElementId { level: lvl, ordinal: ci as u64 }),
        parent_id: parent.and_then(|pidx| tree.get(pidx).map(|e: &CoverageElement| e.id)),
    }
}

/// ★工位 4h：candidate 段前缀缓存（源(b) O(n²) 真修）——caller B（merge，gamma 丢弃）专用。
///
/// ## 源(b)（#20 诊断 acceptance4-4g）
///
/// [`coverage_elements_and_gamma_with_tower_cached_gen`] 的 candidate 段每 bar 全量重建 `Vec<CoverageElement>`
/// （遍历 `classification.levels[*].bsp` 全部，bsp ∝ confirmed ∝ n^1.26 超线性 ⟹ Σ_bar O(bsp_at_bar) = O(n²)）。
/// CL 16K cand≈0.05s，BTC 1.31M 外推 ≈946s 爆。
///
/// ## 增量充要（codex 异质审 verdict，/tmp/codex_4h_prompt.md）
///
/// - **Q1 SOUND**：caller B 丢弃 gamma ⟹ 跳过遍历2（role/gamma 组装）bit-exact。`merge_in_place_split`
///   只读 candidate 字段 {id,eps,level,lambda,rho,parent_id}，不读 role/gamma。本路径**不建** cand_sibling_idx
///   （只为遍历2 role 服务）。
/// - **Q2 SOUND（身份）**：`gen` 不变 ⟹ `extract_elements(tower)` 输出不变 ⟹ 同一 bsp（同 level/source_index/
///   bits）的 `attach_bsp_carrier_indexed` 返回 (parent,attached_dir,carrier_id) + tree[parent].id 全不变
///   ⟹ candidate 前缀身份稳定。
/// - **Q3 SOUND（须前缀校验）**：memo 命中时 bsp 不变（clone）；重算时 confirmed 前缀不变（B2/S2 extend
///   必 bump generation）。无合法路径让 confirmed 前缀变/缩而 gen 不 bump。**但 gen 不变 ≠ 无新 bsp**
///   （L0 seg_len 变可增长 bsp 尾部而不 bump gen，codex Q2）⟹ 不能 gen-only 返旧全量，须按 per-level
///   bsp 前缀长度检测尾部增长 + 只 build 尾部。
/// - **Q4 诚实下界**：前缀校验用 per-level len 比较 O(levels)（≈7 常数），**非** naive 全 bsp 扫描
///   O(bsp)。命中 ⟹ candidate 构建 = O(Σ Δbsp) = O(总 bsp) amortized O(n)。
///
/// ## 命中两级（与 [`TreeCache`] 同构）
///
/// 1. **gen + 前缀快路**：`gen == tower_gen` ∧ 每级 `bsp.len() >= cached_lens[lvl]`（单调追加）⟹ 复用
///    cached candidates 前缀，只对每级 `bsp[cached_lens[lvl]..]` 尾部 `build_candidate_element` 追加。
/// 2. **miss fallback**：gen 不匹配 / 前缀回缩 / 级数变 ⟹ 全量重建（bit-exact 退化）。
#[derive(Default)]
pub struct CandidateCache {
    /// 缓存对应的塔代次（`TowerCache::generation()`）。`Some(g)` ⟹ candidates 由代次 g 的 tree 建得。
    gen: Option<u64>,
    /// ★per-level 分段存储（修正扁平 Vec 顺序 bug）：candidates 按 level 顺序拼接成扁平 Vec
    /// `[L0..., L1..., ...]`。命中时 L0 新增 bsp 必须插在 L0 段末尾（L1 之前），不是整个 Vec 末尾。
    /// 故按级分段存 `per_level[lvl]`，返回时按级拼接 ⟹ level 顺序 bit-exact。
    /// `per_level[lvl].len()` = 该级已 build 的 bsp 数（替代旧 cached_lens）。
    per_level: Vec<Vec<CoverageElement>>,
    /// ★codex Q3 前缀内容校验：每级已处理 bsp 的内容指纹（(source_index, bits 判别码)）。gen 不变但
    /// L0 bsp 内容变（古怪线段重划改 segments → extract_signals 重算，len 可能不变）时，len 校验漏检
    /// （bar 3020 坐实），须逐元素比对前缀身份。指纹比 full CoverageElement 轻（只 (usize, u8)）。
    prefix_fp: Vec<Vec<(usize, u8)>>,
    /// ★codex 实施审 QUESTION A/C UNSOUND 修复：每级**缓存时**的 base_ci（该级首元素全局 flat idx =
    /// candidate_start + Σ_{k<lvl} bsp_len[k]）。fallback id `ordinal=ci`（carrier miss 时）依赖全局
    /// flat ci，ci 依赖**所有前置级** bsp 数。前置级增长（如 L0 +1）⟹ 后置级 base_ci 偏移 ⟹ 后置级
    /// fallback-id 元素的 ordinal 失效（codex 反例：L0 8→9 时 L1 fallback ordinal 应 tree.len()+8→+9）。
    /// 命中时逐级比对 base_ci，变化则重建该级（保 fallback ordinal bit-exact）。
    base_ci: Vec<usize>,
    valid: bool,
}

/// bsp 内容指纹（codex Q3 前缀校验）：(source_index, bits 判别码)。candidate 的 lambda/rho/eps/level
/// 由 (source_index, bits) 决定（attach 查表给 parent/id，由 gen 保证稳定）⟹ 此指纹不变 ⟹ candidate 不变。
fn bsp_fingerprint(p: &BspPoint) -> (usize, u8) {
    let b = &p.bits;
    let disc = (b.buy1 as u8)
        | (b.buy2 as u8) << 1
        | (b.buy3 as u8) << 2
        | (b.sell1 as u8) << 3
        | (b.sell2 as u8) << 4
        | (b.sell3 as u8) << 5;
    (p.source_index, disc)
}

impl CandidateCache {
    pub fn new() -> Self {
        Self::default()
    }
}

/// ★工位 4h：caller B（merge）的 **gamma-free + candidate 前缀缓存** 路径（源(b) O(n²) 真修）。
///
/// 返回 `(tree_rc, candidates)`——**不产 gamma**（codex Q1：caller B 丢弃 gamma）。操作宇宙段（K_i，
/// A12）复用 [`coverage_elements_and_gamma_with_tower_cached_gen`] 的同一 [`TreeCache`]（of_forest
/// 指纹判据）；candidate 段用 [`CandidateCache`] 前缀缓存（gen + per-level bsp 指纹校验 ⟹ 只 build
/// 尾部）。
///
/// ★A12 CandidateCache 在 K_i 下的 soundness 补记：gen 命中 ⟹ upper 级不变；K_i 可因 L0 尾段
/// 重划重建（of_forest miss），但 (a) bsp 附着只命中 **confirmed 域**元素（bsp.source_index=确认段
/// 端点，同级坐标严格递增 ⟹ 与未确认尾段 (level,ρ) 键不重合），confirmed 域附着结果不变；
/// (b) K_i 遍历序=最高级→L0 + dedup 按原始 idx 升序重映射，upper 段与 L0 confirmed 前缀的去重后
/// 索引稳定（L0 内部前缀由 `segments_confirmed_len` 证书保证稳定），尾段变化只动数组尾部 ⟹
/// cached candidate 的 parent 索引/字段不漂移。bsp 内容变化已由 prefix_fp 逐元素校验兜底。
///
/// bit-exact == [`coverage_elements_and_gamma_with_tower_cached_gen`] 的 `(tree, candidates)`（丢 gamma）：
/// - 命中：cached candidates 前缀 == 全量遍历1 前缀（codex Q2 身份稳定）+ 尾部 `build_candidate_element`
///   == 全量遍历1 尾部（同 `attach_bsp_carrier_indexed` 路径）。
/// - miss：全量重建，逐元素 == 全量遍历1。
///
/// candidate 段 `id` fallback ordinal 用全局 `ci`——前缀复用时 ci 接续（cached candidates.len()），与全量
/// 同序（遍历顺序 level×bsp 不变）⟹ ci bit-exact。
pub fn coverage_elements_with_tower_cached_gen(
    classification: &Classification,
    tower: &[Rc<Vec<LeveledMove>>],
    tree_cache: &mut TreeCache,
    cand_cache: &mut CandidateCache,
    tower_gen: Option<u64>,
) -> (Rc<Vec<CoverageElement>>, Vec<CoverageElement>) {
    // ── 操作宇宙段（K_i，A12：of_forest 指纹命中——gen 快路对 K_i 不 sound 已删）。 ──
    let (tree, tree_endpoint_idx) = {
        let mut opt = Some(tree_cache);
        let (t, ep, _sb) = tree_segment_cached_gen(classification, tower, &mut opt);
        (t, ep)
    };
    let candidate_start = tree.len();

    // ── candidate 段（CandidateCache per-level 前缀复用，codex Q2/Q3）。 ──
    // ★关键不变量（ci 全局 idx）：candidates 扁平序 = [L0 全部, L1 全部, ...]（按 level 顺序拼接）。
    // ci = candidate_start + 已拼接元素数。命中路径 L_k 的尾部 ci 必须接 **L_k 段** 末尾（不是整个
    // Vec 末尾，否则 L0 新增插到 L1 之后乱序——bar 3020 bit-exact 失败坐实）。故按级分段存 per_level，
    // ci 由"前 k 级 bsp 总数 + 本级偏移"算（保 fallback id ordinal 全量 bit-exact）。
    let gen_match = matches!((tower_gen, cand_cache.gen), (Some(g), Some(cg)) if g == cg);
    let level_match = cand_cache.per_level.len() == classification.levels.len();
    let cache_usable = gen_match && cand_cache.valid && level_match;

    if !cache_usable {
        // miss：全量重建（bit-exact 退化）。per_level/fp/base_ci 清空重建。
        cand_cache.per_level.clear();
        cand_cache.prefix_fp.clear();
        cand_cache.base_ci.clear();
        for _ in 0..classification.levels.len() {
            cand_cache.per_level.push(Vec::new());
            cand_cache.prefix_fp.push(Vec::new());
            cand_cache.base_ci.push(0);
        }
        cand_cache.gen = tower_gen;
        cand_cache.valid = true;
    }

    // 逐级：base_ci 累积（= 该级首元素全局 ci = candidate_start + Σ_{k<lvl} bsp_len[k]）。命中三条件
    // （全满足 ⟹ 复用前缀只 build 尾部 O(Δbsp)；任一不满足 ⟹ 重建该级）：
    //   (1) base_ci 不变（codex A/C：前置级未增长 ⟹ fallback ordinal 不偏移）；
    //   (2) bsp 单调追加（len >= cached）；
    //   (3) 前缀指纹 bit-exact（codex Q3：gen 不变但 L0 bsp 内容变——古怪线段重划——须逐元素验身份）。
    let mut level_offset = candidate_start;
    for (level_idx, level) in classification.levels.iter().enumerate() {
        let lvl = level_idx as u32;
        let cached_base = cand_cache.base_ci[level_idx];
        let fp_len = cand_cache.prefix_fp[level_idx].len();
        let base_ok = cached_base == level_offset;
        let grow_ok = level.bsp.len() >= fp_len;
        let prefix_ok = grow_ok
            && level.bsp[..fp_len]
                .iter()
                .zip(cand_cache.prefix_fp[level_idx].iter())
                .all(|(p, &f)| bsp_fingerprint(p) == f);
        let level_hit = base_ok && prefix_ok;

        if !level_hit {
            // 重建该级（base_ci 偏移 / 前缀指纹不符）⟹ 清空该级，全量 build（fallback ordinal 用新 base）。
            cand_cache.per_level[level_idx].clear();
            cand_cache.prefix_fp[level_idx].clear();
        }
        let seg = &mut cand_cache.per_level[level_idx];
        let fp = &mut cand_cache.prefix_fp[level_idx];
        let cached = seg.len(); // level_hit ⟹ 前缀保留续 build 尾部；否则 0 ⟹ 全量 build。
        for (off, point) in level.bsp[cached..].iter().enumerate() {
            let ci = level_offset + cached + off;
            seg.push(build_candidate_element(&tree, &tree_endpoint_idx, lvl, point, ci));
            fp.push(bsp_fingerprint(point));
        }
        cand_cache.base_ci[level_idx] = level_offset;
        level_offset += level.bsp.len();
    }

    // 按级顺序拼接成扁平 candidates（[L0..., L1...]）供 merge 消费。
    // ponytail: 拼接 = O(总 bsp) memcpy（CoverageElement: Copy）。这是 merge 接口所需全量 Vec 的下界
    //           （codex Q4 诚实：candidate **构建** 摊还 O(Σ Δbsp)，但 merge 需每 bar 完整快照 ⟹ 输出
    //           拼接/clone 是 O(bsp)/bar，与 merge 内部 O(bsp)/bar 同阶。本工位有效域 = candidate 构建侧
    //           的重复 attach 消除；merge 侧 O(bsp)/bar 是独立残余，见结果包诚实声明）。
    let mut candidates: Vec<CoverageElement> = Vec::with_capacity(level_offset - candidate_start);
    for seg in &cand_cache.per_level {
        candidates.extend_from_slice(seg);
    }
    (tree, candidates)
}

/// [`coverage_elements_and_gamma_with_tower`] 带可选 forest-prefix 缓存（工位 K 性能）。
///
/// bit-exact == 无缓存版：命中复用的森林与 `extract_carrier_forest(tower)` 逐字节相等（§16 +
/// of_forest 全字段指纹）。候选段每次按当前 `classification` 重建 append（候选随 bar 变）。
pub fn coverage_elements_and_gamma_with_tower_cached(
    classification: &Classification,
    tower: &[Rc<Vec<LeveledMove>>],
    cache: &mut Option<&mut TreeCache>,
) -> (Rc<Vec<CoverageElement>>, Vec<CoverageElement>, Vec<Candidate>) {
    // 向后兼容入口（无 generation——合成闭包/全量路径走 TreeKey 比较，O(tree)/bar）。
    coverage_elements_and_gamma_with_tower_cached_gen(classification, tower, cache, None)
}

/// [`coverage_elements_and_gamma_with_tower_cached`] 带塔代次 `tower_gen`（签名保留，A12 后本路径
/// 不再消费——见下）。
///
/// ★A12（648 裁决 D）：操作宇宙段命中判据 = **[`TreeKey::of_forest`] 全级别指纹**（单级判据）。
/// 旧 4g gen 代次快路已删：`TowerCache::generation` 维护点不覆盖 L0 段尾部古怪线段重划
/// （CandidateCache bar 3020 坐实「gen 不变但 L0 内容变」），对 K_i（读全塔含 L0）会假命中返
/// 陈旧森林。`tower_gen` 参数保留避免调用方（runner/incremental）签名波纹；CandidateCache 路径
/// （[`coverage_elements_with_tower_cached_gen`]）仍消费 gen 作 candidate 段判据（soundness 见该函数）。
///
/// bit-exact == 无缓存版：命中复用的森林与 `extract_carrier_forest(tower)` 逐字节相等（debug_assert 守卫）。
pub fn coverage_elements_and_gamma_with_tower_cached_gen(
    classification: &Classification,
    tower: &[Rc<Vec<LeveledMove>>],
    cache: &mut Option<&mut TreeCache>,
    tower_gen: Option<u64>,
) -> (Rc<Vec<CoverageElement>>, Vec<CoverageElement>, Vec<Candidate>) {
    let _ = tower_gen; // A12：gen 快路对 K_i 不 sound 已删（见函数文档）；参数保留签名兼容。
    // ★热点②③ O(n²) 消除：树前缀 + 两个派生索引 `Rc` 共享（命中返 `Rc::clone` O(1)，旧每 bar
    // `extract_elements` + `build_*_index` 全是 O(tree)/bar=O(n²)）。candidate 段不进树/索引 clone，
    // 单独 `candidates` Vec + candidate-only 兄弟 overlay 承载（消费者 ElementView + split 查询双段组装）。
    // ★热点②③ O(n²) 消除：森林前缀 + 派生索引 Rc 共享。抽出 tree_segment_cached_gen 单一来源
    // （PART1 gamma-free 路径共享同逻辑，no-patch 不复制缓存）。A12：宇宙=K_i（tower_gen 只供
    // candidate 段 CandidateCache 消费，K_i 段判据=of_forest）。
    let (tree, tree_endpoint_idx, tree_sibling_idx) =
        tree_segment_cached_gen(classification, tower, cache);
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
        for point in level.bsp.iter() {
            // ★工位 4h：单一来源 build_candidate_element（与 PART1 gamma-free 路径共享，no-patch）。
            let e = build_candidate_element(&tree, &tree_endpoint_idx, lvl, point, ci);
            let parent = e.parent;
            candidates.push(e);
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
        for point in level.bsp.iter() {
            let dir = candidate_dir(point);
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
                force: point.force, // A6 #159：BspPoint.force 纯透传（一类 Some/其余 None）
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
pub(crate) fn theta_key(c: &Candidate) -> (Reverse<u32>, u8, usize, u8, u8, u8, u8, usize) {
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
    interpret_with_close_triggers(gamma, active).0
}

/// [`interpret`] 的**单源 fold 本体** + close 触发归因（G4 typed exit 组合层原料，#134）。
///
/// 返回 `(Buckets, Vec<Candidate>)`：第二分量与 `buckets.close` **一一对应**（第 k 条被关腿的
/// 关闭触发候选 = 第 k 个归因，规则2 的消费配对）——组合层（coverage `pi_theta_step_traced` /
/// runner ledger builder）据此经 [`reverse_exit_type`] 产 typed exit，**不在外部重放 fold 配对**
/// （fold 顺序敏感，外部重放 = 平行实现漂移）。
///
/// **∃! 证明锚不动**：[`interpret`] 签名/行为不变（委托本函数丢弃归因），归因是 fold 的确定性
/// 副产品——同一 fold 单实现，非第二权威（分歧A 裁决：typed 语义在 interpret 与
/// coverage_step_from_buckets 之间的组合层产生，interpret 本体不扩定义域）。
pub fn interpret_with_close_triggers(
    gamma: &[Candidate],
    active: &[ActiveLeg],
) -> (Buckets, Vec<Candidate>) {
    // ① ≺_Θ 排序（拷贝引用，不 mutate 输入）。
    let mut ordered: Vec<&Candidate> = gamma.iter().collect();
    ordered.sort_by(|a, b| theta_key(a).cmp(&theta_key(b)));

    // ② 确定性 fold。working = A_t 的工作拷贝（bool=本 fold 已关闭）；opened=本 fold 已开 (level,σ)。
    let mut working: Vec<(ActiveLeg, bool)> = active.iter().map(|&l| (l, false)).collect();
    let mut opened: Vec<(u32, VoiceSide)> = Vec::new();
    let mut buckets = Buckets::default();
    // close 触发归因（与 buckets.close 同步 push，一一对应）。
    let mut close_triggers: Vec<Candidate> = Vec::new();

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
            close_triggers.push(*c); // 归因：本腿由候选 c 反向关闭（typed exit 原料）
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
    debug_assert_eq!(
        buckets.close.len(),
        close_triggers.len(),
        "close 桶与触发归因一一对应（同步 push 不变量）"
    );
    (buckets, close_triggers)
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
            struct_break_dir: None,
            force: None,
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
            struct_break_dir: None,
            force: None,
        }
    }

    fn classification(levels: Vec<Vec<BspPoint>>) -> Classification {
        Classification {
            levels: levels
                .into_iter()
                .map(|bsp| LevelState { bsp: Rc::new(bsp), ..Default::default() })
                .collect(),
        }
    }

    /// BspPoint 构造：给定 bits + struct_break_dir（P2-R2 守卫测试用）。
    fn pt(bits: BspBits, sbd: Option<Side>) -> BspPoint {
        BspPoint { source_index: 0, bits, pivot_low: 0, pivot_high: 0, center: None, struct_break_dir: sbd, force: None }
    }

    /// ★P2-R2 护栏2（codex-review-20260701-2251 [guard]）：struct_break_dir 恢复方向**只改
    /// candidate_dir，绝不改 class_index**。同一 bits 下六 bit + class_index() 完全不变——
    /// struct_break_dir 不进 BspBits/MuClass/class_index/桶 key。
    #[test]
    fn struct_break_dir_recovers_direction_without_touching_class_index() {
        // 零 bit + struct_break_dir=Some(Long) ⟹ candidate_dir 恢复 Long，class_index 仍 0（六 bit 全零）。
        let zero_long = pt(BspBits::default(), Some(Side::Long));
        assert_eq!(candidate_dir(&zero_long), VoiceSide::Long, "零 bit 破中枢候选恢复 Long 方向");
        assert_eq!(zero_long.bits.class_index(), 0, "六 bit 全零 ⟹ class_index=0（struct_break_dir 不进桶键）");

        // 零 bit + Some(Short) ⟹ Short，class_index 仍 0。
        let zero_short = pt(BspBits::default(), Some(Side::Short));
        assert_eq!(candidate_dir(&zero_short), VoiceSide::Short);
        assert_eq!(zero_short.bits.class_index(), 0);

        // 零 bit + None（非破中枢候选）⟹ Flat（无恢复源），class_index 0。
        let zero_none = pt(BspBits::default(), None);
        assert_eq!(candidate_dir(&zero_none), VoiceSide::Flat, "无 struct_break_dir ⟹ 保持 Flat");
        assert_eq!(zero_none.bits.class_index(), 0);
    }

    /// ★P2-R2 护栏1（codex [致命]）：**有六 bit 方向**的候选走原 root_sel 路径，struct_break_dir
    /// **不覆盖**其方向。且 class_index 由六 bit 唯一决定，与 struct_break_dir 无关（同 bits 下不变）。
    #[test]
    fn six_bit_direction_not_overridden_by_struct_break_dir() {
        let buy1_bits = BspBits { buy1: true, ..Default::default() };
        // buy1 候选（Long）+ 矛盾的 struct_break_dir=Some(Short)：candidate_dir 仍 Long（六 bit 优先）。
        let buy1_conflict_sbd = pt(buy1_bits, Some(Side::Short));
        assert_eq!(candidate_dir(&buy1_conflict_sbd), VoiceSide::Long,
            "有 buy1 六 bit ⟹ 走 root_sel=Long，struct_break_dir=Short 不覆盖（护栏1 严格零 bit 前提不满足）");
        // class_index 只由六 bit：buy1=true ⟹ 1，与 struct_break_dir 取值无关（None/Some 同值）。
        assert_eq!(pt(buy1_bits, None).bits.class_index(), 1);
        assert_eq!(pt(buy1_bits, Some(Side::Short)).bits.class_index(), 1);
        assert_eq!(pt(buy1_bits, Some(Side::Long)).bits.class_index(), 1,
            "class_index 恒 =1（buy1），struct_break_dir 三种取值下逐字节不变");
    }

    /// ★A6（#159）透传护栏：`BspPoint.force` → `Candidate.force` 纯透传（扁平 assemble_gamma 与
    /// 塔路径两条组装线），组装层不改值不兜底——透传断裂（gamma 侧恒 None）在此 fail。
    #[test]
    fn a6_candidate_carries_bsp_point_force() {
        use super::super::super::classifier::divergence::ForceFeatures;
        let ff = |s: f64| ForceFeatures {
            macd_area: 8.0 * s,
            dif_peak: 1.5 * s,
            price_amplitude: (60.0 * s) as i64,
            price_speed: 3.0 * s,
            tv: (90.0 * s) as i64,
        };
        let fp = ForceProxies { seg_a: ff(1.0), seg_c: ff(0.5) };
        let mut p1 = buy_point(3, 1);
        p1.force = Some(fp); // 一类 A/C 对候选携力度
        let p2 = buy_point(7, 2); // 二类无 A/C 对 ⟹ force=None
        let gamma = assemble_gamma(&classification(vec![vec![p1, p2]]));
        assert_eq!(gamma.len(), 2);
        assert_eq!(gamma[0].force, Some(fp), "扁平组装：BspPoint.force 逐字段透传进 Candidate");
        assert_eq!(gamma[1].force, None, "无力度源候选诚实 None（不兜底）");
        // 塔路径同款透传（与扁平版同产候选序；缺塔 ⟹ 空 tree 边界，透传不依赖塔）。
        let gamma_t = assemble_gamma_with_tower(&classification(vec![vec![p1, p2]]), &[]);
        assert_eq!(gamma_t[0].force, Some(fp), "塔路径组装：同款透传");
        assert_eq!(gamma_t[1].force, None);
    }

    /// ★A12 orphan 见证塔（648 裁决 D）：L2 根只收 c2a/c2b，c1（L1）整棵子树掉出 T_i=↓r_i
    /// ——c1 及其 L0 subs 是 orphan frontier。K_i（extract_carrier_forest）全含。
    fn a12_orphan_tower() -> Vec<Rc<Vec<LeveledMove>>> {
        use super::super::super::classifier::recursive_tower::ElementId;
        use super::super::super::classifier::center::UnitRange;
        use super::super::super::types::Direction;
        let u = |si: usize, ei: usize, dir: Direction| UnitRange {
            start_index: si, end_index: ei, direction: dir, lo: 0, hi: 10,
        };
        let e = |level: u32, ordinal: u64| ElementId { level, ordinal };
        let c = |s: usize, x: usize| Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: s, end_index: x };
        let s0 = LeveledMove::from_unit(&u(0, 4, Direction::Up), e(0, 0));
        let s1 = LeveledMove::from_unit(&u(4, 8, Direction::Down), e(0, 1));
        let s2 = LeveledMove::from_unit(&u(8, 12, Direction::Up), e(0, 2));
        let c1 = LeveledMove::compose(&[s0, s1, s2], c(0, 12), 1, e(1, 0)); // 外缘 Up=Long
        let t0 = LeveledMove::from_unit(&u(12, 16, Direction::Down), e(0, 3));
        let t1 = LeveledMove::from_unit(&u(16, 20, Direction::Up), e(0, 4));
        let t2 = LeveledMove::from_unit(&u(20, 24, Direction::Down), e(0, 5));
        let c2a = LeveledMove::compose(&[t0, t1, t2], c(12, 24), 1, e(1, 1));
        let v0 = LeveledMove::from_unit(&u(24, 28, Direction::Up), e(0, 6));
        let v1 = LeveledMove::from_unit(&u(28, 32, Direction::Down), e(0, 7));
        let v2 = LeveledMove::from_unit(&u(32, 36, Direction::Up), e(0, 8));
        let c2b = LeveledMove::compose(&[v0, v1, v2], c(24, 36), 1, e(1, 2));
        let l2 = LeveledMove::compose(&[c2a.clone(), c2b.clone()], c(12, 36), 2, e(2, 0));
        vec![Rc::new(Vec::new()), Rc::new(vec![c1, c2a, c2b]), Rc::new(vec![l2])]
    }

    /// ★A12（648 裁决 D / P1-3 / 676）生产组装线子声部激活见证：orphan frontier 上的反向 bsp
    /// 经 host^op（K_i 宇宙）在 **生产候选组装线**（assemble_gamma_with_tower）判 V=ShortDiff
    /// ——「子声部结构性恒零」在生产路径解除。旧 T_i 宇宙此 bsp host-miss ⟹ ∂ 根 Ambient
    /// （676 根因），本测试在切换前红、切换后绿（可证伪断言）。
    #[test]
    fn a12_orphan_frontier_bsp_enters_production_line_as_shortdiff() {
        use super::super::coverage::Vertical;
        let tower = a12_orphan_tower();
        // L0 卖点 @ s1.ρ=8（orphan c1 的中间子；δ_g=Short = −σ_{p(g)}，父 c1=Long）。
        let gamma = assemble_gamma_with_tower(&classification(vec![vec![sell_point(8, 3)]]), &tower);
        assert_eq!(gamma.len(), 1);
        assert_eq!(
            gamma[0].role.v,
            Vertical::ShortDiff,
            "host^op(K_i) 命中 s1 真父 c1(Long)，Short=−σ_p ⟹ ShortDiff（子声部对冲腿）"
        );
        // 同向（Long）bsp ⟹ FollowParent（顺父）——V 轴两个非 Ambient 值都可达。
        let gamma_b = assemble_gamma_with_tower(&classification(vec![vec![buy_point(8, 3)]]), &tower);
        assert_eq!(gamma_b[0].role.v, Vertical::FollowParent, "同向 ⟹ FollowParent");
    }

    /// ★A12 缓存键 soundness：旧 [`TreeKey::of`] 只指纹最高非空级——对「最高级不变、低级变」
    /// 的塔变异**盲**（K_i 读全塔 ⟹ 会假命中返陈旧森林；L0 段尾部古怪线段重划正是此形态，
    /// gen 也不 bump——bar 3020）。[`TreeKey::of_forest`] 全级别指纹看见该变异。
    #[test]
    fn a12_of_forest_sees_low_level_change_of_is_blind() {
        use super::super::super::classifier::recursive_tower::ElementId;
        use super::super::super::classifier::center::UnitRange;
        use super::super::super::types::Direction;
        let mk = |x_end: usize| {
            let x = LeveledMove::from_unit(
                &UnitRange { start_index: 0, end_index: x_end, direction: Direction::Up, lo: 0, hi: 10 },
                ElementId { level: 0, ordinal: 9 },
            );
            let s0 = LeveledMove::from_unit(
                &UnitRange { start_index: 12, end_index: 16, direction: Direction::Up, lo: 0, hi: 10 },
                ElementId { level: 0, ordinal: 10 },
            );
            let l1 = LeveledMove::compose(
                &[s0],
                Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: 12, end_index: 16 },
                1,
                ElementId { level: 1, ordinal: 0 },
            );
            // x 是 orphan L0（不被 l1 收录）：改 x 只动 L0 级，最高级 L1 逐字节不变。
            vec![Rc::new(vec![x]), Rc::new(vec![l1])]
        };
        let a = mk(4);
        let b = mk(8);
        assert_eq!(TreeKey::of(&a), TreeKey::of(&b), "旧 of 只看最高级 ⟹ 对 L0 变异盲");
        assert_ne!(TreeKey::of_forest(&a), TreeKey::of_forest(&b), "of_forest 全级别 ⟹ 看见");
        // K_i 输出确实不同——of 若作 K_i 缓存键即假命中返陈旧森林（本测试钉死切换必要性）。
        assert_ne!(
            coverage::extract_carrier_forest(&a),
            coverage::extract_carrier_forest(&b),
            "K_i 依赖 L0 级 ⟹ 输出不同"
        );
    }

    /// ★P2-R2 护栏1（**为什么禁 root_sel==Flat**）：(1,1) 双触发（buy1+sell1 非互斥可重合）经
    /// root_sel 消歧为 Flat（镜像不动点），但**不是**严格零 bit ⟹ struct_break_dir **不**恢复方向
    /// （冲突候选保持 Flat 归 𝒦 记录，不被结构方向误改向）。若用 `root_sel==Flat` 作回退条件则此处
    /// 会误改向——本测试锁定严格零 bit `!conf_plus && !conf_minus` 排除 (1,1)。
    #[test]
    fn double_trigger_conflict_stays_flat_not_recovered() {
        let conflict_bits = BspBits { buy1: true, sell1: true, ..Default::default() };
        // root_sel(1,1)=Flat（voice.rs:270），但 conf_plus()=true ⟹ 严格零 bit 前提不满足 ⟹ 不恢复。
        assert!(conflict_bits.conf_plus() && conflict_bits.conf_minus(), "前提：(1,1) 双触发");
        let conflict = pt(conflict_bits, Some(Side::Long));
        assert_eq!(candidate_dir(&conflict), VoiceSide::Flat,
            "(1,1) 冲突候选保持 Flat——struct_break_dir=Some(Long) 不误改向（护栏1 严格零 bit 排除 (1,1)）");
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
            struct_break_dir: None,
            force: None,
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

    /// ★G4 close 触发归因：`interpret_with_close_triggers` 的归因与 close 桶一一对应，
    /// 且 `.0` 与 [`interpret`] 逐字段相等（委托单源，bit-exact）。
    #[test]
    fn close_triggers_pair_with_close_bucket() {
        // 两条不同 level 的持仓 Long 腿 + 各自 level 的反向卖候选（sell1 @L0，sell3 @L1）。
        let gamma = assemble_gamma(&classification(vec![
            vec![sell_point(10, 1)], // L0 一类卖 → 关 L0 Long 腿
            vec![sell_point(12, 3)], // L1 三类卖 → 关 L1 Long 腿
        ]));
        let active = [aleg(0, VoiceSide::Long, 0, 0), aleg(1, VoiceSide::Long, 2, 2)];
        let (b, triggers) = interpret_with_close_triggers(&gamma, &active);
        assert_eq!(b.close.len(), 2, "两腿各被反向候选关闭");
        assert_eq!(triggers.len(), b.close.len(), "归因与 close 桶一一对应");
        // 第 k 条被关腿的触发候选同 level（规则2 只关同级腿）。
        for (leg, trig) in b.close.iter().zip(&triggers) {
            assert_eq!(leg.level, trig.level, "触发候选与被关腿同级");
        }
        // 委托单源 bit-exact：interpret == interpret_with_close_triggers.0。
        let b2 = interpret(&gamma, &active);
        assert_eq!(b.close, b2.close);
        assert_eq!(
            b.open.iter().map(|c| c.gamma_index).collect::<Vec<_>>(),
            b2.open.iter().map(|c| c.gamma_index).collect::<Vec<_>>()
        );
        assert_eq!(
            b.record.iter().map(|c| c.gamma_index).collect::<Vec<_>>(),
            b2.record.iter().map(|c| c.gamma_index).collect::<Vec<_>>()
        );
    }

    /// ★G4/G5 单源判据 [`reverse_exit_type`]（PDF §9 / G5 映射 §6.1）：
    /// ShortDiff 腿→P7；三类反向→P6；一/二类反向→P5（二类归 CloseRoot 读法）。
    #[test]
    fn reverse_exit_type_criteria_table() {
        use ExitType::*;
        // ShortDiff 腿：任何触发类都是 CloseShortDiff（子声部关闭语义压过触发类）。
        assert_eq!(reverse_exit_type(Vertical::ShortDiff, 1), CloseShortDiff);
        assert_eq!(reverse_exit_type(Vertical::ShortDiff, 3), CloseShortDiff);
        // 根腿（Ambient）：三类反向 → ReduceCore；一/二类 → CloseRoot。
        assert_eq!(reverse_exit_type(Vertical::Ambient, 3), ReduceCore);
        assert_eq!(reverse_exit_type(Vertical::Ambient, 1), CloseRoot);
        assert_eq!(reverse_exit_type(Vertical::Ambient, 2), CloseRoot);
        // FollowParent 子腿：按触发类走 P5/P6（非短差对冲腿）。
        assert_eq!(reverse_exit_type(Vertical::FollowParent, 3), ReduceCore);
        assert_eq!(reverse_exit_type(Vertical::FollowParent, 1), CloseRoot);
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
            struct_break_dir: None,
            force: None,
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

#[cfg(test)]
mod candidate_profile {
    //! 工位 #4h profile（L2）：candidate(gamma)段成本分布 + 标度 exp。
    //! 隔离遍历1（candidates 构建）vs 遍历2（role/gamma 组装），量化 caller B 丢弃 gamma 后
    //! 遍历2 是否纯浪费 + candidate 段是否 O(n²) 主导。
    use super::*;
    use super::super::super::backtest::incremental::IncrementalClassifier;
    use super::super::super::backtest::data;
    use super::super::super::config::ThetaConfig;
    use super::super::super::classifier::bsp::BspPoint;
    use super::super::super::classifier::{Classification, LevelState};
    use super::super::super::types::BspBits;

    /// ★工位 4h 回归守卫（codex 实施审 QUESTION A/C UNSOUND 反例，合成 L1）：fallback ordinal id
    /// 依赖全局 flat ci（依赖**所有前置级** bsp 数）。前置级（L0）增长 ⟹ 后置级（L1）fallback-id 元素
    /// ordinal 必须偏移。base_ci 校验未修则命中复用旧 ordinal → bit-exact 破裂。
    ///
    /// 复现 codex 精确反例：空 tower（tree 空 ⟹ **所有 candidate carrier-miss → fallback ordinal**）。
    /// bar t：L0=2 bsp, L1=1 bsp（L1 fallback ordinal=tree.len()+2=2）。bar t+1：L0=3（L0 增长），
    /// L1=1 同前缀。全量重算 L1 fallback ordinal=3；base_ci 未修的命中复用保留 2 → 发散。
    /// 本测试断言增量（cand_cache 跨 bar）== 全量，base_ci 修复后 L1 因 base_ci 偏移被重建为 ordinal 3。
    ///
    /// **L1**（合成构造，验证管线完备性——formalization-validity-domain 231号；fallback 路径在 CL 8K
    /// 未触发，codex "passing ≠ proof"，本合成测试补完未测路径）。
    #[test]
    fn candidate_cache_fallback_ordinal_prefix_shift() {
        fn buy3(si: usize) -> BspPoint {
            BspPoint { source_index: si, bits: BspBits { buy3: true, ..Default::default() },
                pivot_low: 1, pivot_high: 0, center: None, struct_break_dir: None, force: None }
        }
        let cls = |l0: Vec<usize>, l1: Vec<usize>| Classification {
            levels: vec![
                LevelState { bsp: l0.into_iter().map(buy3).collect::<Vec<_>>().into(), ..Default::default() },
                LevelState { bsp: l1.into_iter().map(buy3).collect::<Vec<_>>().into(), ..Default::default() },
            ],
        };
        // 空 tower ⟹ extract_elements 空 ⟹ tree 空 ⟹ 全 candidate carrier-miss（fallback ordinal）。
        let tower: Vec<Rc<Vec<LeveledMove>>> = Vec::new();
        let gen = Some(7u64); // 同 gen 跨两 bar（模拟 gen 不变但 L0 增长，codex Q2 条件）。

        let mut tree_cache = TreeCache::new();
        let mut cand_cache = CandidateCache::new();

        // bar t：L0=[10,20], L1=[30]。
        let cls_t = cls(vec![10, 20], vec![30]);
        let (_t0, _c0) = coverage_elements_with_tower_cached_gen(&cls_t, &tower, &mut tree_cache, &mut cand_cache, gen);

        // bar t+1：L0 增长到 3（[10,20,25]），L1 前缀不变（[30]）。
        let cls_t1 = cls(vec![10, 20, 25], vec![30]);
        let (_t1, inc) = coverage_elements_with_tower_cached_gen(&cls_t1, &tower, &mut tree_cache, &mut cand_cache, gen);

        // 全量基准（fresh cache，从零全量 build）。
        let mut fresh_tree = TreeCache::new();
        let mut fresh_cand = CandidateCache::new();
        let (_tf, full) = coverage_elements_with_tower_cached_gen(&cls_t1, &tower, &mut fresh_tree, &mut fresh_cand, gen);

        assert_eq!(inc, full, "base_ci 偏移修复：L0 增长后 L1 fallback ordinal 须重建（codex A/C 反例）");
        // 显式验证 L1 fallback ordinal = 全局 flat idx 3（candidate_start=0 + L0 3 个 + L1 第 0 个）。
        let l1 = inc.iter().find(|e| e.level == 1).expect("L1 candidate 存在");
        assert_eq!(l1.id, ElementId { level: 1, ordinal: 3 },
            "L1 fallback ordinal = 全局 flat ci = 3（L0 增长到 3 后偏移；未修则陈旧为 2）");
    }

    /// ★工位 4h bit-exact 守卫（L1 管线正确性，硬约束2）：caller B gamma-free + 前缀缓存路径
    /// [`coverage_elements_with_tower_cached_gen`] 的 candidates **逐 bar bit-identical** 于全路径
    /// [`coverage_elements_and_gamma_with_tower_cached_gen`] 的 candidates（丢 gamma）。
    ///
    /// 真实 CL 增量链（跨 bar 共享两个独立 cache：全路径 tree_cache_full / gamma-free 路径
    /// tree_cache_gf + cand_cache）。任何前缀复用陈旧、gen 假命中、ci 错位、尾部 build 偏移都被捕获。
    #[test]
    #[ignore = "工位 4h L1：caller B 增量 candidate bit-exact vs 全路径；需 CL；--release --ignored"]
    fn candidate_incremental_bit_exact_vs_full() {
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let n = 8_000.min(oos.bars.len());
        let bars = &oos.bars[..n];

        let mut incr = IncrementalClassifier::new(bars, &config);
        let mut tree_cache_full = TreeCache::new();
        let mut tree_cache_gf = TreeCache::new();
        let mut cand_cache = CandidateCache::new();
        let mut hits = 0usize;
        for i in 0..n {
            let (cls, tower) = incr.classify_at(i);
            let gen = incr.tower_generation();
            // 全路径（含遍历2，丢 gamma 取 candidates）。
            let (_t_full, cand_full, _g) = coverage_elements_and_gamma_with_tower_cached_gen(
                &cls, &tower, &mut Some(&mut tree_cache_full), Some(gen),
            );
            // gamma-free + 前缀缓存路径。
            let (_t_gf, cand_gf) = coverage_elements_with_tower_cached_gen(
                &cls, &tower, &mut tree_cache_gf, &mut cand_cache, Some(gen),
            );
            assert_eq!(cand_gf, cand_full,
                "bar {i}: gamma-free 增量 candidates != 全路径（前缀复用陈旧/gen 假命中/ci 错位）");
            if cand_cache.valid && matches!((Some(gen), cand_cache.gen), (Some(a),Some(b)) if a==b) {
                hits += 1;
            }
        }
        eprintln!("工位 4h bit-exact：{n} bars 全部 candidates bit-identical，cand_cache 命中 {hits} bars");
    }

    /// ★工位 4h L2 标度：candidate 段**构建侧** 旧全量路径 vs 新增量（前缀缓存）路径累积时间 + exp。
    ///
    /// 旧 `coverage_elements_and_gamma_with_tower_cached_gen`（含遍历1 全量重建 + 遍历2 gamma）vs
    /// 新 `coverage_elements_with_tower_cached_gen`（gamma-free + per-level 前缀复用，只 build 尾部）。
    ///
    /// **诚实有效域**：本 profile 量 candidate **构建**侧（attach 重复消除）。两路均含返回 Vec 拼接/clone
    /// （merge 接口下界 O(bsp)/bar，codex Q4），故新路径 exp 不会到 1.0——构建摊还 O(Σ Δbsp)，但拼接残余
    /// O(bsp)/bar。端到端 strategy 段标度见 `incremental::profile_full_engine_scaling_16k`。
    #[test]
    #[ignore = "工位 4h L2：candidate 旧全量 vs 新增量构建标度；需 CL；--release --ignored"]
    fn profile_candidate_old_vs_new_scaling() {
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let sizes = [5_000usize, 10_000, 16_000];
        let mut old_t = Vec::new();
        let mut new_t = Vec::new();
        let mut used = Vec::new();

        for &n in &sizes {
            if n > oos.bars.len() { break; }
            let bars = &oos.bars[..n];

            // 旧全量路径（_cached_gen，含遍历2）累积。
            let mut incr = IncrementalClassifier::new(bars, &config);
            let mut tc = TreeCache::new();
            let t0 = std::time::Instant::now();
            for i in 0..n {
                let (cls, tower) = incr.classify_at(i);
                let gen = incr.tower_generation();
                let _ = coverage_elements_and_gamma_with_tower_cached_gen(
                    &cls, &tower, &mut Some(&mut tc), Some(gen));
            }
            old_t.push(t0.elapsed().as_secs_f64());

            // 新增量路径（gamma-free + 前缀缓存）累积。
            let mut incr2 = IncrementalClassifier::new(bars, &config);
            let mut tc2 = TreeCache::new();
            let mut cc = CandidateCache::new();
            let t1 = std::time::Instant::now();
            for i in 0..n {
                let (cls, tower) = incr2.classify_at(i);
                let gen = incr2.tower_generation();
                let _ = coverage_elements_with_tower_cached_gen(
                    &cls, &tower, &mut tc2, &mut cc, Some(gen));
            }
            new_t.push(t1.elapsed().as_secs_f64());
            used.push(n);
            eprintln!("n={n}: old={:.3}s new={:.3}s (new/old={:.2}x)",
                old_t.last().unwrap(), new_t.last().unwrap(),
                new_t.last().unwrap() / old_t.last().unwrap().max(1e-12));
        }

        eprintln!("\n===== 工位 4h candidate 旧全量 vs 新增量标度（CL per-bar）=====");
        for w in used.windows(2) {
            let (n0, n1) = (w[0], w[1]);
            let i0 = used.iter().position(|&s| s == n0).unwrap();
            let i1 = i0 + 1;
            let oe = (old_t[i1] / old_t[i0].max(1e-12)).ln() / (n1 as f64 / n0 as f64).ln();
            let ne = (new_t[i1] / new_t[i0].max(1e-12)).ln() / (n1 as f64 / n0 as f64).ln();
            eprintln!("  [{n0}→{n1}] old exp≈{oe:.2} ({:.3}s) | new exp≈{ne:.2} ({:.3}s)",
                old_t[i1], new_t[i1]);
        }
    }
}
