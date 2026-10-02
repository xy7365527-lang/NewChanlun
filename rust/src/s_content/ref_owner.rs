//! #1394 C 内部引用owner：fixed字段无String，不携带闭包/来源证明。
//! 普通ContentRef保持公开exact四字段；RefView只统一借用与根声明检查。
use super::capture_allocation::OwnedBudget;
use super::{CodecError, CodecVec, ContentRef, Kind, NodeDigest, CONTENT_FORMAT, ROOT_PREFIX};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RefOwner {
    digest: NodeDigest,
    root: [u8; 68],
    kind: Kind,
    extent: Option<u64>,
}
impl RefOwner {
    // Caller has performed the common actual-node role/header checks. This constructor
    // adds no trust, does not read the source, and is independent of any allocation budget.
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
impl<'a> From<&'a RefOwner> for RefView<'a> {
    fn from(r: &'a RefOwner) -> Self {
        Self::Owned(r)
    }
}

// Only output representation varies; the path's source reads and checks are shared.
pub(super) trait RefOutput: Sized {
    fn from_checked(kind: Kind, digest: NodeDigest, extent: Option<u64>) -> Self;
}
impl RefOutput for ContentRef {
    fn from_checked(kind: Kind, digest: NodeDigest, extent: Option<u64>) -> Self {
        ContentRef::new(kind, &digest, extent)
    }
}
impl RefOutput for RefOwner {
    fn from_checked(kind: Kind, digest: NodeDigest, extent: Option<u64>) -> Self {
        Self::new(kind, digest, extent)
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
