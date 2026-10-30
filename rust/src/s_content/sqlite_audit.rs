use super::{raw_materials::{RawMaterials, Stats}, job::CheckedFrame, CodecError,
    NodeDigest, NodeRead, NodeSource, RefView, Mode, Flavor, Verifier, WorkSet};
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
}
pub(super) struct SuccessTable {
    slots: Vec<[u8; 33]>,
    next: usize,
    stats: ProofStats,
}
impl SuccessTable {
    fn new(entries: usize) -> Self {
        let mut this = Self { slots: Vec::new(), next: 0, stats: ProofStats::default() };
        if entries == 0 || entries % 4 != 0
            || entries.checked_mul(33).and_then(|n| n.checked_add(META_RESERVE))
                .is_none_or(|n| n > PROOF_CAP)
            || this.slots.try_reserve_exact(entries).is_err() {
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
        this
    }
    fn code(mode: Mode, flavor: Flavor) -> u8 {
        1 + if mode == Mode::Template { 2 } else { 0 }
          + if flavor == Flavor::JsonInner { 1 } else { 0 }
    }
    fn start(&self, digest: &NodeDigest) -> usize {
        (u64::from_le_bytes(digest[..8].try_into().unwrap()) as usize
            % (self.slots.len() / 4)) * 4
    }
    pub(super) fn contains(&mut self, digest: &NodeDigest, mode: Mode, flavor: Flavor) -> bool {
        if self.slots.is_empty() { return false; }
        let start = self.start(digest);
        let code = Self::code(mode, flavor);
        for entry in &self.slots[start..start+4] {
            self.stats.probes += 1;
            if entry[32] == code && &entry[..32] == digest {
                self.stats.hits += 1;
                return true;
            }
        }
        false
    }
    fn save(&mut self, digest: &NodeDigest, mode: Mode, flavor: Flavor) {
        if self.slots.is_empty() { return; }
        let start = self.start(digest);
        let code = Self::code(mode, flavor);
        if self.slots[start..start+4].iter().any(|e| e[32] == code && &e[..32] == digest) {
            return;
        }
        let index = (start..start+4).find(|&i| self.slots[i][32] == 0)
            .unwrap_or(start + self.next % 4);
        self.next = self.next.wrapping_add(1);
        if self.slots[index][32] == 0 { self.stats.entries += 1; }
        self.slots[index][..32].copy_from_slice(digest);
        self.slots[index][32] = code;
    }
}

pub fn read_sql_material(conn: &Connection, digest: &NodeDigest)
    -> Result<Option<(u8, Vec<u8>)>, CodecError> {
    let row: Option<(i64, Vec<u8>)> = conn
        .prepare_cached("SELECT tag, body FROM s_content_nodes WHERE digest=?1")
        .map_err(storage_error)?
        .query_row(rusqlite::params![&digest[..]], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional().map_err(storage_error)?;
    row.map(|(tag, body)| {
        u8::try_from(tag).map(|tag| (tag, body))
            .map_err(|_| CodecError::new(super::code::NODE_DIGEST, "节点 tag 超过一个字节"))
    }).transpose()
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
const _: () = assert!(std::mem::size_of::<SqliteAuditBase<'static>>() * 4
    + std::mem::size_of::<ProofStats>() * 4 <= META_RESERVE);
impl<'conn> SqliteAuditBase<'conn> {
    pub fn new(conn: &'conn Connection, material_metadata: usize) -> Self {
        Self { conn, materials: RefCell::new(RawMaterials::production(material_metadata)),
            proofs: RefCell::new(SuccessTable::new(ENTRIES)) }
    }
    pub fn material_stats(&self) -> Stats { self.materials.borrow().stats() }
    pub fn proof_stats(&self) -> ProofStats { self.proofs.borrow().stats }
    pub fn checked_read(&self, digest: &NodeDigest) -> Result<Option<CheckedFrame>, CodecError> {
        self.materials.borrow_mut().read_material(digest, || {
            read_sql_material(self.conn, digest)?
                .map(|(tag, body)| CheckedFrame::checked(digest, tag, body)).transpose()
        })
    }
    pub fn read(&self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        Ok(match self.checked_read(digest)? {
            None => NodeRead::Missing,
            Some(frame) => { let (tag, body) = frame.into_parts(); NodeRead::Found { tag, body } }
        })
    }
    pub fn verify_ref(&self, reference: RefView<'_>, mode: Mode) -> Result<(), CodecError> {
        self.verify_many([reference], mode)
    }
    pub fn verify_many<'r>(&self, refs: impl IntoIterator<Item=RefView<'r>>, mode: Mode)
        -> Result<(), CodecError> {
        let mut reader = SqlReader(self);
        let mut verifier = Verifier::new(&mut reader, mode);
        verifier.base_proofs = Some(&self.proofs);
        for reference in refs { verifier.verify_refview(reference)?; }
        debug_assert!(verifier.todo.is_empty());
        let WorkSet::Plain(seen) = &verifier.seen else { unreachable!("固定 SQL 无预算域"); };
        let mut proofs = self.proofs.borrow_mut();
        for &(digest, flavor) in seen { proofs.save(&digest, mode, flavor); }
        proofs.stats.commits += 1;
        Ok(())
    }
}
struct SqlReader<'a, 'conn>(&'a SqliteAuditBase<'conn>);
impl NodeSource for SqlReader<'_, '_> {
    fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> { self.0.read(d) }
    fn checked_read(&mut self, d: &NodeDigest) -> Result<Option<CheckedFrame>, CodecError> {
        self.0.checked_read(d)
    }
}
