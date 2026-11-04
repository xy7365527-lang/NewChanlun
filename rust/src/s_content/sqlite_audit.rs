use super::{
    job::CheckedFrame,
    raw_materials::{RawMaterials, Stats},
    CodecError, Flavor, Mode, NodeDigest, NodeRead, NodeSource, RefView, Verifier, WorkSet,
};
use rusqlite::{Connection, OptionalExtension};
use std::cell::RefCell;

pub const PROOF_CAP: usize = 3 * 1024 * 1024;
const ENTRIES: usize = 65536;
const META_RESERVE: usize = 4096;

#[derive(Clone, Copy, Debug, Default)]
pub struct ProofStats {
    pub hits: u64,
    pub commits: u64,
    pub entries: usize,
    pub probes: u64,
    pub retained_bytes: usize,
    pub disabled: bool,
    pub auxiliary_retained_bytes: usize,
    pub auxiliary_peak_bytes: usize,
    pub auxiliary_released_bytes: usize,
    pub combined_peak_bytes: usize,
}
pub(super) struct SuccessTable {
    slots: Vec<[u8; 33]>,
    next: usize,
    stats: ProofStats,
}
impl SuccessTable {
    fn new(entries: usize) -> Self {
        let mut this = Self {
            slots: Vec::new(),
            next: 0,
            stats: ProofStats::default(),
        };
        if entries == 0
            || entries % 4 != 0
            || entries
                .checked_mul(33)
                .and_then(|n| n.checked_add(META_RESERVE))
                .is_none_or(|n| n > PROOF_CAP)
            || this.slots.try_reserve_exact(entries).is_err()
        {
            this.stats.disabled = true;
            return this;
        }
        if this.slots.capacity() * std::mem::size_of::<[u8; 33]>() + META_RESERVE > PROOF_CAP {
            this.slots = Vec::new();
            this.stats.disabled = true;
            return this;
        }
        this.slots.resize(entries, [0; 33]);
        this.stats.retained_bytes = this.slots.capacity() * 33 + META_RESERVE;
        this.stats.combined_peak_bytes = this.stats.retained_bytes;
        this
    }
    fn code(mode: Mode, flavor: Flavor) -> u8 {
        1 + if mode == Mode::Template { 2 } else { 0 }
            + if flavor == Flavor::JsonInner { 1 } else { 0 }
    }
    fn start(&self, digest: &NodeDigest) -> usize {
        (u64::from_le_bytes(digest[..8].try_into().unwrap()) as usize % (self.slots.len() / 4)) * 4
    }
    pub(super) fn contains(&mut self, digest: &NodeDigest, mode: Mode, flavor: Flavor) -> bool {
        if self.slots.is_empty() {
            return false;
        }
        let start = self.start(digest);
        let code = Self::code(mode, flavor);
        for entry in &self.slots[start..start + 4] {
            self.stats.probes += 1;
            if entry[32] == code && &entry[..32] == digest {
                self.stats.hits += 1;
                return true;
            }
        }
        false
    }
    fn save(&mut self, digest: &NodeDigest, mode: Mode, flavor: Flavor) {
        if self.slots.is_empty() {
            return;
        }
        let start = self.start(digest);
        let code = Self::code(mode, flavor);
        if self.slots[start..start + 4]
            .iter()
            .any(|e| e[32] == code && &e[..32] == digest)
        {
            return;
        }
        let index = (start..start + 4)
            .find(|&i| self.slots[i][32] == 0)
            .unwrap_or(start + self.next % 4);
        self.next = self.next.wrapping_add(1);
        if self.slots[index][32] == 0 {
            self.stats.entries += 1;
        }
        self.slots[index][..32].copy_from_slice(digest);
        self.slots[index][32] = code;
    }
}

pub fn read_sql_material(
    conn: &Connection,
    digest: &NodeDigest,
) -> Result<Option<(u8, Vec<u8>)>, CodecError> {
    let row: Option<(i64, Vec<u8>)> = conn
        .prepare_cached("SELECT tag, body FROM s_content_nodes WHERE digest=?1")
        .map_err(storage_error)?
        .query_row(rusqlite::params![&digest[..]], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()
        .map_err(storage_error)?;
    row.map(|(tag, body)| {
        u8::try_from(tag)
            .map(|tag| (tag, body))
            .map_err(|_| CodecError::new(super::code::NODE_DIGEST, "节点 tag 超过一个字节"))
    })
    .transpose()
}
fn storage_error(error: impl std::fmt::Display) -> CodecError {
    CodecError::new(super::code::SOURCE, format!("S3 内容存储：{error}"))
}

/// 本次封闭写前审计的固定 SQL 基础域。材料与成功表均从空开始，不接受外部材料或成功条目。
pub struct SqliteAuditBase<'conn> {
    conn: &'conn Connection,
    materials: RefCell<RawMaterials>,
    proofs: RefCell<SuccessTable>,
}
const _: () = assert!(
    std::mem::size_of::<SqliteAuditBase<'static>>() * 4 + std::mem::size_of::<ProofStats>() * 4
        <= META_RESERVE
);
impl<'conn> SqliteAuditBase<'conn> {
    pub fn new(conn: &'conn Connection, material_metadata: usize) -> Self {
        Self {
            conn,
            materials: RefCell::new(RawMaterials::production(material_metadata)),
            proofs: RefCell::new(SuccessTable::new(ENTRIES)),
        }
    }
    pub fn material_stats(&self) -> Stats {
        self.materials.borrow().stats()
    }
    pub fn proof_stats(&self) -> ProofStats {
        self.proofs.borrow().stats
    }
    /// 仅一张附属短期证明表可占用余量。closure 固定容量不变。
    pub fn reserve_proof_bytes(&self, bytes: usize) -> Option<ProofLease<'_, 'conn>> {
        let mut proofs = self.proofs.borrow_mut();
        let total = proofs.stats.retained_bytes.checked_add(bytes)?;
        if proofs.stats.auxiliary_retained_bytes != 0 || bytes == 0 || total > PROOF_CAP {
            return None;
        }
        proofs.stats.auxiliary_retained_bytes = bytes;
        proofs.stats.auxiliary_peak_bytes = proofs.stats.auxiliary_peak_bytes.max(bytes);
        proofs.stats.combined_peak_bytes = proofs.stats.combined_peak_bytes.max(total);
        Some(ProofLease { base: self, bytes })
    }
    pub fn checked_read(&self, digest: &NodeDigest) -> Result<Option<CheckedFrame>, CodecError> {
        self.materials.borrow_mut().read_material(digest, || {
            read_sql_material(self.conn, digest)?
                .map(|(tag, body)| CheckedFrame::checked(digest, tag, body))
                .transpose()
        })
    }
    pub fn read(&self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        Ok(match self.checked_read(digest)? {
            None => NodeRead::Missing,
            Some(frame) => {
                let (tag, body) = frame.into_parts();
                NodeRead::Found { tag, body }
            }
        })
    }
    pub fn verify_ref(&self, reference: RefView<'_>, mode: Mode) -> Result<(), CodecError> {
        self.verify_many([reference], mode)
    }
    pub fn verify_many<'r>(
        &self,
        refs: impl IntoIterator<Item = RefView<'r>>,
        mode: Mode,
    ) -> Result<(), CodecError> {
        let mut reader = SqlReader(self);
        let mut verifier = Verifier::new(&mut reader, mode);
        verifier.base_proofs = Some(&self.proofs);
        for reference in refs {
            verifier.verify_refview(reference)?;
        }
        debug_assert!(verifier.todo.is_empty());
        let WorkSet::Plain(seen) = &verifier.seen else {
            unreachable!("固定 SQL 无预算域");
        };
        let mut proofs = self.proofs.borrow_mut();
        for &(digest, flavor) in seen {
            proofs.save(&digest, mode, flavor);
        }
        proofs.stats.commits += 1;
        Ok(())
    }
}
pub struct ProofLease<'a, 'conn> {
    base: &'a SqliteAuditBase<'conn>,
    bytes: usize,
}
impl Drop for ProofLease<'_, '_> {
    fn drop(&mut self) {
        let mut proofs = self.base.proofs.borrow_mut();
        proofs.stats.auxiliary_retained_bytes = 0;
        proofs.stats.auxiliary_released_bytes += self.bytes;
    }
}
struct SqlReader<'a, 'conn>(&'a SqliteAuditBase<'conn>);
impl NodeSource for SqlReader<'_, '_> {
    fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
        self.0.read(d)
    }
    fn checked_read(&mut self, d: &NodeDigest) -> Result<Option<CheckedFrame>, CodecError> {
        self.0.checked_read(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::s_content::{self as sc, ContentRef, Kind, MemoryNodes, Value};
    fn database(nodes: &MemoryNodes) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY, tag INTEGER, body BLOB) WITHOUT ROWID").unwrap();
        for (d, t, b) in nodes.iter() {
            conn.execute(
                "INSERT INTO s_content_nodes VALUES(?1,?2,?3)",
                rusqlite::params![&d[..], t, b],
            )
            .unwrap();
        }
        conn
    }
    fn root(nodes: &mut MemoryNodes) -> ContentRef {
        sc::write_value(
            nodes,
            &Value::Array(vec![Value::String("payload".repeat(20)), Value::Bool(true)]),
        )
        .unwrap()
        .content_ref()
    }
    fn oracle(conn: &Connection, r: &ContentRef, mode: Mode) -> Result<(), CodecError> {
        struct Source<'a>(&'a Connection);
        impl NodeSource for Source<'_> {
            fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
                Ok(match read_sql_material(self.0, d)? {
                    None => NodeRead::Missing,
                    Some((tag, body)) => NodeRead::Found { tag, body },
                })
            }
        }
        sc::verify_refview(&mut Source(conn), r.into(), mode)
    }
    #[test]
    fn base_success_reuses_only_closure_and_keeps_every_root_header() {
        let mut nodes = MemoryNodes::new();
        let r = root(&mut nodes);
        let conn = database(&nodes);
        let base = SqliteAuditBase::new(&conn, 4096);
        base.verify_ref((&r).into(), Mode::Public).unwrap();
        let first = base.material_stats();
        let before = sc::frame_calls_on_thread();
        base.verify_ref((&r).into(), Mode::Public).unwrap();
        let second = base.material_stats();
        assert_eq!(second.logical_reads - first.logical_reads, 1);
        assert_eq!(second.sql_material_reads, first.sql_material_reads);
        assert_eq!(sc::frame_calls_on_thread() - before, 1);
        assert!(base.proof_stats().hits > 0);
        for bad in [
            ContentRef::new(Kind::String, &r.digest().unwrap(), Some(2)),
            ContentRef::new(Kind::Array, &r.digest().unwrap(), Some(99)),
        ] {
            let before = base.proof_stats().commits;
            assert_eq!(
                base.verify_ref((&bad).into(), Mode::Public)
                    .unwrap_err()
                    .to_string(),
                oracle(&conn, &bad, Mode::Public).unwrap_err().to_string()
            );
            assert_eq!(base.proof_stats().commits, before);
        }
        eprintln!(
            "SQLITE_BASE_READS first={first:?} second={second:?} proof={:?}",
            base.proof_stats()
        );
    }
    #[test]
    fn base_mode_flavor_and_many_failure_never_publish_partial_success() {
        let mut nodes = MemoryNodes::new();
        let good = root(&mut nodes);
        let template = sc::write_template(&mut nodes, &Value::BatchSelf)
            .unwrap()
            .content_ref();
        let leaf = sc::write_value(&mut nodes, &Value::Bool(true))
            .unwrap()
            .content_ref();
        let inner = sc::frame(sc::tag::JSON_TEXT, &leaf.digest().unwrap());
        nodes.insert_unchecked(inner, sc::tag::JSON_TEXT, leaf.digest().unwrap().to_vec());
        let outer = sc::frame(sc::tag::JSON_TEXT, &inner);
        nodes.insert_unchecked(outer, sc::tag::JSON_TEXT, inner.to_vec());
        let bad = sc::frame(sc::tag::BOOL_TRUE, &[1]);
        nodes.insert_unchecked(bad, sc::tag::BOOL_TRUE, vec![1]);
        let bad = ContentRef::new(Kind::Bool, &bad, None);
        let conn = database(&nodes);
        let base = SqliteAuditBase::new(&conn, 4096);
        base.verify_ref((&template).into(), Mode::Template).unwrap();
        assert_eq!(
            base.verify_ref((&template).into(), Mode::Public)
                .unwrap_err()
                .to_string(),
            oracle(&conn, &template, Mode::Public)
                .unwrap_err()
                .to_string()
        );
        let inner = ContentRef::new(Kind::JsonText, &inner, None);
        let outer = ContentRef::new(Kind::JsonText, &outer, None);
        base.verify_ref((&inner).into(), Mode::Public).unwrap();
        assert_eq!(
            base.verify_ref((&outer).into(), Mode::Public)
                .unwrap_err()
                .to_string(),
            oracle(&conn, &outer, Mode::Public).unwrap_err().to_string()
        );
        drop(base);
        let base = SqliteAuditBase::new(&conn, 4096);
        for _ in 0..2 {
            assert_eq!(
                base.verify_many([(&good).into(), (&bad).into()], Mode::Public)
                    .unwrap_err()
                    .to_string(),
                oracle(&conn, &bad, Mode::Public).unwrap_err().to_string()
            );
            assert_eq!(base.proof_stats().entries, 0);
            assert_eq!(base.proof_stats().commits, 0);
        }
        let before = base.material_stats().logical_reads;
        base.verify_ref((&good).into(), Mode::Public).unwrap();
        assert!(base.material_stats().logical_reads - before > 1);
    }
    #[test]
    fn base_new_audit_observes_changed_and_missing_sql_materials() {
        let mut nodes = MemoryNodes::new();
        let r = root(&mut nodes);
        let conn = database(&nodes);
        {
            let base = SqliteAuditBase::new(&conn, 4096);
            base.verify_ref((&r).into(), Mode::Public).unwrap();
        }
        conn.execute(
            "UPDATE s_content_nodes SET body=x'01' WHERE digest=?1",
            rusqlite::params![&r.digest().unwrap()[..]],
        )
        .unwrap();
        let base = SqliteAuditBase::new(&conn, 4096);
        assert_eq!(base.proof_stats().entries, 0);
        assert_eq!(
            base.verify_ref((&r).into(), Mode::Public)
                .unwrap_err()
                .to_string(),
            oracle(&conn, &r, Mode::Public).unwrap_err().to_string()
        );
        assert_eq!(base.proof_stats().commits, 0);
        drop(base);
        conn.execute(
            "DELETE FROM s_content_nodes WHERE digest=?1",
            rusqlite::params![&r.digest().unwrap()[..]],
        )
        .unwrap();
        let base = SqliteAuditBase::new(&conn, 4096);
        assert_eq!(
            base.verify_ref((&r).into(), Mode::Public)
                .unwrap_err()
                .to_string(),
            oracle(&conn, &r, Mode::Public).unwrap_err().to_string()
        );
    }
    #[test]
    fn base_full_table_and_disabled_table_preserve_verifier_results() {
        let mut nodes = MemoryNodes::new();
        let roots: Vec<_> = (0..20)
            .map(|n| {
                sc::write_value(&mut nodes, &Value::String(format!("value-{n}")))
                    .unwrap()
                    .content_ref()
            })
            .collect();
        let conn = database(&nodes);
        for capacity in [0, 4] {
            let base = SqliteAuditBase {
                conn: &conn,
                materials: RefCell::new(RawMaterials::production(4096)),
                proofs: RefCell::new(SuccessTable::new(capacity)),
            };
            for r in roots.iter().chain(roots.iter().rev()) {
                oracle(&conn, r, Mode::Public).unwrap();
                base.verify_ref(r.into(), Mode::Public).unwrap();
                assert!(base.proof_stats().entries <= capacity);
            }
        }
        let base = SqliteAuditBase::new(&conn, 4096);
        let stats = base.proof_stats();
        assert_eq!(stats.retained_bytes, 65536 * 33 + 4096);
        assert!(stats.retained_bytes <= PROOF_CAP);
        assert_eq!(base.material_stats().slab_capacity, 33550336);
        eprintln!(
            "SQLITE_BASE_LAYOUT table_capacity={} table_entry_size={} owner={} retained={}",
            base.proofs.borrow().slots.capacity(),
            std::mem::size_of::<[u8; 33]>(),
            std::mem::size_of_val(&base),
            stats.retained_bytes
        );
    }
}
