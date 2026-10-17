//! #1394 C 内部引用owner：fixed字段无String，不携带闭包/来源证明。
//! 普通ContentRef保持公开exact四字段；RefView只统一借用与根声明检查。
use super::capture_allocation::audit_json::JsonView;
use super::capture_allocation::OwnedBudget;
use super::{CodecError, CodecVec, ContentRef, Kind, NodeDigest, CONTENT_FORMAT, ROOT_PREFIX};

/// #1394 C 借用 transport 声明：字段形态已核，format/root 仍在 digest() 才核。
/// 不携带闭包、来源或实际节点资格；转换为 fixed owner 也不增添任何资格。
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransportRef<'a> {
    format: &'a str,
    root: &'a str,
    kind: Kind,
    extent: Option<u64>,
}
impl<'a> TransportRef<'a> {
    pub fn from_json_view(node: JsonView<'a>) -> Result<Self, CodecError> {
        Self::from_json_view_typed(node).map_err(|e| e.into_codec())
    }
    pub fn from_json_view_typed(node: JsonView<'a>) -> Result<Self, RefDiagnostic<'a>> {
        let map = node.as_object().ok_or(RefDiagnostic::Object)?;
        for key in map.keys() {
            if !matches!(key, "format" | "root" | "kind" | "extent") {
                return Err(RefDiagnostic::Extra(key));
            }
        }
        let field = |name: &'static str| map.get(name).ok_or(RefDiagnostic::Missing(name));
        let format = field("format")?
            .as_str()
            .ok_or(RefDiagnostic::Text("format"))?;
        let root = field("root")?.as_str().ok_or(RefDiagnostic::Text("root"))?;
        let kind_name = field("kind")?.as_str().ok_or(RefDiagnostic::Text("kind"))?;
        let kind = Kind::from_name(kind_name).ok_or(RefDiagnostic::Kind(kind_name))?;
        let extent_node = field("extent")?;
        let extent = if extent_node.is_null() {
            None
        } else if let Some(text) = extent_node.as_str() {
            Some(super::parse_canonical_u64_typed(text, "extent").map_err(RefDiagnostic::Decimal)?)
        } else {
            return Err(RefDiagnostic::Extent(extent_node));
        };
        if kind.extent_is_byte_or_count() != extent.is_some() {
            return Err(RefDiagnostic::KindExtent(kind));
        }
        Ok(Self {
            format,
            root,
            kind,
            extent,
        })
    }
    pub fn format(self) -> &'a str {
        self.format
    }
    pub fn root(self) -> &'a str {
        self.root
    }
    pub fn kind(self) -> Kind {
        self.kind
    }
    pub fn extent(self) -> Option<u64> {
        self.extent
    }
    pub fn digest(self) -> Result<NodeDigest, CodecError> {
        super::transport_digest(self.format, self.root)
    }
    pub fn try_owned_ref(self) -> Result<RefOwner, CodecError> {
        Ok(RefOwner::new(self.kind, self.digest()?, self.extent))
    }
}

#[derive(Debug)]
pub enum RefDiagnostic<'a> {
    Object,
    Extra(&'a str),
    Missing(&'static str),
    Text(&'static str),
    Kind(&'a str),
    Extent(JsonView<'a>),
    KindExtent(Kind),
    Decimal(super::DecimalError<'a>),
}
impl RefDiagnostic<'_> {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Kind(_) => super::code::KIND,
            Self::Extent(_) | Self::KindExtent(_) => super::code::EXTENT,
            Self::Decimal(e) => e.code(),
            _ => super::code::REF_FIELD,
        }
    }
    fn fmt_detail(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Object => f.write_str("ContentRef 必须是 JSON 对象"),
            Self::Extra(k) => write!(f, "ContentRef 出现 exact 四字段之外的键 `{k}`"),
            Self::Missing(k) => write!(f, "ContentRef 缺字段 `{k}`"),
            Self::Text(k) => write!(f, "{k} 必须是字符串"),
            Self::Kind(k) => write!(f, "未知 kind `{k}`"),
            Self::Extent(v) => {
                f.write_str("extent 必须是规范十进制字符串或 null（U64 长度按 §5.2 用十进制字符串传输），实为 ")?;
                struct Out<'a, 'b>(&'a mut std::fmt::Formatter<'b>);
                impl std::io::Write for Out<'_, '_> {
                    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                        let text = std::str::from_utf8(bytes)
                            .map_err(|_| std::io::ErrorKind::InvalidData)?;
                        self.0
                            .write_str(text)
                            .map_err(|_| std::io::ErrorKind::Other)?;
                        Ok(bytes.len())
                    }
                    fn flush(&mut self) -> std::io::Result<()> {
                        Ok(())
                    }
                }
                super::json_writer::write_json(&mut Out(f), v).map_err(|_| std::fmt::Error)
            }
            Self::KindExtent(k) => write!(
                f,
                "kind={} 的 extent 必须是{}",
                k.name(),
                if k.extent_is_byte_or_count() {
                    "规范十进制字符串"
                } else {
                    "null"
                }
            ),
            Self::Decimal(_) => unreachable!("Decimal has its own complete Display"),
        }
    }
    fn into_codec(self) -> CodecError {
        if let Self::Decimal(e) = self {
            let rendered = e.to_string();
            let code = e.code();
            return CodecError::new(
                code,
                rendered
                    .strip_prefix(code)
                    .unwrap()
                    .strip_prefix(": ")
                    .unwrap(),
            );
        }
        struct Detail<'a, 'b>(&'a RefDiagnostic<'b>);
        impl std::fmt::Display for Detail<'_, '_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt_detail(f)
            }
        }
        CodecError::new(self.code(), Detail(&self).to_string())
    }
}
impl std::fmt::Display for RefDiagnostic<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Self::Decimal(e) = self {
            return std::fmt::Display::fmt(e, f);
        }
        write!(f, "{}: ", self.code())?;
        self.fmt_detail(f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefOwner {
    digest: NodeDigest,
    root: [u8; 68],
    kind: Kind,
    extent: Option<u64>,
}
impl RefOwner {
    // Representation only: some callers have checked actual nodes; TransportRef callers
    // have checked only the declaration. No trust is added and no source is read here.
    pub(super) fn new(kind: Kind, digest: NodeDigest, extent: Option<u64>) -> Self {
        let mut root = [0u8; 68];
        root[..4].copy_from_slice(ROOT_PREFIX.as_bytes());
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for (i, byte) in digest.iter().enumerate() {
            root[4 + 2 * i] = HEX[(byte >> 4) as usize];
            root[5 + 2 * i] = HEX[(byte & 15) as usize];
        }
        Self {
            digest,
            root,
            kind,
            extent,
        }
    }
    pub fn view(&self) -> RefView<'_> {
        RefView::Owned(self)
    }
    pub fn format(&self) -> &'static str {
        CONTENT_FORMAT
    }
    pub fn root(&self) -> &str {
        // SAFETY: private constructor writes only the ASCII prefix and lowercase hex.
        unsafe { std::str::from_utf8_unchecked(&self.root) }
    }
    pub fn kind(&self) -> Kind {
        self.kind
    }
    pub fn extent(&self) -> Option<u64> {
        self.extent
    }
    pub fn digest(&self) -> NodeDigest {
        self.digest
    }
}
#[derive(Clone, Copy, Debug)]
pub enum RefView<'a> {
    Normal(&'a ContentRef),
    Owned(&'a RefOwner),
}
impl RefView<'_> {
    pub fn format(&self) -> &str {
        match self {
            Self::Normal(r) => &r.format,
            Self::Owned(r) => r.format(),
        }
    }
    pub fn root(&self) -> &str {
        match self {
            Self::Normal(r) => &r.root,
            Self::Owned(r) => r.root(),
        }
    }
    pub fn kind(&self) -> Kind {
        match self {
            Self::Normal(r) => r.kind,
            Self::Owned(r) => r.kind(),
        }
    }
    pub fn extent(&self) -> Option<u64> {
        match self {
            Self::Normal(r) => r.extent,
            Self::Owned(r) => r.extent(),
        }
    }
    pub fn digest(&self) -> Result<NodeDigest, CodecError> {
        match self {
            Self::Normal(r) => r.digest(),
            Self::Owned(r) => Ok(r.digest()),
        }
    }
}
impl<'a> From<&'a ContentRef> for RefView<'a> {
    fn from(r: &'a ContentRef) -> Self {
        Self::Normal(r)
    }
}
impl<'a> From<&'a super::capture_allocation::native_owned::NativeValue<ContentRef>>
    for RefView<'a>
{
    fn from(r: &'a super::capture_allocation::native_owned::NativeValue<ContentRef>) -> Self {
        Self::Normal(r.as_ref())
    }
}
impl<'a> From<&'a RefOwner> for RefView<'a> {
    fn from(r: &'a RefOwner) -> Self {
        Self::Owned(r)
    }
}

// Only output representation varies; the path's source reads and checks are shared.
pub(super) trait RefOutput: Sized {
    const NATIVE_HEAP: usize;
    const RETAINS_NATIVE: bool = false;
    fn with_native_permit(
        self,
        permit: Option<super::capture_allocation::scratch::NativeReservation>,
    ) -> Self {
        if let Some(permit) = permit {
            permit.retain();
        }
        self
    }
    fn from_checked(kind: Kind, digest: NodeDigest, extent: Option<u64>) -> Self;
}
impl RefOutput for ContentRef {
    const NATIVE_HEAP: usize = CONTENT_FORMAT.len() + ROOT_PREFIX.len() + 64;
    fn from_checked(kind: Kind, digest: NodeDigest, extent: Option<u64>) -> Self {
        ContentRef::new(kind, &digest, extent)
    }
}
impl RefOutput for RefOwner {
    const NATIVE_HEAP: usize = 0;
    fn from_checked(kind: Kind, digest: NodeDigest, extent: Option<u64>) -> Self {
        Self::new(kind, digest, extent)
    }
}

impl RefOutput for super::capture_allocation::native_owned::NativeValue<ContentRef> {
    const NATIVE_HEAP: usize = CONTENT_FORMAT.len() + ROOT_PREFIX.len() + 64;
    const RETAINS_NATIVE: bool = true;
    fn from_checked(kind: Kind, digest: NodeDigest, extent: Option<u64>) -> Self {
        Self::from_reserved(ContentRef::new(kind, &digest, extent), None)
    }
    fn with_native_permit(
        self,
        permit: Option<super::capture_allocation::scratch::NativeReservation>,
    ) -> Self {
        self.attach_reserved(permit)
    }
}

/// Results retain the exact requested-name borrows and actual same-capture CodecVec owner.
/// No key/root/format String is made; caller names must outlive this result.
pub struct OwnedRefFields<'names> {
    entries: CodecVec<(&'names str, RefOwner)>,
}
impl<'names> OwnedRefFields<'names> {
    pub(super) fn new(budget: &OwnedBudget) -> Self {
        Self {
            entries: CodecVec::new(Some(budget)),
        }
    }
    pub(super) fn append(
        &mut self,
        key: &'names str,
        reference: RefOwner,
    ) -> Result<(), CodecError> {
        // Match CodecFields::append's canonical result-order gate without string ownership.
        if self
            .entries
            .last()
            .is_some_and(|(previous, _)| *previous >= key)
        {
            return Err(CodecError::static_detail(
                super::code::OBJECT_KEY_ORDER,
                "codec字段结果追加必须严格既有canonical序",
            ));
        }
        self.entries.try_push((key, reference))
    }
    pub fn get(&self, key: &str) -> Option<&RefOwner> {
        self.entries
            .binary_search_by(|(name, _)| name.cmp(&key))
            .ok()
            .map(|at| &self.entries[at].1)
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &RefOwner)> {
        self.entries.iter().map(|(name, r)| (*name, r))
    }
    pub fn try_clone(&self) -> Result<Self, CodecError> {
        Ok(Self {
            entries: self.entries.try_clone()?,
        })
    }
}

#[cfg(all(test, feature = "s_allocation_json_testonly"))]
mod tests {
    use super::*;
    use crate::s_content::capture_allocation::audit_json::tests::observed;
    use crate::s_content::capture_allocation::audit_json::OwnedJson;
    fn transport_view_parity(value: &serde_json::Value) {
        let budget = OwnedBudget::new(100000);
        let bytes = serde_json::to_vec(value).unwrap();
        let tree = OwnedJson::parse_legacy_shape(&bytes, &budget).unwrap();
        let normal = ContentRef::from_transport_json(value);
        let borrowed = TransportRef::from_json_view(tree.view());
        match (normal, borrowed) {
            (Ok(normal), Ok(claim)) => {
                assert_eq!(claim.format(), normal.format);
                assert_eq!(claim.root(), normal.root);
                assert_eq!(claim.kind(), normal.kind);
                assert_eq!(claim.extent(), normal.extent);
                assert_eq!(claim.digest(), normal.digest());
                if let Ok(owner) = claim.try_owned_ref() {
                    assert_eq!(owner.root(), normal.root);
                    assert_eq!(owner.kind(), normal.kind);
                    assert_eq!(owner.extent(), normal.extent);
                }
            }
            (Err(normal), Err(borrowed)) => assert_eq!(normal, borrowed),
            other => panic!("transport representation mismatch: {other:?}"),
        }
        drop(tree);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn transport_ref_borrowed_exact_fields_kind_extent_and_diagnostics_match_normal() {
        for kind in Kind::ALL {
            let reference = ContentRef::new(
                kind,
                &[0xad; 32],
                kind.extent_is_byte_or_count()
                    .then_some(super::super::MAX_FRAME_LEN),
            );
            transport_view_parity(&reference.to_transport_json());
        }
        let valid = ContentRef::new(Kind::Array, &[0xad; 32], Some(0)).to_transport_json();
        transport_view_parity(&serde_json::json!([]));
        for (field, replacement) in [
            ("format", serde_json::json!(false)),
            ("root", serde_json::json!([])),
            ("kind", serde_json::json!(0)),
            ("kind", serde_json::json!("unknown")),
            ("extent", serde_json::json!(null)),
            ("extent", serde_json::json!("01")),
            ("extent", serde_json::json!("+1")),
            ("extent", serde_json::json!("9223372036854775808")),
            ("extent", serde_json::json!({"é":[true,"\n"]})),
        ] {
            let mut value = valid.clone();
            value[field] = replacement;
            transport_view_parity(&value);
        }
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove("root");
        transport_view_parity(&missing);
        let mut extra = missing;
        extra["extra"] = serde_json::json!(0);
        transport_view_parity(&extra);
        assert_eq!(
            ContentRef::from_transport_json(&extra)
                .unwrap_err()
                .detail(),
            "ContentRef 出现 exact 四字段之外的键 `extra`"
        );
        let mut invalid_value = valid;
        invalid_value["extent"] = serde_json::json!({"é":[true,"\n"]});
        assert_eq!(
            ContentRef::from_transport_json(&invalid_value).unwrap_err().detail(),
            "extent 必须是规范十进制字符串或 null（U64 长度按 §5.2 用十进制字符串传输），实为 {\"é\":[true,\"\\n\"]}"
        );
    }
    #[test]
    fn transport_ref_format_root_validation_stays_deferred_and_preserves_error_order() {
        for (format, root, expected) in [
            ("other-format", "bad-root", super::super::code::FORMAT),
            (CONTENT_FORMAT, "sc1-Ａ", super::super::code::ROOT),
            (CONTENT_FORMAT, "sc1-ABCDEF", super::super::code::ROOT),
        ] {
            let value = serde_json::json!({
                "format":format,"root":root,"kind":"array","extent":"0"
            });
            transport_view_parity(&value);
            let normal = ContentRef::from_transport_json(&value).unwrap();
            let borrowed = TransportRef::from_json_view((&value).into()).unwrap();
            assert_eq!(normal.digest().unwrap_err().code(), expected);
            assert_eq!(borrowed.digest(), normal.digest());
            assert_eq!(
                borrowed.try_owned_ref().unwrap_err(),
                normal.digest().unwrap_err()
            );
            let mut wrong_extent = value;
            wrong_extent["extent"] = serde_json::json!(null);
            assert_eq!(
                TransportRef::from_json_view((&wrong_extent).into())
                    .unwrap_err()
                    .code(),
                super::super::code::EXTENT
            );
        }
        for root in [
            format!("sc1-{}", "A".repeat(64)),
            format!("sc1-{}", "é".repeat(32)),
        ] {
            let value = serde_json::json!({
                "format":CONTENT_FORMAT,"root":root,"kind":"bool","extent":null
            });
            transport_view_parity(&value);
            let claim = TransportRef::from_json_view((&value).into()).unwrap();
            assert_eq!(claim.digest().unwrap_err().code(), super::super::code::ROOT);
            assert_eq!(
                claim.digest().unwrap_err().detail(),
                format!("根文本 `{root}` 含非小写十六进制字符")
            );
        }
        let mut value = ContentRef::new(Kind::Bool, &[0xad; 32], None).to_transport_json();
        value["extent"] = serde_json::json!("0");
        transport_view_parity(&value);
    }
    #[test]
    fn transport_ref_borrow_and_inline_owner_allocate_no_heap_under_one_byte_budget() {
        let value = ContentRef::new(Kind::Array, &[0xad; 32], Some(17)).to_transport_json();
        let budget = OwnedBudget::new(100000);
        let bytes = serde_json::to_vec(&value).unwrap();
        let tree = OwnedJson::parse_legacy_shape(&bytes, &budget).unwrap();
        let before = budget.snapshot();
        let low = OwnedBudget::new(1);
        let mut occupied = CodecVec::<u8>::with_capacity(1, Some(&low)).unwrap();
        occupied.try_push(0).unwrap();
        let low_before = low.snapshot();
        let ((claim, owner), calls) = observed(|| {
            let claim = TransportRef::from_json_view(tree.view()).unwrap();
            (claim, claim.try_owned_ref().unwrap())
        });
        assert_eq!(calls, 0);
        assert_eq!(claim.root(), owner.root());
        assert!(owner.root().is_ascii());
        assert_eq!(owner.root().len(), ROOT_PREFIX.len() + 64);
        assert_eq!(
            budget.snapshot().allocation_attempts,
            before.allocation_attempts
        );
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            before.live_requested_layout_bytes
        );
        assert_eq!(
            low.snapshot().allocation_attempts,
            low_before.allocation_attempts
        );
        assert_eq!(
            low.snapshot().live_requested_layout_bytes,
            low_before.live_requested_layout_bytes
        );
        drop(occupied);
        assert_eq!(low.snapshot().live_requested_layout_bytes, 0);
        // A borrowed claim/inline owner has no budget argument: the already retained
        // JSON tree remains its real owner, and its capacity is released only on drop.
        drop(tree);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let error = OwnedJson::parse_legacy_shape(&bytes, &low).unwrap_err();
        assert_eq!(error.code(), crate::s_content::capture_allocation::LIMIT);
        assert_eq!(low.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn fixed_ref_constructor_and_views_do_not_allocate_strings() {
        let expected = ContentRef::new(Kind::Array, &[0xad; 32], Some(17));
        let ((reference, copy), calls) = observed(|| {
            let r = RefOwner::new(Kind::Array, [0xad; 32], Some(17));
            let copy = r;
            (r, copy)
        });
        assert_eq!(calls, 0);
        for r in [reference, copy] {
            assert_eq!(r.format(), expected.format);
            assert_eq!(r.root(), expected.root);
            assert_eq!(r.kind(), expected.kind);
            assert_eq!(r.extent(), expected.extent);
            assert_eq!(r.digest(), expected.digest().unwrap());
        }
    }
}
