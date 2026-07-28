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
pub(crate) fn held_leg_tree_index(
    elements: &[CoverageElement],
    candidate_start: usize,
    leg: &ActiveLeg,
) -> HeldLegMatch {
    let tree_end = candidate_start.min(elements.len());
    let tree = &elements[..tree_end];
    let id_idx = build_tree_id_index(tree);
    held_leg_tree_index_indexed(tree, leg, &id_idx)
}

/// ponytail: H6 带预建索引的 held_leg_tree_index 变体——热循环 coverage_step_from_buckets 单次建、多次查。
/// codex Q4：按 `leg.id` 查表（结构映射），删除 CoordDrift 分支（ID 确定性 ⟹ 无需 λ 稳定性 hack）。
pub(crate) fn held_leg_tree_index_indexed(
    tree: &[CoverageElement],
    leg: &ActiveLeg,
    id_idx: &std::collections::HashMap<ElementId, usize>,
) -> HeldLegMatch {
    // 按 ElementId 结构映射匹配（spec §13 p:C_ℓ→C_{ℓ+1}）。
    if let Some(&idx) = id_idx.get(&leg.id) {
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
pub(crate) enum HeldLegMatch {
    /// ID 命中（跨 bar 同走势，父延伸也同 ID）：当前树元素 idx。
    Exact(usize),
    /// ID 未匹配（父真失效）：调用方按 is_boundary_root 决定 root/prune。
    Stale,
}

/// 元素 [`CoverageElement`] → 活动腿 [`ActiveLeg`]（638 坐标身份：`level` / `dir=eps` /
/// `source_index=ρ`）。
///
/// `source_index=ρ`（= `LeveledMove.end_index` / 候选 `source_index`，638 hostOf 身份）——喂下一 bar
/// [`interp::interpret`] 闭环 + [`held_leg_tree_index`] 跨 bar 对位（坐标身份一致：本腿下一 bar 仍按
/// `(level, ρ, dir)` 映射回真树元素）。候选元素 `lambda==rho==source_index`，根/树走势元素 `ρ=end_index`。
pub(crate) fn element_as_leg(e: &CoverageElement) -> ActiveLeg {
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

/// ★#446：held Stale 腿重注册时复用本 bar registry restore 已物化的同 ID 槽。
///
/// `overlay_cand_end` 划分初始候选段与后续 restore/held 段：候选拷贝即使 ID 相同也不能复用，
/// 因其方向、坐标和 `parent_id` 属于新信号，不是持仓身份；restore/held 段则是同一持久身份，
/// 必须复用以维持活动集 `ElementId` 唯一。新增 held 槽同时进入 `overlay_seen` 与统一 parent
/// fixup 队列，保持 #315/#350 的延迟修补时序。
pub(crate) fn held_stale_reregister_idx(
    work: &mut ElementView,
    overlay_seen: &mut std::collections::HashMap<ElementId, usize>,
    overlay_cand_end: usize,
    pending_parent_fixup: &mut Vec<usize>,
    leg: &ActiveLeg,
) -> usize {
    if let Some(&existing) = overlay_seen.get(&leg.id) {
        if existing >= overlay_cand_end {
            // restore 条目可能由同 ID 的当前候选 snapshot 刷新过属性；held 腿才是未关闭持久身份
            // 的权威。复用 idx 但覆盖为 held 坐标/方向/op_parent，禁止静默方向翻转。
            work.replace_overlay(
                existing,
                CoverageElement {
                    lambda: leg.lambda,
                    rho: leg.source_index,
                    eps: leg.dir,
                    level: leg.level,
                    parent: None,
                    attached_dir: None,
                    id: leg.id,
                    parent_id: leg.op_parent,
                },
            );
            return existing;
        }
    }

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
    // held 身份优先于候选拷贝：后续 restore/held 查同 ID 应复用本槽。
    overlay_seen.insert(leg.id, idx);
    pending_parent_fixup.push(idx);
    idx
}

/// 𝒟_x 在 A_t 中的索引集（close 腿 ⊆ A_t；每个 close 腿认领一个匹配占位，保多重性严格）。
///
/// [`interp::Buckets::close`]（𝒟_x）是 A_t 的子集（关闭的是活动腿）——逐个 close 腿在 `prev_active`
/// 找首个未被认领的相等腿（`level`/`dir`/`source_index` 三元相等）标记其索引。close 腿若不匹配
/// 任何 A_t 腿（不应发生——interpret 的 𝒟_x 取自 working A_t）则静默跳过（不伪造删除，no-patch）。
pub(crate) fn close_indices(prev_active: &[ActiveLeg], close: &[ActiveLeg]) -> Vec<usize> {
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

/// **环6：解释器三桶 → 活动集递归 `A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x∪RegistryRestore]` + 目标头寸 `p̃_{t+1}`**
/// （§13 持仓准入实装：ShortDiff 子声部腿**未持父则剔除**，不开 naked 逆势仓——639(c) 兑现）。
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
///    - **∪RegistryRestore**（票#247 缺口一：显式第三来源）：LiveDetached 持仓腿 / open 候选
///      父的操作祖先链经 [`restore_ancestor_chain_from_registry`] 从 persistent registry 恢复入
///      work/raw——理想域中本已在场的持久祖先在 per-bar 因果树上的**物化机制**（anc.pdf §11
///      归纳：每条未关闭腿的操作父 live ⟹ depth<d 祖先全在 raw ⟹ AncOK 通过），**非新数学来源**。
/// 2. `A_{t+1} = AncOK(A^{raw})`，`AncOK(A)={e∈A:Anc(e)⊆A}`：[`ancestor_close`] 剔除**父容器不在
///    `raw`** 的孤儿子腿——ShortDiff（及任意子声部）候选**仅当其真 Compose 父容器腿在持仓集 A_t**
///    才准入（spec line 671「任何子级短差腿存在时，其父容器也存在」）。
///
/// ## §13 持仓准入兑现（639 (c)）
/// 父有向但**未持父仓**的逆向次级候选 = ShortDiff（σ_p=父容器方向，639；其元素 `parent` 指向真
/// Compose 父树元素）。该父树元素**在 `raw` ⟺ 持仓腿对位到它**（[`held_leg_tree_index`]）；未持父
/// ⟹ 父不在 `raw` ⟹ AncOK **剪枝**该 ShortDiff ⟹ **不开仓**（防 garbage trade）。持父 ⟹ 父在
/// `raw` ⟹ ShortDiff **准入**（祖先齐全）。
///
/// ## 目标头寸（spec §14）
/// `p̃_{t+1}=Σ_{g∈A_{t+1}}Leg(g)`：[`strategy_target_legs`]（每腿 ν(g)/ε_e/s_e，depth 权重沿真父链）
/// + [`net_target_units`]（方向净额聚合 Σ ε_e·s_e，ShortDiff 空腿部分对冲父多腿）。
///
/// 返回 `(A_{t+1}: Vec<ActiveLeg>, p̃: f64)`（[`element_as_leg`] 回腿，喂下一 bar interpret 闭环 +
/// 跨 bar 对位）。**immutable**：工作副本上追加不在树的持仓腿，不 mutate `elements`/`prev_active`/`buckets`。
///
/// ## 认识论等级（formalization-validity-domain 231号）
/// **L1**（管线正确性，**非 L2 alpha**）：确定性结构变换（三桶→对位→集差并→AncOK→净额）。AncOK
/// 接入后产/不产 trades 都是管线正确性，**不蕴含** alpha（231 铁律，下游 L2/L3 否证）。
///
/// > **结果包六要素**
/// > - **结论**：环6 桶驱动活动集递归接 §13 AncOK 持仓准入——ShortDiff 子腿未持父则剔除（639(c)），
/// >   产 `(A_{t+1}, p̃)`。
/// > - **定义依据**：spec §13（line 657-685 `A^{raw}=(A_t∖𝒟_x)∪ℬ_x；A_{t+1}=AncOK(A^{raw})`，
/// >   `AncOK(A)={e∈A:Anc(e)⊆A}`，line 671「子级短差腿存在 ⟹ 父容器存在」）+ §14（活动腿聚合）。
/// >   输入特征：`elements` 候选段携真 `parent`（638 附着的真 Compose 父，639 σ_p=父容器方向）；
/// >   `prev_active` 持仓腿经 638 坐标身份对位回真树元素（父容器腿在场判据）。
/// > - **边界条件**：① 候选父=∂（Ambient 根：缺塔 `tower.len()<2` / host 是根）⟹ `parent=None` ⟹
/// >   Anc=∅ ⟹ AncOK 恒等准入（根级无父要求，与持仓无关）。② ShortDiff/FollowParent 子候选父在树
/// >   但**未持父仓** ⟹ 父不在 `raw` ⟹ 剪枝（结论翻转：持父则准入）。③ 持仓父腿关闭（∈𝒟_x）⟹ 其
/// >   子腿同 bar 失祖先 ⟹ AncOK 连带剪枝（覆盖不漂浮，spec §13）。④ 𝒟_x 须 ⊆A_t（interpret 保证）。
/// > - **下游推论**：A_{t+1} 喂下一 bar [`interp::interpret`] 闭环 + 跨 bar [`held_leg_tree_index`]
/// >   对位；p̃ 喂环7 π_Θ LexArgmin。未持父不开 ShortDiff ⟹ `run_theta_v0_pi` 不再持 naked 逆势仓。
/// > - **谱系引用**：639（σ_p=父容器方向 ⊥ 持仓准入；本函数兑现 (c) 承诺的"未持父不开 ShortDiff"）；
/// >   638（hostOf 附着=候选真父来源）；547（真 Fugue，父只来自真 Compose）；
/// >   coverage-engine-needs-tower-export-bridge（多级角色/嵌套对冲=#5 alpha 来源，依赖 AncOK 准入）。
/// > - **影响声明**：重写 coverage.rs §8 `coverage_step_from_buckets`（签名加 `elements`/
/// >   `candidate_start`，接真父子元素数组）；**删** `legs_as_root_elements`/`open_candidate_as_leg`/
/// >   `root_element_as_leg`（平铺全根错路径，no-patch 不留 fallback）；新增 [`held_leg_tree_index`]/
/// >   [`element_as_leg`]；复用 [`close_indices`]/[`ancestor_close`]/[`strategy_target_legs`]/
/// >   [`net_target_units`]；不改 σ_p 来源（assemble_gamma_with_tower）/interp.rs/mod.rs。

/// ★persistent overlay（anc.pdf §11 归纳）：从 registry 递归恢复操作祖先链。
///
/// ★票#247 缺口一（声明一致）：本函数实装活动集递归
/// `A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x∪RegistryRestore]` 的**显式第三来源** `RegistryRestore`——
/// 理想域中本已在场的持久祖先（LiveDetached）在 per-bar 因果树上的**物化机制**，
/// **非新数学来源**。
///
/// LiveDetached 腿的 op_parent 及其祖先（沿 structural_parent_id 链）若不在 raw 中，
/// 从 persistent registry 恢复加入 work/raw。这使 ancestor_close_by_id 通过（I5：
/// AncOK 作用 persistent active set）。
///
/// §11 归纳证明：每条未关闭腿的操作父 live ⟹ 所有 depth<d 腿通过 persistent AncOK。
/// 递归上溯 structural_parent_id 链，遇到已在 raw 中的祖先停止（闭包满足）。
/// ponytail: ceiling=增量 extract_elements 时 confirmed prefix 已含全部祖先，无需恢复。
///
/// ★票#350 修复（#347 评审 LOW-3，#247/#315 同类第三位点）：`parent`/`attached_dir` 修补**不在本
/// 函数内**完成——本函数只把本轮新 push 的恢复元素 idx 追加进调用方持有的 `pending_parent_fixup`
/// 累加器，实际修补由调用方（[`coverage_step_from_buckets_sep`]）在**本 bar 两个物化循环
/// （prev_active held 腿 + open 候选父链恢复，本函数在两处均可能被多次调用）全部结束后**统一执行
/// （同 [`rebuild_placeholder_parent_attached`] 处理 held 腿占位的形状，#315）。
///
/// 核实坐实的时序孔（#350）：修复前，本函数内部在自身 `while` 链结束后立即修补——链内部**同一次
/// 调用**内子→父的解析确无孔（父恰是下一轮要处理的 `cur`，registry/`id_idx` 与调用顺序无关，
/// 详见 `restore_chain_shared_ancestor_across_two_calls_resolves_and_dedupes`）；但当链在某祖先
/// 处因 `registry_lost`（该祖先在 registry 中 `invalidated`）断链时，若该**同一祖先**在本 bar
/// **更晚**作为一条 `Closed|Invalidated` + `is_boundary_root` 的 held 腿被**直接 push 入 raw**
/// （非经 registry、非经本函数）——立即式修补已在更早那次调用内固化为 None/None，之后不会重跑，
/// 即便该祖先客观上已在 raw（AncOK 按 `parent_id` 结构判据，与本函数是否重建 `parent`/`attached_dir`
/// 无关）也检测不到，复现 #247 缺口（角色计算 V=Ambient/depth=0）。测试坐实（RED→GREEN）：
/// `restore_chain_ancestor_unresolved_when_shared_ancestor_only_materializes_via_later_boundary_root_push`。
pub(crate) fn restore_ancestor_chain_from_registry(
    work: &mut ElementView,
    raw: &mut Vec<usize>,
    registry: &super::super::persistent::PersistentRegistry,
    start_pid: super::super::classifier::recursive_tower::ElementId,
    // ★工位 4f：id→idx 查表（base 段缓存 + restore 动态 push 的 overlay 段累积），消 work.iter().position
    // O(work)/层=O(n²)。raw.any 不动（raw 有界，prev_active~O(log n) 实测 9@16K）。
    id_idx: &std::collections::HashMap<ElementId, usize>,
    overlay_seen: &mut std::collections::HashMap<ElementId, usize>,
    // ★#446：初始候选段终点。restore 只能复用 base 或此边界之后的持久 overlay；
    // 候选拷贝的方向/坐标/parent_id 不是 registry 持久身份。
    overlay_cand_end: usize,
    // ★票#350：本轮新 push 的恢复元素 idx 追加于此（调用方持有，跨本函数的多次调用 + held 腿占位
    // 共用同一累加器，见调用方 `pending_parent_fixup`）——不在本函数内修补，见函数头 #350 说明。
    pending_parent_fixup: &mut Vec<usize>,
) {
    ancok_probe_bump(|p| p.restore_calls += 1);
    let mut broke = false;
    let mut cur = Some(start_pid);
    while let Some(pid) = cur {
        // 已在 raw 中？⟹ 闭包满足，停止递归。
        let already_in_raw = raw.iter().any(|&r| work.get(r).map(|e| e.id == pid).unwrap_or(false));
        if already_in_raw {
            ancok_probe_bump(|p| p.restore_break_already_in_raw += 1);
            broke = true;
            break;
        }
        // ★(I-1) 祖先若已在 work 的树前缀或持久 overlay（restore/held 已 push）但不在 raw，
        // **复用现有 idx**入 raw（不 push 重复 id，否则 strategy_target_legs 双计 p̃ 伪证）。
        // 候选段同 ID 拷贝不复用：它不是 registry 身份，须由下方 registry 条目另行物化。
        // bit-exact == 旧 work.iter().position：position 返首个匹配 idx，base 段在 overlay 前 ⟹ base 优先与
        // position 序一致；overlay_seen 用 or_insert 存首次 push idx ⟹ 与 position 在 overlay 段首个匹配一致。
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
        // 通常尚未知（父在后续轮次才复用/push，或跨本函数的另一次调用才 push）⟹ 统一由调用方
        // 在本 bar 全部物化路径结束后修补（票#247 引入、票#350 移出本函数至调用方统一）。
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
        // 持久身份覆盖候选段索引：后续 restore/held 同 ID 必须复用本槽。
        overlay_seen.insert(pe.pid, op_idx);
        pending_parent_fixup.push(op_idx); // 票#350：追加调用方累加器，本 bar 全部物化路径结束后统一修补。
        raw.push(op_idx);
        cur = parent_pid; // 上溯祖先链
    }
    if !broke {
        // 自然收敛（cur=None 抵达真根）：整条操作祖先链已恢复/复用完毕。
        ancok_probe_bump(|p| p.restore_complete += 1);
    }
}

/// ★票#267/#315（#247 缺口二同类位点，#284 评审 MED-1 订正）：held 腿占位元素
/// （LivePresent/LiveDetached 分支）角色输入重建。
///
/// 占位元素携已知 `parent_id`（`leg.op_parent`，anc.pdf §15：LiveDetached 的 parent 仍是 op_parent(L)），
/// 修复前写死 `parent:None, attached_dir:None`。AncOK 按 parent_id 结构映射判（不看 parent 索引）⟹
/// 占位元素可存活进 next_idx；`strategy_target_legs` 角色计算消费 parent/attached_dir
/// （depth=沿 parent 链 / V=σ_{p(g)} / G=ℓ_p）⟹ 写死 None 使 depth=0、V=Ambient、G=SameLevel——
/// 角色输入丢失进 depth_weight/dir_weight/w_grade/units/p̃，承重缺口（测试坐实：
/// `held_leg_live_detached_placeholder_rebuilds_parent_attached_dir`）。
///
/// 用已知 parent_id 解析真父 idx 重建：`parent=Some(父idx)`、`attached_dir=Some(父eps)`
/// （σ_{p(g)}=父元素 eps，与 `push_element_tree` 压子元素传 `Some(父eps)` / 票#247 restore 修补
/// 同口径）。解析序同 #247：id_idx（base 段）→ overlay_seen（candidate+restore 段）→ raw 扫
/// （先前 push 的 held 占位元素不在两张查表，扫 raw 兜底，raw 有界）。
///
/// ★票#315（#284 评审 MED-1）调用时点订正：本 helper 由调用方在 `prev_active`/open 两个物化
/// 循环**结束后**、AncOK 判定前统一调用（同 #247 `restore_ancestor_chain_from_registry` 循环后
/// 统一修补形状），不再在占位 push 当轮立即调用。立即式对「父在本轮更晚迭代才物化进 raw」的
/// 场景查不到（此时父尚未 push）⟹ 误留 None/None（#284 shadow-review-267 MED-1 坐实的时序孔，
/// 测试坐实：`held_leg_placeholder_parent_materializes_in_later_iteration`）。改统一 fixup 后，
/// 三张查表在调用时点均为**本 bar 终态**，父只要本轮曾被物化（不论早于/晚于该占位被处理），
/// 均可解析。
///
/// 边界语义（皆为正确语义，非「防御分支」兜底，与 #247 裁定一致）：
/// - `parent_id=None`（真边界胚元 ∂）：保持 None/None——σ_{p(∂)}=0 ⟹ V=Ambient 是去根化正解
///   （测试坐实：`held_leg_placeholder_boundary_germ_keeps_parent_none`）。
/// - 父无法解析：保持 None/None，**不伪造**——占位元素的 parent_id **在本 bar 统一 fixup 时点
///   仍不在 raw**（真断链 `restore_break_registry_lost` 永不物化，或该占位所在 bar 内父确实
///   从未被任何路径 push）⟹ `ancestor_close_by_id`（AncOK：Anc(e)⊆raw 才保留）剪除 ⟹ 不进
///   next_idx，到不了角色计算（角色/p̃ 无影响）。**订正**（090：#284 MED-1）：此处不是「恒」
///   剪除——「不可解析」是统一 fixup 这一时点的判定，「AncOK 剪除」是循环终态 raw 上的判定，
///   两者現在**同一时点**（统一 fixup 已移到循环后）才重合为全称；命中即计入
///   `placeholder_parent_unresolved` probe（生产窗口可观测，呼应 #301 探针族）。
///   测试坐实：`held_leg_placeholder_broken_chain_pruned_by_ancok`。
///
/// ★#347 MED-1：返回 `false`（未解析）供调用方与随后的 `ancestor_close_by_id` 剪除结果交叉
/// 核对（`placeholder_pruned_by_ancok` probe）——原声明"可与 AncOK 剪除计数交叉核对"此前无
/// 对应计数，声明超出实装（090号声明膨胀）。
///
/// ★#347 LOW-1 / #446：自指排除守卫覆盖三级解析（base `id_idx`、`overlay_seen`、raw 兜底）。
/// 理论上 `parent_id` 不应等于自身 id，但若上游数据出现环形 `parent_id`
/// （如 `leg.op_parent == leg.id`），排除 `pidx == idx` 可防
/// `set_parent_attached(idx, idx, ..)` 自环——`.parent`（索引字段）自环会使下游 `element_depth`
/// 顺 `parent` 链上溯永不终止而挂起。
///
/// ★实测订正（held_leg_placeholder_self_parent_id_does_not_hang 排查坐实）：本函数的 `r==idx`
/// 守卫只堵住 `.parent`（索引）这一条链；同一份环形数据（`work[idx].parent_id == Some(work[idx].id)`）
/// 还独立驱动 `ancestor_close_by_id`→`ancestors_by_id_lookup` 沿 `.parent_id`（结构 ID）链上溯——
/// 那条链此前**无环检测**（其函数头旧注释"环不可能"的前提，仅对 `push_element_tree` 真树成立，
/// held-leg 占位的 `parent_id` 来自外部 `op_parent`，不受该约束），会独立挂起，且比本函数早触发
/// （调用序：本函数→`ancestor_close_by_id`）。故完整修复需两处（见 `ancestors_by_id_lookup` 头部
/// 订正的环检测）——单修本函数不足以让下方测试通过。测试坐实：
/// `held_leg_placeholder_self_parent_id_does_not_hang`。
pub(crate) fn rebuild_placeholder_parent_attached(
    work: &mut ElementView,
    idx: usize,
    parent_id: Option<ElementId>,
    id_idx: &std::collections::HashMap<ElementId, usize>,
    overlay_seen: &std::collections::HashMap<ElementId, usize>,
    raw: &[usize],
) -> bool {
    if let Some(pid) = parent_id {
        let pidx = id_idx
            .get(&pid)
            .copied()
            .filter(|&pidx| pidx != idx)
            .or_else(|| overlay_seen.get(&pid).copied().filter(|&pidx| pidx != idx))
            .or_else(|| {
                raw.iter()
                    .copied()
                    .filter(|&r| r != idx) // ★r==idx 排除守卫（自指保护，见函数头 #347 LOW-1）
                    .find(|&r| work.get(r).map(|e| e.id == pid).unwrap_or(false))
            });
        match pidx {
            Some(pidx) => {
                let p_eps = work[pidx].eps;
                work.set_parent_attached(idx, pidx, p_eps);
                true
            }
            None => {
                // ★票#315 probe：父不可解析（不伪造），纯只读计数，不改 work/raw 控制流。
                ancok_probe_bump(|p| p.placeholder_parent_unresolved += 1);
                false
            }
        }
    } else {
        true // parent_id=None（真边界胚元）：非"未解析"，不计入 unresolved/剪除交叉核对。
    }
}

/// ★票#350（code-review Standards 轴浮出：与测试 helper 逐行同构，抽取消除重复维护）：本 bar
/// `pending_parent_fixup` 累加器（held-leg 占位 + restore 链恢复元素共用，两个物化循环全部结束
/// 后调用）的统一 fixup——对每个 idx 调用 [`rebuild_placeholder_parent_attached`] 解析
/// parent/attached_dir，返回未解析 idx 列表（供调用方与 AncOK 剪除结果交叉核对，#347 MED-1）。
/// 生产路径（[`coverage_step_from_buckets_sep`]）与测试 helper 共用本函数，避免父解析循环两处
/// 独立维护、静默漂移。
pub(crate) fn resolve_pending_parent_fixups(
    work: &mut ElementView,
    pending: &[usize],
    id_idx: &std::collections::HashMap<ElementId, usize>,
    overlay_seen: &std::collections::HashMap<ElementId, usize>,
    raw: &[usize],
) -> Vec<usize> {
    let mut unresolved = Vec::new();
    for &idx in pending {
        let parent_id = work[idx].parent_id;
        let resolved = rebuild_placeholder_parent_attached(work, idx, parent_id, id_idx, overlay_seen, raw);
        if !resolved {
            unresolved.push(idx);
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
