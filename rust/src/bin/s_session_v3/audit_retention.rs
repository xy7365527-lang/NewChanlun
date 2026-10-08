//! #1394 完整候选的附加保留费。只用于 Native；不改变 Owned 分配域。
//! 容量按实际 Vec/String capacity 登记；树/散列表用保守槽位上界（含控制区），
//! 不是 allocator/RSS 实测。共享 owner 每个归属完整收费，不按 Rc/Arc 去重。
//! 饱和溢出为 usize::MAX，候选因此不保留，不能改变已提交的业务结果。
use newchan_rust::s_content::{self as sc, capture_allocation::audit_json::BusinessMap};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    mem::size_of,
};

pub(crate) trait Retained {
    /// 不含 self 的 inline 大小（由外层容器计），包含所有从属分配。
    fn heap_bytes(&self) -> usize;
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
