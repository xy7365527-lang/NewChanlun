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
use std::collections::{BTreeMap, HashMap, HashSet};
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
/// 整次操作的累计资源与期限（#1394 采纳 C §5.6.1）。
pub mod job;

/// 真实 Unix I/O（调用者提供的 `UnixStream`）接到 job 计费与绝对期限的薄适配（仅 unix）。
/// 只覆盖该连接上的读写字节；connect／spawn／HTTP／stderr 由外层各自带期限与计费。
#[cfg(unix)]
pub mod job_wire;

/// 规范序列（M 树）的持久操作：concat / split / append（#1394 采纳 C §5.2）。
pub mod sequence;
pub use sequence::{sequence_append, sequence_concat, sequence_split, NodeStore, SequenceRef};

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
    detail: String,
}

impl CodecError {
    pub fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
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

/// F(tag, body) = SHA256(D || one_byte(tag) || U64(len(body)) || body)。
pub fn frame(t: u8, body: &[u8]) -> NodeDigest {
    FRAME_CALLS.with(|calls| calls.set(calls.get().wrapping_add(1)));
    let mut h = Sha256::new();
    h.update(DOMAIN_SEPARATOR);
    h.update([t]);
    h.update((body.len() as u64).to_be_bytes());
    h.update(body);
    h.finalize().into()
}

/// 32 字节摘要的 transport 文本：`sc1-` + 64 位小写十六进制。
pub fn root_string(digest: &NodeDigest) -> String {
    let mut s = String::with_capacity(ROOT_PREFIX.len() + 64);
    s.push_str(ROOT_PREFIX);
    for b in digest {
        s.push_str(&format!("{b:02x}"));
    }
    s
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
            CodecError::new(
                code::ROOT,
                format!("根文本 `{root}` 含非小写十六进制字符"),
            )
        })?;
        let lo = hex_val(bytes[2 * i + 1]).ok_or_else(|| {
            CodecError::new(
                code::ROOT,
                format!("根文本 `{root}` 含非小写十六进制字符"),
            )
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
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError>;

    // Reached a checked array and an in-range index, immediately before descent.
    // Default leaves existing callers and storage sources unchanged.
    fn before_path_index(&mut self, _prefix: &[path::PathStep], _index: u64) -> Result<(), CodecError> {
        Ok(())
    }

    /// 同次读取的已核帧。旧 NodeSource 无须覆写：默认先读取原 Vec，再核一次 F。
    /// 包装器可沿同一调用栈传递证明；作用域内也可先实际重读全部原件，在请求摘要、tag
    /// 与完整 body 逐字相同后仅复用纯 F 证明。不得按摘要跳过真实读取，不得以旧成功
    /// 兜底错误、Missing 或变化的原件；来源、角色、闭包与业务准入仍须逐次核验。
    fn checked_read(&mut self, digest: &NodeDigest) -> Result<Option<job::CheckedFrame>, CodecError> {
        match self.read(digest)? {
            NodeRead::Missing => Ok(None),
            NodeRead::Found { tag, body } => job::CheckedFrame::checked(digest, tag, body).map(Some),
        }
    }

    /// 审计源可显式限制单次渲染物化；普通流式源不擅自填默认上限。
    fn max_render_bytes(&self) -> Option<u64> { None }
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
        if self.format != CONTENT_FORMAT {
            return Err(CodecError::new(
                code::FORMAT,
                format!(
                    "ContentRef.format 必须是 `{CONTENT_FORMAT}`，实为 `{}`",
                    self.format
                ),
            ));
        }
        parse_root(&self.root)
    }

    /// 严格 transport 形态：四字段齐备、无多余字段、类型精确。
    /// extent 是 U64 长度，协议上用规范十进制字符串传输（非长度型为 null）。
    pub fn to_transport_json(&self) -> JsonNode {
        let mut map = JsonMap::new();
        map.insert("format".into(), JsonNode::String(self.format.clone()));
        map.insert("root".into(), JsonNode::String(self.root.clone()));
        map.insert("kind".into(), JsonNode::String(self.kind.name().to_string()));
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
        let map = node.as_object().ok_or_else(|| {
            CodecError::new(code::REF_FIELD, "ContentRef 必须是 JSON 对象".to_string())
        })?;
        for key in map.keys() {
            if !matches!(key.as_str(), "format" | "root" | "kind" | "extent") {
                return Err(CodecError::new(
                    code::REF_FIELD,
                    format!("ContentRef 出现 exact 四字段之外的键 `{key}`"),
                ));
            }
        }
        let field = |name: &str| -> Result<&JsonNode, CodecError> {
            map.get(name).ok_or_else(|| {
                CodecError::new(code::REF_FIELD, format!("ContentRef 缺字段 `{name}`"))
            })
        };
        let format = field("format")?
            .as_str()
            .ok_or_else(|| CodecError::new(code::REF_FIELD, "format 必须是字符串".to_string()))?
            .to_string();
        let root = field("root")?
            .as_str()
            .ok_or_else(|| CodecError::new(code::REF_FIELD, "root 必须是字符串".to_string()))?
            .to_string();
        let kind_name = field("kind")?
            .as_str()
            .ok_or_else(|| CodecError::new(code::REF_FIELD, "kind 必须是字符串".to_string()))?;
        let kind = Kind::from_name(kind_name).ok_or_else(|| {
            CodecError::new(code::KIND, format!("未知 kind `{kind_name}`"))
        })?;
        let extent = match field("extent")? {
            JsonNode::Null => None,
            JsonNode::String(text) => Some(parse_canonical_u64(text, "extent")?),
            other => {
                return Err(CodecError::new(
                    code::EXTENT,
                    format!(
                        "extent 必须是规范十进制字符串或 null（U64 长度按 §5.2 用十进制字符串传输），实为 {other}"
                    ),
                ))
            }
        };
        if kind.extent_is_byte_or_count() != extent.is_some() {
            return Err(CodecError::new(
                code::EXTENT,
                format!(
                    "kind={} 的 extent 必须是{}",
                    kind.name(),
                    if kind.extent_is_byte_or_count() {
                        "规范十进制字符串"
                    } else {
                        "null"
                    }
                ),
            ));
        }
        Ok(ContentRef {
            format,
            root,
            kind,
            extent,
        })
    }
}

/// 公开控制数值的规范十进制：`0` 或 `[1-9][0-9]*`，无 `+`、无前导零、≤ 2^63-1。
fn parse_canonical_u64(text: &str, what: &str) -> Result<u64, CodecError> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || !bytes.iter().all(|b| b.is_ascii_digit()) {
        return Err(CodecError::new(
            code::EXTENT,
            format!("{what} `{text}` 不是规范十进制：只允许 0-9，无符号、无空白"),
        ));
    }
    if bytes[0] == b'0' && bytes.len() > 1 {
        return Err(CodecError::new(
            code::EXTENT,
            format!("{what} `{text}` 有前导零，非规范十进制"),
        ));
    }
    let n = text.parse::<u64>().map_err(|_| {
        CodecError::new(
            code::LENGTH_OVERFLOW,
            format!("{what} `{text}` 超出 u64 表达"),
        )
    })?;
    if n > MAX_FRAME_LEN {
        return Err(CodecError::new(
            code::LENGTH_OVERFLOW,
            format!("{what} `{text}` 超过上限 2^63-1"),
        ));
    }
    Ok(n)
}

/// 写入字节序列 B：落块节点与序列节点，返回根摘要。
pub fn write_bytes(sink: &mut dyn NodeSink, bytes: &[u8]) -> Result<NodeDigest, CodecError> {
    if bytes.is_empty() {
        let digest = frame(tag::SEQ_EMPTY, &[]);
        sink.put(digest, tag::SEQ_EMPTY, &[])?;
        return Ok(digest);
    }
    let mut leaves = Vec::with_capacity(bytes.len().div_ceil(CHUNK_BYTES));
    for chunk in bytes.chunks(CHUNK_BYTES) {
        let digest = frame(tag::BYTES_CHUNK, chunk);
        sink.put(digest, tag::BYTES_CHUNK, chunk)?;
        leaves.push(digest);
    }
    write_sequence(sink, &leaves)
}

/// 写入序列节点 M(leaves)：空 → 0x11；单叶 → 0x12；其余按 k 规则分解。
pub fn write_sequence(
    sink: &mut dyn NodeSink,
    leaves: &[NodeDigest],
) -> Result<NodeDigest, CodecError> {
    if leaves.is_empty() {
        let digest = frame(tag::SEQ_EMPTY, &[]);
        sink.put(digest, tag::SEQ_EMPTY, &[])?;
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
    let mut stack: Vec<(u64, NodeDigest)> = Vec::new();
    for leaf in leaves {
        let leaf_node = frame(tag::SEQ_LEAF, leaf);
        sink.put(leaf_node, tag::SEQ_LEAF, leaf)?;
        stack.push((1, leaf_node));
        while stack.len() >= 2 {
            let (n1, d1) = stack[stack.len() - 1];
            let (n2, d2) = stack[stack.len() - 2];
            if n1 != n2 {
                break;
            }
            stack.truncate(stack.len() - 2);
            let n = n1 + n2;
            let digest = put_branch(sink, &d2, &d1, n)?;
            stack.push((n, digest));
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
    sink: &mut dyn NodeSink,
    left: &NodeDigest,
    right: &NodeDigest,
    n: u64,
) -> Result<NodeDigest, CodecError> {
    let mut body = [0u8; 72];
    body[..8].copy_from_slice(&checked_frame_len(n)?);
    body[8..40].copy_from_slice(left);
    body[40..72].copy_from_slice(right);
    let digest = frame(tag::SEQ_BRANCH, &body);
    sink.put(digest, tag::SEQ_BRANCH, &body)?;
    Ok(digest)
}

/// 写入值树（通用公开逻辑值）：模板占位（0x08）拒绝。
///
/// 写入 ≠ publish-valid：`Value::Edge` 指向的子根不在本次写入的核验范围内，
/// 也可能指向缺失节点；publish 前必须过 `verify`。
pub fn write_value(sink: &mut dyn NodeSink, value: &Value) -> Result<Built, CodecError> {
    write_in_mode(sink, value, Mode::Public)
}

/// 写入值树（显式模板上下文）：允许 `Value::BatchSelf`。
pub fn write_template(sink: &mut dyn NodeSink, value: &Value) -> Result<Built, CodecError> {
    write_in_mode(sink, value, Mode::Template)
}

fn write_in_mode(
    sink: &mut dyn NodeSink,
    value: &Value,
    mode: Mode,
) -> Result<Built, CodecError> {
    let (kind, extent) = value_kind_extent(value, mode)?;
    let mut steps = vec![WStep::Value(value)];
    let mut done: Vec<NodeDigest> = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            WStep::Value(v) => write_value_step(sink, v, mode, &mut steps, &mut done)?,
            WStep::Json(v) => write_json_step(sink, v, &mut steps, &mut done)?,
            WStep::FinishArray { base } => {
                let children = &done[base..];
                let n = u64::try_from(children.len()).expect("元素数可表达");
                let fields = write_sequence(sink, children)?;
                let mut body = Vec::with_capacity(40);
                body.extend_from_slice(&checked_frame_len(n)?);
                body.extend_from_slice(&fields);
                let digest = put_node(sink, tag::ARRAY, &body)?;
                done.truncate(base);
                done.push(digest);
            }
            WStep::FinishObject { base, keys } => {
                let n = u64::try_from(keys.len()).expect("字段数可表达");
                if done.len() != base + keys.len() {
                    return Err(CodecError::new(
                        code::SEQUENCE_SHAPE,
                        "对象字段记账与已写入子值不符".to_string(),
                    ));
                }
                let mut order: Vec<usize> = (0..keys.len()).collect();
                order.sort_by(|&a, &b| keys[a].as_bytes().cmp(keys[b].as_bytes()));
                for w in order.windows(2) {
                    if keys[w[0]] == keys[w[1]] {
                        return Err(CodecError::new(
                            code::DUPLICATE_KEY,
                            format!("对象出现重复键 `{}`", keys[w[0]]),
                        ));
                    }
                }
                let mut field_digests = Vec::with_capacity(order.len());
                for &i in &order {
                    let key_digest = write_string_node(sink, keys[i])?;
                    let mut body = [0u8; 64];
                    body[..32].copy_from_slice(&key_digest);
                    body[32..].copy_from_slice(&done[base + i]);
                    field_digests.push(put_node(sink, tag::OBJECT_FIELD, &body)?);
                }
                let fields = write_sequence(sink, &field_digests)?;
                let mut body = Vec::with_capacity(40);
                body.extend_from_slice(&checked_frame_len(n)?);
                body.extend_from_slice(&fields);
                let digest = put_node(sink, tag::OBJECT, &body)?;
                done.truncate(base);
                done.push(digest);
            }
            WStep::FinishJsonText { base } => {
                let inner = done[base];
                let digest = put_node(sink, tag::JSON_TEXT, &inner)?;
                done.truncate(base);
                done.push(digest);
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

enum WStep<'a> {
    Value(&'a Value),
    Json(&'a JsonValue),
    FinishArray { base: usize },
    FinishObject { base: usize, keys: Vec<&'a str> },
    FinishJsonText { base: usize },
}

fn put_node(sink: &mut dyn NodeSink, t: u8, body: &[u8]) -> Result<NodeDigest, CodecError> {
    let digest = frame(t, body);
    sink.put(digest, t, body)?;
    Ok(digest)
}

fn write_string_node(sink: &mut dyn NodeSink, text: &str) -> Result<NodeDigest, CodecError> {
    let bytes = text.as_bytes();
    let root = write_bytes(sink, bytes)?;
    let mut body = Vec::with_capacity(40);
    body.extend_from_slice(&checked_frame_len(u64::try_from(bytes.len()).expect("字节数可表达"))?);
    body.extend_from_slice(&root);
    put_node(sink, tag::STRING, &body)
}

fn write_byte_node(
    sink: &mut dyn NodeSink,
    t: u8,
    bytes: &[u8],
) -> Result<NodeDigest, CodecError> {
    let root = write_bytes(sink, bytes)?;
    let mut body = Vec::with_capacity(40);
    body.extend_from_slice(&checked_frame_len(u64::try_from(bytes.len()).expect("字节数可表达"))?);
    body.extend_from_slice(&root);
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
    sink: &mut dyn NodeSink,
    value: &'a Value,
    mode: Mode,
    steps: &mut Vec<WStep<'a>>,
    done: &mut Vec<NodeDigest>,
) -> Result<(), CodecError> {
    match value {
        Value::Null => done.push(put_node(sink, tag::NULL, &[])?),
        Value::Bool(false) => done.push(put_node(sink, tag::BOOL_FALSE, &[])?),
        Value::Bool(true) => done.push(put_node(sink, tag::BOOL_TRUE, &[])?),
        Value::String(s) => done.push(write_string_node(sink, s)?),
        Value::Number(t) => done.push(write_byte_node(sink, tag::NUMBER, t.bytes())?),
        Value::Array(items) => {
            steps.push(WStep::FinishArray { base: done.len() });
            for item in items.iter().rev() {
                steps.push(WStep::Value(item));
            }
        }
        Value::Object(fields) => {
            steps.push(WStep::FinishObject {
                base: done.len(),
                keys: fields.iter().map(|(k, _)| k.as_str()).collect(),
            });
            for (_, v) in fields.iter().rev() {
                steps.push(WStep::Value(v));
            }
        }
        Value::JsonText(inner) => {
            steps.push(WStep::FinishJsonText { base: done.len() });
            steps.push(WStep::Json(inner));
        }
        Value::BatchSelf => {
            if mode != Mode::Template {
                return Err(CodecError::new(
                    code::TEMPLATE_PLACEHOLDER,
                    "模板占位（0x08）只在显式模板上下文里允许写".to_string(),
                ));
            }
            done.push(put_node(sink, tag::BATCH_SELF, &[])?);
        }
        Value::Edge(digest) => done.push(*digest),
    }
    Ok(())
}

fn write_json_step<'a>(
    sink: &mut dyn NodeSink,
    value: &'a JsonValue,
    steps: &mut Vec<WStep<'a>>,
    done: &mut Vec<NodeDigest>,
) -> Result<(), CodecError> {
    match value {
        JsonValue::Null => done.push(put_node(sink, tag::NULL, &[])?),
        JsonValue::Bool(false) => done.push(put_node(sink, tag::BOOL_FALSE, &[])?),
        JsonValue::Bool(true) => done.push(put_node(sink, tag::BOOL_TRUE, &[])?),
        JsonValue::String(s) => done.push(write_string_node(sink, s)?),
        JsonValue::Number(t) => done.push(write_byte_node(sink, tag::NUMBER, t.bytes())?),
        JsonValue::Array(items) => {
            steps.push(WStep::FinishArray { base: done.len() });
            for item in items.iter().rev() {
                steps.push(WStep::Json(item));
            }
        }
        JsonValue::Object(fields) => {
            steps.push(WStep::FinishObject {
                base: done.len(),
                keys: fields.iter().map(|(k, _)| k.as_str()).collect(),
            });
            for (_, v) in fields.iter().rev() {
                steps.push(WStep::Json(v));
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
            Some(Utf8State::Pending { len: len + 1, b: next })
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

// ── 惰性叶迭代（渲染与键比较用；不预收集整棵序列） ──────────────────────────

/// 顺序产出序列树的叶：只需 O(树深) 栈，按消费推进。
#[derive(Debug, Clone)]
struct LeafIter {
    stack: Vec<NodeDigest>,
}

impl LeafIter {
    fn new(root: NodeDigest) -> Self {
        Self { stack: vec![root] }
    }

    /// 下一个叶（tag, 摘要, body）；叶的类型由调用方按上下文核。
    fn next(
        &mut self,
        source: &mut dyn NodeSource,
    ) -> Result<Option<(u8, NodeDigest, Vec<u8>)>, CodecError> {
        while let Some(digest) = self.stack.pop() {
            let (t, body) = read_checked(source, &digest)?;
            match t {
                tag::SEQ_EMPTY => {
                    if !body.is_empty() {
                        return Err(body_shape_error(t, &body, "空序列 body 必须为空"));
                    }
                }
                tag::SEQ_LEAF => {
                    if body.len() != 32 {
                        return Err(sequence_shape_error(
                            "单叶序列 body 必须是 32 字节叶摘要",
                        ));
                    }
                    let leaf = digest_at(&body, 0);
                    let (lt, lbody) = read_checked(source, &leaf)?;
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
                    self.stack.push(right);
                    self.stack.push(left);
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

struct Verifier<'a> {
    source: &'a mut dyn NodeSource,
    mode: Mode,
    /// 已核过的 (摘要, 上下文)。
    seen: HashSet<(NodeDigest, Flavor)>,
    /// 已入待核集合的 (摘要, 上下文) —— 重复子树不重复入队。
    pushed: HashSet<(NodeDigest, Flavor)>,
    todo: Vec<(NodeDigest, Flavor)>,
    /// 实际读到的不同摘要（唯一节点计数）。
    read_digests: HashSet<NodeDigest>,
    bytes_memo: HashMap<(NodeDigest, Utf8State, NumState), ByteAgg>,
    elem_memo: HashMap<(NodeDigest, Flavor), u64>,
    field_memo: HashMap<(NodeDigest, Flavor), FieldAgg>,
    key_cmp: HashMap<(NodeDigest, NodeDigest), std::cmp::Ordering>,
}

impl<'a> Verifier<'a> {
    fn new(source: &'a mut dyn NodeSource, mode: Mode) -> Self {
        Self {
            source,
            mode,
            seen: HashSet::new(),
            pushed: HashSet::new(),
            todo: Vec::new(),
            read_digests: HashSet::new(),
            bytes_memo: HashMap::new(),
            elem_memo: HashMap::new(),
            field_memo: HashMap::new(),
            key_cmp: HashMap::new(),
        }
    }

    /// 读节点 + 核摘要 + 记唯一节点。
    fn read_node(&mut self, digest: &NodeDigest) -> Result<(u8, Vec<u8>), CodecError> {
        let got = read_checked(self.source, digest)?;
        self.read_digests.insert(*digest);
        Ok(got)
    }

    fn push_work(&mut self, digest: NodeDigest, flavor: Flavor) {
        if self.pushed.insert((digest, flavor)) {
            self.todo.push((digest, flavor));
        }
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
        let mut steps = vec![Step::Enter {
            digest: *root,
            utf8: Utf8State::Empty,
            num: NumState::Start,
        }];
        let mut aggs: Vec<ByteAgg> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                Step::Enter {
                    digest,
                    utf8,
                    num,
                } => {
                    if let Some(agg) = self.bytes_memo.get(&(digest, utf8, num)) {
                        aggs.push(*agg);
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
                            self.bytes_memo.insert((digest, utf8, num), agg);
                            aggs.push(agg);
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
                            self.bytes_memo.insert((digest, utf8, num), agg);
                            aggs.push(agg);
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
                            });
                            steps.push(Step::Right { digest: right });
                            steps.push(Step::Enter {
                                digest: left,
                                utf8,
                                num,
                            });
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
                    });
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
                    self.bytes_memo.insert((digest, utf8, num), agg);
                    aggs.push(agg);
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
        let mut steps = vec![Step::Enter(*root)];
        let mut counts: Vec<u64> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                Step::Enter(digest) => {
                    if let Some(count) = self.elem_memo.get(&(digest, flavor)) {
                        counts.push(*count);
                        continue;
                    }
                    let (t, body) = self.read_node(&digest)?;
                    match t {
                        tag::SEQ_EMPTY => {
                            if !body.is_empty() {
                                return Err(body_shape_error(t, &body, "空序列 body 必须为空"));
                            }
                            self.elem_memo.insert((digest, flavor), 0);
                            counts.push(0);
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
                            self.push_work(leaf, flavor);
                            self.elem_memo.insert((digest, flavor), 1);
                            counts.push(1);
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
                            steps.push(Step::Combine { digest, n });
                            steps.push(Step::Enter(right));
                            steps.push(Step::Enter(left));
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
                    self.elem_memo.insert((digest, flavor), n);
                    counts.push(n);
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
        let mut steps = vec![Step::Enter(*root)];
        let mut aggs: Vec<FieldAgg> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                Step::Enter(digest) => {
                    if let Some(agg) = self.field_memo.get(&(digest, flavor)) {
                        aggs.push(*agg);
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
                            self.field_memo.insert((digest, flavor), agg);
                            aggs.push(agg);
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
                            self.seen.insert((key, Flavor::Value));
                            self.push_work(value, flavor);
                            let agg = FieldAgg {
                                count: 1,
                                first: Some(key),
                                last: Some(key),
                            };
                            self.field_memo.insert((digest, flavor), agg);
                            aggs.push(agg);
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
                            steps.push(Step::Combine { digest, n });
                            steps.push(Step::Enter(right));
                            steps.push(Step::Enter(left));
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
                    self.field_memo.insert((digest, flavor), agg);
                    aggs.push(agg);
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
        self.key_cmp.insert((*a, *b), ord);
        self.key_cmp.insert((*b, *a), ord.reverse());
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
        let mut iter_a = LeafIter::new(root_a);
        let mut iter_b = LeafIter::new(root_b);
        let mut buf_a: Vec<u8> = Vec::new();
        let mut buf_b: Vec<u8> = Vec::new();
        let mut pos_a = 0usize;
        let mut pos_b = 0usize;
        let mut dig_a: Option<NodeDigest> = None;
        let mut dig_b: Option<NodeDigest> = None;
        loop {
            if pos_a == buf_a.len() {
                match iter_a.next(self.source)? {
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
                match iter_b.next(self.source)? {
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
                    return Ok(if x < y { Ordering::Less } else { Ordering::Greater });
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
        let digest = reference.digest()?;
        let (t, body) = self.read_node(&digest)?;
        let kind = Kind::from_tag(t).ok_or_else(|| unsupported_value_tag_error(t))?;
        if kind != reference.kind {
            return Err(CodecError::new(
                code::KIND,
                format!(
                    "ContentRef.kind={} 与真实节点 tag={t:02x}（{}）不符",
                    reference.kind.name(),
                    kind.name()
                ),
            ));
        }
        let extent = header_extent(t, &body)?;
        if extent != reference.extent {
            return Err(CodecError::new(
                code::EXTENT,
                format!(
                    "ContentRef.extent={:?} 与真实节点 header {:?} 不符",
                    reference.extent, extent
                ),
            ));
        }
        self.run(&digest)?;
        Ok((digest, kind, extent))
    }

    /// 全闭包核验：值节点按 (摘要, 上下文) 去重，序列按摘要折叠。
    fn run(&mut self, root: &NodeDigest) -> Result<(), CodecError> {
        self.push_work(*root, Flavor::Value);
        while let Some((digest, flavor)) = self.todo.pop() {
            if !self.seen.insert((digest, flavor)) {
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
                        return Err(body_shape_error(t, &body, "body 必须是内部值的 32 字节摘要"));
                    }
                    self.push_work(digest_at(&body, 0), Flavor::JsonInner);
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
    let digest = parse_root(root)?;
    let (t, body) = read_checked(source, &digest)?;
    let kind = Kind::from_tag(t).ok_or_else(|| unsupported_value_tag_error(t))?;
    let extent = header_extent(t, &body)?;
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

// ── 渲染：惰性叶迭代 + 规范 JSON 转义 ────────────────────────────────────────

/// 逐字符写出规范 JSON 转义（斜线不转义；控制字符用小写四位十六进制）。
fn write_escaped_char(ch: char, out: &mut dyn Write) -> Result<(), CodecError> {
    match ch {
        '"' => out.write_all(b"\\\""),
        '\\' => out.write_all(b"\\\\"),
        '\u{0008}' => out.write_all(b"\\b"),
        '\u{0009}' => out.write_all(b"\\t"),
        '\u{000A}' => out.write_all(b"\\n"),
        '\u{000C}' => out.write_all(b"\\f"),
        '\u{000D}' => out.write_all(b"\\r"),
        c if (c as u32) < 0x20 => {
            let esc = format!("\\u{:04x}", c as u32);
            out.write_all(esc.as_bytes())
        }
        c => {
            let mut buf = [0u8; 4];
            out.write_all(c.encode_utf8(&mut buf).as_bytes())
        }
    }
    .map_err(io_error)
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

struct Renderer<'a> {
    source: &'a mut dyn NodeSource,
    out: &'a mut dyn Write,
    escaping: bool,
    esc_decoder: Utf8Decoder,
}

impl Renderer<'_> {
    /// 写出字节：处于转义层时逐字符转义，否则原样写出。
    fn emit(&mut self, bytes: &[u8]) -> Result<(), CodecError> {
        let Renderer {
            out,
            escaping,
            esc_decoder,
            ..
        } = self;
        if !*escaping {
            return out.write_all(bytes).map_err(io_error);
        }
        let mut buf: Vec<u8> = Vec::with_capacity(bytes.len() + 8);
        esc_decoder.feed(bytes, &mut |ch| write_escaped_char(ch, &mut buf))?;
        out.write_all(&buf).map_err(io_error)
    }

    fn run(&mut self, root: NodeDigest) -> Result<(), CodecError> {
        let mut tasks = vec![RTask::Value(root, Flavor::Value)];
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
                        tasks.push(RTask::Emit(b"]"));
                        continue;
                    }
                    let (lt, leaf, _body) = iter
                        .next(&mut *self.source)?
                        .ok_or_else(|| sequence_shape_error("元素序列提前结束"))?;
                    if Kind::from_tag(lt).is_none() {
                        return Err(unsupported_value_tag_error(lt));
                    }
                    tasks.push(RTask::Elements {
                        iter,
                        index: index + 1,
                        total,
                        flavor,
                    });
                    tasks.push(RTask::Value(leaf, flavor));
                    if index > 0 {
                        tasks.push(RTask::Emit(b","));
                    }
                }
                RTask::Fields {
                    mut iter,
                    index,
                    total,
                    flavor,
                } => {
                    if index == total {
                        tasks.push(RTask::Emit(b"}"));
                        continue;
                    }
                    let (ft, _field, fbody) = iter
                        .next(&mut *self.source)?
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
                    });
                    tasks.push(RTask::Value(value, flavor));
                    tasks.push(RTask::Emit(b":"));
                    tasks.push(RTask::Value(key, Flavor::Value));
                    if index > 0 {
                        tasks.push(RTask::Emit(b","));
                    }
                }
                RTask::Value(digest, flavor) => {
                    let (t, body) = read_checked(self.source, &digest)?;
                    match t {
                        tag::NULL => tasks.push(RTask::Emit(b"null")),
                        tag::BOOL_FALSE => tasks.push(RTask::Emit(b"false")),
                        tag::BOOL_TRUE => tasks.push(RTask::Emit(b"true")),
                        tag::STRING => {
                            framed_len(&body, "字符串")?;
                            tasks.push(RTask::Emit(b"\""));
                            tasks.push(RTask::StringBody(digest));
                            tasks.push(RTask::Emit(b"\""));
                        }
                        tag::NUMBER => {
                            framed_len(&body, "数字 token")?;
                            tasks.push(RTask::TokenBody(digest));
                        }
                        tag::ARRAY => {
                            let declared = framed_len(&body, "数组")?;
                            tasks.push(RTask::Elements {
                                iter: LeafIter::new(digest_at(&body, 8)),
                                index: 0,
                                total: declared,
                                flavor,
                            });
                            tasks.push(RTask::Emit(b"["));
                        }
                        tag::OBJECT => {
                            let declared = framed_len(&body, "对象")?;
                            tasks.push(RTask::Fields {
                                iter: LeafIter::new(digest_at(&body, 8)),
                                index: 0,
                                total: declared,
                                flavor,
                            });
                            tasks.push(RTask::Emit(b"{"));
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
                            tasks.push(RTask::Emit(b"\""));
                            tasks.push(RTask::ExitEscape);
                            tasks.push(RTask::Value(digest_at(&body, 0), Flavor::JsonInner));
                            tasks.push(RTask::EnterEscape);
                            tasks.push(RTask::Emit(b"\""));
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
        let (t, body) = read_checked(self.source, digest)?;
        if t != tag::STRING {
            return Err(CodecError::new(
                code::VALUE_KIND,
                format!("字符串节点必须是 0x03，读到 tag={t:02x}"),
            ));
        }
        let declared = framed_len(&body, "字符串")?;
        let mut iter = LeafIter::new(digest_at(&body, 8));
        let mut decoder = Utf8Decoder::default();
        let mut escaped: Vec<u8> = Vec::with_capacity(8);
        let mut total: u64 = 0;
        loop {
            let next = {
                let Renderer { source, .. } = self;
                iter.next(&mut **source)?
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
                write_escaped_char(ch, &mut escaped)?;
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
        let (t, body) = read_checked(self.source, digest)?;
        if t != tag::NUMBER {
            return Err(CodecError::new(
                code::VALUE_KIND,
                format!("数字 token 必须是 0x04 节点，读到 tag={t:02x}"),
            ));
        }
        let declared = framed_len(&body, "数字 token")?;
        let mut iter = LeafIter::new(digest_at(&body, 8));
        let mut state = NumState::Start;
        let mut total: u64 = 0;
        loop {
            let next = {
                let Renderer { source, .. } = self;
                iter.next(&mut **source)?
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
    struct Bounded<'a> { inner: &'a mut dyn Write, limit: Option<u64>, written: u64 }
    impl Write for Bounded<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.limit.is_some_and(|limit| (self.written as u128)+(bytes.len() as u128)>limit as u128) {
                return Err(std::io::Error::other("Incomplete/CaptureBytes：内容渲染超过显式物化上限"));
            }
            let n=self.inner.write(bytes)?;
            self.written=self.written.saturating_add(n as u64);
            Ok(n)
        }
        fn flush(&mut self)->std::io::Result<()> {self.inner.flush()}
    }
    let mut bounded=Bounded { inner:out, limit:source.max_render_bytes(), written:0 };
    let verified = verify(source, reference)?;
    let mut renderer = Renderer {
        source,
        out: &mut bounded,
        escaping: false,
        esc_decoder: Utf8Decoder::default(),
    };
    renderer.run(verified.digest)
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
        for ok in ["0", "-0", "1", "1.0", "1e0", "1E+5", "-12.5e-3", "10", "1e-0"] {
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
            verify(&mut store, &wrong_kind).expect_err("kind 不符").code(),
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
        let second = write_value(&mut store, &Value::Array(vec![Value::Edge(shared.digest)])).unwrap();

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
        let unique = verify_many(
            &mut many,
            vec![first.content_ref(), second.content_ref()],
        )
        .unwrap();
        assert_eq!(unique, many.unique.len());
        assert!(many.reads < separate_reads, "共享根应减少读取：{} >= {}", many.reads, separate_reads);
    }

    #[test]
    fn verify_many_rejects_second_root_header_and_bad_closure() {
        let mut store = MemoryNodes::new();
        let value = write_value(&mut store, &Value::string("x")).unwrap();
        let mut wrong_extent = value.content_ref();
        wrong_extent.extent = Some(999);
        let err = verify_many(
            &mut store,
            vec![value.content_ref(), wrong_extent],
        )
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
        let err = verify_many(
            &mut store,
            vec![value.content_ref(), missing.content_ref()],
        )
        .expect_err("后续根缺子节点必须拒绝");
        assert_eq!(err.code(), code::NODE_MISSING);

        let bad = write_value(&mut store, &Value::string("bad")).unwrap();
        store.insert_unchecked(bad.digest, tag::STRING, vec![0; 40]);
        let err = verify_many(
            &mut store,
            vec![value.content_ref(), bad.content_ref()],
        )
        .expect_err("后续根摘要损坏必须拒绝");
        assert_eq!(err.code(), code::NODE_DIGEST);
    }

    #[test]
    fn verify_many_does_not_inherit_success_or_open_json_text_context() {
        let mut full = MemoryNodes::new();
        let child = write_value(&mut full, &Value::string("child")).unwrap();
        let graph = write_value(&mut full, &Value::Array(vec![Value::Edge(child.digest)])).unwrap();
        assert_eq!(verify_many(&mut full, vec![graph.content_ref()]).unwrap(), 5);

        let (root_tag, root_body) = full
            .get(&graph.digest)
            .map(|(tag, body)| (tag, body.to_vec()))
            .unwrap();
        let mut partial = MemoryNodes::new();
        partial.insert_unchecked(graph.digest, root_tag, root_body);
        let err = verify_many(&mut partial, vec![graph.content_ref()]).expect_err("调用间不得继承旧闭包");
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
        let built = write_value(&mut store, &Value::JsonText(Box::new(inner))).expect("写入文本视图");
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
        assert_eq!(a.digest, b.digest, "对象字段按键的 UTF-8 字节升序，与输入顺序无关");

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
            (JsonNode::String("9223372036854775808".into()), code::LENGTH_OVERFLOW),
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
