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
}
impl NativeReservation {
    pub fn retain(mut self) {
        self.bytes = 0;
    }
}
impl Drop for NativeReservation {
    fn drop(&mut self) {
        if self
            .parent
            .release(self.bytes, ScratchCharge::Native)
            .is_err()
        {
            self.parent.invalidate();
        }
    }
}

/// 实现方必须先原子预留总额，再更新观测分项；不得分别检查分项后相加。
pub trait ScratchParent: Send + Sync {
    fn reserve(&self, bytes: usize, kind: ScratchCharge) -> Result<(), ScratchFailure>;
    fn release(&self, bytes: usize, kind: ScratchCharge) -> Result<(), ScratchFailure>;
    fn health(&self) -> Result<(), ScratchFailure>;
    fn invalidate(&self);
}

// 首次非资源失败只作分类标记；原错误仍由 Result 原样返回。
const SEMANTIC_FAILURE: usize = 4;

struct ScratchCore {
    parent: Arc<dyn ScratchParent>,
    limit: usize,
    metadata_charge: usize,
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
            if core.live.load(Ordering::Acquire) != 0 {
                parent.invalidate();
            }
            drop(core);
            if parent.release(charge, ScratchCharge::Metadata).is_err() {
                parent.invalidate();
            }
        }
    }
}
impl ScratchHandle {
    pub(crate) fn reserve_native(&self, bytes: usize) -> Result<NativeReservation, CodecError> {
        self.check_write()?;
        let parent = self.core().parent.clone();
        parent
            .reserve(bytes, ScratchCharge::Native)
            .map_err(|failure| {
                self.fail(failure);
                CodecError::scratch(failure)
            })?;
        Ok(NativeReservation { parent, bytes })
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
        if failure == ScratchFailure::Ledger {
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
        if error.scratch_failure().is_none() {
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
        if !self.core().active.load(Ordering::Acquire) {
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
        if let Err(e) = c.parent.reserve(bytes, ScratchCharge::Layout) {
            self.fail(e);
            return Err(e);
        }
        if c.live
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                n.checked_add(bytes).filter(|n| *n <= c.limit)
            })
            .is_err()
        {
            self.fail(ScratchFailure::Budget);
            if let Err(e) = c.parent.release(bytes, ScratchCharge::Layout) {
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
        if let Err(e) = c.parent.release(layout.size(), ScratchCharge::Layout) {
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
        let metadata_charge = Self::metadata_bytes();
        parent.reserve(metadata_charge, ScratchCharge::Metadata)?;
        Ok(Self {
            handle: ScratchHandle(Some(Arc::new(ScratchCore {
                parent,
                limit,
                metadata_charge,
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
        scope.handle.core().active.store(false, Ordering::Release);
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
impl WorkScope {
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
