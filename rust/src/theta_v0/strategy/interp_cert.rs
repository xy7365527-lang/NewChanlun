//! 证书投影族 + Buckets 族（#1193 职责块自 `interp.rs` 迁出，零行为）。
//!
//! 承载两组职责：
//! - **证书投影族**：[`ParentCertificateProjection`]（#149 不可伪造投影 token）/
//!   [`parent_certificate_projection`]/[`trigger_projection_sound`] 与 [`EntryCertificate`]/
//!   [`PositionNodeId`]（★A9 开仓证书 + 四元组身份）。
//! - **Buckets 族**：三桶 [`Buckets`] + typed exit（[`ExitType`]/[`reverse_exit_type`]/
//!   [`exit_type_of_classes`]）+ 环5 ℛ_Θ（[`theta_key`]/[`theta_lt`]/[`interpret`] 一族）。
//!
//! 消费面经 `interp` 重导出保持 `interp::X` 原路径（stream/ledger/selector/channel/coverage/探针族
//! 等消费方不改调用）；测试套随块归位至本文件 `mod tests`。[`Candidate`]/[`ActiveLeg`]/
//! [`assemble_gamma`](super::assemble_gamma) 组装留 `interp` 门面。

use super::super::super::classifier::recursive_tower::ElementId;
use super::super::coverage::{self, CoverageElement, Dir, Horizontal, Vertical};
use super::super::exec::reverse_signal;
use super::super::voice::VoiceSide;
use super::{ActiveLeg, Candidate};
use std::cmp::{Ordering, Reverse};

/// #149 `TriggerProjectionSound` 的不可伪造投影载体。
///
/// token 只能由 [`parent_certificate_projection`] 从真塔导出的 `CoverageElement` 父子边构造；
/// 字段保持私有，通道层只能携带并交给 [`trigger_projection_sound`] 核验，不能用一个裸 bool
/// 把 child signal 冒充成 parent-level certificate projection。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParentCertificateProjection {
    parent_id: ElementId,
    parent_level: u32,
    parent_dir: VoiceSide,
    child_level: u32,
    child_source_index: usize,
    child_dir: VoiceSide,
}

impl ParentCertificateProjection {
    /// 父腿身份（#196 shadow 适配层逐声部分发投影的过滤键——只读访问，字段保持私有，
    /// token 仍只能由 [`parent_certificate_projection`] 构造、由 [`trigger_projection_sound`] 核验）。
    pub fn parent_id(&self) -> ElementId {
        self.parent_id
    }
}

/// 从真嵌套塔的候选附着边生成父级证书投影。
///
/// `cand_elems[cand.gamma_index].parent -> tree[parent_idx]` 必须是一条完整、方向一致的真父子边：
/// candidate 坐标与元素逐字段一致、`parent_id`/`attached_dir` 与父元素一致、child level 严格低于
/// parent level、角色 G=SubLevel 且 V 与父子方向关系一致。候选还必须是已确认可交易证书。
/// 任一条件缺失返回 `None`，因此缺塔/孤儿/伪角色/未确认 child 都不能得到 trigger token。
pub fn parent_certificate_projection(
    cand: &Candidate,
    cand_elems: &[CoverageElement],
    tree: &[CoverageElement],
) -> Option<ParentCertificateProjection> {
    if !cand.nest_confirmed || cand.dir == VoiceSide::Flat || cand.bsp_class == u8::MAX {
        return None;
    }
    let child = cand_elems.get(cand.gamma_index)?;
    let parent = child.parent.and_then(|idx| tree.get(idx))?;
    if child.level != cand.level
        || child.rho != cand.source_index
        || child.eps != cand.dir
        || child.parent_id != Some(parent.id)
        || child.attached_dir != Some(parent.eps)
        || cand.level >= parent.level
        || cand.role.grade != coverage::GradeRel::SubLevel
    {
        return None;
    }
    let expected_delta = match cand.dir {
        VoiceSide::Long => Dir::Plus,
        VoiceSide::Short => Dir::Minus,
        VoiceSide::Flat => return None,
    };
    if cand.role.delta != expected_delta {
        return None;
    }
    let expected_v = if cand.dir == parent.eps {
        Vertical::FollowParent
    } else if super::super::voice::reverse_open_side(parent.eps) == Some(cand.dir) {
        Vertical::ReverseOpen
    } else {
        return None;
    };
    if cand.role.v != expected_v {
        return None;
    }
    Some(ParentCertificateProjection {
        parent_id: parent.id,
        parent_level: parent.level,
        parent_dir: parent.eps,
        child_level: cand.level,
        child_source_index: cand.source_index,
        child_dir: cand.dir,
    })
}

/// #149 `TriggerProjectionSound`：ShortDiffEntry/Exit 的 child trigger 必须有当前父腿的真实投影。
///
/// 除 token 身份逐字段匹配外，再次核验候选仍为 confirmed、可交易、严格次级别证书；这样 token
/// 与候选被错配、父腿 campaign 已切换、或 child signal 没有 parent projection 时都返回 false。
pub fn trigger_projection_sound(
    parent: &ActiveLeg,
    child: &Candidate,
    projection: &ParentCertificateProjection,
) -> bool {
    child.nest_confirmed
        && child.dir != VoiceSide::Flat
        && child.bsp_class != u8::MAX
        && child.level < parent.level
        && child.role.grade == coverage::GradeRel::SubLevel
        && projection.parent_id == parent.id
        && projection.parent_level == parent.level
        && projection.parent_dir == parent.dir
        && projection.child_level == child.level
        && projection.child_source_index == child.source_index
        && projection.child_dir == child.dir
}

/// ★A9（Task #166）：开仓证书——入场买卖点信号 g 的坐标身份 `(ℓ_g, source_index_g)`。
///
/// 买卖点叶子只能作**开仓证书**，不能作持仓身份（级别容器.pdf p14/§12 核心裁决：「买卖点叶子
/// 只能作为开仓证书，不能作为父声部持仓节点」）——持仓身份是 carrier（[`ActiveLeg::id`]），
/// 证书是身份四元组的入场分量（[`PositionNodeId`]）。同一 carrier 两次 campaign 若由不同买卖点
/// 触发，证书即区分入场来源；同证书重入场由 generation 区分。
///
/// ★归属层（codex a9-posnode 裁定 C，2026-07-03）：证书/四元组归**账本生命周期层**
/// （[`LedgerOpen`](crate::theta_v0::backtest::runner)/`TypedTrade`），**不进** `ActiveLeg`
/// 结构层——`ActiveLeg` 每 bar 由 `element_as_leg(&CoverageElement)` 从因果树重建（`CoverageElement`
/// 不携 campaign 身份），拿不到入场候选/代次；真正同时持有 `Candidate`+`ActiveLeg` 的点是
/// `StepTrace.opened`（runner 据此登记 `LedgerOpen`）。把证书塞进 `ActiveLeg`（前任半成品）会
/// 使 generation 恒 0/证书恒 None = 声明膨胀（090号）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntryCertificate {
    /// 入场信号级别 ℓ_g。
    pub level: u32,
    /// 入场信号触发点（bsp `source_index`，入场时刻值，冻结不漂移）。
    pub source_index: usize,
}

/// ★A9 多实例身份四元组（级别容器.pdf p14/§13 `posId = hash(carrier_id, entry_signal, side,
/// generation)`，Task #166 ceiling）：position instance 严格身份。
///
/// carrier-id 简化身份（§14，`ActiveLeg::id` 结构对位键）只到 carrier 级——同一 carrier 先后两次
/// campaign 共享 `ActiveLeg::id`。本四元组细化到 campaign 实例级：carrier 相同而 generation（或
/// entry_certificate/side）不同 ⟹ 不同 position instance。**不替换** carrier-id 层：`ActiveLeg::id`
/// 仍是结构对位键（`held_leg_tree_index`/AncOK 按 carrier 匹配），四元组是其上、在账本层的实例细分。
///
/// 生产构造点：[`LedgerOpen`](crate::theta_v0::backtest::runner) 入场登记（carrier=腿 id、
/// entry_certificate=开仓 Candidate 坐标、side=Candidate 方向、generation=同 carrier campaign 高水位），
/// 关腿时写入 `TypedTrade::position_node_id`（账本层唯一身份，供跨笔同 carrier campaign 归因/去重）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionNodeId {
    /// 持仓容器 carrier（= hostOf(g) 的 ElementId，638 附着）。
    pub carrier: ElementId,
    /// 开仓证书（入场信号 g 坐标）；None = 结构激活（restore 祖先，无入场信号）。
    pub entry_certificate: Option<EntryCertificate>,
    /// 持仓方向 δ(g)。
    pub side: VoiceSide,
    /// campaign 代次（同 carrier 顺序 campaign 单调递增，close→reopen 高水位 +1）。
    pub generation: u32,
}

impl PositionNodeId {
    /// 严格 hash 编码（PDF §13 伪码 `posId = hash(...)`）。`DefaultHasher::new()` 固定初始键 ⟹
    /// 同一四元组跨运行/跨路径产同一 u64（确定性，非 RandomState）。
    pub fn hash64(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut h);
        h.finish()
    }
}

/// 解释器输出三桶 (𝒟_x, ℬ_x, 𝒦_x)（spec §11 line 565-571 + §12 line 617）。
///
/// 互斥分流：`𝒟_x`（关闭活动腿）∩`ℬ_x`/`𝒦_x`（候选）为空（不同类型）；`ℬ_x`∩`𝒦_x`=∅
/// （每个候选恰落一桶，[`interpret`] fold 保证）。**#200 例外**：二类反向触发候选可
/// 同时是关闭触发（close_triggers 归因）**与** `ℬ_x`/`𝒦_x` 成员（「先平后开」，
/// spec WP-2 修复 c）——`ℬ_x`∩`𝒦_x`=∅ 不受影响，但「关闭触发候选不入 ℬ_x/𝒦_x」
/// 仅对一/三类成立。
#[derive(Debug, Clone, Default)]
pub struct Buckets {
    /// 𝒟_x：应**关闭**的活动腿（⊆ A_t；同级别已确认反向证书触发，§9 closePred 反向项）。
    pub close: Vec<ActiveLeg>,
    /// ℬ_x：应**开启**的候选（新建腿）。
    pub open: Vec<Candidate>,
    /// 𝒦_x：**记录但不执行**的候选（冲突/重复/非方向）。
    pub record: Vec<Candidate>,
}

/// 正规出场类型 `Exit_Θ`（《完整的策略.pdf》§9 typed exit）——close 桶的类型化出场理由。
///
/// `Exit_Θ(v,x_t) ∈ {CloseRoot, ReduceCore, CloseReverseOpen, RiskExit, Hold}`（原第四枚
/// `CloseShortDiff`，#281 更名 `CloseReverseOpen`，#283 实装——ADR 0001 修正案一·补充一
/// 词汇对齐：S6「短差」名随修1 废止退役，重读为修4「次级别首开反向」）。这是 G4（统计层
/// typed exit：生产 π loop 输出 `TypedTradeLedger.exit_type`）与 G5（解释器 P5/P6/P7/P1 typed
/// close 拆分）的**单一来源**枚举（team-lead 2026-07-03 裁定：由 interp.rs 定义，G4 工位复用），
/// 避免两权威镜像（codex-q2-d1 删 `closed_loop/mutex_interp.rs` 同款矛盾）。P1..P10↔ExitType
/// 映射见 `.chanlun/review-results/g5-interpreter-mapping-20260703.md` §6.1。
///
/// **接线状态**：G4（#134）已在统计层接线——[`reverse_exit_type`] 单源判据 + 生产 π fill loop
/// 的 `TypedTradeLedger`（runner.rs）消费本枚举；#145 T1 已把反向关闭的 typed 裁决**前移到
/// 组合层决策点**（coverage.rs `StepTrace::closed` 第三分量，entry_v 经 `TwStepCtx::entry_v`
/// 在飞映射喂 [`reverse_exit_type`]，runner 直接消费不再结算补算）。interp close 桶公开签名
/// 与 ∃! 证明锚不动，生产订单流 bit-exact 不变（typed 化只是裁决标注）；P5/P6/P7 生产拆分
/// 在 G5 实装阶段（#124，须复用 [`reverse_exit_type`]）。
/// `closed_loop/sell.rs::SellDecision` 曾有 CloseRoot/ReduceCore 重叠（G4 把 μ 管线重接生产 π 后，
/// runner typed 主链不再消费该路径；但 econ 诊断链 `exit_decision_from_bits` 仍以其 type1>type3
/// 平仓优先级为单一权威——#181 AFK 审计据此证伪「纯死路径」前提）——**#181 已收敛完成**：
/// econ 消费方迁移后 SellDecision 侧删除，其优先级语义迁入 [`exit_type_of_classes`]（econ 诊断链
/// bits 入口消费），统一收敛到本枚举单源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitType {
    /// P5 CloseRoot：关 depth-0 根腿（一类反向点=根清仓）。
    CloseRoot,
    /// P6 ReduceCore：减核心仓（三类反向点=核心仓减仓）。
    ReduceCore,
    /// P7 CloseReverseOpen：关首开反向（ReverseOpen）carrier 子声部（次级别首开反向确认关闭）。
    /// 原 `CloseShortDiff`，#281 更名（#283 实装）。
    CloseReverseOpen,
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
/// - `entry_v == ReverseOpen` ⟹ [`ExitType::CloseReverseOpen`]（P7：首开反向子声部反向确认关闭，
///   优先于触发类判定——子声部关闭语义压过触发信号语义）。
/// - 否则 `trigger_class == 3` ⟹ [`ExitType::ReduceCore`]（P6：三类反向点=核心仓减仓）。
/// - 否则 ⟹ [`ExitType::CloseRoot`]（P5：一类反向点=根清仓；**二类反向归 CloseRoot**——
///   PDF §9 五枚举无二类单列，二类是一类的次级确认，同属根反转语义；三类才是中枢离开
///   确认=减仓语义。此读法已向 ws-g5interp 征求意见，翻转条件见 g4-impl 结果包边界条件）。
///   #199 裁定（2026-07-23）：typed 五枚举**不动**——二类 typed 归因保留 CloseRoot；
///   「仅残余才纠错」（CoreResidualCorrection）在账户/理由轴实装（account.rs
///   [`reason_of_reverse_close`](crate::theta_v0::strategy::account::reason_of_reverse_close)），
///   账户身份与退出理由正交。
///
/// `FollowParent` 子腿被反向关闭按触发类走 P5/P6（跟随父方向的级联核心仓，非首开反向对冲腿）。
pub fn reverse_exit_type(entry_v: Vertical, trigger_class: u8) -> ExitType {
    if entry_v == Vertical::ReverseOpen {
        ExitType::CloseReverseOpen
    } else if trigger_class == 3 {
        ExitType::ReduceCore
    } else {
        ExitType::CloseRoot
    }
}

/// 买卖点分类判据 → 平仓出场类型的**优先级映射**（type1>type3 单一来源）。
///
/// **#181 迁移**：本函数自 `closed_loop/sell.rs::sell_decision_of` 迁入——SellDecision 死路径
/// 按改造裁定下线，type1>type3 优先级语义统一收敛到 [`ExitType`] 单源（[`ExitType`] docstring
/// 的收敛预告由本迁移兑现）。返回类型由旧 `SellDecision` 三态换成 [`ExitType`]（CloseRoot/
/// ReduceCore/Hold 一一对应，分支语义逐条不变）。
///
/// 从「是否第一类 / 是否第三类」两 bool 判据映为 [`ExitType`]，编码 §10.1/§11 缠论分支优先级：
/// 第一类（顶背驰清仓）优先于第三类（回抽减核），二者皆否 ⟹ Hold（力度延续/非卖点）。
///
/// **单一来源（no-patch）**：type1>type3 优先级只在此处定义。`backtest::econ_positive::
/// exit_decision_from_bits`（bsp bits 入口）消费本函数，不重编码优先级。出场决策恒为平仓语义
/// （CloseRoot/ReduceCore）；空头出场由买点信号触发（买点镜像卖点，Lean
/// `type1_buy_sell_share_divergence`）但决策仍是平仓 ⟹ 买/卖两方向出场同走本映射。
///
/// 与 [`reverse_exit_type`] 的分工：两入口输入形状不同（本函数接一/三类判据 bool；
/// [`reverse_exit_type`] 接被关腿 `entry_v` + 反向触发类），优先级结论一致（一类触发归
/// CloseRoot 压过三类归 ReduceCore——[`reverse_exit_type`] 侧由调用方喂最小成立类实现）。
/// [`reverse_exit_type`] 的「二类反向归 CloseRoot」是 #199 裁定的生产 typed 链读法，与本函数
/// 服务的 econ 诊断链「二类 ⟹ Type2Missing 诚实标注」口径正交（本函数不消费二类判据）。
pub fn exit_type_of_classes(is_type1: bool, is_type3: bool) -> ExitType {
    if is_type1 {
        // §10.1 第一类：突破中枢 + 背驰 ⟹ 清根仓。
        ExitType::CloseRoot
    } else if is_type3 {
        // §10.1 第三类：离开中枢 + 第一次回抽 + 不破 ZD ⟹ 减核。
        ExitType::ReduceCore
    } else {
        // §11：力度延续 / 非卖点 ⟹ 保持。
        ExitType::Hold
    }
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
        Vertical::ReverseOpen => 2,
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
/// 2. **反向关闭（证书门）**：仅当 `g.nest_confirmed=true`，A_t 中存在同级别、未关闭、方向被 g
///    反向（[`reverse_signal`]，§9 closePred 反向项 χ^{σ_p}）的活动腿，才把该腿送入 `𝒟_x` 并消费
///    g（spec §13 `A_t∖𝒟_x`）。**#209 一类全平**（S7：级别内全平是必须非应当，用户裁 A
///    2026-07-23；spec ID-3「一类卖点：当场全平（该级该方向全部清仓）」）：一类候选关闭该级别
///    **全部**未关闭反向命中腿（Ambient 根 / FollowParent 级联 / §13 restore 祖先一视同仁），
///    每腿恰一触发归因；二/三类维持关闭首个命中腿。`nest_confirmed=false` 只是证书成立层
///    尚未确认的方向信号，直接进入
///    `𝒦_x`：§9 出场层不得重算 N^δ/背驰，也不得消费未成立证书；且 §13 的 `ℬ_x` 会真实激活持仓腿，
///    故不能让被证书门拒绝的反向信号继续落规则3、反向开同一 carrier。**#200 例外（先平后开）**：
///    二类反向触发候选被消费后**不停止**，继续落规则3/4 同款 slot 判据补开反向腿（OpenShort
///    通道，spec WP-2 修复 c）；一/三类维持消费即止。
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
/// 关闭触发候选 = 第 k 个同级别已确认证书归因，规则2 的消费配对）——组合层
/// （coverage `pi_theta_step_traced` /
/// runner ledger builder）据此经 [`reverse_exit_type`] 产 typed exit，**不在外部重放 fold 配对**
/// （fold 顺序敏感，外部重放 = 平行实现漂移）。
///
/// **∃! 证明锚不动**：[`interpret`] 签名/行为不变（委托本函数丢弃归因），归因是 fold 的确定性
/// 副产品——同一 fold 单实现，非第二权威（分歧A 裁决：typed 语义在 interpret 与
/// coverage_step_from_buckets 之间的组合层产生，interpret 本体不扩定义域）。
///
/// **#202 并存声明**：本函数不是 fold 的唯一驱动形态——[`interpret_with_external_closes`]
/// 是同一 fold 的第二驱动（规则2 输入源 = channel P2/P3 域裁决，生产 `pi_theta_step_traced`
/// 正常路径消费）；本函数保留为「规则2 纯候选驱动」的语义基准与 ∃! 文档锚。
///
/// **#396 单源化**：两个驱动共用私有 fold 本体 `fold_theta`（规则2 输入源 = 参数），本函数
/// 退化为 `external_closes = &[]` 的薄 adapter ⟹ 规则演进只需改一处（#202 遗留的「两处
/// 同步」义务随之撤销）。
pub fn interpret_with_close_triggers(
    gamma: &[Candidate],
    active: &[ActiveLeg],
) -> (Buckets, Vec<Candidate>) {
    fold_theta(gamma, active, &[])
}

/// ★#202 阶段 C：规则2 **外部化**的 fold 驱动（spec WP-3 阶段 C「仅替换 P2/P3」）。
///
/// 本级证书平仓域（channel 口径 P2/P3 = entry_v≠ReverseOpen 腿的 CloseRoot/ReduceCore）
/// 由 channel 判据 [`super::channel::cert_close_trigger`] 逐腿裁决后**喂入**
/// （`external_closes: (active_idx, trigger)`——「每声部每步一枚」的 channel 裁决替代
/// 散装 fold 规则2 的候选消费粒度）；组合层 coverage `pi_theta_step_traced` 正常路径
/// 是唯一生产消费点。
///
/// 语义边界（票面「其余通道维持现状」）：
/// - 预置关闭标记后 fold 原逻辑零改：`!was_closed` 检查自动跳过 external 腿 ⟹ 规则2
///   候选驱动语义只作用于剩余腿（entry_v==ReverseOpen 的 S 组——P4 域维持散装现状）。
/// - external 触发候选计入 `closed_any`：一类 ⟹ 消费即止（不入 open）；二类 ⟹
///   「先平后开」dual-effect 照常（#200 OpenShort 通道在 channel 判据下等价成立）。
/// - close 归因序 = `(≺_Θ(trigger), active_idx)` 稳定序——与纯候选驱动归因序（候选 ≺_Θ
///   遍历主序、同候选内 active 次序，#209 一类全平 push 序）同构：同构域 bit-exact，
///   分歧域（多腿/多候选，票面明知非 bit-exact）次序语义一致可逐条对照。
///
/// **#396 单源化**：本函数与 [`interpret_with_close_triggers`] 共用私有 fold 本体
/// `fold_theta`，二者退化为薄 adapter（差异只剩 `external_closes` 实参）；归因仍是 fold 内
/// 同步 push，非第三权威（分歧A 裁决同款纪律）。「external 为空时与本体重逢（bit-exact）」由此
/// 从两份实现在**单射域**上的巧合，升级为构造性恒等（同一实现、同一实参路径）——代价是原自测锁
/// `external_closes_empty_is_bit_exact_with_plain_fold` 退化为 `f(x)==f(x)`，不再可能失败；
/// 规则体的真锁改由 `fold_theta_golden_buckets_pin_rule_body` 承担。
pub fn interpret_with_external_closes(
    gamma: &[Candidate],
    active: &[ActiveLeg],
    external_closes: &[(usize, Candidate)],
) -> (Buckets, Vec<Candidate>) {
    fold_theta(gamma, active, external_closes)
}

/// **fold 规则体单源实现**（#396）：[`interpret_with_close_triggers`] 与
/// [`interpret_with_external_closes`] 的唯一共同本体。
///
/// 两个 pub 驱动的差别只有**规则2 的输入源**，故把它提为参数 `external_closes`：
/// - `&[]` ⟹ 规则2 纯候选驱动（spec §12 ℛ_Θ 定义基例，∃! 文档锚）；
/// - 非空 ⟹ channel P2/P3 域裁决预置关闭，规则2 只作用于剩余（散装域）腿。
///
/// 规则1 / 证书门 / 规则2 / 规则3 / 规则4 + #200 dual-effect + #209 一类全平**只存在这一份**。
///
/// **前置条件（调用方保证）**：`gamma_index` 在 `gamma` 上**单射**。
///
/// 注意这**不是** `theta_key` 的性质——`theta_key` 把 `gamma_index` 列为末位分量，但「某字段
/// 是键的一个分量」推不出「该键无平局」（`bits` 就不在键里：两候选可以八个键分量全同而 `bits`
/// 不同）。单射性是 Γ **构造方**的性质：[`assemble_gamma`](super::assemble_gamma) 取 `enumerate()` 的 i、塔路径取
/// push 循环内严格递增的 `gamma.len()`，两条生产组装线都满足。本函数以 `debug_assert` 兜住
/// 该前提（release 编译掉），使违约在测试构建里立刻响，而不是静默改变归因次序。
///
/// 该前提是 ≺_Θ 全序（spec §12 ∃! 前提2）的实际来源，也是两个驱动能共用一份规则体的边界：
/// 合并前的两份实现在 `theta_key` 出现平局时**本就互不相等**（纯候选驱动返回 push 序；external
/// 驱动无条件按 `(theta_key, active_idx)` 重排，平局时可能重排出另一个序）。即平局域上并不存在
/// 「一个旧行为」可供保持——#202 声明的「external 为空时与本体重逢」本身就只在单射域上成立。
/// 故此处把单射显式化为契约，而不是在实现里假装两者恒等。
///
/// **⑤ 归因序重排的适用域**：在上述前置条件下，`external_closes` 为空时 (close, trigger) 对沿
/// ≺_Θ 遍历主序（单射 ⟹ 跨候选 `theta_key` 严格升）+ 同候选内 `active_idx` 升序（`level_idx`
/// 按 i 升序建）push，已然满足重排目标序 ⟹ 重排是恒等，故跳过。跳过与否在契约域内无语义差别；
/// 选此形态是为了让纯候选驱动路径（spec §12 ℛ_Θ 定义基例 / ∃! 文档锚）在 release 下逐指令等同
/// 于合并前，且不为它引入无谓的 O(n log n)。
fn fold_theta(
    gamma: &[Candidate],
    active: &[ActiveLeg],
    external_closes: &[(usize, Candidate)],
) -> (Buckets, Vec<Candidate>) {
    // ⓪ 前置条件守卫：gamma_index 在 gamma 上单射（见 fn doc）。release 编译掉。
    debug_assert!(
        {
            let mut seen = std::collections::HashSet::with_capacity(gamma.len());
            gamma.iter().all(|c| seen.insert(c.gamma_index))
        },
        "fold_theta 前置条件被破坏：gamma_index 在 gamma 上非单射 ⟹ ≺_Θ 出现平局，close 归因序\
         不再由契约确定（常见成因：测试构造器硬编码 gamma_index: 0 后拼成多候选 gamma）"
    );

    // ① ≺_Θ 排序（拷贝引用，不 mutate 输入）。
    let mut ordered: Vec<&Candidate> = gamma.iter().collect();
    ordered.sort_by_key(|c| theta_key(c));

    // ② 确定性 fold。working = A_t 的工作拷贝（bool=本 fold 已关闭）；opened=本 fold 已开 (level,σ)。
    let mut working: Vec<(ActiveLeg, bool)> = active.iter().map(|&l| (l, false)).collect();
    let mut opened: Vec<(u32, VoiceSide)> = Vec::new();
    let mut buckets = Buckets::default();
    // close 触发归因（与 buckets.close 同步 push，一一对应）。
    let mut close_triggers: Vec<Candidate> = Vec::new();
    // 每条 close 桶成员的 active 索引——⑤ 归因序重排键（仅 external 非空时消费）。
    let mut close_active_idx: Vec<usize> = Vec::new();

    // ②' 规则2 的**外部输入源**：external 预置关闭（channel P2/P3 域裁决）。空切片 ⟹ 整段
    //    空转、working 全 false ⟹ 下方循环即纯候选驱动本体。
    let mut ext_trigger_keys: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for &(i, trig) in external_closes {
        let (leg, was_closed) = working.get_mut(i).map(|(l, c)| (*l, c)).unwrap_or_else(|| {
            panic!(
                "external 关闭索引越界：active_idx={i} ≥ |active|={}",
                active.len()
            )
        });
        debug_assert!(
            !*was_closed,
            "external 关闭重复喂入同一腿（active_idx={i}）"
        );
        *was_closed = true;
        buckets.close.push(leg);
        close_triggers.push(trig);
        close_active_idx.push(i);
        ext_trigger_keys.insert(trig.gamma_index);
    }

    // ③ ponytail: H8 预索引——level → legs idx 列表（reverse_signal 需逐腿判 bits，无法纯 key 查表；
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

    // ④ 确定性 fold 主循环（规则1/证书门/规则2/规则3/规则4 + #200 dual-effect）。
    for &c in &ordered {
        // 规则1：非方向候选 ⟹ 𝒦_x。
        if c.dir == VoiceSide::Flat || c.bsp_class == u8::MAX {
            buckets.record.push(*c);
            continue;
        }
        // 规则2 证书门：§9 closePred 的 χ 只消费证书成立层已确认的 N^δ；本层只读字段，
        // 不重复计算区间套/背驰。未确认者不能落规则3（§13 ℬ_x 会真实开腿），故归 𝒦_x record。
        if !c.nest_confirmed {
            buckets.record.push(*c);
            continue;
        }
        // 规则2：反向关闭 A_t 中同级别活动腿（reverse_signal 复用 §9 closePred 反向项）。
        // ponytail: H8 查 level→idx 列表，逐腿判 reverse_signal（bits 不可 key 化，须逐腿）。
        // ★#209（S7：级别内全平是必须非应当，用户裁 A 2026-07-23；spec ID-3「一类卖点：
        // 当场全平（该级该方向全部清仓）」）：**一类**候选关闭该级别**全部**未关闭反向
        // 命中腿（Ambient 根 / FollowParent 级联 / §13 restore 祖先一视同仁——fold 层不
        // 区分账户，账户归属由 `identity_of` × reason 单源承担）；二/三类维持 find 首个
        // （bit-exact：取首个未关闭且 reverse_signal 命中者 == 旧 working.iter().position）。
        // 外部输入源（②'）已裁决的腿由 `!was_closed` 自动跳过；其触发候选经 ext_trigger_keys
        // 播种 closed_any（channel 已消费 ⟹ 一类消费即止、二类 dual-effect 照常）。
        let mut closed_any = ext_trigger_keys.contains(&c.gamma_index);
        if c.bsp_class == 1 {
            if let Some(idxs) = level_idx.get(&c.level) {
                for &i in idxs {
                    let (leg, was_closed) = (working[i].0, working[i].1);
                    if !was_closed && reverse_signal(leg.dir, &c.bits) {
                        // 标记关闭（不从索引移除——bit-exact：旧 position 也跳过已关闭腿）。
                        working[i].1 = true;
                        buckets.close.push(leg);
                        close_triggers.push(*c); // 归因：每腿恰一（一一对应不变量保持）
                        close_active_idx.push(i);
                        closed_any = true;
                    }
                }
            }
        } else if let Some(pos) = level_idx.get(&c.level).and_then(|idxs| {
            idxs.iter().copied().find(|&i| {
                let (leg, closed) = &working[i];
                !*closed && reverse_signal(leg.dir, &c.bits)
            })
        }) {
            // 标记关闭（不从索引移除——bit-exact：旧 working.iter().position 也跳过已关闭腿）。
            working[pos].1 = true;
            buckets.close.push(working[pos].0);
            close_triggers.push(*c); // 归因：本腿由候选 c 反向关闭（typed exit 原料）
            close_active_idx.push(pos);
            closed_any = true;
        }
        // ★#200 OpenShort 通道（spec WP-2 修复 c / #185 审计发现 2「最早单源接入点」）：
        // **二类**反向候选「先平后开」——被 close 分支消费后不 `continue`，落入下方
        // 规则3/4 同款 slot 判据补开反向腿（ID-3：二类点 = 第二入场/加仓（加空）位；
        // 反手机制 = 先平后开 `A_raw=(A_t∖D_t)∪O_t`，买卖点2 p.6/14 §9）。一/三类维持
        // 消费即止（bit-exact 不动：一类「允许当场反手」是允许非要求，v1 不反手）。
        // 账户归属（父 active⇒短差账 / 无父⇒空仓账的 ambient 守卫）由账户轴
        // `identity_of` × `reason_of_open` 单源承担，本 fold 只放行候选、不另立判据。
        if closed_any && c.bsp_class != 2 {
            continue;
        }
        // 规则3/4：开启 vs 记录（slot = (level, σ_g)）。
        // ponytail: H8 slot_in_at 用 level 索引查同 level 腿里是否有未关闭且 dir==c.dir 者
        // == 旧 working.iter().any(|(leg,closed)| !closed && leg.level==c.level && leg.dir==c.dir)。
        let slot_in_at = level_idx.get(&c.level).is_some_and(|idxs| {
            idxs.iter()
                .any(|&i| !working[i].1 && working[i].0.dir == c.dir)
        });
        let slot_this_fold = opened.iter().any(|&(lv, d)| lv == c.level && d == c.dir);
        if !slot_in_at && !slot_this_fold {
            buckets.open.push(*c); // 规则3：开启
            opened.push((c.level, c.dir));
        } else {
            buckets.record.push(*c); // 规则4：冲突/重复 ⟹ 记录不执行
        }
    }

    // ⑤ 归因序重排：(≺_Θ(trigger), active_idx) 稳定序——与纯候选驱动归因序（候选遍历主序 +
    //    同候选内 active 次序）同构。跨候选无平局**由前置条件 gamma_index 单射给出**（⓪ 守卫），
    //    不是 theta_key 自身的性质；同候选多腿（#209 一类全平 / channel 多腿同触发）由
    //    active_idx 保 active 次序。
    //    external 为空 ⟹ push 序已然满足该序，跳过（见 fn doc「⑤ 归因序重排的适用域」）。
    if !external_closes.is_empty() {
        let mut order: Vec<usize> = (0..buckets.close.len()).collect();
        order.sort_by_key(|&k| (theta_key(&close_triggers[k]), close_active_idx[k]));
        let sorted_close: Vec<ActiveLeg> = order.iter().map(|&k| buckets.close[k]).collect();
        let sorted_triggers: Vec<Candidate> = order.iter().map(|&k| close_triggers[k]).collect();
        buckets.close = sorted_close;
        close_triggers = sorted_triggers;
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
    use super::super::super::super::classifier::bsp::BspPoint;
    use super::super::super::super::classifier::{Classification, LevelState};
    use super::super::super::super::types::{BspBits, Center};
    use super::super::assemble_gamma;
    use super::*;
    use std::rc::Rc;

    fn buy_point(source_index: usize, class: u8) -> BspPoint {
        let bits = match class {
            1 => BspBits {
                buy1: true,
                ..Default::default()
            },
            2 => BspBits {
                buy2: true,
                ..Default::default()
            },
            _ => BspBits {
                buy3: true,
                ..Default::default()
            },
        };
        BspPoint {
            source_index,
            bits,
            pivot_low: 90,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center {
                zd: 100,
                zg: 200,
                dd: 90,
                gg: 210,
                start_index: 0,
                end_index: 9,
            })),
            struct_break_dir: None,
            force: None,
            retrace_breaks_type1: None,
        }
    }

    fn sell_point(source_index: usize, class: u8) -> BspPoint {
        let bits = match class {
            1 => BspBits {
                sell1: true,
                ..Default::default()
            },
            2 => BspBits {
                sell2: true,
                ..Default::default()
            },
            _ => BspBits {
                sell3: true,
                ..Default::default()
            },
        };
        BspPoint {
            source_index,
            bits,
            pivot_low: 0,
            pivot_high: 210,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center {
                zd: 100,
                zg: 200,
                dd: 90,
                gg: 210,
                start_index: 0,
                end_index: 9,
            })),
            struct_break_dir: None,
            force: None,
            retrace_breaks_type1: None,
        }
    }

    fn classification(levels: Vec<Vec<BspPoint>>) -> Classification {
        Classification {
            levels: levels
                .into_iter()
                .map(|bsp| LevelState {
                    bsp: Rc::new(bsp),
                    ..Default::default()
                })
                .collect(),
        }
    }

    /// 测试用 ActiveLeg 构造器（默认 is_boundary_root=true 真边界根 ∂，parent_id=None）。
    fn aleg(level: u32, dir: VoiceSide, source_index: usize, lambda: usize) -> ActiveLeg {
        ActiveLeg {
            level,
            dir,
            source_index,
            lambda,
            id: ElementId {
                level,
                ordinal: source_index as u64,
            },
            parent_id: None,
            is_boundary_root: true,
            op_parent: None,
        }
    }

    /// ★环5 𝒟_x 非空（解释器关闭机制真实坐实，非 stub）：持仓 Long 活动腿遇反向卖候选 ⟹ 关闭。
    #[test]
    fn interpret_reverse_candidate_closes_active_leg() {
        let gamma = assemble_gamma(&classification(vec![vec![sell_point(10, 1)]]));
        let active = [aleg(0, VoiceSide::Long, 0, 0)];
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
        let active = [
            aleg(0, VoiceSide::Long, 0, 0),
            aleg(1, VoiceSide::Long, 2, 2),
        ];
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

    /// ★#209（S7：级别内全平是必须非应当，用户裁 A 2026-07-23）：**一类**反向候选
    /// 关闭该级别**全部**反向本仓腿（Ambient 根 / FollowParent 级联 / §13 restore 祖先
    /// 一视同仁——fold 层不区分账户，账户归属由 `identity_of` × reason 单源承担）。
    /// 每腿恰一触发归因（close 桶与 close_triggers 一一对应不变量保持）。
    #[test]
    fn type1_candidate_closes_all_same_level_reverse_legs() {
        // 同级别两条核心腿（Ambient 根 + 级联形态，fold 层只见级别/方向）+ 一类卖候选。
        let gamma = assemble_gamma(&classification(vec![vec![sell_point(10, 1)]]));
        let active = [
            aleg(0, VoiceSide::Long, 0, 0),
            aleg(0, VoiceSide::Long, 2, 2),
        ];
        let (b, triggers) = interpret_with_close_triggers(&gamma, &active);
        assert_eq!(
            b.close.len(),
            2,
            "一类候选 ⟹ 该级别全部反向腿全关（S7 全平）"
        );
        assert_eq!(triggers.len(), 2, "每腿恰一触发归因（一一对应不变量）");
        assert!(
            triggers.iter().all(|t| t.bsp_class == 1 && t.level == 0),
            "两腿归因同一一类候选"
        );
        assert!(
            b.open.is_empty(),
            "一类消费即止（允许反手非要求，v1 不反手）"
        );
        // 委托单源 bit-exact：interpret == interpret_with_close_triggers.0。
        let b2 = interpret(&gamma, &active);
        assert_eq!(b.close, b2.close);
    }

    /// ★#209 对照锁（行为面严格限定「多腿场景一类全平」）：二/三类维持 find 首个——
    /// 二类先平后开（#200 OpenShort 通道不动）、三类消费即止，单腿外的行为面零扩张。
    #[test]
    fn type2_type3_still_close_first_leg_only() {
        for class in [2u8, 3] {
            let gamma = assemble_gamma(&classification(vec![vec![sell_point(10, class)]]));
            let active = [
                aleg(0, VoiceSide::Long, 0, 0),
                aleg(0, VoiceSide::Long, 2, 2),
            ];
            let (b, triggers) = interpret_with_close_triggers(&gamma, &active);
            assert_eq!(
                b.close.len(),
                1,
                "{class} 类仍只关首条同级反向腿（find 首个）"
            );
            assert_eq!(b.close[0].source_index, 0, "{class} 类关的是首条命中腿");
            assert_eq!(triggers.len(), 1);
            if class == 2 {
                assert_eq!(
                    b.open.len(),
                    1,
                    "二类先平后开（#200 OpenShort 通道不受影响）"
                );
                assert_eq!(b.open[0].dir, VoiceSide::Short, "补开反向腿");
            } else {
                assert!(b.open.is_empty(), "三类消费即止（不补开）");
            }
        }
    }

    // ── ★#202 阶段 C：规则2 外部化 fold 变体（`interpret_with_external_closes`）────────

    /// external 预置关闭（channel P2/P3 域裁决）⟹ 该腿跳过规则2；一类候选的剩余
    /// 反向命中腿（S 组散装域）仍由 fold 全平关闭；归因序 = (≺_Θ 触发, active 次序)。
    #[test]
    fn external_closes_preset_leg_skips_rule2_rest_close_by_fold() {
        // 同级同向两 Long 腿 + 一类卖候选：idx0 经 external（channel 裁决）预置关闭，
        // idx1（散装域剩余腿）由 fold 规则2 一类全平关闭——两腿归因同一候选。
        let gamma = assemble_gamma(&classification(vec![vec![sell_point(10, 1)]]));
        let active = [
            aleg(0, VoiceSide::Long, 0, 0),
            aleg(0, VoiceSide::Long, 2, 2),
        ];
        let external = [(0usize, gamma[0])];
        let (b, triggers) = interpret_with_external_closes(&gamma, &active, &external);
        assert_eq!(b.close.len(), 2, "external 腿 + fold 规则2 剩余腿全关");
        assert_eq!(triggers.len(), 2, "每腿恰一触发归因（一一对应不变量）");
        assert!(
            triggers.iter().all(|t| *t == gamma[0]),
            "两腿归因同一一类候选"
        );
        assert_eq!(b.close[0].source_index, 0, "同候选内归因序 = active 次序");
        assert_eq!(b.close[1].source_index, 2);
        assert!(b.open.is_empty(), "一类消费即止（v1 不反手）");
    }

    /// external 触发候选 = 二类 ⟹ 「先平后开」dual-effect 照常（#200 OpenShort 通道在
    /// channel 判据下等价成立：被 channel 消费的二类候选落入规则3/4 补开反向腿）。
    /// 对照：一类 external 触发 ⟹ 消费即止不开。
    #[test]
    fn external_type2_trigger_keeps_dual_effect_open() {
        let gamma2 = assemble_gamma(&classification(vec![vec![sell_point(10, 2)]]));
        let active = [aleg(0, VoiceSide::Long, 0, 0)];
        let external = [(0usize, gamma2[0])];
        let (b, triggers) = interpret_with_external_closes(&gamma2, &active, &external);
        assert_eq!(b.close.len(), 1);
        assert_eq!(triggers.len(), 1);
        assert_eq!(
            b.open.len(),
            1,
            "二类 external 触发 ⟹ dual-effect 补开反向腿"
        );
        assert_eq!(b.open[0].dir, VoiceSide::Short);

        let gamma1 = assemble_gamma(&classification(vec![vec![sell_point(10, 1)]]));
        let external1 = [(0usize, gamma1[0])];
        let (b1, _) = interpret_with_external_closes(&gamma1, &active, &external1);
        assert_eq!(b1.close.len(), 1);
        assert!(b1.open.is_empty(), "一类 external 触发 ⟹ 消费即止");
    }

    /// 归因序跨级别：≺_Θ 主序（高 level 先）——external 腿（L1，触发 sell3@L1）排在
    /// fold 关闭腿（L0，触发 sell1@L0）之前，与原 fold 候选遍历归因序同构。
    #[test]
    fn external_closes_attribution_order_matches_theta_traversal() {
        let gamma = assemble_gamma(&classification(vec![
            vec![sell_point(10, 1)], // L0 一类卖（θ 后）
            vec![sell_point(12, 3)], // L1 三类卖（θ 先：高 level 先）
        ]));
        let active = [
            aleg(0, VoiceSide::Long, 0, 0),
            aleg(1, VoiceSide::Long, 2, 2),
        ];
        // L1 腿（idx1）经 external 预置关闭（触发 sell3@L1）；L0 腿由 fold 规则2 关。
        let external = [(1usize, gamma[1])];
        let (b, triggers) = interpret_with_external_closes(&gamma, &active, &external);
        assert_eq!(b.close.len(), 2);
        assert_eq!(b.close[0].level, 1, "≺_Θ 主序：高 level 触发归因在前");
        assert_eq!(triggers[0], gamma[1]);
        assert_eq!(b.close[1].level, 0);
        assert_eq!(triggers[1], gamma[0]);
    }

    // ── ★#396 规则体 golden 锁 + 前置条件守卫 ──────────────────────────────────────

    /// golden 场景：跨级别 5 候选 × 4 活动腿，一趟 fold 打穿规则1..4 + #200 + #209。
    ///
    /// `gamma_index`：0=buy3@L0(si0) 1=sell1@L0(si10) 2=buy2@L0(si4) 3=sell2@L1(si12)
    /// 4=buy3@L1(si6)；≺_Θ 遍历序 = [g3, g4, g1, g2, g0]（Reverse(level) 高级别先，同级按 class）。
    fn golden_scenario() -> (Vec<Candidate>, Vec<ActiveLeg>) {
        let gamma = assemble_gamma(&classification(vec![
            vec![buy_point(0, 3), sell_point(10, 1), buy_point(4, 2)],
            vec![sell_point(12, 2), buy_point(6, 3)],
        ]));
        let active = vec![
            aleg(0, VoiceSide::Long, 0, 0),
            aleg(0, VoiceSide::Long, 2, 2),
            aleg(1, VoiceSide::Long, 5, 5),
            aleg(1, VoiceSide::Short, 7, 7),
        ];
        (gamma, active)
    }

    /// 三桶 + 归因的**完整**形状（close 取 (level, source_index)，其余取 gamma_index）。
    #[allow(clippy::type_complexity)]
    fn fold_shape(
        b: &Buckets,
        t: &[Candidate],
    ) -> (Vec<(u32, usize)>, Vec<usize>, Vec<usize>, Vec<usize>) {
        (
            b.close.iter().map(|l| (l.level, l.source_index)).collect(),
            t.iter().map(|c| c.gamma_index).collect(),
            b.open.iter().map(|c| c.gamma_index).collect(),
            b.record.iter().map(|c| c.gamma_index).collect(),
        )
    }

    /// ★#396 规则体 golden 锁：把 fold 的**完整**三桶 + 归因期望值钉死。
    ///
    /// 存在理由：合并后 `external_closes_empty_is_bit_exact_with_plain_fold` 退化为
    /// `f(x)==f(x)`（两个 pub 驱动同一实现同一实参路径），不可能失败 ⟹ 它不再锁住任何东西。
    /// 本测试接替它：期望值取自合并**前**的实现（b7dc9b544d，两个驱动分别跑同一场景所得），
    /// 故它锁的是「合并后的规则体仍算出合并前的结果」，而非两驱动之间的自洽。
    ///
    /// 覆盖（一趟 fold 全打到）：≺_Θ 跨级别主序 / 规则2 二三类 find-first / #200 二类先平后开
    /// 且补开位被占时落规则4 / 规则2 三类消费即止 / #209 一类全平多腿 / 规则3 开启 /
    /// 规则4 本 fold 同 slot 重复 / external 预置关闭改变 dual-effect 的 slot 可用性 /
    /// ⑤ 归因序重排（external 分支 push 序 [g4,g3,..] 被重排为 [g3,g4,..]，非平凡）。
    #[test]
    fn fold_theta_golden_buckets_pin_rule_body() {
        let (gamma, active) = golden_scenario();

        // ── 驱动A：规则2 纯候选驱动（external = &[]）
        // g3(sell2@L1) 关 leg2 后二类续跑，但 L1 Short slot 被未关闭的 leg3 占 ⟹ 落规则4 record；
        // g4(buy3@L1) 关 leg3 三类消费即止；g1(sell1@L0) #209 全平 leg0+leg1；
        // g2(buy2@L0) 开 L0/Long；g0(buy3@L0) 同 slot 本 fold 已开 ⟹ record。
        let (b, t) = interpret_with_close_triggers(&gamma, &active);
        assert_eq!(
            fold_shape(&b, &t),
            (
                vec![(1, 5), (1, 7), (0, 0), (0, 2)],
                vec![3, 4, 1, 1],
                vec![2],
                vec![3, 0],
            ),
            "纯候选驱动 golden 漂移"
        );

        // ── 驱动B：external 预置关闭 leg3（触发 g4），其余同场景
        // leg3 被 channel 先关 ⟹ g3 的 #200 补开不再被 Short slot 挡住，真的开出 g3；
        // g4 经 ext_trigger_keys 计入 closed_any ⟹ 三类消费即止不重复关。
        // push 序 [(g4,idx3), (g3,idx2), (g1,idx0), (g1,idx1)]
        //   ──⑤ 重排──▶ [(g3,idx2), (g4,idx3), (g1,idx0), (g1,idx1)]（g3≺_Θ g4）
        let (b2, t2) = interpret_with_external_closes(&gamma, &active, &[(3usize, gamma[4])]);
        assert_eq!(
            fold_shape(&b2, &t2),
            (
                vec![(1, 5), (1, 7), (0, 0), (0, 2)],
                vec![3, 4, 1, 1],
                vec![3, 2],
                vec![0],
            ),
            "external 驱动 golden 漂移"
        );
    }

    /// ★#396 前置条件守卫：`gamma_index` 在 `gamma` 上非单射 ⟹ ⓪ 守卫当场响。
    ///
    /// 复现评审给出的危险路径：`selector.rs:575/636`、`runner.rs:4355`、`exit.rs:517` 的测试
    /// 构造器硬编码 `gamma_index: 0`，拿它们拼多候选 gamma 就会让 ≺_Θ 出现平局，close 归因序
    /// 不再由契约确定。守卫把「静默偏离」变成「当场 panic」。
    ///
    /// 仅 debug 构建存在：release 下 `debug_assert!` 被编译掉，本测试无对象。
    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "非单射")]
    fn fold_theta_rejects_non_injective_gamma_index() {
        let mut gamma = assemble_gamma(&classification(vec![vec![
            buy_point(0, 3),
            sell_point(10, 1),
        ]]));
        gamma[1].gamma_index = gamma[0].gamma_index; // 模拟硬编码 gamma_index: 0
        let _ = interpret_with_close_triggers(&gamma, &[aleg(0, VoiceSide::Long, 0, 0)]);
    }

    /// 边界：external 为空 ⟹ 与 `interpret_with_close_triggers` 逐字段 bit-exact。
    ///
    /// **#396 后本测试已退化**：两个 pub 驱动同一实现同一实参路径 ⟹ 恒真，不可能失败。保留
    /// 为「两驱动不得再分叉」的结构声明，规则体的真锁见
    /// [`fold_theta_golden_buckets_pin_rule_body`]。
    #[test]
    fn external_closes_empty_is_bit_exact_with_plain_fold() {
        let gamma = assemble_gamma(&classification(vec![
            vec![buy_point(0, 3), sell_point(10, 1)],
            vec![sell_point(12, 2)],
        ]));
        let active = [
            aleg(0, VoiceSide::Long, 0, 0),
            aleg(1, VoiceSide::Long, 2, 2),
        ];
        let (b0, t0) = interpret_with_close_triggers(&gamma, &active);
        let (b1, t1) = interpret_with_external_closes(&gamma, &active, &[]);
        assert_eq!(b0.close, b1.close);
        assert_eq!(t0, t1);
        assert_eq!(
            b0.open.iter().map(|c| c.gamma_index).collect::<Vec<_>>(),
            b1.open.iter().map(|c| c.gamma_index).collect::<Vec<_>>()
        );
        assert_eq!(
            b0.record.iter().map(|c| c.gamma_index).collect::<Vec<_>>(),
            b1.record.iter().map(|c| c.gamma_index).collect::<Vec<_>>()
        );
    }

    /// ★G4/G5 单源判据 [`reverse_exit_type`]（PDF §9 / G5 映射 §6.1）：
    /// ReverseOpen 腿→P7；三类反向→P6；一/二类反向→P5（二类归 CloseRoot 读法）。
    #[test]
    fn reverse_exit_type_criteria_table() {
        use ExitType::*;
        // ReverseOpen 腿：任何触发类都是 CloseReverseOpen（子声部关闭语义压过触发类）。
        assert_eq!(
            reverse_exit_type(Vertical::ReverseOpen, 1),
            CloseReverseOpen
        );
        assert_eq!(
            reverse_exit_type(Vertical::ReverseOpen, 3),
            CloseReverseOpen
        );
        // 根腿（Ambient）：三类反向 → ReduceCore；一/二类 → CloseRoot。
        assert_eq!(reverse_exit_type(Vertical::Ambient, 3), ReduceCore);
        assert_eq!(reverse_exit_type(Vertical::Ambient, 1), CloseRoot);
        assert_eq!(reverse_exit_type(Vertical::Ambient, 2), CloseRoot);
        // FollowParent 子腿：按触发类走 P5/P6（非短差对冲腿）。
        assert_eq!(reverse_exit_type(Vertical::FollowParent, 3), ReduceCore);
        assert_eq!(reverse_exit_type(Vertical::FollowParent, 1), CloseRoot);
    }

    /// ★#181 迁入的 type1>type3 优先级单源 [`exit_type_of_classes`]（自 closed_loop/sell.rs
    /// `sell_decision_of` 迁移，分支语义逐条不变）：一类 → CloseRoot（优先）；三类（无一类）
    /// → ReduceCore；皆否 → Hold；一类+三类共存 → CloseRoot（镜像 Lean 分支顺序）。
    #[test]
    fn exit_type_of_classes_priority_table() {
        use ExitType::*;
        assert_eq!(
            exit_type_of_classes(true, false),
            CloseRoot,
            "一类 → CloseRoot"
        );
        assert_eq!(
            exit_type_of_classes(false, true),
            ReduceCore,
            "三类（无一类）→ ReduceCore"
        );
        assert_eq!(exit_type_of_classes(false, false), Hold, "皆否 → Hold");
        assert_eq!(
            exit_type_of_classes(true, true),
            CloseRoot,
            "一类+三类共存 → CloseRoot（一类优先）"
        );
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
        let gamma = assemble_gamma(&classification(vec![vec![
            buy_point(0, 3),
            buy_point(3, 3),
        ]]));
        let b = interpret(&gamma, &[]);
        assert_eq!(b.open.len(), 1, "同 slot 仅一个开启（唯一化）");
        assert_eq!(b.record.len(), 1, "重复同向候选记录不执行");
    }

    /// 环5 𝒦_x：持仓同向候选（已有 Long 腿遇买候选）⟹ 记录不执行（不重复开同向）。
    #[test]
    fn interpret_same_dir_as_active_records() {
        let gamma = assemble_gamma(&classification(vec![vec![buy_point(0, 1)]]));
        let active = [aleg(0, VoiceSide::Long, 0, 0)];
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
            bits: BspBits {
                buy1: true,
                sell1: true,
                ..Default::default()
            },
            pivot_low: 90,
            pivot_high: 210,
            center: None,
            struct_break_dir: None,
            force: None,
            retrace_breaks_type1: None,
        };
        let gamma = assemble_gamma(&classification(vec![vec![both]]));
        let b = interpret(&gamma, &[]);
        assert_eq!(b.record.len(), 1);
        assert!(b.open.is_empty() && b.close.is_empty());
    }

    /// ★∃! 唯一性（spec §12 line 627）：输入顺序无关——interpret(Γ,A)=interpret(perm(Γ),A)。
    #[test]
    fn interpret_deterministic_order_independent() {
        let g1 = assemble_gamma(&classification(vec![
            vec![buy_point(0, 3)],
            vec![sell_point(5, 2)],
        ]));
        // 反序 Γ（同候选集，不同输入顺序）。
        let mut g2 = g1.clone();
        g2.reverse();
        let active = [aleg(1, VoiceSide::Long, 0, 0)];
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
        assert_eq!(
            before,
            theta_lt(&s0, &s1),
            "≺_Θ 在级别平移 S_k 下不变（自相似全序）"
        );
    }

    /// 环5 互斥分流：每个候选恰落一桶（ℬ⊎𝒦 over 候选；𝒟 over 活动腿）。计数守恒。
    #[test]
    fn interpret_buckets_partition_candidates() {
        let gamma = assemble_gamma(&classification(vec![vec![
            buy_point(0, 3),
            buy_point(3, 3),  // 同向重复 → record
            sell_point(7, 1), // 反向 → 关闭活动 Long
        ]]));
        let active = [aleg(0, VoiceSide::Long, 0, 0)];
        let b = interpret(&gamma, &active);
        // 每个候选恰落 open 或 record（关闭触发候选被消费，不入 open/record）。
        let consumed_as_close = b.close.len(); // 反向候选数（消费为关闭）
        assert_eq!(
            b.open.len() + b.record.len() + consumed_as_close,
            gamma.len(),
            "候选守恒：open + record + 关闭触发 = |Γ|"
        );
    }

    /// 测试用 ElementId。
    fn eid(level: u32, ordinal: u64) -> ElementId {
        ElementId { level, ordinal }
    }

    /// ★A9（Task #166，级别容器.pdf p14/§13）：position instance 四元组身份区分性 + hash 确定性。
    ///
    /// 验收核心：**同一 carrier 两次 campaign 不混淆**——carrier/entry_certificate/side 全同、仅
    /// generation 异 ⟹ 四元组不等 ⟹ hash64 不等（carrier-id 单值会把两次 campaign 混为一谈，
    /// 四元组按 generation 区分）。
    #[test]
    fn position_node_id_distinguishes_campaigns_by_generation() {
        let carrier = eid(0, 5);
        let cert = Some(EntryCertificate {
            level: 0,
            source_index: 3,
        });
        let gen0 = PositionNodeId {
            carrier,
            entry_certificate: cert,
            side: VoiceSide::Long,
            generation: 0,
        };
        let gen1 = PositionNodeId {
            carrier,
            entry_certificate: cert,
            side: VoiceSide::Long,
            generation: 1,
        };
        // carrier-id 层：两次 campaign 共享同一 carrier（混淆源）。
        assert_eq!(
            gen0.carrier, gen1.carrier,
            "两次 campaign 同 carrier（carrier-id 会混淆）"
        );
        // 四元组层：generation 区分 ⟹ 不同 position instance ⟹ hash 不碰撞。
        assert_ne!(gen0, gen1, "generation 异 ⟹ 四元组不等（campaign 不混淆）");
        assert_ne!(
            gen0.hash64(),
            gen1.hash64(),
            "generation 异 ⟹ posId hash 不碰撞"
        );
        // side/entry_certificate 也各自区分（四元组完整性）。
        let short = PositionNodeId {
            carrier,
            entry_certificate: cert,
            side: VoiceSide::Short,
            generation: 0,
        };
        assert_ne!(gen0.hash64(), short.hash64(), "side 异 ⟹ posId 不碰撞");
        let cert2 = Some(EntryCertificate {
            level: 0,
            source_index: 9,
        });
        let entry2 = PositionNodeId {
            carrier,
            entry_certificate: cert2,
            side: VoiceSide::Long,
            generation: 0,
        };
        assert_ne!(
            gen0.hash64(),
            entry2.hash64(),
            "entry_certificate 异 ⟹ posId 不碰撞"
        );
    }

    /// ★A9：hash64 确定性（`DefaultHasher::new()` 固定初始键 ⟹ 同四元组跨调用产同 u64，非 RandomState）。
    #[test]
    fn position_node_id_hash_is_deterministic() {
        let p = PositionNodeId {
            carrier: eid(1, 2),
            entry_certificate: Some(EntryCertificate {
                level: 1,
                source_index: 7,
            }),
            side: VoiceSide::Short,
            generation: 3,
        };
        assert_eq!(p.hash64(), p.hash64(), "同四元组两次 hash64 一致（确定性）");
        let p_copy = p;
        assert_eq!(p.hash64(), p_copy.hash64(), "值拷贝 hash 一致");
    }
}
