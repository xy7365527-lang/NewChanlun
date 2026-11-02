//! #1527 32D：受限 N1 的薄 unix VFS 代理。只在显式 open_native 使用。
use super::*;
use rusqlite::ffi;
use std::os::fd::AsRawFd;

pub(super) struct Registration {
    vfs: Box<ffi::sqlite3_vfs>,
    name: std::ffi::CString,
    context: Box<Context>,
    registered: bool,
}

struct Context {
    parent: *mut ffi::sqlite3_vfs,
    hooks: &'static NativeHooks,
    token: u64,
    pin: std::fs::File,
}

impl Context {
    fn gate(&self) -> Result<(), NativeError> {
        callback_gate(self.hooks, self.token)
    }
    fn measured(&self) -> Result<u64, ()> {
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { (self.hooks.orig.fstat)(self.pin.as_raw_fd(), &mut st) } != 0 {
            return Err(());
        }
        let expected = self
            .hooks
            .lock()
            .active_for(self.token)
            .map_err(|_| ())?
            .pins[Class::Shm as usize];
        if ident(&st) != expected {
            return Err(());
        }
        Ok((st.st_blocks.max(0) as u64).saturating_mul(512))
    }
    fn fail(&self, error: NativeError) {
        let _ = with_owner(|tls| tls.fail(error));
    }
}

impl Registration {
    pub(super) fn new(owner: &NativeOwner, pin: std::fs::File) -> Result<Self, NativeError> {
        if !HOOKS
            .get()
            .is_some_and(|hooks| std::ptr::eq(hooks, owner.hooks))
        {
            return Err(NativeCode::Unavailable.into());
        }
        let parent = unsafe { ffi::sqlite3_vfs_find(std::ptr::null()) };
        if parent.is_null() {
            return Err(NativeCode::Unavailable.into());
        }
        let mut context = Box::new(Context {
            parent,
            hooks: owner.hooks,
            token: owner.token,
            pin,
        });
        context
            .measured()
            .map_err(|_| NativeError::Code(NativeCode::ShmMeasureFailed))?;
        let name = std::ffi::CString::new(format!("newchan-n1-{}", owner.token)).unwrap();
        let mut vfs = Box::new(unsafe { *parent });
        vfs.szOsFile += std::mem::size_of::<ShimFile>() as c_int;
        vfs.zName = name.as_ptr();
        vfs.pNext = std::ptr::null_mut();
        vfs.pAppData = (&mut *context as *mut Context).cast();
        vfs.xOpen = Some(open);
        vfs.xDelete = Some(delete);
        vfs.xSleep = Some(sleep);
        vfs.xAccess = Some(access);
        vfs.xFullPathname = Some(full_path);
        vfs.xRandomness = Some(randomness);
        vfs.xCurrentTime = Some(current_time);
        vfs.xCurrentTimeInt64 = Some(current_time64);
        vfs.xGetLastError = Some(last_error);
        // N1 不加载动态扩展，不允许注册入口暴露真实VFS或修改syscall表。
        vfs.xDlOpen = None;
        vfs.xDlError = None;
        vfs.xDlSym = None;
        vfs.xDlClose = None;
        vfs.xSetSystemCall = None;
        vfs.xGetSystemCall = None;
        vfs.xNextSystemCall = None;
        if unsafe { ffi::sqlite3_vfs_register(&mut *vfs, 0) } != ffi::SQLITE_OK {
            return Err(NativeCode::Unavailable.into());
        }
        Ok(Self {
            vfs,
            name,
            context,
            registered: true,
        })
    }
    pub(super) fn name(&self) -> *const c_char {
        self.name.as_ptr()
    }
    pub(super) fn measured(&self) -> Result<u64, ()> {
        self.context.measured()
    }
    pub(super) fn unregister(&mut self) -> c_int {
        if !self.registered {
            return ffi::SQLITE_OK;
        }
        let rc = unsafe { ffi::sqlite3_vfs_unregister(&mut *self.vfs) };
        if rc == ffi::SQLITE_OK {
            self.registered = false;
        }
        rc
    }
}

// Session在sqlite3_close成功之后才销毁Registration；Alive整个state泄漏保活。
impl Drop for Registration {
    fn drop(&mut self) {
        if self.registered {
            let _ = self.unregister();
        }
    }
}

#[repr(C)]
struct ShimFile {
    base: ffi::sqlite3_file,
    real: *mut ffi::sqlite3_file,
    context: *const Context,
}

unsafe fn context<'a>(vfs: *mut ffi::sqlite3_vfs) -> &'a Context {
    &*(*vfs).pAppData.cast::<Context>()
}
unsafe fn parts<'a>(
    file: *mut ffi::sqlite3_file,
) -> (
    &'a Context,
    *mut ffi::sqlite3_file,
    &'a ffi::sqlite3_io_methods,
) {
    let shim = &*file.cast::<ShimFile>();
    (&*shim.context, shim.real, &*(*shim.real).pMethods)
}
fn protected(ctx: &Context, failure: c_int, run: impl FnOnce() -> c_int) -> c_int {
    catch_unwind(AssertUnwindSafe(run)).unwrap_or_else(|_| {
        ctx.fail(NativeCode::TrackingLost.into());
        failure
    })
}

unsafe extern "C" fn open(
    vfs: *mut ffi::sqlite3_vfs,
    name: *const c_char,
    file: *mut ffi::sqlite3_file,
    flags: c_int,
    out: *mut c_int,
) -> c_int {
    let ctx = context(vfs);
    protected(ctx, ffi::SQLITE_CANTOPEN, || {
        if ctx.gate().is_err() {
            return ffi::SQLITE_CANTOPEN;
        }
        if flags & (ffi::SQLITE_OPEN_MAIN_DB | ffi::SQLITE_OPEN_WAL) == 0 {
            ctx.fail(NativeError::Business("NativeUnexpectedFile".into()));
            return ffi::SQLITE_CANTOPEN;
        }
        let real = file
            .cast::<u8>()
            .add(std::mem::size_of::<ShimFile>())
            .cast::<ffi::sqlite3_file>();
        file.cast::<ShimFile>().write(ShimFile {
            base: ffi::sqlite3_file {
                pMethods: std::ptr::null(),
            },
            real,
            context: ctx,
        });
        let rc = (*ctx.parent).xOpen.unwrap()(
            ctx.parent,
            name,
            real,
            flags & !ffi::SQLITE_OPEN_CREATE,
            out,
        );
        if !(*real).pMethods.is_null() {
            (*file).pMethods = &METHODS;
        }
        if rc != ffi::SQLITE_OK {
            return rc;
        }
        let class = if flags & ffi::SQLITE_OPEN_MAIN_DB != 0 {
            Class::Main
        } else {
            Class::Wal
        };
        let receipt = ctx
            .hooks
            .lock()
            .active_for(ctx.token)
            .is_ok_and(|a| a.receipts.values().any(|c| *c == class));
        if !receipt {
            ctx.fail(NativeCode::IdentityMismatch.into());
            return ffi::SQLITE_CANTOPEN;
        }
        ffi::SQLITE_OK
    })
}

unsafe extern "C" fn delete(
    vfs: *mut ffi::sqlite3_vfs,
    _name: *const c_char,
    _sync: c_int,
) -> c_int {
    context(vfs).fail(NativeError::Business("NativeSidecarDeleteDenied".into()));
    ffi::SQLITE_IOERR_DELETE
}
unsafe extern "C" fn sleep(vfs: *mut ffi::sqlite3_vfs, micros: c_int) -> c_int {
    let ctx = context(vfs);
    protected(ctx, 0, || {
        if ctx.gate().is_err() {
            0
        } else {
            (*ctx.parent).xSleep.unwrap()(ctx.parent, micros)
        }
    })
}
macro_rules! vfs_forward {
    ($name:ident, $method:ident, ($($arg:ident:$ty:ty),*) -> $ret:ty) => {
        unsafe extern "C" fn $name(vfs:*mut ffi::sqlite3_vfs, $($arg:$ty),*) -> $ret {
            let parent = context(vfs).parent;
            (*parent).$method.unwrap()(parent, $($arg),*)
        }
    };
}
vfs_forward!(access, xAccess, (name:*const c_char, flags:c_int, out:*mut c_int) -> c_int);
vfs_forward!(full_path, xFullPathname, (name:*const c_char, n:c_int, out:*mut c_char) -> c_int);
vfs_forward!(randomness, xRandomness, (n:c_int, out:*mut c_char) -> c_int);
vfs_forward!(current_time, xCurrentTime, (out:*mut f64) -> c_int);
vfs_forward!(current_time64, xCurrentTimeInt64, (out:*mut i64) -> c_int);
vfs_forward!(last_error, xGetLastError, (n:c_int, out:*mut c_char) -> c_int);

macro_rules! io_gate {
    ($name:ident, $method:ident, $error:expr, ($($arg:ident:$ty:ty),*)) => {
        unsafe extern "C" fn $name(file:*mut ffi::sqlite3_file, $($arg:$ty),*) -> c_int {
            let (ctx, real, methods) = parts(file);
            protected(ctx, $error, || if ctx.gate().is_err() { $error } else { methods.$method.unwrap()(real, $($arg),*) })
        }
    };
}
io_gate!(read, xRead, ffi::SQLITE_IOERR_READ, (buffer:*mut c_void, n:c_int, offset:i64));
io_gate!(write, xWrite, ffi::SQLITE_IOERR_WRITE, (buffer:*const c_void, n:c_int, offset:i64));
io_gate!(truncate, xTruncate, ffi::SQLITE_IOERR_TRUNCATE, (size:i64));
io_gate!(sync, xSync, ffi::SQLITE_IOERR_FSYNC, (flags:c_int));
io_gate!(file_size, xFileSize, ffi::SQLITE_IOERR_FSTAT, (out:*mut i64));
io_gate!(lock, xLock, ffi::SQLITE_IOERR_LOCK, (level:c_int));
macro_rules! io_forward {
    ($name:ident, $method:ident, ($($arg:ident:$ty:ty),*)) => {
        unsafe extern "C" fn $name(file:*mut ffi::sqlite3_file, $($arg:$ty),*) -> c_int {
            let (_, real, methods) = parts(file);
            methods.$method.unwrap()(real, $($arg),*)
        }
    };
}
io_forward!(close, xClose, ());
io_forward!(unlock, xUnlock, (level:c_int));
io_forward!(reserved, xCheckReservedLock, (out:*mut c_int));
io_forward!(sector_size, xSectorSize, ());
io_forward!(device_characteristics, xDeviceCharacteristics, ());
unsafe extern "C" fn file_control(
    file: *mut ffi::sqlite3_file,
    op: c_int,
    arg: *mut c_void,
) -> c_int {
    let (_, real, methods) = parts(file);
    if op == ffi::SQLITE_FCNTL_MMAP_SIZE {
        if !arg.is_null() {
            *arg.cast::<i64>() = 0;
        }
    }
    methods.xFileControl.unwrap()(real, op, arg)
}
unsafe extern "C" fn shm_map(
    file: *mut ffi::sqlite3_file,
    page: c_int,
    size: c_int,
    extend: c_int,
    out: *mut *mut c_void,
) -> c_int {
    let (ctx, real, methods) = parts(file);
    protected(ctx, ffi::SQLITE_IOERR_SHMMAP, || {
        if ctx.gate().is_err() {
            return ffi::SQLITE_IOERR_SHMMAP;
        }
        if page < 0 || size <= 0 {
            return ffi::SQLITE_IOERR_SHMSIZE;
        }
        let per = (libc::sysconf(libc::_SC_PAGESIZE).max(1) as u64 / size as u64).max(1);
        let end = (page as u64 + 1)
            .div_ceil(per)
            .saturating_mul(per)
            .saturating_mul(size as u64);
        let covered = with_owner(|tls| {
            let result = tls.budget.cover(&tls.gate, end);
            if let Err(error) = &result {
                tls.fail(error.clone());
            }
            if result.is_ok() {
                tls.budget.mark_delegated();
            }
            result
        });
        if !matches!(covered, Some(Ok(()))) {
            return ffi::SQLITE_IOERR_SHMSIZE;
        }
        let Some(map) = methods.xShmMap else {
            return ffi::SQLITE_IOERR_SHMMAP;
        };
        let rc = map(real, page, size, extend, out);
        let measured = ctx.measured();
        let checked = with_owner(|tls| match measured {
            Err(()) => {
                tls.fail(NativeCode::ShmMeasureFailed.into());
                false
            }
            Ok(bytes) => {
                tls.budget.observed = tls.budget.observed.max(bytes);
                if bytes > tls.budget.floor() {
                    tls.fail(NativeCode::ShmBoundViolated.into());
                    false
                } else {
                    true
                }
            }
        })
        .unwrap_or(false);
        if !checked {
            ffi::SQLITE_IOERR_SHMSIZE
        } else {
            rc
        }
    })
}
unsafe extern "C" fn shm_lock(
    file: *mut ffi::sqlite3_file,
    offset: c_int,
    n: c_int,
    flags: c_int,
) -> c_int {
    let (ctx, real, methods) = parts(file);
    protected(ctx, ffi::SQLITE_IOERR_LOCK, || {
        if flags & ffi::SQLITE_SHM_UNLOCK == 0 && ctx.gate().is_err() {
            return ffi::SQLITE_IOERR_LOCK;
        }
        methods.xShmLock.map_or(ffi::SQLITE_IOERR_SHMLOCK, |call| {
            call(real, offset, n, flags)
        })
    })
}
unsafe extern "C" fn shm_barrier(file: *mut ffi::sqlite3_file) {
    let (_, real, methods) = parts(file);
    if let Some(call) = methods.xShmBarrier {
        call(real);
    }
}
unsafe extern "C" fn shm_unmap(file: *mut ffi::sqlite3_file, _delete: c_int) -> c_int {
    let (_, real, methods) = parts(file);
    methods
        .xShmUnmap
        .map_or(ffi::SQLITE_OK, |call| call(real, 0))
}
unsafe extern "C" fn fetch(
    _file: *mut ffi::sqlite3_file,
    _offset: i64,
    _n: c_int,
    out: *mut *mut c_void,
) -> c_int {
    *out = std::ptr::null_mut();
    ffi::SQLITE_OK
}
unsafe extern "C" fn unfetch(
    _file: *mut ffi::sqlite3_file,
    _offset: i64,
    _p: *mut c_void,
) -> c_int {
    ffi::SQLITE_OK
}
static METHODS: ffi::sqlite3_io_methods = ffi::sqlite3_io_methods {
    iVersion: 3,
    xClose: Some(close),
    xRead: Some(read),
    xWrite: Some(write),
    xTruncate: Some(truncate),
    xSync: Some(sync),
    xFileSize: Some(file_size),
    xLock: Some(lock),
    xUnlock: Some(unlock),
    xCheckReservedLock: Some(reserved),
    xFileControl: Some(file_control),
    xSectorSize: Some(sector_size),
    xDeviceCharacteristics: Some(device_characteristics),
    xShmMap: Some(shm_map),
    xShmLock: Some(shm_lock),
    xShmBarrier: Some(shm_barrier),
    xShmUnmap: Some(shm_unmap),
    xFetch: Some(fetch),
    xUnfetch: Some(unfetch),
};
