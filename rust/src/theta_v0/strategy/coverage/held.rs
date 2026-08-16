use super::*;

// ════════════════════════════════════════════════════════════════════════════
//  §7 已删除（GAP-5 收口，no-patch）：旧 `coverage_step`（λ_e 走势边界 per-bar 入场五步合成）
//  曾自称"生产引擎核心入口"，但经 [`active_set_step`] 在 `B_t={e:λ_e=t}` 入场——λ_e=
//  `LeveledMove.start_index`=**走势边界**，正是 GAP-5 判定的错误入场源。spec §13 正典递归用
//  解释器三桶 𝒟_x/ℬ_x=ℛ_Θ(Γ(x))（**买卖点 Γ**），生产入场已收口至 §8 买卖点路径 + §9 π_Θ。
//  （框架纠偏 MEMORY coverage-engine-needs-tower-export-bridge：互斥全定义策略=**买卖点入场**+
//  多级角色/嵌套对冲，**非每元素覆盖**。删除 λ_e 入场组装层，保留 §3 区间递归原语作 Lean 对齐。）
// ════════════════════════════════════════════════════════════════════════════
//  §8 环6：解释器三桶 → 活动集 A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x∪RegistryRestore] → 目标头寸 p̃_{t+1}
//        （RegistryRestore=registry 持久祖先物化，anc.pdf §11 显式第三来源，票#247）
//        （spec §13 line 1172 活动集 + §14 line 1243 头寸，七链 **环6** rust 兑现）
// ════════════════════════════════════════════════════════════════════════════

/// 持仓腿 [`ActiveLeg`] → **当前因果树元素索引**（codex Q4：按 `ElementId` 结构映射匹配，spec §13）。
///
/// ★codex Q4 严格修复（替代值比较 `(level,ρ,eps)`/`(level,λ,eps)` hack）：
/// 按 `leg.id`（跨 bar 稳定的确定性 ElementId）在当前因果树前缀查同 ID 元素。
/// - 找到 → [`HeldLegMatch::Exact`]：持仓腿覆盖的走势在当前因果树在场 ⟹ 其子声部腿的 AncOK
///   祖先齐全（[`ancestor_close`] 保留）。ID 确定性 ⟹ 父延伸（ρ 漂移）仍同 ID ⟹ 不需 CoordDrift 分支。
/// - 未找到 → [`HeldLegMatch::Stale`]：持仓腿走势不在当前前缀因果树（结构演化/不在前缀）。
///   调用方按 `is_boundary_root` 决定：真边界根 ∂ 作根保留，非边界根 prune（发现 A 修复）。
///
/// 只在**真嵌套树元素段** `elements[0..candidate_start)` 查（候选段是本 bar 新触发候选，非持仓）。
///
/// ★真 Fugue 铁律（mod.rs:433-440 codex 裁旧 bug）：父子来自**真 Compose 塔**（树元素由
/// [`extract_elements`] 的 `sub_moves` 真嵌套建，非级别差伪造），本函数按 ID 对位，**不**伪造父子。
///
/// ## 发现 B 归因修正（codex Q4）
/// 旧值比较 `(level,λ,eps)` 的"λ 漂移"归因错误——λ=start_index 在 confirmed 前缀不回写时不变。
/// 真因 = `extract_elements` 每 bar 重建 Vec + 更高级新出现时根结构重构索引重映射 + 值比较非 spec §13
/// 结构映射。Q4 修复：确定性 ElementId 跨 bar 稳定（全量/增量产同 ID），按 ID 匹配非值比较。
fn held_leg_tree_index(
    elements: &[CoverageElement],
    candidate_start: usize,
    leg: &ActiveLeg,
    registry: &super::super::persistent::PersistentRegistry,
) -> HeldLegMatch {
    let tree_end = candidate_start.min(elements.len());
    let tree = &elements[..tree_end];
    let id_idx = build_tree_id_index(tree);
    held_leg_tree_index_indexed(tree, leg, &id_idx, registry)
}

/// ponytail: H6 带预建索引的 held_leg_tree_index 变体——热循环 coverage_step_from_buckets 单次建、多次查。
/// codex Q4：按 `leg.id` 查表（结构映射），删除 CoordDrift 分支（ID 确定性 ⟹ 无需 λ 稳定性 hack）。
pub(super) fn held_leg_tree_index_indexed(
    tree: &[CoverageElement],
    leg: &ActiveLeg,
    id_idx: &std::collections::HashMap<ElementId, usize>,
    registry: &super::super::persistent::PersistentRegistry,
) -> HeldLegMatch {
    // 按 ElementId 结构映射匹配（spec §13 p:C_ℓ→C_{ℓ+1}）。
    if let Some(&idx) = id_idx.get(&leg.id) {
        // ★#269 翻向守卫**事件化**（#261 终裁选项 A，替代 #233 σ/ε 两轴状态对立判据）：
        // 只在「腿存活期间载体发生真实结构翻向**事件**」才判 Flipped——事件谓词 =
        // registry 首见方向（I2 机器锁永固）≠ 当前树元素方向 ⟺ 树段 upsert 方向冲突
        // （载体被 frontier 重组改判），枚举见 [`super::super::persistent::PersistentRegistry::
        // direction_flip_event_active`]。**出生对立不再触发**：BSP 构造使买点恒附下降段
        // 末端、卖点恒附上升段末端 ⟹ σ=−ε 入场即恒真（#264 §2.1 构造性证明），是两轴
        // 出生分层而非事件——#233 判据（tree.eps ≠ leg.dir）据此 t+1 必剪（wf8 92.3%
        // 交易持仓恰好 1 bar 的接线缺陷）。事件口径下无事件的出生对立腿 Exact 存活并随
        // 载体结构方向重登记（`element_as_leg` 以树元素方向重建——#233 前基线语义恢复；
        // 方向盲复活在事件缺席处合法，在事件在场处仍废除）。**frontier 翻向同按终结**
        // （#227 焊缝规则保留：守卫不区分 frontier/confirmed——凡翻向事件即终结）。
        if registry.direction_flip_event_active(&leg.id, tree[idx].eps) {
            return HeldLegMatch::Flipped;
        }
        // ID 命中 ⟹ 同一走势（父延伸也同 ID）。更新 rho/source_index 从当前元素由调用方处理
        //（element_as_leg 重读当前元素坐标）。守卫：当前 rho >= 旧 source_index（父延伸不变量）。
        if tree[idx].rho >= leg.source_index {
            return HeldLegMatch::Exact(idx);
        }
        // rho 守卫失败（理论不可达——ID 确定性 ⟹ 同走势 rho 单调增）⟹ 仍作 Exact（ID 即身份）。
        return HeldLegMatch::Exact(idx);
    }
    // ID 未匹配 ⟹ 父真失效（走势不在当前前缀因果树）。
    HeldLegMatch::Stale
}

/// 持仓腿跨 bar 对位结果（codex Q4：ID 结构映射二分）。
///
/// `Exact` 对位回**当前因果树元素 idx**（携真父链，AncOK 祖先齐全判据有效）；
/// `Stale` 由调用方按 `is_boundary_root` 决定：真边界根 ∂ 作根保留，非边界根 prune（发现 A 修复）。
///
/// ★codex Q4 删除 `CoordDrift` 分支：ID 确定性 ⟹ 父延伸（ρ 漂移）仍同 ID ⟹ Exact 直接覆盖，
/// 无需 λ 稳定性 hack（值比较 `(level,λ,eps)` 已弃用，spec §13 结构映射对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HeldLegMatch {
    /// ID 命中（跨 bar 同走势，父延伸也同 ID）且**无翻向事件**（#269 事件化守卫后 Exact 的
    /// 前提）：当前树元素 idx。σ/ε 出生对立（树 eps ≠ 腿 σ 但载体未翻向）不再阻碍 Exact
    /// ——存活腿随载体结构方向重登记（#233 前基线语义）。
    Exact(usize),
    /// ★#269：ID 命中且**载体翻向结构事件激活**（registry 首见方向 ≠ 当前树元素方向 ⟺
    /// 树段 upsert 方向冲突，frontier/confirmed 同判——#227 焊缝规则保留）⟹ 翻向 = 声部
    /// 终结（#227 裁决蓝图两步形①，anc.pdf §7 I2「同一持久元素方向不变」）。
    /// 旧世代不得当 Exact 对位（事件在场时方向盲 ID 对位复活仍废除——`element_as_leg`
    /// 静默改向路径对翻向载体不可达），进当 bar 翻向种子，其后代经现成 𝒟_x^† 子树清仓连清。
    Flipped,
    /// ID 未匹配（父真失效）：调用方按 is_boundary_root 决定 root/prune。
    Stale,
}

/// 元素 [`CoverageElement`] → 活动腿 [`ActiveLeg`]（638 坐标身份：`level` / `dir=eps` /
/// `source_index=ρ`）。
///
/// `source_index=ρ`（= `LeveledMove.end_index` / 候选 `source_index`，638 hostOf 身份）——喂下一 bar
/// [`interp::interpret`] 闭环 + [`held_leg_tree_index`] 跨 bar 对位（坐标身份一致：本腿下一 bar 仍按
/// `(level, ρ, dir)` 映射回真树元素）。候选元素 `lambda==rho==source_index`，根/树走势元素 `ρ=end_index`。
pub(super) fn element_as_leg(e: &CoverageElement) -> ActiveLeg {
    // ρ=source_index（右端点，漂移）+ λ=lambda（左端点，稳定语义身份，持久身份对位用）。
    // ★codex Q4：携带 id/parent_id/is_boundary_root（跨 bar 稳定身份，spec §13 结构映射）。
    // is_boundary_root = parent_id.is_none()（真边界胚元 ∂ 根）。
    // ★persistent overlay（anc.pdf §6）：op_parent = 入场时 parent_id（操作父，持久）。
    // 新开腿 op_parent=当前 parent_id（入场容器）；后续 bar parent_id 可变（结构父 pstr），
    // op_parent 不变 → 腿不因结构父变化而 Stale（§6）。
    ActiveLeg {
        level: e.level,
        dir: e.eps,
        source_index: e.rho,
        lambda: e.lambda,
        id: e.id,
        parent_id: e.parent_id,
        is_boundary_root: e.parent_id.is_none(),
        op_parent: e.parent_id,
    }
}

/// 𝒟_x 在 A_t 中的索引集（close 腿 ⊆ A_t；每个 close 腿认领一个匹配占位，保多重性严格）。
///
/// [`interp::Buckets::close`]（𝒟_x）是 A_t 的子集（关闭的是活动腿）——逐个 close 腿在 `prev_active`
/// 找首个未被认领的相等腿（`level`/`dir`/`source_index` 三元相等）标记其索引。close 腿若不匹配
/// 任何 A_t 腿（不应发生——interpret 的 𝒟_x 取自 working A_t）则静默跳过（不伪造删除，no-patch）。
pub(super) fn close_indices(prev_active: &[ActiveLeg], close: &[ActiveLeg]) -> Vec<usize> {
    let mut claimed = vec![false; prev_active.len()];
    let mut idx = Vec::new();
    for d in close {
        for (i, leg) in prev_active.iter().enumerate() {
            if !claimed[i] && leg == d {
                claimed[i] = true;
                idx.push(i);
                break;
            }
        }
    }
    idx
}

/// **环6：解释器三桶 → 活动集递归 `A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x]` + 目标头寸 `p̃_{t+1}`**
/// （§13 持仓准入实装：ReverseOpen 子声部腿**未持父则剔除**，不开 naked 逆势仓——639(c) 兑现）。
///
/// 消费 **真父子组合元素数组** `elements`（[`interp::coverage_elements_with_tower`] 产：
/// `elements[0..candidate_start)` = 真嵌套树走势元素（`RMove::Compose` 真父子，547 铁律），
/// `elements[candidate_start..)` = 638 附着买卖点候选元素，**含真 `parent`/`attached_dir`**）+
/// [`interp::Buckets`] 三桶：`close`=𝒟_x（应关活动腿 ⊆A_t）、`open`=ℬ_x（应开候选）、`record`=𝒦_x
/// （不入活动集，本函数不消费）。
///
/// ## 两步活动集递归（spec §13 line 657-685）
/// 1. `A^{raw}_{t+1} = (A_t ∖ 𝒟_x) ∪ ℬ_x`：
///    - **(A_t∖𝒟_x)**：持仓腿（除 𝒟_x，[`close_indices`] 认领）经 [`held_leg_tree_index`] 对位回
///      当前因果树元素索引（638 身份）——持仓的**父容器腿**由此在 `raw` 在场；不在树者追加为根。
///    - **∪ℬ_x**：开启候选映射到其因果树附着元素索引 `candidate_start + gamma_index`（携真父）。
/// 2. `A_{t+1} = AncOK(A^{raw})`，`AncOK(A)={e∈A:Anc(e)⊆A}`：[`ancestor_close`] 剔除**父容器不在
///    `raw`** 的孤儿子腿——ReverseOpen（及任意子声部）候选**仅当其真 Compose 父容器腿在持仓集 A_t**
///    才准入（spec line 671「任何子级短差腿存在时，其父容器也存在」）。
///
/// ## ★#247 缺口一：转移的**第三来源** `ℛ_x = RegistryRestore`（声明与实装对齐）
///
/// 生产实装的转移**不是** spec §13 字面的两来源式，而是三来源：
///
/// ```text
///     A_{t+1} = AncOK[ (A_t ∖ 𝒟_x^†) ∪ ℬ_x ∪ ℛ_x ]
///     ℛ_x = RegistryRestore(A_t, ℬ_x) ⊆ Pi          （persistent registry 恢复的操作祖先）
/// ```
///
/// `ℛ_x` 的元素**既不在输入 `A_t` 的腿里，也不是 `Γ_trade` 落入 ℬ_x/𝒦_x 的候选**，而是
/// [`restore_ancestor_chain_from_registry`] 从 persistent registry 沿 `structural_parent_id` 链
/// 恢复入 `work`/`raw` 的祖先元素（anc.pdf §11 归纳：每条未关闭腿的操作父 live ⟹ depth<d 腿通过
/// persistent AncOK）。两条注入路径：
/// - **held 路**（A_t 段）：Stale + `HeldLegState::LiveDetached` 持仓腿的 `op_parent` 祖先链；
/// - **open 路**（ℬ_x 段）：open 候选 `parent_id` 且 `registry_live` 的父 carrier 祖先链
///   （其中当 bar 关闭种子 ∈𝒟_x 中断，#226）。
///
/// `ℛ_x` **确实进 `A_{t+1}`**：恢复元素随 `raw` 进 `next_idx`，[`strategy_target_legs`] 为其生成
/// 目标腿并计入 `p̃`（≠ 只作 AncOK 的祖先在场性判据）。故声明只写 `AncOK[(A_t∖𝒟)∪ℬ]` 与实装不
/// 一致——本条即为对齐（#247 缺口一；#244 承重核查）。Lean 侧对应关系表述见
/// `formal/Origin/ActiveSet.lean` §5′（该处声明 Lean 的两来源 `rawUpdate` 是 rust 三来源转移的
/// **ℛ_x=∅ 限制**，即有效域声明而非等价声称）。
///
/// `ℛ_x` 元素的**角色输入**（`parent` 索引 / `attached_dir`）由 `parent_id` 重建（#247 缺口二），
/// 不再恒定落 `V=Ambient / G=SameLevel`。★#247 C2（issue #371）起，[`held_stale_reregister_idx`]
/// 新 push 的**持仓腿自身**同口径重建（`op_parent` → work idx），故整条恢复链**口径统一**——
/// 不再是「祖先真、触发腿伪」的混合态。
///
/// ### ★#247 C1：`parent` 图无环性**无写入侧保障** ⟹ [`element_depth`] 带 fuel 硬门
/// 回填让 `parent` 可指向 overlay ⟹ 无环性依赖 registry `structural_parent_id`，而其写入侧
/// （`persistent.rs`，含跨 bar 可变更新）无环检测、无「父 level > 子 level」校验。
/// [`element_depth`] 上溯步数 `> elements.len()` ⟹ 显式 panic + 探针
/// [`AncokProbe::element_depth_fuel_exhausted`]（不静默钳制）。
///
/// ### ★#247 C3：两条**限定声明** + 一条深链行为面（宣称行为影响时必须同时给出）
/// **落点如实**：下列三条在 rustdoc 上属 [`restore_ancestor_chain_from_registry`] 的 doc（本 doc 块
/// 自 §8 抬头起连续，物理位置在该函数之前），**不是** [`coverage_step_from_buckets_sep`] doc 的 C3
/// 小节——那里没有 C3 条目。测试侧标号对应：C3-1=深链（本节 3）/ C3-2=Neutral（本节 1）/
/// C3-3=gross cap（本节 2）。
/// 1. **「units 差异全部来自 `depth`」只在 `ThetaDirPreset::Neutral` + `w_grade=[1,1]` 下成立。**
///    `w = depth_weight(d) × dir_weight(role,d) × w_grade(role)`。Neutral 下 `dir_weight≡1.0`、
///    default `w_grade≡1.0`，故 #247 的对拍读数差异确实只由 `depth_weight` 产生。但
///    `wverify_run.rs` 有环境变量驱动的 `Follow{eta_adv}` / `Adversary{eta_same}` **生产入口**：
///    那些运行下 V 从 `Ambient` 变 `FollowParent`/`ReverseOpen` 会打开 `dir_weight` 的查表 CASE
///    （Ambient 走恒 1.0 的 CASE），G 从 `SameLevel` 变 `SubLevel` 会切 `w_grade` 槽位 ⟹
///    **units 另有两条独立变化通道**。归因不可外推到非 Neutral 运行。
/// 2. **「角色重建不改腿集合身份」只在 gross cap 未激活（`risk=None` / `enforce_gross_cap=false`）
///    下成立。** [`element_as_leg`] 只读 λ/ρ/ε/ℓ/id/parent_id ⟹ AncOK 成员资格确实不受角色影响；
///    但 cap 激活时 [`apply_gross_cap`] 按 units 求缩放 ⟹ units 变 ⟹ `gross_zeroed` 可能变 ⟹
///    幽灵腿防护剔除的开仓腿集合可能变 ⟹ **`next_active` 成员资格会变**。
/// 3. **深链行为面**：default `depth_weights=[0.60,0.30,0.10]` ⟹ **depth ≥ 3 的恢复元素
///    `w_depth=0`，整条腿 units 归零**（spec:42「权重表外深度不获资金、剩余留现金」，是定义
///    行为非 bug）。改前它们恒 depth=0 拿满权 0.60 ⟹ 深链上 #247 的量级是 **0.60→0**，
///    远大于两级链的 0.60→0.30/0.10。见 `restore_deep_chain_depth_ge3_weight_zeroed`。
///    registry 恢复链长度无上界 ⟹ 深链在数据层可达；真实窗口的 depth 分布仍需运行时 trace。
///
/// ## §13 持仓准入兑现（639 (c)）
/// 父有向但**未持父仓**的逆向次级候选 = ReverseOpen（σ_p=父容器方向，639；其元素 `parent` 指向真
/// Compose 父树元素）。该父树元素**在 `raw` ⟺ 持仓腿对位到它**（[`held_leg_tree_index`]）；未持父
/// ⟹ 父不在 `raw` ⟹ AncOK **剪枝**该 ReverseOpen ⟹ **不开仓**（防 garbage trade）。持父 ⟹ 父在
/// `raw` ⟹ ReverseOpen **准入**（祖先齐全）。
///
/// ## 目标头寸（spec §14）
/// `p̃_{t+1}=Σ_{g∈A_{t+1}}Leg(g)`：[`strategy_target_legs`]（每腿 ν(g)/ε_e/s_e，depth 权重沿真父链）
/// + [`net_target_units`]（方向净额聚合 Σ ε_e·s_e，ReverseOpen 空腿部分对冲父多腿）。
///
/// 返回 `(A_{t+1}: Vec<ActiveLeg>, p̃: f64)`（[`element_as_leg`] 回腿，喂下一 bar interpret 闭环 +
/// 跨 bar 对位）。**immutable**：工作副本上追加不在树的持仓腿，不 mutate `elements`/`prev_active`/`buckets`。
///
/// ## 认识论等级（formalization-validity-domain 231号）
/// **L1**（管线正确性，**非 L2 alpha**）：确定性结构变换（三桶→对位→集差并→AncOK→净额）。AncOK
/// 接入后产/不产 trades 都是管线正确性，**不蕴含** alpha（231 铁律，下游 L2/L3 否证）。
///
/// > **结果包六要素**
/// > - **结论**：环6 桶驱动活动集递归接 §13 AncOK 持仓准入——ReverseOpen 子腿未持父则剔除（639(c)），
/// >   产 `(A_{t+1}, p̃)`。
/// > - **定义依据**：spec §13（line 657-685 `A^{raw}=(A_t∖𝒟_x)∪ℬ_x；A_{t+1}=AncOK(A^{raw})`，
/// >   `AncOK(A)={e∈A:Anc(e)⊆A}`，line 671「子级短差腿存在 ⟹ 父容器存在」）+ §14（活动腿聚合）。
/// >   输入特征：`elements` 候选段携真 `parent`（638 附着的真 Compose 父，639 σ_p=父容器方向）；
/// >   `prev_active` 持仓腿经 638 坐标身份对位回真树元素（父容器腿在场判据）。
/// > - **边界条件**：① 候选父=∂（Ambient 根：缺塔 `tower.len()<2` / host 是根）⟹ `parent=None` ⟹
/// >   Anc=∅ ⟹ AncOK 恒等准入（根级无父要求，与持仓无关）。② ReverseOpen/FollowParent 子候选父在树
/// >   但**未持父仓** ⟹ 父不在 `raw` ⟹ 剪枝（结论翻转：持父则准入）。③ 持仓父腿关闭（∈𝒟_x）⟹ 其
/// >   子腿同 bar 失祖先 ⟹ AncOK 连带剪枝（覆盖不漂浮，spec §13）。④ 𝒟_x 须 ⊆A_t（interpret 保证）。
/// > - **下游推论**：A_{t+1} 喂下一 bar [`interp::interpret`] 闭环 + 跨 bar [`held_leg_tree_index`]
/// >   对位；p̃ 喂环7 π_Θ LexArgmin。未持父不开 ReverseOpen ⟹ `run_theta_v0_pi` 不再持 naked 逆势仓。
/// > - **谱系引用**：639（σ_p=父容器方向 ⊥ 持仓准入；本函数兑现 (c) 承诺的"未持父不开 ReverseOpen"）；
/// >   638（hostOf 附着=候选真父来源）；547（真 Fugue，父只来自真 Compose）；
/// >   coverage-engine-needs-tower-export-bridge（多级角色/嵌套对冲=#5 alpha 来源，依赖 AncOK 准入）。
/// > - **影响声明**：重写 coverage.rs §8 `coverage_step_from_buckets`（签名加 `elements`/
/// >   `candidate_start`，接真父子元素数组）；**删** `legs_as_root_elements`/`open_candidate_as_leg`/
/// >   `root_element_as_leg`（平铺全根错路径，no-patch 不留 fallback）；新增 [`held_leg_tree_index`]/
/// >   [`element_as_leg`]；复用 [`close_indices`]/[`ancestor_close`]/[`strategy_target_legs`]/
/// >   [`net_target_units`]；不改 σ_p 来源（assemble_gamma_with_tower）/interp.rs/mod.rs。

/// ★persistent overlay（anc.pdf §11 归纳）：从 registry 递归恢复操作祖先链。
///
/// LiveDetached 腿的 op_parent 及其祖先（沿 structural_parent_id 链）若不在 raw 中，
/// 从 persistent registry 恢复加入 work/raw。这使生产 AncOK（#183 归一后 =
/// [`super::super::exit::step_active_set_with_subtree_close`]）通过（I5：AncOK 作用 persistent active set）。
///
/// §11 归纳证明：每条未关闭腿的操作父 live ⟹ 所有 depth<d 腿通过 persistent AncOK。
/// 递归上溯 structural_parent_id 链，遇到已在 raw 中的祖先停止（闭包满足）。
/// ponytail: ceiling=增量 extract_elements 时 confirmed prefix 已含全部祖先，无需恢复。
///
/// ★#226 `closed_seeds`（S3：父终结 ⟹ 子树清仓——与 A_t 段「restore 不得复活被关父」同一
/// 教义）：walk 中遇 **当 bar 已被裁决终结**（∈ 𝒟_x = `buckets.close`）的祖先即中断——不复活、
/// 不上溯（计数 `restore_break_closed_seed`）。**open 父注入路径必传**（ℬ_x 段不经 𝒟_x^†，
/// 无种子兜底：复活被关父 = 一类批末该级 Core 物理残余，m3 win9 bar=17032 断言①炸点
/// 根因——残余 = 触发腿自身自复活，勘察报告
/// `.chanlun/review-results/assertion1-win9-open-restore-resurrect-20260724.md`）；
/// **held 路传空切片**（A_t 段由 𝒟_x^† 种子命中兜底，语义/轨迹 bit-exact 不变）。
/// `already_in_raw` 检查先行：种子经反手候选同 id 重开已在 raw 时闭包合法收敛
/// （ID-3「允许当场反手」不误伤——过滤只作用 restore 祖先注入，候选推送零改）。
pub(super) fn restore_ancestor_chain_from_registry(
    work: &mut ElementView,
    raw: &mut Vec<usize>,
    registry: &super::super::persistent::PersistentRegistry,
    start_pid: super::super::classifier::recursive_tower::ElementId,
    // ★工位 4f：id→idx 查表（base 段缓存 + restore 动态 push 的 overlay 段累积），消 work.iter().position
    // O(work)/层=O(n²)。raw.any 不动（raw 有界，prev_active~O(log n) 实测 9@16K）。
    id_idx: &std::collections::HashMap<ElementId, usize>,
    overlay_seen: &mut std::collections::HashMap<ElementId, usize>,
    // ★#446 补移植（影子评审 HIGH-1，2026-07-29）：候选段终点。overlay_seen 命中 idx < 此值 ⟹ 候选段
    // 拷贝——门禁语义同 [`held_stale_reregister_idx`]（`held.rs:369-373`）：候选 eps 是信号方向、
    // lambda==rho 点元素、parent_id 非持久 structural_parent_id，复用会让上溯改沿候选伪 parent_id
    // 走、且被复用元素角色输入失真（详见下方修补处注释）。
    overlay_cand_end: usize,
    // ★#226：当 bar 关闭种子（见函数 doc）。
    closed_seeds: &[ActiveLeg],
    // ★票#350（#247/#315 同类第三位点）：本轮新 push 的恢复元素 idx 追加于此（调用方持有，跨本
    // 函数的多次调用 + held 腿占位共用同一累加器，见 [`held_stale_reregister_idx`]）——不在本函数
    // 内修补，见函数头 #350 说明。
    pending_parent_fixup: &mut Vec<usize>,
) {
    ancok_probe_bump(|p| p.restore_calls += 1);
    let mut broke = false;
    let mut cur = Some(start_pid);
    while let Some(pid) = cur {
        // 已在 raw 中？⟹ 闭包满足，停止递归。
        let already_in_raw = raw
            .iter()
            .any(|&r| work.get(r).map(|e| e.id == pid).unwrap_or(false));
        if already_in_raw {
            ancok_probe_bump(|p| p.restore_break_already_in_raw += 1);
            broke = true;
            break;
        }
        // ★#226：当 bar 已被裁决终结的祖先不经 restore 复活（open 父注入路径的种子兜底；
        // 中断后调用侧候选父链断裂，由统一 AncOK 正常剪除——非加特例）。
        if closed_seeds.iter().any(|l| l.id == pid) {
            ancok_probe_bump(|p| p.restore_break_closed_seed += 1);
            broke = true;
            break;
        }
        // ★(I-1) 祖先若已在 work（树前缀 carrier / restore 已 push 的）但不在 raw，**复用现有 idx**入 raw
        // （不 push 重复 id，否则 strategy_target_legs 双计 p̃ 伪证）。查表 O(1)：先 base id_idx 再 overlay_seen。
        // bit-exact == 旧 work.iter().position：position 返首个匹配 idx，base 段在 overlay 前 ⟹ base 优先与
        // position 序一致；overlay_seen 用 or_insert 存首次 push idx ⟹ 与 position 在 overlay 段首个匹配一致。
        //
        // ★#446 补移植（影子评审 HIGH-1）：overlay_seen 命中若落在候选段（idx < overlay_cand_end）是
        // **候选拷贝**——同 [`held_stale_reregister_idx`] 门禁语义（held.rs:369-373），不可复用：候选
        // eps 是信号方向（可与真祖先持久方向相反）、lambda==rho 点元素（坐标失真）、parent_id 是候选
        // 自身 Compose 父（非该祖先持久 structural_parent_id）。复用会使本次上溯 `cur` 沿候选伪
        // parent_id 走（可能提前收敛或走错分支）、且该 idx 被 element_as_leg 采纳候选属性（角色输入
        // 污染，同条目 1 held 侧缺口二同类风险）。base（id_idx）恒安全（树前缀持久身份）不受此门禁
        // 约束；overlay_seen 只放行 restore 本 bar 已 push 的持久身份（idx >= overlay_cand_end）。
        let existing_idx = id_idx.get(&pid).copied().or_else(|| {
            overlay_seen
                .get(&pid)
                .copied()
                .filter(|&idx| idx >= overlay_cand_end)
        });
        if let Some(existing_idx) = existing_idx {
            raw.push(existing_idx);
            cur = work[existing_idx].parent_id; // 沿已有元素的结构父链上溯
            continue;
        }
        // 从 registry 取元素（work 中尚无 ⟹ 真 LiveDetached 祖先，须从持久身份恢复）。
        let pe = match registry.get(&pid) {
            // ★#713（L10）B′ 冲突史守卫：`dir_conflict_seen` 置位（tree 首见 ∧ tree 段方向
            // 冲突，#269 口径的真翻向事件证据）⟹ 该条目方向证据已腐化（I2 永固首见方向，
            // 翻向后的新世代永远进不了 registry）——**不复活**（ElementId 身份键缺方向，
            // 复活即旧世代方向）。按中断处理（同 registry-lost 语义：AncOK 正常剪除），
            // 与 registry_lost 分桶计数。永久禁复活（含翻回，保守口径，编排者 2026-08-16 裁定）。
            Some(e) if e.dir_conflict_seen => {
                ancok_probe_bump(|p| p.restore_break_direction_conflict += 1);
                broke = true;
                break;
            }
            Some(e) if !e.invalidated => e,
            _ => {
                // ★暴露面：registry 无效/已作废 ⟹ 停止（祖先未完整恢复，本应有 parent 但已失去）。
                ancok_probe_bump(|p| p.restore_break_registry_lost += 1);
                broke = true;
                break;
            }
        };
        let parent_pid = pe.structural_parent_id;
        let op_idx = work.len();
        // parent/attached_dir 先以 None 占位：恢复循环**子先父后**上溯，push 子元素时真父 idx
        // 通常尚未知（父在后续轮次才复用/push，或跨本函数的另一次调用、或 held 腿占位路径才
        // push）⟹ 统一由调用方在本 bar 全部物化路径结束后修补（票#247 引入、票#350 移出本函数
        // 至调用方统一，见 [`resolve_pending_parent_fixups`]）。
        work.push(CoverageElement {
            lambda: pe.lambda,
            rho: pe.rho,
            eps: pe.dir,
            level: pe.level,
            parent: None,
            attached_dir: None,
            id: pe.pid,
            parent_id: pe.structural_parent_id,
        });
        overlay_seen.entry(pe.pid).or_insert(op_idx); // 记录新 push 的 overlay idx（首次出现序，复用查 O(1)）。
        pending_parent_fixup.push(op_idx); // 票#350：追加调用方累加器，本 bar 全部物化路径结束后统一修补。
        raw.push(op_idx);
        cur = parent_pid; // 上溯祖先链
    }
    if !broke {
        // 自然收敛（cur=None 抵达真根）：整条操作祖先链已恢复/复用完毕。
        ancok_probe_bump(|p| p.restore_complete += 1);
    }
}

/// ★#216 held Stale 腿重注册（LivePresent/LiveDetached 共用）：同 bar 前序 restore 可能已把本腿
/// 同 id 持久元素恢复入 work（子腿先处理、其 op_parent 祖先链含本腿 ⟹ restore 先 push）——重注册
/// **复用该 restore push 的现有 idx** 入 raw（I1 同一持久身份唯一表示，spec §13 元素集语义；其
/// 坐标取自同一 registry 条目，与腿同值 ⟹ 复用零失真），无则新增并登记 overlay_seen 供后续
/// restore/held 复用查 O(1)。id_idx 必 miss（Stale ⟹ base 无此 id）故只查 overlay 段；不查则
/// 重复 push ⟹ next_active 同 id 两槽 ⟹ strategy_target_legs 双计 p̃。
///
/// ★code-review Spec 轴 (c)1（2026-07-24）：复用**仅限 restore push**（`idx >= overlay_cand_end`）。
/// overlay 命中落在候选段（`< overlay_cand_end`）时是**候选拷贝**——其 eps 是信号方向（可与持仓
/// 反向）、lambda==rho 点元素、parent_id 非本腿 op_parent；复用会让 element_as_leg 采纳候选属性
/// （持仓方向静默翻转、I4 op_parent 失真）。候选碰撞由 open 循环 ① 规则让位（id 已在 raw ⟹
/// 跳过候选拷贝）——持仓身份优先：本腿 push 自身元素（坐标/op_parent 保真）。
///
/// ## ★#247 C2（影子评审承重级，承接 issue #371）：角色输入重建同步修
///
/// 本函数新 push 的**持仓腿自身**元素原同样写死 `parent:None, attached_dir:None`——与
/// [`restore_ancestor_chain_from_registry`] 的缺口二**完全同构**，且在 held 路必然**同时命中**：
/// `HeldLegState::LiveDetached` 分支是 `restore(...)` 紧接本函数，只要 held 路触发 restore，
/// 触发腿自身就必走重注册。⟹ 只修祖先 = 同一条链上混合口径（祖先真、触发腿伪）。
///
/// 数值后果（default `depth_weights=[0.60,0.30,0.10]`）：伪根 ⟹ `element_depth=0 ⟹ w_depth=0.60`
/// 拿满权，而真嵌套 depth 由 `parent` 链定（对拍场景 child→P1→P2 ⟹ d=2 ⟹ 0.10）。故半修
/// （仅祖先）在对拍场景把 `p̃` 从 −600 推到 −300，而全重建口径是 −800 —— **半修在数值上离
/// 真口径更远**（见 `restore_role_rebuild_changes_p_tilde_leg_set_unchanged` 三口径对拍表）。
///
/// 修法（与 #247 缺口二**同一口径**，不引入第二套语义）：新 push 后按 `leg.op_parent` 解析
/// work idx（base `id_idx` 优先、再 `overlay_seen`——与 restore 回填同解析序），命中 ⟹
/// `parent=Some(idx)`、`attached_dir=Some(父.eps)`，计 `restore_parent_rebound`。
///
/// **depth 语义**：按**恢复链深度**——`parent` 链即 `op_parent` 链，与恢复祖先元素同源
/// （`element_depth` 的定义「沿 parent 链、真父子铁律」对本腿与对祖先是同一把尺）。不另立
/// 「持仓腿 depth 恒 0」的第二口径。
///
/// **父不在 work 的语义裁定**（与 #247 缺口二对齐）：`op_parent=None` ⟹ 真 ∂ 边界胚元，
/// `parent/attached_dir` 留 None 是**正确**语义（V=Ambient）；`op_parent=Some` 但链断 ⟹
/// `parent` 留 None、`parent_id` 保持 Some ⟹ `is_boundary_root=false` ⟹ 统一 AncOK 按 id 判
/// 祖先不在集 ⟹ **必被剪除**，从不进 `next_idx`/`strategy_target_legs` ⟹ 角色从不被打分。
/// 计 `restore_parent_unresolved`。
///
/// **复用分支不回填**：`existing >= overlay_cand_end` 命中的是本 bar restore push 的元素，其
/// `parent/attached_dir` 已由统一 fixup（[`resolve_pending_parent_fixups`]）回填 ⟹ 不重写（树前缀/
/// 候选段属性零改，同 #247 口径）。
///
/// ★票#315（#284 评审 MED-1，对齐 #247）：新 push 分支**不再**在此处立即解析 `parent`/
/// `attached_dir`——push 前 `op_parent` 是否已在 work 依调用点而异（`LiveDetached` 调用点前有
/// `restore_ancestor_chain_from_registry` 先行、`LivePresent` 调用点无），但即便调用点已先行，父
/// 仍可能在**本 bar更晚**才经其他路径物化（`Closed|Invalidated`+`is_boundary_root` 边界根直接
/// push、后续 held 循环迭代、或 open 候选父链恢复）——immediate 式解析在那一刻查不到父，误固化
/// None/None（#247 缺口同类重现）。改为新 push 的 idx 收进调用方 `pending_parent_fixup`
/// 累加器，`parent`/`attached_dir` 留 None，两个物化循环全部结束后由
/// [`resolve_pending_parent_fixups`] 统一解析（此时 id_idx/overlay_seen/raw 均为本 bar 终态）。
pub(super) fn held_stale_reregister_idx(
    work: &mut ElementView,
    overlay_seen: &mut std::collections::HashMap<ElementId, usize>,
    overlay_cand_end: usize,
    pending_parent_fixup: &mut Vec<usize>,
    leg: &ActiveLeg,
) -> usize {
    match overlay_seen.get(&leg.id) {
        Some(&existing) if existing >= overlay_cand_end => existing,
        _ => {
            let idx = work.len();
            work.push(CoverageElement {
                lambda: leg.lambda,
                rho: leg.source_index,
                eps: leg.dir,
                level: leg.level,
                parent: None,
                attached_dir: None,
                id: leg.id,
                parent_id: leg.op_parent,
            });
            overlay_seen.entry(leg.id).or_insert(idx);
            pending_parent_fixup.push(idx);
            idx
        }
    }
}

/// ★票#315（#284 评审 MED-1 + #347 评审 LOW-3）：本 bar `pending_parent_fixup` 累加器（held 腿占位
/// [`held_stale_reregister_idx`] 新 push 分支）统一 fixup——两个物化循环（prev_active held 腿 + open
/// 候选父链恢复）全部结束、AncOK 判定前调用。父 idx 解析三级：`id_idx`（base 段）→ `overlay_seen`
/// （candidate+restore 段）→ `raw` 扫兜底（`Closed|Invalidated`+`is_boundary_root` 边界根直接 push
/// 的元素不进 `overlay_seen`，唯一可查途径是扫 `raw`；`r != idx` 排除自环守卫，同 #347 LOW-1）。
/// 解析成功计 `restore_parent_rebound`，父不可解析（真 ∂ 边界胚元 `parent_id=None`，或本 bar 内父
/// 确未被任何路径物化）留 None/None、`parent_id=Some` 时计 `restore_parent_unresolved`（随后交由
/// AncOK 按 `parent_id` 剪除，不伪造）。
///
/// 返回本 bar 未解析的 idx 列表（★票#347 MED-1：供调用方与 `next_idx`（AncOK 存活集）交叉
/// 核对——不在 `next_idx` 即被剪除，计入 [`AncokProbe::placeholder_pruned_by_ancok`]，使
/// 「可与 AncOK 剪除计数交叉核对」这一文档声明可执行）。
pub(super) fn resolve_pending_parent_fixups(
    work: &mut ElementView,
    pending: &[usize],
    id_idx: &std::collections::HashMap<ElementId, usize>,
    overlay_seen: &std::collections::HashMap<ElementId, usize>,
    raw: &[usize],
) -> Vec<usize> {
    let mut unresolved = Vec::new();
    for &idx in pending {
        let pid = match work[idx].parent_id {
            Some(pid) => pid,
            None => continue, // 真边界胚元 ∂：parent/attached_dir 留 None 是正确语义，不计入探针。
        };
        let pidx = id_idx
            .get(&pid)
            .copied()
            .or_else(|| overlay_seen.get(&pid).copied())
            .or_else(|| {
                raw.iter()
                    .copied()
                    .filter(|&r| r != idx)
                    .find(|&r| work.get(r).map(|e| e.id == pid).unwrap_or(false))
            });
        match pidx {
            Some(pidx) => {
                let sigma_p = work[pidx].eps;
                if let Some(e) = work.overlay_mut(idx) {
                    e.parent = Some(pidx);
                    e.attached_dir = Some(sigma_p);
                }
                ancok_probe_bump(|p| p.restore_parent_rebound += 1);
            }
            None => {
                ancok_probe_bump(|p| p.restore_parent_unresolved += 1);
                unresolved.push(idx);
            }
        }
    }
    unresolved
}

#[cfg(test)]
#[path = "held_tests_1.rs"]
mod tests_1;

#[cfg(test)]
#[path = "held_tests_2.rs"]
mod tests_2;
