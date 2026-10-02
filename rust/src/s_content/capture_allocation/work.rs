//! #1394 同capture账本的真实Layout标准allocator及fallible codec工作集。
//! 普通路径使用标准分配器；owned只有try*增长，无额度错误后普通fallback。
use super::*;
use allocator_api2::{
    alloc::{AllocError, Allocator},
    vec::Vec as AllocVec,
};
use std::collections::hash_map::RandomState;
use std::collections::{BTreeMap, HashSet as StdSet};
use std::hash::Hash;
use std::ops::{Deref, DerefMut, Index};

pub(crate) fn allocation_error() -> CodecError {
    CodecError::static_detail(LIMIT, "codec requested Layout 超过显式实验额度或分配失败")
}
#[derive(Clone)]
pub(crate) struct LedgerAllocator {
    budget: OwnedBudget,
    kind: &'static str,
}
impl LedgerAllocator {
    pub(crate) fn new(budget: &OwnedBudget) -> Self {
        Self::new_tag(budget, "codec Layout")
    }
    pub(crate) fn new_tag(budget: &OwnedBudget, kind: &'static str) -> Self {
        Self {
            budget: budget.clone(),
            kind,
        }
    }
}
// SAFETY: every allocation uses the requested valid Layout, and every matching release uses
// the same allocator ledger and Layout. Owned vectors/maps keep this Arc alive through Drop.
unsafe impl Allocator for LedgerAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.budget
            .reserve_layout(layout, self.kind)
            .map_err(|_| AllocError)?;
        if layout.size() == 0 {
            return Ok(NonNull::slice_from_raw_parts(
                NonNull::new(layout.align() as *mut u8).unwrap(),
                0,
            ));
        }
        // SAFETY: nonzero valid caller Layout.
        let p = unsafe { std::alloc::alloc(layout) };
        let Some(p) = NonNull::new(p) else {
            self.budget.rollback_layout(layout);
            return Err(AllocError);
        };
        self.budget.commit_layout(layout);
        Ok(NonNull::slice_from_raw_parts(p, layout.size()))
    }
    unsafe fn deallocate(&self, p: NonNull<u8>, layout: Layout) {
        if layout.size() == 0 {
            return;
        }
        // SAFETY: Allocator contract requires the live allocation and its original Layout.
        unsafe { std::alloc::dealloc(p.as_ptr(), layout) };
        self.budget.release_layout(layout);
    }
    unsafe fn grow(
        &self,
        p: NonNull<u8>,
        old: Layout,
        new: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        // New complete block is gated while OLD is still live. Failure does not touch old bytes.
        let grown = self.allocate(new)?;
        // SAFETY: caller guarantees old live allocation; new is disjoint and at least old.size().
        unsafe {
            std::ptr::copy_nonoverlapping(p.as_ptr(), grown.as_ptr().cast::<u8>(), old.size());
            self.deallocate(p, old)
        };
        Ok(grown)
    }
    unsafe fn shrink(
        &self,
        p: NonNull<u8>,
        old: Layout,
        new: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let shrunk = self.allocate(new)?;
        // SAFETY: caller old live; new disjoint, exactly its requested initialized prefix copied.
        unsafe {
            std::ptr::copy_nonoverlapping(p.as_ptr(), shrunk.as_ptr().cast::<u8>(), new.size());
            self.deallocate(p, old)
        };
        Ok(shrunk)
    }
}

enum VecStorage<T> {
    Plain(Vec<T>),
    Owned(AllocVec<T, LedgerAllocator>),
}
/// Opaque持有者；返回离开codec仍保留真实allocator，不能转成std Vec逃账。
pub struct CodecVec<T> {
    storage: VecStorage<T>,
}
pub(crate) type WorkVec<T> = CodecVec<T>;
impl<T> Default for CodecVec<T> {
    fn default() -> Self {
        Self::from_plain(Vec::new())
    }
}
impl<T> CodecVec<T> {
    /// 只借实际allocator的原账本，不接受外来budget标记。
    pub fn allocation_budget(&self) -> Option<&OwnedBudget> {
        match &self.storage {
            VecStorage::Plain(_) => None,
            VecStorage::Owned(v) => Some(&v.allocator().budget),
        }
    }
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), CodecError> {
        self.reserve_exact(additional)
    }
    /// 普通持有者的消费转移；Owned直接拒绝，不能藉此擦为std。
    pub fn try_into_plain(self) -> Result<Vec<T>, CodecError> {
        self.into_plain()
    }
    pub fn from_plain(v: Vec<T>) -> Self {
        Self {
            storage: VecStorage::Plain(v),
        }
    }
    pub(crate) fn new(b: Option<&OwnedBudget>) -> Self {
        Self {
            storage: match b {
                None => VecStorage::Plain(Vec::new()),
                Some(b) => VecStorage::Owned(AllocVec::new_in(LedgerAllocator::new_tag(
                    b,
                    std::any::type_name::<T>(),
                ))),
            },
        }
    }
    pub(crate) fn with_capacity(n: usize, b: Option<&OwnedBudget>) -> Result<Self, CodecError> {
        match b {
            None => Ok(Self::from_plain(Vec::with_capacity(n))),
            Some(b) => {
                let mut v = Self::new(Some(b));
                v.reserve_exact(n)?;
                Ok(v)
            }
        }
    }
    pub(crate) fn reserve(&mut self, n: usize) -> Result<(), CodecError> {
        match &mut self.storage {
            VecStorage::Plain(v) => {
                v.reserve(n);
                Ok(())
            }
            VecStorage::Owned(v) => v.try_reserve(n).map_err(|_| allocation_error()),
        }
    }
    pub(crate) fn reserve_exact(&mut self, n: usize) -> Result<(), CodecError> {
        match &mut self.storage {
            VecStorage::Plain(v) => {
                v.reserve_exact(n);
                Ok(())
            }
            VecStorage::Owned(v) => v.try_reserve_exact(n).map_err(|_| allocation_error()),
        }
    }
    pub fn try_push(&mut self, x: T) -> Result<(), CodecError> {
        self.push(x)
    }
    pub(crate) fn push(&mut self, x: T) -> Result<(), CodecError> {
        self.reserve(1)?;
        match &mut self.storage {
            VecStorage::Plain(v) => v.push(x),
            VecStorage::Owned(v) => v.push(x),
        };
        Ok(())
    }
    pub fn pop(&mut self) -> Option<T> {
        match &mut self.storage {
            VecStorage::Plain(v) => v.pop(),
            VecStorage::Owned(v) => v.pop(),
        }
    }
    pub fn clear(&mut self) {
        match &mut self.storage {
            VecStorage::Plain(v) => v.clear(),
            VecStorage::Owned(v) => v.clear(),
        }
    }
    pub fn truncate(&mut self, n: usize) {
        match &mut self.storage {
            VecStorage::Plain(v) => v.truncate(n),
            VecStorage::Owned(v) => v.truncate(n),
        }
    }
    pub fn capacity(&self) -> usize {
        match &self.storage {
            VecStorage::Plain(v) => v.capacity(),
            VecStorage::Owned(v) => v.capacity(),
        }
    }
    pub(crate) fn extend_from_slice(&mut self, s: &[T]) -> Result<(), CodecError>
    where
        T: Clone,
    {
        self.reserve(s.len())?;
        match &mut self.storage {
            VecStorage::Plain(v) => v.extend_from_slice(s),
            VecStorage::Owned(v) => v.extend_from_slice(s),
        };
        Ok(())
    }
    pub(crate) fn try_clone(&self) -> Result<Self, CodecError>
    where
        T: Clone,
    {
        match &self.storage {
            VecStorage::Plain(v) => Ok(Self::from_plain(v.clone())),
            VecStorage::Owned(v) => {
                let mut c = Self::new(Some(&v.allocator().budget));
                c.extend_from_slice(v)?;
                Ok(c)
            }
        }
    }
    pub(crate) fn sort_by(&mut self, mut cmp: impl FnMut(&T, &T) -> std::cmp::Ordering) {
        match &mut self.storage {
            VecStorage::Plain(v) => v.sort_by(cmp),
            VecStorage::Owned(v) => v.sort_unstable_by(|a, b| cmp(a, b)),
        }
    }
    pub(crate) fn into_plain(self) -> Result<Vec<T>, CodecError> {
        match self.storage {
            VecStorage::Plain(v) => Ok(v),
            VecStorage::Owned(_) => Err(CodecError::static_detail(
                INCOMPLETE,
                "owned codec容器不可擦除为std Vec",
            )),
        }
    }
}
impl CodecVec<u8> {
    /// 字节追加只复制u8，不开放可能深复制payload的泛型Clone入口。
    pub fn try_extend_bytes(&mut self, bytes: &[u8]) -> Result<(), CodecError> {
        self.extend_from_slice(bytes)
    }
}

impl<T> Deref for CodecVec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        match &self.storage {
            VecStorage::Plain(v) => v,
            VecStorage::Owned(v) => v,
        }
    }
}
impl<T> DerefMut for CodecVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        match &mut self.storage {
            VecStorage::Plain(v) => v,
            VecStorage::Owned(v) => v,
        }
    }
}
impl<T: std::fmt::Debug> std::fmt::Debug for CodecVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}
impl<T: PartialEq> PartialEq for CodecVec<T> {
    fn eq(&self, o: &Self) -> bool {
        self.deref() == o.deref()
    }
}
impl<T: Eq> Eq for CodecVec<T> {}
impl<'a, T> IntoIterator for &'a CodecVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, T> IntoIterator for &'a mut CodecVec<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
enum IterStorage<T> {
    Plain(std::vec::IntoIter<T>),
    Owned(allocator_api2::vec::IntoIter<T, LedgerAllocator>),
}
pub struct CodecVecIntoIter<T> {
    storage: IterStorage<T>,
}
impl<T> Iterator for CodecVecIntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        match &mut self.storage {
            IterStorage::Plain(v) => v.next(),
            IterStorage::Owned(v) => v.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.storage {
            IterStorage::Plain(v) => v.size_hint(),
            IterStorage::Owned(v) => v.size_hint(),
        }
    }
}
impl<T> DoubleEndedIterator for CodecVecIntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        match &mut self.storage {
            IterStorage::Plain(v) => v.next_back(),
            IterStorage::Owned(v) => v.next_back(),
        }
    }
}
impl<T> ExactSizeIterator for CodecVecIntoIter<T> {}
impl<T> IntoIterator for CodecVec<T> {
    type Item = T;
    type IntoIter = CodecVecIntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        CodecVecIntoIter {
            storage: match self.storage {
                VecStorage::Plain(v) => IterStorage::Plain(v.into_iter()),
                VecStorage::Owned(v) => IterStorage::Owned(v.into_iter()),
            },
        }
    }
}

pub(crate) enum WorkMap<K, V> {
    Plain(hashbrown::HashMap<K, V, RandomState, allocator_api2::alloc::Global>),
    Owned(hashbrown::HashMap<K, V, RandomState, LedgerAllocator>),
}
impl<K, V> Default for WorkMap<K, V> {
    fn default() -> Self {
        Self::Plain(hashbrown::HashMap::with_hasher(RandomState::new()))
    }
}
impl<K: Eq + Hash, V> WorkMap<K, V> {
    pub(crate) fn new(b: Option<&OwnedBudget>) -> Self {
        match b {
            None => Self::default(),
            Some(b) => Self::Owned(hashbrown::HashMap::with_hasher_in(
                RandomState::new(),
                LedgerAllocator::new_tag(b, std::any::type_name::<(K, V)>()),
            )),
        }
    }
    pub(crate) fn insert(&mut self, k: K, v: V) -> Result<Option<V>, CodecError> {
        match self {
            Self::Plain(m) => Ok(m.insert(k, v)),
            Self::Owned(m) => {
                if let Some(old) = m.get_mut(&k) {
                    return Ok(Some(std::mem::replace(old, v)));
                }
                m.try_reserve(1).map_err(|_| allocation_error())?;
                Ok(m.insert(k, v))
            }
        }
    }
    pub(crate) fn get<Q: Hash + hashbrown::Equivalent<K> + ?Sized>(&self, key: &Q) -> Option<&V> {
        match self {
            Self::Plain(m) => m.get(key),
            Self::Owned(m) => m.get(key),
        }
    }
    pub(crate) fn get_mut<Q: Hash + hashbrown::Equivalent<K> + ?Sized>(
        &mut self,
        key: &Q,
    ) -> Option<&mut V> {
        match self {
            Self::Plain(m) => m.get_mut(key),
            Self::Owned(m) => m.get_mut(key),
        }
    }
    pub(crate) fn remove<Q: Hash + hashbrown::Equivalent<K> + ?Sized>(
        &mut self,
        key: &Q,
    ) -> Option<V> {
        match self {
            Self::Plain(m) => m.remove(key),
            Self::Owned(m) => m.remove(key),
        }
    }
    pub(crate) fn contains_key<Q: Hash + hashbrown::Equivalent<K> + ?Sized>(
        &self,
        key: &Q,
    ) -> bool {
        match self {
            Self::Plain(m) => m.contains_key(key),
            Self::Owned(m) => m.contains_key(key),
        }
    }
    pub(crate) fn iter(&self) -> hashbrown::hash_map::Iter<'_, K, V> {
        match self {
            Self::Plain(m) => m.iter(),
            Self::Owned(m) => m.iter(),
        }
    }
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Plain(m) => m.len(),
            Self::Owned(m) => m.len(),
        }
    }
    pub(crate) fn capacity(&self) -> usize {
        match self {
            Self::Plain(m) => m.capacity(),
            Self::Owned(m) => m.capacity(),
        }
    }
    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Self::Plain(m) => m.is_empty(),
            Self::Owned(m) => m.is_empty(),
        }
    }
}
pub(crate) enum WorkSet<T> {
    Plain(StdSet<T>),
    Owned(hashbrown::HashSet<T, RandomState, LedgerAllocator>),
}
impl<T> Default for WorkSet<T> {
    fn default() -> Self {
        Self::Plain(StdSet::new())
    }
}
impl<T: Eq + Hash> WorkSet<T> {
    pub(crate) fn new(b: Option<&OwnedBudget>) -> Self {
        match b {
            None => Self::default(),
            Some(b) => Self::Owned(hashbrown::HashSet::with_hasher_in(
                RandomState::new(),
                LedgerAllocator::new_tag(b, std::any::type_name::<T>()),
            )),
        }
    }
    pub(crate) fn insert(&mut self, v: T) -> Result<bool, CodecError> {
        match self {
            Self::Plain(m) => Ok(m.insert(v)),
            Self::Owned(m) => {
                if m.contains(&v) {
                    return Ok(false);
                }
                m.try_reserve(1).map_err(|_| allocation_error())?;
                Ok(m.insert(v))
            }
        }
    }
    pub(crate) fn contains(&self, v: &T) -> bool {
        match self {
            Self::Plain(m) => m.contains(v),
            Self::Owned(m) => m.contains(v),
        }
    }
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Plain(m) => m.len(),
            Self::Owned(m) => m.len(),
        }
    }
    pub(crate) fn capacity(&self) -> usize {
        match self {
            Self::Plain(m) => m.capacity(),
            Self::Owned(m) => m.capacity(),
        }
    }
    pub(crate) fn is_empty(&self) -> bool {
        match self {
            Self::Plain(m) => m.is_empty(),
            Self::Owned(m) => m.is_empty(),
        }
    }
}

pub struct CodecKey {
    bytes: CodecVec<u8>,
}
impl CodecKey {
    fn from_str(s: &str, b: &OwnedBudget) -> Result<Self, CodecError> {
        let mut bytes = CodecVec::new(Some(b));
        bytes.extend_from_slice(s.as_bytes())?;
        Ok(Self { bytes })
    }
}
impl Deref for CodecKey {
    type Target = str;
    fn deref(&self) -> &str {
        std::str::from_utf8(&self.bytes).expect("private key copied from valid UTF8 str")
    }
}
impl std::fmt::Debug for CodecKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}
enum FieldsStorage {
    Plain(BTreeMap<String, super::super::ContentRef>),
    Owned {
        entries: WorkVec<(CodecKey, super::super::ContentRef)>,
        budget: OwnedBudget,
    },
}
/// Canonical顺序结果owner：append摊还O(1)，binary-search O(log n)。
pub struct CodecFields {
    storage: FieldsStorage,
}
impl CodecFields {
    pub fn from_plain(m: BTreeMap<String, super::super::ContentRef>) -> Self {
        Self {
            storage: FieldsStorage::Plain(m),
        }
    }
    pub(crate) fn new(b: Option<&OwnedBudget>) -> Self {
        match b {
            None => Self::from_plain(BTreeMap::new()),
            Some(b) => Self {
                storage: FieldsStorage::Owned {
                    entries: WorkVec::new(Some(b)),
                    budget: b.clone(),
                },
            },
        }
    }
    pub(crate) fn append(
        &mut self,
        key: &str,
        value: super::super::ContentRef,
    ) -> Result<(), CodecError> {
        match &mut self.storage {
            FieldsStorage::Plain(m) => {
                m.insert(key.to_owned(), value);
                Ok(())
            }
            FieldsStorage::Owned { entries, budget } => {
                if entries.last().is_some_and(|(k, _)| k.deref() >= key) {
                    return Err(CodecError::static_detail(
                        super::super::code::OBJECT_KEY_ORDER,
                        "codec字段结果追加必须严格既有canonical序",
                    ));
                }
                let key = CodecKey::from_str(key, budget)?;
                entries.push((key, value))
            }
        }
    }
    pub fn get(&self, key: &str) -> Option<&super::super::ContentRef> {
        match &self.storage {
            FieldsStorage::Plain(m) => m.get(key),
            FieldsStorage::Owned { entries, .. } => entries
                .binary_search_by(|(k, _)| k.deref().cmp(key))
                .ok()
                .map(|i| &entries[i].1),
        }
    }
    pub fn len(&self) -> usize {
        match &self.storage {
            FieldsStorage::Plain(m) => m.len(),
            FieldsStorage::Owned { entries, .. } => entries.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub(crate) fn into_plain(
        self,
    ) -> Result<BTreeMap<String, super::super::ContentRef>, CodecError> {
        match self.storage {
            FieldsStorage::Plain(m) => Ok(m),
            FieldsStorage::Owned { .. } => Err(CodecError::static_detail(
                INCOMPLETE,
                "owned字段结果不可擦除为std BTreeMap",
            )),
        }
    }
}
impl Index<&str> for CodecFields {
    type Output = super::super::ContentRef;
    fn index(&self, k: &str) -> &Self::Output {
        self.get(k).expect("field not present")
    }
}
impl std::fmt::Debug for CodecFields {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.storage {
            FieldsStorage::Plain(m) => m.fmt(f),
            FieldsStorage::Owned { entries, .. } => f
                .debug_map()
                .entries(entries.iter().map(|(k, v)| (k.deref(), v)))
                .finish(),
        }
    }
}
impl PartialEq for CodecFields {
    fn eq(&self, o: &Self) -> bool {
        self.len() == o.len()
            && match &self.storage {
                FieldsStorage::Plain(m) => m.iter().all(|(k, v)| o.get(k) == Some(v)),
                FieldsStorage::Owned { entries, .. } => {
                    entries.iter().all(|(k, v)| o.get(k) == Some(v))
                }
            }
    }
}
impl Eq for CodecFields {}
#[cfg(test)]
mod allocator_tests {
    use super::*;
    #[test]
    fn ledger_grow_preflight_counts_old_new_and_preserves_old_on_failure() {
        for limit in [95, 96] {
            let b = OwnedBudget::new(limit);
            let mut v = CodecVec::<u64>::with_capacity(4, Some(&b)).unwrap();
            v.push(7).unwrap();
            v.push(9).unwrap();
            let before = b.snapshot();
            let r = v.reserve_exact(6);
            if limit == 95 {
                assert_eq!(r.unwrap_err().code(), LIMIT);
                assert_eq!(&*v, &[7, 9]);
                assert_eq!(v.capacity(), 4);
                assert_eq!(b.snapshot().allocated_layouts, before.allocated_layouts);
                assert_eq!(b.snapshot().allocation_attempts, before.allocation_attempts);
                assert_eq!(b.snapshot().live_requested_layout_bytes, 32);
            } else {
                r.unwrap();
                assert_eq!(&*v, &[7, 9]);
                assert_eq!(b.snapshot().peak_requested_layout_bytes, 96);
                assert_eq!(b.snapshot().live_requested_layout_bytes, 64);
            }
            println!(
                "ALLOC_GROW_RECEIPT {}",
                serde_json::to_string(&b.snapshot()).unwrap()
            );
            drop(v);
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn ledger_alignment_zst_overflow_clone_and_panic_drop() {
        let b = OwnedBudget::new(256);
        let a = LedgerAllocator::new(&b);
        let layout = Layout::from_size_align(17, 64).unwrap();
        let p = a.allocate(layout).unwrap();
        assert_eq!(p.as_ptr().cast::<u8>() as usize % 64, 0);
        unsafe { a.deallocate(p.cast(), layout) };
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        let mut z = CodecVec::<()>::new(Some(&b));
        for _ in 0..10 {
            z.push(()).unwrap();
        }
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        drop(z);
        let mut v = CodecVec::<u64>::new(Some(&b));
        let before = b.snapshot();
        let e = v.reserve_exact(usize::MAX).unwrap_err();
        assert_eq!(e.code(), LIMIT);
        assert!(matches!(e.detail, std::borrow::Cow::Borrowed(_)));
        assert_eq!(b.snapshot().allocation_attempts, before.allocation_attempts);
        v.push(42).unwrap();
        let c = v.try_clone().unwrap();
        assert_eq!(&*c, &[42]);
        assert!(b.snapshot().live_requested_layout_bytes >= 64);
        drop(c);
        drop(v);
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        struct Panics(u8);
        impl Drop for Panics {
            fn drop(&mut self) {
                panic!("intentional element drop")
            }
        }
        let mut p = CodecVec::with_capacity(1, Some(&b)).unwrap();
        p.push(Panics(1)).unwrap();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(p)));
        assert!(r.is_err());
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn ledger_hash_reserve_failure_preserves_existing_entries() {
        let b = OwnedBudget::new(128);
        let mut m = WorkMap::<u64, u64>::new(Some(&b));
        for i in 0..3 {
            m.insert(i, i + 10).unwrap();
        }
        let before = b.snapshot();
        let e = m.insert(3, 13).unwrap_err();
        assert_eq!(e.code(), LIMIT);
        for i in 0..3 {
            assert_eq!(m.get(&i), Some(&(i + 10)));
        }
        assert_eq!(m.get(&3), None);
        assert_eq!(b.snapshot().allocated_layouts, before.allocated_layouts);
        assert_eq!(b.snapshot().allocation_attempts, before.allocation_attempts);
        drop(m);
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
    }
}
