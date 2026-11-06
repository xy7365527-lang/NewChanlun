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
//! - 连接只能经 [`SqliteBinding::open`] / [`SqliteBinding::open_with`] 开出，终身归属开它的
//!   父账（[`ChargedConnection`] 不暴露裸连接）；只能经 [`SqliteBinding::run`] 及收尾的
//!   [`SqliteBinding::rollback`] / [`SqliteBinding::close`] 使用。
//! - 连接不可跨线程（`!Send`、`!Sync`，编译期断言）：SQLite 只在开库线程上同步调用调用方的
//!   授权与进度回调，回调上下文因此只需对该线程有效。
//! - `run` 只交出 [`SqlOps`]，不交出 `rusqlite::Connection`。每次进入 C API 的操作
//!   （prepare、execute、query_row、语句的 execute / query_row）之前都重新核：本绑定为当前
//!   绑定、父账当前冻结域仍是本绑定的 origin、连接归属本父账、本父账本 origin 尚无拒绝；
//!   任一不符则该操作不进入 SQLite。同一时刻只有一个 `run`，期间禁止换绑与再入。
//! - `run`（含 open / close）在入口核验通过后，即在父账当前冻结域上持有一个活动作用域，
//!   与 Rust 许可的作用域同一计数，直到 `run` 结束；持有期间父账 `handoff` / 重新冻结均被拒，
//!   所以整个操作域内 origin 固定，参数 `ToSql` 等任何回调都无法在操作中途换域。
//! - 参数 `ToSql` 回调夹在两次准入之间运行，回调里触发的首拒在进入 SQLite 前即被拦下；首拒已在
//!   时参数不被求值。第二次准入之后，SQLite 内部还会同步调用连接上的授权回调与进度回调：进度
//!   回调先过组件门（归属、当前域、首拒），不放行即中止且不调用调用方进度回调；授权回调由
//!   SQLite 在 prepare 时调用，只能放行或拒绝语句，不能绕过组件门。
//! - 组件在操作口内不创建 Rust 堆持有：参数求值结果放栈上定长数组（借用调用方内存），结果行以
//!   `ValueRef` 借给调用方的映射回调，回调期间禁止任何 SQL。需要保留的数据由调用方按其许可复制，
//!   不由组件代付或代管。
//!   [`SqlOps::handle`] 是 `unsafe` 逃生口，调用方须维持本合同（不改进度回调、不关闭连接）。
//! - 只有正处于 `run` 中的线程能让 allocator 收费，其他线程或域外的分配一律按未知归属拒绝。
//! - 首个拒绝按（父账、origin）记，同父同 origin 重绑不清除；只有换到新冻结域才开新记录。
//! - 每条连接另装 SQLite 进度回调（每 1 条 VM 指令计数），在循环底部、每次返回行和语句结束
//!   前核对归属与首拒，不符即以 `SQLITE_INTERRUPT` 中止，覆盖不经新分配的行与写。
//! - realloc 只在块的原父账为当前绑定时进行；跨父 realloc 在收费前拒绝。
use super::scratch::ScratchFailure;
use rusqlite::ffi;
use rusqlite::types::{ToSqlOutput, ValueRef};
use rusqlite::ToSql;
use std::alloc::{alloc, dealloc, Layout};
use std::cell::Cell;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::marker::PhantomData;
use std::ops::ControlFlow;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};

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
    /// 在 `origin` 冻结域上开一个活动作用域（与 Rust 许可作用域同一计数）：持有期间父账拒绝换域。
    /// `origin` 已非当前域或父账破损时返回 `Ledger`。
    fn open_sqlite_scope(&self, origin: u64) -> Result<(), ScratchFailure>;
    /// 关闭 [`Self::open_sqlite_scope`] 开的作用域。
    fn close_sqlite_scope(&self, origin: u64) -> Result<(), ScratchFailure>;
}

const SLOTS: usize = 8;
/// 同时存活、带调用方进度回调的连接上限（定长表，不分配）。
const HOOKS: usize = 16;
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
    /// 槽已交还父句柄、但首拒所在冻结域仍是父账当前域时，以弱引用保留该父账的身份与首拒，
    /// 直到父账换域或析构；不延长父账寿命，弱引用也使其地址不会被别的父账复用。
    retained: Option<Weak<dyn SqliteParent>>,
}

impl Slot {
    fn holds(&self, key: *const ()) -> bool {
        self.parent.as_ref().is_some_and(|p| parent_key(p) == key)
            || self
                .retained
                .as_ref()
                .is_some_and(|w| Weak::as_ptr(w).cast::<()>() == key)
    }
    /// 可分给新父账：无父句柄，且保留的首拒已失效（父账已析构或已换域）。
    fn reusable(&self) -> bool {
        self.parent.is_none()
            && self.retained.as_ref().is_none_or(|w| {
                w.upgrade().is_none_or(|p| {
                    self.refusal
                        .is_none_or(|(origin, _)| !p.admits_sqlite(origin))
                })
            })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Owner {
    slot: usize,
    generation: u32,
}

/// 连接上调用方进度回调的登记项；组件进度门之后串联调用。
#[derive(Clone, Copy)]
struct Hook {
    owner: Owner,
    callback: ProgressFn,
    context: usize,
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
    hooks: [Option<Hook>; HOOKS],
}

const EMPTY: Slot = Slot {
    parent: None,
    generation: 0,
    live: 0,
    blocks: 0,
    refusal: None,
    retained: None,
};
static STATE: Mutex<State> = Mutex::new(State {
    installed: false,
    broken: false,
    slots: [EMPTY; SLOTS],
    current: None,
    epoch: 0,
    unattributed: 0,
    hooks: [None; HOOKS],
});
static INSTALLING: AtomicBool = AtomicBool::new(false);
/// 某个 `run` 正在执行；期间拒绝换绑与再入。
static RUNNING: AtomicBool = AtomicBool::new(false);
thread_local! {
    /// 本线程正处于 `run`；只有它的 SQLite 分配可以收费。
    static IN_RUN: Cell<bool> = const { Cell::new(false) };
    /// 本线程正处于结果行映射回调；其间拒绝一切 SQL 操作。
    static IN_ROW_MAP: Cell<bool> = const { Cell::new(false) };
    /// 本线程正处于收尾域（回滚、关闭、撤 VFS）：首拒后仍允许，且不调用调用方进度回调。
    static IN_TEARDOWN: Cell<bool> = const { Cell::new(false) };
    /// 本线程正在执行的 `run`：绑定身份与连接句柄。只供 WAL 回调内的 [`wal_checkpoint`] 准入与结算。
    static ACTIVE_RUN: Cell<Option<ActiveRun>> = const { Cell::new(None) };
}

#[derive(Clone, Copy)]
struct ActiveRun {
    epoch: u64,
    owner: Owner,
    origin: u64,
    db: usize,
}

fn in_teardown() -> bool {
    IN_TEARDOWN.try_with(Cell::get).unwrap_or(false)
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
    /// 收尾域准入：同 [`Self::admits`]，但不看首拒——首拒之后仍须能回滚与关闭。
    fn admits_teardown(&self, owner: Owner) -> bool {
        !self.broken
            && self
                .current
                .is_some_and(|b| b.owner == owner && self.origin_live(b))
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
            let parent = s.parent.take();
            s.retained = s.refusal.and(parent.as_ref().map(Arc::downgrade));
            parent
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
    // 收尾域（回滚、关闭）在首拒后仍须完成：其分配照常向原父账原冻结域预留，额度不足仍拒；
    // `latch` 不覆盖已有首拒。收尾域内不执行调用方 SQL。
    if s.first_refusal(b).is_some() && !in_teardown() {
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
        let slot = match s.slots.iter().position(|sl| sl.holds(key)) {
            Some(slot) => {
                // 同一父账（含仅以弱引用保留首拒的槽）：沿用代号与首拒。
                let sl = &mut s.slots[slot];
                if sl.parent.is_none() {
                    sl.parent = Some(parent);
                }
                sl.retained = None;
                slot
            }
            None => {
                let slot = s
                    .slots
                    .iter()
                    .position(Slot::reusable)
                    .ok_or(ScratchFailure::Ledger)?;
                let sl = &mut s.slots[slot];
                sl.generation = sl.generation.checked_add(1).ok_or(ScratchFailure::Ledger)?;
                sl.parent = Some(parent);
                sl.refusal = None;
                sl.retained = None;
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

/// `(owner, origin)` 冻结域的首个收费拒绝（[`SqliteBinding::refusal`] 的判据）。
fn refusal_at(owner: Owner, origin: u64) -> Option<ScratchFailure> {
    let s = state()?;
    let sl = &s.slots[owner.slot];
    if sl.generation != owner.generation {
        return None;
    }
    sl.refusal.filter(|(at, _)| *at == origin).map(|(_, f)| f)
}

/// 绑定 `epoch` 为当前绑定、父账当前域仍是其 origin、本域无拒绝（收尾除外）、`owner` 归属该父账
/// （[`SqliteBinding`] 普通与收尾准入的判据）。
fn admit_epoch(
    epoch: u64,
    binding_owner: Owner,
    owner: Option<Owner>,
    teardown: bool,
) -> Result<(), SqlGate> {
    if IN_ROW_MAP.try_with(Cell::get).unwrap_or(true) {
        return Err(SqlGate::Refused(ScratchFailure::Ledger));
    }
    let s = state().ok_or(SqlGate::Refused(ScratchFailure::Ledger))?;
    let current = s.current.filter(|b| b.epoch == epoch);
    let Some(b) = current.filter(|_| !s.broken) else {
        return Err(SqlGate::Refused(ScratchFailure::Ledger));
    };
    if let Some(first) = s.first_refusal(b).filter(|_| !teardown) {
        return Err(SqlGate::Refused(first));
    }
    if !s.origin_live(b) {
        return Err(SqlGate::Refused(ScratchFailure::Ledger));
    }
    if owner.is_some_and(|o| o != binding_owner) {
        return Err(SqlGate::WrongParent);
    }
    Ok(())
}

/// 当前收费绑定。析构时若仍为当前绑定则解除，此后的分配因无归属而被拒。
pub struct SqliteBinding {
    epoch: u64,
    owner: Owner,
    origin: u64,
}

impl SqliteBinding {
    /// 本父账本冻结域的首个收费拒绝。一旦拒绝，此域内调用方 SQL 与普通操作域的分配均被拒，换绑
    /// 同域不清除；只有收尾（[`Self::rollback`]、[`Self::close`]、[`Self::unregister_vfs`]）
    /// 仍可执行，其分配照常向原域预留、不足仍拒，且不覆盖已有首拒。
    pub fn refusal(&self) -> Option<ScratchFailure> {
        refusal_at(self.owner, self.origin)
    }

    /// SQLite 报 `SQLITE_NOMEM`（含扩展码）且本域已记收费拒绝时，返回该首个拒绝。
    pub fn refusal_of(&self, code: c_int) -> Option<ScratchFailure> {
        if code & 0xff == ffi::SQLITE_NOMEM {
            self.refusal()
        } else {
            None
        }
    }

    /// 本绑定此刻能否开始一次普通操作：为当前绑定（epoch）、allocator 未破损、本域无首拒、
    /// 冻结域仍存活。只读核对，不进入操作域；拒绝按值返回本次分类。
    pub fn admitted(&self) -> Result<(), SqlGate> {
        self.admit(None)
    }

    /// 本绑定是否属于 `parent` 的 `origin` 冻结域。只读核对：不绑定、不改状态、不交出句柄。
    pub fn serves(&self, parent: &Arc<dyn SqliteParent>, origin: u64) -> bool {
        self.origin == origin
            && state().is_some_and(|s| {
                self.slot_of(&s)
                    .is_some_and(|sl| sl.holds(parent_key(parent)))
            })
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
        self.enter_mode(conn, false)
    }

    /// 进入收尾域：同 [`Self::enter`]，但首拒之后仍放行；域内只执行组件自己的收尾调用。
    fn enter_teardown(&self, conn: Option<&ChargedConnection>) -> Result<Running, SqlGate> {
        self.enter_mode(conn, true)
    }

    fn enter_mode(
        &self,
        conn: Option<&ChargedConnection>,
        teardown: bool,
    ) -> Result<Running, SqlGate> {
        if RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(SqlGate::Refused(ScratchFailure::Ledger));
        }
        let mut running = Running(None);
        self.admit_mode(conn.map(|c| c.owner), teardown)?;
        running.0 = Some(self.hold_domain()?);
        IN_RUN.with(|r| r.set(true));
        if teardown {
            IN_TEARDOWN.with(|t| t.set(true));
        }
        Ok(running)
    }

    /// 在父账本 origin 上开活动作用域，固定整个操作域的冻结域。
    fn hold_domain(&self) -> Result<(Arc<dyn SqliteParent>, u64), SqlGate> {
        let parent = {
            let s = state().ok_or(SqlGate::Refused(ScratchFailure::Ledger))?;
            let sl = &s.slots[self.owner.slot];
            match sl.parent.as_ref() {
                Some(p) if sl.generation == self.owner.generation => p.clone(),
                _ => return Err(SqlGate::Refused(ScratchFailure::Ledger)),
            }
        };
        parent
            .open_sqlite_scope(self.origin)
            .map_err(SqlGate::Refused)?;
        Ok((parent, self.origin))
    }

    /// 本绑定为当前绑定、父账当前域仍是本 origin、本域无拒绝、`owner` 归属本父账。
    fn admit(&self, owner: Option<Owner>) -> Result<(), SqlGate> {
        self.admit_mode(owner, false)
    }

    fn admit_mode(&self, owner: Option<Owner>, teardown: bool) -> Result<(), SqlGate> {
        admit_epoch(self.epoch, self.owner, owner, teardown)
    }

    /// 首拒优先：本域执行中已记收费拒绝时，一律报该首个拒绝（即使 SQLite 层吞掉了错误）。
    fn settle<R>(&self, result: Result<R, c_int>) -> Result<R, SqlGate> {
        if let Some(first) = self.refusal() {
            return Err(SqlGate::Refused(first));
        }
        result.map_err(SqlGate::Sqlite)
    }

    /// 在本父账名下以默认方式（读写、可创建、URI、默认 VFS）打开连接；见 [`Self::open_with`]。
    pub fn open(&self, path: &CStr) -> Result<ChargedConnection, SqlGate> {
        // SAFETY: the default spec carries no caller callbacks or contexts.
        unsafe { self.open_with(path, &OpenSpec::default()) }
    }

    /// 在本父账名下按 `spec` 打开连接：保留调用方原有的打开标志、VFS、连接配置、限额与
    /// 授权回调；调用方进度回调串在组件进度门之后（门不放行时不调用，收尾域内也不调用）。
    /// 任一配置未生效即关闭连接并报错。连接终身归属本父账。
    ///
    /// # Safety
    /// 满足 [`Callback`] 的安全合同；`spec.vfs` 命名的 VFS 须已注册并在连接关闭（或析构）之前一直
    /// 注册且有效。
    pub unsafe fn open_with(
        &self,
        path: &CStr,
        spec: &OpenSpec<'_>,
    ) -> Result<ChargedConnection, SqlGate> {
        let id = next_connection_id().ok_or(SqlGate::Refused(ScratchFailure::Ledger))?;
        let running = self.enter(None)?;
        let owner = self.owner;
        let hook = match spec.progress {
            Some(p) => match reserve_hook(owner, p) {
                Some(h) => Some(h),
                None => return Err(SqlGate::Refused(ScratchFailure::Ledger)),
            },
            None => None,
        };
        let mut db = ptr::null_mut();
        let vfs = spec.vfs.map_or(ptr::null(), CStr::as_ptr);
        // SAFETY: path is NUL-terminated; db receives a handle (possibly non-null on error).
        let rc = unsafe { ffi::sqlite3_open_v2(path.as_ptr(), &mut db, spec.flags, vfs) };
        let opened = if rc == ffi::SQLITE_OK {
            // SAFETY: db is a live handle; callback contexts are guaranteed by the caller.
            match unsafe { configure(db, spec) } {
                Ok(()) => {
                    // SAFETY: db is live; the component's context is an integer, not a pointer.
                    unsafe {
                        ffi::sqlite3_progress_handler(db, 1, Some(progress), owner.context(hook));
                    }
                    Ok(ChargedConnection {
                        db,
                        owner,
                        hook,
                        id,
                    })
                }
                Err(code) => {
                    // SAFETY: db is live and has no statements yet.
                    unsafe { ffi::sqlite3_close(db) };
                    Err(code)
                }
            }
        } else {
            let code = if db.is_null() {
                rc
            } else {
                // SAFETY: db is a live handle from the failed open.
                unsafe {
                    let code = ffi::sqlite3_extended_errcode(db);
                    ffi::sqlite3_close(db);
                    code
                }
            };
            Err(code)
        };
        drop(running);
        if opened.is_err() {
            release_hook(hook);
        }
        self.settle(opened)
    }

    /// 在本父账名下注册 VFS（不设为默认）。注册中 SQLite 的分配向本父账收费。
    ///
    /// # Safety
    /// `vfs` 必须有效，且在 [`Self::unregister_vfs`] 之前一直有效、不移动。VFS 方法由 SQLite
    /// 在使用它的连接所在线程上调用；收费 allocator 只让正处于 `run` 的线程分配，连接又不可跨
    /// 线程，所以 `pAppData` 等 VFS 状态按 [`Callback`] 的安全合同只需对开库线程有效。
    pub unsafe fn register_vfs(&self, vfs: *mut ffi::sqlite3_vfs) -> Result<(), SqlGate> {
        let running = self.enter(None)?;
        // SAFETY: caller guarantees vfs stays valid until unregistered; make_default = 0.
        let rc = unsafe { ffi::sqlite3_vfs_register(vfs, 0) };
        drop(running);
        self.settle(if rc == ffi::SQLITE_OK {
            Ok(())
        } else {
            Err(rc)
        })
    }

    /// 收尾：撤销 [`Self::register_vfs`] 注册的 VFS。首拒之后仍可执行。
    ///
    /// # Safety
    /// `vfs` 必须是先前注册且已无连接使用的 VFS。
    pub unsafe fn unregister_vfs(&self, vfs: *mut ffi::sqlite3_vfs) -> Result<(), SqlGate> {
        let running = self.enter_teardown(None)?;
        // SAFETY: caller guarantees vfs was registered and is unused.
        let rc = unsafe { ffi::sqlite3_vfs_unregister(vfs) };
        drop(running);
        if rc == ffi::SQLITE_OK {
            Ok(())
        } else {
            Err(SqlGate::Sqlite(rc))
        }
    }

    /// 收尾：回滚连接上未结束的事务（无事务时什么都不做）。首拒之后仍可执行，不调用调用方进度
    /// 回调，也不执行调用方 SQL；结果只报回滚本身，首个拒绝仍由 [`Self::refusal`] 给出。
    pub fn rollback(&self, conn: &ChargedConnection) -> Result<(), SqlGate> {
        let running = self.enter_teardown(Some(conn))?;
        // SAFETY: conn owns a live handle; the statement text is a static C string.
        let rc = unsafe {
            if ffi::sqlite3_get_autocommit(conn.db) != 0 {
                ffi::SQLITE_OK
            } else {
                ffi::sqlite3_exec(
                    conn.db,
                    c"ROLLBACK".as_ptr(),
                    None,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            }
        };
        let result = if rc == ffi::SQLITE_OK {
            Ok(())
        } else {
            Err(SqlGate::Sqlite(last_code(conn.db, rc)))
        };
        drop(running);
        result
    }

    /// 收尾：把 `NO_CKPT_ON_CLOSE` 恢复为 0（关闭时照常检查点），返回 SQLite 读回值（恢复成功为
    /// `false`）。只此一项配置；首拒之后仍可执行，不执行任何 SQL，不清除首拒。
    pub fn restore_close_checkpoint(&self, conn: &ChargedConnection) -> Result<bool, SqlGate> {
        let running = self.enter_teardown(Some(conn))?;
        let mut out: c_int = -1;
        // SAFETY: conn owns a live handle; NO_CKPT_ON_CLOSE takes (int, int*).
        let rc = unsafe {
            ffi::sqlite3_db_config(
                conn.db,
                ffi::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE,
                0 as c_int,
                &mut out as *mut c_int,
            )
        };
        let result = if rc == ffi::SQLITE_OK {
            Ok(out != 0)
        } else {
            Err(SqlGate::Sqlite(last_code(conn.db, rc)))
        };
        drop(running);
        result
    }

    /// 收尾：撤销连接上的授权回调（例如 reader 生涯的只读封印），使收尾回滚不被它拒绝、调用方
    /// 可以安全释放其上下文。首拒之后仍可执行；不执行 SQL、不分配、不清除首拒。首拒之后普通
    /// 操作仍在准入处被拒，撤销授权器不放行任何调用方 SQL。
    pub fn clear_authorizer(&self, conn: &ChargedConnection) -> Result<(), SqlGate> {
        let running = self.enter_teardown(Some(conn))?;
        // SAFETY: conn owns a live handle; clearing the authorizer takes no context.
        let rc = unsafe { ffi::sqlite3_set_authorizer(conn.db, None, ptr::null_mut()) };
        drop(running);
        if rc == ffi::SQLITE_OK {
            Ok(())
        } else {
            Err(SqlGate::Sqlite(rc))
        }
    }

    /// 收尾：撤销连接上的行变更与 WAL 提交回调，使调用方可以安全释放其上下文。首拒之后仍可
    /// 执行，不执行 SQL、不分配、不清除首拒。
    pub fn clear_write_hooks(&self, conn: &ChargedConnection) -> Result<(), SqlGate> {
        let running = self.enter_teardown(Some(conn))?;
        // SAFETY: conn owns a live handle; clearing hooks takes no context.
        unsafe {
            ffi::sqlite3_update_hook(conn.db, None, ptr::null_mut());
            ffi::sqlite3_wal_hook(conn.db, None, ptr::null_mut());
        }
        drop(running);
        Ok(())
    }

    /// 在本操作域执行 `f`。连接须归属本父账、本绑定须为当前、本域须无拒绝，否则 `f` 不执行；
    /// `f` 内每次操作再各自核一次。首个收费拒绝之后，后续操作不再进入 SQLite，结果报首拒。
    pub fn run<R>(
        &self,
        conn: &ChargedConnection,
        f: impl FnOnce(&SqlOps<'_>) -> Result<R, SqlGate>,
    ) -> Result<R, SqlGate> {
        let running = self.enter(Some(conn))?;
        ACTIVE_RUN.with(|a| {
            a.set(Some(ActiveRun {
                epoch: self.epoch,
                owner: self.owner,
                origin: self.origin,
                db: conn.db as usize,
            }))
        });
        let ops = SqlOps {
            db: conn.db,
            binding: self,
            owner: conn.owner,
            _conn: PhantomData,
        };
        let result = f(&ops);
        drop(running);
        match self.refusal() {
            Some(first) => Err(SqlGate::Refused(first)),
            None => result,
        }
    }

    /// 收尾：在本父账名下关闭归属本父账的连接；首拒之后仍可执行。归属不符、绑定非当前或连接
    /// 仍有未结束语句时原样交回连接。
    pub fn close(&self, conn: ChargedConnection) -> Result<(), (ChargedConnection, SqlGate)> {
        let running = match self.enter_teardown(Some(&conn)) {
            Ok(running) => running,
            Err(e) => return Err((conn, e)),
        };
        // SAFETY: conn owns a live handle.
        let rc = unsafe { ffi::sqlite3_close(conn.db) };
        drop(running);
        if rc == ffi::SQLITE_OK {
            release_hook(conn.hook);
            std::mem::forget(conn);
            Ok(())
        } else {
            Err((conn, SqlGate::Sqlite(rc)))
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

/// 独占的操作域；持有父账冻结域的活动作用域直到析构。
struct Running(Option<(Arc<dyn SqliteParent>, u64)>);

impl Drop for Running {
    fn drop(&mut self) {
        if let Some((parent, origin)) = self.0.take() {
            if parent.close_sqlite_scope(origin).is_err() {
                parent.invalidate_sqlite();
            }
        }
        let _ = IN_RUN.try_with(|r| r.set(false));
        let _ = IN_TEARDOWN.try_with(|t| t.set(false));
        let _ = ACTIVE_RUN.try_with(|a| a.set(None));
        RUNNING.store(false, Ordering::Release);
    }
}

/// 操作域边界的拒绝。`Refused` 携本域首个收费拒绝；绑定非当前、已有 `run` 在执行、行映射中
/// 或 allocator 破损时为 `Refused(Ledger)`。`Sqlite` 携 SQLite 扩展结果码（不建错误文本）。
#[derive(Debug, PartialEq, Eq)]
pub enum SqlGate {
    Refused(ScratchFailure),
    /// 连接归属另一父账，未执行任何 SQL。
    WrongParent,
    Sqlite(c_int),
}

impl Owner {
    /// 进度回调的上下文：槽号、调用方回调登记号（0 为无）与代号编进一个整数，不分配。
    fn context(self, hook: Option<usize>) -> *mut c_void {
        let hook = hook.map_or(0, |h| h + 1);
        ((self.generation as usize) << 16 | hook << 8 | self.slot) as *mut c_void
    }
    fn from_context(context: *mut c_void) -> (Self, Option<usize>) {
        let raw = context as usize;
        let owner = Owner {
            slot: raw & 0xff,
            generation: (raw >> 16) as u32,
        };
        (owner, ((raw >> 8) & 0xff).checked_sub(1))
    }
}

/// 调用方的 SQLite 进度回调（`sqlite3_progress_handler` 形式）。
pub type ProgressFn = unsafe extern "C" fn(*mut c_void) -> c_int;
/// 调用方的 SQLite 授权回调（`sqlite3_set_authorizer` 形式）。
pub type AuthorizerFn = unsafe extern "C" fn(
    *mut c_void,
    c_int,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
) -> c_int;
/// 调用方的行变更回调（`sqlite3_update_hook` 形式）。
pub type UpdateFn = unsafe extern "C" fn(*mut c_void, c_int, *const c_char, *const c_char, i64);
/// 调用方的 WAL 提交回调（`sqlite3_wal_hook` 形式）。
pub type WalFn =
    unsafe extern "C" fn(*mut c_void, *mut ffi::sqlite3, *const c_char, c_int) -> c_int;

/// WAL 提交回调内对 `main` 执行 `sqlite3_wal_checkpoint_v2`，返回 `(WAL 帧数, 已回填帧数)`。
/// 准入与结算同普通操作：只在本线程执行中的 `run`（其连接句柄即 `db`）内；进入 SQLite 之前按该
/// 绑定核当前绑定、当前 origin 与首拒，已有首拒即返回该首拒、不调用 checkpoint；返回之前首拒优先，
/// checkpoint 自身的收费拒绝同样按带类型首拒返回。不重新进入 `run`，首拒之后没有豁免。
///
/// # Safety
/// `db` 必须是 SQLite 交给 [`WalFn`] 回调的那个句柄，且只在该回调内调用。
pub unsafe fn wal_checkpoint(
    db: *mut ffi::sqlite3,
    mode: c_int,
) -> Result<(c_int, c_int), SqlGate> {
    let active = ACTIVE_RUN.try_with(Cell::get).ok().flatten();
    let Some(run) = active.filter(|a| a.db == db as usize && in_run() && !in_teardown()) else {
        return Err(SqlGate::Refused(ScratchFailure::Ledger));
    };
    admit_epoch(run.epoch, run.owner, None, false)?;
    let (mut frames, mut backfill) = (-1, -1);
    // SAFETY: db is the live handle passed to the WAL hook of the running connection; "main" is a
    // static C string.
    let rc = unsafe {
        ffi::sqlite3_wal_checkpoint_v2(db, c"main".as_ptr(), mode, &mut frames, &mut backfill)
    };
    if let Some(first) = refusal_at(run.owner, run.origin) {
        return Err(SqlGate::Refused(first));
    }
    if rc == ffi::SQLITE_OK {
        Ok((frames, backfill))
    } else {
        Err(SqlGate::Sqlite(last_code(db, rc)))
    }
}

/// 调用方回调及其上下文。安全合同（[`SqliteBinding::open_with`]、[`SqlOps::set_authorizer`]
/// 与 [`SqliteBinding::register_vfs`] 共用）：
/// - SQLite 只在开库线程上同步调用回调：授权与进度回调在该连接的 `run` / `rollback` / `close`
///   期间；VFS 方法另在 `open_with` 打开期间与 [`ChargedConnection`] 直接析构时（`xClose` 等）。
///   连接 `!Send`，析构也在开库线程，不会换线程。
/// - 上下文从登记起到连接关闭（或析构、或被更换）为止须有效，且在此期间开库线程随时按回调的
///   方式访问它都不得与其他线程的访问构成数据竞争；跨线程共享的状态须由调用方自行同步。
/// - 回调不得重入本连接、不得更改进度回调、不得关闭连接。
#[derive(Clone, Copy, Debug)]
pub struct Callback<F> {
    pub callback: F,
    pub context: *mut c_void,
}

/// [`OpenSpec`] 可设置的连接配置。只收 `sqlite3_db_config(db, op, int, int*)` 签名的布尔开关；
/// `MAINDBNAME`、`LOOKASIDE` 等其他签名的 op 不可表达。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DbFlag {
    NoCheckpointOnClose,
    Defensive,
    EnableLoadExtension,
}

impl DbFlag {
    pub const ALL: [DbFlag; 3] = [
        DbFlag::NoCheckpointOnClose,
        DbFlag::Defensive,
        DbFlag::EnableLoadExtension,
    ];

    pub const fn op(self) -> c_int {
        match self {
            DbFlag::NoCheckpointOnClose => ffi::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE,
            DbFlag::Defensive => ffi::SQLITE_DBCONFIG_DEFENSIVE,
            DbFlag::EnableLoadExtension => ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION,
        }
    }
}

/// [`SqliteBinding::open_with`] 的开库规格：原打开标志、VFS、布尔连接配置（打开后读回核对）、
/// 限额（id, 值，读回核对）、busy 超时、授权回调与调用方进度回调（每 1 条 VM 指令）。
pub struct OpenSpec<'s> {
    pub flags: c_int,
    pub vfs: Option<&'s CStr>,
    pub db_config: &'s [(DbFlag, bool)],
    pub limits: &'s [(c_int, c_int)],
    pub busy_timeout_ms: Option<c_int>,
    pub authorizer: Option<Callback<AuthorizerFn>>,
    pub progress: Option<Callback<ProgressFn>>,
}

impl Default for OpenSpec<'_> {
    fn default() -> Self {
        Self {
            flags: ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_CREATE | ffi::SQLITE_OPEN_URI,
            vfs: None,
            db_config: &[],
            limits: &[],
            busy_timeout_ms: None,
            authorizer: None,
            progress: None,
        }
    }
}

/// 按规格配置刚打开的连接；任一项未生效返回其结果码。
unsafe fn configure(db: *mut ffi::sqlite3, spec: &OpenSpec<'_>) -> Result<(), c_int> {
    // SAFETY: db is live; contexts are guaranteed by the caller of `open_with`.
    unsafe {
        ffi::sqlite3_extended_result_codes(db, 1);
        for &(flag, on) in spec.db_config {
            let value = c_int::from(on);
            let mut out: c_int = -1;
            let rc = ffi::sqlite3_db_config(db, flag.op(), value, &mut out as *mut c_int);
            if rc != ffi::SQLITE_OK {
                return Err(last_code(db, rc));
            }
            if out != value {
                return Err(ffi::SQLITE_MISUSE);
            }
        }
        for &(id, value) in spec.limits {
            ffi::sqlite3_limit(db, id, value);
            if ffi::sqlite3_limit(db, id, -1) != value {
                return Err(ffi::SQLITE_MISUSE);
            }
        }
        if let Some(ms) = spec.busy_timeout_ms {
            let rc = ffi::sqlite3_busy_timeout(db, ms);
            if rc != ffi::SQLITE_OK {
                return Err(rc);
            }
        }
        if let Some(a) = spec.authorizer {
            let rc = ffi::sqlite3_set_authorizer(db, Some(a.callback), a.context);
            if rc != ffi::SQLITE_OK {
                return Err(rc);
            }
        }
    }
    Ok(())
}

fn reserve_hook(owner: Owner, p: Callback<ProgressFn>) -> Option<usize> {
    let mut s = state()?;
    let at = s.hooks.iter().position(Option::is_none)?;
    s.hooks[at] = Some(Hook {
        owner,
        callback: p.callback,
        context: p.context as usize,
    });
    Some(at)
}

fn release_hook(hook: Option<usize>) {
    if let (Some(at), Some(mut s)) = (hook, state()) {
        s.hooks[at] = None;
    }
}

/// 进度回调：归属、父账当前域或首拒不符即中止（`SQLITE_INTERRUPT`），兜底 unsafe 逃生口；
/// 门放行后再调用调用方登记的进度回调（收尾域内不调用，与原回滚前停用进度回调同义）。
unsafe extern "C" fn progress(context: *mut c_void) -> c_int {
    let (owner, hook) = Owner::from_context(context);
    let teardown = in_teardown();
    let user = {
        let Some(s) = state() else {
            return 1;
        };
        let open = if teardown {
            s.admits_teardown(owner)
        } else {
            s.admits(owner)
        };
        if !open {
            return 1;
        }
        if teardown {
            None
        } else {
            hook.and_then(|h| s.hooks.get(h).copied().flatten())
                .filter(|k| k.owner == owner)
        }
    };
    match user {
        // SAFETY: the caller of `open_with` guarantees the context outlives the connection.
        Some(k) => unsafe { (k.callback)(k.context as *mut c_void) },
        None => 0,
    }
}

/// 归属开它的父账的 SQLite 连接，直接持有 C 句柄，不建 rusqlite 包装对象；只能经
/// [`SqliteBinding::run`] 使用。未经 [`SqliteBinding::close`] 直接析构时先 finalize 残留语句再
/// `sqlite3_close`，块按各自块头返还原父账，关闭期间的新分配因不在操作域而被拒。
#[derive(Debug)]
pub struct ChargedConnection {
    db: *mut ffi::sqlite3,
    owner: Owner,
    hook: Option<usize>,
    id: ConnectionId,
}

/// 受管连接的身份：本进程内每次成功开库取一个新值，连接关闭后不复用（不同于句柄地址可能被
/// 新连接重用）。只能比较，不交出句柄。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConnectionId(u64);

static NEXT_CONNECTION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// 取下一个连接身份；计数用尽时不回绕（回绕会让新连接与旧身份相等），开库按 Ledger 拒绝。
fn next_connection_id() -> Option<ConnectionId> {
    NEXT_CONNECTION
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .ok()
        .map(ConnectionId)
}

// ChargedConnection 既非 Send 也非 Sync：实现任一，下面的推断即歧义而编译失败。
const _: fn() = || {
    trait AmbiguousIfSendOrSync<A> {
        fn check() {}
    }
    impl<T: ?Sized> AmbiguousIfSendOrSync<()> for T {}
    impl<T: ?Sized + Send> AmbiguousIfSendOrSync<u8> for T {}
    impl<T: ?Sized + Sync> AmbiguousIfSendOrSync<u16> for T {}
    let _ = <ChargedConnection as AmbiguousIfSendOrSync<_>>::check;
};

impl ChargedConnection {
    /// 连接是否处于自动提交（没有未结束事务）。只读连接状态，不执行 SQL、不分配。
    pub fn is_autocommit(&self) -> bool {
        // SAFETY: db is a live handle owned by this connection.
        unsafe { ffi::sqlite3_get_autocommit(self.db) != 0 }
    }

    /// 连接打开以来的累计变更行数（`sqlite3_total_changes64`）。只读连接状态，不执行 SQL、不分配。
    pub fn total_changes(&self) -> u64 {
        // SAFETY: db is a live handle owned by this connection.
        u64::try_from(unsafe { ffi::sqlite3_total_changes64(self.db) }).unwrap_or(0)
    }

    /// 本连接的身份，见 [`ConnectionId`]。
    pub fn id(&self) -> ConnectionId {
        self.id
    }
}

impl Drop for ChargedConnection {
    fn drop(&mut self) {
        // SAFETY: the handle is live and owned. Statements left by the unsafe escape hatch are
        // finalized first so the close cannot fail with SQLITE_BUSY and leak the handle.
        unsafe {
            if ffi::sqlite3_close(self.db) != ffi::SQLITE_OK {
                loop {
                    let stmt = ffi::sqlite3_next_stmt(self.db, ptr::null_mut());
                    if stmt.is_null() {
                        break;
                    }
                    ffi::sqlite3_finalize(stmt);
                }
                ffi::sqlite3_close(self.db);
            }
        }
        release_hook(self.hook);
    }
}

fn last_code(db: *mut ffi::sqlite3, rc: c_int) -> c_int {
    // SAFETY: db is a live handle.
    let code = unsafe { ffi::sqlite3_extended_errcode(db) };
    if code & 0xff == rc & 0xff {
        code
    } else {
        rc
    }
}

/// 参数先在两次准入之间求值：`ToSql` 是调用方代码，可能触发收费拒绝或尝试换域。结果留在栈上
/// 定长数组，`Borrowed` 只借调用方内存；绑定一律 `SQLITE_TRANSIENT`，由 SQLite 以本父账收费的
/// 块复制，组件不建 Rust 副本。
fn evaluate<'p, const N: usize>(
    params: &[&'p dyn ToSql; N],
) -> Result<[ToSqlOutput<'p>; N], SqlGate> {
    let mut out: [ToSqlOutput<'p>; N] =
        std::array::from_fn(|_| ToSqlOutput::Borrowed(ValueRef::Null));
    for (slot, p) in out.iter_mut().zip(params) {
        let p: &'p dyn ToSql = *p;
        *slot = p
            .to_sql()
            .map_err(|_| SqlGate::Sqlite(ffi::SQLITE_MISMATCH))?;
    }
    Ok(out)
}

fn value_of<'v>(out: &'v ToSqlOutput<'_>) -> Result<ValueRef<'v>, SqlGate> {
    match out {
        ToSqlOutput::Borrowed(v) => Ok(*v),
        // 调用方在 `to_sql` 里新建的堆值（Owned Text/Blob）不进操作域：组件不为其代付，也不让
        // 未计费持有参与 SQL；调用方须以借用交出自己按许可持有的数据。标量 Owned 无堆。
        ToSqlOutput::Owned(rusqlite::types::Value::Text(_) | rusqlite::types::Value::Blob(_)) => {
            Err(SqlGate::Sqlite(ffi::SQLITE_MISMATCH))
        }
        ToSqlOutput::Owned(v) => Ok(ValueRef::from(v)),
        #[allow(unreachable_patterns)]
        _ => Err(SqlGate::Sqlite(ffi::SQLITE_MISMATCH)),
    }
}

/// 把求值后的参数绑到语句；全部 `SQLITE_TRANSIENT`。
fn bind<const N: usize>(
    db: *mut ffi::sqlite3,
    stmt: *mut ffi::sqlite3_stmt,
    values: &[ToSqlOutput<'_>; N],
) -> Result<(), c_int> {
    // SAFETY: stmt is live; pointers and lengths come from live borrowed values copied by SQLite.
    unsafe {
        if usize::try_from(ffi::sqlite3_bind_parameter_count(stmt)).ok() != Some(N) {
            return Err(ffi::SQLITE_RANGE);
        }
        for (i, out) in values.iter().enumerate() {
            let at = i as c_int + 1;
            let rc = match value_of(out).map_err(|_| ffi::SQLITE_MISMATCH)? {
                ValueRef::Null => ffi::sqlite3_bind_null(stmt, at),
                ValueRef::Integer(v) => ffi::sqlite3_bind_int64(stmt, at, v),
                ValueRef::Real(v) => ffi::sqlite3_bind_double(stmt, at, v),
                ValueRef::Text(t) => ffi::sqlite3_bind_text64(
                    stmt,
                    at,
                    t.as_ptr().cast(),
                    t.len() as u64,
                    ffi::SQLITE_TRANSIENT(),
                    ffi::SQLITE_UTF8 as u8,
                ),
                ValueRef::Blob(b) => ffi::sqlite3_bind_blob64(
                    stmt,
                    at,
                    b.as_ptr().cast(),
                    b.len() as u64,
                    ffi::SQLITE_TRANSIENT(),
                ),
            };
            if rc != ffi::SQLITE_OK {
                return Err(last_code(db, rc));
            }
        }
    }
    Ok(())
}

/// 在操作口内逐列借值（每取一列前重核首拒；text/blob 的物化由 SQLite 以本父账收费），
/// 然后在禁止任何 SQL 操作的状态下把借值交给调用方映射。借值活不过本次调用。
fn map_row<T, const M: usize>(
    binding: &SqliteBinding,
    db: *mut ffi::sqlite3,
    stmt: *mut ffi::sqlite3_stmt,
    f: impl FnOnce([ValueRef<'_>; M]) -> T,
) -> Result<T, c_int> {
    let mut refs = [ValueRef::Null; M];
    // SAFETY: stmt is live and positioned on a row; see `fill_row`.
    unsafe { fill_row(binding, db, stmt, &mut refs)? };
    let _mapping = RowMapping::enter();
    Ok(f(refs))
}

/// 一行最多可借出的列数；超过即 `SQLITE_RANGE`，不为列缓冲分配。
pub const MAX_ROW_COLUMNS: usize = 32;

/// 按语句实际列数借出当前行，交 `f` 映射；规则同 [`map_row`]。
fn map_cells<T>(
    binding: &SqliteBinding,
    db: *mut ffi::sqlite3,
    stmt: *mut ffi::sqlite3_stmt,
    f: impl FnOnce(&[ValueRef<'_>]) -> T,
) -> Result<T, c_int> {
    // SAFETY: stmt is live.
    let columns = usize::try_from(unsafe { ffi::sqlite3_column_count(stmt) })
        .ok()
        .filter(|n| *n <= MAX_ROW_COLUMNS)
        .ok_or(ffi::SQLITE_RANGE)?;
    let mut refs = [ValueRef::Null; MAX_ROW_COLUMNS];
    let refs = &mut refs[..columns];
    // SAFETY: stmt is live and positioned on a row; see `fill_row`.
    unsafe { fill_row(binding, db, stmt, refs)? };
    let _mapping = RowMapping::enter();
    Ok(f(refs))
}

/// # Safety
/// stmt is live and positioned on a row; the returned slices stay valid until the next
/// step/reset/finalize on stmt, all of which happen after the caller's mapping returns.
unsafe fn fill_row<'r>(
    binding: &SqliteBinding,
    db: *mut ffi::sqlite3,
    stmt: *mut ffi::sqlite3_stmt,
    refs: &mut [ValueRef<'r>],
) -> Result<(), c_int> {
    unsafe {
        if usize::try_from(ffi::sqlite3_column_count(stmt)).ok() != Some(refs.len()) {
            return Err(ffi::SQLITE_RANGE);
        }
        for (i, slot) in refs.iter_mut().enumerate() {
            if binding.refusal().is_some() {
                return Err(ffi::SQLITE_NOMEM);
            }
            let at = i as c_int;
            *slot = match ffi::sqlite3_column_type(stmt, at) {
                ffi::SQLITE_INTEGER => ValueRef::Integer(ffi::sqlite3_column_int64(stmt, at)),
                ffi::SQLITE_FLOAT => ValueRef::Real(ffi::sqlite3_column_double(stmt, at)),
                ffi::SQLITE_TEXT => {
                    let p = ffi::sqlite3_column_text(stmt, at);
                    let n = ffi::sqlite3_column_bytes(stmt, at) as usize;
                    if p.is_null() {
                        return Err(last_code(db, ffi::SQLITE_NOMEM));
                    }
                    ValueRef::Text(std::slice::from_raw_parts(p, n))
                }
                ffi::SQLITE_BLOB => {
                    let p = ffi::sqlite3_column_blob(stmt, at);
                    let n = ffi::sqlite3_column_bytes(stmt, at) as usize;
                    if p.is_null() {
                        if n != 0 || ffi::sqlite3_errcode(db) == ffi::SQLITE_NOMEM {
                            return Err(ffi::SQLITE_NOMEM);
                        }
                        ValueRef::Blob(&[])
                    } else {
                        ValueRef::Blob(std::slice::from_raw_parts(p.cast(), n))
                    }
                }
                _ => ValueRef::Null,
            };
        }
        if binding.refusal().is_some() {
            return Err(ffi::SQLITE_NOMEM);
        }
    }
    Ok(())
}

/// 行映射回调期间置位；其间一切 SQL 操作在准入处拒绝。
struct RowMapping(());

impl RowMapping {
    fn enter() -> Self {
        let _ = IN_ROW_MAP.try_with(|m| m.set(true));
        Self(())
    }
}

impl Drop for RowMapping {
    fn drop(&mut self) {
        let _ = IN_ROW_MAP.try_with(|m| m.set(false));
    }
}

/// `run` 内的 SQL 操作口，直接走 C API。组件在操作口内不创建任何 Rust 堆持有：SQL 文本按
/// 指针+长度交给 SQLite（不建 C 字符串），参数在栈上定长数组并以 `SQLITE_TRANSIENT` 由
/// SQLite 复制，结果行以 `ValueRef` 借给调用方映射，错误只携结果码（不建错误文本），
/// 进度回调的上下文是整数。每个方法在参数求值前后各核一次操作域。
pub struct SqlOps<'a> {
    db: *mut ffi::sqlite3,
    binding: &'a SqliteBinding,
    owner: Owner,
    _conn: PhantomData<&'a ChargedConnection>,
}

impl<'a> SqlOps<'a> {
    fn admit(&self) -> Result<(), SqlGate> {
        self.binding.admit(Some(self.owner))
    }

    /// 准备一条语句；`tail` 返回未消费的 SQL 余部。
    fn raw_prepare<'s>(&self, sql: &'s str) -> Result<(*mut ffi::sqlite3_stmt, &'s str), SqlGate> {
        self.admit()?;
        let len = c_int::try_from(sql.len()).map_err(|_| SqlGate::Sqlite(ffi::SQLITE_TOOBIG))?;
        let mut stmt = ptr::null_mut();
        let mut tail = ptr::null();
        // SAFETY: db is live; SQLite reads at most len bytes of sql.
        let rc = unsafe {
            ffi::sqlite3_prepare_v2(self.db, sql.as_ptr().cast(), len, &mut stmt, &mut tail)
        };
        let result = if rc == ffi::SQLITE_OK {
            let used = if tail.is_null() {
                sql.len()
            } else {
                (tail as usize).saturating_sub(sql.as_ptr() as usize)
            };
            Ok((stmt, sql.get(used..).unwrap_or("")))
        } else {
            Err(last_code(self.db, rc))
        };
        self.binding.settle(result)
    }

    /// 依次准备并执行 `sql` 中的每条语句，每条之前重核操作域。
    pub fn execute_batch(&self, sql: &str) -> Result<(), SqlGate> {
        let mut rest = sql;
        loop {
            let (stmt, tail) = self.raw_prepare(rest)?;
            if stmt.is_null() {
                return Ok(());
            }
            let mut statement = GatedStatement {
                stmt,
                db: self.db,
                binding: self.binding,
                owner: self.owner,
                _conn: PhantomData,
            };
            statement.execute([])?;
            rest = tail;
        }
    }

    pub fn execute<const N: usize>(
        &self,
        sql: &str,
        params: [&dyn ToSql; N],
    ) -> Result<usize, SqlGate> {
        self.prepare(sql)?.execute(params)
    }

    /// 取首行的 `M` 列交 `f` 映射；`f` 中不能再执行 SQL。
    pub fn query_row<T, const N: usize, const M: usize>(
        &self,
        sql: &str,
        params: [&dyn ToSql; N],
        f: impl FnOnce([ValueRef<'_>; M]) -> T,
    ) -> Result<T, SqlGate> {
        self.prepare(sql)?.query_row(params, f)
    }

    pub fn prepare(&self, sql: &str) -> Result<GatedStatement<'a>, SqlGate> {
        let (stmt, _) = self.raw_prepare(sql)?;
        if stmt.is_null() {
            return Err(SqlGate::Sqlite(ffi::SQLITE_MISUSE));
        }
        Ok(GatedStatement {
            stmt,
            db: self.db,
            binding: self.binding,
            owner: self.owner,
            _conn: PhantomData,
        })
    }

    /// 更换连接的授权回调（原 3A 连接封印式的授权门）；`None` 撤销。授权回调不影响本组件的
    /// 收费与准入合同。
    ///
    /// # Safety
    /// 满足 [`Callback`] 的安全合同，期限到连接关闭或下次更换为止。
    pub unsafe fn set_authorizer(
        &self,
        authorizer: Option<Callback<AuthorizerFn>>,
    ) -> Result<(), SqlGate> {
        self.admit()?;
        let (callback, context) =
            authorizer.map_or((None, ptr::null_mut()), |a| (Some(a.callback), a.context));
        // SAFETY: db is live; the caller guarantees the context's lifetime.
        let rc = unsafe { ffi::sqlite3_set_authorizer(self.db, callback, context) };
        self.binding.settle(if rc == ffi::SQLITE_OK {
            Ok(())
        } else {
            Err(rc)
        })
    }

    /// 把本连接的脏页写入数据库文件或 WAL（`sqlite3_db_cacheflush`），不结束事务。
    pub fn cache_flush(&self) -> Result<(), SqlGate> {
        self.admit()?;
        // SAFETY: db is live.
        let rc = unsafe { ffi::sqlite3_db_cacheflush(self.db) };
        self.binding.settle(if rc == ffi::SQLITE_OK {
            Ok(())
        } else {
            Err(last_code(self.db, rc))
        })
    }

    /// 登记或撤销行变更回调（`sqlite3_update_hook`）。回调在本连接的写语句执行期间同步调用。
    /// 撤销另见收尾能力 [`SqliteBinding::clear_write_hooks`]（首拒之后仍可执行）。
    ///
    /// # Safety
    /// 满足 [`Callback`] 的安全合同，期限到连接关闭、撤销或下次更换为止。
    pub unsafe fn set_update_hook(&self, hook: Option<Callback<UpdateFn>>) -> Result<(), SqlGate> {
        self.admit()?;
        let (callback, context) =
            hook.map_or((None, ptr::null_mut()), |h| (Some(h.callback), h.context));
        // SAFETY: db is live; the caller guarantees the context's lifetime.
        unsafe { ffi::sqlite3_update_hook(self.db, callback, context) };
        self.binding.settle(Ok(()))
    }

    /// 登记或撤销 WAL 提交回调（`sqlite3_wal_hook`）。回调在本连接 COMMIT 写入 WAL 之后同步调用；
    /// 作为本合同的唯一重入例外，回调可在 SQLite 交给它的句柄上调用 [`wal_checkpoint`]。
    ///
    /// # Safety
    /// 满足 [`Callback`] 的安全合同（上述 checkpoint 例外之外），期限同 [`Self::set_update_hook`]。
    pub unsafe fn set_wal_hook(&self, hook: Option<Callback<WalFn>>) -> Result<(), SqlGate> {
        self.admit()?;
        let (callback, context) =
            hook.map_or((None, ptr::null_mut()), |h| (Some(h.callback), h.context));
        // SAFETY: db is live; the caller guarantees the context's lifetime.
        unsafe { ffi::sqlite3_wal_hook(self.db, callback, context) };
        self.binding.settle(Ok(()))
    }

    /// 运行中设置布尔连接配置，返回 SQLite 读回的当前值（`sqlite3_db_config(db, op, int, int*)`）。
    pub fn set_db_flag(&self, flag: DbFlag, on: bool) -> Result<bool, SqlGate> {
        self.admit()?;
        let mut out: c_int = -1;
        // SAFETY: db is live; every DbFlag op takes (int, int*).
        let rc = unsafe {
            ffi::sqlite3_db_config(self.db, flag.op(), c_int::from(on), &mut out as *mut c_int)
        };
        self.binding.settle(if rc == ffi::SQLITE_OK {
            Ok(out != 0)
        } else {
            Err(last_code(self.db, rc))
        })
    }

    /// # Safety
    /// 只用于本操作域内的只读诊断与裸语句操作；不得更改或移除进度回调、不得关闭连接、
    /// 不得把句柄带出本 `run`。经此句柄的操作不受本合同保护。
    pub unsafe fn handle(&self) -> *mut ffi::sqlite3 {
        self.db
    }
}

/// 经 [`SqlOps::prepare`] 得到的语句；规则同 [`SqlOps`]。析构时 finalize。
pub struct GatedStatement<'a> {
    stmt: *mut ffi::sqlite3_stmt,
    db: *mut ffi::sqlite3,
    binding: &'a SqliteBinding,
    owner: Owner,
    _conn: PhantomData<&'a ChargedConnection>,
}

impl Drop for GatedStatement<'_> {
    fn drop(&mut self) {
        // SAFETY: stmt is live and owned.
        unsafe { ffi::sqlite3_finalize(self.stmt) };
    }
}

impl GatedStatement<'_> {
    fn admit(&self) -> Result<(), SqlGate> {
        self.binding.admit(Some(self.owner))
    }

    /// 求值参数（前后各一次准入）并绑定；成功后语句处于可 step 状态。
    fn ready<const N: usize>(&mut self, params: [&dyn ToSql; N]) -> Result<(), SqlGate> {
        self.admit()?;
        let values = evaluate(&params)?;
        self.admit()?;
        // SAFETY: stmt is live.
        unsafe {
            ffi::sqlite3_reset(self.stmt);
            ffi::sqlite3_clear_bindings(self.stmt);
        }
        self.binding.settle(bind(self.db, self.stmt, &values))
    }

    fn step(&mut self) -> Result<c_int, SqlGate> {
        // SAFETY: stmt is live and bound.
        let rc = unsafe { ffi::sqlite3_step(self.stmt) };
        let result = match rc {
            ffi::SQLITE_ROW | ffi::SQLITE_DONE => Ok(rc),
            _ => Err(last_code(self.db, rc)),
        };
        self.binding.settle(result)
    }

    /// 执行到结束（跳过结果行）；返回变更行数。
    pub fn execute<const N: usize>(&mut self, params: [&dyn ToSql; N]) -> Result<usize, SqlGate> {
        self.ready(params)?;
        while self.step()? == ffi::SQLITE_ROW {
            self.admit()?;
        }
        // SAFETY: db is live.
        let changes = unsafe { ffi::sqlite3_changes64(self.db) };
        // SAFETY: stmt is live.
        unsafe { ffi::sqlite3_reset(self.stmt) };
        Ok(usize::try_from(changes).unwrap_or(0))
    }

    /// 取首行的 `M` 列交 `f` 映射；`f` 中不能再执行 SQL。无行时报 `SQLITE_DONE`。
    pub fn query_row<T, const N: usize, const M: usize>(
        &mut self,
        params: [&dyn ToSql; N],
        f: impl FnOnce([ValueRef<'_>; M]) -> T,
    ) -> Result<T, SqlGate> {
        self.ready(params)?;
        if self.step()? != ffi::SQLITE_ROW {
            return Err(SqlGate::Sqlite(ffi::SQLITE_DONE));
        }
        let mapped = map_row(self.binding, self.db, self.stmt, f);
        // SAFETY: stmt is live; borrowed values are gone once `f` returned.
        unsafe { ffi::sqlite3_reset(self.stmt) };
        self.binding.settle(mapped)
    }

    /// 逐行取 `M` 列交 `f` 映射，`f` 返回 `Break` 即停；每行映射前各核一次准入，`f` 中不能再
    /// 执行 SQL。返回已映射的行数。
    pub fn for_each_row<const N: usize, const M: usize>(
        &mut self,
        params: [&dyn ToSql; N],
        mut f: impl FnMut([ValueRef<'_>; M]) -> ControlFlow<()>,
    ) -> Result<usize, SqlGate> {
        self.ready(params)?;
        let mut rows = 0;
        let result = loop {
            match self.step() {
                Ok(ffi::SQLITE_ROW) => {}
                Ok(_) => break Ok(rows),
                Err(e) => break Err(e),
            }
            if let Err(e) = self.admit() {
                break Err(e);
            }
            rows += 1;
            let mapped = map_row(self.binding, self.db, self.stmt, &mut f);
            match self.binding.settle(mapped) {
                Ok(ControlFlow::Continue(())) => {}
                Ok(ControlFlow::Break(())) => break Ok(rows),
                Err(e) => break Err(e),
            }
        };
        // SAFETY: stmt is live; borrowed values are gone once `f` returned.
        unsafe { ffi::sqlite3_reset(self.stmt) };
        result
    }

    /// 同 [`Self::for_each_row`]，但按语句实际列数（至多 [`MAX_ROW_COLUMNS`]）借出整行。
    pub fn for_each_cells<const N: usize>(
        &mut self,
        params: [&dyn ToSql; N],
        mut f: impl FnMut(&[ValueRef<'_>]) -> ControlFlow<()>,
    ) -> Result<usize, SqlGate> {
        self.ready(params)?;
        let mut rows = 0;
        let result = loop {
            match self.step() {
                Ok(ffi::SQLITE_ROW) => {}
                Ok(_) => break Ok(rows),
                Err(e) => break Err(e),
            }
            if let Err(e) = self.admit() {
                break Err(e);
            }
            rows += 1;
            let mapped = map_cells(self.binding, self.db, self.stmt, &mut f);
            match self.binding.settle(mapped) {
                Ok(ControlFlow::Continue(())) => {}
                Ok(ControlFlow::Break(())) => break Ok(rows),
                Err(e) => break Err(e),
            }
        };
        // SAFETY: stmt is live; borrowed values are gone once `f` returned.
        unsafe { ffi::sqlite3_reset(self.stmt) };
        result
    }
}
