//! #1481 U-A：既有 AuditConnection 的观测值，不是第二个连接 owner。
//! 显式组件入口才启用。旧路径不增加前置；这里不签发历史认证。
use super::owner_sql::{CommitObserver, OwnerBackend, OwnerSql, RowsError};
#[cfg(test)]
use super::request_audit::ExactRows;
use super::{effects, native, request_audit::SqlCell, AuditTxn};
use crate::v3::audit_store::{CodecRead, NativeAdmission};
use newchan_rust::s_content::{capture_allocation::scratch::WorkScope, CodecVec};
use rusqlite::types::ValueRef;
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
    fn open(path: &Path, integrity_refusal: &std::cell::Cell<bool>) -> Result<Self, String> {
        Ok(Self {
            path: path.to_owned(),
            file: File::open(path).map_err(|error| {
                integrity_refusal.set(true);
                stale(error)
            })?,
        })
    }
    fn stamp(&self, integrity_refusal: &std::cell::Cell<bool>) -> Result<Stamp, String> {
        // #1481：路径/持有FD元数据失证无法与外部删除/替换区分，保守fail-stop保全证据。
        let source_error = |error| {
            integrity_refusal.set(true);
            stale(error)
        };
        let held = Stamp::read(self.file.metadata().map_err(source_error)?);
        let named = Stamp::read(std::fs::metadata(&self.path).map_err(source_error)?);
        if held != named {
            integrity_refusal.set(true);
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
/// 本切面观测见证；字段私有，只能由 `Clamp::phase` 构造。
#[cfg(test)]
pub(super) struct Phase(Observation);
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
    // #1481：同样的 stale 外观也可能来自资源失败；关闭策略认完整性比较或原件身份失证。
    integrity_refusal: std::cell::Cell<bool>,
    inherited_data_version: Option<i64>,
}
impl Clamp {
    #[cfg(test)]
    pub(super) fn open(
        connection: &dyn OwnerSql,
        path: &Path,
        control_limit: usize,
    ) -> Result<Self, String> {
        Self::open_in_parent(connection, path, control_limit, None)
    }
    #[cfg(test)]
    pub(super) fn open_in_parent(
        connection: &dyn OwnerSql,
        path: &Path,
        control_limit: usize,
        parent: Option<native::Budget>,
    ) -> Result<Self, String> {
        Self::open_in_parent_with_refusal(
            connection,
            path,
            control_limit,
            parent,
            &std::cell::Cell::new(false),
            None,
        )
    }
    pub(super) fn open_in_parent_with_refusal(
        connection: &dyn OwnerSql,
        path: &Path,
        control_limit: usize,
        parent: Option<native::Budget>,
        integrity_refusal: &std::cell::Cell<bool>,
        inherited_data_version: Option<i64>,
    ) -> Result<Self, String> {
        if let Some(parent) = &parent {
            parent
                .check_current()
                .map_err(|_| stale("控制父账已过期"))?;
        }
        let mode = connection.text("PRAGMA main.journal_mode").map_err(stale)?;
        if mode != "wal" {
            integrity_refusal.set(true);
            return Err(stale("H1组件要求既有WAL域，不自动改变journal_mode"));
        }
        let mut wal = path.as_os_str().to_os_string();
        wal.push("-wal");
        let wal = PathBuf::from(wal);
        let checkpoint_threshold = connection
            .integer("PRAGMA wal_autocheckpoint")
            .map_err(stale)
            .and_then(|n| i32::try_from(n).map_err(stale))?;
        let result = Self {
            main: WatchedFile::open(path, integrity_refusal)?,
            wal: WatchedFile::open(&wal, integrity_refusal)?,
            baseline: None,
            parent,
            control_limit,
            checkpoint_threshold,
            progress: physical::initial_progress(connection)?,
            integrity_refusal: std::cell::Cell::new(false),
            inherited_data_version,
        };
        connection.install_commit_observer(&CommitObserver {
            legacy: physical::observe_commit,
            #[cfg(test)]
            charged: physical::observe_commit_charged,
        })?;
        Ok(result)
    }
    pub(super) fn integrity_refusal(&self) -> bool {
        self.integrity_refusal.get()
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
        let main = self.main.stamp(&self.integrity_refusal)?;
        let wal = self.wal.stamp(&self.integrity_refusal)?;
        let mut header = vec![0; (wal.len.min(32)) as usize];
        if !header.is_empty() {
            self.wal
                .file
                .read_exact_at(&mut header, 0)
                .map_err(|error| {
                    self.integrity_refusal.set(true);
                    stale(error)
                })?;
        }
        if main != self.main.stamp(&self.integrity_refusal)?
            || wal != self.wal.stamp(&self.integrity_refusal)?
        {
            self.integrity_refusal.set(true);
            return Err(stale("文件观测期间发生变化"));
        }
        Ok(Files {
            main,
            wal,
            wal_header: header,
        })
    }
    fn observe<B: OwnerBackend>(&self, active: &AuditTxn<'_, B>) -> Result<Observation, String> {
        self.observe_tx(active.sql())
    }
    fn observe_tx(&self, tx: &dyn OwnerSql) -> Result<Observation, String> {
        let before = self.files()?;
        let out = Observation {
            data_version: tx.integer("PRAGMA main.data_version").map_err(stale)?,
            schema_version: tx.integer("PRAGMA main.schema_version").map_err(stale)?,
            domain: super::read_domain_of(tx).map_err(stale)?,
            total_changes: tx.total_changes(),
            controls: match &self.parent {
                Some(parent) => parent
                    .codec_work()
                    .map_err(resource_failure)?
                    .run(|work| Controls::read(self, tx, Some(work)))
                    .map_err(|error| resource_failure(native::Failure::from(error)))?,
                None => Controls::read(self, tx, None)?,
            },
            files: self.files()?,
        };
        if before != out.files {
            self.integrity_refusal.set(true);
            return Err(stale("锁内观测期间文件变化"));
        }
        Ok(out)
    }
    pub(super) fn check<B: OwnerBackend>(
        &mut self,
        active: &AuditTxn<'_, B>,
    ) -> Result<(), String> {
        let now = self.checked_against_baseline(active)?;
        if self.baseline.is_none() {
            self.baseline = Some(now);
        }
        Ok(())
    }
    /// 原 `check` 的唯一判据：交接外部提交、当前观测与生涯基线逐项全等。返回被核过的那一份观测。
    fn checked_against_baseline<B: OwnerBackend>(
        &mut self,
        active: &AuditTxn<'_, B>,
    ) -> Result<Observation, String> {
        // #1481：先核两把锁之间的外部提交；不得让后续控制读的暂时资源失败掩盖它。
        if let Some(inherited) = self.inherited_data_version.take() {
            if active.data_version != inherited {
                self.integrity_refusal.set(true);
                return Err(stale("私有认证交接期间发生外部SQLite提交"));
            }
        }
        let now = self.observe(active)?;
        if let Some(previous) = &self.baseline {
            if previous != &now {
                self.integrity_refusal.set(true);
                return Err(stale("版本、文件、控制原件、写计数或连接域改变"));
            }
        }
        Ok(now)
    }
    pub(super) fn candidate<B: OwnerBackend>(
        &self,
        active: &AuditTxn<'_, B>,
        effects: &effects::Effects,
    ) -> Result<WriteCandidate, String> {
        let (before, after) = self.own_write_versions(
            active,
            effects.shared_parent,
            effects.has_exact_coverage(active.sql().total_changes()),
        )?;
        before.controls.verify_effects(&after.controls, effects)?;
        self.own_write_files(before, &after)?;
        Ok(WriteCandidate { observation: after })
    }
    /// 自身写的父账、覆盖与版本判据（`candidate` 与材料期切面唯一共用）。返回生涯起点与本次唯一观测。
    fn own_write_versions<B: OwnerBackend>(
        &self,
        active: &AuditTxn<'_, B>,
        shared_parent: bool,
        exact_coverage: bool,
    ) -> Result<(&Observation, Observation), String> {
        if self.parent.is_some() && !shared_parent {
            return Err(stale("认证自身写effects未进入同一父账"));
        }
        if !exact_coverage {
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
        Ok((before, after))
    }
    /// 自身写的物理文件判据（`candidate` 与材料期切面唯一共用），在控制原件核之后。
    fn own_write_files(&self, before: &Observation, after: &Observation) -> Result<(), String> {
        Self::own_files(&before.files, &after.files, self.progress.complete())?;
        if before.files != after.files && before.total_changes == after.total_changes {
            return Err(stale("无自身SQL效果却发生物理变化"));
        }
        Ok(())
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
    /// #1419 M8D 3A：本切面观测就是签发时被原判据核过的那一份观测，不另行重新采样：
    /// 无自身写时与生涯基线全等（原 `check`），已闭合自身写时与已核 effects 候选全等
    /// （原 `check_candidate`）。它不是 `WriteCandidate`，不推进生涯基线，也无提交资格。
    #[cfg(test)]
    pub(super) fn phase_from_baseline<B: OwnerBackend>(
        &mut self,
        active: &AuditTxn<'_, B>,
    ) -> Result<Phase, String> {
        if self.baseline.is_none() {
            return Err(stale("缺生涯起点"));
        }
        Ok(Phase(self.checked_against_baseline(active)?))
    }
    /// #1419 M8D 3A 材料期切面：node 自身写之后、Recording 仍活、effects 尚未闭合。沿 `candidate`
    /// 同一父账/覆盖/版本与物理判据，控制原件以截至当前写序的 journal 键和写前原件解释；不闭合集合、
    /// 不产出 `WriteCandidate`、不推进基线。最终 finish/candidate 仍照原门完整执行。
    #[cfg(test)]
    pub(super) fn phase_from_material<B: OwnerBackend>(
        &self,
        active: &AuditTxn<'_, B>,
        writes: &effects::MaterialWrites,
        material: &effects::MaterialLive,
        originals: &[ExactRows],
    ) -> Result<Phase, String> {
        let (before, after) = self.own_write_versions(
            active,
            writes.shared_parent,
            writes.has_exact_coverage(active.sql().total_changes()),
        )?;
        before
            .controls
            .verify_writes(&after.controls, material, originals)?;
        self.own_write_files(before, &after)?;
        Ok(Phase(after))
    }
    #[cfg(test)]
    pub(super) fn phase_from_candidate<B: OwnerBackend>(
        &self,
        active: &AuditTxn<'_, B>,
        candidate: &WriteCandidate,
    ) -> Result<Phase, String> {
        Ok(Phase(self.checked_against_candidate(active, candidate)?))
    }
    /// reader 生涯内事务由读者独占且连接写入被拒，任何变化都不能由自身 effects 解释，
    /// 因而与本切面逐项全等；同一 `observe` 判据，不另立宽松核。
    #[cfg(test)]
    pub(super) fn check_phase(&self, tx: &dyn OwnerSql, phase: &Phase) -> Result<(), String> {
        if self.observe_tx(tx)? != phase.0 {
            self.integrity_refusal.set(true);
            return Err(stale("读期间版本、文件、控制原件、写计数或连接域改变"));
        }
        Ok(())
    }
    pub(super) fn arm_commit(&self) -> Result<CommitMonitor, String> {
        CommitMonitor::begin(self)
    }
    pub(super) fn check_candidate<B: OwnerBackend>(
        &self,
        active: &AuditTxn<'_, B>,
        candidate: &WriteCandidate,
    ) -> Result<(), String> {
        self.checked_against_candidate(active, candidate).map(drop)
    }
    /// 原 `check_candidate` 的唯一判据；返回被核过的那一份观测。
    fn checked_against_candidate<B: OwnerBackend>(
        &self,
        active: &AuditTxn<'_, B>,
        candidate: &WriteCandidate,
    ) -> Result<Observation, String> {
        let now = self.observe(active)?;
        if now != candidate.observation {
            return Err(stale("effects闭合后出现未声明变化"));
        }
        Ok(now)
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
        clamp: &Clamp,
        connection: &dyn OwnerSql,
        work: Option<&WorkScope>,
    ) -> Result<Self, String> {
        let limit = clamp.control_limit;
        let mut out = Self(Default::default());
        let admission = NativeAdmission(work);
        let mut used = 0usize;
        for (i, sql) in Self::SQL.into_iter().enumerate() {
            let rows = &mut out.0[i];
            connection
                .rows(sql, &mut |row| {
                    let mut wire = CodecVec::default();
                    for column in 0..row.len() {
                        encode_value(
                            &mut wire,
                            row.value(column).map_err(stale)?,
                            limit.saturating_sub(used),
                            work,
                        )?;
                    }
                    used = used
                        .checked_add(wire.len())
                        .filter(|n| *n <= limit)
                        .ok_or_else(|| stale("控制原件超过组件调用方额度"))?;
                    admission.native_codec_room(rows, 1).map_err(stale)?;
                    rows.try_push((row.integer(0).map_err(stale)?, wire))
                        .map_err(stale)
                })
                .map_err(|error| match error {
                    RowsError::Sql(error) => stale(error),
                    RowsError::Row(error) => error,
                    // 准入 schema 下行宽至多 8；更宽只可能是外部改动 schema，按完整性拒绝保全证据，
                    // 与原连接上同一改动在基线比对处被判为完整性拒绝一致。
                    #[cfg(test)]
                    RowsError::Shape(error) => {
                        clamp.integrity_refusal.set(true);
                        stale(error)
                    }
                })?;
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
#[cfg(test)]
impl Controls {
    /// 材料期控制原件核：与 `verify_effects` 同一前像判据（写前原件须等于生涯原件），变化只能由
    /// 本事务 journal 截至当前写序已登记的同表同 rowid 写解释；未登记的变化即拒。后像即本次观测本身。
    fn verify_writes(
        &self,
        after: &Self,
        material: &effects::MaterialLive,
        originals: &[ExactRows],
    ) -> Result<(), String> {
        for (index, table) in [effects::Table::Meta, effects::Table::Protocol]
            .into_iter()
            .enumerate()
        {
            let original = effects::table(originals, table)?;
            let mut old = self.0[index].iter().peekable();
            let mut new = after.0[index].iter().peekable();
            loop {
                let Some(rowid) = [old.peek().map(|r| r.0), new.peek().map(|r| r.0)]
                    .into_iter()
                    .flatten()
                    .min()
                else {
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
                if material.covers_row(table, rowid)? {
                    let prior = original
                        .rows
                        .binary_search_by_key(&rowid, |r| {
                            effects::integer(&r[0]).unwrap_or(i64::MIN)
                        })
                        .ok()
                        .map(|i| &original.rows[i][..]);
                    if !matches_row(before, prior) {
                        return Err(stale("effects控制前像与生涯原件不符"));
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

/// #1419 3C U5：夹持观测与提交观察在收费组件连接上运行原判据（连接域核、控制原件、版本、
/// 文件、WAL 提交收据），同父收费；收费首拒之后观测不再进入 SQLite 并带首拒报错。
/// allocator 只能在本进程首次 SQLite 初始化前安装，须全新进程 `--release --ignored --exact`。
#[cfg(test)]
mod charged_tests {
    use super::super::owner_sql::ChargedOwnerSql;
    use super::*;
    use newchan_rust::s_content::capture_allocation::scratch::ScratchFailure;
    use newchan_rust::s_content::capture_allocation::sqlite_parent::{
        SqlGate, SqliteParentAllocator,
    };
    use rusqlite::ffi;

    const LIMIT: usize = 64 << 20;

    #[test]
    #[ignore = "fresh process only: installs the global SQLite allocator; run alone with --ignored --exact"]
    fn clamp_observes_and_receipts_commit_on_charged_owner_connection() {
        // SAFETY: this test runs alone in a fresh process before any SQLite use.
        let allocator = unsafe { SqliteParentAllocator::install() }.expect("install before init");
        let root = native::Budget::fresh(LIMIT).unwrap();
        let current = root.bind_frozen(1, LIMIT, 0, 0).unwrap();
        let (parent, origin) = current.sqlite_parent();
        let binding = allocator.bind(parent, origin).expect("bind");
        let dir = std::env::temp_dir().join(format!("clamp-charged-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("owner.sqlite3");
        let base = current.live();
        let path = std::ffi::CString::new(db.to_str().unwrap()).unwrap();
        let conn = binding.open(&path).unwrap();
        binding
            .run(&conn, |q| {
                q.execute_batch(
                    "PRAGMA journal_mode=WAL;
                     CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                     INSERT INTO meta VALUES('rule_revision', 'r');
                     CREATE TABLE s_protocol_meta(singleton INTEGER PRIMARY KEY CHECK(singleton=1),
                       protocol_revision TEXT NOT NULL, session_generation TEXT NOT NULL,
                       clock_plan_hash TEXT NOT NULL, clock_plan_json TEXT NOT NULL,
                       next_attempt_ordinal INTEGER NOT NULL, logical_phase_frontier TEXT NOT NULL);
                     INSERT INTO s_protocol_meta VALUES(1, 's-session/3', 'g', 'h', '{}', 1, 'f');",
                )
            })
            .unwrap();
        let sql = ChargedOwnerSql {
            binding: &binding,
            connection: &conn,
        };
        let refusal = std::cell::Cell::new(false);
        let clamp = Clamp::open_in_parent_with_refusal(
            &sql,
            &db,
            1 << 20,
            Some(current.clone()),
            &refusal,
            None,
        )
        .unwrap();
        assert!(!refusal.get());
        assert_eq!(
            super::super::read_domain_of(&sql).unwrap(),
            [0, 1, 0, 1, 0, 1],
            "read_uncommitted 0; bundled SQLITE_DEFAULT_FOREIGN_KEYS=1; recursive_triggers 0; \
             trusted_schema default 1; query_only 0; UTF-8"
        );
        let first = clamp.observe_tx(&sql).unwrap();
        let again = clamp.observe_tx(&sql).unwrap();
        assert!(first == again, "no change between observations");
        assert_eq!(
            first.controls.0[0].len(),
            1,
            "meta rows read through the charged cells"
        );
        assert_eq!(first.controls.0[1].len(), 1, "s_protocol_meta rows read");
        assert!(
            binding.live_bytes() > 0,
            "observation SQLite bytes live on the parent"
        );

        let monitor = clamp.arm_commit().unwrap();
        binding
            .run(&conn, |q| {
                q.execute_batch("INSERT INTO meta VALUES('k', 'v');")
            })
            .unwrap();
        assert!(
            matches!(monitor.take(), Some(Ok(_))),
            "the charged WAL hook produced the commit receipt"
        );
        drop(monitor);
        let after = clamp.observe_tx(&sql).unwrap();
        assert!(
            after != first,
            "the committed write is visible to the clamp"
        );

        let refused = binding.run(&conn, |_| {
            // SAFETY: plain SQLite allocation inside the operation domain.
            assert!(unsafe { ffi::sqlite3_malloc64(LIMIT as u64 + 1) }.is_null());
            Ok(())
        });
        assert_eq!(refused, Err(SqlGate::Refused(ScratchFailure::Budget)));
        let stale = clamp
            .observe_tx(&sql)
            .err()
            .expect("observation after first refusal");
        assert!(stale.contains("Refused(Budget)"), "{stale}");
        assert_eq!(
            binding.refusal(),
            Some(ScratchFailure::Budget),
            "first refusal kept"
        );

        drop((first, again, after));
        binding.clear_write_hooks(&conn).unwrap();
        binding.close(conn).map_err(|(_, e)| e).unwrap();
        drop(clamp);
        assert_eq!(
            current.live(),
            base,
            "all connection and control bytes returned"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
