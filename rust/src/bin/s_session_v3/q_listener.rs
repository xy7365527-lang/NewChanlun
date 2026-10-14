//! #1394：有界 Q IPv4 监听与有序关闭。
//!
//! 监听层只负责接纳连接和把连接交给固定 worker；请求读取、资源门、响应和
//! 错误体仍由 `q_runtime` 负责。不会为每个连接另起线程，也不把
//! `poll_interval_ms`解释成监听循环的隐含期限。

use super::{
    q_executor::Pending,
    q_resources::{Limit, QResources},
    q_runtime::Runtime,
};
use newchan_rust::s_content::CodecVec;
use std::{
    net::{SocketAddr, TcpListener, ToSocketAddrs},
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd},
    sync::atomic::{AtomicBool, Ordering},
};

pub(super) struct Listener {
    listener: TcpListener,
}

impl Listener {
    pub(super) fn bind(host: &str, port: u16, resources: &QResources) -> Result<Self, String> {
        let address = (host, port)
            .to_socket_addrs()
            .map_err(|e| format!("Q listener地址解析失败: {e}"))?
            .find(|address| address.is_ipv4())
            .ok_or_else(|| "Q listener只接受IPv4地址".to_string())?;
        let SocketAddr::V4(address) = address else {
            unreachable!("IPv4地址已筛选");
        };
        let backlog = resources.limit(Limit::PendingConnections);
        let backlog = libc::c_int::try_from(backlog)
            .map_err(|_| "Q listener backlog超出本机整数范围".to_string())?;
        if backlog <= 0 {
            return Err("Q listener backlog必须为正".into());
        }

        // SAFETY：不传指针，非负返回值是本调用拥有的文件描述符。
        let raw = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
        if raw < 0 {
            return Err(last_os_error("创建Q listener socket失败"));
        }
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        set_cloexec(fd.as_raw_fd())?;
        set_nonblocking(fd.as_raw_fd())?;
        let reuse: libc::c_int = 1;
        // SAFETY：reuse在整个调用期有效，传入其实际大小。
        let reuse_result = unsafe {
            libc::setsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_REUSEADDR,
                (&reuse as *const libc::c_int).cast(),
                std::mem::size_of_val(&reuse) as libc::socklen_t,
            )
        };
        if reuse_result != 0 {
            return Err(last_os_error("设置Q listener SO_REUSEADDR失败"));
        }
        let mut sockaddr: libc::sockaddr_in = unsafe { std::mem::zeroed() };
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            sockaddr.sin_len = std::mem::size_of::<libc::sockaddr_in>() as u8;
        }
        sockaddr.sin_family = libc::AF_INET as libc::sa_family_t;
        sockaddr.sin_port = address.port().to_be();
        sockaddr.sin_addr = libc::in_addr {
            s_addr: u32::from_ne_bytes(address.ip().octets()),
        };
        // SAFETY：sockaddr已初始化，指针和长度与实际类型一致。
        let bind_result = unsafe {
            libc::bind(
                fd.as_raw_fd(),
                (&sockaddr as *const libc::sockaddr_in).cast(),
                std::mem::size_of_val(&sockaddr) as libc::socklen_t,
            )
        };
        if bind_result != 0 {
            return Err(last_os_error("绑定Q listener地址失败"));
        }
        if unsafe { libc::listen(fd.as_raw_fd(), backlog) } != 0 {
            return Err(last_os_error("设置Q listener backlog失败"));
        }
        // SAFETY：有效socket描述符的所有权只向TcpListener转移一次。
        let listener = unsafe { TcpListener::from_raw_fd(fd.into_raw_fd()) };
        Ok(Self { listener })
    }

    pub(super) fn local_addr(&self) -> Result<SocketAddr, String> {
        self.listener
            .local_addr()
            .map_err(|e| format!("读取Q listener地址失败: {e}"))
    }

    pub(super) fn run(self, runtime: Runtime, stop: &AtomicBool) -> Result<(), String> {
        self.run_with_seam(runtime, stop, |_| {})
    }

    fn run_with_seam<F>(
        self,
        runtime: Runtime,
        stop: &AtomicBool,
        mut event: F,
    ) -> Result<(), String>
    where
        F: FnMut(&'static str),
    {
        let listener_fd = self.listener.as_raw_fd();
        let mut pending = CodecVec::<Pending<()>>::new_with_allocation(runtime.allocation_budget());
        loop {
            reap_ready(&mut pending);
            if stop.load(Ordering::Acquire) {
                drop(self.listener);
                event("listener_closed");
                return finish(runtime, pending, None);
            }
            let mut pollfd = libc::pollfd {
                fd: listener_fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let result = unsafe { libc::poll(&mut pollfd, 1, 500) };
            if result < 0 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                drop(self.listener);
                event("listener_closed");
                return finish(
                    runtime,
                    pending,
                    Some(format!("Q listener poll失败: {error}")),
                );
            }
            if result == 0 {
                continue;
            }
            if pollfd.revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
                drop(self.listener);
                event("listener_closed");
                return finish(
                    runtime,
                    pending,
                    Some("Q listener poll返回不可用socket".into()),
                );
            }
            if pollfd.revents & (libc::POLLIN | libc::POLLHUP) == 0 {
                continue;
            }
            loop {
                reap_ready(&mut pending);
                if stop.load(Ordering::Acquire) {
                    drop(self.listener);
                    event("listener_closed");
                    return finish(runtime, pending, None);
                }
                match self.listener.accept() {
                    Ok((stream, _)) => {
                        event("accepted");
                        if let Err(error) = stream.set_nonblocking(false) {
                            eprintln!("Q listener关闭未配置连接: {error}");
                            continue;
                        }
                        // 在转交socket之前预留实际pending owner槽；拒绝仍持原socket。
                        if let Err(error) = pending.try_reserve(1) {
                            if let Err(reason) = runtime.reject_http(stream, error.to_string()) {
                                eprintln!("Q listener拒绝连接: {reason}");
                            }
                            continue;
                        }
                        match runtime.accept_http(stream) {
                            Ok(task) => {
                                if pending.try_push(task).is_err() {
                                    unreachable!("pre-reserved listener pending slot");
                                }
                            }
                            Err(error) => eprintln!("Q listener拒绝连接: {error}"),
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => {
                        drop(self.listener);
                        event("listener_closed");
                        return finish(
                            runtime,
                            pending,
                            Some(format!("Q listener accept失败: {error}")),
                        );
                    }
                }
            }
        }
    }
}

fn reap_ready(pending: &mut CodecVec<Pending<()>>) {
    let mut index = 0;
    while index < pending.len() {
        if let Some(result) = pending[index].try_result() {
            if let Err(error) = result {
                eprintln!("Q listener worker终态错误: {error}");
            }
            pending.swap_remove(index);
        } else {
            index += 1;
        }
    }
}

fn finish(
    runtime: Runtime,
    pending: CodecVec<Pending<()>>,
    error: Option<String>,
) -> Result<(), String> {
    drop(runtime);
    for mut task in pending {
        if let Some(result) = task.try_result() {
            if let Err(error) = result {
                eprintln!("Q listener worker终态错误: {error}");
            }
        } else if let Err(error) = task.recv() {
            eprintln!("Q listener worker终态错误: {error}");
        }
    }
    error.map_or(Ok(()), Err)
}

fn set_cloexec(fd: libc::c_int) -> Result<(), String> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    if flags < 0 {
        return Err(last_os_error("读取Q listener FD flags失败"));
    }
    if unsafe { libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) } < 0 {
        return Err(last_os_error("设置Q listener CLOEXEC失败"));
    }
    Ok(())
}

fn set_nonblocking(fd: libc::c_int) -> Result<(), String> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(last_os_error("读取Q listener socket flags失败"));
    }
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(last_os_error("设置Q listener非阻塞失败"));
    }
    Ok(())
}

fn last_os_error(context: &str) -> String {
    format!("{context}: {}", std::io::Error::last_os_error())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        io::{Read, Write},
        net::TcpStream,
        sync::{atomic::AtomicBool, mpsc, Arc},
        thread,
        time::Duration,
    };

    fn resources() -> QResources {
        let mut input = super::super::q_resources::tests::functional_input();
        input["limits"]["max_pending_connections"] = json!("3");
        QResources::parse(serde_json::to_vec(&input).unwrap().as_slice()).unwrap()
    }

    #[test]
    fn binds_ipv4_with_ephemeral_port_and_releases_on_drop() {
        let resources = resources();
        let listener = Listener::bind("127.0.0.1", 0, &resources).unwrap();
        let address = listener.local_addr().unwrap();
        assert!(address.ip().is_ipv4());
        assert_ne!(address.port(), 0);
        assert!(Listener::bind("127.0.0.1", address.port(), &resources).is_err());
        drop(listener);
        let rebound = Listener::bind("127.0.0.1", address.port(), &resources).unwrap();
        drop(rebound);
    }

    #[test]
    fn stop_closes_listener_then_drains_an_accepted_incomplete_request() {
        let fixture = super::super::prepared::tests::Fixture::new();
        let resources = Arc::new(resources());
        let runtime = Runtime::new(fixture.db_path(), Arc::clone(&resources), "1").unwrap();
        let listener = Listener::bind("127.0.0.1", 0, resources.as_ref()).unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let (events_tx, events_rx) = mpsc::channel();
        let stop_for_thread = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            listener.run_with_seam(runtime, &stop_for_thread, |event| {
                events_tx.send(event).unwrap();
            })
        });

        let mut first = TcpStream::connect(address).unwrap();
        first
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        first
            .write_all(b"POST /api/v3/snapshot HTTP/1.1\r\nContent-Length: 1\r\n\r\n")
            .unwrap();
        assert_eq!(
            events_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "accepted"
        );
        stop.store(true, Ordering::Release);
        assert_eq!(
            events_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "listener_closed"
        );
        assert!(TcpStream::connect(address).is_err());
        first.write_all(b"{").unwrap();
        let mut response = Vec::new();
        first.read_to_end(&mut response).unwrap();
        assert!(response.starts_with(b"HTTP/1.0 400 Bad Request\r\n"));
        assert!(thread.join().unwrap().is_ok());
    }
}
