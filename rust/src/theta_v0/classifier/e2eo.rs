//! #1088（SPEC #1085 S4）E2E-O 簿装配模块：B 增量化去节拍 + 安全三件套。
//!
//! ## Seam（一行）
//!
//! ```text
//! classify_incremental（逐 bar 因果通道）
//!   → E2eOAssembly::advance（AliveIndex 增量 + 路径 DAG 增量扩展 → ChainCertificateBook /
//!       BspBridgeBook / event_to_bsp 投影，逐 bar O(Δ)、总 O(n)；closed_at 真 bar 位）
//!   → E2eOAssembly::consume（纯 consume_at + prior 幂等累积，跨链只创建一次受管 BSP）
//! ```
//!
//! ## 目标形态（#1088）
//!
//! 1. 模块进 `classifier/`（无条件编译）；对外两入口（推进 [`E2eOAssembly::advance`] / 消费
//!    [`E2eOAssembly::consume`]）；模块内投影 + 幂等、[`consume_at`] 保持纯。
//! 2. 推进 = B 增量化去节拍：`AliveIndex` 增量（[`super::chain_cert::AliveIndex`]，每 bar 只折
//!    Δ 条新事件、总 O(n)）+ 路径 DAG 增量扩展（[`super::chain_cert::PathDag`]，候选出生/生长/
//!    失效时局部改覆盖边，不每 bar 全算 Hasse 表）；`closed_at` 真 bar 位；`advance_every`
//!    参数退役（推进接口按 bar，不再暴露节拍）。
//! 3. 安全三件套（缺一不放行）：`incremental_parity` 因果簿 ≡ 全量重放对拍（
//!    `tests::incremental_parity::e2eo_assembly_incremental_equals_full_replay`）/
//!    CandidateEventBook append-only、前缀稳定（本模块只读事件流、只做 last-wins 折叠，不改写
//!    流）/ 前沿回缩 cascade 配回滚（[`Self::reset`] 退化为全量重建，TowerCache cascade、
//!    retrace_ledger resume 同款）。
//! 4. 测试随模块迁（幂等零增量 / 只收 Closed / 投影 / 跨链 prior 累积）；p985 缩为读数 bin。
//! 5. WireV1 消费点：WireV1 = 薄调用方经本模块两入口取数；BspKey = episode 粒度（#980 裁定 A）；
//!    消费侧不自装配（投影与 prior 都在本模块内）。
//!
//! ## 增量化去节拍的机制（对应 #1065 裁定 B）
//!
//! 原 `ChainCertificateBook::advance(&streams, as_of)` 每 bar 从全流重折 `AliveIndex`（O(总事件)
//! /bar ⟹ O(n²)）并全算 Hasse 覆盖表。本模块改为：
//! - **`AliveIndex` 增量**：按每级游标 `seen_len` 只折本 bar 新追加的事件（O(Δ)/bar，总 O(n)），
//!   `apply` = `build` 的单步折叠（`latest` last-wins + `alive_by_level` 增删）。
//! - **路径 DAG 增量扩展**：候选出生 / 几何生长 / 失效时，[`super::chain_cert::PathDag`] 局部
//!   改覆盖边（标准动态 Hasse 图插入/删除），极大路径从增量子表 DFS 重导出；Delta = 0 的 bar
//!   整条推进零开销（索引/DAG/链簿都不动）。
//!
//! 对拍锁保证：增量侧与全量自 ∅ 重放侧共享同一 [`super::chain_cert::ChainCertificateBook::advance_indexed`]
//! 落簿口径 ⟹ bit-exact（见模块测试）。
//!
//! ## 前沿回缩与回滚（安全三件套之三）
//!
//! 候选事件流是 append-only（`CandidateEventBook` 前缀稳定，安全三件套之二）；正常因果流中
//! `streams` 只增不减（TowerCache `clear()` 保留候选簿）。若调用方以更短的流或回退的 `as_of`
//! 驱动本模块（前沿回缩 / 换血缘），[`Self::advance`] 检测到后走 [`Self::reset`]：`AliveIndex` +
//! 路径 DAG 退化为全量重建（bit-exact 退化，同 TowerCache cascade），append-only 簿不截断——
//! 链侧按 `ChainNodeStatus::Absent` 语义继续照实评估（`chain_cert` 声明支持的驱动语义）。

use std::collections::HashMap;

use super::bsp_bridge::{BspBridgeBook, BspStructuralKey};
use super::cand_event::{CandidateEvent, CandidateKey, CandidateState, CandidateStreams};
use super::chain_cert::{
    AliveIndex, ChainCertificateBook, ChainStatus, PathDag, TowerChainCertificate,
};
use super::consume_at::{
    consume_at, BspLink, ConsumeError, LinkKey, ManagedBsp, ManagedBspCreation, ManagedBspPolicy,
};
use super::Classification;

/// E2E-O 装配：持有链簿 / 桥接簿 / consume prior 态 + 推进节拍 + event_to_bsp 投影 + 幂等。
///
/// 对外两个入口：[`Self::advance`]（推进）与 [`Self::consume`]（消费）。p985 与未来 WireV1 是
/// 本模块的薄调用方（只经两入口取数，不自装配投影与 prior）。
#[derive(Debug, Default)]
pub struct E2eOAssembly {
    /// 增量维护的存活候选索引（`chain_cert::AliveIndex` 的持有版）。
    index: AliveIndex,
    /// 覆盖关系图的增量维护（极大路径随候选出生/生长/失效局部扩展）。
    dag: PathDag,
    chain_book: ChainCertificateBook,
    bridge_book: BspBridgeBook,
    /// `BridgeKey{event, bsp}` → 候选事件 → BSP 的有损 last-wins 投影（#982 A：consume_at 只读）。
    event_to_bsp: HashMap<CandidateKey, BspStructuralKey>,
    /// consume 的跨链 prior（受管 BSP 只创建一次、多条链接组内写入，#980 残余规则）。
    prior_bsp: HashMap<BspStructuralKey, ManagedBsp>,
    prior_links: HashMap<LinkKey, BspLink>,
    /// 每级已折事件数游标（append-only 流前缀稳定 ⟹ 只折新尾部）。
    seen_len: Vec<usize>,
    last_as_of: Option<usize>,
}

impl E2eOAssembly {
    pub fn new() -> Self {
        Self::default()
    }

    /// 推进一个 bar：候选事件流尾部新事件 → 增量索引 / 增量 DAG → 链簿 + 桥接簿 + 投影。
    ///
    /// `as_of` 为本 bar 末的 bar 位（`closed_at` 落真 bar 位的前提）；`classification` 为本 bar
    /// `classify_incremental` 的 `Classification`；`streams` 为其 `candidate_streams`（同一
    /// `TowerCache` 因果簿的 append-only 终态投影）。
    ///
    /// ## 幂等
    ///
    /// 同一 `(classification, streams, as_of)` 重跑：无新事件 ⟹ 零输出（索引/DAG/链簿零变动）；
    /// 有同一批事件但投影未变 ⟹ 链簿/桥接簿幂等跳过（append-only 簿的投影等价锁）。
    pub fn advance(
        &mut self,
        classification: &Classification,
        streams: &CandidateStreams,
        as_of: usize,
    ) -> Vec<TowerChainCertificate> {
        // 前沿回缩 / as_of 回退 ⟹ 退化全量重建（安全三件套之三：cascade 配回滚）。
        if self.retracted(streams, as_of) {
            self.reset(streams);
        }

        // 每级只折新尾部（append-only 前缀稳定 ⟹ 已折前缀无需重看）。
        let mut index_changed = false;
        for level in 0..streams.len() {
            let stream = &streams[level];
            let seen = self.seen_len.get(level).copied().unwrap_or(0);
            for event in &stream[seen..] {
                index_changed = true;
                self.apply_candidate_delta(event);
            }
        }
        self.seen_len = streams.iter().map(|stream| stream.len()).collect();

        // 链簿：仅当候选视图真变时才推进（视图不变 ⟹ 重评必幂等，跳过零开销）。
        let chain_delta = if index_changed {
            self.chain_book
                .advance_indexed(&self.index, self.dag.paths(), as_of)
        } else {
            Vec::new()
        };

        // 桥接簿：每 bar 推进（BSP 点可随 classification 变而候选不变）；投影随 Delta 增量维护。
        let bridge_delta =
            self.bridge_book
                .advance_indexed(classification, self.index.latest(), as_of);
        for edge in bridge_delta {
            self.event_to_bsp
                .insert(edge.key.event, edge.key.bsp.clone());
        }

        self.last_as_of = Some(as_of);
        chain_delta
    }

    /// 消费：对簿内全部 `Closed` 链（按 `closed_at` 升序、同 `as_of` 按 key 升序去随机化）调纯
    /// [`consume_at`]，跨链 prior 累积（受管 BSP 只创建一次、组内多链接）。
    ///
    /// 返回本次消费产出的（新增受管 BSP 创建，新增 BspLink）。任一 [`ConsumeError`] 直接上抛
    /// （fail-closed；同 `as_of` 升序 + prior 单调保证 `LateAuthorization`/`InconsistentState`
    /// 恒不可达，实际只可能由非法 `policy` 触发 `InvalidPolicy`）。
    pub fn consume(
        &mut self,
        policy: &ManagedBspPolicy,
    ) -> Result<(Vec<ManagedBspCreation>, Vec<BspLink>), ConsumeError> {
        let mut closed: Vec<&TowerChainCertificate> = self
            .chain_book
            .heads()
            .into_iter()
            .filter(|certificate| {
                certificate.status == ChainStatus::Closed && certificate.closed_at.is_some()
            })
            .collect();
        closed.sort_by_key(|certificate| (certificate.closed_at, certificate.key.clone()));

        let mut creations_total = Vec::new();
        let mut links_total = Vec::new();
        for certificate in closed {
            let as_of = certificate
                .closed_at
                .expect("Closed 过滤已保证 closed_at 非空");
            let (creations, links) = consume_at(
                as_of,
                &self.prior_bsp,
                &self.prior_links,
                certificate,
                policy,
                &self.event_to_bsp,
            )?;
            for creation in &creations {
                self.prior_bsp
                    .insert(creation.bsp.key.clone(), creation.bsp.clone());
            }
            for link in &links {
                self.prior_links.insert(link.key.clone(), link.clone());
            }
            creations_total.extend(creations);
            links_total.extend(links);
        }
        Ok((creations_total, links_total))
    }

    /// 链簿只读读数（WireV1 薄调用方取数口；不改簿）。
    pub fn chain_book(&self) -> &ChainCertificateBook {
        &self.chain_book
    }

    /// 桥接簿只读读数（WireV1 薄调用方取数口；不改簿）。
    pub fn bridge_book(&self) -> &BspBridgeBook {
        &self.bridge_book
    }

    /// 当前 event_to_bsp 投影（`BridgeKey{event, bsp}` 的有损 last-wins 折叠）。
    pub fn event_to_bsp(&self) -> &HashMap<CandidateKey, BspStructuralKey> {
        &self.event_to_bsp
    }

    /// 当前 consume 的跨链 prior（受管 BSP / 链接；幂等累积的读数口）。
    pub fn prior_bsp(&self) -> &HashMap<BspStructuralKey, ManagedBsp> {
        &self.prior_bsp
    }

    pub fn prior_links(&self) -> &HashMap<LinkKey, BspLink> {
        &self.prior_links
    }

    /// 前沿回缩检测：`as_of` 回退，或任一级流长度回缩（append-only 前缀稳定被破）。
    fn retracted(&self, streams: &CandidateStreams, as_of: usize) -> bool {
        if self.last_as_of.is_some_and(|last| as_of < last) {
            return true;
        }
        if streams.len() < self.seen_len.len() {
            return true;
        }
        self.seen_len
            .iter()
            .enumerate()
            .any(|(level, seen)| streams[level].len() < *seen)
    }

    /// 前沿回缩回滚：索引 + DAG 退化为全量重建（append-only 簿不截断——链侧按 Absent 语义
    /// 继续评估，`chain_cert` 声明支持的驱动）。游标归位到当前流全长。
    fn reset(&mut self, streams: &CandidateStreams) {
        self.index = AliveIndex::build(streams);
        self.dag.recompute(&self.index);
        self.seen_len = streams.iter().map(|stream| stream.len()).collect();
    }

    /// 单条事件 Delta 落索引 + DAG（出生 / 失效 / 几何生长三分，见 [`super::chain_cert::PathDag`]
    /// 类型文档「三操作」）。
    fn apply_candidate_delta(&mut self, event: &CandidateEvent) {
        let prior = self.index.latest().get(&event.key).cloned();
        if event.state == CandidateState::Invalidated {
            // 失效：先落索引（摘出存活集）再摘 DAG（cover_direct 的居中检查须不见 k）。
            if prior
                .as_ref()
                .is_some_and(|p| p.state != CandidateState::Invalidated)
            {
                self.index.apply(event);
                self.dag.apply_invalidation(&self.index, event.key);
            } else {
                // 终态事件重复到达（幂等重放）只刷 latest，DAG 无该存活节点可摘。
                self.index.apply(event);
            }
            return;
        }
        match prior {
            None
            | Some(CandidateEvent {
                state: CandidateState::Invalidated,
                ..
            }) => {
                // 出生（复活不可达——CandidateEventBook 终态挡回，此处仅作防御归入出生）。
                self.index.apply(event);
                self.dag.apply_birth(&self.index, event.key);
            }
            Some(prev) => {
                if prev.interval != event.interval {
                    // 几何生长 = 先按旧几何摘 DAG（k 已摘出存活集），再落新几何重插入。
                    self.index.remove_alive(event.key);
                    self.dag.apply_invalidation(&self.index, event.key);
                    self.index.apply(event);
                    self.dag.apply_birth(&self.index, event.key);
                } else {
                    // 载荷修订（状态/谓词变而几何不变）：DAG 不动，只刷索引供链侧重评读新态。
                    self.index.apply(event);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! #1088 E2E-O 装配模块语义锁。
    //!
    //! N1 事件侧夹具经 `CandidateEventBook::advance` 产出真实事件（同 `chain_cert::tests` /
    //! `bsp_bridge::tests` 先例）；BSP 侧（`BspPoint`/`Classification`）是纯数据记录，手搓字面量
    //! 只测本模块**自己的**装配/投影/幂等/prior 累积逻辑，不冒充判据测试。

    use std::rc::Rc;

    use super::super::super::types::{BspBits, Center, Side};
    use super::super::bsp::{BspPoint, OwnerRef};
    use super::super::cand_event::{
        CandidateEventBook, CandidateKey, CandidateKind, CandidateObservation, CandidateState,
        CandidateStreams, ObservedState, ParentFingerprint, StructuralPredicates,
        CANDIDATE_RULE_VERSION,
    };
    use super::super::chain_cert::ChainStatus;
    use super::super::consume_at::{ManagedBspPolicy, PolicyRule};
    use super::super::{Classification, LevelState};
    use super::*;

    fn key(level: u32, c_start: usize) -> CandidateKey {
        CandidateKey {
            rule_version: CANDIDATE_RULE_VERSION,
            level,
            kind: CandidateKind::Trend,
            side: Side::Long,
            previous_center_start: Some(10),
            parent: ParentFingerprint {
                center_start: 20,
                zd: 100,
                zg: 110,
            },
            seg_a: (11, 19),
            c_start,
        }
    }

    /// Trend 域活候选观察（`Provisional`）。
    fn obs(level: u32, c_start: usize, interval: (usize, usize)) -> CandidateObservation {
        CandidateObservation {
            key: key(level, c_start),
            kind: CandidateKind::Trend,
            center_ids: Some((10, 20)),
            candidate_group_id: 1,
            pair_id: 2,
            structural_predicates: StructuralPredicates {
                direction: true,
                comparable: true,
                extreme: true,
            },
            extreme_proof: (11, 19),
            third_class_proof: None,
            interval,
            state: ObservedState::Provisional,
            first_provable_at: Some(interval.1),
            confirmed_at: None,
        }
    }

    /// Pan 域确认候选观察（`Confirmed`；键按 Pan 域口径不带前中枢）。
    fn confirmed(level: u32, c_start: usize, interval: (usize, usize)) -> CandidateObservation {
        let mut observation = obs(level, c_start, interval);
        observation.key.kind = CandidateKind::Pan;
        observation.key.previous_center_start = None;
        observation.kind = CandidateKind::Pan;
        observation.center_ids = None;
        observation.state = ObservedState::Confirmed;
        observation
    }

    fn streams_of(observations: &[CandidateObservation], as_of: usize) -> CandidateStreams {
        let mut book = CandidateEventBook::default();
        book.advance(observations, as_of);
        book.streams()
    }

    fn center_at(start_index: usize) -> Center {
        Center {
            zd: 100,
            zg: 110,
            dd: 100,
            gg: 110,
            start_index,
            end_index: start_index + 3,
        }
    }

    fn level_with(points: Vec<BspPoint>) -> LevelState {
        LevelState {
            moves: Vec::new(),
            centers: Rc::new(Vec::new()),
            cp_ownership: Rc::new(Vec::new()),
            bsp: Rc::new(points),
            pan_div: Rc::new(Vec::new()),
            first_class_grades: Rc::new(Vec::new()),
            level_projection: None,
        }
    }

    fn buy1_point(source_index: usize, center_start: usize) -> BspPoint {
        BspPoint {
            source_index,
            bits: BspBits {
                buy1: true,
                ..Default::default()
            },
            pivot_low: 0,
            pivot_high: 0,
            center: Some(OwnerRef::Center(center_at(center_start))),
            struct_break_dir: Some(Side::Long),
            force: None,
            retrace_breaks_type1: None,
        }
    }

    fn single_rule_policy() -> ManagedBspPolicy {
        ManagedBspPolicy {
            rules: vec![PolicyRule {
                rule_id: 1,
                policy_id: 1,
                slot: 0,
            }],
        }
    }

    /// ★幂等零增量：同一 `(classification, streams, as_of)` 重跑，链簿零追加。
    #[test]
    fn advance_same_as_of_is_idempotent_zero_delta() {
        let mut assembly = E2eOAssembly::new();
        let streams = streams_of(&[confirmed(1, 0, (0, 100)), obs(0, 20, (20, 40))], 100);
        let classification = Classification::default();

        let first = assembly.advance(&classification, &streams, 100);
        assert!(!first.is_empty(), "首次推进须产链（非真空锁）");
        let second = assembly.advance(&classification, &streams, 100);
        assert!(second.is_empty(), "同 as_of 重跑零 Delta（幂等）");
    }

    /// ★只收 Closed + 跨链 prior 累积：两条 Closed 链共享同一 Trend 节点（L1）⟹ 同一 BSP 只创建
    /// 一次、两条链接组内写入（#980 残余规则：跨多条链只创建一次、多条链接写 BspLink 组内）。
    #[test]
    fn consume_only_closed_and_cross_chain_prior_accumulates() {
        let mut assembly = E2eOAssembly::new();
        // 两条极大路径：L2 → L1 → L0a 与 L2 → L1 → L0b（L0a/L0b 互不包含 ⟹ 各成一支）。
        let streams = streams_of(
            &[
                confirmed(2, 0, (0, 100)),
                obs(1, 10, (10, 60)),
                obs(0, 20, (20, 30)),
                obs(0, 40, (40, 50)),
            ],
            100,
        );
        // 一个 buy1 点落在 L1 的 episode 区间 [10,60] 内 ⟹ 桥接边 L1 → BspStructuralKey。
        let classification = Classification {
            levels: vec![level_with(Vec::new()), level_with(vec![buy1_point(50, 20)])],
        };
        assembly.advance(&classification, &streams, 100);

        // 两条链都 Closed（链头 Confirmed + 不可扩展 + 有链段）。
        let heads = assembly.chain_book().heads();
        let closed: Vec<_> = heads
            .iter()
            .filter(|certificate| certificate.status == ChainStatus::Closed)
            .collect();
        assert_eq!(closed.len(), 2, "两条极大路径都应收口为 Closed");

        // 投影：L1 命中桥接边（event_to_bsp 只含 L1）。
        let event_to_bsp = assembly.event_to_bsp();
        assert_eq!(event_to_bsp.len(), 1, "仅 L1 节点命中断接边");
        assert!(event_to_bsp.contains_key(&key(1, 10)));

        let (creations, links) = assembly
            .consume(&single_rule_policy())
            .expect("Closed + 合法 policy + 空 prior 应产出");
        assert_eq!(creations.len(), 1, "同一 BSP 跨两条链只创建一次");
        assert_eq!(links.len(), 2, "同一 BSP 被两条链授权 ⟹ 两条链接");

        // 幂等：跨链 prior 已累积，再次 consume 零增量。
        let (creations_again, links_again) = assembly
            .consume(&single_rule_policy())
            .expect("prior 未超前，应产出");
        assert!(creations_again.is_empty(), "prior 已有 ⟹ 零创建");
        assert!(links_again.is_empty(), "prior 已有 ⟹ 零链接");
    }

    /// ★投影：event_to_bsp 与桥接簿的 `BridgeKey{event, bsp}` last-wins 折叠一致（#982 A：调用方
    /// 预解析、consume_at 只读不查簿；本模块内部承担该投影）。
    #[test]
    fn projection_event_to_bsp_reflects_bridge_edges() {
        let mut assembly = E2eOAssembly::new();
        let streams = streams_of(&[obs(0, 25, (25, 40))], 40);
        let classification = Classification {
            levels: vec![level_with(vec![buy1_point(40, 20)])],
        };
        assembly.advance(&classification, &streams, 40);

        let event_to_bsp = assembly.event_to_bsp();
        assert_eq!(event_to_bsp.len(), 1);
        let (event, bsp) = event_to_bsp.iter().next().expect("单元素");
        assert_eq!(*event, key(0, 25));
        assert_eq!(bsp.level, 0);
        assert_eq!(bsp.anchor, vec![(11, 19), (25, 25)]);
        // 桥接簿 head 与投影同源（last-wins 折叠后 event 集合一致）。
        let bridge_events: std::collections::BTreeSet<_> = assembly
            .bridge_book()
            .heads()
            .iter()
            .map(|edge| edge.key.event)
            .collect();
        assert_eq!(
            bridge_events,
            std::collections::BTreeSet::from([key(0, 25)])
        );
    }

    /// ★幂等（链簿层）：同 as_of 重推进，链簿 `certificates` 数量不增（append-only 幂等跳过）。
    #[test]
    fn chain_book_append_only_and_rerun_does_not_append() {
        let mut assembly = E2eOAssembly::new();
        let streams = streams_of(
            &[
                confirmed(2, 0, (0, 100)),
                obs(1, 10, (10, 60)),
                obs(0, 20, (20, 40)),
            ],
            100,
        );
        let classification = Classification::default();
        assembly.advance(&classification, &streams, 100);
        let revisions_after_first = assembly.chain_book().certificates().len();
        assert!(revisions_after_first > 0);
        assembly.advance(&classification, &streams, 100);
        assert_eq!(
            assembly.chain_book().certificates().len(),
            revisions_after_first,
            "同 as_of 重跑不追加 revision"
        );
    }
}
