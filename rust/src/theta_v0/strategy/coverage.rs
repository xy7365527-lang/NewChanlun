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
use super::super::classifier::recursive_tower::LeveledMove;
use super::super::classifier::Classification;
use super::super::config::{RiskConfig, VoiceConfig};
use super::super::types::{Direction, Order, StrictAction};
use super::intent::{lex_argmin, JThetaKey, LexCandidate};
use super::interp::{self, ActiveLeg, Buckets};
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
pub fn extract_elements(tower: &[Vec<LeveledMove>]) -> Vec<CoverageElement> {
    let mut elements: Vec<CoverageElement> = Vec::new();
    // 顶级元素（tower 最高级的走势）是根（parent=None）；逐层向下钻取真嵌套子元素。
    // 从最高级开始（顶层走势是边界胚元 ∂ 容器下的兄弟，去根化无 RootRole）。
    for level_moves in tower.iter().rev() {
        for lm in level_moves {
            // 顶层走势 = 根元素（parent=None，attached_dir=None）。
            push_element_tree(&mut elements, lm, None, None);
        }
        // 只展开最高非空级别作为根（更低级别由 Compose.subs 真嵌套带出，不重复作根）。
        if !level_moves.is_empty() {
            break;
        }
    }
    elements
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
    });
    // 真嵌套子声部（descend 取回的子走势携坐标侧车 `sub_moves`）⟹ 子元素 parent=my_idx。
    for sub in &lm.sub_moves {
        push_element_tree(elements, sub, Some(my_idx), Some(eps));
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
    match tree
        .iter()
        .find(|e| e.level == level_g && e.rho == source_index)
    {
        // 继承 hostOf 的真 Compose 父 + 父方向 σ_{p(g)}（push_element_tree 真父子，铁律）。
        Some(host) => (host.parent, host.attached_dir),
        // host 无（未被 Compose 收录 / tree 无该级别 / 顶层根）⟹ 无真父 = ∂ → V=Ambient（去根化）。
        None => (None, None),
    }
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
pub fn ancestors(elements: &[CoverageElement], e_idx: usize) -> Vec<usize> {
    let mut chain = Vec::new();
    let mut cur = elements.get(e_idx).and_then(|e| e.parent);
    while let Some(p) = cur {
        chain.push(p);
        cur = elements.get(p).and_then(|e| e.parent);
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
    // prev(g)：同父容器 + 同级别 + 索引最近的前兄弟（None==None = 同一边界胚元 ∂）。
    let prev = (0..e_idx)
        .rev()
        .find(|&i| elements[i].parent == e.parent && elements[i].level == e.level);
    match prev {
        None => Horizontal::First,
        Some(p) => {
            let sigma_prev = dir_sign(direction_of(elements[p].eps));
            if delta == sigma_prev {
                Horizontal::SameFollow
            } else {
                Horizontal::SameReverse
            }
        }
    }
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
    let depth = element_depth(elements, e_idx);
    let w = depth_weight(depth, config);
    let role = operation_role(elements, e_idx);
    LegTarget {
        e_idx,
        side: e.eps,
        units: base_units * w,
        role,
    }
}

/// 元素的真嵌套深度（沿 parent 链长度，根=0；铁律：真父子，非级别差）。
fn element_depth(elements: &[CoverageElement], e_idx: usize) -> u32 {
    ancestors(elements, e_idx).len() as u32
}

/// **全定义策略目标头寸腿集 `q̄_Θ = LegTarget(AncOK[(A∖D)∪B])`**（M28 §十三 总形式）。
///
/// 对祖先闭合活动集 `active`（A_{t+1}）中**每个元素**生成 [`leg_target`]，得目标头寸腿列表
/// （= 分账本目标头寸 q̄_Θ 的腿分解，对齐 `SeparateStrategyTarget.strategyTargetLegs` + M28）。
/// 每条腿带 `(ν(e), ε_e, s_e, role)`，多空独立坐标（分账本 C25），净额抵消在 [`net_target_units`]。
pub fn strategy_target_legs(
    elements: &[CoverageElement],
    active: &[usize],
    base_units: f64,
    config: &VoiceConfig,
) -> Vec<LegTarget> {
    active
        .iter()
        .map(|&e_idx| leg_target(elements, e_idx, base_units, config))
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

/// 持仓腿 [`ActiveLeg`] → **当前因果树元素索引**（638 坐标身份：本级 `level`、右端点
/// `ρ==source_index`、方向 `eps==dir`）。
///
/// 只在**真嵌套树元素段** `elements[0..candidate_start)` 查（候选段 `[candidate_start..)` 是本 bar
/// 新触发候选，非持仓）。找到 ⟹ 持仓腿覆盖的走势在当前因果树在场 ⟹ 其子声部腿的 AncOK 祖先齐全
/// （[`ancestor_close`] 保留）；未找到 ⟹ 持仓腿走势不在当前前缀因果树（走势已演化/不在前缀，缺塔
/// 边界）⟹ 调用方追加为根元素（无父，AncOK 不剔）。
///
/// ★真 Fugue 铁律（mod.rs:433-440 codex 裁旧 bug）：父子来自**真 Compose 塔**（树元素由
/// [`extract_elements`] 的 `sub_moves` 真嵌套建，非级别差伪造），本函数只按 638 坐标身份把持仓腿
/// 对位回真树元素，**不**伪造父子。
///
/// ## 持久身份对位（codex Q4 ρ 漂移修正）
///
/// 三段判定（按严格性降序）——区分 **coord_drift（父语义有效仅坐标漂移）** vs **stale（父真失效）**：
/// 1. [`HeldLegMatch::Exact`]：`(level, ρ==source_index, eps)` 精确命中——ρ 未漂移的常态对位。
/// 2. [`HeldLegMatch::CoordDrift`]：`(level, λ==lambda, eps)` 稳定身份命中且 `e.rho >= leg.source_index`
///    ——父容器走势**向右延伸**（吸收更多次级别子走势 ⟹ `end_index/ρ` 增大），但**起点 λ 不变** ⟹
///    **同一父**（仅坐标漂移）。返回当前（已延伸）树元素 idx（携真父链）——**不静默降 orphan**。
/// 3. [`HeldLegMatch::Stale`]：无任何语义匹配 ⟹ 持仓腿走势**真失效**（结构演化/不在前缀因果树）⟹
///    调用方作根保留（无父，AncOK 不剔）。
///
/// λ 稳定性依据：走势的 `start_index`（左端点）在其向右延伸时不变（confirmed 前缀不回写，
/// reference:16）；`end_index/ρ`（右端点）随延伸增大。`(level, λ, eps)` 在因果前缀内唯一（同级别
/// 两个不同 confirmed 走势不共享起点）⟹ 是走势的稳定语义身份（独立坐标，无需 generation_id）。
fn held_leg_tree_index(
    elements: &[CoverageElement],
    candidate_start: usize,
    leg: &ActiveLeg,
) -> HeldLegMatch {
    let tree_end = candidate_start.min(elements.len());
    let tree = &elements[..tree_end];
    // 1. 精确身份（level + ρ + eps）：未漂移常态。
    if let Some(idx) = tree
        .iter()
        .position(|e| e.level == leg.level && e.rho == leg.source_index && e.eps == leg.dir)
    {
        return HeldLegMatch::Exact(idx);
    }
    // 2. 稳定身份（level + λ + eps，且当前 ρ ≥ 旧 ρ=向右延伸）：coord_drift（同一父延伸）。
    //    旧 ρ（leg.source_index）落入当前已延伸区间 [λ, ρ] 内 ⟹ 同一走势吸收了旧端点。
    if let Some(idx) = tree.iter().position(|e| {
        e.level == leg.level
            && e.eps == leg.dir
            && e.lambda == leg.lambda
            && e.rho >= leg.source_index
    }) {
        return HeldLegMatch::CoordDrift(idx);
    }
    // 3. 无语义匹配 ⟹ 真失效。
    HeldLegMatch::Stale
}

/// 持仓腿跨 bar 对位结果（持久身份三分；[`held_leg_tree_index`] 产）。
///
/// `Exact`/`CoordDrift` 都对位回**当前因果树元素 idx**（携真父链，AncOK 祖先齐全判据有效）；
/// `Stale` 才作根保留（父真失效）。`CoordDrift` 是 codex Q4 修正核心：旧逻辑只有 ρ 精确匹配，
/// 父延伸（ρ 漂移）即误判 stale → 静默降 orphan → 子声部 ShortDiff 腿父不在 raw → AncOK 误剪
/// （假阴性）。区分 coord_drift 后，延伸父仍在 raw ⟹ ShortDiff 子腿正确准入。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeldLegMatch {
    /// ρ 精确命中（未漂移）：当前树元素 idx。
    Exact(usize),
    /// λ 稳定命中（父延伸，ρ 漂移）：当前（已延伸）树元素 idx。
    CoordDrift(usize),
    /// 无语义匹配（父真失效）：调用方作根保留。
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
    ActiveLeg { level: e.level, dir: e.eps, source_index: e.rho, lambda: e.lambda }
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
pub fn coverage_step_from_buckets(
    elements: &[CoverageElement],
    candidate_start: usize,
    prev_active: &[ActiveLeg],
    buckets: &Buckets,
    base_units: f64,
    config: &VoiceConfig,
) -> (Vec<ActiveLeg>, f64) {
    // 工作副本（immutable：不 mutate 入参；不在当前因果树的持仓腿追加为根）。
    let mut work: Vec<CoverageElement> = elements.to_vec();
    let mut raw: Vec<usize> = Vec::new();

    // (A_t ∖ 𝒟_x)：持仓腿（除 close 认领）对位回当前因果树元素（638 身份）；不在树 ⟹ 追加为根。
    let closed = close_indices(prev_active, &buckets.close);
    for (i, leg) in prev_active.iter().enumerate() {
        if closed.contains(&i) {
            continue; // 𝒟_x：本腿关闭，不入 A^raw
        }
        match held_leg_tree_index(&work, candidate_start, leg) {
            // Exact（ρ 未漂移）/ CoordDrift（父延伸，ρ 漂移但 λ 稳定=同一父）：都对位回当前树元素
            // idx（携真父链）⟹ 其子声部腿的 AncOK 祖先齐全。CoordDrift 不再静默降 orphan（Q4 修正）。
            HeldLegMatch::Exact(idx) | HeldLegMatch::CoordDrift(idx) => {
                if !raw.contains(&idx) {
                    raw.push(idx);
                }
            }
            // Stale（父真失效）：持仓腿走势不在当前前缀因果树 ⟹ 作根保留（无父，AncOK 不剔）。
            HeldLegMatch::Stale => {
                let idx = work.len();
                work.push(CoverageElement {
                    lambda: leg.lambda,
                    rho: leg.source_index,
                    eps: leg.dir,
                    level: leg.level,
                    parent: None,
                    attached_dir: None,
                });
                raw.push(idx);
            }
        }
    }

    // ∪ ℬ_x：开启候选 → 其 638 附着因果树元素索引（candidate_start + gamma_index，携真 Compose 父）。
    for c in &buckets.open {
        let idx = candidate_start + c.gamma_index;
        if idx < work.len() && !raw.contains(&idx) {
            raw.push(idx);
        }
    }

    // 步2：A_{t+1}=AncOK(A^raw)——剔除真 Compose 父容器不在 raw 的孤儿子腿（§13 持仓准入：未持父则剔除）。
    let next_idx = ancestor_close(&work, &raw);

    // p̃=Σ Leg(g)（depth 权重沿真父链 + 方向净额聚合，ShortDiff 空腿部分对冲父多腿）。
    let legs = strategy_target_legs(&work, &next_idx, base_units, config);
    let p_tilde = net_target_units(&legs);

    // A_{t+1} 回 ActiveLeg（638 身份，喂下一 bar interpret 闭环 + 跨 bar 对位）。
    let next_active = next_idx.iter().map(|&i| element_as_leg(&work[i])).collect();
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
    tower: &[Vec<LeveledMove>],
    prev_active: &[ActiveLeg],
    base_units: f64,
    config: &VoiceConfig,
) -> (Vec<ActiveLeg>, f64) {
    // 真父子组合元素（tree ++ 638 附着候选）——σ_p=父容器方向（639）+ AncOK 持仓准入的单一元素来源。
    let (elements, candidate_start) = interp::coverage_elements_with_tower(classification, tower);
    // 环5：候选集 Γ（V 由真父派生）+ 解释器三桶（𝒟_x 反向关闭喂 prev_active）。
    let gamma = interp::assemble_gamma_with_tower(classification, tower);
    let buckets = interp::interpret(&gamma, prev_active);
    // 环6：A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x] + p̃（§13 持仓准入：ShortDiff 未持父则剔除，639(c)）。
    coverage_step_from_buckets(&elements, candidate_start, prev_active, &buckets, base_units, config)
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
    tower: &[Vec<LeveledMove>], // per-bar 因果塔（639 σ_p=父容器方向；runner 喂前缀重分类塔）
    prev_active: &[ActiveLeg],
    p_t: f64,
    exec_index: usize,
    base_units: f64,
    voice: &VoiceConfig,
    risk: &RiskConfig,
    weights: PiThetaWeights,
    gate: KThetaRiskGate, // 𝒦_Θ 风控约束门（close_pred 折入；全开=open()）
) -> (Vec<ActiveLeg>, f64, Order) {
    // 环5+6：买卖点 Γ 入场 → A_{t+1} + p̃（GAP-5：入场源 = BspPoint.source_index 买卖点）。
    // 执行层 σ_p=父容器方向（639；coverage_step_classification 内 assemble_gamma_with_tower 喂因果塔）。
    let (next_active, p_tilde) =
        coverage_step_classification(classification, tower, prev_active, base_units, voice);
    // 环7：p* = LexArgmin J_x（𝒦_Θ，风控门收窄）→ O = Schedule_Θ(p*−p_t)（单一决策出口 §16）。
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
    fn nested_l1(si: usize, ei: usize, sub_dirs: [Direction; 3]) -> LeveledMove {
        let s0 = LeveledMove::from_unit(&unit(si, si + 4, sub_dirs[0], 0, 10));
        let s1 = LeveledMove::from_unit(&unit(si + 4, si + 8, sub_dirs[1], 3, 12));
        let s2 = LeveledMove::from_unit(&unit(si + 8, ei, sub_dirs[2], 5, 15));
        LeveledMove::compose(&[s0, s1, s2], ctr(si, ei), 1)
    }

    // ── §1/§2 元素提取（真嵌套父子，铁律守护）─────────────────────────────────

    /// 元素提取：L1 走势 + 3 个真嵌套 L0 子声部 = 4 个元素（1 根 + 3 子，真父子）。
    #[test]
    fn extract_elements_nested_parent_child() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = vec![Vec::new(), vec![l1]]; // 索引=级别：L0 空（子由 Compose 带出），L1 一个走势
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
        let tower = vec![Vec::new(), vec![l1]];
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
    fn two_parent_tower() -> Vec<Vec<LeveledMove>> {
        let a0 = LeveledMove::from_unit(&unit(0, 4, Direction::Up, 0, 10));
        let a1 = LeveledMove::from_unit(&unit(4, 8, Direction::Down, 3, 12));
        let a2 = LeveledMove::from_unit(&unit(8, 12, Direction::Up, 5, 15));
        let compose_a = LeveledMove::compose(&[a0, a1, a2], ctr(0, 12), 1); // 外缘 10→15 ⟹ Long
        let b0 = LeveledMove::from_unit(&unit(12, 16, Direction::Up, 5, 15)); // 结构 == a2
        let b1 = LeveledMove::from_unit(&unit(16, 20, Direction::Down, 3, 12));
        let b2 = LeveledMove::from_unit(&unit(20, 24, Direction::Down, 0, 8));
        let compose_b = LeveledMove::compose(&[b0, b1, b2], ctr(12, 24), 1); // 外缘 15→8 ⟹ Short
        vec![Vec::new(), vec![compose_a, compose_b]]
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

    /// 638 边界：host 未找到（无 ρ==source_index 的本级元素）⟹ (None,None) 去根化 Ambient。
    #[test]
    fn attach_host_not_found_is_ambient() {
        let tree = extract_elements(&two_parent_tower());
        assert_eq!(attach_bsp_to_tree(&tree, 0, 99), (None, None), "无本级 host ⟹ 无真父 ⟹ Ambient");
    }

    /// 638 边界 + tower-export-i guard：tower.len()<2（仅 L0 全根）⟹ host 是根 ⟹ 恒 Ambient。
    #[test]
    fn attach_guard_no_compose_level_is_ambient() {
        let s0 = LeveledMove::from_unit(&unit(0, 4, Direction::Up, 0, 10));
        let s1 = LeveledMove::from_unit(&unit(4, 8, Direction::Down, 3, 12));
        let tower = vec![vec![s0, s1]]; // 仅 L0，len()==1 < 2（无 Compose 级）
        let tree = extract_elements(&tower);
        // host=s0（ρ=4，level 0）但 s0 是根（parent=None）⟹ 缺塔诚实退化 Ambient。
        assert_eq!(attach_bsp_to_tree(&tree, 0, 4), (None, None), "缺塔（len<2）⟹ host 是根 ⟹ Ambient");
    }

    // ── §3 活动集递归 A_{t+1}=AncOK[(A_t∖D_t)∪B_t]（先关后开 + 祖先闭合）──────────

    /// B_t/D_t：bar t 的开始/结束元素索引（按 λ_e/ρ_e 过滤）。
    #[test]
    fn starting_ending_sets_by_endpoints() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = vec![Vec::new(), vec![l1]];
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
        let tower = vec![Vec::new(), vec![l1]];
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
        let tower = vec![Vec::new(), vec![l1]];
        let elements = extract_elements(&tower);
        // raw 含根（0）+ 子0（1）：子0 祖先=根0 在 raw ⟹ 都保留。
        let active = active_set_step(&elements, &[0, 1], 99);
        assert!(active.contains(&0) && active.contains(&1));
    }

    /// 先关后开：D_t 中的元素被关闭（不在 A_{t+1}），B_t 中的被开启。
    #[test]
    fn raw_close_then_open() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = vec![Vec::new(), vec![l1]];
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
        let tower = vec![Vec::new(), vec![l1]];
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
        let tower = vec![Vec::new(), vec![l1]];
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
            CoverageElement { lambda: 0, rho: 4, eps: VoiceSide::Long, level: 1, parent: None, attached_dir: None },
            CoverageElement { lambda: 4, rho: 8, eps: VoiceSide::Long, level: 1, parent: None, attached_dir: None },
            CoverageElement { lambda: 8, rho: 12, eps: VoiceSide::Short, level: 1, parent: None, attached_dir: None },
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
        let tower = vec![Vec::new(), vec![l1]];
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
        let mut v = vec![
            CoverageElement {
                lambda: 0,
                rho: 0,
                eps: VoiceSide::Flat,
                level: 0,
                parent: None,
                attached_dir: None,
            };
            n
        ];
        for c in open {
            v[c.gamma_index] = CoverageElement {
                lambda: c.source_index,
                rho: c.source_index,
                eps: c.dir,
                level: c.level,
                parent: None,
                attached_dir: None,
            };
        }
        v
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
        let buckets = Buckets {
            close: vec![],
            open: vec![cand(0, 5, VoiceSide::Long, 0)],
            record: vec![],
        };
        let (active, p) = coverage_step_from_buckets(&flat_elements(&buckets.open), 0, &[], &buckets, 1000.0, &cfg());
        assert_eq!(active.len(), 1, "ℬ_x 开启 ⟹ A_{{t+1}} 一条新腿");
        assert_eq!(active[0], ActiveLeg { level: 0, dir: VoiceSide::Long, source_index: 5, lambda: 5 });
        // p̃ = 1000×w_depth(0)=1000×0.60=600（根 depth 0，多腿正号）。
        assert!((p - 600.0).abs() < 1e-9, "p̃ = base×w[0] = 600（单多腿）");
    }

    /// ★环6 关闭：A_t 一条 Long 腿 + 𝒟_x={该腿} ⟹ A_{t+1}=∅，p̃=0（先关后开）。
    #[test]
    fn buckets_close_removes_active_leg() {
        let leg = ActiveLeg { level: 0, dir: VoiceSide::Long, source_index: 3, lambda: 3 };
        let buckets = Buckets {
            close: vec![leg], // 𝒟_x ⊆ A_t
            open: vec![],
            record: vec![],
        };
        let (active, p) = coverage_step_from_buckets(&flat_elements(&buckets.open), 0, &[leg], &buckets, 1000.0, &cfg());
        assert!(active.is_empty(), "𝒟_x 关闭活动腿 ⟹ A_{{t+1}} 空");
        assert_eq!(p, 0.0, "无活动腿 ⟹ p̃=0");
    }

    /// ★环6 净额聚合：A_{t+1} = {Long, Short} ⟹ p̃ = +600 −600 = 0（方向聚合，多空抵消）。
    #[test]
    fn buckets_target_nets_long_and_short() {
        let buckets = Buckets {
            close: vec![],
            open: vec![
                cand(0, 1, VoiceSide::Long, 0),
                cand(1, 2, VoiceSide::Short, 1),
            ],
            record: vec![],
        };
        let (active, p) = coverage_step_from_buckets(&flat_elements(&buckets.open), 0, &[], &buckets, 1000.0, &cfg());
        assert_eq!(active.len(), 2);
        // 两腿同根 depth 0（w[0]=0.60）：Long +600，Short −600 ⟹ 净 0。
        assert!(p.abs() < 1e-9, "p̃ = +600 −600 = 0（方向净额聚合）");
    }

    /// ★环6 先关后开 + 保留：A_t={legA, legB}，𝒟_x={legA}，ℬ_x={candC} ⟹ A_{t+1}={legB, legC}。
    #[test]
    fn buckets_close_then_open_keeps_survivor() {
        let leg_a = ActiveLeg { level: 0, dir: VoiceSide::Long, source_index: 1, lambda: 1 };
        let leg_b = ActiveLeg { level: 1, dir: VoiceSide::Short, source_index: 2, lambda: 2 };
        let buckets = Buckets {
            close: vec![leg_a],                          // 关 legA
            open: vec![cand(0, 9, VoiceSide::Long, 0)],  // 开 candC（level 0 Long）
            record: vec![],
        };
        let (active, _p) = coverage_step_from_buckets(&flat_elements(&buckets.open), 0, &[leg_a, leg_b], &buckets, 1000.0, &cfg());
        assert!(!active.contains(&leg_a), "legA 被 𝒟_x 关闭");
        assert!(active.contains(&leg_b), "legB（不在 𝒟_x）保留");
        assert!(
            active.contains(&ActiveLeg { level: 0, dir: VoiceSide::Long, source_index: 9, lambda: 9 }),
            "candC 被 ℬ_x 开启"
        );
        assert_eq!(active.len(), 2);
    }

    /// ★环6 AncOK 扁平恒等（诚实有效域）：桶路径活动腿全独立根（parent=∂）⟹ Anc=∅ ⟹ 不剔任何腿。
    /// （真嵌套塔剔孤儿在 §3 active_set_step 已验；本桶路径无塔故恒等——MEMORY tower-export-bridge 缺口）。
    #[test]
    fn buckets_ancok_identity_on_flat_roots() {
        let legs = [
            ActiveLeg { level: 0, dir: VoiceSide::Long, source_index: 0, lambda: 0 },
            ActiveLeg { level: 1, dir: VoiceSide::Long, source_index: 1, lambda: 1 },
            ActiveLeg { level: 2, dir: VoiceSide::Short, source_index: 2, lambda: 2 },
        ];
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        let (active, _p) = coverage_step_from_buckets(&flat_elements(&buckets.open), 0, &legs, &buckets, 1000.0, &cfg());
        assert_eq!(active.len(), 3, "扁平根全保留（AncOK 恒等——无父子可剔孤儿）");
    }

    /// ★环6 immutable：coverage_step_from_buckets 不 mutate prev_active。
    #[test]
    fn buckets_step_immutable_prev_active() {
        let prev = vec![ActiveLeg { level: 0, dir: VoiceSide::Long, source_index: 0, lambda: 0 }];
        let snapshot = prev.clone();
        let buckets = Buckets {
            close: vec![],
            open: vec![cand(1, 3, VoiceSide::Short, 0)],
            record: vec![],
        };
        let _ = coverage_step_from_buckets(&flat_elements(&buckets.open), 0, &prev, &buckets, 1000.0, &cfg());
        assert_eq!(prev, snapshot, "桶驱动递归不 mutate prev_active（纯函数）");
    }

    /// 𝒦_x（record 桶）不进活动集（spec「记录但暂不执行」）。
    #[test]
    fn buckets_record_excluded_from_active_set() {
        let buckets = Buckets {
            close: vec![],
            open: vec![],
            record: vec![cand(0, 7, VoiceSide::Long, 0)], // 𝒦_x：记录不执行
        };
        let (active, p) = coverage_step_from_buckets(&flat_elements(&buckets.open), 0, &[], &buckets, 1000.0, &cfg());
        assert!(active.is_empty(), "𝒦_x 不入活动集");
        assert_eq!(p, 0.0);
    }

    /// 空三桶 + 空 A_t ⟹ A_{t+1}=∅，p̃=0（边界）。
    #[test]
    fn buckets_empty_yields_empty_and_zero() {
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        let (active, p) = coverage_step_from_buckets(&flat_elements(&buckets.open), 0, &[], &buckets, 1000.0, &cfg());
        assert!(active.is_empty());
        assert_eq!(p, 0.0);
    }

    /// ★环5+环6 端到端：Classification（一买点）→ Γ → 解释器 → A_{t+1} → p̃。
    #[test]
    fn classification_end_to_end_ring5_ring6() {
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
        let (active, p) = coverage_step_classification(&classification, &[], &[], 1000.0, &cfg());
        assert_eq!(active.len(), 1, "买点 ℬ_x 开启 ⟹ 一条活动腿");
        assert_eq!(active[0].dir, VoiceSide::Long);
        assert_eq!(active[0].source_index, 4);
        assert!((p - 600.0).abs() < 1e-9, "p̃ = base×w[0] = 600");
    }

    /// ★环5↔环6 闭环：A_{t+1} 回喂 interpret——持仓 Long 遇反向卖点 ⟹ 关闭，A_{t+2}=∅。
    #[test]
    fn ring6_active_set_feeds_back_into_interpret() {
        // bar t：买点开 Long。
        let buy = BspPoint {
            source_index: 0,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90, pivot_high: 0,
            center: Some(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 }),
        };
        let c_buy = Classification { levels: vec![LevelState { bsp: vec![buy], ..Default::default() }] };
        let (active_t1, _) = coverage_step_classification(&c_buy, &[], &[], 1000.0, &cfg());
        assert_eq!(active_t1.len(), 1, "买点开 Long 腿");
        // bar t+1：卖点（反向）→ A_{t+1} 回喂 interpret ⟹ 关闭 Long 腿 ⟹ A_{t+2}=∅。
        let sell = BspPoint {
            source_index: 10,
            bits: BspBits { sell1: true, ..Default::default() },
            pivot_low: 0, pivot_high: 210,
            center: Some(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 }),
        };
        let c_sell = Classification { levels: vec![LevelState { bsp: vec![sell], ..Default::default() }] };
        let (active_t2, p2) = coverage_step_classification(&c_sell, &[], &active_t1, 1000.0, &cfg());
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
        let tower = vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ];
        // L0 卖候选 src=8 ⟹ host=sub(4,8) ⟹ 真父 L1 Long ⟹ ShortDiff（δ=Short=−σ_p）。
        let classification = Classification {
            levels: vec![LevelState { bsp: vec![sell_bsp(8)], ..Default::default() }],
        };
        let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
        let gamma = assemble_gamma_with_tower(&classification, &tower);
        assert_eq!(gamma[0].role.v, Vertical::ShortDiff, "前置：候选 V=ShortDiff（639 σ_p=父容器方向）");
        // 持父仓：prev_active 含父容器腿（L1 Long，638 身份 source_index=compose.ρ=12）。
        let held_parent = ActiveLeg { level: 1, dir: VoiceSide::Long, source_index: 12, lambda: 0 };
        let buckets = interpret(&gamma, &[held_parent]);
        let (active, _p) =
            coverage_step_from_buckets(&elements, cstart, &[held_parent], &buckets, 1000.0, &cfg());
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
        let tower = vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ];
        let classification = Classification {
            levels: vec![LevelState { bsp: vec![sell_bsp(8)], ..Default::default() }],
        };
        let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
        let gamma = assemble_gamma_with_tower(&classification, &tower);
        assert_eq!(gamma[0].role.v, Vertical::ShortDiff, "前置：候选 V=ShortDiff（639 σ_p=父容器方向）");
        // ★持仓父腿 ρ 漂移：source_index=8（开仓时旧 ρ，现树无 ρ=8 的 L1 元素），λ=0（稳定起点）。
        // 当前树 L1 元素 ρ=12（已延伸）⟹ Exact(ρ==8) 失配 ⟹ CoordDrift(λ==0, ρ=12≥8) 命中同一父。
        let held_parent = ActiveLeg { level: 1, dir: VoiceSide::Long, source_index: 8, lambda: 0 };
        let buckets = interpret(&gamma, &[held_parent]);
        let (active, _p) =
            coverage_step_from_buckets(&elements, cstart, &[held_parent], &buckets, 1000.0, &cfg());
        assert!(
            active.iter().any(|l| l.level == 0 && l.dir == VoiceSide::Short),
            "coord_drift（λ 稳定对位）⟹ 延伸父仍在 raw ⟹ ShortDiff 子腿准入（Q4 假阴性消除）；实得 {active:?}"
        );
        assert!(
            active.iter().any(|l| l.level == 1 && l.dir == VoiceSide::Long),
            "延伸父容器腿对位回当前树元素并保留（非降 orphan）"
        );
    }

    /// ★测试②（AncOK 剪枝，639(c) 关键测试）：有向父容器 + **未持父仓** + 逆向次级 ShortDiff 候选 ⟹
    /// ShortDiff 子腿**被剔除**（不开 naked 逆势仓 garbage trade）。这是 639(c) 承诺的兑现。
    #[test]
    fn ancok_prunes_shortdiff_when_parent_unheld() {
        use super::super::interp::{assemble_gamma_with_tower, coverage_elements_with_tower, interpret};
        let tower = vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ];
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
        let (active, p) = coverage_step_from_buckets(&elements, cstart, &[], &buckets, 1000.0, &cfg());
        assert!(
            active.is_empty(),
            "未持父 ⟹ ShortDiff 子腿被 AncOK 剪枝（639(c)：不开 naked 逆势仓）；实得 {active:?}"
        );
        assert_eq!(p, 0.0, "ShortDiff 剔除 ⟹ 无活动腿 ⟹ p̃=0（不开仓）");
    }

    /// ★测试③（根级无父要求）：Ambient 根候选（缺塔/host 是根，σ_p=0）⟹ 空持仓也正常准入。
    #[test]
    fn ancok_admits_ambient_root_without_held_parent() {
        use super::super::interp::{assemble_gamma_with_tower, coverage_elements_with_tower, interpret};
        let tower: Vec<Vec<LeveledMove>> = Vec::new(); // 缺塔 ⟹ 候选父=∂ ⟹ Ambient 根
        let classification = Classification {
            levels: vec![LevelState { bsp: vec![buy_bsp(4)], ..Default::default() }],
        };
        let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
        let gamma = assemble_gamma_with_tower(&classification, &tower);
        assert_eq!(gamma[0].role.v, Vertical::Ambient, "缺塔 ⟹ 候选父=∂ ⟹ Ambient 根");
        let buckets = interpret(&gamma, &[]); // 空持仓
        let (active, p) = coverage_step_from_buckets(&elements, cstart, &[], &buckets, 1000.0, &cfg());
        assert_eq!(active.len(), 1, "Ambient 根腿无父要求 ⟹ 空持仓也准入");
        assert_eq!(active[0].dir, VoiceSide::Long);
        assert!((p - 600.0).abs() < 1e-9, "根 depth 0 ⟹ p̃=base×w[0]=600");
    }

    /// ★测试④（持父→撤父→连带剪枝，覆盖不漂浮）：先持父仓准入 ShortDiff 子腿，下一 bar 父腿被反向
    /// 关闭（𝒟_x）⟹ 子腿同 bar 失祖先 ⟹ AncOK 连带剪枝（spec §13 覆盖不漂浮）。
    #[test]
    fn ancok_prunes_child_when_parent_closed_same_step() {
        use super::super::interp::{assemble_gamma_with_tower, coverage_elements_with_tower, interpret};
        let tower = vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ];
        // 同 bar 两候选：L1 卖（反向关闭 L1 Long 父腿）+ L0 卖（ShortDiff 子腿）。
        let classification = Classification {
            levels: vec![
                LevelState { bsp: vec![sell_bsp(8)], ..Default::default() }, // L0：ShortDiff 子
                LevelState { bsp: vec![sell_bsp(12)], ..Default::default() }, // L1：反向关父
            ],
        };
        let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
        let gamma = assemble_gamma_with_tower(&classification, &tower);
        // 持父仓（L1 Long）。
        let held_parent = ActiveLeg { level: 1, dir: VoiceSide::Long, source_index: 12, lambda: 0 };
        let buckets = interpret(&gamma, &[held_parent]);
        // L1 卖反向关闭 L1 Long 父腿（𝒟_x），故 (A_t∖𝒟_x) 不含父 ⟹ 子腿失祖先 ⟹ AncOK 连带剪枝。
        assert!(buckets.close.iter().any(|l| l.level == 1), "L1 卖反向关闭 L1 Long 父腿（𝒟_x）");
        let (active, _p) =
            coverage_step_from_buckets(&elements, cstart, &[held_parent], &buckets, 1000.0, &cfg());
        assert!(
            !active.iter().any(|l| l.level == 0 && l.dir == VoiceSide::Short),
            "父腿同 bar 关闭 ⟹ ShortDiff 子腿连带剪枝（覆盖不漂浮）；实得 {active:?}"
        );
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
            KThetaRiskGate::open(),
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
        let a = pi_theta_step(&classification, &[], &[], 0.0, 1, 1000.0, &cfg(), &r, w, KThetaRiskGate::open());
        let b = pi_theta_step(&classification, &[], &[], 0.0, 1, 1000.0, &cfg(), &r, w, KThetaRiskGate::open());
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
        let tower = vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ];
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
            PiThetaWeights::from_risk(&r), KThetaRiskGate::open(),
        );
        assert!(a.is_empty(), "未持父 ⟹ ShortDiff 子腿 AncOK 剪枝 ⟹ A_{{t+1}} 空");
        assert_eq!(p_star, 0.0, "无活动腿 ⟹ p*=0（不开仓）");
        assert_eq!(order.action, StrictAction::Wait, "未持父 ⟹ ShortDiff 剔除 ⟹ Wait（639(c)）");
        assert_eq!(order.qty, 0, "Wait ⟹ qty=0（不建 naked 逆势仓）");
        assert_eq!(order.exec_index, 9, "订单携 exec_index（延迟成交 bar）");
    }
}
