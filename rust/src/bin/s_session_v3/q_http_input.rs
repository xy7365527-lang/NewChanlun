//! #1394：Q HTTP 输入层的有界读。
//!
//! 这里复用旧只读 HTTP 合同：请求行、全部头和 body 共用一个绝对读期限与
//! `max_frame_bytes`；不接受 chunked，也不按 Content-Length 预分配。它只解析
//! 输入，不决定路由、响应 envelope 或服务端 Q 资源之外的消费者 Job 记账。

use super::{
    business_json::TextValue,
    q_resources::{Limit, QResources},
};
#[cfg(feature = "s_allocation_json_testonly")]
use newchan_rust::s_content::capture_allocation::audit_json as aj;
use newchan_rust::s_content::{capture_allocation::OwnedBudget, CodecError, CodecVec};
use std::{
    io::{self, Read},
    time::{Duration, Instant},
};

const MAX_LINE: usize = 65_536;
// Python http.client 把终止空行也纳入 100 行保护，因此最多接受 99 个非空头行。
const MAX_HEADERS: usize = 100;
const READ_CHUNK: usize = 4096;

/// HTTP 输入失败；上层只需将 status/error/detail 组装成响应。
#[derive(Debug, PartialEq, Eq)]
pub(super) enum FailureDetail {
    Static(&'static str),
    Text(TextValue),
    Allocation(CodecError),
}
impl FailureDetail {
    pub(super) fn as_str(&self) -> &str {
        match self {
            Self::Static(s) => s,
            Self::Text(s) => s.as_str(),
            Self::Allocation(e) if e.code() == newchan_rust::s_content::capture_allocation::LIMIT
                && e.detail() == "codec requested Layout 超过显式实验额度或分配失败" =>
                "prototype_capture_owned_payload_limit: codec requested Layout 超过显式实验额度或分配失败",
            Self::Allocation(e) => e.detail(),
        }
    }
}
impl std::ops::Deref for FailureDetail {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl std::fmt::Display for FailureDetail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}
impl serde::Serialize for FailureDetail {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}
impl From<&'static str> for FailureDetail {
    fn from(s: &'static str) -> Self {
        Self::Static(s)
    }
}
impl From<String> for FailureDetail {
    fn from(s: String) -> Self {
        Self::Text(TextValue::Plain(s))
    }
}
impl From<TextValue> for FailureDetail {
    fn from(s: TextValue) -> Self {
        Self::Text(s)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Failure {
    pub status: u16,
    pub error: &'static str,
    pub detail: FailureDetail,
}

impl Failure {
    fn invalid(detail: impl Into<FailureDetail>) -> Self {
        Self {
            status: 400,
            error: "InvalidQuery",
            detail: detail.into(),
        }
    }

    fn too_long_line(detail: impl Into<FailureDetail>) -> Self {
        Self {
            status: 414,
            error: "InvalidQuery",
            detail: detail.into(),
        }
    }

    fn too_many_headers(detail: impl Into<FailureDetail>) -> Self {
        Self {
            status: 431,
            error: "InvalidQuery",
            detail: detail.into(),
        }
    }

    fn budget(detail: impl Into<FailureDetail>) -> Self {
        Self {
            status: 503,
            error: "QueryBudgetExceeded",
            detail: detail.into(),
        }
    }

    fn allocation(error: CodecError) -> Self {
        // 分配拒绝保留原typed cause；HTTP类沿既有503/Incomplete，失败本身零负载分配。
        Self {
            status: 503,
            error: "QueryBudgetExceeded",
            detail: FailureDetail::Allocation(error),
        }
    }

    pub(super) fn cause_code(&self) -> Option<&'static str> {
        match &self.detail {
            FailureDetail::Allocation(e) => Some(e.code()),
            _ => None,
        }
    }

    fn storage(detail: impl Into<FailureDetail>) -> Self {
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
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Head {
    pub method: TextValue,
    pub path: TextValue,
    version: TextValue,
    content_lengths: CodecVec<TextValue>,
    transfer_encoding: bool,
}

impl Head {
    pub(super) fn allocation_budget(&self) -> Option<&OwnedBudget> {
        self.content_lengths.allocation_budget()
    }
}

pub(super) struct Input<'a, R: TimedRead> {
    reader: &'a mut R,
    deadline: Instant,
    maximum: u64,
    pending: CodecVec<u8>,
    pending_offset: usize,
    allocation: Option<OwnedBudget>,
    total: u64,
    parsed_head: Option<Head>,
    body_read: bool,
}

impl<'a, R: TimedRead> Input<'a, R> {
    pub(super) fn new(reader: &'a mut R, resources: &QResources) -> Result<Self, Failure> {
        Self::with_allocation(reader, resources, None)
    }

    #[cfg(test)]
    fn at(reader: &'a mut R, resources: &QResources, now: Instant) -> Result<Self, Failure> {
        Self::at_with_allocation(reader, resources, now, None)
    }

    pub(super) fn with_allocation(
        reader: &'a mut R,
        resources: &QResources,
        allocation: Option<&OwnedBudget>,
    ) -> Result<Self, Failure> {
        Self::at_with_allocation(reader, resources, Instant::now(), allocation)
    }

    fn at_with_allocation(
        reader: &'a mut R,
        resources: &QResources,
        now: Instant,
        allocation: Option<&OwnedBudget>,
    ) -> Result<Self, Failure> {
        #[cfg(not(feature = "s_allocation_json_testonly"))]
        if allocation.is_some() {
            return Err(Failure {
                status: 503,
                error: newchan_rust::s_content::capture_allocation::INCOMPLETE,
                detail: "Owned HTTP输入入口未启用".into(),
            });
        }
        let deadline = now
            .checked_add(Duration::from_millis(resources.limit(Limit::SocketReadMs)))
            .ok_or_else(|| Failure::budget("HTTP 请求读期限溢出"))?;
        Ok(Self {
            reader,
            deadline,
            maximum: resources.limit(Limit::FrameBytes),
            pending: CodecVec::new_with_allocation(allocation),
            pending_offset: 0,
            allocation: allocation.cloned(),
            total: 0,
            parsed_head: None,
            body_read: false,
        })
    }

    pub(super) fn head(&mut self) -> Result<Head, Failure> {
        if let Some(head) = &self.parsed_head {
            return clone_head(head);
        }
        let request_line = self.line(true)?;
        let head = parse_request_line(&request_line, self.allocation.as_ref())?;
        let mut count = 0usize;
        let mut content_lengths = CodecVec::new_with_allocation(self.allocation.as_ref());
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
            if &*line == b"\r\n" || &*line == b"\n" {
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
                content_lengths
                    .try_push(trim_header_value(
                        &line[colon + 1..],
                        self.allocation.as_ref(),
                    )?)
                    .map_err(Failure::allocation)?;
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
        self.parsed_head = Some(clone_head(&head)?);
        Ok(head)
    }

    pub(super) fn body(&mut self, head: &Head) -> Result<CodecVec<u8>, Failure> {
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
            .saturating_sub(u64::try_from(self.pending_len()).unwrap_or(u64::MAX));
        let remaining = self.maximum.saturating_sub(header_bytes);
        if length > remaining {
            return Err(Failure::budget("请求帧超过 max_frame_bytes"));
        }
        let capacity = usize::try_from(length)
            .map_err(|_| Failure::budget("请求 body 长度超出本机索引范围"))?;
        let mut body = CodecVec::new_with_allocation(self.allocation.as_ref());
        while body.len() < capacity {
            self.check_deadline()?;
            body.try_reserve(1).map_err(Failure::allocation)?;
            if let Some(byte) = self.pop_pending() {
                body.try_push(byte).map_err(Failure::allocation)?;
                continue;
            }
            if !self.read_source()? {
                return Err(Failure::invalid("请求 body 截断"));
            }
        }
        Ok(body)
    }

    fn line(&mut self, request_line: bool) -> Result<CodecVec<u8>, Failure> {
        let mut line = CodecVec::new_with_allocation(self.allocation.as_ref());
        loop {
            self.check_deadline()?;
            line.try_reserve(1).map_err(Failure::allocation)?;
            let byte = match self.next_byte()? {
                Some(byte) => byte,
                None => {
                    if line.is_empty() {
                        return Err(Failure::invalid("HTTP 输入提前结束"));
                    }
                    return Err(Failure::invalid("HTTP 行缺少换行"));
                }
            };
            line.try_push(byte).map_err(Failure::allocation)?;
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
        if let Some(byte) = self.pop_pending() {
            return Ok(Some(byte));
        }
        if self.read_source()? {
            Ok(self.pop_pending())
        } else {
            Ok(None)
        }
    }

    fn pending_len(&self) -> usize {
        self.pending.len() - self.pending_offset
    }

    fn pop_pending(&mut self) -> Option<u8> {
        let byte = self.pending.get(self.pending_offset).copied()?;
        self.pending_offset += 1;
        Some(byte)
    }

    fn read_source(&mut self) -> Result<bool, Failure> {
        self.check_deadline()?;
        let remaining = self.maximum.saturating_sub(self.total);
        let ask = usize::try_from(remaining.saturating_add(1).min(READ_CHUNK as u64))
            .unwrap_or(READ_CHUNK)
            .max(1);
        debug_assert_eq!(self.pending_len(), 0);
        // 在读源前借记真实缓存 Layout；拒绝保留原字节与读偏移。
        self.pending
            .try_reserve(ask.saturating_sub(self.pending.len()))
            .map_err(Failure::allocation)?;
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
                .map_err(|e| self.storage_error(format_args!("设置 HTTP 读 timeout 失败: {e}")))?;
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
                Err(e) => return Err(self.storage_error(format_args!("HTTP 请求读取失败: {e}"))),
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
        self.pending.clear();
        self.pending_offset = 0;
        self.pending
            .try_extend_bytes(&bytes[..n])
            .map_err(Failure::allocation)?;
        Ok(true)
    }

    fn storage_error(&self, detail: std::fmt::Arguments<'_>) -> Failure {
        match format_text(detail, self.allocation.as_ref()) {
            Ok(detail) => Failure::storage(detail),
            Err(error) => error,
        }
    }

    fn check_deadline(&self) -> Result<(), Failure> {
        if Instant::now() >= self.deadline {
            Err(Failure::budget("HTTP 请求读取超过总时限"))
        } else {
            Ok(())
        }
    }
}

fn clone_head(head: &Head) -> Result<Head, Failure> {
    let budget = head.content_lengths.allocation_budget();
    let mut lengths = CodecVec::new_with_allocation(budget);
    for length in &head.content_lengths {
        lengths
            .try_push(copy_text(length.as_str(), budget)?)
            .map_err(Failure::allocation)?;
    }
    Ok(Head {
        method: copy_text(head.method.as_str(), budget)?,
        path: copy_text(head.path.as_str(), budget)?,
        version: copy_text(head.version.as_str(), budget)?,
        content_lengths: lengths,
        transfer_encoding: head.transfer_encoding,
    })
}

fn copy_text(value: &str, budget: Option<&OwnedBudget>) -> Result<TextValue, Failure> {
    #[cfg(feature = "s_allocation_json_testonly")]
    if let Some(budget) = budget {
        return aj::TextOwner::copy_from(value, budget)
            .map(TextValue::Owned)
            .map_err(Failure::allocation);
    }
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if budget.is_some() {
        return Err(Failure {
            status: 503,
            error: newchan_rust::s_content::capture_allocation::INCOMPLETE,
            detail: "Owned HTTP文本入口未启用".into(),
        });
    }
    Ok(TextValue::Plain(value.to_owned()))
}

fn format_text(
    value: std::fmt::Arguments<'_>,
    budget: Option<&OwnedBudget>,
) -> Result<TextValue, Failure> {
    if budget.is_none() {
        return Ok(TextValue::Plain(value.to_string()));
    }
    struct Writer {
        bytes: CodecVec<u8>,
        error: Option<CodecError>,
    }
    impl std::fmt::Write for Writer {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            self.bytes
                .try_extend_bytes(value.as_bytes())
                .map_err(|error| {
                    self.error = Some(error);
                    std::fmt::Error
                })
        }
    }
    let mut writer = Writer {
        bytes: CodecVec::new_with_allocation(budget),
        error: None,
    };
    let result = std::fmt::write(&mut writer, value);
    if let Some(error) = writer.error {
        return Err(Failure::allocation(error));
    }
    if result.is_err() {
        return Err(Failure::storage("HTTP 错误文本格式化失败"));
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    {
        return aj::TextOwner::from_owned_bytes(writer.bytes)
            .map(TextValue::Owned)
            .map_err(Failure::allocation);
    }
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    {
        Err(Failure {
            status: 503,
            error: newchan_rust::s_content::capture_allocation::INCOMPLETE,
            detail: "Owned HTTP文本入口未启用".into(),
        })
    }
}

fn parse_request_line(line: &[u8], budget: Option<&OwnedBudget>) -> Result<Head, Failure> {
    let line = line
        .strip_suffix(b"\n")
        .ok_or_else(|| Failure::invalid("请求行缺少换行"))?;
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    let mut fields = line
        .split(|b| *b == b' ' || *b == b'\t')
        .filter(|f| !f.is_empty());
    let (Some(method), Some(path), Some(version), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return Err(Failure::invalid("请求行字段不完整"));
    };
    let method = ascii_text(method, "HTTP method", budget)?;
    let path = ascii_text(path, "HTTP target", budget)?;
    let version = ascii_text(version, "HTTP version", budget)?;
    if version != "HTTP/1.0" && version != "HTTP/1.1" {
        return Err(Failure::invalid("仅支持 HTTP/1.0 与 HTTP/1.1"));
    }
    Ok(Head {
        method,
        path,
        version,
        content_lengths: CodecVec::new_with_allocation(budget),
        transfer_encoding: false,
    })
}

fn ascii_text(
    value: &[u8],
    what: &str,
    budget: Option<&OwnedBudget>,
) -> Result<TextValue, Failure> {
    if value.is_empty() || !value.is_ascii() {
        return Err(Failure::invalid(format_text(
            format_args!("{what}不是合法 ASCII"),
            budget,
        )?));
    }
    // ASCII 是 UTF-8；在负载分配前直接借用。
    copy_text(std::str::from_utf8(value).expect("ASCII is UTF-8"), budget)
}

fn trim_header_value(value: &[u8], budget: Option<&OwnedBudget>) -> Result<TextValue, Failure> {
    let value = value.strip_suffix(b"\n").unwrap_or(value);
    let value = value.strip_suffix(b"\r").unwrap_or(value);
    let start = value
        .iter()
        .position(|b| !matches!(*b, b' ' | b'\t'))
        .unwrap_or(value.len());
    let text = std::str::from_utf8(&value[start..])
        .map_err(|_| Failure::invalid("HTTP 头值不是 UTF-8"))?;
    copy_text(text, budget)
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
        assert_eq!(&*input.body(&head).unwrap(), b"abc");
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
        assert_eq!(&*input.body(&head).unwrap(), b"abc");

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
        assert_eq!(input.pending_len(), 3);
        input.deadline = Instant::now() - Duration::from_millis(1);
        assert_eq!(input.body(&head).unwrap_err().error, "QueryBudgetExceeded");
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    fn named_budget() -> OwnedBudget {
        OwnedBudget::new(50 * 1024 * 1024)
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    fn consume_available_except(budget: &OwnedBudget, free: usize) -> CodecVec<u8> {
        let mut padding = CodecVec::new_with_allocation(Some(budget));
        let snapshot = budget.snapshot();
        padding
            .try_reserve_exact(snapshot.limit_bytes - snapshot.live_requested_layout_bytes - free)
            .unwrap();
        padding
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    fn assert_balanced(budget: &OwnedBudget) {
        let snapshot = budget.snapshot();
        assert_eq!(snapshot.live_requested_layout_bytes, 0);
        assert_eq!(snapshot.allocated_layouts, snapshot.released_layouts);
        assert!(!budget.ledger_health().invariant_broken);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn actual_owned_http_all_legal_split_boundaries_match_none_and_release() {
        struct SplitAt {
            bytes: &'static [u8],
            offset: usize,
            split: usize,
        }
        impl Read for SplitAt {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                let end = if self.offset < self.split {
                    self.split
                } else {
                    self.bytes.len()
                };
                let n = (end - self.offset).min(out.len());
                out[..n].copy_from_slice(&self.bytes[self.offset..self.offset + n]);
                self.offset += n;
                Ok(n)
            }
        }
        impl TimedRead for SplitAt {
            fn set_read_timeout(&mut self, _: Duration) -> io::Result<()> {
                Ok(())
            }
        }
        for request in [
            b"POST /api/v3/snapshot HTTP/1.1\r\nContent-Length: 3\r\nX-Test: x\r\n\r\nabc"
                .as_slice(),
            b"POST\t/x  HTTP/1.0\ncontent-length:\t3\n\nabc".as_slice(),
        ] {
            for split in 1..request.len() {
                let budget = named_budget();
                let mut plain_source = SplitAt {
                    bytes: request,
                    offset: 0,
                    split,
                };
                let mut owned_source = SplitAt {
                    bytes: request,
                    offset: 0,
                    split,
                };
                let r = resources(request.len() as u64, 1000);
                {
                    let mut plain = Input::new(&mut plain_source, &r).unwrap();
                    let mut owned =
                        Input::with_allocation(&mut owned_source, &r, Some(&budget)).unwrap();
                    let plain_head = plain.head().unwrap();
                    let owned_head = owned.head().unwrap();
                    assert_eq!(plain_head, owned_head);
                    assert!(owned_head.allocation_budget().unwrap().same_ledger(&budget));
                    assert_eq!(owned.head().unwrap(), owned_head);
                    let plain_body = plain.body(&plain_head).unwrap();
                    let owned_body = owned.body(&owned_head).unwrap();
                    assert_eq!(&*plain_body, &*owned_body);
                    assert!(owned_body.allocation_budget().unwrap().same_ledger(&budget));
                    assert_eq!(owned.total, request.len() as u64);
                    assert!(budget.snapshot().live_requested_layout_bytes > 0);
                }
                assert_eq!(plain_source.offset, owned_source.offset);
                assert_balanced(&budget);
            }
        }
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn actual_owned_http_layout_sum_retains_head_body_until_drop() {
        fn head_bytes(head: &Head) -> usize {
            head.method.capacity()
                + head.path.capacity()
                + head.version.capacity()
                + head.content_lengths.capacity() * std::mem::size_of::<TextValue>()
                + head
                    .content_lengths
                    .iter()
                    .map(TextValue::capacity)
                    .sum::<usize>()
        }
        let budget = named_budget();
        let mut source = Cursor::new(b"POST /x HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc".to_vec());
        let mut input =
            Input::with_allocation(&mut source, &resources(1024, 1000), Some(&budget)).unwrap();
        let head = input.head().unwrap();
        let body = input.body(&head).unwrap();
        let retained_input =
            input.pending.capacity() + head_bytes(input.parsed_head.as_ref().unwrap());
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            retained_input + head_bytes(&head) + body.capacity()
        );
        drop(input);
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            head_bytes(&head) + body.capacity()
        );
        drop(head);
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            body.capacity()
        );
        drop(body);
        assert_balanced(&budget);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn actual_owned_http_refusal_precedes_source_and_preserves_cached_head_body() {
        let budget = named_budget();
        let padding = consume_available_except(&budget, 0);
        let mut source = Cursor::new(b"POST /x HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc".to_vec());
        let before = budget.snapshot();
        {
            let mut input =
                Input::with_allocation(&mut source, &resources(1024, 1000), Some(&budget)).unwrap();
            let failure = input.head().unwrap_err();
            assert_eq!(failure.status, 503);
            assert_eq!(failure.error, "QueryBudgetExceeded");
            assert_eq!(
                failure.cause_code(),
                Some(newchan_rust::s_content::capture_allocation::LIMIT)
            );
            assert_eq!(failure.detail.as_str(), "prototype_capture_owned_payload_limit: codec requested Layout 超过显式实验额度或分配失败");
            assert_eq!(input.total, 0);
            assert_eq!(input.reader.position(), 0);
        }
        let after = budget.snapshot();
        assert_eq!(after.allocation_attempts, before.allocation_attempts);
        assert_eq!(after.allocated_layouts, before.allocated_layouts);
        assert_eq!(
            after.live_requested_layout_bytes,
            before.live_requested_layout_bytes
        );
        assert!(after.rejected_layouts > before.rejected_layouts);
        drop(padding);
        assert_balanced(&budget);

        let mut input =
            Input::with_allocation(&mut source, &resources(1024, 1000), Some(&budget)).unwrap();
        let head = input.head().unwrap();
        let offset = input.pending_offset;
        let source_position = input.reader.position();
        let padding = consume_available_except(&budget, 0);
        let before = budget.snapshot();
        assert_eq!(input.head().unwrap_err().status, 503);
        assert_eq!(input.parsed_head.as_ref(), Some(&head));
        assert_eq!(input.pending_offset, offset);
        assert_eq!(input.body(&head).unwrap_err().status, 503);
        assert_eq!(input.pending_offset, offset);
        assert_eq!(&input.pending[offset..], b"abc");
        assert_eq!(input.reader.position(), source_position);
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            before.live_requested_layout_bytes
        );
        assert_eq!(
            budget.snapshot().allocated_layouts,
            before.allocated_layouts
        );
        drop((padding, input, head));
        assert_balanced(&budget);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn actual_owned_http_partial_body_growth_refusal_preserves_unconsumed_tail() {
        let budget = named_budget();
        let mut source =
            Cursor::new(b"POST /x HTTP/1.1\r\nContent-Length: 9\r\n\r\nabcdefghi".to_vec());
        let mut input =
            Input::with_allocation(&mut source, &resources(1024, 1000), Some(&budget)).unwrap();
        let head = input.head().unwrap();
        let start = input.pending_offset;
        // u8 Vec 的首个实际增长为 8，下一块为16；旧8必须与新16同时活跃。
        let padding = consume_available_except(&budget, 16);
        let before = budget.snapshot();
        assert_eq!(input.body(&head).unwrap_err().status, 503);
        assert_eq!(input.pending_offset, start + 8);
        assert_eq!(&input.pending[input.pending_offset..], b"i");
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            before.live_requested_layout_bytes
        );
        let probe = budget.allocation_request_probe();
        assert_eq!(probe.refused_bytes, 16);
        assert_eq!(
            probe.live_before_refusal,
            before.live_requested_layout_bytes + 8
        );
        drop((padding, input, head));
        assert_balanced(&budget);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn actual_owned_http_parse_errors_keep_none_first_error_and_release() {
        let cases: &[&[u8]] = &[
            b"", b"GET / HTTP/1.1", b"GET / HTTP/1.1 extra\r\n\r\n",
            b"GET / HTTP/2\r\n\r\n", b"GET / HTTP/1.1\r\nBad\r\n\r\n",
            b"GET / HTTP/1.1\r\nBad name: x\r\n\r\n",
            b"POST /x HTTP/1.1\r\nContent-Length: 1\r\nContent-Length: 2\r\nTransfer-Encoding: chunked\r\n\r\na",
            b"POST /x HTTP/1.1\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\na",
            b"POST /x HTTP/1.1\r\nContent-Length: 3\r\n\r\nab",
            b"POST /x HTTP/1.1\r\nContent-Length: 1 \r\n\r\na",
            b"POST /x HTTP/1.1\r\nContent-Length: 01\r\n\r\na",
            b"POST /x HTTP/1.1\r\nContent-Length: 0\r\n\r\na",
            b"POST /x HTTP/1.1\r\nContent-Length: 9223372036854775808\r\n\r\na",
            b"POST /x HTTP/1.1\r\nContent-Length: \xff\r\n\r\na",
            b"POST /\xff HTTP/1.1\r\n\r\n",
        ];
        for request in cases {
            for chunk in [1, READ_CHUNK] {
                let budget = named_budget();
                let r = resources(200_000, 1000);
                let mut plain_reader = Chunked {
                    bytes: request.to_vec(),
                    offset: 0,
                    chunk,
                };
                let mut owned_reader = Chunked {
                    bytes: request.to_vec(),
                    offset: 0,
                    chunk,
                };
                {
                    let mut plain = Input::new(&mut plain_reader, &r).unwrap();
                    let mut owned =
                        Input::with_allocation(&mut owned_reader, &r, Some(&budget)).unwrap();
                    let plain_error = match plain.head() {
                        Ok(h) => plain.body(&h).unwrap_err(),
                        Err(e) => e,
                    };
                    let owned_error = match owned.head() {
                        Ok(h) => owned.body(&h).unwrap_err(),
                        Err(e) => e,
                    };
                    assert_eq!(plain_error.status, owned_error.status);
                    assert_eq!(plain_error.error, owned_error.error);
                    assert_eq!(plain_error.detail.as_str(), owned_error.detail.as_str());
                }
                assert_balanced(&budget);
            }
        }
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn actual_owned_http_guard_boundaries_and_timeouts_match_none() {
        let mut requests = Vec::new();
        for count in [99usize, 100usize] {
            let mut request = b"GET / HTTP/1.1\r\n".to_vec();
            for _ in 0..count {
                request.extend_from_slice(b"X: x\r\n");
            }
            request.extend_from_slice(b"\r\n");
            requests.push(request);
        }
        for length in [MAX_LINE, MAX_LINE + 1] {
            let mut request = b"GET /".to_vec();
            request.resize(length - b" HTTP/1.1\r\n".len(), b'x');
            request.extend_from_slice(b" HTTP/1.1\r\n\r\n");
            requests.push(request);
            let mut request = b"GET / HTTP/1.1\r\nX: ".to_vec();
            request.resize(b"GET / HTTP/1.1\r\n".len() + length - 2, b'x');
            request.extend_from_slice(b"\r\n\r\n");
            requests.push(request);
        }
        for request in requests {
            let budget = named_budget();
            let r = resources(200_000, 1000);
            let mut plain_reader = Cursor::new(request.clone());
            let mut owned_reader = Cursor::new(request);
            {
                let mut plain = Input::new(&mut plain_reader, &r).unwrap();
                let mut owned =
                    Input::with_allocation(&mut owned_reader, &r, Some(&budget)).unwrap();
                match (plain.head(), owned.head()) {
                    (Ok(p), Ok(o)) => assert_eq!(p, o),
                    (Err(p), Err(o)) => {
                        assert_eq!(p.status, o.status);
                        assert_eq!(p.error, o.error);
                        assert_eq!(p.detail.as_str(), o.detail.as_str());
                    }
                    other => panic!("None/Some 边界不同: {other:?}"),
                }
            }
            assert_balanced(&budget);
        }
        let budget = named_budget();
        let r = resources(1024, 1);
        let mut reader = Cursor::new(b"GET / HTTP/1.1\r\n\r\n".to_vec());
        {
            let mut input = Input::at_with_allocation(
                &mut reader,
                &r,
                Instant::now() - Duration::from_millis(10),
                Some(&budget),
            )
            .unwrap();
            assert_eq!(
                input.head().unwrap_err().detail.as_str(),
                "HTTP 请求读取超过总时限"
            );
            assert_eq!(input.reader.position(), 0);
        }
        assert_balanced(&budget);
        {
            let mut blocked = WouldBlock;
            let mut input =
                Input::with_allocation(&mut blocked, &resources(1024, 1000), Some(&budget))
                    .unwrap();
            assert_eq!(
                input.head().unwrap_err().detail.as_str(),
                "HTTP 请求读取超过总时限"
            );
        }
        assert_balanced(&budget);
    }

    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn actual_owned_http_io_error_and_buffered_deadline_keep_none_behavior() {
        struct Broken {
            timeout: bool,
        }
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::ErrorKind::PermissionDenied.into())
            }
        }
        impl TimedRead for Broken {
            fn set_read_timeout(&mut self, _: Duration) -> io::Result<()> {
                if self.timeout {
                    Err(io::ErrorKind::InvalidInput.into())
                } else {
                    Ok(())
                }
            }
        }
        for timeout in [false, true] {
            let budget = named_budget();
            let mut plain_source = Broken { timeout };
            let mut owned_source = Broken { timeout };
            {
                let mut plain = Input::new(&mut plain_source, &resources(1024, 1000)).unwrap();
                let mut owned = Input::with_allocation(
                    &mut owned_source,
                    &resources(1024, 1000),
                    Some(&budget),
                )
                .unwrap();
                let plain_error = plain.head().unwrap_err();
                let owned_error = owned.head().unwrap_err();
                assert_eq!(plain_error.status, owned_error.status);
                assert_eq!(plain_error.error, owned_error.error);
                assert_eq!(plain_error.detail.as_str(), owned_error.detail.as_str());
                assert!(matches!(
                    owned_error.detail,
                    FailureDetail::Text(TextValue::Owned(_))
                ));
            }
            assert_balanced(&budget);
        }
        let budget = named_budget();
        let mut source = RetryOnce {
            bytes: b"POST /x HTTP/1.1\r\nContent-Length: 3\r\n\r\nabc".to_vec(),
            interrupted: false,
        };
        {
            let mut input =
                Input::with_allocation(&mut source, &resources(1024, 1000), Some(&budget)).unwrap();
            let head = input.head().unwrap();
            let offset = input.pending_offset;
            let before = budget.snapshot();
            input.deadline = Instant::now() - Duration::from_millis(1);
            assert_eq!(
                input.body(&head).unwrap_err().detail.as_str(),
                "HTTP 请求读取超过总时限"
            );
            assert_eq!(input.pending_offset, offset);
            assert_eq!(
                budget.snapshot().allocation_attempts,
                before.allocation_attempts
            );
        }
        assert_balanced(&budget);
    }
}
