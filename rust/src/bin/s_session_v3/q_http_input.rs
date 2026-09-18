//! #1394：Q HTTP 输入层的有界读。
//!
//! 这里复用旧只读 HTTP 合同：请求行、全部头和 body 共用一个绝对读期限与
//! `max_frame_bytes`；不接受 chunked，也不按 Content-Length 预分配。它只解析
//! 输入，不决定路由、响应 envelope 或服务端 Q 资源之外的消费者 Job 记账。

use super::q_resources::{Limit, QResources};
use std::{
    collections::VecDeque,
    io::{self, Read},
    time::{Duration, Instant},
};

const MAX_LINE: usize = 65_536;
// Python http.client 把终止空行也纳入 100 行保护，因此最多接受 99 个非空头行。
const MAX_HEADERS: usize = 100;
const READ_CHUNK: usize = 4096;

/// HTTP 输入失败；上层只需将 status/error/detail 组装成响应。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Failure {
    pub status: u16,
    pub error: &'static str,
    pub detail: String,
}

impl Failure {
    fn invalid(detail: impl Into<String>) -> Self {
        Self {
            status: 400,
            error: "InvalidQuery",
            detail: detail.into(),
        }
    }

    fn too_long_line(detail: impl Into<String>) -> Self {
        Self {
            status: 414,
            error: "InvalidQuery",
            detail: detail.into(),
        }
    }

    fn too_many_headers(detail: impl Into<String>) -> Self {
        Self {
            status: 431,
            error: "InvalidQuery",
            detail: detail.into(),
        }
    }

    fn budget(detail: impl Into<String>) -> Self {
        Self {
            status: 503,
            error: "QueryBudgetExceeded",
            detail: detail.into(),
        }
    }

    fn storage(detail: impl Into<String>) -> Self {
        Self {
            status: 503,
            error: "StorageUnavailable",
            detail: detail.into(),
        }
    }
}

/// 可由 TCP/Unix socket 或测试 reader 实现的读 timeout 接口。
pub(super) trait TimedRead: Read {
    fn set_read_timeout(&mut self, timeout: Duration) -> io::Result<()>;
}

#[cfg(unix)]
impl TimedRead for std::net::TcpStream {
    fn set_read_timeout(&mut self, timeout: Duration) -> io::Result<()> {
        std::net::TcpStream::set_read_timeout(self, Some(timeout))
    }
}

#[cfg(unix)]
impl TimedRead for std::os::unix::net::UnixStream {
    fn set_read_timeout(&mut self, timeout: Duration) -> io::Result<()> {
        std::os::unix::net::UnixStream::set_read_timeout(self, Some(timeout))
    }
}

/// 请求头的最小公开部分；调用方可在未知 method/path 时跳过 body。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Head {
    pub method: String,
    pub path: String,
    version: String,
    content_lengths: Vec<String>,
    transfer_encoding: bool,
}

pub(super) struct Input<'a, R: TimedRead> {
    reader: &'a mut R,
    deadline: Instant,
    maximum: u64,
    pending: VecDeque<u8>,
    total: u64,
    parsed_head: Option<Head>,
    body_read: bool,
}

impl<'a, R: TimedRead> Input<'a, R> {
    pub(super) fn new(reader: &'a mut R, resources: &QResources) -> Result<Self, Failure> {
        let now = Instant::now();
        Self::at(reader, resources, now)
    }

    fn at(reader: &'a mut R, resources: &QResources, now: Instant) -> Result<Self, Failure> {
        let deadline = now
            .checked_add(Duration::from_millis(resources.limit(Limit::SocketReadMs)))
            .ok_or_else(|| Failure::budget("HTTP 请求读期限溢出"))?;
        Ok(Self {
            reader,
            deadline,
            maximum: resources.limit(Limit::FrameBytes),
            pending: VecDeque::new(),
            total: 0,
            parsed_head: None,
            body_read: false,
        })
    }

    pub(super) fn head(&mut self) -> Result<Head, Failure> {
        if let Some(head) = &self.parsed_head {
            return Ok(head.clone());
        }
        let request_line = self.line(true)?;
        let head = parse_request_line(&request_line)?;
        let mut count = 0usize;
        let mut content_lengths = Vec::new();
        let mut transfer_encoding = false;
        loop {
            let line = self.line(false)?;
            count = count
                .checked_add(1)
                .ok_or_else(|| Failure::too_many_headers("HTTP 头数量溢出"))?;
            // 与http.client._read_headers一致：最终空行也计入100行上限。
            if count > MAX_HEADERS {
                return Err(Failure::too_many_headers("HTTP 头数量超过 100"));
            }
            if line == b"\r\n" || line == b"\n" {
                break;
            }
            let colon = line
                .iter()
                .position(|b| *b == b':')
                .ok_or_else(|| Failure::invalid("HTTP 头缺少冒号"))?;
            let name = &line[..colon];
            if name.is_empty()
                || !name.iter().all(|b| {
                    b.is_ascii_alphanumeric()
                        || matches!(
                            *b,
                            b'!' | b'#'
                                | b'$'
                                | b'%'
                                | b'&'
                                | b'\''
                                | b'*'
                                | b'+'
                                | b'-'
                                | b'.'
                                | b'^'
                                | b'_'
                                | b'`'
                                | b'|'
                                | b'~'
                        )
                })
            {
                return Err(Failure::invalid("HTTP 头名不合法"));
            }
            if name.eq_ignore_ascii_case(b"content-length") {
                content_lengths.push(trim_header_value(&line[colon + 1..])?);
            } else if name.eq_ignore_ascii_case(b"transfer-encoding") {
                transfer_encoding = true;
            }
        }
        let head = Head {
            method: head.method,
            path: head.path,
            version: head.version,
            content_lengths,
            transfer_encoding,
        };
        self.parsed_head = Some(head.clone());
        Ok(head)
    }

    pub(super) fn body(&mut self, head: &Head) -> Result<Vec<u8>, Failure> {
        if self.parsed_head.as_ref() != Some(head) || self.body_read {
            return Err(Failure::invalid("HTTP body读取顺序不符"));
        }
        self.body_read = true;
        if head.transfer_encoding {
            return Err(Failure::invalid("不接受 Transfer-Encoding"));
        }
        if head.content_lengths.len() != 1 {
            return Err(Failure::invalid("需要唯一 Content-Length"));
        }
        let length = parse_content_length(&head.content_lengths[0])?;
        // 解析头时可能已把 body 读入 pending；这些字节已计入 total，不能再当作头部消费。
        let header_bytes = self
            .total
            .saturating_sub(u64::try_from(self.pending.len()).unwrap_or(u64::MAX));
        let remaining = self.maximum.saturating_sub(header_bytes);
        if length > remaining {
            return Err(Failure::budget("请求帧超过 max_frame_bytes"));
        }
        let capacity = usize::try_from(length)
            .map_err(|_| Failure::budget("请求 body 长度超出本机索引范围"))?;
        let mut body = Vec::new();
        while body.len() < capacity {
            self.check_deadline()?;
            if let Some(byte) = self.pending.pop_front() {
                body.push(byte);
                continue;
            }
            if !self.read_source()? {
                return Err(Failure::invalid("请求 body 截断"));
            }
        }
        Ok(body)
    }

    fn line(&mut self, request_line: bool) -> Result<Vec<u8>, Failure> {
        let mut line = Vec::new();
        loop {
            self.check_deadline()?;
            let byte = match self.next_byte()? {
                Some(byte) => byte,
                None => {
                    if line.is_empty() {
                        return Err(Failure::invalid("HTTP 输入提前结束"));
                    }
                    return Err(Failure::invalid("HTTP 行缺少换行"));
                }
            };
            line.push(byte);
            if line.len() > MAX_LINE {
                return Err(if request_line {
                    Failure::too_long_line("请求行超过 65536 字节")
                } else {
                    Failure::too_many_headers("HTTP 头行超过 65536 字节")
                });
            }
            if byte == b'\n' {
                return Ok(line);
            }
        }
    }

    fn next_byte(&mut self) -> Result<Option<u8>, Failure> {
        self.check_deadline()?;
        if let Some(byte) = self.pending.pop_front() {
            return Ok(Some(byte));
        }
        if self.read_source()? {
            Ok(self.pending.pop_front())
        } else {
            Ok(None)
        }
    }

    fn read_source(&mut self) -> Result<bool, Failure> {
        self.check_deadline()?;
        let remaining = self.maximum.saturating_sub(self.total);
        let ask = usize::try_from(remaining.saturating_add(1).min(READ_CHUNK as u64))
            .unwrap_or(READ_CHUNK)
            .max(1);
        let mut bytes = [0u8; READ_CHUNK];
        let n = loop {
            self.check_deadline()?;
            let left = self
                .deadline
                .checked_duration_since(Instant::now())
                .filter(|left| !left.is_zero())
                .ok_or_else(|| Failure::budget("HTTP 请求读取超过总时限"))?;
            self.reader
                .set_read_timeout(left)
                .map_err(|e| Failure::storage(format!("设置 HTTP 读 timeout 失败: {e}")))?;
            match self.reader.read(&mut bytes[..ask]) {
                Ok(n) => break n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e)
                    if matches!(
                        e.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    ) =>
                {
                    return Err(Failure::budget("HTTP 请求读取超过总时限"))
                }
                Err(e) => return Err(Failure::storage(format!("HTTP 请求读取失败: {e}"))),
            }
        };
        self.check_deadline()?;
        if n == 0 {
            return Ok(false);
        }
        self.total = self
            .total
            .checked_add(n as u64)
            .ok_or_else(|| Failure::budget("HTTP 输入字节计数溢出"))?;
        if self.total > self.maximum {
            return Err(Failure::budget("HTTP 请求行/头/body 超过帧上限"));
        }
        self.pending.extend(&bytes[..n]);
        Ok(true)
    }

    fn check_deadline(&self) -> Result<(), Failure> {
        if Instant::now() >= self.deadline {
            Err(Failure::budget("HTTP 请求读取超过总时限"))
        } else {
            Ok(())
        }
    }
}

fn parse_request_line(line: &[u8]) -> Result<Head, Failure> {
    let line = line
        .strip_suffix(b"\n")
        .ok_or_else(|| Failure::invalid("请求行缺少换行"))?;
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    let fields = line
        .split(|b| *b == b' ' || *b == b'\t')
        .filter(|f| !f.is_empty());
    let fields: Vec<&[u8]> = fields.collect();
    if fields.len() != 3 {
        return Err(Failure::invalid("请求行字段不完整"));
    }
    let method = ascii_text(fields[0], "HTTP method")?;
    let path = ascii_text(fields[1], "HTTP target")?;
    let version = ascii_text(fields[2], "HTTP version")?;
    if version != "HTTP/1.0" && version != "HTTP/1.1" {
        return Err(Failure::invalid("仅支持 HTTP/1.0 与 HTTP/1.1"));
    }
    Ok(Head {
        method,
        path,
        version,
        content_lengths: Vec::new(),
        transfer_encoding: false,
    })
}

fn ascii_text(value: &[u8], what: &str) -> Result<String, Failure> {
    if value.is_empty() || !value.is_ascii() {
        return Err(Failure::invalid(format!("{what}不是合法 ASCII")));
    }
    String::from_utf8(value.to_vec()).map_err(|_| Failure::invalid(format!("{what}不是文本")))
}

fn trim_header_value(value: &[u8]) -> Result<String, Failure> {
    let value = value.strip_suffix(b"\n").unwrap_or(value);
    let value = value.strip_suffix(b"\r").unwrap_or(value);
    let start = value
        .iter()
        .position(|b| !matches!(*b, b' ' | b'\t'))
        .unwrap_or(value.len());
    let value = &value[start..];
    String::from_utf8(value.to_vec()).map_err(|_| Failure::invalid("HTTP 头值不是 UTF-8"))
}

fn parse_content_length(text: &str) -> Result<u64, Failure> {
    let bytes = text.as_bytes();
    if bytes.is_empty()
        || !bytes.is_ascii()
        || !bytes.iter().all(|b| b.is_ascii_digit())
        || (bytes.len() > 1 && bytes[0] == b'0')
    {
        return Err(Failure::invalid("Content-Length必须是规范正整数"));
    }
    let value = text
        .parse::<u64>()
        .map_err(|_| Failure::invalid("Content-Length超出i64域"))?;
    if value == 0 || value > i64::MAX as u64 {
        return Err(Failure::invalid("Content-Length超出i64正整数域"));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    fn resources(max_frame: u64, read_ms: u64) -> QResources {
        let mut v = super::super::q_resources::tests::input();
        v["limits"]["max_frame_bytes"] = json!(max_frame.to_string());
        v["limits"]["socket_read_timeout_ms"] = json!(read_ms.to_string());
        QResources::parse(serde_json::to_vec(&v).unwrap().as_slice()).unwrap()
    }

    impl TimedRead for Cursor<Vec<u8>> {
        fn set_read_timeout(&mut self, _: Duration) -> io::Result<()> {
            Ok(())
        }
    }

    struct Chunked {
        bytes: Vec<u8>,
        offset: usize,
        chunk: usize,
    }
    impl Read for Chunked {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if self.offset == self.bytes.len() {
                return Ok(0);
            }
            let n = self
                .chunk
                .min(out.len())
                .min(self.bytes.len() - self.offset);
            out[..n].copy_from_slice(&self.bytes[self.offset..self.offset + n]);
            self.offset += n;
            Ok(n)
        }
    }
    impl TimedRead for Chunked {
        fn set_read_timeout(&mut self, _: Duration) -> io::Result<()> {
            Ok(())
        }
    }

    struct RetryOnce {
        bytes: Vec<u8>,
        interrupted: bool,
    }
    impl Read for RetryOnce {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if !self.interrupted {
                self.interrupted = true;
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            let n = self.bytes.len().min(out.len());
            out[..n].copy_from_slice(&self.bytes[..n]);
            self.bytes.drain(..n);
            Ok(n)
        }
    }
    impl TimedRead for RetryOnce {
        fn set_read_timeout(&mut self, _: Duration) -> io::Result<()> {
            Ok(())
        }
    }

    struct WouldBlock;
    impl Read for WouldBlock {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::WouldBlock))
        }
    }
    impl TimedRead for WouldBlock {
        fn set_read_timeout(&mut self, _: Duration) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn segmented_headers_and_body_are_counted_once() {
        let bytes = b"POST /api/v3/snapshot HTTP/1.1\r\ncontent-length: 3\r\n\r\nabc";
        let mut reader = Chunked {
            bytes: bytes.to_vec(),
            offset: 0,
            chunk: 2,
        };
        let mut input = Input::new(&mut reader, &resources(1024, 1000)).unwrap();
        let head = input.head().unwrap();
        assert_eq!(head.method, "POST");
        assert_eq!(head.path, "/api/v3/snapshot");
        assert_eq!(input.body(&head).unwrap(), b"abc");
    }

    #[test]
    fn duplicate_content_length_transfer_encoding_and_truncation_reject() {
        for request in [
            b"POST /x HTTP/1.1\r\nContent-Length: 1\r\ncontent-length: 1\r\n\r\na".as_slice(),
            b"POST /x HTTP/1.1\r\nContent-Length: 1\r\nTransfer-Encoding: chunked\r\n\r\na"
                .as_slice(),
            b"POST /x HTTP/1.1\r\nContent-Length: 2\r\n\r\na".as_slice(),
            b"POST /x HTTP/1.1\r\nContent-Length: 1 \r\n\r\na".as_slice(),
        ] {
            let mut reader = Cursor::new(request.to_vec());
            let mut input = Input::new(&mut reader, &resources(1024, 1000)).unwrap();
            let head = input.head().unwrap();
            assert_eq!(input.body(&head).unwrap_err().status, 400);
        }
    }

    #[test]
    fn frame_and_line_limits_keep_old_status_classes() {
        let mut reader = Cursor::new(b"POST /x HTTP/1.1\r\n\r\n".to_vec());
        let mut input = Input::new(&mut reader, &resources(4, 1000)).unwrap();
        assert_eq!(input.head().unwrap_err().status, 503);

        let long = format!("POST /{} HTTP/1.1\r\n\r\n", "x".repeat(MAX_LINE));
        let mut reader = Cursor::new(long.into_bytes());
        let mut input = Input::new(&mut reader, &resources(200_000, 1000)).unwrap();
        assert_eq!(input.head().unwrap_err().status, 414);
    }

    #[test]
    fn buffered_body_counts_once_against_exact_frame_limit() {
        let prefix = b"POST /x HTTP/1.1\r\nContent-Length: 3\r\n\r\n";
        let mut bytes = prefix.to_vec();
        bytes.extend_from_slice(b"abc");
        let exact = bytes.len() as u64;
        let mut reader = Cursor::new(bytes.clone());
        let mut input = Input::new(&mut reader, &resources(exact, 1000)).unwrap();
        let head = input.head().unwrap();
        assert_eq!(input.body(&head).unwrap(), b"abc");

        let mut reader = Cursor::new(bytes);
        let mut input = Input::new(&mut reader, &resources(exact - 1, 1000)).unwrap();
        assert_eq!(input.head().unwrap_err().status, 503);
    }

    #[test]
    fn python_header_count_boundary_is_99_nonempty_lines() {
        for count in [99usize, 100usize] {
            let mut bytes = b"GET / HTTP/1.1\r\n".to_vec();
            for i in 0..count {
                bytes.extend_from_slice(format!("X{i}: x\r\n").as_bytes());
            }
            bytes.extend_from_slice(b"\r\n");
            let mut reader = Cursor::new(bytes);
            let mut input = Input::new(&mut reader, &resources(200_000, 1000)).unwrap();
            assert_eq!(
                input.head().map(|_| 200u16).unwrap_or_else(|e| e.status),
                if count == 99 { 200 } else { 431 }
            );
        }
    }

    #[test]
    fn expired_absolute_clock_rejects_before_reader_access() {
        let mut reader = Cursor::new(b"GET / HTTP/1.1\r\n\r\n".to_vec());
        let r = resources(1024, 1);
        let start = Instant::now() - Duration::from_millis(10);
        let mut input = Input::at(&mut reader, &r, start).unwrap();
        assert_eq!(input.head().unwrap_err().status, 503);
    }

    #[test]
    fn interrupted_read_retries_but_would_block_is_budget_failure() {
        let bytes = b"GET / HTTP/1.1\r\n\r\n".to_vec();
        let mut retry = RetryOnce {
            bytes,
            interrupted: false,
        };
        let mut input = Input::new(&mut retry, &resources(1024, 1000)).unwrap();
        assert_eq!(input.head().unwrap().path, "/");

        let mut blocked = WouldBlock;
        let mut input = Input::new(&mut blocked, &resources(1024, 1000)).unwrap();
        assert_eq!(input.head().unwrap_err().status, 503);
    }

    #[test]
    fn interrupt_updates_socket_timeout_and_buffered_body_cannot_renew_deadline() {
        struct InterruptedOnce {
            bytes: Cursor<Vec<u8>>,
            timeouts: Vec<Duration>,
            first: bool,
        }
        impl Read for InterruptedOnce {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                if self.first {
                    self.first = false;
                    std::thread::sleep(Duration::from_millis(2));
                    return Err(io::ErrorKind::Interrupted.into());
                }
                self.bytes.read(out)
            }
        }
        impl TimedRead for InterruptedOnce {
            fn set_read_timeout(&mut self, left: Duration) -> io::Result<()> {
                self.timeouts.push(left);
                Ok(())
            }
        }
        let mut reader = InterruptedOnce {
            bytes: Cursor::new(b"POST /x HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc".to_vec()),
            timeouts: Vec::new(),
            first: true,
        };
        let mut input = Input::new(&mut reader, &resources(1024, 1000)).unwrap();
        let head = input.head().unwrap();
        assert_eq!(input.reader.timeouts.len(), 2);
        assert!(input.reader.timeouts[1] < input.reader.timeouts[0]);
        assert_eq!(input.pending.len(), 3);
        input.deadline = Instant::now() - Duration::from_millis(1);
        assert_eq!(input.body(&head).unwrap_err().error, "QueryBudgetExceeded");
    }
}
