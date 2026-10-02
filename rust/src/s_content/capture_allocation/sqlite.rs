//! #1394 非default、独立唯一TestOnly进程的SQLite requested Layout shim。
//! SQLite全局方法只在该进程首次初始化前安装；不可用于现存生产连接。
//! 回调仅做短锁计数、checked Layout、系统alloc/copy/free；不重入SQLite、不分配容器。
use super::{Meter, OwnedBudget};
use rusqlite::ffi;
use std::alloc::{alloc, dealloc, Layout};
use std::ffi::c_void;
use std::marker::PhantomData;
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Mutex;

static ACTIVE: AtomicPtr<Mutex<Meter>> = AtomicPtr::new(ptr::null_mut());
static INSTALLED: AtomicBool = AtomicBool::new(false);
const MAGIC: usize = 0x1394_5351;
#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Header {
    payload: usize,
    total: usize,
    magic: usize,
}
fn block_layout(payload: usize) -> Option<Layout> {
    // Header alignment and size ensure a fixed, 16-byte aligned payload address.
    let bytes = std::mem::size_of::<Header>().checked_add(payload)?;
    Some(
        Layout::from_size_align(bytes, std::mem::align_of::<Header>())
            .ok()?
            .pad_to_align(),
    )
}
fn core() -> Option<&'static Mutex<Meter>> {
    let p = ACTIVE.load(Ordering::Acquire);
    if p.is_null() {
        None
    } else {
        // SAFETY: install's keeper owns this Arc until quiescent shutdown/restore completes.
        Some(unsafe { &*p })
    }
}
fn broken() {
    if let Some(c) = core() {
        OwnedBudget::lock(c).broken = true;
    }
}
fn sql_error(code: i32) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(ffi::Error::new(code), None)
}
unsafe extern "C" fn malloc(n: i32) -> *mut c_void {
    unsafe { allocate(n, "SQLite xMalloc/header Layout") }
}
unsafe fn allocate(n: i32, kind: &'static str) -> *mut c_void {
    if n <= 0 {
        return ptr::null_mut();
    }
    let Some(c) = core() else {
        return ptr::null_mut();
    };
    let payload = n as usize;
    let Some(layout) = block_layout(payload) else {
        broken();
        return ptr::null_mut();
    };
    if !OwnedBudget::reserve_on(c, layout, kind, Some(payload)) {
        return ptr::null_mut();
    }
    // SAFETY: complete valid Layout already reserved atomically; allocation outside core lock.
    let p = unsafe { alloc(layout) };
    if p.is_null() {
        OwnedBudget::release_on(c, layout, Some(payload), false);
        return ptr::null_mut();
    }
    // SAFETY: header fits, p aligned; returned payload begins immediately after fixed header.
    unsafe {
        p.cast::<Header>().write(Header {
            payload,
            total: layout.size(),
            magic: MAGIC,
        });
    }
    OwnedBudget::commit_on(c, true);
    unsafe { p.add(std::mem::size_of::<Header>()).cast() }
}
unsafe fn header(p: *mut c_void) -> Option<(*mut u8, Header, Layout)> {
    if p.is_null() {
        return None;
    }
    // SAFETY: SQLite only supplies pointers returned by this installed allocator.
    let base = unsafe { p.cast::<u8>().sub(std::mem::size_of::<Header>()) };
    let h = unsafe { base.cast::<Header>().read() };
    let layout = block_layout(h.payload)?;
    if h.magic != MAGIC || h.total != layout.size() || h.payload > i32::MAX as usize {
        broken();
        return None;
    }
    Some((base, h, layout))
}
unsafe extern "C" fn free(p: *mut c_void) {
    if p.is_null() {
        return;
    }
    let Some(c) = core() else {
        return;
    };
    let Some((base, h, layout)) = (unsafe { header(p) }) else {
        broken();
        return;
    };
    // SAFETY: matching original Layout and sole SQLite owner.
    unsafe {
        dealloc(base, layout);
    }
    OwnedBudget::release_on(c, layout, Some(h.payload), true);
}
unsafe extern "C" fn realloc(p: *mut c_void, n: i32) -> *mut c_void {
    if p.is_null() {
        return unsafe { allocate(n, "SQLite xRealloc/header Layout") };
    }
    if n <= 0 {
        unsafe { free(p) };
        return ptr::null_mut();
    }
    let Some((_, h, _)) = (unsafe { header(p) }) else {
        broken();
        return ptr::null_mut();
    };
    // Reserve and allocate the ENTIRE new block while the old complete Layout remains live.
    let grown = unsafe { allocate(n, "SQLite xRealloc/header Layout") };
    if grown.is_null() {
        return ptr::null_mut();
    }
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
    // Overflow cannot become a small request or unwind into C.
    n.checked_add(7).map(|v| v & !7).unwrap_or(0)
}
unsafe extern "C" fn init(_: *mut c_void) -> i32 {
    ffi::SQLITE_OK
}
unsafe extern "C" fn shutdown(_: *mut c_void) {}

/// Keeper cannot move across threads. Callbacks use the shared core, never Rc/Arc clone.
/// An unfinished guard deliberately retains its keeper so a callback cannot use freed memory.
pub struct TestOnlySqliteAllocator {
    keeper: Option<OwnedBudget>,
    original: ffi::sqlite3_mem_methods,
    _isolated: PhantomData<Rc<()>>,
}
impl TestOnlySqliteAllocator {
    /// # Safety
    /// Exactly one fresh isolated process/test, before any SQLite initialization/open.
    /// No other thread or library may initialize/use/configure SQLite until finish completes.
    pub unsafe fn install(budget: &OwnedBudget) -> Result<Self, rusqlite::Error> {
        if INSTALLED
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(sql_error(ffi::SQLITE_MISUSE));
        }
        let mut original = std::mem::MaybeUninit::<ffi::sqlite3_mem_methods>::uninit();
        let code =
            unsafe { ffi::sqlite3_config(ffi::SQLITE_CONFIG_GETMALLOC, original.as_mut_ptr()) };
        if code != ffi::SQLITE_OK {
            INSTALLED.store(false, Ordering::Release);
            return Err(sql_error(code));
        }
        let original = unsafe { original.assume_init() };
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
        let keeper = budget.clone();
        ACTIVE.store(
            std::sync::Arc::as_ptr(&keeper.0).cast_mut(),
            Ordering::Release,
        );
        let code = unsafe { ffi::sqlite3_config(ffi::SQLITE_CONFIG_MALLOC, &methods) };
        if code != ffi::SQLITE_OK {
            ACTIVE.store(ptr::null_mut(), Ordering::Release);
            INSTALLED.store(false, Ordering::Release);
            return Err(sql_error(code));
        }
        Ok(Self {
            keeper: Some(keeper),
            original,
            _isolated: PhantomData,
        })
    }
    /// All later allocations refuse once they exceed current live plus this headroom.
    /// The real explicit test limit remains an upper bound; this never increases it.
    pub fn set_remaining_for_testonly(&self, bytes: usize) {
        if let Some(b) = &self.keeper {
            RefusalControl { budget: b.clone() }.set_remaining_for_testonly(bytes);
        }
    }
    pub fn clear_refusal_for_testonly(&self) {
        if let Some(b) = &self.keeper {
            RefusalControl { budget: b.clone() }.clear_refusal_for_testonly();
        }
    }
    /// Cloneable control has no installation, shutdown or restoration authority.
    pub fn control(&self) -> RefusalControl {
        // keeper remains Some until consuming finish; no path exposes a finished guard.
        RefusalControl {
            budget: self
                .keeper
                .as_ref()
                .expect("unfinished SQLite guard owns budget")
                .clone(),
        }
    }
    /// # Safety
    /// Every statement/transaction/connection and other SQLite handle has been closed,
    /// and no thread can call SQLite. Rust ledger owners may still live after this function.
    pub unsafe fn finish(mut self) -> Result<(), rusqlite::Error> {
        self.clear_refusal_for_testonly();
        let code = unsafe { ffi::sqlite3_shutdown() };
        if code != ffi::SQLITE_OK {
            return Err(sql_error(code));
        }
        let Some(b) = &self.keeper else {
            return Err(sql_error(ffi::SQLITE_MISUSE));
        };
        let h = b.ledger_health();
        if h.invariant_broken || h.sqlite_live_layout_bytes != 0 || h.sqlite_live_payload_bytes != 0
        {
            return Err(sql_error(ffi::SQLITE_BUSY));
        }
        let code = unsafe { ffi::sqlite3_config(ffi::SQLITE_CONFIG_MALLOC, &self.original) };
        if code != ffi::SQLITE_OK {
            return Err(sql_error(code));
        }
        ACTIVE.store(ptr::null_mut(), Ordering::Release);
        INSTALLED.store(false, Ordering::Release);
        self.keeper.take();
        Ok(())
    }
}
impl Drop for TestOnlySqliteAllocator {
    fn drop(&mut self) {
        // Drop cannot infer that raw handles are closed, so it must not shutdown or retire core.
        // A lifecycle error leaks only fixed instrumentation in this terminating TestOnly process.
        if let Some(b) = self.keeper.take() {
            std::mem::forget(b);
        }
    }
}

/// Independent fault/quota controls for the exact caller's cfg(test) injection boundary.
#[derive(Clone)]
pub struct RefusalControl {
    budget: OwnedBudget,
}
impl RefusalControl {
    pub fn set_remaining_for_testonly(&self, bytes: usize) {
        let mut m = OwnedBudget::lock(&self.budget.0);
        m.test_ceiling = Some(m.live.checked_add(bytes).unwrap_or(m.limit).min(m.limit));
    }
    /// A deliberate SQLite fault injection, independent of freed cache/quota headroom.
    /// This is not evidence that the original numeric quota alone necessarily rejects.
    pub fn refuse_sqlite_allocations_for_testonly(&self) {
        OwnedBudget::lock(&self.budget.0).reject_sqlite_for_testonly = true;
    }
    pub fn clear_refusal_for_testonly(&self) {
        let mut m = OwnedBudget::lock(&self.budget.0);
        m.test_ceiling = None;
        m.reject_sqlite_for_testonly = false;
    }
}

/// Read converted TEXT and check NOMEM before interpreting NULL or constructing a slice.
/// # Safety
/// stmt must be a live, stepped SQLITE_ROW statement; column must be in range.
/// The returned borrow expires before any step/reset/finalize or conversion of this column.
/// Caller must tie 'a to that statement/row lifetime; it may not outlive the SQLite connection.
pub unsafe fn checked_column_text<'a>(
    stmt: *mut ffi::sqlite3_stmt,
    column: i32,
) -> Result<Option<&'a [u8]>, rusqlite::Error> {
    let db = unsafe { ffi::sqlite3_db_handle(stmt) };
    let p = unsafe { ffi::sqlite3_column_text(stmt, column) };
    // Must immediately check after a failed conversion; column_bytes may clear later evidence.
    let code = unsafe { ffi::sqlite3_errcode(db) };
    if code == ffi::SQLITE_NOMEM {
        return Err(sql_error(code));
    }
    let bytes = unsafe { ffi::sqlite3_column_bytes(stmt, column) };
    let code = unsafe { ffi::sqlite3_errcode(db) };
    if code == ffi::SQLITE_NOMEM {
        return Err(sql_error(code));
    }
    if bytes < 0 || (p.is_null() && bytes != 0) {
        return Err(sql_error(ffi::SQLITE_MISUSE));
    }
    if p.is_null() {
        return Ok(None);
    }
    Ok(Some(unsafe {
        std::slice::from_raw_parts(p, bytes as usize)
    }))
}

#[cfg(test)]
mod tests {
    use super::super::{overlay::OverlaySlab, CaptureSlab};
    use super::*;
    #[test]
    #[ignore = "only exact test in a fresh isolated SQLite process"]
    fn isolated_sqlite_shared_layout_contract() {
        let budget = OwnedBudget::new(50 * 1024 * 1024);
        let guard = unsafe { TestOnlySqliteAllocator::install(&budget) }.unwrap();
        let c = rusqlite::Connection::open_in_memory().unwrap();
        let version = unsafe { std::ffi::CStr::from_ptr(ffi::sqlite3_libversion()) }
            .to_str()
            .unwrap();
        let source = unsafe { std::ffi::CStr::from_ptr(ffi::sqlite3_sourceid()) }
            .to_str()
            .unwrap();
        let options: Vec<String> = c
            .prepare("PRAGMA compile_options")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        println!(
            "SQLITE_RUNTIME {}",
            serde_json::json!({"version":version,"sourceid":source,"compile_options":options,"threadsafe":unsafe{ffi::sqlite3_threadsafe()}})
        );
        c.execute_batch("CREATE TABLE s_content_nodes(digest BLOB PRIMARY KEY,tag INTEGER,body BLOB) WITHOUT ROWID").unwrap();
        c.execute(
            "INSERT INTO s_content_nodes VALUES(?1,1,?2)",
            rusqlite::params![[1u8; 32].as_slice(), [7u8; 4096].as_slice()],
        )
        .unwrap();
        let mut capture = CaptureSlab::new(1, 4096, &budget).unwrap();
        capture.push([1; 32], 1, &[7; 4096]).unwrap();
        let capture = capture.finish().unwrap();
        let mut overlay = OverlaySlab::new(1, 4096, &budget).unwrap();
        overlay.put([2; 32], 2, &[9; 4096]).unwrap();
        let before = budget.snapshot();
        let health = budget.ledger_health();
        assert!(health.sqlite_live_layout_bytes > 0);
        assert!(before.live_requested_layout_bytes > health.sqlite_live_layout_bytes);
        guard.set_remaining_for_testonly(0);
        let error = c
            .prepare("SELECT digest,tag,body FROM s_content_nodes ORDER BY digest")
            .err()
            .unwrap();
        assert_eq!(
            error.sqlite_error_code(),
            Some(rusqlite::ErrorCode::OutOfMemory)
        );
        assert_eq!(
            budget.snapshot().allocated_layouts,
            before.allocated_layouts
        );
        assert_eq!(capture.original(&[1; 32]).unwrap().1, &[7; 4096]);
        assert_eq!(overlay.original(&[2; 32]).unwrap().1, &[9; 4096]);
        println!(
            "SQLITE_PREPARE_REFUSAL {}",
            serde_json::json!({"error":error.to_string(),"before":before,"health_before":health,"after":budget.snapshot(),"request":budget.allocation_request_probe()})
        );
        guard.clear_refusal_for_testonly();
        let mut statement = ptr::null_mut();
        assert_eq!(
            unsafe {
                ffi::sqlite3_prepare_v2(
                    c.handle(),
                    b"SELECT zeroblob(65536)\0".as_ptr().cast(),
                    -1,
                    &mut statement,
                    ptr::null_mut(),
                )
            },
            ffi::SQLITE_OK
        );
        assert_eq!(unsafe { ffi::sqlite3_step(statement) }, ffi::SQLITE_ROW);
        assert_eq!(
            unsafe { ffi::sqlite3_column_type(statement, 0) },
            ffi::SQLITE_BLOB
        );
        guard.set_remaining_for_testonly(0);
        let before_text = budget.snapshot();
        let text_error = unsafe { checked_column_text(statement, 0) }.unwrap_err();
        assert_eq!(
            text_error.sqlite_error_code(),
            Some(rusqlite::ErrorCode::OutOfMemory)
        );
        assert_eq!(
            budget.snapshot().allocated_layouts,
            before_text.allocated_layouts
        );
        println!(
            "SQLITE_TEXT_REFUSAL {}",
            serde_json::json!({"error":text_error.to_string(),"before":before_text,"after":budget.snapshot(),"request":budget.allocation_request_probe()})
        );
        guard.clear_refusal_for_testonly();
        unsafe {
            ffi::sqlite3_finalize(statement);
        }
        let old = unsafe { ffi::sqlite3_malloc(80) };
        assert!(!old.is_null());
        unsafe {
            ptr::write_bytes(old.cast::<u8>(), 0x5a, 80);
        }
        let old_live = budget.snapshot();
        guard.set_remaining_for_testonly(block_layout(160).unwrap().size() - 1);
        let grown = unsafe { ffi::sqlite3_realloc(old, 160) };
        assert!(grown.is_null());
        assert_eq!(unsafe { ffi::sqlite3_msize(old) }, 80);
        assert_eq!(
            unsafe { std::slice::from_raw_parts(old.cast::<u8>(), 80) },
            &[0x5a; 80]
        );
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            old_live.live_requested_layout_bytes
        );
        assert_eq!(
            budget.snapshot().allocated_layouts,
            old_live.allocated_layouts
        );
        println!(
            "SQLITE_REALLOC_REFUSAL {}",
            serde_json::json!({"before":old_live,"after":budget.snapshot(),"request":budget.allocation_request_probe(),"old_payload":80,"new_payload":160})
        );
        guard.clear_refusal_for_testonly();
        let grown = unsafe { ffi::sqlite3_realloc(old, 160) };
        assert!(!grown.is_null());
        assert_eq!(
            unsafe { std::slice::from_raw_parts(grown.cast::<u8>(), 80) },
            &[0x5a; 80]
        );
        unsafe {
            ffi::sqlite3_free(grown);
        }
        drop(c);
        assert_eq!(budget.ledger_health().sqlite_live_layout_bytes, 0);
        assert!(budget.snapshot().live_requested_layout_bytes > 0);
        unsafe { guard.finish() }.unwrap();
        // Restore does not erase any still-live Rust owners from the request's ledger.
        assert!(budget.snapshot().live_requested_layout_bytes > 0);
        drop(overlay);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert!(!budget.ledger_health().invariant_broken);
        assert_eq!(
            budget.snapshot().allocated_layouts,
            budget.snapshot().released_layouts
        );
        println!(
            "SQLITE_FINAL_RECEIPT {}",
            serde_json::json!({"meter":budget.snapshot(),"health":budget.ledger_health(),"whole_audit_allocation_admitted":false,"production_capture":false})
        );
    }
}
