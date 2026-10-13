//! #1394 C5.7: immutable, owner-local S content for economic-session/2.
//! The success domain is a dedicated fixed-capacity HFS+ volume per owner.
//! Kernel ENOSPC bounds all allocation on that volume; the DB/WAL/SHM files
//! are still measured with st_blocks for actual Job reconciliation. A node's
//! framed bytes or SQLite's logical page count never stand in for disk use.
use super::financial_content::FinancialStage;
use super::store::SCHEMA_V2;
use crate::s_content::{
    self, code,
    job::{Existing, Job, JobLimits},
    CodecError, ContentRef, NodeDigest, NodeRead, NodeSource,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::collections::BTreeSet;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

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
    let pages = u128::from(u64::try_from(pragma_i64(c, "max_page_count")?).map_err(io)?);
    let size = u128::from(u64::try_from(pragma_i64(c, "page_size")?).map_err(io)?);
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
    let revision: String = c
        .query_row("SELECT v FROM meta WHERE k='schema_revision'", [], |r| {
            r.get(0)
        })
        .map_err(io)?;
    let format: String = c
        .query_row(
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
    let (busy, _, _): (i64, i64, i64) = c
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
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
    let before = allocated(db)?;
    let worst = ceiling
        .checked_sub(before)
        .ok_or_else(|| error("allocation above checkpoint ceiling"))?;
    let grant = if staged {
        job.reserve_staged(worst)?
    } else {
        job.reserve_store(worst)?
    };
    let result = checkpoint(c);
    let after = allocated(db)?;
    job.reconcile(&grant, after.saturating_sub(before))?;
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
    let sql = match table {
        "content_nodes" => "SELECT tag,body FROM content_nodes WHERE digest=?1",
        "content_staging" => "SELECT tag,body FROM content_staging WHERE digest=?1",
        _ => return Err(error("unknown content table")),
    };
    let row: Option<(i64, Vec<u8>)> = c
        .query_row(sql, params![&digest[..]], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()
        .map_err(io)?;
    match row {
        Some((tag, body)) => checked(digest, tag, body),
        None => Ok(NodeRead::Missing),
    }
}
fn read_any(c: &Connection, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
    let a = read_table(c, "content_nodes", digest)?;
    let b = read_table(c, "content_staging", digest)?;
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
fn batch_missing<'n>(
    c: &Connection,
    nodes: &'n [(NodeDigest, u8, Vec<u8>)],
) -> Result<Vec<&'n (NodeDigest, u8, Vec<u8>)>, CodecError> {
    let mut chosen: std::collections::BTreeMap<NodeDigest, &'n (NodeDigest, u8, Vec<u8>)> =
        std::collections::BTreeMap::new();
    for node in nodes {
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
        match read_any(c, digest)? {
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
fn insert_staging_batch(
    tx: &Transaction<'_>,
    token: &str,
    nodes: &[&(NodeDigest, u8, Vec<u8>)],
) -> Result<Vec<NodeDigest>, CodecError> {
    let mut insert = tx
        .prepare_cached("INSERT INTO content_staging(digest,tag,body,stage)VALUES(?1,?2,?3,?4)")
        .map_err(io)?;
    let mut inserted = Vec::with_capacity(nodes.len());
    for (digest, tag, body) in nodes.iter().copied() {
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
        removed += u64::try_from(delete.execute(params![&digest[..], token]).map_err(io)?)
            .map_err(io)?;
    }
    Ok(removed)
}
impl FinancialStage for Stage<'_> {
    /// 与 `put_budgeted` 同一组物理判据（cleanup_room、charged_checkpoint、整余量预留、写后对账、
    /// TRUNCATE checkpoint、载体上限），只把逐节点的三次持久动作合成每块一次：一次预留、一个事务、
    /// 一次 checkpoint。块内全部已在本地则不预留、不写。
    /// r2（独审 N1）：块事务一提交，本块实际插入的摘要立即追加到 `inserted`，然后才做 checkpoint、
    /// 对账、WAL 与载体上限这些可能失败的提交后步骤；只追加本块插入的行，块里已在本地的节点不追加。
    fn put_batch_budgeted(
        &mut self,
        job: &mut Job,
        nodes: &[(NodeDigest, u8, Vec<u8>)],
        inserted: &mut Vec<NodeDigest>,
    ) -> Result<(), CodecError> {
        job.pre_io_check()?;
        let missing = batch_missing(self.c, nodes)?;
        if missing.is_empty() {
            return Ok(());
        }
        cleanup_room(self.c, &self.db, true)?;
        charged_checkpoint(self.c, &self.db, job, self.ceiling, true)?;
        cleanup_room(self.c, &self.db, true)?;
        let before = allocated(&self.db)?;
        let worst = self
            .ceiling
            .checked_sub(before)
            .ok_or_else(|| error("physical allocation above ceiling"))?;
        let reservation = job.reserve_staged(worst)?;
        let token = self.token.clone();
        let written = (|| -> Result<Vec<NodeDigest>, CodecError> {
            let tx = self.c.transaction().map_err(io)?;
            let rows = insert_staging_batch(&tx, &token, &missing)?;
            tx.commit().map_err(io)?;
            Ok(rows)
        })();
        match written {
            // 已提交：先进调用方的回滚清单，再做任何可能失败的提交后步骤（N1）。
            Ok(rows) => inserted.extend(rows),
            Err(e) => {
                // 未提交的块已随事务回滚；物理分配照实对账。
                let after = allocated(&self.db)?;
                job.reconcile(&reservation, after.saturating_sub(before))?;
                return Err(e);
            }
        }
        let check = checkpoint(self.c);
        let after = allocated(&self.db)?;
        job.reconcile(&reservation, after.saturating_sub(before))?;
        check?;
        wal_empty(&self.db)?;
        cleanup_room(self.c, &self.db, false)?;
        if after > self.ceiling {
            return Err(error(
                "actual SQLite allocation exceeded fixed carrier capacity",
            ));
        }
        Ok(())
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
        job.pre_io_check()?;
        if s_content::frame(tag, body) != digest {
            return Err(error("node digest mismatch"));
        }
        if let NodeRead::Found {
            tag: old_tag,
            body: old_body,
        } = read_any(self.c, &digest)?
        {
            if old_tag != tag || old_body != body {
                return Err(error("same digest conflict"));
            }
            return Ok(());
        }
        cleanup_room(self.c, &self.db, true)?;
        charged_checkpoint(self.c, &self.db, job, self.ceiling, true)?;
        cleanup_room(self.c, &self.db, true)?;
        let before = allocated(&self.db)?;
        let worst = self
            .ceiling
            .checked_sub(before)
            .ok_or_else(|| error("physical allocation above ceiling"))?;
        let reservation = job.reserve_staged(worst)?;
        let inserted = self.c.execute(
            "INSERT INTO content_staging(digest,tag,body,stage)VALUES(?1,?2,?3,?4)",
            params![&digest[..], i64::from(tag), body, &self.token],
        );
        if let Err(e) = inserted {
            let after = allocated(&self.db)?;
            job.reconcile(&reservation, after.saturating_sub(before))?;
            return Err(io(e));
        }
        // Checkpoint here makes the next write start from an empty WAL. The
        // maximum physical pages were already reserved before the insertion.
        // This second checkpoint was itself covered by the same reservation:
        // before the write the WAL was empty and no second writer can enter
        // the owner process lock. Its full peak is <= the global envelope.
        let check = checkpoint(self.c);
        let after = allocated(&self.db)?;
        job.reconcile(&reservation, after.saturating_sub(before))?;
        check?;
        wal_empty(&self.db)?;
        cleanup_room(self.c, &self.db, false)?;
        if after > self.ceiling {
            return Err(error(
                "actual SQLite allocation exceeded fixed carrier capacity",
            ));
        }
        Ok(())
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
            if let Some(proof) = record.get("e_commit_proof") {
                images.push(
                    proof
                        .get("image")
                        .ok_or_else(|| error("E commit image missing"))?,
                );
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
    format_guard(c)?;
    let evidence = image_evidence(image)?;
    let refs: Vec<ContentRef> = evidence
        .iter()
        .map(|e| evidence_root(e))
        .collect::<Result<_, _>>()?;
    if refs.is_empty() {
        return Ok(());
    }
    let mut source = Closure {
        c,
        seen: BTreeSet::new(),
        published_only: true,
    };
    let mut metered = s_content::job::JobNodeSource::new(job, &mut source);
    s_content::verify_many(&mut metered, refs)?;
    // The codec proves anonymous reachable nodes. Each actual six-family row
    // visit is separately checked and charged by the financial validator.
    for item in evidence {
        let mut local = PublishedSource { c };
        super::financial_content::verify_evidence(structure, item, &mut local, job, None)
            .map_err(error)?;
    }
    Ok(())
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
    struct Import<'a, 'b> {
        peer: &'a Connection,
        stage: &'a mut Stage<'b>,
        job: &'a mut Job,
        pending: Vec<(NodeDigest, u8, Vec<u8>)>,
    }
    impl Import<'_, '_> {
        fn flush(&mut self) -> Result<(), CodecError> {
            if self.pending.is_empty() {
                return Ok(());
            }
            let nodes = std::mem::take(&mut self.pending);
            // peer 导入失败由调用方 `discard_unpublished` 清掉本操作全部暂存，不用逐块回滚清单。
            let mut written = Vec::new();
            self.stage.put_batch_budgeted(self.job, &nodes, &mut written)
        }
    }
    impl NodeSource for Import<'_, '_> {
        fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
            self.job.pre_io_check()?;
            let result = read_table(self.peer, "content_nodes", digest)?;
            if let NodeRead::Found { tag, body } = &result {
                let framed = u64::try_from(body.len())
                    .map_err(io)?
                    .checked_add(
                        u64::try_from(s_content::DOMAIN_SEPARATOR.len()).map_err(io)? + 1 + 8,
                    )
                    .ok_or_else(|| error("peer frame overflow"))?;
                self.job.charge_wire_bytes(framed)?;
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
        pending: Vec::new(),
    };
    s_content::verify_many(&mut importer, refs.iter().cloned())?;
    importer.flush()
}
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
        c.execute("DELETE FROM content_staging WHERE digest=?1", params![&dc[..]])
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
}
