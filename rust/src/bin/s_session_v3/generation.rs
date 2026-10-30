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
//! - **规则域**：历史 A/B 域保留原构造面；`V3_STRUCTURE_RULE_REVISION` 从同次 parser 事实与
//!   已核 prior 接 C/D/E/F，恢复游标先于唯一一次 parser。目录跟实际规则域选择，未知域拒绝。
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
//! CC-005 请求的等待原因复用 `content_producer::group_reasons`（#1488 经
//! `group_reasons_admitted` 先准入容量，原因同序同值）。
//! #1405：同价标签只作证据，极值身份为根集合，多根窗口照常发布。

#![allow(dead_code)]

use super::audit_store::{CodecRead, CodecStore};
use super::business_json::{self as bj, JsonOwner, RawEventsView, TextValue};
use super::fixed_radix::FixedRadix;
use super::{content_structure, object_records};
use crate::content_bi::{BiBuildInputView, BiLedgerView, ContentBi, PriorState};
use crate::content_producer::{
    array_from_sequence, group_reasons_admitted, raw_prefix_coords, Cc004Producer, Cc006Object,
    Cc007Object, FractalDevelopment, ParserFractalShape, PreparedRawHistory, ProducerError,
    RawRefSpec, TypedObject, CC004_STEP_KIND, CC005_GROUP_KIND, CC006_SHAPE_KIND,
    CC007_DEVELOPMENT_BATCH_KIND, CC007_FRACTAL_KIND, CC054_KNOWLEDGE_KIND,
};
use crate::s_fact_cache;
use newchan_rust::s_content::path::PathStep;
use newchan_rust::s_content::{
    self as sc, Built, ContentRef, Kind, NodeDigest, NodeStore, NumberToken, SequenceRef,
    Value as CValue,
};
#[allow(
    unused_imports,
    reason = "#1488 再合 #1608：8组合实测无调用方，保留不删"
)]
use newchan_rust::session_protocol::parse_canonical_i64;
use newchan_rust::theta_v0::classifier::local_shape::{classify_local_shape, Cc006LocalShape};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser;
use newchan_rust::theta_v0::parser::inclusion::{GroupEvidenceView, IncrFactsState};
use newchan_rust::theta_v0::types::Bar;
use sc::capture_allocation::audit_json::JsonView;
use sc::capture_allocation::native_owned::NativeValue;
use sc::{CodecVec, ValueView as VV};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// 同一已核活跃发布索引选择的 C/D/E/F 前代根；空值仅用于首代/旧 A/B 域。
#[derive(Default)]
pub(super) struct StructurePriors<'a> {
    pub segment_records: &'a [Built],
    pub segment_version: Option<&'a Built>,
    pub ef_records: &'a [Built],
    /// #1565：上一代已核活动 `CC-004.undetermined_raw` 记录根（与 bi/version 同一口径，只读
    /// payload 取 `pending_since_cut`）；首代/方向已建立前一代为空。
    pub w_undetermined: &'a [Built],
}

/// 生成输入：全部取调用方**同一读集**的真实值；本文不读库、不重排、不补默认。
pub(super) struct GenerationInput<'a, 'p> {
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
    pub structure_prior: StructurePriors<'p>,
}

/// #1488：raw/effective 不可由调用者另给；仅含当前元数据与已核 prior。
pub(super) struct BoundGenerationInput<'a, 'p> {
    pub generation: i64,
    pub structure_cut: &'a str,
    pub input_frontier: i64,
    pub profile_id: &'a str,
    pub profile_hash: &'a str,
    pub rule_revision: &'a str,
    pub catalog_revision: &'a str,
    pub scope: &'a Value,
    pub semantic_commit_ns: Option<&'a str>,
    pub prior: PriorState<'a>,
    pub structure_prior: StructurePriors<'p>,
}

/// 一条已构造对象：身份 + 生成态记录 + 可复用的构造结果。
///
/// 顺序（生成态 `objects` 数组）＝旧 sealed 批次：CC-006 形态（窗序）→ 已按
/// `(kind, fact_key, object_id)` 排序的泛型事实 → TB-02-B 对象（线序）→ 新结构域的 C/D/E/F。
pub(super) struct ProducedObject {
    kind: NativeValue<String>,
    fact_key: Option<NativeValue<String>>,
    object_id: NativeValue<String>,
    generated: Built,
    body: ObjectBody,
}

/// Native 的资格包装不改变既有 Cc006Object 的布局或公开内容。
enum ShapeBuild {
    Ordinary(Cc006Object),
    Native(ParserFractalShape),
}

impl ShapeBuild {
    fn object(&self) -> &Cc006Object {
        match self {
            Self::Ordinary(object) => object,
            Self::Native(shape) => shape.object(),
        }
    }

    fn into_object(self) -> Cc006Object {
        match self {
            Self::Ordinary(object) => object,
            Self::Native(shape) => shape.into_object(),
        }
    }
}

enum ObjectBody {
    Typed(TypedObject),
    Structure(crate::content_producer::NativeStructureObject),
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
        self.fact_key.as_ref().map(|v| v.as_str())
    }

    /// 生成态记录值根（原 sealed `generated_state.objects[i]` 的逻辑字段）。
    pub(super) fn generated(&self) -> &Built {
        &self.generated
    }

    #[cfg(test)]
    pub(super) fn set_generated_for_test(&mut self, generated: Built) {
        self.generated = generated;
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

    pub(super) fn wire_template_view(
        &self,
        store: &mut (impl crate::v3::audit_store::CodecStore + ?Sized),
        life: object_records::LifeView<'_>,
        current_generation: i64,
    ) -> Result<Built, object_records::RecordError> {
        object_records::wire_template_view(store, self.borrowed(), life, current_generation)
    }

    /// 内部构造记录借用视图：根接线在同一批产物内直接消费（`content_bi::PriorState` 的
    /// 13 字段 private 根），不再由调用方重复构造。
    pub(super) fn borrowed(&self) -> object_records::BorrowedObject<'_> {
        match &self.body {
            ObjectBody::Typed(object) => object_records::BorrowedObject::Typed(object),
            ObjectBody::Structure(object) => object_records::BorrowedObject::Typed(object),
            ObjectBody::Fractal(object) => object_records::BorrowedObject::Fractal(object),
            ObjectBody::Shape(object) => object_records::BorrowedObject::Shape(object),
        }
    }

    /// 对象的公开载荷值根（CC-006 旧配方没有 payload 字段，故为 `None`）。
    fn payload(&self) -> Option<&Built> {
        match &self.body {
            ObjectBody::Typed(object) => Some(&object.payload),
            ObjectBody::Structure(object) => Some(&object.payload),
            ObjectBody::Fractal(object) => Some(&object.payload),
            ObjectBody::Shape(_) => None,
        }
    }
}

/// 一条 CC-006 见证（生成态字段与旧配方同名同型；身份由真实新 `obj3-` 派生）。
pub(super) struct WitnessRecord {
    pub witness_id: NativeValue<String>,
    pub object_id: NativeValue<String>,
    pub slot: usize,
    pub built: Built,
}

/// 跨代共享的两条原始账数组根。
pub(super) struct RawRoots {
    pub raw_events: NativeValue<ContentRef>,
    pub effective_events: NativeValue<ContentRef>,
}

/// 生成态标量（`emit_state` 的输入；OHLC 两段 finish 时复用，不重造对象）。
struct StateMeta {
    generation: i64,
    structure_cut: TextValue,
    input_frontier: i64,
    catalog_revision: TextValue,
    profile_id: TextValue,
    profile_hash: TextValue,
    rule_revision: TextValue,
    scope: Built,
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
    pub objects: CodecVec<ProducedObject>,
    /// 顺序 = 形态序 × 槽位 0..3（旧配方逐字同序）。
    pub witnesses: CodecVec<WitnessRecord>,
    /// 生成态 `relations` 值边：has_witness/classified_as + 新笔关系。
    /// `supersedes`/`replaces` 属发布期，由 root 追加，不在本集合内。
    pub relations: CodecVec<Built>,
    /// 生成态 `observations` 值边（已按旧配方去重）。
    pub observations: CodecVec<Built>,
    pub raw: RawRoots,
    /// OHLC 的真实目录轴（`finish_axes` 之前恒 `None`，绝不假空）。
    pub catalog_axes: Option<NativeValue<ContentRef>>,
    /// 14 字段草案根；OHLC 时必须经 [`Generated::finish_axes`] 才是最终根。
    pub state_without_axes: NativeValue<ContentRef>,
    /// 当前最终生成态根（OHLC 在 `finish_axes` 成功后更新）。
    pub generated_state: NativeValue<ContentRef>,
    /// 生成配置的记录版本戳；只是运行元数据，不进生成态内容字节。
    pub policy_records: String,
    meta: StateMeta,
    roots: StateRoots,
    /// 本代知识请求线序（仅供目录轴最小投影消费，不进生成态对象）。
    requests: CodecVec<Built>,
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
    pub(super) fn finish_axes<'changes>(
        &mut self,
        store: &mut dyn NodeStore,
        previous_frontier: i64,
        withdrawals: impl Into<RawEventsView<'changes>>,
        replaces: impl Into<RawEventsView<'changes>>,
    ) -> Result<NativeValue<ContentRef>, String> {
        let withdrawals = withdrawals.into();
        let replaces = replaces.into();
        if !self.is_ohlc {
            return Err(
                "VersionUnsupported：目录轴只存在于 OHLC 档案；非 OHLC 生成态没有该字段，拒绝假空"
                    .to_string(),
            );
        }
        let mut rows = CodecVec::default();
        for object in &self.objects {
            let row = catalog_projection_entry(&mut *store, object, &self.requests)?;
            push_gen(&mut rows, row, store)?;
        }
        let rows = JsonOwners(&rows);
        let projection = bj::json_from_view_admitted(store, JsonView::BorrowedArray(&rows))?;
        // 与原目录轴函数同一判据；借入实际 owner，不擦成裸 Value 数组。
        let axes_owner = crate::tb02a_facts::catalog_axes_admitted(
            &*store,
            crate::tb02a_facts::CatalogAxisTable::for_rule_revision(&self.meta.rule_revision)
                .ok_or("VersionUnsupported：规则域没有已审目录轴表")?,
            &projection,
            self.all_events,
            previous_frontier,
            withdrawals,
            replaces,
        )?;
        // #1488：producer owner持续存活至同核整数检查和codec写出完成。
        let axes = write_json_view(&mut *store, axes_owner.view())
            .map_err(|error| format!("GenError：目录轴写入失败：{error}"))?;
        let reference = native_ref(store, &axes)?;
        let state = emit_state(store, &self.meta, &self.roots, Some(&reference))?;
        let returned =
            NativeValue::<ContentRef>::try_from_view((&state).into(), store.codec_work())
                .map_err(codec_error)?;
        self.catalog_axes = Some(reference);
        self.generated_state = state;
        Ok(returned)
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
    /// 本生成链的唯一配置来源；生产与诊断由调用方显式注入。
    theta_config: ThetaConfig,
    /// `(source_coord, revision)` → 已写事件节点摘要（原件不可变；见模块文档）。
    event_nodes: FixedRadix<(i64, i64), NodeDigest>,
    /// #1488：owner 绑定的 raw/parser pending 状态；不混用旧线程 Lane 的全指纹路径。
    delta_lane: Option<s_fact_cache::DeltaLane>,
    raw_events_seq: SequenceMemo,
    effective_events_seq: SequenceMemo,
    objects_seq: SequenceMemo,
    witnesses_seq: SequenceMemo,
    relations_seq: SequenceMemo,
    observations_seq: SequenceMemo,
    /// #1491 bounded-layout/1：单元批记录载荷 memo（bar 坐标 → 上一代载荷值根）。只是不重写
    /// 未变载荷的性能结构：脏 bar 一律按本代单元读数重建，memo 不充任准入/证据权威。
    development_batches: std::collections::BTreeMap<i64, Built>,
    /// #1491：上一代各分型（object_id）的单元坐标区间，供本代判定哪些 bar 的批记录变了。
    development_ranges: std::collections::HashMap<String, (i64, i64)>,
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
        for payload in self.development_batches.values() {
            f("generation.development_batches", payload.digest);
        }
    }

    /// `lane_key` 按 `s_fact_cache::lane_key` 的持久身份口径给出（库/会话/规则/档案/目录）。
    #[cfg(test)]
    pub(super) fn new(lane_key: String, profile_id: &str, rule_revision: &str) -> Self {
        Self::with_config(lane_key, profile_id, rule_revision, ThetaConfig::default())
    }

    /// 显式注入配置；生产由 Driver 给当前生效记录，测试给诊断夹具。
    pub(super) fn with_config(
        lane_key: String,
        profile_id: &str,
        rule_revision: &str,
        theta_config: ThetaConfig,
    ) -> Self {
        GenerationRuntime {
            lane_key,
            producer: Cc004Producer::new(profile_id, rule_revision),
            bi: ContentBi::new(),
            profile_id: profile_id.to_string(),
            rule_revision: rule_revision.to_string(),
            theta_config,
            event_nodes: FixedRadix::default(),
            delta_lane: None,
            raw_events_seq: SequenceMemo::default(),
            effective_events_seq: SequenceMemo::default(),
            objects_seq: SequenceMemo::default(),
            witnesses_seq: SequenceMemo::default(),
            relations_seq: SequenceMemo::default(),
            observations_seq: SequenceMemo::default(),
            development_batches: std::collections::BTreeMap::new(),
            development_ranges: std::collections::HashMap::new(),
        }
    }

    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn parser_config_for_test(&self) -> &ThetaConfig {
        &self.theta_config
    }

    pub(super) fn policy_records(&self) -> String {
        self.theta_config.record_labels()
    }

    /// 事务回滚 / 换库 / 换来源域之后必须调用：清掉同核运行态与全部跨代序列记忆。
    ///
    /// 旧线程车道仍由完整输入比较恢复；新 delta 车道没有这道全扫，必须与根一起丢弃。
    pub(super) fn discard_runtime_state(&mut self) {
        self.bi.discard_runtime_state(&mut self.producer);
        self.event_nodes = FixedRadix::default();
        self.delta_lane = None;
        self.raw_events_seq.discard();
        self.effective_events_seq.discard();
        self.objects_seq.discard();
        self.witnesses_seq.discard();
        self.relations_seq.discard();
        self.observations_seq.discard();
        self.development_batches.clear();
        self.development_ranges.clear();
    }

    /// #1488 原账片：消费 owner 当前资格，只构造新 raw 尾与生效账受影响后缀。
    /// 这是完整 generate_delta 的内部前段，不发布 Generated，不替代六阶段接线/验收。
    pub(super) fn advance_raw_bound(
        &mut self,
        store: &mut dyn NodeStore,
        bound: &super::audit_owner::BoundRawAdvance<'_>,
    ) -> Result<RawGenerationDelta, String> {
        let result = self.advance_raw_bound_inner(store, bound);
        if result.is_err() {
            bound.invalidate_after_consumer_failure();
            self.discard_runtime_state();
        }
        result
    }

    fn advance_raw_bound_inner(
        &mut self,
        store: &mut dyn NodeStore,
        bound: &super::audit_owner::BoundRawAdvance<'_>,
    ) -> Result<RawGenerationDelta, String> {
        bound.ensure_current()?;
        if self.delta_lane.is_none() {
            if self.raw_events_seq.bound || self.effective_events_seq.bound {
                return Err("GenError：旧生成态不可无 raw 资格转为 delta 车道，须丢弃".into());
            }
            self.delta_lane = Some(s_fact_cache::DeltaLane::new(bound.raw().budget())?);
        }
        if self.raw_events_seq.keys.as_slice().len() != bound.previous_prefix() {
            return Err("GenError：raw sequence 与 owner 前代 prefix 不一致".into());
        }
        let raw = bound.raw();
        let lane = self.delta_lane.as_mut().expect("刚建立");
        let changes = lane.advance_owner_bound(bound)?;
        let budget = raw.budget();
        let mut appended = sc::CodecVec::new_with_allocation(budget);
        let append_ordinals = bound.append_ordinals();
        if let Some(work) = store.codec_work() {
            appended
                .try_reserve_native(work, append_ordinals.len())
                .map_err(codec_error)?;
        } else {
            appended
                .try_reserve_exact(append_ordinals.len())
                .map_err(codec_error)?;
        }
        for ordinal in append_ordinals {
            let event = raw
                .get(ordinal)
                .ok_or_else(|| "GenError：raw append ordinal 缺失".to_string())?;
            appended
                .try_push(event_node(&mut self.event_nodes, store, event)?)
                .map_err(codec_error)?;
        }
        let prefix = changes
            .effective_dirty_from
            .unwrap_or(lane.rows().len())
            .min(lane.rows().len());
        let mut effective_suffix = sc::CodecVec::new_with_allocation(budget);
        if let Some(work) = store.codec_work() {
            effective_suffix
                .try_reserve_native(work, lane.rows().len() - prefix)
                .map_err(codec_error)?;
        } else {
            effective_suffix
                .try_reserve_exact(lane.rows().len() - prefix)
                .map_err(codec_error)?;
        }
        for row in &lane.rows()[prefix..] {
            // 所有赢家原件都已在 raw append 路径写过；旧尾只查节点，不再读/解码其 raw。
            // 初期没有 ordinal→node 索引时不能按(coord,revision)回读旧 raw 冒充 Δ。
            let node = if row.ordinal < bound.previous_prefix() {
                self.raw_events_seq
                    .keys
                    .as_slice()
                    .get(row.ordinal)
                    .copied()
            } else {
                appended.get(row.ordinal - bound.previous_prefix()).copied()
            }
            .ok_or_else(|| "GenError：生效 winner 对应的 raw 节点缺失".to_string())?;
            effective_suffix.try_push(node).map_err(codec_error)?;
        }
        let raw_events = self.raw_events_seq.sync_splice(
            store,
            SequenceSplice {
                expected_previous: self.raw_events_seq.seq,
                unchanged_prefix: bound.previous_prefix(),
                replacement_suffix: &appended,
            },
            "raw_events",
            budget,
        )?;
        let effective_events = self.effective_events_seq.sync_splice(
            store,
            SequenceSplice {
                expected_previous: self.effective_events_seq.seq,
                unchanged_prefix: prefix,
                replacement_suffix: &effective_suffix,
            },
            "effective_events",
            budget,
        )?;
        Ok(RawGenerationDelta {
            raw_events,
            effective_events,
            changes,
        })
    }
    #[cfg(all(test, feature = "s_allocation_json_testonly"))]
    pub(super) fn sequence_keys_bound_to_for_test(
        &self,
        budget: &sc::capture_allocation::OwnedBudget,
    ) -> bool {
        [
            &self.raw_events_seq,
            &self.effective_events_seq,
            &self.objects_seq,
            &self.witnesses_seq,
            &self.relations_seq,
            &self.observations_seq,
        ]
        .iter()
        .all(|memo| {
            memo.bound
                && memo
                    .allocation_budget()
                    .is_some_and(|b| b.same_ledger(budget))
        })
    }

    /// 一代同核生成（只写节点，不发布）。
    pub(super) fn generate<'b>(
        &mut self,
        store: &mut dyn NodeStore,
        input: &GenerationInput<'b, '_>,
    ) -> Result<Generated<'b>, String> {
        self.generate_with_allocation(store, input, None)
    }

    /// #1394：只给六份持久sequence keys传调用方原账，不声明其他生成分配闭合。
    pub(super) fn generate_with_allocation<'b>(
        &mut self,
        store: &mut dyn NodeStore,
        input: &GenerationInput<'b, '_>,
        budget: Option<&sc::capture_allocation::OwnedBudget>,
    ) -> Result<Generated<'b>, String> {
        let result = self.generate_impl(store, input, budget, None, None);
        if result.is_err() {
            self.discard_runtime_state();
        }
        result
    }

    /// 仅受控 NativeReadLease 分支可提供 token；外部调用方不能用 bool 开启资格。
    pub(super) fn generate_native_lease<'b>(
        &mut self,
        store: &mut dyn NodeStore,
        input: &GenerationInput<'b, '_>,
        native: &super::prepared::NativeGeneration,
    ) -> Result<Generated<'b>, String> {
        let result = self.generate_impl(store, input, None, Some(native), None);
        if result.is_err() {
            self.discard_runtime_state();
        }
        result
    }

    /// 完整生成消费同一 owner raw 资格；结果仍不是 C 的完整集合 seal 或提交回执。
    pub(super) fn generate_bound<'b>(
        &mut self,
        store: &mut dyn NodeStore,
        bound: &super::audit_owner::BoundRawAdvance<'b>,
        input: BoundGenerationInput<'b, '_>,
    ) -> Result<Generated<'b>, String> {
        let result = (|| {
            bound.ensure_current()?;
            self.check_generation_identity(input.profile_id, input.rule_revision)?;
            if bound.raw().budget().is_some() {
                return Err("prototype_audit_allocation_incomplete：完整 delta producer/bi/output 尚非 Owned；禁止 Plain fallback".into());
            }
            let raw_delta = self.advance_raw_bound_inner(store, bound)?;
            // 借用独立持有的车道，生成期间不能推进 parser。只有成功才归还运行态。
            let lane = self
                .delta_lane
                .take()
                .ok_or_else(|| "GenError：delta 车道缺失".to_string())?;
            let view = lane.effective_view(bound)?;
            let full_input = GenerationInput {
                all_events: bound.raw(),
                effective: RawEventsView::plain(&[]),
                generation: input.generation,
                structure_cut: input.structure_cut,
                input_frontier: input.input_frontier,
                profile_id: input.profile_id,
                profile_hash: input.profile_hash,
                rule_revision: input.rule_revision,
                catalog_revision: input.catalog_revision,
                scope: input.scope,
                semantic_commit_ns: input.semantic_commit_ns,
                prior: input.prior,
                structure_prior: input.structure_prior,
            };
            let generated =
                self.generate_impl(store, &full_input, None, None, Some((view, raw_delta)))?;
            bound.ensure_current()?;
            self.delta_lane = Some(lane);
            Ok(generated)
        })();
        if result.is_err() {
            bound.invalidate_after_consumer_failure();
            self.discard_runtime_state();
        }
        result
    }

    /// full/delta 共用原身份与档案前置判据；成功不分配，不触及 parser/store。
    fn check_generation_identity(
        &self,
        profile_id: &str,
        rule_revision: &str,
    ) -> Result<(), String> {
        if profile_id != self.profile_id || rule_revision != self.rule_revision {
            return Err(format!(
                "IdentityConflict：生成运行态的档案/规则修订（{}/{}）与本次输入（{}/{}）不一致",
                self.profile_id, self.rule_revision, profile_id, rule_revision
            ));
        }
        if profile_id != crate::tb02a::PROFILE {
            return Err(format!(
                "VersionUnsupported：S3 同核生成只接线 OHLC 档案 `{}`（CC-006 来源集合配方按档案分派，非 OHLC 支未接线；不静默降级）",
                crate::tb02a::PROFILE
            ));
        }
        Ok(())
    }

    fn generate_impl<'b>(
        &mut self,
        store: &mut dyn NodeStore,
        input: &GenerationInput<'b, '_>,
        budget: Option<&sc::capture_allocation::OwnedBudget>,
        native: Option<&super::prepared::NativeGeneration>,
        delta: Option<(s_fact_cache::EffectiveRawView<'_>, RawGenerationDelta)>,
    ) -> Result<Generated<'b>, String> {
        if budget.is_some() && store.codec_work().is_some() {
            return Err(
                "prototype_audit_allocation_incomplete：生成入口不能混用Owned keys账与Native父账"
                    .into(),
            );
        }
        if self.delta_lane.is_some() {
            return Err("GenError：delta 车道不能切回完整 generate 重判，须显式丢弃".into());
        }
        self.check_generation_identity(input.profile_id, input.rule_revision)?;
        let gen = input.generation;
        let cut = input.structure_cut;
        let frontier = input.input_frontier;
        // #1491 ADR 0037：CC-007 承载与 CC-054 请求次序按本车道冻结规则串的布局组件分派。
        // 穷举分派：布局枚举扩展时此处编译期报错，不静默落 layout/1（#1780 独审 L-1）。
        let bounded = match self.producer.layout().map_err(producer_error)? {
            super::layout::Layout::Sorted => false,
            super::layout::Layout::Bounded1 | super::layout::Layout::Bounded2 => true,
        };
        // ── 原始账：生效账 → coord 表 + Bar 序列 + 事件节点；全修订史 → 事件节点 ──
        let mut raw_by_coord: FixedRadix<i64, JsonView<'_>> = FixedRadix::default();
        let mut raw_bars: CodecVec<Bar> = CodecVec::default();
        let mut effective_nodes: CodecVec<NodeDigest> = CodecVec::default();
        let mut coord_nodes: FixedRadix<i64, NodeDigest> = FixedRadix::default();
        let mut all_nodes: CodecVec<NodeDigest> = CodecVec::default();
        let delta_view = delta.as_ref().map(|(view, _)| *view);
        if delta_view.is_none() {
            for event in input.effective {
                let coord = bj::coord(event)?;
                if raw_by_coord
                    .insert_admitted(coord, event, &*store)
                    .map_err(codec_error)?
                    .is_some()
                {
                    return Err("StorageUnavailable：有效源坐标碰撞，拒绝覆盖见证".to_string());
                }
                push_gen(&mut raw_bars, bar_of(event)?, store)?;
                let node = event_node(&mut self.event_nodes, store, event)?;
                coord_nodes
                    .insert_admitted(coord, node, &*store)
                    .map_err(codec_error)?;
                push_gen(&mut effective_nodes, node, store)?;
            }
            for event in input.all_events {
                push_gen(
                    &mut all_nodes,
                    event_node(&mut self.event_nodes, store, event)?,
                    store,
                )?;
            }
            for (coord, event) in raw_by_coord.iter() {
                self.producer
                    .bind_source(store, *coord, raw_ref_spec(*event)?)
                    .map_err(producer_error)?;
            }
        }
        let raw_ledger = match delta_view {
            Some(view) => BiLedgerView::Delta(view),
            None => BiLedgerView::RadixViews(&raw_by_coord),
        };
        // full oracle 与 delta 共用下方生成核，只有原账/parser 取得方式不同。
        let mut lane_guard = if delta_view.is_none() {
            Some(s_fact_cache::LaneGuard::take(&self.lane_key))
        } else {
            None
        };
        let state: &IncrFactsState = match delta_view {
            Some(view) => view.state(),
            None => {
                let lane = lane_guard
                    .as_mut()
                    .expect("完整入口车道")
                    .lane(&self.lane_key);
                lane.advance(&raw_bars);
                lane.state()
            }
        };
        let structure_enabled = crate::is_structure_rule(&input.rule_revision);
        let cd_prior = content_structure::StructurePrior {
            segment_records: input.structure_prior.segment_records,
            version_record: input.structure_prior.segment_version,
        };
        let (layer, evidence, second_store) = if structure_enabled {
            // 恢复只读上一代已核发布根；随后仅调用一次共享 parser 并保留同次游标簿。
            let mut fold = crate::tb02c::second_store(&self.theta_config.parse);
            content_structure::restore_second_cursors(store, &self.producer, &cd_prior, &mut fold)?;
            let (layer, evidence, fold) =
                parser::parse_layer_with_group_evidence_store(state, &self.theta_config, fold);
            (layer, evidence, Some(fold))
        } else {
            // 历史 A/B 域仍使用原输入路径，不将结构域失败切回这里。
            let (layer, evidence) =
                parser::parse_layer_with_group_evidence(state, &self.theta_config);
            (layer, evidence, None)
        };
        let merged_rc = layer.merged_bars.clone();
        let merged: &[Bar] = merged_rc.as_ref();
        let prepared = match delta_view {
            Some(view) => self.producer.prepare_raw_history_delta(store, view),
            None => self.producer.prepare_raw_history(&raw_bars),
        }
        .map_err(producer_error)?;

        // 只活在本次 generate 调用里，任何 Err/rollback 都不进入跨请求运行缓存。
        let mut fractal_sources = if native.is_some() || delta_view.is_some() {
            Some(
                self.producer
                    .parser_fractal_sources(&prepared, state)
                    .map_err(producer_error)?,
            )
        } else {
            None
        };

        // ── 形态窗：多根极值身份按 #1405 根集合语义发布──
        let wins = (0..merged.len().saturating_sub(2)).map(|start| {
            (
                start,
                classify_local_shape(&merged[start], &merged[start + 1], &merged[start + 2]),
            )
        });
        let mut shape_objects: CodecVec<ProducedObject> = CodecVec::default();
        let mut witnesses: CodecVec<WitnessRecord> = CodecVec::default();
        let mut relations: CodecVec<Built> = CodecVec::default();
        let mut observations: CodecVec<(bool, Built)> = CodecVec::default();
        // 逐形态的请求组（旧配方线序：形态请求 → 该形态的 CC-007 三条请求）。
        let mut shape_requests: CodecVec<(Built, CodecVec<Built>)> = CodecVec::default();
        let mut fractals: CodecVec<ProducedObject> = CodecVec::default();
        let mut fractal_reasons: CodecVec<&'static str> = CodecVec::default();
        // #1491：CC-054 请求的 (window_start, label) 键，与 `requests` 线序逐条对应。
        let mut shape_request_keys: Vec<Vec<RequestOrder>> = Vec::new();
        // #1491 bounded-layout/1：各分型（窗口序）的单元读数，组单元批记录用。
        let mut developments: Vec<(String, FractalDevelopment)> = Vec::new();
        for (start, shape) in wins {
            let built_shape = match &fractal_sources {
                Some(sources) => self
                    .producer
                    .build_parser_shape(store, &prepared, sources, start, gen, cut)
                    .map(|shape| shape.map(ShapeBuild::Native)),
                None => self
                    .producer
                    .build_shape(store, state, start, gen, cut)
                    .map(|shape| shape.map(ShapeBuild::Ordinary)),
            };
            match (shape, built_shape) {
                (_, Ok(None)) => return Err(format!("GenError：形态窗 {start} 缺同次三组证据")),
                (_, Ok(Some(built_shape))) => {
                    let object = built_shape.object();
                    let views = group_views(state, start)?;
                    let (wits, rels) = shape_witnesses(
                        store,
                        &mut self.event_nodes,
                        raw_ledger,
                        &coord_nodes,
                        &views,
                        start,
                        &object,
                    )?;
                    extend_gen(&mut witnesses, wits, store)?;
                    extend_gen(&mut relations, rels, store)?;
                    let request = object.request.clone();
                    let fractal = match (&built_shape, &mut fractal_sources) {
                        (ShapeBuild::Native(shape), Some(sources)) => self
                            .producer
                            .build_parser_fractal(store, &prepared, sources, shape, gen, cut),
                        _ => self.producer.build_fractal(
                            store,
                            &prepared,
                            object,
                            [&views[0], &views[1], &views[2]],
                            gen,
                            cut,
                        ),
                    }
                    .map_err(producer_error)?;
                    let mut fractal_requests: CodecVec<Built> = CodecVec::default();
                    let window_start = coord_i64(object.window[0])?;
                    let mut keys = vec![RequestOrder::new(window_start, RequestLabel::Shape)];
                    if let Some(fractal_object) = fractal {
                        for label in [
                            RequestLabel::ShapeDescription,
                            RequestLabel::DescriptiveLabels,
                            RequestLabel::SubsequentDevelopment,
                        ]
                        .into_iter()
                        .take(fractal_object.requests.len())
                        {
                            keys.push(RequestOrder::new(window_start, label));
                        }
                        if let Some(development) = &fractal_object.development {
                            developments
                                .push((fractal_object.object_id.clone(), development.clone()));
                        }
                        extend_gen(
                            &mut fractal_reasons,
                            fractal_object.knowledge_reasons.iter().copied(),
                            store,
                        )?;
                        extend_gen(
                            &mut fractal_requests,
                            fractal_object.requests.iter().cloned(),
                            store,
                        )?;
                        let generated = object_records::generated(
                            store,
                            object_records::BorrowedObject::Fractal(&fractal_object),
                        )
                        .map_err(record_error)?;
                        push_gen(
                            &mut fractals,
                            ProducedObject {
                                kind: native_text(store, CC007_FRACTAL_KIND)?,
                                fact_key: Some(native_text(store, &fractal_object.fact_key)?),
                                object_id: native_text(store, &fractal_object.object_id)?,
                                generated,
                                body: ObjectBody::Fractal(fractal_object),
                            },
                            store,
                        )?;
                    }
                    push_gen(&mut shape_requests, (request, fractal_requests), store)?;
                    shape_request_keys.push(keys);
                    let generated = object_records::generated(
                        store,
                        object_records::BorrowedObject::Shape(&object),
                    )
                    .map_err(record_error)?;
                    push_gen(
                        &mut shape_objects,
                        ProducedObject {
                            kind: native_text(store, CC006_SHAPE_KIND)?,
                            fact_key: None,
                            object_id: native_text(store, &object.object_id)?,
                            generated,
                            body: ObjectBody::Shape(built_shape.into_object()),
                        },
                        store,
                    )?;
                }
                (
                    Cc006LocalShape::DomainNotSatisfied { reason },
                    Err(ProducerError::WindowNotClassified(_)),
                ) => {
                    let value = shape_observation(
                        store,
                        "domain_not_satisfied",
                        reason.as_str(),
                        [Some(start), Some(start + 1), Some(start + 2)],
                    )?;
                    push_gen(&mut observations, (false, value), store)?;
                }
                (
                    Cc006LocalShape::InsufficientKnowledge,
                    Err(ProducerError::WindowNotClassified(_)),
                ) => {
                    let value = shape_observation(
                        store,
                        "insufficient_knowledge",
                        "fewer_than_three_bars",
                        [Some(start), None, None],
                    )?;
                    push_gen(&mut observations, (false, value), store)?;
                }
                (classified, Err(error)) => {
                    return Err(format!(
                        "GenError：形态窗 {start} 构造失败（滑窗判据={classified:?}）：{}",
                        producer_error(error)
                    ));
                }
            }
        }

        // ── 泛型事实：步骤 → 组 → 形态/分型请求 → 不足三根待决请求 ──
        let mut typed: CodecVec<ProducedObject> = CodecVec::default();
        let mut step_requests: CodecVec<Built> = CodecVec::default();
        let mut group_requests: CodecVec<Built> = CodecVec::default();
        let mut request_keys: Vec<RequestOrder> = Vec::new();
        let mut group_request_keys: Vec<RequestOrder> = Vec::new();
        let mut all_reasons: CodecVec<&'static str> = CodecVec::default();
        // #1393/#1565 W：初始化事实（槽 W）与逐原件未定记录；请求线序在步骤之前（同 v2）。
        let mut init_requests: CodecVec<Built> = CodecVec::default();
        {
            let mut prior_w = Vec::with_capacity(input.structure_prior.w_undetermined.len());
            for record in input.structure_prior.w_undetermined {
                prior_w.push(
                    crate::content_bi::read_prior_undetermined(store, &self.producer, record)
                        .map_err(|error| format!("GenError：上一代未定原件不可核：{error}"))?,
                );
            }
            let w = self
                .producer
                .build_initialization(store, state, &prior_w, gen, cut)
                .map_err(producer_error)?;
            extend_gen(&mut init_requests, w.requests, store)?;
            // 次序键与 `build_initialization` 的请求线序逐条对应：初始化 → 待决组 → 逐原件未定。
            request_keys.push(RequestOrder::initialization());
            if state.unsettled() {
                request_keys.push(RequestOrder::pending());
            }
            for raw_bar in state.pending_raw() {
                request_keys.push(RequestOrder::new(
                    coord_i64(raw_bar.source_index)?,
                    RequestLabel::UndeterminedRaw,
                ));
            }
            for object in w.objects {
                let kind = typed_kind(store, &object)?;
                let generated = object_records::generated(
                    store,
                    object_records::BorrowedObject::Typed(&object),
                )
                .map_err(record_error)?;
                push_gen(
                    &mut typed,
                    ProducedObject {
                        kind,
                        fact_key: Some(native_text(store, &object.fact_key)?),
                        object_id: native_text(store, &object.object_id)?,
                        generated,
                        body: ObjectBody::Typed(object),
                    },
                    store,
                )?;
            }
        }
        // #1393 W 审查 B2：等待期的 waiting 步由未定原件承担，不发布为 CC-004 步骤；建立方向后
        // 状态机已从 raw0 重折，步骤序列里不再有 waiting 步。
        let published_steps = if state.unsettled() {
            0
        } else {
            state.step_count()
        };
        for index in 0..published_steps {
            let view = state
                .shared_step(index)
                .map_err(|error| format!("GenError：共享步骤证据不可用：{}", error.reason))?
                .ok_or_else(|| format!("GenError：步骤 {index} 缺共享证据"))?;
            extend_gen(
                &mut all_reasons,
                view.waiting_reasons
                    .iter()
                    .copied()
                    .filter(|reason| *reason != "equal_extreme_identity"),
                store,
            )?;
            let object = self
                .producer
                .build_step_object(store, &view, gen, cut)
                .map_err(producer_error)?;
            request_keys.push(RequestOrder::new(
                coord_i64(view.incoming.source_index)?,
                RequestLabel::Step,
            ));
            push_gen(
                &mut step_requests,
                object.request.clone().ok_or_else(|| {
                    format!("GenError：CC-004 步骤 {index} 缺知识请求（旧配方必须有）")
                })?,
                store,
            )?;
            let generated =
                object_records::generated(store, object_records::BorrowedObject::Typed(&object))
                    .map_err(record_error)?;
            push_gen(
                &mut typed,
                ProducedObject {
                    kind: native_text(store, CC004_STEP_KIND)?,
                    fact_key: Some(native_text(store, &object.fact_key)?),
                    object_id: native_text(store, &object.object_id)?,
                    generated,
                    body: ObjectBody::Typed(object),
                },
                store,
            )?;
        }
        for index in 0..state.group_count() {
            let view = state
                .shared_group(index)
                .map_err(|error| format!("GenError：共享组证据不可用：{}", error.reason))?
                .ok_or_else(|| format!("GenError：组 {index} 缺共享证据"))?;
            extend_gen(
                &mut all_reasons,
                group_reasons_admitted(store, &view)
                    .map_err(producer_error)?
                    .iter()
                    .copied(),
                store,
            )?;
            let object = self
                .producer
                .build_group_object(
                    store,
                    index,
                    &view,
                    state.initial_direction_node(),
                    gen,
                    cut,
                )
                .map_err(producer_error)?;
            group_request_keys.push(RequestOrder::new(
                coord_i64(view.bar.source_index)?,
                RequestLabel::Group,
            ));
            push_gen(
                &mut group_requests,
                object.request.clone().ok_or_else(|| {
                    format!("GenError：CC-005 组 {index} 缺知识请求（旧配方必须有）")
                })?,
                store,
            )?;
            let generated =
                object_records::generated(store, object_records::BorrowedObject::Typed(&object))
                    .map_err(record_error)?;
            push_gen(
                &mut typed,
                ProducedObject {
                    kind: native_text(store, CC005_GROUP_KIND)?,
                    fact_key: Some(native_text(store, &object.fact_key)?),
                    object_id: native_text(store, &object.object_id)?,
                    generated,
                    body: ObjectBody::Typed(object),
                },
                store,
            )?;
        }
        let mut requests: CodecVec<Built> = CodecVec::default();
        extend_gen(&mut requests, init_requests, store)?;
        extend_gen(&mut requests, step_requests, store)?;
        extend_gen(&mut requests, group_requests, store)?;
        request_keys.extend(group_request_keys);
        for ((shape_request, fractal_requests), keys) in
            shape_requests.iter().zip(shape_request_keys)
        {
            push_gen(&mut requests, shape_request.clone(), store)?;
            extend_gen(&mut requests, fractal_requests.iter().cloned(), store)?;
            request_keys.extend(keys);
        }
        if merged.len() < 3 {
            push_gen(&mut all_reasons, "fewer_than_three_merged_bars", store)?;
            let key = CValue::Array(vec![
                CValue::String(CC006_SHAPE_KIND.to_string()),
                CValue::String("pending".to_string()),
            ]);
            let sources = raw_prefix_coords(state).map_err(producer_error)?;
            push_gen(
                &mut requests,
                self.producer
                    .pending_request(store, key, &["fewer_than_three_merged_bars"], &sources)
                    .map_err(producer_error)?,
                store,
            )?;
            request_keys.push(RequestOrder::pending());
        }
        if request_keys.len() != requests.len() {
            return Err(format!(
                "GenError：CC-054 请求次序键 {} 条与请求 {} 条不符",
                request_keys.len(),
                requests.len()
            ));
        }
        // #1419 A：有界布局按 (phase, window_start, label) 排序。跨代仅保证变化涉及的
        // 最早坐标之前的请求在原下标保持身份与摘要；删除取旧侧，移动取两侧。
        // layout/1 保持原线序。版本绑定与 S86 原门由 #1491 决定。
        if bounded {
            let mut order: Vec<usize> = (0..requests.len()).collect();
            order.sort_by_key(|&i| request_keys[i]);
            let mut sorted: CodecVec<Built> = CodecVec::default();
            for i in order {
                push_gen(&mut sorted, requests[i].clone(), store)?;
            }
            requests = sorted;
        }
        if !state.has_direction_evidence() {
            push_gen(&mut all_reasons, "direction_not_yet_established", store)?;
        }
        if state.unsettled() {
            push_gen(&mut all_reasons, "initial_direction_unsettled", store)?;
        }
        extend_gen(&mut all_reasons, fractal_reasons, store)?;
        extend_gen(&mut typed, fractals, store)?;
        if bounded {
            for object in self.development_batches(store, &prepared, &developments, gen, cut)? {
                let generated = object_records::generated(
                    store,
                    object_records::BorrowedObject::Typed(&object),
                )
                .map_err(record_error)?;
                push_gen(
                    &mut typed,
                    ProducedObject {
                        kind: native_text(store, CC007_DEVELOPMENT_BATCH_KIND)?,
                        fact_key: Some(native_text(store, &object.fact_key)?),
                        object_id: native_text(store, &object.object_id)?,
                        generated,
                        body: ObjectBody::Typed(object),
                    },
                    store,
                )?;
            }
        }
        stable_sort_typed_admitted(&mut typed, store)?;

        let knowledge_position = typed.len();

        // ── TB-02-B：七个 kind 的新笔证据（上一代记录由调用方给根）──
        let bi = self
            .bi
            .build_borrowed(
                &mut self.producer,
                store,
                &BiBuildInputView {
                    facts: &evidence.stroke_facts,
                    raw: raw_ledger,
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
            push_gen(
                &mut typed,
                ProducedObject {
                    kind,
                    fact_key: Some(native_text(store, &object.fact_key)?),
                    object_id: native_text(store, &object.object_id)?,
                    generated,
                    body: ObjectBody::Typed(object),
                },
                store,
            )?;
        }
        for relation in &bi.relations {
            push_gen(
                &mut relations,
                sc::write_value(&mut *store, relation)
                    .map_err(|error| format!("GenError：关系值写入失败：{error}"))?,
                store,
            )?;
        }

        if structure_enabled {
            let known_at = crate::content_bi::known_root(
                store,
                raw_ledger,
                gen,
                frontier,
                input.semantic_commit_ns,
            )?;
            let fact_generation = crate::content_bi::fact_generation(
                JsonView::BorrowedArray(&input.all_events)
                    .as_array()
                    .unwrap(),
            )
            .map_err(|e| format!("GenError：结构事实代际不可用：{e}"))?;
            let mut all_sources = CodecVec::default();
            for coord in raw_ledger.coords() {
                push_gen(
                    &mut all_sources,
                    usize::try_from(coord).map_err(|_| "原始坐标溢出")?,
                    store,
                )?;
            }
            let context = content_structure::StructureContext {
                fact_generation,
                generation: gen,
                cut,
                frontier,
                known_at: &known_at,
                all_sources: &all_sources,
            };
            let structure_evidence = content_structure::StructureEvidence {
                segments: &layer.segments,
                segment_facts: &evidence.segment_facts,
                stroke_facts: &evidence.stroke_facts,
                second_store: second_store
                    .as_ref()
                    .ok_or_else(|| "GenError：结构域缺同次 parser store".to_string())?,
            };
            let cd = content_structure::build_cd(
                &mut self.producer,
                store,
                &structure_evidence,
                &cd_prior,
                &context,
            )?;
            let ef = content_structure::build_ef(
                &mut self.producer,
                store,
                &structure_evidence,
                &cd.segments,
                &content_structure::EfPrior {
                    records: input.structure_prior.ef_records,
                },
                &context,
            )?;
            for output in [cd, ef] {
                extend_gen(&mut all_reasons, output.waiting_reasons, store)?;
                extend_gen(&mut relations, output.relations, store)?;
                // 包含一对象多子轴的完整唯一集合；不再从 TypedObject.request 重复收集。
                extend_gen(&mut requests, output.requests, store)?;
                for object in output.objects {
                    let kind = typed_kind(store, &object)?;
                    let generated = object_records::generated(
                        store,
                        object_records::BorrowedObject::Typed(&object),
                    )
                    .map_err(record_error)?;
                    push_gen(
                        &mut typed,
                        ProducedObject {
                            kind,
                            fact_key: Some(native_text(store, &object.fact_key)?),
                            object_id: native_text(store, &object.object_id)?,
                            generated,
                            body: ObjectBody::Structure(object),
                        },
                        store,
                    )?;
                }
            }
        }

        // 所有本次请求/等待原因收齐后才生成知识态；对象序仍保留历史 CC-054 槽位。
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
                state.unsettled(),
            )
            .map_err(producer_error)?;
        for observation in &knowledge.observations {
            push_gen(&mut observations, (true, observation.clone()), store)?;
        }
        let generated = object_records::generated(
            store,
            object_records::BorrowedObject::Typed(&knowledge.object),
        )
        .map_err(record_error)?;
        let knowledge_object = ProducedObject {
            kind: native_text(store, CC054_KNOWLEDGE_KIND)?,
            fact_key: Some(native_text(store, &knowledge.object.fact_key)?),
            object_id: native_text(store, &knowledge.object.object_id)?,
            generated,
            body: ObjectBody::Typed(knowledge.object),
        };
        if let Some(work) = store.codec_work() {
            typed.try_insert_native(knowledge_position, knowledge_object, work)
        } else {
            typed.try_insert(knowledge_position, knowledge_object)
        }
        .map_err(codec_error)?;

        // ── 生成态六条数组：跨代共享序列 + 旧配方等价去重 ──
        let mut objects: CodecVec<ProducedObject> = shape_objects;
        extend_gen(&mut objects, typed, store)?;
        let deduped = dedupe_observations(store, &observations)?;
        let (raw_events_array, effective_array) = match delta {
            Some((_, roots)) => (roots.raw_events, roots.effective_events),
            None => (
                self.raw_events_seq
                    .sync(store, &all_nodes, "raw_events", budget)?,
                self.effective_events_seq.sync(
                    store,
                    &effective_nodes,
                    "effective_events",
                    budget,
                )?,
            ),
        };
        let object_nodes = collect_gen(objects.iter().map(|o| o.generated.digest), store)?;
        let witness_nodes = collect_gen(witnesses.iter().map(|w| w.built.digest), store)?;
        let relation_nodes = collect_gen(relations.iter().map(|r| r.digest), store)?;
        let observation_nodes = collect_gen(deduped.iter().map(|o| o.digest), store)?;
        let objects_array = self
            .objects_seq
            .sync(store, &object_nodes, "objects", budget)?;
        let witnesses_array =
            self.witnesses_seq
                .sync(store, &witness_nodes, "witnesses", budget)?;
        let relations_array =
            self.relations_seq
                .sync(store, &relation_nodes, "relations", budget)?;
        let observations_array =
            self.observations_seq
                .sync(store, &observation_nodes, "observations", budget)?;

        let meta = StateMeta {
            generation: gen,
            structure_cut: TextValue::Plain(native_text(store, cut)?),
            input_frontier: frontier,
            catalog_revision: TextValue::Plain(native_text(store, input.catalog_revision)?),
            profile_id: TextValue::Plain(native_text(store, input.profile_id)?),
            profile_hash: TextValue::Plain(native_text(store, input.profile_hash)?),
            rule_revision: TextValue::Plain(native_text(store, input.rule_revision)?),
            scope: write_json(store, input.scope)?,
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
                raw_events: native_ref(store, &roots.raw_events)?,
                effective_events: native_ref(store, &roots.effective_events)?,
            },
            catalog_axes: None,
            state_without_axes: NativeValue::<ContentRef>::try_from_view(
                (&state_without_axes).into(),
                store.codec_work(),
            )
            .map_err(codec_error)?,
            generated_state: state_without_axes,
            policy_records: self.policy_records(),
            meta,
            roots,
            requests,
            all_events: input.all_events,
            is_ohlc: true,
        })
    }
}

/// 仅原账两根与 parser 变化，不是完整 Generated/Manifest/提交回执。
#[derive(Debug)]
pub(super) struct RawGenerationDelta {
    pub raw_events: Built,
    pub effective_events: Built,
    pub changes: s_fact_cache::ParserChanges,
}

fn native_text(store: &dyn NodeStore, text: &str) -> Result<NativeValue<String>, String> {
    NativeValue::<String>::try_from_str(text, store.codec_work()).map_err(codec_error)
}
fn native_ref(store: &dyn NodeStore, built: &Built) -> Result<NativeValue<ContentRef>, String> {
    NativeValue::<ContentRef>::try_from_built(built, store.codec_work()).map_err(codec_error)
}
fn push_gen<T>(values: &mut CodecVec<T>, value: T, store: &dyn NodeStore) -> Result<(), String> {
    if let Some(work) = store.codec_work() {
        values.try_reserve_native(work, 1).map_err(codec_error)?;
    }
    values.try_push(value).map_err(codec_error)
}
fn extend_gen<T>(
    values: &mut CodecVec<T>,
    items: impl IntoIterator<Item = T>,
    store: &dyn NodeStore,
) -> Result<(), String> {
    for value in items {
        push_gen(values, value, store)?;
    }
    Ok(())
}
/// #1488：与旧stable sort同序；两份真实持费索引工作区，unstable核以原ordinal破平局。
fn shape_observation(
    store: &mut dyn NodeStore,
    kind: &str,
    reason: &str,
    positions: [Option<usize>; 3],
) -> Result<Built, String> {
    let window = positions.map(|position| position.map(Value::from).unwrap_or(Value::Null));
    let empty = [];
    let fields = [
        ("kind", JsonView::String(kind)),
        ("reason", JsonView::String(reason)),
        ("window_start", JsonView::Plain(&window[0])),
        ("window_mid", JsonView::Plain(&window[1])),
        ("window_end", JsonView::Plain(&window[2])),
        (
            "detail",
            JsonView::object_fields(&empty).map_err(codec_error)?,
        ),
    ];
    write_json_view(
        store,
        JsonView::object_fields(&fields).map_err(codec_error)?,
    )
}

fn stable_sort_typed_admitted(
    values: &mut [ProducedObject],
    store: &dyn NodeStore,
) -> Result<(), String> {
    stable_sort_admitted(values, store, |a, b| {
        (a.kind.as_str(), a.fact_key.as_deref(), a.object_id.as_str()).cmp(&(
            b.kind.as_str(),
            b.fact_key.as_deref(),
            b.object_id.as_str(),
        ))
    })
}
fn stable_sort_admitted<T>(
    values: &mut [T],
    store: &dyn NodeStore,
    mut compare: impl FnMut(&T, &T) -> std::cmp::Ordering,
) -> Result<(), String> {
    let mut order = collect_gen(0..values.len(), store)?;
    order.sort_unstable_by(|left, right| {
        compare(&values[*left], &values[*right]).then_with(|| left.cmp(right))
    });
    let mut destination = collect_gen((0..values.len()).map(|_| 0usize), store)?;
    for (target, original) in order.iter().enumerate() {
        destination[*original] = target;
    }
    for index in 0..values.len() {
        while destination[index] != index {
            let target = destination[index];
            values.swap(index, target);
            destination.swap(index, target);
        }
    }
    Ok(())
}

fn collect_gen<T>(
    items: impl IntoIterator<Item = T>,
    store: &dyn NodeStore,
) -> Result<CodecVec<T>, String> {
    let mut values = CodecVec::default();
    extend_gen(&mut values, items, store)?;
    Ok(values)
}

/// 生成态对象（14 字段 + 可选 `catalog_axes`）：只写对象节点，六条数组借既有根。
fn emit_state(
    store: &mut dyn NodeStore,
    meta: &StateMeta,
    roots: &StateRoots,
    axes: Option<&ContentRef>,
) -> Result<NativeValue<ContentRef>, String> {
    let generation = super::super::content_manifest::Decimal::new(meta.generation);
    let frontier = super::super::content_manifest::Decimal::new(meta.input_frontier);
    let generation = sc::BorrowedNumberToken::parse(generation.as_str()).map_err(codec_error)?;
    let frontier = sc::BorrowedNumberToken::parse(frontier.as_str()).map_err(codec_error)?;
    let axes_digest = axes
        .map(ContentRef::digest)
        .transpose()
        .map_err(codec_error)?;
    let fields = [
        ("generation", VV::Number(generation)),
        ("structure_cut", VV::String(&meta.structure_cut)),
        ("input_frontier", VV::Number(frontier)),
        ("catalog_revision", VV::String(&meta.catalog_revision)),
        ("profile_id", VV::String(&meta.profile_id)),
        ("profile_hash", VV::String(&meta.profile_hash)),
        ("rule_revision", VV::String(&meta.rule_revision)),
        ("raw_events", VV::Edge(roots.raw_events.digest)),
        ("effective_events", VV::Edge(roots.effective_events.digest)),
        ("objects", VV::Edge(roots.objects.digest)),
        ("witnesses", VV::Edge(roots.witnesses.digest)),
        ("relations", VV::Edge(roots.relations.digest)),
        ("observations", VV::Edge(roots.observations.digest)),
        ("scope", VV::Edge(meta.scope.digest)),
        (
            "catalog_axes",
            axes_digest.map_or(VV::Json(JsonView::Null), VV::Edge),
        ),
    ];
    let built = sc::write_value_view(
        &mut *store,
        VV::Object(&fields[..if axes.is_some() { 15 } else { 14 }]),
    )
    .map_err(|error| format!("GenError：生成态写入失败：{error}"))?;
    native_ref(store, &built)
}

/// #1394：Plain维持bulk extend与clear容量复用，Owned仅由实际CodecVec allocator持账。
enum SequenceKeys {
    Plain(CodecVec<NodeDigest>),
    #[cfg(feature = "s_allocation_json_testonly")]
    Owned(sc::CodecVec<NodeDigest>),
}
impl Default for SequenceKeys {
    fn default() -> Self {
        Self::Plain(CodecVec::default())
    }
}
impl SequenceKeys {
    fn as_slice(&self) -> &[NodeDigest] {
        match self {
            Self::Plain(keys) => keys,
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(keys) => keys,
        }
    }
    fn allocation_budget(&self) -> Option<&sc::capture_allocation::OwnedBudget> {
        match self {
            Self::Plain(_) => None,
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(keys) => keys.allocation_budget(),
        }
    }
    fn owned(budget: &sc::capture_allocation::OwnedBudget) -> Result<Self, String> {
        #[cfg(feature = "s_allocation_json_testonly")]
        {
            return Ok(Self::Owned(sc::CodecVec::new_with_allocation(Some(budget))));
        }
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        {
            let _ = budget;
            Err(
                "prototype_audit_allocation_incomplete：sequence keys缺Owned构建，拒绝Plain兜底"
                    .into(),
            )
        }
    }
    fn reserve_admitted(&mut self, store: &dyn NodeStore, length: usize) -> Result<(), String> {
        match self {
            Self::Plain(keys) => {
                let additional = length.saturating_sub(keys.len());
                if let Some(work) = store.codec_work() {
                    keys.try_reserve_native(work, additional)
                        .map_err(codec_error)?;
                } else {
                    keys.try_reserve_exact(additional).map_err(codec_error)?;
                }
                Ok(())
            }
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(keys) => keys
                .try_reserve_exact(length.saturating_sub(keys.len()))
                .map_err(codec_error),
        }
    }

    fn replace_reserved(&mut self, elements: &[NodeDigest]) -> Result<(), String> {
        match self {
            Self::Plain(keys) => {
                keys.clear();
                for digest in elements {
                    keys.try_push(*digest).map_err(codec_error)?;
                }
            }
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(keys) => {
                keys.clear();
                // reserve_admitted已在改变原keys/seq前保证容量，try_push不会另作增长。
                for digest in elements {
                    keys.try_push(*digest).map_err(codec_error)?;
                }
            }
        }
        Ok(())
    }

    /// 容量已在改 keys/root 前预留；只截变化后缀，不复制未变前缀。
    fn splice_reserved(&mut self, prefix: usize, suffix: &[NodeDigest]) -> Result<(), String> {
        match self {
            Self::Plain(keys) => {
                keys.truncate(prefix);
                for digest in suffix {
                    keys.try_push(*digest).map_err(codec_error)?;
                }
            }
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(keys) => {
                keys.truncate(prefix);
                for digest in suffix {
                    keys.try_push(*digest).map_err(codec_error)?;
                }
            }
        }
        Ok(())
    }
}

/// 私有工作描述，**不是** owner 签发的原件资格。
///
/// 只能由本模块在消费真实 owner/raw/parser 变化之后派生；不对 prepared/外部调用者暴露
/// unchecked prefix。当前生产仍走旧 sync，接线前仅测试消费，不能称完整 delta 已上线。
struct SequenceSplice<'a> {
    expected_previous: Option<SequenceRef>,
    unchanged_prefix: usize,
    replacement_suffix: &'a [NodeDigest],
}

/// 跨代共享序列记忆：未变前缀不重写，真变化处一次重建后缀（#1488）。
#[derive(Default)]
pub(super) struct SequenceMemo {
    keys: SequenceKeys,
    seq: Option<SequenceRef>,
    bound: bool,
}

impl SequenceMemo {
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn visit_runtime_roots(
        &self,
        name: &'static str,
        f: &mut dyn FnMut(&'static str, NodeDigest),
    ) {
        for root in self.keys.as_slice() {
            f(name, *root);
        }
        if let Some(seq) = self.seq {
            f(name, seq.root());
        }
    }
    pub(super) fn allocation_budget(&self) -> Option<&sc::capture_allocation::OwnedBudget> {
        self.keys.allocation_budget()
    }
    fn discard(&mut self) {
        match &mut self.keys {
            SequenceKeys::Plain(keys) if !keys.is_native_bound() => keys.clear(),
            _ => self.keys = SequenceKeys::default(),
        }
        self.seq = None;
        self.bound = false;
    }
    fn check_native_scope(&mut self, store: &dyn NodeStore) -> Result<(), String> {
        let keys = match &mut self.keys {
            SequenceKeys::Plain(keys) => keys,
            #[cfg(feature = "s_allocation_json_testonly")]
            SequenceKeys::Owned(_) => return Ok(()),
        };
        match store.codec_work() {
            Some(work) if self.bound => keys.try_reserve_native(work, 0).map_err(codec_error),
            None if keys.is_native_bound() => Err(
                "prototype_audit_allocation_incomplete：Native sequence缓存需要当前scope".into(),
            ),
            _ => Ok(()),
        }
    }

    /// 消费本模块由已核变化导出的 splice；不读取或比较任何未变 keys。
    /// 旧根/长度、原账身份在写前核对；失败不推进 root/keys，不回全量 sync 重判。
    fn sync_splice(
        &mut self,
        store: &mut dyn NodeStore,
        edit: SequenceSplice<'_>,
        what: &'static str,
        budget: Option<&sc::capture_allocation::OwnedBudget>,
    ) -> Result<Built, String> {
        self.check_native_scope(store)?;
        if self.bound
            && !match (self.allocation_budget(), budget) {
                (None, None) => true,
                (Some(a), Some(b)) => a.same_ledger(b),
                _ => false,
            }
        {
            return Err("prototype_audit_allocation_incomplete：缓存账本改变，须discard".into());
        }
        let previous_len = self.keys.as_slice().len();
        if self.seq != edit.expected_previous
            || self.bound != self.seq.is_some()
            || edit.unchanged_prefix > previous_len
        {
            return Err(format!(
                "GenError：{what} splice 旧根或前缀不匹配，须discard"
            ));
        }
        let previous = match self.seq {
            Some(seq) if seq.count() == previous_len as u64 => seq,
            None if previous_len == 0 => SequenceRef::empty(),
            _ => return Err(format!("GenError：{what} 序列缓存计数不一致，须discard")),
        };
        let length = edit
            .unchanged_prefix
            .checked_add(edit.replacement_suffix.len())
            .ok_or_else(|| format!("GenError：{what} splice 长度溢出"))?;
        let mut next_keys = if !self.bound && (budget.is_some() || store.codec_work().is_some()) {
            Some(match budget {
                Some(b) => SequenceKeys::owned(b)?,
                None => SequenceKeys::default(),
            })
        } else {
            None
        };
        next_keys
            .as_mut()
            .unwrap_or(&mut self.keys)
            .reserve_admitted(store, length)?;
        let seq = sc::sequence_rebuild_suffix_with_allocation(
            &mut *store,
            &previous,
            edit.unchanged_prefix as u64,
            edit.replacement_suffix,
            budget,
        )
        .map_err(|error| format!("GenError：{what} 序列后缀重建失败：{error}"))?;
        let built = array_from_sequence(&mut *store, &seq).map_err(producer_error)?;
        next_keys
            .as_mut()
            .unwrap_or(&mut self.keys)
            .splice_reserved(edit.unchanged_prefix, edit.replacement_suffix)?;
        if let Some(keys) = next_keys {
            self.keys = keys;
        }
        self.seq = Some(seq);
        self.bound = true;
        Ok(built)
    }

    pub(super) fn sync(
        &mut self,
        store: &mut dyn NodeStore,
        elements: &[NodeDigest],
        what: &'static str,
        budget: Option<&sc::capture_allocation::OwnedBudget>,
    ) -> Result<Built, String> {
        self.check_native_scope(store)?;
        if self.bound
            && !match (self.allocation_budget(), budget) {
                (None, None) => true,
                (Some(a), Some(b)) => a.same_ledger(b),
                _ => false,
            }
        {
            return Err("prototype_audit_allocation_incomplete：缓存账本改变，须discard".into());
        }
        // 初次Owned绑定暂留局部，失败不冻结模式；现有Owned真实扩容先同时留账旧新块。
        let mut next_keys = if !self.bound && (budget.is_some() || store.codec_work().is_some()) {
            Some(match budget {
                Some(b) => SequenceKeys::owned(b)?,
                None => SequenceKeys::default(),
            })
        } else {
            None
        };
        next_keys
            .as_mut()
            .unwrap_or(&mut self.keys)
            .reserve_admitted(store, elements.len())?;
        let keys = self.keys.as_slice();
        let common = keys
            .iter()
            .zip(elements.iter())
            .take_while(|(left, right)| left == right)
            .count();
        let previous = match self.seq {
            Some(seq) if seq.count() == keys.len() as u64 => seq,
            None if keys.is_empty() => SequenceRef::empty(),
            _ => return Err(format!("GenError：{what} 序列缓存计数不一致，须discard")),
        };
        let seq = sc::sequence_rebuild_suffix_with_allocation(
            &mut *store,
            &previous,
            common as u64,
            &elements[common..],
            budget,
        )
        .map_err(|error| format!("GenError：{what} 序列后缀重建失败：{error}"))?;
        let built = array_from_sequence(&mut *store, &seq).map_err(producer_error)?;
        next_keys
            .as_mut()
            .unwrap_or(&mut self.keys)
            .replace_reserved(elements)?;
        if let Some(keys) = next_keys {
            self.keys = keys;
        }
        self.seq = Some(seq);
        self.bound = true;
        Ok(built)
    }
}

/// 旧配方去重的**等价实现**：`detail.tb02a == true` 的一族按规范编码摘要比较，
/// 其余按 `kind|window_start|window_mid|window_end|reason`。两类各自等价类与旧实现的文本键
/// 逐个相同（等值 ⟺ 同摘要），且不展开观察 `detail.source_coords` 大边。
fn dedupe_observations(
    store: &mut dyn NodeStore,
    observations: &[(bool, Built)],
) -> Result<CodecVec<Built>, String> {
    let mut seen = bj::new_map::<NativeValue<String>, ()>(None);
    let mut out = CodecVec::default();
    for (from_knowledge, built) in observations {
        let root = native_ref(store, built)?;
        let key = if *from_knowledge {
            store
                .native_format(format_args!("\u{1}tb02a\u{1}{}", root.root))
                .map_err(codec_error)?
        } else {
            let kind = field_text_admitted(store, &root, "kind")?;
            let start = field_text_admitted(store, &root, "window_start")?;
            let mid = field_text_admitted(store, &root, "window_mid")?;
            let end = field_text_admitted(store, &root, "window_end")?;
            let reason = field_text_admitted(store, &root, "reason")?;
            store
                .native_format(format_args!(
                    "{}\u{1}{}\u{1}{}\u{1}{}\u{1}{}",
                    kind, start, mid, end, reason
                ))
                .map_err(codec_error)?
        };
        if !seen.contains_key(key.as_str()) {
            store.native_map_room(&mut seen).map_err(codec_error)?;
            seen.insert(key, ()).map_err(codec_error)?;
            push_gen(&mut out, built.clone(), store)?;
        }
    }
    Ok(out)
}
fn field_text_admitted(
    store: &mut dyn NodeStore,
    root: &ContentRef,
    name: &str,
) -> Result<TextValue, String> {
    let reference = store
        .lookup_field_ref(root.into(), name)
        .map_err(|error| format!("GenError：小路径定位失败：{error}"))?;
    let value = bj::render_json(store, reference.view())?;
    json_text(store, value.view())
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
struct JsonOwners<'a>(&'a [JsonOwner]);
impl sc::capture_allocation::audit_json::JsonArray for JsonOwners<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<JsonView<'_>> {
        self.0.get(index).map(JsonOwner::view)
    }
}
fn json_text(store: &dyn NodeStore, view: JsonView<'_>) -> Result<TextValue, String> {
    struct Output<'a> {
        bytes: CodecVec<u8>,
        source: &'a dyn NodeStore,
    }
    impl std::io::Write for Output<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if let Some(work) = self.source.codec_work() {
                self.bytes
                    .try_reserve_native(work, bytes.len())
                    .map_err(std::io::Error::other)?;
            }
            self.bytes
                .try_extend_bytes(bytes)
                .map_err(std::io::Error::other)?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut output = Output {
        bytes: CodecVec::default(),
        source: store,
    };
    serde_json::to_writer(&mut output, &view)
        .map_err(|e| format!("GenError：小字段渲染失败：{e}"))?;
    TextValue::from_bytes(output.bytes)
}
fn projection_field(
    store: &mut dyn NodeStore,
    root: &Built,
    path: &[&str],
) -> Result<JsonOwner, String> {
    let mut reference = store.native_built_ref(root).map_err(codec_error)?;
    for field in path {
        reference = store
            .lookup_field_ref(reference.view(), field)
            .map_err(|e| format!("GenError：小路径定位失败：{e}"))?;
    }
    bj::render_json(store, reference.view())
}
fn catalog_projection_entry(
    store: &mut dyn NodeStore,
    object: &ProducedObject,
    requests: &[Built],
) -> Result<JsonOwner, String> {
    let payload = || {
        object.payload().ok_or_else(|| {
            format!(
                "GenError：{} 对象 {} 缺 payload 值根",
                object.kind(),
                object.object_id()
            )
        })
    };
    let projection = if object.kind() == CC004_STEP_KIND {
        let comparisons = projection_field(store, payload()?, &["comparisons"])?;
        let fields = [("comparisons", comparisons.view())];
        json_text(
            store,
            JsonView::object_fields(&fields).map_err(codec_error)?,
        )?
    } else if object.kind() == CC054_KNOWLEDGE_KIND {
        let waiting = projection_field(store, payload()?, &["waiting_reasons"])?;
        let mut request_items = CodecVec::default();
        for request in requests {
            let request_id = projection_field(store, request, &["request_id"])?;
            let reasons = projection_field(store, request, &["reasons"])?;
            let fields = [
                ("request_id", request_id.view()),
                ("reasons", reasons.view()),
            ];
            let row = bj::json_from_view_admitted(
                store,
                JsonView::object_fields(&fields).map_err(codec_error)?,
            )?;
            push_gen(&mut request_items, row, store)?;
        }
        let items = JsonOwners(&request_items);
        let fields = [
            ("waiting_reasons", waiting.view()),
            ("requests", JsonView::BorrowedArray(&items)),
        ];
        json_text(
            store,
            JsonView::object_fields(&fields).map_err(codec_error)?,
        )?
    } else if matches!(
        object.kind(),
        "CC-056.version"
            | "CC-056.segment_version"
            | "CC-056.center_version"
            | "CC-056.pair_version"
    ) {
        let waiting = projection_field(store, payload()?, &["data", "waiting_reasons"])?;
        let inner = [("waiting_reasons", waiting.view())];
        let outer = [(
            "data",
            JsonView::object_fields(&inner).map_err(codec_error)?,
        )];
        json_text(store, JsonView::object_fields(&outer).map_err(codec_error)?)?
    } else {
        TextValue::Plain(native_text(store, "{}")?)
    };
    let fields = [
        ("object_id", JsonView::String(object.object_id())),
        ("kind", JsonView::String(object.kind())),
        ("payload_json", JsonView::String(&projection)),
    ];
    bj::json_from_view_admitted(
        store,
        JsonView::object_fields(&fields).map_err(codec_error)?,
    )
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
    event_nodes: &mut FixedRadix<(i64, i64), NodeDigest>,
    raw_by_coord: BiLedgerView<'_>,
    coord_nodes: &FixedRadix<i64, NodeDigest>,
    views: &[GroupEvidenceView<'_>; 3],
    start: usize,
    object: &Cc006Object,
) -> Result<(CodecVec<WitnessRecord>, CodecVec<Built>), String> {
    let mut witnesses = CodecVec::default();
    let mut relations = CodecVec::default();
    for (slot, view) in views.iter().enumerate() {
        let mi = start + slot;
        let mut bar_nodes = CodecVec::default();
        for coord in view.members {
            let coord = *coord as i64;
            let event = raw_by_coord.get(coord).ok_or_else(|| {
                format!("StorageUnavailable：见证成员坐标 {coord} 不在本次生效账内")
            })?;
            let node = match coord_nodes.get(&coord) {
                Some(node) => *node,
                None => event_node(event_nodes, store, event)?,
            };
            push_gen(&mut bar_nodes, VV::Edge(node), store)?;
        }
        let bar_array = sc::write_value_view(&mut *store, VV::Array(&bar_nodes))
            .map_err(|error| format!("GenError：见证 raw_bars 写入失败：{error}"))?;
        let witness_id = witness_identity_native(store, &object.object_id, slot, mi, &bar_array)?;
        let source_index =
            super::super::content_manifest::Decimal::new(view.bar.source_index as i64);
        let slot_text = super::super::content_manifest::Decimal::new(slot as i64);
        let close = store
            .native_format(format_args!("{}", view.bar.close))
            .map_err(codec_error)?;
        let high = store
            .native_format(format_args!("{}", view.bar.high))
            .map_err(codec_error)?;
        let low = store
            .native_format(format_args!("{}", view.bar.low))
            .map_err(codec_error)?;
        let open = store
            .native_format(format_args!("{}", view.bar.open))
            .map_err(codec_error)?;
        let fields = [
            ("merged_close", VV::String(&close)),
            ("merged_high", VV::String(&high)),
            ("merged_low", VV::String(&low)),
            ("merged_open", VV::String(&open)),
            (
                "merged_source_index",
                VV::Number(
                    sc::BorrowedNumberToken::parse(source_index.as_str()).map_err(codec_error)?,
                ),
            ),
            ("object_id", VV::String(&object.object_id)),
            ("raw_bars", VV::Edge(bar_array.digest)),
            (
                "slot",
                VV::Number(
                    sc::BorrowedNumberToken::parse(slot_text.as_str()).map_err(codec_error)?,
                ),
            ),
            ("witness_id", VV::String(&witness_id)),
        ];
        let built = sc::write_value_view(&mut *store, VV::Object(&fields))
            .map_err(|error| format!("GenError：见证记录写入失败：{error}"))?;
        let fields = [
            ("object", VV::String(&witness_id)),
            ("relation_type", VV::String("has_witness")),
            ("subject", VV::String(&object.object_id)),
        ];
        let relation = sc::write_value_view(&mut *store, VV::Object(&fields))
            .map_err(|error| format!("GenError：关系值写入失败：{error}"))?;
        push_gen(
            &mut witnesses,
            WitnessRecord {
                witness_id,
                object_id: native_text(store, &object.object_id)?,
                slot,
                built,
            },
            store,
        )?;
        push_gen(&mut relations, relation, store)?;
    }
    let window = store
        .native_format(format_args!(
            "window-{}-{}-{}",
            object.window[0], object.window[1], object.window[2]
        ))
        .map_err(codec_error)?;
    let fields = [
        ("object", VV::String(&object.object_id)),
        ("relation_type", VV::String("classified_as")),
        ("subject", VV::String(&window)),
    ];
    let classified = sc::write_value_view(&mut *store, VV::Object(&fields))
        .map_err(|error| format!("GenError：关系值写入失败：{error}"))?;
    push_gen(&mut relations, classified, store)?;
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
    let bytes = witness_identity_bytes(store, object_id, slot, mi, raw_bars)?;
    // 旧裸 String 接口；Native 来源核改走下方携原许可的输出。
    if store.codec_work().is_some() {
        return Err("Native witness身份必须使用witness_identity_native持有原许可".into());
    }
    let mut result = String::with_capacity(bytes.len());
    result.extend(bytes.into_iter().map(char::from));
    Ok(result)
}

pub(super) fn witness_identity_native(
    store: &mut (impl CodecRead + ?Sized),
    object_id: &str,
    slot: usize,
    mi: usize,
    raw_bars: &Built,
) -> Result<sc::capture_allocation::native_owned::NativeValue<String>, String> {
    let bytes = witness_identity_bytes(store, object_id, slot, mi, raw_bars)?;
    let text = std::str::from_utf8(&bytes).expect("固定 ASCII witness 身份");
    store.native_string(text).map_err(codec_error)
}

fn witness_identity_bytes(
    store: &mut (impl CodecRead + ?Sized),
    object_id: &str,
    slot: usize,
    mi: usize,
    raw_bars: &Built,
) -> Result<[u8; 20], String> {
    if object_id.contains('"') || object_id.contains('\\') {
        return Err(format!(
            "GenError：对象身份 `{object_id}` 含需转义字符，拒绝拼入规范输入"
        ));
    }
    let mut hasher = Sha256::new();
    {
        use std::io::Write;
        let mut writer = DigestWriter(&mut hasher);
        write!(
            writer,
            r#"{{"mi":{mi},"object_id":"{object_id}","raw_bars":"#
        )
        .map_err(|e| format!("GenError：见证输入失败：{e}"))?;
        let reference = store.native_built_ref(raw_bars).map_err(codec_error)?;
        store
            .render_ref(reference.view(), &mut writer)
            .map_err(|error| format!("GenError：见证输入流式渲染失败：{error}"))?;
        write!(writer, r#","slot":{slot}}}"#)
            .map_err(|e| format!("GenError：见证输入失败：{e}"))?;
    }
    let digest = hasher.finalize();
    // 两种输出所有权共用同一个配方；固定 ASCII 暂存不分配堆内存。
    let mut result = [0u8; 20];
    result[..4].copy_from_slice(b"wit-");
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for (i, byte) in digest[..8].iter().enumerate() {
        result[4 + i * 2] = HEX[(byte >> 4) as usize];
        result[5 + i * 2] = HEX[(byte & 15) as usize];
    }
    Ok(result)
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
    memo: &mut FixedRadix<(i64, i64), NodeDigest>,
    store: &mut dyn NodeStore,
    event: JsonView<'_>,
) -> Result<NodeDigest, String> {
    let coord = bj::coord(event)?;
    let revision = bj::field(event, "revision")
        .as_i64()
        .ok_or("StorageUnavailable：raw.revision 不是 i64")?;
    memo.check_admission(&*store).map_err(codec_error)?;
    if let Some(node) = memo.get(&(coord, revision)) {
        return Ok(*node);
    }
    let built = sc::write_json_view(&mut *store, event)
        .map_err(|error| format!("GenError：原始事件节点写入失败：{error}"))?;
    memo.insert_admitted((coord, revision), built.digest, &*store)
        .map_err(codec_error)?;
    Ok(built.digest)
}

/// 生效账事件 → `Bar`（与旧生成段同一取法：OHLC 档案取 open/high/low/close）。
fn bar_of(event: JsonView<'_>) -> Result<Bar, String> {
    s_fact_cache::bar_of(event)
}

/// 生效账事件 → producer 的八字段源引用登记（字段只从原行取，不另判）。
fn raw_ref_spec(event: JsonView<'_>) -> Result<RawRefSpec, String> {
    RawRefSpec::from_event(event)
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
fn typed_kind(
    store: &mut dyn NodeStore,
    object: &TypedObject,
) -> Result<NativeValue<String>, String> {
    let root = native_ref(store, &object.record)?;
    let kind = store
        .lookup_field_ref((&root).into(), "kind")
        .map_err(codec_error)?;
    if kind.kind() != Kind::String {
        return Err(format!(
            "GenError：对象 {} 记录 kind 不是字符串（{:?}）",
            object.object_id,
            kind.kind()
        ));
    }
    let value = bj::render_json(store, kind.view())?;
    let text = value
        .view()
        .as_str()
        .ok_or_else(|| format!("GenError：对象 {} 记录 kind 不是文本", object.object_id))?;
    native_text(store, text)
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
    write_json_view(store, value.into())
}

fn write_json_view(
    store: &mut (impl CodecStore + ?Sized),
    value: JsonView<'_>,
) -> Result<Built, String> {
    fn check(value: JsonView<'_>) -> Result<(), String> {
        if let Some(n) = value.number() {
            if n.as_i64().is_none() && n.as_u64().is_none() {
                return Err(format!(
                    "GenError：原始行出现非整数 JSON 数字 `{n}`（不把 token 静默改写）"
                ));
            }
        } else if let Some(items) = value.as_array() {
            for item in items {
                check(item)?;
            }
        } else if let Some(fields) = value.as_object() {
            for (_, item) in fields.iter() {
                check(item)?;
            }
        }
        Ok(())
    }
    check(value)?;
    store
        .write_json_view(value)
        .map_err(|error| format!("GenError：值写入失败：{error}"))
}

/// #1491 ADR 0037 §一：CC-054 请求的 label 序（同一 window_start 内）。W 逐原件未定、步骤与组请求的
/// 键尾是单坐标，形态与分型请求的键尾是窗口；三条分型请求按原配方 label 顺序。W 未定排在步骤前，
/// 与 layout/1 线序（W 在步骤之前）同向（#1419）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum RequestLabel {
    UndeterminedRaw,
    Step,
    Group,
    Shape,
    ShapeDescription,
    DescriptiveLabels,
    SubsequentDevelopment,
}

/// #1491：bounded-layout/1 的 CC-054 `requests` 排序键 (window_start, label)；无窗口的待决请求
/// （不足三根合并 bar、方向未定的待决组）排最后——它们只在形态/方向出现之前存在。
/// #1419：W 初始化请求（槽 W，每代恰一条、无窗口）排最前，与 layout/1 线序同位且逐代不动。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum RequestPhase {
    Initialization,
    Windowed,
    Pending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct RequestOrder {
    phase: RequestPhase,
    window_start: i64,
    label: Option<RequestLabel>,
}

impl RequestOrder {
    fn new(window_start: i64, label: RequestLabel) -> Self {
        Self {
            phase: RequestPhase::Windowed,
            window_start,
            label: Some(label),
        }
    }

    fn initialization() -> Self {
        Self {
            phase: RequestPhase::Initialization,
            window_start: 0,
            label: None,
        }
    }

    fn pending() -> Self {
        Self {
            phase: RequestPhase::Pending,
            window_start: 0,
            label: None,
        }
    }
}

fn coord_i64(coord: usize) -> Result<i64, String> {
    i64::try_from(coord).map_err(|_| format!("GenError：源坐标 {coord} 超出 i64"))
}

impl GenerationRuntime {
    /// #1491 ADR 0037 §二：本代全部 `CC-007.development_batch`（bar 坐标升序，每根有单元的
    /// 后续 bar 一条）。
    ///
    /// `developments` 是本代全部被跟踪分型的单元读数，按窗口序给出——entries 即按分型窗口序
    /// （`window[0]` 严格升序，ADR 0054 / #1783，#1790 修订；取代 ADR 0037 §二「出生序」一句）：
    /// 纯追加下与分型出生序相同；中段出生或原位重生时，现行身份配方（来源前缀链）使其后分型同代
    /// 重生，出生构成窗口序后缀，两序在可达输入上恒等（#1790 修订；锁见
    /// `bounded_layout_cc007_entries_window_order_on_mid_birth`），判据只认窗口序。
    /// 脏 bar（任一分型本代重算的单元、新出现/消失分型的整段区间）一律按本代读数重建载荷；
    /// 其余 bar 的 entries 与上一代逐项相同，复用载荷值根。
    fn development_batches(
        &mut self,
        store: &mut dyn NodeStore,
        prepared: &PreparedRawHistory<'_>,
        developments: &[(String, FractalDevelopment)],
        gen: i64,
        cut: &str,
    ) -> Result<Vec<TypedObject>, String> {
        let mut ranges: std::collections::HashMap<String, (i64, i64)> =
            std::collections::HashMap::with_capacity(developments.len());
        let mut dirty: Vec<(i64, i64)> = Vec::new();
        for (id, development) in developments {
            let units = &development.units;
            let (Some(first), Some(last)) = (units.first(), units.last()) else {
                continue;
            };
            let range = (first.coord, last.coord);
            match self.development_ranges.get(id) {
                Some(previous) if previous.0 == range.0 => {
                    // 起点 = producer 失效水位与首个重算单元坐标的较小者：被移走的中段坐标
                    // 不在本代单元里，但不小于水位，故必落入脏区间（#1780 独审 C-1）。
                    let recomputed = units.get(development.recomputed_from).map(|u| u.coord);
                    let from = match (development.invalidated_from, recomputed) {
                        (Some(floor), Some(coord)) => Some(floor.min(coord)),
                        (floor, coord) => floor.or(coord),
                    };
                    if let Some(from) = from {
                        dirty.push((from, range.1.max(previous.1)));
                    }
                    if previous.1 > range.1 {
                        dirty.push((range.1, previous.1));
                    }
                }
                Some(previous) => {
                    dirty.push(*previous);
                    dirty.push(range);
                }
                None => dirty.push(range),
            }
            if ranges.insert(id.clone(), range).is_some() {
                return Err(format!("GenError：CC-007 分型 {id} 在同代重复出现"));
            }
        }
        for (id, previous) in &self.development_ranges {
            if !ranges.contains_key(id) {
                dirty.push(*previous);
            }
        }
        dirty.sort_unstable();
        let mut merged: Vec<(i64, i64)> = Vec::with_capacity(dirty.len());
        for (lo, hi) in dirty {
            match merged.last_mut() {
                Some(last) if lo <= last.1.saturating_add(1) => last.1 = last.1.max(hi),
                _ => merged.push((lo, hi)),
            }
        }
        let mut dirty_coords: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();
        for &(lo, hi) in &merged {
            dirty_coords.extend(self.development_batches.range(lo..=hi).map(|(c, _)| *c));
            for (_, development) in developments {
                let units = &development.units;
                let start = units.partition_point(|u| u.coord < lo);
                let end = units.partition_point(|u| u.coord <= hi);
                dirty_coords.extend(units[start..end].iter().map(|u| u.coord));
            }
        }
        let mut rebuilt: std::collections::BTreeMap<i64, TypedObject> =
            std::collections::BTreeMap::new();
        for coord in dirty_coords {
            let mut entries = Vec::new();
            for (id, development) in developments {
                let units = &development.units;
                if let Ok(i) = units.binary_search_by_key(&coord, |u| u.coord) {
                    entries.push((id.as_str(), &units[i]));
                }
            }
            if entries.is_empty() {
                self.development_batches.remove(&coord);
                continue;
            }
            let at = usize::try_from(coord)
                .map_err(|_| format!("GenError：单元批记录坐标 {coord} 非法"))?;
            let object = self
                .producer
                .build_development_batch(store, prepared, at, &entries, None, gen, cut)
                .map_err(producer_error)?;
            self.development_batches
                .insert(coord, object.payload.clone());
            rebuilt.insert(coord, object);
        }
        let mut objects = Vec::with_capacity(self.development_batches.len());
        for (&coord, payload) in &self.development_batches {
            if let Some(object) = rebuilt.remove(&coord) {
                objects.push(object);
                continue;
            }
            let at = usize::try_from(coord)
                .map_err(|_| format!("GenError：单元批记录坐标 {coord} 非法"))?;
            objects.push(
                self.producer
                    .build_development_batch(
                        store,
                        prepared,
                        at,
                        &[],
                        Some(payload.clone()),
                        gen,
                        cut,
                    )
                    .map_err(producer_error)?,
            );
        }
        self.development_ranges = ranges;
        Ok(objects)
    }
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
pub(super) mod tests {
    #[test]
    fn borrowed_json_write_keeps_old_integer_oracle_and_rejects_before_put() {
        let original = serde_json::json!({"items": [i64::MIN, u64::MAX], "text": "1.5"});
        let mut expected_nodes = sc::MemoryNodes::new();
        let expected =
            sc::write_value(&mut expected_nodes, &json_to_cvalue(&original).unwrap()).unwrap();
        let mut actual_nodes = sc::MemoryNodes::new();
        let actual = write_json_view(&mut actual_nodes, (&original).into()).unwrap();
        assert_eq!(actual.digest, expected.digest);
        let bad = serde_json::json!({"outer": [1, {"float": 1.5}]});
        let mut refused = sc::MemoryNodes::new();
        let error = write_json_view(&mut refused, (&bad).into()).unwrap_err();
        assert_eq!(
            error,
            "GenError：原始行出现非整数 JSON 数字 `1.5`（不把 token 静默改写）"
        );
        assert_eq!(refused.iter().count(), 0, "完整整数预检先于任何节点写出");
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_axis_view_uses_same_integer_guard_and_codec_root() {
        let original = serde_json::json!({"CC-004": {"count": 2, "ids": ["a", "b"]}});
        let budget = sc::capture_allocation::OwnedBudget::new(1 << 20);
        let owner =
            sc::capture_allocation::audit_json::OwnedJson::from_view((&original).into(), &budget)
                .unwrap();
        let mut plain_nodes = sc::MemoryNodes::new();
        let expected = write_json(&mut plain_nodes, &original).unwrap();
        let mut owned_nodes = sc::MemoryNodes::new();
        let actual = write_json_view(&mut owned_nodes, owner.view()).unwrap();
        assert_eq!(actual.digest, expected.digest);
        drop(owner);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    use super::*;
    use crate::tests::{
        accept_ohlc, init_test_db, located_ohlc_input, tb02b_oracle_rows, TestFiles,
    };
    use crate::{
        canonical_bytes_of_value, compute_diff, delta_storage, load_batch_bytes, meta_get_opt,
        open_db, read_effective_events, read_stored_objects, verified_root_raw_events,
        StoredObjects, DEFAULT_WRITER_EPOCH,
    };
    use newchan_rust::s_content::{render_canonical, MemoryNodes};
    use serde_json::json;

    #[cfg(feature = "s_capture_native_testonly")]
    struct NativeFixture<'a> {
        inner: &'a mut MemoryNodes,
        work: &'a sc::capture_allocation::scratch::WorkScope,
    }
    #[cfg(feature = "s_capture_native_testonly")]
    impl sc::NodeSource for NativeFixture<'_> {
        fn read(&mut self, digest: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            self.inner.read(digest)
        }
        fn codec_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
            Some(self.work)
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    impl sc::NodeSink for NativeFixture<'_> {
        fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
            self.inner.put(digest, tag, body)
        }
        fn codec_sink_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
            Some(self.work)
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn event_radix_memo_hit_requires_current_native_scope_and_keeps_root_recipe() {
        let event = serde_json::json!({"source_coord":"0","revision":1,"value":7});
        let mut oracle = MemoryNodes::new();
        let expected = sc::write_json_view(&mut oracle, (&event).into())
            .unwrap()
            .digest;
        let budget = super::super::audit_owner::native::Budget::new(1 << 20, 0).unwrap();
        let baseline = budget.live();
        let mut memo = FixedRadix::default();
        let mut nodes = MemoryNodes::new();
        budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let mut store = NativeFixture {
                    inner: &mut nodes,
                    work,
                };
                let actual = event_node(&mut memo, &mut store, (&event).into())?;
                assert_eq!(actual, expected);
                let count = store.inner.iter().count();
                assert_eq!(
                    event_node(&mut memo, &mut store, (&event).into())?,
                    expected
                );
                assert_eq!(store.inner.iter().count(), count);
                Ok::<_, String>(())
            })
            .unwrap();
        let mut plain = MemoryNodes::new();
        assert!(event_node(&mut memo, &mut plain, (&event).into()).is_err());
        assert_eq!(
            plain.iter().count(),
            0,
            "no memo hit may silently bypass current domain"
        );
        drop(memo);
        assert_eq!(budget.live(), baseline);
    }

    #[test]
    fn fixed_shape_observations_match_old_json_roots() {
        // Oracles copy the frozen pre-G1 JSON recipes, independently of helper inputs.
        let start = 7usize;
        let cases = [
            (
                "domain_not_satisfied",
                "adjacent_inclusion",
                [Some(start), Some(start + 1), Some(start + 2)],
                serde_json::json!({
                    "kind": "domain_not_satisfied",
                    "reason": "adjacent_inclusion",
                    "window_start": start,
                    "window_mid": start + 1,
                    "window_end": start + 2,
                    "detail": {},
                }),
            ),
            (
                "insufficient_knowledge",
                "fewer_than_three_bars",
                [Some(start), None, None],
                serde_json::json!({
                    "kind": "insufficient_knowledge",
                    "reason": "fewer_than_three_bars",
                    "window_start": start,
                    "window_mid": Value::Null,
                    "window_end": Value::Null,
                    "detail": {},
                }),
            ),
        ];
        for (kind, reason, positions, old) in cases {
            let mut oracle = MemoryNodes::new();
            let expected = write_json(&mut oracle, &old).unwrap();
            let mut actual = MemoryNodes::new();
            let got = shape_observation(&mut actual, kind, reason, positions).unwrap();
            assert_eq!(got.digest, expected.digest);
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_stable_sort_matches_old_equal_key_order_and_refuses_atomically() {
        let input = (0..128usize).map(|i| ((i * 37) % 9, i)).collect::<Vec<_>>();
        let mut expected = input.clone();
        expected.sort_by(|a, b| a.0.cmp(&b.0));
        let budget = super::super::audit_owner::native::Budget::new(1 << 20, 0).unwrap();
        let baseline = budget.live();
        let mut actual = input.clone();
        let mut nodes = MemoryNodes::new();
        budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let store = NativeFixture {
                    inner: &mut nodes,
                    work,
                };
                stable_sort_admitted(&mut actual, &store, |a, b| a.0.cmp(&b.0))
            })
            .unwrap();
        assert_eq!(actual, expected, "与旧稳定排序逐项同序，等键原ordinal不变");
        assert_eq!(budget.live(), baseline, "排序临时工作区已经真实释放");
        let tiny = super::super::audit_owner::native::Budget::new(
            baseline + sc::capture_allocation::scratch::CodecWork::metadata_bytes() + 8,
            0,
        )
        .unwrap();
        let tiny_baseline = tiny.live();
        let mut refused = input.clone();
        let result = tiny.codec_work().unwrap().run(|work| {
            let store = NativeFixture {
                inner: &mut nodes,
                work,
            };
            stable_sort_admitted(&mut refused, &store, |a, b| a.0.cmp(&b.0))
        });
        assert!(matches!(
            result,
            Err(sc::capture_allocation::scratch::WorkError::Resource(_))
        ));
        assert_eq!(refused, input, "全部工作区准入完成前不改原顺序");
        assert_eq!(tiny.live(), tiny_baseline);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_sequence_cross_generation_same_parent_foreign_refusal_and_discard() {
        let mut nodes = MemoryNodes::new();
        let a = sc::write_value(&mut nodes, &CValue::Bool(true))
            .unwrap()
            .digest;
        let b = sc::write_value(&mut nodes, &CValue::Bool(false))
            .unwrap()
            .digest;
        let c = sc::write_value(&mut nodes, &CValue::Null).unwrap().digest;
        let mut oracle = SequenceMemo::default();
        let first = oracle.sync(&mut nodes, &[a, b], "oracle-1", None).unwrap();
        let second = oracle
            .sync(&mut nodes, &[a, c, b], "oracle-2", None)
            .unwrap();
        let budget = super::super::audit_owner::native::Budget::new(1 << 20, 0).unwrap();
        let baseline = budget.live();
        let mut memo = SequenceMemo::default();
        let got = budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let mut store = NativeFixture {
                    inner: &mut nodes,
                    work,
                };
                memo.sync(&mut store, &[a, b], "native-1", None)
            })
            .unwrap();
        assert_eq!(got.digest, first.digest);
        assert!(budget.live() > baseline, "离域后真实keys容量仍持费");
        let got = budget
            .codec_work()
            .unwrap()
            .run(|work| {
                let mut store = NativeFixture {
                    inner: &mut nodes,
                    work,
                };
                memo.sync(&mut store, &[a, c, b], "native-2", None)
            })
            .unwrap();
        assert_eq!(got.digest, second.digest);
        let before = memo.seq;
        let foreign = super::super::audit_owner::native::Budget::new(1 << 20, 0).unwrap();
        let foreign_baseline = foreign.live();
        let result = foreign.codec_work().unwrap().run(|work| {
            let mut store = NativeFixture {
                inner: &mut nodes,
                work,
            };
            memo.sync(&mut store, &[a, c, b], "foreign-zero-growth", None)
        });
        assert!(matches!(
            result,
            Err(sc::capture_allocation::scratch::WorkError::Resource(_))
        ));
        assert_eq!(memo.seq, before);
        assert_eq!(memo.keys.as_slice(), &[a, c, b]);
        assert_eq!(foreign.live(), foreign_baseline);
        assert!(memo.sync(&mut nodes, &[a], "no-scope", None).is_err());
        assert_eq!(memo.seq, before);
        memo.discard();
        assert_eq!(
            budget.live(),
            baseline,
            "Native discard释放keys真实buffer及绑定"
        );
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_generated_results_match_plain_and_keep_permits_until_drop() {
        let events = delta_raw_fixture();
        let effective = bj::effective(events.as_slice().into(), None).unwrap();
        let scope = json!({"instrument":"native-generation", "exchange":"fixture"});
        let input = GenerationInput {
            structure_prior: Default::default(),
            all_events: events.as_slice().into(),
            effective: effective.raw_view(),
            generation: 1,
            structure_cut: "cut-1",
            input_frontier: events.len() as i64,
            profile_id: crate::tb02a::PROFILE,
            profile_hash: "profile-hash",
            rule_revision: crate::V3_RULE_REVISION,
            catalog_revision: "catalog",
            scope: &scope,
            semantic_commit_ns: None,
            prior: PriorState {
                bi_records: Vec::new(),
                version_record: None,
            },
        };
        let mut plain_nodes = MemoryNodes::new();
        let mut plain_runtime = GenerationRuntime::new(
            "native-parity/plain".into(),
            crate::tb02a::PROFILE,
            crate::V3_RULE_REVISION,
        );
        let mut plain = plain_runtime.generate(&mut plain_nodes, &input).unwrap();
        plain.finish_axes(&mut plain_nodes, -1, &[], &[]).unwrap();
        let budget = super::super::audit_owner::native::Budget::new(16 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let mut native_nodes = MemoryNodes::new();
        let mut runtime = GenerationRuntime::new(
            "native-parity/native".into(),
            crate::tb02a::PROFILE,
            crate::V3_RULE_REVISION,
        );
        let generated = budget
            .codec_work()
            .unwrap()
            .run(|work| -> Result<_, String> {
                let mut store = NativeFixture {
                    inner: &mut native_nodes,
                    work,
                };
                let mut generated = runtime.generate(&mut store, &input)?;
                generated.finish_axes(&mut store, -1, &[], &[])?;
                assert!(generated.generated_state.is_native_admitted());
                assert!(generated.raw.raw_events.is_native_admitted());
                assert!(generated
                    .objects
                    .iter()
                    .all(|o| o.object_id.is_native_admitted()));
                assert!(generated
                    .witnesses
                    .iter()
                    .all(|w| w.witness_id.is_native_admitted()));
                Ok(generated)
            })
            .unwrap();
        assert_eq!(generated.generated_state, plain.generated_state);
        assert_eq!(
            generated
                .objects
                .iter()
                .map(|o| (o.kind(), o.fact_key(), o.object_id()))
                .collect::<Vec<_>>(),
            plain
                .objects
                .iter()
                .map(|o| (o.kind(), o.fact_key(), o.object_id()))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            generated
                .witnesses
                .iter()
                .map(|w| (&w.witness_id, &w.object_id, w.slot, &w.built))
                .collect::<Vec<_>>(),
            plain
                .witnesses
                .iter()
                .map(|w| (&w.witness_id, &w.object_id, w.slot, &w.built))
                .collect::<Vec<_>>()
        );
        assert!(budget.live() > baseline, "离开工作域后结果仍持有原许可");
        drop(generated);
        drop(runtime);
        assert_eq!(budget.live(), baseline, "实际持有者析构后归还原账");
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn finish_axes_resource_rejection_preserves_published_roots() {
        let events = delta_raw_fixture();
        let effective = bj::effective(events.as_slice().into(), None).unwrap();
        let scope = json!({"instrument":"native-finish-reject", "exchange":"fixture"});
        let input = GenerationInput {
            structure_prior: Default::default(),
            all_events: events.as_slice().into(),
            effective: effective.raw_view(),
            generation: 1,
            structure_cut: "cut-1",
            input_frontier: events.len() as i64,
            profile_id: crate::tb02a::PROFILE,
            profile_hash: "profile-hash",
            rule_revision: crate::V3_RULE_REVISION,
            catalog_revision: "catalog",
            scope: &scope,
            semantic_commit_ns: None,
            prior: PriorState {
                bi_records: Vec::new(),
                version_record: None,
            },
        };
        let mut nodes = MemoryNodes::new();
        let mut runtime = GenerationRuntime::new(
            "native-finish-reject".into(),
            crate::tb02a::PROFILE,
            crate::V3_RULE_REVISION,
        );
        let mut generated = runtime.generate(&mut nodes, &input).unwrap();
        generated.finish_axes(&mut nodes, -1, &[], &[]).unwrap();
        let state = generated.generated_state.as_ref().clone();
        let axes = generated.catalog_axes.as_ref().unwrap().as_ref().clone();
        const LIMIT: usize = 16 * 1024 * 1024;
        let budget = super::super::audit_owner::native::Budget::new(LIMIT, 0).unwrap();
        let baseline = budget.live();
        let result = budget
            .codec_work()
            .unwrap()
            .run(|work| -> Result<(), String> {
                let _occupy = budget.reserve(LIMIT - budget.live() - 1).unwrap();
                let mut store = NativeFixture {
                    inner: &mut nodes,
                    work,
                };
                let result = generated.finish_axes(&mut store, -1, &[], &[]);
                assert!(result.is_err());
                assert_eq!(generated.generated_state.as_ref(), &state);
                assert_eq!(generated.catalog_axes.as_ref().unwrap().as_ref(), &axes);
                result.map(|_| ())
            });
        assert!(matches!(
            result,
            Err(sc::capture_allocation::scratch::WorkError::Resource(_))
        ));
        assert_eq!(budget.live(), baseline, "拒绝后释放全部暂存许可");
    }

    fn delta_raw_fixture() -> Vec<Value> {
        [
            ("a", 2, 10, 5, 1), ("b", 7, 12, 7, 1),
            ("c", 12, 11, 8, 2), ("d", 17, 16, 12, 1),
            ("b", 7, 12, 7, 2), // OHLC 不变，真实来源版本变化。
            ("c", 12, 14, 9, 4), ("e", 22, 20, 16, 1),
            ("c", 12, 13, 8, 3), // 迟到较低修订不改变最终赢家。
            ("f", 5, 11, 6, 1), // 中段插入。
            ("a", 17, 17, 12, 2), ("d", 2, 9, 4, 2), // 同 cut 坐标互换。
        ]
        .into_iter()
        .enumerate()
        .map(|(ordinal, (identity, coord, high, low, revision))| json!({
            "identity_key": identity, "source_coord": coord.to_string(),
            "revision": revision, "seq": ordinal + 1, "ts": coord.to_string(),
            "open": low.to_string(), "high": high.to_string(), "low": low.to_string(),
            "close": high.to_string(), "input_revision": revision.to_string(),
            "payload_hash": format!("payload-{ordinal}"), "receipt_id": format!("receipt-{ordinal}"),
            "event_id": format!("event-{identity}"),
        }))
        .collect()
    }

    /// 新结构域的真实生成接线：双构造同根、全目录与原因闭合，恢复只认同域的已核根。
    #[test]
    fn structure_domain_wires_versions_knowledge_axes_and_rejects_foreign_prior() {
        let rows = delta_raw_fixture();
        let raw = RawEventsView::plain(&rows[..4]);
        let effective = bj::effective(raw, None).unwrap();
        let scope = json!({});
        let input = GenerationInput {
            all_events: raw,
            effective: effective.raw_view(),
            generation: 1,
            structure_cut: "cut-1",
            input_frontier: 4,
            profile_id: crate::tb02a::PROFILE,
            profile_hash: "structure-profile",
            rule_revision: crate::V3_STRUCTURE_RULE_REVISION,
            catalog_revision: "structure-catalog",
            scope: &scope,
            semantic_commit_ns: Some("123456789"),
            prior: PriorState {
                bi_records: vec![],
                version_record: None,
            },
            structure_prior: StructurePriors::default(),
        };
        let mut nodes = MemoryNodes::new();
        let mut first = GenerationRuntime::new(
            "structure-wire-first".into(),
            crate::tb02a::PROFILE,
            crate::V3_STRUCTURE_RULE_REVISION,
        );
        let mut generated = first.generate(&mut nodes, &input).unwrap();
        generated.finish_axes(&mut nodes, -1, &[], &[]).unwrap();
        generated.verify_new_closure(&mut nodes).unwrap();
        #[cfg(feature = "s_capture_native_testonly")]
        {
            let budget =
                super::super::audit_owner::native::Budget::new(16 * 1024 * 1024, 0).unwrap();
            let baseline = budget.live();
            let mut native_nodes = MemoryNodes::new();
            let mut native_runtime = GenerationRuntime::new(
                "structure-wire-native".into(),
                crate::tb02a::PROFILE,
                crate::V3_STRUCTURE_RULE_REVISION,
            );
            let result = budget
                .codec_work()
                .unwrap()
                .run(|work| -> Result<_, String> {
                    let mut store = NativeFixture {
                        inner: &mut native_nodes,
                        work,
                    };
                    let mut output = native_runtime.generate(&mut store, &input)?;
                    output.finish_axes(&mut store, -1, &[], &[])?;
                    Ok(output)
                })
                .unwrap();
            assert_eq!(result.generated_state, generated.generated_state);
            assert!(result
                .objects
                .iter()
                .any(|o| matches!(&o.body, ObjectBody::Structure(_))));
            assert!(budget.live() > baseline, "C/D/E/F结果离域后仍持父账许可");
            drop(result);
            drop(native_runtime);
            assert_eq!(budget.live(), baseline);
        }
        let mut canonical = Vec::new();
        render_canonical(&mut nodes, &generated.generated_state, &mut canonical).unwrap();
        let state: Value = serde_json::from_slice(&canonical).unwrap();
        assert_eq!(state["catalog_axes"].as_object().unwrap().len(), 22);
        let objects = state["objects"].as_array().unwrap();
        let knowledge: Value = serde_json::from_str(
            objects
                .iter()
                .find(|o| o["kind"] == CC054_KNOWLEDGE_KIND)
                .unwrap()["payload_json"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        for kind in [
            "CC-056.segment_version",
            "CC-056.center_version",
            "CC-056.pair_version",
        ] {
            let matches: Vec<_> = objects.iter().filter(|o| o["kind"] == kind).collect();
            assert_eq!(matches.len(), 1, "{kind} 应由本次事实实际构造");
            let version: Value =
                serde_json::from_str(matches[0]["payload_json"].as_str().unwrap()).unwrap();
            for reason in version["data"]["waiting_reasons"].as_array().unwrap() {
                assert!(
                    knowledge["waiting_reasons"]
                        .as_array()
                        .unwrap()
                        .contains(reason),
                    "{kind} 原因未进入 CC-054：{reason}"
                );
            }
        }
        let mut other_nodes = MemoryNodes::new();
        let mut other = GenerationRuntime::new(
            "structure-wire-independent".into(),
            crate::tb02a::PROFILE,
            crate::V3_STRUCTURE_RULE_REVISION,
        );
        let mut independent = other.generate(&mut other_nodes, &input).unwrap();
        independent
            .finish_axes(&mut other_nodes, -1, &[], &[])
            .unwrap();
        assert_eq!(generated.generated_state, independent.generated_state);

        let mut segment_records = Vec::new();
        let mut segment_version = None;
        let mut ef_records = Vec::new();
        for object in &generated.objects {
            let typed: &TypedObject = match &object.body {
                ObjectBody::Typed(typed) => typed,
                ObjectBody::Structure(typed) => typed,
                _ => continue,
            };
            match object.kind() {
                "CC-013.segment" => segment_records.push(typed.record.clone()),
                "CC-056.segment_version" => segment_version = Some(typed.record.clone()),
                "CC-016.center_seed_attempt"
                | "CC-016.center"
                | "CC-017.center_roles"
                | "CC-022.unit_assignment"
                | "CC-018.unit_core_position"
                | "CC-020.center_pair"
                | "CC-056.center_version"
                | "CC-056.pair_version" => ef_records.push(typed.record.clone()),
                _ => {}
            }
        }
        let (bi_records, bi_version) = clone_prior(&generated);
        let next = GenerationInput {
            generation: 2,
            structure_cut: "cut-2",
            prior: PriorState {
                bi_records: bi_records.iter().collect(),
                version_record: bi_version.as_ref(),
            },
            structure_prior: StructurePriors {
                segment_records: &segment_records,
                segment_version: segment_version.as_ref(),
                ef_records: &ef_records,
                ..Default::default()
            },
            ..input
        };
        let warm = first.generate(&mut nodes, &next).unwrap();
        let mut cold = GenerationRuntime::new(
            "structure-wire-restart-valid".into(),
            crate::tb02a::PROFILE,
            crate::V3_STRUCTURE_RULE_REVISION,
        );
        let restarted = cold.generate(&mut nodes, &next).unwrap();
        assert_eq!(
            warm.generated_state, restarted.generated_state,
            "重启与续跑必须使用同份已授权 prior"
        );

        // B 版本虽自身内容有效，也不能作为 C/D 的发布恢复许可。
        let wrong = generated
            .objects
            .iter()
            .find_map(|o| match &o.body {
                ObjectBody::Typed(object) if o.kind() == "CC-056.version" => {
                    Some(object.record.clone())
                }
                _ => None,
            })
            .unwrap();
        let forged = GenerationInput {
            structure_prior: StructurePriors {
                segment_version: Some(&wrong),
                ..Default::default()
            },
            ..next
        };
        let mut restart = GenerationRuntime::new(
            "structure-wire-restart-reject".into(),
            crate::tb02a::PROFILE,
            crate::V3_STRUCTURE_RULE_REVISION,
        );
        let error = match restart.generate(&mut nodes, &forged) {
            Ok(_) => panic!("错误 prior 版本不得恢复或切旧核兜底"),
            Err(error) => error,
        };
        assert!(error.contains("prior kind"), "{error}");
    }

    fn delta_bound_input<'a>(
        scope: &'a Value,
        cut: &'a str,
        gen: i64,
        frontier: i64,
        prior: PriorState<'a>,
    ) -> BoundGenerationInput<'a, 'a> {
        BoundGenerationInput {
            structure_prior: StructurePriors::default(),
            generation: gen,
            structure_cut: cut,
            input_frontier: frontier,
            profile_id: crate::tb02a::PROFILE,
            profile_hash: "profile-hash",
            rule_revision: crate::V3_RULE_REVISION,
            catalog_revision: "catalog",
            scope,
            semantic_commit_ns: Some("123456789"),
            prior,
        }
    }

    fn clone_prior(generated: &Generated<'_>) -> (Vec<Built>, Option<Built>) {
        let (bis, version) = prior_roots(generated);
        (bis.into_iter().cloned().collect(), version.cloned())
    }

    fn bound_full_generation_parity(events: &[Value], cuts: &[usize]) -> (usize, usize) {
        use super::super::audit_owner::RawAdvanceFixture;
        let fixture = RawAdvanceFixture::new();
        let mut index = bj::RawPrefixIndex::new(events.into());
        let mut delta = GenerationRuntime::new(
            "A2 delta".into(),
            crate::tb02a::PROFILE,
            crate::V3_RULE_REVISION,
        );
        let mut full = GenerationRuntime::new(
            "A2 full independent raw/parser".into(),
            crate::tb02a::PROFILE,
            crate::V3_RULE_REVISION,
        );
        let mut ds = MemoryNodes::new();
        let mut fs = MemoryNodes::new();
        let scope = json!({"instrument":"A2", "exchange":"fixture"});
        let (mut dp, mut fp) = (
            (Vec::<Built>::new(), None::<Built>),
            (Vec::<Built>::new(), None::<Built>),
        );
        let mut previous = 0;
        let mut witness_total = 0;
        let mut bi_total = 0;
        for (turn, &prefix) in cuts.iter().enumerate() {
            let gen = turn as i64 + 1;
            let cut = format!("cut-{gen}");
            let changes = index.advance(prefix).unwrap();
            let raw = RawEventsView::plain(events).prefix(prefix).unwrap();
            let bound = fixture.bind(raw, previous, &changes.winner_edits).unwrap();
            let effective = bj::effective(raw, None).unwrap();
            let mut actual = delta
                .generate_bound(
                    &mut ds,
                    &bound,
                    delta_bound_input(
                        &scope,
                        &cut,
                        gen,
                        prefix as i64,
                        PriorState {
                            bi_records: dp.0.iter().collect(),
                            version_record: dp.1.as_ref(),
                        },
                    ),
                )
                .unwrap();
            let mut expected = full
                .generate(
                    &mut fs,
                    &GenerationInput {
                        structure_prior: StructurePriors::default(),
                        all_events: raw,
                        effective: effective.raw_view(),
                        generation: gen,
                        structure_cut: &cut,
                        input_frontier: prefix as i64,
                        profile_id: crate::tb02a::PROFILE,
                        profile_hash: "profile-hash",
                        rule_revision: crate::V3_RULE_REVISION,
                        catalog_revision: "catalog",
                        scope: &scope,
                        semantic_commit_ns: Some("123456789"),
                        prior: PriorState {
                            bi_records: fp.0.iter().collect(),
                            version_record: fp.1.as_ref(),
                        },
                    },
                )
                .unwrap();
            // 六数组摘要与完整规范字节分别对拍；对象序和当前包装随全部数组一并核。
            let roots = |g: &Generated<'_>| {
                [
                    g.roots.raw_events.clone(),
                    g.roots.effective_events.clone(),
                    g.roots.objects.clone(),
                    g.roots.witnesses.clone(),
                    g.roots.relations.clone(),
                    g.roots.observations.clone(),
                ]
            };
            assert_eq!(roots(&actual), roots(&expected), "A2 第 {gen} 代六根");
            assert_eq!(actual.state_without_axes, expected.state_without_axes);
            assert_eq!(
                actual
                    .objects
                    .iter()
                    .map(|o| (o.kind(), o.fact_key(), o.object_id()))
                    .collect::<Vec<_>>(),
                expected
                    .objects
                    .iter()
                    .map(|o| (o.kind(), o.fact_key(), o.object_id()))
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                actual
                    .witnesses
                    .iter()
                    .map(|w| (&w.witness_id, &w.object_id, w.slot, &w.built))
                    .collect::<Vec<_>>(),
                expected
                    .witnesses
                    .iter()
                    .map(|w| (&w.witness_id, &w.object_id, w.slot, &w.built))
                    .collect::<Vec<_>>()
            );
            assert_eq!(actual.relations, expected.relations);
            assert_eq!(actual.observations, expected.observations);
            actual
                .finish_axes(&mut ds, previous as i64, &[], &[])
                .unwrap();
            expected
                .finish_axes(&mut fs, previous as i64, &[], &[])
                .unwrap();
            assert_eq!(actual.generated_state, expected.generated_state);
            assert_eq!(actual.catalog_axes, expected.catalog_axes);
            let (a_text, value) = state_value(&mut ds, &actual);
            let (e_text, _) = state_value(&mut fs, &expected);
            assert_eq!(a_text, e_text, "A2 第 {gen} 代完整规范字节");
            assert_eq!(value["generation"], gen);
            assert_eq!(value["structure_cut"], cut);
            assert_eq!(value["raw_events"].as_array().unwrap().len(), prefix);
            actual.verify_new_closure(&mut ds).unwrap();
            expected.verify_new_closure(&mut fs).unwrap();
            witness_total += actual.witnesses.len();
            bi_total += actual
                .objects
                .iter()
                .filter(|o| o.kind() == "CC-010.bi")
                .count();
            for event in raw {
                let coord = bj::coord(event).unwrap();
                let is_current = effective.iter().any(|e| bj::coord(e).unwrap() == coord);
                assert_eq!(
                    delta.producer.source_root(coord).is_some(),
                    is_current,
                    "移走坐标必须撤来源登记：{coord}"
                );
            }
            let next_dp = clone_prior(&actual);
            let next_fp = clone_prior(&expected);
            drop(actual);
            drop(expected);
            dp = next_dp;
            fp = next_fp;
            previous = prefix;
        }
        (witness_total, bi_total)
    }

    #[test]
    fn delta_full_six_roots_revision_swap_insertion_and_metadata_match_full() {
        let events = delta_raw_fixture();
        let (witnesses, _) =
            bound_full_generation_parity(&events, &[0, 1, 2, 4, 5, 6, 8, 8, 9, 11]);
        assert!(witnesses > 0, "完整对拍必须实际产三槽见证");
    }

    fn delta_long_fixture() -> Vec<Value> {
        tb02b_oracle_rows().into_iter().enumerate().map(|(i, (id, coord, open, high, low, close))| json!({
            "identity_key": id, "event_id": id, "source_coord": coord.to_string(), "ts": coord.to_string(),
            "open": open.to_string(), "high": high.to_string(), "low": low.to_string(), "close": close.to_string(),
            "revision": 1, "seq": i + 1, "input_revision": "1", "payload_hash": format!("hash-{i}"),
            "receipt_id": format!("receipt-{i}"),
        })).collect()
    }

    #[test]
    fn delta_full_bi_prior_tail_revision_and_reactivated_windows_match_full() {
        let mut events = delta_long_fixture();
        assert!(events.len() >= 14);
        events.truncate(14);
        let mut revised = events[2].clone();
        revised["revision"] = json!(2);
        revised["input_revision"] = json!("2");
        revised["seq"] = json!(15);
        revised["high"] = json!("13");
        revised["low"] = json!("9");
        revised["open"] = json!("11");
        revised["close"] = json!("11");
        revised["payload_hash"] = json!("revision-2");
        revised["receipt_id"] = json!("receipt-15");
        events.push(revised);
        let mut restored = events[2].clone();
        restored["revision"] = json!(3);
        restored["input_revision"] = json!("3");
        restored["seq"] = json!(16);
        restored["payload_hash"] = json!("restored-3");
        restored["receipt_id"] = json!("receipt-16");
        events.push(restored);
        let mut moved = events[13].clone();
        moved["revision"] = json!(2);
        moved["input_revision"] = json!("2");
        moved["seq"] = json!(17);
        moved["source_coord"] = json!("1000");
        moved["ts"] = json!("1000");
        moved["payload_hash"] = json!("tail-moved");
        moved["receipt_id"] = json!("receipt-17");
        events.push(moved);
        let (witnesses, bis) =
            bound_full_generation_parity(&events, &[1, 2, 8, 14, 15, 16, 17, 17]);
        assert!(witnesses > 0 && bis > 0, "实际包含三槽见证及跨代 bi prior");
    }

    #[test]
    fn delta_full_direction_correction_competing_extremes_and_coordinate_removal_match_full() {
        let mut events: Vec<Value> = [(10,10,5),(11,15,8),(37,15,6),(900,12,6),(9001,9,4),(12345,9,3)]
            .into_iter().enumerate().map(|(i,(coord,high,low))| json!({
                "identity_key": format!("id-{i}"), "event_id": format!("event-{i}"),
                "source_coord": coord.to_string(), "ts": coord.to_string(), "revision":1,
                "seq":i+1, "input_revision":"1", "payload_hash":format!("hash-{i}"),
                "receipt_id":format!("receipt-{i}"), "open":low.to_string(), "close":high.to_string(),
                "high":high.to_string(), "low":low.to_string(),
            })).collect();
        let mut first = events[0].clone();
        first["revision"] = json!(2);
        first["input_revision"] = json!("2");
        first["seq"] = json!(7);
        first["high"] = json!("20");
        first["low"] = json!("10");
        first["open"] = json!("10");
        first["close"] = json!("20");
        first["payload_hash"] = json!("direction-corrected");
        first["receipt_id"] = json!("receipt-7");
        events.push(first);
        let mut moved = events[5].clone();
        moved["revision"] = json!(2);
        moved["input_revision"] = json!("2");
        moved["seq"] = json!(8);
        moved["source_coord"] = json!("20000");
        moved["ts"] = json!("20000");
        moved["payload_hash"] = json!("moved");
        moved["receipt_id"] = json!("receipt-8");
        events.push(moved);
        bound_full_generation_parity(&events, &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn delta_full_cold_matches_independent_sealed_v2_oracle() {
        use super::super::audit_owner::RawAdvanceFixture;
        let oracle_db = OracleDb::new();
        oracle_db.accept(&tb02b_oracle_rows()[..14]);
        let previous_frontier = oracle_db.advance();
        let mut oracle = oracle_db.read();
        restrict_oracle_to_tb02ab(&mut oracle, previous_frontier, &[], &[]);
        let mut index = bj::RawPrefixIndex::new((&oracle.all_events).into());
        let edits = index.advance(oracle.all_events.len()).unwrap();
        let fixture = RawAdvanceFixture::new();
        let bound = fixture
            .bind((&oracle.all_events).into(), 0, &edits.winner_edits)
            .unwrap();
        let mut store = MemoryNodes::new();
        let mut runtime = GenerationRuntime::new(
            "A2 sealed oracle".into(),
            crate::tb02a::PROFILE,
            oracle.batch["rule_revision"].as_str().unwrap(),
        );
        let cut = format!("cut-{}", oracle.generation);
        let mut generated = runtime
            .generate_bound(
                &mut store,
                &bound,
                BoundGenerationInput {
                    structure_prior: StructurePriors::default(),
                    generation: oracle.generation,
                    structure_cut: &cut,
                    input_frontier: oracle.batch["input_frontier"].as_i64().unwrap(),
                    profile_id: oracle.batch["profile_id"].as_str().unwrap(),
                    profile_hash: oracle.batch["profile_hash"].as_str().unwrap(),
                    rule_revision: oracle.batch["rule_revision"].as_str().unwrap(),
                    catalog_revision: oracle.batch["catalog_revision"].as_str().unwrap(),
                    scope: &oracle.batch["scope"],
                    semantic_commit_ns: None,
                    prior: PriorState {
                        bi_records: vec![],
                        version_record: None,
                    },
                },
            )
            .unwrap();
        generated
            .finish_axes(&mut store, previous_frontier, &[], &[])
            .unwrap();
        let (mine, _) =
            assert_state_matches_oracle("A2 独立封存 oracle", &mut store, &generated, &oracle, &[]);
        assert_non_vacuous_evidence("A2 独立封存 oracle", &mine);
    }

    #[test]
    fn delta_full_foreign_expired_and_consumer_failure_revoke_without_fallback() {
        use super::super::audit_owner::RawAdvanceFixture;
        let events = delta_raw_fixture();
        let raw = RawEventsView::plain(&events).prefix(4).unwrap();
        let mut index = bj::RawPrefixIndex::new(raw);
        let edits = index.advance(4).unwrap();
        let fixture = RawAdvanceFixture::new();
        let bound = fixture.bind(raw, 0, &edits.winner_edits).unwrap();
        let mut runtime = GenerationRuntime::new(
            "A2 failure".into(),
            crate::tb02a::PROFILE,
            crate::V3_RULE_REVISION,
        );
        let mut store = MemoryNodes::new();
        let scope = json!({});
        runtime
            .generate_bound(
                &mut store,
                &bound,
                delta_bound_input(
                    &scope,
                    "cut-1",
                    1,
                    4,
                    PriorState {
                        bi_records: vec![],
                        version_record: None,
                    },
                ),
            )
            .unwrap();
        let foreign = RawAdvanceFixture::new();
        let wrong = foreign.bind(raw, 0, &edits.winner_edits).unwrap();
        assert!(runtime
            .generate_bound(
                &mut store,
                &wrong,
                delta_bound_input(
                    &scope,
                    "cut-2",
                    2,
                    4,
                    PriorState {
                        bi_records: vec![],
                        version_record: None
                    }
                )
            )
            .is_err());
        assert!(wrong.ensure_current().is_err());
        assert!(runtime.delta_lane.is_none() && runtime.raw_events_seq.seq.is_none());
        fixture.invalidate();
        assert!(runtime
            .generate_bound(
                &mut store,
                &bound,
                delta_bound_input(
                    &scope,
                    "cut-3",
                    3,
                    4,
                    PriorState {
                        bi_records: vec![],
                        version_record: None
                    }
                )
            )
            .is_err());
        // 错规则由共用身份前置核拒绝，不能留下 raw 推进或回退旧 generate。
        let fresh = RawAdvanceFixture::new();
        let next = fresh.bind(raw, 0, &edits.winner_edits).unwrap();
        let mut input = delta_bound_input(
            &scope,
            "cut-4",
            4,
            4,
            PriorState {
                bi_records: vec![],
                version_record: None,
            },
        );
        input.rule_revision = "foreign-rule";
        assert!(runtime.generate_bound(&mut store, &next, input).is_err());
        assert!(next.ensure_current().is_err());
        assert!(
            runtime.delta_lane.is_none()
                && runtime.raw_events_seq.seq.is_none()
                && runtime.objects_seq.seq.is_none()
        );
        assert_eq!(runtime.producer.source_root(2), None);
    }

    #[test]
    fn delta_full_identity_preflight_preserves_full_first_error_and_zero_write_attempts() {
        use super::super::audit_owner::RawAdvanceFixture;
        for (runtime_profile, input_profile, rule, prefix) in [
            (
                crate::tb02a::PROFILE,
                crate::tb02a::PROFILE,
                "wrong-rule",
                "IdentityConflict",
            ),
            (
                crate::tb02a::PROFILE,
                "wrong-profile",
                crate::V3_RULE_REVISION,
                "IdentityConflict",
            ),
            (
                "unsupported-profile",
                "unsupported-profile",
                crate::V3_RULE_REVISION,
                "VersionUnsupported",
            ),
        ] {
            for malformed_price in [false, true] {
                let mut events = delta_raw_fixture();
                events.truncate(4);
                if malformed_price {
                    events[0]["high"] = json!("not-a-price");
                }
                let raw = RawEventsView::plain(&events);
                let mut index = bj::RawPrefixIndex::new(raw);
                let edits = index.advance(events.len()).unwrap();
                let fixture = RawAdvanceFixture::new();
                let bound = fixture.bind(raw, 0, &edits.winner_edits).unwrap();
                let scope = json!({});
                let effective = bj::effective(raw, None).unwrap();
                let mut full = GenerationRuntime::new(
                    "A2 identity full".into(),
                    runtime_profile,
                    crate::V3_RULE_REVISION,
                );
                let mut delta = GenerationRuntime::new(
                    "A2 identity delta".into(),
                    runtime_profile,
                    crate::V3_RULE_REVISION,
                );
                let mut full_store = SequenceTestStore {
                    writes_left: Some(0),
                    ..Default::default()
                };
                let mut delta_store = SequenceTestStore {
                    writes_left: Some(0),
                    ..Default::default()
                };
                let expected = full
                    .generate(
                        &mut full_store,
                        &GenerationInput {
                            structure_prior: StructurePriors::default(),
                            all_events: raw,
                            effective: effective.raw_view(),
                            generation: 1,
                            structure_cut: "cut-1",
                            input_frontier: 4,
                            profile_id: input_profile,
                            profile_hash: "profile-hash",
                            rule_revision: rule,
                            catalog_revision: "catalog",
                            scope: &scope,
                            semantic_commit_ns: None,
                            prior: PriorState {
                                bi_records: vec![],
                                version_record: None,
                            },
                        },
                    )
                    .err()
                    .unwrap();
                let actual = delta
                    .generate_bound(
                        &mut delta_store,
                        &bound,
                        BoundGenerationInput {
                            structure_prior: StructurePriors::default(),
                            generation: 1,
                            structure_cut: "cut-1",
                            input_frontier: 4,
                            profile_id: input_profile,
                            profile_hash: "profile-hash",
                            rule_revision: rule,
                            catalog_revision: "catalog",
                            scope: &scope,
                            semantic_commit_ns: None,
                            prior: PriorState {
                                bi_records: vec![],
                                version_record: None,
                            },
                        },
                    )
                    .err()
                    .unwrap();
                assert_eq!(actual, expected, "身份首错不受坏价格或首写失败替代");
                assert!(actual.starts_with(prefix));
                assert!(full_store.writes.is_empty() && delta_store.writes.is_empty());
                assert_eq!(
                    full_store.failures + delta_store.failures,
                    0,
                    "连首写尝试都不能发生"
                );
                assert!(delta.delta_lane.is_none() && delta.raw_events_seq.seq.is_none());
                assert!(bound.ensure_current().is_err());
            }
        }
    }

    #[test]
    fn delta_full_node_write_failures_revoke_pending_generation_and_all_six_memos() {
        use super::super::audit_owner::RawAdvanceFixture;
        let events = delta_raw_fixture();
        let raw = RawEventsView::plain(&events).prefix(4).unwrap();
        let mut index = bj::RawPrefixIndex::new(raw);
        let edits = index.advance(4).unwrap();
        let scope = json!({});
        let owner = RawAdvanceFixture::new();
        let bound = owner.bind(raw, 0, &edits.winner_edits).unwrap();
        let mut counted = SequenceTestStore::default();
        let mut successful = GenerationRuntime::new(
            "A2 write count".into(),
            crate::tb02a::PROFILE,
            crate::V3_RULE_REVISION,
        );
        successful
            .generate_bound(
                &mut counted,
                &bound,
                delta_bound_input(
                    &scope,
                    "cut-1",
                    1,
                    4,
                    PriorState {
                        bi_records: vec![],
                        version_record: None,
                    },
                ),
            )
            .unwrap();
        let total = counted.writes.len();
        assert!(total > 3);
        for fail_at in [0, total / 2, total - 1] {
            let owner = RawAdvanceFixture::new();
            let bound = owner.bind(raw, 0, &edits.winner_edits).unwrap();
            let mut store = SequenceTestStore {
                writes_left: Some(fail_at),
                ..Default::default()
            };
            let mut runtime = GenerationRuntime::new(
                "A2 write refusal".into(),
                crate::tb02a::PROFILE,
                crate::V3_RULE_REVISION,
            );
            assert!(runtime
                .generate_bound(
                    &mut store,
                    &bound,
                    delta_bound_input(
                        &scope,
                        "cut-1",
                        1,
                        4,
                        PriorState {
                            bi_records: vec![],
                            version_record: None
                        }
                    )
                )
                .is_err());
            assert_eq!(store.failures, 1);
            assert!(bound.ensure_current().is_err());
            assert!(runtime.delta_lane.is_none());
            assert!([
                &runtime.raw_events_seq,
                &runtime.effective_events_seq,
                &runtime.objects_seq,
                &runtime.witnesses_seq,
                &runtime.relations_seq,
                &runtime.observations_seq
            ]
            .iter()
            .all(|memo| memo.seq.is_none() && !memo.bound));
            assert_eq!(runtime.producer.source_root(2), None);
        }
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn delta_full_owned_is_explicitly_rejected_and_lease_revoked() {
        use super::super::audit_owner::RawAdvanceFixture;
        let events = delta_raw_fixture();
        let budget = sc::capture_allocation::OwnedBudget::new(1 << 20);
        let originals = bj::RawEvents::Owned(
            sc::capture_allocation::audit_json::OwnedJson::from_view(
                (&Value::Array(events[..4].to_vec())).into(),
                &budget,
            )
            .unwrap(),
        );
        let mut index = bj::RawPrefixIndex::new(RawEventsView::plain(&events));
        let edits = index.advance(4).unwrap();
        let fixture = RawAdvanceFixture::new();
        let bound = fixture
            .bind(originals.view(), 0, &edits.winner_edits)
            .unwrap();
        let mut runtime = GenerationRuntime::new(
            "A2 Owned rejection".into(),
            crate::tb02a::PROFILE,
            crate::V3_RULE_REVISION,
        );
        let mut store = SequenceTestStore::default();
        let scope = json!({});
        let error = runtime
            .generate_bound(
                &mut store,
                &bound,
                delta_bound_input(
                    &scope,
                    "cut-1",
                    1,
                    4,
                    PriorState {
                        bi_records: vec![],
                        version_record: None,
                    },
                ),
            )
            .err()
            .unwrap();
        assert!(error.contains("prototype_audit_allocation_incomplete"));
        assert!(bound.ensure_current().is_err());
        assert!(store.writes.is_empty() && runtime.delta_lane.is_none());
        drop(bound);
        drop(originals);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    fn delta_raw_four_paths(budget: Option<&sc::capture_allocation::OwnedBudget>) {
        use super::super::audit_owner::RawAdvanceFixture;
        let events = delta_raw_fixture();
        let originals = match budget {
            None => bj::RawEvents::Plain(events.clone()),
            #[cfg(feature = "s_allocation_json_testonly")]
            Some(budget) => bj::RawEvents::Owned(
                sc::capture_allocation::audit_json::OwnedJson::from_view(
                    (&Value::Array(events.clone())).into(),
                    budget,
                )
                .unwrap(),
            ),
            #[cfg(not(feature = "s_allocation_json_testonly"))]
            Some(_) => unreachable!("无 Owned 构建不调用该分支"),
        };
        // winner edits 来自真实既有算法；Owned 只由 test fixture 把同一变化绑定原账。
        // 这不声称 Owned RawPrefixIndex / owner 生产签发已经接通。
        let mut index = bj::RawPrefixIndex::new(RawEventsView::plain(&events));
        let fixture = RawAdvanceFixture::new();
        let mut runtime = GenerationRuntime::new(
            "delta raw fixture".into(),
            crate::tb02a::PROFILE,
            crate::V3_RULE_REVISION,
        );
        let mut store = MemoryNodes::new();
        let mut previous = 0;
        for prefix in [0, 1, 2, 4, 5, 6, 8, 8, 9, 11] {
            let edits = index.advance(prefix).unwrap();
            let raw = originals.view().prefix(prefix).unwrap();
            let bound = fixture.bind(raw, previous, &edits.winner_edits).unwrap();
            let result = runtime.advance_raw_bound(&mut store, &bound).unwrap();
            // 独立 full effective 排序路径 + full JSON writer，不借 delta roots 或 parser。
            let effective = bj::effective(raw, None).unwrap();
            let mut full = MemoryNodes::new();
            let raw_root = sc::write_json_view(&mut full, JsonView::BorrowedArray(&raw)).unwrap();
            let effective_root = sc::write_json_view(&mut full, effective.view()).unwrap();
            assert_eq!(result.raw_events, raw_root);
            assert_eq!(result.effective_events, effective_root);
            let bars: Vec<Bar> = effective
                .iter()
                .map(|event| bar_of(event).unwrap())
                .collect();
            let lane = runtime.delta_lane.as_ref().unwrap();
            assert_eq!(
                lane.state().materialize_facts(),
                parser::inclusion::process_inclusion_with_facts(&bars)
            );
            assert_eq!(
                lane.rows().iter().map(|row| row.bar).collect::<Vec<_>>(),
                bars
            );
            if prefix == 5 {
                assert_eq!(result.changes.source_revision_from, Some(7));
                assert_eq!(
                    result.changes.parsed_bars, 0,
                    "同 OHLC 修订不能假称数值重放"
                );
                assert_eq!(result.changes.replayed_from, None);
            }
            if prefix == 6 {
                assert_eq!(result.changes.replayed_from, Some(2));
                assert_eq!(result.changes.parsed_bars, 2, "旧数值修订只重放变化后缀");
            }
            if prefix == 8 && previous == 8 {
                assert_eq!(result.changes.parsed_bars, 0);
                assert_eq!(result.changes.effective_dirty_from, None);
            }
            if prefix == 9 {
                assert_eq!(result.changes.replayed_from, Some(1));
                assert_eq!(result.changes.parsed_bars, 5);
            }
            if let Some(budget) = budget {
                assert!(runtime
                    .raw_events_seq
                    .allocation_budget()
                    .unwrap()
                    .same_ledger(budget));
                assert!(runtime
                    .effective_events_seq
                    .allocation_budget()
                    .unwrap()
                    .same_ledger(budget));
            }
            previous = prefix;
        }
        runtime.discard_runtime_state();
        assert!(runtime.delta_lane.is_none());
        drop(originals);
        if let Some(budget) = budget {
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            assert!(!budget.ledger_health().invariant_broken);
        }
    }

    #[test]
    fn delta_raw_plain_append_revision_swap_and_roots_match_full_oracle() {
        delta_raw_four_paths(None);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn delta_raw_owned_append_revision_swap_and_roots_match_full_oracle() {
        let budget = sc::capture_allocation::OwnedBudget::new(1 << 24);
        delta_raw_four_paths(Some(&budget));
    }

    #[test]
    fn delta_raw_expired_foreign_domain_and_rollback_cannot_reuse_pending_state() {
        use super::super::audit_owner::RawAdvanceFixture;
        let events = delta_raw_fixture();
        let mut index = bj::RawPrefixIndex::new(RawEventsView::plain(&events));
        let first = index.advance(2).unwrap();
        let fixture = RawAdvanceFixture::new();
        let mut runtime = GenerationRuntime::new(
            "delta invalidation".into(),
            crate::tb02a::PROFILE,
            crate::V3_RULE_REVISION,
        );
        let mut store = MemoryNodes::new();
        let bound = fixture
            .bind(RawEventsView::plain(&events[..2]), 0, &first.winner_edits)
            .unwrap();
        runtime.advance_raw_bound(&mut store, &bound).unwrap();
        fixture.invalidate();
        assert!(runtime.advance_raw_bound(&mut store, &bound).is_err());
        assert!(runtime.delta_lane.is_none());
        assert!(!runtime.raw_events_seq.bound && !runtime.effective_events_seq.bound);

        let fixture = RawAdvanceFixture::new();
        let bound = fixture
            .bind(RawEventsView::plain(&events[..2]), 0, &first.winner_edits)
            .unwrap();
        runtime.advance_raw_bound(&mut store, &bound).unwrap();
        let second = index.advance(4).unwrap();
        let foreign = RawAdvanceFixture::new();
        let wrong = foreign
            .bind(RawEventsView::plain(&events[..4]), 2, &second.winner_edits)
            .unwrap();
        assert!(runtime
            .advance_raw_bound(&mut store, &wrong)
            .unwrap_err()
            .contains("binding"));
        assert!(runtime.delta_lane.is_none());
        // 回滚后必须冷建且发新域；旧 fixture 下一步不能借 previous_prefix=0 假装初始。
        let stale_step = fixture
            .bind(RawEventsView::plain(&events[..2]), 0, &first.winner_edits)
            .unwrap();
        assert!(runtime.advance_raw_bound(&mut store, &stale_step).is_err());
        let cold = RawAdvanceFixture::new();
        let rebound = cold
            .bind(RawEventsView::plain(&events[..2]), 0, &first.winner_edits)
            .unwrap();
        let got = runtime.advance_raw_bound(&mut store, &rebound).unwrap();
        let mut full = MemoryNodes::new();
        let want =
            sc::write_json_view(&mut full, (&Value::Array(events[..2].to_vec())).into()).unwrap();
        assert_eq!(got.raw_events, want);
    }

    #[test]
    fn delta_raw_initial_write_failure_revokes_token_and_requires_new_owner_domain() {
        use super::super::audit_owner::RawAdvanceFixture;
        let events = delta_raw_fixture();
        let mut index = bj::RawPrefixIndex::new(RawEventsView::plain(&events));
        let changes = index.advance(2).unwrap();
        let owner = RawAdvanceFixture::new();
        let bound = owner
            .bind(RawEventsView::plain(&events[..2]), 0, &changes.winner_edits)
            .unwrap();
        let mut runtime = GenerationRuntime::new(
            "delta failure".into(),
            crate::tb02a::PROFILE,
            crate::V3_RULE_REVISION,
        );
        let mut store = SequenceTestStore::default();
        store.writes_left = Some(0);
        assert!(runtime
            .advance_raw_bound(&mut store, &bound)
            .unwrap_err()
            .contains("test_sequence_write"));
        assert!(bound.ensure_current().is_err());
        assert!(runtime.delta_lane.is_none());
        assert!(!runtime.raw_events_seq.bound && !runtime.effective_events_seq.bound);
        store.writes_left = None;
        let writes = store.writes.len();
        assert!(runtime.advance_raw_bound(&mut store, &bound).is_err());
        assert_eq!(store.writes.len(), writes, "故障旧 token 不能重入构造");
        let new_owner = RawAdvanceFixture::new();
        let cold = new_owner
            .bind(RawEventsView::plain(&events[..2]), 0, &changes.winner_edits)
            .unwrap();
        assert_ne!(bound.domain(), cold.domain());
        runtime.advance_raw_bound(&mut store, &cold).unwrap();
    }

    #[test]
    fn streamed_witness_identity_matches_normal_recipe_at_decimal_boundaries() {
        let raw = json!([{"source_coord":"0", "open":"12.5", "note":"原件"}]);
        let mut nodes = MemoryNodes::default();
        let bars = sc::write_json_view(&mut nodes, (&raw).into()).unwrap();
        for (slot, mi) in [(0usize, 0usize), (2, 127), (usize::MAX, usize::MAX)] {
            let object_id = "obj3-正常身份";
            let canonical = canonical_bytes_of_value(&json!({
                "mi": mi, "object_id": object_id, "raw_bars": raw, "slot": slot,
            }));
            let digest = Sha256::digest(&canonical);
            let expected = format!(
                "wit-{}",
                digest[..8]
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            );
            assert_eq!(
                witness_identity(&mut nodes, object_id, slot, mi, &bars).unwrap(),
                expected
            );
        }
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn sequence_keys_empty_sync_freezes_actual_mode_and_same_ledger_until_discard() {
        let budget = sc::capture_allocation::OwnedBudget::new(96);
        let other = sc::capture_allocation::OwnedBudget::new(96);
        let mut store = MemoryNodes::new();
        let mut memo = SequenceMemo::default();
        let first = memo.sync(&mut store, &[], "空keys", Some(&budget)).unwrap();
        assert!(memo.bound);
        assert!(memo.allocation_budget().unwrap().same_ledger(&budget));
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert_eq!(
            memo.sync(&mut store, &[], "空keys", Some(&budget.clone()))
                .unwrap(),
            first
        );
        for candidate in [Some(&other), None] {
            assert!(memo
                .sync(&mut store, &[], "空keys", candidate)
                .unwrap_err()
                .contains("须discard"));
        }
        memo.discard();
        memo.sync(&mut store, &[], "空keys", None).unwrap();
        assert!(memo.bound && memo.allocation_budget().is_none());
        assert!(memo
            .sync(&mut store, &[], "空keys", Some(&budget))
            .unwrap_err()
            .contains("须discard"));
        memo.discard();
        memo.sync(&mut store, &[], "空keys", Some(&other)).unwrap();
        assert!(memo.allocation_budget().unwrap().same_ledger(&other));
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn sequence_keys_owned_growth_counts_old_and_new_blocks_preserves_failure_and_drops_on_discard()
    {
        // 95/96 只锁 keys allocator 组件；sync 的调用内右脊栈另有真实分配。
        let budget = sc::capture_allocation::OwnedBudget::new(95);
        let other = sc::capture_allocation::OwnedBudget::new(96);
        let first = [1; 32];
        let second = [2; 32];
        // Owned 臂不读 store；空 MemoryNodes 只为满足 reserve_admitted 签名。
        let scratch = MemoryNodes::new();
        let mut keys = SequenceKeys::owned(&budget).unwrap();
        keys.reserve_admitted(&scratch, 1).unwrap();
        keys.replace_reserved(&[first]).unwrap();
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 32);
        let error = keys.reserve_admitted(&scratch, 2).unwrap_err();
        assert!(error.contains(sc::capture_allocation::LIMIT), "{error}");
        let probe = budget.allocation_request_probe();
        assert_eq!(probe.refused_bytes, 64);
        assert_eq!(probe.live_before_refusal, 32);
        assert_eq!(keys.as_slice(), &[first]);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 32);
        assert!(keys.allocation_budget().unwrap().same_ledger(&budget));
        drop(keys);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let mut keys = SequenceKeys::owned(&other).unwrap();
        keys.reserve_admitted(&scratch, 1).unwrap();
        keys.replace_reserved(&[first]).unwrap();
        keys.reserve_admitted(&scratch, 2).unwrap();
        keys.replace_reserved(&[first, second]).unwrap();
        assert_eq!(other.snapshot().live_requested_layout_bytes, 64);
        assert_eq!(other.snapshot().peak_requested_layout_bytes, 96);
        keys.replace_reserved(&[]).unwrap();
        assert_eq!(
            other.snapshot().live_requested_layout_bytes,
            64,
            "clear保留容量"
        );
        let mut memo = SequenceMemo {
            keys,
            seq: None,
            bound: true,
        };
        memo.discard();
        assert_eq!(other.snapshot().live_requested_layout_bytes, 0);
        println!("sequence keys component actual Layout old=32 new=64 peak=96; limit95 refused; discard live=0");
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn sequence_sync_rebuild_stack_uses_keys_ledger_and_propagates_refusal_until_discard() {
        let budget = sc::capture_allocation::OwnedBudget::new(96);
        let first = [1; 32];
        let second = [2; 32];
        let mut store = MemoryNodes::new();
        let root = sc::write_sequence(&mut store, &[first]).unwrap();
        let mut keys = SequenceKeys::owned(&budget).unwrap();
        // 预留两项，隔离调用内右脊栈拒绝，避免先卡在 keys 增长。
        keys.reserve_admitted(&store, 2).unwrap();
        keys.replace_reserved(&[first]).unwrap();
        let original = SequenceRef::from_parts_unchecked(root, 1);
        let mut memo = SequenceMemo {
            keys,
            seq: Some(original),
            bound: true,
        };
        let error = memo
            .sync(&mut store, &[first, second], "真实memo", Some(&budget))
            .unwrap_err();
        assert!(error.contains(sc::capture_allocation::LIMIT), "{error}");
        let probe = budget.allocation_request_probe();
        assert_eq!(
            probe.refused_kind,
            std::any::type_name::<(NodeDigest, u64)>()
        );
        assert_eq!(probe.live_before_refusal, 64);
        assert!(probe.refused_bytes > 32);
        assert_eq!(memo.seq, Some(original));
        assert_eq!(memo.keys.as_slice(), &[first]);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 64);
        // 首次失败可能已经写过node，沿既有事务/Runtime discard边界处理。
        memo.discard();
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert!(!memo.bound);
        let generous = sc::capture_allocation::OwnedBudget::new(1 << 20);
        memo.sync(&mut store, &[first], "真实memo", Some(&generous))
            .unwrap();
        let result = memo
            .sync(&mut store, &[first, second], "真实memo", Some(&generous))
            .unwrap();
        let mut plain = SequenceMemo::default();
        let oracle = plain
            .sync(&mut store, &[first, second], "普通memo", None)
            .unwrap();
        assert_eq!(result, oracle);
        assert_eq!(generous.snapshot().live_requested_layout_bytes, 64);
        assert!(generous.snapshot().peak_requested_layout_bytes > 96);
        memo.discard();
        assert_eq!(generous.snapshot().live_requested_layout_bytes, 0);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn sequence_keys_first_reserve_or_node_failure_does_not_freeze_ledger_and_plain_discard_reuses_capacity(
    ) {
        struct Fails;
        impl sc::NodeSource for Fails {
            fn read(&mut self, _: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
                Ok(sc::NodeRead::Missing)
            }
        }
        impl sc::NodeSink for Fails {
            fn put(&mut self, _: NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
                Err(sc::CodecError::new(
                    "test_sequence_node_write",
                    "首次节点写失败",
                ))
            }
        }
        let mut store = MemoryNodes::new();
        let digest = sc::write_value(&mut store, &CValue::Bool(true))
            .unwrap()
            .digest;
        let short = sc::capture_allocation::OwnedBudget::new(31);
        let budget = sc::capture_allocation::OwnedBudget::new(1 << 20);
        let other = sc::capture_allocation::OwnedBudget::new(1 << 20);
        let mut memo = SequenceMemo::default();
        assert!(memo
            .sync(&mut store, &[digest], "首次keys", Some(&short))
            .unwrap_err()
            .contains(sc::capture_allocation::LIMIT));
        assert!(!memo.bound && memo.seq.is_none() && memo.keys.as_slice().is_empty());
        assert_eq!(short.snapshot().live_requested_layout_bytes, 0);
        assert!(memo
            .sync(&mut Fails, &[digest], "首次keys", Some(&budget))
            .unwrap_err()
            .contains("test_sequence_node_write"));
        assert!(!memo.bound && memo.seq.is_none() && memo.allocation_budget().is_none());
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        memo.sync(&mut store, &[digest], "首次keys", Some(&other))
            .unwrap();
        assert!(memo.allocation_budget().unwrap().same_ledger(&other));
        memo.discard();
        memo.sync(&mut store, &[digest], "普通keys", None).unwrap();
        let capacity = match &memo.keys {
            SequenceKeys::Plain(keys) => keys.capacity(),
            _ => unreachable!(),
        };
        memo.discard();
        assert!(
            matches!(&memo.keys, SequenceKeys::Plain(keys) if keys.capacity() == capacity && keys.is_empty())
        );
        memo.sync(&mut store, &[digest], "普通keys", None).unwrap();
        assert!(matches!(&memo.keys, SequenceKeys::Plain(keys) if keys.capacity() == capacity));
    }

    #[cfg(not(feature = "s_allocation_json_testonly"))]
    #[test]
    fn sequence_keys_some_without_owned_build_refuses_plain_fallback() {
        let mut memo = SequenceMemo::default();
        let budget = sc::capture_allocation::OwnedBudget::new(32);
        let error = memo
            .sync(&mut MemoryNodes::new(), &[], "缺构建keys", Some(&budget))
            .unwrap_err();
        assert!(error.contains("拒绝Plain兜底"));
        assert!(!memo.bound && memo.seq.is_none());
    }

    #[derive(Default)]
    struct SequenceTestStore {
        nodes: MemoryNodes,
        writes: Vec<NodeDigest>,
        reject_reads: bool,
        writes_left: Option<usize>,
        failures: usize,
    }
    impl sc::NodeSource for SequenceTestStore {
        fn read(&mut self, digest: &NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
            if self.reject_reads {
                self.failures += 1;
                return Err(sc::CodecError::new(
                    "test_sequence_read",
                    "真实 memo 读故障",
                ));
            }
            sc::NodeSource::read(&mut self.nodes, digest)
        }
    }
    impl sc::NodeSink for SequenceTestStore {
        fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), sc::CodecError> {
            if let Some(left) = &mut self.writes_left {
                if *left == 0 {
                    self.failures += 1;
                    return Err(sc::CodecError::new(
                        "test_sequence_write",
                        "真实 memo 写故障",
                    ));
                }
                *left -= 1;
            }
            sc::NodeSink::put(&mut self.nodes, digest, tag, body)?;
            self.writes.push(digest);
            Ok(())
        }
    }

    // 只沿数组与 M 树边遍历，叶值摘要仍按内容序列 API 的不透明值口径。
    fn sequence_array_closure(
        nodes: &MemoryNodes,
        root: NodeDigest,
    ) -> std::collections::HashSet<NodeDigest> {
        let mut seen = std::collections::HashSet::new();
        let mut todo = vec![root];
        while let Some(digest) = todo.pop() {
            if !seen.insert(digest) {
                continue;
            }
            let (tag, body) = nodes.get(&digest).expect("结果闭包不能缺节点");
            match tag {
                sc::tag::ARRAY => todo.push(body[8..40].try_into().unwrap()),
                sc::tag::SEQ_BRANCH => {
                    todo.push(body[8..40].try_into().unwrap());
                    todo.push(body[40..72].try_into().unwrap());
                }
                sc::tag::SEQ_LEAF | sc::tag::SEQ_EMPTY => {}
                _ => panic!("数组序列闭包出现非序列节点"),
            }
        }
        seen
    }

    fn sequence_memo_full_oracle_transitions(budget: Option<&sc::capture_allocation::OwnedBudget>) {
        let base: Vec<NodeDigest> = (1..=31).map(|n| [n; 32]).collect();
        let mut middle = base.clone();
        middle[9..].fill([101; 32]);
        let mut tail = middle.clone();
        tail[28..].fill([102; 32]);
        let all_new = vec![[201; 32]; 6];
        // 首次空/非空、追加、原样、中段/近尾替换、截短、全替换、清空、清空后重建。
        let stages = [
            vec![],
            base[..1].to_vec(),
            base[..5].to_vec(),
            base.clone(),
            base.clone(),
            middle,
            tail.clone(),
            tail[..17].to_vec(),
            all_new,
            vec![],
            base[..8].to_vec(),
        ];
        let mut memo = SequenceMemo::default();
        let mut store = SequenceTestStore::default();
        for elements in stages {
            let before: std::collections::HashSet<_> =
                store.nodes.iter().map(|(d, _, _)| *d).collect();
            let previous_keys = memo.keys.as_slice().to_vec();
            let old_sequence = memo.seq;
            let unchanged = memo.bound && previous_keys == elements;
            store.writes.clear();
            let got = memo
                .sync(&mut store, &elements, "真实 memo 转换", budget)
                .unwrap();
            // 独立全量 writer 不经过 SequenceMemo/rebuild，锁住 layout/1 的旧编码。
            let mut oracle = MemoryNodes::new();
            let root = sc::write_sequence(&mut oracle, &elements).unwrap();
            let expected_seq = SequenceRef::from_parts_unchecked(root, elements.len() as u64);
            let expected = array_from_sequence(&mut oracle, &expected_seq).unwrap();
            assert_eq!(got, expected);
            assert_eq!(memo.seq, Some(expected_seq));
            assert_eq!(memo.keys.as_slice(), elements.as_slice());
            let closure = sequence_array_closure(&store.nodes, got.digest);
            for digest in &store.writes {
                assert!(closure.contains(digest), "本次写入不准留下作废中间根");
            }
            for (digest, _, _) in store.nodes.iter().filter(|(d, _, _)| !before.contains(*d)) {
                assert!(closure.contains(digest), "新增节点须全部可达");
            }
            if unchanged {
                assert_eq!(memo.seq, old_sequence);
                assert!(store.nodes.iter().all(|(d, _, _)| before.contains(d)));
            }
            if let Some(budget) = budget {
                assert!(memo.allocation_budget().unwrap().same_ledger(budget));
                let capacity = match &memo.keys {
                    SequenceKeys::Plain(keys) => keys.capacity(),
                    #[cfg(feature = "s_allocation_json_testonly")]
                    SequenceKeys::Owned(keys) => keys.capacity(),
                };
                assert_eq!(
                    budget.snapshot().live_requested_layout_bytes,
                    capacity * std::mem::size_of::<NodeDigest>()
                );
            }
        }
        memo.discard();
        assert!(!memo.bound && memo.seq.is_none() && memo.keys.as_slice().is_empty());
        if let Some(budget) = budget {
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            assert!(!budget.ledger_health().invariant_broken);
        }
    }

    #[test]
    fn sequence_sync_plain_matches_full_oracle_and_writes_only_reachable() {
        sequence_memo_full_oracle_transitions(None);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn sequence_sync_owned_matches_full_oracle_and_writes_only_reachable() {
        let budget = sc::capture_allocation::OwnedBudget::new(1 << 20);
        sequence_memo_full_oracle_transitions(Some(&budget));
        assert!(budget.snapshot().allocated_layouts > 0);
    }

    fn sequence_memo_failure_preserves_state(budget: Option<&sc::capture_allocation::OwnedBudget>) {
        let base: Vec<NodeDigest> = (1..=9).map(|n| [n; 32]).collect();
        let mut changed = base.clone();
        changed[3..].fill([200; 32]);
        let mut memo = SequenceMemo::default();
        let mut store = SequenceTestStore::default();
        memo.sync(&mut store, &base, "故障前", budget).unwrap();
        let seq = memo.seq;
        let live = budget.map(|b| b.snapshot().live_requested_layout_bytes);
        // 首读拒绝、已写一个叶后拒绝、最后数组节点拒绝三种实际边界。
        let mut probe = SequenceTestStore::default();
        let mut probe_memo = SequenceMemo::default();
        probe_memo.sync(&mut probe, &base, "计写", None).unwrap();
        probe.writes.clear();
        probe_memo.sync(&mut probe, &changed, "计写", None).unwrap();
        let before_array = probe.writes.len() - 1;
        for (reject_reads, writes_left, error_code) in [
            (true, None, "test_sequence_read"),
            (false, Some(1), "test_sequence_write"),
            (false, Some(before_array), "test_sequence_write"),
        ] {
            store.reject_reads = reject_reads;
            store.writes_left = writes_left;
            store.failures = 0;
            let error = memo
                .sync(&mut store, &changed, "故障代", budget)
                .unwrap_err();
            assert!(error.contains(error_code), "{error}");
            assert_eq!(store.failures, 1, "第一次错误直接返回，不重试全量");
            assert!(memo.bound);
            assert_eq!(memo.seq, seq);
            assert_eq!(memo.keys.as_slice(), base.as_slice());
            assert_eq!(
                budget.map(|b| b.snapshot().live_requested_layout_bytes),
                live
            );
        }
        store.reject_reads = false;
        store.writes_left = None;
        // 缓存损坏同样拒绝，不能全量重建掩盖旧状态失配。
        memo.seq = Some(SequenceRef::from_parts_unchecked(seq.unwrap().root(), 1));
        store.writes.clear();
        assert!(memo
            .sync(&mut store, &changed, "失配", budget)
            .unwrap_err()
            .contains("计数不一致"));
        assert!(store.writes.is_empty());
        assert_eq!(memo.keys.as_slice(), base.as_slice());
        memo.discard();
        if let Some(budget) = budget {
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }

    #[test]
    fn sequence_sync_plain_errors_preserve_keys_and_root_without_full_fallback() {
        sequence_memo_failure_preserves_state(None);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn sequence_sync_owned_errors_release_stack_and_preserve_keys_and_root() {
        let budget = sc::capture_allocation::OwnedBudget::new(1 << 20);
        sequence_memo_failure_preserves_state(Some(&budget));
    }

    fn delta_splice_matches_full_writer(budget: Option<&sc::capture_allocation::OwnedBudget>) {
        let values: Vec<NodeDigest> = (1..=40).map(|n| [n; 32]).collect();
        let mut memo = SequenceMemo::default();
        let mut store = SequenceTestStore::default();
        let mut expected = Vec::new();
        // 外部输入是明确 rank edit，不先建完整 next keys 去找公共前缀。
        let mut edits: Vec<(usize, Vec<NodeDigest>)> = vec![(0, vec![])];
        let mut length = 0;
        for next in [1, 2, 3, 4, 7, 8, 9, 15, 16, 17, 31, 32, 33] {
            edits.push((length, values[length..next].to_vec()));
            length = next;
        }
        edits.extend([
            (33, vec![]),
            (9, vec![[101; 32]; 12]),    // 中段替换/缩短。
            (5, values[5..17].to_vec()), // 旧子树再激活。
            (7, vec![[102; 32]; 20]),    // 中段插入导致后缀重新布局。
            (4, vec![]),
            (0, vec![]),
            (0, values[..9].to_vec()),
        ]);
        for (prefix, suffix) in edits {
            let previous = memo.seq;
            let got = memo
                .sync_splice(
                    &mut store,
                    SequenceSplice {
                        expected_previous: previous,
                        unchanged_prefix: prefix,
                        replacement_suffix: &suffix,
                    },
                    "delta rank edit",
                    budget,
                )
                .unwrap();
            expected.truncate(prefix);
            expected.extend_from_slice(&suffix);
            let mut full = MemoryNodes::new();
            let root = sc::write_sequence(&mut full, &expected).unwrap();
            let sequence = SequenceRef::from_parts_unchecked(root, expected.len() as u64);
            let oracle = array_from_sequence(&mut full, &sequence).unwrap();
            assert_eq!(got, oracle, "splice 必须保留 layout/1 完整根");
            assert_eq!(memo.seq, Some(sequence));
            assert_eq!(memo.keys.as_slice(), expected.as_slice());
            if let Some(budget) = budget {
                assert!(memo.allocation_budget().unwrap().same_ledger(budget));
            }
        }
        memo.discard();
        if let Some(budget) = budget {
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            assert!(!budget.ledger_health().invariant_broken);
        }
    }

    #[test]
    fn delta_sequence_plain_rank_splice_matches_independent_full_writer() {
        delta_splice_matches_full_writer(None);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn delta_sequence_owned_rank_splice_matches_independent_full_writer() {
        let budget = sc::capture_allocation::OwnedBudget::new(1 << 20);
        delta_splice_matches_full_writer(Some(&budget));
    }

    fn delta_splice_failures(budget: Option<&sc::capture_allocation::OwnedBudget>) {
        let base: Vec<NodeDigest> = (1..=17).map(|n| [n; 32]).collect();
        let suffix = [[201; 32]; 12];
        let mut memo = SequenceMemo::default();
        let mut store = SequenceTestStore::default();
        memo.sync(&mut store, &base, "初始化", budget).unwrap();
        let previous = memo.seq;
        let mut probe = SequenceMemo::default();
        let mut probe_store = SequenceTestStore::default();
        probe.sync(&mut probe_store, &base, "计写", None).unwrap();
        probe_store.writes.clear();
        probe
            .sync_splice(
                &mut probe_store,
                SequenceSplice {
                    expected_previous: previous,
                    unchanged_prefix: 5,
                    replacement_suffix: &suffix,
                },
                "计写",
                None,
            )
            .unwrap();
        // 从第一节点到最后 ARRAY 的每个真实写点，以及共享前缀读取失败。
        for reject in 0..=probe_store.writes.len() {
            store.reject_reads = reject == probe_store.writes.len();
            store.writes_left = (!store.reject_reads).then_some(reject);
            store.failures = 0;
            let error = memo
                .sync_splice(
                    &mut store,
                    SequenceSplice {
                        expected_previous: previous,
                        unchanged_prefix: 5,
                        replacement_suffix: &suffix,
                    },
                    "故障",
                    budget,
                )
                .unwrap_err();
            assert!(error.contains("test_sequence_"), "{error}");
            assert_eq!(store.failures, 1, "失败直接返回，不能全量 fallback");
            assert_eq!(memo.seq, previous);
            assert_eq!(memo.keys.as_slice(), base.as_slice());
        }
        store.reject_reads = false;
        store.writes_left = None;
        // 旧根/旧长度/越界前缀拒绝在任何写入之前。
        for (old, prefix) in [(None, 5), (previous, 18)] {
            store.writes.clear();
            assert!(memo
                .sync_splice(
                    &mut store,
                    SequenceSplice {
                        expected_previous: old,
                        unchanged_prefix: prefix,
                        replacement_suffix: &[],
                    },
                    "错旧态",
                    budget,
                )
                .is_err());
            assert!(store.writes.is_empty());
            assert_eq!(memo.seq, previous);
            assert_eq!(memo.keys.as_slice(), base.as_slice());
        }
        memo.discard();
        if let Some(budget) = budget {
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }

    #[test]
    fn delta_sequence_plain_failures_do_not_advance_keys_or_root() {
        delta_splice_failures(None);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn delta_sequence_owned_failures_do_not_advance_keys_or_root() {
        let budget = sc::capture_allocation::OwnedBudget::new(1 << 20);
        delta_splice_failures(Some(&budget));
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn delta_sequence_foreign_budget_and_allocation_failure_preserve_binding() {
        let budget = sc::capture_allocation::OwnedBudget::new(1 << 20);
        let foreign = sc::capture_allocation::OwnedBudget::new(1 << 20);
        let empty = sc::capture_allocation::OwnedBudget::new(0);
        let mut memo = SequenceMemo::default();
        let mut store = SequenceTestStore::default();
        let suffix = [[8; 32]; 8];
        assert!(memo
            .sync_splice(
                &mut store,
                SequenceSplice {
                    expected_previous: None,
                    unchanged_prefix: 0,
                    replacement_suffix: &suffix,
                },
                "零预算",
                Some(&empty),
            )
            .is_err());
        assert!(!memo.bound && memo.seq.is_none() && memo.keys.as_slice().is_empty());
        assert_eq!(empty.snapshot().live_requested_layout_bytes, 0);
        memo.sync(&mut store, &suffix, "绑定", Some(&budget))
            .unwrap();
        let previous = memo.seq;
        for wrong in [None, Some(&foreign)] {
            store.writes.clear();
            assert!(memo
                .sync_splice(
                    &mut store,
                    SequenceSplice {
                        expected_previous: previous,
                        unchanged_prefix: 8,
                        replacement_suffix: &[],
                    },
                    "错账本",
                    wrong,
                )
                .unwrap_err()
                .contains("缓存账本改变"));
            assert_eq!(memo.seq, previous);
            assert_eq!(memo.keys.as_slice(), suffix.as_slice());
            assert!(store.writes.is_empty());
        }
        memo.discard();
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert_eq!(foreign.snapshot().live_requested_layout_bytes, 0);
    }

    /// #1394 整合 W-3：合并后 oracle 是 main 的 v2 写者，它在 TB-02-A/B 之外另发布 TB-02-C/D
    /// （`tb02c::KINDS` 对象及其关系边）并写 17 轴目录；v3 只发布 TB-02-A/B、写 13 轴目录。
    fn is_tb02ab_object(object: &Value) -> bool {
        !object["kind"]
            .as_str()
            .is_some_and(|kind| crate::tb02c::KINDS.contains(&kind))
    }

    /// 已存身份读集限制到 TB-02-A/B（与 [`restrict_oracle_to_tb02ab`] 同一对象口径）。
    fn tb02ab_stored(stored: StoredObjects) -> StoredObjects {
        StoredObjects {
            active: stored.active.into_iter().filter(is_tb02ab_object).collect(),
            identities: stored
                .identities
                .into_iter()
                .filter(|(_, object)| is_tb02ab_object(object))
                .collect(),
        }
    }

    /// 把 oracle 封存批次限制到 TB-02-A/B：
    ///
    /// - `objects` 去掉 `tb02c::KINDS` 的对象（其余对象次序不变）；
    /// - `relations` 去掉与被去掉的 `CC-055.segment_relation` 对象逐项同三元组的关系边
    ///   （tb02c 每发一条关系边就同时发一条同 source/kind/target 的见证对象）；
    /// - `catalog_axes` 在限制后的对象上用 v3 那张 13 轴表重算，withdrawals/replaces 由调用方
    ///   在限制后的对象与已存身份上跑同一生产 diff 得到（旧身份空间）。目录函数本就与生产共用
    ///   （见模块头），对拍的仍是「同一批输入」；不重算就等于拿 17 轴目录对 13 轴写者。
    /// - 投影的规则标识显式映射到历史 V3 A/B 域；不改封存库或 raw 原件。
    ///   生产目录现在由规则域选表，保留 V2 标识会错误地给 A/B 投影选择 17 轴。
    fn restrict_oracle_to_tb02ab(
        oracle: &mut OracleGeneration,
        previous_frontier: i64,
        withdrawals: &[Value],
        replaces: &[Value],
    ) {
        assert_eq!(
            oracle.batch["rule_revision"],
            crate::RULE_REVISION,
            "A/B oracle 投影只接受真实 V2 源域"
        );
        let objects = oracle.batch["objects"].as_array().expect("oracle objects");
        let mut segment_edges = std::collections::BTreeSet::new();
        for object in objects {
            if object["kind"] == "CC-055.segment_relation" {
                let payload: Value = serde_json::from_str(
                    object["payload_json"].as_str().expect("段关系载荷是文本"),
                )
                .expect("段关系载荷 JSON");
                let data = &payload["data"];
                segment_edges.insert(canonical_bytes_of_value(&json!({
                    "subject": data["source_id"],
                    "relation_type": data["relation_kind"],
                    "object": data["target_id"],
                })));
            }
        }
        let kept: Vec<Value> = objects
            .iter()
            .filter(|object| is_tb02ab_object(object))
            .cloned()
            .collect();
        let relations: Vec<Value> = oracle.batch["relations"]
            .as_array()
            .expect("oracle relations")
            .iter()
            .filter(|relation| !segment_edges.contains(&canonical_bytes_of_value(relation)))
            .cloned()
            .collect();
        let axes = crate::tb02a_facts::catalog_axes(
            crate::tb02a_facts::CatalogAxisTable::NewBi,
            &kept,
            &oracle.all_events,
            previous_frontier,
            withdrawals,
            replaces,
        );
        oracle.batch["objects"] = Value::Array(kept);
        oracle.batch["relations"] = Value::Array(relations);
        oracle.batch["catalog_axes"] = axes;
        oracle.batch["rule_revision"] = json!(crate::V3_RULE_REVISION);
    }

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
            // #1527 M-03：oracle 只测业务/持久态，走原 core，不建立生产准入。
            crate::tests::advance_business(&self.files.db(), DEFAULT_WRITER_EPOCH)
                .expect("oracle advance");
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
            structure_prior: StructurePriors::default(),
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
        let generated = runtime
            .generate(&mut *store, &input)
            .unwrap_or_else(|error| panic!("S3 生成失败：{error}"));
        assert_eq!(generated.policy_records, runtime.policy_records());
        generated
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

    /// 逐字段对拍：先按 restrict_oracle_to_tb02ab 明示投影 A/B 对象、目录与规则域，
    /// 再把旧 `obj-`/`wit-` 身份按顺序映射到本域真实新身份；原始两账与业务载荷不变，
    /// 最后核映射后的规范字节逐字节相等。
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
                new.witness_id.as_str()
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
                new.witness_id.as_str(),
                "{label}: 见证{index} 同配方 + 新 obj3 输入"
            );
            assert_eq!(
                old["raw_bars"], mine_witness["raw_bars"],
                "{label}: 见证{index} 完整 raw_bars"
            );
            map.push((old_id.to_string(), new.witness_id.as_str().to_owned()));
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
            let mut oracle = oracle_db.read();
            restrict_oracle_to_tb02ab(&mut oracle, previous_frontier, &[], &[]);
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
        let mut oracle = oracle_db.read();
        restrict_oracle_to_tb02ab(&mut oracle, previous_frontier, &[], &[]);
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
        let mut oracle_first = oracle_db.read();
        restrict_oracle_to_tb02ab(&mut oracle_first, first_frontier, &[], &[]);
        assert_eq!(oracle_first.generation, 1);
        // 第二次 advance 之前抓旧状态身份读集：与旧目录轴调用同一读面。
        let conn = open_db(&oracle_db.files.db()).expect("打开 oracle 库");
        let mode = delta_storage::read_mode(&conn).expect("读 Delta 域模式");
        let stored_before = tb02ab_stored(read_stored_objects(&conn).expect("读已存身份"));
        drop(conn);
        oracle_db.accept(&rows[..14]);
        let second_frontier = oracle_db.advance();
        let mut oracle_second = oracle_db.read();
        {
            let objects: Vec<Value> = oracle_second.batch["objects"]
                .as_array()
                .expect("gen2 objects")
                .iter()
                .filter(|object| is_tb02ab_object(object))
                .cloned()
                .collect();
            let (_, withdrawals, replaces) =
                compute_diff(&objects, &stored_before, mode).expect("生产 diff（TB-02-A/B）");
            restrict_oracle_to_tb02ab(&mut oracle_second, second_frontier, &withdrawals, &replaces);
        }
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

    // ── #1491 ADR 0037 段 B：bounded-layout/1 的 CC-007 单元批记录与 CC-054 请求次序 ──

    fn full_input<'a>(
        raw: RawEventsView<'a>,
        effective: RawEventsView<'a>,
        scope: &'a Value,
        cut: &'a str,
        gen: i64,
        frontier: i64,
        rule: &'a str,
    ) -> GenerationInput<'a, 'static> {
        GenerationInput {
            structure_prior: StructurePriors::default(),
            all_events: raw,
            effective,
            generation: gen,
            structure_cut: cut,
            input_frontier: frontier,
            profile_id: crate::tb02a::PROFILE,
            profile_hash: "profile-hash",
            rule_revision: rule,
            catalog_revision: "catalog",
            scope,
            semantic_commit_ns: Some("123456789"),
            prior: PriorState {
                bi_records: Vec::new(),
                version_record: None,
            },
        }
    }

    /// 修订组输入：早期 bar 改价修订、再恢复、尾 bar 换坐标（原 25 根 revision 组与布局锁共用）。
    fn revision_events() -> Vec<Value> {
        let mut events = delta_long_fixture();
        events.truncate(14);
        let mut revised = events[2].clone();
        for (k, v) in [
            ("revision", json!(2)),
            ("input_revision", json!("2")),
            ("seq", json!(15)),
            ("high", json!("13")),
            ("low", json!("9")),
            ("open", json!("11")),
            ("close", json!("11")),
            ("payload_hash", json!("revision-2")),
            ("receipt_id", json!("receipt-15")),
        ] {
            revised[k] = v;
        }
        events.push(revised);
        let mut restored = events[2].clone();
        for (k, v) in [
            ("revision", json!(3)),
            ("input_revision", json!("3")),
            ("seq", json!(16)),
            ("payload_hash", json!("restored-3")),
            ("receipt_id", json!("receipt-16")),
        ] {
            restored[k] = v;
        }
        events.push(restored);
        let mut moved = events[13].clone();
        for (k, v) in [
            ("revision", json!(2)),
            ("input_revision", json!("2")),
            ("seq", json!(17)),
            ("source_coord", json!("1000")),
            ("ts", json!("1000")),
            ("payload_hash", json!("tail-moved")),
            ("receipt_id", json!("receipt-17")),
        ] {
            moved[k] = v;
        }
        events.push(moved);
        events
    }

    /// 每个前缀一代：layout/1 增量、bounded 增量各一条车道，另以全新 bounded 运行态冷算同一代。
    /// 返回逐代（layout/1 生成态 JSON，bounded 生成态 JSON）；bounded 增量 ≡ 冷算逐字节在此核。
    fn bounded_layout_runs(events: &[Value], cuts: &[usize]) -> Vec<(Value, Value)> {
        bounded_layout_runs_with(events, cuts, "r")
    }

    fn bounded_layout_runs_with(
        events: &[Value],
        cuts: &[usize],
        rule: &str,
    ) -> Vec<(Value, Value)> {
        let bounded_rule = format!("{rule}+bounded-layout/1");
        let bounded_rule = bounded_rule.as_str();
        let scope = json!({"instrument":"B1", "exchange":"fixture"});
        let mut sorted = GenerationRuntime::new("B1 sorted".into(), crate::tb02a::PROFILE, rule);
        let mut bounded =
            GenerationRuntime::new("B1 bounded".into(), crate::tb02a::PROFILE, bounded_rule);
        let (mut ss, mut bs) = (MemoryNodes::new(), MemoryNodes::new());
        let mut out = Vec::new();
        for (turn, &prefix) in cuts.iter().enumerate() {
            let gen = turn as i64 + 1;
            let cut = format!("cut-{gen}");
            let raw = RawEventsView::plain(events).prefix(prefix).unwrap();
            let effective = bj::effective(raw, None).unwrap();
            let input = |rule| {
                full_input(
                    raw,
                    effective.raw_view(),
                    &scope,
                    &cut,
                    gen,
                    prefix as i64,
                    rule,
                )
            };
            let s = sorted.generate(&mut ss, &input(rule)).unwrap();
            let b = bounded.generate(&mut bs, &input(bounded_rule)).unwrap();
            let mut cold_runtime = GenerationRuntime::new(
                format!("B1 bounded cold {gen}"),
                crate::tb02a::PROFILE,
                bounded_rule,
            );
            let mut cs = MemoryNodes::new();
            let c = cold_runtime
                .generate(&mut cs, &input(bounded_rule))
                .unwrap();
            assert_eq!(
                b.state_without_axes, c.state_without_axes,
                "bounded 增量≠冷算：第 {gen} 代"
            );
            b.verify_new_closure(&mut bs).unwrap();
            let (_, sv) = state_value(&mut ss, &s);
            let (b_text, bv) = state_value(&mut bs, &b);
            let (c_text, _) = state_value(&mut cs, &c);
            assert_eq!(b_text, c_text, "bounded 增量≠冷算规范字节：第 {gen} 代");
            out.push((sv, bv));
        }
        out
    }

    fn bounded2_layout_runs(events: &[Value], cuts: &[usize]) -> Vec<(Value, Value)> {
        let run = |layout| {
            super::super::layout::with_forced_layout_for_test(Some(layout), || {
                let rule = crate::W_STRUCTURE_RULE_REVISION;
                let scope = json!({"instrument":"B1", "exchange":"fixture"});
                let mut runtime = GenerationRuntime::new(
                    "bounded2-layout-lock".into(),
                    crate::tb02a::PROFILE,
                    rule,
                );
                let mut nodes = MemoryNodes::new();
                cuts.iter()
                    .enumerate()
                    .map(|(turn, &prefix)| {
                        let gen = turn as i64 + 1;
                        let cut = format!("cut-{gen}");
                        let raw = RawEventsView::plain(events).prefix(prefix).unwrap();
                        let effective = bj::effective(raw, None).unwrap();
                        let input = full_input(
                            raw,
                            effective.raw_view(),
                            &scope,
                            &cut,
                            gen,
                            prefix as i64,
                            rule,
                        );
                        let generated = runtime.generate(&mut nodes, &input).unwrap();
                        let mut cold = GenerationRuntime::new(
                            format!("bounded2-cold-{gen}"),
                            crate::tb02a::PROFILE,
                            rule,
                        );
                        let mut cold_nodes = MemoryNodes::new();
                        let cold_generated = cold.generate(&mut cold_nodes, &input).unwrap();
                        assert_eq!(
                            state_value(&mut nodes, &generated).0,
                            state_value(&mut cold_nodes, &cold_generated).0
                        );
                        state_value(&mut nodes, &generated).1
                    })
                    .collect::<Vec<_>>()
            })
        };
        run(super::super::layout::Layout::Sorted)
            .into_iter()
            .zip(run(super::super::layout::Layout::Bounded2))
            .collect()
    }

    fn assert_bounded2_crosswalk(gen: usize, sorted: &Value, bounded: &Value, raw: &[Value]) {
        let mut winners = std::collections::BTreeMap::<String, &Value>::new();
        for event in raw {
            let identity = event["identity_key"].as_str().unwrap().to_string();
            let replace = winners.get(&identity).is_none_or(|old| {
                (
                    event["revision"].as_i64().unwrap(),
                    event["seq"].as_i64().unwrap(),
                ) > (
                    old["revision"].as_i64().unwrap(),
                    old["seq"].as_i64().unwrap(),
                )
            });
            if replace {
                winners.insert(identity, event);
            }
        }
        let mut expected = sorted.clone();
        for object in expected["objects"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .filter(|o| o["kind"] == CC007_FRACTAL_KIND)
        {
            let mut payload: Value =
                serde_json::from_str(object["payload_json"].as_str().unwrap()).unwrap();
            for unit in payload["subsequent_development"]["bars"]
                .as_array_mut()
                .unwrap()
            {
                assert_eq!(unit.as_object().unwrap().len(), 2);
                let event = winners
                    .values()
                    .find(|e| e["source_coord"] == unit["bar"]["source_coord"])
                    .unwrap();
                let mut source = serde_json::Map::new();
                for name in [
                    "identity_key",
                    "payload_hash",
                    "receipt_id",
                    "event_id",
                    "input_revision",
                    "revision",
                    "seq",
                    "source_coord",
                ] {
                    source.insert(
                        name.to_string(),
                        json!(event[name]
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| event[name].to_string())),
                    );
                }
                unit["source_ref"] = Value::Object(source);
            }
            object["payload_json"] = json!(crate::canonical_json(&payload));
        }
        assert_bounded_crosswalk(gen, &expected, bounded);
        for kind in [
            CC004_STEP_KIND,
            CC005_GROUP_KIND,
            crate::content_producer::CC004_INITIALIZATION_KIND,
            crate::content_producer::CC004_UNDETERMINED_KIND,
        ] {
            let content = |state: &Value| {
                objects_of(state, kind)
                    .into_iter()
                    .map(|o| {
                        json!([
                            o["fact_key"],
                            o["payload"],
                            o["input_refs"],
                            o["source_coords"]
                        ])
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                content(sorted),
                content(bounded),
                "g{gen} {kind} 其余内容不变"
            );
        }
        let unchanged = |state: &Value| {
            state["objects"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|o| {
                    ![
                        CC007_FRACTAL_KIND,
                        CC007_DEVELOPMENT_BATCH_KIND,
                        CC054_KNOWLEDGE_KIND,
                    ]
                    .contains(&o["kind"].as_str().unwrap())
                })
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(
            unchanged(sorted),
            unchanged(bounded),
            "g{gen} 其余完整事实记录不变"
        );
        for (field, value) in sorted.as_object().unwrap() {
            if field != "objects" {
                assert_eq!(value, &bounded[field], "g{gen} 其余生成态字段 {field}");
            }
        }
        assert_eq!(
            w_request_semantics(sorted).unwrap(),
            w_request_semantics(bounded).unwrap()
        );
    }

    /// 生成态对象行（`*_json` 文本视图解析回值，便于逐字段比较）。
    fn objects_of(state: &Value, kind: &str) -> Vec<Value> {
        state["objects"]
            .as_array()
            .expect("objects")
            .iter()
            .filter(|o| o["kind"] == kind)
            .map(|o| {
                let mut row = o.clone();
                for name in ["payload", "input_refs", "source_coords"] {
                    if let Some(text) = o[format!("{name}_json")].as_str() {
                        row[name] = serde_json::from_str(text).expect("文本视图是 JSON");
                    }
                }
                row
            })
            .collect()
    }

    fn requests_of(state: &Value) -> Vec<Value> {
        let knowledge = objects_of(state, CC054_KNOWLEDGE_KIND);
        assert_eq!(knowledge.len(), 1);
        knowledge[0]["payload"]["requests"]
            .as_array()
            .unwrap()
            .clone()
    }

    fn cross_layout_subject_key(object: &Value) -> Value {
        let kind = object["kind"].as_str().expect("主体 kind 必须是文本");
        if kind == CC006_SHAPE_KIND {
            let mut key = serde_json::Map::new();
            for field in [
                "kind",
                "object_revision",
                "window_start",
                "window_mid",
                "window_end",
                "branch",
                "dir_ab",
                "dir_bc",
                "comparisons_json",
                "input_refs_json",
                "source_coords_json",
            ] {
                let value = object.get(field).expect("CC006 主体语义字段缺失");
                assert!(!value.is_null(), "CC006 主体语义字段 {field} 不可为空");
                key.insert(field.to_string(), value.clone());
            }
            Value::Object(key)
        } else {
            let fact_key = object["fact_key"]
                .as_str()
                .expect("typed 主体须有 fact_key");
            assert!(!fact_key.is_empty(), "typed 主体 fact_key 不可为空");
            json!([kind, fact_key])
        }
    }

    /// crosswalk（ADR 0037 §八，§5.8 形式）在同一代上逐项核：分型对象去 `subsequent_development`
    /// 后逐字段相等；每个分型每个单元在对应 bar 的批记录里恰有一项，单元逐值相等、development
    /// 即 layout/1 载荷 `bars` 的同长前缀；单元总数相等；CC-054 请求 = layout/1 请求去掉
    /// 已有后续 bar 的分型 `subsequent_development` 请求，且按 (window_start, label) 排列。
    /// 返回（本代单元总数，fact_key → bounded 分型 object_id）。
    fn assert_bounded_crosswalk(
        gen: usize,
        sorted: &Value,
        bounded: &Value,
    ) -> (usize, std::collections::BTreeMap<String, (String, Value)>) {
        let mut ids = std::collections::BTreeMap::new();
        let bounded_fractals = objects_of(bounded, CC007_FRACTAL_KIND);
        let sorted_fractals = objects_of(sorted, CC007_FRACTAL_KIND);
        assert_eq!(
            bounded_fractals.len(),
            sorted_fractals.len(),
            "g{gen} 分型数"
        );
        let mut expected_units: std::collections::BTreeMap<String, Vec<(String, Value, Value)>> =
            Default::default();
        let mut total = 0;
        let mut realised: std::collections::BTreeSet<String> = Default::default();
        // entries 按分型窗口序（ADR 0054 / #1783；fact_key 文本序不是窗口序）。
        let mut pairs: Vec<(&Value, &Value)> =
            sorted_fractals.iter().zip(&bounded_fractals).collect();
        pairs.sort_by_key(|(_, b)| {
            b["payload"]["window"][0]
                .as_str()
                .unwrap()
                .parse::<i64>()
                .unwrap()
        });
        for (s, b) in pairs {
            assert_eq!(s["fact_key"], b["fact_key"], "g{gen} 分型槽位次序");
            let mut stripped = s["payload"].clone();
            // CC-006 形态 object_id 含 rule_revision（§5.3），两库必不同；改核指向本库同窗形态。
            stripped["shape_object_id"] = Value::Null;
            let mut mine = b["payload"].clone();
            let shape_id = mine["shape_object_id"].take();
            assert!(
                bounded["objects"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|o| o["kind"] == CC006_SHAPE_KIND && o["object_id"] == shape_id),
                "g{gen} 分型指向本库形态"
            );
            let bars = stripped
                .as_object_mut()
                .unwrap()
                .remove("subsequent_development")
                .expect("layout/1 分型载荷含后续发展")["bars"]
                .as_array()
                .unwrap()
                .clone();
            assert_eq!(stripped, mine, "g{gen} 分型载荷去后续发展后相等");
            assert!(b["payload"].get("subsequent_development").is_none());
            let find_shape = |state: &Value, object_id: &Value, window: &Value| {
                let matches: Vec<_> = objects_of(state, CC006_SHAPE_KIND)
                    .into_iter()
                    .filter(|o| {
                        &o["object_id"] == object_id
                            && json!([
                                o["window_start"].as_i64().unwrap().to_string(),
                                o["window_mid"].as_i64().unwrap().to_string(),
                                o["window_end"].as_i64().unwrap().to_string()
                            ]) == *window
                    })
                    .collect();
                assert_eq!(matches.len(), 1, "g{gen} 分型须指向唯一同窗形态");
                matches[0].clone()
            };
            let source_shape = find_shape(
                sorted,
                &s["payload"]["shape_object_id"],
                &s["payload"]["window"],
            );
            find_shape(bounded, &shape_id, &b["payload"]["window"]);
            let base_coords = source_shape["source_coords"].as_array().unwrap();
            let base_refs: Vec<_> = s["input_refs"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| base_coords.contains(&r["source_coord"]))
                .cloned()
                .collect();
            assert_eq!(base_refs.len(), base_coords.len());
            stripped["shape_object_id"] = shape_id;
            let mut normalized = s.clone();
            normalized["payload"] = stripped;
            normalized["input_refs"] = json!(base_refs);
            normalized["source_coords"] = json!(base_coords);
            for field in ["payload", "input_refs", "source_coords"] {
                normalized[format!("{field}_json")] =
                    json!(crate::canonical_json(&normalized[field]));
            }
            let identity = json!({"kind":normalized["kind"],"fact_key":normalized["fact_key"],
                "payload":normalized["payload"],"input_refs":normalized["input_refs"],
                "profile_id":bounded["profile_id"],"rule_revision":bounded["rule_revision"]});
            let mut identity_nodes = MemoryNodes::new();
            let digest = sc::write_value(&mut identity_nodes, &json_to_cvalue(&identity).unwrap())
                .unwrap()
                .digest;
            normalized["object_id"] = json!(format!(
                "obj3-{}",
                &sc::FixedRoot::new(&sc::frame(0x20, &digest)).as_str()[4..]
            ));
            assert_eq!(&normalized, b, "g{gen} 分型完整记录仅按布局配方规范化");
            let id = b["object_id"].as_str().unwrap().to_string();
            ids.insert(
                b["fact_key"].as_str().unwrap().to_string(),
                (id.clone(), json!([b["payload"], b["source_coords"]])),
            );
            if !bars.is_empty() {
                realised.insert(s["fact_key"].as_str().unwrap().to_string());
            }
            for (i, unit) in bars.iter().enumerate() {
                let coord = unit["bar"]["source_coord"].as_str().unwrap().to_string();
                let prefix = Value::Array(bars[..=i].to_vec());
                expected_units
                    .entry(coord)
                    .or_default()
                    .push((id.clone(), unit.clone(), prefix));
                total += 1;
            }
        }
        let batches = objects_of(bounded, CC007_DEVELOPMENT_BATCH_KIND);
        assert_eq!(
            batches.len(),
            expected_units.len(),
            "g{gen} 每根有单元的 bar 恰一条批记录"
        );
        let mut seen = 0;
        for batch in batches {
            let payload = &batch["payload"];
            let coord = payload["bar"]["source_coord"].as_str().unwrap().to_string();
            assert_eq!(
                batch["fact_key"],
                crate::canonical_json(&json!([CC007_DEVELOPMENT_BATCH_KIND, coord])),
                "g{gen} 批记录槽位"
            );
            assert_eq!(
                batch["source_coords"],
                json!([coord]),
                "g{gen} 批记录来源＝本 bar"
            );
            let want = &expected_units[&coord];
            let entries = payload["entries"].as_array().unwrap();
            assert_eq!(entries.len(), want.len(), "g{gen} bar {coord} 的 entry 数");
            for (entry, (id, unit, prefix)) in entries.iter().zip(want) {
                assert_eq!(
                    entry["fractal_object_id"],
                    json!(id),
                    "g{gen} entry 按分型窗口序"
                );
                assert_eq!(&entry["unit"], unit, "g{gen} 单元逐值相等");
                assert_eq!(&entry["unit"]["bar"], &payload["bar"]);
                assert_eq!(
                    &entry["development"]["root"], prefix,
                    "g{gen} development＝bars 前缀"
                );
                assert_eq!(
                    entry["development"]["count"],
                    json!(prefix.as_array().unwrap().len().to_string())
                );
                seen += 1;
            }
        }
        assert_eq!(seen, total, "g{gen} 单元总数相等");
        // CC-054：请求集合与次序。
        let key_of = |r: &Value| -> (String, Option<String>) {
            let id: Value = serde_json::from_str(r["request_id"].as_str().unwrap()).unwrap();
            match &id[0] {
                Value::Array(slot) => (
                    crate::canonical_json(&Value::Array(slot.clone())),
                    id[1].as_str().map(str::to_string),
                ),
                _ => (crate::canonical_json(&id), None),
            }
        };
        let bounded_requests = requests_of(bounded);
        let expected: Vec<String> = requests_of(sorted)
            .iter()
            .filter(|r| {
                let (slot, label) = key_of(r);
                !(label.as_deref() == Some("subsequent_development") && realised.contains(&slot))
            })
            .map(|r| r["request_id"].as_str().unwrap().to_string())
            .collect();
        let mut actual: Vec<String> = bounded_requests
            .iter()
            .map(|r| r["request_id"].as_str().unwrap().to_string())
            .collect();
        // (阶段, window_start, label)：W 初始化 0 最前、带窗口 1、待决 2 最后（#1419 × #1491）。
        let order: Vec<(u8, i64, usize)> = bounded_requests
            .iter()
            .map(|r| {
                let id: Value = serde_json::from_str(r["request_id"].as_str().unwrap()).unwrap();
                if id[0].is_array() && id[1].is_array() {
                    assert_eq!(
                        id[1][2], "structure-classification/1",
                        "g{gen} structure request contract"
                    );
                    return (3, 0, 0);
                }
                let labels = [
                    "shape_description",
                    "descriptive_labels",
                    "subsequent_development",
                ];
                let (kind, tail, label) = match &id[0] {
                    Value::Array(slot) => (
                        slot[0].as_str().unwrap().to_string(),
                        slot[1].clone(),
                        4 + labels
                            .iter()
                            .position(|l| id[1] == *l)
                            .unwrap_or_else(|| panic!("g{gen} unexpected request key {id}")),
                    ),
                    Value::String(kind) => (kind.clone(), id[1].clone(), 0),
                    _ => unreachable!(),
                };
                if kind == crate::content_producer::CC004_INITIALIZATION_KIND {
                    assert_eq!(tail, "W", "g{gen} W 初始化槽");
                    return (0, 0, 0);
                }
                if tail == "pending" {
                    return (2, 0, 0);
                }
                let start = match &tail {
                    Value::Array(window) => window[0].as_str().unwrap().parse().unwrap(),
                    Value::String(coord) => coord.parse().unwrap(),
                    _ => unreachable!(),
                };
                let rank = match kind.as_str() {
                    crate::content_producer::CC004_UNDETERMINED_KIND => 0,
                    CC004_STEP_KIND => 1,
                    CC005_GROUP_KIND => 2,
                    CC006_SHAPE_KIND => 3,
                    _ => label,
                };
                (1, start, rank)
            })
            .collect();
        let mut sorted_order = order.clone();
        sorted_order.sort();
        assert_eq!(
            order, sorted_order,
            "g{gen} bounded 请求按 (window_start, label) 排列"
        );
        let mut expected_sorted = expected.clone();
        expected_sorted.sort();
        actual.sort();
        assert_eq!(actual, expected_sorted, "g{gen} bounded 请求集合");
        let normalize = |state: &Value, request: &Value| {
            assert_eq!(request.as_object().unwrap().len(), 5);
            let mut result = request.clone();
            if let Some(id) = request["subject_id"].as_str() {
                let matches: Vec<_> = state["objects"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|o| o["object_id"] == id)
                    .collect();
                assert_eq!(matches.len(), 1);
                result["subject_id"] = cross_layout_subject_key(matches[0]);
            }
            result
        };
        let expected_semantics: std::collections::BTreeMap<_, _> = requests_of(sorted)
            .iter()
            .filter(|r| {
                let (slot, label) = key_of(r);
                !(label.as_deref() == Some("subsequent_development") && realised.contains(&slot))
            })
            .map(|r| {
                (
                    r["request_id"].as_str().unwrap().to_string(),
                    normalize(sorted, r),
                )
            })
            .collect();
        let actual_semantics: std::collections::BTreeMap<_, _> = bounded_requests
            .iter()
            .map(|r| {
                (
                    r["request_id"].as_str().unwrap().to_string(),
                    normalize(bounded, r),
                )
            })
            .collect();
        assert_eq!(
            actual_semantics.len(),
            bounded_requests.len(),
            "请求不可重复"
        );
        assert_eq!(
            actual_semantics, expected_semantics,
            "g{gen} 全部请求五字段与主体绑定"
        );
        let structure_keys = |state: &Value| {
            requests_of(state)
                .into_iter()
                .filter_map(|r| {
                    let key: Value =
                        serde_json::from_str(r["request_id"].as_str().unwrap()).unwrap();
                    (key[0].is_array() && key[1].is_array()).then(|| r["request_id"].clone())
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            structure_keys(sorted),
            structure_keys(bounded),
            "g{gen} 结构请求后缀次序"
        );
        let mut expected_knowledge = objects_of(sorted, CC054_KNOWLEDGE_KIND).remove(0);
        let actual_knowledge = objects_of(bounded, CC054_KNOWLEDGE_KIND).remove(0);
        let source_requests = requests_of(sorted);
        let normalized_requests: Vec<_> = bounded_requests
            .iter()
            .map(|target| {
                let source = source_requests
                    .iter()
                    .find(|r| r["request_id"] == target["request_id"])
                    .unwrap();
                let mut request = source.clone();
                if let Some(id) = source["subject_id"].as_str() {
                    let old = sorted["objects"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|o| o["object_id"] == id)
                        .unwrap();
                    let found: Vec<_> = bounded["objects"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|o| cross_layout_subject_key(o) == cross_layout_subject_key(old))
                        .collect();
                    assert_eq!(found.len(), 1);
                    request["subject_id"] = found[0]["object_id"].clone();
                }
                request
            })
            .collect();
        expected_knowledge["payload"]["requests"] = json!(normalized_requests);
        expected_knowledge["payload_json"] =
            json!(crate::canonical_json(&expected_knowledge["payload"]));
        let identity = json!({"kind":expected_knowledge["kind"],"fact_key":expected_knowledge["fact_key"],
            "payload":expected_knowledge["payload"],"input_refs":expected_knowledge["input_refs"],
            "profile_id":bounded["profile_id"],"rule_revision":bounded["rule_revision"]});
        let mut identity_nodes = MemoryNodes::new();
        let digest = sc::write_value(&mut identity_nodes, &json_to_cvalue(&identity).unwrap())
            .unwrap()
            .digest;
        expected_knowledge["object_id"] = json!(format!(
            "obj3-{}",
            &sc::FixedRoot::new(&sc::frame(0x20, &digest)).as_str()[4..]
        ));
        assert_eq!(
            expected_knowledge, actual_knowledge,
            "g{gen} CC054完整记录显式规范化"
        );
        for r in &bounded_requests {
            let (slot, label) = key_of(r);
            if label.as_deref() == Some("subsequent_development") {
                assert_eq!(
                    r["reasons"],
                    json!(["no_subsequent_bar"]),
                    "g{gen} 待决后续请求"
                );
                assert!(!realised.contains(&slot));
            }
        }
        (total, ids)
    }

    /// #1419 × #1491：W 初始化/未定请求在 layout/1 与 bounded-layout/1 两条车道都进 CC-054，且
    /// 生成不因次序键缺 W 而报错。layout/1 保持 W 原线序（初始化 → 待决组 → 逐原件未定，均在
    /// 其余请求之前）；bounded 下 W 初始化恒为首条，逐原件未定按坐标归入 (window_start, label)。
    #[test]
    fn w_requests_keep_line_order_in_layout1_and_lead_bounded_layout() {
        let events = delta_long_fixture();
        let cuts: Vec<usize> = (1..=events.len()).collect();
        let mut runs = bounded_layout_runs(&events, &cuts);
        runs.extend(bounded2_layout_runs(&events, &cuts));
        let id_of = |r: &Value| -> Value {
            serde_json::from_str(r["request_id"].as_str().unwrap()).unwrap()
        };
        let init_id = json!([crate::content_producer::CC004_INITIALIZATION_KIND, "W"]);
        let (mut saw_undetermined, mut saw_established) = (false, false);
        for (i, (sorted, bounded)) in runs.iter().enumerate() {
            let gen = i + 1;
            let line: Vec<Value> = requests_of(sorted).iter().map(id_of).collect();
            assert_eq!(line[0], init_id, "g{gen} layout/1 首条是 W 初始化");
            let mut w_block = vec![init_id.clone()];
            let mut rest = &line[1..];
            if rest
                .first()
                .map(|id| id[0] == CC005_GROUP_KIND && id[1] == "pending")
                == Some(true)
            {
                w_block.push(rest[0].clone());
                rest = &rest[1..];
            }
            let undetermined: Vec<i64> = rest
                .iter()
                .take_while(|id| id[0] == crate::content_producer::CC004_UNDETERMINED_KIND)
                .map(|id| id[1].as_str().unwrap().parse().unwrap())
                .collect();
            assert!(
                undetermined.windows(2).all(|w| w[0] < w[1]),
                "g{gen} layout/1 逐原件未定按坐标升序"
            );
            assert!(
                line[w_block.len() + undetermined.len()..].iter().all(|id| {
                    id[0] != crate::content_producer::CC004_INITIALIZATION_KIND
                        && id[0] != crate::content_producer::CC004_UNDETERMINED_KIND
                }),
                "g{gen} layout/1 W 请求全部在其余请求之前"
            );
            if undetermined.is_empty() {
                saw_established = true;
            } else {
                saw_undetermined = true;
            }
            let lead: Vec<Value> = requests_of(bounded).iter().map(id_of).collect();
            assert_eq!(lead[0], init_id, "g{gen} bounded 首条是 W 初始化");
            for coord in &undetermined {
                assert!(
                    lead.contains(&json!([
                        crate::content_producer::CC004_UNDETERMINED_KIND,
                        coord.to_string()
                    ])),
                    "g{gen} bounded 含未定原件 {coord} 的请求"
                );
            }
        }
        assert!(saw_undetermined, "夹具须覆盖方向未定期的 W 未定请求");
        assert!(saw_established, "夹具须覆盖方向建立后的 W 初始化");
    }

    #[test]
    fn bounded_layout_cc007_batches_crosswalk_layout1_and_fractals_born_once() {
        let events = delta_long_fixture();
        let cuts: Vec<usize> = (1..=events.len()).collect();
        let runs = bounded_layout_runs(&events, &cuts);
        let mut previous_ids: std::collections::BTreeMap<String, (String, Value)> =
            Default::default();
        let mut stable_fractals = 0;
        let mut max_units = 0;
        let mut batch_ids_prev: std::collections::BTreeMap<String, String> = Default::default();
        let mut stable_batches = 0;
        for (i, (sorted, bounded)) in runs.iter().enumerate() {
            let (units, ids) = assert_bounded_crosswalk(i + 1, sorted, bounded);
            max_units = max_units.max(units);
            // 分型只在描述/基础来源变化时出新版：载荷与来源不变 ⟺ object_id 不变（后续 bar 不触发）。
            for (key, (id, basis)) in &ids {
                if let Some((old, old_basis)) = previous_ids.get(key) {
                    assert_eq!(
                        old == id,
                        old_basis == basis,
                        "g{} 分型 {key} 版本与基础来源",
                        i + 1
                    );
                    stable_fractals += usize::from(old == id);
                }
            }
            previous_ids = ids;
            // 已过去 bar 的批记录（无新分型覆盖时）跨代保持同一 object_id。
            let batch_ids: std::collections::BTreeMap<String, String> =
                objects_of(bounded, CC007_DEVELOPMENT_BATCH_KIND)
                    .iter()
                    .map(|o| {
                        (
                            o["fact_key"].as_str().unwrap().to_string(),
                            o["object_id"].as_str().unwrap().to_string(),
                        )
                    })
                    .collect();
            for (key, id) in &batch_ids {
                if batch_ids_prev.get(key) == Some(id) {
                    stable_batches += 1;
                }
            }
            batch_ids_prev = batch_ids;
        }
        assert!(max_units >= 20, "夹具须产生足量比较单元：{max_units}");
        assert!(stable_batches > 0, "须有跨代不变的已过去 bar 批记录");
        assert!(stable_fractals > 0, "须有跨代后续 bar 增加而分型不换版本");
        assert!(!previous_ids.is_empty());
    }

    #[test]
    fn bounded_layout_cc007_batches_survive_revision_swap_insertion_incremental_equals_cold() {
        // 同 `delta_full_bi_prior_tail_revision_and_reactivated_windows_match_full` 的输入：早期 bar
        // 改价修订、再恢复、尾 bar 换坐标（旧坐标移走、新坐标出现）。
        let events = revision_events();
        let runs = bounded_layout_runs(&events, &[1, 2, 8, 14, 15, 16, 17, 17]);
        let mut any_units = false;
        for (i, (sorted, bounded)) in runs.iter().enumerate() {
            any_units |= assert_bounded_crosswalk(i + 1, sorted, bounded).0 > 0;
        }
        assert!(any_units, "修订夹具须产生比较单元");
    }

    /// 以同 identity 更高修订把 `delta_long_fixture` 的若干根改坐标（一次一根，同代施加）。
    fn moved_events(moves: &[(usize, &str)]) -> Vec<Value> {
        let mut events = delta_long_fixture();
        for (n, (idx, coord)) in moves.iter().enumerate() {
            let mut moved = events[*idx].clone();
            let seq = 15 + n;
            for (k, v) in [
                ("revision", json!(2)),
                ("input_revision", json!("2")),
                ("seq", json!(seq)),
                ("source_coord", json!(coord)),
                ("ts", json!(coord)),
                ("payload_hash", json!(format!("moved-{n}"))),
                ("receipt_id", json!(format!("receipt-{seq}"))),
            ] {
                moved[k] = v;
            }
            events.push(moved);
        }
        events
    }

    /// #1780 独审 C-1 真负控：中段 bar（不是任何分型的首/末单元）换坐标。其旧坐标的批记录必须
    /// 判脏移除，与冷算逐字节相同；判脏起点取 producer 失效水位（min(旧坐标, 新坐标)）。
    /// 覆盖后移到尾（b5/b6/b10/b11/b12 → 1000）、前移（b10 100→105）与同代两根中段互换。
    #[test]
    fn bounded_layout_cc007_batches_mid_coordinate_move_incremental_equals_cold() {
        let cases: Vec<(&str, Vec<(usize, &str)>)> = vec![
            ("b5→1000", vec![(5, "1000")]),
            ("b6→1000", vec![(6, "1000")]),
            ("b10→1000", vec![(10, "1000")]),
            ("b11→1000", vec![(11, "1000")]),
            ("b12→1000", vec![(12, "1000")]),
            ("b10→105", vec![(10, "105")]),
            ("b6↔b11", vec![(6, "110"), (11, "60")]),
        ];
        for (label, moves) in cases {
            let events = moved_events(&moves);
            let base = delta_long_fixture().len();
            let runs = bounded_layout_runs(&events, &[base, events.len()]);
            for (i, (sorted, bounded)) in runs.iter().enumerate() {
                let (units, _) = assert_bounded_crosswalk(i + 1, sorted, bounded);
                assert!(units > 0, "{label}: 夹具须产生比较单元");
            }
        }
    }

    /// ADR 0054 #1790 修订·验收 2 中段出生锁。三例：坐标 85 插入新 bar、b9 改价修订、
    /// b8 改价修订，均在第 2 代中段出生新分型（新分型窗口早于第 1 代最后一个分型）。断言：
    /// ① 出生在窗口序上构成后缀——第 2 代不存在「object_id 不在第 1 代」的 X 早于「第 1 代已存在」
    ///    的 Y。分型身份含右组来源前缀链（`group_sources_coords`：方向证据＝处理到组锚为止的完整
    ///    前缀），坐标 c 处的出生/修订使窗口在其后的分型全部同代重生，故在可达输入上出生序（首获知代
    ///    ↑、同代窗口序）与窗口序逐位相同；本条若失败，说明身份配方变了，
    ///    即触发 ADR 0054 #1790 修订段退场条件，须重开 #1790；
    /// ② 每根 bar 的 entries 按 `window[0]` 严格递增，且至少一根同含新旧分型；
    /// ③ 每代增量 ≡ 冷算逐字节（`bounded_layout_runs` 内核）。
    /// 不读 `first_known_generation`：生成层恒写本代（`content_producer.rs` typed 记录）。
    #[test]
    fn bounded_layout_cc007_entries_window_order_on_mid_birth() {
        let base_events = delta_long_fixture();
        let base = base_events.len();
        let revise = |idx: usize, high: i64, low: i64, label: &str| {
            let mut revised = base_events[idx].clone();
            for (k, v) in [
                ("revision", json!(2)),
                ("input_revision", json!("2")),
                ("seq", json!(base + 1)),
                ("high", json!(high.to_string())),
                ("low", json!(low.to_string())),
                ("open", json!(low.to_string())),
                ("close", json!(high.to_string())),
                ("payload_hash", json!(format!("rev-{label}"))),
                ("receipt_id", json!(format!("receipt-{label}"))),
            ] {
                revised[k] = v;
            }
            revised
        };
        let inserted = json!({
            "identity_key": "x85", "event_id": "x85", "source_coord": "85", "ts": "85",
            "open": "15", "high": "25", "low": "15", "close": "25",
            "revision": 1, "seq": base + 1, "input_revision": "1",
            "payload_hash": "hash-x85", "receipt_id": "receipt-x85",
        });
        let cases = [
            ("insert@85", inserted),
            ("revise b9", revise(9, 25, 15, "b9")),
            ("revise b8", revise(8, 30, 20, "b8")),
        ];
        let windows = |state: &Value| -> std::collections::BTreeMap<String, i64> {
            objects_of(state, CC007_FRACTAL_KIND)
                .iter()
                .map(|o| {
                    let start = o["payload"]["window"][0].as_str().unwrap();
                    (
                        o["object_id"].as_str().unwrap().to_string(),
                        start.parse().unwrap(),
                    )
                })
                .collect()
        };
        for (label, event) in cases {
            let mut events = base_events.clone();
            events.push(event);
            let runs = bounded_layout_runs(&events, &[base, events.len()]);
            for (i, (sorted, bounded)) in runs.iter().enumerate() {
                assert_bounded_crosswalk(i + 1, sorted, bounded);
            }
            let (before, after) = (windows(&runs[0].1), windows(&runs[1].1));
            let last_before = *before.values().max().unwrap();
            let born: Vec<i64> = after
                .iter()
                .filter(|(id, _)| !before.contains_key(*id))
                .map(|(_, w)| *w)
                .collect();
            assert!(
                born.iter().any(|w| *w < last_before),
                "{label}: 须在中段出生新分型：{born:?} vs 第 1 代末窗口 {last_before}"
            );
            let first_born = *born.iter().min().unwrap();
            for (id, w) in &after {
                if before.contains_key(id) {
                    assert!(
                        *w < first_born,
                        "{label}: 旧分型 {id}（窗口 {w}）不得晚于新出生分型（{first_born}）"
                    );
                }
            }
            let mut mixed = false;
            for batch in objects_of(&runs[1].1, CC007_DEVELOPMENT_BATCH_KIND) {
                let coord = batch["payload"]["bar"]["source_coord"].clone();
                let ids: Vec<&str> = batch["payload"]["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|e| e["fractal_object_id"].as_str().unwrap())
                    .collect();
                let starts: Vec<i64> = ids.iter().map(|id| after[*id]).collect();
                assert!(
                    starts.windows(2).all(|w| w[0] < w[1]),
                    "{label}: bar {coord} entries 须按 window[0] 严格递增：{starts:?}"
                );
                mixed |= ids.iter().any(|id| before.contains_key(*id))
                    && ids.iter().any(|id| !before.contains_key(*id));
            }
            assert!(mixed, "{label}: 须有一根 bar 的 entries 同含新旧分型");
        }
    }

    /// #1419 A7 F3：冻结历史导出（base `5123c9e273` 生成，run 40831506671 导出）解包到私有临时目录，
    /// 逐文件核 `CONTENTS.sha256` 后才交测试读；仓内原件不被打开写入。
    /// #1419 R1（29BI 返修）：临时目录改用标准库 TestFiles 生命周期，不依赖 dev-dependency
    /// `tempfile`——编译负例脚本只链接普通 bin 依赖，`tempfile` 在其中不可解析。
    pub(crate) fn issue1419_history() -> HistoryTestDir {
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/issue1419_history_5123");
        let dir = HistoryTestDir::new();
        let status = std::process::Command::new("tar")
            .arg("-xzf")
            .arg(fixture.join("export.tar.gz"))
            .arg("-C")
            .arg(dir.path())
            .status()
            .unwrap();
        assert!(status.success(), "冻结历史导出解包失败");
        let manifest = std::fs::read_to_string(fixture.join("CONTENTS.sha256")).unwrap();
        let mut listed = 0;
        for line in manifest.lines() {
            let (digest, name) = line.split_once("  ").unwrap();
            let bytes = std::fs::read(dir.path().join(name)).unwrap();
            assert_eq!(
                newchan_rust::session_protocol::sha256_hex(&bytes),
                digest,
                "冻结原件 {name} 摘要"
            );
            listed += 1;
        }
        assert_eq!(listed, 62, "冻结原件清单条目数");
        dir
    }

    /// #1419 R1：历史夹具临时目录。私有解包、退出清理，与 `s_structure_session` 的 `TestFiles`
    /// 同一生命周期模式；不引入生产依赖，仓内原件仍只读。
    pub(crate) struct HistoryTestDir(std::path::PathBuf);

    impl HistoryTestDir {
        pub(super) fn new() -> Self {
            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::time::{SystemTime, UNIX_EPOCH};
            static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let number = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "1419-history-{}-{stamp}-{number}",
                std::process::id()
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }

        pub(crate) fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for HistoryTestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// #1419 A7：原 25 根仍是 base `5123c9e273` 原件，由本测试经冻结历史导出节点逐字读回；
    /// 候选不再等于这些根。
    #[test]
    fn layout1_historical_draft_nodes_remain_readable() {
        use sc::NodeSink as _;
        let history = issue1419_history();
        let dir = history.path().to_path_buf();
        let table = std::fs::read_to_string(dir.join("compare.tsv")).unwrap();
        let rows: Vec<Vec<&str>> = table
            .lines()
            .skip(1)
            .map(|line| line.split('\t').collect())
            .collect();
        assert_eq!(rows.len(), 25);
        for group in ["long", "revision", "realrule"] {
            let path = dir.join(group);
            let mut store = MemoryNodes::new();
            for line in std::fs::read_to_string(path.join("nodes.jsonl"))
                .unwrap()
                .lines()
            {
                let node: Value = serde_json::from_str(line).unwrap();
                let digest = sc::parse_root(node["digest"].as_str().unwrap()).unwrap();
                let hex = node["body_hex"].as_str().unwrap();
                let bytes: Vec<u8> = hex
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect();
                assert_eq!(hex.len() % 2, 0);
                store
                    .put(digest, node["tag"].as_u64().unwrap() as u8, &bytes)
                    .unwrap();
            }
            let cuts: Value =
                serde_json::from_slice(&std::fs::read(path.join("cuts.json")).unwrap()).unwrap();
            assert_eq!(
                cuts.as_array().unwrap().len(),
                match group {
                    "long" => 14,
                    "revision" => 8,
                    _ => 3,
                }
            );
            for cut in cuts.as_array().unwrap() {
                let generation = cut["gen"].as_i64().unwrap();
                let row = rows
                    .iter()
                    .find(|r| r[0] == group && r[1].parse::<i64>().unwrap() == generation)
                    .unwrap();
                assert_eq!(row[2], cut["cut"].as_str().unwrap());
                assert_eq!(
                    row[3].parse::<i64>().unwrap(),
                    cut["prefix"].as_i64().unwrap()
                );
                assert_eq!(row[4], cut["rule"].as_str().unwrap());
                assert_eq!(row[5], row[6]);
                assert_eq!(row[7], "true");
                for name in ["state_without_axes", "generated_state"] {
                    let meta = &cut[name];
                    assert_eq!(meta["format"], "s-content/1");
                    assert_eq!(meta["kind"], "object");
                    assert_eq!(meta["root"], row[5]);
                    let digest = sc::parse_root(meta["root"].as_str().unwrap()).unwrap();
                    let reference =
                        ContentRef::new(sc::Kind::Object, &digest, meta["extent"].as_u64());
                    sc::verify(&mut store, &reference).unwrap();
                    let mut actual = Vec::new();
                    sc::render_canonical(&mut store, &reference, &mut actual).unwrap();
                    let expected =
                        std::fs::read(path.join(format!("g{generation}.{name}.json"))).unwrap();
                    assert_eq!(actual, expected, "{group} g{generation} {name}");
                }
            }
        }
    }

    /// #1780 独审 M-1 原锁，#1419 A7 迁移：每代仍锚 base main `5123c9e273` 原 25 根，
    /// 生成态两根改与独立 W oracle 资产（`tests/fixtures/issue1419_w_oracle`，只读 base 推导）
    /// 逐字节、逐 ContentRef 比较。base 根真实性见 `layout1_historical_draft_nodes_remain_readable`。
    #[test]
    fn layout1_generation_roots_frozen_from_base_main() {
        fn run(tag: &str, events: &[Value], cuts: &[usize], rule: &str, expected: &[&str]) {
            assert_eq!(cuts.len(), expected.len());
            let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/issue1419_w_oracle");
            let roots: Value =
                serde_json::from_slice(&std::fs::read(dir.join("roots.json")).unwrap()).unwrap();
            let roots = roots[tag].as_array().unwrap();
            assert_eq!(roots.len(), expected.len());
            let scope = json!({"instrument":"B1", "exchange":"fixture"});
            let mut runtime = GenerationRuntime::new("neutral".into(), crate::tb02a::PROFILE, rule);
            let mut store = MemoryNodes::new();
            for (turn, (&prefix, want)) in cuts.iter().zip(expected).enumerate() {
                let gen = turn as i64 + 1;
                let cut = format!("cut-{gen}");
                let raw = RawEventsView::plain(events).prefix(prefix).unwrap();
                let effective = bj::effective(raw, None).unwrap();
                let input = full_input(
                    raw,
                    effective.raw_view(),
                    &scope,
                    &cut,
                    gen,
                    prefix as i64,
                    rule,
                );
                let generated = runtime.generate(&mut store, &input).unwrap();
                let oracle = &roots[turn];
                assert_eq!(oracle["gen"], gen);
                assert_eq!(
                    oracle["base_root"], *want,
                    "{tag} g{gen}: oracle 须锚原 base 根"
                );
                let file = dir.join(tag).join(format!("g{gen}.expected_state.json"));
                let bytes = std::fs::read(file).unwrap();
                let gs: &ContentRef = &generated.generated_state;
                let swa: &ContentRef = &generated.state_without_axes;
                for (name, r) in [("generated_state", gs), ("state_without_axes", swa)] {
                    assert_eq!(r.format, "s-content/1", "{tag} g{gen} {name}");
                    assert_eq!(r.kind.name(), "object", "{tag} g{gen} {name}");
                    assert_eq!(
                        r.root,
                        oracle["expected_root"].as_str().unwrap(),
                        "{tag} g{gen} {name}"
                    );
                    assert_eq!(r.extent, oracle["extent"].as_u64(), "{tag} g{gen} {name}");
                    sc::verify(&mut store, r).unwrap();
                    let text = canonical_text(&mut store, r);
                    assert!(
                        text.as_bytes() == bytes.as_slice(),
                        "{tag} g{gen} {name} 完整字段与次序"
                    );
                }
            }
        }
        let events = delta_long_fixture();
        let cuts: Vec<usize> = (1..=events.len()).collect();
        run(
            "long",
            &events,
            &cuts,
            "r",
            &[
                "sc1-948366d1a1a96702e595c31a981d44a8cd860d8079cbff9f96ac6cb6c174c4a7",
                "sc1-f66ed7a7823e94b2600747308722749f3d6de9a17b4ef0cc79fa4fe2c968b4a4",
                "sc1-20c92b84d00d51bef243e8151905fc94f05fa0d75777f92e9cf16deb03ffe855",
                "sc1-9373c03393f0b22bc9b7419ca0fe3f0a089083a3ec5c322a0d94e7cef6d655db",
                "sc1-94022dcec4c0d9e81058e584398b2a132bac9cb328481261a09b34a3c8f7da37",
                "sc1-9ed3773460b18c74c8878c6b7e759d64f6443ebe0bcbce27b723dc2f3a9c8c2f",
                "sc1-31cd8bc881d70f7bf698ecb9fd6ca9740310f3e111df6a0d1d742434cb10b883",
                "sc1-77f7e973924aa969081800c6a99e2a0d6d800ae2551d2fcec29cb3e9568bd903",
                "sc1-f944daa8922cbae6e56b8b6f6e43746c59ee0bd2002be3f0b2c6a791f88553dc",
                "sc1-55c40900ebcd79380bf25f8eb7d262f7ad79aba660fafb82d3070c519111966b",
                "sc1-4d3ff43c59eb4d8434adb7dee4842a5271d07cfcfa0ed4e8f8eec62140231b5b",
                "sc1-a4c940d2cc628929764d8a94c4f6774dec6cb8f9b98dfb1253cf117cddd8ac49",
                "sc1-cc91d04f817bd368f7ab43b15320656cd20765c87ce2b9d4269995f863f10290",
                "sc1-1f1883260c44c0de1f736eecfa10b260299e8b37638fb83ea79672e050e64b66",
            ],
        );
        let events = revision_events();
        run(
            "revision",
            &events,
            &[1, 2, 8, 14, 15, 16, 17, 17],
            "r",
            &[
                "sc1-948366d1a1a96702e595c31a981d44a8cd860d8079cbff9f96ac6cb6c174c4a7",
                "sc1-f66ed7a7823e94b2600747308722749f3d6de9a17b4ef0cc79fa4fe2c968b4a4",
                "sc1-5f62791ebc8f826d882fcee15ffa09edcdab873bc27b04261e99cc7ed29521f1",
                "sc1-f93b79013d5bea7c79b28f60e64e82115a3e09f838c26c83318e1e7c046cea06",
                "sc1-b8cf282c385cc9878ca8f967ddf4ba8183a8e54be8823a177d04405d589f6db4",
                "sc1-333091400aa357723f809c07d6a923e4755947413d04cbc1beade766072ec854",
                "sc1-236fb13005488b6f7c966188ae62daa7ffa1143eb05fd00cf44dd4433a14b316",
                "sc1-8a20f9cc37496026a94e59b4b45cc7b8f8a500dfd487b19e772a018694d9d063",
            ],
        );
        run(
            "realrule",
            &delta_long_fixture(),
            &[5, 9, 14],
            crate::V3_RULE_REVISION,
            &[
                "sc1-c8f28f2cec73feaae72bf279e0790497d8d8af64197116e45e096b89992b214f",
                "sc1-5f47222d095c66143e8cd5f52d70080f15b93218c7b8499edaa24948ad38a9ae",
                "sc1-cf5c2f1e68e4a50dddc548abb9a2fd00725f23714e4c69fbb9b033549a369f14",
            ],
        );
    }

    /// #1419 A7 F2：一代 W 请求的完整语义，以 request_id 为键。每个 id 恰出现一次；
    /// 字段集恰为五项；subject_id 在本车道 objects 中解析，须恰有一个实际对象，以其 (kind, fact_key) 比较。
    fn w_request_semantics(
        state: &Value,
    ) -> Result<std::collections::BTreeMap<String, Value>, String> {
        use crate::content_producer::{CC004_INITIALIZATION_KIND, CC004_UNDETERMINED_KIND};
        let objects = state["objects"].as_array().ok_or("objects 缺失")?;
        let mut out = std::collections::BTreeMap::new();
        for request in requests_of(state) {
            let id = request["request_id"].as_str().ok_or("request_id 非文本")?;
            if !(id.contains(CC004_INITIALIZATION_KIND) || id.contains(CC004_UNDETERMINED_KIND)) {
                continue;
            }
            let fields: Vec<&String> = request.as_object().ok_or("请求非对象")?.keys().collect();
            if fields
                != [
                    "axes",
                    "reasons",
                    "request_id",
                    "source_coords",
                    "subject_id",
                ]
            {
                return Err(format!("{id} 字段集 {fields:?}"));
            }
            let subject = request["subject_id"].as_str().ok_or("subject_id 非文本")?;
            let hits: Vec<&Value> = objects
                .iter()
                .filter(|o| o["object_id"] == subject)
                .collect();
            if hits.len() != 1 {
                return Err(format!(
                    "{id} subject {subject} 解析到 {} 个对象",
                    hits.len()
                ));
            }
            let semantic = json!({
                "axes": request["axes"],
                "reasons": request["reasons"],
                "source_coords": request["source_coords"],
                "subject": [hits[0]["kind"], hits[0]["fact_key"]],
            });
            if out.insert(id.to_string(), semantic).is_some() {
                return Err(format!("{id} 重复"));
            }
        }
        Ok(out)
    }

    /// 三组原输入（long 14、revision 8、realrule 3 代），与 `layout1_generation_roots_frozen_from_base_main` 同。
    fn a7_groups() -> Vec<(&'static str, Vec<Value>, Vec<usize>, &'static str)> {
        let long = delta_long_fixture();
        let long_cuts: Vec<usize> = (1..=long.len()).collect();
        vec![
            ("long", long, long_cuts, "r"),
            (
                "revision",
                revision_events(),
                vec![1, 2, 8, 14, 15, 16, 17, 17],
                "r",
            ),
            (
                "realrule",
                delta_long_fixture(),
                vec![5, 9, 14],
                crate::V3_RULE_REVISION,
            ),
        ]
    }

    /// #1419 A7：同一 W 语义在 layout/1 与 bounded-layout/1 两种布局下逐代相同，覆盖原 25 代。
    /// W 对象载荷、输入引用、来源坐标逐项相等；W 请求按 request_id 比完整语义（见 `w_request_semantics`）。
    #[test]
    fn w_semantics_layout_neutral_between_layout1_and_bounded() {
        use crate::content_producer::{CC004_INITIALIZATION_KIND, CC004_UNDETERMINED_KIND};
        let (mut undetermined, mut established, mut generations) = (false, false, 0);
        let mut request_kinds = std::collections::BTreeSet::new();
        for (tag, events, cuts, rule) in a7_groups() {
            let runs = bounded_layout_runs_with(&events, &cuts, rule);
            assert_eq!(runs.len(), cuts.len());
            for (i, (sorted, bounded)) in runs.iter().enumerate() {
                let gen = i + 1;
                generations += 1;
                for kind in [CC004_INITIALIZATION_KIND, CC004_UNDETERMINED_KIND] {
                    let w = |state: &Value| -> Vec<Value> {
                        objects_of(state, kind)
                            .iter()
                            .map(|o| {
                                json!([
                                    o["fact_key"],
                                    o["payload"],
                                    o["input_refs"],
                                    o["source_coords"]
                                ])
                            })
                            .collect()
                    };
                    assert_eq!(w(sorted), w(bounded), "{tag} g{gen} {kind} 跨布局中性");
                }
                let s = w_request_semantics(sorted).unwrap();
                let b = w_request_semantics(bounded).unwrap();
                assert!(!s.is_empty(), "{tag} g{gen} 须有 W 请求");
                assert_eq!(s, b, "{tag} g{gen} W 请求完整语义");
                for semantic in s.values() {
                    request_kinds.insert(semantic["subject"][0].as_str().unwrap().to_string());
                }
                let init = &objects_of(sorted, CC004_INITIALIZATION_KIND)[0]["payload"];
                match init["direction_state"].as_str().unwrap() {
                    "Undetermined" => undetermined = true,
                    _ => established = true,
                }
            }
        }
        assert_eq!(generations, 25, "覆盖原 long 14、revision 8、realrule 3 代");
        let mut bounded2_generations = 0;
        let mut crosswalk_subject_kinds = std::collections::BTreeSet::new();
        for (_tag, events, cuts, _historical_rule) in a7_groups() {
            for (i, (sorted, bounded)) in bounded2_layout_runs(&events, &cuts).iter().enumerate() {
                for request in requests_of(bounded) {
                    if let Some(id) = request["subject_id"].as_str() {
                        let subject = bounded["objects"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|o| o["object_id"] == id)
                            .unwrap();
                        crosswalk_subject_kinds
                            .insert(subject["kind"].as_str().unwrap().to_string());
                    }
                }
                assert_bounded2_crosswalk(i + 1, sorted, bounded, &events[..cuts[i]]);
                bounded2_generations += 1;
            }
        }
        assert_eq!(bounded2_generations, 25);
        eprintln!("V6 crosswalk subject kinds {crosswalk_subject_kinds:?}");

        assert!(
            undetermined && established,
            "夹具须覆盖未定与已定两种 W 状态"
        );
        assert_eq!(
            request_kinds.into_iter().collect::<Vec<_>>(),
            [CC004_INITIALIZATION_KIND, CC004_UNDETERMINED_KIND],
            "请求须覆盖两种 W 对象"
        );
    }

    /// F2 负控：改 axes、subject 指向另一实际对象、subject 悬空、追加重复请求，均被判出。
    #[test]
    fn w_request_semantics_detects_axes_subject_and_duplicate_changes() {
        use crate::content_producer::{CC004_INITIALIZATION_KIND, CC004_UNDETERMINED_KIND};
        let events = delta_long_fixture();
        let (historical, _) = bounded_layout_runs(&events, &[1]).remove(0);
        let (_, current) = bounded2_layout_runs(&events, &[1]).remove(0);
        for state in [historical, current] {
            let original = w_request_semantics(&state).unwrap();
            assert_eq!(original.len(), 2);
            let other = objects_of(&state, CC004_UNDETERMINED_KIND)[0]["object_id"].clone();
            let mutate = |edit: &dyn Fn(&mut Vec<Value>)| -> Value {
                let mut copy = state.clone();
                let knowledge = copy["objects"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|o| o["kind"] == CC054_KNOWLEDGE_KIND)
                    .unwrap();
                let mut payload: Value =
                    serde_json::from_str(knowledge["payload_json"].as_str().unwrap()).unwrap();
                let mut requests = payload["requests"].as_array().unwrap().clone();
                edit(&mut requests);
                payload["requests"] = Value::Array(requests);
                knowledge["payload_json"] = json!(payload.to_string());
                copy
            };
            let init_index = |rs: &Vec<Value>| {
                rs.iter()
                    .position(|r| {
                        r["request_id"]
                            .as_str()
                            .unwrap()
                            .contains(CC004_INITIALIZATION_KIND)
                    })
                    .unwrap()
            };
            assert_eq!(w_request_semantics(&mutate(&|_| {})).unwrap(), original);
            let axes = mutate(&|rs| {
                let i = init_index(rs);
                rs[i]["axes"]["validity"] = json!("STALE");
            });
            assert_ne!(
                w_request_semantics(&axes).unwrap(),
                original,
                "axes 改变须判出"
            );
            let swapped = mutate(&|rs| {
                let i = init_index(rs);
                rs[i]["subject_id"] = other.clone();
            });
            assert_ne!(
                w_request_semantics(&swapped).unwrap(),
                original,
                "subject 指向另一实际对象须判出"
            );
            let dangling = mutate(&|rs| {
                let i = init_index(rs);
                rs[i]["subject_id"] = json!("obj3-missing");
            });
            assert!(
                w_request_semantics(&dangling).is_err(),
                "subject 悬空须判出"
            );
            let duplicate = mutate(&|rs| {
                let i = init_index(rs);
                let copy = rs[i].clone();
                rs.push(copy);
            });
            assert!(w_request_semantics(&duplicate).is_err(), "重复请求须判出");
        }
    }

    #[test]
    fn bounded2_crosswalk_rejects_non_w_subject_and_fractal_changes() {
        let events = delta_long_fixture();
        let (sorted, bounded) = bounded2_layout_runs(&events, &[events.len()]).remove(0);
        let shapes = objects_of(&bounded, CC006_SHAPE_KIND);
        assert!(shapes.len() >= 2);
        assert!(shapes.iter().all(|o| o.get("fact_key").is_none()));
        let window = |shape: &Value| {
            json!([
                shape["window_start"],
                shape["window_mid"],
                shape["window_end"]
            ])
        };
        assert_ne!(window(&shapes[0]), window(&shapes[1]));
        eprintln!("CC006 subject counterexample {}", json!(shapes));
        let subjects: std::collections::BTreeSet<_> = requests_of(&bounded)
            .iter()
            .filter_map(|r| r["subject_id"].as_str().map(str::to_string))
            .collect();
        let kinds: std::collections::BTreeSet<_> = bounded["objects"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| subjects.contains(o["object_id"].as_str().unwrap()))
            .map(|o| o["kind"].as_str().unwrap().to_string())
            .collect();
        eprintln!("crosswalk subject kinds {kinds:?}");
        assert_bounded2_crosswalk(1, &sorted, &bounded, &events);
        for mutation in [
            "shape_subject",
            "non_w_axes",
            "non_w_coords",
            "fractal_refs",
            "fractal_coords",
            "fractal_lifecycle",
        ] {
            let mut changed = bounded.clone();
            if mutation.starts_with("fractal_") {
                let object = changed["objects"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|o| o["kind"] == CC007_FRACTAL_KIND)
                    .unwrap();
                match mutation {
                    "fractal_refs" => object["input_refs_json"] = json!("[]"),
                    "fractal_coords" => object["source_coords_json"] = json!("[]"),
                    "fractal_lifecycle" => object["first_known_generation"] = json!(999),
                    _ => unreachable!(),
                }
            } else {
                let knowledge = changed["objects"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|o| o["kind"] == CC054_KNOWLEDGE_KIND)
                    .unwrap();
                let mut payload: Value =
                    serde_json::from_str(knowledge["payload_json"].as_str().unwrap()).unwrap();
                let request = payload["requests"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["subject_id"] == shapes[0]["object_id"])
                    .unwrap();
                match mutation {
                    "shape_subject" => request["subject_id"] = shapes[1]["object_id"].clone(),
                    "non_w_axes" => request["axes"]["validity"] = json!("STALE"),
                    "non_w_coords" => request["source_coords"] = json!([]),
                    _ => unreachable!(),
                }
                knowledge["payload_json"] = json!(crate::canonical_json(&payload));
            }
            assert_ne!(changed, bounded, "{mutation} 必须实际改变夹具");
            assert!(
                std::panic::catch_unwind(|| {
                    assert_bounded2_crosswalk(1, &sorted, &changed, &events);
                })
                .is_err(),
                "{mutation} 必须被完整比较拒绝"
            );
            eprintln!("crosswalk rejected {mutation}");
        }
    }

    #[test]
    fn layout1_generation_has_no_development_batch_and_keeps_line_order_requests() {
        let events = delta_long_fixture();
        let runs = bounded_layout_runs(&events, &[events.len()]);
        let (sorted, _) = &runs[0];
        assert!(objects_of(sorted, CC007_DEVELOPMENT_BATCH_KIND).is_empty());
        // layout/1 线序：步骤请求在全部组请求之前（原配方次序，未按 window_start 重排）。
        let kinds: Vec<String> = requests_of(sorted)
            .iter()
            .map(|r| {
                let id: Value = serde_json::from_str(r["request_id"].as_str().unwrap()).unwrap();
                id[0].as_str().map_or("nested".to_string(), str::to_string)
            })
            .collect();
        let last_step = kinds.iter().rposition(|k| k == CC004_STEP_KIND).unwrap();
        let first_group = kinds.iter().position(|k| k == CC005_GROUP_KIND).unwrap();
        assert!(last_step < first_group, "layout/1 请求保持原线序");
    }
}
