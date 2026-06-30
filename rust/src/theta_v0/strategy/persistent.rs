//! # Persistent Element Layer Pi（anc.pdf §4-§9 最小修复 = persistent overlay）
//!
//! ## 根因（anc.pdf §1-§3）
//! Q4 修了 ID 确定性 + 不伪造根，但**没修跨 bar 持久身份**。每 bar `Ei=extract_elements(Di)` 全量重建
//! → held 腿 pid 在新树找不到（held_exact CL 0.8%/BTC 27.8%）→ Stale → depth>0 腿全被 AncOK 剪 → #5α=0。
//!
//! Q4 把 **LiveDetached**（持久存在但 snapshot 找不到）误处理成 Stale/Invalidated → depth>0 被杀。
//!
//! ## 修复（anc.pdf §9 最小修复 = persistent overlay）
//! 保留 `Ei=extract_elements(Di)`（快照），新增 `Pi`（持久注册表）。
//! `Pi+1 = merge(Pi, Ei+1, held legs)`：
//! - snapshot 能匹配的元素：刷新 snapshot fields
//! - held 引用但 snapshot 找不到的元素：**不删除**，标记 `LiveDetached`
//! - 只有显式 close/risk close/successor invalidation 才删 persistent element
//!
//! ## 不变量 I1-I5（anc.pdf §7）
//! - I1 持久身份：腿未关闭 ⟹ pid(e)∈Pi 所有后续 bar
//! - I2 元素方向不变：δi(e)=δj(e)
//! - I3 parent 是关系非身份：pid(e) 不依赖 parent(e)
//! - I4 操作父持久：op_parent(L)=c ⟹ c∈Pi 或 c⇝c'（确定性 successor）
//! - I5 AncOK 作用 persistent set
//!
//! ## 认识论等级
//! - L0：不变量 I1-I5 是定义性（从 persistent registry 定义推导）
//! - L1：bit-exact（增量==全量，两者都按 persistent overlay）
//! - L2/L3：Stale↓ + depth>0 admission↑ + ΔSharpe≠0 = 待真实数据验证（acceptance[2]）
//!
//! ## ponytail ceiling
//! 本模块是**最小修复**（persistent overlay）。彻底修复（增量 extract_elements，confirmed prefix
//! immutable，frontier only mutable）标为 ceiling——见 anc.pdf §9/§16。当前 overlay 诚实表达
//! "操作记忆还在，当前重建树没把它展示出来"（LiveDetached 非伪造 root）。

use super::coverage::CoverageElement;
use super::interp::ActiveLeg;
use super::voice::VoiceSide;
use crate::theta_v0::classifier::recursive_tower::ElementId;

// ════════════════════════════════════════════════════════════════════════════
//  §10 held leg 四状态（anc.pdf §10）
// ════════════════════════════════════════════════════════════════════════════

/// held 引用元素的四态分类（anc.pdf §10，互斥且穷尽）。
///
/// Q4 当前把 `LiveDetached` 错误合并进 `Invalidated/Stale` → 系统性剪枝的数学根源。
/// 修复：`LiveDetached` 不是 root，也不是 fake parent=None。它的 parent 仍然是 `op_parent(L)`，
/// 只是当前 snapshot 没有展示这个元素。
///
/// `ValidHeldi(L) := pid(L)∈Pi ∧ ¬Invalidatedi(L)`（非 `pid(L)∈Ei`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldLegState {
    /// 持久存在，当前快照也存在（anc.pdf §10）。
    LivePresent,
    /// 持久存在，但当前快照找不到（anc.pdf §10）—— Q4 误处理成 Stale，修复为保留。
    /// 不是 root，parent 仍是 op_parent(L)，只是 snapshot 没展示。
    LiveDetached,
    /// 显式平仓结束（anc.pdf §10）。
    Closed,
    /// 结构或风险显式作废（anc.pdf §10）。
    Invalidated,
}

impl HeldLegState {
    /// `RegistryLivei(u) := u∈Pi ∧ status∈{LivePresent, LiveDetached}`（anc.pdf §8 I2）。
    pub fn registry_live(self) -> bool {
        matches!(self, HeldLegState::LivePresent | HeldLegState::LiveDetached)
    }

    /// `SnapshotPresenti(u) := u∈Id(Ei)`（anc.pdf §8 I2，附加状态）。
    /// LivePresent = registry_live ∧ snapshot_present；LiveDetached = registry_live ∧ !snapshot_present。
    pub fn snapshot_present(self) -> bool {
        matches!(self, HeldLegState::LivePresent)
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  §4-§5 Persistent Element Layer Pi
// ════════════════════════════════════════════════════════════════════════════

/// 持久注册表条目：一个跨 bar 存活的元素（anc.pdf §4-§5）。
///
/// `pid(e) = H(ℓ(e), start_anchor(e), end_anchor(e), δ(e), kind(e), tie_break(e))`（§5）——
/// **不含父路径**（Vec index/root path/parent path/最高级容器/树形位置），这些会变。
/// parent 是另一张表 `parenti: Pi→Pi∪{⊥}`（更新关系不重命名元素）。
///
/// ★ID 内禀性（I3）：`pid` 字段是元素身份；`structural_parent_id` 是当前关系（可变）。
/// "元素 ID 表示'它是谁'；parent 表示'它现在被谁包含'"（§5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistentElement {
    /// 内禀身份（§5 I3）：H(ℓ, start_anchor, end_anchor, δ, kind, tie_break)，不含父路径。
    pub pid: ElementId,
    /// 方向 δ(e)（I2 不变量：不变）。
    pub dir: VoiceSide,
    /// 级别 ℓ(e)。
    pub level: u32,
    /// λ=start_anchor（左端点，稳定语义身份坐标，§5）。
    pub lambda: usize,
    /// ρ=end_anchor（右端点，会随父延伸漂移，§5）。
    pub rho: usize,
    /// 结构父 pstr_i(e)（§6：当前 snapshot 中的结构父关系，可随更高级别涌现而变化）。
    /// I3：parent 是关系非身份——更新此字段不改 pid。
    pub structural_parent_id: Option<ElementId>,
    /// snapshot_present：当前 bar 的 Ei 是否包含此元素（§8 I2 附加状态）。
    /// LivePresent=true，LiveDetached=false。
    pub snapshot_present: bool,
    /// 显式作废标记（§10：只有 close/risk close/invalidation 才让元素退出 live）。
    pub invalidated: bool,
}

/// 持久注册表 Pi（anc.pdf §4）。
///
/// `Pi+1 = merge(Pi, Ei+1, held legs)`（§4/§9）。held 腿引用 `pid∈Pi`，而非直接引用 `Ei`。
/// 跨 bar 存活：腿未显式关闭 ⟹ pid∈Pi 所有后续 bar（I1）。
#[derive(Debug, Clone, Default)]
pub struct PersistentRegistry {
    /// pid → PersistentElement。
    elements: std::collections::HashMap<ElementId, PersistentElement>,
    /// ★工位 K 性能：上一 bar snapshot_present=true 的 tree 段 pid 集合（增量 merge step 1' 用）。
    /// 旧 merge 每 bar 全量 `values_mut()` 扫 registry 置 false（O(registry)，单调增 ⟹ O(n²)）。改增量：
    /// 只把"上 bar present 但本 bar 不 present"的 pid 置 false（O(上 bar snapshot)）。
    /// ★工位 4f：拆 tree/cand 两段——tree_dirty=false（TreeKey 命中）⟹ tree ids 同上 bar ⟹ 全 present
    /// ⟹ step 1' 跳过 tree 段（不重建 snapshot_ids 全集，消第二处 O(tree)/bar）。
    present_last_tree: std::collections::HashSet<ElementId>,
    /// 上一 bar present 的 candidate 段 pid（每 bar 变，O(candidate) 有界，恒重建）。
    present_last_cand: std::collections::HashSet<ElementId>,
}

impl PersistentRegistry {
    /// 空注册表（bar 0 初始）。
    pub fn new() -> Self {
        Self::default()
    }

    /// `RegistryLivei(u) := u∈Pi ∧ status∈{LivePresent, LiveDetached}`（§8 I2）。
    pub fn registry_live(&self, pid: &ElementId) -> bool {
        self.elements
            .get(pid)
            .map(|e| !e.invalidated)
            .unwrap_or(false)
    }

    /// `SnapshotPresenti(u) := u∈Id(Ei)`（§8 I2 附加状态）。
    pub fn snapshot_present(&self, pid: &ElementId) -> bool {
        self.elements
            .get(pid)
            .map(|e| e.snapshot_present && !e.invalidated)
            .unwrap_or(false)
    }

    /// 查询持久元素（None = 不在注册表）。
    pub fn get(&self, pid: &ElementId) -> Option<&PersistentElement> {
        self.elements.get(pid)
    }

    /// 持久元素数量。
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// 标记元素为显式作废（§10：结构/风险显式作废）。
    /// 只有 close signal / risk close / invalidation 才让 held leg 退出 registry live set（§9 rule 5）。
    pub fn invalidate(&mut self, pid: &ElementId) {
        if let Some(e) = self.elements.get_mut(pid) {
            e.invalidated = true;
        }
    }

    /// 标记元素为显式关闭（§10：显式平仓结束）→ 从 live set 退出。
    pub fn close(&mut self, pid: &ElementId) {
        if let Some(e) = self.elements.get_mut(pid) {
            e.invalidated = true;
        }
    }

    // ══════════════════════════════════════════════════════════════════════════
    //  §9 merge: Pi+1 = merge(Pi, Ei+1, held legs)
    //  ══════════════════════════════════════════════════════════════════════════

    /// **§9 最小修复 merge**：`Pi+1 = merge(Pi, Ei+1, held legs)`。
    ///
    /// 规则（§9）：
    /// 1. 当前 snapshot 中能匹配到的元素，刷新其 snapshot fields（snapshot_present=true）
    /// 2. held 引用但 snapshot 找不到的元素，**不删除**，标记 LiveDetached（snapshot_present=false）
    /// 3. Detached held leg 不允许开新子腿，但按自己退出规则继续/显式关闭
    /// 4. 只有显式 close / risk close / invalidation 才删除 persistent element（live 退出）
    ///
    /// **immutable**：返回新 registry（不 mutate self，coding-style.md immutability）。
    pub fn merge(
        &self,
        snapshot: &[CoverageElement],
        held_legs: &[ActiveLeg],
    ) -> PersistentRegistry {
        // immutable 包装（保留 §9 immutable 契约 + 测试调用面）；生产路径用 merge_in_place 避免 clone。
        let mut next = self.clone();
        next.merge_in_place(snapshot, held_legs);
        next
    }

    /// **§9 merge 原地增量版**（工位 K 性能：消除 `self.clone()` O(registry) + 全量 `values_mut()` 扫）。
    ///
    /// bit-exact == [`merge`]：所有调用点用 `reg = reg.merge(..)` consume-replace 语义，原地 mutate 等价。
    /// 增量 snapshot_present 重置：旧版每 bar 扫全 registry（O(registry)，registry 单调增 ⟹ O(n²)）；
    /// 本版只重置"上 bar present 但本 bar 不 present"的 pid（O(上 bar snapshot)）。soundness：snapshot_present
    /// 只在 step 1 snapshot upsert 置 true（held/op_parent 用 `or_insert(false)` 不翻已存在的真值）⟹
    /// 任何 present=true 的元素必在上 bar snapshot ⟹ 必在 `present_last` ⟹ 全量扫与增量扫产同状态。
    pub fn merge_in_place(&mut self, snapshot: &[CoverageElement], held_legs: &[ActiveLeg]) {
        // 单段口径（tree 段恒 dirty=true，全量 upsert）= 旧行为；split 见 [`merge_in_place_split`]。
        self.merge_in_place_split(snapshot, true, &[], held_legs);
    }

    /// **§9 merge 双段增量版（工位 4f：confirmed prefix 增量维护——消 step 1'/2' 的 O(tree)/bar）**。
    ///
    /// snapshot 拆为 `tree`（∝confirmed，TreeCache 缓存）+ `candidates`（每 bar 变，有界）。`tree_dirty=false`
    /// （TreeKey 命中 ⟹ tree 逐字节同上 bar）⟹ **跳过 tree 段 step 1'/2'**（这些元素上 bar 已写同值 + 全 present）。
    ///
    /// bit-exact == 全量（`tree_dirty=true` 恒走全量）：
    /// - step 2' 跳过：tree_dirty=false ⟹ tree 同上 bar ⟹ 上 bar upsert 值 == 本 bar 会写值（断言1）。candidate
    ///   段恒 upsert，且 carrier id 碰撞时 upsert 覆盖**全部** tree-derived 字段 ⟹ 末态 = candidate 值（断言2）。
    /// - step 1' 跳过 tree：tree_dirty=false ⟹ present_last_tree 中每 id 仍在本 bar tree ⟹ 全 present ⟹ 不该置
    ///   false（断言3）。candidate 段恒查：present_last_cand 中本 bar 在 `cand_ids ∪ tree_ids` 找不到的才置 false
    ///   （carrier id 上 bar 在 cand 本 bar 退回 tree 仍 present，不误置）。
    pub fn merge_in_place_split(
        &mut self,
        tree: &[CoverageElement],
        tree_dirty: bool,
        candidates: &[CoverageElement],
        held_legs: &[ActiveLeg],
    ) {
        let tree_ids: std::collections::HashSet<ElementId> = if tree_dirty {
            tree.iter().map(|e| e.id).collect()
        } else {
            // tree_dirty=false ⟹ tree ids 同上 bar present_last_tree（不重建，消 O(tree)）。
            self.present_last_tree.clone()
        };
        let cand_ids: std::collections::HashSet<ElementId> =
            candidates.iter().map(|e| e.id).collect();

        // 1'. 增量重置：上 bar present 但本 bar snapshot 找不到 → snapshot_present=false（LiveDetached）。
        //     tree 段：tree_dirty=true 时按新 tree 重置上 bar present_last_tree 里不在新 tree 的；
        //     tree_dirty=false 时跳过（present_last_tree 全在本 bar tree ⟹ 全 present，断言3）。
        if tree_dirty {
            for pid in &self.present_last_tree {
                if !tree_ids.contains(pid) {
                    if let Some(e) = self.elements.get_mut(pid) {
                        e.snapshot_present = false;
                    }
                }
            }
        }
        // candidate 段：本 bar 在 cand ∪ tree 都找不到的才置 false（carrier id 退回 tree 仍 present）。
        for pid in &self.present_last_cand {
            if !cand_ids.contains(pid) && !tree_ids.contains(pid) {
                if let Some(e) = self.elements.get_mut(pid) {
                    e.snapshot_present = false;
                }
            }
        }

        // 2'. upsert：snapshot 中所有元素刷新 snapshot_present=true + structural_parent_id/rho（§9 rule 1）。
        //     ★工位 4f：tree_dirty=false ⟹ 跳过 tree 段（上 bar 已写同值，断言1）；candidate 段恒 upsert。
        let upsert = |elements: &mut std::collections::HashMap<ElementId, PersistentElement>, e: &CoverageElement| {
            let pid = e.id;
            let entry = elements.entry(pid).or_insert(PersistentElement {
                pid,
                dir: e.eps,
                level: e.level,
                lambda: e.lambda,
                rho: e.rho,
                structural_parent_id: e.parent_id,
                snapshot_present: true,
                invalidated: false,
            });
            entry.dir = e.eps;
            entry.level = e.level;
            entry.lambda = e.lambda;
            entry.rho = e.rho;
            entry.structural_parent_id = e.parent_id;
            entry.snapshot_present = true;
            // 不重置 invalidated（显式作废持久，§9 rule 5）。
        };
        if tree_dirty {
            for e in tree {
                upsert(&mut self.elements, e);
            }
        }
        // candidate 段恒 upsert（断言2：碰撞 carrier id 覆盖全字段，须在 tree 之后保覆盖序与全量一致）。
        for e in candidates {
            upsert(&mut self.elements, e);
        }

        // 3'. held 引用但 snapshot 找不到的元素：**不删除**（§9 rule 2，I1 持久身份）。
        for leg in held_legs {
            self.elements.entry(leg.id).or_insert(PersistentElement {
                pid: leg.id,
                dir: leg.dir,
                level: leg.level,
                lambda: leg.lambda,
                rho: leg.source_index,
                structural_parent_id: leg.parent_id,
                snapshot_present: false, // LiveDetached（snapshot 找不到）
                invalidated: false,
            });
            // ★I4（anc.pdf §7）：操作父容器持久。
            if let Some(op_pid) = leg.op_parent {
                self.elements.entry(op_pid).or_insert(PersistentElement {
                    pid: op_pid,
                    dir: leg.dir,
                    level: leg.level + 1,
                    lambda: leg.lambda,
                    rho: leg.source_index,
                    structural_parent_id: None,
                    snapshot_present: false,
                    invalidated: false,
                });
            }
        }

        // present_last 更新为本 bar snapshot ids（下 bar 增量重置基准）。tree 段：tree_dirty=true 用新
        // tree_ids，false 复用旧（同上 bar，不变）。cand 段每 bar 重建（O(candidate) 有界）。
        if tree_dirty {
            self.present_last_tree = tree_ids;
        }
        self.present_last_cand = cand_ids;
    }

    /// held 腿四态分类（§10）。
    pub fn held_state(&self, leg: &ActiveLeg) -> HeldLegState {
        match self.elements.get(&leg.id) {
            None => HeldLegState::Invalidated, // 不在 registry = 已被删除/作废
            Some(e) if e.invalidated => HeldLegState::Invalidated,
            Some(e) if e.snapshot_present => HeldLegState::LivePresent,
            Some(_) => HeldLegState::LiveDetached,
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  §12 测试矩阵（anc.pdf §12）
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta_v0::classifier::recursive_tower::ElementId;
    use crate::theta_v0::strategy::coverage::CoverageElement;
    use crate::theta_v0::strategy::interp::ActiveLeg;

    fn voice_side(up: bool) -> VoiceSide {
        if up {
            VoiceSide::Long
        } else {
            VoiceSide::Short
        }
    }

    fn cov_elem(level: u32, ordinal: u64, lambda: usize, rho: usize, dir: VoiceSide, parent_id: Option<ElementId>) -> CoverageElement {
        CoverageElement {
            lambda,
            rho,
            eps: dir,
            level,
            parent: None,
            attached_dir: None,
            id: ElementId { level, ordinal },
            parent_id,
        }
    }

    fn active_leg(level: u32, ordinal: u64, lambda: usize, rho: usize, dir: VoiceSide, parent_id: Option<ElementId>) -> ActiveLeg {
        ActiveLeg {
            level,
            dir,
            source_index: rho,
            lambda,
            id: ElementId { level, ordinal },
            parent_id,
            is_boundary_root: parent_id.is_none(),
            op_parent: parent_id,
        }
    }

    /// **§12 测试 1**：更高级别涌现测试——低级别元素 ID 不变，只新增高一级父容器。
    /// I3：parent 是关系非身份——pid 不依赖 parent。
    #[test]
    fn test_higher_level_emergence_pid_stable() {
        let mut reg = PersistentRegistry::new();
        // bar 0: 只有一个 L0 元素
        let e0 = cov_elem(0, 0, 0, 10, VoiceSide::Long, None);
        reg = reg.merge(&[e0], &[]);
        assert!(reg.registry_live(&ElementId { level: 0, ordinal: 0 }));
        assert!(reg.snapshot_present(&ElementId { level: 0, ordinal: 0 }));

        // bar 1: 涌现 L1 父容器，L0 元素 ID 不变（I3），只是 structural_parent_id 更新
        let l1 = cov_elem(1, 0, 0, 20, VoiceSide::Long, None);
        let e0_b1 = cov_elem(0, 0, 0, 10, VoiceSide::Long, Some(ElementId { level: 1, ordinal: 0 }));
        reg = reg.merge(&[l1, e0_b1], &[]);
        // L0 元素 ID 不变（I3），仍在 registry
        assert!(reg.registry_live(&ElementId { level: 0, ordinal: 0 }));
        // structural_parent_id 更新为 L1（关系可变，身份不变）
        assert_eq!(
            reg.get(&ElementId { level: 0, ordinal: 0 }).unwrap().structural_parent_id,
            Some(ElementId { level: 1, ordinal: 0 })
        );
    }

    /// **§12 测试 2**：reparent 测试——parent edge 可变，但 child pid 不变。
    #[test]
    fn test_reparent_pid_stable() {
        let mut reg = PersistentRegistry::new();
        let parent_a = ElementId { level: 1, ordinal: 0 };
        let parent_b = ElementId { level: 1, ordinal: 1 };
        let child = ElementId { level: 0, ordinal: 0 };

        // bar 0: child 的 parent 是 A
        let a = cov_elem(1, 0, 0, 10, VoiceSide::Long, None);
        let c0 = cov_elem(0, 0, 0, 5, VoiceSide::Short, Some(parent_a));
        reg = reg.merge(&[a, c0], &[]);
        assert_eq!(reg.get(&child).unwrap().structural_parent_id, Some(parent_a));

        // bar 1: child 的 parent 变成 B（reparent），child pid 不变
        let b = cov_elem(1, 1, 0, 10, VoiceSide::Long, None);
        let c1 = cov_elem(0, 0, 0, 5, VoiceSide::Short, Some(parent_b));
        reg = reg.merge(&[b, c1], &[]);
        assert_eq!(reg.get(&child).unwrap().structural_parent_id, Some(parent_b));
        // child pid 不变
        assert!(reg.registry_live(&child));
    }

    /// **§12 测试 3**：held detached 测试——snapshot 找不到 held 元素，registry 仍保留，腿不被判 stale。
    /// **★核心修复**：Q4 误把 LiveDetached 当 Stale，修复为保留。
    #[test]
    fn test_held_detached_not_stale() {
        let mut reg = PersistentRegistry::new();
        let leg_id = ElementId { level: 0, ordinal: 5 };
        let leg = active_leg(0, 5, 10, 20, VoiceSide::Long, Some(ElementId { level: 1, ordinal: 0 }));

        // bar 0: held 腿在 snapshot 中
        let e = cov_elem(0, 5, 10, 20, VoiceSide::Long, Some(ElementId { level: 1, ordinal: 0 }));
        reg = reg.merge(&[e], &[leg]);
        assert_eq!(reg.held_state(&leg), HeldLegState::LivePresent);

        // bar 1: held 腿不在 snapshot 中（结构演化），但 registry 保留（I1）
        reg = reg.merge(&[], &[leg]);
        // ★LiveDetached，不是 Stale/Invalidated
        assert_eq!(reg.held_state(&leg), HeldLegState::LiveDetached);
        // registry 仍 live
        assert!(reg.registry_live(&leg_id));
        // snapshot_present=false
        assert!(!reg.snapshot_present(&leg_id));
    }

    /// **§12 测试 4**：no fake root 测试——missing parent 不能伪造成 parent:None。
    /// LiveDetached 的 parent 仍是 op_parent(L)，不是 None（§15）。
    #[test]
    fn test_no_fake_root_detached_parent_preserved() {
        let mut reg = PersistentRegistry::new();
        let op_parent = ElementId { level: 1, ordinal: 0 };
        let leg = active_leg(0, 5, 10, 20, VoiceSide::Short, Some(op_parent));

        // bar 0: held 腿在 snapshot，parent=op_parent
        let e = cov_elem(0, 5, 10, 20, VoiceSide::Short, Some(op_parent));
        reg = reg.merge(&[e], &[leg]);

        // bar 1: held 腿不在 snapshot，但 parent 不被伪造为 None
        reg = reg.merge(&[], &[leg]);
        let state = reg.held_state(&leg);
        assert_eq!(state, HeldLegState::LiveDetached);
        // structural_parent_id 保留（不伪造 None）——★不伪造 root（§15）
        let pe = reg.get(&leg.id).unwrap();
        assert_eq!(pe.structural_parent_id, Some(op_parent));
    }

    /// **§12 测试 5**：op_parent AncOK 测试——held 子腿按入场操作父容器通过 AncOK。
    /// I4：op_parent(L)=c ⟹ c∈Pi 或 c⇝c'（确定性 successor）。
    #[test]
    fn test_op_parent_persistent_alive() {
        let mut reg = PersistentRegistry::new();
        let op_parent_id = ElementId { level: 1, ordinal: 0 };
        let child_leg = active_leg(0, 3, 10, 15, VoiceSide::Short, Some(op_parent_id));

        // bar 0: 父子都在 snapshot
        let parent_e = cov_elem(1, 0, 0, 20, VoiceSide::Long, None);
        let child_e = cov_elem(0, 3, 10, 15, VoiceSide::Short, Some(op_parent_id));
        reg = reg.merge(&[parent_e, child_e], &[child_leg]);

        // bar 1: 子腿不在 snapshot（LiveDetached），但 op_parent 仍在 registry（I4）
        reg = reg.merge(&[], &[child_leg]);
        assert!(reg.registry_live(&op_parent_id)); // I4: op_parent persistent
        assert!(reg.registry_live(&child_leg.id)); // I1: child persistent
    }

    /// **§12 测试 6**：explicit close 测试——只有 close/risk close/invalidation 才让 held leg 退出 live。
    #[test]
    fn test_explicit_close_exits_live() {
        let mut reg = PersistentRegistry::new();
        let leg_id = ElementId { level: 0, ordinal: 0 };
        let leg = active_leg(0, 0, 5, 10, VoiceSide::Long, None);

        // bar 0: 入场
        let e = cov_elem(0, 0, 5, 10, VoiceSide::Long, None);
        reg = reg.merge(&[e], &[leg]);
        assert!(reg.registry_live(&leg_id));

        // bar 1: 显式作废
        reg.invalidate(&leg_id);
        assert_eq!(reg.held_state(&leg), HeldLegState::Invalidated);
        assert!(!reg.registry_live(&leg_id)); // 退出 live

        // bar 2: 即使 snapshot 重新出现，invalidated 持久（§9 rule 5）
        reg = reg.merge(&[e], &[]);
        assert!(!reg.registry_live(&leg_id)); // 仍 invalidated
    }

    /// **I2 不变量**：元素方向不变——δi(e)=δj(e)。
    #[test]
    fn test_i2_direction_invariant() {
        let mut reg = PersistentRegistry::new();
        let pid = ElementId { level: 0, ordinal: 0 };
        let e0 = cov_elem(0, 0, 0, 10, VoiceSide::Long, None);
        reg = reg.merge(&[e0], &[]);
        assert_eq!(reg.get(&pid).unwrap().dir, VoiceSide::Long);

        // bar 1: snapshot 刷新，方向不变（I2）
        let e1 = cov_elem(0, 0, 0, 15, VoiceSide::Long, None);
        reg = reg.merge(&[e1], &[]);
        assert_eq!(reg.get(&pid).unwrap().dir, VoiceSide::Long);
        // rho 漂移（父延伸），方向不变
        assert_eq!(reg.get(&pid).unwrap().rho, 15);
    }

    /// **I1 不变量**：腿未关闭 ⟹ pid∈Pi 所有后续 bar。
    #[test]
    fn test_i1_persistence_until_close() {
        let mut reg = PersistentRegistry::new();
        let leg_id = ElementId { level: 0, ordinal: 0 };
        let leg = active_leg(0, 0, 5, 10, VoiceSide::Long, None);

        reg = reg.merge(&[cov_elem(0, 0, 5, 10, VoiceSide::Long, None)], &[leg]);
        // 连续 5 bar，腿都不在 snapshot 但 held
        for _ in 0..5 {
            reg = reg.merge(&[], &[leg]);
            assert!(reg.registry_live(&leg_id), "I1: 腿未关闭 ⟹ pid∈Pi");
        }
        // 显式关闭后退出
        reg.close(&leg_id);
        assert!(!reg.registry_live(&leg_id));
    }
}
