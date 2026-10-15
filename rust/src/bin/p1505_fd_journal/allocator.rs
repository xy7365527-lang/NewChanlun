//! #1505 独立 reader 的 SQLite 默认 allocator 观察。必须在任何 SQLite 初始化前 install。
//! 不改变分配策略，不替换 xSize/xRoundup 语义；回调不分配、不日志、不调用 SQLite。
#![deny(unsafe_op_in_unsafe_fn)]
use rusqlite::ffi;
use std::cell::UnsafeCell;
use std::ffi::{c_int, c_void};
use std::mem::MaybeUninit;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

struct Methods(UnsafeCell<MaybeUninit<ffi::sqlite3_mem_methods>>);
// SAFETY: install 在单线程新进程中只执行一次；发布以后原 methods 永不再写。
unsafe impl Sync for Methods {}
static ORIGINAL: Methods = Methods(UnsafeCell::new(MaybeUninit::uninit()));
static INSTALLED: AtomicBool = AtomicBool::new(false);
static BROKEN: AtomicBool = AtomicBool::new(false);
static REQUEST: AtomicU64 = AtomicU64::new(0);
static ROUNDED: AtomicU64 = AtomicU64::new(0);
static ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static REALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static FREE_CALLS: AtomicU64 = AtomicU64::new(0);
static FAILURES: AtomicU64 = AtomicU64::new(0);
static CURRENT: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);
static REALLOC_OLD_PLUS_REQUEST: AtomicU64 = AtomicU64::new(0);
static INIT_CALLS: AtomicU64 = AtomicU64::new(0);
static SHUTDOWN_CALLS: AtomicU64 = AtomicU64::new(0);
static FAIL_NEXT: AtomicBool = AtomicBool::new(false);
static CONFIGURED_LIMIT: AtomicU64 = AtomicU64::new(0);
static INIT_CURRENT: AtomicU64 = AtomicU64::new(0);
static INIT_PEAK: AtomicU64 = AtomicU64::new(0);
fn add(x: &AtomicU64, n: u64) {
    if x.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| v.checked_add(n))
        .is_err()
    {
        BROKEN.store(true, Ordering::SeqCst);
    }
}
fn sub(x: &AtomicU64, n: u64) {
    if x.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| v.checked_sub(n))
        .is_err()
    {
        BROKEN.store(true, Ordering::SeqCst);
    }
}
fn peak() {
    PEAK.fetch_max(CURRENT.load(Ordering::SeqCst), Ordering::SeqCst);
}
unsafe fn original() -> &'static ffi::sqlite3_mem_methods {
    // SAFETY: SQLite 只能在 install 完成原表写入且设置 wrapper 后回调。
    unsafe { (&*ORIGINAL.0.get()).assume_init_ref() }
}
unsafe fn actual(p: *mut c_void) -> u64 {
    if p.is_null() {
        return 0;
    }
    let m = unsafe { original() };
    let Some(f) = m.xSize else {
        BROKEN.store(true, Ordering::SeqCst);
        return 0;
    };
    let n = unsafe { f(p) };
    if n < 0 {
        BROKEN.store(true, Ordering::SeqCst);
        0
    } else {
        n as u64
    }
}
unsafe extern "C" fn malloc(n: c_int) -> *mut c_void {
    add(&ALLOC_CALLS, 1);
    if FAIL_NEXT.load(Ordering::SeqCst) {
        add(&FAILURES, 1);
        return ptr::null_mut();
    }
    let Some(f) = unsafe { original() }.xMalloc else {
        BROKEN.store(true, Ordering::SeqCst);
        return ptr::null_mut();
    };
    let p = unsafe { f(n) };
    if p.is_null() {
        add(&FAILURES, 1);
    } else {
        add(&CURRENT, unsafe { actual(p) });
        peak();
    }
    p
}
unsafe extern "C" fn realloc(p: *mut c_void, n: c_int) -> *mut c_void {
    add(&REALLOC_CALLS, 1);
    let old = unsafe { actual(p) };
    if let Some(bound) = CURRENT.load(Ordering::SeqCst).checked_add(n.max(0) as u64) {
        REALLOC_OLD_PLUS_REQUEST.fetch_max(bound, Ordering::SeqCst);
    } else {
        BROKEN.store(true, Ordering::SeqCst);
    }
    if FAIL_NEXT.load(Ordering::SeqCst) {
        add(&FAILURES, 1);
        return ptr::null_mut();
    }
    let Some(f) = unsafe { original() }.xRealloc else {
        BROKEN.store(true, Ordering::SeqCst);
        return ptr::null_mut();
    };
    let q = unsafe { f(p, n) };
    if q.is_null() {
        add(&FAILURES, 1);
    } else {
        sub(&CURRENT, old);
        add(&CURRENT, unsafe { actual(q) });
        peak();
    }
    q
}
unsafe extern "C" fn free(p: *mut c_void) {
    let old = unsafe { actual(p) };
    let Some(f) = unsafe { original() }.xFree else {
        BROKEN.store(true, Ordering::SeqCst);
        return;
    };
    unsafe { f(p) };
    if !p.is_null() {
        sub(&CURRENT, old);
        add(&FREE_CALLS, 1);
    }
}
unsafe extern "C" fn size(p: *mut c_void) -> c_int {
    {
        let Some(f) = unsafe { original() }.xSize else {
            BROKEN.store(true, Ordering::SeqCst);
            return 0;
        };
        unsafe { f(p) }
    }
}
unsafe extern "C" fn roundup(n: c_int) -> c_int {
    if n > 0 {
        add(&REQUEST, n as u64);
    }
    let Some(f) = unsafe { original() }.xRoundup else {
        BROKEN.store(true, Ordering::SeqCst);
        return 0;
    };
    let rounded = unsafe { f(n) };
    if rounded > 0 {
        add(&ROUNDED, rounded as u64);
    }
    rounded
}
unsafe extern "C" fn init(data: *mut c_void) -> c_int {
    add(&INIT_CALLS, 1);
    {
        let Some(f) = unsafe { original() }.xInit else {
            BROKEN.store(true, Ordering::SeqCst);
            return ffi::SQLITE_NOMEM;
        };
        unsafe { f(data) }
    }
}
unsafe extern "C" fn shutdown(data: *mut c_void) {
    let Some(f) = unsafe { original() }.xShutdown else {
        BROKEN.store(true, Ordering::SeqCst);
        return;
    };
    unsafe { f(data) };
    add(&SHUTDOWN_CALLS, 1);
}

pub fn install(limit: u64) -> Result<(), String> {
    if limit == 0 || limit > i64::MAX as u64 || INSTALLED.swap(true, Ordering::SeqCst) {
        return Err("invalid/repeated allocator install".into());
    }
    CONFIGURED_LIMIT.store(limit, Ordering::SeqCst);
    let mut original: ffi::sqlite3_mem_methods = unsafe { std::mem::zeroed() };
    let rc = unsafe { ffi::sqlite3_config(ffi::SQLITE_CONFIG_GETMALLOC, &mut original as *mut _) };
    if rc != ffi::SQLITE_OK {
        return Err(format!("GETMALLOC before init rc={rc}"));
    }
    if original.xMalloc.is_none()
        || original.xRealloc.is_none()
        || original.xFree.is_none()
        || original.xSize.is_none()
        || original.xRoundup.is_none()
        || original.xInit.is_none()
        || original.xShutdown.is_none()
    {
        return Err("incomplete default allocator".into());
    }
    let data = original.pAppData;
    unsafe {
        (*ORIGINAL.0.get()).write(original);
    }
    let wrapper = ffi::sqlite3_mem_methods {
        xMalloc: Some(malloc),
        xFree: Some(free),
        xRealloc: Some(realloc),
        xSize: Some(size),
        xRoundup: Some(roundup),
        xInit: Some(init),
        xShutdown: Some(shutdown),
        pAppData: data,
    };
    let rc = unsafe { ffi::sqlite3_config(ffi::SQLITE_CONFIG_MEMSTATUS, 1 as c_int) };
    if rc != ffi::SQLITE_OK {
        return Err(format!("MEMSTATUS rc={rc}"));
    }
    let rc = unsafe { ffi::sqlite3_config(ffi::SQLITE_CONFIG_MALLOC, &wrapper as *const _) };
    if rc != ffi::SQLITE_OK {
        return Err(format!("MALLOC install rc={rc}"));
    }
    let rc = unsafe { ffi::sqlite3_initialize() };
    if rc != ffi::SQLITE_OK {
        return Err(format!("init rc={rc}"));
    }
    INIT_CURRENT.store(CURRENT.load(Ordering::SeqCst), Ordering::SeqCst);
    INIT_PEAK.store(PEAK.load(Ordering::SeqCst), Ordering::SeqCst);
    let previous = unsafe { ffi::sqlite3_hard_heap_limit64(limit as i64) };
    let effective = unsafe { ffi::sqlite3_hard_heap_limit64(-1) };
    if previous < 0 || effective != limit as i64 {
        return Err("heap limit not effective".into());
    }
    if INIT_PEAK.load(Ordering::SeqCst) > limit {
        return Err(format!(
            "initialization xSize peak {} exceeded configured {} (detected after allocation)",
            INIT_PEAK.load(Ordering::SeqCst),
            limit
        ));
    }
    Ok(())
}
pub fn allocation_contract() -> Result<serde_json::Value, String> {
    let original_limit = unsafe { ffi::sqlite3_hard_heap_limit64(-1) };
    let p = unsafe { ffi::sqlite3_malloc64(64) };
    if p.is_null() {
        return Err("probe seed allocation failed".into());
    }
    unsafe {
        std::ptr::write_bytes(p.cast::<u8>(), 0x6A, 64);
    }
    let before = snapshot()?;
    FAIL_NEXT.store(true, Ordering::SeqCst);
    let q = unsafe { ffi::sqlite3_realloc64(p, 65536) };
    FAIL_NEXT.store(false, Ordering::SeqCst);
    if !q.is_null() {
        unsafe { ffi::sqlite3_free(q) };
        return Err("realloc failure not reached".into());
    }
    let after = snapshot()?;
    let preserved = unsafe { std::slice::from_raw_parts(p.cast::<u8>(), 64) }
        .iter()
        .all(|&x| x == 0x6A);
    unsafe { ffi::sqlite3_free(p) };
    if !preserved || before["xsize_current_bytes"] != after["xsize_current_bytes"] {
        return Err("realloc failure lost old allocation".into());
    }
    FAIL_NEXT.store(true, Ordering::SeqCst);
    let failed = unsafe { ffi::sqlite3_malloc64(16384) };
    FAIL_NEXT.store(false, Ordering::SeqCst);
    if !failed.is_null() {
        unsafe { ffi::sqlite3_free(failed) };
        return Err("malloc failure not reached".into());
    }
    let used = CURRENT.load(Ordering::SeqCst);
    let tight = used.checked_add(16).ok_or("heap arithmetic")?;
    unsafe {
        ffi::sqlite3_hard_heap_limit64(tight as i64);
    }
    let capped = unsafe { ffi::sqlite3_malloc64(65536) };
    unsafe {
        ffi::sqlite3_hard_heap_limit64(original_limit);
    }
    if !capped.is_null() {
        unsafe { ffi::sqlite3_free(capped) };
        return Err("hard heap cap did not reject".into());
    }
    Ok(
        serde_json::json!({"realloc_old_bytes_preserved":preserved,"before_failure":before,"after_failure":after,
        "actual_malloc_failed":true,"actual_hard_heap_rejection":true,"restored_limit_bytes":original_limit,"after":snapshot()?}),
    )
}
pub fn snapshot() -> Result<serde_json::Value, String> {
    let mut current = 0_i64;
    let mut highwater = 0_i64;
    let rc = unsafe {
        ffi::sqlite3_status64(
            ffi::SQLITE_STATUS_MEMORY_USED,
            &mut current,
            &mut highwater,
            0,
        )
    };
    if rc != ffi::SQLITE_OK {
        return Err(format!("status rc={rc}"));
    }
    if current < 0
        || current as u64 != CURRENT.load(Ordering::SeqCst)
        || BROKEN.load(Ordering::SeqCst)
    {
        return Err("allocator accounting mismatch".into());
    }
    let limit = CONFIGURED_LIMIT.load(Ordering::SeqCst);
    let measured_peak = PEAK.load(Ordering::SeqCst).max(highwater.max(0) as u64);
    if measured_peak > limit {
        return Err(format!("measured xSize peak {measured_peak} exceeded configured {limit} (detected after allocation)"));
    }
    Ok(
        serde_json::json!({"raw_request_bytes":REQUEST.load(Ordering::SeqCst),"rounded_request_bytes":ROUNDED.load(Ordering::SeqCst),
        "allocation_calls":ALLOC_CALLS.load(Ordering::SeqCst),"reallocation_calls":REALLOC_CALLS.load(Ordering::SeqCst),"free_calls":FREE_CALLS.load(Ordering::SeqCst),
        "allocation_failures":FAILURES.load(Ordering::SeqCst),"xsize_current_bytes":CURRENT.load(Ordering::SeqCst),"xsize_peak_bytes":PEAK.load(Ordering::SeqCst),
        "sqlite_status_current_bytes":current,"sqlite_status_peak_bytes":highwater,"realloc_old_plus_requested_observation":REALLOC_OLD_PLUS_REQUEST.load(Ordering::SeqCst),
        "realloc_allocator_internal_peak":"NOT_MEASURED","init_calls":INIT_CALLS.load(Ordering::SeqCst),"shutdown_calls":SHUTDOWN_CALLS.load(Ordering::SeqCst),
        "configured_limit_bytes":limit,"initialization_xsize_current_bytes":INIT_CURRENT.load(Ordering::SeqCst),"initialization_xsize_peak_bytes":INIT_PEAK.load(Ordering::SeqCst),
        "observed_xsize_within_configured_limit":true,"physical_preallocation_cap":"NOT_PROVEN","initialization_observed_before_hard_limit_installed":true,
        "request_events_are_not_successful_allocations":true,"memstatus_enabled":true,"custom_pcache2":false,"not_process_rss_cap":true}),
    )
}
pub fn finish() -> Result<serde_json::Value, String> {
    let before = snapshot()?;
    let rc = unsafe { ffi::sqlite3_shutdown() };
    if rc != ffi::SQLITE_OK || CURRENT.load(Ordering::SeqCst) != 0 || BROKEN.load(Ordering::SeqCst)
    {
        return Err("allocator shutdown leaked or failed".into());
    }
    // 不再调用会隐式初始化SQLite的API；原 methods 与 wrapper 静态存活至独立进程结束。
    Ok(
        serde_json::json!({"before_shutdown":before,"after_shutdown_xsize_bytes":CURRENT.load(Ordering::SeqCst),
        "shutdown_calls":SHUTDOWN_CALLS.load(Ordering::SeqCst),"after_shutdown_free_calls":FREE_CALLS.load(Ordering::SeqCst)}),
    )
}
