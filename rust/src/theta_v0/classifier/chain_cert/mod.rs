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
//! 4. **终态三态**（裁定② + #641 地板条款 comment-5121572134）：`Closed` = 链头 `Confirmed`
//!    + 链不可再扩展 + 全链段谓词判过 + **至少一条有效链段**；`Invalidated` = 谓词判不过或
//!    链头 `Invalidated`（成因分档见 [`ChainInvalidationCause`]）；否则 `Open`。终态不复活。
//! 5. **路径扩展走新 key**（裁定②）：加子节点 = 新 [`ChainKey`]，其
//!    [`TowerChainCertificate::extends`] 指向**簿内**最长 proper-prefix key（E2E-L
//!    `extends_lineage_key` 同义，见 [`ChainCertificateBook::resolve_extends`]），旧路径不被篡改。
//!
//! ## 地板条款（#641 comment-5121572134，2026-07-29 编排者裁定）
//!
//! 「全链段谓词判过」**不得在空集上真空成立**：链必须至少有一条有效链段（谓词判过的边，含
//! skip 边）才能 `Closed`。链头独活（全下级证伪/缺失 ⟹ 链段集合为空）= 永远 `Open`。与 E2E-L
//! `UnresolvedFloor` 语义对齐（原型 §5:188/§6.1，git `640609071d`）。机器载体 =
//! [`chain_probe::ChainProbe::floor_blocked`] + 单测
//! `head_only_survivor_stays_open_by_floor_conjunct`。
//!
//! **floor 完整口径**（E2E-L `CloseFloor_at` 三型 `FormalFloor / QuasiFloor / UnresolvedFloor`）
//! 仍留 fog 另裁（#641 取舍裁定 comment-5121793896 第 5 条）：本模块只落地「至少一条有效链段」
//! 这一格，不自行补级别地板（如「必须降到 L0/L1 才能 Close」）。
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

use super::cand_event::{
    interval_is_degenerate, CandidateEvent, CandidateKey, CandidateState, CandidateStreams,
    FNV_OFFSET_BASIS, FNV_PRIME,
};
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

/// 链转 [`ChainStatus::Invalidated`] 的成因（照实分档；只有裁定②授权的两支）。
///
/// #636 裁定②：「`Invalidated` = 谓词判不过 **或** 链头 `Invalidated`」。本枚举恰好两支，
/// **不含**「中间节点失效即判死全链」那条连坐支——它是 2026-07-29 核定明文 supersede 掉的原型
/// 文本，本模块的中间节点证伪走「留痕 + 被跨过 + 链继续由谓词裁」（裁定③，
/// `falsified_middle_node_is_crossed_and_does_not_kill_the_head`）。
///
/// 与三只钟（`observed_at` / `closed_at` / `invalidated_at`）同属**生命史记账**，故不进
/// [`ChainProjection`]（它由载荷派生：`nodes[0].status` 与 `edges` 的谓词取值已完整决定它，
/// 计入等价比较不增加分辨力，只会与钟一样让「载荷未变」判不成立）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChainInvalidationCause {
    /// 链头事件判 `Invalidated`。
    ///
    /// **口径照实**：本档只覆盖 `nodes[0].status` 为 [`ChainNodeStatus::Falsified`]；事件流中
    /// 查无的 [`ChainNodeStatus::Absent`] 是投影抖动，不是证伪，落 `Open` 不判死。
    HeadInvalidated,
    /// 两存活端点间 `C⊆C` 谓词判不过（该边成事实边，见 [`PredicateBreach`]）。
    PredicateFailed,
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
    ///
    /// **Absent 链头落 `Open`（#641 收口批，裁定②字面收窄）的连带后果照实（#667 INFO-3）**：
    /// 终态投影驱动下，该链**永久驻留**重评集合不再封口，且链头在 `Absent ↔ Alive` 间抖动会
    /// 反复刷 payload revision。两个消费 bin（`issue550_event_battery` / `p123_fast_replay`）
    /// 均为因果簿驱动，故真实数据面上 `nodes_absent` 恒 0、该驻留/增长面不可达；N7 若选
    /// fresh-book 驱动须先知此二事。
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

/// 链段资格的唯一判定谓词的**指名**（`cand_sub` 的跨级 `C⊆C`，本模块不重写任何区间不等式）。
///
/// **pub 导出面待收窄**（#667 LOW-1 订正 #641 S-6 登记）：零跨模块非测试调用者，可降
/// `pub(crate)`；但模块内生产路径在用（`build_edge` 写入每条事实边的 `breach.predicate`），
/// 按世代宪法 §1 名分 = **现役**，不是 deprecated。收窄与否则另票裁定。
pub const CHAIN_SEGMENT_PREDICATE: &str = "cand_sub::candidate_is_sub";

/// 事实边（谓词判不过）判不过的**直接原因**，按 [`candidate_is_sub`] 的合取项分档。
///
/// `candidate_is_sub = 跨级分支 ∧ interval_is_sub`，而
/// `interval_is_sub = ¬退化(child) ∧ ¬退化(parent) ∧ parent.0 <= child.0 ∧ child.1 <= parent.1`。
/// 本枚举按该合取式的求值序给出**唯一**的首个失败项——分档不是重判，是把生产谓词已经算出的
/// 假值按其构成拆开读（本模块不复算任何不等式，只读同一组端点值）。
///
/// **不含「级别分支假」档**：本模块的路径级别严格递减（[`ChainKey`] 由 [`chain_paths`] 的覆盖
/// 关系图产出，每条覆盖边按构造满足 `child.event_level < parent.event_level`；簿内驻留路径是
/// 同一批 key），故边的两端在 [`build_edge`] 的调用点上级别分支恒真。该不可达性以
/// `unreachable!` + 推导链作机器载体（同 [`chain_probe::on_append`] 的禁止边口径），不设一个
/// 从不被写入的档——静默留一个恒 0 的枚举变体会让「该情形从未发生」与「发生了但归错档」不可区分。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PredicateBreachReason {
    /// 子端点区间退化（`start > end`）。
    ChildIntervalDegenerate,
    /// 父端点区间退化（`start > end`）。
    ParentIntervalDegenerate,
    /// 子区间左端越出父区间左端（`child.0 < parent.0`），右端未越出。
    LeftOverhang,
    /// 子区间右端越出父区间右端（`child.1 > parent.1`），左端未越出。
    RightOverhang,
    /// 两端均越出父区间。
    BothEndsOverhang,
}

/// 事实边的**指名见证**（#641 取舍裁定 comment-5121793896 第 4 条，形态承自 B 侧
/// `cand_chain.rs` 的 `fact_edge` 指名档，按本模块的对象形态重述）。
///
/// 回答三件事，缺一则「谓词判不过」这条留痕不可复核：**哪条谓词**（[`Self::predicate`]）、
/// **哪两端点**（[`Self::parent`] / [`Self::child`]，指名到候选身份键，并附判定时读到的两个
/// 区间）、**判不过的直接原因**（[`Self::reason`]）。
///
/// ## S8 口径的照实登记（E2E-L §S8「节点几何不入链载荷」）
///
/// 本结构体**携带端点区间** ⟹ 节点几何在事实边这一档进入了链载荷。后果有界且已封口：谓词
/// 判不过 ⟹ 链当次即转 `Invalidated`（终态）⟹ 该链此后不再有任何 revision，故每条链至多有
/// **一条** revision 携带几何。链段（`predicate_holds == true`）一侧恒为 `None`，S8 的原意
/// （节点区间生长不得刷链修订）由 `revisions_are_append_only_and_node_growth_alone_is_not_a_chain_revision`
/// 继续锁住。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PredicateBreach {
    /// 判定谓词的指名，恒为 [`CHAIN_SEGMENT_PREDICATE`]（唯一判定来源，不重写区间不等式）。
    pub predicate: &'static str,
    /// 上端（级别大）存活端点，与 [`ChainEdge::parent`] 同值（见证自包含，同一构造点写入）。
    pub parent: CandidateKey,
    /// 下端（级别小）存活端点，与 [`ChainEdge::child`] 同值。
    pub child: CandidateKey,
    /// 判定时读到的父端点区间（`source_index` 闭区间）。
    pub parent_interval: (usize, usize),
    /// 判定时读到的子端点区间。
    pub child_interval: (usize, usize),
    pub reason: PredicateBreachReason,
}

/// 两个**存活端点**之间的一条边。
///
/// 谓词判过 ⟹ 链段（[`Self::is_segment`]）；判不过 ⟹ **事实边**，照实留在
/// [`TowerChainCertificate::edges`] 里但不构成链段（携带 [`Self::breach`] 指名见证），
/// 并使整条链转 `Invalidated`（裁定②，成因 [`ChainInvalidationCause::PredicateFailed`]）。
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
    /// 判不过时的指名见证；`predicate_holds == true` ⟹ 恒 `None`。
    ///
    /// 不变量（构造点唯一，见 [`build_edge`]）：`breach.is_some() ⟺ !predicate_holds`。
    pub breach: Option<PredicateBreach>,
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

    /// 第 `len` 个节点为止的真前缀键（`2 <= len < path.len()`，否则 `None`）。
    ///
    /// 纯结构派生，**不保证该 key 曾作为对象在簿中物化过**——`extends` 的解析必须经
    /// [`ChainCertificateBook::resolve_extends`] 查簿命中，本函数只提供候选键。
    /// 单节点前缀不是链（[`Self::new`] 的 ≥2 断言），故 `len < 2` 返回 `None`。
    fn prefix(&self, len: usize) -> Option<ChainKey> {
        (2..self.path.len()).contains(&len).then(|| ChainKey {
            rule_version: self.rule_version,
            path: self.path[..len].to_vec(),
        })
    }
}

/// 一等链证书的一次不可变 revision。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TowerChainCertificate {
    pub key: ChainKey,
    /// E2E-L `extends_lineage_key`：本链所扩展的、**曾在本簿物化过**的最长真前缀路径键。
    ///
    /// 由 [`ChainCertificateBook::resolve_extends`] 查簿得出（不是纯结构派生）——结构上存在的
    /// 真前缀若从未作为链落过簿，本字段为 `None`，不指一条不存在的谱系。
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
    /// 判死成因（与 [`Self::invalidated_at`] 同步一次写入）；非 `Invalidated` 恒 `None`。
    pub invalidation_cause: Option<ChainInvalidationCause>,
    pub revision: u32,
    pub supersedes_revision: Option<u32>,
    pub revision_at: usize,
}

/// [`TowerChainCertificate`] 的业务载荷投影 —— 链「是什么」的**唯一比较口径**。
///
/// 进投影的是链的结构载荷与判定结果；**不进投影**的是生命史记账（三只钟 + revision 计数 +
/// [`ChainInvalidationCause`]，后者由载荷派生，见其文档）。
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
///
/// **不含 `extends`**：谱系承继是**簿的事实**（那条前缀链是否真落过簿），不是单次扫描能从
/// 事件视图读出的量；由 [`ChainCertificateBook::resolve_extends`] 在落簿处解析。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainObservation {
    pub key: ChainKey,
    pub root_level: u32,
    pub leaf_level: u32,
    pub nodes: Vec<ChainNodeTrace>,
    pub edges: Vec<ChainEdge>,
    pub extendable: bool,
    pub status: ChainStatus,
    /// `status == Invalidated` 时的判死成因，否则 `None`。
    pub invalidation_cause: Option<ChainInvalidationCause>,
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
///
/// **deprecated 待退役**（世代宪法 §1 登记，#641 S-6）：零非测试外部调用者——当前只有本模块内
/// 与 `tests.rs` 用。**不删**：N4 的「事件 ↔ BSP 稳定身份边」是候选消费面，退役须另票裁定。
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

    let head_confirmed = nodes[0].state == Some(CandidateState::Confirmed);
    let all_segments = edges.iter().all(ChainEdge::is_segment);
    // 地板条款（#641 comment-5121572134）：「全链段谓词判过」不得在空集上真空成立。
    let has_segment = edges.iter().any(ChainEdge::is_segment);
    // 两支判死的优先序：链头证伪先于谓词判不过。两者可同时成立（链头证伪且某条边判不过），
    // 此时记链头这一支——链头是整条链的存在前提，它一没谓词判定的两端点关系已不成立。
    let (status, invalidation_cause) = if nodes[0].status == ChainNodeStatus::Falsified {
        (
            ChainStatus::Invalidated,
            Some(ChainInvalidationCause::HeadInvalidated),
        )
    } else if !all_segments {
        (
            ChainStatus::Invalidated,
            Some(ChainInvalidationCause::PredicateFailed),
        )
    } else if head_confirmed && !extendable && has_segment {
        (ChainStatus::Closed, None)
    } else {
        (ChainStatus::Open, None)
    };

    #[cfg(test)]
    chain_probe::on_evaluate(&nodes, &edges);
    #[cfg(test)]
    if nodes[0].status == ChainNodeStatus::Alive
        && head_confirmed
        && all_segments
        && !extendable
        && !has_segment
    {
        chain_probe::on_floor_blocked();
    }

    ChainObservation {
        root_level: key.root().level,
        leaf_level: key.leaf().level,
        key,
        nodes,
        edges,
        extendable,
        status,
        invalidation_cause,
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
    let predicate_holds = candidate_is_sub(child, parent);
    ChainEdge {
        parent: parent_key,
        child: child_key,
        kind,
        skipped_levels,
        crossed_nodes: nodes[upper + 1..lower]
            .iter()
            .map(|node| node.key)
            .collect(),
        predicate_holds,
        breach: (!predicate_holds).then(|| PredicateBreach {
            predicate: CHAIN_SEGMENT_PREDICATE,
            parent: parent_key,
            child: child_key,
            parent_interval: parent.interval,
            child_interval: child.interval,
            reason: breach_reason(child, parent),
        }),
    }
}

/// 谓词判不过时的首个失败合取项（[`PredicateBreachReason`]）。
///
/// 只在 `candidate_is_sub(child, parent) == false` 时被调用（唯一调用点 [`build_edge`] 的
/// `breach` 构造），故本函数必然能定位到一个失败项；分档只读两端点的 `event_level` 与
/// `interval`，不复算任何不等式的真值。
fn breach_reason(child: &CandidateEvent, parent: &CandidateEvent) -> PredicateBreachReason {
    if child.event_level >= parent.event_level {
        // ★不可达（禁止边的机器载体，非「应该不会发生」的注释声明）。推导链：
        // 1. 本函数的唯一调用点是 `build_edge`，其两端点取自 `ChainKey.path` 的两个位置
        //    （upper < lower，路径序 root→leaf）；
        // 2. 路径来自 `chain_paths` 的覆盖关系图，每条覆盖边按构造满足
        //    `candidate_is_sub(child, parent)` ⟹ `child.event_level < parent.event_level`
        //    ⟹ 路径的 `key.level` 沿 root→leaf 严格递减；簿内驻留路径是同一批 key（`advance`
        //    只把已入簿的 `key.path` 放回评估，不改路径）；
        // 3. `CandidateEvent::event_level` 恒等于 `key.level`（`cand_event` 的观察落事件处），
        //    故 upper 位的 `event_level` 严格大于 lower 位的。
        // 静默把它归进区间档会把「级别拒」误报成「区间越界」，两件事不可混同。
        unreachable!(
            "链路径级别沿 root→leaf 严格递减：跨级分支不可能在 build_edge 的调用点判假\
             （见 breach_reason 头部推导链）；实得 child_level={} parent_level={}",
            child.event_level, parent.event_level
        );
    }
    if interval_is_degenerate(child.interval) {
        return PredicateBreachReason::ChildIntervalDegenerate;
    }
    if interval_is_degenerate(parent.interval) {
        return PredicateBreachReason::ParentIntervalDegenerate;
    }
    match (
        child.interval.0 < parent.interval.0,
        child.interval.1 > parent.interval.1,
    ) {
        (true, true) => PredicateBreachReason::BothEndsOverhang,
        (true, false) => PredicateBreachReason::LeftOverhang,
        (false, true) => PredicateBreachReason::RightOverhang,
        // ★不可达：两端都不越出 + 两区间均非退化 ⟹ `interval_is_sub` 判真 ⟹ 合上已成立的
        // 跨级分支即 `candidate_is_sub` 为真，与本函数的调用前提（谓词判不过）矛盾。
        (false, false) => unreachable!(
            "谓词判不过但两端点区间既不退化也不越界，与 candidate_is_sub 的构成矛盾：\
             child={:?} parent={:?}",
            child.interval, parent.interval
        ),
    }
}

// ── append-only 修订簿 ──────────────────────────────────────────────────────────────────

/// 链簿的全量只读读数（诊断与报告用；**不判真值**）。
///
/// 形态承自 B 侧 `cand_chain_book.rs` 的 `ChainBookSummary`（#641 取舍裁定 comment-5121793896
/// 第 4 条），按本模块的对象形态重述：B 汇总的是 `ChainLineageStreams`、按四档 `EdgeVerdict`
/// 分桶；本结构体汇总的是 [`ChainCertificateBook`] 的 head 集合、按本模块的边形态
/// （[`ChainEdgeKind`]）与谓词两侧（链段 / 事实边）分桶。
///
/// **入库理由**：读数逻辑此前散在两个诊断 bin 里各写一遍（`issue550_event_battery` 的
/// `print_chain_summary` 与 p123 的 `chain_dump_line`），既重复又不可测。放进库内 ⟹ 单测可锁、
/// 报告数与测试数不可能对不上。
///
/// **同源范围（照实，#641 S-2 收窄）**：只有 `issue550_event_battery::settle_chain_readout`
/// （原 `print_chain_summary`，#676-2 拆分后改名）真迁到了本结构体（`summarize()` + `digest()`）。
/// p123 的 `chain_dump_line` 是**逐证书**
/// （per-certificate）的行格式化，粒度与本结构体的**全簿汇总**不同，**未迁**——它仍自带各档
/// 节点/边的计数。故「同源」当前只成立于全簿 `summarize` / `digest` 一侧，不含 p123 的逐行 dump。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChainBookSummary {
    /// 簿内全部 revision 数（append-only 的总行数）。
    pub revisions: usize,
    /// 链身份数（每 key 最新 revision 一条）。
    pub chains: usize,
    pub open: usize,
    pub closed: usize,
    pub invalidated: usize,
    /// 判死成因分档（合计恒等于 [`Self::invalidated`]）。
    pub invalidated_head: usize,
    pub invalidated_predicate: usize,
    /// `extends` 非空（谱系承继命中）的链数。
    pub extends_some: usize,
    /// ★地板条款监视格（#641 comment-5121572134）：`Closed` 且零链段的链数。
    ///
    /// 地板合取上线后**恒 0**；非零 = 地板条款被绕过。单列而非只靠单测，是为了让真实数据面上
    /// 的违规可见（对比评审在 B 侧实测该格 = 其 Closed 的 85%）。
    pub closed_with_zero_segments: usize,
    pub edges: usize,
    pub adjacent_edges: usize,
    pub skip_edges: usize,
    /// 链段数（谓词判过的边）。
    pub segments: usize,
    /// 事实边数（谓词判不过的边，携带 [`PredicateBreach`]）。
    pub fact_edges: usize,
    /// 被边跨过的路径节点数（证伪/查无夹在两存活端点之间）。
    pub crossed_nodes: usize,
    pub nodes_alive: usize,
    pub nodes_falsified: usize,
    pub nodes_absent: usize,
    /// 被跳过级别的三格分解（口径见 [`SkippedLevel`]）：缺 / 断-在父外 / 断-在父内接不上。
    pub skipped_level_missing: usize,
    pub skipped_level_broken_outside: usize,
    pub skipped_level_broken_inside: usize,
    /// 路径节点数 → 链数。
    pub by_path_len: BTreeMap<usize, usize>,
    /// 链头级别 → 链数。
    pub by_root_level: BTreeMap<u32, usize>,
}

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

    /// 按 [`ChainKey`] 取该链身份的最新 revision（无此 key 则 `None`）。
    ///
    /// **pub 导出面待收窄**（#667 LOW-1 订正 #641 S-6 登记）：零跨模块非测试调用者，可降
    /// `pub(crate)`；但模块内生产路径在用（`apply` 取上一 revision），按世代宪法 §1
    /// 名分 = **现役**，不是 deprecated。收窄与否则另票裁定。
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

    /// E2E-L `extends_lineage_key`：**簿内**最长真前缀（`rule_version` 相同）。
    ///
    /// ## 为什么必须查簿（#641 取舍裁定 comment-5121793896 第 3 条）
    ///
    /// 旧实装取纯结构派生的最长真前缀（去掉 leaf），不问它是否真作为链落过簿。BTC 100k 实测
    /// `extends` 非空 12 条、其中**物化命中 0 条**——12 条指针全部指向从未存在过的链。成因是
    /// 构造口径的直接后果：极大路径的真前缀本身不是极大路径（它的 leaf 有存活子），除非它在更
    /// 早的 `as_of` 恰好曾是极大路径并落过簿。字段语义写的是「本链所扩展的谱系」，指一条不存在
    /// 的谱系就是把「结构派生」冒充成「谱系事实」，对 N7 是误导。
    ///
    /// ## 口径
    ///
    /// 从最长（`path.len()-1`）向下逐个试到 2 节点，取**第一个在簿内有 head 的**前缀键。
    /// **不按状态过滤**：`Closed` / `Invalidated` 的前缀同样算数——「那条链曾经存在」是谱系事实，
    /// 与它后来死活无关（既有单测 `downward_extension_creates_new_key_and_leaves_closed_chain_untouched`
    /// 锁的正是「向下扩展指向一条已 `Closed` 的前缀链」）。
    ///
    /// 本解析在**落簿处**做而非在 `evaluate` 里：`evaluate` 是纯函数（同一事件视图上重复调用结果
    /// 相同），承继关系依赖簿的历史，把它塞进 `evaluate` 会让「纯函数」这个声明变假。
    fn resolve_extends(&self, key: &ChainKey) -> Option<ChainKey> {
        let resolved = (2..key.path.len())
            .rev()
            .filter_map(|len| key.prefix(len))
            .find(|prefix| self.latest.contains_key(prefix));
        #[cfg(test)]
        chain_probe::on_extends(key.path.len() >= 3, resolved.is_some());
        resolved
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
        let extends = self.resolve_extends(&observation.key);
        let next = make_revision(prior.as_ref(), observation, extends, as_of);
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

    /// 全量只读读数（每 key 最新 revision 分桶）。不判真值、不改簿。
    pub fn summarize(&self) -> ChainBookSummary {
        let mut summary = ChainBookSummary {
            revisions: self.certificates.len(),
            chains: self.latest.len(),
            ..ChainBookSummary::default()
        };
        for certificate in self.heads() {
            match certificate.status {
                ChainStatus::Open => summary.open += 1,
                ChainStatus::Closed => summary.closed += 1,
                ChainStatus::Invalidated => summary.invalidated += 1,
            }
            match certificate.invalidation_cause {
                Some(ChainInvalidationCause::HeadInvalidated) => summary.invalidated_head += 1,
                Some(ChainInvalidationCause::PredicateFailed) => summary.invalidated_predicate += 1,
                None => {}
            }
            if certificate.extends.is_some() {
                summary.extends_some += 1;
            }
            if certificate.status == ChainStatus::Closed && certificate.segment_count() == 0 {
                summary.closed_with_zero_segments += 1;
            }
            *summary
                .by_path_len
                .entry(certificate.key.path.len())
                .or_default() += 1;
            *summary
                .by_root_level
                .entry(certificate.root_level)
                .or_default() += 1;
            for node in &certificate.nodes {
                match node.status {
                    ChainNodeStatus::Alive => summary.nodes_alive += 1,
                    ChainNodeStatus::Falsified => summary.nodes_falsified += 1,
                    ChainNodeStatus::Absent => summary.nodes_absent += 1,
                }
            }
            for edge in &certificate.edges {
                summary.edges += 1;
                match edge.kind {
                    ChainEdgeKind::Adjacent => summary.adjacent_edges += 1,
                    ChainEdgeKind::Skip => summary.skip_edges += 1,
                }
                if edge.is_segment() {
                    summary.segments += 1;
                } else {
                    summary.fact_edges += 1;
                }
                summary.crossed_nodes += edge.crossed_nodes.len();
                for level in &edge.skipped_levels {
                    if level.alive_at_level == 0 {
                        summary.skipped_level_missing += 1;
                    } else if level.inside_parent == 0 {
                        summary.skipped_level_broken_outside += 1;
                    } else {
                        summary.skipped_level_broken_inside += 1;
                    }
                }
            }
        }
        summary
    }

    /// 簿的确定性摘要（FNV-1a over `Debug`，覆盖**全部** revision 而非仅 head）。
    ///
    /// golden 锁与双路径比对共用同一口径 ⟹ 主缝的 golden 与 bin 的读数不可能各算各的。
    pub fn digest(&self) -> u64 {
        format!("{:?}", self.certificates)
            .bytes()
            .fold(FNV_OFFSET_BASIS, |hash, byte| {
                (hash ^ byte as u64).wrapping_mul(FNV_PRIME)
            })
    }
}

fn make_revision(
    prior: Option<&TowerChainCertificate>,
    observation: ChainObservation,
    extends: Option<ChainKey>,
    as_of: usize,
) -> TowerChainCertificate {
    // observed_at / closed_at / invalidated_at / invalidation_cause 均为「一次写入不后移」：
    // 已在案的值优先。
    let observed_at = prior.map_or(as_of, |certificate| certificate.observed_at);
    let closed_at = prior
        .and_then(|certificate| certificate.closed_at)
        .or((observation.status == ChainStatus::Closed).then_some(as_of));
    let invalidated_at = prior
        .and_then(|certificate| certificate.invalidated_at)
        .or((observation.status == ChainStatus::Invalidated).then_some(as_of));
    let invalidation_cause = prior
        .and_then(|certificate| certificate.invalidation_cause)
        .or(observation.invalidation_cause);
    TowerChainCertificate {
        key: observation.key,
        extends,
        root_level: observation.root_level,
        leaf_level: observation.leaf_level,
        nodes: observation.nodes,
        edges: observation.edges,
        extendable: observation.extendable,
        status: observation.status,
        observed_at,
        closed_at,
        invalidated_at,
        invalidation_cause,
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
        /// ★地板条款（#641 comment-5121572134）拦下的 `Closed`：链头存活且 `Confirmed`、不可再
        /// 扩展、**全边皆链段**，但**链段集合为空** ⟹ 判 `Open` 而非 `Closed`。
        ///
        /// 触发面照实（#667 LOW-3 订正）：`!has_segment && all_segments` 等价于 **`edges` 全空**
        /// （有事实边时「链段集合为空」也成立，但那种链走 `PredicateFailed` 判死支，不记本格）；
        /// 加「链头存活」后精确对应**链头独活**这一情形。
        ///
        /// 非零 = 地板合取真被走到（不是靠注释声明的条款）。BTC 三窗实测该情形零触发，故这条
        /// 只由 `chain_cert::tests` 的语义锁覆盖，见报告 §五。
        pub floor_blocked: u64,
        /// `extends` 解析：结构上存在真前缀（路径 ≥3 节点）且**簿内命中**。
        pub extends_resolved: u64,
        /// `extends` 解析：结构上存在真前缀但**簿内无一命中** ⟹ 不写 `extends`。
        ///
        /// 旧实装在这一格上照写不误（BTC 100k 实测 12 条全部落这一格），是「结构派生冒充谱系
        /// 事实」的机器计数。
        pub extends_not_materialized: u64,
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

    pub fn on_floor_blocked() {
        PROBE.with(|probe| probe.borrow_mut().floor_blocked += 1);
    }

    /// `structural_prefix_exists` = 路径 ≥3 节点（结构上有真前缀）；`resolved` = 簿内命中。
    pub fn on_extends(structural_prefix_exists: bool, resolved: bool) {
        if !structural_prefix_exists {
            return;
        }
        PROBE.with(|probe| {
            let mut probe = probe.borrow_mut();
            if resolved {
                probe.extends_resolved += 1;
            } else {
                probe.extends_not_materialized += 1;
            }
        });
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
