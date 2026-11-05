//! #1419 M8D 3B：SQLite 同父分配前收费（用户 2026-11-05 批准的隔离候选；3C 前无生产调用点）。
//!
//! 进程全局 SQLite allocator，只能在本进程首次 SQLite 初始化之前安装；初始化后 SQLite 以
//! `SQLITE_MISUSE` 拒绝配置，因此不存在热安装。每次 xMalloc/xRealloc 先按完整块 Layout
//! （块头 + 载荷 + 对齐）向当前绑定的原 Native 父账预留，成功后才向系统申请；块头记录父槽、
//! 代号与 origin，释放按原 origin 返还。未绑定、绑定过期、父账破损或预留被拒一律返回空指针，
//! 从不旁路到未计费的 malloc。回调只做短锁记账与系统 alloc/copy/free，不重入 SQLite，
//! 不分配 Rust 容器。
use super::scratch::ScratchFailure;
use rusqlite::ffi;
use std::alloc::{alloc, dealloc, Layout};
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

/// 原 Native 父账面向 SQLite 的收费口。实现方必须与 Rust 许可共用同一总账、同一冻结域。
pub trait SqliteParent: Send + Sync {
    /// 在 `expected` 冻结域内原子预留 `bytes`。域已换或账破损返回 `Ledger`，额度不足返回 `Budget`。
    fn reserve_sqlite(&self, bytes: usize, expected: u64) -> Result<u64, ScratchFailure>;
    /// 按块记录的原 origin 返还；旧 origin 只降累计占用，不回补当前域额度。
    fn release_sqlite(&self, bytes: usize, origin: u64) -> Result<(), ScratchFailure>;
    /// 块头或槽账不一致时标记父账破损。
    fn invalidate_sqlite(&self);
}

const SLOTS: usize = 8;
const MAGIC: u64 = 0x1419_5351_5041_5254;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Header {
    payload: usize,
    total: usize,
    origin: u64,
    slot: u32,
    generation: u32,
    magic: u64,
}

struct Slot {
    parent: Option<Arc<dyn SqliteParent>>,
    generation: u32,
    live: usize,
    blocks: usize,
}

#[derive(Clone, Copy)]
struct Binding {
    slot: usize,
    generation: u32,
    origin: u64,
    epoch: u64,
}

struct State {
    installed: bool,
    broken: bool,
    slots: [Slot; SLOTS],
    current: Option<Binding>,
    epoch: u64,
    refusal: Option<(u64, ScratchFailure)>,
    unattributed: usize,
}

const EMPTY: Slot = Slot {
    parent: None,
    generation: 0,
    live: 0,
    blocks: 0,
};
static STATE: Mutex<State> = Mutex::new(State {
    installed: false,
    broken: false,
    slots: [EMPTY; SLOTS],
    current: None,
    epoch: 0,
    refusal: None,
    unattributed: 0,
});
static INSTALLING: AtomicBool = AtomicBool::new(false);

fn state() -> Option<MutexGuard<'static, State>> {
    STATE.lock().ok()
}

fn block_layout(payload: usize) -> Option<Layout> {
    let bytes = std::mem::size_of::<Header>().checked_add(payload)?;
    Some(
        Layout::from_size_align(bytes, std::mem::align_of::<Header>())
            .ok()?
            .pad_to_align(),
    )
}

/// 一个 `payload` 字节的 SQLite 块向父账收取的完整字节数（含块头与对齐）。
pub fn charged_bytes(payload: usize) -> Option<usize> {
    block_layout(payload).map(|l| l.size())
}

fn parent_key(parent: &Arc<dyn SqliteParent>) -> *const () {
    Arc::as_ptr(parent).cast::<()>()
}

impl State {
    fn latch(&mut self, epoch: u64, failure: ScratchFailure) {
        if self.refusal.is_none_or(|(e, _)| e != epoch) {
            self.refusal = Some((epoch, failure));
        }
    }
    /// 未绑定且已无存活块的槽交还父句柄；调用方在解锁后析构返回值。
    fn retire(&mut self, slot: usize) -> Option<Arc<dyn SqliteParent>> {
        let bound = self.current.is_some_and(|b| b.slot == slot);
        let s = &mut self.slots[slot];
        if !bound && s.blocks == 0 {
            s.parent.take()
        } else {
            None
        }
    }
}

unsafe extern "C" fn malloc(n: i32) -> *mut c_void {
    unsafe { allocate(n) }
}

unsafe fn allocate(n: i32) -> *mut c_void {
    if n <= 0 {
        return ptr::null_mut();
    }
    let Some(mut s) = state() else {
        return ptr::null_mut();
    };
    if s.broken {
        return ptr::null_mut();
    }
    let Some(b) = s.current else {
        s.unattributed = s.unattributed.saturating_add(1);
        return ptr::null_mut();
    };
    if s.refusal.is_some_and(|(e, _)| e == b.epoch) {
        return ptr::null_mut();
    }
    let payload = n as usize;
    let Some(layout) = block_layout(payload) else {
        s.latch(b.epoch, ScratchFailure::Ledger);
        return ptr::null_mut();
    };
    let total = layout.size();
    let reserved = match s.slots[b.slot].parent.as_deref() {
        Some(parent) if s.slots[b.slot].generation == b.generation => {
            match parent.reserve_sqlite(total, b.origin) {
                Ok(origin) if origin == b.origin => Ok(()),
                Ok(origin) => {
                    let _ = parent.release_sqlite(total, origin);
                    Err(ScratchFailure::Ledger)
                }
                Err(failure) => Err(failure),
            }
        }
        _ => Err(ScratchFailure::Ledger),
    };
    if let Err(failure) = reserved {
        s.latch(b.epoch, failure);
        return ptr::null_mut();
    }
    // SAFETY: valid non-zero Layout; the full block is already charged to the parent.
    let base = unsafe { alloc(layout) };
    if base.is_null() {
        let released = s.slots[b.slot]
            .parent
            .as_deref()
            .map(|parent| parent.release_sqlite(total, b.origin));
        if !matches!(released, Some(Ok(()))) {
            s.broken = true;
        }
        s.latch(b.epoch, ScratchFailure::Allocation);
        return ptr::null_mut();
    }
    let slot = &mut s.slots[b.slot];
    let (Some(live), Some(blocks)) = (slot.live.checked_add(total), slot.blocks.checked_add(1))
    else {
        // SAFETY: base was just allocated with this layout and never handed out.
        unsafe { dealloc(base, layout) };
        if let Some(parent) = slot.parent.as_deref() {
            let _ = parent.release_sqlite(total, b.origin);
            parent.invalidate_sqlite();
        }
        s.broken = true;
        return ptr::null_mut();
    };
    slot.live = live;
    slot.blocks = blocks;
    // SAFETY: base is aligned for Header and the block is large enough for it.
    unsafe {
        base.cast::<Header>().write(Header {
            payload,
            total,
            origin: b.origin,
            slot: b.slot as u32,
            generation: b.generation,
            magic: MAGIC,
        });
        base.add(std::mem::size_of::<Header>()).cast()
    }
}

/// SQLite 只交回本 allocator 发出的指针；块头不一致即视为账目损坏。
unsafe fn header(p: *mut c_void) -> Option<(*mut u8, Header, Layout)> {
    if p.is_null() {
        return None;
    }
    // SAFETY: every pointer SQLite passes here was returned by `allocate`.
    let base = unsafe { p.cast::<u8>().sub(std::mem::size_of::<Header>()) };
    let h = unsafe { base.cast::<Header>().read() };
    if h.magic != MAGIC || h.payload > i32::MAX as usize || h.slot as usize >= SLOTS {
        return None;
    }
    let layout = block_layout(h.payload)?;
    (layout.size() == h.total).then_some((base, h, layout))
}

unsafe extern "C" fn free(p: *mut c_void) {
    if p.is_null() {
        return;
    }
    let Some(mut s) = state() else {
        return;
    };
    let Some((base, h, layout)) = (unsafe { header(p) }) else {
        s.broken = true;
        return;
    };
    let slot = h.slot as usize;
    let valid = {
        let sl = &s.slots[slot];
        sl.parent.is_some() && sl.generation == h.generation && sl.blocks > 0 && sl.live >= h.total
    };
    if !valid {
        if let Some(parent) = s.slots[slot].parent.as_deref() {
            parent.invalidate_sqlite();
        }
        s.broken = true;
        return;
    }
    // SAFETY: the block came from `allocate` with exactly this layout and SQLite is its sole owner.
    unsafe { dealloc(base, layout) };
    let sl = &mut s.slots[slot];
    sl.live -= h.total;
    sl.blocks -= 1;
    if let Some(parent) = sl.parent.as_deref() {
        // 父账自己在返还失败时标记破损；块已交还系统，不再重复计费。
        let _ = parent.release_sqlite(h.total, h.origin);
    }
    let retired = s.retire(slot);
    drop(s);
    drop(retired);
}

unsafe extern "C" fn realloc(p: *mut c_void, n: i32) -> *mut c_void {
    if p.is_null() {
        return unsafe { allocate(n) };
    }
    if n <= 0 {
        unsafe { free(p) };
        return ptr::null_mut();
    }
    let Some((_, h, _)) = (unsafe { header(p) }) else {
        if let Some(mut s) = state() {
            s.broken = true;
        }
        return ptr::null_mut();
    };
    // 新块在旧块仍存活时完整预留；失败时旧块与其收费都保持不变。
    let grown = unsafe { allocate(n) };
    if grown.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: both blocks are live and distinct; copy the smaller payload.
    unsafe {
        ptr::copy_nonoverlapping(
            p.cast::<u8>(),
            grown.cast::<u8>(),
            h.payload.min(n as usize),
        );
        free(p);
    }
    grown
}

unsafe extern "C" fn size(p: *mut c_void) -> i32 {
    match unsafe { header(p) } {
        Some((_, h, _)) => h.payload as i32,
        None => 0,
    }
}

unsafe extern "C" fn roundup(n: i32) -> i32 {
    if n <= 0 {
        return 0;
    }
    n.checked_add(7).map(|v| v & !7).unwrap_or(0)
}

unsafe extern "C" fn init(_: *mut c_void) -> i32 {
    ffi::SQLITE_OK
}

unsafe extern "C" fn shutdown(_: *mut c_void) {}

fn sql_error(code: i32) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(ffi::Error::new(code), None)
}

/// 已安装的同父收费 allocator。整个进程生命周期有效，不恢复原 allocator。
pub struct SqliteParentAllocator(());

impl SqliteParentAllocator {
    /// # Safety
    /// 必须在本进程任何 SQLite 初始化（含 rusqlite 打开连接）之前、且无其他线程调用 SQLite 时调用。
    /// SQLite 已初始化时返回 `SQLITE_MISUSE`，不改变任何分配器。
    pub unsafe fn install() -> Result<Self, rusqlite::Error> {
        if INSTALLING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(sql_error(ffi::SQLITE_MISUSE));
        }
        let methods = ffi::sqlite3_mem_methods {
            xMalloc: Some(malloc),
            xFree: Some(free),
            xRealloc: Some(realloc),
            xSize: Some(size),
            xRoundup: Some(roundup),
            xInit: Some(init),
            xShutdown: Some(shutdown),
            pAppData: ptr::null_mut(),
        };
        // SAFETY: caller guarantees no concurrent SQLite use; SQLite copies the method table.
        let code = unsafe { ffi::sqlite3_config(ffi::SQLITE_CONFIG_MALLOC, &methods) };
        if code != ffi::SQLITE_OK {
            INSTALLING.store(false, Ordering::Release);
            return Err(sql_error(code));
        }
        match state() {
            Some(mut s) => {
                s.installed = true;
                Ok(Self(()))
            }
            None => Err(sql_error(ffi::SQLITE_INTERNAL)),
        }
    }

    /// 把此后的 SQLite 分配绑定到 `parent` 的 `origin` 冻结域，取代先前的绑定。
    /// 先前绑定已收费的块仍按各自 origin 返还原父账。
    pub fn bind(
        &self,
        parent: Arc<dyn SqliteParent>,
        origin: u64,
    ) -> Result<SqliteBinding, ScratchFailure> {
        let mut s = state().ok_or(ScratchFailure::Ledger)?;
        if !s.installed || s.broken {
            return Err(ScratchFailure::Ledger);
        }
        let key = parent_key(&parent);
        let slot = match s
            .slots
            .iter()
            .position(|sl| sl.parent.as_ref().is_some_and(|p| parent_key(p) == key))
        {
            Some(slot) => slot,
            None => {
                let slot = s
                    .slots
                    .iter()
                    .position(|sl| sl.parent.is_none())
                    .ok_or(ScratchFailure::Ledger)?;
                let sl = &mut s.slots[slot];
                sl.generation = sl.generation.checked_add(1).ok_or(ScratchFailure::Ledger)?;
                sl.parent = Some(parent);
                slot
            }
        };
        let epoch = s.epoch.checked_add(1).ok_or(ScratchFailure::Ledger)?;
        let generation = s.slots[slot].generation;
        let previous = s.current.replace(Binding {
            slot,
            generation,
            origin,
            epoch,
        });
        s.epoch = epoch;
        let retired = previous
            .filter(|p| p.slot != slot)
            .and_then(|p| s.retire(p.slot));
        let binding = SqliteBinding {
            epoch,
            slot,
            generation,
        };
        drop(s);
        drop(retired);
        Ok(binding)
    }

    /// 无绑定时被拒的 SQLite 分配次数（未知归属）。
    pub fn unattributed_refusals(&self) -> usize {
        state().map_or(usize::MAX, |s| s.unattributed)
    }

    /// 块头或槽账曾不一致；此后一切分配被拒。
    pub fn is_broken(&self) -> bool {
        state().is_none_or(|s| s.broken)
    }
}

/// 当前收费绑定。析构时若仍为当前绑定则解除，此后的分配因无归属而被拒。
pub struct SqliteBinding {
    epoch: u64,
    slot: usize,
    generation: u32,
}

impl SqliteBinding {
    /// 本绑定首个收费拒绝。一旦拒绝，本绑定此后的全部分配均被拒，直至换绑。
    pub fn refusal(&self) -> Option<ScratchFailure> {
        state()?
            .refusal
            .filter(|(e, _)| *e == self.epoch)
            .map(|(_, f)| f)
    }

    /// SQLite 报 `SQLITE_NOMEM` 且本绑定已记收费拒绝时，返回该首个拒绝。
    pub fn refusal_of(&self, error: &rusqlite::Error) -> Option<ScratchFailure> {
        if error.sqlite_error_code() == Some(rusqlite::ErrorCode::OutOfMemory) {
            self.refusal()
        } else {
            None
        }
    }

    /// 本父账名下存活的 SQLite 块完整字节数（全部 origin 合计）。
    pub fn live_bytes(&self) -> usize {
        state().map_or(usize::MAX, |s| self.slot_of(&s).map_or(0, |sl| sl.live))
    }

    /// 本父账名下存活的 SQLite 块数。
    pub fn live_blocks(&self) -> usize {
        state().map_or(usize::MAX, |s| self.slot_of(&s).map_or(0, |sl| sl.blocks))
    }

    fn slot_of<'a>(&self, s: &'a State) -> Option<&'a Slot> {
        let sl = &s.slots[self.slot];
        (sl.parent.is_some() && sl.generation == self.generation).then_some(sl)
    }
}

impl Drop for SqliteBinding {
    fn drop(&mut self) {
        let Some(mut s) = state() else {
            return;
        };
        if s.current.is_some_and(|b| b.epoch == self.epoch) {
            s.current = None;
            let retired = s.retire(self.slot);
            drop(s);
            drop(retired);
        }
    }
}
