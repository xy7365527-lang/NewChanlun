//! #1419 M8D 3B：SQLite 同父分配前收费（用户 2026-11-05 批准的隔离候选；3C 前无生产调用点）。
//!
//! 进程全局 SQLite allocator，只能在本进程首次 SQLite 初始化之前安装；初始化后 SQLite 以
//! `SQLITE_MISUSE` 拒绝配置，因此不存在热安装。每次 xMalloc/xRealloc 先按完整块 Layout
//! （块头 + 载荷 + 对齐）向当前绑定的原 Native 父账预留，成功后才向系统申请；块头记录父槽、
//! 代号与 origin，释放按原 origin 返还。未绑定、绑定过期、父账破损或预留被拒一律返回空指针，
//! 从不旁路到未计费的 malloc。回调只做短锁记账与系统 alloc/copy/free，不重入 SQLite，
//! 不分配 Rust 容器。
//!
//! 操作域：收费只能挡住新分配，lookaside 与已物化的值不再申请内存，所以连接归属与首拒后零
//! SQL 由执行边界保证。
//! - 连接只能经 [`SqliteBinding::open`] 开出，终身归属开它的父账（[`ChargedConnection`]
//!   不暴露裸连接）；只能经 [`SqliteBinding::run`] / [`SqliteBinding::close`] 使用。
//! - `run` 只交出 [`SqlOps`]，不交出 `rusqlite::Connection`。每次进入 C API 的操作
//!   （prepare、execute、query_row、语句的 execute / query_row）之前都重新核：本绑定为当前
//!   绑定、父账当前冻结域仍是本绑定的 origin、连接归属本父账、本父账本 origin 尚无拒绝；
//!   任一不符则该操作不进入 SQLite。同一时刻只有一个 `run`，期间禁止换绑与再入。
//!   [`SqlOps::handle`] 是 `unsafe` 逃生口，调用方须维持本合同（不改进度回调、不关闭连接）。
//! - 只有正处于 `run` 中的线程能让 allocator 收费，其他线程或域外的分配一律按未知归属拒绝。
//! - 首个拒绝按（父账、origin）记，同父同 origin 重绑不清除；只有换到新冻结域才开新记录。
//! - 每条连接另装 SQLite 进度回调（每 1 条 VM 指令计数），在循环底部、每次返回行和语句结束
//!   前核对归属与首拒，不符即以 `SQLITE_INTERRUPT` 中止，覆盖不经新分配的行与写。
//! - realloc 只在块的原父账为当前绑定时进行；跨父 realloc 在收费前拒绝。
use super::scratch::ScratchFailure;
use rusqlite::ffi;
use std::alloc::{alloc, dealloc, Layout};
use std::cell::Cell;
use std::ffi::c_void;
use std::path::Path;
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
    /// `origin` 仍是父账当前冻结域且父账未破损。
    fn admits_sqlite(&self, origin: u64) -> bool;
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
    /// 本父账某冻结域（origin）的首个收费拒绝；只在换到另一 origin 后被新拒绝取代。
    refusal: Option<(u64, ScratchFailure)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Owner {
    slot: usize,
    generation: u32,
}

#[derive(Clone, Copy)]
struct Binding {
    owner: Owner,
    origin: u64,
    epoch: u64,
}

struct State {
    installed: bool,
    broken: bool,
    slots: [Slot; SLOTS],
    current: Option<Binding>,
    epoch: u64,
    unattributed: usize,
}

const EMPTY: Slot = Slot {
    parent: None,
    generation: 0,
    live: 0,
    blocks: 0,
    refusal: None,
};
static STATE: Mutex<State> = Mutex::new(State {
    installed: false,
    broken: false,
    slots: [EMPTY; SLOTS],
    current: None,
    epoch: 0,
    unattributed: 0,
});
static INSTALLING: AtomicBool = AtomicBool::new(false);
/// 某个 `run` 正在执行；期间拒绝换绑与再入。
static RUNNING: AtomicBool = AtomicBool::new(false);
thread_local! {
    /// 本线程正处于 `run`；只有它的 SQLite 分配可以收费。
    static IN_RUN: Cell<bool> = const { Cell::new(false) };
}

fn state() -> Option<MutexGuard<'static, State>> {
    STATE.lock().ok()
}

fn in_run() -> bool {
    IN_RUN.try_with(Cell::get).unwrap_or(false)
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
    fn first_refusal(&self, b: Binding) -> Option<ScratchFailure> {
        self.slots[b.owner.slot]
            .refusal
            .filter(|(origin, _)| *origin == b.origin)
            .map(|(_, f)| f)
    }
    /// `owner` 的连接此刻可执行：父账为当前绑定、父账当前域仍是绑定 origin、
    /// 该冻结域无拒绝、allocator 未破损。
    fn admits(&self, owner: Owner) -> bool {
        !self.broken
            && self.current.is_some_and(|b| {
                b.owner == owner && self.first_refusal(b).is_none() && self.origin_live(b)
            })
    }
    fn origin_live(&self, b: Binding) -> bool {
        let sl = &self.slots[b.owner.slot];
        sl.generation == b.owner.generation
            && sl
                .parent
                .as_deref()
                .is_some_and(|p| p.admits_sqlite(b.origin))
    }
    fn latch(&mut self, b: Binding, failure: ScratchFailure) {
        let slot = &mut self.slots[b.owner.slot];
        if slot.refusal.is_none_or(|(origin, _)| origin != b.origin) {
            slot.refusal = Some((b.origin, failure));
        }
    }
    /// 未绑定且已无存活块的槽交还父句柄；调用方在解锁后析构返回值。
    fn retire(&mut self, slot: usize) -> Option<Arc<dyn SqliteParent>> {
        let bound = self.current.is_some_and(|b| b.owner.slot == slot);
        let s = &mut self.slots[slot];
        if !bound && s.blocks == 0 {
            s.refusal = None;
            s.parent.take()
        } else {
            None
        }
    }
}

unsafe extern "C" fn malloc(n: i32) -> *mut c_void {
    unsafe { allocate(n) }
}

/// 当前绑定且处于 `run` 线程时返回绑定；否则按未知归属记数并拒绝。
fn attributed(s: &mut State) -> Option<Binding> {
    if s.broken {
        return None;
    }
    match s.current {
        Some(b) if in_run() => Some(b),
        _ => {
            s.unattributed = s.unattributed.saturating_add(1);
            None
        }
    }
}

unsafe fn allocate(n: i32) -> *mut c_void {
    if n <= 0 {
        return ptr::null_mut();
    }
    let Some(mut s) = state() else {
        return ptr::null_mut();
    };
    let Some(b) = attributed(&mut s) else {
        return ptr::null_mut();
    };
    unsafe { allocate_for(&mut s, b, n as usize) }
}

unsafe fn allocate_for(s: &mut State, b: Binding, payload: usize) -> *mut c_void {
    if s.first_refusal(b).is_some() {
        return ptr::null_mut();
    }
    let Some(layout) = block_layout(payload) else {
        s.latch(b, ScratchFailure::Ledger);
        return ptr::null_mut();
    };
    let total = layout.size();
    let slot = b.owner.slot;
    let reserved = match s.slots[slot].parent.as_deref() {
        Some(parent) if s.slots[slot].generation == b.owner.generation => {
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
        s.latch(b, failure);
        return ptr::null_mut();
    }
    // SAFETY: valid non-zero Layout; the full block is already charged to the parent.
    let base = unsafe { alloc(layout) };
    if base.is_null() {
        let released = s.slots[slot]
            .parent
            .as_deref()
            .map(|parent| parent.release_sqlite(total, b.origin));
        if !matches!(released, Some(Ok(()))) {
            s.broken = true;
        }
        s.latch(b, ScratchFailure::Allocation);
        return ptr::null_mut();
    }
    let sl = &mut s.slots[slot];
    let (Some(live), Some(blocks)) = (sl.live.checked_add(total), sl.blocks.checked_add(1)) else {
        // SAFETY: base was just allocated with this layout and never handed out.
        unsafe { dealloc(base, layout) };
        if let Some(parent) = sl.parent.as_deref() {
            let _ = parent.release_sqlite(total, b.origin);
            parent.invalidate_sqlite();
        }
        s.broken = true;
        return ptr::null_mut();
    };
    sl.live = live;
    sl.blocks = blocks;
    // SAFETY: base is aligned for Header and the block is large enough for it.
    unsafe {
        base.cast::<Header>().write(Header {
            payload,
            total,
            origin: b.origin,
            slot: slot as u32,
            generation: b.owner.generation,
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
    let Some(mut s) = state() else {
        return ptr::null_mut();
    };
    let Some((_, h, _)) = (unsafe { header(p) }) else {
        s.broken = true;
        return ptr::null_mut();
    };
    let Some(b) = attributed(&mut s) else {
        return ptr::null_mut();
    };
    // 跨父 realloc 在任何收费之前拒绝：块属于别的父账（或已退役的同槽旧代）。
    if b.owner.slot != h.slot as usize || b.owner.generation != h.generation {
        s.latch(b, ScratchFailure::Ledger);
        return ptr::null_mut();
    }
    // 新块在旧块仍存活时完整预留；失败时旧块与其收费都保持不变。
    let grown = unsafe { allocate_for(&mut s, b, n as usize) };
    drop(s);
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

    /// 把此后的 SQLite 操作域绑定到 `parent` 的 `origin` 冻结域，取代先前的绑定。
    /// 先前绑定已收费的块仍按各自 origin 返还原父账。`run` 执行中拒绝换绑。
    /// 同父同 origin 重绑沿用该冻结域的首个拒绝，不清除。
    pub fn bind(
        &self,
        parent: Arc<dyn SqliteParent>,
        origin: u64,
    ) -> Result<SqliteBinding, ScratchFailure> {
        if RUNNING.load(Ordering::Acquire) {
            return Err(ScratchFailure::Ledger);
        }
        let mut s = state().ok_or(ScratchFailure::Ledger)?;
        if !s.installed || s.broken || RUNNING.load(Ordering::Acquire) {
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
                sl.refusal = None;
                slot
            }
        };
        let epoch = s.epoch.checked_add(1).ok_or(ScratchFailure::Ledger)?;
        let owner = Owner {
            slot,
            generation: s.slots[slot].generation,
        };
        let previous = s.current.replace(Binding {
            owner,
            origin,
            epoch,
        });
        s.epoch = epoch;
        let retired = previous
            .filter(|p| p.owner.slot != slot)
            .and_then(|p| s.retire(p.owner.slot));
        drop(s);
        drop(retired);
        Ok(SqliteBinding {
            epoch,
            owner,
            origin,
        })
    }

    /// 无归属（未绑定、不在 `run` 线程）时被拒的 SQLite 分配次数。
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
    owner: Owner,
    origin: u64,
}

impl SqliteBinding {
    /// 本父账本冻结域的首个收费拒绝。一旦拒绝，此域内一切分配与 SQL 执行均被拒，换绑同域不清除。
    pub fn refusal(&self) -> Option<ScratchFailure> {
        let s = state()?;
        let sl = &s.slots[self.owner.slot];
        if sl.parent.is_none() || sl.generation != self.owner.generation {
            return None;
        }
        sl.refusal
            .filter(|(origin, _)| *origin == self.origin)
            .map(|(_, f)| f)
    }

    /// SQLite 报 `SQLITE_NOMEM` 且本域已记收费拒绝时，返回该首个拒绝。
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
        let sl = &s.slots[self.owner.slot];
        (sl.parent.is_some() && sl.generation == self.owner.generation).then_some(sl)
    }

    /// 进入操作域：独占 `run`，核本绑定为当前、本域无拒绝、连接归属本父账。
    fn enter(&self, conn: Option<&ChargedConnection>) -> Result<Running, SqlGate> {
        if RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(SqlGate::Refused(ScratchFailure::Ledger));
        }
        let running = Running(());
        self.admit(conn.map(|c| c.owner))?;
        IN_RUN.with(|r| r.set(true));
        Ok(running)
    }

    /// 本绑定为当前绑定、父账当前域仍是本 origin、本域无拒绝、`owner` 归属本父账。
    fn admit(&self, owner: Option<Owner>) -> Result<(), SqlGate> {
        let s = state().ok_or(SqlGate::Refused(ScratchFailure::Ledger))?;
        let current = s.current.filter(|b| b.epoch == self.epoch);
        let Some(b) = current.filter(|_| !s.broken) else {
            return Err(SqlGate::Refused(ScratchFailure::Ledger));
        };
        if let Some(first) = s.first_refusal(b) {
            return Err(SqlGate::Refused(first));
        }
        if !s.origin_live(b) {
            return Err(SqlGate::Refused(ScratchFailure::Ledger));
        }
        if owner.is_some_and(|o| o != self.owner) {
            return Err(SqlGate::WrongParent);
        }
        Ok(())
    }

    /// 首拒优先：本域执行中已记收费拒绝时，一律报该首个拒绝（即使 SQLite 层吞掉了错误）。
    fn settle<R>(&self, result: rusqlite::Result<R>) -> Result<R, SqlGate> {
        if let Some(first) = self.refusal() {
            return Err(SqlGate::Refused(first));
        }
        result.map_err(SqlGate::Sqlite)
    }

    /// 在本父账名下打开连接；连接终身归属本父账，并装执行边界的进度回调。
    pub fn open(&self, path: impl AsRef<Path>) -> Result<ChargedConnection, SqlGate> {
        self.open_with(|| rusqlite::Connection::open(path))
    }

    /// 同 [`Self::open`]，内存库。
    pub fn open_in_memory(&self) -> Result<ChargedConnection, SqlGate> {
        self.open_with(rusqlite::Connection::open_in_memory)
    }

    fn open_with(
        &self,
        open: impl FnOnce() -> rusqlite::Result<rusqlite::Connection>,
    ) -> Result<ChargedConnection, SqlGate> {
        let running = self.enter(None)?;
        let owner = self.owner;
        let opened = open().and_then(|conn| {
            conn.progress_handler(1, Some(move || state().is_none_or(|s| !s.admits(owner))))?;
            Ok(conn)
        });
        drop(running);
        self.settle(opened)
            .map(|conn| ChargedConnection { conn, owner })
    }

    /// 在本操作域执行 `f`。连接须归属本父账、本绑定须为当前、本域须无拒绝，否则 `f` 不执行；
    /// `f` 内每次操作再各自核一次。首个收费拒绝之后，后续操作不再进入 SQLite，结果报首拒。
    pub fn run<R>(
        &self,
        conn: &ChargedConnection,
        f: impl FnOnce(&SqlOps<'_>) -> Result<R, SqlGate>,
    ) -> Result<R, SqlGate> {
        let running = self.enter(Some(conn))?;
        let ops = SqlOps {
            conn: &conn.conn,
            binding: self,
            owner: conn.owner,
        };
        let result = f(&ops);
        drop(running);
        match self.refusal() {
            Some(first) => Err(SqlGate::Refused(first)),
            None => result,
        }
    }

    /// 在本操作域关闭归属本父账的连接；不符时原样交回连接。
    pub fn close(&self, conn: ChargedConnection) -> Result<(), (ChargedConnection, SqlGate)> {
        let running = match self.enter(Some(&conn)) {
            Ok(running) => running,
            Err(e) => return Err((conn, e)),
        };
        let owner = conn.owner;
        let result = conn.conn.close();
        drop(running);
        match result {
            Ok(()) => Ok(()),
            Err((conn, e)) => Err((ChargedConnection { conn, owner }, SqlGate::Sqlite(e))),
        }
    }
}

impl Drop for SqliteBinding {
    fn drop(&mut self) {
        let Some(mut s) = state() else {
            return;
        };
        if s.current.is_some_and(|b| b.epoch == self.epoch) {
            s.current = None;
            let retired = s.retire(self.owner.slot);
            drop(s);
            drop(retired);
        }
    }
}

struct Running(());

impl Drop for Running {
    fn drop(&mut self) {
        let _ = IN_RUN.try_with(|r| r.set(false));
        RUNNING.store(false, Ordering::Release);
    }
}

/// 操作域边界的拒绝。`Refused` 携本域首个收费拒绝；绑定非当前、已有 `run` 在执行或
/// allocator 破损时为 `Refused(Ledger)`。
#[derive(Debug)]
pub enum SqlGate {
    Refused(ScratchFailure),
    /// 连接归属另一父账，未执行任何 SQL。
    WrongParent,
    Sqlite(rusqlite::Error),
}

/// 归属开它的父账的 SQLite 连接，不暴露裸连接；只能经 [`SqliteBinding::run`] 使用。
/// 未经 [`SqliteBinding::close`] 直接析构时仍关闭连接，块按各自块头返还原父账，
/// 关闭期间的新分配因不在操作域而被拒。
#[derive(Debug)]
pub struct ChargedConnection {
    conn: rusqlite::Connection,
    owner: Owner,
}

/// `run` 内的 SQL 操作口。每个方法进入 SQLite 前重核操作域（绑定、父账当前域、首拒、归属）。
pub struct SqlOps<'a> {
    conn: &'a rusqlite::Connection,
    binding: &'a SqliteBinding,
    owner: Owner,
}

impl<'a> SqlOps<'a> {
    fn gate<T>(&self, op: impl FnOnce() -> rusqlite::Result<T>) -> Result<T, SqlGate> {
        self.binding.admit(Some(self.owner))?;
        self.binding.settle(op())
    }

    pub fn execute_batch(&self, sql: &str) -> Result<(), SqlGate> {
        self.gate(|| self.conn.execute_batch(sql))
    }

    pub fn execute<P: rusqlite::Params>(&self, sql: &str, params: P) -> Result<usize, SqlGate> {
        self.gate(|| self.conn.execute(sql, params))
    }

    pub fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> Result<T, SqlGate>
    where
        P: rusqlite::Params,
        F: FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    {
        self.gate(|| self.conn.query_row(sql, params, f))
    }

    pub fn prepare(&self, sql: &str) -> Result<GatedStatement<'a>, SqlGate> {
        let conn = self.conn;
        let stmt = self.gate(|| conn.prepare(sql))?;
        Ok(GatedStatement {
            stmt,
            binding: self.binding,
            owner: self.owner,
        })
    }

    /// # Safety
    /// 只用于本操作域内的只读诊断与裸语句操作；不得更改或移除进度回调、不得关闭连接、
    /// 不得把句柄带出本 `run`。
    pub unsafe fn handle(&self) -> *mut ffi::sqlite3 {
        unsafe { self.conn.handle() }
    }
}

/// 经 [`SqlOps::prepare`] 得到的语句；每次执行前重核操作域。
pub struct GatedStatement<'a> {
    stmt: rusqlite::Statement<'a>,
    binding: &'a SqliteBinding,
    owner: Owner,
}

impl GatedStatement<'_> {
    pub fn execute<P: rusqlite::Params>(&mut self, params: P) -> Result<usize, SqlGate> {
        self.binding.admit(Some(self.owner))?;
        self.binding.settle(self.stmt.execute(params))
    }

    pub fn query_row<T, P, F>(&mut self, params: P, f: F) -> Result<T, SqlGate>
    where
        P: rusqlite::Params,
        F: FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    {
        self.binding.admit(Some(self.owner))?;
        self.binding.settle(self.stmt.query_row(params, f))
    }
}
