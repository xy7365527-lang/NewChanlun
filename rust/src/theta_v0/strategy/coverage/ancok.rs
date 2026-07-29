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
/// 承载 697 号 ceiling 的**可观测暴露面**：`restore_break_registry_lost` > 0 ⟺ 存在被 admit 的
/// 子声部腿其操作祖先链未被 registry 完整恢复（本应有 parent 但 registry 已失去）⟹ 声部树非严格。
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
    /// ★暴露面：restore 因 registry 丢失/作废祖先提前中断（本应有 parent 但 registry 已失去）。
    /// >0 ⟹ 被 admit 子声部腿祖先链未完整 ⟹ 声部树非严格（697 ceiling 的可观测触发条件）。
    pub restore_break_registry_lost: u64,
    /// ★#226：restore 遇**当 bar 已被裁决终结**的祖先（∈ 𝒟_x 关闭种子）提前中断——open 父注入
    /// 路径（ℬ_x 段无 𝒟_x^† 种子兜底）不得复活被关父（S3 子树清仓；m3 win9 断言①炸点根因）。
    pub restore_break_closed_seed: u64,
    /// ★#233/#269：held 对位翻向守卫命中——**事件口径**（#269 事件化，#261 终裁选项 A）：
    /// 载体 registry 首见方向（tree 来源，I2 机器锁永固）≠ 当前树元素方向 ⟹ 载体被 frontier
    /// 重组改判 = 翻向**事件** ⟹ 声部终结（#227 裁决蓝图两步形①；事件在场时方向盲 ID 对位
    /// 复活废除）。谓词见 [`super::super::persistent::PersistentRegistry::direction_flip_event_active`]。
    /// **已作废的旧描述**：#233 状态轴判据「树元素方向 ≠ **持仓腿**方向即终结」——BSP 构造下
    /// σ=−ε 出生即恒真（#264 §2.1）⟹ 出生对立腿 t+1 必剪（wf8 92.3% 持仓恰好 1 bar），非本
    /// 探针现语义。翻向腿进当 bar 翻向种子，经现成 𝒟_x^† 子树清仓连清其后代（父不在 A 的
    /// 后代数济判据，exit.rs subtree_close）。
    pub held_flip_terminated: u64,
    /// ★#247 缺口二：restore 恢复元素的 `parent` 索引/`attached_dir` **重建成功**次数
    /// （`parent_id` 解析到 work 内某 idx ⟹ 角色输入 (σ_p, ℓ_p) 真实，不再恒定 Ambient/SameLevel）。
    pub restore_parent_rebound: u64,
    /// ★#247 缺口二：restore 恢复元素 `parent_id=Some` 但**祖先链断裂**（registry 丢失 / 当 bar
    /// 关闭种子中断）⟹ 父不在 work ⟹ `parent` 留 None。**与「真边界胚元 ∂」及 `operation_role`
    /// 的「父越界防御分支」语义分离**：该元素 `parent_id` 仍为 Some ⟹ 统一 AncOK
    /// （[`super::super::exit::step_active_set_with_subtree_close`]）按 id 判祖先不在集 ⟹ **必被剪除**，
    /// 不进 `next_idx`/`strategy_target_legs` ⟹ 其角色从不被消费（见 `restore_broken_chain_*` 测试）。
    pub restore_parent_unresolved: u64,
    /// ★票#347 MED-1（语义重放自 kimi #346/#347/#358）：`restore_parent_unresolved`/
    /// `placeholder_parent_unresolved` 命中的 idx 中，本 bar 随后确实被统一 AncOK（不在
    /// `next_idx`）剪除的个数——文档声称"可与 AncOK 剪除计数交叉核对"，此前无对应计数使该
    /// 声明不可执行。恒 ≤ 未解析计数（父不可解析 ⟹ `parent_id` 链断在此 idx，理论上必被剪；
    /// 若 <，说明存在未被剪除的未解析占位，须回查 `ancestor_close_by_id`/`step_active_set_
    /// with_subtree_close` 判据是否有遗漏路径）。**非环形 `parent_id` 数据下**该差值结构性
    /// 恒为 0（票#358 订正：环形 `parent_id` 下 `r != idx` 自环排除守卫会使某未解析元素同时
    /// 经环路径存活进 `next_idx`，此时差值可能 >0，报警判据仍对但不可接生产硬 assert）。
    pub placeholder_pruned_by_ancok: u64,
    /// ★#247 C1（影子评审阻断级）：[`element_depth`] 的 **fuel 上界硬门**命中次数——沿 `parent` 链
    /// 上溯步数超过 `elements.len()` ⟹ 该 parent 图**必含环**（简单路径最长 len−1 条边）。
    /// #247 缺口二回填首次让 restore 元素的 `parent` 可指向 overlay ⟹ 链的无环性转而依赖 registry
    /// `structural_parent_id` 无环，而写入侧（`persistent.rs` 三处直接赋值、跨 bar 可变更新）**无任何
    /// 无环校验** ⟹ 环在数据层不可排除。命中即 bump 本探针**并显式 panic**（不静默钳制：钳制会把
    /// 环化 parent 图伪装成合法深度、静默改 `w_depth` ⟹ 下单权重被脏数据污染而无告警）。
    /// 与 [`super::super::exit::step_active_set_with_subtree_close`] 的 AncOK fuel 门同族（那里超限保守剪除
    /// 一条腿即可降级；此处无降级路径——不设门 = 生产进程挂死）。
    pub element_depth_fuel_exhausted: u64,
    /// ★#594：[`super::step::risk_seed_carrier`] 多 carrier 歧义 fail-closed 命中次数——父
    /// campaign 仍照常 RiskExit，但因身份歧义未能补 risk-close seed ⟹ 该子树后代不被连坐
    /// 清除，残留裸腿（#577 评审尾巴①暴露面：该丢弃此前零可观测）。
    pub risk_seed_carrier_ambiguous: u64,
    /// ★#446：`next_idx`（活动集推进产出）中至少出现一对重复 `ElementId` 的 step 次数。三处
    /// 注册路径（held 重注册/restore 复用/open 候选按 id 判重，#216）按 ID 闭合后应恒为 0；
    /// release 下 `debug_assert!` 被编译消除 ⟹ 该不变量在 release 静默失守（生产 dump 实锤，
    /// p3fold carrier ElementId(0,162)）——此计数在 release/debug 都累计，供真实跑批逐窗验收。
    pub duplicate_active_id_violations: u64,
    /// ★#694（语义重放自 kimi #346/#347 LOW-1，改用本仓已确立的 fuel 硬门惯例而非原提交的
    /// 静默 `seen` HashSet break——见 [`ancestors_by_id_lookup`] 函数头）：`ancestors_by_id_lookup`
    /// 沿 `parent_id` 链上溯步数超过 `elements.len()` ⟹ 图含环，命中即计入本探针**并显式 panic**
    /// （与 [`element_depth`] 的 `element_depth_fuel_exhausted`/#247 C1 同族同风格：钳制/静默截断
    /// 会把环化 `parent_id` 图伪装成一条合法（但残缺）的祖先链，本函数唯一生产消费方
    /// `apply_gross_cap`（G7 毛头寸约束，`risk.enforce_gross_cap` 门控，非 default）据此分组，
    /// 静默失真会让分组归属被脏数据污染而无告警面）。
    pub ancestors_by_id_lookup_fuel_exhausted: u64,
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
        restore_break_closed_seed: 0,
        held_flip_terminated: 0,
        restore_parent_rebound: 0,
        restore_parent_unresolved: 0,
        placeholder_pruned_by_ancok: 0,
        element_depth_fuel_exhausted: 0,
        risk_seed_carrier_ambiguous: 0,
        duplicate_active_id_violations: 0,
        ancestors_by_id_lookup_fuel_exhausted: 0,
    }) };
}

#[inline]
pub(super) fn ancok_probe_bump(f: impl FnOnce(&mut AncokProbe)) {
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
/// 非每 bar Vec 索引。现服务于根子树分组（`parent_id` 结构映射判据，spec §13 line 671 硬约束
/// "子级短差腿存在 ⟹ 父容器存在"）；§13 生产 AncOK 闭合已由
/// [`super::super::exit::step_active_set_with_subtree_close`] 接管（#183 归一）。
///
/// 树深有限 ⟹ 链有限——对 `push_element_tree` 产的真树元素成立（父 level > 子 level，descend
/// 级别严格递减保证环不可能）。
///
/// ★#694 订正（语义重放自 kimi #346/#347 LOW-1）：唯一生产消费方 [`super::leg::apply_gross_cap`]
/// 的 `root_key` 分组（G7 毛头寸约束，`risk.enforce_gross_cap` 门控——default false，非默认生产
/// 路径，仅 wverify 类实跑经环境变量开启）喂入的 `e_idx` 可来自 held-leg 占位——其 `parent_id`
/// 来自外部 `ActiveLeg::op_parent`，**不**经 `push_element_tree` 构造，不受"父 level 严格更高"
/// 约束（同 held.rs `resolve_pending_parent_fixups` 头部 #347 LOW-1 记述的同族数据来源）。若
/// `op_parent` 环形指向自身或更早祖先（数据错误/上游 bug），上方"环不可能"前提被违反，本函数
/// 裸 `while let` 会**挂起**（同 #247 C1 `element_depth` 记述的失效模式，parent_id 图无环性同样
/// "不能白拿"）。
///
/// 硬门风格与本仓已确立惯例统一（`element_depth` #247 C1 / `subtree_close`/
/// `step_active_set_with_subtree_close` 的 `steps > bound`）：上溯步数 `> elements.len()` ⟹
/// 图必含环（简单路径最长 `len−1` 条边）⟹ 计入探针
/// [`AncokProbe::ancestors_by_id_lookup_fuel_exhausted`] 后**显式 panic**——不静默截断成一条
/// 合法但残缺的祖先链（钳制会让分组 `root_key` 被脏数据污染且无告警面，`element_depth` 同一
/// 纪律）。
pub(super) fn ancestors_by_id_lookup(
    elements: &ElementView,
    e_idx: usize,
    lookup: &impl Fn(&ElementId) -> Option<usize>,
) -> Vec<ElementId> {
    let fuel = elements.len();
    let mut chain = Vec::new();
    let mut cur = elements.get(e_idx).and_then(|e| e.parent_id);
    let mut steps = 0usize;
    while let Some(pid) = cur {
        steps += 1;
        if steps > fuel {
            ancok_probe_bump(|p| p.ancestors_by_id_lookup_fuel_exhausted += 1);
            panic!(
                "#694 环路硬门：ancestors_by_id_lookup 自 idx={e_idx} 上溯 {steps} 步超过 fuel \
                 上界 {fuel}（=elements.len()）⟹ parent_id 图含环（held-leg 占位 op_parent 无环性\
                 无写入侧保障）。显式失败，不静默截断。"
            );
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
fn raw_active_set(active: &[usize], ending: &[usize], starting: &[usize]) -> Vec<usize> {
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
/// ★此为**索引链**版本（§3 Lean M16 区间递归原语对齐，非生产入场）。§13 生产 AncOK 由
/// [`super::super::exit::step_active_set_with_subtree_close`] 接管（#183 归一：parent_id 结构映射判据
/// 的散装等价物 `ancestor_close_by_id` 已下线，归一即删除）。
fn ancestor_close(elements: &[CoverageElement], raw: &[usize]) -> Vec<usize> {
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

/// **活动集一步递归 `A_{t+1} = AncOK[(A_t∖D_t)∪B_t]`**（M16 顶点，先关后开 + 祖先闭合）。
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

// ════════════════════════════════════════════════════════════════════════════
//  §4 操作角色 R(g)=(H(g),V(g),δ_g) 24 类完全分类（spec §7-§8 / P6-P7，去根化）
// ════════════════════════════════════════════════════════════════════════════


#[cfg(test)]
#[path = "ancok_tests.rs"]
mod tests;
