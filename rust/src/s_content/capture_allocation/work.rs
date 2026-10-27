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
pub(crate) enum WorkBudget {
    Owned(OwnedBudget),
    Scratch(super::scratch::ScratchHandle),
}
impl WorkBudget {
    pub(crate) fn reserve_native(
        &self,
        bytes: usize,
    ) -> Result<Option<super::scratch::NativeReservation>, CodecError> {
        match self {
            Self::Scratch(handle) => handle.reserve_native(bytes).map(Some),
            Self::Owned(_) => Ok(None),
        }
    }
    pub(crate) fn observe<T>(&self, result: &Result<T, CodecError>) {
        if let (Self::Scratch(b), Err(e)) = (self, result) {
            b.observe(e);
        }
    }

    fn reserve_layout(&self, layout: Layout, kind: &'static str) -> Result<(), CodecError> {
        match self {
            Self::Owned(b) => b.reserve_layout(layout, kind),
            Self::Scratch(b) => b.reserve(layout).map_err(CodecError::scratch),
        }
    }
    fn rollback_layout(&self, layout: Layout) {
        match self {
            Self::Owned(b) => b.rollback_layout(layout),
            Self::Scratch(b) => b.release(layout),
        }
    }
    fn release_layout(&self, layout: Layout) {
        match self {
            Self::Owned(b) => b.release_layout(layout),
            Self::Scratch(b) => b.release(layout),
        }
    }
    fn commit_layout(&self, layout: Layout) {
        if let Self::Owned(b) = self {
            b.commit_layout(layout);
        }
    }
    fn allocation_failed(&self) {
        if let Self::Scratch(b) = self {
            b.fail(super::scratch::ScratchFailure::Allocation);
        }
    }
    pub(crate) fn native_failure(&self, failure: super::scratch::ScratchFailure) -> CodecError {
        if let Self::Scratch(b) = self {
            b.fail(failure);
        }
        self.error()
    }
    pub(crate) fn error(&self) -> CodecError {
        match self {
            Self::Owned(_) => allocation_error(),
            Self::Scratch(b) => b.error(),
        }
    }
}
#[derive(Clone)]
pub(crate) struct LedgerAllocator {
    budget: WorkBudget,
    kind: &'static str,
}
impl LedgerAllocator {
    #[cfg(test)]
    pub(crate) fn new(budget: &OwnedBudget) -> Self {
        Self::new_tag(budget, "codec Layout")
    }
    pub(crate) fn new_tag(budget: &OwnedBudget, kind: &'static str) -> Self {
        Self {
            budget: WorkBudget::Owned(budget.clone()),
            kind,
        }
    }
    fn work(budget: &WorkBudget, kind: &'static str) -> Self {
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
            self.budget.allocation_failed();
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
    // Rust 按声明顺序析构：元素和 backing allocation 必须先于许可释放。
    native_permit: Option<super::scratch::NativeReservation>,
}
pub(crate) type WorkVec<T> = CodecVec<T>;
impl<T> Default for CodecVec<T> {
    fn default() -> Self {
        Self::from_plain(Vec::new())
    }
}
impl<T> CodecVec<T> {
    pub(crate) fn retained_layout_bytes(&self) -> Result<usize, CodecError> {
        self.capacity()
            .checked_mul(std::mem::size_of::<T>())
            .ok_or_else(allocation_error)
    }
    /// 只借实际allocator的原账本，不接受外来budget标记。
    pub fn allocation_budget(&self) -> Option<&OwnedBudget> {
        match &self.storage {
            VecStorage::Plain(_) => None,
            VecStorage::Owned(v) => match &v.allocator().budget {
                WorkBudget::Owned(b) => Some(b),
                WorkBudget::Scratch(_) => None,
            },
        }
    }
    pub fn new_with_allocation(budget: Option<&OwnedBudget>) -> Self {
        Self::new(budget)
    }
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), CodecError> {
        self.reserve(additional)
    }
    /// 同一个 Plain buffer 的原许可随 owner 移动；不是 Owned Layout。
    /// 已有非空裸 buffer 的费用来源不明，不能在这里重签掩盖旧 retain。
    pub fn try_reserve_native(
        &mut self,
        work: &super::scratch::WorkScope,
        additional: usize,
    ) -> Result<(), CodecError> {
        self.reserve_native_work(&work.budget(), additional)
    }
    /// #1488 冷原件接纳的原子入口。所有可能失败的容量/新块准入均在接纳前。
    /// #1488 P3-a：无生产调用方（生产父账恒拒接纳），只随测试/Native测试臂编译。
    #[cfg(any(test, feature = "s_capture_native_testonly"))]
    #[doc(hidden)]
    pub fn try_reserve_native_adopting(
        &mut self,
        work: &super::scratch::WorkScope,
        additional: usize,
    ) -> Result<(), CodecError> {
        use super::scratch::ScratchFailure;
        work.check_current()?;
        if self.native_permit.is_some() {
            return self.try_reserve_native(work, additional);
        }
        let old_bytes = self.retained_layout_bytes()?;
        let VecStorage::Plain(values) = &mut self.storage else {
            return Err(work.reject(ScratchFailure::Ledger));
        };
        let needed = values
            .len()
            .checked_add(additional)
            .ok_or_else(|| work.reject(ScratchFailure::Budget))?;
        if needed <= values.capacity() {
            self.native_permit = Some(work.adopt_native(old_bytes)?);
            return Ok(());
        }
        let capacity = needed
            .max(
                values
                    .capacity()
                    .checked_mul(2)
                    .ok_or_else(|| work.reject(ScratchFailure::Budget))?,
            )
            .max(4);
        let bytes = capacity
            .checked_mul(std::mem::size_of::<T>())
            .ok_or_else(|| work.reject(ScratchFailure::Budget))?;
        let permit = work.reserve_native(bytes)?;
        let mut replacement = Vec::new();
        replacement
            .try_reserve_exact(capacity)
            .map_err(|_| work.reject(ScratchFailure::Allocation))?;
        // 原件仍在values；接纳失败前没有append或绑定。成功后只做不可失败的移动。
        let old_permit = work.adopt_native(old_bytes)?;
        replacement.append(values);
        let old = std::mem::replace(values, replacement);
        drop(old);
        drop(old_permit);
        self.native_permit = Some(permit);
        Ok(())
    }
    pub(crate) fn reserve_native_work(
        &mut self,
        work: &WorkBudget,
        additional: usize,
    ) -> Result<(), CodecError> {
        use super::scratch::ScratchFailure;
        // 统一入口先核当前操作域，再核旧 owner 父身份；容量足够也不能绕过。
        // 双失效时保留当前操作域首错，不把旧 payload 重签到新父账。
        let probe = work
            .reserve_native(0)?
            .ok_or_else(|| work.native_failure(ScratchFailure::Ledger))?;
        if self
            .native_permit
            .as_ref()
            .is_some_and(|old| !old.same_parent(&probe))
        {
            return Err(work.native_failure(ScratchFailure::Ledger));
        }
        drop(probe);
        let VecStorage::Plain(values) = &mut self.storage else {
            return Err(work.native_failure(ScratchFailure::Ledger));
        };
        if self.native_permit.is_none() && values.capacity() != 0 && std::mem::size_of::<T>() != 0 {
            return Err(work.native_failure(ScratchFailure::Ledger));
        }
        let needed = values
            .len()
            .checked_add(additional)
            .ok_or_else(|| work.native_failure(ScratchFailure::Budget))?;
        if needed <= values.capacity() {
            if self.native_permit.is_none() {
                self.native_permit = work.reserve_native(0)?;
            }
            return Ok(());
        }
        let next = needed
            .max(
                values
                    .capacity()
                    .checked_mul(2)
                    .ok_or_else(|| work.native_failure(ScratchFailure::Budget))?,
            )
            .max(4);
        let bytes = next
            .checked_mul(std::mem::size_of::<T>())
            .ok_or_else(|| work.native_failure(ScratchFailure::Budget))?;
        let permit = work.reserve_native(bytes)?;
        // 独立新块使旧 allocation 与旧许可在完整新块准入/分配时都真实存活。
        let mut replacement = Vec::new();
        replacement
            .try_reserve_exact(next)
            .map_err(|_| work.native_failure(ScratchFailure::Allocation))?;
        replacement.append(values);
        let old = std::mem::replace(values, replacement);
        drop(old);
        self.native_permit = permit;
        Ok(())
    }
    pub fn remove_prefix(&mut self, count: usize) {
        match &mut self.storage {
            VecStorage::Plain(values) => {
                drop(values.drain(..count));
            }
            VecStorage::Owned(values) => {
                drop(values.drain(..count));
            }
        }
    }
    /// #1488 新旧块同时持费，载荷移动完成后才销毁旧块和它的原许可。
    pub fn try_compact_native(
        &mut self,
        work: Option<&super::scratch::WorkScope>,
    ) -> Result<(), CodecError> {
        use super::scratch::ScratchFailure;
        if let Some(work) = work {
            work.check_current()?;
            if (self.native_permit.is_none() && self.capacity() != 0)
                || self
                    .native_permit
                    .as_ref()
                    .is_some_and(|p| !p.belongs_to_parent(work))
            {
                return Err(work.reject(ScratchFailure::Ledger));
            }
        } else if self.native_permit.is_some() {
            return Err(allocation_error());
        }
        let VecStorage::Plain(values) = &mut self.storage else {
            return Err(allocation_error());
        };
        if values.len() == values.capacity() {
            return Ok(());
        }
        let bytes = values
            .len()
            .checked_mul(std::mem::size_of::<T>())
            .ok_or_else(allocation_error)?;
        let permit = work.map(|w| w.reserve_native(bytes)).transpose()?;
        let mut replacement = Vec::new();
        replacement.try_reserve_exact(values.len()).map_err(|_| {
            work.map_or_else(allocation_error, |w| w.reject(ScratchFailure::Allocation))
        })?;
        replacement.append(values);
        let old = std::mem::replace(values, replacement);
        drop(old);
        self.native_permit = permit;
        Ok(())
    }
    pub fn remove(&mut self, index: usize) -> T {
        match &mut self.storage {
            VecStorage::Plain(v) => v.remove(index),
            VecStorage::Owned(v) => v.remove(index),
        }
    }
    pub fn swap_remove(&mut self, index: usize) -> T {
        match &mut self.storage {
            VecStorage::Plain(v) => v.swap_remove(index),
            VecStorage::Owned(v) => v.swap_remove(index),
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
            native_permit: None,
        }
    }
    pub(crate) fn new(b: Option<&OwnedBudget>) -> Self {
        Self::new_work(b.map(|b| WorkBudget::Owned(b.clone())).as_ref())
    }
    pub(crate) fn new_work(b: Option<&WorkBudget>) -> Self {
        Self {
            storage: match b {
                None => VecStorage::Plain(Vec::new()),
                Some(b) => VecStorage::Owned(AllocVec::new_in(LedgerAllocator::work(
                    b,
                    std::any::type_name::<T>(),
                ))),
            },
            native_permit: None,
        }
    }
    #[cfg(test)]
    pub(crate) fn with_capacity(n: usize, b: Option<&OwnedBudget>) -> Result<Self, CodecError> {
        Self::with_work_capacity(n, b.map(|b| WorkBudget::Owned(b.clone())).as_ref())
    }
    pub(crate) fn with_work_capacity(n: usize, b: Option<&WorkBudget>) -> Result<Self, CodecError> {
        let mut v = Self::new_work(b);
        v.reserve_exact(n)?;
        Ok(v)
    }
    pub(crate) fn reserve(&mut self, n: usize) -> Result<(), CodecError> {
        if self.native_permit.is_some() {
            return if n <= self.capacity().saturating_sub(self.len()) {
                Ok(())
            } else {
                Err(allocation_error())
            };
        }
        match &mut self.storage {
            VecStorage::Plain(v) => v.try_reserve(n).map_err(|_| allocation_error()),
            VecStorage::Owned(v) => v.try_reserve(n).map_err(|_| v.allocator().budget.error()),
        }
    }
    pub(crate) fn reserve_exact(&mut self, n: usize) -> Result<(), CodecError> {
        if self.native_permit.is_some() {
            return if n <= self.capacity().saturating_sub(self.len()) {
                Ok(())
            } else {
                Err(allocation_error())
            };
        }
        match &mut self.storage {
            VecStorage::Plain(v) => v.try_reserve_exact(n).map_err(|_| allocation_error()),
            VecStorage::Owned(v) => v
                .try_reserve_exact(n)
                .map_err(|_| v.allocator().budget.error()),
        }
    }
    /// 是否已绑定 Native 父账，包括容量为零的许可。
    /// #1488：已有初始原件费的消费式附接，不分配也不复制载荷。
    #[doc(hidden)]
    #[doc(hidden)]
    pub fn native_parent_matches(
        &self,
        parent: &std::sync::Arc<dyn super::scratch::ScratchParent>,
    ) -> bool {
        self.native_permit
            .as_ref()
            .is_some_and(|permit| permit.belongs_to_parent_arc(parent))
    }
    pub fn can_adopt_native(&self) -> bool {
        self.native_permit.is_none() && matches!(self.storage, VecStorage::Plain(_))
    }
    #[doc(hidden)]
    pub fn attach_native_reservation(
        &mut self,
        permit: super::scratch::NativeReservation,
    ) -> Result<(), CodecError> {
        if self.native_permit.is_some()
            || !matches!(self.storage, VecStorage::Plain(_))
            || permit.bytes() != self.retained_layout_bytes()?
        {
            return Err(CodecError::scratch(super::scratch::ScratchFailure::Ledger));
        }
        self.native_permit = Some(permit);
        Ok(())
    }
    pub fn check_retained_source(&self) -> Result<(), CodecError> {
        self.native_permit.as_ref().map_or(
            Ok(()),
            super::scratch::NativeReservation::check_retained_source,
        )
    }
    pub fn native_belongs_to_work(&self, work: &super::scratch::WorkScope) -> bool {
        self.native_permit
            .as_ref()
            .is_some_and(|p| p.belongs_to_parent(work))
    }
    pub fn is_native_bound(&self) -> bool {
        self.native_permit.is_some()
    }
    /// #1488 P3-c：本块实际持有的Native许可字节（只读，不签发、不转移）。
    #[doc(hidden)]
    pub fn native_retained_charge(&self) -> Option<usize> {
        self.native_permit
            .as_ref()
            .map(super::scratch::NativeReservation::bytes)
    }
    /// #1572：本块Native许可的身份与字节（只读）；与 `native_retained_charge` 同一张许可。
    #[doc(hidden)]
    pub fn native_permit_entry(&self) -> Option<(super::scratch::PermitId, usize)> {
        self.native_permit
            .as_ref()
            .map(|p| (p.permit_id(), p.bytes()))
    }
    /// Native 写入必须带本次活 scope；共享入口核父身份并先准入完整新块。
    pub fn try_push_native(
        &mut self,
        value: T,
        work: &super::scratch::WorkScope,
    ) -> Result<(), CodecError> {
        self.try_reserve_native(work, 1)?;
        match &mut self.storage {
            VecStorage::Plain(v) => v.push(value),
            VecStorage::Owned(_) => unreachable!("Native reserve rejects Owned storage"),
        }
        Ok(())
    }
    /// 有序插入；错误 index 在准入和写入前拒绝。
    pub fn try_insert_native(
        &mut self,
        index: usize,
        value: T,
        work: &super::scratch::WorkScope,
    ) -> Result<(), CodecError> {
        self.check_insert_index(index)?;
        self.try_reserve_native(work, 1)?;
        match &mut self.storage {
            VecStorage::Plain(v) => v.insert(index, value),
            VecStorage::Owned(_) => unreachable!("Native reserve rejects Owned storage"),
        }
        Ok(())
    }
    /// Plain/Owned 插入沿原 allocator；Native 必须显式提供活 scope。
    pub fn try_insert(&mut self, index: usize, value: T) -> Result<(), CodecError> {
        self.check_insert_index(index)?;
        if self.is_native_bound() {
            return Err(allocation_error());
        }
        self.reserve(1)?;
        match &mut self.storage {
            VecStorage::Plain(v) => v.insert(index, value),
            VecStorage::Owned(v) => v.insert(index, value),
        }
        Ok(())
    }
    fn check_insert_index(&self, index: usize) -> Result<(), CodecError> {
        if index > self.len() {
            Err(CodecError::static_detail(
                INCOMPLETE,
                "codec有序插入index超出长度",
            ))
        } else {
            Ok(())
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
        if self.native_permit.is_some() {
            return Err(allocation_error());
        }
        match &self.storage {
            VecStorage::Plain(v) => Ok(Self::from_plain(v.clone())),
            VecStorage::Owned(v) => {
                let mut c = Self::new_work(Some(&v.allocator().budget));
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
        if self.native_permit.is_some() {
            return Err(allocation_error());
        }
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
    /// #1488 原 Plain Vec allocation 和原许可一并转为 UTF-8 String；不复制或重签。
    pub fn try_into_native_text(
        self,
    ) -> Result<super::native_owned::NativeValue<String>, CodecError> {
        match self.storage {
            VecStorage::Plain(bytes) => {
                let text = String::from_utf8(bytes).map_err(|_| super::super::utf8_error())?;
                Ok(super::native_owned::NativeValue::from_reserved(
                    text,
                    self.native_permit,
                ))
            }
            VecStorage::Owned(_) => Err(CodecError::static_detail(
                INCOMPLETE,
                "owned codec字节不可擦除为Native文本",
            )),
        }
    }
    /// 字节追加只复制u8，不开放可能深复制payload的泛型Clone入口。
    pub fn try_extend_bytes_native(
        &mut self,
        bytes: &[u8],
        work: &super::scratch::WorkScope,
    ) -> Result<(), CodecError> {
        self.try_reserve_native(work, bytes.len())?;
        match &mut self.storage {
            VecStorage::Plain(v) => v.extend_from_slice(bytes),
            VecStorage::Owned(_) => unreachable!("Native reserve rejects Owned storage"),
        }
        Ok(())
    }
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
    _native_permit: Option<super::scratch::NativeReservation>,
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
            _native_permit: self.native_permit,
        }
    }
}
impl<T> AsRef<[T]> for CodecVec<T> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

/// #1394 测试辅助：hashbrown 0.17.1（Cargo.toml 钉 `=0.17.1`）一张表的 requested Layout 字节数。
/// 组宽照 `control/group/mod.rs` 的选择：x86/x86_64 且 sse2 → 16（`__m128i`）；小端 aarch64 且 neon → 8
/// （`uint8x8_t`）；其余走 generic（`GroupWord` = 指针宽）。字节数照 `raw.rs` `TableLayout::calculate_layout_for`：
/// `round_up(size·buckets, max(align, 组宽)) + buckets + 组宽`。只供测试由同一公式推出期望值；hashbrown 升级
/// 改了布局时 `memo_tables_match_hashbrown_layout_formula` 先红，依赖本模块的期望值须随之复核。
#[cfg(test)]
pub(crate) mod hashbrown_layout {
    pub(crate) const GROUP_WIDTH: usize = if cfg!(all(
        target_feature = "sse2",
        any(target_arch = "x86", target_arch = "x86_64"),
        not(miri)
    )) {
        16
    } else if cfg!(all(
        target_arch = "aarch64",
        target_feature = "neon",
        target_endian = "little",
        not(miri)
    )) {
        8
    } else {
        std::mem::size_of::<usize>()
    };
    /// `buckets` 个桶、元素 `T` 的表分配字节数。
    pub(crate) fn table_bytes_of<T>(buckets: usize) -> usize {
        let (size, align) = (std::mem::size_of::<T>(), std::mem::align_of::<T>());
        let ctrl_align = align.max(GROUP_WIDTH);
        ((size * buckets + ctrl_align - 1) & !(ctrl_align - 1)) + buckets + GROUP_WIDTH
    }
    /// `capacity()`（= `bucket_mask_to_capacity`，表内无墓碑时）→ 桶数。
    pub(crate) fn buckets_for_capacity(capacity: usize) -> usize {
        if capacity < 8 {
            capacity + 1
        } else {
            capacity / 7 * 8
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
    #[cfg(feature = "s_capture_native_testonly")]
    pub(crate) fn try_reserve(&mut self, additional: usize) -> Result<(), CodecError> {
        match self {
            Self::Plain(m) => m.try_reserve(additional).map_err(|_| allocation_error()),
            Self::Owned(m) => m
                .try_reserve(additional)
                .map_err(|_| m.allocator().budget.error()),
        }
    }
    pub(crate) fn retain(&mut self, f: impl FnMut(&K, &mut V) -> bool) {
        match self {
            Self::Plain(m) => m.retain(f),
            Self::Owned(m) => m.retain(f),
        }
    }
    pub(crate) fn iter_mut(&mut self) -> hashbrown::hash_map::IterMut<'_, K, V> {
        match self {
            Self::Plain(m) => m.iter_mut(),
            Self::Owned(m) => m.iter_mut(),
        }
    }
    pub(crate) fn new(b: Option<&OwnedBudget>) -> Self {
        Self::new_work(b.map(|b| WorkBudget::Owned(b.clone())).as_ref())
    }
    pub(crate) fn new_work(b: Option<&WorkBudget>) -> Self {
        match b {
            None => Self::default(),
            Some(b) => Self::Owned(hashbrown::HashMap::with_hasher_in(
                RandomState::new(),
                LedgerAllocator::work(b, std::any::type_name::<(K, V)>()),
            )),
        }
    }
    pub(crate) fn retained_layout_bytes(&self) -> usize {
        match self {
            Self::Plain(m) => m.allocation_size(),
            Self::Owned(m) => m.allocation_size(),
        }
    }
    pub(crate) fn insert(&mut self, k: K, v: V) -> Result<Option<V>, CodecError> {
        match self {
            Self::Plain(m) => Ok(m.insert(k, v)),
            Self::Owned(m) => {
                if let Some(old) = m.get_mut(&k) {
                    return Ok(Some(std::mem::replace(old, v)));
                }
                m.try_reserve(1).map_err(|_| m.allocator().budget.error())?;
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
    /// 测试：已分配表的（实际 allocation_size，hashbrown 公式值）；未分配返回 None。
    #[cfg(test)]
    pub(crate) fn layout_vs_formula(&self) -> Option<(usize, usize)> {
        use hashbrown_layout::{buckets_for_capacity, table_bytes_of};
        let actual = self.retained_layout_bytes();
        (actual > 0).then(|| {
            (
                actual,
                table_bytes_of::<(K, V)>(buckets_for_capacity(self.capacity())),
            )
        })
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
    #[allow(
        dead_code,
        reason = "#1608：与 WorkMap::new/CodecVec::new 对称的 OwnedBudget 入口，现调用方都走 new_work；保留待接线"
    )]
    pub(crate) fn new(b: Option<&OwnedBudget>) -> Self {
        Self::new_work(b.map(|b| WorkBudget::Owned(b.clone())).as_ref())
    }
    pub(crate) fn new_work(b: Option<&WorkBudget>) -> Self {
        match b {
            None => Self::default(),
            Some(b) => Self::Owned(hashbrown::HashSet::with_hasher_in(
                RandomState::new(),
                LedgerAllocator::work(b, std::any::type_name::<T>()),
            )),
        }
    }
    pub(crate) fn retained_layout_bytes(&self) -> Option<usize> {
        match self {
            Self::Plain(_) => None,
            Self::Owned(m) => Some(m.allocation_size()),
        }
    }
    /// 测试：已分配表的（实际 allocation_size，hashbrown 公式值）；Plain 或未分配返回 None。
    #[cfg(test)]
    pub(crate) fn layout_vs_formula(&self) -> Option<(usize, usize)> {
        use hashbrown_layout::{buckets_for_capacity, table_bytes_of};
        let actual = self.retained_layout_bytes()?;
        (actual > 0).then(|| {
            (
                actual,
                table_bytes_of::<T>(buckets_for_capacity(self.capacity())),
            )
        })
    }
    pub(crate) fn insert(&mut self, v: T) -> Result<bool, CodecError> {
        match self {
            Self::Plain(m) => Ok(m.insert(v)),
            Self::Owned(m) => {
                if m.contains(&v) {
                    return Ok(false);
                }
                m.try_reserve(1).map_err(|_| m.allocator().budget.error())?;
                Ok(m.insert(v))
            }
        }
    }
    #[cfg(all(test, feature = "s_capture_native_testonly"))]
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
    /// #1488 Native 保留账：真实 String capacity 加 BTree 节点槽的保守上界。
    /// 只读；每个来源贡献首次入账/替换时调用，不用于每代扫描旧缓存。
    /// Owned 的内存由其原预算持有，返回 None，不能擦成 Native 账；溢出也返回 None。
    #[doc(hidden)]
    pub fn plain_retained_bytes(&self) -> Option<usize> {
        let FieldsStorage::Plain(fields) = &self.storage else {
            return None;
        };
        let slot = std::mem::size_of::<(String, super::super::ContentRef)>()
            .checked_mul(11)?
            .checked_add(16 * std::mem::size_of::<usize>())?;
        fields
            .iter()
            .try_fold(fields.len().checked_mul(slot)?, |bytes, (key, value)| {
                bytes
                    .checked_add(key.capacity())?
                    .checked_add(value.format.capacity())?
                    .checked_add(value.root.capacity())
            })
    }
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
        struct Panics(
            #[allow(
                dead_code,
                reason = "#1608：非零尺寸载荷，使 drop 时 panic 的元素真占 Layout 字节"
            )]
            u8,
        );
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

#[cfg(test)]
mod native_vec_scope_tests {
    use super::super::scratch::{
        CodecWork, ScratchCharge, ScratchFailure, ScratchParent, WorkError,
    };
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct Counts {
        live: usize,
        native: usize,
        peak: usize,
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
    }
    impl ScratchParent for Parent {
        fn reserve(&self, bytes: usize, kind: ScratchCharge) -> Result<u64, ScratchFailure> {
            let mut c = self.counts.lock().unwrap();
            let next = c.live.checked_add(bytes).ok_or(ScratchFailure::Budget)?;
            if next > self.limit {
                return Err(ScratchFailure::Budget);
            }
            c.live = next;
            c.peak = c.peak.max(next);
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
    fn run<T>(
        p: &Arc<Parent>,
        f: impl FnOnce(&super::super::scratch::WorkScope) -> Result<T, CodecError>,
    ) -> T {
        CodecWork::new(p.clone(), p.limit).unwrap().run(f).unwrap()
    }
    #[test]
    fn native_vec_foreign_parent_rejects_spare_growth_and_zero_permit() {
        for additional in [0, 1, 8] {
            let a = Parent::new(1 << 20);
            let b = Parent::new(1 << 20);
            let mut v = run(&a, |w| {
                let mut v = CodecVec::default();
                v.try_push_native(7u64, w)?;
                Ok(v)
            });
            let cap = v.capacity();
            let charged = a.native();
            let result = CodecWork::new(b.clone(), b.limit)
                .unwrap()
                .run(|w| v.try_reserve_native(w, additional));
            assert!(matches!(
                result,
                Err(WorkError::Resource(ScratchFailure::Ledger))
            ));
            assert_eq!(&*v, &[7]);
            assert_eq!(v.capacity(), cap);
            assert_eq!(a.native(), charged);
            assert_eq!(b.live(), 0);
            drop(v);
            assert_eq!(a.live(), 0);
        }
        let a = Parent::new(1 << 20);
        let b = Parent::new(1 << 20);
        let mut empty = run(&a, |w| {
            let mut v = CodecVec::<u64>::default();
            v.try_reserve_native(w, 0)?;
            Ok(v)
        });
        assert!(empty.is_native_bound());
        assert_eq!(empty.capacity(), 0);
        assert!(CodecWork::new(b.clone(), b.limit)
            .unwrap()
            .run(|w| empty.try_reserve_native(w, 0))
            .is_err());
        assert_eq!(b.live(), 0);
        drop(empty);
        assert_eq!(a.live(), 0);
    }
    #[test]
    fn native_vec_same_parent_new_scope_insert_order_and_refund() {
        let p = Parent::new(1 << 20);
        let mut v = run(&p, |w| {
            let mut v = CodecVec::default();
            v.try_push_native(1u64, w)?;
            v.try_push_native(3, w)?;
            Ok(v)
        });
        run(&p, |w| {
            v.try_reserve_native(w, 0)?;
            v.try_insert_native(1, 2, w)?;
            v.try_insert_native(0, 0, w)?;
            v.try_insert_native(4, 4, w)?;
            let cap = v.capacity();
            let before = p.native();
            assert!(v.try_insert_native(6, 99, w).is_err());
            assert_eq!(v.capacity(), cap);
            assert_eq!(p.native(), before);
            Ok(())
        });
        assert_eq!(&*v, &[0, 1, 2, 3, 4]);
        assert_eq!(p.native(), v.capacity() * std::mem::size_of::<u64>());
        assert!(v.try_insert(0, 99).is_err());
        drop(v);
        assert_eq!(p.live(), 0);
        let mut bytes = run(&p, |w| {
            let mut v = CodecVec::default();
            v.try_extend_bytes_native(b"ab", w)?;
            Ok(v)
        });
        run(&p, |w| bytes.try_extend_bytes_native(b"cd", w));
        assert_eq!(&*bytes, b"abcd");
        drop(bytes);
        assert_eq!(p.live(), 0);
    }
    #[test]
    fn native_vec_failed_or_closed_scope_rejects_zero_growth() {
        let p = Parent::new(1 << 20);
        let mut v = run(&p, |w| {
            let mut v = CodecVec::default();
            v.try_push_native(7u64, w)?;
            Ok(v)
        });
        let cap = v.capacity();
        let charge = p.native();
        let result = CodecWork::new(p.clone(), p.limit).unwrap().run(|w| {
            let _ = w.reject(ScratchFailure::Budget);
            assert!(v.try_reserve_native(w, 0).is_err());
            assert!(v.try_push_native(8, w).is_err());
            Ok::<_, CodecError>(())
        });
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        let closed = run(&p, |w| Ok(w.budget()));
        assert!(v.reserve_native_work(&closed, 0).is_err());
        assert_eq!(&*v, &[7]);
        assert_eq!(v.capacity(), cap);
        assert_eq!(p.native(), charge);
        drop(closed);
        drop(v);
        assert_eq!(p.live(), 0);
    }
    #[test]
    fn native_vec_internal_entry_keeps_current_scope_first_failure() {
        let a = Parent::new(1 << 20);
        let b = Parent::new(1 << 20);
        let mut v = run(&a, |w| {
            let mut v = CodecVec::default();
            v.try_push_native(7u64, w)?;
            Ok(v)
        });
        let charge = a.native();
        let result = CodecWork::new(b.clone(), b.limit).unwrap().run(|w| {
            let _ = w.reject(ScratchFailure::Budget);
            v.reserve_native_work(&w.budget(), 0)
        });
        assert!(matches!(
            result,
            Err(WorkError::Resource(ScratchFailure::Budget))
        ));
        assert_eq!(a.native(), charge);
        assert_eq!(&*v, &[7]);
        assert_eq!(b.live(), 0);
        assert!(v
            .reserve_native_work(&WorkBudget::Owned(OwnedBudget::new(4096)), 0)
            .is_err());
        drop(v);
        assert_eq!(a.live(), 0);
    }
    #[test]
    fn native_vec_growth_failure_is_atomic_and_old_new_are_charged() {
        for room in [95, 96] {
            let p = Parent::new(CodecWork::metadata_bytes() + room);
            let mut v = run(&p, |w| {
                let mut v = CodecVec::default();
                for x in 0u64..4 {
                    v.try_push_native(x, w)?;
                }
                Ok(v)
            });
            assert_eq!(v.capacity(), 4);
            let result = CodecWork::new(p.clone(), p.limit)
                .unwrap()
                .run(|w| v.try_insert_native(2, 99, w));
            if room == 95 {
                assert!(matches!(
                    result,
                    Err(WorkError::Resource(ScratchFailure::Budget))
                ));
                assert_eq!(&*v, &[0, 1, 2, 3]);
                assert_eq!(v.capacity(), 4);
                assert_eq!(p.native(), 32);
            } else {
                result.unwrap();
                assert_eq!(&*v, &[0, 1, 99, 2, 3]);
                assert_eq!(v.capacity(), 8);
                assert_eq!(p.native(), 64);
                assert_eq!(
                    p.counts.lock().unwrap().peak,
                    CodecWork::metadata_bytes() + 96
                );
            }
            drop(v);
            assert_eq!(p.live(), 0);
        }
    }
    #[test]
    fn plain_owned_insert_preserves_order_and_failed_capacity() {
        let mut plain = CodecVec::from_plain(vec![1u64, 3]);
        plain.try_insert(1, 2).unwrap();
        assert_eq!(&*plain, &[1, 2, 3]);
        let b = OwnedBudget::new(96);
        let mut owned = CodecVec::with_capacity(4, Some(&b)).unwrap();
        for x in 0u64..4 {
            owned.try_push(x).unwrap();
        }
        owned.try_insert(2, 99).unwrap();
        assert_eq!(&*owned, &[0, 1, 99, 2, 3]);
        drop(owned);
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
    }
}
