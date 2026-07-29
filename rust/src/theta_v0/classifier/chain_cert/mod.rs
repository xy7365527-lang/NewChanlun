//! #641（N3）级别链证书塔对象（#636 裁定 comment-5119573446 落地，SPEC 父票 #529）。
//!
//! 节点 = #550/#551 的背驰段候选事件（身份键沿用 [`CandidateKey`]，本模块不新造身份）；
//! 边 = #552 的跨级 `C⊆C` 谓词 [`candidate_is_sub`] + E2E-L 谱系（`Open/Closed/Invalidated`
//! 三态 + skip edge）。本模块**纯产出零消费**：不被 BSP / 级别形成 / 门 / admission / 订单
//! 路径调用，调用点 = 本模块单测 + 诊断 bin + p123 侧信道 dump（只写不判）。
//!
//! ## 命名独立（#636 裁定⑤）
//!
//! 本对象叫 [`TowerChainCertificate`]，与 nest 对照侧的 `NestCertificate` 是**两个词**：后者是
//! ADR-0005 钉死的独立对照实现产物（`NestInterval` 时间坐标、`assemble_certificate_snapshot`
//! 严格相邻装配），本对象是塔内原生链谱系（`source_index` 坐标、Hasse 覆盖边、skip 合法）。
//! 两者不共享类型、不互相调用、不互为验证。
//!
//! ## 语义（逐条对应 #636 裁定）
//!
//! 1. **谱系照实**（裁定②）：缺失（[`SkippedLevel`]）、证伪（[`ChainNodeStatus::Falsified`]）、
//!    跳过（[`ChainEdgeKind::Skip`]）全部留痕；append-only 修订，旧 revision 保留，禁删除模拟
//!    失效；**禁伪造中间级证书**——本模块任何路径都不构造 [`CandidateKey`]，节点只能来自事件流。
//! 2. **链段有效性统一裁**（裁定②）：一律由 `C⊆C` 直接谓词在**存活端点**间判定。谓词判过的
//!    skip 边与相邻边**同权**（[`ChainEdge::is_segment`] 不看 [`ChainEdge::kind`]）；判不过的边
//!    仍然留在 [`TowerChainCertificate::edges`] 里作**事实边**，但不构成链段。
//! 3. **证伪节点不判死上级**（裁定③）：路径中某节点的事件转 `Invalidated` 时，它降为
//!    [`ChainNodeStatus::Falsified`] 留痕并被**跨过**，链段改由它两侧的存活端点直接判——链的死活
//!    由谓词裁，不由该节点裁。
//! 4. **终态三态**（裁定②）：`Closed` = 链头 `Confirmed` + 链不可再扩展 + 全链段谓词判过；
//!    `Invalidated` = 谓词判不过或链头 `Invalidated`；否则 `Open`。终态不复活。
//! 5. **路径扩展走新 key**（裁定②）：加子节点 = 新 [`ChainKey`]，其
//!    [`TowerChainCertificate::extends`] 指向唯一最长 proper-prefix key（E2E-L
//!    `extends_lineage_key` 同义），旧路径不被篡改。
//!
//! ## 边 = 包含序的**覆盖关系**（构造口径，本实装的判定，非新裁）
//!
//! `C⊆C` 是传递的（闭区间包含 + 级别严格序），故若 `c ⊆ m ⊆ p` 则 `c ⊆ p` 也成立。若把全部
//! 包含对都当链的边，一条 `p→m→c` 的下降会同时产出 `p→c` 这条**传递闭包边**，链数按路径长度
//! 组合爆炸，且 `p→c` 会把一个**真实存在且套得住**的中间级候选 `m` 说成「跳过」——那正是
//! roadmap:64「不得为缺失的中间级别伪造证书」的镜像违规（伪造的是「缺失」这个事实本身）。
//!
//! 故本模块的边取包含序的**覆盖关系**（Hasse 边）：`(c,p)` 是边 ⟺ `c ⊆ p` 且**不存在**存活
//! 候选 `m` 使 `c ⊆ m ⊆ p`（级别严格居中）。教义依据：027:46 的逐级收缩（「在次级别图里找出
//! 相应背驰段，反复进行下去」）与 chan99 §六「第一种情况（完全契合、逐级监控）最普遍」——
//! **能逐级就必须逐级**；skip 边只在中间级别**真缺**（该级无候选）或**真断**（该级有候选但套
//! 不住）时出现，这两种情形由 [`SkippedLevel`] 逐级分别记数，不混同。
//!
//! ## 链的构造口径：极大路径（root→leaf）
//!
//! 027:46 的过程是「从大级别转折点出发，逐级向下，直到最低级别」——一条链是一次**完整下降**。
//! 故 [`chain_paths`] 产出覆盖关系图中的**极大路径**：root 无存活父、leaf 无存活子。单节点路径
//! 不是链（至少要有一重区间套关系），单列计数不静默丢。
//!
//! **未做项（照实）**：路径分叉时选哪一支是 E2E-L 的 `selection_policy`，#636 裁定未涉及，本
//! 模块**不选**（`selection_policy = null`），全部分支各成一条链；44 课「小转大」的替代必要条件
//! （次级别中枢第三类买卖点）证据路径由 #636 裁定③明确**留 fog**，skip 边因此**不携带**任何替代
//! 证据——它只是「这里跳了一级 + 跳过处的事实计数」，比 44 课教义门槛宽，此事实随对象一同登记。
//!
//! ## 复杂度（照实声明）
//!
//! [`chain_paths`] 的覆盖边计算是 `Σ_p O(|⊆p|²)`，路径枚举是输出敏感的 DFS（无上限、无采样、
//! 无截断）。真实窗口读数见 `chanlun/review-results/issue641-n3-chain-impl-20260729.md`。本模块
//! **不在**任何逐 bar 热路径上；若将来要接进逐 bar 循环，须先改判据结构，而不是把窗口调小了事。

use std::collections::{BTreeMap, BTreeSet};

use super::cand_event::{CandidateEvent, CandidateKey, CandidateState, CandidateStreams};
use super::cand_sub::candidate_is_sub;

#[cfg(test)]
mod tests;

/// 链判定规则版本（E2E-L「规则版本变 ⟹ 新 key，禁复活旧 key」的版本分量）。
///
/// 随**链判定规则**（边的覆盖关系口径、极大路径口径、状态映射、留痕字段构成）的任何实质变更
/// 递增。版本进 [`ChainKey`] ⟹ 规则变更后旧 key 不再被观察到；旧 key 的最后一条 revision 原样
/// 留在簿里（链不走「缺席失效」，见 [`ChainCertificateBook::advance`] 文档），新规则的链以新
/// key 从 ∅ 开始，旧生命史不被改写。
pub const CHAIN_RULE_VERSION: u32 = 1;

/// E2E-L 链谱系三态（#636 裁定②）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChainStatus {
    Open,
    Closed,
    Invalidated,
}

impl ChainStatus {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Closed | Self::Invalidated)
    }
}

/// 边的级别形态。**只作留痕，不参与链段有效性判定**（裁定③「同权」的类型层体现）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChainEdgeKind {
    /// 两存活端点级别相邻（`parent.level == child.level + 1`）。
    Adjacent,
    /// 两存活端点之间隔了至少一级（`parent.level > child.level + 1`）。
    Skip,
}

/// 路径节点在本次 `as_of` 的照实状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChainNodeStatus {
    /// 事件在案且非 `Invalidated`——存活端点。
    Alive,
    /// 事件在案且为 `Invalidated`——证伪留痕，被跨过，不判死上级。
    Falsified,
    /// 事件流里查无此 key。
    ///
    /// 由**因果簿**（`classify_with_tower_events_incremental` 的 `TowerCache` 正本）驱动时不可达
    /// ——那条流 append-only，key 一旦出现即永存。由**终态窗口投影**
    /// （`classify_with_tower_events` 每次 fresh book）驱动时可达：fresh 无记忆，上一 `as_of`
    /// 的身份可以整个消失。两种驱动本模块都支持，故本变体是真实可达分支，按「不属存活端点」
    /// 处理并单列计数（[`chain_probe::ChainProbe::absent_node`]），不与 `Falsified` 混同。
    Absent,
}

/// Bₚ 的稳定结构指纹之外，链层要留的**每个被跳过级别**的事实计数（裁定②「缺失/跳过留痕」）。
///
/// 区分「缺」与「断」：`alive_at_level == 0` ⟹ 该级别在本窗口整体没有存活候选（**缺**）；
/// `alive_at_level > 0 && inside_parent == 0` ⟹ 该级别有候选但没有一个落在父端点区间里（**断**
/// 的一种）；`inside_parent > 0` ⟹ 该级别有候选落在父区间内，只是没有一个同时套住子端点
/// （**断**的另一种——中间级别在场却接不上）。三者不混同，消费端自取。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SkippedLevel {
    pub level: u32,
    /// 该级别的存活候选总数。
    pub alive_at_level: usize,
    /// 其中区间被**父端点**包含者的个数。
    pub inside_parent: usize,
}

/// 路径上一个节点的留痕。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainNodeTrace {
    pub key: CandidateKey,
    pub level: u32,
    pub status: ChainNodeStatus,
    /// 事件在案时的候选状态；[`ChainNodeStatus::Absent`] 时为 `None`。
    pub state: Option<CandidateState>,
}

/// 两个**存活端点**之间的一条边。
///
/// 谓词判过 ⟹ 链段（[`Self::is_segment`]）；判不过 ⟹ **事实边**，照实留在
/// [`TowerChainCertificate::edges`] 里但不构成链段，并使整条链转 `Invalidated`（裁定②）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainEdge {
    /// 上端（级别大）存活端点。
    pub parent: CandidateKey,
    /// 下端（级别小）存活端点。
    pub child: CandidateKey,
    pub kind: ChainEdgeKind,
    /// 两端点之间被跳过的级别，按级别升序；[`ChainEdgeKind::Adjacent`] 时为空。
    pub skipped_levels: Vec<SkippedLevel>,
    /// 被跨过的**路径节点**（证伪或查无），按路径序。空 ⟹ 本边不是由证伪跨越产生的。
    pub crossed_nodes: Vec<CandidateKey>,
    /// `C⊆C` 直接谓词在两存活端点上的取值（[`candidate_is_sub`]，唯一判定来源）。
    pub predicate_holds: bool,
}

impl ChainEdge {
    /// 是否构成链段。**只看谓词**——skip 与 adjacent 同权（裁定③）。
    pub fn is_segment(&self) -> bool {
        self.predicate_holds
    }
}

/// 链证书身份：不可变的候选身份路径（root→leaf，级别严格递减）+ 规则版本。
///
/// 与 E2E-L `LineageKey` 同构的裁剪版：`ordered_EventKey_path` 落为 [`Self::path`]，
/// `lineage_rule_version` 落为 [`Self::rule_version`]；`ordered_EdgeKey_path` 不入键——边的形态
/// （Adjacent/Skip、跨过哪些节点）由路径与当时的事件状态**派生**，把派生量放进不可变身份会让
/// 同一条路径在证伪前后变成两个身份，与「证伪节点留痕但不判死上级」冲突。
/// `selection_policy_id/version` 恒 null（本模块不选支，见模块头「未做项」）。
/// Floor/status/clocks/revision 均不入键（与 E2E-L 同）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainKey {
    pub rule_version: u32,
    pub path: Vec<CandidateKey>,
}

impl ChainKey {
    /// 从 root→leaf 路径构造。
    ///
    /// 断言路径 ≥2 个节点：单节点不是链（027:46 的区间套至少要有一重「父背驰段内部的背驰段」
    /// 关系）。构造点全在本模块内（[`chain_paths`] 与簿内驻留路径），越过它构造是编程错误，
    /// 当场失败而不是产出一个没有任何链段的畸形证书。
    pub fn new(path: Vec<CandidateKey>) -> Self {
        assert!(
            path.len() >= 2,
            "链身份至少两个节点（单节点无区间套关系），实得 {}",
            path.len()
        );
        Self {
            rule_version: CHAIN_RULE_VERSION,
            path,
        }
    }

    pub fn root(&self) -> &CandidateKey {
        self.path.first().expect("ChainKey::new 保证路径非空")
    }

    pub fn leaf(&self) -> &CandidateKey {
        self.path.last().expect("ChainKey::new 保证路径非空")
    }

    /// 唯一最长 proper-prefix key（E2E-L `extends_lineage_key`）。
    ///
    /// 去掉 leaf 后仍是合法链（≥2 节点）时为 `Some`，否则 `None`。**向下扩展**（加子节点）才是
    /// E2E-L 意义上的路径扩展；向上多出一个新 root 是另一条链的身份，不构成 prefix 关系，故其
    /// `extends` 为 `None`。返回的是**键引用**，不保证该 key 曾作为对象在簿中物化过。
    pub fn proper_prefix(&self) -> Option<ChainKey> {
        (self.path.len() >= 3).then(|| ChainKey {
            rule_version: self.rule_version,
            path: self.path[..self.path.len() - 1].to_vec(),
        })
    }
}

/// 一等链证书的一次不可变 revision。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TowerChainCertificate {
    pub key: ChainKey,
    /// E2E-L `extends_lineage_key`：本链所扩展的最长 proper-prefix 路径键。
    pub extends: Option<ChainKey>,
    pub root_level: u32,
    pub leaf_level: u32,
    /// 路径全节点留痕，按 root→leaf。长度恒等于 `key.path.len()`。
    pub nodes: Vec<ChainNodeTrace>,
    /// 存活端点间的边，按 root→leaf。
    pub edges: Vec<ChainEdge>,
    /// 本次 `as_of` 下是否还能向下扩展（最低存活端点内部还有存活候选）。
    pub extendable: bool,
    pub status: ChainStatus,
    /// 链首次入簿时的 `as_of`（一次写入不后移）。
    pub observed_at: usize,
    /// 首次进入 `Closed` 的 `as_of`（终态，一次写入）。
    pub closed_at: Option<usize>,
    /// 首次进入 `Invalidated` 的 `as_of`（终态，一次写入）。
    pub invalidated_at: Option<usize>,
    pub revision: u32,
    pub supersedes_revision: Option<u32>,
    pub revision_at: usize,
}

/// [`TowerChainCertificate`] 的业务载荷投影 —— 链「是什么」的**唯一比较口径**。
///
/// 进投影的是链的结构载荷与判定结果；**不进投影**的是生命史记账（三只钟 + revision 计数）。
/// 理由与 `cand_event::CandidateProjection` 同：同一份载荷在不同 `as_of` 重跑必然带不同的钟，
/// 计入等价比较会让「载荷未变」永远判不成立、幂等永远达不到。`key` 亦不入投影（它是簿的索引，
/// 全部比对都在同 key 内进行）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainProjection {
    pub extends: Option<ChainKey>,
    pub root_level: u32,
    pub leaf_level: u32,
    pub nodes: Vec<ChainNodeTrace>,
    pub edges: Vec<ChainEdge>,
    pub extendable: bool,
    pub status: ChainStatus,
}

impl TowerChainCertificate {
    pub fn projection(&self) -> ChainProjection {
        ChainProjection {
            extends: self.extends.clone(),
            root_level: self.root_level,
            leaf_level: self.leaf_level,
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
            extendable: self.extendable,
            status: self.status,
        }
    }

    /// 链段数（谓词判过的边）。
    pub fn segment_count(&self) -> usize {
        self.edges.iter().filter(|edge| edge.is_segment()).count()
    }

    /// 事实边数（谓词判不过、照实留痕但不构成链段的边）。
    pub fn fact_edge_count(&self) -> usize {
        self.edges.iter().filter(|edge| !edge.is_segment()).count()
    }
}

/// 单次链扫描给簿的业务投影（不含钟与 revision）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainObservation {
    pub key: ChainKey,
    pub extends: Option<ChainKey>,
    pub root_level: u32,
    pub leaf_level: u32,
    pub nodes: Vec<ChainNodeTrace>,
    pub edges: Vec<ChainEdge>,
    pub extendable: bool,
    pub status: ChainStatus,
}

// ── 事件索引（每 key 最新 revision + 存活分级） ────────────────────────────────────────────

/// 一次扫描的事件视图：每 key 最新 revision，另按级别分组存活事件。
///
/// 取最新 revision 而非全史（口径与 `cand_sub::latest_by_level` 同）：链问的是「当前几何」，
/// 历史 revision 是同一候选的旧几何。存活 = 状态非 `Invalidated`。
struct AliveIndex<'a> {
    latest: BTreeMap<CandidateKey, &'a CandidateEvent>,
    alive_by_level: BTreeMap<u32, Vec<&'a CandidateEvent>>,
}

impl<'a> AliveIndex<'a> {
    fn build(streams: &'a CandidateStreams) -> Self {
        let mut latest = BTreeMap::<CandidateKey, &'a CandidateEvent>::new();
        for stream in streams.iter() {
            for event in stream.iter() {
                latest.insert(event.key, event);
            }
        }
        let mut alive_by_level = BTreeMap::<u32, Vec<&'a CandidateEvent>>::new();
        for event in latest.values() {
            if event.state != CandidateState::Invalidated {
                alive_by_level
                    .entry(event.event_level)
                    .or_default()
                    .push(event);
            }
        }
        Self {
            latest,
            alive_by_level,
        }
    }

    /// 全部存活事件，级别升序、组内 key 升序（确定序 ⟹ 产出可逐字节比对）。
    fn alive(&self) -> impl Iterator<Item = &'a CandidateEvent> + '_ {
        self.alive_by_level
            .values()
            .flat_map(|events| events.iter().copied())
    }

    /// 区间被 `parent` 包含的全部存活事件（`C⊆C` 成立者，含跨多级）。
    fn contained_in(&self, parent: &CandidateEvent) -> Vec<&'a CandidateEvent> {
        self.alive()
            .filter(|child| candidate_is_sub(child, parent))
            .collect()
    }

    fn alive_at(&self, level: u32) -> &[&'a CandidateEvent] {
        self.alive_by_level
            .get(&level)
            .map_or(&[][..], |events| events.as_slice())
    }
}

// ── 覆盖关系与极大路径 ──────────────────────────────────────────────────────────────────

/// 覆盖关系（Hasse）子节点表：`parent key → 直接子节点 key（级别升序、key 升序）`。
///
/// `c` 是 `p` 的覆盖子 ⟺ `c ⊆ p` 且不存在存活 `m` 使 `c ⊆ m ⊆ p`。因为「级别严格居中的 m」
/// 必然自己也 `⊆ p`，判定只需在 `contained_in(p)` 这一集合内做，无需再扫全表。
fn hasse_children(index: &AliveIndex<'_>) -> BTreeMap<CandidateKey, Vec<CandidateKey>> {
    let mut table = BTreeMap::<CandidateKey, Vec<CandidateKey>>::new();
    for parent in index.alive() {
        let inside = index.contained_in(parent);
        let mut direct = Vec::new();
        for child in &inside {
            let subdivided = inside.iter().any(|middle| {
                middle.event_level > child.event_level && candidate_is_sub(child, middle)
            });
            if !subdivided {
                direct.push(child.key);
            }
        }
        table.insert(parent.key, direct);
    }
    table
}

/// 覆盖关系图中的**极大路径**（root 无存活父、leaf 无存活子），按路径字典序去重后升序。
///
/// 单节点极大路径（既无父也无子的孤立候选）不是链，不产出——它们的个数由
/// [`chain_probe::ChainProbe::isolated_roots`] 计数，不静默丢。
pub fn chain_paths(streams: &CandidateStreams) -> Vec<Vec<CandidateKey>> {
    let index = AliveIndex::build(streams);
    paths_from_index(&index)
}

fn paths_from_index(index: &AliveIndex<'_>) -> Vec<Vec<CandidateKey>> {
    let children = hasse_children(index);
    let mut has_parent = BTreeSet::<CandidateKey>::new();
    for kids in children.values() {
        has_parent.extend(kids.iter().copied());
    }
    let mut paths = BTreeSet::<Vec<CandidateKey>>::new();
    for root in index.alive() {
        if has_parent.contains(&root.key) {
            continue;
        }
        let kids = children.get(&root.key).map_or(0, Vec::len);
        if kids == 0 {
            #[cfg(test)]
            chain_probe::on_isolated_root();
            continue;
        }
        let mut stack = vec![root.key];
        descend(&children, &mut stack, &mut paths);
    }
    paths.into_iter().collect()
}

fn descend(
    children: &BTreeMap<CandidateKey, Vec<CandidateKey>>,
    stack: &mut Vec<CandidateKey>,
    out: &mut BTreeSet<Vec<CandidateKey>>,
) {
    let tail = *stack.last().expect("descend 入口保证栈非空");
    let kids = children.get(&tail).map_or(&[][..], Vec::as_slice);
    if kids.is_empty() {
        if stack.len() >= 2 {
            out.insert(stack.clone());
        }
        return;
    }
    for kid in kids {
        stack.push(*kid);
        descend(children, stack, out);
        stack.pop();
    }
}

// ── 单条路径的照实评估 ──────────────────────────────────────────────────────────────────

/// 对一条**固定路径**在本次 `as_of` 的事件视图上做照实评估。
///
/// 路径身份不因节点证伪而改变（裁定②「append-only、不篡改旧路径」）；变的是节点留痕、存活端点
/// 之间的边、可扩展性与三态。同一份事件视图上重复调用返回相同结果（纯函数）。
fn evaluate(path: &[CandidateKey], index: &AliveIndex<'_>) -> ChainObservation {
    let key = ChainKey::new(path.to_vec());
    let nodes: Vec<ChainNodeTrace> = path
        .iter()
        .map(|node| {
            let event = index.latest.get(node);
            let status = match event.map(|event| event.state) {
                None => ChainNodeStatus::Absent,
                Some(CandidateState::Invalidated) => ChainNodeStatus::Falsified,
                Some(_) => ChainNodeStatus::Alive,
            };
            ChainNodeTrace {
                key: *node,
                level: node.level,
                status,
                state: event.map(|event| event.state),
            }
        })
        .collect();

    let alive_positions: Vec<usize> = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.status == ChainNodeStatus::Alive)
        .map(|(i, _)| i)
        .collect();

    let edges: Vec<ChainEdge> = alive_positions
        .windows(2)
        .map(|pair| build_edge(&nodes, pair[0], pair[1], index))
        .collect();

    let extendable = alive_positions.last().is_some_and(|&last| {
        let leaf = index.latest[&nodes[last].key];
        index.alive().any(|other| candidate_is_sub(other, leaf))
    });

    let head_alive = nodes[0].status == ChainNodeStatus::Alive;
    let head_confirmed = nodes[0].state == Some(CandidateState::Confirmed);
    let all_segments = edges.iter().all(ChainEdge::is_segment);
    let status = if !head_alive || !all_segments {
        ChainStatus::Invalidated
    } else if head_confirmed && !extendable {
        ChainStatus::Closed
    } else {
        ChainStatus::Open
    };

    #[cfg(test)]
    chain_probe::on_evaluate(&nodes, &edges);

    ChainObservation {
        extends: key.proper_prefix(),
        root_level: key.root().level,
        leaf_level: key.leaf().level,
        key,
        nodes,
        edges,
        extendable,
        status,
    }
}

fn build_edge(
    nodes: &[ChainNodeTrace],
    upper: usize,
    lower: usize,
    index: &AliveIndex<'_>,
) -> ChainEdge {
    let parent_key = nodes[upper].key;
    let child_key = nodes[lower].key;
    let parent = index.latest[&parent_key];
    let child = index.latest[&child_key];
    let kind = if parent.event_level == child.event_level + 1 {
        ChainEdgeKind::Adjacent
    } else {
        ChainEdgeKind::Skip
    };
    let skipped_levels = (child.event_level + 1..parent.event_level)
        .map(|level| {
            let at_level = index.alive_at(level);
            SkippedLevel {
                level,
                alive_at_level: at_level.len(),
                inside_parent: at_level
                    .iter()
                    .filter(|event| candidate_is_sub(event, parent))
                    .count(),
            }
        })
        .collect();
    ChainEdge {
        parent: parent_key,
        child: child_key,
        kind,
        skipped_levels,
        crossed_nodes: nodes[upper + 1..lower]
            .iter()
            .map(|node| node.key)
            .collect(),
        predicate_holds: candidate_is_sub(child, parent),
    }
}

// ── append-only 修订簿 ──────────────────────────────────────────────────────────────────

/// 链证书 append-only 修订簿。终态不复活；同一 `as_of` 重跑零 Delta。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChainCertificateBook {
    certificates: Vec<TowerChainCertificate>,
    latest: BTreeMap<ChainKey, usize>,
}

impl ChainCertificateBook {
    /// 全部 revision，落簿序（append-only，旧 revision 永不改写、永不删除）。
    pub fn certificates(&self) -> &[TowerChainCertificate] {
        &self.certificates
    }

    pub fn latest_of(&self, key: &ChainKey) -> Option<&TowerChainCertificate> {
        self.latest.get(key).map(|index| &self.certificates[*index])
    }

    /// 每 key 最新 revision，key 升序。
    pub fn heads(&self) -> Vec<&TowerChainCertificate> {
        self.latest
            .values()
            .map(|index| &self.certificates[*index])
            .collect()
    }

    /// 用本次 `as_of` 的候选事件流推进一步，返回本次追加的 Delta。
    ///
    /// 被评估的路径 = 本次扫描的极大路径 ∪ **簿内非终态**链的路径。后者不可省：一条链被向下
    /// 扩展后就不再是极大路径，若只评估极大路径，它会在扫描里「消失」。链与候选不同，**没有
    /// 「缺席即失效」这条路径**——链的死活只由裁定②的两个条件（谓词判不过 / 链头 Invalidated）
    /// 决定，而这两个条件都是在**重新评估**里读出来的，不是从「本轮没看见」推出来的。
    ///
    /// 簿内**终态**链不重评（终态不复活）。
    pub fn advance(
        &mut self,
        streams: &CandidateStreams,
        as_of: usize,
    ) -> Vec<TowerChainCertificate> {
        let index = AliveIndex::build(streams);
        let mut paths: BTreeSet<Vec<CandidateKey>> = paths_from_index(&index).into_iter().collect();
        let resident: Vec<Vec<CandidateKey>> = self
            .latest
            .iter()
            .filter(|(_, position)| !self.certificates[**position].status.is_terminal())
            .map(|(key, _)| key.path.clone())
            .collect();
        paths.extend(resident);

        let mut delta = Vec::new();
        for path in paths {
            if let Some(certificate) = self.apply(evaluate(&path, &index), as_of) {
                delta.push(certificate);
            }
        }
        delta
    }

    fn apply(
        &mut self,
        observation: ChainObservation,
        as_of: usize,
    ) -> Option<TowerChainCertificate> {
        let prior = self.latest_of(&observation.key).cloned();
        if prior
            .as_ref()
            .is_some_and(|certificate| certificate.status.is_terminal())
        {
            #[cfg(test)]
            chain_probe::on_terminal_block();
            return None;
        }
        let next = make_revision(prior.as_ref(), observation, as_of);
        if prior
            .as_ref()
            .is_some_and(|certificate| certificate.projection() == next.projection())
        {
            #[cfg(test)]
            chain_probe::on_idempotent_skip();
            return None;
        }
        #[cfg(test)]
        chain_probe::on_append(prior.as_ref(), &next);
        self.append(next.clone());
        Some(next)
    }

    fn append(&mut self, certificate: TowerChainCertificate) {
        let position = self.certificates.len();
        self.latest.insert(certificate.key.clone(), position);
        self.certificates.push(certificate);
    }
}

fn make_revision(
    prior: Option<&TowerChainCertificate>,
    observation: ChainObservation,
    as_of: usize,
) -> TowerChainCertificate {
    // observed_at / closed_at / invalidated_at 均为「一次写入不后移」：已在案的值优先。
    let observed_at = prior.map_or(as_of, |certificate| certificate.observed_at);
    let closed_at = prior
        .and_then(|certificate| certificate.closed_at)
        .or((observation.status == ChainStatus::Closed).then_some(as_of));
    let invalidated_at = prior
        .and_then(|certificate| certificate.invalidated_at)
        .or((observation.status == ChainStatus::Invalidated).then_some(as_of));
    TowerChainCertificate {
        key: observation.key,
        extends: observation.extends,
        root_level: observation.root_level,
        leaf_level: observation.leaf_level,
        nodes: observation.nodes,
        edges: observation.edges,
        extendable: observation.extendable,
        status: observation.status,
        observed_at,
        closed_at,
        invalidated_at,
        revision: prior.map_or(0, |certificate| certificate.revision + 1),
        supersedes_revision: prior.map(|certificate| certificate.revision),
        revision_at: as_of,
    }
}

/// 链谱系覆盖探针（防真空绿；与 `cand_event::event_probe` 同形状）。
///
/// 模块常开、调用点 `#[cfg(test)]` ⟹ 生产零开销。计数按**簿与评估实际走过的分支**归类，
/// 测试据此断言「某条边形态 / 转移 / 留痕路径真被触发」，而不是只断言最终态。
pub mod chain_probe {
    use super::{
        ChainEdge, ChainEdgeKind, ChainNodeStatus, ChainNodeTrace, ChainStatus,
        TowerChainCertificate,
    };
    use std::cell::RefCell;

    #[derive(Default, Clone, Debug, PartialEq, Eq)]
    pub struct ChainProbe {
        /// ∅ → 首个 revision，按落地状态分解。
        pub birth_open: u64,
        pub birth_closed: u64,
        /// ★恒 0：首次观察的路径来自覆盖关系图（每条边的 `C⊆C` 按构造成立）且全节点存活
        /// ⟹ 既不可能「谓词判不过」也不可能「链头 Invalidated」。非零 = 构造口径被破坏。
        pub birth_invalidated: u64,
        /// 已有 revision 且状态改变的转移。
        pub to_closed: u64,
        pub to_invalidated: u64,
        /// 状态不变、载荷改变的修订。
        pub payload_revision: u64,
        /// 投影相同 ⟹ 零输出（幂等）。
        pub idempotent_skip: u64,
        /// 终态挡回的重评（禁复活）。
        pub terminal_block: u64,
        /// 评估中见到的边形态（同权证据：两者都能构成链段）。
        pub adjacent_edges: u64,
        pub skip_edges: u64,
        /// 谓词判不过、记为事实边的边数。
        pub fact_edges: u64,
        /// 路径上状态为 `Invalidated` 的节点数（证伪留痕）。
        pub falsified_nodes: u64,
        /// 被**边**跨过的路径节点数（证伪/查无夹在两个存活端点之间）。
        ///
        /// 与 [`Self::falsified_nodes`] 不等价：链头或链尾的证伪节点没有两侧存活端点，不被任何
        /// 边跨过。「证伪留痕但不判死上级」的机器证据是本计数非零 —— 上级与下级之间**新长出**
        /// 一条跨过它的边，链继续由谓词裁。
        pub crossed_by_edge: u64,
        /// 路径节点在事件流中查无（终态窗口投影驱动时可达）。
        pub absent_node: u64,
        /// 既无存活父也无存活子的孤立候选（不成链，单列不静默丢）。
        pub isolated_roots: u64,
    }

    thread_local! {
        static PROBE: RefCell<ChainProbe> = RefCell::new(ChainProbe::default());
    }

    pub fn reset() {
        PROBE.with(|probe| *probe.borrow_mut() = ChainProbe::default());
    }

    pub fn snapshot() -> ChainProbe {
        PROBE.with(|probe| probe.borrow().clone())
    }

    pub fn on_isolated_root() {
        PROBE.with(|probe| probe.borrow_mut().isolated_roots += 1);
    }

    pub fn on_evaluate(nodes: &[ChainNodeTrace], edges: &[ChainEdge]) {
        PROBE.with(|probe| {
            let mut probe = probe.borrow_mut();
            for node in nodes {
                match node.status {
                    ChainNodeStatus::Falsified => probe.falsified_nodes += 1,
                    ChainNodeStatus::Absent => probe.absent_node += 1,
                    ChainNodeStatus::Alive => {}
                }
            }
            for edge in edges {
                match edge.kind {
                    ChainEdgeKind::Adjacent => probe.adjacent_edges += 1,
                    ChainEdgeKind::Skip => probe.skip_edges += 1,
                }
                if !edge.predicate_holds {
                    probe.fact_edges += 1;
                }
                probe.crossed_by_edge += edge.crossed_nodes.len() as u64;
            }
        });
    }

    pub fn on_terminal_block() {
        PROBE.with(|probe| probe.borrow_mut().terminal_block += 1);
    }

    pub fn on_idempotent_skip() {
        PROBE.with(|probe| probe.borrow_mut().idempotent_skip += 1);
    }

    pub fn on_append(prior: Option<&TowerChainCertificate>, next: &TowerChainCertificate) {
        PROBE.with(|probe| {
            let mut probe = probe.borrow_mut();
            match prior {
                None => match next.status {
                    ChainStatus::Open => probe.birth_open += 1,
                    ChainStatus::Closed => probe.birth_closed += 1,
                    ChainStatus::Invalidated => probe.birth_invalidated += 1,
                },
                Some(prior) if prior.status == next.status => probe.payload_revision += 1,
                Some(_) => match next.status {
                    ChainStatus::Closed => probe.to_closed += 1,
                    ChainStatus::Invalidated => probe.to_invalidated += 1,
                    // ★不可达（禁复活边的机器载体，非「应该不会发生」的注释声明）。推导链：
                    // 1. 本函数唯一调用点是 `ChainCertificateBook::apply` 的落簿处，其入口已把
                    //    `prior.status.is_terminal()` 的重评全部挡回（return None）
                    //    ⟹ 走到这里的 `prior.status` 值域 = {`Open`}；
                    // 2. 本臂的前提是 `prior.status != next.status` ⟹ `next.status != Open`。
                    // 故「终态 → Open」（复活）与「Open → Open」（非转移）在此都不可能。静默记零
                    // 会让「该边从未发生」与「该边发生过但没人看见」不可区分。
                    ChainStatus::Open => unreachable!(
                        "E2E-L 禁止边「终态→Open」被走到：终态重评在 apply 入口已挡回，\
                         状态改变臂的 next 不可能是 Open（见 on_append 头部推导链）"
                    ),
                },
            }
        });
    }
}
