//! #1394：财务模块内部的有界输送。响应未知只能查原身份，不铸造新许可。
use super::financial_reader_resources::FinancialReaderResources;
use super::store::{Config, SCHEMA};
use crate::s_content::job::Job;
use crate::session_protocol::{canonical_json, positive, reply_envelope, required, strict_json};
use serde_json::Value;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) fn call(
    config: &Config,
    domain: &str,
    epoch: &str,
    request: Value,
) -> Result<Value, String> {
    let route = if domain == "E" {
        &config.manifest["e"]
    } else {
        config.route(
            domain
                .strip_prefix("B:")
                .ok_or("InvalidDomain：传输目标域无效")?,
        )?
    };
    let r = &config.manifest["resources"];
    let limit = positive(required(r, "max_frame_bytes")?, "frame")? as usize;
    let deadline = Instant::now()
        + Duration::from_millis(positive(required(r, "response_timeout_ms")?, "timeout")? as u64);
    let s = connect(required(route, "socket")?, deadline)?;
    round_trip(config, domain, epoch, request, s, limit, deadline)
}
fn round_trip(
    config: &Config,
    domain: &str,
    epoch: &str,
    request: Value,
    mut s: UnixStream,
    limit: usize,
    deadline: Instant,
) -> Result<Value, String> {
    let mut bytes = canonical_json(&request).into_bytes();
    bytes.push(b'\n');
    if bytes.len() > limit {
        return Err("InvalidDomain：内部财务帧超过事前资源界限".into());
    }
    let left = || {
        deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or("DeliveryUnknown：财务输送达到绝对期限；查询原身份".to_owned())
    };
    while !bytes.is_empty() {
        s.set_write_timeout(Some(left()?))
            .map_err(|e| e.to_string())?;
        let n = s
            .write(&bytes)
            .map_err(|e| format!("DeliveryUnknown：写失败：{e}"))?;
        if n == 0 {
            return Err("DeliveryUnknown：写端提前关闭".into());
        }
        bytes.drain(..n);
    }
    s.shutdown(std::net::Shutdown::Write)
        .map_err(|e| format!("DeliveryUnknown：请求 EOF 失败：{e}"))?;
    fn validate(v: Value) -> Result<Value, String> {
        // 回执用 kind；下面与原请求生成的完整回执逐字段相等，不能套请求的 payload.op 判据。
        if !v.is_object() || !v["payload"].is_object() {
            return Err("SchemaUnsupported：回执及 payload 须对象".into());
        }
        required(&v["payload"], "kind")?;
        Ok(v)
    }
    let reply = crate::session_transport::read_frame(&mut s, limit, deadline, validate)
        .map_err(|e| format!("DeliveryUnknown：{e}；按原身份查询"))?;
    let expected = reply_envelope(
        &request,
        reply["payload"].clone(),
        SCHEMA,
        &config.session,
        &config.generation,
        "economic-session/replies",
        &format!("{domain}:{}", config.session),
        epoch,
    );
    if reply != expected {
        return Err("IdentityConflict：财务回执身份/epoch/因果/规范字节不符".into());
    }
    let p = reply["payload"].clone();
    if p["kind"] == "Error" {
        return Err(required(&p, "error")?.into());
    }
    Ok(p)
}
/// AF_UNIX connect 同样计入绝对期限，满 backlog 不得挂死健康重的后续处理。
fn connect(path: &str, deadline: Instant) -> Result<UnixStream, String> {
    use std::os::fd::{AsRawFd, FromRawFd};
    let bytes = path.as_bytes();
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if bytes.len() >= address.sun_path.len() || bytes.contains(&0) {
        return Err("InvalidDomain：Unix 地址长度/内容非法".into());
    }
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (a, b) in address.sun_path.iter_mut().zip(bytes) {
        *a = *b as libc::c_char;
    }
    let length = std::mem::offset_of!(libc::sockaddr_un, sun_path) + bytes.len() + 1;
    #[cfg(target_os = "macos")]
    {
        address.sun_len = length as u8;
    }
    // fd 立即交给 UnixStream；所有错误路径由 RAII 关闭本次连接。
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(format!(
            "DeliveryUnknown：创建 Unix 连接失败：{}",
            std::io::Error::last_os_error()
        ));
    }
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    stream
        .set_nonblocking(true)
        .map_err(|e| format!("DeliveryUnknown：{e}"))?;
    let result = unsafe {
        libc::connect(
            stream.as_raw_fd(),
            (&address as *const libc::sockaddr_un).cast(),
            length as libc::socklen_t,
        )
    };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINPROGRESS) {
            return Err(format!("DeliveryUnknown：Unix connect：{error}"));
        }
        loop {
            let left = deadline
                .checked_duration_since(Instant::now())
                .ok_or("DeliveryUnknown：Unix connect 超时")?;
            let ms = left.as_millis().saturating_add(1).min(i32::MAX as u128) as i32;
            let mut poll = libc::pollfd {
                fd: stream.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            };
            let n = unsafe { libc::poll(&mut poll, 1, ms) };
            if n < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            if n <= 0 {
                return Err("DeliveryUnknown：Unix connect 达到期限/轮询失败".into());
            }
            let mut code: libc::c_int = 0;
            let mut len = std::mem::size_of_val(&code) as libc::socklen_t;
            let r = unsafe {
                libc::getsockopt(
                    stream.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_ERROR,
                    (&mut code as *mut libc::c_int).cast(),
                    &mut len,
                )
            };
            if r != 0 || code != 0 {
                return Err(format!("DeliveryUnknown：Unix connect socket error {code}"));
            }
            break;
        }
    }
    stream
        .set_nonblocking(false)
        .map_err(|e| format!("DeliveryUnknown：{e}"))?;
    Ok(stream)
}
/// 不访问 shell，不继承任意命令文本。输出有界，子进程超时真实停止并 wait。
pub(super) fn reader(binary: &str, args: &[&str], limit: usize) -> Result<Value, String> {
    let mut child = Command::new(binary)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("MissingDependency：S 正式证据入口无法启动：{e}"))?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut b = vec![];
        stdout
            .take((limit + 1) as u64)
            .read_to_end(&mut b)
            .map(|_| b)
    });
    let error = std::thread::spawn(move || {
        let mut b = vec![];
        stderr.take(65537).read_to_end(&mut b).map(|_| b)
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Ok(s),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break Err("MissingDependency：S 只读证据查询超时/进程错误".to_owned());
            }
        }
    };
    let b = out
        .join()
        .map_err(|_| "StorageUnavailable：S 输出读取线程失败")?
        .map_err(|e| e.to_string())?;
    let e = error
        .join()
        .map_err(|_| "StorageUnavailable：S 错误读取线程失败")?
        .map_err(|e| e.to_string())?;
    if !status?.success() {
        return Err(format!(
            "MissingDependency：S 正式核验失败：{}",
            String::from_utf8_lossy(&e)
        ));
    }
    if b.len() > limit {
        return Err("MissingDependency：S 完整证据超过事前界限".into());
    }
    strict_json(&b)
}

/// 新财务内容 reader：一次调用就是同一经济 job 的一次真实请求。
/// `operation_deadline` 由经济 owner 提供；三种绝对期限取最早者。
/// 本口只负责运输，不授予 S 内容根权限，也不改变旧整份 reader。
#[allow(dead_code)] // 待 tb03b-finance/2 consumer 接线
pub(super) fn financial_reader(
    binary: &str,
    args: &[&str],
    request: &Value,
    resources: &FinancialReaderResources,
    job: &mut Job,
    operation_deadline: Instant,
) -> Result<Value, String> {
    resources.validate()?;
    if job.snapshot().limits != resources.job_limits {
        return Err("IdentityConflict：财务 reader job 限额与冻结资源不符".into());
    }
    let bytes = canonical_json(request).into_bytes();
    strict_json(&bytes).map_err(|_| "SchemaUnsupported：财务 reader 请求非严格JSON")?;
    // 被拒的尝试也消耗 request 次数；只有真实写入/读到的字节才计 wire。
    job.charge_requests(1).map_err(|e| e.to_string())?;
    if bytes.len() as u128 > u128::from(resources.max_request_bytes) {
        return Err("Incomplete：财务 reader 请求超过冻结字节上限".into());
    }
    let started = Instant::now();
    let per_request = started
        .checked_add(Duration::from_millis(resources.request_timeout_ms))
        .ok_or("InvalidDomain：财务 reader 单次期限溢出")?;
    let deadline = per_request
        .min(operation_deadline)
        .min(job.first_read_deadline().map_err(|e| e.to_string())?);
    if Instant::now() >= deadline {
        return Err("Incomplete：财务 reader 绝对期限已过".into());
    }
    let mut child = Command::new(binary)
        .args(args)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "MissingDependency：财务 reader 无法启动")?;
    let outcome = financial_reader_io(&mut child, &bytes, resources, job, deadline);
    if outcome.is_err() {
        let _ = child.kill();
    }
    // 所有结果路径都回收本次启动的 child；不保留可能阻塞的 pipe 读取线程。
    let status = child
        .wait()
        .map_err(|_| "MissingDependency：财务 reader 无法回收")?;
    let reply = outcome?;
    if Instant::now() >= deadline {
        return Err("Incomplete：财务 reader 在回收前达到绝对期限".into());
    }
    if !status.success() {
        return Err("MissingDependency：财务 reader 非成功退出".into());
    }
    let parsed =
        strict_json(&reply).map_err(|_| "SchemaUnsupported：财务 reader 回复非严格JSON")?;
    job.pre_io_check().map_err(|e| e.to_string())?;
    Ok(parsed)
}

fn financial_reader_io(
    child: &mut std::process::Child,
    request: &[u8],
    resources: &FinancialReaderResources,
    job: &mut Job,
    deadline: Instant,
) -> Result<Vec<u8>, String> {
    let mut stdin = child.stdin.take();
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    for fd in [
        stdin.as_ref().map(AsRawFd::as_raw_fd),
        stdout.as_ref().map(AsRawFd::as_raw_fd),
        stderr.as_ref().map(AsRawFd::as_raw_fd),
    ]
    .into_iter()
    .flatten()
    {
        nonblocking(fd)?;
    }
    let mut written = 0;
    let mut reply = Vec::new();
    let mut stderr_bytes = 0u128;
    loop {
        job.pre_io_check().map_err(|e| e.to_string())?;
        let left = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or("Incomplete：财务 reader 达到绝对期限")?;
        if written == request.len() {
            stdin.take(); // EOF 是单份严格 JSON 请求的边界。
        }
        if stdin.is_none() && stdout.is_none() && stderr.is_none() {
            if child
                .try_wait()
                .map_err(|_| "MissingDependency：财务 reader 状态不可读")?
                .is_some()
            {
                return Ok(reply);
            }
        }
        let mut fds = Vec::with_capacity(3);
        let mut kinds = Vec::with_capacity(3);
        if let Some(pipe) = stdin.as_ref() {
            fds.push(libc::pollfd {
                fd: pipe.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            });
            kinds.push(0);
        }
        if let Some(pipe) = stdout.as_ref() {
            fds.push(libc::pollfd {
                fd: pipe.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            });
            kinds.push(1);
        }
        if let Some(pipe) = stderr.as_ref() {
            fds.push(libc::pollfd {
                fd: pipe.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            });
            kinds.push(2);
        }
        let timeout = left.as_millis().min(20).max(1) as i32;
        let n = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, timeout) };
        if n < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err("MissingDependency：财务 reader 管道轮询失败".into());
        }
        for (fd, kind) in fds.iter().zip(kinds) {
            if fd.revents == 0 {
                continue;
            }
            match kind {
                0 => {
                    job.pre_io_check().map_err(|e| e.to_string())?;
                    let pipe = stdin.as_mut().expect("已注册stdin");
                    match pipe.write(&request[written..]) {
                        Ok(0) => return Err("MissingDependency：财务 reader 提前关闭stdin".into()),
                        Ok(n) => {
                            written += n;
                            job.charge_wire_bytes(n as u64).map_err(|e| e.to_string())?;
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
                        Err(_) => return Err("MissingDependency：财务 reader 请求写入失败".into()),
                    }
                }
                1 => {
                    job.pre_io_check().map_err(|e| e.to_string())?;
                    let mut chunk = [0u8; 4096];
                    match stdout.as_mut().expect("已注册stdout").read(&mut chunk) {
                        Ok(0) => {
                            stdout.take();
                        }
                        Ok(n) => {
                            job.charge_wire_bytes(n as u64).map_err(|e| e.to_string())?;
                            if reply.len() as u128 + n as u128
                                > u128::from(resources.max_reply_bytes)
                            {
                                return Err("Incomplete：财务 reader 回复超过冻结字节上限".into());
                            }
                            reply.extend_from_slice(&chunk[..n]);
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
                        Err(_) => return Err("MissingDependency：财务 reader 回复读取失败".into()),
                    }
                }
                _ => {
                    job.pre_io_check().map_err(|e| e.to_string())?;
                    let mut chunk = [0u8; 4096];
                    match stderr.as_mut().expect("已注册stderr").read(&mut chunk) {
                        Ok(0) => {
                            stderr.take();
                        }
                        Ok(n) => {
                            job.charge_wire_bytes(n as u64).map_err(|e| e.to_string())?;
                            stderr_bytes += n as u128;
                            if stderr_bytes > u128::from(resources.max_stderr_bytes) {
                                return Err(
                                    "Incomplete：财务 reader stderr 超过冻结字节上限".into()
                                );
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => (),
                        Err(_) => {
                            return Err("MissingDependency：财务 reader stderr 读取失败".into())
                        }
                    }
                }
            }
        }
    }
}

fn nonblocking(fd: std::os::fd::RawFd) -> Result<(), String> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err("MissingDependency：财务 reader 无法设置有界管道".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_protocol::validate_envelope;
    use serde_json::json;
    #[test]
    fn internal_transport_requires_real_eof_and_binds_original_reply() {
        for valid in [true, false] {
            let (client, mut server) = UnixStream::pair().unwrap();
            let payload =
                json!({"op":"QueryDecision","decision_id":"alpha","cut":null,"as_known_ns":null});
            let request = json!({"schema_revision":SCHEMA,"session_id":"transport-unit","session_generation":"1","source_namespace":"test","source_epoch":"1","producer_id":"test","producer_epoch":"1","message_id":"query","payload_hash":crate::session_protocol::sha256_hex(canonical_json(&payload).as_bytes()),"causal_refs":[],"payload":payload});
            let expected_request = request.clone();
            let task = std::thread::spawn(move || {
                fn validate(v: Value) -> Result<Value, String> {
                    validate_envelope(v, SCHEMA)
                }
                let request = crate::session_transport::read_frame(
                    &mut server,
                    16384,
                    Instant::now() + Duration::from_secs(2),
                    validate,
                )
                .unwrap();
                assert_eq!(request, expected_request);
                let reply = reply_envelope(
                    &request,
                    json!({"kind":"EconomicDecision","domain_id":"B:alpha"}),
                    SCHEMA,
                    "transport-unit",
                    "1",
                    "economic-session/replies",
                    if valid {
                        "B:alpha:transport-unit"
                    } else {
                        "B:wrong:transport-unit"
                    },
                    "1",
                );
                server
                    .write_all((canonical_json(&reply) + "\n").as_bytes())
                    .unwrap();
                server.shutdown(std::net::Shutdown::Write).unwrap();
            });
            let config = Config {
                manifest: json!({}),
                manifest_hash: String::new(),
                db: Default::default(),
                domain: "E".into(),
                session: "transport-unit".into(),
                generation: "1".into(),
                producer: "E".into(),
                socket: Default::default(),
            };
            let result = round_trip(
                &config,
                "B:alpha",
                "1",
                request,
                client,
                16384,
                Instant::now() + Duration::from_secs(2),
            );
            if valid {
                assert_eq!(result.unwrap()["kind"], "EconomicDecision");
            } else {
                assert!(result.unwrap_err().starts_with("IdentityConflict"));
            }
            task.join().unwrap();
        }
    }
}

#[cfg(test)]
mod financial_reader_tests {
    use super::*;
    use crate::s_content::job::{Existing, JobLimits};
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct Script {
        dir: PathBuf,
        path: PathBuf,
    }
    impl Script {
        fn new(body: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "financial-reader-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&dir).unwrap();
            let path = dir.join("reader.sh");
            std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            Self { dir, path }
        }
        fn binary(&self) -> &str {
            self.path.to_str().unwrap()
        }
    }
    impl Drop for Script {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn resources() -> FinancialReaderResources {
        let json = json!({
            "schema_revision":"s-financial-reader-resources/1",
            "max_request_bytes":"4096", "max_reply_bytes":"4096",
            "max_stderr_bytes":"4096", "request_timeout_ms":"2000",
            "job_limits":{
                "max_total_wire_bytes":"65536", "max_total_node_bytes":"65536",
                "max_total_nodes":"100", "max_total_records":"100",
                "max_total_requests":"10", "max_elapsed_ms":"5000",
                "max_staged_bytes":"65536", "max_store_bytes":"65536"
            }
        });
        FinancialReaderResources::from_json(&json).unwrap()
    }
    fn new_job(limits: JobLimits) -> Job {
        Job::new(
            limits,
            Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(Instant::now),
        )
        .unwrap()
    }
    fn request() -> Value {
        json!({"schema_revision":"s-financial-query/1","message":"凭证"})
    }

    #[test]
    fn strict_request_reply_and_two_attempts_share_real_wire_and_request_budget() {
        let script = Script::new("exec /bin/cat");
        let request = request();
        let bytes = canonical_json(&request).len() as u128;
        let r = resources();
        let mut job = new_job(r.job_limits);
        for _ in 0..2 {
            assert_eq!(
                financial_reader(
                    script.binary(),
                    &[],
                    &request,
                    &r,
                    &mut job,
                    Instant::now() + Duration::from_secs(2)
                )
                .unwrap(),
                request
            );
        }
        let used = job.snapshot();
        assert_eq!(used.requests, 2);
        assert_eq!(used.wire_bytes, bytes * 4);
    }

    #[test]
    fn oversize_request_is_rejected_before_spawn_but_attempt_is_counted() {
        let script = Script::new("printf spawned > \"$1\"; exec /bin/cat");
        let marker = script.dir.join("spawned");
        let mut r = resources();
        r.max_request_bytes = 1;
        let mut job = new_job(r.job_limits);
        let error = financial_reader(
            script.binary(),
            &[marker.to_str().unwrap()],
            &request(),
            &r,
            &mut job,
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap_err();
        assert!(error.contains("请求超过"));
        assert!(!marker.exists());
        assert_eq!(job.snapshot().requests, 1);
        assert_eq!(job.snapshot().wire_bytes, 0);
    }

    #[test]
    fn reader_rejects_a_job_with_wider_limits_before_spawn() {
        let script = Script::new("printf spawned > \"$1\"; exec /bin/cat");
        let marker = script.dir.join("spawned");
        let r = resources();
        let mut wider = r.job_limits;
        wider.max_total_wire_bytes += 1;
        let mut job = new_job(wider);
        let error = financial_reader(
            script.binary(),
            &[marker.to_str().unwrap()],
            &request(),
            &r,
            &mut job,
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap_err();
        assert!(error.contains("限额与冻结资源不符"));
        assert!(!marker.exists());
        assert_eq!(job.snapshot().requests, 0);
    }

    #[test]
    fn reply_and_stderr_caps_stop_running_child_without_disclosing_stderr() {
        let large = Script::new("/bin/cat >/dev/null; i=0; while [ \"$i\" -lt 1000 ]; do printf x; i=$((i+1)); done; exec /bin/sleep 5");
        let mut r = resources();
        r.max_reply_bytes = 10;
        let mut job = new_job(r.job_limits);
        let started = Instant::now();
        assert!(financial_reader(
            large.binary(),
            &[],
            &request(),
            &r,
            &mut job,
            started + Duration::from_secs(2)
        )
        .unwrap_err()
        .contains("回复超过"));
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(job.snapshot().wire_bytes > canonical_json(&request()).len() as u128);

        let noisy = Script::new("printf 'PRIVATE_SECRET_123456789' >&2; exec /bin/sleep 5");
        let mut r = resources();
        r.max_stderr_bytes = 5;
        let mut job = new_job(r.job_limits);
        let error = financial_reader(
            noisy.binary(),
            &[],
            &request(),
            &r,
            &mut job,
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap_err();
        assert!(error.contains("stderr 超过"));
        assert!(!error.contains("PRIVATE_SECRET"));
        assert!(job.snapshot().wire_bytes > canonical_json(&request()).len() as u128);
    }

    #[test]
    fn earliest_absolute_deadline_kills_and_reaps_the_child() {
        let script = Script::new("printf '%s' \"$$\" > \"$1\"; exec /bin/sleep 5");
        let pid_file = script.dir.join("pid");
        let mut r = resources();
        r.request_timeout_ms = 1200;
        let mut job = new_job(r.job_limits);
        let start = Instant::now();
        let error = financial_reader(
            script.binary(),
            &[pid_file.to_str().unwrap()],
            &request(),
            &r,
            &mut job,
            start + Duration::from_millis(500),
        )
        .unwrap_err();
        assert!(error.contains("期限"));
        assert!(start.elapsed() < Duration::from_millis(900));
        let pid: i32 = std::fs::read_to_string(pid_file).unwrap().parse().unwrap();
        assert_eq!(unsafe { libc::kill(pid, 0) }, -1, "child须被回收");
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ESRCH)
        );
    }

    #[test]
    fn per_request_timeout_and_failed_retry_keep_same_job_account() {
        let script = Script::new("/bin/cat >/dev/null; printf private-error >&2; exit 7");
        let request = request();
        let mut r = resources();
        r.request_timeout_ms = 1000;
        let mut job = new_job(r.job_limits);
        for _ in 0..2 {
            let error = financial_reader(
                script.binary(),
                &[],
                &request,
                &r,
                &mut job,
                Instant::now() + Duration::from_secs(2),
            )
            .unwrap_err();
            assert!(error.contains("非成功退出"), "{error}");
            assert!(!error.contains("private-error"));
        }
        assert_eq!(job.snapshot().requests, 2);
        assert_eq!(
            job.snapshot().wire_bytes,
            (canonical_json(&request).len() + "private-error".len()) as u128 * 2
        );

        r.request_timeout_ms = 200;
        let slow = Script::new("exec /bin/sleep 5");
        let mut fresh = new_job(r.job_limits);
        let start = Instant::now();
        assert!(financial_reader(
            slow.binary(),
            &[],
            &request,
            &r,
            &mut fresh,
            start + Duration::from_secs(2)
        )
        .unwrap_err()
        .contains("期限"));
        assert!(start.elapsed() < Duration::from_millis(650));
    }

    #[test]
    fn resource_contract_rejects_extra_field_and_noncanonical_number() {
        let mut value = json!({
            "schema_revision":"s-financial-reader-resources/1",
            "max_request_bytes":"1", "max_reply_bytes":"1", "max_stderr_bytes":"1",
            "request_timeout_ms":"1", "job_limits":{
                "max_total_wire_bytes":"1", "max_total_node_bytes":"1", "max_total_nodes":"1",
                "max_total_records":"1", "max_total_requests":"1", "max_elapsed_ms":"1",
                "max_staged_bytes":"1", "max_store_bytes":"1" }
        });
        assert!(FinancialReaderResources::from_json(&value).is_ok());
        value["unexpected"] = json!("1");
        assert!(FinancialReaderResources::from_json(&value).is_err());
        value.as_object_mut().unwrap().remove("unexpected");
        value["max_reply_bytes"] = json!("01");
        assert!(FinancialReaderResources::from_json(&value).is_err());
        value["max_reply_bytes"] = json!(1);
        assert!(FinancialReaderResources::from_json(&value).is_err());
    }
}
