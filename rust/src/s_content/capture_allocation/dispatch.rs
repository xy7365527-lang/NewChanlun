//! #1394 调度owner：实际refcount+payload Layout、擦除任务Layout和ring存储。
//! 不安装全局allocator；None沿同调度核使用std；Some失败绝不回退。
//! std线程的启动packet/name、系统栈和外部io错误仍未资格化。
use super::{
    work::{allocation_error, LedgerAllocator},
    CodecError, OwnedBudget,
};
use allocator_api2::{alloc::Allocator, boxed::Box as AllocBox};
use std::{
    ops::Deref,
    ptr::NonNull,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Instant,
};

// allocator-api2 0.2.21 Box::drop does NOT guard deallocation against T::drop panic.
// Every initialized dispatch-owned allocation therefore uses this local unwind guard.
struct DeallocationGuard {
    ptr: NonNull<u8>,
    layout: std::alloc::Layout,
    allocator: LedgerAllocator,
}
impl Drop for DeallocationGuard {
    fn drop(&mut self) {
        // SAFETY: sole owning dispatch box/final Arc created this guard before dropping T;
        // ptr/layout/allocator are precisely those of the live allocation. Value destruction
        // does not free its outer allocation, including unwind. Zero-size is allocator no-op.
        unsafe {
            self.allocator.deallocate(self.ptr, self.layout);
        }
    }
}
struct PanicSafeBox<T: ?Sized> {
    ptr: NonNull<T>,
    allocator: LedgerAllocator,
}
impl<T: ?Sized> PanicSafeBox<T> {
    fn as_mut(&mut self) -> &mut T {
        // SAFETY: sole owner, initialized T, lifetime tied to exclusive self borrow;
        // no raw pointer/other reference escapes the private holder.
        unsafe { self.ptr.as_mut() }
    }
}
impl<T: ?Sized> Drop for PanicSafeBox<T> {
    fn drop(&mut self) {
        // SAFETY: pointer still owns initialized T; get its dynamic Layout before destructor
        // (which may invalidate contents). Fat pointer metadata came from compiler unsizing.
        let layout = unsafe { std::alloc::Layout::for_value(self.ptr.as_ref()) };
        let _free = DeallocationGuard {
            ptr: self.ptr.cast(),
            layout,
            allocator: self.allocator.clone(),
        };
        // SAFETY: this is sole owner; drop T exactly once. Guard frees allocation on success
        // and unwinding, so a panicking destructor cannot leak the actual block or ledger.
        unsafe {
            std::ptr::drop_in_place(self.ptr.as_ptr());
        }
    }
}
// SAFETY: sole-owner pointer moves together with allocator; T:Send permits value transfer.
// No Sync impl: erased tasks are only borrowed mutably by the worker that owns them.
unsafe impl<T: ?Sized + Send> Send for PanicSafeBox<T> {}

struct ArcBlock<T> {
    refs: AtomicUsize,
    value: T,
}
enum ArcStorage<T> {
    Plain(Arc<T>),
    Owned {
        ptr: NonNull<ArcBlock<T>>,
        allocator: LedgerAllocator,
    },
}
/// 强引用共享持有者；不支持Weak，也不允许裸指针或可变引用逃逸。
///
/// 非Sync payload不得跨线程共享或转移：
/// ```compile_fail
/// use newchan_rust::s_content::capture_allocation::dispatch::DispatchArc;
/// let value = DispatchArc::try_new(std::cell::Cell::new(0), None).ok().unwrap();
/// std::thread::spawn(move || value.set(1));
/// ```
pub struct DispatchArc<T> {
    storage: ArcStorage<T>,
}
impl<T> DispatchArc<T> {
    pub fn try_new(value: T, budget: Option<&OwnedBudget>) -> Result<Self, (CodecError, T)> {
        let Some(budget) = budget else {
            return Ok(Self {
                storage: ArcStorage::Plain(Arc::new(value)),
            });
        };
        let allocator = LedgerAllocator::new_tag(budget, std::any::type_name::<ArcBlock<T>>());
        let mut slot = match AllocBox::<ArcBlock<T>, _>::try_new_uninit_in(allocator) {
            Ok(slot) => slot,
            Err(_) => return Err((allocation_error(), value)),
        };
        slot.write(ArcBlock {
            refs: AtomicUsize::new(1),
            value,
        });
        // SAFETY: whole ArcBlock was initialized exactly once above. Its true alignment/Layout
        // are those allocated by try_new_uninit_in, and no fallible work follows initialization.
        let block = unsafe { slot.assume_init() };
        let (ptr, allocator) = AllocBox::into_raw_with_allocator(block);
        Ok(Self {
            storage: ArcStorage::Owned {
                ptr: NonNull::new(ptr).expect("Box nonnull"),
                allocator,
            },
        })
    }
}
impl<T> Clone for DispatchArc<T> {
    fn clone(&self) -> Self {
        match &self.storage {
            ArcStorage::Plain(value) => Self {
                storage: ArcStorage::Plain(Arc::clone(value)),
            },
            ArcStorage::Owned { ptr, allocator } => {
                // SAFETY: self holds one strong ref, so block remains live for this operation.
                // Only atomic refcount is mutated; value is never mutated through this handle.
                let refs = unsafe { &ptr.as_ref().refs };
                let mut old = refs.load(Ordering::Relaxed);
                loop {
                    // Same hard bound as std Arc: forbid wrapping into an early final release.
                    if old >= isize::MAX as usize {
                        std::process::abort();
                    }
                    match refs.compare_exchange_weak(
                        old,
                        old + 1,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => break,
                        Err(current) => old = current,
                    }
                }
                Self {
                    storage: ArcStorage::Owned {
                        ptr: *ptr,
                        allocator: allocator.clone(),
                    },
                }
            }
        }
    }
}
impl<T> Deref for DispatchArc<T> {
    type Target = T;
    fn deref(&self) -> &T {
        match &self.storage {
            ArcStorage::Plain(value) => value,
            // SAFETY: this strong ref pins the initialized block. Shared-only reference has
            // lifetime tied to self; mutable access/Weak/raw-pointer escape are unavailable.
            ArcStorage::Owned { ptr, .. } => unsafe { &ptr.as_ref().value },
        }
    }
}
impl<T> Drop for DispatchArc<T> {
    fn drop(&mut self) {
        if let ArcStorage::Owned { ptr, allocator } = &self.storage {
            // SAFETY: this live handle owns one counted ref. Release synchronizes every prior
            // access with the Acquire fence of the sole final ref before value destruction.
            if unsafe { ptr.as_ref().refs.fetch_sub(1, Ordering::Release) } != 1 {
                return;
            }
            std::sync::atomic::fence(Ordering::Acquire);
            let _free = DeallocationGuard {
                ptr: ptr.cast(),
                layout: std::alloc::Layout::new::<ArcBlock<T>>(),
                allocator: allocator.clone(),
            };
            // SAFETY: count transitioned 1->0, so sole final owner drops initialized block
            // exactly once. No aliases remain after Acquire fence. Local guard owns the true
            // original Layout/allocator and frees even when T::drop unwinds.
            unsafe {
                std::ptr::drop_in_place(ptr.as_ptr());
            }
        }
    }
}
// SAFETY: atomic strong ownership permits transfer/sharing only if T itself permits both;
// nonnull is never independently dereferenced or exposed. LedgerAllocator is Arc<Mutex>-backed.
unsafe impl<T: Send + Sync> Send for DispatchArc<T> {}
// SAFETY: same proof as Send; only shared references to T and atomic refcount access exist.
unsafe impl<T: Send + Sync> Sync for DispatchArc<T> {}

enum PathStorage {
    Plain(std::path::PathBuf),
    Owned(super::work::CodecVec<u8>),
}
/// 路径编码字节的真实owner；Some不保留caller PathBuf的堆副本。
/// 编码字节只能由当前平台有效OsStr完整复制；私有存储不开放修改或外来字节。
pub struct DispatchPath {
    storage: PathStorage,
}
impl DispatchPath {
    pub fn try_from_path(
        path: std::path::PathBuf,
        budget: Option<&OwnedBudget>,
    ) -> Result<Self, (CodecError, std::path::PathBuf)> {
        let Some(budget) = budget else {
            return Ok(Self {
                storage: PathStorage::Plain(path),
            });
        };
        let mut bytes = super::work::CodecVec::new(Some(budget));
        if let Err(error) = bytes.try_extend_bytes(path.as_os_str().as_encoded_bytes()) {
            return Err((error, path));
        }
        Ok(Self {
            storage: PathStorage::Owned(bytes),
        })
    }
}
impl AsRef<std::path::Path> for DispatchPath {
    fn as_ref(&self) -> &std::path::Path {
        match &self.storage {
            PathStorage::Plain(path) => path,
            PathStorage::Owned(bytes) => {
                // SAFETY: bytes were copied whole from OsStr::as_encoded_bytes on this target
                // and Rust runtime. Private storage permits no mutation/slicing/import of bytes.
                // The borrowed OsStr lifetime is tied to self, whose vector remains live.
                std::path::Path::new(unsafe {
                    std::ffi::OsStr::from_encoded_bytes_unchecked(bytes)
                })
            }
        }
    }
}

/// 调度核的三段动作。执行与拒绝先暂存结果，完成必须由调用方在释放槽后调用。
pub trait DispatchTask<A, B, C>: Send {
    /// true 表示任务panic且reader需要重置；实现须把任务panic转为暂存终态。
    fn run(&mut self, a: &mut A, b: &B, c: &C, queued_at: Instant) -> bool;
    fn reject(&mut self, reason: String) -> bool;
    fn complete(&mut self);
}
enum TaskStorage<A, B, C> {
    Plain(Box<dyn DispatchTask<A, B, C>>),
    Owned(PanicSafeBox<dyn DispatchTask<A, B, C>>),
}
/// 动态任务只分配一块实际具体J Layout；擦除后仍保留原allocator和drop vtable。
pub struct DispatchOwner<A, B, C> {
    storage: TaskStorage<A, B, C>,
}
impl<A, B, C> DispatchOwner<A, B, C> {
    pub fn try_new<J: DispatchTask<A, B, C> + 'static>(
        job: J,
        budget: Option<&OwnedBudget>,
    ) -> Result<Self, (CodecError, J)> {
        let Some(budget) = budget else {
            return Ok(Self {
                storage: TaskStorage::Plain(Box::new(job)),
            });
        };
        let allocator = LedgerAllocator::new_tag(budget, std::any::type_name::<J>());
        let mut slot = match AllocBox::<J, _>::try_new_uninit_in(allocator) {
            Ok(slot) => slot,
            Err(_) => return Err((allocation_error(), job)),
        };
        slot.write(job);
        // SAFETY: full J initialized once; concrete alignment/Layout retained by allocator.
        let boxed = unsafe { slot.assume_init() };
        let (ptr, allocator) = AllocBox::into_raw_with_allocator(boxed);
        let erased: *mut dyn DispatchTask<A, B, C> = ptr;
        // Compiler unsizing preserves J data pointer/vtable. Private sole owner retains
        // original allocator; PanicSafeBox obtains J's true dyn Layout before destruction.
        Ok(Self {
            storage: TaskStorage::Owned(PanicSafeBox {
                ptr: NonNull::new(erased).expect("Box nonnull"),
                allocator,
            }),
        })
    }
    fn task(&mut self) -> &mut dyn DispatchTask<A, B, C> {
        match &mut self.storage {
            TaskStorage::Plain(v) => v.as_mut(),
            TaskStorage::Owned(v) => v.as_mut(),
        }
    }
    pub fn run(&mut self, a: &mut A, b: &B, c: &C, at: Instant) -> bool {
        self.task().run(a, b, c, at)
    }
    pub fn reject(&mut self, reason: String) -> bool {
        self.task().reject(reason)
    }
    pub fn complete(&mut self) {
        self.task().complete()
    }
}

/// Ring slots在实际增长前reserve；旧队列及尚未接纳input在拒绝时保持原所有权。
pub struct DispatchQueue<T> {
    slots: super::work::CodecVec<Option<T>>,
    head: usize,
    len: usize,
}
impl<T> DispatchQueue<T> {
    pub fn new(budget: Option<&OwnedBudget>) -> Self {
        Self {
            slots: super::work::CodecVec::new(budget),
            head: 0,
            len: 0,
        }
    }
    pub fn reserve_one(&mut self) -> Result<(), CodecError> {
        if self.len == self.slots.len() {
            self.slots.try_reserve(1)?;
            self.slots.rotate_left(self.head);
            self.head = 0;
            // Capacity is already granted; push cannot allocate or fail now.
            self.slots.try_push(None)?;
        }
        Ok(())
    }
    pub fn push_reserved(&mut self, value: T) {
        assert!(
            self.len < self.slots.len(),
            "reserve_one must precede dispatch insertion"
        );
        let tail = (self.head + self.len) % self.slots.len();
        debug_assert!(self.slots[tail].is_none());
        self.slots[tail] = Some(value);
        self.len += 1;
    }
    pub fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let value = self.slots[self.head].take();
        self.head = (self.head + 1) % self.slots.len();
        self.len -= 1;
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    #[test]
    fn dispatch_arc_actual_layout_alignment_clone_and_panic_drop() {
        #[repr(align(128))]
        struct Aligned(AtomicBool);
        let b = OwnedBudget::new(4096);
        let a = DispatchArc::try_new(Aligned(AtomicBool::new(false)), Some(&b))
            .ok()
            .unwrap();
        assert_eq!((&*a as *const Aligned as usize) % 128, 0);
        assert_eq!(
            b.snapshot().live_requested_layout_bytes,
            std::mem::size_of::<ArcBlock<Aligned>>()
        );
        let c = a.clone();
        std::thread::spawn(move || {
            c.0.store(true, Ordering::Release);
        })
        .join()
        .unwrap();
        assert!(a.0.load(Ordering::Acquire));
        drop(a);
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        struct Panics;
        impl Drop for Panics {
            fn drop(&mut self) {
                panic!("drop fixture")
            }
        }
        let a = DispatchArc::try_new(Panics, Some(&b)).ok().unwrap();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(a))).is_err());
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn dispatch_arc_refusal_returns_original_value_and_ring_keeps_fifo() {
        let b = OwnedBudget::new(0);
        let (_, value) = DispatchArc::try_new(37u64, Some(&b)).err().unwrap();
        assert_eq!(value, 37);
        assert_eq!(b.snapshot().allocated_layouts, 0);
        for owned in [false, true] {
            let b = OwnedBudget::new(8192);
            let mut q = DispatchQueue::new(owned.then_some(&b));
            for n in 0..3 {
                q.reserve_one().unwrap();
                q.push_reserved(n);
            }
            assert_eq!(q.pop_front(), Some(0));
            for n in 3..10 {
                q.reserve_one().unwrap();
                q.push_reserved(n);
            }
            for n in 1..10 {
                assert_eq!(q.pop_front(), Some(n));
            }
            assert_eq!(q.pop_front(), None);
            drop(q);
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
    }
    struct TestTask {
        dropped: Arc<AtomicUsize>,
        panic_drop: bool,
    }
    impl DispatchTask<(), (), ()> for TestTask {
        fn run(&mut self, _: &mut (), _: &(), _: &(), _: Instant) -> bool {
            false
        }
        fn reject(&mut self, _: String) -> bool {
            false
        }
        fn complete(&mut self) {}
    }
    impl Drop for TestTask {
        fn drop(&mut self) {
            self.dropped.fetch_add(1, Ordering::SeqCst);
            if self.panic_drop {
                panic!("erased drop fixture")
            }
        }
    }
    #[test]
    fn dispatch_erased_owner_exact_layout_refusal_and_panicking_destructor() {
        let dropped = Arc::new(AtomicUsize::new(0));
        let zero = OwnedBudget::new(0);
        let (_, original) = DispatchOwner::<(), (), ()>::try_new(
            TestTask {
                dropped: dropped.clone(),
                panic_drop: false,
            },
            Some(&zero),
        )
        .err()
        .unwrap();
        assert_eq!(dropped.load(Ordering::SeqCst), 0);
        assert_eq!(zero.snapshot().allocated_layouts, 0);
        drop(original);
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
        let b = OwnedBudget::new(4096);
        let mut owner = DispatchOwner::try_new(
            TestTask {
                dropped: dropped.clone(),
                panic_drop: true,
            },
            Some(&b),
        )
        .ok()
        .unwrap();
        assert_eq!(
            b.snapshot().live_requested_layout_bytes,
            std::mem::size_of::<TestTask>()
        );
        assert!(!owner.run(&mut (), &(), &(), Instant::now()));
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(owner))).is_err());
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        assert_eq!(dropped.load(Ordering::SeqCst), 2);
        struct Empty;
        impl DispatchTask<(), (), ()> for Empty {
            fn run(&mut self, _: &mut (), _: &(), _: &(), _: Instant) -> bool {
                false
            }
            fn reject(&mut self, _: String) -> bool {
                false
            }
            fn complete(&mut self) {}
        }
        let z = DispatchOwner::try_new(Empty, Some(&zero)).ok().unwrap();
        drop(z);
        assert_eq!(zero.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn dispatch_path_copy_failure_preserves_source_and_encoded_bytes_roundtrip() {
        let source = std::path::PathBuf::from("caller/dispatch-owner.sqlite");
        let zero = OwnedBudget::new(0);
        let (_, source) = DispatchPath::try_from_path(source, Some(&zero))
            .err()
            .unwrap();
        assert_eq!(source, std::path::Path::new("caller/dispatch-owner.sqlite"));
        for owned in [false, true] {
            let b = OwnedBudget::new(4096);
            let path = DispatchPath::try_from_path(source.clone(), owned.then_some(&b))
                .ok()
                .unwrap();
            assert_eq!(path.as_ref(), source);
            drop(path);
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let source =
                std::path::PathBuf::from(std::ffi::OsString::from_vec(vec![b'/', 255, b'a']));
            let b = OwnedBudget::new(4096);
            let path = DispatchPath::try_from_path(source.clone(), Some(&b))
                .ok()
                .unwrap();
            assert_eq!(path.as_ref(), source);
            drop(path);
            assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
        }
    }
}
