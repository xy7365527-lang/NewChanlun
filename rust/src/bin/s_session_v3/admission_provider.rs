//! #1527 I-1：真实文件域测量。只测分配/固定来源，不解释 SQLite 页、WAL 或 journal。
//! st_blocks*512 是 OS 报告的文件/目录已分配量；不是共享 extent 独占性或 inode 隐藏开销证明。
use super::admission_anchor::{ConsumedAdmission, Stage};
use newchan_rust::s_content::job::Existing;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fs::{File, Metadata, OpenOptions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stamp {
    dev: u64,
    ino: u64,
    mode: u32,
    uid: u32,
    nlink: u64,
    len: u64,
    blocks: u64,
    mtime: (i64, i64),
    ctime: (i64, i64),
}
impl Stamp {
    fn of(m: &Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            mode: m.mode(),
            uid: m.uid(),
            nlink: m.nlink(),
            len: m.len(),
            blocks: m.blocks(),
            mtime: (m.mtime(), m.mtime_nsec()),
            ctime: (m.ctime(), m.ctime_nsec()),
        }
    }
    fn same_object(self, other: Self) -> bool {
        (self.dev, self.ino, self.mode, self.uid, self.nlink)
            == (other.dev, other.ino, other.mode, other.uid, other.nlink)
    }
}
struct Pinned {
    path: PathBuf,
    file: File,
    stamp: Stamp,
    directory: bool,
}
impl Pinned {
    fn open(path: &Path) -> Result<Self, String> {
        let m = std::fs::symlink_metadata(path).map_err(|_| "SourceMetadataUnavailable")?;
        if !(m.is_file() || m.is_dir()) || (m.is_file() && m.nlink() != 1) {
            return Err("SourceTypeOrHardlinkUnsupported".into());
        }
        let f = OpenOptions::new()
            .read(true)
            .custom_flags(
                libc::O_NOFOLLOW
                    | libc::O_NONBLOCK
                    | libc::O_CLOEXEC
                    | if m.is_dir() { libc::O_DIRECTORY } else { 0 },
            )
            .open(path)
            .map_err(|_| "SourcePinFailed")?;
        let fm = f.metadata().map_err(|_| "SourceMetadataUnavailable")?;
        if Stamp::of(&m) != Stamp::of(&fm) {
            return Err("SourceChangedDuringPin".into());
        }
        Ok(Self {
            path: path.into(),
            file: f,
            stamp: Stamp::of(&fm),
            directory: fm.is_dir(),
        })
    }
    fn verify(&self, strict: bool) -> Result<(), String> {
        let a = self.file.metadata().map_err(|_| "SourceFdLost")?;
        let b = std::fs::symlink_metadata(&self.path).map_err(|_| "SourcePathLost")?;
        let matches = |m: &Metadata| {
            if strict {
                Stamp::of(m) == self.stamp
            } else {
                self.stamp.same_object(Stamp::of(m))
            }
        };
        if !matches(&a) || !matches(&b) {
            return Err("SourceIdentityChanged".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Medium {
    MainOnly,
    Wal,
    RollbackJournal,
}

pub(super) struct MeasuredDomain {
    db: PathBuf,
    state: PathBuf,
    files: Vec<Pinned>,
    ledger_cache: RefCell<Vec<Pinned>>,
    members: BTreeSet<PathBuf>,
    main_index: usize,
    wal_index: Option<usize>,
    pub(super) medium: Medium,
    pub(super) existing: Existing,
}

pub(super) fn sidecar(db: &Path, suffix: &str) -> PathBuf {
    let mut path = db.as_os_str().to_os_string();
    path.push(suffix);
    PathBuf::from(path)
}

// #1481/#1644：精确登记可丢弃台账的四个名字；不是通配旁挂白名单。
const AUDIT_LEDGER_SUFFIXES: [&str; 4] = [
    ".audit-ledger.sqlite",
    ".audit-ledger.sqlite-wal",
    ".audit-ledger.sqlite-shm",
    ".audit-ledger.sqlite-journal",
];

/// #1481：完整认证后的受控模式转换只交接物理连续性，不签发历史资格。
#[cfg(feature = "s_incremental")]
pub(super) struct ColdConvertedContent {
    main: Pinned,
    wal: Pinned,
}
#[cfg(feature = "s_incremental")]
impl ColdConvertedContent {
    /// 必须在正式 owner 第一把 IMMEDIATE 锁内核验，不能从此处重取新基线。
    pub(super) fn verify(&self) -> Result<(), String> {
        self.main.verify(true)?;
        self.wal.verify(true)
    }
}

#[cfg(feature = "s_incremental")]
fn verify_unframed_wal(db: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(sidecar(db, "-wal")) {
        Ok(wal) => {
            if !wal.is_file()
                || wal.nlink() != 1
                || wal.uid() != unsafe { libc::geteuid() }
                || wal.len() > 32
            {
                return Err("SourceWalChanged".into());
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("SourceWalMetadataFailed".into()),
    }
    Ok(())
}

impl MeasuredDomain {
    pub(super) fn measure(consumed: &ConsumedAdmission) -> Result<Self, String> {
        consumed.check(Stage::E6).map_err(|e| e.to_string())?;
        let db = consumed.lease().db_path().to_owned();
        let state = consumed.lease().state_path().to_owned();
        // 完整枚举本 DB basename 家族；不忽略未知 super-journal、旧私有目录等残件。
        let members = Self::members(&db, &state, consumed)?;
        let mut files = Vec::new();
        let mut ledger_cache = Vec::new();
        let mut identities = BTreeSet::new();
        let mut store = 0u64;
        let mut staged = 0u64;
        let mut main_index = None;
        let mut wal_index = None;
        let mut journal = false;
        let mut shm = false;
        for path in &members {
            consumed.check(Stage::E6).map_err(|e| e.to_string())?;
            let pinned = Pinned::open(path).map_err(|e| consumed.failure(&e))?;
            consumed.check(Stage::E6).map_err(|e| e.to_string())?;
            // #1527 H-01：监督器的既存控制锁只读固定、照实计量；不创建、不重新 flock。
            if path == &sidecar(&db, ".control.lock") && pinned.directory {
                return Err(consumed.failure("ControlLockNotRegular"));
            }
            let is_ledger = AUDIT_LEDGER_SUFFIXES
                .iter()
                .any(|suffix| path == &sidecar(&db, suffix));
            if pinned.directory && is_ledger {
                return Err(consumed.failure("AuditLedgerCacheNotRegular"));
            }
            if !identities.insert((pinned.stamp.dev, pinned.stamp.ino)) {
                return Err(consumed.failure("SourceAliasedMember"));
            }
            let physical = pinned
                .stamp
                .blocks
                .checked_mul(512)
                .ok_or_else(|| consumed.failure("AllocationOverflow"))?;
            store = store
                .checked_add(physical)
                .ok_or_else(|| consumed.failure("AllocationOverflow"))?;
            // 本实现不创建暂存。已存在的私有子树全部测入暂存和总账，随后具名拒绝，不删除未知来源。
            if path.starts_with(sidecar(&db, ".admission-private")) {
                staged = staged
                    .checked_add(physical)
                    .ok_or_else(|| consumed.failure("AllocationOverflow"))?;
            }
            if store > consumed.limits().max_store_bytes
                || store > 64 * 1024 * 1024 * 1024
                || staged > consumed.limits().max_staged_bytes
            {
                return Err(consumed.failure("ExistingAllocationExceeded"));
            }
            if path == &db {
                main_index = Some(files.len());
            }
            if path == &sidecar(&db, "-wal") {
                wal_index = Some(files.len());
            }
            journal |= path == &sidecar(&db, "-journal");
            shm |= path == &sidecar(&db, "-shm");
            if is_ledger {
                ledger_cache.push(pinned);
            } else {
                files.push(pinned);
            }
        }
        if members
            .iter()
            .any(|p| p.starts_with(sidecar(&db, ".admission-private")))
        {
            return Err(consumed.failure("PrivateResidueOwnershipUnproven"));
        }
        if shm || (journal && wal_index.is_some()) {
            return Err(consumed.failure("UnsupportedWalShmJournalCombination"));
        }
        let medium = if journal {
            Medium::RollbackJournal
        } else if wal_index.is_some() {
            Medium::Wal
        } else {
            Medium::MainOnly
        };
        let result = Self {
            db,
            state,
            files,
            ledger_cache: RefCell::new(ledger_cache),
            members,
            main_index: main_index.ok_or_else(|| consumed.failure("MainMissing"))?,
            wal_index,
            medium,
            existing: Existing {
                store_bytes: store,
                staged_bytes: staged,
            },
        };
        let verified = result.verify_before_open();
        consumed.check(Stage::E6).map_err(|e| e.to_string())?;
        verified.map_err(|e| consumed.failure(&e))?;
        Ok(result)
    }

    fn members(
        db: &Path,
        state: &Path,
        consumed: &ConsumedAdmission,
    ) -> Result<BTreeSet<PathBuf>, String> {
        let parent = db.parent().ok_or("SourceParentMissing")?;
        let basename = db
            .file_name()
            .ok_or("SourceNameMissing")?
            .as_encoded_bytes();
        let mut set = BTreeSet::new();
        set.insert(parent.to_owned());
        for entry in
            std::fs::read_dir(parent).map_err(|_| consumed.failure("SourceEnumerationFailed"))?
        {
            consumed.check(Stage::E6).map_err(|e| e.to_string())?;
            let entry = entry.map_err(|_| consumed.failure("SourceEnumerationFailed"))?;
            let name = entry.file_name();
            let raw = name.as_encoded_bytes();
            if raw == basename
                || (raw.starts_with(basename)
                    && matches!(raw.get(basename.len()), Some(b'.' | b'-')))
            {
                let path = entry.path();
                if path != db
                    && ![
                        "-wal",
                        "-shm",
                        "-journal",
                        ".writer.lock",
                        ".maintenance.lock",
                        ".control.lock",
                        ".control-owner.json",
                        ".admission-private",
                    ]
                    .iter()
                    .chain(AUDIT_LEDGER_SUFFIXES.iter())
                    .any(|suffix| path == sidecar(db, suffix))
                {
                    return Err(consumed.failure("UnknownDatabaseSidecar"));
                }
                Self::walk(&path, &mut set, consumed)?;
            }
        }
        // parent 已列入分配账；state 恰为 parent 时仍须枚举其全部子项。
        if state == parent {
            set.remove(state);
        }
        Self::walk(state, &mut set, consumed)?;
        Ok(set)
    }
    fn walk(
        path: &Path,
        out: &mut BTreeSet<PathBuf>,
        consumed: &ConsumedAdmission,
    ) -> Result<(), String> {
        consumed.check(Stage::E6).map_err(|e| e.to_string())?;
        if !out.insert(path.into()) {
            return Ok(());
        }
        let m = std::fs::symlink_metadata(path)
            .map_err(|_| consumed.failure("SourceMetadataUnavailable"))?;
        if m.is_dir() {
            for e in
                std::fs::read_dir(path).map_err(|_| consumed.failure("SourceEnumerationFailed"))?
            {
                Self::walk(
                    &e.map_err(|_| consumed.failure("SourceEnumerationFailed"))?
                        .path(),
                    out,
                    consumed,
                )?;
            }
        } else if !m.is_file() {
            return Err(consumed.failure("SourceTypeUnsupported"));
        }
        Ok(())
    }
    pub(super) fn main_fd(&self) -> Result<File, String> {
        self.files[self.main_index]
            .file
            .try_clone()
            .map_err(|_| "SourceFdCloneFailed".into())
    }
    pub(super) fn wal_fd(&self) -> Result<Option<File>, String> {
        self.wal_index
            .map(|i| {
                self.files[i]
                    .file
                    .try_clone()
                    .map_err(|_| "SourceFdCloneFailed".into())
            })
            .transpose()
    }
    /// #1481/#1644：只供持有 WriterScope 双锁的 H1 冷启动在删除旧缓存之前调用。
    /// 固定身份及四个名字的在场情况，不读缓存内容、不以旧台账签发历史资格。
    #[cfg(feature = "s_incremental")]
    pub(super) fn verify_cold_ledger_cache(&self) -> Result<(), String> {
        let cache = self.ledger_cache.borrow();
        for suffix in AUDIT_LEDGER_SUFFIXES {
            let path = sidecar(&self.db, suffix);
            match cache.iter().find(|p| p.path == path) {
                Some(pinned) => pinned.verify(false)?,
                None => match std::fs::symlink_metadata(&path) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Ok(_) => return Err("AuditLedgerCacheMembershipChanged".into()),
                    Err(_) => return Err("AuditLedgerCacheMetadataFailed".into()),
                },
            }
        }
        Ok(())
    }
    /// 仅在 H1 精确删除四名缓存后释放旧 inode 的 FD，避免新旧台账同时占盘。
    #[cfg(feature = "s_incremental")]
    pub(super) fn release_cold_ledger_cache(&self) -> Result<(), String> {
        for suffix in AUDIT_LEDGER_SUFFIXES {
            match std::fs::symlink_metadata(sidecar(&self.db, suffix)) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Ok(_) => return Err("AuditLedgerCacheStillPresent".into()),
                Err(_) => return Err("AuditLedgerCacheMetadataFailed".into()),
            }
        }
        self.ledger_cache.borrow_mut().clear();
        Ok(())
    }
    /// 调用者须仍持一次性转换连接的 SQLite EXCLUSIVE 锁；转换前已核原内容 stamp。
    #[cfg(feature = "s_incremental")]
    pub(super) fn capture_cold_conversion(&self) -> Result<ColdConvertedContent, String> {
        if self.medium != Medium::MainOnly {
            return Err("ColdConversionMediumUnsupported".into());
        }
        self.files[self.main_index].verify(false)?;
        let main = Pinned::open(&self.db)?;
        if !self.files[self.main_index].stamp.same_object(main.stamp) {
            return Err("SourceIdentityChanged".into());
        }
        verify_unframed_wal(&self.db)?;
        // 转换 helper 须在 EXCLUSIVE 内真正 BEGIN IMMEDIATE/读库，建立空 WAL 后才取戳。
        // NO_CKPT_ON_CLOSE 保留该空 WAL；普通 owner 首次 IMMEDIATE 不得换 inode 或改 stamp。
        let wal = Pinned::open(&sidecar(&self.db, "-wal"))?;
        if wal.directory || wal.stamp.uid != unsafe { libc::geteuid() } || wal.stamp.len > 32 {
            return Err("SourceWalChanged".into());
        }
        Ok(ColdConvertedContent { main, wal })
    }
    pub(super) fn verify_before_open(&self) -> Result<(), String> {
        for p in &self.files {
            p.verify(true)?;
        }
        for p in self.ledger_cache.borrow().iter() {
            p.verify(true)?;
        }
        // 新增 sidecar/状态成员也不得在私有核验之后悄悄进入源域。
        let mut found = BTreeSet::new();
        fn walk(path: &Path, set: &mut BTreeSet<PathBuf>) -> Result<(), String> {
            if !set.insert(path.into()) {
                return Ok(());
            }
            let m = std::fs::symlink_metadata(path).map_err(|_| "SourcePathLost")?;
            if m.is_dir() {
                for e in std::fs::read_dir(path).map_err(|_| "SourceEnumerationFailed")? {
                    walk(&e.map_err(|_| "SourceEnumerationFailed")?.path(), set)?;
                }
            } else if !m.is_file() {
                return Err("SourceTypeChanged".into());
            }
            Ok(())
        }
        let parent = self.db.parent().ok_or("SourceParentMissing")?;
        found.insert(parent.into());
        let basename = self
            .db
            .file_name()
            .ok_or("SourceNameMissing")?
            .as_encoded_bytes();
        for e in std::fs::read_dir(parent).map_err(|_| "SourceEnumerationFailed")? {
            let e = e.map_err(|_| "SourceEnumerationFailed")?;
            let name = e.file_name();
            let raw = name.as_encoded_bytes();
            if raw == basename
                || (raw.starts_with(basename)
                    && matches!(raw.get(basename.len()), Some(b'.' | b'-')))
            {
                walk(&e.path(), &mut found)?;
            }
        }
        if self.state == parent {
            found.remove(&self.state);
        }
        walk(&self.state, &mut found)?;
        if found != self.members {
            return Err("SourceMembershipChanged".into());
        }
        Ok(())
    }
    /// #1481：私有认证 → owner 写锁内交接，源主库/WAL须保持原完整stamp。
    /// 只检查内容域；本次合法创建的派生审计台账不冒充源成员。
    #[cfg(feature = "s_incremental")]
    pub(super) fn verify_cold_content(&self) -> Result<(), String> {
        self.files[self.main_index].verify(true)?;
        if let Some(index) = self.wal_index {
            self.files[index].verify(true)?;
        } else {
            // H1交接的首个RW/IMMEDIATE可自行建立无帧WAL；不含任何已提交历史。
            verify_unframed_wal(&self.db)?;
        }
        Ok(())
    }
    /// 原件合法写入后主库/目录只核身份；监督器控制锁仍核固定完整 stamp。
    /// 此门不证明 sidecar 的跨开库连续性（I-3）。
    pub(super) fn verify_runtime(&self) -> Result<(), String> {
        self.files[self.main_index].verify(false)?;
        for p in &self.files {
            if p.directory {
                p.verify(false)?;
            } else if p.path == sidecar(&self.db, ".control.lock") {
                // #1527 H-01：关 FD 留路径合法；控制锁不是 SQLite 的可变侧件。
                p.verify(true)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod control_lock_tests {
    // #1527 H-01：仅普通文件/真实 loader 的源码测试；本轮 NOT_RUN。
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture {
        root: PathBuf,
        db: PathBuf,
        control: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "issue1527-control-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            let root = root.canonicalize().unwrap();
            let db = root.join("ordinary.db");
            std::fs::write(&db, b"provider test: no SQLite open").unwrap();
            let control = sidecar(&db, ".control.lock");
            Self { root, db, control }
        }
        fn consumed(&self) -> ConsumedAdmission {
            super::super::admission_anchor::test_prepare(&self.db)
                .consume()
                .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn control_lock_is_measured(held: bool) {
        let f = Fixture::new();
        let mut controller = File::create(&f.control).unwrap();
        controller.write_all(&[0x52; 4096]).unwrap();
        controller.sync_all().unwrap();
        controller.try_lock().unwrap();
        let original = Stamp::of(&controller.metadata().unwrap());
        assert!(original.blocks > 0);
        let controller = if held {
            Some(controller)
        } else {
            drop(controller);
            None
        };
        let consumed = f.consumed();
        let source = MeasuredDomain::measure(&consumed).unwrap();
        let pinned = source.files.iter().find(|p| p.path == f.control).unwrap();
        assert_eq!(pinned.stamp, original);
        assert_eq!(Stamp::of(&pinned.file.metadata().unwrap()), original);
        assert_eq!(Stamp::of(&std::fs::metadata(&f.control).unwrap()), original);
        let expected: u64 = source
            .members
            .iter()
            .map(|p| std::fs::symlink_metadata(p).unwrap().blocks() * 512)
            .sum();
        assert_eq!(source.existing.store_bytes, expected);
        assert_eq!(source.existing.staged_bytes, 0);
        let without_control = expected - original.blocks * 512;
        assert!(source.existing.store_bytes > without_control);
        source.verify_before_open().unwrap();
        source.verify_runtime().unwrap();

        let probe = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&f.control)
            .unwrap();
        if held {
            assert!(probe.try_lock().is_err(), "监督器仍拥有锁");
            drop(controller);
        }
        probe.try_lock().unwrap(); // provider 的只读 FD 没有争抢/延长控制锁。
        assert_eq!(std::fs::read(&f.control).unwrap(), vec![0x52; 4096]);
    }

    #[test]
    fn held_controller_lock_is_pinned_and_physically_counted_without_relocking() {
        control_lock_is_measured(true);
    }
    #[test]
    fn closed_controller_fd_with_retained_path_is_pinned_and_physically_counted() {
        control_lock_is_measured(false);
    }
    #[test]
    fn known_control_lock_does_not_allow_unknown_sidecars() {
        let f = Fixture::new();
        std::fs::write(&f.control, b"").unwrap();
        let consumed = f.consumed();
        std::fs::write(sidecar(&f.db, ".unknown"), b"unknown").unwrap();
        assert!(MeasuredDomain::measure(&consumed)
            .err()
            .unwrap()
            .contains("UnknownDatabaseSidecar"));
    }
    #[test]
    fn h1_ledger_cache_sidecars_are_pinned_and_physically_counted() {
        let f = Fixture::new();
        let original: Vec<_> = AUDIT_LEDGER_SUFFIXES
            .iter()
            .map(|suffix| {
                let path = sidecar(&f.db, suffix);
                std::fs::write(&path, [0x71; 4096]).unwrap();
                let stamp = Stamp::of(&std::fs::metadata(&path).unwrap());
                assert!(stamp.blocks > 0);
                (path, stamp)
            })
            .collect();
        let source = MeasuredDomain::measure(&f.consumed()).unwrap();
        let expected: u64 = source
            .members
            .iter()
            .map(|p| std::fs::symlink_metadata(p).unwrap().blocks() * 512)
            .sum();
        assert_eq!(source.existing.store_bytes, expected);
        assert_eq!(source.existing.staged_bytes, 0);
        for (path, stamp) in original {
            let cache = source.ledger_cache.borrow();
            let pinned = cache.iter().find(|p| p.path == path).unwrap();
            assert_eq!(pinned.stamp, stamp);
            assert_eq!(Stamp::of(&pinned.file.metadata().unwrap()), stamp);
        }
        // 台账的 WAL/SHM/journal 不冒充内容库介质；缓存任意字节不参与内容解释。
        assert_eq!(source.medium, Medium::MainOnly);
        assert!(source.wal_fd().unwrap().is_none());
        source.verify_before_open().unwrap();
    }
    #[test]
    fn h1_ledger_cache_registration_still_rejects_unknown_sidecars() {
        for suffix in [
            ".audit-ledger.sqlite-extra",
            ".audit-ledger.sqlite-wal-extra",
            ".audit-ledger.sqlite-shm-extra",
            ".audit-ledger.sqlite-journal-extra",
        ] {
            let f = Fixture::new();
            std::fs::write(sidecar(&f.db, ".audit-ledger.sqlite"), b"cache").unwrap();
            std::fs::write(sidecar(&f.db, suffix), b"unknown").unwrap();
            assert!(MeasuredDomain::measure(&f.consumed())
                .err()
                .unwrap()
                .contains("UnknownDatabaseSidecar"));
            assert_eq!(std::fs::read(sidecar(&f.db, suffix)).unwrap(), b"unknown");
        }
    }
    #[test]
    fn h1_ledger_cache_substitutes_are_rejected() {
        for suffix in AUDIT_LEDGER_SUFFIXES {
            for kind in ["symlink", "hardlink", "directory"] {
                let f = Fixture::new();
                let path = sidecar(&f.db, suffix);
                match kind {
                    "symlink" => std::os::unix::fs::symlink(&f.db, &path).unwrap(),
                    "hardlink" => {
                        // #1481：替身只能污染缓存；源 DB 多链会先被真实 loader 的 E0 拒绝。
                        let cache_source = f.root.join("cache-source");
                        std::fs::write(&cache_source, b"cache substitute").unwrap();
                        std::fs::hard_link(&cache_source, &path).unwrap();
                        assert_eq!(std::fs::metadata(&path).unwrap().nlink(), 2);
                    }
                    _ => std::fs::create_dir(&path).unwrap(),
                }
                assert_eq!(std::fs::metadata(&f.db).unwrap().nlink(), 1);
                let consumed = f.consumed();
                assert!(
                    MeasuredDomain::measure(&consumed).is_err(),
                    "{suffix}: {kind}"
                );
            }
        }
    }
    #[cfg(all(feature = "s_incremental", unix))]
    #[test]
    fn h1_ledger_cache_uses_original_store_cap_before_discard() {
        let f = Fixture::new();
        let consumed = super::super::admission_anchor::test_prepare_caps(
            &f.db,
            &[("max_store_bytes", 1024 * 1024)],
        )
        .consume()
        .unwrap();
        MeasuredDomain::measure(&consumed).unwrap();
        std::fs::write(
            sidecar(&f.db, ".audit-ledger.sqlite"),
            vec![0x73; 2 * 1024 * 1024],
        )
        .unwrap();
        assert!(MeasuredDomain::measure(&consumed)
            .err()
            .unwrap()
            .contains("ExistingAllocationExceeded"));
        assert!(sidecar(&f.db, ".audit-ledger.sqlite").exists());
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn h1_ledger_cache_replacement_or_new_member_is_rejected_before_discard() {
        for suffix in AUDIT_LEDGER_SUFFIXES {
            let f = Fixture::new();
            let path = sidecar(&f.db, suffix);
            std::fs::write(&path, b"old cache").unwrap();
            let source = MeasuredDomain::measure(&f.consumed()).unwrap();
            source.verify_cold_ledger_cache().unwrap();
            // 原地损坏是可丢弃缓存，内容不具有权威；身份替换仍必须拒绝。
            std::fs::write(&path, b"damaged cache").unwrap();
            source.verify_cold_ledger_cache().unwrap();
            let replacement = f.root.join("replacement");
            std::fs::write(&replacement, b"damaged cache").unwrap();
            std::fs::rename(&replacement, &path).unwrap();
            assert!(source.verify_cold_ledger_cache().is_err());

            let f = Fixture::new();
            let source = MeasuredDomain::measure(&f.consumed()).unwrap();
            std::fs::write(sidecar(&f.db, suffix), b"new cache").unwrap();
            assert!(source.verify_cold_ledger_cache().is_err());
        }
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn h1_ledger_cache_discard_releases_only_cache_pins_before_rebuild() {
        let f = Fixture::new();
        for suffix in AUDIT_LEDGER_SUFFIXES {
            std::fs::write(sidecar(&f.db, suffix), [0x74; 4096]).unwrap();
        }
        let source = MeasuredDomain::measure(&f.consumed()).unwrap();
        assert_eq!(source.ledger_cache.borrow().len(), 4);
        assert!(source.release_cold_ledger_cache().is_err());
        assert_eq!(source.ledger_cache.borrow().len(), 4);
        source.verify_cold_ledger_cache().unwrap();
        let content_pin_count = source.files.len();
        for suffix in AUDIT_LEDGER_SUFFIXES {
            std::fs::remove_file(sidecar(&f.db, suffix)).unwrap();
        }
        source.release_cold_ledger_cache().unwrap();
        assert!(source.ledger_cache.borrow().is_empty());
        assert_eq!(source.files.len(), content_pin_count);
        source.main_fd().unwrap();
        source.verify_runtime().unwrap();
    }
    #[cfg(feature = "s_incremental")]
    #[test]
    fn h1_ledger_cache_conversion_stamp_rejects_changed_main_or_wal_frames() {
        for changed in ["main-bytes", "main-inode", "wal-frames", "wal-inode"] {
            let f = Fixture::new();
            let source = MeasuredDomain::measure(&f.consumed()).unwrap();
            source.verify_cold_content().unwrap();
            // 用文件改动代表独占转换连接已经完成的合法主库头转换。
            std::fs::write(&f.db, b"authenticated controlled conversion").unwrap();
            std::fs::write(sidecar(&f.db, "-wal"), b"").unwrap();
            let handoff = source.capture_cold_conversion().unwrap();
            handoff.verify().unwrap();
            match changed {
                "main-bytes" => std::fs::write(&f.db, b"foreign main commit").unwrap(),
                "main-inode" => {
                    let replacement = f.root.join("replacement");
                    std::fs::write(&replacement, b"authenticated controlled conversion").unwrap();
                    std::fs::rename(replacement, &f.db).unwrap();
                }
                "wal-frames" => std::fs::write(sidecar(&f.db, "-wal"), [0x75; 33]).unwrap(),
                _ => {
                    let replacement = f.root.join("replacement");
                    std::fs::write(&replacement, b"").unwrap();
                    std::fs::rename(replacement, sidecar(&f.db, "-wal")).unwrap();
                }
            }
            assert!(handoff.verify().is_err(), "{changed}");
        }
    }
    #[test]
    fn control_lock_substitutes_are_rejected() {
        for kind in ["symlink", "hardlink", "directory"] {
            let f = Fixture::new();
            let consumed = f.consumed();
            match kind {
                "symlink" => std::os::unix::fs::symlink(&f.db, &f.control).unwrap(),
                "hardlink" => std::fs::hard_link(&f.db, &f.control).unwrap(),
                _ => std::fs::create_dir(&f.control).unwrap(),
            }
            assert!(MeasuredDomain::measure(&consumed).is_err(), "{kind}");
        }
    }
    #[test]
    fn same_bytes_replacement_cannot_replace_the_pinned_control_lock() {
        let f = Fixture::new();
        std::fs::write(&f.control, b"control").unwrap();
        let source = MeasuredDomain::measure(&f.consumed()).unwrap();
        let replacement = f.root.join("replacement");
        std::fs::write(&replacement, b"control").unwrap();
        std::fs::rename(&replacement, &f.control).unwrap();
        assert!(source.verify_before_open().is_err());
        assert!(source.verify_runtime().is_err());
    }
}
