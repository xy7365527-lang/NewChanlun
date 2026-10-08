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

/// 真实 Unix I/O（调用者提供的 `UnixStream`）接到 job 计费与绝对期限的薄适配（仅 unix）。
/// 只覆盖该连接上的读写字节；connect／spawn／HTTP／stderr 由外层各自带期限与计费。
#[cfg(unix)]
pub mod job_wire;

/// 规范序列（M 树）的持久操作：concat / split / append（#1394 采纳 C §5.2）。
pub mod sequence;
pub use sequence::{
    sequence_append, sequence_append_with_allocation, sequence_concat,
    sequence_concat_with_allocation, sequence_split, sequence_split_with_allocation, NodeStore,
    SequenceRef,
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
        error.scratch_failure = Some(failure);
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
        write!(f, "{}: {}", self.code, self.detail)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
}
impl VerifierBody<'_> {
    fn empty() -> Self {
        Self::Plain(Vec::new())
    }
    fn clear(&mut self) {
        match self {
            Self::Plain(v) => v.clear(),
            Self::Borrowed(_) | Self::Metered(_) => *self = Self::empty(),
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
    fn next(
        &mut self,
        source: &mut dyn NodeSource,
    ) -> Result<Option<(u8, NodeDigest, Vec<u8>)>, CodecError> {
        self.next_with(|d| read_checked(source, d))
    }
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
// #1394 Native TestOnly：原件与纯 F 证明共用完整摘要索引；正文仍逐次复制后重绑定。
#[cfg(feature = "s_capture_native_testonly")]
struct NativeNode {
    tag: u8,
    body: Vec<u8>,
    // 成功后才分配证明；未读节点不常驻完整 CheckedFrame。
    proof: std::cell::RefCell<Option<allocator_api2::boxed::Box<NativeFrameProof>>>,
    // owner热写清理链复用节点inline，不另分配全nodes列表。
    owner_dirty: std::cell::Cell<bool>,
    owner_next: std::cell::Cell<Option<NodeDigest>>,
}
#[cfg(feature = "s_capture_native_testonly")]
impl NativeNode {
    fn new(tag: u8, body: Vec<u8>) -> Self {
        Self {
            tag,
            body,
            proof: Default::default(),
            owner_dirty: Default::default(),
            owner_next: Default::default(),
        }
    }
}
#[cfg(feature = "s_capture_native_testonly")]
struct NativeCaptureNodes {
    // 标准 RandomState；未核摘要不使用弱哈希。
    nodes: std::collections::HashMap<NodeDigest, NativeNode>,
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
    fn remove(&mut self, proof: &mut Option<allocator_api2::boxed::Box<NativeFrameProof>>) {
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
        slot: &mut Option<allocator_api2::boxed::Box<NativeFrameProof>>,
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
        slot: &mut Option<allocator_api2::boxed::Box<NativeFrameProof>>,
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
            *slot = Some(proof);
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
        }
    }
}
/// #1394 私有构造：同次全新核验器完成导航与三个完整Public闭包后才生成。
/// 此证书只描述Counts依据；仅整组接纳的三个Value根可封存codec资格，不授予来源业务资格。
#[cfg(feature = "s_capture_native_testonly")]
pub struct NativeCountsSubsetCertificate {
    state: ContentRef,
    codec_revision: u32,
    dependency_revision: u32,
    roots: [ContentRef; 3],
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
            && state == &self.state
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
#[cfg(feature = "s_capture_native_testonly")]
#[derive(Default)]
struct NativeCountsSubsetLedger {
    digests: std::collections::HashSet<NodeDigest>,
    order: Vec<NodeDigest>,
    source_bytes: usize,
}
#[cfg(feature = "s_capture_native_testonly")]
impl NativeCountsSubsetLedger {
    fn record(&mut self, digest: &NodeDigest, capture: &AuditCapture) -> Result<(), CodecError> {
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
        self.digests
            .try_reserve(1)
            .map_err(|e| CodecError::new(code::SOURCE, e.to_string()))?;
        self.order
            .try_reserve(1)
            .map_err(|e| CodecError::new(code::SOURCE, e.to_string()))?;
        self.digests.insert(*digest);
        self.order.push(*digest);
        self.source_bytes = bytes;
        Ok(())
    }
    fn rollback(&mut self, checkpoint: (usize, usize)) -> usize {
        let removed = self.order.len() - checkpoint.0;
        for digest in self.order.drain(checkpoint.0..) {
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
}
#[cfg(feature = "s_capture_native_testonly")]
impl NodeSource for NativeCountsSubsetReader<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        let got = CaptureReader(self.capture).read(digest)?;
        if matches!(&got, NodeRead::Found { .. }) {
            self.ledger.borrow_mut().record(digest, self.capture)?;
        }
        Ok(got)
    }
    fn checked_read(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<job::CheckedFrame>, CodecError> {
        let got = CaptureReader(self.capture).checked_read(digest)?;
        if got.is_some() {
            self.ledger.borrow_mut().record(digest, self.capture)?;
        }
        Ok(got)
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
        mut self,
        groups: impl IntoIterator<Item = (ContentRef, usize)>,
        fixed_source_bytes: usize,
        cap: usize,
    ) -> Result<
        (
            Self,
            Vec<NativeCountsSubsetCertificate>,
            NativeCountsSubsetReport,
        ),
        CodecError,
    > {
        if !matches!(&self.nodes, CaptureNodes::Native(_)) {
            return Err(CodecError::new(
                code::SOURCE,
                "Counts稀疏原件只接受Native捕获",
            ));
        }
        self = self.into_originals_only_for_testonly()?;
        let ledger = std::cell::RefCell::new(NativeCountsSubsetLedger::default());
        let mut certificates = Vec::new();
        let mut report = NativeCountsSubsetReport::default();
        let mut accepted_metadata_bytes = 0usize;
        let mut admitted_public_roots = std::collections::HashSet::new();
        {
            let mut reader = NativeCountsSubsetReader {
                capture: &self,
                ledger: &ledger,
            };
            let mut verifier = Verifier::new(&mut reader, Mode::Public);
            for (state, group_source_bytes) in groups {
                report.attempted_groups += 1;
                let checkpoint = {
                    let material = ledger.borrow();
                    (material.order.len(), material.source_bytes)
                };
                let names = ["objects", "witnesses", "observations"];
                let fields: CodecFields = path::object_fields_output(
                    &mut verifier.source,
                    RefView::from(&state),
                    &names,
                    false,
                )?;
                let roots = [
                    fields
                        .get("objects")
                        .ok_or_else(|| CodecError::new(code::SOURCE, "Counts缺objects"))?
                        .clone(),
                    fields
                        .get("witnesses")
                        .ok_or_else(|| CodecError::new(code::SOURCE, "Counts缺witnesses"))?
                        .clone(),
                    fields
                        .get("observations")
                        .ok_or_else(|| CodecError::new(code::SOURCE, "Counts缺observations"))?
                        .clone(),
                ];
                for root in &roots {
                    verifier.verify_refview(RefView::from(root))?;
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
                certificates
                    .try_reserve(1)
                    .map_err(|e| CodecError::new(code::SOURCE, e.to_string()))?;
                // 分配失败交由post-COMMIT调用方丢弃可选缓存，不改变成功业务回复。
                admitted_public_roots
                    .try_reserve(new_public_root_count)
                    .map_err(|e| CodecError::new(code::SOURCE, e.to_string()))?;
                certificates.push(certificate);
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
        kept.try_reserve(material.digests.len())
            .map_err(|e| CodecError::new(code::SOURCE, e.to_string()))?;
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
            // 不保留的NativeNode在本轮drop；body Vec不复制、不留在旧HashMap或proof中。
        }
        native.nodes = kept;
        // collector产生的F proof及全请求memo都不得跟随稀疏owner保留。
        self = self.into_originals_only_for_testonly()?;
        // generated_state仅导航，不授予完整根资格；Template保持Default空集合。
        self.codec_roots.get_mut().public = admitted_public_roots;
        Ok((self, certificates, report))
    }
}

/// Native owner 的无分配计费读数；不构造诊断 JSON 或赋予审计资格。
#[cfg(feature = "s_capture_native_testonly")]
#[doc(hidden)]
pub struct NativeCaptureRetention {
    pub body_len: usize,
    pub body_capacity: usize,
    pub index_capacity: usize,
    pub index_payload: usize,
    pub public_roots: usize,
    pub template_roots: usize,
    pub public_capacity: usize,
    pub template_capacity: usize,
}

pub struct AuditCapture {
    nodes: CaptureNodes,
    public: std::cell::RefCell<AuditMemo>,
    template: std::cell::RefCell<AuditMemo>,
    reads: std::cell::Cell<u64>,
    #[cfg(feature = "s_capture_native_testonly")]
    codec_roots: std::cell::RefCell<NativeCodecRootCertificates>,
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
                current.nodes.get(digest).is_some_and(|now| {
                    now.tag == node.tag && now.body.as_slice() == node.body.as_slice()
                })
            })
        {
            return Ok(0);
        }
        let mut public = AuditMemo::default();
        let mut template = AuditMemo::default();
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
            if old.tag != tag || old.body != body {
                return Err(CodecError::new(code::NODE_DIGEST, "同摘要异原件"));
            }
            return Ok(true);
        }
        if native.nodes.len() == native.nodes.capacity() {
            let capacity = native.nodes.capacity().saturating_mul(2).max(4);
            let growth = capacity
                .saturating_mul(2)
                .saturating_add(16)
                .saturating_mul(std::mem::size_of::<(NodeDigest, NativeNode)>() + 1);
            if growth > cap || native.nodes.try_reserve(1).is_err() {
                return Ok(false);
            }
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
        native.nodes.insert(digest, NativeNode::new(tag, body));
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
                .is_some_and(|node| node.tag == tag && node.body.as_slice() == body),
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
        }
    }
    /// #1394：显式 Native TestOnly opt-in；资格只来自本捕获的节点数和原 body 总长度。
    /// 不适用于 Owned/AllocationPrototype，不声明整 heap 或 S 资源准入。
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn enable_native_frame_reuse_for_testonly(mut self) -> Result<Self, CodecError> {
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
        native_nodes
            .try_reserve(entry_limit)
            .map_err(|_| CodecError::new(code::SOURCE, "Native捕获索引分配失败"))?;
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
            native_nodes.insert(digest, NativeNode::new(tag, body));
        }
        self.nodes = CaptureNodes::Native(NativeCaptureNodes {
            nodes: native_nodes,
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
            CaptureNodes::Native(nodes) => nodes.get(digest).map(|n| (n.tag, n.body.as_slice())),
            CaptureNodes::AllocationPrototype(slab) => slab.original(digest),
        }
    }
    pub fn reader(&self) -> impl NodeSource + '_ {
        CaptureReader(self)
    }
    pub fn verify(&self, reference: &ContentRef, mode: Mode) -> Result<(), CodecError> {
        self.verify_refview(reference.into(), mode)
    }
    #[doc(hidden)]
    pub fn verify_refview(&self, reference: RefView<'_>, mode: Mode) -> Result<(), CodecError> {
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
        let mut source = CaptureReader(self);
        let mut verifier = Verifier::with_memo_capture(&mut source, mode, memo);
        let result = verifier.verify_refview(reference);
        // 出错不保留 run() 提前插入的 seen/pushed，不能让下一根继承半成功。
        if let Ok((digest, _, _)) = &result {
            *slot.borrow_mut() = verifier.into_memo();
            #[cfg(feature = "s_capture_native_testonly")]
            if matches!(&self.nodes, CaptureNodes::Native(_)) {
                let mut roots = self.codec_roots.borrow_mut();
                let set = if mode == Mode::Public {
                    &mut roots.public
                } else {
                    &mut roots.template
                };
                // 证书保留失败不改变本次已经成功的业务校验；只是下次少一个复用根。
                if set.contains(digest) || set.try_reserve(1).is_ok() {
                    set.insert(*digest);
                }
            }
            #[cfg(not(feature = "s_capture_native_testonly"))]
            let _ = digest;
        }
        result.map(|_| ())
    }
    #[cfg(feature = "s_capture_native_testonly")]
    #[doc(hidden)]
    pub fn native_retention_numbers_for_testonly(&self) -> Option<NativeCaptureRetention> {
        let CaptureNodes::Native(native) = &self.nodes else {
            return None;
        };
        let roots = self.codec_roots.borrow();
        Some(NativeCaptureRetention {
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
                    "proof_slot_bytes_per_entry":std::mem::size_of::<std::cell::RefCell<Option<allocator_api2::boxed::Box<NativeFrameProof>>>>(),
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
        #[cfg(feature = "s_capture_native_testonly")]
        if let CaptureNodes::Native(native) = &self.0.nodes {
            return self.checked_native_current(digest, native, |node| {
                Ok(match node {
                    Some(node) => NodeRead::Found {
                        tag: node.tag,
                        body: node.body.to_vec(),
                    },
                    None => NodeRead::Missing,
                })
            });
        }
        match self.read(digest)? {
            NodeRead::Missing => Ok(None),
            NodeRead::Found { tag, body } => {
                job::CheckedFrame::checked(digest, tag, body).map(Some)
            }
        }
    }
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
mod native_frame_reuse_tests {
    use super::*;

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
            node.body = body;
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
            .append_original_for_owner_testonly(digest, tag::BYTES_CHUNK, body, usize::MAX)
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
}

impl<'a, 'capture> Verifier<'a, 'capture> {
    // #1394 Root 内部实施裁定：具体capture与私有context闭合，不接受dyn成功token。
    fn with_memo_capture(
        source: &'a mut CaptureReader<'capture>,
        mode: Mode,
        memo: AuditMemo,
    ) -> Self {
        Self::with_context(CodecContext::Capture(source.0), mode, memo)
    }
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
    let digest = parse_root(root)?;
    let (kind, extent) = path::reference_header(source, &digest)?;
    Ok(ContentRef {
        format: CONTENT_FORMAT.to_string(),
        root: root.to_string(),
        kind,
        extent,
    })
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
    mut source: CodecContext<'_, '_>,
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
            "22e6b1895c6ec3a95c5c5c225c753030303122",
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
            "225b312e302c5c22e6b1895c5c6e5c225d22",
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
    #[test]
    fn borrowed_key_two_chunks_fit_without_body_layouts_or_copies() {
        let mut nodes = MemoryNodes::new();
        let a = write_value(&mut nodes, &Value::string("a".repeat(4096))).unwrap();
        let b = write_value(&mut nodes, &Value::string("b".repeat(4096))).unwrap();
        let (capture, budget, base) = slab_capture(nodes, 204);
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
            base + 140,
            "只有实际read集合持账"
        );
        assert_eq!(
            after.peak_requested_layout_bytes,
            base + 204,
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
        let (capture, budget, base) = slab_capture(nodes, 8192 + 32 + 204);
        let before = budget.snapshot();
        let before_f = frame_calls_on_thread();
        assert_eq!(
            captured_compare(&capture, &a.digest, &b.digest).unwrap(),
            expected
        );
        assert_eq!(frame_calls_on_thread() - before_f, legacy_f);
        let after = budget.snapshot();
        assert_eq!(after.peak_requested_layout_bytes, base + 204);
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
            assert_eq!(
                after.live_requested_layout_bytes,
                base + 868,
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
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            ),
            (
                55,
                "463eb28e72f82e0a96c0a4cc53690c571281131f672aa229e0d45ae59b598b59",
            ),
            (
                56,
                "da2ae4d6b36748f2a318f23e7ab1dfdf45acdc9d049bd80e59de82a60895f562",
            ),
            (
                63,
                "29af2686fd53374a36b0846694cc342177e428d1647515f078784d69cdb9e488",
            ),
            (
                64,
                "fdeab9acf3710362bd2658cdc9a29e8f9c757fcf9811603a8c447cd1d9151108",
            ),
            (
                65,
                "4bfd2c8b6f1eec7a2afeb48b934ee4b2694182027e6d0fc075074f2fabb31781",
            ),
            (
                127,
                "92ca0fa6651ee2f97b884b7246a562fa71250fedefe5ebf270d31c546bfea976",
            ),
            (
                128,
                "471fb943aa23c511f6f72f8d1652d9c880cfa392ad80503120547703e56a2be5",
            ),
            (
                129,
                "5099c6a56203f9687f7d33f4bfdf576d31dc91f6b695ecea38b2770c87631135",
            ),
            (
                4096,
                "d67c656e01756650d77717b0839985a056ec28ffe174601d690fc407a2ceffca",
            ),
        ];
        assert_eq!(
            format!("{:x}", Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
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

        fn reserve(&self, n: usize, _: ScratchCharge) -> Result<(), ScratchFailure> {
            self.0.fetch_add(n, Ordering::SeqCst);
            Ok(())
        }
        fn release(&self, n: usize, _: ScratchCharge) -> Result<(), ScratchFailure> {
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
