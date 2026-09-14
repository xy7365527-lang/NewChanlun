//! #1394：财务模块内部的有界输送。响应未知只能查原身份，不铸造新许可。
use super::store::{Config, SCHEMA};
use crate::session_protocol::{canonical_json, positive, reply_envelope, required, strict_json};
use serde_json::Value;
use std::io::{Read, Write};
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
