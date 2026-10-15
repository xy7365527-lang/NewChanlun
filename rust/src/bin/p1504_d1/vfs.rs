//! #1504 独立原型。固定 O_RDONLY main/WAL；没有宿主 pathname 回调出口。
//! 私有虚拟锁只在本连接内有效，外部排他性来自隔离夹具假设，不是本 VFS 的保证。
//! SAFETY 总契约：SQLite 给出的回调指针/缓冲区须符合其 C API；非默认注册对象
//! 地址稳定，Registration 活至所有 Connection/Statement/file 关闭之后；不导出裸 handle。
#![deny(unsafe_op_in_unsafe_fn)]

use rusqlite::{config::DbConfig, ffi, Connection, OpenFlags};
use serde::Serialize;
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
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub const EXPECTED_SOURCE_ID: &str =
    "2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24";
const TRACE_CAPACITY: usize = 32_768; // 具名小夹具的轨迹槽，不是全进程 heap 配额。
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

#[derive(Clone, Serialize)]
struct Event {
    op: &'static str,
    kind: i32,
    a: i64,
    b: i64,
    rc: i32,
    exclusive_armed: bool,
}

struct Context {
    main: Source,
    wal: Option<Source>,
    main_name: CString,
    wal_name: CString,
    journal_name: CString,
    shm_name: CString,
    events: Mutex<Vec<Event>>,
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

impl Context {
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
    fn record(&self, op: &'static str, kind: i32, a: i64, b: i64, rc: i32) -> i32 {
        let Ok(mut events) = self.events.lock() else {
            return ffi::SQLITE_IOERR;
        };
        if events.len() == events.capacity() {
            self.trace_overflow.store(true, Ordering::SeqCst);
            return ffi::SQLITE_IOERR_NOMEM;
        }
        events.push(Event {
            op,
            kind,
            a,
            b,
            rc,
            exclusive_armed: self.exclusive_armed.load(Ordering::SeqCst),
        });
        rc
    }
}

#[repr(C)]
struct PinnedFile {
    base: ffi::sqlite3_file, // SQLite 的首字段布局；SQLite 分配 szOsFile 字节。
    ctx: *const Context,
    kind: i32,
    lock: i32,
}

// SAFETY: 仅 SQLite 在本注册对象存活期间调用；pAppData 是稳定 Box<Context>。
unsafe fn context<'a>(vfs: *mut ffi::sqlite3_vfs) -> &'a Context {
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
                    ctx,
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
    let ctx = unsafe { &*f.ctx };
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
    let ctx = unsafe { &*f.ctx };
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
        let mut interrupted = 0;
        while done < buf.len() {
            let part = if ctx.take_fault(Fault::ReadInterrupted) {
                Err(std::io::Error::from(std::io::ErrorKind::Interrupted))
            } else {
                source
                    .file
                    .read_at(&mut buf[done..], offset as u64 + done as u64)
            };
            match part {
                Ok(0) => break,
                Ok(n) => {
                    done += n;
                    ctx.read_bytes.fetch_add(n as u64, Ordering::SeqCst);
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted && interrupted < 3 => {
                    interrupted += 1;
                    ctx.retries.fetch_add(1, Ordering::SeqCst);
                }
                Err(_) => return ffi::SQLITE_IOERR_READ,
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
    let ctx = unsafe { &*f.ctx };
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
    let ctx = unsafe { &*f.ctx };
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
    let ctx = unsafe { &*f.ctx };
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
    let ctx = unsafe { &*f.ctx };
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
    let ctx = unsafe { &*f.ctx };
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
    let ctx = unsafe { &*f.ctx };
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

unsafe extern "C" fn authorizer(
    data: *mut c_void,
    action: c_int,
    a: *const c_char,
    b: *const c_char,
    _: *const c_char,
    _: *const c_char,
) -> c_int {
    let ctx = unsafe { &*data.cast::<Context>() };
    // 不支持的动作一律 DENY；panic 返回 IOERR 并不是合法 authorizer 返回值，转 DENY。
    let rc = guarded(ctx, || {
        let allowed = match action {
            ffi::SQLITE_SELECT | ffi::SQLITE_READ => true,
            ffi::SQLITE_FUNCTION => {
                matches!(unsafe { bytes(b) }, Some(b"count" | b"length" | b"hex"))
            }
            ffi::SQLITE_PRAGMA => match (unsafe { bytes(a) }, unsafe { bytes(b) }) {
                (Some(b"locking_mode"), Some(value)) => {
                    !ctx.exclusive_armed.load(Ordering::SeqCst)
                        && value.eq_ignore_ascii_case(b"exclusive")
                }
                (Some(b"locking_mode"), None) => true,
                (Some(b"query_only"), Some(value)) => value == b"ON" || value == b"1",
                (Some(b"temp_store"), Some(value)) => value.eq_ignore_ascii_case(b"memory"),
                (Some(b"quick_check"), None) => true,
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
    vfs: Option<Box<ffi::sqlite3_vfs>>,
    ctx: Option<Box<Context>>,
    name: Option<CString>,
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
    pub fn new(main_path: &Path, wal_path: Option<&Path>) -> Result<Self> {
        // 原型输入域只接无 journal/SHM 的已冻结夹具；不把已有 journal 藏为不存在。
        // 这次捕获前的 pathname 检查依赖隔离目录，绝不宣称抵抗同 UID 并发替换。
        for suffix in ["-journal", "-shm"] {
            let mut sidecar = main_path.as_os_str().to_os_string();
            sidecar.push(suffix);
            match std::fs::symlink_metadata(Path::new(&sidecar)) {
                Ok(_) => return Err(format!("D1 unsupported existing sidecar {suffix}").into()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(e.into()),
            }
        }
        let main = Source::pin(main_path)?;
        let wal = wal_path.map(Source::pin).transpose()?;
        if wal.as_ref().is_some_and(|w| {
            w.identity.dev == main.identity.dev && w.identity.ino == main.identity.ino
        }) {
            return Err("main and WAL alias".into());
        }
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let name = CString::new(format!("p1504-{}-{id}", std::process::id()))?;
        let main_name = format!("/p1504/{}/{id}/main", std::process::id());
        let mut events = Vec::new();
        events.try_reserve_exact(TRACE_CAPACITY)?;
        let mut ctx = Box::new(Context {
            main,
            wal,
            main_name: CString::new(main_name.clone())?,
            wal_name: CString::new(format!("{main_name}-wal"))?,
            journal_name: CString::new(format!("{main_name}-journal"))?,
            shm_name: CString::new(format!("{main_name}-shm"))?,
            events: Mutex::new(events),
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
        self.name.as_deref().expect("live registration")
    }
    pub fn main_name(&self) -> &str {
        self.ctx().main_name.to_str().expect("generated ASCII")
    }

    pub fn connect(&self, exclusive_before_read: bool) -> Result<Session<'_>> {
        let c = Connection::open_with_flags_and_vfs(
            self.main_name(),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            self.name(),
        )?;
        c.busy_timeout(std::time::Duration::ZERO)?;
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
        let events = ctx.events.lock().map_err(|_| "trace poisoned")?;
        Ok(serde_json::json!({
            "main": ctx.main.identity, "wal": ctx.wal.as_ref().map(|w| &w.identity),
            "active_files": ctx.active.load(Ordering::SeqCst),
            "source_unchanged": ctx.main.unchanged() && ctx.wal.as_ref().is_none_or(Source::unchanged),
            "trace_overflow": ctx.trace_overflow.load(Ordering::SeqCst),
            "caught_panics": ctx.caught_panics.load(Ordering::SeqCst),
            "close_errors": ctx.close_errors.load(Ordering::SeqCst),
            "sql_denials": ctx.sql_denials.load(Ordering::SeqCst),
            "read_bytes": ctx.read_bytes.load(Ordering::SeqCst),
            "eintr_retries": ctx.retries.load(Ordering::SeqCst),
            "trace_capacity": events.capacity(),
            "trace_reserved_bytes": events.capacity() * std::mem::size_of::<Event>(),
            "io_methods_version": METHODS.iVersion, "events": &*events,
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
            self.ctx().wal.as_ref().map(|w| w.file.as_raw_fd()),
        ];
        self.vfs.take(); // 已注销，不让 Drop 再触达注册链。
        self.ctx.take(); // 实际释放源 File，随后在本单线程原型中核 FD 已关闭。
        for fd in source_fds.into_iter().flatten() {
            let rc = unsafe { libc::fcntl(fd, libc::F_GETFD) };
            if rc != -1 || std::io::Error::last_os_error().raw_os_error() != Some(libc::EBADF) {
                return Err("source FD remains open after unregister".into());
            }
        }
        Ok(serde_json::json!({"unregistered": true, "source_fds_closed": true, "report": report}))
    }

    /// 独立 callback 合同负控，不通过私有裸 FD 人工 early-close 制造 UB。
    pub fn callback_contract(&mut self, outside_sentinel: &Path) -> Result<serde_json::Value> {
        let raw = self.vfs.as_deref_mut().ok_or("no vfs")? as *mut ffi::sqlite3_vfs;
        let name = self.ctx().main_name.clone();
        let unknown = CString::new(outside_sentinel.as_os_str().as_bytes())?;
        let uri = CString::new(format!("file:{}?immutable=1", self.main_name()))?;
        let mut p: PinnedFile = unsafe { std::mem::zeroed() };
        let mut out_flags = 0;
        let mut checks = Vec::new();
        // SAFETY: 下列调用全部使用真实 Registry 和本地完整、对齐、足长的缓冲区。
        unsafe {
            let pf = &mut p.base as *mut ffi::sqlite3_file;
            for (label, target, flags) in [
                ("null-temp", ptr::null(), ffi::SQLITE_OPEN_TEMP_DB),
                ("unknown", unknown.as_ptr(), ffi::SQLITE_OPEN_MAIN_DB),
                (
                    "uri",
                    uri.as_ptr(),
                    ffi::SQLITE_OPEN_MAIN_DB | ffi::SQLITE_OPEN_URI,
                ),
            ] {
                let rc = open(
                    raw,
                    target,
                    pf,
                    flags | ffi::SQLITE_OPEN_READONLY,
                    &mut out_flags,
                );
                if rc == ffi::SQLITE_OK || !(*pf).pMethods.is_null() {
                    return Err(label.into());
                }
                checks.push(serde_json::json!({"name": label, "rc": rc}));
            }
            self.inject(Fault::OpenNoMem);
            let rc = open(
                raw,
                name.as_ptr(),
                pf,
                ffi::SQLITE_OPEN_MAIN_DB | ffi::SQLITE_OPEN_READONLY,
                &mut out_flags,
            );
            if rc != ffi::SQLITE_NOMEM || !(*pf).pMethods.is_null() {
                return Err("open oom contract".into());
            }
            checks.push(serde_json::json!({"name":"open-oom", "rc":rc}));
            let rc = open(
                raw,
                name.as_ptr(),
                pf,
                ffi::SQLITE_OPEN_MAIN_DB | ffi::SQLITE_OPEN_READONLY,
                &mut out_flags,
            );
            if rc != ffi::SQLITE_OK || out_flags != ffi::SQLITE_OPEN_READONLY {
                return Err("manual open".into());
            }
            let mut buf = [0xA5_u8; 32];
            let eof = self.ctx().main.identity.len as i64;
            let short = read(pf, buf.as_mut_ptr().cast(), 32, eof);
            if short != ffi::SQLITE_IOERR_SHORT_READ || buf != [0; 32] {
                return Err("short read zero-fill".into());
            }
            checks.push(serde_json::json!({"name":"short-read-zero-fill", "rc":short}));
            for offset in [-1, i64::MAX] {
                let rc = read(pf, buf.as_mut_ptr().cast(), 32, offset);
                if rc != ffi::SQLITE_IOERR_READ {
                    return Err("offset bounds".into());
                }
                checks.push(serde_json::json!({"name":"offset-bounds", "offset":offset,"rc":rc}));
            }
            for (fault, expected) in [
                (Fault::ReadIo, ffi::SQLITE_IOERR_READ),
                (Fault::ReadPanic, ffi::SQLITE_IOERR),
                (Fault::ReadShort, ffi::SQLITE_IOERR_SHORT_READ),
                (Fault::ReadInterrupted, ffi::SQLITE_OK),
            ] {
                self.inject(fault);
                let rc = read(pf, buf.as_mut_ptr().cast(), 32, 0);
                if rc != expected {
                    return Err(format!("fault {fault:?}: {rc}").into());
                }
                checks.push(serde_json::json!({"name":format!("{fault:?}"),"rc":rc}));
            }
            for (label, rc) in [
                ("write", write(pf, buf.as_ptr().cast(), 32, 0)),
                ("truncate", truncate(pf, 0)),
                ("delete-source", delete(raw, name.as_ptr(), 0)),
                ("delete-outside", delete(raw, unknown.as_ptr(), 0)),
            ] {
                if rc != ffi::SQLITE_READONLY {
                    return Err(label.into());
                }
                checks.push(serde_json::json!({"name":label,"rc":rc}));
            }
            let mut exists = 99;
            let rc = access(
                raw,
                unknown.as_ptr(),
                ffi::SQLITE_ACCESS_EXISTS,
                &mut exists,
            );
            if rc == ffi::SQLITE_OK || exists != 0 {
                return Err("unknown access".into());
            }
            checks.push(serde_json::json!({"name":"access-outside","rc":rc}));
            let mut path_buf = [0 as c_char; 4];
            let rc = fullpath(raw, name.as_ptr(), 4, path_buf.as_mut_ptr());
            if rc == ffi::SQLITE_OK {
                return Err("fullpath short buffer".into());
            }
            checks.push(serde_json::json!({"name":"fullpath-short","rc":rc}));
            let rc = control(pf, i32::MAX, ptr::null_mut());
            if rc != ffi::SQLITE_NOTFOUND {
                return Err("unknown file control".into());
            }
            checks.push(serde_json::json!({"name":"unknown-control","rc":rc}));
            self.inject(Fault::CloseIo);
            let rc = close(pf);
            if rc != ffi::SQLITE_IOERR_CLOSE || !(*pf).pMethods.is_null() {
                return Err("close fault".into());
            }
            checks.push(serde_json::json!({"name":"close-failure-release","rc":rc}));
        }
        if self.ctx().active.load(Ordering::SeqCst) != 0 {
            return Err("manual file leak".into());
        }
        Ok(serde_json::json!({"checks":checks,"all_expected":true}))
    }
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
                if let Some(name) = self.name.take() {
                    std::mem::forget(name);
                }
            }
        }
    }
}
