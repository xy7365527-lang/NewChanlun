//! #1527：从 #1504 b9f38106 的固定 FD VFS 适配；只读 main/WAL，非默认注册。
//! 外层 WriterScope 持真实双锁；本层虚拟锁仅维护私有 SQLite 连接状态。
//! SAFETY 总契约：SQLite 给出的回调指针/缓冲区须符合其 C API；非默认注册对象
//! 地址稳定，Registration 活至所有 Connection/Statement/file 关闭之后；不导出裸 handle。
#![deny(unsafe_op_in_unsafe_fn)]

use super::IoAccounting;
use rusqlite::{config::DbConfig, ffi, Connection, OpenFlags};
use serde::Serialize;
use std::cell::RefCell;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::fs::File;
use std::marker::PhantomData;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const EXPECTED_SOURCE_ID: &str =
    "2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24";
const MAIN: i32 = 1;
const WAL: i32 = 2;
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

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
    uid: u32,
    mode: u32,
    ctime: i64,
    ctime_nsec: i64,
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
            return Err(std::io::Error::other(
                "source is not a single-link O_RDONLY regular file",
            ));
        }
        Ok(Identity {
            dev: m.dev(),
            ino: m.ino(),
            len: m.len(),
            nlink: m.nlink(),
            mtime: m.mtime(),
            mtime_nsec: m.mtime_nsec(),
            uid: m.uid(),
            mode: m.mode(),
            ctime: m.ctime(),
            ctime_nsec: m.ctime_nsec(),
            readonly_fd: true,
        })
    }

    fn pin(file: File) -> Result<Self> {
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

struct Context<'a> {
    account: RefCell<&'a mut dyn IoAccounting>,
    main: Source,
    wal: Option<Source>,
    main_name: CString,
    wal_name: CString,
    journal_name: CString,
    shm_name: CString,

    trace_overflow: AtomicBool,
    main_open: AtomicBool,
    wal_open: AtomicBool,
    active: AtomicUsize,
    exclusive_armed: AtomicBool,
    fault: AtomicI32,
    caught_panics: AtomicUsize,
    close_errors: AtomicUsize,
    sql_denials: AtomicUsize,
    read_bytes: AtomicU64,
    retries: AtomicUsize,
}

impl Context<'_> {
    fn source(&self, kind: i32) -> Option<&Source> {
        match kind {
            MAIN => Some(&self.main),
            WAL => self.wal.as_ref(),
            _ => None,
        }
    }
    fn take_fault(&self, f: Fault) -> bool {
        self.fault
            .compare_exchange(f as i32, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }
    fn record(&self, _op: &'static str, _kind: i32, _a: i64, _b: i64, rc: i32) -> i32 {
        // 不保留逐回调轨迹，不把原型 trace cap 变成生产帽。
        rc
    }
    fn before_io(&self) -> bool {
        self.account
            .try_borrow_mut()
            .is_ok_and(|mut a| a.before_io().is_ok())
    }
    fn after_read(&self, n: u64) -> bool {
        self.account
            .try_borrow_mut()
            .is_ok_and(|mut a| a.after_read(n).is_ok())
    }
}

#[repr(C)]
struct PinnedFile {
    base: ffi::sqlite3_file, // SQLite 的首字段布局；SQLite 分配 szOsFile 字节。
    ctx: *const c_void,
    kind: i32,
    lock: i32,
}

// SAFETY: 仅 SQLite 在本注册对象存活期间调用；pAppData 是稳定 Box<Context>。
unsafe fn context<'a>(vfs: *mut ffi::sqlite3_vfs) -> &'a Context<'a> {
    unsafe { &*((*vfs).pAppData.cast::<Context>()) }
}

// 每个 C ABI 的有状态路径都在此捕获 Rust unwind；OOM-abort/OS kill 不伪装成可捕获。
fn guarded(ctx: &Context, f: impl FnOnce() -> i32) -> i32 {
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
    // SAFETY: SQLite 提供对齐且可写的 szOsFile 存储。失败初始 pMethods=NULL。
    unsafe {
        (*out).pMethods = ptr::null();
    }
    if !out_flags.is_null() {
        unsafe {
            *out_flags = 0;
        }
    }
    let ctx = unsafe { context(vfs) };
    guarded(ctx, || {
        let name = unsafe { bytes(name) };
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
            && flags & (ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_CREATE) == 0
        {
            MAIN
        } else if name == Some(ctx.wal_name.as_bytes())
            && flags & types == ffi::SQLITE_OPEN_WAL
            && ctx.wal.is_some()
            && ctx.exclusive_armed.load(Ordering::SeqCst)
            && ctx.main_open.load(Ordering::SeqCst)
        {
            WAL
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
            &ctx.wal_open
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
                    ctx: (ctx as *const Context).cast(),
                    kind,
                    lock: ffi::SQLITE_LOCK_NONE,
                },
            );
        }
        ctx.active.fetch_add(1, Ordering::SeqCst);
        if !out_flags.is_null() {
            unsafe {
                *out_flags = ffi::SQLITE_OPEN_READONLY;
            }
        }
        // 已安装有效对象后若轨迹满，SQLite 仍可用 xClose 正常释放。
        ctx.record(
            "open",
            kind,
            flags.into(),
            ffi::SQLITE_OPEN_READONLY.into(),
            ffi::SQLITE_OK,
        )
    })
}

unsafe extern "C" fn close(file: *mut ffi::sqlite3_file) -> c_int {
    let f = unsafe { &mut *file.cast::<PinnedFile>() };
    let ctx = unsafe { &*f.ctx.cast::<Context>() };
    guarded(ctx, || {
        if f.base.pMethods.is_null() {
            return ffi::SQLITE_IOERR_CLOSE;
        }
        f.base.pMethods = ptr::null();
        if f.kind == MAIN {
            ctx.main_open.store(false, Ordering::SeqCst);
        } else {
            ctx.wal_open.store(false, Ordering::SeqCst);
        }
        ctx.active.fetch_sub(1, Ordering::SeqCst);
        let rc = if ctx.take_fault(Fault::CloseIo) {
            ctx.close_errors.fetch_add(1, Ordering::SeqCst);
            ffi::SQLITE_IOERR_CLOSE
        } else {
            ffi::SQLITE_OK
        };
        ctx.record("close", f.kind, 0, 0, rc)
    })
}

unsafe extern "C" fn read(
    file: *mut ffi::sqlite3_file,
    out: *mut c_void,
    amount: c_int,
    offset: i64,
) -> c_int {
    let f = unsafe { &*file.cast::<PinnedFile>() };
    let ctx = unsafe { &*f.ctx.cast::<Context>() };
    let rc = guarded(ctx, || {
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
            panic!("1504 controlled callback panic");
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
        let mut done = 0;
        while done < buf.len() {
            if !ctx.before_io() {
                return ffi::SQLITE_IOERR_READ;
            }
            let part = if ctx.take_fault(Fault::ReadInterrupted) {
                Err(std::io::Error::from(std::io::ErrorKind::Interrupted))
            } else {
                source
                    .file
                    .read_at(&mut buf[done..], offset as u64 + done as u64)
            };
            match part {
                Ok(0) => {
                    if !ctx.after_read(0) {
                        return ffi::SQLITE_IOERR_READ;
                    }
                    break;
                }
                Ok(n) => {
                    done += n;
                    ctx.read_bytes.fetch_add(n as u64, Ordering::SeqCst);
                    if !ctx.after_read(n as u64) {
                        return ffi::SQLITE_IOERR_READ;
                    }
                }
                Err(_) => {
                    let _ = ctx.after_read(0);
                    return ffi::SQLITE_IOERR_READ;
                }
            }
        }
        if !source.unchanged() {
            return ffi::SQLITE_IOERR_FSTAT;
        }
        if done < buf.len() {
            ffi::SQLITE_IOERR_SHORT_READ
        } else {
            ffi::SQLITE_OK
        }
    });
    guarded(ctx, || {
        ctx.record("read", f.kind, offset, amount.into(), rc)
    })
}

unsafe fn file_event(file: *mut ffi::sqlite3_file, op: &'static str, a: i64, rc: i32) -> i32 {
    let f = unsafe { &*file.cast::<PinnedFile>() };
    let ctx = unsafe { &*f.ctx.cast::<Context>() };
    guarded(ctx, || ctx.record(op, f.kind, a, 0, rc))
}

unsafe extern "C" fn write(
    file: *mut ffi::sqlite3_file,
    _: *const c_void,
    n: c_int,
    _: i64,
) -> c_int {
    unsafe { file_event(file, "write-denied", n.into(), ffi::SQLITE_READONLY) }
}
unsafe extern "C" fn truncate(file: *mut ffi::sqlite3_file, n: i64) -> c_int {
    unsafe { file_event(file, "truncate-denied", n, ffi::SQLITE_READONLY) }
}
unsafe extern "C" fn sync(file: *mut ffi::sqlite3_file, _: c_int) -> c_int {
    unsafe { file_event(file, "sync-readonly-noop", 0, ffi::SQLITE_OK) }
}
unsafe extern "C" fn size(file: *mut ffi::sqlite3_file, out: *mut i64) -> c_int {
    let f = unsafe { &*file.cast::<PinnedFile>() };
    let ctx = unsafe { &*f.ctx.cast::<Context>() };
    guarded(ctx, || {
        if out.is_null() {
            return ffi::SQLITE_IOERR_FSTAT;
        }
        let Some(source) = ctx.source(f.kind) else {
            return ffi::SQLITE_IOERR_FSTAT;
        };
        if !source.unchanged() {
            return ffi::SQLITE_IOERR_FSTAT;
        }
        unsafe {
            *out = source.identity.len as i64;
        }
        ctx.record(
            "size",
            f.kind,
            source.identity.len as i64,
            0,
            ffi::SQLITE_OK,
        )
    })
}
unsafe extern "C" fn lock(file: *mut ffi::sqlite3_file, level: c_int) -> c_int {
    let f = unsafe { &mut *file.cast::<PinnedFile>() };
    let ctx = unsafe { &*f.ctx.cast::<Context>() };
    guarded(ctx, || {
        if !(ffi::SQLITE_LOCK_SHARED..=ffi::SQLITE_LOCK_EXCLUSIVE).contains(&level) {
            return ffi::SQLITE_IOERR_LOCK;
        }
        f.lock = f.lock.max(level);
        ctx.record("lock-virtual", f.kind, level.into(), 0, ffi::SQLITE_OK)
    })
}
unsafe extern "C" fn unlock(file: *mut ffi::sqlite3_file, level: c_int) -> c_int {
    let f = unsafe { &mut *file.cast::<PinnedFile>() };
    let ctx = unsafe { &*f.ctx.cast::<Context>() };
    guarded(ctx, || {
        if ![ffi::SQLITE_LOCK_NONE, ffi::SQLITE_LOCK_SHARED].contains(&level) || level > f.lock {
            return ffi::SQLITE_IOERR_UNLOCK;
        }
        f.lock = level;
        ctx.record("unlock-virtual", f.kind, level.into(), 0, ffi::SQLITE_OK)
    })
}
unsafe extern "C" fn reserved(file: *mut ffi::sqlite3_file, out: *mut c_int) -> c_int {
    let f = unsafe { &*file.cast::<PinnedFile>() };
    let ctx = unsafe { &*f.ctx.cast::<Context>() };
    guarded(ctx, || {
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
    let f = unsafe { &*file.cast::<PinnedFile>() };
    let ctx = unsafe { &*f.ctx.cast::<Context>() };
    guarded(ctx, || {
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
    let ctx = unsafe { context(vfs) };
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
            ctx.wal.is_some()
        } else if name == Some(ctx.journal_name.as_bytes()) || name == Some(ctx.shm_name.as_bytes())
        {
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
            *out = i32::from(exists && flags != ffi::SQLITE_ACCESS_READWRITE);
        }
        ctx.record("access", 0, flags.into(), i64::from(exists), ffi::SQLITE_OK)
    })
}
unsafe extern "C" fn delete(vfs: *mut ffi::sqlite3_vfs, _: *const c_char, _: c_int) -> c_int {
    let ctx = unsafe { context(vfs) };
    guarded(ctx, || {
        ctx.record("delete-denied", 0, 0, 0, ffi::SQLITE_READONLY)
    })
}
unsafe extern "C" fn fullpath(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    n: c_int,
    out: *mut c_char,
) -> c_int {
    let ctx = unsafe { context(vfs) };
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
    let ctx = unsafe { context(vfs) };
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

// #1527：只列原完整审计实际使用的只读动作；无 DDL/DML/ATTACH/SAVEPOINT/未知函数。
fn audit_sql_allowed(
    action: c_int,
    a: Option<&[u8]>,
    b: Option<&[u8]>,
    db: Option<&[u8]>,
    origin: Option<&[u8]>,
    exclusive_armed: bool,
) -> bool {
    if origin.is_some() || !matches!(db, None | Some(b"main")) {
        return false;
    }
    match action {
        ffi::SQLITE_SELECT => true,
        ffi::SQLITE_READ => db == Some(b"main") || a == Some(b"pragma_table_info"),
        ffi::SQLITE_TRANSACTION => matches!(a, Some(b"BEGIN" | b"ROLLBACK")),
        ffi::SQLITE_FUNCTION => matches!(
            b,
            Some(
                b"count"
                    | b"length"
                    | b"hex"
                    | b"typeof"
                    | b"like"
                    | b"coalesce"
                    | b"max"
                    | b"min"
                    | b"sum"
            )
        ),
        ffi::SQLITE_PRAGMA => match (a, b) {
            (Some(b"locking_mode"), Some(value)) => {
                !exclusive_armed && value.eq_ignore_ascii_case(b"exclusive")
            }
            (Some(b"locking_mode"), None) => true,
            (Some(b"query_only"), Some(b"ON" | b"1")) => true,
            (Some(b"temp_store"), Some(value)) => value.eq_ignore_ascii_case(b"memory"),
            (Some(b"quick_check"), None) => true,
            // table_info 的参数是表名，始终只读；pragma_table_info(?1) 原 schema 审计使用。
            (Some(b"table_info"), Some(_)) => true,
            _ => false,
        },
        _ => false,
    }
}
unsafe extern "C" fn authorizer(
    data: *mut c_void,
    action: c_int,
    a: *const c_char,
    b: *const c_char,
    db: *const c_char,
    origin: *const c_char,
) -> c_int {
    let ctx = unsafe { &*data.cast::<Context>() };
    let rc = guarded(ctx, || {
        let allowed = audit_sql_allowed(
            action,
            unsafe { bytes(a) },
            unsafe { bytes(b) },
            unsafe { bytes(db) },
            unsafe { bytes(origin) },
            ctx.exclusive_armed.load(Ordering::SeqCst),
        );
        if allowed {
            ffi::SQLITE_OK
        } else {
            ctx.sql_denials.fetch_add(1, Ordering::SeqCst);
            ffi::SQLITE_DENY
        }
    });
    if rc == ffi::SQLITE_OK {
        rc
    } else {
        ffi::SQLITE_DENY
    }
}

// SQLite 缓存命中没有 VFS read；逐 VM 指令仍核同一个原期限。
// 不运行 SQL、不保留 RefCell 借用，重入或 panic 一律 interrupt。
unsafe extern "C" fn progress(data: *mut c_void) -> c_int {
    let ctx = unsafe { &*data.cast::<Context>() };
    let rc = guarded(ctx, || {
        if ctx.before_io() {
            ffi::SQLITE_OK
        } else {
            ffi::SQLITE_INTERRUPT
        }
    });
    c_int::from(rc != ffi::SQLITE_OK)
}

#[cfg(test)]
mod authorization_tests {
    use super::*;
    // NOT_RUN：仅策略源码锁，真实 SQLite authorizer 事件序列仍需具名运行。
    #[test]
    fn full_audit_read_actions_do_not_enable_write_attach_temp_or_unknown_exits() {
        for action in [
            ffi::SQLITE_INSERT,
            ffi::SQLITE_UPDATE,
            ffi::SQLITE_DELETE,
            ffi::SQLITE_ATTACH,
            ffi::SQLITE_DETACH,
            ffi::SQLITE_CREATE_TABLE,
            ffi::SQLITE_CREATE_TEMP_TABLE,
            ffi::SQLITE_SAVEPOINT,
            -1,
        ] {
            assert!(!audit_sql_allowed(
                action,
                Some(b"meta"),
                None,
                Some(b"main"),
                None,
                true
            ));
        }
        assert!(!audit_sql_allowed(
            ffi::SQLITE_READ,
            Some(b"meta"),
            None,
            Some(b"temp"),
            None,
            true
        ));
        assert!(!audit_sql_allowed(
            ffi::SQLITE_FUNCTION,
            None,
            Some(b"load_extension"),
            None,
            None,
            true
        ));
        assert!(!audit_sql_allowed(
            ffi::SQLITE_PRAGMA,
            Some(b"writable_schema"),
            Some(b"1"),
            None,
            None,
            true
        ));
        assert!(!audit_sql_allowed(
            ffi::SQLITE_PRAGMA,
            Some(b"query_only"),
            Some(b"OFF"),
            None,
            None,
            true
        ));
        assert!(!audit_sql_allowed(
            ffi::SQLITE_READ,
            Some(b"meta"),
            None,
            Some(b"main"),
            Some(b"view"),
            true
        ));
        for function in [
            b"count".as_slice(),
            b"coalesce",
            b"sum",
            b"max",
            b"min",
            b"length",
            b"typeof",
            b"like",
            b"hex",
        ] {
            assert!(audit_sql_allowed(
                ffi::SQLITE_FUNCTION,
                None,
                Some(function),
                None,
                None,
                true
            ));
        }
        assert!(audit_sql_allowed(
            ffi::SQLITE_READ,
            Some(b"pragma_table_info"),
            Some(b"type"),
            None,
            None,
            true
        ));
        assert!(audit_sql_allowed(
            ffi::SQLITE_PRAGMA,
            Some(b"table_info"),
            Some(b"meta"),
            Some(b"main"),
            None,
            true
        ));
        assert!(audit_sql_allowed(
            ffi::SQLITE_TRANSACTION,
            Some(b"ROLLBACK"),
            None,
            None,
            None,
            true
        ));
    }
}

pub(super) struct Registration<'a> {
    vfs: Option<Box<ffi::sqlite3_vfs>>,
    ctx: Option<Box<Context<'a>>>,
    name: Option<CString>,
    _not_send_or_sync: PhantomData<Rc<()>>,
}

pub(super) struct Session<'r, 'a> {
    connection: Option<Connection>,
    owner: &'r Registration<'a>,
}

impl Session<'_, '_> {
    pub fn connection(&self) -> &Connection {
        self.connection.as_ref().expect("live session")
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

impl<'a> Registration<'a> {
    pub fn new(
        main_file: File,
        wal_file: Option<File>,
        account: &'a mut dyn IoAccounting,
    ) -> Result<Self> {
        account.before_io()?;
        // D1 机制与固定 bundled SQLite source id 绑定，不换成宿主 SQLite 重试。
        let source_id = unsafe { CStr::from_ptr(ffi::sqlite3_sourceid()) };
        if source_id.to_bytes() != EXPECTED_SOURCE_ID.as_bytes() {
            return Err("PrivateSqliteSourceIdMismatch".into());
        }
        let main = Source::pin(main_file)?;
        let wal = wal_file.map(Source::pin).transpose()?;
        if wal.as_ref().is_some_and(|w| {
            w.identity.dev == main.identity.dev && w.identity.ino == main.identity.ino
        }) {
            return Err("main and WAL alias".into());
        }
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let name = CString::new(format!("s1527-d1-{}-{id}", std::process::id()))?;
        let main_name = format!("/s1527/d1/{}/{id}/main", std::process::id());
        let mut ctx = Box::new(Context {
            main,
            wal,
            main_name: CString::new(main_name.clone())?,
            wal_name: CString::new(format!("{main_name}-wal"))?,
            journal_name: CString::new(format!("{main_name}-journal"))?,
            shm_name: CString::new(format!("{main_name}-shm"))?,
            account: RefCell::new(account),
            trace_overflow: AtomicBool::new(false),
            main_open: AtomicBool::new(false),
            wal_open: AtomicBool::new(false),
            active: AtomicUsize::new(0),
            exclusive_armed: AtomicBool::new(false),
            fault: AtomicI32::new(0),
            caught_panics: AtomicUsize::new(0),
            close_errors: AtomicUsize::new(0),
            sql_denials: AtomicUsize::new(0),
            read_bytes: AtomicU64::new(0),
            retries: AtomicUsize::new(0),
        });
        let mut vfs = Box::new(ffi::sqlite3_vfs {
            iVersion: 1,
            szOsFile: std::mem::size_of::<PinnedFile>() as i32,
            mxPathname: 256,
            pNext: ptr::null_mut(),
            zName: name.as_ptr(),
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
        });
        // SAFETY: 名字、vfs、ctx 在 Box 中稳定；makeDflt=0，不改变默认 VFS。
        let rc = unsafe { ffi::sqlite3_vfs_register(&mut *vfs, 0) };
        if rc != ffi::SQLITE_OK {
            return Err(format!("register rc={rc}").into());
        }
        Ok(Self {
            vfs: Some(vfs),
            ctx: Some(ctx),
            name: Some(name),
            _not_send_or_sync: PhantomData,
        })
    }

    fn ctx(&self) -> &Context<'a> {
        self.ctx.as_deref().expect("live registration")
    }
    pub fn inject(&self, fault: Fault) {
        self.ctx().fault.store(fault as i32, Ordering::SeqCst);
    }
    pub fn sql_denials(&self) -> usize {
        self.ctx().sql_denials.load(Ordering::SeqCst)
    }
    pub fn name(&self) -> &CStr {
        self.name.as_deref().expect("live registration")
    }
    pub fn main_name(&self) -> &str {
        self.ctx().main_name.to_str().expect("generated ASCII")
    }

    pub fn connect(&self, exclusive_before_read: bool) -> Result<Session<'_, 'a>> {
        let c = Connection::open_with_flags_and_vfs(
            self.main_name(),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            self.name(),
        )?;
        c.busy_timeout(std::time::Duration::ZERO)?;
        c.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_WORKER_THREADS, 0)?;
        if c.limit(rusqlite::limits::Limit::SQLITE_LIMIT_WORKER_THREADS)? != 0 {
            return Err("PrivateSqliteWorkersEnabled".into());
        }
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
        // SAFETY：同 authorizer 的稳定 ctx，Session/connection 必须先于 Registration 析构。
        unsafe {
            ffi::sqlite3_progress_handler(
                c.handle(),
                1,
                Some(progress),
                (self.ctx() as *const Context).cast_mut().cast(),
            );
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

    pub fn finish(mut self) -> Result<()> {
        if self.ctx().active.load(Ordering::SeqCst) != 0
            || self.ctx().caught_panics.load(Ordering::SeqCst) != 0
            || self.ctx().close_errors.load(Ordering::SeqCst) != 0
            || !self.ctx().main.unchanged()
            || !self.ctx().wal.as_ref().is_none_or(Source::unchanged)
        {
            return Err("PrivateVfsCleanupOrSourceChanged".into());
        }
        let rc = unsafe { ffi::sqlite3_vfs_unregister(self.vfs.as_deref_mut().ok_or("no vfs")?) };
        if rc != ffi::SQLITE_OK {
            return Err("PrivateVfsUnregister".into());
        }
        self.vfs.take();
        self.ctx.take();
        Ok(())
    }
}
impl Drop for Registration<'_> {
    fn drop(&mut self) {
        if let Some(mut vfs) = self.vfs.take() {
            // 若内部生命周期被破坏，不能留下指向已离栈记账门面的活 C 回调；立即终止。
            let live = self
                .ctx
                .as_deref()
                .is_some_and(|c| c.active.load(Ordering::SeqCst) != 0);
            let rc = unsafe { ffi::sqlite3_vfs_unregister(&mut *vfs) };
            if live || rc != ffi::SQLITE_OK {
                std::process::abort();
            }
        }
    }
}
