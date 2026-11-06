//! #1647：旁挂审计事实存储与提交生命周期。这里不签发来源或闭包核验资格。
//! 调用者只能送入已核验的单代变化；#1481 的冷折叠与在线生产者共用 stage。
//! 完整台账的事实由同次 prepare 的来源收据生产（36M cover 勘误），H1 只经 `Qualification` 点查，
//! 不持裸 Ledger 引用；准入切换仍受 #1645/#1491 门约束。
#![allow(dead_code)]

#[cfg(feature = "s_incremental")]
use super::audit_owner::native::{Budget, Permit, Reserved};
use super::*;
#[cfg(feature = "s_incremental")]
use newchan_rust::s_content as sc;
#[cfg(feature = "s_incremental")]
use newchan_rust::s_content::capture_allocation::scratch::ScratchFailure;
#[cfg(feature = "s_incremental")]
use newchan_rust::s_content::capture_allocation::sqlite_parent::{
    ChargedConnection, OpenSpec, SqlGate, SqlOps, SqliteBinding,
};
use newchan_rust::s_content::capture_allocation::{native_owned::NativeValue, scratch::WorkScope};
use newchan_rust::s_content::{CodecError, ContentRef, RefView};
use rusqlite::types::ValueRef;
use rusqlite::{OpenFlags, ToSql};
use std::collections::{BTreeMap, BTreeSet};
#[cfg(feature = "s_incremental")]
use std::ffi::CStr;
use std::os::unix::fs::OpenOptionsExt;
#[cfg(feature = "s_incremental")]
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

/// 与 `canonical_json(Value)` 同字节的借用写出视图。`canonical_json` 等于按字节序排好对象键的
/// compact JSON，所以这里只按既有键序把字段送进 `json_writer::write_json`；数字、转义与分隔符
/// 由它给出，不建 `Value`，不建中间文本。`i64` 键的映射由调用方给出按十进制文本排好的条目。
#[cfg_attr(
    all(not(test), not(feature = "s_incremental")),
    allow(
        dead_code,
        reason = "#1419 3C：只有受管台账（s_incremental）在生产中经此写出"
    )
)]
mod canonical {
    use super::{CoverChange, GenerationDelta, ObjectFact, RawFact, Unit};
    use newchan_rust::s_content::json_writer;
    use newchan_rust::s_content::{ContentRef, RefView};
    use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
    use sha2::{Digest, Sha256};
    use std::collections::BTreeMap;

    /// 十进制文本，栈上至多 20 字节（含 `i64::MIN` 的负号与 `u64::MAX`）。
    pub(super) struct Decimal {
        bytes: [u8; 20],
        start: usize,
    }

    impl Decimal {
        pub(super) fn of_i64(n: i64) -> Self {
            Self::of_magnitude(n.unsigned_abs(), n < 0)
        }
        pub(super) fn of_u64(n: u64) -> Self {
            Self::of_magnitude(n, false)
        }
        fn of_magnitude(mut m: u64, negative: bool) -> Self {
            let mut bytes = [0u8; 20];
            let mut start = bytes.len();
            loop {
                start -= 1;
                bytes[start] = b'0' + (m % 10) as u8;
                m /= 10;
                if m == 0 {
                    break;
                }
            }
            if negative {
                start -= 1;
                bytes[start] = b'-';
            }
            Self { bytes, start }
        }
        pub(super) fn as_str(&self) -> &str {
            std::str::from_utf8(&self.bytes[self.start..]).unwrap_or_default()
        }
    }

    /// 按十进制文本排序 `i64` 键的映射条目（`canonical_json` 对字符串键排序，`"10"` 在 `"2"` 前）。
    pub(super) fn sort_by_decimal<T>(entries: &mut [(i64, &T)]) {
        entries.sort_unstable_by(|a, b| {
            Decimal::of_i64(a.0)
                .as_str()
                .cmp(Decimal::of_i64(b.0).as_str())
        });
    }

    struct Root<'a>(RefView<'a>);
    impl Serialize for Root<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let r = self.0;
            let mut m = s.serialize_map(Some(4))?;
            match r.extent() {
                Some(n) => m.serialize_entry("extent", Decimal::of_u64(n).as_str())?,
                None => m.serialize_entry("extent", &())?,
            }
            m.serialize_entry("format", r.format())?;
            m.serialize_entry("kind", r.kind().name())?;
            m.serialize_entry("root", r.root())?;
            m.end()
        }
    }

    pub(super) struct Roots<'a>(pub &'a [ContentRef; 4]);
    impl Serialize for Roots<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut seq = s.serialize_seq(Some(4))?;
            for r in self.0 {
                seq.serialize_element(&Root(r.into()))?;
            }
            seq.end()
        }
    }

    struct Raw<'a>(&'a RawFact);
    impl Serialize for Raw<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut m = s.serialize_map(Some(2))?;
            m.serialize_entry("bar_digest", &self.0.bar_digest)?;
            m.serialize_entry("event_digest", &self.0.event_digest)?;
            m.end()
        }
    }

    struct Object<'a>(&'a ObjectFact);
    impl Serialize for Object<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let o = self.0;
            let mut m = s.serialize_map(Some(6))?;
            m.serialize_entry("first_cut", o.first_cut.as_str())?;
            m.serialize_entry("first_generation", &o.first_generation)?;
            m.serialize_entry("published_generation", &o.published_generation)?;
            m.serialize_entry("record_digest", &o.record_digest)?;
            m.serialize_entry("source_digest", &o.source_digest)?;
            m.serialize_entry("withdrawn", &o.withdrawn)?;
            m.end()
        }
    }

    struct Group<'a> {
        object_id: &'a str,
        seq: i64,
    }
    impl Serialize for Group<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut m = s.serialize_map(Some(2))?;
            m.serialize_entry("object_id", self.object_id)?;
            m.serialize_entry("seq", &self.seq)?;
            m.end()
        }
    }

    struct Seq(i64);
    impl Serialize for Seq {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut m = s.serialize_map(Some(1))?;
            m.serialize_entry("seq", &self.0)?;
            m.end()
        }
    }

    struct Cover<'a>(&'a CoverChange);
    impl Serialize for Cover<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            match self.0 {
                CoverChange::Group { seq, object_id } => {
                    let mut m = s.serialize_map(Some(1))?;
                    m.serialize_entry(
                        "Group",
                        &Group {
                            object_id,
                            seq: *seq,
                        },
                    )?;
                    m.end()
                }
                CoverChange::Pending { seq } => {
                    let mut m = s.serialize_map(Some(1))?;
                    m.serialize_entry("Pending", &Seq(*seq))?;
                    m.end()
                }
                CoverChange::Removed => s.serialize_str("Removed"),
            }
        }
    }

    struct RawMap<'a>(&'a [(i64, &'a RawFact)]);
    impl Serialize for RawMap<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut m = s.serialize_map(Some(self.0.len()))?;
            for (k, v) in self.0 {
                m.serialize_entry(Decimal::of_i64(*k).as_str(), &Raw(v))?;
            }
            m.end()
        }
    }

    struct CoverMap<'a>(&'a [(i64, &'a CoverChange)]);
    impl Serialize for CoverMap<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut m = s.serialize_map(Some(self.0.len()))?;
            for (k, v) in self.0 {
                m.serialize_entry(Decimal::of_i64(*k).as_str(), &Cover(v))?;
            }
            m.end()
        }
    }

    struct ObjectMap<'a>(&'a BTreeMap<String, ObjectFact>);
    impl Serialize for ObjectMap<'_> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut m = s.serialize_map(Some(self.0.len()))?;
            for (k, v) in self.0 {
                m.serialize_entry(k.as_str(), &Object(v))?;
            }
            m.end()
        }
    }

    pub(super) struct Units<U>(pub U);
    impl<'u, U: Iterator<Item = &'u Unit> + Clone> Serialize for Units<U> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut seq = s.serialize_seq(None)?;
            for unit in self.0.clone() {
                seq.serialize_element(unit)?;
            }
            seq.end()
        }
    }

    /// 一代的规范 delta；`units` 为实际写入 node 的新单位（核验后筛选的集合），不是
    /// `d.new_units`。`raw`、`cover` 须是 [`sort_by_decimal`] 排好的同一映射全部条目。
    pub(super) struct Delta<'a, U> {
        pub d: &'a GenerationDelta,
        pub verifier_revision: &'a str,
        pub units: U,
        pub raw: &'a [(i64, &'a RawFact)],
        pub cover: &'a [(i64, &'a CoverChange)],
    }
    impl<'a, 'u, U: Iterator<Item = &'u Unit> + Clone> Serialize for Delta<'a, U> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let d = self.d;
            let mut m = s.serialize_map(Some(11))?;
            m.serialize_entry("cover", &CoverMap(self.cover))?;
            m.serialize_entry("effective", &d.effective)?;
            m.serialize_entry("generation", &d.generation)?;
            m.serialize_entry("new_units", &Units(self.units.clone()))?;
            m.serialize_entry("objects", &ObjectMap(&d.objects))?;
            m.serialize_entry("observations", &d.observations)?;
            m.serialize_entry("raw", &RawMap(self.raw))?;
            m.serialize_entry("roots", &Roots(&d.roots))?;
            m.serialize_entry("verifier_revision", self.verifier_revision)?;
            m.serialize_entry("withdrawals", &d.withdrawals)?;
            m.serialize_entry("witnesses", &d.witnesses)?;
            m.end()
        }
    }

    /// 折叠链输入 `[previous, delta]`。
    pub(super) struct Fold<'a, U> {
        pub previous: &'a str,
        pub delta: Delta<'a, U>,
    }
    impl<'a, 'u, U: Iterator<Item = &'u Unit> + Clone> Serialize for Fold<'a, U> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut seq = s.serialize_seq(Some(2))?;
            seq.serialize_element(self.previous)?;
            seq.serialize_element(&self.delta)?;
            seq.end()
        }
    }

    struct HashWriter(Sha256);
    impl std::io::Write for HashWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct CountWriter(usize);
    impl std::io::Write for CountWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len());
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// 小写十六进制的 SHA-256 文本（与 `sha256_hex` 同字节），栈上 64 字节。
    pub(crate) struct Hex([u8; 64]);
    impl Hex {
        pub(super) fn of_digest(digest: [u8; 32]) -> Self {
            const DIGITS: &[u8; 16] = b"0123456789abcdef";
            let mut out = [0u8; 64];
            for (i, b) in digest.iter().enumerate() {
                out[2 * i] = DIGITS[usize::from(b >> 4)];
                out[2 * i + 1] = DIGITS[usize::from(b & 0x0f)];
            }
            Self(out)
        }
        pub(super) fn of_bytes(bytes: &[u8]) -> Self {
            Self::of_digest(Sha256::digest(bytes).into())
        }
        pub(super) fn as_str(&self) -> &str {
            std::str::from_utf8(&self.0).unwrap_or_default()
        }
    }

    /// `sha256_hex(canonical_json(value))` 的同字节结果，不建中间文本。
    pub(super) fn hash_hex<T: Serialize + ?Sized>(value: &T) -> Result<Hex, json_writer::Error> {
        let mut out = HashWriter(Sha256::new());
        json_writer::write_json(&mut out, value)?;
        Ok(Hex::of_digest(out.0.finalize().into()))
    }

    /// 写出字节数，供分配前按精确容量预留。
    pub(super) fn json_len<T: Serialize + ?Sized>(value: &T) -> Result<usize, json_writer::Error> {
        let mut out = CountWriter(0);
        json_writer::write_json(&mut out, value)?;
        Ok(out.0)
    }

    /// 写进调用方已按 [`json_len`] 预留好容量的缓冲。
    pub(super) fn write_into<T: Serialize + ?Sized>(
        out: &mut Vec<u8>,
        value: &T,
    ) -> Result<(), json_writer::Error> {
        json_writer::write_json(out, value)
    }
}

/// 一代写入 cert/meta 的三项：roots 规范文本、new_units 摘要、折叠摘要。
pub(super) struct StageDigests<T> {
    roots: T,
    units: T,
    fold: T,
}

/// 一代的三段规范文本：cert.roots、new_units 摘要输入、折叠链输入 `[previous, delta]`。
struct StageTexts {
    roots: String,
    units: String,
    fold: String,
}

/// 现行 stage 规范文本（经 `Value` 与 `canonical_json`）。
fn legacy_texts<'u>(
    previous: &str,
    d: &GenerationDelta,
    new_units: impl Iterator<Item = &'u Unit> + Clone,
    verifier_revision: &str,
) -> Result<StageTexts, String> {
    let roots = roots_value(&d.roots);
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
    Ok(StageTexts {
        roots: canonical_json(&roots),
        units,
        fold: canonical_json(&json!([previous, delta])),
    })
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
struct QualifiedSuccessView<'a, B: Backend> {
    ledger: &'a Ledger<B>,
    qualification: &'a Qualification,
    /// 本次点查遇到的 typed 错误，按值留给 `verify_delta` 原样返回。
    refused: std::cell::RefCell<Option<B::Error>>,
}
#[cfg(feature = "s_incremental")]
impl<'a, B: Backend> QualifiedSuccessView<'a, B> {
    fn new(ledger: &'a Ledger<B>, qualification: &'a Qualification) -> Self {
        Self {
            ledger,
            qualification,
            refused: std::cell::RefCell::new(None),
        }
    }
}
#[cfg(feature = "s_incremental")]
impl<B: Backend> sc::closure_delta::Frontier for QualifiedSuccessView<'_, B> {
    fn contains(&self, unit: sc::closure_delta::Unit) -> Result<bool, sc::CodecError> {
        self.ledger
            .contains_qualified_unit(self.qualification, &Unit::of_closure(unit))
            .map_err(|e| match B::frontier_error(e) {
                Ok(typed) => {
                    *self.refused.borrow_mut() = Some(typed);
                    sc::CodecError::new(sc::code::SOURCE, String::new())
                }
                Err(text) => sc::CodecError::new(sc::code::SOURCE, text),
            })
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

pub(super) struct Pending {
    token: NativeValue<String>,
    generation: i64,
    roots: [NativeValue<ContentRef>; 4],
    /// 受管台账为这一代持有的原父许可；放在最后，先释放上面的载荷。
    #[cfg(feature = "s_incremental")]
    fee: Option<Permit>,
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
            #[cfg(feature = "s_incremental")]
            fee: None,
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

mod sealed {
    pub trait Sealed {}
}

/// 台账连接后端，在构造处选定且不切换。私有 `sealed` 只在本模块为 [`Legacy`] 与 [`Charged`]
/// 实现，其他模块无法新增后端。台账阶段机与 SQL 只经这三个操作访问数据库。
pub(super) trait Backend: sealed::Sealed {
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
        f: impl FnOnce([ValueRef<'_>; M]) -> Result<T, Self::Error>,
    ) -> Result<T, Self::Error>;
    /// 行解码失败。
    fn decode(e: rusqlite::types::FromSqlError) -> Self::Error;
    /// 台账调用的外部核验（来源收据、闭包、根格式）失败：Legacy 原样包装其诊断；Charged 只带
    /// 静态上下文，不复制诊断文本。
    fn external(e: impl std::fmt::Display, context: &'static str) -> Self::Error;
    /// codec 错误：Legacy 原样包装其诊断；Charged 先保留它自带的 typed scratch 分类（如
    /// Budget），没有分类时才只带静态上下文。
    fn codec(e: CodecError, context: &'static str) -> Self::Error;
    /// stage 前的根格式核验：Legacy 沿用 `ContentRef::digest` 及其诊断；Charged 用同一判据的
    /// 无分配版本。
    fn root_digest(r: &ContentRef) -> Result<(), Self::Error>;
    /// stage_verified 在闭包四根比对之前的根核验：Legacy 不做（保持原比对的短路次序）；
    /// Charged 用无分配判据先挡住会生成诊断文本的根。
    fn guard_roots(roots: &[ContentRef; 4]) -> Result<(), Self::Error>;
    /// 前沿核验的工作域归属：Legacy 不要求；Charged 要求 source 带着本台账原父当前冻结域的
    /// codec 工作域，否则 verifier 会用无账或他父的工作集。
    #[cfg(feature = "s_incremental")]
    fn frontier_source(&self, source: &dyn sc::NodeSource) -> Result<(), Self::Error>;
    /// 共用 codec 核验期间的诊断方式：Legacy 照常生成诊断文本；Charged 进入静默诊断期，
    /// codec 错误只带 `code` 与 scratch 分类，诊断文本不在原父账之外生成。
    fn diagnostics(&self) -> Option<newchan_rust::s_content::QuietDiagnostics>;
    /// 来源收据核验失败：Legacy 沿用收据自己的诊断；Charged 只带静态上下文。
    #[cfg(feature = "s_incremental")]
    fn source_receipt(e: sc::ContextError) -> Self::Error;
    /// 连接已在失效时丢弃。
    fn detached() -> Self::Error;
    /// 台账判据失败（文本与 Legacy 的 `error(text)` 相同）。
    fn invalid(text: &'static str) -> Self::Error;
    /// 规范文本与摘要的持有：Legacy 为 `String`；Charged 为原父许可持有的文本或栈上摘要。
    type Text: std::ops::Deref<Target = str>;
    /// stage 事务内读取当前已提交的折叠摘要。
    fn previous_fold(&self) -> Result<Self::Text, Self::Error>;
    /// 一代的 cert.roots 文本、new_units 摘要与折叠摘要（与 `legacy_texts` 同字节）。
    fn stage_texts<'u>(
        &self,
        previous: &str,
        d: &GenerationDelta,
        units: impl Iterator<Item = &'u Unit> + Clone,
        verifier_revision: &str,
    ) -> Result<StageDigests<Self::Text>, Self::Error>;
    /// 台账与内容库的持久路径：Legacy 为 `PathBuf`；Charged 为原父许可持有的主文件与旁文件
    /// NUL 路径。
    type Path;
    /// 安装规则文本：Legacy 为 `String`；Charged 为原父许可持有的副本。
    type Rule: std::ops::Deref<Target = str>;
    /// 数据库文件及其 `-wal`、`-journal` 的合计字节。
    fn storage_bytes(path: &Self::Path) -> Result<u64, Self::Error>;
    /// 以 0600 新建台账文件；已存在即失败，不覆盖旧原件。
    fn create_new(path: &Self::Path) -> Result<(), Self::Error>;
    /// meta 初始折叠摘要 `sha256(FORMAT)`。
    fn format_hash() -> Self::Text;
    /// 待提交一代的持有（token 与四根）。
    fn pending(
        &self,
        token: &str,
        generation: i64,
        roots: [RefView<'_>; 4],
        work: Option<&WorkScope>,
    ) -> Result<Pending, Self::Error>;
    /// 核验后的新单位集合（排除已提交单位），供 stage 写 node 与摘要。
    #[cfg(feature = "s_incremental")]
    type Units: std::ops::Deref<Target = [Unit]>;
    #[cfg(feature = "s_incremental")]
    fn new_units(
        &self,
        verified: &sc::closure_delta::VerifiedUnits,
        work: Option<&WorkScope>,
        committed: impl Fn(&Unit) -> Result<bool, Self::Error>,
    ) -> Result<Self::Units, Self::Error>;
    /// stage 时上一代资格与已提交状态不符；`detail` 为查询本身的错误（`None` 为字段不同）。
    fn prior_mismatch(detail: Option<Self::Error>) -> Self::Error;
    /// 前沿点查错误穿过 `CodecError` 接口：`Ok` 为须原样带回调用方的错误，`Err` 为诊断文本。
    fn frontier_error(e: Self::Error) -> Result<Self::Error, String>;
}

impl sealed::Sealed for Legacy {}
#[cfg(feature = "s_incremental")]
impl sealed::Sealed for Charged {}

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
    fn decode(e: rusqlite::types::FromSqlError) -> String {
        error(e)
    }
    fn external(e: impl std::fmt::Display, _: &'static str) -> String {
        error(e)
    }
    fn codec(e: CodecError, _: &'static str) -> String {
        error(e)
    }
    fn root_digest(r: &ContentRef) -> Result<(), String> {
        r.digest().map(drop).map_err(error)
    }
    fn guard_roots(_: &[ContentRef; 4]) -> Result<(), String> {
        Ok(())
    }
    #[cfg(feature = "s_incremental")]
    fn frontier_source(&self, _: &dyn sc::NodeSource) -> Result<(), String> {
        Ok(())
    }
    fn diagnostics(&self) -> Option<newchan_rust::s_content::QuietDiagnostics> {
        None
    }
    #[cfg(feature = "s_incremental")]
    fn source_receipt(e: sc::ContextError) -> String {
        e.into_message()
    }
    fn detached() -> String {
        error("台账已丢弃")
    }
    fn invalid(text: &'static str) -> String {
        error(text)
    }
    type Text = String;
    fn previous_fold(&self) -> Result<String, String> {
        self.query_row(
            "SELECT fold_digest FROM meta WHERE singleton=1",
            [],
            |[v]| text::<Self>(v).map(str::to_owned),
        )
    }
    fn stage_texts<'u>(
        &self,
        previous: &str,
        d: &GenerationDelta,
        units: impl Iterator<Item = &'u Unit> + Clone,
        verifier_revision: &str,
    ) -> Result<StageDigests<String>, String> {
        let texts = legacy_texts(previous, d, units, verifier_revision)?;
        Ok(StageDigests {
            roots: texts.roots,
            units: sha256_hex(texts.units.as_bytes()),
            fold: sha256_hex(texts.fold.as_bytes()),
        })
    }
    type Path = PathBuf;
    type Rule = String;
    fn storage_bytes(path: &PathBuf) -> Result<u64, String> {
        database_bytes(path)
    }
    fn create_new(path: &PathBuf) -> Result<(), String> {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map(drop)
            .map_err(error)
    }
    fn format_hash() -> String {
        sha256_hex(FORMAT.as_bytes())
    }
    fn pending(
        &self,
        token: &str,
        generation: i64,
        roots: [RefView<'_>; 4],
        work: Option<&WorkScope>,
    ) -> Result<Pending, String> {
        Pending::new(token, generation, roots, work)
    }
    #[cfg(feature = "s_incremental")]
    type Units = sc::CodecVec<Unit>;
    #[cfg(feature = "s_incremental")]
    fn new_units(
        &self,
        verified: &sc::closure_delta::VerifiedUnits,
        work: Option<&WorkScope>,
        committed: impl Fn(&Unit) -> Result<bool, String>,
    ) -> Result<sc::CodecVec<Unit>, String> {
        let mut units = sc::CodecVec::default();
        verified.try_for_each_unit(|u| -> Result<(), String> {
            let unit = Unit::of_closure(u);
            if !committed(&unit)? {
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
        Ok(units)
    }
    fn prior_mismatch(detail: Option<String>) -> String {
        let detail = detail.unwrap_or_else(|| "字段不同".to_owned());
        error(format!(
            "上一代资格与台账当前已提交状态不符，不续写折叠链：{detail}"
        ))
    }
    fn frontier_error(e: String) -> Result<String, String> {
        Err(e)
    }
}

/// 受管台账的错误。受管操作口的拒绝与 SQLite 结果码在产生时按值保存为 [`SqlGate`]，
/// 不分配、不依赖之后可能被其他冻结域改写的绑定首拒槽；文本只在输出边界格式化。
#[cfg(feature = "s_incremental")]
#[derive(Debug, PartialEq, Eq)]
pub(super) enum LedgerError {
    /// 台账业务判据或行解码失败（原诊断文本）。
    Stale(String),
    /// 本次受管操作的结果分类；NOMEM 背后有本域首拒时记为该首拒。
    Gate(SqlGate),
    /// 连接已在失效时丢弃。
    Detached,
    /// 台账判据失败，静态文本。
    Invalid(&'static str),
    /// 存储读数的系统错误类别。
    Storage(std::io::ErrorKind),
    /// 原父 Rust 许可不足（分配前预留被拒）。
    Budget,
    /// codec 工作域报告的非额度 scratch 失败（台账、分配）。
    Scratch(ScratchFailure),
}

#[cfg(feature = "s_incremental")]
impl From<String> for LedgerError {
    fn from(text: String) -> Self {
        Self::Stale(text)
    }
}

#[cfg(feature = "s_incremental")]
impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stale(text) => f.write_str(text),
            Self::Gate(gate) => write!(f, "AuditLedgerStale：{gate:?}"),
            Self::Detached => f.write_str("AuditLedgerStale：台账已丢弃"),
            Self::Invalid(text) => write!(f, "AuditLedgerStale：{text}"),
            Self::Storage(kind) => write!(f, "AuditLedgerStale：{kind}"),
            Self::Budget => f.write_str("AuditLedgerStale：原父 Rust 许可不足"),
            Self::Scratch(failure) => write!(f, "AuditLedgerStale：codec 工作域 {failure:?}"),
        }
    }
}

#[cfg(feature = "s_incremental")]
impl From<LedgerError> for String {
    fn from(e: LedgerError) -> Self {
        match e {
            LedgerError::Stale(text) => text,
            other => other.to_string(),
        }
    }
}

#[cfg(feature = "s_incremental")]
impl LedgerError {
    /// 接既有冷认证错误层：本次的 Budget 首拒成为 `BudgetExceeded`，其余保留为诊断。
    #[cfg(feature = "s_incremental")]
    #[allow(
        dead_code,
        reason = "#1419 3C：冷认证接受管台账时经此接 cold_failure，首单元只由锁消费"
    )]
    pub(super) fn into_failure(self) -> super::audit_owner::native::Failure {
        match self {
            Self::Budget => ScratchFailure::Budget.into(),
            Self::Scratch(failure)
            | Self::Gate(SqlGate::Refused(
                failure @ (ScratchFailure::Budget | ScratchFailure::Allocation),
            )) => failure.into(),
            other => super::audit_owner::native::Failure::Invalid(other.into()),
        }
    }
}

#[cfg(feature = "s_incremental")]
/// 受管台账的原父收费能力：同一 Native 父账当前冻结域的 Rust 许可来源与 SQLite 绑定。构造时
/// 核对二者属于同一父账、同一 origin；台账的 Rust 持有与 SQLite 分配都只记在这里。
pub(super) struct ChargedParent {
    budget: Budget,
    binding: Rc<SqliteBinding>,
}

#[cfg(feature = "s_incremental")]
impl ChargedParent {
    #[allow(
        dead_code,
        reason = "#1419 3C：受管台账只由全新进程 release 锁构造，Native 一次启用波接入生产"
    )]
    pub(super) fn new(budget: Budget, binding: Rc<SqliteBinding>) -> Result<Self, LedgerError> {
        let (parent, origin) = budget.sqlite_parent();
        if !binding.serves(&parent, origin) {
            return Err(LedgerError::Gate(SqlGate::WrongParent));
        }
        let parent = Self { budget, binding };
        parent.admit()?;
        Ok(parent)
    }
    /// 此刻能否开始普通工作：绑定仍是当前 epoch、本域无首拒、冻结域存活。任何新持有、文件
    /// 副作用或 SQL 之前都先过这一关，拒绝按本次分类返回。
    fn admit(&self) -> Result<(), LedgerError> {
        self.binding.admitted().map_err(LedgerError::Gate)
    }
    /// 分配前预留；先过 [`Self::admit`]，返回的许可须与载荷同寿命（`Permit::hold`）。
    fn reserve(&self, bytes: usize) -> Result<Permit, LedgerError> {
        self.admit()?;
        self.budget.reserve(bytes).map_err(|_| LedgerError::Budget)
    }
    /// 按十进制文本排好的 `i64` 键条目，索引本身由原父许可持有到调用方释放。
    fn sorted<'a, T>(
        &self,
        map: &'a BTreeMap<i64, T>,
    ) -> Result<Reserved<Vec<(i64, &'a T)>>, LedgerError> {
        let bytes = map
            .len()
            .checked_mul(std::mem::size_of::<(i64, &T)>())
            .ok_or(LedgerError::Budget)?;
        let permit = self.reserve(bytes)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(map.len())
            .map_err(|_| LedgerError::Budget)?;
        entries.extend(map.iter().map(|(k, v)| (*k, v)));
        canonical::sort_by_decimal(&mut entries);
        Ok(permit.hold(entries))
    }
    /// 写出 `value` 的规范文本到按精确长度预留的缓冲。
    fn text<T: serde::Serialize + ?Sized>(&self, value: &T) -> Result<ChargedText, LedgerError> {
        let len = canonical::json_len(value).map_err(|_| LedgerError::Invalid("规范写出失败"))?;
        let permit = self.reserve(len)?;
        let mut buf = Vec::new();
        buf.try_reserve_exact(len)
            .map_err(|_| LedgerError::Budget)?;
        canonical::write_into(&mut buf, value).map_err(|_| LedgerError::Invalid("规范写出失败"))?;
        let text = String::from_utf8(buf).map_err(|_| LedgerError::Invalid("规范写出失败"))?;
        Ok(ChargedText::Held(permit.finish(text.capacity()).hold(text)))
    }
}

#[cfg(feature = "s_incremental")]
/// 受管台账的文本持有：原父许可持有的文本，或栈上的摘要。
pub(super) enum ChargedText {
    Held(Reserved<String>),
    Hex(canonical::Hex),
}

#[cfg(feature = "s_incremental")]
impl std::ops::Deref for ChargedText {
    type Target = str;
    fn deref(&self) -> &str {
        match self {
            Self::Held(text) => text.as_str(),
            Self::Hex(hex) => hex.as_str(),
        }
    }
}

#[cfg(feature = "s_incremental")]
pub(super) struct Charged {
    conn: Option<ChargedConnection>,
    parent: ChargedParent,
}

#[cfg(feature = "s_incremental")]
impl Drop for Charged {
    /// 丢弃即回滚未提交的整代再关库，与旧连接析构的结果相同；两步都是收尾域，首拒之后仍执行。
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            let _ = self.parent.binding.rollback(&conn);
            if let Err((conn, _)) = self.parent.binding.close(conn) {
                drop(conn);
            }
        }
    }
}

#[cfg(feature = "s_incremental")]
impl Charged {
    /// 不在 Rust 堆上建任何持有：路径由调用方以其许可借出，SQL 文本为静态串或栈缓冲。
    fn open(path: &CStr, pages: i64, parent: ChargedParent) -> Result<Self, LedgerError> {
        let spec = OpenSpec {
            flags: rusqlite::ffi::SQLITE_OPEN_READWRITE | rusqlite::ffi::SQLITE_OPEN_NOMUTEX,
            busy_timeout_ms: Some(0),
            ..OpenSpec::default()
        };
        // SAFETY: the spec carries no caller callbacks, contexts or VFS.
        let conn = unsafe { parent.binding.open_with(path, &spec) }
            .map_err(|e| LedgerError::Gate(classify(&parent.binding, e)))?;
        let store = Self {
            conn: Some(conn),
            parent,
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
        self.parent
            .binding
            .run(conn, f)
            .map_err(|e| LedgerError::Gate(classify(&self.parent.binding, e)))
    }
}

#[cfg(feature = "s_incremental")]
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
        f: impl FnOnce([ValueRef<'_>; M]) -> Result<T, LedgerError>,
    ) -> Result<T, LedgerError> {
        self.run(|q| q.query_row(sql, params, f))?
    }
    fn decode(_: rusqlite::types::FromSqlError) -> LedgerError {
        LedgerError::Invalid("台账列类型无效")
    }
    fn external(_: impl std::fmt::Display, context: &'static str) -> LedgerError {
        LedgerError::Invalid(context)
    }
    fn codec(e: CodecError, context: &'static str) -> LedgerError {
        match e.scratch_failure() {
            Some(ScratchFailure::Budget) => LedgerError::Budget,
            Some(other) => LedgerError::Scratch(other),
            None => LedgerError::Invalid(context),
        }
    }
    fn root_digest(r: &ContentRef) -> Result<(), LedgerError> {
        newchan_rust::s_content::transport_digest_quiet(&r.format, &r.root)
            .map(drop)
            .map_err(|_| LedgerError::Invalid("台账根格式无效"))
    }
    fn guard_roots(roots: &[ContentRef; 4]) -> Result<(), LedgerError> {
        for r in roots {
            if r.format != newchan_rust::s_content::CONTENT_FORMAT {
                return Err(LedgerError::Invalid("闭包单位与待准备事实四根不匹配"));
            }
            newchan_rust::s_content::parse_root_quiet(&r.root)
                .map_err(|_| LedgerError::Invalid("闭包单位四根核验失败"))?;
        }
        Ok(())
    }
    fn diagnostics(&self) -> Option<newchan_rust::s_content::QuietDiagnostics> {
        Some(newchan_rust::s_content::QuietDiagnostics::enter())
    }
    fn frontier_source(&self, source: &dyn sc::NodeSource) -> Result<(), LedgerError> {
        self.parent.admit()?;
        match source.codec_work() {
            Some(work) if self.parent.budget.owns_work(work) => Ok(()),
            _ => Err(LedgerError::Gate(SqlGate::WrongParent)),
        }
    }
    fn source_receipt(_: sc::ContextError) -> LedgerError {
        LedgerError::Invalid("来源分区收据核验失败")
    }
    fn detached() -> LedgerError {
        LedgerError::Detached
    }
    fn invalid(text: &'static str) -> LedgerError {
        LedgerError::Invalid(text)
    }
    type Text = ChargedText;
    /// 先读字节长度、从原父预留，再把摘要复制进预留好的缓冲。
    fn previous_fold(&self) -> Result<ChargedText, LedgerError> {
        let len = self.query_row(
            "SELECT length(CAST(fold_digest AS BLOB)) FROM meta WHERE singleton=1",
            [],
            |[v]| int::<Self>(v),
        )?;
        let len = usize::try_from(len).map_err(|_| LedgerError::Invalid("折叠摘要长度无效"))?;
        let permit = self.parent.reserve(len)?;
        let mut out = String::new();
        out.try_reserve_exact(len)
            .map_err(|_| LedgerError::Budget)?;
        let copied = self.query_row(
            "SELECT fold_digest FROM meta WHERE singleton=1",
            [],
            |[v]| {
                let t = text::<Self>(v)?;
                if t.len() == len {
                    out.push_str(t);
                }
                Ok(t.len() == len)
            },
        )?;
        if !copied {
            return Err(LedgerError::Invalid("折叠摘要在两次读取间改变"));
        }
        Ok(ChargedText::Held(permit.finish(out.capacity()).hold(out)))
    }
    /// 流式写出：排序索引与 roots 文本由原父许可持有，摘要在栈上，不建 `Value`。
    fn stage_texts<'u>(
        &self,
        previous: &str,
        d: &GenerationDelta,
        units: impl Iterator<Item = &'u Unit> + Clone,
        verifier_revision: &str,
    ) -> Result<StageDigests<ChargedText>, LedgerError> {
        let raw = self.parent.sorted(&d.raw)?;
        let cover = self.parent.sorted(&d.cover)?;
        let written = |_| LedgerError::Invalid("规范写出失败");
        let fold = canonical::hash_hex(&canonical::Fold {
            previous,
            delta: canonical::Delta {
                d,
                verifier_revision,
                units: units.clone(),
                raw: &raw,
                cover: &cover,
            },
        })
        .map_err(written)?;
        let unit_digest = canonical::hash_hex(&canonical::Units(units)).map_err(written)?;
        Ok(StageDigests {
            roots: self.parent.text(&canonical::Roots(&d.roots))?,
            units: ChargedText::Hex(unit_digest),
            fold: ChargedText::Hex(fold),
        })
    }
    type Path = ChargedPath;
    type Rule = ChargedText;
    fn storage_bytes(path: &ChargedPath) -> Result<u64, LedgerError> {
        let mut total = 0u64;
        for file in path.files() {
            total = total
                .checked_add(stat_file(file)?)
                .ok_or(LedgerError::Invalid("存储计数溢出"))?;
        }
        Ok(total)
    }
    fn create_new(path: &ChargedPath) -> Result<(), LedgerError> {
        // SAFETY: the path is NUL-terminated and borrowed for the call; the fd is closed here.
        let fd = unsafe {
            libc::open(
                path.main().as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(LedgerError::Storage(std::io::Error::last_os_error().kind()));
        }
        // SAFETY: fd was just opened by this call and is not shared.
        unsafe { libc::close(fd) };
        Ok(())
    }
    fn format_hash() -> ChargedText {
        ChargedText::Hex(canonical::Hex::of_bytes(FORMAT.as_bytes()))
    }
    /// token 与四根按 `NativeValue` 的精确复制长度先从原父预留，许可随 Pending 释放。
    fn pending(
        &self,
        token: &str,
        generation: i64,
        roots: [RefView<'_>; 4],
        _work: Option<&WorkScope>,
    ) -> Result<Pending, LedgerError> {
        let bytes = roots
            .iter()
            .try_fold(token.len(), |n, r| {
                n.checked_add(r.format().len())?.checked_add(r.root().len())
            })
            .ok_or(LedgerError::Budget)?;
        let permit = self.parent.reserve(bytes)?;
        let copy = |_| LedgerError::Scratch(ScratchFailure::Allocation);
        Ok(Pending {
            token: NativeValue::<String>::try_from_str(token, None).map_err(copy)?,
            generation,
            roots: [
                NativeValue::<ContentRef>::try_from_view(roots[0], None).map_err(copy)?,
                NativeValue::<ContentRef>::try_from_view(roots[1], None).map_err(copy)?,
                NativeValue::<ContentRef>::try_from_view(roots[2], None).map_err(copy)?,
                NativeValue::<ContentRef>::try_from_view(roots[3], None).map_err(copy)?,
            ],
            fee: Some(permit),
        })
    }
    type Units = ChargedUnits;
    /// 先数出全部候选单位，按该上界从原父预留，再只放入未提交的单位，不在循环中增长。
    fn new_units(
        &self,
        verified: &sc::closure_delta::VerifiedUnits,
        _work: Option<&WorkScope>,
        committed: impl Fn(&Unit) -> Result<bool, LedgerError>,
    ) -> Result<ChargedUnits, LedgerError> {
        let mut total = 0usize;
        verified.try_for_each_unit(|_| -> Result<(), LedgerError> {
            total = total.checked_add(1).ok_or(LedgerError::Budget)?;
            Ok(())
        })?;
        let bytes = total
            .checked_mul(std::mem::size_of::<Unit>())
            .ok_or(LedgerError::Budget)?;
        let permit = self.parent.reserve(bytes)?;
        let mut units = Vec::new();
        units
            .try_reserve_exact(total)
            .map_err(|_| LedgerError::Budget)?;
        verified.try_for_each_unit(|u| -> Result<(), LedgerError> {
            let unit = Unit::of_closure(u);
            if !committed(&unit)? && units.len() < total {
                units.push(unit);
            }
            Ok(())
        })?;
        units.sort_unstable();
        Ok(ChargedUnits(permit.hold(units)))
    }
    /// 查询本身的分类按值透传；只有带文本的判据失败才拼接上下文。
    fn prior_mismatch(detail: Option<LedgerError>) -> LedgerError {
        match detail {
            None => {
                LedgerError::Invalid("上一代资格与台账当前已提交状态不符，不续写折叠链：字段不同")
            }
            Some(LedgerError::Stale(detail)) => LedgerError::Stale(error(format!(
                "上一代资格与台账当前已提交状态不符，不续写折叠链：{detail}"
            ))),
            Some(typed) => typed,
        }
    }
    fn frontier_error(e: LedgerError) -> Result<LedgerError, String> {
        match e {
            LedgerError::Stale(text) => Err(text),
            typed => Ok(typed),
        }
    }
}

/// 本次结果的分类：SQLite 报 NOMEM 且本域已有首拒时记为该首拒，不以结果码掩盖资源拒绝。
#[cfg(feature = "s_incremental")]
fn classify(binding: &SqliteBinding, e: SqlGate) -> SqlGate {
    match e {
        SqlGate::Sqlite(code) => binding
            .refusal_of(code)
            .map_or(SqlGate::Sqlite(code), SqlGate::Refused),
        other => other,
    }
}

#[cfg(feature = "s_incremental")]
/// 定长栈缓冲上的 SQL 文本，不经堆。
struct StackSql {
    bytes: [u8; 48],
    len: usize,
}

#[cfg(feature = "s_incremental")]
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

#[cfg(feature = "s_incremental")]
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

/// witness、observation 两表同形的 upsert（原按表名格式化拼接的同一文本）。
const RELATED_UPSERT: [&str; 2] = [
    "INSERT INTO witness VALUES(?1,?2,?3,?4)
                    ON CONFLICT(id) DO UPDATE SET record_digest=excluded.record_digest,
                    source_digest=excluded.source_digest, certified_generation=excluded.certified_generation",
    "INSERT INTO observation VALUES(?1,?2,?3,?4)
                    ON CONFLICT(id) DO UPDATE SET record_digest=excluded.record_digest,
                    source_digest=excluded.source_digest, certified_generation=excluded.certified_generation",
];

const PRAGMAS: &str =
    "PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;";

fn int<B: Backend>(v: ValueRef<'_>) -> Result<i64, B::Error> {
    v.as_i64().map_err(B::decode)
}
fn text<B: Backend>(v: ValueRef<'_>) -> Result<&str, B::Error> {
    v.as_str().map_err(B::decode)
}

pub(super) struct Ledger<B: Backend = Legacy> {
    connection: Option<B>,
    phase: Phase,
    coverage: Coverage,
    path: B::Path,
    content_path: B::Path,
    max_store_bytes: u64,
    rule_revision: B::Rule,
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
fn legacy_file_bytes(path: &Path) -> Result<u64, String> {
    match std::fs::metadata(path) {
        Ok(m) => Ok(m.len()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(error(e)),
    }
}
fn database_bytes(path: &Path) -> Result<u64, String> {
    let mut n = legacy_file_bytes(path)?;
    for suffix in ["-wal", "-journal"] {
        let mut p = path.as_os_str().to_os_string();
        p.push(suffix);
        n = n
            .checked_add(legacy_file_bytes(Path::new(&p))?)
            .ok_or_else(|| error("存储计数溢出"))?;
    }
    Ok(n)
}

#[cfg(feature = "s_incremental")]
/// 原父许可持有的单位集合。
pub(super) struct ChargedUnits(Reserved<Vec<Unit>>);

#[cfg(feature = "s_incremental")]
impl std::ops::Deref for ChargedUnits {
    type Target = [Unit];
    fn deref(&self) -> &[Unit] {
        &self.0
    }
}

#[cfg(feature = "s_incremental")]
/// 原父许可持有的数据库路径：主文件、`-wal`、`-journal` 三个 NUL 结尾副本连续存放。读数与
/// 创建直接借用这些副本，不再拼接，也不设项目自定的长度上限。
pub(super) struct ChargedPath {
    bytes: Reserved<Vec<u8>>,
    base: usize,
}

#[cfg(feature = "s_incremental")]
impl ChargedPath {
    fn new(path: &CStr, parent: &ChargedParent) -> Result<Self, LedgerError> {
        let base = path.to_bytes().len();
        let total = base
            .checked_mul(3)
            .and_then(|n| n.checked_add(3 + "-wal".len() + "-journal".len()))
            .ok_or(LedgerError::Budget)?;
        let permit = parent.reserve(total)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(total)
            .map_err(|_| LedgerError::Budget)?;
        for suffix in [&b""[..], b"-wal", b"-journal"] {
            bytes.extend_from_slice(path.to_bytes());
            bytes.extend_from_slice(suffix);
            bytes.push(0);
        }
        Ok(Self {
            bytes: permit.hold(bytes),
            base,
        })
    }
    /// 只接受已规范的绝对路径：以 `/` 开头，无空、`.`、`..` 段与尾随 `/`，且每一级前缀都不是
    /// 符号链接。逐前缀 `lstat` 在按真实长度预留的工作副本上放置 NUL，不调用 `realpath`。
    fn canonical(path: &CStr, parent: &ChargedParent) -> Result<Self, LedgerError> {
        let raw = path.to_bytes();
        let shaped = raw.first() == Some(&b'/')
            && (raw.len() == 1 || !raw.ends_with(b"/"))
            && raw[1..]
                .split(|b| *b == b'/')
                .all(|c| !c.is_empty() && c != b"." && c != b"..");
        if !shaped {
            return Err(LedgerError::Invalid("内容库路径不是规范绝对路径"));
        }
        let held = Self::new(path, parent)?;
        let permit = parent.reserve(raw.len() + 1)?;
        let mut work = Vec::new();
        work.try_reserve_exact(raw.len() + 1)
            .map_err(|_| LedgerError::Budget)?;
        work.extend_from_slice(raw);
        work.push(0);
        let mut work = permit.hold(work);
        for end in (1..raw.len())
            .filter(|&i| raw[i] == b'/')
            .chain([raw.len()])
        {
            work[end] = 0;
            let prefix = CStr::from_bytes_with_nul(&work[..=end])
                .map_err(|_| LedgerError::Invalid("内容库路径含 NUL"))?;
            let mut st = std::mem::MaybeUninit::<libc::stat>::uninit();
            // SAFETY: prefix is NUL-terminated; st is written by lstat on success.
            if unsafe { libc::lstat(prefix.as_ptr(), st.as_mut_ptr()) } != 0 {
                return Err(LedgerError::Storage(std::io::Error::last_os_error().kind()));
            }
            // SAFETY: lstat returned 0, so st is initialized.
            let mode = unsafe { st.assume_init() }.st_mode;
            if mode & libc::S_IFMT == libc::S_IFLNK {
                return Err(LedgerError::Invalid("内容库路径含符号链接"));
            }
            if end < raw.len() {
                work[end] = b'/';
            }
        }
        Ok(held)
    }
    fn main(&self) -> &CStr {
        CStr::from_bytes_with_nul(&self.bytes[..=self.base]).unwrap_or_default()
    }
    fn files(&self) -> impl Iterator<Item = &CStr> {
        self.bytes
            .split_inclusive(|b| *b == 0)
            .filter_map(|file| CStr::from_bytes_with_nul(file).ok())
    }
}

#[cfg(feature = "s_incremental")]
/// 借用的 NUL 路径上的 `stat`；不存在计 0，不经堆。
fn stat_file(path: &CStr) -> Result<u64, LedgerError> {
    let mut st = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: path is NUL-terminated; st is written by stat on success.
    if unsafe { libc::stat(path.as_ptr(), st.as_mut_ptr()) } == 0 {
        // SAFETY: stat returned 0, so st is initialized.
        let size = unsafe { st.assume_init() }.st_size;
        return u64::try_from(size).map_err(|_| LedgerError::Invalid("存储计数溢出"));
    }
    let e = std::io::Error::last_os_error();
    if e.kind() == std::io::ErrorKind::NotFound {
        Ok(0)
    } else {
        Err(LedgerError::Storage(e.kind()))
    }
}

impl Ledger {
    /// H1 的私有点查资格：绑定已提交完整水位、四根、规则、verifier 与内容库身份。
    /// 不含连接、不可跨进程；任何字段不符即不能用它点查。拥有型输出无上界，只开放给
    /// Legacy；受管台账经调用方预付的 `qualification_bounded` 取得。
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
        Self::fresh_in(
            path.to_owned(),
            content_path,
            identity.rule_revision,
            &identity.content_identity,
            &identity.binary_commit,
            max_store_bytes,
            |path, pages| Legacy::open(path, pages),
        )
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

#[cfg(feature = "s_incremental")]
impl Ledger<Charged> {
    /// 资格的拥有型副本：六段文本的精确字节在复制前从原父预留，许可随返回值持有、随其释放。
    /// 预留发生在本次已准入的点查映射内，不另做 SQL。
    pub(super) fn qualification_reserved(&self) -> Result<Reserved<Qualification>, LedgerError> {
        let budget = &self.store()?.parent.budget;
        self.with_committed_qualification(|v| {
            let permit = budget
                .reserve(v.string_bytes())
                .map_err(|_| LedgerError::Budget)?;
            Ok(permit.hold(v.owned()))
        })
    }
    /// 受管构造：同一份阶段机与 SQL，连接由 `binding` 当前绑定的原 Native 父账开出，SQLite
    /// 分配向该父账收费。调用方须已在本进程首次 SQLite 初始化前装好 allocator；本构造不安装，
    /// 失败即失败，不改用旧连接。
    #[allow(
        dead_code,
        reason = "#1419 3C：受管台账只由全新进程 release 锁消费，Native 一次启用波接入生产"
    )]
    pub(super) fn fresh_charged(
        path: &CStr,
        content_path: &CStr,
        identity: &Identity,
        max_store_bytes: u64,
        parent: ChargedParent,
    ) -> Result<Self, LedgerError> {
        if identity.rule_revision.is_empty()
            || identity.content_identity.is_empty()
            || identity.binary_commit.is_empty()
        {
            return Err(LedgerError::Invalid("台账身份字段不能为空"));
        }
        let content = ChargedPath::canonical(content_path, &parent)?;
        let ledger = ChargedPath::new(path, &parent)?;
        parent.admit()?;
        let rule = parent
            .budget
            .string(&identity.rule_revision)
            .map_err(|_| LedgerError::Budget)?;
        parent.admit()?;
        Self::fresh_in(
            ledger,
            content,
            ChargedText::Held(rule),
            &identity.content_identity,
            &identity.binary_commit,
            max_store_bytes,
            |path, pages| Charged::open(path.main(), pages, parent),
        )
    }
}

impl<B: Backend> Ledger<B> {
    fn fresh_in(
        path: B::Path,
        content_path: B::Path,
        rule_revision: B::Rule,
        content_identity: &str,
        binary_commit: &str,
        max_store_bytes: u64,
        open: impl FnOnce(&B::Path, i64) -> Result<B, B::Error>,
    ) -> Result<Self, B::Error> {
        let remaining = max_store_bytes
            .checked_sub(B::storage_bytes(&content_path)?)
            .ok_or_else(|| B::invalid("内容库已超过 max_store_bytes"))?;
        let pages = i64::try_from(remaining / 4096).map_err(|e| B::external(e, "台账页数溢出"))?;
        if pages == 0 {
            return Err(B::invalid("没有台账页空间"));
        }
        B::create_new(&path)?;
        let conn = open(&path, pages)?;
        conn.execute_batch(SCHEMA)?;
        conn.execute(
            "INSERT INTO meta VALUES(1,?1,?2,?3,?4,0,?5)",
            [
                &FORMAT,
                &&*rule_revision,
                &content_identity,
                &binary_commit,
                &&*B::format_hash(),
            ],
        )?;
        let ledger = Self {
            connection: Some(conn),
            phase: Phase::Ready,
            coverage: Coverage::Complete,
            path,
            content_path,
            max_store_bytes,
            rule_revision,
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
            _ => Err(B::invalid("不是已提交的来源分区")),
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
        f: impl FnOnce(&QualificationView<'_>) -> Result<R, B::Error>,
    ) -> Result<R, B::Error> {
        let through = self.committed_through()?;
        if through < 1 {
            return Err(B::invalid("完整台账尚无已提交的一代"));
        }
        self.store()?.query_row(
            "SELECT c.roots, c.fold_digest, c.verifier_revision, m.fold_digest, m.content_identity, \
             m.binary_commit, m.rule_revision FROM cert c, meta m WHERE c.generation=?1 AND m.singleton=1",
            [&through],
            |[roots, fold, verifier, meta_fold, content, binary, rule]| {
                let fold_digest = text::<B>(fold)?;
                if text::<B>(meta_fold)? != fold_digest {
                    return Err(B::invalid("完整水位与末代 cert 折叠摘要不一致"));
                }
                let rule_revision = text::<B>(rule)?;
                if rule_revision != &*self.rule_revision {
                    return Err(B::invalid("台账持久规则与安装规则不一致"));
                }
                f(&QualificationView {
                    through,
                    roots: text::<B>(roots)?,
                    fold_digest,
                    verifier_revision: text::<B>(verifier)?,
                    rule_revision,
                    content_identity: text::<B>(content)?,
                    binary_commit: text::<B>(binary)?,
                })
            },
        )
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
            return Err(B::invalid("点查资格与当前已提交台账不符"));
        }
        self.contains_committed_unit(unit)
    }
    #[cfg(feature = "s_incremental")]
    pub(super) fn raw_partition_through(&self) -> Result<i64, B::Error> {
        match (&self.phase, &self.coverage) {
            (Phase::Ready, Coverage::RawPartition { through }) => Ok(*through),
            _ => Err(B::invalid("不是已提交的raw分区")),
        }
    }
    fn check_storage(&self) -> Result<(), B::Error> {
        let size = B::storage_bytes(&self.content_path)?
            .checked_add(B::storage_bytes(&self.path)?)
            .ok_or_else(|| B::invalid("存储合账溢出"))?;
        if size > self.max_store_bytes {
            return Err(B::invalid("内容库/WAL/台账合计超过 max_store_bytes"));
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
            |[v]| int::<B>(v),
        )
    }
    /// 已提交存储读数，不是 U-A 资格；准备中连本连接的未提交行也不暴露。
    pub(super) fn committed_through(&self) -> Result<i64, B::Error> {
        if !matches!(self.phase, Phase::Ready) || !matches!(self.coverage, Coverage::Complete) {
            return Err(B::invalid("完整台账尚未提交或仅有raw分区"));
        }
        self.through()
    }
    pub(super) fn contains_committed_unit(&self, unit: &Unit) -> Result<bool, B::Error> {
        let through = self.committed_through()?;
        self.store()?.query_row(
            "SELECT EXISTS(SELECT 1 FROM node WHERE digest=?1 AND flavor=?2 AND mode=?3 AND first_generation<=?4)",
            [&unit.digest.as_slice(), &unit.flavor(), &unit.mode(), &through],
            |[v]| int::<B>(v).map(|n| n != 0),
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
        self.store()?.frontier_source(source)?;
        if !self.matches_committed(qualification)? {
            return Err(B::invalid("Δ核验资格与当前已提交台账不符"));
        }
        let view = QualifiedSuccessView::new(self, qualification);
        let quiet = self.store()?.diagnostics();
        let verified = sc::closure_delta::verify_four_roots_from(source, roots, &view);
        drop(quiet);
        let verified = match verified {
            Ok(verified) => verified,
            Err(e) => {
                return Err(match view.refused.into_inner() {
                    Some(typed) => typed,
                    None => B::codec(e, "Δ闭包核验失败"),
                })
            }
        };
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
                    return Err(B::invalid("Δ单位绑定资格与上一代资格不符"));
                }
            }
            let verified = &verified.verified;
            B::guard_roots(&delta.roots)?;
            let quiet = self.store()?.diagnostics();
            let matched = verified.matches_roots(delta.roots.each_ref().map(Into::into));
            drop(quiet);
            if !matched.map_err(|e| B::codec(e, "闭包单位四根核验失败"))? {
                return Err(B::invalid("闭包单位与待准备事实四根不匹配"));
            }
            let units = self
                .store()?
                .new_units(verified, work, |unit| self.contains_committed_unit(unit))?;
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
            return Err(B::invalid("台账不在完整事实可准备状态"));
        }
        if token.is_empty()
            || verifier_revision.is_empty()
            || self.through()?.checked_add(1) != Some(d.generation)
        {
            return Err(B::invalid("请求或逐代序号无效"));
        }
        for r in &d.roots {
            B::root_digest(r)?;
        }
        self.check_storage()?;
        let conn = self.store()?;
        conn.execute_batch("BEGIN IMMEDIATE")?;
        if let Some(prior) = prior {
            match self.matches_committed(prior) {
                Ok(true) => {}
                Ok(false) => return Err(B::prior_mismatch(None)),
                Err(e) => return Err(B::prior_mismatch(Some(e))),
            }
        }
        let previous = conn.previous_fold()?;
        let mut next: i64 =
            conn.query_row("SELECT COALESCE(MAX(seq),-1)+1 FROM raw", [], |[v]| {
                int::<B>(v)
            })?;
        for (seq, row) in &d.raw {
            if *seq != next {
                return Err(B::invalid("raw 只允许连续追加"));
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
            next = next
                .checked_add(1)
                .ok_or_else(|| B::invalid("raw 序号溢出"))?;
        }
        for (id, r) in &d.objects {
            if r.first_generation < 1
                || r.first_generation > r.published_generation
                || r.published_generation > d.generation
            {
                return Err(B::invalid("对象代戳无效"));
            }
            conn.execute("INSERT INTO object VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
                ON CONFLICT(id) DO UPDATE SET record_digest=excluded.record_digest,
                first_generation=excluded.first_generation, first_cut=excluded.first_cut,
                published_generation=excluded.published_generation, source_digest=excluded.source_digest,
                withdrawn=excluded.withdrawn, certified_generation=excluded.certified_generation",
                [&id, &r.record_digest.as_slice(), &r.first_generation, &r.first_cut, &r.published_generation, &r.source_digest.as_slice(), &r.withdrawn, &d.generation])?;
        }
        for (sql, rows) in [
            (RELATED_UPSERT[0], &d.witnesses),
            (RELATED_UPSERT[1], &d.observations),
        ] {
            for (id, r) in rows {
                conn.execute(
                    sql,
                    [
                        &id,
                        &r.record_digest.as_slice(),
                        &r.source_digest.as_slice(),
                        &d.generation,
                    ],
                )?;
            }
        }
        for id in &d.withdrawals {
            let changed = conn.execute(
                "UPDATE object SET withdrawn=1,certified_generation=?1 WHERE id=?2 AND withdrawn=0",
                [&d.generation, &id],
            )?;
            if changed != 1 {
                return Err(B::invalid("撤回身份缺失或已撤回"));
            }
        }
        for (coord, change) in &d.cover {
            match change {
                CoverChange::Removed => {
                    if conn.execute("DELETE FROM cover WHERE source_coord=?1", [coord])? != 1 {
                        return Err(B::invalid("cover 失效坐标不在当前分区"));
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
        let covered: i64 = conn.query_row("SELECT COUNT(*) FROM cover", [], |[v]| int::<B>(v))?;
        if covered != d.effective {
            return Err(B::invalid("cover 行数不等于本 cut 生效坐标总数"));
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
        let unit_count = new_units.clone().count();
        let digests = conn.stage_texts(&previous, d, new_units, verifier_revision)?;
        conn.execute(
            "INSERT INTO cert VALUES(?1,?2,?3,?4,?5,?6)",
            [
                &d.generation,
                &&*digests.roots,
                &&*digests.units,
                &i64::try_from(unit_count).map_err(|e| B::external(e, "单位计数溢出"))?,
                &&*digests.fold,
                &verifier_revision,
            ],
        )?;
        conn.execute(
            "UPDATE meta SET certified_through_generation=?1,fold_digest=?2 WHERE singleton=1",
            [&d.generation, &&*digests.fold],
        )?;
        self.check_storage()?;
        self.phase = Phase::Pending(conn.pending(
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
            return Err(B::invalid("来源分区格式不匹配"));
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
            return Err(B::invalid("来源分区格式不匹配"));
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
                || *metadata.rule_revision != *self.rule_revision
            {
                return Err(B::invalid("raw分区代号或规则不连续"));
            }
            let range = source
                .check_raw_partition(token, metadata, roots)
                .map_err(B::source_receipt)?;
            let r = roots.view();
            let pending = self.store()?.pending(
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
                .ok_or_else(|| B::invalid("raw分区已丢弃"))?;
            conn.execute_batch("BEGIN IMMEDIATE")?;
            let mut next: i64 =
                conn.query_row("SELECT COALESCE(MAX(seq),-1)+1 FROM raw", [], |[v]| {
                    int::<B>(v)
                })?;
            if usize::try_from(next).ok() != Some(range.start) || source.rows().len() != range.len()
            {
                return Err(B::invalid("raw来源后缀与已提交分区不衔接"));
            }
            for row in source.rows() {
                if row.seq != next {
                    return Err(B::invalid("raw来源后缀不连续"));
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
                next = next
                    .checked_add(1)
                    .ok_or_else(|| B::invalid("raw序号溢出"))?;
            }
            if self.is_source_partitions() {
                let rows = source.business_rows().map_err(B::source_receipt)?;
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
                        return Err(B::invalid("来源分区撤回身份缺失或已撤回"));
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
                .map_err(B::source_receipt)?;
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
            Err(B::invalid("发布缺本请求台账变化或台账已失效"))
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
                    && m.rule_revision == &*self.rule_revision =>
            {
                if p.roots.iter().zip(expected).all(|(a, b)| {
                    a.format == b.format()
                        && a.root == b.root()
                        && a.kind == b.kind()
                        && a.extent == b.extent()
                }) {
                    Ok(())
                } else {
                    Err(B::invalid("台账四根与本请求 prepare 不一致"))
                }
            }
            _ => Err(B::invalid("台账请求、规则或代号不一致")),
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
                return Err(B::invalid("台账尚未绑定实际 prepare"));
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
            inner: QualifiedSuccessView<'a, Legacy>,
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
                        inner: QualifiedSuccessView::new(&b, &qb),
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

        pub(super) const M8D_LEDGER_EXISTS: &str =
            "SELECT EXISTS(SELECT 1 FROM node WHERE digest=?1 AND flavor=?2 AND mode=?3 AND first_generation<=?4)";
        pub(super) const M8D_LEDGER_QUALIFICATION: &str = "SELECT c.roots, c.fold_digest, c.verifier_revision, m.fold_digest, m.content_identity, m.binary_commit, m.rule_revision FROM cert c, meta m WHERE c.generation=?1 AND m.singleton=1";
        pub(super) const M8D_LEDGER_THROUGH: &str =
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
                    inner: QualifiedSuccessView::new(&b, &q),
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
                let view = QualifiedSuccessView::new(&l, &q);
                let delta_err = cd::verify_four_roots_from(&mut nodes, refs(&r), &view)
                    .err()
                    .unwrap();
                // 损坏节点是 codec 判据失败，前沿点查没有拒绝；Legacy 前沿错误只有文本，也不留 typed 拒绝。
                assert!(view.refused.into_inner().is_none());
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
                let view = QualifiedSuccessView::new(&l, &q);
                // 前沿点查失败即整体失败，不当 miss 退回完整核验。
                assert!(cd::verify_four_roots_from(&mut nodes, refs(&gens[1]), &view).is_err());
                // Legacy 的点查拒绝（未提交）按原文本经 codec SOURCE 返回，不冒充 typed 拒绝。
                assert!(view.refused.into_inner().is_none());
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

    /// 测试用授权回调：在指定读取处于操作域内申请超额块，使本域首拒恰好落在该次 SQL 上。
    /// `TRIP_MODE` 1 为首次读 node；2 为看到 BEGIN 之后首次读 meta。只触发一次，不改授权结果。
    #[cfg(feature = "s_incremental")]
    static TRIP_MODE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    #[cfg(feature = "s_incremental")]
    static TRIP_ARMED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    #[cfg(feature = "s_incremental")]
    static TRIPPED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    #[cfg(feature = "s_incremental")]
    unsafe extern "C" fn trip_refusal(
        _: *mut std::ffi::c_void,
        action: std::ffi::c_int,
        table: *const std::ffi::c_char,
        _: *const std::ffi::c_char,
        _: *const std::ffi::c_char,
        _: *const std::ffi::c_char,
    ) -> std::ffi::c_int {
        use rusqlite::ffi;
        use std::sync::atomic::Ordering;
        let mode = TRIP_MODE.load(Ordering::SeqCst);
        if action == ffi::SQLITE_TRANSACTION && mode == 2 {
            TRIP_ARMED.store(true, Ordering::SeqCst);
        }
        if action == ffi::SQLITE_READ && !table.is_null() {
            // SAFETY: SQLite passes a NUL-terminated table name for SQLITE_READ.
            let table = unsafe { std::ffi::CStr::from_ptr(table) };
            let hit = (mode == 1 && table == c"node")
                || (mode == 2 && TRIP_ARMED.load(Ordering::SeqCst) && table == c"meta");
            if hit && !TRIPPED.swap(true, Ordering::SeqCst) {
                // SAFETY: plain SQLite allocation inside the operation domain; null is expected.
                let block = unsafe { ffi::sqlite3_malloc64(64 << 20) };
                // SAFETY: freeing whatever was returned (null is a no-op).
                unsafe { ffi::sqlite3_free(block) };
            }
        }
        ffi::SQLITE_OK
    }

    /// 带工作域的节点来源；`starve` 时每次读取先向工作域申请超额许可，按真实接口在中途失败。
    #[cfg(feature = "s_incremental")]
    struct Scoped<'a> {
        nodes: &'a mut newchan_rust::s_content::MemoryNodes,
        scope: &'a WorkScope,
        fault: Fault,
        /// 预先读好的 `(摘要, 节点)`，按读取顺序从尾部交出，使 source 自身不在被测窗口内分配。
        prepared: Vec<([u8; 32], newchan_rust::s_content::NodeRead)>,
        /// 实际请求的摘要序列；容量在窗口外预留，窗口内追加不扩容。
        requested: Vec<[u8; 32]>,
        /// 某次请求的摘要与当次交出的预备节点不符。
        mismatched: bool,
        /// 请求数超过窗口外预留的容量；多出的请求未记录，路径锁按失败处理。
        overflow: bool,
    }

    #[cfg(feature = "s_incremental")]
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Fault {
        None,
        Budget,
        Allocation,
    }

    #[cfg(feature = "s_incremental")]
    impl newchan_rust::s_content::NodeSource for Scoped<'_> {
        fn codec_work(&self) -> Option<&WorkScope> {
            Some(self.scope)
        }
        fn read(
            &mut self,
            digest: &newchan_rust::s_content::NodeDigest,
        ) -> Result<newchan_rust::s_content::NodeRead, CodecError> {
            // 先记请求再按注入失败：被拒的那次读取也属于路径。
            if self.requested.len() < self.requested.capacity() {
                self.requested.push(*digest);
            } else {
                self.overflow = true;
            }
            match self.fault {
                Fault::None => {}
                Fault::Budget => drop(self.scope.reserve_native(usize::MAX / 4)?),
                Fault::Allocation => return Err(self.scope.reject(ScratchFailure::Allocation)),
            }
            if let Some((expected, found)) = self.prepared.pop() {
                self.mismatched |= expected != *digest;
                return Ok(found);
            }
            newchan_rust::s_content::NodeSource::read(&mut *self.nodes, digest)
        }
    }

    /// 在 `budget` 的新 codec 工作域里用 `nodes` 调一次 `verify_delta`，只带出错误（成功为
    /// `None`）；核验结果在工作域内释放。
    #[cfg(feature = "s_incremental")]
    fn verify_in(
        budget: &super::super::audit_owner::native::Budget,
        l: &Ledger<Charged>,
        nodes: &mut newchan_rust::s_content::MemoryNodes,
        root: &ContentRef,
        q: &Qualification,
        fault: Fault,
    ) -> Option<LedgerError> {
        verify_measured(budget, l, nodes, root, q, fault, false).error
    }

    /// 窗口内 Rust 请求 layout 的拆分：SQLite 块由组件 allocator 以 Rust layout 分配、按块完整
    /// 字节经同父 census 记为 "sqlite" 预留，窗口读数减去它们即其余 Rust 分配。
    #[cfg(feature = "s_incremental")]
    struct RustWindow {
        rust: u64,
        sqlite: u64,
        probe: serde_json::Value,
    }
    #[cfg(feature = "s_incremental")]
    impl std::fmt::Display for RustWindow {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "rust={} sqlite={} {}",
                self.rust, self.sqlite, self.probe
            )
        }
    }
    /// 对 `run` 同时开 Rust 分配窗口与原父 census，把窗口读数拆成 SQLite 块与其余 Rust 分配。
    #[cfg(feature = "s_incremental")]
    fn rust_window<R>(
        budget: &super::super::audit_owner::native::Budget,
        run: impl FnOnce() -> R,
    ) -> (R, RustWindow) {
        use super::super::audit_owner::allocation_probe::Window;
        budget.census_arm();
        let window = Window::start();
        let result = run();
        let probe = window.finish();
        let sqlite = budget
            .census_take()
            .iter()
            .filter(|e| e.reserve && e.kind == "sqlite")
            .map(|e| e.bytes as u64)
            .sum::<u64>();
        let allocated = probe["allocated_bytes"].as_u64().unwrap();
        let rust = allocated
            .checked_sub(sqlite)
            .expect("每个 SQLite 块都是窗口内的一次 Rust layout 分配");
        (
            result,
            RustWindow {
                rust,
                sqlite,
                probe,
            },
        )
    }

    /// 只供两项隔离负控：在同一窗口内 `verify_delta` 之后追加一次无账 Rust 临时申请，或一条
    /// 多余的台账 SQL。
    #[cfg(feature = "s_incremental")]
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Extra {
        None,
        RustTemp,
        LedgerSql,
    }

    /// [`verify_measured_with`] 的调用方式。
    #[cfg(feature = "s_incremental")]
    #[derive(Clone, Copy)]
    struct How {
        fault: Fault,
        prefetch: bool,
        extra: Extra,
    }

    /// 一次完整 `verify_delta` 窗口的同次读数。`total` 是窗口内累计申请字节（含受管组件以
    /// Rust layout 分配的 SQLite 块；累计申请，不是 live、净增或泄漏）；`sqlite` 是同一窗口
    /// 原父 census 中逐块 SQLite 预留之和；`rust` 是二者之差，即其他 Rust 分配。`path` 是
    /// 窗口内台账语句，`requested` 是 source 收到的请求序列。
    #[cfg(feature = "s_incremental")]
    struct Measured {
        error: Option<LedgerError>,
        rust: u64,
        sqlite: u64,
        total: u64,
        path: SqlPath,
        requested: Vec<[u8; 32]>,
        overflow: bool,
    }
    #[cfg(feature = "s_incremental")]
    impl Measured {
        /// 收费验收：其他 Rust 分配为零，且总申请恰为原父逐块先收费的 SQLite 预留之和。
        fn accounted(&self) -> bool {
            self.rust == 0 && self.total == self.sqlite
        }
        /// 路径验收：窗口内恰好这些台账语句，source 恰好收到这些请求。
        fn followed(&self, statements: &[LedgerSql], reads: &[[u8; 32]]) -> bool {
            self.path.is(statements) && !self.overflow && self.requested == reads
        }
    }
    #[cfg(feature = "s_incremental")]
    impl std::fmt::Display for Measured {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "error={:?} rust={} sqlite={} total={} path={:?} requested={} overflow={}",
                self.error,
                self.rust,
                self.sqlite,
                self.total,
                self.path,
                self.requested.len(),
                self.overflow
            )
        }
    }

    /// 同 [`verify_in`]，可预读根节点；返回同次窗口读数（见 [`Measured`]）。
    #[cfg(feature = "s_incremental")]
    fn verify_measured(
        budget: &super::super::audit_owner::native::Budget,
        l: &Ledger<Charged>,
        nodes: &mut newchan_rust::s_content::MemoryNodes,
        root: &ContentRef,
        q: &Qualification,
        fault: Fault,
        prefetch: bool,
    ) -> Measured {
        let how = How {
            fault,
            prefetch,
            extra: Extra::None,
        };
        verify_measured_with(budget, l, nodes, root, q, how)
    }

    /// 对整次 `verify_delta` 开同一个窗口（`rust_window`），同时记下台账语句路径与 source
    /// 请求。请求容量与路径记录都在窗口外固定预留，窗口内记录不分配。
    #[cfg(feature = "s_incremental")]
    fn verify_measured_with(
        budget: &super::super::audit_owner::native::Budget,
        l: &Ledger<Charged>,
        nodes: &mut newchan_rust::s_content::MemoryNodes,
        root: &ContentRef,
        q: &Qualification,
        how: How,
    ) -> Measured {
        let prepared: Vec<_> = how
            .prefetch
            .then(|| {
                let digest = newchan_rust::s_content::parse_root(&root.root).unwrap();
                let found =
                    newchan_rust::s_content::NodeSource::read(&mut *nodes, &digest).unwrap();
                (digest, found)
            })
            .into_iter()
            .collect();
        let mut seen = None;
        let work = budget.codec_work().ok().expect("codec work");
        let _ = work.run(|scope| {
            let mut source = Scoped {
                nodes: &mut *nodes,
                scope,
                fault: how.fault,
                prepared,
                requested: Vec::with_capacity(PATH_CAPACITY),
                mismatched: false,
                overflow: false,
            };
            SqlPath::reset();
            let (error, probe) = rust_window(budget, || {
                let error = l.verify_delta(&mut source, [root.into(); 4], q).err();
                match how.extra {
                    Extra::None => {}
                    Extra::RustTemp => {
                        let _ = std::hint::black_box(Box::new(0u64));
                    }
                    Extra::LedgerSql => {
                        let _ = std::hint::black_box(l.through());
                    }
                }
                error
            });
            let path = SqlPath::take();
            seen = Some(Measured {
                error,
                rust: probe.rust,
                sqlite: probe.sqlite,
                total: probe.probe["allocated_bytes"].as_u64().unwrap(),
                path,
                requested: std::mem::take(&mut source.requested),
                overflow: source.overflow,
            });
            Ok::<_, ()>(())
        });
        seen.expect("verify ran")
    }

    /// 对完整 `verify_delta` 同时开 Rust 分配窗口与原父 census：返回本次错误、窗口内 Rust
    /// 分配字节、census 中逐次精确预留（layout 与 permit，含 SQLite 块）的字节和、以及 native
    /// 结构上界预留的字节和。`reads` 按读取顺序给出 source 要交出的节点。
    #[cfg(feature = "s_incremental")]
    fn verify_censused(
        budget: &super::super::audit_owner::native::Budget,
        l: &Ledger<Charged>,
        root: &ContentRef,
        q: &Qualification,
        mut reads: Vec<([u8; 32], newchan_rust::s_content::NodeRead)>,
    ) -> Censused {
        use super::super::audit_owner::allocation_probe::Window;
        let expected_reads = reads.len();
        reads.reverse();
        let mut nodes = newchan_rust::s_content::MemoryNodes::new();
        let mut seen = None;
        let work = budget.codec_work().ok().expect("codec work");
        let _ = work.run(|scope| {
            let mut source = Scoped {
                nodes: &mut nodes,
                scope,
                fault: Fault::None,
                prepared: reads,
                requested: Vec::with_capacity(expected_reads + 4),
                mismatched: false,
                overflow: false,
            };
            budget.census_arm();
            let window = Window::start();
            let result = l.verify_delta(&mut source, [root.into(); 4], q).err();
            let probe = window.finish();
            let events = budget.census_take();
            let sum = |kinds: &[&str]| {
                events
                    .iter()
                    .filter(|e| e.reserve && kinds.contains(&e.kind))
                    .map(|e| e.bytes as u64)
                    .sum::<u64>()
            };
            seen = Some(Censused {
                error: result,
                allocated: probe["allocated_bytes"].as_u64().unwrap(),
                reserved: sum(&["layout", "permit", "sqlite"]),
                native: sum(&["native"]),
                requested: std::mem::take(&mut source.requested),
                unread: source.prepared.len(),
                mismatched: source.mismatched,
            });
            Ok::<_, ()>(())
        });
        seen.expect("verify ran")
    }

    /// [`verify_censused`] 的结果：本次错误、窗口内 Rust 分配、精确预留、native 上界预留，以及
    /// source 实际收到的请求摘要序列、未交出的预备节点数、是否有请求与预备节点摘要不符。
    #[cfg(feature = "s_incremental")]
    struct Censused {
        error: Option<LedgerError>,
        allocated: u64,
        reserved: u64,
        native: u64,
        requested: Vec<[u8; 32]>,
        unread: usize,
        mismatched: bool,
    }

    #[cfg(feature = "s_incremental")]
    fn arm_trip(l: &Ledger<Charged>, mode: usize) {
        use newchan_rust::s_content::capture_allocation::sqlite_parent::Callback;
        use std::sync::atomic::Ordering;
        TRIP_MODE.store(mode, Ordering::SeqCst);
        TRIP_ARMED.store(false, Ordering::SeqCst);
        TRIPPED.store(false, Ordering::SeqCst);
        charged(l)
            .run(|q| unsafe {
                q.set_authorizer(Some(Callback {
                    callback: trip_refusal,
                    context: std::ptr::null_mut(),
                }))
            })
            .unwrap();
    }

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

    /// 台账语句路径（主控 ledger 零分配订正的路径锁）：SQLITE_TRACE_STMT 按原语句文本归类。
    /// 记录在窗口外固定容量的静态原子里（16 条 × 4 位），回调只做字节比较与原子写，不分配；
    /// 超出容量的条数仍计入长度，路径锁因此按失败处理。
    #[cfg(feature = "s_incremental")]
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum LedgerSql {
        Through = 1,
        Qualification = 2,
        Exists = 3,
        Other = 15,
    }
    #[cfg(feature = "s_incremental")]
    const PATH_CAPACITY: usize = 16;
    #[cfg(feature = "s_incremental")]
    static PATH_CODES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    #[cfg(feature = "s_incremental")]
    static PATH_LEN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    #[cfg(feature = "s_incremental")]
    unsafe extern "C" fn record_ledger_statement(
        _: std::ffi::c_uint,
        _: *mut std::ffi::c_void,
        _: *mut std::ffi::c_void,
        sql: *mut std::ffi::c_void,
    ) -> std::ffi::c_int {
        use self::m8d_success::{M8D_LEDGER_EXISTS, M8D_LEDGER_QUALIFICATION, M8D_LEDGER_THROUGH};
        use std::sync::atomic::Ordering;
        let text: &[u8] = if sql.is_null() {
            &[]
        } else {
            // SAFETY: for SQLITE_TRACE_STMT SQLite passes the NUL-terminated statement text.
            unsafe { std::ffi::CStr::from_ptr(sql as *const std::ffi::c_char) }.to_bytes()
        };
        let code = if text == M8D_LEDGER_THROUGH.as_bytes() {
            LedgerSql::Through
        } else if text == M8D_LEDGER_QUALIFICATION.as_bytes() {
            LedgerSql::Qualification
        } else if text == M8D_LEDGER_EXISTS.as_bytes() {
            LedgerSql::Exists
        } else {
            LedgerSql::Other
        };
        let at = PATH_LEN.fetch_add(1, Ordering::SeqCst);
        if at < PATH_CAPACITY {
            PATH_CODES.fetch_or((code as u64) << (4 * at), Ordering::SeqCst);
        }
        0
    }

    /// 一个窗口内记录的台账语句序列；`len` 含超出容量的条数。
    #[cfg(feature = "s_incremental")]
    #[derive(Clone, Copy, Debug)]
    struct SqlPath {
        len: usize,
        codes: u64,
    }
    #[cfg(feature = "s_incremental")]
    impl SqlPath {
        fn reset() {
            PATH_CODES.store(0, std::sync::atomic::Ordering::SeqCst);
            PATH_LEN.store(0, std::sync::atomic::Ordering::SeqCst);
        }
        fn take() -> Self {
            Self {
                len: PATH_LEN.swap(0, std::sync::atomic::Ordering::SeqCst),
                codes: PATH_CODES.swap(0, std::sync::atomic::Ordering::SeqCst),
            }
        }
        fn is(&self, expected: &[LedgerSql]) -> bool {
            self.len == expected.len()
                && expected
                    .iter()
                    .enumerate()
                    .all(|(i, s)| (self.codes >> (4 * i)) & 0xF == *s as u64)
        }
    }

    /// 在台账连接上装或卸语句路径记录；在窗口外调用（`sqlite3_trace_v2` 本身不分配）。
    #[cfg(feature = "s_incremental")]
    fn trace_path(l: &Ledger<Charged>, on: bool) {
        use rusqlite::ffi;
        charged(l)
            .run(|q| {
                // SAFETY: the handle stays inside this operation; the callback keeps no context.
                let rc = unsafe {
                    if on {
                        ffi::sqlite3_trace_v2(
                            q.handle(),
                            ffi::SQLITE_TRACE_STMT,
                            Some(record_ledger_statement),
                            std::ptr::null_mut(),
                        )
                    } else {
                        ffi::sqlite3_trace_v2(q.handle(), 0, None, std::ptr::null_mut())
                    }
                };
                assert_eq!(rc, ffi::SQLITE_OK);
                Ok(())
            })
            .unwrap();
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    #[ignore = "fresh process only: installs the global SQLite allocator; run alone with --ignored --exact"]
    fn charged_ledger_same_parent_phase_machine_and_snapshot_interleave() {
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
        let parent = |b: &Budget, binding: &Rc<SqliteBinding>| {
            ChargedParent::new(b.clone(), binding.clone()).expect("same parent and origin")
        };
        let dir = std::env::temp_dir().join(format!("charged-ledger-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c_path = |name: &str| std::ffi::CString::new(dir.join(name).to_str().unwrap()).unwrap();
        std::fs::File::create(dir.join("content.sqlite3")).unwrap();
        let content_real = std::fs::canonicalize(dir.join("content.sqlite3")).unwrap();
        let content = std::ffi::CString::new(content_real.to_str().unwrap()).unwrap();
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
        let mut l1 =
            Ledger::fresh_charged(&path1, &content, &identity(), STORE, parent(&current, &a))
                .unwrap();
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
        assert!(
            Ledger::fresh_charged(&path1, &content, &identity(), STORE, parent(&current, &a))
                .is_err()
        );
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
        let (committed, probe) = rust_window(&current, || l1.content_committed());
        assert_eq!(
            probe.probe["allocated_bytes"].as_u64().unwrap(),
            0,
            "{probe}"
        );
        committed.unwrap();
        assert_eq!(count(&a, &peek, through), 1);
        assert_eq!(l1.committed_through().unwrap(), 1);
        for unit in &d1.new_units {
            assert!(l1.contains_committed_unit(unit).unwrap());
        }
        assert_eq!(l1.qualification_reserved().unwrap().through, 1);
        let oracle = legacy_texts(
            &sha256_hex(FORMAT.as_bytes()),
            &d1,
            d1.new_units.iter(),
            &d1.verifier_revision,
        )
        .unwrap();
        let written = a
            .run(&peek, |q| {
                q.query_row(
                    "SELECT c.roots, c.new_units_digest, c.fold_digest, m.fold_digest \
                     FROM cert c, meta m WHERE c.generation=1 AND m.singleton=1",
                    [],
                    |[roots, units, fold, meta]| {
                        [roots, units, fold, meta].map(|v| v.as_str().unwrap().to_owned())
                    },
                )
            })
            .unwrap();
        assert_eq!(
            written,
            [
                oracle.roots.clone(),
                sha256_hex(oracle.units.as_bytes()),
                sha256_hex(oracle.fold.as_bytes()),
                sha256_hex(oracle.fold.as_bytes()),
            ]
        );
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
        assert!(matches!(
            ChargedParent::new(current.clone(), a.clone()),
            Err(LedgerError::Gate(SqlGate::WrongParent))
        ));
        let other = Budget::fresh(LIMIT)
            .unwrap()
            .bind_frozen(1, LIMIT, 0, 0)
            .unwrap();
        assert!(matches!(
            ChargedParent::new(other, b.clone()),
            Err(LedgerError::Gate(SqlGate::WrongParent))
        ));
        let pb = parent(&current, &b);
        let room = current.remaining();
        assert!(matches!(pb.reserve(room + 1), Err(LedgerError::Budget)));
        drop(pb.reserve(room).unwrap());
        assert!(matches!(
            ChargedPath::canonical(c"relative/content.sqlite3", &pb),
            Err(LedgerError::Invalid(_))
        ));
        let dotted = std::ffi::CString::new(format!(
            "{}/./content.sqlite3",
            content_real.parent().unwrap().display()
        ))
        .unwrap();
        assert!(matches!(
            ChargedPath::canonical(&dotted, &pb),
            Err(LedgerError::Invalid(_))
        ));
        let linked_dir = dir.join("linked");
        std::os::unix::fs::symlink(content_real.parent().unwrap(), &linked_dir).unwrap();
        let through_link =
            std::ffi::CString::new(linked_dir.join("content.sqlite3").to_str().unwrap()).unwrap();
        assert_eq!(
            ChargedPath::canonical(&through_link, &pb).err(),
            Some(LedgerError::Invalid("内容库路径含符号链接"))
        );
        let hard = content_real.with_file_name("content-hard.sqlite3");
        std::fs::hard_link(&content_real, &hard).unwrap();
        let hard = std::ffi::CString::new(hard.to_str().unwrap()).unwrap();
        let by_hard = ChargedPath::canonical(&hard, &pb).unwrap();
        let by_content = ChargedPath::canonical(&content, &pb).unwrap();
        assert_ne!(
            by_hard.files().collect::<Vec<_>>(),
            by_content.files().collect::<Vec<_>>()
        );
        drop((by_hard, by_content, pb));
        let path2 = c_path("ledger-b.sqlite3");
        let mut l2 =
            Ledger::fresh_charged(&path2, &content, &identity(), STORE, parent(&current, &b))
                .unwrap();
        l2.fold_step(token, &d1, &m1, &r1).unwrap();

        // 完整 fresh：Rust 分配恰为预留（两套三份路径、规范检查工作副本、规则），父账读数只多出
        // 仍持有的许可与该连接的 SQLite 块；stage 出 Pending 后丢弃台账，父账回到 fresh 之前。
        let id = identity();
        let fresh_path = c_path("ledger-f.sqlite3");
        let paths = |len: usize| 3 * len + 3 + "-wal".len() + "-journal".len();
        let held = paths(fresh_path.as_bytes().len())
            + paths(content.as_bytes().len())
            + id.rule_revision.len();
        let fresh_parent = parent(&current, &b);
        let (live0, sqlite0) = (current.live(), b.live_bytes());
        let (lf, probe) = rust_window(&current, || {
            Ledger::fresh_charged(&fresh_path, &content, &id, STORE, fresh_parent)
        });
        let mut lf = lf.unwrap();
        assert_eq!(
            probe.rust,
            (held + content.as_bytes().len() + 1) as u64,
            "{probe}"
        );
        let live_delta = |now: usize, then: usize| now as i128 - then as i128;
        assert_eq!(
            live_delta(current.live(), live0),
            held as i128 + live_delta(b.live_bytes(), sqlite0)
        );
        lf.stage(token, &d1).unwrap();
        drop(lf);
        assert_eq!((current.live(), b.live_bytes()), (live0, sqlite0));

        // stage 的非法根：在同一判据下按静态值拒绝，整次调用不生成诊断文本，台账失效。
        let bad_path = c_path("ledger-g.sqlite3");
        let mut lg =
            Ledger::fresh_charged(&bad_path, &content, &id, STORE, parent(&current, &b)).unwrap();
        let mut bad = delta(&m1, &r1);
        bad.roots[1].format = "not-a-format".repeat(64);
        let (staged, probe) = rust_window(&current, || lg.stage(token, &bad));
        assert_eq!(
            probe.probe["allocated_bytes"].as_u64().unwrap(),
            0,
            "{probe}"
        );
        assert_eq!(staged, Err(LedgerError::Invalid("台账根格式无效")));
        assert!(lg.connection.is_none());
        drop((lg, bad));
        let path3 = c_path("ledger-c.sqlite3");
        let mut l3 =
            Ledger::fresh_charged(&path3, &content, &identity(), STORE, parent(&current, &b))
                .unwrap();
        l3.fold_step(token, &d1, &m1, &r1).unwrap();
        let q3 = l3.qualification_reserved().unwrap();
        let (m2, r2) = generation(2);
        let d2 = delta(&m2, &r2);
        let reserved = d2.raw.len() * std::mem::size_of::<(i64, &RawFact)>()
            + d2.cover.len() * std::mem::size_of::<(i64, &CoverChange)>()
            + canonical::json_len(&canonical::Roots(&d2.roots)).unwrap()
            + 64
            + token.len()
            + d2.roots
                .iter()
                .map(|r| r.format.len() + r.root.len())
                .sum::<usize>();
        let pending = token.len()
            + d2.roots
                .iter()
                .map(|r| r.format.len() + r.root.len())
                .sum::<usize>();
        let (live0, sqlite0) = (current.live(), b.live_bytes());
        let (staged, probe) = rust_window(&current, || l2.stage(token, &d2));
        staged.unwrap();
        assert_eq!(probe.rust, reserved as u64, "{probe}");
        assert_eq!(
            live_delta(current.live(), live0),
            pending as i128 + live_delta(b.live_bytes(), sqlite0)
        );
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
        let spare_parent = parent(&current, &b);
        let (spare, probe) =
            rust_window(&current, || Charged::open(&spare_path, pages, spare_parent));
        let spare = spare.unwrap();
        assert_eq!(probe.rust, 0, "{probe}");

        let first = b.run(&snap, |_| {
            assert!(unsafe { ffi::sqlite3_malloc64(ROOM as u64 + 1) }.is_null());
            Ok(())
        });
        assert_eq!(first, Err(SqlGate::Refused(ScratchFailure::Budget)));
        let budget = LedgerError::Gate(SqlGate::Refused(ScratchFailure::Budget));
        let refused_path = c_path("ledger-h.sqlite3");
        let refused_parent = ChargedParent {
            budget: current.clone(),
            binding: b.clone(),
        };
        let live = current.live();
        let (refused_fresh, probe) = rust_window(&current, || {
            Ledger::fresh_charged(&refused_path, &content, &id, STORE, refused_parent)
        });
        assert_eq!(
            probe.probe["allocated_bytes"].as_u64().unwrap(),
            0,
            "{probe}"
        );
        assert_eq!(
            refused_fresh.err(),
            Some(LedgerError::Gate(SqlGate::Refused(ScratchFailure::Budget)))
        );
        assert!(!dir.join("ledger-h.sqlite3").exists());
        assert_eq!(current.live(), live);
        // 首拒之后：台账读、受管开库都返回本次 typed 分类，Rust 侧零分配，不进入 SQLite。
        let entered = LEDGER_STATEMENTS.load(Ordering::SeqCst);
        let (refused, probe) = rust_window(&current, || l2.through());
        assert_eq!(
            probe.probe["allocated_bytes"].as_u64().unwrap(),
            0,
            "{probe}"
        );
        let refused = refused.unwrap_err();
        assert_eq!(refused, budget);
        assert_eq!(LEDGER_STATEMENTS.load(Ordering::SeqCst), entered);
        // 首拒之后 `ChargedParent::new` 自身按当前准入拒绝（核身份后再核准入），零分配、不进入
        // SQLite；受管开库自己的拒绝则用同父直接构造的参数核，与上面的 `refused_parent` 同法。
        let (constructed, probe) = rust_window(&current, || {
            ChargedParent::new(current.clone(), b.clone()).err()
        });
        assert_eq!(
            probe.probe["allocated_bytes"].as_u64().unwrap(),
            0,
            "{probe}"
        );
        assert_eq!(constructed.as_ref(), Some(&budget));
        assert_eq!(LEDGER_STATEMENTS.load(Ordering::SeqCst), entered);
        let spare_parent = ChargedParent {
            budget: current.clone(),
            binding: b.clone(),
        };
        let (reopened, probe) =
            rust_window(&current, || Charged::open(&spare_path, pages, spare_parent));
        assert_eq!(
            probe.probe["allocated_bytes"].as_u64().unwrap(),
            0,
            "{probe}"
        );
        assert_eq!(reopened.err().as_ref(), Some(&budget));
        let unit = newchan_rust::s_content::closure_delta::Unit {
            digest: [1; 32],
            context: newchan_rust::s_content::closure_delta::Context::Value,
            mode: newchan_rust::s_content::closure_delta::VerificationMode::Public,
        };
        let view = QualifiedSuccessView::new(&l3, &q3);
        let (lookup, probe) = rust_window(&current, || {
            newchan_rust::s_content::closure_delta::Frontier::contains(&view, unit)
        });
        assert_eq!(
            probe.probe["allocated_bytes"].as_u64().unwrap(),
            0,
            "{probe}"
        );
        assert!(lookup.is_err());
        let carried = view.refused.into_inner().expect("typed refusal carried");
        assert_eq!(carried, budget);
        assert!(matches!(
            carried.into_failure(),
            super::super::audit_owner::native::Failure::BudgetExceeded
        ));
        drop((l3, q3));
        let (committed, probe) = rust_window(&current, || l2.content_committed());
        assert_eq!(
            probe.probe["allocated_bytes"].as_u64().unwrap(),
            0,
            "{probe}"
        );
        let committed = committed.unwrap_err();
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
        use newchan_rust::s_content as sc;
        let content_root = |nodes: &mut sc::MemoryNodes, tag: &str| {
            sc::write_value(
                nodes,
                &sc::Value::Object(vec![(tag.into(), sc::Value::Null)]),
            )
            .unwrap()
            .content_ref()
        };
        // verify_delta：初始资格核验通过，之后的前沿点查遇本域首拒，按本次 typed 值返回；
        // 只读失败不使台账失效。
        let path4 = c_path("ledger-d.sqlite3");
        let mut l4 =
            Ledger::fresh_charged(&path4, &content, &identity(), STORE, parent(&current, &c))
                .unwrap();
        l4.fold_step(token, &d1, &m1, &r1).unwrap();
        let q4 = l4.qualification_reserved().unwrap();
        let mut nodes = sc::MemoryNodes::new();
        let root = content_root(&mut nodes, "verify");
        let wrong = Some(LedgerError::Gate(SqlGate::WrongParent));
        assert_eq!(
            l4.verify_delta(&mut nodes, [(&root).into(); 4], &q4).err(),
            wrong
        );
        let foreign = Budget::fresh(LIMIT)
            .unwrap()
            .bind_frozen(1, LIMIT, 0, 0)
            .unwrap();
        assert_eq!(
            verify_in(&foreign, &l4, &mut nodes, &root, &q4, Fault::None),
            wrong
        );
        assert_eq!(
            verify_in(&current, &l4, &mut nodes, &root, &q4, Fault::None),
            None
        );
        // 资格有效后坏 format/root 失败（主控 ledger 零分配订正：撤回 fb0d 对本类整次累计申请
        // 为零的承诺，原 4359 红锁不登记为已满足）。同一完整窗口内：共用 verifier 的失败链在
        // 静默诊断期不生成文本，除原父逐块先收费的 SQLite 块外零 Rust 分配，总申请恰为这些
        // SQLite 预留之和；路径恰为合同规定的两条资格 SQL（认证水位、cert×meta 资格行），坏根在
        // 读节点之前失败，source 不收到请求。kind/extent 与真实节点不符时，两者只剩同一份合法
        // 工作集分配（根节点已预读，source 不分配），与诊断内容无关。
        let qualification = [LedgerSql::Through, LedgerSql::Qualification];
        let invalid = Some(LedgerError::Invalid("Δ闭包核验失败"));
        let bads = [
            ContentRef {
                format: "x\"y".repeat(4096),
                ..root.clone()
            },
            ContentRef {
                root: format!("sc1-{}", "Z".repeat(64)),
                ..root.clone()
            },
            ContentRef {
                root: "no-prefix".repeat(512),
                ..root.clone()
            },
        ];
        trace_path(&l4, true);
        for bad in &bads {
            let measured = verify_measured(&current, &l4, &mut nodes, bad, &q4, Fault::None, false);
            assert_eq!(measured.error, invalid, "{measured}");
            assert!(measured.accounted(), "{measured}");
            assert!(measured.followed(&qualification, &[]), "{measured}");
        }
        // 两项隔离负控（同一坏根、同一窗口）：追加一次无账 Rust 临时申请，只有收费验收失败；
        // 追加一条台账 SQL，只有路径验收失败，其 SQLite 块照常原父收费。
        let control = |extra| How {
            fault: Fault::None,
            prefetch: false,
            extra,
        };
        let rust_temp = verify_measured_with(
            &current,
            &l4,
            &mut nodes,
            &bads[0],
            &q4,
            control(Extra::RustTemp),
        );
        assert_eq!(rust_temp.error, invalid, "{rust_temp}");
        assert!(!rust_temp.accounted(), "{rust_temp}");
        assert_eq!(
            rust_temp.rust,
            std::mem::size_of::<u64>() as u64,
            "{rust_temp}"
        );
        assert!(rust_temp.followed(&qualification, &[]), "{rust_temp}");
        let extra_sql = verify_measured_with(
            &current,
            &l4,
            &mut nodes,
            &bads[0],
            &q4,
            control(Extra::LedgerSql),
        );
        assert_eq!(extra_sql.error, invalid, "{extra_sql}");
        assert!(extra_sql.accounted(), "{extra_sql}");
        assert!(!extra_sql.followed(&qualification, &[]), "{extra_sql}");
        assert!(
            extra_sql.followed(
                &[
                    LedgerSql::Through,
                    LedgerSql::Qualification,
                    LedgerSql::Through
                ],
                &[]
            ),
            "{extra_sql}"
        );
        trace_path(&l4, false);
        let mut wrong_kind = root.clone();
        wrong_kind.kind = newchan_rust::s_content::Kind::Array;
        let mut wrong_extent = root.clone();
        wrong_extent.extent = Some(u64::MAX);
        let kind = verify_measured(
            &current,
            &l4,
            &mut nodes,
            &wrong_kind,
            &q4,
            Fault::None,
            true,
        );
        let extent = verify_measured(
            &current,
            &l4,
            &mut nodes,
            &wrong_extent,
            &q4,
            Fault::None,
            true,
        );
        assert_eq!(kind.error, invalid);
        assert_eq!(extent.error, invalid);
        assert_eq!(kind.rust, extent.rust);
        // 两条经包装 helper 的完整入口反例：声明长度超界的 STRING（动态诊断），以及指向 body
        // 只有 31 字节的真实 SEQ_LEAF 的合法 STRING（静态字面量诊断）。整次 verify_delta 的
        // Rust 分配必须恰好等于原父 census 中逐次精确预留之和，诊断没有无账分配。
        {
            use newchan_rust::s_content::{
                frame, root_string, tag, Kind, NodeRead, CONTENT_FORMAT,
            };
            let reference = |digest: &[u8; 32], extent: Option<u64>| ContentRef {
                format: CONTENT_FORMAT.into(),
                root: root_string(digest),
                kind: Kind::String,
                extent,
            };
            let mut overlong = u64::MAX.to_be_bytes().to_vec();
            overlong.extend_from_slice(&[7; 32]);
            let overlong_digest = frame(tag::STRING, &overlong);
            let overlong_root = reference(&overlong_digest, Some(0));
            let short_leaf = vec![9u8; 31];
            let leaf_digest = frame(tag::SEQ_LEAF, &short_leaf);
            let mut string_body = 5u64.to_be_bytes().to_vec();
            string_body.extend_from_slice(&leaf_digest);
            let string_digest = frame(tag::STRING, &string_body);
            let leaf_root = reference(&string_digest, Some(5));
            let string_node = || NodeRead::Found {
                tag: tag::STRING,
                body: string_body.clone(),
            };
            let cases = [
                (
                    overlong_root,
                    vec![(
                        overlong_digest,
                        NodeRead::Found {
                            tag: tag::STRING,
                            body: overlong,
                        },
                    )],
                    vec![overlong_digest],
                ),
                (
                    leaf_root,
                    vec![
                        (string_digest, string_node()),
                        (string_digest, string_node()),
                        (
                            leaf_digest,
                            NodeRead::Found {
                                tag: tag::SEQ_LEAF,
                                body: short_leaf,
                            },
                        ),
                    ],
                    vec![string_digest, string_digest, leaf_digest],
                ),
            ];
            for (root, reads, path) in cases {
                let run = verify_censused(&current, &l4, &root, &q4, reads);
                assert_eq!(run.error, Some(LedgerError::Invalid("Δ闭包核验失败")));
                assert_eq!(run.requested, path, "the failing read is the expected node");
                assert_eq!(run.unread, 0);
                assert!(!run.mismatched);
                assert_eq!(
                    run.allocated, run.reserved,
                    "native upper bounds {}",
                    run.native
                );
            }
        }
        arm_trip(&l4, 1);
        let verified = verify_in(&current, &l4, &mut nodes, &root, &q4, Fault::None);
        assert!(TRIPPED.load(Ordering::SeqCst));
        assert_eq!(
            verified,
            Some(LedgerError::Gate(SqlGate::Refused(ScratchFailure::Budget)))
        );
        assert!(l4.connection.is_some());
        assert_eq!(b.refusal(), None);
        assert_eq!(kept, budget);
        drop((l4, q4));
        drop(spare);
        c.close(peek).map_err(|(_, e)| e).unwrap();

        // stage_verified：BEGIN 之后的上一代资格核验遇本域首拒，typed 值返回且台账失效。
        current = current.handoff(4, ROOM).expect("handoff to D");
        let dom = bind(&current);
        let path5 = c_path("ledger-e.sqlite3");
        let mut l5 =
            Ledger::fresh_charged(&path5, &content, &identity(), STORE, parent(&current, &dom))
                .unwrap();
        l5.fold_step(token, &d1, &m1, &r1).unwrap();
        let q5 = l5.qualification_reserved().unwrap();
        let mut nodes5 = sc::MemoryNodes::new();
        let root5 = content_root(&mut nodes5, "stage");
        let mut m5 = m1.clone();
        m5.generation = 2;
        m5.scope = root5.clone();
        let r5 = content_manifest::ManifestRoots {
            generated_state: root5.clone(),
            publication_template: root5.clone(),
            delta_template: root5.clone(),
        };
        let d5 = delta(&m5, &r5);
        let units = QualifiedUnits::full(
            sc::closure_delta::verify_four_roots(&mut nodes5, [(&root5).into(); 4]).unwrap(),
        )
        .unwrap();
        let mut candidates = 0usize;
        units
            .verified
            .try_for_each_unit(|_| -> Result<(), ()> {
                candidates += 1;
                Ok(())
            })
            .unwrap();
        arm_trip(&l5, 2);
        let (staged, probe) = rust_window(&current, || {
            l5.stage_verified(token, &d5, &units, None, Some(&*q5))
        });
        assert_eq!(
            probe.rust,
            (candidates * std::mem::size_of::<Unit>()) as u64,
            "{probe}"
        );
        assert!(TRIPPED.load(Ordering::SeqCst));
        let staged = staged.unwrap_err();
        assert_eq!(staged, budget);
        assert!(matches!(
            staged.into_failure(),
            super::super::audit_owner::native::Failure::BudgetExceeded
        ));
        assert!(l5.connection.is_none());
        drop((l5, q5));

        // E：source 读取在中途按 codec 工作域额度失败，typed Budget 不被静态诊断抹掉；之后同父
        // 同 origin 重新绑定，旧 epoch 的完整 fresh 在任何分配与建文件之前被拒。
        current = current.handoff(5, ROOM).expect("handoff to E");
        let fifth = bind(&current);
        let path6 = c_path("ledger-i.sqlite3");
        let mut l6 =
            Ledger::fresh_charged(&path6, &content, &id, STORE, parent(&current, &fifth)).unwrap();
        l6.fold_step(token, &d1, &m1, &r1).unwrap();
        let q6 = l6.qualification_reserved().unwrap();
        let mut nodes6 = sc::MemoryNodes::new();
        let root6 = content_root(&mut nodes6, "starve");
        let starved =
            verify_in(&current, &l6, &mut nodes6, &root6, &q6, Fault::Budget).expect("refused");
        assert_eq!(starved, LedgerError::Budget);
        assert!(matches!(
            starved.into_failure(),
            super::super::audit_owner::native::Failure::BudgetExceeded
        ));
        assert!(l6.connection.is_some());
        // 资格有效后 source 读取被拒（同一订正）：同一完整窗口内其他 Rust 为零、总申请恰为原父
        // SQLite 预留之和；路径恰为两条资格 SQL，随后 source 只收到根节点一次请求即被拒。
        let root6_digest = newchan_rust::s_content::parse_root(&root6.root).unwrap();
        trace_path(&l6, true);
        let measured = verify_measured(
            &current,
            &l6,
            &mut nodes6,
            &root6,
            &q6,
            Fault::Allocation,
            false,
        );
        trace_path(&l6, false);
        assert!(measured.accounted(), "{measured}");
        assert!(
            measured.followed(
                &[LedgerSql::Through, LedgerSql::Qualification],
                &[root6_digest]
            ),
            "{measured}"
        );
        let allocation = measured.error.expect("refused");
        assert_eq!(allocation, LedgerError::Scratch(ScratchFailure::Allocation));
        assert!(matches!(
            allocation.into_failure(),
            super::super::audit_owner::native::Failure::BudgetExceeded
        ));
        drop((l6, q6));
        let stale = parent(&current, &fifth);
        let rebound = bind(&current);
        let stale_path = c_path("ledger-j.sqlite3");
        let live = current.live();
        let (stale_fresh, probe) = rust_window(&current, || {
            Ledger::fresh_charged(&stale_path, &content, &id, STORE, stale)
        });
        assert_eq!(
            probe.probe["allocated_bytes"].as_u64().unwrap(),
            0,
            "{probe}"
        );
        assert_eq!(
            stale_fresh.err(),
            Some(LedgerError::Gate(SqlGate::Refused(ScratchFailure::Ledger)))
        );
        assert!(!dir.join("ledger-j.sqlite3").exists());
        assert_eq!(current.live(), live);
        rebound.close(snap).map_err(|(_, e)| e).unwrap();
        drop(dom);
        assert_eq!(unsafe { ffi::sqlite3_shutdown() }, ffi::SQLITE_OK);
        assert_eq!(rebound.live_bytes(), 0);
        assert_eq!(current.live(), base);
        assert!(!allocator.is_broken());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn backend_is_sealed_to_two_implementations() {
        let source = include_str!("audit_ledger.rs");
        assert!(source.contains(concat!("\nmod ", "sealed {")));
        assert!(!source.contains(concat!("pub ", "mod sealed")));
        assert!(!source.contains(concat!("pub(super) ", "mod sealed")));
        assert!(!source.contains(concat!("pub(crate) ", "mod sealed")));
        assert_eq!(
            source
                .matches(concat!("impl sealed", "::Sealed for "))
                .count(),
            2
        );
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn charged_boundary_errors_keep_their_class_without_allocating() {
        use super::super::audit_owner::allocation_probe::Window;
        use newchan_rust::s_content::capture_allocation::scratch::ScratchFailure;
        let budget = || LedgerError::Gate(SqlGate::Refused(ScratchFailure::Budget));
        let inputs = (budget(), budget(), budget());
        let window = Window::start();
        let prior = <Charged as Backend>::prior_mismatch(Some(inputs.0));
        let differs = <Charged as Backend>::prior_mismatch(None);
        let frontier = <Charged as Backend>::frontier_error(inputs.1);
        let stale = <Charged as Backend>::prior_mismatch(Some(LedgerError::Detached));
        let probe = window.finish();
        assert_eq!(probe["allocated_bytes"], 0, "{probe}");
        assert_eq!(prior, inputs.2);
        assert_eq!(frontier, Ok(budget()));
        assert_eq!(stale, LedgerError::Detached);
        assert_eq!(
            String::from(differs),
            <Legacy as Backend>::prior_mismatch(None)
        );
        assert_eq!(
            <Legacy as Backend>::prior_mismatch(Some("x".to_owned())),
            error("上一代资格与台账当前已提交状态不符，不续写折叠链：x")
        );
        assert_eq!(
            <Legacy as Backend>::frontier_error("t".to_owned()),
            Err("t".to_owned())
        );
        assert_eq!(
            String::from(LedgerError::Invalid("台账已丢弃")),
            error("台账已丢弃")
        );
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn storage_reading_does_not_allocate() {
        use super::super::audit_owner::allocation_probe::Window;
        let dir = std::env::temp_dir().join(format!("ledger-bytes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("db.sqlite3"), [0u8; 300]).unwrap();
        let c = |name: &str| std::ffi::CString::new(dir.join(name).to_str().unwrap()).unwrap();
        let (db, missing) = (c("db.sqlite3"), c("missing.sqlite3"));
        let window = Window::start();
        let present = stat_file(&db);
        let absent = stat_file(&missing);
        let probe = window.finish();
        assert_eq!(probe["allocated_bytes"], 0, "{probe}");
        assert_eq!(present, Ok(300));
        assert_eq!(absent, Ok(0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn canonical_case(g: i64, keys: &[i64], text: &str, extent: Option<u64>) -> GenerationDelta {
        use newchan_rust::s_content as sc;
        let mut nodes = sc::MemoryNodes::new();
        let value = sc::Value::Object(vec![(text.into(), sc::Value::Bool(g % 2 == 0))]);
        let mut r = sc::write_value(&mut nodes, &value).unwrap().content_ref();
        r.extent = extent;
        let raw = keys
            .iter()
            .map(|&k| {
                let fill = k.unsigned_abs() as u8;
                (
                    k,
                    RawFact {
                        event_digest: [fill; 32],
                        bar_digest: [fill.wrapping_add(1); 32],
                    },
                )
            })
            .collect();
        let cover = keys
            .iter()
            .enumerate()
            .map(|(i, &k)| {
                let change = match i % 3 {
                    0 => CoverChange::Group {
                        seq: k,
                        object_id: text.into(),
                    },
                    1 => CoverChange::Pending { seq: k },
                    _ => CoverChange::Removed,
                };
                (k, change)
            })
            .collect();
        let objects = if keys.is_empty() {
            BTreeMap::new()
        } else {
            BTreeMap::from([
                (
                    text.to_owned(),
                    ObjectFact {
                        record_digest: [3; 32],
                        first_generation: i64::MIN,
                        first_cut: text.into(),
                        published_generation: i64::MAX,
                        source_digest: [4; 32],
                        withdrawn: g % 2 == 1,
                    },
                ),
                (
                    "a".to_owned(),
                    ObjectFact {
                        record_digest: [5; 32],
                        first_generation: 1,
                        first_cut: String::new(),
                        published_generation: 2,
                        source_digest: [6; 32],
                        withdrawn: false,
                    },
                ),
            ])
        };
        let related = || {
            BTreeMap::from([(
                text.to_owned(),
                RelatedFact {
                    record_digest: [7; 32],
                    source_digest: [8; 32],
                },
            )])
        };
        GenerationDelta {
            generation: g,
            roots: [r.clone(), r.clone(), r.clone(), r],
            verifier_revision: text.into(),
            raw,
            objects,
            witnesses: related(),
            observations: if keys.is_empty() {
                BTreeMap::new()
            } else {
                related()
            },
            withdrawals: BTreeSet::from([text.to_owned(), "z".to_owned()]),
            cover,
            effective: g,
            new_units: BTreeSet::from([
                Unit {
                    digest: [1; 32],
                    flavor: Flavor::Value,
                    mode: Mode::Public,
                },
                Unit {
                    digest: [2; 32],
                    flavor: Flavor::JsonInner,
                    mode: Mode::Template,
                },
            ]),
        }
    }

    #[test]
    fn canonical_stream_matches_canonical_json_byte_for_byte() {
        let long = "长".repeat(3000);
        let texts = [
            "plain",
            "quote\" backslash\\ ctrl\u{1}\n\t",
            "中文与 emoji 😀🧪",
            long.as_str(),
            "",
        ];
        let key_sets: [&[i64]; 4] = [
            &[],
            &[10, 2, 100, 9],
            &[-1, -10, 0, 3, i64::MIN, i64::MAX],
            &[1],
        ];
        for (g, (text, keys)) in texts.iter().zip(key_sets.iter().cycle()).enumerate() {
            for extent in [None, Some(0), Some(u64::MAX)] {
                let d = canonical_case(g as i64 + 1, keys, text, extent);
                let previous = format!("prev-{text}");
                for units in [d.new_units.iter().take(0), d.new_units.iter().take(2)] {
                    let oracle = legacy_texts(&previous, &d, units.clone(), text).unwrap();
                    let mut raw: Vec<_> = d.raw.iter().map(|(k, v)| (*k, v)).collect();
                    canonical::sort_by_decimal(&mut raw);
                    let mut cover: Vec<_> = d.cover.iter().map(|(k, v)| (*k, v)).collect();
                    canonical::sort_by_decimal(&mut cover);
                    let fold = canonical::Fold {
                        previous: &previous,
                        delta: canonical::Delta {
                            d: &d,
                            verifier_revision: text,
                            units: units.clone(),
                            raw: &raw,
                            cover: &cover,
                        },
                    };
                    let mut fold_text = Vec::new();
                    canonical::write_into(&mut fold_text, &fold).unwrap();
                    assert_eq!(String::from_utf8(fold_text).unwrap(), oracle.fold);
                    let mut units_text = Vec::new();
                    canonical::write_into(&mut units_text, &canonical::Units(units.clone()))
                        .unwrap();
                    assert_eq!(String::from_utf8(units_text).unwrap(), oracle.units);
                    let mut roots_text = Vec::new();
                    canonical::write_into(&mut roots_text, &canonical::Roots(&d.roots)).unwrap();
                    assert_eq!(String::from_utf8(roots_text).unwrap(), oracle.roots);
                    assert_eq!(
                        canonical::json_len(&canonical::Roots(&d.roots)).unwrap(),
                        oracle.roots.len()
                    );
                    assert_eq!(
                        canonical::hash_hex(&fold).unwrap().as_str(),
                        sha256_hex(oracle.fold.as_bytes())
                    );
                    assert_eq!(
                        canonical::hash_hex(&canonical::Units(units))
                            .unwrap()
                            .as_str(),
                        sha256_hex(oracle.units.as_bytes())
                    );
                }
            }
        }
        assert_eq!(
            canonical::Hex::of_bytes(FORMAT.as_bytes()).as_str(),
            sha256_hex(FORMAT.as_bytes())
        );
        for n in [0, 9, 10, -1, i64::MIN, i64::MAX] {
            assert_eq!(canonical::Decimal::of_i64(n).as_str(), n.to_string());
        }
        assert_eq!(
            canonical::Decimal::of_u64(u64::MAX).as_str(),
            u64::MAX.to_string()
        );
    }

    #[test]
    fn legacy_storage_reading_keeps_metadata_semantics() {
        use std::os::unix::ffi::OsStrExt;
        let dir = std::env::temp_dir().join(format!("ledger-legacy-bytes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("db.sqlite3");
        std::fs::write(&file, [0u8; 300]).unwrap();
        std::fs::write(dir.join("db.sqlite3-journal"), [0u8; 7]).unwrap();
        let link = dir.join("link.sqlite3");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        let dangling = dir.join("dangling.sqlite3");
        std::os::unix::fs::symlink(dir.join("absent"), &dangling).unwrap();
        assert_eq!(database_bytes(&file), Ok(307));
        assert_eq!(database_bytes(&link), Ok(300));
        assert_eq!(database_bytes(&dangling), Ok(0));
        assert_eq!(database_bytes(&dir.join("missing.sqlite3")), Ok(0));
        let nul = Path::new(std::ffi::OsStr::from_bytes(b"has\0nul"));
        assert_eq!(
            database_bytes(nul),
            Err(error(std::fs::metadata(nul).unwrap_err()))
        );
        let long = dir.join("x".repeat(5000));
        assert_eq!(
            database_bytes(&long),
            Err(error(std::fs::metadata(&long).unwrap_err()))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "s_incremental")]
    #[test]
    fn charged_stat_leaves_long_paths_to_the_platform() {
        let long = std::ffi::CString::new(format!("/{}", "x".repeat(5000))).unwrap();
        let result = stat_file(&long);
        let expected = std::fs::metadata(std::path::Path::new(long.to_str().unwrap()))
            .map(|_| ())
            .unwrap_err()
            .kind();
        assert_eq!(result, Err(LedgerError::Storage(expected)));
    }
}
