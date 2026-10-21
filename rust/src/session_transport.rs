//! #1374：每个 S/E/B 独立实例复用的有界 Unix 输送与进程排他锁。
//!
//! 输送线程只解帧和排队；权威 handler 在调用线程串行执行。
//! 本模块不打开数据库、不恢复领域状态、不分派跨域事务。调用者先核域前件并持锁。
//!
//! #1394 C §5.6.1：读/写暴露带观测回调的入口（[`read_frame_observed`]／[`write_frame_observed`]），
//! 让 job 记账按**实际发生的字节**在错误判定之前回调；[`read_frame`] 的原签名与行为保持不变。
use crate::session_protocol::{canonical_json, strict_json};
use serde_json::{json, Value};
use std::fmt;
use std::fs::{File, OpenOptions};
#[cfg(any(test, not(target_os = "macos")))]
use std::io::Read;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};

fn err(e: impl fmt::Display) -> String {
    format!("StorageUnavailable：{e}")
}

/// 每个域入口显式绑定自己的 schema 校验器；不是 producer 权威证明。
pub type EnvelopeValidator = fn(Value) -> Result<Value, String>;

/// 单个服务实例的请求/响应绝对期限及空间上界。
#[derive(Clone)]
pub struct TransportBounds {
    pub frame: usize,
    pub read: Duration,
    pub write: Duration,
    pub response: Duration,
    pub queue: usize,
    pub connections: usize,
}

/// 规范化数据库路径后持 OS 锁；返回 File 的生存期即持锁期。
pub fn process_lock(db: &Path, domain_name: &str) -> Result<File, String> {
    let canonical = if db.exists() {
        db.canonicalize().map_err(err)?
    } else {
        let parent = db
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        parent
            .canonicalize()
            .map_err(err)?
            .join(db.file_name().ok_or("InvalidDomain：数据库路径缺文件名")?)
    };
    let p = PathBuf::from(format!("{}.writer.lock", canonical.display()));
    let f = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(p)
        .map_err(err)?;
    f.try_lock()
        .map_err(|e| format!("StaleWriter：{domain_name}独占进程锁不可得：{e}"))?;
    Ok(f)
}
/// 单写队列项；回包通道不拥有或撤销权威业务事务。
pub struct Job {
    pub envelope: Value,
    pub result: mpsc::SyncSender<Value>,
}
/// Trusted in-process metadata from the I/O thread. Never serialized into the
/// public envelope, payload hash, request bytes, receipt or economic image.
struct TimedJob {
    job: Job,
    response_deadline: Instant,
}
trait QueuedJob: Send {
    fn result(&self) -> &mpsc::SyncSender<Value>;
}
impl QueuedJob for Job {
    fn result(&self) -> &mpsc::SyncSender<Value> {
        &self.result
    }
}
impl QueuedJob for TimedJob {
    fn result(&self) -> &mpsc::SyncSender<Value> {
        &self.job.result
    }
}
fn legacy_job(envelope: Value, result: mpsc::SyncSender<Value>, _: Instant) -> Job {
    Job { envelope, result }
}
fn timed_job(
    envelope: Value,
    result: mpsc::SyncSender<Value>,
    response_deadline: Instant,
) -> TimedJob {
    TimedJob {
        job: Job { envelope, result },
        response_deadline,
    }
}
// #1374 TestOnly：默认完全关闭，记录输送阶段且不进入公共回复或权威数据库。
// 每连接最多2048条读事件；只存长度/摘要/OS状态，不存原始业务内容。
struct ReadTrace {
    path: PathBuf,
    fd: i32,
    started: Instant,
    events: Vec<Value>,
    dropped_events: usize,
}

impl ReadTrace {
    fn start(stream: &UnixStream) -> Option<Self> {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let directory = std::env::var_os("SESSION_TRANSPORT_TESTONLY_TRACE_DIR")?;
        let directory = PathBuf::from(directory);
        if !directory.is_absolute() || !directory.is_dir() {
            return None;
        }
        let fd = stream.as_raw_fd();
        let number = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut trace = Self {
            path: directory.join(format!("read-{}-{fd}-{number}.json", std::process::id())),
            fd,
            started: Instant::now(),
            events: Vec::new(),
            dropped_events: 0,
        };
        trace.push(
            json!({"phase":"start","pid":std::process::id(),"fd":fd,"fd_state":fd_state(fd)}),
        );
        Some(trace)
    }

    fn push(&mut self, mut event: Value) {
        event["elapsed_ns"] = json!(self.started.elapsed().as_nanos().to_string());
        event["monotonic_ns"] = monotonic_ns()
            .map(|v| json!(v.to_string()))
            .unwrap_or(Value::Null);
        if self.events.len() >= 2048 {
            self.dropped_events += 1;
            // 保留最早事件和最后事件，EOF/error不能被容量上限吞掉。
            *self.events.last_mut().unwrap() = event;
        } else {
            self.events.push(event);
        }
    }
}

impl Drop for ReadTrace {
    fn drop(&mut self) {
        let record = json!({"scope":"TestOnly transport read diagnostics","clock_source":"clock_gettime(CLOCK_MONOTONIC); compare same source or elapsed_ns","fd":self.fd,"events":self.events,"dropped_events":self.dropped_events,"final_fd_state":fd_state(self.fd)});
        if let Ok(mut file) = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.path)
        {
            let _ = file.write_all(canonical_json(&record).as_bytes());
            let _ = file.write_all(b"\n");
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn fd_state(fd: i32) -> Value {
    unsafe extern "C" {
        fn fcntl(fd: i32, command: i32, ...) -> i32;
    }
    // F_GETFD=1/F_GETFL=3 on both supported platforms; these reads do not modify fd flags.
    let fd_flags = unsafe { fcntl(fd, 1) };
    let status_flags = unsafe { fcntl(fd, 3) };
    json!({"fd_flags":fd_flags,"status_flags":status_flags})
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn fd_state(_fd: i32) -> Value {
    Value::Null
}

// TestOnly：不消费任何字节，用零等待poll/peek观察超时后的实际EOF状态。
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn eof_state(fd: i32) -> Value {
    #[repr(C)]
    struct PollFd {
        fd: i32,
        events: i16,
        revents: i16,
    }
    #[cfg(target_os = "macos")]
    type Nfds = u32;
    #[cfg(target_os = "linux")]
    type Nfds = std::os::raw::c_ulong;
    unsafe extern "C" {
        fn poll(fds: *mut PollFd, count: Nfds, timeout: i32) -> i32;
        fn recv(fd: i32, buffer: *mut u8, len: usize, flags: i32) -> isize;
    }
    let mut state = PollFd {
        fd,
        events: 1,
        revents: 0,
    };
    let result = unsafe { poll(&mut state, 1, 0) };
    #[cfg(target_os = "macos")]
    const DONTWAIT: i32 = 0x80;
    #[cfg(target_os = "linux")]
    const DONTWAIT: i32 = 0x40;
    let mut byte = 0;
    let peek = unsafe { recv(fd, &mut byte, 1, 2 | DONTWAIT) };
    let error = if peek < 0 {
        std::io::Error::last_os_error().raw_os_error()
    } else {
        None
    };
    json!({"poll_result":result,"poll_revents":state.revents,"peek_result":peek,"peek_os_error":error})
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn eof_state(_fd: i32) -> Value {
    Value::Null
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn monotonic_ns() -> Option<i128> {
    #[repr(C)]
    struct TimeSpec {
        sec: std::os::raw::c_long,
        nsec: std::os::raw::c_long,
    }
    unsafe extern "C" {
        fn clock_gettime(clock: i32, value: *mut TimeSpec) -> i32;
    }
    #[cfg(target_os = "macos")]
    const CLOCK_MONOTONIC: i32 = 6;
    #[cfg(target_os = "linux")]
    const CLOCK_MONOTONIC: i32 = 1;
    let mut time = TimeSpec { sec: 0, nsec: 0 };
    if unsafe { clock_gettime(CLOCK_MONOTONIC, &mut time) } != 0 {
        return None;
    }
    Some(i128::from(time.sec) * 1_000_000_000 + i128::from(time.nsec))
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn monotonic_ns() -> Option<i128> {
    None
}

// #1374：本机Darwin25.6在SHUT_WR与排空并发时，阻塞recv可漏掉已存在的EOF，
// 直到SO_RCVTIMEO报WouldBlock（现场同刻poll=POLLIN|POLLHUP且非阻塞peek=0）。
// macOS用poll等待同一连接就绪，再非阻塞recv，不能把LF或超时当成EOF。
#[cfg(target_os = "macos")]
fn read_chunk(
    stream: &mut UnixStream,
    bytes: &mut [u8],
    deadline: Instant,
) -> std::io::Result<usize> {
    #[repr(C)]
    struct PollFd {
        fd: i32,
        events: i16,
        revents: i16,
    }
    unsafe extern "C" {
        fn poll(fds: *mut PollFd, count: u32, timeout: i32) -> i32;
        fn recv(fd: i32, buffer: *mut u8, len: usize, flags: i32) -> isize;
    }
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::TimedOut, "请求读期限"))?;
        // poll的毫秒粒度向上取整；返回后仍重核原绝对期限，不延长预算。
        let millis = remaining
            .as_nanos()
            .div_ceil(1_000_000)
            .min(i32::MAX as u128) as i32;
        let mut state = PollFd {
            fd: stream.as_raw_fd(),
            events: 1,
            revents: 0,
        };
        let ready = unsafe { poll(&mut state, 1, millis) };
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if ready == 0 {
            continue;
        }
        if Instant::now() >= deadline {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "请求读期限",
            ));
        }
        // MSG_DONTWAIT=0x80（macOS SDK）。不改fd flags，后续响应仍用原有有界写路径。
        let size = unsafe { recv(stream.as_raw_fd(), bytes.as_mut_ptr(), bytes.len(), 0x80) };
        if size >= 0 {
            return Ok(size as usize);
        }
        let error = std::io::Error::last_os_error();
        if matches!(
            error.kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
        ) {
            // 就绪提示不承诺read一定返回数据；只继续同一帧的接收，绝不重投业务消息。
            continue;
        }
        return Err(error);
    }
}
#[cfg(not(target_os = "macos"))]
fn read_chunk(
    stream: &mut UnixStream,
    bytes: &mut [u8],
    _deadline: Instant,
) -> std::io::Result<usize> {
    stream.read(bytes)
}

/// 帧观测口：唯一读循环与分块写循环共用的计费与期限钩子（#1394 C §5.6.1）。
///
/// 调用时序（读、写一致）：
/// 1. 每次**真正 I/O 之前** `before_io`：job 层在这里拒绝已停止或已过绝对期限的连接，避免开工；
/// 2. 每次底层读/写成功返回 `n>0` 之后、**任何** max/JSON/schema 判断之前 `after_bytes`，
///    按实际 `n` 字节回调。错误路径（坏 JSON、validate 拒绝、超单帧上限、EOF 或超时前的前缀、
///    部分写后失败）不会撤回首字节——它们在错误返回之前已经逐次回调过。
///
/// [`NoObserver`] 什么都不做，[`read_frame`] 用它保持原有签名与行为。
pub trait FrameObserver {
    fn before_io(&mut self) -> Result<(), String>;
    fn after_bytes(&mut self, bytes: &[u8]) -> Result<(), String>;
}

/// 不记账的观测口：[`read_frame`] 的默认行为。
pub struct NoObserver;

impl FrameObserver for NoObserver {
    fn before_io(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn after_bytes(&mut self, _bytes: &[u8]) -> Result<(), String> {
        Ok(())
    }
}

/// 固定写块上界：每次真正写最多这么多字节，不把整个 body 交给一次 `write`。
pub const WRITE_CHUNK_BYTES: usize = 4096;

/// 逐块写目标：真实实现是 [`UnixStream`]；测试可注入确定复现的部分写与失败。
/// 本 trait 只抽「一次真正写」这一块，期限、分块上界与计费口径仍在 [`write_frame_observed`] 里统一。
pub trait FrameSink {
    /// 一次真正写：返回实际写入字节数；`Err`/`0` 都如实返回，不得伪装成部分进度。
    fn write_some(&mut self, chunk: &[u8]) -> std::io::Result<usize>;

    /// 下一次真正写之前的等待上限（真实 socket 用它设 `SO_SNDTIMEO`）；默认不动作。
    fn set_chunk_timeout(&mut self, remaining: Duration) -> std::io::Result<()> {
        let _ = remaining;
        Ok(())
    }
}

impl FrameSink for UnixStream {
    fn write_some(&mut self, chunk: &[u8]) -> std::io::Result<usize> {
        self.write(chunk)
    }

    fn set_chunk_timeout(&mut self, remaining: Duration) -> std::io::Result<()> {
        self.set_write_timeout(Some(remaining))
    }
}

pub fn read_frame(
    stream: &mut UnixStream,
    max: usize,
    deadline: Instant,
    validate: EnvelopeValidator,
) -> Result<Value, String> {
    read_frame_observed(stream, max, deadline, validate, &mut NoObserver)
}

/// 带观测回调的唯一读循环：解析、EOF、超时与单帧约束与 [`read_frame`] 完全一致。
///
/// 每次底层成功读到的 `n>0` 字节在 max/JSON/schema 判断之前先经 `observer.after_bytes`；
/// 已读前缀之后的 EOF/IO 错误不会漏账（前缀在各自成功读到的那一刻已回调）。
pub fn read_frame_observed(
    stream: &mut UnixStream,
    max: usize,
    deadline: Instant,
    validate: EnvelopeValidator,
    observer: &mut dyn FrameObserver,
) -> Result<Value, String> {
    let mut trace = ReadTrace::start(stream);
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        observer.before_io()?;
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("TransportTimeout：请求读期限")?;
        #[cfg(not(target_os = "macos"))]
        stream.set_read_timeout(Some(remaining)).map_err(err)?;
        if let Some(t) = &mut trace {
            t.push(json!({"phase":"before_read","total_bytes":bytes.len(),"remaining_ns":remaining.as_nanos().to_string(),"fd_state":fd_state(stream.as_raw_fd())}));
        }
        let n = match read_chunk(stream, &mut chunk, deadline) {
            Ok(n) => {
                if let Some(t) = &mut trace {
                    t.push(json!({"phase":if n==0 {"eof"} else {"read"},"read_bytes":n,"total_bytes":bytes.len()+n}));
                }
                n
            }
            Err(e) => {
                if let Some(t) = &mut trace {
                    t.push(json!({"phase":"read_error","eof_state":eof_state(stream.as_raw_fd()),"error":e.to_string(),"error_kind":format!("{:?}",e.kind()),"os_error":e.raw_os_error(),"total_bytes":bytes.len(),"ends_with_lf":bytes.last()==Some(&b'\n'),"bytes_sha256":crate::session_protocol::sha256_hex(&bytes),"fd_state":fd_state(stream.as_raw_fd())}));
                }
                if e.kind() == std::io::ErrorKind::TimedOut {
                    return Err("TransportTimeout：请求读期限".into());
                }
                return Err(format!("TransportUnavailable：{e}"));
            }
        };
        if n == 0 {
            break;
        }
        // 先按实际读到的字节回调（含随后会触发 max/JSON/schema 错误的这一块），再判上限。
        observer.after_bytes(&chunk[..n])?;
        if bytes.len() + n > max {
            return Err("SchemaUnsupported：frame字节超过已声明界限".into());
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
    if bytes.last() != Some(&b'\n') || bytes[..bytes.len().saturating_sub(1)].contains(&b'\n') {
        return Err("SchemaUnsupported：只接收一个LF终止帧与请求EOF".into());
    }
    let value = validate(strict_json(&bytes[..bytes.len() - 1])?)?;
    if Instant::now() > deadline {
        return Err("TransportTimeout：JSON解码超期".into());
    }
    Ok(value)
}

/// 带观测回调的分块写：每次真正写成功 `n>0` 字节后、在做任何后续判断之前按**实际 n** 回调。
///
/// 写入按固定小上界（[`WRITE_CHUNK_BYTES`]）分块，不一次写完整 body；部分写后失败时，
/// 已成功写出的前缀不会被撤回（调用方必须先收起已回调的账，再处理错误）。
pub fn write_frame_observed(
    sink: &mut dyn FrameSink,
    body: &[u8],
    deadline: Instant,
    observer: &mut dyn FrameObserver,
) -> Result<(), String> {
    let mut sent = 0;
    while sent < body.len() {
        observer.before_io()?;
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|left| !left.is_zero())
            .ok_or("TransportTimeout：请求写期限")?;
        sink.set_chunk_timeout(remaining)
            .map_err(|e| format!("TransportUnavailable：{e}"))?;
        let end = (sent + WRITE_CHUNK_BYTES).min(body.len());
        let n = sink
            .write_some(&body[sent..end])
            .map_err(|e| format!("TransportUnavailable：{e}"))?;
        if n == 0 {
            return Err("TransportUnavailable：写入返回 0 字节".into());
        }
        if n > end - sent {
            return Err("TransportUnavailable：写入返回数超过本次请求长度".into());
        }
        observer.after_bytes(&body[sent..sent + n])?;
        sent += n;
    }
    Ok(())
}

pub fn enqueue_before_deadline(tx: &mpsc::SyncSender<Job>, job: Job, deadline: Instant) -> bool {
    enqueue_before_deadline_inner(tx, job, deadline)
}
fn enqueue_before_deadline_inner<T>(
    tx: &mpsc::SyncSender<T>,
    mut job: T,
    deadline: Instant,
) -> bool {
    loop {
        // #1372：等待队列后先重核同一期限，不能在过期后的下一轮入队。
        if Instant::now() >= deadline {
            return false;
        }
        match tx.try_send(job) {
            Ok(()) => return true,
            Err(mpsc::TrySendError::Full(returned)) => {
                job = returned;
                std::thread::sleep(
                    Duration::from_millis(1)
                        .min(deadline.saturating_duration_since(Instant::now())),
                );
            }
            Err(_) => return false,
        }
    }
}

pub fn io_connection(
    stream: UnixStream,
    tx: mpsc::SyncSender<Job>,
    bounds: TransportBounds,
    count: Arc<AtomicUsize>,
    validate: EnvelopeValidator,
) {
    io_connection_inner(stream, tx, bounds, count, validate, legacy_job)
}
fn io_connection_with_deadline(
    stream: UnixStream,
    tx: mpsc::SyncSender<TimedJob>,
    bounds: TransportBounds,
    count: Arc<AtomicUsize>,
    validate: EnvelopeValidator,
) {
    io_connection_inner(stream, tx, bounds, count, validate, timed_job)
}
fn io_connection_inner<T: Send + 'static>(
    mut stream: UnixStream,
    tx: mpsc::SyncSender<T>,
    bounds: TransportBounds,
    count: Arc<AtomicUsize>,
    validate: EnvelopeValidator,
    make_job: fn(Value, mpsc::SyncSender<Value>, Instant) -> T,
) {
    struct Release(Arc<AtomicUsize>);
    impl Drop for Release {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }
    let _release = Release(count);
    let deadline = Instant::now() + bounds.read;
    let output = match read_frame(&mut stream, bounds.frame, deadline, validate) {
        Err(error) => json!({"ok":false,"error":error}),
        Ok(e) => {
            let response_deadline = Instant::now() + bounds.response;
            let (reply_tx, reply_rx) = mpsc::sync_channel(1);
            let job = make_job(e, reply_tx, response_deadline);
            if !enqueue_before_deadline_inner(&tx, job, response_deadline) {
                return;
            }
            // 已入actor后，回包超期只结束此连接；不能撤销/重复权威工作。
            match reply_rx.recv_timeout(response_deadline.saturating_duration_since(Instant::now()))
            {
                Ok(v) => v,
                Err(_) => return,
            }
        }
    };
    let body = format!("{}\n", canonical_json(&output));
    if body.len() > bounds.frame {
        return;
    }
    let deadline = Instant::now() + bounds.write;
    let mut sent = 0;
    while sent < body.len() {
        let Some(left) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        if stream.set_write_timeout(Some(left)).is_err() {
            break;
        }
        match stream.write(&body.as_bytes()[sent..]) {
            Ok(0) | Err(_) => break,
            Ok(n) => sent += n,
        }
    }
    let _ = stream.shutdown(std::net::Shutdown::Both);
}
/// 启动有界输送并在当前线程串行调用域 handler；慢/失联客户不阻塞下一权威工作。
/// 不清理既有 socket；残留路径由监督器核实际 PID/控制身份后处理。
pub fn serve_unix(
    socket: &Path,
    bounds: TransportBounds,
    validate: EnvelopeValidator,
    mut handler: impl FnMut(&Value) -> Value,
) -> Result<(), String> {
    serve_unix_inner(socket, bounds, validate, legacy_job, |job: &Job| {
        handler(&job.envelope)
    })
}
/// New economic domain only: deliver the I/O thread's original response
/// deadline to the actor as trusted memory context. It is never wire data.
pub fn serve_unix_with_deadline(
    socket: &Path,
    bounds: TransportBounds,
    validate: EnvelopeValidator,
    mut handler: impl FnMut(&Value, Instant) -> Value,
) -> Result<(), String> {
    serve_unix_inner(socket, bounds, validate, timed_job, |job: &TimedJob| {
        handler(&job.job.envelope, job.response_deadline)
    })
}
fn serve_unix_inner<T: QueuedJob + 'static>(
    socket: &Path,
    bounds: TransportBounds,
    validate: EnvelopeValidator,
    make_job: fn(Value, mpsc::SyncSender<Value>, Instant) -> T,
    mut handler: impl FnMut(&T) -> Value,
) -> Result<(), String> {
    // 不擅自unlink既有socket：SIGKILL残留由监督器核路径/PID后清理。
    let listener = UnixListener::bind(socket)
        .map_err(|e| format!("TransportUnavailable：socket绑定失败 {e}"))?;
    let (tx, rx) = mpsc::sync_channel::<T>(bounds.queue);
    let count = Arc::new(AtomicUsize::new(0));
    let accept_bounds = bounds.clone();
    std::thread::spawn(move || {
        for incoming in listener.incoming() {
            let Ok(stream) = incoming else { break };
            if count
                .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
                    if n < accept_bounds.connections {
                        Some(n + 1)
                    } else {
                        None
                    }
                })
                .is_err()
            {
                drop(stream);
                continue;
            }
            let t = tx.clone();
            let b = accept_bounds.clone();
            let c = count.clone();
            std::thread::spawn(move || io_connection_inner(stream, t, b, c, validate, make_job));
        }
    });
    for job in rx {
        let out = handler(&job);
        let _ = job.result().try_send(out);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn value_only(value: Value) -> Result<Value, String> {
        Ok(value)
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn half_close_during_large_frame_drain_observes_actual_eof() {
        let value = json!({"raw":"x".repeat(22000)});
        let body = format!("{}\n", canonical_json(&value)).into_bytes();
        for index in 0..2048 {
            let (mut receiver, mut sender) = UnixStream::pair().unwrap();
            let frame = body.clone();
            let thread = std::thread::spawn(move || {
                sender.write_all(&frame).unwrap();
                let delay = Duration::from_nanos(
                    [0, 1000, 5000, 10000, 25000, 50000, 100000, 250000][index % 8],
                );
                let until = Instant::now() + delay;
                while Instant::now() < until {
                    std::hint::spin_loop();
                }
                sender.shutdown(std::net::Shutdown::Write).unwrap();
                // 保持客户端读侧打开，不能靠close掩盖SHUT_WR半关闭语义。
                let mut reply = [0_u8; 1];
                let _ = sender.read(&mut reply);
            });
            let result = read_frame(
                &mut receiver,
                1048576,
                Instant::now() + Duration::from_secs(1),
                value_only,
            );
            drop(receiver);
            thread.join().unwrap();
            assert_eq!(result.unwrap(), value, "iteration {index}");
        }
    }

    #[test]
    fn lf_without_peer_eof_still_expires_without_accepting_a_frame() {
        let (mut receiver, mut sender) = UnixStream::pair().unwrap();
        sender.write_all(b"{\"valid\":true}\n").unwrap();
        let started = Instant::now();
        let result = read_frame(
            &mut receiver,
            4096,
            started + Duration::from_millis(40),
            value_only,
        );
        assert!(result.is_err());
        assert!(started.elapsed() >= Duration::from_millis(30));
        assert!(started.elapsed() < Duration::from_millis(500));
        sender.shutdown(std::net::Shutdown::Write).unwrap();
    }

    #[test]
    fn queued_unix_job_keeps_ingress_response_deadline_in_actor() {
        let bounds = TransportBounds {
            frame: 4096,
            read: Duration::from_millis(100),
            write: Duration::from_millis(100),
            response: Duration::from_millis(10),
            queue: 1,
            connections: 1,
        };
        let (mut client, server) = UnixStream::pair().unwrap();
        let (tx, rx) = mpsc::sync_channel::<TimedJob>(1);
        let count = Arc::new(AtomicUsize::new(1));
        let thread = std::thread::spawn(move || {
            io_connection_with_deadline(server, tx, bounds, count, value_only)
        });
        client.write_all(b"{\"op\":\"probe\"}\n").unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();
        let job = rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(job.job.envelope["op"], "probe");
        // Receipt proves enqueue completed. Delay handler work for twice the
        // response window after admission; a fresh now()+10ms would be wrong.
        std::thread::sleep(Duration::from_millis(20));
        assert!(Instant::now() >= job.response_deadline);
        let local_reset = Instant::now() + Duration::from_millis(10);
        assert!(local_reset > Instant::now());
        assert!(job.job.result.try_send(json!({"ok":true})).is_err());
        thread.join().unwrap();
    }
}
