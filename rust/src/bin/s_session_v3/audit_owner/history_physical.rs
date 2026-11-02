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
pub(super) fn initial_progress(connection: &Connection) -> Result<Progress, String> {
    // Bundled SQLite支持NOOP：仅读取WAL前沿，不触发checkpoint或改变阈值。
    let (busy, frames, done): (i32, i32, i32) = connection
        .query_row("PRAGMA main.wal_checkpoint(NOOP)", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(stale)?;
    if busy != 0 {
        return Err(stale("无法读取初始WAL前沿"));
    }
    Progress::checked((frames, done))
}
pub(super) fn observe_commit(wal: &Wal, frames: i32) -> rusqlite::Result<()> {
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
                if wal.name().to_bytes() != b"main" {
                    return Err(stale("非main的WAL提交"));
                }
                let before = state.watch.files()?;
                let start =
                    Progress::checked(wal.checkpoint_v2(CheckpointMode::NOOP).map_err(stale)?)?;
                let checkpoint = state.watch.checkpoint_threshold > 0
                    && frames >= state.watch.checkpoint_threshold;
                let end = if checkpoint {
                    Progress::checked(wal.checkpoint_v2(CheckpointMode::PASSIVE).map_err(stale)?)?
                } else {
                    start
                };
                let after = state.watch.files()?;
                Ok(PhysicalCommit {
                    before,
                    after,
                    checkpoint,
                    start,
                    end,
                    frames,
                })
            })();
            state.result = Some(result);
        }
    });
    Ok(())
}
