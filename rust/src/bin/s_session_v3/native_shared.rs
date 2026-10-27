//! #1488：同父 Native 许可随封闭强/弱句柄持有到真实共享块释放。
//!
//! 只为 std Rc/Arc 的内联载荷、计数头和自身 lease 块计结构预算，不认证 T 的
//! 动态子分配。调用方必须先让动态字段各持原许可；不得将 shallow 包装视为深准入。
//! 两计数头是当前 std 实现的结构模型，不是 Rust ABI 或 allocator/RSS 承诺。
use newchan_rust::s_content::{
    capture_allocation::{
        scratch::{NativeReservation, ScratchFailure, WorkScope},
        LIMIT,
    },
    CodecError,
};
use std::{
    alloc::Layout,
    borrow::Borrow,
    hash::{Hash, Hasher},
    ops::Deref,
    rc::{Rc, Weak as RcWeak},
    sync::{Arc, Weak as ArcWeak},
};

struct LeaseCore {
    // 内联 token 已包含在 lease 块的 Layout 内，无递归 metadata allocation。
    _permit: NativeReservation,
}

#[derive(Clone, Default)]
struct RcLease(Option<Rc<LeaseCore>>);
impl Drop for RcLease {
    fn drop(&mut self) {
        if let Some(lease) = self.0.take() {
            // lease 不产生 Weak；成功移出 core 时该 Rc 块已经释放。
            if let Ok(core) = Rc::try_unwrap(lease) {
                drop(core);
            }
        }
    }
}

#[derive(Clone, Default)]
struct ArcLease(Option<Arc<LeaseCore>>);
impl Drop for ArcLease {
    fn drop(&mut self) {
        if let Some(lease) = self.0.take() {
            // 每个句柄都走 into_inner，保证并发最后退出恰有一次提取。
            // try_unwrap().ok() 不能保证提取者存在；不得换成该写法。
            if let Some(core) = Arc::into_inner(lease) {
                drop(core);
            }
        }
    }
}

fn overflow(work: Option<&WorkScope>) -> CodecError {
    work.map_or_else(
        || CodecError::new(LIMIT, "Native共享块布局溢出"),
        |scope| scope.reject(ScratchFailure::Budget),
    )
}

fn shared_layout(payload: Layout) -> Option<Layout> {
    Layout::new::<[usize; 2]>()
        .extend(payload)
        .ok()
        .map(|(layout, _)| layout.pad_to_align())
}

fn shared_bytes(payload: Layout) -> Option<usize> {
    shared_layout(payload)?
        .size()
        .checked_add(shared_layout(Layout::new::<LeaseCore>())?.size())
}

fn reserve(
    payload: Layout,
    work: Option<&WorkScope>,
) -> Result<Option<NativeReservation>, CodecError> {
    let bytes = shared_bytes(payload).ok_or_else(|| overflow(work))?;
    work.map(|scope| scope.reserve_native(bytes)).transpose()
}

// 不对 value/lease 暴露任何引用或解包口。字段声明顺序承重：真实强/弱句柄先
// 析构，之后才释放 lease。Weak 保留的是整个 value 块，非仅两个计数器。
macro_rules! shared_family {
    ($strong:ident, $weak:ident, $ptr:ident, $weak_ptr:ident, $lease:ident) => {
        pub(crate) struct $strong<T: ?Sized> {
            value: $ptr<T>,
            lease: $lease,
        }
        #[cfg_attr(
            all(not(test), not(feature = "s_incremental")),
            allow(dead_code, reason = "#1488 再合 #1608：s_session 非测试 构建不触达")
        )]
        pub(crate) struct $weak<T: ?Sized> {
            value: $weak_ptr<T>,
            lease: $lease,
        }
        impl<T> $strong<T> {
            /// #1527 mixed缓存：同父仅共享；异父先核源健康，再由封闭字段工厂复制纯内容。
            /// 不携带任何Job/capture资格，调用者仍须完成fresh原件核验。
            #[allow(
                dead_code,
                reason = "#1527：共享宏的Rc/Arc实例消费者不同，保留同一父账接口"
            )]
            pub(crate) fn share_or_copy_mixed(
                &self,
                work: &WorkScope,
                copy: impl FnOnce(&T, &WorkScope) -> Result<T, CodecError>,
            ) -> Result<Self, CodecError> {
                work.check_current()?;
                self.check_retained_source()?;
                if self.belongs_to_parent(work) {
                    return Ok(self.clone());
                }
                let value = copy(&self.value, work)?;
                Self::try_new_shallow(value, Some(work))
            }
            /// 原无工作域入口；Native消费者使用try_new_shallow并显式传当前scope。
            #[allow(dead_code, reason = "#1488：8组合探针部分宏实例无调用方，保留共享接口")]
            pub(crate) fn new(value: T) -> Self {
                Self::try_new_shallow(value, None).expect("unscoped shared layout")
            }
            /// 只准入新共享块；value 的动态字段须已各自持费。None 仅供原无域入口。
            pub(crate) fn try_new_shallow(
                value: T,
                work: Option<&WorkScope>,
            ) -> Result<Self, CodecError> {
                let permit = reserve(Layout::new::<T>(), work)?;
                let lease = $lease(permit.map(|permit| $ptr::new(LeaseCore { _permit: permit })));
                Ok(Self {
                    value: $ptr::new(value),
                    lease,
                })
            }
        }
        impl<T> $strong<T> {
            #[allow(dead_code, reason = "#1488：8组合探针部分宏实例无调用方，保留共享接口")]
            pub(crate) fn try_unwrap(this: Self) -> Result<T, Self> {
                let Self { value, lease } = this;
                match $ptr::try_unwrap(value) {
                    Ok(value) => {
                        drop(lease);
                        Ok(value)
                    }
                    Err(value) => Err(Self { value, lease }),
                }
            }
        }
        impl<T: ?Sized> $strong<T> {
            #[cfg_attr(
                all(not(test), not(feature = "s_incremental")),
                allow(dead_code, reason = "#1488 再合 #1608：s_session 非测试 构建不触达")
            )]
            pub(crate) fn downgrade(this: &Self) -> $weak<T> {
                $weak {
                    value: $ptr::downgrade(&this.value),
                    lease: this.lease.clone(),
                }
            }
            /// #1527：只比较真实分配父；不能把该结果当Job/capture资格。
            #[allow(
                dead_code,
                reason = "#1527：共享宏的Rc/Arc实例消费者不同，保留同一父账接口"
            )]
            pub(crate) fn belongs_to_parent(&self, work: &WorkScope) -> bool {
                self.lease
                    .0
                    .as_ref()
                    .is_some_and(|lease| lease._permit.belongs_to_parent(work))
            }
            /// 同父共享或异父复制前核旧父健康；closed不是failed。
            #[allow(
                dead_code,
                reason = "#1527：共享宏的Rc/Arc实例消费者不同，保留同一父账接口"
            )]
            pub(crate) fn check_retained_source(&self) -> Result<(), CodecError> {
                self.lease
                    .0
                    .as_ref()
                    .map_or(Ok(()), |lease| lease._permit.check_retained_source())
            }
            #[allow(
                dead_code,
                reason = "#1527：共享宏的Rc/Arc实例消费者不同，保留同一父账接口"
            )]
            pub(crate) fn ptr_eq(this: &Self, other: &Self) -> bool {
                $ptr::ptr_eq(&this.value, &other.value)
            }
            /// 仅作身份比较用的地址；不暴露可解引用的原始句柄。
            #[allow(dead_code, reason = "#1488：8组合探针部分宏实例无调用方，保留共享接口")]
            pub(crate) fn as_ptr(this: &Self) -> *const T {
                $ptr::as_ptr(&this.value)
            }
            /// 保留标准库“唯一强且无弱”的条件，不按 lease 数量推断唯一性。
            /// 调用方仅能通过受准入字段操作改变动态内容；不提供 DerefMut/make_mut。
            #[cfg_attr(
                all(not(test), not(feature = "s_incremental")),
                allow(dead_code, reason = "#1488 再合 #1608：s_session 非测试 构建不触达")
            )]
            pub(crate) fn get_mut(this: &mut Self) -> Option<&mut T> {
                $ptr::get_mut(&mut this.value)
            }
            #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
            pub(crate) fn is_native_admitted(&self) -> bool {
                self.lease.0.is_some()
            }
            /// 共享块的结构预算；不含 T 动态子分配，不是退款额度或 RSS 读数。
            /// 按归属统计时每个拥有者可保守全计，不能据此重复签发父账许可。
            #[cfg_attr(
                not(feature = "s_incremental"),
                allow(
                    dead_code,
                    reason = "#1488 再合 #1608：非 s_capture_native_testonly 构建不触达"
                )
            )]
            pub(crate) fn allocation_bytes(&self) -> usize {
                let payload = Layout::for_value(self.value.as_ref());
                if self.lease.0.is_some() {
                    shared_bytes(payload).unwrap_or(usize::MAX)
                } else {
                    shared_layout(payload).map_or(usize::MAX, |layout| layout.size())
                }
            }
        }
        impl<T: ?Sized> Clone for $strong<T> {
            fn clone(&self) -> Self {
                Self {
                    value: self.value.clone(),
                    lease: self.lease.clone(),
                }
            }
        }
        impl<T: ?Sized> Deref for $strong<T> {
            type Target = T;
            fn deref(&self) -> &T {
                &self.value
            }
        }
        impl<T: ?Sized> AsRef<T> for $strong<T> {
            fn as_ref(&self) -> &T {
                &self.value
            }
        }
        impl<T: ?Sized> Borrow<T> for $strong<T> {
            fn borrow(&self) -> &T {
                &self.value
            }
        }
        impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for $strong<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.value.fmt(f)
            }
        }
        impl<T: ?Sized + PartialEq> PartialEq for $strong<T> {
            fn eq(&self, other: &Self) -> bool {
                self.value.as_ref().eq(other.value.as_ref())
            }
        }
        impl<T: ?Sized + Eq> Eq for $strong<T> {}
        impl<T: ?Sized + PartialOrd> PartialOrd for $strong<T> {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                self.value.as_ref().partial_cmp(other.value.as_ref())
            }
        }
        impl<T: ?Sized + Ord> Ord for $strong<T> {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.value.as_ref().cmp(other.value.as_ref())
            }
        }
        impl<T: ?Sized + Hash> Hash for $strong<T> {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.value.as_ref().hash(state);
            }
        }
        impl<T> $weak<T> {
            #[cfg_attr(
                all(not(test), not(feature = "s_incremental")),
                allow(dead_code, reason = "#1488 再合 #1608：s_session 非测试 构建不触达")
            )]
            pub(crate) fn new() -> Self {
                Self {
                    value: $weak_ptr::new(),
                    lease: $lease::default(),
                }
            }
        }
        impl<T> Default for $weak<T> {
            fn default() -> Self {
                Self::new()
            }
        }
        impl<T: ?Sized> $weak<T> {
            #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
            pub(crate) fn upgrade(&self) -> Option<$strong<T>> {
                self.value.upgrade().map(|value| $strong {
                    value,
                    lease: self.lease.clone(),
                })
            }
            /// #1527：只比较真实分配父；不能把该结果当Job/capture资格。
            #[allow(
                dead_code,
                reason = "#1527：共享宏的Rc/Arc实例消费者不同，保留同一父账接口"
            )]
            pub(crate) fn belongs_to_parent(&self, work: &WorkScope) -> bool {
                self.lease
                    .0
                    .as_ref()
                    .is_some_and(|lease| lease._permit.belongs_to_parent(work))
            }
            /// 同父共享或异父复制前核旧父健康；closed不是failed。
            #[allow(
                dead_code,
                reason = "#1527：共享宏的Rc/Arc实例消费者不同，保留同一父账接口"
            )]
            pub(crate) fn check_retained_source(&self) -> Result<(), CodecError> {
                self.lease
                    .0
                    .as_ref()
                    .map_or(Ok(()), |lease| lease._permit.check_retained_source())
            }
            #[allow(
                dead_code,
                reason = "#1527：共享宏的Rc/Arc实例消费者不同，保留同一父账接口"
            )]
            pub(crate) fn ptr_eq(this: &Self, other: &Self) -> bool {
                $weak_ptr::ptr_eq(&this.value, &other.value)
            }
            /// 仅作身份比较用的地址；与强句柄 as_ptr 同址。
            #[allow(dead_code, reason = "#1488：8组合探针部分宏实例无调用方，保留共享接口")]
            pub(crate) fn as_ptr(&self) -> *const T {
                self.value.as_ptr()
            }
        }
        impl<T: ?Sized> Clone for $weak<T> {
            fn clone(&self) -> Self {
                Self {
                    value: self.value.clone(),
                    lease: self.lease.clone(),
                }
            }
        }
        impl<T: ?Sized> std::fmt::Debug for $weak<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!(stringify!($weak), "(..)"))
            }
        }
    };
}

shared_family!(NativeRc, NativeWeak, Rc, RcWeak, RcLease);
shared_family!(NativeArc, NativeWeakArc, Arc, ArcWeak, ArcLease);

// 仅固定无动态子分配的 i64/u8；不提供可能暗分配的泛型 T: Clone copy。
macro_rules! slice_copy {
    ($strong:ident, $ptr:ident, $lease:ident, $element:ty) => {
        impl $strong<[$element]> {
            #[allow(
                dead_code,
                reason = "#1527：共享宏的Rc/Arc实例消费者不同，保留同一父账接口"
            )]
            pub(crate) fn share_or_copy_slice_mixed(
                &self,
                work: &WorkScope,
            ) -> Result<Self, CodecError> {
                work.check_current()?;
                self.check_retained_source()?;
                if self.belongs_to_parent(work) {
                    Ok(self.clone())
                } else {
                    Self::try_copy_slice(self, Some(work))
                }
            }
            #[cfg_attr(not(test), allow(dead_code, reason = "#1488 再合 #1608：仅测试使用"))]
            pub(crate) fn try_copy_slice(
                values: &[$element],
                work: Option<&WorkScope>,
            ) -> Result<Self, CodecError> {
                let payload =
                    Layout::array::<$element>(values.len()).map_err(|_| overflow(work))?;
                let permit = reserve(payload, work)?;
                let lease = $lease(permit.map(|permit| $ptr::new(LeaseCore { _permit: permit })));
                Ok(Self {
                    value: $ptr::from(values),
                    lease,
                })
            }
        }
    };
}
slice_copy!(NativeRc, Rc, RcLease, i64);
slice_copy!(NativeRc, Rc, RcLease, u8);
slice_copy!(NativeArc, Arc, ArcLease, i64);
slice_copy!(NativeArc, Arc, ArcLease, u8);

impl From<&[u8]> for NativeArc<[u8]> {
    fn from(bytes: &[u8]) -> Self {
        Self::try_copy_slice(bytes, None).expect("unscoped byte layout")
    }
}
impl From<Vec<u8>> for NativeArc<[u8]> {
    fn from(bytes: Vec<u8>) -> Self {
        Self::from(bytes.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use newchan_rust::s_content::capture_allocation::{
        native_owned::NativeValue,
        scratch::{CodecWork, ScratchCharge, ScratchParent},
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Barrier, Mutex,
    };

    #[derive(Default)]
    struct Counts {
        live: usize,
        native: usize,
        native_releases: usize,
        broken: bool,
    }
    struct Parent {
        limit: usize,
        counts: Mutex<Counts>,
    }
    impl Parent {
        fn new(limit: usize) -> Arc<Self> {
            Arc::new(Self {
                limit,
                counts: Mutex::new(Counts::default()),
            })
        }
        fn live(&self) -> usize {
            self.counts.lock().unwrap().live
        }
        fn native(&self) -> usize {
            self.counts.lock().unwrap().native
        }
        fn releases(&self) -> usize {
            self.counts.lock().unwrap().native_releases
        }
    }
    impl ScratchParent for Parent {
        fn reserve(&self, bytes: usize, kind: ScratchCharge) -> Result<u64, ScratchFailure> {
            let mut c = self.counts.lock().unwrap();
            let next = c.live.checked_add(bytes).ok_or(ScratchFailure::Budget)?;
            if next > self.limit {
                return Err(ScratchFailure::Budget);
            }
            c.live = next;
            if matches!(kind, ScratchCharge::Native) {
                c.native += bytes;
            }
            Ok(0)
        }
        fn release(
            &self,
            bytes: usize,
            kind: ScratchCharge,
            origin: u64,
        ) -> Result<(), ScratchFailure> {
            if origin != 0 {
                return Err(ScratchFailure::Ledger);
            }
            let mut c = self.counts.lock().unwrap();
            c.live = c.live.checked_sub(bytes).ok_or(ScratchFailure::Ledger)?;
            if matches!(kind, ScratchCharge::Native) {
                c.native = c.native.checked_sub(bytes).ok_or(ScratchFailure::Ledger)?;
                c.native_releases += 1;
            }
            Ok(())
        }
        fn health(&self) -> Result<(), ScratchFailure> {
            if self.counts.lock().unwrap().broken {
                Err(ScratchFailure::Ledger)
            } else {
                Ok(())
            }
        }
        fn invalidate(&self) {
            self.counts.lock().unwrap().broken = true;
        }
    }
    fn run<T>(parent: &Arc<Parent>, f: impl FnOnce(&WorkScope) -> Result<T, CodecError>) -> T {
        CodecWork::new(parent.clone(), parent.limit)
            .unwrap()
            .run(f)
            .unwrap()
    }

    #[test]
    fn rc_dynamic_payload_releases_at_last_strong_but_entire_block_waits_for_weak() {
        let parent = Parent::new(1 << 20);
        let value = run(&parent, |work| {
            NativeRc::try_new_shallow(
                NativeValue::<String>::try_from_str("payload", Some(work))?,
                Some(work),
            )
        });
        let blocks = shared_bytes(Layout::new::<NativeValue<String>>()).unwrap();
        assert_eq!(parent.native(), blocks + 7);
        let copy = value.clone();
        let weak = NativeRc::downgrade(&value);
        let weak_copy = weak.clone();
        assert!(NativeRc::ptr_eq(&value, &weak.upgrade().unwrap()));
        assert!(NativeWeak::ptr_eq(&weak, &weak_copy));
        assert_eq!(parent.native(), blocks + 7);
        drop(value);
        assert_eq!(parent.native(), blocks + 7);
        drop(copy);
        assert!(weak.upgrade().is_none());
        assert_eq!(parent.native(), blocks);
        drop(weak);
        assert_eq!(parent.native(), blocks);
        drop(weak_copy);
        assert_eq!(parent.live(), 0);
        assert_eq!(parent.releases(), 2);
        assert!(parent.health().is_ok());
    }

    #[test]
    fn get_mut_preserves_std_strong_and_weak_exclusivity() {
        let parent = Parent::new(1 << 20);
        let mut value = run(&parent, |work| NativeArc::try_new_shallow(9u64, Some(work)));
        *NativeArc::get_mut(&mut value).unwrap() = 10;
        let clone = value.clone();
        assert!(NativeArc::get_mut(&mut value).is_none());
        drop(clone);
        let weak = NativeArc::downgrade(&value);
        assert!(NativeArc::get_mut(&mut value).is_none());
        drop(weak);
        assert_eq!(NativeArc::get_mut(&mut value), Some(&mut 10));
        drop(value);
        assert_eq!(parent.live(), 0);
    }

    #[test]
    fn slices_use_content_hash_and_borrow_but_pointer_identity_is_distinct() {
        let parent = Parent::new(1 << 20);
        let (left, right, bytes, empty) = run(&parent, |work| {
            Ok((
                NativeRc::<[i64]>::try_copy_slice(&[1, 3], Some(work))?,
                NativeRc::<[i64]>::try_copy_slice(&[1, 3], Some(work))?,
                NativeArc::<[u8]>::try_copy_slice(b"abc", Some(work))?,
                NativeArc::<[i64]>::try_copy_slice(&[], Some(work))?,
            ))
        });
        assert_eq!(left, right);
        assert!(!NativeRc::ptr_eq(&left, &right));
        let mut map = std::collections::HashMap::new();
        map.insert(left.clone(), 7);
        assert_eq!(map.get(&[1i64, 3][..]), Some(&7));
        assert_eq!(bytes.as_ref(), b"abc");
        assert!(empty.is_empty());
        let weak = NativeArc::downgrade(&bytes);
        let before = parent.native();
        drop(bytes);
        assert_eq!(parent.native(), before);
        assert!(weak.upgrade().is_none());
        drop((weak, left, right, empty, map));
        assert_eq!(parent.live(), 0);
    }

    #[test]
    fn empty_weak_and_unscoped_compatibility_have_no_native_lease() {
        let weak = NativeWeak::<u8>::new();
        let atomic = NativeWeakArc::<u8>::new();
        assert!(weak.upgrade().is_none());
        assert!(atomic.upgrade().is_none());
        assert!(weak.lease.0.is_none() && atomic.lease.0.is_none());
        let mut value = NativeRc::try_new_shallow(1u8, None).unwrap();
        assert!(!value.is_native_admitted());
        *NativeRc::get_mut(&mut value).unwrap() = 2;
        assert_eq!(*value, 2);
        let bytes = NativeRc::<[u8]>::try_copy_slice(b"x", None).unwrap();
        assert!(!bytes.is_native_admitted());
    }

    #[test]
    fn resource_failure_keeps_existing_owner_and_exact_limit_succeeds() {
        let block = shared_bytes(Layout::new::<u64>()).unwrap();
        let parent = Parent::new(CodecWork::metadata_bytes() + 2 * block - 1);
        let old = run(&parent, |work| NativeRc::try_new_shallow(17u64, Some(work)));
        let failure = CodecWork::new(parent.clone(), parent.limit)
            .unwrap()
            .run(|work| NativeRc::try_new_shallow(19u64, Some(work)));
        assert!(failure.is_err());
        assert_eq!(*old, 17);
        assert_eq!(parent.native(), block);
        drop(old);
        assert_eq!(parent.live(), 0);
        let exact = Parent::new(CodecWork::metadata_bytes() + block);
        let value = run(&exact, |work| NativeArc::try_new_shallow(2u64, Some(work)));
        assert!(value.is_native_admitted());
        assert_eq!(exact.live(), block);
        drop(value);
        assert_eq!(exact.live(), 0);
    }

    #[test]
    fn alignment_and_checked_layout_overflow_do_not_use_unpadded_payload_size() {
        #[repr(align(256))]
        struct Aligned([u8; 3]);
        let parent = Parent::new(1 << 20);
        let value = run(&parent, |work| {
            NativeRc::try_new_shallow(Aligned([1, 2, 3]), Some(work))
        });
        assert_eq!((&*value as *const Aligned as usize) % 256, 0);
        assert_eq!(value.0, [1, 2, 3]);
        let padded = shared_layout(Layout::new::<Aligned>()).unwrap();
        assert_eq!(padded.align(), 256);
        assert_eq!(padded.size(), 512);
        assert_eq!(
            parent.native(),
            shared_bytes(Layout::new::<Aligned>()).unwrap()
        );
        drop(value);
        assert_eq!(parent.live(), 0);
        let impossible = Layout::from_size_align(isize::MAX as usize, 1).unwrap();
        assert!(shared_bytes(impossible).is_none());
        assert!(Layout::array::<i64>(usize::MAX).is_err());
    }

    #[test]
    fn arc_concurrent_last_strong_and_weak_release_once() {
        struct CountDrop(Arc<AtomicUsize>);
        impl Drop for CountDrop {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let parent = Parent::new(1 << 20);
        let drops = Arc::new(AtomicUsize::new(0));
        let value = run(&parent, |work| {
            NativeArc::try_new_shallow(CountDrop(drops.clone()), Some(work))
        });
        let barrier = Arc::new(Barrier::new(17));
        let mut threads = Vec::new();
        for _ in 0..8 {
            let held = value.clone();
            let ready = barrier.clone();
            threads.push(std::thread::spawn(move || {
                ready.wait();
                drop(held);
            }));
            let weak = NativeArc::downgrade(&value);
            let ready = barrier.clone();
            threads.push(std::thread::spawn(move || {
                ready.wait();
                drop(weak);
            }));
        }
        drop(value);
        barrier.wait();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(parent.releases(), 1);
        assert_eq!(parent.live(), 0);
        assert!(parent.health().is_ok());
    }

    #[test]
    fn arc_does_not_refund_while_payload_destructor_is_still_running() {
        struct BlockingDrop {
            entered: Arc<Barrier>,
            leave: Arc<Barrier>,
        }
        impl Drop for BlockingDrop {
            fn drop(&mut self) {
                self.entered.wait();
                self.leave.wait();
            }
        }
        let parent = Parent::new(1 << 20);
        let entered = Arc::new(Barrier::new(2));
        let leave = Arc::new(Barrier::new(2));
        let value = run(&parent, |work| {
            NativeArc::try_new_shallow(
                BlockingDrop {
                    entered: entered.clone(),
                    leave: leave.clone(),
                },
                Some(work),
            )
        });
        let weak = NativeArc::downgrade(&value);
        let before = parent.native();
        let thread = std::thread::spawn(move || drop(value));
        entered.wait();
        let upgrade_failed = weak.upgrade().is_none();
        drop(weak);
        let during = parent.native();
        leave.wait();
        thread.join().unwrap();
        assert!(upgrade_failed);
        assert_eq!(during, before);
        assert_eq!(parent.releases(), 1);
        assert_eq!(parent.live(), 0);
    }

    #[test]
    fn payload_unwind_drops_payload_before_lease_and_keeps_weak_tail() {
        struct Panics;
        impl Drop for Panics {
            fn drop(&mut self) {
                panic!("payload drop sentinel");
            }
        }
        let parent = Parent::new(1 << 20);
        let value = run(&parent, |work| {
            NativeRc::try_new_shallow(Panics, Some(work))
        });
        let weak = NativeRc::downgrade(&value);
        let before = parent.native();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(value)));
        assert!(result.is_err());
        assert!(weak.upgrade().is_none());
        assert_eq!(parent.native(), before);
        drop(weak);
        assert_eq!(parent.live(), 0);
        assert_eq!(parent.releases(), 1);
    }
}
