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
    pub wire_bytes: u64,
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

static JOB_IDS: AtomicU64 = AtomicU64::new(0);

/// 一个 job 的资源累计核心。
///
/// 一个 job 从用户开始重建或一次 Watch 调用开始，覆盖其全部同代 Snapshot 对拍；
/// 跨分页、片段、证明重传、同代对拍及重试累计，**同一个余额、同一个绝对期限**。
pub struct Job {
    id: u64,
    limits: JobLimits,
    clock: Box<dyn Clock>,
    started: Option<Instant>,
    stopped: Option<(&'static str, String)>,
    wire_bytes: u64,
    node_bytes: u64,
    nodes: u64,
    records: u64,
    requests: u64,
    accepted: BTreeSet<NodeDigest>,
    staged: Space,
    store: Space,
    open: BTreeMap<u64, (SpaceKind, u64)>,
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
            accepted: BTreeSet::new(),
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
            open: BTreeMap::new(),
            next_grant: 1,
        })
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

    /// 显式 charge：应用层实际读写字节总数（重试与祖先重传全额再计，不去重）。
    /// 首次调用即起绝对期限的钟（等价于 [`Job::begin_first_read`]），后续调用只检查不续期。
    pub fn charge_wire_bytes(&mut self, actual_bytes: u64) -> Result<(), CodecError> {
        self.begin_first_read()?;
        let cap = self.limits.max_total_wire_bytes;
        let projected = self.add_cumulative(LIMIT_FIELDS[0], cap, self.wire_bytes, actual_bytes)?;
        self.wire_bytes = projected;
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
        if self.accepted.contains(digest) {
            return Ok(NodeAcceptance::AlreadyAccepted);
        }
        let framed = framed_bytes(body.len());
        let projected_bytes = match self.node_bytes.checked_add(framed) {
            Some(n) => n,
            None => {
                return Err(self.stop(LIMIT_EXCEEDED, format!("{} 累计溢出 u64", LIMIT_FIELDS[1])))
            }
        };
        let projected_nodes = match self.nodes.checked_add(1) {
            Some(n) => n,
            None => {
                return Err(self.stop(LIMIT_EXCEEDED, format!("{} 累计溢出 u64", LIMIT_FIELDS[2])))
            }
        };
        if projected_bytes > self.limits.max_total_node_bytes {
            return Err(self.stop(
                LIMIT_EXCEEDED,
                format!(
                    "{} 累计 {projected_bytes} 超过上限 {}",
                    LIMIT_FIELDS[1], self.limits.max_total_node_bytes
                ),
            ));
        }
        if projected_nodes > self.limits.max_total_nodes {
            return Err(self.stop(
                LIMIT_EXCEEDED,
                format!(
                    "{} 累计 {projected_nodes} 超过上限 {}",
                    LIMIT_FIELDS[2], self.limits.max_total_nodes
                ),
            ));
        }
        self.node_bytes = projected_bytes;
        self.nodes = projected_nodes;
        self.accepted.insert(*digest);
        Ok(NodeAcceptance::FirstAccept)
    }

    /// 写前预留（暂存子集）：同一批字节**同时**占 `max_staged_bytes` 与 `max_store_bytes`，
    /// 一次调用核两本 cap；任一超限即 `job_limit_exceeded` 停止，两本账都不留部分计费状态。
    ///
    /// 峰值按最坏量立即计入两本账；随后由 [`Job::reconcile`] 按实际量对账，
    /// 或由 [`Job::release`] 在取消／失败时退出预留。两处基线都在 [`Job::new`] 一次声明、
    /// 本接口不再接受新的既存量，所以删文件腾挪额度这条通道（§5.6.1）在本层不成立。
    pub fn reserve_staged(&mut self, requested: u64) -> Result<Grant, CodecError> {
        self.gate()?;
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
        self.staged.pending += u128::from(requested);
        self.staged.record_peak(staged_projected);
        self.store.record_peak(store_projected);
        self.next_grant += 1;
        self.open.insert(id, (SpaceKind::Staged, requested));
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
        self.gate()?;
        let store_projected = self.project(
            "存储",
            self.store_exposure(),
            requested,
            self.limits.max_store_bytes,
        )?;
        let id = self.next_grant;
        self.store.pending += u128::from(requested);
        self.store.record_peak(store_projected);
        self.next_grant += 1;
        self.open.insert(id, (SpaceKind::Store, requested));
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
                format!(
                    "{kind:?} 写后实际 {actual} 字节超过预留 {requested} 字节：不足即停止；\
                     真实 actual 已按实际入账，确已删除的暂存用 reclaim_staged 回收"
                ),
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
                format!("{name}既有敞口 {exposure} 已超上限 {cap}"),
            ));
        }
        // exposure ≤ cap ≤ 2^63-1、requested ≤ 2^64-1 ⟹ 和 < 2^65，u128 不可能溢出。
        let projected = exposure + u128::from(requested);
        if projected > cap {
            return Err(self.stop(
                LIMIT_EXCEEDED,
                format!("{name}预留后峰值 {projected} 超过上限 {cap}（既有敞口 {exposure}）"),
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
                return Err(self.stop(LIMIT_EXCEEDED, format!("{name} 累计溢出 u64")));
            }
        };
        if projected > cap {
            return Err(self.stop(
                LIMIT_EXCEEDED,
                format!("{name} 本次 +{add} 后累计 {projected} 超过上限 {cap}"),
            ));
        }
        Ok(projected)
    }

    fn stopped_err(&self) -> Option<CodecError> {
        self.stopped
            .as_ref()
            .map(|(code, detail)| CodecError::new(code, format!("job 已停止（{code}）：{detail}")))
    }

    fn deadline_gate(&self) -> Result<(), CodecError> {
        let Some(elapsed) = self.elapsed() else {
            return Ok(());
        };
        let limit = Duration::from_millis(self.limits.max_elapsed_ms);
        if elapsed >= limit {
            return Err(CodecError::new(
                DEADLINE,
                format!(
                    "已过绝对期限 {} ms（半开区间：必须在期限前收尾）",
                    self.limits.max_elapsed_ms
                ),
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
            return Err(self.stop(DEADLINE, err.detail().to_string()));
        }
        Ok(())
    }

    /// 首次超限即停止：停止不是清账，`snapshot` 仍照实显示全部累计量。
    fn stop(&mut self, code: &'static str, detail: String) -> CodecError {
        if self.stopped.is_none() {
            self.stopped = Some((code, detail.clone()));
        }
        CodecError::new(code, detail)
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
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.job.begin_first_read()?;
        let read = self.inner.read(digest)?;
        // 读后复查：期限在读取期间过掉即拒绝，不再接纳该节点。
        self.job.check_deadline()?;
        match read {
            NodeRead::Missing => Ok(NodeRead::Missing),
            NodeRead::Found { tag, body } => {
                self.job.accept_node(digest, tag, &body)?;
                Ok(NodeRead::Found { tag, body })
            }
        }
    }
}
