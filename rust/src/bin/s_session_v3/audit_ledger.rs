//! #1647：旁挂审计事实存储与提交生命周期。这里不签发来源或闭包核验资格。
//! 调用者只能送入已核验的单代变化；#1481 的冷折叠与在线生产者共用 stage。
//! 完整台账的事实由同次 prepare 的来源收据生产（36M cover 勘误），H1 只经 `Qualification` 点查，
//! 不持裸 Ledger 引用；准入切换仍受 #1645/#1491 门约束。
#![allow(dead_code)]

use super::*;
#[cfg(feature = "s_incremental")]
use newchan_rust::s_content as sc;
use newchan_rust::s_content::capture_allocation::{native_owned::NativeValue, scratch::WorkScope};
use newchan_rust::s_content::{ContentRef, RefView};
use rusqlite::{params, OpenFlags};
use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::OpenOptionsExt;

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
struct QualifiedSuccessView<'a> {
    ledger: &'a Ledger,
    qualification: &'a Qualification,
}
#[cfg(feature = "s_incremental")]
impl sc::closure_delta::Frontier for QualifiedSuccessView<'_> {
    fn contains(&self, unit: sc::closure_delta::Unit) -> Result<bool, sc::CodecError> {
        self.ledger
            .contains_qualified_unit(self.qualification, &Unit::of_closure(unit))
            .map_err(|e| sc::CodecError::new(sc::code::SOURCE, e))
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

pub(super) struct Ledger {
    connection: Option<Connection>,
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
        if identity.rule_revision.is_empty()
            || identity.content_identity.is_empty()
            || identity.binary_commit.is_empty()
        {
            return Err(error("台账身份字段不能为空"));
        }
        let content_path = content_path.canonicalize().map_err(error)?;
        let remaining = max_store_bytes
            .checked_sub(database_bytes(&content_path)?)
            .ok_or_else(|| error("内容库已超过 max_store_bytes"))?;
        let pages = i64::try_from(remaining / 4096).map_err(error)?;
        if pages == 0 {
            return Err(error("没有台账页空间"));
        }
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(error)?;
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(error)?;
        conn.busy_timeout(std::time::Duration::ZERO)
            .map_err(error)?;
        conn.execute_batch(
            "PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;",
        )
        .map_err(error)?;
        conn.pragma_update(None, "page_size", 4096).map_err(error)?;
        conn.pragma_update(None, "max_page_count", pages)
            .map_err(error)?;
        conn.execute_batch(SCHEMA).map_err(error)?;
        conn.execute(
            "INSERT INTO meta VALUES(1,?1,?2,?3,?4,0,?5)",
            params![
                FORMAT,
                identity.rule_revision,
                identity.content_identity,
                identity.binary_commit,
                sha256_hex(FORMAT.as_bytes())
            ],
        )
        .map_err(error)?;
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
    #[cfg(feature = "s_incremental")]
    pub(super) fn fresh_raw_partition(
        path: &Path,
        content_path: &Path,
        identity: Identity,
        max_store_bytes: u64,
    ) -> Result<Self, String> {
        let mut ledger = Self::fresh(path, content_path, identity, max_store_bytes)?;
        ledger
            .connection
            .as_ref()
            .unwrap()
            .execute(
                "UPDATE meta SET format=?1 WHERE singleton=1",
                [RAW_PARTITION_FORMAT],
            )
            .map_err(error)?;
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
        ledger
            .connection
            .as_ref()
            .unwrap()
            .execute(
                "UPDATE meta SET format=?1 WHERE singleton=1",
                [SOURCE_PARTITIONS_FORMAT],
            )
            .map_err(error)?;
        ledger.coverage = Coverage::SourcePartitions { through: 0 };
        Ok(ledger)
    }
    pub(super) fn is_source_partitions(&self) -> bool {
        matches!(self.coverage, Coverage::SourcePartitions { .. })
    }
    pub(super) fn is_partial(&self) -> bool {
        self.is_raw_partition() || self.is_source_partitions()
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn partial_through(&self) -> Result<i64, String> {
        match (&self.phase, &self.coverage) {
            (
                Phase::Ready,
                Coverage::RawPartition { through } | Coverage::SourcePartitions { through },
            ) => Ok(*through),
            _ => Err(error("不是已提交的来源分区")),
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
    ) -> Result<R, String> {
        let through = self.committed_through()?;
        if through < 1 {
            return Err(error("完整台账尚无已提交的一代"));
        }
        let conn = self.connection.as_ref().unwrap();
        conn.query_row(
            "SELECT c.roots, c.fold_digest, c.verifier_revision, m.fold_digest, m.content_identity, \
             m.binary_commit, m.rule_revision FROM cert c, meta m WHERE c.generation=?1 AND m.singleton=1",
            [through],
            |r| {
                let text = |i: usize| r.get_ref(i)?.as_str().map_err(rusqlite::Error::from);
                let fold_digest = text(1)?;
                if text(3)? != fold_digest {
                    return Ok(Err(error("完整水位与末代 cert 折叠摘要不一致")));
                }
                let rule_revision = text(6)?;
                if rule_revision != self.rule_revision {
                    return Ok(Err(error("台账持久规则与安装规则不一致")));
                }
                Ok(f(&QualificationView {
                    through,
                    roots: text(0)?,
                    fold_digest,
                    verifier_revision: text(2)?,
                    rule_revision,
                    content_identity: text(4)?,
                    binary_commit: text(5)?,
                }))
            },
        )
        .map_err(error)?
    }
    /// H1 的私有点查资格：绑定已提交完整水位、四根、规则、verifier 与内容库身份。
    /// 不含连接、不可跨进程；任何字段不符即不能用它点查。
    pub(super) fn qualification(&self) -> Result<Qualification, String> {
        self.with_committed_qualification(|v| Ok(v.owned()))
    }
    /// 39P：字符串总字节先对调用方已预留的上界核过，再按精确容量分配（B2）。
    pub(super) fn qualification_bounded(&self, cap: usize) -> Result<Qualification, String> {
        self.with_committed_qualification(|v| {
            if v.string_bytes() > cap {
                return Err(error("完整资格超过调用方保留上界"));
            }
            Ok(v.owned())
        })
    }
    fn matches_committed(&self, q: &Qualification) -> Result<bool, String> {
        self.with_committed_qualification(|v| Ok(q.matches(v)))
    }
    /// 只接受本台账当前签发且水位未变的资格；台账 Pending/Bound/Stale 或已推进一代即拒绝。
    pub(super) fn contains_qualified_unit(
        &self,
        q: &Qualification,
        unit: &Unit,
    ) -> Result<bool, String> {
        if !self.matches_committed(q)? {
            return Err(error("点查资格与当前已提交台账不符"));
        }
        self.contains_committed_unit(unit)
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn raw_partition_through(&self) -> Result<i64, String> {
        match (&self.phase, &self.coverage) {
            (Phase::Ready, Coverage::RawPartition { through }) => Ok(*through),
            _ => Err(error("不是已提交的raw分区")),
        }
    }
    fn check_storage(&self) -> Result<(), String> {
        let size = database_bytes(&self.content_path)?
            .checked_add(database_bytes(&self.path)?)
            .ok_or_else(|| error("存储合账溢出"))?;
        if size > self.max_store_bytes {
            return Err(error("内容库/WAL/台账合计超过 max_store_bytes"));
        }
        Ok(())
    }
    pub(super) fn invalidate(&mut self) {
        self.phase = Phase::Stale;
        // 丢弃连接也回滚未提交的整代；不重试，也不复原旧进程状态。
        drop(self.connection.take());
    }
    pub(super) fn through(&self) -> Result<i64, String> {
        self.connection
            .as_ref()
            .ok_or_else(|| error("台账已丢弃"))?
            .query_row(
                "SELECT certified_through_generation FROM meta WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .map_err(error)
    }
    /// 已提交存储读数，不是 U-A 资格；准备中连本连接的未提交行也不暴露。
    pub(super) fn committed_through(&self) -> Result<i64, String> {
        if !matches!(self.phase, Phase::Ready) || !matches!(self.coverage, Coverage::Complete) {
            return Err(error("完整台账尚未提交或仅有raw分区"));
        }
        self.through()
    }
    pub(super) fn contains_committed_unit(&self, unit: &Unit) -> Result<bool, String> {
        let through = self.committed_through()?;
        self.connection.as_ref().ok_or_else(||error("台账已丢弃"))?.query_row(
            "SELECT EXISTS(SELECT 1 FROM node WHERE digest=?1 AND flavor=?2 AND mode=?3 AND first_generation<=?4)",
            params![unit.digest.as_slice(), unit.flavor(), unit.mode(), through], |r|r.get(0),
        ).map_err(error)
    }
    pub(super) fn stage(&mut self, token: &str, delta: &GenerationDelta) -> Result<(), String> {
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
    ) -> Result<QualifiedUnits<'q>, String> {
        if !self.matches_committed(qualification)? {
            return Err(error("Δ核验资格与当前已提交台账不符"));
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
    ) -> Result<(), String> {
        let result = (|| {
            self.committed_through()?;
            if let Some(bound) = verified.frontier {
                if prior != Some(bound) {
                    return Err(error("Δ单位绑定资格与上一代资格不符"));
                }
            }
            let verified = &verified.verified;
            if !verified
                .matches_roots(delta.roots.each_ref().map(Into::into))
                .map_err(error)?
            {
                return Err(error("闭包单位与待准备事实四根不匹配"));
            }
            let mut units = sc::CodecVec::default();
            verified.try_for_each_unit(|u| -> Result<(), String> {
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
    ) -> Result<(), String> {
        if !matches!(self.phase, Phase::Ready) || !matches!(self.coverage, Coverage::Complete) {
            return Err(error("台账不在完整事实可准备状态"));
        }
        if token.is_empty()
            || verifier_revision.is_empty()
            || self.through()?.checked_add(1) != Some(d.generation)
        {
            return Err(error("请求或逐代序号无效"));
        }
        for r in &d.roots {
            r.digest().map_err(error)?;
        }
        self.check_storage()?;
        let conn = self.connection.as_ref().unwrap();
        conn.execute_batch("BEGIN IMMEDIATE").map_err(error)?;
        if let Some(prior) = prior {
            let detail = match self.matches_committed(prior) {
                Ok(true) => None,
                Ok(false) => Some("字段不同".to_owned()),
                Err(e) => Some(e),
            };
            if let Some(detail) = detail {
                return Err(error(format!(
                    "上一代资格与台账当前已提交状态不符，不续写折叠链：{detail}"
                )));
            }
        }
        let previous: String = conn
            .query_row("SELECT fold_digest FROM meta WHERE singleton=1", [], |r| {
                r.get(0)
            })
            .map_err(error)?;
        let mut next: i64 = conn
            .query_row("SELECT COALESCE(MAX(seq),-1)+1 FROM raw", [], |r| r.get(0))
            .map_err(error)?;
        for (seq, row) in &d.raw {
            if *seq != next {
                return Err(error("raw 只允许连续追加"));
            }
            conn.execute(
                "INSERT INTO raw VALUES(?1,?2,?3,?4)",
                params![
                    seq,
                    row.event_digest.as_slice(),
                    row.bar_digest.as_slice(),
                    d.generation
                ],
            )
            .map_err(error)?;
            next = next.checked_add(1).ok_or_else(|| error("raw 序号溢出"))?;
        }
        for (id, r) in &d.objects {
            if r.first_generation < 1
                || r.first_generation > r.published_generation
                || r.published_generation > d.generation
            {
                return Err(error("对象代戳无效"));
            }
            conn.execute("INSERT INTO object VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
                ON CONFLICT(id) DO UPDATE SET record_digest=excluded.record_digest,
                first_generation=excluded.first_generation, first_cut=excluded.first_cut,
                published_generation=excluded.published_generation, source_digest=excluded.source_digest,
                withdrawn=excluded.withdrawn, certified_generation=excluded.certified_generation",
                params![id,r.record_digest.as_slice(),r.first_generation,r.first_cut,
                    r.published_generation,r.source_digest.as_slice(),r.withdrawn,d.generation]).map_err(error)?;
        }
        for (table, rows) in [("witness", &d.witnesses), ("observation", &d.observations)] {
            for (id, r) in rows {
                conn.execute(&format!("INSERT INTO {table} VALUES(?1,?2,?3,?4)
                    ON CONFLICT(id) DO UPDATE SET record_digest=excluded.record_digest,
                    source_digest=excluded.source_digest, certified_generation=excluded.certified_generation"),
                    params![id,r.record_digest.as_slice(),r.source_digest.as_slice(),d.generation]).map_err(error)?;
            }
        }
        for id in &d.withdrawals {
            let changed = conn
                .execute(
                    "UPDATE object SET withdrawn=1,certified_generation=?1 WHERE id=?2 AND withdrawn=0",
                    params![d.generation, id],
                )
                .map_err(error)?;
            if changed != 1 {
                return Err(error("撤回身份缺失或已撤回"));
            }
        }
        for (coord, change) in &d.cover {
            match change {
                CoverChange::Removed => {
                    if conn
                        .execute("DELETE FROM cover WHERE source_coord=?1", [coord])
                        .map_err(error)?
                        != 1
                    {
                        return Err(error("cover 失效坐标不在当前分区"));
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
                        params![coord, seq, object_id, d.generation],
                    )
                    .map_err(error)?;
                }
                CoverChange::Pending { seq } => {
                    conn.execute(
                        "INSERT INTO cover VALUES(?1,?2,'pending',NULL,?3) ON CONFLICT(source_coord) DO UPDATE SET
                         seq=excluded.seq, owner=excluded.owner, object_id=excluded.object_id,
                         certified_generation=excluded.certified_generation
                         WHERE seq IS NOT excluded.seq OR owner IS NOT excluded.owner
                         OR object_id IS NOT excluded.object_id",
                        params![coord, seq, d.generation],
                    )
                    .map_err(error)?;
                }
            }
        }
        let covered: i64 = conn
            .query_row("SELECT COUNT(*) FROM cover", [], |r| r.get(0))
            .map_err(error)?;
        if covered != d.effective {
            return Err(error("cover 行数不等于本 cut 生效坐标总数"));
        }
        for unit in new_units.clone() {
            // new_units 必须真是本代新增集合，重复认证单位拒绝而非静默忽略。
            conn.execute(
                "INSERT INTO node VALUES(?1,?2,?3,?4)",
                params![
                    unit.digest.as_slice(),
                    unit.flavor(),
                    unit.mode(),
                    d.generation
                ],
            )
            .map_err(error)?;
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
            params![
                d.generation,
                canonical_json(&roots),
                sha256_hex(units.as_bytes()),
                i64::try_from(unit_count).map_err(error)?,
                fold,
                verifier_revision
            ],
        )
        .map_err(error)?;
        conn.execute(
            "UPDATE meta SET certified_through_generation=?1,fold_digest=?2 WHERE singleton=1",
            params![d.generation, fold],
        )
        .map_err(error)?;
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
    ) -> Result<(), String> {
        if !self.is_raw_partition() {
            self.invalidate();
            return Err(error("来源分区格式不匹配"));
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
    ) -> Result<(), String> {
        if !self.is_source_partitions() {
            self.invalidate();
            return Err(error("来源分区格式不匹配"));
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
    ) -> Result<(), String> {
        let result = (|| {
            let through = self.partial_through()?;
            if through.checked_add(1) != Some(metadata.generation)
                || metadata.rule_revision != self.rule_revision
            {
                return Err(error("raw分区代号或规则不连续"));
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
            conn.execute_batch("BEGIN IMMEDIATE").map_err(error)?;
            let mut next: i64 = conn
                .query_row("SELECT COALESCE(MAX(seq),-1)+1 FROM raw", [], |r| r.get(0))
                .map_err(error)?;
            if usize::try_from(next).ok() != Some(range.start) || source.rows().len() != range.len()
            {
                return Err(error("raw来源后缀与已提交分区不衔接"));
            }
            for row in source.rows() {
                if row.seq != next {
                    return Err(error("raw来源后缀不连续"));
                }
                conn.execute(
                    "INSERT INTO raw VALUES(?1,?2,?3,?4)",
                    params![
                        row.seq,
                        row.event_digest.as_slice(),
                        row.bar_digest.as_slice(),
                        metadata.generation
                    ],
                )
                .map_err(error)?;
                next = next.checked_add(1).ok_or_else(|| error("raw序号溢出"))?;
            }
            if self.is_source_partitions() {
                let rows = source
                    .business_rows()
                    .map_err(sc::ContextError::into_message)?;
                for row in &rows.objects {
                    conn.execute(
                        "INSERT INTO object VALUES(?1,?2,?3,?4,?5,?6,0,?7)",
                        params![
                            row.id.as_str(),
                            row.record.as_slice(),
                            row.first_generation,
                            row.first_cut.as_str(),
                            row.published_generation,
                            row.sources.as_slice(),
                            metadata.generation
                        ],
                    )
                    .map_err(error)?;
                }
                for id in &rows.withdrawals {
                    let changed = conn.execute("UPDATE object SET withdrawn=1,certified_generation=?1 WHERE id=?2 AND withdrawn=0",
                        params![metadata.generation, id.as_str()]).map_err(error)?;
                    if changed != 1 {
                        return Err(error("来源分区撤回身份缺失或已撤回"));
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
                            params![
                                row.id.as_str(),
                                row.record.as_slice(),
                                row.sources.as_slice(),
                                metadata.generation
                            ],
                        )
                        .map_err(error)?;
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
    pub(super) fn require_pending(&self) -> Result<(), String> {
        if matches!(self.phase, Phase::Pending(_) | Phase::Bound(_)) {
            Ok(())
        } else {
            Err(error("发布缺本请求台账变化或台账已失效"))
        }
    }
    pub(super) fn bind(
        &mut self,
        token: &str,
        m: &content_manifest::ManifestMetadata,
        roots: &impl content_manifest::ManifestRootSource,
    ) -> Result<(), String> {
        self.bind_view(token, m.view(), roots.view())
    }
    pub(super) fn bind_view(
        &mut self,
        token: &str,
        m: content_manifest::MetadataView<'_>,
        v: content_manifest::RootsView<'_>,
    ) -> Result<(), String> {
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
                    Err(error("台账四根与本请求 prepare 不一致"))
                }
            }
            _ => Err(error("台账请求、规则或代号不一致")),
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
    pub(super) fn content_committed(&mut self) -> Result<(), String> {
        let result = (|| {
            if !matches!(self.phase, Phase::Bound(_)) {
                return Err(error("台账尚未绑定实际 prepare"));
            }
            self.check_storage()?;
            self.connection
                .as_ref()
                .unwrap()
                .execute_batch("COMMIT")
                .map_err(error)?;
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
    ) -> Result<(), String> {
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
        let conn = online.connection.as_ref().unwrap();
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
        let connection = ledger.connection.as_ref().unwrap();
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

        /// #1419 M8D L9（台账层）：test-only SQLITE_TRACE_STMT 按原语句文本计数，证实每次前沿
        /// 点查恰为资格行 + 完整键 EXISTS 两条台账语句、点查之外无别的台账 SQL；stage/bind/
        /// 内容提交的逐桶语句另列收据。不改台账连接配置，回调随 guard 卸载。
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
                let db = unsafe { b.connection.as_ref().unwrap().handle() };
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
                if lookups > 0 {
                    assert_eq!(lookup_sql.len(), 2, "g{generation}：点查只用两条台账语句");
                }
                assert!(
                    lookup_sql.values().all(|&n| n == lookups),
                    "g{generation}：每次点查恰各一次资格行与 EXISTS，别无台账 SQL"
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
}
