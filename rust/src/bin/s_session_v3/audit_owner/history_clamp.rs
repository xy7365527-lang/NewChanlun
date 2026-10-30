//! #1481 U-A：既有 AuditConnection 的观测值，不是第二个连接 owner。
//! 显式组件入口才启用。旧路径不增加前置；这里不签发历史认证。
use super::{effects, request_audit::SqlCell, AuditTxn};
use rusqlite::{types::ValueRef, Connection};
use std::{collections::BTreeMap, fs::{File, Metadata}, os::unix::fs::MetadataExt, path::{Path, PathBuf}};

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
        Self { dev: m.dev(), ino: m.ino(), len: m.len(),
            modified: (m.mtime(), m.mtime_nsec()), changed: (m.ctime(), m.ctime_nsec()) }
    }
    fn same_file(&self, other: &Self) -> bool {
        (self.dev, self.ino) == (other.dev, other.ino)
    }
}
struct WatchedFile { path: PathBuf, file: File }
impl WatchedFile {
    fn open(path: &Path) -> Result<Self, String> {
        Ok(Self { path: path.to_owned(), file: File::open(path).map_err(stale)? })
    }
    fn stamp(&self) -> Result<Stamp, String> {
        let held = Stamp::read(self.file.metadata().map_err(stale)?);
        let named = Stamp::read(std::fs::metadata(&self.path).map_err(stale)?);
        if held != named { return Err(stale("路径与持有文件身份/stat不一致")); }
        Ok(held)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Files { main: Stamp, wal: Stamp, wal_header: Vec<u8> }
#[derive(Clone, Debug, PartialEq, Eq)]
struct Controls([BTreeMap<i64, Vec<u8>>; 2]);
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
pub(super) struct WriteCandidate { observation: Observation }
pub(super) struct Clamp {
    main: WatchedFile,
    wal: WatchedFile,
    baseline: Option<Observation>,
    control_limit: usize,
}
impl Clamp {
    pub(super) fn open(connection: &Connection, path: &Path, control_limit: usize) -> Result<Self, String> {
        let mode: String = connection.query_row("PRAGMA main.journal_mode", [], |r| r.get(0)).map_err(stale)?;
        if mode != "wal" { return Err(stale("H1组件要求既有WAL域，不自动改变journal_mode")); }
        let mut wal = path.as_os_str().to_os_string();
        wal.push("-wal");
        let wal = PathBuf::from(wal);
        Ok(Self { main: WatchedFile::open(path)?, wal: WatchedFile::open(&wal)?, baseline: None, control_limit })
    }
    fn files(&self) -> Result<Files, String> {
        use std::os::unix::fs::FileExt;
        let main = self.main.stamp()?;
        let wal = self.wal.stamp()?;
        let mut header = vec![0; (wal.len.min(32)) as usize];
        if !header.is_empty() { self.wal.file.read_exact_at(&mut header, 0).map_err(stale)?; }
        if main != self.main.stamp()? || wal != self.wal.stamp()? {
            return Err(stale("文件观测期间发生变化"));
        }
        Ok(Files { main, wal, wal_header: header })
    }
    fn observe(&self, active: &AuditTxn<'_>) -> Result<Observation, String> {
        let before = self.files()?;
        let out = Observation {
            data_version: active.tx().query_row("PRAGMA main.data_version", [], |r| r.get(0)).map_err(stale)?,
            schema_version: active.tx().query_row("PRAGMA main.schema_version", [], |r| r.get(0)).map_err(stale)?,
            domain: active.read_domain().map_err(stale)?,
            total_changes: active.tx().total_changes(),
            controls: Controls::read(active.tx(), self.control_limit)?,
            files: self.files()?,
        };
        if before != out.files { return Err(stale("锁内观测期间文件变化")); }
        Ok(out)
    }
    pub(super) fn check(&mut self, active: &AuditTxn<'_>) -> Result<(), String> {
        let now = self.observe(active)?;
        if let Some(previous) = &self.baseline {
            if previous != &now { return Err(stale("版本、文件、控制原件、写计数或连接域改变")); }
        } else { self.baseline = Some(now); }
        Ok(())
    }
    pub(super) fn candidate(&self, active: &AuditTxn<'_>, effects: &effects::Effects) -> Result<WriteCandidate, String> {
        if !effects.has_exact_coverage(active.tx().total_changes()) {
            return Err(stale("自身写缺完整effects覆盖"));
        }
        let before = self.baseline.as_ref().ok_or_else(|| stale("缺生涯起点"))?;
        let after = self.observe(active)?;
        if before.data_version != after.data_version || before.schema_version != after.schema_version
            || before.domain != after.domain || before.total_changes > after.total_changes {
            return Err(stale("自身写改变版本/schema/domain"));
        }
        let mut expected = before.controls.clone();
        expected.apply(effects, self.control_limit)?;
        if expected != after.controls { return Err(stale("控制原件变化未由effects解释")); }
        Self::own_files(&before.files, &after.files)?;
        Ok(WriteCandidate { observation: after })
    }
    // 信任的SQLite写口只解释WAL同一文件/同一轮次内追加；checkpoint/reset
    // 尚无自身物理收据时拒绝，而不是把任意文件stat改变归给“本次有写”。
    fn own_files(before: &Files, after: &Files) -> Result<(), String> {
        if before.main != after.main || !before.wal.same_file(&after.wal) || after.wal.len < before.wal.len {
            return Err(stale("自身写不能解释main变化、WAL替换或截短"));
        }
        if before.wal.len != 0 && before.wal_header != after.wal_header {
            return Err(stale("WAL轮次变化缺物理收据"));
        }
        if before.wal != after.wal && after.wal.len == before.wal.len {
            return Err(stale("WAL未追加却发生stat变化"));
        }
        if after.wal.len != 0 {
            let h = &after.wal_header;
            if h.len() != 32 { return Err(stale("WAL头不完整")); }
            let page = u32::from_be_bytes(h[8..12].try_into().unwrap()) as u64;
            if page < 512 || page > 65536 || !page.is_power_of_two()
                || (after.wal.len - 32) % (page + 24) != 0 {
                return Err(stale("WAL追加不是完整frame"));
            }
        }
        Ok(())
    }
    pub(super) fn check_candidate(&self, active: &AuditTxn<'_>, candidate: &WriteCandidate) -> Result<(), String> {
        if self.observe(active)? != candidate.observation { return Err(stale("effects闭合后出现未声明变化")); }
        Ok(())
    }
    pub(super) fn committed(&mut self, mut candidate: WriteCandidate) -> Result<(), String> {
        let after = self.files()?;
        Self::own_files(&candidate.observation.files, &after)?;
        candidate.observation.files = after;
        // 不在commit后重读data_version；下一读快照与锁内旧值比较，抓住外写。
        self.baseline = Some(candidate.observation);
        Ok(())
    }
}

impl Controls {
    const SQL: [&'static str; 2] = [
        "SELECT rowid,* FROM main.meta ORDER BY rowid",
        "SELECT rowid,* FROM main.s_protocol_meta ORDER BY rowid",
    ];
    fn read(connection: &Connection, limit: usize) -> Result<Self, String> {
        let mut out = Self(Default::default());
        let mut used = 0usize;
        for (i, sql) in Self::SQL.into_iter().enumerate() {
            let mut statement = connection.prepare(sql).map_err(stale)?;
            let columns = statement.column_count();
            let mut rows = statement.query([]).map_err(stale)?;
            while let Some(row) = rows.next().map_err(stale)? {
                let mut wire = Vec::new();
                for column in 0..columns {
                    encode_value(&mut wire, row.get_ref(column).map_err(stale)?, limit.saturating_sub(used))?;
                }
                used = used.checked_add(wire.len()).filter(|n| *n <= limit).ok_or_else(|| stale("控制原件超过组件调用方额度"))?;
                out.0[i].insert(row.get(0).map_err(stale)?, wire);
            }
        }
        Ok(out)
    }
    fn apply(&mut self, effects: &effects::Effects, limit: usize) -> Result<(), String> {
        for (index, table) in [effects::Table::Meta, effects::Table::Protocol].into_iter().enumerate() {
            for change in effects.rows(table) {
                let old = change.old.as_ref().map(|row| encode_cells(row, limit)).transpose()?;
                if self.0[index].get(&change.rowid) != old.as_ref() { return Err(stale("effects控制前像与生涯原件不符")); }
                if let Some(row) = &change.new { self.0[index].insert(change.rowid, encode_cells(row, limit)?); }
                else { self.0[index].remove(&change.rowid); }
            }
        }
        let bytes: usize = self.0.iter().flat_map(|m| m.values()).map(Vec::len).sum();
        if bytes > limit { return Err(stale("控制原件超过组件调用方额度")); }
        Ok(())
    }
}
fn encode_cells(cells: &[SqlCell], limit: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for cell in cells {
        let value = match cell {
            SqlCell::Null => ValueRef::Null,
            SqlCell::Integer(n) => ValueRef::Integer(*n),
            SqlCell::Real(bits) => ValueRef::Real(f64::from_bits(*bits)),
            SqlCell::Text(bytes) => ValueRef::Text(bytes),
            SqlCell::Blob(bytes) => ValueRef::Blob(bytes),
        };
        encode_value(&mut out, value, limit)?;
    }
    Ok(out)
}
fn encode_value(out: &mut Vec<u8>, value: ValueRef<'_>, limit: usize) -> Result<(), String> {
    let fixed;
    let (tag, bytes): (u8, &[u8]) = match value {
        ValueRef::Null => (0, &[]),
        ValueRef::Integer(n) => { fixed = n.to_le_bytes(); (1, &fixed) },
        ValueRef::Real(n) => { fixed = n.to_bits().to_le_bytes(); (2, &fixed) },
        ValueRef::Text(b) => (3, b), ValueRef::Blob(b) => (4, b),
    };
    let size = out.len().checked_add(9).and_then(|n| n.checked_add(bytes.len())).filter(|n| *n <= limit)
        .ok_or_else(|| stale("控制原件超过组件调用方额度"))?;
    out.try_reserve(size - out.len()).map_err(stale)?;
    out.push(tag); out.extend_from_slice(&(bytes.len() as u64).to_le_bytes()); out.extend_from_slice(bytes);
    Ok(())
}
