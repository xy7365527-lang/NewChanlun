//! #1394 采纳 C §5.2–5.4：S3 对象记录的组装（生成态 sealed 记录 ↔ 公开 wire 记录）。
//!
//! 规范正本：`.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md`
//! 第 5.2（唯一内容编码）、5.3（身份配方与全文绑定）与 5.4（S 发布清单与无循环派生）；
//! 采纳状态以同目录 `ADOPTION.json`、`SPEC-ADDENDUM.md` 为准。
//!
//! 本模块吃的是 [`crate::content_producer`] 已经写好的三型产物（`TypedObject` /
//! `Cc006Object` / `Cc007Object`），产出**单条**记录：
//!
//! - [`generated`]：原 sealed 批次 `generated_state.objects[i]` 的逻辑字段。生成态元数据
//!   （`first_known_generation` / `first_known_cut` / `published_generation`）取 producer 的
//!   **当前生成代**；不含 `batch_id`，也不含任何撤回字段（旧封存记录本就没有）。
//! - [`wire_template`]：现行公开单条记录（Snapshot `objects` / `withdrawn_objects`、Delta
//!   `upserts` 的成员）。首获知、发布、撤回与批号全部由调用者按**独立索引**给出的
//!   [`PublicationLife`] 传进来；本工厂**不猜出生**。本批新生对象的 `batch_id` 写 0x08
//!   占位（因此一律按模板模式写出），旧对象写索引给出的 `batch3-` 文本。
//!
//! ## 两条记录的 exact 字段集
//!
//! 生成态（`generated`，与旧行逐字段同名同型）：
//!
//! - 泛型六字段对象（CC-004/005/007/054…）：`object_id, object_revision, kind, fact_key,
//!   payload_json, input_refs_json, source_coords_json, first_known_generation, first_known_cut,
//!   published_generation`（10）。
//! - CC-006 局部形态：`object_id, object_revision, kind, branch, dir_ab, dir_bc, window_start,
//!   window_mid, window_end, comparisons_json, input_refs_json, source_coords_json,
//!   first_known_generation, first_known_cut, published_generation`（15）。
//!
//! 公开 wire（`wire_template`，与 `tb02a::project` / `project_object_wire` 逐字段同名同型）：
//!
//! - 泛型六字段对象 15 字段：`… batch_id, fact_key, payload, input_refs, source_coords,
//!   … withdrawn_generation, withdrawal_reason, superseded_by, lifecycle`。
//! - CC-006 20 字段：`… batch_id, branch, dir_ab, dir_bc, window_start, window_mid, window_end,
//!   comparisons, input_refs, source_coords, … lifecycle`（**没有** `fact_key`）。
//!
//! ## 三条内容纪律（实现与测试都按它核）
//!
//! 1. **0x07 只在获准位置**：生成态 `payload_json` / `input_refs_json` / `source_coords_json`
//!    直接复用 producer 的 0x07 文本视图边；CC-006 的 `comparisons_json` **不是**获准 0x07
//!    位置（§5.2 的明确列），按旧规范文本重建为普通字符串（比较集是固定小结构，属允许精确
//!    读的小元数据）。其余任何位置不得出现 0x07／0x08。
//! 2. **两种编码各走自己的边，互不代用**：wire 的 `payload` / `input_refs` /
//!    `source_coords` 直接借 producer 给出的公开值根；CC-006 的 `input_refs` 借 producer
//!    **同次构造**的 `wire_input_refs`（原 generic 字符串源叶 + 共享序列，`merged_index` 字符串），
//!    不得改用生成态数值叶重渲染。生成态一侧保留原配方的类型与值：typed 的 `raw_ref` 全文本叶、
//!    CC-006 的数值 `merged_index`/`revision`/`seq` 照存，数值→文本只发生在 wire 一侧（§5.3）。
//! 3. **不做全闭包 `verify`**：`record` 的三条内容边指向载荷／来源／坐标大根；内容层 `verify`
//!    沿去重 DAG 遍历根，不展开成完整逻辑 JSON。发布前的完整闭包核验归发布链自己的读集与
//!    §5.4 的写前审计；本模块只沿 path 触碰小字段，也不代表历史审计已经完成。
//!
//! ## 边界照实
//!
//! - **出生许可不在本模块**：`BatchRef::CurrentBatch` / `Existing(..)` 与 `first_known_*`、
//!   `published_*`、撤回元数据都由调用者的独立索引给；本模块只核它们彼此自洽（见
//!   [`validate_life`] 的逐条清单），**不证明**「这条对象确属本批新生」或「这条撤回确已发生」。
//! - 本模块由根入口挂载（`mod object_records;`）；挂线前其 API 只被本文件测试使用，故整模块
//!   抑制 `dead_code`（与 `content_producer` / `content_bi` 同口径，而不是逐项 allow）。

#![allow(dead_code)]

use crate::content_producer::{
    Cc006Object, Cc007Object, TypedObject, CC006_SHAPE_KIND, CC007_FRACTAL_KIND,
};
use crate::v3::audit_store::{CodecRead, CodecStore};
use newchan_rust::s_content::path::{lookup_path, path_code, PathStep};
use newchan_rust::s_content::{
    self as sc, Built, CodecError, ContentRef, Kind, NodeStore, NumberToken, Value as CValue,
};
use std::fmt;

/// 泛型六字段对象的内部辅助记录字段集：十项旧生成态字段 + 三条内容边
/// （`content_producer::TypedObject::record` 的唯一形状）。
pub(super) const TYPED_RECORD_FIELDS: [&str; 13] = [
    "object_id",
    "object_revision",
    "kind",
    "fact_key",
    "payload",
    "input_refs",
    "source_coords",
    "payload_json",
    "input_refs_json",
    "source_coords_json",
    "first_known_generation",
    "first_known_cut",
    "published_generation",
];
/// 原 sealed `generated_state.objects` 单条（泛型）：旧行的十列。
pub(super) const TYPED_SEALED_FIELDS: [&str; 10] = [
    "object_id",
    "object_revision",
    "kind",
    "fact_key",
    "payload_json",
    "input_refs_json",
    "source_coords_json",
    "first_known_generation",
    "first_known_cut",
    "published_generation",
];
/// 公开 wire 单条（泛型）：`tb02a::project` 的十五项。
pub(super) const TYPED_WIRE_FIELDS: [&str; 15] = [
    "object_id",
    "object_revision",
    "kind",
    "batch_id",
    "fact_key",
    "payload",
    "input_refs",
    "source_coords",
    "first_known_generation",
    "first_known_cut",
    "published_generation",
    "withdrawn_generation",
    "withdrawal_reason",
    "superseded_by",
    "lifecycle",
];
/// CC-006 内部辅助记录字段集：旧行字段 + 三条内容边（`comparisons` 取代 `comparisons_json`）。
pub(super) const SHAPE_RECORD_FIELDS: [&str; 17] = [
    "object_id",
    "object_revision",
    "kind",
    "branch",
    "window_start",
    "window_mid",
    "window_end",
    "dir_ab",
    "dir_bc",
    "comparisons",
    "input_refs",
    "source_coords",
    "input_refs_json",
    "source_coords_json",
    "first_known_generation",
    "first_known_cut",
    "published_generation",
];
/// 原 sealed `generated_state.objects` 单条（CC-006）：旧行的十五列。
pub(super) const SHAPE_SEALED_FIELDS: [&str; 15] = [
    "object_id",
    "object_revision",
    "kind",
    "branch",
    "window_start",
    "window_mid",
    "window_end",
    "dir_ab",
    "dir_bc",
    "comparisons_json",
    "input_refs_json",
    "source_coords_json",
    "first_known_generation",
    "first_known_cut",
    "published_generation",
];
/// 公开 wire 单条（CC-006）：`project_object_wire` 的二十项。
pub(super) const SHAPE_WIRE_FIELDS: [&str; 20] = [
    "object_id",
    "object_revision",
    "kind",
    "batch_id",
    "branch",
    "dir_ab",
    "dir_bc",
    "window_start",
    "window_mid",
    "window_end",
    "comparisons",
    "input_refs",
    "source_coords",
    "first_known_generation",
    "first_known_cut",
    "published_generation",
    "withdrawn_generation",
    "withdrawal_reason",
    "superseded_by",
    "lifecycle",
];
/// §5.3 批次清单 id 前缀（新域唯一形状；与 `s_content::path` 的占位替换同一判据）。
const BATCH_ID_PREFIX: &str = "batch3-";

/// 组装拒绝面：源记录形状／发布生命期／kind／内容编解码。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RecordError {
    /// 内容层编解码：缺节点、摘要不符、形状非法（含路径算子报出的非「缺字段」错误）。
    Codec(CodecError),
    /// producer 内部辅助记录与所调对象不符：缺字段／多字段／小字段值不匹配／边不同根。
    Source { what: &'static str, detail: String },
    /// 独立索引给出的发布生命期不合法（见 [`validate_life`] 的逐条判据）。
    Life { what: &'static str, detail: String },
    /// 跨 kind 混用：所调对象类型的字段面与其记录的 `kind` 不符。
    Kind { what: &'static str, detail: String },
}

impl From<CodecError> for RecordError {
    fn from(error: CodecError) -> Self {
        RecordError::Codec(error)
    }
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecordError::Codec(error) => write!(f, "内容编解码错误：{error}"),
            RecordError::Source { what, detail } => write!(f, "{what} 与构造结果不符：{detail}"),
            RecordError::Life { what, detail } => write!(f, "发布生命期非法（{what}）：{detail}"),
            RecordError::Kind { what, detail } => write!(f, "跨 kind 拒绝（{what}）：{detail}"),
        }
    }
}

impl std::error::Error for RecordError {}

/// 借用的内容对象：三型产物各自带公开边与内部辅助记录。
///
/// `Copy`：三型都只借引用，调用方（生成态 + wire 两趟）不必重建枚举值。
#[derive(Clone, Copy)]
pub(super) enum BorrowedObject<'a> {
    /// 泛型六字段对象（CC-004 步骤 / CC-005 组 / CC-054 知识态…）的产物。
    Typed(&'a TypedObject),
    /// CC-006 局部形态对象（独立十二字段身份）的产物。
    Shape(&'a Cc006Object),
    /// CC-007 分型描述对象（泛型六字段身份 + 三条知识请求）的产物。
    Fractal(&'a Cc007Object),
}

/// 对象首版批号来源：本批新生写 0x08 占位；旧对象写独立索引给的首版 `batch3-` 文本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum BatchRef {
    /// 本批新生：wire 的 `batch_id` 写 0x08 空占位，由 manifest 的 batch3 身份解析。
    CurrentBatch,
    /// 旧对象：首版批号文本（`batch3-` + 64 位小写十六进制），原样保留、不换成本批身份。
    Existing(String),
}

/// 独立索引给出的发布生命期 + 批号（工厂不猜出生，也不写独立索引）。
///
/// 字段语义与 `content_publication::ObjectBirth` 同源：`first_known_*` 是首获知，
/// `published_generation` 是本次发布代号，撤回三元组来自持久撤回记录。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PublicationLife {
    pub first_known_generation: i64,
    pub first_known_cut: String,
    pub published_generation: i64,
    pub withdrawn_generation: Option<i64>,
    pub withdrawal_reason: Option<String>,
    pub superseded_by: Option<String>,
    pub batch: BatchRef,
}

/// 生成态单条记录（原 sealed `generated_state.objects[i]` 的逻辑字段）。
///
/// 生成态元数据取 producer 的**当前生成代**（`record` 里的三件套，本函数逐项与构造结果对核后
/// 才写出）；不含 `batch_id`、不含撤回字段。`payload_json` / `input_refs_json` /
/// `source_coords_json` 是 producer 的 0x07 文本视图边；CC-006 的 `comparisons_json` 是固定
/// 小结构的规范文本字符串（非 0x07）。
pub(super) fn generated(
    store: &mut dyn NodeStore,
    object: BorrowedObject<'_>,
) -> Result<Built, RecordError> {
    match object {
        BorrowedObject::Typed(source) => {
            let view = TypedView::of_typed(source);
            let record = read_typed_record(store, &view, None)?;
            typed_generated(store, &record)
        }
        BorrowedObject::Fractal(source) => {
            let view = TypedView::of_fractal(source);
            let record = read_typed_record(store, &view, Some(CC007_FRACTAL_KIND))?;
            typed_generated(store, &record)
        }
        BorrowedObject::Shape(source) => {
            let record = read_shape_record(store, source)?;
            shape_generated(store, &record)
        }
    }
}

/// 公开单条记录模板（Snapshot `objects` / `withdrawn_objects`、Delta `upserts` 的成员）。
///
/// 一律按**模板模式**写出：`BatchRef::CurrentBatch` 需要 0x08 占位；`Existing` 时同一份字节也
/// 是合法公开值。首获知／发布／撤回元数据全部取自 `life`，本函数不回读也不改写独立索引。
///
/// `current_generation` 是**调用方本批发布代号**（来自本次 Begin/发布上下文），不是生成记录的
/// 代号：旧出生/已撤回对象要从原生成记录直接构造**当前代**快照，原记录必须保留它自己的
/// generation，不得为凑合法性重造或改写旧记录（§5.4「旧对象和旧观察保留原首版 batch_id」）。
pub(super) fn wire_template(
    store: &mut dyn NodeStore,
    object: BorrowedObject<'_>,
    life: &PublicationLife,
    current_generation: i64,
) -> Result<Built, RecordError> {
    wire_template_view(store, object, life.view(), current_generation)
}

pub(super) fn wire_template_view(
    store: &mut (impl CodecStore + ?Sized),
    object: BorrowedObject<'_>,
    life: LifeView<'_>,
    current_generation: i64,
) -> Result<Built, RecordError> {
    match object {
        BorrowedObject::Typed(source) => {
            let view = TypedView::of_typed(source);
            let record = read_typed_record(store, &view, None)?;
            validate_life_view(life, current_generation, &record.object_id)?;
            typed_wire(store, &record, life)
        }
        BorrowedObject::Fractal(source) => {
            let view = TypedView::of_fractal(source);
            let record = read_typed_record(store, &view, Some(CC007_FRACTAL_KIND))?;
            validate_life_view(life, current_generation, &record.object_id)?;
            typed_wire(store, &record, life)
        }
        BorrowedObject::Shape(source) => {
            let record = read_shape_record(store, source)?;
            validate_life_view(life, current_generation, &record.object_id)?;
            shape_wire(store, &record, life)
        }
    }
}

/// 泛型六字段对象的两型产物（`TypedObject` / `Cc007Object`）的公共借用视图。
///
/// 两型在记录层面同形（同一个 `TypedObject::record` 配方），差别只在 `Fractal` 入口额外核
/// `kind == CC-007`：不另写第二份记录组装。
struct TypedView<'a> {
    object_id: &'a str,
    fact_key: &'a str,
    record: &'a Built,
    payload: &'a Built,
    input_refs: &'a Built,
    source_coords: &'a Built,
    payload_json: &'a Built,
    input_refs_json: &'a Built,
    source_coords_json: &'a Built,
    generation: i64,
    cut: &'a str,
}

impl<'a> TypedView<'a> {
    fn of_typed(object: &'a TypedObject) -> Self {
        TypedView {
            object_id: &object.object_id,
            fact_key: &object.fact_key,
            record: &object.record,
            payload: &object.payload,
            input_refs: &object.input_refs,
            source_coords: &object.source_coords,
            payload_json: &object.payload_json,
            input_refs_json: &object.input_refs_json,
            source_coords_json: &object.source_coords_json,
            generation: object.generation,
            cut: &object.cut,
        }
    }

    fn of_fractal(object: &'a Cc007Object) -> Self {
        TypedView {
            object_id: &object.object_id,
            fact_key: &object.fact_key,
            record: &object.record,
            payload: &object.payload,
            input_refs: &object.input_refs,
            source_coords: &object.source_coords,
            payload_json: &object.payload_json,
            input_refs_json: &object.input_refs_json,
            source_coords_json: &object.source_coords_json,
            generation: object.generation,
            cut: &object.cut,
        }
    }
}

/// 泛型记录里精确读出的**小字段**（三条内容边只带根，不带子图）。
struct TypedRecord {
    object_id: super::business_json::TextValue,
    object_revision: i64,
    kind: super::business_json::TextValue,
    fact_key: super::business_json::TextValue,
    first_known_generation: i64,
    first_known_cut: super::business_json::TextValue,
    published_generation: i64,
    payload: super::audit_store::RefValue,
    input_refs: super::audit_store::RefValue,
    source_coords: super::audit_store::RefValue,
    payload_json: super::audit_store::RefValue,
    input_refs_json: super::audit_store::RefValue,
    source_coords_json: super::audit_store::RefValue,
}

/// CC-006 记录里精确读出的**小字段**（`comparisons`、生成态三条边与 producer 同次构造的公开
/// `wire_input_refs` 都只带根）。
struct ShapeRecord {
    object_id: super::business_json::TextValue,
    object_revision: i64,
    kind: super::business_json::TextValue,
    branch: super::business_json::TextValue,
    dir_ab: super::business_json::TextValue,
    dir_bc: super::business_json::TextValue,
    window: [i64; 3],
    comparisons: super::audit_store::RefValue,
    input_refs: super::audit_store::RefValue,
    source_coords: super::audit_store::RefValue,
    wire_input_refs: super::audit_store::RefValue,
    input_refs_json: super::audit_store::RefValue,
    source_coords_json: super::audit_store::RefValue,
    first_known_generation: i64,
    first_known_cut: super::business_json::TextValue,
    published_generation: i64,
}

/// 泛型记录的完整小字段读 + 形状核（唯一的泛型记录判据）。
///
/// 逐条核：字段集 exact（计数 + 全键可定位 ⟹ 无缺无多）→ 文本/整数类型 → 记录与构造结果的
/// `object_id` / `fact_key` / 当前代三件套逐字一致 → `kind` 与槽位一致且不是 CC-006 →
/// 三条内容边与三条 0x07 视图边和构造结果的根与 kind 完全一致。
fn read_typed_record(
    store: &mut (impl CodecStore + ?Sized),
    view: &TypedView<'_>,
    expect_kind: Option<&str>,
) -> Result<TypedRecord, RecordError> {
    let what = "泛型 typed 记录";
    let record = store.native_built_ref(view.record)?;
    expect_object_shape(store, record.view(), what, &TYPED_RECORD_FIELDS)?;

    let object_id = read_text(store, record.view(), "object_id", what)?;
    let object_revision = read_int(store, record.view(), "object_revision", what)?;
    let kind = read_text(store, record.view(), "kind", what)?;
    let fact_key = read_text(store, record.view(), "fact_key", what)?;
    let first_known_generation = read_int(store, record.view(), "first_known_generation", what)?;
    let first_known_cut = read_text(store, record.view(), "first_known_cut", what)?;
    let published_generation = read_int(store, record.view(), "published_generation", what)?;

    if object_id != view.object_id {
        return Err(RecordError::Source {
            what,
            detail: format!(
                "记录声明 `{object_id}`，构造结果身份 `{}`；同一次调用不得混用两条对象",
                view.object_id
            ),
        });
    }
    if fact_key != view.fact_key {
        return Err(RecordError::Source {
            what,
            detail: format!(
                "记录槽位 `{fact_key}` 与构造结果槽位 `{}` 不同",
                view.fact_key
            ),
        });
    }
    if kind.is_empty() {
        return Err(RecordError::Kind {
            what,
            detail: "kind 不得为空".to_string(),
        });
    }
    if kind == CC006_SHAPE_KIND {
        return Err(RecordError::Kind {
            what,
            detail: format!("泛型面不得承载 CC-006 形态对象（kind `{kind}`）"),
        });
    }
    if let Some(expected) = expect_kind {
        if kind != expected {
            return Err(RecordError::Kind {
                what,
                detail: format!("Fractal 入口只接受 `{expected}`，记录 kind 实为 `{kind}`"),
            });
        }
    }
    check_fact_key_kind(store, &fact_key, &kind, what)?;
    if first_known_generation != view.generation
        || published_generation != view.generation
        || first_known_cut != view.cut
    {
        return Err(RecordError::Source {
            what,
            detail: format!(
                "生成态三件套（{first_known_generation}/{first_known_cut}/{published_generation}）与本次构造（{}/{}/{}）不同",
                view.generation, view.cut, view.generation
            ),
        });
    }

    let payload = read_edge(store, record.view(), "payload", Kind::Object, what)?;
    let input_refs = read_edge(store, record.view(), "input_refs", Kind::Array, what)?;
    let source_coords = read_edge(store, record.view(), "source_coords", Kind::Array, what)?;
    let payload_json = read_edge(store, record.view(), "payload_json", Kind::JsonText, what)?;
    let input_refs_json = read_edge(
        store,
        record.view(),
        "input_refs_json",
        Kind::JsonText,
        what,
    )?;
    let source_coords_json = read_edge(
        store,
        record.view(),
        "source_coords_json",
        Kind::JsonText,
        what,
    )?;
    for (name, edge, built) in [
        ("payload", &payload, view.payload),
        ("input_refs", &input_refs, view.input_refs),
        ("source_coords", &source_coords, view.source_coords),
        ("payload_json", &payload_json, view.payload_json),
        ("input_refs_json", &input_refs_json, view.input_refs_json),
        (
            "source_coords_json",
            &source_coords_json,
            view.source_coords_json,
        ),
    ] {
        if edge.digest()? != built.digest || edge.kind() != built.kind {
            return Err(RecordError::Source {
                what,
                detail: format!("`{name}` 边与构造结果的根/类型不符"),
            });
        }
    }
    Ok(TypedRecord {
        object_id,
        object_revision,
        kind,
        fact_key,
        first_known_generation,
        first_known_cut,
        published_generation,
        payload,
        input_refs,
        source_coords,
        payload_json,
        input_refs_json,
        source_coords_json,
    })
}

/// CC-006 记录的完整小字段读 + 形状核（唯一的形态记录判据）。
fn read_shape_record(
    store: &mut (impl CodecStore + ?Sized),
    source: &Cc006Object,
) -> Result<ShapeRecord, RecordError> {
    let what = "CC-006 记录";
    let record = store.native_built_ref(&source.record)?;
    expect_object_shape(store, record.view(), what, &SHAPE_RECORD_FIELDS)?;

    let object_id = read_text(store, record.view(), "object_id", what)?;
    let object_revision = read_int(store, record.view(), "object_revision", what)?;
    let kind = read_text(store, record.view(), "kind", what)?;
    let branch = read_text(store, record.view(), "branch", what)?;
    let dir_ab = read_text(store, record.view(), "dir_ab", what)?;
    let dir_bc = read_text(store, record.view(), "dir_bc", what)?;
    let window = [
        read_int(store, record.view(), "window_start", what)?,
        read_int(store, record.view(), "window_mid", what)?,
        read_int(store, record.view(), "window_end", what)?,
    ];
    let first_known_generation = read_int(store, record.view(), "first_known_generation", what)?;
    let first_known_cut = read_text(store, record.view(), "first_known_cut", what)?;
    let published_generation = read_int(store, record.view(), "published_generation", what)?;

    if kind != CC006_SHAPE_KIND {
        return Err(RecordError::Kind {
            what,
            detail: format!("Shape 入口只接受 `{CC006_SHAPE_KIND}`，记录 kind 实为 `{kind}`"),
        });
    }
    if object_id != source.object_id {
        return Err(RecordError::Source {
            what,
            detail: format!(
                "记录声明 `{object_id}`，构造结果身份 `{}`",
                source.object_id
            ),
        });
    }
    if branch != source.branch {
        return Err(RecordError::Source {
            what,
            detail: format!("记录分支 `{branch}` 与构造结果 `{}` 不同", source.branch),
        });
    }
    let want_window = [
        i64::try_from(source.window[0]).map_err(|_| RecordError::Source {
            what,
            detail: format!("构造结果锚坐标 {} 超出 i64", source.window[0]),
        })?,
        i64::try_from(source.window[1]).map_err(|_| RecordError::Source {
            what,
            detail: format!("构造结果锚坐标 {} 超出 i64", source.window[1]),
        })?,
        i64::try_from(source.window[2]).map_err(|_| RecordError::Source {
            what,
            detail: format!("构造结果锚坐标 {} 超出 i64", source.window[2]),
        })?,
    ];
    if window != want_window {
        return Err(RecordError::Source {
            what,
            detail: format!("记录窗口 {window:?} 与构造结果 {want_window:?} 不同"),
        });
    }
    if first_known_generation != source.generation
        || published_generation != source.generation
        || first_known_cut != source.cut
    {
        return Err(RecordError::Source {
            what,
            detail: format!(
                "生成态三件套（{first_known_generation}/{first_known_cut}/{published_generation}）与本次构造（{}/{}/{}）不同",
                source.generation, source.cut, source.generation
            ),
        });
    }

    // `comparisons` 不是 §5.2 的获准 0x07 位置：这里要求真实数组根，文本视图形态显式拒绝。
    let comparisons = read_edge(store, record.view(), "comparisons", Kind::Array, what)?;
    let input_refs = read_edge(store, record.view(), "input_refs", Kind::Array, what)?;
    let source_coords = read_edge(store, record.view(), "source_coords", Kind::Array, what)?;
    // 公开 `input_refs` 是 producer 同次构造的独立边（原 generic 字符串源叶），不是生成态
    // 数值叶的别名；本模块只核有界外壳后借根（见 [`check_wire_input_refs`]）。
    let wire_input_refs = store.native_built_ref(&source.wire_input_refs)?;
    let wire_input_refs = check_wire_input_refs(store, wire_input_refs)?;
    let input_refs_json = read_edge(
        store,
        record.view(),
        "input_refs_json",
        Kind::JsonText,
        what,
    )?;
    let source_coords_json = read_edge(
        store,
        record.view(),
        "source_coords_json",
        Kind::JsonText,
        what,
    )?;
    for (name, edge, built) in [
        ("comparisons", &comparisons, &source.comparisons),
        ("input_refs", &input_refs, &source.input_refs),
        ("source_coords", &source_coords, &source.source_coords),
        ("input_refs_json", &input_refs_json, &source.input_refs_json),
        (
            "source_coords_json",
            &source_coords_json,
            &source.source_coords_json,
        ),
    ] {
        if edge.digest()? != built.digest || edge.kind() != built.kind {
            return Err(RecordError::Source {
                what,
                detail: format!("`{name}` 边与构造结果的根/类型不符"),
            });
        }
    }
    Ok(ShapeRecord {
        object_id,
        object_revision,
        kind,
        branch,
        dir_ab,
        dir_bc,
        window,
        comparisons,
        input_refs,
        source_coords,
        wire_input_refs,
        input_refs_json,
        source_coords_json,
        first_known_generation,
        first_known_cut,
        published_generation,
    })
}

/// 泛型生成态单条：十项旧字段；三条 `*_json` 用 producer 的 0x07 视图边。
fn typed_generated(
    store: &mut (impl CodecStore + ?Sized),
    record: &TypedRecord,
) -> Result<Built, RecordError> {
    use sc::ValueView as B;
    let revision = super::business_json::Decimal::new(record.object_revision);
    let first = super::business_json::Decimal::new(record.first_known_generation);
    let published = super::business_json::Decimal::new(record.published_generation);
    let fields = [
        ("object_id", B::String(&record.object_id)),
        (
            "object_revision",
            B::Number(sc::BorrowedNumberToken::parse(revision.text())?),
        ),
        ("kind", B::String(&record.kind)),
        ("fact_key", B::String(&record.fact_key)),
        ("payload_json", B::Edge(record.payload_json.digest()?)),
        ("input_refs_json", B::Edge(record.input_refs_json.digest()?)),
        (
            "source_coords_json",
            B::Edge(record.source_coords_json.digest()?),
        ),
        (
            "first_known_generation",
            B::Number(sc::BorrowedNumberToken::parse(first.text())?),
        ),
        ("first_known_cut", B::String(&record.first_known_cut)),
        (
            "published_generation",
            B::Number(sc::BorrowedNumberToken::parse(published.text())?),
        ),
    ];
    Ok(store.write_value_view(B::Object(&fields))?)
}

/// CC-006 生成态单条：十五项旧字段；`comparisons_json` 用固定小结构的规范文本，另两个 `*_json`
/// 用 producer 的 0x07 视图边。
fn shape_generated(
    store: &mut (impl CodecStore + ?Sized),
    record: &ShapeRecord,
) -> Result<Built, RecordError> {
    use sc::ValueView as B;
    let revision = super::business_json::Decimal::new(record.object_revision);
    let first = super::business_json::Decimal::new(record.first_known_generation);
    let published = super::business_json::Decimal::new(record.published_generation);
    let windows = record.window.map(super::business_json::Decimal::new);
    let comparisons = render_canonical_text_view(store, record.comparisons.view(), "comparisons")?;
    let fields = [
        ("object_id", B::String(&record.object_id)),
        (
            "object_revision",
            B::Number(sc::BorrowedNumberToken::parse(revision.text())?),
        ),
        ("kind", B::String(&record.kind)),
        ("branch", B::String(&record.branch)),
        (
            "window_start",
            B::Number(sc::BorrowedNumberToken::parse(windows[0].text())?),
        ),
        (
            "window_mid",
            B::Number(sc::BorrowedNumberToken::parse(windows[1].text())?),
        ),
        (
            "window_end",
            B::Number(sc::BorrowedNumberToken::parse(windows[2].text())?),
        ),
        ("dir_ab", B::String(&record.dir_ab)),
        ("dir_bc", B::String(&record.dir_bc)),
        ("comparisons_json", B::String(&comparisons)),
        ("input_refs_json", B::Edge(record.input_refs_json.digest()?)),
        (
            "source_coords_json",
            B::Edge(record.source_coords_json.digest()?),
        ),
        (
            "first_known_generation",
            B::Number(sc::BorrowedNumberToken::parse(first.text())?),
        ),
        ("first_known_cut", B::String(&record.first_known_cut)),
        (
            "published_generation",
            B::Number(sc::BorrowedNumberToken::parse(published.text())?),
        ),
    ];
    Ok(store.write_value_view(B::Object(&fields))?)
}

/// 泛型公开 wire 单条（十五字段）：大载荷三条边直接借根；批号与生命期来自 `life`。
fn typed_wire(
    store: &mut (impl CodecStore + ?Sized),
    record: &TypedRecord,
    life: LifeView<'_>,
) -> Result<Built, RecordError> {
    use sc::{capture_allocation::audit_json::JsonView as J, ValueView as B};
    let revision = super::business_json::Decimal::new(record.object_revision);
    let first = super::business_json::Decimal::new(life.first_known_generation);
    let published = super::business_json::Decimal::new(life.published_generation);
    let withdrawn = life
        .withdrawn_generation
        .map(super::business_json::Decimal::new);
    let fields = [
        ("object_id", B::String(&record.object_id)),
        ("object_revision", B::String(revision.text())),
        ("kind", B::String(&record.kind)),
        (
            "batch_id",
            match life.batch {
                BatchView::CurrentBatch => B::BatchSelf,
                BatchView::Existing(s) => B::String(s),
            },
        ),
        ("fact_key", B::String(&record.fact_key)),
        ("payload", B::Edge(record.payload.digest()?)),
        ("input_refs", B::Edge(record.input_refs.digest()?)),
        ("source_coords", B::Edge(record.source_coords.digest()?)),
        ("first_known_generation", B::String(first.text())),
        ("first_known_cut", B::String(life.first_known_cut)),
        ("published_generation", B::String(published.text())),
        (
            "withdrawn_generation",
            withdrawn
                .as_ref()
                .map(|v| B::String(v.text()))
                .unwrap_or(B::Json(J::Null)),
        ),
        (
            "withdrawal_reason",
            life.withdrawal_reason
                .map(B::String)
                .unwrap_or(B::Json(J::Null)),
        ),
        (
            "superseded_by",
            life.superseded_by
                .map(B::String)
                .unwrap_or(B::Json(J::Null)),
        ),
        (
            "lifecycle",
            B::String(if life.withdrawn_generation.is_some() {
                "withdrawn"
            } else {
                "active"
            }),
        ),
    ];
    Ok(store.write_template_view(B::Object(&fields))?)
}

/// CC-006 公开 wire 单条（二十字段）：`input_refs` 直接借 producer 的 `wire_input_refs`，
/// `comparisons` 与 `source_coords` 借生成态值根。
fn shape_wire(
    store: &mut (impl CodecStore + ?Sized),
    record: &ShapeRecord,
    life: LifeView<'_>,
) -> Result<Built, RecordError> {
    use sc::{capture_allocation::audit_json::JsonView as J, ValueView as B};
    let revision = super::business_json::Decimal::new(record.object_revision);
    let windows = record.window.map(super::business_json::Decimal::new);
    let first = super::business_json::Decimal::new(life.first_known_generation);
    let published = super::business_json::Decimal::new(life.published_generation);
    let withdrawn = life
        .withdrawn_generation
        .map(super::business_json::Decimal::new);
    let fields = [
        ("object_id", B::String(&record.object_id)),
        ("object_revision", B::String(revision.text())),
        ("kind", B::String(&record.kind)),
        (
            "batch_id",
            match life.batch {
                BatchView::CurrentBatch => B::BatchSelf,
                BatchView::Existing(s) => B::String(s),
            },
        ),
        ("branch", B::String(&record.branch)),
        ("dir_ab", B::String(&record.dir_ab)),
        ("dir_bc", B::String(&record.dir_bc)),
        ("window_start", B::String(windows[0].text())),
        ("window_mid", B::String(windows[1].text())),
        ("window_end", B::String(windows[2].text())),
        ("comparisons", B::Edge(record.comparisons.digest()?)),
        ("input_refs", B::Edge(record.wire_input_refs.digest()?)),
        ("source_coords", B::Edge(record.source_coords.digest()?)),
        ("first_known_generation", B::String(first.text())),
        ("first_known_cut", B::String(life.first_known_cut)),
        ("published_generation", B::String(published.text())),
        (
            "withdrawn_generation",
            withdrawn
                .as_ref()
                .map(|v| B::String(v.text()))
                .unwrap_or(B::Json(J::Null)),
        ),
        (
            "withdrawal_reason",
            life.withdrawal_reason
                .map(B::String)
                .unwrap_or(B::Json(J::Null)),
        ),
        (
            "superseded_by",
            life.superseded_by
                .map(B::String)
                .unwrap_or(B::Json(J::Null)),
        ),
        (
            "lifecycle",
            B::String(if life.withdrawn_generation.is_some() {
                "withdrawn"
            } else {
                "active"
            }),
        ),
    ];
    Ok(store.write_template_view(B::Object(&fields))?)
}

/// 公开面的生命期七项（文本/null + lifecycle），两型共用同一判据。
fn life_fields(life: &PublicationLife) -> Vec<(String, CValue)> {
    let withdrawn = life.withdrawn_generation.is_some();
    vec![
        (
            "first_known_generation".into(),
            text(life.first_known_generation),
        ),
        (
            "first_known_cut".into(),
            CValue::String(life.first_known_cut.clone()),
        ),
        (
            "published_generation".into(),
            text(life.published_generation),
        ),
        (
            "withdrawn_generation".into(),
            match life.withdrawn_generation {
                Some(generation) => text(generation),
                None => CValue::Null,
            },
        ),
        (
            "withdrawal_reason".into(),
            match &life.withdrawal_reason {
                Some(reason) => CValue::String(reason.clone()),
                None => CValue::Null,
            },
        ),
        (
            "superseded_by".into(),
            match &life.superseded_by {
                Some(id) => CValue::String(id.clone()),
                None => CValue::Null,
            },
        ),
        (
            "lifecycle".into(),
            CValue::String(if withdrawn { "withdrawn" } else { "active" }.to_string()),
        ),
    ]
}

/// 已持久出生 wire 的历史/当前生命周期投影。只更换小元数据，所有大字段继续借原值边。
/// 出生根与生成原件的完整绑定由对象索引核验；生命周期规则仍复用本模块唯一判据。
pub(super) fn reproject_stored_wire(
    store: &mut (impl CodecStore + ?Sized),
    birth: &ContentRef,
    life: &PublicationLife,
    current_generation: i64,
) -> Result<Built, RecordError> {
    reproject_stored_wire_view(store, birth.into(), life.view(), current_generation)
}
pub(crate) fn reproject_stored_wire_view(
    store: &mut (impl CodecStore + ?Sized),
    birth: sc::RefView<'_>,
    life: LifeView<'_>,
    current_generation: i64,
) -> Result<Built, RecordError> {
    use sc::{capture_allocation::audit_json::JsonView as J, ValueView as B};
    let what = "持久对象 wire";
    let id_ref = store.lookup_ref(birth, &[PathStep::Field("object_id".into())])?;
    let id = crate::v3::business_json::read_text(store, id_ref.view())
        .map_err(|detail| RecordError::Source { what, detail })?;
    let kind_ref = store.lookup_ref(birth, &[PathStep::Field("kind".into())])?;
    let kind = crate::v3::business_json::read_text(store, kind_ref.view())
        .map_err(|detail| RecordError::Source { what, detail })?;
    validate_life_view(life, current_generation, id.as_str())?;
    let names: &[&str] = if kind.as_str() == CC006_SHAPE_KIND {
        &SHAPE_WIRE_FIELDS
    } else {
        &TYPED_WIRE_FIELDS
    };
    let values = store.lookup_fields_ref(birth, names)?;
    let first = crate::v3::business_json::Decimal::new(life.first_known_generation);
    let published = crate::v3::business_json::Decimal::new(life.published_generation);
    let withdrawn = life
        .withdrawn_generation
        .map(crate::v3::business_json::Decimal::new);
    let lifecycle = [
        ("first_known_generation", B::String(first.text())),
        ("first_known_cut", B::String(life.first_known_cut)),
        ("published_generation", B::String(published.text())),
        (
            "withdrawn_generation",
            withdrawn
                .as_ref()
                .map(|v| B::String(v.text()))
                .unwrap_or(B::Json(J::Null)),
        ),
        (
            "withdrawal_reason",
            life.withdrawal_reason
                .map(B::String)
                .unwrap_or(B::Json(J::Null)),
        ),
        (
            "superseded_by",
            life.superseded_by
                .map(B::String)
                .unwrap_or(B::Json(J::Null)),
        ),
        (
            "lifecycle",
            B::String(if life.withdrawn_generation.is_some() {
                "withdrawn"
            } else {
                "active"
            }),
        ),
    ];
    let mut fields = [("", B::Json(J::Null)); 20];
    let mut n = 0;
    for name in names {
        if *name == "batch_id" || lifecycle.iter().any(|(k, _)| k == name) {
            continue;
        }
        fields[n] = (
            *name,
            B::Edge(
                values
                    .get(name)
                    .ok_or_else(|| RecordError::Source {
                        what,
                        detail: format!("缺字段{name}"),
                    })?
                    .digest()?,
            ),
        );
        n += 1;
    }
    fields[n] = (
        "batch_id",
        match life.batch {
            BatchView::CurrentBatch => B::BatchSelf,
            BatchView::Existing(s) => B::String(s),
        },
    );
    n += 1;
    for v in lifecycle {
        fields[n] = v;
        n += 1;
    }
    Ok(store.write_template_view(B::Object(&fields[..n]))?)
}

/// producer 同次构造的公开 `input_refs`（原 generic 字符串源叶 + 共享序列，`merged_index`
/// 字符串）：只核有界外壳后原样借根。
///
/// 判据（与旧 CC-006 合同同一条，不设宽严两档）：
///
/// - 数组必须恰是三组（旧 CC-006 合同要求完整三组来源）；
/// - 每组是恰含 `merged_index` / `raw_refs` / `dependency_refs` 的对象；
/// - `merged_index` 是规范十进制文本（旧 `project_input_refs` 的数值→文本结果）；
/// - `raw_refs` / `dependency_refs` 是数组，只核根、不展开子图（不读取子树内容）：叶级文本
///   形态是 producer 的构造义务，由同 oracle 对拍锁定（本模块不重渲染、不逐叶转换）。
fn check_wire_input_refs(
    store: &mut (impl CodecRead + ?Sized),
    reference: super::audit_store::RefValue,
) -> Result<super::audit_store::RefValue, RecordError> {
    let what = "CC-006 wire_input_refs";
    if reference.kind() != Kind::Array {
        return Err(RecordError::Source {
            what,
            detail: format!("公开来源根不是数组（{:?}）", reference.kind()),
        });
    }
    let groups = reference.extent().ok_or_else(|| RecordError::Source {
        what,
        detail: "公开来源根缺元素数".to_string(),
    })?;
    if groups != 3 {
        return Err(RecordError::Source {
            what,
            detail: format!("CC-006 必须恰有三组来源，实得 {groups}"),
        });
    }
    for index in 0..3u64 {
        let group = store.lookup_ref(reference.view(), &[PathStep::Index(index)])?;
        if group.kind() != Kind::Object {
            return Err(RecordError::Source {
                what,
                detail: format!("第 {index} 组不是对象（{:?}）", group.kind()),
            });
        }
        if group.extent() != Some(3) {
            return Err(RecordError::Source {
                what,
                detail: format!(
                    "第 {index} 组必须恰含 merged_index/raw_refs/dependency_refs 三键，实得 {:?}",
                    group.extent()
                ),
            });
        }
        let merged = resolve_field(store, group.view(), what, "merged_index")?;
        if merged.kind() != Kind::String {
            return Err(RecordError::Source {
                what,
                detail: format!(
                    "第 {index} 组 merged_index 必须是规范十进制文本（{:?}）",
                    merged.kind()
                ),
            });
        }
        read_edge(store, group.view(), "raw_refs", Kind::Array, what)?;
        read_edge(store, group.view(), "dependency_refs", Kind::Array, what)?;
    }
    Ok(reference)
}

/// [`PublicationLife`] 的逐条合法性判据（唯一一份；生成态不对生命期做任何推断）。
///
/// `current_generation` 是**调用方本批发布代号**（不是生成记录的代号；记录的三件套由构造面
/// 自核，见 [`read_typed_record`] / [`read_shape_record`]）。判据里的「当前代」一律指它：
///
/// 0. `current_generation >= 1`（发布代必须为正）；
/// 1. `first_known_generation >= 1`，且 `first_known_cut == "cut-{first}"`（与独立索引同一不变量）；
/// 2. `published_generation >= first` 且 `<= 当前代`；
/// 3. `batch`：`CurrentBatch` ⟺ `first == 当前代`（本批新生必须写 0x08，不得冒充旧批号）；
///    `Existing(id)` 必须是 `batch3-` + 64 位小写十六进制，且 `first < 当前代`（旧批次身份）；
/// 4. 撤回：有撤回代号时 `first < withdrawn <= 当前代` 且撤回理由必须存在、非空；
///    没有撤回代号时不得携带理由或替代者；
/// 5. `superseded_by` 非空且不得自指。
fn validate_life(
    life: &PublicationLife,
    current_generation: i64,
    object_id: &str,
) -> Result<(), RecordError> {
    validate_life_view(life.view(), current_generation, object_id)
}
pub(super) fn validate_life_view(
    life: LifeView<'_>,
    current_generation: i64,
    object_id: &str,
) -> Result<(), RecordError> {
    let what = "life";
    if current_generation < 1 {
        return Err(RecordError::Life {
            what,
            detail: format!("当前发布代 current_generation={current_generation} 必须为正"),
        });
    }
    let first = life.first_known_generation;
    if first < 1 {
        return Err(RecordError::Life {
            what,
            detail: format!("first_known_generation={first} 必须为正"),
        });
    }
    let cut_generation = super::business_json::Decimal::new(first);
    if life.first_known_cut.strip_prefix("cut-") != Some(cut_generation.text()) {
        return Err(RecordError::Life {
            what,
            detail: format!(
                "first_known_cut `{}` 与首获知代号不符（应为 `cut-{first}`）",
                life.first_known_cut
            ),
        });
    }
    if life.published_generation < first {
        return Err(RecordError::Life {
            what,
            detail: format!(
                "published_generation={} 早于 first_known_generation={first}",
                life.published_generation
            ),
        });
    }
    if life.published_generation > current_generation {
        return Err(RecordError::Life {
            what,
            detail: format!(
                "published_generation={} 晚于本次生成代 {current_generation}",
                life.published_generation
            ),
        });
    }
    match life.batch {
        BatchView::CurrentBatch => {
            if first != current_generation {
                return Err(RecordError::Life {
                    what,
                    detail: format!(
                        "本批占位只属本批新生：first_known_generation={first} ≠ 当前代 {current_generation}"
                    ),
                });
            }
        }
        BatchView::Existing(id) => {
            if !is_new_batch_id(id) {
                return Err(RecordError::Life {
                    what,
                    detail: format!(
                        "旧对象批号 `{id}` 不是 `{BATCH_ID_PREFIX}` + 64 位小写十六进制"
                    ),
                });
            }
            if first >= current_generation {
                return Err(RecordError::Life {
                    what,
                    detail: format!(
                        "本批新生（first_known_generation={first} ≥ 当前代 {current_generation}）不得写旧批号，必须用本批占位"
                    ),
                });
            }
        }
    }
    match life.withdrawn_generation {
        None => {
            if life.withdrawal_reason.is_some() || life.superseded_by.is_some() {
                return Err(RecordError::Life {
                    what,
                    detail: "未撤回却携带 withdrawal_reason/superseded_by".to_string(),
                });
            }
        }
        Some(withdrawn) => {
            if withdrawn <= first {
                return Err(RecordError::Life {
                    what,
                    detail: format!("withdrawn_generation={withdrawn} 不晚于首获知代 {first}"),
                });
            }
            if withdrawn > current_generation {
                return Err(RecordError::Life {
                    what,
                    detail: format!(
                        "withdrawn_generation={withdrawn} 晚于本次生成代 {current_generation}"
                    ),
                });
            }
            match life.withdrawal_reason {
                Some(reason) if !reason.is_empty() => {}
                Some(_) => {
                    return Err(RecordError::Life {
                        what,
                        detail: "撤回理由不得为空文本".to_string(),
                    });
                }
                None => {
                    return Err(RecordError::Life {
                        what,
                        detail: "撤回必须携带非空 withdrawal_reason".to_string(),
                    });
                }
            }
        }
    }
    if let Some(superseded) = life.superseded_by {
        if superseded.is_empty() {
            return Err(RecordError::Life {
                what,
                detail: "superseded_by 不得为空文本".to_string(),
            });
        }
        if superseded == object_id {
            return Err(RecordError::Life {
                what,
                detail: format!("superseded_by 不得自指（{object_id}）"),
            });
        }
    }
    Ok(())
}

/// §5.3 新域批号形状：`batch3-` + 64 位小写十六进制。
fn is_new_batch_id(text: &str) -> bool {
    let Some(digest) = text.strip_prefix(BATCH_ID_PREFIX) else {
        return false;
    };
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// 批号字段值：本批新生 = 0x08 空占位；旧对象 = 索引给的文本。
fn batch_value(batch: &BatchRef) -> CValue {
    match batch {
        BatchRef::CurrentBatch => CValue::BatchSelf,
        BatchRef::Existing(id) => CValue::String(id.clone()),
    }
}

/// `i64` → 生成态 `Number`（规范十进制 token；值域与旧行的整数列一致）。
fn number(value: i64) -> Result<CValue, RecordError> {
    Ok(CValue::Number(NumberToken::parse(&value.to_string())?))
}

/// `i64` → 公开面规范十进制文本。
fn text(value: i64) -> CValue {
    CValue::String(value.to_string())
}

fn field_path(name: &str) -> Vec<PathStep> {
    vec![PathStep::Field(name.to_string())]
}

fn index_path(index: u64) -> Vec<PathStep> {
    vec![PathStep::Index(index)]
}

/// 对象字段集 exact 核：字段数相等 + 每个声明键可定位。
///
/// 对象键唯一（§5.2）⟹ 计数相等且全键在场即无缺无多；本函数不展开任何子图。
fn expect_object_shape(
    store: &mut (impl CodecRead + ?Sized),
    root: sc::RefView<'_>,
    what: &'static str,
    fields: &[&str],
) -> Result<(), RecordError> {
    if root.kind() != Kind::Object {
        return Err(RecordError::Source {
            what,
            detail: format!("不是对象（{:?}）", root.kind()),
        });
    }
    match root.extent() {
        Some(count) if count == fields.len() as u64 => {}
        Some(count) => {
            return Err(RecordError::Source {
                what,
                detail: format!("字段数 {count} ≠ exact {}", fields.len()),
            })
        }
        None => {
            return Err(RecordError::Source {
                what,
                detail: "对象缺字段数".to_string(),
            })
        }
    }
    for name in fields {
        resolve_field(store, root, what, name)?;
    }
    Ok(())
}

/// 路径定位（只沿路径触碰节点）：缺字段/越界映射为 `Source`，其余编码错误原样上报。
fn resolve(
    store: &mut (impl CodecRead + ?Sized),
    root: &ContentRef,
    path: &[PathStep],
    what: &'static str,
    label: &str,
) -> Result<ContentRef, RecordError> {
    store
        .lookup_path(root, path)
        .map_err(|error| match error.code() {
            path_code::PATH_FIELD_MISSING => RecordError::Source {
                what,
                detail: format!("缺字段 `{label}`"),
            },
            path_code::PATH_INDEX_RANGE => RecordError::Source {
                what,
                detail: format!("`{label}` 下标越界"),
            },
            _ => RecordError::Codec(error),
        })
}

// #1488 renderer buffer owns its actual capacity until decoding/copy finishes.
enum RecordBuffer {
    Plain(Vec<u8>),
    Native(sc::capture_allocation::scratch::ScratchBytes),
    #[cfg(feature = "s_allocation_json_testonly")]
    Owned(sc::CodecVec<u8>),
}
impl RecordBuffer {
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Plain(v) => v,
            Self::Native(v) => v,
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(v) => v,
        }
    }
    fn extend(&mut self, bytes: &[u8]) -> Result<(), CodecError> {
        match self {
            Self::Plain(v) => {
                v.extend_from_slice(bytes);
                Ok(())
            }
            Self::Native(v) => v.try_extend_bytes(bytes),
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(v) => sc::capture_allocation::audit_json::extend_bytes(v, bytes),
        }
    }
}
fn record_bytes(
    store: &mut (impl CodecRead + ?Sized),
    root: sc::RefView<'_>,
) -> Result<RecordBuffer, RecordError> {
    let mut bytes = if let Some(work) = store.codec_work() {
        RecordBuffer::Native(work.bytes())
    } else {
        #[cfg(feature = "s_allocation_json_testonly")]
        if let Some(budget) = store.allocation_budget() {
            RecordBuffer::Owned(sc::capture_allocation::audit_json::owned_vec(&budget))
        } else {
            RecordBuffer::Plain(Vec::new())
        }
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        {
            RecordBuffer::Plain(Vec::new())
        }
    };
    struct Out<'a> {
        bytes: &'a mut RecordBuffer,
        error: Option<CodecError>,
    }
    impl std::io::Write for Out<'_> {
        fn write(&mut self, v: &[u8]) -> std::io::Result<usize> {
            match self.bytes.extend(v) {
                Ok(()) => Ok(v.len()),
                Err(e) => {
                    self.error = Some(e);
                    Err(std::io::ErrorKind::Other.into())
                }
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Out {
        bytes: &mut bytes,
        error: None,
    };
    let result = store.render_ref(root, &mut out);
    if let Some(error) = out.error {
        return Err(RecordError::Codec(error));
    }
    result?;
    Ok(bytes)
}
fn record_json(
    store: &mut (impl CodecRead + ?Sized),
    root: sc::RefView<'_>,
    what: &'static str,
    label: &str,
) -> Result<super::business_json::JsonOwner, RecordError> {
    let bytes = record_bytes(store, root)?;
    super::business_json::legacy_json_owner_admitted(
        bytes.bytes(),
        store.allocation_budget().as_ref(),
        store.codec_work(),
    )
    .map_err(|error| RecordError::Source {
        what,
        detail: format!("`{label}` 规范渲染不是合法 JSON：{error}"),
    })
}
fn render_canonical_text_view(
    store: &mut (impl CodecRead + ?Sized),
    root: sc::RefView<'_>,
    label: &str,
) -> Result<super::business_json::TextValue, RecordError> {
    let bytes = record_bytes(store, root)?;
    let text = std::str::from_utf8(bytes.bytes()).map_err(|error| RecordError::Source {
        what: "小固定结构",
        detail: format!("`{label}` 规范渲染不是 UTF-8：{error}"),
    })?;
    store
        .native_text(text)
        .map_err(|detail| RecordError::Source {
            what: "小固定结构",
            detail,
        })
}

fn resolve_field(
    store: &mut (impl CodecRead + ?Sized),
    root: sc::RefView<'_>,
    what: &'static str,
    label: &str,
) -> Result<super::audit_store::RefValue, RecordError> {
    store
        .lookup_field_ref(root, label)
        .map_err(|error| match error.code() {
            path_code::PATH_FIELD_MISSING => RecordError::Source {
                what,
                detail: format!("缺字段 `{label}`"),
            },
            path_code::PATH_INDEX_RANGE => RecordError::Source {
                what,
                detail: format!("`{label}` 下标越界"),
            },
            _ => RecordError::Codec(error),
        })
}

/// 字符串叶精确读：定位 → 类型核 → 只渲染该叶。
fn read_text(
    store: &mut (impl CodecRead + ?Sized),
    root: sc::RefView<'_>,
    name: &str,
    what: &'static str,
) -> Result<super::business_json::TextValue, RecordError> {
    let target = resolve_field(store, root, what, name)?;
    if target.kind() != Kind::String {
        return Err(RecordError::Source {
            what,
            detail: format!("字段 `{name}` 必须是字符串（{:?}）", target.kind()),
        });
    }
    let value = record_json(store, target.view(), what, name)?;
    let view = value.view();
    let text = view.as_str().ok_or_else(|| RecordError::Source {
        what,
        detail: format!("字段 `{name}` 渲染不是字符串"),
    })?;
    store
        .native_text(text)
        .map_err(|detail| RecordError::Source { what, detail })
}

/// 精确整数叶读取：只接受 `Number`（i64），拒绝浮点/布尔/null/文本。
///
/// 生成态与内部记录都是旧行的整数列；**数值→文本转换只发生在公开 wire 面**（[`text`]；
/// CC-006 的公开来源另由 producer 的 `wire_input_refs` 给出），不得在这里先行归一，
/// 否则生成态字节不再等于旧封存文本。
fn read_int(
    store: &mut (impl CodecRead + ?Sized),
    root: sc::RefView<'_>,
    name: &str,
    what: &'static str,
) -> Result<i64, RecordError> {
    let target = resolve_field(store, root, what, name)?;
    if target.kind() != Kind::Number {
        return Err(RecordError::Source {
            what,
            detail: format!(
                "字段 `{name}` 必须是精确整数（{:?}；浮点/布尔/null/文本一律拒绝）",
                target.kind()
            ),
        });
    }
    let value = record_json(store, target.view(), what, name)?;
    value.view().as_i64().ok_or_else(|| RecordError::Source {
        what,
        detail: format!("字段 `{name}` 不是 i64"),
    })
}

/// 边读：定位到目标值根，核 kind，返回其四字段（根文本/类型/extent 由真实节点给出）。
fn read_edge(
    store: &mut (impl CodecRead + ?Sized),
    root: sc::RefView<'_>,
    name: &str,
    expect: Kind,
    what: &'static str,
) -> Result<super::audit_store::RefValue, RecordError> {
    let target = resolve_field(store, root, what, name)?;
    if target.kind() != expect {
        return Err(RecordError::Source {
            what,
            detail: format!(
                "字段 `{name}` 必须是 {}（{:?}）",
                expect.name(),
                target.kind()
            ),
        });
    }
    Ok(target)
}

/// 小值规范渲染为 JSON 值（只对小字段/小固定结构；不用于大载荷）。
fn render_json(
    store: &mut (impl CodecRead + ?Sized),
    reference: &ContentRef,
    what: &'static str,
    label: &str,
) -> Result<crate::Value, RecordError> {
    let bytes = render_bytes(store, reference)?;
    serde_json::from_slice(&bytes).map_err(|error| RecordError::Source {
        what,
        detail: format!("`{label}` 规范渲染不是合法 JSON：{error}"),
    })
}

/// 小固定结构的规范文本（CC-006 `comparisons_json` 走这里；比较集是固定小结构）。
fn render_canonical_text(
    store: &mut dyn NodeStore,
    reference: &ContentRef,
    label: &str,
) -> Result<String, RecordError> {
    let bytes = render_bytes(store, reference)?;
    String::from_utf8(bytes).map_err(|error| RecordError::Source {
        what: "小固定结构",
        detail: format!("`{label}` 规范渲染不是 UTF-8：{error}"),
    })
}

fn render_bytes(
    store: &mut (impl CodecRead + ?Sized),
    reference: &ContentRef,
) -> Result<Vec<u8>, RecordError> {
    let mut out: Vec<u8> = Vec::new();
    store.render_canonical(reference, &mut out)?;
    Ok(out)
}

/// 事实键＝规范 JSON 文本 `[kind, slot]`：首项必须等于记录 `kind`，且文本逐字等于该值的
/// 规范渲染（不放行等价 JSON 写法）。
fn check_fact_key_kind(
    store: &mut (impl CodecRead + ?Sized),
    fact_key: &str,
    kind: &str,
    what: &'static str,
) -> Result<(), RecordError> {
    let parsed = super::business_json::legacy_json_owner_admitted(
        fact_key.as_bytes(),
        store.allocation_budget().as_ref(),
        store.codec_work(),
    )
    .map_err(|error| RecordError::Source {
        what,
        detail: format!("fact_key 不是 JSON：{error}"),
    })?;
    let view = parsed.view();
    let items = view.as_array().ok_or_else(|| RecordError::Source {
        what,
        detail: "fact_key 不是 `[kind, slot]`".into(),
    })?;
    if items.len() != 2 || items.get(0).and_then(|v| v.as_str()) != Some(kind) {
        return Err(RecordError::Source {
            what,
            detail: format!("fact_key `{fact_key}` 的首项不是 `{kind}` 或不是两项"),
        });
    }
    struct Matches<'a> {
        remaining: &'a [u8],
        equal: bool,
    }
    impl std::io::Write for Matches<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.equal {
                match self.remaining.strip_prefix(bytes) {
                    Some(rest) => self.remaining = rest,
                    None => self.equal = false,
                }
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut out = Matches {
        remaining: fact_key.as_bytes(),
        equal: true,
    };
    serde_json::to_writer(&mut out, &view).map_err(|error| RecordError::Source {
        what,
        detail: format!("fact_key 规范编码失败：{error}"),
    })?;
    if !out.equal || !out.remaining.is_empty() {
        return Err(RecordError::Source {
            what,
            detail: format!("fact_key `{fact_key}` 不是该值的规范文本"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content_producer::{Cc004Producer, RawRefSpec};
    use crate::{dir_str, project_object_wire, tb02a, tb02a_facts};
    use newchan_rust::s_content::path::replace_batch_placeholder;
    use newchan_rust::s_content::{
        render_canonical, verify, MemoryNodes, NodeDigest, NodeRead, NodeSink, NodeSource,
    };
    use newchan_rust::theta_v0::classifier::local_shape::{classify_local_shape, Cc006LocalShape};
    use newchan_rust::theta_v0::parser::inclusion::IncrFactsState;
    use newchan_rust::theta_v0::types::Bar;
    use serde_json::{json, Value};
    use std::collections::{BTreeMap, BTreeSet};

    /// 真实生产档案号（`tb02a` 的 OHLC 档案）：旧 `build_object_and_witnesses` 的
    /// `source_coords` 分支按它取值，夹具不得用测试专用档案号绕开生产分支。
    const PROFILE: &str = tb02a::PROFILE;
    const RULE: &str = "test-rule/1";
    const GENERATION: i64 = 3;
    const CUT: &str = "cut-3";
    /// 本批批号（新生 0x08 占位的解析目标）。
    const CURRENT_BATCH: &str =
        "batch3-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    /// 旧对象首版批号（独立索引给的文本）。
    const OLD_BATCH: &str =
        "batch3-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    /// 无同价极值竞争的规则折线（与 `content_producer` 的 `CASE_SHAPE` 同款）。
    const CASE_SHAPE: &[(usize, i64, i64)] = &[
        (10, 10, 5),
        (20, 20, 15),
        (30, 14, 7),
        (40, 25, 18),
        (50, 19, 10),
        (60, 30, 22),
        (70, 24, 14),
    ];

    /// 读自己刚写的节点，并记录**读到的根**（供「不展开大载荷」判据）。
    #[derive(Default)]
    struct Store {
        nodes: MemoryNodes,
        reads: Vec<NodeDigest>,
    }

    impl NodeSource for Store {
        fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
            self.reads.push(*digest);
            NodeSource::read(&mut self.nodes, digest)
        }
    }

    impl NodeSink for Store {
        fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
            NodeSink::put(&mut self.nodes, digest, tag, body)
        }
    }

    struct Fixture {
        store: Store,
        producer: Cc004Producer,
        state: IncrFactsState,
        raw: BTreeMap<i64, Value>,
    }

    /// 生产 raw 行同形：身份/版本/来源字段 + OHLC 文本列（CC-007 oracle 的
    /// `raw_bar`/`unit_content_key` 直接读这四列，缺了就不是真实旧配方输入）。
    fn raw_row(coord: i64, open: i64, high: i64, low: i64, close: i64) -> (Value, RawRefSpec) {
        let raw = json!({
            "identity_key": format!("id-{coord}"),
            "revision": 1,
            "input_revision": "1",
            "payload_hash": format!("ph-{coord}-1"),
            "receipt_id": "r1",
            "event_id": format!("ev-{coord}"),
            "seq": coord + 100,
            "source_coord": coord.to_string(),
            "open": open.to_string(),
            "high": high.to_string(),
            "low": low.to_string(),
            "close": close.to_string(),
        });
        let spec = RawRefSpec {
            identity_key: format!("id-{coord}"),
            payload_hash: format!("ph-{coord}-1"),
            receipt_id: "r1".to_string(),
            event_id: format!("ev-{coord}"),
            input_revision: "1".to_string(),
            revision: 1,
            seq: coord + 100,
            source_coord: coord.to_string(),
        };
        (raw, spec)
    }

    /// 同一同核 fixture：真实 `IncrFactsState`（真实 parser 包含合并）+ 真实源版本登记。
    fn fixture() -> Fixture {
        let mut store = Store::default();
        let mut producer = Cc004Producer::new(PROFILE, RULE);
        let mut state = IncrFactsState::empty();
        let mut raw = BTreeMap::new();
        for (coord, high, low) in CASE_SHAPE {
            state.append(Bar {
                source_index: *coord,
                timestamp: *coord as i64,
                open: *low,
                high: *high,
                low: *low,
                close: *high,
                volume: 1.0,
                untradable: false,
            });
            let (row, spec) = raw_row(*coord as i64, *low, *high, *low, *high);
            assert!(producer
                .bind_source(&mut store, *coord as i64, spec)
                .expect("登记源版本"));
            raw.insert(*coord as i64, row);
        }
        Fixture {
            store,
            producer,
            state,
            raw,
        }
    }

    fn rendered(store: &mut dyn NodeStore, built: &Built) -> Value {
        let mut out: Vec<u8> = Vec::new();
        render_canonical(&mut *store, &built.content_ref(), &mut out).expect("规范渲染");
        serde_json::from_slice(&out).expect("渲染是合法 JSON")
    }

    fn rendered_ref(store: &mut dyn NodeStore, reference: &ContentRef) -> Value {
        rendered(
            store,
            &Built {
                digest: reference.digest().expect("根"),
                kind: reference.kind,
                extent: reference.extent,
            },
        )
    }

    fn oracle_fact<'a>(oracle: &'a [Value], kind: &str, fact_key: &str) -> &'a Value {
        oracle
            .iter()
            .find(|candidate| {
                candidate["kind"] == kind && candidate["fact_key"].as_str() == Some(fact_key)
            })
            .unwrap_or_else(|| panic!("oracle 缺 {kind} 槽位 {fact_key}"))
    }

    fn new_life() -> PublicationLife {
        PublicationLife {
            first_known_generation: GENERATION,
            first_known_cut: CUT.to_string(),
            published_generation: GENERATION,
            withdrawn_generation: None,
            withdrawal_reason: None,
            superseded_by: None,
            batch: BatchRef::CurrentBatch,
        }
    }

    /// 新生 wire：0x08 占位必须在、未解析模板必须过不了公开核验；解析成本批后与旧公开投影
    /// 逐字段相等。
    fn assert_wire_matches_public_projection(
        fixture: &mut Fixture,
        object: BorrowedObject<'_>,
        life: &PublicationLife,
        current_generation: i64,
        want: &Value,
        batch: &str,
        label: &str,
    ) {
        let wire = wire_template(&mut fixture.store, object, life, current_generation)
            .expect("组装 wire 模板");
        let placeholder = lookup_path(
            &mut fixture.store,
            &wire.content_ref(),
            &field_path("batch_id"),
        )
        .expect("批号");
        if matches!(life.batch, BatchRef::CurrentBatch) {
            assert_eq!(
                placeholder.kind,
                Kind::TemplateBatch,
                "{label} 本批新生必须写 0x08 占位"
            );
            assert!(
                verify(&mut fixture.store, &wire.content_ref()).is_err(),
                "{label} 未解析的模板不得通过公开核验"
            );
        } else {
            assert_eq!(placeholder.kind, Kind::String, "{label} 旧对象批号是文本");
        }
        let resolved = replace_batch_placeholder(
            &mut fixture.store,
            &wire.content_ref(),
            &field_path("batch_id"),
            batch,
        )
        .expect("解析本批占位");
        verify(&mut fixture.store, &resolved).expect("解析后公开闭包可核");
        assert_eq!(
            rendered_ref(&mut fixture.store, &resolved),
            *want,
            "{label} wire 逐字段"
        );
    }

    /// 泛型三类断言的公共体：generated 逐字段 = 旧 sealed 单条；wire 逐字段 = 旧公开投影。
    fn assert_typed_pair(
        fixture: &mut Fixture,
        object: BorrowedObject<'_>,
        want: &Value,
        label: &str,
    ) {
        let generated = generated(&mut fixture.store, object).expect("组装生成态");
        let got = rendered(&mut fixture.store, &generated);
        let mut expect = want.clone();
        expect["object_id"] = got["object_id"].clone();
        assert_eq!(got, expect, "{label} 生成态逐字段");
        assert_eq!(
            got.as_object().map(|map| map.len()),
            Some(TYPED_SEALED_FIELDS.len()),
            "{label} 生成态字段数"
        );

        let mut row = want.clone();
        row["batch_id"] = json!(CURRENT_BATCH);
        let mut wire_expect = tb02a::project(&row).expect("旧公开投影");
        // 只规范 obj3 身份：旧公开投影带 `obj-`，其余字段逐项对拍。
        wire_expect["object_id"] = got["object_id"].clone();
        let life = new_life();
        assert_wire_matches_public_projection(
            fixture,
            object,
            &life,
            GENERATION,
            &wire_expect,
            CURRENT_BATCH,
            label,
        );
    }

    type Facts = newchan_rust::theta_v0::parser::inclusion::InclusionFacts;

    fn merged_raws(facts: &Facts) -> Vec<Vec<i64>> {
        facts
            .groups
            .iter()
            .map(|group| group.members.iter().map(|coord| *coord as i64).collect())
            .collect()
    }

    /// 旧生产内联配方的 CC-006 三组来源（`s_structure_session.rs` 原样：`merged_index` 与
    /// `raw_refs`/`dependency_refs` 的 `revision`/`seq` 全是数值，来自 `merged_raws[mi]`）。
    fn old_shape_refs(
        facts: &Facts,
        raw: &BTreeMap<i64, Value>,
        merged_raws: &[Vec<i64>],
        start: usize,
    ) -> Vec<Value> {
        let event_ref = |coord: i64| {
            let event = &raw[&coord];
            json!({
                "identity_key": event["identity_key"],
                "payload_hash": event["payload_hash"],
                "receipt_id": event["receipt_id"],
                "event_id": event["event_id"],
                "input_revision": event["input_revision"],
                "revision": event["revision"],
                "seq": event["seq"],
                "source_coord": event["source_coord"],
            })
        };
        (start..start + 3)
            .map(|merged_index| {
                let group = &facts.groups[merged_index];
                let raw_refs: Vec<Value> = merged_raws[merged_index]
                    .iter()
                    .filter(|coord| raw.contains_key(coord))
                    .map(|coord| event_ref(*coord))
                    .collect();
                let mut dependency_coords = group
                    .direction_evidence
                    .as_ref()
                    .map(|evidence| evidence.source_coords.to_vec())
                    .unwrap_or_default();
                if let Some(boundary) = &group.confirmation_evidence {
                    dependency_coords.extend(&boundary.source_coords);
                }
                dependency_coords.retain(|coord| !group.members.contains(coord));
                dependency_coords.sort_unstable();
                dependency_coords.dedup();
                let dependencies: Vec<Value> = dependency_coords
                    .iter()
                    .map(|coord| event_ref(*coord as i64))
                    .collect();
                json!({"merged_index": merged_index, "raw_refs": raw_refs, "dependency_refs": dependencies})
            })
            .collect()
    }

    /// 旧生成路径的 CC-006 单条（`build_object_and_witnesses`，同一个旧函数做 oracle）。
    fn old_shape_row(
        facts: &Facts,
        raw: &BTreeMap<i64, Value>,
        merged_raws: &[Vec<i64>],
        start: usize,
        input_refs: &[Value],
    ) -> Value {
        let shape = classify_local_shape(
            &facts.merged[start],
            &facts.merged[start + 1],
            &facts.merged[start + 2],
        );
        let Cc006LocalShape::Classified {
            branch,
            dir_ab,
            dir_bc,
            comparisons,
        } = shape
        else {
            panic!("测试窗口必须在可判域内：{shape:?}");
        };
        let comparisons_json = json!(comparisons
            .iter()
            .map(|comparison| json!({
                "axis": comparison.axis.as_str(),
                "pair": comparison.pair.as_str(),
                "prev": comparison.prev.to_string(),
                "cur": comparison.cur.to_string(),
                "strict_up": comparison.strict_up,
                "held": comparison.held,
            }))
            .collect::<Vec<_>>())
        .to_string();
        crate::build_object_and_witnesses(
            &facts.merged,
            raw,
            merged_raws,
            start,
            branch.as_str(),
            dir_str(dir_ab),
            dir_str(dir_bc),
            &comparisons_json,
            input_refs,
            PROFILE,
            GENERATION,
            CUT,
        )
        .0
    }

    /// 闭包核验实际读到的根集（用于证明装配没有展开大载荷子树）。
    fn closure_reads(store: &mut Store, reference: &ContentRef) -> BTreeSet<NodeDigest> {
        store.reads.clear();
        verify(&mut *store, reference).expect("闭包可核");
        store.reads.iter().copied().collect()
    }

    /// 泛型记录自身的字段根集：内容寻址按值去重，判「不展开大子图」时须把这些同值节点扣除。
    fn record_field_digests(store: &mut Store, record: &Built) -> BTreeSet<NodeDigest> {
        TYPED_RECORD_FIELDS
            .iter()
            .map(|name| {
                lookup_path(store, &record.content_ref(), &field_path(name))
                    .expect("记录字段")
                    .digest()
                    .expect("字段根")
            })
            .collect()
    }

    fn clone_typed(source: &TypedObject) -> TypedObject {
        TypedObject {
            object_id: source.object_id.clone(),
            identity: source.identity,
            fact_key: source.fact_key.clone(),
            record: source.record.clone(),
            payload: source.payload.clone(),
            input_refs: source.input_refs.clone(),
            source_coords: source.source_coords.clone(),
            payload_json: source.payload_json.clone(),
            input_refs_json: source.input_refs_json.clone(),
            source_coords_json: source.source_coords_json.clone(),
            generation: source.generation,
            cut: source.cut.clone(),
            request: source.request.clone(),
        }
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_wire_records_and_birth_reference_keep_permits_after_scope() {
        let mut fixture = fixture();
        let object = fixture
            .producer
            .build_step(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .unwrap()
            .unwrap();
        let life = new_life();
        let expected = wire_template(
            &mut fixture.store,
            BorrowedObject::Typed(&object),
            &life,
            GENERATION,
        )
        .unwrap();
        let capture = sc::AuditCapture::new(fixture.store.nodes);
        let budget = super::super::audit_owner::native::Budget::new(8 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let (record, reference) = budget
            .codec_work()
            .unwrap()
            .run(|scope| {
                let mut reader =
                    super::super::history::HashReader::captured_for_audit(&capture, None)
                        .map_err(|detail| RecordError::Source {
                            what: "fixture",
                            detail: detail.to_string(),
                        })?
                        .with_codec_work(scope, &budget);
                let record = read_typed_record(&mut reader, &TypedView::of_typed(&object), None)?;
                let wire = wire_template_view(
                    &mut reader,
                    BorrowedObject::Typed(&object),
                    life.view(),
                    GENERATION,
                )?;
                assert_eq!(wire.digest, expected.digest);
                let reference = reader.native_built_ref(&wire)?;
                Ok::<_, RecordError>((record, reference))
            })
            .unwrap();
        assert_eq!(record.object_id.as_str(), object.object_id);
        assert_eq!(reference.digest().unwrap(), expected.digest);
        assert!(budget.live() > baseline);
        drop(record);
        assert!(budget.live() > baseline, "出生引用仍持实际许可");
        drop(reference);
        assert_eq!(budget.live(), baseline);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_wire_bad_early_record_precedes_bad_late_life_and_releases_all_owners() {
        let mut fixture = fixture();
        let mut object = fixture
            .producer
            .build_step(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .unwrap()
            .unwrap();
        // An existing independent oracle expects record validation before life validation.
        object.record = sc::write_value(&mut fixture.store, &CValue::Object(Vec::new())).unwrap();
        let mut life = new_life();
        life.first_known_generation = -1;
        let expected = wire_template(
            &mut fixture.store,
            BorrowedObject::Typed(&object),
            &life,
            GENERATION,
        )
        .unwrap_err()
        .to_string();
        assert!(expected.contains("exact"));
        let capture = sc::AuditCapture::new(fixture.store.nodes);
        let budget = super::super::audit_owner::native::Budget::new(8 * 1024 * 1024, 0).unwrap();
        let baseline = budget.live();
        let result = budget.codec_work().unwrap().run(|scope| {
            let mut reader = super::super::history::HashReader::captured_for_audit(&capture, None)
                .map_err(|detail| RecordError::Source {
                    what: "fixture",
                    detail: detail.to_string(),
                })?
                .with_codec_work(scope, &budget);
            wire_template_view(
                &mut reader,
                BorrowedObject::Typed(&object),
                life.view(),
                GENERATION,
            )
        });
        match result.unwrap_err() {
            sc::capture_allocation::scratch::WorkError::Operation(error) => {
                assert_eq!(error.to_string(), expected)
            }
            other => panic!("预期记录语义首错，实得{other:?}"),
        }
        assert_eq!(budget.live(), baseline);
    }

    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_wire_valid_record_late_life_error_and_early_resource_keep_priority() {
        let mut fixture = fixture();
        let object = fixture
            .producer
            .build_step(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .unwrap()
            .unwrap();
        let mut life = new_life();
        life.first_known_generation = -1;
        let expected = wire_template(
            &mut fixture.store,
            BorrowedObject::Typed(&object),
            &life,
            GENERATION,
        )
        .unwrap_err()
        .to_string();
        assert!(expected.contains("first_known_generation"));
        let capture = sc::AuditCapture::new(fixture.store.nodes);
        let large = super::super::audit_owner::native::Budget::new(8 * 1024 * 1024, 0).unwrap();
        let baseline = large.live();
        let result = large.codec_work().unwrap().run(|scope| {
            let mut reader = super::super::history::HashReader::captured_for_audit(&capture, None)
                .map_err(|detail| RecordError::Source {
                    what: "fixture",
                    detail: detail.to_string(),
                })?
                .with_codec_work(scope, &large);
            wire_template_view(
                &mut reader,
                BorrowedObject::Typed(&object),
                life.view(),
                GENERATION,
            )
        });
        match result.unwrap_err() {
            sc::capture_allocation::scratch::WorkError::Operation(error) => {
                assert_eq!(error.to_string(), expected)
            }
            other => panic!("预期晚life语义首错，实得{other:?}"),
        }
        assert_eq!(large.live(), baseline);
        let small = super::super::audit_owner::native::Budget::new(
            baseline + sc::capture_allocation::scratch::CodecWork::metadata_bytes() + 8,
            0,
        )
        .unwrap();
        let small_baseline = small.live();
        let result = small.codec_work().unwrap().run(|scope| {
            let mut reader = super::super::history::HashReader::captured_for_audit(&capture, None)
                .map_err(|detail| RecordError::Source {
                    what: "fixture",
                    detail: detail.to_string(),
                })?
                .with_codec_work(scope, &small);
            wire_template_view(
                &mut reader,
                BorrowedObject::Typed(&object),
                life.view(),
                GENERATION,
            )
        });
        assert!(
            matches!(
                result,
                Err(sc::capture_allocation::scratch::WorkError::Resource(_))
            ),
            "根许可不足必须早于晚life错误"
        );
        assert_eq!(small.live(), small_baseline);
    }

    #[test]
    fn typed_generated_and_wire_match_sealed_row_and_public_projection() {
        let mut fixture = fixture();
        let facts = fixture.state.materialize_facts();
        let (oracle, _) = tb02a_facts::build(&facts, &fixture.raw, &[], GENERATION, CUT, 0);

        let mut steps = 0usize;
        for index in 0..fixture.state.step_count() {
            let object = fixture
                .producer
                .build_step(&mut fixture.store, &fixture.state, index, GENERATION, CUT)
                .expect("构造 CC-004")
                .expect("步骤索引在域内");
            let want = oracle_fact(&oracle, "CC-004.inclusion_step", &object.fact_key).clone();
            assert_typed_pair(
                &mut fixture,
                BorrowedObject::Typed(&object),
                &want,
                &format!("CC-004 步骤{index}"),
            );
            steps += 1;
        }
        assert!(steps > 0, "本用例需要至少一个步骤");

        let mut groups = 0usize;
        for index in 0..fixture.state.group_count() {
            let Some(object) = fixture
                .producer
                .build_group(&mut fixture.store, &fixture.state, index, GENERATION, CUT)
                .expect("构造 CC-005")
            else {
                continue;
            };
            let want = oracle_fact(&oracle, "CC-005.inclusion_group", &object.fact_key).clone();
            assert_typed_pair(
                &mut fixture,
                BorrowedObject::Typed(&object),
                &want,
                &format!("CC-005 组{index}"),
            );
            groups += 1;
        }
        assert!(groups > 0, "本用例需要至少一个组");
    }

    #[test]
    fn fractal_generated_and_wire_match_full_oracle_bytes() {
        let mut fixture = fixture();
        let facts = fixture.state.materialize_facts();
        let merged_raws = merged_raws(&facts);
        let raw_bars: Vec<Bar> = fixture.state.raw_iter().copied().collect();
        // 先用旧函数产出的行做 CC-007 oracle；只把身份规范到 obj3 域——CC-007 payload 的
        // `shape_object_id` 指向该 CC-006 对象（`tb02a_facts.rs:402`），身份换代后必须同域。
        let mut shapes: Vec<Value> = Vec::new();
        let mut shape_windows: Vec<(usize, Cc006Object)> = Vec::new();
        for start in 0..facts.groups.len().saturating_sub(2) {
            let Some(shape) = fixture
                .producer
                .build_shape(&mut fixture.store, &fixture.state, start, GENERATION, CUT)
                .expect("构造 CC-006")
            else {
                continue;
            };
            let mut row = old_shape_row(
                &facts,
                &fixture.raw,
                &merged_raws,
                start,
                &old_shape_refs(&facts, &fixture.raw, &merged_raws, start),
            );
            row["object_id"] = json!(shape.object_id);
            shapes.push(row);
            shape_windows.push((start, shape));
        }
        let (oracle, _) = tb02a_facts::build(&facts, &fixture.raw, &shapes, GENERATION, CUT, 0);

        let mut fractals = 0usize;
        for (start, shape) in &shape_windows {
            if !matches!(shape.branch, "TOP" | "BOTTOM") {
                continue;
            }
            let views = [*start, start + 1, start + 2].map(|index| {
                fixture
                    .state
                    .shared_group(index)
                    .expect("共享态可用")
                    .expect("组在域内")
            });
            let fractal = fixture
                .producer
                .build_fractal_raw(
                    &mut fixture.store,
                    &raw_bars,
                    shape,
                    [&views[0], &views[1], &views[2]],
                    GENERATION,
                    CUT,
                )
                .expect("构造 CC-007")
                .expect("TOP/BOTTOM 窗口应产对象");
            let want = oracle_fact(&oracle, CC007_FRACTAL_KIND, &fractal.fact_key).clone();
            assert_typed_pair(
                &mut fixture,
                BorrowedObject::Fractal(&fractal),
                &want,
                &format!("CC-007 窗口{start}"),
            );
            fractals += 1;
        }
        assert!(fractals > 0, "本用例需要至少一个可判的 TOP/BOTTOM 窗口");
    }

    #[test]
    fn shape_generated_and_wire_match_old_shape_oracle() {
        let mut fixture = fixture();
        let facts = fixture.state.materialize_facts();
        let merged_raws = merged_raws(&facts);
        let mut classified = 0usize;
        for start in 0..facts.groups.len().saturating_sub(2) {
            let Some(object) = fixture
                .producer
                .build_shape(&mut fixture.store, &fixture.state, start, GENERATION, CUT)
                .expect("构造 CC-006")
            else {
                continue;
            };
            let label = format!("CC-006 窗口{start}");
            let old_row = old_shape_row(
                &facts,
                &fixture.raw,
                &merged_raws,
                start,
                &old_shape_refs(&facts, &fixture.raw, &merged_raws, start),
            );

            // 生成态：与旧 sealed 单条（原 CC-006 内联配方，`merged_index`/`revision`/`seq`
            // 都是数值叶）逐字段相等；只规范 obj3 身份，其余字段一个不落。
            let generated =
                generated(&mut fixture.store, BorrowedObject::Shape(&object)).expect("生成态");
            let got = rendered(&mut fixture.store, &generated);
            let mut expect = old_row.clone();
            expect["object_id"] = got["object_id"].clone();
            assert_eq!(got, expect, "{label} 生成态逐字段");
            assert_eq!(
                got.as_object().map(|map| map.len()),
                Some(SHAPE_SEALED_FIELDS.len()),
                "{label} 生成态字段数"
            );

            // 公开 wire：与 `project_object_wire`（旧生产数值行的读出投影）逐字段相等；
            // 同样只规范 obj3 身份。
            let mut row = old_row.clone();
            row["batch_id"] = json!(CURRENT_BATCH);
            let mut wire_expect = project_object_wire(&row).expect("旧公开投影");
            let life = new_life();
            let wire = wire_template(
                &mut fixture.store,
                BorrowedObject::Shape(&object),
                &life,
                GENERATION,
            )
            .expect("组装 CC-006 wire");
            let placeholder = lookup_path(
                &mut fixture.store,
                &wire.content_ref(),
                &field_path("batch_id"),
            )
            .expect("批号");
            assert_eq!(placeholder.kind, Kind::TemplateBatch);
            let resolved = replace_batch_placeholder(
                &mut fixture.store,
                &wire.content_ref(),
                &field_path("batch_id"),
                CURRENT_BATCH,
            )
            .expect("解析本批占位");
            verify(&mut fixture.store, &resolved).expect("解析后公开闭包可核");
            let got_wire = rendered_ref(&mut fixture.store, &resolved);
            wire_expect["object_id"] = got_wire["object_id"].clone();
            assert_eq!(got_wire, wire_expect, "{label} wire 逐字段");
            assert_eq!(
                got_wire.as_object().map(|map| map.len()),
                Some(SHAPE_WIRE_FIELDS.len()),
                "{label} wire 字段数"
            );
            classified += 1;
        }
        assert!(classified >= 3, "本用例需要至少三个可判窗口");
    }

    #[test]
    fn new_birth_placeholder_old_batch_and_withdrawn_lifecycle_cover_public_fields() {
        let mut fixture = fixture();
        let object = fixture
            .producer
            .build_step(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .expect("构造 CC-004")
            .expect("步骤索引在域内");

        // 1) 本批新生：0x08 占位。
        let born = new_life();
        let wire = wire_template(
            &mut fixture.store,
            BorrowedObject::Typed(&object),
            &born,
            GENERATION,
        )
        .expect("新生");
        assert_eq!(
            lookup_path(
                &mut fixture.store,
                &wire.content_ref(),
                &field_path("batch_id")
            )
            .expect("批号")
            .kind,
            Kind::TemplateBatch
        );

        // 2) 旧对象：保留首版批号文本，first_known 取索引给的旧代；本批发布代仍是本次生成代
        //    （新生产物在本代发布，故两者一致）。
        let old = PublicationLife {
            first_known_generation: 2,
            first_known_cut: "cut-2".to_string(),
            published_generation: GENERATION,
            withdrawn_generation: None,
            withdrawal_reason: None,
            superseded_by: None,
            batch: BatchRef::Existing(OLD_BATCH.to_string()),
        };
        let wire = wire_template(
            &mut fixture.store,
            BorrowedObject::Typed(&object),
            &old,
            GENERATION,
        )
        .expect("旧对象");
        verify(&mut fixture.store, &wire.content_ref()).expect("无占位的旧对象可直接公开核验");
        let rendered_wire = rendered(&mut fixture.store, &wire);
        assert_eq!(rendered_wire["batch_id"], json!(OLD_BATCH));
        assert_eq!(rendered_wire["first_known_generation"], json!("2"));
        assert_eq!(rendered_wire["first_known_cut"], json!("cut-2"));
        assert_eq!(rendered_wire["published_generation"], json!("3"));
        assert_eq!(rendered_wire["withdrawn_generation"], Value::Null);
        assert_eq!(rendered_wire["lifecycle"], json!("active"));

        // 生成态与 wire 的首获知是两件事：生成态必仍是本次生成代。
        let generated =
            generated(&mut fixture.store, BorrowedObject::Typed(&object)).expect("生成态");
        let rendered_generated = rendered(&mut fixture.store, &generated);
        assert_eq!(
            rendered_generated["first_known_generation"],
            json!(GENERATION)
        );

        // 3) 撤回：代号/理由/替代者进入公开字段，lifecycle=withdrawn。
        let withdrawn = PublicationLife {
            first_known_generation: 2,
            first_known_cut: "cut-2".to_string(),
            published_generation: GENERATION,
            withdrawn_generation: Some(GENERATION),
            withdrawal_reason: Some("fact_removed".to_string()),
            superseded_by: Some(format!("obj3-{}", "c".repeat(64))),
            batch: BatchRef::Existing(OLD_BATCH.to_string()),
        };
        let wire = wire_template(
            &mut fixture.store,
            BorrowedObject::Typed(&object),
            &withdrawn,
            GENERATION,
        )
        .expect("撤回记录");
        let rendered_wire = rendered(&mut fixture.store, &wire);
        assert_eq!(rendered_wire["withdrawn_generation"], json!("3"));
        assert_eq!(rendered_wire["withdrawal_reason"], json!("fact_removed"));
        assert_eq!(
            rendered_wire["superseded_by"],
            json!(format!("obj3-{}", "c".repeat(64)))
        );
        assert_eq!(rendered_wire["lifecycle"], json!("withdrawn"));
    }

    /// 旧记录**直接**在当前代发布：原生成记录（generation=1）不重造，只用调用方给的
    /// `current_generation=2` 组装当前代快照；活动与撤回两种生命期都可构造，源记录/身份不变。
    #[test]
    fn old_record_republishes_at_current_generation_without_rebuilding_the_source() {
        let mut fixture = fixture();
        let record_generation = 1;
        let record_cut = "cut-1";
        let object = fixture
            .producer
            .build_step(
                &mut fixture.store,
                &fixture.state,
                0,
                record_generation,
                record_cut,
            )
            .expect("构造 CC-004")
            .expect("步骤索引在域内");
        let record_digest = object.record.digest;
        let object_id = object.object_id.clone();

        // 原生成记录的三件套仍是它自己那一代：不因本次发布被改写。
        let generated_record =
            generated(&mut fixture.store, BorrowedObject::Typed(&object)).expect("生成态");
        let rendered_generated = rendered(&mut fixture.store, &generated_record);
        assert_eq!(rendered_generated["first_known_generation"], json!(1));
        assert_eq!(rendered_generated["first_known_cut"], json!("cut-1"));
        assert_eq!(rendered_generated["published_generation"], json!(1));

        let current_generation = 2;
        let active = PublicationLife {
            first_known_generation: 1,
            first_known_cut: record_cut.to_string(),
            published_generation: current_generation,
            withdrawn_generation: None,
            withdrawal_reason: None,
            superseded_by: None,
            batch: BatchRef::Existing(OLD_BATCH.to_string()),
        };
        let wire = wire_template(
            &mut fixture.store,
            BorrowedObject::Typed(&object),
            &active,
            current_generation,
        )
        .expect("旧记录可在当前代发布");
        verify(&mut fixture.store, &wire.content_ref()).expect("活动公开闭包可核");
        let rendered_wire = rendered(&mut fixture.store, &wire);
        assert_eq!(rendered_wire["object_id"], json!(object_id));
        assert_eq!(rendered_wire["batch_id"], json!(OLD_BATCH));
        assert_eq!(rendered_wire["first_known_generation"], json!("1"));
        assert_eq!(rendered_wire["published_generation"], json!("2"));
        assert_eq!(rendered_wire["withdrawn_generation"], Value::Null);
        assert_eq!(rendered_wire["lifecycle"], json!("active"));

        // 撤回：撤回代号也可以落在当前代；源记录仍不动。
        let withdrawn = PublicationLife {
            withdrawn_generation: Some(current_generation),
            withdrawal_reason: Some("fact_removed".to_string()),
            ..active.clone()
        };
        let wire = wire_template(
            &mut fixture.store,
            BorrowedObject::Typed(&object),
            &withdrawn,
            current_generation,
        )
        .expect("旧记录可在当前代撤回");
        let rendered_wire = rendered(&mut fixture.store, &wire);
        assert_eq!(rendered_wire["withdrawn_generation"], json!("2"));
        assert_eq!(rendered_wire["lifecycle"], json!("withdrawn"));

        // 源记录未被重造：根、身份与生成态三件套都保持原样。
        assert_eq!(object.record.digest, record_digest);
        assert_eq!(object.object_id, object_id);
        let generated_record =
            generated(&mut fixture.store, BorrowedObject::Typed(&object)).expect("生成态");
        let rendered_generated = rendered(&mut fixture.store, &generated_record);
        assert_eq!(rendered_generated["published_generation"], json!(1));

        // 反向判据不因跨代放松：本批新生（first == 当前代）不得用旧批号冒充。
        let error = wire_template(
            &mut fixture.store,
            BorrowedObject::Typed(&object),
            &PublicationLife {
                first_known_generation: current_generation,
                first_known_cut: "cut-2".to_string(),
                published_generation: current_generation,
                ..active.clone()
            },
            current_generation,
        )
        .expect_err("本批新生不得用旧批号");
        assert!(matches!(error, RecordError::Life { .. }), "{error}");
    }

    #[test]
    fn bad_lifecycles_are_rejected_with_one_rule_set() {
        let mut fixture = fixture();
        let object = fixture
            .producer
            .build_step(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .expect("构造 CC-004")
            .expect("步骤索引在域内");
        let good = new_life();
        let cases: Vec<(&str, PublicationLife)> = vec![
            (
                "first=0",
                PublicationLife {
                    first_known_generation: 0,
                    ..good.clone()
                },
            ),
            (
                "cut 与首获知代不符",
                PublicationLife {
                    first_known_cut: "cut-2".to_string(),
                    ..good.clone()
                },
            ),
            (
                "published < first",
                PublicationLife {
                    first_known_generation: 2,
                    first_known_cut: "cut-2".to_string(),
                    published_generation: 1,
                    batch: BatchRef::Existing(OLD_BATCH.to_string()),
                    ..good.clone()
                },
            ),
            (
                "published > 当前代",
                PublicationLife {
                    published_generation: GENERATION + 1,
                    ..good.clone()
                },
            ),
            (
                "本批新生用旧批号",
                PublicationLife {
                    first_known_generation: GENERATION,
                    first_known_cut: CUT.to_string(),
                    batch: BatchRef::Existing(OLD_BATCH.to_string()),
                    ..good.clone()
                },
            ),
            (
                "旧对象用本批占位",
                PublicationLife {
                    first_known_generation: 2,
                    first_known_cut: "cut-2".to_string(),
                    batch: BatchRef::CurrentBatch,
                    ..good.clone()
                },
            ),
            (
                "旧批号形状非法",
                PublicationLife {
                    first_known_generation: 2,
                    first_known_cut: "cut-2".to_string(),
                    batch: BatchRef::Existing("batch-".to_string() + &"b".repeat(64)),
                    ..good.clone()
                },
            ),
            (
                "withdrawn <= first",
                PublicationLife {
                    first_known_generation: 2,
                    first_known_cut: "cut-2".to_string(),
                    withdrawn_generation: Some(2),
                    withdrawal_reason: Some("fact_removed".to_string()),
                    ..good.clone()
                },
            ),
            (
                "withdrawn > 当前代",
                PublicationLife {
                    withdrawn_generation: Some(GENERATION + 1),
                    withdrawal_reason: Some("fact_removed".to_string()),
                    ..good.clone()
                },
            ),
            (
                "撤回缺理由",
                PublicationLife {
                    withdrawn_generation: Some(GENERATION),
                    ..good.clone()
                },
            ),
            (
                "活动却带理由",
                PublicationLife {
                    withdrawal_reason: Some("fact_removed".to_string()),
                    ..good.clone()
                },
            ),
            (
                "活动却带替代者",
                PublicationLife {
                    superseded_by: Some(format!("obj3-{}", "c".repeat(64))),
                    ..good.clone()
                },
            ),
            (
                "superseded_by 自指",
                PublicationLife {
                    withdrawn_generation: Some(GENERATION),
                    withdrawal_reason: Some("superseded_by_revision".to_string()),
                    superseded_by: Some(object.object_id.clone()),
                    ..good.clone()
                },
            ),
        ];
        for (label, life) in cases {
            let error = wire_template(
                &mut fixture.store,
                BorrowedObject::Typed(&object),
                &life,
                GENERATION,
            )
            .expect_err(label);
            assert!(
                matches!(error, RecordError::Life { .. }),
                "{label} 必须以 Life 拒绝，实得 {error}"
            );
        }
        // 当前发布代必须为正；它与生成记录自身的代号是两件事（记录代号已由构造面自核）。
        let error = wire_template(&mut fixture.store, BorrowedObject::Typed(&object), &good, 0)
            .expect_err("当前发布代 0 必须拒绝");
        assert!(
            matches!(error, RecordError::Life { .. }),
            "当前发布代 0 必须以 Life 拒绝，实得 {error}"
        );
        // 合法基线仍必须通过（拒绝面不是把整条路封死）。
        wire_template(
            &mut fixture.store,
            BorrowedObject::Typed(&object),
            &good,
            GENERATION,
        )
        .expect("合法生命期");
    }

    #[test]
    fn missing_extra_and_cross_kind_records_are_rejected() {
        let mut fixture = fixture();
        let step = fixture
            .producer
            .build_step(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .expect("构造 CC-004")
            .expect("步骤索引在域内");
        let shape = fixture
            .producer
            .build_shape(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .expect("构造 CC-006")
            .expect("窗口应可判");

        // 缺字段：抽掉 `fact_key` 的记忆记录（12 项）。
        let minus_fact_key = CValue::Object(
            TYPED_RECORD_FIELDS
                .iter()
                .filter(|name| **name != "fact_key")
                .map(|name| {
                    (
                        (*name).to_string(),
                        match *name {
                            "object_id" => CValue::String(step.object_id.clone()),
                            "kind" => CValue::String("CC-004.inclusion_step".to_string()),
                            "first_known_cut" => CValue::String(CUT.to_string()),
                            "payload" => CValue::Edge(step.payload.digest),
                            "input_refs" => CValue::Edge(step.input_refs.digest),
                            "source_coords" => CValue::Edge(step.source_coords.digest),
                            "payload_json" => CValue::Edge(step.payload_json.digest),
                            "input_refs_json" => CValue::Edge(step.input_refs_json.digest),
                            "source_coords_json" => CValue::Edge(step.source_coords_json.digest),
                            "object_revision"
                            | "first_known_generation"
                            | "published_generation" => number(1).unwrap(),
                            other => panic!("未覆盖字段 {other}"),
                        },
                    )
                })
                .collect(),
        );
        let broken = sc::write_value(&mut fixture.store, &minus_fact_key).expect("写坏记录");
        let tampered = TypedObject {
            record: broken,
            ..clone_typed(&step)
        };
        let error = generated(&mut fixture.store, BorrowedObject::Typed(&tampered))
            .expect_err("缺字段必须拒绝");
        assert!(matches!(error, RecordError::Source { .. }), "{error}");

        // 跨 kind：把 CC-006 的 17 字段记录塞进泛型面（字段数/键集不符）。
        let cross = TypedObject {
            record: shape.record.clone(),
            ..clone_typed(&step)
        };
        let error = generated(&mut fixture.store, BorrowedObject::Typed(&cross))
            .expect_err("形态记录不得走泛型面");
        assert!(matches!(error, RecordError::Source { .. }), "{error}");

        // 跨 kind：Fractal 入口拿到非 CC-007 的记录（用 CC-004 记录顶替）。
        let raw_bars: Vec<Bar> = fixture.state.raw_iter().copied().collect();
        let views = [0usize, 1, 2].map(|index| {
            fixture
                .state
                .shared_group(index)
                .expect("共享态可用")
                .expect("组在域内")
        });
        let fractal = fixture
            .producer
            .build_fractal_raw(
                &mut fixture.store,
                &raw_bars,
                &shape,
                [&views[0], &views[1], &views[2]],
                GENERATION,
                CUT,
            )
            .expect("构造 CC-007")
            .expect("该窗口应产对象");
        let crossed = Cc007Object {
            record: step.record.clone(),
            fact_key: step.fact_key.clone(),
            object_id: step.object_id.clone(),
            ..fractal
        };
        let error = generated(&mut fixture.store, BorrowedObject::Fractal(&crossed))
            .expect_err("Fractal 不得接受非 CC-007 kind");
        assert!(
            matches!(error, RecordError::Source { .. } | RecordError::Kind { .. }),
            "{error}"
        );
    }

    /// 共享大载荷不展开：装配的读集不得触碰载荷闭包的**真子图**；wire 借的就是生成态同根。
    #[test]
    fn assembly_borrows_large_roots_without_expanding_payload_subgraph() {
        let mut fixture = fixture();
        let object = fixture
            .producer
            .build_step(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .expect("构造 CC-004")
            .expect("步骤索引在域内");

        let payload_closure = closure_reads(&mut fixture.store, &object.payload.content_ref());
        let payload_root = object.payload.digest;
        let payload_inner: BTreeSet<NodeDigest> = payload_closure
            .iter()
            .copied()
            .filter(|digest| *digest != payload_root)
            .collect();
        // 内容寻址按值去重：记录自身的字段根可能与载荷子图里的同值节点共用一个 digest
        // （本夹具里 `source_coords = ["10"]` 与 `high_sources` 同值即一例）。判据是
        // 「除记录自身的字段节点外，载荷子图一个不读」，不是「与整个载荷闭包无交集」。
        let mut allowed: BTreeSet<NodeDigest> =
            record_field_digests(&mut fixture.store, &object.record);
        allowed.insert(object.record.digest);
        let payload_beyond_record: BTreeSet<NodeDigest> =
            payload_inner.difference(&allowed).copied().collect();

        // 生成态：只借 0x07 视图与标量边，不展开载荷子图。
        fixture.store.reads.clear();
        let generated =
            generated(&mut fixture.store, BorrowedObject::Typed(&object)).expect("生成态");
        let generated_reads: BTreeSet<NodeDigest> = fixture.store.reads.iter().copied().collect();
        assert!(
            generated_reads.is_disjoint(&payload_beyond_record),
            "生成态装配不得读取载荷子图（记录自身字段根除外）"
        );
        assert!(
            payload_beyond_record.len() > 8,
            "夹具的载荷子图必须真的够大，否则本判据没有意义（实得 {}）",
            payload_beyond_record.len()
        );

        // 公开 wire：三条大根保持同根，且同样不展开。
        fixture.store.reads.clear();
        let wire = wire_template(
            &mut fixture.store,
            BorrowedObject::Typed(&object),
            &new_life(),
            GENERATION,
        )
        .expect("wire");
        let wire_reads: BTreeSet<NodeDigest> = fixture.store.reads.iter().copied().collect();
        assert!(
            wire_reads.is_disjoint(&payload_beyond_record),
            "wire 装配不得读取载荷子图（记录自身字段根除外）"
        );
        let resolved = replace_batch_placeholder(
            &mut fixture.store,
            &wire.content_ref(),
            &field_path("batch_id"),
            CURRENT_BATCH,
        )
        .expect("解析本批占位");
        for (name, built) in [
            ("payload", &object.payload),
            ("input_refs", &object.input_refs),
            ("source_coords", &object.source_coords),
        ] {
            let edge =
                lookup_path(&mut fixture.store, &resolved, &field_path(name)).expect("wire 边");
            assert_eq!(
                edge.digest().unwrap(),
                built.digest,
                "wire `{name}` 必须借同根"
            );
        }
        let _ = generated;
    }

    /// CC-006 两种编码各走自己的边：生成态 `input_refs` 是原配方数值叶，公开 wire 直接消费
    /// producer 同次构造的 `wire_input_refs`（文本叶），装配不展开其子图。
    #[test]
    fn shape_wire_consumes_producer_wire_input_refs_without_expanding_sources() {
        let mut fixture = fixture();
        let shape = fixture
            .producer
            .build_shape(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .expect("构造 CC-006")
            .expect("窗口应可判");
        let generated_ref = shape.input_refs.content_ref();
        let public_ref = shape.wire_input_refs.content_ref();
        assert!(
            generated_ref.digest().expect("生成态根") != public_ref.digest().expect("公开根"),
            "公开根不得复用生成态数值叶根"
        );
        fixture.store.reads.clear();
        let wire = wire_template(
            &mut fixture.store,
            BorrowedObject::Shape(&shape),
            &new_life(),
            GENERATION,
        )
        .expect("组装 CC-006 wire");
        let assembly_reads: BTreeSet<NodeDigest> = fixture.store.reads.iter().copied().collect();
        let input_refs_ref = lookup_path(
            &mut fixture.store,
            &wire.content_ref(),
            &field_path("input_refs"),
        )
        .expect("wire input_refs");
        assert_eq!(
            input_refs_ref.digest().expect("wire 根"),
            public_ref.digest().expect("producer 公开根"),
            "wire `input_refs` 必须借 producer 的 wire_input_refs 同根"
        );
        assert_eq!(input_refs_ref.kind, Kind::Array);
        assert_eq!(input_refs_ref.extent, Some(3));
        // 装配允许触碰的只有公开 wrapper：三组对象、每组 merged_index 与两个 refs 根。
        let mut wrapper: BTreeSet<NodeDigest> = BTreeSet::new();
        for index in 0..3u64 {
            let group = lookup_path(&mut fixture.store, &input_refs_ref, &index_path(index))
                .expect("wire 组");
            wrapper.insert(group.digest().expect("组根"));
            for name in ["merged_index", "raw_refs", "dependency_refs"] {
                wrapper.insert(
                    lookup_path(&mut fixture.store, &group, &field_path(name))
                        .expect("wrapper 字段")
                        .digest()
                        .expect("wrapper 根"),
                );
            }
        }
        for index in 0..3u64 {
            let generated_group =
                lookup_path(&mut fixture.store, &generated_ref, &index_path(index))
                    .expect("生成组");
            let group = lookup_path(&mut fixture.store, &input_refs_ref, &index_path(index))
                .expect("wire 组");
            // merged_index：生成态是数值（小固定元数据），wire 是规范十进制文本。
            let merged = lookup_path(&mut fixture.store, &group, &field_path("merged_index"))
                .expect("merged_index");
            assert_eq!(merged.kind, Kind::String, "wire merged_index 必须是文本");
            let generated_merged = lookup_path(
                &mut fixture.store,
                &generated_group,
                &field_path("merged_index"),
            )
            .expect("生成态 merged_index");
            assert_eq!(
                generated_merged.kind,
                Kind::Number,
                "生成态 merged_index 仍是数值（禁粗暴复用为 wire）"
            );
            // raw_refs / dependency_refs：同一内容两种已声明编码，根各自独立（数值叶 ↔ 文本叶）。
            for name in ["raw_refs", "dependency_refs"] {
                let public =
                    lookup_path(&mut fixture.store, &group, &field_path(name)).expect("借用根");
                let generated =
                    lookup_path(&mut fixture.store, &generated_group, &field_path(name))
                        .expect("生成态根");
                if public.extent == Some(0) {
                    // 空数组没有可转换的叶：两种编码此时本就是同一根。
                    assert_eq!(
                        public.digest().unwrap(),
                        generated.digest().unwrap(),
                        "空的 `{name}` 两编码同根"
                    );
                } else {
                    assert_ne!(
                        public.digest().unwrap(),
                        generated.digest().unwrap(),
                        "wire `{name}` 不得复用生成态数值叶根"
                    );
                }
                if index == 0 && name == "raw_refs" {
                    let public_closure = closure_reads(&mut fixture.store, &public);
                    let inner: BTreeSet<NodeDigest> = public_closure
                        .iter()
                        .copied()
                        .filter(|digest| *digest != public.digest().unwrap())
                        .collect();
                    // 判据只针对**逻辑值节点**（0x00..0x08）：内容寻址下 wrapper 自己引用的同值
                    // 节点会落进 refs 子图，且路径导航/序列核验本身必然触碰结构节点
                    //（0x10 字节块、0x11..0x13 序列、0x14 字段对）——它们不构成「展开叶值」。
                    // 真正要挡的是读元素对象与其字符串/数字叶。
                    let leaked: BTreeSet<NodeDigest> = inner
                        .iter()
                        .copied()
                        .filter(|digest| !wrapper.contains(digest))
                        .filter(|digest| {
                            matches!(
                                NodeSource::read(&mut fixture.store, digest),
                                Ok(NodeRead::Found { tag, .. }) if tag <= 0x08
                            )
                        })
                        .collect();
                    assert!(
                        !leaked.is_empty(),
                        "夹具的源引用逻辑值必须非空，否则本判据没有意义"
                    );
                    assert!(
                        assembly_reads.is_disjoint(&leaked),
                        "装配不得读取公开源引用子图的逻辑值节点（wrapper 同值节点除外）：实际读到 {:?}",
                        assembly_reads.intersection(&leaked).collect::<Vec<_>>()
                    );
                }
            }
        }
    }

    /// 逐字相等是**生成态**的义务：这里把声明的 exact 字段集与真实产出对拍，防止日后有人加字段
    /// 却忘了同步本模块（生成态与公开 wire 各两组）。
    #[test]
    fn produced_records_match_declared_exact_field_sets() {
        let mut fixture = fixture();
        let step = fixture
            .producer
            .build_step(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .expect("构造 CC-004")
            .expect("步骤索引在域内");
        let shape = fixture
            .producer
            .build_shape(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .expect("构造 CC-006")
            .expect("窗口应可判");

        let typed_generated_row =
            generated(&mut fixture.store, BorrowedObject::Typed(&step)).expect("生成态");
        let keys: BTreeSet<String> = rendered(&mut fixture.store, &typed_generated_row)
            .as_object()
            .expect("对象")
            .keys()
            .cloned()
            .collect();
        assert_eq!(
            keys,
            TYPED_SEALED_FIELDS
                .iter()
                .map(|name| (*name).to_string())
                .collect::<BTreeSet<_>>()
        );
        let shape_generated_row =
            generated(&mut fixture.store, BorrowedObject::Shape(&shape)).expect("生成态");
        let keys: BTreeSet<String> = rendered(&mut fixture.store, &shape_generated_row)
            .as_object()
            .expect("对象")
            .keys()
            .cloned()
            .collect();
        assert_eq!(
            keys,
            SHAPE_SEALED_FIELDS
                .iter()
                .map(|name| (*name).to_string())
                .collect::<BTreeSet<_>>()
        );

        for (object, declared, label) in [
            (
                BorrowedObject::Typed(&step),
                TYPED_WIRE_FIELDS.as_slice(),
                "typed",
            ),
            (
                BorrowedObject::Shape(&shape),
                SHAPE_WIRE_FIELDS.as_slice(),
                "shape",
            ),
        ] {
            let wire =
                wire_template(&mut fixture.store, object, &new_life(), GENERATION).expect("wire");
            let resolved = replace_batch_placeholder(
                &mut fixture.store,
                &wire.content_ref(),
                &field_path("batch_id"),
                CURRENT_BATCH,
            )
            .expect("解析占位");
            let keys: BTreeSet<String> = rendered_ref(&mut fixture.store, &resolved)
                .as_object()
                .expect("对象")
                .keys()
                .cloned()
                .collect();
            assert_eq!(
                keys,
                declared
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect::<BTreeSet<_>>(),
                "{label} wire 字段集"
            );
        }
    }
}

/// 只借用生命周期字段；normal/owned只改变存储，校验与wire工厂同核。
#[derive(Clone, Copy)]
pub(crate) enum BatchView<'a> {
    CurrentBatch,
    Existing(&'a str),
}
#[derive(Clone, Copy)]
pub(crate) struct LifeView<'a> {
    pub first_known_generation: i64,
    pub first_known_cut: &'a str,
    pub published_generation: i64,
    pub withdrawn_generation: Option<i64>,
    pub withdrawal_reason: Option<&'a str>,
    pub superseded_by: Option<&'a str>,
    pub batch: BatchView<'a>,
}
impl PublicationLife {
    pub fn view(&self) -> LifeView<'_> {
        LifeView {
            first_known_generation: self.first_known_generation,
            first_known_cut: &self.first_known_cut,
            published_generation: self.published_generation,
            withdrawn_generation: self.withdrawn_generation,
            withdrawal_reason: self.withdrawal_reason.as_deref(),
            superseded_by: self.superseded_by.as_deref(),
            batch: match &self.batch {
                BatchRef::CurrentBatch => BatchView::CurrentBatch,
                BatchRef::Existing(s) => BatchView::Existing(s),
            },
        }
    }
}
#[derive(PartialEq, Eq)]
pub(crate) enum BatchOwner {
    CurrentBatch,
    Existing(super::business_json::TextValue),
}
#[derive(PartialEq, Eq)]
pub(crate) struct LifeOwner {
    pub first_known_generation: i64,
    pub first_known_cut: super::business_json::TextValue,
    pub published_generation: i64,
    pub withdrawn_generation: Option<i64>,
    pub withdrawal_reason: Option<super::business_json::TextValue>,
    pub superseded_by: Option<super::business_json::TextValue>,
    pub batch: BatchOwner,
}
impl LifeOwner {
    pub fn view(&self) -> LifeView<'_> {
        LifeView {
            first_known_generation: self.first_known_generation,
            first_known_cut: self.first_known_cut.as_str(),
            published_generation: self.published_generation,
            withdrawn_generation: self.withdrawn_generation,
            withdrawal_reason: self.withdrawal_reason.as_ref().map(|v| v.as_str()),
            superseded_by: self.superseded_by.as_ref().map(|v| v.as_str()),
            batch: match &self.batch {
                BatchOwner::CurrentBatch => BatchView::CurrentBatch,
                BatchOwner::Existing(s) => BatchView::Existing(s.as_str()),
            },
        }
    }
    pub fn from_normal(l: PublicationLife) -> Self {
        Self {
            first_known_generation: l.first_known_generation,
            first_known_cut: l.first_known_cut.into(),
            published_generation: l.published_generation,
            withdrawn_generation: l.withdrawn_generation,
            withdrawal_reason: l.withdrawal_reason.map(Into::into),
            superseded_by: l.superseded_by.map(Into::into),
            batch: match l.batch {
                BatchRef::CurrentBatch => BatchOwner::CurrentBatch,
                BatchRef::Existing(s) => BatchOwner::Existing(s.into()),
            },
        }
    }
    pub fn try_clone(&self) -> Result<Self, String> {
        Ok(Self {
            first_known_generation: self.first_known_generation,
            first_known_cut: self.first_known_cut.try_clone()?,
            published_generation: self.published_generation,
            withdrawn_generation: self.withdrawn_generation,
            withdrawal_reason: self
                .withdrawal_reason
                .as_ref()
                .map(|s| s.try_clone())
                .transpose()?,
            superseded_by: self
                .superseded_by
                .as_ref()
                .map(|s| s.try_clone())
                .transpose()?,
            batch: match &self.batch {
                BatchOwner::CurrentBatch => BatchOwner::CurrentBatch,
                BatchOwner::Existing(s) => BatchOwner::Existing(s.try_clone()?),
            },
        })
    }
}

impl std::fmt::Debug for BatchOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CurrentBatch => f.write_str("CurrentBatch"),
            Self::Existing(s) => f.debug_tuple("Existing").field(s).finish(),
        }
    }
}
impl std::fmt::Debug for LifeOwner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PublicationLife")
            .field("first_known_generation", &self.first_known_generation)
            .field("first_known_cut", &self.first_known_cut)
            .field("published_generation", &self.published_generation)
            .field("withdrawn_generation", &self.withdrawn_generation)
            .field("withdrawal_reason", &self.withdrawal_reason)
            .field("superseded_by", &self.superseded_by)
            .field("batch", &self.batch)
            .finish()
    }
}

#[cfg(feature = "s_capture_native_testonly")]
mod owner_retention {
    use super::*;
    use crate::v3::audit_retention::{retained_struct, Retained};

    retained_struct!(LifeOwner {
        first_known_generation,
        first_known_cut,
        published_generation,
        withdrawn_generation,
        withdrawal_reason,
        superseded_by,
        batch
    });
    impl Retained for BatchOwner {
        fn heap_bytes(&self) -> usize {
            match self {
                Self::CurrentBatch => 0,
                Self::Existing(v) => v.heap_bytes(),
            }
        }
    }
}
