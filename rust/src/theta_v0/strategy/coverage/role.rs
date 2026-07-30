use super::*;

// ════════════════════════════════════════════════════════════════════════════
//  §4 操作角色 R(g)=(H(g),V(g),δ_g) 24 类完全分类（spec §7-§8 / P6-P7，去根化）
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
// Hash/Ord：作为 [`MuClass::horizontal`] 分量进 z（HashMap 桶键 + BTreeMap 有序报告）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
/// 三类互斥穷尽（阶段4/6/7 **living authority**，R(g)=(H,V,δ) 三轴）：
/// - `Ambient`：σ_{p(g)}=0（父容器无方向/胚元/空）——spec P7 显式：**Ambient 不是根规则**，
///   而是任意父容器处于无方向状态时的普通情形（去根化核心）。
/// - `FollowParent`：σ_{p(g)}≠0 ∧ δ_g = σ_{p(g)}（顺父方向）。
/// - `ReverseOpen`：σ_{p(g)}≠0 ∧ δ_g = −σ_{p(g)}（反父方向 = GPT `AgainstParent` 商映射）。
///
/// ★GPT 权威裁决（命名冲突.pdf §二-§四，2026-07-05）：阶段3 SameReverse_3（Rel=Same ∧ δ=−σ_p）
/// 到阶段4 V 轴投影 π_V(SameReverse_3)=ShortDiff_V。阶段4 把「同级别反父」与「次级别反父」在 V 轴
/// 合并为 AgainstParent:=δ=−σ_p——**商映射非遗漏**。级别差异作独立特征 G（[`GradeRel`]），不回 V 轴本体。
/// commit 67d20d292f 把 V 改四分类（加 SameReverse）违反 living authority（重定义 V 而非保守扩展）；
/// 本实装回滚三分类 + 新增 G 轴细化（定理1：z'=(H,V,δ,G) 投影 π(H,V,δ,G)=(H,V,δ) 保留原像划分）。
///
/// ★更名（#281 裁定，#283 实装）：`ReverseOpen` 原 `ShortDiff`——ADR 0001 修正案一·补充一
/// 词汇对齐，S6「短差」名随修1 废止退役，V 轴反父方向重读为修4「（次级别）首开反向」。
/// 语义=AgainstParent 不动（δ=−σ_p），级别区分独立到 [`GradeRel`]；判据/行为零改动。
/// 谱系：历史上 `ShortDiff` 在代码库三义——`TwEvent::ShortDiff`（TW 划转，修1 合法减补链，
/// **保留原名**）/ `ExitType::CloseShortDiff`（出场，本票同改 `CloseReverseOpen`）/ 本 V 轴。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vertical {
    /// σ_{p(g)}=0（父无方向 → 去根化 Ambient）。
    Ambient,
    /// δ_g = σ_{p(g)}（顺父方向）。
    FollowParent,
    /// δ_g = −σ_{p(g)}（反父方向首开反向，GPT AgainstParent 商映射；同级别/次级别区分在独立 G 轴
    /// [`GradeRel`]）。原 `ShortDiff`，#281 更名（#283 实装）。
    ReverseOpen,
}

/// 级别关系 G(g)（GPT 命名冲突裁决 §四新增独立轴，基于 ℓ_g vs ℓ_{p(g)}）。
///
/// 两类互斥（GPT §四「最小需要 G ∈ {SameLevel, SubLevel}」）：
/// - `SameLevel`：ℓ_g = ℓ_{p(g)}（同级别——阶段3 `SameReverse_3` 派生角色原料）。
/// - `SubLevel`：ℓ_g < ℓ_{p(g)}（次级别——阶段3 `ShortDiff_3` 派生角色原料）。
///
/// ★GPT 定理1（命名冲突.pdf §五）：G 是状态细化 z'=(H,V,δ,G)，对阶段4 状态有投影
/// π(H,V,δ,G)=(H,V,δ)，保留原分类原像划分。`ThetaDirPreset::Neutral` 下 [`dir_weight`] 不消费 G
/// ⟹ Neutral bit-exact；Follow/Adversary 亦不消费 G ⟹ G 当前是**观察特征**，留待 q_Θ^full
/// （GPT §九 κ/d·w_{…,G,…}）消费。
///
/// 派生角色（非 canonical 谓词，见 [`OperationRole::is_same_level_against_parent`]）：
/// SameLevelAgainstParent = V=ReverseOpen ∧ G=SameLevel；SubLevelReverseOpen = V=ReverseOpen ∧ G=SubLevel。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GradeRel {
    /// ℓ_g = ℓ_{p(g)}（同级别）。
    SameLevel,
    /// ℓ_g < ℓ_{p(g)}（次级别）。
    SubLevel,
}

/// 完整操作角色 R(g) = (H(g), V(g), δ_g, G(g))（spec §8 / P7 + GPT G 轴，3×3×2 = **18 类 canonical**）。
///
/// 角色空间 ℛ = {First,SameFollow,SameReverse} × {Ambient,FollowParent,ReverseOpen} × {+1,-1}，
/// 基数 18（canonical，阶段4 living authority）；G 轴（[`GradeRel`]）作 z'=(H,V,δ,G) 第四维细化，
/// 不改 canonical 基数（GPT 定理1：投影 π(H,V,δ,G)=(H,V,δ) 保留原像划分）。
///
/// ## 去根化（spec §7 / P7 / 差异表 #2）
/// 旧 {Root, Same, Sub}（含根特例）→ 水平 H × 垂直 V × 方向 δ 三正交轴。**无 RootRole 枚举值**：
/// 旧"根元素"now = (First/SameFollow/SameReverse, **Ambient**, ±1)——根被边界胚元 ∂ + Ambient 吸收。
///
/// ## G 轴细化（GPT 命名冲突裁决 §四，2026-07-05）
/// V 轴保持阶段4 三分类 living authority；新增 G∈{SameLevel,SubLevel} 作独立第四维。
/// 派生角色（谓词方法）：[`is_same_level_against_parent`]（=阶段3 SameReverse_3）、
/// [`is_sub_level_reverse_open`]（=阶段3 ShortDiff_3）。
///
/// ## 认识论等级（formalization-validity-domain 231号，强制标注）
/// - **L0**（代数命题，本结构）：3×3×2=18 完全分类 `Σ=1` 是同义反复（三轴各自互斥穷尽的笛卡尔积），
///   信息增量为零。G 细化是 z'投影，不增信息。
/// - **L2 未覆盖**（spec 疑点5，no-声明膨胀）：3×3×2=18 全组合是否在真实数据上**均可达**，
///   还是部分组合经验为空（有效域 < 定义域），spec **未覆盖**——本实装**不**声称 18 类经验全可达。
///
/// [`is_same_level_against_parent`]: OperationRole::is_same_level_against_parent
/// [`is_sub_level_reverse_open`]: OperationRole::is_sub_level_reverse_open
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationRole {
    /// H(g)：水平关系（同级别兄弟轴）。
    pub h: Horizontal,
    /// V(g)：垂直关系（父容器轴，阶段4 living authority 三分类）。
    pub v: Vertical,
    /// δ_g：方向（±1）。
    pub delta: Dir,
    /// G(g)：级别关系（GPT 命名冲突裁决新增独立轴，z'=(H,V,δ,G) 第四维）。
    pub grade: GradeRel,
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
pub(super) fn dir_sign(d: Dir) -> i8 {
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
fn operation_role_indexed(
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
                grade: GradeRel::SameLevel,
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
    // V(g) 三分类（阶段4 living authority）+ G(g) 级别关系（GPT 命名冲突裁决新增独立轴）。
    // classify_vertical 回三分类（δ=−σ_p ⟹ ReverseOpen 不分级别，商映射 AgainstParent）；
    // G 轴消费 ℓ_g vs ℓ_p 作细化（z'=(H,V,δ,G) 投影保留原像，GPT 定理1）。
    let sigma_parent = parent_sign(e.attached_dir);
    let v = classify_vertical(sigma_parent, dir_sign(delta));
    let ell_g = e.level;
    let ell_p = match e.parent {
        // ponytail: 父越界/None 防御归 ℓ_g（⟹ SameLevel）；None 已被 σ_p=0（∂）截断 V=Ambient。
        Some(p) => elements.get(p).map_or(ell_g, |parent| parent.level),
        None => ell_g,
    };
    let grade = classify_grade(ell_g, ell_p);
    OperationRole { h, v, delta, grade }
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
                grade: GradeRel::SameLevel,
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
    // V(g) 三分类（阶段4 living authority）+ G(g) 级别关系（GPT 命名冲突裁决新增独立轴）。
    // classify_vertical 回三分类（δ=−σ_p ⟹ ReverseOpen 不分级别，商映射 AgainstParent）；
    // G 轴消费 ℓ_g vs ℓ_p 作细化（z'=(H,V,δ,G) 投影保留原像，GPT 定理1）。
    let sigma_parent = parent_sign(e.attached_dir);
    let v = classify_vertical(sigma_parent, dir_sign(delta));
    let ell_g = e.level;
    let ell_p = match e.parent {
        // ponytail: 父越界/None 防御归 ℓ_g（⟹ SameLevel）；None 已被 σ_p=0（∂）截断 V=Ambient。
        Some(p) => elements.get(p).map_or(ell_g, |parent| parent.level),
        None => ell_g,
    };
    let grade = classify_grade(ell_g, ell_p);
    OperationRole { h, v, delta, grade }
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
pub(super) fn operation_role_two_segment(
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
                grade: GradeRel::SameLevel,
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
    // V(g) 三分类（阶段4 living authority）+ G(g) 级别关系（GPT 命名冲突裁决新增独立轴）。
    // classify_vertical 回三分类（δ=−σ_p ⟹ ReverseOpen 不分级别，商映射 AgainstParent）；
    // G 轴消费 ℓ_g vs ℓ_p 作细化（z'=(H,V,δ,G) 投影保留原像，GPT 定理1）。
    let sigma_parent = parent_sign(e.attached_dir);
    let v = classify_vertical(sigma_parent, dir_sign(delta));
    let ell_g = e.level;
    let ell_p = match e.parent {
        // ponytail: 父越界/None 防御归 ℓ_g（⟹ SameLevel）；None 已被 σ_p=0（∂）截断 V=Ambient。
        Some(p) => elements.get(p).map_or(ell_g, |parent| parent.level),
        None => ell_g,
    };
    let grade = classify_grade(ell_g, ell_p);
    OperationRole { h, v, delta, grade }
}

/// V(g) 纯分类（单一真相源）：给定 σ_{p(g)}、δ_g 符号 ⟹ Vertical（阶段4 三分类 living authority）。
///
/// 全部 V 判定路径共享此函数——[`vertical_relation`]（单元素）+ `operation_role` 的三个优化变体
/// （`operation_role_indexed` / `operation_role_indexed_split` / `operation_role_two_segment`，后者为
/// 生产热路径 [`strategy_target_legs`] 毛分账本腿 `q̄_Θ` 产腿）。消除多份 V 判定漂移。
///
/// 互斥三分类：σ_p=0→`Ambient`；δ_g=σ_p→`FollowParent`；δ_g=−σ_p→`ReverseOpen`（GPT AgainstParent
/// 商映射，不分级别——同级别/次级别反父合并，级别区分在独立 G 轴 [`classify_grade`]）。
fn classify_vertical(sigma_parent: i8, delta_sgn: i8) -> Vertical {
    if sigma_parent == 0 {
        Vertical::Ambient
    } else if delta_sgn == sigma_parent {
        Vertical::FollowParent
    } else {
        Vertical::ReverseOpen
    }
}

/// G(g) 级别关系纯分类（单一真相源，GPT 命名冲突裁决 §四新增轴）：给定 ℓ_g、ℓ_{p(g)} ⟹ GradeRel。
///
/// 与 [`classify_vertical`] 同源——调用方共享同一 (ℓ_g, ℓ_p) 读取，V 三分类 + G 两分类独立组装
/// [`OperationRole`]。ℓ_g<ℓ_p→`SubLevel`；ℓ_g≥ℓ_p（含等于/越界防御）→`SameLevel`。
fn classify_grade(ell_g: u32, ell_p: u32) -> GradeRel {
    if ell_g < ell_p {
        GradeRel::SubLevel
    } else {
        GradeRel::SameLevel
    }
}

/// 垂直关系 V(g)（spec §7.2 / P6-P7，阶段4 三分类 living authority，全函数唯一判定）。
///
/// 从元素读 (σ_{p(g)}, δ_g) 喂 [`classify_vertical`]（单一真相源，2 参数三分类）。
/// G 轴（级别关系）独立，见 [`grade_relation`]；本函数只返 V。
pub fn vertical_relation(elements: &[CoverageElement], e_idx: usize) -> Vertical {
    let e = match elements.get(e_idx) {
        Some(e) => e,
        None => return Vertical::Ambient, // 越界防御性（不应到达）
    };
    classify_vertical(parent_sign(e.attached_dir), dir_sign(direction_of(e.eps)))
}

/// G(g) 级别关系（GPT 命名冲突裁决 §四，全函数唯一判定，独立于 [`vertical_relation`]）。
///
/// 从元素读 (ℓ_g, ℓ_{p(g)}) 喂 [`classify_grade`]。ℓ_p 经 `parent` 链读取；
/// 父越界/None（边界胚元 ∂）防御归 ℓ_g（⟹ SameLevel，σ_p=0 已截断 V=Ambient，G 当前无消费方不影响 bit-exact）。
pub fn grade_relation(elements: &[CoverageElement], e_idx: usize) -> GradeRel {
    let e = match elements.get(e_idx) {
        Some(e) => e,
        None => return GradeRel::SameLevel, // 越界防御性（不应到达）
    };
    let ell_g = e.level;
    let ell_p = match e.parent {
        Some(p) => elements.get(p).map_or(ell_g, |parent| parent.level),
        None => ell_g,
    };
    classify_grade(ell_g, ell_p)
}

/// 派生元素 e 的完整操作角色 R(g)=(H(g),V(g),δ_g)（spec §8 / P7，24 类，全函数唯一判定）。
///
/// 三轴各自由 [`horizontal_relation`] / [`vertical_relation`] / [`direction_of`] 唯一确定，
/// 元组即角色（spec `R(g)=(H(g),V(g),δ_g)`）。
///
/// > **结果包六要素**
/// > - **结论**：每个语法元素 e 的操作角色 = 三轴元组 (H,V,δ)，落入 ℛ 的 24 类之一。
/// > - **定义依据**：spec §7.1（H 相对前同级兄弟 prev(g)）+ §7.2（V 相对父方向 σ_{p(g)}）
/// >   + §8（R=H×V×δ，3×4×2=24，`Σ_{r∈ℛ}1=1`）。输入 `CoverageElement` 的 `parent`/`level` 喂 H，
/// >   `attached_dir` 喂 V，`eps` 喂 δ。
/// > - **边界条件**：① H 穷尽性依赖 δ_g,σ_{prev}∈{+1,-1} 二值——若元素方向取 Flat（σ=0），δ 不在
/// >   二值域内，H 分类退化（本实装 ε_e 二值不变量保证不发生）。② V 三分依赖 σ_{p(g)} 含 0
/// >   （胚元/Flat→Ambient）；若父方向取值域扩大，V 翻转。③ 同级别兄弟须 `parent` 与 `level` 双相等
/// >   ——只 parent 相等而 level 不等（不同级别根）不算同级兄弟（spec "同级别兄弟"）。
/// > - **下游推论**：classify 输出 (H,V,δ) 三轴元组，喂 R_Θ 解释器 / 活动集腿角色标注；
/// >   `v==ReverseOpen` 标记反向子声部对冲腿（净额执行 [`net_target_units`] 部分抵消父仓）。
/// > - **谱系引用**：差异表 #2（{Root,Same,Sub}→H×V×δ）+ §9 短差去根化（旧"根多头次级别空头"→
/// >   父级反向子操作）；MEMORY coverage-engine-needs-tower-export-bridge（24 类角色 = 多级角色框架
/// >   形式化，是 classify 导出 LeveledMove 塔的前置对象升级）。旧 4 类 OperationRole（对应 Lean
/// >   MW3 / M09 {RootDir,SameDir,SubFollow,ShortDiff}）被本 24 类**替换**（非保留 fallback）。
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
        grade: grade_relation(elements, e_idx),
    }
}

impl OperationRole {
    /// 派生角色（非 canonical 谓词，GPT 命名冲突裁决 §四）：V=ReverseOpen ∧ G=SameLevel。
    /// = 阶段3 SameReverse_3（Rel=Same ∧ δ=−σ_p，同级别反父方向）。
    pub fn is_same_level_against_parent(&self) -> bool {
        self.v == Vertical::ReverseOpen && self.grade == GradeRel::SameLevel
    }
    /// 派生角色（非 canonical 谓词，GPT 命名冲突裁决 §四）：V=ReverseOpen ∧ G=SubLevel。
    /// = 阶段3 ShortDiff_3（Rel=Sub ∧ δ=−σ_p，次级别首开反向）。原 `is_sub_level_short_diff`，
    /// #281 更名（#283 实装）。
    pub fn is_sub_level_reverse_open(&self) -> bool {
        self.v == Vertical::ReverseOpen && self.grade == GradeRel::SubLevel
    }
}

// ════════════════════════════════════════════════════════════════════════════
//  §5 LegTarget 每活动元素一腿（M17/M28，方向 ε_e + 单位 s_e 按 role/depth 权重）
// ════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
#[path = "role_tests.rs"]
mod tests;
