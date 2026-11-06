//! #1647：旁挂审计事实存储与提交生命周期。这里不签发来源或闭包核验资格。
//! 调用者只能送入已核验的单代变化；#1481 的冷折叠与在线生产者共用 stage。
//! 完整台账的事实由同次 prepare 的来源收据生产（36M cover 勘误），H1 只经 `Qualification` 点查，
//! 不持裸 Ledger 引用；准入切换仍受 #1645/#1491 门约束。
#![allow(dead_code)]

use super::*;
#[cfg(feature = "s_incremental")]
use newchan_rust::s_content as sc;
use newchan_rust::s_content::capture_allocation::sqlite_parent::{
    ChargedConnection, OpenSpec, SqlGate, SqlOps, SqliteBinding,
};
use newchan_rust::s_content::capture_allocation::{native_owned::NativeValue, scratch::WorkScope};
use newchan_rust::s_content::{ContentRef, RefView};
use rusqlite::types::ValueRef;
use rusqlite::{OpenFlags, ToSql};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::CStr;
use std::os::unix::fs::OpenOptionsExt;
use std::rc::Rc;

/// 36M：cover 改以 source_coord 为键并记 Group(object_id)/Pending，schema 整体升为 /2。
/// 旧 /1 文件不迁移、不读取；`fresh` 只建新文件。
pub(super) const FORMAT: &str = "s-audit-ledger/2";
const RAW_PARTITION_FORMAT: &str = "s-audit-ledger/raw-partition/2";
const SOURCE_PARTITIONS_FORMAT: &str = "s-audit-ledger/source-partitions/2";
const SCHEMA: &str = "
CREATE TABLE meta(singleton INTEGER PRIMARY KEY CHECK(singleton=1), format TEXT NOT NULL,
 rule_revision TEXT NOT NULL, content_identity TEXT NOT NULL, binary_commit TEXT NOT NULL,
 certified_through_generation INTEGER NOT NULL, fold_digest TEXT NOT NULL);
CREATE TABLE raw(seq INTEGER PRIMARY KEY CHECK(seq>=0), event_digest BLOB NOT NULL,
 bar_digest BLOB NOT NULL, first_generation INTEGER NOT NULL);
CREATE TABLE object(id TEXT PRIMARY KEY, record_digest BLOB NOT NULL, first_generation INTEGER NOT NULL,
 first_cut TEXT NOT NULL, published_generation INTEGER NOT NULL, source_digest BLOB NOT NULL,
 withdrawn INTEGER NOT NULL CHECK(withdrawn IN (0,1)), certified_generation INTEGER NOT NULL);
CREATE TABLE witness(id TEXT PRIMARY KEY, record_digest BLOB NOT NULL,
 source_digest BLOB NOT NULL, certified_generation INTEGER NOT NULL);
CREATE TABLE observation(id TEXT PRIMARY KEY, record_digest BLOB NOT NULL,
 source_digest BLOB NOT NULL, certified_generation INTEGER NOT NULL);
CREATE TABLE cover(source_coord INTEGER PRIMARY KEY, seq INTEGER NOT NULL REFERENCES raw(seq),
 owner TEXT NOT NULL CHECK(owner IN ('group','pending')), object_id TEXT,
 certified_generation INTEGER NOT NULL, CHECK((owner='group')=(object_id IS NOT NULL)));
CREATE TABLE node(digest BLOB NOT NULL, flavor TEXT NOT NULL CHECK(flavor IN ('value','json-inner')),
 mode TEXT NOT NULL CHECK(mode IN ('public','template')), first_generation INTEGER NOT NULL,
 PRIMARY KEY(digest,flavor,mode));
CREATE TABLE cert(generation INTEGER PRIMARY KEY CHECK(generation>=1), roots TEXT NOT NULL,
 new_units_digest TEXT NOT NULL, new_units_count INTEGER NOT NULL, fold_digest TEXT NOT NULL,
 verifier_revision TEXT NOT NULL);
";

#[derive(Clone, serde::Serialize)]
pub(super) struct RawFact {
    pub event_digest: [u8; 32],
    pub bar_digest: [u8; 32],
}
#[derive(Clone, serde::Serialize)]
pub(super) struct ObjectFact {
    pub record_digest: [u8; 32],
    pub first_generation: i64,
    pub first_cut: String,
    pub published_generation: i64,
    pub source_digest: [u8; 32],
    pub withdrawn: bool,
}
#[derive(Clone, serde::Serialize)]
pub(super) struct RelatedFact {
    pub record_digest: [u8; 32],
    pub source_digest: [u8; 32],
}
/// cover 单坐标变化：Group/Pending 覆盖该 source_coord 当前行，Removed 删除已失效坐标。
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) enum CoverChange {
    Group { seq: i64, object_id: String },
    Pending { seq: i64 },
    Removed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub(super) enum Flavor {
    Value,
    JsonInner,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub(super) enum Mode {
    Public,
    Template,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub(super) struct Unit {
    pub digest: [u8; 32],
    pub flavor: Flavor,
    pub mode: Mode,
}
impl Unit {
    fn flavor(&self) -> &'static str {
        match self.flavor {
            Flavor::Value => "value",
            Flavor::JsonInner => "json-inner",
        }
    }
    fn mode(&self) -> &'static str {
        match self.mode {
            Mode::Public => "public",
            Mode::Template => "template",
        }
    }
}

/// 来源及闭包生产者的交接数据，不是可由此类型独立证明的审计结论。
/// roots 固定为 scope、generated_state、publication_template、delta_template。
#[derive(Clone)]
pub(super) struct GenerationDelta {
    pub generation: i64,
    pub roots: [ContentRef; 4],
    pub verifier_revision: String,
    pub raw: BTreeMap<i64, RawFact>,
    pub objects: BTreeMap<String, ObjectFact>,
    pub witnesses: BTreeMap<String, RelatedFact>,
    pub observations: BTreeMap<String, RelatedFact>,
    /// 本代撤回的既有对象身份；行必须已存在且未撤回。
    pub withdrawals: BTreeSet<String>,
    pub cover: BTreeMap<i64, CoverChange>,
    /// 本 cut 生效坐标总数；提交后 cover 行数必须恰等于它。
    pub effective: i64,
    pub new_units: BTreeSet<Unit>,
}

pub(super) struct QueuedFacts {
    pub generation: i64,
    pub rule_revision: String,
    pub raw: BTreeMap<i64, RawFact>,
    pub objects: BTreeMap<String, ObjectFact>,
    pub witnesses: BTreeMap<String, RelatedFact>,
    pub observations: BTreeMap<String, RelatedFact>,
    pub withdrawals: BTreeSet<String>,
    pub cover: BTreeMap<i64, CoverChange>,
    pub effective: i64,
}
impl QueuedFacts {
    /// 唯一生产入口：从同次严格来源核的收据取 raw/业务/cover 事实，不接受调用方拼装。
    #[cfg(feature = "s_incremental")]
    pub(super) fn from_receipt(
        receipt: &super::source_audit::source_receipt::RawReceipt,
        metadata: &content_manifest::ManifestMetadata,
    ) -> Result<Self, sc::ContextError> {
        Self::from_receipt_view(receipt, metadata.view())
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn from_receipt_view(
        receipt: &super::source_audit::source_receipt::RawReceipt,
        metadata: content_manifest::MetadataView<'_>,
    ) -> Result<Self, sc::ContextError> {
        let business = receipt.business_rows()?;
        let exports = receipt.cover();
        let mut raw = BTreeMap::new();
        for row in receipt.rows() {
            if raw
                .insert(
                    row.seq,
                    RawFact {
                        event_digest: row.event_digest,
                        bar_digest: row.bar_digest,
                    },
                )
                .is_some()
            {
                return Err(error("来源收据 raw seq 重复").into());
            }
        }
        let mut objects = BTreeMap::new();
        for row in business.objects.iter() {
            objects.insert(
                row.id.as_str().to_owned(),
                ObjectFact {
                    record_digest: row.record,
                    first_generation: row.first_generation,
                    first_cut: row.first_cut.as_str().to_owned(),
                    published_generation: row.published_generation,
                    source_digest: row.sources,
                    withdrawn: false,
                },
            );
        }
        let related =
            |rows: &sc::CodecVec<super::source_audit::source_receipt::source_facts::RelatedRow>| {
                rows.iter()
                    .map(|row| {
                        (
                            row.id.as_str().to_owned(),
                            RelatedFact {
                                record_digest: row.record,
                                source_digest: row.sources,
                            },
                        )
                    })
                    .collect::<BTreeMap<_, _>>()
            };
        let mut cover = BTreeMap::new();
        for coord in exports.removed.iter() {
            cover.insert(*coord, CoverChange::Removed);
        }
        for row in exports.cover.iter() {
            let change = match &row.owner {
                Some(id) => CoverChange::Group {
                    seq: row.seq,
                    object_id: id.as_str().to_owned(),
                },
                None => CoverChange::Pending { seq: row.seq },
            };
            if cover.insert(row.coord, change).is_some() {
                return Err(error("来源收据 cover 坐标重复").into());
            }
        }
        Ok(Self {
            generation: metadata.generation,
            rule_revision: metadata.rule_revision.to_owned(),
            raw,
            objects,
            witnesses: related(&business.witnesses),
            observations: related(&business.observations),
            withdrawals: business
                .withdrawals
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
            cover,
            effective: i64::try_from(exports.effective).map_err(error)?,
        })
    }
    pub(super) fn bind(
        self,
        metadata: &content_manifest::ManifestMetadata,
        roots: &impl content_manifest::ManifestRootSource,
    ) -> Result<GenerationDelta, String> {
        self.bind_view(metadata.view(), roots.view())
    }
    pub(super) fn bind_view(
        self,
        metadata: content_manifest::MetadataView<'_>,
        roots: content_manifest::RootsView<'_>,
    ) -> Result<GenerationDelta, String> {
        if self.generation != metadata.generation || self.rule_revision != metadata.rule_revision {
            return Err(error("排队事实与实际prepare代号或规则不匹配"));
        }
        let copy = |r: RefView<'_>| ContentRef {
            format: r.format().to_owned(),
            root: r.root().to_owned(),
            kind: r.kind(),
            extent: r.extent(),
        };
        Ok(GenerationDelta {
            generation: self.generation,
            roots: [
                copy(metadata.scope),
                copy(roots.generated_state),
                copy(roots.publication_template),
                copy(roots.delta_template),
            ],
            verifier_revision: String::new(),
            raw: self.raw,
            objects: self.objects,
            witnesses: self.witnesses,
            observations: self.observations,
            withdrawals: self.withdrawals,
            cover: self.cover,
            effective: self.effective,
            new_units: BTreeSet::new(),
        })
    }
}

pub(super) struct Identity {
    pub rule_revision: String,
    pub content_identity: String,
    pub binary_commit: String,
}
/// H1 可点查资格（#1481 消费接口）。字段只读，由 `Ledger::qualification` 签发。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Qualification {
    pub through: i64,
    pub roots: String,
    pub fold_digest: String,
    pub verifier_revision: String,
    pub rule_revision: String,
    pub content_identity: String,
    pub binary_commit: String,
}
impl Qualification {
    /// cert.roots 规范 JSON 的字节上界：四个 `{"extent","format","kind","root"}` 对象。
    pub(super) const ROOTS_BOUND: usize = {
        const EXTENT: usize = 22;
        const ROOT: usize = newchan_rust::s_content::ROOT_PREFIX.len() + 64;
        const KIND: usize = "template_batch".len();
        const SYNTAX: usize = 43;
        5 + 4 * (SYNTAX + EXTENT + newchan_rust::s_content::CONTENT_FORMAT.len() + KIND + ROOT)
    };
    /// 一份资格全部字符串字段的字节上界；调用方据此在读取前预留，超过即拒绝。
    #[cfg(feature = "s_incremental")]
    pub(super) fn retained_bound(
        rule_revision: usize,
        content_identity: usize,
        binary_commit: usize,
    ) -> usize {
        [
            Self::ROOTS_BOUND,
            64,
            sc::closure_delta::VERIFIER_REVISION.len(),
            rule_revision,
            content_identity,
            binary_commit,
        ]
        .into_iter()
        .fold(0usize, usize::saturating_add)
    }
    /// 同一行上的借用视图，不分配；`stage` 前核旧资格与 H1 点查都走这里。
    fn matches(&self, v: &QualificationView<'_>) -> bool {
        self.through == v.through
            && self.roots == v.roots
            && self.fold_digest == v.fold_digest
            && self.verifier_revision == v.verifier_revision
            && self.rule_revision == v.rule_revision
            && self.content_identity == v.content_identity
            && self.binary_commit == v.binary_commit
    }
}
/// #1419 M8D 3B：`stage_verified` 的唯一单位输入。`frontier` 为 None 是完整闭包核验；
/// 为 Some 时单位来自以该资格的已提交完整单位为前沿的同一核验，只能由 `verify_delta` 签发。
#[cfg(feature = "s_incremental")]
pub(super) struct QualifiedUnits<'q> {
    verified: sc::closure_delta::VerifiedUnits,
    frontier: Option<&'q Qualification>,
}
#[cfg(feature = "s_incremental")]
impl QualifiedUnits<'static> {
    /// 完整闭包核验结果；前沿止步的核验结果不能经此冒充完整核验。
    pub(super) fn full(verified: sc::closure_delta::VerifiedUnits) -> Result<Self, String> {
        if verified.frontier_used() {
            return Err(error("前沿止步的单位不能作为完整核验结果"));
        }
        Ok(Self {
            verified,
            frontier: None,
        })
    }
}
/// 已提交合格单位前沿：每次点查都经原 `contains_qualified_unit`（资格逐字段核 + 完整键 EXISTS）。
#[cfg(feature = "s_incremental")]
struct QualifiedSuccessView<'a, B> {
    ledger: &'a Ledger<B>,
    qualification: &'a Qualification,
}
#[cfg(feature = "s_incremental")]
impl<B: Backend> sc::closure_delta::Frontier for QualifiedSuccessView<'_, B> {
    fn contains(&self, unit: sc::closure_delta::Unit) -> Result<bool, sc::CodecError> {
        self.ledger
            .contains_qualified_unit(self.qualification, &Unit::of_closure(unit))
            .map_err(|e| sc::CodecError::new(sc::code::SOURCE, e.to_string()))
    }
}
#[cfg(feature = "s_incremental")]
impl Unit {
    fn of_closure(u: sc::closure_delta::Unit) -> Self {
        use sc::closure_delta::{Context, VerificationMode};
        Self {
            digest: u.digest,
            flavor: match u.context {
                Context::Value => Flavor::Value,
                Context::JsonInner => Flavor::JsonInner,
            },
            mode: match u.mode {
                VerificationMode::Public => Mode::Public,
                VerificationMode::Template => Mode::Template,
            },
        }
    }
}
struct QualificationView<'a> {
    through: i64,
    roots: &'a str,
    fold_digest: &'a str,
    verifier_revision: &'a str,
    rule_revision: &'a str,
    content_identity: &'a str,
    binary_commit: &'a str,
}
impl QualificationView<'_> {
    fn string_bytes(&self) -> usize {
        [
            self.roots,
            self.fold_digest,
            self.verifier_revision,
            self.rule_revision,
            self.content_identity,
            self.binary_commit,
        ]
        .into_iter()
        .map(str::len)
        .fold(0usize, usize::saturating_add)
    }
    fn owned(&self) -> Qualification {
        fn exact(s: &str) -> String {
            let mut owned = String::with_capacity(s.len());
            owned.push_str(s);
            owned
        }
        Qualification {
            through: self.through,
            roots: exact(self.roots),
            fold_digest: exact(self.fold_digest),
            verifier_revision: exact(self.verifier_revision),
            rule_revision: exact(self.rule_revision),
            content_identity: exact(self.content_identity),
            binary_commit: exact(self.binary_commit),
        }
    }
}
/// 与 `stage_inner` 写入 cert.roots 的 `canonical_json(roots_value)` 逐字节相同；写入预留好的缓冲。
pub(super) fn roots_json_into(out: &mut String, roots: [RefView<'_>; 4]) -> Result<(), String> {
    use std::fmt::Write as _;
    out.push('[');
    for (i, r) in roots.into_iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let extent = match r.extent() {
            Some(n) => serde_json::to_string(&n.to_string()).map_err(error)?,
            None => "null".to_owned(),
        };
        write!(
            out,
            "{{\"extent\":{extent},\"format\":{},\"kind\":{},\"root\":{}}}",
            serde_json::to_string(r.format()).map_err(error)?,
            serde_json::to_string(r.kind().name()).map_err(error)?,
            serde_json::to_string(r.root()).map_err(error)?,
        )
        .map_err(error)?;
    }
    out.push(']');
    Ok(())
}

struct Pending {
    token: NativeValue<String>,
    generation: i64,
    roots: [NativeValue<ContentRef>; 4],
}
impl Pending {
    fn new(
        token: &str,
        generation: i64,
        roots: [RefView<'_>; 4],
        work: Option<&WorkScope>,
    ) -> Result<Self, String> {
        Ok(Self {
            token: NativeValue::<String>::try_from_str(token, work).map_err(error)?,
            generation,
            roots: [
                NativeValue::<ContentRef>::try_from_view(roots[0], work).map_err(error)?,
                NativeValue::<ContentRef>::try_from_view(roots[1], work).map_err(error)?,
                NativeValue::<ContentRef>::try_from_view(roots[2], work).map_err(error)?,
                NativeValue::<ContentRef>::try_from_view(roots[3], work).map_err(error)?,
            ],
        })
    }
}
enum Phase {
    Ready,
    Pending(Pending),
    Bound(Pending),
    Stale,
}
enum Coverage {
    Complete,
    RawPartition { through: i64 },
    SourcePartitions { through: i64 },
}

/// 台账连接后端，模块内只有 [`Legacy`] 与 [`Charged`] 两个实现，在构造处选定且不切换。
/// 台账阶段机与 SQL 只经这三个操作访问数据库，两个后端执行同一份 SQL。
pub(super) trait Backend {
    /// Legacy 沿用原 `String` 诊断；Charged 为 [`LedgerError`]，带本次操作的 typed 分类。
    type Error: From<String> + std::fmt::Display;
    fn execute_batch(&self, sql: &str) -> Result<(), Self::Error>;
    fn execute<const N: usize>(
        &self,
        sql: &str,
        params: [&dyn ToSql; N],
    ) -> Result<usize, Self::Error>;
    /// 取首行 `M` 列交 `f` 解码；`f` 内不能访问数据库，需要带出的值由 `f` 复制。
    fn query_row<T, const N: usize, const M: usize>(
        &self,
        sql: &str,
        params: [&dyn ToSql; N],
        f: impl FnOnce([ValueRef<'_>; M]) -> Result<T, String>,
    ) -> Result<T, Self::Error>;
    /// 连接已在失效时丢弃。
    fn detached() -> Self::Error;
}

/// 现行构造：本进程未安装收费 allocator（生产与普通入口沿用）。
pub(super) struct Legacy(Connection);

impl Legacy {
    fn open(path: &Path, pages: i64) -> Result<Self, String> {
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(error)?;
        conn.busy_timeout(std::time::Duration::ZERO)
            .map_err(error)?;
        conn.execute_batch(PRAGMAS).map_err(error)?;
        conn.pragma_update(None, "page_size", 4096).map_err(error)?;
        conn.pragma_update(None, "max_page_count", pages)
            .map_err(error)?;
        Ok(Self(conn))
    }
    #[cfg(test)]
    fn legacy(&self) -> &Connection {
        &self.0
    }
}

impl Backend for Legacy {
    type Error = String;
    fn execute_batch(&self, sql: &str) -> Result<(), String> {
        self.0.execute_batch(sql).map_err(error)
    }
    fn execute<const N: usize>(&self, sql: &str, params: [&dyn ToSql; N]) -> Result<usize, String> {
        self.0.execute(sql, params.as_slice()).map_err(error)
    }
    fn query_row<T, const N: usize, const M: usize>(
        &self,
        sql: &str,
        params: [&dyn ToSql; N],
        f: impl FnOnce([ValueRef<'_>; M]) -> Result<T, String>,
    ) -> Result<T, String> {
        self.0
            .query_row(sql, params.as_slice(), |r| {
                let mut row = [ValueRef::Null; M];
                for (i, v) in row.iter_mut().enumerate() {
                    *v = r.get_ref(i)?;
                }
                Ok(f(row))
            })
            .map_err(error)?
    }
    fn detached() -> String {
        error("台账已丢弃")
    }
}

/// 受管台账的错误。受管操作口的拒绝与 SQLite 结果码在产生时按值保存为 [`SqlGate`]，
/// 不分配、不依赖之后可能被其他冻结域改写的绑定首拒槽；文本只在输出边界格式化。
#[derive(Debug, PartialEq, Eq)]
pub(super) enum LedgerError {
    /// 台账业务判据或行解码失败（原诊断文本）。
    Stale(String),
    /// 本次受管操作的结果分类；NOMEM 背后有本域首拒时记为该首拒。
    Gate(SqlGate),
    /// 连接已在失效时丢弃。
    Detached,
}

impl From<String> for LedgerError {
    fn from(text: String) -> Self {
        Self::Stale(text)
    }
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stale(text) => f.write_str(text),
            Self::Gate(gate) => write!(f, "AuditLedgerStale：{gate:?}"),
            Self::Detached => f.write_str("AuditLedgerStale：台账已丢弃"),
        }
    }
}

impl From<LedgerError> for String {
    fn from(e: LedgerError) -> Self {
        match e {
            LedgerError::Stale(text) => text,
            other => other.to_string(),
        }
    }
}

impl LedgerError {
    /// 接既有冷认证错误层：本次的 Budget 首拒成为 `BudgetExceeded`，其余保留为诊断。
    #[cfg(feature = "s_incremental")]
    #[allow(
        dead_code,
        reason = "#1419 3C：冷认证接受管台账时经此接 cold_failure，首单元只由锁消费"
    )]
    pub(super) fn into_failure(self) -> super::audit_owner::native::Failure {
        use newchan_rust::s_content::capture_allocation::scratch::ScratchFailure;
        match self {
            Self::Gate(SqlGate::Refused(ScratchFailure::Budget)) => {
                super::audit_owner::native::Failure::BudgetExceeded
            }
            other => super::audit_owner::native::Failure::Invalid(other.into()),
        }
    }
}

/// 受管构造：全新进程已装 allocator，连接由原 Native 父账的当前绑定开出。
pub(super) struct Charged {
    conn: Option<ChargedConnection>,
    binding: Rc<SqliteBinding>,
}

impl Drop for Charged {
    /// 丢弃即回滚未提交的整代再关库，与旧连接析构的结果相同；两步都是收尾域，首拒之后仍执行。
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            let _ = self.binding.rollback(&conn);
            if let Err((conn, _)) = self.binding.close(conn) {
                drop(conn);
            }
        }
    }
}

impl Charged {
    /// 不在 Rust 堆上建任何持有：路径由调用方以其许可借出，SQL 文本为静态串或栈缓冲。
    fn open(path: &CStr, pages: i64, binding: Rc<SqliteBinding>) -> Result<Self, LedgerError> {
        let spec = OpenSpec {
            flags: rusqlite::ffi::SQLITE_OPEN_READWRITE | rusqlite::ffi::SQLITE_OPEN_NOMUTEX,
            busy_timeout_ms: Some(0),
            ..OpenSpec::default()
        };
        // SAFETY: the spec carries no caller callbacks, contexts or VFS.
        let conn = unsafe { binding.open_with(path, &spec) }
            .map_err(|e| LedgerError::Gate(classify(&binding, e)))?;
        let store = Self {
            conn: Some(conn),
            binding,
        };
        store.execute_batch(PRAGMAS)?;
        store.execute_batch("PRAGMA page_size=4096")?;
        let mut max_pages = StackSql::new();
        std::fmt::Write::write_fmt(
            &mut max_pages,
            format_args!("PRAGMA max_page_count={pages}"),
        )
        .map_err(|_| LedgerError::Gate(SqlGate::Sqlite(rusqlite::ffi::SQLITE_TOOBIG)))?;
        store.execute_batch(max_pages.as_str())?;
        Ok(store)
    }
    fn run<R>(&self, f: impl FnOnce(&SqlOps<'_>) -> Result<R, SqlGate>) -> Result<R, LedgerError> {
        let conn = self.conn.as_ref().ok_or(LedgerError::Detached)?;
        self.binding
            .run(conn, f)
            .map_err(|e| LedgerError::Gate(classify(&self.binding, e)))
    }
}

impl Backend for Charged {
    type Error = LedgerError;
    fn execute_batch(&self, sql: &str) -> Result<(), LedgerError> {
        self.run(|q| q.execute_batch(sql))
    }
    fn execute<const N: usize>(
        &self,
        sql: &str,
        params: [&dyn ToSql; N],
    ) -> Result<usize, LedgerError> {
        self.run(|q| q.execute(sql, params))
    }
    fn query_row<T, const N: usize, const M: usize>(
        &self,
        sql: &str,
        params: [&dyn ToSql; N],
        f: impl FnOnce([ValueRef<'_>; M]) -> Result<T, String>,
    ) -> Result<T, LedgerError> {
        self.run(|q| q.query_row(sql, params, f))?
            .map_err(LedgerError::Stale)
    }
    fn detached() -> LedgerError {
        LedgerError::Detached
    }
}

/// 本次结果的分类：SQLite 报 NOMEM 且本域已有首拒时记为该首拒，不以结果码掩盖资源拒绝。
fn classify(binding: &SqliteBinding, e: SqlGate) -> SqlGate {
    match e {
        SqlGate::Sqlite(code) => binding
            .refusal_of(code)
            .map_or(SqlGate::Sqlite(code), SqlGate::Refused),
        other => other,
    }
}

/// 定长栈缓冲上的 SQL 文本，不经堆。
struct StackSql {
    bytes: [u8; 48],
    len: usize,
}

impl std::fmt::Write for StackSql {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let end = self
            .len
            .checked_add(s.len())
            .filter(|&end| end <= self.bytes.len())
            .ok_or(std::fmt::Error)?;
        self.bytes[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

impl StackSql {
    fn new() -> Self {
        Self {
            bytes: [0; 48],
            len: 0,
        }
    }
    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap_or_default()
    }
}

const PRAGMAS: &str =
    "PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;";

fn int(v: ValueRef<'_>) -> Result<i64, String> {
    v.as_i64().map_err(error)
}
fn text(v: ValueRef<'_>) -> Result<&str, String> {
    v.as_str().map_err(error)
}

pub(super) struct Ledger<B = Legacy> {
    connection: Option<B>,
    phase: Phase,
    coverage: Coverage,
    path: PathBuf,
    content_path: PathBuf,
    max_store_bytes: u64,
    rule_revision: String,
}

fn error(e: impl std::fmt::Display) -> String {
    format!("AuditLedgerStale：{e}")
}
fn root_value(r: RefView<'_>) -> Value {
    json!({"format":r.format(), "root":r.root(), "kind":r.kind().name(),
        "extent":r.extent().map(|n|n.to_string())})
}
fn roots_value(roots: &[ContentRef; 4]) -> Value {
    Value::Array(roots.iter().map(|r| root_value(r.into())).collect())
}
fn file_bytes(path: &Path) -> Result<u64, String> {
    match std::fs::metadata(path) {
        Ok(m) => Ok(m.len()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(error(e)),
    }
}
fn database_bytes(path: &Path) -> Result<u64, String> {
    let mut n = file_bytes(path)?;
    for suffix in ["-wal", "-journal"] {
        let mut p = path.as_os_str().to_os_string();
        p.push(suffix);
        n = n
            .checked_add(file_bytes(Path::new(&p))?)
            .ok_or_else(|| error("存储计数溢出"))?;
    }
    Ok(n)
}

impl Ledger {
    /// 永不打开旧台账作可信输入。文件名必须是本次重建的新文件；不覆盖旧诊断原件。
    pub(super) fn fresh(
        path: &Path,
        content_path: &Path,
        identity: Identity,
        max_store_bytes: u64,
    ) -> Result<Self, String> {
        Self::fresh_in(path, content_path, identity, max_store_bytes, Legacy::open)
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn fresh_raw_partition(
        path: &Path,
        content_path: &Path,
        identity: Identity,
        max_store_bytes: u64,
    ) -> Result<Self, String> {
        let mut ledger = Self::fresh(path, content_path, identity, max_store_bytes)?;
        ledger.store()?.execute(
            "UPDATE meta SET format=?1 WHERE singleton=1",
            [&RAW_PARTITION_FORMAT],
        )?;
        ledger.coverage = Coverage::RawPartition { through: 0 };
        Ok(ledger)
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn fresh_source_partitions(
        path: &Path,
        content_path: &Path,
        identity: Identity,
        max_store_bytes: u64,
    ) -> Result<Self, String> {
        let mut ledger = Self::fresh(path, content_path, identity, max_store_bytes)?;
        ledger.store()?.execute(
            "UPDATE meta SET format=?1 WHERE singleton=1",
            [&SOURCE_PARTITIONS_FORMAT],
        )?;
        ledger.coverage = Coverage::SourcePartitions { through: 0 };
        Ok(ledger)
    }
}

impl Ledger<Charged> {
    /// 受管构造：同一份阶段机与 SQL，连接由 `binding` 当前绑定的原 Native 父账开出，SQLite
    /// 分配向该父账收费。调用方须已在本进程首次 SQLite 初始化前装好 allocator；本构造不安装，
    /// 失败即失败，不改用旧连接。
    #[allow(
        dead_code,
        reason = "#1419 3C：受管台账只由全新进程 release 锁消费，Native 一次启用波接入生产"
    )]
    pub(super) fn fresh_charged(
        path: &CStr,
        content_path: &Path,
        identity: Identity,
        max_store_bytes: u64,
        binding: Rc<SqliteBinding>,
    ) -> Result<Self, LedgerError> {
        use std::os::unix::ffi::OsStrExt;
        let fs_path = Path::new(std::ffi::OsStr::from_bytes(path.to_bytes()));
        Self::fresh_in(
            fs_path,
            content_path,
            identity,
            max_store_bytes,
            |_, pages| Charged::open(path, pages, binding),
        )
    }
}

impl<B: Backend> Ledger<B> {
    fn fresh_in(
        path: &Path,
        content_path: &Path,
        identity: Identity,
        max_store_bytes: u64,
        open: impl FnOnce(&Path, i64) -> Result<B, B::Error>,
    ) -> Result<Self, B::Error> {
        if identity.rule_revision.is_empty()
            || identity.content_identity.is_empty()
            || identity.binary_commit.is_empty()
        {
            return Err(error("台账身份字段不能为空").into());
        }
        let content_path = content_path.canonicalize().map_err(error)?;
        let remaining = max_store_bytes
            .checked_sub(database_bytes(&content_path)?)
            .ok_or_else(|| error("内容库已超过 max_store_bytes"))?;
        let pages = i64::try_from(remaining / 4096).map_err(error)?;
        if pages == 0 {
            return Err(error("没有台账页空间").into());
        }
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(error)?;
        let conn = open(path, pages)?;
        conn.execute_batch(SCHEMA)?;
        conn.execute(
            "INSERT INTO meta VALUES(1,?1,?2,?3,?4,0,?5)",
            [
                &FORMAT,
                &identity.rule_revision,
                &identity.content_identity,
                &identity.binary_commit,
                &sha256_hex(FORMAT.as_bytes()),
            ],
        )?;
        let ledger = Self {
            connection: Some(conn),
            phase: Phase::Ready,
            coverage: Coverage::Complete,
            path: path.to_owned(),
            content_path,
            max_store_bytes,
            rule_revision: identity.rule_revision,
        };
        ledger.check_storage()?;
        Ok(ledger)
    }
    pub(super) fn is_source_partitions(&self) -> bool {
        matches!(self.coverage, Coverage::SourcePartitions { .. })
    }
    pub(super) fn is_partial(&self) -> bool {
        self.is_raw_partition() || self.is_source_partitions()
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn partial_through(&self) -> Result<i64, B::Error> {
        match (&self.phase, &self.coverage) {
            (
                Phase::Ready,
                Coverage::RawPartition { through } | Coverage::SourcePartitions { through },
            ) => Ok(*through),
            _ => Err(error("不是已提交的来源分区").into()),
        }
    }
    pub(super) fn is_raw_partition(&self) -> bool {
        matches!(self.coverage, Coverage::RawPartition { .. })
    }
    /// 完整台账且无进行中的一代：本次 prepare 须由来源收据生产完整事实（36M）。
    /// 组件路径预先 stage 的 Pending 台账不再生产，Partial 台账走各自分区入口。
    pub(super) fn accepts_production(&self) -> bool {
        matches!(self.phase, Phase::Ready) && matches!(self.coverage, Coverage::Complete)
    }
    /// 本次 prepare 是否需要收据导出业务/cover 事实。
    pub(super) fn wants_business_facts(&self) -> bool {
        self.is_source_partitions() || self.accepts_production()
    }
    /// 当前已提交资格的借用视图：末代 cert 与 meta 同一语句取出，不分配；在 `stage_inner`
    /// 的 BEGIN IMMEDIATE 内调用时读到的就是本事务改写前的已提交状态。
    /// 规则取持久 `meta.rule_revision` 并与安装规则同事务核（40H B1），不以内存副本替代。
    fn with_committed_qualification<R>(
        &self,
        f: impl FnOnce(&QualificationView<'_>) -> Result<R, String>,
    ) -> Result<R, B::Error> {
        let through = self.committed_through()?;
        if through < 1 {
            return Err(error("完整台账尚无已提交的一代").into());
        }
        self.store()?.query_row(
            "SELECT c.roots, c.fold_digest, c.verifier_revision, m.fold_digest, m.content_identity, \
             m.binary_commit, m.rule_revision FROM cert c, meta m WHERE c.generation=?1 AND m.singleton=1",
            [&through],
            |[roots, fold, verifier, meta_fold, content, binary, rule]| {
                let fold_digest = text(fold)?;
                if text(meta_fold)? != fold_digest {
                    return Err(error("完整水位与末代 cert 折叠摘要不一致").into());
                }
                let rule_revision = text(rule)?;
                if rule_revision != self.rule_revision {
                    return Err(error("台账持久规则与安装规则不一致").into());
                }
                f(&QualificationView {
                    through,
                    roots: text(roots)?,
                    fold_digest,
                    verifier_revision: text(verifier)?,
                    rule_revision,
                    content_identity: text(content)?,
                    binary_commit: text(binary)?,
                })
            },
        )
    }
    /// H1 的私有点查资格：绑定已提交完整水位、四根、规则、verifier 与内容库身份。
    /// 不含连接、不可跨进程；任何字段不符即不能用它点查。
    pub(super) fn qualification(&self) -> Result<Qualification, B::Error> {
        self.with_committed_qualification(|v| Ok(v.owned()))
    }
    /// 39P：字符串总字节先对调用方已预留的上界核过，再按精确容量分配（B2）。
    pub(super) fn qualification_bounded(&self, cap: usize) -> Result<Qualification, B::Error> {
        self.with_committed_qualification(|v| {
            if v.string_bytes() > cap {
                return Err(error("完整资格超过调用方保留上界").into());
            }
            Ok(v.owned())
        })
    }
    fn matches_committed(&self, q: &Qualification) -> Result<bool, B::Error> {
        self.with_committed_qualification(|v| Ok(q.matches(v)))
    }
    /// 只接受本台账当前签发且水位未变的资格；台账 Pending/Bound/Stale 或已推进一代即拒绝。
    pub(super) fn contains_qualified_unit(
        &self,
        q: &Qualification,
        unit: &Unit,
    ) -> Result<bool, B::Error> {
        if !self.matches_committed(q)? {
            return Err(error("点查资格与当前已提交台账不符").into());
        }
        self.contains_committed_unit(unit)
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn raw_partition_through(&self) -> Result<i64, B::Error> {
        match (&self.phase, &self.coverage) {
            (Phase::Ready, Coverage::RawPartition { through }) => Ok(*through),
            _ => Err(error("不是已提交的raw分区").into()),
        }
    }
    fn check_storage(&self) -> Result<(), B::Error> {
        let size = database_bytes(&self.content_path)?
            .checked_add(database_bytes(&self.path)?)
            .ok_or_else(|| error("存储合账溢出"))?;
        if size > self.max_store_bytes {
            return Err(error("内容库/WAL/台账合计超过 max_store_bytes").into());
        }
        Ok(())
    }
    pub(super) fn invalidate(&mut self) {
        self.phase = Phase::Stale;
        // 丢弃连接也回滚未提交的整代；不重试，也不复原旧进程状态。
        drop(self.connection.take());
    }
    fn store(&self) -> Result<&B, B::Error> {
        self.connection.as_ref().ok_or_else(B::detached)
    }
    pub(super) fn through(&self) -> Result<i64, B::Error> {
        self.store()?.query_row(
            "SELECT certified_through_generation FROM meta WHERE singleton=1",
            [],
            |[v]| int(v),
        )
    }
    /// 已提交存储读数，不是 U-A 资格；准备中连本连接的未提交行也不暴露。
    pub(super) fn committed_through(&self) -> Result<i64, B::Error> {
        if !matches!(self.phase, Phase::Ready) || !matches!(self.coverage, Coverage::Complete) {
            return Err(error("完整台账尚未提交或仅有raw分区").into());
        }
        self.through()
    }
    pub(super) fn contains_committed_unit(&self, unit: &Unit) -> Result<bool, B::Error> {
        let through = self.committed_through()?;
        self.store()?.query_row(
            "SELECT EXISTS(SELECT 1 FROM node WHERE digest=?1 AND flavor=?2 AND mode=?3 AND first_generation<=?4)",
            [&unit.digest.as_slice(), &unit.flavor(), &unit.mode(), &through],
            |[v]| int(v).map(|n| n != 0),
        )
    }
    pub(super) fn stage(&mut self, token: &str, delta: &GenerationDelta) -> Result<(), B::Error> {
        let result = self.stage_inner(
            token,
            delta,
            delta.new_units.iter(),
            &delta.verifier_revision,
            None,
        );
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    /// #1419 M8D 3B：以本台账当前签发的资格为前沿做同一严格四根核验，完整单位命中即止步。
    /// 资格不符、台账非 Ready 或前沿点查失败都整体失败，不退回完整核验、不当 miss。
    /// 结果绑定该资格；`stage_verified` 在同一台账事务内复核它就是上一代资格。
    #[cfg(feature = "s_incremental")]
    pub(super) fn verify_delta<'q>(
        &self,
        source: &mut dyn sc::NodeSource,
        roots: [RefView<'_>; 4],
        qualification: &'q Qualification,
    ) -> Result<QualifiedUnits<'q>, B::Error> {
        if !self.matches_committed(qualification)? {
            return Err(error("Δ核验资格与当前已提交台账不符").into());
        }
        let view = QualifiedSuccessView {
            ledger: self,
            qualification,
        };
        let verified =
            sc::closure_delta::verify_four_roots_from(source, roots, &view).map_err(error)?;
        Ok(QualifiedUnits {
            verified,
            frontier: Some(qualification),
        })
    }
    /// `prior`（39P B1）：H1 持有的上一代资格；stage 在同一台账事务内先核它与当前已提交
    /// 状态逐字段相同，再改写折叠链。外写 meta/cert 不能被新一代 stage 吸收为自洽。
    /// 唯一单位入口（M8D 条件 A）：完整核验与前沿止步核验同判据，后者要求绑定资格等于 `prior`。
    #[cfg(feature = "s_incremental")]
    pub(super) fn stage_verified(
        &mut self,
        token: &str,
        delta: &GenerationDelta,
        verified: &QualifiedUnits<'_>,
        work: Option<&sc::capture_allocation::scratch::WorkScope>,
        prior: Option<&Qualification>,
    ) -> Result<(), B::Error> {
        let result = (|| -> Result<(), B::Error> {
            self.committed_through()?;
            if let Some(bound) = verified.frontier {
                if prior != Some(bound) {
                    return Err(error("Δ单位绑定资格与上一代资格不符").into());
                }
            }
            let verified = &verified.verified;
            if !verified
                .matches_roots(delta.roots.each_ref().map(Into::into))
                .map_err(error)?
            {
                return Err(error("闭包单位与待准备事实四根不匹配").into());
            }
            let mut units = sc::CodecVec::default();
            verified.try_for_each_unit(|u| -> Result<(), B::Error> {
                let unit = Unit::of_closure(u);
                if !self.contains_committed_unit(&unit)? {
                    match work {
                        Some(work) => units.try_reserve_native(work, 1),
                        None => units.try_reserve(1),
                    }
                    .map_err(error)?;
                    units.try_push(unit).map_err(error)?;
                }
                Ok(())
            })?;
            units.sort_unstable();
            self.stage_inner(
                token,
                delta,
                units.iter(),
                sc::closure_delta::VERIFIER_REVISION,
                prior,
            )
        })();
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    fn stage_inner<'u>(
        &mut self,
        token: &str,
        d: &GenerationDelta,
        new_units: impl Iterator<Item = &'u Unit> + Clone,
        verifier_revision: &str,
        prior: Option<&Qualification>,
    ) -> Result<(), B::Error> {
        if !matches!(self.phase, Phase::Ready) || !matches!(self.coverage, Coverage::Complete) {
            return Err(error("台账不在完整事实可准备状态").into());
        }
        if token.is_empty()
            || verifier_revision.is_empty()
            || self.through()?.checked_add(1) != Some(d.generation)
        {
            return Err(error("请求或逐代序号无效").into());
        }
        for r in &d.roots {
            r.digest().map_err(error)?;
        }
        self.check_storage()?;
        let conn = self.store()?;
        conn.execute_batch("BEGIN IMMEDIATE")?;
        if let Some(prior) = prior {
            let detail = match self.matches_committed(prior) {
                Ok(true) => None,
                Ok(false) => Some("字段不同".to_owned()),
                Err(e) => Some(e.to_string()),
            };
            if let Some(detail) = detail {
                return Err(error(format!(
                    "上一代资格与台账当前已提交状态不符，不续写折叠链：{detail}"
                ))
                .into());
            }
        }
        let previous: String = conn.query_row(
            "SELECT fold_digest FROM meta WHERE singleton=1",
            [],
            |[v]| text(v).map(str::to_owned),
        )?;
        let mut next: i64 =
            conn.query_row("SELECT COALESCE(MAX(seq),-1)+1 FROM raw", [], |[v]| int(v))?;
        for (seq, row) in &d.raw {
            if *seq != next {
                return Err(error("raw 只允许连续追加").into());
            }
            conn.execute(
                "INSERT INTO raw VALUES(?1,?2,?3,?4)",
                [
                    &seq,
                    &row.event_digest.as_slice(),
                    &row.bar_digest.as_slice(),
                    &d.generation,
                ],
            )?;
            next = next.checked_add(1).ok_or_else(|| error("raw 序号溢出"))?;
        }
        for (id, r) in &d.objects {
            if r.first_generation < 1
                || r.first_generation > r.published_generation
                || r.published_generation > d.generation
            {
                return Err(error("对象代戳无效").into());
            }
            conn.execute("INSERT INTO object VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
                ON CONFLICT(id) DO UPDATE SET record_digest=excluded.record_digest,
                first_generation=excluded.first_generation, first_cut=excluded.first_cut,
                published_generation=excluded.published_generation, source_digest=excluded.source_digest,
                withdrawn=excluded.withdrawn, certified_generation=excluded.certified_generation",
                [&id, &r.record_digest.as_slice(), &r.first_generation, &r.first_cut, &r.published_generation, &r.source_digest.as_slice(), &r.withdrawn, &d.generation])?;
        }
        for (table, rows) in [("witness", &d.witnesses), ("observation", &d.observations)] {
            for (id, r) in rows {
                conn.execute(&format!("INSERT INTO {table} VALUES(?1,?2,?3,?4)
                    ON CONFLICT(id) DO UPDATE SET record_digest=excluded.record_digest,
                    source_digest=excluded.source_digest, certified_generation=excluded.certified_generation"),
                    [&id, &r.record_digest.as_slice(), &r.source_digest.as_slice(), &d.generation])?;
            }
        }
        for id in &d.withdrawals {
            let changed = conn.execute(
                "UPDATE object SET withdrawn=1,certified_generation=?1 WHERE id=?2 AND withdrawn=0",
                [&d.generation, &id],
            )?;
            if changed != 1 {
                return Err(error("撤回身份缺失或已撤回").into());
            }
        }
        for (coord, change) in &d.cover {
            match change {
                CoverChange::Removed => {
                    if conn.execute("DELETE FROM cover WHERE source_coord=?1", [coord])? != 1 {
                        return Err(error("cover 失效坐标不在当前分区").into());
                    }
                }
                // 行只在 (seq, owner, object_id) 变化时推进 certified_generation：Native 增量只列
                // 触及坐标、完整原件导出列整份分区，同一内容在两条路径上必须落出同一行。
                CoverChange::Group { seq, object_id } => {
                    conn.execute(
                        "INSERT INTO cover VALUES(?1,?2,'group',?3,?4) ON CONFLICT(source_coord) DO UPDATE SET
                         seq=excluded.seq, owner=excluded.owner, object_id=excluded.object_id,
                         certified_generation=excluded.certified_generation
                         WHERE seq IS NOT excluded.seq OR owner IS NOT excluded.owner
                         OR object_id IS NOT excluded.object_id",
                        [&coord, &seq, &object_id, &d.generation],
                    )?;
                }
                CoverChange::Pending { seq } => {
                    conn.execute(
                        "INSERT INTO cover VALUES(?1,?2,'pending',NULL,?3) ON CONFLICT(source_coord) DO UPDATE SET
                         seq=excluded.seq, owner=excluded.owner, object_id=excluded.object_id,
                         certified_generation=excluded.certified_generation
                         WHERE seq IS NOT excluded.seq OR owner IS NOT excluded.owner
                         OR object_id IS NOT excluded.object_id",
                        [&coord, &seq, &d.generation],
                    )?;
                }
            }
        }
        let covered: i64 = conn.query_row("SELECT COUNT(*) FROM cover", [], |[v]| int(v))?;
        if covered != d.effective {
            return Err(error("cover 行数不等于本 cut 生效坐标总数").into());
        }
        for unit in new_units.clone() {
            // new_units 必须真是本代新增集合，重复认证单位拒绝而非静默忽略。
            conn.execute(
                "INSERT INTO node VALUES(?1,?2,?3,?4)",
                [
                    &unit.digest.as_slice(),
                    &unit.flavor(),
                    &unit.mode(),
                    &d.generation,
                ],
            )?;
        }
        let roots = roots_value(&d.roots);
        let unit_count = new_units.clone().count();
        let unit_values = Value::Array(
            new_units
                .map(serde_json::to_value)
                .collect::<Result<Vec<_>, _>>()
                .map_err(error)?,
        );
        let units = canonical_json(&unit_values);
        let delta = json!({"generation":d.generation,"roots":roots,"verifier_revision":verifier_revision,
            "raw":d.raw,"objects":d.objects,"witnesses":d.witnesses,"observations":d.observations,
            "withdrawals":d.withdrawals,"cover":d.cover,"effective":d.effective,"new_units":unit_values});
        let fold = sha256_hex(canonical_json(&json!([previous, delta])).as_bytes());
        conn.execute(
            "INSERT INTO cert VALUES(?1,?2,?3,?4,?5,?6)",
            [
                &d.generation,
                &canonical_json(&roots),
                &sha256_hex(units.as_bytes()),
                &i64::try_from(unit_count).map_err(error)?,
                &fold,
                &verifier_revision,
            ],
        )?;
        conn.execute(
            "UPDATE meta SET certified_through_generation=?1,fold_digest=?2 WHERE singleton=1",
            [&d.generation, &fold],
        )?;
        self.check_storage()?;
        self.phase = Phase::Pending(Pending::new(
            token,
            d.generation,
            d.roots.each_ref().map(Into::into),
            None,
        )?);
        Ok(())
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn stage_raw_partition(
        &mut self,
        token: &str,
        metadata: &content_manifest::ManifestMetadata,
        roots: &impl content_manifest::ManifestRootSource,
        source: &super::source_audit::source_receipt::RawReceipt,
        work: Option<&WorkScope>,
    ) -> Result<(), B::Error> {
        if !self.is_raw_partition() {
            self.invalidate();
            return Err(error("来源分区格式不匹配").into());
        }
        self.stage_partial(token, metadata, roots, source, work)
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn stage_source_partitions(
        &mut self,
        token: &str,
        metadata: &content_manifest::ManifestMetadata,
        roots: &impl content_manifest::ManifestRootSource,
        source: &super::source_audit::source_receipt::RawReceipt,
        work: Option<&WorkScope>,
    ) -> Result<(), B::Error> {
        if !self.is_source_partitions() {
            self.invalidate();
            return Err(error("来源分区格式不匹配").into());
        }
        self.stage_partial(token, metadata, roots, source, work)
    }
    #[cfg(feature = "s_incremental")]
    fn stage_partial(
        &mut self,
        token: &str,
        metadata: &content_manifest::ManifestMetadata,
        roots: &impl content_manifest::ManifestRootSource,
        source: &super::source_audit::source_receipt::RawReceipt,
        work: Option<&WorkScope>,
    ) -> Result<(), B::Error> {
        let result = (|| -> Result<(), B::Error> {
            let through = self.partial_through()?;
            if through.checked_add(1) != Some(metadata.generation)
                || metadata.rule_revision != self.rule_revision
            {
                return Err(error("raw分区代号或规则不连续").into());
            }
            let range = source
                .check_raw_partition(token, metadata, roots)
                .map_err(sc::ContextError::into_message)?;
            let r = roots.view();
            let pending = Pending::new(
                token,
                metadata.generation,
                [
                    (&metadata.scope).into(),
                    r.generated_state,
                    r.publication_template,
                    r.delta_template,
                ],
                work,
            )?;
            self.check_storage()?;
            let conn = self
                .connection
                .as_ref()
                .ok_or_else(|| error("raw分区已丢弃"))?;
            conn.execute_batch("BEGIN IMMEDIATE")?;
            let mut next: i64 =
                conn.query_row("SELECT COALESCE(MAX(seq),-1)+1 FROM raw", [], |[v]| int(v))?;
            if usize::try_from(next).ok() != Some(range.start) || source.rows().len() != range.len()
            {
                return Err(error("raw来源后缀与已提交分区不衔接").into());
            }
            for row in source.rows() {
                if row.seq != next {
                    return Err(error("raw来源后缀不连续").into());
                }
                conn.execute(
                    "INSERT INTO raw VALUES(?1,?2,?3,?4)",
                    [
                        &row.seq,
                        &row.event_digest.as_slice(),
                        &row.bar_digest.as_slice(),
                        &metadata.generation,
                    ],
                )?;
                next = next.checked_add(1).ok_or_else(|| error("raw序号溢出"))?;
            }
            if self.is_source_partitions() {
                let rows = source
                    .business_rows()
                    .map_err(sc::ContextError::into_message)?;
                for row in &rows.objects {
                    conn.execute(
                        "INSERT INTO object VALUES(?1,?2,?3,?4,?5,?6,0,?7)",
                        [
                            &row.id.as_str(),
                            &row.record.as_slice(),
                            &row.first_generation,
                            &row.first_cut.as_str(),
                            &row.published_generation,
                            &row.sources.as_slice(),
                            &metadata.generation,
                        ],
                    )?;
                }
                for id in &rows.withdrawals {
                    let changed = conn.execute("UPDATE object SET withdrawn=1,certified_generation=?1 WHERE id=?2 AND withdrawn=0",
                        [&metadata.generation, &id.as_str()])?;
                    if changed != 1 {
                        return Err(error("来源分区撤回身份缺失或已撤回").into());
                    }
                }
                for (sql, values) in [
                    ("INSERT INTO witness VALUES(?1,?2,?3,?4)", &rows.witnesses),
                    (
                        "INSERT INTO observation VALUES(?1,?2,?3,?4)",
                        &rows.observations,
                    ),
                ] {
                    for row in values {
                        conn.execute(
                            sql,
                            [
                                &row.id.as_str(),
                                &row.record.as_slice(),
                                &row.sources.as_slice(),
                                &metadata.generation,
                            ],
                        )?;
                    }
                }
            }
            source
                .check_raw_partition(token, metadata, roots)
                .map_err(sc::ContextError::into_message)?;
            self.check_storage()?;
            self.phase = Phase::Pending(pending);
            Ok(())
        })();
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    pub(super) fn require_pending(&self) -> Result<(), B::Error> {
        if matches!(self.phase, Phase::Pending(_) | Phase::Bound(_)) {
            Ok(())
        } else {
            Err(error("发布缺本请求台账变化或台账已失效").into())
        }
    }
    pub(super) fn bind(
        &mut self,
        token: &str,
        m: &content_manifest::ManifestMetadata,
        roots: &impl content_manifest::ManifestRootSource,
    ) -> Result<(), B::Error> {
        self.bind_view(token, m.view(), roots.view())
    }
    pub(super) fn bind_view(
        &mut self,
        token: &str,
        m: content_manifest::MetadataView<'_>,
        v: content_manifest::RootsView<'_>,
    ) -> Result<(), B::Error> {
        let expected: [RefView<'_>; 4] = [
            m.scope,
            v.generated_state,
            v.publication_template,
            v.delta_template,
        ];
        let result = match &self.phase {
            Phase::Pending(p) | Phase::Bound(p)
                if p.token.as_str() == token
                    && p.generation == m.generation
                    && m.rule_revision == self.rule_revision =>
            {
                if p.roots.iter().zip(expected).all(|(a, b)| {
                    a.format == b.format()
                        && a.root == b.root()
                        && a.kind == b.kind()
                        && a.extent == b.extent()
                }) {
                    Ok(())
                } else {
                    Err(error("台账四根与本请求 prepare 不一致").into())
                }
            }
            _ => Err(error("台账请求、规则或代号不一致").into()),
        };
        if result.is_err() {
            self.invalidate();
            return result;
        }
        let phase = std::mem::replace(&mut self.phase, Phase::Stale);
        self.phase = match phase {
            Phase::Pending(p) | Phase::Bound(p) => Phase::Bound(p),
            _ => unreachable!(),
        };
        Ok(())
    }
    /// 只能由内容库 Commit 成功后的 owner 调用，发布前没有台账可见性。
    pub(super) fn content_committed(&mut self) -> Result<(), B::Error> {
        let result = (|| -> Result<(), B::Error> {
            if !matches!(self.phase, Phase::Bound(_)) {
                return Err(error("台账尚未绑定实际 prepare").into());
            }
            self.check_storage()?;
            self.store()?.execute_batch("COMMIT")?;
            self.check_storage()?;
            Ok(())
        })();
        if result.is_ok() {
            if let (
                Coverage::RawPartition { through } | Coverage::SourcePartitions { through },
                Phase::Bound(p),
            ) = (&mut self.coverage, &self.phase)
            {
                *through = p.generation;
            }
            self.phase = Phase::Ready;
        } else {
            self.invalidate();
        }
        result
    }
    /// #1481 在原件单代核验后调用。在线与冷折叠只有内容 Commit 所在位置不同。
    pub(super) fn fold_step(
        &mut self,
        token: &str,
        d: &GenerationDelta,
        m: &content_manifest::ManifestMetadata,
        roots: &impl content_manifest::ManifestRootSource,
    ) -> Result<(), B::Error> {
        self.stage(token, d)?;
        self.bind(token, m, roots)?;
        self.content_committed()
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    pub(crate) fn delta(
        m: &content_manifest::ManifestMetadata,
        r: &content_manifest::ManifestRoots,
    ) -> GenerationDelta {
        let g = m.generation;
        GenerationDelta {
            generation: g,
            roots: [
                m.scope.clone(),
                r.generated_state.clone(),
                r.publication_template.clone(),
                r.delta_template.clone(),
            ],
            verifier_revision: "TestOnly-storage-fixture/1".into(),
            raw: BTreeMap::from([(
                g - 1,
                RawFact {
                    event_digest: [g as u8; 32],
                    bar_digest: [2; 32],
                },
            )]),
            objects: BTreeMap::from([(
                "object-a".into(),
                ObjectFact {
                    record_digest: [3; 32],
                    first_generation: 1,
                    first_cut: "first-cut".into(),
                    published_generation: g,
                    source_digest: [4; 32],
                    withdrawn: g == 3,
                },
            )]),
            witnesses: BTreeMap::from([(
                "witness-a".into(),
                RelatedFact {
                    record_digest: [5; 32],
                    source_digest: [6; 32],
                },
            )]),
            observations: BTreeMap::from([(
                "observation-a".into(),
                RelatedFact {
                    record_digest: [7; 32],
                    source_digest: [8; 32],
                },
            )]),
            withdrawals: BTreeSet::new(),
            cover: BTreeMap::from([(
                g - 1,
                CoverChange::Group {
                    seq: g - 1,
                    object_id: "object-a".into(),
                },
            )]),
            effective: g,
            new_units: BTreeSet::from([
                Unit {
                    digest: [g as u8; 32],
                    flavor: Flavor::Value,
                    mode: Mode::Public,
                },
                Unit {
                    digest: [g as u8; 32],
                    flavor: Flavor::Value,
                    mode: Mode::Template,
                },
                Unit {
                    digest: [g as u8; 32],
                    flavor: Flavor::JsonInner,
                    mode: Mode::Public,
                },
            ]),
        }
    }
    pub(crate) fn ledger(db: &Path, name: &str, rule: &str) -> Ledger {
        Ledger::fresh(
            &db.with_file_name(name),
            db,
            Identity {
                rule_revision: rule.into(),
                content_identity: "TestOnly-same-content".into(),
                binary_commit: "test-only".into(),
            },
            64 * 1024 * 1024,
        )
        .unwrap()
    }
    pub(crate) fn dump(path: &Path) -> Vec<String> {
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let mut result = Vec::new();
        for table in [
            "meta",
            "raw",
            "object",
            "witness",
            "observation",
            "cover",
            "node",
            "cert",
        ] {
            let mut s = conn
                .prepare(&format!(
                    "SELECT * FROM {table} ORDER BY {}",
                    if table == "node" { "1,2,3" } else { "1" }
                ))
                .unwrap();
            let count = s.column_count();
            let rows = s
                .query_map([], |r| {
                    (0..count)
                        .map(|i| r.get::<_, rusqlite::types::Value>(i))
                        .collect::<rusqlite::Result<Vec<_>>>()
                })
                .unwrap();
            for row in rows {
                result.push(format!("{table}:{:?}", row.unwrap()));
            }
        }
        result
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn p6_raw_partition_cannot_reopen_or_append_to_complete_certified_ledger() {
        let (f, _, token, metadata, roots) = prepared::tests::p6_candidate();
        let db = f.db_path();
        let path = db.with_file_name("completed-before-raw.sqlite");
        let mut completed = ledger(&db, "completed-before-raw.sqlite", &metadata.rule_revision);
        completed
            .fold_step(&token, &delta(&metadata, &roots), &metadata, &roots)
            .unwrap();
        drop(completed);
        let before = dump(&path);
        let identity = Identity {
            rule_revision: metadata.rule_revision.clone(),
            content_identity: "TestOnly-same-content".into(),
            binary_commit: "test-only".into(),
        };
        assert!(Ledger::fresh_raw_partition(&path, &db, identity, 64 * 1024 * 1024).is_err());
        assert_eq!(dump(&path), before);
        let reader = Connection::open(&path).unwrap();
        assert_eq!(
            reader
                .query_row("SELECT certified_through_generation FROM meta", [], |r| r
                    .get::<_, i64>(
                    0
                ))
                .unwrap(),
            1
        );
        assert_eq!(
            reader
                .query_row("SELECT format FROM meta", [], |r| r.get::<_, String>(0))
                .unwrap(),
            FORMAT
        );
    }

    #[test]
    fn p6_ledger_component_fold_matches_online_rows_and_partition_keys() {
        let (f, _, token, mut m, r) = prepared::tests::p6_candidate();
        let db = f.db_path();
        let mut online = ledger(&db, "online.sqlite", &m.rule_revision);
        let mut folded = ledger(&db, "fold.sqlite", &m.rule_revision);
        for g in 1..=3 {
            m.generation = g;
            let d = delta(&m, &r);
            online.stage(&token, &d).unwrap();
            // Separate connection cannot observe any row of the pending generation.
            let reader = Connection::open(&online.path).unwrap();
            let through: i64 = reader
                .query_row("SELECT certified_through_generation FROM meta", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(through, g - 1);
            drop(reader);
            online.bind(&token, &m, &r).unwrap();
            online.content_committed().unwrap();
            folded.fold_step(&token, &d, &m, &r).unwrap();
            assert_eq!(dump(&online.path), dump(&folded.path));
        }
        let conn = online.connection.as_ref().unwrap().legacy();
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM node", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            9
        );
        assert_eq!(
            conn.query_row("SELECT withdrawn FROM object", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(online.through().unwrap(), 3);
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn p6_verified_units_difference_keeps_mode_and_first_generation() {
        let (f, _, token, mut metadata, mut roots) = prepared::tests::p6_candidate();
        let mut ledger = ledger(
            &f.db_path(),
            "verified-diff.sqlite",
            &metadata.rule_revision,
        );
        let mut nodes = sc::MemoryNodes::new();
        let root = sc::write_value(&mut nodes, &sc::Value::String("verified".into()))
            .unwrap()
            .content_ref();
        metadata.scope = root.clone();
        roots.generated_state = root.clone();
        roots.publication_template = root.clone();
        roots.delta_template = root.clone();
        let refs = [&root, &root, &root, &root].map(Into::into);
        let verified =
            QualifiedUnits::full(sc::closure_delta::verify_four_roots(&mut nodes, refs).unwrap())
                .unwrap();
        for generation in 1..=2 {
            metadata.generation = generation;
            let facts = delta(&metadata, &roots);
            ledger
                .stage_verified(&token, &facts, &verified, None, None)
                .unwrap();
            ledger.bind(&token, &metadata, &roots).unwrap();
            ledger.content_committed().unwrap();
        }
        let connection = ledger.connection.as_ref().unwrap().legacy();
        let counts: Vec<i64> = connection
            .prepare("SELECT new_units_count FROM cert ORDER BY generation")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(counts, [2, 0]);
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM node WHERE first_generation=1",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            2
        );
        for mode in [Mode::Public, Mode::Template] {
            assert!(ledger
                .contains_committed_unit(&Unit {
                    digest: root.digest().unwrap(),
                    flavor: Flavor::Value,
                    mode
                })
                .unwrap());
        }
        assert!(!ledger
            .contains_committed_unit(&Unit {
                digest: root.digest().unwrap(),
                flavor: Flavor::JsonInner,
                mode: Mode::Public
            })
            .unwrap());
    }

    /// #1419 M8D 3B：成功继承锁（L7/L8/P4/P5）。完整核验是 oracle；Δ 核验以台账已提交
    /// 合格单位为前沿，经同一 `stage_verified` 入口。
    #[cfg(feature = "s_incremental")]
    mod m8d_success {
        use super::*;
        use sc::closure_delta as cd;

        /// 只计节点读，读本身委托给原内存库。
        struct Counting<'a> {
            nodes: &'a mut sc::MemoryNodes,
            reads: u64,
        }
        impl sc::NodeSource for Counting<'_> {
            fn read(&mut self, digest: &sc::NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
                self.reads += 1;
                self.nodes.read(digest)
            }
        }
        /// 只计前沿点查（每次点查 = 资格逐字段核 + 完整键 EXISTS 两条台账 SQL）。
        struct CountingFrontier<'a> {
            inner: QualifiedSuccessView<'a>,
            lookups: std::cell::Cell<u64>,
        }
        impl cd::Frontier for CountingFrontier<'_> {
            fn contains(&self, unit: cd::Unit) -> Result<bool, sc::CodecError> {
                self.lookups.set(self.lookups.get() + 1);
                cd::Frontier::contains(&self.inner, unit)
            }
        }
        fn units(q: &QualifiedUnits<'_>) -> BTreeSet<Unit> {
            let mut out = BTreeSet::new();
            q.verified
                .try_for_each_unit(|u| {
                    out.insert(Unit::of_closure(u));
                    Ok::<_, ()>(())
                })
                .unwrap();
            out
        }
        fn set_roots(
            m: &mut content_manifest::ManifestMetadata,
            roots: &mut content_manifest::ManifestRoots,
            r: &ContentRef,
        ) {
            m.scope = r.clone();
            roots.generated_state = r.clone();
            roots.publication_template = r.clone();
            roots.delta_template = r.clone();
        }
        fn refs(r: &ContentRef) -> [RefView<'_>; 4] {
            [r, r, r, r].map(Into::into)
        }
        fn full(nodes: &mut sc::MemoryNodes, r: &ContentRef) -> (QualifiedUnits<'static>, u64) {
            let mut c = Counting { nodes, reads: 0 };
            let v = cd::verify_four_roots(&mut c, refs(r)).unwrap();
            (QualifiedUnits::full(v).unwrap(), c.reads)
        }
        fn commit(
            l: &mut Ledger,
            token: &str,
            m: &content_manifest::ManifestMetadata,
            roots: &content_manifest::ManifestRoots,
            units: &QualifiedUnits<'_>,
            prior: Option<&Qualification>,
        ) {
            l.stage_verified(token, &delta(m, roots), units, None, prior)
                .unwrap();
            l.bind(token, m, roots).unwrap();
            l.content_committed().unwrap();
        }
        fn values(nodes: &mut sc::MemoryNodes) -> (Vec<ContentRef>, ContentRef) {
            use sc::{JsonValue, Value};
            // 写入但 g1 不可达；g2 首次可达，必须作为新单位重核。
            let latent = sc::write_value(nodes, &Value::Bool(false))
                .unwrap()
                .content_ref();
            let g = [
                Value::Object(vec![
                    ("a".into(), Value::Null),
                    ("t".into(), Value::JsonText(Box::new(JsonValue::Null))),
                ]),
                // 换位置：原 Null/JsonText 子树移到新容器，新增 latent 引用。
                Value::Object(vec![
                    ("b".into(), Value::Null),
                    (
                        "a".into(),
                        Value::Array(vec![Value::Null, Value::edge_of_ref(&latent).unwrap()]),
                    ),
                    ("t".into(), Value::JsonText(Box::new(JsonValue::Null))),
                ]),
                Value::Array(vec![
                    Value::Object(vec![(
                        "t".into(),
                        Value::JsonText(Box::new(JsonValue::Bool(false))),
                    )]),
                    Value::String("x".into()),
                    Value::JsonText(Box::new(JsonValue::Null)),
                ]),
            ];
            let mut out: Vec<ContentRef> = g
                .iter()
                .map(|v| sc::write_value(nodes, v).unwrap().content_ref())
                .collect();
            // 第四代不变：Δ 必须零新单位。
            out.push(out[2].clone());
            (out, latent)
        }

        #[test]
        fn h1_m8d_success_delta_and_full_canonical_diff() {
            let (f, _, token, mut m, mut roots) = prepared::tests::p6_candidate();
            let mut a = ledger(&f.db_path(), "m8d-full.sqlite", &m.rule_revision);
            let mut b = ledger(&f.db_path(), "m8d-delta.sqlite", &m.rule_revision);
            let mut nodes = sc::MemoryNodes::new();
            let (gens, latent) = values(&mut nodes);
            let mut receipts = Vec::new();
            for (i, r) in gens.iter().enumerate() {
                let generation = i as i64 + 1;
                m.generation = generation;
                set_roots(&mut m, &mut roots, r);
                let (whole, full_reads) = full(&mut nodes, r);
                if generation == 1 {
                    // 空 g1：没有已提交资格，前沿为空，两台账同走完整核验。
                    commit(&mut a, &token, &m, &roots, &whole, None);
                    commit(&mut b, &token, &m, &roots, &whole, None);
                    receipts.push((generation, full_reads, full_reads, units(&whole).len(), 0));
                } else {
                    let qa = a.qualification().unwrap();
                    let qb = b.qualification().unwrap();
                    assert_eq!(qa, qb);
                    let mut c = Counting {
                        nodes: &mut nodes,
                        reads: 0,
                    };
                    let d = b.verify_delta(&mut c, refs(r), &qb).unwrap();
                    let delta_reads = c.reads;
                    assert!(d.verified.frontier_used());
                    let mut oracle = BTreeSet::new();
                    for u in units(&whole) {
                        if !a.contains_committed_unit(&u).unwrap() {
                            oracle.insert(u);
                        }
                    }
                    assert_eq!(units(&d), oracle, "g{generation} Δ单位与完整核验差集不同");
                    if generation == 2 {
                        for mode in [Mode::Public, Mode::Template] {
                            assert!(oracle.contains(&Unit {
                                digest: latent.digest().unwrap(),
                                flavor: Flavor::Value,
                                mode,
                            }));
                        }
                    }
                    if generation == 4 {
                        assert!(oracle.is_empty());
                    }
                    assert!(delta_reads < full_reads, "g{generation} Δ未止步");
                    let counted = CountingFrontier {
                        inner: QualifiedSuccessView {
                            ledger: &b,
                            qualification: &qb,
                        },
                        lookups: std::cell::Cell::new(0),
                    };
                    cd::verify_four_roots_from(&mut nodes, refs(r), &counted).unwrap();
                    receipts.push((
                        generation,
                        full_reads,
                        delta_reads,
                        oracle.len(),
                        counted.lookups.get(),
                    ));
                    commit(&mut a, &token, &m, &roots, &whole, Some(&qa));
                    commit(&mut b, &token, &m, &roots, &d, Some(&qb));
                }
                assert_eq!(dump(&a.path), dump(&b.path), "g{generation} 台账不同");
            }
            for (g, full_reads, delta_reads, new_units, lookups) in &receipts {
                println!(
                    "M8D_3B_RECEIPT g={g} full_node_reads={full_reads} delta_node_reads={delta_reads} new_units={new_units} frontier_lookups={lookups}"
                );
            }
        }

        const M8D_LEDGER_EXISTS: &str =
            "SELECT EXISTS(SELECT 1 FROM node WHERE digest=?1 AND flavor=?2 AND mode=?3 AND first_generation<=?4)";
        const M8D_LEDGER_QUALIFICATION: &str = "SELECT c.roots, c.fold_digest, c.verifier_revision, m.fold_digest, m.content_identity, m.binary_commit, m.rule_revision FROM cert c, meta m WHERE c.generation=?1 AND m.singleton=1";
        const M8D_LEDGER_THROUGH: &str =
            "SELECT certified_through_generation FROM meta WHERE singleton=1";
        /// #1419 M8D L9（台账层）：test-only SQLITE_TRACE_STMT 按原语句文本计数。实测冻结：每次
        /// 前沿点查 = 完整键 EXISTS 1 + 资格行（cert×meta）1 + 认证水位 2，共 4 条；verify_delta
        /// 签发另有资格行 1 + 水位 1。点查之外无别的台账 SQL；stage/bind/内容提交逐桶另列收据。
        /// 不改台账连接配置，回调随 guard 卸载。
        #[test]
        fn m8d_metadata_node_f_and_qualified_lookup_receipts() {
            type Counts = std::cell::RefCell<BTreeMap<String, u64>>;
            extern "C" fn count(
                kind: std::ffi::c_uint,
                ctx: *mut std::ffi::c_void,
                _stmt: *mut std::ffi::c_void,
                sql: *mut std::ffi::c_void,
            ) -> std::ffi::c_int {
                if kind == rusqlite::ffi::SQLITE_TRACE_STMT {
                    // SAFETY: ctx 指向安装期间不移动的 Box<Counts>；sql 为 NUL 结尾原文。
                    let counts = unsafe { &*(ctx as *const Counts) };
                    let text = unsafe { std::ffi::CStr::from_ptr(sql as *const std::ffi::c_char) };
                    *counts
                        .borrow_mut()
                        .entry(text.to_string_lossy().into_owned())
                        .or_default() += 1;
                }
                0
            }
            struct Trace(*mut rusqlite::ffi::sqlite3, Box<Counts>);
            impl Trace {
                fn take(&self) -> BTreeMap<String, u64> {
                    std::mem::take(&mut *self.1.borrow_mut())
                }
            }
            impl Drop for Trace {
                fn drop(&mut self) {
                    // SAFETY: 同一台账连接仍活；mask 0 卸载回调。
                    unsafe {
                        rusqlite::ffi::sqlite3_trace_v2(self.0, 0, None, std::ptr::null_mut())
                    };
                }
            }
            let (f, _, token, mut m, mut roots) = prepared::tests::p6_candidate();
            let mut b = ledger(&f.db_path(), "m8d-l9.sqlite", &m.rule_revision);
            let trace = {
                let counts: Box<Counts> = Box::default();
                // SAFETY: 句柄来自活台账连接；回调上下文由 Trace 持有到卸载。
                let db = unsafe { b.connection.as_ref().unwrap().legacy().handle() };
                let rc = unsafe {
                    rusqlite::ffi::sqlite3_trace_v2(
                        db,
                        rusqlite::ffi::SQLITE_TRACE_STMT,
                        Some(count),
                        &*counts as *const Counts as *mut std::ffi::c_void,
                    )
                };
                assert_eq!(rc, rusqlite::ffi::SQLITE_OK);
                Trace(db, counts)
            };
            let mut nodes = sc::MemoryNodes::new();
            let (gens, _) = values(&mut nodes);
            for (i, r) in gens.iter().enumerate() {
                let generation = i as i64 + 1;
                m.generation = generation;
                set_roots(&mut m, &mut roots, r);
                trace.take();
                if generation == 1 {
                    let (whole, _) = full(&mut nodes, r);
                    commit(&mut b, &token, &m, &roots, &whole, None);
                    println!(
                        "M8D_L9_LEDGER g=1 phase=commit statements={:?}",
                        trace.take()
                    );
                    continue;
                }
                let q = b.qualification().unwrap();
                let qualification_sql = trace.take();
                let mut c = Counting {
                    nodes: &mut nodes,
                    reads: 0,
                };
                let d = b.verify_delta(&mut c, refs(r), &q).unwrap();
                let node_reads = c.reads;
                let lookup_sql = trace.take();
                // 同一输入再经计数前沿走一遍，只为取逻辑点查次数；其 SQL 不计入上面的桶。
                let counted = CountingFrontier {
                    inner: QualifiedSuccessView {
                        ledger: &b,
                        qualification: &q,
                    },
                    lookups: std::cell::Cell::new(0),
                };
                cd::verify_four_roots_from(&mut nodes, refs(r), &counted).unwrap();
                let lookups = counted.lookups.get();
                trace.take();
                println!(
                    "M8D_L9_LEDGER g={generation} phase=qualification statements={qualification_sql:?}"
                );
                println!(
                    "M8D_L9_LEDGER g={generation} phase=lookup node_reads={node_reads} frontier_lookups={lookups} statements={lookup_sql:?}"
                );
                let expected: BTreeMap<String, u64> = [
                    (M8D_LEDGER_EXISTS, lookups),
                    (M8D_LEDGER_QUALIFICATION, lookups + 1),
                    (M8D_LEDGER_THROUGH, 2 * lookups + 1),
                ]
                .into_iter()
                .filter(|(_, n)| *n > 0)
                .map(|(sql, n)| (sql.to_owned(), n))
                .collect();
                assert_eq!(
                    lookup_sql, expected,
                    "g{generation}：点查各 1 EXISTS、1 资格行、2 水位，签发另 1 资格行、1 水位，别无台账 SQL"
                );
                commit(&mut b, &token, &m, &roots, &d, Some(&q));
                println!(
                    "M8D_L9_LEDGER g={generation} phase=commit statements={:?}",
                    trace.take()
                );
            }
        }

        #[test]
        fn h1_m8d_success_ledger_commit_failure_and_cold_rebuild_diff() {
            let (f, _, token, mut m, mut roots) = prepared::tests::p6_candidate();
            let mut online = ledger(&f.db_path(), "m8d-online.sqlite", &m.rule_revision);
            let mut nodes = sc::MemoryNodes::new();
            let (gens, _) = values(&mut nodes);
            // 在线：Δ 逐代；第三代绑定失败使台账失效，失效后不得再签资格或点查。
            for (i, r) in gens.iter().enumerate().take(3) {
                m.generation = i as i64 + 1;
                set_roots(&mut m, &mut roots, r);
                if m.generation == 1 {
                    let (whole, _) = full(&mut nodes, r);
                    commit(&mut online, &token, &m, &roots, &whole, None);
                    continue;
                }
                let q = online.qualification().unwrap();
                let d = online.verify_delta(&mut nodes, refs(r), &q).unwrap();
                online
                    .stage_verified(&token, &delta(&m, &roots), &d, None, Some(&q))
                    .unwrap();
                if m.generation == 3 {
                    assert!(online.bind("different-request", &m, &roots).is_err());
                    assert!(online.connection.is_none());
                    assert!(online.qualification().is_err());
                    assert!(online.verify_delta(&mut nodes, refs(r), &q).is_err());
                    break;
                }
                online.bind(&token, &m, &roots).unwrap();
                online.content_committed().unwrap();
            }
            // 冷重建：从空 g1 起每代走同一 Δ 步；结果须与一路完整核验的台账逐字相同。
            let mut cold = ledger(&f.db_path(), "m8d-cold.sqlite", &m.rule_revision);
            let mut oracle = ledger(&f.db_path(), "m8d-oracle.sqlite", &m.rule_revision);
            for (i, r) in gens.iter().enumerate() {
                m.generation = i as i64 + 1;
                set_roots(&mut m, &mut roots, r);
                let (whole, _) = full(&mut nodes, r);
                let prior_o = (m.generation > 1).then(|| oracle.qualification().unwrap());
                commit(&mut oracle, &token, &m, &roots, &whole, prior_o.as_ref());
                match (m.generation > 1).then(|| cold.qualification().unwrap()) {
                    None => commit(&mut cold, &token, &m, &roots, &whole, None),
                    Some(q) => {
                        let d = cold.verify_delta(&mut nodes, refs(r), &q).unwrap();
                        commit(&mut cold, &token, &m, &roots, &d, Some(&q));
                    }
                }
                assert_eq!(dump(&cold.path), dump(&oracle.path));
            }
        }

        fn committed_g1(
            name: &str,
        ) -> (
            prepared::tests::Fixture,
            Ledger,
            sc::MemoryNodes,
            String,
            content_manifest::ManifestMetadata,
            content_manifest::ManifestRoots,
            Vec<ContentRef>,
        ) {
            let (f, _, token, mut m, mut roots) = prepared::tests::p6_candidate();
            let mut l = ledger(&f.db_path(), name, &m.rule_revision);
            let mut nodes = sc::MemoryNodes::new();
            let (gens, _) = values(&mut nodes);
            m.generation = 1;
            set_roots(&mut m, &mut roots, &gens[0]);
            let (whole, _) = full(&mut nodes, &gens[0]);
            commit(&mut l, &token, &m, &roots, &whole, None);
            m.generation = 2;
            set_roots(&mut m, &mut roots, &gens[1]);
            (f, l, nodes, token, m, roots, gens)
        }

        #[test]
        fn m8d_success_complete_unit_context_position_and_first_reachable() {
            // 完整单位 (digest,Flavor,Mode) 点查：同摘要异上下文/异模式不互借；首达旧节点重核；
            // 序列不是单位。逐代差集对拍由 h1_m8d_success_delta_and_full_canonical_diff 承担。
            let (_f, l, mut nodes, _, _, _, gens) = committed_g1("m8d-l7.sqlite");
            let q = l.qualification().unwrap();
            let d = l.verify_delta(&mut nodes, refs(&gens[1]), &q).unwrap();
            let got = units(&d);
            for u in &got {
                assert!(!l.contains_committed_unit(u).unwrap());
            }
            // 每个新单位在 Public 与 Template 两模式都出现：模式独立认证。
            for u in &got {
                let twin = Unit {
                    mode: match u.mode {
                        Mode::Public => Mode::Template,
                        Mode::Template => Mode::Public,
                    },
                    ..u.clone()
                };
                assert!(got.contains(&twin), "模式孪生单位缺失");
            }
            // 上下文切换：g3 把 Bool(false) 放进 JsonText 内层；未提交的 JsonInner 单位
            // 必须出现在 Δ 中，不因同摘要其他上下文而免核（台账此处只提交了 g1）。
            let full_g3 = {
                let mut c = Counting {
                    nodes: &mut nodes,
                    reads: 0,
                };
                QualifiedUnits::full(cd::verify_four_roots(&mut c, refs(&gens[2])).unwrap())
                    .unwrap()
            };
            let inner: BTreeSet<Unit> = units(&full_g3)
                .into_iter()
                .filter(|u| matches!(u.flavor, Flavor::JsonInner))
                .collect();
            assert!(!inner.is_empty(), "夹具未产生 JsonInner 单位");
            let d3 = l.verify_delta(&mut nodes, refs(&gens[2]), &q).unwrap();
            for u in inner {
                if !l.contains_committed_unit(&u).unwrap() {
                    assert!(units(&d3).contains(&u));
                }
            }
        }

        #[test]
        fn m8d_success_partial_failure_and_ledger_phase_never_promote() {
            // (a) 前沿止步的结果不能冒充完整核验。
            {
                let mut nodes = sc::MemoryNodes::new();
                let r = sc::write_value(&mut nodes, &sc::Value::Null)
                    .unwrap()
                    .content_ref();
                let v =
                    cd::verify_four_roots_from(&mut nodes, refs(&r), &cd::EmptyFrontier).unwrap();
                assert!(QualifiedUnits::full(v).is_err());
            }
            // (b) miss 的损坏节点：Δ 与完整核验同错误码失败，台账不变。
            {
                let (_f, l, mut nodes, _, _, _, _) = committed_g1("m8d-l8-corrupt.sqlite");
                let fresh = sc::write_value(&mut nodes, &sc::Value::String("fresh".into()))
                    .unwrap()
                    .content_ref();
                let r = sc::write_value(
                    &mut nodes,
                    &sc::Value::Array(vec![sc::Value::edge_of_ref(&fresh).unwrap()]),
                )
                .unwrap()
                .content_ref();
                let digest = fresh.digest().unwrap();
                let (tag, _) = nodes.get(&digest).unwrap();
                nodes.insert_unchecked(digest, tag, b"bad".to_vec());
                let before = dump(&l.path);
                let q = l.qualification().unwrap();
                let view = QualifiedSuccessView {
                    ledger: &l,
                    qualification: &q,
                };
                let delta_err = cd::verify_four_roots_from(&mut nodes, refs(&r), &view)
                    .err()
                    .unwrap();
                let full_err = cd::verify_four_roots(&mut nodes, refs(&r)).err().unwrap();
                assert_eq!(delta_err.code(), full_err.code());
                assert!(l.verify_delta(&mut nodes, refs(&r), &q).is_err());
                assert_eq!(before, dump(&l.path));
            }
            // (c) 未提交（pending/bound）时不能签发或点查；失效台账同样拒绝。
            {
                let (_f, mut l, mut nodes, token, m, roots, gens) =
                    committed_g1("m8d-l8-pending.sqlite");
                let q = l.qualification().unwrap();
                let d = l.verify_delta(&mut nodes, refs(&gens[1]), &q).unwrap();
                l.stage_verified(&token, &delta(&m, &roots), &d, None, Some(&q))
                    .unwrap();
                assert!(l.verify_delta(&mut nodes, refs(&gens[1]), &q).is_err());
                let view = QualifiedSuccessView {
                    ledger: &l,
                    qualification: &q,
                };
                // 前沿点查失败即整体失败，不当 miss 退回完整核验。
                assert!(cd::verify_four_roots_from(&mut nodes, refs(&gens[1]), &view).is_err());
                l.bind(&token, &m, &roots).unwrap();
                assert!(l.verify_delta(&mut nodes, refs(&gens[1]), &q).is_err());
                l.content_committed().unwrap();
                // (d) 推进一代后旧资格过期。
                assert!(l.verify_delta(&mut nodes, refs(&gens[2]), &q).is_err());
                let fresh = l.qualification().unwrap();
                assert_ne!(fresh, q);
                assert!(l.verify_delta(&mut nodes, refs(&gens[2]), &fresh).is_ok());
            }
            // (e) 绑定资格必须等于 prior：缺 prior 或换资格都拒绝，失败使台账失效且文件不变。
            for case in 0..2 {
                let (_f, mut l, mut nodes, token, m, roots, gens) =
                    committed_g1(&format!("m8d-l8-prior-{case}.sqlite"));
                let q = l.qualification().unwrap();
                let d = l.verify_delta(&mut nodes, refs(&gens[1]), &q).unwrap();
                let before = dump(&l.path);
                let mut other = q.clone();
                other.fold_digest = "0".repeat(64);
                let prior = if case == 0 { None } else { Some(&other) };
                assert!(l
                    .stage_verified(&token, &delta(&m, &roots), &d, None, prior)
                    .is_err());
                assert!(l.connection.is_none());
                assert_eq!(before, dump(&l.path));
            }
        }
    }

    #[test]
    fn p6_ledger_duplicate_unit_rolls_back_every_partition_and_discards() {
        let (f, _, token, mut m, r) = prepared::tests::p6_candidate();
        let mut l = ledger(&f.db_path(), "ledger.sqlite", &m.rule_revision);
        let first = delta(&m, &r);
        l.fold_step(&token, &first, &m, &r).unwrap();
        let before = dump(&l.path);
        m.generation = 2;
        let mut second = delta(&m, &r);
        second.new_units = first.new_units;
        assert!(l.stage(&token, &second).is_err());
        assert!(l.connection.is_none());
        assert!(matches!(l.phase, Phase::Stale));
        assert_eq!(before, dump(&l.path));
        assert!(l.stage(&token, &second).is_err());
    }
    #[test]
    fn p6_ledger_fresh_never_trusts_or_overwrites_previous_file() {
        let (f, _, token, m, r) = prepared::tests::p6_candidate();
        let db = f.db_path();
        let mut l = ledger(&db, "ledger.sqlite", &m.rule_revision);
        l.fold_step(&token, &delta(&m, &r), &m, &r).unwrap();
        let before = dump(&l.path);
        assert!(Ledger::fresh(
            &l.path,
            &db,
            Identity {
                rule_revision: m.rule_revision.clone(),
                content_identity: "other".into(),
                binary_commit: "other".into()
            },
            64 * 1024 * 1024
        )
        .is_err());
        assert_eq!(dump(&l.path), before);
    }
    #[test]
    fn p6_ledger_pending_request_roots_and_sequence_are_bound() {
        let (f, _, token, m, r) = prepared::tests::p6_candidate();
        for mutation in 0..4 {
            let mut l = ledger(
                &f.db_path(),
                &format!("bad-{mutation}.sqlite"),
                &m.rule_revision,
            );
            let mut d = delta(&m, &r);
            if mutation == 0 {
                d.generation = 2;
                assert!(l.stage(&token, &d).is_err());
                continue;
            }
            if mutation == 1 {
                d.raw = BTreeMap::from([(
                    2,
                    RawFact {
                        event_digest: [0; 32],
                        bar_digest: [0; 32],
                    },
                )]);
                assert!(l.stage(&token, &d).is_err());
                continue;
            }
            if mutation == 2 {
                d.roots.swap(0, 1);
            }
            l.stage(&token, &d).unwrap();
            assert!(l
                .bind(
                    if mutation == 3 {
                        "different-request"
                    } else {
                        &token
                    },
                    &m,
                    &r
                )
                .is_err());
            assert!(l.connection.is_none());
            let c = Connection::open(&l.path).unwrap();
            assert_eq!(
                c.query_row("SELECT COUNT(*) FROM cert", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn p6_two_object_generations_commit_disjoint_closure_deltas() {
        use newchan_rust::s_content::{self as sc, closure_delta as cd};
        fn unit(u: cd::Unit) -> Unit {
            Unit {
                digest: u.digest,
                flavor: match u.context {
                    cd::Context::Value => Flavor::Value,
                    cd::Context::JsonInner => Flavor::JsonInner,
                },
                mode: match u.mode {
                    cd::VerificationMode::Public => Mode::Public,
                    cd::VerificationMode::Template => Mode::Template,
                },
            }
        }
        struct Committed<'a>(&'a Ledger);
        impl cd::Frontier for Committed<'_> {
            fn contains(&self, u: cd::Unit) -> Result<bool, sc::CodecError> {
                self.0
                    .contains_committed_unit(&unit(u))
                    .map_err(|e| sc::CodecError::new(sc::code::SOURCE, e))
            }
        }
        let (f, _, token, mut m, _) = prepared::tests::p6_candidate();
        let mut l = ledger(&f.db_path(), "object-generations.sqlite", &m.rule_revision);
        let mut nodes = sc::MemoryNodes::new();
        let mut prior = BTreeSet::new();
        for (generation, value) in [(1, sc::Value::Null), (2, sc::Value::Bool(true))] {
            let r = sc::write_value(&mut nodes, &sc::Value::Object(vec![("k".into(), value)]))
                .unwrap()
                .content_ref();
            m.generation = generation;
            m.scope = r.clone();
            let roots = content_manifest::ManifestRoots {
                generated_state: r.clone(),
                publication_template: r.clone(),
                delta_template: r.clone(),
            };
            let probe = cd::probe_four_roots(&mut nodes, [(&r).into(); 4], &Committed(&l)).unwrap();
            let mut d = delta(&m, &roots);
            d.new_units.clear();
            probe.for_each_unit(|u| {
                d.new_units.insert(unit(u));
            });
            assert_eq!(d.new_units.len(), if generation == 1 { 6 } else { 4 });
            assert!(d.new_units.is_disjoint(&prior));
            l.stage(&token, &d).unwrap();
            assert!(l.committed_through().is_err());
            l.bind(&token, &m, &roots).unwrap();
            l.content_committed().unwrap();
            assert_eq!(l.committed_through().unwrap(), generation);
            for u in &d.new_units {
                assert!(l.contains_committed_unit(u).unwrap());
            }
            prior.extend(d.new_units);
        }
        assert_eq!(prior.len(), 10);
        let conn = Connection::open(&l.path).unwrap();
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM node", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            10
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM cert", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
    #[test]
    fn p6_ledger_committed_queries_reject_pending_bound_and_stale() {
        let (f, _, token, m, r) = prepared::tests::p6_candidate();
        let mut l = ledger(&f.db_path(), "frontier.sqlite", &m.rule_revision);
        let d = delta(&m, &r);
        let unit = d.new_units.iter().next().unwrap();
        assert_eq!(l.committed_through().unwrap(), 0);
        assert!(!l.contains_committed_unit(unit).unwrap());
        l.stage(&token, &d).unwrap();
        assert!(l.committed_through().is_err());
        assert!(l.contains_committed_unit(unit).is_err());
        l.bind(&token, &m, &r).unwrap();
        assert!(l.contains_committed_unit(unit).is_err());
        l.content_committed().unwrap();
        assert_eq!(l.committed_through().unwrap(), 1);
        assert!(l.contains_committed_unit(unit).unwrap());
        let absent = Unit {
            digest: unit.digest,
            flavor: Flavor::JsonInner,
            mode: Mode::Template,
        };
        assert!(!l.contains_committed_unit(&absent).unwrap());
        l.invalidate();
        assert!(l.contains_committed_unit(unit).is_err());
    }
    #[test]
    fn p6_ledger_budget_counts_content_and_sidecar_together() {
        let (f, _, _, m, _) = prepared::tests::p6_candidate();
        let db = f.db_path();
        let path = db.with_file_name("small.sqlite");
        let cap = database_bytes(&db).unwrap();
        assert!(Ledger::fresh(
            &path,
            &db,
            Identity {
                rule_revision: m.rule_revision,
                content_identity: "content".into(),
                binary_commit: "test".into()
            },
            cap
        )
        .is_err());
        assert!(!path.exists());
    }

    #[cfg(feature = "s_incremental")]
    fn charged(l: &Ledger<Charged>) -> &Charged {
        l.connection.as_ref().expect("live ledger")
    }

    #[cfg(feature = "s_incremental")]
    fn generation(
        g: i64,
    ) -> (
        content_manifest::ManifestMetadata,
        content_manifest::ManifestRoots,
    ) {
        use newchan_rust::s_content as sc;
        let mut nodes = sc::MemoryNodes::new();
        let r = sc::write_value(
            &mut nodes,
            &sc::Value::Object(vec![("g".into(), sc::Value::Bool(g % 2 == 0))]),
        )
        .unwrap()
        .content_ref();
        let m = content_manifest::ManifestMetadata {
            session_id: "charged-ledger".into(),
            session_generation: "1".into(),
            writer_epoch: "1".into(),
            generation: g,
            base_cut: "base".into(),
            structure_cut: "structure".into(),
            input_frontier: g,
            source_commit: content_manifest::SourceCommit {
                source_namespace: "ns".into(),
                source_epoch: "1".into(),
                message_id: format!("m{g}"),
                payload_hash: "h".into(),
                producer_id: "p".into(),
                producer_epoch: "1".into(),
                clock_event_id: "c".into(),
                attempt_index: 0,
            },
            clock_plan_hash: "plan".into(),
            semantic_commit_ns: "0".into(),
            profile_id: "profile".into(),
            profile_hash: "profile-hash".into(),
            rule_revision: "charged-rule".into(),
            catalog_revision: "catalog".into(),
            scope: r.clone(),
        };
        let roots = content_manifest::ManifestRoots {
            generated_state: r.clone(),
            publication_template: r.clone(),
            delta_template: r,
        };
        (m, roots)
    }

    #[cfg(feature = "s_incremental")]
    static LEDGER_STATEMENTS: std::sync::atomic::AtomicUsize =
        std::sync::atomic::AtomicUsize::new(0);

    #[cfg(feature = "s_incremental")]
    unsafe extern "C" fn count_ledger_statement(
        _: std::ffi::c_uint,
        _: *mut std::ffi::c_void,
        _: *mut std::ffi::c_void,
        _: *mut std::ffi::c_void,
    ) -> std::ffi::c_int {
        LEDGER_STATEMENTS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        0
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    #[ignore = "fresh process only: installs the global SQLite allocator; run alone with --ignored --exact"]
    fn charged_ledger_same_parent_phase_machine_and_snapshot_interleave() {
        use super::super::audit_owner::allocation_probe::Window;
        use super::super::audit_owner::native::Budget;
        use newchan_rust::s_content::capture_allocation::scratch::ScratchFailure;
        use newchan_rust::s_content::capture_allocation::sqlite_parent::SqliteParentAllocator;
        use rusqlite::ffi;
        use std::sync::atomic::Ordering;
        const LIMIT: usize = 64 << 20;
        const ROOM: usize = 16 << 20;
        const STORE: u64 = 64 * 1024 * 1024;

        // SAFETY: this test runs alone in a fresh process before any SQLite use.
        let allocator = unsafe { SqliteParentAllocator::install() }.expect("install before init");
        let root = Budget::fresh(LIMIT).unwrap();
        let base = root.live();
        let mut current = root.bind_frozen(1, LIMIT, 0, 0).unwrap();
        let bind = |b: &Budget| {
            let (parent, origin) = b.sqlite_parent();
            Rc::new(allocator.bind(parent, origin).expect("bind"))
        };
        let dir = std::env::temp_dir().join(format!("charged-ledger-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c_path = |name: &str| std::ffi::CString::new(dir.join(name).to_str().unwrap()).unwrap();
        let content = dir.join("content.sqlite3");
        std::fs::File::create(&content).unwrap();
        let identity = || Identity {
            rule_revision: "charged-rule".into(),
            content_identity: "charged-content".into(),
            binary_commit: "charged-binary".into(),
        };
        let token = "charged-token";
        let count = |b: &SqliteBinding, conn: &ChargedConnection, sql: &str| {
            b.run(conn, |q| q.query_row(sql, [], |[v]| v.as_i64()))
                .expect("count")
                .expect("integer")
        };

        // A：快照读事务存续期间开受管台账、完成一代，再回到同一快照事务继续读。
        let a = bind(&current);
        let snap = a.open(&c_path("snap.sqlite3")).unwrap();
        a.run(&snap, |q| {
            q.execute_batch("CREATE TABLE s(x INTEGER); INSERT INTO s VALUES(1),(2),(3);")
        })
        .unwrap();
        a.run(&snap, |q| q.execute_batch("BEGIN")).unwrap();
        assert_eq!(count(&a, &snap, "SELECT count(*) FROM s"), 3);

        let path1 = c_path("ledger-a.sqlite3");
        let mut l1 = Ledger::fresh_charged(&path1, &content, identity(), STORE, a.clone()).unwrap();
        let pragma = |l: &Ledger<Charged>, sql: &str| {
            charged(l)
                .run(|q| {
                    q.query_row(sql, [], |[v]| match v {
                        ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
                        other => format!("{:?}", other.as_i64()),
                    })
                })
                .unwrap()
        };
        assert_eq!(pragma(&l1, "PRAGMA journal_mode"), "delete");
        assert_eq!(pragma(&l1, "PRAGMA synchronous"), "Ok(2)");
        assert_eq!(pragma(&l1, "PRAGMA foreign_keys"), "Ok(1)");
        assert_eq!(pragma(&l1, "PRAGMA busy_timeout"), "Ok(0)");
        assert_eq!(pragma(&l1, "PRAGMA page_size"), "Ok(4096)");
        assert_eq!(
            pragma(&l1, "PRAGMA max_page_count"),
            format!("Ok({})", STORE / 4096)
        );
        assert!(a.live_bytes() > 0);
        let live = a.live_bytes();
        assert!(Ledger::fresh_charged(&path1, &content, identity(), STORE, a.clone()).is_err());
        assert_eq!(a.live_bytes(), live);

        let (m1, r1) = generation(1);
        let d1 = delta(&m1, &r1);
        l1.stage(token, &d1).unwrap();
        assert!(l1.committed_through().is_err());
        let peek = a.open(&c_path("ledger-a.sqlite3")).unwrap();
        let through = "SELECT certified_through_generation FROM meta WHERE singleton=1";
        assert_eq!(count(&a, &peek, through), 0);
        a.run(&snap, |q| {
            assert_eq!(unsafe { ffi::sqlite3_get_autocommit(q.handle()) }, 0);
            Ok(())
        })
        .unwrap();
        assert_eq!(count(&a, &snap, "SELECT count(*) FROM s"), 3);
        l1.bind(token, &m1, &r1).unwrap();
        assert_eq!(count(&a, &peek, through), 0);
        l1.content_committed().unwrap();
        assert_eq!(count(&a, &peek, through), 1);
        assert_eq!(l1.committed_through().unwrap(), 1);
        for unit in &d1.new_units {
            assert!(l1.contains_committed_unit(unit).unwrap());
        }
        assert_eq!(l1.qualification().unwrap().through, 1);
        assert_eq!(count(&a, &snap, "SELECT count(*) FROM s"), 3);
        a.run(&snap, |q| q.execute_batch("COMMIT")).unwrap();

        // 失败失效：错误代号整体失败，连接经收尾域回滚并关闭，已提交的一代不受影响。
        let (m3, r3) = generation(3);
        assert!(l1.stage(token, &delta(&m3, &r3)).is_err());
        assert!(l1.connection.is_none());
        assert!(l1.through().is_err());
        assert_eq!(count(&a, &peek, through), 1);
        a.close(peek).map_err(|(_, e)| e).unwrap();
        drop(l1);

        // B：已 stage 未提交的一代遇本域首拒；此后台账不再进入 SQLite，Commit 被拒并回滚。
        current = current.handoff(2, ROOM).expect("handoff to B");
        let b = bind(&current);
        let path2 = c_path("ledger-b.sqlite3");
        let mut l2 = Ledger::fresh_charged(&path2, &content, identity(), STORE, b.clone()).unwrap();
        l2.fold_step(token, &d1, &m1, &r1).unwrap();
        let (m2, r2) = generation(2);
        l2.stage(token, &delta(&m2, &r2)).unwrap();
        l2.bind(token, &m2, &r2).unwrap();
        charged(&l2)
            .run(|q| {
                let rc = unsafe {
                    ffi::sqlite3_trace_v2(
                        q.handle(),
                        ffi::SQLITE_TRACE_STMT,
                        Some(count_ledger_statement),
                        std::ptr::null_mut(),
                    )
                };
                assert_eq!(rc, ffi::SQLITE_OK);
                Ok(())
            })
            .unwrap();
        // 受管开库适配层不在 Rust 堆上建任何持有（路径借出、PRAGMA 走静态串与栈缓冲）。
        let spare_path = c_path("spare.sqlite3");
        std::fs::File::create(dir.join("spare.sqlite3")).unwrap();
        let pages = i64::try_from(STORE / 4096).unwrap();
        let binding = b.clone();
        let window = Window::start();
        let spare = Charged::open(&spare_path, pages, binding);
        let probe = window.finish();
        let spare = spare.unwrap();
        assert_eq!(probe["allocated_bytes"], 0, "{probe}");

        let first = b.run(&snap, |_| {
            assert!(unsafe { ffi::sqlite3_malloc64(ROOM as u64 + 1) }.is_null());
            Ok(())
        });
        assert_eq!(first, Err(SqlGate::Refused(ScratchFailure::Budget)));
        let budget = LedgerError::Gate(SqlGate::Refused(ScratchFailure::Budget));
        // 首拒之后：台账读、受管开库都返回本次 typed 分类，Rust 侧零分配，不进入 SQLite。
        let entered = LEDGER_STATEMENTS.load(Ordering::SeqCst);
        let window = Window::start();
        let refused = l2.through();
        let probe = window.finish();
        assert_eq!(probe["allocated_bytes"], 0, "{probe}");
        let refused = refused.unwrap_err();
        assert_eq!(refused, budget);
        assert_eq!(LEDGER_STATEMENTS.load(Ordering::SeqCst), entered);
        let binding = b.clone();
        let window = Window::start();
        let reopened = Charged::open(&spare_path, pages, binding);
        let probe = window.finish();
        assert_eq!(probe["allocated_bytes"], 0, "{probe}");
        assert_eq!(reopened.err().as_ref(), Some(&budget));
        let committed = l2.content_committed().unwrap_err();
        assert_eq!(committed, budget);
        assert!(l2.connection.is_none());
        assert_eq!(l2.through().unwrap_err(), LedgerError::Detached);
        assert!(matches!(
            committed.into_failure(),
            super::super::audit_owner::native::Failure::BudgetExceeded
        ));
        let kept = refused;
        drop(l2);

        // C：新冻结域里 gen2 已回滚，只剩 gen1；全部关闭后父账回到出生读数。
        current = current.handoff(3, ROOM).expect("handoff to C");
        let c = bind(&current);
        let peek = c.open(&c_path("ledger-b.sqlite3")).unwrap();
        assert_eq!(count(&c, &peek, through), 1);
        assert_eq!(count(&c, &peek, "SELECT count(*) FROM cert"), 1);
        assert_eq!(count(&c, &peek, "SELECT count(*) FROM node"), 3);
        // 旧绑定过期：拒绝按值带回 Ledger 分类，外部首拒槽对它没有记录。
        assert_eq!(
            spare.execute_batch("SELECT 1"),
            Err(LedgerError::Gate(SqlGate::Refused(ScratchFailure::Ledger)))
        );
        // C 域再出首拒后，槽改记 C 的 origin，旧绑定读不到 B 的首拒；保存的错误值不变。
        let second = c.run(&peek, |_| {
            assert!(unsafe { ffi::sqlite3_malloc64(ROOM as u64 + 1) }.is_null());
            Ok(())
        });
        assert_eq!(second, Err(SqlGate::Refused(ScratchFailure::Budget)));
        assert_eq!(b.refusal(), None);
        assert_eq!(kept, budget);
        drop(spare);
        c.close(peek).map_err(|(_, e)| e).unwrap();
        c.close(snap).map_err(|(_, e)| e).unwrap();
        assert_eq!(unsafe { ffi::sqlite3_shutdown() }, ffi::SQLITE_OK);
        assert_eq!(c.live_bytes(), 0);
        assert_eq!(current.live(), base);
        assert!(!allocator.is_broken());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
