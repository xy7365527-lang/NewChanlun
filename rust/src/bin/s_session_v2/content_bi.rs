//! #1394 采纳 C §5.3/5.5：S 生产侧的 TB-02-B 新笔证据内容构造器（七个 kind）。
//!
//! 规范正本：`.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md`
//! 第 5.3（身份配方与全文绑定）与 5.5（Delta 逻辑字段）节；采纳状态以同目录
//! `ADOPTION.json`、`SPEC-ADDENDUM.md` 为准。本模块接的是 **S3 生成态**：输入只有同次
//! parser 的 [`StrokeFacts`] 与本次有效原始账，**不重算任何结构判据**。
//!
//! **逻辑 oracle 只在测试里**：旧 `tb02b` 的 `build`/`Builder` 是冻结的逐字段 oracle
//! （见本文件 `#[cfg(test)]` 的 `parity_*` 用例）。生产路径既不调用 `tb02b::build`、
//! 也不调用 `tb02a_facts::Builder`，更不先造完整 `Value`/JSON 文本再拆图——载荷按
//! `BiFact`/`PairFact`/`Endpoint` 的字段**直接组装 `CValue`**；随历史增长的集合一律走内容根、
//! 共享序列与边（见 `RawCoords` 与 `DepsArrays`）。
//!
//! 三条不变量（与旧配方逐字段对齐，由对拍测试锁定）：
//!
//! 1. **实体 id 不变**：`fx:{事实代际}:{组锚}`、`bi:{事实代际}:{起点锚}`、
//!    `raw:{identity_key}@{revision}` 仍按旧配方给出；换的只有**对象身份**——`obj3-` 六字段
//!    配方，并如实传播到 `CC-055.relation.witness_object_id` 与 `CC-055.change_event.object_id`。
//! 2. **代际与首获知**：载荷里的 `object_generation` 是由原始事件序列导出的**事实代际**
//!    （历史插入/后续修订各计一次，与旧 `fact_basis` 同一条语义，每次**全扫** `all_events`——
//!    这是现有语义，本模块不冒充服务级增量）；记录里的 `first_known_generation`/
//!    `published_generation` 是本次生成态代号——实际首获知元数据仍由发布链的独立索引核定。
//! 3. **重用而非复制**：同一 bi 的对象、全部关系边与就地变更事件共用**同一个**依赖根；
//!    `before`/`after`/`formed_known_at`/`formed_evidence`/`confirmed_known_at`/`version_causes`
//!    直接引用已发布内容根（`Value::Edge`），不复制值、不整展开上一代对象。
//!
//! **上一代对象是信任边界**：`PriorState` 只交内容记录根，本模块先对每条记录做**完整闭包
//! 核验**（`s_content::verify`），再核 `kind` / 事实键 / `payload`、`input_refs` 边形状，并用
//! **本构造器当前的档案 / 规则修订**按同一条六字段配方重导 `object_id` 逐字对比（见
//! [`verify_prior_object`]）；固定标量按真实语法（规范十进制 i64、已定义枚举）核，确认的来源
//! 坐标按内容数组**逐下标读叶**。这里**不设**「第 N 字节 / 第 N 项」这类发明上限：读多少由
//! 记录自身的节点决定，增长型集合留在内容根 / 共享序列上，整块渲染与整树复制都不做。
//!
//! **上一代的契约（照实）**：选哪条记录当「上一代活跃对象」由调用方的发布链独立索引负责
//! （见 [`PriorState`]）。本模块证明的是「这条记录按当前构造器自洽」，**不是**「调用方选对了
//! 记录」；后者不在这里假装验证。
//!
//! 拒绝面（不兜底）：账内缺依赖坐标、上一代对象形状/字段不符、原始行缺字段、上一代确认缺
//! 确认获知时间，一律 `Err`；不回落旧 `Value`、不静默跳过、不造默认值。
//!
//! 本模块由根入口挂载（`mod content_bi;`）；挂线前其 API 只被本文件测试使用，故整模块抑制
//! `dead_code`（与 `content_producer` 同口径，而不是逐项 allow 免得漏掉某个入口）。

#![allow(dead_code)]

use super::content_producer::{array_from_sequence, Cc004Producer, ProducerError, TypedObject};
use super::*;
use newchan_rust::s_content::capture_allocation::audit_json::{ArrayView, JsonArray, JsonView};
use newchan_rust::s_content::path::{lookup_path, PathStep};
use newchan_rust::s_content::{
    self as sc, Built, CodecError, ContentRef, Kind, NodeDigest, NodeStore, SequenceRef,
    Value as CValue,
};
use newchan_rust::theta_v0::parser::stroke::{
    Conditions, Confirmation, Endpoint, PairFact, StrokeFacts, RULE,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// 本模块唯一负责的七个 kind（与旧 `tb02b::KINDS` 同序同名）。
pub(super) const KINDS: [&str; 7] = [
    ENDPOINT_KIND,
    NEW_BI_PAIR_KIND,
    SAME_KIND_KIND,
    BI_KIND,
    RELATION_KIND,
    CHANGE_KIND,
    VERSION_KIND,
];
const ENDPOINT_KIND: &str = "CC-008.endpoint";
const NEW_BI_PAIR_KIND: &str = "CC-008.new_bi_pair";
const SAME_KIND_KIND: &str = "CC-009.same_kind";
const BI_KIND: &str = "CC-010.bi";
const RELATION_KIND: &str = "CC-055.relation";
const CHANGE_KIND: &str = "CC-055.change_event";
const VERSION_KIND: &str = "CC-056.version";
/// CC-056 的槽位常量（`TB-02-B` 版本对象）。
const VERSION_SLOT: &str = "TB-02-B";
/// 载荷头三常量（与旧配方逐字相同）。
const PAYLOAD_SCHEMA: &str = "s-new-bi/1";
const PAYLOAD_POLICY: &str = "standard_raw_gap_3";
const PAYLOAD_VIEW_ROLE: &str = "main";
/// 变更事件的固定原因轴（与旧配方同值 `stroke::RULE`）。
const CHANGE_CAUSE: &str = RULE;

/// TB-02-B 内容构造错误：缺源、形状不符、旧对象字段异常一律显式拒绝，不静默跳过。
#[derive(Debug)]
pub(super) enum BiError {
    /// 内容生产层错误（源版本缺失、共享态退化、编解码）。
    Content(ProducerError),
    /// 原始账缺该坐标或缺该字段；旧配方在 `raw_ref`/下标处同样硬失败。
    Ledger { coord: i64, what: &'static str },
    /// 上一代内容对象形状/字段/闭包不符（含根不一致）。
    Prior { what: &'static str, detail: String },
    /// 同核事实缺项（例如端点没有极值根、坐标超出 i64 表达）。
    Facts { detail: String },
}

impl From<ProducerError> for BiError {
    fn from(error: ProducerError) -> Self {
        BiError::Content(error)
    }
}

impl From<CodecError> for BiError {
    fn from(error: CodecError) -> Self {
        BiError::Content(ProducerError::from(error))
    }
}

impl fmt::Display for BiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BiError::Content(error) => write!(f, "{error}"),
            BiError::Ledger { coord, what } => {
                write!(f, "原始账坐标 {coord} 缺字段 `{what}`")
            }
            BiError::Prior { what, detail } => {
                write!(f, "上一代内容对象 `{what}` 不符：{detail}")
            }
            BiError::Facts { detail } => write!(f, "同核事实缺项：{detail}"),
        }
    }
}

impl std::error::Error for BiError {}

/// 源坐标 `usize → i64`（账本键域）。
fn coord_i64(coord: usize) -> Result<i64, BiError> {
    i64::try_from(coord).map_err(|_| BiError::Facts {
        detail: format!("源坐标 {coord} 超出 i64 表达域"),
    })
}

/// 事实代际（与旧 `tb02b::fact_basis` 同一条语义，逐字段对齐）：
/// 按原始事件到达序，每条**后续修订**（`revision > 1`）或落在此前最大源坐标内的
/// **历史插入**各计一次；普通尾部追加不计。每次**全扫** `all_events`。
///
/// 这是生产实现；`tb02b::fact_basis` 只是冻结测试 oracle（旧模块不再进生产路径）。
fn fact_generation(all_events: ArrayView<'_>) -> Result<i64, BiError> {
    let mut max_coord = i64::MIN;
    let mut generation: i64 = 1;
    for event in all_events.iter() {
        let coord = event_coord(event)?;
        let revision = event
            .get("revision")
            .and_then(JsonView::as_i64)
            .ok_or_else(|| BiError::Facts {
                detail: "原始事件缺 i64 修订号 revision".to_string(),
            })?;
        generation = generation
            .checked_add(i64::from(revision > 1 || coord < max_coord))
            .ok_or_else(|| BiError::Facts {
                detail: "事实代际超出 i64 表达域".to_string(),
            })?;
        max_coord = max_coord.max(coord);
    }
    Ok(generation)
}

/// 原始事件里的源坐标文本（旧 `stored_source_coord` 的同一条判据：规范十进制 + 非负）。
fn event_coord(event: JsonView<'_>) -> Result<i64, BiError> {
    let text = event
        .get("source_coord")
        .and_then(JsonView::as_str)
        .ok_or_else(|| BiError::Facts {
            detail: "原始事件缺 source_coord 文本".to_string(),
        })?;
    let coord =
        parse_canonical_i64(text, "source_coord").map_err(|detail| BiError::Facts { detail })?;
    usize::try_from(coord).map_err(|_| BiError::Facts {
        detail: format!("source_coord `{text}` 不是本平台可无损表示的非负源位置"),
    })?;
    Ok(coord)
}

/// 原始账只借用真实所有者；坐标迭代及闭区间与旧 BTreeMap 同序，不分配投影。
#[derive(Clone, Copy)]
pub(super) enum BiLedgerView<'a> {
    Plain(&'a BTreeMap<i64, Value>),
    Views(&'a BTreeMap<i64, JsonView<'a>>),
}

impl<'a> BiLedgerView<'a> {
    fn len(self) -> usize {
        match self {
            Self::Plain(rows) => rows.len(),
            Self::Views(rows) => rows.len(),
        }
    }

    fn get(self, coord: i64) -> Option<JsonView<'a>> {
        match self {
            Self::Plain(rows) => rows.get(&coord).map(JsonView::from),
            Self::Views(rows) => rows.get(&coord).copied(),
        }
    }

    fn iter(self) -> BiRows<'a> {
        match self {
            Self::Plain(rows) => BiRows::Plain(rows.iter()),
            Self::Views(rows) => BiRows::Views(rows.iter()),
        }
    }

    fn range(self, start: i64, end: i64) -> BiRows<'a> {
        match self {
            Self::Plain(rows) => BiRows::PlainRange(rows.range(start..=end)),
            Self::Views(rows) => BiRows::ViewsRange(rows.range(start..=end)),
        }
    }
}

enum BiRows<'a> {
    Plain(std::collections::btree_map::Iter<'a, i64, Value>),
    Views(std::collections::btree_map::Iter<'a, i64, JsonView<'a>>),
    PlainRange(std::collections::btree_map::Range<'a, i64, Value>),
    ViewsRange(std::collections::btree_map::Range<'a, i64, JsonView<'a>>),
}

impl<'a> Iterator for BiRows<'a> {
    type Item = (i64, JsonView<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Plain(rows) => rows.next().map(|(coord, row)| (*coord, row.into())),
            Self::Views(rows) => rows.next().map(|(coord, row)| (*coord, *row)),
            Self::PlainRange(rows) => rows.next().map(|(coord, row)| (*coord, row.into())),
            Self::ViewsRange(rows) => rows.next().map(|(coord, row)| (*coord, *row)),
        }
    }
}

/// 旧 slice 入口的无分配数组投影，生命周期随真实 slice，不复制事件。
struct PlainEvents<'a>(&'a [Value]);
impl JsonArray for PlainEvents<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<JsonView<'_>> {
        self.0.get(index).map(JsonView::from)
    }
}

/// 本次有效原始账的**有界视图**：只读键集与逐行标量，不复制整行、不物化依赖数组。
struct Ledger<'a> {
    rows: BiLedgerView<'a>,
}

impl<'a> Ledger<'a> {
    fn new(rows: &'a BTreeMap<i64, Value>) -> Self {
        Ledger {
            rows: BiLedgerView::Plain(rows),
        }
    }

    fn row(&self, coord: i64) -> Result<JsonView<'a>, BiError> {
        self.rows.get(coord).ok_or_else(|| BiError::Facts {
            detail: format!("依赖坐标 {coord} 不在本次有效原始账"),
        })
    }

    fn text(&self, coord: i64, field: &'static str) -> Result<&'a str, BiError> {
        self.row(coord)?
            .get(field)
            .and_then(JsonView::as_str)
            .ok_or(BiError::Ledger { coord, what: field })
    }

    fn int(&self, coord: i64, field: &'static str) -> Result<i64, BiError> {
        self.row(coord)?
            .get(field)
            .and_then(JsonView::as_i64)
            .ok_or(BiError::Ledger { coord, what: field })
    }

    /// 最后到达的行：`seq` 最大；同 `seq` 取账内迭代序最后一条（与旧 `max_by_key` 同口径）。
    fn latest(&self) -> Option<JsonView<'a>> {
        let mut best: Option<(JsonView<'a>, i64)> = None;
        for (_, row) in self.rows.iter() {
            let Some(seq) = row.get("seq").and_then(JsonView::as_i64) else {
                continue;
            };
            if best.is_none_or(|(_, best_seq)| seq >= best_seq) {
                best = Some((row, seq));
            }
        }
        best.map(|(row, _)| row)
    }
}

/// 一个对象的依赖面：两条 `0x05` 数组边。
///
/// `input_refs` 是当前源版本下的八字段源引用数组，`source_coords` 是坐标十进制文本数组；
/// 同一个 `DepsArrays` 被同一 bi 的对象与全部关系边复用（同值同根，不逐条复制）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DepsArrays {
    pub input_refs: Built,
    pub source_coords: Built,
}

/// 全账本升序坐标的**增量内容序列**（一个 S3 车道一份；跨代续接，不整段重建）。
///
/// - `text`：叶 = 坐标十进制文本值根（`Cc004Producer::coord_text_leaf`，与
///   `Cc004Producer::coords_array` **同一张叶图、同一份 memo**）；坐标文本不随源修订变化，
///   故永不失效；
/// - `refs`：叶 = 当前源版本的八字段引用值根；`refs_leaves` 是与 `coords` 对齐的**水位索引**。
///   `RawCoords::sync` 逐位与 `Cc004Producer::source_root` 比对，**实际变化**的最早位置起
///   重建尾部（`sequence_split` 取前缀 + 逐叶 `sequence_append`），未变前缀的节点原样保留；
/// - `coords` 是区间定位用的升序坐标索引（与调用方账本同量级），不是内容物化。
struct RawCoords {
    coords: Vec<i64>,
    refs_leaves: Vec<NodeDigest>,
    text: SequenceRef,
    refs: SequenceRef,
}

impl RawCoords {
    fn new() -> Self {
        RawCoords {
            coords: Vec::new(),
            refs_leaves: Vec::new(),
            text: SequenceRef::empty(),
            refs: SequenceRef::empty(),
        }
    }

    /// 与当前账本对齐（坐标集差异与**实际源版本**差异都从首个不同位置起重建尾部）。
    ///
    /// **失败不留半更新态**：新尾部先写进局部变量，全部 `put` 成功后才一次性提交到
    /// `self`（提交本身不再触碰 store，不会失败）。任一步 `Err` 都保持 `self` 为上一次
    /// 成功的整根，因此重试只会从头重建尾部，不会凭索引长度或旧序列混拼两域内容。
    /// 跨事务 / 换库的显式丢弃见 [`RawCoords::discard_runtime_state`]。
    fn sync(
        &mut self,
        store: &mut dyn NodeStore,
        producer: &mut Cc004Producer,
        ledger: &Ledger<'_>,
    ) -> Result<(), BiError> {
        let common = self.coords.len().min(ledger.rows.len());
        let mut first = common;
        for (index, (&mine, (theirs, _))) in self.coords.iter().zip(ledger.rows.iter()).enumerate()
        {
            if mine != theirs {
                first = index;
                break;
            }
        }
        for (index, &coord) in self.coords[..first].iter().enumerate() {
            let root = producer
                .source_root(coord)
                .ok_or(ProducerError::MissingSource(coord))?;
            if self.refs_leaves[index] != root {
                first = index;
                break;
            }
        }
        if first == self.coords.len() && self.coords.len() == ledger.rows.len() {
            return Ok(());
        }
        let (text_prefix, _) = sc::sequence_split(&mut *store, &self.text, first as u64)?;
        let (refs_prefix, _) = sc::sequence_split(&mut *store, &self.refs, first as u64)?;
        let mut text = text_prefix;
        let mut refs = refs_prefix;
        let mut tail: Vec<i64> = Vec::new();
        let mut tail_refs: Vec<NodeDigest> = Vec::new();
        for (coord, _) in ledger.rows.iter().skip(first) {
            let leaf = producer.coord_text_leaf(&mut *store, coord)?;
            text = sc::sequence_append(&mut *store, &text, &leaf)?;
            let source = producer
                .source_root(coord)
                .ok_or(ProducerError::MissingSource(coord))?;
            refs = sc::sequence_append(&mut *store, &refs, &source)?;
            tail.push(coord);
            tail_refs.push(source);
        }
        self.coords.truncate(first);
        self.refs_leaves.truncate(first);
        self.coords.extend(tail);
        self.refs_leaves.extend(tail_refs);
        self.text = text;
        self.refs = refs;
        Ok(())
    }

    /// 丢弃本车道的运行时序列（事务 rollback / 换库 / 换来源域）：回到空根，下次 `sync` 整体重建。
    ///
    /// 与 [`Cc004Producer::discard_runtime_state`] 同一条纪律：**必须**在明确边界调用，且两者
    /// 要一起调（见 [`ContentBi::discard_runtime_state`]）——只清一边会让旧序列根继续参与切片。
    /// 普通 append 上**不得**调用：旧前缀的内容与共享序列仍然有效。
    fn discard_runtime_state(&mut self) {
        *self = RawCoords::new();
    }

    /// 账内位置（升序索引）；不在账内即拒绝（旧配方在 `raw_ref` 处同样硬失败）。
    fn position(&self, coord: i64) -> Result<u64, BiError> {
        self.coords
            .binary_search(&coord)
            .map(|index| index as u64)
            .map_err(|_| BiError::Ledger {
                coord,
                what: "本次有效原始账坐标",
            })
    }

    /// 位置段 → 共享序列切片（每段一次 `sequence_split`，不展开逻辑叶）。
    fn runs_sequence(
        &self,
        store: &mut dyn NodeStore,
        runs: &[(u64, u64)],
    ) -> Result<(SequenceRef, SequenceRef), BiError> {
        let mut text: Option<SequenceRef> = None;
        let mut refs: Option<SequenceRef> = None;
        for (start, end) in runs {
            let piece = slice_sequence(&mut *store, &self.text, *start, *end)?;
            text = Some(match text {
                Some(previous) => sc::sequence_concat(&mut *store, &previous, &piece)?,
                None => piece,
            });
            let piece = slice_sequence(&mut *store, &self.refs, *start, *end)?;
            refs = Some(match refs {
                Some(previous) => sc::sequence_concat(&mut *store, &previous, &piece)?,
                None => piece,
            });
        }
        Ok((
            text.unwrap_or_else(SequenceRef::empty),
            refs.unwrap_or_else(SequenceRef::empty),
        ))
    }

    /// 位置段 → 依赖面（两条数组边；数组节点由 `U64(叶数) || M 根` 一次成帧）。
    fn arrays_of_runs(
        &self,
        store: &mut dyn NodeStore,
        runs: &[(u64, u64)],
    ) -> Result<DepsArrays, BiError> {
        let (text, refs) = self.runs_sequence(store, runs)?;
        Ok(DepsArrays {
            input_refs: array_from_sequence(&mut *store, &refs)?,
            source_coords: array_from_sequence(&mut *store, &text)?,
        })
    }

    /// 全账本（升序全部坐标）依赖面：旧配方里 `sources = raw.keys()` 的那一份。
    fn whole(&self, store: &mut dyn NodeStore) -> Result<DepsArrays, BiError> {
        let count = u64::try_from(self.coords.len()).map_err(|_| BiError::Facts {
            detail: "账本坐标数超出 u64".to_string(),
        })?;
        self.arrays_of_runs(store, &[(0, count)])
    }

    /// 任意坐标集（端点来源集）：排序去重后按位置合并连续段，每段一次切片。
    fn coord_set(
        &self,
        store: &mut dyn NodeStore,
        coords: &[usize],
    ) -> Result<DepsArrays, BiError> {
        let mut xs: Vec<i64> = Vec::with_capacity(coords.len());
        for coord in coords {
            xs.push(coord_i64(*coord)?);
        }
        xs.sort_unstable();
        xs.dedup();
        let mut runs: Vec<(u64, u64)> = Vec::new();
        for coord in xs {
            let position = self.position(coord)?;
            push_run(&mut runs, position, position + 1);
        }
        self.arrays_of_runs(store, &runs)
    }

    /// 旧 `interval_sources` 的同一条语义：`union(a,b)` ∪ `{账内坐标 ∈ [min,max]}` ∪ `extras`。
    ///
    /// 三处都按**账内位置**落在同一个升序坐标序列上，所以区间是 `sequence_split` 的一段切片；
    /// 落在区间外的 `extras`（旧配方里是沿用确认的 `source_coords`）按位置拼在两侧。
    fn interval(
        &self,
        store: &mut dyn NodeStore,
        a: &[usize],
        b: &[usize],
        extras: &[usize],
    ) -> Result<DepsArrays, BiError> {
        let mut xs: Vec<i64> = Vec::with_capacity(a.len() + b.len());
        for coord in a.iter().chain(b.iter()) {
            xs.push(coord_i64(*coord)?);
        }
        xs.sort_unstable();
        xs.dedup();
        // 全部 deps 坐标都必须在账内：旧配方在 `raw_ref` 处同样硬失败，不静默丢坐标。
        for coord in xs.iter().copied() {
            self.position(coord)?;
        }
        let mut extra: Vec<i64> = Vec::with_capacity(extras.len());
        for coord in extras {
            let coord = coord_i64(*coord)?;
            self.position(coord)?;
            extra.push(coord);
        }
        let mut runs: Vec<(u64, u64)> = Vec::new();
        if xs.is_empty() {
            extra.sort_unstable();
            extra.dedup();
            for coord in extra {
                let position = self.position(coord)?;
                push_run(&mut runs, position, position + 1);
            }
        } else {
            let first = xs[0];
            let last = *xs.last().expect("刚核过非空");
            let start = self.position(first)?;
            let end = self.position(last)? + 1;
            extra.sort_unstable();
            extra.dedup();
            for coord in extra.iter().copied().filter(|coord| *coord < first) {
                let position = self.position(coord)?;
                push_run(&mut runs, position, position + 1);
            }
            push_run(&mut runs, start, end);
            for coord in extra.iter().copied().filter(|coord| *coord > last) {
                let position = self.position(coord)?;
                push_run(&mut runs, position, position + 1);
            }
        }
        self.arrays_of_runs(store, &runs)
    }
}

/// 合并相邻/重叠的位置段（调用序升序；`end` 为开区间）。
fn push_run(runs: &mut Vec<(u64, u64)>, start: u64, end: u64) {
    if start >= end {
        return;
    }
    if let Some(last) = runs.last_mut() {
        if start <= last.1 {
            last.1 = last.1.max(end);
            return;
        }
    }
    runs.push((start, end));
}

/// `M` 序列切片 `[start, end)`：**只走规范切点**——先按叶数把区间拆成 O(log n) 个对齐子树
/// 块，再逐块自根下行取子树根（每级切在 [`sc::split_index`]），最后按序 `concat`。
///
/// 为什么不「切 `end` 再切 `start`」：`s_content::sequence` 的「成本」段已照实声明
/// concat/split 的最坏复杂度未证明，实测非规范边界切一刀的**写次数随叶数线性增长**
/// （1024 叶在 3/4 边界切一刀写 376 个节点）。区间依赖面挂在每条关系上，两次 split 会把
/// 这份放大重复 N 次；对齐块覆盖每级只切规范边界、块子树与共享序列**同根**，不复制叶、
/// 不展开整段。**实测**：64 / 1024 叶账本的区间数组各写 6 个节点、全账本数组 ≤ 4 个
/// （`dependency_slices_do_not_scale_with_ledger_size` 锁）；两条切片路径同根由
/// `slice_sequence_tiles_and_matches_the_naive_splits` 逐例锁定。
fn slice_sequence(
    store: &mut dyn NodeStore,
    base: &SequenceRef,
    start: u64,
    end: u64,
) -> Result<SequenceRef, BiError> {
    if start >= end {
        return Ok(SequenceRef::empty());
    }
    if start == 0 && end == base.count() {
        return Ok(*base);
    }
    let mut out: Option<SequenceRef> = None;
    for (lo, len) in cover_blocks(base.count(), start, end) {
        let piece = aligned_block(&mut *store, base, lo, len)?;
        out = Some(match out {
            Some(previous) => sc::sequence_concat(&mut *store, &previous, &piece)?,
            None => piece,
        });
    }
    out.ok_or_else(|| CodecError::new("sequence_cover", "非空区间没有覆盖块").into())
}

/// `[start, end)` 在**叶数 `count` 的规范 `M` 树**上的对齐块覆盖（按序、无缝、无重叠）。
///
/// 纯函数：只用叶数与 `split_index` 的规范分解形状，不读树、不物化叶；块边界都落在某层
/// 规范切点上，故 [`aligned_block`] 的每一次下行都只切规范边界。
fn cover_blocks(count: u64, start: u64, end: u64) -> Vec<(u64, u64)> {
    let mut out: Vec<(u64, u64)> = Vec::new();
    collect_blocks(count, 0, start, end, &mut out);
    out
}

fn collect_blocks(count: u64, lo: u64, start: u64, end: u64, out: &mut Vec<(u64, u64)>) {
    let hi = lo + count;
    if count == 0 || end <= lo || hi <= start {
        return;
    }
    if start <= lo && hi <= end {
        out.push((lo, count));
        return;
    }
    let Some(k) = sc::split_index(count) else {
        // `count == 1` 的叶要么整块被覆盖、要么与区间不相交（块边界与叶边界对齐）；
        // 真走到这里说明覆盖不完整，由调用方按内部不一致拒绝。
        return;
    };
    collect_blocks(k, lo, start, end, out);
    collect_blocks(count - k, lo + k, start, end, out);
}

/// 取 `[offset, offset+len)` 这一**对齐块**的子树根：自根逐级在规范切点下行。
///
/// 每级 `sequence_split` 都切在 `index == k` 处（直接返回左右子树，不重写非边界节点），
/// 返回的根与共享序列上的同一子树**同根**。
fn aligned_block(
    store: &mut dyn NodeStore,
    base: &SequenceRef,
    offset: u64,
    len: u64,
) -> Result<SequenceRef, BiError> {
    let mut current = *base;
    let mut off = offset;
    let want = len;
    loop {
        if off == 0 && want == current.count() {
            return Ok(current);
        }
        let Some(k) = sc::split_index(current.count()) else {
            return Err(CodecError::new(
                "sequence_cover",
                format!("块 [{offset}, {}) 不是规范子树边界", offset + len),
            )
            .into());
        };
        let (left, right) = sc::sequence_split(&mut *store, &current, k)?;
        if off + want <= k {
            current = left;
        } else if off >= k {
            current = right;
            off -= k;
        } else {
            return Err(CodecError::new(
                "sequence_cover",
                format!("块 [{offset}, {}) 跨规范切点 {k}", offset + len),
            )
            .into());
        }
    }
}

/// 上一代发布态：**只交内容记录根**，不交 `previous Vec<Value>`。
///
/// **契约**：`bi_records` / `version_record` 是调用方按发布链的独立索引选出的上一代活跃
/// `CC-010.bi` 与 `CC-056.version` 记录根。本模块对每条记录做**完整闭包核验**
/// （`s_content::verify`）、`kind` / 事实键 / `payload`、`input_refs` 边形状核验，并用
/// **当前构造器的档案 / 规则修订**重导 `object_id` 逐字对比（见 [`verify_prior_object`]）。
/// 因此「记录内容与 id 自洽」由本模块证明；「这条记录确属上一代活跃发布态、上一代索引本身
/// 完整」仍属调用方契约，本模块不声称验证了那一层。
///
/// 两处**照实的代价/收紧**：
/// - 完整闭包核验的成本随该记录来源数组的规模增长（信任边界不做抽样、不做「摘要相同即放行」）；
/// - 同一实体重复出现即 `Err`——旧路径的 `BTreeMap` `.collect()` 会静默覆盖，这是显式收紧。
pub(super) struct PriorState<'a> {
    pub bi_records: Vec<&'a Built>,
    pub version_record: Option<&'a Built>,
}

/// 上一代一条 `CC-010.bi` 的读数（内容根 + 已核标量）。
#[derive(Debug)]
struct PriorBi {
    entity_id: String,
    entity_revision: i64,
    end_group_anchor: i64,
    start: ContentRef,
    end: ContentRef,
    data: ContentRef,
    confirmation: Option<ContentRef>,
    confirmation_successor_start: Option<String>,
    confirmation_source_coords: Option<Vec<usize>>,
    formed_known_at: ContentRef,
    formed_evidence: ContentRef,
    confirmed_known_at: Option<ContentRef>,
    version_causes: ContentRef,
}

/// 上一代发布态的索引：活跃 bi 记录 + 由版本对象给出的当前事实代际。
#[derive(Debug)]
struct PriorIndex {
    bi: BTreeMap<String, PriorBi>,
    generation: i64,
}

impl PriorIndex {
    fn read(
        producer: &Cc004Producer,
        store: &mut dyn NodeStore,
        prior: &PriorState<'_>,
    ) -> Result<Self, BiError> {
        let mut bi: BTreeMap<String, PriorBi> = BTreeMap::new();
        for record in &prior.bi_records {
            let read = PriorBi::read(producer, &mut *store, *record)?;
            if bi.insert(read.entity_id.clone(), read).is_some() {
                return Err(BiError::Prior {
                    what: "entity_id",
                    detail: "上一代 CC-010.bi 出现重复实体".to_string(),
                });
            }
        }
        let generation = match prior.version_record {
            Some(record) => {
                let verified = verify_prior_object(store, producer, record, VERSION_KIND)?;
                let payload = &verified.payload;
                let slot = scalar_text(store, payload, &field_path("slot"))?;
                if slot != verified.slot || slot != VERSION_SLOT {
                    return Err(BiError::Prior {
                        what: "slot",
                        detail: format!(
                            "上一代版本对象的事实键槽位 `{}` 与载荷槽位 `{slot}`（应为 `{VERSION_SLOT}`）不一致",
                            verified.slot
                        ),
                    });
                }
                scalar_i64(store, payload, &field_path("object_generation"))?
            }
            None => 1,
        };
        Ok(PriorIndex { bi, generation })
    }
}

impl PriorBi {
    /// 公共身份核验 + 读数（只读路径触碰的节点，不展开数据正文）。
    fn read(
        producer: &Cc004Producer,
        store: &mut dyn NodeStore,
        record: &Built,
    ) -> Result<Self, BiError> {
        let verified = verify_prior_object(store, producer, record, BI_KIND)?;
        let payload = verified.payload;
        let data = resolve(store, &payload, &field_path("data"))?;
        if data.kind != Kind::Object {
            return Err(BiError::Prior {
                what: "data",
                detail: format!("上一代 bi 数据不是对象（{:?}）", data.kind),
            });
        }
        let entity_id = scalar_text(store, &data, &field_path("entity_id"))?;
        parse_bi_entity(&entity_id)?;
        let payload_slot = scalar_text(store, &payload, &field_path("slot"))?;
        if payload_slot != verified.slot || verified.slot != entity_id {
            return Err(BiError::Prior {
                what: "slot/entity_id",
                detail: format!(
                    "事实键槽位 `{}`、载荷槽位 `{payload_slot}` 与实体 `{entity_id}` 不一致",
                    verified.slot
                ),
            });
        }
        let entity_revision = scalar_i64(store, &data, &field_path("entity_revision"))?;
        let end_group_anchor = scalar_i64(store, &data, &nested_path(&["end", "group_anchor"]))?;
        let start = resolve(store, &data, &field_path("start"))?;
        let end = resolve(store, &data, &field_path("end"))?;
        let formed_known_at = resolve(store, &data, &field_path("formed_known_at"))?;
        let formed_evidence = resolve(store, &data, &field_path("formed_evidence"))?;
        let confirmed_known_at = optional_object(store, &data, &field_path("confirmed_known_at"))?;
        let version_causes = resolve(store, &data, &field_path("version_causes"))?;
        let confirmation = optional_object(store, &data, &field_path("confirmation"))?;
        let (confirmation_successor_start, confirmation_source_coords) = match &confirmation {
            Some(reference) => (
                Some(scalar_text(
                    store,
                    reference,
                    &field_path("successor_start"),
                )?),
                Some(coord_array(store, reference, &field_path("source_coords"))?),
            ),
            None => (None, None),
        };
        Ok(PriorBi {
            entity_id,
            entity_revision,
            end_group_anchor,
            start,
            end,
            data,
            confirmation,
            confirmation_successor_start,
            confirmation_source_coords,
            formed_known_at,
            formed_evidence,
            confirmed_known_at,
            version_causes,
        })
    }
}

/// 上一代一条 `obj3` 记录的公共核验结果：已核的载荷边 + 事实键槽位。
struct PriorRecord {
    payload: ContentRef,
    slot: String,
}

/// 上一代记录的**完整闭包核验**（`verify` 走全树；边指向的子根也在同一读集内）。
fn verify_prior_root(store: &mut dyn NodeStore, record: &Built) -> Result<(), BiError> {
    if record.kind != Kind::Object {
        return Err(BiError::Prior {
            what: "record.kind",
            detail: format!("上一代记录不是对象（{:?}）", record.kind),
        });
    }
    sc::verify(&mut *store, &record.content_ref())?;
    Ok(())
}

/// 上一代一条 `obj3` 记录的公共核验：完整闭包 → `kind` → 事实键形状 → 六字段身份重导 →
/// 载荷头四常量（[`verify_payload_header`]）。
///
/// 身份用**本构造器当前的档案 / 规则修订**重导（[`Cc004Producer::typed_object_id`] 与产出侧
/// 共用同一条 `V({kind,fact_key,payload,input_refs,profile_id,rule_revision})` 配方），再与本条
/// 记录声明的 `object_id` 逐字对比。**同一 id 换载荷即拒绝**：换 `payload`、换 `input_refs`、
/// 换 `fact_key` 或换档案 / 规则修订都会让重导 id 与声明不符。
///
/// 身份通过**不等于**协议语义成立：`schema_revision` / `semantic_version` / `policy_version` /
/// `view_role` 不进 id 判据（id 只绑值根），一条「头改错、又按错头重导 id」的记录在这里被独立拒绝。
///
/// **本函数不假装验证的部分**（见 [`PriorState`] 契约）：这条记录是否确属上一代活跃发布态、
/// 调用方的上一代索引本身是否完整、`payload.data` 的业务语义是否成立，都由调用方的发布链
/// 负责；本模块只证明「给定记录按当前构造器自洽」。
fn verify_prior_object(
    store: &mut dyn NodeStore,
    producer: &Cc004Producer,
    record: &Built,
    expect_kind: &str,
) -> Result<PriorRecord, BiError> {
    verify_prior_root(store, record)?;
    let reference = record.content_ref();
    let kind = scalar_text(store, &reference, &field_path("kind"))?;
    if kind != expect_kind {
        return Err(BiError::Prior {
            what: "kind",
            detail: format!("上一代记录 kind 实为 `{kind}`，应为 `{expect_kind}`"),
        });
    }
    let fact_key = scalar_text(store, &reference, &field_path("fact_key"))?;
    let slot = fact_key_slot(&fact_key, expect_kind)?;
    let payload = resolve(store, &reference, &field_path("payload"))?;
    if payload.kind != Kind::Object {
        return Err(BiError::Prior {
            what: "payload",
            detail: format!("上一代载荷不是对象（{:?}）", payload.kind),
        });
    }
    let input_refs = resolve(store, &reference, &field_path("input_refs"))?;
    if input_refs.kind != Kind::Array {
        return Err(BiError::Prior {
            what: "input_refs",
            detail: format!("上一代输入引用不是数组（{:?}）", input_refs.kind),
        });
    }
    let claimed = scalar_text(store, &reference, &field_path("object_id"))?;
    let derived = producer.typed_object_id(
        expect_kind,
        &fact_key,
        &built_of(&payload)?,
        &built_of(&input_refs)?,
    )?;
    if claimed != derived {
        return Err(BiError::Prior {
            what: "object_id",
            detail: format!(
                "记录声明 `{claimed}`，按本档案/规则修订重导为 `{derived}`；同 id 换载荷不接受"
            ),
        });
    }
    // 重导身份能证明载荷与 ID 相符，但不能证明载荷头等于本域的协议常量。
    verify_payload_header(store, &payload)?;
    Ok(PriorRecord { payload, slot })
}

/// 上一代载荷头四常量：`schema_revision` / `semantic_version` / `policy_version` / `view_role`
/// 必须逐字等于本域协议指定值（与产出侧 [`payload_value`] 同一条配方）。
///
/// 这四项参与内容身份；但修改头后重新求得的 ID 仍然自洽，所以还须独立核对指定常量。
fn verify_payload_header(store: &mut dyn NodeStore, payload: &ContentRef) -> Result<(), BiError> {
    for (field, expected) in [
        ("schema_revision", PAYLOAD_SCHEMA),
        ("semantic_version", RULE),
        ("policy_version", PAYLOAD_POLICY),
        ("view_role", PAYLOAD_VIEW_ROLE),
    ] {
        let actual = scalar_text(store, payload, &field_path(field))?;
        if actual != expected {
            return Err(BiError::Prior {
                what: field,
                detail: format!("载荷头 `{field}` 实为 `{actual}`，应为 `{expected}`"),
            });
        }
    }
    Ok(())
}

/// 由已定位的**真实节点**还原值根四字段（`extent` 来自节点本身，不重算、不展开闭包）。
fn built_of(reference: &ContentRef) -> Result<Built, BiError> {
    Ok(Built {
        digest: reference.digest()?,
        kind: reference.kind,
        extent: reference.extent,
    })
}

/// 事实键＝规范 JSON 文本 `[kind, slot]`：kind 必须与记录 kind 一致，且文本是该值的
/// **规范**文本（逐字，不放行等价 JSON 写法）；返回槽位文本供与载荷槽位对核。
fn fact_key_slot(fact_key: &str, expect_kind: &str) -> Result<String, BiError> {
    let value: Value = serde_json::from_str(fact_key).map_err(|error| BiError::Prior {
        what: "fact_key",
        detail: format!("事实键不是 JSON：{error}"),
    })?;
    let items = value.as_array().ok_or_else(|| BiError::Prior {
        what: "fact_key",
        detail: format!("事实键不是数组：{value}"),
    })?;
    if items.len() != 2 {
        return Err(BiError::Prior {
            what: "fact_key",
            detail: format!("事实键不是 `[kind, slot]` 两项：{value}"),
        });
    }
    let (Some(kind), Some(slot)) = (items[0].as_str(), items[1].as_str()) else {
        return Err(BiError::Prior {
            what: "fact_key",
            detail: format!("事实键两项必须都是字符串：{value}"),
        });
    };
    if kind != expect_kind {
        return Err(BiError::Prior {
            what: "fact_key",
            detail: format!("事实键 kind `{kind}` 与记录 kind `{expect_kind}` 不一致"),
        });
    }
    if canonical_json(&value) != fact_key {
        return Err(BiError::Prior {
            what: "fact_key",
            detail: "事实键不是该值的规范 JSON 文本".to_string(),
        });
    }
    Ok(slot.to_string())
}

/// `bi:{事实代际}:{起点锚}`：两段都必须过 `parse_canonical_i64`（真实语法）。
///
/// 这条语法同时给出实体文本的**长度上界**（`bi:` + 两段 i64 十进制，≤ 3+20+1+20 字节），
/// 因此不需要另立字节上限；文本与账内身份的一致性由调用处逐项核（事实键槽位 / 载荷槽位）。
fn parse_bi_entity(entity_id: &str) -> Result<(), BiError> {
    let rest = entity_id
        .strip_prefix("bi:")
        .ok_or_else(|| BiError::Prior {
            what: "entity_id",
            detail: format!("实体 id 不以 `bi:` 开头：`{entity_id}`"),
        })?;
    let (generation, anchor) = rest.split_once(':').ok_or_else(|| BiError::Prior {
        what: "entity_id",
        detail: format!("实体 id 不是 `bi:<代际>:<锚>`：`{entity_id}`"),
    })?;
    for (text, what) in [(generation, "事实代际"), (anchor, "起点锚")] {
        parse_canonical_i64(text, what).map_err(|detail| BiError::Prior {
            what: "entity_id",
            detail: format!("`{entity_id}` 的{what}不符语法：{detail}"),
        })?;
    }
    Ok(())
}

/// 一次生成的完整输入（同次 parser 事实 + 本次有效原始账 + 上一代内容对象边）。
pub(super) struct BiBuildInput<'a> {
    /// 同次 parser 的新笔事实（判据不重算）。
    pub facts: &'a StrokeFacts,
    /// 本次生成态的**有效**原始账（键集即依赖区间与全账本数组的来源）。
    pub raw: &'a BTreeMap<i64, Value>,
    /// 全部原始事件（含历史修订；只读标量，用于事实代际）。
    pub all_events: &'a [Value],
    /// 上一代发布态的内容记录根。
    pub prior: &'a PriorState<'a>,
    /// 本次生成态代号（进记录的首获知/发布元数据）。
    pub generation: i64,
    /// 本次生成态 cut（进记录）。
    pub cut: &'a str,
    /// 本次输入前沿（进载荷与已知时间）。
    pub frontier: i64,
    /// 本次语义提交时钟（进 `known_at`）。
    pub semantic_commit_ns: Option<String>,
}

/// 借用入口与旧入口共用同一业务核；事件与有效账可来自同账 OwnedJson。
pub(super) struct BiBuildInputView<'a> {
    pub facts: &'a StrokeFacts,
    pub raw: BiLedgerView<'a>,
    pub all_events: ArrayView<'a>,
    pub prior: &'a PriorState<'a>,
    pub generation: i64,
    pub cut: &'a str,
    pub frontier: i64,
    pub semantic_commit_ns: Option<&'a str>,
}

/// 一次构造的产物：本代全部内容对象 + 批次 `relations` 投影。
pub(super) struct BiBuild {
    pub objects: Vec<TypedObject>,
    /// 与旧 `edges` 同序同值的 `{subject,relation_type,object}` 投影（有界小对象）。
    pub relations: Vec<CValue>,
}

/// TB-02-B 内容构造器（一个 S3 车道一份；持跨代续接的坐标序列）。
pub(super) struct ContentBi {
    coords: RawCoords,
}

impl ContentBi {
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn visit_runtime_roots(&self, f: &mut dyn FnMut(&'static str, NodeDigest)) {
        if self.coords.coords.is_empty() {
            return;
        }
        f("bi.RawCoords.text", self.coords.text.root());
        f("bi.RawCoords.refs", self.coords.refs.root());
        for root in &self.coords.refs_leaves {
            f("bi.RawCoords.refs_leaves", *root);
        }
    }

    pub(super) fn new() -> Self {
        ContentBi {
            coords: RawCoords::new(),
        }
    }

    /// 整条车道的运行态丢弃：坐标序列与 `Cc004Producer` 的源注册表 + memo **一起**清。
    ///
    /// 事务 rollback / 换库 / 换来源域后**必须**调用（作用域与
    /// [`Cc004Producer::discard_runtime_state`] 相同）：上轮写下的节点与序列根已不在当前域，
    /// 只清 producer 会让 `RawCoords` 继续凭旧索引长度拼切片，只清 `RawCoords` 会让 producer
    /// 的注册表按「同 spec」直接返回、重建出缺闭包的对象。调用后按真实当前 source 重新
    /// `bind_source`，下一次 [`ContentBi::build`] 从空序列整体重建。
    pub(super) fn discard_runtime_state(&mut self, producer: &mut Cc004Producer) {
        self.coords.discard_runtime_state();
        producer.discard_runtime_state();
    }

    /// 从同次证据直接构造七个 kind 的内容对象；不读旧 `Value`、不重算判据。
    pub(super) fn build(
        &mut self,
        producer: &mut Cc004Producer,
        store: &mut dyn NodeStore,
        input: &BiBuildInput<'_>,
    ) -> Result<BiBuild, BiError> {
        let events = PlainEvents(input.all_events);
        self.build_borrowed(
            producer,
            store,
            &BiBuildInputView {
                facts: input.facts,
                raw: BiLedgerView::Plain(input.raw),
                all_events: JsonView::BorrowedArray(&events).as_array().unwrap(),
                prior: input.prior,
                generation: input.generation,
                cut: input.cut,
                frontier: input.frontier,
                semantic_commit_ns: input.semantic_commit_ns.as_deref(),
            },
        )
    }

    /// 无复制借用适配。门序、判据、身份及生命周期与 normal 入口共用此核。
    pub(super) fn build_borrowed(
        &mut self,
        producer: &mut Cc004Producer,
        store: &mut dyn NodeStore,
        input: &BiBuildInputView<'_>,
    ) -> Result<BiBuild, BiError> {
        let ledger = Ledger { rows: input.raw };
        let fact_generation = fact_generation(input.all_events)?;
        self.coords.sync(&mut *store, producer, &ledger)?;
        let known = sc::write_value(
            &mut *store,
            &known_value(
                &ledger,
                input.generation,
                input.frontier,
                input.semantic_commit_ns.as_deref(),
            )?,
        )?;
        let whole = self.coords.whole(&mut *store)?;
        let prior = PriorIndex::read(producer, &mut *store, input.prior)?;
        let generation_text = input.generation.to_string();
        let mut objects: Vec<TypedObject> = Vec::new();
        let mut relations: Vec<CValue> = Vec::new();

        for endpoint in &input.facts.endpoints {
            let slot = Value::String(format!("fx:{fact_generation}:{}", endpoint.group_anchor));
            let deps = self
                .coords
                .coord_set(&mut *store, &endpoint.source_coords)?;
            objects.push(build_typed(
                producer,
                &mut *store,
                ENDPOINT_KIND,
                &slot,
                fact_generation,
                endpoint_value(endpoint),
                &deps,
                input.generation,
                input.cut,
            )?);
        }

        for pair in &input.facts.pairs {
            let kind = if pair.conditions.is_some() {
                NEW_BI_PAIR_KIND
            } else {
                SAME_KIND_KIND
            };
            let old_anchor = pair.old.group_anchor.to_string();
            let new_anchor = pair.new.group_anchor.to_string();
            let slot = slot_of(&[
                fact_generation.to_string().as_str(),
                old_anchor.as_str(),
                new_anchor.as_str(),
            ]);
            let deps = self.coords.interval(
                &mut *store,
                &pair.old.source_coords,
                &pair.new.source_coords,
                &[],
            )?;
            objects.push(build_typed(
                producer,
                &mut *store,
                kind,
                &slot,
                fact_generation,
                pair_value(pair),
                &deps,
                input.generation,
                input.cut,
            )?);
        }

        let mut active: BTreeSet<String> = BTreeSet::new();
        for stroke in &input.facts.strokes {
            let entity = format!("bi:{fact_generation}:{}", stroke.identity_anchor);
            active.insert(entity.clone());
            let previous = prior.bi.get(&entity);
            let carried = previous.filter(|old| old.confirmation.is_some());
            let start = sc::write_value(&mut *store, &endpoint_value(&stroke.start))?;
            let end = sc::write_value(&mut *store, &endpoint_value(&stroke.end))?;
            let formation = sc::write_value(&mut *store, &conditions_value(&stroke.formation))?;
            // 确认：旧对象已有确认时**沿用首次不可逆见证**（值根原样引用），不借新后继
            // 把确认获知时间推后；否则用本次事实的确认。
            let (confirmation, confirmed_known_at, confirmation_coords, confirmation_successor) =
                match carried {
                    Some(old) => {
                        let old_confirmation = old.confirmation.as_ref().expect("过滤后必有旧确认");
                        let old_confirmed =
                            old.confirmed_known_at
                                .as_ref()
                                .ok_or_else(|| BiError::Prior {
                                    what: "confirmed_known_at",
                                    detail: format!("{entity} 沿用旧确认却没有确认获知时间"),
                                })?;
                        (
                            CValue::Edge(old_confirmation.digest()?),
                            CValue::Edge(old_confirmed.digest()?),
                            old.confirmation_source_coords.clone().unwrap_or_default(),
                            old.confirmation_successor_start.clone(),
                        )
                    }
                    None => match stroke.confirmation.as_ref() {
                        Some(new) => {
                            let built = sc::write_value(&mut *store, &confirmation_value(new))?;
                            (
                                CValue::Edge(built.digest),
                                CValue::Edge(known.digest),
                                new.source_coords.iter().copied().collect::<Vec<usize>>(),
                                Some(new.successor_start.to_string()),
                            )
                        }
                        None => (CValue::Null, CValue::Null, Vec::new(), None),
                    },
                };
            let confirmation_is_null = matches!(&confirmation, CValue::Null);
            let mut changes: Vec<&'static str> = Vec::new();
            match previous {
                Some(old) => {
                    if old.end_group_anchor != coord_i64(stroke.end.group_anchor)? {
                        changes.push("extended");
                        changes.push("endpoint_replaced");
                    }
                    if old.confirmation.is_none() && !confirmation_is_null {
                        changes.push("confirmed");
                    }
                    if old.start.digest()? != start.digest || old.end.digest()? != end.digest {
                        changes.push("source_or_boundary_updated");
                    }
                }
                None => {
                    changes.push("formed");
                    if !confirmation_is_null {
                        changes.push("confirmed");
                    }
                }
            }
            let entity_revision = previous.map(|old| old.entity_revision).unwrap_or(0)
                + i64::from(previous.is_none() || !changes.is_empty());
            let version_causes = if changes.is_empty() {
                let old = previous.ok_or_else(|| BiError::Facts {
                    detail: format!("{entity} 无任何变化却缺上一代对象"),
                })?;
                CValue::Edge(old.version_causes.digest()?)
            } else {
                CValue::Array(
                    changes
                        .iter()
                        .map(|change| CValue::String((*change).to_string()))
                        .collect(),
                )
            };
            let formed_known_at = match previous {
                Some(old) => CValue::Edge(old.formed_known_at.digest()?),
                None => CValue::Edge(known.digest),
            };
            let formed_evidence = match previous {
                Some(old) => CValue::Edge(old.formed_evidence.digest()?),
                None => CValue::Edge(formation.digest),
            };
            let data_value = CValue::Object(vec![
                (
                    "identity_anchor".to_string(),
                    CValue::String(stroke.identity_anchor.to_string()),
                ),
                ("start".to_string(), CValue::Edge(start.digest)),
                ("end".to_string(), CValue::Edge(end.digest)),
                ("formation".to_string(), CValue::Edge(formation.digest)),
                ("confirmation".to_string(), confirmation),
                ("entity_id".to_string(), CValue::String(entity.clone())),
                ("formed_known_at".to_string(), formed_known_at),
                ("formed_evidence".to_string(), formed_evidence),
                ("confirmed_known_at".to_string(), confirmed_known_at),
                (
                    "state".to_string(),
                    CValue::String(
                        if confirmation_is_null {
                            "FORMED_UNCONFIRMED"
                        } else {
                            "CONFIRMED"
                        }
                        .to_string(),
                    ),
                ),
                (
                    "entity_revision".to_string(),
                    CValue::String(entity_revision.to_string()),
                ),
                ("version_causes".to_string(), version_causes),
            ]);
            let data = sc::write_value(&mut *store, &data_value)?;
            let deps = self.coords.interval(
                &mut *store,
                &stroke.start.source_coords,
                &stroke.end.source_coords,
                &confirmation_coords,
            )?;
            let slot = Value::String(entity.clone());
            let object = build_typed(
                producer,
                &mut *store,
                BI_KIND,
                &slot,
                fact_generation,
                CValue::Edge(data.digest),
                &deps,
                input.generation,
                input.cut,
            )?;
            let witness = object.object_id.clone();
            objects.push(object);

            // 关系边：起点/终点派生、实际确认的后继确认、成员归属（与旧配方同序同值）。
            let mut edges: Vec<(String, &'static str, String)> = vec![
                (
                    entity.clone(),
                    "derived_from",
                    format!("fx:{fact_generation}:{}", stroke.start.group_anchor),
                ),
                (
                    entity.clone(),
                    "derived_from",
                    format!("fx:{fact_generation}:{}", stroke.end.group_anchor),
                ),
            ];
            if !confirmation_is_null {
                let successor_start =
                    confirmation_successor
                        .as_deref()
                        .ok_or_else(|| BiError::Facts {
                            detail: format!("{entity} 有确认却没有后继起点"),
                        })?;
                edges.push((
                    format!("bi:{fact_generation}:{successor_start}"),
                    "confirms",
                    entity.clone(),
                ));
            }
            let start_root = first_extreme_root(&stroke.start)?;
            let end_root = first_extreme_root(&stroke.end)?;
            for (coord, row) in input.raw.range(start_root, end_root) {
                let identity =
                    row.get("identity_key")
                        .and_then(JsonView::as_str)
                        .ok_or(BiError::Ledger {
                            coord,
                            what: "identity_key",
                        })?;
                let revision =
                    row.get("revision")
                        .and_then(JsonView::as_i64)
                        .ok_or(BiError::Ledger {
                            coord,
                            what: "revision",
                        })?;
                edges.push((
                    format!("raw:{identity}@{revision}"),
                    "member_of",
                    entity.clone(),
                ));
            }
            for (source_id, relation_kind, target_id) in edges {
                let slot = slot_of(&[source_id.as_str(), relation_kind, target_id.as_str()]);
                let data_value = CValue::Object(vec![
                    ("source_id".to_string(), CValue::String(source_id.clone())),
                    (
                        "relation_kind".to_string(),
                        CValue::String(relation_kind.to_string()),
                    ),
                    ("target_id".to_string(), CValue::String(target_id.clone())),
                    (
                        "version".to_string(),
                        CValue::String(entity_revision.to_string()),
                    ),
                    (
                        "witness_object_id".to_string(),
                        CValue::String(witness.clone()),
                    ),
                ]);
                objects.push(build_typed(
                    producer,
                    &mut *store,
                    RELATION_KIND,
                    &slot,
                    fact_generation,
                    data_value,
                    &deps,
                    input.generation,
                    input.cut,
                )?);
                relations.push(CValue::Object(vec![
                    ("subject".to_string(), CValue::String(source_id)),
                    (
                        "relation_type".to_string(),
                        CValue::String(relation_kind.to_string()),
                    ),
                    ("object".to_string(), CValue::String(target_id)),
                ]));
            }

            // 就地变更事件：依赖面是**全账本**（与旧配方 `sources` 同一条）；`before`/`after`
            // 直接引用上一代与新写出的数据根。
            let event_at = ledger.text(end_root, "ts")?.to_string();
            for change in &changes {
                let slot = slot_of(&[generation_text.as_str(), entity.as_str(), *change]);
                let before = match previous {
                    Some(old) => CValue::Edge(old.data.digest()?),
                    None => CValue::Null,
                };
                let data_value = CValue::Object(vec![
                    ("entity_id".to_string(), CValue::String(entity.clone())),
                    ("change".to_string(), CValue::String((*change).to_string())),
                    ("before".to_string(), before),
                    ("after".to_string(), CValue::Edge(data.digest)),
                    ("known_at".to_string(), CValue::Edge(known.digest)),
                    ("event_at".to_string(), CValue::String(event_at.clone())),
                    (
                        "causes".to_string(),
                        CValue::Array(vec![CValue::String(CHANGE_CAUSE.to_string())]),
                    ),
                    ("object_id".to_string(), CValue::String(witness.clone())),
                ]);
                objects.push(build_typed(
                    producer,
                    &mut *store,
                    CHANGE_KIND,
                    &slot,
                    fact_generation,
                    data_value,
                    &whole,
                    input.generation,
                    input.cut,
                )?);
            }
        }

        // 撤回：上一代有、本代没有的实体（顺序 = 实体 id 升序，与旧 `BTreeMap` 同序）。
        let latest = ledger.latest();
        for (entity, old) in &prior.bi {
            if active.contains(entity) {
                continue;
            }
            let causes = if fact_generation.to_string() != entity.split(':').nth(1).unwrap_or("") {
                vec!["raw_fact_revision"]
            } else {
                input.facts.waiting_reasons.clone()
            };
            let event_at = match latest {
                Some(row) => CValue::String(
                    row.get("ts")
                        .and_then(JsonView::as_str)
                        .ok_or_else(|| BiError::Facts {
                            detail: "原始账最新行缺 ts".to_string(),
                        })?
                        .to_string(),
                ),
                // 旧配方空账时 `latest` 是 `Value::Null`，`latest["ts"]` 因此也是 null：
                // 保留**精确 null**，不造空串默认。
                None => CValue::Null,
            };
            let slot = slot_of(&[generation_text.as_str(), entity.as_str(), "withdrawn"]);
            let data_value = CValue::Object(vec![
                ("entity_id".to_string(), CValue::String(entity.clone())),
                (
                    "change".to_string(),
                    CValue::String("withdrawn".to_string()),
                ),
                ("before".to_string(), CValue::Edge(old.data.digest()?)),
                ("after".to_string(), CValue::Null),
                ("known_at".to_string(), CValue::Edge(known.digest)),
                ("event_at".to_string(), event_at),
                (
                    "causes".to_string(),
                    CValue::Array(
                        causes
                            .iter()
                            .map(|cause| CValue::String((*cause).to_string()))
                            .collect(),
                    ),
                ),
                ("object_id".to_string(), CValue::Null),
            ]);
            objects.push(build_typed(
                producer,
                &mut *store,
                CHANGE_KIND,
                &slot,
                fact_generation,
                data_value,
                &whole,
                input.generation,
                input.cut,
            )?);
        }

        // 版本对象：本代等待原因与前沿；`known_at` 与其余对象同根。
        let version_causes: Vec<&'static str> = if fact_generation > prior.generation {
            vec!["raw_fact_revision"]
        } else {
            vec!["append"]
        };
        let slot = Value::String(VERSION_SLOT.to_string());
        let data_value = CValue::Object(vec![
            (
                "input_frontier".to_string(),
                CValue::String(input.frontier.to_string()),
            ),
            (
                "version_causes".to_string(),
                CValue::Array(
                    version_causes
                        .iter()
                        .map(|cause| CValue::String((*cause).to_string()))
                        .collect(),
                ),
            ),
            (
                "identity_basis".to_string(),
                CValue::String("generation_and_start_fractal_anchor".to_string()),
            ),
            (
                "classification_obligation".to_string(),
                CValue::String("complete_applicable_vector".to_string()),
            ),
            (
                "waiting_reasons".to_string(),
                str_array(&input.facts.waiting_reasons),
            ),
            ("known_at".to_string(), CValue::Edge(known.digest)),
        ]);
        objects.push(build_typed(
            producer,
            &mut *store,
            VERSION_KIND,
            &slot,
            fact_generation,
            data_value,
            &whole,
            input.generation,
            input.cut,
        )?);

        Ok(BiBuild { objects, relations })
    }
}

/// 一个泛型六字段 `obj3` 对象的组装口（身份与记录配方全部复用 `Cc004Producer::build_object`）。
#[allow(clippy::too_many_arguments)]
fn build_typed(
    producer: &Cc004Producer,
    store: &mut dyn NodeStore,
    kind: &str,
    slot: &Value,
    fact_generation: i64,
    data: CValue,
    deps: &DepsArrays,
    generation: i64,
    cut: &str,
) -> Result<TypedObject, BiError> {
    let payload = payload_value(slot, fact_generation, data)?;
    let fact_key = canonical_json(&Value::Array(vec![
        Value::String(kind.to_string()),
        slot.clone(),
    ]));
    Ok(producer.build_object(
        store,
        kind,
        fact_key,
        payload,
        deps.input_refs.clone(),
        deps.source_coords.clone(),
        generation,
        cut,
    )?)
}

/// 载荷头 + 槽位 + 数据（与旧 `tb02b::payload` 逐字段同值；数字均已按旧 `wire` 成文本）。
fn payload_value(slot: &Value, fact_generation: i64, data: CValue) -> Result<CValue, BiError> {
    Ok(CValue::Object(vec![
        (
            "schema_revision".to_string(),
            CValue::String(PAYLOAD_SCHEMA.to_string()),
        ),
        (
            "semantic_version".to_string(),
            CValue::String(RULE.to_string()),
        ),
        (
            "policy_version".to_string(),
            CValue::String(PAYLOAD_POLICY.to_string()),
        ),
        (
            "view_role".to_string(),
            CValue::String(PAYLOAD_VIEW_ROLE.to_string()),
        ),
        (
            "object_generation".to_string(),
            CValue::String(fact_generation.to_string()),
        ),
        ("slot".to_string(), slot_cvalue(slot)?),
        ("data".to_string(), data),
    ]))
}

/// 槽位的 `serde_json::Value → CValue` 有界转换（槽位只允许字符串或字符串数组；
/// 事实键仍由同一 `canonical_json` 给出，不另写一套渲染）。
fn slot_cvalue(slot: &Value) -> Result<CValue, BiError> {
    match slot {
        Value::String(text) => Ok(CValue::String(text.clone())),
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::String(text) => out.push(CValue::String(text.clone())),
                    other => {
                        return Err(BiError::Facts {
                            detail: format!("槽位数组出现非字符串元素：{other}"),
                        })
                    }
                }
            }
            Ok(CValue::Array(out))
        }
        other => Err(BiError::Facts {
            detail: format!("槽位形状不受支持：{other}"),
        }),
    }
}

/// 字符串数组槽位（全字符串，顺序保持）。
fn slot_of(parts: &[&str]) -> Value {
    Value::Array(
        parts
            .iter()
            .map(|part| Value::String((*part).to_string()))
            .collect(),
    )
}

/// 已知时间对象（`known_at`；有界标量小对象，同一代共用同一个根）。
fn known_value(
    ledger: &Ledger<'_>,
    generation: i64,
    frontier: i64,
    semantic_commit_ns: Option<&str>,
) -> Result<CValue, BiError> {
    let latest = ledger.latest();
    let receipt = latest
        .and_then(|row| row.get("receipt_id").and_then(JsonView::as_str))
        .map(|text| CValue::String(text.to_string()))
        .unwrap_or(CValue::Null);
    let received_at = latest
        .and_then(|row| row.get("received_at").and_then(JsonView::as_str))
        .map(|text| CValue::String(text.to_string()))
        .unwrap_or(CValue::Null);
    let commit = match semantic_commit_ns {
        Some(text) => CValue::String(text.to_string()),
        None => CValue::Null,
    };
    Ok(CValue::Object(vec![
        (
            "generation".to_string(),
            CValue::String(generation.to_string()),
        ),
        (
            "input_frontier".to_string(),
            CValue::String(frontier.to_string()),
        ),
        ("receipt_id".to_string(), receipt),
        ("received_at".to_string(), received_at),
        ("semantic_commit_ns".to_string(), commit),
    ]))
}

/// 端点值（旧 `wire(serde_json::to_value(Endpoint))` 的同一条文本编码）。
fn endpoint_value(endpoint: &Endpoint) -> CValue {
    CValue::Object(vec![
        (
            "kind".to_string(),
            CValue::String(endpoint.kind.to_string()),
        ),
        (
            "merged_index".to_string(),
            CValue::String(endpoint.merged_index.to_string()),
        ),
        (
            "group_anchor".to_string(),
            CValue::String(endpoint.group_anchor.to_string()),
        ),
        (
            "price".to_string(),
            CValue::String(endpoint.price.to_string()),
        ),
        (
            "extreme_roots".to_string(),
            usize_array(&endpoint.extreme_roots),
        ),
        (
            "raw_position".to_string(),
            optional_usize(endpoint.raw_position),
        ),
        (
            "source_coords".to_string(),
            usize_array(&endpoint.source_coords),
        ),
        ("sealed_at".to_string(), optional_usize(endpoint.sealed_at)),
        (
            "waiting_reasons".to_string(),
            str_array(&endpoint.waiting_reasons),
        ),
    ])
}

/// 条件值（旧 `Conditions` 的文本编码；`vector` 保留 bool/null 差别）。
fn conditions_value(conditions: &Conditions) -> CValue {
    CValue::Object(vec![
        (
            "merged_gap".to_string(),
            CValue::String(conditions.merged_gap.to_string()),
        ),
        (
            "raw_between_actual_extrema".to_string(),
            optional_usize(conditions.raw_between_actual_extrema),
        ),
        (
            "top_price".to_string(),
            CValue::String(conditions.top_price.to_string()),
        ),
        (
            "bottom_price".to_string(),
            CValue::String(conditions.bottom_price.to_string()),
        ),
        (
            "vector".to_string(),
            CValue::Array(
                conditions
                    .vector
                    .iter()
                    .map(|value| match value {
                        Some(flag) => CValue::Bool(*flag),
                        None => CValue::Null,
                    })
                    .collect(),
            ),
        ),
        (
            "failed_conditions".to_string(),
            str_array(&conditions.failed_conditions),
        ),
        (
            "waiting_reasons".to_string(),
            str_array(&conditions.waiting_reasons),
        ),
    ])
}

/// 确认值（旧 `Confirmation` 的文本编码）。
fn confirmation_value(confirmation: &Confirmation) -> CValue {
    CValue::Object(vec![
        (
            "successor_start".to_string(),
            CValue::String(confirmation.successor_start.to_string()),
        ),
        (
            "successor_end".to_string(),
            CValue::String(confirmation.successor_end.to_string()),
        ),
        (
            "successor_conditions".to_string(),
            conditions_value(&confirmation.successor_conditions),
        ),
        (
            "right_group_sealed_at".to_string(),
            CValue::String(confirmation.right_group_sealed_at.to_string()),
        ),
        (
            "source_coords".to_string(),
            usize_array(&confirmation.source_coords),
        ),
    ])
}

/// 端点对值（旧 `PairFact` 的文本编码）。
fn pair_value(pair: &PairFact) -> CValue {
    CValue::Object(vec![
        ("old".to_string(), endpoint_value(&pair.old)),
        ("new".to_string(), endpoint_value(&pair.new)),
        (
            "endpoint_kind_pair".to_string(),
            CValue::String(pair.endpoint_kind_pair.clone()),
        ),
        (
            "conditions".to_string(),
            match &pair.conditions {
                Some(conditions) => conditions_value(conditions),
                None => CValue::Null,
            },
        ),
        ("comparison".to_string(), optional_str(pair.comparison)),
        ("selection".to_string(), optional_str(pair.selection)),
        (
            "retained_anchors".to_string(),
            usize_array(&pair.retained_anchors),
        ),
        (
            "waiting_reasons".to_string(),
            str_array(&pair.waiting_reasons),
        ),
    ])
}

fn usize_array(values: &[usize]) -> CValue {
    CValue::Array(
        values
            .iter()
            .map(|value| CValue::String(value.to_string()))
            .collect(),
    )
}

fn str_array(values: &[&'static str]) -> CValue {
    CValue::Array(
        values
            .iter()
            .map(|value| CValue::String((*value).to_string()))
            .collect(),
    )
}

fn optional_usize(value: Option<usize>) -> CValue {
    match value {
        Some(value) => CValue::String(value.to_string()),
        None => CValue::Null,
    }
}

fn optional_str(value: Option<&'static str>) -> CValue {
    match value {
        Some(value) => CValue::String(value.to_string()),
        None => CValue::Null,
    }
}

/// 端点极值根首项（关系成员区间的端点；缺项即拒绝，不静默取空区间）。
fn first_extreme_root(endpoint: &Endpoint) -> Result<i64, BiError> {
    endpoint
        .extreme_roots
        .first()
        .map(|root| coord_i64(*root))
        .transpose()?
        .ok_or_else(|| BiError::Facts {
            detail: format!(
                "端点 {} 没有极值根，无法定位关系成员区间",
                endpoint.group_anchor
            ),
        })
}

/// 路径读点：单步 `{field: name}`。
fn field_path(name: &str) -> Vec<PathStep> {
    vec![PathStep::Field(name.to_string())]
}

/// 路径读点：多级 `{field: …}`。
fn nested_path(names: &[&str]) -> Vec<PathStep> {
    names
        .iter()
        .map(|name| PathStep::Field((*name).to_string()))
        .collect()
}

/// 按路径定位（只核路径触碰到的节点与序列形状，见 `s_content::path`）。
fn resolve(
    store: &mut dyn NodeStore,
    reference: &ContentRef,
    path: &[PathStep],
) -> Result<ContentRef, BiError> {
    Ok(lookup_path(&mut *store, reference, path)?)
}

/// 单个**叶**值的规范渲染（字符串 / 数字叶没有子树，闭包即该节点；容器不许走这里）。
fn render_json(store: &mut dyn NodeStore, reference: &ContentRef) -> Result<Value, BiError> {
    let mut out: Vec<u8> = Vec::new();
    sc::render_canonical(&mut *store, reference, &mut out)?;
    serde_json::from_slice(&out).map_err(|error| BiError::Prior {
        what: "规范渲染",
        detail: error.to_string(),
    })
}

/// 标量文本解码：按路径定位到**字符串叶**后只渲染该叶，不做整树展开，也不设发明上限。
///
/// 长度由 `lookup_path` 从真实节点读出的 extent 给出（字符串 extent 即叶字节数），
/// 即「读多少」由内容自身决定；增长型文本不在这里复制成通用容器，留在其内容根上。
fn scalar_text(
    store: &mut dyn NodeStore,
    reference: &ContentRef,
    path: &[PathStep],
) -> Result<String, BiError> {
    let target = resolve(store, reference, path)?;
    if target.kind != Kind::String {
        return Err(BiError::Prior {
            what: "标量文本",
            detail: format!("按路径读到的不是字符串（{:?}）", target.kind),
        });
    }
    match render_json(store, &target)? {
        Value::String(text) => Ok(text),
        other => Err(BiError::Prior {
            what: "标量文本",
            detail: format!("规范渲染结果不是字符串：{other}"),
        }),
    }
}

fn scalar_i64(
    store: &mut dyn NodeStore,
    reference: &ContentRef,
    path: &[PathStep],
) -> Result<i64, BiError> {
    let text = scalar_text(store, reference, path)?;
    parse_canonical_i64(&text, "上一代内容标量").map_err(|detail| BiError::Prior {
        what: "规范十进制标量",
        detail,
    })
}

/// `null` = 缺省（`None`）；其余必须是对象（确认、确认获知时间），不按字段名猜。
fn optional_object(
    store: &mut dyn NodeStore,
    reference: &ContentRef,
    path: &[PathStep],
) -> Result<Option<ContentRef>, BiError> {
    let target = resolve(store, reference, path)?;
    if target.kind == Kind::Null {
        return Ok(None);
    }
    if target.kind != Kind::Object {
        return Err(BiError::Prior {
            what: "可选对象",
            detail: format!("既不是 null 也不是对象（{:?}）", target.kind),
        });
    }
    Ok(Some(target))
}

/// 坐标文本数组（上一代确认的来源坐标）：只核数组类型，元素数取节点自身的 extent，
/// 然后**按下标逐叶**定位 + 规范十进制解析。不整块渲染、不预分配、不设默认项数上限。
fn coord_array(
    store: &mut dyn NodeStore,
    reference: &ContentRef,
    path: &[PathStep],
) -> Result<Vec<usize>, BiError> {
    let target = resolve(store, reference, path)?;
    if target.kind != Kind::Array {
        return Err(BiError::Prior {
            what: "坐标数组",
            detail: format!("按路径读到的不是数组（{:?}）", target.kind),
        });
    }
    let count = target.extent.ok_or_else(|| BiError::Prior {
        what: "坐标数组",
        detail: "数组节点缺元素数声明".to_string(),
    })?;
    let mut out: Vec<usize> = Vec::new();
    for index in 0..count {
        let mut item_path = path.to_vec();
        item_path.push(PathStep::Index(index));
        let item = resolve(store, reference, &item_path)?;
        if item.kind != Kind::String {
            return Err(BiError::Prior {
                what: "坐标数组",
                detail: format!("第 {index} 项不是字符串（{:?}）", item.kind),
            });
        }
        let text = match render_json(store, &item)? {
            Value::String(text) => text,
            other => {
                return Err(BiError::Prior {
                    what: "坐标数组",
                    detail: format!("第 {index} 项规范渲染结果不是文本：{other}"),
                })
            }
        };
        let coord = parse_canonical_i64(&text, "上一代坐标").map_err(|detail| BiError::Prior {
            what: "坐标数组",
            detail,
        })?;
        out.push(usize::try_from(coord).map_err(|_| BiError::Prior {
            what: "坐标数组",
            detail: format!("坐标 {coord} 不是非负源位置"),
        })?);
    }
    Ok(out)
}

// ────────────────────────────────────────────────────────────────────────────
// 测试面：真实 parser 事实 + 冻结 oracle（旧 `tb02b::build` **只在这里**当判据）。
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::content_producer::RawRefSpec;
    use super::*;
    use newchan_rust::s_content::{MemoryNodes, NodeRead, NodeSink, NodeSource};
    use newchan_rust::theta_v0::config::ThetaConfig;
    use newchan_rust::theta_v0::parser;
    use newchan_rust::theta_v0::types::Bar;

    const GENERATION: i64 = 3;
    const CUT: &str = "cut-3";
    const FRONTIER: i64 = 130;
    const COMMIT: &str = "2026-09-18T00:00:00Z";

    /// 计数节点表：写/读次数是「是否重复物化」的测量面（不参与任何判据）。
    #[derive(Default)]
    struct CountingStore {
        nodes: MemoryNodes,
        puts: usize,
        /// `Some(n)`：全局第 `n` 次之后的写一律拒绝（只用于拒写 / 回滚恢复测试的注入面）。
        refuse_put_after: Option<usize>,
    }

    impl NodeSink for CountingStore {
        fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
            self.puts += 1;
            if let Some(limit) = self.refuse_put_after {
                if self.puts > limit {
                    return Err(CodecError::new("test_write_refused", "注入拒写"));
                }
            }
            self.nodes.put(digest, tag, body)
        }
    }

    impl NodeSource for CountingStore {
        fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
            self.nodes.read(digest)
        }
    }

    /// 一条有效原始行（同代同坐标只有一条；修订行以更大的 `seq` 后到）。
    #[derive(Clone, Debug)]
    struct Row {
        coord: i64,
        high: i64,
        low: i64,
        revision: i64,
        seq: i64,
    }

    /// 与旧 `tb02b_oracle_rows` 同源的 14 根 OHLC（**稀疏坐标** ×10，O=C=floor((H+L)/2)）。
    fn fixture_rows() -> Vec<Row> {
        let raw: [(i64, i64, i64); 14] = [
            (0, 10, 7),
            (10, 8, 5),
            (20, 12, 8),
            (30, 11, 9),
            (40, 14, 11),
            (50, 17, 14),
            (60, 15, 12),
            (70, 19, 16),
            (80, 16, 13),
            (90, 14, 10),
            (100, 12, 8),
            (110, 10, 6),
            (120, 13, 9),
            (130, 15, 11),
        ];
        raw.iter()
            .enumerate()
            .map(|(index, &(coord, high, low))| Row {
                coord,
                high,
                low,
                revision: 1,
                seq: index as i64,
            })
            .collect()
    }

    fn row_value(row: &Row) -> Value {
        json!({
            "identity_key": format!("k-{}", row.coord),
            "revision": row.revision,
            "input_revision": row.revision.to_string(),
            "payload_hash": format!("ph-{}-{}", row.coord, row.revision),
            "receipt_id": format!("rc-{}", row.coord),
            "seq": row.seq,
            "received_at": "2026-09-18T00:00:00Z",
            "raw_text": format!("{}/{}", row.low, row.high),
            "ts": row.coord.to_string(),
            "source_coord": row.coord.to_string(),
            "event_id": format!("ev-{}-{}", row.coord, row.revision),
        })
    }

    fn bar_of(row: &Row) -> Bar {
        let mid = (row.high + row.low) / 2;
        Bar {
            source_index: row.coord as usize,
            timestamp: row.coord,
            open: mid,
            high: row.high,
            low: row.low,
            close: mid,
            volume: 1.0,
            untradable: false,
        }
    }

    fn spec_of(row: &Value) -> RawRefSpec {
        RawRefSpec {
            identity_key: row["identity_key"]
                .as_str()
                .expect("identity_key")
                .to_string(),
            payload_hash: row["payload_hash"]
                .as_str()
                .expect("payload_hash")
                .to_string(),
            receipt_id: row["receipt_id"].as_str().expect("receipt_id").to_string(),
            event_id: row["event_id"].as_str().expect("event_id").to_string(),
            input_revision: row["input_revision"]
                .as_str()
                .expect("input_revision")
                .to_string(),
            revision: row["revision"].as_i64().expect("revision"),
            seq: row["seq"].as_i64().expect("seq"),
            source_coord: row["source_coord"]
                .as_str()
                .expect("source_coord")
                .to_string(),
        }
    }

    /// 一代输入：parser 用升序 bar，账本按坐标取当前有效行，`all_events` 按**到达序**。
    fn ledger(rows: &[Row]) -> (Vec<Bar>, BTreeMap<i64, Value>, Vec<Value>) {
        let mut bars: Vec<Bar> = rows.iter().map(bar_of).collect();
        bars.sort_by_key(|bar| bar.source_index);
        let mut raw: BTreeMap<i64, Value> = BTreeMap::new();
        for row in rows {
            assert!(
                raw.insert(row.coord, row_value(row)).is_none(),
                "同代同坐标只能有一条有效行"
            );
        }
        let all_events: Vec<Value> = rows.iter().map(row_value).collect();
        (bars, raw, all_events)
    }

    fn facts_of(bars: &[Bar]) -> StrokeFacts {
        let (_, facts) = parser::parse_layer_with_inclusion_facts(bars, &ThetaConfig::default());
        facts.stroke_facts.expect("完整解析附带新笔事实")
    }

    fn object_kind(object: &TypedObject) -> String {
        let key: Value = serde_json::from_str(&object.fact_key).expect("事实键是规范 JSON");
        key[0].as_str().expect("事实键首项是 kind").to_string()
    }

    fn render_built(store: &mut dyn NodeStore, built: &Built) -> String {
        let mut out: Vec<u8> = Vec::new();
        sc::render_canonical(&mut *store, &built.content_ref(), &mut out).expect("闭包可核");
        String::from_utf8(out).expect("规范渲染是 UTF-8")
    }

    fn render_value(value: &CValue) -> Value {
        let mut scratch = MemoryNodes::default();
        let built = sc::write_value(&mut scratch, value).expect("写值");
        let mut out: Vec<u8> = Vec::new();
        sc::render_canonical(&mut scratch, &built.content_ref(), &mut out).expect("渲染");
        serde_json::from_slice(&out).expect("合法 JSON")
    }

    /// 旧对象 id → 新 `obj3` id 的身份映射（按 kind + 事实键一一对应）。
    fn old_to_new_ids(build: &BiBuild, oracle: &[Value], label: &str) -> BTreeMap<String, String> {
        let mut new_by_key: BTreeMap<String, String> = BTreeMap::new();
        for object in &build.objects {
            let key = format!("{}\u{1}{}", object_kind(object), object.fact_key);
            assert!(
                new_by_key
                    .insert(key.clone(), object.object_id.clone())
                    .is_none(),
                "{label} 内容对象事实键重复：{key}"
            );
        }
        let mut mapping: BTreeMap<String, String> = BTreeMap::new();
        for want in oracle {
            let kind = want["kind"].as_str().expect("kind");
            if kind != BI_KIND {
                continue;
            }
            let key = format!(
                "{kind}\u{1}{}",
                want["fact_key"].as_str().expect("fact_key")
            );
            let new_id = new_by_key
                .get(&key)
                .unwrap_or_else(|| panic!("{label} oracle 身份没有对应内容对象"));
            mapping.insert(
                want["object_id"].as_str().expect("object_id").to_string(),
                new_id.clone(),
            );
        }
        mapping
    }

    /// 新旧**全部逻辑字段**对拍：三条公开边的规范渲染字节、载荷（只规范 obj3 身份及其内嵌
    /// 关联：`witness_object_id` / `change_event.object_id`）、记录（旧十项 + 三条内容边）。
    fn assert_parity(
        store: &mut dyn NodeStore,
        build: &BiBuild,
        oracle: &[Value],
        oracle_relations: &[Value],
        label: &str,
    ) {
        assert_eq!(build.objects.len(), oracle.len(), "{label} 对象数");
        let mapping = old_to_new_ids(build, oracle, label);
        for (index, want) in oracle.iter().enumerate() {
            let object = &build.objects[index];
            let kind = want["kind"].as_str().expect("kind");
            let fact_key = want["fact_key"].as_str().expect("fact_key");
            assert_eq!(object.fact_key, fact_key, "{label} #{index} 事实键");
            assert_eq!(object_kind(object), kind, "{label} #{index} kind");
            // 源引用 / 坐标两条公开文本视图不含对象身份：逐字节对拍。
            for (built, field) in [
                (&object.input_refs, "input_refs_json"),
                (&object.source_coords, "source_coords_json"),
            ] {
                assert_eq!(
                    render_built(&mut *store, built),
                    want[field].as_str().expect("oracle 文本"),
                    "{label} #{index} {kind} {field}"
                );
            }
            // 载荷文本视图内嵌 `witness_object_id` / `change_event.object_id`（obj- → obj3-），
            // 故按值对拍：**只**规范这两处 obj3 身份及其内嵌关联，其余字段逐字段对拍。
            let mut expected: Value =
                serde_json::from_str(want["payload_json"].as_str().expect("payload_json"))
                    .expect("oracle 载荷是合法 JSON");
            let mut normalized_identity = false;
            match kind {
                RELATION_KIND => {
                    let witness = expected["data"]["witness_object_id"]
                        .as_str()
                        .expect("关系见证是文本");
                    let mapped = mapping
                        .get(witness)
                        .unwrap_or_else(|| panic!("{label} 关系见证没有对应 bi 对象"));
                    expected["data"]["witness_object_id"] = Value::String(mapped.clone());
                    normalized_identity = true;
                }
                CHANGE_KIND => {
                    if let Some(changed) = expected["data"]["object_id"].as_str() {
                        let mapped = mapping
                            .get(changed)
                            .unwrap_or_else(|| panic!("{label} 变更对象没有对应 bi 对象"));
                        expected["data"]["object_id"] = Value::String(mapped.clone());
                        normalized_identity = true;
                    }
                }
                _ => {}
            }
            if !normalized_identity {
                // 载荷里没有 obj3 内嵌关联时保留逐字节对拍（不放宽）。
                assert_eq!(
                    render_built(&mut *store, &object.payload),
                    want["payload_json"].as_str().expect("oracle 文本"),
                    "{label} #{index} {kind} payload_json"
                );
            }
            let rendered: Value = serde_json::from_str(&render_built(&mut *store, &object.payload))
                .expect("新载荷是合法 JSON");
            assert_eq!(rendered, expected, "{label} #{index} {kind} 载荷");

            let mut record_expected = want.clone();
            record_expected["object_id"] = Value::String(object.object_id.clone());
            // 记录里的载荷边与其文本视图同样内嵌 obj3 关联：文本视图＝已规范载荷的规范 JSON。
            record_expected["payload"] = expected.clone();
            record_expected["payload_json"] = Value::String(canonical_json(&expected));
            for (edge, field) in [
                ("input_refs", "input_refs_json"),
                ("source_coords", "source_coords_json"),
            ] {
                record_expected[edge] =
                    serde_json::from_str(want[field].as_str().expect("oracle 文本"))
                        .expect("oracle 文本是合法 JSON");
            }
            let rendered_record: Value =
                serde_json::from_str(&render_built(&mut *store, &object.record))
                    .expect("记录是合法 JSON");
            assert_eq!(
                rendered_record, record_expected,
                "{label} #{index} {kind} 记录"
            );
        }
        assert_eq!(
            build.relations.len(),
            oracle_relations.len(),
            "{label} 关系数"
        );
        for (index, (relation, want)) in build.relations.iter().zip(oracle_relations).enumerate() {
            assert_eq!(
                render_value(relation),
                want.clone(),
                "{label} 关系 #{index}"
            );
        }
    }

    /// oracle 侧覆盖统计（只读旧对象，用于断言场景真的覆盖了要求的分支）。
    #[derive(Default)]
    struct Summary {
        kinds: BTreeSet<String>,
        changes: BTreeMap<String, usize>,
    }

    impl Summary {
        fn observe(&mut self, oracle: &[Value]) {
            for object in oracle {
                let kind = object["kind"].as_str().expect("kind").to_string();
                self.kinds.insert(kind.clone());
                if kind == CHANGE_KIND {
                    let payload: Value = serde_json::from_str(
                        object["payload_json"].as_str().expect("payload_json"),
                    )
                    .expect("合法 JSON");
                    let change = payload["data"]["change"]
                        .as_str()
                        .expect("change")
                        .to_string();
                    *self.changes.entry(change).or_insert(0) += 1;
                }
            }
        }

        fn count(&self, change: &str) -> usize {
            self.changes.get(change).copied().unwrap_or(0)
        }
    }

    struct Harness {
        store: CountingStore,
        producer: Cc004Producer,
        bi: ContentBi,
    }

    impl Harness {
        fn new() -> Self {
            Harness {
                store: CountingStore::default(),
                producer: Cc004Producer::new(tb02a::PROFILE, RULE_REVISION),
                bi: ContentBi::new(),
            }
        }

        fn bind(&mut self, raw: &BTreeMap<i64, Value>) {
            for (coord, row) in raw {
                self.producer
                    .bind_source(&mut self.store, *coord, spec_of(row))
                    .expect("登记源版本");
            }
        }
    }

    /// 一代构造结果（旧 oracle + 新内容对象），供下一代当 `previous` / `prior`。
    #[derive(Debug)]
    struct Generation {
        objects: Vec<TypedObject>,
        relations: Vec<CValue>,
        oracle: Vec<Value>,
    }

    impl Generation {
        fn bi_records(&self) -> Vec<&Built> {
            self.objects
                .iter()
                .filter(|object| object_kind(object) == BI_KIND)
                .map(|object| &object.record)
                .collect()
        }

        fn version_record(&self) -> Option<&Built> {
            self.objects
                .iter()
                .find(|object| object_kind(object) == VERSION_KIND)
                .map(|object| &object.record)
        }
    }

    fn run_generation(
        harness: &mut Harness,
        rows: &[Row],
        previous: Option<&Generation>,
        label: &str,
    ) -> Generation {
        let (bars, raw, all_events) = ledger(rows);
        let facts = facts_of(&bars);
        harness.bind(&raw);
        let prior_records = previous.map(Generation::bi_records).unwrap_or_default();
        let version_record = previous.and_then(Generation::version_record);
        let prior = PriorState {
            bi_records: prior_records,
            version_record,
        };
        let previous_oracle: Vec<Value> =
            previous.map(|old| old.oracle.clone()).unwrap_or_default();
        let (oracle, oracle_relations) = tb02b::build(
            &facts,
            &raw,
            &all_events,
            &previous_oracle,
            GENERATION,
            CUT,
            FRONTIER,
            Some(COMMIT.to_string()),
        );
        let build = harness
            .bi
            .build(
                &mut harness.producer,
                &mut harness.store,
                &BiBuildInput {
                    facts: &facts,
                    raw: &raw,
                    all_events: &all_events,
                    prior: &prior,
                    generation: GENERATION,
                    cut: CUT,
                    frontier: FRONTIER,
                    semantic_commit_ns: Some(COMMIT.to_string()),
                },
            )
            .expect("内容构造");
        assert_parity(
            &mut harness.store,
            &build,
            &oracle,
            &oracle_relations,
            label,
        );
        Generation {
            objects: build.objects,
            relations: build.relations,
            oracle,
        }
    }

    /// 在给定车道上尝试构造一代（不跑 oracle，只回内容对象）：先绑源，再 `arm` 注入拒写，
    /// 最后 `build`——失败原样上抛，供拒写 / 回滚恢复测试用。
    fn attempt_generation(
        harness: &mut Harness,
        rows: &[Row],
        previous: Option<&Generation>,
        arm: impl FnOnce(&mut CountingStore),
    ) -> Result<Generation, BiError> {
        let (bars, raw, all_events) = ledger(rows);
        let facts = facts_of(&bars);
        harness.bind(&raw);
        let prior_records = previous.map(Generation::bi_records).unwrap_or_default();
        let version_record = previous.and_then(Generation::version_record);
        let prior = PriorState {
            bi_records: prior_records,
            version_record,
        };
        let input = BiBuildInput {
            facts: &facts,
            raw: &raw,
            all_events: &all_events,
            prior: &prior,
            generation: GENERATION,
            cut: CUT,
            frontier: FRONTIER,
            semantic_commit_ns: Some(COMMIT.to_string()),
        };
        arm(&mut harness.store);
        let build = harness
            .bi
            .build(&mut harness.producer, &mut harness.store, &input)?;
        Ok(Generation {
            objects: build.objects,
            relations: build.relations,
            oracle: Vec::new(),
        })
    }

    fn assert_same_roots(left: &Generation, right: &Generation, label: &str) {
        assert_eq!(left.objects.len(), right.objects.len(), "{label} 对象数");
        for (index, (a, b)) in left.objects.iter().zip(&right.objects).enumerate() {
            assert_eq!(a.object_id, b.object_id, "{label} #{index} object_id");
            assert_eq!(
                a.payload.digest, b.payload.digest,
                "{label} #{index} payload"
            );
            assert_eq!(
                a.input_refs.digest, b.input_refs.digest,
                "{label} #{index} input_refs"
            );
            assert_eq!(
                a.source_coords.digest, b.source_coords.digest,
                "{label} #{index} source_coords"
            );
            assert_eq!(a.record.digest, b.record.digest, "{label} #{index} record");
        }
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn borrowed_owned_matches_normal_lifecycle_and_exact_refs() {
        use newchan_rust::s_content::capture_allocation::{audit_json::OwnedJson, OwnedBudget};
        let rows = fixture_rows();
        let mut revised = rows[..9].to_vec();
        revised[2].revision = 2;
        revised[2].seq = 9;
        let mut inserted = rows[..9].to_vec();
        inserted.push(Row {
            coord: 65,
            high: 18,
            low: 15,
            revision: 1,
            seq: 10,
        });
        let cases = [
            rows[..6].to_vec(),
            rows[..9].to_vec(),
            rows.clone(),
            revised,
            inserted,
            vec![],
        ];
        let budget = OwnedBudget::new(4 * 1024 * 1024);
        let mut normal = Harness::new();
        let mut borrowed = Harness::new();
        let mut previous_normal = None;
        let mut previous_borrowed: Option<Generation> = None;
        for (index, case) in cases.iter().enumerate() {
            let expected = run_generation(
                &mut normal,
                case,
                previous_normal.as_ref(),
                "borrowed oracle",
            );
            let (bars, raw, events) = ledger(case);
            let facts = facts_of(&bars);
            borrowed.bind(&raw);
            let owner = OwnedJson::parse(&serde_json::to_vec(&events).unwrap(), &budget).unwrap();
            let all_events = owner.view().as_array().unwrap();
            let raw_views = all_events
                .iter()
                .map(|event| (event_coord(event).unwrap(), event))
                .collect();
            let prior = PriorState {
                bi_records: previous_borrowed
                    .as_ref()
                    .map(Generation::bi_records)
                    .unwrap_or_default(),
                version_record: previous_borrowed
                    .as_ref()
                    .and_then(Generation::version_record),
            };
            let attempts = budget.snapshot().allocation_attempts;
            let actual = borrowed
                .bi
                .build_borrowed(
                    &mut borrowed.producer,
                    &mut borrowed.store,
                    &BiBuildInputView {
                        facts: &facts,
                        raw: BiLedgerView::Views(&raw_views),
                        all_events,
                        prior: &prior,
                        generation: GENERATION,
                        cut: CUT,
                        frontier: FRONTIER,
                        semantic_commit_ns: Some(COMMIT),
                    },
                )
                .unwrap();
            // 适配只借 arena 字段，不新造或克隆第二份 JSON 所有者。
            assert_eq!(budget.snapshot().allocation_attempts, attempts);
            for (a, b) in actual.objects.iter().zip(&expected.objects) {
                assert_eq!(
                    a.record.content_ref(),
                    b.record.content_ref(),
                    "case {index} record exact refs"
                );
                assert_eq!(
                    a.payload.content_ref(),
                    b.payload.content_ref(),
                    "case {index} payload exact refs"
                );
                assert_eq!(
                    a.input_refs.content_ref(),
                    b.input_refs.content_ref(),
                    "case {index} input refs"
                );
                assert_eq!(
                    a.source_coords.content_ref(),
                    b.source_coords.content_ref(),
                    "case {index} source coords"
                );
            }
            assert_eq!(actual.relations.len(), expected.relations.len(), "关系数量");
            for (a, b) in actual.relations.iter().zip(&expected.relations) {
                assert_eq!(render_value(a), render_value(b), "关系投影逐字段");
            }
            let actual_generation = Generation {
                objects: actual.objects,
                relations: actual.relations,
                oracle: vec![],
            };
            assert_same_roots(&actual_generation, &expected, "owned borrowed lifecycle");
            drop(owner);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            previous_normal = Some(expected);
            previous_borrowed = Some(actual_generation);
        }
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn borrowed_owned_rejects_bad_event_before_writes_and_keeps_latest_ties() {
        use newchan_rust::s_content::capture_allocation::{audit_json::OwnedJson, OwnedBudget};
        let rows = fixture_rows();
        let (bars, mut raw, mut events) = ledger(&rows[..9]);
        let facts = facts_of(&bars);
        for row in raw.values_mut() {
            row["seq"] = json!(100);
        }
        let budget = OwnedBudget::new(4 * 1024 * 1024);
        let raw_owner = OwnedJson::parse(
            &serde_json::to_vec(&raw.values().collect::<Vec<_>>()).unwrap(),
            &budget,
        )
        .unwrap();
        let raw_views = raw_owner
            .view()
            .as_array()
            .unwrap()
            .iter()
            .map(|row| (event_coord(row).unwrap(), row))
            .collect();
        let plain_latest = Ledger::new(&raw).latest().unwrap();
        let view_latest = Ledger {
            rows: BiLedgerView::Views(&raw_views),
        }
        .latest()
        .unwrap();
        assert_eq!(
            plain_latest, view_latest,
            "equal seq chooses last coordinate"
        );
        events[0]["source_coord"] = json!("01");
        let event_owner = OwnedJson::parse(&serde_json::to_vec(&events).unwrap(), &budget).unwrap();
        let prior = PriorState {
            bi_records: vec![],
            version_record: None,
        };
        let mut normal = Harness::new();
        let mut borrowed = Harness::new();
        let normal_error = normal
            .bi
            .build(
                &mut normal.producer,
                &mut normal.store,
                &BiBuildInput {
                    facts: &facts,
                    raw: &raw,
                    all_events: &events,
                    prior: &prior,
                    generation: GENERATION,
                    cut: CUT,
                    frontier: FRONTIER,
                    semantic_commit_ns: Some(COMMIT.to_string()),
                },
            )
            .err()
            .unwrap();
        let borrowed_error = borrowed
            .bi
            .build_borrowed(
                &mut borrowed.producer,
                &mut borrowed.store,
                &BiBuildInputView {
                    facts: &facts,
                    raw: BiLedgerView::Views(&raw_views),
                    all_events: event_owner.view().as_array().unwrap(),
                    prior: &prior,
                    generation: GENERATION,
                    cut: CUT,
                    frontier: FRONTIER,
                    semantic_commit_ns: Some(COMMIT),
                },
            )
            .err()
            .unwrap();
        assert_eq!(normal_error.to_string(), borrowed_error.to_string());
        assert_eq!(normal.store.puts, 0);
        assert_eq!(borrowed.store.puts, 0);
        // 事件门通过后，缺源仍按原门序显式上抛，不能重跑 normal 作兜底。
        events[0]["source_coord"] = json!(rows[0].coord.to_string());
        let valid_owner = OwnedJson::parse(&serde_json::to_vec(&events).unwrap(), &budget).unwrap();
        let normal_source_error = normal
            .bi
            .build(
                &mut normal.producer,
                &mut normal.store,
                &BiBuildInput {
                    facts: &facts,
                    raw: &raw,
                    all_events: &events,
                    prior: &prior,
                    generation: GENERATION,
                    cut: CUT,
                    frontier: FRONTIER,
                    semantic_commit_ns: None,
                },
            )
            .err()
            .unwrap();
        let borrowed_source_error = borrowed
            .bi
            .build_borrowed(
                &mut borrowed.producer,
                &mut borrowed.store,
                &BiBuildInputView {
                    facts: &facts,
                    raw: BiLedgerView::Views(&raw_views),
                    all_events: valid_owner.view().as_array().unwrap(),
                    prior: &prior,
                    generation: GENERATION,
                    cut: CUT,
                    frontier: FRONTIER,
                    semantic_commit_ns: None,
                },
            )
            .err()
            .unwrap();
        assert!(matches!(
            &borrowed_source_error,
            BiError::Content(ProducerError::MissingSource(_))
        ));
        assert_eq!(
            normal_source_error.to_string(),
            borrowed_source_error.to_string()
        );
        assert_eq!(normal.store.puts, borrowed.store.puts, "缺源前写入门序相同");
        drop(valid_owner);
        drop(event_owner);
        drop(raw_owner);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    /// 多前缀普通 append：覆盖同代确认沿用、端点延伸、稀疏坐标与七个 kind 齐备。
    #[test]
    fn parity_every_prefix_append_confirm_extension_and_sparse_coords() {
        let rows = fixture_rows();
        let mut harness = Harness::new();
        let mut previous: Option<Generation> = None;
        let mut summary = Summary::default();
        for prefix in 3..=rows.len() {
            let generation =
                run_generation(&mut harness, &rows[..prefix], previous.as_ref(), "前缀");
            summary.observe(&generation.oracle);
            previous = Some(generation);
        }
        for kind in KINDS {
            assert!(
                summary.kinds.contains(kind),
                "前缀扫描必须覆盖 kind {kind}（实际 {:?}）",
                summary.kinds
            );
        }
        assert!(
            summary.count("confirmed") > 0,
            "夹具必须覆盖同代确认沿用（confirmed 事件实际 0 次）"
        );
        assert!(
            summary.count("extended") > 0 || summary.count("endpoint_replaced") > 0,
            "夹具必须覆盖端点延伸（extended/endpoint_replaced 实际 0 次）"
        );
    }

    /// 源修订与历史插入：修订行后到、插入行坐标更小，事实代际与依赖面都要跟着变。
    #[test]
    fn parity_source_revision_and_historical_insertion() {
        let rows = fixture_rows();
        let mut harness = Harness::new();
        let base = run_generation(&mut harness, &rows[..9], None, "修订前");
        let mut revised_rows = rows[..9].to_vec();
        revised_rows[2] = Row {
            revision: 2,
            seq: 9,
            ..revised_rows[2].clone()
        };
        let revised = run_generation(&mut harness, &revised_rows, Some(&base), "源修订");
        let mut inserted_rows = rows[..9].to_vec();
        inserted_rows.push(Row {
            coord: 65,
            high: 18,
            low: 15,
            revision: 1,
            seq: 10,
        });
        let inserted = run_generation(&mut harness, &inserted_rows, Some(&revised), "历史插入");
        let generation_of = |generation: &Generation| -> String {
            let object = generation
                .oracle
                .iter()
                .find(|object| object["kind"] == BI_KIND)
                .expect("至少一条 bi");
            let payload: Value =
                serde_json::from_str(object["payload_json"].as_str().expect("payload_json"))
                    .expect("合法 JSON");
            payload["object_generation"]
                .as_str()
                .expect("object_generation")
                .to_string()
        };
        assert_eq!(generation_of(&base), "1");
        assert_eq!(generation_of(&revised), "2");
        assert_eq!(generation_of(&inserted), "2");
    }

    /// 撤回：上一代有、本代没有的实体必须产出 `withdrawn` 变更（含跨代原因轴）。
    #[test]
    fn parity_withdrawal_across_fact_generations() {
        let rows = fixture_rows();
        let mut harness = Harness::new();
        let mut revised_rows = rows[..9].to_vec();
        revised_rows[2] = Row {
            revision: 2,
            seq: 9,
            ..revised_rows[2].clone()
        };
        let older = run_generation(&mut harness, &revised_rows, None, "撤回前");
        let generation = run_generation(&mut harness, &rows[..9], Some(&older), "撤回");
        let mut summary = Summary::default();
        summary.observe(&generation.oracle);
        assert!(
            summary.count("withdrawn") > 0,
            "跨事实代际的上一代实体必须全部撤回（实际 0 次）"
        );
    }

    /// #1394 整合 W-4（#804 对拍锁）：TB-02-B 新笔事实在 v2（`tb02b::build`，合并后即 main 版、
    /// #1392 定稿）与 v3（本模块 `content_bi`）各有一份生产实现，判的是同一件事。本用例在 main
    /// 冻结的三份原始账上逐代续接地跑两侧，并用 [`assert_parity`] 逐字段对拍：TB-02-B 手工账
    /// 18 例与 TB-02-C 2 例逐前缀；TB-02-D 14 例（28–48 根）取四分点前缀续接（1/4、1/2、3/4、全长）。
    /// 任何一侧改了判据而另一侧没跟，这里先红。
    ///
    /// TB-02-D 不逐前缀是耗时取舍：debug 下单代开销随前缀长度超线性增长（实测 TB-02-C 28 根逐前缀
    /// 约 32 秒/例），14 例 × 全部前缀会把本用例拉到半小时以上；四分点续接仍让两侧都走「上一代
    /// 已发布对象 + 多根追加」这条输入。
    ///
    /// 两侧收到同一份输入（同一 parser 事实、同一有效账、同一到达序）；行的 open/close 按本夹具
    /// 既有口径取 H/L 中值（新笔事实只读 H/L 与原始引用，与冻结账给的 open/close 无关）。
    #[test]
    fn parity_lock_804_on_frozen_tb02bcd_raw_ledgers() {
        let load = |path: &str| -> Value {
            serde_json::from_slice(
                &std::fs::read(crate::tests::fixture(path)).expect("读冻结原始账"),
            )
            .expect("冻结原始账是 JSON")
        };
        // (例名, 原始行, 是否逐前缀)
        let mut cases: Vec<(String, Vec<Value>, bool)> = Vec::new();
        for case in load("tests/fixtures/tb02b/raw-ledger.json")["cases"]
            .as_array()
            .expect("TB-02-B cases")
        {
            cases.push((
                format!("tb02b/{}", case["case_id"].as_str().expect("case_id")),
                case["raw_bars"].as_array().expect("raw_bars").clone(),
                true,
            ));
        }
        for (ledger_name, path, every_prefix) in [
            ("tb02c", "tests/fixtures/tb02c/raw-ledger.json", true),
            ("tb02d", "tests/fixtures/tb02d/raw-ledger.json", false),
        ] {
            for (name, rows) in load(path)["cases"].as_object().expect("cases") {
                cases.push((
                    format!("{ledger_name}/{name}"),
                    rows.as_array().expect("rows").clone(),
                    every_prefix,
                ));
            }
        }
        assert_eq!(cases.len(), 18 + 2 + 14, "三份冻结原始账的例数");
        let mut summary = Summary::default();
        let mut generations = 0usize;
        for (name, bars, every_prefix) in &cases {
            let rows: Vec<Row> = bars
                .iter()
                .enumerate()
                .map(|(seq, bar)| Row {
                    coord: bar["raw_index"].as_i64().expect("raw_index"),
                    high: bar["high"].as_i64().expect("high"),
                    low: bar["low"].as_i64().expect("low"),
                    revision: 1,
                    seq: seq as i64,
                })
                .collect();
            let len = rows.len();
            let prefixes: Vec<usize> = if *every_prefix {
                (1..=len).collect()
            } else {
                vec![len / 4, len / 2, len * 3 / 4, len]
            };
            let started = std::time::Instant::now();
            let mut harness = Harness::new();
            let mut previous: Option<Generation> = None;
            for prefix in prefixes {
                let generation = run_generation(
                    &mut harness,
                    &rows[..prefix],
                    previous.as_ref(),
                    &format!("{name} 前缀{prefix}"),
                );
                summary.observe(&generation.oracle);
                generations += 1;
                previous = Some(generation);
            }
            eprintln!("#804 对拍 {name}: {len} 根，{:?}", started.elapsed());
        }
        assert!(generations > cases.len(), "逐代续接对拍不是假空");
        for kind in KINDS {
            assert!(
                summary.kinds.contains(kind),
                "冻结原始账对拍必须覆盖 kind {kind}（实际 {:?}）",
                summary.kinds
            );
        }
    }

    /// 相同输入的暖 memo / 冷 memo 必须给出同一批根（身份只由内容决定）。
    #[test]
    fn warm_and_cold_memos_give_identical_roots() {
        let rows = fixture_rows();
        let mut warm = Harness::new();
        let first = run_generation(&mut warm, &rows, None, "暖 memo 首轮");
        let second = run_generation(&mut warm, &rows, None, "暖 memo 次轮");
        let mut cold = Harness::new();
        let third = run_generation(&mut cold, &rows, None, "冷 memo");
        assert_same_roots(&first, &second, "暖/暖");
        assert_same_roots(&first, &third, "暖/冷");
    }

    /// 大 deps 多关系：每条关系只引用它那条 bi 的依赖根，不逐条复制。
    #[test]
    fn relations_share_their_bi_dependency_roots() {
        let rows = fixture_rows();
        let mut harness = Harness::new();
        let generation = run_generation(&mut harness, &rows, None, "关系共享");
        // 依赖根去重计数：多条关系可以共用同一组 deps，但不得各自复制一份。
        let mut distinct_deps: BTreeSet<(String, String)> = BTreeSet::new();
        let mut relations = 0usize;
        for object in &generation.objects {
            if object_kind(object) != RELATION_KIND {
                continue;
            }
            relations += 1;
            distinct_deps.insert((object.input_refs.root(), object.source_coords.root()));
            let payload: Value =
                serde_json::from_str(&render_built(&mut harness.store, &object.payload))
                    .expect("载荷是合法 JSON");
            let witness = payload["data"]["witness_object_id"]
                .as_str()
                .expect("见证")
                .to_string();
            let bi = generation
                .objects
                .iter()
                .find(|candidate| {
                    object_kind(candidate) == BI_KIND && candidate.object_id == witness
                })
                .expect("关系见证必须指向本代 bi 对象");
            assert_eq!(
                object.input_refs.root(),
                bi.input_refs.root(),
                "关系必须与其见证 bi 共用同一条输入引用根"
            );
            assert_eq!(
                object.source_coords.root(),
                bi.source_coords.root(),
                "关系必须与其见证 bi 共用同一条源坐标根"
            );
        }
        assert!(relations > 0, "夹具必须产出关系边");
        assert!(
            distinct_deps.len() < relations,
            "关系边数 {relations} 必须多于依赖根数 {}（否则每条关系都在复制 deps）",
            distinct_deps.len()
        );
    }

    /// 区间切片必须随树高而不是元素数：全账本数组 O(1) 成帧，区间数组写次数有界且不随账本增长。
    #[test]
    fn dependency_slices_do_not_scale_with_ledger_size() {
        let mut measured: Vec<usize> = Vec::new();
        for count in [64usize, 1024] {
            let rows: Vec<Row> = (0..count)
                .map(|index| Row {
                    coord: index as i64 * 10,
                    high: 100 + index as i64,
                    low: 50 + index as i64,
                    revision: 1,
                    seq: index as i64,
                })
                .collect();
            let (_, raw, _) = ledger(&rows);
            let mut harness = Harness::new();
            harness.bind(&raw);
            let mut coords = RawCoords::new();
            coords
                .sync(
                    &mut harness.store,
                    &mut harness.producer,
                    &Ledger::new(&raw),
                )
                .expect("对齐坐标序列");
            let before = harness.store.puts;
            let whole = coords.whole(&mut harness.store).expect("全账本数组");
            let whole_puts = harness.store.puts - before;
            assert_eq!(whole.source_coords.extent, Some(count as u64));
            assert!(
                whole_puts <= 4,
                "全账本数组必须 O(1) 成帧，实际 {whole_puts} 次写（{count} 坐标）"
            );
            let first = raw.keys().nth(count / 4).copied().expect("非空账本");
            let last = raw.keys().nth(count * 3 / 4).copied().expect("非空账本");
            let before = harness.store.puts;
            let interval = coords
                .interval(&mut harness.store, &[first as usize], &[last as usize], &[])
                .expect("区间数组");
            let interval_puts = harness.store.puts - before;
            assert_eq!(
                interval.source_coords.extent,
                Some((count / 2 + 1) as u64),
                "区间元素数"
            );
            assert!(
                interval_puts <= 64,
                "区间数组必须随树高而非元素数，实际 {interval_puts} 次写（{count} 坐标）"
            );
            measured.push(interval_puts);
        }
        assert!(
            measured[1] <= measured[0] + 32,
            "账本从 64 涨到 1024 时区间写次数不得随之线性增长（实际 {measured:?}）"
        );
    }

    /// 增量内容序列与共享 coords 树是**同一条判据**：同一逻辑坐标集必须给出同根
    /// （否则「同一判断两档」）。
    #[test]
    fn sequence_arrays_match_the_shared_coords_tree_roots() {
        let rows = fixture_rows();
        let mut harness = Harness::new();
        let (_, raw, _) = ledger(&rows);
        harness.bind(&raw);
        let mut coords = RawCoords::new();
        coords
            .sync(
                &mut harness.store,
                &mut harness.producer,
                &Ledger::new(&raw),
            )
            .expect("对齐坐标序列");
        let picked: Vec<usize> = raw
            .keys()
            .enumerate()
            .filter(|(index, _)| index % 3 != 1)
            .map(|(_, coord)| *coord as usize)
            .collect();
        let from_sequence = coords
            .coord_set(&mut harness.store, &picked)
            .expect("序列路径");
        let tree = super::content_producer::coords_ref_from_slice(&picked);
        let from_tree = harness
            .producer
            .coords_array(&mut harness.store, &tree)
            .expect("共享树路径");
        assert_eq!(
            from_sequence.source_coords.digest, from_tree.digest,
            "坐标数组两条路径必须同根"
        );
        assert_eq!(from_sequence.source_coords.extent, from_tree.extent);
        let refs_from_tree = harness
            .producer
            .source_refs_array(&mut harness.store, &tree)
            .expect("共享树源引用路径");
        assert_eq!(
            from_sequence.input_refs.digest, refs_from_tree.digest,
            "源引用数组两条路径必须同根"
        );
    }

    /// 上一代是信任边界：非对象记录拒绝；实体文本靠**真实语法**（`bi:` + 两段 i64）拒绝，
    /// 不靠发明的字节上限。
    #[test]
    fn prior_records_reject_non_objects_and_ungrammatical_entities() {
        let producer = Cc004Producer::new(tb02a::PROFILE, RULE_REVISION);
        let mut store = CountingStore::default();
        let scalar =
            sc::write_value(&mut store, &CValue::String("not-a-record".to_string())).expect("写值");
        let prior = PriorState {
            bi_records: vec![&scalar],
            version_record: None,
        };
        let error =
            PriorIndex::read(&producer, &mut store, &prior).expect_err("非对象记录必须拒绝");
        assert!(matches!(error, BiError::Prior { .. }), "{error}");

        let mut harness = Harness::new();
        let empty =
            sc::write_value(&mut harness.store, &CValue::Array(Vec::new())).expect("空数组");
        // 锚文本远超任何「合理长度」直觉，但它被判据拒绝的理由是 i64 语法，不是字节数。
        let entity = format!("bi:1:{}", "9".repeat(4096));
        let slot = Value::String(entity.clone());
        let data = CValue::Object(vec![("entity_id".to_string(), CValue::String(entity))]);
        let payload = payload_value(&slot, 1, data).expect("载荷");
        let fact_key = canonical_json(&Value::Array(vec![
            Value::String(BI_KIND.to_string()),
            slot.clone(),
        ]));
        let record = harness
            .producer
            .build_object(
                &mut harness.store,
                BI_KIND,
                fact_key,
                payload,
                empty.clone(),
                empty,
                1,
                CUT,
            )
            .expect("造上一代记录");
        let prior = PriorState {
            bi_records: vec![&record.record],
            version_record: None,
        };
        let error = PriorIndex::read(&harness.producer, &mut harness.store, &prior)
            .expect_err("超出 i64 语法的实体必须拒绝");
        assert!(
            matches!(
                error,
                BiError::Prior {
                    what: "entity_id",
                    ..
                }
            ),
            "{error}"
        );
    }

    /// 同一 `object_id` 换载荷不接受：用同一代两条真 bi 记录的部件交叉拼一条记录，
    /// 声称沿用左条 id、实际挂右条载荷 + 事实键，必须被六字段重导挡下。
    #[test]
    fn prior_identity_rejects_same_id_with_other_payload() {
        let rows = fixture_rows();
        let mut harness = Harness::new();
        let generation = run_generation(&mut harness, &rows, None, "同 id 换载荷");
        let bis: Vec<&TypedObject> = generation
            .objects
            .iter()
            .filter(|object| object_kind(object) == BI_KIND)
            .collect();
        assert!(bis.len() >= 2, "夹具必须产出至少两条 bi 记录");
        let (left, right) = (bis[0], bis[1]);
        let forged = sc::write_value(
            &mut harness.store,
            &CValue::Object(vec![
                (
                    "object_id".to_string(),
                    CValue::String(left.object_id.clone()),
                ),
                ("kind".to_string(), CValue::String(BI_KIND.to_string())),
                (
                    "fact_key".to_string(),
                    CValue::String(right.fact_key.clone()),
                ),
                ("payload".to_string(), CValue::Edge(right.payload.digest)),
                (
                    "input_refs".to_string(),
                    CValue::Edge(right.input_refs.digest),
                ),
            ]),
        )
        .expect("造「同 id 换载荷」记录");
        let prior = PriorState {
            bi_records: vec![&forged],
            version_record: None,
        };
        let error = PriorIndex::read(&harness.producer, &mut harness.store, &prior)
            .expect_err("同 id 换载荷必须拒绝");
        assert!(
            matches!(
                error,
                BiError::Prior {
                    what: "object_id",
                    ..
                }
            ),
            "{error}"
        );
    }

    /// 上一代版本记录必须真是 `CC-056.version`：拿一条 `CC-010.bi` 冒充必须被 `kind` 挡下。
    #[test]
    fn prior_version_record_must_be_cc056_version() {
        let rows = fixture_rows();
        let mut harness = Harness::new();
        let generation = run_generation(&mut harness, &rows, None, "版本记录 kind");
        let bi = generation
            .objects
            .iter()
            .find(|object| object_kind(object) == BI_KIND)
            .expect("夹具必须产出 bi 对象");
        let prior = PriorState {
            bi_records: Vec::new(),
            version_record: Some(&bi.record),
        };
        let error = PriorIndex::read(&harness.producer, &mut harness.store, &prior)
            .expect_err("bi 对象冒充版本记录必须拒绝");
        assert!(
            matches!(error, BiError::Prior { what: "kind", .. }),
            "{error}"
        );
    }

    /// 载荷头是**指定协议语义**，重导 id 不能代替：造一条 id 自洽但 `semantic_version` 错的
    /// 记录——先证明六字段身份核验会放行（重导 id 与声明逐字相同），再证明头部核验拒绝它。
    #[test]
    fn prior_payload_header_is_required_not_implied_by_identity() {
        let mut harness = Harness::new();
        let empty =
            sc::write_value(&mut harness.store, &CValue::Array(Vec::new())).expect("空数组");
        let slot = Value::String("bi:1:1".to_string());
        // 一条**其余字段齐备**的旧记录：只把载荷头改错，且 id 按被改过的载荷重导。
        let leaf = sc::write_value(&mut harness.store, &CValue::String("leaf".to_string()))
            .expect("叶节点");
        let data = CValue::Object(vec![
            (
                "entity_id".to_string(),
                CValue::String("bi:1:1".to_string()),
            ),
            (
                "entity_revision".to_string(),
                CValue::String("1".to_string()),
            ),
            (
                "end".to_string(),
                CValue::Object(vec![(
                    "group_anchor".to_string(),
                    CValue::String("1".to_string()),
                )]),
            ),
            ("start".to_string(), CValue::Edge(leaf.digest)),
            ("formed_known_at".to_string(), CValue::Edge(leaf.digest)),
            ("formed_evidence".to_string(), CValue::Edge(leaf.digest)),
            ("confirmed_known_at".to_string(), CValue::Null),
            ("confirmation".to_string(), CValue::Null),
            ("version_causes".to_string(), CValue::Edge(leaf.digest)),
        ]);
        let CValue::Object(mut fields) = payload_value(&slot, 1, data).expect("载荷") else {
            panic!("载荷必须是对象");
        };
        for (name, value) in fields.iter_mut() {
            if name == "semantic_version" {
                *value = CValue::String("not-the-rule/0".to_string());
            }
        }
        let fact_key = canonical_json(&Value::Array(vec![
            Value::String(BI_KIND.to_string()),
            slot.clone(),
        ]));
        let record = harness
            .producer
            .build_object(
                &mut harness.store,
                BI_KIND,
                fact_key.clone(),
                CValue::Object(fields),
                empty.clone(),
                empty,
                1,
                CUT,
            )
            .expect("造头错的上一代记录");
        // 前提：id 由同一份被改过的载荷重导，逐字相同 ⟹ 只做身份核验的实现会放行。
        let derived = harness
            .producer
            .typed_object_id(BI_KIND, &fact_key, &record.payload, &record.input_refs)
            .expect("按同一配方重导 id");
        assert_eq!(derived, record.object_id, "本用例前提：id 自洽但头错");

        let prior = PriorState {
            bi_records: vec![&record.record],
            version_record: None,
        };
        let error = PriorIndex::read(&harness.producer, &mut harness.store, &prior)
            .expect_err("载荷头错误必须拒绝");
        assert!(
            matches!(
                error,
                BiError::Prior {
                    what: "semantic_version",
                    ..
                }
            ),
            "{error}"
        );
    }

    /// 空账本上的撤回：旧配方 `latest` 为空时是 `Value::Null`，`event_at` 必须是**精确 null**，
    /// 不能造成空字符串默认。
    #[test]
    fn withdrawal_on_empty_ledger_keeps_null_event_at() {
        let rows = fixture_rows();
        let mut harness = Harness::new();
        let prior = run_generation(&mut harness, &rows[..9], None, "撤回前");
        let generation = run_generation(&mut harness, &[], Some(&prior), "空账撤回");
        let mut withdrawn = 0usize;
        for object in &generation.objects {
            if object_kind(object) != CHANGE_KIND {
                continue;
            }
            let payload: Value =
                serde_json::from_str(&render_built(&mut harness.store, &object.payload))
                    .expect("载荷是合法 JSON");
            if payload["data"]["change"].as_str() != Some("withdrawn") {
                continue;
            }
            withdrawn += 1;
            assert_eq!(
                payload["data"]["event_at"],
                Value::Null,
                "空账撤回的 event_at 必须是精确 null"
            );
        }
        assert!(withdrawn > 0, "夹具必须产出撤回事件");
    }

    /// 对齐块切片与「先切 end 再切 start」的朴素路径是**同一条判据**：块必须恰好铺满区间，
    /// 两条路径必须给出同根同叶数（否则共享序列上就出现了两个切片口径）。
    #[test]
    fn slice_sequence_tiles_and_matches_the_naive_splits() {
        for count in [1u64, 2, 3, 7, 8, 16, 63, 64, 100, 1024] {
            for (start, end) in [
                (0, count),
                (0, 0),
                (0, count / 3),
                (count / 4, count * 3 / 4 + 1),
                (count.saturating_sub(1), count),
            ] {
                let blocks = cover_blocks(count, start, end);
                let mut cursor = start;
                for (lo, len) in &blocks {
                    assert_eq!(
                        *lo, cursor,
                        "覆盖块必须按序无缝（n={count} {start}..{end}）"
                    );
                    assert!(*len > 0, "覆盖块必须非空（n={count} {start}..{end}）");
                    cursor += len;
                }
                assert_eq!(
                    cursor, end,
                    "覆盖块必须恰好铺满区间（n={count} {start}..{end}）"
                );

                let rows: Vec<Row> = (0..count)
                    .map(|index| Row {
                        coord: index as i64 * 10,
                        high: 100 + index as i64,
                        low: 50 + index as i64,
                        revision: 1,
                        seq: index as i64,
                    })
                    .collect();
                let (_, raw, _) = ledger(&rows);
                let mut harness = Harness::new();
                harness.bind(&raw);
                let mut coords = RawCoords::new();
                coords
                    .sync(
                        &mut harness.store,
                        &mut harness.producer,
                        &Ledger::new(&raw),
                    )
                    .expect("对齐坐标序列");
                let sliced =
                    slice_sequence(&mut harness.store, &coords.text, start, end).expect("块切片");
                assert_eq!(
                    sliced.count(),
                    end - start,
                    "切片叶数（n={count} {start}..{end}）"
                );
                let (prefix, _) =
                    sc::sequence_split(&mut harness.store, &coords.text, end).expect("先切 end");
                let naive = if start == 0 {
                    prefix
                } else {
                    sc::sequence_split(&mut harness.store, &prefix, start)
                        .expect("前缀再切 start")
                        .1
                };
                assert_eq!(
                    sliced.root(),
                    naive.root(),
                    "块切片与朴素双切必须同根（n={count} {start}..{end}）"
                );
            }
        }
    }

    /// 拒写 / 回滚后的恢复对拍：`RawCoords::sync` 半途失败不得留下半更新车道；解除拒写后
    /// 重跑同一输入必须与干净重跑逐根相同（车道级 + 构造器级两条路径）。
    #[test]
    fn raw_coords_recover_after_refused_write_with_same_roots() {
        let rows = fixture_rows();
        let head = &rows[..rows.len() - 1];
        let (_, raw_head, _) = ledger(head);
        let (_, raw_full, _) = ledger(&rows);

        // ① 车道级：首代建好后，追加尾部时在 sync 内部拒写。
        let mut store = CountingStore::default();
        let mut producer = Cc004Producer::new(tb02a::PROFILE, RULE_REVISION);
        for (coord, row) in &raw_head {
            producer
                .bind_source(&mut store, *coord, spec_of(row))
                .expect("登记源版本");
        }
        let mut lane = RawCoords::new();
        lane.sync(&mut store, &mut producer, &Ledger::new(&raw_head))
            .expect("首代车道");
        let head_coords: Vec<i64> = raw_head.keys().copied().collect();
        assert_eq!(lane.coords, head_coords, "首代车道坐标");
        let before = store.puts;
        store.refuse_put_after = Some(before + 3);
        let error = lane
            .sync(&mut store, &mut producer, &Ledger::new(&raw_full))
            .expect_err("追加尾部时注入拒写必须上抛");
        assert!(matches!(error, BiError::Content(_)), "{error}");
        assert!(
            store.puts > before + 3,
            "拒写前必须已成功写过若干节点，否则测的不是半途失败"
        );
        assert_eq!(
            lane.coords, head_coords,
            "半途失败后坐标车道必须逐项保持上一代整根"
        );
        assert_eq!(
            lane.refs_leaves.len(),
            head_coords.len(),
            "半途失败后引用叶车道必须保持上一代整根"
        );
        store.refuse_put_after = None;
        for (coord, row) in &raw_full {
            if !raw_head.contains_key(coord) {
                producer
                    .bind_source(&mut store, *coord, spec_of(row))
                    .expect("登记新源版本");
            }
        }
        lane.sync(&mut store, &mut producer, &Ledger::new(&raw_full))
            .expect("解除拒写后必须可恢复");

        let mut clean_store = CountingStore::default();
        let mut clean_producer = Cc004Producer::new(tb02a::PROFILE, RULE_REVISION);
        for (coord, row) in &raw_full {
            clean_producer
                .bind_source(&mut clean_store, *coord, spec_of(row))
                .expect("登记源版本");
        }
        let mut clean_lane = RawCoords::new();
        clean_lane
            .sync(
                &mut clean_store,
                &mut clean_producer,
                &Ledger::new(&raw_full),
            )
            .expect("干净车道");
        let recovered_whole = lane.whole(&mut store).expect("恢复后整账本依赖面");
        let clean_whole = clean_lane
            .whole(&mut clean_store)
            .expect("干净整账本依赖面");
        assert_eq!(
            recovered_whole.source_coords.digest, clean_whole.source_coords.digest,
            "拒写恢复后坐标数组必须与干净车道同根"
        );
        assert_eq!(
            recovered_whole.input_refs.digest, clean_whole.input_refs.digest,
            "拒写恢复后源引用数组必须与干净车道同根"
        );

        // ② 构造器级：首代与追加代各拒写一次，解除后都与干净跑对拍同根。
        let mut clean = Harness::new();
        let clean_head = run_generation(&mut clean, head, None, "干净首代");
        let clean_full = run_generation(&mut clean, &rows, Some(&clean_head), "干净追加代");

        let mut harness = Harness::new();
        let error = attempt_generation(&mut harness, head, None, |store| {
            store.refuse_put_after = Some(store.puts + 4);
        })
        .expect_err("注入拒写必须上抛");
        assert!(matches!(error, BiError::Content(_)), "{error}");
        assert!(
            harness.bi.coords.coords.is_empty(),
            "首代半途失败后坐标车道必须仍是失败前的空整根"
        );
        harness.store.refuse_put_after = None;
        let recovered_head =
            attempt_generation(&mut harness, head, None, |_| {}).expect("解除拒写后必须可恢复");
        assert_same_roots(&clean_head, &recovered_head, "首代拒写恢复");
        let recovered_full = attempt_generation(&mut harness, &rows, Some(&recovered_head), |_| {})
            .expect("追加代必须可恢复");
        assert_same_roots(&clean_full, &recovered_full, "追加代拒写恢复");
    }
}
