//! #1527 I-1：真实文件域测量。只测分配/固定来源，不解释 SQLite 页、WAL 或 journal。
//! st_blocks*512 是 OS 报告的文件/目录已分配量；不是共享 extent 独占性或 inode 隐藏开销证明。
use super::admission_anchor::{ConsumedAdmission, Stage};
use newchan_rust::s_content::job::Existing;
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

impl MeasuredDomain {
    pub(super) fn measure(consumed: &ConsumedAdmission) -> Result<Self, String> {
        consumed.check(Stage::E6).map_err(|e| e.to_string())?;
        let db = consumed.lease().db_path().to_owned();
        let state = consumed.lease().state_path().to_owned();
        // 完整枚举本 DB basename 家族；不忽略未知 super-journal、旧私有目录等残件。
        let members = Self::members(&db, &state, consumed)?;
        let mut files = Vec::new();
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
            files.push(pinned);
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
                        ".control-owner.json",
                        ".admission-private",
                    ]
                    .iter()
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
    pub(super) fn verify_before_open(&self) -> Result<(), String> {
        for p in &self.files {
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
    /// 原件开始合法写入后不能再要求旧 mtime/长度；只核原主库和固定目录身份。
    /// 此门不证明 sidecar 的跨开库连续性（I-3）。
    pub(super) fn verify_runtime(&self) -> Result<(), String> {
        self.files[self.main_index].verify(false)?;
        for p in &self.files {
            if p.directory {
                p.verify(false)?;
            }
        }
        Ok(())
    }
}
