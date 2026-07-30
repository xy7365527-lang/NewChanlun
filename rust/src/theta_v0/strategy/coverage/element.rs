use super::*;

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
    pub(crate) base: &'a [CoverageElement],
    pub(crate) overlay: Vec<CoverageElement>,
    /// ★工位 4d 热点①②：base（tree 前缀）的两个派生索引（`(parent,level)→idx` 兄弟表 + `ElementId→idx`
    /// 表），由调用方从 [`super::super::interp::TreeCache`] 注入（命中返 `Rc::clone` O(1)）。`None` ⟹ 消费者
    /// fallback 现建（[`coverage_step_from_buckets`] 测试路径/无缓存）。§16 tree 前缀不变 ⟹ 索引随
    /// tree 缓存复用，消除每 bar `build_prev_sibling_index`/`build_tree_id_index` O(tree)/bar=O(n²)。
    pub(crate) base_sibling_idx:
        Option<Rc<std::collections::HashMap<(Option<usize>, u32), Vec<usize>>>>,
    pub(crate) base_id_idx: Option<Rc<std::collections::HashMap<ElementId, usize>>>,
}

impl<'a> ElementView<'a> {
    pub(crate) fn new(base: &'a [CoverageElement]) -> Self {
        ElementView {
            base,
            overlay: Vec::new(),
            base_sibling_idx: None,
            base_id_idx: None,
        }
    }

    /// 双段构造（base=持久树前缀借用零拷贝 + overlay=本 bar candidate 段 owned）。
    /// `_cached` 返回 `(tree_rc, candidates)` 后由消费者组装——消除旧 `tree.clone()` O(tree)/bar。
    pub(crate) fn from_parts(base: &'a [CoverageElement], overlay: Vec<CoverageElement>) -> Self {
        ElementView {
            base,
            overlay,
            base_sibling_idx: None,
            base_id_idx: None,
        }
    }

    /// ★工位 4d：注入 base 段缓存索引（runner 从 [`super::super::interp::TreeCache`] 取，bit-exact 与现建相等）。
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
    pub(crate) fn len(&self) -> usize {
        self.base.len() + self.overlay.len()
    }

    /// base 段长度 = candidate_start（candidate 段从此起，在 overlay）。
    pub(crate) fn base_len(&self) -> usize {
        self.base.len()
    }

    /// candidate 段只读切片（#196 shadow：`interp::parent_certificate_projection` 的
    /// `cand_elems` 原料——gamma_index 索引本段；只读借用，与 [`Self::into_overlay`] 互补）。
    pub(crate) fn overlay(&self) -> &[CoverageElement] {
        &self.overlay
    }

    /// 取回 overlay（`_cached` 遍历2 算完 role 后取回 candidates Vec；view drop）。
    pub(crate) fn into_overlay(self) -> Vec<CoverageElement> {
        self.overlay
    }

    /// 按全局 idx 取元素（`< base.len()` 查 base，否则查 overlay），= 旧 `work.get(i)`。
    pub(crate) fn get(&self, idx: usize) -> Option<&CoverageElement> {
        if idx < self.base.len() {
            self.base.get(idx)
        } else {
            self.overlay.get(idx - self.base.len())
        }
    }

    /// 尾部追加（restore 路径恢复 registry 祖先），返新元素的全局 idx，= 旧 `work.push(e); work.len()-1`。
    pub(crate) fn push(&mut self, e: CoverageElement) -> usize {
        let idx = self.len();
        self.overlay.push(e);
        idx
    }

    /// ★#247：overlay 段元素可变借用（**仅 restore 回填角色输入用**——base 是不可变借用的树前缀，
    /// 恢复元素恒在 overlay）。idx < base.len() ⟹ None（不越权改树前缀）。
    pub(crate) fn overlay_mut(&mut self, idx: usize) -> Option<&mut CoverageElement> {
        let base_len = self.base.len();
        idx.checked_sub(base_len)
            .and_then(move |i| self.overlay.get_mut(i))
    }

    /// 首个满足 `pred` 的元素全局 idx，= 旧 `work.iter().position(pred)`（base 在前 overlay 在后）。
    pub(crate) fn position(&self, mut pred: impl FnMut(&CoverageElement) -> bool) -> Option<usize> {
        self.base.iter().position(&mut pred).or_else(|| {
            self.overlay
                .iter()
                .position(pred)
                .map(|i| self.base.len() + i)
        })
    }

    /// 全元素迭代（base 前缀 ++ overlay），= 旧 `work.iter()`。
    pub(crate) fn iter(&self) -> impl Iterator<Item = &CoverageElement> {
        self.base.iter().chain(self.overlay.iter())
    }

    /// base 前缀切片（树元素段，candidate_start 在 base 内 ⟹ 零拷贝），= 旧 `&work[..tree_end]`。
    pub(crate) fn tree_prefix(&self, tree_end: usize) -> &[CoverageElement] {
        &self.base[..tree_end.min(self.base.len())]
    }

    /// 连续切片视图（喂仍接 `&[CoverageElement]` 的 [`strategy_target_legs`]）：overlay 空 ⟹ **借 base
    /// 零拷贝**（热循环常态）；非空（restore 罕触发）⟹ materialize 一次 O(tree+overlay)。
    pub(crate) fn as_contiguous(&self) -> std::borrow::Cow<'_, [CoverageElement]> {
        if self.overlay.is_empty() {
            std::borrow::Cow::Borrowed(self.base)
        } else {
            std::borrow::Cow::Owned(
                self.base
                    .iter()
                    .chain(self.overlay.iter())
                    .copied()
                    .collect(),
            )
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
    let mut best_idx: std::collections::HashMap<ElementId, usize> =
        std::collections::HashMap::new();
    for (i, e) in elements.iter().enumerate() {
        match best_idx.get(&e.id) {
            // 已有记录：仅当新出现带真 parent_id 而旧的无父时，替换（父链更全，host^op 父链所需）。
            Some(&old) if elements[old].parent_id.is_none() && e.parent_id.is_some() => {
                best_idx.insert(e.id, i);
            }
            Some(_) => {} // 旧已带父或新也无父 ⟹ 保旧（首次出现序）。
            None => {
                best_idx.insert(e.id, i);
            }
        }
    }
    // 重建去重 Vec：选中元素按原 idx 升序（父在子前不变量保持），parent 索引重映射到去重后位置。
    let mut selected: Vec<usize> = best_idx.values().copied().collect();
    selected.sort_unstable();
    let pos_of: std::collections::HashMap<usize, usize> = selected
        .iter()
        .enumerate()
        .map(|(new, &old)| (old, new))
        .collect();
    selected
        .iter()
        .map(|&old| {
            let e = elements[old];
            CoverageElement {
                // parent（per-bar Vec 索引）重映射：旧 parent idx → 其 id 的去重后选中位置。
                parent: e.parent.and_then(|pidx| {
                    let pid = elements[pidx].id;
                    best_idx
                        .get(&pid)
                        .and_then(|&sel| pos_of.get(&sel).copied())
                }),
                ..e
            }
        })
        .collect()
}

/// ★A12 双视图双向映射一致性（648 裁决 D / 子声部.pdf §12-13 / gap-master-list P1-3）：
/// 校验结构视图树 **T_i**（[`extract_elements`]，host^struct 宇宙）与操作 carrier forest **K_i**
/// （[`extract_carrier_forest`]，host^op 宇宙）的分离不变量。`Ok(())` 或首个违反的描述。
///
/// 三组不变量（L0 代数性质，对任意塔成立）：
/// 1. **T_i ↪ K_i 嵌入**：∀e∈T_i ∃e'∈K_i 同 `ElementId` 且 (λ,ρ,ε,ℓ,parent_id) 全等——
///    host^op 限制在 Dom(host^struct) 上与 host^struct 逐点一致（「双向映射一致性」：
///    host^struct 命中 ⟹ host^op 命中同 carrier 同父，两套 host 在共同定义域无分叉）。
/// 2. **K_i ElementId 唯一**（dedup 后无重复——host^op 命中不二义，codex NO#2）。
/// 3. **K_i (level,ρ) 端点键唯一**（endpoint-complete 的 ∃! 分量：每个走势端点在 K_i 中
///    恰一个 carrier；[`build_tree_endpoint_index`] 的 or_insert 假设在 K_i 上成立）。
///
/// ★endpoint-complete 的 ∀ 分量（∀g∈B_i: host^op(g)≠⊥）是 **B_i 相对**性质，不在本函数
/// （无 bsp 输入）——由 K_i=U_i 构造保证（bsp.source_index=产出走势 end_index，signal.rs:101，
/// 该走势必在塔某级 ⟹ 必在 K_i；638 边界④若 source_index 语义变则须重裁）。实测 host-miss
/// 率是 L2 问题（下游跑批），本函数只封 L0 代数分量——不声明生产已证 Γ^K≅B_i（090/231）。
pub fn dual_view_consistency(
    t_i: &[CoverageElement],
    k_i: &[CoverageElement],
) -> Result<(), String> {
    let mut k_by_id: std::collections::HashMap<ElementId, &CoverageElement> =
        std::collections::HashMap::with_capacity(k_i.len());
    for e in k_i {
        if k_by_id.insert(e.id, e).is_some() {
            return Err(format!(
                "K_i ElementId 重复：{:?}（dedup 破裂，host^op 二义）",
                e.id
            ));
        }
    }
    let mut k_endpoints: std::collections::HashSet<(u32, usize)> =
        std::collections::HashSet::with_capacity(k_i.len());
    for e in k_i {
        if !k_endpoints.insert((e.level, e.rho)) {
            return Err(format!(
                "K_i (level,ρ) 端点键重复：({},{})（endpoint ∃! 破裂）",
                e.level, e.rho
            ));
        }
    }
    for e in t_i {
        match k_by_id.get(&e.id) {
            None => return Err(format!("T_i 元素 {:?} 不在 K_i（T_i⊆K_i 破裂）", e.id)),
            Some(k) => {
                if (k.lambda, k.rho, k.eps, k.level, k.parent_id)
                    != (e.lambda, e.rho, e.eps, e.level, e.parent_id)
                {
                    return Err(format!(
                        "T_i/K_i 同 id {:?} 字段分叉：T=({},{},{:?},{},{:?}) K=({},{},{:?},{},{:?})",
                        e.id, e.lambda, e.rho, e.eps, e.level, e.parent_id,
                        k.lambda, k.rho, k.eps, k.level, k.parent_id
                    ));
                }
            }
        }
    }
    Ok(())
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
    for sub in lm.sub_moves.iter() {
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
/// >   FollowParent（δ_g=σ_{p(g)}）/ ReverseOpen（δ_g=−σ_{p(g)} 反向子声部对冲腿），非恒 Ambient；
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
    let mut idx: std::collections::HashMap<(u32, usize), usize> = std::collections::HashMap::new();
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
///
/// ★P0-2 设计边界裁定（codex-f2 问题4，选项 (a)）：**单 carrier 单 active instance**。
/// 当前策略每 carrier 至多持一个 active 仓位（无加仓/分批/多 generation），故 §14 的 carrier-id
/// 简化 = 正确身份，共享 id 不产生错误——不升级 §13 四元组 hash（YAGNI：加仓/多批次未实装前
/// 四元组是无消费者的死机制）。此边界由 [`coverage_step_from_buckets`] 出口的 `next_active`
/// ElementId 唯一性 `debug_assert`（见该函数）看守：若同 carrier 以两个不同 (dir/entry) 同时进
/// active，assert 触发即暴露越界。**升级触发条件**（何时上四元组）：策略引入同 carrier 加仓 / 反手
/// 双开 / 分批建仓（需区分 entry_signal/side/generation）时，本简化失效，改 `posId=hash(...)`。
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
/// σ_{p(g)}=0，垂直关系 V 恒 **Ambient**（**不产** FollowParent/ReverseOpen，短差需真父子方向）；水平
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
    // element-coverage（含方向感知 + ReverseOpen）用真塔，本扁平入口仅作根级覆盖区间基线。
    for (level_idx, level) in classification.levels.iter().enumerate() {
        for center in level.centers.iter() {
            elements.push(CoverageElement {
                lambda: center.start_index,
                rho: center.end_index,
                eps: VoiceSide::Long, // 占位：Classification 不携元素方向（见上诚实标注）
                level: level_idx as u32,
                parent: None,
                attached_dir: None,
                // 扁平入口无真塔 ⟹ 确定性 ID 退化为 (level, center 序号)；parent_id=None=∂。
                id: ElementId {
                    level: level_idx as u32,
                    ordinal: elements.len() as u64,
                },
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

#[cfg(test)]
#[path = "element_tests.rs"]
mod tests;
