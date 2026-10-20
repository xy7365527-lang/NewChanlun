//! #1394 采纳 C §5.2/5.3：S 生产侧的 CC-004 内容构造器（同核共享证据直接成值边）。
//!
//! 规范正本：`.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md`
//! 第 5.2（唯一内容编码）与 5.3（身份配方与全文绑定）节；采纳状态以同目录 `ADOPTION.json`
//! 与 `SPEC-ADDENDUM.md` 为准。逻辑 oracle（**只许测试用**）是 [`tb02a_facts`] 的
//! `step_payload` / `Builder::build_object` / `raw_ref`：本模块按同一批逻辑字段与规范字节
//! 重建内容图，但走的是「共享证据直接成值边」，不是「先造完整旧 `Value`/JSON 再拆图」。
//!
//! **位置（照实）**：这是供正式 S3 入口调用的**生产构造器**。它不接 S3Commit、不建自己的
//! 内容存储、不发 wire、不改任何既有路径的输出；`obj3-` 与旧 `obj-` 是两个配方，两者不
//! 逐字节相等（见 §5.3）。发布模板占位（0x08）、manifest、批次与索引仍归 S3 发布链。
//!
//! 三条边界：
//!
//! 1. **直接成值边**：输入只有同次 parser 的 [`IncrFactsState::shared_step`] 借用视图与
//!    坐标共享句柄；不物化坐标集合、不重算结构判据、不把旧 JSON 字符串再解析成图。
//! 2. **身份只由内容决定**：坐标集合按 §5.2 的 `M` 唯一分解构造（`k` = 严格小于 n 的最大
//!    2 的幂），与 `CoordsRef` 的 AVL 物理树形无关；`obj3` = `"obj3-"+hex(F(0x20, V))`，
//!    `V` 是严格六字段 `{kind,fact_key,payload,input_refs,profile_id,rule_revision}` 的值摘要。
//! 3. **memo 是性能结构**：三张 memo（每坐标十进制文本值根 / 源坐标数组 / 源引用数组）清空后
//!    重建的根逐字节相同；源版本注册表是**事实登记**，不是缓存——它只记「当前有效的八字段
//!    原始引用」，身份仍由内容寻址给出。
//!
//! 拒绝面（不兜底）：共享态退化、坐标没有当前源版本、`source_coord` 文本与注册坐标不一致、
//! 长度溢出以及任何节点形状错误一律 `Err`，不返回空数组、不回落旧 `Value`、不静默跳过。
//!
//! 本模块是供正式 S3 入口调用的生产构造器：接线前其 API 只被本文件测试使用，
//! 故整模块抑制 `dead_code`（而不是逐项 allow，免得漏掉某个入口）。

#![allow(dead_code)]

use super::*;
use newchan_rust::s_content::{
    self as sc, Built, CodecError, Kind, NodeDigest, NodeSink, NodeStore, NumberToken, SequenceRef,
    Value as CValue,
};
use newchan_rust::theta_v0::classifier::local_shape::{classify_local_shape, Cc006LocalShape};
use newchan_rust::theta_v0::parser::inclusion::{
    CoordNode, CoordsRef, EvidenceNode, GroupEvidenceView, IncrFactsState, StepEvidenceView,
};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::rc::Rc;

/// CC-004 步骤对象的 kind（与全量配方同名）。
pub(super) const CC004_STEP_KIND: &str = "CC-004.inclusion_step";
/// CC-005 组对象的 kind（与全量配方同名）。
pub(super) const CC005_GROUP_KIND: &str = "CC-005.inclusion_group";
/// CC-006 局部形态的 kind（与全量配方同名；身份走 §5.3 的**独立十二字段**配方）。
pub(super) const CC006_SHAPE_KIND: &str = "CC-006.local_shape";
/// CC-007 分型描述的 kind（与全量配方同名；身份走泛型六字段配方）。
pub(super) const CC007_FRACTAL_KIND: &str = "CC-007.fractal_description";
/// CC-054 知识状态的 kind（与全量配方同名；身份走泛型六字段配方）。
pub(super) const CC054_KNOWLEDGE_KIND: &str = "CC-054.knowledge_state";
/// 对象发布修订。内容身份不含修订，修订只进记录（与全量配方同值）。
const OBJECT_REVISION: i64 = 1;
/// §5.3 的类型化对象身份帧：`F(0x20, V)`。
const OBJECT_ID_TAG: u8 = 0x20;

/// 内容生产错误：缺源、共享态退化、类型不符与长度溢出一律显式拒绝。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ProducerError {
    /// 同核共享证据不可用（当前唯一成因：源坐标非严格递增）。**先于索引判定**，
    /// 不静默改用另一套判据，也不无声返回缺对象。
    Degraded(&'static str),
    /// 坐标没有当前源版本：不拿空数组或旧 `Value` 兜底。
    MissingSource(i64),
    /// 字段类型/身份不符（例如登记的 `source_coord` 文本与注册坐标不一致）。
    Type { what: &'static str, detail: String },
    /// 长度超出规范表达域（叶数/坐标域）。
    Overflow(String),
    /// CC-006 窗口不在可判域内（域前件不满足）：旧路径同样不造第五分型、不静默旁路。
    WindowNotClassified(&'static str),
    /// 内容层编解码错误（节点形状、摘要、长度、序列分解）。
    Codec(CodecError),
}

impl From<CodecError> for ProducerError {
    fn from(error: CodecError) -> Self {
        ProducerError::Codec(error)
    }
}

impl fmt::Display for ProducerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProducerError::Degraded(reason) => write!(f, "共享证据不可用：{reason}"),
            ProducerError::MissingSource(coord) => {
                write!(f, "源坐标 {coord} 尚未登记当前源版本")
            }
            ProducerError::Type { what, detail } => write!(f, "{what} 身份/类型不符：{detail}"),
            ProducerError::Overflow(detail) => write!(f, "长度溢出：{detail}"),
            ProducerError::WindowNotClassified(reason) => {
                write!(f, "CC-006 窗口不在可判域内：{reason}")
            }
            ProducerError::Codec(error) => write!(f, "内容编码错误：{error}"),
        }
    }
}

impl std::error::Error for ProducerError {}

/// 八字段原始引用（与全量配方 [`tb02a_facts::raw_ref`] 逐字段同型）。
///
/// 四个身份字段与 `input_revision` 在生产 raw 行里是**文本**；`revision`/`seq` 是 i64，
/// 按同一配方转十进制文本；`source_coord` 是已存的源坐标文本，登记时与注册坐标核对
/// 一致性，不一致即 [`ProducerError::Type`]。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RawRefSpec {
    pub identity_key: String,
    pub payload_hash: String,
    pub receipt_id: String,
    pub event_id: String,
    pub input_revision: String,
    pub revision: i64,
    pub seq: i64,
    pub source_coord: String,
}

impl RawRefSpec {
    pub(super) fn from_event(
        event: newchan_rust::s_content::capture_allocation::audit_json::JsonView<'_>,
    ) -> Result<RawRefSpec, String> {
        let text = |key: &str| -> Result<String, String> {
            crate::v3::business_json::field(event, key)
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
            revision: crate::v3::business_json::field(event, "revision")
                .as_i64()
                .ok_or("StorageUnavailable：raw.revision 不是 i64")?,
            seq: crate::v3::business_json::field(event, "seq")
                .as_i64()
                .ok_or("StorageUnavailable：raw.seq 不是 i64")?,
            source_coord: text("source_coord")?,
        })
    }

    /// 有界小对象（八字段全文本），不是巨大载荷。
    fn to_value(&self) -> CValue {
        CValue::Object(vec![
            (
                "identity_key".to_string(),
                CValue::String(self.identity_key.clone()),
            ),
            (
                "payload_hash".to_string(),
                CValue::String(self.payload_hash.clone()),
            ),
            (
                "receipt_id".to_string(),
                CValue::String(self.receipt_id.clone()),
            ),
            (
                "event_id".to_string(),
                CValue::String(self.event_id.clone()),
            ),
            (
                "input_revision".to_string(),
                CValue::String(self.input_revision.clone()),
            ),
            (
                "revision".to_string(),
                CValue::String(self.revision.to_string()),
            ),
            ("seq".to_string(), CValue::String(self.seq.to_string())),
            (
                "source_coord".to_string(),
                CValue::String(self.source_coord.clone()),
            ),
        ])
    }

    /// CC-006 旧生成态的同一八字段：`revision`/`seq` 保持 **i64 数值**（旧 raw 行原类型），
    /// 其余六字段与 [`RawRefSpec::to_value`] 同值。字段来源只有一份（本类型），不是第二套登记。
    fn to_shape_value(&self) -> Result<CValue, ProducerError> {
        Ok(CValue::Object(vec![
            (
                "identity_key".to_string(),
                CValue::String(self.identity_key.clone()),
            ),
            (
                "payload_hash".to_string(),
                CValue::String(self.payload_hash.clone()),
            ),
            (
                "receipt_id".to_string(),
                CValue::String(self.receipt_id.clone()),
            ),
            (
                "event_id".to_string(),
                CValue::String(self.event_id.clone()),
            ),
            (
                "input_revision".to_string(),
                CValue::String(self.input_revision.clone()),
            ),
            ("revision".to_string(), number(self.revision)?),
            ("seq".to_string(), number(self.seq)?),
            (
                "source_coord".to_string(),
                CValue::String(self.source_coord.clone()),
            ),
        ]))
    }
}

/// 只算摘要、不落盘的写入面。
///
/// `obj3` 的六字段身份对象是**派生输入**，不是要发布的节点：它的值摘要 `V` 只进
/// `F(0x20, ·)`，本身不必成为内容节点，故不污染节点表。
struct DigestOnly<'a>(Option<&'a sc::capture_allocation::scratch::WorkScope>);

impl NodeSink for DigestOnly<'_> {
    fn codec_sink_work(&self) -> Option<&sc::capture_allocation::scratch::WorkScope> {
        self.0
    }
    fn put(&mut self, _digest: NodeDigest, _tag: u8, _body: &[u8]) -> Result<(), CodecError> {
        Ok(())
    }
}

/// 一个 coords 子树的数组构造结果。
#[derive(Clone)]
struct ArrayEntry {
    /// 强句柄：条目存活期间该物理树不会被回收，键（地址）不会指向另一棵树。
    tree: Rc<CoordNode>,
    /// 元素摘要的规范 `M` 序列（父节点拼接接的是序列，不是数组根）。
    seq: SequenceRef,
    /// 0x05 数组值根。
    root: NodeDigest,
    /// 逻辑叶数（= `seq.count()`）。
    len: u64,
    /// 最小/最大坐标（源修订失效筛选用；由子树本身给出，不展开整棵树）。
    min: i64,
    max: i64,
}

/// CC-007 一条后续历史的**共享序列** memo（性能结构，不充任准入/证据权威）。
///
/// - 键：参考集指纹（6 个精确参考价/源身份/名字）；`end` 变化（右组末成员变了）即整条重建；
/// - `sequence` 是已写出的单元值序列（每元素 = `{bar, relations[24]}` 的值根，按 raw 坐标升序），
///   新 bar 整段 `sequence_rebuild_suffix` 一次接尾（只写新右脊），不重写已覆盖前缀、不落逐叶作废根；
/// - 精确内容键仅在 prepare 处与上一份 raw 账本比较一次；实际变化的最早坐标传入
///   每个窗口的 `floor`。窗口内二分定位截断点再续接，不逐窗扫描或保存整份历史键；
/// - `union_tail` 是 tail 坐标的**并集增量**链（只 `append_greater` 续接）：右组被下一组
///   封口时其 `conf.coords` 末项已是 tail 首坐标，故并集增量跳过该项——原配方
///   `sources.sort/dedup` 的同一条语义，供 CC-007 的 `input_refs`/`source_coords` 与请求共享；
/// - 跨事务/换库必须 [`Cc004Producer::discard_runtime_state`]（序列里的根不再属于当前域）。
struct FractalHistory {
    /// 本窗口右组末成员坐标（end 变化 ⟹ 内容变了，整条重建）。
    end: usize,
    /// 序列首元素在当次 raw 表里的下标（截断/续接定位用）。
    tail_start: usize,
    /// 已覆盖单元数（= `sequence.count()`）。
    count: usize,
    /// 源修订水位：`bind_source` 报告的**实际变化坐标**的最小值；续接时坐标 ≥ 该水位的
    /// 已覆盖单元整段截断重建（保守口径：源修订必使该点起的 memo 失效）。
    floor: Option<i64>,
    /// 已覆盖 tail 的并集增量坐标链（跳过被 `conf.coords` 覆盖的 tail 首项后，与 units 同序）。
    union_tail: CoordsRef,
    /// 单元值序列（0x11/0x12/0x13 规范 M 树）。
    sequence: SequenceRef,
}

/// 一代 raw 账本的一次**受控准备**结果（owner/epoch 见证）。
///
/// 只能经 [`Cc004Producer::prepare_raw_history`] 构造，字段私有：调用方既拿不到伪造入口，
/// 也省不掉排序 / source 存在 / 精确内容键三种核（一次遍历核完，见该方法文档）。
///
/// - `raw` 借用调用方的 raw 切片：借用在 ⟹ 本视图存活期内 raw 不被改写（Rust 借用保证）；
/// - 当前准备版本由独立 Rc 见证；成功重新 prepare 后旧视图即失效，即使 source_ref 未换。
///   精确内容键由 producer 保存一份，变更在 prepare 中比较并传入所有窗口的失效水位；
/// - `owner` / `epoch` 在 [`Cc004Producer::build_fractal`] 消费口校验：换 producer 或
///   source 修订推进后，旧视图被显式拒绝（陈旧视图不得静默复用）。
pub(super) struct PreparedRawHistory<'a> {
    owner: Rc<()>,
    epoch: u64,
    snapshot: Rc<()>,
    raw: super::s_fact_cache::RawBars<'a>,
    delta: Option<super::s_fact_cache::EffectiveRawView<'a>>,
}

/// 同一次 parser / prepare 的生成 memo；不是节点存在、审计或发布证明。
/// 仅保存本次成功生成的两数组根，生命周期不进入 producer 跨代缓存。
pub(super) struct ParserFractalSources<'a> {
    state: &'a IncrFactsState,
    owner: Rc<()>,
    epoch: u64,
    snapshot: Rc<()>,
    call: Rc<()>,
    arrays: Option<(Built, Built)>,
    #[cfg(test)]
    whole_source_builds: usize,
}

/// 只能由上述 session 的真实 build_shape 产生；不提供可变对象或人工 sources 入口。
pub(super) struct ParserFractalShape {
    object: Cc006Object,
    start: usize,
    call: Rc<()>,
}

impl ParserFractalShape {
    pub(super) fn object(&self) -> &Cc006Object {
        &self.object
    }

    pub(super) fn into_object(self) -> Cc006Object {
        self.object
    }
}

/// coords 子树的构造 memo（性能结构，不充任准入/证据权威）。
#[derive(Default)]
struct ArrayCache {
    /// 键 = 物理树地址。条目自带 `Rc` 强句柄 ⟹ 键在条目存活期内唯一且不复用。
    entries: HashMap<usize, ArrayEntry>,
    /// 全部条目涉及坐标的上界：普通追加（新坐标更大）据此跳过整表遍历。
    max_coord: Option<i64>,
    hits: usize,
    misses: usize,
    invalidated: usize,
    skipped_scans: usize,
}

impl ArrayCache {
    fn get(&self, node: &Rc<CoordNode>) -> Option<ArrayEntry> {
        self.entries.get(&(Rc::as_ptr(node) as usize)).cloned()
    }

    fn insert(&mut self, entry: ArrayEntry) {
        self.max_coord = Some(match self.max_coord {
            Some(bound) => bound.max(entry.max),
            None => entry.max,
        });
        self.entries.insert(Rc::as_ptr(&entry.tree) as usize, entry);
    }

    /// 源版本变化：只失效**依赖该坐标**的条目（`min ≤ coord ≤ max`）。
    ///
    /// 新坐标大于全部条目上界时整表遍历直接跳过——普通追加（新坐标即最大坐标）不走
    /// 全缓存扫描，也不使任何旧 prefix 失效。
    fn invalidate(&mut self, coord: i64) {
        if let Some(bound) = self.max_coord {
            if coord > bound {
                self.skipped_scans += 1;
                return;
            }
        }
        let before = self.entries.len();
        self.entries
            .retain(|_, entry| !(entry.min <= coord && coord <= entry.max));
        self.invalidated += before - self.entries.len();
    }

    fn clear(&mut self) {
        self.entries.clear();
    }
}

/// 叶编码器：同一套 `M` 遍历，只换叶摘要的来源。
enum Leaves<'a> {
    /// 源坐标数组：叶是坐标的十进制文本值根（按**坐标值**记忆，与地址无关）。
    CoordText(&'a mut HashMap<i64, NodeDigest>),
    /// 源引用数组：叶是当前源版本（八字段 `raw_ref`）值根。
    SourceRef(&'a BTreeMap<i64, (RawRefSpec, NodeDigest)>),
    /// CC-006 旧生成态源引用数组：叶是数值型八字段值根（`revision`/`seq` 为数值）。
    ShapeSourceRef(&'a BTreeMap<i64, NodeDigest>),
}

impl Leaves<'_> {
    fn element(
        &mut self,
        store: &mut dyn NodeStore,
        coord: i64,
    ) -> Result<NodeDigest, ProducerError> {
        match self {
            Leaves::CoordText(memo) => coord_text_root(&mut *store, memo, coord),
            Leaves::SourceRef(sources) => sources
                .get(&coord)
                .map(|(_, root)| *root)
                .ok_or(ProducerError::MissingSource(coord)),
            Leaves::ShapeSourceRef(roots) => roots
                .get(&coord)
                .copied()
                .ok_or(ProducerError::MissingSource(coord)),
        }
    }
}

/// 源引用数组的两种**叶编码**（不是两套判据）：generic typed 对象与 CC-006 公开 wire 用
/// 十进制文本叶，CC-006 旧生成态用 raw 行原类型（`revision`/`seq` 数值）。
///
/// 两者共用同一份坐标集合、同一条 [`array_from_node`] 构造路径、同一份 dependency 计算，
/// 只换叶来源与 memo；源版本变化时两者按同一坐标失效。
#[derive(Clone, Copy, PartialEq, Eq)]
enum RefEncoding {
    /// `RawRefSpec::to_value`：八字段全文本（generic 与 CC-006 wire）。
    WireText,
    /// `RawRefSpec::to_shape_value`：`revision`/`seq` 数值（CC-006 旧生成态 / 十二字段身份）。
    ShapeInteger,
}

/// 一个**泛型 typed 对象**（§5.3 六字段身份家族：CC-004/005/007/054）的构造结果。
///
/// 各根都已按**同一次调用所用的 store** 写出。
///
/// 写 ≠ publish-valid：构造器返回不是发布证明。发布前必须用同一读集过 `verify`（全闭包），
/// 因为 `Value::Edge` 指向的子根不在本次写入的核验范围内。
#[derive(Debug, Clone)]
pub(super) struct TypedObject {
    /// `obj3-` + 小写十六进制 `F(0x20, V)`。
    pub object_id: String,
    /// 六字段身份对象的值摘要 `V`（obj3 配方的输入，供独立复算）。
    pub identity: NodeDigest,
    /// 槽位规范文本（与全量配方同一条：`canonical_json([kind, coord])`）。
    pub fact_key: String,
    /// 内部辅助记录：十项旧生成态字段 + 三条内部值边。
    /// 它不是 Q wire 或实际首获知发布记录；接线时须按各自 exact schema 组装。
    pub record: Built,
    /// 公开边：载荷值根（`payload_json` 的内部值）。
    pub payload: Built,
    /// 公开边：源引用数组根（`input_refs_json` 的内部值）。
    pub input_refs: Built,
    /// 公开边：源坐标数组根（`source_coords_json` 的内部值）。
    pub source_coords: Built,
    /// 0x07 文本视图根（内部值 = `payload`）。
    pub payload_json: Built,
    /// 0x07 文本视图根（内部值 = `input_refs`）。
    pub input_refs_json: Built,
    /// 0x07 文本视图根（内部值 = `source_coords`）。
    pub source_coords_json: Built,
    /// 本次生成态投射的代号；实际首获知代仍由独立索引核定。
    pub generation: i64,
    /// 本次生成态投射的 cut；实际首获知 cut 仍由独立索引核定。
    pub cut: String,
    /// 该事实的知识请求值边（`subject_id` = 本对象 `obj3` 身份）。
    ///
    /// `None` 只出现在**旧配方本就没有对应请求**的 kind（CC-054）；不造默认请求。
    pub request: Option<Built>,
}

/// 旧名保留：CC-004 步骤对象就是泛型 typed 对象。
pub(super) type Cc004Object = TypedObject;

/// 一个 CC-006 局部形态对象的构造结果。
///
/// **独立身份配方**（§5.3）：`obj3` = `"obj3-"+hex(F(0x20, V(十二字段)))`，不套泛型六字段；
/// 十二字段 = `{branch,dir_ab,dir_bc,window_start,window_mid,window_end,merged_highs,
/// merged_lows,comparisons,input_refs,rule_revision,profile_id}`，字段类型与值取旧配方。
#[derive(Debug, Clone)]
pub(super) struct Cc006Object {
    /// `obj3-` + 小写十六进制 `F(0x20, V)`。
    pub object_id: String,
    /// 十二字段身份对象的值摘要 `V`（供独立复算）。
    pub identity: NodeDigest,
    /// 内部辅助记录：旧 CC-006 行字段 + 三条内容边（`comparisons` 无对应文本列，只成值边）。
    pub record: Built,
    /// 四个严格比较的数组值根（旧 `comparisons_json` 的内部值）。
    pub comparisons: Built,
    /// 旧生成态的 `input_refs`：三组 `{merged_index,raw_refs,dependency_refs}` 的数组值根，
    /// `merged_index` 与 `revision`/`seq` 保持**旧行原类型（数值）**。它同时是十二字段身份的
    /// `input_refs`、`record` 的 `input_refs` 值边与 `input_refs_json` 的内部值。
    pub input_refs: Built,
    /// CC-006 **公开 wire** 的 `input_refs` 数组值根：与旧 v2 投影 `project_input_refs` 同一
    /// 结果（`merged_index` 与 `revision`/`seq` 为规范十进制文本，其余六字段文本不变），
    /// 同次生成、与 [`Cc006Object::input_refs`] 共享坐标与 dependency 计算。
    ///
    /// 它**不参与**十二字段身份与生成态记录绑定；发布侧只借根，不得用生成态数值叶重渲染。
    pub wire_input_refs: Built,
    /// 来源坐标文本数组根（旧 `source_coords_json` 的内部值）。
    pub source_coords: Built,
    /// 0x07 文本视图根（内部值 = `input_refs`）。
    pub input_refs_json: Built,
    /// 0x07 文本视图根（内部值 = `source_coords`）。
    pub source_coords_json: Built,
    /// 该形态的来源坐标**共享链**（右组 `group_sources_coords`）= CC-007 的 `base_sources`。
    ///
    /// 依据：三个连续同 kernel 组的来源链逐级嵌套（`sources_i ⊆ sources_{i+1}`，见
    /// [`group_sources_coords`] 的归纳），故并集 = 右组来源链——不是「挑最大链」，
    /// 而是归纳包含关系；由 `derived_coords_match_materialized_union_for_every_prefix`
    /// 的「三连窗并集 == 右组 sources」逐前缀对拍锁定。
    pub sources: CoordsRef,
    /// 三组锚坐标（升序）。
    pub window: [usize; 3],
    /// 知识请求值边（槽位 `["CC-006.local_shape", [三锚坐标]]`，主体 = 本对象）。
    pub request: Built,
    /// 四分支名（`RISING`/`TOP`/`BOTTOM`/`FALLING`）：CC-007 只对 `TOP`/`BOTTOM` 建对象。
    pub branch: &'static str,
    /// 本次生成态投射的代号；实际首获知代仍由独立索引核定。
    pub generation: i64,
    /// 本次生成态投射的 cut；实际首获知 cut 仍由独立索引核定。
    pub cut: String,
}

/// 一个 CC-054 知识状态对象连同本代观察（`awaiting_scope`，每个去重原因一条）。
#[derive(Debug, Clone)]
pub(super) struct Cc054Assembly {
    /// 知识状态对象（泛型六字段身份；旧配方里它没有对应请求，故 `request` 为 `None`）。
    pub object: TypedObject,
    /// 观察值边（顺序 = 排序去重后的原因序，与全量配方同序）。
    pub observations: Vec<Built>,
}

/// 一个 CC-007 分型描述对象的构造结果（含三条知识请求）。
#[derive(Debug, Clone)]
pub(super) struct Cc007Object {
    /// `obj3-` + 小写十六进制 `F(0x20, V)`（泛型六字段配方）。
    pub object_id: String,
    /// 六字段身份对象的值摘要 `V`。
    pub identity: NodeDigest,
    /// 槽位规范文本（`canonical_json([kind, [三个锚坐标]])`）。
    pub fact_key: String,
    /// 内部辅助记录：旧十项字段 + 三条内部值边。
    pub record: Built,
    /// 公开边：载荷值根。
    pub payload: Built,
    /// 公开边：源引用数组根。
    pub input_refs: Built,
    /// 公开边：源坐标数组根。
    pub source_coords: Built,
    /// 0x07 文本视图根（内部值 = `payload`）。
    pub payload_json: Built,
    /// 0x07 文本视图根（内部值 = `input_refs`）。
    pub input_refs_json: Built,
    /// 0x07 文本视图根（内部值 = `source_coords`）。
    pub source_coords_json: Built,
    /// 三条知识请求值边，顺序 = `shape_description` / `descriptive_labels` /
    /// `subsequent_development`（与全量配方同序）。
    pub requests: Vec<Built>,
    /// `subsequent_development` 请求的原因（`no_subsequent_bar` 或空）。
    pub request_reasons: Vec<&'static str>,
    /// 进 CC-054 `waiting_reasons` 的原因（本对象贡献：`request_reasons` +
    /// `no_settled_numeric_rule`，与全量配方同一条）。
    pub knowledge_reasons: Vec<&'static str>,
    /// 本次生成态投射的代号；实际首获知代仍由独立索引核定。
    pub generation: i64,
    /// 本次生成态投射的 cut；实际首获知 cut 仍由独立索引核定。
    pub cut: String,
}

/// 已核字段的借用值；不把核对象来源的职责藏入身份哈希。
pub(super) struct ShapeIdentity<'a> {
    pub branch: &'a str,
    pub dir_ab: &'a str,
    pub dir_bc: &'a str,
    pub window: [String; 3],
    pub highs: [String; 3],
    pub lows: [String; 3],
    pub comparisons: &'a Built,
    pub input_refs: &'a Built,
}

/// 十二字段身份的无复制视图；字段来源核验仍由调用方承担。
pub(super) struct ShapeIdentityView<'a> {
    pub branch: &'a str,
    pub dir_ab: &'a str,
    pub dir_bc: &'a str,
    pub window: [&'a str; 3],
    pub highs: [&'a str; 3],
    pub lows: [&'a str; 3],
    pub comparisons: &'a Built,
    pub input_refs: &'a Built,
}

impl TypedObject {
    /// 旧逻辑字段名到新内容边的对应（供调用方/测试核对，不参与身份）。
    pub fn content_edges(&self) -> [(&'static str, &Built); 3] {
        [
            ("payload", &self.payload),
            ("input_refs", &self.input_refs),
            ("source_coords", &self.source_coords),
        ]
    }
}

/// S 生产侧的 CC-004 构造器：源版本注册表 + 两张 coords 子树 memo。
///
/// 作用域：一个（库、会话代、规则修订、档案、目录修订）车道一份。`profile_id` /
/// `rule_revision` 是会话冻结值，进身份配方，故由构造器持有而不是逐次传入。
pub(super) struct Cc004Producer {
    profile_id: String,
    rule_revision: String,
    /// 事实登记：coord →（最后登记的八字段原件, 当前值根）。
    sources: BTreeMap<i64, (RawRefSpec, NodeDigest)>,
    /// coord → 十进制文本字符串值根（纯函数，按坐标值记忆）。
    coord_text: HashMap<i64, NodeDigest>,
    /// 源坐标数组 memo：只依赖坐标集合，不随源修订失效。
    text_arrays: ArrayCache,
    /// 源引用数组 memo（wire 文本叶）：依赖当前源绑定，源修订按坐标失效。
    ref_arrays: ArrayCache,
    /// CC-006 旧生成态源引用值根（coord → 数值型八字段值根）；与 `sources` 同键、同生同死。
    shape_sources: BTreeMap<i64, NodeDigest>,
    /// CC-006 旧生成态源引用数组 memo（数值叶）：与 `ref_arrays` 同源绑定、同坐标失效。
    shape_ref_arrays: ArrayCache,
    /// CC-007 后续历史 memo：按 `(参考集指纹, end)` **保留**（交错窗口互不驱逐）。
    fractal_histories: HashMap<(String, usize), FractalHistory>,
    /// CC-007 测量面计数（不参与判据）：新求值单元 / 续接复用单元 / 源修订失效单元。
    history_computed: usize,
    history_reused: usize,
    history_invalidated: usize,
    /// source 修订代号：`bind_source` 实际变化即 +1（`discard_runtime_state` 也 +1）；
    /// `prepare_raw_history` 的视图按它做陈旧判定。
    source_epoch: u64,
    /// 自上次 `prepare_raw_history` 以来实际变化的**最早源坐标**：prepare 时一次并入
    /// 全部已保留窗口的失效水位（登记面 O(1)，不逐窗口扫描）。
    unprepared_floor: Option<i64>,
    /// 本 producer 的 raw 账本身份 token：`PreparedRawHistory` 的 owner 防伪用。
    raw_token: Rc<()>,
    /// 当前 prepare 的独立见证，防止两份 raw 值在同一 source epoch 内交错消费。
    prepared_snapshot: Rc<()>,
    /// 上次准备的实际坐标和精确内容键；全 producer 仅一份，不按窗口重复存储。
    prepared_raw_keys: Vec<(usize, String)>,
    delta_binding: Option<crate::v3::audit_owner::RawAdvanceBinding>,
    delta_epoch: Option<u64>,
}

/// #1488 来源身份核只借用 producer 的只读方法；初始化载荷先销毁，再归还原许可。
/// 此壳不提供可变入口，也不声称覆盖可变生产构造器随后建立的缓存。
pub(super) struct SourceIdentityProducer {
    inner: Cc004Producer,
    _initial: Option<sc::capture_allocation::scratch::NativeReservation>,
}

impl std::ops::Deref for SourceIdentityProducer {
    type Target = Cc004Producer;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
/// 测试/测量面读数（不参与任何判据）。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct MemoCounts {
    pub text_hits: usize,
    pub text_misses: usize,
    pub ref_hits: usize,
    pub ref_misses: usize,
    /// 因源版本变化被丢弃的 memo 条目数。
    pub invalidated: usize,
    /// CC-006 旧生成态（数值叶）源引用 memo：命中次数。
    pub shape_hits: usize,
    /// CC-006 旧生成态（数值叶）源引用 memo：未命中次数。
    pub shape_misses: usize,
    /// CC-006 旧生成态（数值叶）源引用 memo：因源版本变化被丢弃的条目数。
    pub shape_invalidated: usize,
    /// 新坐标大于 memo 上界而跳过的整表遍历次数。
    pub skipped_scans: usize,
    /// CC-007 比较单元：实际求值次数。
    pub unit_computed: usize,
    /// CC-007 比较单元：精确内容键命中复用次数。
    pub unit_reused: usize,
    /// CC-007 比较单元：因源修订失效丢弃的条目数。
    pub unit_invalidated: usize,
}

impl Cc004Producer {
    pub(super) fn for_source_audit(
        profile_id: &str,
        rule_revision: &str,
        work: Option<&sc::capture_allocation::scratch::WorkScope>,
    ) -> Result<SourceIdentityProducer, CodecError> {
        let bytes = profile_id
            .len()
            .checked_add(rule_revision.len())
            .and_then(|n| n.checked_add(2 * std::mem::size_of::<[usize; 2]>()))
            .ok_or_else(|| {
                work.map_or_else(
                    || CodecError::new(sc::capture_allocation::LIMIT, "Native容量溢出"),
                    |w| w.reject(sc::capture_allocation::scratch::ScratchFailure::Budget),
                )
            })?;
        let permit = work.map(|w| w.reserve_native(bytes)).transpose()?;
        Ok(SourceIdentityProducer {
            inner: Self::new(profile_id, rule_revision),
            _initial: permit,
        })
    }

    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    pub(super) fn visit_runtime_roots(&self, f: &mut dyn FnMut(&'static str, NodeDigest)) {
        for (_, root) in self.sources.values() {
            f("producer.sources", *root);
        }
        for root in self.shape_sources.values() {
            f("producer.shape_sources", *root);
        }
        for root in self.coord_text.values() {
            f("producer.coord_text", *root);
        }
        for (name, cache) in [
            ("producer.text_arrays", &self.text_arrays),
            ("producer.ref_arrays", &self.ref_arrays),
            ("producer.shape_ref_arrays", &self.shape_ref_arrays),
        ] {
            for entry in cache.entries.values() {
                f(name, entry.root);
                f(name, entry.seq.root());
            }
        }
        for history in self.fractal_histories.values() {
            f(
                "producer.fractal_histories.sequence",
                history.sequence.root(),
            );
        }
    }

    /// 会话冻结身份（进 §5.3 的六字段配方）。
    pub(super) fn new(profile_id: &str, rule_revision: &str) -> Self {
        Cc004Producer {
            profile_id: profile_id.to_string(),
            rule_revision: rule_revision.to_string(),
            sources: BTreeMap::new(),
            coord_text: HashMap::new(),
            text_arrays: ArrayCache::default(),
            ref_arrays: ArrayCache::default(),
            shape_sources: BTreeMap::new(),
            shape_ref_arrays: ArrayCache::default(),
            fractal_histories: HashMap::new(),
            history_computed: 0,
            history_reused: 0,
            history_invalidated: 0,
            source_epoch: 0,
            unprepared_floor: None,
            raw_token: Rc::new(()),
            prepared_snapshot: Rc::new(()),
            prepared_raw_keys: Vec::new(),
            delta_binding: None,
            delta_epoch: None,
        }
    }

    /// 会话冻结规则修订（来源审计按它选新笔/知识域合同，#1405）。
    pub(super) fn rule_revision(&self) -> &str {
        &self.rule_revision
    }

    /// 登记/更新一个源坐标的当前版本，返回该坐标的源引用根是否**变化**（首次登记也算）。
    ///
    /// 变化 ⟹ 依赖该坐标的源引用数组 memo 失效；普通追加（新坐标大于已登记上界）只
    /// 跳过扫描，不使任何旧 prefix 失效，也不动源坐标数组 memo。
    pub(super) fn bind_source(
        &mut self,
        store: &mut dyn NodeStore,
        coord: i64,
        spec: RawRefSpec,
    ) -> Result<bool, ProducerError> {
        let parsed = parse_canonical_i64(&spec.source_coord, "source_coord").map_err(|detail| {
            ProducerError::Type {
                what: "source_coord",
                detail,
            }
        })?;
        if parsed != coord {
            return Err(ProducerError::Type {
                what: "source_coord",
                detail: format!("登记文本 `{}` 与注册坐标 {coord} 不一致", spec.source_coord),
            });
        }
        if let Some((previous, _)) = self.sources.get(&coord) {
            if previous == &spec {
                return Ok(false);
            }
        }
        let built = sc::write_value(&mut *store, &spec.to_value())?;
        // 同一份登记事实的第二种编码：CC-006 旧生成态（`revision`/`seq` 数值）。两叶同写同换，
        // 任一源版本变化都不得留下旧编码的叶。
        let shape_built = sc::write_value(&mut *store, &spec.to_shape_value()?)?;
        let changed = match self.sources.get(&coord) {
            Some((_, root)) => *root != built.digest,
            None => true,
        };
        self.sources.insert(coord, (spec, built.digest));
        self.shape_sources.insert(coord, shape_built.digest);
        if changed {
            self.ref_arrays.invalidate(coord);
            self.shape_ref_arrays.invalidate(coord);
            // 历史 memo 不在此处整条丢弃，只记**实际变化坐标**的水位：prepare 时一次并入
            // 全部已保留窗口（见 `prepare_raw_history`），续接时坐标 ≥ 该水位的已覆盖单元
            // 截断重建（`sequence_rebuild_suffix`）；prepare 中的精确内容键比较另核 raw 内容漂移。
            self.source_epoch += 1;
            self.unprepared_floor = Some(
                self.unprepared_floor
                    .map_or(coord, |floor| floor.min(coord)),
            );
        }
        Ok(changed)
    }

    /// 某坐标当前登记的源引用值根（未登记 = `None`）。
    pub(super) fn source_root(&self, coord: i64) -> Option<NodeDigest> {
        self.sources.get(&coord).map(|(_, root)| *root)
    }

    /// `source_coords`：坐标集合 → 十进制文本数组（0x05）。
    pub(super) fn coords_array(
        &mut self,
        store: &mut dyn NodeStore,
        coords: &CoordsRef,
    ) -> Result<Built, ProducerError> {
        let Some(root) = coords.root_node() else {
            return empty_array(store);
        };
        let entry = array_from_node(
            &mut *store,
            root,
            &mut self.text_arrays,
            &mut Leaves::CoordText(&mut self.coord_text),
        )?;
        Ok(array_built(&entry))
    }

    /// 共享树直接入口（`coords_array` 的底层）：同一逻辑叶序的不同物理树形必须同根。
    pub(super) fn coords_array_of_node(
        &mut self,
        store: &mut dyn NodeStore,
        node: &Rc<CoordNode>,
    ) -> Result<Built, ProducerError> {
        let entry = array_from_node(
            &mut *store,
            node,
            &mut self.text_arrays,
            &mut Leaves::CoordText(&mut self.coord_text),
        )?;
        Ok(array_built(&entry))
    }

    /// #1394 C §5.3：坐标的十进制文本**值叶**（与 `coords_array` 用**同一张** coords 叶图与
    /// **同一份** memo）。
    ///
    /// 供按增量内容序列续接坐标数组的调用方（TB-02-B 的区间依赖面）直接取叶：不另写一套文本
    /// 编码，也不复制叶图；坐标文本与源版本无关，这份 memo 永不失效。
    pub(super) fn coord_text_leaf(
        &mut self,
        store: &mut dyn NodeStore,
        coord: i64,
    ) -> Result<NodeDigest, ProducerError> {
        coord_text_root(&mut *store, &mut self.coord_text, coord)
    }

    /// `input_refs`（generic typed 对象与 CC-006 公开 wire 的文本编码）：坐标集合 →
    /// 当前源绑定下的八字段源引用数组（0x05）。
    ///
    /// 与 [`Cc004Producer::coords_array`] 用**同一张 coords 叶图**，只是叶按当前源版本
    /// 绑定；任一坐标的源版本变化即失效依赖它的条目。
    pub(super) fn source_refs_array(
        &mut self,
        store: &mut dyn NodeStore,
        coords: &CoordsRef,
    ) -> Result<Built, ProducerError> {
        self.source_refs_array_in(store, coords, RefEncoding::WireText)
    }

    /// 两种源引用编码的共用入口：坐标集合 → 八字段源引用数组（0x05），只按 `encoding`
    /// 选叶值编码与 memo；坐标集合、序列分解与构造路径完全同一份。
    fn source_refs_array_in(
        &mut self,
        store: &mut dyn NodeStore,
        coords: &CoordsRef,
        encoding: RefEncoding,
    ) -> Result<Built, ProducerError> {
        let Some(root) = coords.root_node() else {
            return empty_array(store);
        };
        let entry = self.ref_array_over(store, root, encoding)?;
        Ok(array_built(&entry))
    }

    /// [`array_from_node`] 的编码分派口（唯一构造规则；两个 memo/叶来源按编码二选一）。
    fn ref_array_over(
        &mut self,
        store: &mut dyn NodeStore,
        root: &Rc<CoordNode>,
        encoding: RefEncoding,
    ) -> Result<ArrayEntry, ProducerError> {
        match encoding {
            RefEncoding::WireText => array_from_node(
                &mut *store,
                root,
                &mut self.ref_arrays,
                &mut Leaves::SourceRef(&self.sources),
            ),
            RefEncoding::ShapeInteger => array_from_node(
                &mut *store,
                root,
                &mut self.shape_ref_arrays,
                &mut Leaves::ShapeSourceRef(&self.shape_sources),
            ),
        }
    }

    /// 某坐标当前登记的源引用**叶**根（按编码选文本叶或 CC-006 数值叶）。
    fn ref_leaf_root(&self, encoding: RefEncoding, coord: i64) -> Option<NodeDigest> {
        match encoding {
            RefEncoding::WireText => self.sources.get(&coord).map(|(_, root)| *root),
            RefEncoding::ShapeInteger => self.shape_sources.get(&coord).copied(),
        }
    }

    /// 从同次 parser 的共享态借出单步证据并构造 CC-004 对象。
    ///
    /// - `Ok(Some(object))`：该步存在，对象的节点已写入**本次调用所用的同一个 store**。
    ///   写成功只说明「图被构造出来了」：发布前必须用同一读集过 `verify`（全闭包），
    ///   构造器返回**不是**发布证明；
    /// - `Ok(None)`：该索引没有步骤（`index >= step_count()`）；
    /// - `Err(Degraded)`：共享态退化，**先于索引判定**。
    pub(super) fn build_step(
        &mut self,
        store: &mut dyn NodeStore,
        state: &IncrFactsState,
        index: usize,
        generation: i64,
        cut: &str,
    ) -> Result<Option<Cc004Object>, ProducerError> {
        let view = state
            .shared_step(index)
            .map_err(|error| ProducerError::Degraded(error.reason))?;
        match view {
            Some(view) => self
                .build_step_object(store, &view, generation, cut)
                .map(Some),
            None => Ok(None),
        }
    }

    /// 从借用视图构造对象（`build_step` 的视图面；不占索引、不读全量事实）。
    pub(super) fn build_step_object(
        &mut self,
        store: &mut dyn NodeStore,
        view: &StepEvidenceView<'_>,
        generation: i64,
        cut: &str,
    ) -> Result<Cc004Object, ProducerError> {
        let fact_key = canonical_json(&json!([
            CC004_STEP_KIND,
            view.incoming.source_index.to_string()
        ]));
        let payload_value = self.step_payload_value(store, view)?;
        let input_refs = self.source_refs_array(&mut *store, view.coords)?;
        let source_coords = self.coords_array(&mut *store, view.coords)?;
        let mut object = self.typed_object(
            store,
            CC004_STEP_KIND,
            fact_key,
            payload_value,
            input_refs,
            source_coords,
            generation,
            cut,
        )?;
        let mut reasons: Vec<_> = view
            .waiting_reasons
            .iter()
            .copied()
            .filter(|reason| *reason != "equal_extreme_identity")
            .collect();
        if view.evidence.is_none() && reasons.is_empty() {
            reasons.push("direction_not_yet_established");
        }
        let key = CValue::Array(vec![
            CValue::String(CC004_STEP_KIND.to_string()),
            CValue::String(view.incoming.source_index.to_string()),
        ]);
        let request = self.request_value(
            key,
            Some(&object.object_id),
            &reasons,
            &object.source_coords,
        )?;
        object.request = Some(sc::write_value(&mut *store, &request)?);
        Ok(object)
    }

    /// §5.3 泛型六字段身份对象的唯一实现（CC-004/005/007/054 共用）。
    ///
    /// `payload` / `input_refs` / `source_coords` 是三条**公开内容边**（值根），
    /// 记录里的三个 `*_json` 字段是 §5.2 的 0x07 文本视图（内部值根 = 同一值根）——
    /// 渲染为规范 JSON 文本字符串，与全量配方存下来的文本列逐字同值。
    #[allow(clippy::too_many_arguments)]
    fn typed_object(
        &self,
        store: &mut dyn NodeStore,
        kind: &str,
        fact_key: String,
        payload_value: CValue,
        input_refs: Built,
        source_coords: Built,
        generation: i64,
        cut: &str,
    ) -> Result<TypedObject, ProducerError> {
        let payload = sc::write_value(&mut *store, &payload_value)?;
        let payload_json = Built {
            digest: put_json_text_node(&mut *store, &payload.digest)?,
            kind: Kind::JsonText,
            extent: None,
        };
        let input_refs_json = Built {
            digest: put_json_text_node(&mut *store, &input_refs.digest)?,
            kind: Kind::JsonText,
            extent: None,
        };
        let source_coords_json = Built {
            digest: put_json_text_node(&mut *store, &source_coords.digest)?,
            kind: Kind::JsonText,
            extent: None,
        };
        let identity = self.typed_identity(kind, &fact_key, &payload, &input_refs)?;
        let object_id = object_id_of(&identity);
        let record_value = CValue::Object(vec![
            ("object_id".to_string(), CValue::String(object_id.clone())),
            ("object_revision".to_string(), number(OBJECT_REVISION)?),
            ("kind".to_string(), CValue::String(kind.to_string())),
            ("fact_key".to_string(), CValue::String(fact_key.clone())),
            ("payload".to_string(), CValue::Edge(payload.digest)),
            ("input_refs".to_string(), CValue::Edge(input_refs.digest)),
            (
                "source_coords".to_string(),
                CValue::Edge(source_coords.digest),
            ),
            (
                "payload_json".to_string(),
                CValue::Edge(payload_json.digest),
            ),
            (
                "input_refs_json".to_string(),
                CValue::Edge(input_refs_json.digest),
            ),
            (
                "source_coords_json".to_string(),
                CValue::Edge(source_coords_json.digest),
            ),
            ("first_known_generation".to_string(), number(generation)?),
            (
                "first_known_cut".to_string(),
                CValue::String(cut.to_string()),
            ),
            ("published_generation".to_string(), number(generation)?),
        ]);
        let record = sc::write_value(&mut *store, &record_value)?;
        Ok(TypedObject {
            object_id,
            identity,
            fact_key,
            record,
            payload,
            input_refs,
            source_coords,
            payload_json,
            input_refs_json,
            source_coords_json,
            generation,
            cut: cut.to_string(),
            request: None,
        })
    }

    /// #1394 C §5.3：泛型六字段 `obj3` 对象的**共享构造口**。
    ///
    /// 与 `build_step_object` 等入口走同一条 `typed_object` 实现（同一份身份配方、同一份记录
    /// 字段），供同样只从同核共享证据直接成值边的兄弟构造器（TB-02-B 的 CC-008/009/010/055/056）
    /// 复用。本口不接受预渲染的 JSON 文本，也不新增第二种身份配方。
    #[allow(clippy::too_many_arguments)]
    pub(super) fn build_object(
        &self,
        store: &mut dyn NodeStore,
        kind: &str,
        fact_key: String,
        payload: CValue,
        input_refs: Built,
        source_coords: Built,
        generation: i64,
        cut: &str,
    ) -> Result<TypedObject, ProducerError> {
        self.typed_object(
            store,
            kind,
            fact_key,
            payload,
            input_refs,
            source_coords,
            generation,
            cut,
        )
    }

    /// #1394 C §5.3：按本构造器**当前档案 / 规则修订**重导一个泛型六字段 `obj3` 对象 id。
    ///
    /// 与 [`Cc004Producer::typed_object`] 共用同一份 [`Cc004Producer::typed_identity`] 与同一个
    /// [`object_id_of`]：审阅上一代记录时用它按**同一配方**重导 id，不在调用方抄第二份配方。
    /// 返回的只是 id 文本；对象是否自洽、载荷与输入引用是否可信由调用方按各自契约核。
    pub(super) fn typed_object_id(
        &self,
        kind: &str,
        fact_key: &str,
        payload: &Built,
        input_refs: &Built,
    ) -> Result<String, ProducerError> {
        Ok(object_id_of(
            &self.typed_identity(kind, fact_key, payload, input_refs)?,
        ))
    }

    /// §5.3：泛型身份的 `V({kind,fact_key,payload,input_refs,profile_id,rule_revision})`。
    fn typed_identity(
        &self,
        kind: &str,
        fact_key: &str,
        payload: &Built,
        input_refs: &Built,
    ) -> Result<NodeDigest, ProducerError> {
        self.typed_identity_scoped(kind, fact_key, payload, input_refs, None)
    }

    /// 生产与审计共享六字段配方；scope 只决定分配账，不改变身份。
    pub(super) fn typed_identity_scoped(
        &self,
        kind: &str,
        fact_key: &str,
        payload: &Built,
        input_refs: &Built,
        work: Option<&sc::capture_allocation::scratch::WorkScope>,
    ) -> Result<NodeDigest, ProducerError> {
        use sc::ValueView as V;
        let fields = [
            ("kind", V::String(kind)),
            ("fact_key", V::String(fact_key)),
            ("payload", V::Edge(payload.digest)),
            ("input_refs", V::Edge(input_refs.digest)),
            ("profile_id", V::String(&self.profile_id)),
            ("rule_revision", V::String(&self.rule_revision)),
        ];
        Ok(sc::write_value_view(&mut DigestOnly(work), V::Object(&fields))?.digest)
    }

    /// 只求身份对象的值摘要（不落节点表：身份对象是派生输入，不是要发布的节点）。
    /// CC-006 唯一十二字段身份配方。构造与只读审计共用；调用者分别从同核组证据/同cut组事实取价。
    pub(super) fn shape_identity(
        &self,
        part: ShapeIdentity<'_>,
    ) -> Result<NodeDigest, ProducerError> {
        self.shape_identity_borrowed(ShapeIdentityView {
            branch: part.branch,
            dir_ab: part.dir_ab,
            dir_bc: part.dir_bc,
            window: part.window.each_ref().map(String::as_str),
            highs: part.highs.each_ref().map(String::as_str),
            lows: part.lows.each_ref().map(String::as_str),
            comparisons: part.comparisons,
            input_refs: part.input_refs,
        })
    }

    /// 旧入口与只读审计共用唯一十二字段值配方，不复制字段文本或先造普通 JSON。
    pub(super) fn shape_identity_borrowed(
        &self,
        part: ShapeIdentityView<'_>,
    ) -> Result<NodeDigest, ProducerError> {
        self.shape_identity_scoped(part, None)
    }

    pub(super) fn shape_identity_scoped(
        &self,
        part: ShapeIdentityView<'_>,
        work: Option<&sc::capture_allocation::scratch::WorkScope>,
    ) -> Result<NodeDigest, ProducerError> {
        use sc::ValueView as V;
        let highs = part.highs.map(V::String);
        let lows = part.lows.map(V::String);
        let fields = [
            ("branch", V::String(part.branch)),
            ("dir_ab", V::String(part.dir_ab)),
            ("dir_bc", V::String(part.dir_bc)),
            ("window_start", V::String(part.window[0])),
            ("window_mid", V::String(part.window[1])),
            ("window_end", V::String(part.window[2])),
            ("merged_highs", V::Array(&highs)),
            ("merged_lows", V::Array(&lows)),
            ("comparisons", V::Edge(part.comparisons.digest)),
            ("input_refs", V::Edge(part.input_refs.digest)),
            ("rule_revision", V::String(&self.rule_revision)),
            ("profile_id", V::String(&self.profile_id)),
        ];
        let mut derived = DigestOnly(work);
        Ok(sc::write_value_view(&mut derived, V::Object(&fields))?.digest)
    }

    fn identity_summary(&self, identity: &CValue) -> Result<NodeDigest, ProducerError> {
        let mut derived = DigestOnly(None);
        Ok(sc::write_value(&mut derived, identity)?.digest)
    }

    /// 一条知识请求值（与全量配方 `request` 逐字段同值）：主体身份 + 四轴 + 原因 + 源坐标。
    ///
    /// `source_coords` 传已写出的坐标文本数组值根（同一数组可被多条请求复用）；
    /// `request_id` 是请求键的**规范 JSON 文本**，由同一规范渲染器给出，不另写一套转义。
    fn request_value(
        &self,
        key: CValue,
        subject_id: Option<&str>,
        reasons: &[&str],
        source_coords: &Built,
    ) -> Result<CValue, ProducerError> {
        let request_id = canonical_text(&key)?;
        let insufficient = reasons.iter().any(|reason| {
            matches!(
                *reason,
                "fewer_than_three_merged_bars"
                    | "no_subsequent_bar"
                    | "direction_not_yet_established"
            )
        });
        let axes = CValue::Object(vec![
            (
                "input_quality".to_string(),
                CValue::String(
                    if insufficient {
                        "INSUFFICIENT"
                    } else {
                        "SUFFICIENT"
                    }
                    .to_string(),
                ),
            ),
            (
                "applicability".to_string(),
                CValue::String(
                    if reasons.is_empty() {
                        "IN_DOMAIN"
                    } else {
                        "DOMAIN_PROOF_MISSING"
                    }
                    .to_string(),
                ),
            ),
            (
                "computation".to_string(),
                CValue::String(
                    if reasons.is_empty() {
                        "COMPLETE"
                    } else {
                        "INCOMPLETE"
                    }
                    .to_string(),
                ),
            ),
            (
                "validity".to_string(),
                CValue::String("CURRENT".to_string()),
            ),
        ]);
        Ok(CValue::Object(vec![
            ("request_id".to_string(), CValue::String(request_id)),
            (
                "subject_id".to_string(),
                match subject_id {
                    Some(id) => CValue::String(id.to_string()),
                    None => CValue::Null,
                },
            ),
            ("axes".to_string(), axes),
            (
                "reasons".to_string(),
                CValue::Array(
                    reasons
                        .iter()
                        .map(|reason| CValue::String((*reason).to_string()))
                        .collect(),
                ),
            ),
            (
                "source_coords".to_string(),
                CValue::Edge(source_coords.digest),
            ),
        ]))
    }

    /// 待决请求（主体为空）：键与原因由调用方按全量配方给出，本口不造默认值。
    ///
    /// 两类待决形态都用本口：竞争极值窗（键尾 = 三锚坐标数组）与不足三根的
    /// `pending`（键尾 = 字符串 `"pending"`）。
    pub(super) fn pending_request(
        &mut self,
        store: &mut dyn NodeStore,
        key: CValue,
        reasons: &[&'static str],
        sources: &CoordsRef,
    ) -> Result<Built, ProducerError> {
        let coords = self.coords_array(&mut *store, sources)?;
        let request = self.request_value(key, None, reasons, &coords)?;
        Ok(sc::write_value(&mut *store, &request)?)
    }

    /// 构造 CC-054 知识状态对象与观察（供正式 S3 生成态接线）。
    ///
    /// - `requests`：本代全部知识请求值边，**按线序**（步骤 → 组 → 待决形态 → 每个形态的
    ///   CC-006 请求与 CC-007 三条请求 → 不足三根的待决请求）。顺序进载荷内容，本口不重排；
    /// - `reasons`：本代全部等待原因（未排序、可有重复）——本口按与全量配方**同一条**
    ///   排序去重后再进载荷与观察；
    /// - `raw_coords`：本代全部原始坐标的**共享链**（生产入口用 [`raw_prefix_coords`] 借出；
    ///   观察的 `source_coords` 直接复用本对象的同一条边）；
    /// - `frontier`：`input_frontier`，规范十进制文本进载荷与观察。
    pub(super) fn build_knowledge_state(
        &mut self,
        store: &mut dyn NodeStore,
        raw_coords: &CoordsRef,
        frontier: i64,
        requests: &[Built],
        reasons: &[&'static str],
        generation: i64,
        cut: &str,
    ) -> Result<Cc054Assembly, ProducerError> {
        let mut waiting_reasons: Vec<&'static str> = reasons.to_vec();
        waiting_reasons.sort_unstable();
        waiting_reasons.dedup();
        let reasons_value = || {
            CValue::Array(
                waiting_reasons
                    .iter()
                    .map(|reason| CValue::String((*reason).to_string()))
                    .collect(),
            )
        };
        let payload_value = CValue::Object(vec![
            ("scope".to_string(), CValue::String("TB-02-A".to_string())),
            (
                "input_frontier".to_string(),
                CValue::String(frontier.to_string()),
            ),
            (
                "known_facts".to_string(),
                CValue::Array(vec![
                    CValue::String("raw_ohlc".to_string()),
                    CValue::String("inclusion_trace".to_string()),
                    CValue::String("group_provenance".to_string()),
                ]),
            ),
            ("waiting_reasons".to_string(), reasons_value()),
            (
                "domain".to_string(),
                CValue::String(tb02b::CURRENT_CONTRACT.knowledge_domain.to_string()),
            ),
            (
                "requests".to_string(),
                CValue::Array(
                    requests
                        .iter()
                        .map(|request| CValue::Edge(request.digest))
                        .collect(),
                ),
            ),
        ]);
        let fact_key = canonical_json(&json!([CC054_KNOWLEDGE_KIND, "TB-02-A"]));
        let input_refs = self.source_refs_array(&mut *store, raw_coords)?;
        let source_coords = self.coords_array(&mut *store, raw_coords)?;
        let object = self.typed_object(
            store,
            CC054_KNOWLEDGE_KIND,
            fact_key,
            payload_value,
            input_refs,
            source_coords,
            generation,
            cut,
        )?;
        let mut observations: Vec<Built> = Vec::with_capacity(waiting_reasons.len());
        for reason in &waiting_reasons {
            let reason_views = ReasonViews(&waiting_reasons);
            observations.push(write_knowledge_observation(
                store,
                reason,
                &frontier.to_string(),
                object.source_coords.digest,
                &reason_views,
            )?);
        }
        Ok(Cc054Assembly {
            object,
            observations,
        })
    }

    /// 一代 raw 账本的一次**受控准备**：排序、source 存在与每根 bar 的精确单元内容键
    /// 全在一次遍历里核完，并把自上次准备以来的源修订水位一次并入全部已保留窗口。
    ///
    /// - `Ok(view)`：视图借用 `raw_bars`（Rust 借用保证其存活期内 raw 不被改写）；
    ///   逐窗口 [`Cc004Producer::build_fractal`] 只做「按 dirty 二分截断 + append」，
    ///   不逐窗口重扫全 raw、不重算规范文本；
    /// - `Err(Type)`：`source_index` 非严格递增（与全量入口同一条拒绝，不近似定位）；
    /// - `Err(MissingSource)`：某根 raw 尚无当前源版本（不拿旧 `Value` 或空数组兜底）。
    ///
    /// 视图带 owner/epoch 见证：换 producer、或 prepare 之后 [`Cc004Producer::bind_source`]
    /// 实际改变任一源版本，旧视图在消费口**显式拒绝**（必须先重新 prepare）；
    /// 绕过 `bind_source` 的 raw 内容漂移在 prepare 中与上一份账本比较一次，按实际坐标
    /// 向全部保留窗口传播；成功重新 prepare 也使上一份视图失效。
    pub(super) fn prepare_raw_history<'a>(
        &mut self,
        raw_bars: &'a [Bar],
    ) -> Result<PreparedRawHistory<'a>, ProducerError> {
        if self.delta_binding.is_some() {
            return Err(ProducerError::Type {
                what: "raw 账本准备态",
                detail: "delta producer 不得切回无资格 prepare".into(),
            });
        }
        for pair in raw_bars.windows(2) {
            if pair[0].source_index >= pair[1].source_index {
                return Err(ProducerError::Type {
                    what: "原始账顺序",
                    detail: format!(
                        "source_index 必须严格递增：{} 与 {}",
                        pair[0].source_index, pair[1].source_index
                    ),
                });
            }
        }
        let mut keys = Vec::with_capacity(raw_bars.len());
        let mut changed_floor = self.unprepared_floor;
        let mut first_difference = None;
        for (index, bar) in raw_bars.iter().enumerate() {
            let coord = i64::try_from(bar.source_index).map_err(|_| {
                ProducerError::Overflow(format!("源坐标 {} 超出 i64", bar.source_index))
            })?;
            let (revision, input_revision) = match self.sources.get(&coord) {
                Some((spec, _)) => (spec.revision, spec.input_revision.as_str()),
                None => return Err(ProducerError::MissingSource(coord)),
            };
            let key = fractal_unit_key(coord, revision, input_revision, bar)?;
            if first_difference.is_none() {
                match self.prepared_raw_keys.get(index) {
                    Some((old_coord, old_key))
                        if *old_coord == bar.source_index && old_key == &key => {}
                    previous => {
                        // 插入/替换同时取旧新坐标较小值，坐标稀疏时也不漏失效窗口。
                        first_difference =
                            Some(previous.map_or(coord, |(old, _)| coord.min(*old as i64)));
                    }
                }
            }
            keys.push((bar.source_index, key));
        }
        if first_difference.is_none() && self.prepared_raw_keys.len() > raw_bars.len() {
            first_difference = Some(self.prepared_raw_keys[raw_bars.len()].0 as i64);
        }
        if let Some(changed) = first_difference {
            changed_floor = Some(changed_floor.map_or(changed, |floor| floor.min(changed)));
        }
        // 完整检查成功之后才修改准备态；失败不会消耗旧水位或伪造新见证。
        if let Some(floor) = changed_floor {
            for history in self.fractal_histories.values_mut() {
                history.floor = Some(history.floor.map_or(floor, |old| old.min(floor)));
            }
        }
        self.prepared_raw_keys = keys;
        self.unprepared_floor = None;
        self.prepared_snapshot = Rc::new(());
        Ok(PreparedRawHistory {
            owner: Rc::clone(&self.raw_token),
            epoch: self.source_epoch,
            snapshot: Rc::clone(&self.prepared_snapshot),
            raw: super::s_fact_cache::RawBars::Slice(raw_bars),
            delta: None,
        })
    }

    /// #1488：来源更新与 prepare 是同一次 bound 消费；无调用者自报 dirty/prefix 入口。
    pub(super) fn prepare_raw_history_delta<'a>(
        &mut self,
        store: &mut dyn NodeStore,
        view: super::s_fact_cache::EffectiveRawView<'a>,
    ) -> Result<PreparedRawHistory<'a>, ProducerError> {
        let result = self.prepare_raw_history_delta_inner(store, view);
        if result.is_err() {
            view.bound().invalidate_after_consumer_failure();
            self.discard_runtime_state();
        }
        result
    }

    fn prepare_raw_history_delta_inner<'a>(
        &mut self,
        store: &mut dyn NodeStore,
        view: super::s_fact_cache::EffectiveRawView<'a>,
    ) -> Result<PreparedRawHistory<'a>, ProducerError> {
        let invalid = |detail: String| ProducerError::Type {
            what: "delta raw 准备态",
            detail,
        };
        view.ensure_current().map_err(invalid)?;
        let bound = view.bound();
        let matches = match self.delta_binding {
            Some(old) => {
                old == bound.previous_binding() && self.delta_epoch == Some(self.source_epoch)
            }
            None => {
                bound.previous_binding().is_initial()
                    && self.sources.is_empty()
                    && self.prepared_raw_keys.is_empty()
            }
        };
        if !matches || self.prepared_raw_keys.len() != view.previous_len() {
            return Err(invalid(
                "producer 前代 binding/epoch/长度不符；须丢弃".into(),
            ));
        }
        // 先撤所有移走坐标，才能正确消费同 cut 坐标互换；原位修订仍走 bind_source 同核。
        for edit in bound.winner_edits() {
            if let Some(old) = edit.old {
                if edit.new.is_none_or(|new| new.coord != old.coord) {
                    if self.sources.remove(&old.coord).is_none()
                        || self.shape_sources.remove(&old.coord).is_none()
                    {
                        return Err(invalid("旧 winner 来源登记缺失".into()));
                    }
                    self.ref_arrays.invalidate(old.coord);
                    self.shape_ref_arrays.invalidate(old.coord);
                    self.source_epoch += 1;
                    self.unprepared_floor = Some(
                        self.unprepared_floor
                            .map_or(old.coord, |f| f.min(old.coord)),
                    );
                }
            }
        }
        for edit in bound.winner_edits() {
            if let Some(new) = edit.new {
                let raw = bound
                    .raw()
                    .get(new.ordinal)
                    .ok_or_else(|| invalid("新 winner 原账缺失".into()))?;
                self.bind_source(
                    store,
                    new.coord,
                    RawRefSpec::from_event(raw).map_err(invalid)?,
                )?;
            }
        }
        let first = view.dirty_from();
        if first > self.prepared_raw_keys.len() {
            return Err(invalid("prepared 后缀越过旧账".into()));
        }
        let mut keys = Vec::with_capacity(view.len() - first);
        for bar in view.bars().range(first..view.len()).iter() {
            let coord = bar.source_index as i64;
            let (spec, _) = self
                .sources
                .get(&coord)
                .ok_or(ProducerError::MissingSource(coord))?;
            keys.push((
                bar.source_index,
                fractal_unit_key(coord, spec.revision, &spec.input_revision, bar)?,
            ));
        }
        if let Some(floor) = self.unprepared_floor {
            for history in self.fractal_histories.values_mut() {
                history.floor = Some(history.floor.map_or(floor, |old| old.min(floor)));
            }
        }
        self.prepared_raw_keys.truncate(first);
        self.prepared_raw_keys.extend(keys);
        self.unprepared_floor = None;
        self.delta_binding = Some(bound.current_binding());
        self.delta_epoch = Some(self.source_epoch);
        self.prepared_snapshot = Rc::new(());
        Ok(PreparedRawHistory {
            owner: Rc::clone(&self.raw_token),
            epoch: self.source_epoch,
            snapshot: Rc::clone(&self.prepared_snapshot),
            raw: view.bars(),
            delta: Some(view),
        })
    }

    pub(super) fn check_effective_view(
        &self,
        view: super::s_fact_cache::EffectiveRawView<'_>,
    ) -> Result<(), ProducerError> {
        view.ensure_current()
            .map_err(|detail| ProducerError::Type {
                what: "delta raw 准备态",
                detail,
            })?;
        if self.delta_binding != Some(view.bound().current_binding())
            || self.delta_epoch != Some(self.source_epoch)
        {
            return Err(ProducerError::Type {
                what: "delta raw 准备态",
                detail: "当前来源绑定或 epoch 不符".into(),
            });
        }
        Ok(())
    }

    /// 逐窗口便利口（测试/单窗调用）：内部 prepare 后立即构造。
    ///
    /// 正式生成入口应「一代一次 [`Cc004Producer::prepare_raw_history`] + 逐窗口
    /// [`Cc004Producer::build_fractal`]」，否则每个窗口都要重扫一遍全 raw。
    pub(super) fn build_fractal_raw(
        &mut self,
        store: &mut dyn NodeStore,
        raw_bars: &[Bar],
        shape: &Cc006Object,
        views: [&GroupEvidenceView<'_>; 3],
        generation: i64,
        cut: &str,
    ) -> Result<Option<Cc007Object>, ProducerError> {
        let prepared = self.prepare_raw_history(raw_bars)?;
        self.build_fractal(store, &prepared, shape, views, generation, cut)
    }

    /// 构造 CC-007 分型描述对象（三条知识请求一并给出）。
    ///
    /// - `prepared`：本代 raw 账本的准备态（见 [`Cc004Producer::prepare_raw_history`]）；
    ///   后续历史 = 全部坐标 > 右组末成员的原始 bar（与全量配方 `raw.range(Excluded(end),..)`
    ///   同一条），未分组的 pending raw 也算在内；
    /// - 后续历史按参考集指纹 + 源坐标 + **精确内容键**（revision/四价）续接：新 bar 只
    ///   整段接尾，源修订/raw 漂移按实际坐标截断；二者合为一次 `sequence_rebuild_suffix`；
    /// - `Ok(None)`：四分支不是 `TOP`/`BOTTOM`（旧路径对这些分支不产 CC-007 对象）。
    pub(super) fn build_fractal(
        &mut self,
        store: &mut dyn NodeStore,
        prepared: &PreparedRawHistory<'_>,
        shape: &Cc006Object,
        views: [&GroupEvidenceView<'_>; 3],
        generation: i64,
        cut: &str,
    ) -> Result<Option<Cc007Object>, ProducerError> {
        self.build_fractal_impl(store, prepared, shape, views, generation, cut, None)
    }

    fn check_prepared(&self, prepared: &PreparedRawHistory<'_>) -> Result<(), ProducerError> {
        if let Some(view) = prepared.delta {
            self.check_effective_view(view)?;
        }
        if !Rc::ptr_eq(&prepared.owner, &self.raw_token) {
            return Err(ProducerError::Type {
                what: "raw 账本准备态",
                detail: "owner 不符：该视图不属于本 producer".to_string(),
            });
        }
        if prepared.epoch != self.source_epoch {
            return Err(ProducerError::Type {
                what: "raw 账本准备态",
                detail: format!(
                    "source 修订已推进（prepare epoch {} ≠ 当前 {}）：陈旧视图必须重新 prepare",
                    prepared.epoch, self.source_epoch
                ),
            });
        }
        if !Rc::ptr_eq(&prepared.snapshot, &self.prepared_snapshot) {
            return Err(ProducerError::Type {
                what: "raw 账本准备态",
                detail: "raw 账本已重新准备：旧视图必须重取".to_string(),
            });
        }
        Ok(())
    }

    /// 同次 parser 的完整 raw 前缀与 prepared 账逐元素相同才建立资格。
    /// 不用最大坐标/数量猜等价；state 借用阻止该次 parser 快照在 session 内推进。
    pub(super) fn parser_fractal_sources<'a>(
        &self,
        prepared: &PreparedRawHistory<'_>,
        state: &'a IncrFactsState,
    ) -> Result<ParserFractalSources<'a>, ProducerError> {
        self.check_prepared(prepared)?;
        // 仍让共享证据口执行原来的 Degraded 判据。
        state
            .shared_group(0)
            .map_err(|error| ProducerError::Degraded(error.reason))?;
        let same_parser = match prepared.delta {
            Some(view) => std::ptr::eq(state, view.state()),
            None => state.raw_iter().eq(prepared.raw.iter()),
        };
        if !same_parser {
            return Err(ProducerError::Type {
                what: "parser raw 账本",
                detail: "同次 parser 与 prepared raw 必须逐元素相同".to_string(),
            });
        }
        Ok(ParserFractalSources {
            state,
            owner: Rc::clone(&prepared.owner),
            epoch: prepared.epoch,
            snapshot: Rc::clone(&prepared.snapshot),
            call: Rc::new(()),
            arrays: None,
            #[cfg(test)]
            whole_source_builds: 0,
        })
    }

    fn check_parser_sources(
        &self,
        prepared: &PreparedRawHistory<'_>,
        sources: &ParserFractalSources<'_>,
    ) -> Result<(), ProducerError> {
        self.check_prepared(prepared)?;
        if !Rc::ptr_eq(&sources.owner, &prepared.owner)
            || sources.epoch != prepared.epoch
            || !Rc::ptr_eq(&sources.snapshot, &prepared.snapshot)
        {
            return Err(ProducerError::Type {
                what: "parser raw 账本",
                detail: "session 不属于当前 prepared 快照".to_string(),
            });
        }
        Ok(())
    }

    pub(super) fn build_parser_shape(
        &mut self,
        store: &mut dyn NodeStore,
        prepared: &PreparedRawHistory<'_>,
        sources: &ParserFractalSources<'_>,
        start: usize,
        generation: i64,
        cut: &str,
    ) -> Result<Option<ParserFractalShape>, ProducerError> {
        self.check_parser_sources(prepared, sources)?;
        Ok(self
            .build_shape(store, sources.state, start, generation, cut)?
            .map(|object| ParserFractalShape {
                object,
                start,
                call: Rc::clone(&sources.call),
            }))
    }

    pub(super) fn build_parser_fractal(
        &mut self,
        store: &mut dyn NodeStore,
        prepared: &PreparedRawHistory<'_>,
        sources: &mut ParserFractalSources<'_>,
        shape: &ParserFractalShape,
        generation: i64,
        cut: &str,
    ) -> Result<Option<Cc007Object>, ProducerError> {
        self.check_parser_sources(prepared, sources)?;
        if !Rc::ptr_eq(&sources.call, &shape.call) {
            return Err(ProducerError::Type {
                what: "parser 分型窗口",
                detail: "形态不是同次 session 的真实 parser 窗口".to_string(),
            });
        }
        let state = sources.state;
        let views = [shape.start, shape.start + 1, shape.start + 2].map(|index| {
            state
                .shared_group(index)
                .map_err(|error| ProducerError::Degraded(error.reason))?
                .ok_or(ProducerError::Type {
                    what: "parser 分型窗口",
                    detail: "已构造形态缺少同次共享组".to_string(),
                })
        });
        let [left, mid, right] = views;
        self.build_fractal_impl(
            store,
            prepared,
            &shape.object,
            [&left?, &mid?, &right?],
            generation,
            cut,
            Some(sources),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build_fractal_impl(
        &mut self,
        store: &mut dyn NodeStore,
        prepared: &PreparedRawHistory<'_>,
        shape: &Cc006Object,
        views: [&GroupEvidenceView<'_>; 3],
        generation: i64,
        cut: &str,
        mut shared: Option<&mut ParserFractalSources<'_>>,
    ) -> Result<Option<Cc007Object>, ProducerError> {
        self.check_prepared(prepared)?;
        let raw_bars = prepared.raw;
        if !matches!(shape.branch, "TOP" | "BOTTOM") {
            return Ok(None);
        }
        let [left, mid, right] = views;
        let end = right.members.last().copied().ok_or(ProducerError::Type {
            what: "CC-007 右组",
            detail: "组成员为空：不接受无来源的分型窗口".to_string(),
        })?;
        // 严格递增序与 source 存在已由 `prepare_raw_history` 一次核完（不在此逐窗口重扫）。
        // 六个参考价（三组 × 高/低；名字/轴与全量配方逐字相同）。
        let mut references: Vec<CValue> = Vec::with_capacity(6);
        let mut reference_meta: Vec<(String, i64)> = Vec::with_capacity(6);
        for (label, view) in [("left", left), ("mid", mid), ("right", right)] {
            for (axis, price) in [("high", view.bar.high), ("low", view.bar.low)] {
                let name = format!("{label}.{axis}");
                references.push(CValue::Object(vec![
                    ("name".to_string(), CValue::String(name.clone())),
                    (
                        "source_coord".to_string(),
                        CValue::String(view.bar.source_index.to_string()),
                    ),
                    ("axis".to_string(), CValue::String(axis.to_string())),
                    ("value".to_string(), CValue::String(price.to_string())),
                ]));
                reference_meta.push((name, price));
            }
        }
        let references_value = CValue::Array(references);
        let refs_key = canonical_text(&references_value)?;
        // 后续历史：坐标 > 右组末成员的全部原始 bar（升序表上做分区定位）。
        let tail_start = raw_bars.partition_point(|bar| bar.source_index <= end);
        let tail_len = raw_bars.len() - tail_start;
        // 右组可能已被下一组封口：`conf.coords` 的末项就是下一组锚 = tail 首坐标（tail 的
        // 其余坐标都严格更大）⟹ 并集增量跳掉这一项，等价原配方 `sources` 的排序去重；
        // 不能拿「取最大链」替代，也不能把重复坐标发进源引用/坐标数组。
        let tail_first = raw_bars.get(tail_start).map(|bar| bar.source_index);
        let union_skip_first = tail_first.is_some() && shape.sources.max() == tail_first;
        // prepare 已把实际 source/raw 变化一次传播到每个窗口；这里仅按坐标二分截断。
        let cached = self.fractal_histories.remove(&(refs_key.clone(), end));
        // 截断不先落 split 前缀根：保留数与新尾在循环后一次 `sequence_rebuild_suffix`（#1488 S3）。
        let (previous, mut covered, mut union_tail) = match cached {
            Some(history) if history.tail_start == tail_start && history.count <= tail_len => {
                let count = history.count;
                let keep = history.floor.map_or(count, |floor| {
                    raw_bars
                        .range(tail_start..tail_start + count)
                        .partition_point(|bar| {
                            // prepare 已核每个实际坐标可表达为 i64，借用期内 raw 不变。
                            (bar.source_index as i64) < floor
                        })
                });
                let sequence = history.sequence;
                let mut union_tail = history.union_tail;
                if keep < count {
                    self.history_invalidated += count - keep;
                    // 只在实际修订时重建保留的坐标尾；普通追加保留共享链。
                    let mut kept: Vec<usize> = raw_bars
                        .range(tail_start..tail_start + keep)
                        .iter()
                        .map(|bar| bar.source_index)
                        .collect();
                    if union_skip_first && !kept.is_empty() {
                        kept.remove(0);
                    }
                    union_tail = coords_ref_from_slice(&kept);
                }
                self.history_reused += keep;
                (sequence, keep, union_tail)
            }
            Some(history) => {
                self.history_invalidated += history.count;
                (SequenceRef::empty(), 0, CoordsRef::empty())
            }
            None => (SequenceRef::empty(), 0, CoordsRef::empty()),
        };
        let appended_from = covered;
        let mut leaves: Vec<NodeDigest> = Vec::with_capacity(tail_len - appended_from);
        for (offset, bar) in raw_bars
            .range(tail_start + appended_from..raw_bars.len())
            .iter()
            .enumerate()
        {
            let relations = fractal_relations(bar, &reference_meta);
            let relations = {
                #[cfg(test)]
                let _phase = tests::emission_phase("fractal_relations_write_value");
                let value = CValue::Array(relations);
                #[cfg(feature = "s_capture_native_testonly")]
                let built = if shared.is_some() {
                    sc::write_value_native_local_for_testonly(&mut *store, &value)?
                } else {
                    sc::write_value(&mut *store, &value)?
                };
                #[cfg(not(feature = "s_capture_native_testonly"))]
                let built = sc::write_value(&mut *store, &value)?;
                built
            };
            let unit = CValue::Object(vec![
                ("bar".to_string(), bar_value(bar)),
                ("relations".to_string(), CValue::Edge(relations.digest)),
            ]);
            let leaf = {
                #[cfg(test)]
                let _phase = tests::emission_phase("fractal_unit_write_value");
                sc::write_value(&mut *store, &unit)?.digest
            };
            leaves.push(leaf);
            let coord_index = tail_start + appended_from + offset;
            if !(union_skip_first && coord_index == tail_start) {
                union_tail = union_tail.append_greater(bar.source_index);
            }
            covered += 1;
        }
        let sequence = {
            #[cfg(test)]
            let _phase = tests::emission_phase("fractal_sequence_append");
            sc::sequence_rebuild_suffix(&mut *store, &previous, appended_from as u64, &leaves)?
        };
        self.history_computed += covered - appended_from;
        let units_count = u64::try_from(covered)
            .map_err(|_| ProducerError::Overflow(format!("后续单元数 {covered} 超出 u64")))?;
        let units_array = Built {
            digest: {
                #[cfg(test)]
                let _phase = tests::emission_phase("fractal_units_array");
                put_array_node(&mut *store, units_count, &sequence.root())?
            },
            kind: Kind::Array,
            extent: Some(units_count),
        };
        let no_future = covered == 0;
        self.fractal_histories.insert(
            (refs_key, end),
            FractalHistory {
                end,
                tail_start,
                count: covered,
                floor: None,
                union_tail: union_tail.clone(),
                sequence,
            },
        );
        // 本对象的来源 = CC-006 窗来源链 ++ 后续 tail 的**去重增量**（首项若已由封口链
        // 覆盖即跳过；两段严格升序且不交叠）。
        let all_sources = CoordsRef::concat(&shape.sources, &union_tail);
        // `descriptive_labels.raw_ohlc`：三组全部成员的原始 bar（按 左→中→右、组内升序）。
        let mut label_sources: Vec<usize> = Vec::new();
        let mut raw_ohlc: Vec<CValue> = Vec::new();
        for view in [left, mid, right] {
            for coord in view.members {
                let bar = raw_bar_at(raw_bars, *coord)?;
                label_sources.push(*coord);
                raw_ohlc.push(bar_value(bar));
            }
        }
        let raw_ohlc = sc::write_value(&mut *store, &CValue::Array(raw_ohlc))?;
        let branch = shape.branch;
        let payload_value = CValue::Object(vec![
            (
                "shape_object_id".to_string(),
                CValue::String(shape.object_id.clone()),
            ),
            ("branch".to_string(), CValue::String(branch.to_string())),
            (
                "window".to_string(),
                CValue::Array(
                    shape
                        .window
                        .iter()
                        .map(|coord| CValue::String(coord.to_string()))
                        .collect(),
                ),
            ),
            (
                "description".to_string(),
                CValue::String(
                    if branch == "TOP" {
                        "中组高点和低点均严格高于相邻两组"
                    } else {
                        "中组高点和低点均严格低于相邻两组"
                    }
                    .to_string(),
                ),
            ),
            ("reference_prices".to_string(), references_value),
            (
                "descriptive_labels".to_string(),
                CValue::Object(vec![
                    (
                        "status".to_string(),
                        CValue::String("not_determined".to_string()),
                    ),
                    (
                        "reason".to_string(),
                        CValue::String("no_settled_numeric_rule".to_string()),
                    ),
                    ("labels".to_string(), CValue::Array(Vec::new())),
                    (
                        "source".to_string(),
                        CValue::String("fenxing.md:64-74".to_string()),
                    ),
                    ("raw_ohlc".to_string(), CValue::Edge(raw_ohlc.digest)),
                ]),
            ),
            (
                "subsequent_development".to_string(),
                CValue::Object(vec![
                    (
                        "status".to_string(),
                        CValue::String(
                            if no_future {
                                "insufficient_knowledge"
                            } else {
                                "observed"
                            }
                            .to_string(),
                        ),
                    ),
                    (
                        "reason".to_string(),
                        if no_future {
                            CValue::String("no_subsequent_bar".to_string())
                        } else {
                            CValue::Null
                        },
                    ),
                    ("bars".to_string(), CValue::Edge(units_array.digest)),
                ]),
            ),
        ]);
        let window_text: Vec<String> = shape.window.iter().map(ToString::to_string).collect();
        let fact_key = canonical_json(&json!([CC007_FRACTAL_KIND, window_text.clone()]));
        // 同 kernel 的方向/封口证据递推完整 raw 前缀；右组 sources 加去重 tail
        // 恰为完整生效 raw（含稀疏坐标）。只有不可伪造的同次 parser 窗承载此命题。
        // 首窗仍在原位置走原 array 构造（含原 put）；成功后如 ArrayCache 一样仅 memo
        // 生成结果。它不授予 checked frame，不能省写前/后续审计或发布闭包核。
        let (input_refs, source_coords) = if let Some(session) = shared.as_deref_mut() {
            match &session.arrays {
                Some(arrays) => arrays.clone(),
                None => {
                    let input_refs = self.source_refs_array(&mut *store, &all_sources)?;
                    let source_coords = self.coords_array(&mut *store, &all_sources)?;
                    session.arrays = Some((input_refs.clone(), source_coords.clone()));
                    #[cfg(test)]
                    {
                        session.whole_source_builds += 1;
                    }
                    (input_refs, source_coords)
                }
            }
        } else {
            (
                self.source_refs_array(&mut *store, &all_sources)?,
                self.coords_array(&mut *store, &all_sources)?,
            )
        };
        let object = self.typed_object(
            store,
            CC007_FRACTAL_KIND,
            fact_key,
            payload_value,
            input_refs,
            source_coords,
            generation,
            cut,
        )?;
        let request_reasons: Vec<&'static str> = if no_future {
            vec!["no_subsequent_bar"]
        } else {
            Vec::new()
        };
        let slot_key = || {
            CValue::Array(vec![
                CValue::String(CC007_FRACTAL_KIND.to_string()),
                CValue::Array(
                    window_text
                        .iter()
                        .map(|coord| CValue::String(coord.clone()))
                        .collect(),
                ),
            ])
        };
        let mut requests: Vec<Built> = Vec::with_capacity(3);
        let label_coords = coords_ref_from_slice(&label_sources);
        for (label, reasons, sources) in [
            ("shape_description", Vec::new(), shape.sources.clone()),
            (
                "descriptive_labels",
                vec!["no_settled_numeric_rule"],
                label_coords,
            ),
            (
                "subsequent_development",
                request_reasons.clone(),
                all_sources.clone(),
            ),
        ] {
            let key = CValue::Array(vec![slot_key(), CValue::String(label.to_string())]);
            let coords = if shared.is_some() && label == "subsequent_development" {
                object.source_coords.clone()
            } else {
                self.coords_array(&mut *store, &sources)?
            };
            let request = self.request_value(key, Some(&object.object_id), &reasons, &coords)?;
            requests.push(sc::write_value(&mut *store, &request)?);
        }
        let mut knowledge_reasons = request_reasons.clone();
        knowledge_reasons.push("no_settled_numeric_rule");
        Ok(Some(Cc007Object {
            object_id: object.object_id.clone(),
            identity: object.identity,
            fact_key: object.fact_key.clone(),
            record: object.record.clone(),
            payload: object.payload.clone(),
            input_refs: object.input_refs.clone(),
            source_coords: object.source_coords.clone(),
            payload_json: object.payload_json.clone(),
            input_refs_json: object.input_refs_json.clone(),
            source_coords_json: object.source_coords_json.clone(),
            requests,
            request_reasons,
            knowledge_reasons,
            generation,
            cut: cut.to_string(),
        }))
    }

    /// 从同次 parser 的共享态借出三组窗口并构造 CC-006 形态对象。
    ///
    /// - `Ok(None)`：`start + 2` 越界（没有该窗口）；#1405 多根极值身份不挡形态发布。
    /// - `Err(Degraded)`：共享态退化，**先于索引判定**；
    /// - `Err(WindowNotClassified)`：窗口不在 CC-006 可判域内（不造第五分型、不回落旧 Value）。
    pub(super) fn build_shape(
        &mut self,
        store: &mut dyn NodeStore,
        state: &IncrFactsState,
        start: usize,
        generation: i64,
        cut: &str,
    ) -> Result<Option<Cc006Object>, ProducerError> {
        let left = state
            .shared_group(start)
            .map_err(|error| ProducerError::Degraded(error.reason))?;
        let mid = state
            .shared_group(start + 1)
            .map_err(|error| ProducerError::Degraded(error.reason))?;
        let right = state
            .shared_group(start + 2)
            .map_err(|error| ProducerError::Degraded(error.reason))?;
        let (Some(left), Some(mid), Some(right)) = (left, mid, right) else {
            return Ok(None);
        };
        let views = [left, mid, right];
        self.build_shape_object(
            store,
            [start, start + 1, start + 2],
            [&views[0], &views[1], &views[2]],
            generation,
            cut,
        )
        .map(Some)
    }

    /// 从三组借用视图构造 CC-006 形态对象（`build_shape` 的视图面）。
    ///
    /// 四分支与四个严格比较**直接调用** [`classify_local_shape`]（同一判据，不另写一套）；
    /// 三组窗口分类的前提与旧生成路径逐条相同（组 bar 即 merged K、组下标即 merged 下标）。
    pub(super) fn build_shape_object(
        &mut self,
        store: &mut dyn NodeStore,
        indices: [usize; 3],
        views: [&GroupEvidenceView<'_>; 3],
        generation: i64,
        cut: &str,
    ) -> Result<Cc006Object, ProducerError> {
        let [left, mid, right] = views;
        let window = [
            left.bar.source_index,
            mid.bar.source_index,
            right.bar.source_index,
        ];
        let classified = classify_local_shape(&left.bar, &mid.bar, &right.bar);
        let Cc006LocalShape::Classified {
            branch,
            dir_ab,
            dir_bc,
            comparisons,
        } = classified
        else {
            let reason = match classified {
                Cc006LocalShape::DomainNotSatisfied { reason } => reason.as_str(),
                Cc006LocalShape::InsufficientKnowledge => "insufficient_knowledge",
                Cc006LocalShape::Classified { .. } => unreachable!("刚判过非 Classified"),
            };
            return Err(ProducerError::WindowNotClassified(reason));
        };
        let comparisons_value = CValue::Array(
            comparisons
                .iter()
                .map(|comparison| {
                    CValue::Object(vec![
                        (
                            "axis".to_string(),
                            CValue::String(comparison.axis.as_str().to_string()),
                        ),
                        (
                            "pair".to_string(),
                            CValue::String(comparison.pair.as_str().to_string()),
                        ),
                        (
                            "prev".to_string(),
                            CValue::String(comparison.prev.to_string()),
                        ),
                        (
                            "cur".to_string(),
                            CValue::String(comparison.cur.to_string()),
                        ),
                        ("strict_up".to_string(), CValue::Bool(comparison.strict_up)),
                        ("held".to_string(), CValue::Bool(comparison.held)),
                    ])
                })
                .collect(),
        );
        let comparisons = sc::write_value(&mut *store, &comparisons_value)?;
        // 两组来源引用，同一次生成、同一份坐标集合与 dependency 计算，只换叶编码与 memo：
        // - `groups`（数值叶）：旧生成态，进十二字段身份、记录边与 `input_refs_json` 内部值；
        // - `wire_groups`（文本叶 + 字符串 merged_index）：公开 wire 的 `input_refs`（旧
        //   `project_input_refs` 投影结果），不参与身份。
        let mut groups: Vec<CValue> = Vec::with_capacity(3);
        let mut wire_groups: Vec<CValue> = Vec::with_capacity(3);
        // 三组来源并集 = 右组来源链（归纳嵌套前缀性；逐前缀对拍见测试）。
        let sources = group_sources_coords(right);
        for (offset, view) in [left, mid, right].iter().enumerate() {
            let index = i64::try_from(indices[offset]).map_err(|_| {
                ProducerError::Overflow(format!("组下标 {} 超出 i64", indices[offset]))
            })?;
            let members = group_members_coords(view);
            let shape_raw =
                self.source_refs_array_in(&mut *store, &members, RefEncoding::ShapeInteger)?;
            let shape_dependency =
                self.group_dependency_array(&mut *store, view, RefEncoding::ShapeInteger)?;
            groups.push(CValue::Object(vec![
                ("merged_index".to_string(), number(index)?),
                ("raw_refs".to_string(), CValue::Edge(shape_raw.digest)),
                (
                    "dependency_refs".to_string(),
                    CValue::Edge(shape_dependency.digest),
                ),
            ]));
            let wire_raw =
                self.source_refs_array_in(&mut *store, &members, RefEncoding::WireText)?;
            let wire_dependency =
                self.group_dependency_array(&mut *store, view, RefEncoding::WireText)?;
            wire_groups.push(CValue::Object(vec![
                (
                    "merged_index".to_string(),
                    CValue::String(index.to_string()),
                ),
                ("raw_refs".to_string(), CValue::Edge(wire_raw.digest)),
                (
                    "dependency_refs".to_string(),
                    CValue::Edge(wire_dependency.digest),
                ),
            ]));
        }
        let input_refs = sc::write_value(&mut *store, &CValue::Array(groups))?;
        let wire_input_refs = sc::write_value(&mut *store, &CValue::Array(wire_groups))?;
        let input_refs_json = Built {
            digest: put_json_text_node(&mut *store, &input_refs.digest)?,
            kind: Kind::JsonText,
            extent: None,
        };
        let source_coords = self.coords_array(&mut *store, &sources)?;
        let source_coords_json = Built {
            digest: put_json_text_node(&mut *store, &source_coords.digest)?,
            kind: Kind::JsonText,
            extent: None,
        };
        let identity = self.shape_identity(ShapeIdentity {
            branch: branch.as_str(),
            dir_ab: dir_str(dir_ab),
            dir_bc: dir_str(dir_bc),
            window: window.map(|coord| coord.to_string()),
            highs: [left.bar.high, mid.bar.high, right.bar.high].map(|price| price.to_string()),
            lows: [left.bar.low, mid.bar.low, right.bar.low].map(|price| price.to_string()),
            comparisons: &comparisons,
            input_refs: &input_refs,
        })?;
        let object_id = object_id_of(&identity);
        let record_value = CValue::Object(vec![
            ("object_id".to_string(), CValue::String(object_id.clone())),
            ("object_revision".to_string(), number(OBJECT_REVISION)?),
            (
                "kind".to_string(),
                CValue::String(CC006_SHAPE_KIND.to_string()),
            ),
            (
                "branch".to_string(),
                CValue::String(branch.as_str().to_string()),
            ),
            (
                "window_start".to_string(),
                number(i64::try_from(window[0]).map_err(|_| {
                    ProducerError::Overflow(format!("源坐标 {} 超出 i64", window[0]))
                })?)?,
            ),
            (
                "window_mid".to_string(),
                number(i64::try_from(window[1]).map_err(|_| {
                    ProducerError::Overflow(format!("源坐标 {} 超出 i64", window[1]))
                })?)?,
            ),
            (
                "window_end".to_string(),
                number(i64::try_from(window[2]).map_err(|_| {
                    ProducerError::Overflow(format!("源坐标 {} 超出 i64", window[2]))
                })?)?,
            ),
            (
                "dir_ab".to_string(),
                CValue::String(dir_str(dir_ab).to_string()),
            ),
            (
                "dir_bc".to_string(),
                CValue::String(dir_str(dir_bc).to_string()),
            ),
            ("comparisons".to_string(), CValue::Edge(comparisons.digest)),
            ("input_refs".to_string(), CValue::Edge(input_refs.digest)),
            (
                "source_coords".to_string(),
                CValue::Edge(source_coords.digest),
            ),
            (
                "input_refs_json".to_string(),
                CValue::Edge(input_refs_json.digest),
            ),
            (
                "source_coords_json".to_string(),
                CValue::Edge(source_coords_json.digest),
            ),
            ("first_known_generation".to_string(), number(generation)?),
            (
                "first_known_cut".to_string(),
                CValue::String(cut.to_string()),
            ),
            ("published_generation".to_string(), number(generation)?),
        ]);
        let record = sc::write_value(&mut *store, &record_value)?;
        let key = CValue::Array(vec![
            CValue::String(CC006_SHAPE_KIND.to_string()),
            CValue::Array(
                window
                    .iter()
                    .map(|coord| CValue::String(coord.to_string()))
                    .collect(),
            ),
        ]);
        let request = self.request_value(key, Some(&object_id), &[], &source_coords)?;
        let request = sc::write_value(&mut *store, &request)?;
        Ok(Cc006Object {
            object_id,
            identity,
            record,
            comparisons,
            input_refs,
            wire_input_refs,
            source_coords,
            input_refs_json,
            source_coords_json,
            sources,
            window,
            request,
            branch: branch.as_str(),
            generation,
            cut: cut.to_string(),
        })
    }

    /// 证据对象（方向/封口共用同一形状）：与全量配方 `direction` 逐字段同值，
    /// `source_coords` 只以共享 `CoordsRef` 成边，不展开成 `InclusionDirectionEvidence`。
    fn evidence_value(
        &mut self,
        store: &mut dyn NodeStore,
        node: &Rc<EvidenceNode>,
    ) -> Result<CValue, ProducerError> {
        let coords = self.coords_array(&mut *store, &node.coords)?;
        Ok(CValue::Object(vec![
            (
                "direction".to_string(),
                CValue::String(dir_str(node.direction).to_string()),
            ),
            (
                "established_at".to_string(),
                CValue::String(node.incoming.source_index.to_string()),
            ),
            ("previous_acc".to_string(), bar_value(&node.previous_acc)),
            ("incoming".to_string(), bar_value(&node.incoming)),
            ("source_coords".to_string(), CValue::Edge(coords.digest)),
        ]))
    }

    /// 从同次 parser 的共享态借出单组证据并构造 CC-005 组对象。
    ///
    /// 契约与 [`Cc004Producer::build_step`] 一致：`Ok(None)` = 该索引没有组；
    /// `Err(Degraded)` = 共享态退化，**先于索引判定**。
    pub(super) fn build_group(
        &mut self,
        store: &mut dyn NodeStore,
        state: &IncrFactsState,
        index: usize,
        generation: i64,
        cut: &str,
    ) -> Result<Option<TypedObject>, ProducerError> {
        let view = state
            .shared_group(index)
            .map_err(|error| ProducerError::Degraded(error.reason))?;
        match view {
            Some(view) => self
                .build_group_object(store, index, &view, generation, cut)
                .map(Some),
            None => Ok(None),
        }
    }

    /// 从借用视图构造 CC-005 组对象（`build_group` 的视图面）。
    pub(super) fn build_group_object(
        &mut self,
        store: &mut dyn NodeStore,
        index: usize,
        view: &GroupEvidenceView<'_>,
        generation: i64,
        cut: &str,
    ) -> Result<TypedObject, ProducerError> {
        let anchor = view.bar.source_index;
        let reasons = group_reasons(view);
        // 来源集合与全量配方 `group_sources` 同值：共享链拼接，不展开证据前缀。
        let sources = group_sources_coords(view);
        let fact_key = canonical_json(&json!([CC005_GROUP_KIND, anchor.to_string()]));
        let payload_value = self.group_payload_value(store, index, view, &reasons)?;
        let input_refs = self.source_refs_array(&mut *store, &sources)?;
        let source_coords = self.coords_array(&mut *store, &sources)?;
        let mut object = self.typed_object(
            store,
            CC005_GROUP_KIND,
            fact_key,
            payload_value,
            input_refs,
            source_coords,
            generation,
            cut,
        )?;
        let key = CValue::Array(vec![
            CValue::String(CC005_GROUP_KIND.to_string()),
            CValue::String(anchor.to_string()),
        ]);
        let request = self.request_value(
            key,
            Some(&object.object_id),
            &reasons,
            &object.source_coords,
        )?;
        object.request = Some(sc::write_value(&mut *store, &request)?);
        Ok(object)
    }

    /// CC-005 载荷：字段与全量配方 `group_payload` 逐字段同型（集合只以值边接入）。
    /// 单元素源引用数组（0x05）。
    fn source_refs_single(
        &self,
        store: &mut dyn NodeStore,
        coord: i64,
        encoding: RefEncoding,
    ) -> Result<Built, ProducerError> {
        let leaf = self
            .ref_leaf_root(encoding, coord)
            .ok_or(ProducerError::MissingSource(coord))?;
        let seq = single_leaf_sequence(&mut *store, &leaf)?;
        let root = put_array_node(&mut *store, 1, &seq.root())?;
        Ok(Built {
            digest: root,
            kind: Kind::Array,
            extent: Some(1),
        })
    }

    /// CC-006 的 `dependency_refs`：`(dir ∪ conf) \ members` 的源引用数组（续接构造）。
    ///
    /// 与全量配方 `dependency_coords`（dir ∪ conf 去掉 members 后排序去重）同值。依据同一
    /// 归纳：`conf \ members = (dir \ {锚}) ∪ {conf.incoming}`——dir 的末元素就是本组锚，
    /// conf.incoming 是关闭边界（严格大于全部成员，且不在 dir 里）。故本口只做
    /// 一次 `sequence_rebuild_suffix(count-1, [边界叶]?)` 取 dir 左段并接单叶，不重写整段。
    ///
    /// 有效域与非退化域同 [`group_sources_coords`]；逐前缀等价性由
    /// `derived_coords_match_materialized_union_for_every_prefix` 对拍锁定。
    fn group_dependency_array(
        &mut self,
        store: &mut dyn NodeStore,
        view: &GroupEvidenceView<'_>,
        encoding: RefEncoding,
    ) -> Result<Built, ProducerError> {
        let boundary = match view.confirmation_evidence {
            Some(node) => Some(i64::try_from(node.incoming.source_index).map_err(|_| {
                ProducerError::Overflow(format!("源坐标 {} 超出 i64", node.incoming.source_index))
            })?),
            None => None,
        };
        let Some(dir) = view.direction_evidence else {
            return match boundary {
                Some(coord) => self.source_refs_single(&mut *store, coord, encoding),
                None => empty_array(store),
            };
        };
        let entry = self.ref_array_over(
            &mut *store,
            dir.coords.root_node().expect("方向证据链非空"),
            encoding,
        )?;
        let keep = entry.len.saturating_sub(1);
        // 一次重建「左段 ++ 边界叶」：不落 split 的左/右段根与 append 前的作废根（#1488 S3）。
        let (seq, len) = match boundary {
            Some(coord) => {
                let leaf = self
                    .ref_leaf_root(encoding, coord)
                    .ok_or(ProducerError::MissingSource(coord))?;
                (
                    sc::sequence_rebuild_suffix(&mut *store, &entry.seq, keep, &[leaf])?,
                    keep + 1,
                )
            }
            None => (
                sc::sequence_rebuild_suffix(&mut *store, &entry.seq, keep, &[])?,
                keep,
            ),
        };
        let root = put_array_node(&mut *store, len, &seq.root())?;
        Ok(Built {
            digest: root,
            kind: Kind::Array,
            extent: Some(len),
        })
    }

    /// CC-005 载荷：字段与全量配方 `group_payload` 逐字段同型（集合只以值边接入）。
    fn group_payload_value(
        &mut self,
        store: &mut dyn NodeStore,
        index: usize,
        view: &GroupEvidenceView<'_>,
        reasons: &[&'static str],
    ) -> Result<CValue, ProducerError> {
        // #1405：标签保留在原件里，请求原因仅反映方向未建立。
        let mut labels = reasons.to_vec();
        if view.high_sources.len() > 1 || view.low_sources.len() > 1 {
            labels.push("equal_extreme_identity");
        }
        let members = self.coords_array(&mut *store, &group_members_coords(view))?;
        let high_sources = self.coord_texts_slice(&mut *store, view.high_sources)?;
        let low_sources = self.coord_texts_slice(&mut *store, view.low_sources)?;
        let direction_evidence = match view.direction_evidence {
            Some(node) => self.evidence_value(&mut *store, node)?,
            None => CValue::Null,
        };
        let confirmation_evidence = match view.confirmation_evidence {
            Some(node) => self.evidence_value(&mut *store, node)?,
            None => CValue::Null,
        };
        let direction = match view.direction_evidence {
            Some(node) => CValue::String(dir_str(node.direction).to_string()),
            None => CValue::Null,
        };
        Ok(CValue::Object(vec![
            ("group_index".to_string(), CValue::String(index.to_string())),
            (
                "group_anchor".to_string(),
                CValue::String(view.bar.source_index.to_string()),
            ),
            ("members".to_string(), CValue::Edge(members.digest)),
            (
                "open".to_string(),
                CValue::String(view.bar.open.to_string()),
            ),
            (
                "high".to_string(),
                CValue::String(view.bar.high.to_string()),
            ),
            ("low".to_string(), CValue::String(view.bar.low.to_string())),
            (
                "close".to_string(),
                CValue::String(view.bar.close.to_string()),
            ),
            (
                "high_sources".to_string(),
                CValue::Edge(high_sources.digest),
            ),
            ("low_sources".to_string(), CValue::Edge(low_sources.digest)),
            ("confirmed".to_string(), CValue::Bool(view.confirmed)),
            ("confirmation_evidence".to_string(), confirmation_evidence),
            ("direction".to_string(), direction),
            ("direction_evidence".to_string(), direction_evidence),
            (
                "waiting_reasons".to_string(),
                CValue::Array(
                    labels
                        .iter()
                        .map(|reason| CValue::String((*reason).to_string()))
                        .collect(),
                ),
            ),
        ]))
    }

    /// CC-004 载荷：全部字段都是固定小对象 + 值边（坐标集合只以边接入）。
    ///
    /// `high_sources` / `low_sources` 是**当前 slice**：按精确内容编码（复用按坐标值记忆的
    /// 文本值根），不做不安全地址 memo。
    fn step_payload_value(
        &mut self,
        store: &mut dyn NodeStore,
        view: &StepEvidenceView<'_>,
    ) -> Result<CValue, ProducerError> {
        let (comparisons, endpoint_order) =
            comparisons_value(view.acc_before.as_ref(), &view.incoming);
        let direction = match view.evidence {
            Some(evidence) => CValue::String(dir_str(evidence.direction).to_string()),
            None => CValue::Null,
        };
        let direction_evidence = match view.evidence {
            Some(evidence) => self.evidence_value(&mut *store, evidence)?,
            None => CValue::Null,
        };
        let high_sources = self.coord_texts_slice(&mut *store, view.high)?;
        let low_sources = self.coord_texts_slice(&mut *store, view.low)?;
        Ok(CValue::Object(vec![
            (
                "action".to_string(),
                CValue::String(view.action.to_string()),
            ),
            ("incoming".to_string(), bar_value(&view.incoming)),
            (
                "acc_before".to_string(),
                view.acc_before
                    .as_ref()
                    .map(bar_value)
                    .unwrap_or(CValue::Null),
            ),
            (
                "acc_after".to_string(),
                view.acc_after
                    .as_ref()
                    .map(bar_value)
                    .unwrap_or(CValue::Null),
            ),
            ("direction".to_string(), direction),
            ("direction_evidence".to_string(), direction_evidence),
            ("comparisons".to_string(), comparisons),
            ("endpoint_order".to_string(), endpoint_order),
            (
                "contains".to_string(),
                view.contains.map(CValue::Bool).unwrap_or(CValue::Null),
            ),
            (
                "high_sources".to_string(),
                CValue::Edge(high_sources.digest),
            ),
            ("low_sources".to_string(), CValue::Edge(low_sources.digest)),
            (
                "waiting_reasons".to_string(),
                CValue::Array(
                    view.waiting_reasons
                        .iter()
                        .map(|reason| CValue::String((*reason).to_string()))
                        .collect(),
                ),
            ),
        ]))
    }

    /// slice 形态的坐标文本数组（当前值，不物化坐标集合、不做地址 memo）。
    fn coord_texts_slice(
        &mut self,
        store: &mut dyn NodeStore,
        coords: &[usize],
    ) -> Result<Built, ProducerError> {
        let mut leaves: Vec<NodeDigest> = Vec::with_capacity(coords.len());
        for coord in coords {
            let value = i64::try_from(*coord)
                .map_err(|_| ProducerError::Overflow(format!("源坐标 {coord} 超出 i64 表达")))?;
            leaves.push(coord_text_root(&mut *store, &mut self.coord_text, value)?);
        }
        array_of_leaves(&mut *store, leaves)
    }

    /// 清空三张 memo（纯缓存：清空后重建的根逐字节相同）。
    ///
    /// 源版本注册表**不清**——它是事实登记，不是缓存。跨事务/跨库/跨来源域时只调本口**不够**：
    /// 注册表里的根可能已随 rollback 消失，此时重绑同 spec 会直接返回、重建出缺闭包的对象。
    /// 那种场合要调 [`Cc004Producer::discard_runtime_state`]。
    pub(super) fn clear_memos(&mut self) {
        self.coord_text.clear();
        self.text_arrays.clear();
        self.ref_arrays.clear();
        self.shape_ref_arrays.clear();
        self.fractal_histories.clear();
    }

    /// 整份运行态丢弃/重建口：清空源版本注册表与全部 memo，保留冻结的
    /// `profile_id` / `rule_revision`（身份配方是会话冻结值，不是运行态）。
    ///
    /// **必须调用**（文档化作用域）：事务 rollback、换库、源会话/来源版本域变化。此时上轮写下的
    /// 节点已不存在（或属于另一域），而注册表里的根**不再**是当前事实；只清 memo 会让
    /// `bind_source` 按「同 spec」直接返回、不重新写节点，重建出来的对象闭包缺节点。
    /// 调用后必须按**真实的当前 source** 逐个 [`Cc004Producer::bind_source`] 再构造对象。
    ///
    /// **不得**在普通 append 上调用：追加只登记新的最大坐标，旧 prefix 的内容与 memo 全部有效。
    pub(super) fn discard_runtime_state(&mut self) {
        self.sources.clear();
        self.shape_sources.clear();
        // 视图见证同步作废：旧的 `PreparedRawHistory` 不得跨事务/换库继续消费。
        self.source_epoch += 1;
        self.unprepared_floor = None;
        self.prepared_raw_keys.clear();
        self.delta_binding = None;
        self.delta_epoch = None;
        self.prepared_snapshot = Rc::new(());
        self.clear_memos();
    }

    /// #1488 S3 测试面：三张 coords 数组 memo 的全部条目数组根（跨代复用的子树条目）。
    #[cfg(test)]
    pub(super) fn memo_array_refs(&self) -> Vec<sc::ContentRef> {
        [&self.text_arrays, &self.ref_arrays, &self.shape_ref_arrays]
            .into_iter()
            .flat_map(|cache| cache.entries.values().map(|entry| array_built(entry).content_ref()))
            .collect()
    }

    /// 测试/测量面读数（不参与任何判据）。
    #[cfg(test)]
    pub(super) fn memo_counts(&self) -> MemoCounts {
        MemoCounts {
            text_hits: self.text_arrays.hits,
            text_misses: self.text_arrays.misses,
            ref_hits: self.ref_arrays.hits,
            ref_misses: self.ref_arrays.misses,
            invalidated: self.ref_arrays.invalidated,
            shape_hits: self.shape_ref_arrays.hits,
            shape_misses: self.shape_ref_arrays.misses,
            shape_invalidated: self.shape_ref_arrays.invalidated,
            skipped_scans: self.ref_arrays.skipped_scans,
            unit_computed: self.history_computed,
            unit_reused: self.history_reused,
            unit_invalidated: self.history_invalidated,
        }
    }
}

/// 按 `M` 唯一分解构造「coords 子树 → 数组根」。
///
/// - 命中 memo：直接复用（连 `sequence_concat` 都不做，故不写任何节点）；
/// - 未命中：显式栈后序下钻该子树，叶按 `Leaves` 编码，内部节点用 [`sequence_concat`]
///   接元素序列再写 0x05 节点。
///
/// 全程不展开逻辑叶、不 materialize 坐标集合、不依赖 AVL 的高度/分组形状。
///
/// 返回内部条目（根 + 元素 `SequenceRef` + 叶数）：`sequence_split`/`sequence_append`
/// 之类的续接操作要用元素序列，而不是只拿数组根。
fn array_from_node(
    store: &mut dyn NodeStore,
    root: &Rc<CoordNode>,
    cache: &mut ArrayCache,
    leaves: &mut Leaves<'_>,
) -> Result<ArrayEntry, ProducerError> {
    #[cfg(test)]
    let _phase = tests::emission_phase("coords_array_memo");
    enum Step {
        Enter(Rc<CoordNode>),
        Exit(Rc<CoordNode>),
    }
    let mut stack: Vec<Step> = vec![Step::Enter(Rc::clone(root))];
    let mut done: Vec<ArrayEntry> = Vec::new();
    while let Some(step) = stack.pop() {
        match step {
            Step::Enter(node) => {
                if let Some(entry) = cache.get(&node) {
                    cache.hits += 1;
                    done.push(entry);
                    continue;
                }
                cache.misses += 1;
                match node.as_ref() {
                    CoordNode::Leaf(leaf) => {
                        let coord = i64::try_from(*leaf).map_err(|_| {
                            ProducerError::Overflow(format!("源坐标 {leaf} 超出 i64 表达"))
                        })?;
                        let leaf = leaves.element(&mut *store, coord)?;
                        let seq = single_leaf_sequence(&mut *store, &leaf)?;
                        let root = put_array_node(&mut *store, 1, &seq.root())?;
                        let entry = ArrayEntry {
                            tree: Rc::clone(&node),
                            seq,
                            root,
                            len: 1,
                            min: coord,
                            max: coord,
                        };
                        cache.insert(entry.clone());
                        done.push(entry);
                    }
                    CoordNode::Concat(left, right, _) => {
                        stack.push(Step::Exit(Rc::clone(&node)));
                        stack.push(Step::Enter(Rc::clone(right)));
                        stack.push(Step::Enter(Rc::clone(left)));
                    }
                }
            }
            Step::Exit(node) => {
                let right = done.pop().expect("右子树先完成");
                let left = done.pop().expect("左子树先于右子树完成");
                let len = left.len.checked_add(right.len).ok_or_else(|| {
                    ProducerError::Overflow(format!("数组元素数 {} + {} 溢出", left.len, right.len))
                })?;
                if len > sc::MAX_FRAME_LEN {
                    return Err(ProducerError::Overflow(format!(
                        "数组元素数 {len} 超过 2^63-1"
                    )));
                }
                let seq = sc::sequence_concat(&mut *store, &left.seq, &right.seq)?;
                let root = put_array_node(&mut *store, len, &seq.root())?;
                let entry = ArrayEntry {
                    tree: Rc::clone(&node),
                    seq,
                    root,
                    len,
                    min: left.min,
                    max: right.max,
                };
                cache.insert(entry.clone());
                done.push(entry);
            }
        }
    }
    if done.len() != 1 {
        return Err(ProducerError::Type {
            what: "coords 数组",
            detail: format!("后序遍历记账异常：{} 个结果", done.len()),
        });
    }
    Ok(done.pop().expect("刚核过恰 1 个结果"))
}

/// 条目 → 公开 `Built`（数组根 + 叶数；不复制任何子树）。
fn array_built(entry: &ArrayEntry) -> Built {
    Built {
        digest: entry.root,
        kind: Kind::Array,
        extent: Some(entry.len),
    }
}

/// 单叶序列 `M([h]) = F(0x12, h)`：节点必须落盘，父节点的 `sequence_concat` 会读它。
fn single_leaf_sequence(
    store: &mut dyn NodeSink,
    leaf: &NodeDigest,
) -> Result<SequenceRef, CodecError> {
    let digest = sc::frame(sc::tag::SEQ_LEAF, leaf);
    store.put(digest, sc::tag::SEQ_LEAF, leaf)?;
    Ok(SequenceRef::from_parts_unchecked(digest, 1))
}

/// `0x05` 数组节点：body = `U64(元素数) || M(元素摘要)`。
fn put_array_node(
    store: &mut dyn NodeSink,
    count: u64,
    seq_root: &NodeDigest,
) -> Result<NodeDigest, CodecError> {
    let mut body = Vec::with_capacity(40);
    body.extend_from_slice(&sc::checked_frame_len(count)?);
    body.extend_from_slice(seq_root);
    let digest = sc::frame(sc::tag::ARRAY, &body);
    store.put(digest, sc::tag::ARRAY, &body)?;
    Ok(digest)
}

/// 由已写出的元素值根列表构造规范数组（0x05）：`U64(n) || M([h…])`。
///
/// 叶序即元素序；同一逻辑序列的任意物理树形都会得到同一个根（`M` 按元素数唯一分解）。
fn array_of_leaves(
    store: &mut dyn NodeStore,
    leaves: Vec<NodeDigest>,
) -> Result<Built, ProducerError> {
    let count = u64::try_from(leaves.len())
        .map_err(|_| ProducerError::Overflow(format!("数组元素数 {} 超出 u64", leaves.len())))?;
    let seq_root = sc::write_sequence(&mut *store, &leaves)?;
    let root = put_array_node(&mut *store, count, &seq_root)?;
    Ok(Built {
        digest: root,
        kind: Kind::Array,
        extent: Some(count),
    })
}

/// #1394 C §5.3：由**已写出的规范序列**（`0x11/0x12/0x13` 的 M 树）生成 `0x05` 数组值。
///
/// body = `U64(叶数) || M 根`，与 `write_value` 的数组分支是同一个帧配方；同一逻辑叶序的不同
/// 物理树形都得到同一个根（`M` 按元素数唯一分解）。空序列按 `empty_array` 同口径落 `0x11` 节点。
/// 本口只写一次数组节点，不展开序列逻辑叶；切片成本由调用方的序列操作决定，
/// 不能由本口的固定写入成本推出任意切片的最坏复杂度。
pub(super) fn array_from_sequence(
    store: &mut dyn NodeStore,
    seq: &SequenceRef,
) -> Result<Built, ProducerError> {
    let count = seq.count();
    let seq_root = if count == 0 {
        let empty = SequenceRef::empty();
        store.put(empty.root(), sc::tag::SEQ_EMPTY, &[])?;
        empty.root()
    } else {
        seq.root()
    };
    let root = put_array_node(&mut *store, count, &seq_root)?;
    Ok(Built {
        digest: root,
        kind: Kind::Array,
        extent: Some(count),
    })
}

/// 一个逻辑值的规范 JSON 文本（§5.1/§5.2 的同一渲染器给出的文本）。
///
/// 在**私有草稿表**里落节点再渲染，不污染调用方节点表：该文本只进请求键
/// （`request_id`），请求键值本身不是要发布的内容节点。
fn canonical_text(value: &CValue) -> Result<String, ProducerError> {
    let mut scratch = sc::MemoryNodes::default();
    let built = sc::write_value(&mut scratch, value)?;
    let mut out: Vec<u8> = Vec::new();
    sc::render_canonical(&mut scratch, &built.content_ref(), &mut out)?;
    String::from_utf8(out).map_err(|error| ProducerError::Type {
        what: "规范 JSON 文本",
        detail: error.to_string(),
    })
}

/// 空数组：`M([])` 由规范唯一确定，节点仍按「写下即核」口径落盘。
fn empty_array(store: &mut dyn NodeStore) -> Result<Built, ProducerError> {
    let empty = SequenceRef::empty();
    store.put(empty.root(), sc::tag::SEQ_EMPTY, &[])?;
    let root = put_array_node(&mut *store, 0, &empty.root())?;
    Ok(Built {
        digest: root,
        kind: Kind::Array,
        extent: Some(0),
    })
}

/// `0x07` 文本视图节点：body 恰好是内部值摘要（32 字节），不渲染整份文本。
fn put_json_text_node(
    store: &mut dyn NodeSink,
    inner: &NodeDigest,
) -> Result<NodeDigest, CodecError> {
    let digest = sc::frame(sc::tag::JSON_TEXT, inner);
    store.put(digest, sc::tag::JSON_TEXT, inner)?;
    Ok(digest)
}

/// 坐标的十进制文本值根（按坐标值记忆；坐标不可变，故永不失效）。
fn coord_text_root(
    store: &mut dyn NodeStore,
    memo: &mut HashMap<i64, NodeDigest>,
    coord: i64,
) -> Result<NodeDigest, ProducerError> {
    if let Some(root) = memo.get(&coord) {
        return Ok(*root);
    }
    let built = sc::write_value(&mut *store, &CValue::String(coord.to_string()))?;
    memo.insert(coord, built.digest);
    Ok(built.digest)
}

/// raw bar 投影（与全量配方 `bar` 逐字段同型：坐标与四价都是十进制文本）。
fn bar_value(bar: &Bar) -> CValue {
    CValue::Object(vec![
        (
            "source_coord".to_string(),
            CValue::String(bar.source_index.to_string()),
        ),
        ("open".to_string(), CValue::String(bar.open.to_string())),
        ("high".to_string(), CValue::String(bar.high.to_string())),
        ("low".to_string(), CValue::String(bar.low.to_string())),
        ("close".to_string(), CValue::String(bar.close.to_string())),
    ])
}

/// 当前 raw 链的共享前缀句柄（继续拼接只新建 O(log n) 脊，子树沿用）。
///
/// 依据（inclusion.rs seed / new_group / merge 三处 `coords` 构造）：每个 StepRecord 的
/// `coords` 恰是「append 到该步 incoming 为止的完整 raw 前缀」——seed = single(incoming)；
/// new_group = concat(前证据或 single(前锚), 前组 tail) 再接 incoming；merge 同形；
/// waiting 步直接取 `raw_prefix`。故末步（step_count-1）的 `coords` 即当前前缀。
///
/// 有效域：非退化共享态（degraded 显式 `Err`，回落全量入口）；空态返回空集。
/// 逐前缀等价性由测试 `raw_prefix_chain_is_last_step_coords_for_every_prefix` 对拍锁定。
pub(super) fn raw_prefix_coords(state: &IncrFactsState) -> Result<CoordsRef, ProducerError> {
    if state.step_count() == 0 {
        return Ok(CoordsRef::empty());
    }
    let view = state
        .shared_step(state.step_count() - 1)
        .map_err(|error| ProducerError::Degraded(error.reason))?;
    match view {
        Some(view) => Ok(view.coords.clone()),
        None => Err(ProducerError::Type {
            what: "raw 前缀链",
            detail: "末步索引在域内却借不出视图".to_string(),
        }),
    }
}

/// 升序去重坐标切片 → 共享链（逐项 `append_greater`：只新增 O(log n) 脊，旧前缀继续共享）。
///
/// 有效域：调用方保证切片严格升序（本口的输入都是 raw 到达序的升序子序列；违反即
/// `append_greater` 的既有前提被破坏，属调用方 bug，不在此静默重排）。
pub(super) fn coords_ref_from_slice(coords: &[usize]) -> CoordsRef {
    let mut out = CoordsRef::empty();
    for coord in coords {
        out = out.append_greater(*coord);
    }
    out
}

/// 借用原因数组；不复制文本、不建立第二棵业务 JSON 树。
struct ReasonViews<'a>(&'a [&'static str]);
impl sc::capture_allocation::audit_json::JsonArray for ReasonViews<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn get(&self, index: usize) -> Option<sc::capture_allocation::audit_json::JsonView<'_>> {
        self.0
            .get(index)
            .map(|reason| sc::capture_allocation::audit_json::JsonView::String(reason))
    }
}
fn write_knowledge_observation(
    store: &mut dyn NodeStore,
    reason: &str,
    frontier: &str,
    source_coords: NodeDigest,
    reasons: &dyn sc::capture_allocation::audit_json::JsonArray,
) -> Result<Built, ProducerError> {
    use sc::capture_allocation::audit_json::JsonView as J;
    use sc::ValueView as V;
    let detail = [
        ("tb02a", V::Json(J::Bool(true))),
        ("source_coords", V::Edge(source_coords)),
        ("input_frontier", V::String(frontier)),
        ("waiting_reasons", V::Json(J::BorrowedArray(reasons))),
    ];
    let fields = [
        ("kind", V::String("awaiting_scope")),
        ("reason", V::String(reason)),
        ("window_start", V::Json(J::Null)),
        ("window_mid", V::Json(J::Null)),
        ("window_end", V::Json(J::Null)),
        ("detail", V::Object(&detail)),
    ];
    Ok(sc::write_value_view(store, V::Object(&fields))?)
}

/// 组对象的等待原因（与全量配方 `group_reasons` 同序同值）。
pub(super) fn group_reasons(view: &GroupEvidenceView<'_>) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    if view.direction_evidence.is_none() {
        reasons.push("direction_not_yet_established");
    }
    reasons
}

/// #1488 原原因序不变；新容器的实际容量由当前操作先准入。
pub(super) fn group_reasons_admitted(
    store: &dyn NodeStore,
    view: &GroupEvidenceView<'_>,
) -> Result<sc::CodecVec<&'static str>, ProducerError> {
    // #1405：同价多根只作组原件标签，不再进请求原因（与 `group_reasons` 同步）。
    let count = usize::from(view.direction_evidence.is_none());
    let mut reasons = if let Some(work) = store.codec_work() {
        // 当前 scope 活性独立核验；此处是空新容器，不借旧许可的同父判断。
        let _active = work.reserve_native(0)?;
        let mut values = sc::CodecVec::default();
        values.try_reserve_native(work, count)?;
        values
    } else {
        #[cfg(feature = "s_allocation_json_testonly")]
        let budget = store.allocation_budget();
        #[cfg(feature = "s_allocation_json_testonly")]
        let mut values = sc::CodecVec::new_with_allocation(budget.as_ref());
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        let mut values = sc::CodecVec::default();
        values.try_reserve(count)?;
        values
    };
    if view.direction_evidence.is_none() {
        reasons.try_push("direction_not_yet_established")?;
    }
    Ok(reasons)
}

/// 竞争窗口的待决请求来源：三组来源链的并集 = **右组来源链**（归纳嵌套前缀性，见
/// [`group_sources_coords`]；逐前缀对拍锁定「三连窗并集 == 右组 sources」）。
pub(super) fn group_window_sources(views: &[GroupEvidenceView<'_>; 3]) -> CoordsRef {
    group_sources_coords(&views[2])
}

/// 原始账里按坐标取 bar（升序表二分；找不到即拒绝，不近似回落）。
fn raw_bar_at<'a>(
    raw_bars: super::s_fact_cache::RawBars<'a>,
    coord: usize,
) -> Result<&'a Bar, ProducerError> {
    let index = raw_bars.partition_point(|bar| bar.source_index < coord);
    raw_bars
        .get(index)
        .filter(|bar| bar.source_index == coord)
        .ok_or(ProducerError::MissingSource(coord as i64))
}

/// CC-007 比较单元的精确内容键（与全量配方 `unit_content_key` 同字段同型）。
///
/// 源坐标/四价是十进制文本，`revision` 是精确整数 token，`input_revision` 是文本。
fn fractal_unit_key(
    coord: i64,
    revision: i64,
    input_revision: &str,
    bar: &Bar,
) -> Result<String, ProducerError> {
    canonical_text(&CValue::Object(vec![
        (
            "source_coord".to_string(),
            CValue::String(coord.to_string()),
        ),
        ("revision".to_string(), number(revision)?),
        (
            "input_revision".to_string(),
            CValue::String(input_revision.to_string()),
        ),
        ("open".to_string(), CValue::String(bar.open.to_string())),
        ("high".to_string(), CValue::String(bar.high.to_string())),
        ("low".to_string(), CValue::String(bar.low.to_string())),
        ("close".to_string(), CValue::String(bar.close.to_string())),
    ]))
}

/// 一根后续 bar 的 4 轴 × 6 参考价 = 24 个比较单元（顺序与全量入口相同）。
fn fractal_relations(bar: &Bar, references: &[(String, i64)]) -> Vec<CValue> {
    let mut relations: Vec<CValue> = Vec::with_capacity(24);
    for (axis, price) in [
        ("open", bar.open),
        ("high", bar.high),
        ("low", bar.low),
        ("close", bar.close),
    ] {
        for (name, target) in references {
            relations.push(CValue::Object(vec![
                ("axis".to_string(), CValue::String(axis.to_string())),
                ("value".to_string(), CValue::String(price.to_string())),
                ("reference".to_string(), CValue::String(name.clone())),
                (
                    "reference_value".to_string(),
                    CValue::String(target.to_string()),
                ),
                (
                    "order".to_string(),
                    CValue::String(order_of(price, *target).to_string()),
                ),
            ]));
        }
    }
    relations
}

/// 组对象的来源集合（与全量配方 `group_sources` 同一条）：成员 ∪ 方向证据 ∪ 封口证据。
///
/// 这里给出**共享链**构造（不展开方向/封口证据前缀），与全量配方 `group_sources`
/// （members ∪ dir ∪ conf，排序去重）同值。依据（inclusion.rs:1057–1110 的新证据组合 +
/// merge 分支的 `coords` 构造归纳）：
///
/// - `dir.coords` = 处理到本组锚为止的完整前缀（new_group 的证据 = concat(前证据, 前组
///   tail) 再接 incoming）；`members` = `single(锚) ++ tail`，锚 ∈ dir、tail 全大于锚；
/// - `conf.coords` = 本组关闭时的同一条归纳链（= dir ∪ tail ∪ {关闭边界}）⟹ 有 confirm 时
///   它已包含 members 与 dir，并集即 conf.coords；
/// - 仅 dir ⟹ 并集 = dir ∪ tail = `concat(dir.coords, member_tail)`（两段严格升序、不交叠）；
/// - 两者皆无 ⟹ 并集 = members。
///
/// 有效域：非退化域的已分组 raw（每根 raw 恰属一组）。逐前缀等价性由测试
/// `derived_coords_match_materialized_union_for_every_prefix` 对拍锁定，不凭推导免测。
pub(super) fn group_sources_coords(view: &GroupEvidenceView<'_>) -> CoordsRef {
    if let Some(node) = view.confirmation_evidence {
        return node.coords.clone();
    }
    match view.direction_evidence {
        Some(node) => CoordsRef::concat(&node.coords, view.member_tail),
        None => group_members_coords(view),
    }
}

/// 组成员坐标链：`single(anchor) ++ member_tail`（与 `members` 同值，共享拼接）。
///
/// 依据：组锚 = 组内首成员（`shared_group` 契约），`member_tail` = `members[1..]` 的既有
/// 共享尾链（提交 90901b9809 的逐前缀对拍已钉）。
pub(super) fn group_members_coords(view: &GroupEvidenceView<'_>) -> CoordsRef {
    CoordsRef::concat(&CoordsRef::single(view.bar.source_index), view.member_tail)
}

/// 与全量配方 `comparisons` 同一判据：高/低两轴比较 + 端点等价类。
fn comparisons_value(acc_before: Option<&Bar>, incoming: &Bar) -> (CValue, CValue) {
    let Some(acc) = acc_before else {
        return (CValue::Array(vec![]), CValue::Array(vec![]));
    };
    let axis = |name: &str, acc_price: i64, incoming_price: i64| {
        CValue::Object(vec![
            ("axis".to_string(), CValue::String(name.to_string())),
            ("acc".to_string(), CValue::String(acc_price.to_string())),
            (
                "incoming".to_string(),
                CValue::String(incoming_price.to_string()),
            ),
            (
                "order".to_string(),
                CValue::String(order_of(acc_price, incoming_price).to_string()),
            ),
        ])
    };
    let comparisons = CValue::Array(vec![
        axis("high", acc.high, incoming.high),
        axis("low", acc.low, incoming.low),
    ]);
    let mut endpoints = [
        (acc.low, "acc.low"),
        (acc.high, "acc.high"),
        (incoming.low, "incoming.low"),
        (incoming.high, "incoming.high"),
    ];
    endpoints.sort();
    let mut classes: Vec<Vec<&str>> = Vec::new();
    let mut previous: Option<i64> = None;
    for (price, label) in endpoints {
        if previous != Some(price) {
            classes.push(Vec::new());
        }
        classes.last_mut().expect("刚按价格边界开新类").push(label);
        previous = Some(price);
    }
    let endpoint_order = CValue::Array(
        classes
            .into_iter()
            .map(|class| {
                CValue::Array(
                    class
                        .into_iter()
                        .map(|label| CValue::String(label.to_string()))
                        .collect(),
                )
            })
            .collect(),
    );
    (comparisons, endpoint_order)
}

/// 与全量配方 `order` 同值（全量侧是 `tb02a_facts` 的私有函数，故在此复写同一判据）。
fn order_of(a: i64, b: i64) -> &'static str {
    match a.cmp(&b) {
        std::cmp::Ordering::Less => "LT",
        std::cmp::Ordering::Equal => "EQ",
        std::cmp::Ordering::Greater => "GT",
    }
}

fn number(value: i64) -> Result<CValue, ProducerError> {
    let token = NumberToken::parse(&value.to_string()).map_err(|error| ProducerError::Type {
        what: "数字 token",
        detail: error.to_string(),
    })?;
    Ok(CValue::Number(token))
}

fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// §5.3：`obj3-` + `F(0x20, V)` 的小写十六进制文本；全模块唯一的对象 id 文本构造口。
///
/// `V` 是对应身份配方的值摘要（泛型六字段或 CC-006 十二字段）。该文本只是 id **视图**：
/// `V` 本身不必落成内容节点
/// （见 [`DigestOnly`]），故这里只做帧摘要与十六进制渲染，不写节点表。
pub(super) fn object_id_of(identity: &NodeDigest) -> String {
    object_id_display(identity).to_string()
}

/// 固定摘要的文本视图；审计可先计长并准入，生产沿同一格式化实现。
pub(super) fn object_id_display(identity: &NodeDigest) -> impl fmt::Display {
    struct ObjectId(NodeDigest);
    impl fmt::Display for ObjectId {
        fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
            let root = sc::FixedRoot::new(&self.0);
            write!(out, "obj3-{}", &root.as_str()[4..])
        }
    }
    ObjectId(sc::frame(OBJECT_ID_TAG, identity))
}

// ────────────────────────────────────────────────────────────────────────────
// 测试面：真实构造器 + 同次 parser；旧 `Builder` 只在这里充当 oracle。
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::content_store::{init_schema, ReadStore, TxStore};
    use super::*;
    use newchan_rust::s_content::{render_canonical, verify, MemoryNodes, NodeRead, NodeSource};
    use sha2::{Digest, Sha256};

    /// NOT_RUN：本机重型运行禁令；比对根及每次 put 的完整顺序/字节。
    #[test]
    fn knowledge_observation_view_matches_legacy_root_and_write_order() {
        #[derive(Default)]
        struct Trace {
            nodes: MemoryNodes,
            writes: Vec<(NodeDigest, u8, Vec<u8>)>,
        }
        impl NodeSink for Trace {
            fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
                self.writes.push((digest, tag, body.to_vec()));
                self.nodes.put(digest, tag, body)
            }
        }
        impl NodeSource for Trace {
            fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
                self.nodes.read(digest)
            }
        }
        for reasons in [
            &[][..],
            &["direction_not_yet_established"][..],
            &["equal_extreme_identity", "no_subsequent_bar"][..],
        ] {
            for frontier in ["0", "9001", "-1"] {
                let mut old = Trace::default();
                let mut new = Trace::default();
                let old_coords = empty_array(&mut old).unwrap();
                let new_coords = empty_array(&mut new).unwrap();
                assert_eq!(old_coords.digest, new_coords.digest);
                for reason in reasons {
                    let legacy = CValue::Object(vec![
                        ("kind".into(), CValue::String("awaiting_scope".into())),
                        ("reason".into(), CValue::String((*reason).into())),
                        ("window_start".into(), CValue::Null),
                        ("window_mid".into(), CValue::Null),
                        ("window_end".into(), CValue::Null),
                        (
                            "detail".into(),
                            CValue::Object(vec![
                                ("tb02a".into(), CValue::Bool(true)),
                                ("source_coords".into(), CValue::Edge(old_coords.digest)),
                                ("input_frontier".into(), CValue::String(frontier.into())),
                                (
                                    "waiting_reasons".into(),
                                    CValue::Array(
                                        reasons
                                            .iter()
                                            .map(|r| CValue::String((*r).into()))
                                            .collect(),
                                    ),
                                ),
                            ]),
                        ),
                    ]);
                    let want = sc::write_value(&mut old, &legacy).unwrap();
                    let got = write_knowledge_observation(
                        &mut new,
                        reason,
                        frontier,
                        new_coords.digest,
                        &ReasonViews(reasons),
                    )
                    .unwrap();
                    assert_eq!(got.digest, want.digest);
                }
                assert_eq!(new.writes, old.writes);
            }
        }
    }

    const PROFILE: &str = "testonly_ohlc/1";
    const RULE: &str = "test-rule/1";
    const GENERATION: i64 = 3;
    const CUT: &str = "cut-3";

    /// 稀疏坐标 + 包含合并 + 换组/方向变化 + 同价极值身份竞争。
    const CASE_MERGE: &[(usize, i64, i64)] = &[
        (10, 10, 5),
        (11, 15, 8),
        (37, 15, 6),
        (900, 12, 6),
        (9001, 9, 4),
        (12345, 9, 3),
    ];
    /// 初始方向永久未定：首步 seed 之后全部 waiting（无方向证据、无极值快照）。
    const CASE_WAITING: &[(usize, i64, i64)] = &[(0, 10, 5), (1, 9, 6), (2, 12, 7)];
    /// 不足三根 merged：覆盖「已建方向前知识不足」的待决请求与原因。
    const CASE_TWO: &[(usize, i64, i64)] = &[(10, 10, 5), (20, 15, 8)];
    /// 无同价极值竞争的规则折线：三组窗口可判 CC-006（供形态/分型对拍）。
    const CASE_SHAPE: &[(usize, i64, i64)] = &[
        (10, 10, 5),
        (20, 20, 15),
        (30, 14, 7),
        (40, 25, 18),
        (50, 19, 10),
        (60, 30, 22),
        (70, 24, 14),
    ];

    /// 同一事务内可读写（读自己刚写的节点）、另记读写次数的节点表。
    #[derive(Default)]
    struct CountingStore {
        nodes: MemoryNodes,
        reads: usize,
        puts: usize,
        array_puts: usize,
        branch_puts: usize,
        emissions: Option<EmissionInventory>,
    }

    impl NodeSource for CountingStore {
        fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
            self.reads += 1;
            NodeSource::read(&mut self.nodes, digest)
        }
    }

    impl NodeSink for CountingStore {
        fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
            self.puts += 1;
            self.array_puts += usize::from(tag == sc::tag::ARRAY);
            self.branch_puts += usize::from(tag == sc::tag::SEQ_BRANCH);
            if let Some(emissions) = &mut self.emissions {
                let already_stored =
                    self.nodes
                        .get(&digest)
                        .is_some_and(|(stored_tag, stored_body)| {
                            stored_tag == tag && stored_body == body
                        });
                emissions.observe(digest, tag, body, already_stored);
            }
            NodeSink::put(&mut self.nodes, digest, tag, body)
        }
    }

    thread_local! {
        static EMISSION_PHASE_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
        static EMISSION_PHASE: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
    }

    pub(super) struct EmissionPhaseGuard {
        enabled_before: bool,
        phase_before: Option<&'static str>,
    }

    impl Drop for EmissionPhaseGuard {
        fn drop(&mut self) {
            EMISSION_PHASE_ENABLED.with(|enabled| enabled.set(self.enabled_before));
            EMISSION_PHASE.with(|phase| phase.set(self.phase_before));
        }
    }

    fn enable_emission_phases() -> EmissionPhaseGuard {
        EmissionPhaseGuard {
            enabled_before: EMISSION_PHASE_ENABLED.with(|enabled| enabled.replace(true)),
            phase_before: EMISSION_PHASE.with(|phase| phase.replace(None)),
        }
    }

    /// 线程局部、默认关闭；仅诊断测试启用，guard 离开真实调用作用域即恢复。
    pub(super) fn emission_phase(phase: &'static str) -> EmissionPhaseGuard {
        let enabled_before = EMISSION_PHASE_ENABLED.with(|enabled| enabled.get());
        let phase_before = EMISSION_PHASE.with(|current| {
            if enabled_before {
                current.replace(Some(phase))
            } else {
                current.get()
            }
        });
        EmissionPhaseGuard {
            enabled_before,
            phase_before,
        }
    }

    #[derive(Default)]
    struct EmissionEntry {
        body: Vec<u8>,
        calls: usize,
        phases: BTreeMap<&'static str, usize>,
        existed_before_sample: bool,
    }

    /// #1394 只观测的测试账本：摘要/标签只是桶索引，body 必须逐字相同才累计。
    /// 默认关闭；绝不查询此账本来省略或替换 NodeSink::put。
    #[derive(Default)]
    struct EmissionInventory {
        phase: &'static str,
        materials: HashMap<(NodeDigest, u8), Vec<EmissionEntry>>,
        phase_calls: BTreeMap<&'static str, usize>,
    }

    impl EmissionInventory {
        fn unit_calls(&self) -> usize {
            self.phase_calls
                .iter()
                .filter(|(phase, _)| {
                    matches!(
                        **phase,
                        "fractal_relations_write_value"
                            | "fractal_unit_write_value"
                            | "fractal_sequence_append"
                            | "fractal_units_array"
                    )
                })
                .map(|(_, calls)| calls)
                .sum()
        }

        fn observe(&mut self, digest: NodeDigest, tag: u8, body: &[u8], already_stored: bool) {
            let phase = EMISSION_PHASE
                .with(|phase| phase.get())
                .unwrap_or(self.phase);
            *self.phase_calls.entry(phase).or_default() += 1;
            let bucket = self.materials.entry((digest, tag)).or_default();
            let index = match bucket.iter().position(|entry| entry.body == body) {
                Some(index) => index,
                None => {
                    bucket.push(EmissionEntry {
                        body: body.to_vec(),
                        existed_before_sample: already_stored,
                        ..EmissionEntry::default()
                    });
                    bucket.len() - 1
                }
            };
            let entry = &mut bucket[index];
            entry.calls += 1;
            *entry.phases.entry(phase).or_default() += 1;
        }

        fn summary(&self) -> Value {
            let mut tags: BTreeMap<u8, (usize, usize, usize, usize)> = BTreeMap::new();
            let mut phases: BTreeMap<&str, (usize, usize, usize, usize)> = BTreeMap::new();
            let mut top = Vec::new();
            for ((digest, tag), bucket) in &self.materials {
                for entry in bucket {
                    let bytes = entry.body.len();
                    let total = tags.entry(*tag).or_default();
                    total.0 += entry.calls;
                    total.1 += 1;
                    total.2 += entry.calls * bytes;
                    total.3 += bytes;
                    for (phase, calls) in &entry.phases {
                        let total = phases.entry(phase).or_default();
                        total.0 += calls;
                        total.1 += 1;
                        total.2 += calls * bytes;
                        total.3 += bytes;
                    }
                    top.push((*digest, *tag, entry));
                }
            }
            // 同频按 tag/body/digest 排序，固定输出不受 HashMap 随机种子影响。
            top.sort_by(|a, b| {
                b.2.calls
                    .cmp(&a.2.calls)
                    .then_with(|| a.1.cmp(&b.1))
                    .then_with(|| a.2.body.cmp(&b.2.body))
                    .then_with(|| a.0.cmp(&b.0))
            });
            let row = |(calls, unique, bytes, unique_bytes): (usize, usize, usize, usize)| {
                json!({"calls": calls, "distinct_materials": unique, "body_bytes": bytes,
                    "distinct_body_bytes": unique_bytes, "repeat_calls": calls - unique,
                    "repeat_body_bytes": bytes - unique_bytes})
            };
            let totals = tags.values().fold((0, 0, 0, 0), |a, b| {
                (a.0 + b.0, a.1 + b.1, a.2 + b.2, a.3 + b.3)
            });
            let material = |(digest, tag, entry): &(NodeDigest, u8, &EmissionEntry)| {
                // STRING/NUMBER body 含字节树摘要；可识别文本落在 BYTES_CHUNK。
                let text = if *tag == sc::tag::BYTES_CHUNK && entry.body.len() <= 96 {
                    std::str::from_utf8(&entry.body).ok()
                } else {
                    None
                };
                json!({"digest": lower_hex(digest), "tag": tag, "body_len": entry.body.len(),
                    "calls": entry.calls, "body_bytes": entry.calls * entry.body.len(),
                    "repeat_calls": entry.calls - 1, "text": text, "phases": entry.phases,
                    "existed_before_sample": entry.existed_before_sample})
            };
            let mut summary = row(totals);
            summary["preexisting_material_calls"] = json!(top
                .iter()
                .filter(|(_, _, entry)| entry.existed_before_sample)
                .map(|(_, _, entry)| entry.calls)
                .sum::<usize>());
            summary["calls_with_material_already_stored"] = json!(top
                .iter()
                .map(|(_, _, entry)| entry.calls - usize::from(!entry.existed_before_sample))
                .sum::<usize>());
            summary["byte_metric"] = json!("NodeSink body bytes; frame/DB overhead excluded");
            summary["repeat_scope"] =
                json!("within this sample; preexisting material reported separately");
            summary["repeat_call_fraction"] = json!(if totals.0 == 0 {
                0.0
            } else {
                (totals.0 - totals.1) as f64 / totals.0 as f64
            });
            summary["by_tag"] = json!(tags
                .into_iter()
                .map(|(tag, values)| {
                    let mut result = row(values);
                    result["tag"] = json!(tag);
                    result
                })
                .collect::<Vec<_>>());
            summary["by_phase"] = json!(phases
                .into_iter()
                .map(|(phase, values)| {
                    let mut result = row(values);
                    result["phase"] = json!(phase);
                    result
                })
                .collect::<Vec<_>>());
            summary["top20"] = json!(top.iter().take(20).map(material).collect::<Vec<_>>());
            summary["small_text_top20"] = json!(top
                .iter()
                .filter(|(_, tag, entry)| *tag == sc::tag::BYTES_CHUNK
                    && entry.body.len() <= 96
                    && std::str::from_utf8(&entry.body).is_ok())
                .take(20)
                .map(material)
                .collect::<Vec<_>>());
            summary
        }
    }

    fn render(store: &mut dyn NodeSource, built: &Built) -> String {
        let mut out: Vec<u8> = Vec::new();
        render_canonical(store, &built.content_ref(), &mut out).expect("规范渲染");
        String::from_utf8(out).expect("UTF-8")
    }

    #[test]
    fn borrowed_shape_identity_matches_original_recipe_and_frame_count() {
        let producer = Cc004Producer::new("档案\"x", "rule-\\v1");
        let comparisons = Built {
            digest: [0x11; 32],
            kind: Kind::Array,
            extent: Some(4),
        };
        let input_refs = Built {
            digest: [0x22; 32],
            kind: Kind::Array,
            extent: Some(3),
        };
        for (branch, dir_ab, dir_bc) in [("TOP", "UP", "DOWN"), ("BOTTOM", "DOWN", "UP")] {
            let window = ["0", "10", "9223372036854775807"];
            let highs = ["-9223372036854775808", "0", "9223372036854775807"];
            let lows = ["-7", "0", "2"];
            // 冻结旧字段配方仅作测试 oracle；生产两个入口共用上面的借用核。
            let old = CValue::Object(vec![
                ("branch".into(), CValue::String(branch.into())),
                ("dir_ab".into(), CValue::String(dir_ab.into())),
                ("dir_bc".into(), CValue::String(dir_bc.into())),
                ("window_start".into(), CValue::String(window[0].into())),
                ("window_mid".into(), CValue::String(window[1].into())),
                ("window_end".into(), CValue::String(window[2].into())),
                (
                    "merged_highs".into(),
                    CValue::Array(highs.map(|s| CValue::String(s.into())).into()),
                ),
                (
                    "merged_lows".into(),
                    CValue::Array(lows.map(|s| CValue::String(s.into())).into()),
                ),
                ("comparisons".into(), CValue::Edge(comparisons.digest)),
                ("input_refs".into(), CValue::Edge(input_refs.digest)),
                (
                    "rule_revision".into(),
                    CValue::String(producer.rule_revision.clone()),
                ),
                (
                    "profile_id".into(),
                    CValue::String(producer.profile_id.clone()),
                ),
            ]);
            let before = sc::frame_calls_on_thread();
            let expected = producer.identity_summary(&old).unwrap();
            let old_frames = sc::frame_calls_on_thread() - before;
            let before = sc::frame_calls_on_thread();
            let borrowed = producer
                .shape_identity_borrowed(ShapeIdentityView {
                    branch,
                    dir_ab,
                    dir_bc,
                    window,
                    highs,
                    lows,
                    comparisons: &comparisons,
                    input_refs: &input_refs,
                })
                .unwrap();
            assert_eq!(borrowed, expected);
            assert_eq!(sc::frame_calls_on_thread() - before, old_frames);
            let normal = producer
                .shape_identity(ShapeIdentity {
                    branch,
                    dir_ab,
                    dir_bc,
                    window: window.map(str::to_owned),
                    highs: highs.map(str::to_owned),
                    lows: lows.map(str::to_owned),
                    comparisons: &comparisons,
                    input_refs: &input_refs,
                })
                .unwrap();
            assert_eq!(normal, borrowed);
        }
    }

    fn bar(coord: usize, high: i64, low: i64) -> Bar {
        Bar {
            source_index: coord,
            timestamp: coord as i64,
            open: low,
            high,
            low,
            close: high,
            volume: 1.0,
            untradable: false,
        }
    }

    /// 生产 raw 行（八字段 `raw_ref` 的输入）。
    ///
    /// oracle 与构造器从**同一个来源**取字段，两侧不可能各自漂移；`raw_json` 的类型与
    /// 生产 raw 行一致（四个身份字段 + `input_revision` 是文本，`revision`/`seq` 是整数，
    /// `source_coord` 是已存文本）。
    #[derive(Clone)]
    struct Row {
        coord: i64,
        revision: i64,
        receipt: String,
        /// OHLC（生产 raw 行同字段）：`None` = 该夹具行不投影价格列。
        prices: Option<(i64, i64, i64, i64)>,
    }

    impl Row {
        fn new(coord: i64, revision: i64, receipt: &str) -> Self {
            Row {
                coord,
                revision,
                receipt: receipt.to_string(),
                prices: None,
            }
        }

        /// 补上原始价格列（`open/high/low/close`；生产 raw 行同字段）。
        fn with_prices(mut self, open: i64, high: i64, low: i64, close: i64) -> Self {
            self.prices = Some((open, high, low, close));
            self
        }

        fn raw_json(&self) -> Value {
            let mut row = json!({
                "identity_key": format!("id-{}", self.coord),
                "revision": self.revision,
                "input_revision": self.revision.to_string(),
                "payload_hash": format!("ph-{}-{}", self.coord, self.revision),
                "receipt_id": self.receipt.clone(),
                "seq": self.coord + 100,
                "source_coord": self.coord.to_string(),
                "event_id": format!("ev-{}", self.coord),
            });
            if let Some((open, high, low, close)) = self.prices {
                row["open"] = json!(open.to_string());
                row["high"] = json!(high.to_string());
                row["low"] = json!(low.to_string());
                row["close"] = json!(close.to_string());
            }
            row
        }

        fn spec(&self) -> RawRefSpec {
            RawRefSpec {
                identity_key: format!("id-{}", self.coord),
                payload_hash: format!("ph-{}-{}", self.coord, self.revision),
                receipt_id: self.receipt.clone(),
                event_id: format!("ev-{}", self.coord),
                input_revision: self.revision.to_string(),
                revision: self.revision,
                seq: self.coord + 100,
                source_coord: self.coord.to_string(),
            }
        }
    }

    struct Fixture {
        store: CountingStore,
        producer: Cc004Producer,
        state: IncrFactsState,
        raw: BTreeMap<i64, Value>,
        /// 本用例的原始行（重绑真实当前 source 时用同一份事实）。
        rows: Vec<Row>,
    }

    /// 只建同次 parser 共享态与原始行；源版本登记交给调用方（内存表或真实事务）。
    fn unbound_fixture(bars: &[(usize, i64, i64)]) -> Fixture {
        let mut state = IncrFactsState::empty();
        let mut raw = BTreeMap::new();
        let mut rows = Vec::new();
        for (coord, high, low) in bars {
            // 与 `bar()` 同形：open = low、close = high（原始行的四价列）。
            let row = Row::new(*coord as i64, 1, "r1").with_prices(*low, *high, *low, *high);
            raw.insert(row.coord, row.raw_json());
            rows.push(row);
            state.append(bar(*coord, *high, *low));
        }
        Fixture {
            store: CountingStore::default(),
            producer: Cc004Producer::new(PROFILE, RULE),
            state,
            raw,
            rows,
        }
    }

    /// 内存表夹具：源版本登记在同一张可读写表里。
    fn fixture(bars: &[(usize, i64, i64)]) -> Fixture {
        fixture_with_profile(bars, PROFILE)
    }

    /// 同款夹具，可换会话冻结档案（进 §5.3 身份配方）。
    fn fixture_with_profile(bars: &[(usize, i64, i64)], profile: &str) -> Fixture {
        let mut fixture = unbound_fixture(bars);
        fixture.producer = Cc004Producer::new(profile, RULE);
        for row in &fixture.rows {
            assert!(fixture
                .producer
                .bind_source(&mut fixture.store, row.coord, row.spec())
                .expect("登记源版本"));
        }
        fixture
    }

    fn build_all(fixture: &mut Fixture) -> Vec<Cc004Object> {
        let Fixture {
            store,
            producer,
            state,
            ..
        } = fixture;
        build_objects(store, producer, state)
    }

    /// 按同一份共享态为每步构造对象（store 可换：内存表 / 真实 SQLite 事务）。
    fn build_objects(
        store: &mut dyn NodeStore,
        producer: &mut Cc004Producer,
        state: &IncrFactsState,
    ) -> Vec<Cc004Object> {
        (0..state.step_count())
            .map(|index| {
                producer
                    .build_step(&mut *store, &*state, index, GENERATION, CUT)
                    .expect("构造 CC-004")
                    .expect("步骤索引在域内")
            })
            .collect()
    }

    /// 生产构造器与旧 `Builder` 的完整对拍：三条 `*_json` 的**规范渲染字节**逐字相同，
    /// 共用对拍面：三条公开边的规范渲染字节 + 记录全部逻辑字段 + 全闭包可核。
    ///
    /// `object_id` 是 §5.3 的新配方（`obj3-`），与旧 `obj-` 不求逐字节相等——调用方须
    /// 另有独立配方测试；本函数只把该字段对齐后逐项对拍其余字段。
    fn assert_object_matches_oracle(
        fixture: &mut Fixture,
        object: &TypedObject,
        want: &Value,
        label: &str,
    ) {
        for (built, field) in [
            (&object.payload, "payload_json"),
            (&object.input_refs, "input_refs_json"),
            (&object.source_coords, "source_coords_json"),
        ] {
            assert_eq!(
                render(&mut fixture.store, built),
                want[field].as_str().expect("oracle 字段是文本"),
                "{label} {field} 规范字节"
            );
        }
        let mut expected = want.clone();
        expected["object_id"] = Value::String(object.object_id.clone());
        for (edge, field) in [
            ("payload", "payload_json"),
            ("input_refs", "input_refs_json"),
            ("source_coords", "source_coords_json"),
        ] {
            expected[edge] = serde_json::from_str(want[field].as_str().expect("oracle 字段是文本"))
                .expect("oracle 字段是合法 JSON");
        }
        let rendered = render(&mut fixture.store, &object.record);
        assert_eq!(
            serde_json::from_str::<Value>(&rendered).expect("记录渲染是合法 JSON"),
            expected,
            "{label} 记录字段"
        );
        verify(&mut fixture.store, &object.record.content_ref()).expect("记录闭包可核");
        for (_, edge) in object.content_edges() {
            verify(&mut fixture.store, &edge.content_ref()).expect("公开边闭包可核");
        }
    }

    /// oracle 里按 kind + 槽位找一条事实（找不到即失败，不静默回退）。
    fn oracle_fact<'a>(oracle: &'a [Value], kind: &str, fact_key: &str) -> &'a Value {
        oracle
            .iter()
            .find(|candidate| {
                candidate["kind"] == kind && candidate["fact_key"].as_str() == Some(fact_key)
            })
            .unwrap_or_else(|| panic!("oracle 缺 {kind} 槽位 {fact_key}"))
    }

    /// oracle CC-054 载荷里的全部知识请求（本代请求的唯一 oracle 面）。
    fn oracle_requests(oracle: &[Value]) -> Vec<Value> {
        let payload: Value = serde_json::from_str(
            oracle
                .iter()
                .find(|object| object["kind"] == "CC-054.knowledge_state")
                .expect("oracle 缺 CC-054")["payload_json"]
                .as_str()
                .expect("CC-054 载荷是文本"),
        )
        .expect("CC-054 载荷是合法 JSON");
        payload["requests"].as_array().expect("请求数组").clone()
    }

    /// 单条请求的对拍：按 `request_id` 对齐 oracle（键相同 ⟹ 同一条请求）。
    ///
    /// `subject_id` 是**本对象的身份**：§5.3 换配方后按新 `obj3` 对齐，其余字段逐字对拍。
    fn assert_request_matches_oracle(
        fixture: &mut Fixture,
        request: &Built,
        subject_id: &str,
        wants: &[Value],
        label: &str,
    ) {
        let rendered: Value = serde_json::from_str(&render(&mut fixture.store, request))
            .expect("请求渲染是合法 JSON");
        let request_id = rendered["request_id"].as_str().expect("请求键是文本");
        let mut want = wants
            .iter()
            .find(|candidate| candidate["request_id"].as_str() == Some(request_id))
            .unwrap_or_else(|| panic!("{label}: oracle 缺请求 {request_id}"))
            .clone();
        if !want["subject_id"].is_null() {
            want["subject_id"] = Value::String(subject_id.to_string());
        }
        assert_eq!(&rendered, &want, "{label} 请求字段");
    }

    /// 生产构造器与旧 `Builder` 的完整对拍：三条 `*_json` 的**规范渲染字节**逐字相同，
    /// 记录的全部逻辑字段（旧十项 + 三条公开内容边）逐字段相同，且整闭包可核。
    #[test]
    fn cc004_record_matches_full_oracle_bytes() {
        for (label, bars) in [
            ("merge/new-group/direction", CASE_MERGE),
            ("initial-unsettled/waiting", CASE_WAITING),
        ] {
            let mut fixture = fixture(bars);
            let objects = build_all(&mut fixture);
            let facts = fixture.state.materialize_facts();
            let (oracle, _) = tb02a_facts::build(&facts, &fixture.raw, &[], GENERATION, CUT, 0);
            assert_eq!(objects.len(), facts.steps.len(), "{label}: 步数");
            for (index, object) in objects.iter().enumerate() {
                let coord = facts.steps[index].incoming.source_index.to_string();
                let want_fact_key = canonical_json(&json!([CC004_STEP_KIND, coord]));
                let want = oracle
                    .iter()
                    .find(|candidate| {
                        candidate["kind"] == CC004_STEP_KIND
                            && candidate["fact_key"].as_str() == Some(want_fact_key.as_str())
                    })
                    .unwrap_or_else(|| panic!("{label} 步{index}: oracle 缺该步对象"));
                assert_eq!(object.fact_key, want_fact_key, "{label} 步{index} 槽位");
                assert_eq!(object.generation, GENERATION);
                assert_eq!(object.cut, CUT);
                // 旧逻辑字段：三条 *_json 的字节
                for (built, field) in [
                    (&object.payload, "payload_json"),
                    (&object.input_refs, "input_refs_json"),
                    (&object.source_coords, "source_coords_json"),
                ] {
                    assert_eq!(
                        render(&mut fixture.store, built),
                        want[field].as_str().expect("oracle 字段是文本"),
                        "{label} 步{index} {field} 规范字节"
                    );
                }
                // 完整逻辑字段：旧十项 + 三条公开边（边按内部值根接入，不是四字段描述对象）
                let mut expected = want.clone();
                // 身份配方不同（§5.3）：`obj3-` 由内容图派生，旧 `obj-` 由规范 JSON 文本派生，
                // 二者不求逐字节相等；先把该字段对齐，其余字段逐项对拍（obj3 配方另有独立测试）。
                let hex = object.object_id.strip_prefix("obj3-").unwrap_or_default();
                assert!(
                    hex.len() == 64
                        && hex
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
                    "{label} 步{index} obj3 形状：{}",
                    object.object_id
                );
                expected["object_id"] = Value::String(object.object_id.clone());
                for (edge, field) in [
                    ("payload", "payload_json"),
                    ("input_refs", "input_refs_json"),
                    ("source_coords", "source_coords_json"),
                ] {
                    expected[edge] =
                        serde_json::from_str(want[field].as_str().expect("oracle 字段是文本"))
                            .expect("oracle 字段是合法 JSON");
                }
                let rendered = render(&mut fixture.store, &object.record);
                assert_eq!(
                    serde_json::from_str::<Value>(&rendered).expect("记录渲染是合法 JSON"),
                    expected,
                    "{label} 步{index} 记录字段"
                );
                // 全闭包核验（含 json_text 内部与全部值边）
                verify(&mut fixture.store, &object.record.content_ref()).expect("记录闭包可核");
                for (_, edge) in object.content_edges() {
                    verify(&mut fixture.store, &edge.content_ref()).expect("公开边闭包可核");
                }
            }
        }
    }

    /// CC-005 组对象与旧 `Builder` 的完整对拍（同价极值 / 方向未定 / 稀疏坐标在域内），
    /// 并证明请求与 oracle 逐字段同值、索引越界不是错误。
    #[test]
    fn cc005_group_matches_full_oracle_bytes() {
        for (case, bars, want_equal_extreme, want_any_direction) in [
            ("merge/new-group/direction", CASE_MERGE, true, true),
            ("initial-unsettled/waiting", CASE_WAITING, false, false),
        ] {
            let mut fixture = fixture(bars);
            let facts = fixture.state.materialize_facts();
            let (oracle, _) = tb02a_facts::build(&facts, &fixture.raw, &[], GENERATION, CUT, 0);
            let wants = oracle_requests(&oracle);
            assert_eq!(fixture.state.group_count(), facts.groups.len());
            assert!(
                facts
                    .groups
                    .iter()
                    .any(|group| group.high_sources.len() > 1 || group.low_sources.len() > 1)
                    == want_equal_extreme,
                "{case}: 同价极值覆盖口径不符"
            );
            assert!(
                facts
                    .groups
                    .iter()
                    .any(|group| group.direction_evidence.is_some())
                    == want_any_direction,
                "{case}: 方向证据覆盖口径不符"
            );
            if !want_any_direction {
                assert!(
                    facts
                        .groups
                        .iter()
                        .all(|group| group.direction_evidence.is_none()),
                    "{case}: 初始方向永久未定时不得有方向证据"
                );
            }
            for index in 0..fixture.state.group_count() {
                let object = fixture
                    .producer
                    .build_group(&mut fixture.store, &fixture.state, index, GENERATION, CUT)
                    .expect("构造 CC-005")
                    .expect("组索引在域内");
                let anchor = facts.groups[index].bar.source_index.to_string();
                let want_fact_key = canonical_json(&json!([CC005_GROUP_KIND, anchor]));
                let want = oracle_fact(&oracle, CC005_GROUP_KIND, &want_fact_key);
                let label = format!("{case} 组{index}");
                assert_eq!(object.fact_key, want_fact_key, "{label} 槽位");
                assert_object_matches_oracle(&mut fixture, &object, want, &label);
                assert_request_matches_oracle(
                    &mut fixture,
                    object.request.as_ref().expect("组对象必须带请求"),
                    &object.object_id,
                    &wants,
                    &label,
                );
            }
            let past_end = fixture.state.group_count();
            assert!(fixture
                .producer
                .build_group(
                    &mut fixture.store,
                    &fixture.state,
                    past_end,
                    GENERATION,
                    CUT
                )
                .expect("越界索引不是错误")
                .is_none());
        }
    }

    /// 旧生成路径的同一条件：三组里任一组的同角色极值身份有竞争就不给分型选根。
    fn group_facts_competing(
        facts: &newchan_rust::theta_v0::parser::inclusion::InclusionFacts,
        start: usize,
    ) -> bool {
        facts.groups[start..start + 3]
            .iter()
            .any(|group| group.high_sources.len() != 1 || group.low_sources.len() != 1)
    }

    /// 测试侧 oracle：旧生成路径对每个可判窗口的 CC-006 旧行（`(窗口起点, 旧行)`）。
    ///
    /// 旧行同时是 `tb02a_facts::build` 的 `shapes` 入参（CC-007 与 CC-006 请求的 oracle 面）。
    fn oracle_shape_records(
        fixture: &mut Fixture,
        facts: &newchan_rust::theta_v0::parser::inclusion::InclusionFacts,
    ) -> Vec<(usize, Value)> {
        let merged_raws: Vec<Vec<i64>> = facts
            .groups
            .iter()
            .map(|group| group.members.iter().map(|coord| *coord as i64).collect())
            .collect();
        let mut records = Vec::new();
        for start in 0..facts.groups.len().saturating_sub(2) {
            let input_refs = oracle_shape_input_refs(facts, &fixture.raw, start);
            let (branch, dir_ab, dir_bc, comparisons_json) = oracle_shape_parts(
                &facts.merged[start],
                &facts.merged[start + 1],
                &facts.merged[start + 2],
            );
            let (want, _, _) = crate::build_object_and_witnesses(
                &facts.merged,
                &fixture.raw,
                &merged_raws,
                start,
                &branch,
                &dir_ab,
                &dir_bc,
                &comparisons_json,
                &input_refs,
                tb02a::PROFILE,
                GENERATION,
                CUT,
            );
            records.push((start, want));
        }
        records
    }

    /// 旧生成路径的 CC-006 八字段源引用（`s_structure_session` CC-006 生成调用点的原样）。
    ///
    /// **不是** [`tb02a_facts::raw_ref`]：那个口把 `revision`/`seq` 转成十进制文本，是泛型
    /// typed 对象（CC-004/007）的配方；CC-006 旧行直接取 raw 行原值（两者都是 i64 数值）。
    fn oracle_shape_raw_ref(event: &Value) -> Value {
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
    }

    /// 旧生成路径的 CC-006 三组 `input_refs`（测试侧 oracle：与
    /// `s_structure_session` 调用 `build_object_and_witnesses` 的构造逐字段相同）。
    fn oracle_shape_input_refs(
        facts: &newchan_rust::theta_v0::parser::inclusion::InclusionFacts,
        raw: &BTreeMap<i64, Value>,
        start: usize,
    ) -> Vec<Value> {
        (start..start + 3)
            .map(|mi| {
                let group = &facts.groups[mi];
                let raw_refs: Vec<Value> = group
                    .members
                    .iter()
                    .map(|coord| oracle_shape_raw_ref(&raw[&(*coord as i64)]))
                    .collect();
                let mut dependency_coords = group
                    .direction_evidence
                    .as_ref()
                    .map(|evidence| evidence.source_coords.clone())
                    .unwrap_or_default();
                if let Some(boundary) = &group.confirmation_evidence {
                    dependency_coords.extend(&boundary.source_coords);
                }
                dependency_coords.retain(|coord| !group.members.contains(coord));
                dependency_coords.sort_unstable();
                dependency_coords.dedup();
                let dependencies: Vec<Value> = dependency_coords
                    .iter()
                    .map(|coord| oracle_shape_raw_ref(&raw[&(*coord as i64)]))
                    .collect();
                json!({"merged_index": mi, "raw_refs": raw_refs, "dependency_refs": dependencies})
            })
            .collect()
    }

    /// 旧生成路径的 CC-006 分支/方向/比较文本（同一 `classify_local_shape` 判据与同一字段序）。
    fn oracle_shape_parts(left: &Bar, mid: &Bar, right: &Bar) -> (String, String, String, String) {
        match classify_local_shape(left, mid, right) {
            Cc006LocalShape::Classified {
                branch,
                dir_ab,
                dir_bc,
                comparisons,
            } => (
                branch.as_str().to_string(),
                dir_str(dir_ab).to_string(),
                dir_str(dir_bc).to_string(),
                json!(comparisons
                    .iter()
                    .map(|c| {
                        json!({
                            "axis": c.axis.as_str(),
                            "pair": c.pair.as_str(),
                            "prev": c.prev.to_string(),
                            "cur": c.cur.to_string(),
                            "strict_up": c.strict_up,
                            "held": c.held,
                        })
                    })
                    .collect::<Vec<_>>())
                .to_string(),
            ),
            other => panic!("测试窗口必须在可判域内：{other:?}"),
        }
    }

    /// CC-006 与**旧生成路径函数**（`build_object_and_witnesses`）对拍：
    /// `comparisons` / `input_refs` / `source_coords` 的规范渲染字节逐字相同，
    /// 身份按 §5.3 的独立十二字段配方重算并与旧 `obj-` 配方区分。
    #[test]
    fn cc006_shape_matches_generation_path_bytes() {
        // 旧生成路径的 `all_source_coords` 有一段按 `profile_id == tb02a::PROFILE` 分派；
        // 用同一档案跑两侧，才能对拍同一分支（档案本身也进身份配方）。
        let mut fixture = fixture_with_profile(CASE_SHAPE, tb02a::PROFILE);
        let facts = fixture.state.materialize_facts();
        assert!(facts.groups.len() >= 3, "本用例需要至少三组");
        let merged_raws: Vec<Vec<i64>> = facts
            .groups
            .iter()
            .map(|group| group.members.iter().map(|coord| *coord as i64).collect())
            .collect();
        let mut classified = 0usize;
        for start in 0..facts.groups.len() - 2 {
            let input_refs = oracle_shape_input_refs(&facts, &fixture.raw, start);
            let (branch, dir_ab, dir_bc, comparisons_json) = oracle_shape_parts(
                &facts.merged[start],
                &facts.merged[start + 1],
                &facts.merged[start + 2],
            );
            let (want, _, _) = crate::build_object_and_witnesses(
                &facts.merged,
                &fixture.raw,
                &merged_raws,
                start,
                &branch,
                &dir_ab,
                &dir_bc,
                &comparisons_json,
                &input_refs,
                tb02a::PROFILE,
                GENERATION,
                CUT,
            );
            let object = fixture
                .producer
                .build_shape(&mut fixture.store, &fixture.state, start, GENERATION, CUT)
                .expect("构造 CC-006")
                .expect("该窗口应可判");
            classified += 1;
            let label = format!("CC-006 窗口{start}");
            for (built, field) in [
                (&object.comparisons, "comparisons_json"),
                (&object.input_refs, "input_refs_json"),
                (&object.source_coords, "source_coords_json"),
            ] {
                assert_eq!(
                    render(&mut fixture.store, built),
                    want[field].as_str().expect("旧行是文本"),
                    "{label} {field} 规范字节"
                );
            }
            // 公开 wire 的 `input_refs`：与根 `project_input_refs` 对**同一旧行**的投影逐字节
            // 相同（数值 → 十进制文本），且与生成态数值根是两个不同的值根（两种编码不互代）。
            let old_row_refs: Value =
                serde_json::from_str(want["input_refs_json"].as_str().expect("旧行是文本"))
                    .expect("来源文本是合法 JSON");
            let projected =
                crate::project_input_refs(&old_row_refs).expect("旧行 input_refs 在投影域内");
            assert_eq!(
                render(&mut fixture.store, &object.wire_input_refs),
                canonical_json(&projected),
                "{label} wire input_refs 规范字节"
            );
            assert_ne!(
                object.input_refs.digest, object.wire_input_refs.digest,
                "{label} 生成态数值根与公开 wire 文本根不得同一"
            );
            // 记录：旧行全部字段 + 三条内容边（object_id 按 §5.3 新配方对齐后逐项对拍）。
            let mut expected = want.clone();
            expected["object_id"] = Value::String(object.object_id.clone());
            expected["comparisons"] =
                serde_json::from_str(want["comparisons_json"].as_str().expect("旧行是文本"))
                    .expect("比较文本是合法 JSON");
            expected["input_refs"] =
                serde_json::from_str(want["input_refs_json"].as_str().expect("旧行是文本"))
                    .expect("来源文本是合法 JSON");
            expected["source_coords"] =
                serde_json::from_str(want["source_coords_json"].as_str().expect("旧行是文本"))
                    .expect("坐标文本是合法 JSON");
            expected
                .as_object_mut()
                .expect("旧行是对象")
                .remove("comparisons_json");
            let rendered = render(&mut fixture.store, &object.record);
            assert_eq!(
                serde_json::from_str::<Value>(&rendered).expect("记录渲染是合法 JSON"),
                expected,
                "{label} 记录字段"
            );
            verify(&mut fixture.store, &object.record.content_ref()).expect("记录闭包可核");
            for edge in [
                &object.comparisons,
                &object.input_refs,
                &object.wire_input_refs,
                &object.source_coords,
            ] {
                verify(&mut fixture.store, &edge.content_ref()).expect("内容边闭包可核");
            }
            // 独立复算十二字段身份：不借构造器内部，按 §5.1 的 F 定义直接算。
            let mut identity_store = MemoryNodes::new();
            let identity = CValue::Object(vec![
                (
                    "branch".to_string(),
                    CValue::String(want["branch"].as_str().expect("分支").to_string()),
                ),
                (
                    "dir_ab".to_string(),
                    CValue::String(want["dir_ab"].as_str().expect("方向").to_string()),
                ),
                (
                    "dir_bc".to_string(),
                    CValue::String(want["dir_bc"].as_str().expect("方向").to_string()),
                ),
                (
                    "window_start".to_string(),
                    CValue::String(object.window[0].to_string()),
                ),
                (
                    "window_mid".to_string(),
                    CValue::String(object.window[1].to_string()),
                ),
                (
                    "window_end".to_string(),
                    CValue::String(object.window[2].to_string()),
                ),
                (
                    "merged_highs".to_string(),
                    CValue::Array(
                        (0..3)
                            .map(|offset| {
                                CValue::String(facts.merged[start + offset].high.to_string())
                            })
                            .collect(),
                    ),
                ),
                (
                    "merged_lows".to_string(),
                    CValue::Array(
                        (0..3)
                            .map(|offset| {
                                CValue::String(facts.merged[start + offset].low.to_string())
                            })
                            .collect(),
                    ),
                ),
                (
                    "comparisons".to_string(),
                    CValue::Edge(object.comparisons.digest),
                ),
                (
                    "input_refs".to_string(),
                    CValue::Edge(object.input_refs.digest),
                ),
                (
                    "rule_revision".to_string(),
                    CValue::String(RULE.to_string()),
                ),
                (
                    "profile_id".to_string(),
                    CValue::String(tb02a::PROFILE.to_string()),
                ),
            ]);
            let v = sc::write_value(&mut identity_store, &identity)
                .expect("十二字段身份对象")
                .digest;
            if std::env::var("DEBUG_SHAPE").is_ok() {
                let probe = sc::write_value(&mut fixture.store, &identity).expect("身份对象");
                println!("test identity text: {}", render(&mut fixture.store, &probe));
            }
            let mut hasher = Sha256::new();
            hasher.update(sc::DOMAIN_SEPARATOR);
            hasher.update([OBJECT_ID_TAG]);
            hasher.update(32u64.to_be_bytes());
            hasher.update(v);
            assert_eq!(object.identity, v, "{label} 暴露的 V 必须是配方输入");
            assert_eq!(
                object.object_id,
                format!("obj3-{}", lower_hex(&hasher.finalize())),
                "{label}: obj3 独立配方"
            );
            let legacy = sha256_hex(
                canonical_json(&json!({
                    "branch": want["branch"],
                    "dir_ab": want["dir_ab"],
                    "dir_bc": want["dir_bc"],
                    "window_start": object.window[0].to_string(),
                    "window_mid": object.window[1].to_string(),
                    "window_end": object.window[2].to_string(),
                    "merged_highs": (0..3)
                        .map(|offset| facts.merged[start + offset].high.to_string())
                        .collect::<Vec<_>>(),
                    "merged_lows": (0..3)
                        .map(|offset| facts.merged[start + offset].low.to_string())
                        .collect::<Vec<_>>(),
                    "comparisons": serde_json::from_str::<Value>(&comparisons_json)
                        .expect("比较文本"),
                    "input_refs": serde_json::from_str::<Value>(
                        want["input_refs_json"].as_str().expect("旧行是文本"),
                    )
                    .expect("来源文本"),
                    "rule_revision": RULE,
                    "profile_id": tb02a::PROFILE,
                }))
                .as_bytes(),
            );
            assert_ne!(
                object.object_id.trim_start_matches("obj3-"),
                legacy,
                "{label}: obj3 不是旧配方的换前缀"
            );
        }
        assert!(classified > 0, "本用例必须覆盖至少一个可判窗口");
        assert!(
            fixture
                .producer
                .build_shape(
                    &mut fixture.store,
                    &fixture.state,
                    facts.groups.len().saturating_sub(1),
                    GENERATION,
                    CUT,
                )
                .expect("越界窗口不是错误")
                .is_none(),
            "越界窗口必须返回 None"
        );
    }

    /// 源修订与 rollback：CC-006 两种 `input_refs` 编码（旧生成态数值叶 / 公开 wire 文本叶）
    /// 都必须随真实源版本改根；旧根在旧叶仍在同一 store 时仍完整可核；`discard_runtime_state`
    /// 后按真实当前源在**新 store** 重建，两种闭包与记录逐字节相同。
    #[test]
    fn cc006_shape_revision_moves_both_ref_encodings_and_rebuilds_after_discard() {
        let mut fixture = fixture_with_profile(CASE_SHAPE, tb02a::PROFILE);
        let facts = fixture.state.materialize_facts();
        assert!(facts.groups.len() >= 3, "本夹具至少一个可判窗口");
        let start = 0;
        let before = fixture
            .producer
            .build_shape(&mut fixture.store, &fixture.state, start, GENERATION, CUT)
            .expect("构造 CC-006")
            .expect("该窗口应可判");
        for edge in [&before.input_refs, &before.wire_input_refs, &before.record] {
            verify(&mut fixture.store, &edge.content_ref()).expect("修订前闭包可核");
        }

        // 真实修订：窗口内某成员坐标 revision+1（OHLC 结构不变），两种编码都必须改根。
        let coord = i64::try_from(facts.groups[start].members[0]).expect("坐标在 i64 域内");
        let mut revised = fixture
            .rows
            .iter()
            .find(|row| row.coord == coord)
            .expect("夹具行必须存在")
            .spec();
        revised.revision += 1;
        revised.input_revision = revised.revision.to_string();
        revised.payload_hash = format!("ph-{coord}-{}", revised.revision);
        revised.receipt_id = "r2".to_string();
        assert!(fixture
            .producer
            .bind_source(&mut fixture.store, coord, revised.clone())
            .expect("真实修订"));

        let after = fixture
            .producer
            .build_shape(&mut fixture.store, &fixture.state, start, GENERATION, CUT)
            .expect("构造 CC-006")
            .expect("该窗口应可判");
        assert_ne!(
            before.input_refs.digest, after.input_refs.digest,
            "生成态数值叶数组必须随源修订改根"
        );
        assert_ne!(
            before.wire_input_refs.digest, after.wire_input_refs.digest,
            "公开 wire 文本叶数组必须随源修订改根"
        );
        assert_ne!(
            before.identity, after.identity,
            "身份嵌 input_refs，必须改根"
        );
        assert_ne!(before.object_id, after.object_id);
        assert!(
            fixture.producer.memo_counts().shape_invalidated > 0,
            "数值叶 memo 必须按坐标失效（不只文本叶）"
        );
        // 修订后的两种闭包可核；旧根在旧叶仍在同一 store 时仍完整可核。
        for edge in [
            &after.input_refs,
            &after.wire_input_refs,
            &after.record,
            &before.input_refs,
            &before.wire_input_refs,
        ] {
            verify(&mut fixture.store, &edge.content_ref()).expect("闭包可核");
        }

        // rollback 口：运行态（含两种叶注册表与 memo）整份丢弃，按真实当前源在**新 store** 重建。
        fixture.producer.discard_runtime_state();
        let mut fresh = CountingStore::default();
        for row in &fixture.rows {
            let spec = if row.coord == coord {
                revised.clone()
            } else {
                row.spec()
            };
            fixture
                .producer
                .bind_source(&mut fresh, row.coord, spec)
                .expect("新域重新登记");
        }
        let rebuilt = fixture
            .producer
            .build_shape(&mut fresh, &fixture.state, start, GENERATION, CUT)
            .expect("构造 CC-006")
            .expect("该窗口应可判");
        assert_eq!(rebuilt.input_refs.digest, after.input_refs.digest);
        assert_eq!(rebuilt.wire_input_refs.digest, after.wire_input_refs.digest);
        assert_eq!(rebuilt.identity, after.identity);
        assert_eq!(rebuilt.object_id, after.object_id);
        assert_eq!(rebuilt.record.digest, after.record.digest);
        for edge in [
            &rebuilt.input_refs,
            &rebuilt.wire_input_refs,
            &rebuilt.record,
        ] {
            verify(&mut fresh, &edge.content_ref()).expect("新 store 闭包可核");
        }
    }

    /// oracle 载荷里的旧 `obj-` 身份换成本次构造的新 `obj3-` 身份（身份配方差异，非内容差异）。
    fn rewrite_legacy_ids(value: &mut Value, map: &BTreeMap<String, String>) {
        match value {
            Value::String(text) => {
                if let Some(new_id) = map.get(text) {
                    *text = new_id.clone();
                }
            }
            Value::Array(items) => items
                .iter_mut()
                .for_each(|item| rewrite_legacy_ids(item, map)),
            Value::Object(fields) => fields
                .values_mut()
                .for_each(|field| rewrite_legacy_ids(field, map)),
            _ => {}
        }
    }

    /// CC-054 与全量 oracle 的完整对拍：知识请求线序（步骤 → 组 → 待决形态 → 形态请求 +
    /// 三条 CC-007 请求 → 不足三根）、载荷/源引用/坐标三条文本列、记录与观察逐字段相同。
    ///
    /// 身份差异按 §5.3 对齐：请求主体与 `shape_object_id` 换成本次新 `obj3` 身份后再对拍。
    #[test]
    fn cc054_knowledge_matches_full_oracle_bytes() {
        for (case, bars, want_competing, want_two) in [
            ("shape/top-bottom", CASE_SHAPE, false, false),
            ("merge/equal-extremes", CASE_MERGE, true, false),
            ("two-merged/fewer-than-three", CASE_TWO, false, true),
        ] {
            let mut fixture = fixture_with_profile(bars, tb02a::PROFILE);
            let facts = fixture.state.materialize_facts();
            let records = oracle_shape_records(&mut fixture, &facts);
            let shapes_json: Vec<Value> = records.iter().map(|(_, want)| want.clone()).collect();
            let frontier = 0i64;
            // 旧行 oracle 先备好：下面构造 CC-007 时要用它的旧身份做映射。
            let (oracle, oracle_observations) = tb02a_facts::build(
                &facts,
                &fixture.raw,
                &shapes_json,
                GENERATION,
                CUT,
                frontier,
            );
            let mut requests: Vec<Built> = Vec::new();
            let mut reasons: Vec<&'static str> = Vec::new();
            let mut id_map: BTreeMap<String, String> = BTreeMap::new();
            // 1) 步骤
            for index in 0..fixture.state.step_count() {
                let object = fixture
                    .producer
                    .build_step(&mut fixture.store, &fixture.state, index, GENERATION, CUT)
                    .expect("构造 CC-004")
                    .expect("步骤索引在域内");
                // 请求主体 = 对象自身：旧身份一并换成本代新 `obj3`。
                id_map.insert(
                    oracle_fact(&oracle, CC004_STEP_KIND, &object.fact_key)["object_id"]
                        .as_str()
                        .expect("旧身份")
                        .to_string(),
                    object.object_id.clone(),
                );
                requests.push(object.request.clone().expect("步骤请求"));
                reasons.extend(
                    facts.steps[index]
                        .waiting_reasons
                        .iter()
                        .copied()
                        .filter(|reason| *reason != "equal_extreme_identity"),
                );
            }
            // 2) 组
            for index in 0..fixture.state.group_count() {
                let object = fixture
                    .producer
                    .build_group(&mut fixture.store, &fixture.state, index, GENERATION, CUT)
                    .expect("构造 CC-005")
                    .expect("组索引在域内");
                id_map.insert(
                    oracle_fact(&oracle, CC005_GROUP_KIND, &object.fact_key)["object_id"]
                        .as_str()
                        .expect("旧身份")
                        .to_string(),
                    object.object_id.clone(),
                );
                let view = fixture
                    .state
                    .shared_group(index)
                    .expect("共享态可用")
                    .expect("组在域内");
                reasons.extend(group_reasons(&view));
                requests.push(object.request.clone().expect("组请求"));
            }
            // #1405：多根极值仍覆盖，但不再生成待决形态请求。
            let competing = (0..facts.groups.len().saturating_sub(2))
                .filter(|start| group_facts_competing(&facts, *start))
                .count();
            assert_eq!(competing > 0, want_competing, "{case}: 多根窗覆盖口径不符");
            // 4) 形态请求 + 三条 CC-007 请求（同序，逐个窗口）
            for (start, shape_record) in &records {
                let shape = fixture
                    .producer
                    .build_shape(&mut fixture.store, &fixture.state, *start, GENERATION, CUT)
                    .expect("构造 CC-006")
                    .expect("该窗口应可判");
                id_map.insert(
                    shape_record["object_id"]
                        .as_str()
                        .expect("旧身份")
                        .to_string(),
                    shape.object_id.clone(),
                );
                requests.push(shape.request.clone());
                let window = shape.window;
                let views = [start + 0, start + 1, start + 2].map(|index| {
                    fixture
                        .state
                        .shared_group(index)
                        .expect("共享态可用")
                        .expect("组在域内")
                });
                let raw_bars: Vec<Bar> = fixture.state.raw_iter().copied().collect();
                let Some(fractal) = fixture
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
                else {
                    assert_eq!(
                        window.len(),
                        3,
                        "{case}: 非 TOP/BOTTOM 形态仍应有 CC-006 请求"
                    );
                    continue;
                };
                // CC-007 三条请求的主体是**本对象自身**的旧身份：一并换成本代新 `obj3`。
                let fractal_fact_key = canonical_json(&json!([
                    CC007_FRACTAL_KIND,
                    window.iter().map(ToString::to_string).collect::<Vec<_>>()
                ]));
                id_map.insert(
                    oracle_fact(&oracle, CC007_FRACTAL_KIND, &fractal_fact_key)["object_id"]
                        .as_str()
                        .expect("旧身份")
                        .to_string(),
                    fractal.object_id.clone(),
                );
                requests.extend(fractal.requests.iter().cloned());
                reasons.extend(fractal.knowledge_reasons.iter().copied());
            }
            // 5) 不足三根 merged 的待决请求
            if facts.merged.len() < 3 {
                let all = raw_prefix_coords(&fixture.state).expect("raw 前缀链可用");
                requests.push(
                    fixture
                        .producer
                        .pending_request(
                            &mut fixture.store,
                            CValue::Array(vec![
                                CValue::String(CC006_SHAPE_KIND.to_string()),
                                CValue::String("pending".to_string()),
                            ]),
                            &["fewer_than_three_merged_bars"],
                            &all,
                        )
                        .expect("不足三根请求"),
                );
                reasons.push("fewer_than_three_merged_bars");
                assert!(want_two, "{case}: 不足三根覆盖口径不符");
            }
            if !facts
                .steps
                .iter()
                .any(|step| step.direction_evidence.is_some())
            {
                reasons.push("direction_not_yet_established");
            }
            if facts.initial_direction_unsettled {
                reasons.push("initial_direction_unsettled");
            }
            let raw_coords = raw_prefix_coords(&fixture.state).expect("raw 前缀链可用");
            let assembly = fixture
                .producer
                .build_knowledge_state(
                    &mut fixture.store,
                    &raw_coords,
                    frontier,
                    &requests,
                    &reasons,
                    GENERATION,
                    CUT,
                )
                .expect("构造 CC-054");
            let fact_key = canonical_json(&json!([CC054_KNOWLEDGE_KIND, "TB-02-A"]));
            let want = oracle_fact(&oracle, CC054_KNOWLEDGE_KIND, &fact_key);
            let label = format!("{case} CC-054");
            for (built, field) in [
                (&assembly.object.input_refs, "input_refs_json"),
                (&assembly.object.source_coords, "source_coords_json"),
            ] {
                assert_eq!(
                    render(&mut fixture.store, built),
                    want[field].as_str().expect("oracle 字段是文本"),
                    "{label} {field} 规范字节"
                );
            }
            let mut want_payload: Value =
                serde_json::from_str(want["payload_json"].as_str().expect("oracle 载荷是文本"))
                    .expect("载荷是合法 JSON");
            rewrite_legacy_ids(&mut want_payload, &id_map);
            let got_payload: Value =
                serde_json::from_str(&render(&mut fixture.store, &assembly.object.payload))
                    .expect("载荷渲染是合法 JSON");
            assert_eq!(got_payload, want_payload, "{label} 载荷（身份对齐后）");
            let mut expected = want.clone();
            expected["object_id"] = Value::String(assembly.object.object_id.clone());
            expected["payload"] = got_payload;
            // 文本列 = 对应内容边在本代新身份下的规范文本（旧行的文本里还嵌着 `obj-` 身份）。
            expected["payload_json"] =
                Value::String(render(&mut fixture.store, &assembly.object.payload));
            for (edge, field) in [
                ("input_refs", "input_refs_json"),
                ("source_coords", "source_coords_json"),
            ] {
                expected[edge] =
                    serde_json::from_str(want[field].as_str().expect("oracle 字段是文本"))
                        .expect("oracle 字段是合法 JSON");
                expected[field] = Value::String(render(
                    &mut fixture.store,
                    if edge == "input_refs" {
                        &assembly.object.input_refs
                    } else {
                        &assembly.object.source_coords
                    },
                ));
            }
            let rendered = render(&mut fixture.store, &assembly.object.record);
            assert_eq!(
                serde_json::from_str::<Value>(&rendered).expect("记录渲染是合法 JSON"),
                expected,
                "{label} 记录字段"
            );
            verify(&mut fixture.store, &assembly.object.record.content_ref())
                .expect("记录闭包可核");
            assert_eq!(
                assembly.observations.len(),
                oracle_observations.len(),
                "{label} 观察条数"
            );
            for (index, observation) in assembly.observations.iter().enumerate() {
                assert_eq!(
                    serde_json::from_str::<Value>(&render(&mut fixture.store, observation))
                        .expect("观察渲染是合法 JSON"),
                    oracle_observations[index],
                    "{label} 观察{index}"
                );
                verify(&mut fixture.store, &observation.content_ref()).expect("观察闭包可核");
            }
            assert!(
                assembly.object.request.is_none(),
                "{label}: CC-054 旧配方无请求"
            );
        }
    }

    /// CC-007 与全量 oracle 的对拍：源引用/坐标两条文本列逐字节相同，载荷与三条请求逐字段
    /// 同值（载荷内嵌与请求 `subject_id` 的旧身份按新 `obj3` 对齐），并覆盖「有后续历史」
    /// 与「无后续 bar」两态。
    #[test]
    fn cc007_fractal_matches_full_oracle_bytes() {
        let mut fixture = fixture_with_profile(CASE_SHAPE, tb02a::PROFILE);
        let facts = fixture.state.materialize_facts();
        let records = oracle_shape_records(&mut fixture, &facts);
        assert!(
            records.len() >= 3,
            "本用例需要至少三个可判窗口，实得 {}",
            records.len()
        );
        let shapes: Vec<Value> = records.iter().map(|(_, want)| want.clone()).collect();
        let (oracle, _) = tb02a_facts::build(&facts, &fixture.raw, &shapes, GENERATION, CUT, 0);
        let wants = oracle_requests(&oracle);
        let raw_bars: Vec<Bar> = fixture.state.raw_iter().copied().collect();
        let mut saw_future = false;
        let mut saw_insufficient = false;
        for (start, shape_record) in &records {
            let shape = fixture
                .producer
                .build_shape(&mut fixture.store, &fixture.state, *start, GENERATION, CUT)
                .expect("构造 CC-006")
                .expect("该窗口应可判");
            // 身份配方不同（§5.3）：CC-006 的十二字段 `obj3-` 身份由 CC-006 测试独立复算锁定；
            // 此处只锁形状与「不是旧配方换前缀」，不要求与旧行 `obj-` 逐字相等。
            assert!(
                shape.object_id.starts_with("obj3-") && shape.object_id.len() == 5 + 64,
                "窗口{start}: 形态身份形状"
            );
            assert_ne!(
                shape.object_id.trim_start_matches("obj3-"),
                shape_record["object_id"]
                    .as_str()
                    .expect("旧身份")
                    .trim_start_matches("obj-"),
                "窗口{start}: 新身份不得退化成旧配方换前缀"
            );
            let window: Vec<usize> = ["window_start", "window_mid", "window_end"]
                .iter()
                .map(|key| shape_record[*key].as_i64().expect("窗口坐标") as usize)
                .collect();
            let views = [window[0], window[1], window[2]].map(|anchor| {
                let index = facts
                    .groups
                    .iter()
                    .position(|group| group.bar.source_index == anchor)
                    .expect("锚在组表里");
                fixture
                    .state
                    .shared_group(index)
                    .expect("共享态可用")
                    .expect("组在域内")
            });
            let object = fixture
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
                .expect("TOP/BOTTOM 窗口应产对象");
            let window_text: Vec<String> = window.iter().map(ToString::to_string).collect();
            let want_fact_key = canonical_json(&json!([CC007_FRACTAL_KIND, window_text]));
            assert_eq!(object.fact_key, want_fact_key, "窗口{start} 槽位");
            let want = oracle_fact(&oracle, CC007_FRACTAL_KIND, &want_fact_key);
            let label = format!("CC-007 窗口{start}");
            for (built, field) in [
                (&object.input_refs, "input_refs_json"),
                (&object.source_coords, "source_coords_json"),
            ] {
                assert_eq!(
                    render(&mut fixture.store, built),
                    want[field].as_str().expect("oracle 字段是文本"),
                    "{label} {field} 规范字节"
                );
            }
            // 载荷只差身份配方（§5.3）：载荷内嵌 `shape_object_id`，把旧行身份换成本次
            // 新 `obj3` 后逐字段对拍（新身份本身已由独立复算锁）。
            let mut want_payload: Value =
                serde_json::from_str(want["payload_json"].as_str().expect("oracle 载荷是文本"))
                    .expect("载荷是合法 JSON");
            let mut id_map: BTreeMap<String, String> = BTreeMap::new();
            id_map.insert(
                shape_record["object_id"]
                    .as_str()
                    .expect("旧身份")
                    .to_string(),
                shape.object_id.clone(),
            );
            rewrite_legacy_ids(&mut want_payload, &id_map);
            let got_payload: Value =
                serde_json::from_str(&render(&mut fixture.store, &object.payload))
                    .expect("载荷渲染是合法 JSON");
            assert_eq!(got_payload, want_payload, "{label} 载荷（身份对齐后）");
            let mut expected = want.clone();
            expected["object_id"] = Value::String(object.object_id.clone());
            expected["payload"] = got_payload;
            // 文本列 = 对应内容边在本代新身份下的规范文本（旧行文本里嵌着旧 `obj-` 身份）。
            expected["payload_json"] = Value::String(render(&mut fixture.store, &object.payload));
            expected["input_refs_json"] =
                Value::String(render(&mut fixture.store, &object.input_refs));
            expected["source_coords_json"] =
                Value::String(render(&mut fixture.store, &object.source_coords));
            for (edge, field) in [
                ("input_refs", "input_refs_json"),
                ("source_coords", "source_coords_json"),
            ] {
                expected[edge] =
                    serde_json::from_str(want[field].as_str().expect("oracle 字段是文本"))
                        .expect("oracle 字段是合法 JSON");
            }
            let rendered = render(&mut fixture.store, &object.record);
            assert_eq!(
                serde_json::from_str::<Value>(&rendered).expect("记录渲染是合法 JSON"),
                expected,
                "{label} 记录字段"
            );
            verify(&mut fixture.store, &object.record.content_ref()).expect("记录闭包可核");
            // 独立复算泛型六字段身份（不借构造器的 `identity_summary`）。
            let mut identity_store = MemoryNodes::new();
            let identity_value = CValue::Object(vec![
                (
                    "kind".to_string(),
                    CValue::String(CC007_FRACTAL_KIND.to_string()),
                ),
                (
                    "fact_key".to_string(),
                    CValue::String(object.fact_key.clone()),
                ),
                ("payload".to_string(), CValue::Edge(object.payload.digest)),
                (
                    "input_refs".to_string(),
                    CValue::Edge(object.input_refs.digest),
                ),
                (
                    "profile_id".to_string(),
                    CValue::String(tb02a::PROFILE.to_string()),
                ),
                (
                    "rule_revision".to_string(),
                    CValue::String(RULE.to_string()),
                ),
            ]);
            let want_identity = sc::write_value(&mut identity_store, &identity_value)
                .expect("身份对象可写")
                .digest;
            assert_eq!(object.identity, want_identity, "{label}: 泛型身份 V");
            assert_eq!(
                object.object_id,
                format!(
                    "obj3-{}",
                    lower_hex(&sc::frame(OBJECT_ID_TAG, &want_identity))
                ),
                "{label}: obj3 配方"
            );
            let status = serde_json::from_str::<Value>(
                want["payload_json"].as_str().expect("oracle 字段是文本"),
            )
            .expect("载荷是合法 JSON")["subsequent_development"]["status"]
                .as_str()
                .expect("状态是文本")
                .to_string();
            match status.as_str() {
                "observed" => saw_future = true,
                "insufficient_knowledge" => saw_insufficient = true,
                other => panic!("{label}: 未知后续状态 {other}"),
            }
            assert_eq!(object.requests.len(), 3, "{label}: 恰三条请求");
            for request in &object.requests {
                assert_request_matches_oracle(
                    &mut fixture,
                    request,
                    &object.object_id,
                    &wants,
                    &label,
                );
            }
        }
        assert!(saw_future, "必须覆盖有后续历史的窗口");
        assert!(saw_insufficient, "必须覆盖无后续 bar 的窗口");
    }

    /// #1394 固定 cold32/cold64/warm65 样本的真实 put 诊断，默认不参与常规测试。
    #[test]
    #[ignore = "#1394 有界 material 计量；由主控显式运行，不是计时基准"]
    fn parser_fractal_put_material_inventory_cold32_cold64_warm65() {
        fn row(i: usize) -> (usize, i64, i64) {
            (
                10 + i * 17,
                if i % 2 == 0 { 12 } else { 24 },
                if i % 2 == 0 { 3 } else { 15 },
            )
        }

        fn measured_fixture(rows: &[(usize, i64, i64)]) -> Fixture {
            let mut fixture = unbound_fixture(rows);
            fixture.producer = Cc004Producer::new(tb02a::PROFILE, RULE);
            fixture.store.emissions = Some(EmissionInventory {
                phase: "bind_sources",
                ..EmissionInventory::default()
            });
            for row in &fixture.rows {
                assert!(fixture
                    .producer
                    .bind_source(&mut fixture.store, row.coord, row.spec())
                    .unwrap());
            }
            fixture
        }

        fn append(fixture: &mut Fixture, next: (usize, i64, i64)) {
            fixture.store.reads = 0;
            fixture.store.puts = 0;
            fixture.store.array_puts = 0;
            fixture.store.branch_puts = 0;
            fixture.store.emissions = Some(EmissionInventory {
                phase: "bind_sources",
                ..EmissionInventory::default()
            });
            let (coord, high, low) = next;
            let source = Row::new(coord as i64, 1, "r1").with_prices(low, high, low, high);
            assert!(fixture
                .producer
                .bind_source(&mut fixture.store, source.coord, source.spec())
                .unwrap());
            fixture.raw.insert(source.coord, source.raw_json());
            fixture.rows.push(source);
            fixture.state.append(bar(coord, high, low));
        }

        fn run(
            fixture: &mut Fixture,
            reuse: bool,
            generation: i64,
            sample: &str,
        ) -> (Vec<Cc007Object>, Value) {
            let _phases = enable_emission_phases();
            let history_before = (
                fixture.producer.history_computed,
                fixture.producer.history_reused,
            );
            let raw: Vec<Bar> = fixture.state.raw_iter().copied().collect();
            fixture.store.emissions.as_mut().unwrap().phase = "prepare_and_parser_session";
            let prepared = fixture.producer.prepare_raw_history(&raw).unwrap();
            let mut session = reuse.then(|| {
                fixture
                    .producer
                    .parser_fractal_sources(&prepared, &fixture.state)
                    .unwrap()
            });
            let count = fixture.state.materialize_facts().groups.len();
            let mut objects = Vec::new();
            let mut branches: BTreeMap<&str, (usize, usize, usize, usize)> = BTreeMap::new();
            enum SampleShape {
                Native(ParserFractalShape),
                General(Cc006Object),
            }
            for start in 0..count.saturating_sub(2) {
                fixture.store.emissions.as_mut().unwrap().phase = "shape";
                let shape_start = fixture.store.puts;
                let shape = if let Some(session) = &mut session {
                    fixture
                        .producer
                        .build_parser_shape(
                            &mut fixture.store,
                            &prepared,
                            session,
                            start,
                            generation,
                            CUT,
                        )
                        .unwrap()
                        .map(SampleShape::Native)
                } else {
                    fixture
                        .producer
                        .build_shape(&mut fixture.store, &fixture.state, start, generation, CUT)
                        .unwrap()
                        .map(SampleShape::General)
                };
                let shape_calls = fixture.store.puts - shape_start;
                let Some(shape) = shape else {
                    continue;
                };
                let branch = match &shape {
                    SampleShape::Native(shape) => shape.object().branch,
                    SampleShape::General(shape) => shape.branch,
                };
                assert!(matches!(branch, "TOP" | "BOTTOM"));
                let views = [start, start + 1, start + 2]
                    .map(|index| fixture.state.shared_group(index).unwrap().unwrap());
                let unit_start = fixture.store.emissions.as_ref().unwrap().unit_calls();
                fixture.store.emissions.as_mut().unwrap().phase = "fractal_other";
                let fractal_start = fixture.store.puts;
                let object = match &shape {
                    SampleShape::Native(shape) => fixture
                        .producer
                        .build_parser_fractal(
                            &mut fixture.store,
                            &prepared,
                            session
                                .as_mut()
                                .expect("Native shape 必须保留 parser session"),
                            shape,
                            generation,
                            CUT,
                        )
                        .unwrap(),
                    SampleShape::General(shape) => fixture
                        .producer
                        .build_fractal(
                            &mut fixture.store,
                            &prepared,
                            shape,
                            [&views[0], &views[1], &views[2]],
                            generation,
                            CUT,
                        )
                        .unwrap(),
                }
                .expect("TOP/BOTTOM 窗口必产 CC007");
                let unit_calls =
                    fixture.store.emissions.as_ref().unwrap().unit_calls() - unit_start;
                let branch = branches.entry(branch).or_default();
                branch.0 += 1;
                branch.1 += shape_calls;
                branch.2 += fixture.store.puts - fractal_start;
                branch.3 += unit_calls;
                objects.push(object);
            }
            assert_eq!(objects.len(), raw.len() - 2);
            let whole_builds = session
                .as_ref()
                .map_or(objects.len(), |s| s.whole_source_builds);
            assert_eq!(whole_builds, if reuse { 1 } else { objects.len() });
            let mut output = fixture.store.emissions.as_ref().unwrap().summary();
            assert_eq!(output["calls"].as_u64(), Some(fixture.store.puts as u64));
            output["sample"] = json!(sample);
            output["arm"] = json!(if reuse { "native_shared" } else { "general" });
            output["raw"] = json!(raw.len());
            output["windows"] = json!(objects.len());
            output["whole_source_builds"] = json!(whole_builds);
            output["reads"] = json!(fixture.store.reads);
            output["history_computed_delta"] =
                json!(fixture.producer.history_computed - history_before.0);
            output["history_reused_delta"] =
                json!(fixture.producer.history_reused - history_before.1);
            if sample == "warm65" {
                assert_eq!(
                    fixture.producer.history_computed - history_before.0,
                    raw.len() - 3,
                    "64→65 暖追加仅给既有窗口追加一根后续 bar"
                );
                assert!(
                    fixture.producer.history_reused > history_before.1,
                    "暖追加必须保留并消费合法 history"
                );
            }
            output["branches"] = json!(branches
                .into_iter()
                .map(|(branch, counts)| {
                    json!({"branch": branch, "windows": counts.0, "shape_calls": counts.1,
                    "fractal_calls": counts.2, "fractal_units_prefix_calls": counts.3})
                })
                .collect::<Vec<_>>());
            (objects, output)
        }

        for n in [32usize, 64] {
            let rows: Vec<_> = (0..n).map(row).collect();
            let mut general = measured_fixture(&rows);
            let mut native = measured_fixture(&rows);
            let sample = format!("cold{n}");
            let (expected, general_summary) = run(&mut general, false, GENERATION, &sample);
            let (actual, native_summary) = run(&mut native, true, GENERATION, &sample);
            // 规范渲染/完整材料 parity 的读数发生在冻结 summary 之后。
            assert_parser_fractals_equal(&mut general, &mut native, &expected, &actual);
            println!("CC007_EMISSION_INVENTORY {}", general_summary);
            println!("CC007_EMISSION_INVENTORY {}", native_summary);
            if n == 64 {
                append(&mut general, row(64));
                append(&mut native, row(64));
                let (expected, general_summary) =
                    run(&mut general, false, GENERATION + 1, "warm65");
                let (actual, native_summary) = run(&mut native, true, GENERATION + 1, "warm65");
                assert_parser_fractals_equal(&mut general, &mut native, &expected, &actual);
                println!("CC007_EMISSION_INVENTORY {}", general_summary);
                println!("CC007_EMISSION_INVENTORY {}", native_summary);
            }
        }
    }

    /// 只作测试 oracle 的双入口运行器；资格仍由生产 parser session 构造。
    fn collect_parser_fractals(
        fixture: &mut Fixture,
        reuse: bool,
        generation: i64,
    ) -> (Vec<Cc007Object>, usize) {
        let raw: Vec<Bar> = fixture.state.raw_iter().copied().collect();
        let prepared = fixture.producer.prepare_raw_history(&raw).unwrap();
        let mut session = reuse.then(|| {
            fixture
                .producer
                .parser_fractal_sources(&prepared, &fixture.state)
                .unwrap()
        });
        let count = fixture.state.materialize_facts().groups.len();
        let mut objects = Vec::new();
        for start in 0..count.saturating_sub(2) {
            let fractal = if let Some(session) = &mut session {
                let Some(shape) = fixture
                    .producer
                    .build_parser_shape(
                        &mut fixture.store,
                        &prepared,
                        session,
                        start,
                        generation,
                        CUT,
                    )
                    .unwrap()
                else {
                    continue;
                };
                fixture
                    .producer
                    .build_parser_fractal(
                        &mut fixture.store,
                        &prepared,
                        session,
                        &shape,
                        generation,
                        CUT,
                    )
                    .unwrap()
            } else {
                let Some(shape) = fixture
                    .producer
                    .build_shape(&mut fixture.store, &fixture.state, start, generation, CUT)
                    .unwrap()
                else {
                    continue;
                };
                let views = [start, start + 1, start + 2]
                    .map(|index| fixture.state.shared_group(index).unwrap().unwrap());
                fixture
                    .producer
                    .build_fractal(
                        &mut fixture.store,
                        &prepared,
                        &shape,
                        [&views[0], &views[1], &views[2]],
                        generation,
                        CUT,
                    )
                    .unwrap()
            };
            if let Some(object) = fractal {
                objects.push(object);
            }
        }
        let builds = session
            .as_ref()
            .map_or(objects.len(), |session| session.whole_source_builds);
        (objects, builds)
    }

    fn assert_parser_fractals_equal(
        old: &mut Fixture,
        shared: &mut Fixture,
        expected: &[Cc007Object],
        actual: &[Cc007Object],
    ) {
        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.iter().zip(actual) {
            assert_eq!(expected.object_id, actual.object_id);
            assert_eq!(expected.identity, actual.identity);
            assert_eq!(expected.fact_key, actual.fact_key);
            assert_eq!(expected.request_reasons, actual.request_reasons);
            assert_eq!(expected.knowledge_reasons, actual.knowledge_reasons);
            for (expected, actual) in [
                (&expected.record, &actual.record),
                (&expected.payload, &actual.payload),
                (&expected.input_refs, &actual.input_refs),
                (&expected.source_coords, &actual.source_coords),
                (&expected.payload_json, &actual.payload_json),
                (&expected.input_refs_json, &actual.input_refs_json),
                (&expected.source_coords_json, &actual.source_coords_json),
            ]
            .into_iter()
            .chain(expected.requests.iter().zip(&actual.requests))
            {
                assert_eq!(expected.digest, actual.digest);
                assert_eq!(expected.kind, actual.kind);
                assert_eq!(expected.extent, actual.extent);
                assert_eq!(
                    render(&mut old.store, expected),
                    render(&mut shared.store, actual)
                );
            }
        }
    }

    /// `verify` 全闭包实际读到的节点即可达集（空序列按规范不读节点，单独豁免）。
    fn reachable_nodes(
        nodes: &mut MemoryNodes,
        refs: &[sc::ContentRef],
    ) -> std::collections::BTreeSet<NodeDigest> {
        struct Reads<'a> {
            nodes: &'a mut MemoryNodes,
            seen: std::collections::BTreeSet<NodeDigest>,
        }
        impl NodeSource for Reads<'_> {
            fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
                self.seen.insert(*digest);
                self.nodes.read(digest)
            }
        }
        let mut reads = Reads {
            nodes,
            seen: std::collections::BTreeSet::new(),
        };
        for reference in refs {
            verify(&mut reads, reference).expect("产出全闭包可核");
        }
        reads.seen.insert(SequenceRef::empty().root());
        reads.seen
    }

    /// 一代全部 CC-007 窗口（`reuse` 选 parser session 口）；另返回**只在 fractal 构造调用内**
    /// 新落盘的节点集（CC-006 形状对象不在本锁范围）。
    fn fractal_generation_with_new_nodes(
        fixture: &mut Fixture,
        reuse: bool,
        generation: i64,
    ) -> (Vec<Cc007Object>, std::collections::BTreeSet<NodeDigest>) {
        let raw: Vec<Bar> = fixture.state.raw_iter().copied().collect();
        let prepared = fixture.producer.prepare_raw_history(&raw).unwrap();
        let mut session = reuse.then(|| {
            fixture
                .producer
                .parser_fractal_sources(&prepared, &fixture.state)
                .unwrap()
        });
        let count = fixture.state.materialize_facts().groups.len();
        let mut objects = Vec::new();
        let mut written = std::collections::BTreeSet::new();
        let stored = |store: &CountingStore| -> std::collections::BTreeSet<NodeDigest> {
            store.nodes.iter().map(|(digest, _, _)| *digest).collect()
        };
        for start in 0..count.saturating_sub(2) {
            let fractal = if let Some(session) = &mut session {
                let Some(shape) = fixture
                    .producer
                    .build_parser_shape(
                        &mut fixture.store,
                        &prepared,
                        session,
                        start,
                        generation,
                        CUT,
                    )
                    .unwrap()
                else {
                    continue;
                };
                let before = stored(&fixture.store);
                let fractal = fixture
                    .producer
                    .build_parser_fractal(
                        &mut fixture.store,
                        &prepared,
                        session,
                        &shape,
                        generation,
                        CUT,
                    )
                    .unwrap();
                written.extend(stored(&fixture.store).difference(&before).copied());
                fractal
            } else {
                let Some(shape) = fixture
                    .producer
                    .build_shape(&mut fixture.store, &fixture.state, start, generation, CUT)
                    .unwrap()
                else {
                    continue;
                };
                let views = [start, start + 1, start + 2]
                    .map(|index| fixture.state.shared_group(index).unwrap().unwrap());
                let before = stored(&fixture.store);
                let fractal = fixture
                    .producer
                    .build_fractal(
                        &mut fixture.store,
                        &prepared,
                        &shape,
                        [&views[0], &views[1], &views[2]],
                        generation,
                        CUT,
                    )
                    .unwrap();
                written.extend(stored(&fixture.store).difference(&before).copied());
                fractal
            };
            if let Some(object) = fractal {
                objects.push(object);
            }
        }
        (objects, written)
    }

    /// #1488 S3 锁：fractal 历史续接（每代追加多根 bar）一次 `sequence_rebuild_suffix` 接尾——
    /// 逐代 fractal 调用内新写节点全部可从本代 CC-007 产出闭包到达（不可达 = 0，不落逐 bar
    /// 作废中间根），且末代暖续接产出与冷全量逐位相同（#804）。通用口与 parser session 口各跑一遍。
    #[test]
    fn fractal_history_suffix_writes_no_orphans_and_matches_cold() {
        fn row(i: usize) -> (usize, i64, i64) {
            (
                10 + i * 17,
                if i % 2 == 0 { 12 } else { 24 },
                if i % 2 == 0 { 3 } else { 15 },
            )
        }
        fn append(fixture: &mut Fixture, next: (usize, i64, i64)) {
            let (coord, high, low) = next;
            let source = Row::new(coord as i64, 1, "r1").with_prices(low, high, low, high);
            assert!(fixture
                .producer
                .bind_source(&mut fixture.store, source.coord, source.spec())
                .unwrap());
            fixture.raw.insert(source.coord, source.raw_json());
            fixture.rows.push(source);
            fixture.state.append(bar(coord, high, low));
        }
        let all: Vec<_> = (0..40).map(row).collect();
        for reuse in [false, true] {
            let mut warm = fixture_with_profile(&all[..24], tb02a::PROFILE);
            let mut generation = GENERATION;
            let mut last = Vec::new();
            let mut covered = 0usize;
            for (step, grow) in [0usize, 5, 7, 4].into_iter().enumerate() {
                for _ in 0..grow {
                    let next = all[warm.rows.len()];
                    append(&mut warm, next);
                }
                generation = GENERATION + step as i64;
                let _phases = enable_emission_phases();
                warm.store.emissions = Some(EmissionInventory {
                    phase: "fractal_other",
                    ..EmissionInventory::default()
                });
                let (objects, written) =
                    fractal_generation_with_new_nodes(&mut warm, reuse, generation);
                let refs: Vec<sc::ContentRef> = objects
                    .iter()
                    .flat_map(|object| {
                        [
                            &object.record,
                            &object.payload,
                            &object.input_refs,
                            &object.source_coords,
                            &object.payload_json,
                            &object.input_refs_json,
                            &object.source_coords_json,
                        ]
                        .into_iter()
                        .chain(&object.requests)
                        .map(Built::content_ref)
                    })
                    .collect();
                let reachable = reachable_nodes(&mut warm.store.nodes, &refs);
                // 唯一许可的残留：只在 `array_from_node`（共享 coords AVL 的数组 memo）里落的节点——
                // ① memo 子树条目（数组帧 + 序列根，留作下代命中复用）；② 其 `sequence_concat`
                // 内部 split 片段。二者都不是 append 作废根，逐代读数打印入报告。
                let memo_refs = warm.producer.memo_array_refs();
                let memo = reachable_nodes(&mut warm.store.nodes, &memo_refs);
                let emissions = warm.store.emissions.take().expect("本代已开账");
                let memo_only = |digest: &NodeDigest| {
                    emissions.materials.iter().any(|((stored, _), entries)| {
                        stored == digest
                            && entries.iter().all(|entry| {
                                entry.phases.keys().all(|phase| *phase == "coords_array_memo")
                            })
                    })
                };
                let unreachable: Vec<NodeDigest> = written
                    .iter()
                    .filter(|digest| !reachable.contains(*digest))
                    .copied()
                    .collect();
                let held = unreachable.iter().filter(|digest| memo.contains(*digest)).count();
                let orphans: Vec<NodeDigest> = unreachable
                    .iter()
                    .filter(|digest| !memo_only(digest))
                    .copied()
                    .collect();
                println!(
                    "S3_FRACTAL_ORPHANS reuse={reuse} step={step} written={} unreachable={} \
                     memo_held={held} memo_concat_internal={} other={}",
                    written.len(),
                    unreachable.len(),
                    unreachable.len() - held - orphans.len(),
                    orphans.len()
                );
                let mut by_tag: BTreeMap<u8, usize> = BTreeMap::new();
                let mut samples: Vec<String> = Vec::new();
                for digest in &orphans {
                    let (tag, body) = warm.store.nodes.get(digest).expect("新写节点在表");
                    *by_tag.entry(tag).or_default() += 1;
                    if samples.len() < 6 {
                        samples.push(format!(
                            "{tag:#04x}:{}",
                            String::from_utf8_lossy(&body[..body.len().min(48)])
                        ));
                    }
                }
                assert!(
                    orphans.is_empty(),
                    "reuse={reuse} 第 {step} 代 fractal 构造留下 {} 个不可达节点（新写 {}）：按 tag {by_tag:?}，样本 {samples:?}",
                    orphans.len(),
                    written.len()
                );
                covered += objects.len();
                last = objects;
            }
            assert_eq!(warm.rows.len(), all.len());
            assert!(covered > 0 && !last.is_empty(), "逐代不可达锁不是假空");
            assert!(
                warm.producer.history_reused > 0,
                "reuse={reuse}：必须真走到历史续接（否则锁测不到接尾）"
            );
            let mut cold = fixture_with_profile(&all, tb02a::PROFILE);
            let (expected, _) = fractal_generation_with_new_nodes(&mut cold, reuse, generation);
            assert_parser_fractals_equal(&mut cold, &mut warm, &expected, &last);
        }
    }

    /// #1394 固定真实 24 关系树：保留四轴、六参考与全部实际价格，仅减少重复材料写入。
    #[cfg(feature = "s_capture_native_testonly")]
    #[test]
    fn native_local_fractal_relations_preserve_full_material_and_render_with_fewer_puts() {
        let mut tail = bar(123, 35, -5);
        tail.open = 7;
        tail.close = 16;
        let references = vec![
            ("left.high".to_string(), 20),
            ("left.low".to_string(), 5),
            ("mid.high".to_string(), 24),
            ("mid.low".to_string(), -10),
            ("right.high".to_string(), 30),
            ("right.low".to_string(), 0),
        ];
        let value = CValue::Array(fractal_relations(&tail, &references));
        let mut old = CountingStore::default();
        let mut local = CountingStore::default();
        let expected = sc::write_value(&mut old, &value).unwrap();
        let actual = sc::write_value_native_local_for_testonly(&mut local, &value).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(old.puts, 1128);
        assert_eq!(local.puts, local.nodes.len());
        assert!(local.puts <= 268, "实际唯一材料数 {}", local.puts);
        assert!(local.puts * 4 < old.puts);
        assert_eq!(
            old.nodes.iter().collect::<Vec<_>>(),
            local.nodes.iter().collect::<Vec<_>>()
        );
        assert_eq!(render(&mut old, &expected), render(&mut local, &actual));
        verify(&mut local, &actual.content_ref()).unwrap();
        let per_call = local.puts;
        assert_eq!(
            sc::write_value_native_local_for_testonly(&mut local, &value).unwrap(),
            actual
        );
        assert_eq!(local.puts, 2 * per_call, "第二次调用重新写入全部唯一材料");
    }

    /// Rust 真构造计数：多个实际 TOP/BOTTOM 窗、稀疏坐标、包含合并/封口，逐根逐字节对拍。
    #[test]
    fn parser_fractal_sources_reuse_full_raw_once_with_exact_parity() {
        let merged = [
            (0, 8, 3),
            (10, 10, 5),
            (11, 9, 6),
            (20, 20, 15),
            (21, 19, 16),
            (30, 14, 7),
            (40, 25, 18),
            (50, 19, 10),
            (60, 30, 22),
            (70, 24, 14),
        ];
        let scaled: Vec<Vec<(usize, i64, i64)>> = [32usize, 64]
            .into_iter()
            .map(|n| {
                (0..n)
                    .map(|i| {
                        (
                            10 + i * 17,
                            if i % 2 == 0 { 12 } else { 24 },
                            if i % 2 == 0 { 3 } else { 15 },
                        )
                    })
                    .collect()
            })
            .collect();
        for rows in [CASE_SHAPE, &merged[..], &scaled[0][..], &scaled[1][..]] {
            let mut old = fixture_with_profile(rows, tb02a::PROFILE);
            let mut shared = fixture_with_profile(rows, tb02a::PROFILE);
            let (expected, old_builds) = collect_parser_fractals(&mut old, false, GENERATION);
            let (actual, shared_builds) = collect_parser_fractals(&mut shared, true, GENERATION);
            assert!(expected.len() >= 3);
            assert_eq!(old_builds, expected.len());
            assert_eq!(shared_builds, 1);
            // 在 render/verify 前冻结读写计数，计的是实际 codec/NodeSink 构造工作。
            let old_counts = (
                old.store.reads,
                old.store.puts,
                old.store.array_puts,
                old.store.branch_puts,
            );
            let shared_counts = (
                shared.store.reads,
                shared.store.puts,
                shared.store.array_puts,
                shared.store.branch_puts,
            );
            eprintln!("CC007 raw={} windows={} whole_builds={old_builds}/{shared_builds} old(reads,puts,arrays,branches)={old_counts:?} shared={shared_counts:?}", rows.len(), expected.len());
            assert!(shared_counts.0 < old_counts.0);
            assert!(shared_counts.1 < old_counts.1);
            assert!(shared_counts.2 < old_counts.2);
            assert!(shared_counts.3 < old_counts.3);
            assert_parser_fractals_equal(&mut old, &mut shared, &expected, &actual);
            let coords = render(&mut shared.store, &actual[0].source_coords);
            assert_eq!(
                serde_json::from_str::<Value>(&coords).unwrap(),
                json!(rows.iter().map(|row| row.0.to_string()).collect::<Vec<_>>())
            );
        }
    }

    /// 同价高 revision 不改变比较价格，但新 session 必须生成新 source_refs；追加继续对拍。
    #[test]
    fn parser_fractal_sources_revision_and_append_match_general_path() {
        let mut old = fixture_with_profile(CASE_SHAPE, tb02a::PROFILE);
        let mut shared = fixture_with_profile(CASE_SHAPE, tb02a::PROFILE);
        for generation in 3..6 {
            if generation == 4 {
                let row = Row::new(40, 99, "r99").with_prices(18, 25, 18, 25);
                for fixture in [&mut old, &mut shared] {
                    fixture
                        .producer
                        .bind_source(&mut fixture.store, row.coord, row.spec())
                        .unwrap();
                }
            } else if generation == 5 {
                let row = Row::new(9001, 1, "r1").with_prices(25, 35, 25, 35);
                for fixture in [&mut old, &mut shared] {
                    fixture
                        .producer
                        .bind_source(&mut fixture.store, row.coord, row.spec())
                        .unwrap();
                    fixture.state.append(bar(9001, 35, 25));
                }
            }
            let (expected, _) = collect_parser_fractals(&mut old, false, generation);
            let (actual, builds) = collect_parser_fractals(&mut shared, true, generation);
            assert_eq!(builds, 1, "不跨请求保留共享根");
            assert_parser_fractals_equal(&mut old, &mut shared, &expected, &actual);
        }
    }

    #[test]
    fn parser_fractal_sources_reject_foreign_stale_and_manual_shapes() {
        let mut fixture = fixture_with_profile(CASE_SHAPE, tb02a::PROFILE);
        let raw: Vec<Bar> = fixture.state.raw_iter().copied().collect();
        let first = fixture.producer.prepare_raw_history(&raw).unwrap();
        let mut session = fixture
            .producer
            .parser_fractal_sources(&first, &fixture.state)
            .unwrap();
        assert!(session.arrays.is_none(), "prepare 不提前生成数组");
        let shape = fixture
            .producer
            .build_parser_shape(&mut fixture.store, &first, &session, 0, GENERATION, CUT)
            .unwrap()
            .unwrap();
        let other_session = fixture
            .producer
            .parser_fractal_sources(&first, &fixture.state)
            .unwrap();
        let other_shape = fixture
            .producer
            .build_parser_shape(
                &mut fixture.store,
                &first,
                &other_session,
                0,
                GENERATION,
                CUT,
            )
            .unwrap()
            .unwrap();
        assert!(fixture
            .producer
            .build_parser_fractal(
                &mut fixture.store,
                &first,
                &mut session,
                &other_shape,
                GENERATION,
                CUT,
            )
            .is_err());
        let mut manual = shape.object().clone();
        manual.sources = coords_ref_from_slice(&[10]);
        let views = [0, 1, 2].map(|index| fixture.state.shared_group(index).unwrap().unwrap());
        let manual_result = fixture
            .producer
            .build_fractal(
                &mut fixture.store,
                &first,
                &manual,
                [&views[0], &views[1], &views[2]],
                GENERATION,
                CUT,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            render(&mut fixture.store, &manual_result.source_coords),
            "[\"10\",\"40\",\"50\",\"60\",\"70\"]"
        );
        assert!(session.arrays.is_none(), "人工对象只能走通用口");
        let current = fixture.producer.prepare_raw_history(&raw).unwrap();
        assert!(fixture
            .producer
            .build_parser_fractal(
                &mut fixture.store,
                &first,
                &mut session,
                &shape,
                GENERATION,
                CUT,
            )
            .is_err());
        assert!(fixture
            .producer
            .build_parser_fractal(
                &mut fixture.store,
                &current,
                &mut session,
                &shape,
                GENERATION,
                CUT,
            )
            .is_err());
        let foreign = fixture_with_profile(CASE_SHAPE, tb02a::PROFILE);
        assert!(
            fixture
                .producer
                .parser_fractal_sources(&current, &foreign.state)
                .is_ok(),
            "逐元素相同 raw 的另一次 parser 也须自行创建新 session"
        );
        let different = fixture_with_profile(CASE_MERGE, tb02a::PROFILE);
        assert!(fixture
            .producer
            .parser_fractal_sources(&current, &different.state)
            .is_err());
        let foreign_producer = Cc004Producer::new(tb02a::PROFILE, RULE);
        assert!(foreign_producer
            .parser_fractal_sources(&current, &fixture.state)
            .is_err());
        let mut fresh_session = fixture
            .producer
            .parser_fractal_sources(&current, &fixture.state)
            .unwrap();
        fixture.producer.discard_runtime_state();
        assert!(fixture
            .producer
            .build_parser_fractal(
                &mut fixture.store,
                &current,
                &mut fresh_session,
                &shape,
                GENERATION,
                CUT,
            )
            .is_err());
    }

    /// CC-007 比较单元 memo：同参考集重复构造全命中（零重算），源修订按坐标失效并重算；
    /// OHLC 未变时载荷同根、源引用与 obj3 必变。
    #[test]
    fn cc007_unit_memo_reuses_and_invalidates_by_source_revision() {
        let mut fixture = fixture_with_profile(CASE_SHAPE, tb02a::PROFILE);
        let facts = fixture.state.materialize_facts();
        let start = 0usize;
        let raw_bars: Vec<Bar> = fixture.state.raw_iter().copied().collect();
        let build = |fixture: &mut Fixture| -> Cc007Object {
            let shape = fixture
                .producer
                .build_shape(&mut fixture.store, &fixture.state, start, GENERATION, CUT)
                .expect("构造 CC-006")
                .expect("该窗口应可判");
            let views = [0usize, 1, 2].map(|index| {
                fixture
                    .state
                    .shared_group(index)
                    .expect("共享态可用")
                    .expect("组在域内")
            });
            fixture
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
                .expect("TOP/BOTTOM 窗口应产对象")
        };
        let before = build(&mut fixture);
        let counts_before = fixture.producer.memo_counts();
        assert!(counts_before.unit_computed > 0, "首轮必须真的求过比较单元");
        let again = build(&mut fixture);
        let counts_after = fixture.producer.memo_counts();
        assert_eq!(before.object_id, again.object_id, "重复构造身份必须相同");
        assert_eq!(before.payload.digest, again.payload.digest);
        assert_eq!(
            counts_after.unit_computed, counts_before.unit_computed,
            "同参考集重复构造不得重算比较单元"
        );
        assert!(
            counts_after.unit_reused > counts_before.unit_reused,
            "重复构造必须命中单元 memo"
        );

        // 真实修订：同坐标换 revision（OHLC 不变）⟹ 该坐标起的单元失效并重算。
        let row = Row::new(50, 2, "r2").with_prices(10, 19, 10, 19);
        assert!(fixture
            .producer
            .bind_source(&mut fixture.store, row.coord, row.spec())
            .expect("真实修订"));
        fixture.raw.insert(row.coord, row.raw_json());
        let revised = build(&mut fixture);
        let counts_revised = fixture.producer.memo_counts();
        assert!(
            counts_revised.unit_invalidated > counts_after.unit_invalidated,
            "源修订必须按坐标失效单元 memo"
        );
        assert!(
            counts_revised.unit_computed > counts_after.unit_computed,
            "失效的单元必须重算"
        );
        assert_eq!(
            before.payload.digest, revised.payload.digest,
            "OHLC 未变：载荷同根"
        );
        assert_ne!(
            before.input_refs.digest, revised.input_refs.digest,
            "源引用变化必须改根"
        );
        assert_ne!(before.object_id, revised.object_id, "源引用变化必须改 obj3");
        // 非 TOP/BOTTOM 分支不产 CC-007（旧路径对这些分支同样不建对象）。
        let last = facts.groups.len() - 3;
        let mut rising = fixture
            .producer
            .build_shape(&mut fixture.store, &fixture.state, last, GENERATION, CUT)
            .expect("构造 CC-006")
            .expect("末窗口应可判");
        rising.branch = "RISING";
        let views = [last, last + 1, last + 2].map(|index| {
            fixture
                .state
                .shared_group(index)
                .expect("共享态可用")
                .expect("组在域内")
        });
        assert!(fixture
            .producer
            .build_fractal_raw(
                &mut fixture.store,
                &raw_bars,
                &rising,
                [&views[0], &views[1], &views[2]],
                GENERATION,
                CUT,
            )
            .expect("分支判定不是错误")
            .is_none());
    }

    /// 交错窗口的保留与失效：两个窗口在同一代内反复交错构造必须零重算（单槽 memo 会被对方
    /// 驱逐、逐代全量重建），源修订后**所有受影响窗口**都按实际坐标截断重算，重算后再次零重算。
    #[test]
    fn fractal_history_keeps_interleaved_windows_and_invalidates_every_window() {
        let mut fixture = fixture_with_profile(CASE_SHAPE, tb02a::PROFILE);
        let raw_bars: Vec<Bar> = fixture.state.raw_iter().copied().collect();
        let build = |fixture: &mut Fixture,
                     prepared: &PreparedRawHistory<'_>,
                     start: usize|
         -> Cc007Object {
            let shape = fixture
                .producer
                .build_shape(&mut fixture.store, &fixture.state, start, GENERATION, CUT)
                .expect("构造 CC-006")
                .expect("该窗口应可判");
            let views = [start, start + 1, start + 2].map(|index| {
                fixture
                    .state
                    .shared_group(index)
                    .expect("共享态可用")
                    .expect("组在域内")
            });
            fixture
                .producer
                .build_fractal(
                    &mut fixture.store,
                    prepared,
                    &shape,
                    [&views[0], &views[1], &views[2]],
                    GENERATION,
                    CUT,
                )
                .expect("构造 CC-007")
                .expect("TOP/BOTTOM 窗口应产对象")
        };
        // 一代一次 prepare，两个窗口共用同一视图（正式生成入口的用法）。
        let prepared = fixture
            .producer
            .prepare_raw_history(&raw_bars)
            .expect("准备 raw 账本");
        let a1 = build(&mut fixture, &prepared, 0);
        let b1 = build(&mut fixture, &prepared, 2);
        let counts_first = fixture.producer.memo_counts();
        let a2 = build(&mut fixture, &prepared, 0);
        let b2 = build(&mut fixture, &prepared, 2);
        let counts_second = fixture.producer.memo_counts();
        assert_eq!(a1.object_id, a2.object_id, "窗口0 身份必须稳定");
        assert_eq!(a1.record.digest, a2.record.digest, "窗口0 记录根必须稳定");
        assert_eq!(b1.object_id, b2.object_id, "窗口2 身份必须稳定");
        assert_eq!(b1.record.digest, b2.record.digest, "窗口2 记录根必须稳定");
        assert_eq!(
            counts_second.unit_computed, counts_first.unit_computed,
            "交错窗口第二次构造必须零重算（单槽 memo 会被对方驱逐）"
        );
        assert!(
            counts_second.unit_reused > counts_first.unit_reused,
            "两窗必须各自命中自己的 memo"
        );

        // 源修订坐标 60 同时落在窗口0 与窗口2 的覆盖区内：两窗都必须按实际坐标截断重算。
        let row = Row::new(60, 2, "r2").with_prices(22, 30, 22, 30);
        assert!(fixture
            .producer
            .bind_source(&mut fixture.store, row.coord, row.spec())
            .expect("真实修订"));
        let prepared = fixture
            .producer
            .prepare_raw_history(&raw_bars)
            .expect("修订后重新准备");
        let _ = build(&mut fixture, &prepared, 0);
        let after_a = fixture.producer.memo_counts();
        assert!(
            after_a.unit_computed > counts_second.unit_computed,
            "窗口0 必须重算被修订的 suffix"
        );
        let _ = build(&mut fixture, &prepared, 2);
        let after_b = fixture.producer.memo_counts();
        assert!(
            after_b.unit_computed > after_a.unit_computed,
            "窗口2 同样必须重算"
        );
        assert!(
            after_b.unit_invalidated > counts_second.unit_invalidated,
            "两窗都要按实际坐标记失效"
        );
        let _ = build(&mut fixture, &prepared, 0);
        let _ = build(&mut fixture, &prepared, 2);
        let settled = fixture.producer.memo_counts();
        assert_eq!(
            settled.unit_computed, after_b.unit_computed,
            "重算后必须回到零重算"
        );
        assert!(settled.unit_reused > after_b.unit_reused);
    }

    #[test]
    fn replacing_prepared_raw_content_invalidates_the_previous_view() {
        let mut fixture = fixture_with_profile(CASE_SHAPE, tb02a::PROFILE);
        let raw: Vec<Bar> = fixture.state.raw_iter().copied().collect();
        let shape = fixture
            .producer
            .build_shape(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .unwrap()
            .unwrap();
        let views = [0usize, 1, 2].map(|index| fixture.state.shared_group(index).unwrap().unwrap());
        let previous = fixture.producer.prepare_raw_history(&raw).unwrap();
        let mut changed = raw.clone();
        changed.last_mut().unwrap().high += 1;
        // source_ref 未换也可能收到不同 raw 值；新准备态必须封闭旧视图的消费口。
        let current = fixture.producer.prepare_raw_history(&changed).unwrap();
        let error = fixture
            .producer
            .build_fractal(
                &mut fixture.store,
                &previous,
                &shape,
                [&views[0], &views[1], &views[2]],
                GENERATION,
                CUT,
            )
            .expect_err("重新准备不同 raw 后，旧视图必须失效");
        assert!(matches!(
            error,
            ProducerError::Type {
                what: "raw 账本准备态",
                ..
            }
        ));
        fixture
            .producer
            .build_fractal(
                &mut fixture.store,
                &current,
                &shape,
                [&views[0], &views[1], &views[2]],
                GENERATION,
                CUT,
            )
            .unwrap()
            .unwrap();
    }

    /// 陈旧 raw 账本视图必须显式拒绝：source 修订推进、换 producer、rollback 后都不许继续消费
    /// （不靠调用方承诺，也不留绕过 `bind_source` 的旁路）。
    #[test]
    fn stale_prepared_raw_history_is_rejected() {
        let mut fixture = fixture_with_profile(CASE_SHAPE, tb02a::PROFILE);
        let raw_bars: Vec<Bar> = fixture.state.raw_iter().copied().collect();
        let shape = fixture
            .producer
            .build_shape(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .expect("构造 CC-006")
            .expect("该窗口应可判");
        let views = [0usize, 1, 2].map(|index| {
            fixture
                .state
                .shared_group(index)
                .expect("共享态可用")
                .expect("组在域内")
        });
        let prepared = fixture
            .producer
            .prepare_raw_history(&raw_bars)
            .expect("准备 raw 账本");
        fixture
            .producer
            .build_fractal(
                &mut fixture.store,
                &prepared,
                &shape,
                [&views[0], &views[1], &views[2]],
                GENERATION,
                CUT,
            )
            .expect("见证期内构造")
            .expect("TOP 窗口应产对象");
        // ① 源修订推进 epoch：旧视图必须拒绝，而不是静默复用旧 keys。
        let row = Row::new(40, 2, "r2").with_prices(18, 25, 18, 25);
        assert!(fixture
            .producer
            .bind_source(&mut fixture.store, row.coord, row.spec())
            .expect("真实修订"));
        let error = fixture
            .producer
            .build_fractal(
                &mut fixture.store,
                &prepared,
                &shape,
                [&views[0], &views[1], &views[2]],
                GENERATION,
                CUT,
            )
            .expect_err("修订后的陈旧视图必须拒绝");
        assert!(
            matches!(
                error,
                ProducerError::Type {
                    what: "raw 账本准备态",
                    ..
                }
            ),
            "{error:?}"
        );
        // ② 换 producer 的视图不认（owner 防伪）。
        let mut other_fixture = fixture_with_profile(CASE_SHAPE, tb02a::PROFILE);
        let other_raw: Vec<Bar> = other_fixture.state.raw_iter().copied().collect();
        let foreign = other_fixture
            .producer
            .prepare_raw_history(&other_raw)
            .expect("另一方准备 raw 账本");
        let error = fixture
            .producer
            .build_fractal(
                &mut fixture.store,
                &foreign,
                &shape,
                [&views[0], &views[1], &views[2]],
                GENERATION,
                CUT,
            )
            .expect_err("外 producer 的视图必须拒绝");
        assert!(
            matches!(
                error,
                ProducerError::Type {
                    what: "raw 账本准备态",
                    ..
                }
            ),
            "{error:?}"
        );
        // ③ rollback 口（discard_runtime_state）同样使视图陈旧。
        fixture.producer.discard_runtime_state();
        let error = fixture
            .producer
            .build_fractal(
                &mut fixture.store,
                &foreign,
                &shape,
                [&views[0], &views[1], &views[2]],
                GENERATION,
                CUT,
            )
            .expect_err("rollback 后视图仍必须拒绝");
        assert!(
            matches!(
                error,
                ProducerError::Type {
                    what: "raw 账本准备态",
                    ..
                }
            ),
            "{error:?}"
        );
    }

    /// CC-005 拒绝面：源坐标未登记当前版本、共享态退化都显式 `Err`，不落空数组。
    #[test]
    fn cc005_missing_source_and_degraded_are_rejected() {
        let mut fixture = unbound_fixture(CASE_MERGE);
        let error = fixture
            .producer
            .build_group(&mut fixture.store, &fixture.state, 0, GENERATION, CUT)
            .expect_err("未登记源版本必须拒绝");
        assert!(
            matches!(error, ProducerError::MissingSource(_)),
            "{error:?}"
        );

        let mut degraded = IncrFactsState::empty();
        degraded.append(bar(5, 10, 5));
        degraded.append(bar(5, 12, 6));
        assert!(degraded.degraded());
        let error = fixture
            .producer
            .build_group(&mut fixture.store, &degraded, 0, GENERATION, CUT)
            .expect_err("共享态退化必须拒绝");
        assert!(matches!(error, ProducerError::Degraded(_)), "{error:?}");
    }

    /// 覆盖口径：等待 / 合并 / 方向变化 / 同价极值 / 稀疏坐标确实在本用例的输入里出现。
    #[test]
    fn coverage_waiting_merge_direction_and_equal_extremes() {
        let mut merge_fixture = fixture(CASE_MERGE);
        let merge_objects = build_all(&mut merge_fixture);
        let merge_facts = merge_fixture.state.materialize_facts();
        assert!(merge_facts.steps.iter().any(|step| step.action == "merge"));
        assert!(merge_facts
            .steps
            .iter()
            .any(|step| step.action == "new_group"));
        assert!(merge_facts
            .steps
            .iter()
            .any(|step| step.direction_evidence.is_some()));
        assert!(merge_facts
            .steps
            .iter()
            .any(|step| step.high_sources.len() > 1 || step.low_sources.len() > 1));
        assert_eq!(
            merge_facts.steps[0].source_coords,
            vec![10],
            "稀疏坐标不得被区间化"
        );
        assert_eq!(merge_objects.len(), merge_facts.steps.len());
        assert!(merge_objects
            .iter()
            .all(|object| object.payload.extent == Some(12)));

        let mut waiting_fixture = fixture(CASE_WAITING);
        let waiting_objects = build_all(&mut waiting_fixture);
        let waiting_facts = waiting_fixture.state.materialize_facts();
        assert!(waiting_facts.initial_direction_unsettled);
        assert_eq!(waiting_facts.steps[0].action, "seed");
        assert!(waiting_facts.steps[1..]
            .iter()
            .all(|step| step.action == "waiting"
                && step.direction_evidence.is_none()
                && step.waiting_reasons == vec!["initial_direction_unsettled"]));
        assert_eq!(waiting_facts.steps[1].source_coords, vec![0, 1]);
        // 未建方向的 waiting 步绑定的是**全部已见 raw 前缀**（与全量入口同形）。
        assert_eq!(waiting_facts.steps[2].source_coords, vec![0, 1, 2]);
        assert_eq!(waiting_objects.len(), waiting_facts.steps.len());
    }

    /// 派生链逐前缀对拍 ①：任一前缀状态下，末步（刚追加的那一步）的 `coords` 就是该前缀的
    /// 完整 raw 坐标链，`raw_prefix_coords` 对同一前缀返回同一集合——共享链不是近似前缀，
    /// 也不是「最新一段」。
    #[test]
    fn raw_prefix_chain_is_last_step_coords_for_every_prefix() {
        for (case, bars) in [
            ("merge/new-group/direction", CASE_MERGE),
            ("waiting", CASE_WAITING),
            ("shape/top-bottom", CASE_SHAPE),
        ] {
            for prefix in 1..=bars.len() {
                let mut state = IncrFactsState::empty();
                for (coord, high, low) in &bars[..prefix] {
                    state.append(bar(*coord, *high, *low));
                }
                let expect: Vec<usize> =
                    bars[..prefix].iter().map(|(coord, _, _)| *coord).collect();
                let chain = raw_prefix_coords(&state).expect("raw 前缀链可用");
                assert_eq!(chain.materialize(), expect, "{case} 前缀{prefix}: 前缀链");
                let last = state.step_count() - 1;
                let view = state
                    .shared_step(last)
                    .expect("共享态可用")
                    .expect("末步在域内");
                assert_eq!(
                    view.coords.materialize(),
                    expect,
                    "{case} 前缀{prefix}: 末步 coords"
                );
            }
        }
    }

    /// 派生链逐前缀对拍 ②：`group_sources_coords`（共享拼接）与「members ∪ 方向证据 ∪
    /// 封口证据」的排序去重集逐前缀逐组同值；`group_members_coords` 与 `members` 同值；
    /// 三个连续组的来源并集 == 右组来源（CC-006 取右组 sources 的前提，含竞争窗）。
    #[test]
    fn derived_coords_match_materialized_union_for_every_prefix() {
        for (case, bars) in [
            ("merge/new-group/direction", CASE_MERGE),
            ("shape/top-bottom", CASE_SHAPE),
        ] {
            for prefix in 1..=bars.len() {
                let mut state = IncrFactsState::empty();
                for (coord, high, low) in &bars[..prefix] {
                    state.append(bar(*coord, *high, *low));
                }
                for index in 0..state.group_count() {
                    let view = state
                        .shared_group(index)
                        .expect("共享态可用")
                        .expect("组在域内");
                    let mut union: Vec<usize> = view.members.to_vec();
                    if let Some(node) = view.direction_evidence {
                        union.extend(node.coords.materialize());
                    }
                    if let Some(node) = view.confirmation_evidence {
                        union.extend(node.coords.materialize());
                    }
                    union.sort_unstable();
                    union.dedup();
                    assert_eq!(
                        group_members_coords(&view).materialize(),
                        view.members.to_vec(),
                        "{case} 前缀{prefix} 组{index}: members"
                    );
                    assert_eq!(
                        group_sources_coords(&view).materialize(),
                        union,
                        "{case} 前缀{prefix} 组{index}: sources 三集并集"
                    );
                }
                for index in 0..state.group_count().saturating_sub(2) {
                    let views = [index, index + 1, index + 2].map(|slot| {
                        state
                            .shared_group(slot)
                            .expect("共享态可用")
                            .expect("组在域内")
                    });
                    let mut window: Vec<usize> = views
                        .iter()
                        .flat_map(|view| group_sources_coords(view).materialize())
                        .collect();
                    window.sort_unstable();
                    window.dedup();
                    assert_eq!(
                        window,
                        group_sources_coords(&views[2]).materialize(),
                        "{case} 前缀{prefix} 窗{index}: 三连窗并集 == 右组 sources"
                    );
                    assert_eq!(
                        window,
                        group_window_sources(&views).materialize(),
                        "{case} 前缀{prefix} 窗{index}: 窗口来源口"
                    );
                }
            }
        }
    }

    /// 独立复算 obj3 配方：不借构造器的 `F`/十六进制，按 §5.1 的 `F` 定义直接算 SHA-256；
    /// 并证明它与旧「canonical JSON 再 SHA-256」不是同一个配方。
    #[test]
    fn obj3_recipe_is_verified_independently() {
        let mut fixture = fixture(CASE_MERGE);
        let objects = build_all(&mut fixture);
        let object = objects.last().expect("末步对象");

        let mut identity_store = MemoryNodes::new();
        let identity = CValue::Object(vec![
            (
                "kind".to_string(),
                CValue::String(CC004_STEP_KIND.to_string()),
            ),
            (
                "fact_key".to_string(),
                CValue::String(object.fact_key.clone()),
            ),
            ("payload".to_string(), CValue::Edge(object.payload.digest)),
            (
                "input_refs".to_string(),
                CValue::Edge(object.input_refs.digest),
            ),
            (
                "profile_id".to_string(),
                CValue::String(PROFILE.to_string()),
            ),
            (
                "rule_revision".to_string(),
                CValue::String(RULE.to_string()),
            ),
        ]);
        let built = sc::write_value(&mut identity_store, &identity).expect("六字段身份对象");
        assert_eq!(built.kind, Kind::Object);
        assert_eq!(built.extent, Some(6), "obj3 配方必须是严格六字段");
        let v = built.digest;

        let mut hasher = Sha256::new();
        hasher.update(sc::DOMAIN_SEPARATOR);
        hasher.update([OBJECT_ID_TAG]);
        hasher.update(32u64.to_be_bytes());
        hasher.update(v);
        let expected = format!("obj3-{}", lower_hex(&hasher.finalize()));
        assert_eq!(object.object_id, expected, "obj3 = obj3- + hex(F(0x20, V))");
        assert_eq!(object.identity, v, "构造器暴露的 V 必须就是配方输入");

        let payload: Value =
            serde_json::from_str(&render(&mut fixture.store, &object.payload)).expect("载荷");
        let input_refs: Value =
            serde_json::from_str(&render(&mut fixture.store, &object.input_refs)).expect("源引用");
        let legacy = sha256_hex(
            canonical_json(&json!({
                "kind": CC004_STEP_KIND,
                "fact_key": object.fact_key,
                "payload": payload,
                "input_refs": input_refs,
                "profile_id": PROFILE,
                "rule_revision": RULE,
            }))
            .as_bytes(),
        );
        assert_ne!(
            object.object_id.trim_start_matches("obj3-"),
            legacy,
            "obj3 不是旧配方的换前缀"
        );
    }

    /// 追加（新的最大坐标）保留旧 prefix 的逐字节内容，并跳过 refs memo 的整表遍历。
    #[test]
    fn append_keeps_old_objects_and_skips_refs_cache_scan() {
        let mut fixture = fixture(CASE_MERGE);
        let before = build_all(&mut fixture);
        let counts_before = fixture.producer.memo_counts();

        let row = Row::new(50000, 1, "r1");
        assert!(fixture
            .producer
            .bind_source(&mut fixture.store, row.coord, row.spec())
            .expect("登记新坐标"));
        fixture.raw.insert(row.coord, row.raw_json());
        fixture.state.append(bar(50000, 16, 7));
        let after = build_all(&mut fixture);

        assert!(after.len() > before.len());
        for (index, (old, fresh)) in before.iter().zip(&after).enumerate() {
            assert_eq!(
                old.object_id, fresh.object_id,
                "步{index} obj3 被后续追加改写"
            );
            assert_eq!(
                old.record.digest, fresh.record.digest,
                "步{index} 记录根被改写"
            );
            assert_eq!(
                old.payload.digest, fresh.payload.digest,
                "步{index} 载荷根被改写"
            );
            assert_eq!(
                old.input_refs.digest, fresh.input_refs.digest,
                "步{index} 源引用根被改写"
            );
            assert_eq!(old.source_coords.digest, fresh.source_coords.digest);
        }
        let counts_after = fixture.producer.memo_counts();
        assert_eq!(
            counts_after.invalidated, counts_before.invalidated,
            "普通追加不得使旧 prefix 失效"
        );
        assert_eq!(
            counts_after.skipped_scans,
            counts_before.skipped_scans + 1,
            "新坐标大于 memo 上界时应跳过整表遍历"
        );
    }

    /// 同坐标、同 OHLC，只换 revision/receipt：载荷与坐标集合不动，源引用与 obj3 必变。
    #[test]
    fn same_ohlc_new_revision_moves_refs_and_obj3() {
        let mut fixture = fixture(CASE_MERGE);
        let before = build_all(&mut fixture);
        let old = before.last().expect("末步").clone();

        let row = Row::new(12345, 2, "r2");
        assert!(fixture
            .producer
            .bind_source(&mut fixture.store, row.coord, row.spec())
            .expect("真实修订"));
        let after = build_all(&mut fixture);
        let fresh = after.last().expect("末步");

        assert_eq!(
            old.payload.digest, fresh.payload.digest,
            "OHLC 未变：载荷同根"
        );
        assert_eq!(
            old.source_coords.digest, fresh.source_coords.digest,
            "坐标集合未变：源坐标数组同根"
        );
        assert_ne!(
            old.input_refs.digest, fresh.input_refs.digest,
            "源引用变化必须改根"
        );
        assert_ne!(old.object_id, fresh.object_id, "源引用变化必须改 obj3");
        assert_ne!(old.record.digest, fresh.record.digest);
        let counts = fixture.producer.memo_counts();
        assert!(counts.invalidated > 0, "源修订必须按坐标失效 refs memo");
    }

    /// 清空 memo 后重建：全部根与 obj3 逐字节相同（缓存不参与身份）。
    #[test]
    fn clearing_memos_keeps_roots_identical() {
        let mut fixture = fixture(CASE_MERGE);
        let before = build_all(&mut fixture);
        fixture.producer.clear_memos();
        let after = build_all(&mut fixture);
        assert_eq!(before.len(), after.len());
        for (index, (old, fresh)) in before.iter().zip(&after).enumerate() {
            assert_eq!(old.object_id, fresh.object_id, "步{index} obj3");
            assert_eq!(old.identity, fresh.identity);
            assert_eq!(old.record.digest, fresh.record.digest, "步{index} 记录根");
            assert_eq!(old.payload.digest, fresh.payload.digest);
            assert_eq!(old.input_refs.digest, fresh.input_refs.digest);
            assert_eq!(old.source_coords.digest, fresh.source_coords.digest);
            assert_eq!(old.payload_json.digest, fresh.payload_json.digest);
        }
        assert!(
            fixture.producer.memo_counts().text_misses > 0,
            "清空后应重新填充 memo"
        );
    }

    /// 同一逻辑叶序的不同物理树形（AVL 共享拼接 vs 手工右倾链）必须同根；
    /// 并直接检查数组节点的规范 `M` 分解（4 叶 → k = 2，左右各 2 叶分支）。
    #[test]
    fn array_root_ignores_private_tree_shape() {
        let mut store = CountingStore::default();
        let mut producer = Cc004Producer::new(PROFILE, RULE);
        let mut shared = CoordsRef::empty();
        for coord in [2usize, 5, 9, 14] {
            shared = shared.append_greater(coord);
        }
        let from_shared = producer
            .coords_array(&mut store, &shared)
            .expect("共享拼接树");
        let skewed = Rc::new(CoordNode::Concat(
            Rc::new(CoordNode::Concat(
                Rc::new(CoordNode::Concat(
                    Rc::new(CoordNode::Leaf(2)),
                    Rc::new(CoordNode::Leaf(5)),
                    1,
                )),
                Rc::new(CoordNode::Leaf(9)),
                1,
            )),
            Rc::new(CoordNode::Leaf(14)),
            1,
        ));
        let from_skewed = producer
            .coords_array_of_node(&mut store, &skewed)
            .expect("手工形状");
        assert_eq!(
            from_shared.digest, from_skewed.digest,
            "物理树形泄漏进内容身份"
        );
        assert_eq!(from_shared.extent, Some(4));
        assert_eq!(
            render(&mut store, &from_shared),
            "[\"2\",\"5\",\"9\",\"14\"]"
        );

        let (tag, body) = store.nodes.get(&from_shared.digest).expect("数组节点");
        assert_eq!(tag, sc::tag::ARRAY);
        assert_eq!(&body[..8], &4u64.to_be_bytes());
        let mut seq_root = [0u8; 32];
        seq_root.copy_from_slice(&body[8..40]);
        let (tag, body) = store.nodes.get(&seq_root).expect("序列根");
        assert_eq!(tag, sc::tag::SEQ_BRANCH);
        assert_eq!(&body[..8], &4u64.to_be_bytes());
        for offset in [8usize, 40] {
            let mut child = [0u8; 32];
            child.copy_from_slice(&body[offset..offset + 32]);
            let (tag, child_body) = store.nodes.get(&child).expect("子分支");
            assert_eq!(tag, sc::tag::SEQ_BRANCH);
            assert_eq!(
                &child_body[..8],
                &2u64.to_be_bytes(),
                "k = 严格小于 4 的最大 2 的幂 = 2"
            );
        }
    }

    /// memo 命中零读写；普通追加只沿右脊构造新节点（不是 O(叶数) 重写）。
    #[test]
    fn memo_reuse_and_incremental_append_write_bounds() {
        let mut store = CountingStore::default();
        let mut producer = Cc004Producer::new(PROFILE, RULE);
        let mut coords = CoordsRef::empty();
        for step in 0..32usize {
            coords = coords.append_greater(step * 7);
        }
        let first = producer
            .coords_array(&mut store, &coords)
            .expect("首次构造");
        let writes_first = store.puts;
        let reads_first = store.reads;
        assert!(
            writes_first > 100,
            "首次构造应落 4n 级节点，实得 {writes_first}"
        );

        let again = producer
            .coords_array(&mut store, &coords)
            .expect("重复构造");
        assert_eq!(again.digest, first.digest);
        assert_eq!(again.extent, Some(32));
        assert_eq!(store.puts, writes_first, "memo 命中不应写节点");
        assert_eq!(store.reads, reads_first, "memo 命中不应读节点");

        let extended = coords.append_greater(9999);
        let built = producer
            .coords_array(&mut store, &extended)
            .expect("追加构造");
        let delta = store.puts - writes_first;
        assert!(
            delta * 4 <= writes_first,
            "增量写放大 {delta} 相对首次 {writes_first} 不成比例"
        );
        assert!(delta <= 32, "增量写放大 {delta} 超出 O(log n) 界");
        let want: Vec<String> = extended
            .materialize()
            .iter()
            .map(|coord| format!("\"{coord}\""))
            .collect();
        assert_eq!(render(&mut store, &built), format!("[{}]", want.join(",")));
    }

    /// 缺源 / 退化 / 类型不符一律显式 `Err`：不返回空数组、不回落旧 `Value`。
    #[test]
    fn missing_source_degraded_and_type_errors_are_explicit() {
        let mut store = CountingStore::default();
        let mut producer = Cc004Producer::new(PROFILE, RULE);
        let mut state = IncrFactsState::empty();
        state.append(bar(3, 10, 5));

        let error = producer
            .build_step(&mut store, &state, 0, GENERATION, CUT)
            .expect_err("未登记源版本必须拒绝");
        assert!(
            matches!(error, ProducerError::MissingSource(3)),
            "{error:?}"
        );
        assert!(producer
            .build_step(&mut store, &state, 99, GENERATION, CUT)
            .expect("越界索引不是错误")
            .is_none());
        let error = producer
            .source_refs_array(&mut store, &CoordsRef::single(3))
            .expect_err("源引用数组不缺源兜底");
        assert!(
            matches!(error, ProducerError::MissingSource(3)),
            "{error:?}"
        );

        let mut degraded = IncrFactsState::empty();
        degraded.append(bar(5, 10, 5));
        degraded.append(bar(5, 12, 6));
        assert!(degraded.degraded());
        let error = producer
            .build_step(&mut store, &degraded, 0, GENERATION, CUT)
            .expect_err("共享态退化必须拒绝");
        assert!(matches!(error, ProducerError::Degraded(_)), "{error:?}");

        let mut spec = Row::new(3, 1, "r1").spec();
        spec.source_coord = "4".to_string();
        let error = producer
            .bind_source(&mut store, 3, spec)
            .expect_err("坐标文本与注册坐标不一致必须拒绝");
        assert!(
            matches!(
                error,
                ProducerError::Type {
                    what: "source_coord",
                    ..
                }
            ),
            "{error:?}"
        );
    }

    /// 真实 SQLite 事务边界（`content_store::{init_schema,TxStore,ReadStore}`）：
    /// `shared_step` 生产 → rollback → **只清 memo 不够**（同 spec 重绑不写、闭包缺节点）→
    /// 丢弃整份运行态并按真实当前 source 重新登记 → 新事务生产并核验 → commit 后读事务可核。
    ///
    /// 本用例证明的是「同次 parser → 真实持久节点 → 读侧核验/渲染」这条链的**节点持久边界**，
    /// 不是正式 S3 入口：不发布 cut、不建独立索引、不发 wire。
    #[test]
    fn sqlite_transaction_rollback_requires_discarding_runtime_state() {
        let mut conn = Connection::open_in_memory().expect("内存库");
        conn.execute_batch(super::super::v2::SCHEMA_V2)
            .expect("v2 schema");
        conn.execute(
            "INSERT INTO s_protocol_meta VALUES(1, 's-session/3', '1', 'test-clock', '{}', 1, 'accept')",
            [],
        )
        .expect("s-session/3 域登记");
        let mut fixture = unbound_fixture(CASE_MERGE);

        // ① 首个事务：在本事务的 store 里登记真实当前 source，从共享态直接生产，
        //    事务内全闭包可核；随后 rollback（节点随之消失）。
        let first = {
            let tx = conn.transaction().expect("事务");
            init_schema(&tx).expect("建内容表");
            let mut store = TxStore::new(&tx).expect("写侧");
            for row in &fixture.rows {
                assert!(fixture
                    .producer
                    .bind_source(&mut store, row.coord, row.spec())
                    .expect("登记真实当前 source"));
            }
            let objects = build_objects(&mut store, &mut fixture.producer, &fixture.state);
            for object in &objects {
                verify(&mut store, &object.record.content_ref()).expect("事务内可核");
            }
            tx.rollback().expect("rollback");
            objects
        };

        // ② 只清 memo 不够：源版本注册表还留着上轮事务写下的根，`bind_source` 同 spec 直接返回
        //    （不重新写节点），重建出来的对象闭包缺节点——这正是必须有「整份运行态丢弃」的原因。
        {
            let tx = conn.transaction().expect("事务");
            init_schema(&tx).expect("建内容表");
            let mut store = TxStore::new(&tx).expect("写侧");
            fixture.producer.clear_memos();
            for row in &fixture.rows {
                assert!(
                    !fixture
                        .producer
                        .bind_source(&mut store, row.coord, row.spec())
                        .expect("同 spec 重绑"),
                    "同 spec 重绑不得写节点"
                );
            }
            let objects = build_objects(&mut store, &mut fixture.producer, &fixture.state);
            let error = verify(&mut store, &objects[0].record.content_ref())
                .expect_err("上轮事务的根已消失：闭包必然缺节点");
            assert_eq!(error.code(), sc::code::NODE_MISSING, "{error}");
            tx.rollback().expect("rollback");
        }

        // ③ 丢弃整份运行态（保留冻结 profile/rule），按真实当前 source 重新登记，
        //    再在新事务里生产、核验并 commit。
        fixture.producer.discard_runtime_state();
        let (committed, rendered) = {
            let tx = conn.transaction().expect("事务");
            init_schema(&tx).expect("建内容表");
            let mut store = TxStore::new(&tx).expect("写侧");
            for row in &fixture.rows {
                assert!(
                    fixture
                        .producer
                        .bind_source(&mut store, row.coord, row.spec())
                        .expect("重新登记真实当前 source"),
                    "丢弃运行态后必须真的重新写节点"
                );
            }
            let objects = build_objects(&mut store, &mut fixture.producer, &fixture.state);
            for object in &objects {
                verify(&mut store, &object.record.content_ref()).expect("重建后可核");
            }
            let rendered = render(&mut store, &objects.last().expect("末步").record);
            tx.commit().expect("commit");
            (objects, rendered)
        };

        // ④ commit 后同一读事务可核：内容身份与首轮逐字节相同（不靠缓存，只靠内容）。
        assert_eq!(committed.len(), first.len());
        for (index, (old, fresh)) in first.iter().zip(&committed).enumerate() {
            assert_eq!(old.record.digest, fresh.record.digest, "步{index} 记录根");
            assert_eq!(old.object_id, fresh.object_id, "步{index} obj3");
        }
        let tx = conn.transaction().expect("读事务");
        let mut reader = ReadStore::new(&tx).expect("读侧");
        let last = committed.last().expect("末步");
        verify(&mut reader, &last.record.content_ref()).expect("commit 后读事务可核");
        assert_eq!(render(&mut reader, &last.record), rendered);
    }
}
