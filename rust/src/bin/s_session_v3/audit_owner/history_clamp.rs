//! #1481 U-A：既有 AuditConnection 的观测值，不是第二个连接 owner。
//! 显式组件入口才启用。旧路径不增加前置；这里不签发历史认证。
use super::{effects, native, request_audit::SqlCell, AuditTxn};
use crate::v3::audit_store::{CodecRead, NativeAdmission};
use newchan_rust::s_content::{capture_allocation::scratch::WorkScope, CodecVec};
use rusqlite::{types::ValueRef, Connection};
use std::{cell::RefCell, rc::Rc};
use std::{
    fs::{File, Metadata},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

pub(super) fn stale(message: impl std::fmt::Display) -> String {
    format!("HistoricalAuditStale：{message}")
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    dev: u64,
    ino: u64,
    len: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
impl Stamp {
    fn read(m: Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            len: m.len(),
            modified: (m.mtime(), m.mtime_nsec()),
            changed: (m.ctime(), m.ctime_nsec()),
        }
    }
    fn same_file(&self, other: &Self) -> bool {
        (self.dev, self.ino) == (other.dev, other.ino)
    }
}
struct WatchedFile {
    path: PathBuf,
    file: File,
}
impl WatchedFile {
    fn open(path: &Path) -> Result<Self, String> {
        Ok(Self {
            path: path.to_owned(),
            file: File::open(path).map_err(stale)?,
        })
    }
    fn stamp(&self) -> Result<Stamp, String> {
        let held = Stamp::read(self.file.metadata().map_err(stale)?);
        let named = Stamp::read(std::fs::metadata(&self.path).map_err(stale)?);
        if held != named {
            return Err(stale("路径与持有文件身份/stat不一致"));
        }
        Ok(held)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Files {
    main: Stamp,
    wal: Stamp,
    wal_header: Vec<u8>,
}
#[derive(Debug, PartialEq, Eq)]
struct Controls([CodecVec<(i64, CodecVec<u8>)>; 2]);
#[derive(Debug, PartialEq, Eq)]
struct Observation {
    data_version: i64,
    schema_version: i64,
    domain: [i64; 6],
    total_changes: u64,
    controls: Controls,
    files: Files,
}
/// 只有 owner 的 effects 完整覆盖与原语义核均成功才保留此候选。
/// 候选仍未提交；自身写后没有候选的连接不能推进夹持基线。
pub(super) struct WriteCandidate {
    observation: Observation,
}
#[path = "history_physical.rs"]
mod physical;
pub(super) use physical::CommitMonitor;
pub(super) struct Clamp {
    main: WatchedFile,
    wal: WatchedFile,
    baseline: Option<Observation>,
    parent: Option<native::Budget>,
    control_limit: usize,
    checkpoint_threshold: i32,
    progress: physical::Progress,
}
impl Clamp {
    pub(super) fn open(
        connection: &Connection,
        path: &Path,
        control_limit: usize,
    ) -> Result<Self, String> {
        Self::open_in_parent(connection, path, control_limit, None)
    }
    pub(super) fn open_in_parent(
        connection: &Connection,
        path: &Path,
        control_limit: usize,
        parent: Option<native::Budget>,
    ) -> Result<Self, String> {
        if let Some(parent) = &parent {
            parent
                .check_current()
                .map_err(|_| stale("控制父账已过期"))?;
        }
        let mode: String = connection
            .query_row("PRAGMA main.journal_mode", [], |r| r.get(0))
            .map_err(stale)?;
        if mode != "wal" {
            return Err(stale("H1组件要求既有WAL域，不自动改变journal_mode"));
        }
        let mut wal = path.as_os_str().to_os_string();
        wal.push("-wal");
        let wal = PathBuf::from(wal);
        let checkpoint_threshold = connection
            .query_row("PRAGMA wal_autocheckpoint", [], |r| r.get(0))
            .map_err(stale)?;
        let result = Self {
            main: WatchedFile::open(path)?,
            wal: WatchedFile::open(&wal)?,
            baseline: None,
            parent,
            control_limit,
            checkpoint_threshold,
            progress: physical::initial_progress(connection)?,
        };
        connection.wal_hook(Some(physical::observe_commit));
        Ok(result)
    }
    pub(super) fn rebind_parent(&mut self, parent: &native::Budget) -> Result<(), String> {
        let old = self
            .parent
            .as_ref()
            .ok_or_else(|| stale("控制夹持缺原父账"))?;
        if !old.same_parent(parent) {
            return Err(stale("控制夹持不能换父账"));
        }
        parent
            .check_current()
            .map_err(|_| stale("控制夹持新冻结域无效"))?;
        self.parent = Some(parent.clone());
        Ok(())
    }
    pub(super) fn parent(&self) -> Option<&native::Budget> {
        self.parent.as_ref()
    }
    pub(super) fn retained_bytes(&self) -> usize {
        if self.parent.is_none() {
            return 0;
        }
        self.baseline
            .as_ref()
            .map_or(0, |b| b.controls.retained_bytes())
    }
    pub(super) fn retire(&mut self) {
        self.baseline = None;
        self.parent = None;
    }
    fn files(&self) -> Result<Files, String> {
        use std::os::unix::fs::FileExt;
        let main = self.main.stamp()?;
        let wal = self.wal.stamp()?;
        let mut header = vec![0; (wal.len.min(32)) as usize];
        if !header.is_empty() {
            self.wal.file.read_exact_at(&mut header, 0).map_err(stale)?;
        }
        if main != self.main.stamp()? || wal != self.wal.stamp()? {
            return Err(stale("文件观测期间发生变化"));
        }
        Ok(Files {
            main,
            wal,
            wal_header: header,
        })
    }
    fn observe(&self, active: &AuditTxn<'_>) -> Result<Observation, String> {
        let before = self.files()?;
        let out = Observation {
            data_version: active
                .tx()
                .query_row("PRAGMA main.data_version", [], |r| r.get(0))
                .map_err(stale)?,
            schema_version: active
                .tx()
                .query_row("PRAGMA main.schema_version", [], |r| r.get(0))
                .map_err(stale)?,
            domain: active.read_domain().map_err(stale)?,
            total_changes: active.tx().total_changes(),
            controls: match &self.parent {
                Some(parent) => parent
                    .codec_work()
                    .map_err(resource_failure)?
                    .run(|work| Controls::read(active.tx(), self.control_limit, Some(work)))
                    .map_err(|error| resource_failure(native::Failure::from(error)))?,
                None => Controls::read(active.tx(), self.control_limit, None)?,
            },
            files: self.files()?,
        };
        if before != out.files {
            return Err(stale("锁内观测期间文件变化"));
        }
        Ok(out)
    }
    pub(super) fn check(&mut self, active: &AuditTxn<'_>) -> Result<(), String> {
        let now = self.observe(active)?;
        if let Some(previous) = &self.baseline {
            if previous != &now {
                return Err(stale("版本、文件、控制原件、写计数或连接域改变"));
            }
        } else {
            self.baseline = Some(now);
        }
        Ok(())
    }
    pub(super) fn candidate(
        &self,
        active: &AuditTxn<'_>,
        effects: &effects::Effects,
    ) -> Result<WriteCandidate, String> {
        if self.parent.is_some() && !effects.shared_parent {
            return Err(stale("认证自身写effects未进入同一父账"));
        }
        if !effects.has_exact_coverage(active.tx().total_changes()) {
            return Err(stale("自身写缺完整effects覆盖"));
        }
        let before = self.baseline.as_ref().ok_or_else(|| stale("缺生涯起点"))?;
        let after = self.observe(active)?;
        if before.data_version != after.data_version
            || before.schema_version != after.schema_version
            || before.domain != after.domain
            || before.total_changes > after.total_changes
        {
            return Err(stale("自身写改变版本/schema/domain"));
        }
        before.controls.verify_effects(&after.controls, effects)?;
        Self::own_files(&before.files, &after.files, self.progress.complete())?;
        if before.files != after.files && before.total_changes == after.total_changes {
            return Err(stale("无自身SQL效果却发生物理变化"));
        }
        Ok(WriteCandidate { observation: after })
    }
    fn own_files(before: &Files, after: &Files, reset_allowed: bool) -> Result<(), String> {
        if before.main != after.main
            || !before.wal.same_file(&after.wal)
            || after.wal.len < before.wal.len
        {
            return Err(stale("自身写不能解释main变化、WAL替换或截短"));
        }
        let reset = before.wal.len != 0 && before.wal_header != after.wal_header;
        if reset {
            let (old, new) = (&before.wal_header, &after.wal_header);
            let next = |offset| {
                u32::from_be_bytes(old[offset..offset + 4].try_into().unwrap()).wrapping_add(1)
                    == u32::from_be_bytes(new[offset..offset + 4].try_into().unwrap())
            };
            if !reset_allowed
                || old.len() != 32
                || new.len() != 32
                || old[..12] != new[..12]
                || !next(12)
                || !next(16)
            {
                return Err(stale("WAL轮次变化不匹配本连接已完成checkpoint收据"));
            }
        }
        if after.wal.len != 0 {
            let h = &after.wal_header;
            if h.len() != 32 {
                return Err(stale("WAL头不完整"));
            }
            let page = u32::from_be_bytes(h[8..12].try_into().unwrap()) as u64;
            if page < 512
                || page > 65536
                || !page.is_power_of_two()
                || (after.wal.len - 32) % (page + 24) != 0
            {
                return Err(stale("WAL追加不是完整frame"));
            }
        }
        Ok(())
    }
    pub(super) fn arm_commit(&self) -> Result<CommitMonitor, String> {
        CommitMonitor::begin(self)
    }
    pub(super) fn check_candidate(
        &self,
        active: &AuditTxn<'_>,
        candidate: &WriteCandidate,
    ) -> Result<(), String> {
        if self.observe(active)? != candidate.observation {
            return Err(stale("effects闭合后出现未声明变化"));
        }
        Ok(())
    }
    pub(super) fn committed(
        &mut self,
        mut candidate: WriteCandidate,
        monitor: &CommitMonitor,
    ) -> Result<(), String> {
        let after = self.files()?;
        let baseline = &self
            .baseline
            .as_ref()
            .ok_or_else(|| stale("缺生涯起点"))?
            .files;
        if let Some(receipt) = monitor.take() {
            let receipt = receipt?;
            Self::own_files(baseline, &receipt.before, self.progress.complete())?;
            if receipt.after != after
                || !receipt.before.main.same_file(&after.main)
                || receipt.before.wal != after.wal
                || receipt.before.wal_header != after.wal_header
            {
                return Err(stale("提交/checkpoint收据后出现未解释物理变化"));
            }
            let reset = baseline.wal_header != receipt.before.wal_header;
            let previous = if reset {
                physical::Progress::empty()
            } else {
                self.progress
            };
            if receipt.start.frames != receipt.frames
                || receipt.frames <= previous.frames
                || receipt.start.backfill != previous.backfill
            {
                return Err(stale("实际WAL帧/已回填前沿不能由本次提交解释"));
            }
            let h = &receipt.before.wal_header;
            let page = u32::from_be_bytes(h[8..12].try_into().unwrap()) as u64;
            let expected_len = baseline
                .wal
                .len
                .max(32 + receipt.frames as u64 * (page + 24));
            if receipt.before.wal.len != expected_len {
                return Err(stale("WAL长度不等于实际帧范围与既有保留空间"));
            }
            if receipt.end.frames != receipt.frames
                || receipt.end.backfill < receipt.start.backfill
                || (!receipt.checkpoint && receipt.end != receipt.start)
            {
                return Err(stale("checkpoint回填进度不匹配本次收据"));
            }
            if receipt.end.backfill == receipt.start.backfill && receipt.before.main != after.main {
                return Err(stale("没有自身回填进展却改变main"));
            }
            self.progress = receipt.end;
        } else {
            // 无WAL回调时也不可把effects前的变化吞进新基线。
            if baseline != &after || candidate.observation.files != after {
                return Err(stale("缺实际WAL提交收据"));
            }
        }
        candidate.observation.files = after;
        // data_version保留锁内值，下一快照必须与它相同，不以提交后重读刷新。
        self.baseline = Some(candidate.observation);
        Ok(())
    }
}

fn resource_failure(error: native::Failure) -> String {
    match error {
        native::Failure::BudgetExceeded => stale("控制原件超过当前父账额度"),
        native::Failure::Invalid(error) => stale(error),
    }
}

impl Controls {
    const SQL: [&'static str; 2] = [
        "SELECT rowid,* FROM main.meta ORDER BY rowid",
        "SELECT rowid,* FROM main.s_protocol_meta ORDER BY rowid",
    ];
    fn retained_bytes(&self) -> usize {
        self.0
            .iter()
            .map(|rows| {
                rows.capacity() * std::mem::size_of::<(i64, CodecVec<u8>)>()
                    + rows.iter().map(|(_, wire)| wire.capacity()).sum::<usize>()
            })
            .sum::<usize>()
    }
    fn read(
        connection: &Connection,
        limit: usize,
        work: Option<&WorkScope>,
    ) -> Result<Self, String> {
        let mut out = Self(Default::default());
        let admission = NativeAdmission(work);
        let mut used = 0usize;
        for (i, sql) in Self::SQL.into_iter().enumerate() {
            let mut statement = connection.prepare(sql).map_err(stale)?;
            let columns = statement.column_count();
            let mut rows = statement.query([]).map_err(stale)?;
            while let Some(row) = rows.next().map_err(stale)? {
                let mut wire = CodecVec::default();
                for column in 0..columns {
                    encode_value(
                        &mut wire,
                        row.get_ref(column).map_err(stale)?,
                        limit.saturating_sub(used),
                        work,
                    )?;
                }
                used = used
                    .checked_add(wire.len())
                    .filter(|n| *n <= limit)
                    .ok_or_else(|| stale("控制原件超过组件调用方额度"))?;
                admission
                    .native_codec_room(&mut out.0[i], 1)
                    .map_err(stale)?;
                out.0[i]
                    .try_push((row.get(0).map_err(stale)?, wire))
                    .map_err(stale)?;
            }
        }
        Ok(out)
    }
    fn verify_effects(&self, after: &Self, effects: &effects::Effects) -> Result<(), String> {
        for (index, table) in [effects::Table::Meta, effects::Table::Protocol]
            .into_iter()
            .enumerate()
        {
            let mut old = self.0[index].iter().peekable();
            let mut new = after.0[index].iter().peekable();
            // Recording::finish 按 (table, rowid, digest) 排序；控制表的 rowid 唯一。
            let mut changes = effects.rows(table).peekable();
            loop {
                let Some(rowid) = [
                    old.peek().map(|r| r.0),
                    new.peek().map(|r| r.0),
                    changes.peek().map(|c| c.rowid),
                ]
                .into_iter()
                .flatten()
                .min() else {
                    break;
                };
                let before = if old.peek().is_some_and(|r| r.0 == rowid) {
                    old.next().map(|r| &r.1)
                } else {
                    None
                };
                let actual = if new.peek().is_some_and(|r| r.0 == rowid) {
                    new.next().map(|r| &r.1)
                } else {
                    None
                };
                if changes.peek().is_some_and(|c| c.rowid == rowid) {
                    let change = changes.next().unwrap();
                    if !matches_row(before, change.old.as_deref()) {
                        return Err(stale("effects控制前像与生涯原件不符"));
                    }
                    if !matches_row(actual, change.new.as_deref()) {
                        return Err(stale("控制原件变化未由effects解释"));
                    }
                } else if before != actual {
                    return Err(stale("控制原件变化未由effects解释"));
                }
            }
        }
        Ok(())
    }
}
fn matches_row(wire: Option<&CodecVec<u8>>, cells: Option<&[SqlCell]>) -> bool {
    let (wire, cells) = match (wire, cells) {
        (None, None) => return true,
        (Some(wire), Some(cells)) => (wire, cells),
        _ => return false,
    };
    let mut offset = 0usize;
    for cell in cells {
        let value = match cell {
            SqlCell::Null => ValueRef::Null,
            SqlCell::Integer(n) => ValueRef::Integer(*n),
            SqlCell::Real(bits) => ValueRef::Real(f64::from_bits(*bits)),
            SqlCell::Text(bytes) => ValueRef::Text(bytes),
            SqlCell::Blob(bytes) => ValueRef::Blob(bytes),
        };
        let equal = with_value(value, |tag, bytes| {
            let Some(end) = offset
                .checked_add(9)
                .and_then(|n| n.checked_add(bytes.len()))
            else {
                return false;
            };
            if end > wire.len()
                || wire[offset] != tag
                || wire[offset + 1..offset + 9] != (bytes.len() as u64).to_le_bytes()
                || wire[offset + 9..end] != *bytes
            {
                return false;
            }
            offset = end;
            true
        });
        if !equal {
            return false;
        }
    }
    offset == wire.len()
}
fn with_value<T>(value: ValueRef<'_>, consume: impl FnOnce(u8, &[u8]) -> T) -> T {
    let fixed;
    let (tag, bytes): (u8, &[u8]) = match value {
        ValueRef::Null => (0, &[]),
        ValueRef::Integer(n) => {
            fixed = n.to_le_bytes();
            (1, &fixed)
        }
        ValueRef::Real(n) => {
            fixed = n.to_bits().to_le_bytes();
            (2, &fixed)
        }
        ValueRef::Text(b) => (3, b),
        ValueRef::Blob(b) => (4, b),
    };
    consume(tag, bytes)
}
fn encode_value(
    out: &mut CodecVec<u8>,
    value: ValueRef<'_>,
    limit: usize,
    work: Option<&WorkScope>,
) -> Result<(), String> {
    with_value(value, |tag, bytes| {
        let size = out
            .len()
            .checked_add(9)
            .and_then(|n| n.checked_add(bytes.len()))
            .filter(|n| *n <= limit)
            .ok_or_else(|| stale("控制原件超过组件调用方额度"))?;
        NativeAdmission(work)
            .native_codec_room(out, size - out.len())
            .map_err(stale)?;
        out.try_push(tag).map_err(stale)?;
        out.try_extend_bytes(&(bytes.len() as u64).to_le_bytes())
            .map_err(stale)?;
        out.try_extend_bytes(bytes).map_err(stale)
    })
}
