//! #1394 完整候选的附加保留费。只用于 Native；不改变 Owned 分配域。
//! 容量按实际 Vec/String capacity 登记；树/散列表用保守槽位上界（含控制区），
//! 不是 allocator/RSS 实测。共享 owner 每个归属完整收费，不按 Rc/Arc 去重。
//! 全量 oracle 的饱和值只可用于准入；增量账遇溢出必须撤候选，不能再减费。
use newchan_rust::s_content::{self as sc, capture_allocation::audit_json::BusinessMap};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    mem::size_of,
};

pub(crate) trait Retained {
    /// 不含 self 的 inline 大小（由外层容器计），包含所有从属分配。
    fn heap_bytes(&self) -> usize;
}

/// #1394：只记录实际变更处的保留费，不索引或重访旧历史。
/// overflow 独立于数值保存，禁止把饱和 MAX 当作可继续减费的余额。
#[derive(Default, Debug)]
pub(crate) struct RetainedDelta {
    added: usize,
    released: usize,
    overflow: bool,
}
impl RetainedDelta {
    pub(crate) fn add_bytes(&mut self, bytes: usize) {
        match self
            .added
            .checked_add(bytes)
            .filter(|_| bytes != usize::MAX)
        {
            Some(added) => self.added = added,
            None => self.overflow = true,
        }
    }
    pub(crate) fn release_bytes(&mut self, bytes: usize) {
        match self
            .released
            .checked_add(bytes)
            .filter(|_| bytes != usize::MAX)
        {
            Some(released) => self.released = released,
            None => self.overflow = true,
        }
    }
    pub(crate) fn add(&mut self, value: &impl Retained) {
        self.add_bytes(value.heap_bytes());
    }
    pub(crate) fn release(&mut self, value: &impl Retained) {
        self.release_bytes(value.heap_bytes());
    }
    pub(crate) fn merge(&mut self, other: Self) {
        self.overflow |= other.overflow;
        self.add_bytes(other.added);
        self.release_bytes(other.released);
    }
    fn capacity(&mut self, before: Option<usize>, after: Option<usize>) {
        match (before, after) {
            (Some(before), Some(after)) if before != usize::MAX && after != usize::MAX => {
                if after >= before {
                    self.add_bytes(after - before);
                } else {
                    self.release_bytes(before - after);
                }
            }
            _ => self.overflow = true,
        }
    }
    pub(crate) fn vec_capacity<T>(&mut self, before: usize, after: usize) {
        self.capacity(
            before.checked_mul(size_of::<T>()),
            after.checked_mul(size_of::<T>()),
        );
    }
    pub(crate) fn hash_capacity<K, V>(&mut self, before: usize, after: usize) {
        fn bytes<K, V>(capacity: usize) -> Option<usize> {
            if capacity == 0 {
                return Some(0);
            }
            capacity
                .checked_mul(2)?
                .checked_add(16)?
                .checked_mul(size_of::<(K, V)>().checked_add(1)?)
        }
        self.capacity(bytes::<K, V>(before), bytes::<K, V>(after));
    }
    pub(crate) fn tree_len<K, V>(&mut self, before: usize, after: usize) {
        let slot = size_of::<(K, V)>()
            .checked_mul(11)
            .and_then(|n| n.checked_add(16 * size_of::<usize>()));
        self.capacity(
            slot.and_then(|n| before.checked_mul(n)),
            slot.and_then(|n| after.checked_mul(n)),
        );
    }
    /// BTree 删除最后一项后可能仍留空根，清空后才允许按零项释放。
    pub(crate) fn tree_remove<K: Ord + Retained>(&mut self, set: &mut BTreeSet<K>, key: &K) {
        if let Some(old) = set.take(key) {
            self.release(&old);
            self.tree_len::<K, ()>(1, 0);
            if set.is_empty() {
                set.clear();
            }
        }
    }
    pub(crate) fn tree_insert<K: Ord + Retained>(&mut self, set: &mut BTreeSet<K>, key: K) {
        let bytes = key.heap_bytes();
        if set.insert(key) {
            self.add_bytes(bytes);
            self.tree_len::<K, ()>(0, 1);
        }
    }
    pub(crate) fn apply(
        self,
        retained: usize,
    ) -> Result<usize, super::audit_owner::native::Failure> {
        use super::audit_owner::native::Failure;
        if self.overflow || retained == usize::MAX {
            return Err(Failure::BudgetExceeded);
        }
        let total = retained
            .checked_add(self.added)
            .filter(|total| *total != usize::MAX)
            .ok_or(Failure::BudgetExceeded)?;
        total
            .checked_sub(self.released)
            .ok_or_else(|| Failure::Invalid("StorageUnavailable：历史保留费账本下溢".into()))
    }
}
pub(crate) fn sum(values: impl IntoIterator<Item = usize>) -> usize {
    values.into_iter().fold(0usize, usize::saturating_add)
}
pub(crate) fn vec_bytes<T>(capacity: usize) -> usize {
    capacity.saturating_mul(size_of::<T>())
}
pub(crate) fn hash_slots<K, V>(capacity: usize) -> usize {
    if capacity == 0 {
        return 0;
    }
    // SwissTable buckets >= capacity；2*capacity+16 覆盖装载率/小表/控制哨兵。
    capacity
        .saturating_mul(2)
        .saturating_add(16)
        .saturating_mul(size_of::<(K, V)>().saturating_add(1))
}
pub(crate) fn tree_slots<K, V>(len: usize) -> usize {
    // 每个条目最多独占一棵含 11 key/value、12 edge 的节点；上界刻意不减费。
    len.saturating_mul(
        size_of::<(K, V)>()
            .saturating_mul(11)
            .saturating_add(16 * size_of::<usize>()),
    )
}
macro_rules! scalar {
    ($($t:ty),*) => { $(impl Retained for $t { fn heap_bytes(&self)->usize {0} })* };
}
scalar!((), u8, u32, u64, usize, i64, bool, sc::Built);
impl Retained for String {
    fn heap_bytes(&self) -> usize {
        self.capacity()
    }
}
impl<T: Retained> Retained for Option<T> {
    fn heap_bytes(&self) -> usize {
        self.as_ref().map_or(0, Retained::heap_bytes)
    }
}
impl<T: Retained> Retained for Vec<T> {
    fn heap_bytes(&self) -> usize {
        sum(std::iter::once(vec_bytes::<T>(self.capacity()))
            .chain(self.iter().map(Retained::heap_bytes)))
    }
}
impl<T: Retained, const N: usize> Retained for [T; N] {
    fn heap_bytes(&self) -> usize {
        sum(self.iter().map(Retained::heap_bytes))
    }
}
impl<T: Retained> Retained for sc::CodecVec<T> {
    fn heap_bytes(&self) -> usize {
        sum(std::iter::once(vec_bytes::<T>(self.capacity()))
            .chain(self.iter().map(Retained::heap_bytes)))
    }
}
impl<K: Retained + Eq + std::hash::Hash, V: Retained> Retained for HashMap<K, V> {
    fn heap_bytes(&self) -> usize {
        sum(std::iter::once(hash_slots::<K, V>(self.capacity())).chain(
            self.iter()
                .map(|(k, v)| sum([k.heap_bytes(), v.heap_bytes()])),
        ))
    }
}
impl<K: Retained + Eq + std::hash::Hash, V: Retained> Retained for BusinessMap<K, V> {
    fn heap_bytes(&self) -> usize {
        sum(std::iter::once(hash_slots::<K, V>(self.capacity())).chain(
            self.iter()
                .map(|(k, v)| sum([k.heap_bytes(), v.heap_bytes()])),
        ))
    }
}
impl<K: Retained, V: Retained> Retained for BTreeMap<K, V> {
    fn heap_bytes(&self) -> usize {
        sum(std::iter::once(tree_slots::<K, V>(self.len())).chain(
            self.iter()
                .map(|(k, v)| sum([k.heap_bytes(), v.heap_bytes()])),
        ))
    }
}
impl<K: Retained> Retained for BTreeSet<K> {
    fn heap_bytes(&self) -> usize {
        sum(std::iter::once(tree_slots::<K, ()>(self.len()))
            .chain(self.iter().map(Retained::heap_bytes)))
    }
}
impl<T: Retained> Retained for std::sync::Arc<T> {
    fn heap_bytes(&self) -> usize {
        sum([
            2 * size_of::<usize>(),
            size_of::<T>(),
            (**self).heap_bytes(),
        ])
    }
}
impl<T: Retained> Retained for std::rc::Rc<T> {
    fn heap_bytes(&self) -> usize {
        sum([
            2 * size_of::<usize>(),
            size_of::<T>(),
            (**self).heap_bytes(),
        ])
    }
}
impl<A: Retained, B: Retained> Retained for (A, B) {
    fn heap_bytes(&self) -> usize {
        sum([self.0.heap_bytes(), self.1.heap_bytes()])
    }
}
impl<A: Retained, B: Retained, C: Retained> Retained for (A, B, C) {
    fn heap_bytes(&self) -> usize {
        sum([
            self.0.heap_bytes(),
            self.1.heap_bytes(),
            self.2.heap_bytes(),
        ])
    }
}
impl Retained for serde_json::Value {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::String(v) => v.heap_bytes(),
            Self::Array(v) => v.heap_bytes(),
            Self::Object(v) => sum(std::iter::once(tree_slots::<String, Self>(v.len()))
                .chain(v.iter().map(|(k, v)| sum([k.heap_bytes(), v.heap_bytes()])))),
            _ => 0,
        }
    }
}
impl Retained for super::business_json::PlainJson {
    fn heap_bytes(&self) -> usize {
        self.retained_heap_bytes()
    }
}
impl Retained for super::business_json::TextValue {
    fn heap_bytes(&self) -> usize {
        self.capacity()
    }
}
impl Retained for super::audit_store::RefValue {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Plain(r) => sum([r.format.capacity(), r.root.capacity()]),
            Self::Owned(_) => 0,
        }
    }
}

// 穷尽解构让新增字段不能默默漏账；只在声明类型的模块内展开以保持字段私有。
macro_rules! retained_struct {
    ($t:ty {$($field:ident),* $(,)?}) => {
        #[cfg(feature="s_capture_native_testonly")]
        impl crate::v3::audit_retention::Retained for $t {
            fn heap_bytes(&self)->usize {
                let Self {$($field),*} = self;
                crate::v3::audit_retention::sum([$(crate::v3::audit_retention::Retained::heap_bytes($field)),*])
            }
        }
    };
}
pub(crate) use retained_struct;

// 统计载荷上界与 RAII 退账独立；这里绝不根据统计差释放 Native 许可。
impl<T: Retained> Retained for sc::capture_allocation::native_owned::NativeValue<T> {
    fn heap_bytes(&self) -> usize {
        self.as_ref()
            .heap_bytes()
            .max(self.retained_charge().unwrap_or(0))
    }
}
impl<K: Ord + Retained, V: Retained> Retained for super::audit_store::NativeTree<K, V> {
    fn heap_bytes(&self) -> usize {
        sum(std::iter::once(self.allocation_bytes()).chain(
            self.iter()
                .map(|(k, v)| sum([k.heap_bytes(), v.heap_bytes()])),
        ))
    }
}

impl<T: Retained> Retained for super::native_shared::NativeRc<T> {
    fn heap_bytes(&self) -> usize {
        sum([self.allocation_bytes(), self.as_ref().heap_bytes()])
    }
}
