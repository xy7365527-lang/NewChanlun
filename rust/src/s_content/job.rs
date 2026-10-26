//! s-content/1 的共用 job 资源累计核心（#1394 采纳 C §5.6.1 的记账面）。
//!
//! 规范正本：`.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md` §5.6.1。
//!
//! 本模块**只做记账**：八字段上限的显式校验、累计量、绝对期限、预留与对账。它不做持久化、
//! 不做网络、不分配暂存文件、不读时钟以外的任何外部状态。**核心计费接通不等于所有 I/O 已经接线**：
//! `wire_bytes`／`records`／`requests` 必须由 adapter 在真实发生过之后显式 `charge`，
//! `max_staged_bytes`／`max_store_bytes` 的既存分配基线也必须由调用方实测后显式给出。
//!
//! 计费口径（逐条对应 §5.6.1 表）：
//! - `max_total_wire_bytes`：应用层实际读写字节累计。重试与证明祖先重传**全额再计**，不去重。
//!   入账口径是**先照实记录、再核上限与期限**：失败那一次已经发生的物理字节仍进累计
//!   （旧行为「失败不留部分计费」按 §5.6.1 的实读后计费修正）；已停止的 job 不再接纳新字节。
//! - `max_total_node_bytes` / `max_total_nodes`：只有 `F(tag, body)` 与摘要核对通过的
//!   不同根节点才计入，同一根只计一次（**包括**来自本地缓存的复用）；未核摘要不进已计费集合，
//!   坏节点重复出现也不能靠「已计过」蒙混过关。
//! - `max_total_records`：顶层记录访问次数；同一次读取内的重访照实再计，节点去重不免除记录计数。
//! - `max_total_requests`：本 job 发起的 HTTP／reader 子进程／对端捕获尝试次数，含错误与重试。
//! - `max_elapsed_ms`：**首次读取**起算的绝对期限；排队、重试、来源变化重读都消耗它，
//!   逐片段调用不会续期（[`Job::begin_first_read`] 幂等）。
//! - `max_staged_bytes`：本 job **暂存子集**（暂存文件、索引、日志）的实际分配峰值。
//! - `max_store_bytes`：本地内容存储**总账**上限，含既存已提交内容、本 job 暂存子集、索引与日志。
//!
//! 暂存是总存储的**子集**，不是两笔独立额度：
//! - [`Existing`] 的两项基线在 [`Job::new`] 当场核上限，因此纯读取 job（一次 `reserve` 都不发）
//!   也照旧受 `max_store_bytes` 约束；基线没有缺省，0 只能作为「实测为空」的声明。
//! - 一笔暂存预留的同一批字节**同时**占 `max_staged_bytes` 与 `max_store_bytes`
//!   （[`Job::reserve_staged`] 一次调用核两本 cap，失败原子，不需要 adapter 先后发两笔预留）。
//! - 总账里本 job 的暂存子集只出现一次：`Existing.store_bytes` 按契约**不含**本 job 自己的暂存。
//! - 会保留的索引／日志走 [`Job::reserve_store`]（本模块不为它开回收口，因为不能因 job 失败就删）；
//!   可能被取消的导入内容走 [`Job::reserve_staged`]，那条路能按实际删除量回收。
//!
//! 停止语义：任一上限首次超限即**停止**该 job（见 [`JobSnapshot::stopped`]），此后不再接纳新的
//! charge 与预留；已开预留仍可 `reconcile`／`release`，已对账暂存仍可 `reclaim`（只减少敞口）。
//! `reconcile` 的实际量超过预留量时同样停止，但**真实 actual 照实入账**（不低估、不溢出）。
//!
//! 空间账（[`SpaceSnapshot`] 的 `baseline`／`committed`／`pending`／`exposure`／`peak`）是纯内部
//! 记账量，用 `u128`：上限与 `actual`／`requested` 仍是 `u64`，但失败路径上的实际分配与基线相加
//! 会越过 `u64`，`u64` 会溢出或低估。这些数不是 wire 协议，没有对外字节形状承诺。
//!
//! 累计账是**永久**的：`wire_bytes`／`nodes`／`node_bytes`／`records`／`requests` 不因取消 job、
//! 释放预留或暂存回收而归零，也没有 `reset` 入口——重开一个 job 只能新建 [`Job`]。
//!
//! 已知局限（照实）：
//! - 读包装在底层 `read` 完整返回 body **之后**才计数，所以 `node_bytes` 是记账值、不是内存上限；
//!   底层读取的内存预分配不由本包装拦住。
//! - 写入面（本地构造／导入）没有包装，也不恢复 `NodeSink`：调用方按**待写实际节点**先在写前核
//!   `F(tag, body)` 与节点预算（[`Job::accept_node`]），再落盘；节点账不承诺落盘回滚，实际 wire
//!   是事后显式 charge，容量走写前 `reserve_*` + 写后 `reconcile`，三条通道彼此分开。
//! - `Box<dyn Clock>` 未加 `Send` 约束：需要把 [`Job`] 搬到别的线程的调用方自行包装。
//! - 本模块不判节点类型语义、不做存储／网络适配，也不提供 §5.6.1 的首批数值。

use super::{
    code, frame, root_string, CodecError, NodeDigest, NodeRead, NodeSource, DOMAIN_SEPARATOR,
    MAX_FRAME_LEN,
};
use serde_json::{Map as JsonMap, Value as JsonNode};
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// §5.6.1 表序的八字段名（exact：多一个、少一个、换名字都不接受）。
pub const LIMIT_FIELDS: [&str; 8] = [
    "max_total_wire_bytes",
    "max_total_node_bytes",
    "max_total_nodes",
    "max_total_records",
    "max_total_requests",
    "max_elapsed_ms",
    "max_staged_bytes",
    "max_store_bytes",
];

/// 八字段的解析域上限：2^63-1（与 §5.1 的 U64 帧上限同一数）。
pub const MAX_LIMIT: u64 = MAX_FRAME_LEN;

/// `job_limit_field`：`job_limits` 字段集合不 exact（缺字段、多字段，或值不是规范十进制字符串）。
pub const LIMIT_FIELD: &str = "job_limit_field";
/// `job_limit_domain`：字段值不在 1..=2^63-1，或暂存基线大于存储基线（子集关系不成立）。
pub const LIMIT_DOMAIN: &str = "job_limit_domain";
/// `job_limit_exceeded`：某项累计、基线或预留会超出上限（本 job 就此停止）。
pub const LIMIT_EXCEEDED: &str = "job_limit_exceeded";
/// `job_deadline`：绝对期限已过（本 job 就此停止）。
pub const DEADLINE: &str = "job_deadline";
/// `job_reservation`：预留句柄不属于本 job、已关闭，或收尾时仍有未对账预留。
pub const RESERVATION: &str = "job_reservation";
/// `job_reconcile`：写后实际量超过预留量（不足即停止）。
pub const RECONCILE: &str = "job_reconcile";
/// `job_reclaim`：回收量超过本 job 已对账的暂存量。
pub const RECLAIM: &str = "job_reclaim";

/// §5.6.1 的 exact 八字段上限。内存形态用整数，公开 JSON 配置用规范十进制字符串。
///
/// **没有 `Default`**：八项必须逐项显式给出；缺项、零值、无限量都不是合法入口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobLimits {
    pub max_total_wire_bytes: u64,
    pub max_total_node_bytes: u64,
    pub max_total_nodes: u64,
    pub max_total_records: u64,
    pub max_total_requests: u64,
    pub max_elapsed_ms: u64,
    pub max_staged_bytes: u64,
    pub max_store_bytes: u64,
}

impl JobLimits {
    /// 公开 JSON 配置入口：对象必须 exact 含八个键，值必须是规范十进制字符串。
    ///
    /// JSON 数字、缺字段、多字段、`+`／`-`／前导零／空白／小数点一律拒绝。
    pub fn from_canonical_json(node: &JsonNode) -> Result<JobLimits, CodecError> {
        let object = node.as_object().ok_or_else(|| {
            CodecError::new(
                LIMIT_FIELD,
                "job_limits 必须是 JSON 对象，本模块不提供缺省值",
            )
        })?;
        for key in object.keys() {
            if !LIMIT_FIELDS.contains(&key.as_str()) {
                return Err(CodecError::new(
                    LIMIT_FIELD,
                    format!("job_limits 含未声明字段 `{key}`；exact 只需八个字段"),
                ));
            }
        }
        Ok(JobLimits {
            max_total_wire_bytes: limit_field(object, LIMIT_FIELDS[0])?,
            max_total_node_bytes: limit_field(object, LIMIT_FIELDS[1])?,
            max_total_nodes: limit_field(object, LIMIT_FIELDS[2])?,
            max_total_records: limit_field(object, LIMIT_FIELDS[3])?,
            max_total_requests: limit_field(object, LIMIT_FIELDS[4])?,
            max_elapsed_ms: limit_field(object, LIMIT_FIELDS[5])?,
            max_staged_bytes: limit_field(object, LIMIT_FIELDS[6])?,
            max_store_bytes: limit_field(object, LIMIT_FIELDS[7])?,
        })
    }

    /// 八项逐项校验：1..=2^63-1。内存形态构建后仍须过这一关（[`Job::new`] 会调用）。
    pub fn validate(&self) -> Result<(), CodecError> {
        for (name, value) in self.entries() {
            if value == 0 || value > MAX_LIMIT {
                return Err(CodecError::new(
                    LIMIT_DOMAIN,
                    format!("{name} = {value} 不在 1..={MAX_LIMIT}；无缺省、无无限量"),
                ));
            }
        }
        Ok(())
    }

    /// 八项按 §5.6.1 表序导出（字段名，值）：供校验、诊断与规范 SHA 使用。
    pub fn entries(&self) -> [(&'static str, u64); 8] {
        [
            (LIMIT_FIELDS[0], self.max_total_wire_bytes),
            (LIMIT_FIELDS[1], self.max_total_node_bytes),
            (LIMIT_FIELDS[2], self.max_total_nodes),
            (LIMIT_FIELDS[3], self.max_total_records),
            (LIMIT_FIELDS[4], self.max_total_requests),
            (LIMIT_FIELDS[5], self.max_elapsed_ms),
            (LIMIT_FIELDS[6], self.max_staged_bytes),
            (LIMIT_FIELDS[7], self.max_store_bytes),
        ]
    }
}

fn limit_field(object: &JsonMap<String, JsonNode>, key: &'static str) -> Result<u64, CodecError> {
    let value = object
        .get(key)
        .ok_or_else(|| CodecError::new(LIMIT_FIELD, format!("job_limits 缺字段 `{key}`")))?;
    let text = value.as_str().ok_or_else(|| {
        CodecError::new(
            LIMIT_FIELD,
            format!("`{key}` 必须是规范十进制字符串，不是 JSON 数字或其它类型"),
        )
    })?;
    parse_limit(text, key)
}

/// 规范十进制：`[1-9][0-9]*`，无 `+`、无前导零、无空白，≤ 2^63-1（§5.6.1 数域是 1 起的）。
fn parse_limit(text: &str, key: &str) -> Result<u64, CodecError> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || !bytes.iter().all(|b| b.is_ascii_digit()) {
        return Err(CodecError::new(
            LIMIT_DOMAIN,
            format!("`{key}` = `{text}` 不是规范十进制：只允许 0-9，无符号、无空白"),
        ));
    }
    if bytes[0] == b'0' {
        return Err(CodecError::new(
            LIMIT_DOMAIN,
            format!("`{key}` = `{text}` 有前导零或为 0：数域是 1..=2^63-1"),
        ));
    }
    let value = text
        .parse::<u64>()
        .map_err(|_| CodecError::new(LIMIT_DOMAIN, format!("`{key}` = `{text}` 超出 u64 表达")))?;
    if value > MAX_LIMIT {
        return Err(CodecError::new(
            LIMIT_DOMAIN,
            format!("`{key}` = `{text}` 超过上限 2^63-1"),
        ));
    }
    Ok(value)
}

/// 调用方实测的**既存分配基线**，[`Job::new`] 的必填输入。没有缺省、没有 `Default`：
/// 省略或写 0 都是在声明「本机无既存内容」，不是「未测量」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Existing {
    /// 本地内容存储总账既存量，含既存已提交内容、索引与日志，**不含本 job 自己的暂存**。
    pub store_bytes: u64,
    /// 本 job 之外已存在的暂存量（暂存是总存储的子集，故必须 ≤ `store_bytes`）。
    pub staged_bytes: u64,
}

/// 一次预留占哪本账：暂存是总存储的子集，不是两笔独立额度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpaceKind {
    /// [`Job::reserve_staged`]：暂存子集，**同时**占 `max_staged_bytes` 与 `max_store_bytes`。
    Staged,
    /// [`Job::reserve_store`]：非暂存的存储写入（如导入内容），只占 `max_store_bytes`。
    Store,
}

/// 一本账：`baseline`（声明时实测的既存量）+ `committed`（本 job 已对账）
/// + `pending`（已预留未对账）。
///
/// 内部量一律用 `u128`：这是**纯记账**（不是 wire 协议），上限／`actual`／`requested` 仍是 `u64`。
/// 写后实际量可能大于预留量（失败路径），真实 `actual` 与基线相加会越过 `u64`，用 `u64` 会溢出或低估。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Space {
    baseline: u128,
    committed: u128,
    pending: u128,
    peak: u128,
}

impl Space {
    fn record_peak(&mut self, projected: u128) {
        if projected > self.peak {
            self.peak = projected;
        }
    }
}

/// 一本账的对账快照。内部记账量用 `u128`（非 wire 协议），`limits`／入参仍是 `u64`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceSnapshot {
    /// [`Job::new`] 声明时实测的既存分配基线。
    pub baseline: u128,
    /// 本 job 写后按实际对账过、尚未被回收的字节数。
    pub committed: u128,
    /// 已预留、尚未 reconcile／release 的最坏量。
    pub pending: u128,
    /// 上限判据实际比较的那个数：暂存账是 `baseline+committed+pending`；
    /// 总账是 `baseline+committed+pending` **再加**暂存子集的 `committed+pending`（同一批字节占两本限额）。
    pub exposure: u128,
    /// `exposure` 的历史峰值（写前按最坏量计入，回收不回退）。
    pub peak: u128,
}

/// 一次预留的句柄。收尾口按 `&Grant` 收：错传别的 job 或重复调用都不会让 owner 丢掉句柄，
/// 而 open 表仍保证**每个 grant 最多关闭一次**（重复调用报 `job_reservation`）。
#[derive(Debug, PartialEq, Eq)]
pub struct Grant {
    job: u64,
    id: u64,
    kind: SpaceKind,
    requested: u64,
}

impl Grant {
    /// 该预留占的账：`Staged` 表示它同时占暂存与总存储两本限额。
    pub fn kind(&self) -> SpaceKind {
        self.kind
    }

    /// 写前预留的最坏量。
    pub fn requested(&self) -> u64 {
        self.requested
    }
}

/// 单调时钟缝：生产写 `Box::new(Instant::now)`，测试注入假时钟（本模块不 sleep）。
pub trait Clock {
    fn now(&self) -> Instant;
}

impl<F: Fn() -> Instant> Clock for F {
    fn now(&self) -> Instant {
        self()
    }
}

/// job 累计量的只读快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSnapshot {
    pub limits: JobLimits,
    /// 首次读取是否已开始（未开始则绝对期限尚未起算）。
    pub started: bool,
    /// 首次读取到本次取快照的单调时长。
    pub elapsed_ms: Option<u64>,
    /// 永久累计：实际读写字节（重传全计，取消／释放／回收都不归零）。
    /// 内部记账量用 `u128`（非 wire 协议，`limits` 仍是 `u64`）：越限那一次已经发生的
    /// 尾字节照实入账，累计可能越过 `u64`。
    pub wire_bytes: u128,
    /// 永久累计：已核不同根节点的分帧字节。
    pub node_bytes: u64,
    /// 永久累计：已核不同根节点的数量。
    pub nodes: u64,
    /// 永久累计：顶层记录访问次数。
    pub records: u64,
    /// 永久累计：请求／对端捕获尝试次数。
    pub requests: u64,
    /// 本 job 的暂存子集账。
    pub staged: SpaceSnapshot,
    /// 本地存储总账（`exposure` 已含暂存子集）。
    pub store: SpaceSnapshot,
    /// 尚未对账／释放的预留数量。
    pub open_reservations: u64,
    /// 首次超限的拒绝码；`Some` 即本 job 已停止，不再接纳新的 charge 与预留。
    pub stopped: Option<&'static str>,
}

/// 节点接纳结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeAcceptance {
    /// 首次接纳：已计入 `nodes` 与 `node_bytes`。
    FirstAccept,
    /// 同一根再次见到：只走期限检查，不重复计入。
    AlreadyAccepted,
}

/// 已由本次真实 `F(tag, body)` 核出的不可变节点计费信息。
///
/// 字段保持私有；除 [`CheckedNodeUsage::checked`] 与已核帧 [`CheckedFrame::usage`] 外没有
/// 构造入口，因此它只能来自当前字节原件的摘要核对。这个值可以由内部审计 receipt 重放到
/// 另一个 Job，但不代替该 Job 对内容、上下文或可达性的核验。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckedNodeUsage {
    digest: NodeDigest,
    framed_bytes: u64,
}

impl CheckedNodeUsage {
    pub fn checked(digest: &NodeDigest, tag: u8, body: &[u8]) -> Result<Self, CodecError> {
        let actual = frame(tag, body);
        if actual != *digest {
            return Err(CodecError::new(
                code::NODE_DIGEST,
                format!(
                    "节点 (tag, body) 与摘要不符：声称 {}，实算 {}；未核摘要不计入",
                    root_string(digest),
                    root_string(&actual)
                ),
            ));
        }
        Ok(Self {
            digest: *digest,
            framed_bytes: framed_bytes(body.len()),
        })
    }

    pub fn digest(&self) -> &NodeDigest {
        &self.digest
    }
}

/// 已核验的节点帧：`(tag, body)` 与**声称摘要**当场重算 `F(tag, body)` 相符的不可变原件。
///
/// 新原件经 [`CheckedFrame::checked`] 核验；逐字相同原件可受控重绑定证明。字段私有、不实现
/// `Deserialize`、不提供
/// `&mut body` 入口，也没有未核构造。它证明的是「帧与构造时那枚摘要相符」，**不**证明
/// 「帧正是调用方本次请求的那枚摘要」——请求侧仍须逐次核对 [`CheckedFrame::digest`]。
///
/// 它同时携带同一次核验导出的 [`CheckedNodeUsage`]，让记账层（[`Job::accept_checked_usage`]）
/// 直接重放已核用量，不必对同一份字节再算一次 `F`。
pub struct CheckedFrame {
    tag: u8,
    body: Vec<u8>,
    usage: CheckedNodeUsage,
}

/// Native 临时帧：正文先析构，分配许可随后归还；不改变原件/帧资格。
#[doc(hidden)]
pub struct ScopedFrame {
    frame: CheckedFrame,
    permit: Option<super::capture_allocation::scratch::NativeReservation>,
}
pub struct ScopedBody {
    body: Vec<u8>,
    _permit: Option<super::capture_allocation::scratch::NativeReservation>,
}
impl std::ops::Deref for ScopedBody {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.body
    }
}
pub enum ScopedNodeRead {
    Missing,
    Found { tag: u8, body: ScopedBody },
}
impl ScopedNodeRead {
    pub fn from_plain(
        read: NodeRead,
        permit: Option<super::capture_allocation::scratch::NativeReservation>,
    ) -> Self {
        match read {
            NodeRead::Missing => Self::Missing,
            NodeRead::Found { tag, body } => Self::Found {
                tag,
                body: ScopedBody {
                    body,
                    _permit: permit,
                },
            },
        }
    }
    pub fn into_plain(self) -> Result<NodeRead, CodecError> {
        match self {
            Self::Missing => Ok(NodeRead::Missing),
            Self::Found { tag, body } => Ok(NodeRead::Found {
                tag,
                body: body.into_plain()?,
            }),
        }
    }
    pub fn from_checked(frame: Option<ScopedFrame>) -> Self {
        match frame {
            None => Self::Missing,
            Some(frame) => {
                let (tag, body) = frame.into_body();
                Self::Found { tag, body }
            }
        }
    }
}
impl ScopedBody {
    pub fn check_with(
        self,
        f: impl FnOnce(Vec<u8>) -> Result<CheckedFrame, CodecError>,
    ) -> Result<ScopedFrame, CodecError> {
        let Self { body, _permit } = self;
        f(body).map(|frame| ScopedFrame::new(frame, _permit))
    }
    pub(super) fn into_plain(self) -> Result<Vec<u8>, CodecError> {
        if self._permit.is_some() {
            return Err(CodecError::scratch(
                super::capture_allocation::scratch::ScratchFailure::Ledger,
            ));
        }
        Ok(self.body)
    }
    /// #1488：正文与同一次读取的许可一起按值移出，供 owner 原件移交绑定；不复制、不释放。
    #[doc(hidden)]
    pub fn into_parts(
        self,
    ) -> (
        Vec<u8>,
        Option<super::capture_allocation::scratch::NativeReservation>,
    ) {
        (self.body, self._permit)
    }
}
impl ScopedFrame {
    /// 只借用正在持许可的帧，不转移正文或提前释放许可。
    pub fn tag(&self) -> u8 {
        self.frame.tag()
    }
    pub fn body(&self) -> &[u8] {
        self.frame.body()
    }

    pub fn new(
        frame: CheckedFrame,
        permit: Option<super::capture_allocation::scratch::NativeReservation>,
    ) -> Self {
        Self { frame, permit }
    }
    pub fn digest(&self) -> &NodeDigest {
        self.frame.digest()
    }
    /// 消费帧且保留从 checked_read_scoped 签发的原正文许可。
    pub fn into_native(self) -> super::capture_allocation::native_owned::NativeValue<CheckedFrame> {
        super::capture_allocation::native_owned::NativeValue::from_reserved(self.frame, self.permit)
    }
    pub fn usage(&self) -> &CheckedNodeUsage {
        self.frame.usage()
    }
    pub fn into_frame(self) -> Result<CheckedFrame, CodecError> {
        if self.permit.is_some() {
            return Err(CodecError::scratch(
                super::capture_allocation::scratch::ScratchFailure::Ledger,
            ));
        }
        Ok(self.frame)
    }
    pub(super) fn into_body(self) -> (u8, ScopedBody) {
        let (tag, body) = self.frame.into_parts();
        (
            tag,
            ScopedBody {
                body,
                _permit: self.permit,
            },
        )
    }
}

/// 逐字核对当前 owned Vec 后的结果；变化时原 Vec 退回调用方，尚未核验新帧。
pub enum SameMaterialRebind {
    /// 纯 F 证明已重绑定到本次传入的 Vec。
    Same(CheckedFrame),
    /// 摘要、tag 或正文不同；调用方须核验这里退回的当前 Vec。
    Changed(Vec<u8>),
}

impl CheckedFrame {
    /// 对新原件 `(tag, body)` 算一次 `F`，与声称摘要比较；不符即拒绝。
    pub fn checked(digest: &NodeDigest, tag: u8, body: Vec<u8>) -> Result<Self, CodecError> {
        let actual = frame(tag, &body);
        if actual != *digest {
            return Err(CodecError::new(
                code::NODE_DIGEST,
                format!(
                    "节点 (tag, body) 与摘要不符：声称 {}，实算 {}；未核摘要不成帧",
                    root_string(digest),
                    root_string(&actual)
                ),
            ));
        }
        let framed = framed_bytes(body.len());
        Ok(Self {
            tag,
            body,
            usage: CheckedNodeUsage {
                digest: *digest,
                framed_bytes: framed,
            },
        })
    }

    /// 仅当摘要、tag 与完整 body 逐字相同，才将纯 F 证明重绑定到当前 owned Vec。
    /// 只比较一次正文，不计算 F；变化时退回当前 Vec，供调用方驱逐旧证明后重新核验。
    /// 本方法不证明 SQL 来源、角色或业务准入。
    pub fn try_rebind_same_material(
        &self,
        digest: &NodeDigest,
        tag: u8,
        body: Vec<u8>,
    ) -> SameMaterialRebind {
        if self.digest() == digest && self.tag == tag && self.body == body {
            SameMaterialRebind::Same(Self {
                tag,
                body,
                usage: self.usage,
            })
        } else {
            SameMaterialRebind::Changed(body)
        }
    }

    /// 逐字相同则重绑定；任一不同均重新核验当前原件，保留原来的拒绝语义。
    pub fn checked_same_material(
        &self,
        digest: &NodeDigest,
        tag: u8,
        body: Vec<u8>,
    ) -> Result<Self, CodecError> {
        match self.try_rebind_same_material(digest, tag, body) {
            SameMaterialRebind::Same(frame) => Ok(frame),
            SameMaterialRebind::Changed(body) => Self::checked(digest, tag, body),
        }
    }

    /// 本帧核出相符的那枚摘要（不是「本次请求的摘要」；请求侧须自行对照）。
    pub fn digest(&self) -> &NodeDigest {
        &self.usage.digest
    }

    pub fn tag(&self) -> u8 {
        self.tag
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// 同一次核验导出的不可变计费值；重放它不代替对当前字节的核验。
    pub fn usage(&self) -> &CheckedNodeUsage {
        &self.usage
    }

    /// 取出已核原件（所有权转移；帧本身被消费，不留下可变入口）。
    pub fn into_parts(self) -> (u8, Vec<u8>) {
        (self.tag, self.body)
    }
}

impl fmt::Debug for CheckedFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 不打印 body 内容：帧可能很大，调试输出只给身份与体量。
        f.debug_struct("CheckedFrame")
            .field("digest", &root_string(&self.usage.digest))
            .field("tag", &self.tag)
            .field("body_len", &self.body.len())
            .finish()
    }
}

static JOB_IDS: AtomicU64 = AtomicU64::new(0);

/// 一个 job 的资源累计核心。
///
/// 一个 job 从用户开始重建或一次 Watch 调用开始，覆盖其全部同代 Snapshot 对拍；
/// 跨分页、片段、证明重传、同代对拍及重试累计，**同一个余额、同一个绝对期限**。
enum AcceptedNodes {
    Ordinary(BTreeSet<NodeDigest>),
    Mixed {
        binding: super::capture_allocation::scratch::NativeReservation,
        content: super::capture_allocation::native_tree::NativeTree<NodeDigest, u64>,
        legacy: super::capture_allocation::native_tree::NativeTree<NodeDigest, (i64, usize, u64)>,
    },
}

enum OpenReservations {
    Ordinary(BTreeMap<u64, (SpaceKind, u64)>),
    Mixed(super::capture_allocation::native_tree::NativeTree<u64, (SpaceKind, u64)>),
}
impl OpenReservations {
    fn len(&self) -> usize {
        match self {
            Self::Ordinary(m) => m.len(),
            Self::Mixed(m) => m.len(),
        }
    }
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn remove(&mut self, id: &u64) -> Option<(SpaceKind, u64)> {
        match self {
            Self::Ordinary(m) => m.remove(id),
            Self::Mixed(m) => m.remove(id),
        }
    }
    fn insert(
        &mut self,
        id: u64,
        entry: (SpaceKind, u64),
        work: Option<&super::capture_allocation::scratch::WorkScope>,
    ) -> Result<(), CodecError> {
        match self {
            Self::Ordinary(m) => {
                m.insert(id, entry);
            }
            Self::Mixed(m) => {
                let work =
                    work.ok_or_else(|| CodecError::new(RESERVATION, "mixed预留缺当前工作域"))?;
                m.insert_admitted(id, entry, &MixedAllocation(work))?;
            }
        }
        Ok(())
    }
}

pub struct Job {
    id: u64,
    limits: JobLimits,
    clock: Box<dyn Clock>,
    started: Option<Instant>,
    stopped: Option<(&'static str, Cow<'static, str>)>,
    wire_bytes: u128,
    node_bytes: u64,
    nodes: u64,
    records: u64,
    requests: u64,
    accepted: AcceptedNodes,
    staged: Space,
    store: Space,
    open: OpenReservations,
    next_grant: u64,
}

impl fmt::Debug for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Job")
            .field("id", &self.id)
            .field("wire_bytes", &self.wire_bytes)
            .field("nodes", &self.nodes)
            .field("node_bytes", &self.node_bytes)
            .field("records", &self.records)
            .field("requests", &self.requests)
            .field("open_reservations", &self.open.len())
            .field("stopped", &self.stopped.as_ref().map(|(c, _)| *c))
            .finish()
    }
}

impl Job {
    /// 新建 job：八项上限逐项校验（1..=2^63-1，无缺省），并按调用方**实测**的既存基线
    /// （[`Existing`]，必填、无缺省）当场核上限与子集关系：
    /// `staged_bytes ≤ store_bytes`、`staged_bytes ≤ max_staged_bytes`、`store_bytes ≤ max_store_bytes`。
    ///
    /// 基线在构造时即进账，所以纯读取 job（一次 `reserve` 都不发）也照旧受 `max_store_bytes` 约束。
    pub fn new(
        limits: JobLimits,
        existing: Existing,
        clock: Box<dyn Clock>,
    ) -> Result<Job, CodecError> {
        limits.validate()?;
        if existing.staged_bytes > existing.store_bytes {
            return Err(CodecError::new(
                LIMIT_DOMAIN,
                format!(
                    "暂存基线 {} 大于存储基线 {}：暂存是总存储的子集，基线也必须满足子集关系",
                    existing.staged_bytes, existing.store_bytes
                ),
            ));
        }
        if existing.staged_bytes > limits.max_staged_bytes {
            return Err(CodecError::new(
                LIMIT_EXCEEDED,
                format!(
                    "既存暂存 {} 已超 max_staged_bytes {}",
                    existing.staged_bytes, limits.max_staged_bytes
                ),
            ));
        }
        if existing.store_bytes > limits.max_store_bytes {
            return Err(CodecError::new(
                LIMIT_EXCEEDED,
                format!(
                    "既存存储 {} 已超 max_store_bytes {}",
                    existing.store_bytes, limits.max_store_bytes
                ),
            ));
        }
        Ok(Job {
            id: JOB_IDS.fetch_add(1, Ordering::Relaxed) + 1,
            limits,
            clock,
            started: None,
            stopped: None,
            wire_bytes: 0,
            node_bytes: 0,
            nodes: 0,
            records: 0,
            requests: 0,
            accepted: AcceptedNodes::Ordinary(BTreeSet::new()),
            staged: Space {
                // 构造时 exposure = baseline + 0 + 0，所以峰值就是实测基线本身。
                baseline: u128::from(existing.staged_bytes),
                peak: u128::from(existing.staged_bytes),
                ..Space::default()
            },
            store: Space {
                baseline: u128::from(existing.store_bytes),
                peak: u128::from(existing.store_bytes),
                ..Space::default()
            },
            open: OpenReservations::Ordinary(BTreeMap::new()),
            next_grant: 1,
        })
    }

    /// #1527：沿用真实控制原件首读时刻；不制造基线、不改变旧 new 行为。
    pub fn new_started(
        limits: JobLimits,
        existing: Existing,
        clock: Box<dyn Clock>,
        first_read_at: Instant,
    ) -> Result<Job, CodecError> {
        limits.validate()?;
        let now = clock.now();
        let deadline = first_read_at
            .checked_add(Duration::from_millis(limits.max_elapsed_ms))
            .ok_or_else(|| CodecError::new(DEADLINE, "首读绝对期限溢出"))?;
        if now < first_read_at || now >= deadline {
            return Err(CodecError::new(DEADLINE, "真实首读时刻无效或期限已到"));
        }
        let mut job = Self::new(limits, existing, clock)?;
        job.started = Some(first_read_at);
        job.check_deadline()?;
        Ok(job)
    }

    /// 首次读取开始：**幂等**，重复调用不会续期（§5.6.1「不能逐片续期」）。
    ///
    /// adapter 在第一个字节真正开始流动时调用；`charge_*`／`accept_node` 与读包装也会代为开始。
    pub fn begin_first_read(&mut self) -> Result<(), CodecError> {
        self.gate()?;
        if self.started.is_none() {
            self.started = Some(self.clock.now());
        }
        Ok(())
    }

    /// 只读期限门：已停止或已过绝对期限即 `Err`。adapter 可在排队、分片边界显式调用。
    pub fn check_deadline(&self) -> Result<(), CodecError> {
        if let Some(err) = self.stopped_err() {
            return Err(err);
        }
        self.deadline_gate()
    }

    /// 首次读取的绝对截止 `Instant`（§5.6.1 `max_elapsed_ms`）：**gate + 起钟一次**，不续期。
    ///
    /// adapter 用它和旧每操作期限取更早者，两个限制同时生效。拒绝语义：
    /// 已停止的 job 原样报停止原因；期限已过就地停止并报 `job_deadline`；
    /// `started + max_elapsed_ms` 在单调时钟域溢出（极端 `Instant`）时同样停定并拒绝，
    /// 不用饱和值冒充可用期限。
    pub fn first_read_deadline(&mut self) -> Result<Instant, CodecError> {
        self.gate()?;
        if self.started.is_none() {
            self.started = Some(self.clock.now());
        }
        let started = self.started.expect("上面已保证起钟");
        match started.checked_add(Duration::from_millis(self.limits.max_elapsed_ms)) {
            Some(deadline) => Ok(deadline),
            None => Err(self.stop(
                DEADLINE,
                self.stop_detail(format_args!(
                    "起始 Instant 加上 max_elapsed_ms {} ms 溢出单调时钟域：无法给出绝对期限",
                    self.limits.max_elapsed_ms
                )),
            )),
        }
    }

    /// 每次**真正 I/O 之前**的门：已停止原样报停止原因；绝对期限已过则就地停止并报 `job_deadline`。
    ///
    /// 通过本口只说明「此刻尚未停止、尚未过期」，不是无限开工许可：I/O 结束后仍必须按实际字节
    /// [`Job::charge_wire_bytes`]；停止后本口即拒绝后续 I/O。
    pub fn pre_io_check(&mut self) -> Result<(), CodecError> {
        self.gate()
    }

    /// 实际 wire 字节的入账口（#1394 C §5.6.1）：**先照实记录，再核上限与绝对期限**。
    ///
    /// 一次真实读/写已经发生的物理字节必须先进累计账，然后才判是否越限：
    /// - 越过 `max_total_wire_bytes`：停止本 job 并报 `job_limit_exceeded`，**本次字节已入账**；
    /// - 已过绝对期限：同样先入账，再以 `job_deadline` 停止（半开区间：必须在期限前收尾）。
    ///
    /// 已停止的 job 不再接纳任何新字节——本口按停止原因拒绝，**不**把入账当成重新开工的许可。
    /// 首次调用即起绝对期限的钟（等价于 [`Job::begin_first_read`]），后续调用只检查不续期。
    pub fn charge_wire_bytes(&mut self, actual_bytes: u64) -> Result<(), CodecError> {
        if let Some(err) = self.stopped_err() {
            return Err(err);
        }
        if self.started.is_none() {
            self.started = Some(self.clock.now());
        }
        // 物理字节已经发生：先照实入账，再核上限/期限。
        self.wire_bytes = match self.wire_bytes.checked_add(u128::from(actual_bytes)) {
            Some(total) => total,
            None => {
                self.wire_bytes = u128::MAX;
                return Err(self.stop(
                    LIMIT_EXCEEDED,
                    self.stop_detail(format_args!("{} 累计溢出内部 u128 记账域", LIMIT_FIELDS[0])),
                ));
            }
        };
        if self.wire_bytes > u128::from(self.limits.max_total_wire_bytes) {
            return Err(self.stop(
                LIMIT_EXCEEDED,
                self.stop_detail(format_args!(
                    "{} 累计 {} 超过上限 {}（实际字节已先入账再停止）",
                    LIMIT_FIELDS[0], self.wire_bytes, self.limits.max_total_wire_bytes
                )),
            ));
        }
        if let Err(err) = self.deadline_gate() {
            return Err(self.stop(DEADLINE, self.stop_detail(format_args!("{}", err.detail()))));
        }
        Ok(())
    }

    /// 显式 charge：顶层记录访问次数（同一记录重访照实再计）。首次调用即起钟，不续期。
    pub fn charge_records(&mut self, accesses: u64) -> Result<(), CodecError> {
        self.begin_first_read()?;
        let cap = self.limits.max_total_records;
        let projected = self.add_cumulative(LIMIT_FIELDS[3], cap, self.records, accesses)?;
        self.records = projected;
        Ok(())
    }

    /// 显式 charge：本 job 发起的请求／对端捕获尝试次数（含错误与重试）。首次调用即起钟，不续期。
    pub fn charge_requests(&mut self, attempts: u64) -> Result<(), CodecError> {
        self.begin_first_read()?;
        let cap = self.limits.max_total_requests;
        let projected = self.add_cumulative(LIMIT_FIELDS[4], cap, self.requests, attempts)?;
        self.requests = projected;
        Ok(())
    }

    /// 接纳一个节点（来源可以是本地复用、对端 proof 流或本地写入流）。
    ///
    /// 首次调用即起绝对期限的钟（不调用 [`Job::begin_first_read`] 也不会漏掉期限）。
    /// 先重算 `F(tag, body)` 与 `digest` 对照：不符即 `node_digest` 拒绝，且**不**计入；
    /// 核对通过后同一根只计一次 `nodes`／`node_bytes`；重复调用仍照走期限与停止门。
    ///
    /// 写入面按**待写实际节点**先调用本口核 hash 与节点预算，再落盘；节点账不做落盘回滚承诺
    /// （底层 sink 不是事务），实际 wire 与容量分别走显式 charge 与 `reserve_*`／`reconcile`。
    pub fn accept_node(
        &mut self,
        digest: &NodeDigest,
        tag: u8,
        body: &[u8],
    ) -> Result<NodeAcceptance, CodecError> {
        self.begin_first_read()?;
        // 先核 F(tag, body) 再动任何账：未核摘要绝不进入已计费集合，
        // 否则坏节点重复出现会靠「已计过」绕开计费与校验。
        let usage = CheckedNodeUsage::checked(digest, tag, body)?;
        self.accept_checked_usage_inner(&usage)
    }

    /// 同一次读取的已核帧：保持 `accept_node` 的起钟/门 → F → 入账顺序，
    /// 并把原 Vec 连同证明交给消费端。`accept_checked_usage` 的重放门不用于此处，
    /// 否则会在 F 后额外检查一次期限，改变旧读取的接纳结果。
    pub fn accept_frame(
        &mut self,
        digest: &NodeDigest,
        tag: u8,
        body: Vec<u8>,
    ) -> Result<CheckedFrame, CodecError> {
        self.begin_first_read()?;
        let checked = CheckedFrame::checked(digest, tag, body)?;
        self.accept_checked_usage_inner(checked.usage())?;
        Ok(checked)
    }

    /// 重放一个已经由 [`CheckedNodeUsage::checked`] 从真实原件核出的不可变计费值。
    ///
    /// 这里仍执行本 Job 的期限、停止状态、去重和节点预算门；它不重新声称核验了
    /// 当前内容图；它不接收或核对本次节点字节。调用方必须另外证明复用原件及判据上下文
    /// 未变；有新字节时仍须调用 `accept_node`，不能拿旧用量票据替代当前摘要核对。
    pub fn accept_checked_usage(
        &mut self,
        usage: &CheckedNodeUsage,
    ) -> Result<NodeAcceptance, CodecError> {
        self.begin_first_read()?;
        self.accept_checked_usage_inner(usage)
    }

    fn accept_checked_usage_inner(
        &mut self,
        usage: &CheckedNodeUsage,
    ) -> Result<NodeAcceptance, CodecError> {
        if !matches!(self.accepted, AcceptedNodes::Ordinary(_)) {
            return Err(CodecError::new(
                RESERVATION,
                "mixed节点接纳必须携带当前WorkScope",
            ));
        }
        let digest = usage.digest;
        if matches!(&self.accepted, AcceptedNodes::Ordinary(set) if set.contains(&digest)) {
            return Ok(NodeAcceptance::AlreadyAccepted);
        }
        let framed = usage.framed_bytes;
        let projected_bytes = match self.node_bytes.checked_add(framed) {
            Some(n) => n,
            None => {
                return Err(self.stop(
                    LIMIT_EXCEEDED,
                    self.stop_detail(format_args!("{} 累计溢出 u64", LIMIT_FIELDS[1])),
                ))
            }
        };
        let projected_nodes = match self.nodes.checked_add(1) {
            Some(n) => n,
            None => {
                return Err(self.stop(
                    LIMIT_EXCEEDED,
                    self.stop_detail(format_args!("{} 累计溢出 u64", LIMIT_FIELDS[2])),
                ))
            }
        };
        if projected_bytes > self.limits.max_total_node_bytes {
            return Err(self.stop(
                LIMIT_EXCEEDED,
                self.stop_detail(format_args!(
                    "{} 累计 {projected_bytes} 超过上限 {}",
                    LIMIT_FIELDS[1], self.limits.max_total_node_bytes
                )),
            ));
        }
        if projected_nodes > self.limits.max_total_nodes {
            return Err(self.stop(
                LIMIT_EXCEEDED,
                self.stop_detail(format_args!(
                    "{} 累计 {projected_nodes} 超过上限 {}",
                    LIMIT_FIELDS[2], self.limits.max_total_nodes
                )),
            ));
        }
        self.node_bytes = projected_bytes;
        self.nodes = projected_nodes;
        let AcceptedNodes::Ordinary(set) = &mut self.accepted else {
            unreachable!()
        };
        set.insert(digest);
        Ok(NodeAcceptance::FirstAccept)
    }

    /// 原 Job 当前独占的 mixed 节点索引与开放预留许可，不含缓存。
    pub fn mixed_tracking_native_bytes(&self) -> Result<usize, CodecError> {
        use super::capture_allocation::scratch::ScratchFailure;
        let AcceptedNodes::Mixed { binding, content, legacy } = &self.accepted else {
            return Err(CodecError::scratch(ScratchFailure::Ledger));
        };
        let OpenReservations::Mixed(open) = &self.open else {
            return Err(CodecError::scratch(ScratchFailure::Ledger));
        };
        [
            binding.bytes(),
            content.native_permit_bytes()?,
            legacy.native_permit_bytes()?,
            open.native_permit_bytes()?,
        ].into_iter().try_fold(0usize, |total, bytes| {
            total.checked_add(bytes)
                .ok_or_else(|| CodecError::scratch(ScratchFailure::Ledger))
        })
    }
    /// #1527：具名mixed构造器；两域集合共同受当前父工作域约束。
    pub fn new_started_mixed(
        limits: JobLimits,
        existing: Existing,
        clock: Box<dyn Clock>,
        first: Instant,
        work: &super::capture_allocation::scratch::WorkScope,
    ) -> Result<Self, CodecError> {
        work.check_current()?;
        // Mixed拒绝在构造任何动态诊断前完成；普通构造器保留原详细文本。
        if limits
            .entries()
            .iter()
            .any(|(_, value)| *value == 0 || *value > MAX_LIMIT)
        {
            return Err(CodecError::static_detail(
                LIMIT_DOMAIN,
                "mixed Job限额不在合法数域",
            ));
        }
        if existing.staged_bytes > existing.store_bytes {
            return Err(CodecError::static_detail(
                LIMIT_DOMAIN,
                "mixed暂存基线超过存储基线",
            ));
        }
        if existing.staged_bytes > limits.max_staged_bytes
            || existing.store_bytes > limits.max_store_bytes
        {
            return Err(CodecError::static_detail(
                LIMIT_EXCEEDED,
                "mixed既存基线超过Job帽",
            ));
        }
        let now = clock.now();
        let deadline = first
            .checked_add(Duration::from_millis(limits.max_elapsed_ms))
            .ok_or_else(|| CodecError::static_detail(DEADLINE, "mixed首读期限溢出"))?;
        if now < first || now >= deadline {
            return Err(CodecError::static_detail(
                DEADLINE,
                "mixed首读时刻无效或期限已到",
            ));
        }
        // 上述已逐项验证new的所有拒绝条件；其成功路径只建空容器。
        let mut job = Self::new(limits, existing, clock)?;
        job.started = Some(first);
        job.accepted = AcceptedNodes::Mixed {
            binding: work.reserve_native(0)?,
            content: Default::default(),
            legacy: Default::default(),
        };
        job.open = OpenReservations::Mixed(Default::default());
        job.check_deadline()?;
        Ok(job)
    }
    pub fn accept_checked_usage_mixed(
        &mut self,
        usage: &CheckedNodeUsage,
        work: &super::capture_allocation::scratch::WorkScope,
    ) -> Result<NodeAcceptance, CodecError> {
        self.begin_first_read()?;
        work.check_current()?;
        let AcceptedNodes::Mixed {
            content, binding, ..
        } = &self.accepted
        else {
            return Err(CodecError::static_detail(
                RESERVATION,
                "普通Job不接受mixed节点",
            ));
        };
        if !binding.belongs_to_parent(work) {
            return Err(work.reject(super::capture_allocation::scratch::ScratchFailure::Ledger));
        }
        if let Some(bytes) = content.get(&usage.digest) {
            if *bytes != usage.framed_bytes {
                return Err(CodecError::static_detail(
                    RESERVATION,
                    "同节点原件费用不一致",
                ));
            }
            return Ok(NodeAcceptance::AlreadyAccepted);
        }
        let (nodes, bytes) = self.project_mixed_node(usage.framed_bytes)?;
        let AcceptedNodes::Mixed { content, .. } = &mut self.accepted else {
            unreachable!()
        };
        content.insert_admitted(usage.digest, usage.framed_bytes, &MixedAllocation(work))?;
        self.nodes = nodes;
        self.node_bytes = bytes;
        Ok(NodeAcceptance::FirstAccept)
    }
    pub fn accept_legacy_evidence(
        &mut self,
        row: &super::legacy_evidence::CheckedLegacyEvidenceRow,
        work: &super::capture_allocation::scratch::WorkScope,
    ) -> Result<NodeAcceptance, CodecError> {
        self.begin_first_read()?;
        work.check_current()?;
        let identity = (row.kind(), row.payload_len(), row.declared_length());
        let AcceptedNodes::Mixed {
            legacy, binding, ..
        } = &self.accepted
        else {
            return Err(CodecError::static_detail(
                RESERVATION,
                "普通Job不接受legacy域",
            ));
        };
        if !binding.belongs_to_parent(work) {
            return Err(work.reject(super::capture_allocation::scratch::ScratchFailure::Ledger));
        }
        if let Some(previous) = legacy.get(row.id()) {
            if *previous != identity {
                return Err(CodecError::static_detail(
                    RESERVATION,
                    "同legacy节点原件不一致",
                ));
            }
            return Ok(NodeAcceptance::AlreadyAccepted);
        }
        let framed = u64::try_from(row.payload_len())
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or_else(|| self.stop(LIMIT_EXCEEDED, "legacy节点费用溢出".into()))?;
        let (nodes, bytes) = self.project_mixed_node(framed)?;
        let AcceptedNodes::Mixed { legacy, .. } = &mut self.accepted else {
            unreachable!()
        };
        legacy.insert_admitted(*row.id(), identity, &MixedAllocation(work))?;
        self.nodes = nodes;
        self.node_bytes = bytes;
        Ok(NodeAcceptance::FirstAccept)
    }
    fn project_mixed_node(&mut self, framed: u64) -> Result<(u64, u64), CodecError> {
        let bytes = self
            .node_bytes
            .checked_add(framed)
            .ok_or_else(|| self.stop(LIMIT_EXCEEDED, "节点字节累计溢出".into()))?;
        let nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| self.stop(LIMIT_EXCEEDED, "节点数量累计溢出".into()))?;
        if bytes > self.limits.max_total_node_bytes || nodes > self.limits.max_total_nodes {
            return Err(self.stop(LIMIT_EXCEEDED, "mixed两域聚合节点费用超出签署帽".into()));
        }
        Ok((nodes, bytes))
    }

    fn check_reservation_scope(
        &self,
        work: Option<&super::capture_allocation::scratch::WorkScope>,
    ) -> Result<(), CodecError> {
        match (&self.accepted, work) {
            (AcceptedNodes::Ordinary(_), None) => Ok(()),
            (AcceptedNodes::Mixed { binding, .. }, Some(work)) => {
                work.check_current()?;
                if !binding.belongs_to_parent(work) {
                    return Err(
                        work.reject(super::capture_allocation::scratch::ScratchFailure::Ledger)
                    );
                }
                Ok(())
            }
            _ => Err(CodecError::new(RESERVATION, "预留入口与Job计量域不匹配")),
        }
    }

    /// 写前预留（暂存子集）：同一批字节**同时**占 `max_staged_bytes` 与 `max_store_bytes`，
    /// 一次调用核两本 cap；任一超限即 `job_limit_exceeded` 停止，两本账都不留部分计费状态。
    ///
    /// 峰值按最坏量立即计入两本账；随后由 [`Job::reconcile`] 按实际量对账，
    /// 或由 [`Job::release`] 在取消／失败时退出预留。两处基线都在 [`Job::new`] 一次声明、
    /// 本接口不再接受新的既存量，所以删文件腾挪额度这条通道（§5.6.1）在本层不成立。
    pub fn reserve_staged(&mut self, requested: u64) -> Result<Grant, CodecError> {
        self.reserve_staged_in(requested, None)
    }
    pub fn reserve_staged_mixed(
        &mut self,
        requested: u64,
        work: &super::capture_allocation::scratch::WorkScope,
    ) -> Result<Grant, CodecError> {
        self.reserve_staged_in(requested, Some(work))
    }
    fn reserve_staged_in(
        &mut self,
        requested: u64,
        work: Option<&super::capture_allocation::scratch::WorkScope>,
    ) -> Result<Grant, CodecError> {
        self.gate()?;
        self.check_reservation_scope(work)?;
        let staged_projected = self.project(
            "暂存",
            self.staged_exposure(),
            requested,
            self.limits.max_staged_bytes,
        )?;
        let store_projected = self.project(
            "存储",
            self.store_exposure(),
            requested,
            self.limits.max_store_bytes,
        )?;
        let id = self.next_grant;
        self.open.insert(id, (SpaceKind::Staged, requested), work)?;
        self.staged.pending += u128::from(requested);
        self.staged.record_peak(staged_projected);
        self.store.record_peak(store_projected);
        self.next_grant += 1;
        Ok(Grant {
            job: self.id,
            id,
            kind: SpaceKind::Staged,
            requested,
        })
    }

    /// 写前预留（非暂存的存储写入）：只占 `max_store_bytes`，不占暂存账。
    ///
    /// 只给**会保留**的本地存储写入用（索引、日志等）：这类 `committed` 不能因为 job 失败就被删，
    /// 所以本模块不为它开回收口（不为历史内容开任意删除通道）。可能被取消的导入内容走
    /// [`Job::reserve_staged`]，那条路有 [`Job::reclaim_staged`] 按实际删除量回收。
    pub fn reserve_store(&mut self, requested: u64) -> Result<Grant, CodecError> {
        self.reserve_store_in(requested, None)
    }
    pub fn reserve_store_mixed(
        &mut self,
        requested: u64,
        work: &super::capture_allocation::scratch::WorkScope,
    ) -> Result<Grant, CodecError> {
        self.reserve_store_in(requested, Some(work))
    }
    fn reserve_store_in(
        &mut self,
        requested: u64,
        work: Option<&super::capture_allocation::scratch::WorkScope>,
    ) -> Result<Grant, CodecError> {
        self.gate()?;
        self.check_reservation_scope(work)?;
        let store_projected = self.project(
            "存储",
            self.store_exposure(),
            requested,
            self.limits.max_store_bytes,
        )?;
        let id = self.next_grant;
        self.open.insert(id, (SpaceKind::Store, requested), work)?;
        self.store.pending += u128::from(requested);
        self.store.record_peak(store_projected);
        self.next_grant += 1;
        Ok(Grant {
            job: self.id,
            id,
            kind: SpaceKind::Store,
            requested,
        })
    }

    /// 写后按实际对账：**先确认本 job 的合法未决 grant**，再把 `pending` 的原预留换成实际
    /// `committed`（两本账与峰值都更新），最后把未用掉的差额还回额度。
    ///
    /// `Staged` 预留同时更新两本账（暂存子集与总账）；`Store` 预留只更新总账。
    ///
    /// 实际量超过预留量时**照实入账**——真实分配不会因为失败就消失，账面也不许低估——随后
    /// `job_reconcile` 停止并报错：该 job 不再借出，确已删除的暂存由 [`Job::reclaim_staged`]
    /// 按实际删除字节回收。该 grant 此时已闭合，重复对账报 `job_reservation`。
    pub fn reconcile(&mut self, grant: &Grant, actual: u64) -> Result<(), CodecError> {
        if grant.job != self.id {
            return Err(CodecError::new(
                RESERVATION,
                "预留句柄不属于本 job：跨 job 对账会让两本账失真",
            ));
        }
        let (kind, requested) = match self.open.remove(&grant.id) {
            Some(entry) => entry,
            None => {
                return Err(CodecError::new(
                    RESERVATION,
                    format!("预留 {} 已对账或已释放", grant.id),
                ))
            }
        };
        match kind {
            SpaceKind::Staged => {
                self.staged.pending -= u128::from(requested);
                self.staged.committed += u128::from(actual);
            }
            SpaceKind::Store => {
                self.store.pending -= u128::from(requested);
                self.store.committed += u128::from(actual);
            }
        }
        self.refresh_peaks();
        if actual > requested {
            return Err(self.stop(
                RECONCILE,
                self.stop_detail(format_args!(
                    "{kind:?} 写后实际 {actual} 字节超过预留 {requested} 字节：不足即停止；\
                     真实 actual 已按实际入账，确已删除的暂存用 reclaim_staged 回收"
                )),
            ));
        }
        Ok(())
    }

    /// 取消／失败时释放**未决**预留：只退出 `pending`，**不动** `committed` 与任何累计账。
    ///
    /// 已经 `reconcile` 过的暂存走 [`Job::reclaim_staged`]；两者都不归零
    /// `wire_bytes`／`nodes`／`node_bytes`／`records`／`requests`。
    /// 句柄按 `&Grant` 收：重复释放或错传别的 job 都报 `job_reservation`，不会误关别人的预留。
    pub fn release(&mut self, grant: &Grant) -> Result<(), CodecError> {
        if grant.job != self.id {
            return Err(CodecError::new(RESERVATION, "预留句柄不属于本 job"));
        }
        let (kind, requested) = match self.open.remove(&grant.id) {
            Some(entry) => entry,
            None => {
                return Err(CodecError::new(
                    RESERVATION,
                    format!("预留 {} 已对账或已释放", grant.id),
                ))
            }
        };
        match kind {
            SpaceKind::Staged => self.staged.pending -= u128::from(requested),
            SpaceKind::Store => self.store.pending -= u128::from(requested),
        }
        Ok(())
    }

    /// 取消／失败后**按实际删除量**回收本 job 已对账的暂存：`actual_deleted` 必须 ≤
    /// 本 job 的 `staged.committed`（调用方实测的删除量）。
    ///
    /// 这也是 `reconcile` 超限停止后的清理通道：那条路已按真实 actual 入账，确已删除的部分在此回收。
    /// 只减暂存 `committed`（总账因含暂存子集同步下降），峰值不回退，累计
    /// `wire_bytes`／`nodes`／`node_bytes`／`records`／`requests` 一律照旧。
    /// 本入口**只**面向本 job 自己写出的暂存，不提供对已提交历史内容的任意删除。
    pub fn reclaim_staged(&mut self, actual_deleted: u64) -> Result<(), CodecError> {
        if u128::from(actual_deleted) > self.staged.committed {
            return Err(CodecError::new(
                RECLAIM,
                format!(
                    "回收 {actual_deleted} 字节超过本 job 已对账暂存 {} 字节：历史内容不在本入口管辖内",
                    self.staged.committed
                ),
            ));
        }
        self.staged.committed -= u128::from(actual_deleted);
        Ok(())
    }

    /// 安装／经济提交前的收尾检查：未停止、无未对账预留、绝对期限未过。
    ///
    /// 未对账预留是调用方次序错误（不影响账面），只报 `job_reservation` 不停止；
    /// 期限已过则本来就是超限，停止并报 `job_deadline`。
    pub fn finish_check(&mut self) -> Result<(), CodecError> {
        if let Some(err) = self.stopped_err() {
            return Err(err);
        }
        if !self.open.is_empty() {
            return Err(CodecError::new(
                RESERVATION,
                format!(
                    "尚有 {} 个预留未对账：写后必须 reconcile 或 release",
                    self.open.len()
                ),
            ));
        }
        self.gate()
    }

    /// 当前累计量快照（只读；不影响任何额度）。
    pub fn snapshot(&self) -> JobSnapshot {
        let elapsed = self.elapsed();
        JobSnapshot {
            limits: self.limits,
            started: self.started.is_some(),
            elapsed_ms: elapsed.map(|d| {
                let ms = d.as_millis();
                if ms > u128::from(u64::MAX) {
                    u64::MAX
                } else {
                    ms as u64
                }
            }),
            wire_bytes: self.wire_bytes,
            node_bytes: self.node_bytes,
            nodes: self.nodes,
            records: self.records,
            requests: self.requests,
            staged: self.staged_snapshot(),
            store: self.store_snapshot(),
            open_reservations: self.open.len() as u64,
            stopped: self.stopped.as_ref().map(|(code, _)| *code),
        }
    }

    /// 暂存账的上限判据数：`baseline + committed + pending`。
    fn staged_exposure(&self) -> u128 {
        self.staged.baseline + self.staged.committed + self.staged.pending
    }

    /// 总账的上限判据数：暂存子集在里面只出现一次（`store.baseline` 按契约不含本 job 暂存）。
    fn store_exposure(&self) -> u128 {
        self.store.baseline
            + self.store.committed
            + self.store.pending
            + self.staged.committed
            + self.staged.pending
    }

    fn staged_snapshot(&self) -> SpaceSnapshot {
        SpaceSnapshot {
            baseline: self.staged.baseline,
            committed: self.staged.committed,
            pending: self.staged.pending,
            exposure: self.staged_exposure(),
            peak: self.staged.peak,
        }
    }

    fn store_snapshot(&self) -> SpaceSnapshot {
        SpaceSnapshot {
            baseline: self.store.baseline,
            committed: self.store.committed,
            pending: self.store.pending,
            exposure: self.store_exposure(),
            peak: self.store.peak,
        }
    }

    fn refresh_peaks(&mut self) {
        let staged = self.staged_exposure();
        let store = self.store_exposure();
        self.staged.record_peak(staged);
        self.store.record_peak(store);
    }

    fn elapsed(&self) -> Option<Duration> {
        self.started
            .map(|started| self.clock.now().saturating_duration_since(started))
    }

    /// 预留后的峰值：先判既有敞口，再判加上本次预留（两本账各自判，任一处失败即整体失败）。
    fn project(
        &mut self,
        name: &str,
        exposure: u128,
        requested: u64,
        cap: u64,
    ) -> Result<u128, CodecError> {
        let cap = u128::from(cap);
        if exposure > cap {
            return Err(self.stop(
                LIMIT_EXCEEDED,
                self.stop_detail(format_args!("{name}既有敞口 {exposure} 已超上限 {cap}")),
            ));
        }
        // exposure ≤ cap ≤ 2^63-1、requested ≤ 2^64-1 ⟹ 和 < 2^65，u128 不可能溢出。
        let projected = exposure + u128::from(requested);
        if projected > cap {
            return Err(self.stop(
                LIMIT_EXCEEDED,
                self.stop_detail(format_args!(
                    "{name}预留后峰值 {projected} 超过上限 {cap}（既有敞口 {exposure}）"
                )),
            ));
        }
        Ok(projected)
    }

    fn add_cumulative(
        &mut self,
        name: &'static str,
        cap: u64,
        current: u64,
        add: u64,
    ) -> Result<u64, CodecError> {
        if add == 0 {
            return Ok(current);
        }
        let projected = match current.checked_add(add) {
            Some(n) => n,
            None => {
                return Err(self.stop(
                    LIMIT_EXCEEDED,
                    self.stop_detail(format_args!("{name} 累计溢出 u64")),
                ));
            }
        };
        if projected > cap {
            return Err(self.stop(
                LIMIT_EXCEEDED,
                self.stop_detail(format_args!(
                    "{name} 本次 +{add} 后累计 {projected} 超过上限 {cap}"
                )),
            ));
        }
        Ok(projected)
    }

    /// #1527：mixed 错误保留静态细节、原 code 和累计账；不渲染动态格式参数。
    fn stop_detail(&self, detail: fmt::Arguments<'_>) -> Cow<'static, str> {
        if matches!(self.accepted, AcceptedNodes::Mixed { .. }) {
            Cow::Borrowed("mixed Job resource rejection; see code and snapshot counters")
        } else {
            Cow::Owned(detail.to_string())
        }
    }

    fn error_detail(code: &'static str, detail: Cow<'static, str>) -> CodecError {
        match detail {
            Cow::Borrowed(text) => CodecError::static_detail(code, text),
            Cow::Owned(text) => CodecError::new(code, text),
        }
    }

    fn stopped_err(&self) -> Option<CodecError> {
        self.stopped.as_ref().map(|(code, detail)| {
            Self::error_detail(
                code,
                self.stop_detail(format_args!("job 已停止（{code}）：{detail}")),
            )
        })
    }

    fn deadline_gate(&self) -> Result<(), CodecError> {
        let Some(elapsed) = self.elapsed() else {
            return Ok(());
        };
        let limit = Duration::from_millis(self.limits.max_elapsed_ms);
        if elapsed >= limit {
            return Err(Self::error_detail(
                DEADLINE,
                self.stop_detail(format_args!(
                    "已过绝对期限 {} ms（半开区间：必须在期限前收尾）",
                    self.limits.max_elapsed_ms
                )),
            ));
        }
        Ok(())
    }

    /// 变更入口的统一门：已停止即原样报停止原因；期限已过则**就此停止**。
    fn gate(&mut self) -> Result<(), CodecError> {
        if let Some(err) = self.stopped_err() {
            return Err(err);
        }
        if let Err(err) = self.deadline_gate() {
            return Err(self.stop(DEADLINE, self.stop_detail(format_args!("{}", err.detail()))));
        }
        Ok(())
    }

    /// 首次超限即停止：停止不是清账，`snapshot` 仍照实显示全部累计量。
    fn stop(&mut self, code: &'static str, detail: Cow<'static, str>) -> CodecError {
        if self.stopped.is_none() {
            self.stopped = Some((code, detail.clone()));
        }
        Self::error_detail(code, detail)
    }
}

/// §5.6.1 的节点分帧字节：`len(D) + 1 + 8 + len(body)`（不按展开值重复记）。
fn framed_bytes(body_len: usize) -> u64 {
    DOMAIN_SEPARATOR.len() as u64 + 1 + 8 + body_len as u64
}

/// 借同一个 [`Job`] 的 [`NodeSource`] 包装：每次读前／读后检查绝对期限，
/// 按**已核 frame** 计不同节点（重复 read 不新增节点计数，但仍检查时间）。
///
/// 边界（照实）：
/// - 本包装看不到实际传输字节，`max_total_wire_bytes` 与 `max_total_requests` 仍由 adapter 显式 charge。
/// - 底层 `read` 已完整返回 body 之后才计数，故本包装**不是**底层读取内存预分配的上限。
/// - `F(tag, body)` 与摘要不符的节点按 `node_digest` 拒绝，且不进入已计费集合。
///
/// 本模块**不提供 `NodeSink` 包装**（也不恢复它）：底层 sink 的写入不是事务，包装无法既在写前核完
/// 摘要与节点预算、又在写后保证不落盘。写入面由调用方按**待写实际节点**先在写前核
/// `F(tag, body)` 与节点预算（[`Job::accept_node`]），再落盘；节点账不承诺回滚，实际 wire 与容量
/// 分别走事后显式 charge 与 `reserve_*`／`reconcile`。
pub struct JobNodeSource<'a> {
    job: &'a mut Job,
    inner: &'a mut dyn NodeSource,
}

impl<'a> JobNodeSource<'a> {
    pub fn new(job: &'a mut Job, inner: &'a mut dyn NodeSource) -> Self {
        Self { job, inner }
    }
}

impl NodeSource for JobNodeSource<'_> {
    fn codec_work(&self) -> Option<&super::capture_allocation::scratch::WorkScope> {
        self.inner.codec_work()
    }
    fn max_render_bytes(&self) -> Option<u64> {
        self.inner.max_render_bytes()
    }
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.read_scoped(digest)?.into_plain()
    }
    fn read_scoped(&mut self, digest: &NodeDigest) -> Result<ScopedNodeRead, CodecError> {
        self.job.begin_first_read()?;
        let read = self.inner.read_scoped(digest)?;
        self.job.check_deadline()?;
        if let ScopedNodeRead::Found { tag, body } = &read {
            self.job.accept_node(digest, *tag, body)?;
        }
        Ok(read)
    }
    fn checked_read(&mut self, digest: &NodeDigest) -> Result<Option<CheckedFrame>, CodecError> {
        self.checked_read_scoped(digest)?
            .map(ScopedFrame::into_frame)
            .transpose()
    }
    fn checked_read_scoped(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<Option<ScopedFrame>, CodecError> {
        self.job.begin_first_read()?;
        let read = self.inner.read_scoped(digest)?;
        self.job.check_deadline()?;
        match read {
            ScopedNodeRead::Missing => Ok(None),
            ScopedNodeRead::Found { tag, body } => body
                .check_with(|body| self.job.accept_frame(digest, tag, body))
                .map(Some),
        }
    }
}

#[cfg(test)]
mod same_material_tests {
    use super::*;

    #[test]
    fn try_rebind_same_material_moves_current_vec_or_returns_it_without_hashing() {
        let tag = super::super::tag::BYTES_CHUNK;
        let digest = frame(tag, &[1, 2, 3]);
        let original = CheckedFrame::checked(&digest, tag, vec![1, 2, 3]).unwrap();
        let current = vec![1, 2, 3];
        let ptr = current.as_ptr();
        let before = super::super::frame_calls_on_thread();
        let SameMaterialRebind::Same(rebound) =
            original.try_rebind_same_material(&digest, tag, current)
        else {
            panic!("相同原件须重绑定");
        };
        assert_eq!(rebound.body().as_ptr(), ptr);
        assert_ne!(rebound.body().as_ptr(), original.body().as_ptr());
        assert_eq!(rebound.usage(), original.usage());
        assert_eq!(super::super::frame_calls_on_thread(), before);

        let (_, mut returned) = rebound.into_parts();
        returned[0] = 9;
        for (d, t, body) in [
            (digest, tag, returned),
            (digest, 0, vec![1, 2, 3]),
            ([0; 32], tag, vec![1, 2, 3]),
        ] {
            let ptr = body.as_ptr();
            let capacity = body.capacity();
            let expected = body.clone();
            let before = super::super::frame_calls_on_thread();
            let SameMaterialRebind::Changed(current) =
                original.try_rebind_same_material(&d, t, body)
            else {
                panic!("变化原件不得沿用旧证明");
            };
            assert_eq!(current.as_ptr(), ptr);
            assert_eq!(current.capacity(), capacity);
            assert_eq!(current, expected);
            assert_eq!(super::super::frame_calls_on_thread(), before);
            assert_eq!(
                CheckedFrame::checked(&d, t, current).unwrap_err().code(),
                code::NODE_DIGEST
            );
            assert_eq!(super::super::frame_calls_on_thread() - before, 1);
        }
    }

    #[test]
    fn checked_same_material_requires_exact_digest_tag_body_and_moves_current_vec() {
        let body = vec![1, 2, 3];
        let tag = super::super::tag::BYTES_CHUNK;
        let digest = frame(tag, &body);
        let original = CheckedFrame::checked(&digest, tag, body).unwrap();
        let current = vec![1, 2, 3];
        let ptr = current.as_ptr();
        let before = super::super::frame_calls_on_thread();
        let rebound = original
            .checked_same_material(&digest, tag, current)
            .unwrap();
        assert_eq!(rebound.body().as_ptr(), ptr);
        assert_eq!(rebound.usage(), original.usage());
        assert_eq!(super::super::frame_calls_on_thread(), before);
        for (d, t, b) in [
            (digest, tag, vec![1, 2, 4]),
            (digest, 0, vec![1, 2, 3]),
            ([0; 32], tag, vec![1, 2, 3]),
        ] {
            let before = super::super::frame_calls_on_thread();
            assert_eq!(
                original.checked_same_material(&d, t, b).unwrap_err().code(),
                code::NODE_DIGEST
            );
            assert_eq!(super::super::frame_calls_on_thread() - before, 1);
        }
        let new_body = vec![9];
        let new_digest = frame(tag, &new_body);
        let before = super::super::frame_calls_on_thread();
        assert_eq!(
            original
                .checked_same_material(&new_digest, tag, new_body)
                .unwrap()
                .digest(),
            &new_digest
        );
        assert_eq!(super::super::frame_calls_on_thread() - before, 1);
    }
}

struct MixedAllocation<'a>(&'a super::capture_allocation::scratch::WorkScope);
impl super::NodeSource for MixedAllocation<'_> {
    fn codec_work(&self) -> Option<&super::capture_allocation::scratch::WorkScope> {
        Some(self.0)
    }
    fn read(&mut self, _: &NodeDigest) -> Result<super::NodeRead, CodecError> {
        Err(CodecError::new(
            super::code::SOURCE,
            "分配能力不提供节点原件",
        ))
    }
}

#[cfg(test)]
mod mixed_domain_tests {
    use super::super::capture_allocation::scratch::{
        CodecWork, ScratchCharge, ScratchFailure, ScratchParent,
    };
    use super::*;
    use std::sync::{Arc, Mutex};
    struct Parent(Mutex<usize>);
    impl ScratchParent for Parent {
        fn reserve(&self, bytes: usize, _: ScratchCharge) -> Result<u64, ScratchFailure> {
            let mut live = self.0.lock().unwrap();
            *live = live.checked_add(bytes).ok_or(ScratchFailure::Budget)?;
            Ok(0)
        }
        fn release(&self, bytes: usize, _: ScratchCharge, _: u64) -> Result<(), ScratchFailure> {
            let mut live = self.0.lock().unwrap();
            *live = live.checked_sub(bytes).ok_or(ScratchFailure::Ledger)?;
            Ok(())
        }
        fn health(&self) -> Result<(), ScratchFailure> {
            Ok(())
        }
        fn invalidate(&self) {
            panic!("test ledger invalidated")
        }
    }
    #[test]
    fn mixed_forced_equal_digest_stays_separate_across_domains() {
        // 只测域键分离：人为设同digest，不声称构造出了跨编码hash碰撞。
        let parent = Arc::new(Parent(Mutex::new(0)));
        let limits = JobLimits::from_canonical_json(&serde_json::json!({
            "max_total_wire_bytes":"1000", "max_total_node_bytes":"1000", "max_total_nodes":"2",
            "max_total_records":"100", "max_total_requests":"100", "max_elapsed_ms":"60000",
            "max_staged_bytes":"1000", "max_store_bytes":"1000"
        }))
        .unwrap();
        CodecWork::new(parent.clone(), 1 << 20)
            .unwrap()
            .run(|work| {
                let mut job = Job::new_started_mixed(
                    limits,
                    Existing {
                        store_bytes: 0,
                        staged_bytes: 0,
                    },
                    Box::new(Instant::now),
                    Instant::now(),
                    work,
                )?;
                let legacy = super::super::legacy_evidence::CheckedLegacyEvidenceRow::verify(
                    &super::super::legacy_evidence::node_id(0, b"abc"),
                    0,
                    b"abc",
                    3,
                )
                .unwrap();
                let content = CheckedNodeUsage {
                    digest: *legacy.id(),
                    framed_bytes: 13,
                };
                assert_eq!(
                    job.accept_checked_usage_mixed(&content, work)?,
                    NodeAcceptance::FirstAccept
                );
                assert_eq!(
                    job.accept_legacy_evidence(&legacy, work)?,
                    NodeAcceptance::FirstAccept
                );
                assert_eq!((job.snapshot().nodes, job.snapshot().node_bytes), (2, 17));
                assert_eq!(
                    job.accept_checked_usage_mixed(&content, work)?,
                    NodeAcceptance::AlreadyAccepted
                );
                assert_eq!(
                    job.accept_legacy_evidence(&legacy, work)?,
                    NodeAcceptance::AlreadyAccepted
                );
                Ok::<_, CodecError>(())
            })
            .unwrap();
        assert_eq!(*parent.0.lock().unwrap(), 0);
    }
    #[test]
    fn mixed_stop_diagnostics_do_not_render_after_resource_rejection() {
        struct MustNotFormat;
        impl fmt::Display for MustNotFormat {
            fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
                panic!("mixed resource failure formatted dynamically");
            }
        }
        let parent = Arc::new(Parent(Mutex::new(0)));
        let limits = JobLimits::from_canonical_json(&serde_json::json!({
            "max_total_wire_bytes":"10", "max_total_node_bytes":"1000", "max_total_nodes":"2",
            "max_total_records":"1", "max_total_requests":"1", "max_elapsed_ms":"60000",
            "max_staged_bytes":"1000", "max_store_bytes":"1000"
        }))
        .unwrap();
        CodecWork::new(parent.clone(), 1 << 20)
            .unwrap()
            .run(|work| {
                let mut job = Job::new_started_mixed(
                    limits,
                    Existing {
                        store_bytes: 0,
                        staged_bytes: 0,
                    },
                    Box::new(Instant::now),
                    Instant::now(),
                    work,
                )?;
                assert!(matches!(
                    job.stop_detail(format_args!("{}", MustNotFormat)),
                    Cow::Borrowed(_)
                ));
                let first = job.charge_wire_bytes(11).unwrap_err();
                assert_eq!(first.code(), LIMIT_EXCEEDED);
                assert_eq!(job.snapshot().wire_bytes, 11);
                assert!(matches!(job.stopped.as_ref().unwrap().1, Cow::Borrowed(_)));
                // Retry is rejected without clearing or fabricating the already observed bytes.
                assert_eq!(job.charge_wire_bytes(3).unwrap_err().code(), LIMIT_EXCEEDED);
                assert_eq!(job.snapshot().wire_bytes, 11);
                assert_eq!(job.pre_io_check().unwrap_err().code(), LIMIT_EXCEEDED);
                assert_eq!(job.finish_check().unwrap_err().code(), LIMIT_EXCEEDED);
                Ok::<_, CodecError>(())
            })
            .unwrap();
        assert_eq!(*parent.0.lock().unwrap(), 0);
        // Ordinary entry keeps its original detailed diagnostic and stopped prefix.
        let mut job = Job::new(
            limits,
            Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(Instant::now),
        )
        .unwrap();
        let err = job.charge_wire_bytes(11).unwrap_err();
        assert!(err.detail().contains("累计 11 超过上限 10"));
        assert!(job
            .pre_io_check()
            .unwrap_err()
            .detail()
            .starts_with("job 已停止"));
        assert!(matches!(job.stopped.as_ref().unwrap().1, Cow::Owned(_)));
    }
}
