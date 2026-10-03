//! #1394 采纳 C §5.3/5.4：S3 同核增量生成器（`s-session/3` 的**生成态**）。
//!
//! 规范正本：`.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md`
//! 第 5.3（身份配方与全文绑定）与 5.4（S 发布清单与无循环派生）节；采纳状态以同目录
//! `ADOPTION.json` 与 `SPEC-ADDENDUM.md` 为准。逻辑 oracle 是既有 v2 生产入口
//! `s_structure_session.rs` 的 `advance_core` **生成段**（`build_object_and_witnesses` +
//! `tb02a_facts::build` + `tb02b::build`）：同一批逻辑字段、同一序，但走「同核共享证据直接
//! 成值边」，不是「先造完整旧 `Value`/JSON 再拆图」。
//!
//! ## 位置与职责边界
//!
//! - **只做生成态**：吃调用方同一读集里的真实 `all_events`（全修订史，`read_raw_events`
//!   seq 序）与 `effective`（当前生效账，coord 序），产出原 sealed 批次 `generated_state`
//!   的全部逻辑字段（14 项 + OHLC 的 `catalog_axes`）与其值根。
//! - **不碰发布链**：批次清单（`content_manifest`）、本批占位解析（`content_publication`）、
//!   独立出生/撤回索引、`batch3-`、Delta、clock 与前沿归 root。本模块不发出生许可，不写任何
//!   生命周期元数据，也不把 `wit-`/`window-` 之外的任何 `*_id` 当发布身份。
//! - **顺序不可颠倒**：调用方在 `generate` **之前**完成完整旧状态审计；生成后由调用方
//!   （或 [`Generated::verify_new_closure`]）核新闭包。本模块不替调用方做旧账审计。
//!
//! ## 同核三件套与跨代共享
//!
//! [`GenerationRuntime`] 持有同车道的 [`s_fact_cache::Lane`]（`IncrFactsState`）、
//! [`Cc004Producer`] 与 [`ContentBi`]。逐代动作：`bind_source` 登记真实生效账的当前源版本 →
//! **一代一次** `prepare_raw_history`、逐窗口 `build_fractal` → 生成态六条数组各走一份
//! [`SequenceMemo`]：未变前缀复用既有 `SequenceRef`，只有真实变化的尾巴重建。因此巨大源数组
//! （`raw_events`/`effective_events`）与新笔历史都不逐代整段重写；元素本身（有界小原件）
//! 逐条写节点。
//!
//! ## 本域边界（照实）
//!
//! - **只接线 OHLC 档案**（`tb02a::PROFILE`）。旧生成段对非 OHLC 档案另有一套 CC-006
//!   `source_coords` 配方（三个锚坐标，而非三组来源链），producer 没有实现那一支；本模块
//!   **显式拒绝**，不静默降级、不产出与旧配方不同的对象。
//! - **witness 配方**：§5.3 未定义 `wit3-`，且原文写「原 witness 不重命名」⟹ 保留 `wit-` +
//!   前 16 位十六进制，输入 = `{object_id(本域 obj3-), slot, mi, raw_bars}`；SHA256 走
//!   **规范 JSON 流式**写入（`render_canonical` 直接写哈希器），不 materialize。
//! - **目录轴两段 finish**：`catalog_axes` 依赖真实 withdrawals/replaces 与前代前沿，故
//!   `generate` 只产 14 字段草案根，[`Generated::finish_axes`] 才是最终根；未调用前
//!   `catalog_axes` 恒为 `None`，绝不假空。目录函数仍是 `tb02a_facts::catalog_axes` 原函数
//!   （不复刻公式），喂给它的是**最小消费投影**（见 `catalog_projection_entry`）。
//! - **事件节点 memo**：按 `(source_coord, revision)` 复用已写节点。该复用建立在本域既有
//!   不变式上——原始行按该键落库即不可变，且调用方的 2D 根审计在写入前逐字节核过；本 memo
//!   不重复那道审计，也不能替代它。
//!
//! ## 需要的 producer 最小公开口（一行，root 裁定）
//!
//! CC-005 组的等待原因只有一份实现（`content_producer::group_reasons`，约第 2524 行）。
//! 知识状态的 `waiting_reasons` 必须逐项等于旧配方，故本模块**调用**它而**不复写**判据：
//! 需把该函数从 `fn` 提升为 `pub(super) fn`。竞争极值窗判定不需要新口——直接用
//! `Cc004Producer::build_shape` 的 `Ok(None)`（其内部就是同一份
//! `shape_window_has_competing_extremes`）。

#![allow(dead_code)]

use super::audit_store::CodecRead;
use super::business_json::{self as bj, RawEventsView};
use super::object_records;
use crate::content_bi::{BiBuildInputView, BiLedgerView, ContentBi, PriorState};
use crate::content_producer::{
    array_from_sequence, group_reasons, group_window_sources, raw_prefix_coords, Cc004Producer,
    Cc006Object, Cc007Object, ProducerError, RawRefSpec, TypedObject, CC004_STEP_KIND,
    CC005_GROUP_KIND, CC006_SHAPE_KIND, CC007_FRACTAL_KIND, CC054_KNOWLEDGE_KIND,
};
use crate::s_fact_cache;
use newchan_rust::s_content::path::PathStep;
use newchan_rust::s_content::{
    self as sc, Built, ContentRef, Kind, NodeDigest, NodeStore, NumberToken, SequenceRef,
    Value as CValue,
};
use newchan_rust::session_protocol::parse_canonical_i64;
use newchan_rust::theta_v0::classifier::local_shape::{
    classify_local_shape_sliding, Cc006LocalShape,
};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser;
use newchan_rust::theta_v0::parser::inclusion::{GroupEvidenceView, IncrFactsState};
use newchan_rust::theta_v0::types::Bar;
use sc::capture_allocation::audit_json::JsonView;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// 生成输入：全部取调用方**同一读集**的真实值；本文不读库、不重排、不补默认。
pub(super) struct GenerationInput<'a> {
    /// 全修订史（`read_raw_events` 的 seq 序，含历史修订）。
    pub all_events: RawEventsView<'a>,
    /// 当前生效账（每个业务身份的最新 revision，按 source_coord 升序）。
    pub effective: RawEventsView<'a>,
    /// 本次生成态代号（进生成态标量与对象元数据投影）。
    pub generation: i64,
    /// 本次生成态 cut（旧配方 `cut-{gen}`；由调用方给出，本文不自行编造）。
    pub structure_cut: &'a str,
    /// 本次输入前沿（规范十进制进载荷/观察）。
    pub input_frontier: i64,
    pub profile_id: &'a str,
    pub profile_hash: &'a str,
    pub rule_revision: &'a str,
    pub catalog_revision: &'a str,
    /// 生成态 `scope`（旧配方对象，原样进生成态）。
    pub scope: &'a Value,
    /// 本次语义提交时钟（进 TB-02-B 的 `known_at`；缺省即 None）。
    pub semantic_commit_ns: Option<&'a str>,
    /// 上一代发布态的**已核** CC-010.bi / CC-056.version 记录根（调用方独立索引给出）。
    pub prior: PriorState<'a>,
}

/// 一条已构造对象：身份 + 生成态记录 + 可复用的构造结果。
///
/// 顺序（生成态 `objects` 数组）＝旧 sealed 批次：CC-006 形态（窗序）→ 已按
/// `(kind, fact_key, object_id)` 排序的泛型事实 → TB-02-B 对象（线序）。
pub(super) struct ProducedObject {
    kind: String,
    fact_key: Option<String>,
    object_id: String,
    generated: Built,
    body: ObjectBody,
}

enum ObjectBody {
    Typed(TypedObject),
    Fractal(Cc007Object),
    Shape(Cc006Object),
}

impl ProducedObject {
    pub(super) fn object_id(&self) -> &str {
        &self.object_id
    }

    pub(super) fn kind(&self) -> &str {
        &self.kind
    }

    /// 泛型六字段对象的槽位规范文本；CC-006 的旧配方没有该字段（返回 `None`）。
    pub(super) fn fact_key(&self) -> Option<&str> {
        self.fact_key.as_deref()
    }

    /// 生成态记录值根（原 sealed `generated_state.objects[i]` 的逻辑字段）。
    pub(super) fn generated(&self) -> &Built {
        &self.generated
    }

    /// 公开 wire 模板：`life` 由调用方的独立索引给出；本批新生对象写 0x08 占位。
    pub(super) fn wire_template(
        &self,
        store: &mut dyn NodeStore,
        life: &object_records::PublicationLife,
        current_generation: i64,
    ) -> Result<Built, object_records::RecordError> {
        object_records::wire_template(store, self.borrowed(), life, current_generation)
    }

    /// 内部构造记录借用视图：根接线在同一批产物内直接消费（`content_bi::PriorState` 的
    /// 13 字段 private 根），不再由调用方重复构造。
    pub(super) fn borrowed(&self) -> object_records::BorrowedObject<'_> {
        match &self.body {
            ObjectBody::Typed(object) => object_records::BorrowedObject::Typed(object),
            ObjectBody::Fractal(object) => object_records::BorrowedObject::Fractal(object),
            ObjectBody::Shape(object) => object_records::BorrowedObject::Shape(object),
        }
    }

    /// 对象的公开载荷值根（CC-006 旧配方没有 payload 字段，故为 `None`）。
    fn payload(&self) -> Option<&Built> {
        match &self.body {
            ObjectBody::Typed(object) => Some(&object.payload),
            ObjectBody::Fractal(object) => Some(&object.payload),
            ObjectBody::Shape(_) => None,
        }
    }
}

/// 一条 CC-006 见证（生成态字段与旧配方同名同型；身份由真实新 `obj3-` 派生）。
pub(super) struct WitnessRecord {
    pub witness_id: String,
    pub object_id: String,
    pub slot: usize,
    pub built: Built,
}

/// 跨代共享的两条原始账数组根。
pub(super) struct RawRoots {
    pub raw_events: ContentRef,
    pub effective_events: ContentRef,
}

/// 生成态标量（`emit_state` 的输入；OHLC 两段 finish 时复用，不重造对象）。
struct StateMeta {
    generation: i64,
    structure_cut: String,
    input_frontier: i64,
    catalog_revision: String,
    profile_id: String,
    profile_hash: String,
    rule_revision: String,
    scope: CValue,
}

/// 生成态六条数组根（数组节点；其序列在 [`SequenceMemo`] 里跨代续接）。
struct StateRoots {
    raw_events: Built,
    effective_events: Built,
    objects: Built,
    witnesses: Built,
    relations: Built,
    observations: Built,
}

/// 一代生成结果。
///
/// 生命周期参数只借调用方的 `all_events`（目录轴需要真实 seq/revision 投影，不复制整份历史）。
pub(super) struct Generated<'a> {
    /// 顺序 = 旧 sealed 批次（见 [`ProducedObject`]）。
    pub objects: Vec<ProducedObject>,
    /// 顺序 = 形态序 × 槽位 0..3（旧配方逐字同序）。
    pub witnesses: Vec<WitnessRecord>,
    /// 生成态 `relations` 值边：has_witness/classified_as + 新笔关系。
    /// `supersedes`/`replaces` 属发布期，由 root 追加，不在本集合内。
    pub relations: Vec<Built>,
    /// 生成态 `observations` 值边（已按旧配方去重）。
    pub observations: Vec<Built>,
    pub raw: RawRoots,
    /// OHLC 的真实目录轴（`finish_axes` 之前恒 `None`，绝不假空）。
    pub catalog_axes: Option<ContentRef>,
    /// 14 字段草案根；OHLC 时必须经 [`Generated::finish_axes`] 才是最终根。
    pub state_without_axes: ContentRef,
    /// 当前最终生成态根（OHLC 在 `finish_axes` 成功后更新）。
    pub generated_state: ContentRef,
    meta: StateMeta,
    roots: StateRoots,
    /// 本代知识请求线序（仅供目录轴最小投影消费，不进生成态对象）。
    requests: Vec<Built>,
    all_events: RawEventsView<'a>,
    is_ohlc: bool,
}

impl<'a> Generated<'a> {
    /// 生成之后的**新闭包**核验（§5.4 写前读集的一道）；不替代调用方的完整审计。
    pub(super) fn verify_new_closure(&self, store: &mut dyn NodeStore) -> Result<(), String> {
        sc::verify(&mut *store, &self.generated_state)
            .map_err(|error| format!("GenError：生成态闭包核验失败：{error}"))?;
        Ok(())
    }

    /// 第二段 finish：把**真实** withdrawals/replaces 与前代前沿交给目录轴，重写最终根。
    ///
    /// 只改生成态对象节点与 `catalog_axes`，不展开任何大字段；对象/见证/关系/观察的内容根
    /// 都不变。非 OHLC 域没有该字段，调用即拒绝（不假空）。
    pub(super) fn finish_axes(
        &mut self,
        store: &mut dyn NodeStore,
        previous_frontier: i64,
        withdrawals: &[Value],
        replaces: &[Value],
    ) -> Result<ContentRef, String> {
        if !self.is_ohlc {
            return Err(
                "VersionUnsupported：目录轴只存在于 OHLC 档案；非 OHLC 生成态没有该字段，拒绝假空"
                    .to_string(),
            );
        }
        let mut projection: Vec<Value> = Vec::with_capacity(self.objects.len());
        for object in &self.objects {
            projection.push(catalog_projection_entry(
                &mut *store,
                object,
                &self.requests,
            )?);
        }
        let axes_value = crate::tb02a_facts::catalog_axes(
            &projection,
            self.all_events,
            previous_frontier,
            withdrawals,
            replaces,
        );
        let axes = sc::write_value(&mut *store, &json_to_cvalue(&axes_value)?)
            .map_err(|error| format!("GenError：目录轴写入失败：{error}"))?;
        let reference = axes.content_ref();
        let state = emit_state(store, &self.meta, &self.roots, Some(&reference))?;
        self.catalog_axes = Some(reference);
        self.generated_state = state.clone();
        Ok(state)
    }
}

/// 同核增量生成运行态（一个车道一份；事务回滚/换库后必须 `discard_runtime_state`）。
pub(super) struct GenerationRuntime {
    lane_key: String,
    producer: Cc004Producer,
    bi: ContentBi,
    /// 会话冻结身份（进身份配方）；逐代输入必须与之一致，不一致即拒绝。
    profile_id: String,
    rule_revision: String,
    /// `(source_coord, revision)` → 已写事件节点摘要（原件不可变；见模块文档）。
    event_nodes: BTreeMap<(i64, i64), NodeDigest>,
    raw_events_seq: SequenceMemo,
    effective_events_seq: SequenceMemo,
    objects_seq: SequenceMemo,
    witnesses_seq: SequenceMemo,
    relations_seq: SequenceMemo,
    observations_seq: SequenceMemo,
}

impl GenerationRuntime {
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn visit_runtime_roots(&self, f: &mut dyn FnMut(&'static str, NodeDigest)) {
        self.producer.visit_runtime_roots(f);
        self.bi.visit_runtime_roots(f);
        for root in self.event_nodes.values() {
            f("generation.event_nodes", *root);
        }
        for (name, memo) in [
            ("generation.raw_events_seq", &self.raw_events_seq),
            (
                "generation.effective_events_seq",
                &self.effective_events_seq,
            ),
            ("generation.objects_seq", &self.objects_seq),
            ("generation.witnesses_seq", &self.witnesses_seq),
            ("generation.relations_seq", &self.relations_seq),
            ("generation.observations_seq", &self.observations_seq),
        ] {
            memo.visit_runtime_roots(name, f);
        }
    }

    /// `lane_key` 按 `s_fact_cache::lane_key` 的持久身份口径给出（库/会话/规则/档案/目录）。
    pub(super) fn new(lane_key: String, profile_id: &str, rule_revision: &str) -> Self {
        GenerationRuntime {
            lane_key,
            producer: Cc004Producer::new(profile_id, rule_revision),
            bi: ContentBi::new(),
            profile_id: profile_id.to_string(),
            rule_revision: rule_revision.to_string(),
            event_nodes: BTreeMap::new(),
            raw_events_seq: SequenceMemo::default(),
            effective_events_seq: SequenceMemo::default(),
            objects_seq: SequenceMemo::default(),
            witnesses_seq: SequenceMemo::default(),
            relations_seq: SequenceMemo::default(),
            observations_seq: SequenceMemo::default(),
        }
    }

    /// 事务回滚 / 换库 / 换来源域之后必须调用：清掉同核运行态与全部跨代序列记忆。
    ///
    /// 车道（`IncrFactsState`）按持久身份键隔离且 `advance` 对同一输入幂等，这里不重建；
    /// 键不符时 `LaneGuard::take` 自然丢弃旧车道。
    pub(super) fn discard_runtime_state(&mut self) {
        self.bi.discard_runtime_state(&mut self.producer);
        self.event_nodes.clear();
        self.raw_events_seq.discard();
        self.effective_events_seq.discard();
        self.objects_seq.discard();
        self.witnesses_seq.discard();
        self.relations_seq.discard();
        self.observations_seq.discard();
    }

    /// 一代同核生成（只写节点，不发布）。
    pub(super) fn generate<'b>(
        &mut self,
        store: &mut dyn NodeStore,
        input: &GenerationInput<'b>,
    ) -> Result<Generated<'b>, String> {
        if input.profile_id != self.profile_id || input.rule_revision != self.rule_revision {
            return Err(format!(
                "IdentityConflict：生成运行态的档案/规则修订（{}/{}）与本次输入（{}/{}）不一致",
                self.profile_id, self.rule_revision, input.profile_id, input.rule_revision
            ));
        }
        if input.profile_id != crate::tb02a::PROFILE {
            return Err(format!(
                "VersionUnsupported：S3 同核生成只接线 OHLC 档案 `{}`（CC-006 来源集合配方按档案分派，非 OHLC 支未接线；不静默降级）",
                crate::tb02a::PROFILE
            ));
        }
        let gen = input.generation;
        let cut = input.structure_cut;
        let frontier = input.input_frontier;
        let config = ThetaConfig::default();

        // ── 原始账：生效账 → coord 表 + Bar 序列 + 事件节点；全修订史 → 事件节点 ──
        let mut raw_by_coord: BTreeMap<i64, JsonView<'_>> = BTreeMap::new();
        let mut raw_bars: Vec<Bar> = Vec::with_capacity(input.effective.len());
        let mut effective_nodes: Vec<NodeDigest> = Vec::with_capacity(input.effective.len());
        let mut coord_nodes: BTreeMap<i64, NodeDigest> = BTreeMap::new();
        for event in input.effective {
            let coord = bj::coord(event)?;
            if raw_by_coord.insert(coord, event).is_some() {
                return Err("StorageUnavailable：有效源坐标碰撞，拒绝覆盖见证".to_string());
            }
            raw_bars.push(bar_of(event)?);
            let node = event_node(&mut self.event_nodes, store, event)?;
            coord_nodes.insert(coord, node);
            effective_nodes.push(node);
        }
        let mut all_nodes: Vec<NodeDigest> = Vec::with_capacity(input.all_events.len());
        for event in input.all_events {
            all_nodes.push(event_node(&mut self.event_nodes, store, event)?);
        }
        for (coord, event) in &raw_by_coord {
            self.producer
                .bind_source(store, *coord, raw_ref_spec(*event)?)
                .map_err(producer_error)?;
        }

        // ── 同核解析：与旧入口同款「本代取用车道、用完归还线程槽位」──
        let mut lane_guard = s_fact_cache::LaneGuard::take(&self.lane_key);
        let lane = lane_guard.lane(&self.lane_key);
        lane.advance(&raw_bars);
        let state: &IncrFactsState = lane.state();
        let (layer, evidence) = parser::parse_layer_with_group_evidence(state, &config);
        let merged_rc = layer.merged_bars.clone();
        let merged: &[Bar] = merged_rc.as_ref();
        let prepared = self
            .producer
            .prepare_raw_history(&raw_bars)
            .map_err(producer_error)?;

        // ── 形态窗：竞争极值判决取 producer 的同一条判据（build_shape 的 Ok(None)）──
        let wins = classify_local_shape_sliding(merged);
        let mut shape_objects: Vec<ProducedObject> = Vec::new();
        let mut witnesses: Vec<WitnessRecord> = Vec::new();
        let mut relations: Vec<Built> = Vec::new();
        let mut observations: Vec<(bool, Built)> = Vec::new();
        let mut competing: Vec<usize> = Vec::new();
        // 逐形态的请求组（旧配方线序：形态请求 → 该形态的 CC-007 三条请求）。
        let mut shape_requests: Vec<(Built, Vec<Built>)> = Vec::new();
        let mut fractals: Vec<ProducedObject> = Vec::new();
        let mut fractal_reasons: Vec<&'static str> = Vec::new();
        for (start, shape) in &wins {
            let start = *start;
            match (
                shape,
                self.producer.build_shape(store, state, start, gen, cut),
            ) {
                (_, Ok(None)) => competing.push(start),
                (_, Ok(Some(object))) => {
                    let views = group_views(state, start)?;
                    let (wits, rels) = shape_witnesses(
                        store,
                        &mut self.event_nodes,
                        &raw_by_coord,
                        &coord_nodes,
                        &views,
                        start,
                        &object,
                    )?;
                    witnesses.extend(wits);
                    relations.extend(rels);
                    let request = object.request.clone();
                    let fractal = self
                        .producer
                        .build_fractal(
                            store,
                            &prepared,
                            &object,
                            [&views[0], &views[1], &views[2]],
                            gen,
                            cut,
                        )
                        .map_err(producer_error)?;
                    let mut fractal_requests: Vec<Built> = Vec::new();
                    if let Some(fractal_object) = fractal {
                        fractal_reasons.extend(fractal_object.knowledge_reasons.iter().copied());
                        fractal_requests.extend(fractal_object.requests.iter().cloned());
                        let generated = object_records::generated(
                            store,
                            object_records::BorrowedObject::Fractal(&fractal_object),
                        )
                        .map_err(record_error)?;
                        fractals.push(ProducedObject {
                            kind: CC007_FRACTAL_KIND.to_string(),
                            fact_key: Some(fractal_object.fact_key.clone()),
                            object_id: fractal_object.object_id.clone(),
                            generated,
                            body: ObjectBody::Fractal(fractal_object),
                        });
                    }
                    shape_requests.push((request, fractal_requests));
                    let generated = object_records::generated(
                        store,
                        object_records::BorrowedObject::Shape(&object),
                    )
                    .map_err(record_error)?;
                    shape_objects.push(ProducedObject {
                        kind: CC006_SHAPE_KIND.to_string(),
                        fact_key: None,
                        object_id: object.object_id.clone(),
                        generated,
                        body: ObjectBody::Shape(object),
                    });
                }
                (
                    Cc006LocalShape::DomainNotSatisfied { reason },
                    Err(ProducerError::WindowNotClassified(_)),
                ) => {
                    let value = serde_json::json!({
                        "kind": "domain_not_satisfied",
                        "reason": reason.as_str(),
                        "window_start": start,
                        "window_mid": start + 1,
                        "window_end": start + 2,
                        "detail": {},
                    });
                    observations.push((false, write_json(store, &value)?));
                }
                (
                    Cc006LocalShape::InsufficientKnowledge,
                    Err(ProducerError::WindowNotClassified(_)),
                ) => {
                    let value = serde_json::json!({
                        "kind": "insufficient_knowledge",
                        "reason": "fewer_than_three_bars",
                        "window_start": start,
                        "window_mid": Value::Null,
                        "window_end": Value::Null,
                        "detail": {},
                    });
                    observations.push((false, write_json(store, &value)?));
                }
                (classified, Err(error)) => {
                    return Err(format!(
                        "GenError：形态窗 {start} 构造失败（滑窗判据={classified:?}）：{}",
                        producer_error(error)
                    ));
                }
            }
        }

        // ── 泛型事实：步骤 → 组 → 竞争窗待决请求 → 形态/分型请求 → 不足三根待决请求 ──
        let mut typed: Vec<ProducedObject> = Vec::new();
        let mut step_requests: Vec<Built> = Vec::new();
        let mut group_requests: Vec<Built> = Vec::new();
        let mut all_reasons: Vec<&'static str> = Vec::new();
        for index in 0..state.step_count() {
            let view = state
                .shared_step(index)
                .map_err(|error| format!("GenError：共享步骤证据不可用：{}", error.reason))?
                .ok_or_else(|| format!("GenError：步骤 {index} 缺共享证据"))?;
            all_reasons.extend_from_slice(view.waiting_reasons);
            let object = self
                .producer
                .build_step_object(store, &view, gen, cut)
                .map_err(producer_error)?;
            step_requests.push(object.request.clone().ok_or_else(|| {
                format!("GenError：CC-004 步骤 {index} 缺知识请求（旧配方必须有）")
            })?);
            let generated =
                object_records::generated(store, object_records::BorrowedObject::Typed(&object))
                    .map_err(record_error)?;
            typed.push(ProducedObject {
                kind: CC004_STEP_KIND.to_string(),
                fact_key: Some(object.fact_key.clone()),
                object_id: object.object_id.clone(),
                generated,
                body: ObjectBody::Typed(object),
            });
        }
        for index in 0..state.group_count() {
            let view = state
                .shared_group(index)
                .map_err(|error| format!("GenError：共享组证据不可用：{}", error.reason))?
                .ok_or_else(|| format!("GenError：组 {index} 缺共享证据"))?;
            all_reasons.extend_from_slice(&group_reasons(&view));
            let object = self
                .producer
                .build_group_object(store, index, &view, gen, cut)
                .map_err(producer_error)?;
            group_requests.push(object.request.clone().ok_or_else(|| {
                format!("GenError：CC-005 组 {index} 缺知识请求（旧配方必须有）")
            })?);
            let generated =
                object_records::generated(store, object_records::BorrowedObject::Typed(&object))
                    .map_err(record_error)?;
            typed.push(ProducedObject {
                kind: CC005_GROUP_KIND.to_string(),
                fact_key: Some(object.fact_key.clone()),
                object_id: object.object_id.clone(),
                generated,
                body: ObjectBody::Typed(object),
            });
        }
        let mut requests: Vec<Built> = Vec::new();
        requests.extend(step_requests);
        requests.extend(group_requests);
        for start in &competing {
            let views = group_views(state, *start)?;
            let key = CValue::Array(vec![
                CValue::String(CC006_SHAPE_KIND.to_string()),
                CValue::Array(
                    [
                        views[0].bar.source_index.to_string(),
                        views[1].bar.source_index.to_string(),
                        views[2].bar.source_index.to_string(),
                    ]
                    .into_iter()
                    .map(CValue::String)
                    .collect(),
                ),
            ]);
            let sources = group_window_sources(&views);
            requests.push(
                self.producer
                    .pending_request(store, key, &["equal_extreme_identity"], &sources)
                    .map_err(producer_error)?,
            );
        }
        for (shape_request, fractal_requests) in &shape_requests {
            requests.push(shape_request.clone());
            requests.extend(fractal_requests.iter().cloned());
        }
        if merged.len() < 3 {
            all_reasons.push("fewer_than_three_merged_bars");
            let key = CValue::Array(vec![
                CValue::String(CC006_SHAPE_KIND.to_string()),
                CValue::String("pending".to_string()),
            ]);
            let sources = raw_prefix_coords(state).map_err(producer_error)?;
            requests.push(
                self.producer
                    .pending_request(store, key, &["fewer_than_three_merged_bars"], &sources)
                    .map_err(producer_error)?,
            );
        }
        if !state.has_direction_evidence() {
            all_reasons.push("direction_not_yet_established");
        }
        if state.unsettled() {
            all_reasons.push("initial_direction_unsettled");
        }
        all_reasons.extend(fractal_reasons);
        typed.extend(fractals);
        typed.sort_by(|left, right| {
            (
                left.kind.as_str(),
                left.fact_key.as_deref(),
                left.object_id.as_str(),
            )
                .cmp(&(
                    right.kind.as_str(),
                    right.fact_key.as_deref(),
                    right.object_id.as_str(),
                ))
        });

        // ── CC-054 知识状态（旧配方在泛型事实之后、TB-02-B 之前）──
        let raw_coords = raw_prefix_coords(state).map_err(producer_error)?;
        let knowledge = self
            .producer
            .build_knowledge_state(
                store,
                &raw_coords,
                frontier,
                &requests,
                &all_reasons,
                gen,
                cut,
            )
            .map_err(producer_error)?;
        for observation in &knowledge.observations {
            observations.push((true, observation.clone()));
        }
        let generated = object_records::generated(
            store,
            object_records::BorrowedObject::Typed(&knowledge.object),
        )
        .map_err(record_error)?;
        typed.push(ProducedObject {
            kind: CC054_KNOWLEDGE_KIND.to_string(),
            fact_key: Some(knowledge.object.fact_key.clone()),
            object_id: knowledge.object.object_id.clone(),
            generated,
            body: ObjectBody::Typed(knowledge.object),
        });

        // ── TB-02-B：七个 kind 的新笔证据（上一代记录由调用方给根）──
        let bi = self
            .bi
            .build_borrowed(
                &mut self.producer,
                store,
                &BiBuildInputView {
                    facts: &evidence.stroke_facts,
                    raw: BiLedgerView::Views(&raw_by_coord),
                    all_events: JsonView::BorrowedArray(&input.all_events)
                        .as_array()
                        .unwrap(),
                    prior: &input.prior,
                    generation: gen,
                    cut,
                    frontier,
                    semantic_commit_ns: input.semantic_commit_ns,
                },
            )
            .map_err(|error| format!("GenError：TB-02-B 构造失败：{error}"))?;
        for object in bi.objects {
            let kind = typed_kind(store, &object)?;
            let generated =
                object_records::generated(store, object_records::BorrowedObject::Typed(&object))
                    .map_err(record_error)?;
            typed.push(ProducedObject {
                kind,
                fact_key: Some(object.fact_key.clone()),
                object_id: object.object_id.clone(),
                generated,
                body: ObjectBody::Typed(object),
            });
        }
        for relation in &bi.relations {
            relations.push(
                sc::write_value(&mut *store, relation)
                    .map_err(|error| format!("GenError：关系值写入失败：{error}"))?,
            );
        }

        // ── 生成态六条数组：跨代共享序列 + 旧配方等价去重 ──
        let mut objects: Vec<ProducedObject> = shape_objects;
        objects.extend(typed);
        let deduped = dedupe_observations(store, &observations)?;
        let raw_events_array = self.raw_events_seq.sync(store, &all_nodes, "raw_events")?;
        let effective_array =
            self.effective_events_seq
                .sync(store, &effective_nodes, "effective_events")?;
        let object_nodes: Vec<NodeDigest> = objects
            .iter()
            .map(|object| object.generated.digest)
            .collect();
        let witness_nodes: Vec<NodeDigest> = witnesses
            .iter()
            .map(|witness| witness.built.digest)
            .collect();
        let relation_nodes: Vec<NodeDigest> =
            relations.iter().map(|relation| relation.digest).collect();
        let observation_nodes: Vec<NodeDigest> = deduped
            .iter()
            .map(|observation| observation.digest)
            .collect();
        let objects_array = self.objects_seq.sync(store, &object_nodes, "objects")?;
        let witnesses_array = self
            .witnesses_seq
            .sync(store, &witness_nodes, "witnesses")?;
        let relations_array = self
            .relations_seq
            .sync(store, &relation_nodes, "relations")?;
        let observations_array =
            self.observations_seq
                .sync(store, &observation_nodes, "observations")?;

        let meta = StateMeta {
            generation: gen,
            structure_cut: cut.to_string(),
            input_frontier: frontier,
            catalog_revision: input.catalog_revision.to_string(),
            profile_id: input.profile_id.to_string(),
            profile_hash: input.profile_hash.to_string(),
            rule_revision: input.rule_revision.to_string(),
            scope: json_to_cvalue(input.scope)?,
        };
        let roots = StateRoots {
            raw_events: raw_events_array,
            effective_events: effective_array,
            objects: objects_array,
            witnesses: witnesses_array,
            relations: relations_array,
            observations: observations_array,
        };
        let state_without_axes = emit_state(store, &meta, &roots, None)?;
        Ok(Generated {
            objects,
            witnesses,
            relations,
            observations: deduped,
            raw: RawRoots {
                raw_events: roots.raw_events.content_ref(),
                effective_events: roots.effective_events.content_ref(),
            },
            catalog_axes: None,
            state_without_axes: state_without_axes.clone(),
            generated_state: state_without_axes,
            meta,
            roots,
            requests,
            all_events: input.all_events,
            is_ohlc: true,
        })
    }
}

/// 生成态对象（14 字段 + 可选 `catalog_axes`）：只写对象节点，六条数组借既有根。
fn emit_state(
    store: &mut dyn NodeStore,
    meta: &StateMeta,
    roots: &StateRoots,
    axes: Option<&ContentRef>,
) -> Result<ContentRef, String> {
    let mut fields: Vec<(String, CValue)> = vec![
        ("generation".into(), number(meta.generation)?),
        (
            "structure_cut".into(),
            CValue::String(meta.structure_cut.clone()),
        ),
        ("input_frontier".into(), number(meta.input_frontier)?),
        (
            "catalog_revision".into(),
            CValue::String(meta.catalog_revision.clone()),
        ),
        ("profile_id".into(), CValue::String(meta.profile_id.clone())),
        (
            "profile_hash".into(),
            CValue::String(meta.profile_hash.clone()),
        ),
        (
            "rule_revision".into(),
            CValue::String(meta.rule_revision.clone()),
        ),
        ("raw_events".into(), CValue::Edge(roots.raw_events.digest)),
        (
            "effective_events".into(),
            CValue::Edge(roots.effective_events.digest),
        ),
        ("objects".into(), CValue::Edge(roots.objects.digest)),
        ("witnesses".into(), CValue::Edge(roots.witnesses.digest)),
        ("relations".into(), CValue::Edge(roots.relations.digest)),
        (
            "observations".into(),
            CValue::Edge(roots.observations.digest),
        ),
        ("scope".into(), meta.scope.clone()),
    ];
    if let Some(axes) = axes {
        fields.push((
            "catalog_axes".into(),
            CValue::Edge(axes.digest().map_err(codec_error)?),
        ));
    }
    let built = sc::write_value(&mut *store, &CValue::Object(fields))
        .map_err(|error| format!("GenError：生成态写入失败：{error}"))?;
    Ok(built.content_ref())
}

/// 跨代共享序列记忆：未变前缀不重写，真变化处 `sequence_split` + 尾部续接。
#[derive(Default)]
pub(super) struct SequenceMemo {
    keys: Vec<NodeDigest>,
    seq: Option<SequenceRef>,
}

impl SequenceMemo {
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn visit_runtime_roots(
        &self,
        name: &'static str,
        f: &mut dyn FnMut(&'static str, NodeDigest),
    ) {
        for root in &self.keys {
            f(name, *root);
        }
        if let Some(seq) = self.seq {
            f(name, seq.root());
        }
    }

    fn discard(&mut self) {
        self.keys.clear();
        self.seq = None;
    }

    pub(super) fn sync(
        &mut self,
        store: &mut dyn NodeStore,
        elements: &[NodeDigest],
        what: &'static str,
    ) -> Result<Built, String> {
        let common = self
            .keys
            .iter()
            .zip(elements.iter())
            .take_while(|(left, right)| left == right)
            .count();
        let consistent = self
            .seq
            .as_ref()
            .is_some_and(|seq| seq.count() == self.keys.len() as u64);
        let seq = match (self.seq, consistent) {
            (Some(seq), true) if common == self.keys.len() => {
                let mut seq = seq;
                for element in &elements[common..] {
                    seq = sc::sequence_append(&mut *store, &seq, element)
                        .map_err(|error| format!("GenError：{what} 序列续接失败：{error}"))?;
                }
                seq
            }
            (Some(seq), true) if common > 0 => {
                let (mut left, _right) = sc::sequence_split(&mut *store, &seq, common as u64)
                    .map_err(|error| format!("GenError：{what} 序列截断失败：{error}"))?;
                for element in &elements[common..] {
                    left = sc::sequence_append(&mut *store, &left, element)
                        .map_err(|error| format!("GenError：{what} 序列续接失败：{error}"))?;
                }
                left
            }
            _ => {
                let root = sc::write_sequence(&mut *store, elements)
                    .map_err(|error| format!("GenError：{what} 序列写入失败：{error}"))?;
                SequenceRef::from_parts_unchecked(root, elements.len() as u64)
            }
        };
        self.keys.clear();
        self.keys.extend_from_slice(elements);
        self.seq = Some(seq);
        array_from_sequence(&mut *store, &seq).map_err(producer_error)
    }
}

/// 旧配方去重的**等价实现**：`detail.tb02a == true` 的一族按规范编码摘要比较，
/// 其余按 `kind|window_start|window_mid|window_end|reason`。两类各自等价类与旧实现的文本键
/// 逐个相同（等值 ⟺ 同摘要），且不展开观察 `detail.source_coords` 大边。
fn dedupe_observations(
    store: &mut dyn NodeStore,
    observations: &[(bool, Built)],
) -> Result<Vec<Built>, String> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out: Vec<Built> = Vec::with_capacity(observations.len());
    for (from_knowledge, built) in observations {
        let key = if *from_knowledge {
            format!("\u{1}tb02a\u{1}{}", built.root())
        } else {
            let root = built.content_ref();
            format!(
                "{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}",
                field_text(store, &root, "kind")?,
                field_text(store, &root, "window_start")?,
                field_text(store, &root, "window_mid")?,
                field_text(store, &root, "window_end")?,
                field_text(store, &root, "reason")?,
            )
        };
        if seen.insert(key) {
            out.push(built.clone());
        }
    }
    Ok(out)
}

/// 目录轴的**最小消费投影**：每项只给 `object_id`/`kind` 与目录函数真正读取的小路径值。
///
/// 明确声明：这里的 `payload_json` 是**仅供 `tb02a_facts::catalog_axes` 消费的投影**，
/// 不是生成态对象、不是 `payload` 值根、不进任何身份配方。字段全部是真实小路径：
///
/// - CC-004 步骤：`{"comparisons": <真实比较集>}`（旧函数只读它判 CC-001/002/003 非空）；
/// - CC-054 知识状态：`{"requests":[{"reasons":..,"request_id":..}..],"waiting_reasons":..}`；
/// - CC-056 版本：`{"data":{"waiting_reasons":..}}`；
/// - 其余 kind：空对象（旧函数对这些 kind 从不解析 `payload_json`）。
///
/// 不读 `source_coords`/`input_refs` 等大边，也不重造整份 payload。
fn catalog_projection_entry(
    store: &mut dyn NodeStore,
    object: &ProducedObject,
    requests: &[Built],
) -> Result<Value, String> {
    let mut entry = serde_json::Map::new();
    entry.insert(
        "object_id".to_string(),
        Value::String(object.object_id().to_string()),
    );
    entry.insert("kind".to_string(), Value::String(object.kind().to_string()));
    let projection = if object.kind() == CC004_STEP_KIND {
        let payload = object.payload().ok_or_else(|| {
            format!(
                "GenError：CC-004 对象 {} 缺 payload 值根",
                object.object_id()
            )
        })?;
        let comparisons = path_of(store, &payload.content_ref(), &[step_field("comparisons")])?;
        format!("{{\"comparisons\":{}}}", render_text(store, &comparisons)?)
    } else if object.kind() == CC054_KNOWLEDGE_KIND {
        let payload = object.payload().ok_or_else(|| {
            format!(
                "GenError：CC-054 对象 {} 缺 payload 值根",
                object.object_id()
            )
        })?;
        let root = payload.content_ref();
        let waiting_path = path_of(store, &root, &[step_field("waiting_reasons")])?;
        let waiting = render_text(store, &waiting_path)?;
        let mut request_items: Vec<Value> = Vec::with_capacity(requests.len());
        for request in requests {
            let request_root = request.content_ref();
            let request_id_path = path_of(store, &request_root, &[step_field("request_id")])?;
            // `request_id` 在载荷里是**文本字段**，其内容才是规范键 JSON；投影必须还原成
            // 同一个字符串值（旧目录函数随后 `from_str` 得到键数组），不能带渲染外层引号。
            let request_id = parse_small_json(
                &render_text(store, &request_id_path)?,
                "知识请求 request_id",
            )?;
            let reasons_path = path_of(store, &request_root, &[step_field("reasons")])?;
            let reasons = render_text(store, &reasons_path)?;
            request_items.push(serde_json::json!({
                "request_id": request_id,
                "reasons": parse_small_json(&reasons, "知识请求 reasons")?,
            }));
        }
        serde_json::json!({
            "waiting_reasons": parse_small_json(&waiting, "知识状态 waiting_reasons")?,
            "requests": request_items,
        })
        .to_string()
    } else if object.kind() == "CC-056.version" {
        let payload = object.payload().ok_or_else(|| {
            format!(
                "GenError：CC-056 对象 {} 缺 payload 值根",
                object.object_id()
            )
        })?;
        let root = payload.content_ref();
        let waiting_path = path_of(
            store,
            &root,
            &[step_field("data"), step_field("waiting_reasons")],
        )?;
        let waiting = render_text(store, &waiting_path)?;
        format!("{{\"data\":{{\"waiting_reasons\":{waiting}}}}}")
    } else {
        String::from("{}")
    };
    entry.insert("payload_json".to_string(), Value::String(projection));
    Ok(Value::Object(entry))
}

fn step_field(name: &str) -> PathStep {
    PathStep::Field(name.to_string())
}

fn path_of(
    store: &mut dyn NodeStore,
    root: &ContentRef,
    path: &[PathStep],
) -> Result<ContentRef, String> {
    sc::path::lookup_path(&mut *store, root, path)
        .map_err(|error| format!("GenError：小路径定位失败：{error}"))
}

/// 小字段规范渲染（只对目录轴消费的小路径调用，不用于大载荷）。
fn render_text(store: &mut dyn NodeStore, reference: &ContentRef) -> Result<String, String> {
    let mut out: Vec<u8> = Vec::new();
    sc::render_canonical(&mut *store, reference, &mut out)
        .map_err(|error| format!("GenError：小字段渲染失败：{error}"))?;
    String::from_utf8(out).map_err(|error| format!("GenError：小字段渲染非 UTF-8：{error}"))
}

fn field_text(store: &mut dyn NodeStore, root: &ContentRef, name: &str) -> Result<String, String> {
    let path = path_of(store, root, &[step_field(name)])?;
    render_text(store, &path)
}

fn parse_small_json(text: &str, what: &'static str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|error| format!("GenError：{what} 渲染不是 JSON：{error}"))
}

/// CC-006 三组窗口的见证与关系（配方与旧生成段逐字相同，只换真实新 `obj3-` 身份）。
fn shape_witnesses(
    store: &mut dyn NodeStore,
    event_nodes: &mut BTreeMap<(i64, i64), NodeDigest>,
    raw_by_coord: &BTreeMap<i64, JsonView<'_>>,
    coord_nodes: &BTreeMap<i64, NodeDigest>,
    views: &[GroupEvidenceView<'_>; 3],
    start: usize,
    object: &Cc006Object,
) -> Result<(Vec<WitnessRecord>, Vec<Built>), String> {
    let mut witnesses: Vec<WitnessRecord> = Vec::with_capacity(3);
    let mut relations: Vec<Built> = Vec::with_capacity(4);
    for (slot, view) in views.iter().enumerate() {
        let mi = start + slot;
        let mut bar_nodes: Vec<NodeDigest> = Vec::with_capacity(view.members.len());
        for coord in view.members {
            let coord = *coord as i64;
            let event = raw_by_coord.get(&coord).ok_or_else(|| {
                format!("StorageUnavailable：见证成员坐标 {coord} 不在本次生效账内")
            })?;
            match coord_nodes.get(&coord) {
                Some(node) => bar_nodes.push(*node),
                None => bar_nodes.push(event_node(event_nodes, store, *event)?),
            }
        }
        let bar_array = sc::write_value(
            &mut *store,
            &CValue::Array(bar_nodes.into_iter().map(CValue::Edge).collect()),
        )
        .map_err(|error| format!("GenError：见证 raw_bars 写入失败：{error}"))?;
        let witness_id = witness_identity(store, &object.object_id, slot, mi, &bar_array)?;
        let record = CValue::Object(vec![
            (
                "merged_close".to_string(),
                CValue::String(view.bar.close.to_string()),
            ),
            (
                "merged_high".to_string(),
                CValue::String(view.bar.high.to_string()),
            ),
            (
                "merged_low".to_string(),
                CValue::String(view.bar.low.to_string()),
            ),
            (
                "merged_open".to_string(),
                CValue::String(view.bar.open.to_string()),
            ),
            (
                "merged_source_index".to_string(),
                number(view.bar.source_index as i64)?,
            ),
            (
                "object_id".to_string(),
                CValue::String(object.object_id.clone()),
            ),
            ("raw_bars".to_string(), CValue::Edge(bar_array.digest)),
            ("slot".to_string(), number(slot as i64)?),
            ("witness_id".to_string(), CValue::String(witness_id.clone())),
        ]);
        let built = sc::write_value(&mut *store, &record)
            .map_err(|error| format!("GenError：见证记录写入失败：{error}"))?;
        witnesses.push(WitnessRecord {
            witness_id: witness_id.clone(),
            object_id: object.object_id.clone(),
            slot,
            built,
        });
        let relation = CValue::Object(vec![
            ("object".to_string(), CValue::String(witness_id)),
            (
                "relation_type".to_string(),
                CValue::String("has_witness".to_string()),
            ),
            (
                "subject".to_string(),
                CValue::String(object.object_id.clone()),
            ),
        ]);
        relations.push(
            sc::write_value(&mut *store, &relation)
                .map_err(|error| format!("GenError：关系值写入失败：{error}"))?,
        );
    }
    let window = format!(
        "window-{}-{}-{}",
        object.window[0], object.window[1], object.window[2]
    );
    let classified = CValue::Object(vec![
        (
            "object".to_string(),
            CValue::String(object.object_id.clone()),
        ),
        (
            "relation_type".to_string(),
            CValue::String("classified_as".to_string()),
        ),
        ("subject".to_string(), CValue::String(window)),
    ]);
    relations.push(
        sc::write_value(&mut *store, &classified)
            .map_err(|error| format!("GenError：关系值写入失败：{error}"))?,
    );
    Ok((witnesses, relations))
}

/// witness 身份：`wit-` + 前 16 位 hex（旧具名配方；输入含**真实新** `obj3-`）。
///
/// SHA256 的规范 JSON 输入**流式**写入哈希器：`raw_bars` 直接渲染既有数组根，
/// 不先 materialize 成整份字符串/`Value`。
pub(super) fn witness_identity(
    store: &mut (impl CodecRead + ?Sized),
    object_id: &str,
    slot: usize,
    mi: usize,
    raw_bars: &Built,
) -> Result<String, String> {
    if object_id.contains('"') || object_id.contains('\\') {
        return Err(format!(
            "GenError：对象身份 `{object_id}` 含需转义字符，拒绝拼入规范输入"
        ));
    }
    let mut hasher = Sha256::new();
    hasher.update(format!(r#"{{"mi":{mi},"object_id":"{object_id}","raw_bars":"#).as_bytes());
    {
        let mut writer = DigestWriter(&mut hasher);
        store
            .render_canonical(&raw_bars.content_ref(), &mut writer)
            .map_err(|error| format!("GenError：见证输入流式渲染失败：{error}"))?;
    }
    hasher.update(format!(r#","slot":{slot}}}"#).as_bytes());
    let digest = hasher.finalize();
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!("wit-{}", &hex[..16]))
}

/// `std::io::Write` → SHA256 的流式适配（`render_canonical` 的 sink）。
struct DigestWriter<'a>(&'a mut Sha256);

impl std::io::Write for DigestWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.update(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// 事件节点：`(source_coord, revision)` memo + 逐字段写节点（小原件，非大数组）。
fn event_node(
    memo: &mut BTreeMap<(i64, i64), NodeDigest>,
    store: &mut dyn NodeStore,
    event: JsonView<'_>,
) -> Result<NodeDigest, String> {
    let coord = bj::coord(event)?;
    let revision = bj::field(event, "revision")
        .as_i64()
        .ok_or("StorageUnavailable：raw.revision 不是 i64")?;
    if let Some(node) = memo.get(&(coord, revision)) {
        return Ok(*node);
    }
    let built = sc::write_json_view(&mut *store, event)
        .map_err(|error| format!("GenError：原始事件节点写入失败：{error}"))?;
    memo.insert((coord, revision), built.digest);
    Ok(built.digest)
}

/// 生效账事件 → `Bar`（与旧生成段同一取法：OHLC 档案取 open/high/low/close）。
fn bar_of(event: JsonView<'_>) -> Result<Bar, String> {
    let price = |key: &str| -> Result<i64, String> {
        parse_canonical_i64(
            bj::field(event, key)
                .as_str()
                .ok_or_else(|| format!("StorageUnavailable：OHLC价 `{key}` 缺失"))?,
            key,
        )
    };
    Ok(Bar {
        source_index: usize::try_from(bj::coord(event)?)
            .map_err(|_| "StorageUnavailable：源坐标不能无损转换为 source_index".to_string())?,
        timestamp: parse_canonical_i64(bj::field(event, "ts").as_str().unwrap_or(""), "timestamp")?,
        open: price("open")?,
        high: price("high")?,
        low: price("low")?,
        close: price("close")?,
        volume: 1.0,
        untradable: false,
    })
}

/// 生效账事件 → producer 的八字段源引用登记（字段只从原行取，不另判）。
fn raw_ref_spec(event: JsonView<'_>) -> Result<RawRefSpec, String> {
    let text = |key: &str| -> Result<String, String> {
        bj::field(event, key)
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| format!("StorageUnavailable：raw.{key} 不是字符串"))
    };
    Ok(RawRefSpec {
        identity_key: text("identity_key")?,
        payload_hash: text("payload_hash")?,
        receipt_id: text("receipt_id")?,
        event_id: text("event_id")?,
        input_revision: text("input_revision")?,
        revision: bj::field(event, "revision")
            .as_i64()
            .ok_or("StorageUnavailable：raw.revision 不是 i64")?,
        seq: bj::field(event, "seq")
            .as_i64()
            .ok_or("StorageUnavailable：raw.seq 不是 i64")?,
        source_coord: text("source_coord")?,
    })
}

fn group_views(state: &IncrFactsState, start: usize) -> Result<[GroupEvidenceView<'_>; 3], String> {
    let left = state
        .shared_group(start)
        .map_err(|error| format!("GenError：共享组证据不可用：{}", error.reason))?
        .ok_or_else(|| format!("GenError：组 {start} 缺共享证据"))?;
    let mid = state
        .shared_group(start + 1)
        .map_err(|error| format!("GenError：共享组证据不可用：{}", error.reason))?
        .ok_or_else(|| format!("GenError：组 {} 缺共享证据", start + 1))?;
    let right = state
        .shared_group(start + 2)
        .map_err(|error| format!("GenError：共享组证据不可用：{}", error.reason))?
        .ok_or_else(|| format!("GenError：组 {} 缺共享证据", start + 2))?;
    Ok([left, mid, right])
}

/// TB-02-B 对象的 kind：从记录读回（不重算判据，不猜默认）。
fn typed_kind(store: &mut dyn NodeStore, object: &TypedObject) -> Result<String, String> {
    let root = object.record.content_ref();
    let kind = path_of(store, &root, &[step_field("kind")])?;
    if kind.kind != Kind::String {
        return Err(format!(
            "GenError：对象 {} 记录 kind 不是字符串（{:?}）",
            object.object_id, kind.kind
        ));
    }
    let text = render_text(store, &kind)?;
    parse_small_json(&text, "记录 kind")?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("GenError：对象 {} 记录 kind 不是文本", object.object_id))
}

/// 规范 JSON 数字节点（精确整数）。
fn number(value: i64) -> Result<CValue, String> {
    let token = NumberToken::parse(&value.to_string())
        .map_err(|error| format!("GenError：整数 token 非法：{error}"))?;
    Ok(CValue::Number(token))
}

/// `serde_json::Value` → 内容值：数字只接受精确整数 token（旧行没有浮点；出现即拒绝，不静默改写）。
fn json_to_cvalue(value: &Value) -> Result<CValue, String> {
    Ok(match value {
        Value::Null => CValue::Null,
        Value::Bool(flag) => CValue::Bool(*flag),
        Value::String(text) => CValue::String(text.clone()),
        Value::Number(number) => {
            let token = number
                .as_i64()
                .map(|value| value.to_string())
                .or_else(|| number.as_u64().map(|value| value.to_string()))
                .ok_or_else(|| {
                    format!(
                        "GenError：原始行出现非整数 JSON 数字 `{number}`（不把 token 静默改写）"
                    )
                })?;
            CValue::Number(
                NumberToken::parse(&token)
                    .map_err(|error| format!("GenError：数字 token 非法：{error}"))?,
            )
        }
        Value::Array(items) => CValue::Array(
            items
                .iter()
                .map(json_to_cvalue)
                .collect::<Result<Vec<CValue>, String>>()?,
        ),
        Value::Object(fields) => CValue::Object(
            fields
                .iter()
                .map(|(key, value)| Ok((key.clone(), json_to_cvalue(value)?)))
                .collect::<Result<Vec<(String, CValue)>, String>>()?,
        ),
    })
}

fn write_json(store: &mut dyn NodeStore, value: &Value) -> Result<Built, String> {
    sc::write_value(&mut *store, &json_to_cvalue(value)?)
        .map_err(|error| format!("GenError：值写入失败：{error}"))
}

fn producer_error(error: ProducerError) -> String {
    format!("GenError：{error}")
}

fn record_error(error: object_records::RecordError) -> String {
    format!("GenError：{error}")
}

fn codec_error(error: sc::CodecError) -> String {
    format!("GenError：{error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{
        accept_ohlc, init_test_db, located_ohlc_input, tb02b_oracle_rows, TestFiles,
    };
    use crate::{
        canonical_bytes_of_value, cmd_advance, compute_diff, delta_storage, load_batch_bytes,
        meta_get_opt, open_db, read_effective_events, read_stored_objects,
        verified_root_raw_events, DEFAULT_WRITER_EPOCH,
    };
    use newchan_rust::s_content::{render_canonical, MemoryNodes};
    use serde_json::json;

    /// 一代 oracle：封存批次原件（完整生成态）+ 同一读集的真实两账。
    struct OracleGeneration {
        generation: i64,
        batch: Value,
        all_events: Vec<Value>,
        effective: Vec<Value>,
    }

    /// 真实 v2 生产入口的临时库：逐代 `accept` + `advance`，再读封存批次。
    struct OracleDb {
        files: TestFiles,
    }

    impl OracleDb {
        fn new() -> Self {
            let files = TestFiles::new();
            init_test_db(&files);
            OracleDb { files }
        }

        fn accept(&self, rows: &[(&str, i64, i64, i64, i64, i64)]) {
            accept_ohlc(&self.files, &located_ohlc_input(rows));
        }

        /// 本次 advance；返回**推进前**的 `last_advance_frontier`
        /// （旧目录轴 `previous_frontier` 的同一口径）。
        fn advance(&self) -> i64 {
            let conn = open_db(&self.files.db()).expect("打开 oracle 库");
            let previous_frontier = meta_get_opt(&conn, "last_advance_frontier")
                .expect("读上一代前沿")
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(-1);
            drop(conn);
            cmd_advance(&self.files.db(), DEFAULT_WRITER_EPOCH).expect("oracle advance");
            previous_frontier
        }

        /// 读当前已提交一代的完整 oracle 生成态（封存批次原件）。
        fn read(&self) -> OracleGeneration {
            let conn = open_db(&self.files.db()).expect("打开 oracle 库");
            let all_events = verified_root_raw_events(&conn).expect("根审计读全修订史");
            let effective = read_effective_events(&all_events).expect("有效账");
            let generation: i64 = meta_get_opt(&conn, "generation")
                .expect("读 generation")
                .expect("已提交代")
                .parse()
                .expect("generation 是整数");
            let batch_id: String = conn
                .query_row(
                    "SELECT index_frontier FROM structure_deltas ORDER BY generation DESC LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .expect("读本代批号");
            let bytes = load_batch_bytes(&conn, &batch_id).expect("读封存批次");
            let batch: Value = serde_json::from_slice(&bytes).expect("封存批次是规范 JSON");
            OracleGeneration {
                generation,
                batch,
                all_events,
                effective,
            }
        }

        fn lane_key(&self, label: &str) -> String {
            format!("v3-generation-oracle/{label}")
        }
    }

    /// 我方同核生成草案（未 finish 目录轴；真实 withdrawals/replaces 需要本代对象身份）。
    fn generate_draft<'b>(
        store: &mut MemoryNodes,
        runtime: &mut GenerationRuntime,
        oracle: &'b OracleGeneration,
        structure_cut: &'b str,
        prior: PriorState<'b>,
    ) -> Generated<'b> {
        let input = GenerationInput {
            all_events: (&oracle.all_events).into(),
            effective: (&oracle.effective).into(),
            generation: oracle.generation,
            structure_cut,
            input_frontier: oracle.batch["input_frontier"]
                .as_i64()
                .expect("oracle 前沿"),
            profile_id: oracle.batch["profile_id"].as_str().expect("oracle 档案号"),
            profile_hash: oracle.batch["profile_hash"]
                .as_str()
                .expect("oracle 档案哈希"),
            rule_revision: oracle.batch["rule_revision"]
                .as_str()
                .expect("oracle 规则修订"),
            catalog_revision: oracle.batch["catalog_revision"]
                .as_str()
                .expect("oracle 目录修订"),
            scope: &oracle.batch["scope"],
            semantic_commit_ns: None,
            prior,
        };
        runtime
            .generate(&mut *store, &input)
            .unwrap_or_else(|error| panic!("S3 生成失败：{error}"))
    }

    /// 我方同核生成 + 第二段目录轴 finish（参数与旧段同一口径）。
    fn generate_mine<'b>(
        store: &mut MemoryNodes,
        runtime: &mut GenerationRuntime,
        oracle: &'b OracleGeneration,
        structure_cut: &'b str,
        previous_frontier: i64,
        withdrawals: &[Value],
        replaces: &[Value],
        prior: PriorState<'b>,
    ) -> Generated<'b> {
        let mut generated = generate_draft(&mut *store, runtime, oracle, structure_cut, prior);
        generated
            .finish_axes(&mut *store, previous_frontier, withdrawals, replaces)
            .unwrap_or_else(|error| panic!("S3 目录轴 finish 失败：{error}"));
        generated
    }

    fn canonical_text(store: &mut MemoryNodes, reference: &ContentRef) -> String {
        let mut bytes = Vec::new();
        render_canonical(&mut *store, reference, &mut bytes).expect("规范渲染");
        String::from_utf8(bytes).expect("规范 JSON 是 UTF-8")
    }

    fn state_value(store: &mut MemoryNodes, generated: &Generated<'_>) -> (String, Value) {
        let text = canonical_text(store, &generated.generated_state);
        let value: Value = serde_json::from_str(&text).expect("生成态是规范 JSON");
        (text, value)
    }

    /// 全文身份映射：`obj-`/`wit-` 两族按旧 sealed 批次顺序一一对应，映射只换身份文本，
    /// 不碰任何内容字段；`payload_json` 等文本内部的引用也一并替换（共享边）。
    fn map_ids(value: &Value, map: &[(String, String)]) -> Value {
        match value {
            Value::String(text) => {
                let mut out = text.clone();
                for (old, new) in map {
                    out = out.replace(old, new);
                }
                Value::String(out)
            }
            Value::Array(items) => {
                Value::Array(items.iter().map(|item| map_ids(item, map)).collect())
            }
            Value::Object(fields) => Value::Object(
                fields
                    .iter()
                    .map(|(key, item)| (key.clone(), map_ids(item, map)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    fn mapped_id<'a>(map: &'a [(String, String)], old: &str) -> Option<&'a str> {
        map.iter()
            .find(|(candidate, _)| candidate == old)
            .map(|(_, new)| new.as_str())
    }

    /// 旧 CC-006 见证配方（`wit-` + 前 16 位 hex）的独立复算：规范 JSON 键序由
    /// `canonical_bytes_of_value` 统一，输入 = `{mi, object_id, raw_bars, slot}`。
    fn witness_recipe_id(object_id: &str, slot: usize, mi: i64, raw_bars: &Value) -> String {
        let canonical = canonical_bytes_of_value(&json!({
            "mi": mi,
            "object_id": object_id,
            "raw_bars": raw_bars,
            "slot": slot,
        }));
        let digest = Sha256::digest(&canonical);
        let hex = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        format!("wit-{}", &hex[..16])
    }

    /// 同一有效账的 merged bar source_index 序列（共享解析器、全量入口；只用于定位见证 mi）。
    fn merged_source_indexes(oracle: &OracleGeneration) -> Vec<i64> {
        let config = ThetaConfig::default();
        let bars: Vec<Bar> = oracle
            .effective
            .iter()
            .map(|event| bar_of(event.into()).expect("有效账 bar"))
            .collect();
        let (layer, _) = parser::parse_layer_with_inclusion_facts(&bars, &config);
        layer
            .merged_bars
            .iter()
            .map(|bar| bar.source_index as i64)
            .collect()
    }

    /// 逐字段对拍：15 项标量/数组全部来自**封存批次原件**，只有旧 `obj-`/`wit-` 身份
    /// 按顺序映射到本域真实新身份；最后再核映射后的规范字节逐字节相等。
    fn assert_state_matches_oracle(
        label: &str,
        store: &mut MemoryNodes,
        generated: &Generated<'_>,
        oracle: &OracleGeneration,
        inherited: &[(String, String)],
    ) -> (Value, Vec<(String, String)>) {
        let (mine_text, mine) = state_value(store, generated);
        let oracle_objects = oracle.batch["objects"].as_array().expect("oracle objects");
        let oracle_witnesses = oracle.batch["witnesses"]
            .as_array()
            .expect("oracle witnesses");
        let merged_indexes = merged_source_indexes(oracle);
        assert_eq!(
            generated.objects.len(),
            oracle_objects.len(),
            "{label}: objects 条数"
        );
        assert_eq!(
            generated.witnesses.len(),
            oracle_witnesses.len(),
            "{label}: witnesses 条数"
        );
        // 身份映射 = 前代已核映射 + 本代 `obj-`/`wit-` 两族按 sealed 顺序一位一映射。
        let mut map: Vec<(String, String)> = inherited.to_vec();
        for (index, (old, new)) in oracle_objects.iter().zip(&generated.objects).enumerate() {
            let old_id = old["object_id"]
                .as_str()
                .unwrap_or_else(|| panic!("{label}: oracle 对象{index} 缺身份"));
            assert!(
                old_id.starts_with("obj-"),
                "{label}: oracle 对象{index} 身份配方 `{old_id}`"
            );
            assert_eq!(
                old["kind"].as_str(),
                Some(new.kind()),
                "{label}: 对象{index} kind"
            );
            assert!(
                new.object_id().starts_with("obj3-"),
                "{label}: 我方对象{index} 身份配方 `{}`",
                new.object_id()
            );
            map.push((old_id.to_string(), new.object_id().to_string()));
        }
        for (index, (old, new)) in oracle_witnesses
            .iter()
            .zip(&generated.witnesses)
            .enumerate()
        {
            let old_object = old["object_id"]
                .as_str()
                .unwrap_or_else(|| panic!("{label}: oracle 见证{index} 缺 object_id"));
            assert_eq!(
                mapped_id(&map, old_object),
                Some(new.object_id.as_str()),
                "{label}: 见证{index} object_id 映射"
            );
            assert_eq!(
                old["slot"].as_i64(),
                Some(new.slot as i64),
                "{label}: 见证{index} 槽位"
            );
            let old_id = old["witness_id"]
                .as_str()
                .unwrap_or_else(|| panic!("{label}: oracle 见证{index} 缺身份"));
            assert!(
                old_id.starts_with("wit-") && new.witness_id.starts_with("wit-"),
                "{label}: 见证{index} 身份族（旧 `{old_id}` / 新 `{}`）",
                new.witness_id
            );
            // 配方锁：旧配方在测试内独立复算；本域必须是同一配方、只把输入换成真实新 `obj3-`。
            let owner = oracle_objects
                .iter()
                .find(|object| object["object_id"] == old_object)
                .unwrap_or_else(|| panic!("{label}: 见证{index} 的对象不在 objects 里"));
            let window = [
                owner["window_start"].as_i64(),
                owner["window_mid"].as_i64(),
                owner["window_end"].as_i64(),
            ];
            let merged_index = old["merged_source_index"]
                .as_i64()
                .unwrap_or_else(|| panic!("{label}: 见证{index} 缺 merged_source_index"));
            let slot = usize::try_from(
                old["slot"]
                    .as_u64()
                    .unwrap_or_else(|| panic!("{label}: 见证{index} 缺 slot")),
            )
            .expect("槽位在 usize 内");
            assert_eq!(
                window[slot],
                Some(merged_index),
                "{label}: 见证{index} 槽位与窗口位次不一致"
            );
            let mut positions = merged_indexes
                .iter()
                .enumerate()
                .filter(|(_, value)| **value == merged_index)
                .map(|(position, _)| position);
            let mi = i64::try_from(positions.next().unwrap_or_else(|| {
                panic!("{label}: 见证{index} merged_source_index 不在 merged 序列")
            }))
            .expect("merged 位次在 i64 内");
            assert!(
                positions.next().is_none(),
                "{label}: 见证{index} merged source_index 重复"
            );
            let mine_witness = &mine["witnesses"][index];
            assert_eq!(
                witness_recipe_id(old_object, slot, mi, &old["raw_bars"]),
                old_id,
                "{label}: 见证{index} 旧配方独立复算"
            );
            assert_eq!(
                witness_recipe_id(new.object_id.as_str(), slot, mi, &mine_witness["raw_bars"]),
                new.witness_id,
                "{label}: 见证{index} 同配方 + 新 obj3 输入"
            );
            assert_eq!(
                old["raw_bars"], mine_witness["raw_bars"],
                "{label}: 见证{index} 完整 raw_bars"
            );
            map.push((old_id.to_string(), new.witness_id.clone()));
        }
        let mapped = map_ids(&oracle.batch, &map);
        const FIELDS: [&str; 15] = [
            "generation",
            "structure_cut",
            "input_frontier",
            "catalog_revision",
            "profile_id",
            "profile_hash",
            "rule_revision",
            "raw_events",
            "effective_events",
            "objects",
            "witnesses",
            "relations",
            "observations",
            "scope",
            "catalog_axes",
        ];
        for field in FIELDS {
            let want = mapped
                .get(field)
                .unwrap_or_else(|| panic!("{label}: oracle 缺字段 {field}"));
            let got = mine
                .get(field)
                .unwrap_or_else(|| panic!("{label}: 我方缺字段 {field}"));
            if field == "catalog_axes" && want != got {
                for axis in [
                    "CC-001", "CC-002", "CC-003", "CC-004", "CC-005", "CC-006", "CC-007", "CC-008",
                    "CC-009", "CC-010", "CC-054", "CC-055", "CC-056",
                ] {
                    if want[axis] != got[axis] {
                        eprintln!(
                            "{label}: 目录轴 {axis} 不等\n  want={}\n  got ={}",
                            want[axis], got[axis]
                        );
                    }
                }
            }
            assert_eq!(want, got, "{label}: 字段 {field} 逐项不等");
        }
        assert_eq!(
            String::from_utf8(canonical_bytes_of_value(&mapped)).expect("规范字节 UTF-8"),
            mine_text,
            "{label}: 映射后规范字节不等"
        );
        (mine, map)
    }

    /// 非空与关键共享边证据：不是只比计数，逐项相等已在上面的 15 字段里完成。
    fn assert_non_vacuous_evidence(label: &str, mine: &Value) {
        let objects = mine["objects"].as_array().expect("objects");
        let witnesses = mine["witnesses"].as_array().expect("witnesses");
        let observations = mine["observations"].as_array().expect("observations");
        let relations = mine["relations"].as_array().expect("relations");
        assert!(!objects.is_empty(), "{label}: objects 非空");
        assert!(!witnesses.is_empty(), "{label}: witnesses 非空");
        assert!(!observations.is_empty(), "{label}: observations 非空");
        assert!(!relations.is_empty(), "{label}: relations 非空");
        let knowledge = objects
            .iter()
            .find(|object| object["kind"] == "CC-054.knowledge_state")
            .unwrap_or_else(|| panic!("{label}: 缺 CC-054 知识态"));
        let payload: Value = serde_json::from_str(
            knowledge["payload_json"]
                .as_str()
                .unwrap_or_else(|| panic!("{label}: CC-054 载荷是文本")),
        )
        .expect("CC-054 载荷 JSON");
        let requests = payload["requests"]
            .as_array()
            .unwrap_or_else(|| panic!("{label}: 知识请求是数组"));
        assert!(!requests.is_empty(), "{label}: 知识请求非空");
        let ids: std::collections::BTreeSet<&str> = objects
            .iter()
            .map(|object| object["object_id"].as_str().expect("对象身份"))
            .collect();
        let mut shared_edges = 0usize;
        for request in requests {
            if let Some(subject) = request["subject_id"].as_str() {
                assert!(
                    ids.contains(subject),
                    "{label}: 请求 subject_id `{subject}` 不指向本代对象（共享边）"
                );
                shared_edges += 1;
            }
        }
        assert!(shared_edges > 0, "{label}: 至少一条请求共享边指向本代对象");
        let mut witness_bars = 0usize;
        for witness in witnesses {
            let bars = witness["raw_bars"]
                .as_array()
                .unwrap_or_else(|| panic!("{label}: 见证 raw_bars 是数组"));
            assert!(
                !bars.is_empty(),
                "{label}: 见证 raw_bars 不得为空（完整 raw_bars 已逐项对拍）"
            );
            for bar in bars {
                for key in [
                    "identity_key",
                    "payload_hash",
                    "receipt_id",
                    "event_id",
                    "input_revision",
                    "revision",
                    "seq",
                    "source_coord",
                ] {
                    assert!(
                        bar.get(key).is_some(),
                        "{label}: 见证 raw_bars 缺账字段 {key}（不是投影）"
                    );
                }
            }
            witness_bars += bars.len();
        }
        assert!(
            witness_bars >= witnesses.len(),
            "{label}: 见证 raw_bars 覆盖"
        );
    }

    /// 上一代活动 bi / version 的内部记录根（与 `content_bi::PriorState` 同一口径）。
    fn prior_roots<'g>(generated: &'g Generated<'_>) -> (Vec<&'g Built>, Option<&'g Built>) {
        let mut bis: Vec<&Built> = Vec::new();
        let mut version: Option<&Built> = None;
        for object in &generated.objects {
            if let ObjectBody::Typed(typed) = &object.body {
                match object.kind.as_str() {
                    "CC-010.bi" => bis.push(&typed.record),
                    "CC-056.version" => {
                        assert!(version.is_none(), "一代不得出现两个版本对象");
                        version = Some(&typed.record);
                    }
                    _ => {}
                }
            }
        }
        (bis, version)
    }

    #[test]
    fn oracle_prefix_parity_two_prefixes() {
        let rows = tb02b_oracle_rows();
        for prefix in [8usize, 14] {
            let oracle_db = OracleDb::new();
            oracle_db.accept(&rows[..prefix]);
            let previous_frontier = oracle_db.advance();
            let oracle = oracle_db.read();
            assert_eq!(oracle.generation, 1, "前缀 {prefix}: 首代");
            let mut store = MemoryNodes::new();
            let mut runtime = GenerationRuntime::new(
                oracle_db.lane_key(&format!("prefix-{prefix}")),
                crate::tb02a::PROFILE,
                oracle.batch["rule_revision"].as_str().expect("规则修订"),
            );
            let cut = format!("cut-{}", oracle.generation);
            let generated = generate_mine(
                &mut store,
                &mut runtime,
                &oracle,
                &cut,
                previous_frontier,
                &[],
                &[],
                PriorState {
                    bi_records: Vec::new(),
                    version_record: None,
                },
            );
            assert!(
                generated.catalog_axes.is_some(),
                "前缀 {prefix}: OHLC 必须产出真实目录轴"
            );
            let label = format!("前缀 {prefix}");
            let (mine, _) =
                assert_state_matches_oracle(&label, &mut store, &generated, &oracle, &[]);
            assert_non_vacuous_evidence(&label, &mine);
        }
    }

    #[test]
    fn oracle_historical_revision_parity() {
        let rows = tb02b_oracle_rows();
        let prefix = 10usize;
        let oracle_db = OracleDb::new();
        oracle_db.accept(&rows[..prefix]);
        // 历史修订：b2（coord 20）推 revision 2 并改价；旧 revision 留在全修订史里。
        let mut revised = rows[..prefix].to_vec();
        revised[2] = ("b2", 20, 11, 13, 9, 11);
        let mut input = located_ohlc_input(&revised);
        input["events"][2]["revision"] = json!("2");
        accept_ohlc(&oracle_db.files, &input);
        let previous_frontier = oracle_db.advance();
        let oracle = oracle_db.read();
        assert_eq!(oracle.generation, 1, "修订后首代");
        let b2_revisions: Vec<i64> = oracle
            .all_events
            .iter()
            .filter(|event| event["event_id"] == "b2")
            .map(|event| event["revision"].as_i64().expect("修订号"))
            .collect();
        assert_eq!(
            b2_revisions,
            vec![1, 2],
            "全修订史必须保留 b2 的两版（seq 序）"
        );
        assert!(
            oracle.all_events.len() > oracle.effective.len(),
            "历史修订：全修订史条数必须大于有效账条数"
        );
        let effective_b2 = oracle
            .effective
            .iter()
            .find(|event| event["event_id"] == "b2")
            .expect("有效账 b2");
        assert_eq!(
            effective_b2["revision"].as_i64(),
            Some(2),
            "有效账取最新修订"
        );
        let mut store = MemoryNodes::new();
        let mut runtime = GenerationRuntime::new(
            oracle_db.lane_key("historical-revision"),
            crate::tb02a::PROFILE,
            oracle.batch["rule_revision"].as_str().expect("规则修订"),
        );
        let cut = format!("cut-{}", oracle.generation);
        let generated = generate_mine(
            &mut store,
            &mut runtime,
            &oracle,
            &cut,
            previous_frontier,
            &[],
            &[],
            PriorState {
                bi_records: Vec::new(),
                version_record: None,
            },
        );
        let (mine, _) =
            assert_state_matches_oracle("历史修订", &mut store, &generated, &oracle, &[]);
        assert_non_vacuous_evidence("历史修订", &mine);
    }

    #[test]
    fn oracle_incremental_second_generation_parity() {
        let rows = tb02b_oracle_rows();
        let oracle_db = OracleDb::new();
        oracle_db.accept(&rows[..8]);
        let first_frontier = oracle_db.advance();
        let oracle_first = oracle_db.read();
        assert_eq!(oracle_first.generation, 1);
        // 第二次 advance 之前抓旧状态身份读集：与旧目录轴调用同一读面。
        let conn = open_db(&oracle_db.files.db()).expect("打开 oracle 库");
        let mode = delta_storage::read_mode(&conn).expect("读 Delta 域模式");
        let stored_before = read_stored_objects(&conn).expect("读已存身份");
        drop(conn);
        oracle_db.accept(&rows[..14]);
        let second_frontier = oracle_db.advance();
        let oracle_second = oracle_db.read();
        assert_eq!(oracle_second.generation, 2);

        let mut store = MemoryNodes::new();
        let mut runtime = GenerationRuntime::new(
            oracle_db.lane_key("incremental"),
            crate::tb02a::PROFILE,
            oracle_second.batch["rule_revision"]
                .as_str()
                .expect("规则修订"),
        );
        let cut_first = format!("cut-{}", oracle_first.generation);
        let first = generate_mine(
            &mut store,
            &mut runtime,
            &oracle_first,
            &cut_first,
            first_frontier,
            &[],
            &[],
            PriorState {
                bi_records: Vec::new(),
                version_record: None,
            },
        );
        let (mine_first, map_first) =
            assert_state_matches_oracle("增量 gen1", &mut store, &first, &oracle_first, &[]);
        assert_non_vacuous_evidence("增量 gen1", &mine_first);

        // 上一代活动 bi/version 根来自本运行态自己的 gen1 产物（同核累计，不重造）。
        let (bis, version) = prior_roots(&first);
        // gen2 草案：真实 diff 需要本代对象身份，先不 finish 目录轴。
        let cut_second = format!("cut-{}", oracle_second.generation);
        let mut second = generate_draft(
            &mut store,
            &mut runtime,
            &oracle_second,
            &cut_second,
            PriorState {
                bi_records: bis,
                version_record: version,
            },
        );
        // 真实 withdrawals/replaces：在推进前已存身份 + 本代封存对象上跑同一生产 diff，
        // 再按 approved 身份映射换到本域 `obj3-` 空间（上代映射 + 本代按位对齐）。
        let objects: Vec<Value> = oracle_second.batch["objects"]
            .as_array()
            .expect("gen2 objects")
            .clone();
        let (_, old_withdrawals, old_replaces) =
            compute_diff(&objects, &stored_before, mode).expect("生产 diff");
        let mut diff_map = map_first.clone();
        for (old, new) in objects.iter().zip(&second.objects) {
            diff_map.push((
                old["object_id"].as_str().expect("oracle 身份").to_string(),
                new.object_id().to_string(),
            ));
        }
        let withdrawals = map_ids(&Value::Array(old_withdrawals), &diff_map);
        let replaces = map_ids(&Value::Array(old_replaces), &diff_map);
        for mapped in [&withdrawals, &replaces] {
            assert!(
                !String::from_utf8(canonical_bytes_of_value(mapped))
                    .expect("规范字节 UTF-8")
                    .contains("obj-"),
                "增量 gen2: 发布期参数仍有未映射的旧身份"
            );
        }
        second
            .finish_axes(
                &mut store,
                second_frontier,
                withdrawals.as_array().expect("withdrawals"),
                replaces.as_array().expect("replaces"),
            )
            .expect("S3 目录轴 finish");
        let (mine_second, _) = assert_state_matches_oracle(
            "增量 gen2",
            &mut store,
            &second,
            &oracle_second,
            &map_first,
        );
        assert_non_vacuous_evidence("增量 gen2", &mine_second);
        // 跨代复用不得是假空：gen2 的 raw 账根必须覆盖全部 14 根，objects 严格增长。
        assert_eq!(
            mine_second["raw_events"]
                .as_array()
                .expect("raw_events")
                .len(),
            14,
            "增量 gen2: raw_events 覆盖全前缀"
        );
        assert!(
            mine_second["objects"].as_array().expect("objects").len()
                > mine_first["objects"].as_array().expect("objects").len(),
            "增量 gen2: 对象集必须严格增长"
        );
    }
}
