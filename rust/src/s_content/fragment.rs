//! #1394 C §5.6：从调用方已核 anchor 根取得片段，接收方仅凭本片证明重新核范围。
//! 本层不授予裸根读取权限，不生成 token，不证明未读子图或业务来源。
//! 消费者的同一Job计唯一接纳节点与绝对期限；服务端沿自己的资源门读取。
//! wire/request/六族顶层记录及磁盘分配由实际调用层计；接收验证仍强制消费者Job。

#[cfg(feature = "s_allocation_json_testonly")]
use super::capture_allocation::audit_json::OwnedJson;
use super::capture_allocation::work::WorkMap;
use super::capture_allocation::{
    audit_json::{JsonArray, JsonObject, JsonView},
    OwnedBudget,
};
use super::{
    access::Access,
    job::{CheckedFrame, Job},
    path::{self, PathStep},
    *,
};
use serde_json::{json, Value as J};

pub const INVALID: &str = "content_fragment_invalid";
pub const REPLY_LIMIT: &str = "content_fragment_reply_limit";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Items,
    Bytes,
    Value,
}
impl Unit {
    pub fn name(self) -> &'static str {
        match self {
            Self::Items => "items",
            Self::Bytes => "bytes",
            Self::Value => "value",
        }
    }
    pub fn parse(s: &str) -> Result<Self, CodecError> {
        Self::parse_typed(s).map_err(|e| e.into_codec())
    }
    pub fn parse_typed(s: &str) -> Result<Self, PathDiagnostic<'static>> {
        match s {
            "items" => Ok(Self::Items),
            "bytes" => Ok(Self::Bytes),
            "value" => Ok(Self::Value),
            _ => Err(PathDiagnostic::Fixed(PathFailure::Unit)),
        }
    }
}

/// 证明节点的 exact transport 三字段；不是已完成子图的证书。
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ProofNode {
    pub root: String,
    pub tag: String,
    pub body_hex: String,
}
impl ProofNode {
    pub fn to_json(&self) -> J {
        json!({"root":self.root,"tag":self.tag,"body_hex":self.body_hex})
    }
    pub fn from_json(v: &J) -> Result<Self, CodecError> {
        exact(v, &["root", "tag", "body_hex"])?;
        let text = |k| {
            v[k].as_str()
                .map(str::to_owned)
                .ok_or_else(|| err("证明字段须为字符串"))
        };
        let node = Self {
            root: text("root")?,
            tag: text("tag")?,
            body_hex: text("body_hex")?,
        };
        node.decode()?;
        Ok(node)
    }
    fn decode(&self) -> Result<(NodeDigest, u8, Vec<u8>), CodecError> {
        let digest = parse_root(&self.root)?;
        let tag = unhex(&self.tag)?;
        if tag.len() != 1 {
            return Err(err("tag须为两位小写十六进制"));
        }
        if !matches!(tag[0], 0x00..=0x08 | 0x10..=0x14) {
            return Err(err("未知节点标签"));
        }
        let body = unhex(&self.body_hex)?;
        if frame(tag[0], &body) != digest {
            return Err(CodecError::new(code::NODE_DIGEST, "证明节点摘要不符"));
        }
        Ok((digest, tag[0], body))
    }
}

/// 外层添加真实anchor/path、token和本次capture摘要，组成规范13字段响应。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fragment {
    pub target_ref: ContentRef,
    pub unit: Unit,
    pub offset: u64,
    pub end: u64,
    pub total: u64,
    pub items: J,
    pub nodes: Vec<ProofNode>,
    pub done: bool,
}
impl Fragment {
    fn encoded_size(&self) -> Result<u64, CodecError> {
        #[derive(serde::Serialize)]
        struct View<'a> {
            target_ref: J,
            unit: &'a str,
            offset: String,
            end: String,
            total: String,
            items: &'a J,
            nodes: &'a [ProofNode],
            done: bool,
        }
        encoded_len(&View {
            target_ref: self.target_ref.to_transport_json(),
            unit: self.unit.name(),
            offset: self.offset.to_string(),
            end: self.end.to_string(),
            total: self.total.to_string(),
            items: &self.items,
            nodes: &self.nodes,
            done: self.done,
        })
    }
    pub fn fields(&self) -> serde_json::Map<String, J> {
        json!({"target_ref":self.target_ref.to_transport_json(),"unit":self.unit.name(),"offset":self.offset.to_string(),"end":self.end.to_string(),"total":self.total.to_string(),"items":self.items,"nodes":self.nodes.iter().map(ProofNode::to_json).collect::<Vec<_>>(),"done":self.done}).as_object().unwrap().clone()
    }
}

/// Fragment 的实际 JSON/文本/数组所有者；只有普通读取可消费成旧的 std Fragment。
/// fixed 数字和 RefOwner 在栈内；动态结果、证明文本与每个 array layout 跟随来源账本。
pub struct OwnedFragment {
    pub target_ref: RefOwner,
    pub unit: Unit,
    pub offset: u64,
    pub end: u64,
    pub total: u64,
    pub done: bool,
    items: FragmentValue,
    nodes: WorkVec<OwnedProofNode>,
    target_wire: ReferenceWire,
    offsets: [Decimal; 3],
}
impl OwnedFragment {
    pub fn fields_view(&self) -> Result<JsonView<'_>, CodecError> {
        JsonView::borrowed_object(self)
    }
    pub fn allocation_budget(&self) -> Option<&OwnedBudget> {
        self.nodes.allocation_budget()
    }
    pub fn into_plain(self) -> Result<Fragment, CodecError> {
        let items = self.items.into_plain()?;
        let mut nodes = Vec::new();
        for node in self.nodes.into_plain()? {
            nodes.push(ProofNode {
                root: node.root.into_plain()?,
                tag: node.tag.into_plain()?,
                body_hex: node.body_hex.into_plain()?,
            });
        }
        Ok(Fragment {
            target_ref: ContentRef::new(
                self.target_ref.kind(),
                &self.target_ref.digest(),
                self.target_ref.extent(),
            ),
            unit: self.unit,
            offset: self.offset,
            end: self.end,
            total: self.total,
            items,
            nodes,
            done: self.done,
        })
    }
}
impl JsonObject for OwnedFragment {
    fn len(&self) -> usize {
        8
    }
    fn field(&self, index: usize) -> Option<(&str, JsonView<'_>)> {
        Some(match index {
            0 => ("target_ref", self.target_wire.view()),
            1 => ("unit", JsonView::String(self.unit.name())),
            2 => ("offset", JsonView::String(self.offsets[0].text())),
            3 => ("end", JsonView::String(self.offsets[1].text())),
            4 => ("total", JsonView::String(self.offsets[2].text())),
            5 => ("items", self.items.view()),
            6 => ("nodes", JsonView::BorrowedArray(&self.nodes)),
            7 => ("done", JsonView::Bool(self.done)),
            _ => return None,
        })
    }
}
struct ReferenceWire {
    reference: RefOwner,
    extent: Option<Decimal>,
}
impl ReferenceWire {
    fn new(reference: RefOwner) -> Self {
        Self {
            extent: reference.extent().map(Decimal::new),
            reference,
        }
    }
    fn view(&self) -> JsonView<'_> {
        JsonView::borrowed_object(self).expect("fixed exact ref fields")
    }
}
impl JsonObject for ReferenceWire {
    fn len(&self) -> usize {
        4
    }
    fn field(&self, index: usize) -> Option<(&str, JsonView<'_>)> {
        Some(match index {
            0 => ("format", JsonView::String(self.reference.format())),
            1 => ("root", JsonView::String(self.reference.root())),
            2 => ("kind", JsonView::String(self.reference.kind().name())),
            3 => (
                "extent",
                self.extent
                    .as_ref()
                    .map_or(JsonView::Null, |v| JsonView::String(v.text())),
            ),
            _ => return None,
        })
    }
}
struct Decimal {
    bytes: [u8; 20],
    start: usize,
}
impl Decimal {
    fn new(mut value: u64) -> Self {
        let mut out = Self {
            bytes: [0; 20],
            start: 20,
        };
        loop {
            out.start -= 1;
            out.bytes[out.start] = b'0' + (value % 10) as u8;
            value /= 10;
            if value == 0 {
                return out;
            }
        }
    }
    fn text(&self) -> &str {
        std::str::from_utf8(&self.bytes[self.start..]).expect("decimal ASCII")
    }
}
struct FragmentText {
    bytes: WorkVec<u8>,
}
impl FragmentText {
    fn copy(text: &str, budget: Option<&OwnedBudget>) -> Result<Self, CodecError> {
        let mut bytes = WorkVec::new(budget);
        bytes.extend_from_slice(text.as_bytes())?;
        Ok(Self { bytes })
    }
    fn hex(raw: &[u8], budget: Option<&OwnedBudget>) -> Result<Self, CodecError> {
        let mut bytes = WorkVec::new(budget);
        let count = raw.len().checked_mul(2).ok_or_else(|| err("hex长度溢出"))?;
        bytes.reserve_exact(count)?;
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for byte in raw {
            bytes.extend_from_slice(&[HEX[(byte >> 4) as usize], HEX[(byte & 15) as usize]])?;
        }
        Ok(Self { bytes })
    }
    fn text(&self) -> &str {
        std::str::from_utf8(&self.bytes).expect("private UTF8 text")
    }
    fn into_plain(self) -> Result<String, CodecError> {
        String::from_utf8(self.bytes.into_plain()?).map_err(|_| super::utf8_error())
    }
}
struct OwnedProofNode {
    root: FragmentText,
    tag: FragmentText,
    body_hex: FragmentText,
}
impl OwnedProofNode {
    fn new(
        digest: &NodeDigest,
        tag: u8,
        body: &[u8],
        budget: Option<&OwnedBudget>,
    ) -> Result<Self, CodecError> {
        let reference = RefOwner::new(Kind::Null, *digest, None);
        Ok(Self {
            root: FragmentText::copy(reference.root(), budget)?,
            tag: FragmentText::hex(&[tag], budget)?,
            body_hex: FragmentText::hex(body, budget)?,
        })
    }
    fn view(&self) -> JsonView<'_> {
        JsonView::borrowed_object(self).expect("fixed proof fields")
    }
}
impl JsonObject for OwnedProofNode {
    fn len(&self) -> usize {
        3
    }
    fn field(&self, index: usize) -> Option<(&str, JsonView<'_>)> {
        Some(match index {
            0 => ("root", JsonView::String(self.root.text())),
            1 => ("tag", JsonView::String(self.tag.text())),
            2 => ("body_hex", JsonView::String(self.body_hex.text())),
            _ => return None,
        })
    }
}
impl JsonArray for WorkVec<OwnedProofNode> {
    fn len(&self) -> usize {
        self[..].len()
    }
    fn get(&self, index: usize) -> Option<JsonView<'_>> {
        self[..].get(index).map(OwnedProofNode::view)
    }
}
enum FragmentValue {
    Plain(J),
    #[cfg(feature = "s_allocation_json_testonly")]
    Owned(OwnedJson),
}
impl FragmentValue {
    fn from_view(value: JsonView<'_>, budget: Option<&OwnedBudget>) -> Result<Self, CodecError> {
        if let Some(budget) = budget {
            #[cfg(feature = "s_allocation_json_testonly")]
            {
                return OwnedJson::from_view(value, budget).map(Self::Owned);
            }
            #[cfg(not(feature = "s_allocation_json_testonly"))]
            {
                let _ = budget;
                return Err(CodecError::static_detail(
                    capture_allocation::INCOMPLETE,
                    "owned fragment JSON未启用",
                ));
            }
        }
        serde_json::to_value(value).map(Self::Plain).map_err(err)
    }
    fn view(&self) -> JsonView<'_> {
        match self {
            Self::Plain(v) => v.into(),
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(v) => v.view(),
        }
    }
    fn push(&mut self, value: JsonView<'_>) -> Result<(), CodecError> {
        match self {
            Self::Plain(J::Array(rows)) => {
                rows.push(serde_json::to_value(value).map_err(err)?);
                Ok(())
            }
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(rows) => rows.push(value),
            _ => Err(err("items非数组")),
        }
    }
    fn into_plain(self) -> Result<J, CodecError> {
        match self {
            Self::Plain(v) => Ok(v),
            #[cfg(feature = "s_allocation_json_testonly")]
            Self::Owned(_) => Err(CodecError::static_detail(
                capture_allocation::INCOMPLETE,
                "owned fragment不能擦除为std JSON",
            )),
        }
    }
}

/// transport path 的唯一 exact 解析：field / index / json_text 三者只取其一。
pub fn parse_path(value: &J) -> Result<Vec<PathStep>, CodecError> {
    use path::PathView;
    let path = parse_path_view(value.into())?;
    (0..path.len())
        .map(|i| match path.step(i).expect("checked immutable path") {
            path::PathStepView::Field(v) => Ok(PathStep::Field(v.into())),
            path::PathStepView::Index(i) => Ok(PathStep::Index(i)),
            path::PathStepView::JsonText => Ok(PathStep::JsonText),
        })
        .collect()
}
/// 私有有效域来自本次逐项exact解析；只借请求JSON的字段文本。
pub struct WirePath<'a> {
    value: JsonView<'a>,
}
impl path::PathView for WirePath<'_> {
    fn len(&self) -> usize {
        self.value.as_array().expect("checked path array").len()
    }
    fn step(&self, i: usize) -> Option<path::PathStepView<'_>> {
        self.value
            .index(i)
            .map(|v| parse_step(v).expect("checked immutable path step"))
    }
}
#[derive(Debug)]
pub enum PathFailure {
    Unit,
    Array,
    Fields,
    FieldText,
    IndexText,
    Unknown,
}
impl std::fmt::Display for PathFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unit => "未知unit",
            Self::Array => "path须为数组",
            Self::Fields => "path步骤须exact单字段",
            Self::FieldText => "field须为UTF-8字符串",
            Self::IndexText => "index须为规范非负整数文本",
            Self::Unknown => "未知path步骤或json_text非true",
        })
    }
}
#[derive(Debug)]
pub enum PathDiagnostic<'a> {
    Fixed(PathFailure),
    Decimal(super::DecimalError<'a>),
}
impl std::fmt::Display for PathDiagnostic<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixed(s) => write!(f, "{INVALID}: {s}"),
            Self::Decimal(e) => std::fmt::Display::fmt(e, f),
        }
    }
}
impl PathDiagnostic<'_> {
    fn into_codec(self) -> CodecError {
        match self {
            Self::Fixed(s) => CodecError::new(INVALID, s.to_string()),
            Self::Decimal(e) => {
                let rendered = e.to_string();
                let code = e.code();
                CodecError::new(
                    code,
                    rendered
                        .strip_prefix(code)
                        .unwrap()
                        .strip_prefix(": ")
                        .unwrap(),
                )
            }
        }
    }
}
pub fn parse_path_view(value: JsonView<'_>) -> Result<WirePath<'_>, CodecError> {
    parse_path_view_typed(value).map_err(|e| e.into_codec())
}
pub fn parse_path_view_typed(value: JsonView<'_>) -> Result<WirePath<'_>, PathDiagnostic<'_>> {
    let rows = value
        .as_array()
        .ok_or(PathDiagnostic::Fixed(PathFailure::Array))?;
    for row in rows.iter() {
        parse_step_typed(row)?;
    }
    Ok(WirePath { value })
}
fn parse_step(row: JsonView<'_>) -> Result<path::PathStepView<'_>, CodecError> {
    parse_step_typed(row).map_err(|e| e.into_codec())
}
fn parse_step_typed(row: JsonView<'_>) -> Result<path::PathStepView<'_>, PathDiagnostic<'_>> {
    let object = row
        .as_object()
        .filter(|o| o.len() == 1)
        .ok_or(PathDiagnostic::Fixed(PathFailure::Fields))?;
    if let Some(v) = object.get("field") {
        return v
            .as_str()
            .map(path::PathStepView::Field)
            .ok_or(PathDiagnostic::Fixed(PathFailure::FieldText));
    }
    if let Some(v) = object.get("index") {
        return Ok(path::PathStepView::Index(
            super::parse_canonical_u64_typed(
                v.as_str()
                    .ok_or(PathDiagnostic::Fixed(PathFailure::IndexText))?,
                "index",
            )
            .map_err(PathDiagnostic::Decimal)?,
        ));
    }
    if object.get("json_text").and_then(JsonView::as_bool) == Some(true) {
        return Ok(path::PathStepView::JsonText);
    }
    Err(PathDiagnostic::Fixed(PathFailure::Unknown))
}

/// 每片重新收齐其触碰到的证明，不能因调用方曾接纳过节点就省略祖先。
pub fn read(
    source: &mut dyn NodeSource,
    root: &ContentRef,
    path: &[PathStep],
    unit: Unit,
    offset: u64,
    limit: u64,
    max_reply_bytes: u64,
    access: &mut dyn Access,
) -> Result<Fragment, CodecError> {
    read_refview(
        source,
        root.into(),
        path,
        unit,
        offset,
        limit,
        max_reply_bytes,
        access,
    )
}

/// 普通查询路径的借用引用入口，与 `read` 共用范围/证明核；不授予 Closed 分配资格。
#[doc(hidden)]
pub fn read_refview(
    source: &mut dyn NodeSource,
    root: RefView<'_>,
    path: &[PathStep],
    unit: Unit,
    offset: u64,
    limit: u64,
    max_reply_bytes: u64,
    access: &mut dyn Access,
) -> Result<Fragment, CodecError> {
    read_with_filter(
        source,
        root,
        path,
        unit,
        offset,
        limit,
        max_reply_bytes,
        access,
        None,
    )
}

/// 具体captured owner入口；不接受动态source的预算标记，不擦掉Closed资格。
pub fn read_owned(
    source: &mut ClosedCapturedOwner<'_>,
    root: RefView<'_>,
    path: &dyn path::PathView,
    unit: Unit,
    offset: u64,
    limit: u64,
    max_reply_bytes: u64,
    access: &mut dyn Access,
) -> Result<OwnedFragment, CodecError> {
    read_kernel(
        CodecContext::Closed(source),
        root,
        path,
        unit,
        offset,
        limit,
        max_reply_bytes,
        access,
        None,
    )
}
/// 普通source也沿相同范围、证明、path与第一错误核，仅所有者表示选择None。
pub fn read_view(
    source: &mut dyn NodeSource,
    root: RefView<'_>,
    path: &dyn path::PathView,
    unit: Unit,
    offset: u64,
    limit: u64,
    max_reply_bytes: u64,
    access: &mut dyn Access,
) -> Result<OwnedFragment, CodecError> {
    read_kernel(
        CodecContext::Source(source),
        root,
        path,
        unit,
        offset,
        limit,
        max_reply_bytes,
        access,
        None,
    )
}
fn read_with_filter(
    source: &mut dyn NodeSource,
    root: RefView<'_>,
    path: &[PathStep],
    unit: Unit,
    offset: u64,
    limit: u64,
    max_reply_bytes: u64,
    access: &mut dyn Access,
    record_filter: Option<fn(&[PathStep]) -> bool>,
) -> Result<Fragment, CodecError> {
    read_kernel(
        CodecContext::Source(source),
        root,
        &path,
        unit,
        offset,
        limit,
        max_reply_bytes,
        access,
        record_filter,
    )?
    .into_plain()
}
fn read_kernel<'read, 'capture>(
    source: CodecContext<'read, 'capture>,
    root: RefView<'_>,
    path: &dyn path::PathView,
    unit: Unit,
    offset: u64,
    limit: u64,
    max_reply_bytes: u64,
    access: &'read mut dyn Access,
    record_filter: Option<fn(&[PathStep]) -> bool>,
) -> Result<OwnedFragment, CodecError> {
    access.begin_first_read()?;
    if limit == 0
        || offset > MAX_FRAME_LEN
        || limit > MAX_FRAME_LEN
        || max_reply_bytes == 0
        || max_reply_bytes > MAX_FRAME_LEN
    {
        return Err(err("offset/limit越域或limit为零"));
    }
    let requested_end = offset
        .checked_add(limit)
        .ok_or_else(|| err("offset+limit溢出u64"))?;
    let budget = source.budget();
    let mut source = Recorder {
        source,
        access,
        nodes: WorkMap::new(budget.as_ref()),
        reply_bytes: 0,
        max_reply_bytes,
        record_filter,
    };
    let target: RefOwner =
        path::lookup_path_view_output(&mut CodecContext::Fragment(&mut source), root, path, false)?;
    let total = match (target.kind(), unit) {
        (Kind::Array | Kind::Object, Unit::Items) | (Kind::String | Kind::Number, Unit::Bytes) => {
            target.extent().ok_or_else(|| err("目标缺extent"))?
        }
        (Kind::Null | Kind::Bool | Kind::JsonText, Unit::Value) if limit == 1 => 1,
        _ => return Err(err("目标类型/unit/limit不匹配；公开值不允许模板占位")),
    };
    if offset > total {
        return Err(err("offset超过total"));
    }
    let end = requested_end.min(total);
    let target_wire = ReferenceWire::new(target);
    let offsets = [Decimal::new(offset), Decimal::new(end), Decimal::new(total)];
    // 空值只借栈内组合计算长度，动态items/证明所有者随后按实际来源选择。
    let initial_items = if unit == Unit::Bytes {
        JsonView::String("")
    } else {
        JsonView::ArraySlice(&[])
    };
    let empty_fields = [
        ("target_ref", target_wire.view()),
        ("unit", JsonView::String(unit.name())),
        ("offset", JsonView::String(offsets[0].text())),
        ("end", JsonView::String(offsets[1].text())),
        ("total", JsonView::String(offsets[2].text())),
        ("items", initial_items),
        ("nodes", JsonView::ArraySlice(&[])),
        ("done", JsonView::Bool(end == total)),
    ];
    source.reserve_reply(encoded_len(&JsonView::object_fields(&empty_fields)?)?)?;
    path::check_range_root_view_ctx(&mut CodecContext::Fragment(&mut source), target.view())?;
    let mut items = FragmentValue::from_view(initial_items, budget.as_ref())?;
    match target.kind() {
        Kind::Array => {
            for i in offset..end {
                let steps = path::ExtendedPath {
                    base: path,
                    last: path::PathStepView::Index(i),
                };
                let child: RefOwner = path::lookup_path_view_output(
                    &mut CodecContext::Fragment(&mut source),
                    root,
                    &steps,
                    false,
                )?;
                public_header_view(child.view())?;
                let wire = ReferenceWire::new(child);
                source.reserve_reply(encoded_len(&wire.view())? + u64::from(i != offset))?;
                items.push(wire.view())?;
            }
        }
        Kind::Object => {
            for i in offset..end {
                let key_ref: RefOwner = path::object_key_at_view_ctx(
                    &mut CodecContext::Fragment(&mut source),
                    target.view(),
                    i,
                )?;
                let key = source.read_key(key_ref.view())?;
                let steps = path::ExtendedPath {
                    base: path,
                    last: path::PathStepView::Field(key.text()),
                };
                let child: RefOwner = path::lookup_path_view_output(
                    &mut CodecContext::Fragment(&mut source),
                    root,
                    &steps,
                    false,
                )?;
                public_header_view(child.view())?;
                let wire = ReferenceWire::new(child);
                let fields = [
                    ("key", JsonView::String(key.text())),
                    ("value_ref", wire.view()),
                ];
                let item = JsonView::object_fields(&fields)?;
                source.reserve_reply(encoded_len(&item)? + u64::from(i != offset))?;
                items.push(item)?;
            }
        }
        Kind::String | Kind::Number => {
            source.reserve_reply(
                (end - offset)
                    .checked_mul(2)
                    .ok_or_else(|| err("回复字节溢出"))?,
            )?;
            let bytes = path::bytes_between_view_ctx(
                &mut CodecContext::Fragment(&mut source),
                target.view(),
                offset,
                end,
            )?;
            let text = FragmentText::hex(&bytes, budget.as_ref())?;
            items = FragmentValue::from_view(JsonView::String(text.text()), budget.as_ref())?;
        }
        _ => {
            if offset != end {
                source.reserve_reply(encoded_len(&target_wire.view())?)?;
                items.push(target_wire.view())?;
            }
        }
    }
    source.access.pre_io_check()?;
    let mut roots = WorkVec::new(budget.as_ref());
    for (digest, _) in source.nodes.iter() {
        roots.push(*digest)?;
    }
    roots.sort_unstable();
    let mut nodes = WorkVec::new(budget.as_ref());
    for digest in roots {
        let (tag, body) = source.nodes.remove(&digest).expect("recorded root");
        nodes.push(OwnedProofNode::new(&digest, tag, &body, budget.as_ref())?)?;
    }
    Ok(OwnedFragment {
        target_ref: target,
        unit,
        offset,
        end,
        total,
        items,
        nodes,
        done: end == total,
        target_wire,
        offsets,
    })
}

/// 只用本片自带节点重新走同一规范路径/范围；不从接收方旧缓存补缺证明。
/// 接纳全部提供节点时仍计本Job唯一节点。额外节点不因此取得anchor归属或完整语义证明。
pub fn verify(
    root: &ContentRef,
    path: &[PathStep],
    unit: Unit,
    offset: u64,
    limit: u64,
    fragment: &Fragment,
    max_reply_bytes: u64,
    job: &mut Job,
) -> Result<(), CodecError> {
    verify_impl(
        root,
        path,
        unit,
        offset,
        limit,
        fragment,
        max_reply_bytes,
        job,
        None,
    )
}

/// 同一严格范围核验，额外在真实路径遍历到数组 Index 前由调用方指定
/// 哪些语义路径属于需计费的顶层记录。无额外读取，不按回复声称的 items 计。
pub fn verify_with_record_filter(
    root: &ContentRef,
    path: &[PathStep],
    unit: Unit,
    offset: u64,
    limit: u64,
    fragment: &Fragment,
    max_reply_bytes: u64,
    job: &mut Job,
    record_filter: fn(&[PathStep]) -> bool,
) -> Result<(), CodecError> {
    verify_impl(
        root,
        path,
        unit,
        offset,
        limit,
        fragment,
        max_reply_bytes,
        job,
        Some(record_filter),
    )
}

fn verify_impl(
    root: &ContentRef,
    path: &[PathStep],
    unit: Unit,
    offset: u64,
    limit: u64,
    fragment: &Fragment,
    max_reply_bytes: u64,
    job: &mut Job,
    record_filter: Option<fn(&[PathStep]) -> bool>,
) -> Result<(), CodecError> {
    job.begin_first_read()?;
    if max_reply_bytes == 0 || max_reply_bytes > MAX_FRAME_LEN {
        return Err(err("回复上界越域"));
    }
    if fragment.encoded_size()? > max_reply_bytes {
        return Err(CodecError::new(REPLY_LIMIT, "片段字段超过实际回复上界"));
    }
    let mut nodes = MemoryNodes::new();
    let mut previous = None;
    for node in &fragment.nodes {
        job.pre_io_check()?;
        let (digest, tag, body) = node.decode()?;
        if previous.is_some_and(|p| p >= digest) {
            return Err(err("证明节点须按root升序且去重"));
        }
        job.accept_node(&digest, tag, &body)?;
        nodes.put(digest, tag, &body)?;
        previous = Some(digest);
    }
    let expected = read_with_filter(
        &mut nodes,
        root.into(),
        path,
        unit,
        offset,
        limit,
        max_reply_bytes,
        job,
        record_filter,
    )?;
    if fragment.target_ref != expected.target_ref
        || fragment.unit != expected.unit
        || fragment.offset != expected.offset
        || fragment.end != expected.end
        || fragment.total != expected.total
        || fragment.items != expected.items
        || fragment.done != expected.done
    {
        return Err(err("片段内容/目标/位置与已给根的范围证明不同"));
    }
    job.pre_io_check()
}

/// 私有context适配器；外部NodeSource不能实现此trait或伪造Captured预算。
pub(super) trait FragmentRead<'capture> {
    fn budget(&self) -> Option<OwnedBudget>;
    fn read(&mut self, digest: &NodeDigest) -> Result<(u8, VerifierBody<'capture>), CodecError>;
    fn before_path_index(
        &mut self,
        prefix: &dyn path::PathView,
        end: usize,
        index: u64,
    ) -> Result<(), CodecError>;
}
struct Recorder<'read, 'capture> {
    source: CodecContext<'read, 'capture>,
    access: &'read mut dyn Access,
    nodes: WorkMap<NodeDigest, (u8, WorkVec<u8>)>,
    reply_bytes: u64,
    max_reply_bytes: u64,
    record_filter: Option<fn(&[PathStep]) -> bool>,
}
impl<'capture> Recorder<'_, 'capture> {
    fn read_key(&mut self, reference: RefView<'_>) -> Result<FragmentText, CodecError> {
        let remaining = self.max_reply_bytes - self.reply_bytes;
        if reference
            .extent()
            .unwrap_or(MAX_FRAME_LEN)
            .saturating_add(2)
            > remaining
        {
            return Err(CodecError::new(REPLY_LIMIT, "对象键超过剩余回复上界"));
        }
        struct KeyBuffer {
            bytes: WorkVec<u8>,
            limit: u64,
            exceeded: bool,
            failure: Option<CodecError>,
        }
        impl std::io::Write for KeyBuffer {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if (self.bytes.len() as u64).saturating_add(bytes.len() as u64) > self.limit {
                    self.exceeded = true;
                    return Err(std::io::Error::other("对象键JSON转义超过回复上界"));
                }
                if let Err(error) = self.bytes.extend_from_slice(bytes) {
                    self.failure = Some(error);
                    return Err(std::io::Error::other("owned key allocation refused"));
                }
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let budget = self.source.budget();
        let mut buffer = KeyBuffer {
            bytes: WorkVec::new(budget.as_ref()),
            limit: remaining,
            exceeded: false,
            failure: None,
        };
        let result = render_refview_ctx(CodecContext::Fragment(self), reference, &mut buffer);
        if buffer.exceeded {
            return Err(CodecError::new(
                REPLY_LIMIT,
                "对象键JSON转义超过剩余回复上界",
            ));
        }
        if let Some(error) = buffer.failure {
            return Err(error);
        }
        result?;
        self.access.pre_io_check()?;
        if let Some(budget) = budget.as_ref() {
            #[cfg(feature = "s_allocation_json_testonly")]
            {
                let parsed = OwnedJson::parse(&buffer.bytes, budget)?;
                let text = parsed
                    .view()
                    .as_str()
                    .ok_or_else(|| err("对象键非字符串"))?;
                return FragmentText::copy(text, Some(budget));
            }
            #[cfg(not(feature = "s_allocation_json_testonly"))]
            {
                let _ = budget;
                return Err(CodecError::static_detail(
                    capture_allocation::INCOMPLETE,
                    "owned key parser未启用",
                ));
            }
        }
        let text: String = serde_json::from_slice(&buffer.bytes)
            .map_err(|e| CodecError::new(code::UTF8, e.to_string()))?;
        Ok(FragmentText {
            bytes: WorkVec::from_plain(text.into_bytes()),
        })
    }
    fn reserve_reply(&mut self, bytes: u64) -> Result<(), CodecError> {
        self.reply_bytes = self
            .reply_bytes
            .checked_add(bytes)
            .filter(|n| *n <= self.max_reply_bytes)
            .ok_or_else(|| CodecError::new(REPLY_LIMIT, "片段字段/证明超过显式回复上界"))?;
        Ok(())
    }
    fn record(&mut self, digest: &NodeDigest, tag: u8, body: &[u8]) -> Result<(), CodecError> {
        if let Some(old) = self.nodes.get(digest) {
            if old.0 != tag || &old.1[..] != body {
                return Err(err("同根不同原件"));
            }
        } else {
            let budget = self.source.budget();
            let proof = OwnedProofNode::new(digest, tag, body, budget.as_ref())?;
            self.reserve_reply(encoded_len(&proof.view())? + u64::from(!self.nodes.is_empty()))?;
            let mut copy = WorkVec::new(budget.as_ref());
            copy.extend_from_slice(body)?;
            self.nodes.insert(*digest, (tag, copy))?;
        }
        Ok(())
    }
    /// 动态source仍按旧raw读取→gate/F→证明→post顺序；Closed从具体真实原件计费body读取。
    fn read_record(
        &mut self,
        digest: &NodeDigest,
    ) -> Result<(u8, VerifierBody<'capture>), CodecError> {
        self.access.pre_io_check()?;
        let (tag, body) = match &mut self.source {
            CodecContext::Source(source) => match source.read(digest)? {
                NodeRead::Missing => {
                    self.access.pre_io_check()?;
                    return Err(CodecError::new(
                        code::NODE_MISSING,
                        format!("节点缺失：{}", root_string(digest)),
                    ));
                }
                NodeRead::Found { tag, body } => {
                    self.access.accept_node(digest, tag, &body)?;
                    (tag, VerifierBody::Plain(body))
                }
            },
            CodecContext::Closed(owner) => owner.read_fragment(digest, self.access)?,
            _ => {
                return Err(CodecError::new(
                    code::SOURCE,
                    "fragment来源不是dynamic或实际Closed owner",
                ))
            }
        };
        self.record(digest, tag, &body)?;
        self.access.pre_io_check()?;
        Ok((tag, body))
    }
}
impl<'capture> FragmentRead<'capture> for Recorder<'_, 'capture> {
    fn budget(&self) -> Option<OwnedBudget> {
        self.source.budget()
    }
    fn read(&mut self, digest: &NodeDigest) -> Result<(u8, VerifierBody<'capture>), CodecError> {
        self.read_record(digest)
    }
    fn before_path_index(
        &mut self,
        prefix: &dyn path::PathView,
        end: usize,
        _index: u64,
    ) -> Result<(), CodecError> {
        if let Some(filter) = self.record_filter {
            let prefix = prefix
                .plain_prefix(end)
                .ok_or_else(|| err("record filter须保持原请求路径借用"))?;
            if filter(prefix) {
                self.access.charge_records(1)?;
            }
        }
        Ok(())
    }
}
// 仅普通旧调用/直接门序probe的适配；Closed没有NodeSource实现或owned→std逃逸。
impl<'capture> NodeSource for Recorder<'_, 'capture> {
    fn read(&mut self, digest: &NodeDigest) -> Result<NodeRead, CodecError> {
        let (tag, body) = self.read_record(digest)?;
        match body {
            VerifierBody::Plain(body) => Ok(NodeRead::Found { tag, body }),
            VerifierBody::Scoped(body) => Ok(NodeRead::Found {
                tag,
                body: body.into_plain(),
            }),
            VerifierBody::Borrowed(_) | VerifierBody::Metered(_) => Err(CodecError::static_detail(
                capture_allocation::INCOMPLETE,
                "owned Recorder不允许擦为NodeSource",
            )),
        }
    }
    fn checked_read(&mut self, digest: &NodeDigest) -> Result<Option<CheckedFrame>, CodecError> {
        self.access.pre_io_check()?;
        let CodecContext::Source(source) = &mut self.source else {
            return Err(CodecError::static_detail(
                capture_allocation::INCOMPLETE,
                "owned Recorder不允许擦为NodeSource",
            ));
        };
        let read = source.read(digest)?;
        let checked = match read {
            NodeRead::Missing => None,
            NodeRead::Found { tag, body } => {
                let frame = self.access.accept_frame(digest, tag, body)?;
                self.record(digest, frame.tag(), frame.body())?;
                Some(frame)
            }
        };
        self.access.pre_io_check()?;
        Ok(checked)
    }
}

#[cfg(test)]
mod checked_frame_tests {
    use super::*;
    use crate::s_content::{
        frame_calls_on_thread,
        job::{Existing, JobLimits, DEADLINE},
        MemoryNodes,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn recorder_keeps_old_accounting_when_deadline_lands_during_f() {
        let digest = frame(tag::NULL, &[]);
        let mut source = MemoryNodes::new();
        source.insert_unchecked(digest, tag::NULL, vec![]);
        let limits = JobLimits::from_canonical_json(&json!({
            "max_total_wire_bytes":"1024", "max_total_node_bytes":"1024",
            "max_total_nodes":"8", "max_total_records":"8", "max_total_requests":"8",
            "max_elapsed_ms":"1", "max_staged_bytes":"1024", "max_store_bytes":"1024"
        }))
        .unwrap();
        for checked in [false, true] {
            let before = frame_calls_on_thread();
            let now = Instant::now();
            let mut job = Job::new(
                limits,
                Existing {
                    store_bytes: 0,
                    staged_bytes: 0,
                },
                Box::new(move || {
                    now + Duration::from_millis(u64::from(frame_calls_on_thread() > before))
                }),
            )
            .unwrap();
            {
                let mut recorder = Recorder {
                    source: CodecContext::Source(&mut source),
                    access: &mut job,
                    nodes: WorkMap::new(None),
                    reply_bytes: 0,
                    max_reply_bytes: 1024,
                    record_filter: None,
                };
                let result = if checked {
                    recorder.checked_read(&digest).map(|_| ())
                } else {
                    NodeSource::read(&mut recorder, &digest).map(|_| ())
                };
                assert_eq!(
                    result.unwrap_err().code(),
                    DEADLINE,
                    "旧 post precheck 行为"
                );
                assert_eq!(recorder.nodes.len(), 1, "原证明收集已完成");
            }
            assert_eq!(frame_calls_on_thread() - before, 1);
            assert_eq!(job.snapshot().nodes, 1, "F 后不得多加计账前期限门");
        }
    }
}
fn public_header_view(r: RefView<'_>) -> Result<(), CodecError> {
    if r.kind() == Kind::TemplateBatch {
        Err(CodecError::new(
            code::TEMPLATE_PLACEHOLDER,
            "公开片段不能泄漏本批占位",
        ))
    } else {
        Ok(())
    }
}
fn public_header(r: &ContentRef) -> Result<(), CodecError> {
    if r.kind == Kind::TemplateBatch {
        Err(CodecError::new(
            code::TEMPLATE_PLACEHOLDER,
            "公开片段不能泄漏本批占位",
        ))
    } else {
        Ok(())
    }
}
fn err(e: impl std::fmt::Display) -> CodecError {
    CodecError::new(INVALID, e.to_string())
}
fn encoded_len(value: &impl serde::Serialize) -> Result<u64, CodecError> {
    struct Count(u64);
    impl std::io::Write for Count {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(b.len() as u64)
                .ok_or_else(|| std::io::Error::other("encoded length overflow"))?;
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut count = Count(0);
    serde_json::to_writer(&mut count, value).map_err(|e| err(e))?;
    Ok(count.0)
}
fn exact(v: &J, keys: &[&str]) -> Result<(), CodecError> {
    if v.as_object()
        .is_some_and(|o| o.len() == keys.len() && o.keys().all(|k| keys.contains(&k.as_str())))
    {
        Ok(())
    } else {
        Err(err("字段集合非exact"))
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(text: &str) -> Result<Vec<u8>, CodecError> {
    if text.len() % 2 != 0 {
        return Err(err("hex长度须为偶数"));
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            Ok(hex_val(pair[0]).ok_or_else(|| err("hex须为小写"))? * 16
                + hex_val(pair[1]).ok_or_else(|| err("hex须为小写"))?)
        })
        .collect()
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod owned_fragment_tests {
    use super::*;
    struct NoJob;
    impl Access for NoJob {
        fn owner_job(&mut self) -> Option<&mut Job> {
            None
        }
    }
    fn captured(nodes: MemoryNodes, extra: usize) -> (AuditCapture, OwnedBudget, usize) {
        let count = nodes.nodes.len();
        let bytes = nodes.nodes.values().map(|(_, body)| body.len()).sum();
        let base = capture_allocation::CaptureSlab::requested_bytes(count, bytes).unwrap();
        let budget = OwnedBudget::new(base + extra);
        let mut slab = capture_allocation::CaptureSlab::new(count, bytes, &budget).unwrap();
        for (digest, (tag, body)) in nodes.nodes {
            slab.push(digest, tag, &body).unwrap();
        }
        (
            AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap()),
            budget,
            base,
        )
    }
    fn job() -> Job {
        let limits = job::JobLimits::from_canonical_json(&json!({"max_total_wire_bytes":"1000000","max_total_node_bytes":"1000000","max_total_nodes":"10000","max_total_records":"10000","max_total_requests":"10000","max_elapsed_ms":"10000","max_staged_bytes":"1000000","max_store_bytes":"1000000"})).unwrap();
        let now = std::time::Instant::now();
        Job::new(
            limits,
            job::Existing {
                store_bytes: 0,
                staged_bytes: 0,
            },
            Box::new(move || now),
        )
        .unwrap()
    }
    #[test]
    fn owned_fragment_actual_source_common_kernel_proofs_values_and_drop0() {
        let cases = [
            (
                Value::Array(vec![
                    Value::Null,
                    Value::string("nested"),
                    Value::Bool(true),
                ]),
                Unit::Items,
                1,
                2,
            ),
            (
                Value::Object(vec![
                    ("a\\汉\"".into(), Value::Bool(false)),
                    ("z".into(), Value::Null),
                ]),
                Unit::Items,
                0,
                2,
            ),
            (Value::string("汉abc".repeat(1600)), Unit::Bytes, 4088, 20),
            (
                Value::JsonText(Box::new(JsonValue::Array(vec![JsonValue::Null]))),
                Unit::Value,
                0,
                1,
            ),
        ];
        for (value, unit, offset, limit) in cases {
            let mut nodes = MemoryNodes::new();
            let built = write_value(&mut nodes, &value).unwrap();
            let root = built.content_ref();
            let mut ordinary_job = job();
            let before = frame_calls_on_thread();
            let ordinary = read(
                &mut nodes.clone(),
                &root,
                &[],
                unit,
                offset,
                limit,
                1_000_000,
                &mut ordinary_job,
            )
            .unwrap();
            let ordinary_f = frame_calls_on_thread() - before;
            let (capture, budget, base) = captured(nodes, 1_000_000);
            let material = capture.original(&built.digest).unwrap().1.to_vec();
            let mut owner = capture
                .closed_codec_for_allocation_prototype(0, 0, None)
                .unwrap();
            let mut owned_job = job();
            let before = frame_calls_on_thread();
            let fragment = read_owned(
                &mut owner,
                (&root).into(),
                &[],
                unit,
                offset,
                limit,
                1_000_000,
                &mut owned_job,
            )
            .unwrap();
            let owned_f = frame_calls_on_thread() - before;
            assert_eq!(owned_f, ordinary_f, "同算法真实F读取计量");
            assert_eq!(
                owned_job.snapshot(),
                ordinary_job.snapshot(),
                "真实Job累计门与节点用量保持"
            );
            assert_eq!(
                serde_json::to_value(fragment.fields_view().unwrap()).unwrap(),
                J::Object(ordinary.fields())
            );
            assert!(fragment.allocation_budget().is_some());
            assert!(budget.snapshot().live_requested_layout_bytes > base);
            assert_eq!(capture.original(&built.digest).unwrap().1, material);
            assert_eq!(owner.overlay_peak_used(), (0, 0));
            assert!(
                fragment.into_plain().is_err(),
                "Owned结果不能擦成旧std Fragment"
            );
            assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
            drop(owner);
            drop(capture);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn owned_fragment_readonly_scope_rejects_and_budget_refusal_preserves_capture() {
        let mut nodes = MemoryNodes::new();
        let built = write_value(&mut nodes, &Value::Array(vec![Value::Null])).unwrap();
        let root = built.content_ref();
        let (capture, budget, base) = captured(nodes.clone(), 100_000);
        let mut owner = capture
            .closed_codec_for_allocation_prototype(0, 0, None)
            .unwrap();
        for (offset, limit, unit) in [
            (2, 1, Unit::Items),
            (0, 0, Unit::Items),
            (0, 1, Unit::Bytes),
        ] {
            let expected = read(
                &mut nodes.clone(),
                &root,
                &[],
                unit,
                offset,
                limit,
                100_000,
                &mut NoJob,
            )
            .unwrap_err();
            let actual = read_owned(
                &mut owner,
                (&root).into(),
                &[],
                unit,
                offset,
                limit,
                100_000,
                &mut NoJob,
            )
            .err()
            .unwrap();
            assert_eq!(actual.code(), expected.code());
            assert_eq!(actual.detail(), expected.detail());
            assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
            assert_eq!(owner.overlay_peak_used(), (0, 0));
        }
        drop(owner);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let (capture, budget, base) = captured(nodes, 0);
        let material = capture.original(&built.digest).unwrap().1.to_vec();
        let mut owner = capture
            .closed_codec_for_allocation_prototype(0, 0, None)
            .unwrap();
        let error = read_owned(
            &mut owner,
            (&root).into(),
            &[],
            Unit::Items,
            0,
            1,
            100_000,
            &mut NoJob,
        )
        .err()
        .unwrap();
        assert_eq!(error.code(), capture_allocation::LIMIT);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        assert_eq!(capture.original(&built.digest).unwrap().1, material);
        assert_eq!(owner.overlay_peak_used(), (0, 0));
        drop(owner);
        drop(capture);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}
