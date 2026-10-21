//! #1394 C5.7: immutable, owner-local S content for economic-session/2.
//! The success domain is a dedicated fixed-capacity HFS+ volume per owner.
//! Kernel ENOSPC bounds all allocation on that volume; the DB/WAL/SHM files
//! are still measured with st_blocks for actual Job reconciliation. A node's
//! framed bytes or SQLite's logical page count never stand in for disk use.
use super::financial_content::FinancialStage;
use super::store::{ReadBudget, SCHEMA_V2};
use crate::s_content::{
    self, code,
    job::{Existing, Job, JobLimits},
    CodecError, ContentRef, NodeDigest, NodeRead, NodeSource,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::collections::BTreeSet;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")] // #1608：唯一消费点在 configure 的 macOS 分支
const SQLITE_SOURCE_ID: &str =
    "2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24";

fn error(message: impl std::fmt::Display) -> CodecError {
    CodecError::new(code::SINK, format!("E/B content store: {message}"))
}
fn io(error_value: impl std::fmt::Display) -> CodecError {
    error(error_value)
}
fn sidecar(db: &Path, suffix: &str) -> PathBuf {
    let mut name = db.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}
fn allocated_path(path: &Path) -> Result<u64, CodecError> {
    match std::fs::metadata(path) {
        Ok(metadata) => metadata
            .blocks()
            .checked_mul(512)
            .ok_or_else(|| error("st_blocks overflow")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(io(e)),
    }
}
/// Includes economic history, content rows, B-tree indexes and the WAL/SHM files.
pub(super) fn allocated(db: &Path) -> Result<u64, CodecError> {
    let mut total = 0u64;
    for path in [db.to_path_buf(), sidecar(db, "-wal"), sidecar(db, "-shm")] {
        total = total
            .checked_add(allocated_path(&path)?)
            .ok_or_else(|| error("physical total overflow"))?;
    }
    Ok(total)
}
/// This is an OS-enforced upper bound, not an estimate from SQLite page bytes.
/// Every economic owner has its own mount root and only its DB/WAL/SHM files
/// on it. The HFS+ fixed image's size and mount are supplied before init.
pub(super) fn carrier_capacity(db: &Path) -> Result<u64, CodecError> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::ffi::OsStrExt;
        let target = db
            .parent()
            .ok_or_else(|| error("DB has no parent"))?
            .canonicalize()
            .map_err(io)?;
        let path = std::ffi::CString::new(target.as_os_str().as_bytes()).map_err(io)?;
        let mut status = std::mem::MaybeUninit::<libc::statfs>::uninit();
        if unsafe { libc::statfs(path.as_ptr(), status.as_mut_ptr()) } != 0 {
            return Err(io(std::io::Error::last_os_error()));
        }
        let status = unsafe { status.assume_init() };
        let kind: Vec<u8> = status
            .f_fstypename
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| b as u8)
            .collect();
        if kind != b"hfs"
            || status.f_flags & libc::MNT_LOCAL as u32 == 0
            || status.f_flags & libc::MNT_RDONLY as u32 != 0
        {
            return Err(error(
                "new domain requires a local writable fixed HFS+ carrier",
            ));
        }
        let mounted: Vec<u8> = status
            .f_mntonname
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| b as u8)
            .collect();
        let mount = PathBuf::from(std::ffi::OsStr::from_bytes(&mounted))
            .canonicalize()
            .map_err(io)?;
        if target != mount {
            return Err(error(
                "owner DB must live directly at its dedicated mount root",
            ));
        }
        let dev = target.metadata().map_err(io)?.dev();
        for file in [db.to_path_buf(), sidecar(db, "-wal"), sidecar(db, "-shm")] {
            match std::fs::symlink_metadata(&file) {
                Ok(meta)
                    if meta.file_type().is_file() && meta.nlink() == 1 && meta.dev() == dev =>
                {
                    ()
                }
                Ok(_) => {
                    return Err(error(
                        "owner DB/WAL/SHM must be regular unlinked-once files on one carrier",
                    ))
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(io(e)),
            }
        }
        let mut vfs = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        if unsafe { libc::statvfs(path.as_ptr(), vfs.as_mut_ptr()) } != 0 {
            return Err(io(std::io::Error::last_os_error()));
        }
        let vfs = unsafe { vfs.assume_init() };
        let cap = u128::from(vfs.f_blocks) * u128::from(vfs.f_frsize);
        let statfs_cap =
            u128::from(status.f_blocks) * u128::from(u64::try_from(status.f_bsize).map_err(io)?);
        if cap == 0 || cap != statfs_cap {
            return Err(error("carrier statvfs/statfs capacity disagree"));
        }
        u64::try_from(cap).map_err(|_| error("carrier capacity exceeds u64"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = db;
        Err(error(
            "new domain fixed-volume bound is only established on macOS HFS+",
        ))
    }
}
fn pragma_i64(c: &Connection, name: &str) -> Result<i64, CodecError> {
    c.query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
        .map_err(io)
}
/// Leave two bounded full-page WAL/checkpoint envelopes unused for deletion
/// after a failed stage, including a killed process resumed by recover.
/// This intentionally narrows the named 16 MiB HFS trial's usable DB pages.
fn cleanup_room(c: &Connection, db: &Path, include_next_write: bool) -> Result<(), CodecError> {
    cleanup_room_metered(c, db, include_next_write, &mut ReadBudget::new(None, None))
}
fn cleanup_room_metered(
    c: &Connection,
    db: &Path,
    include_next_write: bool,
    budget: &mut ReadBudget<'_>,
) -> Result<(), CodecError> {
    let pages: i64 = budget
        .one(c, "PRAGMA max_page_count", [], |r| r.get(0))
        .map_err(io)?;
    let size: i64 = budget
        .one(c, "PRAGMA page_size", [], |r| r.get(0))
        .map_err(io)?;
    let pages = u128::from(u64::try_from(pages).map_err(io)?);
    let size = u128::from(u64::try_from(size).map_err(io)?);
    let one = pages * (size + 24) + 65536;
    let needed = if include_next_write { 4 * one } else { 2 * one };
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::ffi::OsStrExt;
        let parent = db.parent().ok_or_else(|| error("DB has no parent"))?;
        let path = std::ffi::CString::new(parent.as_os_str().as_bytes()).map_err(io)?;
        let mut vfs = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        if unsafe { libc::statvfs(path.as_ptr(), vfs.as_mut_ptr()) } != 0 {
            return Err(io(std::io::Error::last_os_error()));
        }
        let vfs = unsafe { vfs.assume_init() };
        let free = u128::from(vfs.f_bavail) * u128::from(vfs.f_frsize);
        if free < needed {
            return Err(error(format!(
                "fixed carrier lacks prepaid cleanup room: free={free}, needed={needed}"
            )));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (db, needed);
        Err(error("cleanup room requires fixed HFS carrier"))
    }
}
fn format_guard(c: &Connection) -> Result<(), CodecError> {
    format_guard_metered(c, &mut ReadBudget::new(None, None))
}
fn format_guard_metered(c: &Connection, budget: &mut ReadBudget<'_>) -> Result<(), CodecError> {
    let revision: String = budget
        .one(c, "SELECT v FROM meta WHERE k='schema_revision'", [], |r| {
            r.get(0)
        })
        .map_err(io)?;
    let format: String = budget
        .one(
            c,
            "SELECT content_format FROM content_meta WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .map_err(io)?;
    if revision != SCHEMA_V2 || format != s_content::CONTENT_FORMAT {
        return Err(error("domain/content format mismatch"));
    }
    Ok(())
}
fn checkpoint(c: &Connection) -> Result<(), CodecError> {
    checkpoint_metered(c, &mut ReadBudget::new(None, None))
}
fn checkpoint_metered(c: &Connection, budget: &mut ReadBudget<'_>) -> Result<(), CodecError> {
    let (busy, _, _): (i64, i64, i64) = budget
        .one(c, "PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(io)?;
    if busy != 0 {
        return Err(error("WAL checkpoint blocked by another reader"));
    }
    Ok(())
}
fn wal_empty(db: &Path) -> Result<(), CodecError> {
    let wal = sidecar(db, "-wal");
    match std::fs::metadata(&wal) {
        Ok(meta) if meta.len() == 0 => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(error("TRUNCATE checkpoint left nonempty WAL")),
        Err(e) => Err(io(e)),
    }
}
/// A checkpoint itself can allocate main DB pages while the old WAL still
/// exists. Reserve its whole remaining envelope before invoking SQLite.
fn charged_checkpoint(
    c: &Connection,
    db: &Path,
    job: &mut Job,
    ceiling: u64,
    staged: bool,
) -> Result<(), CodecError> {
    charged_checkpoint_with_budget(
        c,
        db,
        &mut ReadBudget::new(Some(job), None),
        ceiling,
        staged,
    )
}
fn charged_checkpoint_with_budget(
    c: &Connection,
    db: &Path,
    budget: &mut ReadBudget<'_>,
    ceiling: u64,
    staged: bool,
) -> Result<(), CodecError> {
    budget.before().map_err(io)?;
    let before = allocated(db)?;
    let worst = ceiling
        .checked_sub(before)
        .ok_or_else(|| error("allocation above checkpoint ceiling"))?;
    let job = budget
        .job
        .as_deref_mut()
        .ok_or_else(|| error("checkpoint Job missing"))?;
    let grant = if staged {
        job.reserve_staged(worst)?
    } else {
        job.reserve_store(worst)?
    };
    let result = checkpoint_metered(c, budget);
    let after = allocated(db)?;
    // checkpoint may already allocate before read accounting reports expiry/limit.
    budget
        .job
        .as_deref_mut()
        .ok_or_else(|| error("checkpoint Job missing"))?
        .reconcile(&grant, after.saturating_sub(before))?;
    result?;
    if after > ceiling {
        return Err(error("checkpoint exceeded fixed carrier capacity"));
    }
    wal_empty(db)?;
    Ok(())
}
/// Process-local pragma setup must be repeated on every new writer connection.
/// `max_page_count` is an additional SQLite guard; the kernel's finite carrier
/// capacity is the physical proof, including APFS-independent allocation effects.
pub(super) fn configure(c: &Connection, db: &Path, limits: JobLimits) -> Result<u64, CodecError> {
    format_guard(c)?;
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (db, limits);
        return Err(error("new domain physical budget is macOS-only"));
    }
    #[cfg(target_os = "macos")]
    {
        let source: String = c
            .query_row("SELECT sqlite_source_id()", [], |r| r.get(0))
            .map_err(io)?;
        if source != SQLITE_SOURCE_ID {
            return Err(error("SQLite source id differs from audited 3.53.2 pager"));
        }
        let mode: String = c
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .map_err(io)?;
        if mode != "wal" {
            return Err(error("WAL mode required"));
        }
        c.execute_batch(
            "PRAGMA cache_spill=OFF; PRAGMA wal_autocheckpoint=0; PRAGMA temp_store=MEMORY;",
        )
        .map_err(io)?;
        if pragma_i64(c, "cache_spill")? != 0
            || pragma_i64(c, "wal_autocheckpoint")? != 0
            || pragma_i64(c, "temp_store")? != 2
        {
            return Err(error("SQLite budget pragmas did not take effect"));
        }
        let page_size = u64::try_from(pragma_i64(c, "page_size")?).map_err(io)?;
        let capacity = carrier_capacity(db)?;
        if capacity > limits.max_store_bytes || capacity > limits.max_staged_bytes {
            return Err(error(
                "fixed carrier capacity exceeds frozen staged/store limits",
            ));
        }
        // A page cap derived from this carrier, with room for a complete
        // failed transaction and a separate cleanup/checkpoint transaction.
        let max_pages = capacity / (8 * (page_size + 24));
        if max_pages == 0 || u64::try_from(pragma_i64(c, "page_count")?).map_err(io)? > max_pages {
            return Err(error("current economic DB cannot fit fixed carrier"));
        }
        let set: i64 = c
            .query_row(&format!("PRAGMA max_page_count={max_pages}"), [], |r| {
                r.get(0)
            })
            .map_err(io)?;
        if u64::try_from(set).map_err(io)? != max_pages {
            return Err(error("max_page_count did not take effect"));
        }
        if allocated(db)? > capacity {
            return Err(error("existing DB/WAL/SHM allocation exceeds carrier"));
        }
        Ok(capacity)
    }
}
/// Existing storage always counts actual physical allocation, including any old
/// unreachable staging rows. If such rows exist we classify the entire DB as
/// staged, conservatively, until the owner has cleaned them after recovery.
pub(super) fn existing(c: &Connection, db: &Path) -> Result<Existing, CodecError> {
    format_guard(c)?;
    let total = allocated(db)?;
    let staged: i64 = c
        .query_row("SELECT count(*) FROM content_staging", [], |r| r.get(0))
        .map_err(io)?;
    Ok(Existing {
        store_bytes: total,
        staged_bytes: if staged > 0 { total } else { 0 },
    })
}
fn checked(digest: &NodeDigest, tag: i64, body: Vec<u8>) -> Result<NodeRead, CodecError> {
    let tag = u8::try_from(tag).map_err(io)?;
    if s_content::frame(tag, &body) != *digest {
        return Err(error("same digest has different bytes"));
    }
    Ok(NodeRead::Found { tag, body })
}
fn read_table(c: &Connection, table: &str, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
    read_table_metered(c, table, digest, &mut ReadBudget::new(None, None))
}
pub(super) fn read_table_metered(
    c: &Connection,
    table: &str,
    digest: &NodeDigest,
    budget: &mut ReadBudget<'_>,
) -> Result<NodeRead, CodecError> {
    let sql = match table {
        "content_nodes" => "SELECT tag,body FROM content_nodes WHERE digest=?1",
        "content_staging" => "SELECT tag,body FROM content_staging WHERE digest=?1",
        _ => return Err(error("unknown content table")),
    };
    // The actual SQL payload (i64 tag + BLOB body) is charged by the common
    // row reader before tag conversion or digest checks, including failures.
    let row: Option<(i64, Vec<u8>)> = budget
        .optional(c, sql, params![&digest[..]], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(io)?;
    match row {
        Some((tag, body)) => checked(digest, tag, body),
        None => Ok(NodeRead::Missing),
    }
}
fn read_any(c: &Connection, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
    read_any_metered(c, digest, &mut ReadBudget::new(None, None))
}
pub(super) fn read_any_metered(
    c: &Connection,
    digest: &NodeDigest,
    budget: &mut ReadBudget<'_>,
) -> Result<NodeRead, CodecError> {
    let a = read_table_metered(c, "content_nodes", digest, budget)?;
    let b = read_table_metered(c, "content_staging", digest, budget)?;
    if let (NodeRead::Found { tag: at, body: ab }, NodeRead::Found { tag: bt, body: bb }) = (&a, &b)
    {
        if at != bt || ab != bb {
            return Err(error("same digest conflicts across content tables"));
        }
    }
    if matches!(a, NodeRead::Found { .. }) {
        Ok(a)
    } else {
        Ok(b)
    }
}
/// Owner stage is a real SQLite row under the same economic DB, never a
/// MemoryNodes surrogate. It has no permission to publish an economic image.
pub(super) struct Stage<'a> {
    c: &'a mut Connection,
    db: PathBuf,
    token: String,
    ceiling: u64,
}
impl<'a> Stage<'a> {
    pub(super) fn new(
        c: &'a mut Connection,
        db: &Path,
        limits: JobLimits,
        token: String,
    ) -> Result<Self, CodecError> {
        if token.is_empty() || token.len() > 256 {
            return Err(error("invalid stage token"));
        }
        let ceiling = configure(c, db, limits)?;
        Ok(Self {
            c,
            db: db.to_path_buf(),
            token,
            ceiling,
        })
    }
}
impl NodeSource for Stage<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        read_any(self.c, digest)
    }
}
/// #1394 第三问选项 A：一块里尚未在本地（公开或暂存）的节点，按摘要升序、块内去重。
/// 先逐个重算 F(tag, body)；同根不同字节（块内或与本地）一律拒绝，不写任何行。
#[cfg(test)]
fn batch_missing<'n>(
    c: &Connection,
    nodes: &'n [(NodeDigest, u8, Vec<u8>)],
) -> Result<Vec<&'n (NodeDigest, u8, Vec<u8>)>, CodecError> {
    batch_missing_metered(c, nodes, &mut ReadBudget::new(None, None))
}
fn batch_missing_metered<'n>(
    c: &Connection,
    nodes: &'n [(NodeDigest, u8, Vec<u8>)],
    budget: &mut ReadBudget<'_>,
) -> Result<Vec<&'n (NodeDigest, u8, Vec<u8>)>, CodecError> {
    let mut chosen: std::collections::BTreeMap<NodeDigest, &'n (NodeDigest, u8, Vec<u8>)> =
        std::collections::BTreeMap::new();
    for node in nodes {
        budget.before().map_err(io)?;
        let (digest, tag, body) = node;
        if s_content::frame(*tag, body) != *digest {
            return Err(error("node digest mismatch"));
        }
        if let Some(prior) = chosen.get(digest) {
            if prior.1 != *tag || prior.2 != *body {
                return Err(error("same digest conflict within batch"));
            }
            continue;
        }
        match read_any_metered(c, digest, budget)? {
            NodeRead::Found {
                tag: old_tag,
                body: old_body,
            } => {
                if old_tag != *tag || old_body != *body {
                    return Err(error("same digest conflict"));
                }
            }
            NodeRead::Missing => {
                chosen.insert(*digest, node);
            }
        }
    }
    Ok(chosen.into_values().collect())
}
/// 一块 = 一个 SQLite 事务：调用方持有事务，失败即整块回滚（drop 未提交事务）。返回写入的摘要。
#[cfg(test)]
fn insert_staging_batch(
    tx: &Transaction<'_>,
    token: &str,
    nodes: &[&(NodeDigest, u8, Vec<u8>)],
) -> Result<Vec<NodeDigest>, CodecError> {
    insert_staging_batch_metered(tx, token, nodes, &mut ReadBudget::new(None, None))
}
fn insert_staging_batch_metered(
    tx: &Transaction<'_>,
    token: &str,
    nodes: &[&(NodeDigest, u8, Vec<u8>)],
    budget: &mut ReadBudget<'_>,
) -> Result<Vec<NodeDigest>, CodecError> {
    budget.before().map_err(io)?;
    let mut insert = tx
        .prepare_cached("INSERT INTO content_staging(digest,tag,body,stage)VALUES(?1,?2,?3,?4)")
        .map_err(io)?;
    let mut inserted = Vec::with_capacity(nodes.len());
    for (digest, tag, body) in nodes.iter().copied() {
        budget.before().map_err(io)?;
        insert
            .execute(params![&digest[..], i64::from(*tag), body, token])
            .map_err(io)?;
        inserted.push(*digest);
    }
    Ok(inserted)
}
/// 只删本 stage token 写下、且在给定摘要表里的暂存行；公开表与他 token 的行不动。
fn delete_staged_rows(
    tx: &Transaction<'_>,
    token: &str,
    digests: &[NodeDigest],
) -> Result<u64, CodecError> {
    let mut delete = tx
        .prepare_cached("DELETE FROM content_staging WHERE digest=?1 AND stage=?2")
        .map_err(io)?;
    let mut removed = 0u64;
    for digest in digests {
        removed +=
            u64::try_from(delete.execute(params![&digest[..], token]).map_err(io)?).map_err(io)?;
    }
    Ok(removed)
}
impl Stage<'_> {
    /// 与 `put_budgeted` 同一组物理判据（cleanup_room、charged_checkpoint、整余量预留、写后对账、
    /// TRUNCATE checkpoint、载体上限），只把逐节点的三次持久动作合成每块一次：一次预留、一个事务、
    /// 一次 checkpoint。块内全部已在本地则不预留、不写。
    /// r2（独审 N1）：块事务一提交，本块实际插入的摘要立即追加到 `inserted`，然后才做 checkpoint、
    /// 对账、WAL 与载体上限这些可能失败的提交后步骤；只追加本块插入的行，块里已在本地的节点不追加。
    fn put_batch_inner(
        &mut self,
        job: &mut Job,
        nodes: &[(NodeDigest, u8, Vec<u8>)],
        inserted: &mut Vec<NodeDigest>,
        deadline: Option<std::time::Instant>,
    ) -> Result<(), CodecError> {
        let mut budget = ReadBudget::new(Some(job), deadline);
        budget.before().map_err(io)?;
        let missing = batch_missing_metered(self.c, nodes, &mut budget)?;
        if missing.is_empty() {
            return budget.before().map_err(io);
        }
        cleanup_room_metered(self.c, &self.db, true, &mut budget)?;
        charged_checkpoint_with_budget(self.c, &self.db, &mut budget, self.ceiling, true)?;
        cleanup_room_metered(self.c, &self.db, true, &mut budget)?;
        let before = allocated(&self.db)?;
        let worst = self
            .ceiling
            .checked_sub(before)
            .ok_or_else(|| error("physical allocation above ceiling"))?;
        let reservation = budget
            .job
            .as_deref_mut()
            .ok_or_else(|| error("stage Job missing"))?
            .reserve_staged(worst)?;
        let token = self.token.clone();
        let written = (|| -> Result<Vec<NodeDigest>, CodecError> {
            budget.before().map_err(io)?;
            let tx = self.c.transaction().map_err(io)?;
            let rows = insert_staging_batch_metered(&tx, &token, &missing, &mut budget)?;
            budget.before().map_err(io)?;
            tx.commit().map_err(io)?;
            Ok(rows)
        })();
        match written {
            // 已提交：先进调用方的回滚清单，再做任何可能失败的提交后步骤（N1）。
            Ok(rows) => inserted.extend(rows),
            Err(e) => {
                // 未提交的块已随事务回滚；物理分配照实对账。
                let after = allocated(&self.db)?;
                budget
                    .job
                    .as_deref_mut()
                    .ok_or_else(|| error("stage Job missing"))?
                    .reconcile(&reservation, after.saturating_sub(before))?;
                return Err(e);
            }
        }
        let check = checkpoint_metered(self.c, &mut budget);
        let after = allocated(&self.db)?;
        budget
            .job
            .as_deref_mut()
            .ok_or_else(|| error("stage Job missing"))?
            .reconcile(&reservation, after.saturating_sub(before))?;
        check?;
        wal_empty(&self.db)?;
        cleanup_room_metered(self.c, &self.db, false, &mut budget)?;
        if after > self.ceiling {
            return Err(error(
                "actual SQLite allocation exceeded fixed carrier capacity",
            ));
        }
        budget.before().map_err(io)
    }
    fn put_node_inner(
        &mut self,
        job: &mut Job,
        digest: NodeDigest,
        tag: u8,
        body: &[u8],
        deadline: Option<std::time::Instant>,
    ) -> Result<(), CodecError> {
        let mut budget = ReadBudget::new(Some(job), deadline);
        budget.before().map_err(io)?;
        if s_content::frame(tag, body) != digest {
            return Err(error("node digest mismatch"));
        }
        if let NodeRead::Found {
            tag: old_tag,
            body: old_body,
        } = read_any_metered(self.c, &digest, &mut budget)?
        {
            if old_tag != tag || old_body != body {
                return Err(error("same digest conflict"));
            }
            return budget.before().map_err(io);
        }
        cleanup_room_metered(self.c, &self.db, true, &mut budget)?;
        charged_checkpoint_with_budget(self.c, &self.db, &mut budget, self.ceiling, true)?;
        cleanup_room_metered(self.c, &self.db, true, &mut budget)?;
        let before = allocated(&self.db)?;
        let worst = self
            .ceiling
            .checked_sub(before)
            .ok_or_else(|| error("physical allocation above ceiling"))?;
        let reservation = budget
            .job
            .as_deref_mut()
            .ok_or_else(|| error("stage Job missing"))?
            .reserve_staged(worst)?;
        let inserted = budget.before().map_err(io).and_then(|()| {
            self.c
                .execute(
                    "INSERT INTO content_staging(digest,tag,body,stage)VALUES(?1,?2,?3,?4)",
                    params![&digest[..], i64::from(tag), body, &self.token],
                )
                .map_err(io)
        });
        if let Err(e) = inserted {
            let after = allocated(&self.db)?;
            budget
                .job
                .as_deref_mut()
                .ok_or_else(|| error("stage Job missing"))?
                .reconcile(&reservation, after.saturating_sub(before))?;
            return Err(e);
        }
        // Checkpoint here makes the next write start from an empty WAL. The
        // maximum physical pages were already reserved before the insertion.
        // This second checkpoint was itself covered by the same reservation:
        // before the write the WAL was empty and no second writer can enter
        // the owner process lock. Its full peak is <= the global envelope.
        let check = checkpoint_metered(self.c, &mut budget);
        let after = allocated(&self.db)?;
        budget
            .job
            .as_deref_mut()
            .ok_or_else(|| error("stage Job missing"))?
            .reconcile(&reservation, after.saturating_sub(before))?;
        check?;
        wal_empty(&self.db)?;
        cleanup_room_metered(self.c, &self.db, false, &mut budget)?;
        if after > self.ceiling {
            return Err(error(
                "actual SQLite allocation exceeded fixed carrier capacity",
            ));
        }
        budget.before().map_err(io)
    }
}
impl FinancialStage for Stage<'_> {
    fn read_budgeted(
        &mut self,
        digest: &NodeDigest,
        job: &mut Job,
        deadline: Option<std::time::Instant>,
    ) -> Result<NodeRead, CodecError> {
        read_any_metered(self.c, digest, &mut ReadBudget::new(Some(job), deadline))
    }
    fn put_batch_budgeted(
        &mut self,
        job: &mut Job,
        nodes: &[(NodeDigest, u8, Vec<u8>)],
        inserted: &mut Vec<NodeDigest>,
    ) -> Result<(), CodecError> {
        self.put_batch_inner(job, nodes, inserted, None)
    }
    fn put_batch_budgeted_with_deadline(
        &mut self,
        job: &mut Job,
        nodes: &[(NodeDigest, u8, Vec<u8>)],
        inserted: &mut Vec<NodeDigest>,
        deadline: Option<std::time::Instant>,
    ) -> Result<(), CodecError> {
        self.put_batch_inner(job, nodes, inserted, deadline)
    }
    /// 闭包尝试失败时的回滚：一个事务删去本尝试写下的暂存行，使暂存集合回到尝试前。删除只用
    /// 每次写后已核的 cleanup_room 余量，不经 Job（Job 可能已停止）；同 `discard_unpublished`。
    fn discard_staged(&mut self, digests: &[NodeDigest]) -> Result<u64, CodecError> {
        if digests.is_empty() {
            return Ok(0);
        }
        format_guard(self.c)?;
        let tx = self.c.transaction().map_err(io)?;
        let removed = delete_staged_rows(&tx, &self.token, digests)?;
        tx.commit().map_err(io)?;
        if allocated(&self.db)? > self.ceiling {
            return Err(error("closure rollback exceeded fixed carrier"));
        }
        Ok(removed)
    }
    fn put_budgeted(
        &mut self,
        job: &mut Job,
        digest: NodeDigest,
        tag: u8,
        body: &[u8],
    ) -> Result<(), CodecError> {
        self.put_node_inner(job, digest, tag, body, None)
    }
    fn put_budgeted_with_deadline(
        &mut self,
        job: &mut Job,
        digest: NodeDigest,
        tag: u8,
        body: &[u8],
        deadline: Option<std::time::Instant>,
    ) -> Result<(), CodecError> {
        self.put_node_inner(job, digest, tag, body, deadline)
    }
}
struct Closure<'a> {
    c: &'a Connection,
    seen: BTreeSet<NodeDigest>,
    published_only: bool,
}
impl NodeSource for Closure<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        let result = if self.published_only {
            read_table(self.c, "content_nodes", digest)?
        } else {
            read_any(self.c, digest)?
        };
        if matches!(result, NodeRead::Found { .. }) {
            self.seen.insert(*digest);
        }
        Ok(result)
    }
}
/// Run inside the same local economic transaction as image/receipt publication.
/// The caller passes every schema-path root, including nested historical peer
/// images. Verified remnants from a killed earlier job may be reused, but only
/// when re-reached and rechecked through this operation's bound roots.
pub(super) fn publish(
    tx: &Transaction<'_>,
    refs: &[ContentRef],
    job: &mut Job,
) -> Result<(), CodecError> {
    format_guard(tx)?;
    let mut reader = Closure {
        c: tx,
        seen: BTreeSet::new(),
        published_only: false,
    };
    {
        let mut metered = s_content::job::JobNodeSource::new(job, &mut reader);
        s_content::verify_many(&mut metered, refs.iter().cloned())?;
    }
    for digest in &reader.seen {
        let staged: Option<String> = tx
            .query_row(
                "SELECT stage FROM content_staging WHERE digest=?1",
                params![&digest[..]],
                |r| r.get(0),
            )
            .optional()
            .map_err(io)?;
        if staged.is_some() {
            tx.execute("INSERT INTO content_nodes(digest,tag,body) SELECT digest,tag,body FROM content_staging WHERE digest=?1", params![&digest[..]]).map_err(io)?;
            tx.execute(
                "DELETE FROM content_staging WHERE digest=?1",
                params![&digest[..]],
            )
            .map_err(io)?;
        }
    }
    let mut published = Closure {
        c: tx,
        seen: BTreeSet::new(),
        published_only: true,
    };
    let mut metered = s_content::job::JobNodeSource::new(job, &mut published);
    s_content::verify_many(&mut metered, refs.iter().cloned())?;
    Ok(())
}

fn evidence_root(value: &serde_json::Value) -> Result<ContentRef, CodecError> {
    let fields = value
        .as_object()
        .ok_or_else(|| error("FinancialBasisEvidence is not object"))?;
    const EXPECTED: [&str; 9] = [
        "schema_revision",
        "content_format",
        "kind",
        "session_id",
        "session_generation",
        "writer_epoch",
        "pending_revision",
        "commit_ns",
        "basis_ref",
    ];
    if fields.len() != EXPECTED.len()
        || fields.keys().any(|k| !EXPECTED.contains(&k.as_str()))
        || value["schema_revision"] != "s-financial-evidence/1"
        || value["content_format"] != s_content::CONTENT_FORMAT
        || value["kind"] != "FinancialBasisEvidence"
        || value["pending_revision"] != false
    {
        return Err(error(
            "new economic image has invalid FinancialBasisEvidence",
        ));
    }
    let reference = ContentRef::from_transport_json(&value["basis_ref"])?;
    if reference.kind != s_content::Kind::Object {
        return Err(error("FinancialBasisEvidence basis_ref is not object"));
    }
    Ok(reference)
}
/// Enumerate only the schema paths sanctioned by C5.7. Peer images are walked
/// as captured, without changing their bytes or searching arbitrary field names.
fn image_evidence(image: &serde_json::Value) -> Result<Vec<&serde_json::Value>, CodecError> {
    let mut evidence = Vec::new();
    let mut images = vec![image];
    while let Some(i) = images.pop() {
        let history = i["history"]
            .as_array()
            .ok_or_else(|| error("economic history is not array"))?;
        for event in history {
            if let Some(witness) = event.get("witness") {
                if let Some(structure) = witness.get("structure") {
                    evidence.push(structure);
                }
                if let Some(proofs) = witness.get("prepared_proofs") {
                    for proof in proofs
                        .as_array()
                        .ok_or_else(|| error("prepared_proofs is not array"))?
                    {
                        images.push(
                            proof
                                .get("image")
                                .ok_or_else(|| error("prepared proof image missing"))?,
                        );
                        images.push(
                            proof
                                .get("current_image")
                                .ok_or_else(|| error("prepared proof current_image missing"))?,
                        );
                    }
                }
                if let Some(proof) = witness.get("grant_proof") {
                    images.push(
                        proof
                            .get("image")
                            .ok_or_else(|| error("grant proof image missing"))?,
                    );
                    images.push(
                        proof
                            .get("current_image")
                            .ok_or_else(|| error("grant proof current_image missing"))?,
                    );
                }
            }
            if let Some(structure) = event
                .get("record")
                .and_then(|r| r.get("structure_evidence"))
            {
                evidence.push(structure);
            }
        }
        if let Some(finance) = i.get("finance") {
            for decision in finance["decisions"]
                .as_array()
                .ok_or_else(|| error("finance decisions is not array"))?
            {
                if let Some(structure) = decision.get("structure_evidence") {
                    evidence.push(structure);
                }
            }
        }
        for record in i["recorded"]
            .as_array()
            .ok_or_else(|| error("recorded is not array"))?
        {
            for key in [
                "e_commit_proof",
                "e_resolution_proof",
                "b_source_debit_proof",
            ] {
                if let Some(proof) = record.get(key) {
                    images.push(
                        proof
                            .get("image")
                            .ok_or_else(|| error("E commit image missing"))?,
                    );
                    if key == "e_resolution_proof" || key == "b_source_debit_proof" {
                        images.push(
                            proof
                                .get("previous_image")
                                .ok_or_else(|| error("E resolution previous image missing"))?,
                        );
                    }
                }
            }
        }
    }
    Ok(evidence)
}
pub(super) fn image_refs(image: &serde_json::Value) -> Result<Vec<ContentRef>, CodecError> {
    image_evidence(image)?
        .into_iter()
        .map(evidence_root)
        .collect()
}
/// Offline closure audit reads only this owner's immutable table. It never
/// asks S or a peer, and a missing nested historical node rejects recovery.
pub(super) fn verify_image(
    c: &Connection,
    image: &serde_json::Value,
    structure: &serde_json::Value,
    job: &mut Job,
) -> Result<(), CodecError> {
    verify_image_with_budget(c, image, structure, &mut ReadBudget::new(Some(job), None))
}
pub(super) fn verify_image_with_budget(
    c: &Connection,
    image: &serde_json::Value,
    structure: &serde_json::Value,
    budget: &mut ReadBudget<'_>,
) -> Result<(), CodecError> {
    format_guard_metered(c, budget)?;
    let evidence = image_evidence(image)?;
    let refs: Vec<ContentRef> = evidence
        .iter()
        .map(|e| evidence_root(e))
        .collect::<Result<_, _>>()?;
    if refs.is_empty() {
        return Ok(());
    }
    struct AuditSource<'a, 'b> {
        c: &'a Connection,
        budget: &'a mut ReadBudget<'b>,
    }
    impl NodeSource for AuditSource<'_, '_> {
        fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
            let node = read_table_metered(self.c, "content_nodes", digest, self.budget)?;
            if let NodeRead::Found { tag, body } = &node {
                self.budget
                    .job
                    .as_deref_mut()
                    .ok_or_else(|| error("audit Job missing"))?
                    .accept_node(digest, *tag, body)?;
            }
            Ok(node)
        }
    }
    s_content::verify_many(
        &mut AuditSource {
            c,
            budget: &mut *budget,
        },
        refs,
    )?;
    // The codec proves anonymous reachable nodes. Each actual six-family row
    // visit is separately checked and charged by the financial validator.
    for item in evidence {
        let deadline = budget.deadline;
        let mut read = |digest: &NodeDigest, job: &mut Job| {
            read_table_metered(
                c,
                "content_nodes",
                digest,
                &mut ReadBudget::new(Some(job), deadline),
            )
        };
        let job = budget
            .job
            .as_deref_mut()
            .ok_or_else(|| error("audit Job missing"))?;
        super::financial_content::verify_evidence_with_read(structure, item, &mut read, job, None)
            .map_err(error)?;
    }
    budget.before().map_err(error)
}

/// Immutable published-node source for recovery, replay and current evidence.
pub(super) struct PublishedSource<'a> {
    pub c: &'a Connection,
}
impl NodeSource for PublishedSource<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        read_table(self.c, "content_nodes", digest)
    }
}
/// Current owner transaction sees its own verified staging rows before the
/// final atomic move; replay and recovery use PublishedSource instead.
pub(super) struct LocalSource<'a> {
    pub c: &'a Connection,
}
impl NodeSource for LocalSource<'_> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        read_any(self.c, digest)
    }
}
/// #1394 第三问选项 A：peer 导入每块节点数（只决定写入分组；读取、计费与核验次序不变）。
pub(super) const PEER_IMPORT_BATCH_NODES: usize = 4096;
/// Import a peer's entire closed historical graph while its read transaction
/// still pins the peer image/epoch/receipt. Every actually read node is checked
/// and charged to the caller's one Job before it is staged locally.
/// #1394 第三问选项 A：写入改为每 `PEER_IMPORT_BATCH_NODES` 个节点一块（`put_batch_budgeted`，
/// 一块一个事务）；任何失败由调用方 `discard_unpublished` 清掉本操作全部暂存，同逐节点版本。
pub(super) fn import_peer_refs(
    peer: &Connection,
    refs: &[ContentRef],
    stage: &mut Stage<'_>,
    job: &mut Job,
) -> Result<(), CodecError> {
    import_peer_refs_with_deadline(peer, refs, stage, job, None)
}
pub(super) fn import_peer_refs_with_deadline(
    peer: &Connection,
    refs: &[ContentRef],
    stage: &mut Stage<'_>,
    job: &mut Job,
    deadline: Option<std::time::Instant>,
) -> Result<(), CodecError> {
    struct Import<'a, 'b> {
        peer: &'a Connection,
        stage: &'a mut Stage<'b>,
        job: &'a mut Job,
        deadline: Option<std::time::Instant>,
        pending: Vec<(NodeDigest, u8, Vec<u8>)>,
    }
    impl Import<'_, '_> {
        fn flush(&mut self) -> Result<(), CodecError> {
            if self.pending.is_empty() {
                return Ok(());
            }
            #[cfg(test)]
            {
                self.deadline = tb03c_c5_stage_tests::before_peer_flush(self.deadline);
            }
            let nodes = std::mem::take(&mut self.pending);
            // peer 导入失败由调用方 `discard_unpublished` 清掉本操作全部暂存，不用逐块回滚清单。
            let mut written = Vec::new();
            self.stage.put_batch_budgeted_with_deadline(
                self.job,
                &nodes,
                &mut written,
                self.deadline,
            )
        }
    }
    impl NodeSource for Import<'_, '_> {
        fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
            let result = read_table_metered(
                self.peer,
                "content_nodes",
                digest,
                &mut ReadBudget::new(Some(&mut *self.job), self.deadline),
            )?;
            if let NodeRead::Found { tag, body } = &result {
                self.job.accept_node(digest, *tag, body)?;
                self.pending.push((*digest, *tag, body.clone()));
                if self.pending.len() >= PEER_IMPORT_BATCH_NODES {
                    self.flush()?;
                }
            }
            Ok(result)
        }
    }
    let mut importer = Import {
        peer,
        stage,
        job,
        deadline,
        pending: Vec::new(),
    };
    s_content::verify_many(&mut importer, refs.iter().cloned())?;
    importer.flush()
}

#[cfg(test)]
#[path = "tb03c_c5_stage_tests.rs"]
mod tb03c_c5_stage_tests;
/// Reserve the entire remaining physical envelope before the final economic
/// transaction; it covers the WAL append and eventual checkpoint growth.
pub(super) fn reserve_final(
    c: &Connection,
    db: &Path,
    job: &mut Job,
) -> Result<(crate::s_content::job::Grant, u64, u64), CodecError> {
    let ceiling = configure(c, db, job.snapshot().limits)?;
    cleanup_room(c, db, true)?;
    charged_checkpoint(c, db, job, ceiling, false)?;
    cleanup_room(c, db, true)?;
    let before = allocated(db)?;
    let worst = ceiling
        .checked_sub(before)
        .ok_or_else(|| error("physical allocation above final ceiling"))?;
    let grant = job.reserve_store(worst)?;
    Ok((grant, before, ceiling))
}
/// Called after a successful COMMIT. The write was already covered by the
/// reservation; actual is measured rather than inferred from body bytes.
pub(super) fn reconcile_final(
    c: &Connection,
    db: &Path,
    job: &mut Job,
    grant: &crate::s_content::job::Grant,
    before: u64,
    ceiling: u64,
) -> Result<u64, CodecError> {
    let check = checkpoint(c);
    let after = allocated(db)?;
    let actual = after.saturating_sub(before);
    job.reconcile(grant, actual)?;
    check?;
    wal_empty(db)?;
    if after > ceiling {
        return Err(error(
            "post-commit physical allocation exceeded fixed carrier",
        ));
    }
    Ok(actual)
}

/// Drop only unreachable staging rows after this owner has already audited all
/// committed image roots against `content_nodes` under its single-writer lock.
/// This cleanup deliberately does not consult a stopped Job or delete a
/// published node. The fixed HFS carrier remains the prepaid hard envelope.
pub(super) fn discard_unpublished(
    c: &mut Connection,
    db: &Path,
) -> Result<(i64, u64, u64), CodecError> {
    format_guard(c)?;
    let ceiling = carrier_capacity(db)?;
    let before = allocated(db)?;
    let tx = c.transaction().map_err(io)?;
    let removed = discard_in_transaction(&tx)?;
    tx.commit().map_err(io)?;
    let after = allocated(db)?;
    if after > ceiling {
        return Err(error("failed-job cleanup exceeded fixed carrier"));
    }
    Ok((removed, before, after))
}
/// Recovery calls this only after its independent full published-history audit.
pub(super) fn discard_in_transaction(tx: &Transaction<'_>) -> Result<i64, CodecError> {
    format_guard(tx)?;
    let count: i64 = tx
        .query_row("SELECT count(*) FROM content_staging", [], |r| r.get(0))
        .map_err(io)?;
    if count > 0 {
        tx.execute("DELETE FROM content_staging", []).map_err(io)?;
    }
    Ok(count)
}

/// #1394 第三问选项 A：批量写的事务语义锁（普通文件 SQLite；物理载体判据另由 HFS 载体实测）。
#[cfg(test)]
mod batch_tests {
    use super::*;

    fn fresh(tag: &str) -> (PathBuf, Connection) {
        let dir = std::env::temp_dir().join(format!(
            "eb-closure-batch-{}-{tag}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("eb.sqlite");
        let c = Connection::open(&db).unwrap();
        c.execute_batch("CREATE TABLE meta(k TEXT PRIMARY KEY,v TEXT NOT NULL) STRICT;")
            .unwrap();
        c.execute_batch(super::super::store::CONTENT_SQL).unwrap();
        c.execute("INSERT INTO meta VALUES('schema_revision',?1)", [SCHEMA_V2])
            .unwrap();
        c.execute(
            "INSERT INTO content_meta VALUES(1,?1)",
            [s_content::CONTENT_FORMAT],
        )
        .unwrap();
        (dir, c)
    }
    fn node(i: u8) -> (NodeDigest, u8, Vec<u8>) {
        let mut store = s_content::MemoryNodes::new();
        let r = s_content::write_value(&mut store, &s_content::Value::string(format!("n-{i}")))
            .unwrap()
            .content_ref();
        let d = r.digest().unwrap();
        let (t, b) = store.get(&d).unwrap();
        (d, t, b.to_vec())
    }
    fn rows(c: &Connection, table: &str) -> i64 {
        c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn missing_set_is_sorted_deduped_and_conflicts_write_nothing() {
        let (dir, c) = fresh("missing");
        let nodes: Vec<_> = (0..6).map(node).collect();
        let (d0, t0, b0) = nodes[0].clone();
        c.execute(
            "INSERT INTO content_nodes VALUES(?1,?2,?3)",
            params![&d0[..], i64::from(t0), b0],
        )
        .unwrap();
        let mut batch = nodes.clone();
        batch.push(nodes[3].clone());
        let missing = batch_missing(&c, &batch).unwrap();
        let digests: Vec<NodeDigest> = missing.iter().map(|n| n.0).collect();
        let mut want: Vec<NodeDigest> = nodes[1..].iter().map(|n| n.0).collect();
        want.sort();
        assert_eq!(digests, want, "已公开者跳过、块内重复去重、按摘要升序");
        let mut bad = nodes.clone();
        bad[2].2.push(b'x');
        assert!(batch_missing(&c, &bad).is_err(), "F 不符");
        let mut twin = nodes.clone();
        let (d, t, mut b) = nodes[4].clone();
        b.push(b'y');
        twin.push((d, t, b));
        assert!(batch_missing(&c, &twin).is_err(), "块内同根不同字节");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_chunk_is_one_transaction_and_rollback_deletes_only_this_attempt() {
        let (dir, mut c) = fresh("tx");
        let nodes: Vec<_> = (10..18).map(node).collect();
        // 他 token 的暂存行与公开行都不受回滚影响。
        let (dp, tp, bp) = nodes[0].clone();
        c.execute(
            "INSERT INTO content_nodes VALUES(?1,?2,?3)",
            params![&dp[..], i64::from(tp), bp],
        )
        .unwrap();
        let (dq, tq, bq) = nodes[1].clone();
        c.execute(
            "INSERT INTO content_staging VALUES(?1,?2,?3,'stg-other')",
            params![&dq[..], i64::from(tq), bq],
        )
        .unwrap();
        // 块内一行在写入时撞主键（模拟写入期失败）：整块回滚，一行都不留。
        let chunk = batch_missing(&c, &nodes[2..6]).unwrap();
        let (dc, tc, bc) = nodes[5].clone();
        c.execute(
            "INSERT INTO content_staging VALUES(?1,?2,?3,'stg-other')",
            params![&dc[..], i64::from(tc), bc],
        )
        .unwrap();
        {
            let tx = c.transaction().unwrap();
            assert!(insert_staging_batch(&tx, "stg-me", &chunk).is_err());
        }
        assert_eq!(rows(&c, "content_staging"), 2, "失败块一行不留");
        c.execute(
            "DELETE FROM content_staging WHERE digest=?1",
            params![&dc[..]],
        )
        .unwrap();
        let tx = c.transaction().unwrap();
        let inserted = insert_staging_batch(&tx, "stg-me", &chunk).unwrap();
        tx.commit().unwrap();
        assert_eq!(inserted.len(), 4);
        assert_eq!(rows(&c, "content_staging"), 5);
        let tx = c.transaction().unwrap();
        let mut attempt = inserted.clone();
        attempt.push(dp);
        attempt.push(dq);
        assert_eq!(delete_staged_rows(&tx, "stg-me", &attempt).unwrap(), 4);
        tx.commit().unwrap();
        assert_eq!(rows(&c, "content_staging"), 1, "他 token 的暂存仍在");
        assert_eq!(rows(&c, "content_nodes"), 1, "公开行不动");
        drop(c);
        std::fs::remove_dir_all(dir).unwrap();
    }
    /// #1406 C4a：Resolve 两个历史图像中的真实内容节点都须在本 owner 闭包。
    /// 此锁验证恢复所调用的闭包读取层；业务 FIN 证据语义由既有 verify_evidence 锁另核。
    #[test]
    fn resolution_proof_content_closure_reopens_locally_and_rejects_missing_node() {
        use serde_json::json;
        let (dir, c) = fresh("resolution-closure");
        let mut nodes = s_content::MemoryNodes::new();
        let mut refs = Vec::new();
        for label in ["capture", "resolve", "resolve-previous"] {
            refs.push(
                s_content::write_value(
                    &mut nodes,
                    &s_content::Value::Object(vec![(
                        "history".into(),
                        s_content::Value::string(label.to_owned()),
                    )]),
                )
                .unwrap()
                .content_ref(),
            );
        }
        for (digest, tag, body) in nodes.iter() {
            c.execute(
                "INSERT INTO content_nodes VALUES(?1,?2,?3)",
                params![&digest[..], i64::from(tag), body],
            )
            .unwrap();
        }
        let peer_image = |reference: &ContentRef| json!({"recorded":[],"history":[{"record":{"structure_evidence":{"schema_revision":"s-financial-evidence/1","content_format":s_content::CONTENT_FORMAT,"kind":"FinancialBasisEvidence","session_id":"closure-fixture","session_generation":"1","writer_epoch":"1","pending_revision":false,"commit_ns":"1","basis_ref":reference.to_transport_json()}}}]});
        let image = json!({"history":[],"recorded":[{"e_commit_proof":{"image":peer_image(&refs[0])},"e_resolution_proof":{"image":peer_image(&refs[1]),"previous_image":peer_image(&refs[2])}}]});
        let observed = image_refs(&image).unwrap();
        assert_eq!(observed.len(), 3);
        for reference in &refs {
            assert!(observed.contains(reference));
        }
        drop(c);
        let reopened = Connection::open(dir.join("eb.sqlite")).unwrap();
        s_content::verify_many(&mut PublishedSource { c: &reopened }, observed.clone()).unwrap();
        let missing = refs[2].digest().unwrap();
        assert_eq!(
            reopened
                .execute(
                    "DELETE FROM content_nodes WHERE digest=?1",
                    params![&missing[..]]
                )
                .unwrap(),
            1
        );
        assert!(s_content::verify_many(&mut PublishedSource { c: &reopened }, observed).is_err());
        let mut malformed = image.clone();
        malformed["recorded"][0]["e_resolution_proof"]
            .as_object_mut()
            .unwrap()
            .remove("previous_image");
        assert!(image_refs(&malformed).is_err());
        drop(reopened);
        std::fs::remove_dir_all(dir).unwrap();
    }
    /// #1406 C5：目标必须自持源扣账前后两份图像的真实闭包；缺任一份禁止恢复。
    #[test]
    fn transfer_source_proof_closure_reopens_and_rejects_missing_previous_node() {
        use serde_json::json;
        let (dir, c) = fresh("transfer-source-closure");
        let mut nodes = s_content::MemoryNodes::new();
        let refs: Vec<_> = ["source-applied", "source-before"]
            .into_iter()
            .map(|label| {
                s_content::write_value(
                    &mut nodes,
                    &s_content::Value::Object(vec![(
                        "history".into(),
                        s_content::Value::string(label.to_owned()),
                    )]),
                )
                .unwrap()
                .content_ref()
            })
            .collect();
        for (digest, tag, body) in nodes.iter() {
            c.execute(
                "INSERT INTO content_nodes VALUES(?1,?2,?3)",
                params![&digest[..], i64::from(tag), body],
            )
            .unwrap();
        }
        let peer = |r: &ContentRef| json!({"recorded":[],"history":[{"record":{"structure_evidence":{"schema_revision":"s-financial-evidence/1","content_format":s_content::CONTENT_FORMAT,"kind":"FinancialBasisEvidence","session_id":"transfer-closure","session_generation":"1","writer_epoch":"1","pending_revision":false,"commit_ns":"1","basis_ref":r.to_transport_json()}}}]});
        let image = json!({"history":[],"recorded":[{"b_source_debit_proof":{"image":peer(&refs[0]),"previous_image":peer(&refs[1])}}]});
        let observed = image_refs(&image).unwrap();
        assert_eq!(observed.len(), 2);
        for r in &refs {
            assert!(observed.contains(r));
        }
        drop(c);
        let reopened = Connection::open(dir.join("eb.sqlite")).unwrap();
        s_content::verify_many(&mut PublishedSource { c: &reopened }, observed.clone()).unwrap();
        let missing = refs[1].digest().unwrap();
        reopened
            .execute(
                "DELETE FROM content_nodes WHERE digest=?1",
                params![&missing[..]],
            )
            .unwrap();
        assert!(s_content::verify_many(&mut PublishedSource { c: &reopened }, observed).is_err());
        let mut malformed = image;
        malformed["recorded"][0]["b_source_debit_proof"]
            .as_object_mut()
            .unwrap()
            .remove("previous_image");
        assert!(image_refs(&malformed).is_err());
        drop(reopened);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
