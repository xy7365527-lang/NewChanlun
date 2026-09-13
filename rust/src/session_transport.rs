//! #1374：每个 S/E/B 独立实例复用的有界 Unix 输送与进程排他锁。
//!
//! 输送线程只解帧和排队；权威 handler 在调用线程串行执行。
//! 本模块不打开数据库、不恢复领域状态、不分派跨域事务。调用者先核域前件并持锁。
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

pub fn read_frame(
    stream: &mut UnixStream,
    max: usize,
    deadline: Instant,
    validate: EnvelopeValidator,
) -> Result<Value, String> {
    let mut trace = ReadTrace::start(stream);
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
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
pub fn enqueue_before_deadline(
    tx: &mpsc::SyncSender<Job>,
    mut job: Job,
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
    mut stream: UnixStream,
    tx: mpsc::SyncSender<Job>,
    bounds: TransportBounds,
    count: Arc<AtomicUsize>,
    validate: EnvelopeValidator,
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
            let job = Job {
                envelope: e,
                result: reply_tx,
            };
            if !enqueue_before_deadline(&tx, job, response_deadline) {
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
    // 不擅自unlink既有socket：SIGKILL残留由监督器核路径/PID后清理。
    let listener = UnixListener::bind(socket)
        .map_err(|e| format!("TransportUnavailable：socket绑定失败 {e}"))?;
    let (tx, rx) = mpsc::sync_channel::<Job>(bounds.queue);
    let count = Arc::new(AtomicUsize::new(0));
    let accept_bounds = bounds.clone();
    std::thread::spawn(move || {
        for incoming in listener.incoming() {
            let Ok(stream) = incoming else { break };
            if count
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
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
            std::thread::spawn(move || io_connection(stream, t, b, c, validate));
        }
    });
    for job in rx {
        let out = handler(&job.envelope);
        let _ = job.result.try_send(out);
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
}
