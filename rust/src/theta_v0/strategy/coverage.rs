//! 互斥全定义策略 element-coverage 执行引擎——port **M29 互斥全定义策略最终定理**
//! （`Origin/MutexFinalTheorem.lean` + `SeparateStrategyTarget.lean`(M16/M17) +
//! `AncestorClosure.lean`(M16 AncOK) + `OperationRole.lean`(M09 Role 四分)）。
//!
//! ## 工位定位（生产引擎核心，与买卖点 v1 正交的新路径）
//!
//! 被全窗 L3 8/8 否证的是**买卖点 v1**（`strategy/mod.rs::recognize` 每 bsp 一 decision，
//! 离散择时）。**本文件不是它**——本文件实装 Lean 已全量真封的**全定义策略 = element-coverage**：
//! 在**每个语法元素** λ_e 入场、ρ_e 平腿，**覆盖每个笔/线段/走势**（多级嵌套赋格），而非只在
//! 离散买卖点动作。这是 Lean M29 三结论合一的「互斥全定义策略 π_Θ」在 rust 执行层的兑现
//! （rust 执行层此前零实装——`LegTarget` / 活动集 `A_t/B_t/D_t` 只在 Lean，本文件首次港下来）。
//!
//! ## Lean → rust 语义映射（每 bar t 的五步）
//!
//! | 步 | Lean 规格 | rust 实装 |
//! |----|----------|----------|
//! | 1 元素集 E | `SyntaxElement`(§二 C27 四元组 `(I_e,ε_e,ℓ_e,par)`) | [`CoverageElement`]（从 classifier levels + `RMove::Compose` 塔提取）|
//! | 2 活动集 | M16 `A_t`/`B_t`/`D_t` + `AncOK[(A_t∖D_t)∪B_t]` | [`active_set_step`]（先关后开 + 祖先闭合）|
//! | 3 角色 R(g) | spec §8 `R(g)=(H(g),V(g),δ_g)` 18 类（3×3×2） | [`operation_role`]（H 水平 × V 垂直 × δ 方向三轴）|
//! | 4 LegTarget | M17/M28 每活动元素一腿（方向 ε_e，单位 s_e） | [`leg_target`]（role/depth 权重 `w_depth`）|
//! | 5 净额执行 | Nautilus 净额兼容（毛账本 → 净持仓） | [`net_target_units`]（所有腿合并为净 `units:f64`）|
//!
//! ## ★真 Fugue 严防级别差伪造（铁律，mod.rs:433-440 codex 已裁旧 bug）
//!
//! `recognize`（v1）旧 bug：用 `depth = l_star - level_idx` 把**级别差**伪造成赋格嵌套深度
//! （codex 异质裁决 + L2 OKLO bisect 坐实 2026-06-27）。本文件**严禁**重蹈：[`CoverageElement`]
//! 的 `depth` / `parent` 只来自 `RMove::Compose.subs` 的**真父子关系**（descend 取回的真嵌套
//! 子走势），不把「元素在第几级」当作 depth。短差（ShortDiff）的反向子声部腿 depth≥1 必须有
//! 真 Compose 父（[`extract_elements`] 的 `parent` 字段来自塔的真嵌套结构）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - **L1**（本文件）：element-coverage 引擎消费 classifier 塔产元素集 + 活动集递归 + Role 派生 +
//!   LegTarget 净额合并 = bit-exact 对齐 Lean M29 规格的**管线正确性**（验证管线，零信息增量）。
//! - **NOT L2 alpha**（gatekeeper，no-声明膨胀）：element-coverage 有无净账户 alpha 是 L2/L3
//!   经验问题，本文件**不预判有 alpha**。Lean M29 顶点诚实声明（`MutexFinalTheorem.lean` §7）：
//!   M29 是 goal 的**形式化**顶点（L0 语法层完备性：角色互斥分类 + 全元素覆盖 + 策略全定义 + ∃!），
//!   与全窗 L3（v1 实盘择时 8/8 否证）**不矛盾**——「分类完备性 ≠ 择时盈利」。本引擎是 M29 的
//!   rust 兑现，同样**不蕴含**实盘 alpha；是否盈利由下一步 L2/L3 净额回测否证检验（Lead 派）。
//!
//! ## owner 边界（互斥铁律）
//!
//! 本文件**新建**，自登记 `pub mod coverage;` 于 `strategy/mod.rs`。**不改**买卖点 v1
//! `recognize`（保留作基线对照）。runner 若需新入口，新增函数不改现有 `run_theta_v0`。
//!
//! immutable 风格：所有构造新对象，不原地修改（活动集递归返回新集合，不 mutate 旧集合）。

use super::super::classifier::descend::RMove;
use super::super::classifier::recursive_tower::{compose_level, compose_level_resume, ElementId, LeveledMove};
use std::rc::Rc;
use super::super::classifier::Classification;
use super::super::config::{RiskConfig, VoiceConfig};
use super::super::types::{Direction, Order, StrictAction};
use super::intent::{lex_argmin, JThetaKey, LexCandidate};
use super::interp::{self, ActiveLeg, Buckets, Candidate};
use super::voice::{depth_weight, VoiceSide};

// ════════════════════════════════════════════════════════════════════════════
//  §1 语法元素 e（M16/M17 `SyntaxElement` 的 rust 镜像，从 classifier 塔提取）
// ════════════════════════════════════════════════════════════════════════════

/// 缠论语法元素 e（M16/M17 `SyntaxElement` 四元组 `(I_e, ε_e, ℓ_e, par)` 的 rust 镜像）。
///
/// 逐分量对照（`SeparateStrategyTarget.lean` §B / `MutexElement`）：
/// - `lambda : usize`（`λ_e` = `Start(e)`，操作区间左端点 source_index，M16 `B_t={e:λ_e=t}` 读此）。
/// - `rho : usize`（`ρ_e` = `End(e)`，操作区间右端点 source_index，M16 `D_t={e:ρ_e=t}` 读此）。
/// - `eps : VoiceSide`（`ε_e∈{+1,-1}` = 元素绝对方向 = δ_g，M17 `1[ε_e=±1]` 读此；Up↦Long、Down↦Short）。
/// - `level : u32`（`ℓ_e` = 元素级别，0=L0 线段；水平关系 H 的**同级别兄弟**判定用，spec §7.1）。
/// - `parent : Option<usize>`（`par(e)` = p(g) 父容器在元素集中的索引；顶层 `None` = 父为边界胚元 ∂，spec §4 去根化）。
/// - `attached_dir : Option<VoiceSide>`（σ_{p(g)} = 父容器方向；None / Flat = 胚元无方向 σ=0；垂直关系 V 用，spec §7.2）。
///
/// ★`parent`/`attached_dir` 来自塔的**真嵌套结构**（Compose 父子），非级别差伪造（铁律）。
/// ★去根化（spec §4/§7）：`parent=None` **不是**"无父根"，而是父容器为边界胚元 ∂（σ_{p(g)}=0），
/// 在角色分类中由 V=Ambient 吸收——**无 RootRole 特例**（spec P7：Ambient 非根规则）。
///
/// ★codex Q4 确定性 ID（spec §13 `p:C_ℓ→C_{ℓ+1}` 结构映射对象身份）：
/// - `id`：跨 bar 稳定的确定性 ElementId（来自 `LeveledMove.id`，全量/增量产同 ID）。
/// - `parent_id`：父容器的 ElementId（跨 bar 稳定，真 Compose 父）。`None` = 真边界胚元 ∂。
/// `parent: Option<usize>` 保留作 per-bar Vec 索引（`ancestor_close` 等仍用索引查 elements Vec），
/// 但 AncOK 的**结构判据**改用 `parent_id`（spec §13 结构映射，非 per-bar 索引）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageElement {
    /// λ_e：操作区间左端点（source_index，入场 bar）。
    pub lambda: usize,
    /// ρ_e：操作区间右端点（source_index，平腿 bar）。
    pub rho: usize,
    /// ε_e = δ_g：元素绝对方向（Up=Long / Down=Short）。
    pub eps: VoiceSide,
    /// ℓ_e：元素级别（0=L0 线段，逐级递增；H 同级别兄弟判定用）。
    pub level: u32,
    /// p(g)：父容器在元素集中的索引（顶层 None = 边界胚元 ∂；来自 Compose 真父子）。
    pub parent: Option<usize>,
    /// σ_{p(g)}：父容器方向（顶层 None / Flat = 胚元 σ=0；V 垂直关系用）。
    pub attached_dir: Option<VoiceSide>,
    /// ★codex Q4：确定性元素身份（跨 bar 稳定，来自 `LeveledMove.id`）。
    pub id: ElementId,
    /// ★codex Q4：父容器的 ElementId（跨 bar 稳定，spec §13 结构映射）。None = 真边界胚元 ∂。
    pub parent_id: Option<ElementId>,
}

/// ★热点② O(n²) 消除（element-view-refactor-hotspot1-verdict 残留）：元素 Vec **借用 base 前缀
/// + overlay 追加**，消 [`coverage_step_from_buckets`] per-bar `elements.to_vec()`（O(tree)×n=O(n²)）。
///
/// `base`：当前 bar 的真嵌套树元素（[`interp::coverage_elements_with_tower`] 产，**借用零拷贝**）。
/// `overlay`：仅 Stale/restore 路径（[`restore_ancestor_chain_from_registry`]）追加的 registry 恢复
/// 祖先——**绝大多数 bar 为空**（L2 诊断 sd_parent_held=0，restore 罕触发），故零拷贝路径覆盖热循环。
///
/// 索引语义（与旧 `work: Vec` 逐字节一致）：`0..base.len()` 查 `base`，`base.len()..` 查 `overlay`。
/// 追加只在尾部（`push` 返新 idx），不重排不跳号 ⟹ ElementId 确定性层不变（bit-exact 守卫）。
pub(crate) struct ElementView<'a> {
    base: &'a [CoverageElement],
    overlay: Vec<CoverageElement>,
    /// ★工位 4d 热点①②：base（tree 前缀）的两个派生索引（`(parent,level)→idx` 兄弟表 + `ElementId→idx`
    /// 表），由调用方从 [`super::interp::TreeCache`] 注入（命中返 `Rc::clone` O(1)）。`None` ⟹ 消费者
    /// fallback 现建（[`coverage_step_from_buckets`] 测试路径/无缓存）。§16 tree 前缀不变 ⟹ 索引随
    /// tree 缓存复用，消除每 bar `build_prev_sibling_index`/`build_tree_id_index` O(tree)/bar=O(n²)。
    base_sibling_idx: Option<Rc<std::collections::HashMap<(Option<usize>, u32), Vec<usize>>>>,
    base_id_idx: Option<Rc<std::collections::HashMap<ElementId, usize>>>,
}

impl<'a> ElementView<'a> {
    fn new(base: &'a [CoverageElement]) -> Self {
        ElementView { base, overlay: Vec::new(), base_sibling_idx: None, base_id_idx: None }
    }

    /// 双段构造（base=持久树前缀借用零拷贝 + overlay=本 bar candidate 段 owned）。
    /// `_cached` 返回 `(tree_rc, candidates)` 后由消费者组装——消除旧 `tree.clone()` O(tree)/bar。
    pub(crate) fn from_parts(base: &'a [CoverageElement], overlay: Vec<CoverageElement>) -> Self {
        ElementView { base, overlay, base_sibling_idx: None, base_id_idx: None }
    }

    /// ★工位 4d：注入 base 段缓存索引（runner 从 [`super::interp::TreeCache`] 取，bit-exact 与现建相等）。
    /// 链式 builder：`ElementView::from_parts(..).with_base_indices(sib, id)`。
    pub(crate) fn with_base_indices(
        mut self,
        sibling_idx: Rc<std::collections::HashMap<(Option<usize>, u32), Vec<usize>>>,
        id_idx: Rc<std::collections::HashMap<ElementId, usize>>,
    ) -> Self {
        self.base_sibling_idx = Some(sibling_idx);
        self.base_id_idx = Some(id_idx);
        self
    }

    /// 元素总数（base 前缀 + overlay 追加），= 旧 `work.len()`。
    fn len(&self) -> usize {
        self.base.len() + self.overlay.len()
    }

    /// base 段长度 = candidate_start（candidate 段从此起，在 overlay）。
    fn base_len(&self) -> usize {
        self.base.len()
    }

    /// 取回 overlay（`_cached` 遍历2 算完 role 后取回 candidates Vec；view drop）。
    pub(crate) fn into_overlay(self) -> Vec<CoverageElement> {
        self.overlay
    }

    /// 按全局 idx 取元素（`< base.len()` 查 base，否则查 overlay），= 旧 `work.get(i)`。
    fn get(&self, idx: usize) -> Option<&CoverageElement> {
        if idx < self.base.len() {
            self.base.get(idx)
        } else {
            self.overlay.get(idx - self.base.len())
        }
    }

    /// 尾部追加（restore 路径恢复 registry 祖先），返新元素的全局 idx，= 旧 `work.push(e); work.len()-1`。
    fn push(&mut self, e: CoverageElement) -> usize {
        let idx = self.len();
        self.overlay.push(e);
        idx
    }

    /// 首个满足 `pred` 的元素全局 idx，= 旧 `work.iter().position(pred)`（base 在前 overlay 在后）。
    fn position(&self, mut pred: impl FnMut(&CoverageElement) -> bool) -> Option<usize> {
        self.base.iter().position(&mut pred).or_else(|| {
            self.overlay.iter().position(pred).map(|i| self.base.len() + i)
        })
    }

    /// 全元素迭代（base 前缀 ++ overlay），= 旧 `work.iter()`。
    fn iter(&self) -> impl Iterator<Item = &CoverageElement> {
        self.base.iter().chain(self.overlay.iter())
    }

    /// base 前缀切片（树元素段，candidate_start 在 base 内 ⟹ 零拷贝），= 旧 `&work[..tree_end]`。
    fn tree_prefix(&self, tree_end: usize) -> &[CoverageElement] {
        &self.base[..tree_end.min(self.base.len())]
    }

    /// 连续切片视图（喂仍接 `&[CoverageElement]` 的 [`strategy_target_legs`]）：overlay 空 ⟹ **借 base
    /// 零拷贝**（热循环常态）；非空（restore 罕触发）⟹ materialize 一次 O(tree+overlay)。
    pub(crate) fn as_contiguous(&self) -> std::borrow::Cow<'_, [CoverageElement]> {
        if self.overlay.is_empty() {
            std::borrow::Cow::Borrowed(self.base)
        } else {
            std::borrow::Cow::Owned(self.base.iter().chain(self.overlay.iter()).copied().collect())
        }
    }
}

impl std::ops::Index<usize> for ElementView<'_> {
    type Output = CoverageElement;
    fn index(&self, idx: usize) -> &CoverageElement {
        self.get(idx).expect("ElementView index out of bounds")
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  §2 元素集 E 提取（从 classifier levels + RMove::Compose 塔，真嵌套父子）
// ════════════════════════════════════════════════════════════════════════════

/// 从递归塔（`upper_moves` 序列）提取语法元素集 E（真嵌套，铁律守护）。
///
/// 输入 `tower`：classifier 各级的上级走势塔（`Vec<Vec<LeveledMove>>`，索引=级别 ℓ）。每级的
/// `LeveledMove`（`RMove::Compose`）携 `sub_moves`（真嵌套子走势，descend 取回的子声部）。本函数
/// 递归展开塔：每个 `LeveledMove` 是一个元素 e（带 λ_e/ρ_e/ε_e/ℓ_e），其 `sub_moves` 是 e 的子
/// 元素（`parent = Some(e 的索引)`，真父子）。
///
/// ★真 Fugue 严防级别差伪造（铁律）：子元素的 `parent` 指向**包含它的 Compose 父**（真嵌套），
/// 子元素 `level = 父 level − 1`（descend 级别严格递减）。**不**把「元素在 `tower` 第几级」当父子
/// ——同级别的两个走势是兄弟（无父子），跨级别只有 Compose 嵌套才是真父子。
///
/// 返回 `Vec<CoverageElement>`：元素按提取序排列（父在子前，`parent` 索引指向已提取的父）。
/// 空塔 ⟹ 空元素集。
pub fn extract_elements(tower: &[Rc<Vec<LeveledMove>>]) -> Vec<CoverageElement> {
    let mut elements: Vec<CoverageElement> = Vec::new();
    // 顶级元素（tower 最高级的走势）是根（parent=None）；逐层向下钻取真嵌套子元素。
    // 从最高级开始（顶层走势是边界胚元 ∂ 容器下的兄弟，去根化无 RootRole）。
    for level_moves in tower.iter().rev() {
        for lm in level_moves.iter() {
            // 顶层走势 = 根元素（parent=None，attached_dir=None，parent_id=None=边界胚元 ∂）。
            push_element_tree(&mut elements, lm, None, None, None);
        }
        // 只展开最高非空级别作为根（更低级别由 Compose.subs 真嵌套带出，不重复作根）。
        if !level_moves.is_empty() {
            break;
        }
    }
    elements
}

/// ★方案D（视图分离，裁决648 / 子声部.pdf §3/§19）：**K_i 操作 carrier forest**——
/// host^op 的载体宇宙，**endpoint-complete**（∀g∈B_i ∃!c∈K_i: ρ(c)=s(g)）。
///
/// 与 [`extract_elements`]（T_i = ↓r_i，**只展开最高非空级别根**，覆盖 ~36% L0）的唯一区别：
/// 本函数遍历 tower 的**所有级别**所有 `LeveledMove`（K_i = U_i = 全量 tower 元素），使每个产出
/// 买卖点的走势（无论在哪一级）的右端点 ρ=end_index 都在 K_i 中可命中——消除 orphan frontier
/// host-miss（PDF §8/§11：T_i 只覆盖最高 compose chain ⟹ 大量 bsp 落树外 host=⊥ ⟹ 子声部恒 0）。
///
/// ★host^op 仍**严格右端点命中**（PDF P2a 保留），只是宇宙从 T_i 扩到 K_i（P2b：T_i→K_i）。父子
/// 关系仍来自真 Compose `sub_moves`（铁律：非级别差伪造，547），`parent_id` 仍是真 structural id。
///
/// ★dedup（codex 异质审查 NO#2 坐实）：同一 `LeveledMove` 会被遍历两次——一次作为**低级根**
/// （某级 `level_moves` 的顶层），一次作为**高级父的 sub_move**（高一级 Compose 的子走势）。两次产
/// 同 `ElementId`（确定性 ID）但 `parent` 不同（作根时 None，作子时指向父）。本函数按 `ElementId`
/// dedup，**优先保留带真 `parent_id` 的出现**（作子声部时携真父，是 host^op 父链所需；作根时父=∂
/// 丢失父信息）——保 K_i 中每元素唯一表示且父链最全（endpoint 唯一命中不二义）。
///
/// > **认识论 L0/L1**（formalization-validity-domain 231号）：纯结构遍历（全量 tower 展开 + dedup），
/// > 不依赖经验数据。endpoint-complete 是 K_i 定义的代数性质（L0）。是否让 π^bsp 子声部激活 >0 是
/// > 下游解释器（含父子证书链）+ L2 经验问题，本函数只提供 host^op 宇宙，**不**蕴含子声部激活。
pub fn extract_carrier_forest(tower: &[Rc<Vec<LeveledMove>>]) -> Vec<CoverageElement> {
    let mut elements: Vec<CoverageElement> = Vec::new();
    // 遍历**所有级别**（不像 extract_elements 只 break 最高非空级），每级每个走势作根向下展开真嵌套。
    for level_moves in tower.iter().rev() {
        for lm in level_moves.iter() {
            push_element_tree(&mut elements, lm, None, None, None);
        }
    }
    // dedup（codex NO#2）：同一 ElementId 多次出现（作低级根 + 作高级父的 sub），保带真 parent_id 者。
    let mut best_idx: std::collections::HashMap<ElementId, usize> = std::collections::HashMap::new();
    for (i, e) in elements.iter().enumerate() {
        match best_idx.get(&e.id) {
            // 已有记录：仅当新出现带真 parent_id 而旧的无父时，替换（父链更全，host^op 父链所需）。
            Some(&old) if elements[old].parent_id.is_none() && e.parent_id.is_some() => {
                best_idx.insert(e.id, i);
            }
            Some(_) => {}                  // 旧已带父或新也无父 ⟹ 保旧（首次出现序）。
            None => { best_idx.insert(e.id, i); }
        }
    }
    // 重建去重 Vec：选中元素按原 idx 升序（父在子前不变量保持），parent 索引重映射到去重后位置。
    let mut selected: Vec<usize> = best_idx.values().copied().collect();
    selected.sort_unstable();
    let pos_of: std::collections::HashMap<usize, usize> =
        selected.iter().enumerate().map(|(new, &old)| (old, new)).collect();
    selected
        .iter()
        .map(|&old| {
            let e = elements[old];
            CoverageElement {
                // parent（per-bar Vec 索引）重映射：旧 parent idx → 其 id 的去重后选中位置。
                parent: e.parent.and_then(|pidx| {
                    let pid = elements[pidx].id;
                    best_idx.get(&pid).and_then(|&sel| pos_of.get(&sel).copied())
                }),
                ..e
            }
        })
        .collect()
}

/// 递归把一个 `LeveledMove` 及其真嵌套子走势压入元素集（父在子前，parent 索引真父子）。
///
/// `lm`：当前走势（一个元素 e）。`parent_idx`：父元素在 `elements` 中的索引（根 None）。
/// `parent_dir`：父元素方向 σ_{α_e}（根 None）。先压入 e 自身，记其索引 `my_idx`；再对 e 的
/// `sub_moves`（真嵌套子声部）递归，子元素 `parent = Some(my_idx)`（真父子）。
fn push_element_tree(
    elements: &mut Vec<CoverageElement>,
    lm: &LeveledMove,
    parent_idx: Option<usize>,
    parent_dir: Option<VoiceSide>,
    parent_id: Option<ElementId>,
) {
    let eps = rmove_side(&lm.rmove);
    let my_idx = elements.len();
    elements.push(CoverageElement {
        lambda: lm.start_index,
        rho: lm.end_index,
        eps,
        level: lm.rmove.level(),
        parent: parent_idx,
        attached_dir: parent_dir,
        id: lm.id,
        parent_id,
    });
    // 真嵌套子声部（descend 取回的子走势携坐标侧车 `sub_moves`）⟹ 子元素 parent=my_idx。
    for sub in &lm.sub_moves {
        push_element_tree(elements, sub, Some(my_idx), Some(eps), Some(lm.id));
    }
}

/// 走势的绝对方向 ε_e（Up=Long / Down=Short，M17 方向二值）。
///
/// `RMove::Segment` 直接取 direction；`RMove::Compose` 取外缘趋势方向（首尾 hi 比较，与
/// `recursive_tower::LeveledMove::fold_direction` / `classifier::mod::rmove_direction` 同口径——
/// 外缘上移=Up=Long，下移=Down=Short）。空 subs ⟹ 缺省 Up（防御性，与塔口径一致）。
fn rmove_side(m: &RMove) -> VoiceSide {
    let dir = match m {
        RMove::Segment { direction, .. } => *direction,
        RMove::Compose { subs, .. } => match (subs.first(), subs.last()) {
            (Some(f), Some(l)) if l.hi() >= f.hi() => Direction::Up,
            (Some(_), Some(_)) => Direction::Down,
            _ => Direction::Up,
        },
    };
    match dir {
        Direction::Up => VoiceSide::Long,
        Direction::Down => VoiceSide::Short,
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  §2b 买卖点候选 → 真元素树附着（638 hostOf：本级右端点命中，非区间包含）
//  谱系：.chanlun/genealogy/pending/638-bsp-host-attachment-rule-endpoint-hit-not-interval-containment.md
// ════════════════════════════════════════════════════════════════════════════

/// 把一个买卖点候选 g 按 **638 hostOf 判准**附着到 [`extract_elements`] 建出的真元素树。
///
/// 给定真元素树 `tree`（[`extract_elements`] 产，含 `RMove::Compose` 真父子）+ 候选所在级别
/// `level_g`（ℓ_g）+ 候选 `source_index`，返回候选继承的 `(parent(g), σ_{p(g)})`：
///
/// ```text
/// hostOf(g) = tree 中 级别==ℓ_g、右端点 ρ_e==source_index 的元素（产出该买卖点的走势）
/// parent(g) = hostOf(g).parent（hostOf 在塔里的真 Compose 父；push_element_tree 真父子）
/// σ_{p(g)}  = hostOf(g).attached_dir（= parent(g) 的走势方向，push_element_tree 已填）
/// ```
///
/// 返回 `(host.parent, host.attached_dir)`——候选 g 坐在 hostOf 右端点上（buy/sell 点 = 走势端点，
/// reference:16），其操作角色 V 相对 hostOf 的父容器 p(g) 判定，故**继承 hostOf 的 `parent`
/// （父元素索引）+ `attached_dir`（σ_{p(g)}）**。
///
/// ## 638 三漏洞防护（codex 标，实装必防，违则重蹈旧 bug）
///
/// 1. **ℓ_g 上下文（漏洞①）**：`level_g` 是显式入参（调用方从 `Classification.levels` 层索引取）
///    ——**不从 `source_index` 反推级别**。host 匹配按 `e.level == level_g` 过滤：右端点 ρ 沿塔
///    最右脊跨级共享（`compose` 的 `end_index == 末 sub.end_index`），不过滤级别会误命中上级走势。
/// 2. **严格右端点命中（漏洞②）**：匹配 `e.rho == source_index`（hostOf 是**产出**该买卖点的走势，
///    其 `end_index == source_index`，signal.rs:101 构造）。**禁** `e.lambda == source_index`（左
///    端点）或 `e.lambda <= source_index <= e.rho`（区间包含）——相邻走势共享端点 ⟹ 区间包含恒
///    二义（638 矛盾的精确形式）。共享端点处（前一走势 ρ == 后一走势 λ）唯一命中**右端归产出段**
///    （前一走势，ρ 命中），非后一走势。
/// 3. **ordinal 身份非结构相等（漏洞③）**：host 按 `(level, rho)` **坐标身份**匹配——`rho`
///    （= end_index）在同级别单调唯一（`compose_level` 非重叠窗口，坐标严格递增）。**不**用
///    `RMove` 结构相等（`index_of_in` recursive_tower.rs:251 的局限：结构全等走势坐标不可区分）。
///    `tree` 的父链由 [`push_element_tree`] 按**位置 my_idx** 建（非结构查表），故结构全等的子走势
///    在 `tree` 里是不同索引的不同元素（ρ 区分），附着按 ρ 命中正确 host + 其真父。
///
/// > **结果包六要素**
/// > - **结论**：买卖点候选 g 附着到 hostOf（本级右端点命中），继承 hostOf 的真 Compose 父
/// >   `parent(g)` + 父方向 σ_{p(g)}；返回 `(host.parent, host.attached_dir)`。
/// > - **定义依据**：638 裁定式（canonical 定义式）`hostOf(g)=规范塔中 ℓ_g 级、ρ==source_index 的
/// >   走势 LeveledMove；parent(g)=hostOf 的真 Compose 父；σ_{p(g)}=parent.rmove_side`。输入
/// >   `CoverageElement.rho`（= `LeveledMove.end_index`）满足 ρ==source_index（signal.rs:101
/// >   `source_index: s.end_index`）；`parent`/`attached_dir` 由 [`push_element_tree`] 从 `sub_moves`
/// >   真包含填（铁律：父只来自真 Compose，非级别差伪造，547 否定旧 bug）。
/// > - **边界条件**：① host 未找到（候选级别 ℓ_g 无 ρ==source_index 的元素：该走势未被任何
/// >   Compose 父收录，或 `tree` 不含该级别）⟹ `(None,None)`（无真 Compose 父 = 边界胚元 ∂，σ=0 →
/// >   V=Ambient，去根化诚实退化）。② host 是顶层根（`tree` 最高级走势，parent=None）⟹ `(None,None)`
/// >   （根走势无父，V=Ambient）。③ `tower.len()<2`（仅 L0）⟹ `tree` 全根 segment ⟹ 恒 `(None,None)`
/// >   （缺塔诚实退化，tower-export-i 边界；接真塔 tower.len()>=2 才翻转为真父附着）。④ 若构造改变使
/// >   source_index 不再恒等于 end_index（638 边界条件）⟹ 右端点命中失效，需重裁。
/// > - **下游推论**：候选元素继承真 `parent`/`attached_dir` ⟹ [`vertical_relation`] V 真出
/// >   FollowParent（δ_g=σ_{p(g)}）/ ShortDiff（δ_g=−σ_{p(g)} 反向子声部对冲腿），非恒 Ambient；
/// >   [`ancestor_close`] AncOK 对附着候选真剪枝（祖先=hostOf 父链，非空）；[`leg_target`] 深度权重
/// >   按真嵌套深度（沿真父链）≥1。#5 多声部对冲 alpha 来源**结构性就位**（alpha 未验证，待 L2/L3）。
/// > - **谱系引用**：638（本附着判准 settle 条件之一=本函数实装坐实）；547（级别差伪造父的否定，
/// >   铁律来源）；coverage-engine-needs-tower-export-bridge（互斥全定义策略=买卖点入场+多级角色/
/// >   嵌套对冲）；b2s2-still-missing-tower（盲接 classify() 空跑的反面：本函数喂真嵌套塔）。
/// >   独立未裁自由度：638 (B) 操作角色"主力级别"绝对锚（生成态，编排者待裁）——本函数只兑现
/// >   settle 的 hostOf 附着（σ_{p(g)}），不触 (B) 主力锚。
/// > - **影响声明**：新增 coverage.rs §2b（本函数）；不改 [`extract_elements`]/[`push_element_tree`]
/// >   /[`from_classification_levels`]/活动集/角色判定/mod.rs。被 interp.rs
/// >   `coverage_elements_with_tower`/`assemble_gamma_with_tower` 消费（替换扁平全根）。
/// >
/// > **认识论 L0/L1**（formalization-validity-domain 231号）：纯结构查表（端点坐标匹配 + 真父继承），
/// > 不依赖经验数据。
pub fn attach_bsp_to_tree(
    tree: &[CoverageElement],
    level_g: u32,
    source_index: usize,
) -> (Option<usize>, Option<VoiceSide>) {
    // hostOf(g)：本级（漏洞①级别过滤）右端点命中（漏洞② ρ==source_index）、坐标 ordinal 身份
    // （漏洞③ 非 RMove 结构相等）的元素。ρ 同级唯一 ⟹ find 首个即唯一 host。
    // ponytail: H7 预建 (level, ρ)→idx HashMap 把 O(|tree|) 线性 .find() 降为 O(1) 查表。
    // bit-exact：ρ 同级单调唯一（compose 非重叠窗口）⟹ (level,ρ) 唯一命中 == 旧 .find() 首个。
    let idx = build_tree_endpoint_index(tree);
    match idx.get(&(level_g, source_index)).copied() {
        Some(host_idx) => {
            let host = &tree[host_idx];
            (host.parent, host.attached_dir)
        }
        None => (None, None),
    }
}

/// ponytail: H7 预建 (level, ρ) → idx 索引——hostOf 查表 O(1)。
/// bit-exact 依据：ρ（= end_index）在同级别单调唯一（compose_level 非重叠窗口，坐标严格递增）
/// ⟹ (level, ρ) 唯一，无多匹配。热循环 coverage_elements_and_gamma_with_tower 单次建、多次查
/// （tree 前缀不变）。
pub fn build_tree_endpoint_index(
    tree: &[CoverageElement],
) -> std::collections::HashMap<(u32, usize), usize> {
    let mut idx: std::collections::HashMap<(u32, usize), usize> =
        std::collections::HashMap::new();
    for (i, e) in tree.iter().enumerate() {
        // ρ 同级唯一 ⟹ 后插入不会覆盖已存在的（compose 非重叠）。用 entry 保首个（与 .find() 首个一致）。
        idx.entry((e.level, e.rho)).or_insert(i);
    }
    idx
}

/// ponytail: H7 带预建索引的 attach_bsp_to_tree 变体——热循环单次建索引、多次查。
/// bit-exact == attach_bsp_to_tree（同 hostOf 判定：(level,ρ) 命中取首个）。
pub fn attach_bsp_to_tree_indexed(
    tree_idx: &std::collections::HashMap<(u32, usize), usize>,
    tree: &[CoverageElement],
    level_g: u32,
    source_index: usize,
) -> (Option<usize>, Option<VoiceSide>) {
    match tree_idx.get(&(level_g, source_index)).copied() {
        Some(host_idx) => {
            let host = &tree[host_idx];
            (host.parent, host.attached_dir)
        }
        None => (None, None),
    }
}

/// ★工位 H（级别容器.pdf §13/§14）：hostOf(g) = **carrier 容器**本身（产出 g 的走势元素），
/// 返回 `(host.parent, host.attached_dir, host.id)`——比 [`attach_bsp_to_tree_indexed`] 多返回
/// **carrier 自身 ElementId**。
///
/// PDF §13 修复要害：开仓激活的 PositionNode 身份 = `hostOf(g)`（carrier 容器），**不是**买卖点叶子
/// 的新 ordinal（PDF §6/§7：叶子作持仓 ⟹ a_carrier=0 ⟹ 祖先闭合 a_child≤a_carrier=0 ⟹ depth>0 永剪）。
/// carrier 自身的 ElementId 跨 bar 稳定（确定性 (level,ordinal)），故位置节点按 carrier 身份跨 bar 对位
/// （§9.1 `p_V(u)=v ⟺ par_C(κ(u))=κ(v)`：子声部 carrier 的父容器 = 父声部 carrier）。
///
/// ★简化标注（PDF §14）：本实装用 carrier id 作位置节点身份（= PDF §14 简化版 `host.active=true`），
/// **同一 carrier 同 bar 多买卖点会共享 id**（损失 entry-level 区分）。PDF 更严格版 = `posId =
/// hash(carrier_id, entry_signal, side, generation)` position instance——见 §H ceiling。
pub fn attach_bsp_carrier_indexed(
    tree_idx: &std::collections::HashMap<(u32, usize), usize>,
    tree: &[CoverageElement],
    level_g: u32,
    source_index: usize,
) -> (Option<usize>, Option<VoiceSide>, Option<ElementId>) {
    match tree_idx.get(&(level_g, source_index)).copied() {
        Some(host_idx) => {
            let host = &tree[host_idx];
            (host.parent, host.attached_dir, Some(host.id))
        }
        None => (None, None, None),
    }
}

/// ★Lift.Context^δ_j 投影载体（pi_bsp_timing 工位 D续）：返回 host 的真 Compose 父容器 c 的
/// `(parent_id, level_c, rho_c)`——子声部 Lift 的 Context 谓词需上级 carrier c 的 **级别 ℓ_c +
/// 右端点 ρ_c** 才能去 `Classification.levels[ℓ_c].bsp` 找「g 证成上级第 j 类」的已确认买卖点。
///
/// 与 [`attach_bsp_carrier_indexed`] 同 hostOf 判准（(level,ρ)==source_index 命中），只是额外返回父
/// carrier 的 level/rho（[`attach_bsp_carrier_indexed`] 只返回 `host.parent` 索引 + `host.id`，丢了
/// 父的坐标）。host-miss / host 是根（无父）⟹ 父三元组 None。
///
/// > 认识论 L0：纯结构查表（端点坐标 + 父链坐标），不依赖经验数据。
pub fn attach_bsp_parent_carrier_indexed(
    tree_idx: &std::collections::HashMap<(u32, usize), usize>,
    tree: &[CoverageElement],
    level_g: u32,
    source_index: usize,
) -> Option<(ElementId, u32, usize)> {
    let host_idx = tree_idx.get(&(level_g, source_index)).copied()?;
    let parent_idx = tree[host_idx].parent?;
    let parent = &tree[parent_idx];
    Some((parent.id, parent.level, parent.rho))
}

/// 直接从 [`Classification`] 提取元素集 E（生产入口：消费 classifier 输出）。
///
/// classifier 的 `Classification.levels` 当前不直接携 `LeveledMove` 塔（塔在 classify 内部构造后
/// 投影为 `UnitRange` 喂下一级，`LeveledMove` 不出 classify）——故本入口接受**外部传入的塔**
/// （由调用方在 classify 时旁路保留，或由 [`from_classification_levels`] 从 levels 的中枢/走势
/// 重建一个**单级近似塔**）。生产路径优先用真塔（[`extract_elements`]）。
///
/// ★诚实 still-MISSING（no-声明膨胀）：`Classification` 不导出 `LeveledMove` 塔（owner 边界：
/// 不改 classify 签名）。本入口从 `levels[ℓ].moves` 重建的是**级别×走势的扁平近似**（每级走势作
/// 边界胚元 ∂ 下的同级兄弟，`parent=None`+`attached_dir=None`，无 Compose 真嵌套）——故父容器方向
/// σ_{p(g)}=0，垂直关系 V 恒 **Ambient**（**不产** FollowParent/ShortDiff，短差需真父子方向）；水平
/// 关系 H 在同级兄弟间取 First/SameFollow/SameReverse。完整 element-coverage（含短差反向子声部腿）
/// 须用真 `LeveledMove` 塔（[`extract_elements`]）。本扁平入口用于 L2 净额回测的**根级覆盖基线**。
pub fn from_classification_levels(classification: &Classification) -> Vec<CoverageElement> {
    let mut elements: Vec<CoverageElement> = Vec::new();
    // 每级的每个走势中枢区间作一个根级覆盖元素（同级兄弟，parent=None=边界胚元 ∂，去根化）。
    // 区间 [λ_e,ρ_e) 取该级中枢的 start_index..end_index（走势的操作区间近似）。
    //
    // ★诚实占位（no-声明膨胀）：`eps` 恒 `Long`——`MoveKind`（Trend/Consolidation）**不携方向**
    // （types.rs:127），`Center` 也不携方向，故扁平入口**无法**从 `Classification` 可靠取元素绝对
    // 方向 ε_e。`eps=Long` 是占位（不伪装从 move 读方向）。这是扁平入口不完整的另一面：元素方向
    // 须从真 `LeveledMove` 塔（[`extract_elements`] 的 `rmove_side` 读外缘趋势/线段方向）取——完整
    // element-coverage（含方向感知 + ShortDiff）用真塔，本扁平入口仅作根级覆盖区间基线。
    for (level_idx, level) in classification.levels.iter().enumerate() {
        for center in &level.centers {
            elements.push(CoverageElement {
                lambda: center.start_index,
                rho: center.end_index,
                eps: VoiceSide::Long, // 占位：Classification 不携元素方向（见上诚实标注）
                level: level_idx as u32,
                parent: None,
                attached_dir: None,
                // 扁平入口无真塔 ⟹ 确定性 ID 退化为 (level, center 序号)；parent_id=None=∂。
                id: ElementId { level: level_idx as u32, ordinal: elements.len() as u64 },
                parent_id: None,
            });
        }
    }
    elements
}

// ════════════════════════════════════════════════════════════════════════════
//  §3 活动集递归 A_{t+1} = AncOK[(A_t∖D_t)∪B_t]（M16 AncestorClosure）
//
//  ★GAP-5 入场源边界（铁律）：本节 `B_t={e:λ_e=t}`/`D_t={e:ρ_e=t}` 是 **Lean M16 区间递归
//  原语**（元素操作区间端点 λ_e/ρ_e 驱动）——`λ_e` 来自走势边界（`LeveledMove.start_index`，
//  见 [`push_element_tree`] line 137）。**这不是生产入场源。** spec §13（line 657-661）的**正典**
//  活动集递归用解释器三桶 `𝒟_x/ℬ_x = ℛ_Θ(Γ(x))`（**买卖点 Γ(x)** 驱动，§8
//  [`coverage_step_from_buckets`]）。生产入场 = §8 买卖点 Γ 路径 + §9 全定义策略 π_Θ，**非**本节
//  λ_e 走势边界。本节原语（[`active_set_step`]/[`starting_set`]/[`ending_set`]）保留作 Lean M16
//  形式化对齐（公开 + 测试），其下层 [`raw_active_set`]/[`ancestor_close`] 被 §8 买卖点路径复用。
// ════════════════════════════════════════════════════════════════════════════

/// 开始集 `B_t = {e : λ_e = t}`（M16 §五，bar t 新开始的元素索引）。
pub fn starting_set(elements: &[CoverageElement], t: usize) -> Vec<usize> {
    elements
        .iter()
        .enumerate()
        .filter(|(_, e)| e.lambda == t)
        .map(|(i, _)| i)
        .collect()
}

/// 结束集 `D_t = {e : ρ_e = t}`（M16 §五，bar t 结束的元素索引）。
pub fn ending_set(elements: &[CoverageElement], t: usize) -> Vec<usize> {
    elements
        .iter()
        .enumerate()
        .filter(|(_, e)| e.rho == t)
        .map(|(i, _)| i)
        .collect()
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
/// 树深有限 ⟹ 链有限，无 fuel 需要。环不可能——parent_id 严格指向更高级别（`push_element_tree`
/// 父 level > 子 level，descend 级别严格递减保证）。
fn ancestors_by_id_lookup(
    elements: &ElementView,
    e_idx: usize,
    lookup: &impl Fn(&ElementId) -> Option<usize>,
) -> Vec<ElementId> {
    let mut chain = Vec::new();
    let mut cur = elements.get(e_idx).and_then(|e| e.parent_id);
    while let Some(pid) = cur {
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
/// ★此为**索引链**版本（§3 Lean M16 区间递归原语对齐，非生产入场）。§13 生产 AncOK 用
/// [`ancestor_close_by_id`]（parent_id 结构映射，spec §13）。
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
fn ancestor_close_by_id(elements: &ElementView, raw: &[usize]) -> Vec<usize> {
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
//  §4 操作角色 R(g)=(H(g),V(g),δ_g) 18 类完全分类（spec §7-§8 / P6-P7，去根化）
// ════════════════════════════════════════════════════════════════════════════

/// 方向 δ_g ∈ {+1,-1}（spec §8 / P7 第三轴）。
///
/// `+1` = 买入方向，`-1` = 卖出方向（spec §6 / P4 `δ∈{+1,-1}`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    /// +1：买入方向。
    Plus,
    /// -1：卖出方向。
    Minus,
}

/// 水平关系 H(g)（spec §7.1 / P6，相对同一父容器下的前一同级别兄弟 prev(g)）。
///
/// 三类互斥穷尽（spec P6 `Σ_{h} 1[H(g)=h]=1`；穷尽性依赖 δ_g,σ_{prev}∈{+1,-1} 二值）：
/// - `First`：prev(g)=∅（无前兄弟——去根化：顶层元素亦是边界胚元 ∂ 容器下的兄弟，`First` 非根特例）。
/// - `SameFollow`：prev(g)≠∅ ∧ δ_g = σ_{prev(g)}（与前兄弟同向延续）。
/// - `SameReverse`：prev(g)≠∅ ∧ δ_g = −σ_{prev(g)}（相对前兄弟反向，方向二值下非顺即反）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Horizontal {
    /// prev(g)=∅（无前兄弟）。
    First,
    /// δ_g = σ_{prev(g)}（同向前兄弟）。
    SameFollow,
    /// δ_g = −σ_{prev(g)}（反向前兄弟）。
    SameReverse,
}

/// 垂直关系 V(g)（spec §7.2 / P6-P7，相对父容器方向 σ_{p(g)} ∈ {-1,0,+1}）。
///
/// 三类互斥穷尽（spec P6 `Σ_{v} 1[V(g)=v]=1`）：
/// - `Ambient`：σ_{p(g)}=0（父容器无方向/胚元/空）——spec P7 显式：**Ambient 不是根规则**，
///   而是任意父容器处于无方向状态时的普通情形（去根化核心）。
/// - `FollowParent`：σ_{p(g)}≠0 ∧ δ_g = σ_{p(g)}（次级别顺父方向）。
/// - `ShortDiff`：σ_{p(g)}≠0 ∧ δ_g = −σ_{p(g)}（短差 = 父级方向的反向子操作，spec §9 / P8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vertical {
    /// σ_{p(g)}=0（父无方向 → 去根化 Ambient）。
    Ambient,
    /// δ_g = σ_{p(g)}（顺父方向）。
    FollowParent,
    /// δ_g = −σ_{p(g)}（短差，反向子操作）。
    ShortDiff,
}

/// 完整操作角色 R(g) = (H(g), V(g), δ_g)（spec §8 / P7，3×3×2 = **18 类完全分类**）。
///
/// 角色空间 ℛ = {First,SameFollow,SameReverse} × {Ambient,FollowParent,ShortDiff} × {+1,-1}，
/// 基数 18；对任意 g 严格互斥完全分类 `Σ_{r∈ℛ} 1[R(g)=r]=1`（spec P7 方框）。
///
/// ## 去根化（spec §7 / P7 / 差异表 #2）
/// 旧 {Root, Same, Sub}（含根特例）→ 水平 H × 垂直 V × 方向 δ 三正交轴。**无 RootRole 枚举值**：
/// 旧"根元素"now = (First/SameFollow/SameReverse, **Ambient**, ±1)——根被边界胚元 ∂ + Ambient 吸收。
///
/// ## 认识论等级（formalization-validity-domain 231号，强制标注）
/// - **L0**（代数命题，本结构）：18 = 3×3×2 完全分类 `Σ=1` 是同义反复（三轴各自互斥穷尽的笛卡尔
///   积），信息增量为零。
/// - **L2 未覆盖**（spec 疑点5，no-声明膨胀）：3×3×2=18 全组合是否在真实数据上**均可达**，还是部分
///   组合经验为空（有效域 < 定义域），spec **未覆盖**——本实装**不**声称 18 类经验全可达。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationRole {
    /// H(g)：水平关系（同级别兄弟轴）。
    pub h: Horizontal,
    /// V(g)：垂直关系（父容器轴）。
    pub v: Vertical,
    /// δ_g：方向（±1）。
    pub delta: Dir,
}

/// δ_g：元素方向（spec §8，`VoiceSide → {+1,-1}`）。Long=+1=Plus，Short=−1=Minus。
///
/// ★ε_e/δ_g 二值不变量（M17 / `CoverageElement` 文档）：元素绝对方向恒 ∈{Long,Short}
/// （`rmove_side` 只产 Long/Short）；`Flat` 不是合法元素方向（spec δ∈{+1,-1} 无 0），不可达，
/// 防御性归 Plus 以保全函数性（不伪装 Flat 有方向）。
fn direction_of(side: VoiceSide) -> Dir {
    match side {
        VoiceSide::Long => Dir::Plus,
        VoiceSide::Short => Dir::Minus,
        VoiceSide::Flat => Dir::Plus, // ε_e 二值不变量下不可达（防御性，不伪装方向）
    }
}

/// δ_g 的符号 {+1,-1}（与 σ 比较用）。
fn dir_sign(d: Dir) -> i8 {
    match d {
        Dir::Plus => 1,
        Dir::Minus => -1,
    }
}

/// σ_{p(g)} ∈ {-1,0,+1}：父容器方向（spec §7.2）。
///
/// `None`（父为边界胚元 ∂，去根化）或 `Some(Flat)`（父无方向）→ 0；`Some(Long)`→+1；`Some(Short)`→−1。
/// （voice.rs `Flat` 文档明示 `σ=0 是无方向态`，与此一致。）
fn parent_sign(attached_dir: Option<VoiceSide>) -> i8 {
    match attached_dir {
        None => 0,
        Some(VoiceSide::Long) => 1,
        Some(VoiceSide::Short) => -1,
        Some(VoiceSide::Flat) => 0,
    }
}

/// 水平关系 H(g)（spec §7.1 / P6，全函数唯一判定）。
///
/// prev(g) = 同一父容器（`parent` 相等，None==None 即同一边界胚元 ∂）下、同级别（`level` 相等）、
/// 索引 < `e_idx` 的**最近**兄弟。无则 `First`；有则按 δ_g vs σ_{prev(g)} 分 SameFollow/SameReverse。
///
/// ★去根化：顶层元素（`parent=None`）亦视为边界胚元 ∂ 容器下的兄弟——`First` 不是根特例，
/// 只是"无前兄弟"的普通情形（spec §7.1 + P7）。
pub fn horizontal_relation(elements: &[CoverageElement], e_idx: usize) -> Horizontal {
    let e = match elements.get(e_idx) {
        Some(e) => e,
        None => return Horizontal::First, // 越界防御性（不应到达）
    };
    let delta = dir_sign(direction_of(e.eps));
    // ponytail: H5 预建 (parent,level)→idx 列表 + 二分查 < e_idx 的最大 idx，把 O(e_idx) 线性
    // .rev().find() 降为 O(log n) 查表。bit-exact：同级兄弟 ρ 单调递增（compose 非重叠窗口）⟹
    // 列表升序，partition_point 取 < e_idx 的末个 == 旧 .rev().find() 首个。
    let idx = build_prev_sibling_index(elements);
    let prev = idx.get(&(e.parent, e.level)).and_then(|idxs| {
        // idxs 升序（push 序）⟹ partition_point(|&i| i < e_idx) 取首个 >= e_idx 的位置，
        // 前一位即 < e_idx 的最大 idx（最近前兄弟）。
        let pos = idxs.partition_point(|&i| i < e_idx);
        if pos == 0 {
            None
        } else {
            Some(idxs[pos - 1])
        }
    });
    match prev {
        Some(p) => {
            let sigma_prev = dir_sign(direction_of(elements[p].eps));
            if delta == sigma_prev {
                Horizontal::SameFollow
            } else {
                Horizontal::SameReverse
            }
        }
        None => Horizontal::First,
    }
}

/// ponytail: H5 预建 (parent, level) → 该键所有元素索引（升序，push 序）。
/// bit-exact 依据：同级兄弟按 push 序排列（extract_elements 父在子前 + 候选 append 序），
/// 二分查 < e_idx 的最大 idx == 旧 (0..e_idx).rev().find() 首个。
/// 返回 HashMap 可被多次 operation_role 调用复用（热循环 strategy_target_legs 单次建、多次查）。
pub fn build_prev_sibling_index(
    elements: &[CoverageElement],
) -> std::collections::HashMap<(Option<usize>, u32), Vec<usize>> {
    let mut idx: std::collections::HashMap<(Option<usize>, u32), Vec<usize>> =
        std::collections::HashMap::new();
    for (i, e) in elements.iter().enumerate() {
        idx.entry((e.parent, e.level)).or_default().push(i);
    }
    idx
}

/// ponytail: H5 带预建索引的 operation_role 变体——热循环（strategy_target_legs）单次建索引、
/// 多次查，消除每元素 O(e_idx) 线性扫。bit-exact == operation_role（同 prev 判定逻辑）。
pub(crate) fn operation_role_indexed(
    elements: &ElementView,
    e_idx: usize,
    sibling_idx: &std::collections::HashMap<(Option<usize>, u32), Vec<usize>>,
) -> OperationRole {
    let e = match elements.get(e_idx) {
        Some(e) => e,
        None => {
            return OperationRole {
                h: Horizontal::First,
                v: Vertical::Ambient,
                delta: Dir::Plus,
            }
        }
    };
    let delta = direction_of(e.eps);
    // H(g)：查 sibling_idx 取 < e_idx 的最大 idx（最近前兄弟）。
    let h = match sibling_idx.get(&(e.parent, e.level)).and_then(|idxs| {
        let pos = idxs.partition_point(|&i| i < e_idx);
        if pos == 0 {
            None
        } else {
            Some(idxs[pos - 1])
        }
    }) {
        Some(p) => {
            let sigma_prev = dir_sign(direction_of(elements[p].eps));
            if dir_sign(delta) == sigma_prev {
                Horizontal::SameFollow
            } else {
                Horizontal::SameReverse
            }
        }
        None => Horizontal::First,
    };
    // V(g)：不变（O(1)，parent_sign + direction_of）。
    let sigma_parent = parent_sign(e.attached_dir);
    let v = if sigma_parent == 0 {
        Vertical::Ambient
    } else if dir_sign(delta) == sigma_parent {
        Vertical::FollowParent
    } else {
        Vertical::ShortDiff
    };
    OperationRole { h, v, delta }
}

/// ★工位 4c：双段兄弟索引的 [`operation_role_indexed`]——`tree_sibling`（缓存的 tree-only，命中
/// `Rc::clone` 复用）+ `cand_sibling`（本 bar candidate-only overlay）。消除每 bar 把 candidate idx
/// merge 进 tree sibling_idx 的 O(tree) clone（缓存被 mutate ⟹ 不能 Rc 共享）。
///
/// **bit-exact == [`operation_role_indexed`]（合并 sibling_idx）**：合并列表 = tree 段同键（全
/// `< candidate_start ≤ e_idx`，升序）++ candidate 段同键（升序）整体升序（tree idx < candidate idx）。
/// `partition_point(< e_idx)` 的前一个 = candidate 段 `< e_idx` 的最大 idx（存在则 > 任何 tree idx）；
/// 否则 = tree 段同键最末（最大 tree idx，全 `< e_idx`）。
pub(crate) fn operation_role_indexed_split(
    elements: &ElementView,
    e_idx: usize,
    tree_sibling: &std::collections::HashMap<(Option<usize>, u32), Vec<usize>>,
    cand_sibling: &std::collections::HashMap<(Option<usize>, u32), Vec<usize>>,
) -> OperationRole {
    let e = match elements.get(e_idx) {
        Some(e) => e,
        None => {
            return OperationRole {
                h: Horizontal::First,
                v: Vertical::Ambient,
                delta: Dir::Plus,
            }
        }
    };
    let delta = direction_of(e.eps);
    let key = (e.parent, e.level);
    // 最近前兄弟：先查 candidate overlay（< e_idx 的最大；其 idx > 任何 tree idx），无则 tree 段末尾。
    let prev = cand_sibling
        .get(&key)
        .and_then(|idxs| {
            let pos = idxs.partition_point(|&i| i < e_idx);
            (pos > 0).then(|| idxs[pos - 1])
        })
        .or_else(|| tree_sibling.get(&key).and_then(|idxs| idxs.last().copied()));
    let h = match prev {
        Some(p) => {
            let sigma_prev = dir_sign(direction_of(elements[p].eps));
            if dir_sign(delta) == sigma_prev {
                Horizontal::SameFollow
            } else {
                Horizontal::SameReverse
            }
        }
        None => Horizontal::First,
    };
    let sigma_parent = parent_sign(e.attached_dir);
    let v = if sigma_parent == 0 {
        Vertical::Ambient
    } else if dir_sign(delta) == sigma_parent {
        Vertical::FollowParent
    } else {
        Vertical::ShortDiff
    };
    OperationRole { h, v, delta }
}

/// ★工位 4d：双段兄弟索引的 [`operation_role_indexed`] 通用版——`base_sibling`（缓存的 tree-only，
/// `Rc::clone` 复用）+ `overlay_sibling`（本 bar overlay 段 = candidate ++ restore，现建小索引）。
/// 用于 [`strategy_target_legs`]：active 元素可能在 **base 段**（持仓腿对位回 tree 元素）**或 overlay 段**
/// （candidate/restore），故对**两段都做** `partition_point(< e_idx)`（不同于 [`operation_role_indexed_split`]
/// 假设 e_idx 恒在 candidate 段而 base 用 `.last()`）。
///
/// **bit-exact == [`operation_role_indexed`]（合并 sibling_idx）**：合并列表同键 = base 段同键 idx
/// （全 `< base_len`）++ overlay 段同键 idx（全 `≥ base_len`），整体升序（base idx < overlay idx）。
/// `partition_point(< e_idx)` 的前一个：overlay 段若有 `< e_idx` 的同键（其 idx > 任何 base idx）⟹ 取
/// overlay 段 `< e_idx` 最大；否则 ⟹ base 段 `< e_idx` 最大。本函数先查 overlay（partition_point），
/// 无则 fallback base（partition_point），与合并列表 partition_point 逐位等价。
pub(crate) fn operation_role_two_segment(
    elements: &ElementView,
    e_idx: usize,
    base_sibling: &std::collections::HashMap<(Option<usize>, u32), Vec<usize>>,
    overlay_sibling: &std::collections::HashMap<(Option<usize>, u32), Vec<usize>>,
) -> OperationRole {
    let e = match elements.get(e_idx) {
        Some(e) => e,
        None => {
            return OperationRole {
                h: Horizontal::First,
                v: Vertical::Ambient,
                delta: Dir::Plus,
            }
        }
    };
    let delta = direction_of(e.eps);
    let key = (e.parent, e.level);
    let prev_lt = |idxs: &Vec<usize>| {
        // idxs 升序（push 序）⟹ partition_point(< e_idx) 前一位 = < e_idx 的最大 idx（最近前兄弟）。
        let pos = idxs.partition_point(|&i| i < e_idx);
        (pos > 0).then(|| idxs[pos - 1])
    };
    let prev = overlay_sibling
        .get(&key)
        .and_then(prev_lt)
        .or_else(|| base_sibling.get(&key).and_then(prev_lt));
    let h = match prev {
        Some(p) => {
            let sigma_prev = dir_sign(direction_of(elements[p].eps));
            if dir_sign(delta) == sigma_prev {
                Horizontal::SameFollow
            } else {
                Horizontal::SameReverse
            }
        }
        None => Horizontal::First,
    };
    let sigma_parent = parent_sign(e.attached_dir);
    let v = if sigma_parent == 0 {
        Vertical::Ambient
    } else if dir_sign(delta) == sigma_parent {
        Vertical::FollowParent
    } else {
        Vertical::ShortDiff
    };
    OperationRole { h, v, delta }
}

/// 垂直关系 V(g)（spec §7.2 / P6-P7，全函数唯一判定）。
///
/// 按父容器方向 σ_{p(g)}（[`parent_sign`]）分：σ=0 → `Ambient`（去根化，spec P7）；
/// σ≠0 且 δ_g=σ → `FollowParent`；σ≠0 且 δ_g=−σ → `ShortDiff`（短差，spec §9 / P8）。
pub fn vertical_relation(elements: &[CoverageElement], e_idx: usize) -> Vertical {
    let e = match elements.get(e_idx) {
        Some(e) => e,
        None => return Vertical::Ambient, // 越界防御性（不应到达）
    };
    let sigma_parent = parent_sign(e.attached_dir);
    if sigma_parent == 0 {
        Vertical::Ambient
    } else if dir_sign(direction_of(e.eps)) == sigma_parent {
        Vertical::FollowParent
    } else {
        Vertical::ShortDiff
    }
}

/// 派生元素 e 的完整操作角色 R(g)=(H(g),V(g),δ_g)（spec §8 / P7，18 类，全函数唯一判定）。
///
/// 三轴各自由 [`horizontal_relation`] / [`vertical_relation`] / [`direction_of`] 唯一确定，
/// 元组即角色（spec `R(g)=(H(g),V(g),δ_g)`）。
///
/// > **结果包六要素**
/// > - **结论**：每个语法元素 e 的操作角色 = 三轴元组 (H,V,δ)，落入 ℛ 的 18 类之一。
/// > - **定义依据**：spec §7.1（H 相对前同级兄弟 prev(g)）+ §7.2（V 相对父方向 σ_{p(g)}）
/// >   + §8（R=H×V×δ，3×3×2=18，`Σ_{r∈ℛ}1=1`）。输入 `CoverageElement` 的 `parent`/`level` 喂 H，
/// >   `attached_dir` 喂 V，`eps` 喂 δ。
/// > - **边界条件**：① H 穷尽性依赖 δ_g,σ_{prev}∈{+1,-1} 二值——若元素方向取 Flat（σ=0），δ 不在
/// >   二值域内，H 分类退化（本实装 ε_e 二值不变量保证不发生）。② V 三分依赖 σ_{p(g)} 含 0
/// >   （胚元/Flat→Ambient）；若父方向取值域扩大，V 翻转。③ 同级别兄弟须 `parent` 与 `level` 双相等
/// >   ——只 parent 相等而 level 不等（不同级别根）不算同级兄弟（spec "同级别兄弟"）。
/// > - **下游推论**：classify 输出 (H,V,δ) 三轴元组，喂 R_Θ 解释器 / 活动集腿角色标注；
/// >   `v==ShortDiff` 标记反向子声部对冲腿（净额执行 [`net_target_units`] 部分抵消父仓）。
/// > - **谱系引用**：差异表 #2（{Root,Same,Sub}→H×V×δ）+ §9 短差去根化（旧"根多头次级别空头"→
/// >   父级反向子操作）；MEMORY coverage-engine-needs-tower-export-bridge（18 类角色 = 多级角色框架
/// >   形式化，是 classify 导出 LeveledMove 塔的前置对象升级）。旧 4 类 OperationRole（对应 Lean
/// >   MW3 / M09 {RootDir,SameDir,SubFollow,ShortDiff}）被本 18 类**替换**（非保留 fallback）。
/// > - **影响声明**：删除旧 `LevelRelation`(Root/Same/Sub) 轴 + `level_relation` + 旧 4 类
/// >   `OperationRole`；新增 `Dir`/`Horizontal`/`Vertical`/三轴 `OperationRole` 结构 + 3 个判定函数。
/// >   仅改角色分类逻辑，不动入场源（extract_elements / from_classification_levels / 活动集）。
pub fn operation_role(elements: &[CoverageElement], e_idx: usize) -> OperationRole {
    let delta = match elements.get(e_idx) {
        Some(e) => direction_of(e.eps),
        None => Dir::Plus, // 越界防御性（不应到达）
    };
    OperationRole {
        h: horizontal_relation(elements, e_idx),
        v: vertical_relation(elements, e_idx),
        delta,
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  §5 LegTarget 每活动元素一腿（M17/M28，方向 ε_e + 单位 s_e 按 role/depth 权重）
// ════════════════════════════════════════════════════════════════════════════

/// 单元素的目标头寸腿 `LegTarget(e)`（M17/M28 `TargetLeg` 的 rust 镜像）。
///
/// 每个活动元素一腿（对齐 `SeparateStrategyTarget.legTarget`）：
/// - `e_idx`：腿归属的元素索引（声部坐标 ν(e) 的 rust 表示）。
/// - `side`：腿方向 = ε_e（M17 `σ_{ν(e)}=ε_e`：多腿 Long / 空腿 Short）。
/// - `units`：目标单位数 s_e（按 role/depth 权重 `w_depth` × 基准，M28 深度权重）。
/// - `role`：元素角色 R(g)=(H,V,δ)（`role.v==ShortDiff` 标记反向子声部腿，净额执行时与父对冲）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegTarget {
    /// ν(e)：腿归属的元素索引。
    pub e_idx: usize,
    /// ε_e：腿方向（Long=多腿 / Short=空腿）。
    pub side: VoiceSide,
    /// s_e：目标单位数（role/depth 权重）。
    pub units: f64,
    /// R(g)=(H,V,δ)：元素 18 类角色（`role.v==Vertical::ShortDiff`=反向子声部腿）。
    pub role: OperationRole,
}

/// 生成单元素的目标头寸腿 `LegTarget(e)`（M17/M28，方向 ε_e + role/depth 权重单位数）。
///
/// 单位数 `s_e` = `base_units × w_depth(depth)`（M28 深度资金权重，对齐 `voice::depth_weight`：
/// w=[0.60,0.30,0.10]）。`depth` = 元素在嵌套树的深度（根=0，沿 parent 链长度）——**真嵌套深度**
/// （铁律：来自 parent 链，非级别差）。腿方向直接取 `e.eps`（M17 `σ_{ν(e)}=ε_e` 定义性满足）。
///
/// ★ShortDiff 反向子声部腿（M11/M28，hedge-subvoice 提案 P1-P4）：短差元素的腿方向 = ε_e = δ_g =
/// −σ_{p(g)}（父级方向翻转，`operation_role` 已判 `role.v==Vertical::ShortDiff`，spec §9）——父声部
/// 不动，本腿作独立反向子声部（净额执行 [`net_target_units`] 时部分对冲父仓）。本函数只产腿，对冲
/// 在净额合并实现。
pub fn leg_target(
    elements: &[CoverageElement],
    e_idx: usize,
    base_units: f64,
    config: &VoiceConfig,
) -> LegTarget {
    let e = &elements[e_idx];
    // element_depth 现接 ElementView（双段）；非 indexed 简单版包 base-only view（overlay 空，零拷贝）。
    let depth = element_depth(&ElementView::new(elements), e_idx);
    let w = depth_weight(depth, config);
    let role = operation_role(elements, e_idx);
    LegTarget {
        e_idx,
        side: e.eps,
        units: base_units * w,
        role,
    }
}

/// ★工位 4d：双段索引的 leg_target 变体——base 兄弟（缓存 tree-only）+ overlay 兄弟（本 bar candidate/
/// restore 段），热循环 strategy_target_legs 单次建 overlay 索引、多次查。
/// bit-exact == [`leg_target`]（role 经 [`operation_role_two_segment`] 双段 partition_point 同逻辑）。
fn leg_target_two_segment(
    elements: &ElementView,
    e_idx: usize,
    base_units: f64,
    config: &VoiceConfig,
    base_sibling: &std::collections::HashMap<(Option<usize>, u32), Vec<usize>>,
    overlay_sibling: &std::collections::HashMap<(Option<usize>, u32), Vec<usize>>,
) -> LegTarget {
    let e = &elements[e_idx];
    let depth = element_depth(elements, e_idx);
    let w = depth_weight(depth, config);
    let role = operation_role_two_segment(elements, e_idx, base_sibling, overlay_sibling);
    LegTarget {
        e_idx,
        side: e.eps,
        units: base_units * w,
        role,
    }
}

/// 元素的真嵌套深度（沿 parent 链长度，根=0；铁律：真父子，非级别差）。
/// parent（usize 索引）指向 base 段 carrier（< candidate_start ≤ base.len），链全在 base，bit-exact == ancestors().len()。
fn element_depth(elements: &ElementView, e_idx: usize) -> u32 {
    let mut depth = 0u32;
    let mut cur = elements.get(e_idx).and_then(|e| e.parent);
    while let Some(p) = cur {
        depth += 1;
        cur = elements.get(p).and_then(|e| e.parent);
    }
    depth
}

/// **全定义策略目标头寸腿集 `q̄_Θ = LegTarget(AncOK[(A∖D)∪B])`**（M28 §十三 总形式）。
///
/// 对祖先闭合活动集 `active`（A_{t+1}）中**每个元素**生成 [`leg_target`]，得目标头寸腿列表
/// （= 分账本目标头寸 q̄_Θ 的腿分解，对齐 `SeparateStrategyTarget.strategyTargetLegs` + M28）。
/// 每条腿带 `(ν(e), ε_e, s_e, role)`，多空独立坐标（分账本 C25），净额抵消在 [`net_target_units`]。
pub(crate) fn strategy_target_legs(
    elements: &ElementView,
    active: &[usize],
    base_units: f64,
    config: &VoiceConfig,
) -> Vec<LegTarget> {
    // ★工位 4d 热点① O(n²) 消除：base（tree 前缀）兄弟索引缓存命中复用 `Rc`（O(1)，§16 不变），
    // 仅 overlay 段（candidate ++ restore，绝大多数 bar 仅 candidate）现建小索引。旧版每 bar
    // build_prev_sibling_index(contiguous=tree+overlay) O(tree)/bar=O(n²)，现 base 段 O(1) 命中。
    // bit-exact：operation_role_two_segment 双段 partition_point == 旧合并 sibling_idx partition_point
    // （base idx 全 < base_len ≤ overlay idx ⟹ 合并列表升序，分段查等价；见 two_segment 函数 doc）。
    let base_len = elements.base.len();
    // overlay 段兄弟索引（全局 idx = base_len + i）。overlay 空（热循环常态）⟹ 空 HashMap，全查 base。
    let mut overlay_sibling: std::collections::HashMap<(Option<usize>, u32), Vec<usize>> =
        std::collections::HashMap::new();
    for (i, e) in elements.overlay.iter().enumerate() {
        overlay_sibling.entry((e.parent, e.level)).or_default().push(base_len + i);
    }
    // base 段兄弟：缓存命中复用 Rc（O(1)），缺失 fallback 现建 O(tree)（测试/无缓存路径）。
    let base_sibling_owned;
    let base_sibling: &std::collections::HashMap<(Option<usize>, u32), Vec<usize>> =
        match &elements.base_sibling_idx {
            Some(rc) => rc.as_ref(),
            None => {
                base_sibling_owned = build_prev_sibling_index(elements.base);
                &base_sibling_owned
            }
        };
    active
        .iter()
        .map(|&e_idx| {
            leg_target_two_segment(elements, e_idx, base_units, config, base_sibling, &overlay_sibling)
        })
        .collect()
}

// ════════════════════════════════════════════════════════════════════════════
//  §6 净额执行（Nautilus 净额兼容：毛分账本腿 → 净持仓 units:f64）
// ════════════════════════════════════════════════════════════════════════════

/// **净额目标持仓 `q̄_net`（毛分账本腿 → 净持仓，Nautilus 净额兼容）**。
///
/// 所有目标腿（[`strategy_target_legs`] 产）合并为**净持仓单位数** `units:f64`（有符号，正=净多/
/// 负=净空），对齐 `plan_and_fill_mtm` 的有符号 `units` 净额账本（runner.rs:306）。多腿（Long）+
/// 单位、空腿（Short）− 单位——**对冲腿部分抵消父仓**（ShortDiff 反向子声部腿的空单位抵消父多腿，
/// 对齐 M11 短差「父声部不动建反向子声部」的净额体现）。
///
/// ★分账本（毛）→ 净账本的语义降维（M29 §7 诚实声明）：分账本目标头寸 q̄_Θ 是**多空独立坐标**
/// （C25，毛收益级，G_net=0 双开净额退化）；Nautilus 净额账户只持**单一净持仓**——本函数把毛腿
/// 净额化（Σ ε_e·s_e）。净额化**丢失**双开的毛敞口信息（净杠杆 ≤ 毛杠杆，risk.rs `net_le_gross`）
/// ——这是 Nautilus 净额兼容的必然降维，**非** bug。完整毛分账本执行须 hedging 账户（v0 净额）。
pub fn net_target_units(legs: &[LegTarget]) -> f64 {
    legs.iter()
        .map(|leg| match leg.side {
            VoiceSide::Long => leg.units,
            VoiceSide::Short => -leg.units,
            VoiceSide::Flat => 0.0,
        })
        .sum()
}

/// **毛敞口 G = Σ|s_e|**（分账本毛账本，双开两腿都计入；对齐 risk.rs `gross_notional`）。
///
/// 各腿单位数绝对值之和——双开（多腿+空腿）的毛敞口高（净额 [`net_target_units`] 可低）。
/// 用于诊断净额降维丢失的毛敞口（净 ≤ 毛，element-coverage 的分账本毛收益级覆盖见证）。
pub fn gross_target_units(legs: &[LegTarget]) -> f64 {
    legs.iter().map(|leg| leg.units.abs()).sum()
}

// ════════════════════════════════════════════════════════════════════════════
//  §7 已删除（GAP-5 收口，no-patch）：旧 `coverage_step`（λ_e 走势边界 per-bar 入场五步合成）
//  曾自称"生产引擎核心入口"，但经 [`active_set_step`] 在 `B_t={e:λ_e=t}` 入场——λ_e=
//  `LeveledMove.start_index`=**走势边界**，正是 GAP-5 判定的错误入场源。spec §13 正典递归用
//  解释器三桶 𝒟_x/ℬ_x=ℛ_Θ(Γ(x))（**买卖点 Γ**），生产入场已收口至 §8 买卖点路径 + §9 π_Θ。
//  （框架纠偏 MEMORY coverage-engine-needs-tower-export-bridge：互斥全定义策略=**买卖点入场**+
//  多级角色/嵌套对冲，**非每元素覆盖**。删除 λ_e 入场组装层，保留 §3 区间递归原语作 Lean 对齐。）
// ════════════════════════════════════════════════════════════════════════════
//  §8 环6：解释器三桶 → 活动集 A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x] → 目标头寸 p̃_{t+1}
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
) -> HeldLegMatch {
    let tree_end = candidate_start.min(elements.len());
    let tree = &elements[..tree_end];
    let id_idx = build_tree_id_index(tree);
    held_leg_tree_index_indexed(tree, leg, &id_idx)
}

/// ponytail: H6 预建 `ElementId → idx` 索引（结构映射查表 O(1)，spec §13）。
/// 确定性 ID 跨 bar 稳定 ⟹ 全量/增量产同 ID ⟹ 同一走势跨 bar 命中同 idx（父延伸也同 ID）。
pub(crate) fn build_tree_id_index(
    tree: &[CoverageElement],
) -> std::collections::HashMap<ElementId, usize> {
    let mut idx: std::collections::HashMap<ElementId, usize> = std::collections::HashMap::new();
    for (i, e) in tree.iter().enumerate() {
        idx.entry(e.id).or_insert(i);
    }
    idx
}

/// ponytail: H6 带预建索引的 held_leg_tree_index 变体——热循环 coverage_step_from_buckets 单次建、多次查。
/// codex Q4：按 `leg.id` 查表（结构映射），删除 CoordDrift 分支（ID 确定性 ⟹ 无需 λ 稳定性 hack）。
fn held_leg_tree_index_indexed(
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
enum HeldLegMatch {
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
fn element_as_leg(e: &CoverageElement) -> ActiveLeg {
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
fn close_indices(prev_active: &[ActiveLeg], close: &[ActiveLeg]) -> Vec<usize> {
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
/// LiveDetached 腿的 op_parent 及其祖先（沿 structural_parent_id 链）若不在 raw 中，
/// 从 persistent registry 恢复加入 work/raw。这使 ancestor_close_by_id 通过（I5：
/// AncOK 作用 persistent active set）。
///
/// §11 归纳证明：每条未关闭腿的操作父 live ⟹ 所有 depth<d 腿通过 persistent AncOK。
/// 递归上溯 structural_parent_id 链，遇到已在 raw 中的祖先停止（闭包满足）。
/// ponytail: ceiling=增量 extract_elements 时 confirmed prefix 已含全部祖先，无需恢复。
fn restore_ancestor_chain_from_registry(
    work: &mut ElementView,
    raw: &mut Vec<usize>,
    registry: &super::persistent::PersistentRegistry,
    start_pid: super::classifier::recursive_tower::ElementId,
    // ★工位 4f：id→idx 查表（base 段缓存 + restore 动态 push 的 overlay 段累积），消 work.iter().position
    // O(work)/层=O(n²)。raw.any 不动（raw 有界，prev_active~O(log n) 实测 9@16K）。
    id_idx: &std::collections::HashMap<ElementId, usize>,
    overlay_seen: &mut std::collections::HashMap<ElementId, usize>,
) {
    let mut cur = Some(start_pid);
    while let Some(pid) = cur {
        // 已在 raw 中？⟹ 闭包满足，停止递归。
        let already_in_raw = raw.iter().any(|&r| work.get(r).map(|e| e.id == pid).unwrap_or(false));
        if already_in_raw {
            break;
        }
        // ★(I-1) 祖先若已在 work（树前缀 carrier / restore 已 push 的）但不在 raw，**复用现有 idx**入 raw
        // （不 push 重复 id，否则 strategy_target_legs 双计 p̃ 伪证）。查表 O(1)：先 base id_idx 再 overlay_seen。
        // bit-exact == 旧 work.iter().position：position 返首个匹配 idx，base 段在 overlay 前 ⟹ base 优先与
        // position 序一致；overlay_seen 用 or_insert 存首次 push idx ⟹ 与 position 在 overlay 段首个匹配一致。
        if let Some(&existing_idx) = id_idx.get(&pid).or_else(|| overlay_seen.get(&pid)) {
            raw.push(existing_idx);
            cur = work[existing_idx].parent_id; // 沿已有元素的结构父链上溯
            continue;
        }
        // 从 registry 取元素（work 中尚无 ⟹ 真 LiveDetached 祖先，须从持久身份恢复）。
        let pe = match registry.get(&pid) {
            Some(e) if !e.invalidated => e,
            _ => break, // registry 无效或已作废 ⟹ 停止（不再恢复祖先）
        };
        let parent_pid = pe.structural_parent_id;
        let op_idx = work.len();
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
        raw.push(op_idx);
        cur = parent_pid; // 上溯祖先链
    }
}

pub(crate) fn coverage_step_from_buckets(
    mut work: ElementView,
    prev_active: &[ActiveLeg],
    buckets: &Buckets,
    base_units: f64,
    config: &VoiceConfig,
    registry: &super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64) {
    // ★热点② O(n²) 消除：work = ElementView{base=持久树前缀借用零拷贝, overlay=本 bar candidate 段}。
    // 旧 `elements.to_vec()` + 上游 `tree.clone()` 每 bar O(tree)×n=O(n²) 双双消除（base 借 Rc 树，
    // candidate 在 overlay）。restore 路径继续 push overlay 尾（绝大多数 bar 不触发）。
    // candidate_start = base 段长（candidate 从此起，在 overlay）。
    let candidate_start = work.base_len();
    let mut raw: Vec<usize> = Vec::new();

    // ponytail: H6 单次建 tree 前缀 ElementId→idx 索引——tree 前缀在持仓腿对位期间不变
    //（Stale 追加在 candidate_start 之后，不污染 tree 前缀）⟹ 建一次、多次查 O(1)。
    // codex Q4：按 ElementId 结构映射查表（spec §13），替代旧 (level,ρ,eps)/(level,λ,eps) 值比较。
    // tree_prefix(tree_end) **借用 base**（candidate_start ≤ base.len()）⟹ build_tree_id_index 不 to_vec。
    let tree_end = candidate_start.min(work.len());
    // ★工位 4d 热点②：缓存命中复用 `Rc<id_idx>`（`Rc::clone` O(1)，§16 tree 前缀不变 ⟹ ElementId→idx
    // 不变；独立持有 ⟹ 不借 work，下游 restore push 可变借用 work 不冲突）；缺失（测试/无缓存路径）⟹
    // fallback 现建 O(tree)。tree_end==candidate_start==base_len ⟹ 缓存覆盖范围 == tree_prefix(tree_end)
    // == 完整 base，bit-exact 一致。
    let id_idx: Rc<std::collections::HashMap<ElementId, usize>> = match &work.base_id_idx {
        Some(rc) => Rc::clone(rc),
        None => Rc::new(build_tree_id_index(work.tree_prefix(tree_end))),
    };
    // ★工位 4f：overlay 段（candidate ++ restore push）id→idx，restore 复用查 O(1)（消 work.iter().position
    // O(work)/层=O(n²)）。初始 = candidate 段（base_len..work.len()，首次出现序）；restore push 时累积。
    // bit-exact == 旧 work.iter().position：查找 id_idx(base) 优先 → overlay_seen，与 position「base 段在前」
    // 序一致；or_insert 存首次 idx，与 position 首个匹配一致。
    let mut overlay_seen: std::collections::HashMap<ElementId, usize> =
        std::collections::HashMap::new();
    for i in candidate_start..work.len() {
        if let Some(e) = work.get(i) {
            overlay_seen.entry(e.id).or_insert(i);
        }
    }

    // (A_t ∖ 𝒟_x)：持仓腿（除 close 认领）按 ElementId 对位回当前因果树元素；不在树 ⟹ 按
    // is_boundary_root 决定 root/prune（发现 A 修复：Stale 不伪造 parent:None）。
    let closed = close_indices(prev_active, &buckets.close);
    for (i, leg) in prev_active.iter().enumerate() {
        if closed.contains(&i) {
            continue; // 𝒟_x：本腿关闭，不入 A^raw
        }
        // tree_end 固定 + Stale 追加在 overlay（tree_end 之后）⟹ base 前缀内容不变；每轮重借（NLL）。
        let m = held_leg_tree_index_indexed(work.tree_prefix(tree_end), leg, &id_idx);
        match m {
            // Exact（ID 命中=同一走势，父延伸也同 ID）：对位回当前树元素 idx（携真父链）⟹
            // 其子声部腿的 AncOK 祖先齐全。
            HeldLegMatch::Exact(idx) => {
                if !raw.contains(&idx) {
                    raw.push(idx);
                }
            }
            // Stale（snapshot 找不到）：★persistent overlay 修复（anc.pdf §8/§10）。
            // Stale(L) ⟺ pid(e)∉Pj or explicit invalidation（非 pid(e)∉Ej）。
            // e∉Ej 只是 snapshot_present(e)=0，不是 persistent_alive(e)=0（§8）。
            // 检查 persistent registry：LiveDetached 保留（op_parent 持久，I4+I5），Invalidated 才 prune。
            HeldLegMatch::Stale => {
                let held_state = registry.held_state(leg);
                match held_state {
                    super::persistent::HeldLegState::LivePresent => {
                        // 理论不可达（Exact 未命中但 registry LivePresent = snapshot 不一致）；
                        // 按持久身份保留（I1），op_parent 驱动 AncOK。
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
                        raw.push(idx);
                    }
                    super::persistent::HeldLegState::LiveDetached => {
                        // ★anc.pdf §10 核心修复：LiveDetached 不 prune，不伪造 root。
                        // parent 仍是 op_parent(L)（§15），只是当前 snapshot 没展示。
                        // op_parent 在 persistent registry 中 live（I4）→ AncOK 通过（I5）。
                        // ★I5 + §11 归纳：从 registry 递归恢复整条操作祖先链（op_parent 及其祖先），
                        // 全部加入 work/raw，使 ancestor_close_by_id 通过（§11：每条未关闭腿的操作父
                        // live ⟹ 所有 depth<d 腿通过 persistent AncOK）。
                        if let Some(op_pid) = leg.op_parent {
                            restore_ancestor_chain_from_registry(
                                &mut work, &mut raw, registry, op_pid, &id_idx, &mut overlay_seen,
                            );
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
                        raw.push(idx);
                    }
                    super::persistent::HeldLegState::Closed | super::persistent::HeldLegState::Invalidated => {
                        // 显式关闭/作废 → prune（§9 rule 5：只有 close/risk close/invalidation 才退出 live）。
                        if leg.is_boundary_root {
                            // 真边界根 ∂：作根保留（parent_id=None 合法，AncOK 不剔）。
                            let idx = work.len();
                            work.push(CoverageElement {
                                lambda: leg.lambda,
                                rho: leg.source_index,
                                eps: leg.dir,
                                level: leg.level,
                                parent: None,
                                attached_dir: None,
                                id: leg.id,
                                parent_id: None,
                            });
                            raw.push(idx);
                        }
                        // 非边界根且 invalidated/closed → prune（不入 raw，§9 rule 5）。
                    }
                }
            }
        }
    }

    // ∪ ℬ_x：开启候选 → 其 638 附着因果树元素索引（candidate_start + gamma_index）。
    //
    // ★工位 H 修复（级别容器.pdf §13）：开仓激活的位置节点身份 = carrier 容器 hostOf(g)（其 ElementId
    // 在候选元素 id 上携带，见 interp.rs `coverage_elements_and_gamma_with_tower`），**不是**买卖点叶子。
    // 故候选自身入 raw 即等价于「激活 carrier 上的位置节点」（PDF §14 简化 position instance）。子声部
    // 的 parent_id（= par_C(carrier)）由 ancestor_close_by_id 检查是否在 raw（= 持仓父位置节点在 A_t）。
    //
    // ★删除旧 G host 注入（级别容器.pdf §6/§7 + §13 否定）：旧 G 把 `(c.level, c.source_index)` 命中的
    // **同级 host 叶子**注入 raw（叶子作持仓 = a_carrier 仍 0），是 PDF §6/§7 反证的"开叶子"错形式，
    // 且对 depth>0 子无效（注入的是叶子自身非父 carrier）。位置节点身份改为 carrier id 后，候选 id=
    // carrier id 入 raw 作位置节点。
    //
    // ★(I-1) 修正（codex 行级坐实）：候选自身入 raw 只激活 **本级** carrier 位置节点；depth>0 **子声部**
    // 候选的准入还需其**父** carrier（= par_C(carrier)，候选 parent_id）在 raw（§13 AncOK 子声部不漂浮
    // 在不存在的父上）。父 carrier 几乎从不与子同 bar 共现/持仓（sd_parent_held=0），仅 registry
    // LiveDetached 存活——故 open 候选自身入 raw **不足以**让父在场。下方补 open 候选父链注入（no-patch：
    // 缺失逻辑补全，非 AncOK 加特例）。
    for c in &buckets.open {
        let idx = candidate_start + c.gamma_index;
        if idx < work.len() && !raw.contains(&idx) {
            raw.push(idx);
        }
        // ★(I-1) open 候选父注入（codex 异质审查行级坐实，642/644）：
        //
        // depth>0 子声部候选（ShortDiff/FollowParent）的真 Compose 父 carrier（= par_C(carrier)，候选
        // 元素 parent_id 携带）几乎从不与子同 bar 作 open 候选，也几乎从不是 prev_active 持仓腿
        // （L2 诊断：sd_parent_held=0），但**在 persistent registry 中 LiveDetached 存活**
        // （sd_parent_registry_alive≈sd_total）。Stale 持仓腿路径（上方 LiveDetached 分支）已用
        // [`restore_ancestor_chain_from_registry`] 把 op_parent 祖先链注入 raw，但 open 候选路径
        // **从不触发**该恢复 ⟹ 父 carrier 不在 raw ⟹ ancestor_close_by_id 判子声部祖先不齐 ⟹ AncOK
        // 全剪 depth>0 子腿（accepted_cert_carrier=0）。
        //
        // 修复：把 Stale 路径的祖先链恢复机制**扩展到 open 路径**——对每个 open 候选的父 carrier
        // （work[idx].parent_id），若 registry live 且不在 raw，从 registry 递归恢复整条结构祖先链
        // （§11 归纳：每条子声部腿的操作父 live ⟹ depth<d 祖先全在 raw ⟹ AncOK 通过）。这让持仓父
        // carrier 的位置节点进入 A_t（§8 父声部 carrier 跨 bar 持有，§9 祖先闭合兑现），depth>0 子腿
        // 准入。**非** AncOK 加特例放行——父链真实注入后由原 ancestor_close_by_id 正常判定（no-patch）。
        if idx < work.len() {
            if let Some(parent_pid) = work[idx].parent_id {
                let parent_in_raw =
                    raw.iter().any(|&r| work.get(r).map(|e| e.id == parent_pid).unwrap_or(false));
                if !parent_in_raw && registry.registry_live(&parent_pid) {
                    restore_ancestor_chain_from_registry(&mut work, &mut raw, registry, parent_pid, &id_idx, &mut overlay_seen);
                }
            }
        }
    }

    // 步2：A_{t+1}=AncOK(A^raw)——剔除真 Compose 父容器不在 raw 的孤儿子腿（§13 持仓准入：未持父则剔除）。
    // ★codex Q4：按 parent_id 结构映射闭包（spec §13 `p:C_ℓ→C_{ℓ+1}`），非 per-bar 索引链。
    let next_idx = ancestor_close_by_id(&work, &raw);

    // p̃=Σ Leg(g)（depth 权重沿真父链 + 方向净额聚合，ShortDiff 空腿部分对冲父多腿）。
    let legs = strategy_target_legs(&work, &next_idx, base_units, config);
    let p_tilde = net_target_units(&legs);

    // A_{t+1} 回 ActiveLeg（638 身份，喂下一 bar interpret 闭环 + 跨 bar 对位）。
    let next_active: Vec<ActiveLeg> = next_idx.iter().map(|&i| element_as_leg(&work[i])).collect();
    // ★(I-1) 双计守卫（codex 异质审查）：next_active 每 ElementId 必唯一——同 carrier 不得在 raw 中以
    // 两个 idx（树前缀 + registry 追加）出现，否则 strategy_target_legs 双计 ⟹ p̃ 伪证。
    // restore_ancestor_chain_from_registry 已复用现有 idx 保证唯一；此 assert 锁不变量防回归。
    debug_assert!(
        {
            let mut ids: Vec<_> = next_active.iter().map(|l| l.id).collect();
            ids.sort_by_key(|id| (id.level, id.ordinal));
            ids.windows(2).all(|w| w[0] != w[1])
        },
        "next_active 含重复 ElementId ⟹ strategy_target_legs 双计 p̃（restore 未复用现有 idx）"
    );
    (next_active, p_tilde)
}

/// 环5+环6 端到端（`Classification` + per-bar 因果塔 → Γ → 三桶 → `A_{t+1}=AncOK[...]` → `p̃`），
/// element-coverage 生产入口（**σ_p 来源 + §13 AncOK 持仓准入双机制就位**）。
///
/// 串接：
/// 1. **真父子组合元素** `(elements, candidate_start)`=[`interp::coverage_elements_with_tower`]
///    （tree ++ 638 附着候选；候选携真 `parent`=Compose 父、`attached_dir`=σ_p=**父容器方向**，639）。
/// 2. **环5**：`gamma`=[`interp::assemble_gamma_with_tower`]（角色 V 由真父派生：FollowParent/ShortDiff/
///    Ambient）+ [`interp::interpret`] ℛ_Θ 唯一化三桶（含反向关闭 𝒟_x，喂 `prev_active`）。
/// 3. **环6**：[`coverage_step_from_buckets`] 活动集递归 + §13 AncOK 持仓准入（未持父则剔除 ShortDiff）。
///
/// **★两机制正交（639）**：① **σ_p 来源**（§7.2，`assemble_gamma_with_tower` 经 `attach_bsp_to_tree`
/// 从 per-bar 因果塔查父容器方向，**与持仓无关**）；② **持仓准入**（§13 AncOK，`coverage_step_from_buckets`
/// 经 `prev_active` 对位真树元素判父容器腿是否持有）。`prev_active` **进** interpret（𝒟_x 反向关闭）
/// **与** AncOK 准入（父容器腿在场判据），**不进** σ_p 计算（错口径"活动父腿"已删，639）。
///
/// **★执行层因果塔**：调用方（runner）须喂 **per-bar 前缀因果塔**（`classify_with_tower(l0[0..=t])`，
/// 只用 ≤t 数据 → 因果）；全窗塔非因果，执行层禁用（639）。有向父容器 ⟹ V 真出 FollowParent/ShortDiff；
/// 父=胚元∂（缺塔/host 是根）⟹ σ_p=0 ⟹ Ambient。
///
/// **不变量契约**：[`interp::coverage_elements_with_tower`] 与 [`interp::assemble_gamma_with_tower`]
/// 在同一 `(classification, tower)` 上**确定重建同一树**（同 `levels×bsp` 序），故候选 `gamma_index`
/// 与 `elements` 候选段偏移 1:1 对齐（`elements[candidate_start + gamma_index]` 即该候选元素）。
///
/// **边界条件**：`tower.len()<2`（仅 L0，无 Compose 父）⟹ 所有候选 σ_p=0 ⟹ 全 Ambient 根 ⟹ AncOK
/// 恒等准入（与扁平 [`interp::assemble_gamma`] 一致，tower-export-i 边界）。空 `levels`/空 bsp ⟹ 空 Γ。
/// **认识论 L1**（管线正确性，非 L2 alpha）。
pub fn coverage_step_classification(
    classification: &Classification,
    tower: &[Rc<Vec<LeveledMove>>],
    prev_active: &[ActiveLeg],
    base_units: f64,
    config: &VoiceConfig,
    registry: &super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64) {
    // H2 优化：单次建树产 (elements, candidate_start) + gamma——消除旧版每 bar 双调
    // extract_elements(tower) 的冗余（coverage_elements_with_tower + assemble_gamma_with_tower
    // 各建树一次，第二次纯重复）。bit-exact：同 (classification, tower) 同一建树输出。
    let (tree, candidates, gamma) =
        interp::coverage_elements_and_gamma_with_tower(classification, tower);
    let work = ElementView::from_parts(&tree, candidates);
    coverage_step_prebuilt(work, &gamma, prev_active, base_units, config, registry)
}

/// **工位 K 性能：环5+环6 用预建 `(elements, candidate_start, gamma)`**（消除 runner per-bar
/// 双调 `coverage_elements_and_gamma_with_tower`——一次 `pi_theta_step` + 一次 merge 的 elements，
/// 现共享同一预建产物）。bit-exact == [`coverage_step_classification`]：同 elements/gamma 同 interpret
/// 同 AncOK。**不改 AncOK 准入逻辑**（[`coverage_step_from_buckets`] 原样），仅消除重复建树。
pub(crate) fn coverage_step_prebuilt(
    work: ElementView,
    gamma: &[Candidate],
    prev_active: &[ActiveLeg],
    base_units: f64,
    config: &VoiceConfig,
    registry: &super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64) {
    // 环5：解释器三桶（𝒟_x 反向关闭喂 prev_active）。
    let buckets = interp::interpret(gamma, prev_active);
    // 环6：A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x] + p̃（§13 持仓准入：ShortDiff 未持父则剔除，639(c)）。
    coverage_step_from_buckets(work, prev_active, &buckets, base_units, config, registry)
}

// ════════════════════════════════════════════════════════════════════════════
//  §9 环7：目标头寸 p̃_{t+1} → 全定义策略 π_Θ → 唯一订单 O_{t+1}
//        （spec §15 P12 line 717-756：𝒦_Θ / J_x / LexArgmin / Schedule_Θ；
//         定理 spec §16 P13 line 795：∀x ∃! O_{t+1}=π_Θ(x)，七链 **环7** rust 兑现）
//
//  π_Θ(x) = Schedule_Θ[ LexArgmin_{p∈𝒦_Θ(x)} J_x(p) − p_t ]   （spec line 756 方框）
// ════════════════════════════════════════════════════════════════════════════

/// J_x 目标函数权重（spec §15 line 740：`J_x = ‖p−p̃‖²_W + λ·Cost_x(p) + ν·RiskPenalty_x(p)`）。
///
/// 三权重全是 **Θ_risk 参数**（formalization-validity-domain，**非缠论可导**）：
/// - `w`：加权范数权重 w_a（spec line 744 `w_a>0` ⟹ 范数正定 ⟹ 主键跟踪误差严格凸）。
/// - `lambda`：成本倍数 λ（**复用** [`RiskConfig`] 的 `kappa` 成本倍数 κ，不引入新参数）。
/// - `nu`：风险罚倍数 ν（**复用** [`RiskConfig`] 的 `rho` 单声部风险 ρ，不引入新参数）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PiThetaWeights {
    /// w_a > 0：加权范数权重（主键跟踪误差，spec line 744）。
    pub w: f64,
    /// λ：成本倍数（次键，复用 RiskConfig.kappa）。
    pub lambda: f64,
    /// ν：风险罚倍数（第三键，复用 RiskConfig.rho）。
    pub nu: f64,
}

impl PiThetaWeights {
    /// 从 [`RiskConfig`] 派生 J_x 权重（**复用**既有 Θ_risk 参数，no-声明膨胀不引新参数）。
    ///
    /// `w=1.0`（范数主键权重，满足 spec line 744 `w_a>0` 正定要求）；`λ=κ`（`RiskConfig.kappa`
    /// 成本倍数）；`ν=ρ`（`RiskConfig.rho` 单声部风险）。
    pub fn from_risk(risk: &RiskConfig) -> Self {
        PiThetaWeights { w: 1.0, lambda: risk.kappa, nu: risk.rho }
    }
}

/// J_x 定点放大因子（浮点 J_x 分量 → [`JThetaKey`] i64 字典序键，bit-exact 确定比较）。
///
/// 对齐 intent.rs `JThetaKey` 文档「浮点 J_Θ 值乘固定缩放因子后取整，避免浮点比较的非确定性」。
const J_SCALE: f64 = 1_000.0;

/// 净持仓 flat（零仓）判定阈（lot 对齐下 |p|<此阈即视为空仓）。
const FLAT_EPS: f64 = 1e-9;

/// 定点放大 + 溢出钳制（浮点 J_x 分量 → i64 字典序键，**无 panic**）。
///
/// `(x·J_SCALE).round()` 钳到 i64 值域（边界条件：p 有界于 ±cap、权重有限 ⟹ 常规配置不触钳制；
/// 极端 base_units 触上界时钳到 i64::MAX，保字典序方向不翻转，非 bug）。
fn scale_key(x: f64) -> i64 {
    (x * J_SCALE).round().clamp(i64::MIN as f64, i64::MAX as f64) as i64
}

/// 净持仓 lot 对齐（向最近 lot 取整；`lot≥1` 由调用方 `RiskConfig.default_lot.max(1)` 保证）。
fn lot_round(p: f64, lot: f64) -> f64 {
    (p / lot).round() * lot
}

/// 𝒦_Θ 净持仓可行幅度**无量纲**上限 `γ̄`（**方案A协变**，spec §3/§5/§6 钦定，CovariantCapital.lean GREEN）。
///
/// 返回 `risk.gamma.abs()`（= `γ̄`，无量纲比例上限，`b_j(S_k Θ) = a_k^{d_j} b_j(Θ)` 边界协变，
/// d_j=1 名义上限）。绝对上限由调用方 [`pi_theta_position`] 完成：`cap = U_ℓ · γ̄`
/// （`U_ℓ = base_units`，runner 注入的协变资本单位，满足 `U_{ℓ+k}(S_k x) = a_k U_ℓ(x)`）。
///
/// **方案A落地**（2026-06-28-absolute-capital-equivariance-resolution §6 钦定）：
/// `γ̄` 是无量纲 Θ 参数（`S_k Θ = Θ` 强形式，不含特权绝对尺度）；约束 `|p| ≤ U_ℓ · γ̄` 随
/// `a_k` 协变缩放（d_j=1 名义上限）——绝对资本特权尺度已消除，三阶段资本比例化完成。
///
/// **d_j 默认（对齐 CovariantCapital.lean §6）**：名义上限 d_j=1（本函数）；杠杆比 d_j=0
/// （[`risk::leverage_ok`]，无量纲比值天然满足）；跟踪误差 `w·(p−p̃)²` d_j=2（[`j_theta_key`]）。
///
/// **★诚实有效域**：美元级杠杆/保证金（[`risk::leverage_ok`]）仍需 runner 注入 price/equity。
/// 实盘 Nautilus 路径：`a_t = U_ℓ · ā_t`（`ā_t = p*`，`U_ℓ = base_units`，runner 层还原绝对值）。
fn feasible_net_cap(risk: &RiskConfig) -> f64 {
    risk.gamma.abs()
}

/// **𝒦_Θ 风控约束门（close_pred 折入可行集，非第二决策出口，§16 单一决策出口）**。
///
/// ## Q2 编排者裁定（close_pred 风控折进 𝒦_Θ）
///
/// reference §16 钦定**单一决策出口** `π_Θ(x)=Schedule_Θ[LexArgmin_{p∈𝒦_Θ}J_x(p)−p_t]`。退出
/// 不能有第二出口——故 close_pred 的**风控项（stop/risk）折入 𝒦_Θ 约束门**：风控触发 ⟹ 𝒦_Θ
/// 可行净持仓收窄（`force_flat`→{0}；`stop_long`→禁净多仓；`stop_short`→禁净空仓），[`lex_argmin`]
/// 在收窄集上自然产 p*→平/减——退出仍走**唯一决策出口**（p*），非独立 exit 订单。
///
/// ## close_pred 契约锚保留（no-patch-keep-primitive）
///
/// 本门由 [`super::exec::close_pred`] 计算（runner discharge stop/risk 读出 → close_pred → 本门），
/// **不删** close_pred 原语。`reverse_signal` 项**不**入本门——反向信号关闭活动腿走 [`interp::interpret`]
/// 的 𝒟_x（活动集递归腿级决策，那才是 §16 的腿级单出口）；本门只承载 stop/risk（账户/价格层风控）。
///
/// ## 认识论 L0（formalization-validity-domain 231号）
/// 给定风控读出后，约束门是 𝒦_Θ 区间收窄的布尔代数（确定）。风控读出本身（`stop_hit`/
/// `global_risk_close`）由 runner discharge（账户层运行时输入 E_t/价格触及，**非缠论可导**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KThetaRiskGate {
    /// GlobalRiskClose（Insolvent/Liquidation，P1 最高优先级）⟹ 𝒦_Θ={0}（强制全平）。
    pub force_flat: bool,
    /// 多头风控触发（结构止损触及，`close_pred(stop_long)`）⟹ 𝒦_Θ 禁净多仓（hi_cap=0）。
    pub stop_long: bool,
    /// 空头风控触发（结构止损触及，`close_pred(stop_short)`）⟹ 𝒦_Θ 禁净空仓（lo_cap=0）。
    pub stop_short: bool,
}

impl KThetaRiskGate {
    /// 无约束门（𝒦_Θ=[−cap,+cap] 全开，风控未触发）——执行层默认 + 既有 π_Θ 测试用。
    pub fn open() -> Self {
        KThetaRiskGate { force_flat: false, stop_long: false, stop_short: false }
    }

    /// 应用约束门到对称 cap，产 `(lo_cap, hi_cap)` 幅度（`force_flat` 优先收到 {0}）。
    fn caps(&self, cap: f64) -> (f64, f64) {
        if self.force_flat {
            return (0.0, 0.0); // GlobalRiskClose ⟹ 𝒦_Θ={0}（净持仓只能 0）
        }
        let hi = if self.stop_long { 0.0 } else { cap }; // 禁净多 ⟹ 上限 0
        let lo = if self.stop_short { 0.0 } else { cap }; // 禁净空 ⟹ 下限 0
        (lo, hi)
    }
}

/// 构造有限可行集 𝒦_Θ(x) 的 **LexArgmin 代表点**（spec §15 line 721-736 八约束 + line 725 `𝒦_Θ≠∅`）。
///
/// 𝒦_Θ = lot 对齐的净持仓 `p ∈ [−cap,+cap]`（有限网格，spec P13 假设10「𝒦_Θ 非空且有限」）。返回
/// **字典序 argmin 必含的代表点**（凸跟踪目标在 lot 网格的精确有限表示，**非近似**）：
/// - `clamp(p̃)` 的 floor/ceil lot 点（**bracketing**：凸二次跟踪项 `w(p−p̃)²` 在 lot 网格的全局
///   最小点必是 `clamp(p̃)` 的两个相邻 lot 点之一）。
/// - 端点 `±hi`（杠杆/资本 cap binding 时的最优；`hi`=最大 lot 对齐幅度 `≤cap`）。
/// - 安全锚 `0`（**𝒦_Θ≠∅ 的构造性非空见证**，spec line 725 硬前提 / Lean `feasible_nonempty`）
///   + `clamp(lot(p_t))`（保持当前仓位的可行点）。
///
/// 全 lot 对齐 ∈[−hi,hi]，升序去重，`grid_index`=升序位次（**固定字典序平局规则**，spec line 791
/// 假设11 ⟹ [`JThetaKey`] 单射 ⟹ p* 唯一）。
///
/// ★凸性精确性（formalization-validity-domain，**非近似**）：主键跟踪误差 `w(p−p̃)²`（w>0）在 lot
/// 离散区间的全局最小在 `clamp(p̃)` 相邻 lot 点取得；二点等距（p̃ 恰在 lot 中点）⟹ 跟踪并列 ⟹ 次键
/// 成本破并列——两点均在本集 ⟹ **本代表集上 LexArgmin = 全 𝒦_Θ 网格 LexArgmin（精确相等）**。
fn feasible_candidates(p_tilde: f64, p_t: f64, lo_cap: f64, hi_cap: f64, lot: f64) -> Vec<(f64, i64)> {
    // 非对称 cap（𝒦_Θ 风控约束门 [`KThetaRiskGate::caps`] 注入）：hi_cap=净多上限、lo_cap=净空上限
    // （幅度）。对称全开时 lo_cap=hi_cap=cap（退化为旧 [−hi,hi]）；force_flat ⟹ 两者 0 ⟹ 𝒦_Θ={0}。
    let hi = ((hi_cap / lot).floor() * lot).max(0.0); // 净多最大 lot 对齐幅度 ≤ hi_cap
    let lo = -((lo_cap / lot).floor() * lot).max(0.0); // 净空最大 lot 对齐幅度 ≤ lo_cap
    let clamp = |x: f64| x.max(lo).min(hi);
    let pc = clamp(p_tilde);
    let floor_pt = clamp((pc / lot).floor() * lot);
    let ceil_pt = clamp((pc / lot).ceil() * lot);
    let anchor_pt = clamp(lot_round(p_t, lot));
    let mut pts = vec![floor_pt, ceil_pt, hi, lo, 0.0, anchor_pt];
    pts.sort_by(|a, b| a.partial_cmp(b).expect("有限 f64 候选可序"));
    pts.dedup_by(|a, b| (*a - *b).abs() < lot * 0.5); // 同 lot 点去重（保升序首个）
    pts.into_iter()
        .enumerate()
        .map(|(i, p)| (p, i as i64))
        .collect()
}

/// J_x(p) 的字典序键 [`JThetaKey`]（spec §15 line 740-744 三项 → 主/次/三键 + 平局键）。
///
/// - `tracking_err` = `w·(p−p̃)²`（‖p−p̃‖²_W 净额降维，**主键**——优先贴合目标头寸 p̃）。
/// - `trade_cost` = `λ·|p−p_t|`（Cost_x(p) 换手成本代理，**次键**）。
/// - `risk_penalty` = `ν·|p|`（RiskPenalty_x(p) 毛敞口 |p| 代理，**第三键**）。
/// - `turnover` = 0（spec §15 J_x 仅三项无独立换手项；[`JThetaKey`] 第四键留 0，换手已并入成本）。
/// - `grid_index`（升序位次，**固定平局键**，spec line 791 假设11）。
fn j_theta_key(
    p: f64,
    p_tilde: f64,
    p_t: f64,
    weights: PiThetaWeights,
    grid_index: i64,
) -> JThetaKey {
    JThetaKey {
        tracking_err: scale_key(weights.w * (p - p_tilde).powi(2)),
        trade_cost: scale_key(weights.lambda * (p - p_t).abs()),
        risk_penalty: scale_key(weights.nu * p.abs()),
        turnover: 0,
        grid_index,
    }
}

/// **最优头寸 p\* = LexArgmin_{p∈𝒦_Θ(x)} J_x(p)**（spec §15 line 748 方框）。
///
/// 在有限可行代表集 [`feasible_candidates`]（𝒦_Θ 的精确 LexArgmin 表示）上，用 **真字典序**
/// [`lex_argmin`]（intent.rs，对齐 Lean `Origin.LexArgmin`）选 J_x 字典序最小净持仓。
///
/// 边界条件：𝒦_Θ 恒含安全锚 0（非空，spec line 725）⟹ [`lex_argmin`] 必返回 `Some`；防御性 `None`
/// （理论不可达）归 0.0（flat 安全侧）。p̃ 在 cap 内 ⟹ p\*=lot(p̃)（跟踪主键）；p̃ 超 cap ⟹ p\*=±hi
/// （杠杆/资本 cap binding）。
pub fn pi_theta_position(
    p_tilde: f64,
    p_t: f64,
    base_units: f64, // U_ℓ：runner 注入的协变资本单位（随级别 a_k 缩放；方案A）
    risk: &RiskConfig,
    weights: PiThetaWeights,
    gate: KThetaRiskGate, // 𝒦_Θ 风控约束门（close_pred 折入，非第二出口；全开=open()）
) -> f64 {
    let lot = risk.default_lot.max(1) as f64;
    // 方案A：γ̄ = feasible_net_cap(risk) 无量纲；绝对上限 cap = U_ℓ · γ̄（d_j=1 协变缩放）
    let cap = feasible_net_cap(risk) * base_units.abs();
    // 𝒦_Θ 风控约束门：force_flat→{0}，stop_long→禁净多，stop_short→禁净空（Q2 折入可行集）。
    let (lo_cap, hi_cap) = gate.caps(cap);
    let candidates: Vec<LexCandidate<f64>> = feasible_candidates(p_tilde, p_t, lo_cap, hi_cap, lot)
        .into_iter()
        .map(|(p, gi)| LexCandidate {
            control: p,
            key: j_theta_key(p, p_tilde, p_t, weights, gi),
        })
        .collect();
    lex_argmin(&candidates).unwrap_or(0.0)
}

/// **Schedule_Θ(p\* − p_t) → 唯一订单 O_{t+1}**（spec §15 line 752 方框；八约束之 **订单执行约束**）。
///
/// 排程净持仓增量 `Δ = p\* − p_t`（lot 对齐）为单净额订单（Nautilus 净额账户单持仓）。这是**全函数**
/// （spec §16 假设12「Schedule_Θ 是函数」）——无交易时返回 `Hold`/`Wait`（qty=0），保 ∀x ∃! O_{t+1}：
/// - `Δ=0`：`Wait`（当前空仓）/ `Hold`（当前持仓），qty=0（`qty≤0` 不交易，types.rs Order 契约）。
/// - 空仓→持仓：`Buy`（p\*>0）/ `Sell`（p\*<0），qty=|p\*|。
/// - 持仓→空仓：`Close`，qty=|p_t|。
/// - 同号增持：`Add`；同号减持：`Reduce`，qty=|Δ|。
/// - 反号穿零（净反转）：`Buy`（p\*>0）/ `Sell`（p\*<0），qty=|Δ|（单净订单跨零）。
///
/// `exec_index`：执行延迟后的成交 bar（runner 传入，对齐 types.rs `Order.exec_index` / spec exec 延迟）。
pub fn schedule_order(p_star: f64, p_t: f64, exec_index: usize) -> Order {
    let delta = p_star - p_t;
    let qty = delta.abs().round() as i64;
    let flat_now = p_t.abs() < FLAT_EPS;
    let flat_next = p_star.abs() < FLAT_EPS;
    let action = if qty == 0 {
        if flat_now {
            StrictAction::Wait
        } else {
            StrictAction::Hold
        }
    } else if flat_now {
        if p_star > 0.0 {
            StrictAction::Buy
        } else {
            StrictAction::Sell
        }
    } else if flat_next {
        StrictAction::Close
    } else if (p_t > 0.0) == (p_star > 0.0) {
        // 同号：幅度增=Add，幅度减=Reduce。
        if p_star.abs() > p_t.abs() {
            StrictAction::Add
        } else {
            StrictAction::Reduce
        }
    } else {
        // 反号穿零（净反转）：方向由 p* 符号定。
        if p_star > 0.0 {
            StrictAction::Buy
        } else {
            StrictAction::Sell
        }
    };
    Order { action, qty, exec_index }
}

/// **全链 π_Θ(x)：买卖点 Γ 入场（环5+6） → 全定义策略 π_Θ（环7） → 唯一订单 O_{t+1}**。
///
/// 串通七链终段（生产入场入口，GAP-5 收口）：
/// 1. **环5+6**（[`coverage_step_classification`]）：`Classification → assemble_gamma(BspPoint) →
///    interpret(ℛ_Θ) → A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x] → p̃_{t+1}`。入场源 = **买卖点 Γ(x)**
///    （候选 `source_index` = `BspPoint.source_index`），**非走势边界** `LeveledMove.start_index`。
/// 2. **环7**（[`pi_theta_position`] + [`schedule_order`]）：`p* = LexArgmin_{p∈𝒦_Θ} J_x(p)` →
///    `O_{t+1} = Schedule_Θ(p*−p_t)`。
///
/// 返回 `(A_{t+1}, p*, O_{t+1})`：新活动集（喂下一 bar interpret 闭环）+ 新净持仓（喂下一 bar 的
/// `p_t`）+ 当前订单。**immutable**：不 mutate 输入。`base_units`=NAV 预算的基准腿仓位（runner 提供）。
///
/// ## 认识论等级（formalization-validity-domain 231号）
/// **L0/L1**（操作语义结构，**非 L2 alpha**）：确定性结构变换（Γ→桶→活动集→p̃→LexArgmin→订单），
/// `cargo test` 通过 = 管线正确性 + π_Θ 唯一订单（∀x ∃! O，spec §16）的**结构**兑现，**不**蕴含实盘
/// alpha（买卖点 v1 全窗 8/8 L3 已否证；本引擎是 M29 element-coverage 的 rust 兑现，盈利由下一步
/// L2/L3 净额回测否证检验）。
///
/// > **结果包六要素**
/// > - **结论**：全定义策略 π_Θ 的 rust 终环——买卖点 Γ 入场 → p̃ → `p*=LexArgmin_{p∈𝒦_Θ}J_x(p)`
/// >   → `O=Schedule_Θ(p*−p_t)`，产唯一订单 `(A_{t+1}, p*, O)`。
/// > - **定义依据**：spec §15 P12（𝒦_Θ⊆𝒫^sep、`𝒦_Θ≠∅`、八约束、J_x 方框 line 740、LexArgmin
/// >   方框 line 748、Schedule_Θ 方框 line 752、π_Θ 方框 line 756）+ §16 P13（∀x ∃! O_{t+1} line 795，
/// >   假设10 𝒦_Θ 非空有限、假设11 LexArgmin 固定平局、假设12 Schedule_Θ 是函数）。输入特征满足：
/// >   p̃ 来自环5+6 唯一活动集（𝒟_x/ℬ_x=ℛ_Θ(Γ(x)) 买卖点路径）；𝒦_Θ 恒含 0（非空）；JThetaKey
/// >   grid_index 单射（破平局）⟹ p* 唯一；Schedule_Θ 全函数 ⟹ O_{t+1} 唯一存在。
/// > - **边界条件**：① `𝒦_Θ≠∅` 由安全锚 0 构造性保证——若 cap<lot 则 `hi=0`，𝒦_Θ={0} 仍非空。
/// >   ② 唯一性依赖 **LexArgmin**（字典序，非普通 argmin）+ grid_index 单射——若仅 argmin 且 J_x
/// >   多最优则唯一性翻转（spec line 761）。③ p̃ 在 cap 内 ⟹ p*=lot(p̃)；超 cap ⟹ p*=±hi（杠杆/
/// >   资本 cap binding 翻转结论）。④ 八约束中 **AncOK/同单位短差/级别自相似/分账本** 在环5+6 上游
/// >   已施于 p̃（𝒦_Θ 经 p̃ 继承，不重复机件）；**手数/订单执行** 在本环施（lot 网格 + Schedule_Θ）；
/// >   **杠杆保证金** 仍需 runner price/equity（[需 runner 注入]）；**三阶段资本** 已方案A协变
/// >   （`cap=U_ℓ·γ̄`，base_units=U_ℓ，runner 按级别注入，d_j=1 名义上限，无特权绝对尺度）。
/// >   ⑤ 净额降维：p 是有符号净持仓（分账本多空腿 q^± 在 net_target_units
/// >   已降维，毛分账本须 hedging 账户，v0 净额，M29 §7 诚实声明）。
/// > - **下游推论**：`(A_{t+1}, p*)` 喂下一 bar（p* 成为下一 `p_t`，A_{t+1} 喂 interpret 闭环）；
/// >   O_{t+1} 喂 runner 净额账本（接 Nautilus 入场/出场点）。GAP-5 收口 ⟹ recognize↔coverage 入场
/// >   源统一为买卖点 Γ（trades-vs-closedloop-disjoint-paths 谱系：入场源接 Γ 连通两路径）。
/// > - **谱系引用**：GAP-5（入场源=买卖点 Γ 非走势边界，本环收口）；MEMORY
/// >   coverage-engine-needs-tower-export-bridge（互斥全定义策略=**买卖点入场**+多级角色/嵌套对冲，
/// >   非每元素覆盖——§7 λ_e 入场已删）；newchanlun-v1-fullwindow-l3-falsified（v1 8/8 否证，本环
/// >   不蕴含 alpha，L0/L1）；no-patch-keep-primitive（删 λ_e 入场组装层 §7，保 §3 区间递归原语）。
/// > - **影响声明**：新增 coverage.rs §9（[`PiThetaWeights`]/[`feasible_net_cap`]/
/// >   [`feasible_candidates`]/[`j_theta_key`]/[`pi_theta_position`]/[`schedule_order`]/本函数）；
/// >   复用 intent.rs（[`JThetaKey`]/[`LexCandidate`]/[`lex_argmin`]）+ RiskConfig（κ/ρ/γ/lot）+
/// >   types.rs（Order/StrictAction）；删除 §7 `coverage_step`（λ_e 走势边界入场，GAP-5）+ 其 3 测试；
/// >   不改 interp.rs/risk.rs/mod.rs/lakefile，coverage.rs 已注册（`pub mod coverage;` mod.rs:43）。
/// >   **方案A实装（本工位）**：`feasible_net_cap` 改为返回无量纲 `γ̄`（`risk.gamma`），
/// >   `pi_theta_position` 计算 `cap = U_ℓ·γ̄`（`U_ℓ=base_units`）——绝对资本协变化完成，
/// >   `[方案A-rust-todo]` 清零；信源：CovariantCapital.lean GREEN（feasibleSet_equivariant 全证）。
#[allow(clippy::too_many_arguments)]
pub fn pi_theta_step(
    classification: &Classification,
    tower: &[Rc<Vec<LeveledMove>>], // per-bar 因果塔（639 σ_p=父容器方向；runner 喂前缀重分类塔）
    prev_active: &[ActiveLeg],
    p_t: f64,
    exec_index: usize,
    base_units: f64,
    voice: &VoiceConfig,
    risk: &RiskConfig,
    weights: PiThetaWeights,
    gate: KThetaRiskGate, // 𝒦_Θ 风控约束门（close_pred 折入；全开=open()）
    registry: &super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64, Order) {
    // 环5+6：买卖点 Γ 入场 → A_{t+1} + p̃（GAP-5：入场源 = BspPoint.source_index 买卖点）。
    // 执行层 σ_p=父容器方向（639；coverage_step_classification 内 assemble_gamma_with_tower 喂因果塔）。
    let (next_active, p_tilde) =
        coverage_step_classification(classification, tower, prev_active, base_units, voice, registry);
    // 环7：p* = LexArgmin J_x（𝒦_Θ，风控门收窄）→ O = Schedule_Θ(p*−p_t)（单一决策出口 §16）。
    let p_star = pi_theta_position(p_tilde, p_t, base_units, risk, weights, gate);
    let order = schedule_order(p_star, p_t, exec_index);
    (next_active, p_star, order)
}

/// **工位 K 性能：`pi_theta_step` 用预建 `(elements, candidate_start, gamma)`**（bit-exact ==
/// [`pi_theta_step`]，仅把内部 `coverage_elements_and_gamma_with_tower` 重建替换为 runner 缓存的
/// 预建产物——消除 per-bar 双调建树 + 跨 bar 全前缀重建 O(confirmed)）。
#[allow(clippy::too_many_arguments)]
pub(crate) fn pi_theta_step_prebuilt(
    work: ElementView,
    gamma: &[Candidate],
    prev_active: &[ActiveLeg],
    p_t: f64,
    exec_index: usize,
    base_units: f64,
    risk: &RiskConfig,
    weights: PiThetaWeights,
    gate: KThetaRiskGate,
    config: &VoiceConfig,
    registry: &super::persistent::PersistentRegistry,
) -> (Vec<ActiveLeg>, f64, Order) {
    let (next_active, p_tilde) =
        coverage_step_prebuilt(work, gamma, prev_active, base_units, config, registry);
    let p_star = pi_theta_position(p_tilde, p_t, base_units, risk, weights, gate);
    let order = schedule_order(p_star, p_t, exec_index);
    (next_active, p_star, order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::classifier::recursive_tower::LeveledMove;
    use super::super::super::classifier::center::UnitRange;
    use super::super::super::types::{Center, Direction, Tick};

    fn cfg() -> VoiceConfig {
        VoiceConfig::default() // max_depth=3, depth_weights=[0.60,0.30,0.10]
    }

    /// ★O(n) 重构测试适配：把 `Vec<Vec<LeveledMove>>` 字面量塔逐级包 `Rc`（生产塔现为
    /// `Vec<Rc<Vec<LeveledMove>>>`）。bit-exact 无关——仅类型适配，Rc deref 后内容不变。
    fn rc_tower(levels: Vec<Vec<LeveledMove>>) -> Vec<Rc<Vec<LeveledMove>>> {
        levels.into_iter().map(Rc::new).collect()
    }

    /// 测试用 18 类角色构造器（三轴元组）。
    fn role(h: Horizontal, v: Vertical, d: Dir) -> OperationRole {
        OperationRole { h, v, delta: d }
    }

    fn unit(si: usize, ei: usize, dir: Direction, lo: Tick, hi: Tick) -> UnitRange {
        UnitRange { start_index: si, end_index: ei, direction: dir, lo, hi }
    }

    fn ctr(start: usize, end: usize) -> Center {
        Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: start, end_index: end }
    }

    /// 构造一个真嵌套 L1 走势（Compose 三段 L0 线段子声部，真父子）。
    /// ★codex Q4：注入确定性 ElementId（L0 子 ordinal=0/1/2，L1 父 ordinal=0）。
    fn nested_l1(si: usize, ei: usize, sub_dirs: [Direction; 3]) -> LeveledMove {
        let s0 = LeveledMove::from_unit(&unit(si, si + 4, sub_dirs[0], 0, 10), ElementId { level: 0, ordinal: 0 });
        let s1 = LeveledMove::from_unit(&unit(si + 4, si + 8, sub_dirs[1], 3, 12), ElementId { level: 0, ordinal: 1 });
        let s2 = LeveledMove::from_unit(&unit(si + 8, ei, sub_dirs[2], 5, 15), ElementId { level: 0, ordinal: 2 });
        LeveledMove::compose(&[s0, s1, s2], ctr(si, ei), 1, ElementId { level: 1, ordinal: 0 })
    }

    /// 测试用 ElementId（默认根级 (0,0)）。
    fn eid(level: u32, ordinal: u64) -> ElementId {
        ElementId { level, ordinal }
    }

    /// 测试用 ActiveLeg 构造器（默认 is_boundary_root=true 真边界根 ∂，parent_id=None）。
    /// 旧测试字面量用此辅助补齐 id/parent_id/is_boundary_root 字段（codex Q4）。
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

    // ── §1/§2 元素提取（真嵌套父子，铁律守护）─────────────────────────────────

    /// 元素提取：L1 走势 + 3 个真嵌套 L0 子声部 = 4 个元素（1 根 + 3 子，真父子）。
    #[test]
    fn extract_elements_nested_parent_child() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]); // 索引=级别：L0 空（子由 Compose 带出），L1 一个走势
        let elements = extract_elements(&tower);
        // 1 根（L1 走势）+ 3 子（L0 线段，真嵌套）= 4 元素。
        assert_eq!(elements.len(), 4);
        // 根元素 parent=None（边界胚元 ∂，去根化）。
        assert_eq!(elements[0].parent, None);
        assert_eq!(elements[0].level, 1);
        // 子元素 parent=Some(0)（真父子，非级别差伪造）。
        assert_eq!(elements[1].parent, Some(0));
        assert_eq!(elements[2].parent, Some(0));
        assert_eq!(elements[3].parent, Some(0));
        // 子元素 level=0（descend 级别严格递减）。
        assert_eq!(elements[1].level, 0);
    }

    /// 元素区间 [λ_e,ρ_e)：根覆盖全跨度，子覆盖各自 L0 段（操作区间真坐标）。
    #[test]
    fn element_intervals_from_tower_coords() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]);
        let elements = extract_elements(&tower);
        // 根：λ=0, ρ=12（窗口首起点..末终点）。
        assert_eq!((elements[0].lambda, elements[0].rho), (0, 12));
        // 子0：λ=0, ρ=4（首 L0 段）。
        assert_eq!((elements[1].lambda, elements[1].rho), (0, 4));
        // 子1：λ=4, ρ=8。
        assert_eq!((elements[2].lambda, elements[2].rho), (4, 8));
    }

    // ── §2b 638 附着判准（hostOf 本级右端点命中；三漏洞防护）─────────────────────

    /// 两父塔：compose_a(Long) + compose_b(Short) 相邻；a2 与 b0 **结构全等**(Seg{Up,5,15})
    /// 但属不同父 + 不同 ρ（漏洞③可观测）。a2.ρ=12==b0.λ=12（共享端点，漏洞②）。
    /// compose_a.ρ=12==a2.ρ=12 跨级共享（漏洞①）。
    fn two_parent_tower() -> Vec<Rc<Vec<LeveledMove>>> {
        let a0 = LeveledMove::from_unit(&unit(0, 4, Direction::Up, 0, 10), eid(0, 0));
        let a1 = LeveledMove::from_unit(&unit(4, 8, Direction::Down, 3, 12), eid(0, 1));
        let a2 = LeveledMove::from_unit(&unit(8, 12, Direction::Up, 5, 15), eid(0, 2));
        let compose_a = LeveledMove::compose(&[a0, a1, a2], ctr(0, 12), 1, eid(1, 0)); // 外缘 10→15 ⟹ Long
        let b0 = LeveledMove::from_unit(&unit(12, 16, Direction::Up, 5, 15), eid(0, 3)); // 结构 == a2
        let b1 = LeveledMove::from_unit(&unit(16, 20, Direction::Down, 3, 12), eid(0, 4));
        let b2 = LeveledMove::from_unit(&unit(20, 24, Direction::Down, 0, 8), eid(0, 5));
        let compose_b = LeveledMove::compose(&[b0, b1, b2], ctr(12, 24), 1, eid(1, 1)); // 外缘 15→8 ⟹ Short
        rc_tower(vec![Vec::new(), vec![compose_a, compose_b]])
    }

    /// 638 附着：候选继承 hostOf 的真 Compose 父 + 父方向 σ_{p(g)}（V≠Ambient 前提，真父附着）。
    #[test]
    fn attach_inherits_host_compose_parent_dir() {
        let tree = extract_elements(&two_parent_tower());
        // 级别 0、source_index=8（= a1 的 end_index，hostOf=a1，父=compose_a=Long）。
        let (parent, sigma) = attach_bsp_to_tree(&tree, 0, 8);
        assert_eq!(parent, Some(0), "hostOf(a1) 的真 Compose 父 = compose_a(idx0)");
        assert_eq!(sigma, Some(VoiceSide::Long), "sigma_p(g) = 父走势方向 Long");
    }

    /// 638 漏洞②：严格右端点命中——共享端点处归**产出段**（前一走势 ρ 命中），非后一走势 λ。
    #[test]
    fn attach_right_endpoint_归产出段_not_next_start() {
        let tree = extract_elements(&two_parent_tower());
        // source_index=12：a2.ρ=12（产出段，父 compose_a=Long）∧ b0.λ=12（后一走势起点）。
        // 右端点命中 ⟹ host=a2 → (compose_a, Long)，**非** b0（b0.ρ=16，父 compose_b=Short）。
        let (parent, sigma) = attach_bsp_to_tree(&tree, 0, 12);
        assert_eq!(parent, Some(0), "右端归产出段 a2 → 父 compose_a");
        assert_eq!(sigma, Some(VoiceSide::Long), "非后一走势 b0（那会是 Short）");
        // 反证：b0 的右端点是 16，不是 12 ⟹ 12 不附着到 b0。
        let (pb, sb) = attach_bsp_to_tree(&tree, 0, 16);
        assert_eq!((pb, sb), (Some(4), Some(VoiceSide::Short)), "16=b0.ρ ⟹ host=b0,父 compose_b=Short");
    }

    /// 638 漏洞①：级别上下文消歧——同 source_index 跨级共享端点，按候选级别命中本级 host。
    #[test]
    fn attach_level_context_disambiguates_shared_rho() {
        let tree = extract_elements(&two_parent_tower());
        // compose_a.ρ=12（L1）与 a2.ρ=12（L0）共享 ρ=12。
        // 级别 0 ⟹ host=a2（有真父 compose_a=Long）。
        assert_eq!(attach_bsp_to_tree(&tree, 0, 12), (Some(0), Some(VoiceSide::Long)));
        // 级别 1 ⟹ host=compose_a（顶层根，无父）⟹ Ambient（不误命中 L0 host）。
        assert_eq!(attach_bsp_to_tree(&tree, 1, 12), (None, None));
    }

    /// 638 漏洞③：ordinal 坐标身份——结构全等子走势(a2/b0=Seg{Up,5,15})按 (level,ρ) 区分附着。
    #[test]
    fn attach_ordinal_identity_not_rmove_struct_equal() {
        let tree = extract_elements(&two_parent_tower());
        // a2 与 b0 RMove 结构全等。结构相等查表(index_of_in)会把 16 误配 a2(首个结构匹配,父Long)。
        // (level,ρ) 身份 ⟹ ρ=16 唯一命中 b0（父 compose_b=Short），ρ=12 唯一命中 a2（父 Long）。
        assert_eq!(attach_bsp_to_tree(&tree, 0, 16).1, Some(VoiceSide::Short), "ρ=16 → b0 真父 Short");
        assert_eq!(attach_bsp_to_tree(&tree, 0, 12).1, Some(VoiceSide::Long), "ρ=12 → a2 真父 Long");
    }

    /// ★方案D K_i（裁决648）：extract_carrier_forest 含**所有级别**元素（不像 T_i 只最高级根），
    /// dedup 后每 ElementId 唯一、parent 索引有效（codex NO#2 守卫）。
    #[test]
    fn carrier_forest_all_levels_dedup_unique_id() {
        // two_parent_tower：L1 有 compose_a/compose_b（各 3 个 L0 sub），L0 级为空（sub 由 Compose 带出）。
        let tower = two_parent_tower();
        let t_i = extract_elements(&tower); // T_i：只展开最高非空级（L1）根 → 2 根 + 6 子 = 8
        let k_i = extract_carrier_forest(&tower); // K_i：所有级别（L1 同上；L0 空 ⟹ 无额外根）
        // 本塔 L0 级为空（sub_moves 携子），故 K_i 与 T_i 元素数相同（dedup 后），但 ElementId 必唯一。
        let mut ids: Vec<_> = k_i.iter().map(|e| (e.id.level, e.id.ordinal)).collect();
        let n_before = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n_before, "K_i dedup 后无重复 ElementId（codex NO#2）");
        // parent 索引有效（指向 K_i 内更早元素，父在子前不变量）。
        for (i, e) in k_i.iter().enumerate() {
            if let Some(p) = e.parent {
                assert!(p < i, "parent 索引 {p} 必 < 子索引 {i}（父在子前）");
                assert!(p < k_i.len(), "parent 索引越界");
            }
        }
        // endpoint-complete 见证：每个 L0 子走势的 ρ 在 K_i 中可命中（host^op 不 miss）。
        // a1.ρ=8（L0），compose_a.ρ=12（L1）都应在 K_i。
        let kidx = build_tree_endpoint_index(&k_i);
        assert!(kidx.contains_key(&(0, 8)), "L0 子 ρ=8 在 K_i（endpoint-complete）");
        assert!(kidx.contains_key(&(1, 12)), "L1 根 ρ=12 在 K_i");
    }

    /// ★方案D K_i dedup **真触发**（codex NO#2 核心）：L0 级非空 ∧ 其元素同时是 L1 compose 的 sub
    /// ⟹ 同一 ElementId 被遍历两次（作 L0 根 parent=None + 作 L1 子 parent=Some）。dedup 须保留**带
    /// parent_id 的出现**（子声部父链所需），且最终无重复 ElementId、parent 索引有效。
    #[test]
    fn carrier_forest_dedup_triggers_when_l0_nonempty() {
        let s0 = LeveledMove::from_unit(&unit(0, 4, Direction::Up, 0, 10), eid(0, 0));
        let s1 = LeveledMove::from_unit(&unit(4, 8, Direction::Down, 3, 12), eid(0, 1));
        let s2 = LeveledMove::from_unit(&unit(8, 12, Direction::Up, 5, 15), eid(0, 2));
        let l1 = LeveledMove::compose(&[s0.clone(), s1.clone(), s2.clone()], ctr(0, 12), 1, eid(1, 0));
        // L0 级**非空**（含 s0/s1/s2）+ L1 级含 compose（其 sub_moves 也是 s0/s1/s2，同 ElementId）。
        let tower = rc_tower(vec![vec![s0, s1, s2], vec![l1]]);
        let k_i = extract_carrier_forest(&tower);
        // 4 个唯一元素：L1 根 (1,0) + 3 个 L0 (0,0)/(0,1)/(0,2)——dedup 合并了重复遍历。
        let mut ids: Vec<_> = k_i.iter().map(|e| (e.id.level, e.id.ordinal)).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "dedup 后无重复 ElementId");
        assert_eq!(n, 4, "L1 根 + 3 L0 子 = 4 唯一元素（重复遍历被 dedup）");
        // L0 元素保留**带 parent 的出现**（作 L1 子时 parent_id=Some(1,0)），非作根的 None。
        let l0_child = k_i.iter().find(|e| e.id == eid(0, 0)).expect("s0 在 K_i");
        assert_eq!(l0_child.parent_id, Some(eid(1, 0)), "dedup 保留带真 parent_id 的出现（codex NO#2）");
        assert!(l0_child.parent.is_some(), "parent 索引指向 L1 根");
        // parent 索引有效。
        for (i, e) in k_i.iter().enumerate() {
            if let Some(p) = e.parent {
                assert!(p < i && p < k_i.len(), "parent 索引有效");
            }
        }
    }

    /// 638 边界：host 未找到（无 ρ==source_index 的本级元素）⟹ (None,None) 去根化 Ambient。
    #[test]
    fn attach_host_not_found_is_ambient() {
        let tree = extract_elements(&two_parent_tower());
        assert_eq!(attach_bsp_to_tree(&tree, 0, 99), (None, None), "无本级 host ⟹ 无真父 ⟹ Ambient");
    }

    /// 638 边界 + tower-export-i guard：tower.len()<2（仅 L0 全根）⟹ host 是根 ⟹ 恒 Ambient。
    #[test]
    fn attach_guard_no_compose_level_is_ambient() {
        let s0 = LeveledMove::from_unit(&unit(0, 4, Direction::Up, 0, 10), eid(0, 0));
        let s1 = LeveledMove::from_unit(&unit(4, 8, Direction::Down, 3, 12), eid(0, 1));
        let tower = rc_tower(vec![vec![s0, s1]]); // 仅 L0，len()==1 < 2（无 Compose 级）
        let tree = extract_elements(&tower);
        // host=s0（ρ=4，level 0）但 s0 是根（parent=None）⟹ 缺塔诚实退化 Ambient。
        assert_eq!(attach_bsp_to_tree(&tree, 0, 4), (None, None), "缺塔（len<2）⟹ host 是根 ⟹ Ambient");
    }

    // ── §3 活动集递归 A_{t+1}=AncOK[(A_t∖D_t)∪B_t]（先关后开 + 祖先闭合）──────────

    /// B_t/D_t：bar t 的开始/结束元素索引（按 λ_e/ρ_e 过滤）。
    #[test]
    fn starting_ending_sets_by_endpoints() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]);
        let elements = extract_elements(&tower);
        // bar 0：根（λ=0）+ 子0（λ=0）开始。
        let b0 = starting_set(&elements, 0);
        assert!(b0.contains(&0) && b0.contains(&1));
        // bar 4：子1（λ=4）开始；子0（ρ=4）结束。
        assert!(starting_set(&elements, 4).contains(&2));
        assert!(ending_set(&elements, 4).contains(&1));
    }

    /// ★祖先闭合裁掉祖先不齐者：子元素激活但父不在 raw ⟹ 被 AncOK 裁掉（覆盖不漂浮）。
    #[test]
    fn ancestor_close_prunes_orphan_child() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]);
        let elements = extract_elements(&tower);
        // 人为构造：raw 只含子1（idx=2），不含其父（根 idx=0）⟹ AncOK 裁掉子1（祖先不齐）。
        let active = active_set_step(&elements, &[2], 99); // t=99 无新开始/结束，raw=active∖∅
        // 子1 的祖先（根 0）不在 raw{2} ⟹ 被裁 ⟹ 空活动集。
        assert!(active.is_empty(), "孤儿子元素（父不在场）被祖先闭合裁掉");
    }

    /// ★祖先齐全则保留：根 + 子同在 raw ⟹ AncOK 都保留（覆盖闭合）。
    #[test]
    fn ancestor_close_keeps_closed_family() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]);
        let elements = extract_elements(&tower);
        // raw 含根（0）+ 子0（1）：子0 祖先=根0 在 raw ⟹ 都保留。
        let active = active_set_step(&elements, &[0, 1], 99);
        assert!(active.contains(&0) && active.contains(&1));
    }

    /// 先关后开：D_t 中的元素被关闭（不在 A_{t+1}），B_t 中的被开启。
    #[test]
    fn raw_close_then_open() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]);
        let elements = extract_elements(&tower);
        // t=4：子0（ρ=4）结束被关，子1（λ=4）开始被开。active 初始含根+子0。
        let active = active_set_step(&elements, &[0, 1], 4);
        // 子0（idx=1，ρ=4∈D_t）被关闭，不在 A_{t+1}。
        assert!(!active.contains(&1), "结束元素 D_t 被关闭");
        // 子1（idx=2，λ=4∈B_t）被开启（其祖先根0在 active）。
        assert!(active.contains(&2), "开始元素 B_t 被开启");
        // 根（idx=0，ρ=12≠4）保持。
        assert!(active.contains(&0), "未结束的根保持");
    }

    // ── §4 操作角色 R(g)=(H,V,δ) 18 类（H 兄弟轴 / V 父轴 / δ 方向，去根化）──────────

    /// ★去根化：顶层元素（parent=None=边界胚元 ∂）= (First, Ambient, δ)——**无 RootRole**。
    #[test]
    fn role_toplevel_is_first_ambient_not_root() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]);
        let elements = extract_elements(&tower);
        let r = operation_role(&elements, 0);
        // 顶层根（唯一同级元素）：H=First（无前兄弟），V=Ambient（父胚元 σ=0），δ=Plus（外缘上移 Long）。
        assert_eq!(r.h, Horizontal::First);
        assert_eq!(r.v, Vertical::Ambient, "去根化：顶层父=胚元 σ=0 → Ambient（非 RootRole）");
        assert_eq!(r.delta, Dir::Plus);
    }

    /// ★垂直轴 V：次级别顺父子 ⟹ FollowParent（δ_g=σ_{p(g)}）；反父子 ⟹ ShortDiff（δ_g=−σ_{p(g)}）。
    #[test]
    fn vertical_followparent_vs_shortdiff_by_parent_direction() {
        // 根 L1 走势外缘上移 ⟹ ε_root=Long（σ_{p(g)}=+1 for 子）。
        // 子0=Up(Long)=顺父 ⟹ FollowParent；子1=Down(Short)=反父 ⟹ ShortDiff。
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]);
        let elements = extract_elements(&tower);
        // 根方向（外缘 hi：首子 hi=10, 末子 hi=15 ⟹ 上移 ⟹ Long）。
        assert_eq!(elements[0].eps, VoiceSide::Long);
        // 子0（Up=Long）顺父（+1）⟹ V=FollowParent。
        assert_eq!(vertical_relation(&elements, 1), Vertical::FollowParent);
        // 子1（Down=Short）反父（+1）⟹ V=ShortDiff（短差，反向子声部）。
        assert_eq!(vertical_relation(&elements, 2), Vertical::ShortDiff);
        // H 轴：子0 无前同级兄弟 ⟹ First；子1 前兄弟=子0(Long,+1)，δ=−1 ⟹ SameReverse。
        assert_eq!(horizontal_relation(&elements, 1), Horizontal::First);
        assert_eq!(horizontal_relation(&elements, 2), Horizontal::SameReverse);
        // 完整角色元组（子1）：(SameReverse, ShortDiff, Minus)。
        assert_eq!(operation_role(&elements, 2), role(Horizontal::SameReverse, Vertical::ShortDiff, Dir::Minus));
    }

    /// ★水平轴 H（同级别兄弟）：同一父容器下前兄弟 ⟹ SameFollow（同向）/SameReverse（反向）。
    #[test]
    fn horizontal_sibling_follow_vs_reverse() {
        // 三个同级别兄弟（同父=边界胚元 ∂ parent=None，同 level=1）——去根化下顶层亦是兄弟。
        let elements = vec![
            CoverageElement { lambda: 0, rho: 4, eps: VoiceSide::Long, level: 1, parent: None, attached_dir: None, id: eid(1, 0), parent_id: None },
            CoverageElement { lambda: 4, rho: 8, eps: VoiceSide::Long, level: 1, parent: None, attached_dir: None, id: eid(1, 1), parent_id: None },
            CoverageElement { lambda: 8, rho: 12, eps: VoiceSide::Short, level: 1, parent: None, attached_dir: None, id: eid(1, 2), parent_id: None },
        ];
        // idx0：无前兄弟 ⟹ First。
        assert_eq!(horizontal_relation(&elements, 0), Horizontal::First);
        // idx1：前兄弟 idx0(Long,+1)，δ=+1 ⟹ SameFollow。
        assert_eq!(horizontal_relation(&elements, 1), Horizontal::SameFollow);
        // idx2：前兄弟 idx1(Long,+1)，δ=−1 ⟹ SameReverse。
        assert_eq!(horizontal_relation(&elements, 2), Horizontal::SameReverse);
        // 三者 V 均 Ambient（父胚元 σ=0，去根化无 RootRole）。
        assert!((0..elements.len()).all(|i| vertical_relation(&elements, i) == Vertical::Ambient));
    }

    // ── §5 LegTarget（方向 ε_e + role/depth 权重单位数）──────────────────────────

    /// 腿方向 = ε_e（M17 σ_{ν(e)}=ε_e）；单位 = base × w_depth（M28 深度权重）。
    #[test]
    fn leg_target_side_and_depth_weighted_units() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]);
        let elements = extract_elements(&tower);
        let c = cfg();
        // 根腿（depth 0）：side=ε_root=Long，units=1000×w[0]=1000×0.60=600。
        let root_leg = leg_target(&elements, 0, 1000.0, &c);
        assert_eq!(root_leg.side, VoiceSide::Long);
        assert!((root_leg.units - 600.0).abs() < 1e-9, "根 depth 0 权重 0.60");
        // 顶层根角色：(First, Ambient, Plus)——去根化无 RootRole。
        assert_eq!(root_leg.role, role(Horizontal::First, Vertical::Ambient, Dir::Plus));
        // 子腿（depth 1）：units=1000×w[1]=1000×0.30=300。
        let sub_leg = leg_target(&elements, 1, 1000.0, &c);
        assert!((sub_leg.units - 300.0).abs() < 1e-9, "子 depth 1 权重 0.30");
        // 子1 是短差（V=ShortDiff），方向 Short（短差反向子声部腿）。
        let shortdiff_leg = leg_target(&elements, 2, 1000.0, &c);
        assert_eq!(shortdiff_leg.side, VoiceSide::Short);
        assert_eq!(shortdiff_leg.role.v, Vertical::ShortDiff);
    }

    // ── §6 净额执行（毛腿 → 净持仓，对冲腿部分抵消父仓）──────────────────────────

    /// ★净额合并：多腿+空腿净额抵消（ShortDiff 空腿抵消父多腿，M11 短差对冲）。
    #[test]
    fn net_target_units_hedges_parent_with_shortdiff() {
        // 根多腿 600 + 子0 顺势多腿 300 + 子1 短差空腿 300 ⟹ 净 = 600+300−300 = 600。
        let legs = vec![
            LegTarget { e_idx: 0, side: VoiceSide::Long, units: 600.0, role: role(Horizontal::First, Vertical::Ambient, Dir::Plus) },
            LegTarget { e_idx: 1, side: VoiceSide::Long, units: 300.0, role: role(Horizontal::First, Vertical::FollowParent, Dir::Plus) },
            LegTarget { e_idx: 2, side: VoiceSide::Short, units: 300.0, role: role(Horizontal::SameReverse, Vertical::ShortDiff, Dir::Minus) },
        ];
        let net = net_target_units(&legs);
        assert!((net - 600.0).abs() < 1e-9, "净 = 600+300−300（短差空腿对冲）");
        // 毛敞口 = |600|+|300|+|300| = 1200（双开毛敞口 > 净额 600，net_le_gross）。
        let gross = gross_target_units(&legs);
        assert!((gross - 1200.0).abs() < 1e-9, "毛敞口 1200 > 净 600（双开毛收益级覆盖）");
        assert!(net.abs() <= gross, "净 ≤ 毛（risk.rs net_le_gross）");
    }

    /// 精确对冲：单位数相同的多空腿 ⟹ 净额 0（毛敞口满额，net_le_gross 极端情形）。
    #[test]
    fn net_zero_when_hedged_equal() {
        let legs = vec![
            LegTarget { e_idx: 0, side: VoiceSide::Long, units: 400.0, role: role(Horizontal::First, Vertical::Ambient, Dir::Plus) },
            LegTarget { e_idx: 1, side: VoiceSide::Short, units: 400.0, role: role(Horizontal::SameReverse, Vertical::ShortDiff, Dir::Minus) },
        ];
        assert!((net_target_units(&legs)).abs() < 1e-9, "等单位多空 ⟹ 净 0");
        assert!((gross_target_units(&legs) - 800.0).abs() < 1e-9, "毛 800（双开满额）");
    }

    // ── §3 活动集递归原语（λ_e 区间，Lean M16 对齐——非生产入场，见 §3 GAP-5 note）已在上方测 ──

    /// ★扁平入口 from_classification_levels：从 Classification 的中枢提取根级覆盖元素
    /// （生产入口验证，no-声明膨胀——坐实它真从 levels 产元素）。诚实边界：扁平近似无真嵌套
    /// ⟹ 父=胚元 σ=0，V 恒 Ambient（无 FollowParent/ShortDiff，短差需真 Compose 塔，用 extract_elements）。
    #[test]
    fn from_classification_levels_flat_root_coverage() {
        use super::super::super::classifier::{Classification, LevelState};
        use super::super::super::types::MoveKind;
        // L0 一个中枢（盘整），L1 一个中枢（盘整）——两个根级覆盖元素（同级兄弟，无父子）。
        let classification = Classification {
            levels: vec![
                LevelState {
                    moves: vec![MoveKind::Consolidation],
                    centers: vec![ctr(0, 12)],
                    ..Default::default()
                },
                LevelState {
                    moves: vec![MoveKind::Trend],
                    centers: vec![ctr(0, 30)],
                    ..Default::default()
                },
            ],
        };
        let elements = from_classification_levels(&classification);
        assert_eq!(elements.len(), 2, "两级各一中枢 ⟹ 两个根级覆盖元素");
        // 去根化：扁平入口父=胚元 σ=0 ⟹ V=Ambient（无 RootRole）；不同级别(L0/L1)非同级兄弟 ⟹ 各自 First。
        assert!(elements.iter().all(|e| e.parent.is_none()), "扁平入口全根级（无真父子）");
        assert_eq!(vertical_relation(&elements, 0), Vertical::Ambient);
        assert_eq!(vertical_relation(&elements, 1), Vertical::Ambient);
        assert_eq!(horizontal_relation(&elements, 0), Horizontal::First);
        assert_eq!(horizontal_relation(&elements, 1), Horizontal::First);
        // 级别索引：L0 元素 level=0，L1 元素 level=1。
        assert_eq!(elements[0].level, 0);
        assert_eq!(elements[1].level, 1);
        // 区间来自中枢 [start_index, end_index)。
        assert_eq!((elements[0].lambda, elements[0].rho), (0, 12));
    }

    /// 空塔 ⟹ 空元素集（提取边界）。
    #[test]
    fn empty_tower_yields_empty_elements() {
        let elements = extract_elements(&[]);
        assert!(elements.is_empty());
    }

    // ── §8 环6：解释器三桶 → A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x] → p̃（桶驱动，闭环 interpret）──

    use super::super::interp::{ActiveLeg, Buckets, Candidate};
    use super::super::super::classifier::bsp::BspPoint;
    use super::super::super::classifier::LevelState;
    use super::super::super::types::BspBits;

    /// 测试用候选构造器（ℬ_x 开启候选，方向/级别可控）。
    fn cand(level: u32, source_index: usize, dir: VoiceSide, gamma_index: usize) -> Candidate {
        Candidate {
            level,
            source_index,
            bits: BspBits::default(),
            dir,
            bsp_class: 1,
            role: role(Horizontal::First, Vertical::Ambient, Dir::Plus),
            nest_confirmed: true,
            gamma_index,
        }
    }

    /// 测试辅助：ℬ_x 开启候选 → 扁平根元素数组（缺塔/Ambient 路径，`candidate_start=0`）。
    /// `elements[gamma_index]` = 候选根（`parent=None` ⟹ AncOK 恒等准入，与缺塔生产路径一致）。
    /// 按 `gamma_index` 定位（测试 `cand` 用连续 0..n，无父子）。
    fn flat_elements(open: &[Candidate]) -> Vec<CoverageElement> {
        let n = open.iter().map(|c| c.gamma_index + 1).max().unwrap_or(0);
        let mut v: Vec<CoverageElement> = (0..n)
            .map(|i| CoverageElement {
                lambda: 0,
                rho: 0,
                eps: VoiceSide::Flat,
                level: 0,
                parent: None,
                attached_dir: None,
                id: ElementId { level: 0, ordinal: i as u64 },
                parent_id: None,
            })
            .collect();
        for c in open {
            v[c.gamma_index] = CoverageElement {
                lambda: c.source_index,
                rho: c.source_index,
                eps: c.dir,
                level: c.level,
                parent: None,
                attached_dir: None,
                id: ElementId { level: c.level, ordinal: c.source_index as u64 },
                parent_id: None,
            };
        }
        v
    }

    /// ★O(n²) 重构测试适配：把旧 `(elements, candidate_start)` 拆成 ElementView 双段
    /// （base=tree 前缀 elements[..cstart]，overlay=candidate 段 elements[cstart..]）。
    /// bit-exact == 旧合并 Vec：索引语义不变（base 在前 overlay 在后）。
    fn view_split(elements: &[CoverageElement], cstart: usize) -> ElementView {
        ElementView::from_parts(&elements[..cstart], elements[cstart..].to_vec())
    }

    /// ★工位 4c bit-exact 守卫：`operation_role_indexed_split`（双段 tree+candidate 兄弟索引）
    /// == `operation_role_indexed`（旧单合并 sibling_idx）。LCG 压力构造多 (parent,level,eps,cstart)
    /// 配置，逐 candidate 元素断言 role 三轴逐字段相等。覆盖前兄弟在 tree 段 / candidate 段 / 无前兄弟
    /// 三种分支（split 的 cand-overlay 优先 + tree-fallback last() 路径）。
    #[test]
    fn operation_role_split_matches_merged_lcg() {
        let mut seed = 0x4c_u64;
        let mut next = || { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (seed >> 33) as usize };
        for _ in 0..200 {
            let tree_n = next() % 8;       // tree 段元素数 0..7
            let cand_n = 1 + next() % 6;   // candidate 段 1..6（至少 1 个被测）
            let n = tree_n + cand_n;
            // 构造 elements：parent 指向更早的 tree idx（或 None），level 小集合、eps 双向。
            let elems: Vec<CoverageElement> = (0..n).map(|i| {
                let parent = if tree_n > 0 && next() % 2 == 0 { Some(next() % tree_n.max(1)) } else { None };
                CoverageElement {
                    lambda: i, rho: i,
                    eps: if next() % 2 == 0 { VoiceSide::Long } else { VoiceSide::Short },
                    level: (next() % 3) as u32,
                    parent: parent.filter(|&p| p < i), // parent 必在自身之前（树前序）
                    attached_dir: if next() % 2 == 0 { None } else { Some(VoiceSide::Long) },
                    id: ElementId { level: 0, ordinal: i as u64 },
                    parent_id: None,
                }
            }).collect();
            let cstart = tree_n;
            // 旧路径：合并 sibling_idx（全 elements）。
            let merged = build_prev_sibling_index(&elems);
            // 新路径：tree-only + candidate-only 双段。
            let tree_sib = build_prev_sibling_index(&elems[..cstart]);
            let mut cand_sib: std::collections::HashMap<(Option<usize>, u32), Vec<usize>> = std::collections::HashMap::new();
            for ci in cstart..n {
                cand_sib.entry((elems[ci].parent, elems[ci].level)).or_default().push(ci);
            }
            let view = ElementView::from_parts(&elems[..cstart], elems[cstart..].to_vec());
            for ci in cstart..n {
                let old = operation_role_indexed(&view, ci, &merged);
                let new = operation_role_indexed_split(&view, ci, &tree_sib, &cand_sib);
                assert_eq!(old, new, "split≠merged @ci={ci} cstart={cstart} n={n}");
            }
        }
    }

    /// L0 卖买卖点（src=si；host 右端点 ρ=si；pivot 远离 ⟹ 止损不触及）。
    fn sell_bsp(si: usize) -> BspPoint {
        BspPoint {
            source_index: si,
            bits: BspBits { sell1: true, ..Default::default() },
            pivot_low: 0,
            pivot_high: 210,
            center: Some(ctr(0, si)),
        }
    }

    /// L0 买买卖点（src=si）。
    fn buy_bsp(si: usize) -> BspPoint {
        BspPoint {
            source_index: si,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90,
            pivot_high: 0,
            center: Some(ctr(0, si)),
        }
    }

    /// ★环6 开启：空 A_t + ℬ_x 一个买候选 ⟹ A_{t+1} 含该腿，p̃ = base×w[0]（根 depth 0）。
    #[test]
    fn buckets_open_creates_active_leg_and_target() {
        let reg = super::super::persistent::PersistentRegistry::new();
        let buckets = Buckets {
            close: vec![],
            open: vec![cand(0, 5, VoiceSide::Long, 0)],
            record: vec![],
        };
        let (active, p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &[], &buckets, 1000.0, &cfg(), &reg);
        assert_eq!(active.len(), 1, "ℬ_x 开启 ⟹ A_{{t+1}} 一条新腿");
        assert_eq!(active[0], aleg(0, VoiceSide::Long, 5, 5 ));
        // p̃ = 1000×w_depth(0)=1000×0.60=600（根 depth 0，多腿正号）。
        assert!((p - 600.0).abs() < 1e-9, "p̃ = base×w[0] = 600（单多腿）");
    }

    /// ★环6 关闭：A_t 一条 Long 腿 + 𝒟_x={该腿} ⟹ A_{t+1}=∅，p̃=0（先关后开）。
    #[test]
    fn buckets_close_removes_active_leg() {
        let reg = super::super::persistent::PersistentRegistry::new();
        let leg = aleg(0, VoiceSide::Long, 3, 3 );
        let buckets = Buckets {
            close: vec![leg], // 𝒟_x ⊆ A_t
            open: vec![],
            record: vec![],
        };
        let (active, p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &[leg], &buckets, 1000.0, &cfg(), &reg);
        assert!(active.is_empty(), "𝒟_x 关闭活动腿 ⟹ A_{{t+1}} 空");
        assert_eq!(p, 0.0, "无活动腿 ⟹ p̃=0");
    }

    /// ★环6 净额聚合：A_{t+1} = {Long, Short} ⟹ p̃ = +600 −600 = 0（方向聚合，多空抵消）。
    #[test]
    fn buckets_target_nets_long_and_short() {
        let reg = super::super::persistent::PersistentRegistry::new();
        let buckets = Buckets {
            close: vec![],
            open: vec![
                cand(0, 1, VoiceSide::Long, 0),
                cand(1, 2, VoiceSide::Short, 1),
            ],
            record: vec![],
        };
        let (active, p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &[], &buckets, 1000.0, &cfg(), &reg);
        assert_eq!(active.len(), 2);
        // 两腿同根 depth 0（w[0]=0.60）：Long +600，Short −600 ⟹ 净 0。
        assert!(p.abs() < 1e-9, "p̃ = +600 −600 = 0（方向净额聚合）");
    }

    /// ★环6 先关后开 + 保留：A_t={legA, legB}，𝒟_x={legA}，ℬ_x={candC} ⟹ A_{t+1}={legB, legC}。
    #[test]
    fn buckets_close_then_open_keeps_survivor() {
        let reg = super::super::persistent::PersistentRegistry::new();
        let leg_a = aleg(0, VoiceSide::Long, 1, 1 );
        let leg_b = aleg(1, VoiceSide::Short, 2, 2 );
        let buckets = Buckets {
            close: vec![leg_a],                          // 关 legA
            open: vec![cand(0, 9, VoiceSide::Long, 0)],  // 开 candC（level 0 Long）
            record: vec![],
        };
        let (active, _p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &[leg_a, leg_b], &buckets, 1000.0, &cfg(), &reg);
        assert!(!active.contains(&leg_a), "legA 被 𝒟_x 关闭");
        assert!(active.contains(&leg_b), "legB（不在 𝒟_x）保留");
        assert!(
            active.contains(&aleg(0, VoiceSide::Long, 9, 9 )),
            "candC 被 ℬ_x 开启"
        );
        assert_eq!(active.len(), 2);
    }

    /// ★环6 AncOK 扁平恒等（诚实有效域）：桶路径活动腿全独立根（parent=∂）⟹ Anc=∅ ⟹ 不剔任何腿。
    /// （真嵌套塔剔孤儿在 §3 active_set_step 已验；本桶路径无塔故恒等——MEMORY tower-export-bridge 缺口）。
    #[test]
    fn buckets_ancok_identity_on_flat_roots() {
        let reg = super::super::persistent::PersistentRegistry::new();
        let legs = [
            aleg(0, VoiceSide::Long, 0, 0 ),
            aleg(1, VoiceSide::Long, 1, 1 ),
            aleg(2, VoiceSide::Short, 2, 2 ),
        ];
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        let (active, _p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &legs, &buckets, 1000.0, &cfg(), &reg);
        assert_eq!(active.len(), 3, "扁平根全保留（AncOK 恒等——无父子可剔孤儿）");
        let reg = super::super::persistent::PersistentRegistry::new();
    }

    /// ★环6 immutable：coverage_step_from_buckets 不 mutate prev_active。
    #[test]
    fn buckets_step_immutable_prev_active() {
        let reg = super::super::persistent::PersistentRegistry::new();
        let prev = vec![aleg(0, VoiceSide::Long, 0, 0 )];
        let snapshot = prev.clone();
        let buckets = Buckets {
            close: vec![],
            open: vec![cand(1, 3, VoiceSide::Short, 0)],
            record: vec![],
        };
        let _ = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &prev, &buckets, 1000.0, &cfg(), &reg);
        assert_eq!(prev, snapshot, "桶驱动递归不 mutate prev_active（纯函数）");
    }

    /// 𝒦_x（record 桶）不进活动集（spec「记录但暂不执行」）。
    #[test]
    fn buckets_record_excluded_from_active_set() {
        let reg = super::super::persistent::PersistentRegistry::new();
        let buckets = Buckets {
            close: vec![],
            open: vec![],
            record: vec![cand(0, 7, VoiceSide::Long, 0)], // 𝒦_x：记录不执行
        };
        let (active, p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &[], &buckets, 1000.0, &cfg(), &reg);
        assert!(active.is_empty(), "𝒦_x 不入活动集");
        assert_eq!(p, 0.0);
    }

    /// 空三桶 + 空 A_t ⟹ A_{t+1}=∅，p̃=0（边界）。
    #[test]
    fn buckets_empty_yields_empty_and_zero() {
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        let reg = super::super::persistent::PersistentRegistry::new();
        let (active, p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &[], &buckets, 1000.0, &cfg(), &reg);
        assert!(active.is_empty());
        assert_eq!(p, 0.0);
    }

    /// ★环5+环6 端到端：Classification（一买点）→ Γ → 解释器 → A_{t+1} → p̃。
    #[test]
    fn classification_end_to_end_ring5_ring6() {
        let reg = super::super::persistent::PersistentRegistry::new();
        // L0 一个一类买点（source_index=4）。
        let bsp = BspPoint {
            source_index: 4,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90,
            pivot_high: 0,
            center: Some(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 }),
        };
        let classification = Classification {
            levels: vec![LevelState { bsp: vec![bsp], ..Default::default() }],
        };
        // 空 A_t：买候选开启 ⟹ A_{t+1} 一条 Long 腿，p̃ = 600。
        // 空塔（tower &[]）⟹ 候选父=∂ ⟹ Ambient（与扁平一致）；本测试只验开腿/p̃，角色不约束。
        let (active, p) = coverage_step_classification(&classification, &[], &[], 1000.0, &cfg(), &reg);
        assert_eq!(active.len(), 1, "买点 ℬ_x 开启 ⟹ 一条活动腿");
        assert_eq!(active[0].dir, VoiceSide::Long);
        assert_eq!(active[0].source_index, 4);
        assert!((p - 600.0).abs() < 1e-9, "p̃ = base×w[0] = 600");
    }

    /// ★环5↔环6 闭环：A_{t+1} 回喂 interpret——持仓 Long 遇反向卖点 ⟹ 关闭，A_{t+2}=∅。
    #[test]
    fn ring6_active_set_feeds_back_into_interpret() {
        let reg = super::super::persistent::PersistentRegistry::new();
        // bar t：买点开 Long。
        let buy = BspPoint {
            source_index: 0,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90, pivot_high: 0,
            center: Some(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 }),
        };
        let c_buy = Classification { levels: vec![LevelState { bsp: vec![buy], ..Default::default() }] };
        let (active_t1, _) = coverage_step_classification(&c_buy, &[], &[], 1000.0, &cfg(), &reg);
        assert_eq!(active_t1.len(), 1, "买点开 Long 腿");
        // bar t+1：卖点（反向）→ A_{t+1} 回喂 interpret ⟹ 关闭 Long 腿 ⟹ A_{t+2}=∅。
        let sell = BspPoint {
            source_index: 10,
            bits: BspBits { sell1: true, ..Default::default() },
            pivot_low: 0, pivot_high: 210,
            center: Some(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 }),
        };
        let c_sell = Classification { levels: vec![LevelState { bsp: vec![sell], ..Default::default() }] };
        let (active_t2, p2) = coverage_step_classification(&c_sell, &[], &active_t1, 1000.0, &cfg(), &reg);
        assert!(active_t2.is_empty(), "反向卖点关闭持仓 Long（𝒟_x）⟹ A_{{t+2}}=∅（闭环）");
        assert_eq!(p2, 0.0, "无活动腿 ⟹ p̃=0");
    }

    // ── §8b §13 AncOK 持仓准入（639(c)：ShortDiff 未持父则剔除，不开 naked 逆势仓）──────────
    //   真嵌套塔（L1 Long 父走势）+ 638 附着候选 ⟹ AncOK 在真 Compose 父链上对附着候选剪枝/准入。

    /// ★测试①（AncOK 准入）：有向父容器 + **已持父仓** + 逆向次级 ShortDiff 候选 ⟹ ShortDiff 子腿
    /// **准入**（祖先齐全：父容器腿在 A_t）。
    #[test]
    fn ancok_admits_shortdiff_when_parent_held() {
        use super::super::interp::{assemble_gamma_with_tower, coverage_elements_with_tower, interpret};
        // per-bar 因果塔：L1 Long 父走势（compose idx0）+ 3 L0 子（idx1/2/3，sub(4,8) ρ=8）。
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        // L0 卖候选 src=8 ⟹ host=sub(4,8) ⟹ 真父 L1 Long ⟹ ShortDiff（δ=Short=−σ_p）。
        let classification = Classification {
            levels: vec![LevelState { bsp: vec![sell_bsp(8)], ..Default::default() }],
        };
        let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
        let gamma = assemble_gamma_with_tower(&classification, &tower);
        assert_eq!(gamma[0].role.v, Vertical::ShortDiff, "前置：候选 V=ShortDiff（639 σ_p=父容器方向）");
        // 持父仓：prev_active 含父容器腿（L1 Long，ID=(1,0) 与塔 compose 元素同 ID，source_index=ρ=12）。
        // ★codex Q4：held_leg_tree_index 按 ElementId 匹配——leg.id 必须与塔元素 id 一致。
        let held_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 12, lambda: 0,
            id: eid(1, 0), parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let buckets = interpret(&gamma, &[held_parent]);
        let reg = super::super::persistent::PersistentRegistry::new();
        let (active, _p) =
            coverage_step_from_buckets(view_split(&elements, cstart), &[held_parent], &buckets, 1000.0, &cfg(), &reg);
        assert!(
            active.iter().any(|l| l.level == 0 && l.dir == VoiceSide::Short),
            "持父仓 ⟹ ShortDiff 子腿准入（AncOK 祖先齐全）；实得 {active:?}"
        );
        assert!(
            active.iter().any(|l| l.level == 1 && l.dir == VoiceSide::Long),
            "父容器腿保留"
        );
    }

    /// ★测试①b（持久身份 coord_drift，codex Q4 假阴性消除）：父走势**延伸 end_index 变但同一父**
    /// （吸收更多次级别子走势 ⟹ ρ 漂移）+ 已持父仓 + 逆向次级 ShortDiff 候选 ⟹ ShortDiff 子腿
    /// **仍准入**（持久身份 λ 稳定对位识别为 coord_drift，非误判 stale 剪枝）。
    ///
    /// 旧逻辑（仅 ρ 精确匹配）：持仓父腿 ρ 漂移 ⟹ `held_leg_tree_index` 误判 None ⟹ 静默降 orphan ⟹
    /// 子腿父不在 raw ⟹ AncOK 误剪 ShortDiff（**假阴性**）。本测试断言修正后不再剪。
    #[test]
    fn ancok_admits_shortdiff_under_parent_coord_drift() {
        use super::super::interp::{assemble_gamma_with_tower, coverage_elements_with_tower, interpret};
        // 当前因果塔：L1 Long 父走势**已延伸**到 ρ=12（持仓时旧 ρ 曾=8，现吸收更多子走势 ρ 漂移）。
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        let classification = Classification {
            levels: vec![LevelState { bsp: vec![sell_bsp(8)], ..Default::default() }],
        };
        let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
        let gamma = assemble_gamma_with_tower(&classification, &tower);
        assert_eq!(gamma[0].role.v, Vertical::ShortDiff, "前置：候选 V=ShortDiff（639 σ_p=父容器方向）");
        // ★codex Q4 发现 B 归因修正：旧注释"ρ 漂移"归因错误——λ=start_index 不变，真因是值比较非
        // spec §13 结构映射。Q4 修复：按确定性 ElementId 匹配——leg.id=(1,0) 与塔 L1 父元素同 ID
        //（父延伸 ρ 8→12 不变 ID）⟹ Exact 命中 ⟹ 延伸父仍在 raw ⟹ ShortDiff 子腿准入。
        let held_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 8, lambda: 0,
            id: eid(1, 0), parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let buckets = interpret(&gamma, &[held_parent]);
        let reg = super::super::persistent::PersistentRegistry::new();
        let (active, _p) =
            coverage_step_from_buckets(view_split(&elements, cstart), &[held_parent], &buckets, 1000.0, &cfg(), &reg);
        assert!(
            active.iter().any(|l| l.level == 0 && l.dir == VoiceSide::Short),
            "ID 匹配（确定性 ElementId）⟹ 延伸父仍在 raw ⟹ ShortDiff 子腿准入（Q4 假阴性消除）；实得 {active:?}"
        );
        let reg = super::super::persistent::PersistentRegistry::new();
        assert!(
            active.iter().any(|l| l.level == 1 && l.dir == VoiceSide::Long),
            "延伸父容器腿按 ID 对位回当前树元素并保留（非降 orphan）"
        );
    }

    /// ★测试②（AncOK 剪枝，639(c) 关键测试）：有向父容器 + **未持父仓** + 逆向次级 ShortDiff 候选 ⟹
    /// ShortDiff 子腿**被剔除**（不开 naked 逆势仓 garbage trade）。这是 639(c) 承诺的兑现。
    #[test]
    fn ancok_prunes_shortdiff_when_parent_unheld() {
        use super::super::interp::{assemble_gamma_with_tower, coverage_elements_with_tower, interpret};
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        let classification = Classification {
            levels: vec![LevelState { bsp: vec![sell_bsp(8)], ..Default::default() }],
        };
        let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
        let gamma = assemble_gamma_with_tower(&classification, &tower);
        assert_eq!(gamma[0].role.v, Vertical::ShortDiff, "前置：候选 V=ShortDiff（σ 来源 ⊥ 持仓）");
        // 未持父仓：空 prev_active。
        let buckets = interpret(&gamma, &[]);
        // ★Q3 加固（codex）：坐实剪枝**前**候选确实进 open 桶 + 携非空父指针——否则
        // active.is_empty() 在「候选从未入场」时也假性通过（interpret 行为变后防假阳性）。
        assert_eq!(buckets.open.len(), 1, "interpret 确实把 ShortDiff 候选放入 open 桶（非从未入场）");
        assert!(
            matches!(elements[cstart].parent, Some(_)),
            "候选携非空真 Compose 父指针（AncOK 剪枝的前提是父存在但未持，非父=None）"
        );
        let reg = super::super::persistent::PersistentRegistry::new();
        let (active, p) = coverage_step_from_buckets(view_split(&elements, cstart), &[], &buckets, 1000.0, &cfg(), &reg);
        assert!(
            active.is_empty(),
            "未持父 ⟹ ShortDiff 子腿被 AncOK 剪枝（639(c)：不开 naked 逆势仓）；实得 {active:?}"
        );
        assert_eq!(p, 0.0, "ShortDiff 剔除 ⟹ 无活动腿 ⟹ p̃=0（不开仓）");
    }

    /// ★测试③（根级无父要求）：Ambient 根候选（缺塔/host 是根，σ_p=0）⟹ 空持仓也正常准入。
    #[test]
    fn ancok_admits_ambient_root_without_held_parent() {
        let reg = super::super::persistent::PersistentRegistry::new();
        use super::super::interp::{assemble_gamma_with_tower, coverage_elements_with_tower, interpret};
        let tower: Vec<Rc<Vec<LeveledMove>>> = Vec::new(); // 缺塔 ⟹ 候选父=∂ ⟹ Ambient 根
        let classification = Classification {
            levels: vec![LevelState { bsp: vec![buy_bsp(4)], ..Default::default() }],
        };
        let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
        let gamma = assemble_gamma_with_tower(&classification, &tower);
        assert_eq!(gamma[0].role.v, Vertical::Ambient, "缺塔 ⟹ 候选父=∂ ⟹ Ambient 根");
        let buckets = interpret(&gamma, &[]); // 空持仓
        let (active, p) = coverage_step_from_buckets(view_split(&elements, cstart), &[], &buckets, 1000.0, &cfg(), &reg);
        assert_eq!(active.len(), 1, "Ambient 根腿无父要求 ⟹ 空持仓也准入");
        assert_eq!(active[0].dir, VoiceSide::Long);
        assert!((p - 600.0).abs() < 1e-9, "根 depth 0 ⟹ p̃=base×w[0]=600");
    }

    /// ★测试④（持父→撤父→连带剪枝，覆盖不漂浮）：先持父仓准入 ShortDiff 子腿，下一 bar 父腿被反向
    /// 关闭（𝒟_x）⟹ 子腿同 bar 失祖先 ⟹ AncOK 连带剪枝（spec §13 覆盖不漂浮）。
    #[test]
    fn ancok_prunes_child_when_parent_closed_same_step() {
        use super::super::interp::{assemble_gamma_with_tower, coverage_elements_with_tower, interpret};
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        // 同 bar 两候选：L1 卖（反向关闭 L1 Long 父腿）+ L0 卖（ShortDiff 子腿）。
        let classification = Classification {
            levels: vec![
                LevelState { bsp: vec![sell_bsp(8)], ..Default::default() }, // L0：ShortDiff 子
                LevelState { bsp: vec![sell_bsp(12)], ..Default::default() }, // L1：反向关父
            ],
        };
        let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
        let gamma = assemble_gamma_with_tower(&classification, &tower);
        // 持父仓（L1 Long，ID=(1,0) 与塔 compose 元素同 ID）。
        let held_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 12, lambda: 0,
            id: eid(1, 0), parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let buckets = interpret(&gamma, &[held_parent]);
        // L1 卖反向关闭 L1 Long 父腿（𝒟_x），故 (A_t∖𝒟_x) 不含父 ⟹ 子腿失祖先 ⟹ AncOK 连带剪枝。
        assert!(buckets.close.iter().any(|l| l.level == 1), "L1 卖反向关闭 L1 Long 父腿（𝒟_x）");
        let reg = super::super::persistent::PersistentRegistry::new();
        let (active, _p) =
            coverage_step_from_buckets(view_split(&elements, cstart), &[held_parent], &buckets, 1000.0, &cfg(), &reg);
        assert!(
            !active.iter().any(|l| l.level == 0 && l.dir == VoiceSide::Short),
            "父腿同 bar 关闭 ⟹ ShortDiff 子腿连带剪枝（覆盖不漂浮）；实得 {active:?}"
        );
    }

    /// ★工位 G 引擎自举测试（depth>0 准入 bootstrap，H2 裁决）：level-(ℓ+1) BSP 确认 ⟹ 其 **host
    /// 容器** 同 bar 开腿入 raw（容器作 §8 σ_r 根声部持仓）⟹ 同 bar level-ℓ ShortDiff 子候选的真
    /// Compose 父（= 该容器）在 raw ⟹ AncOK 准入 depth>0 子腿。**空 prev_active 即可产 depth>0**，
    /// 不再依赖外部预注入持仓父腿（死循环根因消除）。
    ///
    /// 死循环根因（H2 settled）：旧引擎 open 集 100% 来自 BSP 叶子点（lambda==rho==source_index，
    /// 挂容器**之下**作叶子），容器自身从不入 next_active ⟹ 永不成持仓腿 ⟹ depth>0 子腿父永不在
    /// raw ⟹ AncOK 永剪。本测试断言：容器自身的 BSP 确认时，容器作根声部开腿（§8 σ_r=最高级别=持仓）。
    ///
    /// 定义依据：pasted-text §8（根声部 σ_r=持仓）+ §9 祖先闭合（子声部激活 ⟹ 父声部激活持仓=AncOK）。
    #[test]
    fn engine_bootstrap_container_bsp_admits_depth_child_from_empty() {
        let reg = super::super::persistent::PersistentRegistry::new();
        // per-bar 因果塔：L1 Long 父走势（compose ρ=12，idx0）+ 3 L0 子（idx1/2/3）。
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        // 两个 BSP 同 bar：
        //  - L1 容器自身的卖点（src=12=容器 ρ，host=L1 容器，是根 ⟹ Ambient 根腿，AncOK 无父要求准入）。
        //  - L0 卖点（src=8=sub(4,8) ρ，host=sub，真父=L1 容器 ⟹ ShortDiff depth=1 子腿）。
        let classification = Classification {
            levels: vec![
                LevelState { bsp: vec![sell_bsp(8)], ..Default::default() },  // L0：ShortDiff 子
                LevelState { bsp: vec![sell_bsp(12)], ..Default::default() }, // L1：容器自身买卖点
            ],
        };
        // ★空 prev_active：无外部预注入持仓父腿（死循环场景）。
        let (active, _p) =
            coverage_step_classification(&classification, &tower, &[], 1000.0, &cfg(), &reg);
        // 自举后：L1 容器腿开（其 BSP 确认）+ L0 ShortDiff 子腿准入（父=L1 容器在 raw）。
        assert!(
            active.iter().any(|l| l.level == 1),
            "L1 容器自身 BSP 确认 ⟹ 容器开腿（§8 σ_r 根声部持仓）；实得 {active:?}"
        );
        assert!(
            active.iter().any(|l| l.level == 0 && l.dir == VoiceSide::Short),
            "容器在 raw ⟹ L0 ShortDiff depth>0 子腿 AncOK 准入（空 prev_active 即产 depth>0）；实得 {active:?}"
        );
    }

    /// ★工位 G 自举非膨胀守卫（639(c) 保护）：**没有**容器自身 BSP 时，孤立 L0 ShortDiff 候选仍被剪枝
    /// （不开 naked 逆势仓）。自举只在容器自身买卖点确认时开容器腿，不无条件放行所有 ShortDiff。
    #[test]
    fn engine_bootstrap_does_not_admit_orphan_shortdiff_without_container_bsp() {
        let reg = super::super::persistent::PersistentRegistry::new();
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        // 仅 L0 ShortDiff 候选，**无** L1 容器 BSP ⟹ 容器不开腿 ⟹ 子腿父不在 raw ⟹ 剪枝（639(c)）。
        let classification = Classification {
            levels: vec![LevelState { bsp: vec![sell_bsp(8)], ..Default::default() }],
        };
        let (active, p) =
            coverage_step_classification(&classification, &tower, &[], 1000.0, &cfg(), &reg);
        assert!(
            active.is_empty(),
            "无容器 BSP ⟹ ShortDiff 子腿仍剪枝（自举不膨胀，639(c) 保护）；实得 {active:?}"
        );
        assert_eq!(p, 0.0, "孤立 ShortDiff 剪枝 ⟹ p̃=0");
    }

    /// ★工位 H 跨 bar 持仓父链测试（depth>0 准入的**真**生产场景，非同 bar 共现）：
    ///
    /// 真实数据（L2 诊断坐实）：L1 容器 BSP 极稀疏（全窗仅 ~5 个），几乎从不与 L0 子 BSP 同 bar 共现
    /// ⟹ G 的同 bar 自举（`engine_bootstrap_*`）结构上几乎不触发 ⟹ active_depth1=0。真实场景是：
    /// **bar t 容器 BSP 开容器腿 → 容器腿跨 bar 持有 → bar t+1（无 L1 BSP）L0 子 ShortDiff 借持仓容器
    /// 准入**（§8 σ_r 持仓跨 bar，§9 祖先=持仓父在 A_t）。
    ///
    /// 本测试用**两 bar 序列**复现真实路径：
    /// - bar1：仅 L1 容器卖点（src=12）⟹ 容器腿开（根，§8 σ_r）。
    /// - bar2：仅 L0 子卖点（src=8，**无** L1 BSP）+ prev_active=bar1 的容器腿 ⟹ L0 ShortDiff 子腿
    ///   的真 Compose 父（= 持仓容器腿）在 A_t ⟹ AncOK 准入（639(c) 兑现：父**已持仓**才放行）。
    ///
    /// **RED（修复前）**：bar2 的 host 注入用 `(c.level=0, c.source_index=8)` = L0 子 host 自身（叶子），
    /// 容器（L1，parent_id 指向）从不在 raw；prev_active 容器腿对位回树后入 raw，但 G 的注入逻辑不补
    /// 容器到 raw——实际上**持仓容器腿**（prev_active）才是父在场源。若持仓对位生效，bar2 应准入子腿。
    ///
    /// 定义依据：639(c)（持仓准入 §13 AncOK，父在 A_t 才放行——与本测试断言一致，非膨胀）；
    /// §8 σ_r 根声部持仓跨 bar；§9 祖先闭合（子激活 ⟹ 持仓父在场）。
    #[test]
    fn cross_bar_held_container_admits_depth_child_next_bar() {
        let reg = super::super::persistent::PersistentRegistry::new();
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        // bar1：仅 L1 容器卖点（src=12=容器 ρ）⟹ 容器腿开（根，§8 σ_r 持仓）。
        let bar1 = Classification {
            levels: vec![
                LevelState::default(),                                    // L0：无 BSP
                LevelState { bsp: vec![sell_bsp(12)], ..Default::default() }, // L1：容器自身卖点
            ],
        };
        let (active1, _p1) =
            coverage_step_classification(&bar1, &tower, &[], 1000.0, &cfg(), &reg);
        assert!(
            active1.iter().any(|l| l.level == 1),
            "bar1：L1 容器 BSP ⟹ 容器腿开（§8 σ_r 持仓根）；实得 {active1:?}"
        );
        // registry merge（生产 instrument_loop 同序）：跨 bar 持久身份。
        let (elements1, _c1) =
            super::super::interp::coverage_elements_with_tower(&bar1, &tower);
        let reg2 = reg.merge(&elements1, &active1);

        // bar2：仅 L0 子卖点（src=8，**无** L1 BSP）+ prev_active=bar1 容器腿。
        let bar2 = Classification {
            levels: vec![LevelState { bsp: vec![sell_bsp(8)], ..Default::default() }],
        };
        let (active2, _p2) =
            coverage_step_classification(&bar2, &tower, &active1, 1000.0, &cfg(), &reg2);
        // 持仓容器腿（prev_active）= L0 子腿真 Compose 父在 A_t ⟹ AncOK 准入 depth>0 子腿。
        assert!(
            active2.iter().any(|l| l.level == 0 && l.dir == VoiceSide::Short),
            "bar2：L0 ShortDiff 子腿借跨 bar 持仓容器准入（639(c) 父已持仓放行）；实得 {active2:?}"
        );
    }

    /// ★(I-1) open 候选父注入测试（depth>0 准入的**真**生产场景，codex 行级裁决坐实，642/644）：
    ///
    /// L2 诊断坐实根因：父 carrier **不在 prev_active 持仓腿**（sd_parent_held=0），也几乎不与子同 bar
    /// 共现（容器 BSP 极稀疏），但**在 persistent registry LiveDetached 存活**（sd_parent_registry_alive
    /// ≈51525）。`cross_bar_held_container_*` 测的是父作 prev_active 持仓腿的路径——**不**覆盖此真实场景。
    /// 本测试覆盖：**prev_active 为空、父仅 registry-live** ⟹ open 候选父注入必须从 registry 恢复祖先链
    /// 才能让子腿 AncOK 准入。
    ///
    /// **RED（修复前）**：open 候选路径从不调 `restore_ancestor_chain_from_registry` ⟹ 父 carrier 不在
    /// raw（既非 held 又非 open 候选自身）⟹ ancestor_close_by_id 判子声部祖先不齐 ⟹ AncOK 全剪 ⟹
    /// active 不含 L0 ShortDiff 子腿。**GREEN（修复后）**：open 候选父 parent_id registry-live ⟹ 恢复父
    /// carrier 祖先链入 raw ⟹ 子腿准入。
    ///
    /// 定义依据：级别容器.pdf §13（AncOK 持仓准入，父在 A_t 放行）+ anc.pdf §11（操作父 live ⟹ depth<d
    /// 祖先全在 raw）；§8 σ_r 父声部 carrier 跨 bar 持有（registry LiveDetached = 操作上仍持有）。
    #[test]
    fn open_candidate_parent_injected_from_registry_admits_depth_child() {
        let reg = super::super::persistent::PersistentRegistry::new();
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        // bar1：L1 容器卖点（src=12）⟹ 容器腿开 ⟹ merge 把容器 carrier 写入 registry（LiveDetached 源）。
        let bar1 = Classification {
            levels: vec![
                LevelState::default(),
                LevelState { bsp: vec![sell_bsp(12)], ..Default::default() },
            ],
        };
        let (active1, _p1) =
            coverage_step_classification(&bar1, &tower, &[], 1000.0, &cfg(), &reg);
        let (elements1, _c1) =
            super::super::interp::coverage_elements_with_tower(&bar1, &tower);
        let reg2 = reg.merge(&elements1, &active1);

        // bar2：仅 L0 子卖点（src=8，**无** L1 BSP）+ **prev_active 为空**（父 carrier 非持仓腿）。
        // 父 carrier 仅在 reg2 中 LiveDetached 存活 ⟹ 唯有 open 候选父注入恢复祖先链才能准入子腿。
        let bar2 = Classification {
            levels: vec![LevelState { bsp: vec![sell_bsp(8)], ..Default::default() }],
        };
        let (active2, _p2) =
            coverage_step_classification(&bar2, &tower, &[], 1000.0, &cfg(), &reg2);
        assert!(
            active2.iter().any(|l| l.level == 0 && l.dir == VoiceSide::Short),
            "bar2：L0 ShortDiff 子腿借 registry-live 父 carrier（非持仓腿）经 open 父注入准入 depth>0；实得 {active2:?}"
        );
        // ★(I-1) 双计守卫（codex 异质审查）：父 carrier 在 bar2 树前缀中也存在 ⟹ restore 须复用现有 idx，
        // 不得 push 重复 id ⟹ next_active 每 ElementId 唯一（否则 p̃ 双计）。
        let mut ids: Vec<_> = active2.iter().map(|l| l.id).collect();
        let n = ids.len();
        ids.sort_by_key(|id| (id.level, id.ordinal));
        ids.dedup();
        assert_eq!(ids.len(), n, "next_active 含重复 ElementId（restore 未复用现有 idx）⟹ p̃ 双计；实得 {active2:?}");
    }

    /// ★(I-1) **双计根因直测**（codex HIGH 行级坐实，644）：`restore_ancestor_chain_from_registry` 在
    /// 祖先**已存在于 work 树前缀**（如 carrier 走势）但未入 raw 时，必须**复用其现有 idx**，不得 push
    /// 重复 id 新元素。
    ///
    /// **RED（修复前 push 新元素）**：carrier id 已在 `work[0]`，restore 从 registry 取同 id 又 push 到
    /// `work[1]` ⟹ `work.len()==2`、`raw==[1]` 指向重复 id ⟹ 同一 carrier 在 next_idx 产两条 leg ⟹
    /// strategy_target_legs/net_target_units 双计 p̃、next_active 含两条同 id 腿（伪证仓位规模）。
    /// **GREEN（修复后复用现有 idx）**：restore 查得 `work[0].id==pid` ⟹ `raw==[0]`、`work.len()==1`
    /// 不增 ⟹ 每 id 在 raw 中唯一表示（spec §13 元素集语义）。
    ///
    /// 定义依据：codex 异质审查 HIGH（restore 只查 raw 不查 work 树前缀 ⟹ 重复 id）；spec §13 元素集
    /// （同一 ElementId 在 A_t 唯一）。
    #[test]
    fn restore_reuses_existing_work_idx_no_duplicate_id() {
        let carrier = eid(1, 0);
        // work 树前缀已含 carrier（如跨 bar 持仓的父声部走势对位回当前树）。
        let base = vec![CoverageElement {
            lambda: 0,
            rho: 12,
            eps: VoiceSide::Short,
            level: 1,
            parent: None,
            attached_dir: None,
            id: carrier,
            parent_id: None,
        }];
        let mut work = ElementView::new(&base);
        let mut raw: Vec<usize> = Vec::new();
        // registry 持有同一 carrier id（LiveDetached：snapshot_present 经下一 bar 增量重置为 false）。
        let reg = super::super::persistent::PersistentRegistry::new().merge(&base, &[]);

        let id_idx = build_tree_id_index(&base);
        let mut overlay_seen = std::collections::HashMap::new();
        restore_ancestor_chain_from_registry(&mut work, &mut raw, &reg, carrier, &id_idx, &mut overlay_seen);

        assert_eq!(work.len(), 1, "restore 不得 push 重复 id 元素（应复用 work[0]，overlay 空）");
        assert_eq!(raw, vec![0], "raw 须复用现有 idx 0，非追加新 idx");
        let dup = raw.iter().filter(|&&r| work[r].id == carrier).count();
        assert_eq!(dup, 1, "carrier 在 raw 中须唯一表示（双计根因守卫）");
    }

    /// ★(I-1) open 父注入非膨胀守卫：父 carrier **不在 registry**（既非持仓又非 registry-live）⟹ 子腿
    /// 仍被剪枝。open 父注入只在父真实 live 时恢复祖先链，不无条件放行（no-patch：非 AncOK 加特例）。
    #[test]
    fn open_candidate_parent_not_in_registry_still_pruned() {
        let reg = super::super::persistent::PersistentRegistry::new();
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        // 空 registry（父 carrier 从未出现在任何 snapshot）+ 仅 L0 ShortDiff 子卖点 + 空 prev_active。
        let bar = Classification {
            levels: vec![LevelState { bsp: vec![sell_bsp(8)], ..Default::default() }],
        };
        let (active, p) =
            coverage_step_classification(&bar, &tower, &[], 1000.0, &cfg(), &reg);
        assert!(
            active.is_empty(),
            "父 carrier 不在 registry ⟹ open 父注入不恢复 ⟹ 子腿仍剪枝（非膨胀）；实得 {active:?}"
        );
        assert_eq!(p, 0.0, "孤立 ShortDiff（父不可恢复）剪枝 ⟹ p̃=0");
    }

    /// ★codex Q4 发现 A 修复测试：Stale 非边界根被 prune（非伪造 parent:None root）。
    ///
    /// 持仓腿 ID 不在当前因果树（Stale）+ `is_boundary_root=false`（非真边界根 ∂）⟹ prune（不入 raw），
    /// AncOK 严格 §13 line 671。旧逻辑伪造 `parent:None` ⟹ AncOK 恒等放行（放宽 spec §13）。
    /// Q4 修复：保留原 `is_boundary_root`，非边界根 Stale = prune。
    #[test]
    fn stale_non_boundary_root_is_pruned_not_fabricated_root() {
        use super::super::interp::{assemble_gamma_with_tower, coverage_elements_with_tower, interpret};
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        let classification = Classification {
            levels: vec![LevelState { bsp: vec![sell_bsp(8)], ..Default::default() }],
        };
        let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
        let gamma = assemble_gamma_with_tower(&classification, &tower);
        // 持仓腿：ID=(99,99) 不在当前因果树（Stale）+ is_boundary_root=false（非真边界根 ∂）。
        // parent_id=Some((1,0)) 表示它本应有父（非 ∂ 根），但父不在当前树。
        let stale_non_root = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 12, lambda: 0,
            id: eid(99, 99), parent_id: Some(eid(1, 0)), is_boundary_root: false, op_parent: Some(eid(1, 0)),
        };
        let buckets = interpret(&gamma, &[stale_non_root]);
        let reg = super::super::persistent::PersistentRegistry::new();
        let (active, p) =
            coverage_step_from_buckets(view_split(&elements, cstart), &[stale_non_root], &buckets, 1000.0, &cfg(), &reg);
        // Stale 非边界根 ⟹ prune（不入 raw）⟹ 不在 A_{t+1}。
        assert!(
            !active.iter().any(|l| l.id == eid(99, 99)),
            "Stale 非边界根被 prune（非伪造 root，spec §13 严格）；实得 {active:?}"
        );
        // p̃ 不含该腿（pruned ⟹ 不贡献）。
        let _ = p; // p̃ 可非零（若 ShortDiff 候选准入），关键是 stale_non_root 不在 active。
    }

    /// ★codex Q4 确定性 ElementId 跨 bar 稳定测试：全量/增量产同 ID。
    #[test]
    fn element_id_deterministic_full_vs_incremental() {
        // 全量 compose_level 产 ID (level, ordinal) 从 0 起。
        let units: Vec<UnitRange> = (0..9)
            .map(|i| unit(i * 4, i * 4 + 4, if i % 2 == 0 { Direction::Up } else { Direction::Down }, 0, 100))
            .collect();
        let moves: Vec<LeveledMove> = units
            .iter()
            .enumerate()
            .map(|(i, u)| LeveledMove::from_unit(u, ElementId { level: 0, ordinal: i as u64 }))
            .collect();
        let (_fc, full_upper) = compose_level(&units, &moves, true, 1);
        // 增量 resume(prefix_count=0) == 全量。
        let (_tc, tail_upper, _) = compose_level_resume(&units, &moves, true, 1, 0, 0);
        assert_eq!(full_upper.len(), tail_upper.len());
        for (f, t) in full_upper.iter().zip(tail_upper.iter()) {
            assert_eq!(f.id, t.id, "全量/增量产同 ElementId（确定性）");
        }
        // 增量续扫：前 6 段 + 追加 3 段，tail ID 接续前缀（prefix_count + i）。
        let (_pc, prefix_upper, cursor6) = compose_level_resume(&units[..6], &moves[..6], true, 1, 0, 0);
        let (_tc2, tail_upper2, _) = compose_level_resume(&units, &moves, true, 1, cursor6.consumed, prefix_upper.len());
        let mut comb = prefix_upper.clone();
        comb.extend(tail_upper2);
        assert_eq!(comb.len(), full_upper.len());
        for (f, c) in full_upper.iter().zip(comb.iter()) {
            assert_eq!(f.id, c.id, "增量续扫 ID 接续前缀（全量/增量产同 ID）");
        }
    }

    // ── §9 环7 π_Θ：J_x + 𝒦_Θ + LexArgmin + Schedule_Θ → 唯一订单 ──────────────

    fn rcfg() -> RiskConfig {
        RiskConfig::default() // rho=0.005, beta=0.5, gamma=1.0, kappa=2.0, default_lot=1
    }

    /// PiThetaWeights::from_risk 复用 κ/ρ（不引新参数）。
    #[test]
    fn pi_theta_weights_reuse_kappa_rho() {
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        assert_eq!(w.w, 1.0);
        assert_eq!(w.lambda, r.kappa);
        assert_eq!(w.nu, r.rho);
    }

    /// ★𝒦_Θ≠∅：可行集恒含安全锚 0（非空性构造见证，spec line 725 硬前提）。
    #[test]
    fn feasible_set_nonempty_contains_zero() {
        let cands = feasible_candidates(600.0, 0.0, 1000.0, 1000.0, 1.0);
        assert!(!cands.is_empty(), "𝒦_Θ 非空");
        assert!(cands.iter().any(|&(p, _)| p.abs() < 1e-9), "含安全锚 0");
        // cap<lot 退化：hi=0 ⟹ 𝒦_Θ={0} 仍非空。
        let degen = feasible_candidates(5.0, 0.0, 0.4, 0.4, 1.0);
        assert!(!degen.is_empty());
        assert!(degen.iter().all(|&(p, _)| p.abs() < 1e-9), "cap<lot ⟹ 𝒦_Θ={{0}}");
    }

    /// ★候选全 lot 对齐 ∈[−hi,hi] + grid_index 升序单射（手数约束 + 固定平局规则）。
    #[test]
    fn feasible_candidates_lot_aligned_within_cap() {
        let (lot, cap) = (10.0, 100.0);
        let cands = feasible_candidates(23.0, 0.0, cap, cap, lot);
        for &(p, _) in &cands {
            assert!((p / lot).fract().abs() < 1e-9, "lot 对齐");
            assert!(p.abs() <= cap + 1e-9, "∈[−cap,cap]");
        }
        for i in 1..cands.len() {
            assert!(cands[i].1 > cands[i - 1].1 && cands[i].0 > cands[i - 1].0, "升序单射");
        }
    }

    /// ★p* 跟踪 p̃（cap 内）：p̃=600 在 cap=1000 内 ⟹ p*=600（主键跟踪误差 0）。
    #[test]
    fn pi_theta_position_tracks_ptilde_within_cap() {
        let r = rcfg();
        let p_star =
            pi_theta_position(600.0, 0.0, 1000.0, &r, PiThetaWeights::from_risk(&r), KThetaRiskGate::open());
        assert!((p_star - 600.0).abs() < 1e-9, "p* = p̃ = 600（cap 内跟踪）");
    }

    /// ★杠杆/资本 cap binding：p̃=1500 超 cap=1000 ⟹ p*=1000（±hi 截断，结论翻转）。
    #[test]
    fn pi_theta_position_clamps_over_cap() {
        let r = rcfg(); // base_units=1000（U_ℓ）, gamma=1.0（γ̄）⟹ cap=U_ℓ·γ̄=1000
        let p_star =
            pi_theta_position(1500.0, 0.0, 1000.0, &r, PiThetaWeights::from_risk(&r), KThetaRiskGate::open());
        assert!((p_star - 1000.0).abs() < 1e-9, "p* = +hi = cap = 1000（杠杆/资本 cap binding）");
    }

    /// ★lot 取整：lot=10、p̃=23 ⟹ p*=20（最近 lot，|23−20|<|23−30|）。
    #[test]
    fn pi_theta_position_lot_rounds_to_nearest() {
        let mut r = rcfg();
        r.default_lot = 10;
        let p_star =
            pi_theta_position(23.0, 0.0, 1000.0, &r, PiThetaWeights::from_risk(&r), KThetaRiskGate::open());
        assert!((p_star - 20.0).abs() < 1e-9, "p* = 20（最近 lot 点）");
    }

    /// ★Schedule_Θ 开仓：空仓 → p*=±600 ⟹ Buy/Sell 600。
    #[test]
    fn schedule_open_from_flat() {
        let o = schedule_order(600.0, 0.0, 7);
        assert_eq!((o.action, o.qty, o.exec_index), (StrictAction::Buy, 600, 7));
        assert_eq!(schedule_order(-600.0, 0.0, 7).action, StrictAction::Sell);
    }

    /// ★Schedule_Θ 平仓：持多 p_t=600 → p*=0 ⟹ Close 600。
    #[test]
    fn schedule_close_to_flat() {
        let o = schedule_order(0.0, 600.0, 3);
        assert_eq!((o.action, o.qty), (StrictAction::Close, 600));
    }

    /// ★Schedule_Θ 增/减持：同号幅度增=Add、减=Reduce（多空对称）。
    #[test]
    fn schedule_add_reduce() {
        assert_eq!(schedule_order(900.0, 600.0, 0).action, StrictAction::Add);
        assert_eq!(schedule_order(900.0, 600.0, 0).qty, 300);
        assert_eq!(schedule_order(300.0, 600.0, 0).action, StrictAction::Reduce);
        assert_eq!(schedule_order(-900.0, -600.0, 0).action, StrictAction::Add); // 更空=Add
    }

    /// ★Schedule_Θ 无交易（全函数）：Δ=0 ⟹ 持仓 Hold / 空仓 Wait（qty=0）。
    #[test]
    fn schedule_hold_wait_no_trade() {
        assert_eq!(schedule_order(600.0, 600.0, 0).action, StrictAction::Hold);
        assert_eq!(schedule_order(600.0, 600.0, 0).qty, 0);
        assert_eq!(schedule_order(0.0, 0.0, 0).action, StrictAction::Wait);
    }

    /// ★Schedule_Θ 反号穿零（净反转）：持多 p_t=600 → p*=−200 ⟹ Sell 800。
    #[test]
    fn schedule_sign_flip_reverse() {
        let o = schedule_order(-200.0, 600.0, 0);
        assert_eq!((o.action, o.qty), (StrictAction::Sell, 800));
    }

    /// ★全链 π_Θ（GAP-5）：买点 Γ 入场 → π_Θ → Buy；入场源=买卖点 source_index（非走势边界）。
    #[test]
    fn pi_theta_step_buy_point_entry_gap5() {
        let bsp = BspPoint {
            source_index: 4,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90,
            pivot_high: 0,
            center: Some(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 }),
        };
        let classification = Classification {
            levels: vec![LevelState { bsp: vec![bsp], ..Default::default() }],
        };
        let r = rcfg();
        let (active, p_star, order) = pi_theta_step(
            &classification, &[], &[], 0.0, 5, 1000.0, &cfg(), &r, PiThetaWeights::from_risk(&r),
            KThetaRiskGate::open(), &super::super::persistent::PersistentRegistry::new(),
        );
        // GAP-5：活动腿 source_index = BspPoint.source_index = 4（买卖点，非 LeveledMove.start_index）。
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].source_index, 4, "GAP-5：入场源=买卖点 source_index");
        assert_eq!(active[0].dir, VoiceSide::Long);
        assert!((p_star - 600.0).abs() < 1e-9, "p̃=600 cap 内 ⟹ p*=600");
        assert_eq!((order.action, order.qty, order.exec_index), (StrictAction::Buy, 600, 5));
    }

    /// ★∀x ∃! O_{t+1}（spec §16）：同输入 ⟹ 同订单 + 同 p*（确定唯一）。
    #[test]
    fn pi_theta_step_deterministic_unique_order() {
        let bsp = BspPoint {
            source_index: 0,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90,
            pivot_high: 0,
            center: Some(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 }),
        };
        let classification = Classification {
            levels: vec![LevelState { bsp: vec![bsp], ..Default::default() }],
        };
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let a = pi_theta_step(&classification, &[], &[], 0.0, 1, 1000.0, &cfg(), &r, w, KThetaRiskGate::open(), &super::super::persistent::PersistentRegistry::new());
        let b = pi_theta_step(&classification, &[], &[], 0.0, 1, 1000.0, &cfg(), &r, w, KThetaRiskGate::open(), &super::super::persistent::PersistentRegistry::new());
        assert_eq!(a.2, b.2, "∀x ∃! O_{{t+1}}：确定唯一订单");
        assert_eq!(a.1, b.1);
    }

    /// ★Q2 close_pred 折 𝒦_Θ：force_flat ⟹ 𝒦_Θ={0} ⟹ p*=0（强平，单一决策出口产平仓 O）。
    #[test]
    fn pi_theta_position_force_flat_gate_clamps_to_zero() {
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        // 无门：p̃=600 cap 内 ⟹ p*=600。force_flat 门：𝒦_Θ={0} ⟹ p*=0（不论 p̃）。
        let open = pi_theta_position(600.0, 600.0, 1000.0, &r, w, KThetaRiskGate::open());
        assert!((open - 600.0).abs() < 1e-9, "全开门 ⟹ p*=600");
        let flat = KThetaRiskGate { force_flat: true, stop_long: false, stop_short: false };
        let p_star = pi_theta_position(600.0, 600.0, 1000.0, &r, w, flat);
        assert_eq!(p_star, 0.0, "force_flat ⟹ 𝒦_Θ={{0}} ⟹ p*=0（持仓 600 → Close）");
    }

    /// ★Q2 stop_long 门：禁净多 ⟹ 持多 p_t=600 + p̃=600（信号仍要多）⟹ p*=0（止损经 𝒦_Θ 平多，
    /// 非第二 exit 出口）。stop_short 不触发 ⟹ 净空仍可（对称见证）。
    #[test]
    fn pi_theta_position_stop_long_gate_forbids_net_long() {
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let gate = KThetaRiskGate { force_flat: false, stop_long: true, stop_short: false };
        // 持多 600 + 信号要多 600，但 stop_long 禁净多 ⟹ 𝒦_Θ⊆[−cap,0] ⟹ p*=0（平多，单出口）。
        let p_star = pi_theta_position(600.0, 600.0, 1000.0, &r, w, gate);
        assert!(p_star <= 1e-9, "stop_long ⟹ 禁净多 ⟹ p*≤0（止损平多走 𝒦_Θ，非第二出口），实得 {p_star}");
    }

    /// ★执行层 σ_p = 父容器方向（639，端到端 pi_theta_step；取代旧"活动父腿"错口径测试）：
    /// per-bar 因果塔含有向 L1 Long 父走势 + L0 逆向卖候选 ⟹ 候选 V=ShortDiff（来自**父容器方向**，
    /// **非持仓**——空 `prev_active` 仍 ShortDiff，坐实 σ 来源 ⊥ 持仓）。
    ///
    /// ★两机制正交端到端坐实（639，AncOK 已接入）：① **σ_p 来源**=ShortDiff（`assemble_gamma_with_tower`，
    /// 不接受 active ⟹ 与持仓无关）；② **§13 AncOK 持仓准入**=空 `prev_active`（未持父）⟹ ShortDiff 子腿
    /// 被剪枝 ⟹ pi_theta_step 产 **Wait/qty=0**（不开 naked 逆势仓，639(c)）。σ 仍分类 ShortDiff 但准入
    /// 剔除——正是两正交机制（σ 用因果塔，准入用持仓台账）。
    #[test]
    fn pi_theta_step_shortdiff_from_parent_container_not_position() {
        use super::super::interp::assemble_gamma_with_tower;
        // per-bar 因果塔：L1 Long 父走势（结构对象；3 个 L0 子，sub(8,12) 右端点 ρ=12）。
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        // L0 卖候选 source_index=12 ⟹ host=sub(8,12)（ρ=12）⟹ 真父 L1 Long ⟹ σ_p=Long。
        let sell = BspPoint {
            source_index: 12,
            bits: BspBits { sell1: true, ..Default::default() },
            pivot_low: 0, pivot_high: 210,
            center: Some(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 12 }),
        };
        let classification = Classification {
            levels: vec![LevelState { bsp: vec![sell], ..Default::default() }],
        };
        // ★639 核心坐实：空 prev_active（未持仓）+ 有向父容器 ⟹ ShortDiff（σ_p=父容器方向，非持仓父腿）。
        let gamma = assemble_gamma_with_tower(&classification, &tower);
        assert_eq!(gamma[0].dir, VoiceSide::Short);
        assert_eq!(
            gamma[0].role.v,
            Vertical::ShortDiff,
            "L0 卖 δ=Short = −σ_p（父容器 L1 Long）⟹ ShortDiff（639：来自父容器方向，未持仓仍成立）"
        );
        // 端到端 pi_theta_step（GAP-5 入场 + 父容器 σ_p + §13 AncOK 持仓准入 + 风控门全开）。
        // 空 prev_active（未持父）⟹ ShortDiff 子腿被 AncOK 剪枝 ⟹ Wait/qty=0（639(c)：不开 naked 逆势仓）。
        let r = rcfg();
        let (a, p_star, order) = pi_theta_step(
            &classification, &tower, &[], 0.0, 9, 1000.0, &cfg(), &r,
            PiThetaWeights::from_risk(&r), KThetaRiskGate::open(), &super::super::persistent::PersistentRegistry::new(),
        );
        assert!(a.is_empty(), "未持父 ⟹ ShortDiff 子腿 AncOK 剪枝 ⟹ A_{{t+1}} 空");
        assert_eq!(p_star, 0.0, "无活动腿 ⟹ p*=0（不开仓）");
        assert_eq!(order.action, StrictAction::Wait, "未持父 ⟹ ShortDiff 剔除 ⟹ Wait（639(c)）");
        assert_eq!(order.qty, 0, "Wait ⟹ qty=0（不建 naked 逆势仓）");
        assert_eq!(order.exec_index, 9, "订单携 exec_index（延迟成交 bar）");
    }

    /// **★工位 4d L1 bit-exact 守卫：注入缓存 base 索引 vs fallback 现建逐 bar 对拍**（真实 CL）。
    ///
    /// 热点①（`strategy_target_legs` 兄弟索引）+ ②（`held_leg_tree_index` ID 索引）改为 base 段复用
    /// 缓存 `Rc` 索引（runner 从 TreeCache 注入）。本守卫逐 bar 跑**两条并行账本**：
    /// - `with`：`ElementView::with_base_indices` 注入缓存 sibling/id 索引（生产路径）。
    /// - `without`：不注入 ⟹ `coverage_step_from_buckets` fallback 现建（旧路径）。
    /// 两路径各自独立演进 `prev_active`，断言每 bar `(next_active, p_tilde)` 逐字节相等。任何缓存路径
    /// 与现建路径的发散（split partition_point ≠ 合并 partition_point、id_idx 覆盖范围错位）立即捕获。
    ///
    /// 认识论 L1（formalization-validity-domain 231号）：管线正确性验证（增量缓存 == 全量现建），
    /// 非 L2 alpha。bit-exact == 旧 [`operation_role_indexed`]/[`build_tree_id_index`] 现建逻辑。
    #[test]
    #[ignore = "工位 4d L1 bit-exact：注入缓存 vs 现建逐 bar 对拍；需 CL；--release --ignored"]
    fn bit_exact_cached_indices_vs_fallback() {
        use super::super::super::backtest::data;
        use super::super::super::config::ThetaConfig;
        use super::super::super::{classifier, parser};
        use super::super::interp::{self, TreeCache};
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let n = 4000.min(oos.bars.len());
        let voice = config.voice.clone();
        let mut cache = TreeCache::new();
        let mut reg_with = super::super::persistent::PersistentRegistry::new();
        let mut reg_without = super::super::persistent::PersistentRegistry::new();
        let mut prev_with: Vec<ActiveLeg> = Vec::new();
        let mut prev_without: Vec<ActiveLeg> = Vec::new();
        let mut hits = 0usize;
        for i in 0..n {
            let l0 = parser::parse_layer(&oos.bars[..=i], &config);
            let (cls, tower) = classifier::classify_with_tower(&l0, &config);
            let (tree, candidates, gamma) =
                interp::coverage_elements_and_gamma_with_tower_cached(&cls, &tower, &mut Some(&mut cache));

            // with：注入缓存 base 索引（生产路径）。
            let mut work_with = ElementView::from_parts(&tree, candidates.clone());
            if let Some((sib, id)) = cache.tree_sibling_and_id() {
                work_with = work_with.with_base_indices(sib, id);
                hits += 1;
            }
            let (next_with, p_with) =
                coverage_step_prebuilt(work_with, &gamma, &prev_with, 1000.0, &voice, &reg_with);

            // without：不注入 ⟹ fallback 现建（旧路径）。
            let work_without = ElementView::from_parts(&tree, candidates);
            let (next_without, p_without) =
                coverage_step_prebuilt(work_without, &gamma, &prev_without, 1000.0, &voice, &reg_without);

            assert_eq!(
                next_with, next_without,
                "bar {i}: 缓存索引路径 next_active ≠ 现建路径（split partition_point/id_idx 发散）"
            );
            assert_eq!(
                p_with.to_bits(), p_without.to_bits(),
                "bar {i}: 缓存索引路径 p_tilde ≠ 现建路径（bit-exact 破裂）"
            );

            reg_with.merge_in_place(
                ElementView::from_parts(&tree, Vec::new()).as_contiguous().as_ref(), &next_with);
            reg_without.merge_in_place(
                ElementView::from_parts(&tree, Vec::new()).as_contiguous().as_ref(), &next_without);
            prev_with = next_with;
            prev_without = next_without;
        }
        eprintln!("bit_exact_cached_indices_vs_fallback: {n} bars 全部 with==without，{hits} bars 缓存命中");
    }
}
