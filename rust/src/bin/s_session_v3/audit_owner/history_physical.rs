//! #1481：记录本连接实际 COMMIT 与原阈值 PASSIVE checkpoint 的物理效果。
use super::*;
use rusqlite::hooks::{CheckpointMode, Wal};
thread_local! {
    static ACTIVE: RefCell<Option<Rc<RefCell<HookState>>>> = const { RefCell::new(None) };
}
struct HookState {
    watch: Clamp,
    result: Option<Result<PhysicalCommit, String>>,
}
pub(super) struct PhysicalCommit {
    pub(super) before: Files,
    pub(super) after: Files,
    pub(super) checkpoint: bool,
    pub(super) start: Progress,
    pub(super) end: Progress,
    pub(super) frames: i32,
}
pub(crate) struct CommitMonitor {
    state: Rc<RefCell<HookState>>,
}
impl CommitMonitor {
    pub(super) fn begin(clamp: &Clamp) -> Result<Self, String> {
        fn copy(file: &WatchedFile) -> Result<WatchedFile, String> {
            Ok(WatchedFile {
                path: file.path.clone(),
                file: file.file.try_clone().map_err(stale)?,
            })
        }
        let state = Rc::new(RefCell::new(HookState {
            watch: Clamp {
                main: copy(&clamp.main)?,
                wal: copy(&clamp.wal)?,
                baseline: None,
                parent: None,
                control_limit: 0,
                checkpoint_threshold: clamp.checkpoint_threshold,
                progress: clamp.progress,
                integrity_refusal: std::cell::Cell::new(clamp.integrity_refusal.get()),
                inherited_data_version: None,
            },
            result: None,
        }));
        ACTIVE.with(|slot| {
            if slot.borrow().is_some() {
                return Err(stale("已有提交物理观察"));
            }
            *slot.borrow_mut() = Some(state.clone());
            Ok(Self { state })
        })
    }
    pub(super) fn take(&self) -> Option<Result<PhysicalCommit, String>> {
        self.state.borrow_mut().result.take()
    }
}
impl Drop for CommitMonitor {
    fn drop(&mut self) {
        ACTIVE.with(|slot| {
            slot.borrow_mut().take();
        });
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Progress {
    pub(super) frames: i32,
    pub(super) backfill: i32,
}
impl Progress {
    pub(super) fn empty() -> Self {
        Self {
            frames: 0,
            backfill: 0,
        }
    }
    fn checked((frames, backfill): (i32, i32)) -> Result<Self, String> {
        if (frames, backfill) == (-1, -1) {
            return Ok(Self::empty());
        }
        if frames < 0 || backfill < 0 || backfill > frames {
            return Err(stale("无效WAL回填前沿"));
        }
        Ok(Self { frames, backfill })
    }
    pub(super) fn complete(self) -> bool {
        self.frames > 0 && self.frames == self.backfill
    }
}
pub(super) fn initial_progress(connection: &dyn OwnerSql) -> Result<Progress, String> {
    // Bundled SQLite支持NOOP：仅读取WAL前沿，不触发checkpoint或改变阈值。
    let (busy, frames, done) = connection
        .int3("PRAGMA main.wal_checkpoint(NOOP)")
        .map_err(stale)?;
    if busy != 0 {
        return Err(stale("无法读取初始WAL前沿"));
    }
    Progress::checked((frames, done))
}
pub(super) fn observe_commit(wal: &Wal, frames: i32) -> rusqlite::Result<()> {
    record_commit(wal.name().to_bytes() == b"main", frames, &mut |mode| {
        wal.checkpoint_v2(mode).map_err(stale)
    });
    Ok(())
}

/// #1419 3C U5：收费组件连接上的同一提交观察。回调内 checkpoint 经组件 `wal_checkpoint`，
/// 与普通操作同准入、首拒优先结算；判据与 [`observe_commit`] 共用 [`record_commit`]。目前只由
/// 测试编译的收费 owner 消费（见 `owner_sql` 收费一族的 cfg）。
#[cfg(test)]
pub(super) unsafe extern "C" fn observe_commit_charged(
    _: *mut std::ffi::c_void,
    db: *mut rusqlite::ffi::sqlite3,
    name: *const std::ffi::c_char,
    frames: std::ffi::c_int,
) -> std::ffi::c_int {
    use newchan_rust::s_content::capture_allocation::sqlite_parent::wal_checkpoint;
    // SAFETY: SQLite passes the attached database name as a NUL-terminated string.
    let is_main =
        !name.is_null() && unsafe { std::ffi::CStr::from_ptr(name) }.to_bytes() == b"main";
    record_commit(is_main, frames, &mut |mode| {
        let mode = match mode {
            CheckpointMode::NOOP => rusqlite::ffi::SQLITE_CHECKPOINT_NOOP,
            CheckpointMode::PASSIVE => rusqlite::ffi::SQLITE_CHECKPOINT_PASSIVE,
            _ => return Err(stale("提交观察只用 NOOP/PASSIVE")),
        };
        // SAFETY: db is the handle SQLite passed to this WAL hook.
        unsafe { wal_checkpoint(db, mode) }.map_err(|e| stale(format!("{e:?}")))
    });
    rusqlite::ffi::SQLITE_OK
}

fn record_commit(
    is_main: bool,
    frames: i32,
    checkpoint: &mut dyn FnMut(CheckpointMode) -> Result<(i32, i32), String>,
) {
    ACTIVE.with(|slot| {
        let state = slot.borrow().as_ref().cloned();
        if let Some(state) = state {
            let mut state = state.borrow_mut();
            if state.result.is_some() {
                state.result = Some(Err(stale("同一次COMMIT出现多个WAL回调")));
                return;
            }
            // checkpoint失败不篡改已完成的SQL COMMIT；收据失败由owner退役。
            let result = (|| {
                if !is_main {
                    return Err(stale("非main的WAL提交"));
                }
                let before = state.watch.files()?;
                let start = Progress::checked(checkpoint(CheckpointMode::NOOP)?)?;
                let run_checkpoint = state.watch.checkpoint_threshold > 0
                    && frames >= state.watch.checkpoint_threshold;
                let end = if run_checkpoint {
                    Progress::checked(checkpoint(CheckpointMode::PASSIVE)?)?
                } else {
                    start
                };
                let after = state.watch.files()?;
                Ok(PhysicalCommit {
                    before,
                    after,
                    checkpoint: run_checkpoint,
                    start,
                    end,
                    frames,
                })
            })();
            state.result = Some(result);
        }
    });
}
