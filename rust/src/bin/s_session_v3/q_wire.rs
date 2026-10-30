//! #1394：Q v3共用的完整请求校验与完整回复编码；不执行HTTP或虚构wire账。
use super::q_resources::{Limit, QResources};
use newchan_rust::s_content::access::Access;
use newchan_rust::session_protocol::{positive, strict_json, validate_envelope};
use serde_json::Value;
use std::io::{self, Write};

#[allow(
    dead_code,
    reason = "#1608：无预算/普通值的旧包装入口，调用方已改走带 owner/admission/typed 的版本；先量再动，保留不删"
)]
pub(super) fn parse(bytes: &[u8], resources: &QResources) -> Result<Value, String> {
    if bytes.len() as u128 > u128::from(resources.limit(Limit::FrameBytes)) {
        return Err(err("Incomplete：完整请求超过max_frame_bytes"));
    }
    validate_envelope(strict_json(bytes)?, "s-session/3")
}
#[derive(Debug)]
pub(crate) enum FrameFailure {
    Limit,
}
impl std::fmt::Display for FrameFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("QReplyInvalid：Incomplete：完整请求超过max_frame_bytes")
    }
}
pub(crate) enum ParseOwnerError {
    Frame(FrameFailure),
    Syntax(serde_json::Error),
    #[cfg(feature = "s_allocation_json_testonly")]
    OwnedSyntax(sc::capture_allocation::audit_json::strict::Diagnostic),
    Allocation(sc::CodecError),
}
impl std::fmt::Display for ParseOwnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Frame(cause) => std::fmt::Display::fmt(cause, f),
            Self::Syntax(cause) => write!(f, "SchemaUnsupported：{cause}"),
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::OwnedSyntax(cause) => write!(f, "SchemaUnsupported：{cause}"),
            Self::Allocation(cause) => write!(f, "QReplyInvalid：{cause}"),
        }
    }
}
pub(crate) fn decode_owner(
    bytes: &[u8],
    resources: &QResources,
    allocation: Option<&OwnedBudget>,
) -> Result<bj::JsonOwner, ParseOwnerError> {
    if bytes.len() as u128 > resources.limit(Limit::FrameBytes) as u128 {
        return Err(ParseOwnerError::Frame(FrameFailure::Limit));
    }
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if allocation.is_some() {
        return Err(ParseOwnerError::Allocation(sc::CodecError::new(
            sc::capture_allocation::INCOMPLETE,
            "owned Q请求未启用",
        )));
    }
    bj::strict_json_owner(bytes, allocation).map_err(|cause| match cause {
        bj::StrictOwnerError::Syntax(cause) => ParseOwnerError::Syntax(cause),
        #[cfg(feature = "s_allocation_json_testonly")]
        bj::StrictOwnerError::OwnedSyntax(cause) => ParseOwnerError::OwnedSyntax(cause),
        bj::StrictOwnerError::Allocation(cause) => ParseOwnerError::Allocation(cause),
    })
}
/// 兼容String入口；HTTP直接分别调用decode与typed公共核以持住失败的来源owner。
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "#1608：仅测试使用（非测试构建中只被已豁免的未用项调用）"
    )
)]
pub(crate) fn parse_owner(
    bytes: &[u8],
    resources: &QResources,
    allocation: Option<&OwnedBudget>,
) -> Result<bj::JsonOwner, String> {
    let owner = decode_owner(bytes, resources, allocation).map_err(|e| e.to_string())?;
    newchan_rust::session_protocol::validate_envelope_typed(
        owner.view(),
        "s-session/3",
        allocation,
    )
    .map_err(|e| e.to_string())?;
    Ok(owner)
}
#[derive(Debug)]
pub(crate) enum RouteFixed {
    SnapshotOp,
    SnapshotVersion,
    SnapshotDigest,
    SnapshotChecksum,
    SnapshotMode,
    SnapshotExact,
    SnapshotLimit,
    ContentOp,
    ContentVersion,
    ContentDigest,
    ContentChecksum,
    ContentMode,
    ContentAnchor,
    ContentOverflow,
    ContentExact,
    WatchOp,
    WatchIdentity,
    WatchExact,
    WatchLimit,
    Route,
}
impl std::fmt::Display for RouteFixed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::SnapshotOp => "QSnapshotInvalid：op必须是snapshot",
            Self::SnapshotVersion => "QSnapshotInvalid：snapshot_token版本不符",
            Self::SnapshotDigest => "QSnapshotInvalid：签发捕获摘要不是64位小写hex",
            Self::SnapshotChecksum => "QSnapshotInvalid：snapshot_token checksum不符",
            Self::SnapshotMode => "QSnapshotInvalid：history_mode不符",
            Self::SnapshotExact => "QSnapshotInvalid：exact字段不完整或多余",
            Self::SnapshotLimit => "QSnapshotInvalid：Incomplete：请求超过冻结frame/page_size上限",
            Self::ContentOp => "QContentInvalid：op必须是content",
            Self::ContentVersion => "QContentInvalid：content_token版本不符",
            Self::ContentDigest => "QContentInvalid：签发摘要须为64位小写hex",
            Self::ContentChecksum => "QContentInvalid：content_token checksum不符",
            Self::ContentMode => "QContentInvalid：history_mode不符",
            Self::ContentAnchor => "QContentInvalid：anchor kind不符；不可用裸Hash访问",
            Self::ContentOverflow => "QContentInvalid：offset+limit溢出",
            Self::ContentExact => "QContentInvalid：exact字段不完整或多余",
            Self::WatchOp => "QWatchInvalid：op必须是watch",
            Self::WatchIdentity => "QWatchInvalid：外层消息与cursor身份不同",
            Self::WatchExact => "QWatchInvalid：exact字段不符",
            Self::WatchLimit => "QWatchInvalid：Incomplete：Watch请求超过冻结frame/max_batches界",
            Self::Route => "QRouteInvalid：不是已声明的v3查询路由",
        })
    }
}
#[derive(Debug)]
pub(crate) enum AdmissionError<'a> {
    Fixed(RouteFixed),
    Protocol(newchan_rust::session_protocol::ProtocolError<'a>),
    Allocation(sc::CodecError),
    ContentAllocation(sc::CodecError),
    SnapshotFields(sc::CodecError),
    ContentFields(sc::CodecError),
    Ref(sc::RefDiagnostic<'a>),
    Path(sc::fragment::PathDiagnostic<'a>),
}
impl<'a> From<newchan_rust::session_protocol::ProtocolError<'a>> for AdmissionError<'a> {
    fn from(e: newchan_rust::session_protocol::ProtocolError<'a>) -> Self {
        Self::Protocol(e)
    }
}
impl std::fmt::Display for AdmissionError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixed(e) => std::fmt::Display::fmt(e, f),
            Self::Protocol(e) => std::fmt::Display::fmt(e, f),
            Self::Allocation(e) => std::fmt::Display::fmt(e, f),
            Self::ContentAllocation(e) | Self::ContentFields(e) => {
                write!(f, "QContentInvalid：{e}")
            }
            Self::SnapshotFields(e) => write!(f, "QSnapshotInvalid：{e}"),
            Self::Ref(e) => write!(f, "QContentInvalid：{e}"),
            Self::Path(e) => write!(f, "QContentInvalid：{e}"),
        }
    }
}
impl AdmissionError<'_> {
    pub fn typed_budget(&self) -> bool {
        matches!(
            self,
            Self::Allocation(_)
                | Self::ContentAllocation(_)
                | Self::Protocol(newchan_rust::session_protocol::ProtocolError::Allocation(_))
                | Self::Protocol(newchan_rust::session_protocol::ProtocolError::Fixed(
                    newchan_rust::session_protocol::EnvelopeFailure::HashUnavailable
                ))
        )
    }
}
/// 沿完整Display逐字查旧五类字符串，动态用户输入仍保留原HTTP选择语义。
pub(crate) fn legacy_budget(detail: &dyn std::fmt::Display) -> bool {
    struct Search<'a> {
        needle: &'a [u8],
        window: [u8; 32],
        used: usize,
        found: bool,
    }
    impl std::fmt::Write for Search<'_> {
        fn write_str(&mut self, text: &str) -> std::fmt::Result {
            for byte in text.bytes() {
                if self.used == self.needle.len() {
                    self.window.copy_within(1..self.used, 0);
                    self.used -= 1;
                }
                self.window[self.used] = byte;
                self.used += 1;
                self.found |= &self.window[..self.used] == self.needle;
            }
            Ok(())
        }
    }
    [
        "Incomplete",
        "CaptureDeadline",
        "VerifyDeadline",
        "CaptureBytes",
        "atomic_delta_limit",
    ]
    .iter()
    .any(|needle| {
        let mut out = Search {
            needle: needle.as_bytes(),
            window: [0; 32],
            used: 0,
            found: false,
        };
        let _ = std::fmt::write(&mut out, format_args!("{detail}"));
        out.found
    })
}
/// payload必须由已核切面生成；源namespace/消息ID/因果回引沿共同Q配方。
#[cfg(test)]
pub(super) fn reply(
    request: &Value,
    payload: Value,
    session: &str,
    incarnation: &str,
    epoch: &str,
    resources: &QResources,
    job: &mut dyn Access,
) -> Result<Vec<u8>, String> {
    reply_view(
        request.into(),
        (&payload).into(),
        session,
        incarnation,
        epoch,
        resources,
        job,
        None,
    )?
    .try_into_plain()
    .map_err(|e| e.to_string())
}
#[allow(
    dead_code,
    reason = "#1608：无预算/普通值的旧包装入口，调用方已改走带 owner/admission/typed 的版本；先量再动，保留不删"
)]
pub(super) fn try_reply(
    request: &Value,
    payload: Value,
    session: &str,
    incarnation: &str,
    epoch: &str,
    resources: &QResources,
    job: &mut dyn Access,
) -> Result<Option<Vec<u8>>, String> {
    try_reply_view(
        request.into(),
        (&payload).into(),
        session,
        incarnation,
        epoch,
        resources,
        job,
        None,
    )?
    .map(|v| v.try_into_plain().map_err(|e| e.to_string()))
    .transpose()
}
fn err(e: impl std::fmt::Display) -> String {
    format!("QReplyInvalid：{e}")
}

use super::business_json::{self as bj};
use newchan_rust::s_content::{
    self as sc,
    capture_allocation::{audit_json::JsonView, OwnedBudget},
};
#[allow(
    dead_code,
    reason = "#1608：无预算/普通值的旧包装入口，调用方已改走带 owner/admission/typed 的版本；先量再动，保留不删"
)]
pub(super) fn encode(
    value: &Value,
    limit: u64,
    job: &mut dyn Access,
) -> Result<Option<Vec<u8>>, String> {
    encode_view(value.into(), limit, job, None)?
        .map(|v| v.try_into_plain().map_err(|e| e.to_string()))
        .transpose()
}
pub(crate) fn encode_view(
    value: JsonView<'_>,
    limit: u64,
    job: &mut dyn Access,
    allocation: Option<&OwnedBudget>,
) -> Result<Option<sc::CodecVec<u8>>, String> {
    #[cfg(not(feature = "s_allocation_json_testonly"))]
    if allocation.is_some() {
        return Err("prototype_audit_allocation_incomplete：owned Q编码未启用".into());
    }
    struct Buffer<'a> {
        bytes: sc::CodecVec<u8>,
        limit: u64,
        job: &'a mut dyn Access,
        exceeded: bool,
        cause: Option<String>,
    }
    impl Write for Buffer<'_> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if let Err(e) = self.job.pre_io_check() {
                self.cause = Some(e.to_string());
                return Err(io::ErrorKind::Other.into());
            }
            if self.bytes.len() as u128 + bytes.len() as u128 > self.limit as u128 {
                self.exceeded = true;
                return Err(io::ErrorKind::Other.into());
            }
            if let Err(e) = self.bytes.try_extend_bytes(bytes) {
                self.cause = Some(e.to_string());
                return Err(io::ErrorKind::Other.into());
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut out = Buffer {
        bytes: bj::new_work(allocation),
        limit,
        job,
        exceeded: false,
        cause: None,
    };
    let result = newchan_rust::s_content::json_writer::write_json(&mut out, &value);
    out.job.pre_io_check().map_err(err)?;
    if let Some(cause) = out.cause {
        return Err(err(cause));
    }
    if out.exceeded {
        return Ok(None);
    }
    result.map_err(err)?;
    Ok(Some(out.bytes))
}
fn hex(digest: [u8; 32]) -> [u8; 64] {
    let mut out = [0; 64];
    let alphabet = b"0123456789abcdef";
    for (i, b) in digest.into_iter().enumerate() {
        out[2 * i] = alphabet[(b >> 4) as usize];
        out[2 * i + 1] = alphabet[(b & 15) as usize];
    }
    out
}
pub(crate) fn reply_view(
    request: JsonView<'_>,
    payload: JsonView<'_>,
    session: &str,
    incarnation: &str,
    epoch: &str,
    resources: &QResources,
    job: &mut dyn Access,
    allocation: Option<&OwnedBudget>,
) -> Result<sc::CodecVec<u8>, String> {
    try_reply_view(
        request,
        payload,
        session,
        incarnation,
        epoch,
        resources,
        job,
        allocation,
    )?
    .ok_or_else(|| err("Incomplete：完整Q消息体超过max_reply_bytes"))
}
pub(crate) fn try_reply_view(
    request: JsonView<'_>,
    payload: JsonView<'_>,
    session: &str,
    incarnation: &str,
    epoch: &str,
    resources: &QResources,
    job: &mut dyn Access,
    allocation: Option<&OwnedBudget>,
) -> Result<Option<sc::CodecVec<u8>>, String> {
    positive(epoch, "Q producer_epoch")?;
    if encode_view(payload, resources.limit(Limit::ReplyBytes), job, allocation)?.is_none() {
        return Ok(None);
    }
    let hash = hex(bj::sha_json(payload, allocation)?);
    let hash = std::str::from_utf8(&hash).expect("ASCII digest");
    let id_material = [
        bj::field(request, "source_namespace"),
        bj::field(request, "source_epoch"),
        bj::field(request, "message_id"),
        JsonView::String(hash),
    ];
    let id_hash = hex(bj::sha_json(
        JsonView::ArraySlice(&id_material),
        allocation,
    )?);
    let mut id = [0; 70];
    id[..6].copy_from_slice(b"reply-");
    id[6..].copy_from_slice(&id_hash);
    let id = std::str::from_utf8(&id).expect("ASCII reply ID");
    let mut producer = bj::new_work(allocation);
    producer.try_extend_bytes(b"Q:").map_err(err)?;
    producer.try_extend_bytes(session.as_bytes()).map_err(err)?;
    let producer = bj::TextValue::from_bytes(producer).map_err(newchan_rust::s_content::ContextError::into_message)?;
    let causal_fields = [
        ("source_namespace", bj::field(request, "source_namespace")),
        ("source_epoch", bj::field(request, "source_epoch")),
        ("message_id", bj::field(request, "message_id")),
        ("payload_hash", bj::field(request, "payload_hash")),
    ];
    let causal = [JsonView::object_fields(&causal_fields).map_err(err)?];
    let fields = [
        ("schema_revision", JsonView::String("s-session/3")),
        ("session_id", JsonView::String(session)),
        ("session_generation", JsonView::String(incarnation)),
        ("source_namespace", JsonView::String("s-observe/replies")),
        ("source_epoch", JsonView::String(epoch)),
        ("message_id", JsonView::String(id)),
        ("producer_id", JsonView::String(&producer)),
        ("producer_epoch", JsonView::String(epoch)),
        ("payload_hash", JsonView::String(hash)),
        ("causal_refs", JsonView::ArraySlice(&causal)),
        ("payload", payload),
    ];
    encode_view(
        JsonView::object_fields(&fields).map_err(err)?,
        resources.limit(Limit::ReplyBytes),
        job,
        allocation,
    )
}

/// 错误公头只借用已核请求/身份。摘要与producer前缀由固定栈值生成，
/// 无论None/Some都用本序列化核，预算拒绝不会转回普通JSON树或Vec。
pub(crate) struct ErrorEnvelope<'a, P: serde::Serialize + ?Sized> {
    request: JsonView<'a>,
    payload: &'a P,
    session: &'a str,
    incarnation: &'a str,
    epoch: &'a str,
    payload_hash: [u8; 64],
    message_id: [u8; 70],
}
impl<'a, P: serde::Serialize + ?Sized> ErrorEnvelope<'a, P> {
    pub(crate) fn new(
        request: JsonView<'a>,
        payload: &'a P,
        session: &'a str,
        incarnation: &'a str,
        epoch: &'a str,
    ) -> Result<Self, String> {
        positive(epoch, "Q producer_epoch")?;
        let payload_hash = hex(stream_hash(&payload)?);
        let id_material = [
            bj::field(request, "source_namespace"),
            bj::field(request, "source_epoch"),
            bj::field(request, "message_id"),
            JsonView::String(std::str::from_utf8(&payload_hash).expect("ASCII digest")),
        ];
        let id_hash = hex(stream_hash(&JsonView::ArraySlice(&id_material))?);
        let mut message_id = [0; 70];
        message_id[..6].copy_from_slice(b"reply-");
        message_id[6..].copy_from_slice(&id_hash);
        Ok(Self {
            request,
            payload,
            session,
            incarnation,
            epoch,
            payload_hash,
            message_id,
        })
    }
}
fn stream_hash(value: &(impl serde::Serialize + ?Sized)) -> Result<[u8; 32], String> {
    use sha2::{Digest, Sha256};
    struct HashWriter(Sha256);
    impl Write for HashWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut out = HashWriter(Sha256::new());
    newchan_rust::s_content::json_writer::write_json(&mut out, value).map_err(err)?;
    Ok(out.0.finalize().into())
}
impl<P: serde::Serialize + ?Sized> serde::Serialize for ErrorEnvelope<'_, P> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        struct Producer<'a>(&'a str);
        impl std::fmt::Display for Producer<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("Q:")?;
                f.write_str(self.0)
            }
        }
        impl serde::Serialize for Producer<'_> {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                // 受控json_writer逐段转义Display；保持旧serde_json compact字节。
                serializer.collect_str(self)
            }
        }
        let causal_fields = [
            (
                "source_namespace",
                bj::field(self.request, "source_namespace"),
            ),
            ("source_epoch", bj::field(self.request, "source_epoch")),
            ("message_id", bj::field(self.request, "message_id")),
            ("payload_hash", bj::field(self.request, "payload_hash")),
        ];
        // 四个键是上方固定且互异的字面量；object_fields的重复键错误在此不可达。
        // 保留原custom诊断分支，不削减异常信息。
        let causal = [JsonView::object_fields(&causal_fields).map_err(serde::ser::Error::custom)?];
        // 与reply_view的JsonView UTF8键排序逐字一致。
        let mut map = serializer.serialize_map(Some(11))?;
        map.serialize_entry("causal_refs", &causal)?;
        map.serialize_entry(
            "message_id",
            std::str::from_utf8(&self.message_id).expect("ASCII reply ID"),
        )?;
        map.serialize_entry("payload", &self.payload)?;
        map.serialize_entry(
            "payload_hash",
            std::str::from_utf8(&self.payload_hash).expect("ASCII digest"),
        )?;
        map.serialize_entry("producer_epoch", self.epoch)?;
        map.serialize_entry("producer_id", &Producer(self.session))?;
        map.serialize_entry("schema_revision", "s-session/3")?;
        map.serialize_entry("session_generation", self.incarnation)?;
        map.serialize_entry("session_id", self.session)?;
        map.serialize_entry("source_epoch", self.epoch)?;
        map.serialize_entry("source_namespace", "s-observe/replies")?;
        map.end()
    }
}
