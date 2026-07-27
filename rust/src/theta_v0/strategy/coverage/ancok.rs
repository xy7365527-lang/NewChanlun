use super::*;

// ════════════════════════════════════════════════════════════════════════════
//  AncOK L2 探针（697 号 ceiling 暴露面度量）——thread_local 计数器
// ════════════════════════════════════════════════════════════════════════════
// ponytail: 常驻探针，非临时诊断。697 ceiling 自陈「L2/L3 待验证」——保留探针 = 让任一真实
// 数据 run 可复测 Stale 四态分派频次与 restore 祖先链恢复率。增量成本（thread_local Cell +=1）
// 只在 held-leg Stale/restore 路径命中（cdylib 分类热路径不经此），被环绕的 HashMap 操作淹没。
// 计数纯旁路，不改任何 work/raw 控制流 ⟹ 生产 bit-exact 不变。

/// AncOK Stale 四态分派 + 祖先链恢复的运行时计数（anc.pdf §10 四态 + restore）。
///
/// 承载 697 号 ceiling 的**可观测暴露面**：`restore_break_registry_lost` > 0 ⟹ 至少一次祖先链
/// 上溯在 registry 单独判据下未能物化（本应有 parent 但 registry 已失去/作废）。
///
/// **订正（票#350）**：此前文档在此处断言"⟹ 声部树非严格"——该推论已被 #350 推翻：`>0` 只表示
/// registry **单独**这一条物化路径失败，**不再**蕴含该元素最终未被正确接线——若同一祖先在本 bar
/// 内经**另一条非 registry 路径**（如 `Closed|Invalidated`+`is_boundary_root` held 腿直接
/// push）物化，[`coverage_step_from_buckets_sep`] 函数尾部的统一 fixup（两个物化循环全部结束后
/// 执行）仍能正确接线（测试坐实：
/// `restore_chain_ancestor_unresolved_when_shared_ancestor_only_materializes_via_later_boundary_root_push`）。
/// 声部树是否真非严格，须看 [`placeholder_parent_unresolved`] 与 [`placeholder_pruned_by_ancok`]
/// 的**差值**（unresolved 但未被 AncOK 剪除 = 被 admit 却接线不上，**非环形 `parent_id` 数据下**
/// 该差值 #350 修复后结构性恒为 0；若真实数据观测到 >0，才是 697 ceiling 应处理的真实暴露面）——
/// `restore_break_registry_lost` 本身只是 registry 命中率的旁路诊断，非 ceiling 判据。
///
/// ★#358 影子评审订正：上述"恒为 0"不覆盖环形 `parent_id`（`op_parent`/`leg.parent_id` 自环或短环，
/// 见 [`rebuild_placeholder_parent_attached`] 头部 #347 LOW-1 订正）——环形数据下
/// `rebuild_placeholder_parent_attached` 的 `r != idx` 守卫仍可能记 `placeholder_parent_unresolved`，
/// 而 [`ancestors_by_id_lookup`] 的环检测在自环场景令 `chain=[自身]`，自身 id 必然 `⊆ raw_ids`
/// （e_idx 本就在 raw 中），[`ancestor_close_by_id`]（AncOK）因此**不剪除**该元素——即差值可能 >0。
/// 本节"恒为 0"的报警判据本身成立，但直接接生产 `assert!` 会在环形输入下误伤（090 严格性：
/// 声明域需与验证域一致，见 formalization-validity-domain）。
///
/// [`placeholder_parent_unresolved`]: AncokProbe::placeholder_parent_unresolved
/// [`placeholder_pruned_by_ancok`]: AncokProbe::placeholder_pruned_by_ancok
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AncokProbe {
    /// `HeldLegMatch::Stale` 命中总次数（snapshot 找不到持仓腿）。
    pub stale_arm: u64,
    /// Stale 内四态：LivePresent（Exact 未命中但 registry present——理论不可达，按持久身份保留）。
    pub state_live_present: u64,
    /// Stale 内四态：LiveDetached（持久存活、快照未展示——修复核心，触发 restore）。
    pub state_live_detached: u64,
    /// Stale 内四态：Closed/Invalidated 且 `is_boundary_root` ⟹ 作根保留（parent_id=None 合法）。
    pub closed_inval_boundary_kept: u64,
    /// Stale 内四态：Closed/Invalidated 且非边界根 ⟹ prune（诚实剪，建议6 选项1）。
    pub closed_inval_pruned: u64,
    /// `restore_ancestor_chain_from_registry` 调用总次数（LiveDetached 持仓腿 + open 候选父链）。
    pub restore_calls: u64,
    /// restore 自然收敛（cur=None，抵达真根，整条祖先链已恢复/复用完毕）。
    pub restore_complete: u64,
    /// restore 因祖先已在 raw 提前收敛（闭包已满足，合法完整）。
    pub restore_break_already_in_raw: u64,
    /// restore 因 registry 丢失/作废祖先提前中断（本应有 parent 但 registry 已失去）——registry
    /// 单独判据下的失败旁路诊断。**订正（票#350）**：`>0` 不再单独蕴含"声部树非严格"（见结构体头
    /// 文档订正）——该祖先仍可能经非 registry 路径同 bar 内物化、由统一 fixup 正确接线。697 ceiling
    /// 的真实可观测触发条件是 `placeholder_parent_unresolved - placeholder_pruned_by_ancok > 0`。
    pub restore_break_registry_lost: u64,
    /// ★票#315/#350（呼应 #301 探针族）：本 bar 待修补父/attached_dir 的元素（held 腿占位
    /// LivePresent/LiveDetached + restore 链恢复元素，#350 起两者共用同一统一 fixup，见
    /// `pending_parent_fixup`）在**两个物化循环 + 全部 restore 调用均结束后**统一 fixup 时点，
    /// `parent_id` 已知但经 id_idx/overlay_seen/raw 三级解析仍未命中（父本轮从未物化进 raw，
    /// 含真断链与"永不物化"两种成因）——[`rebuild_placeholder_parent_attached`] 保持 None/None
    /// 不伪造。>0 ⟹ 该路径在生产窗口被命中，可与 [`placeholder_pruned_by_ancok`] 交叉核对。
    /// 纯只读计数，不改 work/raw 控制流。
    ///
    /// ★#358 影子评审注记：本字段语义已随 #350 从"仅 held 腿占位"扩到"held 腿占位 + restore 链
    /// 恢复元素"共用同一统计口径——历史窗口（#350 前）读数与新读数不可直接比较（090 声明膨胀
    /// 禁令：同一字段名下统计口径变化须显式标注，不可默认可比）。
    ///
    /// [`placeholder_pruned_by_ancok`]: AncokProbe::placeholder_pruned_by_ancok
    pub placeholder_parent_unresolved: u64,
    /// ★#347 MED-1/#350：`placeholder_parent_unresolved` 命中的 idx 中，本 bar 随后确实被
    /// `ancestor_close_by_id`（AncOK）剪除（不在 `next_idx`）的个数——文档声称"可与 AncOK
    /// 剪除计数交叉核对"，此前无对应计数使该声明不可执行（#347 MED-1）。恒 ≤
    /// `placeholder_parent_unresolved`（父不可解析 ⟹ parent_id 链断在此 idx，理论上必被剪；
    /// 若 <，说明存在未被剪除的未解析占位，须回查 `ancestor_close_by_id` 判据是否有遗漏路径）。
    pub placeholder_pruned_by_ancok: u64,
}

thread_local! {
    static ANCOK_PROBE: std::cell::Cell<AncokProbe> = const { std::cell::Cell::new(AncokProbe {
        stale_arm: 0,
        state_live_present: 0,
        state_live_detached: 0,
        closed_inval_boundary_kept: 0,
        closed_inval_pruned: 0,
        restore_calls: 0,
        restore_complete: 0,
        restore_break_already_in_raw: 0,
        restore_break_registry_lost: 0,
        placeholder_parent_unresolved: 0,
        placeholder_pruned_by_ancok: 0,
    }) };
}

#[inline]
pub(crate) fn ancok_probe_bump(f: impl FnOnce(&mut AncokProbe)) {
    ANCOK_PROBE.with(|c| {
        let mut p = c.get();
        f(&mut p);
        c.set(p);
    });
}

/// 归零 AncOK 探针（L2 run 前调用）。
pub fn ancok_probe_reset() {
    ANCOK_PROBE.with(|c| c.set(AncokProbe::default()));
}

/// 读取 AncOK 探针快照（L2 run 后调用）。
pub fn ancok_probe_snapshot() -> AncokProbe {
    ANCOK_PROBE.with(std::cell::Cell::get)
}
/// 祖先链 `Anc(e)`（M16 AncOK：e 沿 parent 上溯的全部祖先元素索引）。
///
/// 从 e 的父 `parent` 出发，沿 parent 链上溯收集祖先（对齐 `AncestorClosure.ancestors`）。
/// 树深有限（塔级别有限）⟹ 链有限，无 fuel 需要（rust 用 while 上溯，环不可能——parent 索引
/// 严格小于子索引，`push_element_tree` 父在子前保证）。
///
/// ★codex Q4：此为**索引链**版本（per-bar Vec 索引），用于 §3 `active_set_step` 的 Lean M16
/// 区间递归原语对齐（非生产入场，GAP-5 note）。§13 生产 AncOK 用 [`ancestors_by_id`]（parent_id
/// 结构映射，spec §13 line 675 `p:C_ℓ→C_{ℓ+1}`）。
pub fn ancestors(elements: &[CoverageElement], e_idx: usize) -> Vec<usize> {
    let mut chain = Vec::new();
    let mut cur = elements.get(e_idx).and_then(|e| e.parent);
    while let Some(p) = cur {
        chain.push(p);
        cur = elements.get(p).and_then(|e| e.parent);
    }
    chain
}

/// ★codex Q4 祖先链（按 `parent_id` 结构映射，spec §13 `p:C_ℓ→C_{ℓ+1}`）。
///
/// 从 e 的父 `parent_id` 出发，沿 parent_id 链上溯收集祖先的 `ElementId`。跨 bar 稳定（ID 确定性），
/// 非每 bar Vec 索引。用于 §13 生产 AncOK（[`ancestor_close_by_id`]）——spec §13 line 671 硬约束
/// "子级短差腿存在 ⟹ 父容器存在"按结构映射判据，非 per-bar 索引。
///
/// 树深有限 ⟹ 链有限——对 `push_element_tree` 产的真树元素成立（父 level > 子 level，descend
/// 级别严格递减保证环不可能）。
///
/// ★#347 LOW-1 订正：held-leg 占位路径（`rebuild_placeholder_parent_attached` 调用方，coverage.rs
/// §held 腿占位）的 `parent_id` 来自外部 `ActiveLeg::op_parent`，**不**经 `push_element_tree` 构造，
/// 不受"父 level 严格更高"约束——若 `op_parent` 环形指向自身或更早祖先（数据错误/上游 bug），
/// 上方"环不可能"前提被违反，`lookup` 会在 `elements.overlay`（占位元素本身也在其中）里查到
/// 自己，`cur` 永不变为 `None`，本函数原实现会**挂起**（实测坐实：
/// `held_leg_placeholder_self_parent_id_does_not_hang`，纯树路径下该场景永不出现，故此前未暴露）。
/// 加 `seen` 环检测兜底——链元素有限（`elements.len()`），一旦重访同一 `ElementId` 立即终止，
/// 诚实返回"能收集到的祖先前缀"（非伪造完整链）。
pub(crate) fn ancestors_by_id_lookup(
    elements: &ElementView,
    e_idx: usize,
    lookup: &impl Fn(&ElementId) -> Option<usize>,
) -> Vec<ElementId> {
    let mut chain = Vec::new();
    let mut seen: std::collections::HashSet<ElementId> = std::collections::HashSet::new();
    let mut cur = elements.get(e_idx).and_then(|e| e.parent_id);
    while let Some(pid) = cur {
        if !seen.insert(pid) {
            break; // 环检测：pid 已在链中出现过（held-leg 占位 op_parent 破坏"环不可能"前提）。
        }
        chain.push(pid);
        // 按 parent_id 查下一级祖先（双段 lookup：base 缓存 + overlay，§16 tree 前缀不变）。
        cur = lookup(&pid).and_then(|pidx| elements.get(pidx).and_then(|e| e.parent_id));
    }
    chain
}

/// 先关后开活动集 `(A_t∖D_t)∪B_t`（M16 §五 `targetActiveSet`，祖先闭合前）。
///
/// `active`（A_t）先滤除结束元素 `ending`（D_t），再并入新开始元素 `starting`（B_t）。
/// **关闭在前 开启在后**（对齐 `SeparateStrategyTarget.targetActiveSet`）。返回去重后的索引集
/// （同一元素不重复——B_t 中已在 A_t∖D_t 的不重复加）。
pub(crate) fn raw_active_set(active: &[usize], ending: &[usize], starting: &[usize]) -> Vec<usize> {
    // ponytail: HashSet O(1) 替 Vec.contains O(n)——ending/starting 去重查询从线性降常数
    let ending_set: std::collections::HashSet<usize> = ending.iter().copied().collect();
    let mut raw: Vec<usize> = active
        .iter()
        .copied()
        .filter(|i| !ending_set.contains(i))
        .collect();
    let mut raw_set: std::collections::HashSet<usize> = raw.iter().copied().collect();
    for &b in starting {
        if raw_set.insert(b) {
            raw.push(b);
        }
    }
    raw
}

/// 祖先闭合 `AncOK(B) = {e∈B : Anc(e)⊆B}`（M16 §八，裁掉祖先不齐的元素）。
///
/// 保留「全部祖先也在 raw 集中」的元素——子激活 ⟹ 全祖先在场（覆盖不漂浮，对齐
/// `AncestorClosure.ancOK`）。祖先不齐（父不在 raw）的元素被裁掉（子声部不漂浮在不存在的父上）。
///
/// ★此为**索引链**版本（§3 Lean M16 区间递归原语对齐，非生产入场）。§13 生产 AncOK 用
/// [`ancestor_close_by_id`]（parent_id 结构映射，spec §13）。
pub(crate) fn ancestor_close(elements: &[CoverageElement], raw: &[usize]) -> Vec<usize> {
    // ponytail: HashSet O(1) 替 raw.contains O(n)——祖先查询从线性降常数
    let raw_set: std::collections::HashSet<usize> = raw.iter().copied().collect();
    raw.iter()
        .copied()
        .filter(|&e_idx| {
            ancestors(elements, e_idx)
                .iter()
                .all(|a| raw_set.contains(a))
        })
        .collect()
}

/// ★codex Q4 祖先闭合（按 `parent_id` 结构映射，spec §13 line 661/665/671）。
///
/// `AncOK(A)={a∈A:Anc(a)⊆A}`——保留「全部祖先（沿 parent_id 链）也在 raw 中」的元素。
/// 祖先判据用 `parent_id`（跨 bar 稳定的 ElementId，spec §13 `p:C_ℓ→C_{ℓ+1}` 结构映射），
/// 非 per-bar Vec 索引。祖先不齐（父不在 raw_ids）的元素被裁掉——spec §13 line 671 硬约束
/// "任何子级短差腿存在时，它的父容器也存在"。
///
/// ★与 [`ancestor_close`] 的区别：[`ancestor_close`] 按 `parent: Option<usize>` 索引链（§3 Lean
/// M16 原语），本函数按 `parent_id: Option<ElementId>` 结构映射链（§13 生产）。两者在单 bar 内
/// 元素集上等价（parent 索引与 parent_id 一一对应），但本函数的判据跨 bar 稳定（ID 确定性）。
pub(crate) fn ancestor_close_by_id(elements: &ElementView, raw: &[usize]) -> Vec<usize> {
    // raw_ids：raw 中元素的 id 集合（结构映射判据）。
    let raw_ids: std::collections::HashSet<ElementId> =
        raw.iter().filter_map(|&i| elements.get(i).map(|e| e.id)).collect();
    // ★工位 4f：id→idx 索引——base 段复用缓存 `base_id_idx`（§16 tree 前缀不变，O(1) 命中），仅 overlay
    // 段（candidate++restore，绝大多数 bar 空）现建小 map（全局 idx = base_len + i）。旧版每 bar
    // `elements.iter()` 全量 rebuild O(work)=O(tree)/bar=O(n²)。bit-exact：双段查 == 全量 map（base
    // id 与 overlay id 不交叠——overlay 是候选叶子/restore 追加，与 tree id 碰撞时 base 优先，与旧
    // `or_insert` 首次出现序一致：旧 collect 按 iter 序 base 在前 ⟹ base 先插，等价 base 优先）。
    let base_id_idx_owned;
    let base_id_idx: &std::collections::HashMap<ElementId, usize> = match &elements.base_id_idx {
        Some(rc) => rc.as_ref(),
        None => {
            base_id_idx_owned = build_tree_id_index(elements.base);
            &base_id_idx_owned
        }
    };
    let base_len = elements.base.len();
    let mut overlay_id_idx: std::collections::HashMap<ElementId, usize> =
        std::collections::HashMap::new();
    for (i, e) in elements.overlay.iter().enumerate() {
        overlay_id_idx.entry(e.id).or_insert(base_len + i);
    }
    // 查找闭包：base 优先（与旧全量 map iter 序 base 在前一致），overlay fallback。
    let lookup = |pid: &ElementId| -> Option<usize> {
        base_id_idx.get(pid).copied().or_else(|| overlay_id_idx.get(pid).copied())
    };
    raw.iter()
        .copied()
        .filter(|&e_idx| {
            ancestors_by_id_lookup(elements, e_idx, &lookup)
                .iter()
                .all(|a| raw_ids.contains(a))
        })
        .collect()
}

/// **活动集一步递归 `A_{t+1} = AncOK[(A_t∖D_t)∪B_t]`**（M16 顶点，先关后开 + 祖先闭合）。
///
/// ★票#247 缺口一（声明一致）：本函数是 Lean M16 **理想式原语**（§3，GAP-5 note 非生产入场），
/// 无第三来源。生产 §8 路径完整转移为 `A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x∪RegistryRestore]`
/// （RegistryRestore=registry 持久祖先物化，anc.pdf §11，非新数学来源），
/// 见 [`coverage_step_from_buckets`] + [`restore_ancestor_chain_from_registry`]。
///
/// 给定当前活动集 `active`（A_t）+ 当前 bar t，从全元素集 `elements` 算 B_t（λ_e=t）/ D_t（ρ_e=t），
/// 先关后开得 raw `(A_t∖D_t)∪B_t`，再施祖先闭合 AncOK 裁掉祖先不齐者。返回 A_{t+1}（新集合，
/// immutable——不 mutate `active`）。对齐 `AncestorClosure.ancestorClose`。
pub fn active_set_step(elements: &[CoverageElement], active: &[usize], t: usize) -> Vec<usize> {
    let starting = starting_set(elements, t);
    let ending = ending_set(elements, t);
    let raw = raw_active_set(active, &ending, &starting);
    ancestor_close(elements, &raw)
}

#[cfg(test)]
#[path = "ancok_tests.rs"]
mod tests;
