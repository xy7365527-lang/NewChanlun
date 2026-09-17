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
use newchan_rust::theta_v0::parser::inclusion::{
    CoordNode, CoordsRef, IncrFactsState, StepEvidenceView,
};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::rc::Rc;

/// CC-004 步骤对象的 kind（与全量配方同名）。
pub(super) const CC004_STEP_KIND: &str = "CC-004.inclusion_step";
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
}

/// 只算摘要、不落盘的写入面。
///
/// `obj3` 的六字段身份对象是**派生输入**，不是要发布的节点：它的值摘要 `V` 只进
/// `F(0x20, ·)`，本身不必成为内容节点，故不污染节点表。
struct DigestOnly;

impl NodeSink for DigestOnly {
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
        }
    }
}

/// 一个 CC-004 步骤对象的构造结果（各根都已按**同一次调用所用的 store** 写出）。
///
/// 写 ≠ publish-valid：构造器返回不是发布证明。发布前必须用同一读集过 `verify`（全闭包），
/// 因为 `Value::Edge` 指向的子根不在本次写入的核验范围内。
#[derive(Debug, Clone)]
pub(super) struct Cc004Object {
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
}

impl Cc004Object {
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
    /// 源引用数组 memo：依赖当前源绑定，源修订按坐标失效。
    ref_arrays: ArrayCache,
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
    /// 新坐标大于 memo 上界而跳过的整表遍历次数。
    pub skipped_scans: usize,
}

impl Cc004Producer {
    /// 会话冻结身份（进 §5.3 的六字段配方）。
    pub(super) fn new(profile_id: &str, rule_revision: &str) -> Self {
        Cc004Producer {
            profile_id: profile_id.to_string(),
            rule_revision: rule_revision.to_string(),
            sources: BTreeMap::new(),
            coord_text: HashMap::new(),
            text_arrays: ArrayCache::default(),
            ref_arrays: ArrayCache::default(),
        }
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
        let changed = match self.sources.get(&coord) {
            Some((_, root)) => *root != built.digest,
            None => true,
        };
        self.sources.insert(coord, (spec, built.digest));
        if changed {
            self.ref_arrays.invalidate(coord);
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
        array_from_node(
            &mut *store,
            root,
            &mut self.text_arrays,
            &mut Leaves::CoordText(&mut self.coord_text),
        )
    }

    /// 共享树直接入口（`coords_array` 的底层）：同一逻辑叶序的不同物理树形必须同根。
    pub(super) fn coords_array_of_node(
        &mut self,
        store: &mut dyn NodeStore,
        node: &Rc<CoordNode>,
    ) -> Result<Built, ProducerError> {
        array_from_node(
            &mut *store,
            node,
            &mut self.text_arrays,
            &mut Leaves::CoordText(&mut self.coord_text),
        )
    }

    /// `input_refs`：坐标集合 → 当前源绑定下的八字段源引用数组（0x05）。
    ///
    /// 与 [`Cc004Producer::coords_array`] 用**同一张 coords 叶图**，只是叶按当前源版本
    /// 绑定；任一坐标的源版本变化即失效依赖它的条目。
    pub(super) fn source_refs_array(
        &mut self,
        store: &mut dyn NodeStore,
        coords: &CoordsRef,
    ) -> Result<Built, ProducerError> {
        let Some(root) = coords.root_node() else {
            return empty_array(store);
        };
        array_from_node(
            &mut *store,
            root,
            &mut self.ref_arrays,
            &mut Leaves::SourceRef(&self.sources),
        )
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
        let payload = sc::write_value(&mut *store, &payload_value)?;
        let payload_json = put_json_text_node(&mut *store, &payload.digest)?;
        let input_refs = self.source_refs_array(&mut *store, view.coords)?;
        let input_refs_json = put_json_text_node(&mut *store, &input_refs.digest)?;
        let source_coords = self.coords_array(&mut *store, view.coords)?;
        let source_coords_json = put_json_text_node(&mut *store, &source_coords.digest)?;
        let (object_id, identity) = self.object_identity(&fact_key, &payload, &input_refs)?;
        let record_value = CValue::Object(vec![
            ("object_id".to_string(), CValue::String(object_id.clone())),
            ("object_revision".to_string(), number(OBJECT_REVISION)?),
            (
                "kind".to_string(),
                CValue::String(CC004_STEP_KIND.to_string()),
            ),
            ("fact_key".to_string(), CValue::String(fact_key.clone())),
            ("payload".to_string(), CValue::Edge(payload.digest)),
            ("input_refs".to_string(), CValue::Edge(input_refs.digest)),
            (
                "source_coords".to_string(),
                CValue::Edge(source_coords.digest),
            ),
            ("payload_json".to_string(), CValue::Edge(payload_json)),
            ("input_refs_json".to_string(), CValue::Edge(input_refs_json)),
            (
                "source_coords_json".to_string(),
                CValue::Edge(source_coords_json),
            ),
            ("first_known_generation".to_string(), number(generation)?),
            (
                "first_known_cut".to_string(),
                CValue::String(cut.to_string()),
            ),
            ("published_generation".to_string(), number(generation)?),
        ]);
        let record = sc::write_value(&mut *store, &record_value)?;
        Ok(Cc004Object {
            object_id,
            identity,
            fact_key,
            record,
            payload,
            input_refs,
            source_coords,
            payload_json: Built {
                digest: payload_json,
                kind: Kind::JsonText,
                extent: None,
            },
            input_refs_json: Built {
                digest: input_refs_json,
                kind: Kind::JsonText,
                extent: None,
            },
            source_coords_json: Built {
                digest: source_coords_json,
                kind: Kind::JsonText,
                extent: None,
            },
            generation,
            cut: cut.to_string(),
        })
    }

    /// §5.3：`obj3` = `"obj3-"+hex(F(0x20, V))`，`V` 严格六字段。
    fn object_identity(
        &self,
        fact_key: &str,
        payload: &Built,
        input_refs: &Built,
    ) -> Result<(String, NodeDigest), ProducerError> {
        let identity = CValue::Object(vec![
            (
                "kind".to_string(),
                CValue::String(CC004_STEP_KIND.to_string()),
            ),
            ("fact_key".to_string(), CValue::String(fact_key.to_string())),
            ("payload".to_string(), CValue::Edge(payload.digest)),
            ("input_refs".to_string(), CValue::Edge(input_refs.digest)),
            (
                "profile_id".to_string(),
                CValue::String(self.profile_id.clone()),
            ),
            (
                "rule_revision".to_string(),
                CValue::String(self.rule_revision.clone()),
            ),
        ]);
        let mut derived = DigestOnly;
        let identity_digest = sc::write_value(&mut derived, &identity)?.digest;
        let id = sc::frame(OBJECT_ID_TAG, &identity_digest);
        Ok((format!("obj3-{}", lower_hex(&id)), identity_digest))
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
            Some(evidence) => {
                let coords = self.coords_array(&mut *store, &evidence.coords)?;
                CValue::Object(vec![
                    (
                        "direction".to_string(),
                        CValue::String(dir_str(evidence.direction).to_string()),
                    ),
                    (
                        "established_at".to_string(),
                        CValue::String(evidence.incoming.source_index.to_string()),
                    ),
                    (
                        "previous_acc".to_string(),
                        bar_value(&evidence.previous_acc),
                    ),
                    ("incoming".to_string(), bar_value(&evidence.incoming)),
                    ("source_coords".to_string(), CValue::Edge(coords.digest)),
                ])
            }
            None => CValue::Null,
        };
        let high_sources = self.slice_array(&mut *store, view.high)?;
        let low_sources = self.slice_array(&mut *store, view.low)?;
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

    /// slice 形态的坐标数组（当前值，不物化坐标集合、不做地址 memo）。
    fn slice_array(
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
        let count = u64::try_from(coords.len())
            .map_err(|_| ProducerError::Overflow(format!("坐标数 {} 超出 u64", coords.len())))?;
        let seq_root = sc::write_sequence(&mut *store, &leaves)?;
        let root = put_array_node(&mut *store, count, &seq_root)?;
        Ok(Built {
            digest: root,
            kind: Kind::Array,
            extent: Some(count),
        })
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
        self.clear_memos();
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
            skipped_scans: self.ref_arrays.skipped_scans,
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
fn array_from_node(
    store: &mut dyn NodeStore,
    root: &Rc<CoordNode>,
    cache: &mut ArrayCache,
    leaves: &mut Leaves<'_>,
) -> Result<Built, ProducerError> {
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
    let entry = done.pop().expect("刚核过恰 1 个结果");
    Ok(Built {
        digest: entry.root,
        kind: Kind::Array,
        extent: Some(entry.len),
    })
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

// ────────────────────────────────────────────────────────────────────────────
// 测试面：真实构造器 + 同次 parser；旧 `Builder` 只在这里充当 oracle。
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::content_store::{init_schema, ReadStore, TxStore};
    use super::*;
    use newchan_rust::s_content::{render_canonical, verify, MemoryNodes, NodeRead, NodeSource};
    use sha2::{Digest, Sha256};

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

    /// 同一事务内可读写（读自己刚写的节点）、另记读写次数的节点表。
    #[derive(Default)]
    struct CountingStore {
        nodes: MemoryNodes,
        reads: usize,
        puts: usize,
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
            NodeSink::put(&mut self.nodes, digest, tag, body)
        }
    }

    fn render(store: &mut dyn NodeSource, built: &Built) -> String {
        let mut out: Vec<u8> = Vec::new();
        render_canonical(store, &built.content_ref(), &mut out).expect("规范渲染");
        String::from_utf8(out).expect("UTF-8")
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
    }

    impl Row {
        fn new(coord: i64, revision: i64, receipt: &str) -> Self {
            Row {
                coord,
                revision,
                receipt: receipt.to_string(),
            }
        }

        fn raw_json(&self) -> Value {
            json!({
                "identity_key": format!("id-{}", self.coord),
                "revision": self.revision,
                "input_revision": self.revision.to_string(),
                "payload_hash": format!("ph-{}-{}", self.coord, self.revision),
                "receipt_id": self.receipt.clone(),
                "seq": self.coord + 100,
                "source_coord": self.coord.to_string(),
                "event_id": format!("ev-{}", self.coord),
            })
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
            let row = Row::new(*coord as i64, 1, "r1");
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
        let mut fixture = unbound_fixture(bars);
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
