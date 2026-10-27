//! s-content/1：内容编码与严格节点校验（#1394 采纳 C 第 5.1–5.3 节）。
//!
//! 规范正本：`.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md`
//! 第 5.1（版本、数字和规范字节）、5.2（唯一内容编码）、5.3（身份配方与全文绑定）；
//! 独立正负向量：同目录 `inputs/content-format-vectors.json`。
//!
//! 边界（照实）：
//! - 编解码核心不判业务字段语义、不决定模板占位的出生位置（§5.4 的发布模板位置归 S）。
//!   `job` 与 `job_wire` 另外提供资源记账和 Unix I/O 薄适配，不设资源默认值，
//!   不代表存储、正式 S/Q/E/B 服务和其他传输已经接线。
//! - 精确数字不以 f64 往返：`NumberToken` 只接受合法 JSON 数字 token 并逐字保存。
//! - 读取面不递归：值遍历与序列遍历都用显式栈 + 已核集合，恶意图不会造成无界递归。
//! - ContentRef 是带 schema 的边：构造 V 时接入其指向的值摘要，而不是对
//!   「描述引用的四字段对象」再求值摘要；长得一样的普通 JSON 对象仍是普通对象。
//! - U64 帧长度允许 0..=2^63-1；声明长度只用于比对，绝不用来预分配。

use serde_json::{Map as JsonMap, Value as JsonNode};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::io::Write;

/// 内容格式标识：ContentRef.format 的唯一合法值。
pub const CONTENT_FORMAT: &str = "s-content/1";
/// 根摘要文本前缀：ContentRef.root = `sc1-` + 64 位小写十六进制。
pub const ROOT_PREFIX: &str = "sc1-";
/// 域分隔字节 D = UTF8("NewChanlun/SContent/1") || 0x00。
pub const DOMAIN_SEPARATOR: &[u8] = b"NewChanlun/SContent/1\x00";
/// 字节块长度；只有末块可以更短，非空输入不允许空块。
pub const CHUNK_BYTES: usize = 4096;
/// U64 帧长度上限：0..=2^63-1。
pub const MAX_FRAME_LEN: u64 = (1u64 << 63) - 1;

pub mod access;
#[doc(hidden)]
pub mod capture_allocation;
mod closed;
/// 受控 compact JSON 写出（#1394 局部发送失败闭合）。
pub mod json_writer;
mod ref_owner;
pub use capture_allocation::work::{CodecFields, CodecVec};
use capture_allocation::work::{WorkBudget, WorkMap, WorkSet, WorkVec};
use closed::CodecContext;
#[doc(hidden)]
pub use closed::{ClosedAccess, ClosedCapturedOwner, ClosedReadObserver};
#[doc(hidden)]
pub use ref_owner::{OwnedRefFields, RefDiagnostic, RefOwner, RefView, TransportRef};
/// 整次操作的累计资源与期限（#1394 采纳 C §5.6.1）。
pub mod job;

/// #1527：旧证据图纯行验证；独立于 S-content 编码与计量。
pub mod legacy_evidence;

/// 真实 Unix I/O（调用者提供的 `UnixStream`）接到 job 计费与绝对期限的薄适配（仅 unix）。
/// 只覆盖该连接上的读写字节；connect／spawn／HTTP／stderr 由外层各自带期限与计费。
#[cfg(unix)]
pub mod job_wire;

/// 规范序列（M 树）的持久操作：concat / split / append（#1394 采纳 C §5.2）。
pub mod sequence;
pub use sequence::{
    sequence_append, sequence_append_with_allocation, sequence_concat,
    sequence_concat_with_allocation, sequence_rebuild_suffix,
    sequence_rebuild_suffix_with_allocation, sequence_split, sequence_split_with_allocation,
    NodeStore, SequenceRef,
};

/// §5.4/§5.6 的路径定位与本批占位（0x08）定点替换（#1394 采纳 C）。
pub mod path;

/// 已获准根上的有界片段与独立范围证明；不代替 anchor 授权或完整业务核验。
pub mod fragment;

/// 节点标签表（§5.2）。
pub mod tag {
    pub const NULL: u8 = 0x00;
    pub const BOOL_FALSE: u8 = 0x01;
    pub const BOOL_TRUE: u8 = 0x02;
    pub const STRING: u8 = 0x03;
    pub const NUMBER: u8 = 0x04;
    pub const ARRAY: u8 = 0x05;
    pub const OBJECT: u8 = 0x06;
    pub const JSON_TEXT: u8 = 0x07;
    pub const BATCH_SELF: u8 = 0x08;
    pub const BYTES_CHUNK: u8 = 0x10;
    pub const SEQ_EMPTY: u8 = 0x11;
    pub const SEQ_LEAF: u8 = 0x12;
    pub const SEQ_BRANCH: u8 = 0x13;
    pub const OBJECT_FIELD: u8 = 0x14;
}

/// 稳定拒绝码。名称与独立向量 `python_error` 逐字对应，供 Q/E/B 与迁移对拍。
pub mod code {
    pub const NODE_DIGEST: &str = "node_digest";
    pub const NODE_MISSING: &str = "node_missing";
    pub const SOURCE: &str = "source";
    pub const VALUE_KIND: &str = "value_kind";
    pub const UNKNOWN_TAG: &str = "unknown_tag";
    pub const BODY_SHAPE: &str = "body_shape";
    pub const SEQUENCE_SHAPE: &str = "sequence_shape";
    pub const BYTE_CHUNK_SHAPE: &str = "byte_chunk_shape";
    pub const BYTE_EXTENT: &str = "byte_extent";
    pub const LENGTH_OVERFLOW: &str = "length_overflow";
    pub const UTF8: &str = "utf8";
    pub const NUMBER_TOKEN: &str = "number_token";
    pub const OBJECT_KEY_ORDER: &str = "object_key_order";
    pub const DUPLICATE_KEY: &str = "duplicate_key";
    pub const JSON_TEXT_INTERNAL_KIND: &str = "json_text_internal_kind";
    pub const TEMPLATE_PLACEHOLDER: &str = "template_placeholder";
    pub const FORMAT: &str = "format";
    pub const ROOT: &str = "root";
    pub const KIND: &str = "kind";
    pub const EXTENT: &str = "extent";
    pub const REF_FIELD: &str = "ref_field";
    pub const SINK: &str = "sink";
    pub const IO: &str = "io";
}

/// 内容层错误：稳定 `code` + 人类可读细节。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecError {
    code: &'static str,
    detail: std::borrow::Cow<'static, str>,
    scratch_failure: Option<capture_allocation::scratch::ScratchFailure>,
}

impl CodecError {
    fn scratch(failure: capture_allocation::scratch::ScratchFailure) -> Self {
        let mut error = Self::static_detail(capture_allocation::LIMIT, "codec 工作域分配失败");
        error.scratch_failure = Some(capture_allocation::scratch::traced(failure));
        error
    }
    pub fn scratch_failure(&self) -> Option<capture_allocation::scratch::ScratchFailure> {
        self.scratch_failure
    }

    pub(crate) fn static_detail(code: &'static str, detail: &'static str) -> Self {
        Self {
            code,
            detail: std::borrow::Cow::Borrowed(detail),
            scratch_failure: None,
        }
    }
    pub fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: std::borrow::Cow::Owned(detail.into()),
            scratch_failure: None,
        }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, *self.shown())
    }
}

impl std::error::Error for CodecError {}

fn io_error(err: std::io::Error) -> CodecError {
    if let Some(e) = err
        .get_ref()
        .and_then(|e| e.downcast_ref::<CodecError>())
        .filter(|e| e.scratch_failure().is_some())
    {
        return e.clone();
    }
    CodecError::new(code::IO, err.to_string())
}

/// 逻辑值类型（ContentRef.kind）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Null,
    Bool,
    String,
    Number,
    Array,
    Object,
    JsonText,
    TemplateBatch,
}

impl Kind {
    pub const ALL: [Kind; 8] = [
        Kind::Null,
        Kind::Bool,
        Kind::String,
        Kind::Number,
        Kind::Array,
        Kind::Object,
        Kind::JsonText,
        Kind::TemplateBatch,
    ];

    /// transport 里的 kind 文本。
    pub fn name(self) -> &'static str {
        match self {
            Kind::Null => "null",
            Kind::Bool => "bool",
            Kind::String => "string",
            Kind::Number => "number",
            Kind::Array => "array",
            Kind::Object => "object",
            Kind::JsonText => "json_text",
            Kind::TemplateBatch => "template_batch",
        }
    }

    pub fn from_name(name: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.name() == name)
    }

    /// 值标签的默认标签。bool 的假/真节点分别用 `tag::BOOL_FALSE`/`tag::BOOL_TRUE`。
    pub fn tag(self) -> u8 {
        match self {
            Kind::Null => tag::NULL,
            Kind::Bool => tag::BOOL_FALSE,
            Kind::String => tag::STRING,
            Kind::Number => tag::NUMBER,
            Kind::Array => tag::ARRAY,
            Kind::Object => tag::OBJECT,
            Kind::JsonText => tag::JSON_TEXT,
            Kind::TemplateBatch => tag::BATCH_SELF,
        }
    }

    /// 值标签（0x00..=0x08）到 Kind；内部标签返回 None。
    pub fn from_tag(t: u8) -> Option<Kind> {
        Some(match t {
            tag::NULL => Kind::Null,
            tag::BOOL_FALSE | tag::BOOL_TRUE => Kind::Bool,
            tag::STRING => Kind::String,
            tag::NUMBER => Kind::Number,
            tag::ARRAY => Kind::Array,
            tag::OBJECT => Kind::Object,
            tag::JSON_TEXT => Kind::JsonText,
            tag::BATCH_SELF => Kind::TemplateBatch,
            _ => return None,
        })
    }

    /// string/number 计字节数，array/object 计元素/字段数，其余为 null。
    pub fn extent_is_byte_or_count(self) -> bool {
        matches!(
            self,
            Kind::String | Kind::Number | Kind::Array | Kind::Object
        )
    }
}

/// 校验上下文。模板占位（0x08）只在显式模板上下文里允许。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// 通用公开逻辑值：不得出现模板占位。
    Public,
    /// 显式模板上下文（§5.4 的发布模板位置由 S 另判，codec 不伪造位置规则）。
    Template,
}

/// 32 字节原始摘要（不把十六进制文本拼进 Hash）。
pub type NodeDigest = [u8; 32];

fn be_u64(bytes: &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&bytes[..8]);
    u64::from_be_bytes(buf)
}

fn digest_at(bytes: &[u8], offset: usize) -> NodeDigest {
    let mut d = [0u8; 32];
    d.copy_from_slice(&bytes[offset..offset + 32]);
    d
}

/// U64 帧：八字节无符号大端长度。私有：公开面只有 `checked_frame_len`，
/// 调用点要么先过契约检查，要么其长度直接来自切片（usize ≤ 2^63-1）。
fn frame_len(n: u64) -> [u8; 8] {
    debug_assert!(n <= MAX_FRAME_LEN, "U64 帧长度必须 ≤ 2^63-1");
    n.to_be_bytes()
}

/// 可失败版本：声明长度越界即 `length_overflow`。
pub fn checked_frame_len(n: u64) -> Result<[u8; 8], CodecError> {
    if n > MAX_FRAME_LEN {
        return Err(CodecError::new(
            code::LENGTH_OVERFLOW,
            format!("U64 帧长度 {n} 超过上限 2^63-1"),
        ));
    }
    Ok(n.to_be_bytes())
}

std::thread_local! {
    /// 本线程内 `frame` 的实际调用次数（诊断探针，见 [`frame_calls_on_thread`]）。
    static FRAME_CALLS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    #[cfg(feature = "s_capture_native_testonly")]
    static FRAME_BYTES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// 诊断探针：**本线程**内 [`frame`] 的实际计算次数（只增；跨线程不聚合）。
///
/// 唯一用途是让测试（含二进制 crate 的计费读测试）能对「一次计费读只算一次 `F`」
/// 这类**计数本身**做断言，而不是只能断言间接调用次数。定位如实：**不是**资源判据、
/// 不进任何 Job 账、不参与准入或授权；它不影响 `F` 的结果，也不构成内容核验。
#[doc(hidden)]
pub fn frame_calls_on_thread() -> u64 {
    FRAME_CALLS.with(std::cell::Cell::get)
}

/// 与实际 frame 同位置累计 SHA256 输入字节，仅 Native 诊断使用。
#[cfg(feature = "s_capture_native_testonly")]
#[doc(hidden)]
pub fn frame_bytes_on_thread() -> u64 {
    FRAME_BYTES.with(std::cell::Cell::get)
}

/// F(tag, body) = SHA256(D || one_byte(tag) || U64(len(body)) || body)。
pub fn frame(t: u8, body: &[u8]) -> NodeDigest {
    FRAME_CALLS.with(|calls| calls.set(calls.get().wrapping_add(1)));
    #[cfg(feature = "s_capture_native_testonly")]
    FRAME_BYTES.with(|bytes| {
        bytes.set(
            bytes
                .get()
                .wrapping_add((DOMAIN_SEPARATOR.len() + 9 + body.len()) as u64),
        )
    });
    let mut h = Sha256::new();
    h.update(DOMAIN_SEPARATOR);
    h.update([t]);
    h.update((body.len() as u64).to_be_bytes());
    h.update(body);
    h.finalize().into()
}

/// 32 字节摘要的 transport 文本：`sc1-` + 64 位小写十六进制。
pub fn root_string(digest: &NodeDigest) -> String {
    FixedRoot::new(digest).as_str().to_owned()
}

/// 解析 transport 根文本；前缀、长度、大小写与十六进制字符全部严格。
pub fn parse_root(root: &str) -> Result<NodeDigest, CodecError> {
    let hex = root.strip_prefix(ROOT_PREFIX).ok_or_else(|| {
        CodecError::new(
            code::ROOT,
            format!("根文本 `{root}` 缺少前缀 `{ROOT_PREFIX}`"),
        )
    })?;
    if hex.len() != 64 {
        return Err(CodecError::new(
            code::ROOT,
            format!("根文本 `{root}` 的十六进制长度 {} ≠ 64", hex.len()),
        ));
    }
    let mut d = [0u8; 32];
    let bytes = hex.as_bytes();
    for i in 0..32 {
        let hi = hex_val(bytes[2 * i]).ok_or_else(|| {
            CodecError::new(code::ROOT, format!("根文本 `{root}` 含非小写十六进制字符"))
        })?;
        let lo = hex_val(bytes[2 * i + 1]).ok_or_else(|| {
            CodecError::new(code::ROOT, format!("根文本 `{root}` 含非小写十六进制字符"))
        })?;
        d[i] = (hi << 4) | lo;
    }
    Ok(d)
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

/// k = 严格小于 n 的最大 2 的幂（n ≤ 1 时为 None）。序列分解形状由 k 唯一确定。
pub fn split_index(n: u64) -> Option<u64> {
    if n < 2 {
        None
    } else {
        Some(1u64 << (63 - (n - 1).leading_zeros()))
    }
}

/// M(digests)：序列摘要。空 → 0x11；单叶 → 0x12；n>1 → 0x13（按 k 规则二分）。
pub fn seq_root(leaves: &[NodeDigest]) -> NodeDigest {
    if leaves.is_empty() {
        return frame(tag::SEQ_EMPTY, &[]);
    }
    let mut stack: Vec<(u64, NodeDigest)> = Vec::new();
    for leaf in leaves {
        stack.push((1, frame(tag::SEQ_LEAF, leaf)));
        while stack.len() >= 2 {
            let (n1, d1) = stack[stack.len() - 1];
            let (n2, d2) = stack[stack.len() - 2];
            if n1 != n2 {
                break;
            }
            stack.truncate(stack.len() - 2);
            let n = n1 + n2;
            let mut body = [0u8; 72];
            body[..8].copy_from_slice(&frame_len(n));
            body[8..40].copy_from_slice(&d2);
            body[40..72].copy_from_slice(&d1);
            stack.push((n, frame(tag::SEQ_BRANCH, &body)));
        }
    }
    // 栈内计数自底向上严格递减的 2 的幂；自顶向下折叠即规范 M 的右倾分解。
    let mut it = stack.into_iter().rev();
    let (mut count, mut digest) = it.next().expect("leaves 非空");
    for (left_count, left) in it {
        let total = left_count + count;
        let mut body = [0u8; 72];
        body[..8].copy_from_slice(&frame_len(total));
        body[8..40].copy_from_slice(&left);
        body[40..72].copy_from_slice(&digest);
        digest = frame(tag::SEQ_BRANCH, &body);
        count = total;
    }
    digest
}

/// B(bytes)：按 4096 字节块唯一分解的字节序列摘要（空输入 M([])）。
pub fn bytes_root(bytes: &[u8]) -> NodeDigest {
    if bytes.is_empty() {
        return frame(tag::SEQ_EMPTY, &[]);
    }
    let leaves: Vec<NodeDigest> = bytes
        .chunks(CHUNK_BYTES)
        .map(|c| frame(tag::BYTES_CHUNK, c))
        .collect();
    seq_root(&leaves)
}

/// 节点写入面：`put` 收到的摘要必须等于 F(tag, body)（写入面自己算摘要，故恒成立）。
/// 本模块不把 sink 变成跨服务校验器：写入面只保证自己算的摘要与 (tag, body) 一致。
pub trait NodeSink {
    fn codec_sink_work(&self) -> Option<&capture_allocation::scratch::WorkScope> {
        None
    }

    fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError>;
}

/// 节点读取结果。`Missing` 与「读取出错」是两件事。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeRead {
    Missing,
    Found { tag: u8, body: Vec<u8> },
}

/// 节点来源：按摘要读取实际的 tag 与 body。
///
/// 调用方必须提供**同一次核验内一致**的读集：同一摘要必须返回同一 (tag, body)
/// （不可变快照或等价的一致性保证）。生产 job 可以在任意一次 `read` 返回 `Err` 中止
/// 本轮核验；`Err` 原样上抛，不会变成「部分通过」。
pub trait NodeSource {
    /// 原始读取仍保持原门序，正文与原许可一起移动；没有预先核F或预算降级。
    fn read_scoped(&mut self, digest: &NodeDigest) -> Result<job::ScopedNodeRead, CodecError> {
        self.read(digest)
            .map(|read| job::ScopedNodeRead::from_plain(read, None))
    }

    /// 同一次真实帧读取的临时正文许可；普通来源沿既有检查，Native 包装器保留生命周期。
    fn checked_read_scoped(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<job::ScopedFrame>, CodecError> {
        self.checked_read(digest)
            .map(|frame| frame.map(|f| job::ScopedFrame::new(f, None)))
    }
    /// 独立临时工作域，不改变节点、业务返回值或 Owned 分派。
    fn codec_work(&self) -> Option<&capture_allocation::scratch::WorkScope> {
        None
    }

    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError>;

    // Reached a checked array and an in-range index, immediately before descent.
    // Default leaves existing callers and storage sources unchanged.
    fn before_path_index(
        &mut self,
        _prefix: &[path::PathStep],
        _index: u64,
    ) -> Result<(), CodecError> {
        Ok(())
    }

    /// 仅完整审计 BudgetNone 显式选择 root[] 线性枚举；默认保持旧逐 Index 读取。
    /// 不影响 lookup_path/fragment，不承诺闭包或 JsonInner 上下文已核验。
    fn audit_full_array_scan(&self) -> bool {
        false
    }

    /// 同次读取的已核帧。旧 NodeSource 无须覆写：默认先读取原 Vec，再核一次 F。
    /// 包装器可沿同一调用栈传递证明；作用域内也可先实际重读全部原件，在请求摘要、tag
    /// 与完整 body 逐字相同后仅复用纯 F 证明。不得按摘要跳过真实读取，不得以旧成功
    /// 兜底错误、Missing 或变化的原件；来源、角色、闭包与业务准入仍须逐次核验。
    fn checked_read(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<job::CheckedFrame>, CodecError> {
        match self.read(digest)? {
            NodeRead::Missing => Ok(None),
            NodeRead::Found { tag, body } => {
                job::CheckedFrame::checked(digest, tag, body).map(Some)
            }
        }
    }

    /// 审计源可显式限制单次渲染物化；普通流式源不擅自填默认上限。
    fn max_render_bytes(&self) -> Option<u64> {
        None
    }
}

/// 内存节点表：缓冲/测试用。写入面按摘要校验，损坏节点用 `insert_unchecked` 注入。
#[derive(Debug, Clone, Default)]
pub struct MemoryNodes {
    nodes: BTreeMap<NodeDigest, (u8, Vec<u8>)>,
}

impl MemoryNodes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn contains(&self, digest: &NodeDigest) -> bool {
        self.nodes.contains_key(digest)
    }

    pub fn get(&self, digest: &NodeDigest) -> Option<(u8, &[u8])> {
        self.nodes.get(digest).map(|(t, b)| (*t, b.as_slice()))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&NodeDigest, u8, &[u8])> {
        self.nodes.iter().map(|(d, (t, b))| (d, *t, b.as_slice()))
    }

    /// 直接注入（不核摘要）：只供负例与重建路径。
    pub fn insert_unchecked(&mut self, digest: NodeDigest, tag: u8, body: Vec<u8>) {
        self.nodes.insert(digest, (tag, body));
    }
}

impl NodeSink for MemoryNodes {
    fn put(&mut self, digest: NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
        let actual = frame(tag, body);
        if actual != digest {
            return Err(CodecError::new(
                code::SINK,
                format!(
                    "写入摘要与 (tag, body) 重算不符：声称 {}，实算 {}",
                    root_string(&digest),
                    root_string(&actual)
                ),
            ));
        }
        self.nodes.insert(digest, (tag, body.to_vec()));
        Ok(())
    }
}

impl NodeSource for MemoryNodes {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        Ok(match self.nodes.get(digest) {
            Some((t, b)) => NodeRead::Found {
                tag: *t,
                body: b.clone(),
            },
            None => NodeRead::Missing,
        })
    }
}

/// 精确 JSON 数字 token。只接受合法 JSON 数字文法，绝不经 f64 往返。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NumberToken(String);

impl NumberToken {
    /// RFC 8259 数字文法：`-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`。
    pub fn parse(token: &str) -> Result<NumberToken, CodecError> {
        if is_json_number(token) {
            Ok(NumberToken(token.to_string()))
        } else {
            Err(CodecError::new(
                code::NUMBER_TOKEN,
                format!("`{token}` 不是合法 JSON 数字 token"),
            ))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Display for NumberToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn is_json_number(token: &str) -> bool {
    let b = token.as_bytes();
    let mut i = 0;
    if i < b.len() && b[i] == b'-' {
        i += 1;
    }
    match b.get(i) {
        Some(b'0') => i += 1,
        Some(c) if c.is_ascii_digit() => {
            i += 1;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
        }
        _ => return false,
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return false;
        }
    }
    i == b.len()
}

/// json_text（0x07）内部值树：只允许值标签 0x00..=0x06。
/// 类型上就装不进 0x07/0x08，构造期即挡住嵌套文本视图与批次占位。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    String(String),
    Number(NumberToken),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

/// 编码输入：明确 Rust 类型或原始合法 token；`Edge` 表示已有子根直接成边。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    String(String),
    Number(NumberToken),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
    JsonText(Box<JsonValue>),
    BatchSelf,
    /// 已有子根直接成边：V 即该摘要，不对任何「四字段对象」重哈希。
    /// 边只引用摘要：不写入也不核验其节点，写入成功不等于闭包可核。
    Edge(NodeDigest),
}

impl Value {
    pub fn string(s: impl Into<String>) -> Value {
        Value::String(s.into())
    }

    /// 由 ContentRef 得到边值：只取其指向的值摘要。
    pub fn edge_of_ref(reference: &ContentRef) -> Result<Value, CodecError> {
        Ok(Value::Edge(reference.digest()?))
    }
}

/// 写入结果：根摘要 + 逻辑类型 + extent（string/number 为字节数，array/object 为元素/字段数）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Built {
    pub digest: NodeDigest,
    pub kind: Kind,
    pub extent: Option<u64>,
}

impl Built {
    #[cfg(feature = "s_allocation_json_testonly")]
    pub fn owned_ref(&self) -> RefOwner {
        RefOwner::new(self.kind, self.digest, self.extent)
    }
    pub fn root(&self) -> String {
        root_string(&self.digest)
    }

    pub fn content_ref(&self) -> ContentRef {
        ContentRef {
            format: CONTENT_FORMAT.to_string(),
            root: self.root(),
            kind: self.kind,
            extent: self.extent,
        }
    }
}

/// content 图的 transport 引用边。exact 字段为 format,root,kind,extent。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentRef {
    pub format: String,
    pub root: String,
    pub kind: Kind,
    pub extent: Option<u64>,
}

impl ContentRef {
    pub fn new(kind: Kind, digest: &NodeDigest, extent: Option<u64>) -> ContentRef {
        ContentRef {
            format: CONTENT_FORMAT.to_string(),
            root: root_string(digest),
            kind,
            extent,
        }
    }

    pub fn digest(&self) -> Result<NodeDigest, CodecError> {
        transport_digest(&self.format, &self.root)
    }

    /// 严格 transport 形态：四字段齐备、无多余字段、类型精确。
    /// extent 是 U64 长度，协议上用规范十进制字符串传输（非长度型为 null）。
    pub fn to_transport_json(&self) -> JsonNode {
        let mut map = JsonMap::new();
        map.insert("format".into(), JsonNode::String(self.format.clone()));
        map.insert("root".into(), JsonNode::String(self.root.clone()));
        map.insert(
            "kind".into(),
            JsonNode::String(self.kind.name().to_string()),
        );
        map.insert(
            "extent".into(),
            match self.extent {
                Some(n) => JsonNode::String(n.to_string()),
                None => JsonNode::Null,
            },
        );
        JsonNode::Object(map)
    }

    pub fn from_transport_json(node: &JsonNode) -> Result<ContentRef, CodecError> {
        let claim = TransportRef::from_json_view(node.into())?;
        Ok(ContentRef {
            format: claim.format().to_string(),
            root: claim.root().to_string(),
            kind: claim.kind(),
            extent: claim.extent(),
        })
    }
}

// Both normal and borrowed transport declarations defer these checks to digest().
fn transport_digest(format: &str, root: &str) -> Result<NodeDigest, CodecError> {
    if format != CONTENT_FORMAT {
        return Err(CodecError::new(
            code::FORMAT,
            format!("ContentRef.format 必须是 `{CONTENT_FORMAT}`，实为 `{format}`"),
        ));
    }
    parse_root(root)
}

/// 公开控制数值的规范十进制：`0` 或 `[1-9][0-9]*`，无 `+`、无前导零、≤ 2^63-1。
#[derive(Debug)]
pub enum DecimalError<'a> {
    Characters {
        input: &'a str,
        field: &'a str,
    },
    LeadingZero {
        input: &'a str,
        field: &'a str,
    },
    Overflow {
        input: &'a str,
        field: &'a str,
        cause: std::num::ParseIntError,
    },
    Limit {
        input: &'a str,
        field: &'a str,
    },
}
impl DecimalError<'_> {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Characters { .. } | Self::LeadingZero { .. } => code::EXTENT,
            _ => code::LENGTH_OVERFLOW,
        }
    }
}
impl fmt::Display for DecimalError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Self::Characters { input, field } => write!(
                f,
                "{field} `{input}` 不是规范十进制：只允许 0-9，无符号、无空白"
            ),
            Self::LeadingZero { input, field } => {
                write!(f, "{field} `{input}` 有前导零，非规范十进制")
            }
            Self::Overflow { input, field, .. } => write!(f, "{field} `{input}` 超出 u64 表达"),
            Self::Limit { input, field } => write!(f, "{field} `{input}` 超过上限 2^63-1"),
        }
    }
}
#[allow(
    dead_code,
    reason = "#1608：CodecError 口径的十进制解析便捷包装，现有调用方均直接用 parse_canonical_u64_typed"
)]
fn parse_canonical_u64(text: &str, what: &str) -> Result<u64, CodecError> {
    parse_canonical_u64_typed(text, what).map_err(|e| {
        let code = e.code();
        let rendered = e.to_string();
        CodecError::new(
            code,
            rendered
                .strip_prefix(code)
                .unwrap()
                .strip_prefix(": ")
                .unwrap(),
        )
    })
}
pub fn parse_canonical_u64_typed<'a>(
    text: &'a str,
    what: &'a str,
) -> Result<u64, DecimalError<'a>> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || !bytes.iter().all(|b| b.is_ascii_digit()) {
        return Err(DecimalError::Characters {
            input: text,
            field: what,
        });
    }
    if bytes[0] == b'0' && bytes.len() > 1 {
        return Err(DecimalError::LeadingZero {
            input: text,
            field: what,
        });
    }
    let n = text
        .parse::<u64>()
        .map_err(|cause| DecimalError::Overflow {
            input: text,
            field: what,
            cause,
        })?;
    if n > MAX_FRAME_LEN {
        return Err(DecimalError::Limit {
            input: text,
            field: what,
        });
    }
    Ok(n)
}

/// 写入字节序列 B：落块节点与序列节点，返回根摘要。
pub fn write_bytes(sink: &mut dyn NodeSink, bytes: &[u8]) -> Result<NodeDigest, CodecError> {
    write_bytes_ctx(&mut CodecContext::Sink(sink, None), bytes)
}
pub fn write_sequence(
    sink: &mut dyn NodeSink,
    leaves: &[NodeDigest],
) -> Result<NodeDigest, CodecError> {
    write_sequence_ctx(&mut CodecContext::Sink(sink, None), leaves)
}
fn write_bytes_ctx(
    sink: &mut CodecContext<'_, '_>,
    bytes: &[u8],
) -> Result<NodeDigest, CodecError> {
    if bytes.is_empty() {
        let digest = put_node(sink, tag::SEQ_EMPTY, &[])?;
        return Ok(digest);
    }
    let budget = sink.work_budget();
    let mut leaves =
        WorkVec::with_work_capacity(bytes.len().div_ceil(CHUNK_BYTES), budget.as_ref())?;
    for chunk in bytes.chunks(CHUNK_BYTES) {
        let digest = put_node(sink, tag::BYTES_CHUNK, chunk)?;
        leaves.push(digest)?;
    }
    write_sequence_ctx(sink, &leaves)
}

/// 写入序列节点 M(leaves)：空 → 0x11；单叶 → 0x12；其余按 k 规则分解。
fn write_sequence_ctx(
    sink: &mut CodecContext<'_, '_>,
    leaves: &[NodeDigest],
) -> Result<NodeDigest, CodecError> {
    if leaves.is_empty() {
        let digest = put_node(sink, tag::SEQ_EMPTY, &[])?;
        return Ok(digest);
    }
    if u64::try_from(leaves.len()).map_err(|_| {
        CodecError::new(
            code::LENGTH_OVERFLOW,
            format!("序列叶数 {} 超过 u64 表达", leaves.len()),
        )
    })? > MAX_FRAME_LEN
    {
        return Err(CodecError::new(
            code::LENGTH_OVERFLOW,
            format!("序列叶数 {} 超过 2^63-1", leaves.len()),
        ));
    }
    let budget = sink.work_budget();
    let mut stack: WorkVec<(u64, NodeDigest)> = WorkVec::new_work(budget.as_ref());
    for leaf in leaves {
        let leaf_node = put_node(sink, tag::SEQ_LEAF, leaf)?;
        stack.push((1, leaf_node))?;
        while stack.len() >= 2 {
            let (n1, d1) = stack[stack.len() - 1];
            let (n2, d2) = stack[stack.len() - 2];
            if n1 != n2 {
                break;
            }
            stack.truncate(stack.len() - 2);
            let n = n1 + n2;
            let digest = put_branch(sink, &d2, &d1, n)?;
            stack.push((n, digest))?;
        }
    }
    let mut it = stack.into_iter().rev();
    let (mut count, mut digest) = it.next().expect("leaves 非空");
    for (left_count, left) in it {
        let total = left_count + count;
        digest = put_branch(sink, &left, &digest, total)?;
        count = total;
    }
    let expected = u64::try_from(leaves.len()).expect("叶数可表达");
    if count != expected {
        return Err(CodecError::new(
            code::SEQUENCE_SHAPE,
            format!("序列叶数记账不符：实得 {count}，应为 {expected}"),
        ));
    }
    Ok(digest)
}

fn put_branch(
    sink: &mut CodecContext<'_, '_>,
    left: &NodeDigest,
    right: &NodeDigest,
    n: u64,
) -> Result<NodeDigest, CodecError> {
    let mut body = [0u8; 72];
    body[..8].copy_from_slice(&checked_frame_len(n)?);
    body[8..40].copy_from_slice(left);
    body[40..72].copy_from_slice(right);
    let digest = put_node(sink, tag::SEQ_BRANCH, &body)?;
    Ok(digest)
}

/// 写入值树（通用公开逻辑值）：模板占位（0x08）拒绝。
///
/// 写入 ≠ publish-valid：`Value::Edge` 指向的子根不在本次写入的核验范围内，
/// 也可能指向缺失节点；publish 前必须过 `verify`。
pub fn write_value(sink: &mut dyn NodeSink, value: &Value) -> Result<Built, CodecError> {
    write_in_mode(&mut CodecContext::Sink(sink, None), value, Mode::Public)
}

/// #1394 Native 对照臂的单次值树材料驻留；每次返回（含错误）即销毁。
///
/// 只按标签和完整 body 复用本次成功写入的材料，不证明当前存储、闭包或来源资格。
/// 调用方仅用于已取得同次 parser 资格的固定 24 关系树；发布仍须完整冷核验。
/// 普通/Owned 入口不经此处，也不由此声称任意大 Value 的 heap 资源合格。
#[cfg(feature = "s_capture_native_testonly")]
#[doc(hidden)]
pub fn write_value_native_local_for_testonly(
    sink: &mut dyn NodeSink,
    value: &Value,
) -> Result<Built, CodecError> {
    let mut memo = closed::NativeLocalMaterialMemo::default();
    write_in_mode(
        &mut CodecContext::NativeLocal {
            sink,
            memo: &mut memo,
        },
        value,
        Mode::Public,
    )
}

/// 写入值树（显式模板上下文）：允许 `Value::BatchSelf`。
pub fn write_template(sink: &mut dyn NodeSink, value: &Value) -> Result<Built, CodecError> {
    write_in_mode(&mut CodecContext::Sink(sink, None), value, Mode::Template)
}

fn write_in_mode(
    sink: &mut CodecContext<'_, '_>,
    value: &Value,
    mode: Mode,
) -> Result<Built, CodecError> {
    let (kind, extent) = value_kind_extent(value, mode)?;
    write_steps(sink, WStep::Value(value), kind, extent, mode)
}
fn write_steps<'a>(
    sink: &mut CodecContext<'_, '_>,
    initial: WStep<'a>,
    kind: Kind,
    extent: Option<u64>,
    mode: Mode,
) -> Result<Built, CodecError> {
    let operation_work = sink.work_budget();
    let operation_result = write_steps_inner(sink, initial, kind, extent, mode);
    if let Some(work) = operation_work {
        work.observe(&operation_result);
    }
    operation_result
}
fn write_steps_inner<'a>(
    sink: &mut CodecContext<'_, '_>,
    initial: WStep<'a>,
    kind: Kind,
    extent: Option<u64>,
    mode: Mode,
) -> Result<Built, CodecError> {
    let budget = sink.work_budget();
    let mut steps = WorkVec::with_work_capacity(1, budget.as_ref())?;
    steps.push(initial)?;
    let mut done: WorkVec<NodeDigest> = WorkVec::new_work(budget.as_ref());
    while let Some(step) = steps.pop() {
        match step {
            WStep::Value(v) => write_value_step(sink, v, mode, &mut steps, &mut done)?,
            WStep::View(v) => write_value_view_step(sink, v, mode, &mut steps, &mut done)?,
            WStep::ViewArray(mut i) => {
                if let Some(v) = i.next() {
                    steps.push(WStep::ViewArray(i))?;
                    steps.push(WStep::View(*v))?;
                }
            }
            WStep::ViewObject(mut i) => {
                if let Some((_, v)) = i.next() {
                    steps.push(WStep::ViewObject(i))?;
                    steps.push(WStep::View(*v))?;
                }
            }
            WStep::Json(v) => write_json_step(sink, v, &mut steps, &mut done)?,
            WStep::Borrowed(v) => write_borrowed_json_step(sink, v, &mut steps, &mut done)?,
            WStep::BorrowedArray(mut i) => {
                if let Some(v) = i.next() {
                    steps.push(WStep::BorrowedArray(i))?;
                    steps.push(WStep::Borrowed(v))?;
                }
            }
            WStep::BorrowedObject(mut i) => {
                if let Some((_, v)) = i.next() {
                    steps.push(WStep::BorrowedObject(i))?;
                    steps.push(WStep::Borrowed(v))?;
                }
            }
            WStep::FinishArray { base } => {
                let children = &done[base..];
                let n = u64::try_from(children.len()).expect("元素数可表达");
                let fields = write_sequence_ctx(sink, children)?;
                let mut body = WorkVec::with_work_capacity(40, sink.work_budget().as_ref())?;
                body.extend_from_slice(&checked_frame_len(n)?)?;
                body.extend_from_slice(&fields)?;
                let digest = put_node(sink, tag::ARRAY, &body)?;
                done.truncate(base);
                done.push(digest)?;
            }
            WStep::FinishObject { base, keys } => {
                let n = u64::try_from(keys.len()).expect("字段数可表达");
                if done.len() != base + keys.len() {
                    return Err(CodecError::new(
                        code::SEQUENCE_SHAPE,
                        "对象字段记账与已写入子值不符".to_string(),
                    ));
                }
                let mut order = WorkVec::with_work_capacity(keys.len(), budget.as_ref())?;
                for i in 0..keys.len() {
                    order.push(i)?;
                }
                order.sort_by(|&a, &b| keys[a].as_bytes().cmp(keys[b].as_bytes()));
                for w in order.windows(2) {
                    if keys[w[0]] == keys[w[1]] {
                        return Err(CodecError::new(
                            code::DUPLICATE_KEY,
                            format!("对象出现重复键 `{}`", keys[w[0]]),
                        ));
                    }
                }
                let mut field_digests = WorkVec::with_work_capacity(order.len(), budget.as_ref())?;
                for &i in &order {
                    let key_digest = write_string_node(sink, keys[i])?;
                    let mut body = [0u8; 64];
                    body[..32].copy_from_slice(&key_digest);
                    body[32..].copy_from_slice(&done[base + i]);
                    field_digests.push(put_node(sink, tag::OBJECT_FIELD, &body)?)?;
                }
                let fields = write_sequence_ctx(sink, &field_digests)?;
                let mut body = WorkVec::with_work_capacity(40, sink.work_budget().as_ref())?;
                body.extend_from_slice(&checked_frame_len(n)?)?;
                body.extend_from_slice(&fields)?;
                let digest = put_node(sink, tag::OBJECT, &body)?;
                done.truncate(base);
                done.push(digest)?;
            }
            WStep::FinishJsonText { base } => {
                let inner = done[base];
                let digest = put_node(sink, tag::JSON_TEXT, &inner)?;
                done.truncate(base);
                done.push(digest)?;
            }
        }
    }
    if done.len() != 1 {
        return Err(CodecError::new(
            code::SEQUENCE_SHAPE,
            format!("写入结果记账异常：{} 个根", done.len()),
        ));
    }
    Ok(Built {
        digest: done[0],
        kind,
        extent,
    })
}

/// 只借用逻辑数据和已有边的内部构造视图，编码/模式判断继续共用WStep。
#[derive(Clone, Copy)]
pub enum ValueView<'a> {
    Json(capture_allocation::audit_json::JsonView<'a>),
    String(&'a str),
    Number(BorrowedNumberToken<'a>),
    Array(&'a [ValueView<'a>]),
    Object(&'a [(&'a str, ValueView<'a>)]),
    Edge(NodeDigest),
    BatchSelf,
}
#[derive(Clone, Copy)]
pub struct BorrowedNumberToken<'a>(&'a str);
impl<'a> BorrowedNumberToken<'a> {
    pub fn parse(token: &'a str) -> Result<Self, CodecError> {
        if is_json_number(token) {
            Ok(Self(token))
        } else {
            Err(CodecError::new(
                code::NUMBER_TOKEN,
                format!("`{token}` 不是合法 JSON 数字 token"),
            ))
        }
    }
}
impl<'a> From<capture_allocation::audit_json::JsonView<'a>> for ValueView<'a> {
    fn from(v: capture_allocation::audit_json::JsonView<'a>) -> Self {
        Self::Json(v)
    }
}
enum WStep<'a> {
    Value(&'a Value),
    View(ValueView<'a>),
    ViewArray(std::slice::Iter<'a, ValueView<'a>>),
    ViewObject(std::slice::Iter<'a, (&'a str, ValueView<'a>)>),
    Json(&'a JsonValue),
    Borrowed(capture_allocation::audit_json::JsonView<'a>),
    BorrowedArray(capture_allocation::audit_json::ArrayIter<'a>),
    BorrowedObject(capture_allocation::audit_json::ObjectIter<'a>),
    FinishArray { base: usize },
    FinishObject { base: usize, keys: WorkVec<&'a str> },
    FinishJsonText { base: usize },
}

/// 同一 WStep/FinishObject 核直接消费借用业务树，不构造 sc::Value 深复制。
pub fn write_json_view(
    sink: &mut dyn NodeSink,
    v: capture_allocation::audit_json::JsonView<'_>,
) -> Result<Built, CodecError> {
    write_json_view_ctx(&mut CodecContext::Sink(sink, None), v)
}
#[cfg(feature = "s_allocation_json_testonly")]
#[doc(hidden)]
pub fn write_json_view_for_allocation_prototype(
    sink: &mut dyn NodeSink,
    v: capture_allocation::audit_json::JsonView<'_>,
    b: &capture_allocation::OwnedBudget,
) -> Result<Built, CodecError> {
    write_json_view_ctx(&mut CodecContext::Sink(sink, Some(b.clone())), v)
}
fn write_json_view_ctx(
    sink: &mut CodecContext<'_, '_>,
    v: capture_allocation::audit_json::JsonView<'_>,
) -> Result<Built, CodecError> {
    let (kind, extent) = json_view_kind_extent(v)?;
    write_steps(sink, WStep::Borrowed(v), kind, extent, Mode::Public)
}
fn json_view_kind_extent(
    v: capture_allocation::audit_json::JsonView<'_>,
) -> Result<(Kind, Option<u64>), CodecError> {
    use capture_allocation::audit_json::JsonKind;
    Ok(match v.kind() {
        JsonKind::Null => (Kind::Null, None),
        JsonKind::Bool => (Kind::Bool, None),
        JsonKind::String => (Kind::String, Some(v.as_str().unwrap().len() as u64)),
        JsonKind::Number => {
            let b = capture_allocation::audit_json::formatted_number(v.number().unwrap());
            (Kind::Number, Some(b.as_bytes().len() as u64))
        }
        JsonKind::Array => (Kind::Array, Some(v.as_array().unwrap().len() as u64)),
        JsonKind::Object => (Kind::Object, Some(v.as_object().unwrap().len() as u64)),
    })
}
fn write_borrowed_json_step<'a>(
    sink: &mut CodecContext<'_, '_>,
    v: capture_allocation::audit_json::JsonView<'a>,
    steps: &mut WorkVec<WStep<'a>>,
    done: &mut WorkVec<NodeDigest>,
) -> Result<(), CodecError> {
    use capture_allocation::audit_json::JsonKind;
    match v.kind() {
        JsonKind::Null => done.push(put_node(sink, tag::NULL, &[])?)?,
        JsonKind::Bool => done.push(put_node(
            sink,
            if v.as_bool().unwrap() {
                tag::BOOL_TRUE
            } else {
                tag::BOOL_FALSE
            },
            &[],
        )?)?,
        JsonKind::String => done.push(write_string_node(sink, v.as_str().unwrap())?)?,
        JsonKind::Number => {
            let b = capture_allocation::audit_json::formatted_number(v.number().unwrap());
            done.push(write_byte_node(sink, tag::NUMBER, b.as_bytes())?)?;
        }
        JsonKind::Array => {
            steps.push(WStep::FinishArray { base: done.len() })?;
            steps.push(WStep::BorrowedArray(v.as_array().unwrap().iter()))?;
        }
        JsonKind::Object => {
            let object = v.as_object().unwrap();
            let mut keys = WorkVec::with_work_capacity(object.len(), sink.work_budget().as_ref())?;
            for k in object.keys() {
                keys.push(k)?;
            }
            steps.push(WStep::FinishObject {
                base: done.len(),
                keys,
            })?;
            steps.push(WStep::BorrowedObject(object.iter()))?;
        }
    }
    Ok(())
}

pub fn write_value_view(sink: &mut dyn NodeSink, v: ValueView<'_>) -> Result<Built, CodecError> {
    write_value_view_ctx(&mut CodecContext::Sink(sink, None), v, Mode::Public)
}
pub fn write_template_view(sink: &mut dyn NodeSink, v: ValueView<'_>) -> Result<Built, CodecError> {
    write_value_view_ctx(&mut CodecContext::Sink(sink, None), v, Mode::Template)
}
fn view_kind_extent(v: ValueView<'_>, mode: Mode) -> Result<(Kind, Option<u64>), CodecError> {
    match v {
        ValueView::String(s) => Ok((Kind::String, Some(s.len() as u64))),
        ValueView::Number(n) => Ok((Kind::Number, Some(n.0.len() as u64))),
        ValueView::Array(a) => Ok((Kind::Array, Some(a.len() as u64))),
        ValueView::Object(a) => Ok((Kind::Object, Some(a.len() as u64))),
        ValueView::BatchSelf => value_kind_extent(&Value::BatchSelf, mode),
        ValueView::Edge(d) => value_kind_extent(&Value::Edge(d), mode),
        ValueView::Json(j) => json_view_kind_extent(j),
    }
}
fn write_value_view_ctx(
    sink: &mut CodecContext<'_, '_>,
    v: ValueView<'_>,
    mode: Mode,
) -> Result<Built, CodecError> {
    let (k, e) = view_kind_extent(v, mode)?;
    write_steps(sink, WStep::View(v), k, e, mode)
}
fn write_value_view_step<'a>(
    sink: &mut CodecContext<'_, '_>,
    v: ValueView<'a>,
    mode: Mode,
    steps: &mut WorkVec<WStep<'a>>,
    done: &mut WorkVec<NodeDigest>,
) -> Result<(), CodecError> {
    match v {
        ValueView::Json(j) => write_borrowed_json_step(sink, j, steps, done)?,
        ValueView::String(s) => done.push(write_string_node(sink, s)?)?,
        ValueView::Number(n) => done.push(write_byte_node(sink, tag::NUMBER, n.0.as_bytes())?)?,
        ValueView::Edge(d) => done.push(d)?,
        ValueView::BatchSelf => write_value_step(sink, &Value::BatchSelf, mode, steps, done)?,
        ValueView::Array(a) => {
            steps.push(WStep::FinishArray { base: done.len() })?;
            steps.push(WStep::ViewArray(a.iter()))?;
        }
        ValueView::Object(a) => {
            let mut keys = WorkVec::with_work_capacity(a.len(), sink.work_budget().as_ref())?;
            for (k, _) in a {
                keys.push(*k)?;
            }
            steps.push(WStep::FinishObject {
                base: done.len(),
                keys,
            })?;
            steps.push(WStep::ViewObject(a.iter()))?;
        }
    }
    Ok(())
}
fn put_node(sink: &mut CodecContext<'_, '_>, t: u8, body: &[u8]) -> Result<NodeDigest, CodecError> {
    sink.emit(t, body)
}

fn write_string_node(
    sink: &mut CodecContext<'_, '_>,
    text: &str,
) -> Result<NodeDigest, CodecError> {
    let bytes = text.as_bytes();
    let root = write_bytes_ctx(sink, bytes)?;
    let mut body = WorkVec::with_work_capacity(40, sink.work_budget().as_ref())?;
    body.extend_from_slice(&checked_frame_len(
        u64::try_from(bytes.len()).expect("字节数可表达"),
    )?)?;
    body.extend_from_slice(&root)?;
    put_node(sink, tag::STRING, &body)
}

fn write_byte_node(
    sink: &mut CodecContext<'_, '_>,
    t: u8,
    bytes: &[u8],
) -> Result<NodeDigest, CodecError> {
    let root = write_bytes_ctx(sink, bytes)?;
    let mut body = WorkVec::with_work_capacity(40, sink.work_budget().as_ref())?;
    body.extend_from_slice(&checked_frame_len(
        u64::try_from(bytes.len()).expect("字节数可表达"),
    )?)?;
    body.extend_from_slice(&root)?;
    put_node(sink, t, &body)
}

fn value_kind_extent(value: &Value, mode: Mode) -> Result<(Kind, Option<u64>), CodecError> {
    Ok(match value {
        Value::Null => (Kind::Null, None),
        Value::Bool(_) => (Kind::Bool, None),
        Value::String(s) => (Kind::String, Some(s.len() as u64)),
        Value::Number(t) => (Kind::Number, Some(t.as_str().len() as u64)),
        Value::Array(items) => (Kind::Array, Some(items.len() as u64)),
        Value::Object(fields) => (Kind::Object, Some(fields.len() as u64)),
        Value::JsonText(_) => (Kind::JsonText, None),
        Value::BatchSelf => {
            if mode != Mode::Template {
                return Err(CodecError::new(
                    code::TEMPLATE_PLACEHOLDER,
                    "模板占位（0x08）只在显式模板上下文里允许写".to_string(),
                ));
            }
            (Kind::TemplateBatch, None)
        }
        Value::Edge(_) => {
            return Err(CodecError::new(
                code::KIND,
                "根值不能是既有边：边没有 kind/extent；请用 content_ref_of 从节点来源读根"
                    .to_string(),
            ))
        }
    })
}

fn write_value_step<'a>(
    sink: &mut CodecContext<'_, '_>,
    value: &'a Value,
    mode: Mode,
    steps: &mut WorkVec<WStep<'a>>,
    done: &mut WorkVec<NodeDigest>,
) -> Result<(), CodecError> {
    match value {
        Value::Null => done.push(put_node(sink, tag::NULL, &[])?)?,
        Value::Bool(false) => done.push(put_node(sink, tag::BOOL_FALSE, &[])?)?,
        Value::Bool(true) => done.push(put_node(sink, tag::BOOL_TRUE, &[])?)?,
        Value::String(s) => done.push(write_string_node(sink, s)?)?,
        Value::Number(t) => done.push(write_byte_node(sink, tag::NUMBER, t.bytes())?)?,
        Value::Array(items) => {
            steps.push(WStep::FinishArray { base: done.len() })?;
            for item in items.iter().rev() {
                steps.push(WStep::Value(item))?;
            }
        }
        Value::Object(fields) => {
            steps.push(WStep::FinishObject {
                base: done.len(),
                keys: {
                    let mut keys =
                        WorkVec::with_work_capacity(fields.len(), sink.work_budget().as_ref())?;
                    for (k, _) in fields {
                        keys.push(k.as_str())?;
                    }
                    keys
                },
            })?;
            for (_, v) in fields.iter().rev() {
                steps.push(WStep::Value(v))?;
            }
        }
        Value::JsonText(inner) => {
            steps.push(WStep::FinishJsonText { base: done.len() })?;
            steps.push(WStep::Json(inner))?;
        }
        Value::BatchSelf => {
            if mode != Mode::Template {
                return Err(CodecError::new(
                    code::TEMPLATE_PLACEHOLDER,
                    "模板占位（0x08）只在显式模板上下文里允许写".to_string(),
                ));
            }
            done.push(put_node(sink, tag::BATCH_SELF, &[])?)?;
        }
        Value::Edge(digest) => done.push(*digest)?,
    }
    Ok(())
}

fn write_json_step<'a>(
    sink: &mut CodecContext<'_, '_>,
    value: &'a JsonValue,
    steps: &mut WorkVec<WStep<'a>>,
    done: &mut WorkVec<NodeDigest>,
) -> Result<(), CodecError> {
    match value {
        JsonValue::Null => done.push(put_node(sink, tag::NULL, &[])?)?,
        JsonValue::Bool(false) => done.push(put_node(sink, tag::BOOL_FALSE, &[])?)?,
        JsonValue::Bool(true) => done.push(put_node(sink, tag::BOOL_TRUE, &[])?)?,
        JsonValue::String(s) => done.push(write_string_node(sink, s)?)?,
        JsonValue::Number(t) => done.push(write_byte_node(sink, tag::NUMBER, t.bytes())?)?,
        JsonValue::Array(items) => {
            steps.push(WStep::FinishArray { base: done.len() })?;
            for item in items.iter().rev() {
                steps.push(WStep::Json(item))?;
            }
        }
        JsonValue::Object(fields) => {
            steps.push(WStep::FinishObject {
                base: done.len(),
                keys: {
                    let mut keys =
                        WorkVec::with_work_capacity(fields.len(), sink.work_budget().as_ref())?;
                    for (k, _) in fields {
                        keys.push(k.as_str())?;
                    }
                    keys
                },
            })?;
            for (_, v) in fields.iter().rev() {
                steps.push(WStep::Json(v))?;
            }
        }
    }
    Ok(())
}

// ── 读取面：直接处理唯一节点/图的严格校验与惰性规范渲染 ───────────────────────
//
// 直接处理图，不展开逻辑重复子树：
// - 值节点按 (实际值根, 上下文) 去重核验；同一摘要在公开值与 json_text 内部各自核。
// - 序列按上下文缓存折叠摘要：规范 M 形状与哈希仍逐节点核，同一子序列只走一次。
// - 字节序列的 UTF-8 与数字文法是有限状态转换，(摘要, 进态) 记忆化组合：
//   跨 4096 块照样逐字节推进，共享重复块只算一次。
// - 对象字段序列缓存 (计数, 首键, 末键)：顺序与唯一性在组合处判定；
//   同一键摘要即同一字符串，重复键无须读键内容。
// - 渲染用惰性叶迭代，按 sink 实际消费推进；sink 报错即中止，可能已写出部分字节。

/// 已核验的根：ContentRef 四字段与真实节点逐项对应，且全闭包通过严格核验。
///
/// `nodes` 是闭包里**实际读到的不同摘要数**（唯一节点数），重复子树不重复计。
/// 写入成功（`write_value`）只说明「图被构造出来了」：`Value::Edge` 指向的子根不写入
/// 也不核验，publish 前必须过 `verify`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    pub content_ref: ContentRef,
    pub digest: NodeDigest,
    pub kind: Kind,
    pub extent: Option<u64>,
    pub nodes: usize,
}

/// 节点在树里的位置：逻辑值位 / json_text 内部位。两者各自核验（同一摘要可两处出现）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Flavor {
    Value,
    JsonInner,
}

fn utf8_error() -> CodecError {
    CodecError::new(
        code::UTF8,
        "文本不是有效 UTF-8（含孤立代理项、截断或非法序列）".to_string(),
    )
}

fn value_kind_error(t: u8) -> CodecError {
    CodecError::new(
        code::VALUE_KIND,
        format!("tag={t:02x} 是内部节点/字节块，不能充当逻辑值"),
    )
}

fn body_shape_error(t: u8, body: &[u8], want: &str) -> CodecError {
    CodecError::new(
        code::BODY_SHAPE,
        format!("tag={t:02x} 的 body 长度 {} 不符：{want}", body.len()),
    )
}

fn sequence_shape_error(detail: impl Into<String>) -> CodecError {
    CodecError::new(code::SEQUENCE_SHAPE, detail.into())
}

fn byte_chunk_shape_error(detail: impl Into<String>) -> CodecError {
    CodecError::new(code::BYTE_CHUNK_SHAPE, detail.into())
}

fn length_overflow_error(detail: impl Into<String>) -> CodecError {
    CodecError::new(code::LENGTH_OVERFLOW, detail.into())
}

fn byte_extent_error(got: u64, declared: u64) -> CodecError {
    CodecError::new(
        code::BYTE_EXTENT,
        format!("字节序列实得 {got} 字节，声明 {declared}"),
    )
}

fn is_sequence_tag(t: u8) -> bool {
    matches!(t, tag::SEQ_EMPTY | tag::SEQ_LEAF | tag::SEQ_BRANCH)
}

fn is_internal_tag(t: u8) -> bool {
    matches!(
        t,
        tag::BYTES_CHUNK | tag::SEQ_EMPTY | tag::SEQ_LEAF | tag::SEQ_BRANCH | tag::OBJECT_FIELD
    )
}

fn unknown_tag_error(t: u8) -> CodecError {
    CodecError::new(code::UNKNOWN_TAG, format!("未知标签 tag={t:02x}"))
}

/// 逻辑值位读到非值标签：已知内部节点 → `value_kind`；未登记标签 → `unknown_tag`。
fn unsupported_value_tag_error(t: u8) -> CodecError {
    if is_internal_tag(t) {
        value_kind_error(t)
    } else {
        unknown_tag_error(t)
    }
}

fn tag_label(t: u8) -> &'static str {
    match t {
        tag::NULL => "null",
        tag::BOOL_FALSE => "bool/false",
        tag::BOOL_TRUE => "bool/true",
        tag::STRING => "string",
        tag::NUMBER => "number",
        tag::ARRAY => "array",
        tag::OBJECT => "object",
        tag::JSON_TEXT => "json_text",
        tag::BATCH_SELF => "template_batch",
        tag::BYTES_CHUNK => "bytes_chunk",
        tag::SEQ_EMPTY => "seq_empty",
        tag::SEQ_LEAF => "seq_leaf",
        tag::SEQ_BRANCH => "seq_branch",
        tag::OBJECT_FIELD => "object_field",
        _ => "unknown",
    }
}

/// 读节点并核摘要：缺失与「摘要与 (tag, body) 重算不符」都是硬拒绝。
fn read_checked(
    source: &mut dyn NodeSource,
    digest: &NodeDigest,
) -> Result<(u8, Vec<u8>), CodecError> {
    match source.checked_read(digest)? {
        None => Err(CodecError::new(
            code::NODE_MISSING,
            format!("节点缺失：{}", root_string(digest)),
        )),
        Some(checked) => {
            if checked.digest() != digest {
                return Err(CodecError::new(
                    code::NODE_DIGEST,
                    format!(
                        "已核节点摘要与本次请求不符：请求 {}，实算 {}",
                        root_string(digest),
                        root_string(checked.digest())
                    ),
                ));
            }
            Ok(checked.into_parts())
        }
    }
}

fn read_scoped_checked<'a>(
    source: &mut dyn NodeSource,
    digest: &NodeDigest,
) -> Result<(u8, VerifierBody<'a>), CodecError> {
    let frame = source.checked_read_scoped(digest)?.ok_or_else(|| {
        CodecError::new(
            code::NODE_MISSING,
            format!("节点缺失：{}", root_string(digest)),
        )
    })?;
    if frame.digest() != digest {
        return Err(CodecError::new(
            code::NODE_DIGEST,
            format!(
                "已核节点摘要与本次请求不符：请求 {}，实算 {}",
                root_string(digest),
                root_string(frame.digest())
            ),
        ));
    }
    let (tag, body) = frame.into_body();
    Ok((tag, VerifierBody::Scoped(body)))
}

/// U64 帧长度（字符串/数字的字节数、数组/对象的分量数）。长度契约集中在这里。
fn framed_len(body: &[u8], what: &str) -> Result<u64, CodecError> {
    if body.len() != 40 {
        return Err(CodecError::new(
            code::BODY_SHAPE,
            format!(
                "{what} 的 body 长度 {} ≠ 40（U64 长度 + 32 字节子摘要）",
                body.len()
            ),
        ));
    }
    let n = be_u64(body);
    if n > MAX_FRAME_LEN {
        return Err(length_overflow_error(format!(
            "{what} 声明长度 {n} 超过 2^63-1"
        )));
    }
    Ok(n)
}

/// 该节点自身的 body framing 与 extent。只描述单节点，不核闭包。
fn header_extent(t: u8, body: &[u8]) -> Result<Option<u64>, CodecError> {
    match t {
        tag::NULL | tag::BOOL_FALSE | tag::BOOL_TRUE | tag::BATCH_SELF => {
            if !body.is_empty() {
                return Err(body_shape_error(t, body, "该标签 body 必须为空"));
            }
            Ok(None)
        }
        tag::JSON_TEXT => {
            if body.len() != 32 {
                return Err(body_shape_error(t, body, "body 必须是内部值的 32 字节摘要"));
            }
            Ok(None)
        }
        tag::STRING | tag::NUMBER | tag::ARRAY | tag::OBJECT => {
            Ok(Some(framed_len(body, tag_label(t))?))
        }
        _ => Err(unsupported_value_tag_error(t)),
    }
}

// ── 有限状态：UTF-8 与 JSON 数字文法（跨块推进，可组合） ─────────────────────

/// UTF-8 解码状态：空 = 可接受的边界；Pending = 当前字符已收集的字节。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Utf8State {
    Empty,
    Pending { len: u8, b: [u8; 3] },
}

fn utf8_step(state: Utf8State, byte: u8) -> Option<Utf8State> {
    match state {
        Utf8State::Empty => match byte {
            0x00..=0x7f => Some(Utf8State::Empty),
            0xc2..=0xdf | 0xe0..=0xef | 0xf0..=0xf4 => Some(Utf8State::Pending {
                len: 1,
                b: [byte, 0, 0],
            }),
            _ => None,
        },
        Utf8State::Pending { len, b } => {
            let need: u8 = match b[0] {
                0xc2..=0xdf => 2,
                0xe0..=0xef => 3,
                _ => 4,
            };
            let idx = len as usize;
            let ok = match (idx, b[0]) {
                (1, 0xe0) => (0xa0..=0xbf).contains(&byte),
                (1, 0xed) => (0x80..=0x9f).contains(&byte),
                (1, 0xf0) => (0x90..=0xbf).contains(&byte),
                (1, 0xf4) => (0x80..=0x8f).contains(&byte),
                _ => (0x80..=0xbf).contains(&byte),
            };
            if !ok {
                return None;
            }
            if len + 1 == need {
                return Some(Utf8State::Empty);
            }
            let mut next = b;
            next[idx.min(2)] = byte;
            Some(Utf8State::Pending {
                len: len + 1,
                b: next,
            })
        }
    }
}

fn utf8_feed(state: Utf8State, bytes: &[u8]) -> Option<Utf8State> {
    let mut s = state;
    for &b in bytes {
        s = utf8_step(s, b)?;
    }
    Some(s)
}

/// JSON 数字文法（RFC 8259）状态。`Dead` 一旦进入不可恢复。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum NumState {
    Start,
    Sign,
    Zero,
    Int,
    DotZero,
    DotInt,
    Frac,
    Exp,
    ExpSign,
    ExpDigits,
    Dead,
}

fn num_step(state: NumState, byte: u8) -> NumState {
    use NumState::*;
    match (state, byte) {
        (Start, b'-') => Sign,
        (Start, b'0') | (Sign, b'0') => Zero,
        (Start, b'1'..=b'9') | (Sign, b'1'..=b'9') => Int,
        (Zero, b'.') => DotZero,
        (Zero, b'e') | (Zero, b'E') => Exp,
        (Int, b'0'..=b'9') => Int,
        (Int, b'.') => DotInt,
        (Int, b'e') | (Int, b'E') => Exp,
        (DotZero, b'0'..=b'9') | (DotInt, b'0'..=b'9') => Frac,
        (Frac, b'0'..=b'9') => Frac,
        (Frac, b'e') | (Frac, b'E') => Exp,
        (Exp, b'+') | (Exp, b'-') => ExpSign,
        (Exp, b'0'..=b'9') => ExpDigits,
        (ExpSign, b'0'..=b'9') => ExpDigits,
        (ExpDigits, b'0'..=b'9') => ExpDigits,
        _ => Dead,
    }
}

fn num_feed(state: NumState, bytes: &[u8]) -> NumState {
    bytes.iter().fold(state, |s, &b| num_step(s, b))
}

fn num_accepts(state: NumState) -> bool {
    matches!(
        state,
        NumState::Zero | NumState::Int | NumState::Frac | NumState::ExpDigits
    )
}

/// 字节序列（B）的折叠摘要：逻辑叶数/字节数 + 末叶长度 + 「除末叶外都是 4096」+
/// UTF-8 出态 + 数字文法出态。组合只做常数时间聚合，不展开逻辑叶。
#[derive(Debug, Clone, Copy)]
struct ByteAgg {
    leaves: u64,
    total_bytes: u64,
    last_len: usize,
    all_but_last_ok: bool,
    utf8: Utf8State,
    num: NumState,
}

/// 对象字段序列的折叠摘要：计数 + 首/末键摘要（顺序与唯一性在组合处判）。
#[derive(Debug, Clone, Copy)]
struct FieldAgg {
    count: u64,
    first: Option<NodeDigest>,
    last: Option<NodeDigest>,
}

// #1394 本次不可变 capture 原件直接借用；overlay 的副本持有独立布局。
// 普通源保持 Vec 所有权，不提供 into_vec 或 Clone。
enum VerifierBody<'capture> {
    Borrowed(&'capture [u8]),
    Plain(Vec<u8>),
    Metered(capture_allocation::body::MeteredBody),
    Scoped(job::ScopedBody),
}
impl VerifierBody<'_> {
    fn empty() -> Self {
        Self::Plain(Vec::new())
    }
    fn clear(&mut self) {
        match self {
            Self::Plain(v) => v.clear(),
            Self::Borrowed(_) | Self::Metered(_) | Self::Scoped(_) => *self = Self::empty(),
        }
    }
}
impl std::ops::Deref for VerifierBody<'_> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match self {
            Self::Plain(v) => v,
            Self::Borrowed(v) => v,
            Self::Metered(v) => v,
            Self::Scoped(v) => v,
        }
    }
}
// #1394 只在靶向测试启用的实际捕获读取/F顺序仪表，默认无Vec且不影响准入。
#[cfg(test)]
thread_local! {
    static CAPTURE_BODY_READ_TRACE: std::cell::RefCell<Option<(u64,Vec<(NodeDigest,u64)>)>> = const { std::cell::RefCell::new(None) };
}
#[cfg(test)]
fn trace_capture_body_read(d: &NodeDigest) {
    CAPTURE_BODY_READ_TRACE.with(|trace| {
        if let Some((start, rows)) = trace.borrow_mut().as_mut() {
            rows.push((*d, frame_calls_on_thread() - *start));
        }
    });
}
fn read_body_checked<'capture>(
    source: &mut dyn NodeSource,
    capture: Option<&'capture AuditCapture>,
    d: &NodeDigest,
) -> Result<(u8, VerifierBody<'capture>), CodecError> {
    // 具体 CaptureReader 私有构造器是唯一入口；动态 NodeSource 无法提供或覆盖 capture。
    let Some(capture) = capture else {
        return read_checked(source, d).map(|(t, b)| (t, VerifierBody::Plain(b)));
    };
    let CaptureNodes::AllocationPrototype(_) = &capture.nodes else {
        unreachable!()
    };
    #[cfg(test)]
    trace_capture_body_read(d);
    capture.reads.set(capture.reads.get() + 1);
    let (t, bytes) = capture.original(d).ok_or_else(|| {
        CodecError::new(code::NODE_MISSING, format!("节点缺失：{}", root_string(d)))
    })?;
    let body = VerifierBody::Borrowed(bytes);
    // 实际原件完整 F 校验，不复用外部成功、不吞读取错误。
    let actual = frame(t, &body);
    if actual != *d {
        return Err(CodecError::new(
            code::NODE_DIGEST,
            format!(
                "节点 (tag, body) 与摘要不符：声称 {}，实算 {}；未核摘要不成帧",
                root_string(d),
                root_string(&actual)
            ),
        ));
    }
    Ok((t, body))
}

// ── 惰性叶迭代（渲染与键比较用；不预收集整棵序列） ──────────────────────────

/// 顺序产出序列树的叶：只需 O(树深) 栈，按消费推进。
#[derive(Debug)]
struct LeafIter {
    stack: WorkVec<NodeDigest>,
}

impl LeafIter {
    fn new(root: NodeDigest, budget: Option<&WorkBudget>) -> Result<Self, CodecError> {
        let mut stack = WorkVec::with_work_capacity(1, budget)?;
        stack.push(root)?;
        Ok(Self { stack })
    }

    /// 下一个叶（tag, 摘要, body）；叶的类型由调用方按上下文核。
    #[allow(
        dead_code,
        reason = "#1608：LeafIter 便捷入口，现有调用方均直接用 next_with"
    )]
    fn next(
        &mut self,
        source: &mut dyn NodeSource,
    ) -> Result<Option<(u8, NodeDigest, Vec<u8>)>, CodecError> {
        self.next_with(|d| read_checked(source, d))
    }
    #[allow(
        dead_code,
        reason = "#1608：LeafIter 便捷入口，现有调用方均直接用 next_with"
    )]
    fn next_body<'capture>(
        &mut self,
        source: &mut dyn NodeSource,
        capture: Option<&'capture AuditCapture>,
    ) -> Result<Option<(u8, NodeDigest, VerifierBody<'capture>)>, CodecError> {
        self.next_with(|d| read_body_checked(source, capture, d))
    }
    fn next_context<'capture>(
        &mut self,
        source: &mut CodecContext<'_, 'capture>,
    ) -> Result<Option<(u8, NodeDigest, VerifierBody<'capture>)>, CodecError> {
        self.next_with(|d| source.read(d))
    }
    fn next_with<B: std::ops::Deref<Target = [u8]>>(
        &mut self,
        mut read: impl FnMut(&NodeDigest) -> Result<(u8, B), CodecError>,
    ) -> Result<Option<(u8, NodeDigest, B)>, CodecError> {
        while let Some(digest) = self.stack.pop() {
            let (t, body) = read(&digest)?;
            match t {
                tag::SEQ_EMPTY => {
                    if !body.is_empty() {
                        return Err(body_shape_error(t, &body, "空序列 body 必须为空"));
                    }
                }
                tag::SEQ_LEAF => {
                    if body.len() != 32 {
                        return Err(sequence_shape_error("单叶序列 body 必须是 32 字节叶摘要"));
                    }
                    let leaf = digest_at(&body, 0);
                    let (lt, lbody) = read(&leaf)?;
                    if is_sequence_tag(lt) {
                        return Err(sequence_shape_error(format!(
                            "单叶序列的叶不能又是序列节点（tag={lt:02x}）"
                        )));
                    }
                    return Ok(Some((lt, leaf, lbody)));
                }
                tag::SEQ_BRANCH => {
                    if body.len() != 72 {
                        return Err(sequence_shape_error(
                            "分支序列 body 必须是 U64(n) + 左右子摘要（72 字节）",
                        ));
                    }
                    let left = digest_at(&body, 8);
                    let right = digest_at(&body, 40);
                    self.stack.push(right)?;
                    self.stack.push(left)?;
                }
                other => {
                    return Err(sequence_shape_error(format!(
                        "序列位置读到非序列节点 tag={other:02x}"
                    )))
                }
            }
        }
        Ok(None)
    }
}

// ── 核验器：值节点去重 + 序列摘要折叠 ────────────────────────────────────────

#[derive(Default)]
struct AuditMemo {
    budget: Option<WorkBudget>,
    /// 已核过的 (摘要, 上下文)。
    seen: WorkSet<(NodeDigest, Flavor)>,
    /// 已入待核集合的 (摘要, 上下文) —— 重复子树不重复入队。
    pushed: WorkSet<(NodeDigest, Flavor)>,
    todo: WorkVec<(NodeDigest, Flavor)>,
    /// 实际读到的不同摘要（唯一节点计数）。
    read_digests: WorkSet<NodeDigest>,
    bytes_memo: WorkMap<(NodeDigest, Utf8State, NumState), ByteAgg>,
    elem_memo: WorkMap<(NodeDigest, Flavor), u64>,
    field_memo: WorkMap<(NodeDigest, Flavor), FieldAgg>,
    key_cmp: WorkMap<(NodeDigest, NodeDigest), std::cmp::Ordering>,
    native_seen_permit: Option<capture_allocation::scratch::NativeReservation>,
}

impl AuditMemo {
    fn new(b: Option<&capture_allocation::OwnedBudget>) -> Self {
        Self::new_work(b.map(|b| WorkBudget::Owned(b.clone())).as_ref())
    }
    fn new_work(b: Option<&WorkBudget>) -> Self {
        Self {
            budget: b.cloned(),
            seen: WorkSet::new_work(b),
            pushed: WorkSet::new_work(b),
            todo: WorkVec::new_work(b),
            read_digests: WorkSet::new_work(b),
            bytes_memo: WorkMap::new_work(b),
            elem_memo: WorkMap::new_work(b),
            field_memo: WorkMap::new_work(b),
            key_cmp: WorkMap::new_work(b),
            native_seen_permit: None,
        }
    }
}
/// #1394 算法原型：完整自有节点捕获，无 NodeSink、无可变原件入口。
/// 仅复用本捕获内 Public/Template 独立验证上下文，不授予来源或业务资格。
enum CaptureNodes {
    Legacy(BTreeMap<NodeDigest, (u8, Vec<u8>)>),
    #[cfg(feature = "s_capture_native_testonly")]
    Native(NativeCaptureNodes),
    AllocationPrototype(capture_allocation::CaptureSlab),
}
// #1394 Native TestOnly：原件与纯 F 证明共用完整摘要索引。
// #1488 R 正文可持原 CodecVec 许可；旧 Vec 入口仍保持其既有来源计费合同。
#[cfg(feature = "s_capture_native_testonly")]
struct NativeNode {
    tag: u8,
    body: CodecVec<u8>,
    source_claim: Option<SourceClaim>,
    // 成功后才分配证明；未读节点不常驻完整 CheckedFrame。
    proof:
        std::cell::RefCell<Option<capture_allocation::native_owned::NativeBox<NativeFrameProof>>>,
    // owner热写清理链复用节点inline，不另分配全nodes列表。
    owner_dirty: std::cell::Cell<bool>,
    owner_next: std::cell::Cell<Option<NodeDigest>>,
}
#[cfg(feature = "s_capture_native_testonly")]
impl NativeNode {
    #[cfg_attr(not(test), allow(dead_code))]
    fn new(tag: u8, body: Vec<u8>) -> Self {
        Self::new_codec(tag, CodecVec::from_plain(body))
    }
    fn new_codec(tag: u8, body: CodecVec<u8>) -> Self {
        Self {
            tag,
            body,
            source_claim: None,
            proof: Default::default(),
            owner_dirty: Default::default(),
            owner_next: Default::default(),
        }
    }
}
/// #1488 P2-A（主控裁定）：S7来源合同角色（正文+digest/tag/证书=body.len()+97），只进E/O合同账，
/// 不在父账物理reserve；父账物理只由正文块自身的Native许可计一次。只能对同一节点内已物理计费
/// （Native绑定）的正文块签出，指不到即拒。原先这里是一张同额物理许可（r14：803张/111,334B），
/// 与正文块许可对同一allocation双计。
#[cfg(feature = "s_capture_native_testonly")]
#[derive(Debug)]
struct SourceClaim {
    bytes: usize,
}
#[cfg(feature = "s_capture_native_testonly")]
impl SourceClaim {
    fn on_body(body: &CodecVec<u8>) -> Result<Self, CodecError> {
        if !body.is_native_bound() {
            return Err(CodecError::new(
                code::SOURCE,
                "来源合同项指不到已物理计费的正文块",
            ));
        }
        Self::on_len(body.len())
    }

    fn on_len(len: usize) -> Result<Self, CodecError> {
        let bytes = len
            .checked_add(97)
            .ok_or_else(|| CodecError::new(code::SOURCE, "来源合同额溢出"))?;
        Ok(Self { bytes })
    }
}
/// #1572：capture内一张Native许可所保护的载荷（持有者位置）。
#[cfg(feature = "s_capture_native_testonly")]
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapturePermitRole {
    OwnerCodec,
    NodeProof,
    MemoSeen,
    NodeIndex,
    NodeBody,
    CodecRoots,
    OriginalFeeBodies,
    OriginalFeeBody,
    OriginalFeeTable,
    OriginalFeeTableEntry,
}
/// #1572：逐张物理许可（身份、字节、所保护载荷、所属父账与来源域）。只由 capture 的持有者清单
/// 产出，借用该 capture：身份是地址，只在同一静止点有效，不得跨借用保存。正文许可另带所属节点
/// 摘要与正文长度，供来源合同对账按 backing 独立推额。
#[cfg(feature = "s_capture_native_testonly")]
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapturePermitEntry<'a> {
    id: capture_allocation::scratch::PermitId,
    bytes: usize,
    role: CapturePermitRole,
    ledger: usize,
    origin: u64,
    body: Option<(NodeDigest, usize)>,
    _capture: std::marker::PhantomData<&'a AuditCapture>,
}
#[cfg(feature = "s_capture_native_testonly")]
impl CapturePermitEntry<'_> {
    fn held(
        permit: &capture_allocation::scratch::NativeReservation,
        role: CapturePermitRole,
        body: Option<(NodeDigest, usize)>,
    ) -> Self {
        Self {
            id: permit.permit_id(),
            bytes: permit.bytes(),
            role,
            ledger: permit.ledger_id(),
            origin: permit.origin(),
            body,
            _capture: std::marker::PhantomData,
        }
    }
    pub fn id(&self) -> capture_allocation::scratch::PermitId {
        self.id
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
    pub fn role(&self) -> CapturePermitRole {
        self.role
    }
    pub fn ledger(&self) -> usize {
        self.ledger
    }
    pub fn origin(&self) -> u64 {
        self.origin
    }
    /// 正文许可所属节点摘要与正文长度；非正文许可为 None。
    pub fn body(&self) -> Option<(NodeDigest, usize)> {
        self.body
    }
    /// 负控注入：改记来源域。
    #[cfg(test)]
    fn with_origin_for_test(mut self, origin: u64) -> Self {
        self.origin = origin;
        self
    }
}
/// #1572：一项来源合同（#1488 P2-A SourceClaim）：只记合同额，物理由 `backing` 所指的
/// 正文许可承担；它自身不对应任何物理许可。`site` 是正文所在位置（节点或未转换原件费），
/// `node` 是持有者生成时记下的节点摘要。只由持有者产出，借用该 capture。
#[cfg(feature = "s_capture_native_testonly")]
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceClaimEntry<'a> {
    amount: usize,
    backing: capture_allocation::scratch::PermitId,
    site: CapturePermitRole,
    node: NodeDigest,
    _capture: std::marker::PhantomData<&'a AuditCapture>,
}
#[cfg(feature = "s_capture_native_testonly")]
impl SourceClaimEntry<'_> {
    fn new(
        amount: usize,
        backing: capture_allocation::scratch::PermitId,
        site: CapturePermitRole,
        node: NodeDigest,
    ) -> Self {
        Self {
            amount,
            backing,
            site,
            node,
            _capture: std::marker::PhantomData,
        }
    }
    pub fn amount(&self) -> usize {
        self.amount
    }
    pub fn backing(&self) -> capture_allocation::scratch::PermitId {
        self.backing
    }
    pub fn site(&self) -> CapturePermitRole {
        self.site
    }
    pub fn node(&self) -> NodeDigest {
        self.node
    }
    /// 负控注入：改指 backing。
    #[cfg(test)]
    fn with_backing_for_test(mut self, backing: capture_allocation::scratch::PermitId) -> Self {
        self.backing = backing;
        self
    }
    /// 负控注入：改合同额。
    #[cfg(test)]
    fn with_amount_for_test(mut self, amount: usize) -> Self {
        self.amount = amount;
        self
    }
}
/// #1572：来源合同与物理清单的对账结果。
#[cfg(feature = "s_capture_native_testonly")]
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceClaimReconciliation {
    pub claims: usize,
    pub claim_amount: usize,
    pub backing_bytes: usize,
}
#[cfg(feature = "s_capture_native_testonly")]
struct NativeCaptureNodes {
    // 标准 RandomState；未核摘要不使用弱哈希。
    nodes: std::collections::HashMap<NodeDigest, NativeNode>,
    index_permit: Option<capture_allocation::scratch::NativeReservation>,
    body_len: usize,
    body_capacity: usize,
    owner_dirty_head: std::cell::Cell<Option<NodeDigest>>,
    memo: std::cell::RefCell<NativeFrameMemo>,
    #[cfg(test)]
    index_lookups: std::cell::Cell<u64>,
}
#[cfg(feature = "s_capture_native_testonly")]
impl NativeCaptureNodes {
    fn get(&self, digest: &NodeDigest) -> Option<&NativeNode> {
        #[cfg(test)]
        self.index_lookups
            .set(self.index_lookups.get().saturating_add(1));
        // 计数紧贴实际容器访问；原件与 proof 都只能从此次返回的 entry 取。
        self.nodes.get(digest)
    }
}
#[cfg(feature = "s_capture_native_testonly")]
struct NativeFrameProof {
    frame: job::CheckedFrame,
    body_capacity: usize,
}
#[cfg(feature = "s_capture_native_testonly")]
struct NativeFrameMemo {
    entry_limit: usize,
    body_capacity_limit: usize,
    retained_entries: usize,
    retained_body_capacity: usize,
    same_material_hits: u64,
    first_f_calls: u64,
    changed_f_calls: u64,
    strict_f_calls: u64,
    failed_f_calls: u64,
    retention_skips: u64,
}
#[cfg(feature = "s_capture_native_testonly")]
impl NativeFrameMemo {
    fn new(entry_limit: usize, body_capacity_limit: usize) -> Self {
        Self {
            entry_limit,
            body_capacity_limit,
            retained_entries: 0,
            retained_body_capacity: 0,
            same_material_hits: 0,
            first_f_calls: 0,
            changed_f_calls: 0,
            strict_f_calls: 0,
            failed_f_calls: 0,
            retention_skips: 0,
        }
    }
    fn remove(
        &mut self,
        proof: &mut Option<capture_allocation::native_owned::NativeBox<NativeFrameProof>>,
    ) {
        if let Some(old) = proof.take() {
            self.retained_body_capacity = self
                .retained_body_capacity
                .checked_sub(old.body_capacity)
                .expect("Native retained body capacity invariant");
            self.retained_entries = self
                .retained_entries
                .checked_sub(1)
                .expect("Native retained entry invariant");
        }
    }
    fn retain(
        &mut self,
        slot: &mut Option<capture_allocation::native_owned::NativeBox<NativeFrameProof>>,
        frame: &job::CheckedFrame,
    ) {
        // 先检查本次原件长度；复制后以 Vec 的实际 capacity 再核保留资格。
        let remaining = self
            .body_capacity_limit
            .saturating_sub(self.retained_body_capacity);
        if self.retained_entries >= self.entry_limit || frame.body().len() > remaining {
            self.retention_skips = self.retention_skips.saturating_add(1);
            return;
        }
        let mut body = Vec::new();
        if body.try_reserve_exact(frame.body().len()).is_err() {
            self.retention_skips = self.retention_skips.saturating_add(1);
            return;
        }
        body.extend_from_slice(frame.body());
        self.retain_copy(slot, frame, body);
    }
    fn retain_copy(
        &mut self,
        slot: &mut Option<capture_allocation::native_owned::NativeBox<NativeFrameProof>>,
        frame: &job::CheckedFrame,
        body: Vec<u8>,
    ) {
        let capacity = body.capacity();
        let Some(retained) = self.retained_body_capacity.checked_add(capacity) else {
            self.retention_skips = self.retention_skips.saturating_add(1);
            return;
        };
        let Some(entries) = self.retained_entries.checked_add(1) else {
            self.retention_skips = self.retention_skips.saturating_add(1);
            return;
        };
        if slot.is_some() || entries > self.entry_limit || retained > self.body_capacity_limit {
            self.retention_skips = self.retention_skips.saturating_add(1);
            return;
        }
        if let job::SameMaterialRebind::Same(proof) =
            frame.try_rebind_same_material(frame.digest(), frame.tag(), body)
        {
            let Ok(proof) = allocator_api2::boxed::Box::try_new(NativeFrameProof {
                frame: proof,
                body_capacity: capacity,
            }) else {
                self.retention_skips = self.retention_skips.saturating_add(1);
                return;
            };
            *slot = Some(capture_allocation::native_owned::NativeBox::from_reserved(
                proof, None,
            ));
            self.retained_entries = entries;
            self.retained_body_capacity = retained;
        }
    }
    fn checked_current(
        &mut self,
        digest: &NodeDigest,
        node: Option<&NativeNode>,
        current: Result<NodeRead, CodecError>,
    ) -> Result<Option<job::CheckedFrame>, CodecError> {
        let (tag, mut body) = match current {
            Ok(NodeRead::Found { tag, body }) => (tag, body),
            Ok(NodeRead::Missing) => {
                if let Some(node) = node {
                    self.remove(&mut node.proof.borrow_mut());
                }
                return Ok(None);
            }
            Err(error) => {
                if let Some(node) = node {
                    self.remove(&mut node.proof.borrow_mut());
                }
                return Err(error);
            }
        };
        let node = node.expect("Native Found must come from the same index entry");
        let mut proof = node.proof.borrow_mut();
        let changed = if let Some(old) = proof.as_ref() {
            match old.frame.try_rebind_same_material(digest, tag, body) {
                job::SameMaterialRebind::Same(frame) => {
                    self.same_material_hits = self.same_material_hits.saturating_add(1);
                    return Ok(Some(frame));
                }
                job::SameMaterialRebind::Changed(current) => body = current,
            }
            self.remove(&mut proof);
            true
        } else {
            false
        };
        self.strict_f_calls = self.strict_f_calls.saturating_add(1);
        if changed {
            self.changed_f_calls = self.changed_f_calls.saturating_add(1);
        } else {
            self.first_f_calls = self.first_f_calls.saturating_add(1);
        }
        let frame = match job::CheckedFrame::checked(digest, tag, body) {
            Ok(frame) => frame,
            Err(error) => {
                self.failed_f_calls = self.failed_f_calls.saturating_add(1);
                return Err(error);
            }
        };
        self.retain(&mut proof, &frame);
        Ok(Some(frame))
    }
}
// #1394 私有成功根证书。修改codec判据时须同步递增版本；不序列化、不接受外来构造。
// format与版本来自当前codec；集合分别固定Mode，所有根只对应Flavor::Value。
#[cfg(feature = "s_capture_native_testonly")]
struct NativeCodecRootCertificates {
    format: &'static str,
    predicate_revision: u32,
    flavor: Flavor,
    public: std::collections::HashSet<NodeDigest>,
    template: std::collections::HashSet<NodeDigest>,
    public_permit: Option<capture_allocation::scratch::NativeReservation>,
}
#[cfg(feature = "s_capture_native_testonly")]
impl Default for NativeCodecRootCertificates {
    fn default() -> Self {
        Self {
            format: CONTENT_FORMAT,
            predicate_revision: 1,
            flavor: Flavor::Value,
            public: Default::default(),
            template: Default::default(),
            public_permit: None,
        }
    }
}
// #1488 成功证明用确定性 AVL 索引：每次只沿索引路径/旋转，无历史整表重哈希。
// key/value 均为固定 Copy tuple；节点 Layout 含左右 Box、height、padding，无带外 heap。
#[cfg(feature = "s_capture_native_testonly")]
type OwnerProofLink<K, V> =
    Option<capture_allocation::native_owned::NativeBox<OwnerProofNode<K, V>>>;
#[cfg(feature = "s_capture_native_testonly")]
struct OwnerProofNode<K, V> {
    key: K,
    value: V,
    left: OwnerProofLink<K, V>,
    right: OwnerProofLink<K, V>,
    height: usize,
}
#[cfg(feature = "s_capture_native_testonly")]
impl<K, V> OwnerProofNode<K, V> {
    fn height(link: &OwnerProofLink<K, V>) -> usize {
        link.as_ref().map_or(0, |node| node.height)
    }
    fn refresh_height(&mut self) {
        self.height = Self::height(&self.left).max(Self::height(&self.right)) + 1;
    }
    fn balance(&self) -> isize {
        Self::height(&self.left) as isize - Self::height(&self.right) as isize
    }
}
#[cfg(feature = "s_capture_native_testonly")]
struct OwnerProofTree<K, V> {
    root: OwnerProofLink<K, V>,
    len: usize,
    retained_bytes: usize,
}
#[cfg(feature = "s_capture_native_testonly")]
impl<K, V> Default for OwnerProofTree<K, V> {
    fn default() -> Self {
        Self {
            root: None,
            len: 0,
            retained_bytes: 0,
        }
    }
}
#[cfg(all(test, feature = "s_capture_native_testonly"))]
thread_local! {
    // 比较、旋转、新节点数；不参与资格，不由生产输入设置。
    static OWNER_CODEC_INDEX_TRACE: std::cell::Cell<(usize, usize, usize)> = const { std::cell::Cell::new((0, 0, 0)) };
}
#[cfg(feature = "s_capture_native_testonly")]
impl<K: Ord + Copy, V: Copy> OwnerProofTree<K, V> {
    fn get(&self, key: &K) -> Option<&V> {
        let mut link = &self.root;
        while let Some(node) = link {
            #[cfg(test)]
            OWNER_CODEC_INDEX_TRACE.with(|p| {
                let (a, b, c) = p.get();
                p.set((a + 1, b, c));
            });
            match key.cmp(&node.key) {
                std::cmp::Ordering::Less => link = &node.left,
                std::cmp::Ordering::Greater => link = &node.right,
                std::cmp::Ordering::Equal => return Some(&node.value),
            }
        }
        None
    }
    fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }
    #[cfg_attr(not(test), allow(dead_code))]
    fn len(&self) -> usize {
        self.len
    }
    fn retained_layout_bytes(&self) -> usize {
        self.retained_bytes
    }
    /// #1572：逐节点列出本树持有的Native许可（AVL节点盒）。
    fn permit_entries<'a>(&self, out: &mut Vec<CapturePermitEntry<'a>>) {
        let mut stack: Vec<&OwnerProofLink<K, V>> = vec![&self.root];
        while let Some(link) = stack.pop() {
            if let Some(node) = link {
                if let Some(p) = node.permit() {
                    out.push(CapturePermitEntry::held(
                        p,
                        CapturePermitRole::OwnerCodec,
                        None,
                    ));
                }
                stack.push(&node.left);
                stack.push(&node.right);
            }
        }
    }

    fn insert(
        &mut self,
        key: K,
        value: V,
        work: &capture_allocation::scratch::WorkScope,
    ) -> Result<(), CodecError> {
        // 重复键不分配、不改已证明的值，也不在额度已满时虚构新增长。
        if self.contains_key(&key) {
            return Ok(());
        }
        let layout = std::alloc::Layout::new::<OwnerProofNode<K, V>>();
        let overflow = || work.reject(capture_allocation::scratch::ScratchFailure::Budget);
        let next_len = self.len.checked_add(1).ok_or_else(overflow)?;
        let next_bytes = self
            .retained_bytes
            .checked_add(layout.size())
            .ok_or_else(overflow)?;
        Self::insert_absent(&mut self.root, key, value, work)?;
        self.len = next_len;
        self.retained_bytes = next_bytes;
        Ok(())
    }

    fn insert_absent(
        link: &mut OwnerProofLink<K, V>,
        key: K,
        value: V,
        work: &capture_allocation::scratch::WorkScope,
    ) -> Result<(), CodecError> {
        let Some(node) = link.as_mut() else {
            let layout = std::alloc::Layout::new::<OwnerProofNode<K, V>>();
            let permit = work.reserve_native(layout.size())?;
            let node = allocator_api2::boxed::Box::try_new(OwnerProofNode {
                key,
                value,
                left: None,
                right: None,
                height: 1,
            })
            .map_err(|_| work.reject(capture_allocation::scratch::ScratchFailure::Allocation))?;
            // 此后只有不可失败的指针旋转；许可仍保守留本轮原父账，不带进新 scope。
            *link = Some(capture_allocation::native_owned::NativeBox::from_reserved(
                node,
                Some(permit),
            ));
            #[cfg(test)]
            OWNER_CODEC_INDEX_TRACE.with(|p| {
                let (a, b, c) = p.get();
                p.set((a, b, c + 1));
            });
            return Ok(());
        };
        #[cfg(test)]
        OWNER_CODEC_INDEX_TRACE.with(|p| {
            let (a, b, c) = p.get();
            p.set((a + 1, b, c));
        });
        match key.cmp(&node.key) {
            std::cmp::Ordering::Less => Self::insert_absent(&mut node.left, key, value, work)?,
            std::cmp::Ordering::Greater => Self::insert_absent(&mut node.right, key, value, work)?,
            std::cmp::Ordering::Equal => unreachable!("同一私有树已排除重复键"),
        }
        // 叶分配失败在此前直接返回，未修改任何祖先链接/高度。
        Self::rebalance(link);
        Ok(())
    }
    fn rebalance(link: &mut OwnerProofLink<K, V>) {
        let node = link.as_mut().expect("插入祖先存在");
        node.refresh_height();
        let balance = node.balance();
        if balance > 1 {
            if node.left.as_ref().expect("AVL左重必有左子").balance() < 0 {
                Self::rotate_left(&mut node.left);
            }
            Self::rotate_right(link);
        } else if balance < -1 {
            if node.right.as_ref().expect("AVL右重必有右子").balance() > 0 {
                Self::rotate_right(&mut node.right);
            }
            Self::rotate_left(link);
        }
    }
    fn rotate_left(link: &mut OwnerProofLink<K, V>) {
        let mut old = link.take().expect("旋转根存在");
        let mut next = old.right.take().expect("左旋右子存在");
        old.right = next.left.take();
        old.refresh_height();
        next.left = Some(old);
        next.refresh_height();
        *link = Some(next);
        #[cfg(test)]
        OWNER_CODEC_INDEX_TRACE.with(|p| {
            let (a, b, c) = p.get();
            p.set((a, b + 1, c));
        });
    }
    fn rotate_right(link: &mut OwnerProofLink<K, V>) {
        let mut old = link.take().expect("旋转根存在");
        let mut next = old.left.take().expect("右旋左子存在");
        old.left = next.right.take();
        old.refresh_height();
        next.right = Some(old);
        next.refresh_height();
        *link = Some(next);
        #[cfg(test)]
        OWNER_CODEC_INDEX_TRACE.with(|p| {
            let (a, b, c) = p.get();
            p.set((a, b + 1, c));
        });
    }
}

// #1488 封闭 owner 的跨操作纯 codec 证明。与 Value 根证书及 AuditMemo 分离：
// 不持 WorkBudget，不存 pushed/todo；只在整根完整成功后接纳本轮 delta。
// 表键保存完整解释上下文；Public/Template 两套表从不互借。
#[cfg(feature = "s_capture_native_testonly")]
#[derive(Default)]
struct NativeOwnerCodecMemo {
    seen: OwnerProofTree<(NodeDigest, Flavor), ()>,
    bytes: OwnerProofTree<(NodeDigest, Utf8State, NumState), ByteAgg>,
    elems: OwnerProofTree<(NodeDigest, Flavor), u64>,
    fields: OwnerProofTree<(NodeDigest, Flavor), FieldAgg>,
    key_cmp: OwnerProofTree<(NodeDigest, NodeDigest), std::cmp::Ordering>,
}
#[cfg(feature = "s_capture_native_testonly")]
impl NativeOwnerCodecMemo {
    fn permit_entries<'a>(&self, out: &mut Vec<CapturePermitEntry<'a>>) {
        self.seen.permit_entries(out);
        self.bytes.permit_entries(out);
        self.elems.permit_entries(out);
        self.fields.permit_entries(out);
        self.key_cmp.permit_entries(out);
    }
    fn retained_bytes(&self) -> usize {
        // 精确 Box 节点 Layout 随成功插入累计；只取常数张树的计数，不扫描旧证明。
        [
            self.seen.retained_layout_bytes(),
            self.bytes.retained_layout_bytes(),
            self.elems.retained_layout_bytes(),
            self.fields.retained_layout_bytes(),
            self.key_cmp.retained_layout_bytes(),
        ]
        .into_iter()
        .fold(0usize, usize::saturating_add)
    }
    fn merge_success(
        &mut self,
        memo: &AuditMemo,
        work: &capture_allocation::scratch::WorkScope,
    ) -> Result<(), CodecError> {
        // 只枚举当次新增 scratch；每一新成功键走索引路径，不扫描/复制/rehash历史证明。
        // 旧节点、当前 scratch 和新增节点始终同父账共存。
        match &memo.seen {
            WorkSet::Plain(seen) => {
                for &key in seen {
                    self.seen.insert(key, (), work)?;
                }
            }
            WorkSet::Owned(seen) => {
                for &key in seen {
                    self.seen.insert(key, (), work)?;
                }
            }
        }
        for (&key, &value) in memo.bytes_memo.iter() {
            self.bytes.insert(key, value, work)?;
        }
        for (&key, &value) in memo.elem_memo.iter() {
            self.elems.insert(key, value, work)?;
        }
        for (&key, &value) in memo.field_memo.iter() {
            self.fields.insert(key, value, work)?;
        }
        for (&key, &value) in memo.key_cmp.iter() {
            self.key_cmp.insert(key, value, work)?;
        }
        Ok(())
    }
}

#[cfg(feature = "s_capture_native_testonly")]
struct NativeOwnerCodecCertificates {
    format: &'static str,
    predicate_revision: u32,
    revoked: bool,
    public: NativeOwnerCodecMemo,
    template: NativeOwnerCodecMemo,
}
#[cfg(feature = "s_capture_native_testonly")]
impl Default for NativeOwnerCodecCertificates {
    fn default() -> Self {
        Self {
            format: CONTENT_FORMAT,
            predicate_revision: 1,
            revoked: false,
            public: Default::default(),
            template: Default::default(),
        }
    }
}
#[cfg(feature = "s_capture_native_testonly")]
impl NativeOwnerCodecCertificates {
    fn clear(&mut self) {
        self.public = NativeOwnerCodecMemo::default();
        self.template = NativeOwnerCodecMemo::default();
    }
    fn revoke(&mut self) {
        self.clear();
        self.revoked = true;
    }
    fn compatible(&self) -> bool {
        !self.revoked && self.format == CONTENT_FORMAT && self.predicate_revision == 1
    }
}

/// #1394 私有构造：同次全新核验器完成导航与三个完整Public闭包后才生成。
/// 此证书只描述Counts依据；仅整组接纳的三个Value根可封存codec资格，不授予来源业务资格。
#[cfg(feature = "s_capture_native_testonly")]
pub struct NativeCountsSubsetCertificate {
    state: capture_allocation::native_owned::NativeValue<ContentRef>,
    codec_revision: u32,
    dependency_revision: u32,
    roots: [capture_allocation::native_owned::NativeValue<ContentRef>; 3],
}
#[cfg(feature = "s_capture_native_testonly")]
impl NativeCountsSubsetCertificate {
    #[doc(hidden)]
    pub fn source_bytes(&self) -> Result<usize, CodecError> {
        std::iter::once(&self.state)
            .chain(&self.roots)
            .try_fold(8usize, |sum, root| {
                sum.checked_add(8 + root.format.len() + 8 + root.root.len() + 1 + 9)
                    .ok_or_else(|| CodecError::new(code::SOURCE, "Counts闭包证书费用溢出"))
            })
    }
    #[doc(hidden)]
    pub fn string_capacity_bytes(&self) -> usize {
        std::iter::once(&self.state)
            .chain(&self.roots)
            .map(|r| r.format.capacity() + r.root.capacity())
            .sum()
    }
    #[doc(hidden)]
    pub fn matches_state(&self, state: &ContentRef) -> bool {
        self.codec_revision == NativeCodecRootCertificates::default().predicate_revision
            && self.dependency_revision == 1
            && state == self.state.as_ref()
    }
    /// #1488 P3-c：逐张列出证书实际持有的Native许可字节。
    #[doc(hidden)]
    pub fn native_permit_inventory(&self, out: &mut Vec<usize>) {
        out.extend(
            std::iter::once(&self.state)
                .chain(&self.roots)
                .filter_map(|root| root.retained_charge()),
        );
    }
}
/// 诊断计数不充当单组依赖列表；实际持有的闭包证书另逐字计费。
#[cfg(feature = "s_capture_native_testonly")]
#[derive(Default, serde::Serialize)]
pub struct NativeCountsSubsetReport {
    pub attempted_groups: usize,
    pub accepted_groups: usize,
    pub budget_stopped: bool,
    pub rolled_back_nodes: usize,
    pub first_admitted_source_bytes: usize,
    pub last_attempted_source_bytes: usize,
    pub retained_source_bytes: usize,
}
/// Counts封存的Plain容器许可；完整新块在旧块仍存活时先入同父账。
/// Native许可保守留到父作用域结束，codec工作集仍由真实Layout账释放。
#[cfg(feature = "s_capture_native_testonly")]
#[derive(Clone, Copy)]
struct NativeCountsSubsetAdmission<'a>(Option<&'a capture_allocation::scratch::WorkScope>);
#[cfg(feature = "s_capture_native_testonly")]
impl NativeCountsSubsetAdmission<'_> {
    fn failure(&self, failure: capture_allocation::scratch::ScratchFailure) -> CodecError {
        self.0.map_or_else(
            || CodecError::static_detail(code::SOURCE, "Counts封存容量溢出或分配失败"),
            |work| work.reject(failure),
        )
    }
    fn reserve(
        &self,
        bytes: usize,
    ) -> Result<Option<capture_allocation::scratch::NativeReservation>, CodecError> {
        self.0.map(|work| work.reserve_native(bytes)).transpose()
    }
    fn hash_room<K, V>(
        &self,
        len: usize,
        capacity: usize,
        additional: usize,
    ) -> Result<Option<capture_allocation::scratch::NativeReservation>, CodecError> {
        use capture_allocation::scratch::ScratchFailure;
        let needed = len
            .checked_add(additional)
            .ok_or_else(|| self.failure(ScratchFailure::Budget))?;
        if self.0.is_none() || needed <= capacity {
            return Ok(None);
        }
        // 与NativeAdmission同一SwissTable档位：4/8桶留一空槽，其后7/8装载率。
        let buckets = if needed <= 3 {
            4
        } else if needed <= 7 {
            8
        } else {
            needed
                .checked_mul(8)
                .and_then(|n| n.checked_add(6))
                .map(|n| n / 7)
                .and_then(usize::checked_next_power_of_two)
                .ok_or_else(|| self.failure(ScratchFailure::Budget))?
        };
        let bytes = buckets
            .checked_add(16)
            .and_then(|n| n.checked_mul(std::mem::size_of::<(K, V)>() + 1))
            .ok_or_else(|| self.failure(ScratchFailure::Budget))?;
        self.reserve(bytes)
    }
    fn vec_room<T>(&self, values: &mut CodecVec<T>, additional: usize) -> Result<(), CodecError> {
        match self.0 {
            Some(work) => values.try_reserve_native(work, additional),
            None => values.try_reserve(additional),
        }
    }
    fn set_room<T: Eq + std::hash::Hash>(
        &self,
        values: &mut std::collections::HashSet<T>,
        fee: &mut Option<capture_allocation::scratch::NativeReservation>,
        additional: usize,
    ) -> Result<(), CodecError> {
        use capture_allocation::scratch::ScratchFailure;
        if let Some(work) = self.0 {
            work.check_current()?;
            if fee.as_ref().is_some_and(|p| !p.belongs_to_parent(work))
                || (fee.is_none() && values.capacity() != 0)
            {
                return Err(work.reject(ScratchFailure::Ledger));
            }
        } else if fee.is_some() {
            return Err(self.failure(ScratchFailure::Ledger));
        }
        let needed = values
            .len()
            .checked_add(additional)
            .ok_or_else(|| self.failure(ScratchFailure::Budget))?;
        if needed <= values.capacity() {
            return Ok(());
        }
        let permit = self.hash_room::<T, ()>(values.len(), values.capacity(), additional)?;
        let mut replacement = std::collections::HashSet::new();
        replacement
            .try_reserve(needed)
            .map_err(|_| self.failure(ScratchFailure::Allocation))?;
        replacement.extend(values.drain());
        *values = replacement;
        *fee = permit;
        Ok(())
    }
}

#[cfg(feature = "s_capture_native_testonly")]
#[derive(Default)]
struct NativeCountsSubsetLedger {
    digests: std::collections::HashSet<NodeDigest>,
    order: CodecVec<NodeDigest>,
    source_bytes: usize,
    digests_permit: Option<capture_allocation::scratch::NativeReservation>,
}
#[cfg(feature = "s_capture_native_testonly")]
impl NativeCountsSubsetLedger {
    fn record(
        &mut self,
        digest: &NodeDigest,
        capture: &AuditCapture,
        admission: NativeCountsSubsetAdmission<'_>,
    ) -> Result<(), CodecError> {
        if let Some(work) = admission.0 {
            work.check_current()?;
        }
        if self.digests.contains(digest) {
            return Ok(());
        }
        let (_, body) = capture
            .original(digest)
            .ok_or_else(|| CodecError::new(code::SOURCE, "Counts依赖必须来自完整capture原件"))?;
        let bytes = self
            .source_bytes
            .checked_add(body.len())
            .and_then(|v| v.checked_add(32 + 1 + 64))
            .ok_or_else(|| CodecError::new(code::SOURCE, "Counts原件费用溢出"))?;
        admission.set_room(&mut self.digests, &mut self.digests_permit, 1)?;
        admission.vec_room(&mut self.order, 1)?;
        self.digests.insert(*digest);
        self.order.try_push(*digest)?;
        self.source_bytes = bytes;
        Ok(())
    }
    fn rollback(&mut self, checkpoint: (usize, usize)) -> usize {
        let removed = self.order.len() - checkpoint.0;
        while self.order.len() > checkpoint.0 {
            let digest = self.order.pop().expect("已核回滚长度");
            self.digests.remove(&digest);
        }
        self.source_bytes = checkpoint.1;
        removed
    }
}
#[cfg(feature = "s_capture_native_testonly")]
struct NativeCountsSubsetReader<'a> {
    capture: &'a AuditCapture,
    ledger: &'a std::cell::RefCell<NativeCountsSubsetLedger>,
    admission: NativeCountsSubsetAdmission<'a>,
}
#[cfg(feature = "s_capture_native_testonly")]
impl NodeSource for NativeCountsSubsetReader<'_> {
    fn codec_work(&self) -> Option<&capture_allocation::scratch::WorkScope> {
        self.admission.0
    }
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.read_scoped(digest)?.into_plain()
    }
    fn read_scoped(&mut self, digest: &NodeDigest) -> Result<job::ScopedNodeRead, CodecError> {
        let bytes = self
            .capture
            .original(digest)
            .map_or(0, |(_, body)| body.len());
        let permit = self.admission.reserve(bytes)?;
        let got = CaptureReader(self.capture).read(digest)?;
        if matches!(&got, NodeRead::Found { .. }) {
            self.ledger
                .borrow_mut()
                .record(digest, self.capture, self.admission)?;
        }
        Ok(job::ScopedNodeRead::from_plain(got, permit))
    }
    fn checked_read(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<job::CheckedFrame>, CodecError> {
        self.checked_read_scoped(digest)
            .and_then(|frame| frame.map(job::ScopedFrame::into_frame).transpose())
    }
    fn checked_read_scoped(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<job::ScopedFrame>, CodecError> {
        let bytes = self
            .capture
            .original(digest)
            .map_or(0, |(_, body)| body.len());
        let body_permit = self.admission.reserve(bytes)?;
        // CaptureReader的既有F复用第一次成功会复制正文并Box证明；在调用之前精确许可。
        // 原件不可变，已持有proof时走逐字rebind，不再分配proof正文。
        let CaptureNodes::Native(native) = &self.capture.nodes else {
            unreachable!()
        };
        let retains_proof = native.nodes.get(digest).is_some_and(|node| {
            let memo = native.memo.borrow();
            node.proof.borrow().is_none()
                && memo.retained_entries < memo.entry_limit
                && bytes
                    <= memo
                        .body_capacity_limit
                        .saturating_sub(memo.retained_body_capacity)
        });
        let proof_permit = if retains_proof {
            self.admission.reserve(
                bytes
                    .checked_add(std::mem::size_of::<NativeFrameProof>())
                    .ok_or_else(|| {
                        self.admission
                            .failure(capture_allocation::scratch::ScratchFailure::Budget)
                    })?,
            )?
        } else {
            None
        };
        let got = CaptureReader(self.capture).checked_read(digest)?;
        if let Some(permit) = proof_permit {
            if let Some(node) = native.nodes.get(digest) {
                if let Some(proof) = node.proof.borrow_mut().as_mut() {
                    proof.attach(permit);
                }
            }
        }
        if got.is_some() {
            self.ledger
                .borrow_mut()
                .record(digest, self.capture, self.admission)?;
        }
        Ok(got.map(|frame| job::ScopedFrame::new(frame, body_permit)))
    }
    fn audit_full_array_scan(&self) -> bool {
        true
    }
}
#[cfg(feature = "s_capture_native_testonly")]
impl AuditCapture {
    /// 按调用方给出的新到旧顺序，仅接纳整组导航+三完整闭包。
    /// fixed费用含SQL及一次codec固定费，group费用含Key/Counts；不提供验证资格。
    /// 全新Verifier只建一次；跨组memo仅引用此前已接纳D。首次超额回滚新增D并终止，
    /// 核验器与半组资格立即丢弃。最终裁剪按值移交正文，仅封存已接纳三闭包的根。
    #[doc(hidden)]
    pub fn into_counts_original_subset_for_testonly(
        self,
        groups: impl IntoIterator<Item = (ContentRef, usize)>,
        fixed_source_bytes: usize,
        cap: usize,
    ) -> Result<
        (
            Self,
            CodecVec<NativeCountsSubsetCertificate>,
            NativeCountsSubsetReport,
        ),
        CodecError,
    > {
        self.into_counts_original_subset_admitted_for_testonly(
            groups.into_iter().map(|(state, bytes)| {
                (
                    capture_allocation::native_owned::NativeValue::<ContentRef>::from_unscoped(
                        state,
                    ),
                    bytes,
                )
            }),
            fixed_source_bytes,
            cap,
            None,
        )
    }
    #[doc(hidden)]
    pub fn into_counts_original_subset_admitted_for_testonly(
        mut self,
        groups: impl IntoIterator<
            Item = (
                capture_allocation::native_owned::NativeValue<ContentRef>,
                usize,
            ),
        >,
        fixed_source_bytes: usize,
        cap: usize,
        work: Option<&capture_allocation::scratch::WorkScope>,
    ) -> Result<
        (
            Self,
            CodecVec<NativeCountsSubsetCertificate>,
            NativeCountsSubsetReport,
        ),
        CodecError,
    > {
        let admission = NativeCountsSubsetAdmission(work);
        if !matches!(&self.nodes, CaptureNodes::Native(_)) {
            return Err(CodecError::new(
                code::SOURCE,
                "Counts稀疏原件只接受Native捕获",
            ));
        }
        self = self.into_originals_only_for_testonly()?;
        let ledger = std::cell::RefCell::new(NativeCountsSubsetLedger::default());
        let mut certificates = CodecVec::default();
        let mut report = NativeCountsSubsetReport::default();
        let mut accepted_metadata_bytes = 0usize;
        let mut admitted_public_roots = std::collections::HashSet::new();
        let mut admitted_public_fee = None;
        {
            let mut reader = NativeCountsSubsetReader {
                capture: &self,
                ledger: &ledger,
                admission,
            };
            let mut verifier = Verifier::new(&mut reader, Mode::Public);
            for (state, group_source_bytes) in groups {
                report.attempted_groups += 1;
                let checkpoint = {
                    let material = ledger.borrow();
                    (material.order.len(), material.source_bytes)
                };
                let names = ["objects", "witnesses", "observations"];
                let fields: capture_allocation::native_owned::NativeRefFields =
                    path::object_fields_output(
                        &mut verifier.source,
                        RefView::from(state.as_ref()),
                        &names,
                        false,
                    )?;
                // 字段拥有者先持原许可；证书另在当前域准入三根，fields随即释放。
                let root = |name| {
                    let value = fields
                        .get(name)
                        .ok_or_else(|| CodecError::new(code::SOURCE, format!("Counts缺{name}")))?;
                    capture_allocation::native_owned::NativeValue::<ContentRef>::try_from_view(
                        value.as_ref().into(),
                        work,
                    )
                };
                let roots = [root("objects")?, root("witnesses")?, root("observations")?];
                drop(fields);
                for root in &roots {
                    verifier.verify_refview(RefView::from(root.as_ref()))?;
                }
                // 完整核验成功后才允许私有构造；未核半组不会得到可携带证书。
                let certificate = NativeCountsSubsetCertificate {
                    state,
                    codec_revision: NativeCodecRootCertificates::default().predicate_revision,
                    dependency_revision: 1,
                    roots,
                };
                let certificate_bytes = certificate.source_bytes()?;
                // 只计算候选；组内及跨组去重，超额组不能写入或删除既有共享根。
                let mut new_public_roots = [None; 3];
                let mut new_public_root_count = 0usize;
                for root in &certificate.roots {
                    let digest = root.digest()?;
                    if !admitted_public_roots.contains(&digest)
                        && !new_public_roots.contains(&Some(digest))
                    {
                        new_public_roots[new_public_root_count] = Some(digest);
                        new_public_root_count += 1;
                    }
                }
                let codec_root_bytes = admitted_public_roots
                    .len()
                    .checked_add(new_public_root_count)
                    .and_then(|v| v.checked_mul(32))
                    .ok_or_else(|| CodecError::new(code::SOURCE, "Counts codec根费用溢出"))?;
                let metadata_bytes = accepted_metadata_bytes
                    .checked_add(group_source_bytes)
                    .and_then(|v| v.checked_add(certificate_bytes))
                    .ok_or_else(|| CodecError::new(code::SOURCE, "Counts整组费用溢出"))?;
                let required = fixed_source_bytes
                    .checked_add(metadata_bytes)
                    .and_then(|v| v.checked_add(ledger.borrow().source_bytes))
                    .and_then(|v| v.checked_add(codec_root_bytes))
                    .ok_or_else(|| CodecError::new(code::SOURCE, "Counts整池费用溢出"))?;
                report.last_attempted_source_bytes = required;
                if required > cap {
                    report.budget_stopped = true;
                    report.rolled_back_nodes = ledger.borrow_mut().rollback(checkpoint);
                    // 不继续使用已含失败组seen的Verifier，也不接受该组证书。
                    break;
                }
                admission.vec_room(&mut certificates, 1)?;
                // 资源失败直达可选缓存外边界，不变成source cap回滚或None重跑。
                admission.set_room(
                    &mut admitted_public_roots,
                    &mut admitted_public_fee,
                    new_public_root_count,
                )?;
                certificates.try_push(certificate)?;
                admitted_public_roots.extend(new_public_roots.into_iter().flatten());
                accepted_metadata_bytes = metadata_bytes;
                report.accepted_groups += 1;
                if report.accepted_groups == 1 {
                    report.first_admitted_source_bytes = required;
                }
                report.retained_source_bytes = required;
            }
        }
        let material = ledger.into_inner();
        let mut kept = std::collections::HashMap::new();
        let permit = admission.hash_room::<NodeDigest, NativeNode>(0, 0, material.digests.len())?;
        kept.try_reserve(material.digests.len()).map_err(|_| {
            admission.failure(capture_allocation::scratch::ScratchFailure::Allocation)
        })?;
        let CaptureNodes::Native(native) = &mut self.nodes else {
            unreachable!()
        };
        native.body_len = 0;
        native.body_capacity = 0;
        for (digest, node) in native.nodes.drain() {
            if material.digests.contains(&digest) {
                native.body_len = native.body_len.saturating_add(node.body.len());
                native.body_capacity = native.body_capacity.saturating_add(node.body.capacity());
                kept.insert(digest, node);
            }
            // 不保留的NativeNode在本轮drop；正文与其许可不复制、不留在旧HashMap或proof中。
        }
        native.nodes = kept;
        native.index_permit = permit;
        // collector产生的F proof及全请求memo都不得跟随稀疏owner保留。
        self = self.into_originals_only_for_testonly()?;
        // generated_state仅导航，不授予完整根资格；Template保持Default空集合。
        self.codec_roots.get_mut().public = admitted_public_roots;
        self.codec_roots.get_mut().public_permit = admitted_public_fee;
        Ok((self, certificates, report))
    }
}

/// Native owner 的无分配计费读数；不构造诊断 JSON 或赋予审计资格。
#[cfg(feature = "s_capture_native_testonly")]
#[doc(hidden)]
pub struct NativeCaptureRetention {
    pub nodes: usize,
    pub body_len: usize,
    pub body_capacity: usize,
    pub index_capacity: usize,
    pub index_payload: usize,
    pub public_roots: usize,
    pub template_roots: usize,
    pub public_capacity: usize,
    pub template_capacity: usize,
}

/// #1572 任务D：Native capture 物理许可的常数时间读数（固定个数标量与单张许可字节，不扫节点、
/// 不分配）。逐项依据见 [`AuditCapture::native_physical_fast_for_testonly`]。正文项不在此给出
/// 总额：`body_capacity` 计全部节点正文容量（含未绑定的 Plain 正文），只在全部正文已 Native 绑定
/// 时才等于正文许可之和，而“全部已绑定”不是 Native capture 的构造不变式。
#[cfg(feature = "s_capture_native_testonly")]
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeCapturePhysical {
    pub memo_seen: usize,
    pub node_index: usize,
    pub codec_roots: usize,
    pub owner_codec: usize,
    pub nodes: usize,
    pub body_capacity: usize,
}
#[cfg(feature = "s_capture_native_testonly")]
impl NativeCapturePhysical {
    /// capture 物理总额。`bound_nodes` 须由调用方独立取得（如
    /// `native_bound_body_nodes_for_testonly`，O(n)）；≠nodes 时拒给总额——常数时间判定缺
    /// `NativeCaptureNodes` 的“已绑定正文数（或已绑定正文许可字节）”增量标量。
    pub fn total_if_all_bound(&self, bound_nodes: usize) -> Result<usize, CodecError> {
        if bound_nodes != self.nodes {
            return Err(CodecError::new(
                code::SOURCE,
                format!(
                    "物理快读数：正文未全绑定（bound={bound_nodes} nodes={}）",
                    self.nodes
                ),
            ));
        }
        [
            self.memo_seen,
            self.node_index,
            self.codec_roots,
            self.owner_codec,
            self.body_capacity,
        ]
        .into_iter()
        .try_fold(0usize, usize::checked_add)
        .ok_or_else(|| CodecError::new(code::SOURCE, "物理快读数溢出"))
    }
}

#[cfg(feature = "s_capture_native_testonly")]
struct OriginalFees {
    bodies: CodecVec<(
        NodeDigest,
        Option<capture_allocation::scratch::NativeReservation>,
        // #1488 P2-A：来源合同额（只记账，不物理reserve），须指到同条已计费正文。
        Option<usize>,
    )>,
    table: CodecVec<capture_allocation::scratch::NativeReservation>,
}
pub struct AuditCapture {
    nodes: CaptureNodes,
    public: std::cell::RefCell<AuditMemo>,
    template: std::cell::RefCell<AuditMemo>,
    reads: std::cell::Cell<u64>,
    #[cfg(feature = "s_capture_native_testonly")]
    codec_roots: std::cell::RefCell<NativeCodecRootCertificates>,
    #[cfg(feature = "s_capture_native_testonly")]
    owner_codec: std::cell::RefCell<NativeOwnerCodecCertificates>,
    // nodes先释放；无复制初始费用随对应Legacy原件直到按值转换。
    #[cfg(feature = "s_capture_native_testonly")]
    original_fees: Option<OriginalFees>,
}
impl AuditCapture {
    /// #1394 隔离来源缓存：消费审计捕获，只保留原始tuple owner，清除全部验证和帧证明。
    /// 不复制正文、不重新读SQL；返回值的存在不表示其中每个无关节点都被验证过。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn into_originals_only_for_testonly(mut self) -> Result<Self, CodecError> {
        *self.codec_roots.get_mut() = NativeCodecRootCertificates::default();
        self.clear_request_codec_and_frames_for_testonly()?;
        Ok(self)
    }

    /// #1394 同一成功handler封存原件与纯codec根证书；全部语义memo和Frame均清空。
    /// 原件证明须由下一次当前Native capture重新逐tuple绑定，证书不授予业务资格。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn into_originals_and_codec_roots_for_testonly(mut self) -> Result<Self, CodecError> {
        if !matches!(&self.nodes, CaptureNodes::Native(_)) {
            return Err(CodecError::new(code::SOURCE, "codec根证书只封存Native捕获"));
        }
        self.clear_request_codec_and_frames_for_testonly()?;
        Ok(self)
    }

    #[cfg(feature = "s_capture_native_testonly")]
    fn clear_request_codec_and_frames_for_testonly(&mut self) -> Result<(), CodecError> {
        if matches!(self.nodes, CaptureNodes::AllocationPrototype(_)) {
            return Err(CodecError::new(
                code::SOURCE,
                "Owned捕获不得进入Native来源缓存",
            ));
        }
        *self.public.get_mut() = AuditMemo::default();
        *self.template.get_mut() = AuditMemo::default();
        self.owner_codec.get_mut().clear();
        if let CaptureNodes::Native(native) = &mut self.nodes {
            let bytes = native.body_len;
            for node in native.nodes.values_mut() {
                *node.proof.get_mut() = None;
                node.owner_dirty.set(false);
                node.owner_next.set(None);
            }
            native.owner_dirty_head.set(None);
            *native.memo.get_mut() = NativeFrameMemo::new(native.nodes.len(), bytes);
        }
        self.reads.set(0);
        Ok(())
    }

    /// #1394 APPROVED-SOURCE.md:65、286：本次完整原件相同包含旧capture全部tuple才复用。
    /// 调用方不能传same资格；Legacy/Owned、上下文不兼容或材料不包含时保持当前冷路径。
    /// 仅在新的独立memo.seen播种已成功的Value根，所有其余工作集与用量收据均冷。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn seed_codec_roots_from_native_capture_for_testonly(
        &self,
        prior: &Self,
    ) -> Result<usize, CodecError> {
        self.seed_codec_roots_admitted_for_testonly(prior, None)
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn seed_codec_roots_admitted_for_testonly(
        &self,
        prior: &Self,
        work: Option<&capture_allocation::scratch::WorkScope>,
    ) -> Result<usize, CodecError> {
        let (CaptureNodes::Native(current), CaptureNodes::Native(previous)) =
            (&self.nodes, &prior.nodes)
        else {
            return Ok(0);
        };
        let old = prior.codec_roots.borrow();
        let fresh = NativeCodecRootCertificates::default();
        if old.format != fresh.format
            || old.predicate_revision != fresh.predicate_revision
            || old.flavor != Flavor::Value
            || previous.nodes.len() > current.nodes.len()
            || !previous.nodes.iter().all(|(digest, node)| {
                current
                    .nodes
                    .get(digest)
                    .is_some_and(|now| now.tag == node.tag && now.body[..] == node.body[..])
            })
        {
            return Ok(0);
        }
        let mut public = AuditMemo::default();
        let mut template = AuditMemo::default();
        // StdSet即标准HashSet；先许可含装载率/桶舍入上界，再用fallible reserve建表。
        for (memo, len) in [
            (&mut public, old.public.len()),
            (&mut template, old.template.len()),
        ] {
            if let Some(work) = work {
                let bytes = if len == 0 {
                    0
                } else {
                    len.checked_next_power_of_two()
                        .and_then(|n| n.checked_mul(4))
                        .and_then(|n| n.checked_add(16))
                        .and_then(|n| {
                            n.checked_mul(std::mem::size_of::<(NodeDigest, Flavor)>() + 1)
                        })
                        .ok_or_else(|| {
                            work.reject(capture_allocation::scratch::ScratchFailure::Budget)
                        })?
                };
                let permit = work.reserve_native(bytes)?;
                let WorkSet::Plain(values) = &mut memo.seen else {
                    unreachable!()
                };
                values.try_reserve(len).map_err(|_| {
                    work.reject(capture_allocation::scratch::ScratchFailure::Allocation)
                })?;
                memo.native_seen_permit = Some(permit);
            }
        }
        for digest in &old.public {
            public.seen.insert((*digest, Flavor::Value))?;
        }
        for digest in &old.template {
            template.seen.insert((*digest, Flavor::Value))?;
        }
        let roots = old
            .public
            .len()
            .checked_add(old.template.len())
            .ok_or_else(|| CodecError::new(code::SOURCE, "codec根证书计数溢出"))?;
        *self.public.borrow_mut() = public;
        *self.template.borrow_mut() = template;
        // 不复制旧root证书：只有本capture verify_refview再次完整成功才记录可封存根。
        Ok(roots)
    }

    /// #1394 owner 已在锁内回读的追加 tuple。只保持内容关系，不能签发 Q/C 资格。
    /// cap 是调用方扣除全部共存归属后给出的索引扩容余量；正文按值移动。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn append_original_for_owner_testonly(
        &mut self,
        digest: NodeDigest,
        tag: u8,
        body: Vec<u8>,
        cap: usize,
        work: Option<&capture_allocation::scratch::WorkScope>,
    ) -> Result<bool, CodecError> {
        let result = self.append_original_for_owner_inner(
            digest,
            tag,
            CodecVec::from_plain(body),
            cap,
            work,
        );
        if !matches!(&result, Ok(true)) {
            self.owner_codec.get_mut().revoke();
        }
        result
    }
    /// #1488 R 正文移交：只接受已有 Native 绑定，原 backing 与许可随节点移动。
    /// 来源 body.len()+97 与索引旧费用另计；旧 E retain 通道仍未迁移。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn append_original_codec_for_owner_testonly(
        &mut self,
        digest: NodeDigest,
        tag: u8,
        mut body: CodecVec<u8>,
        cap: usize,
        work: &capture_allocation::scratch::WorkScope,
    ) -> Result<bool, CodecError> {
        let result = (|| {
            if !body.is_native_bound() {
                return Err(CodecError::new(
                    code::SOURCE,
                    "owner正文移交只接受已有Native绑定",
                ));
            }
            // 零增长也核本次活动域与原许可同父，含容量为零的正文。
            body.try_reserve_native(work, 0)?;
            self.append_original_for_owner_inner(digest, tag, body, cap, Some(work))
        })();
        if !matches!(&result, Ok(true)) {
            self.owner_codec.get_mut().revoke();
        }
        result
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn append_original_for_owner_inner(
        &mut self,
        digest: NodeDigest,
        tag: u8,
        body: CodecVec<u8>,
        cap: usize,
        work: Option<&capture_allocation::scratch::WorkScope>,
    ) -> Result<bool, CodecError> {
        if frame(tag, &body) != digest {
            return Err(CodecError::new(code::NODE_DIGEST, "实际追加节点摘要不符"));
        }
        let CaptureNodes::Native(native) = &mut self.nodes else {
            return Err(CodecError::new(
                code::SOURCE,
                "owner 追加只接受 Native 原件",
            ));
        };
        if let Some(old) = native.nodes.get(&digest) {
            if old.tag != tag || old.body[..] != body[..] {
                return Err(CodecError::new(code::NODE_DIGEST, "同摘要异原件"));
            }
            return Ok(true);
        }
        // body 按值从 effects 移入，其分配已收费；S7 正文/证书归属只登记合同项（#1488 P2-A），
        // 须指到这块已物理计费的正文，不再在父账同额物理reserve。
        let source_claim = work.map(|_| SourceClaim::on_body(&body)).transpose()?;
        if native.nodes.len() == native.nodes.capacity() {
            let capacity = native.nodes.capacity().saturating_mul(2).max(4);
            let growth = capacity
                .saturating_mul(2)
                .saturating_add(16)
                .saturating_mul(std::mem::size_of::<(NodeDigest, NativeNode)>() + 1);
            let permit = work.map(|w| w.reserve_native(growth)).transpose()?;
            if growth > cap || native.nodes.try_reserve(1).is_err() {
                if let Some(work) = work {
                    return Err(
                        work.reject(capture_allocation::scratch::ScratchFailure::Allocation)
                    );
                }
                return Ok(false);
            }
            native.index_permit = permit;
        }
        let bytes = body.capacity();
        native.body_len = native
            .body_len
            .checked_add(body.len())
            .ok_or_else(|| CodecError::new(code::SOURCE, "原件body总长度溢出"))?;
        native.body_capacity = native
            .body_capacity
            .checked_add(bytes)
            .ok_or_else(|| CodecError::new(code::SOURCE, "原件body总capacity溢出"))?;
        let mut node = NativeNode::new_codec(tag, body);
        node.source_claim = source_claim;
        native.nodes.insert(digest, node);
        let memo = native.memo.get_mut();
        memo.entry_limit = memo.entry_limit.saturating_add(1);
        memo.body_capacity_limit = memo.body_capacity_limit.saturating_add(bytes);
        Ok(true)
    }

    /// #1394 owner正常追加的热交接：只清本步实际生成过proof的节点。
    /// 完整原件与根证书继续保留；不扫描旧节点求body总量。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn finish_owner_step_for_testonly(mut self) -> Result<Self, CodecError> {
        let CaptureNodes::Native(native) = &mut self.nodes else {
            return Err(CodecError::new(code::SOURCE, "owner交接只接受Native"));
        };
        *self.public.get_mut() = AuditMemo::default();
        *self.template.get_mut() = AuditMemo::default();
        let mut next = native.owner_dirty_head.take();
        while let Some(digest) = next {
            let node = native
                .nodes
                .get_mut(&digest)
                .expect("dirty链节点由同owner完整持有");
            next = node.owner_next.take();
            node.owner_dirty.set(false);
            native.memo.get_mut().remove(node.proof.get_mut());
        }
        *native.memo.get_mut() = NativeFrameMemo::new(native.nodes.len(), native.body_len);
        self.reads.set(0);
        Ok(self)
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
    fn assert_owner_retention_oracle(&self) {
        let CaptureNodes::Native(native) = &self.nodes else {
            panic!("Native oracle");
        };
        assert_eq!(
            native.body_len,
            native.nodes.values().map(|v| v.body.len()).sum::<usize>()
        );
        assert_eq!(
            native.body_capacity,
            native
                .nodes
                .values()
                .map(|v| v.body.capacity())
                .sum::<usize>()
        );
    }

    /// #1394 私有Native请求复用的原件绑定：比较完整digest/tag/body集合；不读取证明或摘要替代正文。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn exact_original_tuples_equal(&self, other: &Self) -> Result<bool, CodecError> {
        let count = |nodes: &CaptureNodes| match nodes {
            CaptureNodes::Legacy(nodes) => Some(nodes.len()),
            CaptureNodes::Native(nodes) => Some(nodes.nodes.len()),
            CaptureNodes::AllocationPrototype(_) => None,
        };
        let Some(left_len) = count(&self.nodes) else {
            return Err(CodecError::new(
                code::SOURCE,
                "Owned捕获不得参与Native请求历史复用",
            ));
        };
        let Some(right_len) = count(&other.nodes) else {
            return Err(CodecError::new(
                code::SOURCE,
                "Owned捕获不得参与Native请求历史复用",
            ));
        };
        if left_len != right_len {
            return Ok(false);
        }
        let matches = |digest: &NodeDigest, tag: u8, body: &[u8]| match &other.nodes {
            CaptureNodes::Legacy(nodes) => nodes
                .get(digest)
                .is_some_and(|(t, b)| *t == tag && b.as_slice() == body),
            CaptureNodes::Native(nodes) => nodes
                .nodes
                .get(digest)
                .is_some_and(|node| node.tag == tag && &node.body[..] == body),
            CaptureNodes::AllocationPrototype(_) => false,
        };
        Ok(match &self.nodes {
            CaptureNodes::Legacy(nodes) => nodes
                .iter()
                .all(|(digest, (tag, body))| matches(digest, *tag, body)),
            CaptureNodes::Native(nodes) => nodes
                .nodes
                .iter()
                .all(|(digest, node)| matches(digest, node.tag, &node.body)),
            CaptureNodes::AllocationPrototype(_) => unreachable!(),
        })
    }
    pub fn new(nodes: MemoryNodes) -> Self {
        Self {
            nodes: CaptureNodes::Legacy(nodes.nodes),
            public: Default::default(),
            template: Default::default(),
            reads: Default::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            codec_roots: Default::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            owner_codec: Default::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            original_fees: None,
        }
    }
    #[doc(hidden)]
    pub fn from_slab_for_allocation_prototype(slab: capture_allocation::CaptureSlab) -> Self {
        let budget = slab.owned_budget();
        Self {
            nodes: CaptureNodes::AllocationPrototype(slab),
            public: std::cell::RefCell::new(AuditMemo::new(Some(&budget))),
            template: std::cell::RefCell::new(AuditMemo::new(Some(&budget))),
            reads: Default::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            codec_roots: Default::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            owner_codec: Default::default(),
            #[cfg(feature = "s_capture_native_testonly")]
            original_fees: None,
        }
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn with_original_fees_for_testonly(
        mut self,
        table: CodecVec<capture_allocation::scratch::NativeReservation>,
        mut bodies: CodecVec<(
            NodeDigest,
            Option<capture_allocation::scratch::NativeReservation>,
            Option<usize>,
        )>,
    ) -> Result<Self, CodecError> {
        bodies.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        self.original_fees = Some(OriginalFees { bodies, table });
        let CaptureNodes::Legacy(nodes) = &self.nodes else {
            return Err(CodecError::scratch(
                capture_allocation::scratch::ScratchFailure::Ledger,
            ));
        };
        let fees = self.original_fees.as_ref().unwrap();
        if nodes.len() != fees.bodies.len()
            || nodes.iter().zip(fees.bodies.iter().rev()).any(
                |((digest, (_, body)), (key, fee, source))| {
                    digest != key
                        || fee
                            .as_ref()
                            .is_some_and(|fee| fee.bytes() != body.capacity())
                        // #1488 P2-A：来源合同项指不到已物理计费的正文块即拒。
                        || source.is_some_and(|bytes| {
                            fee.is_none() || body.len().checked_add(97) != Some(bytes)
                        })
                },
            )
        {
            return Err(CodecError::scratch(
                capture_allocation::scratch::ScratchFailure::Ledger,
            ));
        }
        Ok(self)
    }
    /// #1394：显式 Native TestOnly opt-in；资格只来自本捕获的节点数和原 body 总长度。
    /// 不适用于 Owned/AllocationPrototype，不声明整 heap 或 S 资源准入。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn enable_native_frame_reuse_for_testonly(self) -> Result<Self, CodecError> {
        self.enable_native_frame_reuse_admitted_for_testonly(None)
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn enable_native_frame_reuse_admitted_for_testonly(
        mut self,
        work: Option<&capture_allocation::scratch::WorkScope>,
    ) -> Result<Self, CodecError> {
        if let CaptureNodes::Native(native) = &mut self.nodes {
            let body_capacity_limit = native.nodes.values().try_fold(0usize, |sum, node| {
                sum.checked_add(node.body.len())
                    .ok_or_else(|| CodecError::new(code::SOURCE, "Native捕获body总长度溢出"))
            })?;
            // 保留旧重复 opt-in 的冷 proof 语义；不重置原件、reads 或语义 memo。
            for node in native.nodes.values_mut() {
                *node.proof.get_mut() = None;
                node.owner_dirty.set(false);
                node.owner_next.set(None);
            }
            native.owner_dirty_head.set(None);
            *native.memo.get_mut() = NativeFrameMemo::new(native.nodes.len(), body_capacity_limit);
            return Ok(self);
        }
        let CaptureNodes::Legacy(nodes) = &self.nodes else {
            return Err(CodecError::new(
                code::SOURCE,
                "Native帧复用只接受本次Legacy捕获",
            ));
        };
        let body_capacity_limit = nodes.values().try_fold(0usize, |sum, (_, body)| {
            sum.checked_add(body.len())
                .ok_or_else(|| CodecError::new(code::SOURCE, "Native捕获body总长度溢出"))
        })?;
        let entry_limit = nodes.len();
        let mut native_nodes = std::collections::HashMap::new();
        let index_bytes = if entry_limit == 0 {
            0
        } else {
            entry_limit
                .checked_next_power_of_two()
                .and_then(|n| n.checked_mul(4))
                .and_then(|n| n.checked_add(16))
                .and_then(|n| n.checked_mul(std::mem::size_of::<(NodeDigest, NativeNode)>() + 1))
                .ok_or_else(|| {
                    work.map_or_else(
                        || CodecError::new(capture_allocation::LIMIT, "Native索引容量溢出"),
                        |w| w.reject(capture_allocation::scratch::ScratchFailure::Budget),
                    )
                })?
        };
        let permit = work.map(|w| w.reserve_native(index_bytes)).transpose()?;
        native_nodes.try_reserve(entry_limit).map_err(|_| {
            work.map_or_else(
                || CodecError::new(code::SOURCE, "Native捕获索引分配失败"),
                |w| w.reject(capture_allocation::scratch::ScratchFailure::Allocation),
            )
        })?;
        let CaptureNodes::Legacy(nodes) =
            std::mem::replace(&mut self.nodes, CaptureNodes::Legacy(BTreeMap::new()))
        else {
            unreachable!()
        };
        // 移动原件 Vec，不预核 F、不复制全部正文，不改变第一处严格拒绝的位置。
        let mut body_capacity = 0usize;
        for (digest, (tag, body)) in nodes {
            body_capacity = body_capacity
                .checked_add(body.capacity())
                .ok_or_else(|| CodecError::new(code::SOURCE, "原件capacity溢出"))?;
            let mut body = CodecVec::from_plain(body);
            let mut source_claim = None;
            if let Some(fees) = &mut self.original_fees {
                let (key, fee, source) = fees.bodies.pop().expect("已核原件费用条目");
                debug_assert_eq!(key, digest);
                if let Some(fee) = fee {
                    body.attach_native_reservation(fee)?;
                }
                if let Some(bytes) = source {
                    let claim = SourceClaim::on_body(&body)?;
                    if claim.bytes != bytes {
                        return Err(CodecError::scratch(
                            capture_allocation::scratch::ScratchFailure::Ledger,
                        ));
                    }
                    source_claim = Some(claim);
                }
            }
            let mut node = NativeNode::new_codec(tag, body);
            node.source_claim = source_claim;
            native_nodes.insert(digest, node);
        }
        // Legacy BTree迭代器已耗尽，树节点已经释放；现在才归还旧表及费用索引容量。
        drop(self.original_fees.take());
        self.nodes = CaptureNodes::Native(NativeCaptureNodes {
            nodes: native_nodes,
            index_permit: permit,
            body_len: body_capacity_limit,
            body_capacity,
            owner_dirty_head: Default::default(),
            memo: std::cell::RefCell::new(NativeFrameMemo::new(entry_limit, body_capacity_limit)),
            #[cfg(test)]
            index_lookups: Default::default(),
        });
        Ok(self)
    }
    pub fn contains(&self, digest: &NodeDigest) -> bool {
        self.original(digest).is_some()
    }
    /// #1394：只读见证实际Native owner，不扫描原件或改变任何证明。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn native_frame_reuse_active_for_testonly(&self) -> bool {
        matches!(&self.nodes, CaptureNodes::Native(_))
    }
    pub fn original(&self, digest: &NodeDigest) -> Option<(u8, &[u8])> {
        match &self.nodes {
            CaptureNodes::Legacy(nodes) => nodes.get(digest).map(|(t, b)| (*t, b.as_slice())),
            #[cfg(feature = "s_capture_native_testonly")]
            CaptureNodes::Native(nodes) => nodes.get(digest).map(|n| (n.tag, &n.body[..])),
            CaptureNodes::AllocationPrototype(slab) => slab.original(digest),
        }
    }
    pub fn reader(&self) -> impl NodeSource + '_ {
        CaptureReader(self)
    }
    #[doc(hidden)]
    pub fn reader_observed<'a>(
        &'a self,
        observer: &'a dyn CaptureReadObserver,
    ) -> impl NodeSource + 'a {
        ObservedCaptureReader {
            reader: CaptureReader(self),
            observer: Some(observer),
        }
    }
    pub fn verify(&self, reference: &ContentRef, mode: Mode) -> Result<(), CodecError> {
        self.verify_refview(reference.into(), mode)
    }
    #[doc(hidden)]
    pub fn verify_refview(&self, reference: RefView<'_>, mode: Mode) -> Result<(), CodecError> {
        self.verify_refview_observed(reference, mode, None)
    }
    /// 当前调用的读取观察；保留原 capture memo，不保存观察者。
    #[doc(hidden)]
    pub fn verify_refview_observed(
        &self,
        reference: RefView<'_>,
        mode: Mode,
        observer: Option<&dyn CaptureReadObserver>,
    ) -> Result<(), CodecError> {
        let slot = if mode == Mode::Public {
            &self.public
        } else {
            &self.template
        };
        let budget = match &self.nodes {
            CaptureNodes::AllocationPrototype(s) => Some(s.owned_budget()),
            _ => None,
        };
        let memo = std::mem::replace(&mut *slot.borrow_mut(), AuditMemo::new(budget.as_ref()));
        let mut verifier =
            Verifier::with_context(CodecContext::Capture(self, observer), mode, memo);
        let result = verifier.verify_refview(reference);
        // 出错不保留 run() 提前插入的 seen/pushed，不能让下一根继承半成功。
        if let Ok((digest, _, _)) = &result {
            *slot.borrow_mut() = verifier.into_memo();
            #[cfg(feature = "s_capture_native_testonly")]
            if matches!(&self.nodes, CaptureNodes::Native(_)) {
                let mut roots = self.codec_roots.borrow_mut();
                // 已封存的Native表没有本次WorkScope，不能从旧许可新签扩容。
                let sealed_public = mode == Mode::Public && roots.public_permit.is_some();
                let set = if mode == Mode::Public {
                    &mut roots.public
                } else {
                    &mut roots.template
                };
                // 证书保留失败不改变本次已经成功的业务校验；只是下次少一个复用根。
                if set.contains(digest) || (!sealed_public && set.try_reserve(1).is_ok()) {
                    set.insert(*digest);
                }
            }
            #[cfg(not(feature = "s_capture_native_testonly"))]
            let _ = digest;
        }
        result.map(|_| ())
    }
    /// #1488 仅供实际 owner 的 append-only/effects 续接：持同一个原件 owner，
    /// 不接受外来 source、成功集合或资格 bool。调用方负责真实事务/effects/COMMIT。
    /// 新 scratch 只借本轮父账；成功证明无旧 WorkBudget，跨轮不会污染新操作账。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn verify_owner_refview_for_testonly(
        &self,
        reference: RefView<'_>,
        mode: Mode,
        work: &capture_allocation::scratch::WorkScope,
    ) -> Result<(), CodecError> {
        let result = (|| {
            // 零新增的 warm hit 同样必须核当前工作域健康，不凭旧容量绕过停止门。
            let _health = work.reserve_native(0)?;
            if !matches!(&self.nodes, CaptureNodes::Native(_)) {
                return Err(CodecError::new(
                    code::SOURCE,
                    "owner codec 只接受同一 Native 原件",
                ));
            }
            let certificates = self.owner_codec.borrow();
            if !certificates.compatible() {
                return Err(CodecError::new(
                    code::SOURCE,
                    "owner codec 续接已撤销或解释版本不符",
                ));
            }
            let prior = if mode == Mode::Public {
                &certificates.public
            } else {
                &certificates.template
            };
            let mut verifier = Verifier::with_context(
                CodecContext::OwnerCapture {
                    capture: self,
                    work,
                },
                mode,
                AuditMemo::default(),
            );
            verifier.owner_codec = Some(prior);
            verifier.verify_refview(reference)?;
            // 全部待核后继成功后才转出 scratch。失败提前插入的 seen/序列摘要永不提升。
            let memo = verifier.into_memo();
            drop(certificates);
            let mut certificates = self.owner_codec.borrow_mut();
            let retained = if mode == Mode::Public {
                &mut certificates.public
            } else {
                &mut certificates.template
            };
            retained.merge_success(&memo, work)?;
            // memo 到此析构，scratch 真 Layout 与父账一起释放；retained 无旧作用域引用。
            Ok(())
        })();
        if result.is_err() {
            self.owner_codec.borrow_mut().revoke();
        }
        result
    }

    /// 持久证明实际 heap Layout；常数张表，固定头由 enclosing capture owner 计费。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn owner_codec_retained_bytes_for_testonly(&self) -> usize {
        let certificates = self.owner_codec.borrow();
        certificates
            .public
            .retained_bytes()
            .saturating_add(certificates.template.retained_bytes())
    }

    /// #1572 任务D：capture 物理许可的常数时间读数（逐项见 [`NativeCapturePhysical`]）。
    /// - MemoSeen：Public/Template 两份 memo 的 `native_seen_permit` 各一张许可；
    /// - NodeIndex：`index_permit` 一张许可；CodecRoots：`public_permit` 一张许可（Template 根集合无许可）；
    /// - OwnerCodec：两份证书各五棵树的 `retained_bytes` 之和。`OwnerProofTree::insert` 只在
    ///   `insert_absent` 以 `reserve_native(Layout::new::<OwnerProofNode<K, V>>().size())` 签出许可并
    ///   `NativeBox::from_reserved(node, Some(permit))` 挂上之后，才累加同一个 `layout.size()`；
    ///   预留或分配失败不累加（许可随错误出口归还）；旋转只移指针不动许可；无单节点删除，树只经
    ///   `clear`/`revoke` 整体换成 Default（计数同时归零）⟹ 每棵树 `retained_bytes` 恒等于其节点盒
    ///   许可之和；
    /// - 节点证明盒：许可额=原件长+`size_of::<NativeFrameProof>()`（`checked_read_scoped` 预留后
    ///   `attach`），但 `NativeFrameMemo::retain_copy` 也建无许可的盒，memo 只计帧正文容量与盒数、
    ///   不计已附许可额 ⟹ 只在 `retained_entries==0`（无盒：盒只由 `retain_copy` 建且同步计数，
    ///   拆盒经 `remove` 同步减计数，整体清空处同时重置 memo）时可证为 0，否则 Err——缺
    ///   `NativeFrameMemo` 的“已附证明许可字节”增量；
    /// - 未转换原件费：只在 Legacy 上存在——`with_original_fees_for_testonly` 只接 Legacy，转换末尾
    ///   `drop(self.original_fees.take())`，其余构造均置 None；Native 上若出现即 Err；
    /// - 正文：只给 `body_capacity` 与 nodes，总额须经 [`NativeCapturePhysical::total_if_all_bound`]。
    ///   绑定正文的许可额恒等于其 capacity（`attach_native_reservation` 核 bytes==capacity；
    ///   `reserve_native_work`/`try_reserve_native_adopting`/`try_compact_native` 均按新块精确容量签许可；
    ///   已绑定后 `reserve`/`reserve_exact` 拒绝越容量增长），`body_capacity` 在转换、追加、稀疏保留
    ///   三处按 capacity 维护；但“每个正文都已绑定”不由构造保证：无原件费转换（
    ///   `enable_native_frame_reuse_for_testonly`）、原件费表中 fee=None 的条目、
    ///   `append_original_for_owner_testonly(.., None)` 都留下 Plain 正文。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn native_physical_fast_for_testonly(&self) -> Result<NativeCapturePhysical, CodecError> {
        let CaptureNodes::Native(native) = &self.nodes else {
            return Err(CodecError::new(
                code::SOURCE,
                "物理快读数只接Native capture",
            ));
        };
        if self.original_fees.is_some() {
            return Err(CodecError::new(
                code::SOURCE,
                "物理快读数：Native capture 不应持未转换原件费",
            ));
        }
        if native.memo.borrow().retained_entries != 0 {
            return Err(CodecError::new(
                code::SOURCE,
                "物理快读数：存活节点证明盒无常数时间许可读数",
            ));
        }
        let bytes = capture_allocation::scratch::NativeReservation::bytes;
        let mut memo_seen = 0usize;
        for memo in [&self.public, &self.template] {
            let permit = memo.borrow().native_seen_permit.as_ref().map_or(0, bytes);
            memo_seen = memo_seen
                .checked_add(permit)
                .ok_or_else(|| CodecError::new(code::SOURCE, "物理快读数溢出"))?;
        }
        let roots = self.codec_roots.borrow();
        Ok(NativeCapturePhysical {
            memo_seen,
            node_index: native.index_permit.as_ref().map_or(0, bytes),
            codec_roots: roots.public_permit.as_ref().map_or(0, bytes),
            owner_codec: self.owner_codec_retained_bytes_for_testonly(),
            nodes: native.nodes.len(),
            body_capacity: native.body_capacity,
        })
    }

    /// #1488 P3-c：逐个列出本capture实际持有的Native许可字节（索引、正文块、codec根、
    /// 未转换原件费及其容器、memo seen）；按持有者枚举，不由live差额或释放读数推导。
    /// 证明、owner codec证书仍存活时拒绝（封存后应已清空，不得漏列）。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn native_permit_inventory_for_testonly(
        &self,
        out: &mut Vec<usize>,
    ) -> Result<(), CodecError> {
        self.visit_native_permits(true, &mut |entry: CapturePermitEntry| out.push(entry.bytes))
    }
    /// #1572：同上清单，逐张带身份与所保护载荷；字节序列与 `native_permit_inventory_for_testonly`
    /// 逐项相同（二者共用逐许可 visitor，边遍历边写入调用者的 out）。条目借用本 capture。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn native_permit_entries_for_testonly<'a>(
        &'a self,
        out: &mut Vec<CapturePermitEntry<'a>>,
    ) -> Result<(), CodecError> {
        self.visit_native_permits(true, &mut |entry: CapturePermitEntry<'a>| out.push(entry))
    }
    /// #1572：存活候选的完整清单——在封存清单之外再列 owner codec 证书节点与节点证明盒的许可，
    /// 不要求二者已清空（封存池仍用上面的严格版本）。条目借用本 capture。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn native_live_permit_entries_for_testonly<'a>(
        &'a self,
        out: &mut Vec<CapturePermitEntry<'a>>,
    ) -> Result<(), CodecError> {
        {
            let owner = self.owner_codec.borrow();
            owner.public.permit_entries(out);
            owner.template.permit_entries(out);
        }
        self.visit_native_permits(false, &mut |entry: CapturePermitEntry<'a>| out.push(entry))
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn visit_native_permits<'a>(
        &'a self,
        sealed: bool,
        visit: &mut impl FnMut(CapturePermitEntry<'a>),
    ) -> Result<(), CodecError> {
        use CapturePermitRole as R;
        let held = CapturePermitEntry::held;
        if sealed && self.owner_codec_retained_bytes_for_testonly() != 0 {
            return Err(CodecError::new(
                code::SOURCE,
                "许可清单：owner codec证书未清空",
            ));
        }
        for memo in [&self.public, &self.template] {
            let memo = memo.borrow();
            if let Some(p) = memo.native_seen_permit.as_ref() {
                visit(held(p, R::MemoSeen, None));
            }
        }
        if let CaptureNodes::Native(native) = &self.nodes {
            if let Some(p) = native.index_permit.as_ref() {
                visit(held(p, R::NodeIndex, None));
            }
            for (digest, node) in native.nodes.iter() {
                let proof = node.proof.borrow();
                if sealed && proof.is_some() {
                    return Err(CodecError::new(code::SOURCE, "许可清单：节点证明未清空"));
                }
                if let Some(p) = proof.as_ref().and_then(|b| b.permit()) {
                    visit(held(p, R::NodeProof, None));
                }
                if let Some(p) = node.body.native_permit_ref() {
                    visit(held(p, R::NodeBody, Some((*digest, node.body.len()))));
                }
            }
        }
        {
            let roots = self.codec_roots.borrow();
            if let Some(p) = roots.public_permit.as_ref() {
                visit(held(p, R::CodecRoots, None));
            }
        }
        if let Some(fees) = &self.original_fees {
            if let Some(p) = fees.bodies.native_permit_ref() {
                visit(held(p, R::OriginalFeeBodies, None));
            }
            // 费用表按摘要降序、Legacy 原件升序（见 `with_original_fees_for_testonly`），逆序逐条
            // 对位取正文长度；不查表、不分配。对不上的条目不带节点身份，对账时拒绝。
            let mut legacy = match &self.nodes {
                CaptureNodes::Legacy(nodes) => Some(nodes.iter().rev()),
                _ => None,
            };
            for (key, permit, _) in fees.bodies.iter() {
                let original = legacy.as_mut().and_then(Iterator::next);
                if let Some(p) = permit.as_ref() {
                    let body = original
                        .filter(|(digest, _)| *digest == key)
                        .map(|(_, (_, body))| (*key, body.len()));
                    visit(held(p, R::OriginalFeeBody, body));
                }
            }
            if let Some(p) = fees.table.native_permit_ref() {
                visit(held(p, R::OriginalFeeTable, None));
            }
            for p in fees.table.iter() {
                visit(held(p, R::OriginalFeeTableEntry, None));
            }
        }
        Ok(())
    }
    /// #1572：逐项列出来源合同（节点正文及未转换原件费），每项带生成时所在节点的摘要与它所指
    /// 正文许可的身份。节点项按 #1488 P2-A 先核合同额=正文+97 且正文已Native绑定；未转换原件费项
    /// 须有同条许可。条目借用本 capture。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn native_source_claim_entries_for_testonly(
        &self,
    ) -> Result<Vec<SourceClaimEntry<'_>>, CodecError> {
        let unbacked = || CodecError::new(code::SOURCE, "来源合同项指不到已物理计费的正文块");
        let mut out = Vec::new();
        self.visit_node_source_claims(|digest, node, bytes| {
            let backing = node.body.native_permit_ref().ok_or_else(unbacked)?;
            out.push(SourceClaimEntry::new(
                bytes,
                backing.permit_id(),
                CapturePermitRole::NodeBody,
                *digest,
            ));
            Ok(())
        })?;
        if let Some(fees) = &self.original_fees {
            for (key, permit, claim) in fees.bodies.iter() {
                if let Some(amount) = claim {
                    out.push(SourceClaimEntry::new(
                        *amount,
                        permit.as_ref().ok_or_else(unbacked)?.permit_id(),
                        CapturePermitRole::OriginalFeeBody,
                        *key,
                    ));
                }
            }
        }
        Ok(out)
    }
    /// #1572：来源合同对物理清单对账（顺序无关）。物理表每张许可恰列一次，且都挂期望父账
    /// `ledger`、来源域属 `origins`；每项合同须指到一张同位置的正文许可，且每张正文许可恰有一项
    /// 合同（不得两项指同一正文，也不得漏签）。合同的节点须与所指正文许可的节点相同，合同额须等于
    /// 按该正文长度独立推出的 `SourceClaim::on_len`。合同额全额保留，物理只由正文许可计一次；
    /// 合同项本身不产生物理字节。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn reconcile_native_source_claims_for_testonly(
        entries: &[CapturePermitEntry<'_>],
        claims: &[SourceClaimEntry<'_>],
        ledger: usize,
        origins: &[u64],
    ) -> Result<SourceClaimReconciliation, CodecError> {
        use std::collections::BTreeMap;
        let fail = |m: &'static str| CodecError::new(code::SOURCE, m);
        let mut physical = BTreeMap::new();
        for entry in entries {
            if entry.ledger != ledger {
                return Err(fail("许可清单：许可不属期望父账"));
            }
            if !origins.contains(&entry.origin) {
                return Err(fail("许可清单：许可不属期望来源域"));
            }
            if physical.insert(entry.id, entry).is_some() {
                return Err(fail("许可清单：同一许可重复列出"));
            }
        }
        let mut backed = BTreeMap::new();
        let mut paired = Vec::with_capacity(claims.len());
        for claim in claims {
            let body = physical
                .get(&claim.backing)
                .filter(|e| e.role == claim.site)
                .ok_or_else(|| fail("来源合同项指不到已物理计费的正文块"))?;
            if backed.insert(claim.backing, ()).is_some() {
                return Err(fail("两项来源合同指同一正文块"));
            }
            paired.push((claim, *body));
        }
        let mut out = SourceClaimReconciliation::default();
        for (claim, body) in paired {
            let (node, len) = body
                .body
                .ok_or_else(|| fail("来源合同项所指正文许可缺节点身份"))?;
            if node != claim.node {
                return Err(fail("来源合同项与所指正文块不是同一节点"));
            }
            if SourceClaim::on_len(len)?.bytes != claim.amount {
                return Err(fail("来源合同额与所指正文块不符"));
            }
            out.claims += 1;
            out.claim_amount = out
                .claim_amount
                .checked_add(claim.amount)
                .ok_or_else(|| fail("来源合同额溢出"))?;
            out.backing_bytes = out
                .backing_bytes
                .checked_add(body.bytes)
                .ok_or_else(|| fail("来源合同额溢出"))?;
        }
        if physical
            .values()
            .any(|e| e.role == CapturePermitRole::NodeBody && !backed.contains_key(&e.id))
        {
            return Err(fail("已物理计费的正文块缺来源合同项"));
        }
        Ok(out)
    }

    #[cfg(feature = "s_capture_native_testonly")]
    fn visit_node_source_claims(
        &self,
        mut visit: impl FnMut(&NodeDigest, &NativeNode, usize) -> Result<(), CodecError>,
    ) -> Result<(), CodecError> {
        let CaptureNodes::Native(native) = &self.nodes else {
            return Ok(());
        };
        for (digest, node) in native.nodes.iter() {
            let Some(claim) = node.source_claim.as_ref() else {
                continue;
            };
            let backed = SourceClaim::on_body(&node.body)?;
            if backed.bytes != claim.bytes {
                return Err(CodecError::scratch(
                    capture_allocation::scratch::ScratchFailure::Ledger,
                ));
            }
            visit(digest, node, claim.bytes)?;
        }
        Ok(())
    }

    /// #1488 P2-A：逐节点复核来源合同项——每项须指到同节点已物理计费（Native绑定）的正文块，
    /// 且合同额=正文+97；指不到即拒。返回（合同项数，合同额和）；无项的节点不计。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn native_source_claims_for_testonly(&self) -> Result<(usize, usize), CodecError> {
        let mut acc = (0usize, 0usize);
        self.visit_node_source_claims(|_, _, bytes| {
            acc = (
                acc.0 + 1,
                acc.1
                    .checked_add(bytes)
                    .ok_or_else(|| CodecError::new(code::SOURCE, "来源合同额溢出"))?,
            );
            Ok(())
        })?;
        Ok(acc)
    }
    /// #1488 P3-β：正文块已物理计费（Native绑定）的节点数；与合同项数对拍，漏签claim即可检出。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn native_bound_body_nodes_for_testonly(&self) -> usize {
        let CaptureNodes::Native(native) = &self.nodes else {
            return 0;
        };
        native
            .nodes
            .values()
            .filter(|node| node.body.is_native_bound())
            .count()
    }
    /// #1488 P3-α：逐节点来源合同额（多重集），供普查实测“与合同额同额、却无持有者的物理许可”张数。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn native_source_claim_amounts_for_testonly(&self) -> Vec<usize> {
        let CaptureNodes::Native(native) = &self.nodes else {
            return Vec::new();
        };
        native
            .nodes
            .values()
            .filter_map(|node| node.source_claim.as_ref().map(|claim| claim.bytes))
            .collect()
    }
    /// #1488 P2-β：索引许可布局实测——（索引许可字节，现槽宽=size_of<(NodeDigest,NativeNode)>+1，
    /// P2-A前槽宽：同布局镜像但`source_permit: Option<NativeReservation>`）。索引许可=槽数×槽宽，
    /// 故P2-A的布局连带降幅=槽数×(旧槽宽−现槽宽)。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn native_index_layout_for_testonly(&self) -> Option<(usize, usize, usize)> {
        #[allow(dead_code)]
        struct NativeNodeBeforeP2a {
            tag: u8,
            body: CodecVec<u8>,
            source_permit: Option<capture_allocation::scratch::NativeReservation>,
            proof: std::cell::RefCell<
                Option<capture_allocation::native_owned::NativeBox<NativeFrameProof>>,
            >,
            owner_dirty: std::cell::Cell<bool>,
            owner_next: std::cell::Cell<Option<NodeDigest>>,
        }
        let CaptureNodes::Native(native) = &self.nodes else {
            return None;
        };
        Some((
            native
                .index_permit
                .as_ref()
                .map_or(0, capture_allocation::scratch::NativeReservation::bytes),
            std::mem::size_of::<(NodeDigest, NativeNode)>() + 1,
            std::mem::size_of::<(NodeDigest, NativeNodeBeforeP2a)>() + 1,
        ))
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn native_retention_numbers_for_testonly(&self) -> Option<NativeCaptureRetention> {
        let CaptureNodes::Native(native) = &self.nodes else {
            return None;
        };
        let roots = self.codec_roots.borrow();
        Some(NativeCaptureRetention {
            nodes: native.nodes.len(),
            body_len: native.body_len,
            body_capacity: native.body_capacity,
            index_capacity: native.nodes.capacity(),
            index_payload: native
                .nodes
                .capacity()
                .saturating_mul(std::mem::size_of::<(NodeDigest, NativeNode)>()),
            public_roots: roots.public.len(),
            template_roots: roots.template.len(),
            public_capacity: roots.public.capacity(),
            template_capacity: roots.template.capacity(),
        })
    }
    pub fn retention(&self) -> serde_json::Value {
        let memo = |m: &AuditMemo| {
            serde_json::json!({
            "seen_capacity":m.seen.capacity(),"pushed_capacity":m.pushed.capacity(),
            "todo_vec_capacity":m.todo.capacity(),"read_digests_capacity":m.read_digests.capacity(),
            "bytes_memo_capacity":m.bytes_memo.capacity(),"elem_memo_capacity":m.elem_memo.capacity(),
            "field_memo_capacity":m.field_memo.capacity(),"key_cmp_capacity":m.key_cmp.capacity(),
            "todo_vec_capacity_bytes":m.todo.capacity()*std::mem::size_of::<(NodeDigest,Flavor)>(),
            "retained_string_capacity_bytes":0,
            "map_set_capacity_payload_bytes":m.seen.capacity()*std::mem::size_of::<(NodeDigest,Flavor)>()
                +m.pushed.capacity()*std::mem::size_of::<(NodeDigest,Flavor)>()
                +m.read_digests.capacity()*std::mem::size_of::<NodeDigest>()
                +m.bytes_memo.capacity()*std::mem::size_of::<((NodeDigest,Utf8State,NumState),ByteAgg)>()
                +m.elem_memo.capacity()*std::mem::size_of::<((NodeDigest,Flavor),u64)>()
                +m.field_memo.capacity()*std::mem::size_of::<((NodeDigest,Flavor),FieldAgg)>()
                +m.key_cmp.capacity()*std::mem::size_of::<((NodeDigest,NodeDigest),std::cmp::Ordering)>()})
        };
        let (nodes, body_len, body_capacity, allocation) = match &self.nodes {
            CaptureNodes::Legacy(nodes) => (
                nodes.len(),
                nodes.values().map(|(_, b)| b.len()).sum::<usize>(),
                nodes.values().map(|(_, b)| b.capacity()).sum::<usize>(),
                serde_json::Value::Null,
            ),
            #[cfg(feature = "s_capture_native_testonly")]
            CaptureNodes::Native(native) => (
                native.nodes.len(),
                native.nodes.values().map(|n| n.body.len()).sum::<usize>(),
                native
                    .nodes
                    .values()
                    .map(|n| n.body.capacity())
                    .sum::<usize>(),
                serde_json::Value::Null,
            ),
            CaptureNodes::AllocationPrototype(slab) => (
                slab.len(),
                slab.body_bytes(),
                0,
                serde_json::json!({"meter":slab.snapshot(),"body_arena_bytes":slab.body_bytes(),
                    "whole_audit_admitted":false,"uncovered":["Verifier","memo","steps/aggs","NodeRead clone","overlay","SQLite","allocator overhead","RSS"]}),
            ),
        };
        let native_frame_reuse = serde_json::json!({
            "enabled":false,"retained_entries":0,"retained_body_vec_capacity_bytes":0,
            "entry_limit":0,"body_capacity_limit_bytes":0,"same_material_hits":0,
            "first_f_calls":0,"changed_f_calls":0,"strict_f_calls":0,"failed_f_calls":0,
            "retention_skips":0
        });
        #[cfg(feature = "s_capture_native_testonly")]
        let native_frame_reuse = match &self.nodes {
            CaptureNodes::Native(native) => {
                let m = native.memo.borrow();
                serde_json::json!({
                    "enabled":true,"retained_entries":m.retained_entries,
                    "retained_body_vec_capacity_bytes":m.retained_body_capacity,
                    "entry_limit":m.entry_limit,"body_capacity_limit_bytes":m.body_capacity_limit,
                    "same_material_hits":m.same_material_hits,"first_f_calls":m.first_f_calls,
                    "changed_f_calls":m.changed_f_calls,"strict_f_calls":m.strict_f_calls,
                    "failed_f_calls":m.failed_f_calls,"retention_skips":m.retention_skips,
                    "source_index_capacity":native.nodes.capacity(),
                    "source_index_capacity_payload_bytes":native.nodes.capacity().saturating_mul(std::mem::size_of::<(NodeDigest,NativeNode)>()),
                    "proof_slot_bytes_per_entry":std::mem::size_of::<std::cell::RefCell<Option<capture_allocation::native_owned::NativeBox<NativeFrameProof>>>>(),
                    "retained_proof_box_payload_bytes":m.retained_entries.saturating_mul(std::mem::size_of::<NativeFrameProof>())
                })
            }
            _ => native_frame_reuse,
        };
        #[cfg(feature = "s_capture_native_testonly")]
        let codec_root_certificates = {
            let r = self.codec_roots.borrow();
            serde_json::json!({"format":r.format,"predicate_revision":r.predicate_revision,
                "flavor":"Value","public_mode":"Public","template_mode":"Template",
                "public_roots":r.public.len(),"template_roots":r.template.len(),
                "public_capacity":r.public.capacity(),"template_capacity":r.template.capacity()})
        };
        #[cfg(not(feature = "s_capture_native_testonly"))]
        let codec_root_certificates = serde_json::Value::Null;
        serde_json::json!({"captured_nodes":nodes,"body_len_bytes":body_len,
            "body_vec_capacity_bytes":body_capacity,"capture_allocation_prototype":allocation,
            "capture_string_capacity_bytes":0,"capture_reads":self.reads.get(),
            "native_frame_reuse":native_frame_reuse,
            "native_codec_root_certificates":codec_root_certificates,
            "public":memo(&self.public.borrow()),"template":memo(&self.template.borrow()),
            "scope":"retained Vec/String and map/set capacity payload; Native source index capacity payload, proof box payload and body Vec actual capacity; excludes BTree allocation, HashMap control bytes, allocator overhead, transient reads, total heap/RSS/IO peak; no resource eligibility"})
    }
}
struct CaptureReader<'a>(&'a AuditCapture);

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod codec_root_success_tests {
    use super::*;

    fn native(nodes: &MemoryNodes) -> AuditCapture {
        AuditCapture::new(nodes.clone())
            .enable_native_frame_reuse_for_testonly()
            .unwrap()
    }
    fn fixture() -> (MemoryNodes, ContentRef, ContentRef) {
        let mut nodes = MemoryNodes::new();
        let public = write_value(
            &mut nodes,
            &Value::Object(vec![
                ("a".into(), Value::String("正常正文".repeat(1600))),
                (
                    "b".into(),
                    Value::Array(vec![Value::Bool(true), Value::Null]),
                ),
            ]),
        )
        .unwrap()
        .content_ref();
        let template = write_template(
            &mut nodes,
            &Value::Object(vec![
                ("a".into(), Value::String("正常模板".repeat(1200))),
                ("batch".into(), Value::BatchSelf),
            ]),
        )
        .unwrap()
        .content_ref();
        (nodes, public, template)
    }
    fn assert_only_seen(capture: &AuditCapture, public: usize, template: usize) {
        for (slot, roots) in [(&capture.public, public), (&capture.template, template)] {
            let memo = slot.borrow();
            assert_eq!(memo.seen.len(), roots);
            assert!(memo.pushed.is_empty() && memo.todo.is_empty() && memo.read_digests.is_empty());
            assert_eq!(memo.bytes_memo.len(), 0);
            assert_eq!(memo.elem_memo.len(), 0);
            assert_eq!(memo.field_memo.len(), 0);
            assert_eq!(memo.key_cmp.len(), 0);
        }
        assert_eq!(capture.retention()["capture_reads"], 0);
        assert_eq!(
            capture.retention()["native_frame_reuse"]["retained_entries"],
            0
        );
    }
    fn verify_pair(
        warm: &AuditCapture,
        cold: &AuditCapture,
        public: &ContentRef,
        template: &ContentRef,
    ) -> (u64, u64) {
        let start = frame_calls_on_thread();
        let actual = [
            warm.verify(public, Mode::Public),
            warm.verify(template, Mode::Template),
        ];
        let warm_f = frame_calls_on_thread() - start;
        let start = frame_calls_on_thread();
        let expected = [
            cold.verify(public, Mode::Public),
            cold.verify(template, Mode::Template),
        ];
        let cold_f = frame_calls_on_thread() - start;
        assert!(actual.iter().all(Result::is_ok));
        assert_eq!(actual, expected);
        (warm_f, cold_f)
    }

    #[test]
    fn codec_root_success_reacquired_materials_seed_only_closed_roots() {
        let (nodes, public, template) = fixture();
        let old = native(&nodes);
        old.verify(&public, Mode::Public).unwrap();
        old.verify(&public, Mode::Public).unwrap();
        old.verify(&template, Mode::Template).unwrap();
        let pointer = old.original(&public.digest().unwrap()).unwrap().1.as_ptr();
        let old = old.into_originals_and_codec_roots_for_testonly().unwrap();
        assert_eq!(
            pointer,
            old.original(&public.digest().unwrap()).unwrap().1.as_ptr()
        );
        assert_only_seen(&old, 0, 0);
        let certificates = &old.retention()["native_codec_root_certificates"];
        assert_eq!(certificates["public_roots"], 1);
        assert_eq!(certificates["template_roots"], 1);
        let warm = native(&nodes);
        assert_eq!(
            warm.seed_codec_roots_from_native_capture_for_testonly(&old)
                .unwrap(),
            2
        );
        assert_only_seen(&warm, 1, 1);
        assert_eq!(warm.codec_roots.borrow().public.len(), 0);
        assert_eq!(warm.codec_roots.borrow().template.len(), 0);
        let cold = native(&nodes);
        let (warm_f, cold_f) = verify_pair(&warm, &cold, &public, &template);
        assert_eq!(warm.retention()["capture_reads"], 2);
        assert_eq!(warm_f, 2, "当前根必须仍读取并严格核Frame/header");
        assert!(cold.reads.get() > warm.reads.get());
        assert!(cold_f > warm_f);
        assert_eq!(warm.public.borrow().read_digests.len(), 1);
        assert_eq!(warm.template.borrow().read_digests.len(), 1);
        eprintln!("codec_root_success same roots=2 warm_reads={} cold_reads={} warm_f={warm_f} cold_f={cold_f}", warm.reads.get(), cold.reads.get());
    }

    #[test]
    fn codec_root_success_legal_append_and_new_roots_match_cold() {
        let (mut nodes, public, template) = fixture();
        let old = native(&nodes);
        old.verify(&public, Mode::Public).unwrap();
        old.verify(&template, Mode::Template).unwrap();
        let old = old.into_originals_and_codec_roots_for_testonly().unwrap();
        write_value(&mut nodes, &Value::String("合法独立新增节点".into())).unwrap();
        let public_new = write_value(
            &mut nodes,
            &Value::Array(vec![
                Value::edge_of_ref(&public).unwrap(),
                Value::String("新增公开值".into()),
            ]),
        )
        .unwrap()
        .content_ref();
        let template_new = write_value(
            &mut nodes,
            &Value::Array(vec![
                Value::edge_of_ref(&template).unwrap(),
                Value::String("新增模板值".into()),
            ]),
        )
        .unwrap()
        .content_ref();
        let warm = native(&nodes);
        assert_eq!(
            warm.seed_codec_roots_from_native_capture_for_testonly(&old)
                .unwrap(),
            2
        );
        assert_only_seen(&warm, 1, 1);
        assert!(!warm
            .public
            .borrow()
            .seen
            .contains(&(public_new.digest().unwrap(), Flavor::Value)));
        assert!(!warm
            .template
            .borrow()
            .seen
            .contains(&(template_new.digest().unwrap(), Flavor::Value)));
        let cold = native(&nodes);
        let (warm_f, cold_f) = verify_pair(&warm, &cold, &public_new, &template_new);
        assert!(warm.reads.get() < cold.reads.get());
        assert!(warm_f < cold_f);
        let mut warm_text = Vec::new();
        let mut cold_text = Vec::new();
        render_canonical(&mut warm.reader(), &public_new, &mut warm_text).unwrap();
        render_canonical(&mut cold.reader(), &public_new, &mut cold_text).unwrap();
        assert_eq!(warm_text, cold_text);
        let certificates = warm.codec_roots.borrow();
        assert_eq!(certificates.public.len(), 1);
        assert_eq!(certificates.template.len(), 1);
        assert!(certificates.public.contains(&public_new.digest().unwrap()));
        assert!(certificates
            .template
            .contains(&template_new.digest().unwrap()));
        eprintln!(
            "codec_root_success append old_roots=2 new_roots=2 warm_f={warm_f} cold_f={cold_f}"
        );
    }

    #[test]
    fn codec_root_success_public_template_and_json_inner_stay_separate() {
        let mut nodes = MemoryNodes::new();
        let text = "同一合法字符串".repeat(1600);
        let public = write_value(&mut nodes, &Value::String(text.clone()))
            .unwrap()
            .content_ref();
        let json = write_value(
            &mut nodes,
            &Value::JsonText(Box::new(JsonValue::String(text))),
        )
        .unwrap()
        .content_ref();
        let old = native(&nodes);
        old.verify(&public, Mode::Public).unwrap();
        let old = old.into_originals_and_codec_roots_for_testonly().unwrap();
        let warm = native(&nodes);
        assert_eq!(
            warm.seed_codec_roots_from_native_capture_for_testonly(&old)
                .unwrap(),
            1
        );
        assert_only_seen(&warm, 1, 0);
        let cold = native(&nodes);
        assert_eq!(
            warm.verify(&json, Mode::Public),
            cold.verify(&json, Mode::Public)
        );
        assert_eq!(
            warm.reads.get(),
            cold.reads.get(),
            "Value证书不得复用JsonInner判定"
        );
        assert_eq!(
            warm.verify(&public, Mode::Template),
            cold.verify(&public, Mode::Template)
        );
        assert_eq!(
            warm.reads.get(),
            cold.reads.get(),
            "Public证书不得跨入Template"
        );
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod counts_subset_codec_root_tests {
    use super::*;

    fn fixture() -> (MemoryNodes, [ContentRef; 2], [ContentRef; 3]) {
        let mut nodes = MemoryNodes::new();
        let roots = ["shared", "first", "second"].map(|name| {
            write_value(
                &mut nodes,
                &Value::Array(vec![Value::String(name.repeat(1600)), Value::Bool(true)]),
            )
            .unwrap()
            .content_ref()
        });
        let states = [[0, 0, 1], [0, 2, 2]].map(|indices| {
            write_value(
                &mut nodes,
                &Value::Object(
                    ["objects", "witnesses", "observations"]
                        .into_iter()
                        .zip(indices)
                        .map(|(name, index)| {
                            (name.into(), Value::edge_of_ref(&roots[index]).unwrap())
                        })
                        .collect(),
                ),
            )
            .unwrap()
            .content_ref()
        });
        (nodes, states, roots)
    }
    fn native(nodes: &MemoryNodes) -> AuditCapture {
        AuditCapture::new(nodes.clone())
            .enable_native_frame_reuse_for_testonly()
            .unwrap()
    }
    fn assert_cold_workspaces(capture: &AuditCapture) {
        let CaptureNodes::Native(native) = &capture.nodes else {
            panic!("正常稀疏原件测试必须保留 Native 域");
        };
        let actual_len: usize = native.nodes.values().map(|node| node.body.len()).sum();
        let actual_capacity: usize = native.nodes.values().map(|node| node.body.capacity()).sum();
        assert_eq!(native.body_len, actual_len, "裁剪后正文长度缓存须匹配实存");
        assert_eq!(
            native.body_capacity, actual_capacity,
            "裁剪后正文容量缓存须匹配实存"
        );
        assert_eq!(capture.retention()["body_len_bytes"], actual_len);
        assert_eq!(
            capture.retention()["body_vec_capacity_bytes"],
            actual_capacity
        );
        for mode in ["public", "template"] {
            assert!(capture.retention()[mode]
                .as_object()
                .unwrap()
                .values()
                .all(|v| v.as_u64() == Some(0)));
        }
        assert_eq!(
            capture.retention()["native_frame_reuse"]["retained_entries"],
            0
        );
        assert_eq!(
            capture.retention()["native_frame_reuse"]["retained_body_vec_capacity_bytes"],
            0
        );
        assert_eq!(capture.retention()["capture_reads"], 0);
        assert!(capture.codec_roots.borrow().template.is_empty());
    }

    #[test]
    fn counts_subset_codec_normal_exact_roots_dedup_fees_and_success_seed() {
        let (nodes, states, roots) = fixture();
        let capture = native(&nodes);
        // 旧collector的完整state及Template成功根必须全部清除。
        for state in &states {
            capture.verify(state, Mode::Public).unwrap();
            capture.verify(state, Mode::Template).unwrap();
        }
        let (subset, certificates, report) = capture
            .into_counts_original_subset_for_testonly(
                [(states[0].clone(), 19), (states[1].clone(), 23)],
                43,
                usize::MAX,
            )
            .unwrap();
        assert_eq!(report.accepted_groups, 2);
        assert!(certificates
            .iter()
            .all(|c| c.source_bytes().unwrap() == 428));
        let expected: std::collections::HashSet<_> =
            roots.iter().map(|r| r.digest().unwrap()).collect();
        assert_eq!(subset.codec_roots.borrow().public, expected);
        for state in &states {
            assert!(!subset
                .codec_roots
                .borrow()
                .public
                .contains(&state.digest().unwrap()));
        }
        assert_cold_workspaces(&subset);
        let retained = subset.retention();
        assert_eq!(
            retained["native_codec_root_certificates"]["public_roots"],
            3
        );
        assert!(
            retained["native_codec_root_certificates"]["public_capacity"]
                .as_u64()
                .unwrap()
                >= 3
        );
        assert_eq!(
            report.retained_source_bytes,
            43 + 19
                + 23
                + 2 * 428
                + retained["body_len_bytes"].as_u64().unwrap() as usize
                + 97 * retained["captured_nodes"].as_u64().unwrap() as usize
                + 32 * 3
        );
        let warm = native(&nodes);
        let cold = native(&nodes);
        assert_eq!(
            warm.seed_codec_roots_from_native_capture_for_testonly(&subset)
                .unwrap(),
            3
        );
        assert_eq!(warm.public.borrow().seen.len(), 3);
        assert!(warm.template.borrow().seen.is_empty());
        for root in &roots {
            warm.verify(root, Mode::Public).unwrap();
            cold.verify(root, Mode::Public).unwrap();
        }
        assert_eq!(warm.reads.get(), 3, "成功种子仍逐根严格读取当前header");
        assert!(
            cold.reads.get() > warm.reads.get(),
            "完整闭包应得到实际codec复用"
        );
    }

    #[test]
    fn counts_subset_codec_normal_low_cap_keeps_admitted_shared_roots_only() {
        let (nodes, states, roots) = fixture();
        let groups = || [(states[0].clone(), 19), (states[1].clone(), 23)];
        let (_, _, measured) = native(&nodes)
            .into_counts_original_subset_for_testonly(groups(), 43, usize::MAX)
            .unwrap();
        let cap = measured.first_admitted_source_bytes;
        let (subset, certificates, report) = native(&nodes)
            .into_counts_original_subset_for_testonly(groups(), 43, cap)
            .unwrap();
        assert_eq!(certificates.len(), 1);
        assert!(certificates[0].matches_state(&states[0]));
        assert_eq!((report.attempted_groups, report.accepted_groups), (2, 1));
        assert!(report.budget_stopped && report.rolled_back_nodes > 0);
        assert_eq!(report.retained_source_bytes, cap);
        let expected: std::collections::HashSet<_> =
            roots[..2].iter().map(|r| r.digest().unwrap()).collect();
        assert_eq!(subset.codec_roots.borrow().public, expected);
        assert!(!subset
            .codec_roots
            .borrow()
            .public
            .contains(&roots[2].digest().unwrap()));
        assert_cold_workspaces(&subset);
        let current = native(&nodes);
        assert_eq!(
            current
                .seed_codec_roots_from_native_capture_for_testonly(&subset)
                .unwrap(),
            2
        );
        assert!(!current
            .public
            .borrow()
            .seen
            .contains(&(roots[2].digest().unwrap(), Flavor::Value)));
    }

    #[test]
    fn counts_subset_codec_normal_first_group_cap_includes_unique_root_fee() {
        let (nodes, states, _) = fixture();
        let groups = || [(states[0].clone(), 19)];
        let (_, _, measured) = native(&nodes)
            .into_counts_original_subset_for_testonly(groups(), 43, usize::MAX)
            .unwrap();
        // 正常低cap能装下原件与Counts证书，但装不下两个唯一codec根。
        let cap = measured.first_admitted_source_bytes - 32 * 2;
        let (subset, certificates, report) = native(&nodes)
            .into_counts_original_subset_for_testonly(groups(), 43, cap)
            .unwrap();
        assert!(certificates.is_empty());
        assert_eq!(report.accepted_groups, 0);
        assert!(report.budget_stopped && report.rolled_back_nodes > 0);
        assert_eq!(
            report.last_attempted_source_bytes,
            measured.first_admitted_source_bytes
        );
        assert!(subset.codec_roots.borrow().public.is_empty());
        assert_eq!(subset.retention()["captured_nodes"], 0);
        assert_cold_workspaces(&subset);
        assert_eq!(
            native(&nodes)
                .seed_codec_roots_from_native_capture_for_testonly(&subset)
                .unwrap(),
            0
        );
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod source_cache_original_owner_tests {
    use super::*;
    #[test]
    fn source_cache_original_owner_clears_all_memos_without_copying_body() {
        let mut nodes = MemoryNodes::new();
        let root = write_value(
            &mut nodes,
            &Value::Object(vec![(
                "a".into(),
                Value::Array(vec![Value::String("original".into())]),
            )]),
        )
        .unwrap()
        .content_ref();
        let digest = root.digest().unwrap();
        let capture = AuditCapture::new(nodes)
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        let pointer = capture.original(&digest).unwrap().1.as_ptr();
        capture.verify(&root, Mode::Public).unwrap();
        capture.verify(&root, Mode::Template).unwrap();
        assert!(
            capture.retention()["public"]["seen_capacity"]
                .as_u64()
                .unwrap()
                > 0
        );
        let keeper = capture.into_originals_only_for_testonly().unwrap();
        assert_eq!(keeper.original(&digest).unwrap().1.as_ptr(), pointer);
        let retention = keeper.retention();
        for mode in ["public", "template"] {
            assert!(retention[mode]
                .as_object()
                .unwrap()
                .values()
                .all(|v| v.as_u64() == Some(0)));
        }
        assert_eq!(retention["native_frame_reuse"]["retained_entries"], 0);
        assert_eq!(
            retention["native_frame_reuse"]["retained_body_vec_capacity_bytes"],
            0
        );
        assert_eq!(retention["capture_reads"], 0);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn source_cache_original_owner_rejects_owned() {
        let budget = capture_allocation::OwnedBudget::new(4096);
        let slab = capture_allocation::CaptureSlab::new(0, 0, &budget).unwrap();
        let capture = AuditCapture::from_slab_for_allocation_prototype(slab);
        assert!(capture.into_originals_only_for_testonly().is_err());
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}
/// 当前调用的节点访问计量；不保存在 capture 或 memo 内。
#[doc(hidden)]
pub trait CaptureReadObserver {
    fn before_read(&self) -> Result<(), CodecError>;
    fn prepare_node(&self) -> Result<(), CodecError>;
    fn accept_prepared_frame(&self, frame: &job::CheckedFrame) -> Result<(), CodecError>;
}
struct ObservedCaptureReader<'a, 'o> {
    reader: CaptureReader<'a>,
    observer: Option<&'o dyn CaptureReadObserver>,
}
impl NodeSource for ObservedCaptureReader<'_, '_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        if let Some(observer) = self.observer {
            observer.before_read()?;
        }
        self.reader.read(digest)
    }
    fn checked_read(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<job::CheckedFrame>, CodecError> {
        self.reader.checked_observed(digest, self.observer)
    }
}
impl CaptureReader<'_> {
    fn record_read(&self, digest: &NodeDigest) {
        #[cfg(test)]
        trace_capture_body_read(digest);
        #[cfg(not(test))]
        let _ = digest;
        self.0.reads.set(self.0.reads.get().saturating_add(1));
    }
    #[cfg(feature = "s_capture_native_testonly")]
    fn checked_native_current(
        &self,
        digest: &NodeDigest,
        native: &NativeCaptureNodes,
        read_current: impl FnOnce(Option<&NativeNode>) -> Result<NodeRead, CodecError>,
    ) -> Result<Option<job::CheckedFrame>, CodecError> {
        self.record_read(digest);
        let node = native.get(digest);
        // 本次原件先实际读取/完整复制；proof 槽沿用同一次实际 get 返回的 entry。
        let current = read_current(node);
        let result = native
            .memo
            .borrow_mut()
            .checked_current(digest, node, current);
        if let Some(node) = node.filter(|node| node.proof.borrow().is_some()) {
            if !node.owner_dirty.replace(true) {
                node.owner_next
                    .set(native.owner_dirty_head.replace(Some(*digest)));
            }
        }
        result
    }
}
impl NodeSource for CaptureReader<'_> {
    fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.record_read(d);
        Ok(match self.0.original(d) {
            Some((t, b)) => NodeRead::Found {
                tag: t,
                body: b.to_vec(),
            },
            None => NodeRead::Missing,
        })
    }
    fn checked_read(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<job::CheckedFrame>, CodecError> {
        self.checked_observed(digest, None)
    }
}
impl CaptureReader<'_> {
    fn checked_observed(
        &mut self,
        digest: &NodeDigest,
        observer: Option<&dyn CaptureReadObserver>,
    ) -> Result<Option<job::CheckedFrame>, CodecError> {
        if let Some(observer) = observer {
            observer.before_read()?;
        }
        let result = self.checked_observed_inner(digest, observer)?;
        if let (Some(observer), Some(frame)) = (observer, result.as_ref()) {
            observer.accept_prepared_frame(frame)?;
        }
        Ok(result)
    }
    fn checked_observed_inner(
        &mut self,
        digest: &NodeDigest,
        observer: Option<&dyn CaptureReadObserver>,
    ) -> Result<Option<job::CheckedFrame>, CodecError> {
        #[cfg(feature = "s_capture_native_testonly")]
        if let CaptureNodes::Native(native) = &self.0.nodes {
            return self.checked_native_current(digest, native, |node| {
                let read = match node {
                    Some(node) => NodeRead::Found {
                        tag: node.tag,
                        body: node.body.to_vec(),
                    },
                    None => NodeRead::Missing,
                };
                if matches!(&read, NodeRead::Found { .. }) {
                    if let Some(observer) = observer {
                        observer.prepare_node()?;
                    }
                }
                Ok(read)
            });
        }
        match self.read(digest)? {
            NodeRead::Missing => Ok(None),
            NodeRead::Found { tag, body } => {
                if let Some(observer) = observer {
                    observer.prepare_node()?;
                }
                job::CheckedFrame::checked(digest, tag, body).map(Some)
            }
        }
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod native_frame_reuse_tests {
    use super::*;

    #[derive(Default)]
    struct ReadMeter {
        reads: std::cell::Cell<usize>,
        prepared: std::cell::Cell<usize>,
        accepted: std::cell::Cell<usize>,
        stop: std::cell::Cell<bool>,
    }
    impl CaptureReadObserver for ReadMeter {
        fn before_read(&self) -> Result<(), CodecError> {
            self.reads.set(self.reads.get() + 1);
            Ok(())
        }
        fn prepare_node(&self) -> Result<(), CodecError> {
            self.prepared.set(self.prepared.get() + 1);
            if self.stop.get() {
                return Err(CodecError::new("test_stop", "stop"));
            }
            Ok(())
        }
        fn accept_prepared_frame(&self, _: &job::CheckedFrame) -> Result<(), CodecError> {
            self.accepted.set(self.accepted.get() + 1);
            Ok(())
        }
    }
    #[test]
    fn measured_capture_preserves_memo_and_observes_only_actual_reads() {
        let mut nodes = MemoryNodes::new();
        let built = write_value(&mut nodes, &Value::String("原件".into())).unwrap();
        let capture = AuditCapture::new(nodes)
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        let reference = built.content_ref();
        let meter = ReadMeter::default();
        capture
            .verify_refview_observed((&reference).into(), Mode::Public, Some(&meter))
            .unwrap();
        assert!(meter.accepted.get() > 0);
        assert_eq!(meter.prepared.get(), meter.accepted.get());
        let reads = capture.reads.get();
        let observed = meter.reads.get();
        capture
            .verify_refview_observed((&reference).into(), Mode::Public, Some(&meter))
            .unwrap();
        assert_eq!(
            capture.reads.get() - reads,
            (meter.reads.get() - observed) as u64
        );
        let missing = [255; 32];
        let prepared = meter.prepared.get();
        assert!(capture
            .reader_observed(&meter)
            .checked_read(&missing)
            .unwrap()
            .is_none());
        assert_eq!(meter.prepared.get(), prepared);
        meter.stop.set(true);
        let accepted = meter.accepted.get();
        assert!(capture
            .reader_observed(&meter)
            .checked_read(&built.digest)
            .is_err());
        assert_eq!(meter.accepted.get(), accepted);
        meter.stop.set(false);
        capture
            .reader_observed(&meter)
            .checked_read(&built.digest)
            .unwrap()
            .unwrap();
        assert_eq!(meter.accepted.get(), accepted + 1);
    }

    fn capture_node(tag: u8, body: &[u8]) -> (AuditCapture, MemoryNodes, NodeDigest) {
        let digest = frame(tag, body);
        let mut nodes = MemoryNodes::new();
        nodes.insert_unchecked(digest, tag, body.to_vec());
        let capture = AuditCapture::new(nodes.clone())
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        (capture, nodes, digest)
    }
    fn native(capture: &AuditCapture) -> &NativeCaptureNodes {
        let CaptureNodes::Native(nodes) = &capture.nodes else {
            panic!("test Native source")
        };
        nodes
    }
    fn native_mut(capture: &mut AuditCapture) -> &mut NativeCaptureNodes {
        let CaptureNodes::Native(nodes) = &mut capture.nodes else {
            panic!("test Native source")
        };
        nodes
    }
    fn replace_original(capture: &mut AuditCapture, digest: NodeDigest, tag: u8, body: Vec<u8>) {
        let nodes = &mut native_mut(capture).nodes;
        if let Some(node) = nodes.get_mut(&digest) {
            node.tag = tag;
            node.body = CodecVec::from_plain(body);
        } else {
            nodes.insert(digest, NativeNode::new(tag, body));
        }
    }
    fn stats(capture: &AuditCapture) -> serde_json::Value {
        capture.retention()["native_frame_reuse"].clone()
    }
    fn read(
        capture: &AuditCapture,
        digest: &NodeDigest,
    ) -> Result<Option<job::CheckedFrame>, CodecError> {
        capture.reader().checked_read(digest)
    }
    fn strict_result(
        nodes: &mut MemoryNodes,
        digest: &NodeDigest,
    ) -> Result<Option<job::CheckedFrame>, CodecError> {
        // 未覆写 checked_read 的 MemoryNodes 是同次原实现 oracle。
        nodes.checked_read(digest)
    }
    #[test]
    fn owner_normal_append_counters_and_dirty_proofs_match_full_oracle() {
        let mut nodes = MemoryNodes::new();
        let first = write_value(&mut nodes, &Value::String("正常原件é".into())).unwrap();
        let mut capture = AuditCapture::new(nodes)
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        capture.assert_owner_retention_oracle();
        let pointer = capture.original(&first.digest).unwrap().1.as_ptr();
        capture.verify(&first.content_ref(), Mode::Public).unwrap();
        assert!(native(&capture).owner_dirty_head.get().is_some());
        capture = capture.finish_owner_step_for_testonly().unwrap();
        assert!(native(&capture).owner_dirty_head.get().is_none());
        assert_eq!(native(&capture).memo.borrow().retained_entries, 0);
        assert!(native(&capture)
            .nodes
            .values()
            .all(|n| n.proof.borrow().is_none()));
        let old = capture.native_retention_numbers_for_testonly().unwrap();
        let body = "第二个合法字节块".as_bytes().to_vec();
        let digest = frame(tag::BYTES_CHUNK, &body);
        let len = body.len();
        let cap = body.capacity();
        assert!(capture
            .append_original_for_owner_testonly(digest, tag::BYTES_CHUNK, body, usize::MAX, None)
            .unwrap());
        capture.assert_owner_retention_oracle();
        let new = capture.native_retention_numbers_for_testonly().unwrap();
        assert_eq!(new.body_len, old.body_len + len);
        assert_eq!(new.body_capacity, old.body_capacity + cap);
        assert_eq!(capture.original(&first.digest).unwrap().1.as_ptr(), pointer);
        capture.reader().checked_read(&digest).unwrap().unwrap();
        capture = capture.finish_owner_step_for_testonly().unwrap();
        assert_eq!(native(&capture).memo.borrow().retained_entries, 0);
        capture.assert_owner_retention_oracle();
        assert_eq!(capture.original(&first.digest).unwrap().1.as_ptr(), pointer);
    }

    #[test]
    fn native_single_index_optin_moves_original_buffers_and_keeps_default_representation() {
        let mut body = Vec::with_capacity(128);
        body.extend_from_slice(b"material");
        let pointer = body.as_ptr();
        let capacity = body.capacity();
        let digest = frame(tag::BYTES_CHUNK, &body);
        let mut nodes = MemoryNodes::new();
        nodes.insert_unchecked(digest, tag::BYTES_CHUNK, body);
        let capture = AuditCapture::new(nodes);
        assert!(matches!(&capture.nodes, CaptureNodes::Legacy(_)));
        assert!(!capture.is_allocation_prototype());
        let capture = capture.enable_native_frame_reuse_for_testonly().unwrap();
        assert!(!capture.is_allocation_prototype());
        let node = native(&capture).nodes.get(&digest).unwrap();
        assert_eq!(node.body.as_ptr(), pointer);
        assert_eq!(node.body.capacity(), capacity);
        assert!(node.proof.borrow().is_none());
        assert_eq!(
            capture.original(&digest),
            Some((tag::BYTES_CHUNK, b"material".as_slice()))
        );
        assert!(capture.contains(&digest));
        assert_eq!(stats(&capture)["entry_limit"], 1);
        assert_eq!(stats(&capture)["body_capacity_limit_bytes"], 8);
        assert_eq!(capture.retention()["body_vec_capacity_bytes"], capacity);
    }
    #[test]
    fn native_single_index_repeat_optin_preserves_original_and_semantic_memos_but_cools_proof() {
        let mut body = Vec::with_capacity(128);
        body.extend_from_slice(b"material");
        let pointer = body.as_ptr();
        let capacity = body.capacity();
        let digest = frame(tag::BYTES_CHUNK, &body);
        let mut nodes = MemoryNodes::new();
        nodes.insert_unchecked(digest, tag::BYTES_CHUNK, body);
        let null = write_value(&mut nodes, &Value::Null).unwrap().content_ref();
        let mut oracle = nodes.clone();
        let expected = strict_result(&mut oracle, &digest)
            .unwrap()
            .unwrap()
            .into_parts();
        let capture = AuditCapture::new(nodes)
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        for mode in [Mode::Public, Mode::Template] {
            capture.verify(&null, mode).unwrap();
        }
        assert_eq!(
            read(&capture, &digest).unwrap().unwrap().into_parts(),
            expected
        );
        let before = capture.retention();
        assert_eq!(before["native_frame_reuse"]["retained_entries"], 2);
        let source_capacity = native(&capture).nodes.capacity();
        let capture = capture.enable_native_frame_reuse_for_testonly().unwrap();
        let after = capture.retention();
        assert_eq!(after["public"], before["public"]);
        assert_eq!(after["template"], before["template"]);
        assert_eq!(after["capture_reads"], before["capture_reads"]);
        assert_eq!(native(&capture).nodes.capacity(), source_capacity);
        let node = native(&capture).nodes.get(&digest).unwrap();
        assert_eq!(node.body.as_ptr(), pointer);
        assert_eq!(node.body.capacity(), capacity);
        assert!(native(&capture)
            .nodes
            .values()
            .all(|n| n.proof.borrow().is_none()));
        assert_eq!(stats(&capture)["retained_entries"], 0);
        assert_eq!(stats(&capture)["retained_body_vec_capacity_bytes"], 0);
        assert_eq!(stats(&capture)["strict_f_calls"], 0);
        assert_eq!(stats(&capture)["same_material_hits"], 0);
        assert_eq!(stats(&capture)["entry_limit"], 2);
        assert_eq!(stats(&capture)["body_capacity_limit_bytes"], 8);
        let start = frame_calls_on_thread();
        assert_eq!(
            read(&capture, &digest).unwrap().unwrap().into_parts(),
            expected
        );
        assert_eq!(frame_calls_on_thread() - start, 1);
        assert_eq!(stats(&capture)["first_f_calls"], 1);
        assert_eq!(stats(&capture)["same_material_hits"], 0);
        assert_eq!(
            capture.reads.get(),
            before["capture_reads"].as_u64().unwrap() + 1
        );
    }
    #[test]
    fn native_single_index_checked_read_uses_one_actual_lookup_for_original_and_proof() {
        let (capture, _, digest) = capture_node(tag::BYTES_CHUNK, b"material");
        let nodes = native(&capture);
        let start = frame_calls_on_thread();
        for expected in 1..=4 {
            let current = read(&capture, &digest).unwrap().unwrap();
            assert_eq!(current.digest(), &digest);
            assert_eq!(current.tag(), tag::BYTES_CHUNK);
            assert_eq!(current.body(), b"material");
            let node = nodes.nodes.get(&digest).unwrap();
            assert_ne!(current.body().as_ptr(), node.body.as_ptr());
            assert_ne!(
                current.body().as_ptr(),
                node.proof.borrow().as_ref().unwrap().frame.body().as_ptr()
            );
            assert_eq!(nodes.index_lookups.get(), expected);
            assert_eq!(capture.reads.get(), expected);
        }
        assert_eq!(frame_calls_on_thread() - start, 1);
        assert_eq!(stats(&capture)["same_material_hits"], 3);
        assert_eq!(stats(&capture)["retained_entries"], 1);
        let missing = frame(tag::NULL, b"");
        assert!(read(&capture, &missing).unwrap().is_none());
        assert_eq!(nodes.index_lookups.get(), 5);
        assert_eq!(capture.reads.get(), 5);
        assert_eq!(stats(&capture)["retained_entries"], 1);
        assert_eq!(stats(&capture)["strict_f_calls"], 1);
    }
    #[test]
    fn native_single_index_rebind_returns_the_complete_current_vec_and_capacity() {
        let (capture, _, digest) = capture_node(tag::BYTES_CHUNK, b"material");
        read(&capture, &digest).unwrap().unwrap();
        let mut current = Vec::with_capacity(128);
        current.extend_from_slice(b"material");
        let pointer = current.as_ptr();
        let capacity = current.capacity();
        let start = frame_calls_on_thread();
        let rebound = CaptureReader(&capture)
            .checked_native_current(&digest, native(&capture), |_| {
                Ok(NodeRead::Found {
                    tag: tag::BYTES_CHUNK,
                    body: current,
                })
            })
            .unwrap()
            .unwrap();
        let (_, body) = rebound.into_parts();
        assert_eq!(body.as_ptr(), pointer);
        assert_eq!(body.capacity(), capacity);
        assert_eq!(body, b"material");
        assert_eq!(frame_calls_on_thread(), start);
        assert_eq!(stats(&capture)["same_material_hits"], 1);
        assert_eq!(stats(&capture)["retained_body_vec_capacity_bytes"], 8);
        assert_eq!(native(&capture).index_lookups.get(), 2);
    }
    #[test]
    fn native_single_index_capacity_skip_cleanup_and_empty_proof_counts_stay_symmetric() {
        let (capture, _, digest) = capture_node(tag::BYTES_CHUNK, b"material");
        let nodes = native(&capture);
        let node = nodes.nodes.get(&digest).unwrap();
        let frame =
            job::CheckedFrame::checked(&digest, tag::BYTES_CHUNK, b"material".to_vec()).unwrap();
        let mut oversized = Vec::with_capacity(64);
        oversized.extend_from_slice(frame.body());
        nodes
            .memo
            .borrow_mut()
            .retain_copy(&mut node.proof.borrow_mut(), &frame, oversized);
        assert_eq!(stats(&capture)["retention_skips"], 1);
        assert_eq!(stats(&capture)["retained_entries"], 0);
        assert_eq!(stats(&capture)["retained_body_vec_capacity_bytes"], 0);
        for _ in 0..2 {
            read(&capture, &digest).unwrap().unwrap();
            assert_eq!(stats(&capture)["retained_entries"], 1);
            assert_eq!(stats(&capture)["retained_body_vec_capacity_bytes"], 8);
            for _ in 0..2 {
                assert!(CaptureReader(&capture)
                    .checked_native_current(&digest, nodes, |_| Ok(NodeRead::Missing))
                    .unwrap()
                    .is_none());
                assert_eq!(stats(&capture)["retained_entries"], 0);
                assert_eq!(stats(&capture)["retained_body_vec_capacity_bytes"], 0);
            }
        }
        assert_eq!(stats(&capture)["first_f_calls"], 2);
        assert_eq!(stats(&capture)["same_material_hits"], 0);
        assert_eq!(stats(&capture)["retention_skips"], 1);
        let (empty, _, digest) = capture_node(tag::NULL, b"");
        read(&empty, &digest).unwrap().unwrap();
        assert_eq!(stats(&empty)["retained_entries"], 1);
        assert_eq!(stats(&empty)["retained_body_vec_capacity_bytes"], 0);
        assert!(CaptureReader(&empty)
            .checked_native_current(&digest, native(&empty), |_| Ok(NodeRead::Missing))
            .unwrap()
            .is_none());
        assert_eq!(stats(&empty)["retained_entries"], 0);
        assert_eq!(stats(&empty)["retained_body_vec_capacity_bytes"], 0);
        read(&empty, &digest).unwrap().unwrap();
        assert_eq!(stats(&empty)["first_f_calls"], 2);
        assert_eq!(stats(&empty)["retained_entries"], 1);
    }
    #[test]
    fn native_frame_reuse_rebinds_f_but_repeats_actual_read_across_readers() {
        let (capture, mut oracle, digest) = capture_node(tag::BYTES_CHUNK, b"material");
        let expected = strict_result(&mut oracle, &digest)
            .unwrap()
            .unwrap()
            .into_parts();
        let start = frame_calls_on_thread();
        CAPTURE_BODY_READ_TRACE.with(|t| *t.borrow_mut() = Some((start, Vec::new())));
        for _ in 0..4 {
            assert_eq!(
                read(&capture, &digest).unwrap().unwrap().into_parts(),
                expected
            );
        }
        let trace = CAPTURE_BODY_READ_TRACE.with(|t| t.borrow_mut().take().unwrap().1);
        assert_eq!(
            trace,
            vec![(digest, 0), (digest, 1), (digest, 1), (digest, 1)]
        );
        assert_eq!(frame_calls_on_thread() - start, 1);
        assert_eq!(capture.reads.get(), 4);
        assert_eq!(stats(&capture)["first_f_calls"], 1);
        assert_eq!(stats(&capture)["same_material_hits"], 3);
        assert_eq!(stats(&capture)["retained_entries"], 1);
        assert_eq!(stats(&capture)["retained_body_vec_capacity_bytes"], 8);
    }
    #[test]
    fn native_frame_reuse_digest_tag_and_body_changes_require_current_f() {
        for change in 0..3 {
            let (mut capture, mut oracle, digest) = capture_node(tag::BYTES_CHUNK, b"material");
            read(&capture, &digest).unwrap().unwrap();
            let requested = if change == 0 {
                let other = frame(tag::BYTES_CHUNK, b"other");
                // 模拟摘要键关联错位，证明本身仍携带原请求，不能凭map命中复用。
                let nodes = &mut native_mut(&mut capture).nodes;
                let old = nodes.remove(&digest).unwrap();
                nodes.insert(other, old);
                oracle.insert_unchecked(other, tag::BYTES_CHUNK, b"material".to_vec());
                other
            } else {
                let (tag, body) = if change == 1 {
                    (tag::STRING, b"material".to_vec())
                } else {
                    (tag::BYTES_CHUNK, b"changed!".to_vec())
                };
                replace_original(&mut capture, digest, tag, body.clone());
                oracle.insert_unchecked(digest, tag, body);
                digest
            };
            let expected = strict_result(&mut oracle, &requested).unwrap_err();
            let start = frame_calls_on_thread();
            assert_eq!(read(&capture, &requested).unwrap_err(), expected);
            assert_eq!(frame_calls_on_thread() - start, 1);
            assert_eq!(stats(&capture)["same_material_hits"], 0);
            assert_eq!(stats(&capture)["changed_f_calls"], 1);
            assert_eq!(stats(&capture)["retained_entries"], 0);
            assert_eq!(stats(&capture)["retained_body_vec_capacity_bytes"], 0);
        }
    }
    #[test]
    fn native_frame_reuse_bad_first_frame_never_becomes_success_cache() {
        let (mut capture, mut oracle, digest) = capture_node(tag::BYTES_CHUNK, b"material");
        replace_original(&mut capture, digest, tag::BYTES_CHUNK, b"corrupt".to_vec());
        oracle.insert_unchecked(digest, tag::BYTES_CHUNK, b"corrupt".to_vec());
        let expected = strict_result(&mut oracle, &digest).unwrap_err();
        let start = frame_calls_on_thread();
        for _ in 0..2 {
            assert_eq!(read(&capture, &digest).unwrap_err(), expected);
        }
        assert_eq!(frame_calls_on_thread() - start, 2);
        assert_eq!(stats(&capture)["failed_f_calls"], 2);
        assert_eq!(stats(&capture)["retained_entries"], 0);
        replace_original(&mut capture, digest, tag::BYTES_CHUNK, b"material".to_vec());
        read(&capture, &digest).unwrap().unwrap();
        assert_eq!(frame_calls_on_thread() - start, 3);
        assert_eq!(stats(&capture)["retained_entries"], 1);
    }
    #[test]
    fn native_frame_reuse_missing_and_read_error_evict_without_old_success() {
        let (mut capture, _, digest) = capture_node(tag::BYTES_CHUNK, b"material");
        read(&capture, &digest).unwrap().unwrap();
        let start = frame_calls_on_thread();
        // 不可变生产原件无 Missing/Err 来源；在同一次实际 get 后模拟读取失败。
        assert!(CaptureReader(&capture)
            .checked_native_current(&digest, native(&capture), |_| Ok(NodeRead::Missing))
            .unwrap()
            .is_none());
        assert_eq!(frame_calls_on_thread(), start);
        assert_eq!(capture.reads.get(), 2);
        assert_eq!(stats(&capture)["retained_entries"], 0);
        replace_original(&mut capture, digest, tag::BYTES_CHUNK, b"material".to_vec());
        read(&capture, &digest).unwrap().unwrap();
        let failure = CodecError::new(code::SOURCE, "exact original read error");
        // 同一次实际 get 后的 Err 必须驱逐该 entry 的旧证明，保留原错误。
        let start = frame_calls_on_thread();
        assert_eq!(
            CaptureReader(&capture)
                .checked_native_current(&digest, native(&capture), |_| Err(failure.clone()))
                .unwrap_err(),
            failure
        );
        assert_eq!(frame_calls_on_thread(), start);
        assert_eq!(stats(&capture)["retained_entries"], 0);
        assert_eq!(stats(&capture)["retained_body_vec_capacity_bytes"], 0);
        read(&capture, &digest).unwrap().unwrap();
        assert_eq!(frame_calls_on_thread() - start, 1);
        assert_eq!(stats(&capture)["same_material_hits"], 0);
    }
    #[test]
    fn native_frame_reuse_warm_template_still_rejects_public_jsoninner_and_declarations() {
        let mut nodes = MemoryNodes::new();
        let placeholder = write_template(&mut nodes, &Value::BatchSelf).unwrap();
        let inner = write_value(&mut nodes, &Value::JsonText(Box::new(JsonValue::Null))).unwrap();
        let outer_digest = frame(tag::JSON_TEXT, &inner.digest);
        nodes.insert_unchecked(outer_digest, tag::JSON_TEXT, inner.digest.to_vec());
        let outer = ContentRef::new(Kind::JsonText, &outer_digest, None);
        let capture = AuditCapture::new(nodes.clone())
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        let oracle = AuditCapture::new(nodes);
        for reference in [placeholder.content_ref(), inner.content_ref()] {
            assert_eq!(
                capture.verify(&reference, Mode::Template),
                oracle.verify(&reference, Mode::Template)
            );
        }
        for (reference, mode) in [
            (placeholder.content_ref(), Mode::Public),
            (outer, Mode::Template),
        ] {
            let expected = oracle.verify(&reference, mode).unwrap_err();
            assert_eq!(capture.verify(&reference, mode).unwrap_err(), expected);
        }
        let mut declaration = inner.content_ref();
        declaration.kind = Kind::Array;
        assert_eq!(
            capture.verify(&declaration, Mode::Template).unwrap_err(),
            oracle.verify(&declaration, Mode::Template).unwrap_err()
        );
        declaration = inner.content_ref();
        declaration.extent = Some(99);
        assert_eq!(
            capture.verify(&declaration, Mode::Template).unwrap_err(),
            oracle.verify(&declaration, Mode::Template).unwrap_err()
        );
        assert!(stats(&capture)["same_material_hits"].as_u64().unwrap() > 0);
    }
    #[test]
    fn native_frame_reuse_new_capture_default_and_owned_have_no_old_proof() {
        let (capture, nodes, digest) = capture_node(tag::BYTES_CHUNK, b"material");
        read(&capture, &digest).unwrap().unwrap();
        let fresh = AuditCapture::new(nodes.clone())
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        let start = frame_calls_on_thread();
        read(&fresh, &digest).unwrap().unwrap();
        assert_eq!(frame_calls_on_thread() - start, 1);
        assert_eq!(stats(&fresh)["same_material_hits"], 0);
        let plain = AuditCapture::new(nodes);
        let start = frame_calls_on_thread();
        for _ in 0..2 {
            read(&plain, &digest).unwrap().unwrap();
        }
        assert_eq!(frame_calls_on_thread() - start, 2);
        assert_eq!(stats(&plain)["enabled"], false);
        assert_eq!(stats(&plain)["strict_f_calls"], 0);
        let bytes = b"material";
        let budget = capture_allocation::OwnedBudget::new(
            capture_allocation::CaptureSlab::requested_bytes(1, bytes.len()).unwrap(),
        );
        let mut slab = capture_allocation::CaptureSlab::new(1, bytes.len(), &budget).unwrap();
        slab.push(digest, tag::BYTES_CHUNK, bytes).unwrap();
        let owned = AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        assert!(matches!(&owned.nodes, CaptureNodes::AllocationPrototype(_)));
        assert!(owned.is_allocation_prototype());
        let start = frame_calls_on_thread();
        for _ in 0..2 {
            read(&owned, &digest).unwrap().unwrap();
        }
        assert_eq!(frame_calls_on_thread() - start, 2);
        assert_eq!(stats(&owned)["enabled"], false);
        assert_eq!(stats(&owned)["retained_entries"], 0);
        assert!(
            matches!(owned.enable_native_frame_reuse_for_testonly(), Err(e) if e.code() == code::SOURCE)
        );
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn native_frame_reuse_retention_full_keeps_strict_success_and_current_read() {
        for full_entries in [false, true] {
            let (mut capture, _, digest) = if full_entries {
                capture_node(tag::NULL, b"")
            } else {
                capture_node(tag::BYTES_CHUNK, b"material")
            };
            if full_entries {
                read(&capture, &digest).unwrap().unwrap();
            }
            let (other_tag, bytes): (u8, &[u8]) = if full_entries {
                (tag::BOOL_TRUE, b"")
            } else {
                (tag::BYTES_CHUNK, b"larger current material")
            };
            let other = frame(other_tag, bytes);
            // 仅测试私有原件变动，使超出初始捕获保留资格的新好帧可达。
            replace_original(&mut capture, other, other_tag, bytes.to_vec());
            let start = frame_calls_on_thread();
            for _ in 0..2 {
                assert_eq!(read(&capture, &other).unwrap().unwrap().body(), bytes);
            }
            assert_eq!(frame_calls_on_thread() - start, 2);
            assert_eq!(stats(&capture)["retention_skips"], 2);
            assert_eq!(
                stats(&capture)["retained_entries"],
                if full_entries { 1 } else { 0 }
            );
            assert_eq!(stats(&capture)["entry_limit"], 1);
            assert_eq!(
                stats(&capture)["body_capacity_limit_bytes"],
                if full_entries { 0 } else { 8 }
            );
        }
    }
    #[test]
    fn native_frame_reuse_actual_copy_capacity_cannot_exceed_original_body_len() {
        let (capture, _, digest) = capture_node(tag::BYTES_CHUNK, b"material");
        let frame =
            job::CheckedFrame::checked(&digest, tag::BYTES_CHUNK, b"material".to_vec()).unwrap();
        let mut copied = Vec::with_capacity(64);
        copied.extend_from_slice(frame.body());
        assert!(copied.capacity() > 8);
        let nodes = native(&capture);
        let node = nodes.nodes.get(&digest).unwrap();
        nodes
            .memo
            .borrow_mut()
            .retain_copy(&mut node.proof.borrow_mut(), &frame, copied);
        assert_eq!(stats(&capture)["retention_skips"], 1);
        assert_eq!(stats(&capture)["retained_entries"], 0);
        let start = frame_calls_on_thread();
        assert_eq!(
            read(&capture, &digest).unwrap().unwrap().body(),
            frame.body()
        );
        assert_eq!(frame_calls_on_thread() - start, 1);
        let receipt = stats(&capture);
        assert!(
            receipt["retained_body_vec_capacity_bytes"]
                .as_u64()
                .unwrap()
                <= receipt["body_capacity_limit_bytes"].as_u64().unwrap()
        );
    }
}

struct Verifier<'a, 'capture> {
    source: CodecContext<'a, 'capture>,
    #[cfg(feature = "s_capture_native_testonly")]
    owner_codec: Option<&'a NativeOwnerCodecMemo>,
    mode: Mode,
    work: Option<WorkBudget>,
    /// 已核过的 (摘要, 上下文)。
    seen: WorkSet<(NodeDigest, Flavor)>,
    /// 已入待核集合的 (摘要, 上下文) —— 重复子树不重复入队。
    pushed: WorkSet<(NodeDigest, Flavor)>,
    todo: WorkVec<(NodeDigest, Flavor)>,
    /// 实际读到的不同摘要（唯一节点计数）。
    read_digests: WorkSet<NodeDigest>,
    bytes_memo: WorkMap<(NodeDigest, Utf8State, NumState), ByteAgg>,
    elem_memo: WorkMap<(NodeDigest, Flavor), u64>,
    field_memo: WorkMap<(NodeDigest, Flavor), FieldAgg>,
    key_cmp: WorkMap<(NodeDigest, NodeDigest), std::cmp::Ordering>,
    native_seen_permit: Option<capture_allocation::scratch::NativeReservation>,
}

impl<'a, 'capture> Verifier<'a, 'capture> {
    // #1394 Root 内部实施裁定：具体capture与私有context闭合，不接受dyn成功token。
    #[cfg(test)]
    fn with_memo_capture(
        source: &'a mut CaptureReader<'capture>,
        mode: Mode,
        memo: AuditMemo,
    ) -> Self {
        Self::with_context(CodecContext::Capture(source.0, None), mode, memo)
    }
    #[cfg(test)]
    fn with_memo(source: &'a mut dyn NodeSource, mode: Mode, memo: AuditMemo) -> Self {
        Self::with_context(CodecContext::Source(source), mode, memo)
    }
    fn with_context(source: CodecContext<'a, 'capture>, mode: Mode, memo: AuditMemo) -> Self {
        let budget = source.work_budget();
        let memo = if budget.is_some() && memo.budget.is_none() {
            assert!(memo.seen.is_empty() && memo.pushed.is_empty() && memo.read_digests.is_empty());
            AuditMemo::new_work(budget.as_ref())
        } else {
            memo
        };
        Self {
            source,
            #[cfg(feature = "s_capture_native_testonly")]
            owner_codec: None,
            mode,
            work: memo.budget,
            seen: memo.seen,
            pushed: memo.pushed,
            todo: memo.todo,
            read_digests: memo.read_digests,
            bytes_memo: memo.bytes_memo,
            elem_memo: memo.elem_memo,
            field_memo: memo.field_memo,
            key_cmp: memo.key_cmp,
            native_seen_permit: memo.native_seen_permit,
        }
    }
    fn into_memo(self) -> AuditMemo {
        AuditMemo {
            budget: self.work.clone(),
            seen: self.seen,
            pushed: self.pushed,
            todo: self.todo,
            read_digests: self.read_digests,
            bytes_memo: self.bytes_memo,
            elem_memo: self.elem_memo,
            field_memo: self.field_memo,
            key_cmp: self.key_cmp,
            native_seen_permit: self.native_seen_permit,
        }
    }

    fn new(source: &'a mut dyn NodeSource, mode: Mode) -> Self {
        Self::with_context(CodecContext::Source(source), mode, AuditMemo::default())
    }
    /// 读节点 + 核摘要 + 记唯一节点。
    fn read_node(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<(u8, VerifierBody<'capture>), CodecError> {
        let got = self.source.read(digest)?;
        self.read_digests.insert(*digest)?;
        Ok(got)
    }

    fn push_work(&mut self, digest: NodeDigest, flavor: Flavor) -> Result<(), CodecError> {
        #[cfg(feature = "s_capture_native_testonly")]
        if self
            .owner_codec
            .is_some_and(|prior| prior.seen.contains_key(&(digest, flavor)))
        {
            return Ok(());
        }
        if self.pushed.insert((digest, flavor))? {
            self.todo.push((digest, flavor))?;
        }
        Ok(())
    }

    /// 字节序列 B 的折叠摘要。按 (摘要, UTF-8 进态, 数字进态) 记忆化：
    /// 规范 M 形状（含 k 规则）与哈希逐节点核，共享子树只走一次。
    fn byte_agg(&mut self, root: &NodeDigest) -> Result<ByteAgg, CodecError> {
        enum Step {
            Enter {
                digest: NodeDigest,
                utf8: Utf8State,
                num: NumState,
            },
            Right {
                digest: NodeDigest,
            },
            Combine {
                digest: NodeDigest,
                n: u64,
                utf8: Utf8State,
                num: NumState,
            },
        }
        let budget = self.work.clone();
        let mut steps = WorkVec::with_work_capacity(1, budget.as_ref())?;
        steps.push(Step::Enter {
            digest: *root,
            utf8: Utf8State::Empty,
            num: NumState::Start,
        })?;
        let mut aggs: WorkVec<ByteAgg> = WorkVec::new_work(budget.as_ref());
        while let Some(step) = steps.pop() {
            match step {
                Step::Enter { digest, utf8, num } => {
                    #[cfg(feature = "s_capture_native_testonly")]
                    if let Some(agg) = self
                        .owner_codec
                        .and_then(|p| p.bytes.get(&(digest, utf8, num)))
                    {
                        aggs.push(*agg)?;
                        continue;
                    }
                    if let Some(agg) = self.bytes_memo.get(&(digest, utf8, num)) {
                        aggs.push(*agg)?;
                        continue;
                    }
                    let (t, body) = self.read_node(&digest)?;
                    match t {
                        tag::SEQ_EMPTY => {
                            if !body.is_empty() {
                                return Err(body_shape_error(t, &body, "空序列 body 必须为空"));
                            }
                            let agg = ByteAgg {
                                leaves: 0,
                                total_bytes: 0,
                                last_len: 0,
                                all_but_last_ok: true,
                                utf8,
                                num,
                            };
                            self.bytes_memo.insert((digest, utf8, num), agg)?;
                            aggs.push(agg)?;
                        }
                        tag::SEQ_LEAF => {
                            if body.len() != 32 {
                                return Err(sequence_shape_error(
                                    "单叶序列 body 必须是 32 字节叶摘要",
                                ));
                            }
                            let leaf = digest_at(&body, 0);
                            let (lt, lbody) = self.read_node(&leaf)?;
                            if lt != tag::BYTES_CHUNK {
                                return Err(byte_chunk_shape_error(format!(
                                    "字节序列的叶必须是 0x10 块节点，读到 tag={lt:02x}"
                                )));
                            }
                            if lbody.is_empty() || lbody.len() > CHUNK_BYTES {
                                return Err(byte_chunk_shape_error(format!(
                                    "块长度 {} 必须落在 1..=4096",
                                    lbody.len()
                                )));
                            }
                            let utf8_out = utf8_feed(utf8, &lbody).ok_or_else(utf8_error)?;
                            let agg = ByteAgg {
                                leaves: 1,
                                total_bytes: lbody.len() as u64,
                                last_len: lbody.len(),
                                all_but_last_ok: true,
                                utf8: utf8_out,
                                num: num_feed(num, &lbody),
                            };
                            self.bytes_memo.insert((digest, utf8, num), agg)?;
                            aggs.push(agg)?;
                        }
                        tag::SEQ_BRANCH => {
                            if body.len() != 72 {
                                return Err(sequence_shape_error(
                                    "分支序列 body 必须是 U64(n) + 左右子摘要（72 字节）",
                                ));
                            }
                            let n = be_u64(&body);
                            if n > MAX_FRAME_LEN {
                                return Err(length_overflow_error(format!(
                                    "分支序列声明 n={n} 超过 2^63-1"
                                )));
                            }
                            if n < 2 {
                                return Err(sequence_shape_error(format!(
                                    "分支序列 n={n} 必须 ≥ 2"
                                )));
                            }
                            let left = digest_at(&body, 8);
                            let right = digest_at(&body, 40);
                            steps.push(Step::Combine {
                                digest,
                                n,
                                utf8,
                                num,
                            })?;
                            steps.push(Step::Right { digest: right })?;
                            steps.push(Step::Enter {
                                digest: left,
                                utf8,
                                num,
                            })?;
                        }
                        other => {
                            return Err(sequence_shape_error(format!(
                                "序列位置读到非序列节点 tag={other:02x}"
                            )))
                        }
                    }
                }
                Step::Right { digest } => {
                    let left = *aggs
                        .last()
                        .ok_or_else(|| sequence_shape_error("字节序列计数栈缺左子"))?;
                    steps.push(Step::Enter {
                        digest,
                        utf8: left.utf8,
                        num: left.num,
                    })?;
                }
                Step::Combine {
                    digest,
                    n,
                    utf8,
                    num,
                } => {
                    let right = aggs
                        .pop()
                        .ok_or_else(|| sequence_shape_error("字节序列计数栈缺右子"))?;
                    let left = aggs
                        .pop()
                        .ok_or_else(|| sequence_shape_error("字节序列计数栈缺左子"))?;
                    let k = split_index(n).expect("n ≥ 2");
                    if left.leaves != k || right.leaves != n - k {
                        return Err(sequence_shape_error(format!(
                            "序列分解形状不符：n={n} 要求左 k={k}、右 {}，实得左 {}、右 {}",
                            n - k,
                            left.leaves,
                            right.leaves
                        )));
                    }
                    let agg = ByteAgg {
                        leaves: left.leaves + right.leaves,
                        total_bytes: left.total_bytes + right.total_bytes,
                        last_len: right.last_len,
                        // 左子末叶不是整条序列的末叶 ⟹ 必须恰好 4096
                        all_but_last_ok: left.all_but_last_ok
                            && left.last_len == CHUNK_BYTES
                            && right.all_but_last_ok,
                        utf8: right.utf8,
                        num: right.num,
                    };
                    self.bytes_memo.insert((digest, utf8, num), agg)?;
                    aggs.push(agg)?;
                }
            }
        }
        aggs.pop()
            .ok_or_else(|| sequence_shape_error("字节序列计数栈为空"))
    }

    /// 数组元素序列：逻辑叶数折叠；叶必须是值节点，值根按 (摘要, 上下文) 入待核集合。
    fn elem_count(&mut self, root: &NodeDigest, flavor: Flavor) -> Result<u64, CodecError> {
        enum Step {
            Enter(NodeDigest),
            Combine { digest: NodeDigest, n: u64 },
        }
        let budget = self.work.clone();
        let mut steps = WorkVec::with_work_capacity(1, budget.as_ref())?;
        steps.push(Step::Enter(*root))?;
        let mut counts: WorkVec<u64> = WorkVec::new_work(budget.as_ref());
        while let Some(step) = steps.pop() {
            match step {
                Step::Enter(digest) => {
                    #[cfg(feature = "s_capture_native_testonly")]
                    if let Some(count) = self
                        .owner_codec
                        .and_then(|p| p.elems.get(&(digest, flavor)))
                    {
                        counts.push(*count)?;
                        continue;
                    }
                    if let Some(count) = self.elem_memo.get(&(digest, flavor)) {
                        counts.push(*count)?;
                        continue;
                    }
                    let (t, body) = self.read_node(&digest)?;
                    match t {
                        tag::SEQ_EMPTY => {
                            if !body.is_empty() {
                                return Err(body_shape_error(t, &body, "空序列 body 必须为空"));
                            }
                            self.elem_memo.insert((digest, flavor), 0)?;
                            counts.push(0)?;
                        }
                        tag::SEQ_LEAF => {
                            if body.len() != 32 {
                                return Err(sequence_shape_error(
                                    "单叶序列 body 必须是 32 字节叶摘要",
                                ));
                            }
                            let leaf = digest_at(&body, 0);
                            let (lt, _lbody) = self.read_node(&leaf)?;
                            if is_sequence_tag(lt) {
                                return Err(sequence_shape_error(format!(
                                    "单叶序列的叶不能又是序列节点（tag={lt:02x}）"
                                )));
                            }
                            if Kind::from_tag(lt).is_none() {
                                return Err(unsupported_value_tag_error(lt));
                            }
                            self.push_work(leaf, flavor)?;
                            self.elem_memo.insert((digest, flavor), 1)?;
                            counts.push(1)?;
                        }
                        tag::SEQ_BRANCH => {
                            if body.len() != 72 {
                                return Err(sequence_shape_error(
                                    "分支序列 body 必须是 U64(n) + 左右子摘要（72 字节）",
                                ));
                            }
                            let n = be_u64(&body);
                            if n > MAX_FRAME_LEN {
                                return Err(length_overflow_error(format!(
                                    "分支序列声明 n={n} 超过 2^63-1"
                                )));
                            }
                            if n < 2 {
                                return Err(sequence_shape_error(format!(
                                    "分支序列 n={n} 必须 ≥ 2"
                                )));
                            }
                            let left = digest_at(&body, 8);
                            let right = digest_at(&body, 40);
                            steps.push(Step::Combine { digest, n })?;
                            steps.push(Step::Enter(right))?;
                            steps.push(Step::Enter(left))?;
                        }
                        other => {
                            return Err(sequence_shape_error(format!(
                                "序列位置读到非序列节点 tag={other:02x}"
                            )))
                        }
                    }
                }
                Step::Combine { digest, n } => {
                    let right = counts
                        .pop()
                        .ok_or_else(|| sequence_shape_error("元素序列计数栈缺右子"))?;
                    let left = counts
                        .pop()
                        .ok_or_else(|| sequence_shape_error("元素序列计数栈缺左子"))?;
                    let k = split_index(n).expect("n ≥ 2");
                    if left != k || right != n - k {
                        return Err(sequence_shape_error(format!(
                            "序列分解形状不符：n={n} 要求左 k={k}、右 {}，实得左 {left}、右 {right}",
                            n - k
                        )));
                    }
                    self.elem_memo.insert((digest, flavor), n)?;
                    counts.push(n)?;
                }
            }
        }
        counts
            .pop()
            .ok_or_else(|| sequence_shape_error("元素序列计数栈为空"))
    }

    /// 对象字段序列：折叠 (计数, 首键, 末键)，组合处判「严格升序 + 唯一」；
    /// 键节点逐摘要核验一次，字段值按 (摘要, 上下文) 入待核集合。
    fn field_agg(&mut self, root: &NodeDigest, flavor: Flavor) -> Result<FieldAgg, CodecError> {
        enum Step {
            Enter(NodeDigest),
            Combine { digest: NodeDigest, n: u64 },
        }
        let budget = self.work.clone();
        let mut steps = WorkVec::with_work_capacity(1, budget.as_ref())?;
        steps.push(Step::Enter(*root))?;
        let mut aggs: WorkVec<FieldAgg> = WorkVec::new_work(budget.as_ref());
        while let Some(step) = steps.pop() {
            match step {
                Step::Enter(digest) => {
                    #[cfg(feature = "s_capture_native_testonly")]
                    if let Some(agg) = self
                        .owner_codec
                        .and_then(|p| p.fields.get(&(digest, flavor)))
                    {
                        aggs.push(*agg)?;
                        continue;
                    }
                    if let Some(agg) = self.field_memo.get(&(digest, flavor)) {
                        aggs.push(*agg)?;
                        continue;
                    }
                    let (t, body) = self.read_node(&digest)?;
                    match t {
                        tag::SEQ_EMPTY => {
                            if !body.is_empty() {
                                return Err(body_shape_error(t, &body, "空序列 body 必须为空"));
                            }
                            let agg = FieldAgg {
                                count: 0,
                                first: None,
                                last: None,
                            };
                            self.field_memo.insert((digest, flavor), agg)?;
                            aggs.push(agg)?;
                        }
                        tag::SEQ_LEAF => {
                            if body.len() != 32 {
                                return Err(sequence_shape_error(
                                    "单叶序列 body 必须是 32 字节叶摘要",
                                ));
                            }
                            let field = digest_at(&body, 0);
                            let (ft, fbody) = self.read_node(&field)?;
                            if ft != tag::OBJECT_FIELD {
                                return Err(sequence_shape_error(format!(
                                    "对象字段序列的叶必须是 0x14 字段节点，读到 tag={ft:02x}"
                                )));
                            }
                            if fbody.len() != 64 {
                                return Err(sequence_shape_error(
                                    "字段节点 body 必须是键摘要 + 值摘要（64 字节）",
                                ));
                            }
                            let key = digest_at(&fbody, 0);
                            let value = digest_at(&fbody, 32);
                            self.validate_string_node(&key)?;
                            self.seen.insert((key, Flavor::Value))?;
                            self.push_work(value, flavor)?;
                            let agg = FieldAgg {
                                count: 1,
                                first: Some(key),
                                last: Some(key),
                            };
                            self.field_memo.insert((digest, flavor), agg)?;
                            aggs.push(agg)?;
                        }
                        tag::SEQ_BRANCH => {
                            if body.len() != 72 {
                                return Err(sequence_shape_error(
                                    "分支序列 body 必须是 U64(n) + 左右子摘要（72 字节）",
                                ));
                            }
                            let n = be_u64(&body);
                            if n > MAX_FRAME_LEN {
                                return Err(length_overflow_error(format!(
                                    "分支序列声明 n={n} 超过 2^63-1"
                                )));
                            }
                            if n < 2 {
                                return Err(sequence_shape_error(format!(
                                    "分支序列 n={n} 必须 ≥ 2"
                                )));
                            }
                            let left = digest_at(&body, 8);
                            let right = digest_at(&body, 40);
                            steps.push(Step::Combine { digest, n })?;
                            steps.push(Step::Enter(right))?;
                            steps.push(Step::Enter(left))?;
                        }
                        other => {
                            return Err(sequence_shape_error(format!(
                                "序列位置读到非序列节点 tag={other:02x}"
                            )))
                        }
                    }
                }
                Step::Combine { digest, n } => {
                    let right = aggs
                        .pop()
                        .ok_or_else(|| sequence_shape_error("字段序列计数栈缺右子"))?;
                    let left = aggs
                        .pop()
                        .ok_or_else(|| sequence_shape_error("字段序列计数栈缺左子"))?;
                    let k = split_index(n).expect("n ≥ 2");
                    if left.count != k || right.count != n - k {
                        return Err(sequence_shape_error(format!(
                            "序列分解形状不符：n={n} 要求左 k={k}、右 {}，实得左 {}、右 {}",
                            n - k,
                            left.count,
                            right.count
                        )));
                    }
                    let (boundary_left, boundary_right) = match (left.last, right.first) {
                        (Some(a), Some(b)) => (a, b),
                        _ => {
                            return Err(sequence_shape_error(
                                "字段序列分支两侧都必须非空（n ≥ 2 且 k 规则保证）",
                            ))
                        }
                    };
                    match self.compare_keys(&boundary_left, &boundary_right)? {
                        std::cmp::Ordering::Less => {}
                        std::cmp::Ordering::Equal => {
                            return Err(CodecError::new(
                                code::DUPLICATE_KEY,
                                "对象出现重复键（同一键摘要跨分支重复）".to_string(),
                            ))
                        }
                        std::cmp::Ordering::Greater => {
                            return Err(CodecError::new(
                                code::OBJECT_KEY_ORDER,
                                "对象键未按 UTF-8 字节升序".to_string(),
                            ))
                        }
                    }
                    let agg = FieldAgg {
                        count: n,
                        first: left.first,
                        last: right.last,
                    };
                    self.field_memo.insert((digest, flavor), agg)?;
                    aggs.push(agg)?;
                }
            }
        }
        aggs.pop()
            .ok_or_else(|| sequence_shape_error("字段序列计数栈为空"))
    }

    /// 核字符串节点自身（不收集内容、不因声明长度预分配）。
    fn validate_string_node(&mut self, digest: &NodeDigest) -> Result<u64, CodecError> {
        let (t, body) = self.read_node(digest)?;
        if t != tag::STRING {
            return Err(CodecError::new(
                code::VALUE_KIND,
                format!("对象键必须是字符串节点（0x03），读到 tag={t:02x}"),
            ));
        }
        let declared = framed_len(&body, "字符串")?;
        let agg = self.byte_agg(&digest_at(&body, 8))?;
        if agg.utf8 != Utf8State::Empty {
            return Err(utf8_error());
        }
        if !agg.all_but_last_ok {
            return Err(byte_chunk_shape_error(
                "非末块长度必须恰好 4096（只有末块可以较短）",
            ));
        }
        if agg.total_bytes != declared {
            return Err(byte_extent_error(agg.total_bytes, declared));
        }
        Ok(declared)
    }

    /// 字符串节点的字节序列根（键比较用，只核该节点 framing）。
    fn string_bytes_root(&mut self, digest: &NodeDigest) -> Result<NodeDigest, CodecError> {
        let (t, body) = self.read_node(digest)?;
        if t != tag::STRING {
            return Err(CodecError::new(
                code::VALUE_KIND,
                format!("对象键必须是字符串节点（0x03），读到 tag={t:02x}"),
            ));
        }
        framed_len(&body, "字符串")?;
        Ok(digest_at(&body, 8))
    }

    /// 键字节流的字典序比较：同摘要即同串（O(1)），否则惰性逐块比到首个差异。
    fn compare_keys(
        &mut self,
        a: &NodeDigest,
        b: &NodeDigest,
    ) -> Result<std::cmp::Ordering, CodecError> {
        use std::cmp::Ordering;
        if a == b {
            return Ok(Ordering::Equal);
        }
        #[cfg(feature = "s_capture_native_testonly")]
        if let Some(ord) = self.owner_codec.and_then(|p| p.key_cmp.get(&(*a, *b))) {
            return Ok(*ord);
        }
        if let Some(ord) = self.key_cmp.get(&(*a, *b)) {
            return Ok(*ord);
        }
        let ord = self.compare_key_streams(a, b)?;
        self.key_cmp.insert((*a, *b), ord)?;
        self.key_cmp.insert((*b, *a), ord.reverse())?;
        Ok(ord)
    }

    fn compare_key_streams(
        &mut self,
        a: &NodeDigest,
        b: &NodeDigest,
    ) -> Result<std::cmp::Ordering, CodecError> {
        use std::cmp::Ordering;
        let root_a = self.string_bytes_root(a)?;
        let root_b = self.string_bytes_root(b)?;
        let mut iter_a = LeafIter::new(root_a, self.work.clone().as_ref())?;
        let mut iter_b = LeafIter::new(root_b, self.work.clone().as_ref())?;
        let mut buf_a = VerifierBody::empty();
        let mut buf_b = VerifierBody::empty();
        let mut pos_a = 0usize;
        let mut pos_b = 0usize;
        let mut dig_a: Option<NodeDigest> = None;
        let mut dig_b: Option<NodeDigest> = None;
        loop {
            if pos_a == buf_a.len() {
                match iter_a.next_context(&mut self.source)? {
                    Some((t, d, body)) => {
                        if t != tag::BYTES_CHUNK {
                            return Err(byte_chunk_shape_error(format!(
                                "字节序列的叶必须是 0x10 块节点，读到 tag={t:02x}"
                            )));
                        }
                        buf_a = body;
                        pos_a = 0;
                        dig_a = Some(d);
                    }
                    None => {
                        buf_a.clear();
                        dig_a = None;
                    }
                }
            }
            if pos_b == buf_b.len() {
                match iter_b.next_context(&mut self.source)? {
                    Some((t, d, body)) => {
                        if t != tag::BYTES_CHUNK {
                            return Err(byte_chunk_shape_error(format!(
                                "字节序列的叶必须是 0x10 块节点，读到 tag={t:02x}"
                            )));
                        }
                        buf_b = body;
                        pos_b = 0;
                        dig_b = Some(d);
                    }
                    None => {
                        buf_b.clear();
                        dig_b = None;
                    }
                }
            }
            let more_a = pos_a < buf_a.len();
            let more_b = pos_b < buf_b.len();
            match (more_a, more_b) {
                (false, false) => return Ok(Ordering::Equal),
                (false, true) => return Ok(Ordering::Less),
                (true, false) => return Ok(Ordering::Greater),
                (true, true) => {}
            }
            if pos_a == 0 && pos_b == 0 && dig_a.is_some() && dig_a == dig_b {
                // 整块同摘要：同字节，直接跳过，不比对内容。
                pos_a = buf_a.len();
                pos_b = buf_b.len();
                continue;
            }
            let n = (buf_a.len() - pos_a).min(buf_b.len() - pos_b);
            for i in 0..n {
                let x = buf_a[pos_a + i];
                let y = buf_b[pos_b + i];
                if x != y {
                    return Ok(if x < y {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    });
                }
            }
            pos_a += n;
            pos_b += n;
        }
    }

    /// 在同一个核验器中逐根检查 ContentRef。每次都重新读取并核对根的
    /// format/digest/kind/extent；`seen` 只负责闭包内部节点去重，不会跳过根声明。
    fn verify_reference(
        &mut self,
        reference: &ContentRef,
    ) -> Result<(NodeDigest, Kind, Option<u64>), CodecError> {
        self.verify_refview(reference.into())
    }
    fn verify_refview(
        &mut self,
        reference: RefView<'_>,
    ) -> Result<(NodeDigest, Kind, Option<u64>), CodecError> {
        let operation_work = self.work.clone();
        let operation_result = self.verify_refview_inner(reference);
        if let Some(work) = operation_work {
            work.observe(&operation_result);
        }
        operation_result
    }
    fn verify_refview_inner(
        &mut self,
        reference: RefView<'_>,
    ) -> Result<(NodeDigest, Kind, Option<u64>), CodecError> {
        let digest = reference.digest()?;
        let (t, body) = self.read_node(&digest)?;
        let kind = Kind::from_tag(t).ok_or_else(|| unsupported_value_tag_error(t))?;
        if kind != reference.kind() {
            return Err(CodecError::new(
                code::KIND,
                format!(
                    "ContentRef.kind={} 与真实节点 tag={t:02x}（{}）不符",
                    reference.kind().name(),
                    kind.name()
                ),
            ));
        }
        let extent = header_extent(t, &body)?;
        if extent != reference.extent() {
            return Err(CodecError::new(
                code::EXTENT,
                format!(
                    "ContentRef.extent={:?} 与真实节点 header {:?} 不符",
                    reference.extent(),
                    extent
                ),
            ));
        }
        self.run(&digest)?;
        Ok((digest, kind, extent))
    }

    /// 全闭包核验：值节点按 (摘要, 上下文) 去重，序列按摘要折叠。
    fn run(&mut self, root: &NodeDigest) -> Result<(), CodecError> {
        self.push_work(*root, Flavor::Value)?;
        while let Some((digest, flavor)) = self.todo.pop() {
            if !self.seen.insert((digest, flavor))? {
                continue;
            }
            let (t, body) = self.read_node(&digest)?;
            match t {
                tag::NULL | tag::BOOL_FALSE | tag::BOOL_TRUE => {
                    if !body.is_empty() {
                        return Err(body_shape_error(t, &body, "该标签 body 必须为空"));
                    }
                }
                tag::STRING => {
                    let declared = framed_len(&body, "字符串")?;
                    let agg = self.byte_agg(&digest_at(&body, 8))?;
                    if agg.utf8 != Utf8State::Empty {
                        return Err(utf8_error());
                    }
                    if !agg.all_but_last_ok {
                        return Err(byte_chunk_shape_error(
                            "非末块长度必须恰好 4096（只有末块可以较短）",
                        ));
                    }
                    if agg.total_bytes != declared {
                        return Err(byte_extent_error(agg.total_bytes, declared));
                    }
                }
                tag::NUMBER => {
                    let declared = framed_len(&body, "数字 token")?;
                    let agg = self.byte_agg(&digest_at(&body, 8))?;
                    if !num_accepts(agg.num) {
                        return Err(CodecError::new(
                            code::NUMBER_TOKEN,
                            "数字节点内容不是合法 JSON 数字 token".to_string(),
                        ));
                    }
                    if !agg.all_but_last_ok {
                        return Err(byte_chunk_shape_error(
                            "非末块长度必须恰好 4096（只有末块可以较短）",
                        ));
                    }
                    if agg.total_bytes != declared {
                        return Err(byte_extent_error(agg.total_bytes, declared));
                    }
                }
                tag::ARRAY => {
                    let declared = framed_len(&body, "数组")?;
                    let count = self.elem_count(&digest_at(&body, 8), flavor)?;
                    if count != declared {
                        return Err(sequence_shape_error(format!(
                            "数组声明 {declared} 个元素，元素序列实得 {count} 叶"
                        )));
                    }
                }
                tag::OBJECT => {
                    let declared = framed_len(&body, "对象")?;
                    let agg = self.field_agg(&digest_at(&body, 8), flavor)?;
                    if agg.count != declared {
                        return Err(sequence_shape_error(format!(
                            "对象声明 {declared} 个字段，字段序列实得 {} 叶",
                            agg.count
                        )));
                    }
                }
                tag::JSON_TEXT => {
                    if flavor == Flavor::JsonInner {
                        return Err(CodecError::new(
                            code::JSON_TEXT_INTERNAL_KIND,
                            "json_text 内部不允许再出现文本视图（0x07）".to_string(),
                        ));
                    }
                    if body.len() != 32 {
                        return Err(body_shape_error(
                            t,
                            &body,
                            "body 必须是内部值的 32 字节摘要",
                        ));
                    }
                    self.push_work(digest_at(&body, 0), Flavor::JsonInner)?;
                }
                tag::BATCH_SELF => {
                    if !body.is_empty() {
                        return Err(body_shape_error(t, &body, "占位 body 必须为空"));
                    }
                    if flavor == Flavor::JsonInner {
                        return Err(CodecError::new(
                            code::JSON_TEXT_INTERNAL_KIND,
                            "json_text 内部不允许批次占位（0x08）".to_string(),
                        ));
                    }
                    if self.mode != Mode::Template {
                        return Err(CodecError::new(
                            code::TEMPLATE_PLACEHOLDER,
                            "模板占位（0x08）只在显式模板上下文里允许".to_string(),
                        ));
                    }
                }
                other => return Err(unsupported_value_tag_error(other)),
            }
        }
        Ok(())
    }
}

/// 以节点来源的实际 tag/body 读根节点并产出其 ContentRef。
///
/// 只核该节点自身与摘要：**不是 publish-valid 证明**（闭包未核、也没有来源一致性证明）；
/// publish 前必须再过 `verify`。
pub fn content_ref_of(source: &mut dyn NodeSource, root: &str) -> Result<ContentRef, CodecError> {
    content_ref_ctx(&mut CodecContext::Source(source), root)
}
fn content_ref_ctx(
    source: &mut CodecContext<'_, '_>,
    root: &str,
) -> Result<ContentRef, CodecError> {
    if matches!(source.work_budget(), Some(WorkBudget::Scratch(_))) {
        return Err(CodecError::scratch(
            capture_allocation::scratch::ScratchFailure::Ledger,
        ));
    }
    let (reference, permit) = content_ref_reserved(source, root)?;
    debug_assert!(permit.is_none());
    Ok(reference)
}
// 同一根解析/真实读取/头核验核；Native adapter 直接接原准入 token。
fn content_ref_reserved(
    source: &mut CodecContext<'_, '_>,
    root: &str,
) -> Result<
    (
        ContentRef,
        Option<capture_allocation::scratch::NativeReservation>,
    ),
    CodecError,
> {
    let digest = parse_root(root)?;
    let (kind, extent) = path::reference_header(source, &digest)?;
    let permit = source.reserve_native(CONTENT_FORMAT.len() + root.len())?;
    let reference = ContentRef {
        format: CONTENT_FORMAT.to_string(),
        root: root.to_string(),
        kind,
        extent,
    };
    Ok((reference, permit))
}

/// 严格核验通用公开逻辑值：`ContentRef` 四字段逐项对真实节点，且全闭包通过。
pub fn verify(source: &mut dyn NodeSource, reference: &ContentRef) -> Result<Verified, CodecError> {
    verify_with_mode(source, reference, Mode::Public)
}

/// 严格核验显式模板上下文（允许 0x08 占位；json_text 内部仍不允许）。
pub fn verify_template(
    source: &mut dyn NodeSource,
    reference: &ContentRef,
) -> Result<Verified, CodecError> {
    verify_with_mode(source, reference, Mode::Template)
}

fn verify_with_mode(
    source: &mut dyn NodeSource,
    reference: &ContentRef,
    mode: Mode,
) -> Result<Verified, CodecError> {
    let mut verifier = Verifier::new(source, mode);
    let (digest, kind, extent) = verifier.verify_reference(reference)?;
    let nodes = verifier.read_digests.len();
    Ok(Verified {
        content_ref: reference.clone(),
        digest,
        kind,
        extent,
        nodes,
    })
}

/// 在一次公开模式核验中检查多个根，并返回本次闭包实际读取的唯一节点数。
///
/// 各根的 `ContentRef` 仍逐项核对四字段；只有同一调用内的真实共享节点会去重。
/// 与单根核验一样，来源在本次调用期间须保持一致；调用者不可在根迭代中改写节点。
/// 该入口不提供来源语义、发布有效性或跨调用缓存证明，调用结束后核验器即丢弃。
pub fn verify_many(
    source: &mut dyn NodeSource,
    references: impl IntoIterator<Item = ContentRef>,
) -> Result<usize, CodecError> {
    verify_many_with_mode(source, references, Mode::Public)
}

/// 同一次一致读集中全核多个模板根；各根声明逐项重核，json_text内部仍不允许占位。
/// 不带出核验记忆，也不授予公开发布资格。
pub fn verify_template_many(
    source: &mut dyn NodeSource,
    references: impl IntoIterator<Item = ContentRef>,
) -> Result<usize, CodecError> {
    verify_many_with_mode(source, references, Mode::Template)
}

fn verify_many_with_mode(
    source: &mut dyn NodeSource,
    references: impl IntoIterator<Item = ContentRef>,
    mode: Mode,
) -> Result<usize, CodecError> {
    let mut verifier = Verifier::new(source, mode);
    for reference in references {
        verifier.verify_reference(&reference)?;
    }
    Ok(verifier.read_digests.len())
}

/// 通用动态来源始终走普通完整 verifier；不接受捕获能力或跨调用成功。
pub fn audit_verify(
    source: &mut dyn NodeSource,
    reference: &ContentRef,
    mode: Mode,
) -> Result<(), CodecError> {
    verify_with_mode(source, reference, mode).map(|_| ())
}
pub fn audit_verify_many(
    source: &mut dyn NodeSource,
    references: impl IntoIterator<Item = ContentRef>,
    mode: Mode,
) -> Result<(), CodecError> {
    verify_many_with_mode(source, references, mode).map(|_| ())
}

pub fn audit_verify_public(s: &mut dyn NodeSource, r: &ContentRef) -> Result<(), CodecError> {
    audit_verify(s, r, Mode::Public)
}
pub fn audit_verify_public_many(
    s: &mut dyn NodeSource,
    r: impl IntoIterator<Item = ContentRef>,
) -> Result<(), CodecError> {
    audit_verify_many(s, r, Mode::Public)
}
pub fn audit_verify_template_many(
    s: &mut dyn NodeSource,
    r: impl IntoIterator<Item = ContentRef>,
) -> Result<(), CodecError> {
    audit_verify_many(s, r, Mode::Template)
}

// ── 渲染：惰性叶迭代 + 规范 JSON 转义 ────────────────────────────────────────

/// 逐字符写出规范 JSON 转义（斜线不转义；控制字符用小写四位十六进制）。
fn write_escaped_char(ch: char, out: &mut dyn Write) -> Result<(), CodecError> {
    escaped_char_with(ch, |bytes| out.write_all(bytes).map_err(io_error))
}
fn write_escaped_char_work(ch: char, out: &mut WorkVec<u8>) -> Result<(), CodecError> {
    // 私有fallible写口直接承接同allocator的Result，外部io kind不授予资源因果证明。
    escaped_char_with(ch, |bytes| out.extend_from_slice(bytes))
}
fn escaped_char_with(
    ch: char,
    mut emit: impl FnMut(&[u8]) -> Result<(), CodecError>,
) -> Result<(), CodecError> {
    match ch {
        '"' => emit(b"\\\""),
        '\\' => emit(b"\\\\"),
        '\u{0008}' => emit(b"\\b"),
        '\u{0009}' => emit(b"\\t"),
        '\u{000A}' => emit(b"\\n"),
        '\u{000C}' => emit(b"\\f"),
        '\u{000D}' => emit(b"\\r"),
        c if (c as u32) < 0x20 => {
            let n = c as u8;
            const HEX: &[u8; 16] = b"0123456789abcdef";
            let bytes = [
                b'\\',
                b'u',
                b'0',
                b'0',
                HEX[(n >> 4) as usize],
                HEX[(n & 15) as usize],
            ];
            emit(&bytes)
        }
        c => {
            let mut bytes = [0u8; 4];
            emit(c.encode_utf8(&mut bytes).as_bytes())
        }
    }
}

/// 增量 UTF-8 解码（渲染转义用）：UTF-8 可以在块间分开。
#[derive(Debug, Default, Clone)]
struct Utf8Decoder {
    buf: [u8; 4],
    len: usize,
}

impl Utf8Decoder {
    fn feed<F>(&mut self, bytes: &[u8], mut on_char: F) -> Result<(), CodecError>
    where
        F: FnMut(char) -> Result<(), CodecError>,
    {
        for &b in bytes {
            if self.len >= 4 {
                return Err(utf8_error());
            }
            self.buf[self.len] = b;
            self.len += 1;
            let need = match self.buf[0] {
                0x00..=0x7f => 1,
                0xc2..=0xdf => 2,
                0xe0..=0xef => 3,
                0xf0..=0xf4 => 4,
                _ => return Err(utf8_error()),
            };
            if self.len == need {
                let text = std::str::from_utf8(&self.buf[..need]).map_err(|_| utf8_error())?;
                let ch = text.chars().next().expect("恰一个字符");
                self.len = 0;
                on_char(ch)?;
            }
        }
        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.len == 0
    }
}

enum RTask {
    Value(NodeDigest, Flavor),
    Emit(&'static [u8]),
    StringBody(NodeDigest),
    TokenBody(NodeDigest),
    EnterEscape,
    ExitEscape,
    /// 数组元素：惰性迭代，按 sink 消费逐项推进。
    Elements {
        iter: LeafIter,
        index: u64,
        total: u64,
        flavor: Flavor,
    },
    /// 对象字段：同上。
    Fields {
        iter: LeafIter,
        index: u64,
        total: u64,
        flavor: Flavor,
    },
}

struct Renderer<'a, 'capture> {
    source: CodecContext<'a, 'capture>,
    out: &'a mut dyn Write,
    escaping: bool,
    esc_decoder: Utf8Decoder,
}

impl Renderer<'_, '_> {
    /// 写出字节：处于转义层时逐字符转义，否则原样写出。
    fn emit(&mut self, bytes: &[u8]) -> Result<(), CodecError> {
        let budget = self.source.work_budget();
        let Renderer {
            out,
            escaping,
            esc_decoder,
            ..
        } = self;
        if !*escaping {
            return out.write_all(bytes).map_err(io_error);
        }
        let mut buf = WorkVec::with_work_capacity(
            bytes
                .len()
                .checked_add(8)
                .ok_or_else(capture_allocation::work::allocation_error)?,
            budget.as_ref(),
        )?;
        esc_decoder.feed(bytes, &mut |ch| write_escaped_char_work(ch, &mut buf))?;
        out.write_all(&buf).map_err(io_error)
    }

    fn run(&mut self, root: NodeDigest) -> Result<(), CodecError> {
        let budget = self.source.work_budget();
        let mut tasks = WorkVec::with_work_capacity(1, budget.as_ref())?;
        tasks.push(RTask::Value(root, Flavor::Value))?;
        while let Some(task) = tasks.pop() {
            match task {
                RTask::Emit(bytes) => self.emit(bytes)?,
                RTask::EnterEscape => {
                    if self.escaping {
                        return Err(CodecError::new(
                            code::JSON_TEXT_INTERNAL_KIND,
                            "渲染遇到嵌套文本视图层".to_string(),
                        ));
                    }
                    self.escaping = true;
                }
                RTask::ExitEscape => {
                    if !self.esc_decoder.is_complete() {
                        return Err(utf8_error());
                    }
                    self.escaping = false;
                }
                RTask::StringBody(digest) => self.write_string_body(&digest)?,
                RTask::TokenBody(digest) => self.write_token_body(&digest)?,
                RTask::Elements {
                    mut iter,
                    index,
                    total,
                    flavor,
                } => {
                    if index == total {
                        tasks.push(RTask::Emit(b"]"))?;
                        continue;
                    }
                    let (lt, leaf, _body) = iter
                        .next_context(&mut self.source)?
                        .ok_or_else(|| sequence_shape_error("元素序列提前结束"))?;
                    if Kind::from_tag(lt).is_none() {
                        return Err(unsupported_value_tag_error(lt));
                    }
                    tasks.push(RTask::Elements {
                        iter,
                        index: index + 1,
                        total,
                        flavor,
                    })?;
                    tasks.push(RTask::Value(leaf, flavor))?;
                    if index > 0 {
                        tasks.push(RTask::Emit(b","))?;
                    }
                }
                RTask::Fields {
                    mut iter,
                    index,
                    total,
                    flavor,
                } => {
                    if index == total {
                        tasks.push(RTask::Emit(b"}"))?;
                        continue;
                    }
                    let (ft, _field, fbody) = iter
                        .next_context(&mut self.source)?
                        .ok_or_else(|| sequence_shape_error("字段序列提前结束"))?;
                    if ft != tag::OBJECT_FIELD || fbody.len() != 64 {
                        return Err(sequence_shape_error(format!(
                            "对象字段序列的叶必须是 0x14 字段节点（64 字节 body），读到 tag={ft:02x}"
                        )));
                    }
                    let key = digest_at(&fbody, 0);
                    let value = digest_at(&fbody, 32);
                    tasks.push(RTask::Fields {
                        iter,
                        index: index + 1,
                        total,
                        flavor,
                    })?;
                    tasks.push(RTask::Value(value, flavor))?;
                    tasks.push(RTask::Emit(b":"))?;
                    tasks.push(RTask::Value(key, Flavor::Value))?;
                    if index > 0 {
                        tasks.push(RTask::Emit(b","))?;
                    }
                }
                RTask::Value(digest, flavor) => {
                    let (t, body) = self.source.read(&digest)?;
                    match t {
                        tag::NULL => tasks.push(RTask::Emit(b"null"))?,
                        tag::BOOL_FALSE => tasks.push(RTask::Emit(b"false"))?,
                        tag::BOOL_TRUE => tasks.push(RTask::Emit(b"true"))?,
                        tag::STRING => {
                            framed_len(&body, "字符串")?;
                            tasks.push(RTask::Emit(b"\""))?;
                            tasks.push(RTask::StringBody(digest))?;
                            tasks.push(RTask::Emit(b"\""))?;
                        }
                        tag::NUMBER => {
                            framed_len(&body, "数字 token")?;
                            tasks.push(RTask::TokenBody(digest))?;
                        }
                        tag::ARRAY => {
                            let declared = framed_len(&body, "数组")?;
                            tasks.push(RTask::Elements {
                                iter: LeafIter::new(digest_at(&body, 8), budget.as_ref())?,
                                index: 0,
                                total: declared,
                                flavor,
                            })?;
                            tasks.push(RTask::Emit(b"["))?;
                        }
                        tag::OBJECT => {
                            let declared = framed_len(&body, "对象")?;
                            tasks.push(RTask::Fields {
                                iter: LeafIter::new(digest_at(&body, 8), budget.as_ref())?,
                                index: 0,
                                total: declared,
                                flavor,
                            })?;
                            tasks.push(RTask::Emit(b"{"))?;
                        }
                        tag::JSON_TEXT => {
                            if flavor == Flavor::JsonInner {
                                return Err(CodecError::new(
                                    code::JSON_TEXT_INTERNAL_KIND,
                                    "json_text 内部不允许再出现文本视图（0x07）".to_string(),
                                ));
                            }
                            if body.len() != 32 {
                                return Err(body_shape_error(
                                    t,
                                    &body,
                                    "body 必须是内部值的 32 字节摘要",
                                ));
                            }
                            tasks.push(RTask::Emit(b"\""))?;
                            tasks.push(RTask::ExitEscape)?;
                            tasks.push(RTask::Value(digest_at(&body, 0), Flavor::JsonInner))?;
                            tasks.push(RTask::EnterEscape)?;
                            tasks.push(RTask::Emit(b"\""))?;
                        }
                        tag::BATCH_SELF => {
                            return Err(CodecError::new(
                                code::TEMPLATE_PLACEHOLDER,
                                "未解析的批次占位（0x08）不渲染".to_string(),
                            ))
                        }
                        other => return Err(unsupported_value_tag_error(other)),
                    }
                }
            }
        }
        Ok(())
    }

    /// 字符串内容：按块惰性取出、跨块增量解码、本层转义后经当前转义层写出。
    fn write_string_body(&mut self, digest: &NodeDigest) -> Result<(), CodecError> {
        let (t, body) = self.source.read(digest)?;
        if t != tag::STRING {
            return Err(CodecError::new(
                code::VALUE_KIND,
                format!("字符串节点必须是 0x03，读到 tag={t:02x}"),
            ));
        }
        let declared = framed_len(&body, "字符串")?;
        let budget = self.source.work_budget();
        let mut iter = LeafIter::new(digest_at(&body, 8), budget.as_ref())?;
        let mut decoder = Utf8Decoder::default();
        let mut escaped = WorkVec::with_work_capacity(8, budget.as_ref())?;
        let mut total: u64 = 0;
        loop {
            let next = {
                let Renderer { source, .. } = self;
                iter.next_context(source)?
            };
            let Some((lt, _leaf, lbody)) = next else {
                break;
            };
            if lt != tag::BYTES_CHUNK {
                return Err(byte_chunk_shape_error(format!(
                    "字节序列的叶必须是 0x10 块节点，读到 tag={lt:02x}"
                )));
            }
            if lbody.is_empty() || lbody.len() > CHUNK_BYTES {
                return Err(byte_chunk_shape_error(format!(
                    "块长度 {} 必须落在 1..=4096",
                    lbody.len()
                )));
            }
            total = total
                .checked_add(lbody.len() as u64)
                .ok_or_else(|| length_overflow_error("字节总数溢出"))?;
            let Renderer {
                out,
                escaping,
                esc_decoder,
                ..
            } = self;
            decoder.feed(&lbody, &mut |ch| {
                escaped.clear();
                write_escaped_char_work(ch, &mut escaped)?;
                if *escaping {
                    esc_decoder.feed(&escaped, &mut |inner| write_escaped_char(inner, &mut **out))
                } else {
                    out.write_all(&escaped).map_err(io_error)
                }
            })?;
        }
        if !decoder.is_complete() {
            return Err(utf8_error());
        }
        if total != declared {
            return Err(byte_extent_error(total, declared));
        }
        Ok(())
    }

    /// 数字 token：流式推进文法状态并原样写出，不收集整条 token。
    fn write_token_body(&mut self, digest: &NodeDigest) -> Result<(), CodecError> {
        let (t, body) = self.source.read(digest)?;
        if t != tag::NUMBER {
            return Err(CodecError::new(
                code::VALUE_KIND,
                format!("数字 token 必须是 0x04 节点，读到 tag={t:02x}"),
            ));
        }
        let declared = framed_len(&body, "数字 token")?;
        let budget = self.source.work_budget();
        let mut iter = LeafIter::new(digest_at(&body, 8), budget.as_ref())?;
        let mut state = NumState::Start;
        let mut total: u64 = 0;
        loop {
            let next = {
                let Renderer { source, .. } = self;
                iter.next_context(source)?
            };
            let Some((lt, _leaf, lbody)) = next else {
                break;
            };
            if lt != tag::BYTES_CHUNK {
                return Err(byte_chunk_shape_error(format!(
                    "字节序列的叶必须是 0x10 块节点，读到 tag={lt:02x}"
                )));
            }
            if lbody.is_empty() || lbody.len() > CHUNK_BYTES {
                return Err(byte_chunk_shape_error(format!(
                    "块长度 {} 必须落在 1..=4096",
                    lbody.len()
                )));
            }
            total = total
                .checked_add(lbody.len() as u64)
                .ok_or_else(|| length_overflow_error("字节总数溢出"))?;
            state = num_feed(state, &lbody);
            if state == NumState::Dead {
                return Err(CodecError::new(
                    code::NUMBER_TOKEN,
                    "数字节点内容不是合法 JSON 数字 token".to_string(),
                ));
            }
            self.emit(&lbody)?;
        }
        if !num_accepts(state) {
            return Err(CodecError::new(
                code::NUMBER_TOKEN,
                "数字节点内容不是合法 JSON 数字 token".to_string(),
            ));
        }
        if total != declared {
            return Err(byte_extent_error(total, declared));
        }
        Ok(())
    }
}

/// 规范 JSON 文本渲染（流式 Write sink）。
///
/// 先全闭包核验（`verify`），再逐节点复算摘要、按 sink 实际消费惰性写出：
/// 不要求调用方先备好整份渲染字符串，也不预收集元素/字段清单。
///
/// **sink 不是事务**：本函数返回 `Err` 时 sink 上可能已有部分字节（含 sink 自身的写错误）。
/// 只在成功后发布 staged 内容，由调用方负责。
pub fn render_canonical(
    source: &mut dyn NodeSource,
    reference: &ContentRef,
    out: &mut dyn Write,
) -> Result<(), CodecError> {
    render_ctx(CodecContext::Source(source), reference, out)
}
fn render_ctx(
    source: CodecContext<'_, '_>,
    reference: &ContentRef,
    out: &mut dyn Write,
) -> Result<(), CodecError> {
    render_refview_ctx(source, reference.into(), out)
}
fn render_refview_ctx(
    source: CodecContext<'_, '_>,
    reference: RefView<'_>,
    out: &mut dyn Write,
) -> Result<(), CodecError> {
    let operation_work = source.work_budget();
    let operation_result = render_refview_ctx_inner(source, reference, out);
    if let Some(work) = operation_work {
        work.observe(&operation_result);
    }
    operation_result
}
fn render_refview_ctx_inner(
    mut source: CodecContext<'_, '_>,
    reference: RefView<'_>,
    out: &mut dyn Write,
) -> Result<(), CodecError> {
    struct Bounded<'a> {
        inner: &'a mut dyn Write,
        limit: Option<u64>,
        written: u64,
    }
    impl Write for Bounded<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self
                .limit
                .is_some_and(|limit| (self.written as u128) + (bytes.len() as u128) > limit as u128)
            {
                return Err(std::io::Error::other(
                    "Incomplete/CaptureBytes：内容渲染超过显式物化上限",
                ));
            }
            let n = self.inner.write(bytes)?;
            self.written = self.written.saturating_add(n as u64);
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.flush()
        }
    }
    let mut bounded = Bounded {
        inner: out,
        limit: source.max_render_bytes(),
        written: 0,
    };
    let digest = {
        let mut verifier =
            Verifier::with_context(source.reborrow(), Mode::Public, AuditMemo::default());
        verifier.verify_refview(reference)?.0
    };
    let mut renderer = Renderer {
        source: source.reborrow(),
        out: &mut bounded,
        escaping: false,
        esc_decoder: Utf8Decoder::default(),
    };
    renderer.run(digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 独立向量 content-format-vectors.json 的 null 正例根。
    const VECTOR_NULL_ROOT: &str =
        "sc1-8373191693a3a2ee636963cdfcae8775f1ca22610c77f1c11804dd3406bc5806";

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    struct CountingSource<'a> {
        inner: &'a mut MemoryNodes,
        reads: usize,
        unique: std::collections::HashSet<NodeDigest>,
    }

    impl NodeSource for CountingSource<'_> {
        fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
            self.reads += 1;
            self.unique.insert(*digest);
            self.inner.read(digest)
        }
    }

    #[test]
    fn null_root_matches_independent_vector() {
        let mut store = MemoryNodes::new();
        let built = write_value(&mut store, &Value::Null).expect("写入 null");
        assert_eq!(built.root(), VECTOR_NULL_ROOT);
        assert_eq!(built.kind, Kind::Null);
        assert_eq!(built.extent, None);
    }

    #[test]
    fn split_index_is_largest_power_of_two_below_n() {
        assert_eq!(split_index(0), None);
        assert_eq!(split_index(1), None);
        assert_eq!(split_index(2), Some(1));
        assert_eq!(split_index(3), Some(2));
        assert_eq!(split_index(4), Some(2));
        assert_eq!(split_index(5), Some(4));
        assert_eq!(split_index(8), Some(4));
        assert_eq!(split_index(9), Some(8));
        assert_eq!(split_index(1 << 63), Some(1 << 62));
    }

    #[test]
    fn number_token_grammar() {
        for ok in [
            "0", "-0", "1", "1.0", "1e0", "1E+5", "-12.5e-3", "10", "1e-0",
        ] {
            assert!(NumberToken::parse(ok).is_ok(), "应接受 `{ok}`");
        }
        for bad in [
            "", "-", "+1", "01", "1.", ".5", "1e", "1e+", "0x1", "1_0", " 1", "1 ", "NaN",
            "Infinity", "1,0",
        ] {
            let err = NumberToken::parse(bad).expect_err("应拒绝");
            assert_eq!(err.code(), code::NUMBER_TOKEN, "`{bad}`");
        }
    }

    /// 构造带引用的图 ≠ 闭包已核：边指向缺失节点时写入成功、严格核验拒绝。
    #[test]
    fn construction_with_edge_is_not_closure_verified() {
        let mut store = MemoryNodes::new();
        let mut child_store = MemoryNodes::new();
        let child = write_value(&mut child_store, &Value::string("x")).expect("写入子值");
        let graph = Value::Array(vec![Value::Edge(child.digest)]);
        let built = write_value(&mut store, &graph).expect("构造带边的图");
        let reference = built.content_ref();
        assert_eq!(reference.kind, Kind::Array);
        assert_eq!(reference.extent, Some(1));

        let err = verify(&mut store, &reference).expect_err("闭包缺子节点必须拒绝");
        assert_eq!(err.code(), code::NODE_MISSING);

        for (digest, t, body) in child_store.iter() {
            store.insert_unchecked(*digest, t, body.to_vec());
        }
        let verified = verify(&mut store, &reference).expect("补上子节点后闭包可核");
        // 读取核过：数组、元素序列 0x12、子字符串、子字符串的字节序列 0x12、字节块，
        // 其中子字符串在序列叶与工作项各读一次 —— 计数是「核过多少次」而非「多少不同节点」。
        // 计数是**不同摘要数**：数组、元素序列 0x12、子字符串、其字节序列 0x12、字节块。
        assert_eq!(verified.nodes, 5);
    }

    /// 严格核验不凭调用者声明的 kind/extent 放行：逐项对真实节点 header。
    #[test]
    fn claimed_header_is_checked_against_real_node() {
        let mut store = MemoryNodes::new();
        let built = write_value(&mut store, &Value::string("1")).expect("写入字符串");

        let mut wrong_kind = built.content_ref();
        wrong_kind.kind = Kind::Number;
        assert_eq!(
            verify(&mut store, &wrong_kind)
                .expect_err("kind 不符")
                .code(),
            code::KIND
        );

        let mut wrong_extent = built.content_ref();
        wrong_extent.extent = Some(999);
        assert_eq!(
            verify(&mut store, &wrong_extent)
                .expect_err("extent 不符")
                .code(),
            code::EXTENT
        );

        let mut wrong_format = built.content_ref();
        wrong_format.format = "s-content/2".to_string();
        assert_eq!(
            verify(&mut store, &wrong_format)
                .expect_err("format 不符")
                .code(),
            code::FORMAT
        );

        let mut wrong_root = built.content_ref();
        wrong_root.root = wrong_root.root.to_uppercase();
        assert_eq!(
            verify(&mut store, &wrong_root)
                .expect_err("根文本大小写不符")
                .code(),
            code::ROOT
        );
    }

    #[test]
    fn verify_many_reuses_shared_nodes_but_rechecks_each_root_header() {
        let mut store = MemoryNodes::new();
        let shared = write_value(&mut store, &Value::string("shared")).unwrap();
        let first = write_value(
            &mut store,
            &Value::Array(vec![Value::Edge(shared.digest), Value::Edge(shared.digest)]),
        )
        .unwrap();
        let second =
            write_value(&mut store, &Value::Array(vec![Value::Edge(shared.digest)])).unwrap();

        let mut separate_store = store.clone();
        let mut separate = CountingSource {
            inner: &mut separate_store,
            reads: 0,
            unique: std::collections::HashSet::new(),
        };
        verify(&mut separate, &first.content_ref()).unwrap();
        verify(&mut separate, &second.content_ref()).unwrap();
        let separate_reads = separate.reads;
        drop(separate);

        let mut many_store = store.clone();
        let mut many = CountingSource {
            inner: &mut many_store,
            reads: 0,
            unique: std::collections::HashSet::new(),
        };
        let unique =
            verify_many(&mut many, vec![first.content_ref(), second.content_ref()]).unwrap();
        assert_eq!(unique, many.unique.len());
        assert!(
            many.reads < separate_reads,
            "共享根应减少读取：{} >= {}",
            many.reads,
            separate_reads
        );
    }

    #[test]
    fn verify_many_rejects_second_root_header_and_bad_closure() {
        let mut store = MemoryNodes::new();
        let value = write_value(&mut store, &Value::string("x")).unwrap();
        let mut wrong_extent = value.content_ref();
        wrong_extent.extent = Some(999);
        let err = verify_many(&mut store, vec![value.content_ref(), wrong_extent])
            .expect_err("已见根也必须重新核对 extent");
        assert_eq!(err.code(), code::EXTENT);

        let mut wrong_kind = value.content_ref();
        wrong_kind.kind = Kind::Number;
        let err = verify_many(&mut store, vec![value.content_ref(), wrong_kind])
            .expect_err("已见根也必须重新核对 kind");
        assert_eq!(err.code(), code::KIND);

        let missing = write_value(
            &mut store,
            &Value::Array(vec![Value::Edge(frame(tag::NULL, &[]))]),
        )
        .unwrap();
        let err = verify_many(&mut store, vec![value.content_ref(), missing.content_ref()])
            .expect_err("后续根缺子节点必须拒绝");
        assert_eq!(err.code(), code::NODE_MISSING);

        let bad = write_value(&mut store, &Value::string("bad")).unwrap();
        store.insert_unchecked(bad.digest, tag::STRING, vec![0; 40]);
        let err = verify_many(&mut store, vec![value.content_ref(), bad.content_ref()])
            .expect_err("后续根摘要损坏必须拒绝");
        assert_eq!(err.code(), code::NODE_DIGEST);
    }

    #[test]
    fn verify_many_does_not_inherit_success_or_open_json_text_context() {
        let mut full = MemoryNodes::new();
        let child = write_value(&mut full, &Value::string("child")).unwrap();
        let graph = write_value(&mut full, &Value::Array(vec![Value::Edge(child.digest)])).unwrap();
        assert_eq!(
            verify_many(&mut full, vec![graph.content_ref()]).unwrap(),
            5
        );

        let (root_tag, root_body) = full
            .get(&graph.digest)
            .map(|(tag, body)| (tag, body.to_vec()))
            .unwrap();
        let mut partial = MemoryNodes::new();
        partial.insert_unchecked(graph.digest, root_tag, root_body);
        let err =
            verify_many(&mut partial, vec![graph.content_ref()]).expect_err("调用间不得继承旧闭包");
        assert_eq!(err.code(), code::NODE_MISSING);

        let placeholder = frame(tag::BATCH_SELF, &[]);
        full.insert_unchecked(placeholder, tag::BATCH_SELF, Vec::new());
        let leaf = frame(tag::SEQ_LEAF, &placeholder);
        full.insert_unchecked(leaf, tag::SEQ_LEAF, placeholder.to_vec());
        let mut array_body = Vec::new();
        array_body.extend_from_slice(&checked_frame_len(1).unwrap());
        array_body.extend_from_slice(&leaf);
        let array = frame(tag::ARRAY, &array_body);
        full.insert_unchecked(array, tag::ARRAY, array_body);
        let json_text = frame(tag::JSON_TEXT, &array);
        full.insert_unchecked(json_text, tag::JSON_TEXT, array.to_vec());
        let json_ref = ContentRef::new(Kind::JsonText, &json_text, None);
        let err = verify_many(&mut full, vec![child.content_ref(), json_ref])
            .expect_err("json_text内部占位不得被共享上下文放行");
        assert_eq!(err.code(), code::JSON_TEXT_INTERNAL_KIND);

        // 同一 JSON_TEXT 根先作为 Value 核过，再作为另一个 JSON_TEXT 的
        // JsonInner 出现；Flavor 不能因摘要已见而跳过内部类型判据。
        let scalar = write_value(&mut full, &Value::string("inner")).unwrap();
        let inner_json = frame(tag::JSON_TEXT, &scalar.digest);
        full.insert_unchecked(inner_json, tag::JSON_TEXT, scalar.digest.to_vec());
        let outer_json = frame(tag::JSON_TEXT, &inner_json);
        full.insert_unchecked(outer_json, tag::JSON_TEXT, inner_json.to_vec());
        let err = verify_many(
            &mut full,
            vec![
                ContentRef::new(Kind::JsonText, &inner_json, None),
                ContentRef::new(Kind::JsonText, &outer_json, None),
            ],
        )
        .expect_err("同摘要切换到JsonInner后仍必须拒嵌套json_text");
        assert_eq!(err.code(), code::JSON_TEXT_INTERNAL_KIND);
    }

    #[test]
    fn placeholder_requires_explicit_template_context() {
        let mut store = MemoryNodes::new();
        let err = write_value(&mut store, &Value::BatchSelf).expect_err("公开逻辑值不得写占位");
        assert_eq!(err.code(), code::TEMPLATE_PLACEHOLDER);

        let built = write_template(&mut store, &Value::BatchSelf).expect("模板上下文可写占位");
        assert_eq!(built.kind, Kind::TemplateBatch);
        let reference = built.content_ref();

        assert_eq!(
            verify(&mut store, &reference)
                .expect_err("公开逻辑值核验拒绝占位")
                .code(),
            code::TEMPLATE_PLACEHOLDER
        );
        let verified = verify_template(&mut store, &reference).expect("模板上下文核验通过");
        assert_eq!(verified.nodes, 1);

        let mut rendered = Vec::new();
        let err = render_canonical(&mut store, &reference, &mut rendered)
            .expect_err("未解析的批次占位不渲染");
        assert_eq!(err.code(), code::TEMPLATE_PLACEHOLDER);
        assert!(rendered.is_empty(), "拒绝时不得写出半截渲染");
    }

    /// 模板权限不得穿透 json_text 内部：内部出现 0x08 一律拒绝。
    #[test]
    fn template_context_does_not_open_json_text_internals() {
        // 手写敌意图：0x07 → 0x05（1 元素）→ 0x12 → 0x08 占位。
        let mut store = MemoryNodes::new();
        let placeholder = frame(tag::BATCH_SELF, &[]);
        store.insert_unchecked(placeholder, tag::BATCH_SELF, Vec::new());
        let leaf = frame(tag::SEQ_LEAF, &placeholder);
        store.insert_unchecked(leaf, tag::SEQ_LEAF, placeholder.to_vec());
        let mut array_body = Vec::new();
        array_body.extend_from_slice(&checked_frame_len(1).expect("长度"));
        array_body.extend_from_slice(&leaf);
        let array = frame(tag::ARRAY, &array_body);
        store.insert_unchecked(array, tag::ARRAY, array_body);
        let json_text = frame(tag::JSON_TEXT, &array);
        store.insert_unchecked(json_text, tag::JSON_TEXT, array.to_vec());

        let reference = ContentRef::new(Kind::JsonText, &json_text, None);
        assert_eq!(
            verify(&mut store, &reference)
                .expect_err("公开核验拒绝")
                .code(),
            code::JSON_TEXT_INTERNAL_KIND
        );
        assert_eq!(
            verify_template(&mut store, &reference)
                .expect_err("模板核验也拒绝（权限不得穿透 json_text 内部）")
                .code(),
            code::JSON_TEXT_INTERNAL_KIND
        );
    }

    #[test]
    fn canonical_render_matches_vector_bytes() {
        let mut store = MemoryNodes::new();
        let text = Value::string("汉\né\\\"\u{1}");
        let built = write_value(&mut store, &text).expect("写入字符串");
        let mut rendered = Vec::new();
        render_canonical(&mut store, &built.content_ref(), &mut rendered).expect("渲染");
        assert_eq!(
            hex(&rendered),
            "22e6b1895c6ec3a95c5c5c225c753030303122", // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
            "unicode_escape 向量渲染逐字节一致"
        );

        let inner = JsonValue::Array(vec![
            JsonValue::Number(NumberToken::parse("1.0").unwrap()),
            JsonValue::String("汉\n".to_string()),
        ]);
        let built =
            write_value(&mut store, &Value::JsonText(Box::new(inner))).expect("写入文本视图");
        let mut rendered = Vec::new();
        render_canonical(&mut store, &built.content_ref(), &mut rendered).expect("渲染");
        assert_eq!(
            hex(&rendered),
            "225b312e302c5c22e6b1895c5c6e5c225d22", // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
            "json_text 向量渲染逐字节一致"
        );
    }

    /// 普通长得像 ContentRef 的 JSON 对象仍是普通对象；边直接接值摘要。
    #[test]
    fn ref_like_object_stays_plain_object() {
        let mut store = MemoryNodes::new();
        let target = write_value(&mut store, &Value::string("x")).expect("写入目标");
        let ref_like = Value::Object(vec![
            ("format".to_string(), Value::string("s-content/1")),
            ("root".to_string(), Value::string(target.root())),
            ("kind".to_string(), Value::string("string")),
            (
                "extent".to_string(),
                Value::Number(NumberToken::parse("1").unwrap()),
            ),
        ]);
        let built = write_value(&mut store, &ref_like).expect("写入四字段对象");
        assert_eq!(built.kind, Kind::Object);
        assert_ne!(
            built.digest, target.digest,
            "四字段对象不得被当成引用再求值摘要"
        );
        assert!(verify(&mut store, &built.content_ref()).is_ok());

        let edge = Value::edge_of_ref(&target.content_ref()).expect("边取根摘要");
        assert_eq!(edge, Value::Edge(target.digest));
        let array = write_value(&mut store, &Value::Array(vec![edge])).expect("数组带边");
        assert!(verify(&mut store, &array.content_ref()).is_ok());
    }

    #[test]
    fn object_keys_are_sorted_and_duplicates_rejected() {
        let mut store = MemoryNodes::new();
        let fields = vec![
            ("汉".to_string(), Value::string("c")),
            ("é".to_string(), Value::string("b")),
            ("z".to_string(), Value::string("a")),
        ];
        let mut reversed = fields.clone();
        reversed.reverse();
        let a = write_value(&mut store, &Value::Object(fields)).expect("写入对象");
        let b = write_value(&mut store, &Value::Object(reversed)).expect("写入逆序对象");
        assert_eq!(
            a.digest, b.digest,
            "对象字段按键的 UTF-8 字节升序，与输入顺序无关"
        );

        let dup = Value::Object(vec![
            ("k".to_string(), Value::string("a")),
            ("k".to_string(), Value::string("b")),
        ]);
        let err = write_value(&mut store, &dup).expect_err("重复键必须拒绝");
        assert_eq!(err.code(), code::DUPLICATE_KEY);
    }

    #[test]
    fn content_ref_transport_shape_is_exact() {
        let mut store = MemoryNodes::new();
        let built = write_value(&mut store, &Value::string("1")).expect("写入字符串");
        let reference = built.content_ref();
        let json = reference.to_transport_json();
        assert_eq!(
            json["extent"].as_str(),
            Some("1"),
            "extent 是 U64 长度，按 §5.2 用规范十进制字符串传输"
        );
        assert_eq!(
            ContentRef::from_transport_json(&json).expect("回读"),
            reference
        );

        let mut extra = json.clone();
        extra
            .as_object_mut()
            .unwrap()
            .insert("session".into(), JsonNode::String("s".into()));
        assert_eq!(
            ContentRef::from_transport_json(&extra)
                .expect_err("多余字段")
                .code(),
            code::REF_FIELD
        );

        let mut missing = json.clone();
        missing.as_object_mut().unwrap().remove("extent");
        assert_eq!(
            ContentRef::from_transport_json(&missing)
                .expect_err("缺字段")
                .code(),
            code::REF_FIELD
        );

        for (bad, want) in [
            (JsonNode::Number(1.into()), code::EXTENT),
            (
                JsonNode::Number(serde_json::Number::from_f64(1.0).unwrap()),
                code::EXTENT,
            ),
            (JsonNode::String("+1".into()), code::EXTENT),
            (JsonNode::String("01".into()), code::EXTENT),
            (JsonNode::String("".into()), code::EXTENT),
            (JsonNode::String(" 1".into()), code::EXTENT),
            (JsonNode::String("1.0".into()), code::EXTENT),
            (
                JsonNode::String("9223372036854775808".into()),
                code::LENGTH_OVERFLOW,
            ),
            (
                JsonNode::String("18446744073709551616".into()),
                code::LENGTH_OVERFLOW,
            ),
        ] {
            let mut wrong = json.clone();
            wrong
                .as_object_mut()
                .unwrap()
                .insert("extent".into(), bad.clone());
            assert_eq!(
                ContentRef::from_transport_json(&wrong)
                    .expect_err(&format!("extent={bad} 必须拒绝"))
                    .code(),
                want,
                "extent={bad}"
            );
        }
        // 边界：2^63-1 是允许的最大声明长度。
        let mut max_extent = json.clone();
        max_extent.as_object_mut().unwrap().insert(
            "extent".into(),
            JsonNode::String("9223372036854775807".into()),
        );
        assert_eq!(
            ContentRef::from_transport_json(&max_extent)
                .expect("2^63-1 合法")
                .extent,
            Some(MAX_FRAME_LEN)
        );

        // extent 形状必须与 kind 对应：长度型必须给字符串，非长度型必须 null。
        for (kind_name, extent) in [
            ("string", JsonNode::Null),
            ("json_text", JsonNode::String("1".into())),
            ("null", JsonNode::String("0".into())),
            ("template_batch", JsonNode::String("3".into())),
        ] {
            let mut wrong = json.clone();
            let map = wrong.as_object_mut().unwrap();
            map.insert("kind".into(), JsonNode::String(kind_name.to_string()));
            map.insert("extent".into(), extent.clone());
            assert_eq!(
                ContentRef::from_transport_json(&wrong)
                    .expect_err(&format!("kind={kind_name} extent={extent}"))
                    .code(),
                code::EXTENT,
                "kind={kind_name}"
            );
        }
    }
}

#[cfg(test)]
mod metered_key_body_tests {
    use super::*;
    /// 键比较的读集合：一张 4 桶 hashbrown 表（公式见 work::hashbrown_layout；aarch64 140，x86_64 sse2 148）。
    fn key_read_set() -> usize {
        capture_allocation::work::hashbrown_layout::table_bytes_of::<NodeDigest>(4)
    }
    /// 键比较峰值：读集合 + 两个各 1 槽的叶迭代栈（`LeafIter`，与平台无关；aarch64 204，x86_64 sse2 212）。
    fn key_peak() -> usize {
        key_read_set() + 2 * std::mem::size_of::<NodeDigest>()
    }
    #[test]
    fn borrowed_key_two_chunks_fit_without_body_layouts_or_copies() {
        let mut nodes = MemoryNodes::new();
        let a = write_value(&mut nodes, &Value::string("a".repeat(4096))).unwrap();
        let b = write_value(&mut nodes, &Value::string("b".repeat(4096))).unwrap();
        let (capture, budget, base) = slab_capture(nodes, key_peak());
        let before = budget.snapshot();
        let f = frame_calls_on_thread();
        let mut source = CaptureReader(&capture);
        let mut verifier =
            Verifier::with_memo_capture(&mut source, Mode::Public, AuditMemo::default());
        assert_eq!(
            verifier.compare_key_streams(&a.digest, &b.digest).unwrap(),
            std::cmp::Ordering::Less
        );
        let after = budget.snapshot();
        assert_eq!(frame_calls_on_thread() - f, 6);
        assert_eq!(
            after.live_requested_layout_bytes,
            base + key_read_set(),
            "只有实际read集合持账"
        );
        assert_eq!(
            after.peak_requested_layout_bytes,
            base + key_peak(),
            "集合与两个叶迭代栈持账"
        );
        assert_eq!(after.allocation_attempts, before.allocation_attempts + 3);
        assert_eq!(after.copied_rows, before.copied_rows);
        assert_eq!(after.copied_body_bytes, before.copied_body_bytes);
        assert_eq!(after.rejected_layouts, before.rejected_layouts);
        drop(verifier);
        drop(source);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    fn slab_capture(
        nodes: MemoryNodes,
        extra: usize,
    ) -> (AuditCapture, capture_allocation::OwnedBudget, usize) {
        let count = nodes.nodes.len();
        let bytes = nodes.nodes.values().map(|(_, b)| b.len()).sum();
        let base = capture_allocation::CaptureSlab::requested_bytes(count, bytes).unwrap();
        let budget = capture_allocation::OwnedBudget::new(base + extra);
        let mut slab = capture_allocation::CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, (t, b)) in nodes.nodes {
            slab.push(d, t, &b).unwrap();
        }
        (
            AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap()),
            budget,
            base,
        )
    }
    fn captured_compare(
        capture: &AuditCapture,
        a: &NodeDigest,
        b: &NodeDigest,
    ) -> Result<std::cmp::Ordering, CodecError> {
        let mut source = CaptureReader(capture);
        Verifier::with_memo_capture(&mut source, Mode::Public, AuditMemo::default())
            .compare_key_streams(a, b)
    }
    #[test]
    fn metered_key_success_real_two_chunks_and_full_object_verifier() {
        let mut nodes = MemoryNodes::new();
        let a = write_value(&mut nodes, &Value::string("a".repeat(4096))).unwrap();
        let b = write_value(&mut nodes, &Value::string("b".repeat(4096))).unwrap();
        let mut legacy = nodes.clone();
        let before_f = frame_calls_on_thread();
        let expected = Verifier::new(&mut legacy, Mode::Public)
            .compare_key_streams(&a.digest, &b.digest)
            .unwrap();
        let legacy_f = frame_calls_on_thread() - before_f;
        let (capture, budget, base) = slab_capture(nodes, 8192 + 32 + key_peak());
        let before = budget.snapshot();
        let before_f = frame_calls_on_thread();
        assert_eq!(
            captured_compare(&capture, &a.digest, &b.digest).unwrap(),
            expected
        );
        assert_eq!(frame_calls_on_thread() - before_f, legacy_f);
        let after = budget.snapshot();
        assert_eq!(after.peak_requested_layout_bytes, base + key_peak());
        assert_eq!(after.allocated_layouts, before.allocated_layouts + 3);
        assert_eq!(after.released_layouts, before.released_layouts + 3);
        assert_eq!(after.copied_rows, before.copied_rows);
        assert_eq!(after.copied_body_bytes, before.copied_body_bytes);
        assert_eq!(after.live_requested_layout_bytes, base);
        assert_eq!(after.rejected_layouts, 0);
        println!(
            "METERED_KEY_TWO_BODY_RECEIPT {}",
            serde_json::to_string(&after).unwrap()
        );
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        // 真实 AuditCapture.verify 的对象字段顺序检查进入同一键比较路径。
        let mut nodes = MemoryNodes::new();
        let object = write_value(
            &mut nodes,
            &Value::Object(vec![
                ("a".repeat(4096), Value::Null),
                ("b".repeat(4096), Value::Null),
            ]),
        )
        .unwrap();
        let reference = ContentRef::new(object.kind, &object.digest, object.extent);
        let (capture, budget, base) = slab_capture(nodes, 32768);
        let copied_before = budget.snapshot().copied_rows;
        capture.verify(&reference, Mode::Public).unwrap();
        assert_eq!(budget.snapshot().copied_rows, copied_before);
        assert!(
            budget.snapshot().live_requested_layout_bytes > base,
            "成功capture Public memo的真实布局必须持续活到capture Drop"
        );
        assert!(
            budget.snapshot().peak_requested_layout_bytes > base,
            "完整核验的实际memo与栈仍计量"
        );
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn metered_key_empty_order_prefix_and_multiple_chunks_preserve_results() {
        for (case, (a, b)) in [
            ("".to_owned(), "x".to_owned()),
            ("x".to_owned(), "".to_owned()),
            ("a".repeat(4096) + "x", "a".repeat(4096) + "y"),
            ("a".repeat(4096), "a".repeat(4096) + "x"),
            ("identical".to_owned(), "identical".to_owned()),
        ]
        .into_iter()
        .enumerate()
        {
            let mut nodes = MemoryNodes::new();
            let a = write_value(&mut nodes, &Value::string(a)).unwrap();
            let b = write_value(&mut nodes, &Value::string(b)).unwrap();
            let mut legacy = nodes.clone();
            let before_f = frame_calls_on_thread();
            let expected = Verifier::new(&mut legacy, Mode::Public)
                .compare_key_streams(&a.digest, &b.digest)
                .unwrap();
            let expected_f = frame_calls_on_thread() - before_f;
            let (capture, budget, base) = slab_capture(nodes, 16384);
            let before_f = frame_calls_on_thread();
            assert_eq!(
                captured_compare(&capture, &a.digest, &b.digest).unwrap(),
                expected
            );
            assert_eq!(frame_calls_on_thread() - before_f, expected_f);
            println!(
                "METERED_KEY_CASE_RECEIPT {}",
                serde_json::json!({"case":case,"base":base,"a_extent":a.extent,"b_extent":b.extent,"meter":budget.snapshot()})
            );
            assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
            drop(capture);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn metered_key_bad_original_shape_missing_and_rollback() {
        for expected in [
            code::NODE_DIGEST,
            code::SEQUENCE_SHAPE,
            code::BYTE_CHUNK_SHAPE,
            code::NODE_MISSING,
        ] {
            let mut nodes = MemoryNodes::new();
            let b = write_value(&mut nodes, &Value::string("b")).unwrap();
            let chunk = match expected {
                code::BYTE_CHUNK_SHAPE => {
                    let d = frame(tag::NULL, &[]);
                    nodes.insert_unchecked(d, tag::NULL, vec![]);
                    d
                }
                code::NODE_MISSING => [0x55; 32],
                _ => {
                    let d = frame(tag::BYTES_CHUNK, b"a");
                    nodes.insert_unchecked(d, tag::BYTES_CHUNK, b"a".to_vec());
                    d
                }
            };
            let sequence = if expected == code::SEQUENCE_SHAPE {
                let d = frame(tag::SEQ_LEAF, b"bad");
                nodes.insert_unchecked(d, tag::SEQ_LEAF, b"bad".to_vec());
                d
            } else {
                let d = frame(tag::SEQ_LEAF, &chunk);
                nodes.insert_unchecked(d, tag::SEQ_LEAF, chunk.to_vec());
                d
            };
            if expected == code::NODE_DIGEST {
                nodes.insert_unchecked(chunk, tag::BYTES_CHUNK, b"changed".to_vec());
            }
            let mut header = 1u64.to_be_bytes().to_vec();
            header.extend_from_slice(&sequence);
            let a = frame(tag::STRING, &header);
            nodes.insert_unchecked(a, tag::STRING, header);
            let mut legacy = nodes.clone();
            let legacy_error = Verifier::new(&mut legacy, Mode::Public)
                .compare_key_streams(&a, &b.digest)
                .unwrap_err();
            assert_eq!(legacy_error.code(), expected);
            let (capture, budget, base) = slab_capture(nodes, 8192);
            assert_eq!(
                captured_compare(&capture, &a, &b.digest).unwrap_err(),
                legacy_error
            );
            assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
            drop(capture);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn metered_key_dynamic_source_err_is_not_overridden_by_capture() {
        struct FailOnRead {
            nodes: MemoryNodes,
            reads: u64,
            reject: u64,
        }
        impl NodeSource for FailOnRead {
            fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
                self.reads += 1;
                if self.reads == self.reject {
                    return Err(CodecError::new(code::SOURCE, "dynamic exact failure"));
                }
                self.nodes.read(d)
            }
        }
        let mut nodes = MemoryNodes::new();
        let a = write_value(&mut nodes, &Value::string("a".repeat(4096))).unwrap();
        let b = write_value(&mut nodes, &Value::string("b".repeat(4096))).unwrap();
        let (capture, budget, base) = slab_capture(nodes.clone(), 0);
        for reject in 1..=6 {
            let mut dynamic = FailOnRead {
                nodes: nodes.clone(),
                reads: 0,
                reject,
            };
            let mut verifier = Verifier::new(&mut dynamic, Mode::Public);
            assert!(matches!(verifier.source, CodecContext::Source(_)));
            let error = verifier
                .compare_key_streams(&a.digest, &b.digest)
                .unwrap_err();
            assert_eq!(error.code(), code::SOURCE);
            assert_eq!(error.detail(), "dynamic exact failure");
            assert_eq!(dynamic.reads, reject);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
            assert_eq!(budget.snapshot().allocated_layouts, 2);
        }
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}

#[cfg(test)]
mod metered_verifier_body_tests {
    use super::*;
    fn captured_string(
        text: &str,
        extra: usize,
    ) -> (
        AuditCapture,
        capture_allocation::OwnedBudget,
        usize,
        ContentRef,
        NodeDigest,
    ) {
        let mut nodes = MemoryNodes::new();
        let built = write_value(&mut nodes, &Value::string(text)).unwrap();
        let seq = digest_at(nodes.get(&built.digest).unwrap().1, 8);
        let reference = ContentRef::new(built.kind, &built.digest, built.extent);
        let count = nodes.nodes.len();
        let bytes = nodes.nodes.values().map(|(_, b)| b.len()).sum();
        let base = capture_allocation::CaptureSlab::requested_bytes(count, bytes).unwrap();
        let budget = capture_allocation::OwnedBudget::new(base + extra);
        let mut slab = capture_allocation::CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, (t, b)) in nodes.nodes {
            slab.push(d, t, &b).unwrap();
        }
        (
            AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap()),
            budget,
            base,
            reference,
            seq,
        )
    }
    #[test]
    fn borrowed_verifier_header_checks_f_before_actual_workset_refusal() {
        let (capture, budget, base, reference, _) = captured_string("a", 39);
        let before = budget.snapshot();
        let f = frame_calls_on_thread();
        let error = capture.verify(&reference, Mode::Public).unwrap_err();
        assert_eq!(error.code(), capture_allocation::LIMIT);
        let after = budget.snapshot();
        assert_eq!(after.allocated_layouts, before.allocated_layouts);
        assert_eq!(after.allocation_attempts, before.allocation_attempts);
        assert_eq!(after.copied_rows, before.copied_rows);
        assert_eq!(after.copied_body_bytes, before.copied_body_bytes);
        assert_eq!(
            frame_calls_on_thread() - f,
            1,
            "借用header仍核本次F，再遇真实容器额度门"
        );
        assert_eq!(after.live_requested_layout_bytes, base);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn borrowed_verifier_byte_agg_checks_leaf_without_body_copy() {
        let (capture, budget, base, _, seq) = captured_string(&"a".repeat(4096), 8192);
        let before = budget.snapshot();
        let f = frame_calls_on_thread();
        let mut reader = CaptureReader(&capture);
        let mut verifier =
            Verifier::with_memo_capture(&mut reader, Mode::Public, AuditMemo::default());
        let agg = verifier.byte_agg(&seq).unwrap();
        assert_eq!(agg.total_bytes, 4096);
        assert_eq!(frame_calls_on_thread() - f, 2);
        let after = budget.snapshot();
        assert_eq!(after.copied_rows, before.copied_rows);
        assert_eq!(after.copied_body_bytes, before.copied_body_bytes);
        assert!(
            after.live_requested_layout_bytes > base,
            "实际工作集合与memo持续计量"
        );
        drop(verifier);
        drop(reader);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    fn from_nodes(
        nodes: MemoryNodes,
        extra: usize,
    ) -> (AuditCapture, capture_allocation::OwnedBudget, usize) {
        let count = nodes.nodes.len();
        let bytes = nodes.nodes.values().map(|(_, b)| b.len()).sum();
        let base = capture_allocation::CaptureSlab::requested_bytes(count, bytes).unwrap();
        let budget = capture_allocation::OwnedBudget::new(base + extra);
        let mut slab = capture_allocation::CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, (t, b)) in nodes.nodes {
            slab.push(d, t, &b).unwrap();
        }
        (
            AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap()),
            budget,
            base,
        )
    }
    fn traced_verify(
        capture: &AuditCapture,
        reference: &ContentRef,
        mode: Mode,
    ) -> (Result<(), CodecError>, Vec<(NodeDigest, u64)>, u64) {
        let f = frame_calls_on_thread();
        CAPTURE_BODY_READ_TRACE.with(|t| {
            assert!(t.borrow().is_none());
            *t.borrow_mut() = Some((f, Vec::new()));
        });
        let result = capture.verify(reference, mode);
        let trace = CAPTURE_BODY_READ_TRACE.with(|t| t.borrow_mut().take().unwrap().1);
        (result, trace, frame_calls_on_thread() - f)
    }
    #[test]
    fn metered_verifier_all_shapes_context_errors_and_read_f_order_match_plain() {
        let values = vec![
            Value::Array(vec![
                Value::Null,
                Value::Bool(true),
                Value::Bool(false),
                Value::Number(NumberToken::parse("-1.25e+8").unwrap()),
                Value::string("汉".repeat(3000)),
                Value::Object(vec![
                    ("z".into(), Value::Null),
                    ("a".into(), Value::Bool(true)),
                ]),
            ]),
            Value::JsonText(Box::new(JsonValue::Object(vec![
                (
                    "n".into(),
                    JsonValue::Number(NumberToken::parse("1.20").unwrap()),
                ),
                ("s".into(), JsonValue::String("汉x".repeat(1200))),
            ]))),
            Value::Array(vec![Value::BatchSelf, Value::string("ok")]),
            Value::Object(vec![
                ("a".repeat(8192) + "x", Value::Null),
                ("a".repeat(8192) + "y", Value::Null),
            ]),
        ];
        for (case, value) in values.into_iter().enumerate() {
            for mode in [Mode::Public, Mode::Template] {
                let mut nodes = MemoryNodes::new();
                let built = write_template(&mut nodes, &value).unwrap();
                let legacy = AuditCapture::new(nodes.clone());
                let (owned, budget, base) = from_nodes(nodes, 32768);
                let plain = traced_verify(&legacy, &built.content_ref(), mode);
                let before = budget.snapshot();
                let bounded = traced_verify(&owned, &built.content_ref(), mode);
                assert_eq!(budget.snapshot().copied_rows, before.copied_rows);
                assert_eq!(
                    budget.snapshot().copied_body_bytes,
                    before.copied_body_bytes
                );
                assert_eq!(bounded, plain, "case {case}, {mode:?}");
                if bounded.0.is_ok() {
                    assert!(
                        budget.snapshot().live_requested_layout_bytes > base,
                        "成功memo布局持续活到capture Drop"
                    );
                } else {
                    assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
                }
                assert_eq!(owned.reads.get(), legacy.reads.get());
                println!(
                    "VERIFIER_ALL_SHAPES_RECEIPT {}",
                    serde_json::json!({"case":case,"mode":format!("{mode:?}"),"base":base,"result":format!("{:?}",bounded.0),"reads":bounded.1.len(),"f_calls":bounded.2,"meter":budget.snapshot()})
                );
                drop(owned);
                assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
            }
        }
        // 正确F但非法数字内容、非法UTF8、未知tag、坏原件、Missing、根kind/extent错误。
        for case in 0..7 {
            let mut nodes = MemoryNodes::new();
            let (tag, bytes) = match case {
                0 => (tag::NUMBER, b"not-a-number".to_vec()),
                1 => (tag::STRING, vec![0xff]),
                _ => (tag::STRING, b"abc".to_vec()),
            };
            let leaf = frame(tag::BYTES_CHUNK, &bytes);
            nodes.insert_unchecked(leaf, tag::BYTES_CHUNK, bytes.clone());
            let seq = frame(tag::SEQ_LEAF, &leaf);
            nodes.insert_unchecked(seq, tag::SEQ_LEAF, leaf.to_vec());
            let mut header = (bytes.len() as u64).to_be_bytes().to_vec();
            header.extend_from_slice(&seq);
            let root = frame(tag, &header);
            nodes.insert_unchecked(root, tag, header);
            if case == 2 {
                nodes.insert_unchecked(leaf, tag::BYTES_CHUNK, b"wrong".to_vec());
            }
            if case == 3 {
                nodes.nodes.remove(&leaf);
            }
            let mut reference = ContentRef::new(
                Kind::from_tag(tag).unwrap(),
                &root,
                Some(bytes.len() as u64),
            );
            if case == 4 {
                reference.kind = Kind::Array;
            }
            if case == 5 {
                reference.extent = Some(99);
            }
            if case == 6 {
                let d = frame(0xee, &[]);
                nodes.insert_unchecked(d, 0xee, vec![]);
                reference.root = root_string(&d);
            }
            let legacy = AuditCapture::new(nodes.clone());
            let (owned, budget, base) = from_nodes(nodes, 8192);
            let plain = traced_verify(&legacy, &reference, Mode::Public);
            let bounded = traced_verify(&owned, &reference, Mode::Public);
            assert!(plain.0.is_err());
            assert_eq!(bounded, plain, "invalid case {case}");
            assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
            assert!(owned.public.borrow().seen.is_empty());
            assert!(owned.public.borrow().pushed.is_empty());
            assert!(owned.public.borrow().bytes_memo.is_empty());
            drop(owned);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn metered_verifier_failed_run_does_not_poison_next_root_or_context() {
        let mut nodes = MemoryNodes::new();
        let good = write_value(&mut nodes, &Value::string("good")).unwrap();
        let bad = write_value(&mut nodes, &Value::string("bad")).unwrap();
        let bad_leaf = frame(tag::BYTES_CHUNK, b"bad");
        nodes.insert_unchecked(bad_leaf, tag::BYTES_CHUNK, b"corrupt".to_vec());
        let array = write_value(
            &mut nodes,
            &Value::Array(vec![Value::Edge(bad.digest), Value::Edge(good.digest)]),
        )
        .unwrap();
        let template = write_template(
            &mut nodes,
            &Value::Array(vec![Value::BatchSelf, Value::Edge(good.digest)]),
        )
        .unwrap();
        let legacy = AuditCapture::new(nodes.clone());
        let (owned, budget, base) = from_nodes(nodes, 8192);
        for (reference, mode) in [
            (array.content_ref(), Mode::Public),
            (good.content_ref(), Mode::Public),
            (array.content_ref(), Mode::Public),
            (template.content_ref(), Mode::Template),
            (template.content_ref(), Mode::Public),
            (good.content_ref(), Mode::Public),
        ] {
            assert_eq!(
                traced_verify(&owned, &reference, mode),
                traced_verify(&legacy, &reference, mode)
            );
            assert!(
                budget.snapshot().live_requested_layout_bytes >= base,
                "缓存允许持账，但后续root与普通算法对拍必须相同"
            );
        }
        drop(owned);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }

    #[test]
    fn borrowed_verifier_nested_headers_and_leaf_do_not_allocate_body_layouts() {
        for extra in [4207, 8192] {
            let (capture, budget, base, reference, _) = captured_string(&"a".repeat(4096), extra);
            let before = budget.snapshot();
            let f = frame_calls_on_thread();
            capture.verify(&reference, Mode::Public).unwrap();
            let after = budget.snapshot();
            assert_eq!(frame_calls_on_thread() - f, 4);
            assert_eq!(after.copied_rows, before.copied_rows);
            assert_eq!(after.copied_body_bytes, before.copied_body_bytes);
            // seen、pushed、read_digests、bytes_memo 各一张 4 桶表 + todo 4 槽（公式见 work::hashbrown_layout；
            // aarch64 868，x86_64 sse2 916）。
            use capture_allocation::work::hashbrown_layout::table_bytes_of as t;
            let public_memo = 2 * t::<(NodeDigest, Flavor)>(4)
                + t::<NodeDigest>(4)
                + t::<((NodeDigest, Utf8State, NumState), ByteAgg)>(4)
                + 4 * std::mem::size_of::<(NodeDigest, Flavor)>();
            assert_eq!(
                after.live_requested_layout_bytes,
                base + public_memo,
                "成功memo仍持账"
            );
            assert!(
                after.peak_requested_layout_bytes < base + 4096,
                "4096B叶不再新增body布局"
            );
            drop(capture);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }
}

/// #1394 hashbrown 0.17.1 布局公式锁：逐张 memo 表断言公式值 == 实际 allocation_size。aarch64（组宽 8）
/// 与 x86_64 sse2（组宽 16）都须通过；上面几处由公式推出的期望值以此为前提。hashbrown 升级改了布局时本锁先红。
#[cfg(test)]
mod hashbrown_layout_formula_lock {
    use super::*;
    #[test]
    fn memo_tables_match_hashbrown_layout_formula() {
        let mut nodes = MemoryNodes::new();
        let built = write_value(
            &mut nodes,
            &Value::Object(vec![
                (
                    "a".repeat(4096),
                    Value::Array(
                        (0..40)
                            .map(|i| Value::string(format!("item-{i}")))
                            .collect(),
                    ),
                ),
                ("b".repeat(4096), Value::string("z".repeat(5000))),
                (
                    "c".repeat(64),
                    Value::Object(vec![("k".into(), Value::Null)]),
                ),
            ]),
        )
        .unwrap();
        let reference = ContentRef::new(built.kind, &built.digest, built.extent);
        let count = nodes.nodes.len();
        let bytes = nodes.nodes.values().map(|(_, b)| b.len()).sum();
        let base = capture_allocation::CaptureSlab::requested_bytes(count, bytes).unwrap();
        let budget = capture_allocation::OwnedBudget::new(base + (1 << 20));
        let mut slab = capture_allocation::CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, (t, b)) in nodes.nodes {
            slab.push(d, t, &b).unwrap();
        }
        let capture = AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        capture.verify(&reference, Mode::Public).unwrap();
        let memo = capture.public.borrow();
        let mut checked = Vec::new();
        for (name, table) in [
            ("seen", memo.seen.layout_vs_formula()),
            ("pushed", memo.pushed.layout_vs_formula()),
            ("read_digests", memo.read_digests.layout_vs_formula()),
            ("bytes_memo", memo.bytes_memo.layout_vs_formula()),
            ("elem_memo", memo.elem_memo.layout_vs_formula()),
            ("field_memo", memo.field_memo.layout_vs_formula()),
            ("key_cmp", memo.key_cmp.layout_vs_formula()),
        ] {
            if let Some((actual, formula)) = table {
                assert_eq!(
                    actual,
                    formula,
                    "{name}：实际 allocation_size 与 hashbrown 0.17.1 公式不符（组宽 {}）",
                    capture_allocation::work::hashbrown_layout::GROUP_WIDTH
                );
                checked.push(name);
            }
        }
        assert_eq!(
            checked,
            [
                "seen",
                "pushed",
                "read_digests",
                "bytes_memo",
                "elem_memo",
                "field_memo",
                "key_cmp"
            ],
            "七张 memo 表都须分配并逐张核对"
        );
    }
}

#[cfg(test)]
mod codec_workset_budget_probe {
    use super::*;
    #[test]
    fn captured_public_template_memo_next_layout_refuses_before_allocate() {
        let mut nodes = MemoryNodes::new();
        let built = write_value(
            &mut nodes,
            &Value::Array(
                (0..120)
                    .map(|i| Value::string(format!("shared-{i}")))
                    .collect(),
            ),
        )
        .unwrap();
        let count = nodes.nodes.len();
        let bytes = nodes.nodes.values().map(|(_, b)| b.len()).sum();
        let base = capture_allocation::CaptureSlab::requested_bytes(count, bytes).unwrap();
        let budget = capture_allocation::OwnedBudget::new(base + 4096);
        let mut slab = capture_allocation::CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, (t, b)) in nodes.nodes {
            slab.push(d, t, &b).unwrap();
        }
        let cap = AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap());
        let result = cap.verify(&built.content_ref(), Mode::Public);
        let retention = cap.retention();
        println!(
            "MEMO_RED_RECEIPT {}",
            serde_json::json!({"base":base,"meter":budget.snapshot(),"memo":retention["public"]})
        );
        assert!(
            result.is_err(),
            "真实memo增长超过4096余量必须写前拒绝，实得{result:?}"
        );
        assert_eq!(result.unwrap_err().code(), capture_allocation::LIMIT);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        let result = cap.verify(&built.content_ref(), Mode::Template);
        assert!(result.is_err());
        assert_eq!(cap.original(&built.digest).unwrap().0, tag::ARRAY);
        drop(cap);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}

#[doc(hidden)]
pub fn verify_refview(
    source: &mut dyn NodeSource,
    r: RefView<'_>,
    mode: Mode,
) -> Result<(), CodecError> {
    Verifier::new(source, mode).verify_refview(r).map(|_| ())
}
#[doc(hidden)]
pub fn render_refview(
    source: &mut dyn NodeSource,
    r: RefView<'_>,
    out: &mut dyn Write,
) -> Result<(), CodecError> {
    render_refview_ctx(CodecContext::Source(source), r, out)
}

#[doc(hidden)]
pub fn audit_verify_many_refview<'a>(
    source: &mut dyn NodeSource,
    refs: impl IntoIterator<Item = RefView<'a>>,
    mode: Mode,
) -> Result<(), CodecError> {
    let mut verifier = Verifier::new(source, mode);
    for r in refs {
        verifier.verify_refview(r)?;
    }
    Ok(())
}

#[cfg(feature = "s_allocation_json_testonly")]
#[doc(hidden)]
pub fn write_value_view_for_allocation_prototype(
    sink: &mut dyn NodeSink,
    v: ValueView<'_>,
    b: &capture_allocation::OwnedBudget,
) -> Result<Built, CodecError> {
    write_value_view_ctx(
        &mut CodecContext::Sink(sink, Some(b.clone())),
        v,
        Mode::Public,
    )
}

/// 只格式化摘要，不携带节点kind/extent或来源证明，不分配String。
pub struct FixedRoot([u8; 68]);
impl FixedRoot {
    pub fn new(digest: &NodeDigest) -> Self {
        const H: &[u8; 16] = b"0123456789abcdef";
        let mut bytes = [0; 68];
        bytes[..4].copy_from_slice(ROOT_PREFIX.as_bytes());
        for (i, b) in digest.iter().enumerate() {
            bytes[4 + 2 * i] = H[(b >> 4) as usize];
            bytes[5 + 2 * i] = H[(b & 15) as usize];
        }
        Self(bytes)
    }
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("private canonical root ASCII")
    }
}

#[cfg(test)]
mod sha256_backend_tests {
    use sha2::{Digest, Sha256};

    #[test]
    fn padding_boundaries_and_streaming_keep_standard_sha256_bytes() {
        // 固定独立 hashlib 向量；后端选择不能改变算法、padding 或分段 update 语义。
        let vectors: &[(usize, &str)] = &[
            (
                0,
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
            ),
            (
                55,
                "463eb28e72f82e0a96c0a4cc53690c571281131f672aa229e0d45ae59b598b59", // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
            ),
            (
                56,
                "da2ae4d6b36748f2a318f23e7ab1dfdf45acdc9d049bd80e59de82a60895f562", // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
            ),
            (
                63,
                "29af2686fd53374a36b0846694cc342177e428d1647515f078784d69cdb9e488", // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
            ),
            (
                64,
                "fdeab9acf3710362bd2658cdc9a29e8f9c757fcf9811603a8c447cd1d9151108", // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
            ),
            (
                65,
                "4bfd2c8b6f1eec7a2afeb48b934ee4b2694182027e6d0fc075074f2fabb31781", // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
            ),
            (
                127,
                "92ca0fa6651ee2f97b884b7246a562fa71250fedefe5ebf270d31c546bfea976", // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
            ),
            (
                128,
                "471fb943aa23c511f6f72f8d1652d9c880cfa392ad80503120547703e56a2be5", // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
            ),
            (
                129,
                "5099c6a56203f9687f7d33f4bfdf576d31dc91f6b695ecea38b2770c87631135", // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
            ),
            (
                4096,
                "d67c656e01756650d77717b0839985a056ec28ffe174601d690fc407a2ceffca", // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
            ),
        ];
        assert_eq!(
            format!("{:x}", Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" // DevSkim: ignore DS173237 SHA-256 摘要/测试向量，非凭据
        );
        for &(len, expected) in vectors {
            let input: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
            assert_eq!(
                format!("{:x}", Sha256::digest(&input)),
                expected,
                "len={len}"
            );
            for chunk in [1, 7, 63, 64, 65] {
                let mut hash = Sha256::new();
                for part in input.chunks(chunk) {
                    hash.update(part);
                }
                assert_eq!(
                    format!("{:x}", hash.finalize()),
                    expected,
                    "len={len} chunk={chunk}"
                );
            }
        }
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod request_original_tuple_tests {
    use super::*;
    #[test]
    fn request_original_tuple_binding_compares_body_tag_digest_and_complete_set() {
        let mut nodes = MemoryNodes::new();
        nodes.nodes.insert([1; 32], (7, vec![1, 2, 3]));
        let original = AuditCapture::new(nodes.clone())
            .enable_native_frame_reuse_for_testonly()
            .unwrap();
        assert!(original
            .exact_original_tuples_equal(&AuditCapture::new(nodes.clone()))
            .unwrap());
        for variant in 0..4 {
            let mut changed = nodes.clone();
            match variant {
                0 => changed.nodes.get_mut(&[1; 32]).unwrap().1[2] = 4,
                1 => changed.nodes.get_mut(&[1; 32]).unwrap().0 = 8,
                2 => {
                    changed.nodes.clear();
                }
                _ => {
                    changed.nodes.insert([2; 32], (7, vec![1, 2, 3]));
                }
            }
            assert!(!original
                .exact_original_tuples_equal(&AuditCapture::new(changed))
                .unwrap());
        }
    }
}

#[cfg(test)]
mod r6_codec_memo_tests {
    use super::*;
    use capture_allocation::scratch::{
        CodecWork, ScratchCharge, ScratchFailure, ScratchParent, WorkScope,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    struct Parent(AtomicUsize);
    impl ScratchParent for Parent {
        fn health(&self) -> Result<(), ScratchFailure> {
            Ok(())
        }
        fn invalidate(&self) {
            panic!("正常生命周期不应损坏账本");
        }

        fn reserve(&self, n: usize, _: ScratchCharge) -> Result<u64, ScratchFailure> {
            self.0.fetch_add(n, Ordering::SeqCst);
            Ok(0)
        }
        fn release(&self, n: usize, _: ScratchCharge, origin: u64) -> Result<(), ScratchFailure> {
            if origin != 0 {
                return Err(ScratchFailure::Ledger);
            }
            assert!(self.0.fetch_sub(n, Ordering::SeqCst) >= n);
            Ok(())
        }
    }
    struct Scoped<'a> {
        nodes: &'a mut MemoryNodes,
        scope: &'a WorkScope,
    }
    impl NodeSource for Scoped<'_> {
        fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
            self.nodes.read(d)
        }
        fn codec_work(&self) -> Option<&WorkScope> {
            Some(self.scope)
        }
    }
    #[test]
    fn r6_codec_into_memo_preserves_actual_work_domain() {
        let mut nodes = MemoryNodes::new();
        let reference = write_value(
            &mut nodes,
            &Value::Array(vec![Value::String("same".into()); 8]),
        )
        .unwrap()
        .content_ref();
        let parent = Arc::new(Parent(AtomicUsize::new(0)));
        CodecWork::new(parent.clone(), 1024 * 1024)
            .unwrap()
            .run(|scope| {
                let mut source = Scoped {
                    nodes: &mut nodes,
                    scope,
                };
                let mut verifier = Verifier::new(&mut source, Mode::Public);
                verifier.verify_reference(&reference)?;
                let memo = verifier.into_memo();
                assert!(matches!(memo.budget, Some(WorkBudget::Scratch(_))));
                assert!(scope.snapshot().live > 0);
                // reborrow/context 的预算不能覆盖真实保留容器的预算。
                let mut resumed = Verifier::with_memo(&mut *source.nodes, Mode::Public, memo);
                resumed.verify_reference(&reference)?;
                let memo = resumed.into_memo();
                assert!(matches!(memo.budget, Some(WorkBudget::Scratch(_))));
                drop(memo);
                assert_eq!(scope.snapshot().live, 0);
                assert_eq!(parent.0.load(Ordering::SeqCst), CodecWork::metadata_bytes());
                Ok::<_, CodecError>(())
            })
            .unwrap();
        assert_eq!(parent.0.load(Ordering::SeqCst), 0);
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod native_owner_codec_bridge_tests {
    use super::*;
    use capture_allocation::scratch::{
        CodecWork, ScratchCharge, ScratchFailure, ScratchParent, WorkError, WorkScope,
    };
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };

    struct Parent {
        limit: usize,
        live: AtomicUsize,
        layout: AtomicUsize,
        native: AtomicUsize,
        native_limit: AtomicUsize,
        peak: AtomicUsize,
        failed: AtomicBool,
    }
    impl Parent {
        fn new(limit: usize, retained: usize) -> Arc<Self> {
            Arc::new(Self {
                limit,
                live: AtomicUsize::new(retained),
                layout: AtomicUsize::new(0),
                native: AtomicUsize::new(retained),
                native_limit: AtomicUsize::new(usize::MAX),
                peak: AtomicUsize::new(retained),
                failed: AtomicBool::new(false),
            })
        }
    }
    impl ScratchParent for Parent {
        fn reserve(&self, n: usize, kind: ScratchCharge) -> Result<u64, ScratchFailure> {
            if let ScratchCharge::Native = kind {
                if self
                    .native
                    .load(Ordering::SeqCst)
                    .checked_add(n)
                    .is_none_or(|total| total > self.native_limit.load(Ordering::SeqCst))
                {
                    return Err(ScratchFailure::Budget);
                }
            }
            let previous = self
                .live
                .try_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                    old.checked_add(n).filter(|next| *next <= self.limit)
                })
                .map_err(|_| ScratchFailure::Budget)?;
            self.peak.fetch_max(previous + n, Ordering::SeqCst);
            match kind {
                ScratchCharge::Layout => {
                    self.layout.fetch_add(n, Ordering::SeqCst);
                }
                ScratchCharge::Native => {
                    self.native.fetch_add(n, Ordering::SeqCst);
                }
                ScratchCharge::Metadata => {}
            }
            Ok(0)
        }
        fn release(
            &self,
            n: usize,
            kind: ScratchCharge,
            origin: u64,
        ) -> Result<(), ScratchFailure> {
            if origin != 0 {
                return Err(ScratchFailure::Ledger);
            }
            assert!(self.live.fetch_sub(n, Ordering::SeqCst) >= n);
            match kind {
                ScratchCharge::Layout => {
                    assert!(self.layout.fetch_sub(n, Ordering::SeqCst) >= n);
                }
                ScratchCharge::Native => {
                    assert!(self.native.fetch_sub(n, Ordering::SeqCst) >= n);
                }
                ScratchCharge::Metadata => {}
            }
            Ok(())
        }
        fn health(&self) -> Result<(), ScratchFailure> {
            if self.failed.load(Ordering::SeqCst) {
                Err(ScratchFailure::Ledger)
            } else {
                Ok(())
            }
        }
        fn invalidate(&self) {
            self.failed.store(true, Ordering::SeqCst);
        }
    }
    fn native(nodes: MemoryNodes) -> AuditCapture {
        AuditCapture::new(nodes)
            .enable_native_frame_reuse_for_testonly()
            .unwrap()
    }
    fn run<T>(
        parent: &Arc<Parent>,
        op: impl FnOnce(&WorkScope) -> Result<T, CodecError>,
    ) -> Result<T, WorkError<CodecError>> {
        CodecWork::new(parent.clone(), parent.limit)
            .unwrap()
            .run(op)
    }
    fn verify(
        capture: &AuditCapture,
        root: &ContentRef,
        mode: Mode,
    ) -> Result<(), WorkError<CodecError>> {
        let parent = Parent::new(32 << 20, capture.owner_codec_retained_bytes_for_testonly());
        run(&parent, |work| {
            capture.verify_owner_refview_for_testonly(root.into(), mode, work)
        })
    }
    fn append(capture: &mut AuditCapture, nodes: MemoryNodes) {
        let parent = Parent::new(32 << 20, capture.owner_codec_retained_bytes_for_testonly());
        run(&parent, |work| {
            // #1488 P2-A：活动域追加的正文须已物理计费，来源合同项才指得到它。
            for (digest, (tag, body)) in nodes.nodes {
                let mut codec = CodecVec::default();
                codec.try_reserve_native(work, body.len())?;
                for byte in body {
                    codec.try_push_native(byte, work)?;
                }
                assert!(capture.append_original_codec_for_owner_testonly(
                    digest,
                    tag,
                    codec,
                    usize::MAX,
                    work
                )?);
            }
            Ok(())
        })
        .unwrap();
    }
    fn operation_error(result: Result<(), WorkError<CodecError>>) -> CodecError {
        match result {
            Err(WorkError::Operation(error)) => error,
            other => panic!("expected codec semantic rejection, got {other:?}"),
        }
    }
    fn assert_plain_proofs(capture: &AuditCapture) {
        fn tree<K: Ord + Copy + std::fmt::Debug, V: Copy>(t: &OwnerProofTree<K, V>) {
            fn scan<K: Ord + Copy + std::fmt::Debug, V: Copy>(
                link: &OwnerProofLink<K, V>,
                low: Option<K>,
                high: Option<K>,
            ) -> (usize, usize, usize) {
                let Some(node) = link else {
                    return (0, 0, 0);
                };
                assert!(low.is_none_or(|key| key < node.key));
                assert!(high.is_none_or(|key| node.key < key));
                let (lh, ln, lb) = scan(&node.left, low, Some(node.key));
                let (rh, rn, rb) = scan(&node.right, Some(node.key), high);
                assert!(lh.abs_diff(rh) <= 1);
                assert_eq!(node.height, lh.max(rh) + 1);
                (
                    node.height,
                    ln + rn + 1,
                    lb + rb + std::mem::size_of_val(&**node),
                )
            }
            let (_, count, bytes) = scan(&t.root, None, None);
            assert_eq!(count, t.len());
            assert_eq!(bytes, t.retained_layout_bytes());
            assert_eq!(
                bytes,
                count * std::alloc::Layout::new::<OwnerProofNode<K, V>>().size()
            );
        }
        let proofs = capture.owner_codec.borrow();
        for memo in [&proofs.public, &proofs.template] {
            tree(&memo.seen);
            tree(&memo.bytes);
            tree(&memo.elems);
            tree(&memo.fields);
            tree(&memo.key_cmp);
        }
    }

    #[test]
    fn b2_owner_codec_cross_scope_preserves_only_success_and_charges_current_parent() {
        let mut nodes = MemoryNodes::new();
        let first = write_value(
            &mut nodes,
            &Value::Object(vec![
                (
                    "a".into(),
                    Value::Array((0..64).map(|i| Value::string(format!("v-{i}"))).collect()),
                ),
                ("z".into(), Value::Null),
            ]),
        )
        .unwrap()
        .content_ref();
        let capture = native(nodes);
        let old_parent = Parent::new(32 << 20, 0);
        run(&old_parent, |work| {
            capture.verify_owner_refview_for_testonly((&first).into(), Mode::Public, work)?;
            assert_eq!(work.snapshot().live, 0, "scratch 已析构");
            Ok(())
        })
        .unwrap();
        let retained = capture.owner_codec_retained_bytes_for_testonly();
        assert!(retained > 0);
        assert_plain_proofs(&capture);
        assert_eq!(old_parent.layout.load(Ordering::SeqCst), 0);
        // P1c（control/1488-budget-full-chain-counterexample-fix-v2.md:9，959d860065）：证明节点实际载荷
        // 持原父许可到真实析构，不再 retain 丢许可；旧 scope 已析构，父引用只由这些许可持有（末尾 drop 核归零）。
        // 0088 P3-2：精确计数（探针 close-0085 log 20261021-212331-46ca88b909：strong=268，
        // 即本地 1 + 证明节点许可 267），固定输入下确定。
        assert_eq!(
            Arc::strong_count(&old_parent),
            268,
            "证明许可持原父账到真实析构"
        );
        let old_charge = old_parent.live.load(Ordering::SeqCst);
        assert!(old_charge >= retained, "Native 旧许可保守留父账");
        let capture = capture.finish_owner_step_for_testonly().unwrap();
        assert_eq!(capture.owner_codec_retained_bytes_for_testonly(), retained);
        let current_parent = Parent::new(32 << 20, retained);
        run(&current_parent, |work| {
            capture.verify_owner_refview_for_testonly((&first).into(), Mode::Public, work)?;
            assert!(work.snapshot().attempts > 0, "新根 header scratch 记本轮账");
            assert_eq!(work.snapshot().live, 0);
            Ok(())
        })
        .unwrap();
        assert_eq!(capture.reads.get(), 1, "warm 只重核真实根 header");
        assert_eq!(capture.owner_codec_retained_bytes_for_testonly(), retained);
        assert_eq!(current_parent.live.load(Ordering::SeqCst), retained);
        assert!(
            current_parent.peak.load(Ordering::SeqCst) > retained + CodecWork::metadata_bytes()
        );
        assert_eq!(old_parent.live.load(Ordering::SeqCst), old_charge);
        assert_eq!(Arc::strong_count(&current_parent), 1);
        drop(capture);
        assert_eq!(
            old_parent.live.load(Ordering::SeqCst),
            0,
            "真实析构全额还原父账"
        );
        assert_eq!(Arc::strong_count(&old_parent), 1, "析构后不留旧父引用");
    }

    /// #1572 任务D：capture 物理总额的常数时间读数=逐许可清单之和=capture 析构下降（三方对拍）；
    /// 有未绑定的 Plain 正文时快读数拒给总额。
    #[test]
    fn i1572_native_physical_fast_matches_listing_and_drop() {
        let members = ["alpha", "beta-gamma"];
        let mut additions = MemoryNodes::new();
        let root = write_value(
            &mut additions,
            &Value::Array(members.iter().map(|m| Value::string(*m)).collect()),
        )
        .unwrap()
        .content_ref();
        let mut capture = native(MemoryNodes::new());
        let parent = Parent::new(32 << 20, 0);
        run(&parent, |work| {
            for (digest, (tag, body)) in additions.nodes {
                let mut codec = CodecVec::default();
                // 容量故意大于长度：正文许可按真实容量。
                codec.try_reserve_native(work, body.len() + 5)?;
                for byte in &body {
                    codec.try_push_native(*byte, work)?;
                }
                assert!(capture.append_original_codec_for_owner_testonly(
                    digest,
                    tag,
                    codec,
                    usize::MAX,
                    work
                )?);
            }
            capture.verify_owner_refview_for_testonly((&root).into(), Mode::Public, work)
        })
        .unwrap();
        let fast = capture.native_physical_fast_for_testonly().unwrap();
        assert!(fast.owner_codec > 0 && fast.node_index > 0);
        assert!(fast.nodes >= 3);
        let total = fast
            .total_if_all_bound(capture.native_bound_body_nodes_for_testonly())
            .unwrap();
        let mut entries = Vec::new();
        capture
            .native_live_permit_entries_for_testonly(&mut entries)
            .unwrap();
        let listed = entries.iter().map(|e| e.bytes()).sum::<usize>();
        drop(entries);
        let before = parent.native.load(Ordering::SeqCst);
        drop(capture);
        let dropped = before - parent.native.load(Ordering::SeqCst);
        assert_eq!((total, listed), (dropped, dropped), "三方对拍");
        // 负控：无原件费转换留下 Plain 正文，bound<nodes，快读数拒给总额。
        let mut plain = MemoryNodes::new();
        write_value(&mut plain, &Value::string("plain")).unwrap();
        let plain = native(plain);
        let fast = plain.native_physical_fast_for_testonly().unwrap();
        let error = fast
            .total_if_all_bound(plain.native_bound_body_nodes_for_testonly())
            .unwrap_err();
        assert!(error.to_string().contains("正文未全绑定"), "{error}");
    }

    /// #1572：同一正文的来源合同（ns 的 len+97）与物理容量（nc 的 capacity）指同一张许可。
    /// 物理清单每张许可恰一行，SourceClaim 不产生第二张许可；顺序无关；删项、指错、重复、互换
    /// backing、改额、混入另一父账与来源域不符均拒。
    #[test]
    fn i1572_source_claim_backs_single_body_permit_and_rejects_tampering() {
        use capture_allocation::scratch::PermitId;
        type Built = (
            AuditCapture,
            Arc<Parent>,
            (usize, usize, usize),
            (usize, u64),
        );
        fn appended(base: &str, members: [&str; 2]) -> Built {
            let mut nodes = MemoryNodes::new();
            write_value(&mut nodes, &Value::string(base)).unwrap();
            let mut capture = native(nodes);
            let mut additions = MemoryNodes::new();
            write_value(
                &mut additions,
                &Value::Array(members.iter().map(|m| Value::string(*m)).collect()),
            )
            .unwrap();
            let parent = Parent::new(32 << 20, 0);
            let mut expected = (0usize, 0usize, 0usize);
            let domain = run(&parent, |work| {
                for (digest, (tag, body)) in additions.nodes {
                    let mut codec = CodecVec::default();
                    // 容量故意大于长度：合同额按 len+97，物理许可按真实容量。
                    codec.try_reserve_native(work, body.len() + 9)?;
                    for byte in &body {
                        codec.try_push_native(*byte, work)?;
                    }
                    assert!(codec.capacity() > codec.len());
                    let (len, capacity) = (codec.len(), codec.capacity());
                    if capture.append_original_codec_for_owner_testonly(
                        digest,
                        tag,
                        codec,
                        usize::MAX,
                        work,
                    )? {
                        expected.0 += 1;
                        expected.1 += len + 97;
                        expected.2 += capacity;
                    }
                }
                // 期望父账与来源域取自真实工作域，不由被测清单算出。
                Ok((work.ledger_id(), work.current_origin()))
            })
            .unwrap();
            (capture, parent, expected, domain)
        }
        let (capture, parent, expected, (ledger, origin)) =
            appended("base", ["alpha", "beta-gamma"]);
        assert!(expected.0 >= 3, "至少三份新正文：两串与数组根");
        let mut entries = Vec::new();
        capture
            .native_permit_entries_for_testonly(&mut entries)
            .unwrap();
        let mut bytes_only = Vec::new();
        capture
            .native_permit_inventory_for_testonly(&mut bytes_only)
            .unwrap();
        assert_eq!(
            bytes_only,
            entries.iter().map(|e| e.bytes()).collect::<Vec<_>>(),
            "字节清单由身份清单投影"
        );
        let ids: std::collections::BTreeSet<PermitId> = entries.iter().map(|e| e.id()).collect();
        assert_eq!(ids.len(), entries.len(), "每张许可恰一行");
        assert!(entries
            .iter()
            .all(|e| e.ledger() == ledger && e.origin() == origin));
        let bodies: Vec<_> = entries
            .iter()
            .filter(|e| e.role() == CapturePermitRole::NodeBody)
            .copied()
            .collect();
        assert_eq!(bodies.len(), expected.0);
        assert_eq!(bodies.iter().map(|e| e.bytes()).sum::<usize>(), expected.2);
        assert!(bodies.iter().all(|e| e.body().is_some()));
        // 本父账上所有Native许可都在清单中：合同项没有在父账另签同额物理许可。
        assert_eq!(
            entries.iter().map(|e| e.bytes()).sum::<usize>(),
            parent.native.load(Ordering::SeqCst),
            "父账Native live 恰等于清单之和"
        );
        let claims = capture.native_source_claim_entries_for_testonly().unwrap();
        let reconcile = |entries: &[CapturePermitEntry], claims: &[SourceClaimEntry]| {
            AuditCapture::reconcile_native_source_claims_for_testonly(
                entries,
                claims,
                ledger,
                &[origin],
            )
        };
        let ok = reconcile(&entries, &claims).unwrap();
        assert_eq!(
            ok,
            SourceClaimReconciliation {
                claims: expected.0,
                claim_amount: expected.1,
                backing_bytes: expected.2,
            }
        );
        assert_eq!(
            capture.native_source_claims_for_testonly().unwrap(),
            (expected.0, expected.1)
        );
        assert!(claims
            .iter()
            .all(|c| bodies.iter().any(|b| b.id() == c.backing())));
        // 顺序无关。
        let (mut rev_entries, mut rev_claims) = (entries.clone(), claims.clone());
        rev_entries.reverse();
        rev_claims.reverse();
        assert_eq!(reconcile(&rev_entries, &rev_claims).unwrap(), ok);
        let reject = |entries: &[CapturePermitEntry], claims: &[SourceClaimEntry]| {
            reconcile(entries, claims)
                .err()
                .expect("必须拒绝")
                .to_string()
        };
        // 删一项合同：已计费正文缺合同项。
        let error = reject(&entries, &claims[1..]);
        assert!(error.contains("缺来源合同项"), "{error}");
        // 指到另一节点的正文：两项指同一正文。
        let mut wrong = claims.clone();
        wrong[0] = wrong[0].with_backing_for_test(wrong[1].backing());
        let error = reject(&entries, &wrong);
        assert!(error.contains("两项来源合同指同一正文块"), "{error}");
        // 两项合法合同互换 backing：许可各用一次、总额不变，但合同节点与正文节点不符。
        let mut swapped = claims.clone();
        let (first, second) = (swapped[0].backing(), swapped[1].backing());
        swapped[0] = swapped[0].with_backing_for_test(second);
        swapped[1] = swapped[1].with_backing_for_test(first);
        let error = reject(&entries, &swapped);
        assert!(
            error.contains("来源合同项与所指正文块不是同一节点"),
            "{error}"
        );
        // 改一项合同额：按 backing 正文长度独立推出的合同额不符。
        let mut amended = claims.clone();
        amended[0] = amended[0].with_amount_for_test(amended[0].amount() + 1);
        let error = reject(&entries, &amended);
        assert!(error.contains("来源合同额与所指正文块不符"), "{error}");
        // 指到非正文许可，或指到清单外不存在的许可。
        let absent = 0u8;
        let mut targets: Vec<PermitId> = entries
            .iter()
            .filter(|e| e.role() != CapturePermitRole::NodeBody)
            .map(|e| e.id())
            .collect();
        targets.push(PermitId::of(&absent));
        for target in targets {
            let mut wrong = claims.clone();
            wrong[0] = wrong[0].with_backing_for_test(target);
            let error = reject(&entries, &wrong);
            assert!(error.contains("指不到已物理计费的正文块"), "{error}");
        }
        // 正文许可重复列出。
        let mut doubled = entries.clone();
        doubled.push(bodies[0]);
        let error = reject(&doubled, &claims);
        assert!(error.contains("同一许可重复列出"), "{error}");
        // 来源域不符：一张许可改记另一来源域；或调用方期望另一来源域。
        let mut stale = entries.clone();
        stale[0] = stale[0].with_origin_for_test(origin + 1);
        let error = reject(&stale, &claims);
        assert!(error.contains("许可不属期望来源域"), "{error}");
        let error = AuditCapture::reconcile_native_source_claims_for_testonly(
            &entries,
            &claims,
            ledger,
            &[origin + 1],
        )
        .err()
        .expect("必须拒绝")
        .to_string();
        assert!(error.contains("许可不属期望来源域"), "{error}");
        // 混入另一个 Parent 下 capture 的条目与合同；其自身在本父账下对账成功。
        let (foreign, foreign_parent, foreign_expected, (foreign_ledger, foreign_origin)) =
            appended("other", ["delta", "epsilon-zeta"]);
        assert_ne!(foreign_ledger, ledger);
        let mut foreign_entries = Vec::new();
        foreign
            .native_permit_entries_for_testonly(&mut foreign_entries)
            .unwrap();
        let foreign_claims = foreign.native_source_claim_entries_for_testonly().unwrap();
        assert_eq!(
            AuditCapture::reconcile_native_source_claims_for_testonly(
                &foreign_entries,
                &foreign_claims,
                foreign_ledger,
                &[foreign_origin],
            )
            .unwrap()
            .claims,
            foreign_expected.0
        );
        let foreign_body = foreign_entries
            .iter()
            .find(|e| e.role() == CapturePermitRole::NodeBody)
            .copied()
            .unwrap();
        let mut mixed = entries.clone();
        mixed.push(foreign_body);
        let error = reject(&mixed, &claims);
        assert!(error.contains("许可不属期望父账"), "{error}");
        let (mut mixed, mut mixed_claims) = (entries.clone(), claims.clone());
        mixed.extend(foreign_entries.iter().copied());
        mixed_claims.extend(foreign_claims.iter().copied());
        let error = reject(&mixed, &mixed_claims);
        assert!(error.contains("许可不属期望父账"), "{error}");
        // 合同改指另一父账的正文 backing（条目未混入）：指不到本清单的正文块。
        let mut wrong = claims.clone();
        wrong[0] = wrong[0].with_backing_for_test(foreign_body.id());
        let error = reject(&entries, &wrong);
        assert!(error.contains("指不到已物理计费的正文块"), "{error}");
        drop(foreign_entries);
        drop(foreign_claims);
        drop(foreign);
        assert_eq!(foreign_parent.native.load(Ordering::SeqCst), 0);
        drop((entries, claims, bodies, rev_entries, rev_claims));
        drop(capture);
        assert_eq!(parent.native.load(Ordering::SeqCst), 0, "析构全额归还");
    }

    #[test]
    fn b2_owner_codec_append_delta_skips_successful_sequence_subtrees() {
        let mut nodes = MemoryNodes::new();
        let values: Vec<Value> = (0..128)
            .map(|i| Value::string(format!("member-{i}")))
            .collect();
        let root = write_value(&mut nodes, &Value::Array(values.clone())).unwrap();
        let mut capture = native(nodes);
        verify(&capture, &root.content_ref(), Mode::Public).unwrap();
        let cold_reads = capture.reads.get();
        let before = capture.owner_codec.borrow().public.seen.len();
        capture = capture.finish_owner_step_for_testonly().unwrap();
        let mut additions = MemoryNodes::new();
        let mut next = values;
        next.push(Value::string("member-next"));
        let next_root = write_value(&mut additions, &Value::Array(next))
            .unwrap()
            .content_ref();
        append(&mut capture, additions);
        verify(&capture, &next_root, Mode::Public).unwrap();
        assert_eq!(
            capture.owner_codec.borrow().public.seen.len() - before,
            2,
            "只新增数组根和新字符串"
        );
        assert!(
            capture.reads.get() <= 12,
            "旧 128 元素序列整体证明应跳过，实际 {}",
            capture.reads.get()
        );
        assert!(cold_reads > capture.reads.get() * 10);
        assert_plain_proofs(&capture);
    }

    #[test]
    fn b2_owner_codec_failed_partial_closure_never_becomes_reusable() {
        let mut nodes = MemoryNodes::new();
        let malformed = frame(tag::BOOL_TRUE, b"invalid-body");
        nodes
            .nodes
            .insert(malformed, (tag::BOOL_TRUE, b"invalid-body".to_vec()));
        let bad = write_value(
            &mut nodes,
            &Value::Array(vec![Value::Edge(malformed), Value::Null]),
        )
        .unwrap()
        .content_ref();
        let good = write_value(&mut nodes, &Value::Null).unwrap().content_ref();
        let capture = native(nodes);
        let error = operation_error(verify(&capture, &bad, Mode::Public));
        assert_eq!(error.code(), code::BODY_SHAPE);
        assert_eq!(capture.owner_codec_retained_bytes_for_testonly(), 0);
        assert!(capture.owner_codec.borrow().revoked);
        assert_eq!(
            operation_error(verify(&capture, &good, Mode::Public)).code(),
            code::SOURCE
        );
        let capture = capture.into_originals_only_for_testonly().unwrap();
        assert_eq!(
            operation_error(verify(&capture, &good, Mode::Public)).code(),
            code::SOURCE,
            "冷清理不得把失败 owner 洗回有效"
        );
        // 原 full verify 保持独立判据；它的成功不能重新签发 owner 桥。
        capture.verify(&good, Mode::Public).unwrap();
        assert_eq!(
            operation_error(verify(&capture, &good, Mode::Public)).code(),
            code::SOURCE
        );
    }

    #[test]
    fn b2_owner_codec_same_digest_different_body_revokes_existing_success() {
        let mut nodes = MemoryNodes::new();
        let root = write_value(&mut nodes, &Value::Null).unwrap();
        let mut capture = native(nodes);
        verify(&capture, &root.content_ref(), Mode::Public).unwrap();
        let error = capture
            .append_original_for_owner_testonly(root.digest, tag::NULL, vec![0], usize::MAX, None)
            .unwrap_err();
        assert_eq!(error.code(), code::NODE_DIGEST);
        assert_eq!(capture.owner_codec_retained_bytes_for_testonly(), 0);
        assert_eq!(
            operation_error(verify(&capture, &root.content_ref(), Mode::Public)).code(),
            code::SOURCE
        );
    }

    #[test]
    fn b2_owner_codec_mode_and_json_role_do_not_share_success() {
        let mut nodes = MemoryNodes::new();
        let placeholder = write_template(&mut nodes, &Value::BatchSelf)
            .unwrap()
            .content_ref();
        let capture = native(nodes.clone());
        verify(&capture, &placeholder, Mode::Template).unwrap();
        let actual = operation_error(verify(&capture, &placeholder, Mode::Public));
        let expected = AuditCapture::new(nodes)
            .verify(&placeholder, Mode::Public)
            .unwrap_err();
        assert_eq!(actual, expected);

        let mut nodes = MemoryNodes::new();
        let inner = write_value(&mut nodes, &Value::JsonText(Box::new(JsonValue::Null))).unwrap();
        let outer_digest = frame(tag::JSON_TEXT, &inner.digest);
        nodes
            .nodes
            .insert(outer_digest, (tag::JSON_TEXT, inner.digest.to_vec()));
        let outer = ContentRef::new(Kind::JsonText, &outer_digest, None);
        let capture = native(nodes.clone());
        verify(&capture, &inner.content_ref(), Mode::Public).unwrap();
        let actual = operation_error(verify(&capture, &outer, Mode::Public));
        let expected = AuditCapture::new(nodes)
            .verify(&outer, Mode::Public)
            .unwrap_err();
        assert_eq!(actual, expected);
        assert_eq!(actual.code(), code::JSON_TEXT_INTERNAL_KIND);
    }

    #[test]
    fn b2_owner_codec_sequence_and_field_proofs_keep_json_role() {
        for field_sequence in [false, true] {
            let mut nodes = MemoryNodes::new();
            let inner =
                write_value(&mut nodes, &Value::JsonText(Box::new(JsonValue::Null))).unwrap();
            let root = write_value(
                &mut nodes,
                &if field_sequence {
                    Value::Object(vec![("key".into(), Value::Edge(inner.digest))])
                } else {
                    Value::Array(vec![Value::Edge(inner.digest)])
                },
            )
            .unwrap();
            let wrapped_digest = frame(tag::JSON_TEXT, &root.digest);
            nodes
                .nodes
                .insert(wrapped_digest, (tag::JSON_TEXT, root.digest.to_vec()));
            let wrapped = ContentRef::new(Kind::JsonText, &wrapped_digest, None);
            let capture = native(nodes.clone());
            verify(&capture, &root.content_ref(), Mode::Public).unwrap();
            let actual = operation_error(verify(&capture, &wrapped, Mode::Public));
            let expected = AuditCapture::new(nodes)
                .verify(&wrapped, Mode::Public)
                .unwrap_err();
            assert_eq!(actual, expected);
            assert_eq!(actual.code(), code::JSON_TEXT_INTERNAL_KIND);
        }
    }

    #[test]
    fn b2_owner_codec_full_verify_and_value_root_seed_do_not_mint_owner_proof() {
        let mut nodes = MemoryNodes::new();
        let root = write_value(&mut nodes, &Value::Array(vec![Value::string("x"); 32]))
            .unwrap()
            .content_ref();
        let old = native(nodes.clone());
        old.verify(&root, Mode::Public).unwrap();
        assert_eq!(old.owner_codec_retained_bytes_for_testonly(), 0);
        let prior = old.into_originals_and_codec_roots_for_testonly().unwrap();
        let fresh = native(nodes);
        assert_eq!(
            fresh
                .seed_codec_roots_from_native_capture_for_testonly(&prior)
                .unwrap(),
            1
        );
        assert_eq!(fresh.owner_codec_retained_bytes_for_testonly(), 0);
        verify(&fresh, &root, Mode::Public).unwrap();
        assert!(
            fresh.reads.get() > 1,
            "旧 Value 根证书不得进入本次 owner 子树资格"
        );
        assert!(fresh.owner_codec_retained_bytes_for_testonly() > 0);
    }

    #[test]
    fn b2_owner_codec_checks_headers_and_revision_even_on_warm_success() {
        for fault in 0..4 {
            let mut nodes = MemoryNodes::new();
            let mut root = write_value(&mut nodes, &Value::Array(vec![Value::Null]))
                .unwrap()
                .content_ref();
            let capture = native(nodes.clone());
            verify(&capture, &root, Mode::Public).unwrap();
            match fault {
                0 => root.kind = Kind::Object,
                1 => root.extent = Some(999),
                2 => root.format = "other-format".into(),
                _ => capture.owner_codec.borrow_mut().predicate_revision += 1,
            }
            let actual = operation_error(verify(&capture, &root, Mode::Public));
            if fault == 3 {
                assert_eq!(actual.code(), code::SOURCE);
            } else {
                assert_eq!(
                    actual,
                    AuditCapture::new(nodes)
                        .verify(&root, Mode::Public)
                        .unwrap_err()
                );
            }
            assert_eq!(capture.owner_codec_retained_bytes_for_testonly(), 0);
        }
    }

    #[test]
    fn b2_owner_codec_old_new_and_scratch_share_total_cap_before_growth() {
        let mut nodes = MemoryNodes::new();
        let root = write_value(&mut nodes, &Value::Array(vec![Value::Null; 16]))
            .unwrap()
            .content_ref();
        let capture = native(nodes);
        verify(&capture, &root, Mode::Public).unwrap();
        let retained = capture.owner_codec_retained_bytes_for_testonly();
        // 足够已存证明+scope头，但没有新 header read_digests 的一个真实 Layout。
        let parent = Parent::new(retained + CodecWork::metadata_bytes(), retained);
        let result = run(&parent, |work| {
            capture.verify_owner_refview_for_testonly((&root).into(), Mode::Public, work)
        });
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert_eq!(
            parent.peak.load(Ordering::SeqCst),
            retained + CodecWork::metadata_bytes()
        );
        assert_eq!(parent.layout.load(Ordering::SeqCst), 0);
        assert_eq!(capture.owner_codec_retained_bytes_for_testonly(), 0);
        assert!(capture.owner_codec.borrow().revoked);
    }

    #[test]
    fn b2_owner_codec_parent_invalidation_rejects_warm_hit_before_read() {
        let mut nodes = MemoryNodes::new();
        let root = write_value(&mut nodes, &Value::Null).unwrap().content_ref();
        let capture = native(nodes);
        verify(&capture, &root, Mode::Public).unwrap();
        let reads = capture.reads.get();
        let parent = Parent::new(32 << 20, capture.owner_codec_retained_bytes_for_testonly());
        let work = CodecWork::new(parent.clone(), parent.limit).unwrap();
        parent.failed.store(true, Ordering::SeqCst);
        let result = work.run(|scope| {
            capture.verify_owner_refview_for_testonly((&root).into(), Mode::Public, scope)
        });
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Ledger))
        ));
        assert_eq!(capture.reads.get(), reads);
        assert!(capture.owner_codec.borrow().revoked);
    }

    fn tree_snapshot(tree: &OwnerProofTree<u64, u64>) -> BTreeMap<u64, usize> {
        fn scan(
            link: &OwnerProofLink<u64, u64>,
            low: Option<u64>,
            high: Option<u64>,
            out: &mut BTreeMap<u64, usize>,
        ) -> usize {
            let Some(node) = link else {
                return 0;
            };
            assert!(low.is_none_or(|key| key < node.key));
            assert!(high.is_none_or(|key| node.key < key));
            assert_eq!(node.value, node.key);
            assert!(out
                .insert(
                    node.key,
                    (&**node as *const OwnerProofNode<u64, u64>) as usize
                )
                .is_none());
            let left = scan(&node.left, low, Some(node.key), out);
            let right = scan(&node.right, Some(node.key), high, out);
            assert!(left.abs_diff(right) <= 1);
            assert_eq!(node.height, left.max(right) + 1);
            node.height
        }
        let mut out = BTreeMap::new();
        let height = scan(&tree.root, None, None, &mut out);
        assert_eq!(out.len(), tree.len());
        assert_eq!(
            tree.retained_layout_bytes(),
            out.len() * std::alloc::Layout::new::<OwnerProofNode<u64, u64>>().size()
        );
        let bits = usize::BITS as usize - (tree.len() + 1).leading_zeros() as usize;
        assert!(height <= 2 * bits);
        out
    }

    #[test]
    fn b2_owner_codec_avl_fixed_delta_keeps_old_addresses_and_logarithmic_paths() {
        for order in 0..3 {
            let parent = Parent::new(32 << 20, 0);
            let mut tree = OwnerProofTree::<u64, u64>::default();
            run(&parent, |work| {
                for index in 0..512u64 {
                    let key = match order {
                        0 => index,
                        1 => 511 - index,
                        _ => {
                            if index % 2 == 0 {
                                index / 2
                            } else {
                                511 - index / 2
                            }
                        }
                    };
                    let old = tree_snapshot(&tree);
                    let height = OwnerProofNode::height(&tree.root);
                    let before_native = parent.native.load(Ordering::SeqCst);
                    OWNER_CODEC_INDEX_TRACE.with(|p| p.set((0, 0, 0)));
                    tree.insert(key, key, work)?;
                    let (compares, rotates, allocations) =
                        OWNER_CODEC_INDEX_TRACE.with(|p| p.get());
                    assert!(
                        compares <= height * 2,
                        "two index paths only: {compares} vs {height}"
                    );
                    assert!(rotates <= 2, "AVL insertion needs at most double rotation");
                    assert_eq!(allocations, 1);
                    assert_eq!(
                        parent.native.load(Ordering::SeqCst) - before_native,
                        std::alloc::Layout::new::<OwnerProofNode<u64, u64>>().size()
                    );
                    let current = tree_snapshot(&tree);
                    for (old_key, address) in old {
                        assert_eq!(current[&old_key], address, "old node must not migrate");
                    }
                    OWNER_CODEC_INDEX_TRACE.with(|p| p.set((0, 0, 0)));
                    assert_eq!(tree.get(&key), Some(&key));
                    let (reads, rotates, allocations) = OWNER_CODEC_INDEX_TRACE.with(|p| p.get());
                    assert!(reads <= OwnerProofNode::height(&tree.root));
                    assert_eq!((rotates, allocations), (0, 0));
                }
                Ok(())
            })
            .unwrap();
            assert_eq!(parent.layout.load(Ordering::SeqCst), 0);
            assert_eq!(
                parent.live.load(Ordering::SeqCst),
                tree.retained_layout_bytes()
            );
            // P1c：每个节点载荷持一张原父许可到真实析构。
            assert_eq!(Arc::strong_count(&parent), 1 + tree.len());
            drop(tree);
            assert_eq!(parent.live.load(Ordering::SeqCst), 0);
            assert_eq!(Arc::strong_count(&parent), 1);
        }
    }

    #[test]
    fn b2_owner_codec_avl_duplicate_and_one_byte_refusal_preserve_original_tree() {
        let mut tree = OwnerProofTree::<u64, u64>::default();
        let parent = Parent::new(32 << 20, 0);
        run(&parent, |work| {
            for key in 0..64 {
                tree.insert(key, key, work)?;
            }
            Ok(())
        })
        .unwrap();
        let old = tree_snapshot(&tree);
        let retained = tree.retained_layout_bytes();
        let tight = Parent::new(retained + CodecWork::metadata_bytes(), retained);
        OWNER_CODEC_INDEX_TRACE.with(|p| p.set((0, 0, 0)));
        run(&tight, |work| tree.insert(32, 999, work)).unwrap();
        assert_eq!(OWNER_CODEC_INDEX_TRACE.with(|p| p.get()).2, 0);
        assert_eq!(
            tree.get(&32),
            Some(&32),
            "duplicate must not replace proven value"
        );
        assert_eq!(tree_snapshot(&tree), old);
        assert_eq!(tight.live.load(Ordering::SeqCst), retained);
        let layout = std::alloc::Layout::new::<OwnerProofNode<u64, u64>>().size();
        let refused = Parent::new(
            retained + CodecWork::metadata_bytes() + layout - 1,
            retained,
        );
        OWNER_CODEC_INDEX_TRACE.with(|p| p.set((0, 0, 0)));
        let result = run(&refused, |work| tree.insert(64, 64, work));
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert_eq!(OWNER_CODEC_INDEX_TRACE.with(|p| p.get()).2, 0);
        assert_eq!(tree_snapshot(&tree), old);
        assert_eq!(refused.live.load(Ordering::SeqCst), retained);
        assert_eq!(tree.retained_layout_bytes(), retained);
    }

    #[test]
    fn b2_owner_codec_successful_verify_then_partial_promotion_failure_revokes_all() {
        let mut nodes = MemoryNodes::new();
        let root = write_value(
            &mut nodes,
            &Value::Array(vec![Value::Null, Value::Bool(true)]),
        )
        .unwrap()
        .content_ref();
        let capture = native(nodes);
        let parent = Parent::new(32 << 20, 0);
        let one_node = std::alloc::Layout::new::<OwnerProofNode<(NodeDigest, Flavor), ()>>().size();
        parent.native_limit.store(one_node, Ordering::SeqCst);
        OWNER_CODEC_INDEX_TRACE.with(|p| p.set((0, 0, 0)));
        let result = run(&parent, |work| {
            capture.verify_owner_refview_for_testonly((&root).into(), Mode::Public, work)
        });
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert_eq!(
            OWNER_CODEC_INDEX_TRACE.with(|p| p.get()).2,
            1,
            "first success certificate admitted before injected second-node refusal"
        );
        assert_eq!(capture.owner_codec_retained_bytes_for_testonly(), 0);
        assert!(capture.owner_codec.borrow().revoked);
        assert_eq!(parent.layout.load(Ordering::SeqCst), 0);
        // P1c：撤销即真实析构已成功节点，许可随之还原父账（不再 retain 留账到父生命周期结束）。
        assert!(parent.peak.load(Ordering::SeqCst) >= one_node);
        assert_eq!(
            parent.live.load(Ordering::SeqCst),
            0,
            "revoked Native proof releases its permit on real drop"
        );
        assert_eq!(Arc::strong_count(&parent), 1);
        assert_eq!(
            operation_error(verify(&capture, &root, Mode::Public)).code(),
            code::SOURCE
        );
    }

    #[test]
    fn b2_owner_codec_avl_order_equality_matches_all_context_components() {
        let utf8 = [
            Utf8State::Empty,
            Utf8State::Pending {
                len: 1,
                b: [0xc2, 0, 0],
            },
            Utf8State::Pending {
                len: 2,
                b: [0xe0, 0xa0, 0],
            },
            Utf8State::Pending {
                len: 3,
                b: [0xf0, 0x90, 0x80],
            },
        ];
        let numeric = [
            NumState::Start,
            NumState::Sign,
            NumState::Zero,
            NumState::Int,
            NumState::DotZero,
            NumState::DotInt,
            NumState::Frac,
            NumState::Exp,
            NumState::ExpSign,
            NumState::ExpDigits,
            NumState::Dead,
        ];
        let mut keys: Vec<(NodeDigest, Utf8State, NumState)> = Vec::new();
        for digest in [[0; 32], [1; 32], [255; 32]] {
            for u in utf8 {
                for n in numeric {
                    keys.push((digest, u, n));
                }
            }
        }
        for a in &keys {
            for b in &keys {
                assert_eq!(a.cmp(b) == std::cmp::Ordering::Equal, a == b);
            }
        }
        assert_ne!(
            ([0; 32], Flavor::Value).cmp(&([0; 32], Flavor::JsonInner)),
            std::cmp::Ordering::Equal
        );
        let parent = Parent::new(32 << 20, 0);
        let mut tree = OwnerProofTree::default();
        run(&parent, |work| {
            for (i, key) in keys.iter().copied().enumerate() {
                tree.insert(key, i, work)?;
            }
            Ok(())
        })
        .unwrap();
        assert_eq!(tree.len(), keys.len());
        for (i, key) in keys.iter().enumerate() {
            assert_eq!(tree.get(key), Some(&i));
        }
    }
    fn codec_body(parent: &Arc<Parent>, bytes: &[u8]) -> CodecVec<u8> {
        run(parent, |work| {
            let mut body = CodecVec::default();
            body.try_reserve_native(work, bytes.len())?;
            for byte in bytes {
                body.try_push_native(*byte, work)?;
            }
            Ok(body)
        })
        .unwrap()
    }

    #[test]
    fn codec_original_moves_pointer_capacity_and_releases_all_permits_on_drop() {
        let parent = Parent::new(32 << 20, 0);
        let mut capture = native(MemoryNodes::new());
        // Creation scope is closed; existing owner may move through a new live same-parent scope.
        let body = codec_body(&parent, b"held");
        let pointer = body.as_ptr();
        let capacity = body.capacity();
        let digest = frame(tag::BYTES_CHUNK, &body);
        assert_eq!(parent.live.load(Ordering::SeqCst), capacity);
        run(&parent, |work| {
            assert!(capture.append_original_codec_for_owner_testonly(
                digest,
                tag::BYTES_CHUNK,
                body,
                usize::MAX,
                work,
            )?);
            Ok(())
        })
        .unwrap();
        assert_eq!(capture.original(&digest).unwrap().1.as_ptr(), pointer);
        let CaptureNodes::Native(nodes) = &capture.nodes else {
            unreachable!()
        };
        assert_eq!(nodes.nodes[&digest].body.capacity(), capacity);
        let retained_before_drop = parent.live.load(Ordering::SeqCst);
        assert!(
            retained_before_drop > capacity,
            "old semantic source/index retain remains separate"
        );
        drop(capture);
        // P1c：正文与索引/来源许可都由实际载荷持有，capture 真实析构全部还原。
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn codec_original_duplicate_releases_only_new_owner_and_preserves_old_bytes() {
        let parent = Parent::new(32 << 20, 0);
        let mut capture = native(MemoryNodes::new());
        let body = codec_body(&parent, b"same");
        let pointer = body.as_ptr();
        let digest = frame(tag::BYTES_CHUNK, &body);
        run(&parent, |work| {
            capture.append_original_codec_for_owner_testonly(
                digest,
                tag::BYTES_CHUNK,
                body,
                usize::MAX,
                work,
            )
        })
        .unwrap();
        let baseline = parent.live.load(Ordering::SeqCst);
        let duplicate = codec_body(&parent, b"same");
        run(&parent, |work| {
            capture.append_original_codec_for_owner_testonly(
                digest,
                tag::BYTES_CHUNK,
                duplicate,
                usize::MAX,
                work,
            )
        })
        .unwrap();
        assert_eq!(parent.live.load(Ordering::SeqCst), baseline);
        assert_eq!(capture.original(&digest).unwrap().1.as_ptr(), pointer);
        assert!(!capture.owner_codec.borrow().revoked);
    }

    #[test]
    fn codec_original_digest_error_releases_body_and_revokes_without_insert() {
        let parent = Parent::new(32 << 20, 0);
        let mut capture = native(MemoryNodes::new());
        let body = codec_body(&parent, b"bad digest");
        let result = run(&parent, |work| {
            capture
                .append_original_codec_for_owner_testonly(
                    [0; 32],
                    tag::BYTES_CHUNK,
                    body,
                    usize::MAX,
                    work,
                )
                .map(|_| ())
        });
        assert_eq!(operation_error(result).code(), code::NODE_DIGEST);
        assert!(capture.owner_codec.borrow().revoked);
        assert!(capture.original(&[0; 32]).is_none());
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn codec_original_rejects_unbound_plain_owned_and_non_native_capture() {
        let parent = Parent::new(32 << 20, 0);
        let owned_budget = capture_allocation::OwnedBudget::new(1 << 20);
        let mut owned = CodecVec::new_with_allocation(Some(&owned_budget));
        owned.try_push(1u8).unwrap();
        for body in [CodecVec::from_plain(vec![1u8]), CodecVec::default(), owned] {
            let mut capture = native(MemoryNodes::new());
            let digest = frame(tag::BYTES_CHUNK, &body);
            let error = operation_error(run(&parent, |work| {
                capture
                    .append_original_codec_for_owner_testonly(
                        digest,
                        tag::BYTES_CHUNK,
                        body,
                        usize::MAX,
                        work,
                    )
                    .map(|_| ())
            }));
            assert_eq!(error.code(), code::SOURCE);
            assert!(capture.owner_codec.borrow().revoked);
        }
        assert_eq!(owned_budget.snapshot().live_requested_layout_bytes, 0);
        let body = codec_body(&parent, b"legacy");
        let digest = frame(tag::BYTES_CHUNK, &body);
        let mut capture = AuditCapture::new(MemoryNodes::new());
        assert_eq!(
            operation_error(run(&parent, |work| capture
                .append_original_codec_for_owner_testonly(
                    digest,
                    tag::BYTES_CHUNK,
                    body,
                    usize::MAX,
                    work,
                )
                .map(|_| ())))
            .code(),
            code::SOURCE
        );
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn codec_original_empty_binding_requires_same_parent_and_live_healthy_operation() {
        let original = Parent::new(32 << 20, 0);
        let foreign = Parent::new(32 << 20, 0);
        let mut capture = native(MemoryNodes::new());
        let body = codec_body(&original, b"");
        assert!(body.is_native_bound());
        assert!(matches!(
            run(&foreign, |work| capture
                .append_original_codec_for_owner_testonly(
                    frame(tag::BYTES_CHUNK, b""),
                    tag::BYTES_CHUNK,
                    body,
                    usize::MAX,
                    work,
                )),
            Err(WorkError::Resource(ScratchFailure::Ledger))
        ));
        assert!(capture.owner_codec.borrow().revoked);
        assert_eq!(original.live.load(Ordering::SeqCst), 0);
        let body = codec_body(&original, b"");
        let mut capture = native(MemoryNodes::new());
        assert!(matches!(
            run(&original, |work| {
                let _ = work.reject(ScratchFailure::Budget);
                capture.append_original_codec_for_owner_testonly(
                    frame(tag::BYTES_CHUNK, b""),
                    tag::BYTES_CHUNK,
                    body,
                    usize::MAX,
                    work,
                )
            }),
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert!(capture.owner_codec.borrow().revoked);
        let body = codec_body(&original, b"");
        let mut capture = native(MemoryNodes::new());
        run(&original, |work| {
            capture.append_original_codec_for_owner_testonly(
                frame(tag::BYTES_CHUNK, b""),
                tag::BYTES_CHUNK,
                body,
                usize::MAX,
                work,
            )
        })
        .unwrap();
        assert_eq!(
            capture
                .original(&frame(tag::BYTES_CHUNK, b""))
                .unwrap()
                .1
                .len(),
            0
        );
    }

    #[test]
    fn codec_original_sparse_node_move_drops_only_discarded_body_permits() {
        let parent = Parent::new(32 << 20, 0);
        let mut capture = native(MemoryNodes::new());
        let keep = codec_body(&parent, b"keep");
        let _keep_capacity = keep.capacity();
        let keep_pointer = keep.as_ptr();
        let keep_digest = frame(tag::BYTES_CHUNK, &keep);
        let discard = codec_body(&parent, b"discarded");
        let discard_capacity = discard.capacity();
        let discard_digest = frame(tag::BYTES_CHUNK, &discard);
        run(&parent, |work| {
            capture.append_original_codec_for_owner_testonly(
                keep_digest,
                tag::BYTES_CHUNK,
                keep,
                usize::MAX,
                work,
            )?;
            capture.append_original_codec_for_owner_testonly(
                discard_digest,
                tag::BYTES_CHUNK,
                discard,
                usize::MAX,
                work,
            )?;
            Ok(())
        })
        .unwrap();
        let before = parent.live.load(Ordering::SeqCst);
        // Exercise the sparse collector's actual NativeNode drain/move ownership operation.
        let CaptureNodes::Native(nodes) = &mut capture.nodes else {
            unreachable!()
        };
        let mut kept = std::collections::HashMap::new();
        for (digest, node) in nodes.nodes.drain() {
            if digest == keep_digest {
                kept.insert(digest, node);
            }
        }
        nodes.nodes = kept;
        assert_eq!(
            parent.live.load(Ordering::SeqCst),
            before - discard_capacity
        );
        assert_eq!(
            capture.original(&keep_digest).unwrap().1.as_ptr(),
            keep_pointer
        );
        assert!(capture.original(&discard_digest).is_none());
        drop(capture);
        // P1c：保留正文与索引许可在 capture 真实析构时一并还原。
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn codec_original_old_vec_entry_still_moves_unbound_buffer_without_native_gate() {
        let mut capture = native(MemoryNodes::new());
        let body = b"old E path".to_vec();
        let pointer = body.as_ptr();
        let digest = frame(tag::BYTES_CHUNK, &body);
        assert!(capture
            .append_original_for_owner_testonly(digest, tag::BYTES_CHUNK, body, usize::MAX, None)
            .unwrap());
        assert_eq!(capture.original(&digest).unwrap().1.as_ptr(), pointer);
    }
    #[test]
    fn codec_original_same_digest_corrupt_old_payload_fails_and_keeps_old_owner() {
        let parent = Parent::new(32 << 20, 0);
        let mut capture = native(MemoryNodes::new());
        let original = codec_body(&parent, b"original");
        let digest = frame(tag::BYTES_CHUNK, &original);
        run(&parent, |work| {
            capture.append_original_codec_for_owner_testonly(
                digest,
                tag::BYTES_CHUNK,
                original,
                usize::MAX,
                work,
            )
        })
        .unwrap();
        let CaptureNodes::Native(nodes) = &mut capture.nodes else {
            unreachable!()
        };
        nodes.nodes.get_mut(&digest).unwrap().tag = tag::STRING;
        let baseline = parent.live.load(Ordering::SeqCst);
        let incoming = codec_body(&parent, b"original");
        let error = operation_error(run(&parent, |work| {
            capture
                .append_original_codec_for_owner_testonly(
                    digest,
                    tag::BYTES_CHUNK,
                    incoming,
                    usize::MAX,
                    work,
                )
                .map(|_| ())
        }));
        assert_eq!(error.code(), code::NODE_DIGEST);
        assert!(capture.owner_codec.borrow().revoked);
        assert_eq!(parent.live.load(Ordering::SeqCst), baseline);
        assert_eq!(capture.original(&digest).unwrap().0, tag::STRING);
    }

    #[test]
    fn codec_original_index_cap_failure_refunds_incoming_body_and_provisional_charges() {
        let parent = Parent::new(32 << 20, 0);
        let mut capture = native(MemoryNodes::new());
        let body = codec_body(&parent, b"cap failure");
        let digest = frame(tag::BYTES_CHUNK, &body);
        let result = run(&parent, |work| {
            capture.append_original_codec_for_owner_testonly(
                digest,
                tag::BYTES_CHUNK,
                body,
                0,
                work,
            )
        });
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Allocation))
        ));
        assert!(capture.owner_codec.borrow().revoked);
        assert!(!capture.contains(&digest));
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn codec_original_allocation_prototype_capture_remains_rejected() {
        let parent = Parent::new(32 << 20, 0);
        let budget = capture_allocation::OwnedBudget::new(4096);
        let slab = capture_allocation::CaptureSlab::new(0, 0, &budget).unwrap();
        let mut capture = AuditCapture::from_slab_for_allocation_prototype(slab);
        let body = codec_body(&parent, b"owned capture");
        let digest = frame(tag::BYTES_CHUNK, &body);
        assert_eq!(
            operation_error(run(&parent, |work| capture
                .append_original_codec_for_owner_testonly(
                    digest,
                    tag::BYTES_CHUNK,
                    body,
                    usize::MAX,
                    work
                )
                .map(|_| ())))
            .code(),
            code::SOURCE
        );
        assert!(matches!(
            &capture.nodes,
            CaptureNodes::AllocationPrototype(_)
        ));
        assert_eq!(parent.live.load(Ordering::SeqCst), 0);
    }
}

#[cfg(feature = "s_capture_native_testonly")]
impl CodecError {
    /// #1488 0072 测试面：不经任何作用域直接构造 scratch 错误，与 SourceClaim 字节不符、
    /// check_write 域关闭等 Ledger 构造点同一入口（经类型化失败记录）。
    #[doc(hidden)]
    pub fn scratch_unscoped_for_test(failure: capture_allocation::scratch::ScratchFailure) -> Self {
        Self::scratch(failure)
    }
}

impl CodecError {
    /// #1488 0072 Display 挂点：生产构建为 `&self.detail` 恒等，testonly 门内登记过桥渲染。
    #[inline(always)]
    fn shown(&self) -> &std::borrow::Cow<'static, str> {
        capture_allocation::scratch::shown(self)
    }
}
