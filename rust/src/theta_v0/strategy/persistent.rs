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
//! - I2 元素方向不变：δi(e)=δj(e)——**#233 机器锁落地**：upsert 方向守卫拒绝同 id 静默
//!   覆写 dir（[`FlipGuardProbe`] 计数为凭；此前生产每 bar 无条件覆写 dir 实证违例，
//!   见 leg-2-98-lifecycle-20260724 §5）。翻向的终结事件化在 coverage 对位层兑现。
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

/// registry 条目的**首见来源**（#271）：该 pid **第一次**进 registry 时走的是哪条 merge 路径。
///
/// 为什么需要：条目的 `dir` 是**首见方向**（I2 机器锁永固，见 [`PersistentRegistry::merge_in_place_split`]
/// 的 #233 守卫），而三条登记路径写入的方向语义并不同轴——tree 段写结构方向 ε，candidate 段与
/// held 兜底写信号/持仓方向 σ。[`PersistentRegistry::direction_flip_event_active`] 的枚举注释
/// 「candidate 段冲突不算」若只看 `dir≠tree_eps`，在「候选/持仓先于树元素登记该 id」的构型下
/// 不可执行（首见 dir=σ ⟹ 与树 ε 恒分歧 ⟹ 理论伪阳性）。记录来源使该枚举在**所有构型**下可执行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstSeenSource {
    /// snapshot **tree 段** upsert 首见 ⟹ `dir` = 结构树方向 ε（唯一与 `tree_eps` 同轴的来源）。
    Tree,
    /// snapshot **candidate 段** upsert 首见 ⟹ `dir` = 候选点信号方向 σ（BSP 构造下 σ=−ε 恒真）。
    Candidate,
    /// **held leg / op_parent 的 `or_insert`** 兜底首见（LiveDetached，§9 rule 2）⟹ `dir` = 持仓腿方向 σ。
    Held,
}

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
    /// ★#271 首见来源：本条目**首次登记**走的路径（tree / candidate / held-or_insert）。
    /// 与 `dir`（首见方向永固）同期写入、此后**不再改写**（首见即定，与 I2 同寿命）。
    pub first_seen: FirstSeenSource,
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
        self.merge_in_place_split(snapshot, true, &[], true, held_legs);
    }

    /// **§9 merge 双段增量版（工位 4f：confirmed prefix 增量维护——消 step 1'/2' 的 O(tree)/bar）**。
    ///
    /// snapshot 拆为 `tree`（∝confirmed，TreeCache 缓存）+ `candidates`（每 bar 变，有界）。`tree_dirty=false`
    /// （TreeKey 命中 ⟹ tree 逐字节同上 bar）⟹ **跳过 tree 段 step 1'/2'**（这些元素上 bar 已写同值 + 全 present）。
    ///
    /// bit-exact == 全量（`tree_dirty=true` 恒走全量）：
    /// - step 2' 跳过：tree_dirty=false ⟹ tree 同上 bar ⟹ 上 bar upsert 值 == 本 bar 会写值（断言1）。candidate
    ///   段恒 upsert，且 carrier id 碰撞时 upsert 覆盖**除 dir 外**全部 tree-derived 字段（断言2；
    ///   dir 入 #233 I2 守卫不参与覆盖）⟹ 末态 = candidate 值（dir 永固首见方向）。
    /// - step 1' 跳过 tree：tree_dirty=false ⟹ present_last_tree 中每 id 仍在本 bar tree ⟹ 全 present ⟹ 不该置
    ///   false（断言3）。candidate 段恒查：present_last_cand 中本 bar 在 `cand_ids ∪ tree_ids` 找不到的才置 false
    ///   （carrier id 上 bar 在 cand 本 bar 退回 tree 仍 present，不误置）。
    pub fn merge_in_place_split(
        &mut self,
        tree: &[CoverageElement],
        tree_dirty: bool,
        candidates: &[CoverageElement],
        cand_dirty: bool,
        held_legs: &[ActiveLeg],
    ) {
        // ★on2w3 神谕（debug）：跳过路径（tree_dirty=false ∨ cand_dirty=false）与「强制全量」逐字段等价。
        // 全量 = 恒 tree_dirty=true ∧ cand_dirty=true。skip 后与全量末态（elements + present_last）比对。
        #[cfg(debug_assertions)]
        let oracle_full = if !tree_dirty || !cand_dirty {
            let mut shadow = self.clone();
            shadow.merge_full_oracle(tree, candidates, held_legs);
            Some(shadow)
        } else {
            None
        };
        self.merge_in_place_split_core(tree, tree_dirty, candidates, cand_dirty, held_legs);
        #[cfg(debug_assertions)]
        if let Some(full) = oracle_full {
            debug_assert!(
                self.elements == full.elements
                    && self.present_last_tree == full.present_last_tree
                    && self.present_last_cand == full.present_last_cand,
                "on2w3 merge 跳过路径 != 全量（tree_dirty={tree_dirty} cand_dirty={cand_dirty}）——\
                 candidates 逐字节稳定/id集恒等前提被违反"
            );
        }
    }

    /// 强制全量 merge（神谕基准，debug 构建）：tree_dirty=true ∧ cand_dirty=true。
    #[cfg(debug_assertions)]
    fn merge_full_oracle(
        &mut self,
        tree: &[CoverageElement],
        candidates: &[CoverageElement],
        held_legs: &[ActiveLeg],
    ) {
        self.merge_in_place_split_core(tree, true, candidates, true, held_legs);
    }

    fn merge_in_place_split_core(
        &mut self,
        tree: &[CoverageElement],
        tree_dirty: bool,
        candidates: &[CoverageElement],
        cand_dirty: bool,
        held_legs: &[ActiveLeg],
    ) {
        let tree_ids: std::collections::HashSet<ElementId> = if tree_dirty {
            tree.iter().map(|e| e.id).collect()
        } else {
            // tree_dirty=false ⟹ tree ids 同上 bar present_last_tree（不重建，消 O(tree)）。
            self.present_last_tree.clone()
        };
        // ★on2w3：cand_dirty=false ⟹ candidates 逐字节同上 bar（CandidateCache whole-cache 命中 + 零新尾）
        // ⟹ cand_ids == present_last_cand（集合不变）。此时整段 candidate 工作（cand_ids 建 + reset + upsert
        // + present_last_cand 重建，全 O(cand)/bar，cand∝n ⟹ O(n²)）是 no-op：同 id 全 present、同值上 bar 已
        // upsert（invalidate 不被 upsert 重置 ⟹ skip 与全量末态一致）、present_last_cand 不变。跳过 ⟹ O(1)。
        let cand_ids: std::collections::HashSet<ElementId> = if cand_dirty {
            candidates.iter().map(|e| e.id).collect()
        } else {
            // 不重建（消 O(cand)）；下方 present_last_cand 也不重写（集合恒等）。
            std::collections::HashSet::new()
        };

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
        //   ★on2w3：cand_dirty=false ⟹ present_last_cand 全在 cand_ids（集合恒等）⟹ 循环恒 no-op ⟹ 跳过。
        if cand_dirty {
            for pid in &self.present_last_cand {
                if !cand_ids.contains(pid) && !tree_ids.contains(pid) {
                    if let Some(e) = self.elements.get_mut(pid) {
                        e.snapshot_present = false;
                    }
                }
            }
        }

        // 2'. upsert：snapshot 中所有元素刷新 snapshot_present=true + structural_parent_id/rho（§9 rule 1）。
        //     ★工位 4f：tree_dirty=false ⟹ 跳过 tree 段（上 bar 已写同值，断言1）；candidate 段恒 upsert。
        //     ★#233 I2 机器锁（票面要求 1，anc.pdf §7 I2「同一持久元素方向不变」）：已存在条目
        //     方向冲突 ⟹ **拒绝静默覆写 dir**（dir 永固首见方向），其余 snapshot 字段照刷
        //     （§9 rule 1 坐标/关系可变、I3）；冲突显式计数（[`FlipGuardProbe`]，机器锁审查证据）。
        //     tree/candidate 段同一闭包 ⟹ χ 域外候选 flip-flop 同路径守卫（票面第 4 条）。
        //     翻向的**终结事件化**（父关闭 + 子树连清 + 新世代登记，#227 裁决蓝图两步形）在
        //     coverage 对位层（`held_leg_tree_index_indexed` 方向守卫）兑现——本层是持久防线。
        let upsert = |elements: &mut std::collections::HashMap<ElementId, PersistentElement>, e: &CoverageElement, cand_segment: bool| {
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
                // ★#271 首见来源（首见即定，此后不改写——下方只刷 snapshot 字段）。
                first_seen: if cand_segment {
                    FirstSeenSource::Candidate
                } else {
                    FirstSeenSource::Tree
                },
            });
            // ★#233 I2 守卫：同 id 方向不一致 ⟹ 不覆写 dir（禁止静默覆写；拒绝形态，非异常——
            // 翻向是合法事件，事件化在 coverage 层），冲突显式计数（分 tree/cand 段）。
            if entry.dir != e.eps {
                flip_guard_probe_bump(|p| {
                    if cand_segment {
                        p.cand_blocked += 1;
                    } else {
                        p.tree_blocked += 1;
                    }
                });
            } else {
                entry.dir = e.eps;
            }
            entry.level = e.level;
            entry.lambda = e.lambda;
            entry.rho = e.rho;
            entry.structural_parent_id = e.parent_id;
            entry.snapshot_present = true;
            // 不重置 invalidated（显式作废持久，§9 rule 5）。
        };
        if tree_dirty {
            for e in tree {
                upsert(&mut self.elements, e, false);
            }
        }
        // candidate 段 upsert（断言2 收窄：碰撞 carrier id 覆盖**除 dir 外**全部 tree-derived
        // 字段——dir 入 I2 守卫（#233），仍须在 tree 之后保覆盖序与全量一致）。
        //   ★on2w3：跳过仅当 **tree 与 cand 都不脏**——tree_dirty=true 时 tree 段可能 upsert 了与 candidate
        //   碰撞的 carrier id（tree 值），须重跑 candidate upsert 恢复「candidate 胜」覆盖序（断言2）。
        //   cand_dirty=false 且 tree_dirty=false ⟹ 无碰撞覆盖 + 同值上 bar 已 upsert ⟹ 跳过。
        if cand_dirty || tree_dirty {
            for e in candidates {
                upsert(&mut self.elements, e, true);
            }
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
                first_seen: FirstSeenSource::Held, // ★#271：持仓腿兜底登记 ⟹ dir = 腿方向 σ
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
                    first_seen: FirstSeenSource::Held, // ★#271：op_parent 兜底（dir 借腿方向 σ）
                });
            }
        }

        // present_last 更新为本 bar snapshot ids（下 bar 增量重置基准）。tree 段：tree_dirty=true 用新
        // tree_ids，false 复用旧（同上 bar，不变）。cand 段：cand_dirty=true 用新 cand_ids；false 复用旧
        // （集合恒等，cand_ids 为空占位不可写入——on2w3）。
        if tree_dirty {
            self.present_last_tree = tree_ids;
        }
        if cand_dirty {
            self.present_last_cand = cand_ids;
        }
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

    /// ★#269 载体翻向**结构事件**显式谓词（翻向守卫事件化，#261 终裁选项 A）：
    /// `pid` 的 registry 首见方向（I2 机器锁永固）与**当前树元素方向** `tree_eps` 不一致
    /// ⟺ 树段 upsert 方向冲突（[`FlipGuardProbe::tree_blocked`] 计数的那类事件）在腿存活
    /// 期间发生——即「载体被 frontier 重组改判为相反方向」的真实结构翻向。
    ///
    /// **事件类型枚举**（塔事件流中什么算「该载体的翻向结构事件」）：
    /// - **算数（唯一一类）：载体元素方向翻转** = 同 `ElementId` 树元素的方向被结构树改判
    ///   （frontier 重组，leg-2-98 勘察 (3,22) PREG Short→Long 单行形态：无候选参与、无关闭
    ///   事件、树 upsert 单行）。I2 下同一持久元素方向不变 ⟹ 首见方向 ≠ 当前树方向当且仅当
    ///   该冲突发生（或正持续）；翻回首见方向则分歧消失、事件不再激活（诚实口径）。
    /// - **不算：candidate 段 upsert 冲突**（[`FlipGuardProbe::cand_blocked`]）——候选点元素
    ///   eps=σ（信号方向）与载体 ε（结构方向）的对立是 BSP 构造下**出生即存在的两轴分层**
    ///   （买点恒附下降段末端 ⟹ σ=−ε 恒真，#264 §2.1），含 χ 域外候选 flip-flop；是状态
    ///   分层不是事件。
    ///
    /// ★#271 **首见来源门（该枚举在所有构型下可执行）**：条目 `dir` 是首见方向，而首见可能来自
    /// candidate 段或 held 兜底（[`FirstSeenSource`]），此时 `dir=σ` 与 `tree_eps=ε` **恒分歧**——
    /// 不是翻向事件，而是两轴出生分层被记进了同一字段。故谓词**只对 `first_seen==Tree` 的分歧
    /// 开火**（唯一与 `tree_eps` 同轴的来源，即 `tree_blocked` 计的那类事件）；非 tree 来源的分歧
    /// **如实计数后忽略**（[`FlipGuardProbe::nontree_origin_divergence`]，不判 Flipped）。
    ///
    /// 口径边界（诚实标注）：非 tree 首见条目此后**永远**不参与翻向事件判定——其 I2 首见方向
    /// 与结构轴不可比，本层无从区分「σ/ε 出生分层」与「该载体真翻向」。当前主路径（tree 先于
    /// candidate/held 登记载体 id）下该分支不可达；χ 域外候选可达性既不能证实也不能证伪（#269），
    /// 探针计数 >0 即为其在场证据。宁可漏判（不伪造杀）不可误杀——与 `None ⟹ false` 同一取向。
    /// - **不存在：父元素 destroy+反向重建**——前缀因果塔单调增长，无 destroy 概念
    ///   （opsem_dump.rs:167 诚实缺席）；「翻向」在本系统唯一可观察形态即树段方向冲突。
    /// - **不算：载体退出树（Stale）**——归 Stale 四态既有分派（§10 persistent overlay 域），
    ///   不是翻向事件。
    ///
    /// 用事件不用状态轴（#264 未能判定②：ε 轴有 id 重指/Stale 坑——26 笔幸存笔三态未分）：
    /// 本谓词**不读腿方向 σ**，只读「该载体自身是否发生了方向改判事件」。`pid` 无 registry
    /// 条目（树元素尚未经 merge 首见登记）⟹ 无事件证据 ⟹ false（诚实缺席，不伪造杀）。
    pub fn direction_flip_event_active(&self, pid: &ElementId, tree_eps: VoiceSide) -> bool {
        match self.elements.get(pid) {
            // 首见非 tree 段 ⟹ dir 与 tree_eps 不同轴（σ vs ε）⟹ 分歧不是事件：如实计数后忽略。
            Some(e) if e.dir != tree_eps && e.first_seen != FirstSeenSource::Tree => {
                flip_guard_probe_bump(|p| p.nontree_origin_divergence += 1);
                false
            }
            Some(e) => e.dir != tree_eps,
            None => false,
        }
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  ★#233 I2 方向守卫探针（机器锁可观测面，断言①②③同款 thread_local 恒在计数模式）
// ════════════════════════════════════════════════════════════════════════════

/// I2 方向守卫运行时计数：同 id 方向冲突被守卫**拒绝静默覆写**的笔数（分 tree/candidate 段）。
/// 计数 >0 ⟺ 翻向事件发生——翻向的事件化（父声部关闭 + 子树连带清仓 + 新世代重登记，#227 裁决
/// 蓝图两步形）在 coverage 对位层（`held_leg_tree_index_indexed` 方向守卫/Flipped 分派）兑现；
/// 本计数是持久层「方向不可变机器锁」的审查证据（release 下可观测，与断言①②③探针同款）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlipGuardProbe {
    /// tree 段 upsert 方向冲突守卫笔数（树元素/frontier 重组翻向）。
    pub tree_blocked: u64,
    /// candidate 段 upsert 方向冲突守卫笔数（χ 域外候选 flip-flop——与树元素 dir 覆写
    /// 同一路径（同一 upsert 闭包），票面第 4 条：一并按守卫处理）。
    pub cand_blocked: u64,
    /// ★#271：[`PersistentRegistry::direction_flip_event_active`] 遇到「首见来源非 tree 段
    /// （candidate / held 兜底）的条目 dir ≠ tree_eps」的次数——**不判翻向事件**（σ/ε 两轴
    /// 不同轴，非结构改判），只如实计数。>0 ⟹ 首见来源门在生产中真被走到（理论伪阳性边
    /// 的可达性证据）；当前主路径预期恒 0（tree 先于 candidate/held 登记载体 id）。
    pub nontree_origin_divergence: u64,
}

thread_local! {
    static FLIP_GUARD_PROBE: std::cell::Cell<FlipGuardProbe> =
        const {
            std::cell::Cell::new(FlipGuardProbe {
                tree_blocked: 0,
                cand_blocked: 0,
                nontree_origin_divergence: 0,
            })
        };
}

#[inline]
fn flip_guard_probe_bump(f: impl FnOnce(&mut FlipGuardProbe)) {
    FLIP_GUARD_PROBE.with(|c| {
        let mut p = c.get();
        f(&mut p);
        c.set(p);
    });
}

/// 归零 I2 方向守卫探针（L2 run/见证测试前调用）。
pub fn flip_guard_probe_reset() {
    FLIP_GUARD_PROBE.with(|c| c.set(FlipGuardProbe::default()));
}

/// 读取 I2 方向守卫探针快照（L2 run/见证测试后调用）。
pub fn flip_guard_probe_snapshot() -> FlipGuardProbe {
    FLIP_GUARD_PROBE.with(std::cell::Cell::get)
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

    /// ★#233 **I2 机器锁**（票面要求 1，#227 裁决 anc.pdf §7 I2「同一持久元素方向不变」）：
    /// 同 id 方向覆写输入 ⟹ upsert 守卫**拒绝静默覆写**——dir 保持首见方向，其余 snapshot
    /// 字段（rho/level/lambda/structural_parent_id）照刷（§9 rule 1：坐标/关系可变、I3），
    /// tree 段方向冲突显式计数（事件化原料；翻向的终结事件化在 coverage 对位层兑现）。
    ///
    /// **RED（守卫前）**：upsert 每 bar 无条件 `entry.dir = e.eps`（旧 :330）⟹ dir 被静默
    /// 翻成 Short（leg-2-98 勘察 §5 实证 I2 违例：(3,22) 两次翻转、(2,98) 每代翻转）。
    /// **GREEN（守卫后）**：dir=Long 永固，rho 照刷 15，tree 段 flip 探针=1。
    #[test]
    fn i2_direction_guard_blocks_silent_dir_overwrite() {
        flip_guard_probe_reset();
        let mut reg = PersistentRegistry::new();
        let pid = ElementId { level: 0, ordinal: 0 };
        let e0 = cov_elem(0, 0, 0, 10, VoiceSide::Long, None);
        reg = reg.merge(&[e0], &[]);
        assert_eq!(reg.get(&pid).unwrap().dir, VoiceSide::Long);

        // bar 1：同 id 方向覆写输入（frontier 重组翻向）——守卫：dir 不变，坐标照刷。
        let e1 = cov_elem(0, 0, 0, 15, VoiceSide::Short, None);
        reg = reg.merge(&[e1], &[]);
        let pe = reg.get(&pid).unwrap();
        assert_eq!(
            pe.dir,
            VoiceSide::Long,
            "I2：同一持久元素方向不变——守卫拒绝静默覆写（#233 机器锁）"
        );
        assert_eq!(pe.rho, 15, "rho 照刷（§9 rule 1 snapshot 坐标刷新不收守卫影响）");
        assert!(pe.snapshot_present, "snapshot_present 照刷（§9 rule 1）");
        assert_eq!(
            flip_guard_probe_snapshot().tree_blocked,
            1,
            "tree 段方向冲突显式计数（I2 机器锁审查证据）"
        );
        assert_eq!(flip_guard_probe_snapshot().cand_blocked, 0);
    }

    /// ★#233 **I2 机器锁（candidate 段同路径，票面第 4 条）**：χ 域外候选对 registry dir 的
    /// 覆盖与树元素 dir 覆写**同一路径**（同一 upsert 闭包，candidate 胜序）⟹ 一并按守卫
    /// 处理：候选方向与持久条目冲突 ⟹ 拒绝覆写 dir（其余字段照刷，candidate 胜序收窄为
    /// 「除 dir 外全字段」），cand 段探针计数。
    ///
    /// 现场（leg-2-98 勘察 §2.3）：χ 未过候选元素（dir=Long）每 bar 把 registry[(2,98)].dir
    /// 覆盖成 Long（物理 Short）——registry dir 与物理长期背离（I2 生产实证违例，flip-flop
    /// 无关闭效力副产）。
    ///
    /// **RED（守卫前）**：candidate 胜序覆盖全字段 ⟹ dir=Long（背离物理）。**GREEN（守卫
    /// 后）**：dir=Short 永固，rho=30 照刷（胜序收窄：仅 dir 入守卫），cand 探针=1。
    #[test]
    fn i2_direction_guard_candidate_segment_same_guard() {
        flip_guard_probe_reset();
        let mut reg = PersistentRegistry::new();
        let pid = ElementId { level: 2, ordinal: 98 };
        let tree_elem = cov_elem(2, 98, 10, 20, VoiceSide::Short, None);
        // 双段 merge：tree 段首见 Short 登记；candidate 段同 id 同向 upsert（无冲突基线）。
        reg.merge_in_place_split(&[tree_elem], true, &[tree_elem], true, &[]);
        assert_eq!(reg.get(&pid).unwrap().dir, VoiceSide::Short);
        // 下一 bar：candidate 段同 id **Long** 冲突（χ 域外 flip-flop 形态）⟹ 守卫。
        let cand_long = cov_elem(2, 98, 30, 30, VoiceSide::Long, None);
        reg.merge_in_place_split(&[tree_elem], true, &[cand_long], true, &[]);
        let pe = reg.get(&pid).unwrap();
        assert_eq!(
            pe.dir,
            VoiceSide::Short,
            "I2：candidate 段方向冲突同守卫——registry dir 不被 χ 域外候选覆盖（票面第 4 条）"
        );
        assert_eq!(
            pe.rho, 30,
            "candidate 胜序的坐标刷新保留（断言2 收窄：仅 dir 字段入守卫，其余字段仍 candidate 胜）"
        );
        assert_eq!(
            flip_guard_probe_snapshot().cand_blocked,
            1,
            "cand 段方向冲突显式计数（I2 机器锁审查证据）"
        );
        assert_eq!(flip_guard_probe_snapshot().tree_blocked, 0);
    }

    /// ★#271 **首见来源标记**：三条登记路径各自留下 [`FirstSeenSource`]（tree / candidate /
    /// held-or_insert），使翻向事件枚举的「candidate 段冲突不算」在所有构型下可执行。
    #[test]
    fn first_seen_source_recorded_per_registration_path() {
        let mut reg = PersistentRegistry::new();
        let tree_pid = ElementId { level: 1, ordinal: 1 };
        let cand_pid = ElementId { level: 0, ordinal: 2 };
        let held_pid = ElementId { level: 0, ordinal: 3 };
        let op_pid = ElementId { level: 1, ordinal: 9 };
        let leg = active_leg(0, 3, 0, 5, VoiceSide::Long, Some(op_pid));
        reg.merge_in_place_split(
            &[cov_elem(1, 1, 0, 10, VoiceSide::Long, None)],
            true,
            &[cov_elem(0, 2, 0, 4, VoiceSide::Short, None)],
            true,
            &[leg],
        );
        assert_eq!(reg.get(&tree_pid).unwrap().first_seen, FirstSeenSource::Tree);
        assert_eq!(reg.get(&cand_pid).unwrap().first_seen, FirstSeenSource::Candidate);
        assert_eq!(reg.get(&held_pid).unwrap().first_seen, FirstSeenSource::Held);
        assert_eq!(
            reg.get(&op_pid).unwrap().first_seen,
            FirstSeenSource::Held,
            "op_parent 兜底同属 held 路径（I4）"
        );
    }

    /// ★#271 **首见来源门**：翻向事件谓词只对 tree 来源的分歧开火；candidate / held 首见的
    /// 分歧是 σ/ε 两轴出生分层（非结构改判）⟹ 不判事件，只计 `nontree_origin_divergence`。
    ///
    /// **RED（#271 前）**：谓词只看 `dir != tree_eps` ⟹ 候选/持仓先登记的构型下恒 true（理论
    /// 伪阳性 ⟹ 出生对立腿被误判 Flipped）。
    #[test]
    fn flip_event_predicate_gated_by_first_seen_source() {
        flip_guard_probe_reset();
        let mut reg = PersistentRegistry::new();
        // ① candidate 首见（σ=Long），树元素同 id 为 ε=Short：分歧但非事件。
        let cand_pid = ElementId { level: 0, ordinal: 7 };
        reg.merge_in_place_split(&[], true, &[cov_elem(0, 7, 0, 4, VoiceSide::Long, None)], true, &[]);
        assert!(
            !reg.direction_flip_event_active(&cand_pid, VoiceSide::Short),
            "candidate 首见的方向分歧 = 两轴出生分层，不是翻向事件（枚举『不算』条可执行）"
        );
        // ② held 兜底首见（σ=Long），同 id 树元素 ε=Short：同样不是事件。
        let held_pid = ElementId { level: 0, ordinal: 8 };
        reg.merge_in_place_split(&[], true, &[], true, &[active_leg(0, 8, 0, 5, VoiceSide::Long, None)]);
        assert!(!reg.direction_flip_event_active(&held_pid, VoiceSide::Short));
        assert_eq!(
            flip_guard_probe_snapshot().nontree_origin_divergence,
            2,
            "非 tree 来源分歧如实计数（忽略但不静默）"
        );
        // ③ tree 首见（ε=Long）+ 树改判 Short ⟹ 真翻向事件，照常开火。
        let tree_pid = ElementId { level: 0, ordinal: 9 };
        reg.merge_in_place_split(&[cov_elem(0, 9, 0, 10, VoiceSide::Long, None)], true, &[], true, &[]);
        assert!(
            reg.direction_flip_event_active(&tree_pid, VoiceSide::Short),
            "tree 来源分歧 = 载体被 frontier 重组改判 ⟹ 翻向事件（#269 口径不倒退）"
        );
        assert!(
            !reg.direction_flip_event_active(&tree_pid, VoiceSide::Long),
            "同向 ⟹ 无事件"
        );
        assert_eq!(
            flip_guard_probe_snapshot().nontree_origin_divergence,
            2,
            "tree 来源的开火/无事件均不进非 tree 计数"
        );
    }
}
