//! #1394 Native codec 独立工作域。父账回调只做原子记账，不分配、不回入 codec。
use super::super::CodecError;
use std::alloc::Layout;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum ScratchFailure {
    Budget = 1,
    Allocation = 2,
    Ledger = 3,
}

#[derive(Clone, Copy, Debug)]
pub enum ScratchCharge {
    Metadata,
    Layout,
    /// Native/Plain 业务容器的结构上界，独立于 scratch 的真实 Layout。
    Native,
}

/// Native 容器许可；只在实际载荷析构后释放，或保守留在父账到父作用域结束。
/// 它不改变业务值的 Plain/Owned 归属，也不写入 Owned requested Layout 账。
pub struct NativeReservation {
    parent: Arc<dyn ScratchParent>,
    bytes: usize,
    origin: u64,
}
impl NativeReservation {
    /// #1488：只消费父账已有的未分配原件费；不签发新额度。
    /// #1488 P3-a：生产父账（trait默认）恒拒且无生产调用方，只随测试/Native测试臂编译。
    #[cfg(any(test, feature = "s_incremental"))]
    #[doc(hidden)]
    pub fn adopt(
        parent: Arc<dyn ScratchParent>,
        bytes: usize,
        expected: u64,
    ) -> Result<Self, ScratchFailure> {
        let origin = parent.adopt_native(bytes, expected)?;
        Ok(Self {
            parent,
            bytes,
            origin,
        })
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// 只核父账身份；有效操作域仍由本次reserve_native检查。
    #[doc(hidden)]
    pub fn belongs_to_parent(&self, work: &WorkScope) -> bool {
        self.belongs_to_parent_arc(&work.handle.core().parent)
    }
    pub(super) fn belongs_to_parent_arc(&self, parent: &Arc<dyn ScratchParent>) -> bool {
        Arc::ptr_eq(&self.parent, parent)
    }
    /// #1527：正常closed父的保留载荷仍可读；失败父或来源失效不得复制洗白。
    pub fn check_retained_source(&self) -> Result<(), CodecError> {
        self.parent.health().map_err(CodecError::scratch)?;
        if self.parent.origin() != self.origin {
            return Err(CodecError::scratch(ScratchFailure::Ledger));
        }
        Ok(())
    }
    /// 同级所有权容器只比较父身份，不授予当前操作域写资格。
    #[doc(hidden)]
    pub(super) fn same_parent(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.parent, &other.parent)
    }
    /// #1488：分拆只移动既有费用，不能复制许可或跨来源合并。
    pub fn split(&mut self, bytes: usize) -> Result<Self, ScratchFailure> {
        let remaining = self
            .bytes
            .checked_sub(bytes)
            .ok_or(ScratchFailure::Ledger)?;
        self.bytes = remaining;
        Ok(Self {
            parent: self.parent.clone(),
            bytes,
            origin: self.origin,
        })
    }
    pub fn merge(&mut self, other: &mut Self) -> Result<(), ScratchFailure> {
        if !self.same_parent(other) || self.origin != other.origin {
            return Err(ScratchFailure::Ledger);
        }
        let bytes = self
            .bytes
            .checked_add(other.bytes)
            .ok_or(ScratchFailure::Ledger)?;
        self.bytes = bytes;
        other.bytes = 0;
        Ok(())
    }
    pub fn hold<T>(self, value: T) -> super::native_owned::NativeValue<T> {
        super::native_owned::NativeValue::from_reserved(value, Some(self))
    }
}
impl Drop for NativeReservation {
    fn drop(&mut self) {
        if self
            .parent
            .release(self.bytes, ScratchCharge::Native, self.origin)
            .is_err()
        {
            self.parent.invalidate();
        }
    }
}

/// 实现方必须先原子预留总额，再更新观测分项；不得分别检查分项后相加。
pub trait ScratchParent: Send + Sync {
    fn reserve(&self, bytes: usize, kind: ScratchCharge) -> Result<u64, ScratchFailure>;
    fn reserve_in_origin(
        &self,
        bytes: usize,
        kind: ScratchCharge,
        expected: u64,
    ) -> Result<u64, ScratchFailure> {
        if self.origin() != expected {
            return Err(ScratchFailure::Ledger);
        }
        let origin = self.reserve(bytes, kind)?;
        if origin != expected {
            self.release(bytes, kind, origin)?;
            return Err(ScratchFailure::Ledger);
        }
        Ok(origin)
    }
    fn release(&self, bytes: usize, kind: ScratchCharge, origin: u64)
        -> Result<(), ScratchFailure>;
    /// 单域测试父账使用0；生产父账返回当前冻结域。
    fn origin(&self) -> u64 {
        0
    }
    #[cfg(any(test, feature = "s_incremental"))]
    fn adopt_native(&self, _bytes: usize, _expected: u64) -> Result<u64, ScratchFailure> {
        Err(ScratchFailure::Ledger)
    }
    fn close_scope(&self, _origin: u64) -> Result<(), ScratchFailure> {
        Ok(())
    }
    fn health(&self) -> Result<(), ScratchFailure>;
    fn invalidate(&self);
}

// 首次非资源失败只作分类标记；原错误仍由 Result 原样返回。
const SEMANTIC_FAILURE: usize = 4;

struct ScratchCore {
    parent: Arc<dyn ScratchParent>,
    limit: usize,
    metadata_charge: usize,
    origin: u64,
    live: AtomicUsize,
    peak: AtomicUsize,
    failure: AtomicUsize,
    attempts: AtomicUsize,
    active: AtomicBool,
}

/// 所有强引用都经此包装；不暴露 Arc/Weak，父账不反持子账。
pub(crate) struct ScratchHandle(Option<Arc<ScratchCore>>);
impl Clone for ScratchHandle {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl Drop for ScratchHandle {
    fn drop(&mut self) {
        // Rust 1.98 Arc::into_inner 在返回 core 前已释放隐式 Weak 和控制块。
        // 不能用 try_unwrap().ok()：并发最后 Drop 时可能无人收到账。
        if let Some(core) = Arc::into_inner(self.0.take().expect("scratch handle")) {
            let parent = core.parent.clone();
            let charge = core.metadata_charge;
            let origin = core.origin;
            if core.live.load(Ordering::Acquire) != 0 {
                parent.invalidate();
            }
            drop(core);
            if parent
                .release(charge, ScratchCharge::Metadata, origin)
                .is_err()
            {
                parent.invalidate();
            }
        }
    }
}
impl ScratchHandle {
    fn close(&self) {
        if self.core().active.swap(false, Ordering::AcqRel) {
            if let Err(error) = self.core().parent.close_scope(self.core().origin) {
                self.fail(error);
                self.core().parent.invalidate();
            }
        }
    }
    pub(crate) fn reserve_native(&self, bytes: usize) -> Result<NativeReservation, CodecError> {
        self.check_write()?;
        let parent = self.core().parent.clone();
        let origin = parent
            .reserve_in_origin(bytes, ScratchCharge::Native, self.core().origin)
            .map_err(|failure| {
                self.fail(failure);
                CodecError::scratch(failure)
            })?;
        Ok(NativeReservation {
            parent,
            bytes,
            origin,
        })
    }
    fn core(&self) -> &ScratchCore {
        self.0.as_ref().expect("scratch handle")
    }
    pub(crate) fn failure(&self) -> Option<ScratchFailure> {
        match self.core().failure.load(Ordering::Acquire) {
            0 => None,
            1 => Some(ScratchFailure::Budget),
            2 => Some(ScratchFailure::Allocation),
            3 => Some(ScratchFailure::Ledger),
            _ => None,
        }
    }
    pub(crate) fn fail(&self, failure: ScratchFailure) {
        if traced(failure) == ScratchFailure::Ledger {
            let _ = self
                .core()
                .failure
                .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                    (n != SEMANTIC_FAILURE).then_some(failure as usize)
                });
        } else {
            let _ = self.core().failure.compare_exchange(
                0,
                failure as usize,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }
    pub(crate) fn observe(&self, error: &CodecError) {
        if traced_semantic(error.scratch_failure().is_none()) {
            let _ = self.core().failure.compare_exchange(
                0,
                SEMANTIC_FAILURE,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }
    /// 写操作即使复用已有容量，也必须仍处于健康的活动域。
    fn check_write(&self) -> Result<(), CodecError> {
        if !self.core().active.load(Ordering::Acquire)
            || self.core().origin != self.core().parent.origin()
        {
            return Err(CodecError::scratch(ScratchFailure::Ledger));
        }
        if let Some(failure) = self.failure() {
            return Err(CodecError::scratch(failure));
        }
        if self.core().failure.load(Ordering::Acquire) == SEMANTIC_FAILURE {
            return Err(CodecError::static_detail(
                super::INCOMPLETE,
                "codec 工作域已因语义错误结束",
            ));
        }
        if let Err(failure) = self.core().parent.health() {
            self.fail(failure);
            return Err(CodecError::scratch(failure));
        }
        Ok(())
    }
    pub(crate) fn reserve(&self, layout: Layout) -> Result<(), ScratchFailure> {
        if let Some(e) = self.failure() {
            return Err(e);
        }
        if !self.core().active.load(Ordering::Acquire)
            || self.core().failure.load(Ordering::Acquire) == SEMANTIC_FAILURE
        {
            return Err(ScratchFailure::Ledger);
        }
        let c = self.core();
        let bytes = layout.size();
        if bytes == 0 {
            return Ok(());
        }
        // 父先、子后；回调在任何子锁之外（本核心没有锁）。
        let origin = c
            .parent
            .reserve_in_origin(bytes, ScratchCharge::Layout, c.origin)
            .map_err(|e| {
                self.fail(e);
                e
            })?;
        if origin != c.origin {
            let _ = c.parent.release(bytes, ScratchCharge::Layout, origin);
            self.fail(ScratchFailure::Ledger);
            return Err(ScratchFailure::Ledger);
        }
        if c.live
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                n.checked_add(bytes).filter(|n| *n <= c.limit)
            })
            .is_err()
        {
            self.fail(ScratchFailure::Budget);
            if let Err(e) = c.parent.release(bytes, ScratchCharge::Layout, c.origin) {
                self.fail(e);
            }
            return Err(self.failure().unwrap());
        }
        c.peak
            .fetch_max(c.live.load(Ordering::Acquire), Ordering::Relaxed);
        if c.attempts
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
            .is_err()
        {
            self.fail(ScratchFailure::Ledger);
            self.release(layout);
            return Err(ScratchFailure::Ledger);
        }
        Ok(())
    }
    pub(crate) fn release(&self, layout: Layout) {
        if layout.size() == 0 {
            return;
        }
        let c = self.core();
        if c.live
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                n.checked_sub(layout.size())
            })
            .is_err()
        {
            self.fail(ScratchFailure::Ledger);
            return;
        }
        if let Err(e) = c
            .parent
            .release(layout.size(), ScratchCharge::Layout, c.origin)
        {
            self.fail(e);
        }
    }
    pub(crate) fn error(&self) -> CodecError {
        if self.core().failure.load(Ordering::Acquire) == SEMANTIC_FAILURE {
            return CodecError::static_detail(super::INCOMPLETE, "codec 工作域已因语义错误结束");
        }
        if !self.core().active.load(Ordering::Acquire) {
            self.fail(ScratchFailure::Ledger);
        }
        // try_reserve 的容量溢出不会进入 Allocator，也属于资源拒绝。
        if self.failure().is_none() {
            self.fail(ScratchFailure::Budget);
        }
        CodecError::scratch(self.failure().unwrap())
    }
}

/// 一个 session 只执行一次；Scope 不可 Send/Sync，不能并行共享或内层重置失败原因。
pub struct CodecWork {
    handle: ScratchHandle,
    _exclusive: PhantomData<Rc<()>>,
}
pub struct WorkScope {
    handle: ScratchHandle,
    _exclusive: PhantomData<Rc<()>>,
}
#[derive(Debug)]
pub enum WorkError<E> {
    Resource(ScratchFailure),
    Operation(E),
}
#[derive(Clone, Copy, Debug)]
pub struct WorkSnapshot {
    pub live: usize,
    pub peak: usize,
    pub attempts: usize,
}
impl CodecWork {
    /// 固定当前 Rust 1.98 ArcInner: strong, weak, data；不是永久 ABI。
    /// Arc::new 的系统 OOM 仍服从 std abort 合同，元数据额度在它之前严格准入。
    pub fn metadata_bytes() -> usize {
        Layout::new::<[AtomicUsize; 2]>()
            .extend(Layout::new::<ScratchCore>())
            .expect("Arc layout")
            .0
            .pad_to_align()
            .size()
    }
    pub fn new(parent: Arc<dyn ScratchParent>, limit: usize) -> Result<Self, ScratchFailure> {
        let origin = parent.origin();
        Self::new_in_origin(parent, limit, origin)
    }
    #[doc(hidden)]
    pub fn new_in_origin(
        parent: Arc<dyn ScratchParent>,
        limit: usize,
        origin: u64,
    ) -> Result<Self, ScratchFailure> {
        let metadata_charge = Self::metadata_bytes();
        let origin = parent.reserve_in_origin(metadata_charge, ScratchCharge::Metadata, origin)?;
        Ok(Self {
            handle: ScratchHandle(Some(Arc::new(ScratchCore {
                parent,
                limit,
                metadata_charge,
                origin,
                live: AtomicUsize::new(0),
                peak: AtomicUsize::new(0),
                failure: AtomicUsize::new(0),
                attempts: AtomicUsize::new(0),
                active: AtomicBool::new(true),
            }))),
            _exclusive: PhantomData,
        })
    }
    /// f 必须按首次 Err 短路，不得吞掉失败再继续；嵌套 codec 使用同一个 Scope。
    /// 单域通道跨现有 String 桥保存资源原因，不根据错误文字或稳定 code 推断。
    pub fn run<T, E>(self, f: impl FnOnce(&WorkScope) -> Result<T, E>) -> Result<T, WorkError<E>> {
        let scope = WorkScope {
            handle: self.handle.clone(),
            _exclusive: PhantomData,
        };
        let result = f(&scope);
        scope.handle.close();
        let failure = scope.handle.failure();
        let semantic_first =
            scope.handle.core().failure.load(Ordering::Acquire) == SEMANTIC_FAILURE;
        let parent = scope.handle.core().parent.clone();
        drop(scope);
        drop(self);
        // 最后控制块释放后再检查父健康；已发生的语义失败不被清理错误覆盖。
        let failure = if semantic_first {
            None
        } else {
            parent.health().err().or(failure)
        };
        match (result, failure) {
            (Err(e), None) => Err(WorkError::Operation(e)),
            (_, Some(e)) => Err(WorkError::Resource(e)),
            (Ok(v), None) => Ok(v),
        }
    }
}
impl Drop for CodecWork {
    fn drop(&mut self) {
        self.handle.close();
    }
}
impl WorkScope {
    /// #1488：无增长/替换也先核当前域；不预留零字节、不改变账本观察值。
    pub fn check_current(&self) -> Result<(), CodecError> {
        self.handle.check_write()
    }
    #[cfg(any(test, feature = "s_incremental"))]
    pub(crate) fn adopt_native(&self, bytes: usize) -> Result<NativeReservation, CodecError> {
        self.check_current()?;
        NativeReservation::adopt(
            self.handle.core().parent.clone(),
            bytes,
            self.handle.core().origin,
        )
        .map_err(|e| self.reject(e))
    }
    /// 实际 Plain 分配之前预留；溢出/容量不足沿当前独占操作域分类。
    pub fn reserve_native(&self, bytes: usize) -> Result<NativeReservation, CodecError> {
        self.handle.reserve_native(bytes)
    }
    /// 父 Native 解析许可等同域资源拒绝，分类不穿过错误文字。
    pub fn reject(&self, failure: ScratchFailure) -> CodecError {
        self.handle.fail(failure);
        CodecError::scratch(failure)
    }

    pub(crate) fn budget(&self) -> super::work::WorkBudget {
        super::work::WorkBudget::Scratch(self.handle.clone())
    }
    pub fn snapshot(&self) -> WorkSnapshot {
        let c = self.handle.core();
        WorkSnapshot {
            live: c.live.load(Ordering::Acquire),
            peak: c.peak.load(Ordering::Acquire),
            attempts: c.attempts.load(Ordering::Acquire),
        }
    }
    /// 临时输出只能借 bytes 消费；Owned 容器不能擦除为 Plain。
    pub fn bytes(&self) -> ScratchBytes {
        self.vector()
    }
    pub fn vector<T>(&self) -> ScratchVec<T> {
        ScratchVec {
            bytes: super::work::CodecVec::new_work(Some(&self.budget())),
            scope: self.handle.clone(),
            _exclusive: PhantomData,
        }
    }
}

/// 私有 CodecVec 的借字节门；不可 Send/Sync、不可拆出底层容器或擦为 Plain。
/// 即使调用者把它带出 run，元数据仍随真实块保留，结束域只能读取，写入与扩容均拒绝。
pub type ScratchBytes = ScratchVec<u8>;
pub struct ScratchVec<T> {
    bytes: super::work::CodecVec<T>,
    scope: ScratchHandle,
    _exclusive: PhantomData<Rc<()>>,
}
impl ScratchVec<u8> {
    pub fn try_extend_bytes(&mut self, bytes: &[u8]) -> Result<(), CodecError> {
        self.scope.check_write()?;
        self.bytes.try_extend_bytes(bytes)
    }
}
impl<T> ScratchVec<T> {
    pub fn try_push(&mut self, value: T) -> Result<(), CodecError> {
        self.scope.check_write()?;
        self.bytes.try_push(value)
    }
    pub fn try_reserve_exact(&mut self, n: usize) -> Result<(), CodecError> {
        self.scope.check_write()?;
        self.bytes.try_reserve_exact(n)
    }
    pub fn capacity(&self) -> usize {
        self.bytes.capacity()
    }
}
impl<T> std::ops::Deref for ScratchVec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.bytes
    }
}

// #1488 0072：类型化失败旁路挂点。挂点替换原表达式且行号中性；生产构建下均为恒等函数，
// 不记录、不改变行为。记录只在 `failure_trace::capture` 活动期间、且仅 testonly 构建发生。
#[inline(always)]
pub(crate) fn traced(failure: ScratchFailure) -> ScratchFailure {
    #[cfg(feature = "s_incremental")]
    failure_trace::record(failure);
    failure
}
#[inline(always)]
pub(crate) fn traced_semantic(semantic: bool) -> bool {
    #[cfg(feature = "s_incremental")]
    if semantic {
        failure_trace::record_semantic();
    }
    semantic
}
#[cfg(feature = "s_incremental")]
pub mod failure_trace {
    use super::ScratchFailure;
    use crate::s_content::{CodecError, ContextError};
    use std::cell::Cell;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) struct FailureOrigin {
        capture: u64,
        born: u64,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) struct BridgeProof {
        origin: FailureOrigin,
        bridged: u64,
        failure: ScratchFailure,
    }
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct FailureTrace {
        pub budget: usize,
        pub allocation: usize,
        pub ledger: usize,
        pub semantic: usize,
        pub bridged: usize,
        capture: Option<u64>,
        start: u64,
        end: u64,
    }
    impl FailureTrace {
        pub fn only_resource(&self) -> bool {
            self.ledger == 0 && self.semantic == 0 && self.budget + self.allocation > 0
        }
        pub fn bridges(&self, error: &ContextError) -> bool {
            error.proof.is_some_and(|proof| {
                self.capture == Some(proof.origin.capture)
                    && self.start < proof.origin.born
                    && proof.origin.born < proof.bridged
                    && proof.bridged <= self.end
                    && matches!(proof.failure, ScratchFailure::Budget | ScratchFailure::Allocation)
            })
        }
        fn absorb(&mut self, other: FailureTrace) {
            self.budget = self.budget.saturating_add(other.budget);
            self.allocation = self.allocation.saturating_add(other.allocation);
            self.ledger = self.ledger.saturating_add(other.ledger);
            self.semantic = self.semantic.saturating_add(other.semantic);
            self.bridged = self.bridged.saturating_add(other.bridged);
        }
    }
    static NEXT_CAPTURE: AtomicU64 = AtomicU64::new(1);
    thread_local! {
        static ACTIVE: Cell<Option<FailureTrace>> = const { Cell::new(None) };
        static ROOT: Cell<Option<u64>> = const { Cell::new(None) };
        static EVENT: Cell<u64> = const { Cell::new(0) };
    }
    fn next_event() -> Option<u64> {
        EVENT.with(|event| {
            let next = event.get().checked_add(1)?;
            event.set(next);
            Some(next)
        })
    }
    pub(crate) fn born() -> Option<FailureOrigin> {
        let capture = ROOT.with(Cell::get)?;
        Some(FailureOrigin { capture, born: next_event()? })
    }
    pub(crate) fn bridge(error: &CodecError) -> Option<BridgeProof> {
        let failure = error.scratch_failure?;
        if !matches!(failure, ScratchFailure::Budget | ScratchFailure::Allocation) {
            return None;
        }
        let origin = error.origin?;
        if ROOT.with(Cell::get) != Some(origin.capture) {
            return None;
        }
        let proof = BridgeProof { origin, bridged: next_event()?, failure };
        update(|trace| trace.bridged = trace.bridged.saturating_add(1));
        Some(proof)
    }
    fn update(f: impl FnOnce(&mut FailureTrace)) {
        let _ = ACTIVE.try_with(|active| {
            if let Some(mut trace) = active.get() {
                f(&mut trace);
                active.set(Some(trace));
            }
        });
    }
    pub(super) fn record(failure: ScratchFailure) {
        update(|t| match failure {
            ScratchFailure::Budget => t.budget = t.budget.saturating_add(1),
            ScratchFailure::Allocation => t.allocation = t.allocation.saturating_add(1),
            ScratchFailure::Ledger => t.ledger = t.ledger.saturating_add(1),
        });
    }
    pub(super) fn record_semantic() {
        update(|t| t.semantic = t.semantic.saturating_add(1));
    }
    pub fn capture<T>(f: impl FnOnce() -> T) -> (T, FailureTrace) {
        struct Restore {
            outer: Option<Option<FailureTrace>>,
            root: Option<u64>,
            event: u64,
            top: bool,
        }
        impl Drop for Restore {
            fn drop(&mut self) {
                if let Some(outer) = self.outer.take() {
                    ACTIVE.with(|active| active.set(outer));
                }
                if self.top {
                    ROOT.with(|root| root.set(self.root));
                    EVENT.with(|event| event.set(self.event));
                }
            }
        }
        let outer = ACTIVE.with(Cell::get);
        let mut restore = Restore {
            outer: Some(outer),
            root: ROOT.with(Cell::get),
            event: EVENT.with(Cell::get),
            top: outer.is_none(),
        };
        if restore.top {
            let id = NEXT_CAPTURE.try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1)).ok();
            ROOT.with(|root| root.set(id));
            EVENT.with(|event| event.set(0));
        }
        ACTIVE.with(|active| active.set(Some(FailureTrace {
            capture: ROOT.with(Cell::get),
            start: EVENT.with(Cell::get),
            ..FailureTrace::default()
        })));
        let value = f();
        let mut trace = ACTIVE.with(|active| active.replace(restore.outer.take().unwrap())).unwrap();
        trace.end = EVENT.with(Cell::get);
        drop(restore);
        update(|outer| outer.absorb(trace));
        (value, trace)
    }
}

/// #1488 0077 P55：测试面——以真实 `observe` 挂点（`traced_semantic`）登记一次非 scratch codec
/// 错误，供 bin 侧降级判据真值表构造“语义失败”行。生产构建无此方法；置于文件末尾，生产行号不动。
#[cfg(all(feature = "s_incremental", feature = "s_capture_native_testonly"))]
impl WorkScope {
    #[doc(hidden)]
    pub fn observe_for_test(&self, error: &CodecError) {
        self.handle.observe(error);
    }
}
