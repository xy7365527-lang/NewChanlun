//! #1505 D2 原型：由 D1 b9f38106 的 VFS 派生。固定 O_RDONLY main/journal，虚拟 RW 仅作用于私有 overlay。
//! 私有虚拟锁只在本连接内有效，外部排他性来自隔离夹具假设，不是本 VFS 的保证。
//! SAFETY 总契约：SQLite 给出的回调指针/缓冲区须符合其 C API；非默认注册对象
//! 地址稳定，Registration 活至所有 Connection/Statement/file 关闭之后；不导出裸 handle。
#![deny(unsafe_op_in_unsafe_fn)]

use rusqlite::{config::DbConfig, ffi, Connection, OpenFlags};
use serde::Serialize;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::fs::{File, OpenOptions};
use std::marker::PhantomData;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const EXPECTED_SOURCE_ID: &str =
    "2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24";
use super::overlay::{self, FallibleBox, Fixed, Overlay, JOURNAL, MAIN};
use super::Limits;
use std::fmt::Write as FmtWrite;
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static REGISTRATIONS: AtomicU64 = AtomicU64::new(0);
static OPENS: AtomicU64 = AtomicU64::new(0);
pub fn counters() -> serde_json::Value {
    serde_json::json!({"registrations":REGISTRATIONS.load(Ordering::SeqCst),"open_calls":OPENS.load(Ordering::SeqCst)})
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Fault {
    OpenNoMem = 1,
    ReadIo = 2,
    ReadPanic = 3,
    ReadShort = 4,
    ReadInterrupted = 5,
    CloseIo = 6,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Identity {
    dev: u64,
    ino: u64,
    len: u64,
    nlink: u64,
    mtime: i64,
    mtime_nsec: i64,
    readonly_fd: bool,
}

struct Source {
    file: File,
    identity: Identity,
}

impl Source {
    fn identity(file: &File) -> std::io::Result<Identity> {
        let m = file.metadata()?;
        // SAFETY: F_GETFL 只查询活着的 File 所拥有的 FD。
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        if flags < 0 {
            return Err(std::io::Error::last_os_error());
        }
        if !m.is_file() || m.nlink() != 1 || flags & libc::O_ACCMODE != libc::O_RDONLY {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        }
        Ok(Identity {
            dev: m.dev(),
            ino: m.ino(),
            len: m.len(),
            nlink: m.nlink(),
            mtime: m.mtime(),
            mtime_nsec: m.mtime_nsec(),
            readonly_fd: true,
        })
    }

    fn pin(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        let identity = Self::identity(&file)?;
        if identity.len > i64::MAX as u64 {
            return Err("source too large".into());
        }
        Ok(Self { file, identity })
    }

    fn unchanged(&self) -> bool {
        Self::identity(&self.file).is_ok_and(|now| now == self.identity)
    }
}

#[derive(Clone, Copy, Serialize)]
struct Event {
    op: &'static str,
    kind: i32,
    a: i64,
    b: i64,
    rc: i32,
    exclusive_armed: bool,
}

#[derive(Clone, Copy)]
struct Name {
    bytes: [u8; 128],
    len: usize,
}
impl std::fmt::Write for Name {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let end = self.len.checked_add(s.len()).ok_or(std::fmt::Error)?;
        if end >= self.bytes.len() || s.as_bytes().contains(&0) {
            return Err(std::fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
impl Name {
    fn make(args: std::fmt::Arguments<'_>) -> Result<Self> {
        let mut out = Self {
            bytes: [0; 128],
            len: 0,
        };
        out.write_fmt(args)?;
        Ok(out)
    }
    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
    fn as_bytes_with_nul(&self) -> &[u8] {
        &self.bytes[..=self.len]
    }
    fn as_ptr(&self) -> *const c_char {
        self.bytes.as_ptr().cast()
    }
    fn as_c_str(&self) -> &CStr {
        unsafe { CStr::from_bytes_with_nul_unchecked(self.as_bytes_with_nul()) }
    }
    fn to_str(&self) -> std::result::Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.as_bytes())
    }
}

struct Context {
    owner_thread: libc::pthread_t,
    callback_active: AtomicBool,
    sqlite_worker_limit: AtomicI32,
    registration_name: Name,
    main: Source,
    journal: Option<Source>,
    main_name: Name,
    wal_name: Name,
    journal_name: Name,
    shm_name: Name,
    events: RefCell<Fixed<Event>>,
    overlay: RefCell<Overlay>,
    trace_overflow: AtomicBool,
    main_open: AtomicBool,
    journal_open: AtomicBool,
    active: AtomicUsize,
    exclusive_armed: AtomicBool,
    fault: AtomicI32,
    caught_panics: AtomicUsize,
    close_errors: AtomicUsize,
    sql_denials: AtomicUsize,
    read_bytes: AtomicU64,
    retries: AtomicUsize,
}

impl Context {
    fn source(&self, kind: i32) -> Option<&Source> {
        match kind {
            MAIN => Some(&self.main),
            JOURNAL => self.journal.as_ref(),
            _ => None,
        }
    }
    fn take_fault(&self, f: Fault) -> bool {
        self.fault
            .compare_exchange(f as i32, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }
    fn record(&self, op: &'static str, kind: i32, a: i64, b: i64, rc: i32) -> i32 {
        let Ok(mut events) = self.events.try_borrow_mut() else {
            return ffi::SQLITE_IOERR;
        };
        if events.len() == events.capacity() {
            self.trace_overflow.store(true, Ordering::SeqCst);
            return ffi::SQLITE_IOERR_NOMEM;
        }
        if !events.push(Event {
            op,
            kind,
            a,
            b,
            rc,
            exclusive_armed: self.exclusive_armed.load(Ordering::SeqCst),
        }) {
            self.trace_overflow.store(true, Ordering::SeqCst);
            return ffi::SQLITE_IOERR_NOMEM;
        }
        rc
    }
}

#[repr(C)]
struct PinnedFile {
    base: ffi::sqlite3_file, // SQLite 的首字段布局；SQLite 分配 szOsFile 字节。
    ctx: *const Context,
    kind: i32,
    lock: i32,
    writable: bool,
}

// SAFETY: 仅 SQLite 在本注册对象存活期间调用；pAppData 是稳定 Box<Context>。
// 先仅读不可变 owner 域；跨线程拒绝前不创建 &Context、不触碰 RefCell。
unsafe fn checked_context<'a>(raw: *const Context) -> Option<&'a Context> {
    if raw.is_null()
        || unsafe {
            libc::pthread_equal(
                ptr::addr_of!((*raw).owner_thread).read(),
                libc::pthread_self(),
            )
        } == 0
    {
        None
    } else {
        Some(unsafe { &*raw })
    }
}
unsafe fn context<'a>(vfs: *mut ffi::sqlite3_vfs) -> Option<&'a Context> {
    unsafe { checked_context(ptr::addr_of!((*vfs).pAppData).read().cast::<Context>()) }
}
unsafe fn file_context<'a>(file: *mut ffi::sqlite3_file) -> Option<&'a Context> {
    let raw = unsafe { ptr::addr_of!((*file.cast::<PinnedFile>()).ctx).read() };
    unsafe { checked_context(raw) }
}

// 每个 C ABI 的有状态路径都在此捕获 Rust unwind；OOM-abort/OS kill 不伪装成可捕获。
fn guarded(ctx: &Context, f: impl FnOnce() -> i32) -> i32 {
    if ctx.callback_active.swap(true, Ordering::SeqCst) {
        return ffi::SQLITE_MISUSE;
    }
    struct CallbackGuard<'a>(&'a AtomicBool);
    impl Drop for CallbackGuard<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _guard = CallbackGuard(&ctx.callback_active);
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(rc) => rc,
        Err(_) => {
            ctx.caught_panics.fetch_add(1, Ordering::SeqCst);
            ffi::SQLITE_IOERR
        }
    }
}

unsafe fn bytes<'a>(name: *const c_char) -> Option<&'a [u8]> {
    if name.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(name) }.to_bytes())
    }
}

unsafe extern "C" fn open(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    out: *mut ffi::sqlite3_file,
    flags: c_int,
    out_flags: *mut c_int,
) -> c_int {
    OPENS.fetch_add(1, Ordering::SeqCst);
    // SAFETY: SQLite 提供对齐且可写的 szOsFile 存储。失败初始 pMethods=NULL。
    unsafe {
        (*out).pMethods = ptr::null();
    }
    if !out_flags.is_null() {
        unsafe {
            *out_flags = 0;
        }
    }
    let Some(ctx) = (unsafe { context(vfs) }) else {
        return ffi::SQLITE_MISUSE;
    };
    guarded(ctx, || {
        let supported = ffi::SQLITE_OPEN_READONLY
            | ffi::SQLITE_OPEN_READWRITE
            | ffi::SQLITE_OPEN_MAIN_DB
            | ffi::SQLITE_OPEN_MAIN_JOURNAL;
        if flags & !supported != 0 {
            return ctx.record(
                "open-capability-denied",
                0,
                flags.into(),
                0,
                ffi::SQLITE_CANTOPEN,
            );
        }
        let name = unsafe { bytes(name) };
        let access = flags & (ffi::SQLITE_OPEN_READONLY | ffi::SQLITE_OPEN_READWRITE);
        if ![ffi::SQLITE_OPEN_READONLY, ffi::SQLITE_OPEN_READWRITE].contains(&access) {
            return ctx.record(
                "open-access-denied",
                0,
                flags.into(),
                0,
                ffi::SQLITE_CANTOPEN,
            );
        }
        let types = ffi::SQLITE_OPEN_MAIN_DB
            | ffi::SQLITE_OPEN_TEMP_DB
            | ffi::SQLITE_OPEN_TRANSIENT_DB
            | ffi::SQLITE_OPEN_MAIN_JOURNAL
            | ffi::SQLITE_OPEN_TEMP_JOURNAL
            | ffi::SQLITE_OPEN_SUBJOURNAL
            | ffi::SQLITE_OPEN_SUPER_JOURNAL
            | ffi::SQLITE_OPEN_WAL;
        let kind = if name == Some(ctx.main_name.as_bytes())
            && flags & types == ffi::SQLITE_OPEN_MAIN_DB
            && flags & ffi::SQLITE_OPEN_READWRITE != 0
            && flags & ffi::SQLITE_OPEN_CREATE == 0
        {
            MAIN
        } else if name == Some(ctx.journal_name.as_bytes())
            && flags & types == ffi::SQLITE_OPEN_MAIN_JOURNAL
            && ctx.journal.is_some()
            && ctx
                .overlay
                .try_borrow()
                .is_ok_and(|o| o.state(JOURNAL).is_ok_and(|s| s.exists))
            && flags & ffi::SQLITE_OPEN_CREATE == 0
            && ctx.exclusive_armed.load(Ordering::SeqCst)
            && ctx.main_open.load(Ordering::SeqCst)
        {
            JOURNAL
        } else {
            return ctx.record("open-denied", 0, flags.into(), 0, ffi::SQLITE_CANTOPEN);
        };
        if flags & (ffi::SQLITE_OPEN_URI | ffi::SQLITE_OPEN_DELETEONCLOSE | ffi::SQLITE_OPEN_MEMORY)
            != 0
        {
            return ctx.record("open-denied", kind, flags.into(), 0, ffi::SQLITE_CANTOPEN);
        }
        if ctx.take_fault(Fault::OpenNoMem) {
            return ctx.record("open-oom", kind, flags.into(), 0, ffi::SQLITE_NOMEM);
        }
        let opened = if kind == MAIN {
            &ctx.main_open
        } else {
            &ctx.journal_open
        };
        if opened.swap(true, Ordering::SeqCst) {
            return ctx.record(
                "open-second-denied",
                kind,
                flags.into(),
                0,
                ffi::SQLITE_CANTOPEN,
            );
        }
        if kind == MAIN {
            ctx.exclusive_armed.store(false, Ordering::SeqCst);
        }
        // 不分配、不 dup、不重新解析路径；file 仅借 Registry 固定持有的 FD。
        unsafe {
            ptr::write(
                out.cast::<PinnedFile>(),
                PinnedFile {
                    base: ffi::sqlite3_file { pMethods: &METHODS },
                    ctx,
                    kind,
                    lock: ffi::SQLITE_LOCK_NONE,
                    writable: flags & ffi::SQLITE_OPEN_READWRITE != 0,
                },
            );
        }
        ctx.active.fetch_add(1, Ordering::SeqCst);
        if !out_flags.is_null() {
            unsafe {
                *out_flags = flags & (ffi::SQLITE_OPEN_READONLY | ffi::SQLITE_OPEN_READWRITE);
            }
        }
        // 已安装有效对象后若轨迹满，SQLite 仍可用 xClose 正常释放。
        ctx.record(
            "open",
            kind,
            flags.into(),
            i64::from(flags & (ffi::SQLITE_OPEN_READONLY | ffi::SQLITE_OPEN_READWRITE)),
            ffi::SQLITE_OK,
        )
    })
}

unsafe extern "C" fn close(file: *mut ffi::sqlite3_file) -> c_int {
    let Some(ctx) = (unsafe { file_context(file) }) else {
        return ffi::SQLITE_MISUSE;
    };
    guarded(ctx, || {
        // 只访问字段；绝不创建覆盖整个file的 &mut，重入前的ctx裸读不会与其别名冲突。
        let f = file.cast::<PinnedFile>();
        let kind = unsafe { ptr::addr_of!((*f).kind).read() };
        if unsafe { ptr::addr_of!((*f).base.pMethods).read() }.is_null() {
            return ffi::SQLITE_IOERR_CLOSE;
        }
        unsafe {
            ptr::addr_of_mut!((*f).base.pMethods).write(ptr::null());
        }
        if kind == MAIN {
            ctx.main_open.store(false, Ordering::SeqCst);
        } else {
            ctx.journal_open.store(false, Ordering::SeqCst);
        }
        ctx.active.fetch_sub(1, Ordering::SeqCst);
        let rc = if ctx.take_fault(Fault::CloseIo) {
            ctx.close_errors.fetch_add(1, Ordering::SeqCst);
            ffi::SQLITE_IOERR_CLOSE
        } else {
            ffi::SQLITE_OK
        };
        ctx.record("close", kind, 0, 0, rc)
    })
}

unsafe extern "C" fn read(
    file: *mut ffi::sqlite3_file,
    out: *mut c_void,
    amount: c_int,
    offset: i64,
) -> c_int {
    let Some(ctx) = (unsafe { file_context(file) }) else {
        return ffi::SQLITE_MISUSE;
    };
    let rc = guarded(ctx, || {
        let f = unsafe { &*file.cast::<PinnedFile>() };
        if amount < 0
            || offset < 0
            || offset.checked_add(i64::from(amount)).is_none()
            || (amount > 0 && out.is_null())
        {
            return ffi::SQLITE_IOERR_READ;
        }
        if amount == 0 {
            return ffi::SQLITE_OK;
        }
        // SAFETY: SQLite 承诺 out 可写 amount 字节；先清零满足 short-read 合同。
        let buf = unsafe { std::slice::from_raw_parts_mut(out.cast::<u8>(), amount as usize) };
        buf.fill(0);
        if ctx.take_fault(Fault::ReadPanic) {
            panic!("1505 controlled callback panic");
        }
        if ctx.take_fault(Fault::ReadIo) {
            return ffi::SQLITE_IOERR_READ;
        }
        if ctx.take_fault(Fault::ReadShort) {
            return ffi::SQLITE_IOERR_SHORT_READ;
        }
        let Some(source) = ctx.source(f.kind) else {
            return ffi::SQLITE_IOERR_READ;
        };
        if !source.unchanged() {
            return ffi::SQLITE_IOERR_FSTAT;
        }
        if !ctx.exclusive_armed.load(Ordering::SeqCst)
            && !(f.kind == MAIN && offset == 0 && amount == 100)
        {
            return ffi::SQLITE_IOERR_READ;
        }
        if ctx.take_fault(Fault::ReadInterrupted) {
            // 仅可控计数探针；没有让宿主 pread 返回 EINTR，不计真实 EINTR 覆盖。
            ctx.retries.fetch_add(1, Ordering::SeqCst);
        }
        let Ok(overlay) = ctx.overlay.try_borrow() else {
            return ffi::SQLITE_IOERR;
        };
        let result = overlay.read(f.kind, &source.file, offset as u64, buf);
        if !source.unchanged() {
            return ffi::SQLITE_IOERR_FSTAT;
        }
        ctx.read_bytes.fetch_add(buf.len() as u64, Ordering::SeqCst);
        match result {
            Ok(false) => ffi::SQLITE_OK,
            Ok(true) => ffi::SQLITE_IOERR_SHORT_READ,
            Err(_) => ffi::SQLITE_IOERR_READ,
        }
    });
    guarded(ctx, || {
        let f = unsafe { &*file.cast::<PinnedFile>() };
        ctx.record("read", f.kind, offset, amount.into(), rc)
    })
}

unsafe fn file_event(file: *mut ffi::sqlite3_file, op: &'static str, a: i64, rc: i32) -> i32 {
    let Some(ctx) = (unsafe { file_context(file) }) else {
        return ffi::SQLITE_MISUSE;
    };
    guarded(ctx, || {
        let f = unsafe { &*file.cast::<PinnedFile>() };
        ctx.record(op, f.kind, a, 0, rc)
    })
}

fn overlay_error(error: overlay::Error) -> i32 {
    match error {
        overlay::Error::Capacity => ffi::SQLITE_IOERR_NOMEM,
        _ => ffi::SQLITE_IOERR,
    }
}
unsafe extern "C" fn write(
    file: *mut ffi::sqlite3_file,
    input: *const c_void,
    n: c_int,
    offset: i64,
) -> c_int {
    let Some(ctx) = (unsafe { file_context(file) }) else {
        return ffi::SQLITE_MISUSE;
    };
    guarded(ctx, || {
        let f = unsafe { &*file.cast::<PinnedFile>() };
        if n < 0
            || offset < 0
            || offset.checked_add(i64::from(n)).is_none()
            || (n > 0 && input.is_null())
        {
            return ffi::SQLITE_IOERR_WRITE;
        }
        if !f.writable || !ctx.exclusive_armed.load(Ordering::SeqCst) {
            return ffi::SQLITE_READONLY;
        }
        if n == 0 {
            return ffi::SQLITE_OK;
        }
        let Some(source) = ctx.source(f.kind) else {
            return ffi::SQLITE_IOERR_WRITE;
        };
        if !source.unchanged() {
            return ffi::SQLITE_IOERR_FSTAT;
        }
        let bytes = unsafe { std::slice::from_raw_parts(input.cast::<u8>(), n as usize) };
        let Ok(mut overlay) = ctx.overlay.try_borrow_mut() else {
            return ffi::SQLITE_IOERR;
        };
        let rc = match overlay.write(f.kind, &source.file, offset as u64, bytes) {
            Ok(()) => ffi::SQLITE_OK,
            Err(e) => overlay_error(e),
        };
        ctx.record("write-overlay", f.kind, offset, n.into(), rc)
    })
}
unsafe extern "C" fn truncate(file: *mut ffi::sqlite3_file, n: i64) -> c_int {
    let Some(ctx) = (unsafe { file_context(file) }) else {
        return ffi::SQLITE_MISUSE;
    };
    guarded(ctx, || {
        let f = unsafe { &*file.cast::<PinnedFile>() };
        if n < 0 {
            return ffi::SQLITE_IOERR_TRUNCATE;
        }
        if !f.writable || !ctx.exclusive_armed.load(Ordering::SeqCst) {
            return ffi::SQLITE_READONLY;
        }
        let Ok(mut overlay) = ctx.overlay.try_borrow_mut() else {
            return ffi::SQLITE_IOERR;
        };
        let rc = match overlay.truncate(f.kind, n as u64) {
            Ok(()) => ffi::SQLITE_OK,
            Err(e) => overlay_error(e),
        };
        ctx.record("truncate-overlay", f.kind, n, 0, rc)
    })
}
unsafe extern "C" fn sync(file: *mut ffi::sqlite3_file, flags: c_int) -> c_int {
    unsafe { file_event(file, "sync-volatile-overlay", flags.into(), ffi::SQLITE_OK) }
}
unsafe extern "C" fn size(file: *mut ffi::sqlite3_file, out: *mut i64) -> c_int {
    let Some(ctx) = (unsafe { file_context(file) }) else {
        return ffi::SQLITE_MISUSE;
    };
    guarded(ctx, || {
        let f = unsafe { &*file.cast::<PinnedFile>() };
        if out.is_null() {
            return ffi::SQLITE_IOERR_FSTAT;
        }
        let Some(source) = ctx.source(f.kind) else {
            return ffi::SQLITE_IOERR_FSTAT;
        };
        if !source.unchanged() {
            return ffi::SQLITE_IOERR_FSTAT;
        }
        let Ok(overlay) = ctx.overlay.try_borrow() else {
            return ffi::SQLITE_IOERR;
        };
        let Ok(st) = overlay.state(f.kind) else {
            return ffi::SQLITE_IOERR_FSTAT;
        };
        unsafe {
            *out = st.logical_len as i64;
        }
        ctx.record("size", f.kind, st.logical_len as i64, 0, ffi::SQLITE_OK)
    })
}
unsafe extern "C" fn lock(file: *mut ffi::sqlite3_file, level: c_int) -> c_int {
    let Some(ctx) = (unsafe { file_context(file) }) else {
        return ffi::SQLITE_MISUSE;
    };
    guarded(ctx, || {
        if !(ffi::SQLITE_LOCK_SHARED..=ffi::SQLITE_LOCK_EXCLUSIVE).contains(&level) {
            return ffi::SQLITE_IOERR_LOCK;
        }
        let f = file.cast::<PinnedFile>();
        let old = unsafe { ptr::addr_of!((*f).lock).read() };
        unsafe {
            ptr::addr_of_mut!((*f).lock).write(old.max(level));
        }
        let kind = unsafe { ptr::addr_of!((*f).kind).read() };
        ctx.record("lock-virtual", kind, level.into(), 0, ffi::SQLITE_OK)
    })
}
unsafe extern "C" fn unlock(file: *mut ffi::sqlite3_file, level: c_int) -> c_int {
    let Some(ctx) = (unsafe { file_context(file) }) else {
        return ffi::SQLITE_MISUSE;
    };
    guarded(ctx, || {
        let f = file.cast::<PinnedFile>();
        let old = unsafe { ptr::addr_of!((*f).lock).read() };
        if ![ffi::SQLITE_LOCK_NONE, ffi::SQLITE_LOCK_SHARED].contains(&level) || level > old {
            return ffi::SQLITE_IOERR_UNLOCK;
        }
        unsafe {
            ptr::addr_of_mut!((*f).lock).write(level);
        }
        let kind = unsafe { ptr::addr_of!((*f).kind).read() };
        ctx.record("unlock-virtual", kind, level.into(), 0, ffi::SQLITE_OK)
    })
}

unsafe extern "C" fn reserved(file: *mut ffi::sqlite3_file, out: *mut c_int) -> c_int {
    let Some(ctx) = (unsafe { file_context(file) }) else {
        return ffi::SQLITE_MISUSE;
    };
    guarded(ctx, || {
        let f = unsafe { &*file.cast::<PinnedFile>() };
        if out.is_null() {
            return ffi::SQLITE_IOERR_CHECKRESERVEDLOCK;
        }
        unsafe {
            *out = i32::from(f.lock >= ffi::SQLITE_LOCK_RESERVED);
        }
        ctx.record("reserved-virtual", f.kind, f.lock.into(), 0, ffi::SQLITE_OK)
    })
}
unsafe extern "C" fn control(file: *mut ffi::sqlite3_file, op: c_int, arg: *mut c_void) -> c_int {
    let Some(ctx) = (unsafe { file_context(file) }) else {
        return ffi::SQLITE_MISUSE;
    };
    guarded(ctx, || {
        let f = unsafe { &*file.cast::<PinnedFile>() };
        let rc = if op == ffi::SQLITE_FCNTL_LOCKSTATE && !arg.is_null() {
            unsafe {
                *arg.cast::<c_int>() = f.lock;
            }
            ffi::SQLITE_OK
        } else {
            ffi::SQLITE_NOTFOUND
        };
        ctx.record("file-control", f.kind, op.into(), 0, rc)
    })
}
unsafe extern "C" fn sector(_: *mut ffi::sqlite3_file) -> c_int {
    512
}
unsafe extern "C" fn characteristics(_: *mut ffi::sqlite3_file) -> c_int {
    0
}

static METHODS: ffi::sqlite3_io_methods = ffi::sqlite3_io_methods {
    iVersion: 1,
    xClose: Some(close),
    xRead: Some(read),
    xWrite: Some(write),
    xTruncate: Some(truncate),
    xSync: Some(sync),
    xFileSize: Some(size),
    xLock: Some(lock),
    xUnlock: Some(unlock),
    xCheckReservedLock: Some(reserved),
    xFileControl: Some(control),
    xSectorSize: Some(sector),
    xDeviceCharacteristics: Some(characteristics),
    xShmMap: None,
    xShmLock: None,
    xShmBarrier: None,
    xShmUnmap: None,
    xFetch: None,
    xUnfetch: None,
};

unsafe extern "C" fn access(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    flags: c_int,
    out: *mut c_int,
) -> c_int {
    let Some(ctx) = (unsafe { context(vfs) }) else {
        return ffi::SQLITE_MISUSE;
    };
    guarded(ctx, || {
        if out.is_null() {
            return ffi::SQLITE_IOERR_ACCESS;
        }
        unsafe {
            *out = 0;
        }
        let name = unsafe { bytes(name) };
        let exists = if name == Some(ctx.main_name.as_bytes()) {
            true
        } else if name == Some(ctx.wal_name.as_bytes()) {
            false
        } else if name == Some(ctx.journal_name.as_bytes()) {
            let Ok(overlay) = ctx.overlay.try_borrow() else {
                return ffi::SQLITE_IOERR_ACCESS;
            };
            let Ok(state) = overlay.state(JOURNAL) else {
                return ffi::SQLITE_IOERR_ACCESS;
            };
            state.exists
        } else if name == Some(ctx.shm_name.as_bytes()) {
            false
        } else {
            return ctx.record(
                "access-domain-denied",
                0,
                flags.into(),
                0,
                ffi::SQLITE_IOERR_ACCESS,
            );
        };
        if ![
            ffi::SQLITE_ACCESS_EXISTS,
            ffi::SQLITE_ACCESS_READ,
            ffi::SQLITE_ACCESS_READWRITE,
        ]
        .contains(&flags)
        {
            return ffi::SQLITE_IOERR_ACCESS;
        }
        unsafe {
            *out = i32::from(exists);
        }
        ctx.record("access", 0, flags.into(), i64::from(exists), ffi::SQLITE_OK)
    })
}
unsafe extern "C" fn delete(vfs: *mut ffi::sqlite3_vfs, name: *const c_char, _: c_int) -> c_int {
    let Some(ctx) = (unsafe { context(vfs) }) else {
        return ffi::SQLITE_MISUSE;
    };
    guarded(ctx, || {
        if unsafe { bytes(name) } != Some(ctx.journal_name.as_bytes())
            || !ctx.exclusive_armed.load(Ordering::SeqCst)
        {
            return ctx.record("delete-domain-denied", 0, 0, 0, ffi::SQLITE_IOERR_DELETE);
        }
        let Ok(mut overlay) = ctx.overlay.try_borrow_mut() else {
            return ffi::SQLITE_IOERR;
        };
        let rc = match overlay.delete_journal() {
            Ok(()) => ffi::SQLITE_OK,
            Err(e) => overlay_error(e),
        };
        ctx.record("delete-virtual-journal", JOURNAL, 0, 0, rc)
    })
}
unsafe extern "C" fn fullpath(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    n: c_int,
    out: *mut c_char,
) -> c_int {
    let Some(ctx) = (unsafe { context(vfs) }) else {
        return ffi::SQLITE_MISUSE;
    };
    guarded(ctx, || {
        if unsafe { bytes(name) } != Some(ctx.main_name.as_bytes())
            || out.is_null()
            || n < 1
            || n as usize <= ctx.main_name.as_bytes().len()
        {
            return ctx.record("fullpath-denied", 0, n.into(), 0, ffi::SQLITE_CANTOPEN);
        }
        unsafe {
            ptr::copy_nonoverlapping(
                ctx.main_name.as_ptr(),
                out,
                ctx.main_name.as_bytes_with_nul().len(),
            );
        }
        ctx.record("fullpath-virtual", 0, n.into(), 0, ffi::SQLITE_OK)
    })
}
unsafe extern "C" fn dl_open(vfs: *mut ffi::sqlite3_vfs, _: *const c_char) -> *mut c_void {
    let Some(ctx) = (unsafe { context(vfs) }) else {
        return ptr::null_mut();
    };
    guarded(ctx, || {
        ctx.record("extension-denied", 0, 0, 0, ffi::SQLITE_AUTH)
    });
    ptr::null_mut()
}
unsafe extern "C" fn dl_error(_: *mut ffi::sqlite3_vfs, n: c_int, out: *mut c_char) {
    if n > 0 && !out.is_null() {
        unsafe {
            *out = 0;
        }
    }
}
unsafe extern "C" fn dl_close(_: *mut ffi::sqlite3_vfs, _: *mut c_void) {}
unsafe extern "C" fn randomness(_: *mut ffi::sqlite3_vfs, n: c_int, out: *mut c_char) -> c_int {
    if n <= 0 || out.is_null() {
        return 0;
    }
    // getentropy 的系统接口不接 pathname；每次最多 256 字节。
    let mut done = 0;
    while done < n {
        let count = (n - done).min(256) as usize;
        if unsafe { libc::getentropy(out.add(done as usize).cast(), count) } != 0 {
            return done;
        }
        done += count as i32;
    }
    n
}
unsafe extern "C" fn sleep(_: *mut ffi::sqlite3_vfs, _: c_int) -> c_int {
    0
}
unsafe extern "C" fn current_time(_: *mut ffi::sqlite3_vfs, out: *mut f64) -> c_int {
    if out.is_null() {
        return ffi::SQLITE_ERROR;
    }
    std::panic::catch_unwind(|| match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => {
            unsafe {
                *out = 2_440_587.5 + d.as_secs_f64() / 86400.0;
            }
            ffi::SQLITE_OK
        }
        Err(_) => ffi::SQLITE_ERROR,
    })
    .unwrap_or(ffi::SQLITE_ERROR)
}

unsafe extern "C" fn authorizer(
    data: *mut c_void,
    action: c_int,
    a: *const c_char,
    b: *const c_char,
    _: *const c_char,
    _: *const c_char,
) -> c_int {
    let Some(ctx) = (unsafe { checked_context(data.cast::<Context>()) }) else {
        return ffi::SQLITE_DENY;
    };
    // 不支持的动作一律 DENY；panic 返回 IOERR 并不是合法 authorizer 返回值，转 DENY。
    let rc = guarded(ctx, || {
        let allowed = match action {
            ffi::SQLITE_SELECT | ffi::SQLITE_READ => true,
            ffi::SQLITE_FUNCTION => {
                matches!(
                    unsafe { bytes(b) },
                    Some(b"count" | b"length" | b"hex" | b"typeof")
                )
            }
            ffi::SQLITE_PRAGMA => match (unsafe { bytes(a) }, unsafe { bytes(b) }) {
                (Some(b"locking_mode"), Some(value)) => {
                    !ctx.exclusive_armed.load(Ordering::SeqCst)
                        && value.eq_ignore_ascii_case(b"exclusive")
                }
                (Some(b"locking_mode"), None) => true,
                (Some(b"query_only"), Some(value)) => value == b"ON" || value == b"1",
                (Some(b"temp_store"), Some(value)) => value.eq_ignore_ascii_case(b"memory"),
                (Some(b"quick_check" | b"page_count" | b"page_size"), None) => true,
                _ => false,
            },
            _ => false,
        };
        if allowed {
            ffi::SQLITE_OK
        } else {
            ctx.sql_denials.fetch_add(1, Ordering::SeqCst);
            ctx.record("sql-denied", 0, action.into(), 0, ffi::SQLITE_DENY)
        }
    });
    if rc == ffi::SQLITE_OK {
        rc
    } else {
        ffi::SQLITE_DENY
    }
}

pub struct Registration {
    vfs: Option<FallibleBox<ffi::sqlite3_vfs>>,
    ctx: Option<FallibleBox<Context>>,
    _not_send_or_sync: PhantomData<Rc<()>>,
}

pub struct Session<'a> {
    connection: Option<Connection>,
    owner: &'a Registration,
}

impl Session<'_> {
    pub fn connection(&self) -> &Connection {
        self.connection.as_ref().expect("live session")
    }
    /// L01 真实负控；局部原始 statement 的寿命始终被本 Session/Registration 覆盖。
    pub fn statement_lifecycle_contract(&mut self) -> Result<serde_json::Value> {
        struct Statement(*mut ffi::sqlite3_stmt);
        impl Drop for Statement {
            fn drop(&mut self) {
                if !self.0.is_null() {
                    unsafe {
                        ffi::sqlite3_finalize(self.0);
                    }
                }
            }
        }
        let c = self.connection.take().ok_or("closed session")?;
        // c先声明、statement后声明：普通早退按逆序先finalize，再析构Connection。
        let original_db = unsafe { c.handle() };
        let mut statement = Statement(ptr::null_mut());
        let sql = b"SELECT id FROM records ORDER BY id\0";
        let prepare_rc = unsafe {
            ffi::sqlite3_prepare_v2(
                original_db,
                sql.as_ptr().cast(),
                -1,
                &mut statement.0,
                ptr::null_mut(),
            )
        };
        if prepare_rc != ffi::SQLITE_OK || statement.0.is_null() {
            return Err(format!("live statement prepare rc={prepare_rc}").into());
        }
        let step_rc = unsafe { ffi::sqlite3_step(statement.0) };
        if step_rc != ffi::SQLITE_ROW {
            return Err(format!("live statement step rc={step_rc}").into());
        }
        let (c, error) = match c.close() {
            Err(pair) => pair,
            Ok(()) => {
                // 违反锁定SQLite的close合同；不再触碰可能失效的stmt，不伪装可恢复。
                statement.0 = ptr::null_mut();
                return Err("close unexpectedly accepted an active statement".into());
            }
        };
        let close_rc = error.sqlite_error().map(|e| e.extended_code).unwrap_or(-1);
        let original_connection_retained = unsafe { c.handle() } == original_db;
        self.connection = Some(c); // 在任何后续错误前，把仍活Connection的所有权交回Session。
                                   // rusqlite close在调用sqlite3_close前移除hooks；BUSY后立即恢复原authorizer。
        let authorizer_rc = unsafe {
            ffi::sqlite3_set_authorizer(
                self.connection().handle(),
                Some(authorizer),
                (self.owner.ctx() as *const Context).cast_mut().cast(),
            )
        };
        if authorizer_rc != ffi::SQLITE_OK {
            return Err("authorizer restoration after busy close failed".into());
        }
        if close_rc != ffi::SQLITE_BUSY || !original_connection_retained {
            return Err(format!("unexpected live statement close rc={close_rc}").into());
        }
        let active = self.owner.ctx().active.load(Ordering::SeqCst);
        let expected =
            self.owner.vfs.as_deref().ok_or("missing owner VFS")? as *const ffi::sqlite3_vfs;
        let registered =
            unsafe { ffi::sqlite3_vfs_find(self.owner.name().as_ptr()) }.cast_const() == expected;
        if active == 0 || !registered {
            return Err("owner released before active statement finalized".into());
        }
        let raw = std::mem::replace(&mut statement.0, ptr::null_mut());
        let finalize_rc = unsafe { ffi::sqlite3_finalize(raw) };
        if finalize_rc != ffi::SQLITE_OK {
            return Err(format!("live statement finalize rc={finalize_rc}").into());
        }
        Ok(
            serde_json::json!({"prepare_rc":prepare_rc,"step_rc":step_rc,"close_rc":close_rc,"close_refused":true,
            "original_connection_retained":original_connection_retained,"owner_registration_retained":registered,
            "active_files_after_refusal":active,"authorizer_restored_rc":authorizer_rc,"finalize_rc":finalize_rc}),
        )
    }
    pub fn close(mut self) -> Result<()> {
        let c = self.connection.take().ok_or("already closed")?;
        match c.close() {
            Ok(()) => {
                if self.owner.ctx().close_errors.load(Ordering::SeqCst) > 0 {
                    Err("VFS close callback failed".into())
                } else {
                    Ok(())
                }
            }
            Err((c, e)) => {
                drop(c);
                Err(e.into())
            }
        }
    }
}

impl Registration {
    pub fn new(main_path: &Path, journal_path: Option<&Path>, limits: &Limits) -> Result<Self> {
        limits.validate()?;
        for suffix in ["-wal", "-shm"] {
            let mut path = main_path.as_os_str().to_os_string();
            path.push(suffix);
            match std::fs::symlink_metadata(Path::new(&path)) {
                Ok(_) => return Err(format!("D2 unsupported existing sidecar {suffix}").into()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.into()),
            }
        }
        let mut expected_journal = main_path.as_os_str().to_os_string();
        expected_journal.push("-journal");
        let expected_journal = Path::new(&expected_journal);
        let present = match std::fs::symlink_metadata(expected_journal) {
            Ok(_) => true,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
            Err(e) => return Err(e.into()),
        };
        if present != journal_path.is_some() || journal_path.is_some_and(|p| p != expected_journal)
        {
            return Err("journal source binding mismatch".into());
        }
        let main = Source::pin(main_path)?;
        let journal = journal_path.map(Source::pin).transpose()?;
        if main.identity.len > limits.max_source_main_bytes
            || journal
                .as_ref()
                .is_some_and(|j| j.identity.len > limits.max_source_journal_bytes)
        {
            return Err("source fixture limit".into());
        }
        if let Some(j) = &journal {
            if j.identity.dev == main.identity.dev && j.identity.ino == main.identity.ino {
                return Err("source alias".into());
            }
            if external_super_journal(&j.file)? {
                return Err("ExternalSuperJournalBeforeVirtualRW".into());
            }
        }
        let owner_bytes = std::mem::size_of::<Context>()
            .checked_add(std::mem::size_of::<ffi::sqlite3_vfs>())
            .ok_or("metadata owner overflow")?;
        let slot_budget = (limits.overlay_metadata_capacity_bytes as usize)
            .checked_sub(owner_bytes)
            .ok_or("metadata owner capacity")?;
        let overlay = Overlay::new(
            limits.overlay_payload_capacity_bytes as usize,
            slot_budget,
            main.identity.len,
            journal.as_ref().map(|j| j.identity.len),
        )?;
        let events = Fixed::new(limits.evidence_capacity_bytes as usize)?;
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let name = Name::make(format_args!("p1505-{}-{id}", std::process::id()))?;
        let main_name = Name::make(format_args!("/p1505/{}/{id}/main", std::process::id()))?;
        let mut ctx = FallibleBox::new(Context {
            owner_thread: unsafe { libc::pthread_self() },
            callback_active: AtomicBool::new(false),
            sqlite_worker_limit: AtomicI32::new(-1),
            registration_name: name,
            main,
            journal,
            main_name,
            wal_name: Name::make(format_args!("{}-wal", main_name.to_str()?))?,
            journal_name: Name::make(format_args!("{}-journal", main_name.to_str()?))?,
            shm_name: Name::make(format_args!("{}-shm", main_name.to_str()?))?,
            events: RefCell::new(events),
            overlay: RefCell::new(overlay),
            trace_overflow: AtomicBool::new(false),
            main_open: AtomicBool::new(false),
            journal_open: AtomicBool::new(false),
            active: AtomicUsize::new(0),
            exclusive_armed: AtomicBool::new(false),
            fault: AtomicI32::new(0),
            caught_panics: AtomicUsize::new(0),
            close_errors: AtomicUsize::new(0),
            sql_denials: AtomicUsize::new(0),
            read_bytes: AtomicU64::new(0),
            retries: AtomicUsize::new(0),
        })?;
        let mut vfs = FallibleBox::new(ffi::sqlite3_vfs {
            iVersion: 1,
            szOsFile: std::mem::size_of::<PinnedFile>() as i32,
            mxPathname: 512,
            pNext: ptr::null_mut(),
            zName: ctx.registration_name.as_ptr(),
            pAppData: (&mut *ctx as *mut Context).cast(),
            xOpen: Some(open),
            xDelete: Some(delete),
            xAccess: Some(access),
            xFullPathname: Some(fullpath),
            xDlOpen: Some(dl_open),
            xDlError: Some(dl_error),
            xDlSym: None,
            xDlClose: Some(dl_close),
            xRandomness: Some(randomness),
            xSleep: Some(sleep),
            xCurrentTime: Some(current_time),
            xGetLastError: None,
            xCurrentTimeInt64: None,
            xSetSystemCall: None,
            xGetSystemCall: None,
            xNextSystemCall: None,
        })?;
        // SAFETY: 名字、vfs、ctx 在 Box 中稳定；makeDflt=0，不改变默认 VFS。
        let rc = unsafe { ffi::sqlite3_vfs_register(&mut *vfs, 0) };
        if rc != ffi::SQLITE_OK {
            return Err(format!("register rc={rc}").into());
        }
        REGISTRATIONS.fetch_add(1, Ordering::SeqCst);
        Ok(Self {
            vfs: Some(vfs),
            ctx: Some(ctx),
            _not_send_or_sync: PhantomData,
        })
    }

    fn ctx(&self) -> &Context {
        self.ctx.as_deref().expect("live registration")
    }
    pub fn inject(&self, fault: Fault) {
        self.ctx().fault.store(fault as i32, Ordering::SeqCst);
    }
    pub fn sql_denials(&self) -> usize {
        self.ctx().sql_denials.load(Ordering::SeqCst)
    }
    pub fn name(&self) -> &CStr {
        self.ctx().registration_name.as_c_str()
    }
    pub fn main_name(&self) -> &str {
        self.ctx().main_name.to_str().expect("generated ASCII")
    }

    pub fn connect(&self, exclusive_before_read: bool) -> Result<Session<'_>> {
        let c = Connection::open_with_flags_and_vfs(
            self.main_name(),
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            self.name(),
        )?;
        c.busy_timeout(std::time::Duration::ZERO)?;
        c.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_WORKER_THREADS, 0)?;
        let worker_limit = c.limit(rusqlite::limits::Limit::SQLITE_LIMIT_WORKER_THREADS)?;
        if worker_limit != 0 {
            return Err("SQLite worker threads remain enabled".into());
        }
        self.ctx()
            .sqlite_worker_limit
            .store(worker_limit, Ordering::SeqCst);
        for (cfg, value) in [
            (DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true),
            (DbConfig::SQLITE_DBCONFIG_DEFENSIVE, true),
        ] {
            if c.set_db_config(cfg, value)? != value {
                return Err("db config did not stick".into());
            }
        }
        // rusqlite 没有公开此 DbConfig 变体；直接调用 bundled C API 并核回值。
        let mut extensions_enabled: c_int = -1;
        let rc = unsafe {
            ffi::sqlite3_db_config(
                c.handle(),
                ffi::SQLITE_DBCONFIG_ENABLE_LOAD_EXTENSION,
                0 as c_int,
                &mut extensions_enabled as *mut c_int,
            )
        };
        if rc != ffi::SQLITE_OK || extensions_enabled != 0 {
            return Err("extension loading remains enabled".into());
        }
        // SAFETY: 回调 ctx 和 Connection 同受 Registration 的借用寿命覆盖。
        let rc = unsafe {
            ffi::sqlite3_set_authorizer(
                c.handle(),
                Some(authorizer),
                (self.ctx() as *const Context).cast_mut().cast(),
            )
        };
        if rc != ffi::SQLITE_OK {
            return Err(format!("authorizer rc={rc}").into());
        }
        if exclusive_before_read {
            let mode: String = c.query_row("PRAGMA locking_mode=EXCLUSIVE", [], |r| r.get(0))?;
            let check: String = c.query_row("PRAGMA locking_mode", [], |r| r.get(0))?;
            if !mode.eq_ignore_ascii_case("exclusive") || mode != check {
                return Err("exclusive mode missing".into());
            }
            self.ctx().exclusive_armed.store(true, Ordering::SeqCst);
            if self
                .ctx()
                .record("exclusive-armed", 0, 0, 0, ffi::SQLITE_OK)
                != ffi::SQLITE_OK
            {
                return Err("trace capacity exhausted".into());
            }
        }
        c.execute_batch("PRAGMA query_only=ON; PRAGMA temp_store=MEMORY;")?;
        Ok(Session {
            connection: Some(c),
            owner: self,
        })
    }

    pub fn report(&self) -> Result<serde_json::Value> {
        let ctx = self.ctx();
        let events = ctx.events.try_borrow().map_err(|_| "trace poisoned")?;
        Ok(serde_json::json!({
            "main": ctx.main.identity, "journal": ctx.journal.as_ref().map(|w| &w.identity),
            "active_files": ctx.active.load(Ordering::SeqCst),
            "source_unchanged": ctx.main.unchanged() && ctx.journal.as_ref().is_none_or(Source::unchanged),
            "trace_overflow": ctx.trace_overflow.load(Ordering::SeqCst),
            "caught_panics": ctx.caught_panics.load(Ordering::SeqCst),
            "close_errors": ctx.close_errors.load(Ordering::SeqCst),
            "sql_denials": ctx.sql_denials.load(Ordering::SeqCst),
            "read_bytes": ctx.read_bytes.load(Ordering::SeqCst),
            "injected_interrupt_counter": ctx.retries.load(Ordering::SeqCst),
            "actual_os_eintr":"NOT_RUN",
            "sqlite_worker_threads":ctx.sqlite_worker_limit.load(Ordering::SeqCst),
            "thread_guard":"pthread owner before Context/PinnedFile references",
            "callback_reentry_rejected":true,
            "trace_capacity": events.capacity(),
            "trace_allocation": events.accounting(),
            "metadata_owner_layout_bytes":std::mem::size_of::<Context>()+std::mem::size_of::<ffi::sqlite3_vfs>(),
            "overlay":ctx.overlay.try_borrow().map_err(|_|"overlay poisoned")?.report(),
            "io_methods_version": METHODS.iVersion, "events": events.as_slice(),
        }))
    }

    pub fn finish(mut self) -> Result<serde_json::Value> {
        let report = self.report()?;
        if self.ctx().active.load(Ordering::SeqCst) != 0 {
            return Err("live files at unregister".into());
        }
        let rc = unsafe { ffi::sqlite3_vfs_unregister(self.vfs.as_deref_mut().ok_or("no vfs")?) };
        if rc != ffi::SQLITE_OK {
            return Err(format!("unregister rc={rc}").into());
        }
        let absent = unsafe { ffi::sqlite3_vfs_find(self.name().as_ptr()) }.is_null();
        if !absent {
            return Err("VFS remains registered".into());
        }
        let source_fds = [
            Some(self.ctx().main.file.as_raw_fd()),
            self.ctx().journal.as_ref().map(|w| w.file.as_raw_fd()),
        ];
        self.vfs.take(); // 已注销，不让 Drop 再触达注册链。
        self.ctx.take(); // 实际释放源 File，随后在本单线程原型中核 FD 已关闭。
        for fd in source_fds.into_iter().flatten() {
            let rc = unsafe { libc::fcntl(fd, libc::F_GETFD) };
            if rc != -1 || std::io::Error::last_os_error().raw_os_error() != Some(libc::EBADF) {
                return Err("source FD remains open after unregister".into());
            }
        }
        Ok(
            serde_json::json!({"unregistered": true, "source_fds_closed": true, "owned_arenas_and_context_freed":true, "held_capacity_after_finish":0, "report": report}),
        )
    }

    pub fn bytes(&self, kind: i32) -> Result<Vec<u8>> {
        let ctx = self.ctx();
        let o = ctx.overlay.try_borrow().map_err(|_| "overlay poisoned")?;
        let st = o.state(kind)?;
        let source = ctx.source(kind).ok_or("missing source")?;
        let size = usize::try_from(st.logical_len)?;
        let mut out = Vec::new();
        out.try_reserve_exact(size)?;
        out.resize(size, 0);
        if o.read(kind, &source.file, 0, &mut out)? {
            return Err("unexpected virtual short read".into());
        }
        Ok(out)
    }
    pub fn callback_contract(&mut self, outside_sentinel: &Path) -> Result<serde_json::Value> {
        let raw = self.vfs.as_deref_mut().ok_or("no vfs")? as *mut ffi::sqlite3_vfs;
        let name = self.ctx().main_name.clone();
        let unknown = CString::new(outside_sentinel.as_os_str().as_bytes())?;
        let mut p: PinnedFile = unsafe { std::mem::zeroed() };
        let mut flags = 0;
        let mut checks = Vec::new();
        unsafe {
            for (label, target, bits) in [
                ("null-temp", ptr::null(), ffi::SQLITE_OPEN_TEMP_DB),
                ("unknown", unknown.as_ptr(), ffi::SQLITE_OPEN_MAIN_DB),
                (
                    "create",
                    name.as_ptr(),
                    ffi::SQLITE_OPEN_MAIN_DB | ffi::SQLITE_OPEN_CREATE,
                ),
                (
                    "uri",
                    name.as_ptr(),
                    ffi::SQLITE_OPEN_MAIN_DB | ffi::SQLITE_OPEN_URI,
                ),
                (
                    "unknown-capability",
                    name.as_ptr(),
                    ffi::SQLITE_OPEN_MAIN_DB | i32::MIN,
                ),
            ] {
                let rc = open(
                    raw,
                    target,
                    &mut p.base,
                    bits | ffi::SQLITE_OPEN_READWRITE,
                    &mut flags,
                );
                if rc == ffi::SQLITE_OK || !p.base.pMethods.is_null() {
                    return Err(label.into());
                }
                checks.push(serde_json::json!({"name":label,"rc":rc}));
            }
            self.inject(Fault::OpenNoMem);
            let rc = open(
                raw,
                name.as_ptr(),
                &mut p.base,
                ffi::SQLITE_OPEN_MAIN_DB | ffi::SQLITE_OPEN_READWRITE,
                &mut flags,
            );
            if rc != ffi::SQLITE_NOMEM {
                return Err("open OOM".into());
            }
            checks.push(serde_json::json!({"name":"open-oom","rc":rc}));
            let rc = open(
                raw,
                name.as_ptr(),
                &mut p.base,
                ffi::SQLITE_OPEN_MAIN_DB | ffi::SQLITE_OPEN_READWRITE,
                &mut flags,
            );
            if rc != ffi::SQLITE_OK || flags != ffi::SQLITE_OPEN_READWRITE {
                return Err("virtual RW open".into());
            }
            let file_address = (&mut p.base as *mut ffi::sqlite3_file) as usize;
            let wrong_thread = std::thread::scope(|scope| {
                scope
                    .spawn(move || {
                        // 仅负控故意跨过 !Send；scope join 保证有效 owner/file 不被释放。
                        read(
                            file_address as *mut ffi::sqlite3_file,
                            ptr::null_mut(),
                            0,
                            0,
                        )
                    })
                    .join()
            })
            .map_err(|_| "wrong-thread probe panicked")?;
            if wrong_thread != ffi::SQLITE_MISUSE {
                return Err("wrong thread callback accepted".into());
            }
            let file_ptr = &mut p.base as *mut ffi::sqlite3_file;
            let reentry = guarded(self.ctx(), || {
                // 真实外层 guard 与活共享file借用；拒绝路径不得先造可变全对象引用。
                let borrowed = &*file_ptr.cast::<PinnedFile>();
                let old_lock = borrowed.lock;
                let old_methods = borrowed.base.pMethods;
                let codes = [
                    read(file_ptr, ptr::null_mut(), 0, 0),
                    close(file_ptr),
                    lock(file_ptr, ffi::SQLITE_LOCK_EXCLUSIVE),
                ];
                if codes != [ffi::SQLITE_MISUSE; 3]
                    || borrowed.lock != old_lock
                    || borrowed.base.pMethods != old_methods
                {
                    ffi::SQLITE_ERROR
                } else {
                    ffi::SQLITE_MISUSE
                }
            });
            if reentry != ffi::SQLITE_MISUSE {
                return Err("callback reentry accepted".into());
            }
            checks.push(serde_json::json!({"name":"thread-owner-and-reentry","wrong_thread_rc":wrong_thread,"reentry_rc":reentry,"actual_outer_guard_and_live_shared_file_borrow":true,"denied_calls":["read","close","lock"]}));
            let journal_name = self.ctx().journal_name;
            let held = self
                .ctx()
                .overlay
                .try_borrow_mut()
                .map_err(|_| "borrow-conflict setup")?;
            let mut denied_exists = 99;
            let borrow_rc = access(
                raw,
                journal_name.as_ptr(),
                ffi::SQLITE_ACCESS_EXISTS,
                &mut denied_exists,
            );
            drop(held);
            if borrow_rc != ffi::SQLITE_IOERR_ACCESS || denied_exists != 0 {
                return Err("borrow conflict became journal absence".into());
            }
            checks.push(serde_json::json!({"name":"borrow-conflict-fails-access","rc":borrow_rc}));
            self.ctx().exclusive_armed.store(true, Ordering::SeqCst);
            let mut buf = [0xA5_u8; 32];
            let eof = self.ctx().main.identity.len as i64;
            let rc = read(&mut p.base, buf.as_mut_ptr().cast(), 32, eof);
            if rc != ffi::SQLITE_IOERR_SHORT_READ || buf != [0; 32] {
                return Err("EOF zero".into());
            }
            checks.push(serde_json::json!({"name":"eof-zero","rc":rc}));
            for offset in [-1, i64::MAX] {
                let rc = read(&mut p.base, buf.as_mut_ptr().cast(), 32, offset);
                if rc != ffi::SQLITE_IOERR_READ {
                    return Err("offset".into());
                }
                checks.push(serde_json::json!({"name":"offset","offset":offset,"rc":rc}));
            }
            if read(&mut p.base, ptr::null_mut(), 0, 0) != ffi::SQLITE_OK
                || write(&mut p.base, ptr::null(), 0, 0) != ffi::SQLITE_OK
                || read(&mut p.base, ptr::null_mut(), i32::MAX, 0) != ffi::SQLITE_IOERR_READ
                || write(&mut p.base, ptr::null(), i32::MAX, i64::MAX) != ffi::SQLITE_IOERR_WRITE
            {
                return Err("zero/null/huge callback bounds".into());
            }
            checks.push(serde_json::json!({"name":"zero-null-huge-bounds","all_expected":true}));
            for (fault, expected) in [
                (Fault::ReadIo, ffi::SQLITE_IOERR_READ),
                (Fault::ReadPanic, ffi::SQLITE_IOERR),
                (Fault::ReadShort, ffi::SQLITE_IOERR_SHORT_READ),
                (Fault::ReadInterrupted, ffi::SQLITE_OK),
            ] {
                self.inject(fault);
                let rc = read(&mut p.base, buf.as_mut_ptr().cast(), 32, 0);
                if rc != expected {
                    return Err(format!("read fault {fault:?}: {rc}").into());
                }
                checks.push(serde_json::json!({"name":format!("{fault:?}"),"rc":rc}));
            }
            let mut exists = 99;
            let rc = access(
                raw,
                unknown.as_ptr(),
                ffi::SQLITE_ACCESS_EXISTS,
                &mut exists,
            );
            if rc == ffi::SQLITE_OK || exists != 0 {
                return Err("external access".into());
            }
            checks.push(serde_json::json!({"name":"external-access","rc":rc}));
            for (label, target) in [
                ("delete-main", name.as_ptr()),
                ("delete-external", unknown.as_ptr()),
            ] {
                let rc = delete(raw, target, 0);
                if rc == ffi::SQLITE_OK {
                    return Err(label.into());
                }
                checks.push(serde_json::json!({"name":label,"rc":rc}));
            }
            if control(&mut p.base, i32::MAX, ptr::null_mut()) != ffi::SQLITE_NOTFOUND {
                return Err("control".into());
            }
            let mut small = [0_i8; 4];
            if fullpath(raw, name.as_ptr(), 4, small.as_mut_ptr().cast()) == ffi::SQLITE_OK {
                return Err("small path".into());
            }
            let journal_name = self.ctx().journal_name;
            let mut jf: PinnedFile = std::mem::zeroed();
            let journal_flags = ffi::SQLITE_OPEN_MAIN_JOURNAL | ffi::SQLITE_OPEN_READWRITE;
            if open(
                raw,
                journal_name.as_ptr(),
                &mut jf.base,
                journal_flags,
                &mut flags,
            ) != ffi::SQLITE_OK
            {
                return Err("known journal callback open".into());
            }
            if delete(raw, journal_name.as_ptr(), 0) != ffi::SQLITE_OK
                || access(
                    raw,
                    journal_name.as_ptr(),
                    ffi::SQLITE_ACCESS_EXISTS,
                    &mut exists,
                ) != ffi::SQLITE_OK
                || exists != 0
                || read(&mut jf.base, buf.as_mut_ptr().cast(), 32, 0) != ffi::SQLITE_OK
            {
                return Err("delete invalidated active journal handle".into());
            }
            let mut reopened: PinnedFile = std::mem::zeroed();
            if open(
                raw,
                journal_name.as_ptr(),
                &mut reopened.base,
                journal_flags,
                &mut flags,
            ) != ffi::SQLITE_CANTOPEN
                || !reopened.base.pMethods.is_null()
                || close(&mut jf.base) != ffi::SQLITE_OK
            {
                return Err("deleted journal reopen/close contract".into());
            }
            checks.push(
                serde_json::json!({"name":"delete-live-handle-and-reopen","all_expected":true}),
            );
            self.inject(Fault::CloseIo);
            let rc = close(&mut p.base);
            if rc != ffi::SQLITE_IOERR_CLOSE || !p.base.pMethods.is_null() {
                return Err("close fault".into());
            }
            checks.push(serde_json::json!({"name":"close-fault","rc":rc}));
        }
        if self.ctx().active.load(Ordering::SeqCst) != 0 {
            return Err("manual file leak".into());
        }
        Ok(
            serde_json::json!({"checks":checks,"all_expected":true,"actual_os_eintr":"NOT_RUN","eintr_probe":"counter injection only"}),
        )
    }
}

/// 与 v4 admission.rs:1339 的冻结 footer 防护同义；不解释 journal 页或执行恢复。
pub fn external_super_journal(journal: &File) -> Result<bool> {
    let size = journal.metadata()?.len();
    if size < 16 {
        return Ok(false);
    }
    let mut first = [0_u8; 1];
    journal.read_exact_at(&mut first, 0)?;
    if first[0] == 0 {
        return Ok(false);
    }
    let mut footer = [0_u8; 16];
    journal.read_exact_at(&mut footer, size - 16)?;
    let length = u32::from_be_bytes(footer[0..4].try_into()?) as u64;
    const MAGIC: [u8; 8] = [0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7];
    if length == 0 || length >= 513 || length > size - 16 || footer[8..16] != MAGIC {
        return Ok(false);
    }
    let mut name = [0_u8; 512];
    let name = &mut name[..length as usize];
    journal.read_exact_at(name, size - 16 - length)?;
    let mut sum = u32::from_be_bytes(footer[4..8].try_into()?);
    for &b in name.iter() {
        sum = sum.wrapping_sub(b as c_char as i32 as u32);
    }
    Ok(sum == 0 && name[0] != 0)
}

impl Drop for Registration {
    fn drop(&mut self) {
        if let Some(mut vfs) = self.vfs.take() {
            // 若调用方违反生命周期而遗留句柄，宁可泄露并让进程失败，不释放活裸指针。
            let live = self
                .ctx
                .as_deref()
                .is_some_and(|c| c.active.load(Ordering::SeqCst) != 0);
            let rc = unsafe { ffi::sqlite3_vfs_unregister(&mut *vfs) };
            if live || rc != ffi::SQLITE_OK {
                std::mem::forget(vfs);
                if let Some(ctx) = self.ctx.take() {
                    std::mem::forget(ctx);
                }
            }
        }
    }
}
