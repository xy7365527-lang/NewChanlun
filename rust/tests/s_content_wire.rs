#![cfg(all(feature = "s_session", unix))]
//! #1394 C §5.6.1「真实 Unix I/O 的 job 计费与绝对期限」的接线回归。
//!
//! 验收面（全部在真实 `UnixStream::pair` 或可确定复现的写目标上）：
//! 唯一读循环与分块写循环按**实际 n 字节**累计 wire、每次 attempt 记一次 request
//! （重试全额再计）、同一 job 的绝对期限跨 attempt 不续期、旧每操作期限与 job 期限取更早者、
//! 错误路径（坏 JSON／validate 拒绝／超单帧上限／EOF 与超时前的前缀／部分写后失败）保留已发生字节、
//! 停止后不再开工、request 耗尽在 I/O 前拒绝。
//!
//! 边界（照实）：本文件不覆盖 S/E/B/Q 正式接入、不覆盖 connect/spawn/HTTP/stderr 字节、
//! 不构成 §5.6.1 的首批实测数值；用例里的八字段都是显式测试输入。

use newchan_rust::s_content::job::{Clock, Existing, Job, JobLimits, DEADLINE, LIMIT_EXCEEDED};
use newchan_rust::s_content::job_wire::WireAttempt;
use newchan_rust::session_protocol::canonical_json;
use newchan_rust::session_transport::{FrameSink, WRITE_CHUNK_BYTES};
use serde_json::{json, Value as J};
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// 显式八字段配置（规范十进制字符串）：exact 八项，无缺省、无无限量。
fn limits_json() -> J {
    json!({
        "max_total_wire_bytes": "65536",
        "max_total_node_bytes": "4096",
        "max_total_nodes": "64",
        "max_total_records": "64",
        "max_total_requests": "8",
        "max_elapsed_ms": "7000",
        "max_staged_bytes": "1024",
        "max_store_bytes": "8192",
    })
}

fn limits() -> JobLimits {
    JobLimits::from_canonical_json(&limits_json()).expect("八字段合法配置")
}

fn limits_with(edit: impl FnOnce(&mut JobLimits)) -> JobLimits {
    let mut limits = limits();
    edit(&mut limits);
    limits
}

/// 显式存量基线：0 只声明「实测为空」，不是缺省、也不等于服务默认值。
fn existing() -> Existing {
    Existing {
        store_bytes: 0,
        staged_bytes: 0,
    }
}

/// 假单调时钟：`store` 毫秒即推进；测试全程不 sleep（真实 socket 期限除外）。
fn mock_clock() -> (Arc<AtomicU64>, Instant, Box<dyn Clock>) {
    let ms = Arc::new(AtomicU64::new(0));
    let handle = ms.clone();
    let base = Instant::now();
    let clock: Box<dyn Clock> =
        Box::new(move || base + Duration::from_millis(handle.load(Ordering::SeqCst)));
    (ms, base, clock)
}

/// 假时钟停在 0：job 的绝对期限在测试期间不会自然到期。
fn job_with(limits: JobLimits) -> Job {
    let (_ms, _base, clock) = mock_clock();
    Job::new(limits, existing(), clock).expect("job 建立")
}

fn value_only(value: J) -> Result<J, String> {
    Ok(value)
}

fn reject_all(_value: J) -> Result<J, String> {
    Err("SchemaUnsupported：业务拒绝".to_string())
}

#[test]
fn successful_validation_and_empty_write_still_require_final_job_deadline() {
    thread_local! {
        static ELAPSED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    }
    fn validate_after_deadline(value: J) -> Result<J, String> {
        ELAPSED.with(|ms| ms.set(7000));
        Ok(value)
    }
    let base = Instant::now();
    let clock: Box<dyn Clock> =
        Box::new(move || base + Duration::from_millis(ELAPSED.with(|ms| ms.get())));
    let mut job = Job::new(limits(), existing(), clock).unwrap();
    let frame = b"{\"ok\":true}\n";
    let (mut receiver, peer) = peer_writes(frame.to_vec());
    let mut attempt = WireAttempt::begin(&mut job).unwrap();
    let outcome = attempt.read_frame(
        &mut receiver,
        4096,
        base + Duration::from_secs(60),
        validate_after_deadline,
    );
    drop(attempt);
    drop(receiver);
    peer.join().unwrap();
    assert!(outcome.unwrap_err().contains(DEADLINE));
    assert_eq!(job.snapshot().wire_bytes, frame.len() as u128);
    assert_eq!(job.snapshot().stopped, Some(DEADLINE));

    let (ms, base, clock) = mock_clock();
    let mut job = Job::new(limits(), existing(), clock).unwrap();
    let mut attempt = WireAttempt::begin(&mut job).unwrap();
    ms.store(7000, Ordering::SeqCst);
    let mut sink = PartialSink::default();
    assert!(attempt
        .write_frame(&mut sink, b"", base + Duration::from_secs(60))
        .unwrap_err()
        .contains(DEADLINE));
    assert_eq!(sink.calls, 0);
    assert_eq!(attempt.job().snapshot().wire_bytes, 0);
}

/// 对端写出 `bytes` 后 `SHUT_WR`：读循环靠 EOF 收帧（与既有 read_frame 语义一致）。
/// 读侧保持打开，直到调用方 drop 读端，半关闭语义不被 close 掩盖。
fn peer_writes(bytes: Vec<u8>) -> (UnixStream, JoinHandle<()>) {
    let (reader, mut writer) = UnixStream::pair().expect("socketpair");
    let handle = std::thread::spawn(move || {
        writer.write_all(&bytes).expect("对端写出");
        writer
            .shutdown(std::net::Shutdown::Write)
            .expect("对端半关闭");
        let mut sink = [0u8; 8];
        while let Ok(n) = writer.read(&mut sink) {
            if n == 0 {
                break;
            }
        }
    });
    (reader, handle)
}

/// 对端读到 EOF 为止：返回实际收到的全部字节。
fn peer_reads() -> (UnixStream, JoinHandle<Vec<u8>>) {
    let (writer, mut reader) = UnixStream::pair().expect("socketpair");
    let handle = std::thread::spawn(move || {
        let mut got = Vec::new();
        reader.read_to_end(&mut got).expect("对端读满");
        got
    });
    (writer, handle)
}

/// 可确定复现的写目标：接受前 `accept` 字节，其后报 BrokenPipe；记录每次请求的块长。
#[derive(Default)]
struct PartialSink {
    accept: usize,
    written: usize,
    calls: usize,
    max_offered: usize,
}

impl FrameSink for PartialSink {
    fn write_some(&mut self, chunk: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        self.max_offered = self.max_offered.max(chunk.len());
        if self.written >= self.accept {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "对端已断开"));
        }
        let n = (self.accept - self.written).min(chunk.len());
        self.written += n;
        Ok(n)
    }
}

/// 一次注定失败的读：返回（实际入账字节，错误文本）。
fn read_error_case(
    bytes: &[u8],
    max: usize,
    validate: fn(J) -> Result<J, String>,
) -> (u128, String) {
    let mut job = job_with(limits());
    let (mut receiver, peer) = peer_writes(bytes.to_vec());
    let mut attempt = WireAttempt::begin(&mut job).expect("request");
    let err = attempt
        .read_frame(
            &mut receiver,
            max,
            Instant::now() + Duration::from_secs(3),
            validate,
        )
        .expect_err("必须拒绝");
    let wire = attempt.job().snapshot().wire_bytes;
    assert_eq!(
        attempt.job().snapshot().stopped,
        None,
        "非资源错误不停止 job"
    );
    drop(attempt);
    drop(receiver);
    peer.join().expect("对端退出");
    (wire, err)
}

#[test]
fn real_unix_stream_read_and_write_accumulate_exact_bytes() {
    let mut job = job_with(limits());
    let request = format!("{}\n", canonical_json(&json!({"valid": true}))).into_bytes();
    let (mut receiver, peer) = peer_writes(request.clone());

    let mut attempt = WireAttempt::begin(&mut job).expect("记一次 request");
    assert_eq!(attempt.job().snapshot().requests, 1);
    assert_eq!(attempt.job().snapshot().wire_bytes, 0);

    let deadline = Instant::now() + Duration::from_secs(3);
    let value = attempt
        .read_frame(&mut receiver, 4096, deadline, value_only)
        .expect("真实读帧");
    assert_eq!(value, json!({"valid": true}));
    assert_eq!(
        attempt.job().snapshot().wire_bytes,
        request.len() as u128,
        "读按实际读到的 n 字节记账"
    );

    let reply = format!("{}\n", canonical_json(&json!({"ok": true}))).into_bytes();
    let (mut writer, reader) = peer_reads();
    attempt
        .write_frame(&mut writer, &reply, deadline)
        .expect("真实分块写");
    writer
        .shutdown(std::net::Shutdown::Write)
        .expect("写端半关闭");
    assert_eq!(reader.join().expect("对端读完"), reply);
    let snap = attempt.job().snapshot();
    assert_eq!(
        snap.wire_bytes,
        (request.len() + reply.len()) as u128,
        "同一 job 上读与写进同一本 wire 账"
    );
    assert_eq!(
        snap.requests, 1,
        "同一 attempt 的 read/write 不重复记 request"
    );

    drop(attempt);
    drop(receiver);
    peer.join().expect("对端退出");
}

#[test]
fn retry_counts_request_and_bytes_in_full() {
    let mut job = job_with(limits_with(|l| {
        l.max_total_requests = 2;
        l.max_total_wire_bytes = 4096;
    }));
    let body = b"retry-body\n".to_vec();
    let deadline = Instant::now() + Duration::from_secs(3);

    for round in 1..=2u64 {
        let mut attempt = WireAttempt::begin(&mut job).expect("每次 attempt 记一次 request");
        assert_eq!(attempt.job().snapshot().requests, round);
        let (mut writer, reader) = peer_reads();
        attempt
            .write_frame(&mut writer, &body, deadline)
            .expect("真实写");
        writer
            .shutdown(std::net::Shutdown::Write)
            .expect("写端半关闭");
        assert_eq!(reader.join().expect("对端读完"), body);
        assert_eq!(
            attempt.job().snapshot().wire_bytes,
            body.len() as u128 * u128::from(round),
            "重试全额再计，不按根去重"
        );
        drop(attempt);
    }

    // 第 3 次 begin 在 request 上限处拒绝：任何 I/O 之前。
    assert_eq!(
        WireAttempt::begin(&mut job).unwrap_err().code(),
        LIMIT_EXCEEDED
    );
    let snap = job.snapshot();
    assert_eq!(snap.requests, 2, "被拒绝的 attempt 不记 request");
    assert_eq!(snap.wire_bytes, body.len() as u128 * 2);
    assert_eq!(snap.stopped, Some(LIMIT_EXCEEDED));
}

#[test]
fn frame_errors_keep_every_received_byte() {
    // 坏 JSON：整个帧都已实际读到才在解码处失败。
    let bad_json = b"{oops}\n".to_vec();
    let (wire, err) = read_error_case(&bad_json, 4096, value_only);
    assert!(err.contains("SchemaUnsupported"), "{err}");
    assert_eq!(wire, bad_json.len() as u128);

    // validate 拒绝：字节已收齐、已记账，拒绝发生在解析之后。
    let valid = format!("{}\n", canonical_json(&json!({"valid": true}))).into_bytes();
    let (wire, err) = read_error_case(&valid, 4096, reject_all);
    assert_eq!(err, "SchemaUnsupported：业务拒绝", "校验器的拒绝原样上抛");
    assert_eq!(wire, valid.len() as u128);

    // 超过单帧上限：触发上限的那一块实收字节也要入账（不是按预计长度）。
    let mut over = vec![b'x'; 64];
    over.push(b'\n');
    let (wire, err) = read_error_case(&over, 64, value_only);
    assert!(err.contains("超过已声明界限"), "{err}");
    assert_eq!(wire, over.len() as u128, "实收的每一块都记账");
}

#[test]
fn prefixes_before_eof_and_timeout_are_kept() {
    let prefix = vec![b'{'; 400]; // 无 LF 的半截帧

    // EOF：前缀已读入，错误在 EOF 处才报。
    let mut job = job_with(limits());
    let (mut receiver, peer) = peer_writes(prefix.clone());
    let mut attempt = WireAttempt::begin(&mut job).expect("request");
    let err = attempt
        .read_frame(
            &mut receiver,
            65_536,
            Instant::now() + Duration::from_secs(3),
            value_only,
        )
        .expect_err("半截帧必须拒绝");
    assert!(err.contains("只接收一个LF终止帧"), "{err}");
    assert_eq!(attempt.job().snapshot().wire_bytes, prefix.len() as u128);
    drop(attempt);
    drop(receiver);
    peer.join().expect("对端退出");

    // 超时：对端保持连接，前缀在期限前已收到，仍要全额记账。
    let mut job = job_with(limits());
    let (mut receiver, mut sender) = UnixStream::pair().expect("socketpair");
    sender.write_all(&prefix).expect("对端写出前缀");
    let mut attempt = WireAttempt::begin(&mut job).expect("request");
    let started = Instant::now();
    let err = attempt
        .read_frame(
            &mut receiver,
            65_536,
            started + Duration::from_millis(150),
            value_only,
        )
        .expect_err("无 EOF 的帧到期必须拒绝");
    assert!(err.contains("TransportTimeout"), "{err}");
    assert!(
        started.elapsed() >= Duration::from_millis(80),
        "确实等到了期限"
    );
    assert_eq!(attempt.job().snapshot().wire_bytes, prefix.len() as u128);
    assert_eq!(
        attempt.job().snapshot().stopped,
        None,
        "旧每操作期限过期不等同 job 绝对期限过期"
    );
}

#[test]
fn chunked_write_keeps_sent_prefix_and_bounds_each_write() {
    let mut job = job_with(limits());
    let body = vec![b'x'; 500];
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut attempt = WireAttempt::begin(&mut job).expect("request");

    let mut sink = PartialSink {
        accept: 100,
        ..PartialSink::default()
    };
    let err = attempt
        .write_frame(&mut sink, &body, deadline)
        .expect_err("部分写后失败");
    assert!(err.contains("TransportUnavailable"), "{err}");
    assert_eq!(sink.written, 100, "对端只收了 100 字节");
    assert_eq!(
        attempt.job().snapshot().wire_bytes,
        100,
        "已成功写出的字节照实入账，不按 body 预计长度"
    );
    assert_eq!(sink.calls, 2, "第一块部分成功后第二块才失败");
    assert!(
        sink.max_offered <= WRITE_CHUNK_BYTES,
        "每次真正写不把整个 body 交出去"
    );

    // 真实路径：20000 字节分块写完整跑通，账 = 实际写出的全部字节。
    let big = vec![b'y'; 20_000];
    let (mut writer, reader) = peer_reads();
    attempt
        .write_frame(&mut writer, &big, deadline)
        .expect("真实分块写");
    writer
        .shutdown(std::net::Shutdown::Write)
        .expect("写端半关闭");
    assert_eq!(reader.join().expect("对端读完"), big);
    assert_eq!(attempt.job().snapshot().wire_bytes, 100 + big.len() as u128);

    // 分块上界：注入目标看到的最大块长不超过固定上界。
    let mut sink = PartialSink {
        accept: usize::MAX,
        ..PartialSink::default()
    };
    attempt
        .write_frame(&mut sink, &big, deadline)
        .expect("注入写");
    assert!(sink.max_offered <= WRITE_CHUNK_BYTES);
    assert!(
        sink.calls >= big.len() / WRITE_CHUNK_BYTES,
        "上界之外的 body 必须分成多块"
    );
}

#[test]
fn job_deadline_is_absolute_across_attempts_and_never_renewed() {
    let (ms, base, clock) = mock_clock();
    let mut job = Job::new(
        limits_with(|l| {
            l.max_elapsed_ms = 1_000;
        }),
        existing(),
        clock,
    )
    .expect("job 建立");

    let first = {
        let attempt = WireAttempt::begin(&mut job).expect("首次 attempt");
        assert_eq!(attempt.job().snapshot().requests, 1);
        attempt.deadline()
    };
    assert_eq!(first, base + Duration::from_millis(1_000));

    ms.store(400, Ordering::SeqCst);
    let second = {
        let attempt = WireAttempt::begin(&mut job).expect("重试 attempt");
        attempt.deadline()
    };
    assert_eq!(second, first, "同一 job 的绝对期限不因新 attempt 续期");
    assert_eq!(job.snapshot().requests, 2);

    // 期限走到：新 attempt 在 I/O 之前拒绝，且不记 request。
    ms.store(1_000, Ordering::SeqCst);
    assert_eq!(WireAttempt::begin(&mut job).unwrap_err().code(), DEADLINE);
    let snap = job.snapshot();
    assert_eq!(snap.requests, 2, "过期的那次 attempt 不算已发起");
    assert_eq!(snap.stopped, Some(DEADLINE));

    // 已开 attempt 在期限走过之后：真正 I/O 之前拒绝，一次写都没发生。
    let (ms, base, clock) = mock_clock();
    let mut job = Job::new(
        limits_with(|l| {
            l.max_elapsed_ms = 1_000;
        }),
        existing(),
        clock,
    )
    .expect("job 建立");
    let mut attempt = WireAttempt::begin(&mut job).expect("attempt");
    ms.store(1_000, Ordering::SeqCst);
    let mut sink = PartialSink {
        accept: usize::MAX,
        ..PartialSink::default()
    };
    let err = attempt
        .write_frame(&mut sink, b"hello", base + Duration::from_millis(60_000))
        .expect_err("期限已过必须拒绝");
    assert!(err.contains(DEADLINE), "{err}");
    assert_eq!(sink.calls, 0, "I/O 前拒绝：没有发生任何一次真正写");
    assert_eq!(attempt.job().snapshot().wire_bytes, 0);
    assert_eq!(attempt.job().snapshot().stopped, Some(DEADLINE));

    // 旧每操作期限与 job 绝对期限取更早者。
    let (_ms, base, clock) = mock_clock();
    let mut job = Job::new(
        limits_with(|l| {
            l.max_elapsed_ms = 5_000;
        }),
        existing(),
        clock,
    )
    .expect("job 建立");
    let attempt = WireAttempt::begin(&mut job).expect("attempt");
    let earlier = base + Duration::from_millis(500);
    let later = base + Duration::from_millis(9_000);
    assert_eq!(
        attempt.effective_deadline(earlier),
        earlier,
        "旧每操作期限更早时用它"
    );
    assert_eq!(
        attempt.effective_deadline(later),
        attempt.deadline(),
        "job 绝对期限更早时用它"
    );
}

#[test]
fn stopped_job_refuses_new_attempt_and_new_io() {
    let mut job = job_with(limits_with(|l| l.max_total_wire_bytes = 10));
    let mut attempt = WireAttempt::begin(&mut job).expect("request");
    let mut sink = PartialSink {
        accept: usize::MAX,
        ..PartialSink::default()
    };
    let body = vec![b'x'; 11];
    let err = attempt
        .write_frame(&mut sink, &body, Instant::now() + Duration::from_secs(3))
        .expect_err("wire 超限必须先入账再停止");
    assert!(err.contains(LIMIT_EXCEEDED), "{err}");
    let snap = attempt.job().snapshot();
    assert_eq!(
        snap.wire_bytes, 11,
        "越限的尾字节照实入账（旧预期「不留部分计费」已按 §5.6.1 修正）"
    );
    assert_eq!(snap.stopped, Some(LIMIT_EXCEEDED));
    let calls = sink.calls;

    // 同一 attempt 在停止后再读：I/O 前拒绝，对端已写的帧一个字节都没被消费。
    let frame = b"{\"valid\":true}\n".to_vec();
    let (mut receiver, mut sender) = UnixStream::pair().expect("socketpair");
    sender.write_all(&frame).expect("对端写出整帧");
    let err = attempt
        .read_frame(
            &mut receiver,
            4096,
            Instant::now() + Duration::from_secs(3),
            value_only,
        )
        .expect_err("停止后不得再读");
    assert!(err.contains(LIMIT_EXCEEDED), "{err}");
    receiver.set_nonblocking(true).expect("非阻塞");
    let mut got = [0u8; 32];
    let n = receiver.read(&mut got).expect("帧仍在 socket 缓冲里");
    assert_eq!(&got[..n], frame.as_slice(), "停止没有消费对端字节");
    drop(attempt);

    // 新 attempt（重试）在 request 记账前就被停止拒绝：不做任何新 I/O。
    assert_eq!(
        WireAttempt::begin(&mut job).unwrap_err().code(),
        LIMIT_EXCEEDED
    );
    assert_eq!(sink.calls, calls, "停止后没有再发生真正写");
    let snap = job.snapshot();
    assert_eq!(snap.wire_bytes, 11);
    assert_eq!(snap.requests, 1, "停止后的 attempt 不记 request");
}

#[test]
fn request_exhaustion_refuses_before_any_io() {
    let mut job = job_with(limits_with(|l| l.max_total_requests = 1));
    let mut attempt = WireAttempt::begin(&mut job).expect("唯一一次 request");
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut sink = PartialSink {
        accept: usize::MAX,
        ..PartialSink::default()
    };
    attempt
        .write_frame(&mut sink, b"one\n", deadline)
        .expect("首次写");
    let calls = sink.calls;
    assert_eq!(attempt.job().snapshot().requests, 1);
    drop(attempt);

    // 新 attempt 耗尽：在任何真正写之前拒绝。
    let err = WireAttempt::begin(&mut job).expect_err("request 耗尽");
    assert_eq!(err.code(), LIMIT_EXCEEDED);
    assert_eq!(sink.calls, calls, "拒绝发生在 I/O 之前");
    let snap = job.snapshot();
    assert_eq!(snap.requests, 1);
    assert_eq!(snap.wire_bytes, 4);
    assert_eq!(snap.stopped, Some(LIMIT_EXCEEDED));
    assert_eq!(
        WireAttempt::begin(&mut job).unwrap_err().code(),
        LIMIT_EXCEEDED,
        "停止后的重复尝试同样在 I/O 前拒绝"
    );
}
