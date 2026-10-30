//! #1647：旁挂审计事实存储与提交生命周期。这里不签发来源或闭包核验资格。
//! 调用者只能送入已核验的单代变化；#1481 的冷折叠与在线生产者共用 stage。
//! 本组件尚不接准入读取，未实现的来源/闭包 Δ 不得借此跳过现有核验。
#![allow(dead_code)]

use super::*;
use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::OpenOptionsExt;
use newchan_rust::s_content::{ContentRef, RefView};
use rusqlite::{params, OpenFlags};

const FORMAT: &str = "s-audit-ledger/1";
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
CREATE TABLE cover(seq INTEGER PRIMARY KEY REFERENCES raw(seq), group_id TEXT NOT NULL);
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub(super) enum Flavor { Value, JsonInner }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub(super) enum Mode { Public, Template }
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub(super) struct Unit {
    pub digest: [u8; 32],
    pub flavor: Flavor,
    pub mode: Mode,
}
impl Unit {
    fn flavor(&self) -> &'static str {
        match self.flavor { Flavor::Value => "value", Flavor::JsonInner => "json-inner" }
    }
    fn mode(&self) -> &'static str {
        match self.mode { Mode::Public => "public", Mode::Template => "template" }
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
    pub cover: BTreeMap<i64, String>,
    pub new_units: BTreeSet<Unit>,
}

pub(super) struct Identity {
    pub rule_revision: String,
    pub content_identity: String,
    pub binary_commit: String,
}

struct Pending {
    token: String,
    generation: i64,
    roots: [ContentRef; 4],
}
enum Phase { Ready, Pending(Pending), Bound(Pending), Stale }

pub(super) struct Ledger {
    connection: Option<Connection>,
    phase: Phase,
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
    Value::Array(roots.iter().map(|r|root_value(r.into())).collect())
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
        n = n.checked_add(file_bytes(Path::new(&p))?).ok_or_else(||error("存储计数溢出"))?;
    }
    Ok(n)
}

impl Ledger {
    /// 永不打开旧台账作可信输入。文件名必须是本次重建的新文件；不覆盖旧诊断原件。
    pub(super) fn fresh(path: &Path, content_path: &Path, identity: Identity,
        max_store_bytes: u64) -> Result<Self, String> {
        if identity.rule_revision.is_empty() || identity.content_identity.is_empty()
            || identity.binary_commit.is_empty() {
            return Err(error("台账身份字段不能为空"));
        }
        let content_path = content_path.canonicalize().map_err(error)?;
        let remaining = max_store_bytes.checked_sub(database_bytes(&content_path)?)
            .ok_or_else(||error("内容库已超过 max_store_bytes"))?;
        let pages = remaining / 4096;
        if pages == 0 { return Err(error("没有台账页空间")); }
        std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600)
            .open(path).map_err(error)?;
        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX).map_err(error)?;
        conn.busy_timeout(std::time::Duration::ZERO).map_err(error)?;
        conn.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;")
            .map_err(error)?;
        conn.pragma_update(None, "page_size", 4096).map_err(error)?;
        conn.pragma_update(None, "max_page_count", pages).map_err(error)?;
        conn.execute_batch(SCHEMA).map_err(error)?;
        conn.execute("INSERT INTO meta VALUES(1,?1,?2,?3,?4,0,?5)", params![FORMAT,
            identity.rule_revision, identity.content_identity, identity.binary_commit,
            sha256_hex(b"s-audit-ledger/1")]).map_err(error)?;
        let ledger = Self { connection: Some(conn), phase: Phase::Ready, path:path.to_owned(),
            content_path, max_store_bytes, rule_revision:identity.rule_revision };
        ledger.check_storage()?;
        Ok(ledger)
    }
    fn check_storage(&self) -> Result<(), String> {
        let size = database_bytes(&self.content_path)?.checked_add(database_bytes(&self.path)?)
            .ok_or_else(||error("存储合账溢出"))?;
        if size > self.max_store_bytes { return Err(error("内容库/WAL/台账合计超过 max_store_bytes")); }
        Ok(())
    }
    pub(super) fn invalidate(&mut self) {
        self.phase = Phase::Stale;
        // 丢弃连接也回滚未提交的整代；不重试，也不复原旧进程状态。
        drop(self.connection.take());
    }
    pub(super) fn through(&self) -> Result<i64, String> {
        self.connection.as_ref().ok_or_else(||error("台账已丢弃"))?
            .query_row("SELECT certified_through_generation FROM meta WHERE singleton=1", [], |r|r.get(0))
            .map_err(error)
    }
    pub(super) fn stage(&mut self, token: &str, delta: &GenerationDelta) -> Result<(), String> {
        let result = self.stage_inner(token, delta);
        if result.is_err() { self.invalidate(); }
        result
    }
    fn stage_inner(&mut self, token: &str, d: &GenerationDelta) -> Result<(), String> {
        if !matches!(self.phase, Phase::Ready) { return Err(error("台账不在可准备状态")); }
        if token.is_empty() || d.verifier_revision.is_empty() || self.through()?.checked_add(1) != Some(d.generation) {
            return Err(error("请求或逐代序号无效"));
        }
        for r in &d.roots { r.digest().map_err(error)?; }
        self.check_storage()?;
        let conn = self.connection.as_ref().unwrap();
        conn.execute_batch("BEGIN IMMEDIATE").map_err(error)?;
        let previous:String = conn.query_row("SELECT fold_digest FROM meta WHERE singleton=1", [], |r|r.get(0)).map_err(error)?;
        let mut next:i64 = conn.query_row("SELECT COALESCE(MAX(seq),-1)+1 FROM raw", [], |r|r.get(0)).map_err(error)?;
        for (seq, row) in &d.raw {
            if *seq != next { return Err(error("raw 只允许连续追加")); }
            conn.execute("INSERT INTO raw VALUES(?1,?2,?3,?4)", params![seq,
                row.event_digest.as_slice(),row.bar_digest.as_slice(),d.generation]).map_err(error)?;
            next = next.checked_add(1).ok_or_else(||error("raw 序号溢出"))?;
        }
        for (id,r) in &d.objects {
            if r.first_generation < 1 || r.first_generation > r.published_generation
                || r.published_generation > d.generation { return Err(error("对象代戳无效")); }
            conn.execute("INSERT INTO object VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
                ON CONFLICT(id) DO UPDATE SET record_digest=excluded.record_digest,
                first_generation=excluded.first_generation, first_cut=excluded.first_cut,
                published_generation=excluded.published_generation, source_digest=excluded.source_digest,
                withdrawn=excluded.withdrawn, certified_generation=excluded.certified_generation",
                params![id,r.record_digest.as_slice(),r.first_generation,r.first_cut,
                    r.published_generation,r.source_digest.as_slice(),r.withdrawn,d.generation]).map_err(error)?;
        }
        for (table, rows) in [("witness",&d.witnesses),("observation",&d.observations)] {
            for (id,r) in rows {
                conn.execute(&format!("INSERT INTO {table} VALUES(?1,?2,?3,?4)
                    ON CONFLICT(id) DO UPDATE SET record_digest=excluded.record_digest,
                    source_digest=excluded.source_digest, certified_generation=excluded.certified_generation"),
                    params![id,r.record_digest.as_slice(),r.source_digest.as_slice(),d.generation]).map_err(error)?;
            }
        }
        for (seq,group) in &d.cover {
            conn.execute("INSERT INTO cover VALUES(?1,?2) ON CONFLICT(seq) DO UPDATE SET group_id=excluded.group_id",
                params![seq,group]).map_err(error)?;
        }
        for unit in &d.new_units {
            // new_units 必须真是本代新增集合，重复认证单位拒绝而非静默忽略。
            conn.execute("INSERT INTO node VALUES(?1,?2,?3,?4)",params![unit.digest.as_slice(),unit.flavor(),unit.mode(),d.generation]).map_err(error)?;
        }
        let roots = roots_value(&d.roots);
        let units = canonical_json(&serde_json::to_value(&d.new_units).map_err(error)?);
        let delta = json!({"generation":d.generation,"roots":roots,"verifier_revision":d.verifier_revision,
            "raw":d.raw,"objects":d.objects,"witnesses":d.witnesses,"observations":d.observations,
            "cover":d.cover,"new_units":d.new_units});
        let fold = sha256_hex(canonical_json(&json!([previous,delta])).as_bytes());
        conn.execute("INSERT INTO cert VALUES(?1,?2,?3,?4,?5,?6)",params![d.generation,
            canonical_json(&roots),sha256_hex(units.as_bytes()),i64::try_from(d.new_units.len()).map_err(error)?,
            fold,d.verifier_revision]).map_err(error)?;
        conn.execute("UPDATE meta SET certified_through_generation=?1,fold_digest=?2 WHERE singleton=1",
            params![d.generation,fold]).map_err(error)?;
        self.check_storage()?;
        self.phase = Phase::Pending(Pending {token:token.to_owned(),generation:d.generation,roots:d.roots.clone()});
        Ok(())
    }
    pub(super) fn require_pending(&self) -> Result<(), String> {
        if matches!(self.phase, Phase::Pending(_) | Phase::Bound(_)) { Ok(()) }
        else { Err(error("发布缺本请求台账变化或台账已失效")) }
    }
    pub(super) fn bind(&mut self, token: &str, m: &content_manifest::ManifestMetadata,
        roots: &impl content_manifest::ManifestRootSource) -> Result<(), String> {
        let v = roots.view();
        let expected: [RefView<'_>;4] = [(&m.scope).into(), v.generated_state,
            v.publication_template, v.delta_template];
        let result = match &self.phase {
            Phase::Pending(p) | Phase::Bound(p) if p.token == token && p.generation == m.generation
                && m.rule_revision == self.rule_revision => {
                if p.roots.iter().zip(expected).all(|(a,b)|root_value(a.into()) == root_value(b)) { Ok(()) }
                else { Err(error("台账四根与本请求 prepare 不一致")) }
            },
            _ => Err(error("台账请求、规则或代号不一致")),
        };
        if result.is_err() { self.invalidate(); return result; }
        let phase = std::mem::replace(&mut self.phase, Phase::Stale);
        self.phase = match phase { Phase::Pending(p) | Phase::Bound(p) => Phase::Bound(p), _ => unreachable!() };
        Ok(())
    }
    /// 只能由内容库 Commit 成功后的 owner 调用，发布前没有台账可见性。
    pub(super) fn content_committed(&mut self) -> Result<(), String> {
        let result = (|| {
            if !matches!(self.phase, Phase::Bound(_)) { return Err(error("台账尚未绑定实际 prepare")); }
            self.check_storage()?;
            self.connection.as_ref().unwrap().execute_batch("COMMIT").map_err(error)?;
            self.check_storage()?;
            Ok(())
        })();
        if result.is_ok() { self.phase = Phase::Ready; } else { self.invalidate(); }
        result
    }
    /// #1481 在原件单代核验后调用。在线与冷折叠只有内容 Commit 所在位置不同。
    pub(super) fn fold_step(&mut self, token:&str, d:&GenerationDelta,
        m:&content_manifest::ManifestMetadata, roots:&impl content_manifest::ManifestRootSource) -> Result<(),String> {
        self.stage(token,d)?;
        self.bind(token,m,roots)?;
        self.content_committed()
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    pub(crate) fn delta(m: &content_manifest::ManifestMetadata, r: &content_manifest::ManifestRoots) -> GenerationDelta {
        let g = m.generation;
        GenerationDelta {
            generation:g, roots:[m.scope.clone(),r.generated_state.clone(),r.publication_template.clone(),r.delta_template.clone()],
            verifier_revision:"TestOnly-storage-fixture/1".into(),
            raw:BTreeMap::from([(g-1,RawFact {event_digest:[g as u8;32],bar_digest:[2;32]})]),
            objects:BTreeMap::from([("object-a".into(),ObjectFact { record_digest:[3;32],first_generation:1,
                first_cut:"first-cut".into(),published_generation:g,source_digest:[4;32],withdrawn:g==3})]),
            witnesses:BTreeMap::from([("witness-a".into(),RelatedFact {record_digest:[5;32],source_digest:[6;32]})]),
            observations:BTreeMap::from([("observation-a".into(),RelatedFact {record_digest:[7;32],source_digest:[8;32]})]),
            cover:BTreeMap::from([(g-1,"group-a".into())]),
            new_units:BTreeSet::from([Unit {digest:[g as u8;32],flavor:Flavor::Value,mode:Mode::Public},
                Unit {digest:[g as u8;32],flavor:Flavor::Value,mode:Mode::Template},
                Unit {digest:[g as u8;32],flavor:Flavor::JsonInner,mode:Mode::Public}]),
        }
    }
    pub(crate) fn ledger(db:&Path,name:&str,rule:&str) -> Ledger {
        Ledger::fresh(&db.with_file_name(name),db,Identity {rule_revision:rule.into(),
            content_identity:"TestOnly-same-content".into(),binary_commit:"test-only".into()},64*1024*1024).unwrap()
    }
    pub(crate) fn dump(path:&Path) -> Vec<String> {
        let conn = Connection::open_with_flags(path,OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let mut result=Vec::new();
        for table in ["meta","raw","object","witness","observation","cover","node","cert"] {
            let mut s=conn.prepare(&format!("SELECT * FROM {table} ORDER BY {}", if table=="node" {"1,2,3"} else {"1"})).unwrap();
            let count=s.column_count();
            let rows=s.query_map([],|r| {
                (0..count).map(|i|r.get::<_,rusqlite::types::Value>(i)).collect::<rusqlite::Result<Vec<_>>>()
            }).unwrap();
            for row in rows { result.push(format!("{table}:{:?}",row.unwrap())); }
        }
        result
    }
    #[test]
    fn p6_ledger_component_fold_matches_online_rows_and_partition_keys() {
        let (f,_,token,mut m,r)=prepared::tests::p6_candidate();
        let db=f.db_path();
        let mut online=ledger(&db,"online.sqlite",&m.rule_revision);
        let mut folded=ledger(&db,"fold.sqlite",&m.rule_revision);
        for g in 1..=3 {
            m.generation=g;
            let d=delta(&m,&r);
            online.stage(&token,&d).unwrap();
            // Separate connection cannot observe any row of the pending generation.
            let reader=Connection::open(&online.path).unwrap();
            let through:i64=reader.query_row("SELECT certified_through_generation FROM meta",[],|r|r.get(0)).unwrap();
            assert_eq!(through,g-1);
            drop(reader);
            online.bind(&token,&m,&r).unwrap();
            online.content_committed().unwrap();
            folded.fold_step(&token,&d,&m,&r).unwrap();
            assert_eq!(dump(&online.path),dump(&folded.path));
        }
        let conn=online.connection.as_ref().unwrap();
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM node",[],|r|r.get::<_,i64>(0)).unwrap(),9);
        assert_eq!(conn.query_row("SELECT withdrawn FROM object",[],|r|r.get::<_,i64>(0)).unwrap(),1);
        assert_eq!(online.through().unwrap(),3);
    }
    #[test]
    fn p6_ledger_duplicate_unit_rolls_back_every_partition_and_discards() {
        let (f,_,token,mut m,r)=prepared::tests::p6_candidate();
        let mut l=ledger(&f.db_path(),"ledger.sqlite",&m.rule_revision);
        let first=delta(&m,&r);
        l.fold_step(&token,&first,&m,&r).unwrap();
        let before=dump(&l.path);
        m.generation=2;
        let mut second=delta(&m,&r);
        second.new_units=first.new_units;
        assert!(l.stage(&token,&second).is_err());
        assert!(l.connection.is_none());
        assert!(matches!(l.phase,Phase::Stale));
        assert_eq!(before,dump(&l.path));
        assert!(l.stage(&token,&second).is_err());
    }
    #[test]
    fn p6_ledger_fresh_never_trusts_or_overwrites_previous_file() {
        let (f,_,token,m,r)=prepared::tests::p6_candidate();
        let db=f.db_path();
        let mut l=ledger(&db,"ledger.sqlite",&m.rule_revision);
        l.fold_step(&token,&delta(&m,&r),&m,&r).unwrap();
        let before=dump(&l.path);
        assert!(Ledger::fresh(&l.path,&db,Identity {rule_revision:m.rule_revision.clone(),
            content_identity:"other".into(),binary_commit:"other".into()},64*1024*1024).is_err());
        assert_eq!(dump(&l.path),before);
    }
    #[test]
    fn p6_ledger_pending_request_roots_and_sequence_are_bound() {
        let (f,_,token,m,r)=prepared::tests::p6_candidate();
        for mutation in 0..4 {
            let mut l=ledger(&f.db_path(),&format!("bad-{mutation}.sqlite"),&m.rule_revision);
            let mut d=delta(&m,&r);
            if mutation==0 { d.generation=2; assert!(l.stage(&token,&d).is_err()); continue; }
            if mutation==1 { d.raw=BTreeMap::from([(2,RawFact {event_digest:[0;32],bar_digest:[0;32]})]);
                assert!(l.stage(&token,&d).is_err()); continue; }
            if mutation==2 { d.roots.swap(0,1); }
            l.stage(&token,&d).unwrap();
            assert!(l.bind(if mutation==3 {"different-request"} else {&token},&m,&r).is_err());
            assert!(l.connection.is_none());
            let c=Connection::open(&l.path).unwrap();
            assert_eq!(c.query_row("SELECT COUNT(*) FROM cert",[],|r|r.get::<_,i64>(0)).unwrap(),0);
        }
    }
    #[test]
    fn p6_ledger_budget_counts_content_and_sidecar_together() {
        let (f,_,_,m,_)=prepared::tests::p6_candidate();
        let db=f.db_path();
        let path=db.with_file_name("small.sqlite");
        let cap=database_bytes(&db).unwrap();
        assert!(Ledger::fresh(&path,&db,Identity {rule_revision:m.rule_revision,
            content_identity:"content".into(),binary_commit:"test".into()},cap).is_err());
        assert!(!path.exists());
    }
}
